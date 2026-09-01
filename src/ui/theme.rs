//! Colony's theme — now shared with the rest of the ecosystem.
//!
//! The 38-field palette, all 57 theme palettes, the `(family, variant)`
//! resolver, the picker catalog, the accent overrides and the high-contrast
//! derivation live in `colony-ui`, generated from the design tokens in
//! Project-Colony-Resources.
//!
//! Re-exported here so every existing `crate::ui::theme::…` call site keeps
//! working unchanged. What stays below is Colony's own: button styling built on
//! top of the shared palette.
//!
//! **Adding a theme family no longer touches this repository.** Add the TOML
//! upstream, regenerate, and bump the `colony-ui` version in `Cargo.toml`.

pub use colony_ui::theme::*;

pub fn action_button_style(
    status: iced::widget::button::Status,
    normal: iced::Color,
    hover: iced::Color,
    pressed: iced::Color,
) -> iced::widget::button::Style {
    use iced::widget::button::Status;
    let (background, text_color) = match status {
        Status::Hovered => (hover, Palette::TEXT_PRIMARY()),
        Status::Pressed => (pressed, Palette::TEXT_PRIMARY()),
        Status::Disabled => (
            // Halfway to the page background reads as "not available now"
            // without inventing a new palette entry per theme.
            mix(normal, Palette::BG_PRIMARY(), 0.6),
            Palette::TEXT_DIMMER(),
        ),
        Status::Active => (normal, Palette::TEXT_PRIMARY()),
    };
    iced::widget::button::Style {
        background: Some(background.into()),
        text_color,
        border: iced::Border::default().rounded(8),
        ..Default::default()
    }
}

/// Background and text colour for a two-state (normal/hover) button, with the
/// disabled case handled. Companion to [`action_button_style`] for the buttons
/// that carry their own text colour rather than TEXT_PRIMARY.
pub fn button_colors(
    status: iced::widget::button::Status,
    normal: iced::Color,
    hover: iced::Color,
    text: iced::Color,
) -> (iced::Color, iced::Color) {
    use iced::widget::button::Status;
    match status {
        Status::Hovered | Status::Pressed => (hover, text),
        Status::Disabled => (
            mix(normal, Palette::BG_PRIMARY(), 0.6),
            mix(text, Palette::BG_PRIMARY(), 0.55),
        ),
        Status::Active => (normal, text),
    }
}

/// Linear blend of two colours; `t` is how much of `b` to take.
fn mix(a: iced::Color, b: iced::Color, t: f32) -> iced::Color {
    let t = t.clamp(0.0, 1.0);
    iced::Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

#[cfg(test)]
mod doc_parity_tests {
    /// Five documents state the family and palette counts, and nothing kept
    /// them true: they said 24 families when there were 25, and the Stellar
    /// Blade family (five variants) was missing from the README's table
    /// altogether. Adding a palette should fail here, not go unnoticed.
    ///
    /// This used to parse this file's own source for `=> ThemePalette::` arms.
    /// The palettes now come from colony-ui, so it counts the catalog itself —
    /// which is what the picker renders, and therefore what the docs describe.
    #[test]
    fn the_documented_theme_counts_match_the_code() {
        let families = colony_ui::THEME_FAMILIES.len();
        let palettes: usize = colony_ui::THEME_FAMILIES
            .iter()
            .map(|f| f.variants.len())
            .sum();

        assert_eq!(
            families, 25,
            "the docs say 25 theme families; colony-ui ships {families}. Update README.md \
             (including its table of family names), docs/faq.md, docs/tutorial.md, \
             docs/architecture.md and CONTRIBUTING.md."
        );
        assert_eq!(
            palettes, 57,
            "the docs say 57 palettes; colony-ui ships {palettes}. Update the same five documents."
        );
    }
}
