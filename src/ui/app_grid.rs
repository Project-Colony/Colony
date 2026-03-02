use iced::font::Weight;
use iced::widget::{button, column, container, row, scrollable, text, text_input, Column, Row};
use iced::{Element, Fill};

use crate::github::ColonyRepo;
use crate::scan::Application;
use crate::ui::theme::Palette;
use crate::state::App;
use crate::message::Message;

impl App {
    pub(crate) fn view_content(&self) -> Element<'_, Message> {
        // Show Colony repo detail view if one is selected
        if let Some(index) = self.active_colony_repo {
            let repos = self.colony_repos();
            if let Some(repo) = repos.get(index) {
                return self.view_colony_detail(repo);
            }
        }

        let search = text_input(&crate::i18n::t("search_placeholder"), &self.search_query)
            .on_input(Message::SearchChanged)
            .padding(12)
            .size(self.sz(16))
            .width(Fill);

        // Show search result count when query is active
        let status_text = if !self.search_query.is_empty() {
            let filtered_count = self.filtered_applications().len() + self.filtered_colony_repos().len();
            crate::i18n::t_fmt("n_results_found", &[
                ("count", &filtered_count.to_string()),
                ("query", &self.search_query),
            ])
        } else {
            self.status_message.clone()
        };

        // Show spinner indicator for async operations
        let spinner = if self.is_scanning || self.is_checking_updates || self.is_fetching_repos {
            text("\u{f110} ").size(self.sz(12)).font(self.app_font()).color(Palette::ACCENT())
        } else {
            text("").size(self.sz(12))
        };

        let status = text(status_text)
            .size(self.sz(12))
            .font(self.app_font())
            .color(Palette::TEXT_DIMMER());

        let header = row![search, spinner, status]
            .spacing(8)
            .align_y(iced::Alignment::Center);

        let app_grid = self.view_app_grid();

        let content = column![
            header,
            container(text("")).height(16),
            app_grid
        ]
        .spacing(8)
        .padding(24)
        .width(Fill);

        container(content)
            .style(|_theme| container::Style {
                background: Some(Palette::BG_PRIMARY().into()),
                ..Default::default()
            })
            .width(Fill)
            .height(Fill)
            .into()
    }

    pub(crate) fn view_app_grid(&self) -> Element<'_, Message> {
        let selected_section = self.sections.get(self.selected_section);
        let is_favorites = selected_section.map(|s| s.is_favorites).unwrap_or(false);
        let has_category_filter = selected_section
            .and_then(|s| s.category())
            .is_some();

        // If a category section is selected (not All/Favorites), show Colony repos for that category
        if has_category_filter && !is_favorites {
            let filtered_repos = self.filtered_colony_repos();
            if !filtered_repos.is_empty() {
                return self.view_colony_grid(&filtered_repos);
            }
        }

        // Show combined view for "All", "Favorites", or sections without category
        let is_combined = selected_section
            .map(|s| s.category().is_none() || s.is_favorites)
            .unwrap_or(true);

        let filtered: Vec<&Application> = self.filtered_applications();

        // Include Colony repos for combined sections (All, Favorites)
        let colony_repos = if is_combined {
            self.filtered_colony_repos()
        } else {
            Vec::new()
        };

        if filtered.is_empty() && colony_repos.is_empty() {
            let empty_msg = if !self.search_query.is_empty() {
                crate::i18n::t_fmt("no_results_for", &[("query", &self.search_query)])
            } else {
                crate::i18n::t("no_apps_found")
            };
            return container(
                column![
                    text("\u{f002}").size(self.sz(32)).font(self.app_font()).color(Palette::TEXT_DIMMEST()),
                    container(text("")).height(12),
                    text(empty_msg)
                        .size(self.sz(16))
                        .font(self.app_font())
                        .color(Palette::TEXT_PLACEHOLDER()),
                ]
                .align_x(iced::Alignment::Center),
            )
            .width(Fill)
            .height(Fill)
            .center_x(Fill)
            .center_y(Fill)
            .into();
        }

        // Build a flat list of card elements, then chunk into rows of 4
        let mut all_cards: Vec<Element<'_, Message>> = Vec::new();

        // Colony repo cards first
        for (index, repo) in &colony_repos {
            all_cards.push(self.view_colony_card(*index, repo));
        }

        // Then local app cards
        for app in &filtered {
            all_cards.push(self.view_app_card(app));
        }

        // Drain into rows of 4
        let mut rows: Vec<Element<'_, Message>> = Vec::new();
        let mut current_row: Vec<Element<'_, Message>> = Vec::new();
        for card in all_cards {
            current_row.push(card);
            if current_row.len() == 4 {
                rows.push(Row::with_children(std::mem::take(&mut current_row)).spacing(12).into());
            }
        }
        if !current_row.is_empty() {
            while current_row.len() < 4 {
                current_row.push(container(column![]).width(Fill).into());
            }
            rows.push(Row::with_children(current_row).spacing(12).into());
        }

        let grid = Column::with_children(rows).spacing(12);
        scrollable(grid).height(Fill).into()
    }

    pub(crate) fn view_colony_grid<'a>(&'a self, repos: &[(usize, &'a ColonyRepo)]) -> Element<'a, Message> {
        if repos.is_empty() {
            return container(
                text(crate::i18n::t("no_apps_found"))
                    .size(self.sz(16))
                    .color(Palette::TEXT_PLACEHOLDER()),
            )
            .width(Fill)
            .height(Fill)
            .center_x(Fill)
            .center_y(Fill)
            .into();
        }

        let mut rows: Vec<Element<'a, Message>> = Vec::new();

        for chunk in repos.chunks(4) {
            let mut row_items: Vec<Element<'a, Message>> = Vec::new();

            for (index, repo) in chunk {
                row_items.push(self.view_colony_card(*index, repo));
            }

            while row_items.len() < 4 {
                row_items.push(container(column![]).width(Fill).into());
            }

            rows.push(Row::with_children(row_items).spacing(12).into());
        }

        let grid = Column::with_children(rows).spacing(12);

        scrollable(grid).height(Fill).into()
    }

    pub(crate) fn view_colony_card<'a>(&'a self, index: usize, repo: &'a ColonyRepo) -> Element<'a, Message> {
        let icon_char = repo.name.chars().next().unwrap_or('?').to_uppercase().next().unwrap_or('?');

        let icon = text(icon_char.to_string())
            .size(self.sz(32))
            .font(self.app_font_with_weight(Weight::Medium))
            .color(Palette::ACCENT_ICON());

        let name = text(&repo.name)
            .size(self.sz(14))
            .font(self.app_font())
            .color(Palette::TEXT_PRIMARY());

        let card_content = column![
            container(icon)
                .width(Fill)
                .center_x(Fill),
            container(text("")).height(8),
            container(name)
                .width(Fill)
                .height(32)
                .center_x(Fill)
                .center_y(Fill),
        ]
        .spacing(4)
        .padding(16)
        .width(Fill);

        button(card_content)
            .on_press(Message::ColonyRepoSelected(index))
            .padding(0)
            .width(Fill)
            .height(120)
            .style(|_theme, status| {
                let bg = match status {
                    button::Status::Hovered => Palette::BG_CARD_HOVER(),
                    button::Status::Pressed => Palette::BG_CARD_PRESSED(),
                    _ => Palette::BG_CARD(),
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: Palette::TEXT_PRIMARY(),
                    border: iced::Border::default().rounded(12),
                    ..Default::default()
                }
            })
            .into()
    }

    pub(crate) fn view_app_card(&self, app: &Application) -> Element<'_, Message> {
        let icon_char = app.name.chars().next().unwrap_or('?').to_uppercase().next().unwrap_or('?');

        let icon = text(icon_char.to_string())
            .size(self.sz(32))
            .font(self.app_font_with_weight(Weight::Medium))
            .color(Palette::ACCENT_ICON());

        let name = text(app.name.clone())
            .size(self.sz(14))
            .font(self.app_font())
            .color(Palette::TEXT_PRIMARY());

        let card_content = column![
            container(icon)
                .width(Fill)
                .center_x(Fill),
            container(text("")).height(8),
            container(name)
                .width(Fill)
                .height(32)
                .center_x(Fill)
                .center_y(Fill),
        ]
        .spacing(4)
        .padding(16)
        .width(Fill);

        let exec = app.exec.clone();
        button(card_content)
            .on_press(Message::LaunchApp(exec))
            .padding(0)
            .width(Fill)
            .height(120)
            .style(|_theme, status| {
                let bg = match status {
                    button::Status::Hovered => Palette::BG_CARD_HOVER(),
                    button::Status::Pressed => Palette::BG_CARD_PRESSED(),
                    _ => Palette::BG_CARD(),
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: Palette::TEXT_PRIMARY(),
                    border: iced::Border::default().rounded(12),
                    ..Default::default()
                }
            })
            .into()
    }
}
