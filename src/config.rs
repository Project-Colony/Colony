//! Locating Colony's external configuration files.
//!
//! Colony ships default `config/*.toml` / `config/*.json` files in the repo, but a
//! released build is a single executable launched from an arbitrary working
//! directory, so a bare `config/<name>` relative path only ever resolves when the
//! app runs from the repo root (`cargo run`). To make configuration reachable for
//! installed binaries we search a few well-known locations and let callers fall back
//! to embedded/built-in defaults when nothing is found.

use std::path::PathBuf;

/// Resolve an external config file named `file_name` (e.g. `"categories.json"`).
///
/// Search order, first existing file wins:
///   1. `~/.config/colony/<file_name>` — per-user override (XDG config dir).
///   2. `<exe_dir>/config/<file_name>` — config shipped next to the binary.
///   3. `./config/<file_name>`         — current working dir (dev: `cargo run`).
///
/// Returns `None` when no candidate exists; callers should then use their embedded
/// or built-in defaults.
pub fn resolve_config_path(file_name: &str) -> Option<PathBuf> {
    first_existing(candidate_paths(file_name))
}

/// Build the ordered list of candidate paths for `file_name` (see [`resolve_config_path`]).
fn candidate_paths(file_name: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    // 1. ~/.config/colony/<file_name>
    if let Some(config_dir) = dirs::config_dir() {
        candidates.push(config_dir.join("colony").join(file_name));
    }

    // 2. <exe_dir>/config/<file_name>
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            candidates.push(exe_dir.join("config").join(file_name));
        }
    }

    // 3. ./config/<file_name> (dev convenience)
    candidates.push(PathBuf::from("config").join(file_name));

    candidates
}

/// First path in `candidates` that exists as a file.
fn first_existing(candidates: Vec<PathBuf>) -> Option<PathBuf> {
    candidates.into_iter().find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_paths_are_ordered_and_end_with_cwd() {
        let candidates = candidate_paths("categories.json");
        assert!(!candidates.is_empty());
        // The dev/CWD fallback is always the last resort.
        assert_eq!(
            candidates.last().unwrap(),
            &PathBuf::from("config").join("categories.json")
        );
    }

    #[test]
    fn first_existing_returns_first_present_file() {
        let dir = std::env::temp_dir();
        let present = dir.join("colony_resolve_present.json");
        let missing = dir.join("colony_resolve_missing_first.json");
        std::fs::write(&present, "[]").expect("write temp file");
        let _ = std::fs::remove_file(&missing);

        let resolved = first_existing(vec![missing, present.clone()]);
        assert_eq!(resolved.as_deref(), Some(present.as_path()));

        let _ = std::fs::remove_file(&present);
    }

    #[test]
    fn first_existing_none_when_all_absent() {
        let missing = std::env::temp_dir().join("colony_resolve_definitely_absent_9f3a.json");
        let _ = std::fs::remove_file(&missing);
        assert_eq!(first_existing(vec![missing]), None);
    }
}
