//! Wire types: the shape of `colony.json` and of the GitHub API responses this
//! module deserializes. No behaviour lives here.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Per-platform release info from colony.json.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseFileEntry {
    pub tag: String,
    /// Exact asset filename to download. Required unless `file_pattern` is set.
    pub file: Option<String>,
    /// Substring pattern to match against release asset names (case-insensitive).
    /// Colony fetches the release assets list and picks the one matching this pattern.
    /// Mutually exclusive with `file` — use one or the other.
    pub file_pattern: Option<String>,
    /// Optional binary name inside an archive. When present, the downloaded file
    /// is treated as an archive (.zip / .tar.gz) and Colony extracts this binary.
    /// When absent, the downloaded file is the final binary (legacy behaviour).
    pub binary: Option<String>,
    /// Optional SHA256 checksum for integrity verification.
    pub sha256: Option<String>,
}

/// Parsed manifest from colony.json inside a repo.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColonyManifest {
    pub name: String,
    pub category: String,
    #[serde(default)]
    pub platforms: Vec<String>,
    #[serde(default)]
    pub release_files: HashMap<String, ReleaseFileEntry>,
    /// Optional path (relative to the repo root) to a square PNG app icon shown
    /// in the Colony grid. When absent, Colony probes a conventional `icon.png`
    /// at the repo root, then falls back to the tinted category hexagon.
    #[serde(default)]
    pub icon: Option<String>,
    /// When true, every release asset MUST ship a valid `<asset>.sig`
    /// (ed25519, Project-Colony org key): a missing signature aborts the
    /// install instead of falling back to the legacy unsigned path.
    #[serde(default)]
    pub signed: bool,
}

/// The platform keys Colony understands. A manifest naming anything else is
/// declaring a platform no client will ever match.
pub const KNOWN_PLATFORMS: &[&str] = &["linux", "windows", "macos", "macos-x86"];

impl ColonyManifest {
    /// Check a manifest for the mistakes that make an app silently
    /// un-installable, returning one message per problem.
    ///
    /// Serde catches a manifest that does not PARSE. It cannot catch one that
    /// parses and then resolves to nothing, which is the failure that actually
    /// happens: of the eight manifests the org publishes, one declares only a
    /// name and a category and matches no release asset (so it renders as a
    /// card with no Download button), and nobody noticed, because there is no
    /// schema, no lint and no CI step anywhere on either side.
    ///
    /// Deliberately NOT called from the catalog fetch: a client refusing repos
    /// that a newer Colony would accept is worse than showing them. This is for
    /// `colony validate-manifest`, which app authors run in their own CI.
    pub fn validation_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.name.trim().is_empty() {
            errors.push("`name` is empty - it is the display name shown in Colony".into());
        }
        if self.category.trim().is_empty() {
            errors.push("`category` is empty".into());
        } else if matches!(
            crate::scan::AppCategory::from_name(&self.category),
            crate::scan::AppCategory::Other
        ) && !self.category.eq_ignore_ascii_case("other")
        {
            errors.push(format!(
                "`category` is {:?}, which Colony does not recognise - the app will be filed under Other. Known: development, graphics, network, office, multimedia, system, utilities, security, games, other",
                self.category
            ));
        }

        for platform in &self.platforms {
            if !KNOWN_PLATFORMS.contains(&platform.as_str()) {
                errors.push(format!(
                    "`platforms` names {platform:?}, which is not one of {KNOWN_PLATFORMS:?}"
                ));
            }
        }

        for (platform, entry) in &self.release_files {
            let at = format!("releaseFiles.{platform}");
            if !KNOWN_PLATFORMS.contains(&platform.as_str()) {
                errors.push(format!(
                    "`{at}` is not one of {KNOWN_PLATFORMS:?}, so no client will ever select it"
                ));
            }
            if entry.tag.trim().is_empty() {
                errors.push(format!(
                    "`{at}.tag` is empty - use \"latest\" or a real tag"
                ));
            }
            match (&entry.file, &entry.file_pattern) {
                (None, None) => errors.push(format!(
                    "`{at}` declares neither `file` nor `filePattern`, so nothing can be downloaded"
                )),
                (Some(_), Some(_)) => errors.push(format!(
                    "`{at}` declares BOTH `file` and `filePattern`; they are mutually exclusive and `file` wins"
                )),
                _ => {}
            }
            if let Some(sha) = &entry.sha256 {
                if sha.len() != 64 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
                    errors.push(format!("`{at}.sha256` is not a 64-character hex digest"));
                }
            }
            // A pinned sha256 cannot describe a moving target.
            if entry.sha256.is_some() && entry.tag.eq_ignore_ascii_case("latest") {
                errors.push(format!(
                    "`{at}` pins a sha256 but tracks \"latest\", so the digest breaks at the next release"
                ));
            }
        }

        // Declaring platforms without release entries is how an app ends up
        // listed but un-installable.
        for platform in &self.platforms {
            if !self.release_files.is_empty() && !self.release_files.contains_key(platform) {
                errors.push(format!(
                    "`platforms` lists {platform:?} but `releaseFiles` has no entry for it"
                ));
            }
        }

        if let Some(icon) = &self.icon {
            if icon.starts_with('/') || icon.contains("..") {
                errors.push(format!(
                    "`icon` must be a path relative to the repo root, got {icon:?}"
                ));
            } else if !icon.to_lowercase().ends_with(".png") {
                errors.push(format!(
                    "`icon` must be a PNG - Colony ships no other decoder - got {icon:?}"
                ));
            }
        }

        errors
    }
}

/// Metadata for a Colony-compatible repository (has colony.json).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ColonyRepo {
    pub name: String,
    pub description: String,
    pub language: String,
    pub html_url: String,
    pub manifest: ColonyManifest,
}

impl ColonyRepo {
    /// What to SHOW the user: the manifest's declared name, falling back to the
    /// repo slug.
    ///
    /// The spec defines `name` as "Display name in Colony", and two of the
    /// eight published manifests deliberately set one that differs from the
    /// slug ("Lilypad" for Lilypad-Vault, "SAM - Colony Edition"). Colony threw
    /// it away everywhere except the launch button, so a card titled
    /// "Lilypad-Vault" carried a button reading "Launch Lilypad", and the
    /// desktop menu got the slug too.
    ///
    /// `name` stays the IDENTITY key - install paths, caches, favorites and the
    /// active-detail-page pointer all key off it, and none of those may follow
    /// a display name the repo can change at will.
    pub fn display_name(&self) -> &str {
        let declared = self.manifest.name.trim();
        if declared.is_empty() {
            &self.name
        } else {
            declared
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct GithubRepo {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) language: Option<String>,
    pub(crate) html_url: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GithubContent {
    pub(crate) name: String,
    pub(crate) content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GithubReadme {
    pub(crate) content: Option<String>,
}
