//! Local persistence for Colony: data directories, config resolution, installed
//! app / version state, offline caches, favorites, and user preferences. This
//! module holds pure on-disk storage and does not touch the GitHub API.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::github::{current_platform_key, ColonyRepo};

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
pub(crate) fn save_repo_doc(repo_name: &str, filename: &str, content: &str) {
    if let Ok(dir) = repo_docs_dir(repo_name) {
        let _ = std::fs::write(dir.join(filename), content);
    }
}

/// Read a cached document from disk. Returns None if file doesn't exist.
pub fn read_repo_doc(repo_name: &str, filename: &str) -> Option<String> {
    let dir = repo_docs_dir(repo_name).ok()?;
    std::fs::read_to_string(dir.join(filename)).ok()
}

/// Directory for the cached per-repo app icon: `~/.config/Colony/Colony/repo-icons/{repo_name}/`
fn repo_icon_dir(repo_name: &str) -> Result<PathBuf> {
    let base = colony_data_dir()?.join("repo-icons").join(repo_name);
    std::fs::create_dir_all(&base)?;
    Ok(base)
}

/// Save the raw (PNG) app icon bytes to disk cache.
pub(crate) fn save_repo_icon(repo_name: &str, bytes: &[u8]) {
    if let Ok(dir) = repo_icon_dir(repo_name) {
        let _ = std::fs::write(dir.join("icon.png"), bytes);
    }
}

/// Read the cached app icon bytes from disk. Returns None if none cached.
pub fn load_repo_icon(repo_name: &str) -> Option<Vec<u8>> {
    let dir = repo_icon_dir(repo_name).ok()?;
    std::fs::read(dir.join("icon.png")).ok()
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
    // The filename comes straight from the repo's own manifest: the same
    // traversal guard as the install path applies, or a hostile manifest
    // could point the Launch button at an arbitrary executable on disk
    // (e.g. `binary: "../../somewhere/else"`).
    if crate::download::ensure_safe_component(&filename).is_err() {
        tracing::warn!(
            repo = %repo.name,
            %filename,
            "refusing manifest launch path outside the app's install dir"
        );
        return None;
    }
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
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
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
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
}

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

/// Write a `.desktop` launcher entry for an installed store app, so desktop
/// environments (rofi/wofi/GNOME/KDE) index it like any other application.
/// The entry is tagged `X-Colony-Managed=true`, which Colony's own scan skips
/// (the app is already represented by its store card). Linux only; no-op
/// elsewhere.
#[cfg(target_os = "linux")]
pub fn write_desktop_entry(repo_name: &str, exec_path: &std::path::Path) -> Result<()> {
    let dir = dirs::data_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine data directory"))?
        .join("applications");
    std::fs::create_dir_all(&dir)?;
    let icon_line = repo_icon_dir(repo_name)
        .ok()
        .map(|d| d.join("icon.png"))
        .filter(|p| p.exists())
        .map(|p| format!("Icon={}\n", p.display()))
        .unwrap_or_default();
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName={repo_name}\nExec=\"{}\"\nTerminal=false\nCategories=Utility;\nComment=Installed by Colony\nX-Colony-Managed=true\n{icon_line}",
        exec_path.display()
    );
    std::fs::write(dir.join(desktop_entry_filename(repo_name)), entry)?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn write_desktop_entry(_repo_name: &str, _exec_path: &std::path::Path) -> Result<()> {
    Ok(())
}

/// Remove the `.desktop` entry written by [`write_desktop_entry`] (no-op when
/// absent or on non-Linux platforms).
pub fn remove_desktop_entry(repo_name: &str) {
    #[cfg(target_os = "linux")]
    if let Some(data) = dirs::data_dir() {
        let path = data
            .join("applications")
            .join(desktop_entry_filename(repo_name));
        if path.exists() {
            if let Err(e) = std::fs::remove_file(&path) {
                tracing::warn!("failed to remove desktop entry {}: {e}", path.display());
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = repo_name;
}

#[cfg(target_os = "linux")]
fn desktop_entry_filename(repo_name: &str) -> String {
    format!("colony-{}.desktop", repo_name.to_lowercase())
}

/// Remove ALL store caches (docs + icons for every repo). Manual cache
/// management from Settings > Storage; installs and preferences are NOT
/// touched. Returns the number of cache directories removed.
pub fn clear_store_caches() -> usize {
    let Ok(base) = colony_data_dir() else {
        return 0;
    };
    let mut removed = 0;
    for parent in ["repo-docs", "repo-icons"] {
        let Ok(entries) = std::fs::read_dir(base.join(parent)) else {
            continue;
        };
        for entry in entries.flatten() {
            if std::fs::remove_dir_all(entry.path()).is_ok() {
                removed += 1;
            }
        }
    }
    removed
}

/// Remove doc/icon caches for repos that are NO LONGER in the catalog, so a
/// deleted or renamed repo does not leave its caches behind forever. Runs
/// after each successful catalog fetch (never on a cache fallback, where a
/// transient absence must not purge anything). Uninstalling a still-listed
/// app deliberately keeps its caches - they render the catalog entry.
pub fn prune_orphaned_repo_caches(live_repo_names: &[String]) {
    let Ok(base) = colony_data_dir() else {
        return;
    };
    for parent in ["repo-docs", "repo-icons"] {
        let Ok(entries) = std::fs::read_dir(base.join(parent)) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !live_repo_names.iter().any(|r| r == &name) {
                let path = entry.path();
                tracing::info!("pruning orphaned cache {}", path.display());
                if let Err(e) = std::fs::remove_dir_all(&path) {
                    tracing::warn!("failed to prune {}: {e}", path.display());
                }
            }
        }
    }
}

/// Load the cached application scan (`None` if absent or unreadable). Read at
/// boot when the startup scan is disabled, so the local-apps grid restores the
/// last known state instead of showing "0 apps" at every launch.
pub fn load_scan_cache() -> Option<Vec<CachedApp>> {
    let path = scan_cache_path().ok()?;
    let json = std::fs::read_to_string(path).ok()?;
    let cached: CachedScanResult = serde_json::from_str(&json).ok()?;
    Some(cached.apps)
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
    fn colony_apps_dir_returns_path() {
        let dir = colony_apps_dir();
        assert!(dir.is_ok());
        let path = dir.unwrap();
        assert!(path.ends_with("Colony/apps"));
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
}
