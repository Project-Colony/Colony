use iced::Task;
use std::time::Duration;

use crate::github;
use crate::i18n;
use crate::message::Message;
use crate::oauth;
use crate::scan;
use crate::state::{App, GitHubState, Notification, NotificationLevel};
use crate::ui::theme::{accent_key_to_color, set_active_accent, set_active_theme, set_high_contrast};

impl App {
    pub fn push_notification(&mut self, message: String, level: NotificationLevel) -> Task<Message> {
        let id = self.next_notification_id;
        self.next_notification_id += 1;
        let timeout = match level {
            NotificationLevel::Error => Duration::from_secs(10),
            NotificationLevel::Warning => Duration::from_secs(7),
            NotificationLevel::Info => Duration::from_secs(5),
        };
        self.notifications.push(Notification::new(id, message, level));
        // When reduce_motion or animations off, don't auto-dismiss (user must click)
        if self.reduce_motion || !self.animations {
            Task::none()
        } else {
            Task::perform(
                async move { tokio::time::sleep(timeout).await; },
                |_| Message::TickNotifications,
            )
        }
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
            auto_scan: Some(self.auto_scan),
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
                        self.status_message = i18n::t_fmt("apps_found", &[("count", &apps.len().to_string())]);
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
                            .map_err(|error| i18n::t_fmt("launch_error", &[("error", &error.to_string())]))
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
                                            i18n::t_fmt("launch_error", &[("error", &error.to_string())])
                                        })
                                } else {
                                    Err(i18n::t("launch_error_empty"))
                                }
                            }
                            Err(error) => Err(i18n::t_fmt("launch_error", &[("error", &error.to_string())])),
                        }
                    }
                };

                match launch_result {
                    Ok(()) => {
                        self.status_message = i18n::t("app_launched");
                        Task::perform(
                            async { tokio::time::sleep(Duration::from_secs(4)).await; },
                            |_| Message::ClearStatus,
                        )
                    }
                    Err(msg) => {
                        self.status_message = msg.clone();
                        self.push_notification(msg, NotificationLevel::Error)
                    }
                }
            }
            Message::ColonyRepoSelected(index) => {
                self.active_colony_repo = Some(index);
                Task::none()
            }
            Message::ColonyRepoBack => {
                self.active_colony_repo = None;
                self.confirm_uninstall = None;
                self.detail_tab = crate::state::DetailTab::ReadMe;
                Task::none()
            }
            Message::ClearStatus => {
                self.status_message = i18n::t_fmt("apps_found", &[("count", &self.applications.len().to_string())]);
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
            Message::GitHubDeviceCodeReceived(result) => {
                match result {
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
                        self.push_notification(i18n::t_fmt("oauth_error", &[("error", &e)]), NotificationLevel::Error)
                    }
                }
            }
            Message::GitHubLoginCompleted(result) => {
                match result {
                    Ok(session) => {
                        self.github_state = GitHubState::Connected {
                            session: session.clone(),
                            repos: Vec::new(),
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
                    Err(e) => {
                        self.github_state = GitHubState::Error(e.clone());
                        self.status_message = i18n::t_fmt("oauth_error", &[("error", &e)]);
                        self.push_notification(i18n::t_fmt("oauth_error", &[("error", &e)]), NotificationLevel::Error)
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
                if let GitHubState::Connected { repos: r, .. } = &mut self.github_state {
                    *r = repos;
                }
                self.status_message = i18n::t_fmt("github_repos_detected", &[("count", &count.to_string())]);
                if self.auto_check_updates {
                    Task::done(Message::CheckUpdates)
                } else {
                    Task::none()
                }
            }
            Message::GitHubError(e) => {
                self.is_fetching_repos = false;
                tracing::error!(error = %e, "GitHub error");
                if let GitHubState::Connected { repos, .. } = &mut self.github_state {
                    if repos.is_empty() {
                        if let Some(cached) = github::load_repos_cache() {
                            tracing::info!("Using {} cached repos as fallback", cached.len());
                            *repos = cached;
                        }
                    }
                }
                self.status_message = i18n::t_fmt("github_api_error", &[("error", &e)]);
                self.push_notification(i18n::t_fmt("github_api_error", &[("error", &e)]), NotificationLevel::Error)
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
                        let token = if let GitHubState::Connected { session, .. } = &self.github_state {
                            Some(session.access_token.clone())
                        } else {
                            None
                        };
                        let display_name = file.as_deref()
                            .or(file_pattern.as_deref())
                            .unwrap_or(&repo.name)
                            .to_string();
                        self.status_message = i18n::t_fmt("downloading", &[("file", &display_name)]);
                        self.download_progress = Some((display_name.clone(), 0.0));
                        self.is_downloading = true;
                        let dl_repo = repo_name.clone();
                        let (progress_tx, progress_rx) = futures::channel::mpsc::unbounded::<f32>();
                        let progress_name = display_name;

                        let download_task = Task::perform(
                            async move {
                                // Fetch release info if we need tag resolution or asset matching
                                let needs_release_info = tag.eq_ignore_ascii_case("latest") || file_pattern.is_some();

                                let (resolved_tag, resolved_file) = if needs_release_info {
                                    let client = github::build_update_client(token.as_deref())?;
                                    let release_info = github::fetch_release_info(&client, &repo_name, &tag).await?;
                                    let filename = if let Some(ref f) = file {
                                        f.clone()
                                    } else if let Some(ref pattern) = file_pattern {
                                        github::find_asset_by_pattern(&release_info.asset_names, pattern)?
                                    } else {
                                        anyhow::bail!("colony.json: 'file' or 'filePattern' is required");
                                    };
                                    (release_info.tag, filename)
                                } else {
                                    let f = file.ok_or_else(|| anyhow::anyhow!("colony.json: 'file' or 'filePattern' is required"))?;
                                    (tag, f)
                                };

                                let path = github::download_release_asset(
                                    token,
                                    repo_name.clone(),
                                    resolved_tag.clone(),
                                    resolved_file.clone(),
                                    binary,
                                    expected_sha256,
                                    Some(progress_tx),
                                ).await?;

                                // Save resolved asset name when using filePattern (for installed_app_path)
                                if file_pattern.is_some() {
                                    let _ = github::save_installed_asset(&repo_name, &resolved_file);
                                }

                                Ok((path, dl_repo, resolved_tag))
                            },
                            |result: Result<_, anyhow::Error>| {
                                Message::DownloadCompleted(result.map_err(|e| e.to_string()))
                            },
                        );

                        let progress_task = Task::run(
                            progress_rx,
                            move |pct| Message::DownloadProgress(progress_name.clone(), pct),
                        );

                        return Task::batch([download_task, progress_task]);
                    } else {
                        self.status_message = i18n::t_fmt("no_release_for", &[("platform", &platform_key)]);
                    }
                }
                Task::none()
            }
            Message::DownloadProgress(filename, progress) => {
                self.download_progress = Some((filename, progress));
                Task::none()
            }
            Message::DownloadCompleted(result) => {
                self.download_progress = None;
                self.is_downloading = false;
                match result {
                    Ok((path, repo_name, tag)) => {
                        if let Err(e) = github::save_installed_version(&repo_name, &tag) {
                            tracing::warn!("Failed to save version info: {e}");
                        }
                        let display_name = path.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| path.display().to_string());
                        self.status_message = i18n::t_fmt("installed", &[("path", &path.display().to_string())]);
                        self.push_notification(
                            i18n::t_fmt("installed", &[("path", &display_name)]),
                            NotificationLevel::Info,
                        )
                    }
                    Err(e) => {
                        self.status_message = i18n::t_fmt("download_error", &[("error", &e)]);
                        self.push_notification(i18n::t_fmt("download_error", &[("error", &e)]), NotificationLevel::Error)
                    }
                }
            }
            Message::CancelDownload => {
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
                let result = std::process::Command::new(&path)
                    .spawn()
                    .map(|_| ());

                match result {
                    Ok(()) => {
                        self.status_message = i18n::t("app_launched");
                        Task::perform(
                            async { tokio::time::sleep(Duration::from_secs(4)).await; },
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
                match github::colony_apps_dir() {
                    Ok(apps_dir) => {
                        let app_dir = apps_dir.join(&repo_name);
                        if app_dir.exists() {
                            if let Err(e) = std::fs::remove_dir_all(&app_dir) {
                                self.status_message = i18n::t_fmt("uninstall_error", &[("error", &e.to_string())]);
                            } else {
                                self.status_message = i18n::t_fmt("uninstalled", &[("name", &repo_name)]);
                                return Task::perform(
                                    async { tokio::time::sleep(Duration::from_secs(4)).await; },
                                    |_| Message::ClearStatus,
                                );
                            }
                        }
                    }
                    Err(e) => {
                        self.status_message = i18n::t_fmt("scan_error", &[("error", &e.to_string())]);
                    }
                }
                Task::none()
            }
            Message::GitHubRefreshRepos => {
                if self.is_fetching_repos {
                    return Task::none();
                }
                self.is_fetching_repos = true;
                if let GitHubState::Connected { session, .. } = &self.github_state {
                    let token = session.access_token.clone();
                    return Task::perform(
                        async move {
                            github::fetch_colony_repos(Some(&token)).await
                        },
                        |result| match result {
                            Ok(repos) => Message::GitHubReposFetched(repos),
                            Err(e) => Message::GitHubError(e.to_string()),
                        },
                    );
                }
                Task::none()
            }
            Message::CopyToClipboard(value) => {
                iced::clipboard::write(value)
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
                        if (1.0 - notif.fade_in) < SNAP { notif.fade_in = 1.0; }
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
                        if notif.fade_out < SNAP { notif.fade_out = 0.0; }
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
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab) => {
                            if !self.show_settings && !self.show_github_menu && !self.show_first_launch {
                                let len = self.sections.len();
                                if len > 0 {
                                    self.sidebar_indicator_from = self.sidebar_indicator_pos();
                                    if modifiers.shift() {
                                        self.selected_section = if self.selected_section == 0 { len - 1 } else { self.selected_section - 1 };
                                    } else {
                                        self.selected_section = (self.selected_section + 1) % len;
                                    }
                                    self.sidebar_indicator_target = self.selected_section as f32 * 44.0;
                                    self.sidebar_indicator_start = Some(std::time::Instant::now());
                                    self.active_colony_repo = None;
                                    self.save_preferences();
                                }
                            }
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown) => {
                            if self.show_settings {
                                self.settings_category = (self.settings_category + 1).min(5);
                            }
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp) => {
                            if self.show_settings {
                                self.settings_category = self.settings_category.saturating_sub(1);
                            }
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::PageDown) => {
                            if self.show_settings {
                                self.settings_category = (self.settings_category + 3).min(5);
                            }
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::PageUp) => {
                            if self.show_settings {
                                self.settings_category = self.settings_category.saturating_sub(3);
                            }
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter) => {
                            if !self.show_settings && !self.show_github_menu && !self.show_first_launch
                                && self.active_colony_repo.is_none() {
                                    let filtered = self.filtered_colony_repos();
                                    if let Some((idx, _)) = filtered.first() {
                                        self.active_colony_repo = Some(*idx);
                                    }
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
                let repos: Vec<String> = self.colony_repos()
                    .iter()
                    .filter(|r| github::installed_app_path(r).is_some())
                    .map(|r| r.name.clone())
                    .collect();

                if repos.is_empty() {
                    return Task::none();
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
                        let futs: Vec<_> = repos.iter().map(|name| {
                            let c = client.clone();
                            let n = name.clone();
                            async move {
                                github::check_update_available(&c, &n).await.map(|v| (n, v))
                            }
                        }).collect();
                        futures::future::join_all(futs).await.into_iter().flatten().collect()
                    },
                    Message::UpdatesChecked,
                )
            }
            Message::UpdatesChecked(updates) => {
                self.is_checking_updates = false;
                let notif_task = if updates.is_empty() {
                    self.status_message = i18n::t_fmt("apps_found", &[("count", &self.applications.len().to_string())]);
                    Task::none()
                } else {
                    let names: Vec<&str> = updates.iter().map(|(n, _)| n.as_str()).collect();
                    let msg = i18n::t_fmt("updates_available", &[("count", &updates.len().to_string()), ("names", &names.join(", "))]);
                    self.push_notification(msg, NotificationLevel::Info)
                };
                // Also check for launcher self-update
                Task::batch([notif_task, Task::done(Message::CheckLauncherUpdate)])
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
            Message::ToggleAutoScan => {
                self.auto_scan = !self.auto_scan;
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
                Task::none()
            }
            // --- Launcher self-update ---
            Message::CheckLauncherUpdate => {
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
                        let client = github::build_update_client(token.as_deref()).ok()?;
                        github::check_launcher_update(&client).await
                    },
                    Message::LauncherUpdateChecked,
                )
            }
            Message::LauncherUpdateChecked(result) => {
                self.is_checking_launcher_update = false;
                match result {
                    Some((ref tag, _)) => {
                        let tag_display = tag.clone();
                        self.launcher_update_available = result;
                        self.push_notification(
                            i18n::t_fmt("launcher_update_available", &[("version", &tag_display)]),
                            NotificationLevel::Info,
                        )
                    }
                    None => Task::none(),
                }
            }
            Message::DownloadLauncherUpdate => {
                if self.is_downloading {
                    return Task::none();
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

                let progress_task = Task::run(
                    progress_rx,
                    Message::LauncherDownloadProgress,
                );

                Task::batch([download_task, progress_task])
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
            Message::ApplyLauncherUpdate(new_binary) => {
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            github::apply_launcher_update(&new_binary)
                                .map_err(|e| e.to_string())
                        })
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                    },
                    |result: Result<std::path::PathBuf, String>| {
                        match result {
                            Ok(exe_path) => {
                                tracing::info!("Launching updated Colony: {}", exe_path.display());
                                let _ = std::process::Command::new(&exe_path).spawn();
                                std::process::exit(0);
                            }
                            Err(e) => {
                                Message::LauncherDownloadCompleted(Err(e))
                            }
                        }
                    },
                )
            }
        }
    }
}
