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
    LOCALE
        .read()
        .ok()
        .and_then(|l| l.as_ref().and_then(|l| l.strings.get(key).cloned()))
        .unwrap_or_else(|| {
            tracing::warn!("Missing translation key: {key}");
            key.to_string()
        })
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

    #[test]
    fn t_fmt_substitution() {
        // Initialize with English for test
        set_language("en");
        let result = t_fmt("apps_found", &[("count", "42")]);
        assert_eq!(result, "42 applications found");
    }
}
