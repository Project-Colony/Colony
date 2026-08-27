//! The store side of the launcher: install, update, uninstall, release notes.

use iced::Task;
use std::time::Duration;

use crate::github;
use crate::i18n;
use crate::message::Message;
use crate::state::{App, NotificationLevel};
use crate::ui::markdown_blocks;

impl App {
    pub(super) fn download_release(
        &mut self,
        repo_name: String,
        platform_key: String,
    ) -> Task<Message> {
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
                // Only what the manifest declares; the installer ORs in
                // its own pin from any previously verified install, so
                // that rule lives next to the check it feeds.
                let require_signature = repo.manifest.signed;
                let repo_name = repo.name.clone();
                // The app's menu name, distinct from `display_name` below,
                // which is the FILE being downloaded (shown in the toast).
                let app_display_name = repo.display_name().to_string();
                let app_category = crate::scan::AppCategory::from_name(&repo.manifest.category);
                // API calls (release resolution) use the token for
                // rate limits; the asset download itself is a public
                // endpoint and gets NO token - no reason to present
                // credentials where none are needed.
                let token = self.github_token();
                let display_name = file
                    .as_deref()
                    .or(file_pattern.as_deref())
                    .unwrap_or(&repo.name)
                    .to_string();
                self.status_message = i18n::t_fmt("downloading", &[("file", &display_name)]);
                self.download_progress = Some((display_name.clone(), 0.0));
                self.is_downloading = true;
                self.downloading_repo = Some(repo_name.clone());
                let dl_repo = repo_name.clone();
                let (progress_tx, progress_rx) =
                    futures::channel::mpsc::unbounded::<(u64, Option<u64>)>();
                let progress_name = display_name;

                let download_task = Task::perform(
                    async move {
                        // Fetch release info if we need tag resolution or asset matching
                        let needs_release_info =
                            tag.eq_ignore_ascii_case("latest") || file_pattern.is_some();

                        let (resolved_tag, resolved_file) = if needs_release_info {
                            let client = github::build_update_client(token.as_deref())?;
                            let release_info =
                                github::fetch_release_info(&client, &repo_name, &tag).await?;
                            let filename = if let Some(ref f) = file {
                                f.clone()
                            } else if let Some(ref pattern) = file_pattern {
                                github::find_asset_by_pattern(&release_info.asset_names, pattern)?
                            } else {
                                anyhow::bail!("colony.json: 'file' or 'filePattern' is required");
                            };
                            (release_info.tag, filename)
                        } else {
                            let f = file.ok_or_else(|| {
                                anyhow::anyhow!("colony.json: 'file' or 'filePattern' is required")
                            })?;
                            (tag, f)
                        };

                        // The version/asset records are written by
                        // download_release_asset itself, inside the
                        // blocking install step: writing them here (or
                        // in DownloadCompleted) meant a cancel landing
                        // mid-install detached the blocking task and
                        // left an installed binary with no metadata.
                        let path = crate::download::download_release_asset(
                            None,
                            crate::download::AssetInstall {
                                repo_name: repo_name.clone(),
                                display_name: app_display_name,
                                category: app_category,
                                tag: resolved_tag.clone(),
                                filename: resolved_file.clone(),
                                binary_name: binary,
                                expected_sha256,
                                record_asset: file_pattern.is_some(),
                                require_signature,
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

                let progress_task = Task::run(progress_rx, move |(downloaded, total)| {
                    Message::DownloadProgress(progress_name.clone(), downloaded, total)
                });

                // Keep an abort handle so CancelDownload actually stops
                // the download and its progress stream (dropping the
                // progress sender), instead of only clearing the UI.
                let (task, handle) = Task::batch([download_task, progress_task]).abortable();
                self.download_abort = Some(handle);
                return task;
            } else {
                self.status_message = i18n::t_fmt("no_release_for", &[("platform", &platform_key)]);
            }
        }

        // We got here without starting a download: either the repo vanished
        // from the catalog (a refresh can land mid-queue and replaces it
        // wholesale) or it ships nothing for this platform. Say so, and keep
        // the "Update all" chain moving - the queue is otherwise only advanced
        // by a completion, so it would sit parked until some later, unrelated
        // install silently drained it.
        let skipped = i18n::t_fmt("update_skipped", &[("name", &repo_name)]);
        Task::batch([
            self.push_notification(skipped, NotificationLevel::Warning),
            self.dispatch_next_queued_update(),
        ])
    }

    pub(super) fn download_progress(
        &mut self,
        filename: String,
        downloaded: u64,
        total: Option<u64>,
    ) -> Task<Message> {
        // Ignore late progress events from a cancelled/finished download
        // so the toast cannot resurrect after CancelDownload.
        if self.is_downloading {
            let fraction = total
                .filter(|t| *t > 0)
                .map(|t| downloaded as f32 / t as f32)
                .unwrap_or(0.0);
            self.download_progress = Some((filename, fraction));
            self.download_bytes = Some((downloaded, total));
            // Transfer speed: exponential moving average over samples.
            let now = std::time::Instant::now();
            if let Some((t0, b0)) = self.last_progress_sample {
                let dt = now.duration_since(t0).as_secs_f32();
                if dt > 0.05 && downloaded >= b0 {
                    let inst = (downloaded - b0) as f32 / dt;
                    self.download_speed = if self.download_speed > 0.0 {
                        0.7 * self.download_speed + 0.3 * inst
                    } else {
                        inst
                    };
                    self.last_progress_sample = Some((now, downloaded));
                }
            } else {
                self.last_progress_sample = Some((now, downloaded));
            }
        }
        Task::none()
    }

    pub(super) fn download_completed(
        &mut self,
        result: Result<(std::path::PathBuf, String, String), String>,
    ) -> Task<Message> {
        self.download_progress = None;
        self.download_bytes = None;
        self.download_speed = 0.0;
        self.last_progress_sample = None;
        self.is_downloading = false;
        self.download_abort = None;
        self.downloading_repo = None;
        match result {
            // Version/asset records were written atomically with the
            // install (inside download_release_asset), so the tag is
            // no longer needed here.
            Ok((path, repo_name, _tag)) => {
                // The just-installed version IS the one the badge was
                // advertising: clear it, or the card keeps showing
                // "Update vX -> vX" until the next global check.
                self.available_updates.remove(&repo_name);
                self.refresh_install_status();
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

    pub(super) fn cancel_download(&mut self) -> Task<Message> {
        // Actually abort the running download + progress tasks so no
        // phantom install completes and no second writer can race the
        // same file on a retry. Cancel also empties the "Update all"
        // queue: cancelling means stop, not "skip this one".
        self.update_queue.clear();
        if let Some(handle) = self.download_abort.take() {
            handle.abort();
        }
        // The aborted task cannot clean up its staging file: sweep the
        // cancelled repo's leftovers here, including the `.part.id` sidecar
        // that would otherwise invite a resume of a transfer the user stopped
        // on purpose.
        if let Some(repo) = self.downloading_repo.take() {
            if let Ok(app_dir) = crate::persistence::colony_app_dir(&repo) {
                if let Ok(entries) = std::fs::read_dir(&app_dir) {
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if name.ends_with(".part") || name.ends_with(".part.id") {
                            let _ = std::fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }
        self.download_progress = None;
        self.download_bytes = None;
        self.download_speed = 0.0;
        self.last_progress_sample = None;
        self.is_downloading = false;
        // The install itself runs in a DETACHED blocking task that runs to
        // completion, so a cancel landing after the rename and marker writes
        // leaves the app genuinely installed while install_status - the only
        // source the grid and detail views read - still says otherwise.
        self.refresh_install_status();
        self.status_message = i18n::t("download_cancelled");
        self.push_notification(i18n::t("download_cancelled"), NotificationLevel::Warning)
    }

    pub(super) fn launch_colony_app(&mut self, path: std::path::PathBuf) -> Task<Message> {
        // Executed directly on every platform, never through `cmd /C`:
        // Windows only quotes an argument containing a space or tab, so a
        // manifest-derived path like `app&calc` reached cmd unquoted and
        // its `&` was parsed as a command separator. A store install is
        // always a real executable, so the shell buys nothing here.
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

    pub(super) fn confirm_uninstall(&mut self, repo_name: String) -> Task<Message> {
        self.confirm_uninstall = Some(repo_name);
        Task::none()
    }

    pub(super) fn cancel_uninstall(&mut self) -> Task<Message> {
        self.confirm_uninstall = None;
        Task::none()
    }

    pub(super) fn uninstall_colony_app(&mut self, repo_name: String) -> Task<Message> {
        self.confirm_uninstall = None;

        let app_dir = match crate::persistence::colony_app_dir(&repo_name) {
            Ok(dir) => dir,
            Err(e) => {
                let msg = i18n::t_fmt("scan_error", &[("error", &e.to_string())]);
                self.status_message = msg.clone();
                return self.push_notification(msg, NotificationLevel::Error);
            }
        };

        // Do the thing that can FAIL first. The teardown used to run in the
        // opposite order - drop the update badge, drop the release notes, delete
        // the desktop entry, and only then try the removal - so a directory that
        // could not be deleted (a running binary on Windows, a busy or read-only
        // path on Unix) left the app on disk with its integration already ripped
        // out, and nothing said so.
        if app_dir.exists() {
            if let Err(e) = crate::persistence::remove_app_dir(&app_dir) {
                let msg = i18n::t_fmt("uninstall_error", &[("error", &e.to_string())]);
                self.status_message = msg.clone();
                // The card must reflect what is actually on disk either way.
                self.refresh_install_status();
                return self.push_notification(msg, NotificationLevel::Error);
            }
        }

        // Committed: now the state that cannot be undone.
        // An uninstalled app has no meaningful "update available".
        self.available_updates.remove(&repo_name);
        // Stale notes describe the version that was just removed.
        // (Doc/icon caches and the favorite deliberately survive: they
        // belong to the CATALOG entry, which is still listed - orphan
        // cleanup happens on catalog refresh instead.)
        self.release_notes.remove(&repo_name);
        crate::persistence::remove_desktop_entry(&repo_name);
        // AFTER the directory removal, so the cache records the app as gone.
        self.refresh_install_status();
        // Removing something already absent is a success, not a silent skip:
        // the confirm dialog used to just close with no message at all.
        self.status_message = i18n::t_fmt("uninstalled", &[("name", &repo_name)]);
        Task::perform(
            async {
                tokio::time::sleep(Duration::from_secs(4)).await;
            },
            |_| Message::ClearStatus,
        )
    }

    pub(super) fn clear_store_caches(&mut self) -> Task<Message> {
        let removed = crate::persistence::clear_store_caches();
        // Including the remembered 404s: a user who clears caches because a
        // repo just added a CHANGELOG should not wait out the negative TTL.
        crate::github::clear_http_cache();
        self.app_icons.clear();
        self.release_notes.clear();
        self.detail_md_source = None;
        self.refresh_detail_markdown();
        let msg = i18n::t_fmt("caches_cleared", &[("count", &removed.to_string())]);
        self.status_message = msg.clone();
        self.push_notification(msg, NotificationLevel::Info)
    }

    pub(super) fn check_updates(&mut self) -> Task<Message> {
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
            .filter(|r| crate::persistence::installed_app_path(r).is_some())
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

        let token = self.github_token();

        Task::perform(
            async move {
                let client = match github::build_update_client(token.as_deref()) {
                    Ok(c) => c,
                    Err(e) => {
                        // The check did not run for ANY repo. Report that per
                        // repo rather than returning an empty list, which the
                        // handler would read as "everything is current".
                        let e = e.to_string();
                        return repos
                            .into_iter()
                            .map(|(name, _)| (name, Err(e.clone())))
                            .collect();
                    }
                };
                let futs: Vec<_> = repos
                    .iter()
                    .map(|(name, tag)| {
                        let c = client.clone();
                        let n = name.clone();
                        let t = tag.clone();
                        async move {
                            let outcome = github::check_update_available(&c, &n, &t)
                                .await
                                .map_err(|e| e.to_string());
                            (n, outcome)
                        }
                    })
                    .collect();
                futures::future::join_all(futs).await
            },
            Message::UpdatesChecked,
        )
    }

    pub(super) fn update_all(&mut self) -> Task<Message> {
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

    pub(super) fn fetch_release_notes(&mut self, repo_name: String) -> Task<Message> {
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
        // What this platform would actually download, when the manifest names
        // it outright (the filePattern case is resolved at install time).
        let asset_hint = self
            .colony_repos()
            .iter()
            .find(|r| r.name == repo_name)
            .and_then(|r| r.manifest.release_files.get(platform))
            .and_then(|e| e.file.clone());
        self.fetching_notes.insert(repo_name.clone());
        let token = self.github_token();
        let repo_for_result = repo_name.clone();
        Task::perform(
            async move {
                let client =
                    github::build_update_client(token.as_deref()).map_err(|e| e.to_string())?;
                let info = github::fetch_release_info(&client, &repo_name, &tag)
                    .await
                    .map_err(|e| e.to_string())?;
                // The size of the asset THIS platform would download - not the
                // release total, which would be four binaries.
                let size = asset_hint
                    .as_deref()
                    .and_then(|name| info.asset_sizes.get(name).copied())
                    .or_else(|| {
                        // filePattern, or no declared filename: fall back to the
                        // single asset if the release has exactly one.
                        (info.asset_sizes.len() == 1)
                            .then(|| info.asset_sizes.values().copied().next())
                            .flatten()
                    });
                let facts = crate::state::ReleaseFacts {
                    tag: info.tag,
                    size,
                    published_at: info.published_at,
                };
                Ok((facts, info.body.unwrap_or_default()))
            },
            move |result: Result<(crate::state::ReleaseFacts, String), String>| {
                Message::ReleaseNotesFetched(repo_for_result, result)
            },
        )
    }

    pub(super) fn release_notes_fetched(
        &mut self,
        repo_name: String,
        result: Result<(crate::state::ReleaseFacts, String), String>,
    ) -> Task<Message> {
        self.fetching_notes.remove(&repo_name);
        match result {
            Ok((facts, body)) => {
                let blocks = markdown_blocks::parse(&body);
                self.release_notes
                    .insert(repo_name.clone(), (facts.tag.clone(), blocks));
                self.release_facts.insert(repo_name, facts);
            }
            Err(e) => {
                // Non-blocking feature: a failed fetch surfaces in the
                // status line, never as a modal interruption.
                self.status_message = i18n::t_fmt("github_api_error", &[("error", &e)]);
            }
        }
        Task::none()
    }

    pub(super) fn updates_checked(
        &mut self,
        outcomes: Vec<(String, Result<Option<String>, String>)>,
    ) -> Task<Message> {
        self.is_checking_updates = false;

        // Merge, never replace: a repo whose check could not run keeps the
        // badge it already had. Replacing the whole map meant that going
        // offline (or simply hitting the anonymous rate limit on a second
        // launch) cleared every badge and told the user they were current.
        let mut failed = 0usize;
        let mut fresh: Vec<String> = Vec::new();
        for (name, outcome) in &outcomes {
            match outcome {
                Ok(Some(tag)) => {
                    self.available_updates.insert(name.clone(), tag.clone());
                    fresh.push(name.clone());
                }
                Ok(None) => {
                    self.available_updates.remove(name);
                }
                Err(e) => {
                    failed += 1;
                    tracing::warn!("Update check failed for {name}: {e}");
                }
            }
        }

        let notif_task = if failed > 0 {
            // Never write the all-clear line when part of the check did not
            // run - say so, and say how many apps we could not speak for.
            let msg = i18n::t_fmt("update_check_failed", &[("count", &failed.to_string())]);
            self.status_message = msg.clone();
            self.push_notification(msg, NotificationLevel::Warning)
        } else if fresh.is_empty() {
            self.status_message = i18n::t_fmt(
                "apps_found",
                &[("count", &self.applications.len().to_string())],
            );
            Task::none()
        } else {
            let msg = i18n::t_fmt(
                "updates_available",
                &[
                    ("count", &fresh.len().to_string()),
                    ("names", &fresh.join(", ")),
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

    pub(super) fn toggle_favorite(&mut self, name: String) -> Task<Message> {
        if let Some(pos) = self.favorites.iter().position(|f| f == &name) {
            self.favorites.remove(pos);
        } else {
            self.favorites.push(name);
        }
        if let Err(e) = crate::persistence::save_favorites(&self.favorites) {
            tracing::warn!("Failed to save favorites: {e}");
        }
        Task::none()
    }
}
