use iced::font::Weight;
use iced::widget::{button, column, container, row, scrollable, stack, text, text_input, Column, Row};
use iced::{Color, Element, Fill, Length};

use crate::github::ColonyRepo;
use crate::scan::{AppCategory, Application};
use crate::ui::theme::{self, Palette};
use crate::state::App;
use crate::message::Message;

/// Filled Material-Design hexagon glyph from the embedded Nerd Font — the
/// Colony "cell" that is the app tile, the brand mark, and the status badge.
const HEX_FILLED: &str = "\u{f02d8}";

/// One-line card summary from a repo description: skip Markdown headings, blank
/// lines and badge rows, take the first real sentence line (README first lines
/// are often just "# ProjectName"), and strip light inline Markdown so the card
/// shows plain prose, not `**bold**` / `[text](url)`.
fn card_summary(desc: &str) -> String {
    let line = desc
        .lines()
        .map(|l| l.trim())
        .find(|l| {
            !l.is_empty() && !l.starts_with('#') && !l.starts_with("![") && !l.starts_with("[![")
        })
        .unwrap_or("");
    strip_inline_markdown(line)
}

/// Drop `*` / `_` emphasis and `` ` `` code marks, and reduce `[text](url)` (and
/// `![alt](url)`) to just the text.
fn strip_inline_markdown(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '*' | '`' | '_' => i += 1,
            '!' if chars.get(i + 1) == Some(&'[') => i += 1, // image marker
            '[' => {
                if let Some(close) = chars[i..].iter().position(|&c| c == ']') {
                    out.extend(chars[i + 1..i + close].iter());
                    i += close + 1;
                    if chars.get(i) == Some(&'(') {
                        if let Some(p) = chars[i..].iter().position(|&c| c == ')') {
                            i += p + 1;
                        } else {
                            i += 1;
                        }
                    }
                } else {
                    out.push('[');
                    i += 1;
                }
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    out.trim().to_string()
}

/// Arrange card elements into a grid of `cols` columns, padding the final row
/// with empty cells so cards keep a consistent width.
fn build_card_grid<'a>(cards: Vec<Element<'a, Message>>, cols: usize) -> Column<'a, Message> {
    let cols = cols.max(1);
    let mut rows: Vec<Element<'a, Message>> = Vec::new();
    let mut current: Vec<Element<'a, Message>> = Vec::new();
    for card in cards {
        current.push(card);
        if current.len() == cols {
            rows.push(Row::with_children(std::mem::take(&mut current)).spacing(12).into());
        }
    }
    if !current.is_empty() {
        while current.len() < cols {
            current.push(container(column![]).width(Fill).into());
        }
        rows.push(Row::with_children(current).spacing(12).into());
    }
    Column::with_children(rows).spacing(12)
}

impl App {
    /// Horizontal "cell" cards render as a single-column list: full-width rows
    /// keep the icon / name / status on one clean vertical rail, and a small
    /// curated catalog reads better as a list than a sparse grid.
    fn grid_columns(_width: f32) -> usize {
        1
    }

    /// FontAwesome solid font (weight 900) for category / status glyphs.
    fn fa_solid(&self) -> iced::Font {
        iced::Font {
            weight: Weight::Black,
            ..iced::Font::with_name(crate::state::FA_FONT_NAME)
        }
    }

    /// The 54px hexagon "cell": a filled hexagon in the app's stable tint, with
    /// the category glyph centered on top. Both glyphs are centered via the text
    /// widget's own alignment (width/height Fill + centered) so they co-register
    /// in the same box. Zero assets — both are embedded font glyphs.
    fn hex_tile<'a>(&self, tint: Color, glyph: &'static str) -> Element<'a, Message> {
        use iced::alignment::{Horizontal, Vertical};
        let fg = theme::contrast_on(tint);
        let hexagon = text(HEX_FILLED)
            .size(self.sz(50))
            .font(self.app_font())
            .color(tint)
            .width(Fill)
            .height(Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center);
        // Render the category glyph in the SAME font as the hexagon (the Nerd
        // Font carries the FontAwesome glyphs too), so the two share identical
        // metrics and centre on the exact same point — no manual nudging.
        let cat = text(glyph)
            .size(self.sz(15))
            .font(self.app_font())
            .color(fg)
            .width(Fill)
            .height(Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center);
        stack![
            container(hexagon).width(54).height(54),
            container(cat).width(54).height(54),
        ]
        .width(54)
        .height(54)
        .into()
    }

    /// A quiet metadata pill (category / language / platform).
    fn chip<'a>(&self, label: String) -> Element<'a, Message> {
        container(
            text(label)
                .size(self.sz(11))
                .font(self.app_font())
                .color(Palette::TEXT_DIMMER()),
        )
        .padding([2, 7])
        .style(|_theme| container::Style {
            border: iced::Border {
                color: Palette::BORDER_SUBTLE(),
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        })
        .into()
    }

    /// Right-aligned install status for a Colony repo: Get / Installed / Update.
    /// Informational (the action lives on the detail page) so the whole card can
    /// stay a single button that opens the detail view.
    fn repo_status<'a>(&self, repo: &'a ColonyRepo) -> Element<'a, Message> {
        let installed = crate::github::installed_app_path(repo).is_some();
        if !installed {
            return text(crate::i18n::t("status_get"))
                .size(self.sz(12))
                .font(self.app_font())
                .color(Palette::ACCENT())
                .into();
        }
        let version = crate::github::load_installed_version(&repo.name);
        if let Some(new_tag) = self.available_updates.get(&repo.name) {
            let head = row![
                text(HEX_FILLED).size(self.sz(11)).font(self.app_font()).color(Palette::WARNING()),
                text(crate::i18n::t("status_update")).size(self.sz(12)).font(self.app_font()).color(Palette::WARNING()),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center);
            let delta = text(format!("{} → {}", version.as_deref().unwrap_or("?"), new_tag))
                .size(self.sz(11))
                .font(self.app_font())
                .color(Palette::TEXT_DIMMER());
            column![head, delta].spacing(3).align_x(iced::Alignment::End).into()
        } else {
            let head = row![
                text(HEX_FILLED).size(self.sz(11)).font(self.app_font()).color(Palette::SUCCESS()),
                text(crate::i18n::t("status_installed")).size(self.sz(12)).font(self.app_font()).color(Palette::SUCCESS()),
            ]
            .spacing(6)
            .align_y(iced::Alignment::Center);
            let mut col = column![head].spacing(3).align_x(iced::Alignment::End);
            if let Some(v) = version {
                col = col.push(
                    text(v).size(self.sz(11)).font(self.app_font()).color(Palette::TEXT_DIMMER()),
                );
            }
            col.into()
        }
    }

    /// Wrap card content in the shared "cell" shell: a bordered box with a
    /// left accent rail in the app's tint (echoing the sidebar's selected item),
    /// plus hover/selected states. `selected` links the grid to the open detail.
    fn cell_shell<'a>(
        &self,
        content: Element<'a, Message>,
        on_press: Message,
        selected: bool,
        accent: Color,
    ) -> Element<'a, Message> {
        let bar_color = if selected { Palette::ACCENT() } else { accent };
        let bar = container(text(""))
            .width(4)
            .height(Fill)
            .style(move |_theme| container::Style {
                background: Some(bar_color.into()),
                border: iced::Border {
                    radius: iced::border::left(12.0),
                    ..Default::default()
                },
                ..Default::default()
            });
        let inner = row![
            bar,
            container(content)
                .padding([0, 15])
                .width(Fill)
                .height(Fill)
                .center_y(Fill)
        ]
        .height(Fill);
        button(inner)
            .on_press(on_press)
            .padding(0)
            .width(Fill)
            .height(Length::Fixed(self.sz(96)))
            .style(move |_theme, status| {
                let bg = match status {
                    _ if selected => Palette::BG_SELECTED(),
                    button::Status::Hovered => Palette::BG_CARD_HOVER(),
                    button::Status::Pressed => Palette::BG_CARD_PRESSED(),
                    _ => Palette::BG_CARD(),
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: Palette::TEXT_PRIMARY(),
                    border: iced::Border {
                        color: Palette::BORDER_SUBTLE(),
                        width: 1.0,
                        radius: 12.0.into(),
                    },
                    ..Default::default()
                }
            })
            .into()
    }

    pub(crate) fn view_content(&self) -> Element<'_, Message> {
        // Show Colony repo detail view if one is selected
        if let Some(index) = self.active_colony_repo {
            let repos = self.colony_repos();
            if let Some(repo) = repos.get(index) {
                return self.view_colony_detail(repo);
            }
        }

        let search_input = text_input(&crate::i18n::t("search_placeholder"), &self.search_query)
            .on_input(Message::SearchChanged)
            .padding(12)
            .size(self.sz(16))
            .width(Fill);
        let search = container(search_input)
            .id(crate::ui::tutorial::ID_SEARCH)
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

        let app_grid = container(self.view_app_grid())
            .id(crate::ui::tutorial::ID_GRID)
            .width(Fill)
            .height(Fill);

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

        // Chunk cards into a grid whose column count adapts to the available
        // width (instead of a hardcoded 4), rebuilt via `responsive` on resize.
        iced::widget::responsive(move |size| {
            let cols = App::grid_columns(size.width);
            let mut all_cards: Vec<Element<'_, Message>> = Vec::new();
            for (index, repo) in &colony_repos {
                all_cards.push(self.view_colony_card(*index, repo));
            }
            for app in &filtered {
                all_cards.push(self.view_app_card(app));
            }
            scrollable(build_card_grid(all_cards, cols)).height(Fill).into()
        })
        .into()
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

        let repos: Vec<(usize, &'a ColonyRepo)> = repos.to_vec();
        iced::widget::responsive(move |size| {
            let cols = App::grid_columns(size.width);
            let mut cards: Vec<Element<'_, Message>> = Vec::new();
            for (index, repo) in &repos {
                cards.push(self.view_colony_card(*index, repo));
            }
            scrollable(build_card_grid(cards, cols)).height(Fill).into()
        })
        .into()
    }

    pub(crate) fn view_colony_card<'a>(&'a self, index: usize, repo: &'a ColonyRepo) -> Element<'a, Message> {
        let category = AppCategory::from_name(&repo.manifest.category);
        let tint = theme::app_tint(&repo.name);
        let tile = self.hex_tile(tint, category.glyph());

        let name = text(&repo.name)
            .size(self.sz(15))
            .font(self.app_font_with_weight(Weight::Medium))
            .color(Palette::TEXT_PRIMARY());

        // One-line summary so every card keeps a uniform, aligned height.
        let summary = card_summary(&repo.description);
        let desc: Element<'a, Message> = if summary.is_empty() {
            container(text("")).into()
        } else {
            container(
                text(summary)
                    .size(self.sz(12))
                    .font(self.app_font())
                    .color(Palette::TEXT_SECONDARY()),
            )
            .width(Fill)
            .max_height(self.sz(12) * 1.6)
            .clip(true)
            .into()
        };

        // Chip row: category, language, present platforms.
        let mut chips = row![self.chip(repo.manifest.category.clone())].spacing(7);
        if !repo.language.is_empty() && repo.language != "Unknown" {
            chips = chips.push(self.chip(repo.language.clone()));
        }
        for plat in repo.manifest.release_files.keys() {
            chips = chips.push(self.chip(crate::state::capitalize_platform(plat)));
        }

        let mid = column![name, desc, chips].spacing(6).width(Fill);

        let status = container(self.repo_status(repo))
            .width(Length::Fixed(136.0))
            .align_x(iced::alignment::Horizontal::Right);

        let content = row![tile, mid, status]
            .spacing(15)
            .align_y(iced::Alignment::Center)
            .width(Fill);

        let selected = self.active_colony_repo == Some(index);
        self.cell_shell(content.into(), Message::ColonyRepoSelected(index), selected, tint)
    }

    pub(crate) fn view_app_card(&self, app: &Application) -> Element<'_, Message> {
        let tint = theme::app_tint(&app.name);
        let tile = self.hex_tile(tint, app.category.glyph());

        let name = text(app.name.clone())
            .size(self.sz(15))
            .font(self.app_font_with_weight(Weight::Medium))
            .color(Palette::TEXT_PRIMARY());

        let chips = row![self.chip(format!("{:?}", app.category))].spacing(7);
        let mid = column![name, chips].spacing(6).width(Fill);

        // Locally-detected apps are launchable; a quiet play glyph is the cue.
        let launch = text("\u{f04b}")
            .size(self.sz(13))
            .font(self.fa_solid())
            .color(Palette::TEXT_DIMMER());
        let status = container(launch)
            .width(Length::Fixed(136.0))
            .align_x(iced::alignment::Horizontal::Right);

        let content = row![tile, mid, status]
            .spacing(15)
            .align_y(iced::Alignment::Center)
            .width(Fill);

        self.cell_shell(content.into(), Message::LaunchApp(app.exec.clone()), false, tint)
    }
}
