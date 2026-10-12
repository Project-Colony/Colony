//! The Preferences page, built from colony-ui's preferences kit.
//!
//! General's heading, Appearance and Accessibility come from
//! [`PreferencesView`]; what is Colony's own (startup, language, updates,
//! keyboard navigation, storage, shortcuts, about) is drawn here with the
//! kit's section and row widgets, so the whole page reads as one.

use colony_ui::widgets::{self, PreferencesView, ABOUT_CATEGORY, STANDARD_CATEGORIES};
use colony_ui::{fonts, Typography};
use iced::widget::{button, column, container, row, text, Column};
use iced::{Element, Fill};

use crate::i18n;
use crate::message::Message;
use crate::state::App;
use crate::ui::theme::Palette;

/// The categories' label keys, in display order: the three every Colony
/// program opens with, Colony's own, then About, last.
pub(crate) const SETTINGS_CATEGORIES: [&str; 6] = [
    STANDARD_CATEGORIES[0],
    STANDARD_CATEGORIES[1],
    STANDARD_CATEGORIES[2],
    STORAGE,
    SHORTCUTS,
    ABOUT_CATEGORY,
];

const STORAGE: &str = "settings_cat_storage";
const SHORTCUTS: &str = "settings_cat_shortcuts";

impl App {
    /// The Preferences page. It replaces the content area; the sidebar stays.
    pub(crate) fn view_settings_page(&self) -> Element<'_, Message> {
        let typo = Typography::current();
        let kit = PreferencesView::new(
            typo,
            &self.standard,
            &self.settings_expanded_sections,
            Message::Preference,
            Message::SettingsToggleSection,
        );

        let category = SETTINGS_CATEGORIES
            .get(self.settings_category)
            .copied()
            .unwrap_or(SETTINGS_CATEGORIES[0]);
        let content: Element<'_, Message> = match category {
            "settings_cat_appearance" => kit.appearance().into(),
            "settings_cat_accessibility" => kit
                .accessibility_heading()
                .push(kit.vision_section())
                .push(kit.motion_section())
                .push(self.navigation_section(&typo))
                .push(kit.reading_section())
                .into(),
            STORAGE => self.view_settings_storage(&typo),
            SHORTCUTS => self.view_settings_shortcuts(&typo),
            ABOUT_CATEGORY => self.view_settings_about(&typo),
            _ => kit
                .general()
                .push(self.startup_section(&typo))
                .push(self.language_section(&typo))
                .push(self.updates_section(&typo))
                .into(),
        };

        let labels = SETTINGS_CATEGORIES.iter().map(|key| i18n::t(key));
        widgets::preferences_page(
            &typo,
            widgets::category_list(
                &typo,
                labels,
                self.settings_category,
                Message::SettingsCategory,
            ),
            content,
            Message::ToggleSettings,
        )
    }

    // ── General ──

    fn startup_section(&self, typo: &Typography) -> Element<'_, Message> {
        let default_view = widgets::setting_row(
            typo,
            &i18n::t("settings_default_view"),
            &i18n::t("settings_default_view_desc"),
            widgets::dropdown(
                typo,
                [
                    ("all".to_string(), i18n::t("settings_default_view_all")),
                    (
                        "favorites".to_string(),
                        i18n::t("settings_default_view_favorites"),
                    ),
                ],
                &self.default_view,
                Message::PickDefaultView,
            ),
        );
        self.section(
            typo,
            "startup",
            "settings_section_startup",
            Some("settings_startup_section_desc"),
            vec![
                widgets::functional_toggle(
                    typo,
                    &i18n::t("settings_restore_session"),
                    &i18n::t("settings_restore_session_desc"),
                    self.restore_session,
                    Message::ToggleRestoreSession,
                ),
                default_view,
            ],
        )
    }

    fn language_section(&self, typo: &Typography) -> Element<'_, Message> {
        // Each language is named in itself, so it can be found from either.
        let picker = widgets::setting_row(
            typo,
            &i18n::t("settings_current_language"),
            &i18n::t("settings_current_language_desc"),
            widgets::dropdown(
                typo,
                [
                    ("fr".to_string(), "Français".to_string()),
                    ("en".to_string(), "English".to_string()),
                ],
                &self.language,
                Message::PickLanguage,
            ),
        );
        self.section(
            typo,
            "lang",
            "settings_section_language",
            Some("settings_language_desc"),
            vec![picker],
        )
    }

    fn updates_section(&self, typo: &Typography) -> Element<'_, Message> {
        self.section(
            typo,
            "updates",
            "settings_section_updates",
            Some("settings_updates_desc"),
            vec![
                widgets::functional_toggle(
                    typo,
                    &i18n::t("settings_auto_check_updates"),
                    &i18n::t("settings_auto_check_updates_desc"),
                    self.auto_check_updates,
                    Message::ToggleAutoCheckUpdates,
                ),
                container(action_button(
                    typo,
                    "\u{f0ed}",
                    i18n::t("settings_check_updates"),
                    Message::CheckUpdates,
                ))
                .padding([12, 0])
                .into(),
            ],
        )
    }

    // ── Accessibility ──

    /// Colony's own Accessibility section, between Motion and Reading.
    fn navigation_section(&self, typo: &Typography) -> Element<'_, Message> {
        self.section(
            typo,
            "navigation",
            "settings_section_navigation",
            Some("settings_navigation_desc"),
            vec![widgets::functional_toggle(
                typo,
                &i18n::t("settings_keyboard_nav"),
                &i18n::t("settings_keyboard_nav_desc"),
                self.keyboard_nav,
                Message::ToggleKeyboardNav,
            )],
        )
    }

    // ── Storage ──

    fn view_settings_storage(&self, typo: &Typography) -> Element<'_, Message> {
        let scan = self.section(
            typo,
            "scan",
            "settings_section_scan",
            Some("settings_scan_desc"),
            vec![
                widgets::functional_toggle(
                    typo,
                    &i18n::t("settings_startup"),
                    &i18n::t("settings_startup_desc"),
                    self.scan_on_startup,
                    Message::ToggleScanOnStartup,
                ),
                container(action_button(
                    typo,
                    "\u{f021}",
                    i18n::t("rescan"),
                    Message::Rescan,
                ))
                .padding([12, 0])
                .into(),
            ],
        );

        let caches = self.section(
            typo,
            "caches",
            "clear_caches",
            Some("clear_caches_desc"),
            vec![action_button(
                typo,
                "\u{f1f8}",
                i18n::t("clear_caches"),
                Message::ClearStoreCaches,
            )],
        );

        let install = self.section(
            typo,
            "install",
            "settings_section_install",
            None,
            vec![
                info_row(
                    typo,
                    "\u{f1c0}",
                    i18n::t("settings_local_apps"),
                    self.applications.len().to_string(),
                ),
                divider(),
                info_row(
                    typo,
                    "\u{f0c2}",
                    i18n::t("settings_colony_repos"),
                    self.colony_repos().len().to_string(),
                ),
                divider(),
                info_row(
                    typo,
                    "\u{f07c}",
                    i18n::t("settings_favorites"),
                    self.favorites.len().to_string(),
                ),
            ],
        );

        widgets::category_heading(typo, &i18n::t(STORAGE), &i18n::t("settings_storage_desc"))
            .push(scan)
            .push(caches)
            .push(install)
            .into()
    }

    // ── Shortcuts ──

    fn view_settings_shortcuts(&self, typo: &Typography) -> Element<'_, Message> {
        let mut col = widgets::category_heading(
            typo,
            &i18n::t("shortcuts_title"),
            &i18n::t("shortcuts_desc"),
        );

        for key in [
            "shortcut_esc",
            "shortcut_tab",
            "shortcut_arrows",
            "shortcut_enter",
            "shortcut_pageupdown",
        ] {
            col = col
                .push(
                    container(
                        text(i18n::t(key))
                            .size(typo.sz(13))
                            .font(typo.regular)
                            .color(Palette::TEXT_SECONDARY()),
                    )
                    .padding([8, 0]),
                )
                .push(divider());
        }

        let mut hints = Column::new().spacing(8).padding(iced::Padding {
            top: 14.0,
            ..iced::Padding::ZERO
        });
        for key in [
            "hint_settings",
            "hint_search",
            "hint_favorites",
            "hint_keyboard",
        ] {
            hints = hints.push(
                row![
                    text("\u{f05a}")
                        .size(typo.sz(12))
                        .font(fonts::glyph_font())
                        .color(Palette::TEXT_MUTED()),
                    text(i18n::t(key))
                        .size(typo.sz(12))
                        .font(typo.regular)
                        .color(Palette::TEXT_MUTED()),
                ]
                .spacing(10),
            );
        }

        col.push(hints).into()
    }

    // ── About ──

    fn view_settings_about(&self, typo: &Typography) -> Element<'_, Message> {
        let version = row![
            text("\u{f015}")
                .size(typo.sz(15))
                .font(fonts::glyph_font())
                .color(Palette::ACCENT()),
            text(format!("Colony v{}", env!("CARGO_PKG_VERSION")))
                .size(typo.sz(15))
                .font(typo.regular)
                .color(Palette::TEXT_PRIMARY()),
        ]
        .spacing(10);

        let source = row![
            text("\u{f09b}")
                .size(typo.sz(15))
                .font(fonts::glyph_font())
                .color(Palette::TEXT_DIM()),
            text("Project-Colony/Colony")
                .size(typo.sz(14))
                .font(typo.regular)
                .color(Palette::TEXT_MUTED()),
        ]
        .spacing(10);

        widgets::category_heading(
            typo,
            &i18n::t("settings_about_title"),
            &i18n::t("settings_about_desc"),
        )
        .push(
            column![version, source, self.launcher_update_control(typo)]
                .spacing(12)
                .padding([0, 4]),
        )
        .into()
    }

    /// The launcher's own update, in the state it is in: up to date (check),
    /// available (download), staged (restart), or managed by the system's
    /// package manager (how to update instead).
    fn launcher_update_control(&self, typo: &Typography) -> Element<'_, Message> {
        let Some((tag, _)) = &self.launcher_update_available else {
            let is_checking = self.is_checking_launcher_update;
            let mut label = row![].spacing(10).align_y(iced::Alignment::Center);
            if is_checking {
                label = label.push(text("\u{f110}").size(typo.sz(13)).font(fonts::glyph_font()));
            }
            let label = label.push(
                text(i18n::t("check_launcher_updates"))
                    .size(typo.sz(13))
                    .font(typo.regular),
            );
            let btn = button(label).padding([6, 14]).style(|_theme, status| {
                let (bg, text_color) = crate::ui::theme::button_colors(
                    status,
                    iced::Color::TRANSPARENT,
                    Palette::BG_CARD_HOVER(),
                    Palette::TEXT_DIM(),
                );
                button::Style {
                    background: Some(bg.into()),
                    text_color,
                    border: iced::Border::default().rounded(6),
                    ..Default::default()
                }
            });
            return if is_checking {
                btn.into()
            } else {
                btn.on_press(Message::CheckLauncherUpdate { manual: true })
                    .into()
            };
        };

        if self.launcher_system_managed {
            // Package-manager install: the download/apply flow is guaranteed
            // to fail (the exe dir is not user-writable), so show the
            // actionable path instead of a doomed button.
            return text(i18n::t_fmt(
                "launcher_update_system_managed",
                &[("version", tag.as_str())],
            ))
            .size(typo.sz(13))
            .font(typo.regular)
            .color(Palette::WARNING())
            .into();
        }

        let style = |_theme: &iced::Theme, status: button::Status| {
            let bg = match status {
                button::Status::Hovered => Palette::BG_CARD_HOVER(),
                _ => Palette::BG_SELECTED(),
            };
            button::Style {
                background: Some(bg.into()),
                text_color: Palette::ACCENT(),
                border: iced::Border::default().rounded(6),
                ..Default::default()
            }
        };

        if let Some(path) = &self.launcher_update_staged {
            return button(crate::ui::glyph_label(
                "\u{f021}",
                i18n::t("launcher_restart_to_update"),
                13,
                typo.regular,
            ))
            .on_press(Message::ApplyLauncherUpdate(path.clone()))
            .padding([6, 14])
            .style(style)
            .into();
        }

        let btn = button(
            text(i18n::t_fmt(
                "launcher_download_update",
                &[("version", tag.as_str())],
            ))
            .size(typo.sz(13))
            .font(typo.regular),
        )
        .padding([6, 14])
        .style(style);
        if self.is_downloading {
            btn.into()
        } else {
            btn.on_press(Message::DownloadLauncherUpdate).into()
        }
    }

    /// One of Colony's own collapsible sections, drawn the way the kit draws
    /// its: the description first, then the controls.
    fn section<'a>(
        &self,
        typo: &Typography,
        key: &'static str,
        title_key: &str,
        description_key: Option<&str>,
        controls: Vec<Element<'a, Message>>,
    ) -> Element<'a, Message> {
        let mut body = Column::new().width(Fill);
        if let Some(description) = description_key {
            body = body.push(widgets::section_description(typo, &i18n::t(description)));
        }
        widgets::collapsible_section(
            typo,
            &i18n::t(title_key),
            self.settings_expanded_sections.contains(key),
            Message::SettingsToggleSection(key),
            body.push(Column::with_children(controls).width(Fill))
                .into(),
        )
    }
}

/// An icon, a label and a value, for the Installation counts.
fn info_row<'a>(
    typo: &Typography,
    icon: &'a str,
    label: String,
    value: String,
) -> Element<'a, Message> {
    row![
        text(icon)
            .size(typo.sz(13))
            .font(fonts::glyph_font())
            .color(Palette::ACCENT()),
        text(label)
            .size(typo.sz(13))
            .font(typo.regular)
            .color(Palette::TEXT_MUTED()),
        container(text("")).width(Fill),
        text(value)
            .size(typo.sz(13))
            .font(typo.regular)
            .color(Palette::TEXT_PRIMARY()),
    ]
    .spacing(10)
    .padding([6, 0])
    .align_y(iced::Alignment::Center)
    .into()
}

/// A subtle horizontal divider.
fn divider() -> Element<'static, Message> {
    container(text(""))
        .width(Fill)
        .height(1)
        .style(|_theme| container::Style {
            background: Some(Palette::DIVIDER().into()),
            ..Default::default()
        })
        .into()
}

/// A button that does something now rather than set a preference: an icon in
/// the accent, then its label.
fn action_button<'a>(
    typo: &Typography,
    icon: &'a str,
    label: String,
    message: Message,
) -> Element<'a, Message> {
    button(
        row![
            text(icon)
                .size(typo.sz(13))
                .font(fonts::glyph_font())
                .color(Palette::ACCENT()),
            text(label).size(typo.sz(13)).font(typo.regular),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center),
    )
    .on_press(message)
    .padding([10, 16])
    .style(|_theme, status| {
        let bg = match status {
            button::Status::Hovered => Palette::BTN_HOVER(),
            _ => Palette::BTN_DEFAULT(),
        };
        button::Style {
            background: Some(bg.into()),
            text_color: Palette::TEXT_PRIMARY(),
            border: iced::Border::default().rounded(8),
            ..Default::default()
        }
    })
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// design/settings-page.md: General, Appearance and Accessibility first and
    /// in that order, the program's own next, About last.
    #[test]
    fn the_categories_are_in_the_shared_order() {
        assert_eq!(SETTINGS_CATEGORIES[..3], STANDARD_CATEGORIES);
        assert_eq!(
            SETTINGS_CATEGORIES,
            [
                "settings_cat_general",
                "settings_cat_appearance",
                "settings_cat_accessibility",
                "settings_cat_storage",
                "settings_cat_shortcuts",
                "settings_cat_about",
            ]
        );
        assert_eq!(SETTINGS_CATEGORIES.last(), Some(&ABOUT_CATEGORY));
    }
}
