use iced::font;
use iced::keyboard;
use std::path::PathBuf;

use crate::oauth::OAuthSession;
use crate::scan::Application;
use crate::state::DetailTab;
use crate::github::ColonyRepo;

#[derive(Debug, Clone)]
pub enum Message {
    SearchChanged(String),
    SectionSelected(usize),
    Rescan,
    RescanCompleted(Result<Vec<Application>, String>),
    LaunchApp(String),
    ColonyRepoSelected(usize),
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
    DownloadRelease(String, String),               // (repo_name, platform_key)
    #[allow(dead_code)]
    DownloadProgress(String, f32),                  // (filename, progress 0.0..1.0)
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
    // Favorites
    ToggleFavorite(String),
    // First launch
    DismissFirstLaunch,
    WelcomeNext,
    WelcomeBack,
    WelcomeConnectGithub,
    // Settings
    ToggleSettings,
    SettingsCategory(usize),
    SettingsToggleSection(String),
    // Appearance
    SelectThemeVariant(String, String), // (theme, variant)
    SelectAccentColor(String),
    ToggleAutoAccent,
    // Preference toggles
    ToggleAutoScan,
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
    CheckLauncherUpdate,
    LauncherUpdateChecked(Option<(String, String)>),           // Option<(tag, asset_filename)>
    DownloadLauncherUpdate,
    LauncherDownloadProgress(f32),
    LauncherDownloadCompleted(Result<std::path::PathBuf, String>),
    ApplyLauncherUpdate(std::path::PathBuf),
}
