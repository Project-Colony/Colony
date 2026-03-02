use iced::Color;
use std::sync::RwLock;

/// Runtime theme palette — all semantic UI colors.
#[derive(Debug, Clone, Copy)]
pub struct ThemePalette {
    // --- Backgrounds ---
    pub bg_primary: Color,
    pub bg_sidebar: Color,
    pub bg_card: Color,
    pub bg_card_hover: Color,
    pub bg_card_pressed: Color,
    pub bg_selected: Color,
    pub bg_input: Color,
    pub bg_progress: Color,

    // --- Text ---
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub text_dim: Color,
    pub text_dimmer: Color,
    pub text_dimmest: Color,
    pub text_placeholder: Color,

    // --- Accent ---
    pub accent_blue: Color,
    pub accent_icon: Color,
    pub accent_progress: Color,

    // --- Buttons ---
    pub btn_default: Color,
    pub btn_hover: Color,
    pub btn_pressed: Color,

    // --- Success ---
    pub success: Color,
    pub success_bg: Color,
    pub btn_success: Color,
    pub btn_success_hover: Color,
    pub btn_success_pressed: Color,

    // --- Warning ---
    pub warning: Color,
    pub warning_bg: Color,

    // --- Error ---
    pub error: Color,
    pub error_light: Color,
    pub error_bg: Color,
    pub btn_danger_bg: Color,
    pub btn_danger_hover: Color,
    pub btn_trash_hover: Color,
    pub btn_trash_pressed: Color,

    // --- Modal ---
    pub bg_modal_section: Color,
    pub border_subtle: Color,
    pub divider: Color,
}

// ── Global active palette ──

static ACTIVE_PALETTE: RwLock<ThemePalette> = RwLock::new(ThemePalette::GRUVBOX_DARK);

/// User-chosen accent color override. `None` means "auto" (use theme default).
static ACTIVE_ACCENT: RwLock<Option<Color>> = RwLock::new(None);

/// Set the user accent override. Pass `None` for auto (theme default).
pub fn set_active_accent(color: Option<Color>) {
    *ACTIVE_ACCENT.write().unwrap() = color;
}

/// Resolve the effective accent color: user override or theme default.
pub fn effective_accent() -> Color {
    ACTIVE_ACCENT.read().unwrap().unwrap_or_else(|| active_palette().accent_blue)
}

/// Convert an accent key to its Color, or None for "auto".
pub fn accent_key_to_color(key: &str) -> Option<Color> {
    let hex: u32 = match key {
        "red"    => 0xE05555,
        "orange" => 0xE0855A,
        "yellow" => 0xC8A832,
        "green"  => 0x55B87A,
        "blue"   => 0x6B8BD6,
        "indigo" => 0x7B6BD6,
        "violet" => 0xB06BD6,
        "amber"  => 0xD4A030,
        _ => return None,
    };
    Some(Color {
        r: ((hex >> 16) & 0xFF) as f32 / 255.0,
        g: ((hex >> 8) & 0xFF) as f32 / 255.0,
        b: (hex & 0xFF) as f32 / 255.0,
        a: 1.0,
    })
}

/// Set the active palette based on theme + variant keys.
pub fn set_active_theme(theme: &str, variant: &str) {
    let palette = match (theme, variant) {
        // Catppuccin
        ("catppuccin", "latte")      => ThemePalette::CATPPUCCIN_LATTE,
        ("catppuccin", "frappe")     => ThemePalette::CATPPUCCIN_FRAPPE,
        ("catppuccin", "macchiato")  => ThemePalette::CATPPUCCIN_MACCHIATO,
        ("catppuccin", "mocha")      => ThemePalette::CATPPUCCIN_MOCHA,
        // Gruvbox
        ("gruvbox", "light")         => ThemePalette::GRUVBOX_LIGHT,
        ("gruvbox", "dark")          => ThemePalette::GRUVBOX_DARK,
        // Everblush
        ("everblush", "light")       => ThemePalette::EVERBLUSH_LIGHT,
        ("everblush", "dark")        => ThemePalette::EVERBLUSH_DARK,
        // Kanagawa
        ("kanagawa", "light")        => ThemePalette::KANAGAWA_LOTUS,
        ("kanagawa", "dark")         => ThemePalette::KANAGAWA_WAVE,
        ("kanagawa", "journal")      => ThemePalette::KANAGAWA_DRAGON,
        // Nord
        ("nord", "dark")             => ThemePalette::NORD_DARK,
        ("nord", "light")            => ThemePalette::NORD_LIGHT,
        // Dracula
        ("dracula", "dark")          => ThemePalette::DRACULA_DARK,
        ("dracula", "light")         => ThemePalette::DRACULA_LIGHT,
        // Solarized
        ("solarized", "dark")        => ThemePalette::SOLARIZED_DARK,
        ("solarized", "light")       => ThemePalette::SOLARIZED_LIGHT,
        // Tokyo Night
        ("tokyonight", "night")      => ThemePalette::TOKYONIGHT_NIGHT,
        ("tokyonight", "day")        => ThemePalette::TOKYONIGHT_DAY,
        // Rosé Pine
        ("rosepine", "main")         => ThemePalette::ROSEPINE_MAIN,
        ("rosepine", "moon")         => ThemePalette::ROSEPINE_MOON,
        ("rosepine", "dawn")         => ThemePalette::ROSEPINE_DAWN,
        // One Dark
        ("onedark", "dark")          => ThemePalette::ONEDARK_DARK,
        ("onedark", "light")         => ThemePalette::ONEDARK_LIGHT,
        // Monokai Pro
        ("monokai", "pro")           => ThemePalette::MONOKAI_PRO,
        ("monokai", "classic")       => ThemePalette::MONOKAI_CLASSIC,
        ("monokai", "spectrum")      => ThemePalette::MONOKAI_SPECTRUM,
        // Ayu
        ("ayu", "dark")              => ThemePalette::AYU_DARK,
        ("ayu", "mirage")            => ThemePalette::AYU_MIRAGE,
        ("ayu", "light")             => ThemePalette::AYU_LIGHT,
        // Everforest
        ("everforest", "dark")       => ThemePalette::EVERFOREST_DARK,
        ("everforest", "light")      => ThemePalette::EVERFOREST_LIGHT,
        // Material
        ("material", "oceanic")      => ThemePalette::MATERIAL_OCEANIC,
        ("material", "palenight")    => ThemePalette::MATERIAL_PALENIGHT,
        ("material", "deepocean")    => ThemePalette::MATERIAL_DEEPOCEAN,
        // Flexoki
        ("flexoki", "dark")          => ThemePalette::FLEXOKI_DARK,
        ("flexoki", "light")         => ThemePalette::FLEXOKI_LIGHT,
        // Nightfox
        ("nightfox", "nightfox")     => ThemePalette::NIGHTFOX,
        ("nightfox", "dawnfox")      => ThemePalette::DAWNFOX,
        // Sonokai
        ("sonokai", "default")       => ThemePalette::SONOKAI_DEFAULT,
        // Oxocarbon
        ("oxocarbon", "dark")        => ThemePalette::OXOCARBON_DARK,
        ("oxocarbon", "light")       => ThemePalette::OXOCARBON_LIGHT,
        // Night Owl
        ("nightowl", "dark")         => ThemePalette::NIGHTOWL_DARK,
        ("nightowl", "light")        => ThemePalette::NIGHTOWL_LIGHT,
        // Iceberg
        ("iceberg", "dark")          => ThemePalette::ICEBERG_DARK,
        ("iceberg", "light")         => ThemePalette::ICEBERG_LIGHT,
        // Horizon
        ("horizon", "dark")          => ThemePalette::HORIZON_DARK,
        // Melange
        ("melange", "dark")          => ThemePalette::MELANGE_DARK,
        ("melange", "light")         => ThemePalette::MELANGE_LIGHT,
        // Synthwave '84
        ("synthwave", "dark")        => ThemePalette::SYNTHWAVE_DARK,
        // Modus
        ("modus", "operandi")        => ThemePalette::MODUS_OPERANDI,
        ("modus", "vivendi")         => ThemePalette::MODUS_VIVENDI,
        // Fallback
        _ => ThemePalette::GRUVBOX_DARK,
    };
    *ACTIVE_PALETTE.write().unwrap() = palette;
}

/// High-contrast mode flag.
static HIGH_CONTRAST: RwLock<bool> = RwLock::new(false);

/// Set high-contrast mode.
pub fn set_high_contrast(enabled: bool) {
    *HIGH_CONTRAST.write().unwrap() = enabled;
}

/// Check if high contrast is active.
pub fn is_high_contrast() -> bool {
    *HIGH_CONTRAST.read().unwrap()
}

/// Convenience accessor — read the current palette.
pub fn active_palette() -> ThemePalette {
    let base = *ACTIVE_PALETTE.read().unwrap();
    if is_high_contrast() {
        base.with_high_contrast()
    } else {
        base
    }
}

// ── Public façade (drop-in replacement for the old `Palette::CONST` API) ──

pub struct Palette;

#[allow(non_snake_case, dead_code)]
impl Palette {
    // Backgrounds
    pub fn BG_PRIMARY()       -> Color { active_palette().bg_primary }
    pub fn BG_SIDEBAR()       -> Color { active_palette().bg_sidebar }
    pub fn BG_CARD()          -> Color { active_palette().bg_card }
    pub fn BG_CARD_HOVER()    -> Color { active_palette().bg_card_hover }
    pub fn BG_CARD_PRESSED()  -> Color { active_palette().bg_card_pressed }
    pub fn BG_SELECTED()      -> Color { active_palette().bg_selected }
    pub fn BG_INPUT()         -> Color { active_palette().bg_input }
    pub fn BG_PROGRESS()      -> Color { active_palette().bg_progress }

    // Text
    pub fn TEXT_PRIMARY()     -> Color { active_palette().text_primary }
    pub fn TEXT_SECONDARY()   -> Color { active_palette().text_secondary }
    pub fn TEXT_MUTED()       -> Color { active_palette().text_muted }
    pub fn TEXT_DIM()         -> Color { active_palette().text_dim }
    pub fn TEXT_DIMMER()      -> Color { active_palette().text_dimmer }
    pub fn TEXT_DIMMEST()     -> Color { active_palette().text_dimmest }
    pub fn TEXT_PLACEHOLDER() -> Color { active_palette().text_placeholder }

    // Accent
    /// User-selected accent (or theme default if auto).
    pub fn ACCENT()           -> Color { effective_accent() }
    pub fn ACCENT_ICON()      -> Color { active_palette().accent_icon }
    pub fn ACCENT_PROGRESS()  -> Color { active_palette().accent_progress }

    // Buttons
    pub fn BTN_DEFAULT()      -> Color { active_palette().btn_default }
    pub fn BTN_HOVER()        -> Color { active_palette().btn_hover }
    pub fn BTN_PRESSED()      -> Color { active_palette().btn_pressed }

    // Success
    pub fn SUCCESS()            -> Color { active_palette().success }
    pub fn SUCCESS_BG()         -> Color { active_palette().success_bg }
    pub fn BTN_SUCCESS()        -> Color { active_palette().btn_success }
    pub fn BTN_SUCCESS_HOVER()  -> Color { active_palette().btn_success_hover }
    pub fn BTN_SUCCESS_PRESSED()-> Color { active_palette().btn_success_pressed }

    // Warning
    pub fn WARNING()    -> Color { active_palette().warning }
    pub fn WARNING_BG() -> Color { active_palette().warning_bg }

    // Error
    pub fn ERROR()            -> Color { active_palette().error }
    pub fn ERROR_LIGHT()      -> Color { active_palette().error_light }
    pub fn ERROR_BG()         -> Color { active_palette().error_bg }
    pub fn BTN_DANGER_BG()    -> Color { active_palette().btn_danger_bg }
    pub fn BTN_DANGER_HOVER() -> Color { active_palette().btn_danger_hover }
    pub fn BTN_TRASH_HOVER()  -> Color { active_palette().btn_trash_hover }
    pub fn BTN_TRASH_PRESSED()-> Color { active_palette().btn_trash_pressed }

    // Modal
    pub fn BG_MODAL_SECTION() -> Color { active_palette().bg_modal_section }
    pub fn BORDER_SUBTLE()    -> Color { active_palette().border_subtle }
    pub fn DIVIDER()          -> Color { active_palette().divider }
}

// ── Compile-time hex → Color helper ──

const fn hex(h: u32) -> Color {
    let r = ((h >> 16) & 0xFF) as f32 / 255.0;
    let g = ((h >> 8) & 0xFF) as f32 / 255.0;
    let b = (h & 0xFF) as f32 / 255.0;
    Color { r, g, b, a: 1.0 }
}

// ════════════════════════════════════════════════════════════════════════
//  Theme palette definitions
// ════════════════════════════════════════════════════════════════════════

impl ThemePalette {
    /// Boost contrast for accessibility: brighten text, darken backgrounds, sharpen borders.
    pub fn with_high_contrast(mut self) -> Self {
        fn boost(c: Color, amount: f32) -> Color {
            Color {
                r: (c.r + amount).min(1.0),
                g: (c.g + amount).min(1.0),
                b: (c.b + amount).min(1.0),
                a: c.a,
            }
        }
        fn darken(c: Color, amount: f32) -> Color {
            Color {
                r: (c.r - amount).max(0.0),
                g: (c.g - amount).max(0.0),
                b: (c.b - amount).max(0.0),
                a: c.a,
            }
        }
        // Detect light vs dark theme by bg brightness
        let bg_luma = self.bg_primary.r * 0.299 + self.bg_primary.g * 0.587 + self.bg_primary.b * 0.114;
        let is_light = bg_luma > 0.5;

        if is_light {
            // Light theme: darken text more, lighten backgrounds
            self.text_primary = darken(self.text_primary, 0.15);
            self.text_secondary = darken(self.text_secondary, 0.12);
            self.text_muted = darken(self.text_muted, 0.10);
            self.text_dim = darken(self.text_dim, 0.10);
            self.text_dimmer = darken(self.text_dimmer, 0.08);
            self.border_subtle = darken(self.border_subtle, 0.15);
            self.divider = darken(self.divider, 0.15);
        } else {
            // Dark theme: brighten text, sharpen borders
            self.text_primary = boost(self.text_primary, 0.12);
            self.text_secondary = boost(self.text_secondary, 0.10);
            self.text_muted = boost(self.text_muted, 0.10);
            self.text_dim = boost(self.text_dim, 0.08);
            self.text_dimmer = boost(self.text_dimmer, 0.08);
            self.border_subtle = boost(self.border_subtle, 0.12);
            self.divider = boost(self.divider, 0.12);
        }
        self
    }

    // ── Catppuccin Latte (light) ──
    pub const CATPPUCCIN_LATTE: Self = Self {
        bg_primary:       hex(0xeff1f5),
        bg_sidebar:       hex(0xe6e9ef),
        bg_card:          hex(0xe6e9ef),
        bg_card_hover:    hex(0xdce0e8),
        bg_card_pressed:  hex(0xccd0da),
        bg_selected:      hex(0xccd0da),
        bg_input:         hex(0xdce0e8),
        bg_progress:      hex(0xe6e9ef),

        text_primary:     hex(0x4c4f69),
        text_secondary:   hex(0x5c5f77),
        text_muted:       hex(0x6c6f85),
        text_dim:         hex(0x7c7f93),
        text_dimmer:      hex(0x8c8fa1),
        text_dimmest:     hex(0x9ca0b0),
        text_placeholder: hex(0x8c8fa1),

        accent_blue:      hex(0x1e66f5),
        accent_icon:      hex(0x7287fd),
        accent_progress:  hex(0x7287fd),

        btn_default:      hex(0xdce0e8),
        btn_hover:        hex(0xccd0da),
        btn_pressed:      hex(0xbcc0cc),

        success:          hex(0x40a02b),
        success_bg:       hex(0xd5f0cd),
        btn_success:      hex(0x40a02b),
        btn_success_hover: hex(0x369222),
        btn_success_pressed: hex(0x2c8219),

        warning:          hex(0xdf8e1d),
        warning_bg:       hex(0xf5e6c8),

        error:            hex(0xd20f39),
        error_light:      hex(0xe64553),
        error_bg:         hex(0xf5d0d6),
        btn_danger_bg:    hex(0xf5d0d6),
        btn_danger_hover: hex(0xeab0ba),
        btn_trash_hover:  hex(0xd20f39),
        btn_trash_pressed: hex(0xb00c30),

        bg_modal_section: hex(0xe6e9ef),
        border_subtle:    hex(0xccd0da),
        divider:          hex(0xccd0da),
    };

    // ── Catppuccin Frappé ──
    pub const CATPPUCCIN_FRAPPE: Self = Self {
        bg_primary:       hex(0x303446),
        bg_sidebar:       hex(0x292c3c),
        bg_card:          hex(0x292c3c),
        bg_card_hover:    hex(0x414559),
        bg_card_pressed:  hex(0x51576d),
        bg_selected:      hex(0x51576d),
        bg_input:         hex(0x414559),
        bg_progress:      hex(0x292c3c),

        text_primary:     hex(0xc6d0f5),
        text_secondary:   hex(0xb5bfe2),
        text_muted:       hex(0xa5adce),
        text_dim:         hex(0x949cbb),
        text_dimmer:      hex(0x838ba7),
        text_dimmest:     hex(0x737994),
        text_placeholder: hex(0x838ba7),

        accent_blue:      hex(0x8caaee),
        accent_icon:      hex(0xbabbf1),
        accent_progress:  hex(0xbabbf1),

        btn_default:      hex(0x414559),
        btn_hover:        hex(0x51576d),
        btn_pressed:      hex(0x626880),

        success:          hex(0xa6d189),
        success_bg:       hex(0x2a3a28),
        btn_success:      hex(0x549444),
        btn_success_hover: hex(0x468838),
        btn_success_pressed: hex(0x387a2c),

        warning:          hex(0xe5c890),
        warning_bg:       hex(0x3a3828),

        error:            hex(0xe78284),
        error_light:      hex(0xea999c),
        error_bg:         hex(0x3a2628),
        btn_danger_bg:    hex(0x3a2628),
        btn_danger_hover: hex(0x5a3638),
        btn_trash_hover:  hex(0x9a3234),
        btn_trash_pressed: hex(0x7a2224),

        bg_modal_section: hex(0x292c3c),
        border_subtle:    hex(0x414559),
        divider:          hex(0x414559),
    };

    // ── Catppuccin Macchiato ──
    pub const CATPPUCCIN_MACCHIATO: Self = Self {
        bg_primary:       hex(0x24273a),
        bg_sidebar:       hex(0x1e2030),
        bg_card:          hex(0x1e2030),
        bg_card_hover:    hex(0x363a4f),
        bg_card_pressed:  hex(0x494d64),
        bg_selected:      hex(0x494d64),
        bg_input:         hex(0x363a4f),
        bg_progress:      hex(0x1e2030),

        text_primary:     hex(0xcad3f5),
        text_secondary:   hex(0xb8c0e0),
        text_muted:       hex(0xa5adcb),
        text_dim:         hex(0x939ab7),
        text_dimmer:      hex(0x8087a2),
        text_dimmest:     hex(0x6e738d),
        text_placeholder: hex(0x8087a2),

        accent_blue:      hex(0x8aadf4),
        accent_icon:      hex(0xb7bdf8),
        accent_progress:  hex(0xb7bdf8),

        btn_default:      hex(0x363a4f),
        btn_hover:        hex(0x494d64),
        btn_pressed:      hex(0x5b6078),

        success:          hex(0xa6da95),
        success_bg:       hex(0x243826),
        btn_success:      hex(0x4e9240),
        btn_success_hover: hex(0x408634),
        btn_success_pressed: hex(0x327a28),

        warning:          hex(0xeed49f),
        warning_bg:       hex(0x383428),

        error:            hex(0xed8796),
        error_light:      hex(0xee99a0),
        error_bg:         hex(0x382428),
        btn_danger_bg:    hex(0x382428),
        btn_danger_hover: hex(0x583438),
        btn_trash_hover:  hex(0x983038),
        btn_trash_pressed: hex(0x782028),

        bg_modal_section: hex(0x1e2030),
        border_subtle:    hex(0x363a4f),
        divider:          hex(0x363a4f),
    };

    // ── Catppuccin Mocha ──
    pub const CATPPUCCIN_MOCHA: Self = Self {
        bg_primary:       hex(0x1e1e2e),
        bg_sidebar:       hex(0x181825),
        bg_card:          hex(0x181825),
        bg_card_hover:    hex(0x313244),
        bg_card_pressed:  hex(0x45475a),
        bg_selected:      hex(0x45475a),
        bg_input:         hex(0x313244),
        bg_progress:      hex(0x181825),

        text_primary:     hex(0xcdd6f4),
        text_secondary:   hex(0xbac2de),
        text_muted:       hex(0xa6adc8),
        text_dim:         hex(0x9399b2),
        text_dimmer:      hex(0x7f849c),
        text_dimmest:     hex(0x6c7086),
        text_placeholder: hex(0x7f849c),

        accent_blue:      hex(0x89b4fa),
        accent_icon:      hex(0xb4befe),
        accent_progress:  hex(0xb4befe),

        btn_default:      hex(0x313244),
        btn_hover:        hex(0x45475a),
        btn_pressed:      hex(0x585b70),

        success:          hex(0xa6e3a1),
        success_bg:       hex(0x1e3a1e),
        btn_success:      hex(0x48904a),
        btn_success_hover: hex(0x3a843c),
        btn_success_pressed: hex(0x2c782e),

        warning:          hex(0xf9e2af),
        warning_bg:       hex(0x3a3420),

        error:            hex(0xf38ba8),
        error_light:      hex(0xf5a0b8),
        error_bg:         hex(0x3a1e28),
        btn_danger_bg:    hex(0x3a1e28),
        btn_danger_hover: hex(0x5a2e38),
        btn_trash_hover:  hex(0x952e3a),
        btn_trash_pressed: hex(0x751e2a),

        bg_modal_section: hex(0x181825),
        border_subtle:    hex(0x313244),
        divider:          hex(0x313244),
    };

    // ── Gruvbox Dark ──
    pub const GRUVBOX_DARK: Self = Self {
        bg_primary:       hex(0x282828),
        bg_sidebar:       hex(0x1d2021),
        bg_card:          hex(0x3c3836),
        bg_card_hover:    hex(0x504945),
        bg_card_pressed:  hex(0x665c54),
        bg_selected:      hex(0x504945),
        bg_input:         hex(0x3c3836),
        bg_progress:      hex(0x32302f),

        text_primary:     hex(0xebdbb2),
        text_secondary:   hex(0xd5c4a1),
        text_muted:       hex(0xbdae93),
        text_dim:         hex(0xa89984),
        text_dimmer:      hex(0x928374),
        text_dimmest:     hex(0x7c6f64),
        text_placeholder: hex(0x928374),

        accent_blue:      hex(0x83a598),
        accent_icon:      hex(0x83a598),
        accent_progress:  hex(0x83a598),

        btn_default:      hex(0x3c3836),
        btn_hover:        hex(0x504945),
        btn_pressed:      hex(0x665c54),

        success:          hex(0xb8bb26),
        success_bg:       hex(0x2a3020),
        btn_success:      hex(0x689d6a),
        btn_success_hover: hex(0x5a8f5c),
        btn_success_pressed: hex(0x4c814e),

        warning:          hex(0xfabd2f),
        warning_bg:       hex(0x3a3420),

        error:            hex(0xfb4934),
        error_light:      hex(0xfe6050),
        error_bg:         hex(0x3c1f1f),
        btn_danger_bg:    hex(0x3c1f1f),
        btn_danger_hover: hex(0x5c2f2f),
        btn_trash_hover:  hex(0x9d3030),
        btn_trash_pressed: hex(0x7d2020),

        bg_modal_section: hex(0x282828),
        border_subtle:    hex(0x3c3836),
        divider:          hex(0x3c3836),
    };

    // ── Gruvbox Light ──
    pub const GRUVBOX_LIGHT: Self = Self {
        bg_primary:       hex(0xfbf1c7),
        bg_sidebar:       hex(0xf2e5bc),
        bg_card:          hex(0xebdbb2),
        bg_card_hover:    hex(0xd5c4a1),
        bg_card_pressed:  hex(0xbdae93),
        bg_selected:      hex(0xd5c4a1),
        bg_input:         hex(0xebdbb2),
        bg_progress:      hex(0xf2e5bc),

        text_primary:     hex(0x3c3836),
        text_secondary:   hex(0x504945),
        text_muted:       hex(0x665c54),
        text_dim:         hex(0x7c6f64),
        text_dimmer:      hex(0x928374),
        text_dimmest:     hex(0xa89984),
        text_placeholder: hex(0x928374),

        accent_blue:      hex(0x458588),
        accent_icon:      hex(0x458588),
        accent_progress:  hex(0x458588),

        btn_default:      hex(0xebdbb2),
        btn_hover:        hex(0xd5c4a1),
        btn_pressed:      hex(0xbdae93),

        success:          hex(0x98971a),
        success_bg:       hex(0xdde6b0),
        btn_success:      hex(0x689d6a),
        btn_success_hover: hex(0x5a8f5c),
        btn_success_pressed: hex(0x4c814e),

        warning:          hex(0xd79921),
        warning_bg:       hex(0xf0e0b0),

        error:            hex(0xcc241d),
        error_light:      hex(0xd44040),
        error_bg:         hex(0xf0c8c8),
        btn_danger_bg:    hex(0xf0c8c8),
        btn_danger_hover: hex(0xe0a8a8),
        btn_trash_hover:  hex(0xcc241d),
        btn_trash_pressed: hex(0xaa1818),

        bg_modal_section: hex(0xfbf1c7),
        border_subtle:    hex(0xd5c4a1),
        divider:          hex(0xd5c4a1),
    };

    // ── Everblush Dark ──
    pub const EVERBLUSH_DARK: Self = Self {
        bg_primary:       hex(0x141b1e),
        bg_sidebar:       hex(0x1a2124),
        bg_card:          hex(0x232a2d),
        bg_card_hover:    hex(0x2c3538),
        bg_card_pressed:  hex(0x354043),
        bg_selected:      hex(0x2c3538),
        bg_input:         hex(0x232a2d),
        bg_progress:      hex(0x1a2124),

        text_primary:     hex(0xdadada),
        text_secondary:   hex(0xc4c4c4),
        text_muted:       hex(0xb3b9b8),
        text_dim:         hex(0x9aa0a0),
        text_dimmer:      hex(0x808888),
        text_dimmest:     hex(0x667070),
        text_placeholder: hex(0x808888),

        accent_blue:      hex(0x67b0e8),
        accent_icon:      hex(0x67b0e8),
        accent_progress:  hex(0x67b0e8),

        btn_default:      hex(0x232a2d),
        btn_hover:        hex(0x2c3538),
        btn_pressed:      hex(0x354043),

        success:          hex(0x8ccf7e),
        success_bg:       hex(0x1a2e1e),
        btn_success:      hex(0x5aaa50),
        btn_success_hover: hex(0x4c9e44),
        btn_success_pressed: hex(0x3e9238),

        warning:          hex(0xe5c76b),
        warning_bg:       hex(0x2e2a1a),

        error:            hex(0xe57474),
        error_light:      hex(0xf08888),
        error_bg:         hex(0x2e1a1a),
        btn_danger_bg:    hex(0x2e1a1a),
        btn_danger_hover: hex(0x4e2a2a),
        btn_trash_hover:  hex(0x9a3030),
        btn_trash_pressed: hex(0x7a2020),

        bg_modal_section: hex(0x232a2d),
        border_subtle:    hex(0x2c3538),
        divider:          hex(0x2c3538),
    };

    // ── Everblush Light ──
    pub const EVERBLUSH_LIGHT: Self = Self {
        bg_primary:       hex(0xe8eded),
        bg_sidebar:       hex(0xdce3e3),
        bg_card:          hex(0xd0d8d8),
        bg_card_hover:    hex(0xc4cece),
        bg_card_pressed:  hex(0xb8c4c4),
        bg_selected:      hex(0xc4cece),
        bg_input:         hex(0xd0d8d8),
        bg_progress:      hex(0xdce3e3),

        text_primary:     hex(0x2d3437),
        text_secondary:   hex(0x3a4448),
        text_muted:       hex(0x4a5558),
        text_dim:         hex(0x5a6668),
        text_dimmer:      hex(0x6a7878),
        text_dimmest:     hex(0x7a8a8a),
        text_placeholder: hex(0x6a7878),

        accent_blue:      hex(0x3a88c0),
        accent_icon:      hex(0x3a88c0),
        accent_progress:  hex(0x3a88c0),

        btn_default:      hex(0xd0d8d8),
        btn_hover:        hex(0xc4cece),
        btn_pressed:      hex(0xb8c4c4),

        success:          hex(0x5aaa4e),
        success_bg:       hex(0xc8e8c4),
        btn_success:      hex(0x5aaa4e),
        btn_success_hover: hex(0x4c9e42),
        btn_success_pressed: hex(0x3e9236),

        warning:          hex(0xc0a030),
        warning_bg:       hex(0xe8e0c0),

        error:            hex(0xc85050),
        error_light:      hex(0xd86868),
        error_bg:         hex(0xf0d0d0),
        btn_danger_bg:    hex(0xf0d0d0),
        btn_danger_hover: hex(0xe0b0b0),
        btn_trash_hover:  hex(0xc85050),
        btn_trash_pressed: hex(0xa84040),

        bg_modal_section: hex(0xe8eded),
        border_subtle:    hex(0xc4cece),
        divider:          hex(0xc4cece),
    };

    // ── Kanagawa Wave (dark — default) ──
    pub const KANAGAWA_WAVE: Self = Self {
        bg_primary:       hex(0x1F1F28),
        bg_sidebar:       hex(0x181820),
        bg_card:          hex(0x2A2A37),
        bg_card_hover:    hex(0x363646),
        bg_card_pressed:  hex(0x54546D),
        bg_selected:      hex(0x363646),
        bg_input:         hex(0x2A2A37),
        bg_progress:      hex(0x223249),

        text_primary:     hex(0xDCD7BA),
        text_secondary:   hex(0xC8C093),
        text_muted:       hex(0x9CABCA),
        text_dim:         hex(0x938AA9),
        text_dimmer:      hex(0x727169),
        text_dimmest:     hex(0x54546D),
        text_placeholder: hex(0x727169),

        accent_blue:      hex(0x7E9CD8),
        accent_icon:      hex(0x7FB4CA),
        accent_progress:  hex(0x7FB4CA),

        btn_default:      hex(0x2A2A37),
        btn_hover:        hex(0x363646),
        btn_pressed:      hex(0x54546D),

        success:          hex(0x98BB6C),
        success_bg:       hex(0x2B3328),
        btn_success:      hex(0x76946A),
        btn_success_hover: hex(0x68885c),
        btn_success_pressed: hex(0x5a7c4e),

        warning:          hex(0xE6C384),
        warning_bg:       hex(0x49443C),

        error:            hex(0xE46876),
        error_light:      hex(0xFF5D62),
        error_bg:         hex(0x43242B),
        btn_danger_bg:    hex(0x43242B),
        btn_danger_hover: hex(0x63343B),
        btn_trash_hover:  hex(0xC34043),
        btn_trash_pressed: hex(0xA33033),

        bg_modal_section: hex(0x1F1F28),
        border_subtle:    hex(0x2A2A37),
        divider:          hex(0x2A2A37),
    };

    // ── Kanagawa Journal (warm parchment light) ──
    // Inspired by the Grape "Mode journal" — warm beige/sepia paper tones
    pub const KANAGAWA_DRAGON: Self = Self {
        bg_primary:       hex(0xd5cea3), // warm parchment
        bg_sidebar:       hex(0xc8b98e), // slightly darker sidebar
        bg_card:          hex(0xc8b98e),
        bg_card_hover:    hex(0xbdae80),
        bg_card_pressed:  hex(0xb0a070),
        bg_selected:      hex(0xbdae80),
        bg_input:         hex(0xc8b98e),
        bg_progress:      hex(0xc8b98e),

        text_primary:     hex(0x43412e), // dark brown
        text_secondary:   hex(0x5c5840),
        text_muted:       hex(0x736e55),
        text_dim:         hex(0x8a856c),
        text_dimmer:      hex(0xa09a80),
        text_dimmest:     hex(0xb5ae94),
        text_placeholder: hex(0x8a856c),

        accent_blue:      hex(0x7a6840), // warm brown accent (matching screenshot)
        accent_icon:      hex(0x7a6840),
        accent_progress:  hex(0x7a6840),

        btn_default:      hex(0xc8b98e),
        btn_hover:        hex(0xbdae80),
        btn_pressed:      hex(0xb0a070),

        success:          hex(0x6f894e),
        success_bg:       hex(0xcbd8b8),
        btn_success:      hex(0x6e915f),
        btn_success_hover: hex(0x608554),
        btn_success_pressed: hex(0x527949),

        warning:          hex(0xc49a20),
        warning_bg:       hex(0xe0d4a0),

        error:            hex(0xc84053),
        error_light:      hex(0xd7474b),
        error_bg:         hex(0xe0bfb0),
        btn_danger_bg:    hex(0xe0bfb0),
        btn_danger_hover: hex(0xd0a898),
        btn_trash_hover:  hex(0xc84053),
        btn_trash_pressed: hex(0xa83043),

        bg_modal_section: hex(0xd5cea3),
        border_subtle:    hex(0xb8a878),
        divider:          hex(0xb8a878),
    };

    // ── Kanagawa Lotus (light) ──
    pub const KANAGAWA_LOTUS: Self = Self {
        bg_primary:       hex(0xf2ecbc),
        bg_sidebar:       hex(0xe5ddb0),
        bg_card:          hex(0xe7dba0),
        bg_card_hover:    hex(0xd5cea3),
        bg_card_pressed:  hex(0xc8c093),
        bg_selected:      hex(0xd5cea3),
        bg_input:         hex(0xe4d794),
        bg_progress:      hex(0xe5ddb0),

        text_primary:     hex(0x545464),
        text_secondary:   hex(0x43436c),
        text_muted:       hex(0x716e61),
        text_dim:         hex(0x8a8980),
        text_dimmer:      hex(0xa09cac),
        text_dimmest:     hex(0xb0acb8),
        text_placeholder: hex(0x8a8980),

        accent_blue:      hex(0x4d699b),
        accent_icon:      hex(0x5d57a3),
        accent_progress:  hex(0x5d57a3),

        btn_default:      hex(0xe7dba0),
        btn_hover:        hex(0xd5cea3),
        btn_pressed:      hex(0xc8c093),

        success:          hex(0x6f894e),
        success_bg:       hex(0xb7d0ae),
        btn_success:      hex(0x6e915f),
        btn_success_hover: hex(0x608554),
        btn_success_pressed: hex(0x527949),

        warning:          hex(0xde9800),
        warning_bg:       hex(0xf9d791),

        error:            hex(0xc84053),
        error_light:      hex(0xd7474b),
        error_bg:         hex(0xd9a594),
        btn_danger_bg:    hex(0xd9a594),
        btn_danger_hover: hex(0xc89484),
        btn_trash_hover:  hex(0xc84053),
        btn_trash_pressed: hex(0xa83043),

        bg_modal_section: hex(0xf2ecbc),
        border_subtle:    hex(0xd5cea3),
        divider:          hex(0xd5cea3),
    };
}

// ════════════════════════════════════════════════════════════════════════
//  Additional theme palettes
// ════════════════════════════════════════════════════════════════════════

impl ThemePalette {

    // ── Nord Dark ──
    pub const NORD_DARK: Self = Self {
        bg_primary: hex(0x2E3440), bg_sidebar: hex(0x252B37), bg_card: hex(0x3B4252),
        bg_card_hover: hex(0x434C5E), bg_card_pressed: hex(0x4C566A), bg_selected: hex(0x434C5E),
        bg_input: hex(0x3B4252), bg_progress: hex(0x252B37),
        text_primary: hex(0xD8DEE9), text_secondary: hex(0xC0C8D4), text_muted: hex(0xA0AEC0),
        text_dim: hex(0x7E8FA4), text_dimmer: hex(0x616E88), text_dimmest: hex(0x4C566A),
        text_placeholder: hex(0x616E88),
        accent_blue: hex(0x88C0D0), accent_icon: hex(0x81A1C1), accent_progress: hex(0x81A1C1),
        btn_default: hex(0x3B4252), btn_hover: hex(0x434C5E), btn_pressed: hex(0x4C566A),
        success: hex(0xA3BE8C), success_bg: hex(0x2A3425),
        btn_success: hex(0x809B6C), btn_success_hover: hex(0x728F60), btn_success_pressed: hex(0x648354),
        warning: hex(0xEBCB8B), warning_bg: hex(0x3A3828),
        error: hex(0xBF616A), error_light: hex(0xD08080), error_bg: hex(0x3C2830),
        btn_danger_bg: hex(0x3C2830), btn_danger_hover: hex(0x5C3840),
        btn_trash_hover: hex(0xBF616A), btn_trash_pressed: hex(0x9F5158),
        bg_modal_section: hex(0x2E3440), border_subtle: hex(0x3B4252), divider: hex(0x3B4252),
    };

    // ── Nord Light ──
    pub const NORD_LIGHT: Self = Self {
        bg_primary: hex(0xECEFF4), bg_sidebar: hex(0xE5E9F0), bg_card: hex(0xD8DEE9),
        bg_card_hover: hex(0xCCD4DF), bg_card_pressed: hex(0xB8C2D0), bg_selected: hex(0xCCD4DF),
        bg_input: hex(0xD8DEE9), bg_progress: hex(0xE5E9F0),
        text_primary: hex(0x2E3440), text_secondary: hex(0x3B4252), text_muted: hex(0x4C566A),
        text_dim: hex(0x636E80), text_dimmer: hex(0x7E8FA4), text_dimmest: hex(0xA0AAB8),
        text_placeholder: hex(0x7E8FA4),
        accent_blue: hex(0x5E81AC), accent_icon: hex(0x81A1C1), accent_progress: hex(0x81A1C1),
        btn_default: hex(0xD8DEE9), btn_hover: hex(0xCCD4DF), btn_pressed: hex(0xB8C2D0),
        success: hex(0x6B8C56), success_bg: hex(0xD0E0C8),
        btn_success: hex(0x6B8C56), btn_success_hover: hex(0x5E8048), btn_success_pressed: hex(0x50743A),
        warning: hex(0xB5A050), warning_bg: hex(0xE8DFC0),
        error: hex(0xA04048), error_light: hex(0xBF616A), error_bg: hex(0xE8C8C8),
        btn_danger_bg: hex(0xE8C8C8), btn_danger_hover: hex(0xD8B0B0),
        btn_trash_hover: hex(0xA04048), btn_trash_pressed: hex(0x883438),
        bg_modal_section: hex(0xECEFF4), border_subtle: hex(0xCCD4DF), divider: hex(0xCCD4DF),
    };

    // ── Dracula Dark ──
    pub const DRACULA_DARK: Self = Self {
        bg_primary: hex(0x282A36), bg_sidebar: hex(0x21222C), bg_card: hex(0x44475A),
        bg_card_hover: hex(0x515470), bg_card_pressed: hex(0x606382), bg_selected: hex(0x44475A),
        bg_input: hex(0x44475A), bg_progress: hex(0x21222C),
        text_primary: hex(0xF8F8F2), text_secondary: hex(0xE8E8E0), text_muted: hex(0xC8C8C0),
        text_dim: hex(0x9898A0), text_dimmer: hex(0x6272A4), text_dimmest: hex(0x4C5478),
        text_placeholder: hex(0x6272A4),
        accent_blue: hex(0xBD93F9), accent_icon: hex(0xFF79C6), accent_progress: hex(0xBD93F9),
        btn_default: hex(0x44475A), btn_hover: hex(0x515470), btn_pressed: hex(0x606382),
        success: hex(0x50FA7B), success_bg: hex(0x1E3228),
        btn_success: hex(0x40C862), btn_success_hover: hex(0x38B858), btn_success_pressed: hex(0x30A84E),
        warning: hex(0xF1FA8C), warning_bg: hex(0x3A382A),
        error: hex(0xFF5555), error_light: hex(0xFF7777), error_bg: hex(0x3C222A),
        btn_danger_bg: hex(0x3C222A), btn_danger_hover: hex(0x5C323A),
        btn_trash_hover: hex(0xFF5555), btn_trash_pressed: hex(0xDD4444),
        bg_modal_section: hex(0x282A36), border_subtle: hex(0x44475A), divider: hex(0x44475A),
    };

    // ── Dracula Light ──
    pub const DRACULA_LIGHT: Self = Self {
        bg_primary: hex(0xFFFBEB), bg_sidebar: hex(0xF4EFD8), bg_card: hex(0xE8E4CF),
        bg_card_hover: hex(0xDCD8C0), bg_card_pressed: hex(0xD0CCB8), bg_selected: hex(0xDCD8C0),
        bg_input: hex(0xE8E4CF), bg_progress: hex(0xF4EFD8),
        text_primary: hex(0x1F1F1F), text_secondary: hex(0x333333), text_muted: hex(0x555555),
        text_dim: hex(0x777777), text_dimmer: hex(0x7970A9), text_dimmest: hex(0xA09CA0),
        text_placeholder: hex(0x7970A9),
        accent_blue: hex(0x7C5FC2), accent_icon: hex(0xE04886), accent_progress: hex(0x7C5FC2),
        btn_default: hex(0xE8E4CF), btn_hover: hex(0xDCD8C0), btn_pressed: hex(0xD0CCB8),
        success: hex(0x2BA479), success_bg: hex(0xD0E8D8),
        btn_success: hex(0x2BA479), btn_success_hover: hex(0x24966C), btn_success_pressed: hex(0x1D8860),
        warning: hex(0xD5A212), warning_bg: hex(0xEDE0C0),
        error: hex(0xDE3535), error_light: hex(0xE85858), error_bg: hex(0xEEDCDC),
        btn_danger_bg: hex(0xEEDCDC), btn_danger_hover: hex(0xDEC8C8),
        btn_trash_hover: hex(0xDE3535), btn_trash_pressed: hex(0xC02828),
        bg_modal_section: hex(0xFFFBEB), border_subtle: hex(0xDCD8C0), divider: hex(0xDCD8C0),
    };

    // ── Solarized Dark ──
    pub const SOLARIZED_DARK: Self = Self {
        bg_primary: hex(0x002B36), bg_sidebar: hex(0x00222C), bg_card: hex(0x073642),
        bg_card_hover: hex(0x0A4858), bg_card_pressed: hex(0x1A5A6C), bg_selected: hex(0x073642),
        bg_input: hex(0x073642), bg_progress: hex(0x00222C),
        text_primary: hex(0x839496), text_secondary: hex(0x93A1A1), text_muted: hex(0x738C90),
        text_dim: hex(0x657B83), text_dimmer: hex(0x586E75), text_dimmest: hex(0x475860),
        text_placeholder: hex(0x586E75),
        accent_blue: hex(0x268BD2), accent_icon: hex(0x2AA198), accent_progress: hex(0x268BD2),
        btn_default: hex(0x073642), btn_hover: hex(0x0A4858), btn_pressed: hex(0x1A5A6C),
        success: hex(0x859900), success_bg: hex(0x0A2A18),
        btn_success: hex(0x6A7A00), btn_success_hover: hex(0x5C6C00), btn_success_pressed: hex(0x4E5E00),
        warning: hex(0xB58900), warning_bg: hex(0x1A2818),
        error: hex(0xDC322F), error_light: hex(0xE8504E), error_bg: hex(0x2A1A1A),
        btn_danger_bg: hex(0x2A1A1A), btn_danger_hover: hex(0x3A2A2A),
        btn_trash_hover: hex(0xDC322F), btn_trash_pressed: hex(0xBC2220),
        bg_modal_section: hex(0x002B36), border_subtle: hex(0x073642), divider: hex(0x073642),
    };

    // ── Solarized Light ──
    pub const SOLARIZED_LIGHT: Self = Self {
        bg_primary: hex(0xFDF6E3), bg_sidebar: hex(0xEEE8D5), bg_card: hex(0xE6E0CA),
        bg_card_hover: hex(0xDCD6C0), bg_card_pressed: hex(0xD0CAB4), bg_selected: hex(0xDCD6C0),
        bg_input: hex(0xE6E0CA), bg_progress: hex(0xEEE8D5),
        text_primary: hex(0x657B83), text_secondary: hex(0x586E75), text_muted: hex(0x7C8E94),
        text_dim: hex(0x93A1A1), text_dimmer: hex(0xA8B4B8), text_dimmest: hex(0xC0C8CA),
        text_placeholder: hex(0xA8B4B8),
        accent_blue: hex(0x268BD2), accent_icon: hex(0x2AA198), accent_progress: hex(0x268BD2),
        btn_default: hex(0xE6E0CA), btn_hover: hex(0xDCD6C0), btn_pressed: hex(0xD0CAB4),
        success: hex(0x859900), success_bg: hex(0xDAE8C8),
        btn_success: hex(0x6A7A00), btn_success_hover: hex(0x5C6C00), btn_success_pressed: hex(0x4E5E00),
        warning: hex(0xB58900), warning_bg: hex(0xECE0C0),
        error: hex(0xDC322F), error_light: hex(0xE8504E), error_bg: hex(0xECCCCC),
        btn_danger_bg: hex(0xECCCCC), btn_danger_hover: hex(0xDCB8B8),
        btn_trash_hover: hex(0xDC322F), btn_trash_pressed: hex(0xBC2220),
        bg_modal_section: hex(0xFDF6E3), border_subtle: hex(0xDCD6C0), divider: hex(0xDCD6C0),
    };

    // ── Tokyo Night ──
    pub const TOKYONIGHT_NIGHT: Self = Self {
        bg_primary: hex(0x1A1B26), bg_sidebar: hex(0x16161E), bg_card: hex(0x292E42),
        bg_card_hover: hex(0x3B4261), bg_card_pressed: hex(0x414868), bg_selected: hex(0x2D3F76),
        bg_input: hex(0x292E42), bg_progress: hex(0x16161E),
        text_primary: hex(0xC0CAF5), text_secondary: hex(0xA9B1D6), text_muted: hex(0x8890B0),
        text_dim: hex(0x737AA2), text_dimmer: hex(0x565F89), text_dimmest: hex(0x414868),
        text_placeholder: hex(0x565F89),
        accent_blue: hex(0x7AA2F7), accent_icon: hex(0x7DCFFF), accent_progress: hex(0x7AA2F7),
        btn_default: hex(0x292E42), btn_hover: hex(0x3B4261), btn_pressed: hex(0x414868),
        success: hex(0x9ECE6A), success_bg: hex(0x202830),
        btn_success: hex(0x7EAA50), btn_success_hover: hex(0x709E44), btn_success_pressed: hex(0x629238),
        warning: hex(0xE0AF68), warning_bg: hex(0x302820),
        error: hex(0xF7768E), error_light: hex(0xDB4B4B), error_bg: hex(0x302025),
        btn_danger_bg: hex(0x302025), btn_danger_hover: hex(0x503035),
        btn_trash_hover: hex(0xF7768E), btn_trash_pressed: hex(0xD75E76),
        bg_modal_section: hex(0x1A1B26), border_subtle: hex(0x292E42), divider: hex(0x292E42),
    };

    // ── Tokyo Night Day ──
    pub const TOKYONIGHT_DAY: Self = Self {
        bg_primary: hex(0xE1E2E7), bg_sidebar: hex(0xD0D5E3), bg_card: hex(0xC4C8DA),
        bg_card_hover: hex(0xB7C1E3), bg_card_pressed: hex(0xA8B4D4), bg_selected: hex(0xB7C1E3),
        bg_input: hex(0xC4C8DA), bg_progress: hex(0xD0D5E3),
        text_primary: hex(0x3760BF), text_secondary: hex(0x4A6CA8), text_muted: hex(0x6172B0),
        text_dim: hex(0x848CB5), text_dimmer: hex(0xA1A6C5), text_dimmest: hex(0xB4B5B9),
        text_placeholder: hex(0xA1A6C5),
        accent_blue: hex(0x2E7DE9), accent_icon: hex(0x007197), accent_progress: hex(0x2E7DE9),
        btn_default: hex(0xC4C8DA), btn_hover: hex(0xB7C1E3), btn_pressed: hex(0xA8B4D4),
        success: hex(0x587539), success_bg: hex(0xC8DCC0),
        btn_success: hex(0x587539), btn_success_hover: hex(0x4C692E), btn_success_pressed: hex(0x405D24),
        warning: hex(0x8C6C3E), warning_bg: hex(0xDCD8C0),
        error: hex(0xF52A65), error_light: hex(0xC64343), error_bg: hex(0xE8C0C8),
        btn_danger_bg: hex(0xE8C0C8), btn_danger_hover: hex(0xD8A8B4),
        btn_trash_hover: hex(0xF52A65), btn_trash_pressed: hex(0xD52050),
        bg_modal_section: hex(0xE1E2E7), border_subtle: hex(0xC4C8DA), divider: hex(0xC4C8DA),
    };

    // ── Rosé Pine ──
    pub const ROSEPINE_MAIN: Self = Self {
        bg_primary: hex(0x191724), bg_sidebar: hex(0x1F1D2E), bg_card: hex(0x26233A),
        bg_card_hover: hex(0x403D52), bg_card_pressed: hex(0x524F67), bg_selected: hex(0x403D52),
        bg_input: hex(0x26233A), bg_progress: hex(0x1F1D2E),
        text_primary: hex(0xE0DEF4), text_secondary: hex(0xCCCADD), text_muted: hex(0x908CAA),
        text_dim: hex(0x7E7A96), text_dimmer: hex(0x6E6A86), text_dimmest: hex(0x555168),
        text_placeholder: hex(0x6E6A86),
        accent_blue: hex(0x9CCFD8), accent_icon: hex(0xC4A7E7), accent_progress: hex(0x9CCFD8),
        btn_default: hex(0x26233A), btn_hover: hex(0x403D52), btn_pressed: hex(0x524F67),
        success: hex(0x31748F), success_bg: hex(0x1E2830),
        btn_success: hex(0x2A6478), btn_success_hover: hex(0x24586C), btn_success_pressed: hex(0x1E4C60),
        warning: hex(0xF6C177), warning_bg: hex(0x302820),
        error: hex(0xEB6F92), error_light: hex(0xF08098), error_bg: hex(0x301828),
        btn_danger_bg: hex(0x301828), btn_danger_hover: hex(0x502838),
        btn_trash_hover: hex(0xEB6F92), btn_trash_pressed: hex(0xCB5878),
        bg_modal_section: hex(0x191724), border_subtle: hex(0x26233A), divider: hex(0x26233A),
    };

    // ── Rosé Pine Moon ──
    pub const ROSEPINE_MOON: Self = Self {
        bg_primary: hex(0x232136), bg_sidebar: hex(0x2A273F), bg_card: hex(0x393552),
        bg_card_hover: hex(0x44415A), bg_card_pressed: hex(0x56526E), bg_selected: hex(0x44415A),
        bg_input: hex(0x393552), bg_progress: hex(0x2A273F),
        text_primary: hex(0xE0DEF4), text_secondary: hex(0xCCCADD), text_muted: hex(0x908CAA),
        text_dim: hex(0x7E7A96), text_dimmer: hex(0x6E6A86), text_dimmest: hex(0x555168),
        text_placeholder: hex(0x6E6A86),
        accent_blue: hex(0x9CCFD8), accent_icon: hex(0xC4A7E7), accent_progress: hex(0x9CCFD8),
        btn_default: hex(0x393552), btn_hover: hex(0x44415A), btn_pressed: hex(0x56526E),
        success: hex(0x3E8FB0), success_bg: hex(0x202830),
        btn_success: hex(0x347A98), btn_success_hover: hex(0x2C6E88), btn_success_pressed: hex(0x246278),
        warning: hex(0xF6C177), warning_bg: hex(0x382820),
        error: hex(0xEB6F92), error_light: hex(0xF08098), error_bg: hex(0x381828),
        btn_danger_bg: hex(0x381828), btn_danger_hover: hex(0x582838),
        btn_trash_hover: hex(0xEB6F92), btn_trash_pressed: hex(0xCB5878),
        bg_modal_section: hex(0x232136), border_subtle: hex(0x393552), divider: hex(0x393552),
    };

    // ── Rosé Pine Dawn (light) ──
    pub const ROSEPINE_DAWN: Self = Self {
        bg_primary: hex(0xFAF4ED), bg_sidebar: hex(0xF2E9E1), bg_card: hex(0xF4EDE8),
        bg_card_hover: hex(0xDFDAD9), bg_card_pressed: hex(0xCECACD), bg_selected: hex(0xDFDAD9),
        bg_input: hex(0xF4EDE8), bg_progress: hex(0xF2E9E1),
        text_primary: hex(0x575279), text_secondary: hex(0x6E6A86), text_muted: hex(0x797593),
        text_dim: hex(0x9893A5), text_dimmer: hex(0xB0ACC0), text_dimmest: hex(0xC0BCC8),
        text_placeholder: hex(0xB0ACC0),
        accent_blue: hex(0x56949F), accent_icon: hex(0x907AA9), accent_progress: hex(0x56949F),
        btn_default: hex(0xF4EDE8), btn_hover: hex(0xDFDAD9), btn_pressed: hex(0xCECACD),
        success: hex(0x286983), success_bg: hex(0xD0E0D8),
        btn_success: hex(0x286983), btn_success_hover: hex(0x205C74), btn_success_pressed: hex(0x184F65),
        warning: hex(0xEA9D34), warning_bg: hex(0xECE0C8),
        error: hex(0xB4637A), error_light: hex(0xC87890), error_bg: hex(0xE8D0D4),
        btn_danger_bg: hex(0xE8D0D4), btn_danger_hover: hex(0xD8BCC4),
        btn_trash_hover: hex(0xB4637A), btn_trash_pressed: hex(0x965268),
        bg_modal_section: hex(0xFAF4ED), border_subtle: hex(0xDFDAD9), divider: hex(0xDFDAD9),
    };

    // ── One Dark ──
    pub const ONEDARK_DARK: Self = Self {
        bg_primary: hex(0x282C34), bg_sidebar: hex(0x21252B), bg_card: hex(0x3E4452),
        bg_card_hover: hex(0x4B5162), bg_card_pressed: hex(0x5C6370), bg_selected: hex(0x3E4452),
        bg_input: hex(0x3E4452), bg_progress: hex(0x21252B),
        text_primary: hex(0xABB2BF), text_secondary: hex(0x9DA4B0), text_muted: hex(0x848B98),
        text_dim: hex(0x6B7280), text_dimmer: hex(0x5C6370), text_dimmest: hex(0x4B5162),
        text_placeholder: hex(0x5C6370),
        accent_blue: hex(0x61AFEF), accent_icon: hex(0x56B6C2), accent_progress: hex(0x61AFEF),
        btn_default: hex(0x3E4452), btn_hover: hex(0x4B5162), btn_pressed: hex(0x5C6370),
        success: hex(0x98C379), success_bg: hex(0x1E3428),
        btn_success: hex(0x78A060), btn_success_hover: hex(0x6C9454), btn_success_pressed: hex(0x608848),
        warning: hex(0xE5C07B), warning_bg: hex(0x343020),
        error: hex(0xE06C75), error_light: hex(0xE88888), error_bg: hex(0x342028),
        btn_danger_bg: hex(0x342028), btn_danger_hover: hex(0x543038),
        btn_trash_hover: hex(0xE06C75), btn_trash_pressed: hex(0xC05860),
        bg_modal_section: hex(0x282C34), border_subtle: hex(0x3E4452), divider: hex(0x3E4452),
    };

    // ── One Dark Light ──
    pub const ONEDARK_LIGHT: Self = Self {
        bg_primary: hex(0xFAFAFA), bg_sidebar: hex(0xF0F0F0), bg_card: hex(0xE2E2E2),
        bg_card_hover: hex(0xD4D4D4), bg_card_pressed: hex(0xC8C8C8), bg_selected: hex(0xD4D4D4),
        bg_input: hex(0xE2E2E2), bg_progress: hex(0xF0F0F0),
        text_primary: hex(0x383A42), text_secondary: hex(0x4A4C56), text_muted: hex(0x686A76),
        text_dim: hex(0x818387), text_dimmer: hex(0xA0A1A7), text_dimmest: hex(0xB8B9BC),
        text_placeholder: hex(0xA0A1A7),
        accent_blue: hex(0x4078F2), accent_icon: hex(0x0184BC), accent_progress: hex(0x4078F2),
        btn_default: hex(0xE2E2E2), btn_hover: hex(0xD4D4D4), btn_pressed: hex(0xC8C8C8),
        success: hex(0x50A14F), success_bg: hex(0xD0E8D0),
        btn_success: hex(0x50A14F), btn_success_hover: hex(0x449444), btn_success_pressed: hex(0x388838),
        warning: hex(0xC18401), warning_bg: hex(0xECE0C0),
        error: hex(0xE45649), error_light: hex(0xF06860), error_bg: hex(0xEACCCC),
        btn_danger_bg: hex(0xEACCCC), btn_danger_hover: hex(0xDAB8B8),
        btn_trash_hover: hex(0xE45649), btn_trash_pressed: hex(0xC44038),
        bg_modal_section: hex(0xFAFAFA), border_subtle: hex(0xD4D4D4), divider: hex(0xD4D4D4),
    };

    // ── Monokai Pro ──
    pub const MONOKAI_PRO: Self = Self {
        bg_primary: hex(0x2D2A2E), bg_sidebar: hex(0x221F22), bg_card: hex(0x403E41),
        bg_card_hover: hex(0x525052), bg_card_pressed: hex(0x5B595C), bg_selected: hex(0x403E41),
        bg_input: hex(0x403E41), bg_progress: hex(0x221F22),
        text_primary: hex(0xFCFCFA), text_secondary: hex(0xE0E0DE), text_muted: hex(0xC1C0C0),
        text_dim: hex(0x939293), text_dimmer: hex(0x727072), text_dimmest: hex(0x5B595C),
        text_placeholder: hex(0x727072),
        accent_blue: hex(0x78DCE8), accent_icon: hex(0xAB9DF2), accent_progress: hex(0x78DCE8),
        btn_default: hex(0x403E41), btn_hover: hex(0x525052), btn_pressed: hex(0x5B595C),
        success: hex(0xA9DC76), success_bg: hex(0x222820),
        btn_success: hex(0x88B860), btn_success_hover: hex(0x7CAC54), btn_success_pressed: hex(0x70A048),
        warning: hex(0xFFD866), warning_bg: hex(0x302A20),
        error: hex(0xFF6188), error_light: hex(0xFF8098), error_bg: hex(0x341E28),
        btn_danger_bg: hex(0x341E28), btn_danger_hover: hex(0x542E38),
        btn_trash_hover: hex(0xFF6188), btn_trash_pressed: hex(0xDF5070),
        bg_modal_section: hex(0x2D2A2E), border_subtle: hex(0x403E41), divider: hex(0x403E41),
    };

    // ── Monokai Classic ──
    pub const MONOKAI_CLASSIC: Self = Self {
        bg_primary: hex(0x272822), bg_sidebar: hex(0x1A1A18), bg_card: hex(0x49483E),
        bg_card_hover: hex(0x5A5950), bg_card_pressed: hex(0x6B6A62), bg_selected: hex(0x49483E),
        bg_input: hex(0x49483E), bg_progress: hex(0x1A1A18),
        text_primary: hex(0xF8F8F2), text_secondary: hex(0xE0E0D8), text_muted: hex(0xC1C0C0),
        text_dim: hex(0x939293), text_dimmer: hex(0x75715E), text_dimmest: hex(0x5B595C),
        text_placeholder: hex(0x75715E),
        accent_blue: hex(0x66D9EF), accent_icon: hex(0xAE81FF), accent_progress: hex(0x66D9EF),
        btn_default: hex(0x49483E), btn_hover: hex(0x5A5950), btn_pressed: hex(0x6B6A62),
        success: hex(0xA6E22E), success_bg: hex(0x1E2E18),
        btn_success: hex(0x86C020), btn_success_hover: hex(0x7AB418), btn_success_pressed: hex(0x6EA810),
        warning: hex(0xE6DB74), warning_bg: hex(0x302A18),
        error: hex(0xF92672), error_light: hex(0xFF5090), error_bg: hex(0x341828),
        btn_danger_bg: hex(0x341828), btn_danger_hover: hex(0x542838),
        btn_trash_hover: hex(0xF92672), btn_trash_pressed: hex(0xD91860),
        bg_modal_section: hex(0x272822), border_subtle: hex(0x49483E), divider: hex(0x49483E),
    };

    // ── Monokai Spectrum ──
    pub const MONOKAI_SPECTRUM: Self = Self {
        bg_primary: hex(0x222222), bg_sidebar: hex(0x191919), bg_card: hex(0x363537),
        bg_card_hover: hex(0x484749), bg_card_pressed: hex(0x5A595B), bg_selected: hex(0x363537),
        bg_input: hex(0x363537), bg_progress: hex(0x191919),
        text_primary: hex(0xF7F1FF), text_secondary: hex(0xE0DAE8), text_muted: hex(0xC0BAC8),
        text_dim: hex(0x938EA0), text_dimmer: hex(0x69676C), text_dimmest: hex(0x4A484C),
        text_placeholder: hex(0x69676C),
        accent_blue: hex(0x5AD4E6), accent_icon: hex(0x948AE3), accent_progress: hex(0x5AD4E6),
        btn_default: hex(0x363537), btn_hover: hex(0x484749), btn_pressed: hex(0x5A595B),
        success: hex(0x7BD88F), success_bg: hex(0x1A2820),
        btn_success: hex(0x62B074), btn_success_hover: hex(0x58A468), btn_success_pressed: hex(0x4E985C),
        warning: hex(0xFCE566), warning_bg: hex(0x2E2A1A),
        error: hex(0xFC618D), error_light: hex(0xFF80A0), error_bg: hex(0x2E1A22),
        btn_danger_bg: hex(0x2E1A22), btn_danger_hover: hex(0x4E2A32),
        btn_trash_hover: hex(0xFC618D), btn_trash_pressed: hex(0xDC5078),
        bg_modal_section: hex(0x222222), border_subtle: hex(0x363537), divider: hex(0x363537),
    };

    // ── Ayu Dark ──
    pub const AYU_DARK: Self = Self {
        bg_primary: hex(0x0B0E14), bg_sidebar: hex(0x070A10), bg_card: hex(0x1B2028),
        bg_card_hover: hex(0x252B35), bg_card_pressed: hex(0x303842), bg_selected: hex(0x1B3A4B),
        bg_input: hex(0x1B2028), bg_progress: hex(0x070A10),
        text_primary: hex(0xBFBDB6), text_secondary: hex(0xA8A6A0), text_muted: hex(0x8A8880),
        text_dim: hex(0x7A786E), text_dimmer: hex(0x626A73), text_dimmest: hex(0x4A505A),
        text_placeholder: hex(0x626A73),
        accent_blue: hex(0xE6B450), accent_icon: hex(0x59C2FF), accent_progress: hex(0xE6B450),
        btn_default: hex(0x1B2028), btn_hover: hex(0x252B35), btn_pressed: hex(0x303842),
        success: hex(0xAAD94C), success_bg: hex(0x0C1A10),
        btn_success: hex(0x88B040), btn_success_hover: hex(0x7CA436), btn_success_pressed: hex(0x70982C),
        warning: hex(0xFFB454), warning_bg: hex(0x1A1808),
        error: hex(0xD95757), error_light: hex(0xE87070), error_bg: hex(0x1A0C0C),
        btn_danger_bg: hex(0x1A0C0C), btn_danger_hover: hex(0x2A1C1C),
        btn_trash_hover: hex(0xD95757), btn_trash_pressed: hex(0xB94848),
        bg_modal_section: hex(0x0B0E14), border_subtle: hex(0x1B2028), divider: hex(0x1B2028),
    };

    // ── Ayu Mirage ──
    pub const AYU_MIRAGE: Self = Self {
        bg_primary: hex(0x1F2430), bg_sidebar: hex(0x1A1F2B), bg_card: hex(0x2A303E),
        bg_card_hover: hex(0x33415E), bg_card_pressed: hex(0x3D4A68), bg_selected: hex(0x33415E),
        bg_input: hex(0x2A303E), bg_progress: hex(0x1A1F2B),
        text_primary: hex(0xCCCAC2), text_secondary: hex(0xB4B2AA), text_muted: hex(0x9A988E),
        text_dim: hex(0x858380), text_dimmer: hex(0x707A8C), text_dimmest: hex(0x555D6E),
        text_placeholder: hex(0x707A8C),
        accent_blue: hex(0xFFCC66), accent_icon: hex(0x73D0FF), accent_progress: hex(0xFFCC66),
        btn_default: hex(0x2A303E), btn_hover: hex(0x33415E), btn_pressed: hex(0x3D4A68),
        success: hex(0xBAE67E), success_bg: hex(0x1A2A1E),
        btn_success: hex(0x98C066), btn_success_hover: hex(0x8CB45A), btn_success_pressed: hex(0x80A84E),
        warning: hex(0xFFD580), warning_bg: hex(0x2A2818),
        error: hex(0xF28779), error_light: hex(0xFF9E90), error_bg: hex(0x2A1E1E),
        btn_danger_bg: hex(0x2A1E1E), btn_danger_hover: hex(0x4A2E2E),
        btn_trash_hover: hex(0xF28779), btn_trash_pressed: hex(0xD27060),
        bg_modal_section: hex(0x1F2430), border_subtle: hex(0x2A303E), divider: hex(0x2A303E),
    };

    // ── Ayu Light ──
    pub const AYU_LIGHT: Self = Self {
        bg_primary: hex(0xFAFAFA), bg_sidebar: hex(0xF0EEE4), bg_card: hex(0xE8E6DC),
        bg_card_hover: hex(0xDCD8CC), bg_card_pressed: hex(0xD0CCC0), bg_selected: hex(0xD1E4F4),
        bg_input: hex(0xE8E6DC), bg_progress: hex(0xF0EEE4),
        text_primary: hex(0x575F66), text_secondary: hex(0x6B737A), text_muted: hex(0x848C94),
        text_dim: hex(0x9CA4AC), text_dimmer: hex(0xABB0B6), text_dimmest: hex(0xC0C4C8),
        text_placeholder: hex(0xABB0B6),
        accent_blue: hex(0xFF9940), accent_icon: hex(0x36A3D9), accent_progress: hex(0xFF9940),
        btn_default: hex(0xE8E6DC), btn_hover: hex(0xDCD8CC), btn_pressed: hex(0xD0CCC0),
        success: hex(0x86B300), success_bg: hex(0xD8E8C8),
        btn_success: hex(0x86B300), btn_success_hover: hex(0x78A400), btn_success_pressed: hex(0x6A9600),
        warning: hex(0xF29718), warning_bg: hex(0xEDE0C0),
        error: hex(0xF51818), error_light: hex(0xF04040), error_bg: hex(0xEEC8C8),
        btn_danger_bg: hex(0xEEC8C8), btn_danger_hover: hex(0xDEB0B0),
        btn_trash_hover: hex(0xF51818), btn_trash_pressed: hex(0xD50E0E),
        bg_modal_section: hex(0xFAFAFA), border_subtle: hex(0xDCD8CC), divider: hex(0xDCD8CC),
    };

    // ── Everforest Dark ──
    pub const EVERFOREST_DARK: Self = Self {
        bg_primary: hex(0x2D353B), bg_sidebar: hex(0x232A2E), bg_card: hex(0x343F44),
        bg_card_hover: hex(0x3D484D), bg_card_pressed: hex(0x475258), bg_selected: hex(0x543A48),
        bg_input: hex(0x343F44), bg_progress: hex(0x232A2E),
        text_primary: hex(0xD3C6AA), text_secondary: hex(0xC0B498), text_muted: hex(0xA09880),
        text_dim: hex(0x859289), text_dimmer: hex(0x7A8478), text_dimmest: hex(0x5C6A62),
        text_placeholder: hex(0x7A8478),
        accent_blue: hex(0x7FBBB3), accent_icon: hex(0xA7C080), accent_progress: hex(0x7FBBB3),
        btn_default: hex(0x343F44), btn_hover: hex(0x3D484D), btn_pressed: hex(0x475258),
        success: hex(0xA7C080), success_bg: hex(0x1E2E24),
        btn_success: hex(0x86A066), btn_success_hover: hex(0x7A945A), btn_success_pressed: hex(0x6E884E),
        warning: hex(0xDBBC7F), warning_bg: hex(0x2E2A1E),
        error: hex(0xE67E80), error_light: hex(0xF09090), error_bg: hex(0x2E2020),
        btn_danger_bg: hex(0x2E2020), btn_danger_hover: hex(0x4E3030),
        btn_trash_hover: hex(0xE67E80), btn_trash_pressed: hex(0xC66868),
        bg_modal_section: hex(0x2D353B), border_subtle: hex(0x343F44), divider: hex(0x343F44),
    };

    // ── Everforest Light ──
    pub const EVERFOREST_LIGHT: Self = Self {
        bg_primary: hex(0xFDF6E3), bg_sidebar: hex(0xEFECD4), bg_card: hex(0xF4F0D9),
        bg_card_hover: hex(0xE6E2CC), bg_card_pressed: hex(0xE0DCC7), bg_selected: hex(0xEADDC0),
        bg_input: hex(0xF4F0D9), bg_progress: hex(0xEFECD4),
        text_primary: hex(0x5C6A72), text_secondary: hex(0x6E7A80), text_muted: hex(0x829181),
        text_dim: hex(0x939F91), text_dimmer: hex(0xA6B0A0), text_dimmest: hex(0xBCC5B8),
        text_placeholder: hex(0xA6B0A0),
        accent_blue: hex(0x3A94C5), accent_icon: hex(0x8DA101), accent_progress: hex(0x3A94C5),
        btn_default: hex(0xF4F0D9), btn_hover: hex(0xE6E2CC), btn_pressed: hex(0xE0DCC7),
        success: hex(0x8DA101), success_bg: hex(0xD0E0C4),
        btn_success: hex(0x8DA101), btn_success_hover: hex(0x7E9200), btn_success_pressed: hex(0x6F8300),
        warning: hex(0xDFA000), warning_bg: hex(0xECE0B8),
        error: hex(0xF85552), error_light: hex(0xE86868), error_bg: hex(0xECC8C4),
        btn_danger_bg: hex(0xECC8C4), btn_danger_hover: hex(0xDCB0AC),
        btn_trash_hover: hex(0xF85552), btn_trash_pressed: hex(0xD84040),
        bg_modal_section: hex(0xFDF6E3), border_subtle: hex(0xE6E2CC), divider: hex(0xE6E2CC),
    };

    // ── Material Oceanic ──
    pub const MATERIAL_OCEANIC: Self = Self {
        bg_primary: hex(0x263238), bg_sidebar: hex(0x1E272C), bg_card: hex(0x2E3C42),
        bg_card_hover: hex(0x3A4A52), bg_card_pressed: hex(0x465862), bg_selected: hex(0x546E7A),
        bg_input: hex(0x2E3C42), bg_progress: hex(0x1E272C),
        text_primary: hex(0xB0BEC5), text_secondary: hex(0x9AAAB2), text_muted: hex(0x849AA4),
        text_dim: hex(0x6E8490), text_dimmer: hex(0x546E7A), text_dimmest: hex(0x405A64),
        text_placeholder: hex(0x546E7A),
        accent_blue: hex(0x89DDFF), accent_icon: hex(0x80CBC4), accent_progress: hex(0x89DDFF),
        btn_default: hex(0x2E3C42), btn_hover: hex(0x3A4A52), btn_pressed: hex(0x465862),
        success: hex(0xC3E88D), success_bg: hex(0x1A3028),
        btn_success: hex(0xA0C070), btn_success_hover: hex(0x94B464), btn_success_pressed: hex(0x88A858),
        warning: hex(0xFFCB6B), warning_bg: hex(0x2E2E1E),
        error: hex(0xFF5370), error_light: hex(0xFF7088), error_bg: hex(0x2E1E22),
        btn_danger_bg: hex(0x2E1E22), btn_danger_hover: hex(0x4E2E32),
        btn_trash_hover: hex(0xFF5370), btn_trash_pressed: hex(0xDF4058),
        bg_modal_section: hex(0x263238), border_subtle: hex(0x2E3C42), divider: hex(0x2E3C42),
    };

    // ── Material Palenight ──
    pub const MATERIAL_PALENIGHT: Self = Self {
        bg_primary: hex(0x292D3E), bg_sidebar: hex(0x232838), bg_card: hex(0x343A4E),
        bg_card_hover: hex(0x414662), bg_card_pressed: hex(0x515772), bg_selected: hex(0x717CB4),
        bg_input: hex(0x343A4E), bg_progress: hex(0x232838),
        text_primary: hex(0xA6ACCD), text_secondary: hex(0x929ABE), text_muted: hex(0x7E86AE),
        text_dim: hex(0x717CB4), text_dimmer: hex(0x676E95), text_dimmest: hex(0x515772),
        text_placeholder: hex(0x676E95),
        accent_blue: hex(0xC792EA), accent_icon: hex(0x82AAFF), accent_progress: hex(0xC792EA),
        btn_default: hex(0x343A4E), btn_hover: hex(0x414662), btn_pressed: hex(0x515772),
        success: hex(0xC3E88D), success_bg: hex(0x202E28),
        btn_success: hex(0xA0C070), btn_success_hover: hex(0x94B464), btn_success_pressed: hex(0x88A858),
        warning: hex(0xFFCB6B), warning_bg: hex(0x2E2C1E),
        error: hex(0xFF5370), error_light: hex(0xFF7088), error_bg: hex(0x2E1E22),
        btn_danger_bg: hex(0x2E1E22), btn_danger_hover: hex(0x4E2E32),
        btn_trash_hover: hex(0xFF5370), btn_trash_pressed: hex(0xDF4058),
        bg_modal_section: hex(0x292D3E), border_subtle: hex(0x343A4E), divider: hex(0x343A4E),
    };

    // ── Material Deep Ocean ──
    pub const MATERIAL_DEEPOCEAN: Self = Self {
        bg_primary: hex(0x0F111A), bg_sidebar: hex(0x090B10), bg_card: hex(0x1A1C28),
        bg_card_hover: hex(0x252836), bg_card_pressed: hex(0x3B3F51), bg_selected: hex(0x44475A),
        bg_input: hex(0x1A1C28), bg_progress: hex(0x090B10),
        text_primary: hex(0x8F93A2), text_secondary: hex(0xA0A4B4), text_muted: hex(0x7880A0),
        text_dim: hex(0x606888), text_dimmer: hex(0x464B5D), text_dimmest: hex(0x3B3F51),
        text_placeholder: hex(0x464B5D),
        accent_blue: hex(0x84FFFF), accent_icon: hex(0x82AAFF), accent_progress: hex(0x84FFFF),
        btn_default: hex(0x1A1C28), btn_hover: hex(0x252836), btn_pressed: hex(0x3B3F51),
        success: hex(0xC3E88D), success_bg: hex(0x0C1A14),
        btn_success: hex(0xA0C070), btn_success_hover: hex(0x94B464), btn_success_pressed: hex(0x88A858),
        warning: hex(0xFFCB6B), warning_bg: hex(0x1A180C),
        error: hex(0xFF5370), error_light: hex(0xFF7088), error_bg: hex(0x1A0C10),
        btn_danger_bg: hex(0x1A0C10), btn_danger_hover: hex(0x3A1C20),
        btn_trash_hover: hex(0xFF5370), btn_trash_pressed: hex(0xDF4058),
        bg_modal_section: hex(0x0F111A), border_subtle: hex(0x1A1C28), divider: hex(0x1A1C28),
    };

    // ── Flexoki Dark ──
    pub const FLEXOKI_DARK: Self = Self {
        bg_primary: hex(0x100F0F), bg_sidebar: hex(0x1C1B1A), bg_card: hex(0x282726),
        bg_card_hover: hex(0x343331), bg_card_pressed: hex(0x403E3C), bg_selected: hex(0x403E3C),
        bg_input: hex(0x282726), bg_progress: hex(0x1C1B1A),
        text_primary: hex(0xCECDC3), text_secondary: hex(0xB7B5AC), text_muted: hex(0x9F9D96),
        text_dim: hex(0x878580), text_dimmer: hex(0x6F6E69), text_dimmest: hex(0x575653),
        text_placeholder: hex(0x6F6E69),
        accent_blue: hex(0x4385BE), accent_icon: hex(0xDA702C), accent_progress: hex(0x4385BE),
        btn_default: hex(0x282726), btn_hover: hex(0x343331), btn_pressed: hex(0x403E3C),
        success: hex(0x879A39), success_bg: hex(0x141A10),
        btn_success: hex(0x6E8030), btn_success_hover: hex(0x627428), btn_success_pressed: hex(0x566820),
        warning: hex(0xD0A215), warning_bg: hex(0x1E1A0C),
        error: hex(0xD14D41), error_light: hex(0xE06858), error_bg: hex(0x1E100C),
        btn_danger_bg: hex(0x1E100C), btn_danger_hover: hex(0x3E201C),
        btn_trash_hover: hex(0xD14D41), btn_trash_pressed: hex(0xB13830),
        bg_modal_section: hex(0x100F0F), border_subtle: hex(0x282726), divider: hex(0x282726),
    };

    // ── Flexoki Light ──
    pub const FLEXOKI_LIGHT: Self = Self {
        bg_primary: hex(0xFFFCF0), bg_sidebar: hex(0xF2F0E5), bg_card: hex(0xE6E4D9),
        bg_card_hover: hex(0xDAD8CE), bg_card_pressed: hex(0xCECDC3), bg_selected: hex(0xE6E4D9),
        bg_input: hex(0xE6E4D9), bg_progress: hex(0xF2F0E5),
        text_primary: hex(0x403E3C), text_secondary: hex(0x575653), text_muted: hex(0x6F6E69),
        text_dim: hex(0x878580), text_dimmer: hex(0x9F9D96), text_dimmest: hex(0xB7B5AC),
        text_placeholder: hex(0x9F9D96),
        accent_blue: hex(0x205EA6), accent_icon: hex(0xBC5215), accent_progress: hex(0x205EA6),
        btn_default: hex(0xE6E4D9), btn_hover: hex(0xDAD8CE), btn_pressed: hex(0xCECDC3),
        success: hex(0x66800B), success_bg: hex(0xD0E0C0),
        btn_success: hex(0x66800B), btn_success_hover: hex(0x587208), btn_success_pressed: hex(0x4A6405),
        warning: hex(0xAD8301), warning_bg: hex(0xE0DCC0),
        error: hex(0xAF3029), error_light: hex(0xD14D41), error_bg: hex(0xE0C8C0),
        btn_danger_bg: hex(0xE0C8C0), btn_danger_hover: hex(0xD0B4AC),
        btn_trash_hover: hex(0xAF3029), btn_trash_pressed: hex(0x902420),
        bg_modal_section: hex(0xFFFCF0), border_subtle: hex(0xDAD8CE), divider: hex(0xDAD8CE),
    };

    // ── Nightfox ──
    pub const NIGHTFOX: Self = Self {
        bg_primary: hex(0x192330), bg_sidebar: hex(0x131A24), bg_card: hex(0x2B3B51),
        bg_card_hover: hex(0x3C5372), bg_card_pressed: hex(0x39506D), bg_selected: hex(0x2B3B51),
        bg_input: hex(0x2B3B51), bg_progress: hex(0x131A24),
        text_primary: hex(0xCDCECF), text_secondary: hex(0xAEAFB0), text_muted: hex(0x8E9098),
        text_dim: hex(0x738091), text_dimmer: hex(0x5C6A7C), text_dimmest: hex(0x3E4E62),
        text_placeholder: hex(0x5C6A7C),
        accent_blue: hex(0x719CD6), accent_icon: hex(0x63CDCF), accent_progress: hex(0x719CD6),
        btn_default: hex(0x2B3B51), btn_hover: hex(0x3C5372), btn_pressed: hex(0x39506D),
        success: hex(0x81B29A), success_bg: hex(0x142820),
        btn_success: hex(0x68907E), btn_success_hover: hex(0x5C8472), btn_success_pressed: hex(0x507866),
        warning: hex(0xDBC074), warning_bg: hex(0x282418),
        error: hex(0xC94F6D), error_light: hex(0xD66880), error_bg: hex(0x281820),
        btn_danger_bg: hex(0x281820), btn_danger_hover: hex(0x482830),
        btn_trash_hover: hex(0xC94F6D), btn_trash_pressed: hex(0xA9405C),
        bg_modal_section: hex(0x192330), border_subtle: hex(0x2B3B51), divider: hex(0x2B3B51),
    };

    // ── Dawnfox (light) ──
    pub const DAWNFOX: Self = Self {
        bg_primary: hex(0xFAF4ED), bg_sidebar: hex(0xEBE5DF), bg_card: hex(0xEBD8CE),
        bg_card_hover: hex(0xDACDC3), bg_card_pressed: hex(0xC8BEB4), bg_selected: hex(0xEBD8CE),
        bg_input: hex(0xEBD8CE), bg_progress: hex(0xEBE5DF),
        text_primary: hex(0x575279), text_secondary: hex(0x625C87), text_muted: hex(0x6E6A86),
        text_dim: hex(0x9893A5), text_dimmer: hex(0xAEA8B8), text_dimmest: hex(0xC0BAC8),
        text_placeholder: hex(0xAEA8B8),
        accent_blue: hex(0x286983), accent_icon: hex(0x907AA9), accent_progress: hex(0x286983),
        btn_default: hex(0xEBD8CE), btn_hover: hex(0xDACDC3), btn_pressed: hex(0xC8BEB4),
        success: hex(0x618774), success_bg: hex(0xD0E0D4),
        btn_success: hex(0x618774), btn_success_hover: hex(0x547A68), btn_success_pressed: hex(0x476D5C),
        warning: hex(0xEA9D34), warning_bg: hex(0xECE0C8),
        error: hex(0xB4637A), error_light: hex(0xC87890), error_bg: hex(0xE8D0D4),
        btn_danger_bg: hex(0xE8D0D4), btn_danger_hover: hex(0xD8BCC4),
        btn_trash_hover: hex(0xB4637A), btn_trash_pressed: hex(0x965268),
        bg_modal_section: hex(0xFAF4ED), border_subtle: hex(0xDACDC3), divider: hex(0xDACDC3),
    };

    // ── Sonokai ──
    pub const SONOKAI_DEFAULT: Self = Self {
        bg_primary: hex(0x2C2E34), bg_sidebar: hex(0x242529), bg_card: hex(0x33353F),
        bg_card_hover: hex(0x3B3E48), bg_card_pressed: hex(0x414550), bg_selected: hex(0x3B3E48),
        bg_input: hex(0x33353F), bg_progress: hex(0x242529),
        text_primary: hex(0xE2E2E3), text_secondary: hex(0xCCCCD0), text_muted: hex(0xA8A8B0),
        text_dim: hex(0x8C8C96), text_dimmer: hex(0x7F8490), text_dimmest: hex(0x585C68),
        text_placeholder: hex(0x7F8490),
        accent_blue: hex(0x76CCE0), accent_icon: hex(0xB39DF3), accent_progress: hex(0x76CCE0),
        btn_default: hex(0x33353F), btn_hover: hex(0x3B3E48), btn_pressed: hex(0x414550),
        success: hex(0x9ED072), success_bg: hex(0x1E2E22),
        btn_success: hex(0x7EAA5A), btn_success_hover: hex(0x729E4E), btn_success_pressed: hex(0x669242),
        warning: hex(0xE7C664), warning_bg: hex(0x2E2A1A),
        error: hex(0xFC5D7C), error_light: hex(0xFF7890), error_bg: hex(0x2E1A20),
        btn_danger_bg: hex(0x2E1A20), btn_danger_hover: hex(0x4E2A30),
        btn_trash_hover: hex(0xFC5D7C), btn_trash_pressed: hex(0xDC4C66),
        bg_modal_section: hex(0x2C2E34), border_subtle: hex(0x33353F), divider: hex(0x33353F),
    };

    // ── Oxocarbon Dark ──
    pub const OXOCARBON_DARK: Self = Self {
        bg_primary: hex(0x161616), bg_sidebar: hex(0x0E0E0E), bg_card: hex(0x262626),
        bg_card_hover: hex(0x393939), bg_card_pressed: hex(0x525252), bg_selected: hex(0x393939),
        bg_input: hex(0x262626), bg_progress: hex(0x0E0E0E),
        text_primary: hex(0xF2F4F8), text_secondary: hex(0xDDE1E6), text_muted: hex(0xB0B4BC),
        text_dim: hex(0x8A8E96), text_dimmer: hex(0x6E7278), text_dimmest: hex(0x525252),
        text_placeholder: hex(0x6E7278),
        accent_blue: hex(0x78A9FF), accent_icon: hex(0xBE95FF), accent_progress: hex(0x78A9FF),
        btn_default: hex(0x262626), btn_hover: hex(0x393939), btn_pressed: hex(0x525252),
        success: hex(0x42BE65), success_bg: hex(0x0C1C12),
        btn_success: hex(0x359E52), btn_success_hover: hex(0x2C9048), btn_success_pressed: hex(0x24823E),
        warning: hex(0x08BDBA), warning_bg: hex(0x0C1A1A),
        error: hex(0xEE5396), error_light: hex(0xFF7EB6), error_bg: hex(0x1C0C14),
        btn_danger_bg: hex(0x1C0C14), btn_danger_hover: hex(0x3C1C24),
        btn_trash_hover: hex(0xEE5396), btn_trash_pressed: hex(0xCE4480),
        bg_modal_section: hex(0x161616), border_subtle: hex(0x262626), divider: hex(0x262626),
    };

    // ── Oxocarbon Light ──
    pub const OXOCARBON_LIGHT: Self = Self {
        bg_primary: hex(0xFFFFFF), bg_sidebar: hex(0xF2F4F8), bg_card: hex(0xDDE1E6),
        bg_card_hover: hex(0xC8CCD2), bg_card_pressed: hex(0xB4B8C0), bg_selected: hex(0xDDE1E6),
        bg_input: hex(0xDDE1E6), bg_progress: hex(0xF2F4F8),
        text_primary: hex(0x262626), text_secondary: hex(0x393939), text_muted: hex(0x525252),
        text_dim: hex(0x6E7278), text_dimmer: hex(0x8A8E96), text_dimmest: hex(0xB0B4BC),
        text_placeholder: hex(0x8A8E96),
        accent_blue: hex(0x0F62FE), accent_icon: hex(0x8A3FFC), accent_progress: hex(0x0F62FE),
        btn_default: hex(0xDDE1E6), btn_hover: hex(0xC8CCD2), btn_pressed: hex(0xB4B8C0),
        success: hex(0x198038), success_bg: hex(0xD0F0D8),
        btn_success: hex(0x198038), btn_success_hover: hex(0x14702E), btn_success_pressed: hex(0x106024),
        warning: hex(0x005D5D), warning_bg: hex(0xD0E8E8),
        error: hex(0xDA1E28), error_light: hex(0xEE5396), error_bg: hex(0xF0D0D8),
        btn_danger_bg: hex(0xF0D0D8), btn_danger_hover: hex(0xE0B8C4),
        btn_trash_hover: hex(0xDA1E28), btn_trash_pressed: hex(0xBA1420),
        bg_modal_section: hex(0xFFFFFF), border_subtle: hex(0xC8CCD2), divider: hex(0xC8CCD2),
    };

    // ── Night Owl Dark ──
    pub const NIGHTOWL_DARK: Self = Self {
        bg_primary: hex(0x011627), bg_sidebar: hex(0x011221), bg_card: hex(0x0B2942),
        bg_card_hover: hex(0x1D3B53), bg_card_pressed: hex(0x2A4F6C), bg_selected: hex(0x1D3B53),
        bg_input: hex(0x0B2942), bg_progress: hex(0x011221),
        text_primary: hex(0xD6DEEB), text_secondary: hex(0xB4C0D0), text_muted: hex(0x8CA0B4),
        text_dim: hex(0x7E94A8), text_dimmer: hex(0x637777), text_dimmest: hex(0x4A6060),
        text_placeholder: hex(0x637777),
        accent_blue: hex(0x82AAFF), accent_icon: hex(0x7FDBCA), accent_progress: hex(0x82AAFF),
        btn_default: hex(0x0B2942), btn_hover: hex(0x1D3B53), btn_pressed: hex(0x2A4F6C),
        success: hex(0xADDB67), success_bg: hex(0x011E14),
        btn_success: hex(0x8CB852), btn_success_hover: hex(0x80AC48), btn_success_pressed: hex(0x74A03E),
        warning: hex(0xECC48D), warning_bg: hex(0x1A1808),
        error: hex(0xEF5350), error_light: hex(0xFF6E6A), error_bg: hex(0x1A0808),
        btn_danger_bg: hex(0x1A0808), btn_danger_hover: hex(0x3A1818),
        btn_trash_hover: hex(0xEF5350), btn_trash_pressed: hex(0xCF4040),
        bg_modal_section: hex(0x011627), border_subtle: hex(0x0B2942), divider: hex(0x0B2942),
    };

    // ── Night Owl Light ──
    pub const NIGHTOWL_LIGHT: Self = Self {
        bg_primary: hex(0xFBFBFB), bg_sidebar: hex(0xF0F0F0), bg_card: hex(0xE8E8E8),
        bg_card_hover: hex(0xE0E0E0), bg_card_pressed: hex(0xD4D4D4), bg_selected: hex(0xE0E0E0),
        bg_input: hex(0xE8E8E8), bg_progress: hex(0xF0F0F0),
        text_primary: hex(0x403F53), text_secondary: hex(0x565570), text_muted: hex(0x6E6D88),
        text_dim: hex(0x8888A0), text_dimmer: hex(0x989FB1), text_dimmest: hex(0xB0B4C0),
        text_placeholder: hex(0x989FB1),
        accent_blue: hex(0x4876D6), accent_icon: hex(0x0C969B), accent_progress: hex(0x4876D6),
        btn_default: hex(0xE8E8E8), btn_hover: hex(0xE0E0E0), btn_pressed: hex(0xD4D4D4),
        success: hex(0x2AA298), success_bg: hex(0xD0E8E0),
        btn_success: hex(0x2AA298), btn_success_hover: hex(0x22948C), btn_success_pressed: hex(0x1A8680),
        warning: hex(0xD98E24), warning_bg: hex(0xECE0C8),
        error: hex(0xDE3D3B), error_light: hex(0xE85858), error_bg: hex(0xECC8C8),
        btn_danger_bg: hex(0xECC8C8), btn_danger_hover: hex(0xDCB0B0),
        btn_trash_hover: hex(0xDE3D3B), btn_trash_pressed: hex(0xBE2E2C),
        bg_modal_section: hex(0xFBFBFB), border_subtle: hex(0xE0E0E0), divider: hex(0xE0E0E0),
    };

    // ── Iceberg Dark ──
    pub const ICEBERG_DARK: Self = Self {
        bg_primary: hex(0x161821), bg_sidebar: hex(0x0F1117), bg_card: hex(0x1E2132),
        bg_card_hover: hex(0x2E313F), bg_card_pressed: hex(0x3D425B), bg_selected: hex(0x2E313F),
        bg_input: hex(0x1E2132), bg_progress: hex(0x0F1117),
        text_primary: hex(0xC6C8D1), text_secondary: hex(0xB0B4C0), text_muted: hex(0x9498A8),
        text_dim: hex(0x818596), text_dimmer: hex(0x6B7089), text_dimmest: hex(0x4E5268),
        text_placeholder: hex(0x6B7089),
        accent_blue: hex(0x84A0C6), accent_icon: hex(0x89B8C2), accent_progress: hex(0x84A0C6),
        btn_default: hex(0x1E2132), btn_hover: hex(0x2E313F), btn_pressed: hex(0x3D425B),
        success: hex(0xB4BE82), success_bg: hex(0x141E18),
        btn_success: hex(0x949E68), btn_success_hover: hex(0x88925C), btn_success_pressed: hex(0x7C8650),
        warning: hex(0xE2A478), warning_bg: hex(0x201C14),
        error: hex(0xE27878), error_light: hex(0xF09090), error_bg: hex(0x201418),
        btn_danger_bg: hex(0x201418), btn_danger_hover: hex(0x402428),
        btn_trash_hover: hex(0xE27878), btn_trash_pressed: hex(0xC26060),
        bg_modal_section: hex(0x161821), border_subtle: hex(0x1E2132), divider: hex(0x1E2132),
    };

    // ── Iceberg Light ──
    pub const ICEBERG_LIGHT: Self = Self {
        bg_primary: hex(0xE8E9EC), bg_sidebar: hex(0xDCDFE7), bg_card: hex(0xD0D4DE),
        bg_card_hover: hex(0xCAD0DE), bg_card_pressed: hex(0xBEC4D2), bg_selected: hex(0xCAD0DE),
        bg_input: hex(0xD0D4DE), bg_progress: hex(0xDCDFE7),
        text_primary: hex(0x33374C), text_secondary: hex(0x444862), text_muted: hex(0x5C6080),
        text_dim: hex(0x747896), text_dimmer: hex(0x8B98B6), text_dimmest: hex(0xA8B0C8),
        text_placeholder: hex(0x8B98B6),
        accent_blue: hex(0x2D539E), accent_icon: hex(0x33635C), accent_progress: hex(0x2D539E),
        btn_default: hex(0xD0D4DE), btn_hover: hex(0xCAD0DE), btn_pressed: hex(0xBEC4D2),
        success: hex(0x668E3D), success_bg: hex(0xD0E0C8),
        btn_success: hex(0x668E3D), btn_success_hover: hex(0x588032), btn_success_pressed: hex(0x4A7228),
        warning: hex(0xC57339), warning_bg: hex(0xE4D8C8),
        error: hex(0xCC517A), error_light: hex(0xDD6890), error_bg: hex(0xE4C8D4),
        btn_danger_bg: hex(0xE4C8D4), btn_danger_hover: hex(0xD4B4C0),
        btn_trash_hover: hex(0xCC517A), btn_trash_pressed: hex(0xAC4068),
        bg_modal_section: hex(0xE8E9EC), border_subtle: hex(0xCAD0DE), divider: hex(0xCAD0DE),
    };

    // ── Horizon Dark ──
    pub const HORIZON_DARK: Self = Self {
        bg_primary: hex(0x1C1E26), bg_sidebar: hex(0x16161C), bg_card: hex(0x2E303E),
        bg_card_hover: hex(0x3A3C4E), bg_card_pressed: hex(0x484A60), bg_selected: hex(0x2E303E),
        bg_input: hex(0x2E303E), bg_progress: hex(0x16161C),
        text_primary: hex(0xD5D8DA), text_secondary: hex(0xBEC0C4), text_muted: hex(0xA0A2A8),
        text_dim: hex(0x888A92), text_dimmer: hex(0x6C6F93), text_dimmest: hex(0x50526E),
        text_placeholder: hex(0x6C6F93),
        accent_blue: hex(0x26BBD9), accent_icon: hex(0xB877DB), accent_progress: hex(0x26BBD9),
        btn_default: hex(0x2E303E), btn_hover: hex(0x3A3C4E), btn_pressed: hex(0x484A60),
        success: hex(0x29D398), success_bg: hex(0x141E1C),
        btn_success: hex(0x22B480), btn_success_hover: hex(0x1CA874), btn_success_pressed: hex(0x169C68),
        warning: hex(0xFAC29A), warning_bg: hex(0x2A2218),
        error: hex(0xE95678), error_light: hex(0xF07090), error_bg: hex(0x2A1620),
        btn_danger_bg: hex(0x2A1620), btn_danger_hover: hex(0x4A2630),
        btn_trash_hover: hex(0xE95678), btn_trash_pressed: hex(0xC94460),
        bg_modal_section: hex(0x1C1E26), border_subtle: hex(0x2E303E), divider: hex(0x2E303E),
    };

    // ── Melange Dark ──
    pub const MELANGE_DARK: Self = Self {
        bg_primary: hex(0x292522), bg_sidebar: hex(0x221E1B), bg_card: hex(0x34302C),
        bg_card_hover: hex(0x403A36), bg_card_pressed: hex(0x4C4640), bg_selected: hex(0x403A36),
        bg_input: hex(0x34302C), bg_progress: hex(0x221E1B),
        text_primary: hex(0xECE1D7), text_secondary: hex(0xD4C8BC), text_muted: hex(0xBAB0A4),
        text_dim: hex(0x9E9488), text_dimmer: hex(0x867462), text_dimmest: hex(0x6A5E50),
        text_placeholder: hex(0x867462),
        accent_blue: hex(0xA3A9CE), accent_icon: hex(0x89B3B6), accent_progress: hex(0xA3A9CE),
        btn_default: hex(0x34302C), btn_hover: hex(0x403A36), btn_pressed: hex(0x4C4640),
        success: hex(0x85B695), success_bg: hex(0x1E2820),
        btn_success: hex(0x6C9A7C), btn_success_hover: hex(0x608E70), btn_success_pressed: hex(0x548264),
        warning: hex(0xEBC06D), warning_bg: hex(0x2A261A),
        error: hex(0xD47766), error_light: hex(0xE89080), error_bg: hex(0x2A1C18),
        btn_danger_bg: hex(0x2A1C18), btn_danger_hover: hex(0x4A2C28),
        btn_trash_hover: hex(0xD47766), btn_trash_pressed: hex(0xB46050),
        bg_modal_section: hex(0x292522), border_subtle: hex(0x34302C), divider: hex(0x34302C),
    };

    // ── Melange Light ──
    pub const MELANGE_LIGHT: Self = Self {
        bg_primary: hex(0xF4F0ED), bg_sidebar: hex(0xE9E1DB), bg_card: hex(0xDDD2C8),
        bg_card_hover: hex(0xD0C6BA), bg_card_pressed: hex(0xC4BAAE), bg_selected: hex(0xDDD2C8),
        bg_input: hex(0xDDD2C8), bg_progress: hex(0xE9E1DB),
        text_primary: hex(0x54433A), text_secondary: hex(0x6B5C4D), text_muted: hex(0x7E6E5E),
        text_dim: hex(0x948472), text_dimmer: hex(0xA89888), text_dimmest: hex(0xBEB0A0),
        text_placeholder: hex(0xA89888),
        accent_blue: hex(0x5E6DAB), accent_icon: hex(0x3F7C82), accent_progress: hex(0x5E6DAB),
        btn_default: hex(0xDDD2C8), btn_hover: hex(0xD0C6BA), btn_pressed: hex(0xC4BAAE),
        success: hex(0x4E7548), success_bg: hex(0xD0E0D0),
        btn_success: hex(0x4E7548), btn_success_hover: hex(0x42683C), btn_success_pressed: hex(0x365C30),
        warning: hex(0x9A7C24), warning_bg: hex(0xE0DCC0),
        error: hex(0xA44C36), error_light: hex(0xC06048), error_bg: hex(0xE0CCC4),
        btn_danger_bg: hex(0xE0CCC4), btn_danger_hover: hex(0xD0B8B0),
        btn_trash_hover: hex(0xA44C36), btn_trash_pressed: hex(0x843C28),
        bg_modal_section: hex(0xF4F0ED), border_subtle: hex(0xD0C6BA), divider: hex(0xD0C6BA),
    };

    // ── Synthwave '84 ──
    pub const SYNTHWAVE_DARK: Self = Self {
        bg_primary: hex(0x262335), bg_sidebar: hex(0x241B2F), bg_card: hex(0x34294F),
        bg_card_hover: hex(0x3E3460), bg_card_pressed: hex(0x463465), bg_selected: hex(0x463465),
        bg_input: hex(0x2A2139), bg_progress: hex(0x241B2F),
        text_primary: hex(0xFFFFFF), text_secondary: hex(0xE2E2E2), text_muted: hex(0xC0BCD0),
        text_dim: hex(0xA09CB4), text_dimmer: hex(0x848BBD), text_dimmest: hex(0x614D85),
        text_placeholder: hex(0x848BBD),
        accent_blue: hex(0xFF7EDB), accent_icon: hex(0x36F9F6), accent_progress: hex(0xFF7EDB),
        btn_default: hex(0x34294F), btn_hover: hex(0x3E3460), btn_pressed: hex(0x463465),
        success: hex(0x72F1B8), success_bg: hex(0x1A2828),
        btn_success: hex(0x5CC898), btn_success_hover: hex(0x50BC8C), btn_success_pressed: hex(0x44B080),
        warning: hex(0xFEDE5D), warning_bg: hex(0x2A281A),
        error: hex(0xFE4450), error_light: hex(0xFF6670), error_bg: hex(0x2A1A1E),
        btn_danger_bg: hex(0x2A1A1E), btn_danger_hover: hex(0x4A2A2E),
        btn_trash_hover: hex(0xFE4450), btn_trash_pressed: hex(0xDE3440),
        bg_modal_section: hex(0x262335), border_subtle: hex(0x34294F), divider: hex(0x34294F),
    };

    // ── Modus Operandi (light, WCAG AAA) ──
    pub const MODUS_OPERANDI: Self = Self {
        bg_primary: hex(0xFFFFFF), bg_sidebar: hex(0xF0F0F0), bg_card: hex(0xE0E0E0),
        bg_card_hover: hex(0xD0D0D0), bg_card_pressed: hex(0xC4C4C4), bg_selected: hex(0xD0D0D0),
        bg_input: hex(0xE0E0E0), bg_progress: hex(0xF0F0F0),
        text_primary: hex(0x000000), text_secondary: hex(0x1A1A1A), text_muted: hex(0x333333),
        text_dim: hex(0x595959), text_dimmer: hex(0x7F7F7F), text_dimmest: hex(0x9F9F9F),
        text_placeholder: hex(0x7F7F7F),
        accent_blue: hex(0x0031A9), accent_icon: hex(0x005E8B), accent_progress: hex(0x0031A9),
        btn_default: hex(0xE0E0E0), btn_hover: hex(0xD0D0D0), btn_pressed: hex(0xC4C4C4),
        success: hex(0x006800), success_bg: hex(0xD0F0D0),
        btn_success: hex(0x006800), btn_success_hover: hex(0x005800), btn_success_pressed: hex(0x004800),
        warning: hex(0x6F5500), warning_bg: hex(0xF0E8C8),
        error: hex(0xA60000), error_light: hex(0xD00000), error_bg: hex(0xF0C8C8),
        btn_danger_bg: hex(0xF0C8C8), btn_danger_hover: hex(0xE0B0B0),
        btn_trash_hover: hex(0xA60000), btn_trash_pressed: hex(0x860000),
        bg_modal_section: hex(0xFFFFFF), border_subtle: hex(0xD0D0D0), divider: hex(0xD0D0D0),
    };

    // ── Modus Vivendi (dark, WCAG AAA) ──
    pub const MODUS_VIVENDI: Self = Self {
        bg_primary: hex(0x000000), bg_sidebar: hex(0x1E1E1E), bg_card: hex(0x303030),
        bg_card_hover: hex(0x404040), bg_card_pressed: hex(0x535353), bg_selected: hex(0x404040),
        bg_input: hex(0x303030), bg_progress: hex(0x1E1E1E),
        text_primary: hex(0xFFFFFF), text_secondary: hex(0xE0E0E0), text_muted: hex(0xC0C0C0),
        text_dim: hex(0x989898), text_dimmer: hex(0x707070), text_dimmest: hex(0x535353),
        text_placeholder: hex(0x707070),
        accent_blue: hex(0x2FAFFF), accent_icon: hex(0x00D3D0), accent_progress: hex(0x2FAFFF),
        btn_default: hex(0x303030), btn_hover: hex(0x404040), btn_pressed: hex(0x535353),
        success: hex(0x44BC44), success_bg: hex(0x0A1A0A),
        btn_success: hex(0x38A038), btn_success_hover: hex(0x30942E), btn_success_pressed: hex(0x288824),
        warning: hex(0xD0BC00), warning_bg: hex(0x1A1A00),
        error: hex(0xFF5F59), error_light: hex(0xFF7F86), error_bg: hex(0x1A0808),
        btn_danger_bg: hex(0x1A0808), btn_danger_hover: hex(0x3A1818),
        btn_trash_hover: hex(0xFF5F59), btn_trash_pressed: hex(0xDF4F48),
        bg_modal_section: hex(0x000000), border_subtle: hex(0x303030), divider: hex(0x303030),
    };
}
