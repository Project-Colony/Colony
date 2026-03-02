mod github;
mod i18n;
mod message;
mod oauth;
mod scan;
mod sections;
mod state;
mod ui;
mod update;

use iced::font::{self, Weight};
use iced::widget::{
    button, column, container, row, stack, text, Column,
};
use iced::keyboard;
use iced::{Element, Fill, Subscription, Task, Theme};
use ui::theme::{Palette, set_active_theme, set_active_accent, set_high_contrast, accent_key_to_color};
use std::collections::HashSet;

use message::Message;
use state::{
    App, DetailTab, GitHubState,
    APP_FONT_BYTES, DYSLEXIA_FONT_BYTES,
    default_font,
};

pub fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    i18n::init();

    iced::application(
        App::boot,
        App::update,
        App::view,
    )
    .title(App::title)
    .theme(App::theme)
    .subscription(App::subscription)
    .default_font(default_font())
    .window_size((1000.0, 700.0))
    .run()
}

fn load_fonts() -> Task<Message> {
    let main_fonts = APP_FONT_BYTES
        .iter()
        .map(|data| font::load(data.to_vec()).map(Message::FontLoaded));
    let dyslexia_font = std::iter::once(
        font::load(DYSLEXIA_FONT_BYTES.to_vec()).map(Message::FontLoaded),
    );
    Task::batch(main_fonts.chain(dyslexia_font))
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        let prefs = github::load_preferences();

        let should_scan = prefs.scan_on_startup.unwrap_or(true);
        let applications = if should_scan {
            scan::scan_applications().unwrap_or_else(|e| {
                tracing::error!("Scan error: {e}");
                Vec::new()
            })
        } else {
            Vec::new()
        };

        // Save scan results to cache
        let cached_apps: Vec<github::CachedApp> = applications
            .iter()
            .map(|app| github::CachedApp {
                name: app.name.clone(),
                exec: app.exec.clone(),
                icon: app.icon.clone(),
                category: format!("{:?}", app.category),
                origin: format!("{:?}", app.origin),
            })
            .collect();
        if let Err(e) = github::save_scan_cache(&cached_apps) {
            tracing::warn!("Failed to save scan cache: {e}");
        }

        let status_message = i18n::t_fmt("apps_found", &[("count", &applications.len().to_string())]);

        let sections = sections::load_sections();

        let font = default_font();

        let favorites = github::load_favorites();

        // Determine initial section: if restore_session is on use last section,
        // otherwise use default_view to pick "favorites" section if configured.
        let default_view = prefs.default_view.clone().unwrap_or_else(|| "all".into());
        let restore = prefs.restore_session.unwrap_or(true);
        let selected_section = if restore {
            prefs.selected_section.unwrap_or(0)
        } else {
            match default_view.as_str() {
                "favorites" => {
                    sections.iter().position(|s| s.is_favorites).unwrap_or(0)
                }
                _ => 0,
            }
        };
        let show_first_launch = prefs.first_launch_done != Some(true);

        // Try to restore a saved OAuth session
        let github_state = match oauth::load_saved_token() {
            Some(session) => {
                tracing::info!("Restored GitHub session for {:?}", session.username);
                GitHubState::Connected {
                    session,
                    repos: Vec::new(),
                }
            }
            None => GitHubState::Disconnected,
        };

        // If we have a saved token, kick off a repo fetch
        let startup_task = match &github_state {
            GitHubState::Connected { session, .. } => {
                let token = session.access_token.clone();
                Task::perform(
                    async move {
                        github::fetch_colony_repos(Some(&token)).await
                    },
                    |result| match result {
                        Ok(repos) => Message::GitHubReposFetched(repos),
                        Err(e) => Message::GitHubError(e.to_string()),
                    },
                )
            }
            _ => Task::none(),
        };

        let app = Self {
            applications,
            search_query: String::new(),
            sections,
            selected_section,
            status_message,
            active_colony_repo: None,
            font,
            github_state,
            show_github_menu: false,
            notifications: Vec::new(),
            next_notification_id: 0,
            download_progress: None,
            favorites,
            confirm_uninstall: None,
            show_first_launch,
            show_settings: false,
            settings_category: 0,
            selected_theme: prefs.selected_theme.clone().unwrap_or_else(|| "gruvbox".into()),
            selected_variant: prefs.selected_variant.clone().unwrap_or_else(|| "dark".into()),
            selected_accent: prefs.selected_accent.clone().unwrap_or_else(|| "blue".into()),
            auto_accent: false,
            // General
            auto_scan: prefs.auto_scan.unwrap_or(true),
            restore_session: prefs.restore_session.unwrap_or(true),
            default_view: prefs.default_view.clone().unwrap_or_else(|| "all".into()),
            language: prefs.language.clone().unwrap_or_else(|| i18n::current_lang().to_string()),
            auto_check_updates: prefs.auto_check_updates.unwrap_or(false),
            // Appearance extras
            font_size: prefs.font_size.clone().unwrap_or_else(|| "default".into()),
            animations: prefs.animations.unwrap_or(true),
            // Accessibility
            high_contrast: prefs.high_contrast.unwrap_or(false),
            text_size_a11y: prefs.text_size_a11y.clone().unwrap_or_else(|| "default".into()),
            reduce_motion: prefs.reduce_motion.unwrap_or(false),
            keyboard_nav: prefs.keyboard_nav.unwrap_or(true),
            dyslexia_font: prefs.dyslexia_font.unwrap_or(false),
            // Storage
            scan_on_startup: prefs.scan_on_startup.unwrap_or(true),
            // Async operation tracking
            is_scanning: false,
            is_downloading: false,
            is_checking_updates: false,
            is_fetching_repos: oauth::load_saved_token().is_some(),
            // Settings section state persistence
            settings_expanded_sections: HashSet::new(),
            // Detail tabs
            detail_tab: DetailTab::ReadMe,
            // Animation state
            progress_display: 0.0,
            sidebar_indicator_from: selected_section as f32 * 44.0,
            sidebar_indicator_target: selected_section as f32 * 44.0,
            sidebar_indicator_start: None,
            // Launcher self-update
            launcher_update_available: None,
            is_checking_launcher_update: false,
            launcher_update_staged: None,
        };

        set_active_theme(&app.selected_theme, &app.selected_variant);
        set_high_contrast(app.high_contrast);
        if !app.auto_accent {
            set_active_accent(accent_key_to_color(&app.selected_accent));
        }

        let launcher_check_task = if app.auto_check_updates {
            Task::done(Message::CheckLauncherUpdate)
        } else {
            Task::none()
        };

        (app, Task::batch([load_fonts(), startup_task, launcher_check_task]))
    }

    fn title(&self) -> String {
        String::from("Colony Launcher")
    }

    fn view(&self) -> Element<'_, Message> {
        let sidebar = self.view_sidebar();

        let content = if self.show_settings {
            self.view_settings_page()
        } else if self.show_github_menu {
            self.view_github_panel()
        } else {
            self.view_content()
        };

        let main_layout = row![sidebar, content].spacing(0);

        let page = container(main_layout)
            .width(Fill)
            .height(Fill);

        // Build overlay toasts (download progress + notifications) anchored to bottom-left
        let mut overlay_items: Vec<Element<'_, Message>> = Vec::new();

        // Download progress bar with graphical bar and cancel button
        if let Some((ref filename, progress)) = self.download_progress {
            let pct = (progress * 100.0) as u32;
            let bar_label = format!("\u{f019}  {} — {}%", filename, pct);
            let cancel_btn = button(
                text("\u{f00d}").size(self.sz(12)).font(self.app_font()).color(Palette::TEXT_DIMMER())
            )
            .on_press(Message::CancelDownload)
            .padding([4, 8])
            .style(|_theme, _status| button::Style {
                background: Some(iced::Color::TRANSPARENT.into()),
                ..Default::default()
            });

            // Graphical progress bar (smooth interpolation when animations enabled)
            let bar_width: f32 = 200.0;
            let display_progress = if self.animations && !self.reduce_motion {
                self.progress_display
            } else {
                progress
            };
            let filled_width = (bar_width * display_progress).max(2.0);
            let bar_filled = container(text(""))
                .width(iced::Length::Fixed(filled_width))
                .height(6)
                .style(|_theme| container::Style {
                    background: Some(Palette::ACCENT_PROGRESS().into()),
                    border: iced::Border::default().rounded(3),
                    ..Default::default()
                });
            let bar_track = container(bar_filled)
                .width(iced::Length::Fixed(bar_width))
                .height(6)
                .style(|_theme| container::Style {
                    background: Some(Palette::BG_CARD().into()),
                    border: iced::Border::default().rounded(3),
                    ..Default::default()
                });

            let progress_widget = container(
                column![
                    row![
                        text(bar_label)
                            .size(self.sz(13))
                            .font(self.app_font())
                            .color(Palette::ACCENT_PROGRESS()),
                        cancel_btn,
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
                    bar_track,
                ]
                .spacing(6),
            )
            .padding([8, 16])
            .style(|_theme| container::Style {
                background: Some(Palette::BG_PROGRESS().into()),
                border: iced::Border::default().rounded(8),
                ..Default::default()
            });
            overlay_items.push(progress_widget.into());
        }

        // Notification toasts (with fade-in / fade-out animation)
        for notif in &self.notifications {
            let dismiss_id = notif.id;
            let alpha = if self.animations && !self.reduce_motion {
                notif.opacity()
            } else {
                1.0
            };
            let text_color = state::with_alpha(notif.color(), alpha);
            let bg_color = state::with_alpha(notif.bg_color(), alpha);
            let dismiss_color = state::with_alpha(Palette::TEXT_DIMMER(), alpha);
            let primary_color = state::with_alpha(Palette::TEXT_PRIMARY(), alpha);

            let toast = button(
                row![
                    text(&notif.message)
                        .size(self.sz(13))
                        .font(self.app_font())
                        .color(text_color),
                    text("\u{f00d}")
                        .size(self.sz(12))
                        .font(self.app_font())
                        .color(dismiss_color),
                ]
                .spacing(12)
                .align_y(iced::Alignment::Center),
            )
            .on_press(Message::DismissNotification(dismiss_id))
            .padding([8, 16])
            .style(move |_theme, _status| {
                button::Style {
                    background: Some(bg_color.into()),
                    text_color: primary_color,
                    border: iced::Border::default().rounded(8),
                    ..Default::default()
                }
            });
            overlay_items.push(toast.into());
        }

        // Build the base page with overlays
        let base: Element<'_, Message> = if overlay_items.is_empty() {
            page.into()
        } else {
            let overlay = container(
                Column::with_children(overlay_items)
                    .spacing(6)
            )
            .padding(iced::Padding { top: 0.0, right: 0.0, bottom: 12.0, left: 216.0 })
            .width(Fill)
            .height(Fill)
            .align_y(iced::alignment::Vertical::Bottom)
            .align_x(iced::alignment::Horizontal::Left);

            stack![page, overlay].width(Fill).height(Fill).into()
        };

        // Uninstall confirmation overlay
        if let Some(ref repo_name) = self.confirm_uninstall {
            let confirm_msg = i18n::t_fmt("confirm_uninstall", &[("name", repo_name)]);
            let repo_clone = repo_name.clone();
            let dialog = container(
                column![
                    text(confirm_msg)
                        .size(self.sz(16))
                        .font(self.app_font())
                        .color(Palette::TEXT_PRIMARY()),
                    container(text("")).height(16),
                    row![
                        button(
                            text(i18n::t("cancel")).size(self.sz(13)).font(self.app_font())
                        )
                        .on_press(Message::CancelUninstall)
                        .padding([10, 20])
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
                        }),
                        button(
                            text(i18n::t("confirm_delete")).size(self.sz(13)).font(self.app_font())
                        )
                        .on_press(Message::UninstallColonyApp(repo_clone))
                        .padding([10, 20])
                        .style(|_theme, status| {
                            let bg = match status {
                                button::Status::Hovered => Palette::BTN_DANGER_HOVER(),
                                _ => Palette::BTN_DANGER_BG(),
                            };
                            button::Style {
                                background: Some(bg.into()),
                                text_color: Palette::ERROR_LIGHT(),
                                border: iced::Border::default().rounded(8),
                                ..Default::default()
                            }
                        }),
                    ]
                    .spacing(12)
                ]
                .padding(24),
            )
            .style(|_theme| container::Style {
                background: Some(Palette::BG_SIDEBAR().into()),
                border: iced::Border::default().rounded(12),
                ..Default::default()
            });

            let backdrop = container(
                container(dialog)
                    .center_x(Fill)
                    .center_y(Fill)
                    .width(Fill)
                    .height(Fill),
            )
            .width(Fill)
            .height(Fill)
            .style(|_theme| container::Style {
                background: Some(iced::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.5 }.into()),
                ..Default::default()
            });

            return stack![base, backdrop].width(Fill).height(Fill).into();
        }

        // First-launch welcome overlay
        if self.show_first_launch {
            let welcome = container(
                column![
                    text(i18n::t("welcome_title"))
                        .size(self.sz(24))
                        .font(self.app_font_with_weight(Weight::Bold))
                        .color(Palette::TEXT_PRIMARY()),
                    container(text("")).height(12),
                    text(i18n::t("welcome_desc"))
                        .size(self.sz(14))
                        .font(self.app_font())
                        .color(Palette::TEXT_SECONDARY()),
                    container(text("")).height(8),
                    text(i18n::t("welcome_tip_1"))
                        .size(self.sz(13))
                        .font(self.app_font())
                        .color(Palette::TEXT_MUTED()),
                    text(i18n::t("welcome_tip_2"))
                        .size(self.sz(13))
                        .font(self.app_font())
                        .color(Palette::TEXT_MUTED()),
                    text(i18n::t("welcome_tip_3"))
                        .size(self.sz(13))
                        .font(self.app_font())
                        .color(Palette::TEXT_MUTED()),
                    container(text("")).height(16),
                    button(
                        text(i18n::t("welcome_start")).size(self.sz(14)).font(self.app_font())
                    )
                    .on_press(Message::DismissFirstLaunch)
                    .padding([12, 24])
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
                    }),
                ]
                .spacing(6)
                .padding(32),
            )
            .style(|_theme| container::Style {
                background: Some(Palette::BG_SIDEBAR().into()),
                border: iced::Border::default().rounded(16),
                ..Default::default()
            });

            let backdrop = container(
                container(welcome)
                    .center_x(Fill)
                    .center_y(Fill)
                    .width(Fill)
                    .height(Fill),
            )
            .width(Fill)
            .height(Fill)
            .style(|_theme| container::Style {
                background: Some(iced::Color { r: 0.0, g: 0.0, b: 0.0, a: 0.6 }.into()),
                ..Default::default()
            });

            return stack![base, backdrop].width(Fill).height(Fill).into();
        }

        base
    }

    fn subscription(&self) -> Subscription<Message> {
        let keyboard = keyboard::listen().map(Message::KeyboardEvent);
        if self.has_active_animations() {
            let tick = iced::time::every(std::time::Duration::from_millis(16))
                .map(|_| Message::AnimationTick);
            Subscription::batch([keyboard, tick])
        } else {
            keyboard
        }
    }

    fn theme(&self) -> Theme {
        use ui::theme::active_palette;
        let bg = active_palette().bg_primary;
        let luma = bg.r * 0.299 + bg.g * 0.587 + bg.b * 0.114;
        if luma > 0.5 { Theme::Light } else { Theme::Dark }
    }
}
