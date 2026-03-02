use iced::font::Weight;
use iced::widget::{button, column, container, row, scrollable, text, Column};
use iced::{Element, Fill, Length};

use crate::github::ColonyRepo;
use crate::ui::theme::Palette;
use crate::state::{App, GitHubState};
use crate::message::Message;

impl App {
    pub(crate) fn view_github_panel(&self) -> Element<'_, Message> {
        let header_text = text("GitHub")
            .size(self.sz(24))
            .font(self.app_font_with_weight(Weight::Bold))
            .color(Palette::TEXT_PRIMARY());

        let content: Element<'_, Message> = match &self.github_state {
            GitHubState::Disconnected => {
                let desc = text(crate::i18n::t("github_connect_desc"))
                    .size(self.sz(14))
                    .font(self.app_font())
                    .color(Palette::TEXT_SECONDARY());

                let login_btn_content = row![
                    text("\u{f09b}").size(self.sz(18)).font(self.app_font()),
                    text(crate::i18n::t("github_login")).size(self.sz(14)).font(self.app_font()),
                ]
                .spacing(10)
                .align_y(iced::Alignment::Center);

                let login_btn = button(login_btn_content)
                    .on_press_maybe(if matches!(self.github_state, crate::GitHubState::Connecting { .. }) {
                        None
                    } else {
                        Some(Message::GitHubLogin)
                    })
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

                let info = text(crate::i18n::t("github_public_api"))
                    .size(self.sz(12))
                    .font(self.app_font())
                    .color(Palette::TEXT_MUTED());

                column![desc, container(text("")).height(16), login_btn, container(text("")).height(12), info]
                    .spacing(8)
                    .into()
            }
            GitHubState::Connecting { user_code } => {
                match user_code {
                    Some(code) => {
                        let spinner_label = text("\u{f110}  ")
                            .size(self.sz(14))
                            .font(self.app_font())
                            .color(Palette::ACCENT());
                        let label = text(crate::i18n::t("github_enter_code"))
                            .size(self.sz(14))
                            .font(self.app_font())
                            .color(Palette::TEXT_MUTED());
                        let code_btn = button(
                            text(code.as_str())
                                .size(self.sz(28))
                                .font(self.app_font())
                                .color(Palette::TEXT_SECONDARY())
                        )
                        .on_press(Message::CopyToClipboard(code.clone()))
                        .padding([8, 16])
                        .style(|_theme, status| {
                            let bg = match status {
                                button::Status::Hovered => Palette::BG_SELECTED(),
                                _ => Palette::BG_INPUT(),
                            };
                            button::Style {
                                background: Some(bg.into()),
                                text_color: Palette::TEXT_SECONDARY(),
                                border: iced::Border::default().rounded(8),
                                ..Default::default()
                            }
                        });
                        let hint = text(crate::i18n::t("github_copy_hint"))
                            .size(self.sz(12))
                            .font(self.app_font())
                            .color(Palette::TEXT_DIMMEST());
                        column![spinner_label, label, code_btn, hint]
                            .spacing(8)
                            .into()
                    }
                    None => {
                        row![
                            text("\u{f110}").size(self.sz(16)).font(self.app_font()).color(Palette::ACCENT()),
                            text(crate::i18n::t("github_connecting"))
                                .size(self.sz(16))
                                .font(self.app_font())
                                .color(Palette::TEXT_SECONDARY()),
                        ]
                        .spacing(8)
                        .align_y(iced::Alignment::Center)
                        .into()
                    }
                }
            }
            GitHubState::Connected { session, repos } => {
                let connected_text = format!(
                    "{}{}",
                    crate::i18n::t("github_connected"),
                    session.username.as_ref().map(|u| format!(" — {u}")).unwrap_or_default()
                );
                let user_label: Element<'_, Message> = if self.is_fetching_repos {
                    row![
                        text("\u{f110}").size(self.sz(14)).font(self.app_font()).color(Palette::ACCENT()),
                        text(format!("{} ({})", connected_text, crate::i18n::t("syncing_repos")))
                            .size(self.sz(14))
                            .font(self.app_font())
                            .color(Palette::SUCCESS()),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center)
                    .into()
                } else {
                    text(connected_text)
                        .size(self.sz(14))
                        .font(self.app_font())
                        .color(Palette::SUCCESS())
                        .into()
                };

                let repo_header = text(crate::i18n::t_fmt(
                    "github_repos_detected",
                    &[("count", &repos.len().to_string())],
                ))
                .size(self.sz(16))
                .font(self.app_font_with_weight(Weight::Medium))
                .color(Palette::TEXT_PRIMARY());

                let mut repo_list_items: Vec<Element<'_, Message>> = Vec::new();
                for repo in repos {
                    repo_list_items.push(self.view_repo_card(repo));
                }

                let repo_list = if repo_list_items.is_empty() {
                    column![
                        text(crate::i18n::t("github_no_repos"))
                            .size(self.sz(13))
                            .font(self.app_font())
                            .color(Palette::TEXT_DIMMER())
                    ]
                } else {
                    Column::with_children(repo_list_items).spacing(8)
                };

                let refresh_btn_base = button(
                    text(crate::i18n::t("github_refresh")).size(self.sz(13)).font(self.app_font()),
                )
                .padding([8, 16]);
                let refresh_btn = if self.is_fetching_repos {
                    refresh_btn_base
                } else {
                    refresh_btn_base.on_press(Message::GitHubRefreshRepos)
                }
                .style(|_theme, status| {
                    let bg = match status {
                        button::Status::Hovered => Palette::BG_SELECTED(),
                        _ => Palette::BG_CARD_HOVER(),
                    };
                    button::Style {
                        background: Some(bg.into()),
                        text_color: Palette::TEXT_PRIMARY(),
                        border: iced::Border::default().rounded(6),
                        ..Default::default()
                    }
                });

                let logout_btn = button(
                    text(crate::i18n::t("github_logout")).size(self.sz(13)).font(self.app_font()),
                )
                .on_press(Message::GitHubLogout)
                .padding([8, 16])
                .style(|_theme, status| {
                    let bg = match status {
                        button::Status::Hovered => Palette::BTN_DANGER_HOVER(),
                        _ => Palette::BTN_DANGER_BG(),
                    };
                    button::Style {
                        background: Some(bg.into()),
                        text_color: Palette::ERROR_LIGHT(),
                        border: iced::Border::default().rounded(6),
                        ..Default::default()
                    }
                });

                let actions = row![refresh_btn, logout_btn].spacing(12);

                column![
                    user_label,
                    container(text("")).height(8),
                    repo_header,
                    container(text("")).height(8),
                    scrollable(repo_list).height(Length::Fill),
                    container(text("")).height(12),
                    actions,
                ]
                .spacing(4)
                .into()
            }
            GitHubState::Error(e) => {
                let err = text(crate::i18n::t_fmt("github_error", &[("error", e.as_str())]))
                    .size(self.sz(14))
                    .font(self.app_font())
                    .color(Palette::ERROR());

                let retry_btn = button(
                    text(crate::i18n::t("github_retry")).size(self.sz(13)).font(self.app_font()),
                )
                .on_press(Message::GitHubLogin)
                .padding([8, 16]);

                column![err, container(text("")).height(12), retry_btn]
                    .spacing(8)
                    .into()
            }
        };

        let panel = column![
            header_text,
            container(text("")).height(16),
            content,
        ]
        .spacing(8)
        .padding(24)
        .width(Fill)
        .height(Fill);

        container(panel)
            .style(|_theme| container::Style {
                background: Some(Palette::BG_PRIMARY().into()),
                ..Default::default()
            })
            .width(Fill)
            .height(Fill)
            .into()
    }

    pub(crate) fn view_repo_card<'a>(&'a self, repo: &'a ColonyRepo) -> Element<'a, Message> {
        let name = text(&repo.name)
            .size(self.sz(15))
            .font(self.app_font_with_weight(Weight::Medium))
            .color(Palette::TEXT_PRIMARY());

        let card = container(name)
            .padding([10, 14])
            .width(Fill)
            .style(|_theme| container::Style {
                background: Some(Palette::BG_CARD().into()),
                border: iced::Border::default().rounded(8),
                ..Default::default()
            });

        card.into()
    }
}
