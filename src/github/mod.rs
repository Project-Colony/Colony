//! GitHub integration: the store catalog and release resolution.
//!
//! Split by layer - [`http`] owns the client and cache, [`types`] the wire
//! shapes, [`catalog`] the store listing, [`releases`] tag and asset resolution.
//! Everything the rest of the crate uses is re-exported here, so call sites keep
//! naming `crate::github::X` whichever file X lives in.

mod catalog;
mod http;
mod releases;
mod types;

pub use catalog::*;
pub use http::*;
pub use releases::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;

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
        assert_eq!(
            manifest.release_files["windows"].file.as_deref(),
            Some("TestApp.exe")
        );
        assert_eq!(manifest.release_files["linux"].tag, "Linux");
        assert_eq!(
            manifest.release_files["linux"].file.as_deref(),
            Some("TestApp")
        );
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
        assert_eq!(
            win.file.as_deref(),
            Some("lilypad-x86_64-pc-windows-msvc.zip")
        );
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
    fn is_not_found_matches_typed_status_not_message_text() {
        // A 404 is recognized through the typed status...
        let e404 = anyhow::Error::new(HttpStatus(404)).context("GitHub API error 404: Not Found");
        assert!(is_not_found(&e404));
        // ...a different status is not, even if its BODY contains "404" (the
        // old substring check misclassified this and dropped live repos).
        let e500 = anyhow::Error::new(HttpStatus(500))
            .context("GitHub API error 500: upstream said 404 somewhere");
        assert!(!is_not_found(&e500));
        // ...and a plain network error without a status is not a 404 either.
        assert!(!is_not_found(&anyhow::anyhow!("Network error: dns 404ish")));
    }

    #[test]
    fn find_asset_by_pattern_exact_name_beats_substring_overlap() {
        // "app-macos" is a substring of "app-macos-x86": an exact name match
        // must win instead of erroring as ambiguous, so Apple Silicon
        // manifests can pin the shorter asset name.
        let assets = vec!["app-macos".to_string(), "app-macos-x86".to_string()];
        assert_eq!(
            find_asset_by_pattern(&assets, "app-macos").unwrap(),
            "app-macos"
        );
        assert_eq!(
            find_asset_by_pattern(&assets, "app-macos-x86").unwrap(),
            "app-macos-x86"
        );
    }

    #[test]
    fn spec_conformant_manifest_parses_field_for_field() {
        // Locks docs/colony-spec.md <-> code parity: this sample uses every
        // documented manifest field with the spec's exact camelCase names.
        // If a rename or removal breaks the spec, this test fails first.
        let json = r#"{
            "name": "Lilypad",
            "category": "Security",
            "platforms": ["windows", "linux", "macos", "macos-x86"],
            "icon": "assets/icons/icon.png",
            "signed": true,
            "releaseFiles": {
                "linux": {
                    "tag": "latest",
                    "filePattern": "lilypad-*-linux.tar.gz, !*-arm64*",
                    "binary": "lilypad-cli",
                    "sha256": "abc123"
                },
                "windows": {
                    "tag": "v1.0.0",
                    "file": "lilypad-windows.zip",
                    "binary": "lilypad-cli.exe"
                }
            }
        }"#;
        let m: ColonyManifest = serde_json::from_str(json).expect("spec sample must parse");
        assert_eq!(m.name, "Lilypad");
        assert_eq!(m.category, "Security");
        assert_eq!(m.platforms.len(), 4);
        assert_eq!(m.icon.as_deref(), Some("assets/icons/icon.png"));
        assert!(m.signed);
        let linux = &m.release_files["linux"];
        assert_eq!(linux.tag, "latest");
        assert_eq!(
            linux.file_pattern.as_deref(),
            Some("lilypad-*-linux.tar.gz, !*-arm64*")
        );
        assert_eq!(linux.binary.as_deref(), Some("lilypad-cli"));
        assert_eq!(linux.sha256.as_deref(), Some("abc123"));
        let windows = &m.release_files["windows"];
        assert_eq!(windows.tag, "v1.0.0");
        assert_eq!(windows.file.as_deref(), Some("lilypad-windows.zip"));
        // Every spec category value (and its documented aliases) maps to a
        // real category - never silently to Other (except Other itself).
        for cat in [
            "Development",
            "Graphics",
            "Network",
            "Office",
            "Multimedia",
            "System",
            "Utility",
            "Utilities",
            "Security",
            "Game",
            "Games",
        ] {
            assert_ne!(
                crate::scan::AppCategory::from_name(cat),
                crate::scan::AppCategory::Other,
                "spec category '{cat}' must not fall back to Other"
            );
        }
    }

    #[test]
    fn manifest_signed_flag_parses_and_defaults_off() {
        let json = r#"{ "name": "App", "category": "Utility", "signed": true }"#;
        let m: ColonyManifest = serde_json::from_str(json).unwrap();
        assert!(m.signed);
        let json = r#"{ "name": "App", "category": "Utility" }"#;
        let m: ColonyManifest = serde_json::from_str(json).unwrap();
        assert!(!m.signed, "signed must default to false (legacy manifests)");
    }

    #[test]
    fn find_asset_by_pattern_glob_with_exclusion_resolves_electron_builder_layout() {
        // SphereCord's real release layout: electron-builder publishes both
        // architectures plus updater metadata. Substring matching could never
        // express this; an anchored glob with an exclusion can.
        let assets = vec![
            "SphereCord-3.2.7.AppImage".to_string(),
            "SphereCord-3.2.7-arm64.AppImage".to_string(),
            "SphereCord-Setup-3.2.7.exe".to_string(),
            "latest-linux.yml".to_string(),
            "spherecord-3.2.7.tar.gz".to_string(),
        ];
        assert_eq!(
            find_asset_by_pattern(&assets, "spherecord-*.appimage, !*-arm64*").unwrap(),
            "SphereCord-3.2.7.AppImage"
        );
        assert_eq!(
            find_asset_by_pattern(&assets, "*-arm64.appimage").unwrap(),
            "SphereCord-3.2.7-arm64.AppImage"
        );
    }

    #[test]
    fn find_asset_by_pattern_glob_is_anchored() {
        let assets = vec!["app-linux".to_string(), "app-linux-musl".to_string()];
        // Anchored: "*-linux" must NOT match "app-linux-musl".
        assert_eq!(
            find_asset_by_pattern(&assets, "*-linux").unwrap(),
            "app-linux"
        );
    }

    #[test]
    fn find_asset_by_pattern_ignores_signature_and_checksum_siblings() {
        // The day a repo signs its releases (like Colony itself), every binary
        // grows a .sig sibling containing the same name: the pattern must
        // keep resolving to the binary, not error as ambiguous.
        let assets = vec![
            "app-linux".to_string(),
            "app-linux.sig".to_string(),
            "app-linux.sha256".to_string(),
            "latest-linux.yml".to_string(),
        ];
        assert_eq!(
            find_asset_by_pattern(&assets, "linux").unwrap(),
            "app-linux"
        );
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
    fn base64_decode_manifest() {
        let json = r#"{"name":"Test","category":"Games","platforms":["linux"],"releaseFiles":{"linux":{"tag":"v1","file":"test"}}}"#;
        let encoded = base64::engine::general_purpose::STANDARD.encode(json);
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&encoded)
            .unwrap();
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
        let assets = vec!["orcal-linux".to_string(), "orcal-windows.exe".to_string()];
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
