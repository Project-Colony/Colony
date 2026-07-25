#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

//! Crate root: module declarations and process entry point. The `App` itself
//! (boot, view, subscription, theme) lives in `app.rs`.

mod app;
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

use state::{default_font, App};

pub fn main() -> iced::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    // Honor the saved language preference over environment locale detection,
    // and reopen at the last persisted window size (clamped to sanity).
    let prefs = crate::persistence::load_preferences();
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
