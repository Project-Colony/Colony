use colony_ui::widgets;
use colony_ui::{fonts, sz, ui_font, Typography};
use iced::widget::{button, column, container, row, scrollable, text, Column};

use iced::{Element, Fill, Length};

use crate::message::Message;
use crate::sections::Section;
use crate::state::{App, GitHubState};
use crate::ui::glyph_label;
use crate::ui::theme::Palette;

impl App {
    pub(crate) fn view_sidebar(&self) -> Element<'_, Message> {
        let typo = Typography::current();

        // The way into Preferences, and back out: the page's Close button
        // sends the same message.
        let title = widgets::identity_button(
            &typo,
            "Colony",
            30,
            self.show_settings,
            Message::ToggleSettings,
        );

        let category_header = widgets::nav_label(&typo, &crate::i18n::t("categories"));
        let hint_text = widgets::nav_hint(&typo, &crate::i18n::t("hint_keyboard"));

        let category_buttons: Vec<Element<'_, Message>> = self
            .sections
            .iter()
            .enumerate()
            .map(|(index, section)| self.view_section_button(index, section))
            .collect();

        let category_list = Column::with_children(category_buttons).spacing(4);
        let category_scroll = container(scrollable(category_list).height(Length::Fill))
            .id(crate::ui::tutorial::ID_SIDEBAR_CATS)
            .height(Length::Fill);

        // GitHub button — Nerd Font icon  (U+F09B)
        let github_connected = matches!(self.github_state, GitHubState::Connected { .. });
        let github_icon_color = if github_connected {
            Palette::ACCENT()
        } else if self.show_github_menu {
            Palette::TEXT_PRIMARY()
        } else {
            Palette::TEXT_DIM()
        };

        let mut github_btn_content = row![
            text("\u{f09b}")
                .size(sz(18))
                .font(fonts::glyph_font())
                .color(github_icon_color),
            text("GitHub")
                .size(sz(13))
                .font(ui_font())
                .color(github_icon_color),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center);
        if github_connected {
            github_btn_content = github_btn_content.push(
                text("\u{f00c}")
                    .size(sz(13))
                    .font(fonts::glyph_font())
                    .color(github_icon_color),
            );
        }

        let show_menu = self.show_github_menu;
        let github_btn = button(github_btn_content)
            .on_press(Message::ToggleGitHubMenu)
            .padding([10, 14])
            .width(Fill)
            .style(move |_theme, _status| button::Style {
                background: Some(
                    if show_menu {
                        Palette::BG_SELECTED()
                    } else {
                        iced::Color::TRANSPARENT
                    }
                    .into(),
                ),
                text_color: Palette::TEXT_PRIMARY(),
                border: iced::Border::default().rounded(6),
                ..Default::default()
            });

        let rescan_label: Element<'_, Message> = if self.is_scanning {
            glyph_label("\u{f110}", crate::i18n::t("scanning"), 13, ui_font()).into()
        } else {
            text(crate::i18n::t("rescan"))
                .size(sz(13))
                .font(ui_font())
                .into()
        };
        let rescan_btn_base = button(rescan_label).padding([8, 16]).width(Fill);
        let rescan_btn = if self.is_scanning {
            rescan_btn_base
        } else {
            rescan_btn_base.on_press(Message::Rescan)
        };

        // Launcher update badge
        let update_badge: Element<'_, Message> = if let Some(ref update_info) =
            self.launcher_update_available
        {
            // A package-managed install cannot apply the in-app update: the exe
            // directory is not writable. Settings > About already replaces the
            // button with guidance here; the sidebar used to keep offering a
            // press that only ever produced a warning toast, with no dismiss,
            // for the whole session.
            if self.launcher_system_managed {
                return_system_managed_badge(&update_info.0)
            } else {
                let (glyph, label, msg) = if let Some(ref path) = self.launcher_update_staged {
                    (
                        "\u{f021}",
                        crate::i18n::t("launcher_restart_to_update"),
                        Message::ApplyLauncherUpdate(path.clone()),
                    )
                } else {
                    let tag = &update_info.0;
                    (
                        "\u{f0aa}",
                        crate::i18n::t_fmt("launcher_update_available_short", &[("version", tag)]),
                        Message::DownloadLauncherUpdate,
                    )
                };

                let is_downloading = self.is_downloading;
                // The texts take button_colors' accent, dimmed while disabled.
                button(glyph_label(glyph, label, 11, ui_font()))
                    .on_press_maybe(if is_downloading { None } else { Some(msg) })
                    .padding([6, 12])
                    .width(Fill)
                    .style(|_theme, status| {
                        let (bg, text_color) = crate::ui::theme::button_colors(
                            status,
                            Palette::BG_SELECTED(),
                            Palette::BG_CARD_HOVER(),
                            Palette::ACCENT(),
                        );
                        button::Style {
                            background: Some(bg.into()),
                            text_color,
                            border: iced::Border::default().rounded(6),
                            ..Default::default()
                        }
                    })
                    .into()
            }
        } else {
            container(text("")).height(0).into()
        };

        let github_group =
            container(column![github_btn, container(text("")).height(4), rescan_btn,].spacing(0))
                .id(crate::ui::tutorial::ID_GITHUB)
                .width(Fill);

        let sidebar_content = column![
            title,
            update_badge,
            container(text("")).height(24),
            category_header,
            category_scroll,
            hint_text,
            container(text("")).height(4),
            github_group,
        ]
        .spacing(10)
        .padding(16)
        .width(sidebar_width());

        container(sidebar_content)
            .style(|_theme| container::Style {
                background: Some(Palette::BG_SIDEBAR().into()),
                ..Default::default()
            })
            .height(Fill)
            .into()
    }

    /// One section of the sidebar: its glyph and its name, selected by
    /// background like every list in a Colony program.
    pub(crate) fn view_section_button(
        &self,
        index: usize,
        section: &Section,
    ) -> Element<'_, Message> {
        let is_selected = self.selected_section == index && !self.show_github_menu;

        // No colour on the texts: they take the button's, which
        // selection_style sets to contrast_on(accent) when selected.
        let content = row![
            text(section.icon.clone())
                .size(sz(15))
                .font(fonts::glyph_font()),
            text(crate::i18n::section_display_name(&section.name))
                .size(sz(14))
                .font(ui_font()),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center);

        button(content)
            .on_press(Message::SectionSelected(index))
            .padding([10, 14])
            .width(Fill)
            .style(widgets::selection_style(is_selected))
            .into()
    }
}

/// The sidebar's width: 200 at a text scale of 1.0, growing with the text so
/// "Développement" still fits on one line at 1.68x.
pub(crate) fn sidebar_width() -> f32 {
    200.0 * colony_ui::font_scale()
}

/// The sidebar's update badge for an install Colony cannot update itself:
/// informational text, not a button that is guaranteed to fail.
fn return_system_managed_badge<'a>(version: &str) -> Element<'a, Message> {
    container(
        text(crate::i18n::t_fmt(
            "launcher_update_system_managed",
            &[("version", version)],
        ))
        .size(sz(11))
        .font(ui_font())
        .color(Palette::TEXT_DIMMER()),
    )
    .padding([6, 12])
    .width(Fill)
    .into()
}
