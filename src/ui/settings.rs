use iced::font::Weight;
use iced::widget::overlay::menu as overlay_menu;
use iced::widget::{button, column, container, pick_list, row, scrollable, text, Column};
use iced::{Element, Fill, Length};

use crate::i18n;
use crate::message::Message;
use crate::state::App;
use crate::ui::theme::Palette;

/// Settings category names (keys for i18n).
const SETTINGS_CATEGORIES: &[&str] = &[
    "settings_cat_general",
    "settings_cat_appearance",
    "settings_cat_accessibility",
    "settings_cat_storage",
    "settings_cat_about",
    "settings_cat_shortcuts",
];

impl App {
    /// Full-page settings view (replaces content area).
    pub(crate) fn view_settings_page(&self) -> Element<'_, Message> {
        // ── Settings sidebar (left) ──
        let header = row![
            text(i18n::t("settings_title"))
                .size(self.sz(22))
                .font(self.app_font_with_weight(Weight::Bold))
                .color(Palette::TEXT_PRIMARY()),
            container(text("")).width(Fill),
            button(
                text(i18n::t("settings_close"))
                    .size(self.sz(13))
                    .font(self.app_font())
            )
            .on_press(Message::ToggleSettings)
            .padding([6, 14])
            .style(|_theme, status| {
                let bg = match status {
                    button::Status::Hovered => Palette::BG_CARD_HOVER(),
                    _ => iced::Color::TRANSPARENT,
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: Palette::TEXT_MUTED(),
                    border: iced::Border::default().rounded(6),
                    ..Default::default()
                }
            }),
        ]
        .align_y(iced::Alignment::Center);

        let mut cat_buttons: Vec<Element<'_, Message>> = Vec::new();
        for (i, key) in SETTINGS_CATEGORIES.iter().enumerate() {
            let is_selected = self.settings_category == i;
            let idx = i;
            cat_buttons.push(
                button(text(i18n::t(key)).size(self.sz(13)).font(self.app_font()))
                    .on_press(Message::SettingsCategory(idx))
                    .padding([8, 14])
                    .width(Fill)
                    .style(move |_theme, status| {
                        let bg = match status {
                            _ if is_selected => Palette::ACCENT(),
                            button::Status::Hovered => Palette::BG_CARD_HOVER(),
                            _ => iced::Color::TRANSPARENT,
                        };
                        button::Style {
                            background: Some(bg.into()),
                            text_color: if is_selected {
                                Palette::TEXT_PRIMARY()
                            } else {
                                Palette::TEXT_MUTED()
                            },
                            border: iced::Border::default().rounded(8),
                            ..Default::default()
                        }
                    })
                    .into(),
            );
        }

        let settings_nav = container(Column::with_children(cat_buttons).spacing(2))
            .width(Length::Fixed(160.0))
            .padding(iced::Padding {
                top: 0.0,
                right: 16.0,
                bottom: 0.0,
                left: 0.0,
            });

        // ── Settings content (right) ──
        let settings_content = match self.settings_category {
            0 => self.view_settings_general(),
            1 => self.view_settings_appearance(),
            2 => self.view_settings_accessibility(),
            3 => self.view_settings_storage(),
            4 => self.view_settings_about(),
            5 => self.view_settings_shortcuts(),
            _ => self.view_settings_general(),
        };

        let content_area = container(
            scrollable(container(settings_content).padding(iced::Padding {
                top: 0.0,
                right: 24.0,
                bottom: 24.0,
                left: 0.0,
            }))
            .id(iced::widget::Id::new("settings-scroll"))
            .height(Fill),
        )
        .width(Fill)
        .height(Fill);

        let body = row![settings_nav, content_area].spacing(0);

        let page = column![header, container(text("")).height(16), body,]
            .padding(24)
            .width(Fill)
            .height(Fill);

        container(page)
            .style(|_theme| container::Style {
                background: Some(Palette::BG_PRIMARY().into()),
                ..Default::default()
            })
            .width(Fill)
            .height(Fill)
            .into()
    }

    // ── General settings ──
    fn view_settings_general(&self) -> Element<'_, Message> {
        let cat_title = text(i18n::t("settings_general_title"))
            .size(self.sz(18))
            .font(self.app_font_with_weight(Weight::Bold))
            .color(Palette::TEXT_PRIMARY());

        let cat_desc = text(i18n::t("settings_general_desc"))
            .size(self.sz(12))
            .font(self.app_font())
            .color(Palette::TEXT_MUTED());

        let mut sections = column![
            cat_title,
            container(text("")).height(4),
            cat_desc,
            container(text("")).height(20),
        ]
        .spacing(0);

        // Section: Démarrage
        sections = sections.push(
            self.view_collapsible_section(
                "startup",
                &i18n::t("settings_section_startup"),
                column![
                    text(i18n::t("settings_startup_section_desc"))
                        .size(self.sz(12))
                        .font(self.app_font())
                        .color(Palette::TEXT_MUTED()),
                    container(text("")).height(12),
                    self.view_functional_toggle(
                        &i18n::t("settings_restore_session"),
                        &i18n::t("settings_restore_session_desc"),
                        self.restore_session,
                        Message::ToggleRestoreSession,
                    ),
                    container(text("")).height(4),
                    self.view_pick_list(
                        &i18n::t("settings_default_view"),
                        &i18n::t("settings_default_view_desc"),
                        vec![
                            ("all".into(), i18n::t("settings_default_view_all")),
                            (
                                "favorites".into(),
                                i18n::t("settings_default_view_favorites")
                            ),
                        ],
                        &self.default_view,
                        Message::PickDefaultView,
                    ),
                ]
                .spacing(0)
                .into(),
            ),
        );
        sections = sections.push(container(text("")).height(6));

        // Section: Langue
        sections = sections.push(
            self.view_collapsible_section(
                "lang",
                &i18n::t("settings_section_language"),
                column![
                    text(i18n::t("settings_language_desc"))
                        .size(self.sz(12))
                        .font(self.app_font())
                        .color(Palette::TEXT_MUTED()),
                    container(text("")).height(12),
                    self.view_pick_list(
                        &i18n::t("settings_current_language"),
                        &i18n::t("settings_current_language_desc"),
                        vec![
                            ("fr".into(), "Français".into()),
                            ("en".into(), "English".into()),
                        ],
                        &self.language,
                        Message::PickLanguage,
                    ),
                ]
                .spacing(0)
                .into(),
            ),
        );
        sections = sections.push(container(text("")).height(6));

        // Section: Mises à jour
        sections = sections.push(
            self.view_collapsible_section(
                "updates",
                &i18n::t("settings_section_updates"),
                column![
                    text(i18n::t("settings_updates_desc"))
                        .size(self.sz(12))
                        .font(self.app_font())
                        .color(Palette::TEXT_MUTED()),
                    container(text("")).height(12),
                    self.view_functional_toggle(
                        &i18n::t("settings_auto_check_updates"),
                        &i18n::t("settings_auto_check_updates_desc"),
                        self.auto_check_updates,
                        Message::ToggleAutoCheckUpdates,
                    ),
                    container(text("")).height(12),
                    self.action_button(
                        "\u{f0ed}",
                        i18n::t("settings_check_updates"),
                        Message::CheckUpdates,
                    ),
                ]
                .spacing(0)
                .into(),
            ),
        );

        sections.into()
    }

    // ── Appearance settings ──
    fn view_settings_appearance(&self) -> Element<'_, Message> {
        let cat_title = text(i18n::t("settings_appearance_title"))
            .size(self.sz(18))
            .font(self.app_font_with_weight(Weight::Bold))
            .color(Palette::TEXT_PRIMARY());

        let cat_desc = text(i18n::t("settings_appearance_desc"))
            .size(self.sz(12))
            .font(self.app_font())
            .color(Palette::TEXT_MUTED());

        let mut sections = column![
            cat_title,
            container(text("")).height(4),
            cat_desc,
            container(text("")).height(20),
        ]
        .spacing(0);

        // Thème
        sections = sections.push(self.view_collapsible_section(
            "theme",
            &i18n::t("settings_section_theme"),
            self.view_theme_section(),
        ));
        sections = sections.push(container(text("")).height(6));

        // Couleurs & accents
        sections = sections.push(self.view_collapsible_section(
            "colors",
            &i18n::t("settings_section_colors"),
            self.view_colors_section(),
        ));
        sections = sections.push(container(text("")).height(6));

        // Typographie
        sections = sections.push(
            self.view_collapsible_section(
                "typography",
                &i18n::t("settings_section_typography"),
                column![
                    text(i18n::t("settings_typography_desc"))
                        .size(self.sz(12))
                        .font(self.app_font())
                        .color(Palette::TEXT_MUTED()),
                    container(text("")).height(12),
                    self.view_pick_list(
                        &i18n::t("settings_font_size"),
                        &i18n::t("settings_font_size_desc"),
                        vec![
                            ("small".into(), i18n::t("settings_font_size_small")),
                            ("default".into(), i18n::t("settings_font_size_default")),
                            ("large".into(), i18n::t("settings_font_size_large")),
                        ],
                        &self.font_size,
                        Message::PickFontSize,
                    ),
                ]
                .spacing(0)
                .into(),
            ),
        );
        sections = sections.push(container(text("")).height(6));

        // Arrière-plans & effets
        sections = sections.push(
            self.view_collapsible_section(
                "effects",
                &i18n::t("settings_section_effects"),
                column![
                    text(i18n::t("settings_effects_desc"))
                        .size(self.sz(12))
                        .font(self.app_font())
                        .color(Palette::TEXT_MUTED()),
                    container(text("")).height(12),
                    self.view_functional_toggle(
                        &i18n::t("settings_animations"),
                        &i18n::t("settings_animations_desc"),
                        self.animations,
                        Message::ToggleAnimations,
                    ),
                ]
                .spacing(0)
                .into(),
            ),
        );
        sections = sections.push(container(text("")).height(6));

        // Aperçu
        sections = sections.push(
            self.view_collapsible_section(
                "preview",
                &i18n::t("settings_section_preview"),
                column![container(
                    column![
                        text(i18n::t("settings_preview_card"))
                            .size(self.sz(14))
                            .font(self.app_font_with_weight(Weight::Bold))
                            .color(Palette::TEXT_PRIMARY()),
                        container(text("")).height(6),
                        text(i18n::t("settings_preview_summary"))
                            .size(self.sz(12))
                            .font(self.app_font())
                            .color(Palette::TEXT_MUTED()),
                    ]
                    .spacing(0)
                )
                .padding(16)
                .width(Fill)
                .style(|_theme| container::Style {
                    background: Some(Palette::BG_CARD().into()),
                    border: iced::Border {
                        color: Palette::BORDER_SUBTLE(),
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                }),]
                .spacing(0)
                .into(),
            ),
        );

        sections.into()
    }

    // ── Accessibility settings ──
    fn view_settings_accessibility(&self) -> Element<'_, Message> {
        let cat_title = text(i18n::t("settings_accessibility_title"))
            .size(self.sz(18))
            .font(self.app_font_with_weight(Weight::Bold))
            .color(Palette::TEXT_PRIMARY());

        let cat_desc = text(i18n::t("settings_accessibility_desc"))
            .size(self.sz(12))
            .font(self.app_font())
            .color(Palette::TEXT_MUTED());

        let mut sections = column![
            cat_title,
            container(text("")).height(4),
            cat_desc,
            container(text("")).height(20),
        ]
        .spacing(0);

        // Vision
        sections = sections.push(
            self.view_collapsible_section(
                "vision",
                &i18n::t("settings_section_vision"),
                column![
                    text(i18n::t("settings_vision_desc"))
                        .size(self.sz(12))
                        .font(self.app_font())
                        .color(Palette::TEXT_MUTED()),
                    container(text("")).height(12),
                    self.view_functional_toggle(
                        &i18n::t("settings_high_contrast"),
                        &i18n::t("settings_high_contrast_desc"),
                        self.high_contrast,
                        Message::ToggleHighContrast,
                    ),
                    container(text("")).height(4),
                    self.view_pick_list(
                        &i18n::t("settings_text_size_a11y"),
                        &i18n::t("settings_text_size_a11y_desc"),
                        vec![
                            ("small".into(), i18n::t("settings_font_size_small")),
                            ("default".into(), i18n::t("settings_font_size_default")),
                            ("large".into(), i18n::t("settings_font_size_large")),
                            ("xlarge".into(), i18n::t("settings_font_size_xlarge")),
                        ],
                        &self.text_size_a11y,
                        Message::PickTextSizeA11y,
                    ),
                ]
                .spacing(0)
                .into(),
            ),
        );
        sections = sections.push(container(text("")).height(6));

        // Mouvement
        sections = sections.push(
            self.view_collapsible_section(
                "motion",
                &i18n::t("settings_section_motion"),
                column![
                    text(i18n::t("settings_motion_desc"))
                        .size(self.sz(12))
                        .font(self.app_font())
                        .color(Palette::TEXT_MUTED()),
                    container(text("")).height(12),
                    self.view_functional_toggle(
                        &i18n::t("settings_reduce_motion"),
                        &i18n::t("settings_reduce_motion_desc"),
                        self.reduce_motion,
                        Message::ToggleReduceMotion,
                    ),
                ]
                .spacing(0)
                .into(),
            ),
        );
        sections = sections.push(container(text("")).height(6));

        // Navigation & interaction
        sections = sections.push(
            self.view_collapsible_section(
                "navigation",
                &i18n::t("settings_section_navigation"),
                column![
                    text(i18n::t("settings_navigation_desc"))
                        .size(self.sz(12))
                        .font(self.app_font())
                        .color(Palette::TEXT_MUTED()),
                    container(text("")).height(12),
                    self.view_functional_toggle(
                        &i18n::t("settings_keyboard_nav"),
                        &i18n::t("settings_keyboard_nav_desc"),
                        self.keyboard_nav,
                        Message::ToggleKeyboardNav,
                    ),
                ]
                .spacing(0)
                .into(),
            ),
        );
        sections = sections.push(container(text("")).height(6));

        // Lecture
        sections = sections.push(
            self.view_collapsible_section(
                "reading",
                &i18n::t("settings_section_reading"),
                column![
                    text(i18n::t("settings_reading_desc"))
                        .size(self.sz(12))
                        .font(self.app_font())
                        .color(Palette::TEXT_MUTED()),
                    container(text("")).height(12),
                    self.view_functional_toggle(
                        &i18n::t("settings_dyslexia_font"),
                        &i18n::t("settings_dyslexia_font_desc"),
                        self.dyslexia_font,
                        Message::ToggleDyslexiaFont,
                    ),
                ]
                .spacing(0)
                .into(),
            ),
        );

        sections.into()
    }

    // ── Storage settings ──
    fn view_settings_storage(&self) -> Element<'_, Message> {
        let cat_title = text(i18n::t("settings_storage_title"))
            .size(self.sz(18))
            .font(self.app_font_with_weight(Weight::Bold))
            .color(Palette::TEXT_PRIMARY());

        let cat_desc = text(i18n::t("settings_storage_desc"))
            .size(self.sz(12))
            .font(self.app_font())
            .color(Palette::TEXT_MUTED());

        let mut sections = column![
            cat_title,
            container(text("")).height(4),
            cat_desc,
            container(text("")).height(20),
        ]
        .spacing(0);

        // Section: Scan
        sections = sections.push(
            self.view_collapsible_section(
                "scan",
                &i18n::t("settings_section_scan"),
                column![
                    text(i18n::t("settings_scan_desc"))
                        .size(self.sz(12))
                        .font(self.app_font())
                        .color(Palette::TEXT_MUTED()),
                    container(text("")).height(12),
                    self.view_functional_toggle(
                        &i18n::t("settings_startup"),
                        &i18n::t("settings_startup_desc"),
                        self.scan_on_startup,
                        Message::ToggleScanOnStartup,
                    ),
                    container(text("")).height(12),
                    self.action_button("\u{f021}", i18n::t("rescan"), Message::Rescan,),
                ]
                .spacing(0)
                .into(),
            ),
        );

        sections = sections.push(container(text("")).height(6));

        // Section: Caches
        sections = sections.push(
            self.view_collapsible_section(
                "caches",
                &i18n::t("clear_caches"),
                column![
                    text(i18n::t("clear_caches_desc"))
                        .size(self.sz(12))
                        .font(self.app_font())
                        .color(Palette::TEXT_MUTED()),
                    container(text("")).height(12),
                    self.action_button(
                        "\u{f1f8}",
                        i18n::t("clear_caches"),
                        Message::ClearStoreCaches,
                    ),
                ]
                .spacing(0)
                .into(),
            ),
        );

        sections = sections.push(container(text("")).height(6));

        // Section: Installation
        let apps_count = self.applications.len().to_string();
        let repos_count = self.colony_repos().len().to_string();
        let fav_count = self.favorites.len().to_string();

        let label_local = i18n::t("settings_local_apps");
        let label_repos = i18n::t("settings_colony_repos");
        let label_favs = i18n::t("settings_favorites");

        sections = sections.push(
            self.view_collapsible_section(
                "install",
                &i18n::t("settings_section_install"),
                column![
                    self.info_row("\u{f1c0}", label_local, apps_count),
                    Self::divider(),
                    self.info_row("\u{f0c2}", label_repos, repos_count),
                    Self::divider(),
                    self.info_row("\u{f07c}", label_favs, fav_count),
                ]
                .spacing(0)
                .into(),
            ),
        );

        sections.into()
    }

    // ── About settings ──
    fn view_settings_about(&self) -> Element<'_, Message> {
        let cat_title = text(i18n::t("settings_about_title"))
            .size(self.sz(18))
            .font(self.app_font_with_weight(Weight::Bold))
            .color(Palette::TEXT_PRIMARY());

        let mut sections = column![cat_title, container(text("")).height(20),].spacing(0);

        // Version + update button
        let version_label = format!("Colony v{}", env!("CARGO_PKG_VERSION"));

        let update_btn: Element<'_, Message> =
            if let Some((ref tag, _)) = self.launcher_update_available {
                if self.launcher_system_managed {
                    // Package-manager install: the download/apply flow is
                    // guaranteed to fail (the exe dir is not user-writable), so
                    // show the actionable path instead of a doomed button.
                    text(i18n::t_fmt(
                        "launcher_update_system_managed",
                        &[("version", tag)],
                    ))
                    .size(self.sz(13))
                    .font(self.app_font())
                    .color(Palette::WARNING())
                    .into()
                } else if let Some(ref path) = self.launcher_update_staged {
                    let path = path.clone();
                    button(
                        text(i18n::t_fmt("launcher_restart_to_update", &[]))
                            .size(self.sz(13))
                            .font(self.app_font()),
                    )
                    .on_press(Message::ApplyLauncherUpdate(path))
                    .padding([6, 14])
                    .style(|_theme, status| {
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
                    })
                    .into()
                } else {
                    let tag = tag.clone();
                    let is_downloading = self.is_downloading;
                    let btn = button(
                        text(i18n::t_fmt(
                            "launcher_download_update",
                            &[("version", &tag)],
                        ))
                        .size(self.sz(13))
                        .font(self.app_font()),
                    )
                    .padding([6, 14])
                    .style(|_theme, status| {
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
                    });
                    if is_downloading {
                        btn.into()
                    } else {
                        btn.on_press(Message::DownloadLauncherUpdate).into()
                    }
                }
            } else {
                let is_checking = self.is_checking_launcher_update;
                let label = if is_checking {
                    format!("\u{f110}  {}...", i18n::t("check_launcher_updates"))
                } else {
                    i18n::t("check_launcher_updates")
                };
                let btn = button(text(label).size(self.sz(13)).font(self.app_font()))
                    .padding([6, 14])
                    .style(|_theme, status| {
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
                if is_checking {
                    btn.into()
                } else {
                    btn.on_press(Message::CheckLauncherUpdate { manual: true })
                        .into()
                }
            };

        sections = sections.push(
            self.view_collapsible_section(
                "about",
                &i18n::t("settings_about"),
                column![
                    row![
                        text("\u{f015}")
                            .size(self.sz(15))
                            .font(self.app_font())
                            .color(Palette::ACCENT()),
                        text(version_label)
                            .size(self.sz(15))
                            .font(self.app_font())
                            .color(Palette::TEXT_PRIMARY()),
                    ]
                    .spacing(10),
                    container(text("")).height(8),
                    row![
                        text("\u{f09b}")
                            .size(self.sz(15))
                            .font(self.app_font())
                            .color(Palette::TEXT_DIM()),
                        text("MotherSphere/Colony")
                            .size(self.sz(14))
                            .font(self.app_font())
                            .color(Palette::TEXT_MUTED()),
                    ]
                    .spacing(10),
                    container(text("")).height(8),
                    update_btn,
                ]
                .spacing(4)
                .into(),
            ),
        );

        sections.into()
    }

    // ── Keyboard shortcuts reference (task 10) ──
    fn view_settings_shortcuts(&self) -> Element<'_, Message> {
        let cat_title = text(i18n::t("shortcuts_title"))
            .size(self.sz(18))
            .font(self.app_font_with_weight(Weight::Bold))
            .color(Palette::TEXT_PRIMARY());

        let shortcuts = [
            "shortcut_esc",
            "shortcut_tab",
            "shortcut_arrows",
            "shortcut_enter",
            "shortcut_pageupdown",
        ];

        let mut col = column![cat_title, container(text("")).height(20),].spacing(0);

        for key in &shortcuts {
            let shortcut_text = i18n::t(key);
            col = col.push(
                container(
                    text(shortcut_text)
                        .size(self.sz(13))
                        .font(self.app_font())
                        .color(Palette::TEXT_SECONDARY()),
                )
                .padding([8, 0]),
            );
            col = col.push(Self::divider());
        }

        // Hints section
        col = col.push(container(text("")).height(20));
        let hints = [
            "hint_settings",
            "hint_search",
            "hint_favorites",
            "hint_keyboard",
        ];
        for key in &hints {
            col = col.push(
                container(
                    text(format!("\u{f05a}  {}", i18n::t(key)))
                        .size(self.sz(12))
                        .font(self.app_font())
                        .color(Palette::TEXT_MUTED()),
                )
                .padding([4, 0]),
            );
        }

        col.into()
    }

    // ── Theme sub-section ──
    fn view_theme_section(&self) -> Element<'_, Message> {
        let font = self.app_font();
        let medium = self.app_font_with_weight(Weight::Medium);

        let mut col = column![].spacing(12);

        // Rendered straight from colony-ui's generated catalog: adding a theme
        // family upstream needs no change here.
        for family in colony_ui::THEME_FAMILIES {
            let (theme_key, label_key, icon) = (family.key, family.label_key, family.icon);
            let is_selected_family = self.selected_theme == theme_key;
            let label = i18n::t(label_key);

            // Family name label with optional themed icon
            let label_text = if icon.is_empty() {
                label.clone()
            } else {
                format!("{} {}", icon, label)
            };
            let family_label =
                text(label_text)
                    .size(self.sz(13))
                    .font(medium)
                    .color(if is_selected_family {
                        Palette::TEXT_PRIMARY()
                    } else {
                        Palette::TEXT_SECONDARY()
                    });

            // Variant cards as a horizontal row of mini color-swatch cards
            let mut variant_row = row![].spacing(8);

            for variant in family.variants {
                let (var_key, var_label_key) = (&variant.key, variant.label_key);
                let is_active = is_selected_family && self.selected_variant == *var_key;
                let theme_owned = theme_key.to_string();
                let var_owned = var_key.to_string();

                let bg_color = variant.swatch_bg_color();
                let accent_color = variant.swatch_accent_color();

                // Color swatch: bg stripe + accent dot
                let swatch_bg = container(text(""))
                    .width(Length::Fill)
                    .height(Length::Fixed(4.0))
                    .style(move |_theme| container::Style {
                        background: Some(accent_color.into()),
                        border: iced::Border::default().rounded(2),
                        ..Default::default()
                    });

                let swatch = container(swatch_bg)
                    .width(Length::Fill)
                    .height(Length::Fixed(28.0))
                    .padding(iced::Padding {
                        top: 20.0,
                        right: 6.0,
                        bottom: 4.0,
                        left: 6.0,
                    })
                    .style(move |_theme| container::Style {
                        background: Some(bg_color.into()),
                        border: iced::Border::default().rounded(6),
                        ..Default::default()
                    });

                // Variant label below the swatch
                let var_label = text(i18n::t(var_label_key))
                    .size(self.sz(10))
                    .font(font)
                    .color(if is_active {
                        Palette::TEXT_PRIMARY()
                    } else {
                        Palette::TEXT_MUTED()
                    });

                // Check indicator for active variant
                let indicator: Element<'_, Message> = if is_active {
                    text("\u{f00c}")
                        .size(self.sz(8))
                        .font(font)
                        .color(Palette::ACCENT())
                        .into()
                } else {
                    text("").size(self.sz(8)).into()
                };

                let card_content = column![
                    swatch,
                    container(
                        row![var_label, indicator]
                            .spacing(4)
                            .align_y(iced::Alignment::Center)
                    )
                    .padding(iced::Padding {
                        top: 4.0,
                        right: 0.0,
                        bottom: 0.0,
                        left: 2.0
                    }),
                ]
                .spacing(0)
                .width(Length::Fill);

                let card = button(card_content)
                    .on_press(Message::SelectThemeVariant(theme_owned, var_owned))
                    .padding(4)
                    .width(Length::Fill)
                    .style(move |_theme, status| {
                        let border_color = match status {
                            _ if is_active => Palette::ACCENT(),
                            button::Status::Hovered => Palette::TEXT_DIMMER(),
                            _ => Palette::BORDER_SUBTLE(),
                        };
                        button::Style {
                            background: Some(Palette::BG_CARD().into()),
                            text_color: Palette::TEXT_PRIMARY(),
                            border: iced::Border {
                                color: border_color,
                                width: if is_active { 2.0 } else { 1.0 },
                                radius: 8.0.into(),
                            },
                            ..Default::default()
                        }
                    });

                variant_row = variant_row.push(card);
            }

            col = col.push(column![family_label, variant_row,].spacing(6));
        }

        col.into()
    }

    // ── Colors & accents sub-section ──
    fn view_colors_section(&self) -> Element<'_, Message> {
        let font = self.app_font();

        // Accent colors: (key, i18n_label_key, hex_color)
        let accent_colors: Vec<(&str, &str, u32)> = vec![
            ("red", "settings_accent_red", 0xE05555),
            ("orange", "settings_accent_orange", 0xE0855A),
            ("yellow", "settings_accent_yellow", 0xC8A832),
            ("green", "settings_accent_green", 0x55B87A),
            ("blue", "settings_accent_blue", 0x6B8BD6),
            ("indigo", "settings_accent_indigo", 0x7B6BD6),
            ("violet", "settings_accent_violet", 0xB06BD6),
            ("amber", "settings_accent_amber", 0xD4A030),
        ];

        let mut color_row = row![].spacing(8).align_y(iced::Alignment::Center);

        for (color_key, _label_key, hex) in &accent_colors {
            let is_active = self.selected_accent == *color_key;
            let color_key_owned = color_key.to_string();
            let r = ((*hex >> 16) & 0xFF) as f32 / 255.0;
            let g = ((*hex >> 8) & 0xFF) as f32 / 255.0;
            let b = (*hex & 0xFF) as f32 / 255.0;
            let dot_color = iced::Color { r, g, b, a: 1.0 };

            // Circular color swatch button
            let check_icon: Element<'_, Message> = if is_active {
                text("\u{f00c}")
                    .size(self.sz(8))
                    .font(font)
                    .color(iced::Color::WHITE)
                    .into()
            } else {
                text("").size(self.sz(8)).into()
            };
            let swatch = button(
                container(check_icon)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
            )
            .on_press(Message::SelectAccentColor(color_key_owned))
            .width(Length::Fixed(28.0))
            .height(Length::Fixed(28.0))
            .padding(0)
            .style(move |_theme, status| {
                let border_color = match status {
                    _ if is_active => Palette::TEXT_PRIMARY(),
                    button::Status::Hovered => Palette::TEXT_DIMMER(),
                    _ => iced::Color::TRANSPARENT,
                };
                button::Style {
                    background: Some(dot_color.into()),
                    text_color: iced::Color::WHITE,
                    border: iced::Border {
                        color: border_color,
                        width: if is_active { 2.0 } else { 0.0 },
                        radius: 14.0.into(),
                    },
                    ..Default::default()
                }
            });

            color_row = color_row.push(swatch);
        }

        // Auto accent toggle
        let auto_accent_row = self.view_functional_toggle(
            &i18n::t("settings_auto_accent"),
            &i18n::t("settings_auto_accent_desc"),
            self.auto_accent,
            Message::ToggleAutoAccent,
        );

        column![color_row, container(text("")).height(12), auto_accent_row,]
            .spacing(0)
            .into()
    }

    // ── Collapsible section ──
    fn view_collapsible_section<'a>(
        &self,
        key: &str,
        title: &str,
        content: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let is_expanded = self.settings_expanded_sections.contains(key);
        let arrow = if is_expanded { "\u{f078}" } else { "\u{f054}" }; // chevron down / right
        let key_owned = key.to_string();
        let title_owned = title.to_string();

        // Header: clean flat style, no box — just text + chevron
        let header_btn = button(
            row![
                text(title_owned)
                    .size(self.sz(15))
                    .font(self.app_font_with_weight(Weight::Bold))
                    .color(Palette::TEXT_PRIMARY()),
                container(text("")).width(Fill),
                text(arrow)
                    .size(self.sz(9))
                    .font(self.app_font())
                    .color(Palette::TEXT_DIMMER()),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .on_press(Message::SettingsToggleSection(key_owned))
        .padding([12, 4])
        .width(Fill)
        .style(move |_theme, status| {
            let bg = match status {
                button::Status::Hovered => Palette::BG_CARD_HOVER(),
                _ => iced::Color::TRANSPARENT,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: Palette::TEXT_PRIMARY(),
                border: iced::Border::default().rounded(6),
                ..Default::default()
            }
        });

        if is_expanded {
            // Thin divider line under header
            let divider =
                container(text(""))
                    .width(Fill)
                    .height(1)
                    .style(|_theme| container::Style {
                        background: Some(Palette::DIVIDER().into()),
                        ..Default::default()
                    });

            let body = container(content)
                .padding(iced::Padding {
                    top: 12.0,
                    right: 4.0,
                    bottom: 4.0,
                    left: 4.0,
                })
                .width(Fill);

            column![header_btn, divider, body].spacing(0).into()
        } else {
            header_btn.into()
        }
    }

    /// A functional toggle: clicking sends the given message.
    fn view_functional_toggle(
        &self,
        title: &str,
        desc: &str,
        on: bool,
        msg: Message,
    ) -> Element<'_, Message> {
        let font = self.app_font();
        let track_bg = if on {
            Palette::ACCENT()
        } else {
            Palette::BG_CARD_HOVER()
        };
        let knob_offset: f32 = if on { 16.0 } else { 2.0 };

        let knob = container(text(""))
            .width(Length::Fixed(14.0))
            .height(Length::Fixed(14.0))
            .style(move |_theme| container::Style {
                background: Some(Palette::TEXT_PRIMARY().into()),
                border: iced::Border::default().rounded(7),
                ..Default::default()
            });
        let toggle_visual = container(container(knob).padding(iced::Padding {
            top: 1.0,
            right: 0.0,
            bottom: 0.0,
            left: knob_offset,
        }))
        .width(Length::Fixed(34.0))
        .height(Length::Fixed(18.0))
        .style(move |_theme| container::Style {
            background: Some(track_bg.into()),
            border: iced::Border::default().rounded(9),
            ..Default::default()
        });

        button(
            row![
                column![
                    text(title.to_string())
                        .size(self.sz(13))
                        .font(font)
                        .color(Palette::TEXT_PRIMARY()),
                    text(desc.to_string())
                        .size(self.sz(11))
                        .font(font)
                        .color(Palette::TEXT_DIMMER()),
                ]
                .spacing(2),
                container(text("")).width(Fill),
                toggle_visual,
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center),
        )
        .on_press(msg)
        .padding([6, 4])
        .width(Fill)
        .style(|_theme, status| {
            let bg = match status {
                button::Status::Hovered => Palette::BG_CARD_HOVER(),
                _ => iced::Color::TRANSPARENT,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: Palette::TEXT_PRIMARY(),
                border: iced::Border::default().rounded(6),
                ..Default::default()
            }
        })
        .into()
    }

    /// A setting row with a pick_list dropdown for selecting from options.
    /// `options`: Vec of (internal_key, display_label) pairs.
    fn view_pick_list(
        &self,
        title: &str,
        desc: &str,
        options: Vec<(String, String)>,
        selected_key: &str,
        on_select: impl Fn(String) -> Message + 'static,
    ) -> Element<'_, Message> {
        let font = self.app_font();

        let labels: Vec<String> = options.iter().map(|(_, label)| label.clone()).collect();
        let selected_label: Option<String> = options
            .iter()
            .find(|(key, _)| key == selected_key)
            .map(|(_, label)| label.clone());

        let keys: Vec<String> = options.iter().map(|(key, _)| key.clone()).collect();
        let labels_for_map: Vec<String> = labels.clone();

        let pl = pick_list(labels, selected_label, move |chosen_label: String| {
            let idx = labels_for_map
                .iter()
                .position(|l| *l == chosen_label)
                .unwrap_or(0);
            let key = keys.get(idx).cloned().unwrap_or_default();
            on_select(key)
        })
        .text_size(12)
        .padding([4, 10])
        .font(font)
        .style(|_theme, status| {
            let bg = match status {
                pick_list::Status::Active => Palette::BG_CARD(),
                pick_list::Status::Hovered | pick_list::Status::Opened { .. } => {
                    Palette::BG_CARD_HOVER()
                }
            };
            pick_list::Style {
                text_color: Palette::TEXT_SECONDARY(),
                placeholder_color: Palette::TEXT_DIMMER(),
                handle_color: Palette::TEXT_DIMMER(),
                background: bg.into(),
                border: iced::Border {
                    color: Palette::BORDER_SUBTLE(),
                    width: 1.0,
                    radius: 6.0.into(),
                },
            }
        })
        .menu_style(|_theme| overlay_menu::Style {
            background: Palette::BG_CARD().into(),
            border: iced::Border {
                color: Palette::BORDER_SUBTLE(),
                width: 1.0,
                radius: 6.0.into(),
            },
            text_color: Palette::TEXT_PRIMARY(),
            selected_text_color: Palette::TEXT_PRIMARY(),
            selected_background: Palette::ACCENT().into(),
            shadow: iced::Shadow::default(),
        });

        row![
            column![
                text(title.to_string())
                    .size(self.sz(13))
                    .font(font)
                    .color(Palette::TEXT_PRIMARY()),
                text(desc.to_string())
                    .size(self.sz(11))
                    .font(font)
                    .color(Palette::TEXT_DIMMER()),
            ]
            .spacing(2),
            container(text("")).width(Fill),
            pl,
        ]
        .spacing(10)
        .padding([6, 4])
        .align_y(iced::Alignment::Center)
        .into()
    }

    /// Info row with icon, label and value.
    fn info_row<'a>(&self, icon: &'a str, label: String, value: String) -> Element<'a, Message> {
        let font = self.app_font();
        row![
            text(icon)
                .size(self.sz(13))
                .font(font)
                .color(Palette::ACCENT()),
            text(label)
                .size(self.sz(13))
                .font(font)
                .color(Palette::TEXT_MUTED()),
            container(text("")).width(Fill),
            text(value)
                .size(self.sz(13))
                .font(font)
                .color(Palette::TEXT_PRIMARY()),
        ]
        .spacing(10)
        .padding(iced::Padding {
            top: 6.0,
            right: 0.0,
            bottom: 6.0,
            left: 0.0,
        })
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

    /// An action button with icon.
    fn action_button<'a>(
        &self,
        icon: &'a str,
        label: String,
        message: Message,
    ) -> Element<'a, Message> {
        let font = self.app_font();
        button(
            row![
                text(icon)
                    .size(self.sz(13))
                    .font(font)
                    .color(Palette::ACCENT()),
                text(label).size(self.sz(13)).font(font),
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
}
