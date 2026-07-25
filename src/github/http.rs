//! The HTTP layer: shared clients, the conditional-request cache with its
//! per-URL locks, and status plumbing. Nothing above this module touches headers.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::Mutex as TokioMutex;

pub(crate) const GITHUB_API: &str = "https://api.github.com";

pub(crate) const GITHUB_ACCOUNT: &str = "Project-Colony";

pub(crate) const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Owner/repo for the Colony launcher itself.
pub(crate) const LAUNCHER_OWNER: &str = "Project-Colony";

pub(crate) const LAUNCHER_REPO: &str = "Colony";

/// Default HTTP timeout for all GitHub API requests.
pub(crate) const HTTP_TIMEOUT: Duration = Duration::from_secs(30);

/// Default connect timeout.
pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Cap on concurrent per-repo fetches during a store refresh so a large org
/// cannot fire an unbounded burst of requests at the GitHub API at once.
pub(crate) const MAX_CONCURRENT_REPO_FETCHES: usize = 8;

// --- HTTP ETag Cache ---

struct CacheEntry {
    etag: String,
    body: String,
}

static HTTP_CACHE: std::sync::LazyLock<Mutex<HashMap<String, CacheEntry>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Per-URL lock to prevent concurrent requests to the same endpoint.
static URL_LOCKS: std::sync::LazyLock<Mutex<HashMap<String, std::sync::Arc<TokioMutex<()>>>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// Acquire a per-URL lock to prevent race conditions on the same endpoint.
fn url_lock(url: &str) -> std::sync::Arc<TokioMutex<()>> {
    let mut locks = URL_LOCKS.lock().expect("URL_LOCKS mutex poisoned");
    locks
        .entry(url.to_string())
        .or_insert_with(|| std::sync::Arc::new(TokioMutex::new(())))
        .clone()
}

/// Rate-limit information from GitHub API response headers.
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    pub remaining: u64,
    pub limit: u64,
    pub reset: u64,
}

/// Perform a GET request with ETag caching, per-URL locking, and rate-limit awareness.
/// Returns (body_string, optional_rate_limit_info).
pub(crate) async fn cached_get(
    client: &reqwest::Client,
    url: &str,
) -> Result<(String, Option<RateLimitInfo>)> {
    let lock = url_lock(url);
    let _guard = lock.lock().await;

    let mut request = client.get(url);

    // Add If-None-Match if we have a cached ETag
    if let Ok(cache) = HTTP_CACHE.lock() {
        if let Some(entry) = cache.get(url) {
            request = request.header("If-None-Match", &entry.etag);
        }
    }

    let resp = request.send().await.map_err(|e| {
        if e.is_timeout() {
            anyhow::anyhow!("Request timed out for {url}")
        } else if e.is_connect() {
            anyhow::anyhow!("Connection failed for {url}: {e}")
        } else {
            anyhow::anyhow!("Network error for {url}: {e}")
        }
    })?;

    // Parse rate-limit headers
    let rate_limit = parse_rate_limit(resp.headers());

    if let Some(ref rl) = rate_limit {
        if rl.remaining < 10 {
            tracing::warn!(
                "GitHub API rate limit low: {}/{} remaining (resets at {})",
                rl.remaining,
                rl.limit,
                rl.reset
            );
        }
    }

    match resp.status().as_u16() {
        304 => {
            // Not Modified — return cached body
            if let Ok(cache) = HTTP_CACHE.lock() {
                if let Some(entry) = cache.get(url) {
                    tracing::debug!("Cache hit (304) for {}", url);
                    return Ok((entry.body.clone(), rate_limit));
                }
            }
            anyhow::bail!("304 received but no cached body for {url}");
        }
        200 => {
            let etag = resp
                .headers()
                .get("etag")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());
            let body = resp.text().await?;

            // Store in cache if we got an ETag
            if let Some(etag) = etag {
                if let Ok(mut cache) = HTTP_CACHE.lock() {
                    cache.insert(
                        url.to_string(),
                        CacheEntry {
                            etag,
                            body: body.clone(),
                        },
                    );
                }
            }
            Ok((body, rate_limit))
        }
        status => {
            // Only treat an exhausted quota as a rate-limit error on the
            // statuses GitHub actually uses for it (403 / 429). A 200 or 304
            // that merely happened to consume the last quota unit is handled
            // above and its body is preserved.
            if matches!(status, 403 | 429) {
                if let Some(ref rl) = rate_limit {
                    if rl.remaining == 0 {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs();
                        if rl.reset > now {
                            let wait = rl.reset - now;
                            anyhow::bail!(
                                "{}",
                                crate::i18n::t_fmt(
                                    "github_rate_limit",
                                    &[("wait", &wait.to_string())]
                                )
                            );
                        }
                    }
                }
            }
            let body = resp.text().await.unwrap_or_default();
            Err(anyhow::Error::new(HttpStatus(status))
                .context(format!("GitHub API error {status}: {body}")))
        }
    }
}

/// Typed HTTP failure status carried inside the `anyhow` chain, so callers can
/// classify not-found precisely with [`is_not_found`] instead of substring-
/// matching "404" against the message - which misfired on any response body
/// that merely CONTAINED "404" and silently dropped legitimate repos from the
/// catalog (then clobbered the offline cache without them).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpStatus(pub u16);

impl std::fmt::Display for HttpStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HTTP {}", self.0)
    }
}

impl std::error::Error for HttpStatus {}

/// True when the error chain carries an HTTP 404 from the GitHub API.
pub fn is_not_found(e: &anyhow::Error) -> bool {
    e.downcast_ref::<HttpStatus>().is_some_and(|s| s.0 == 404)
}

pub(crate) fn parse_rate_limit(headers: &reqwest::header::HeaderMap) -> Option<RateLimitInfo> {
    let remaining = headers
        .get("x-ratelimit-remaining")?
        .to_str()
        .ok()?
        .parse()
        .ok()?;
    let limit = headers
        .get("x-ratelimit-limit")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(60);
    let reset = headers
        .get("x-ratelimit-reset")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    Some(RateLimitInfo {
        remaining,
        limit,
        reset,
    })
}

/// Build an HTTP client for API calls (public wrapper).
pub fn build_update_client(token: Option<&str>) -> Result<reqwest::Client> {
    build_client(token)
}

pub(crate) fn build_client(token: Option<&str>) -> Result<reqwest::Client> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(
        reqwest::header::ACCEPT,
        "application/vnd.github.v3+json".parse()?,
    );
    headers.insert(
        reqwest::header::USER_AGENT,
        format!("Colony-Launcher/{APP_VERSION}").parse()?,
    );
    if let Some(token) = token {
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {token}").parse()?,
        );
    }
    Ok(reqwest::Client::builder()
        .default_headers(headers)
        .timeout(HTTP_TIMEOUT)
        .connect_timeout(CONNECT_TIMEOUT)
        .build()?)
}
