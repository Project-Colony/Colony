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

/// Where diagnostics land. A plain truncate-on-start file, not a rolling
/// appender: one run's worth of log is what a bug report needs, and it keeps
/// the dependency set unchanged.
fn log_file_path() -> Option<std::path::PathBuf> {
    let dir = dirs::cache_dir()?.join("colony");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("colony.log"))
}

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
        "--version" | "-V" => {
            println!("colony {}", env!("CARGO_PKG_VERSION"));
            true
        }
        "--help" | "-h" => {
            println!(
                "colony {}\nThe hub for the Colony ecosystem.\n\n\
                 Usage: colony [OPTIONS]\n\n\
                 Options:\n  \
                 -V, --version  Print the version and exit\n  \
                 -h, --help     Print this help and exit\n\n\
                 Colony takes no other arguments; everything else is configured in the app.\n\n\
                 Diagnostics are written to {} and to stderr.\n\
                 Set RUST_LOG=debug for more detail.",
                env!("CARGO_PKG_VERSION"),
                log_file_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "(no cache directory)".into()),
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

    // `EnvFilter::from_default_env().add_directive(INFO)` looked like "default
    // to info", but a bare `RUST_LOG=debug` parses to a directive that compares
    // Equal to the added one, so add_directive REPLACED it and the user's
    // request was silently discarded. Build the default only when RUST_LOG is
    // absent or unparseable, and otherwise honour it verbatim.
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    // Log to a file as well as stderr: a .desktop launch has no terminal, and
    // on Windows there is no console at all, so stderr-only meant that every
    // warning in the codebase reached nobody in any shipped configuration.
    use tracing_subscriber::fmt::writer::MakeWriterExt;
    match log_file_path().and_then(|p| std::fs::File::create(p).ok()) {
        Some(file) => tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(std::sync::Mutex::new(file).and(std::io::stderr))
            .with_ansi(false)
            .init(),
        None => tracing_subscriber::fmt().with_env_filter(filter).init(),
    }

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
