#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod config;
mod download;
mod github;
mod i18n;
mod icons;
mod message;
mod oauth;
mod persistence;
mod scan;
mod sections;
mod signing;
mod state;
mod ui;
mod update;

use iced::font;
use iced::keyboard;
use iced::widget::{button, column, container, mouse_area, opaque, row, stack, text, Column};
use iced::{Element, Fill, Subscription, Task, Theme};
use std::collections::HashSet;
use ui::theme::{
    accent_key_to_color, set_active_accent, set_active_theme, set_high_contrast, Palette,
};

use message::Message;
use state::{
    default_font, App, DetailTab, GitHubState, APP_FONT_BYTES, DYSLEXIA_FONT_BYTES, FA_FONT_BYTES,
};

pub fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // Honor the saved language preference over environment locale detection,
    // and reopen at the last persisted window size (clamped to sanity).
    let prefs = github::load_preferences();
    i18n::init(prefs.language.clone());
    let width = prefs.window_width.unwrap_or(1000.0).clamp(640.0, 7680.0);
    let height = prefs.window_height.unwrap_or(700.0).clamp(480.0, 4320.0);

    iced::application(App::boot, App::update, App::view)
        .title(App::title)
        .theme(App::theme)
        .subscription(App::subscription)
        .default_font(default_font())
        .window_size((width, height))
        .run()
}

fn load_fonts() -> Task<Message> {
    let main_fonts = APP_FONT_BYTES
        .iter()
        .map(|data| font::load(data.to_vec()).map(Message::FontLoaded));
    let dyslexia_font =
        std::iter::once(font::load(DYSLEXIA_FONT_BYTES.to_vec()).map(Message::FontLoaded));
    let fa_fonts = FA_FONT_BYTES
        .iter()
        .map(|data| font::load(data.to_vec()).map(Message::FontLoaded));
    Task::batch(main_fonts.chain(dyslexia_font).chain(fa_fonts))
}

impl App {
    fn boot() -> (Self, Task<Message>) {
        let prefs = github::load_preferences();

        // The filesystem application scan and its cache write are deferred off
        // the boot path (dispatched as a Rescan task below) so the window
        // appears immediately instead of after a recursive directory walk.
        let should_scan = prefs.scan_on_startup.unwrap_or(true);
        let applications: Vec<scan::Application> = if should_scan {
            Vec::new()
        } else {
            // The startup scan is disabled: restore the last scan from cache
            // (which was written on every scan but never read back) instead of
            // greeting the user with a permanently empty local-apps grid.
            github::load_scan_cache()
                .unwrap_or_default()
                .into_iter()
                .map(|c| scan::Application {
                    name: c.name,
                    exec: c.exec,
                    icon: c.icon,
                    category: scan::AppCategory::from_name(&c.category),
                    origin: match c.origin.as_str() {
                        "Windows" => scan::AppOrigin::Windows,
                        "Colony" => scan::AppOrigin::Colony,
                        "Linux" => scan::AppOrigin::Linux,
                        _ => scan::AppOrigin::External,
                    },
                })
                .collect()
        };
        let status_message = if should_scan {
            i18n::t("scanning")
        } else {
            i18n::t_fmt("apps_found", &[("count", &applications.len().to_string())])
        };

        let sections = sections::load_sections();

        let font = default_font();

        let favorites = github::load_favorites();

        // Determine initial section: if restore_session is on use last section,
        // otherwise use default_view to pick "favorites" section if configured.
        let default_view = prefs.default_view.clone().unwrap_or_else(|| "all".into());
        let restore = prefs.restore_session.unwrap_or(true);
        let selected_section = if restore {
            // Clamp against the LOADED sections: a categories.json override
            // that shrank the list must not leave a dangling index.
            prefs
                .selected_section
                .unwrap_or(0)
                .min(sections.len().saturating_sub(1))
        } else {
            match default_view.as_str() {
                "favorites" => sections.iter().position(|s| s.is_favorites).unwrap_or(0),
                _ => 0,
            }
        };
        let show_first_launch = prefs.first_launch_done != Some(true);

        // Try to restore a saved OAuth session (load the token exactly once).
        let saved_token = oauth::load_saved_token();
        let github_state = match saved_token {
            Some(session) => {
                tracing::info!("Restored GitHub session for {:?}", session.username);
                GitHubState::Connected { session }
            }
            None => GitHubState::Disconnected,
        };

        // The catalog is public data: show the on-disk cache instantly, then
        // refresh over the network - anonymously when no token is saved (the
        // unauthenticated GitHub API allows 60 req/h, plenty for one boot
        // fetch). Signing in is optional, exactly as the welcome flow promises.
        let colony_repo_list = github::load_repos_cache().unwrap_or_default();
        let startup_task = {
            let token = match &github_state {
                GitHubState::Connected { session } => Some(session.access_token.clone()),
                _ => None,
            };
            Task::perform(
                async move { github::fetch_colony_repos(token.as_deref()).await },
                |result| match result {
                    Ok(repos) => Message::GitHubReposFetched(repos),
                    Err(e) => Message::GitHubError(e.to_string()),
                },
            )
        };

        let mut app = Self {
            applications,
            search_query: String::new(),
            sections,
            selected_section,
            status_message,
            active_colony_repo: None,
            font,
            github_state,
            show_github_menu: false,
            colony_repo_list,
            notifications: Vec::new(),
            next_notification_id: 0,
            app_icons: std::collections::HashMap::new(),
            download_progress: None,
            download_abort: None,
            downloading_repo: None,
            favorites,
            confirm_uninstall: None,
            show_first_launch,
            welcome_step: 0,
            tutorial_bounds: Default::default(),
            show_settings: false,
            settings_category: 0,
            selected_theme: prefs
                .selected_theme
                .clone()
                .unwrap_or_else(|| "gruvbox".into()),
            selected_variant: prefs
                .selected_variant
                .clone()
                .unwrap_or_else(|| "dark".into()),
            selected_accent: prefs
                .selected_accent
                .clone()
                .unwrap_or_else(|| "blue".into()),
            auto_accent: false,
            // General
            restore_session: prefs.restore_session.unwrap_or(true),
            default_view: prefs.default_view.clone().unwrap_or_else(|| "all".into()),
            language: prefs
                .language
                .clone()
                .unwrap_or_else(|| i18n::current_lang().to_string()),
            // ON by default: a store that never checks for updates leaves its
            // badges permanently invisible. The check is cheap (batched; zero
            // API calls for apps pinned to a fixed tag).
            auto_check_updates: prefs.auto_check_updates.unwrap_or(true),
            // Appearance extras
            font_size: prefs.font_size.clone().unwrap_or_else(|| "default".into()),
            animations: prefs.animations.unwrap_or(true),
            // Accessibility
            high_contrast: prefs.high_contrast.unwrap_or(false),
            text_size_a11y: prefs
                .text_size_a11y
                .clone()
                .unwrap_or_else(|| "default".into()),
            reduce_motion: prefs.reduce_motion.unwrap_or(false),
            keyboard_nav: prefs.keyboard_nav.unwrap_or(true),
            dyslexia_font: prefs.dyslexia_font.unwrap_or(false),
            // Storage
            scan_on_startup: prefs.scan_on_startup.unwrap_or(true),
            // Async operation tracking
            is_scanning: false,
            is_downloading: false,
            is_checking_updates: false,
            // A catalog fetch (token'd or anonymous) always starts at boot.
            is_fetching_repos: true,
            // Settings section state persistence
            settings_expanded_sections: HashSet::new(),
            // Detail tabs
            detail_tab: DetailTab::ReadMe,
            detail_blocks: Vec::new(),
            detail_md_source: None,
            detail_is_placeholder: false,
            // Animation state
            progress_display: 0.0,
            sidebar_indicator_from: selected_section as f32 * 44.0,
            sidebar_indicator_target: selected_section as f32 * 44.0,
            sidebar_indicator_start: None,
            available_updates: std::collections::HashMap::new(),
            update_queue: Vec::new(),
            release_notes: std::collections::HashMap::new(),
            fetching_notes: std::collections::HashSet::new(),
            window_size: (
                prefs.window_width.unwrap_or(1000.0).clamp(640.0, 7680.0),
                prefs.window_height.unwrap_or(700.0).clamp(480.0, 4320.0),
            ),
            window_save_gen: 0,
            // Launcher self-update
            launcher_update_available: None,
            is_checking_launcher_update: false,
            launcher_update_staged: None,
            launcher_system_managed: download::launcher_is_system_managed(),
        };

        // Cached catalog repos may have icons already on disk: decode them now
        // so the offline/pre-fetch grid is not a wall of fallback hexagons.
        app.reload_app_icons();

        set_active_theme(&app.selected_theme, &app.selected_variant);
        set_high_contrast(app.high_contrast);
        if !app.auto_accent {
            set_active_accent(accent_key_to_color(&app.selected_accent));
        }

        // No direct launcher-update check here: with auto-check on, the boot
        // catalog fetch chains GitHubReposFetched -> CheckUpdates ->
        // CheckLauncherUpdate already - a second dispatch meant every boot ran
        // the check twice.
        let launcher_check_task = Task::none();

        let tutorial_task = if app.show_first_launch {
            ui::fetch_bounds_task()
        } else {
            Task::none()
        };

        // Run the initial application scan off the boot thread.
        let scan_task = if should_scan {
            Task::done(Message::Rescan)
        } else {
            Task::none()
        };

        (
            app,
            Task::batch([
                load_fonts(),
                startup_task,
                launcher_check_task,
                tutorial_task,
                scan_task,
            ]),
        )
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

        let page = container(main_layout).width(Fill).height(Fill);

        // Build overlay toasts (download progress + notifications) anchored to bottom-left
        let mut overlay_items: Vec<Element<'_, Message>> = Vec::new();

        // Download progress bar with graphical bar and cancel button
        if let Some((ref filename, progress)) = self.download_progress {
            let pct = (progress * 100.0) as u32;
            let bar_label = format!("\u{f019}  {} — {}%", filename, pct);
            let cancel_btn = button(
                text("\u{f00d}")
                    .size(self.sz(12))
                    .font(self.app_font())
                    .color(Palette::TEXT_DIMMER()),
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
            .style(move |_theme, _status| button::Style {
                background: Some(bg_color.into()),
                text_color: primary_color,
                border: iced::Border::default().rounded(8),
                ..Default::default()
            });
            overlay_items.push(toast.into());
        }

        // Build the base page with overlays
        let base: Element<'_, Message> = if overlay_items.is_empty() {
            page.into()
        } else {
            let overlay = container(Column::with_children(overlay_items).spacing(6))
                .padding(iced::Padding {
                    top: 0.0,
                    right: 0.0,
                    bottom: 12.0,
                    left: 216.0,
                })
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
                            text(i18n::t("cancel"))
                                .size(self.sz(13))
                                .font(self.app_font())
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
                            text(i18n::t("confirm_delete"))
                                .size(self.sz(13))
                                .font(self.app_font())
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
                background: Some(
                    iced::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.5,
                    }
                    .into(),
                ),
                ..Default::default()
            });

            // Make the overlay modal: `opaque` stops clicks from reaching the
            // page underneath, and a click on the dimmed backdrop dismisses the
            // dialog (standard click-outside-to-cancel).
            let modal = opaque(mouse_area(backdrop).on_press(Message::CancelUninstall));
            return stack![base, modal].width(Fill).height(Fill).into();
        }

        // First-launch guided tutorial: real UI visible, spotlight zooms in on
        // each zone one by one (sidebar → search → grid → GitHub → finish).
        if self.show_first_launch {
            // `opaque` keeps clicks on the dimmed tutorial bands from falling
            // through to (and operating) the real UI underneath.
            let tutorial = opaque(self.view_tutorial());
            return stack![base, tutorial].width(Fill).height(Fill).into();
        }

        base
    }

    fn subscription(&self) -> Subscription<Message> {
        let keyboard = keyboard::listen().map(Message::KeyboardEvent);
        // Track live window size so it can be persisted (debounced) and the
        // next boot reopens at the same dimensions.
        let resizes = iced::event::listen_with(|event, _status, _id| match event {
            iced::Event::Window(iced::window::Event::Resized(size)) => {
                Some(Message::WindowResized(size.width, size.height))
            }
            _ => None,
        });
        if self.has_active_animations() {
            let tick = iced::time::every(std::time::Duration::from_millis(16))
                .map(|_| Message::AnimationTick);
            Subscription::batch([keyboard, resizes, tick])
        } else {
            Subscription::batch([keyboard, resizes])
        }
    }

    fn theme(&self) -> Theme {
        use ui::theme::active_palette;
        let bg = active_palette().bg_primary;
        let luma = bg.r * 0.299 + bg.g * 0.587 + bg.b * 0.114;
        if luma > 0.5 {
            Theme::Light
        } else {
            Theme::Dark
        }
    }
}
