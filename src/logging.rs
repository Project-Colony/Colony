//! Logging: to a file in the data directory and to stderr, set up once at
//! startup, after the legacy-path migration whose own lines it keeps.

/// Run `f` with its log lines held in memory, and return them.
///
/// The startup migration must run before anything creates a directory, the
/// log file included, yet what it reports belongs in that file.
pub fn capture(f: impl FnOnce()) -> Vec<u8> {
    #[derive(Clone, Default)]
    struct Buffer(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);
    impl std::io::Write for Buffer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let mut buffer = self.0.lock().unwrap_or_else(|e| e.into_inner());
            buffer.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let buffer = Buffer::default();
    let writer = buffer.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || writer.clone())
        .with_ansi(false)
        .finish();
    tracing::subscriber::with_default(subscriber, f);
    let mut lines = buffer.0.lock().unwrap_or_else(|e| e.into_inner());
    std::mem::take(&mut *lines)
}

/// Start logging to stderr and to a fresh log file, which opens with `early`:
/// the lines [`capture`] held back.
pub fn init(early: &[u8]) {
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
    // A plain truncate-on-start file in the data directory, not a rolling
    // appender: one run's worth of log is what a bug report needs, and it keeps
    // the dependency set unchanged.
    use std::io::Write as _;
    use tracing_subscriber::fmt::writer::MakeWriterExt;
    let _ = std::io::stderr().write_all(early);
    let log_file = crate::persistence::colony_log_path()
        .ok()
        .and_then(|path| std::fs::File::create(path).ok());
    match log_file {
        Some(mut file) => {
            let _ = file.write_all(early);
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_writer(std::sync::Mutex::new(file).and(std::io::stderr))
                .with_ansi(false)
                .init()
        }
        None => tracing_subscriber::fmt().with_env_filter(filter).init(),
    }
}
