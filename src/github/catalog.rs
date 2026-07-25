//! Building the store catalog: list the org's repos, read each `colony.json`,
//! and gather the docs and icon that go with it.

use anyhow::Result;
use base64::Engine;

use crate::persistence::{load_repos_cache, save_repo_doc, save_repo_icon};

use super::releases::auto_detect_release;

use super::http::*;
use super::types::*;

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
