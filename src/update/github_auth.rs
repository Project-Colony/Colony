//! GitHub device-flow authentication and catalog fetching.

use iced::Task;

use crate::github;
use crate::i18n;
use crate::message::Message;
use crate::oauth;
use crate::state::{App, GitHubState, NotificationLevel};

impl App {
    pub(super) fn toggle_github_menu(&mut self) -> Task<Message> {
        self.show_github_menu = !self.show_github_menu;
        Task::none()
    }

    pub(super) fn github_login(&mut self) -> Task<Message> {
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

    pub(super) fn github_device_code_received(
        &mut self,
        result: Result<crate::oauth::DeviceCode, String>,
    ) -> Task<Message> {
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
                self.push_notification(
                    i18n::t_fmt("oauth_error", &[("error", &e)]),
                    NotificationLevel::Error,
                )
            }
        }
    }

    pub(super) fn github_login_completed(
        &mut self,
        result: Result<crate::oauth::OAuthSession, String>,
    ) -> Task<Message> {
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

    pub(super) fn github_logout(&mut self) -> Task<Message> {
        let _ = oauth::delete_saved_token();
        self.github_state = GitHubState::Disconnected;
        self.status_message = i18n::t("github_disconnected");
        Task::none()
    }

    pub(super) fn github_repos_fetched(
        &mut self,
        repos: Vec<crate::github::ColonyRepo>,
    ) -> Task<Message> {
        self.is_fetching_repos = false;
        let count = repos.len();
        if let Err(e) = crate::persistence::save_repos_cache(&repos) {
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
        self.refresh_install_status();
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

    pub(super) fn github_error(&mut self, e: String) -> Task<Message> {
        self.is_fetching_repos = false;
        tracing::error!(error = %e, "GitHub error");
        if self.colony_repo_list.is_empty() {
            if let Some(cached) = crate::persistence::load_repos_cache() {
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

    pub(super) fn github_refresh_repos(&mut self) -> Task<Message> {
        if self.is_fetching_repos {
            return Task::none();
        }
        self.is_fetching_repos = true;
        // Anonymous refresh is supported: the token only raises the
        // rate limit (60 req/h unauthenticated vs 5000 signed-in).
        let token = self.github_token();
        Task::perform(
            async move { github::fetch_colony_repos(token.as_deref()).await },
            |result| match result {
                Ok(repos) => Message::GitHubReposFetched(repos),
                Err(e) => Message::GitHubError(e.to_string()),
            },
        )
    }
}
