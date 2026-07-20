use anyhow::Result;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::Mutex as TokioMutex;

const GITHUB_API: &str = "https://api.github.com";
pub(crate) const GITHUB_ACCOUNT: &str = "Project-Colony";
pub(crate) const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Owner/repo for the Colony launcher itself.
pub(crate) const LAUNCHER_OWNER: &str = "Project-Colony";
pub(crate) const LAUNCHER_REPO: &str = "Colony";

/// Default HTTP timeout for all GitHub API requests.
const HTTP_TIMEOUT: Duration = Duration::from_secs(30);
/// Default connect timeout.
pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Cap on concurrent per-repo fetches during a store refresh so a large org
/// cannot fire an unbounded burst of requests at the GitHub API at once.
const MAX_CONCURRENT_REPO_FETCHES: usize = 8;

use crate::persistence::save_repo_doc;
use crate::persistence::save_repo_icon;
use crate::persistence::{load_installed_version, load_repos_cache};

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
async fn cached_get(
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

/// Per-platform release info from colony.json.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseFileEntry {
    pub tag: String,
    /// Exact asset filename to download. Required unless `file_pattern` is set.
    pub file: Option<String>,
    /// Substring pattern to match against release asset names (case-insensitive).
    /// Colony fetches the release assets list and picks the one matching this pattern.
    /// Mutually exclusive with `file` — use one or the other.
    pub file_pattern: Option<String>,
    /// Optional binary name inside an archive. When present, the downloaded file
    /// is treated as an archive (.zip / .tar.gz) and Colony extracts this binary.
    /// When absent, the downloaded file is the final binary (legacy behaviour).
    pub binary: Option<String>,
    /// Optional SHA256 checksum for integrity verification.
    pub sha256: Option<String>,
}

/// Parsed manifest from colony.json inside a repo.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColonyManifest {
    pub name: String,
    pub category: String,
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub release_files: HashMap<String, ReleaseFileEntry>,
    /// Optional path (relative to the repo root) to a square PNG app icon shown
    /// in the Colony grid. When absent, Colony probes a conventional `icon.png`
    /// at the repo root, then falls back to the tinted category hexagon.
    #[serde(default)]
    pub icon: Option<String>,
    /// When true, every release asset MUST ship a valid `<asset>.sig`
    /// (ed25519, Project-Colony org key): a missing signature aborts the
    /// install instead of falling back to the legacy unsigned path.
    #[serde(default)]
    pub signed: bool,
}

/// Metadata for a Colony-compatible repository (has colony.json).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ColonyRepo {
    pub name: String,
    pub description: String,
    pub language: String,
    pub html_url: String,
    pub manifest: ColonyManifest,
}

#[derive(Debug, Deserialize)]
struct GithubRepo {
    name: String,
    description: Option<String>,
    language: Option<String>,
    html_url: String,
}

#[derive(Debug, Deserialize)]
struct GithubContent {
    name: String,
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubReadme {
    content: Option<String>,
}

/// Fetch all Colony repos (those containing colony.json) from Project-Colony.
/// Fetches manifests and READMEs concurrently for all repos.
pub async fn fetch_colony_repos(token: Option<&str>) -> Result<Vec<ColonyRepo>> {
    let client = build_client(token)?;

    // 1. List all repos for Project-Colony (with pagination)
    let repos = list_repos_paginated(&client).await?;

    // Track whether any repo failed for a transient reason (timeout, 5xx,
    // rate-limit) as opposed to genuinely lacking a colony.json (404). A
    // transient failure must not silently drop an installed app from the store
    // nor clobber the offline cache with a shortened list.
    let transient_failures: std::sync::Arc<std::sync::Mutex<std::collections::HashSet<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));

    // 2. Fetch manifest + README concurrently for all repos
    let futures: Vec<_> = repos
        .iter()
        .map(|repo| {
            let client = client.clone();
            let name = repo.name.clone();
            let fallback_desc = repo.description.clone().unwrap_or_default();
            let html_url = repo.html_url.clone();
            let language = repo.language.clone().unwrap_or_else(|| "Unknown".into());
            let transient_failures = transient_failures.clone();

            async move {
                let mut manifest = match fetch_colony_manifest(&client, &name).await {
                    Ok(Some(m)) => m,
                    Ok(None) => return None,
                    Err(e) => {
                        // fetch_colony_manifest already maps 404 to Ok(None),
                        // so an Err here is transient, not "no manifest".
                        tracing::warn!("Error checking colony.json for {}: {e}", name);
                        if let Ok(mut failed) = transient_failures.lock() {
                            failed.insert(name.clone());
                        }
                        return None;
                    }
                };

                // Auto-detect platforms from release assets if manifest is minimal
                if manifest.release_files.is_empty() {
                    if let Err(e) = auto_detect_release(&client, &name, &mut manifest).await {
                        tracing::debug!("Auto-detect skipped for {name}: {e}");
                    }
                }

                // Fetch README, LICENSE, CHANGELOG, icon concurrently
                let readme_fut = fetch_readme(&client, &name);
                let license_fut = fetch_license_with_fallback(&client, &name);
                let changelog_fut = fetch_repo_file_candidates(
                    &client,
                    &name,
                    &["CHANGELOG.md", "CHANGES.md", "CHANGELOG"],
                );
                let icon_fut = fetch_icon(&client, &name, manifest.icon.as_deref());

                let (readme_result, license_result, changelog_result, icon_result) =
                    futures::future::join4(readme_fut, license_fut, changelog_fut, icon_fut).await;

                let description = readme_result.unwrap_or(fallback_desc);

                // Save docs + icon to disk cache
                save_repo_doc(&name, "README.md", &description);
                if let Ok(Some(ref content)) = license_result {
                    save_repo_doc(&name, "LICENSE.md", content);
                }
                if let Ok(Some(ref content)) = changelog_result {
                    save_repo_doc(&name, "CHANGELOG.md", content);
                }
                if let Ok(Some(ref bytes)) = icon_result {
                    save_repo_icon(&name, bytes);
                }

                Some(ColonyRepo {
                    name,
                    description,
                    language,
                    html_url,
                    manifest,
                })
            }
        })
        .collect();

    // Cap concurrency (order-preserving) instead of firing every repo's fetch
    // chain at once.
    use futures::StreamExt;
    let results: Vec<Option<ColonyRepo>> = futures::stream::iter(futures)
        .buffered(MAX_CONCURRENT_REPO_FETCHES)
        .collect()
        .await;
    let mut repos_out: Vec<ColonyRepo> = results.into_iter().flatten().collect();

    // On a partially failed refresh, merge back ONLY the specific repos whose
    // fetch failed transiently. The old any-failure flag resurrected EVERY
    // cached repo, including ones genuinely deleted from the catalog.
    let failed = transient_failures
        .lock()
        .map(|s| s.clone())
        .unwrap_or_default();
    if !failed.is_empty() {
        if let Some(cached) = load_repos_cache() {
            for repo in cached {
                if failed.contains(&repo.name) && !repos_out.iter().any(|r| r.name == repo.name) {
                    repos_out.push(repo);
                }
            }
        }
    }

    Ok(repos_out)
}

/// Build an HTTP client for API calls (public wrapper).
pub fn build_update_client(token: Option<&str>) -> Result<reqwest::Client> {
    build_client(token)
}

fn build_client(token: Option<&str>) -> Result<reqwest::Client> {
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

/// List repos with pagination support (follows GitHub Link header).
async fn list_repos_paginated(client: &reqwest::Client) -> Result<Vec<GithubRepo>> {
    let mut all_repos = Vec::new();
    let mut page = 1u32;

    loop {
        let url = format!(
            "{GITHUB_API}/orgs/{GITHUB_ACCOUNT}/repos?per_page=100&sort=updated&page={page}"
        );
        let (body, _) = cached_get(client, &url).await?;
        let repos: Vec<GithubRepo> = serde_json::from_str(&body)?;

        if repos.is_empty() {
            break;
        }

        let count = repos.len();
        all_repos.extend(repos);

        // If we got fewer than 100, we've reached the last page
        if count < 100 {
            break;
        }

        page += 1;

        // Safety limit to prevent infinite loops
        if page > 50 {
            tracing::warn!("Pagination safety limit reached at page {page}");
            break;
        }
    }

    Ok(all_repos)
}

/// Fetch and parse colony.json from a repo. Returns None if the file doesn't exist.
async fn fetch_colony_manifest(
    client: &reqwest::Client,
    repo_name: &str,
) -> Result<Option<ColonyManifest>> {
    let url = format!("{GITHUB_API}/repos/{GITHUB_ACCOUNT}/{repo_name}/contents/colony.json");
    match cached_get(client, &url).await {
        Ok((body, _)) => {
            let content: GithubContent = serde_json::from_str(&body).map_err(|e| {
                anyhow::anyhow!("Failed to parse GitHub content response for {repo_name}: {e}")
            })?;
            if content.name != "colony.json" {
                return Ok(None);
            }
            // Decode Base64 content
            let raw = content.content.unwrap_or_default();
            let cleaned: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(&cleaned)
                .map_err(|e| {
                    anyhow::anyhow!("Failed to decode base64 for {repo_name}/colony.json: {e}")
                })?;
            let manifest: ColonyManifest = serde_json::from_slice(&bytes)
                .map_err(|e| anyhow::anyhow!("Invalid colony.json in {repo_name}: {e}"))?;
            Ok(Some(manifest))
        }
        Err(e) => {
            if is_not_found(&e) {
                Ok(None)
            } else {
                Err(e)
            }
        }
    }
}

/// Fetch the README content from a repo, returning the first ~500 chars as plain text.
async fn fetch_readme(client: &reqwest::Client, repo_name: &str) -> Result<String> {
    let url = format!("{GITHUB_API}/repos/{GITHUB_ACCOUNT}/{repo_name}/readme");
    let (body, _) = cached_get(client, &url).await?;
    let readme: GithubReadme = serde_json::from_str(&body)?;
    let raw = readme.content.unwrap_or_default();
    let cleaned: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = base64::engine::general_purpose::STANDARD.decode(&cleaned)?;
    let text = String::from_utf8_lossy(&bytes).trim().to_string();

    if text.is_empty() {
        anyhow::bail!("README is empty");
    }

    Ok(text)
}

/// Fetch the repo's LICENSE via GitHub's dedicated license endpoint — one
/// request that returns the detected license file, instead of probing several
/// candidate filenames (which cost one request each, mostly 404s).
async fn fetch_license(client: &reqwest::Client, repo_name: &str) -> Result<Option<String>> {
    let url = format!("{GITHUB_API}/repos/{GITHUB_ACCOUNT}/{repo_name}/license");
    match cached_get(client, &url).await {
        Ok((body, _)) => {
            let content: GithubContent = serde_json::from_str(&body)?;
            let raw = content.content.unwrap_or_default();
            let cleaned: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
            let bytes = base64::engine::general_purpose::STANDARD.decode(&cleaned)?;
            Ok(Some(String::from_utf8_lossy(&bytes).to_string()))
        }
        Err(e) => {
            if is_not_found(&e) {
                Ok(None)
            } else {
                Err(e)
            }
        }
    }
}

/// LICENSE for display: try the fast dedicated endpoint first, then fall back
/// to probing common filenames only if GitHub could not auto-classify one — so
/// a repo carrying a nonstandard/undetectable license file is still surfaced
/// while the common case stays a single request.
async fn fetch_license_with_fallback(
    client: &reqwest::Client,
    repo_name: &str,
) -> Result<Option<String>> {
    match fetch_license(client, repo_name).await {
        Ok(Some(content)) => Ok(Some(content)),
        Ok(None) => {
            fetch_repo_file_candidates(
                client,
                repo_name,
                &["LICENSE", "LICENSE.md", "LICENSE.txt", "COPYING"],
            )
            .await
        }
        Err(e) => Err(e),
    }
}

/// Fetch a file from a repo, trying multiple candidate paths.
/// Returns the decoded UTF-8 content of the first file found, or None if all return 404.
async fn fetch_repo_file_candidates(
    client: &reqwest::Client,
    repo_name: &str,
    candidates: &[&str],
) -> Result<Option<String>> {
    for path in candidates {
        let url = format!("{GITHUB_API}/repos/{GITHUB_ACCOUNT}/{repo_name}/contents/{path}");
        match cached_get(client, &url).await {
            Ok((body, _)) => {
                let content: GithubContent = serde_json::from_str(&body)?;
                let raw = content.content.unwrap_or_default();
                let cleaned: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
                let bytes = base64::engine::general_purpose::STANDARD.decode(&cleaned)?;
                let text = String::from_utf8_lossy(&bytes).to_string();
                return Ok(Some(text));
            }
            Err(e) => {
                if is_not_found(&e) {
                    continue;
                }
                return Err(e);
            }
        }
    }
    Ok(None)
}

/// Fetch the app icon bytes from a repo: the manifest-declared `icon` path
/// first, then a conventional `icon.png` at the repo root. Returns the raw PNG
/// bytes of the first that exists, or None if neither is present (404).
async fn fetch_icon(
    client: &reqwest::Client,
    repo_name: &str,
    declared: Option<&str>,
) -> Result<Option<Vec<u8>>> {
    let mut candidates: Vec<&str> = Vec::new();
    if let Some(p) = declared {
        candidates.push(p);
    }
    if !candidates.contains(&"icon.png") {
        candidates.push("icon.png");
    }
    for path in candidates {
        let url = format!("{GITHUB_API}/repos/{GITHUB_ACCOUNT}/{repo_name}/contents/{path}");
        match cached_get(client, &url).await {
            Ok((body, _)) => {
                let content: GithubContent = serde_json::from_str(&body)?;
                let raw = content.content.unwrap_or_default();
                let cleaned: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
                let bytes = base64::engine::general_purpose::STANDARD.decode(&cleaned)?;
                if !bytes.is_empty() {
                    return Ok(Some(bytes));
                }
            }
            Err(e) => {
                if is_not_found(&e) {
                    continue;
                }
                return Err(e);
            }
        }
    }
    Ok(None)
}

/// Return the current platform key ("windows", "linux", "macos", or "macos-x86").
/// On macOS, distinguishes Apple Silicon (aarch64 → "macos") from Intel (x86_64 → "macos-x86").
pub fn current_platform_key() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "macos"
        } else {
            "macos-x86"
        }
    } else {
        "linux"
    }
}

/// Fetch the latest release tag for an arbitrary owner/repo combination.
pub async fn fetch_latest_release_tag_for(
    client: &reqwest::Client,
    owner: &str,
    repo: &str,
) -> Result<String> {
    let url = format!("{GITHUB_API}/repos/{owner}/{repo}/releases/latest");
    let (body, _) = cached_get(client, &url).await?;

    #[derive(Deserialize)]
    struct Release {
        tag_name: String,
    }

    let release: Release = serde_json::from_str(&body)?;
    Ok(release.tag_name)
}

/// Fetch the latest release tag for a Colony app repo.
pub async fn fetch_latest_release_tag(client: &reqwest::Client, repo_name: &str) -> Result<String> {
    fetch_latest_release_tag_for(client, GITHUB_ACCOUNT, repo_name).await
}

/// Resolved release information from GitHub API.
#[derive(Debug)]
pub struct ResolvedRelease {
    pub tag: String,
    pub asset_names: Vec<String>,
    /// The release notes (GitHub release body, markdown). Previously never
    /// fetched anywhere: the detail Changelog tab only showed the repo's
    /// CHANGELOG.md file frozen at catalog-fetch time.
    pub body: Option<String>,
}

/// Fetch release info (tag + asset list) for a repo.
/// If tag is "latest", resolves to the actual latest release.
/// Otherwise fetches the specific tagged release.
pub async fn fetch_release_info(
    client: &reqwest::Client,
    repo_name: &str,
    tag: &str,
) -> Result<ResolvedRelease> {
    let url = if tag.eq_ignore_ascii_case("latest") {
        format!("{GITHUB_API}/repos/{GITHUB_ACCOUNT}/{repo_name}/releases/latest")
    } else {
        format!("{GITHUB_API}/repos/{GITHUB_ACCOUNT}/{repo_name}/releases/tags/{tag}")
    };
    let (body, _) = cached_get(client, &url).await?;

    #[derive(Deserialize)]
    struct Asset {
        name: String,
    }
    #[derive(Deserialize)]
    struct Release {
        tag_name: String,
        assets: Vec<Asset>,
        body: Option<String>,
    }

    let release: Release = serde_json::from_str(&body)?;
    Ok(ResolvedRelease {
        tag: release.tag_name,
        asset_names: release.assets.into_iter().map(|a| a.name).collect(),
        body: release.body,
    })
}

/// Find an asset whose name contains the given pattern (case-insensitive).
/// Returns an error if zero or multiple assets match.
/// Metadata companions published alongside release binaries (signatures,
/// checksums, updater manifests). Never installable, so they are excluded from
/// pattern matching - otherwise `app-linux.sig` would make the pattern
/// "linux" ambiguous the day a repo starts signing its releases (Colony's own
/// releases already ship `.sig` siblings).
const NON_INSTALLABLE_SUFFIXES: &[&str] = &[
    ".sig",
    ".asc",
    ".sha256",
    ".sha256sum",
    ".txt",
    ".yml",
    ".yaml",
    ".json",
];

/// Anchored glob match: `*` matches any run of characters, everything else is
/// literal (case-insensitive - both inputs must already be lowercase). The
/// pattern must cover the WHOLE name, unlike the legacy substring mode.
fn glob_matches(pattern: &str, name: &str) -> bool {
    fn inner(p: &[u8], n: &[u8]) -> bool {
        match (p.first(), n.first()) {
            (None, None) => true,
            (Some(b'*'), _) => {
                // Star: match zero characters, or consume one and retry.
                inner(&p[1..], n) || (!n.is_empty() && inner(p, &n[1..]))
            }
            (Some(pc), Some(nc)) if pc == nc => inner(&p[1..], &n[1..]),
            _ => false,
        }
    }
    inner(pattern.as_bytes(), name.as_bytes())
}

/// Resolve a `filePattern` against release asset names.
///
/// Three matching modes, so real-world release layouts (e.g. electron-builder
/// publishing `App-1.2.3.AppImage` AND `App-1.2.3-arm64.AppImage`) stay
/// expressible:
/// - exact name match always wins (never ambiguous);
/// - a pattern containing `*` is an ANCHORED glob; comma-separated terms are
///   supported, where `!term` excludes: `"*.AppImage, !*-arm64*"`;
/// - otherwise the legacy case-insensitive substring match applies.
///
/// Signature/checksum siblings (`.sig`, `.sha256`, ...) are never candidates.
pub fn find_asset_by_pattern(assets: &[String], pattern: &str) -> Result<String> {
    let pattern_lower = pattern.to_lowercase();
    if let Some(exact) = assets.iter().find(|n| n.to_lowercase() == pattern_lower) {
        return Ok(exact.clone());
    }

    let terms: Vec<&str> = pattern_lower
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect();
    let has_glob = terms.iter().any(|t| t.contains('*') || t.starts_with('!'));
    let positives: Vec<&str> = terms
        .iter()
        .filter(|t| !t.starts_with('!'))
        .copied()
        .collect();
    let negatives: Vec<&str> = terms.iter().filter_map(|t| t.strip_prefix('!')).collect();

    let matches: Vec<&String> = assets
        .iter()
        .filter(|name| {
            let lower = name.to_lowercase();
            if NON_INSTALLABLE_SUFFIXES.iter().any(|s| lower.ends_with(s)) {
                return false;
            }
            if has_glob {
                positives.iter().any(|p| glob_matches(p, &lower))
                    && !negatives.iter().any(|n| glob_matches(n, &lower))
            } else {
                lower.contains(&pattern_lower)
            }
        })
        .collect();
    match matches.len() {
        0 => anyhow::bail!("No release asset matching pattern '{pattern}'"),
        1 => Ok(matches[0].clone()),
        n => {
            anyhow::bail!(
            "Ambiguous pattern '{pattern}': {n} assets match ({}). Use a more specific pattern.",
            matches.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
        )
        }
    }
}

/// Platform detection entries: (expected asset suffix, platform key).
/// Order matters: "macos-x86" must come before "macos" to avoid false matches.
const PLATFORM_CONVENTIONS: &[(&str, &str)] = &[
    ("-linux", "linux"),
    ("-windows.exe", "windows"),
    ("-macos-x86", "macos-x86"),
    ("-macos", "macos"),
];

/// Detect which platforms are available from release asset names using the
/// Colony naming convention: `{name}-linux`, `{name}-windows.exe`,
/// `{name}-macos`, `{name}-macos-x86`.
pub fn detect_platforms_from_assets(repo_name: &str, asset_names: &[String]) -> Vec<String> {
    let repo_lower = repo_name.to_lowercase();
    let mut platforms = Vec::new();

    for &(suffix, platform) in PLATFORM_CONVENTIONS {
        let expected = format!("{repo_lower}{suffix}");
        if asset_names.iter().any(|a| a.to_lowercase() == expected) {
            platforms.push(platform.to_string());
        }
    }

    platforms
}

/// Build a `release_files` HashMap from detected assets, using the "latest" tag
/// and convention-based filenames. Uses the exact asset name found in the release.
pub fn build_release_files_from_assets(
    repo_name: &str,
    asset_names: &[String],
) -> HashMap<String, ReleaseFileEntry> {
    let repo_lower = repo_name.to_lowercase();
    let mut map = HashMap::new();

    for &(suffix, platform) in PLATFORM_CONVENTIONS {
        let expected = format!("{repo_lower}{suffix}");
        if let Some(actual_name) = asset_names.iter().find(|a| a.to_lowercase() == expected) {
            map.insert(
                platform.to_string(),
                ReleaseFileEntry {
                    tag: "latest".to_string(),
                    file: Some(actual_name.clone()),
                    file_pattern: None,
                    binary: None,
                    sha256: None,
                },
            );
        }
    }

    map
}

/// For a repo with empty platforms/release_files (minimal colony.json), fetch the
/// latest release and auto-detect available platforms from its assets.
pub async fn auto_detect_release(
    client: &reqwest::Client,
    repo_name: &str,
    manifest: &mut ColonyManifest,
) -> Result<()> {
    let release = fetch_release_info(client, repo_name, "latest").await?;
    let platforms = detect_platforms_from_assets(repo_name, &release.asset_names);
    let release_files = build_release_files_from_assets(repo_name, &release.asset_names);

    if !platforms.is_empty() {
        tracing::info!("Auto-detected platforms for {repo_name}: {:?}", platforms);
        manifest.platforms = platforms;
        manifest.release_files = release_files;
    }

    Ok(())
}

/// Parse a version tag (e.g. "v1.2.3" or "1.2.3") into a semver::Version.
pub fn parse_version_tag(tag: &str) -> Option<semver::Version> {
    let cleaned = tag.strip_prefix('v').unwrap_or(tag);
    semver::Version::parse(cleaned).ok()
}

/// Check if an update is available for a repo whose manifest pins `pinned_tag`
/// for the current platform. Returns Some(target_tag) if the installed version
/// differs from what the manifest would install, None otherwise.
///
/// `pinned_tag` is compared directly unless it is "latest", in which case the
/// repo's latest release is resolved. This avoids a perpetual "update
/// available" loop for apps pinned to a specific (older) release, and falls
/// back to string comparison when tags are not semver so detection is not
/// silently disabled.
pub async fn check_update_available(
    client: &reqwest::Client,
    repo_name: &str,
    pinned_tag: &str,
) -> Option<String> {
    let installed = load_installed_version(repo_name)?;

    let target = if pinned_tag.eq_ignore_ascii_case("latest") {
        fetch_latest_release_tag(client, repo_name).await.ok()?
    } else {
        pinned_tag.to_string()
    };

    // Case-insensitive: "Nightly" vs "nightly" must not read as an update
    // (with non-semver tags the string fallback below would flag it forever).
    if target.eq_ignore_ascii_case(&installed) {
        return None;
    }

    match (parse_version_tag(&installed), parse_version_tag(&target)) {
        (Some(installed_ver), Some(target_ver)) => {
            if target_ver > installed_ver {
                Some(target)
            } else {
                None
            }
        }
        _ => {
            tracing::warn!(
                "Non-semver tags for {repo_name} (installed '{installed}', target '{target}'); using string comparison"
            );
            Some(target)
        }
    }
}

// --- Launcher self-update ---

/// Expected release asset name for the Colony launcher binary on the current platform.
pub fn launcher_asset_name() -> String {
    let platform = current_platform_key();
    let ext = if cfg!(target_os = "windows") {
        ".exe"
    } else {
        ""
    };
    format!("colony-{platform}{ext}")
}

/// Check if a newer version of the Colony launcher itself is available.
/// Returns Some((latest_tag, asset_filename)) if an update exists, None otherwise.
/// `Ok(None)` means the check RAN and Colony is current; failures propagate so
/// the UI never reports "up to date" when the check could not run at all
/// (offline, rate limited, or an unparseable release tag).
pub async fn check_launcher_update(client: &reqwest::Client) -> Result<Option<(String, String)>> {
    let latest_tag = fetch_latest_release_tag_for(client, LAUNCHER_OWNER, LAUNCHER_REPO).await?;

    let current = parse_version_tag(APP_VERSION)
        .ok_or_else(|| anyhow::anyhow!("unparseable app version '{APP_VERSION}'"))?;
    let latest = parse_version_tag(&latest_tag)
        .ok_or_else(|| anyhow::anyhow!("unrecognized release tag '{latest_tag}'"))?;

    Ok((latest > current).then(|| (latest_tag, launcher_asset_name())))
}

// --- Offline cache ---

// --- Favorites persistence ---

// --- User preferences persistence ---

// --- Application scan cache ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_colony_manifest() {
        let json = r#"{
            "name": "TestApp",
            "category": "Utilities",
            "platforms": ["windows", "linux"],
            "releaseFiles": {
                "windows": { "tag": "Windows", "file": "TestApp.exe" },
                "linux": { "tag": "Linux", "file": "TestApp" }
            }
        }"#;
        let manifest: ColonyManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.name, "TestApp");
        assert_eq!(manifest.category, "Utilities");
        assert_eq!(manifest.platforms, vec!["windows", "linux"]);
        assert_eq!(manifest.release_files.len(), 2);
        assert_eq!(manifest.release_files["windows"].tag, "Windows");
        assert_eq!(
            manifest.release_files["windows"].file.as_deref(),
            Some("TestApp.exe")
        );
        assert_eq!(manifest.release_files["linux"].tag, "Linux");
        assert_eq!(
            manifest.release_files["linux"].file.as_deref(),
            Some("TestApp")
        );
    }

    #[test]
    fn parse_colony_manifest_with_sha256() {
        let json = r#"{
            "name": "TestApp",
            "category": "Utilities",
            "platforms": ["linux"],
            "releaseFiles": {
                "linux": { "tag": "v1.0", "file": "app", "sha256": "abc123def456" }
            }
        }"#;
        let manifest: ColonyManifest = serde_json::from_str(json).unwrap();
        assert_eq!(
            manifest.release_files["linux"].sha256.as_deref(),
            Some("abc123def456")
        );
    }

    #[test]
    fn parse_colony_manifest_with_binary_and_latest() {
        let json = r#"{
            "name": "Lilypad",
            "category": "Security",
            "platforms": ["windows", "linux", "macos"],
            "releaseFiles": {
                "windows": {
                    "tag": "latest",
                    "file": "lilypad-x86_64-pc-windows-msvc.zip",
                    "binary": "lilypad-cli.exe",
                    "sha256": "abc123"
                },
                "linux": {
                    "tag": "latest",
                    "file": "lilypad-x86_64-unknown-linux-gnu.tar.gz",
                    "binary": "lilypad-cli"
                },
                "macos": {
                    "tag": "v0.1.0",
                    "file": "lilypad-aarch64-apple-darwin.tar.gz",
                    "binary": "lilypad-cli"
                }
            }
        }"#;
        let manifest: ColonyManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.name, "Lilypad");
        assert_eq!(manifest.platforms.len(), 3);
        // Windows: archive + binary + latest
        let win = &manifest.release_files["windows"];
        assert_eq!(win.tag, "latest");
        assert_eq!(
            win.file.as_deref(),
            Some("lilypad-x86_64-pc-windows-msvc.zip")
        );
        assert_eq!(win.binary.as_deref(), Some("lilypad-cli.exe"));
        assert_eq!(win.sha256.as_deref(), Some("abc123"));
        // Linux: archive + binary + latest, no sha256
        let linux = &manifest.release_files["linux"];
        assert_eq!(linux.tag, "latest");
        assert_eq!(linux.binary.as_deref(), Some("lilypad-cli"));
        assert!(linux.sha256.is_none());
        // macOS: pinned tag
        let macos = &manifest.release_files["macos"];
        assert_eq!(macos.tag, "v0.1.0");
        assert_eq!(macos.binary.as_deref(), Some("lilypad-cli"));
    }

    #[test]
    fn parse_colony_manifest_binary_absent() {
        // Legacy format without binary field still works
        let json = r#"{
            "name": "TestApp",
            "category": "Utilities",
            "platforms": ["windows"],
            "releaseFiles": {
                "windows": { "tag": "Windows", "file": "TestApp.exe" }
            }
        }"#;
        let manifest: ColonyManifest = serde_json::from_str(json).unwrap();
        assert!(manifest.release_files["windows"].binary.is_none());
    }

    #[test]
    fn parse_colony_manifest_with_file_pattern() {
        let json = r#"{
            "name": "Lilypad",
            "category": "Security",
            "platforms": ["windows", "linux", "macos"],
            "releaseFiles": {
                "windows": {
                    "tag": "latest",
                    "filePattern": "windows",
                    "binary": "lilypad-cli.exe"
                },
                "linux": {
                    "tag": "latest",
                    "filePattern": "linux",
                    "binary": "lilypad-cli"
                },
                "macos": {
                    "tag": "latest",
                    "filePattern": "darwin",
                    "binary": "lilypad-cli"
                }
            }
        }"#;
        let manifest: ColonyManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.name, "Lilypad");
        // Windows: filePattern instead of file
        let win = &manifest.release_files["windows"];
        assert_eq!(win.tag, "latest");
        assert!(win.file.is_none());
        assert_eq!(win.file_pattern.as_deref(), Some("windows"));
        assert_eq!(win.binary.as_deref(), Some("lilypad-cli.exe"));
        // Linux
        let linux = &manifest.release_files["linux"];
        assert_eq!(linux.file_pattern.as_deref(), Some("linux"));
        // macOS
        let macos = &manifest.release_files["macos"];
        assert_eq!(macos.file_pattern.as_deref(), Some("darwin"));
    }

    #[test]
    fn find_asset_by_pattern_single_match() {
        let assets = vec![
            "lilypad-x86_64-pc-windows-msvc.zip".to_string(),
            "lilypad-x86_64-unknown-linux-gnu.tar.gz".to_string(),
            "lilypad-aarch64-apple-darwin.tar.gz".to_string(),
        ];
        let result = find_asset_by_pattern(&assets, "windows");
        assert_eq!(result.unwrap(), "lilypad-x86_64-pc-windows-msvc.zip");

        let result = find_asset_by_pattern(&assets, "linux");
        assert_eq!(result.unwrap(), "lilypad-x86_64-unknown-linux-gnu.tar.gz");

        let result = find_asset_by_pattern(&assets, "darwin");
        assert_eq!(result.unwrap(), "lilypad-aarch64-apple-darwin.tar.gz");
    }

    #[test]
    fn find_asset_by_pattern_case_insensitive() {
        let assets = vec!["MyApp-Windows-x64.zip".to_string()];
        let result = find_asset_by_pattern(&assets, "windows");
        assert!(result.is_ok());
    }

    #[test]
    fn find_asset_by_pattern_no_match() {
        let assets = vec!["app-linux.tar.gz".to_string()];
        let result = find_asset_by_pattern(&assets, "windows");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No release asset"));
    }

    #[test]
    fn find_asset_by_pattern_ambiguous() {
        let assets = vec![
            "app-linux-x64.tar.gz".to_string(),
            "app-linux-arm64.tar.gz".to_string(),
        ];
        let result = find_asset_by_pattern(&assets, "linux");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Ambiguous"));
    }

    #[test]
    fn is_not_found_matches_typed_status_not_message_text() {
        // A 404 is recognized through the typed status...
        let e404 = anyhow::Error::new(HttpStatus(404)).context("GitHub API error 404: Not Found");
        assert!(is_not_found(&e404));
        // ...a different status is not, even if its BODY contains "404" (the
        // old substring check misclassified this and dropped live repos).
        let e500 = anyhow::Error::new(HttpStatus(500))
            .context("GitHub API error 500: upstream said 404 somewhere");
        assert!(!is_not_found(&e500));
        // ...and a plain network error without a status is not a 404 either.
        assert!(!is_not_found(&anyhow::anyhow!("Network error: dns 404ish")));
    }

    #[test]
    fn find_asset_by_pattern_exact_name_beats_substring_overlap() {
        // "app-macos" is a substring of "app-macos-x86": an exact name match
        // must win instead of erroring as ambiguous, so Apple Silicon
        // manifests can pin the shorter asset name.
        let assets = vec!["app-macos".to_string(), "app-macos-x86".to_string()];
        assert_eq!(
            find_asset_by_pattern(&assets, "app-macos").unwrap(),
            "app-macos"
        );
        assert_eq!(
            find_asset_by_pattern(&assets, "app-macos-x86").unwrap(),
            "app-macos-x86"
        );
    }

    #[test]
    fn spec_conformant_manifest_parses_field_for_field() {
        // Locks docs/colony-spec.md <-> code parity: this sample uses every
        // documented manifest field with the spec's exact camelCase names.
        // If a rename or removal breaks the spec, this test fails first.
        let json = r#"{
            "name": "Lilypad",
            "category": "Security",
            "platforms": ["windows", "linux", "macos", "macos-x86"],
            "icon": "assets/icons/icon.png",
            "signed": true,
            "releaseFiles": {
                "linux": {
                    "tag": "latest",
                    "filePattern": "lilypad-*-linux.tar.gz, !*-arm64*",
                    "binary": "lilypad-cli",
                    "sha256": "abc123"
                },
                "windows": {
                    "tag": "v1.0.0",
                    "file": "lilypad-windows.zip",
                    "binary": "lilypad-cli.exe"
                }
            }
        }"#;
        let m: ColonyManifest = serde_json::from_str(json).expect("spec sample must parse");
        assert_eq!(m.name, "Lilypad");
        assert_eq!(m.category, "Security");
        assert_eq!(m.platforms.len(), 4);
        assert_eq!(m.icon.as_deref(), Some("assets/icons/icon.png"));
        assert!(m.signed);
        let linux = &m.release_files["linux"];
        assert_eq!(linux.tag, "latest");
        assert_eq!(
            linux.file_pattern.as_deref(),
            Some("lilypad-*-linux.tar.gz, !*-arm64*")
        );
        assert_eq!(linux.binary.as_deref(), Some("lilypad-cli"));
        assert_eq!(linux.sha256.as_deref(), Some("abc123"));
        let windows = &m.release_files["windows"];
        assert_eq!(windows.tag, "v1.0.0");
        assert_eq!(windows.file.as_deref(), Some("lilypad-windows.zip"));
        // Every spec category value (and its documented aliases) maps to a
        // real category - never silently to Other (except Other itself).
        for cat in [
            "Development",
            "Graphics",
            "Network",
            "Office",
            "Multimedia",
            "System",
            "Utility",
            "Utilities",
            "Security",
            "Game",
            "Games",
        ] {
            assert_ne!(
                crate::scan::AppCategory::from_name(cat),
                crate::scan::AppCategory::Other,
                "spec category '{cat}' must not fall back to Other"
            );
        }
    }

    #[test]
    fn manifest_signed_flag_parses_and_defaults_off() {
        let json = r#"{ "name": "App", "category": "Utility", "signed": true }"#;
        let m: ColonyManifest = serde_json::from_str(json).unwrap();
        assert!(m.signed);
        let json = r#"{ "name": "App", "category": "Utility" }"#;
        let m: ColonyManifest = serde_json::from_str(json).unwrap();
        assert!(!m.signed, "signed must default to false (legacy manifests)");
    }

    #[test]
    fn find_asset_by_pattern_glob_with_exclusion_resolves_electron_builder_layout() {
        // SphereCord's real release layout: electron-builder publishes both
        // architectures plus updater metadata. Substring matching could never
        // express this; an anchored glob with an exclusion can.
        let assets = vec![
            "SphereCord-3.2.7.AppImage".to_string(),
            "SphereCord-3.2.7-arm64.AppImage".to_string(),
            "SphereCord-Setup-3.2.7.exe".to_string(),
            "latest-linux.yml".to_string(),
            "spherecord-3.2.7.tar.gz".to_string(),
        ];
        assert_eq!(
            find_asset_by_pattern(&assets, "spherecord-*.appimage, !*-arm64*").unwrap(),
            "SphereCord-3.2.7.AppImage"
        );
        assert_eq!(
            find_asset_by_pattern(&assets, "*-arm64.appimage").unwrap(),
            "SphereCord-3.2.7-arm64.AppImage"
        );
    }

    #[test]
    fn find_asset_by_pattern_glob_is_anchored() {
        let assets = vec!["app-linux".to_string(), "app-linux-musl".to_string()];
        // Anchored: "*-linux" must NOT match "app-linux-musl".
        assert_eq!(
            find_asset_by_pattern(&assets, "*-linux").unwrap(),
            "app-linux"
        );
    }

    #[test]
    fn find_asset_by_pattern_ignores_signature_and_checksum_siblings() {
        // The day a repo signs its releases (like Colony itself), every binary
        // grows a .sig sibling containing the same name: the pattern must
        // keep resolving to the binary, not error as ambiguous.
        let assets = vec![
            "app-linux".to_string(),
            "app-linux.sig".to_string(),
            "app-linux.sha256".to_string(),
            "latest-linux.yml".to_string(),
        ];
        assert_eq!(
            find_asset_by_pattern(&assets, "linux").unwrap(),
            "app-linux"
        );
    }

    #[test]
    fn parse_colony_manifest_missing_required_field() {
        // category is required, so missing it should fail
        let json = r#"{ "name": "TestApp" }"#;
        let result: Result<ColonyManifest, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn colony_manifest_minimal_deserialize() {
        // platforms and release_files are optional (serde default)
        let json = r#"{ "name": "orCAL", "category": "Utilities" }"#;
        let manifest: ColonyManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.name, "orCAL");
        assert_eq!(manifest.category, "Utilities");
        assert!(manifest.platforms.is_empty());
        assert!(manifest.release_files.is_empty());
    }

    #[test]
    fn current_platform_key_is_valid() {
        let key = current_platform_key();
        assert!(
            key == "windows" || key == "linux" || key == "macos" || key == "macos-x86",
            "unexpected platform key: {key}"
        );
    }

    #[test]
    fn base64_decode_manifest() {
        let json = r#"{"name":"Test","category":"Games","platforms":["linux"],"releaseFiles":{"linux":{"tag":"v1","file":"test"}}}"#;
        let encoded = base64::engine::general_purpose::STANDARD.encode(json);
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&encoded)
            .unwrap();
        let manifest: ColonyManifest = serde_json::from_slice(&decoded).unwrap();
        assert_eq!(manifest.name, "Test");
        assert_eq!(manifest.category, "Games");
    }

    #[test]
    fn parse_version_tag_with_v_prefix() {
        let v = parse_version_tag("v1.2.3").unwrap();
        assert_eq!(v, semver::Version::new(1, 2, 3));
    }

    #[test]
    fn parse_version_tag_without_prefix() {
        let v = parse_version_tag("2.0.0").unwrap();
        assert_eq!(v, semver::Version::new(2, 0, 0));
    }

    #[test]
    fn parse_version_tag_invalid() {
        assert!(parse_version_tag("not-a-version").is_none());
    }

    #[test]
    fn version_comparison() {
        let old = parse_version_tag("v1.0.0").unwrap();
        let new = parse_version_tag("v1.1.0").unwrap();
        assert!(new > old);
    }

    #[test]
    fn detect_platforms_convention_naming() {
        let assets = vec![
            "orcal-linux".to_string(),
            "orcal-windows.exe".to_string(),
            "orcal-macos".to_string(),
        ];
        let platforms = detect_platforms_from_assets("orcal", &assets);
        assert_eq!(platforms, vec!["linux", "windows", "macos"]);
    }

    #[test]
    fn detect_platforms_with_x86() {
        let assets = vec![
            "myapp-linux".to_string(),
            "myapp-macos".to_string(),
            "myapp-macos-x86".to_string(),
        ];
        let platforms = detect_platforms_from_assets("myapp", &assets);
        assert!(platforms.contains(&"linux".to_string()));
        assert!(platforms.contains(&"macos".to_string()));
        assert!(platforms.contains(&"macos-x86".to_string()));
    }

    #[test]
    fn detect_platforms_empty_assets() {
        let assets: Vec<String> = vec![];
        let platforms = detect_platforms_from_assets("myapp", &assets);
        assert!(platforms.is_empty());
    }

    #[test]
    fn detect_platforms_case_insensitive() {
        let assets = vec!["MyApp-Linux".to_string()];
        let platforms = detect_platforms_from_assets("MyApp", &assets);
        assert_eq!(platforms, vec!["linux"]);
    }

    #[test]
    fn build_release_files_creates_entries() {
        let assets = vec!["orcal-linux".to_string(), "orcal-windows.exe".to_string()];
        let files = build_release_files_from_assets("orcal", &assets);
        assert_eq!(files.len(), 2);

        let linux = files.get("linux").unwrap();
        assert_eq!(linux.tag, "latest");
        assert_eq!(linux.file.as_deref(), Some("orcal-linux"));
        assert!(linux.file_pattern.is_none());
        assert!(linux.binary.is_none());

        let win = files.get("windows").unwrap();
        assert_eq!(win.tag, "latest");
        assert_eq!(win.file.as_deref(), Some("orcal-windows.exe"));
    }
}
