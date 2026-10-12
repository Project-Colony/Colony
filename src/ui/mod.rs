mod app_grid;
mod detail;
mod github_panel;
pub mod markdown_blocks;
mod settings;
mod sidebar;
pub mod theme;
pub(crate) mod tutorial;

pub(crate) use settings::SETTINGS_CATEGORIES;
pub(crate) use sidebar::sidebar_width;
pub(crate) use tutorial::{fetch_bounds_task, TutorialBounds, TUTORIAL_LAST_STEP};

/// A Nerd Font glyph and its label, as a button's content. Two texts because
/// they are two fonts: the glyph stays in the Nerd Font whatever the dyslexia
/// setting (OpenDyslexic has none of them), the label follows it. Both take
/// the button's text colour.
pub(crate) fn glyph_label<'a, M: 'a>(
    glyph: &'a str,
    label: String,
    size: u16,
    font: iced::Font,
) -> iced::widget::Row<'a, M> {
    iced::widget::row![
        iced::widget::text(glyph)
            .size(colony_ui::sz(size))
            .font(colony_ui::fonts::glyph_font()),
        iced::widget::text(label)
            .size(colony_ui::sz(size))
            .font(font),
    ]
    .spacing(8)
    .align_y(iced::Alignment::Center)
}
