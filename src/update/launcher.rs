//! Colony's own self-update: check, download, apply, relaunch.
//!
//! The trust rules live in [`crate::download`] and [`crate::signing`]; this
//! module only drives the UI state machine around them.

use iced::Task;

use crate::github;
use crate::i18n;
use crate::message::Message;
use crate::state::{App, NotificationLevel};

impl App {
    pub(super) fn check_launcher_update(&mut self, manual: bool) -> Task<Message> {
        if self.is_checking_launcher_update {
            return Task::none();
        }
        self.is_checking_launcher_update = true;

        let token = self.github_token();

        Task::perform(
            async move {
                let client =
                    github::build_update_client(token.as_deref()).map_err(|e| e.to_string())?;
                github::check_launcher_update(&client)
                    .await
                    .map_err(|e| e.to_string())
            },
            move |result| Message::LauncherUpdateChecked(manual, result),
        )
    }

    pub(super) fn launcher_update_checked(
        &mut self,
        manual: bool,
        result: Result<Option<(String, String)>, String>,
    ) -> Task<Message> {
        self.is_checking_launcher_update = false;
        match result {
            Ok(Some((tag, asset))) => {
                let tag_display = tag.clone();
                self.launcher_update_available = Some((tag, asset));
                // On a package-managed install the in-app flow cannot apply:
                // announce the update with the pacman guidance instead of
                // pointing at a doomed download button.
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
                    // Explicit feedback for an explicit click; the automatic
                    // boot check stays quiet when current.
                    self.push_notification(i18n::t("launcher_up_to_date"), NotificationLevel::Info)
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

    pub(super) fn download_launcher_update(&mut self) -> Task<Message> {
        if self.is_downloading {
            return Task::none();
        }
        // Defense in depth behind the UI gate: a package-managed exe dir is not
        // writable, so the flow would download the whole asset and then die on
        // the backup rename with EACCES.
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

        let token = self.github_token();

        self.is_downloading = true;
        self.download_progress = Some((asset.clone(), 0.0));
        self.status_message = i18n::t_fmt("downloading", &[("file", &asset)]);

        let (progress_tx, progress_rx) = futures::channel::mpsc::unbounded::<(u64, Option<u64>)>();

        let download_task = Task::perform(
            async move {
                crate::download::download_launcher_asset(token, tag, asset, Some(progress_tx))
                    .await
                    .map_err(|e| e.to_string())
            },
            Message::LauncherDownloadCompleted,
        );

        let progress_task = Task::run(progress_rx, |(downloaded, total)| {
            Message::LauncherDownloadProgress(
                total
                    .filter(|t| *t > 0)
                    .map(|t| downloaded as f32 / t as f32)
                    .unwrap_or(0.0),
            )
        });

        let (task, handle) = Task::batch([download_task, progress_task]).abortable();
        self.download_abort = Some(handle);
        task
    }

    pub(super) fn launcher_download_progress(&mut self, progress: f32) -> Task<Message> {
        if let Some((ref name, _)) = self.download_progress {
            self.download_progress = Some((name.clone(), progress));
        }
        Task::none()
    }

    pub(super) fn launcher_download_completed(
        &mut self,
        result: Result<std::path::PathBuf, String>,
    ) -> Task<Message> {
        self.download_progress = None;
        self.is_downloading = false;
        self.download_abort = None;
        match result {
            Ok(path) => {
                self.launcher_update_staged = Some(path);
                self.status_message = i18n::t("launcher_update_ready");
                self.push_notification(i18n::t("launcher_update_ready"), NotificationLevel::Info)
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

    pub(super) fn apply_launcher_update(
        &mut self,
        new_binary: std::path::PathBuf,
    ) -> Task<Message> {
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    crate::download::apply_launcher_update(&new_binary).map_err(|e| e.to_string())
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
        )
    }
}
