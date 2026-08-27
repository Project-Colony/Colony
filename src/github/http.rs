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

/// How long a remembered 404 is trusted before Colony probes the URL again.
///
/// The catalog deliberately generates 404s: it tries several CHANGELOG names,
/// several licence filenames, a second icon path, and colony.json for every
/// repo in the org including the ones that will never have one. GitHub bills
/// each of those against the quota and an ETag can never help, because a 404
/// carries no ETag to send back. Remembering them is what keeps a cold boot
/// inside the 60/h anonymous budget. The TTL is the cost of the trade: a repo
/// that ADDS a CHANGELOG is picked up on the next window, or immediately via
/// Settings > "Clear caches".
const NOT_FOUND_TTL_SECS: u64 = 6 * 3600;

/// Skip persisting any single body larger than this. READMEs are the big
/// entries here and a pathological one should not bloat every launch.
const MAX_CACHED_BODY_BYTES: usize = 512 * 1024;

/// Total on-disk budget for the persisted cache.
const MAX_CACHE_FILE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct CacheEntry {
    etag: String,
    body: String,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct PersistedCache {
    /// url -> last ETag and the body it described.
    #[serde(default)]
    entries: HashMap<String, CacheEntry>,
    /// url -> unix seconds when GitHub last answered 404.
    #[serde(default)]
    not_found: HashMap<String, u64>,
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// The conditional-request cache, restored from disk at first use.
///
/// This used to start empty on every launch, which made a cold boot and a warm
/// boot cost exactly the same: ~55 full 200s for the org catalog, against an
/// anonymous budget of 60/h. Opening Colony twice within the hour rate-limited
/// the second launch. GitHub does not bill a 304, so replaying the ETags turns
/// almost the whole refresh free - it only ever needed to survive the process.
static HTTP_CACHE: std::sync::LazyLock<Mutex<PersistedCache>> = std::sync::LazyLock::new(|| {
    Mutex::new(crate::persistence::load_http_cache_json().unwrap_or_default())
});

/// Write the conditional-request cache to disk. Called after a successful
/// catalog refresh, where the map is at its most useful and most complete.
pub fn save_http_cache() {
    let Ok(cache) = HTTP_CACHE.lock() else {
        return;
    };
    let cutoff = now_secs().saturating_sub(NOT_FOUND_TTL_SECS);
    let snapshot = PersistedCache {
        entries: bound_entries(&cache.entries, MAX_CACHE_FILE_BYTES),
        not_found: cache
            .not_found
            .iter()
            .filter(|(_, &seen)| seen > cutoff)
            .map(|(k, v)| (k.clone(), *v))
            .collect(),
    };
    drop(cache);
    if let Err(e) = crate::persistence::save_http_cache_json(&snapshot) {
        tracing::warn!("could not persist the HTTP cache: {e}");
    }
}

/// Select the entries that fit the on-disk budget.
///
/// Smallest first, so a squeeze drops the handful of giant READMEs rather than
/// the dozens of small manifest and release responses that make up most of the
/// request count - saving quota is the point, not saving bytes.
fn bound_entries(
    entries: &HashMap<String, CacheEntry>,
    mut budget: usize,
) -> HashMap<String, CacheEntry> {
    let mut sorted: Vec<(&String, &CacheEntry)> = entries.iter().collect();
    sorted.sort_by_key(|(url, e)| (e.body.len(), url.as_str()));
    let mut keep = HashMap::new();
    for (url, entry) in sorted {
        if entry.body.len() > MAX_CACHED_BODY_BYTES {
            continue;
        }
        let cost = url.len() + entry.etag.len() + entry.body.len();
        if cost > budget {
            break;
        }
        budget -= cost;
        keep.insert(url.clone(), entry.clone());
    }
    keep
}

/// Drop every remembered response. Wired to Settings > "Clear caches" so a user
/// who suspects staleness has a button, including for remembered 404s.
pub fn clear_http_cache() {
    if let Ok(mut cache) = HTTP_CACHE.lock() {
        cache.entries.clear();
        cache.not_found.clear();
    }
    let _ = crate::persistence::save_http_cache_json(&PersistedCache::default());
}

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
///
/// Private: `cached_get` used to hand this back to every caller and all six
/// discarded it, so it read like a supported channel while being dead weight.
/// The values are consumed here - the low-quota warning and the 403/429
/// exhaustion check. Surfacing the remaining quota in the UI would be a
/// deliberate feature, not a reason to keep an unread return value.
#[derive(Debug, Clone)]
struct RateLimitInfo {
    pub remaining: u64,
    pub limit: u64,
    pub reset: u64,
}

/// Perform a GET request with ETag caching, per-URL locking, and rate-limit
/// awareness. Returns the body.
pub(crate) async fn cached_get(client: &reqwest::Client, url: &str) -> Result<String> {
    let lock = url_lock(url);
    let _guard = lock.lock().await;

    let mut request = client.get(url);

    // Add If-None-Match if we have a cached ETag
    if let Ok(cache) = HTTP_CACHE.lock() {
        // A 404 we saw recently costs nothing to answer from here, and unlike a
        // 200 it can never be revalidated with an ETag.
        if let Some(&seen) = cache.not_found.get(url) {
            if now_secs().saturating_sub(seen) < NOT_FOUND_TTL_SECS {
                tracing::debug!("negative cache hit for {url}");
                return Err(anyhow::Error::new(HttpStatus(404))
                    .context(format!("GitHub API error 404 (remembered): {url}")));
            }
        }
        if let Some(entry) = cache.entries.get(url) {
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
                if let Some(entry) = cache.entries.get(url) {
                    tracing::debug!("Cache hit (304) for {}", url);
                    return Ok(entry.body.clone());
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
                    // The resource exists again; drop any remembered 404.
                    cache.not_found.remove(url);
                    cache.entries.insert(
                        url.to_string(),
                        CacheEntry {
                            etag,
                            body: body.clone(),
                        },
                    );
                }
            }
            Ok(body)
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
            if status == 404 {
                if let Ok(mut cache) = HTTP_CACHE.lock() {
                    cache.not_found.insert(url.to_string(), now_secs());
                    cache.entries.remove(url);
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

fn parse_rate_limit(headers: &reqwest::header::HeaderMap) -> Option<RateLimitInfo> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(body_len: usize) -> CacheEntry {
        CacheEntry {
            etag: "e".into(),
            body: "x".repeat(body_len),
        }
    }

    #[test]
    fn the_cache_budget_keeps_the_many_small_entries_over_the_one_huge_one() {
        let mut entries = HashMap::new();
        entries.insert("u/big".to_string(), entry(900));
        entries.insert("u/a".to_string(), entry(10));
        entries.insert("u/b".to_string(), entry(10));
        entries.insert("u/c".to_string(), entry(10));

        // Room for the three small ones and their keys, not for the big one.
        let kept = bound_entries(&entries, 100);
        assert_eq!(kept.len(), 3, "kept: {:?}", kept.keys().collect::<Vec<_>>());
        assert!(!kept.contains_key("u/big"));

        // A single body over the per-entry ceiling is never persisted, however
        // much room is left.
        let mut huge = HashMap::new();
        huge.insert("u/huge".to_string(), entry(MAX_CACHED_BODY_BYTES + 1));
        assert!(bound_entries(&huge, MAX_CACHE_FILE_BYTES).is_empty());
    }

    #[test]
    fn a_remembered_404_expires() {
        let now = now_secs();
        let fresh = now.saturating_sub(NOT_FOUND_TTL_SECS / 2);
        let stale = now.saturating_sub(NOT_FOUND_TTL_SECS + 1);
        assert!(now.saturating_sub(fresh) < NOT_FOUND_TTL_SECS);
        assert!(now.saturating_sub(stale) >= NOT_FOUND_TTL_SECS);
    }
}
