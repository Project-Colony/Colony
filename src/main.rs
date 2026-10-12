#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

//! Crate root: module declarations and process entry point. The `App` itself
//! (boot, view, subscription, theme) lives in `app.rs`.

mod app;
mod config;
mod download;
mod github;
mod i18n;
mod icons;
mod logging;
mod manifest;
mod message;
mod oauth;
mod persistence;
mod scan;
mod sections;
mod signing;
mod state;
mod ui;
mod update;

use state::App;

/// Answer `--version` / `--help` without opening a window. The bug template
/// makes `colony --version` a required field, and on Windows
/// `windows_subsystem = "windows"` means stderr goes nowhere, so the GUI was
/// the only possible answer to either flag.
///
/// Returns true when the process should exit without starting the UI.
fn handle_cli_flags() -> bool {
    let Some(arg) = std::env::args().nth(1) else {
        return false;
    };
    match arg.as_str() {
        // Lets an app author check their own manifest in their own CI, which is
        // the only place that can catch the mistake before users see a card
        // with no Download button. Decentralised on purpose: each repo
        // validates itself, there is no central registry to register with.
        "validate-manifest" => {
            let mut rest = std::env::args().skip(2);
            let path = rest.next().unwrap_or_else(|| "colony.json".to_string());
            // Everything after the path is a release asset name. Passing the
            // assets being published is what turns a shape check into a real
            // one - a structurally perfect manifest still resolves to nothing
            // if the release names its files differently.
            let assets: Vec<String> = rest.collect();
            std::process::exit(manifest::validate(&path, &assets));
        }
        "--version" | "-V" => {
            println!("colony {}", env!("CARGO_PKG_VERSION"));
            true
        }
        "--help" | "-h" => {
            println!(
                "colony {}\nThe hub for the Colony ecosystem.\n\n\
                 Usage: colony [OPTIONS] [COMMAND]\n\n\
                 Options:\n  \
                 -V, --version  Print the version and exit\n  \
                 -h, --help     Print this help and exit\n\n\
                 Commands:\n  \
                 validate-manifest [PATH] [ASSET...]\n      \
                 Check a colony.json (default: ./colony.json). Pass the release\n      \
                 asset names to also verify each platform actually resolves.\n\n\
                 With no arguments, Colony opens its window.\n\n\
                 Diagnostics are written to {} and to stderr.\n\
                 Set RUST_LOG=debug for more detail.",
                env!("CARGO_PKG_VERSION"),
                // `locate`, not the creating helper: printing help must not
                // bring a directory into existence before the migration runs.
                colony_ui::paths::locate::data_dir("Colony")
                    .map(|dir| dir.join("colony.log").display().to_string())
                    .unwrap_or_else(|_| "(no data directory)".into()),
            );
            true
        }
        _ => false,
    }
}

pub fn main() -> iced::Result {
    if handle_cli_flags() {
        return Ok(());
    }

    // Earlier versions wrote to different directories; move them before
    // anything creates a path. That includes the log file: on Windows its data
    // directory is the very config directory the first move targets, and a
    // move skips a target that already exists. No-op once done, and on a fresh
    // install.
    let migration_log = logging::capture(persistence::migrate_legacy_paths);
    logging::init(&migration_log);

    // Honor the saved language preference over environment locale detection,
    // and reopen at the last persisted window size (clamped to sanity).
    let prefs = crate::persistence::load_preferences();
    i18n::init(prefs.language.clone());
    let width = prefs.window_width.unwrap_or(1000.0).clamp(640.0, 7680.0);
    let height = prefs.window_height.unwrap_or(700.0).clamp(480.0, 4320.0);

    // The interface, dyslexia and icon fonts ship inside colony-ui, licences
    // included.
    colony_ui::fonts::BYTES
        .into_iter()
        .fold(
            iced::application(App::boot, App::update, App::view),
            |app, font| app.font(font),
        )
        .title(App::title)
        .theme(App::theme)
        .subscription(App::subscription)
        .default_font(colony_ui::ui_font())
        .window_size((width, height))
        .run()
}
