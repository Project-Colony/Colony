use iced::font::Weight;
use iced::widget::{button, column, container, row, scrollable, text, Row};
use iced::{Element, Fill};

use crate::github::{self, ColonyRepo};
use crate::ui::theme::Palette;
use crate::state::{App, DetailTab, capitalize_platform};
use crate::message::Message;

impl App {
    pub(crate) fn view_colony_detail<'a>(&'a self, repo: &'a ColonyRepo) -> Element<'a, Message> {
        let back_button = button(text(crate::i18n::t("back")).size(self.sz(13)).font(self.app_font()))
            .on_press(Message::ColonyRepoBack)
            .padding([8, 16]);

        let title = text(&repo.name)
            .size(self.sz(24))
            .font(self.app_font_with_weight(Weight::Bold))
            .color(Palette::TEXT_PRIMARY());

        // Tab bar
        let tabs: Vec<(DetailTab, String)> = vec![
            (DetailTab::ReadMe, crate::i18n::t("tab_readme")),
            (DetailTab::License, crate::i18n::t("tab_license")),
            (DetailTab::Changelog, crate::i18n::t("tab_changelog")),
        ];
        let mut tab_buttons: Vec<Element<'_, Message>> = Vec::new();
        for (tab_val, label) in tabs {
            let is_selected = self.detail_tab == tab_val;
            tab_buttons.push(
                button(
                    text(label)
                        .size(self.sz(13))
                        .font(self.app_font())
                )
                .on_press(Message::DetailTabSelected(tab_val))
                .padding([6, 14])
                .style(move |_theme, status| {
                    let bg = match status {
                        _ if is_selected => Palette::ACCENT(),
                        button::Status::Hovered => Palette::BG_CARD_HOVER(),
                        _ => iced::Color::TRANSPARENT,
                    };
                    button::Style {
                        background: Some(bg.into()),
                        text_color: if is_selected { Palette::TEXT_PRIMARY() } else { Palette::TEXT_MUTED() },
                        border: iced::Border::default().rounded(6),
                        ..Default::default()
                    }
                })
                .into(),
            );
        }
        let tab_bar = Row::with_children(tab_buttons)
            .spacing(4)
            .align_y(iced::Alignment::Center);

        // Tab content — read from disk cache
        let tab_content_text: String = match self.detail_tab {
            DetailTab::ReadMe => repo.description.clone(),
            DetailTab::License => {
                github::read_repo_doc(&repo.name, "LICENSE.md")
                    .unwrap_or_else(|| crate::i18n::t("tab_not_available"))
            }
            DetailTab::Changelog => {
                github::read_repo_doc(&repo.name, "CHANGELOG.md")
                    .unwrap_or_else(|| crate::i18n::t("tab_not_available"))
            }
        };

        let is_placeholder = match self.detail_tab {
            DetailTab::ReadMe => false,
            DetailTab::License => github::read_repo_doc(&repo.name, "LICENSE.md").is_none(),
            DetailTab::Changelog => github::read_repo_doc(&repo.name, "CHANGELOG.md").is_none(),
        };

        let description = text(tab_content_text)
            .size(self.sz(14))
            .font(self.app_font())
            .color(if is_placeholder { Palette::TEXT_MUTED() } else { Palette::TEXT_SECONDARY() });

        let language = text(crate::i18n::t_fmt("language_label", &[("lang", &repo.language)]))
            .size(self.sz(12))
            .font(self.app_font())
            .color(Palette::TEXT_MUTED());

        // Favorite button
        let is_fav = self.is_favorite(&repo.name);
        let fav_icon = if is_fav { "\u{f005}" } else { "\u{f006}" }; // filled/empty star
        let fav_color = if is_fav { Palette::WARNING() } else { Palette::TEXT_DIM() };
        let fav_btn = button(
            text(fav_icon)
                .size(self.sz(18))
                .font(self.app_font())
                .color(fav_color),
        )
        .on_press(Message::ToggleFavorite(repo.name.clone()))
        .padding([8, 12])
        .style(move |_theme, status| {
            let bg = match status {
                button::Status::Hovered => Palette::BG_CARD_HOVER(),
                _ => iced::Color::TRANSPARENT,
            };
            button::Style {
                background: Some(bg.into()),
                text_color: Palette::TEXT_PRIMARY(),
                border: iced::Border::default().rounded(8),
                ..Default::default()
            }
        });

        // Platform tags
        let platform_labels: Vec<Element<'_, Message>> = repo
            .manifest
            .platforms
            .iter()
            .map(|p| {
                text(capitalize_platform(p))
                    .size(self.sz(12))
                    .font(self.app_font())
                    .color(Palette::TEXT_DIM())
                    .into()
            })
            .collect();

        let mut footer_items: Vec<Element<'_, Message>> = vec![
            container(text("")).width(Fill).into(),
        ];
        for pt in platform_labels {
            footer_items.push(pt);
        }
        footer_items.push(language.into());

        let footer = Row::with_children(footer_items)
            .spacing(16)
            .align_y(iced::Alignment::End);

        // Action row: Launch if installed, Download if not
        let installed_path = github::installed_app_path(repo);
        let current_platform = github::current_platform_key();

        let action_row = if let Some(app_path) = installed_path {
            // App is installed — show Launch + Update buttons, and uninstall trash button
            let launch_label = format!("\u{f04b}  {}", crate::i18n::t_fmt("launch", &[("name", &repo.manifest.name)]));
            let launch_btn = button(
                text(launch_label)
                    .size(self.sz(14))
                    .font(self.app_font_with_weight(Weight::Medium)),
            )
            .on_press_maybe(if self.is_downloading { None } else { Some(Message::LaunchColonyApp(app_path)) })
            .padding([12, 24])
            .style(|_theme, status| {
                let bg = match status {
                    button::Status::Hovered => Palette::BTN_SUCCESS_HOVER(),
                    button::Status::Pressed => Palette::BTN_SUCCESS_PRESSED(),
                    _ => Palette::BTN_SUCCESS(),
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: Palette::TEXT_PRIMARY(),
                    border: iced::Border::default().rounded(8),
                    ..Default::default()
                }
            });

            let repo_name = repo.name.clone();
            let platform_key = current_platform.to_string();
            let update_btn = button(
                text(format!("\u{f021}  {}", crate::i18n::t("update")))
                    .size(self.sz(13))
                    .font(self.app_font()),
            )
            .on_press_maybe(if self.is_downloading { None } else { Some(Message::DownloadRelease(repo_name, platform_key)) })
            .padding([10, 20])
            .style(|_theme, status| {
                let bg = match status {
                    button::Status::Hovered => Palette::BTN_HOVER(),
                    button::Status::Pressed => Palette::BTN_PRESSED(),
                    _ => Palette::BTN_DEFAULT(),
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: Palette::TEXT_PRIMARY(),
                    border: iced::Border::default().rounded(8),
                    ..Default::default()
                }
            });

            // Uninstall now triggers confirmation dialog
            let uninstall_repo_name = repo.name.clone();
            let uninstall_btn = button(
                text("\u{f1f8}")
                    .size(self.sz(14))
                    .font(self.app_font())
                    .center(),
            )
            .on_press(Message::ConfirmUninstall(uninstall_repo_name))
            .padding(iced::Padding { top: 10.0, right: 14.0, bottom: 10.0, left: 12.0 })
            .style(|_theme, status| {
                let bg = match status {
                    button::Status::Hovered => Palette::BTN_TRASH_HOVER(),
                    button::Status::Pressed => Palette::BTN_TRASH_PRESSED(),
                    _ => Palette::BTN_DEFAULT(),
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: Palette::TEXT_PRIMARY(),
                    border: iced::Border::default().rounded(8),
                    ..Default::default()
                }
            });

            let spacer: Element<'_, Message> = container(text("")).width(Fill).into();
            Row::new()
                .push(uninstall_btn)
                .push(spacer)
                .push(launch_btn)
                .push(update_btn)
                .spacing(12)
                .align_y(iced::Alignment::Center)
        } else {
            // Not installed — single download button for current platform
            let spacer: Element<'_, Message> = container(text("")).width(Fill).into();
            if repo.manifest.release_files.contains_key(current_platform) {
                let repo_name = repo.name.clone();
                let platform_key = current_platform.to_string();
                let is_dl = self.is_downloading;
                let dl_btn = button(
                    text(format!("\u{f019}  {}", crate::i18n::t("download")))
                        .size(self.sz(14))
                        .font(self.app_font_with_weight(Weight::Medium)),
                )
                .on_press_maybe(if is_dl { None } else { Some(Message::DownloadRelease(repo_name, platform_key)) })
                .padding([12, 24])
                .style(|_theme, status| {
                    let bg = match status {
                        button::Status::Hovered => Palette::BTN_HOVER(),
                        button::Status::Pressed => Palette::BTN_PRESSED(),
                        _ => Palette::BTN_DEFAULT(),
                    };
                    button::Style {
                        background: Some(bg.into()),
                        text_color: Palette::TEXT_PRIMARY(),
                        border: iced::Border::default().rounded(8),
                        ..Default::default()
                    }
                });
                Row::new().push(spacer).push(dl_btn).spacing(12).align_y(iced::Alignment::Center)
            } else {
                let no_release = text(crate::i18n::t("no_release_platform"))
                    .size(self.sz(12))
                    .font(self.app_font())
                    .color(Palette::TEXT_MUTED());
                let spacer2: Element<'_, Message> = container(text("")).width(Fill).into();
                Row::new().push(spacer2).push(no_release)
            }
        };

        let header = row![back_button, container(text("")).width(Fill), fav_btn]
            .width(Fill)
            .align_y(iced::Alignment::Center);

        let desc_container = container(description)
            .width(Fill)
            .padding([16, 24]);

        let body = scrollable(desc_container)
            .width(Fill)
            .height(Fill);

        let detail = column![
            header,
            container(title).width(Fill).center_x(Fill),
            container(text("")).height(8),
            container(tab_bar).width(Fill).center_x(Fill),
            container(text("")).height(4),
            body,
            container(action_row).width(Fill),
            container(text("")).height(8),
            footer
        ]
        .spacing(8)
        .padding(24)
        .width(Fill)
        .height(Fill);

        container(detail)
            .style(|_theme| container::Style {
                background: Some(Palette::BG_PRIMARY().into()),
                ..Default::default()
            })
            .width(Fill)
            .height(Fill)
            .into()
    }
}
