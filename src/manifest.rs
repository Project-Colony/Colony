//! `colony validate-manifest`: the check every app in the org runs on its own
//! `colony.json` before it publishes a release.

use crate::github;

/// Report on a `colony.json`. Returns the process exit code: 0 clean, 1 invalid,
/// 2 unreadable.
pub fn validate(path: &str, assets: &[String]) -> i32 {
    let raw = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("cannot read {path}: {e}");
            return 2;
        }
    };
    let manifest: github::ColonyManifest = match serde_json::from_slice(&raw) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{path} is not a valid colony.json: {e}");
            return 1;
        }
    };
    let mut errors = manifest.validation_errors();

    if assets.is_empty() {
        if manifest.release_files.is_empty() {
            println!(
                "  note: no releaseFiles - Colony auto-detects platforms from asset names \
                 following the <name>-<platform> convention. Pass your release asset names \
                 (colony validate-manifest {path} <asset>...) to check that this actually \
                 resolves; without them this is a shape check only."
            );
        }
    } else {
        errors.extend(resolution_errors(&manifest, path, assets));
    }

    if errors.is_empty() {
        println!("{path}: OK ({}, {})", manifest.name, manifest.category);
        return 0;
    }
    eprintln!("{path}: {} problem(s)", errors.len());
    for e in &errors {
        eprintln!("  - {e}");
    }
    1
}

/// Check that the manifest resolves to something installable against the asset
/// names a release actually publishes.
///
/// This is the check that catches the failure that really happens: a manifest
/// declaring only a name and a category is structurally perfect, and still
/// renders as a card with no Download button because none of its assets match
/// the `<name>-<platform>` convention auto-detection looks for.
fn resolution_errors(
    manifest: &github::ColonyManifest,
    path: &str,
    assets: &[String],
) -> Vec<String> {
    let mut errors = Vec::new();

    if manifest.release_files.is_empty() {
        // Auto-detection is the only path. `<repo>` is not in the manifest, so
        // derive it the way Colony does: from the manifest name, then from the
        // file's own directory, since app authors run this in their repo root.
        let repo_hint = std::path::Path::new(path)
            .parent()
            .and_then(|p| p.canonicalize().ok())
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| manifest.name.clone());
        let detected = github::detect_platforms_from_assets(&repo_hint, assets);
        if detected.is_empty() {
            errors.push(format!(
                "no releaseFiles, and none of the {} published asset(s) match the \
                 <name>-<platform> convention for {repo_hint:?} (expected e.g. \
                 {}-linux). The app will list in Colony with no Download button. \
                 Declare releaseFiles explicitly, or rename the assets.",
                assets.len(),
                repo_hint.to_lowercase()
            ));
        } else {
            println!("  auto-detected platforms: {}", detected.join(", "));
        }
        return errors;
    }

    for (platform, entry) in &manifest.release_files {
        let at = format!("releaseFiles.{platform}");
        if let Some(file) = &entry.file {
            if !assets.iter().any(|a| a == file) {
                errors.push(format!(
                    "`{at}.file` is {file:?}, which this release does not publish"
                ));
            }
        } else if let Some(pattern) = &entry.file_pattern {
            match github::find_asset_by_pattern(assets, pattern) {
                Ok(resolved) => println!("  {platform} -> {resolved}"),
                Err(e) => errors.push(format!("`{at}.filePattern` does not resolve: {e}")),
            }
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::validate;

    /// Every org release runs `colony validate-manifest` and reads its exit
    /// code: 0 clean, 1 invalid, 2 unreadable.
    #[test]
    fn exit_codes() {
        let dir = tempfile::tempdir().unwrap();
        let write = |name: &str, json: &str| {
            let path = dir.path().join(name);
            std::fs::write(&path, json).unwrap();
            path.to_string_lossy().into_owned()
        };
        let assets =
            |names: &[&str]| -> Vec<String> { names.iter().map(|n| n.to_string()).collect() };

        let missing = dir.path().join("absent.json");
        assert_eq!(validate(&missing.to_string_lossy(), &[]), 2);

        assert_eq!(validate(&write("garbage.json", "not json"), &[]), 1);

        let empty_name = write("empty.json", r#"{"name": " ", "category": "games"}"#);
        assert_eq!(validate(&empty_name, &[]), 1);

        let minimal = write(
            "minimal.json",
            r#"{"name": "Grape", "category": "multimedia"}"#,
        );
        assert_eq!(validate(&minimal, &[]), 0, "a shape check without assets");

        let pinned = write(
            "pinned.json",
            r#"{
                "name": "Grape",
                "category": "multimedia",
                "releaseFiles": { "linux": { "tag": "latest", "file": "grape-linux" } }
            }"#,
        );
        assert_eq!(validate(&pinned, &assets(&["grape-linux"])), 0);
        assert_eq!(
            validate(&pinned, &assets(&["grape-windows.exe"])),
            1,
            "the release does not publish the file the manifest names"
        );
    }
}
