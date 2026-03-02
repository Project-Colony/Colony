use iced::font::Weight;
use iced::widget::{button, column, container, row, scrollable, text, Column};

use iced::{Element, Fill, Length};

use crate::sections::Section;
use crate::ui::theme::Palette;
use crate::state::{App, GitHubState};
use crate::message::Message;

impl App {
    pub(crate) fn view_sidebar(&self) -> Element<'_, Message> {
        let title_text = row![
            text("Colony")
                .size(self.sz(30))
                .font(self.app_font_with_weight(Weight::Bold)),
            text("\u{f013}")
                .size(self.sz(14))
                .font(self.app_font())
                .color(if self.show_settings { Palette::TEXT_PRIMARY() } else { Palette::TEXT_DIMMER() }),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center);

        let show_settings = self.show_settings;
        let title = button(title_text)
            .on_press(Message::ToggleSettings)
            .padding([4, 8])
            .style(move |_theme, status| {
                let bg = match status {
                    button::Status::Hovered => Palette::BG_CARD_HOVER(),
                    _ if show_settings => Palette::BG_SELECTED(),
                    _ => iced::Color::TRANSPARENT,
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: Palette::TEXT_PRIMARY(),
                    border: iced::Border::default().rounded(8),
                    ..Default::default()
                }
            });

        let category_header = text(crate::i18n::t("categories"))
            .size(self.sz(13))
            .font(self.app_font())
            .color(Palette::TEXT_MUTED());

        let hint_text = text(crate::i18n::t("hint_keyboard"))
            .size(self.sz(10))
            .font(self.app_font())
            .color(Palette::TEXT_DIMMEST());

        let category_buttons: Vec<Element<'_, Message>> = self
            .sections
            .iter()
            .enumerate()
            .map(|(index, section)| self.view_section_button(index, section))
            .collect();

        let category_list = Column::with_children(category_buttons).spacing(4);
        let category_scroll = scrollable(category_list).height(Length::Fill);

        // GitHub button — Nerd Font icon  (U+F09B)
        let github_connected = matches!(self.github_state, GitHubState::Connected { .. });
        let github_icon_color = if github_connected {
            Palette::ACCENT()
        } else if self.show_github_menu {
            Palette::TEXT_PRIMARY()
        } else {
            Palette::TEXT_DIM()
        };

        let github_label = if github_connected {
            "GitHub \u{f00c}"
        } else {
            "GitHub"
        };

        let github_btn_content = row![
            text("\u{f09b}").size(self.sz(18)).font(self.app_font()).color(github_icon_color),
            text(github_label).size(self.sz(13)).font(self.app_font()).color(github_icon_color),
        ]
        .spacing(10)
        .align_y(iced::Alignment::Center);

        let show_menu = self.show_github_menu;
        let github_btn = button(github_btn_content)
            .on_press(Message::ToggleGitHubMenu)
            .padding([10, 14])
            .width(Fill)
            .style(move |_theme, _status| {
                button::Style {
                    background: Some(if show_menu {
                        Palette::BG_SELECTED()
                    } else {
                        iced::Color::TRANSPARENT
                    }.into()),
                    text_color: Palette::TEXT_PRIMARY(),
                    border: iced::Border::default().rounded(6),
                    ..Default::default()
                }
            });

        let rescan_label = if self.is_scanning {
            format!("\u{f110}  {}", crate::i18n::t("scanning"))
        } else {
            crate::i18n::t("rescan")
        };
        let rescan_btn_base = button(text(rescan_label).size(self.sz(13)).font(self.app_font()))
            .padding([8, 16])
            .width(Fill);
        let rescan_btn = if self.is_scanning {
            rescan_btn_base
        } else {
            rescan_btn_base.on_press(Message::Rescan)
        };

        // Launcher update badge
        let update_badge: Element<'_, Message> = if self.launcher_update_available.is_some() {
            let (label, msg): (String, Message) = if let Some(ref path) = self.launcher_update_staged {
                (
                    crate::i18n::t("launcher_restart_to_update"),
                    Message::ApplyLauncherUpdate(path.clone()),
                )
            } else {
                let tag = &self.launcher_update_available.as_ref().unwrap().0;
                (
                    crate::i18n::t_fmt("launcher_update_available_short", &[("version", tag)]),
                    Message::DownloadLauncherUpdate,
                )
            };

            let is_downloading = self.is_downloading;
            button(
                text(label)
                    .size(self.sz(11))
                    .font(self.app_font())
                    .color(Palette::ACCENT()),
            )
            .on_press_maybe(if is_downloading { None } else { Some(msg) })
            .padding([6, 12])
            .width(Fill)
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
            container(text("")).height(0).into()
        };

        let sidebar_content = column![
            title,
            update_badge,
            container(text("")).height(24),
            category_header,
            category_scroll,
            hint_text,
            container(text("")).height(4),
            github_btn,
            container(text("")).height(4),
            rescan_btn,
        ]
        .spacing(10)
        .padding(16)
        .width(200);

        container(sidebar_content)
            .style(|_theme| container::Style {
                background: Some(Palette::BG_SIDEBAR().into()),
                ..Default::default()
            })
            .height(Fill)
            .into()
    }

    pub(crate) fn view_section_button(&self, index: usize, section: &Section) -> Element<'_, Message> {
        let is_selected = self.selected_section == index && !self.show_github_menu;

        let text_color = if is_selected {
            Palette::TEXT_PRIMARY()
        } else {
            Palette::TEXT_DIM()
        };

        // Animated indicator: alpha based on distance from animated position
        let indicator_alpha = if self.animations && !self.reduce_motion {
            let button_y = index as f32 * 44.0;
            let distance = (self.sidebar_indicator_pos() - button_y).abs();
            (1.0 - distance / 44.0).clamp(0.0, 1.0)
        } else if is_selected {
            1.0
        } else {
            0.0
        };
        let accent = Palette::ACCENT();
        let indicator_color = iced::Color { a: accent.a * indicator_alpha, ..accent };

        let indicator = container(text(""))
            .width(4)
            .height(Length::Fill)
            .style(move |_theme| container::Style {
                background: Some(indicator_color.into()),
                ..Default::default()
            });

        let icon = text(section.icon.clone())
            .size(self.sz(15))
            .font(self.app_font())
            .color(text_color);

        let label = text(section.name.clone())
            .size(self.sz(14))
            .font(self.app_font())
            .color(text_color);

        let content = row![indicator, icon, label]
            .spacing(10)
            .align_y(iced::Alignment::Center);

        let btn = button(content)
            .on_press(Message::SectionSelected(index))
            .padding([10, 14])
            .width(Fill)
            .style(move |theme, status| {
                if is_selected {
                    button::Style {
                        background: Some(Palette::BG_SELECTED().into()),
                        text_color: Palette::TEXT_PRIMARY(),
                        border: iced::Border::default().rounded(6),
                        ..button::primary(theme, status)
                    }
                } else {
                    button::Style {
                        background: Some(iced::Color::TRANSPARENT.into()),
                        text_color: Palette::TEXT_DIMMER(),
                        border: iced::Border::default().rounded(6),
                        ..button::secondary(theme, status)
                    }
                }
            });

        btn.into()
    }
}
