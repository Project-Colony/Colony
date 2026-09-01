//! Local persistence for Colony: data directories, config resolution, installed
//! app / version state, offline caches, favorites, and user preferences. This
//! module holds pure on-disk storage and does not touch the GitHub API.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::github::{current_platform_key, ColonyRepo};

/// Colony's own name in the shared `Colony/<Program>/` tree.
const PROGRAM: &str = "Colony";

/// Central config directory for Colony's own files.
///
/// The layout — which root on which platform — is defined once in colony-ui;
/// see `design/filesystem.md` in Project-Colony-Resources. On Linux this is
/// `~/.config/Colony/Colony/`.
pub fn colony_data_dir() -> Result<PathBuf> {
    Ok(colony_ui::paths::config_dir(PROGRAM)?)
}

/// Regenerable state: the repo listing and the scan results.
pub(crate) fn colony_cache_dir() -> Result<PathBuf> {
    Ok(colony_ui::paths::cache_dir(PROGRAM)?)
}

/// Move state written by earlier versions to where the shared layout puts it.
///
/// Must run before anything reads a path — `main` calls it first.
///
/// Deliberately conservative: it only moves when the old location exists and
/// the new one does not, and it **never deletes the source**. A user who ends
/// up with a copy in both places has lost nothing; a user whose preferences
/// were deleted by a half-finished migration has.
pub fn migrate_legacy_paths() {
    // Windows used to resolve config to Roaming (dirs::config_dir), and the
    // layout is Local. Identical on Linux and macOS, so this is a no-op there.
    if let (Some(legacy_root), Ok(current)) = (
        dirs::config_dir(),
        colony_ui::paths::locate::config_dir(PROGRAM),
    ) {
        relocate(
            &legacy_root.join("Colony").join(PROGRAM),
            &current,
            "config directory",
        );
    }

    // Everything regenerable used to live inside the config directory. All of
    // it is re-fetched when missing, so a failure here costs a round trip to
    // GitHub and nothing more.
    if let (Ok(config), Ok(cache)) = (
        colony_ui::paths::locate::config_dir(PROGRAM),
        colony_ui::paths::locate::cache_dir(PROGRAM),
    ) {
        relocate(&config.join("cache"), &cache, "cache");
        for sub in ["repo-docs", "repo-icons", "update-staging"] {
            relocate(&config.join(sub), &cache.join(sub), sub);
        }
    }
}

/// Move `from` to `to`, once, without ever destroying `from`.
fn relocate(from: &Path, to: &Path, what: &str) {
    if from == to || !from.is_dir() || to.exists() {
        return;
    }
    let Some(parent) = to.parent() else { return };
    if let Err(e) = std::fs::create_dir_all(parent) {
        tracing::warn!("cannot prepare {} for the {what}: {e}", parent.display());
        return;
    }

    // A rename is atomic and cheap, but fails across filesystems (EXDEV) — and
    // ~/.config and ~/.cache are not guaranteed to be on the same one.
    if std::fs::rename(from, to).is_ok() {
        tracing::info!("moved the {what} to {}", to.display());
        return;
    }
    match copy_tree(from, to) {
        Ok(()) => tracing::info!(
            "copied the {what} to {}; the old copy at {} is left in place and can be deleted by hand",
            to.display(),
            from.display()
        ),
        Err(e) => {
            // Leave no half-copied directory behind: the next start would see
            // `to` existing and skip the migration, stranding the real data.
            let _ = std::fs::remove_dir_all(to);
            tracing::error!(
                "could not move the {what} from {}: {e}. Nothing was lost — the old                  location still holds it — but Colony will start with an empty one.",
                from.display()
            );
        }
    }
}

/// Recursive copy. Files and directories only; anything else is skipped.
fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else if kind.is_file() {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Write `bytes` to `path` atomically: a temp sibling, then a rename.
///
/// Every state writer here used a bare `std::fs::write`, so a crash, an OOM
/// kill or power loss mid-write left exactly the truncated JSON that the
/// loaders then treated as "no file" - which the next save overwrote with
/// defaults, destroying any chance of recovery. The install path already knew
/// better: it stages and renames precisely so an interrupted write cannot
/// truncate the target.
fn write_atomic(path: &std::path::Path, bytes: &[u8]) -> Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Parse JSON from `path`, telling "absent" apart from "corrupt".
///
/// On a parse failure the file is moved aside to `<name>.corrupt` before
/// defaults are returned, so the next save cannot clobber it and the user has
/// something to attach to a bug report.
fn load_json_or_quarantine<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Option<T> {
    let content = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str(&content) {
        Ok(value) => Some(value),
        Err(e) => {
            let quarantine = path.with_extension("corrupt");
            tracing::warn!(
                "{} is not valid JSON ({e}); moving it to {} and starting from defaults",
                path.display(),
                quarantine.display()
            );
            let _ = std::fs::rename(path, &quarantine);
            None
        }
    }
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

/// Cached repo documentation: `<cache>/repo-docs/{repo_name}/`. Re-fetched from
/// GitHub when missing, so it is cache rather than config.
fn repo_docs_dir(repo_name: &str) -> Result<PathBuf> {
    let base = join_repo_component(colony_cache_dir()?.join("repo-docs"), repo_name)?;
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

/// Cached per-repo app icon: `<cache>/repo-icons/{repo_name}/`. Re-downloaded
/// when missing, so it is cache rather than config.
fn repo_icon_dir(repo_name: &str) -> Result<PathBuf> {
    let base = join_repo_component(colony_cache_dir()?.join("repo-icons"), repo_name)?;
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

/// The shared install root: `<data>/Colony/apps/`.
///
/// Deliberately a sibling of Colony's own directory rather than a child —
/// installed programs belong to the ecosystem, not to the launcher. Does not
/// create the directory; callers that write do that themselves.
pub fn colony_apps_dir() -> Result<PathBuf> {
    Ok(colony_ui::paths::locate::apps_dir()?)
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

/// Marker recording that this app was installed with a verified signed METADATA
/// sidecar, not just a bare signature.
const META_FILE: &str = ".colony_meta";

/// Record that the installed build carried a verified `.meta` sidecar.
///
/// Same "only ever raises the bar" contract as [`save_installed_signed`]: a
/// bare `.sig` proves the bytes came from the org key, but not WHICH artefact
/// or version they are, so a compromised maintainer can re-upload an old,
/// genuinely signed, known-vulnerable build under a new tag. Once an app has
/// been installed with a sidecar, later updates must keep providing one - so
/// the replay cannot be performed by simply dropping the sidecar.
///
/// Never cleared while the app stays installed; uninstalling is the deliberate
/// way out.
pub fn save_installed_metadata(repo_name: &str) -> Result<()> {
    let path = colony_app_dir(repo_name)?.join(META_FILE);
    std::fs::write(&path, "1")?;
    Ok(())
}

/// True when a previous install of this repo carried a verified sidecar.
pub fn load_installed_metadata(repo_name: &str) -> bool {
    colony_app_dir(repo_name)
        .map(|d| d.join(META_FILE).exists())
        .unwrap_or(false)
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
    Ok(colony_cache_dir()?.join("repos_cache.json"))
}

/// Save Colony repos to local cache for offline use.
pub fn save_repos_cache(repos: &[ColonyRepo]) -> Result<()> {
    let path = repos_cache_path()?;
    write_atomic(&path, serde_json::to_string(repos)?.as_bytes())?;
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
    Ok(colony_cache_dir()?.join("http_etags.json"))
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
    write_atomic(&path, serde_json::to_string(value)?.as_bytes())
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
        .and_then(|p| load_json_or_quarantine(&p))
        .unwrap_or_default()
}

/// Save the list of favorite application names.
pub fn save_favorites(favorites: &[String]) -> Result<()> {
    let path = favorites_path()?;
    write_atomic(&path, serde_json::to_string(favorites)?.as_bytes())
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
    /// Whether the accent follows the theme. The toggle existed and was
    /// applied at boot, but was never written, so it silently reset every
    /// restart.
    pub auto_accent: Option<bool>,
    // General
    pub restore_session: Option<bool>,
    pub default_view: Option<String>,
    pub language: Option<String>,
    pub auto_check_updates: Option<bool>,
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
        .and_then(|p| load_json_or_quarantine(&p))
        .unwrap_or_default()
}

/// Save user preferences.
pub fn save_preferences(prefs: &UserPreferences) -> Result<()> {
    let path = preferences_path()?;
    write_atomic(&path, serde_json::to_string_pretty(prefs)?.as_bytes())
}

fn scan_cache_path() -> Result<PathBuf> {
    Ok(colony_cache_dir()?.join("scan_cache.json"))
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
pub fn write_desktop_entry(
    repo_name: &str,
    display_name: &str,
    category: crate::scan::AppCategory,
    exec_path: &std::path::Path,
) -> Result<()> {
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
    // The FILENAME keys off the slug (identity, and what remove_desktop_entry
    // looks up); the Name= the user reads is the manifest's display name.
    let name = desktop_value(if display_name.trim().is_empty() {
        repo_name
    } else {
        display_name
    })?;
    let exec = desktop_value(&exec_path.to_string_lossy())?;
    let icon_line = match repo_icon_dir(repo_name)
        .ok()
        .map(|d| d.join("icon.png"))
        .filter(|p| p.exists())
    {
        Some(p) => format!("Icon={}\n", desktop_value(&p.to_string_lossy())?),
        None => String::new(),
    };
    let categories = category.desktop_categories();
    let entry = format!(
        "[Desktop Entry]\nType=Application\nName={name}\nExec=\"{exec}\"\nTerminal=false\nCategories={categories}\nComment=Installed by Colony\nX-Colony-Managed=true\n{icon_line}"
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
pub fn write_desktop_entry(
    _repo_name: &str,
    _display_name: &str,
    _category: crate::scan::AppCategory,
    _exec_path: &std::path::Path,
) -> Result<()> {
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
    let Ok(base) = colony_cache_dir() else {
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

/// Remove an installed app's directory, coping with a binary that is currently
/// running.
///
/// `remove_dir_all` cannot delete a live executable on Windows, so uninstalling
/// an app the user had left open failed outright and left it half-removed. A
/// running image can always be RENAMED though, so fall back to renaming each
/// file aside: the directory then empties and [`prune_staging`] collects the
/// leftovers at the next start, once the process holding them has exited.
pub fn remove_app_dir(app_dir: &std::path::Path) -> std::io::Result<()> {
    match std::fs::remove_dir_all(app_dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(first) => {
            let Ok(entries) = std::fs::read_dir(app_dir) else {
                return Err(first);
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && std::fs::remove_file(&path).is_err() {
                    let _ = std::fs::rename(&path, path.with_extension("old"));
                }
            }
            // Second pass: everything deletable is gone, and anything renamed
            // aside is now a `.old` the boot sweep will take.
            std::fs::remove_dir_all(app_dir).or(Err(first))
        }
    }
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
                    // An uninstall that could not delete a running binary
                    // renames it aside and leaves the shell of the directory;
                    // once the sweep above has taken the `.old` file, finish
                    // the job.
                    if std::fs::read_dir(entry.path())
                        .map(|mut d| d.next().is_none())
                        .unwrap_or(false)
                    {
                        let _ = std::fs::remove_dir(entry.path());
                    }
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
    let Ok(base) = colony_cache_dir() else {
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
    write_atomic(&path, serde_json::to_string(&entry)?.as_bytes())?;
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

#[cfg(test)]
mod path_migration_tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("colony_migrate_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("scratch dir");
        dir
    }

    fn seed(dir: &Path, rel: &str, contents: &str) {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    #[test]
    fn moves_a_legacy_directory_and_keeps_its_contents() {
        let root = scratch("moves");
        let (from, to) = (root.join("old"), root.join("new"));
        seed(
            &from,
            "preferences/preferences.json",
            "{\"theme\":\"gruvbox\"}",
        );
        seed(&from, "auth/github_token.json", "token");

        relocate(&from, &to, "config directory");

        assert!(!from.exists(), "the source should have been renamed away");
        assert_eq!(
            std::fs::read_to_string(to.join("preferences/preferences.json")).unwrap(),
            "{\"theme\":\"gruvbox\"}"
        );
        assert!(to.join("auth/github_token.json").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn never_clobbers_an_existing_destination() {
        let root = scratch("clobber");
        let (from, to) = (root.join("old"), root.join("new"));
        seed(&from, "preferences.json", "old");
        seed(&to, "preferences.json", "current");

        relocate(&from, &to, "config directory");

        // The current data wins and the old copy is left untouched, not merged.
        assert_eq!(
            std::fs::read_to_string(to.join("preferences.json")).unwrap(),
            "current"
        );
        assert_eq!(
            std::fs::read_to_string(from.join("preferences.json")).unwrap(),
            "old"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn does_nothing_on_a_fresh_install_or_a_second_run() {
        let root = scratch("noop");
        let (from, to) = (root.join("absent"), root.join("new"));

        relocate(&from, &to, "config directory");

        assert!(!to.exists(), "nothing to migrate should create nothing");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_source_equal_to_the_destination_is_left_alone() {
        // This is the Linux case for the config directory: the old and new
        // resolvers return the same path, so the migration must be inert.
        let root = scratch("same");
        let dir = root.join("config");
        seed(&dir, "preferences.json", "kept");

        relocate(&dir, &dir, "config directory");

        assert_eq!(
            std::fs::read_to_string(dir.join("preferences.json")).unwrap(),
            "kept"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn copy_tree_reproduces_nested_contents() {
        let root = scratch("copy");
        let (from, to) = (root.join("src"), root.join("dst"));
        seed(&from, "a.json", "a");
        seed(&from, "deep/b.json", "b");
        seed(&from, "deep/deeper/c.json", "c");

        copy_tree(&from, &to).expect("copy");

        for (rel, want) in [
            ("a.json", "a"),
            ("deep/b.json", "b"),
            ("deep/deeper/c.json", "c"),
        ] {
            assert_eq!(
                std::fs::read_to_string(to.join(rel)).unwrap(),
                want,
                "{rel}"
            );
        }
        // The source survives a copy — that is the whole point of the fallback.
        assert!(from.join("a.json").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The unit tests above cover `relocate` and `copy_tree` in isolation.
    /// This exercises `migrate_legacy_paths` itself — the thing that actually
    /// runs on a user's machine — against a planted legacy layout.
    ///
    /// Linux only: it drives the XDG variables, and on Linux the config root is
    /// unchanged by the migration, so what it proves is the cache move, which
    /// is the part that relocates real files for the majority of users.
    #[cfg(target_os = "linux")]
    #[test]
    fn migrating_a_real_legacy_layout_moves_the_caches_and_leaves_config_alone() {
        // Driving process-wide environment variables; must not race the other
        // tests that read a path.
        static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let root = scratch("e2e");
        let (old_config, old_data, old_cache) = (
            std::env::var_os("XDG_CONFIG_HOME"),
            std::env::var_os("XDG_DATA_HOME"),
            std::env::var_os("XDG_CACHE_HOME"),
        );
        std::env::set_var("XDG_CONFIG_HOME", root.join("config"));
        std::env::set_var("XDG_DATA_HOME", root.join("data"));
        std::env::set_var("XDG_CACHE_HOME", root.join("cache"));

        // A profile as an older Colony left it: everything under config/.
        let legacy = root.join("config/Colony/Colony");
        seed(
            &legacy,
            "preferences/preferences.json",
            "{\"theme\":\"gruvbox\"}",
        );
        seed(&legacy, "auth/github_token.json", "token");
        seed(&legacy, "cache/repos_cache.json", "[]");
        seed(&legacy, "cache/http_etags.json", "{}");
        seed(&legacy, "repo-docs/Eidos/README.md", "# Eidos");
        seed(&legacy, "repo-icons/Grape/icon.png", "png");
        seed(&legacy, "update-staging/colony-linux", "binary");

        migrate_legacy_paths();

        let cache = root.join("cache/Colony/Colony");
        for (rel, want) in [
            ("repos_cache.json", "[]"),
            ("http_etags.json", "{}"),
            ("repo-docs/Eidos/README.md", "# Eidos"),
            ("repo-icons/Grape/icon.png", "png"),
            ("update-staging/colony-linux", "binary"),
        ] {
            assert_eq!(
                std::fs::read_to_string(cache.join(rel)).unwrap_or_default(),
                want,
                "{rel} should have moved to the cache root"
            );
        }

        // What the user chose stays put, and stays readable.
        assert_eq!(
            std::fs::read_to_string(legacy.join("preferences/preferences.json")).unwrap(),
            "{\"theme\":\"gruvbox\"}"
        );
        assert!(legacy.join("auth/github_token.json").exists());

        // Nothing regenerable is left behind to be read again by mistake.
        assert!(
            !legacy.join("cache").exists(),
            "the old cache dir should be gone"
        );
        assert!(!legacy.join("repo-docs").exists());

        // Running twice is inert.
        migrate_legacy_paths();
        assert!(cache.join("repos_cache.json").exists());

        for (k, v) in [
            ("XDG_CONFIG_HOME", old_config),
            ("XDG_DATA_HOME", old_data),
            ("XDG_CACHE_HOME", old_cache),
        ] {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn colony_lives_under_the_shared_tree() {
        let config = colony_ui::paths::locate::config_dir(PROGRAM).unwrap();
        assert!(
            config.ends_with("Colony/Colony") || config.ends_with("Colony\\Colony"),
            "{config:?}"
        );

        // apps/ is a SIBLING of Colony's own data directory, not a child of
        // it: uninstalling the launcher must not look like it takes the
        // installed programs with it.
        let apps = colony_apps_dir().unwrap();
        let data = colony_ui::paths::locate::data_dir(PROGRAM).unwrap();
        assert!(
            apps.ends_with("Colony/apps") || apps.ends_with("Colony\\apps"),
            "{apps:?}"
        );
        assert!(
            !apps.starts_with(&data),
            "installed programs must not live inside Colony's own directory"
        );
        assert_eq!(
            apps.parent(),
            data.parent(),
            "apps/ and Colony/ share a parent"
        );
    }
}
