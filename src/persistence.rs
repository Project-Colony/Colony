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

/// Resolve a bundled/overridable config file (e.g. `colony.toml`,
/// `categories.json`) from a stable location instead of the process CWD —
/// which is `/` or the user's home when Colony is launched from a menu entry,
/// so CWD-relative config never loaded for installed binaries.
///
/// Checks, in order: the user data dir (`<config>/Colony/Colony/config/`), the
/// `config/` directory next to the executable, then `config/<name>` relative to
/// the CWD (dev convenience). Returns the first path that exists.
pub fn find_config_file(name: &str) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(dir) = colony_data_dir() {
        candidates.push(dir.join("config").join(name));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            candidates.push(exe_dir.join("config").join(name));
        }
    }
    candidates.push(PathBuf::from("config").join(name));
    candidates.into_iter().find(|p| p.exists())
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
