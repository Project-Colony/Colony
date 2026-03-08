use anyhow::Result;
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tokio::sync::Mutex as TokioMutex;

const GITHUB_API: &str = "https://api.github.com";
const GITHUB_ACCOUNT: &str = "Project-Colony";
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Owner/repo for the Colony launcher itself.
const LAUNCHER_OWNER: &str = "Project-Colony";
const LAUNCHER_REPO: &str = "Colony";

/// Default HTTP timeout for all GitHub API requests.
const HTTP_TIMEOUT: Duration = Duration::from_secs(30);
/// Default connect timeout.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

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
        if rl.remaining == 0 {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            if rl.reset > now {
                let wait = rl.reset - now;
                anyhow::bail!(
                    "{}",
                    crate::i18n::t_fmt("github_rate_limit", &[("wait", &wait.to_string())])
                );
            }
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
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("GitHub API error {status}: {body}");
        }
    }
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

    // 2. Fetch manifest + README concurrently for all repos
    let futures: Vec<_> = repos
        .iter()
        .map(|repo| {
            let client = client.clone();
            let name = repo.name.clone();
            let fallback_desc = repo.description.clone().unwrap_or_default();
            let html_url = repo.html_url.clone();
            let language = repo.language.clone().unwrap_or_else(|| "Unknown".into());

            async move {
                let mut manifest = match fetch_colony_manifest(&client, &name).await {
                    Ok(Some(m)) => m,
                    Ok(None) => return None,
                    Err(e) => {
                        tracing::warn!("Error checking colony.json for {}: {e}", name);
                        return None;
                    }
                };

                // Auto-detect platforms from release assets if manifest is minimal
                if manifest.release_files.is_empty() {
                    if let Err(e) = auto_detect_release(&client, &name, &mut manifest).await {
                        tracing::debug!("Auto-detect skipped for {name}: {e}");
                    }
                }

                // Fetch README, LICENSE, CHANGELOG concurrently
                let readme_fut = fetch_readme(&client, &name);
                let license_fut = fetch_repo_file_candidates(
                    &client, &name, &["LICENSE", "LICENSE.md", "LICENSE.txt"],
                );
                let changelog_fut = fetch_repo_file_candidates(
                    &client, &name, &["CHANGELOG.md", "CHANGES.md", "CHANGELOG"],
                );

                let (readme_result, license_result, changelog_result) =
                    futures::future::join3(readme_fut, license_fut, changelog_fut).await;

                let description = readme_result.unwrap_or(fallback_desc);

                // Save docs to disk cache
                save_repo_doc(&name, "README.md", &description);
                if let Ok(Some(ref content)) = license_result {
                    save_repo_doc(&name, "LICENSE.md", content);
                }
                if let Ok(Some(ref content)) = changelog_result {
                    save_repo_doc(&name, "CHANGELOG.md", content);
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

    let results = futures::future::join_all(futures).await;
    Ok(results.into_iter().flatten().collect())
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
    let url = format!(
        "{GITHUB_API}/repos/{GITHUB_ACCOUNT}/{repo_name}/contents/colony.json"
    );
    match cached_get(client, &url).await {
        Ok((body, _)) => {
            let content: GithubContent = serde_json::from_str(&body)
                .map_err(|e| anyhow::anyhow!("Failed to parse GitHub content response for {repo_name}: {e}"))?;
            if content.name != "colony.json" {
                return Ok(None);
            }
            // Decode Base64 content
            let raw = content.content.unwrap_or_default();
            let cleaned: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
            let bytes = base64::engine::general_purpose::STANDARD.decode(&cleaned)
                .map_err(|e| anyhow::anyhow!("Failed to decode base64 for {repo_name}/colony.json: {e}"))?;
            let manifest: ColonyManifest = serde_json::from_slice(&bytes)
                .map_err(|e| anyhow::anyhow!("Invalid colony.json in {repo_name}: {e}"))?;
            Ok(Some(manifest))
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("404") {
                Ok(None)
            } else {
                Err(e)
            }
        }
    }
}

/// Fetch the README content from a repo, returning the first ~500 chars as plain text.
async fn fetch_readme(client: &reqwest::Client, repo_name: &str) -> Result<String> {
    let url = format!(
        "{GITHUB_API}/repos/{GITHUB_ACCOUNT}/{repo_name}/readme"
    );
    let (body, _) = cached_get(client, &url).await?;
    let readme: GithubReadme = serde_json::from_str(&body)?;
    let raw = readme.content.unwrap_or_default();
    let cleaned: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = base64::engine::general_purpose::STANDARD.decode(&cleaned)?;
    let text = String::from_utf8_lossy(&bytes);

    // Strip markdown headings/images, keep line structure, limit to 1000 chars
    let plain: String = text
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with('#') && !trimmed.starts_with("![") && !trimmed.starts_with("```")
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();

    let truncated = if plain.len() > 1000 {
        format!("{}…", &plain[..plain.floor_char_boundary(1000)])
    } else {
        plain
    };

    if truncated.is_empty() {
        anyhow::bail!("README is empty");
    }

    Ok(truncated)
}

/// Fetch a file from a repo, trying multiple candidate paths.
/// Returns the decoded UTF-8 content of the first file found, or None if all return 404.
async fn fetch_repo_file_candidates(
    client: &reqwest::Client,
    repo_name: &str,
    candidates: &[&str],
) -> Result<Option<String>> {
    for path in candidates {
        let url = format!(
            "{GITHUB_API}/repos/{GITHUB_ACCOUNT}/{repo_name}/contents/{path}"
        );
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
                if e.to_string().contains("404") {
                    continue;
                }
                return Err(e);
            }
        }
    }
    Ok(None)
}

/// Central data directory for all Colony files: `~/.config/Colony/Colony/`
pub fn colony_data_dir() -> Result<PathBuf> {
    let base = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("No config directory"))?
        .join("Colony")
        .join("Colony");
    std::fs::create_dir_all(&base)?;
    Ok(base)
}

/// Directory for cached repo documentation files: `~/.config/Colony/Colony/repo-docs/{repo_name}/`
fn repo_docs_dir(repo_name: &str) -> Result<PathBuf> {
    let base = colony_data_dir()?.join("repo-docs").join(repo_name);
    std::fs::create_dir_all(&base)?;
    Ok(base)
}

/// Save a document to disk cache.
fn save_repo_doc(repo_name: &str, filename: &str, content: &str) {
    if let Ok(dir) = repo_docs_dir(repo_name) {
        let _ = std::fs::write(dir.join(filename), content);
    }
}

/// Read a cached document from disk. Returns None if file doesn't exist.
pub fn read_repo_doc(repo_name: &str, filename: &str) -> Option<String> {
    let dir = repo_docs_dir(repo_name).ok()?;
    std::fs::read_to_string(dir.join(filename)).ok()
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

/// Return the Colony apps directory: `<data_local>/Colony/apps/`
pub fn colony_apps_dir() -> Result<PathBuf> {
    let base = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine local data directory"))?;
    Ok(base.join("Colony").join("apps"))
}

/// Check if a Colony app is installed for the current platform.
/// Returns Some(path) if the binary exists, None otherwise.
pub fn installed_app_path(repo: &ColonyRepo) -> Option<PathBuf> {
    let platform = current_platform_key();
    let entry = repo.manifest.release_files.get(platform)?;
    // Priority: binary > file > saved asset name (from filePattern resolution)
    let filename = if let Some(ref bin) = entry.binary {
        bin.clone()
    } else if let Some(ref file) = entry.file {
        file.clone()
    } else {
        // filePattern was used — check saved resolved asset name
        load_installed_asset(&repo.name)?
    };
    let path = colony_apps_dir().ok()?.join(&repo.name).join(&filename);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

/// Installed version info stored alongside the binary.
const VERSION_FILE: &str = ".colony_version";
/// Saved resolved asset name (when using filePattern).
const ASSET_FILE: &str = ".colony_asset";

/// Save the installed version tag for a repo.
pub fn save_installed_version(repo_name: &str, tag: &str) -> Result<()> {
    let path = colony_apps_dir()?.join(repo_name).join(VERSION_FILE);
    std::fs::write(&path, tag)?;
    Ok(())
}

/// Load the installed version tag for a repo.
pub fn load_installed_version(repo_name: &str) -> Option<String> {
    let path = colony_apps_dir().ok()?.join(repo_name).join(VERSION_FILE);
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

/// Save the resolved asset name for a repo (when using filePattern).
pub fn save_installed_asset(repo_name: &str, filename: &str) -> Result<()> {
    let path = colony_apps_dir()?.join(repo_name).join(ASSET_FILE);
    std::fs::write(&path, filename)?;
    Ok(())
}

/// Load the saved resolved asset name for a repo.
pub fn load_installed_asset(repo_name: &str) -> Option<String> {
    let path = colony_apps_dir().ok()?.join(repo_name).join(ASSET_FILE);
    std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
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
pub async fn fetch_latest_release_tag(
    client: &reqwest::Client,
    repo_name: &str,
) -> Result<String> {
    fetch_latest_release_tag_for(client, GITHUB_ACCOUNT, repo_name).await
}

/// Resolved release information from GitHub API.
#[derive(Debug)]
pub struct ResolvedRelease {
    pub tag: String,
    pub asset_names: Vec<String>,
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
    }

    let release: Release = serde_json::from_str(&body)?;
    Ok(ResolvedRelease {
        tag: release.tag_name,
        asset_names: release.assets.into_iter().map(|a| a.name).collect(),
    })
}

/// Find an asset whose name contains the given pattern (case-insensitive).
/// Returns an error if zero or multiple assets match.
pub fn find_asset_by_pattern(assets: &[String], pattern: &str) -> Result<String> {
    let pattern_lower = pattern.to_lowercase();
    let matches: Vec<&String> = assets
        .iter()
        .filter(|name| name.to_lowercase().contains(&pattern_lower))
        .collect();
    match matches.len() {
        0 => anyhow::bail!("No release asset matching pattern '{pattern}'"),
        1 => Ok(matches[0].clone()),
        n => anyhow::bail!(
            "Ambiguous pattern '{pattern}': {n} assets match ({}). Use a more specific pattern.",
            matches.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
        ),
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
        tracing::info!(
            "Auto-detected platforms for {repo_name}: {:?}",
            platforms
        );
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

/// Check if an update is available.
/// Returns Some(latest_tag) if a newer version exists, None otherwise.
pub async fn check_update_available(
    client: &reqwest::Client,
    repo_name: &str,
) -> Option<String> {
    let installed = load_installed_version(repo_name)?;
    let latest = fetch_latest_release_tag(client, repo_name).await.ok()?;

    let installed_ver = parse_version_tag(&installed)?;
    let latest_ver = parse_version_tag(&latest)?;

    if latest_ver > installed_ver {
        Some(latest)
    } else {
        None
    }
}

/// Download progress info.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

/// Verify SHA256 checksum of a file against an expected hex digest.
fn verify_sha256(path: &std::path::Path, expected_hex: &str) -> Result<()> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    let computed = format!("{:x}", hasher.finalize());
    if computed != expected_hex.to_lowercase() {
        anyhow::bail!(
            "SHA256 mismatch: expected {}, got {}",
            expected_hex.to_lowercase(),
            computed
        );
    }
    Ok(())
}

/// Extract a single file from a .zip archive.
fn extract_from_zip(
    archive_path: &std::path::Path,
    binary_name: &str,
    dest_dir: &std::path::Path,
) -> Result<PathBuf> {
    // Guard against path traversal: binary_name must be a single normal component.
    let name_path = std::path::Path::new(binary_name);
    anyhow::ensure!(
        name_path.components().count() == 1
            && matches!(
                name_path.components().next(),
                Some(std::path::Component::Normal(_))
            ),
        "Invalid binary name (path traversal attempt?): {binary_name}"
    );
    let file = std::fs::File::open(archive_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let entry_name = entry.name().to_string();
        // Match by exact filename (last component), handles entries like "dir/binary"
        let matches = std::path::Path::new(&entry_name)
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == binary_name)
            .unwrap_or(false);
        if matches {
            let dest = dest_dir.join(binary_name);
            let mut out = std::fs::File::create(&dest)?;
            std::io::copy(&mut entry, &mut out)?;
            return Ok(dest);
        }
    }
    anyhow::bail!("Binary '{binary_name}' not found in zip archive")
}

/// Extract a single file from a .tar.gz archive.
fn extract_from_tar_gz(
    archive_path: &std::path::Path,
    binary_name: &str,
    dest_dir: &std::path::Path,
) -> Result<PathBuf> {
    // Guard against path traversal: binary_name must be a single normal component.
    let name_path = std::path::Path::new(binary_name);
    anyhow::ensure!(
        name_path.components().count() == 1
            && matches!(
                name_path.components().next(),
                Some(std::path::Component::Normal(_))
            ),
        "Invalid binary name (path traversal attempt?): {binary_name}"
    );
    let file = std::fs::File::open(archive_path)?;
    let gz = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(gz);

    for entry_result in archive.entries()? {
        let mut entry = entry_result?;
        let path = entry.path()?;
        let matches = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n == binary_name)
            .unwrap_or(false);
        if matches {
            let dest = dest_dir.join(binary_name);
            entry.unpack(&dest)?;
            return Ok(dest);
        }
    }
    anyhow::bail!("Binary '{binary_name}' not found in tar.gz archive")
}

/// Extract a binary from an archive based on its extension.
fn extract_binary_from_archive(
    archive_path: &std::path::Path,
    binary_name: &str,
    dest_dir: &std::path::Path,
) -> Result<PathBuf> {
    let filename = archive_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    if filename.ends_with(".zip") {
        let result = extract_from_zip(archive_path, binary_name, dest_dir);
        let _ = std::fs::remove_file(archive_path);
        result
    } else if filename.ends_with(".tar.gz") || filename.ends_with(".tgz") {
        let result = extract_from_tar_gz(archive_path, binary_name, dest_dir);
        let _ = std::fs::remove_file(archive_path);
        result
    } else {
        // Raw binary (e.g. .exe, no archive extension) — rename to binary_name in dest_dir
        let dest = dest_dir.join(binary_name);
        std::fs::rename(archive_path, &dest)?;
        Ok(dest)
    }
}

/// Download a release asset to `<colony_apps_dir>/<repo_name>/<filename>`.
/// If `expected_sha256` is provided, verifies the file integrity after download.
/// If `binary_name` is provided, treats the downloaded file as an archive and
/// extracts the named binary from it (supports .zip and .tar.gz).
/// Returns the final path on success.
pub async fn download_release_asset(
    token: Option<String>,
    repo_name: String,
    tag: String,
    filename: String,
    binary_name: Option<String>,
    expected_sha256: Option<String>,
    progress_tx: Option<futures::channel::mpsc::UnboundedSender<f32>>,
) -> Result<PathBuf> {
    let dest_dir = colony_apps_dir()?.join(&repo_name);
    std::fs::create_dir_all(&dest_dir)?;
    let dest_path = dest_dir.join(&filename);

    let url = format!(
        "https://github.com/{GITHUB_ACCOUNT}/{repo_name}/releases/download/{tag}/{filename}"
    );

    let client = reqwest::Client::builder()
        .user_agent(format!("Colony-Launcher/{APP_VERSION}"))
        .timeout(Duration::from_secs(300))
        .connect_timeout(CONNECT_TIMEOUT)
        .build()?;

    let mut request = client.get(&url);
    if let Some(ref t) = token {
        request = request.header(reqwest::header::AUTHORIZATION, format!("Bearer {t}"));
    }

    let resp = request.send().await.map_err(|e| {
        if e.is_timeout() {
            anyhow::anyhow!("Download timed out for {filename}")
        } else {
            anyhow::anyhow!("Download failed for {filename}: {e}")
        }
    })?;

    if !resp.status().is_success() {
        let status = resp.status();
        anyhow::bail!("Download failed: HTTP {status} for {url}");
    }

    let total = resp.content_length();
    let mut downloaded: u64 = 0;

    // Stream the download in chunks using async bytes_stream
    use futures::StreamExt;
    use std::io::Write;
    let mut file = std::fs::File::create(&dest_path)?;
    let mut stream = resp.bytes_stream();

    let mut last_pct: u32 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;

        // Send progress updates (throttled to whole-percent changes)
        if let (Some(ref tx), Some(total)) = (&progress_tx, total) {
            if total > 0 {
                let pct = ((downloaded as f64 / total as f64) * 100.0) as u32;
                if pct != last_pct {
                    last_pct = pct;
                    let _ = tx.unbounded_send(downloaded as f32 / total as f32);
                }
            }
        }
    }
    file.flush()?;

    // SHA256 verification and archive extraction are CPU-bound — run in spawn_blocking
    let final_path = {
        let dest_path = dest_path.clone();
        let expected_sha256 = expected_sha256.clone();
        let binary_name = binary_name.clone();
        let filename = filename.clone();
        let dest_dir = dest_dir.clone();

        tokio::task::spawn_blocking(move || -> Result<PathBuf> {
            // Verify SHA256 checksum if provided (on the downloaded file, before extraction)
            if let Some(ref expected) = expected_sha256 {
                if let Err(e) = verify_sha256(&dest_path, expected) {
                    // Remove the corrupt file
                    let _ = std::fs::remove_file(&dest_path);
                    return Err(e);
                }
                tracing::info!("SHA256 verified for {filename}");
            }

            // If `binary_name` is set, extract the binary from the archive
            let final_path = if let Some(ref bin) = binary_name {
                tracing::info!("Extracting '{bin}' from archive '{filename}'");
                extract_binary_from_archive(&dest_path, bin, &dest_dir)?
            } else {
                dest_path
            };

            // Make executable on Linux/macOS
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&final_path)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&final_path, perms)?;
            }

            Ok(final_path)
        })
        .await??
    };

    Ok(final_path)
}

// --- Launcher self-update ---

/// Expected release asset name for the Colony launcher binary on the current platform.
pub fn launcher_asset_name() -> String {
    let platform = current_platform_key();
    let ext = if cfg!(target_os = "windows") { ".exe" } else { "" };
    format!("colony-{platform}{ext}")
}

/// Check if a newer version of the Colony launcher itself is available.
/// Returns Some((latest_tag, asset_filename)) if an update exists, None otherwise.
pub async fn check_launcher_update(
    client: &reqwest::Client,
) -> Option<(String, String)> {
    let latest_tag = fetch_latest_release_tag_for(client, LAUNCHER_OWNER, LAUNCHER_REPO)
        .await
        .ok()?;

    let current = parse_version_tag(APP_VERSION)?;
    let latest = parse_version_tag(&latest_tag)?;

    if latest > current {
        Some((latest_tag, launcher_asset_name()))
    } else {
        None
    }
}

/// Download a release asset from the Colony launcher repo.
/// Returns the path to the downloaded file in a staging directory.
pub async fn download_launcher_asset(
    token: Option<String>,
    tag: String,
    filename: String,
    progress_tx: Option<futures::channel::mpsc::UnboundedSender<f32>>,
) -> Result<PathBuf> {
    let temp_dir = colony_data_dir()?.join("update-staging");
    std::fs::create_dir_all(&temp_dir)?;
    let dest_path = temp_dir.join(&filename);

    let url = format!(
        "https://github.com/{LAUNCHER_OWNER}/{LAUNCHER_REPO}/releases/download/{tag}/{filename}"
    );

    let client = reqwest::Client::builder()
        .user_agent(format!("Colony-Launcher/{APP_VERSION}"))
        .timeout(Duration::from_secs(300))
        .connect_timeout(CONNECT_TIMEOUT)
        .build()?;

    let mut request = client.get(&url);
    if let Some(ref t) = token {
        request = request.header(reqwest::header::AUTHORIZATION, format!("Bearer {t}"));
    }

    let resp = request.send().await.map_err(|e| {
        if e.is_timeout() {
            anyhow::anyhow!("Download timed out for {filename}")
        } else {
            anyhow::anyhow!("Download failed for {filename}: {e}")
        }
    })?;

    if !resp.status().is_success() {
        let status = resp.status();
        anyhow::bail!("Download failed: HTTP {status} for {url}");
    }

    let total = resp.content_length();
    let mut downloaded: u64 = 0;

    use futures::StreamExt;
    use std::io::Write;
    let mut file = std::fs::File::create(&dest_path)?;
    let mut stream = resp.bytes_stream();
    let mut last_pct: u32 = 0;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;

        if let (Some(ref tx), Some(total)) = (&progress_tx, total) {
            if total > 0 {
                let pct = ((downloaded as f64 / total as f64) * 100.0) as u32;
                if pct != last_pct {
                    last_pct = pct;
                    let _ = tx.unbounded_send(downloaded as f32 / total as f32);
                }
            }
        }
    }
    file.flush()?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dest_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest_path, perms)?;
    }

    Ok(dest_path)
}

/// Replace the running Colony binary with the downloaded update.
/// Returns the exe path for relaunch on success. Restores backup on failure.
pub fn apply_launcher_update(new_binary: &std::path::Path) -> Result<PathBuf> {
    let current_exe = std::env::current_exe()
        .map_err(|e| anyhow::anyhow!("Cannot determine current exe path: {e}"))?;

    let backup_path = current_exe.with_extension("old");

    // Remove stale backup from previous update
    if backup_path.exists() {
        let _ = std::fs::remove_file(&backup_path);
    }

    // Rename current binary to .old (running binary can be renamed on all platforms)
    std::fs::rename(&current_exe, &backup_path)
        .map_err(|e| anyhow::anyhow!("Failed to backup current binary: {e}"))?;

    // Copy new binary into the current exe path
    match std::fs::copy(new_binary, &current_exe) {
        Ok(_) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&current_exe)?.permissions();
                perms.set_mode(0o755);
                std::fs::set_permissions(&current_exe, perms)?;
            }
            // Clean up staged download
            let _ = std::fs::remove_file(new_binary);
            let _ = std::fs::remove_dir(new_binary.parent().unwrap_or(new_binary));
            Ok(current_exe)
        }
        Err(e) => {
            // Restore backup on failure
            tracing::error!("Failed to copy new binary, restoring backup: {e}");
            let _ = std::fs::rename(&backup_path, &current_exe);
            Err(anyhow::anyhow!("Failed to install new binary: {e}"))
        }
    }
}

// --- Offline cache ---

fn repos_cache_path() -> Result<PathBuf> {
    let cache_dir = colony_data_dir()?.join("cache");
    std::fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir.join("repos_cache.json"))
}

/// Save Colony repos to local cache for offline use.
pub fn save_repos_cache(repos: &[ColonyRepo]) -> Result<()> {
    let path = repos_cache_path()?;
    let json = serde_json::to_string(repos)?;
    std::fs::write(&path, json)?;
    tracing::debug!("Saved {} repos to cache", repos.len());
    Ok(())
}

/// Load cached Colony repos for offline use.
pub fn load_repos_cache() -> Option<Vec<ColonyRepo>> {
    let path = repos_cache_path().ok()?;
    let content = std::fs::read_to_string(&path).ok()?;
    let repos: Vec<ColonyRepo> = serde_json::from_str(&content).ok()?;
    tracing::info!("Loaded {} repos from offline cache", repos.len());
    Some(repos)
}

// --- Favorites persistence ---

fn favorites_path() -> Result<PathBuf> {
    let dir = colony_data_dir()?.join("preferences");
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("favorites.json"))
}

/// Load the list of favorite application names.
pub fn load_favorites() -> Vec<String> {
    favorites_path()
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}

/// Save the list of favorite application names.
pub fn save_favorites(favorites: &[String]) -> Result<()> {
    let path = favorites_path()?;
    let json = serde_json::to_string(favorites)?;
    std::fs::write(&path, json)?;
    Ok(())
}

// --- User preferences persistence ---

/// User preferences saved between sessions.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserPreferences {
    pub selected_section: Option<usize>,
    pub window_width: Option<f32>,
    pub window_height: Option<f32>,
    pub first_launch_done: Option<bool>,
    pub selected_theme: Option<String>,
    pub selected_variant: Option<String>,
    pub selected_accent: Option<String>,
    // General
    pub auto_scan: Option<bool>,
    pub restore_session: Option<bool>,
    pub default_view: Option<String>,
    pub close_behavior: Option<String>,
    pub language: Option<String>,
    pub auto_check_updates: Option<bool>,
    pub update_channel: Option<String>,
    pub auto_install_updates: Option<bool>,
    // Appearance
    pub font_size: Option<String>,
    pub animations: Option<bool>,
    // Accessibility
    pub high_contrast: Option<bool>,
    pub text_size_a11y: Option<String>,
    pub reduce_motion: Option<bool>,
    pub keyboard_nav: Option<bool>,
    pub dyslexia_font: Option<bool>,
    // Storage
    pub scan_on_startup: Option<bool>,
}

fn preferences_path() -> Result<PathBuf> {
    let dir = colony_data_dir()?.join("preferences");
    std::fs::create_dir_all(&dir)?;
    Ok(dir.join("preferences.json"))
}

/// Load user preferences.
pub fn load_preferences() -> UserPreferences {
    preferences_path()
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}

/// Save user preferences.
pub fn save_preferences(prefs: &UserPreferences) -> Result<()> {
    let path = preferences_path()?;
    let json = serde_json::to_string_pretty(prefs)?;
    std::fs::write(&path, json)?;
    Ok(())
}

// --- Application scan cache ---

fn scan_cache_path() -> Result<PathBuf> {
    let cache_dir = colony_data_dir()?.join("cache");
    std::fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir.join("scan_cache.json"))
}

/// Cached scan entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedScanResult {
    pub apps: Vec<CachedApp>,
    pub timestamp: u64,
}

/// Serializable application for cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedApp {
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
    pub category: String,
    pub origin: String,
}

/// Save scanned applications to cache.
pub fn save_scan_cache(apps: &[CachedApp]) -> Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let entry = CachedScanResult {
        apps: apps.to_vec(),
        timestamp,
    };
    let path = scan_cache_path()?;
    let json = serde_json::to_string(&entry)?;
    std::fs::write(&path, json)?;
    Ok(())
}

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
        assert_eq!(manifest.release_files["windows"].file.as_deref(), Some("TestApp.exe"));
        assert_eq!(manifest.release_files["linux"].tag, "Linux");
        assert_eq!(manifest.release_files["linux"].file.as_deref(), Some("TestApp"));
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
        assert_eq!(win.file.as_deref(), Some("lilypad-x86_64-pc-windows-msvc.zip"));
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
    fn extract_from_zip_works() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("colony_test_zip_extract");
        let _ = std::fs::create_dir_all(&dir);

        // Create a zip archive with a binary inside
        let zip_path = dir.join("test.zip");
        let file = std::fs::File::create(&zip_path).unwrap();
        let mut zip_writer = zip::ZipWriter::new(file);
        zip_writer
            .start_file("subdir/my-binary", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip_writer.write_all(b"binary-content").unwrap();
        zip_writer.finish().unwrap();

        // Extract
        let result = extract_from_zip(&zip_path, "my-binary", &dir);
        assert!(result.is_ok());
        let extracted = result.unwrap();
        assert_eq!(extracted.file_name().unwrap().to_str().unwrap(), "my-binary");
        assert_eq!(std::fs::read_to_string(&extracted).unwrap(), "binary-content");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extract_from_tar_gz_works() {
        let dir = std::env::temp_dir().join("colony_test_targz_extract");
        let _ = std::fs::create_dir_all(&dir);

        // Create a tar.gz archive with a binary inside
        let tar_gz_path = dir.join("test.tar.gz");
        let file = std::fs::File::create(&tar_gz_path).unwrap();
        let gz = flate2::write::GzEncoder::new(file, flate2::Compression::default());
        let mut tar_builder = tar::Builder::new(gz);

        let content = b"binary-content-tar";
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        tar_builder
            .append_data(&mut header, "subdir/my-cli", &content[..])
            .unwrap();
        // Finish tar, then finish gzip encoder to write the gzip footer
        let gz = tar_builder.into_inner().unwrap();
        gz.finish().unwrap();

        // Extract
        let result = extract_from_tar_gz(&tar_gz_path, "my-cli", &dir);
        assert!(result.is_ok(), "extract failed: {:?}", result.err());
        let extracted = result.unwrap();
        assert_eq!(extracted.file_name().unwrap().to_str().unwrap(), "my-cli");
        assert_eq!(
            std::fs::read_to_string(&extracted).unwrap(),
            "binary-content-tar"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extract_from_zip_missing_binary() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("colony_test_zip_missing");
        let _ = std::fs::create_dir_all(&dir);

        let zip_path = dir.join("empty.zip");
        let file = std::fs::File::create(&zip_path).unwrap();
        let mut zip_writer = zip::ZipWriter::new(file);
        zip_writer
            .start_file("other-file", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip_writer.write_all(b"data").unwrap();
        zip_writer.finish().unwrap();

        let result = extract_from_zip(&zip_path, "nonexistent", &dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));

        let _ = std::fs::remove_dir_all(&dir);
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
    fn colony_apps_dir_returns_path() {
        let dir = colony_apps_dir();
        assert!(dir.is_ok());
        let path = dir.unwrap();
        assert!(path.ends_with("Colony/apps"));
    }

    #[test]
    fn base64_decode_manifest() {
        let json = r#"{"name":"Test","category":"Games","platforms":["linux"],"releaseFiles":{"linux":{"tag":"v1","file":"test"}}}"#;
        let encoded = base64::engine::general_purpose::STANDARD.encode(json);
        let decoded = base64::engine::general_purpose::STANDARD.decode(&encoded).unwrap();
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
    fn sha256_verification_correct() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("colony_test_sha256");
        let _ = std::fs::create_dir_all(&dir);
        let file_path = dir.join("test.bin");
        let content = b"hello world";
        let mut f = std::fs::File::create(&file_path).unwrap();
        f.write_all(content).unwrap();
        f.flush().unwrap();

        // SHA256 of "hello world"
        let expected = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        assert!(verify_sha256(&file_path, expected).is_ok());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sha256_verification_mismatch() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("colony_test_sha256_bad");
        let _ = std::fs::create_dir_all(&dir);
        let file_path = dir.join("test.bin");
        let mut f = std::fs::File::create(&file_path).unwrap();
        f.write_all(b"hello world").unwrap();
        f.flush().unwrap();

        assert!(verify_sha256(&file_path, "0000000000000000").is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn preferences_default() {
        let prefs = UserPreferences::default();
        assert!(prefs.selected_section.is_none());
        assert!(prefs.first_launch_done.is_none());
    }

    #[test]
    fn preferences_serialization() {
        let prefs = UserPreferences {
            selected_section: Some(2),
            window_width: Some(1200.0),
            window_height: Some(800.0),
            first_launch_done: Some(true),
            ..Default::default()
        };
        let json = serde_json::to_string(&prefs).unwrap();
        let loaded: UserPreferences = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.selected_section, Some(2));
        assert_eq!(loaded.first_launch_done, Some(true));
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
        let assets = vec![
            "orcal-linux".to_string(),
            "orcal-windows.exe".to_string(),
        ];
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
