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

/// Join a repo name onto `base` as a single directory component.
///
/// `repo_name` is remote data (the `name` field of the GitHub API listing, also
/// rehydrated from `repos_cache.json`), and it is joined into a path in every
/// per-repo helper below plus the install directory and a `remove_dir_all`. The
/// leaf-filename guard was applied everywhere but here, so the containment
/// invariant "Colony only writes under `apps/<repo>/`" rested on GitHub refusing
/// `/` and `..` in repo names. Enforce it locally instead of inheriting it.
fn join_repo_component(base: PathBuf, repo_name: &str) -> Result<PathBuf> {
    crate::download::ensure_safe_component(repo_name)?;
    Ok(base.join(repo_name))
}

/// Per-repo directory under the apps root: `<data_local>/Colony/apps/{repo_name}/`
pub(crate) fn colony_app_dir(repo_name: &str) -> Result<PathBuf> {
    join_repo_component(colony_apps_dir()?, repo_name)
}

/// Directory for cached repo documentation files: `~/.config/Colony/Colony/repo-docs/{repo_name}/`
fn repo_docs_dir(repo_name: &str) -> Result<PathBuf> {
    let base = join_repo_component(colony_data_dir()?.join("repo-docs"), repo_name)?;
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
    let base = join_repo_component(colony_data_dir()?.join("repo-icons"), repo_name)?;
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
    let path = colony_app_dir(&repo.name).ok()?.join(&filename);
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

/// Marker recording that this app was installed with a verified signature.
const SIGNED_FILE: &str = ".colony_signed";

/// Save the installed version tag for a repo.
pub fn save_installed_version(repo_name: &str, tag: &str) -> Result<()> {
    let path = colony_app_dir(repo_name)?.join(VERSION_FILE);
    std::fs::write(&path, tag)?;
    Ok(())
}

/// Load the installed version tag for a repo.
pub fn load_installed_version(repo_name: &str) -> Option<String> {
    let path = colony_app_dir(repo_name).ok()?.join(VERSION_FILE);
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
}

/// Record whether the installed build was signature-verified.
///
/// Read back by the installer to pin the requirement: `manifest.signed` lives in
/// the very repo the signature is meant to protect, so an attacker with write
/// access could flip it to false and drop the `.sig` to install unsigned code
/// silently. Once an app has been installed with a verified signature, later
/// updates must stay signed regardless of what the manifest now claims.
/// Only ever sets the marker: the pin must not be clearable by a later unsigned
/// install, which is the whole point. Uninstalling removes the app directory and
/// with it the pin, and that is the deliberate way out.
pub fn save_installed_signed(repo_name: &str) -> Result<()> {
    let path = colony_app_dir(repo_name)?.join(SIGNED_FILE);
    std::fs::write(&path, "1")?;
    Ok(())
}

/// True when a previous install of this repo was signature-verified.
///
/// Matched case-insensitively: GitHub repo names are case-insensitive for lookup
/// while the install directory is not, so a rename from `Spotter` to `spotter`
/// would otherwise present as a brand-new app and silently drop the pin (while
/// still overwriting the same lowercased `.desktop` entry).
pub fn load_installed_signed(repo_name: &str) -> bool {
    if colony_app_dir(repo_name)
        .map(|d| d.join(SIGNED_FILE).exists())
        .unwrap_or(false)
    {
        return true;
    }
    let Ok(apps) = colony_apps_dir() else {
        return false;
    };
    let wanted = repo_name.to_lowercase();
    let Ok(entries) = std::fs::read_dir(apps) else {
        return false;
    };
    entries.flatten().any(|e| {
        e.file_name().to_string_lossy().to_lowercase() == wanted
            && e.path().join(SIGNED_FILE).exists()
    })
}

/// Save the resolved asset name for a repo (when using filePattern).
pub fn save_installed_asset(repo_name: &str, filename: &str) -> Result<()> {
    let path = colony_app_dir(repo_name)?.join(ASSET_FILE);
    std::fs::write(&path, filename)?;
    Ok(())
}

/// Load the saved resolved asset name for a repo.
pub fn load_installed_asset(repo_name: &str) -> Option<String> {
    let path = colony_app_dir(repo_name).ok()?.join(ASSET_FILE);
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

fn http_cache_path() -> Result<PathBuf> {
    let cache_dir = colony_data_dir()?.join("cache");
    std::fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir.join("http_etags.json"))
}

/// Load the persisted conditional-request cache (see `github::http`).
///
/// Generic so the cache's shape stays private to the HTTP layer - this module
/// only knows where the file goes. A corrupt or older-shaped file simply
/// deserialises to `None` and the cache starts empty, which costs quota but is
/// never wrong.
pub fn load_http_cache_json<T: serde::de::DeserializeOwned>() -> Option<T> {
    let path = http_cache_path().ok()?;
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Persist the conditional-request cache. The caller is responsible for
/// bounding what it hands over.
pub fn save_http_cache_json<T: serde::Serialize>(value: &T) -> Result<()> {
    let path = http_cache_path()?;
    std::fs::write(&path, serde_json::to_string(value)?)?;
    Ok(())
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
    // `repo_name` and the install path both derive from remote data, and both are
    // interpolated into a line-oriented key=value format where glib keeps the
    // FIRST occurrence of a key. Unescaped, a newline in either injects arbitrary
    // keys - including an `Exec=` that shadows the legitimate one - and a quote in
    // the path closes the Exec quoting to append arguments. The icon path contains
    // repo_name too, so it goes through the same check rather than relying on
    // being emitted last.
    let name = desktop_value(repo_name)?;
    let exec = desktop_value(&exec_path.to_string_lossy())?;
    let icon_line = match repo_icon_dir(repo_name)
        .ok()
        .map(|d| d.join("icon.png"))
        .filter(|p| p.exists())
    {
        Some(p) => format!("Icon={}\n", desktop_value(&p.to_string_lossy())?),
        None => String::new(),
    };
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName={name}\nExec=\"{exec}\"\nTerminal=false\nCategories=Utility;\nComment=Installed by Colony\nX-Colony-Managed=true\n{icon_line}"
    );
    std::fs::write(dir.join(desktop_entry_filename(repo_name)?), entry)?;
    Ok(())
}

/// Escape a string for use as a quoted Desktop Entry value.
///
/// Per the Desktop Entry spec, a quoted argument must backslash-escape `"`, `` ` ``,
/// `$` and `\` - the last three because implementations may expand them. Control
/// characters have no valid escape and cannot appear in a value at all (a newline
/// would inject a whole new key, and glib keeps the FIRST occurrence of a key), so
/// they are rejected outright rather than mangled.
#[cfg(target_os = "linux")]
fn desktop_value(raw: &str) -> Result<String> {
    anyhow::ensure!(
        !raw.chars().any(|c| c.is_control()),
        "refusing to write a desktop entry containing control characters: {raw:?}"
    );
    let mut out = String::with_capacity(raw.len());
    for c in raw.chars() {
        if matches!(c, '\\' | '"' | '`' | '$') {
            out.push('\\');
        }
        // The Desktop Entry spec writes a literal percent as `%%`. Unescaped,
        // the launcher expands the FIELD CODE instead: a manifest declaring
        // `binary: "app%f"` yields Exec="/…/app%f", glib substitutes an empty
        // file list, and the entry silently launches the wrong path.
        if c == '%' {
            out.push('%');
        }
        out.push(c);
    }
    Ok(out)
}

#[cfg(not(target_os = "linux"))]
pub fn write_desktop_entry(_repo_name: &str, _exec_path: &std::path::Path) -> Result<()> {
    Ok(())
}

/// Remove the `.desktop` entry written by [`write_desktop_entry`] (no-op when
/// absent or on non-Linux platforms).
pub fn remove_desktop_entry(repo_name: &str) {
    #[cfg(target_os = "linux")]
    if let (Some(data), Ok(name)) = (dirs::data_dir(), desktop_entry_filename(repo_name)) {
        let path = data.join("applications").join(name);
        if path.exists() {
            if let Err(e) = std::fs::remove_file(&path) {
                tracing::warn!("failed to remove desktop entry {}: {e}", path.display());
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = repo_name;
}

/// Filename of a repo's `.desktop` entry, guarded because `repo_name` is remote
/// data and this string is joined into `~/.local/share/applications`: a separator
/// or `..` in it would let a manifest choose which entry to write or delete.
#[cfg(target_os = "linux")]
fn desktop_entry_filename(repo_name: &str) -> Result<String> {
    crate::download::ensure_safe_component(repo_name)?;
    Ok(format!("colony-{}.desktop", repo_name.to_lowercase()))
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

/// Delete staging leftovers from interrupted transfers, and report the bytes
/// reclaimed.
///
/// The only sweep that existed ran inside Cancel, so a crash, an OOM, a SIGKILL
/// or simply closing the window mid-download left the whole partial asset in
/// `apps/<repo>/` with no UI that showed or removed it. A `filePattern` app
/// whose asset name carries the version leaves one per version, so it
/// accumulates. Run once at boot: any staging file present then is by
/// definition orphaned, because nothing is in flight yet.
///
/// The `update-staging` directory gets the same treatment, plus the `.old`
/// backup a completed self-update leaves next to the executable - two full
/// copies of the launcher could otherwise sit on disk indefinitely.
pub fn prune_staging() -> u64 {
    fn sweep(dir: &std::path::Path, reclaimed: &mut u64) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !(name.ends_with(".part")
                || name.ends_with(".part.id")
                || name.ends_with(".new")
                || name.ends_with(".old"))
            {
                continue;
            }
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            if std::fs::remove_file(&path).is_ok() {
                tracing::info!("pruned staging leftover {}", path.display());
                *reclaimed += size;
            }
        }
    }

    let mut reclaimed = 0;
    if let Ok(apps) = colony_apps_dir() {
        if let Ok(entries) = std::fs::read_dir(&apps) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    sweep(&entry.path(), &mut reclaimed);
                }
            }
        }
    }
    if let Ok(base) = colony_data_dir() {
        sweep(&base.join("update-staging"), &mut reclaimed);
    }
    // Only our OWN backup, by exact path - never a directory sweep here. The
    // executable may well live in /usr/bin, where an extension-based sweep
    // would happily delete somebody else's `.old` file.
    if let Ok(exe) = std::env::current_exe() {
        for stale in [exe.with_extension("old"), exe.with_extension("new")] {
            let size = std::fs::metadata(&stale).map(|m| m.len()).unwrap_or(0);
            if stale.exists() && std::fs::remove_file(&stale).is_ok() {
                tracing::info!("pruned launcher leftover {}", stale.display());
                reclaimed += size;
            }
        }
    }
    reclaimed
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

    /// `repo_name` is remote data (the GitHub API listing, and `repos_cache.json`
    /// on top of it) and is joined into every per-repo path plus a
    /// `remove_dir_all`, so containment must not depend on GitHub's own naming
    /// rules.
    #[test]
    fn repo_component_refuses_anything_but_a_plain_name() {
        let base = PathBuf::from("/tmp/colony-test");
        assert_eq!(
            join_repo_component(base.clone(), "Colony").unwrap(),
            base.join("Colony")
        );
        for hostile in ["../../../../.local/bin", "..", "/etc", "a/b", "", "."] {
            assert!(
                join_repo_component(base.clone(), hostile).is_err(),
                "repo name {hostile:?} must be refused"
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn desktop_values_are_escaped_and_control_chars_refused() {
        // Quotes and backslashes are escaped rather than closing our Exec="..."
        assert_eq!(desktop_value(r#"a"b"#).unwrap(), r#"a\"b"#);
        assert_eq!(desktop_value(r"a\b").unwrap(), r"a\\b");
        assert_eq!(desktop_value("plain-name").unwrap(), "plain-name");
        // A newline would inject a whole key; glib keeps the FIRST Exec= it sees.
        assert!(desktop_value("app\nExec=sh").is_err());
        assert!(desktop_value("app\rExec=sh").is_err());
        assert!(desktop_value("app\0").is_err());
        // A literal percent must be doubled, or the field code is expanded.
        assert_eq!(desktop_value("app%f").unwrap(), "app%%f");
    }

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
