use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(not(windows))]
use std::sync::OnceLock;

use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Application {
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
    pub category: AppCategory,
    pub origin: AppOrigin,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppCategory {
    Development,
    Graphics,
    Network,
    Office,
    Multimedia,
    System,
    Utility,
    Game,
    Other,
}

impl AppCategory {
    /// Parse a category name (case-insensitive) into an AppCategory.
    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "development" => Self::Development,
            "graphics" => Self::Graphics,
            "network" => Self::Network,
            "office" => Self::Office,
            "multimedia" => Self::Multimedia,
            "system" => Self::System,
            "utility" | "utilities" => Self::Utility,
            "game" | "games" => Self::Game,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppOrigin {
    Windows,
    Colony,
    External,
    Linux,
}

#[derive(Debug, Deserialize)]
struct ColonyConfig {
    scan: Option<ScanConfig>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ScanConfig {
    windows: Option<Vec<String>>,
    unix: Option<Vec<String>>,
    colony: Option<Vec<String>>,
}

pub fn scan_applications() -> Result<Vec<Application>> {
    let mut apps = Vec::new();
    let mut seen_names: HashMap<String, bool> = HashMap::new();

    let search_dirs = get_application_dirs();

    for dir in &search_dirs {
        if dir.exists() {
            tracing::debug!("Scanning: {:?}", dir);
            match scan_directory(dir, &mut apps, &mut seen_names) {
                Ok(_) => tracing::debug!("Found {} apps so far", apps.len()),
                Err(e) => tracing::warn!("Error scanning {:?}: {}", dir, e),
            }
        } else {
            tracing::debug!("Directory does not exist: {:?}", dir);
        }
    }

    tracing::info!("Total applications found: {}", apps.len());
    apps.sort_by_key(|a| a.name.to_lowercase());
    Ok(apps)
}

#[cfg(windows)]
fn get_application_dirs() -> Vec<PathBuf> {
    if let Some(dirs) = load_scan_dirs_from_config() {
        return dirs;
    }

    default_windows_dirs()
}

#[cfg(windows)]
fn load_scan_dirs_from_config() -> Option<Vec<PathBuf>> {
    let path = Path::new("config/colony.toml");
    let content = fs::read_to_string(path).ok()?;
    let config: ColonyConfig = match toml::from_str(&content) {
        Ok(config) => config,
        Err(error) => {
            tracing::warn!("Invalid config {}: {}", path.display(), error);
            return None;
        }
    };
    let dirs = config.scan?.windows?;
    let expanded: Vec<PathBuf> = dirs
        .into_iter()
        .map(|dir| PathBuf::from(expand_env_vars(&dir)))
        .collect();
    if expanded.is_empty() {
        None
    } else {
        Some(expanded)
    }
}

#[cfg(windows)]
fn default_windows_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // Common Start Menu (all users)
    if let Ok(programdata) = std::env::var("ProgramData") {
        dirs.push(PathBuf::from(format!(
            "{}\\Microsoft\\Windows\\Start Menu\\Programs",
            programdata
        )));
    }

    // User Start Menu
    if let Ok(appdata) = std::env::var("APPDATA") {
        dirs.push(PathBuf::from(format!(
            "{}\\Microsoft\\Windows\\Start Menu\\Programs",
            appdata
        )));
    }

    dirs
}

#[cfg(not(windows))]
fn get_application_dirs() -> Vec<PathBuf> {
    let mut dirs = load_scan_dirs_from_config().unwrap_or_else(default_unix_dirs);
    for dir in colony_application_dirs() {
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    }
    dirs
}

#[cfg(not(windows))]
fn load_scan_dirs_from_config() -> Option<Vec<PathBuf>> {
    let path = Path::new("config/colony.toml");
    let content = fs::read_to_string(path).ok()?;
    let config: ColonyConfig = match toml::from_str(&content) {
        Ok(config) => config,
        Err(error) => {
            tracing::warn!("Invalid config {}: {}", path.display(), error);
            return None;
        }
    };
    let dirs = config.scan?.unix?;
    let expanded: Vec<PathBuf> = dirs
        .into_iter()
        .map(|dir| PathBuf::from(expand_env_vars(&dir)))
        .collect();
    if expanded.is_empty() {
        None
    } else {
        Some(expanded)
    }
}

#[cfg(not(windows))]
fn default_unix_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    #[cfg(target_os = "macos")]
    {
        // macOS application directories
        dirs.push(PathBuf::from("/Applications"));
        dirs.push(PathBuf::from("/System/Applications"));
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(format!("{}/Applications", home)));
        }
        // Homebrew Cask
        dirs.push(PathBuf::from("/opt/homebrew/Caskroom"));
        dirs.push(PathBuf::from("/usr/local/Caskroom"));
    }

    #[cfg(not(target_os = "macos"))]
    {
        // Linux: User applications
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(format!("{}/.local/share/applications", home)));
        }

        // XDG data dirs
        if let Ok(xdg_data_dirs) = std::env::var("XDG_DATA_DIRS") {
            for dir in xdg_data_dirs.split(':') {
                dirs.push(PathBuf::from(format!("{}/applications", dir)));
            }
        } else {
            dirs.push(PathBuf::from("/usr/share/applications"));
            dirs.push(PathBuf::from("/usr/local/share/applications"));
        }

        // Flatpak
        dirs.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(format!(
                "{}/.local/share/flatpak/exports/share/applications",
                home
            )));
        }

        // Snap
        dirs.push(PathBuf::from("/var/lib/snapd/desktop/applications"));
    }

    dirs
}

#[cfg(not(windows))]
fn colony_application_dirs() -> Vec<PathBuf> {
    static COLONY_DIRS: OnceLock<Vec<PathBuf>> = OnceLock::new();
    COLONY_DIRS
        .get_or_init(|| load_colony_dirs_from_config().unwrap_or_else(default_colony_dirs))
        .clone()
}

#[cfg(not(windows))]
fn load_colony_dirs_from_config() -> Option<Vec<PathBuf>> {
    let path = Path::new("config/colony.toml");
    let content = fs::read_to_string(path).ok()?;
    let config: ColonyConfig = match toml::from_str(&content) {
        Ok(config) => config,
        Err(error) => {
            tracing::warn!("Invalid config {}: {}", path.display(), error);
            return None;
        }
    };
    let dirs = config.scan?.colony?;
    let expanded: Vec<PathBuf> = dirs
        .into_iter()
        .map(|dir| PathBuf::from(expand_env_vars(&dir)))
        .collect();
    if expanded.is_empty() {
        None
    } else {
        Some(expanded)
    }
}

#[cfg(not(windows))]
fn default_colony_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(format!("{}/.local/share/colony/applications", home)));
    }

    dirs
}

#[cfg(not(windows))]
fn is_colony_app(path: &Path) -> bool {
    colony_application_dirs()
        .iter()
        .any(|dir| path.starts_with(dir))
}

fn expand_env_vars(value: &str) -> String {
    let mut output = String::new();
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' && matches!(chars.peek(), Some('{')) {
            chars.next();
            let mut name = String::new();
            let mut closed = false;
            for next in chars.by_ref() {
                if next == '}' {
                    closed = true;
                    break;
                }
                name.push(next);
            }
            if closed {
                if let Ok(value) = std::env::var(&name) {
                    output.push_str(&value);
                } else {
                    output.push_str("${");
                    output.push_str(&name);
                    output.push('}');
                }
            } else {
                output.push('$');
                output.push('{');
                output.push_str(&name);
            }
        } else if ch == '%' {
            let mut name = String::new();
            let mut closed = false;
            for next in chars.by_ref() {
                if next == '%' {
                    closed = true;
                    break;
                }
                name.push(next);
            }
            if closed {
                if let Ok(value) = std::env::var(&name) {
                    output.push_str(&value);
                } else {
                    output.push('%');
                    output.push_str(&name);
                    output.push('%');
                }
            } else {
                output.push('%');
                output.push_str(&name);
            }
        } else {
            output.push(ch);
        }
    }

    output
}

fn scan_directory(
    dir: &Path,
    apps: &mut Vec<Application>,
    seen: &mut HashMap<String, bool>,
) -> Result<()> {
    scan_directory_recursive(dir, apps, seen, 0)
}

fn scan_directory_recursive(
    dir: &Path,
    apps: &mut Vec<Application>,
    seen: &mut HashMap<String, bool>,
    depth: usize,
) -> Result<()> {
    if depth > 3 {
        return Ok(());
    }

    let entries = fs::read_dir(dir)?;

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            // macOS: treat .app bundles as applications
            #[cfg(target_os = "macos")]
            if path.extension().and_then(|e| e.to_str()) == Some("app") {
                if let Some(app) = parse_macos_app(&path) {
                    if !seen.contains_key(&app.name) {
                        seen.insert(app.name.clone(), true);
                        apps.push(app);
                    }
                }
                continue;
            }
            // Recurse into subdirectories (for Start Menu folders)
            let _ = scan_directory_recursive(&path, apps, seen, depth + 1);
        } else if let Some(app) = parse_application_file(&path) {
            if !seen.contains_key(&app.name) {
                seen.insert(app.name.clone(), true);
                apps.push(app);
            }
        }
    }

    Ok(())
}

#[cfg(windows)]
fn parse_application_file(path: &Path) -> Option<Application> {
    let ext = path.extension()?.to_str()?;

    if ext.eq_ignore_ascii_case("lnk") {
        parse_lnk_file(path)
    } else if ext.eq_ignore_ascii_case("exe") {
        parse_exe_file(path)
    } else {
        None
    }
}

#[cfg(windows)]
fn parse_lnk_file(path: &Path) -> Option<Application> {
    // Get the name from the filename (without .lnk extension)
    let name = path.file_stem()?.to_str()?.to_string();

    // Skip certain system entries
    let lower = name.to_lowercase();
    if lower.contains("uninstall")
        || lower.contains("readme")
        || lower.contains("help")
        || lower.contains("website")
        || lower.contains("manual")
        || lower.contains("license")
    {
        return None;
    }

    // Use the .lnk file path directly - Windows can execute it
    let exec = path.to_str()?.to_string();

    let category = categorize_windows_app(&name, &exec);

    Some(Application {
        name,
        exec,
        icon: None,
        category,
        origin: AppOrigin::Windows,
    })
}

#[cfg(windows)]
fn parse_exe_file(path: &Path) -> Option<Application> {
    let name = path.file_stem()?.to_str()?.to_string();
    let exec = path.to_str()?.to_string();

    Some(Application {
        name,
        exec,
        icon: None,
        category: AppCategory::Other,
        origin: AppOrigin::Windows,
    })
}

#[cfg(windows)]
fn categorize_windows_app(name: &str, exec: &str) -> AppCategory {
    let lower_name = name.to_lowercase();
    let lower_exec = exec.to_lowercase();

    if lower_name.contains("code") || lower_name.contains("studio")
        || lower_name.contains("developer") || lower_exec.contains("ide")
        || lower_name.contains("python") || lower_name.contains("node")
        || lower_name.contains("git") || lower_name.contains("terminal")
    {
        AppCategory::Development
    } else if lower_name.contains("photoshop") || lower_name.contains("gimp")
        || lower_name.contains("paint") || lower_name.contains("photo")
        || lower_name.contains("image") || lower_name.contains("draw")
    {
        AppCategory::Graphics
    } else if lower_name.contains("chrome") || lower_name.contains("firefox")
        || lower_name.contains("edge") || lower_name.contains("browser")
        || lower_name.contains("mail") || lower_name.contains("outlook")
        || lower_name.contains("teams") || lower_name.contains("slack")
        || lower_name.contains("discord") || lower_name.contains("zoom")
    {
        AppCategory::Network
    } else if lower_name.contains("word") || lower_name.contains("excel")
        || lower_name.contains("powerpoint") || lower_name.contains("office")
        || lower_name.contains("libre") || lower_name.contains("calc")
        || lower_name.contains("writer") || lower_name.contains("document")
    {
        AppCategory::Office
    } else if lower_name.contains("spotify") || lower_name.contains("vlc")
        || lower_name.contains("media") || lower_name.contains("player")
        || lower_name.contains("music") || lower_name.contains("video")
        || lower_name.contains("audio")
    {
        AppCategory::Multimedia
    } else if lower_name.contains("settings") || lower_name.contains("control")
        || lower_name.contains("system") || lower_name.contains("config")
        || lower_name.contains("manager") || lower_name.contains("monitor")
    {
        AppCategory::System
    } else if lower_name.contains("notepad") || lower_name.contains("calculator")
        || lower_name.contains("util") || lower_name.contains("tool")
        || lower_name.contains("7-zip") || lower_name.contains("winrar")
    {
        AppCategory::Utility
    } else if lower_name.contains("game") || lower_name.contains("steam")
        || lower_name.contains("epic") || lower_name.contains("play")
        || lower_exec.contains("game")
    {
        AppCategory::Game
    } else {
        AppCategory::Other
    }
}

#[cfg(not(windows))]
fn parse_application_file(path: &Path) -> Option<Application> {
    // macOS .app bundles are directories, handled in scan_directory_recursive
    let ext = path.extension()?.to_str()?;

    if ext == "desktop" {
        parse_desktop_file(path).ok()
    } else {
        None
    }
}

#[cfg(not(windows))]
fn parse_desktop_file(path: &Path) -> Result<Application> {
    let content = fs::read_to_string(path)?;

    let mut name = None;
    let mut exec = None;
    let mut icon = None;
    let mut categories = String::new();
    let mut no_display = false;
    let mut hidden = false;
    let mut in_desktop_entry = false;

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with('[') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }

        if !in_desktop_entry {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            match key {
                "Name" if name.is_none() => name = Some(value.to_string()),
                "Exec" => exec = Some(clean_exec(value)),
                "Icon" => icon = Some(value.to_string()),
                "Categories" => categories = value.to_string(),
                "NoDisplay" => no_display = value.eq_ignore_ascii_case("true"),
                "Hidden" => hidden = value.eq_ignore_ascii_case("true"),
                _ => {}
            }
        }
    }

    if no_display || hidden {
        anyhow::bail!("Application is hidden");
    }

    let name = name.ok_or_else(|| anyhow::anyhow!("No name found"))?;
    let exec = exec.ok_or_else(|| anyhow::anyhow!("No exec found"))?;

    let origin = if is_colony_app(path) {
        AppOrigin::Colony
    } else {
        AppOrigin::External
    };

    Ok(Application {
        name,
        exec,
        icon,
        category: categorize_linux_app(&categories),
        origin,
    })
}

#[cfg(not(windows))]
fn categorize_linux_app(categories: &str) -> AppCategory {
    let cats: Vec<&str> = categories.split(';').collect();

    if cats.iter().any(|c| matches!(*c, "Development" | "IDE")) {
        AppCategory::Development
    } else if cats.iter().any(|c| matches!(*c, "Graphics" | "Photography" | "2DGraphics" | "3DGraphics")) {
        AppCategory::Graphics
    } else if cats.iter().any(|c| matches!(*c, "Network" | "WebBrowser" | "Email" | "Chat")) {
        AppCategory::Network
    } else if cats.iter().any(|c| matches!(*c, "Office" | "WordProcessor" | "Spreadsheet")) {
        AppCategory::Office
    } else if cats.iter().any(|c| matches!(*c, "AudioVideo" | "Audio" | "Video" | "Player")) {
        AppCategory::Multimedia
    } else if cats.iter().any(|c| matches!(*c, "System" | "Settings" | "Monitor")) {
        AppCategory::System
    } else if cats.iter().any(|c| matches!(*c, "Utility" | "FileManager" | "Archiving")) {
        AppCategory::Utility
    } else if cats.iter().any(|c| matches!(*c, "Game")) {
        AppCategory::Game
    } else {
        AppCategory::Other
    }
}

#[cfg(not(windows))]
fn clean_exec(exec: &str) -> String {
    let mut result = String::new();
    let mut chars = exec.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            chars.next();
        } else {
            result.push(c);
        }
    }

    let trimmed = result.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    match shell_words::split(trimmed) {
        Ok(parts) => {
            let filtered: Vec<String> = parts.into_iter().filter(|part| !part.is_empty()).collect();
            if filtered.is_empty() {
                return String::new();
            }
            shell_words::join(&filtered)
        }
        Err(_) => trimmed.to_string(),
    }
}

#[cfg(target_os = "macos")]
fn parse_macos_app(path: &Path) -> Option<Application> {
    let name = path.file_stem()?.to_str()?.to_string();

    // Skip system utilities
    let lower = name.to_lowercase();
    if lower.contains("uninstall") || lower.contains("migration assistant") {
        return None;
    }

    // The exec is "open <path>"
    let exec = format!("open {}", shell_words::quote(path.to_str()?));

    let category = categorize_macos_app(&name);

    let origin = if is_colony_app(path) {
        AppOrigin::Colony
    } else {
        AppOrigin::External
    };

    Some(Application {
        name,
        exec,
        icon: None,
        category,
        origin,
    })
}

#[cfg(target_os = "macos")]
fn categorize_macos_app(name: &str) -> AppCategory {
    let lower = name.to_lowercase();
    if lower.contains("xcode") || lower.contains("terminal") || lower.contains("code")
        || lower.contains("developer") || lower.contains("git")
    {
        AppCategory::Development
    } else if lower.contains("preview") || lower.contains("photo")
        || lower.contains("image") || lower.contains("sketch") || lower.contains("pixelmator")
    {
        AppCategory::Graphics
    } else if lower.contains("safari") || lower.contains("chrome") || lower.contains("firefox")
        || lower.contains("mail") || lower.contains("messages") || lower.contains("slack")
        || lower.contains("discord") || lower.contains("zoom")
    {
        AppCategory::Network
    } else if lower.contains("pages") || lower.contains("numbers") || lower.contains("keynote")
        || lower.contains("word") || lower.contains("excel")
    {
        AppCategory::Office
    } else if lower.contains("music") || lower.contains("quicktime") || lower.contains("vlc")
        || lower.contains("spotify") || lower.contains("garageband")
    {
        AppCategory::Multimedia
    } else if lower.contains("system") || lower.contains("settings") || lower.contains("preferences")
        || lower.contains("monitor") || lower.contains("disk utility")
    {
        AppCategory::System
    } else if lower.contains("calculator") || lower.contains("archive")
        || lower.contains("utility") || lower.contains("finder")
    {
        AppCategory::Utility
    } else if lower.contains("game") || lower.contains("chess") || lower.contains("steam") {
        AppCategory::Game
    } else {
        AppCategory::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- expand_env_vars tests ---

    #[test]
    fn expand_env_vars_with_set_var() {
        std::env::set_var("COLONY_TEST_VAR", "/test/path");
        assert_eq!(expand_env_vars("${COLONY_TEST_VAR}/bin"), "/test/path/bin");
        std::env::remove_var("COLONY_TEST_VAR");
    }

    #[test]
    fn expand_env_vars_with_unset_var() {
        std::env::remove_var("COLONY_UNSET_VAR");
        assert_eq!(expand_env_vars("${COLONY_UNSET_VAR}/bin"), "${COLONY_UNSET_VAR}/bin");
    }

    #[test]
    fn expand_env_vars_windows_style() {
        std::env::set_var("COLONY_TEST_WIN", "C:\\Users");
        assert_eq!(expand_env_vars("%COLONY_TEST_WIN%\\test"), "C:\\Users\\test");
        std::env::remove_var("COLONY_TEST_WIN");
    }

    #[test]
    fn expand_env_vars_no_vars() {
        assert_eq!(expand_env_vars("/usr/share/applications"), "/usr/share/applications");
    }

    #[test]
    fn expand_env_vars_unclosed_brace() {
        assert_eq!(expand_env_vars("${UNCLOSED"), "${UNCLOSED");
    }

    #[test]
    fn expand_env_vars_unclosed_percent() {
        assert_eq!(expand_env_vars("%UNCLOSED"), "%UNCLOSED");
    }

    #[test]
    fn expand_env_vars_multiple() {
        std::env::set_var("COLONY_A", "hello");
        std::env::set_var("COLONY_B", "world");
        assert_eq!(expand_env_vars("${COLONY_A}/${COLONY_B}"), "hello/world");
        std::env::remove_var("COLONY_A");
        std::env::remove_var("COLONY_B");
    }

    // --- categorize_linux_app tests ---

    #[cfg(not(windows))]
    #[test]
    fn categorize_development() {
        assert_eq!(categorize_linux_app("Development;IDE;"), AppCategory::Development);
    }

    #[cfg(not(windows))]
    #[test]
    fn categorize_graphics() {
        assert_eq!(categorize_linux_app("Graphics;Photography;"), AppCategory::Graphics);
    }

    #[cfg(not(windows))]
    #[test]
    fn categorize_network() {
        assert_eq!(categorize_linux_app("Network;WebBrowser;"), AppCategory::Network);
    }

    #[cfg(not(windows))]
    #[test]
    fn categorize_multimedia() {
        assert_eq!(categorize_linux_app("AudioVideo;Player;"), AppCategory::Multimedia);
    }

    #[cfg(not(windows))]
    #[test]
    fn categorize_other() {
        assert_eq!(categorize_linux_app("SomethingRandom;"), AppCategory::Other);
    }

    #[cfg(not(windows))]
    #[test]
    fn categorize_empty() {
        assert_eq!(categorize_linux_app(""), AppCategory::Other);
    }

    // --- clean_exec tests ---

    #[cfg(not(windows))]
    #[test]
    fn clean_exec_removes_field_codes() {
        assert_eq!(clean_exec("firefox %u"), "firefox");
    }

    #[cfg(not(windows))]
    #[test]
    fn clean_exec_preserves_args() {
        assert_eq!(clean_exec("/usr/bin/app --flag value"), "/usr/bin/app --flag value");
    }

    #[cfg(not(windows))]
    #[test]
    fn clean_exec_empty() {
        assert_eq!(clean_exec(""), "");
    }

    #[cfg(not(windows))]
    #[test]
    fn clean_exec_only_field_code() {
        assert_eq!(clean_exec("%f"), "");
    }

    // --- AppCategory equality ---

    #[test]
    fn app_category_equality() {
        assert_eq!(AppCategory::Development, AppCategory::Development);
        assert_ne!(AppCategory::Development, AppCategory::Graphics);
    }
}
