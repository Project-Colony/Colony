use iced::Task;
use std::time::Duration;

use crate::github;
use crate::i18n;
use crate::message::Message;
use crate::oauth;
use crate::scan;
use crate::state::{App, DetailTab, GitHubState, Notification, NotificationLevel};
use crate::ui::markdown_blocks;
use crate::ui::theme::{
    accent_key_to_color, set_active_accent, set_active_theme, set_high_contrast,
};

impl App {
    pub fn push_notification(
        &mut self,
        message: String,
        level: NotificationLevel,
    ) -> Task<Message> {
        let id = self.next_notification_id;
        self.next_notification_id += 1;
        let timeout = match level {
            NotificationLevel::Error => Duration::from_secs(10),
            NotificationLevel::Warning => Duration::from_secs(7),
            NotificationLevel::Info => Duration::from_secs(5),
        };
        self.notifications
            .push(Notification::new(id, message, level));
        // When reduce_motion or animations off, don't auto-dismiss (user must click)
        if self.reduce_motion || !self.animations {
            Task::none()
        } else {
            Task::perform(
                async move {
                    tokio::time::sleep(timeout).await;
                },
                |_| Message::TickNotifications,
            )
        }
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

    /// Pop the next repo queued by "Update all" and start its download; no-op
    /// when the queue is empty. Called from BOTH completion arms so one failed
    /// install never strands the remaining queue.
    fn dispatch_next_queued_update(&mut self) -> Task<Message> {
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
            DetailTab::License => match github::read_repo_doc(&repo.name, "LICENSE.md") {
                Some(c) => (c, false),
                None => (String::new(), true),
            },
            DetailTab::Changelog => match github::read_repo_doc(&repo.name, "CHANGELOG.md") {
                Some(c) => (c, false),
                None => (String::new(), true),
            },
        };
        self.detail_blocks = markdown_blocks::parse(&content);
        self.detail_is_placeholder = is_placeholder;
        self.detail_md_source = Some(key);
    }

    pub fn save_preferences(&self) {
        let prefs = github::UserPreferences {
            selected_section: Some(self.selected_section),
            window_width: None,
            window_height: None,
            first_launch_done: Some(!self.show_first_launch),
            selected_theme: Some(self.selected_theme.clone()),
            selected_variant: Some(self.selected_variant.clone()),
            selected_accent: Some(self.selected_accent.clone()),
            // General
            restore_session: Some(self.restore_session),
            default_view: Some(self.default_view.clone()),
            close_behavior: None,
            language: Some(self.language.clone()),
            auto_check_updates: Some(self.auto_check_updates),
            update_channel: None,
            auto_install_updates: None,
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
        if let Err(e) = github::save_preferences(&prefs) {
            tracing::warn!("Failed to save preferences: {e}");
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SearchChanged(query) => {
                self.search_query = query;
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
                        let cached: Vec<github::CachedApp> = apps
                            .iter()
                            .map(|app| github::CachedApp {
                                name: app.name.clone(),
                                exec: app.exec.clone(),
                                icon: app.icon.clone(),
                                category: format!("{:?}", app.category),
                                origin: format!("{:?}", app.origin),
                            })
                            .collect();
                        if let Err(e) = github::save_scan_cache(&cached) {
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
                        std::process::Command::new("cmd")
                            .args(["/C", "start", "", &exec])
                            .spawn()
                            .map(|_| ())
                            .map_err(|error| {
                                i18n::t_fmt("launch_error", &[("error", &error.to_string())])
                            })
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

            // --- GitHub / OAuth ---
            Message::ToggleGitHubMenu => {
                self.show_github_menu = !self.show_github_menu;
                Task::none()
            }
            Message::GitHubLogin => {
                self.github_state = GitHubState::Connecting { user_code: None };
                Task::perform(
                    async {
                        oauth::request_device_code()
                            .await
                            .map_err(|e| e.to_string())
                    },
                    Message::GitHubDeviceCodeReceived,
                )
            }
            Message::GitHubDeviceCodeReceived(result) => match result {
                Ok(device) => {
                    self.github_state = GitHubState::Connecting {
                        user_code: Some(device.user_code.clone()),
                    };
                    Task::perform(
                        async move {
                            oauth::poll_for_token(device)
                                .await
                                .map_err(|e| e.to_string())
                        },
                        Message::GitHubLoginCompleted,
                    )
                }
                Err(e) => {
                    self.github_state = GitHubState::Error(e.clone());
                    self.status_message = i18n::t_fmt("oauth_error", &[("error", &e)]);
                    self.push_notification(
                        i18n::t_fmt("oauth_error", &[("error", &e)]),
                        NotificationLevel::Error,
                    )
                }
            },
            Message::GitHubLoginCompleted(result) => {
                match result {
                    Ok(session) => {
                        self.github_state = GitHubState::Connected {
                            session: session.clone(),
                        };
                        self.status_message = format!(
                            "{}{}",
                            i18n::t("github_connected"),
                            session
                                .username
                                .as_ref()
                                .map(|u| format!(" ({u})"))
                                .unwrap_or_default()
                        );
                        let token = session.access_token.clone();
                        // Guard the refetch like every other fetch path, so a
                        // concurrent GitHubRefreshRepos cannot double-fetch.
                        self.is_fetching_repos = true;
                        Task::perform(
                            async move { github::fetch_colony_repos(Some(&token)).await },
                            |result| match result {
                                Ok(repos) => Message::GitHubReposFetched(repos),
                                Err(e) => Message::GitHubError(e.to_string()),
                            },
                        )
                    }
                    Err(e) => {
                        self.github_state = GitHubState::Error(e.clone());
                        self.status_message = i18n::t_fmt("oauth_error", &[("error", &e)]);
                        self.push_notification(
                            i18n::t_fmt("oauth_error", &[("error", &e)]),
                            NotificationLevel::Error,
                        )
                    }
                }
            }
            Message::GitHubLogout => {
                let _ = oauth::delete_saved_token();
                self.github_state = GitHubState::Disconnected;
                self.status_message = i18n::t("github_disconnected");
                Task::none()
            }
            Message::GitHubReposFetched(repos) => {
                self.is_fetching_repos = false;
                let count = repos.len();
                if let Err(e) = github::save_repos_cache(&repos) {
                    tracing::warn!("Failed to save repos cache: {e}");
                }
                // The catalog is stored regardless of sign-in state: anonymous
                // fetches land here too.
                self.colony_repo_list = repos;
                // A successful fetch is the one moment we KNOW which repos
                // exist: drop doc/icon caches of repos that left the catalog.
                let live: Vec<String> = self
                    .colony_repo_list
                    .iter()
                    .map(|r| r.name.clone())
                    .collect();
                crate::persistence::prune_orphaned_repo_caches(&live);
                // Decode any freshly-cached app icons into image handles.
                self.reload_app_icons();
                // New docs may have landed for the repo currently viewed.
                self.detail_md_source = None;
                self.refresh_detail_markdown();
                self.status_message =
                    i18n::t_fmt("github_repos_detected", &[("count", &count.to_string())]);
                if self.auto_check_updates {
                    Task::done(Message::CheckUpdates)
                } else {
                    Task::none()
                }
            }
            Message::GitHubError(e) => {
                self.is_fetching_repos = false;
                tracing::error!(error = %e, "GitHub error");
                if self.colony_repo_list.is_empty() {
                    if let Some(cached) = github::load_repos_cache() {
                        tracing::info!("Using {} cached repos as fallback", cached.len());
                        self.colony_repo_list = cached;
                    }
                }
                // Offline fallback repos may have cached icons on disk.
                self.reload_app_icons();
                self.status_message = i18n::t_fmt("github_api_error", &[("error", &e)]);
                if self.colony_repo_list.is_empty() {
                    self.push_notification(
                        i18n::t_fmt("github_api_error", &[("error", &e)]),
                        NotificationLevel::Error,
                    )
                } else {
                    // The catalog is showing (cached or previously fetched): a
                    // toast on every offline boot would be pure noise - the
                    // status line already carries the error. Only an EMPTY
                    // catalog warrants interrupting the user.
                    Task::none()
                }
            }
            Message::DownloadRelease(repo_name, platform_key) => {
                if self.is_downloading {
                    return Task::none();
                }
                let repos = self.colony_repos();
                if let Some(repo) = repos.iter().find(|r| r.name == repo_name) {
                    if let Some(entry) = repo.manifest.release_files.get(&platform_key) {
                        let tag = entry.tag.clone();
                        let file = entry.file.clone();
                        let file_pattern = entry.file_pattern.clone();
                        let binary = entry.binary.clone();
                        let expected_sha256 = entry.sha256.clone();
                        let repo_name = repo.name.clone();
                        let token =
                            if let GitHubState::Connected { session, .. } = &self.github_state {
                                Some(session.access_token.clone())
                            } else {
                                None
                            };
                        let display_name = file
                            .as_deref()
                            .or(file_pattern.as_deref())
                            .unwrap_or(&repo.name)
                            .to_string();
                        self.status_message =
                            i18n::t_fmt("downloading", &[("file", &display_name)]);
                        self.download_progress = Some((display_name.clone(), 0.0));
                        self.is_downloading = true;
                        let dl_repo = repo_name.clone();
                        let (progress_tx, progress_rx) = futures::channel::mpsc::unbounded::<f32>();
                        let progress_name = display_name;

                        let download_task = Task::perform(
                            async move {
                                // Fetch release info if we need tag resolution or asset matching
                                let needs_release_info =
                                    tag.eq_ignore_ascii_case("latest") || file_pattern.is_some();

                                let (resolved_tag, resolved_file) = if needs_release_info {
                                    let client = github::build_update_client(token.as_deref())?;
                                    let release_info =
                                        github::fetch_release_info(&client, &repo_name, &tag)
                                            .await?;
                                    let filename = if let Some(ref f) = file {
                                        f.clone()
                                    } else if let Some(ref pattern) = file_pattern {
                                        github::find_asset_by_pattern(
                                            &release_info.asset_names,
                                            pattern,
                                        )?
                                    } else {
                                        anyhow::bail!(
                                            "colony.json: 'file' or 'filePattern' is required"
                                        );
                                    };
                                    (release_info.tag, filename)
                                } else {
                                    let f = file.ok_or_else(|| {
                                        anyhow::anyhow!(
                                            "colony.json: 'file' or 'filePattern' is required"
                                        )
                                    })?;
                                    (tag, f)
                                };

                                // The version/asset records are written by
                                // download_release_asset itself, inside the
                                // blocking install step: writing them here (or
                                // in DownloadCompleted) meant a cancel landing
                                // mid-install detached the blocking task and
                                // left an installed binary with no metadata.
                                let path = github::download_release_asset(
                                    token,
                                    crate::download::AssetInstall {
                                        repo_name: repo_name.clone(),
                                        tag: resolved_tag.clone(),
                                        filename: resolved_file.clone(),
                                        binary_name: binary,
                                        expected_sha256,
                                        record_asset: file_pattern.is_some(),
                                    },
                                    Some(progress_tx),
                                )
                                .await?;

                                Ok((path, dl_repo, resolved_tag))
                            },
                            |result: Result<_, anyhow::Error>| {
                                Message::DownloadCompleted(result.map_err(|e| e.to_string()))
                            },
                        );

                        let progress_task = Task::run(progress_rx, move |pct| {
                            Message::DownloadProgress(progress_name.clone(), pct)
                        });

                        // Keep an abort handle so CancelDownload actually stops
                        // the download and its progress stream (dropping the
                        // progress sender), instead of only clearing the UI.
                        let (task, handle) =
                            Task::batch([download_task, progress_task]).abortable();
                        self.download_abort = Some(handle);
                        return task;
                    } else {
                        self.status_message =
                            i18n::t_fmt("no_release_for", &[("platform", &platform_key)]);
                    }
                }
                Task::none()
            }
            Message::DownloadProgress(filename, progress) => {
                // Ignore late progress events from a cancelled/finished download
                // so the toast cannot resurrect after CancelDownload.
                if self.is_downloading {
                    self.download_progress = Some((filename, progress));
                }
                Task::none()
            }
            Message::DownloadCompleted(result) => {
                self.download_progress = None;
                self.is_downloading = false;
                self.download_abort = None;
                match result {
                    // Version/asset records were written atomically with the
                    // install (inside download_release_asset), so the tag is
                    // no longer needed here.
                    Ok((path, repo_name, _tag)) => {
                        // The just-installed version IS the one the badge was
                        // advertising: clear it, or the card keeps showing
                        // "Update vX -> vX" until the next global check.
                        self.available_updates.remove(&repo_name);
                        let display_name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| path.display().to_string());
                        // Use the short binary name (not the full install path)
                        // so the header status text can't squeeze the search box.
                        self.status_message = i18n::t_fmt("installed", &[("path", &display_name)]);
                        let notif = self.push_notification(
                            i18n::t_fmt("installed", &[("path", &display_name)]),
                            NotificationLevel::Info,
                        );
                        Task::batch([notif, self.dispatch_next_queued_update()])
                    }
                    Err(e) => {
                        self.status_message = i18n::t_fmt("download_error", &[("error", &e)]);
                        let notif = self.push_notification(
                            i18n::t_fmt("download_error", &[("error", &e)]),
                            NotificationLevel::Error,
                        );
                        // A failed item does not strand the rest of the queue.
                        Task::batch([notif, self.dispatch_next_queued_update()])
                    }
                }
            }
            Message::CancelDownload => {
                // Actually abort the running download + progress tasks so no
                // phantom install completes and no second writer can race the
                // same file on a retry. Cancel also empties the "Update all"
                // queue: cancelling means stop, not "skip this one".
                self.update_queue.clear();
                if let Some(handle) = self.download_abort.take() {
                    handle.abort();
                }
                self.download_progress = None;
                self.is_downloading = false;
                self.status_message = i18n::t("download_cancelled");
                self.push_notification(i18n::t("download_cancelled"), NotificationLevel::Warning)
            }
            Message::LaunchColonyApp(path) => {
                #[cfg(windows)]
                let result = std::process::Command::new("cmd")
                    .args(["/C", "start", "", &path.display().to_string()])
                    .spawn()
                    .map(|_| ());
                #[cfg(not(windows))]
                let result = std::process::Command::new(&path).spawn().map(|_| ());

                match result {
                    Ok(()) => {
                        self.status_message = i18n::t("app_launched");
                        Task::perform(
                            async {
                                tokio::time::sleep(Duration::from_secs(4)).await;
                            },
                            |_| Message::ClearStatus,
                        )
                    }
                    Err(e) => {
                        let msg = i18n::t_fmt("launch_error_msg", &[("error", &e.to_string())]);
                        self.status_message = msg.clone();
                        self.push_notification(msg, NotificationLevel::Error)
                    }
                }
            }
            Message::ConfirmUninstall(repo_name) => {
                self.confirm_uninstall = Some(repo_name);
                Task::none()
            }
            Message::CancelUninstall => {
                self.confirm_uninstall = None;
                Task::none()
            }
            Message::UninstallColonyApp(repo_name) => {
                self.confirm_uninstall = None;
                // An uninstalled app has no meaningful "update available".
                self.available_updates.remove(&repo_name);
                // Stale notes describe the version that was just removed.
                // (Doc/icon caches and the favorite deliberately survive: they
                // belong to the CATALOG entry, which is still listed - orphan
                // cleanup happens on catalog refresh instead.)
                self.release_notes.remove(&repo_name);
                match github::colony_apps_dir() {
                    Ok(apps_dir) => {
                        let app_dir = apps_dir.join(&repo_name);
                        if app_dir.exists() {
                            if let Err(e) = std::fs::remove_dir_all(&app_dir) {
                                self.status_message =
                                    i18n::t_fmt("uninstall_error", &[("error", &e.to_string())]);
                            } else {
                                self.status_message =
                                    i18n::t_fmt("uninstalled", &[("name", &repo_name)]);
                                return Task::perform(
                                    async {
                                        tokio::time::sleep(Duration::from_secs(4)).await;
                                    },
                                    |_| Message::ClearStatus,
                                );
                            }
                        }
                    }
                    Err(e) => {
                        self.status_message =
                            i18n::t_fmt("scan_error", &[("error", &e.to_string())]);
                    }
                }
                Task::none()
            }
            Message::GitHubRefreshRepos => {
                if self.is_fetching_repos {
                    return Task::none();
                }
                self.is_fetching_repos = true;
                // Anonymous refresh is supported: the token only raises the
                // rate limit (60 req/h unauthenticated vs 5000 signed-in).
                let token = if let GitHubState::Connected { session } = &self.github_state {
                    Some(session.access_token.clone())
                } else {
                    None
                };
                Task::perform(
                    async move { github::fetch_colony_repos(token.as_deref()).await },
                    |result| match result {
                        Ok(repos) => Message::GitHubReposFetched(repos),
                        Err(e) => Message::GitHubError(e.to_string()),
                    },
                )
            }
            Message::CopyToClipboard(value) => iced::clipboard::write(value),
            Message::OpenUrl(url) => {
                if let Err(err) = open::that(&url) {
                    tracing::warn!("failed to open url {url:?}: {err}");
                }
                Task::none()
            }
            Message::DismissNotification(id) => {
                self.notifications.retain(|n| n.id != id);
                Task::none()
            }
            Message::TickNotifications => {
                self.notifications.retain(|n| !n.is_expired());
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
                    let timeout = match notif.level {
                        NotificationLevel::Error => Duration::from_secs(10),
                        NotificationLevel::Warning => Duration::from_secs(7),
                        NotificationLevel::Info => Duration::from_secs(5),
                    };
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
            Message::KeyboardEvent(event) => {
                if !self.keyboard_nav {
                    return Task::none();
                }
                if let iced::keyboard::Event::KeyPressed { key, modifiers, .. } = event {
                    match key {
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) => {
                            if self.show_settings {
                                self.show_settings = false;
                            } else if self.confirm_uninstall.is_some() {
                                self.confirm_uninstall = None;
                            } else if self.show_first_launch {
                                self.show_first_launch = false;
                                self.save_preferences();
                            } else if self.active_colony_repo.is_some() {
                                self.active_colony_repo = None;
                            } else if self.show_github_menu {
                                self.show_github_menu = false;
                            }
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab)
                            if !self.show_settings
                                && !self.show_github_menu
                                && !self.show_first_launch =>
                        {
                            let len = self.sections.len();
                            if len > 0 {
                                self.sidebar_indicator_from = self.sidebar_indicator_pos();
                                if modifiers.shift() {
                                    self.selected_section = if self.selected_section == 0 {
                                        len - 1
                                    } else {
                                        self.selected_section - 1
                                    };
                                } else {
                                    self.selected_section = (self.selected_section + 1) % len;
                                }
                                self.sidebar_indicator_target = self.selected_section as f32 * 44.0;
                                self.sidebar_indicator_start = Some(std::time::Instant::now());
                                self.active_colony_repo = None;
                                self.save_preferences();
                            }
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown)
                            if self.show_settings =>
                        {
                            self.settings_category = (self.settings_category + 1).min(5);
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp)
                            if self.show_settings =>
                        {
                            self.settings_category = self.settings_category.saturating_sub(1);
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::PageDown)
                            if self.show_settings =>
                        {
                            self.settings_category = (self.settings_category + 3).min(5);
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::PageUp)
                            if self.show_settings =>
                        {
                            self.settings_category = self.settings_category.saturating_sub(3);
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter)
                            if !self.show_settings
                                && !self.show_github_menu
                                && !self.show_first_launch
                                && self.active_colony_repo.is_none() =>
                        {
                            let first =
                                self.filtered_colony_repos().first().map(|r| r.name.clone());
                            if let Some(name) = first {
                                self.active_colony_repo = Some(name);
                                // Refresh the (repo, tab) markdown cache — the
                                // detail view reads only cached blocks/placeholder
                                // now, so opening a repo must recompute them.
                                self.refresh_detail_markdown();
                            }
                        }
                        _ => {}
                    }
                }
                Task::none()
            }
            Message::CheckUpdates => {
                if self.is_checking_updates {
                    return Task::none();
                }
                self.is_checking_updates = true;
                self.status_message = i18n::t("checking_updates");
                // Collect (repo, pinned tag for this platform) for every
                // installed Colony app so update detection compares against the
                // tag that would actually be installed, not /releases/latest.
                let platform = github::current_platform_key();
                let repos: Vec<(String, String)> = self
                    .colony_repos()
                    .iter()
                    .filter(|r| github::installed_app_path(r).is_some())
                    .filter_map(|r| {
                        r.manifest
                            .release_files
                            .get(platform)
                            .map(|entry| (r.name.clone(), entry.tag.clone()))
                    })
                    .collect();

                if repos.is_empty() {
                    // Nothing to check — reset the guard (otherwise it stays true
                    // forever, blocking all later checks) and still run the
                    // chained launcher self-update check.
                    self.is_checking_updates = false;
                    self.status_message = i18n::t_fmt(
                        "apps_found",
                        &[("count", &self.applications.len().to_string())],
                    );
                    return Task::done(Message::CheckLauncherUpdate { manual: false });
                }

                let token = if let GitHubState::Connected { session, .. } = &self.github_state {
                    Some(session.access_token.clone())
                } else {
                    None
                };

                Task::perform(
                    async move {
                        let client = match github::build_update_client(token.as_deref()) {
                            Ok(c) => c,
                            Err(_) => return Vec::new(),
                        };
                        let futs: Vec<_> = repos
                            .iter()
                            .map(|(name, tag)| {
                                let c = client.clone();
                                let n = name.clone();
                                let t = tag.clone();
                                async move {
                                    github::check_update_available(&c, &n, &t)
                                        .await
                                        .map(|v| (n, v))
                                }
                            })
                            .collect();
                        futures::future::join_all(futs)
                            .await
                            .into_iter()
                            .flatten()
                            .collect()
                    },
                    Message::UpdatesChecked,
                )
            }
            Message::UpdateAll => {
                if self.is_downloading {
                    return Task::none();
                }
                let platform = github::current_platform_key();
                // Queue every updatable repo that actually ships an asset for
                // this platform; order follows the catalog for predictability.
                let mut queue: Vec<String> = self
                    .colony_repos()
                    .iter()
                    .filter(|r| {
                        self.available_updates.contains_key(&r.name)
                            && r.manifest.release_files.contains_key(platform)
                    })
                    .map(|r| r.name.clone())
                    .collect();
                if queue.is_empty() {
                    return Task::none();
                }
                let first = queue.remove(0);
                self.update_queue = queue;
                Task::done(Message::DownloadRelease(first, platform.to_string()))
            }
            Message::FetchReleaseNotes(repo_name) => {
                if self.fetching_notes.contains(&repo_name) {
                    return Task::none();
                }
                // Show the notes of the AVAILABLE update when there is one,
                // otherwise of the manifest's pinned/latest release.
                let platform = github::current_platform_key();
                let tag = self.available_updates.get(&repo_name).cloned().or_else(|| {
                    self.colony_repos()
                        .iter()
                        .find(|r| r.name == repo_name)
                        .and_then(|r| r.manifest.release_files.get(platform))
                        .map(|e| e.tag.clone())
                });
                let Some(tag) = tag else {
                    return Task::none();
                };
                self.fetching_notes.insert(repo_name.clone());
                let token = if let GitHubState::Connected { session, .. } = &self.github_state {
                    Some(session.access_token.clone())
                } else {
                    None
                };
                let repo_for_result = repo_name.clone();
                Task::perform(
                    async move {
                        let client = github::build_update_client(token.as_deref())
                            .map_err(|e| e.to_string())?;
                        let info = github::fetch_release_info(&client, &repo_name, &tag)
                            .await
                            .map_err(|e| e.to_string())?;
                        Ok((info.tag, info.body.unwrap_or_default()))
                    },
                    move |result: Result<(String, String), String>| {
                        Message::ReleaseNotesFetched(repo_for_result, result)
                    },
                )
            }
            Message::ReleaseNotesFetched(repo_name, result) => {
                self.fetching_notes.remove(&repo_name);
                match result {
                    Ok((tag, body)) => {
                        let blocks = markdown_blocks::parse(&body);
                        self.release_notes.insert(repo_name, (tag, blocks));
                    }
                    Err(e) => {
                        // Non-blocking feature: a failed fetch surfaces in the
                        // status line, never as a modal interruption.
                        self.status_message = i18n::t_fmt("github_api_error", &[("error", &e)]);
                    }
                }
                Task::none()
            }
            Message::UpdatesChecked(updates) => {
                self.is_checking_updates = false;
                // Record which apps have a pending update so the grid cards can
                // show an update badge (not just a transient toast).
                self.available_updates = updates.iter().cloned().collect();
                let notif_task = if updates.is_empty() {
                    self.status_message = i18n::t_fmt(
                        "apps_found",
                        &[("count", &self.applications.len().to_string())],
                    );
                    Task::none()
                } else {
                    let names: Vec<&str> = updates.iter().map(|(n, _)| n.as_str()).collect();
                    let msg = i18n::t_fmt(
                        "updates_available",
                        &[
                            ("count", &updates.len().to_string()),
                            ("names", &names.join(", ")),
                        ],
                    );
                    self.push_notification(msg, NotificationLevel::Info)
                };
                // Also check for launcher self-update
                Task::batch([
                    notif_task,
                    Task::done(Message::CheckLauncherUpdate { manual: false }),
                ])
            }
            Message::ToggleFavorite(name) => {
                if let Some(pos) = self.favorites.iter().position(|f| f == &name) {
                    self.favorites.remove(pos);
                } else {
                    self.favorites.push(name);
                }
                if let Err(e) = github::save_favorites(&self.favorites) {
                    tracing::warn!("Failed to save favorites: {e}");
                }
                Task::none()
            }
            Message::DismissFirstLaunch => {
                self.show_first_launch = false;
                self.welcome_step = 0;
                self.save_preferences();
                Task::none()
            }
            Message::WelcomeNext => {
                const LAST_STEP: u8 = crate::ui::TUTORIAL_LAST_STEP;
                if self.welcome_step >= LAST_STEP {
                    self.show_first_launch = false;
                    self.welcome_step = 0;
                    self.save_preferences();
                    Task::none()
                } else {
                    self.welcome_step += 1;
                    crate::ui::fetch_bounds_task()
                }
            }
            Message::WelcomeBack => {
                self.welcome_step = self.welcome_step.saturating_sub(1);
                crate::ui::fetch_bounds_task()
            }
            Message::TutorialBoundsUpdated(bounds) => {
                self.tutorial_bounds = bounds;
                Task::none()
            }
            Message::WelcomeConnectGithub => {
                // Close the welcome overlay and jump straight to the GitHub
                // panel so the user can start the device-flow login without
                // an extra "dismiss then navigate" step.
                self.show_first_launch = false;
                self.welcome_step = 0;
                self.show_github_menu = true;
                self.save_preferences();
                Task::none()
            }
            Message::ToggleSettings => {
                self.show_settings = !self.show_settings;
                if !self.show_settings {
                    self.settings_category = 0;
                }
                Task::none()
            }
            Message::SettingsCategory(idx) => {
                self.settings_category = idx;
                Task::none()
            }
            Message::SettingsToggleSection(key) => {
                if !self.settings_expanded_sections.remove(&key) {
                    self.settings_expanded_sections.insert(key);
                }
                Task::none()
            }
            Message::SelectThemeVariant(theme, variant) => {
                self.selected_theme = theme;
                self.selected_variant = variant;
                set_active_theme(&self.selected_theme, &self.selected_variant);
                self.save_preferences();
                self.push_notification(i18n::t("theme_applied"), NotificationLevel::Info)
            }
            Message::SelectAccentColor(color) => {
                set_active_accent(accent_key_to_color(&color));
                self.selected_accent = color;
                self.auto_accent = false;
                self.save_preferences();
                Task::none()
            }
            Message::ToggleAutoAccent => {
                self.auto_accent = !self.auto_accent;
                if self.auto_accent {
                    set_active_accent(None);
                } else {
                    set_active_accent(accent_key_to_color(&self.selected_accent));
                }
                self.save_preferences();
                Task::none()
            }
            Message::ToggleRestoreSession => {
                self.restore_session = !self.restore_session;
                self.save_preferences();
                Task::none()
            }
            Message::PickDefaultView(v) => {
                self.default_view = v;
                self.save_preferences();
                Task::none()
            }
            Message::PickLanguage(v) => {
                self.language = v;
                self.save_preferences();
                self.push_notification(i18n::t("language_restart_notice"), NotificationLevel::Info)
            }
            Message::ToggleAutoCheckUpdates => {
                self.auto_check_updates = !self.auto_check_updates;
                self.save_preferences();
                Task::none()
            }
            Message::PickFontSize(v) => {
                self.font_size = v;
                self.save_preferences();
                Task::none()
            }
            Message::ToggleAnimations => {
                self.animations = !self.animations;
                self.save_preferences();
                Task::none()
            }
            Message::ToggleHighContrast => {
                self.high_contrast = !self.high_contrast;
                set_high_contrast(self.high_contrast);
                self.save_preferences();
                Task::none()
            }
            Message::PickTextSizeA11y(v) => {
                self.text_size_a11y = v;
                self.save_preferences();
                Task::none()
            }
            Message::ToggleReduceMotion => {
                self.reduce_motion = !self.reduce_motion;
                self.save_preferences();
                Task::none()
            }
            Message::ToggleKeyboardNav => {
                self.keyboard_nav = !self.keyboard_nav;
                self.save_preferences();
                Task::none()
            }
            Message::ToggleDyslexiaFont => {
                self.dyslexia_font = !self.dyslexia_font;
                self.save_preferences();
                Task::none()
            }
            Message::ToggleScanOnStartup => {
                self.scan_on_startup = !self.scan_on_startup;
                self.save_preferences();
                Task::none()
            }
            Message::DetailTabSelected(tab) => {
                self.detail_tab = tab;
                self.refresh_detail_markdown();
                Task::none()
            }
            // --- Launcher self-update ---
            Message::CheckLauncherUpdate { manual } => {
                if self.is_checking_launcher_update {
                    return Task::none();
                }
                self.is_checking_launcher_update = true;

                let token = if let GitHubState::Connected { session, .. } = &self.github_state {
                    Some(session.access_token.clone())
                } else {
                    None
                };

                Task::perform(
                    async move {
                        let client = github::build_update_client(token.as_deref())
                            .map_err(|e| e.to_string())?;
                        github::check_launcher_update(&client)
                            .await
                            .map_err(|e| e.to_string())
                    },
                    move |result| Message::LauncherUpdateChecked(manual, result),
                )
            }
            Message::LauncherUpdateChecked(manual, result) => {
                self.is_checking_launcher_update = false;
                match result {
                    Ok(Some((tag, asset))) => {
                        let tag_display = tag.clone();
                        self.launcher_update_available = Some((tag, asset));
                        // On a package-managed install the in-app flow cannot
                        // apply: announce the update with the pacman guidance
                        // instead of pointing at a doomed download button.
                        let key = if self.launcher_system_managed {
                            "launcher_update_system_managed"
                        } else {
                            "launcher_update_available"
                        };
                        self.push_notification(
                            i18n::t_fmt(key, &[("version", &tag_display)]),
                            NotificationLevel::Info,
                        )
                    }
                    Ok(None) => {
                        self.launcher_update_available = None;
                        self.status_message = i18n::t("launcher_up_to_date");
                        if manual {
                            // Explicit feedback for an explicit click; the
                            // automatic boot check stays quiet when current.
                            self.push_notification(
                                i18n::t("launcher_up_to_date"),
                                NotificationLevel::Info,
                            )
                        } else {
                            Task::none()
                        }
                    }
                    Err(e) => {
                        // The check DID NOT run: never claim "up to date".
                        self.status_message = i18n::t_fmt("github_api_error", &[("error", &e)]);
                        if manual {
                            self.push_notification(
                                i18n::t_fmt("github_api_error", &[("error", &e)]),
                                NotificationLevel::Error,
                            )
                        } else {
                            Task::none()
                        }
                    }
                }
            }
            Message::DownloadLauncherUpdate => {
                if self.is_downloading {
                    return Task::none();
                }
                // Defense in depth behind the UI gate: a package-managed exe
                // dir is not writable, so the flow would download the whole
                // asset and then die on the backup rename with EACCES.
                if self.launcher_system_managed {
                    let msg = i18n::t_fmt(
                        "launcher_update_system_managed",
                        &[(
                            "version",
                            &self
                                .launcher_update_available
                                .as_ref()
                                .map(|(t, _)| t.clone())
                                .unwrap_or_default(),
                        )],
                    );
                    self.status_message = msg.clone();
                    return self.push_notification(msg, NotificationLevel::Warning);
                }
                let (tag, asset) = match &self.launcher_update_available {
                    Some(t) => t.clone(),
                    None => return Task::none(),
                };

                let token = if let GitHubState::Connected { session, .. } = &self.github_state {
                    Some(session.access_token.clone())
                } else {
                    None
                };

                self.is_downloading = true;
                self.download_progress = Some((asset.clone(), 0.0));
                self.status_message = i18n::t_fmt("downloading", &[("file", &asset)]);

                let (progress_tx, progress_rx) = futures::channel::mpsc::unbounded::<f32>();

                let download_task = Task::perform(
                    async move {
                        github::download_launcher_asset(token, tag, asset, Some(progress_tx))
                            .await
                            .map_err(|e| e.to_string())
                    },
                    Message::LauncherDownloadCompleted,
                );

                let progress_task = Task::run(progress_rx, Message::LauncherDownloadProgress);

                let (task, handle) = Task::batch([download_task, progress_task]).abortable();
                self.download_abort = Some(handle);
                task
            }
            Message::LauncherDownloadProgress(progress) => {
                if let Some((ref name, _)) = self.download_progress {
                    self.download_progress = Some((name.clone(), progress));
                }
                Task::none()
            }
            Message::LauncherDownloadCompleted(result) => {
                self.download_progress = None;
                self.is_downloading = false;
                self.download_abort = None;
                match result {
                    Ok(path) => {
                        self.launcher_update_staged = Some(path);
                        self.status_message = i18n::t("launcher_update_ready");
                        self.push_notification(
                            i18n::t("launcher_update_ready"),
                            NotificationLevel::Info,
                        )
                    }
                    Err(e) => {
                        self.status_message = i18n::t_fmt("download_error", &[("error", &e)]);
                        self.push_notification(
                            i18n::t_fmt("download_error", &[("error", &e)]),
                            NotificationLevel::Error,
                        )
                    }
                }
            }
            Message::ApplyLauncherUpdate(new_binary) => Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                        github::apply_launcher_update(&new_binary).map_err(|e| e.to_string())
                    })
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|r| r)
                },
                |result: Result<std::path::PathBuf, String>| match result {
                    Ok(exe_path) => {
                        tracing::info!("Launching updated Colony: {}", exe_path.display());
                        let _ = std::process::Command::new(&exe_path).spawn();
                        std::process::exit(0);
                    }
                    Err(e) => Message::LauncherDownloadCompleted(Err(e)),
                },
            ),
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
        std::env::set_var("XDG_CONFIG_HOME", tmp.path().join("config"));
        std::env::set_var("XDG_DATA_HOME", tmp.path().join("data"));
        f();
        match old_config {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_data {
            Some(v) => std::env::set_var("XDG_DATA_HOME", v),
            None => std::env::remove_var("XDG_DATA_HOME"),
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
        assert!(app.update_queue.is_empty(), "failure must not strand the queue");
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
            let orphan = crate::persistence::colony_data_dir()
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
    #[test]
    fn toggle_favorite_persists_to_disk() {
        with_temp_dirs(|| {
            let mut app = App::new_for_test();
            let _ = app.update(Message::ToggleFavorite("Grape".into()));
            assert!(app.is_favorite("Grape"));
            assert_eq!(crate::github::load_favorites(), vec!["Grape".to_string()]);
            let _ = app.update(Message::ToggleFavorite("Grape".into()));
            assert!(!app.is_favorite("Grape"));
            assert!(crate::github::load_favorites().is_empty());
        });
    }
}
