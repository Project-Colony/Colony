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
