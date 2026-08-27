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
            std::process::exit(validate_manifest(&path, &assets));
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
                log_file_path()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "(no cache directory)".into()),
            );
            true
        }
        _ => false,
    }
}

/// Report on a `colony.json`. Returns the process exit code: 0 clean, 1 invalid,
/// 2 unreadable.
fn validate_manifest(path: &str, assets: &[String]) -> i32 {
    let raw = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("cannot read {path}: {e}");
            return 2;
        }
    };
    let manifest: github::ColonyManifest = match serde_json::from_slice(&raw) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{path} is not a valid colony.json: {e}");
            return 1;
        }
    };
    let mut errors = manifest.validation_errors();

    if assets.is_empty() {
        if manifest.release_files.is_empty() {
            println!(
                "  note: no releaseFiles - Colony auto-detects platforms from asset names \
                 following the <name>-<platform> convention. Pass your release asset names \
                 (colony validate-manifest {path} <asset>...) to check that this actually \
                 resolves; without them this is a shape check only."
            );
        }
    } else {
        errors.extend(resolution_errors(&manifest, path, assets));
    }

    if errors.is_empty() {
        println!("{path}: OK ({}, {})", manifest.name, manifest.category);
        return 0;
    }
    eprintln!("{path}: {} problem(s)", errors.len());
    for e in &errors {
        eprintln!("  - {e}");
    }
    1
}

/// Check that the manifest resolves to something installable against the asset
/// names a release actually publishes.
///
/// This is the check that catches the failure that really happens: a manifest
/// declaring only a name and a category is structurally perfect, and still
/// renders as a card with no Download button because none of its assets match
/// the `<name>-<platform>` convention auto-detection looks for.
fn resolution_errors(
    manifest: &github::ColonyManifest,
    path: &str,
    assets: &[String],
) -> Vec<String> {
    let mut errors = Vec::new();

    if manifest.release_files.is_empty() {
        // Auto-detection is the only path. `<repo>` is not in the manifest, so
        // derive it the way Colony does: from the manifest name, then from the
        // file's own directory, since app authors run this in their repo root.
        let repo_hint = std::path::Path::new(path)
            .parent()
            .and_then(|p| p.canonicalize().ok())
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| manifest.name.clone());
        let detected = github::detect_platforms_from_assets(&repo_hint, assets);
        if detected.is_empty() {
            errors.push(format!(
                "no releaseFiles, and none of the {} published asset(s) match the \
                 <name>-<platform> convention for {repo_hint:?} (expected e.g. \
                 {}-linux). The app will list in Colony with no Download button. \
                 Declare releaseFiles explicitly, or rename the assets.",
                assets.len(),
                repo_hint.to_lowercase()
            ));
        } else {
            println!("  auto-detected platforms: {}", detected.join(", "));
        }
        return errors;
    }

    for (platform, entry) in &manifest.release_files {
        let at = format!("releaseFiles.{platform}");
        if let Some(file) = &entry.file {
            if !assets.iter().any(|a| a == file) {
                errors.push(format!(
                    "`{at}.file` is {file:?}, which this release does not publish"
                ));
            }
        } else if let Some(pattern) = &entry.file_pattern {
            match github::find_asset_by_pattern(assets, pattern) {
                Ok(resolved) => println!("  {platform} -> {resolved}"),
                Err(e) => errors.push(format!("`{at}.filePattern` does not resolve: {e}")),
            }
        }
    }
    errors
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
    //
    // The default is scoped rather than a bare "info": iced_winit and wgpu are
    // chatty at info (a full pretty-printed WindowAttributes and adapter dump
    // per launch), which drowned Colony's own lines in the very file a bug
    // report is supposed to attach. Their warnings still come through.
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn,colony=info"));

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
