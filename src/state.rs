use iced::font::Weight;
use iced::Font;
use std::collections::HashSet;
use std::time::{Duration, Instant};

use crate::github::ColonyRepo;
use crate::oauth::OAuthSession;
use crate::scan::{self, Application};
use crate::sections::Section;
use crate::ui::theme::Palette;

// --- Animation helpers ---

/// Ease-out cubic: fast start, smooth deceleration. t in 0.0..1.0.
pub fn ease_out_cubic(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

/// Apply alpha to a Color by multiplying its existing alpha channel.
pub fn with_alpha(color: iced::Color, alpha: f32) -> iced::Color {
    iced::Color {
        a: color.a * alpha,
        ..color
    }
}

// --- Font constants ---

pub const APP_FONT_NAME: &str = "JetBrainsMono Nerd Font";
pub const APP_FONT_BYTES: [&[u8]; 3] = [
    include_bytes!("ui/assets/fonts/JetBrainsMonoNerdFont/JetBrainsMonoNerdFont-Regular.ttf"),
    include_bytes!("ui/assets/fonts/JetBrainsMonoNerdFont/JetBrainsMonoNerdFont-Medium.ttf"),
    include_bytes!("ui/assets/fonts/JetBrainsMonoNerdFont/JetBrainsMonoNerdFont-Bold.ttf"),
];

pub const DYSLEXIA_FONT_NAME: &str = "OpenDyslexic";
pub const DYSLEXIA_FONT_BYTES: &[u8] =
    include_bytes!("ui/assets/fonts/OpenDyslexic/OpenDyslexic-Regular.otf");

pub const FA_FONT_NAME: &str = "Font Awesome 6 Free";
pub const FA_FONT_BYTES: [&[u8]; 2] = [
    include_bytes!("ui/assets/fonts/FontAwesome/fa-solid-900.ttf"),
    include_bytes!("ui/assets/fonts/FontAwesome/fa-regular-400.ttf"),
];

pub fn default_font() -> Font {
    Font::with_name(APP_FONT_NAME)
}

// --- Notification system ---

#[derive(Debug, Clone)]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone)]
pub struct Notification {
    pub id: u64,
    pub message: String,
    pub level: NotificationLevel,
    pub created_at: Instant,
    // Animation state
    pub fade_in: f32,   // 0.0 → 1.0 on creation
    pub fade_out: f32,  // 1.0 → 0.0 before removal
    pub removing: bool, // true when fade-out has started
}

impl Notification {
    pub fn new(id: u64, message: String, level: NotificationLevel) -> Self {
        Self {
            id,
            message,
            level,
            created_at: Instant::now(),
            fade_in: 0.0,
            fade_out: 1.0,
            removing: false,
        }
    }

    /// Combined opacity from fade-in and fade-out animations.
    pub fn opacity(&self) -> f32 {
        ease_out_cubic(self.fade_in) * self.fade_out
    }

    pub fn color(&self) -> iced::Color {
        match self.level {
            NotificationLevel::Info => Palette::SUCCESS(),
            NotificationLevel::Warning => Palette::WARNING(),
            NotificationLevel::Error => Palette::ERROR(),
        }
    }

    pub fn bg_color(&self) -> iced::Color {
        match self.level {
            NotificationLevel::Info => Palette::SUCCESS_BG(),
            NotificationLevel::Warning => Palette::WARNING_BG(),
            NotificationLevel::Error => Palette::ERROR_BG(),
        }
    }

    pub fn is_expired(&self) -> bool {
        let timeout = match self.level {
            NotificationLevel::Error => Duration::from_secs(10),
            NotificationLevel::Warning => Duration::from_secs(7),
            NotificationLevel::Info => Duration::from_secs(5),
        };
        self.created_at.elapsed() > timeout
    }
}

// --- GitHub connection state ---

#[derive(Debug, Clone)]
pub enum GitHubState {
    Disconnected,
    Connecting { user_code: Option<String> },
    Connected { session: OAuthSession },
    Error(String),
}

// --- Detail tab ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DetailTab {
    ReadMe,
    License,
    Changelog,
}

// --- App state ---

pub struct App {
    pub applications: Vec<Application>,
    pub search_query: String,
    pub sections: Vec<Section>,
    pub selected_section: usize,
    pub status_message: String,
    /// The repo whose detail page is open, tracked by NAME: the catalog vector
    /// is replaced and reordered by every refresh (GitHub sorts by last push),
    /// so an index here would silently swap the open page to a different app.
    pub active_colony_repo: Option<String>,
    pub font: Font,
    // GitHub / OAuth
    pub github_state: GitHubState,
    pub show_github_menu: bool,
    /// The store catalog. Lives OUTSIDE GitHubState on purpose: the catalog is
    /// public data, browsable anonymously (60 req/h unauthenticated GitHub API)
    /// and restored from the on-disk cache at boot - signing in only raises the
    /// rate limit and unlocks account features. Keyed UI state (e.g. the open
    /// detail page) must reference repos by NAME, not index: refreshes replace
    /// and reorder this vector.
    pub colony_repo_list: Vec<ColonyRepo>,
    // Notifications
    pub notifications: Vec<Notification>,
    pub next_notification_id: u64,
    /// Decoded per-app icons (repo_name -> iced image handle), built from the
    /// on-disk icon cache when repos load. Runtime-only, never serialized. Cards
    /// fall back to the tinted category hexagon when a repo has no icon.
    pub app_icons: std::collections::HashMap<String, iced::widget::image::Handle>,
    // Download progress
    pub download_progress: Option<(String, f32)>, // (filename, 0.0..1.0)
    /// Abort handle for the in-flight download (app asset or launcher self-
    /// update). Cancelling actually aborts the task instead of only clearing UI.
    pub download_abort: Option<iced::task::Handle>,
    // Favorites
    pub favorites: Vec<String>,
    // Uninstall confirmation
    pub confirm_uninstall: Option<String>, // repo_name pending confirmation
    // First launch
    pub show_first_launch: bool,
    /// Index of the active step in the first-launch guided tutorial.
    pub welcome_step: u8,
    /// Live bounds of key UI zones, collected via widget::operate for the
    /// spotlight overlay. Falls back to hardcoded rects until populated.
    pub tutorial_bounds: crate::ui::TutorialBounds,
    // Settings
    pub show_settings: bool,
    pub settings_category: usize,
    // Appearance
    pub selected_theme: String,   // e.g. "gruvbox"
    pub selected_variant: String, // e.g. "dark"
    pub selected_accent: String,  // e.g. "blue"
    pub auto_accent: bool,
    // General preferences
    pub restore_session: bool,
    pub default_view: String, // "all", "favorites"
    pub language: String,     // "fr", "en"
    pub auto_check_updates: bool,
    // Appearance extras
    pub font_size: String, // "small", "default", "large"
    pub animations: bool,
    // Accessibility
    pub high_contrast: bool,
    pub text_size_a11y: String, // "small", "default", "large", "xlarge"
    pub reduce_motion: bool,
    pub keyboard_nav: bool,
    pub dyslexia_font: bool,
    // Storage
    pub scan_on_startup: bool,
    // Async operation tracking
    pub is_scanning: bool,
    pub is_downloading: bool,
    pub is_checking_updates: bool,
    pub is_fetching_repos: bool,
    // Settings section state persistence
    pub settings_expanded_sections: HashSet<String>,
    // Detail tabs
    pub detail_tab: DetailTab,
    /// Parsed Markdown blocks for the currently viewed (repo, tab). Rebuilt
    /// in update.rs whenever the tab changes, a repo is selected, or its
    /// docs arrive over the wire. Avoids re-parsing on every frame.
    pub detail_blocks: Vec<crate::ui::markdown_blocks::DetailBlock>,
    pub detail_md_source: Option<(String, DetailTab)>,
    /// Whether the current detail tab has no document (show placeholder text).
    /// Computed with the markdown cache so the view does no per-frame disk I/O.
    pub detail_is_placeholder: bool,
    /// Repos with a pending app update: repo_name -> available tag. Populated by
    /// Message::UpdatesChecked; read by the grid cards to show an update badge.
    pub available_updates: std::collections::HashMap<String, String>,
    /// Repos queued behind the in-flight download by "Update all". Installs
    /// run sequentially (one progress slot, one writer); each completion -
    /// success or failure - dispatches the next entry.
    pub update_queue: Vec<String>,
    /// Fetched release notes per repo: repo_name -> (tag, pre-parsed markdown
    /// blocks). Parsed once at fetch time so the view renders cached blocks
    /// with zero per-frame parsing (same discipline as detail_blocks).
    pub release_notes:
        std::collections::HashMap<String, (String, Vec<crate::ui::markdown_blocks::DetailBlock>)>,
    /// Repos whose release notes are currently being fetched.
    pub fetching_notes: std::collections::HashSet<String>,
    // Launcher self-update
    pub launcher_update_available: Option<(String, String)>, // (tag, asset_filename)
    pub is_checking_launcher_update: bool,
    pub launcher_update_staged: Option<std::path::PathBuf>,
    /// True when the exe lives in a package-manager-owned location (/usr,
    /// /opt): self-update cannot apply there, the UI offers pacman guidance
    /// instead. Computed once at boot.
    pub launcher_system_managed: bool,
    // Animation state
    pub progress_display: f32, // smoothly interpolated progress bar value
    pub sidebar_indicator_from: f32, // start Y position of current animation
    pub sidebar_indicator_target: f32, // target Y position
    pub sidebar_indicator_start: Option<Instant>, // when the animation started (None = idle)
}

impl App {
    /// Get the store catalog (available signed-in or anonymous).
    pub fn colony_repos(&self) -> &[ColonyRepo] {
        &self.colony_repo_list
    }

    /// Resolve the repo whose detail page is open, surviving catalog refreshes
    /// (`None` if the repo disappeared from the catalog).
    pub fn active_repo(&self) -> Option<&ColonyRepo> {
        let name = self.active_colony_repo.as_deref()?;
        self.colony_repo_list.iter().find(|r| r.name == name)
    }

    /// Filter local applications by the currently selected section.
    pub fn filtered_applications(&self) -> Vec<&Application> {
        let query = self.search_query.to_lowercase();
        let selected_section = self.sections.get(self.selected_section);
        let is_favorites = selected_section.map(|s| s.is_favorites).unwrap_or(false);
        self.applications
            .iter()
            .filter(|app| {
                if is_favorites {
                    if !self.is_favorite(&app.name) {
                        return false;
                    }
                } else if let Some(section) = selected_section {
                    if !section.filter.matches(app) {
                        return false;
                    }
                }
                if query.is_empty() {
                    return true;
                }
                app.name.to_lowercase().contains(&query)
            })
            .collect()
    }

    /// Filter Colony repos by the currently selected section's category.
    pub fn filtered_colony_repos(&self) -> Vec<&ColonyRepo> {
        let query = self.search_query.to_lowercase();
        let selected_section = self.sections.get(self.selected_section);
        let is_favorites = selected_section.map(|s| s.is_favorites).unwrap_or(false);

        self.colony_repos()
            .iter()
            .filter(|repo| {
                if is_favorites {
                    if !self.is_favorite(&repo.name) {
                        return false;
                    }
                } else if let Some(section) = selected_section {
                    if let Some(section_category) = section.category() {
                        let repo_category = scan::AppCategory::from_name(&repo.manifest.category);
                        if &repo_category != section_category {
                            return false;
                        }
                    }
                }
                if query.is_empty() {
                    return true;
                }
                // Search the description and display name too: "music" must
                // find Grape ("Lecteur musique..."), not just name matches.
                repo.name.to_lowercase().contains(&query)
                    || repo.manifest.name.to_lowercase().contains(&query)
                    || repo.description.to_lowercase().contains(&query)
            })
            .collect()
    }

    /// Check if an app name is in favorites.
    pub fn is_favorite(&self, name: &str) -> bool {
        self.favorites.iter().any(|f| f == name)
    }

    pub fn app_font(&self) -> Font {
        if self.dyslexia_font {
            Font::with_name(DYSLEXIA_FONT_NAME)
        } else {
            self.font
        }
    }

    pub fn app_font_with_weight(&self, weight: Weight) -> Font {
        if self.dyslexia_font {
            Font {
                weight,
                ..Font::with_name(DYSLEXIA_FONT_NAME)
            }
        } else {
            Font {
                weight,
                ..self.font
            }
        }
    }

    /// Compute the font scale factor from combined font_size + text_size_a11y settings.
    pub fn font_scale(&self) -> f32 {
        let base = match self.font_size.as_str() {
            "small" => 0.85,
            "large" => 1.2,
            _ => 1.0, // "default"
        };
        let a11y = match self.text_size_a11y.as_str() {
            "small" => 0.85,
            "large" => 1.2,
            "xlarge" => 1.4,
            _ => 1.0, // "default"
        };
        base * a11y
    }

    /// Return a font size scaled by the user's font preferences.
    pub fn sz(&self, base: u16) -> f32 {
        (base as f32 * self.font_scale()).round()
    }

    /// Duration of the sidebar slide animation.
    pub const SIDEBAR_ANIM_MS: f32 = 200.0;

    /// Compute the current sidebar indicator Y position (time-based easing).
    pub fn sidebar_indicator_pos(&self) -> f32 {
        match self.sidebar_indicator_start {
            Some(start) => {
                let elapsed = start.elapsed().as_secs_f32() * 1000.0;
                let t = (elapsed / Self::SIDEBAR_ANIM_MS).min(1.0);
                let eased = ease_out_cubic(t);
                self.sidebar_indicator_from
                    + (self.sidebar_indicator_target - self.sidebar_indicator_from) * eased
            }
            None => self.sidebar_indicator_target,
        }
    }

    /// Returns true if any animation is currently in-flight.
    /// Used to conditionally activate the 60fps tick subscription.
    pub fn has_active_animations(&self) -> bool {
        if !self.animations || self.reduce_motion {
            return false;
        }
        // Notification fade-in or fade-out in progress
        for notif in &self.notifications {
            if notif.fade_in < 1.0 || notif.removing {
                return true;
            }
        }
        // Progress bar interpolating toward target
        if let Some((_, target)) = &self.download_progress {
            if (self.progress_display - target).abs() > 0.001 {
                return true;
            }
        }
        // Sidebar indicator animating
        if self.sidebar_indicator_start.is_some() {
            return true;
        }
        false
    }
}

pub fn capitalize_platform(p: &str) -> String {
    match p.to_lowercase().as_str() {
        "windows" => "Windows".to_string(),
        "linux" => "Linux".to_string(),
        "macos" | "darwin" => "macOS".to_string(),
        "macos-x86" => "macOS (Intel)".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
impl App {
    /// A hermetic App for update-loop tests: no disk reads, no network, no
    /// animation/auto-dismiss tasks (reduce_motion on, animations off,
    /// auto_check_updates off). Tests that exercise disk-writing handlers must
    /// additionally redirect XDG dirs - see `update::tests::with_temp_dirs`.
    pub(crate) fn new_for_test() -> Self {
        Self {
            applications: Vec::new(),
            search_query: String::new(),
            sections: Vec::new(),
            selected_section: 0,
            status_message: String::new(),
            active_colony_repo: None,
            font: default_font(),
            github_state: GitHubState::Disconnected,
            show_github_menu: false,
            colony_repo_list: Vec::new(),
            notifications: Vec::new(),
            next_notification_id: 0,
            app_icons: std::collections::HashMap::new(),
            download_progress: None,
            download_abort: None,
            favorites: Vec::new(),
            confirm_uninstall: None,
            show_first_launch: false,
            welcome_step: 0,
            tutorial_bounds: Default::default(),
            show_settings: false,
            settings_category: 0,
            selected_theme: "gruvbox".into(),
            selected_variant: "dark".into(),
            selected_accent: "blue".into(),
            auto_accent: false,
            restore_session: true,
            default_view: "all".into(),
            language: "en".into(),
            auto_check_updates: false,
            font_size: "default".into(),
            animations: false,
            high_contrast: false,
            text_size_a11y: "default".into(),
            reduce_motion: true,
            keyboard_nav: true,
            dyslexia_font: false,
            scan_on_startup: false,
            is_scanning: false,
            is_downloading: false,
            is_checking_updates: false,
            is_fetching_repos: false,
            settings_expanded_sections: std::collections::HashSet::new(),
            detail_tab: DetailTab::ReadMe,
            detail_blocks: Vec::new(),
            detail_md_source: None,
            detail_is_placeholder: false,
            available_updates: std::collections::HashMap::new(),
            update_queue: Vec::new(),
            release_notes: std::collections::HashMap::new(),
            fetching_notes: std::collections::HashSet::new(),
            window_size: (1000.0, 700.0),
            window_save_gen: 0,
            launcher_update_available: None,
            is_checking_launcher_update: false,
            launcher_update_staged: None,
            launcher_system_managed: false,
            progress_display: 0.0,
            sidebar_indicator_from: 0.0,
            sidebar_indicator_target: 0.0,
            sidebar_indicator_start: None,
        }
    }
}
