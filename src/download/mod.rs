//! Asset download, archive extraction, and launcher self-update.
//!
//! Downloads stream to a temporary file with length/integrity checks and are
//! atomically promoted into place; launcher self-updates are verified against a
//! signed detached signature (see [`crate::signing`]) before being applied.

mod files;
mod http;
mod install;
mod launcher;
mod verify;

pub(crate) use files::ensure_safe_component;
pub(crate) use http::{build_url, web_url};
pub use install::{download_release_asset, AssetInstall};
pub use launcher::{apply_launcher_update, download_launcher_asset, launcher_is_system_managed};
