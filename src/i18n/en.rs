//! English strings.
//!
//! One file per locale so a new string is a one-locale diff. The key sets of the
//! two locales must match exactly - `super::tests::fr_and_en_have_identical_key_sets`
//! fails otherwise.

use std::collections::HashMap;

pub(super) fn insert_all(strings: &mut HashMap<String, String>) {
    // English (default)
    // Sidebar
    strings.insert("categories".into(), "Categories".into());
    strings.insert("rescan".into(), "Rescan".into());

    // GitHub panel
    strings.insert("github_connect_desc".into(), "Connect to GitHub to detect Colony repos (colony.json) from the Project-Colony organization.".into());
    strings.insert("github_login".into(), "Sign in with GitHub".into());
    strings.insert(
        "github_public_api".into(),
        "Not connected: Public GitHub API (60 req/h)".into(),
    );
    strings.insert(
        "github_rate_limit".into(),
        "GitHub rate limit reached. Retry in {wait} seconds.".into(),
    );
    strings.insert(
        "github_enter_code".into(),
        "Enter this code on GitHub:".into(),
    );
    strings.insert(
        "github_copy_hint".into(),
        "Click to copy — Waiting for authorization...".into(),
    );
    strings.insert("github_connecting".into(), "Connecting...".into());
    strings.insert("github_connected".into(), "Connected".into());
    strings.insert(
        "github_repos_detected".into(),
        "{count} Colony repos detected".into(),
    );
    strings.insert(
        "github_no_repos".into(),
        "No repos with colony.json found.".into(),
    );
    strings.insert("github_refresh".into(), "Refresh repos".into());
    strings.insert("github_logout".into(), "Sign out".into());
    strings.insert("github_error".into(), "Error: {error}".into());
    strings.insert("github_retry".into(), "Retry".into());
    strings.insert(
        "github_disconnected".into(),
        "Disconnected from GitHub".into(),
    );

    // App grid
    strings.insert("no_apps_found".into(), "No applications found".into());
    strings.insert("search_placeholder".into(), "Search applications...".into());
    strings.insert("status_installed".into(), "Installed".into());
    strings.insert("status_get".into(), "Get".into());
    strings.insert("status_unavailable".into(), "Not available".into());
    strings.insert("status_update".into(), "Update".into());

    // Detail view
    strings.insert("back".into(), "Back".into());
    strings.insert("language_label".into(), "Language: {lang}".into());
    strings.insert("launch".into(), "Launch {name}".into());
    strings.insert("update".into(), "Update".into());
    strings.insert("download".into(), "Download".into());
    strings.insert("no_release".into(), "No release available".into());
    strings.insert("offered_version".into(), "Release {version}".into());
    strings.insert(
        "no_release_unrecognized".into(),
        "No installable release found - this app has not published assets Colony recognizes".into(),
    );
    strings.insert(
        "no_release_platform".into(),
        "Not available for your platform".into(),
    );

    // Status messages
    strings.insert("apps_found".into(), "{count} applications found".into());
    strings.insert("app_launched".into(), "Application launched.".into());
    strings.insert("installed".into(), "Installed: {path}".into());
    strings.insert("download_error".into(), "Download error: {error}".into());
    strings.insert("downloading".into(), "Downloading {file}…".into());
    strings.insert("no_release_for".into(), "No release for {platform}".into());
    strings.insert("uninstalled".into(), "{name} uninstalled.".into());
    strings.insert("launch_error".into(), "Cannot launch: {error}".into());
    strings.insert(
        "launch_error_empty".into(),
        "Cannot launch: empty command".into(),
    );
    strings.insert("uninstall_error".into(), "Uninstall error: {error}".into());

    // OAuth errors
    strings.insert("oauth_error".into(), "OAuth error: {error}".into());
    strings.insert(
        "oauth_device_expired".into(),
        "Timed out: GitHub authorization was not confirmed in time.".into(),
    );
    strings.insert(
        "oauth_device_failed".into(),
        "GitHub sign-in failed: {error} — {desc}".into(),
    );
    strings.insert("github_api_error".into(), "GitHub error: {error}".into());
    strings.insert("scan_error".into(), "Error: {error}".into());
    strings.insert("launch_error_msg".into(), "Launch error: {error}".into());
    strings.insert(
        "updates_available".into(),
        "{count} update(s) available: {names}".into(),
    );
    strings.insert(
        "launcher_relaunch_failed".into(),
        "Colony updated, but the new version would not start ({error}). The previous version is kept at {backup} - rename it back to recover.".into(),
    );
    strings.insert(
        "logout_incomplete".into(),
        "Signed out, but the stored credential could not be removed ({error}). Revoke it at github.com/settings/applications.".into(),
    );
    strings.insert(
        "update_skipped".into(),
        "Skipped {name}: no longer in the catalog, or no build for this platform".into(),
    );
    strings.insert(
        "update_check_failed".into(),
        "Could not check {count} app(s) for updates — they may not be up to date".into(),
    );

    // Sidebar section names (localized)
    strings.insert("section_all".into(), "All".into());
    strings.insert("section_favorites".into(), "Favorites".into());
    strings.insert("section_windows".into(), "Windows".into());
    strings.insert("section_linux".into(), "Linux".into());
    strings.insert("section_macos".into(), "macOS".into());
    strings.insert("section_development".into(), "Development".into());
    strings.insert("section_graphics".into(), "Graphics".into());
    strings.insert("section_network".into(), "Network".into());
    strings.insert("section_office".into(), "Office".into());
    strings.insert("section_multimedia".into(), "Multimedia".into());
    strings.insert("section_system".into(), "System".into());
    strings.insert("section_utilities".into(), "Utilities".into());
    strings.insert("section_games".into(), "Games".into());
    strings.insert("section_other".into(), "Other".into());

    // Thread errors
    strings.insert(
        "error_thread_panic".into(),
        "Internal error: background thread panicked".into(),
    );

    // Download cancellation
    strings.insert("download_cancelled".into(), "Download cancelled".into());

    // Uninstall confirmation
    strings.insert(
        "confirm_uninstall".into(),
        "Are you sure you want to uninstall \"{name}\"? This action cannot be undone.".into(),
    );
    strings.insert("cancel".into(), "Cancel".into());
    strings.insert("confirm_delete".into(), "Uninstall".into());

    // Favorites
    strings.insert("add_favorite".into(), "Add to favorites".into());
    strings.insert("remove_favorite".into(), "Remove from favorites".into());

    // First launch — carousel (3 steps)
    strings.insert("welcome_title".into(), "Welcome to Colony".into());
    strings.insert("welcome_desc".into(), "The centralized launcher for the Project-Colony ecosystem. Discover, install and launch apps in one click.".into());
    // Step 1 — interface tour
    strings.insert(
        "welcome_step1_title".into(),
        "The interface, in 3 zones".into(),
    );
    strings.insert("welcome_step1_tip1_title".into(), "Sidebar".into());
    strings.insert(
        "welcome_step1_tip1_desc".into(),
        "Filter by category or origin (Colony / system apps).".into(),
    );
    strings.insert("welcome_step1_tip2_title".into(), "Search".into());
    strings.insert(
        "welcome_step1_tip2_desc".into(),
        "Type an app name in the top bar to filter instantly.".into(),
    );
    strings.insert("welcome_step1_tip3_title".into(), "Detail".into());
    strings.insert(
        "welcome_step1_tip3_desc".into(),
        "Click any app to read its README, changelog and install it.".into(),
    );
    // Step 2 — GitHub + ready
    strings.insert(
        "welcome_step2_title".into(),
        "Connect GitHub (optional)".into(),
    );
    strings.insert("welcome_step2_desc".into(), "Without an account: 60 GitHub requests per hour. With an account: 5000/h + access to your private repos. Recommended if you plan to browse a lot.".into());
    strings.insert(
        "welcome_step2_hint1".into(),
        "\u{f005}  Favorites (⭐) for quick access".into(),
    );
    strings.insert(
        "welcome_step2_hint2".into(),
        "\u{f53f}  24 theme families in the preferences".into(),
    );
    strings.insert(
        "welcome_step2_hint3".into(),
        "\u{f059}  Full tutorial + FAQ on GitHub".into(),
    );
    // Navigation
    strings.insert("welcome_start".into(), "Let's go!".into());
    strings.insert("welcome_next".into(), "Next".into());
    strings.insert("welcome_back".into(), "Back".into());
    strings.insert("welcome_skip".into(), "Skip".into());
    strings.insert("welcome_connect_now".into(), "Connect now".into());
    strings.insert("welcome_later".into(), "Later".into());

    // Guided tutorial (spotlight over real UI)
    strings.insert("tut_sidebar_title".into(), "Categories".into());
    strings.insert("tut_sidebar_desc".into(), "Filter your apps by type — games, tools, favorites — or by origin (Colony ecosystem vs. system). The sidebar stays visible at all times.".into());
    strings.insert("tut_search_title".into(), "Search".into());
    strings.insert(
        "tut_search_desc".into(),
        "Type an app name here to find it instantly, regardless of the selected category.".into(),
    );
    strings.insert("tut_grid_title".into(), "Your applications".into());
    strings.insert("tut_grid_desc".into(), "These are your installed apps plus the Colony apps available to install. Click a card to read its README, changelog and install in one click.".into());
    strings.insert(
        "tut_github_title".into(),
        "GitHub connection (optional)".into(),
    );
    strings.insert("tut_github_desc".into(), "Without an account: 60 requests/h. With one: 5000/h + private repo access. Recommended if you plan to browse a lot. The Rescan button below refreshes the system scan.".into());
    strings.insert("tut_finish_title".into(), "You're all set!".into());
    strings.insert("tut_finish_desc".into(), "The gear icon next to the title opens Preferences: 24 theme families, keyboard shortcuts, accessibility. Enjoy Colony!".into());

    // Loading / async feedback
    strings.insert("loading".into(), "Loading...".into());
    strings.insert("scanning".into(), "Scanning...".into());
    strings.insert("checking_updates".into(), "Checking for updates...".into());
    strings.insert("syncing_repos".into(), "Syncing repositories...".into());
    strings.insert("no_results_for".into(), "No results for \"{query}\"".into());
    strings.insert(
        "n_results_found".into(),
        "{count} result(s) for \"{query}\"".into(),
    );
    strings.insert("theme_applied".into(), "Theme applied.".into());

    // Keyboard shortcuts
    strings.insert("shortcuts_title".into(), "Keyboard shortcuts".into());
    strings.insert("shortcut_esc".into(), "Esc — Close active panel".into());
    strings.insert(
        "shortcut_tab".into(),
        "Tab / Shift+Tab — Navigate categories".into(),
    );
    strings.insert("shortcut_arrows".into(), "↑ ↓ — Navigate settings".into());
    strings.insert(
        "shortcut_enter".into(),
        "Enter — Open first visible item".into(),
    );
    strings.insert(
        "shortcut_pageupdown".into(),
        "Page Up/Down — Fast navigation in settings".into(),
    );

    // Tooltips / hints
    strings.insert("hint_settings".into(), "Open preferences".into());
    strings.insert("hint_search".into(), "Type to filter applications".into());
    strings.insert(
        "hint_favorites".into(),
        "Click the star to add to favorites".into(),
    );
    strings.insert(
        "hint_keyboard".into(),
        "Use Tab and arrow keys to navigate".into(),
    );

    // Settings
    strings.insert("settings_title".into(), "Preferences".into());
    strings.insert("settings_close".into(), "Close".into());
    strings.insert("settings_cat_general".into(), "General".into());
    strings.insert("settings_cat_appearance".into(), "Appearance".into());
    strings.insert("settings_cat_accessibility".into(), "Accessibility".into());
    strings.insert("settings_cat_storage".into(), "Storage".into());
    strings.insert("settings_cat_about".into(), "About".into());
    strings.insert("settings_cat_shortcuts".into(), "Shortcuts".into());
    // General
    strings.insert("settings_general_title".into(), "General settings".into());
    strings.insert(
        "settings_general_desc".into(),
        "Preferences are saved automatically.".into(),
    );
    // Startup
    strings.insert("settings_section_startup".into(), "Startup".into());
    strings.insert(
        "settings_startup_section_desc".into(),
        "Manage Colony startup and session restoration.".into(),
    );
    strings.insert(
        "settings_restore_session".into(),
        "Restore last session".into(),
    );
    strings.insert(
        "settings_restore_session_desc".into(),
        "Category and screen from last usage.".into(),
    );
    strings.insert("settings_default_view".into(), "Open on".into());
    strings.insert(
        "settings_default_view_desc".into(),
        "Choose the default screen.".into(),
    );
    strings.insert("settings_default_view_all".into(), "All".into());
    strings.insert("settings_default_view_favorites".into(), "Favorites".into());
    strings.insert("settings_default_view_recent".into(), "Recent".into());
    strings.insert("settings_close_behavior".into(), "Close behavior".into());
    strings.insert(
        "settings_close_behavior_desc".into(),
        "Choose action on close.".into(),
    );
    strings.insert("settings_close_quit".into(), "Quit".into());
    strings.insert("settings_close_tray".into(), "Minimize to tray".into());
    // Language
    strings.insert("settings_section_language".into(), "Language".into());
    strings.insert(
        "settings_language_desc".into(),
        "Customize the interface and time format.".into(),
    );
    strings.insert(
        "settings_current_language".into(),
        "Interface language".into(),
    );
    strings.insert(
        "settings_current_language_desc".into(),
        "Synced with system.".into(),
    );
    strings.insert("settings_time_format".into(), "Time format".into());
    strings.insert(
        "settings_time_format_desc".into(),
        "Format used in the application.".into(),
    );
    // Updates
    strings.insert("settings_section_updates".into(), "Updates".into());
    strings.insert(
        "settings_updates_desc".into(),
        "Manage update checking and channel.".into(),
    );
    strings.insert(
        "settings_auto_check_updates".into(),
        "Check automatically".into(),
    );
    strings.insert(
        "settings_auto_check_updates_desc".into(),
        "Check for new versions on launch.".into(),
    );
    strings.insert("settings_update_channel".into(), "Channel".into());
    strings.insert(
        "settings_update_channel_desc".into(),
        "Choose version stability.".into(),
    );
    strings.insert(
        "settings_auto_install_updates".into(),
        "Install automatically".into(),
    );
    strings.insert(
        "settings_auto_install_updates_desc".into(),
        "Install updates in background.".into(),
    );
    strings.insert("settings_check_updates".into(), "Check for updates".into());
    // Privacy
    strings.insert("settings_section_privacy".into(), "Privacy".into());
    strings.insert(
        "settings_privacy_desc".into(),
        "Choose data shared with Colony.".into(),
    );
    strings.insert("settings_error_reports".into(), "Send error reports".into());
    strings.insert(
        "settings_error_reports_desc".into(),
        "Helps improve stability.".into(),
    );
    strings.insert(
        "settings_usage_stats".into(),
        "Anonymous usage statistics".into(),
    );
    strings.insert(
        "settings_usage_stats_desc".into(),
        "Helps understand Colony usage.".into(),
    );
    // Appearance
    strings.insert(
        "settings_appearance_title".into(),
        "Appearance settings".into(),
    );
    strings.insert(
        "settings_appearance_desc".into(),
        "Adjust theme, accents and visual effects.".into(),
    );
    strings.insert("settings_section_theme".into(), "Theme".into());
    strings.insert(
        "settings_theme_desc".into(),
        "Choose the interface theme.".into(),
    );
    strings.insert("settings_theme_current".into(), "Current theme".into());
    strings.insert(
        "settings_theme_current_desc".into(),
        "Overall application appearance.".into(),
    );
    strings.insert("settings_theme_dark".into(), "Dark".into());
    // Theme families
    strings.insert("settings_theme_catppuccin".into(), "Catppuccin".into());
    strings.insert("settings_theme_catppuccin_latte".into(), "Latte".into());
    strings.insert("settings_theme_catppuccin_frappe".into(), "Frappé".into());
    strings.insert(
        "settings_theme_catppuccin_macchiato".into(),
        "Macchiato".into(),
    );
    strings.insert("settings_theme_catppuccin_mocha".into(), "Mocha".into());
    strings.insert("settings_theme_gruvbox".into(), "Gruvbox".into());
    strings.insert("settings_theme_light".into(), "Light mode".into());
    strings.insert("settings_theme_dark_mode".into(), "Dark mode".into());
    strings.insert("settings_theme_everblush".into(), "Everblush".into());
    strings.insert("settings_theme_kanagawa".into(), "Kanagawa".into());
    strings.insert(
        "settings_theme_kanagawa_journal".into(),
        "Journal mode".into(),
    );
    // New theme families
    strings.insert("settings_theme_nord".into(), "Nord".into());
    strings.insert("settings_theme_dracula".into(), "Dracula".into());
    strings.insert("settings_theme_solarized".into(), "Solarized".into());
    strings.insert("settings_theme_tokyonight".into(), "Tokyo Night".into());
    strings.insert("settings_theme_tokyonight_night".into(), "Night".into());
    strings.insert("settings_theme_tokyonight_day".into(), "Day".into());
    strings.insert("settings_theme_rosepine".into(), "Rosé Pine".into());
    strings.insert("settings_theme_rosepine_main".into(), "Main".into());
    strings.insert("settings_theme_rosepine_moon".into(), "Moon".into());
    strings.insert("settings_theme_rosepine_dawn".into(), "Dawn".into());
    strings.insert("settings_theme_onedark".into(), "One Dark".into());
    strings.insert("settings_theme_monokai".into(), "Monokai Pro".into());
    strings.insert("settings_theme_monokai_pro".into(), "Pro".into());
    strings.insert("settings_theme_monokai_classic".into(), "Classic".into());
    strings.insert("settings_theme_monokai_spectrum".into(), "Spectrum".into());
    strings.insert("settings_theme_ayu".into(), "Ayu".into());
    strings.insert("settings_theme_ayu_mirage".into(), "Mirage".into());
    strings.insert("settings_theme_everforest".into(), "Everforest".into());
    strings.insert("settings_theme_material".into(), "Material".into());
    strings.insert("settings_theme_material_oceanic".into(), "Oceanic".into());
    strings.insert(
        "settings_theme_material_palenight".into(),
        "Palenight".into(),
    );
    strings.insert(
        "settings_theme_material_deepocean".into(),
        "Deep Ocean".into(),
    );
    strings.insert("settings_theme_flexoki".into(), "Flexoki".into());
    strings.insert("settings_theme_nightfox".into(), "Nightfox".into());
    strings.insert("settings_theme_nightfox_nightfox".into(), "Nightfox".into());
    strings.insert("settings_theme_nightfox_dawnfox".into(), "Dawnfox".into());
    strings.insert("settings_theme_sonokai".into(), "Sonokai".into());
    strings.insert("settings_theme_sonokai_default".into(), "Default".into());
    strings.insert("settings_theme_oxocarbon".into(), "Oxocarbon".into());
    strings.insert("settings_theme_nightowl".into(), "Night Owl".into());
    strings.insert("settings_theme_iceberg".into(), "Iceberg".into());
    strings.insert("settings_theme_horizon".into(), "Horizon".into());
    strings.insert("settings_theme_melange".into(), "Melange".into());
    strings.insert("settings_theme_synthwave".into(), "Synthwave '84".into());
    strings.insert("settings_theme_modus".into(), "Modus".into());
    strings.insert("settings_theme_modus_operandi".into(), "Operandi".into());
    strings.insert("settings_theme_modus_vivendi".into(), "Vivendi".into());
    strings.insert(
        "settings_theme_stellar_blade".into(),
        "Stellar Blade".into(),
    );
    strings.insert("settings_theme_stellar_blade_eve".into(), "EVE".into());
    strings.insert("settings_theme_stellar_blade_tachy".into(), "Tachy".into());
    strings.insert("settings_theme_stellar_blade_lily".into(), "Lily".into());
    strings.insert("settings_theme_stellar_blade_enya".into(), "Enya".into());
    strings.insert("settings_theme_stellar_blade_kaya".into(), "Kaya".into());
    // Colors & accents
    strings.insert("settings_section_colors".into(), "Colors & accents".into());
    strings.insert(
        "settings_colors_desc".into(),
        "Customize the interface accent color.".into(),
    );
    strings.insert("settings_accent_color".into(), "Accent color".into());
    strings.insert(
        "settings_accent_color_desc".into(),
        "Color used for interactive elements.".into(),
    );
    strings.insert("settings_accent_red".into(), "Red".into());
    strings.insert("settings_accent_orange".into(), "Orange".into());
    strings.insert("settings_accent_yellow".into(), "Yellow".into());
    strings.insert("settings_accent_green".into(), "Green".into());
    strings.insert("settings_accent_blue".into(), "Blue".into());
    strings.insert("settings_accent_indigo".into(), "Indigo".into());
    strings.insert("settings_accent_violet".into(), "Violet".into());
    strings.insert("settings_accent_amber".into(), "Amber".into());
    strings.insert(
        "settings_auto_accent".into(),
        "Auto accent from background".into(),
    );
    strings.insert(
        "settings_auto_accent_desc".into(),
        "Automatically adapts accent to backgrounds.".into(),
    );
    strings.insert("settings_enabled_label".into(), "Enabled".into());
    strings.insert("settings_disabled_label".into(), "Disabled".into());
    strings.insert("settings_section_typography".into(), "Typography".into());
    strings.insert(
        "settings_typography_desc".into(),
        "Configure font and text size.".into(),
    );
    strings.insert("settings_font".into(), "Font".into());
    strings.insert(
        "settings_font_desc".into(),
        "Font used in the interface.".into(),
    );
    strings.insert("settings_font_size".into(), "Text size".into());
    strings.insert("settings_font_size_desc".into(), "Base text size.".into());
    strings.insert("settings_font_size_default".into(), "Default".into());
    strings.insert("settings_font_size_small".into(), "Small".into());
    strings.insert("settings_font_size_large".into(), "Large".into());
    strings.insert("settings_font_size_xlarge".into(), "Extra large".into());
    strings.insert(
        "settings_section_effects".into(),
        "Backgrounds & effects".into(),
    );
    strings.insert(
        "settings_effects_desc".into(),
        "Manage animations and visual effects.".into(),
    );
    strings.insert("settings_animations".into(), "Animations".into());
    strings.insert(
        "settings_animations_desc".into(),
        "Enable animated transitions.".into(),
    );
    strings.insert("settings_section_preview".into(), "Preview".into());
    strings.insert("settings_preview_card".into(), "Preview card".into());
    strings.insert(
        "settings_preview_summary".into(),
        "Theme: Dark · Accent: Blue · Text: Default · Effects: Enabled".into(),
    );
    // Accessibility
    strings.insert(
        "settings_accessibility_title".into(),
        "Accessibility settings".into(),
    );
    strings.insert(
        "settings_accessibility_desc".into(),
        "Improve reading, navigation and media playback.".into(),
    );
    strings.insert("settings_section_vision".into(), "Vision".into());
    strings.insert(
        "settings_vision_desc".into(),
        "Options to improve readability.".into(),
    );
    strings.insert("settings_high_contrast".into(), "High contrast".into());
    strings.insert(
        "settings_high_contrast_desc".into(),
        "Increase contrast of elements.".into(),
    );
    strings.insert("settings_disabled".into(), "Disabled".into());
    strings.insert("settings_text_size_a11y".into(), "Text size".into());
    strings.insert(
        "settings_text_size_a11y_desc".into(),
        "Adjust text size for comfort.".into(),
    );
    strings.insert("settings_section_motion".into(), "Motion".into());
    strings.insert(
        "settings_motion_desc".into(),
        "Reduce animations for comfort.".into(),
    );
    strings.insert("settings_reduce_motion".into(), "Reduce motion".into());
    strings.insert(
        "settings_reduce_motion_desc".into(),
        "Limit transitions and movements.".into(),
    );
    strings.insert(
        "settings_section_navigation".into(),
        "Navigation & interaction".into(),
    );
    strings.insert(
        "settings_navigation_desc".into(),
        "Keyboard navigation and interaction options.".into(),
    );
    strings.insert("settings_keyboard_nav".into(), "Keyboard navigation".into());
    strings.insert(
        "settings_keyboard_nav_desc".into(),
        "Navigate with Tab and arrow keys.".into(),
    );
    strings.insert("settings_section_reading".into(), "Reading".into());
    strings.insert(
        "settings_reading_desc".into(),
        "Reading comfort options.".into(),
    );
    strings.insert("settings_dyslexia_font".into(), "Dyslexia font".into());
    strings.insert(
        "settings_dyslexia_font_desc".into(),
        "Use a font adapted for dyslexia.".into(),
    );
    // Storage
    strings.insert("settings_storage_title".into(), "Storage".into());
    strings.insert(
        "settings_storage_desc".into(),
        "Manage application locations and cache.".into(),
    );
    strings.insert("settings_section_scan".into(), "Scan".into());
    strings.insert(
        "settings_scan_desc".into(),
        "Configure directories scanned at startup.".into(),
    );
    strings.insert("settings_scan_dirs".into(), "Scan directories".into());
    strings.insert(
        "settings_scan_dirs_desc".into(),
        "Directories scanned for applications.".into(),
    );
    strings.insert("settings_scan_dirs_value".into(), "Default".into());
    strings.insert("settings_startup".into(), "Scan on startup".into());
    strings.insert(
        "settings_startup_desc".into(),
        "Updates the library at startup.".into(),
    );
    strings.insert("settings_enabled".into(), "Enabled".into());
    strings.insert("settings_section_install".into(), "Installation".into());
    strings.insert("settings_local_apps".into(), "Local applications".into());
    strings.insert("settings_colony_repos".into(), "Colony repos".into());
    strings.insert("settings_favorites".into(), "Favorites".into());
    // Placeholders
    strings.insert("settings_coming_soon".into(), "Coming soon".into());
    // About
    strings.insert("settings_about_title".into(), "About Colony".into());
    strings.insert("settings_about".into(), "About".into());
    strings.insert("settings_version".into(), "Colony v0.1.0".into());
    // Launcher self-update
    strings.insert(
        "launcher_update_available".into(),
        "Colony {version} is available!".into(),
    );
    strings.insert(
        "launcher_update_available_short".into(),
        "\u{f0aa}  Update {version}".into(),
    );
    strings.insert(
        "launcher_update_ready".into(),
        "Update ready. Click to restart Colony.".into(),
    );
    strings.insert(
        "launcher_restart_to_update".into(),
        "\u{f021}  Restart to update".into(),
    );
    strings.insert(
        "launcher_download_update".into(),
        "Download update {version}".into(),
    );
    strings.insert(
        "launcher_update_failed".into(),
        "Update failed: {error}".into(),
    );
    strings.insert("check_launcher_updates".into(), "Check for updates".into());
    strings.insert("launcher_up_to_date".into(), "Colony is up to date".into());
    strings.insert("update_all".into(), "Update all ({count})".into());
    strings.insert("whats_new".into(), "What's new in {version}".into());
    strings.insert("view_on_github".into(), "View on GitHub".into());
    strings.insert("installed_version".into(), "Installed: {version}".into());
    strings.insert("launch_action".into(), "Launch".into());
    strings.insert("section_security".into(), "Security".into());
    strings.insert("language_changed".into(), "Language changed".into());
    strings.insert("clear_caches".into(), "Clear store caches".into());
    strings.insert("clear_caches_desc".into(), "Removes cached descriptions and icons (they re-download on the next refresh). Installed applications are not touched.".into());
    strings.insert("caches_cleared".into(), "{count} cache(s) removed".into());
    strings.insert("launcher_update_system_managed".into(), "Update {version} is available - this install is managed by the package manager, update via 'pacman -Syu' (colony-bin)".into());
    // Detail tabs
    strings.insert("tab_readme".into(), "ReadMe".into());
    strings.insert("tab_license".into(), "License".into());
    strings.insert("tab_changelog".into(), "Changelog".into());
    strings.insert("tab_loading".into(), "Loading...".into());
    strings.insert("tab_not_available".into(), "Not available".into());
}
