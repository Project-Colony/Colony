//! Colony's theme — now shared with the rest of the ecosystem.
//!
//! The 38-field palette, all 57 theme palettes, the `(family, variant)`
//! resolver, the picker catalog, the accent overrides and the high-contrast
//! derivation live in `colony-ui`, generated from the design tokens in
//! Project-Colony-Resources.
//!
//! This module re-exports them so every existing `crate::ui::theme::…` call
//! site keeps working unchanged.
//!
//! **Adding a theme family no longer touches this repository.** Add the TOML
//! upstream, regenerate, and bump the `colony-ui` tag in `Cargo.toml`.

pub use colony_ui::theme::*;
