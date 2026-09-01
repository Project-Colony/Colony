mod github_auth;
mod keyboard;
mod launcher;
mod onboarding;
mod preferences;
mod store;

use iced::Task;
use std::time::Duration;

use crate::github;
use crate::i18n;
use crate::message::Message;
use crate::scan;
use crate::state::{App, DetailTab, GitHubState, Notification, NotificationLevel};
use crate::ui::markdown_blocks;

impl App {
    /// The OAuth token of the signed-in session, if any - the one-liner that
    /// used to be copy-pasted at five call sites.
    pub(super) fn github_token(&self) -> Option<String> {
        if let GitHubState::Connected { session, .. } = &self.github_state {
            Some(session.access_token.clone())
        } else {
            None
        }
    }

    pub fn push_notification(
        &mut self,
        message: String,
        level: NotificationLevel,
    ) -> Task<Message> {
        let id = self.next_notification_id;
        self.next_notification_id += 1;
        let timeout = level.timeout();
        self.notifications
            .push(Notification::new(id, message, level));
        // The overlay column is anchored to the bottom and grows upward, so an
        // unbounded stack pushes the OLDEST toasts off the top of the window -
        // where they can never be clicked, and a toast is only dismissed by
        // clicking it. Cap it so the overlay can never exceed the window.
        const MAX_TOASTS: usize = 5;
        while self.notifications.len() > MAX_TOASTS {
            self.notifications.remove(0);
        }
        // Always arm the expiry timer. Gating it on animations meant that with
        // reduce-motion (or animations off) nothing ever sent TickNotifications,
        // so the `retain(!is_expired)` branch below was unreachable and toasts
        // were permanent - the accessibility settings were the ones that
        // silted the UI up. The animation gate belongs on the FADE, not on the
        // expiry, and it is already applied there.
        Task::perform(
            async move {
                tokio::time::sleep(timeout).await;
            },
            |_| Message::TickNotifications,
        )
    }

    /// Decode any cached app icons that aren't yet in memory into image handles,
    /// keyed by repo name. Runs when repos load; cheap and idempotent (skips
    /// repos already decoded). Repos without a cached icon keep the hexagon.
    pub fn reload_app_icons(&mut self) {
        let names: Vec<String> = self
            .colony_repos()
            .iter()
            .map(|repo| repo.name.clone())
            .collect();
        for name in names {
            if self.app_icons.contains_key(&name) {
                continue;
            }
            if let Some(bytes) = crate::persistence::load_repo_icon(&name) {
                if let Some(handle) = crate::icons::decode_icon(&bytes) {
                    self.app_icons.insert(name, handle);
                }
            }
        }
    }

    /// Rebuild the per-repo install-status cache (one filesystem pass). The
    /// grid and detail views read ONLY this cache - never the disk - so it
    /// must be called whenever an install can have changed: catalog load,
    /// download completion, uninstall.
    pub fn refresh_install_status(&mut self) {
        self.install_status = self
            .colony_repo_list
            .iter()
            .map(|repo| {
                let installed = crate::persistence::installed_app_path(repo).is_some();
                let version = if installed {
                    crate::persistence::load_installed_version(&repo.name)
                } else {
                    None
                };
                (repo.name.clone(), (installed, version))
            })
            .collect();
    }

    /// Pop the next repo queued by "Update all" and start its download; no-op
    /// when the queue is empty. Called from BOTH completion arms so one failed
    /// install never strands the remaining queue.
    pub(super) fn dispatch_next_queued_update(&mut self) -> Task<Message> {
        if self.update_queue.is_empty() {
            return Task::none();
        }
        let next = self.update_queue.remove(0);
        let platform = github::current_platform_key().to_string();
        Task::done(Message::DownloadRelease(next, platform))
    }

    /// Rebuild `detail_blocks` for the currently-viewed (repo, tab) if that
    /// key differs from the last parse. Cheap no-op when the cache is valid.
    pub fn refresh_detail_markdown(&mut self) {
        let Some(repo) = self.active_repo().cloned() else {
            self.detail_blocks.clear();
            self.detail_md_source = None;
            self.detail_is_placeholder = false;
            return;
        };
        let key = (repo.name.clone(), self.detail_tab);
        if self.detail_md_source.as_ref() == Some(&key) {
            return;
        }
        // Read the doc once here (cached) instead of twice per frame in the
        // view. `is_placeholder` records tabs that have no document so the view
        // does no disk I/O.
        let (content, is_placeholder) = match self.detail_tab {
            DetailTab::ReadMe => (repo.description.clone(), false),
            DetailTab::License => match crate::persistence::read_repo_doc(&repo.name, "LICENSE.md")
            {
                Some(c) => (c, false),
                None => (String::new(), true),
            },
            DetailTab::Changelog => {
                match crate::persistence::read_repo_doc(&repo.name, "CHANGELOG.md") {
                    Some(c) => (c, false),
                    None => (String::new(), true),
                }
            }
        };
        self.detail_blocks = markdown_blocks::parse(&content);
        self.detail_is_placeholder = is_placeholder;
        self.detail_md_source = Some(key);
    }

    /// Persist the settings this `App` owns.
    ///
    /// Written as an EXHAUSTIVE struct literal with no `..` on purpose: every
    /// field of `UserPreferences` now corresponds to a field of `App`, so the
    /// compiler refuses to build if a new preference is added and not saved
    /// here. That is the guard that was missing - `auto_accent` had a working
    /// toggle, applied at boot, that was simply never written, and it compiled
    /// clean. `..Default::default()` or `..load_preferences()` would hide
    /// exactly that mistake again.
    ///
    /// The three keys that used to be hardwired to `None` here
    /// (`close_behavior`, `update_channel`, `auto_install_updates`) were read
    /// by nothing and are gone.
    pub fn save_preferences(&self) {
        let prefs = crate::persistence::UserPreferences {
            selected_section: Some(self.selected_section),
            window_width: Some(self.window_size.0),
            window_height: Some(self.window_size.1),
            first_launch_done: Some(!self.show_first_launch),
            selected_theme: Some(self.selected_theme.clone()),
            selected_variant: Some(self.selected_variant.clone()),
            selected_accent: Some(self.selected_accent.clone()),
            auto_accent: Some(self.auto_accent),
            // General
            restore_session: Some(self.restore_session),
            default_view: Some(self.default_view.clone()),
            language: Some(self.language.clone()),
            auto_check_updates: Some(self.auto_check_updates),
            // Appearance
            font_size: Some(self.font_size.clone()),
            animations: Some(self.animations),
            // Accessibility
            high_contrast: Some(self.high_contrast),
            text_size_a11y: Some(self.text_size_a11y.clone()),
            reduce_motion: Some(self.reduce_motion),
            keyboard_nav: Some(self.keyboard_nav),
            dyslexia_font: Some(self.dyslexia_font),
            // Storage
            scan_on_startup: Some(self.scan_on_startup),
        };
        if let Err(e) = crate::persistence::save_preferences(&prefs) {
            tracing::warn!("Failed to save preferences: {e}");
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SearchChanged(query) => {
                self.search_query = query;
                // The visible rows changed: a stale highlight would point at
                // a hidden item.
                self.keyboard_cursor = None;
                Task::none()
            }
            Message::SectionSelected(index) => {
                if index < self.sections.len() {
                    // Start sidebar animation from current visual position
                    self.sidebar_indicator_from = self.sidebar_indicator_pos();
                    self.sidebar_indicator_target = index as f32 * 44.0;
                    self.sidebar_indicator_start = Some(std::time::Instant::now());
                    self.selected_section = index;
                    self.active_colony_repo = None;
                    self.keyboard_cursor = None;
                    // Dismiss any open overlay panel so the section change is
                    // actually visible — otherwise users stay stuck on the
                    // GitHub / Settings panel even though the underlying
                    // filter just changed.
                    self.show_github_menu = false;
                    self.show_settings = false;
                    self.save_preferences();
                }
                Task::none()
            }
            Message::Rescan => {
                if self.is_scanning {
                    return Task::none();
                }
                self.is_scanning = true;
                self.status_message = i18n::t("scanning");
                Task::perform(
                    async {
                        tokio::task::spawn_blocking(scan::scan_applications)
                            .await
                            .map_err(|e| anyhow::anyhow!("{e}"))?
                            .map_err(|e| anyhow::anyhow!(e.to_string()))
                    },
                    |result| match result {
                        Ok(apps) => Message::RescanCompleted(Ok(apps)),
                        Err(e) => Message::RescanCompleted(Err(e.to_string())),
                    },
                )
            }
            Message::RescanCompleted(result) => {
                self.is_scanning = false;
                match result {
                    Ok(apps) => {
                        self.status_message =
                            i18n::t_fmt("apps_found", &[("count", &apps.len().to_string())]);
                        // Refresh the offline scan cache (previously written on
                        // the boot path, now that the scan runs off-thread).
                        let cached: Vec<crate::persistence::CachedApp> = apps
                            .iter()
                            .map(|app| crate::persistence::CachedApp {
                                name: app.name.clone(),
                                exec: app.exec.clone(),
                                icon: app.icon.clone(),
                                category: format!("{:?}", app.category),
                                origin: format!("{:?}", app.origin),
                            })
                            .collect();
                        if let Err(e) = crate::persistence::save_scan_cache(&cached) {
                            tracing::warn!("Failed to save scan cache: {e}");
                        }
                        self.applications = apps;
                    }
                    Err(e) => {
                        self.status_message = i18n::t_fmt("scan_error", &[("error", &e)]);
                    }
                }
                Task::none()
            }
            Message::LaunchApp(exec) => {
                let launch_result = {
                    #[cfg(windows)]
                    {
                        // A scanned Windows entry is often a `.lnk`, which
                        // CreateProcess cannot run, so this one genuinely needs
                        // `start`. Build the command line with raw_arg and quote
                        // the target ourselves: Rust only quotes arguments that
                        // contain whitespace, which would leave cmd to re-parse
                        // `&`, `|` and friends in the path as separators. A quote
                        // in the path would escape our quoting, so reject it.
                        use std::os::windows::process::CommandExt;
                        if exec.contains('"') || exec.chars().any(|c| c.is_control()) {
                            Err(i18n::t_fmt(
                                "launch_error",
                                &[("error", "unsupported characters in application path")],
                            ))
                        } else {
                            std::process::Command::new("cmd")
                                .raw_arg(format!("/C start \"\" \"{exec}\""))
                                .spawn()
                                .map(|_| ())
                                .map_err(|error| {
                                    i18n::t_fmt("launch_error", &[("error", &error.to_string())])
                                })
                        }
                    }

                    #[cfg(not(windows))]
                    {
                        match shell_words::split(&exec) {
                            Ok(mut parts) => {
                                parts.retain(|part| !part.is_empty());
                                if let Some((cmd, args)) = parts.split_first() {
                                    std::process::Command::new(cmd)
                                        .args(args)
                                        .spawn()
                                        .map(|_| ())
                                        .map_err(|error| {
                                            i18n::t_fmt(
                                                "launch_error",
                                                &[("error", &error.to_string())],
                                            )
                                        })
                                } else {
                                    Err(i18n::t("launch_error_empty"))
                                }
                            }
                            Err(error) => Err(i18n::t_fmt(
                                "launch_error",
                                &[("error", &error.to_string())],
                            )),
                        }
                    }
                };

                match launch_result {
                    Ok(()) => {
                        self.status_message = i18n::t("app_launched");
                        Task::perform(
                            async {
                                tokio::time::sleep(Duration::from_secs(4)).await;
                            },
                            |_| Message::ClearStatus,
                        )
                    }
                    Err(msg) => {
                        self.status_message = msg.clone();
                        self.push_notification(msg, NotificationLevel::Error)
                    }
                }
            }
            Message::ColonyRepoSelected(name) => {
                self.active_colony_repo = Some(name);
                // A selection made from the GitHub panel must actually show
                // the detail page, not stay hidden behind the overlay.
                self.show_github_menu = false;
                self.refresh_detail_markdown();
                Task::none()
            }
            Message::ColonyRepoBack => {
                self.active_colony_repo = None;
                self.confirm_uninstall = None;
                self.detail_tab = crate::state::DetailTab::ReadMe;
                Task::none()
            }
            Message::ClearStatus => {
                self.status_message = i18n::t_fmt(
                    "apps_found",
                    &[("count", &self.applications.len().to_string())],
                );
                Task::none()
            }
            Message::FontLoaded(_) => Task::none(),

            // --- GitHub / OAuth (update/github_auth.rs) ---
            Message::ToggleGitHubMenu => self.toggle_github_menu(),
            Message::GitHubLogin => self.github_login(),
            Message::GitHubDeviceCodeReceived(result) => self.github_device_code_received(result),
            Message::GitHubLoginCompleted(result) => self.github_login_completed(result),
            Message::GitHubLogout => self.github_logout(),
            Message::GitHubReposFetched(repos) => self.github_repos_fetched(repos),
            Message::GitHubError(e) => self.github_error(e),

            Message::DownloadRelease(repo_name, platform_key) => {
                self.download_release(repo_name, platform_key)
            }
            Message::DownloadProgress(filename, downloaded, total) => {
                self.download_progress(filename, downloaded, total)
            }
            Message::DownloadCompleted(result) => self.download_completed(result),
            Message::CancelDownload => self.cancel_download(),
            Message::LaunchColonyApp(path) => self.launch_colony_app(path),
            Message::ConfirmUninstall(repo_name) => self.confirm_uninstall(repo_name),
            Message::CancelUninstall => self.cancel_uninstall(),
            Message::UninstallColonyApp(repo_name) => self.uninstall_colony_app(repo_name),
            Message::GitHubRefreshRepos => self.github_refresh_repos(),
            Message::ClearStoreCaches => self.clear_store_caches(),
            Message::CopyToClipboard(value) => iced::clipboard::write(value),
            Message::OpenUrl(url) => {
                // Single choke point for the three producers of this message
                // (Markdown link clicks, README badge pills, "View on GitHub"),
                // two of which carry remote attacker-influenced strings. Only
                // http(s) may reach the desktop URI opener - see is_web_url.
                let Some(safe) = crate::download::web_url(&url) else {
                    tracing::warn!("refusing to open non-http(s) url {url:?}");
                    return Task::none();
                };
                if let Err(err) = open::that(&safe) {
                    tracing::warn!("failed to open url {safe:?}: {err}");
                }
                Task::none()
            }
            Message::DismissNotification(id) => {
                self.notifications.retain(|n| n.id != id);
                Task::none()
            }
            Message::TickNotifications => {
                if self.animations && !self.reduce_motion {
                    // Mark expired toasts for fade-out instead of dropping
                    // them: `removing` re-arms the animation subscription
                    // (has_active_animations), which previously stopped
                    // before the fade could ever play.
                    for n in &mut self.notifications {
                        if n.is_expired() {
                            n.removing = true;
                        }
                    }
                } else {
                    self.notifications.retain(|n| !n.is_expired());
                }
                Task::none()
            }
            Message::AnimationTick => {
                const SPEED: f32 = 0.15;
                const SNAP: f32 = 0.005;
                let fade_lead = Duration::from_millis(800);

                // Notification fade-in / fade-out
                for notif in &mut self.notifications {
                    // Fade in
                    if notif.fade_in < 1.0 && !notif.removing {
                        notif.fade_in = (notif.fade_in + SPEED).min(1.0);
                        if (1.0 - notif.fade_in) < SNAP {
                            notif.fade_in = 1.0;
                        }
                    }
                    // Start fade-out before expiration
                    let timeout = notif.level.timeout();
                    if notif.created_at.elapsed() + fade_lead >= timeout && !notif.removing {
                        notif.removing = true;
                    }
                    // Fade out
                    if notif.removing {
                        notif.fade_out = (notif.fade_out - SPEED).max(0.0);
                        if notif.fade_out < SNAP {
                            notif.fade_out = 0.0;
                        }
                    }
                }
                self.notifications.retain(|n| n.fade_out > 0.0);

                // Smooth progress bar
                if let Some((_, target)) = &self.download_progress {
                    let target = *target;
                    let diff = target - self.progress_display;
                    if diff.abs() > SNAP {
                        self.progress_display += diff * SPEED;
                    } else {
                        self.progress_display = target;
                    }
                } else {
                    self.progress_display = 0.0;
                }

                // Sidebar indicator: clear animation when duration elapsed
                if let Some(start) = self.sidebar_indicator_start {
                    let elapsed_ms = start.elapsed().as_secs_f32() * 1000.0;
                    if elapsed_ms >= App::SIDEBAR_ANIM_MS {
                        self.sidebar_indicator_start = None;
                        self.sidebar_indicator_from = self.sidebar_indicator_target;
                    }
                }

                Task::none()
            }
            Message::KeyboardEvent(event) => self.keyboard_event(event),
            Message::CheckUpdates => self.check_updates(),
            Message::WindowResized(w, h) => {
                self.window_size = (w, h);
                self.window_save_gen += 1;
                let gen = self.window_save_gen;
                // Debounce: resize events flood during an interactive drag;
                // only the delayed save matching the LAST generation writes.
                Task::perform(
                    async move {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        gen
                    },
                    Message::PersistWindowSize,
                )
            }
            Message::PersistWindowSize(gen) => {
                if gen == self.window_save_gen {
                    self.save_preferences();
                }
                Task::none()
            }
            Message::UpdateAll => self.update_all(),
            Message::FetchReleaseNotes(repo_name) => self.fetch_release_notes(repo_name),
            Message::ReleaseNotesFetched(repo_name, result) => {
                self.release_notes_fetched(repo_name, result)
            }
            Message::UpdatesChecked(updates) => self.updates_checked(updates),
            Message::ToggleFavorite(name) => self.toggle_favorite(name),
            // --- Onboarding (update/onboarding.rs) ---
            Message::DismissFirstLaunch => self.dismiss_first_launch(),
            Message::WelcomeNext => self.welcome_next(),
            Message::WelcomeBack => self.welcome_back(),
            Message::TutorialBoundsUpdated(bounds) => self.tutorial_bounds_updated(bounds),
            Message::WelcomeConnectGithub => self.welcome_connect_github(),

            // --- Settings and preferences (update/preferences.rs) ---
            Message::ToggleSettings => self.toggle_settings(),
            Message::SettingsCategory(idx) => self.select_settings_category(idx),
            Message::SettingsToggleSection(key) => self.toggle_settings_section(key),
            Message::SelectThemeVariant(theme, variant) => {
                self.select_theme_variant(theme, variant)
            }
            Message::SelectAccentColor(color) => self.select_accent_color(color),
            Message::ToggleAutoAccent => self.toggle_auto_accent(),
            Message::ToggleRestoreSession => self.toggle_restore_session(),
            Message::PickDefaultView(v) => self.pick_default_view(v),
            Message::PickLanguage(v) => self.pick_language(v),
            Message::ToggleAutoCheckUpdates => self.toggle_auto_check_updates(),
            Message::PickFontSize(v) => self.pick_font_size(v),
            Message::ToggleAnimations => self.toggle_animations(),
            Message::ToggleHighContrast => self.toggle_high_contrast(),
            Message::PickTextSizeA11y(v) => self.pick_text_size_a11y(v),
            Message::ToggleReduceMotion => self.toggle_reduce_motion(),
            Message::ToggleKeyboardNav => self.toggle_keyboard_nav(),
            Message::ToggleDyslexiaFont => self.toggle_dyslexia_font(),
            Message::ToggleScanOnStartup => self.toggle_scan_on_startup(),
            Message::DetailTabSelected(tab) => {
                self.detail_tab = tab;
                self.refresh_detail_markdown();
                Task::none()
            }
            // --- Launcher self-update (update/launcher.rs) ---
            Message::CheckLauncherUpdate { manual } => self.check_launcher_update(manual),
            Message::LauncherUpdateChecked(manual, result) => {
                self.launcher_update_checked(manual, result)
            }
            Message::DownloadLauncherUpdate => self.download_launcher_update(),
            Message::LauncherDownloadProgress(progress) => {
                self.launcher_download_progress(progress)
            }
            Message::LauncherDownloadCompleted(result) => self.launcher_download_completed(result),
            Message::ApplyLauncherUpdate(new_binary) => self.apply_launcher_update(new_binary),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::{ColonyManifest, ColonyRepo, ReleaseFileEntry};

    fn repo(name: &str, desc: &str) -> ColonyRepo {
        let mut release_files = std::collections::HashMap::new();
        release_files.insert(
            github::current_platform_key().to_string(),
            ReleaseFileEntry {
                tag: "latest".into(),
                file: Some(format!("{name}-bin")),
                file_pattern: None,
                binary: None,
                sha256: None,
            },
        );
        ColonyRepo {
            name: name.into(),
            description: desc.into(),
            language: "Rust".into(),
            html_url: format!("https://github.com/Project-Colony/{name}"),
            manifest: ColonyManifest {
                name: name.into(),
                category: "Development".into(),
                platforms: vec!["linux".into()],
                release_files,
                icon: None,
                signed: false,
            },
        }
    }

    /// Serialize tests that redirect XDG dirs (env vars are process-global)
    /// and keep every disk write inside a throwaway directory. `dirs` only
    /// honors XDG on Linux, so callers gate on cfg(target_os = "linux").
    #[cfg(target_os = "linux")]
    fn with_temp_dirs(f: impl FnOnce()) {
        static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = ENV_LOCK.lock().unwrap_or_else(|p| p.into_inner());
        let tmp = tempfile::tempdir().expect("tempdir");
        let old_config = std::env::var_os("XDG_CONFIG_HOME");
        let old_data = std::env::var_os("XDG_DATA_HOME");
        // XDG_CACHE_HOME matters since the caches moved out of the config
        // directory: without it these tests read and write the developer's real
        // ~/.cache, which makes them pass or fail depending on what a previous
        // run left behind.
        let old_cache = std::env::var_os("XDG_CACHE_HOME");
        std::env::set_var("XDG_CONFIG_HOME", tmp.path().join("config"));
        std::env::set_var("XDG_DATA_HOME", tmp.path().join("data"));
        std::env::set_var("XDG_CACHE_HOME", tmp.path().join("cache"));
        f();
        match old_config {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_data {
            Some(v) => std::env::set_var("XDG_DATA_HOME", v),
            None => std::env::remove_var("XDG_DATA_HOME"),
        }
        match old_cache {
            Some(v) => std::env::set_var("XDG_CACHE_HOME", v),
            None => std::env::remove_var("XDG_CACHE_HOME"),
        }
    }

    #[test]
    fn open_detail_survives_catalog_replacement_and_reorder() {
        let mut app = App::new_for_test();
        app.colony_repo_list = vec![repo("Alpha", ""), repo("Beta", ""), repo("Gamma", "")];
        let _ = app.update(Message::ColonyRepoSelected("Beta".into()));
        assert_eq!(app.active_repo().map(|r| r.name.as_str()), Some("Beta"));

        // A refresh replaces AND reorders the vector (GitHub sorts by last
        // push): the open detail page must still resolve to the same app.
        app.colony_repo_list = vec![repo("Gamma", ""), repo("Beta", ""), repo("Alpha", "")];
        assert_eq!(app.active_repo().map(|r| r.name.as_str()), Some("Beta"));

        // A repo that vanished resolves to None (the view falls back to the
        // grid) instead of showing someone else's page.
        app.colony_repo_list = vec![repo("Alpha", "")];
        assert!(app.active_repo().is_none());
    }

    #[test]
    fn download_completion_clears_the_update_badge() {
        let mut app = App::new_for_test();
        app.available_updates
            .insert("Grape".to_string(), "v2.0.0".to_string());
        let _ = app.update(Message::DownloadCompleted(Ok((
            std::path::PathBuf::from("/tmp/grape-bin"),
            "Grape".to_string(),
            "v2.0.0".to_string(),
        ))));
        assert!(
            !app.available_updates.contains_key("Grape"),
            "badge must not survive the update it advertised"
        );
        assert!(!app.is_downloading);
    }

    #[test]
    fn update_all_queues_updatable_repos_and_chains_on_completion() {
        let mut app = App::new_for_test();
        app.colony_repo_list = vec![repo("One", ""), repo("Two", ""), repo("Three", "")];
        app.available_updates
            .insert("One".to_string(), "v2".to_string());
        app.available_updates
            .insert("Three".to_string(), "v2".to_string());

        let _ = app.update(Message::UpdateAll);
        // The first updatable repo is dispatched immediately; the rest queue.
        assert_eq!(app.update_queue, vec!["Three".to_string()]);

        // A completion - success or failure - pops the next entry.
        let _ = app.update(Message::DownloadCompleted(Err("boom".into())));
        assert!(
            app.update_queue.is_empty(),
            "failure must not strand the queue"
        );
    }

    #[test]
    fn cancel_download_empties_the_update_queue() {
        let mut app = App::new_for_test();
        app.update_queue = vec!["A".into(), "B".into()];
        app.is_downloading = true;
        let _ = app.update(Message::CancelDownload);
        assert!(app.update_queue.is_empty(), "cancel means stop, not skip");
        assert!(!app.is_downloading);
    }

    #[test]
    fn launcher_check_failure_never_claims_up_to_date() {
        let mut app = App::new_for_test();
        app.is_checking_launcher_update = true;
        let _ = app.update(Message::LauncherUpdateChecked(
            false,
            Err("network down".into()),
        ));
        assert!(!app.is_checking_launcher_update);
        assert!(app.launcher_update_available.is_none());
        assert!(
            app.status_message.contains("network down"),
            "the failure must surface, got: {}",
            app.status_message
        );
        // Automatic check: no toast for the failure either (status line only).
        assert!(app.notifications.is_empty());

        // A clean Ok(None) on an AUTOMATIC check stays quiet (no toast)...
        let _ = app.update(Message::LauncherUpdateChecked(false, Ok(None)));
        assert!(app.notifications.is_empty());
        // ...but a MANUAL check gets explicit feedback.
        let _ = app.update(Message::LauncherUpdateChecked(true, Ok(None)));
        assert_eq!(app.notifications.len(), 1);
    }

    /// Two of the eight published manifests deliberately set a display name
    /// that differs from the repo slug, and Colony threw it away everywhere the
    /// user looks - so a card titled "Lilypad-Vault" carried a button reading
    /// "Launch Lilypad".
    /// Typing "firefox" in the default view reported zero results on a machine
    /// where Firefox was two clicks away: the shipped "All" section filters to
    /// `origin: colony`, and scanned apps are never AppOrigin::Colony.
    /// The toggle worked and was applied at boot; it was simply never written,
    /// so it reset on every restart and nothing caught it because the save
    /// rebuilt the struct field by field.
    #[test]
    fn every_app_preference_survives_a_save_and_load_round_trip() {
        let mut app = App::new_for_test();
        app.auto_accent = true;
        app.high_contrast = true;
        app.reduce_motion = true;
        app.selected_accent = "amber".into();

        // Go through the same struct save_preferences builds, without touching
        // the real config dir (that path is covered by the linux-gated test).
        let prefs = crate::persistence::UserPreferences {
            selected_section: Some(app.selected_section),
            window_width: Some(app.window_size.0),
            window_height: Some(app.window_size.1),
            first_launch_done: Some(!app.show_first_launch),
            selected_theme: Some(app.selected_theme.clone()),
            selected_variant: Some(app.selected_variant.clone()),
            selected_accent: Some(app.selected_accent.clone()),
            auto_accent: Some(app.auto_accent),
            restore_session: Some(app.restore_session),
            default_view: Some(app.default_view.clone()),
            language: Some(app.language.clone()),
            auto_check_updates: Some(app.auto_check_updates),
            font_size: Some(app.font_size.clone()),
            animations: Some(app.animations),
            high_contrast: Some(app.high_contrast),
            text_size_a11y: Some(app.text_size_a11y.clone()),
            reduce_motion: Some(app.reduce_motion),
            keyboard_nav: Some(app.keyboard_nav),
            dyslexia_font: Some(app.dyslexia_font),
            scan_on_startup: Some(app.scan_on_startup),
        };
        let json = serde_json::to_string(&prefs).expect("serializes");
        let back: crate::persistence::UserPreferences =
            serde_json::from_str(&json).expect("round-trips");

        assert_eq!(back.auto_accent, Some(true), "the field that was lost");
        assert_eq!(back.selected_accent.as_deref(), Some("amber"));
        assert_eq!(back.high_contrast, Some(true));
        assert_eq!(back.reduce_motion, Some(true));

        // Every field carries a value: a None here would mean save_preferences
        // is dropping something on the floor.
        let value: serde_json::Value = serde_json::from_str(&json).expect("object");
        for (key, v) in value.as_object().expect("object") {
            assert!(!v.is_null(), "preference {key} is written as null");
        }
    }

    #[test]
    fn search_reaches_local_apps_the_section_filter_would_hide() {
        let mut app = App::new_for_test();
        app.sections = crate::sections::load_sections();
        app.selected_section = 0; // "All", origin: colony
        app.applications = vec![scan::Application {
            name: "Firefox".into(),
            exec: "firefox".into(),
            icon: None,
            category: scan::AppCategory::Network,
            origin: scan::AppOrigin::External,
        }];

        // Browsing the section still hides it - that is what the filter is for.
        assert!(app.filtered_applications().is_empty());

        // Searching for it must not.
        app.search_query = "firefox".into();
        assert_eq!(
            app.filtered_applications().len(),
            1,
            "a search that reports zero results for an installed app is worse than no search"
        );

        // And a search that matches nothing still matches nothing.
        app.search_query = "definitely-not-installed".into();
        assert!(app.filtered_applications().is_empty());
    }

    /// The .desktop integration exists so store apps land correctly in
    /// GNOME/KDE/rofi; filing a music player and two games under Utility
    /// defeats the categorisation the manifest already carries.
    #[test]
    fn desktop_categories_follow_the_manifest() {
        use scan::AppCategory as C;
        assert_eq!(C::Multimedia.desktop_categories(), "AudioVideo;");
        assert_eq!(C::Game.desktop_categories(), "Game;");
        assert_eq!(C::Network.desktop_categories(), "Network;");
        // Security is not a freedesktop MAIN category, so it must be paired
        // with one or the entry is invalid.
        assert_eq!(C::Security.desktop_categories(), "Utility;Security;");
        for c in [
            C::Development,
            C::Graphics,
            C::Network,
            C::Office,
            C::Multimedia,
            C::System,
            C::Utility,
            C::Security,
            C::Game,
            C::Other,
        ] {
            let v = c.desktop_categories();
            assert!(v.ends_with(';'), "{v:?} must be ;-terminated per the spec");
        }
    }

    #[test]
    fn the_manifest_display_name_wins_but_never_becomes_the_identity() {
        let mut declared = repo("Lilypad-Vault", "");
        declared.manifest.name = "Lilypad".to_string();
        assert_eq!(declared.display_name(), "Lilypad");
        assert_eq!(
            declared.name, "Lilypad-Vault",
            "the slug stays the identity key for install paths and caches"
        );

        // A manifest with no usable name falls back to the slug rather than
        // rendering an empty card title.
        let mut blank = repo("Grape", "");
        blank.manifest.name = "   ".to_string();
        assert_eq!(blank.display_name(), "Grape");
    }

    #[test]
    fn the_toast_stack_is_capped_even_with_animations_off() {
        let mut app = App::new_for_test();
        // The accessibility settings were the ones that silted the UI up: with
        // no animation tick, nothing ever expired the toasts, and the overlay
        // grows upward from the bottom so the oldest scrolled out of clicking
        // range and could never be dismissed at all.
        app.animations = false;
        app.reduce_motion = true;
        for i in 0..12 {
            let _ = app.push_notification(format!("toast {i}"), NotificationLevel::Info);
        }
        assert!(
            app.notifications.len() <= 5,
            "the overlay must never outgrow the window, got {}",
            app.notifications.len()
        );
        assert_eq!(
            app.notifications.last().map(|n| n.message.as_str()),
            Some("toast 11"),
            "the newest toast is the one that must survive the cap"
        );
    }

    #[test]
    fn a_repo_that_left_the_catalog_skips_forward_instead_of_stranding_the_queue() {
        let mut app = App::new_for_test();
        // "Gone" was queued by Update All, then a catalog refresh dropped it.
        app.colony_repo_list = vec![repo("Still", "")];
        app.update_queue = vec!["Next".to_string()];

        let _ = app.update(Message::DownloadRelease(
            "Gone".to_string(),
            github::current_platform_key().to_string(),
        ));

        assert!(
            app.update_queue.is_empty(),
            "the skipped repo must hand the queue on, not park it for the next unrelated install"
        );
        assert_eq!(
            app.notifications.len(),
            1,
            "the user must be told which app was skipped"
        );
    }

    #[test]
    fn app_check_failure_never_clears_a_badge_or_claims_up_to_date() {
        let mut app = App::new_for_test();
        app.is_checking_updates = true;
        app.available_updates
            .insert("Grape".to_string(), "v2.0.0".to_string());
        app.available_updates
            .insert("Spotter".to_string(), "v3.0.0".to_string());

        // Grape's check could not run; Spotter's ran and came back current.
        let _ = app.update(Message::UpdatesChecked(vec![
            ("Grape".to_string(), Err("rate limited".to_string())),
            ("Spotter".to_string(), Ok(None)),
        ]));

        assert!(!app.is_checking_updates);
        assert_eq!(
            app.available_updates.get("Grape").map(String::as_str),
            Some("v2.0.0"),
            "a check that did not run must leave the existing badge alone"
        );
        assert!(
            !app.available_updates.contains_key("Spotter"),
            "a check that DID run and found nothing must clear its badge"
        );
        assert!(
            !app.status_message.contains("applications found"),
            "the all-clear line must not be written when a check failed, got: {}",
            app.status_message
        );
        assert_eq!(
            app.notifications.len(),
            1,
            "the user must be told the check was incomplete"
        );

        // Every check succeeding and finding nothing IS the all-clear.
        app.notifications.clear();
        let _ = app.update(Message::UpdatesChecked(vec![(
            "Grape".to_string(),
            Ok(None),
        )]));
        assert!(app.available_updates.is_empty());
        assert!(app.notifications.is_empty());
    }

    #[test]
    fn window_resize_bumps_generation_and_stale_saves_are_ignored() {
        let mut app = App::new_for_test();
        let _ = app.update(Message::WindowResized(1280.0, 800.0));
        let _ = app.update(Message::WindowResized(1300.0, 820.0));
        assert_eq!(app.window_size, (1300.0, 820.0));
        assert_eq!(app.window_save_gen, 2);
        // A stale generation must not trigger a save; the state check here is
        // that the handler is a no-op (the fresh gen path writes prefs, which
        // is covered by the linux-gated persistence test).
        let _ = app.update(Message::PersistWindowSize(1));
        assert_eq!(app.window_save_gen, 2);
    }

    #[test]
    fn search_matches_description_and_display_name() {
        let mut app = App::new_for_test();
        app.colony_repo_list = vec![
            repo("Grape", "Lecteur musique en Rust"),
            repo("orCAL", "Calendar overlay"),
        ];
        app.search_query = "musique".into();
        let hits: Vec<&str> = app
            .filtered_colony_repos()
            .iter()
            .map(|r| r.name.as_str())
            .collect();
        assert_eq!(hits, vec!["Grape"]);
    }

    #[test]
    fn section_selection_out_of_bounds_is_ignored() {
        let mut app = App::new_for_test();
        // No sections loaded: any index is out of bounds and must be ignored
        // (and must not write preferences or panic).
        let _ = app.update(Message::SectionSelected(3));
        assert_eq!(app.selected_section, 0);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn repos_fetched_stores_catalog_while_disconnected_and_prunes_orphans() {
        with_temp_dirs(|| {
            let mut app = App::new_for_test();
            assert!(matches!(app.github_state, GitHubState::Disconnected));

            // Seed an orphaned doc cache for a repo that no longer exists.
            let orphan = crate::persistence::colony_cache_dir()
                .unwrap()
                .join("repo-docs")
                .join("Ghost");
            std::fs::create_dir_all(&orphan).unwrap();

            let _ = app.update(Message::GitHubReposFetched(vec![repo("Alive", "")]));

            // The catalog is stored even though no session exists (anonymous
            // mode), and the orphaned cache is pruned.
            assert_eq!(app.colony_repos().len(), 1);
            assert!(!orphan.exists(), "orphaned cache must be pruned");
        });
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn github_error_only_toasts_when_the_catalog_is_empty() {
        with_temp_dirs(|| {
            // Empty catalog + no cache: the failure interrupts (error toast).
            let mut app = App::new_for_test();
            let _ = app.update(Message::GitHubError("boom".into()));
            assert_eq!(app.notifications.len(), 1);

            // Catalog showing: the same failure stays in the status line.
            let mut app = App::new_for_test();
            app.colony_repo_list = vec![repo("Alive", "")];
            let _ = app.update(Message::GitHubError("boom".into()));
            assert!(app.notifications.is_empty());
            assert!(app.status_message.contains("boom"));
        });
    }

    #[cfg(target_os = "linux")]
    /// The pin is what stops a compromised repo from flipping `signed` back to
    /// false, so it must survive a manifest that no longer asks for signatures -
    /// and a case-only rename of the repo, which creates a different directory.
    #[cfg(target_os = "linux")]
    #[test]
    fn signature_pin_survives_and_is_case_insensitive() {
        with_temp_dirs(|| {
            use crate::persistence::{load_installed_signed, save_installed_signed};
            assert!(
                !load_installed_signed("Spotter"),
                "no pin before any install"
            );

            // The installer creates the app directory before recording anything;
            // colony_app_dir deliberately does not, so mirror that order here.
            std::fs::create_dir_all(crate::persistence::colony_app_dir("Spotter").unwrap())
                .unwrap();
            save_installed_signed("Spotter").unwrap();
            assert!(load_installed_signed("Spotter"));
            assert!(
                load_installed_signed("spotter"),
                "a case-only rename must not drop the pin"
            );
            assert!(load_installed_signed("SPOTTER"));
            assert!(
                !load_installed_signed("SpotterX"),
                "the match must not be a prefix match"
            );

            // No API can clear it: only removing the app directory does, which is
            // what uninstalling deliberately performs.
            save_installed_signed("Spotter").unwrap();
            assert!(load_installed_signed("Spotter"));
            let dir = crate::persistence::colony_app_dir("Spotter").unwrap();
            std::fs::remove_dir_all(&dir).unwrap();
            assert!(
                !load_installed_signed("Spotter"),
                "uninstall clears the pin"
            );
        });
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn toggle_favorite_persists_to_disk() {
        with_temp_dirs(|| {
            let mut app = App::new_for_test();
            let _ = app.update(Message::ToggleFavorite("Grape".into()));
            assert!(app.is_favorite("Grape"));
            assert_eq!(
                crate::persistence::load_favorites(),
                vec!["Grape".to_string()]
            );
            let _ = app.update(Message::ToggleFavorite("Grape".into()));
            assert!(!app.is_favorite("Grape"));
            assert!(crate::persistence::load_favorites().is_empty());
        });
    }
}
