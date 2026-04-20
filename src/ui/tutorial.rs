use iced::advanced::widget::operation::Outcome;
use iced::advanced::widget::{Id, Operation};
use iced::font::Weight;
use iced::widget::{button, column, container, row, text, Column};
use iced::{Color, Element, Fill, Length, Rectangle, Task};

use crate::i18n;
use crate::message::Message;
use crate::state::App;
use crate::ui::theme::Palette;

// ---- container IDs (tagged in the real widget tree) -------------------------
pub(crate) const ID_SIDEBAR_CATS: Id = Id::new("tut-sidebar-cats");
pub(crate) const ID_SEARCH: Id = Id::new("tut-search");
pub(crate) const ID_GRID: Id = Id::new("tut-grid");
pub(crate) const ID_GITHUB: Id = Id::new("tut-github");

// ---- bounds collected from the live layout ----------------------------------
#[derive(Debug, Default, Clone, Copy)]
pub struct TutorialBounds {
    pub sidebar_cats: Option<Rectangle>,
    pub search: Option<Rectangle>,
    pub grid: Option<Rectangle>,
    pub github_area: Option<Rectangle>,
}

/// Widget operation that walks the tree and grabs bounds of tagged containers.
struct CollectBounds {
    out: TutorialBounds,
}

impl Operation<TutorialBounds> for CollectBounds {
    fn traverse(
        &mut self,
        operate: &mut dyn FnMut(&mut dyn Operation<TutorialBounds>),
    ) {
        operate(self);
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        let Some(id) = id else { return };
        if *id == ID_SIDEBAR_CATS {
            self.out.sidebar_cats = Some(bounds);
        } else if *id == ID_SEARCH {
            self.out.search = Some(bounds);
        } else if *id == ID_GRID {
            self.out.grid = Some(bounds);
        } else if *id == ID_GITHUB {
            self.out.github_area = Some(bounds);
        }
    }

    fn finish(&self) -> Outcome<TutorialBounds> {
        Outcome::Some(self.out)
    }
}

pub fn fetch_bounds_task() -> Task<Message> {
    iced::advanced::widget::operate(CollectBounds {
        out: TutorialBounds::default(),
    })
    .map(Message::TutorialBoundsUpdated)
}

// ---- step definitions -------------------------------------------------------

#[derive(Clone, Copy)]
struct SpotRect {
    x: u16,
    y: u16,
    w: u16,
    h: u16,
}

impl SpotRect {
    fn from_rect(r: Rectangle, pad: u16) -> Self {
        let p = pad as f32;
        SpotRect {
            x: (r.x - p).max(0.0) as u16,
            y: (r.y - p).max(0.0) as u16,
            w: (r.width + 2.0 * p).max(1.0) as u16,
            h: (r.height + 2.0 * p).max(1.0) as u16,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Placement {
    Center,
    Right,
    Left,
    Below,
    Above,
}

#[derive(Clone, Copy)]
enum Zone {
    None,
    SidebarCats,
    Search,
    Grid,
    GithubArea,
}

struct Step {
    title_key: &'static str,
    desc_key: &'static str,
    zone: Zone,
    fallback: Option<SpotRect>,
    placement: Placement,
    show_github_btn: bool,
    pad: u16,
}

pub(crate) const TUTORIAL_LAST_STEP: u8 = 5;

fn steps() -> [Step; 6] {
    [
        Step {
            title_key: "welcome_title",
            desc_key: "welcome_desc",
            zone: Zone::None,
            fallback: None,
            placement: Placement::Center,
            show_github_btn: false,
            pad: 0,
        },
        Step {
            title_key: "tut_sidebar_title",
            desc_key: "tut_sidebar_desc",
            zone: Zone::SidebarCats,
            fallback: Some(SpotRect { x: 0, y: 56, w: 200, h: 380 }),
            placement: Placement::Right,
            show_github_btn: false,
            pad: 8,
        },
        Step {
            title_key: "tut_search_title",
            desc_key: "tut_search_desc",
            zone: Zone::Search,
            fallback: Some(SpotRect { x: 216, y: 16, w: 760, h: 68 }),
            placement: Placement::Below,
            show_github_btn: false,
            pad: 10,
        },
        Step {
            title_key: "tut_grid_title",
            desc_key: "tut_grid_desc",
            zone: Zone::Grid,
            fallback: Some(SpotRect { x: 216, y: 92, w: 760, h: 560 }),
            placement: Placement::Left,
            show_github_btn: false,
            pad: 10,
        },
        Step {
            title_key: "tut_github_title",
            desc_key: "tut_github_desc",
            zone: Zone::GithubArea,
            fallback: Some(SpotRect { x: 0, y: 540, w: 200, h: 130 }),
            placement: Placement::Above,
            show_github_btn: true,
            pad: 8,
        },
        Step {
            title_key: "tut_finish_title",
            desc_key: "tut_finish_desc",
            zone: Zone::None,
            fallback: None,
            placement: Placement::Center,
            show_github_btn: false,
            pad: 0,
        },
    ]
}

fn resolve_rect(step: &Step, bounds: &TutorialBounds) -> Option<SpotRect> {
    let live = match step.zone {
        Zone::None => return None,
        Zone::SidebarCats => bounds.sidebar_cats,
        Zone::Search => bounds.search,
        Zone::Grid => bounds.grid,
        Zone::GithubArea => bounds.github_area,
    };
    live.map(|r| SpotRect::from_rect(r, step.pad)).or(step.fallback)
}

impl App {
    pub(crate) fn view_tutorial(&self) -> Element<'_, Message> {
        let all = steps();
        let idx = (self.welcome_step as usize).min(all.len() - 1);
        let step = &all[idx];

        let bubble = self.tutorial_bubble(idx, step);

        match resolve_rect(step, &self.tutorial_bounds) {
            None => self.tutorial_centered(bubble),
            Some(rect) => self.tutorial_spotlight(rect, step.placement, bubble),
        }
    }

    fn tutorial_centered<'a>(&'a self, bubble: Element<'a, Message>) -> Element<'a, Message> {
        let card = container(bubble).max_width(560);
        container(card)
            .width(Fill)
            .height(Fill)
            .center_x(Fill)
            .center_y(Fill)
            .style(|_| container::Style {
                background: Some(Color { r: 0.0, g: 0.0, b: 0.0, a: 0.65 }.into()),
                ..Default::default()
            })
            .into()
    }

    fn tutorial_spotlight<'a>(
        &'a self,
        rect: SpotRect,
        placement: Placement,
        bubble: Element<'a, Message>,
    ) -> Element<'a, Message> {
        let bg = Color { r: 0.0, g: 0.0, b: 0.0, a: 0.7 };
        let band_style = move |_: &iced::Theme| container::Style {
            background: Some(bg.into()),
            ..Default::default()
        };

        let mut bubble_slot: Option<Element<'a, Message>> = Some(bubble);

        let take = |slot: &mut Option<Element<'a, Message>>| -> Element<'a, Message> {
            slot.take().unwrap_or_else(|| container(text("")).into())
        };

        // ---- Top band ----
        let top_band: Element<'a, Message> = if placement == Placement::Above {
            let content = container(take(&mut bubble_slot)).max_width(420);
            container(
                container(content)
                    .center_x(Fill)
                    .align_y(iced::alignment::Vertical::Bottom)
                    .width(Fill)
                    .height(Fill)
                    .padding(iced::Padding { top: 0.0, right: 16.0, bottom: 16.0, left: 16.0 }),
            )
            .width(Fill)
            .height(Length::Fixed(rect.y as f32))
            .style(band_style)
            .into()
        } else {
            container(text(""))
                .width(Fill)
                .height(Length::Fixed(rect.y as f32))
                .style(band_style)
                .into()
        };

        // ---- Middle row: left | hole | right ----
        let hole: Element<'a, Message> = container(text(""))
            .width(Length::Fixed(rect.w as f32))
            .height(Length::Fixed(rect.h as f32))
            .into();

        let left_band: Element<'a, Message> = if placement == Placement::Left {
            let bubble_max = rect.x.saturating_sub(24).max(160) as f32;
            let content = container(take(&mut bubble_slot)).max_width(bubble_max);
            container(
                container(content)
                    .center_y(Fill)
                    .width(Fill)
                    .height(Fill)
                    .padding(iced::Padding { top: 0.0, right: 12.0, bottom: 0.0, left: 12.0 }),
            )
            .width(Length::Fixed(rect.x as f32))
            .height(Length::Fixed(rect.h as f32))
            .style(band_style)
            .into()
        } else {
            container(text(""))
                .width(Length::Fixed(rect.x as f32))
                .height(Length::Fixed(rect.h as f32))
                .style(band_style)
                .into()
        };

        let right_band: Element<'a, Message> = if placement == Placement::Right {
            let content = container(take(&mut bubble_slot)).max_width(340);
            container(
                container(content)
                    .center_y(Fill)
                    .width(Fill)
                    .height(Fill)
                    .padding(iced::Padding { top: 0.0, right: 16.0, bottom: 0.0, left: 16.0 }),
            )
            .width(Fill)
            .height(Length::Fixed(rect.h as f32))
            .style(band_style)
            .into()
        } else {
            container(text(""))
                .width(Fill)
                .height(Length::Fixed(rect.h as f32))
                .style(band_style)
                .into()
        };

        let middle = row![left_band, hole, right_band].height(Length::Fixed(rect.h as f32));

        // ---- Bottom band ----
        let bottom_band: Element<'a, Message> = if placement == Placement::Below {
            let content = container(take(&mut bubble_slot)).max_width(560);
            container(
                container(content)
                    .center_x(Fill)
                    .align_y(iced::alignment::Vertical::Top)
                    .width(Fill)
                    .height(Fill)
                    .padding(iced::Padding { top: 16.0, right: 16.0, bottom: 0.0, left: 16.0 }),
            )
            .width(Fill)
            .height(Fill)
            .style(band_style)
            .into()
        } else {
            container(text(""))
                .width(Fill)
                .height(Fill)
                .style(band_style)
                .into()
        };

        column![top_band, middle, bottom_band]
            .width(Fill)
            .height(Fill)
            .into()
    }

    fn tutorial_bubble<'a>(&'a self, idx: usize, step: &Step) -> Element<'a, Message> {
        let is_last = idx as u8 >= TUTORIAL_LAST_STEP;
        let is_first = idx == 0;

        let primary_btn = |label_key: &'static str, msg: Message| -> iced::widget::Button<'a, Message> {
            button(
                text(i18n::t(label_key)).size(self.sz(13)).font(self.app_font()),
            )
            .on_press(msg)
            .padding([9, 18])
            .style(|_theme, status| {
                let bg = match status {
                    button::Status::Hovered => Palette::BTN_SUCCESS_HOVER(),
                    _ => Palette::BTN_SUCCESS(),
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: Palette::TEXT_PRIMARY(),
                    border: iced::Border::default().rounded(8),
                    ..Default::default()
                }
            })
        };

        let ghost_btn = |label_key: &'static str, msg: Message| -> iced::widget::Button<'a, Message> {
            button(
                text(i18n::t(label_key))
                    .size(self.sz(12))
                    .font(self.app_font())
                    .color(Palette::TEXT_SECONDARY()),
            )
            .on_press(msg)
            .padding([8, 14])
            .style(|_theme, status| {
                let bg = match status {
                    button::Status::Hovered => Palette::BTN_HOVER(),
                    _ => Color::TRANSPARENT,
                };
                button::Style {
                    background: Some(bg.into()),
                    text_color: Palette::TEXT_SECONDARY(),
                    border: iced::Border::default().rounded(8),
                    ..Default::default()
                }
            })
        };

        let dot = |active: bool| -> iced::widget::Container<'a, Message> {
            let c = if active { Palette::ACCENT() } else { Palette::BTN_HOVER() };
            container(text(""))
                .width(7)
                .height(7)
                .style(move |_| container::Style {
                    background: Some(c.into()),
                    border: iced::Border::default().rounded(4),
                    ..Default::default()
                })
        };

        let dots = row![
            dot(idx == 0),
            dot(idx == 1),
            dot(idx == 2),
            dot(idx == 3),
            dot(idx == 4),
            dot(idx == 5),
        ]
        .spacing(6);

        let left_btn: Element<'a, Message> = if is_first {
            ghost_btn("welcome_skip", Message::DismissFirstLaunch).into()
        } else {
            ghost_btn("welcome_back", Message::WelcomeBack).into()
        };
        let right_label = if is_last { "welcome_start" } else { "welcome_next" };
        let right_btn: Element<'a, Message> = primary_btn(right_label, Message::WelcomeNext).into();

        let nav = row![
            left_btn,
            container(text("")).width(Fill),
            container(dots).center_x(Fill),
            container(text("")).width(Fill),
            right_btn,
        ]
        .align_y(iced::Alignment::Center);

        let title_size = if matches!(step.zone, Zone::None) { self.sz(24) } else { self.sz(18) };

        let mut body: Column<'a, Message> = column![
            text(i18n::t(step.title_key))
                .size(title_size)
                .font(self.app_font_with_weight(Weight::Bold))
                .color(Palette::TEXT_PRIMARY()),
            container(text("")).height(8),
            text(i18n::t(step.desc_key))
                .size(self.sz(13))
                .font(self.app_font())
                .color(Palette::TEXT_SECONDARY()),
        ]
        .spacing(2);

        if step.show_github_btn {
            body = body.push(container(text("")).height(10));
            body = body.push(
                row![
                    primary_btn("welcome_connect_now", Message::WelcomeConnectGithub),
                    ghost_btn("welcome_later", Message::WelcomeNext),
                ]
                .spacing(8),
            );
        }

        let card = column![body, container(text("")).height(14), nav]
            .spacing(0)
            .padding(22);

        container(card)
            .style(|_| container::Style {
                background: Some(Palette::BG_SIDEBAR().into()),
                border: iced::Border {
                    color: Palette::ACCENT(),
                    width: 1.0,
                    radius: 14.0.into(),
                },
                ..Default::default()
            })
            .into()
    }
}

