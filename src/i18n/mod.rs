mod en;
mod fr;

use std::collections::HashMap;
use std::sync::RwLock;

static LOCALE: RwLock<Option<Locale>> = RwLock::new(None);

pub struct Locale {
    strings: HashMap<String, String>,
    lang: String,
}

impl Locale {
    fn new(lang: &str) -> Self {
        let mut strings = HashMap::new();

        match lang {
            "fr" => fr::insert_all(&mut strings),
            _ => en::insert_all(&mut strings),
        }

        Self {
            strings,
            lang: lang.to_string(),
        }
    }
}

/// Initialize the locale system. Call once at startup.
///
/// `preferred` is the user's saved language preference ("fr"/"en"); when it is
/// a recognized language it wins over environment detection, so the in-app
/// language picker actually takes effect on the next launch. Falls back to
/// `detect_language()` (LC_ALL / LC_MESSAGES / LANG) when unset or unknown.
pub fn init(preferred: Option<String>) {
    let lang = preferred
        .filter(|l| l == "fr" || l == "en")
        .unwrap_or_else(detect_language);
    set_language(&lang);
}

/// Swap the active locale at runtime. Views call `t()` on every render, so
/// the whole UI re-labels on the next frame - no restart required (the locale
/// used to live in a OnceLock, forcing one).
pub fn set_language(lang: &str) {
    // Keep the shared label table on the same locale, or the theme picker would
    // stay English while the rest of the page switched.
    colony_ui::i18n::set_locale(colony_ui::i18n::Locale::from_tag(lang));
    let lang = if lang == "fr" || lang == "en" {
        lang
    } else {
        "en"
    };
    tracing::info!("Locale: {lang}");
    if let Ok(mut locale) = LOCALE.write() {
        *locale = Some(Locale::new(lang));
    }
}

/// Localized display name for a built-in sidebar section, keyed by its
/// canonical English name. Custom/user sections (no matching key) fall back to
/// their raw name so nothing is lost.
pub fn section_display_name(name: &str) -> String {
    let key = match name.to_lowercase().as_str() {
        "all" => "section_all",
        "favorites" | "favoris" => "section_favorites",
        "windows" => "section_windows",
        "linux" => "section_linux",
        "macos" => "section_macos",
        "development" => "section_development",
        "graphics" => "section_graphics",
        "network" => "section_network",
        "office" => "section_office",
        "multimedia" => "section_multimedia",
        "system" => "section_system",
        "utilities" | "utility" => "section_utilities",
        "security" => "section_security",
        "games" | "game" => "section_games",
        "other" => "section_other",
        _ => return name.to_string(),
    };
    LOCALE
        .read()
        .ok()
        .and_then(|l| l.as_ref().and_then(|l| l.strings.get(key).cloned()))
        .unwrap_or_else(|| name.to_string())
}

/// Get a translated string by key.
pub fn t(key: &str) -> String {
    if let Some(s) = LOCALE
        .read()
        .ok()
        .and_then(|l| l.as_ref().and_then(|l| l.strings.get(key).cloned()))
    {
        return s;
    }
    // Theme and accent labels are not ours: they name shared design objects and
    // ship with colony-ui, generated from the same tokens as the palettes. They
    // used to be copied into en.rs and fr.rs, where they drifted from the
    // catalog they describe.
    let shared = colony_ui::i18n::t(key);
    if shared != key {
        return shared.to_string();
    }
    tracing::warn!("Missing translation key: {key}");
    key.to_string()
}

/// Get a translated string with variable substitution.
/// Variables use `{name}` syntax.
pub fn t_fmt(key: &str, vars: &[(&str, &str)]) -> String {
    let mut result = t(key);
    for (name, value) in vars {
        result = result.replace(&format!("{{{name}}}"), value);
    }
    result
}

/// Get the current language code.
pub fn current_lang() -> String {
    LOCALE
        .read()
        .ok()
        .and_then(|l| l.as_ref().map(|l| l.lang.clone()))
        .unwrap_or_else(|| "en".to_string())
}

/// Detect the user's language from environment.
fn detect_language() -> String {
    // Check LANG, LC_ALL, LC_MESSAGES
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            let lang = val.split('.').next().unwrap_or(&val);
            let lang = lang.split('_').next().unwrap_or(lang);
            if lang == "fr" {
                return "fr".to_string();
            }
        }
    }

    "en".to_string()
}

#[cfg(test)]
mod tests {
    /// The active locale is process-global, so tests that swap it must not run
    /// concurrently with each other — the failure is a value from whichever
    /// language the other test happened to set.
    static LOCALE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn with_locale<T>(f: impl FnOnce() -> T) -> T {
        let _guard = LOCALE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        f()
    }

    /// Theme and accent labels live in colony-ui, not in en.rs / fr.rs. They
    /// must still resolve through `t()`, and must follow the active locale —
    /// otherwise the theme picker would sit in English inside a French page.
    #[test]
    fn shared_labels_resolve_through_colony_ui_and_follow_the_locale() {
        with_locale(|| {
            super::set_language("fr");
            assert_eq!(super::t("settings_theme_dark_mode"), "Mode sombre");
            assert_eq!(super::t("settings_accent_violet"), "Violet");

            super::set_language("en");
            assert_eq!(super::t("settings_theme_dark_mode"), "Dark mode");

            // Proper nouns read the same either way.
            assert_eq!(super::t("settings_theme_stellar_blade_eve"), "EVE");

            // And a key belonging to neither table still renders as itself.
            assert_eq!(super::t("no_such_key_anywhere"), "no_such_key_anywhere");
        });
    }

    use super::*;

    #[test]
    fn english_locale_has_keys() {
        let locale = Locale::new("en");
        assert!(locale.strings.contains_key("categories"));
        assert!(locale.strings.contains_key("github_login"));
        assert!(locale.strings.contains_key("back"));
        assert!(locale.strings.contains_key("error_thread_panic"));
        assert!(locale.strings.contains_key("confirm_uninstall"));
        assert!(locale.strings.contains_key("welcome_title"));
        assert!(locale.strings.contains_key("download_cancelled"));
        assert!(locale.strings.contains_key("add_favorite"));
    }

    #[test]
    fn french_locale_has_keys() {
        let locale = Locale::new("fr");
        assert_eq!(locale.strings.get("categories").unwrap(), "Catégories");
        assert_eq!(locale.strings.get("back").unwrap(), "Retour");
        assert!(locale.strings.contains_key("error_thread_panic"));
        assert!(locale.strings.contains_key("confirm_uninstall"));
        assert!(locale.strings.contains_key("welcome_title"));
    }

    #[test]
    fn unknown_lang_defaults_to_english() {
        let locale = Locale::new("xx");
        assert_eq!(locale.strings.get("categories").unwrap(), "Categories");
    }

    #[test]
    fn fr_and_en_have_identical_key_sets() {
        let fr = Locale::new("fr");
        let en = Locale::new("en");
        let fr_keys: std::collections::BTreeSet<&String> = fr.strings.keys().collect();
        let en_keys: std::collections::BTreeSet<&String> = en.strings.keys().collect();
        let only_fr: Vec<&&String> = fr_keys.difference(&en_keys).collect();
        let only_en: Vec<&&String> = en_keys.difference(&fr_keys).collect();
        assert!(
            only_fr.is_empty() && only_en.is_empty(),
            "Locale key mismatch — only in fr: {only_fr:?}; only in en: {only_en:?}"
        );
    }

    /// 52 keys (104 entries across the two locales) were dead: settings that
    /// were never built, a welcome carousel that was replaced, placeholders
    /// for features that never shipped. Translators had no way to tell them
    /// from live strings.
    ///
    /// Reads the source rather than the runtime map, which is the only way to
    /// see a key that nothing looks up.
    #[test]
    fn no_locale_key_is_unreferenced() {
        const EN: &str = include_str!("en.rs");
        // Every .rs in the crate EXCEPT the two locale tables themselves.
        const SOURCES: &[&str] = &[
            include_str!("mod.rs"),
            include_str!("../app.rs"),
            include_str!("../state.rs"),
            include_str!("../update/mod.rs"),
            include_str!("../update/store.rs"),
            include_str!("../update/github_auth.rs"),
            include_str!("../update/launcher.rs"),
            include_str!("../update/preferences.rs"),
            include_str!("../update/keyboard.rs"),
            include_str!("../update/onboarding.rs"),
            include_str!("../ui/app_grid.rs"),
            include_str!("../ui/detail.rs"),
            include_str!("../ui/sidebar.rs"),
            include_str!("../ui/settings.rs"),
            include_str!("../ui/github_panel.rs"),
            include_str!("../ui/tutorial.rs"),
            include_str!("../scan.rs"),
            include_str!("../sections.rs"),
            include_str!("../github/http.rs"),
            include_str!("../github/catalog.rs"),
            include_str!("../github/releases.rs"),
            include_str!("../oauth.rs"),
            include_str!("../download.rs"),
            include_str!("../persistence.rs"),
            include_str!("../config.rs"),
            include_str!("../main.rs"),
        ];

        let mut dead = Vec::new();
        for line in EN.lines() {
            // A KEY is `"snake_case".into(),` - the value on a wrapped insert
            // is also a quoted string on its own line, so match the shape, not
            // just "starts with a quote".
            let Some(rest) = line.trim().strip_prefix('"') else {
                continue;
            };
            let Some(key) = rest.split('"').next() else {
                continue;
            };
            if key.is_empty()
                || !key
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                || !rest[key.len()..].starts_with("\".into()")
            {
                continue;
            }
            let needle = format!("\"{key}\"");
            if !SOURCES.iter().any(|src| src.contains(&needle)) {
                dead.push(key.to_string());
            }
        }
        assert!(
            dead.is_empty(),
            "locale keys nothing looks up: {dead:?}\n\
             Either delete them from en.rs and fr.rs, or - if they ARE used - add the \
             module that uses them to SOURCES above (this list is hand-maintained, and a \
             missing entry fails LOUDLY here rather than silently letting dead keys back in)."
        );
    }

    #[test]
    fn t_fmt_substitution() {
        with_locale(|| {
            set_language("en");
            let result = t_fmt("apps_found", &[("count", "42")]);
            assert_eq!(result, "42 applications found");
        });
    }
}
