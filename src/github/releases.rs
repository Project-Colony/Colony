//! Release resolution: which tag, which asset, and whether an update exists.

use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;

use crate::persistence::load_installed_version;

use super::http::*;
use super::types::*;

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
    let body = cached_get(client, &url).await?;

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
    /// Byte size per asset name, as reported by the API. The store could not
    /// answer "how big is this download?" before committing to it: the total
    /// was only learned from Content-Length once the transfer had started.
    pub asset_sizes: HashMap<String, u64>,
    /// ISO-8601 publication timestamp of the release, when the API gave one.
    pub published_at: Option<String>,
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
    // `tag` is remote data (colony.json) and this request carries the user's
    // bearer token, so build the path from encoded segments: interpolating it
    // lets `..` shorten the path onto a different endpoint entirely. See
    // `crate::download::build_url`.
    let url = if tag.eq_ignore_ascii_case("latest") {
        format!("{GITHUB_API}/repos/{GITHUB_ACCOUNT}/{repo_name}/releases/latest")
    } else {
        crate::download::build_url(
            GITHUB_API,
            &["repos", GITHUB_ACCOUNT, repo_name, "releases", "tags", tag],
        )?
    };
    let body = cached_get(client, &url).await?;

    #[derive(Deserialize)]
    struct Asset {
        name: String,
        #[serde(default)]
        size: u64,
    }
    #[derive(Deserialize)]
    struct Release {
        tag_name: String,
        assets: Vec<Asset>,
        body: Option<String>,
        #[serde(default)]
        published_at: Option<String>,
    }

    let release: Release = serde_json::from_str(&body)?;
    let asset_sizes: HashMap<String, u64> = release
        .assets
        .iter()
        .map(|a| (a.name.clone(), a.size))
        .collect();
    Ok(ResolvedRelease {
        tag: release.tag_name,
        asset_names: release.assets.into_iter().map(|a| a.name).collect(),
        asset_sizes,
        published_at: release.published_at,
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
    // The signed metadata sidecar (see crate::signing). Colony's own releases
    // already ship `colony-linux.meta`, so the day an ecosystem app adopts the
    // sidecar, a documented legacy substring pattern like "linux" would start
    // matching two assets and fail with "Ambiguous pattern" - which is exactly
    // what docs/colony-spec.md promises cannot happen.
    ".meta",
    ".sha256",
    ".sha256sum",
    // electron-builder publishes one per installer (SphereCord already does).
    ".blockmap",
    ".txt",
    ".yml",
    ".yaml",
    ".json",
];

/// Anchored glob match: `*` matches any run of characters, everything else is
/// literal (case-insensitive - both inputs must already be lowercase). The
/// pattern must cover the WHOLE name, unlike the legacy substring mode.
pub(crate) fn glob_matches(pattern: &str, name: &str) -> bool {
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
/// for the current platform.
///
/// `Ok(Some(tag))` = an update to `tag` is available. `Ok(None)` = the check
/// RAN and there is nothing to install (either the app is not installed at all,
/// or it is current). `Err` = the check could NOT run, and the caller must not
/// turn that into "up to date" — the same fail-loud contract
/// [`check_launcher_update`] already keeps for the launcher's own check.
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
) -> Result<Option<String>> {
    // Not installed is a real answer, not a failure: there is nothing to update.
    let Some(installed) = load_installed_version(repo_name) else {
        return Ok(None);
    };

    let target = if pinned_tag.eq_ignore_ascii_case("latest") {
        fetch_latest_release_tag(client, repo_name).await?
    } else {
        pinned_tag.to_string()
    };

    // Case-insensitive: "Nightly" vs "nightly" must not read as an update
    // (with non-semver tags the string fallback below would flag it forever).
    if target.eq_ignore_ascii_case(&installed) {
        return Ok(None);
    }

    match (parse_version_tag(&installed), parse_version_tag(&target)) {
        (Some(installed_ver), Some(target_ver)) => {
            Ok((target_ver > installed_ver).then_some(target))
        }
        _ => {
            tracing::warn!(
                "Non-semver tags for {repo_name} (installed '{installed}', target '{target}'); using string comparison"
            );
            Ok(Some(target))
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
