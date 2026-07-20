use iced::font;
use iced::keyboard;
use std::path::PathBuf;

use crate::ui::TutorialBounds;

use crate::github::ColonyRepo;
use crate::oauth::OAuthSession;
use crate::scan::Application;
use crate::state::DetailTab;

#[derive(Debug, Clone)]
pub enum Message {
    SearchChanged(String),
    SectionSelected(usize),
    Rescan,
    RescanCompleted(Result<Vec<Application>, String>),
    LaunchApp(String),
    /// Open a repo's detail page, keyed by repo NAME (indexes go stale when a
    /// catalog refresh reorders the vector).
    ColonyRepoSelected(String),
    ColonyRepoBack,
    ClearStatus,
    FontLoaded(Result<(), font::Error>),
    // GitHub / OAuth messages
    ToggleGitHubMenu,
    GitHubLogin,
    GitHubDeviceCodeReceived(Result<crate::oauth::DeviceCode, String>),
    GitHubLoginCompleted(Result<OAuthSession, String>),
    GitHubLogout,
    GitHubReposFetched(Vec<ColonyRepo>),
    GitHubError(String),
    GitHubRefreshRepos,
    DownloadRelease(String, String), // (repo_name, platform_key)
    #[allow(dead_code)]
    /// (filename, downloaded bytes, total bytes when the server sent
    /// Content-Length) - raw bytes so the UI can show size AND speed, not
    /// just a bare percentage.
    DownloadProgress(String, u64, Option<u64>),
    DownloadCompleted(Result<(PathBuf, String, String), String>), // (path, repo_name, tag)
    CancelDownload,
    LaunchColonyApp(PathBuf),
    UninstallColonyApp(String), // repo_name
    ConfirmUninstall(String),   // repo_name — show confirmation
    CancelUninstall,
    CopyToClipboard(String),
    DismissNotification(u64),
    TickNotifications,
    AnimationTick,
    KeyboardEvent(keyboard::Event),
    CheckUpdates,
    UpdatesChecked(Vec<(String, String)>), // Vec<(repo_name, latest_tag)>
    /// One-click sequential update of every app with a pending update.
    UpdateAll,
    /// Fetch the release notes ("what's new") for a repo's available update.
    FetchReleaseNotes(String),
    /// (repo, Ok((tag, body_markdown))) - notes fetched (or failed).
    ReleaseNotesFetched(String, Result<(String, String), String>),
    WindowResized(f32, f32),
    /// Debounced save of the window size (fires 1s after the LAST resize).
    PersistWindowSize(u64),
    // Favorites
    ToggleFavorite(String),
    // First launch
    DismissFirstLaunch,
    WelcomeNext,
    WelcomeBack,
    WelcomeConnectGithub,
    TutorialBoundsUpdated(TutorialBounds),
    // Settings
    ToggleSettings,
    SettingsCategory(usize),
    SettingsToggleSection(String),
    // Appearance
    SelectThemeVariant(String, String), // (theme, variant)
    SelectAccentColor(String),
    ToggleAutoAccent,
    // Preference toggles
    ToggleRestoreSession,
    PickDefaultView(String),
    PickLanguage(String),
    ToggleAutoCheckUpdates,
    PickFontSize(String),
    ToggleAnimations,
    ToggleHighContrast,
    PickTextSizeA11y(String),
    ToggleReduceMotion,
    ToggleKeyboardNav,
    ToggleDyslexiaFont,
    ToggleScanOnStartup,
    // Detail tabs
    DetailTabSelected(DetailTab),
    OpenUrl(String),
    // Launcher self-update
    /// `manual` is true when the user clicked "Check for updates": only a
    /// manual check gets an "up to date" toast (an automatic one would toast
    /// on every boot) and network failures surface as errors instead of the
    /// check lying that Colony is current.
    CheckLauncherUpdate {
        manual: bool,
    },
    /// (manual, result of the check). Ok(None) = genuinely up to date;
    /// Err = the check could not run (network/rate limit/bad tag).
    LauncherUpdateChecked(bool, Result<Option<(String, String)>, String>),
    DownloadLauncherUpdate,
    LauncherDownloadProgress(f32),
    LauncherDownloadCompleted(Result<std::path::PathBuf, String>),
    ApplyLauncherUpdate(std::path::PathBuf),
}
