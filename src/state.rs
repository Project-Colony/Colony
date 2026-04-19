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
    iced::Color { a: color.a * alpha, ..color }
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
    Connected {
        session: OAuthSession,
        repos: Vec<ColonyRepo>,
    },
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
    pub active_colony_repo: Option<usize>,
    pub font: Font,
    // GitHub / OAuth
    pub github_state: GitHubState,
    pub show_github_menu: bool,
    // Notifications
    pub notifications: Vec<Notification>,
    pub next_notification_id: u64,
    // Download progress
    pub download_progress: Option<(String, f32)>, // (filename, 0.0..1.0)
    // Favorites
    pub favorites: Vec<String>,
    // Uninstall confirmation
    pub confirm_uninstall: Option<String>, // repo_name pending confirmation
    // First launch
    pub show_first_launch: bool,
    /// Index (0..=2) of the active step in the first-launch carousel.
    pub welcome_step: u8,
    // Settings
    pub show_settings: bool,
    pub settings_category: usize,
    // Appearance
    pub selected_theme: String,        // e.g. "gruvbox"
    pub selected_variant: String,      // e.g. "dark"
    pub selected_accent: String,       // e.g. "blue"
    pub auto_accent: bool,
    // General preferences
    pub auto_scan: bool,
    pub restore_session: bool,
    pub default_view: String,          // "all", "favorites"
    pub language: String,              // "fr", "en"
    pub auto_check_updates: bool,
    // Appearance extras
    pub font_size: String,             // "small", "default", "large"
    pub animations: bool,
    // Accessibility
    pub high_contrast: bool,
    pub text_size_a11y: String,        // "small", "default", "large", "xlarge"
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
    // Launcher self-update
    pub launcher_update_available: Option<(String, String)>,  // (tag, asset_filename)
    pub is_checking_launcher_update: bool,
    pub launcher_update_staged: Option<std::path::PathBuf>,
    // Animation state
    pub progress_display: f32,             // smoothly interpolated progress bar value
    pub sidebar_indicator_from: f32,       // start Y position of current animation
    pub sidebar_indicator_target: f32,     // target Y position
    pub sidebar_indicator_start: Option<Instant>, // when the animation started (None = idle)
}

impl App {
    /// Get the list of Colony repos from GitHub state.
    pub fn colony_repos(&self) -> &[ColonyRepo] {
        if let GitHubState::Connected { repos, .. } = &self.github_state {
            repos
        } else {
            &[]
        }
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
    pub fn filtered_colony_repos(&self) -> Vec<(usize, &ColonyRepo)> {
        let query = self.search_query.to_lowercase();
        let selected_section = self.sections.get(self.selected_section);
        let is_favorites = selected_section.map(|s| s.is_favorites).unwrap_or(false);

        self.colony_repos()
            .iter()
            .enumerate()
            .filter(|(_, repo)| {
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
                repo.name.to_lowercase().contains(&query)
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
            Font { weight, ..Font::with_name(DYSLEXIA_FONT_NAME) }
        } else {
            Font { weight, ..self.font }
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
                self.sidebar_indicator_from + (self.sidebar_indicator_target - self.sidebar_indicator_from) * eased
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
