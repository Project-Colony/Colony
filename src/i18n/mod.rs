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

        // The shared vocabulary FIRST: theme families, theme variants and accent
        // names, generated from the design tokens in Project-Colony-Resources.
        //
        // They used to be copied into `fr.rs` and `en.rs` by hand, which made
        // the picker's promise false: `THEME_FAMILIES` renders whatever the
        // catalog holds, so a family added upstream reached the screen with no
        // code change here - and then displayed its raw key, because no hand
        // written line existed to name it. Seeding from the crate closes that.
        //
        // Colony's own strings load second and would win a collision, but
        // `no_shared_key_is_redefined_locally` fails if one ever exists: an
        // override is how the drift this deletion removed would come back.
        let shared = match lang {
            "fr" => colony_ui::i18n::Locale::Fr,
            _ => colony_ui::i18n::Locale::En,
        };
        strings.extend(
            colony_ui::i18n::all(shared).map(|(k, v)| (k.to_string(), v.to_string())),
        );

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
    // The shared widgets call `colony_ui::i18n::t` directly - they cannot reach
    // Colony's table - so the crate's own active locale has to move in step or
    // the theme picker stays English while the rest of the page turns French.
    colony_ui::i18n::set_locale(colony_ui::i18n::Locale::from_tag(lang));
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

    /// The active locale is process-wide, so a test that SETS it and then reads
    /// it back cannot run beside another that sets it too - the second write
    /// lands between the first test's write and its assertion. Every test that
    /// calls `set_language` takes this first.
    ///
    /// The lock is poisoned by a failing test, which would cascade into a second
    /// misleading failure, so the guard is taken through `unwrap_or_else`.
    static LOCALE: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn locale_guard() -> std::sync::MutexGuard<'static, ()> {
        LOCALE.lock().unwrap_or_else(|e| e.into_inner())
    }

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

    /// Colony's own locale files must not carry a string colony-ui already
    /// ships. They did - all 62 of them, in both locales - and the copies had
    /// not drifted *yet*, which is the only reason the deletion was safe.
    ///
    /// `Locale::new` loads Colony's table second, so a re-added line would win
    /// silently and put the drift back. This is what makes that loud.
    #[test]
    fn no_shared_key_is_redefined_locally() {
        for (lang, insert_all) in [
            ("fr", fr::insert_all as fn(&mut HashMap<String, String>)),
            ("en", en::insert_all as fn(&mut HashMap<String, String>)),
        ] {
            let mut own = HashMap::new();
            insert_all(&mut own);

            let shared = colony_ui::i18n::Locale::from_tag(lang);
            let clashes: Vec<&str> = colony_ui::i18n::all(shared)
                .map(|(k, _)| k)
                .filter(|k| own.contains_key(*k))
                .collect();

            assert!(
                clashes.is_empty(),
                "{lang}.rs redefines strings colony-ui already ships: {clashes:?}. \
                 Delete them - the shared table is seeded first in Locale::new."
            );
        }
    }

    /// The point of the deletion: the names still reach the screen, localized,
    /// through exactly the same `t()` every call site already uses.
    #[test]
    fn shared_theme_and_accent_labels_still_resolve() {
        let fr = Locale::new("fr");
        let en = Locale::new("en");

        // Translated.
        assert_eq!(fr.strings.get("settings_accent_red").unwrap(), "Rouge");
        assert_eq!(en.strings.get("settings_accent_red").unwrap(), "Red");
        assert_eq!(fr.strings.get("settings_theme_light").unwrap(), "Mode clair");

        // A proper noun, identical in both - and never hand-typed again.
        assert_eq!(fr.strings.get("settings_theme_gruvbox").unwrap(), "Gruvbox");
        assert_eq!(en.strings.get("settings_theme_gruvbox").unwrap(), "Gruvbox");
    }

    /// A family added upstream must reach the picker NAMED. Before the seeding
    /// it reached it as its raw key, because naming it took a hand-written line
    /// in a file the upstream change never touched.
    #[test]
    fn every_catalog_entry_has_a_name_in_both_locales() {
        let fr = Locale::new("fr");
        let en = Locale::new("en");

        for family in colony_ui::THEME_FAMILIES {
            for key in std::iter::once(family.label_key)
                .chain(family.variants.iter().map(|v| v.label_key))
            {
                for (lang, locale) in [("fr", &fr), ("en", &en)] {
                    let name = locale.strings.get(key);
                    assert!(
                        name.is_some_and(|n| n != key),
                        "{lang}: theme catalog key {key} would render as itself"
                    );
                }
            }
        }
    }

    /// Same for the accents, whose order is load-bearing (each app's identity
    /// tint is a hash bucketed into this list).
    #[test]
    fn every_accent_has_a_name_in_both_locales() {
        let fr = Locale::new("fr");
        let en = Locale::new("en");
        for accent in colony_ui::ACCENT_OVERRIDES {
            for (lang, locale) in [("fr", &fr), ("en", &en)] {
                assert!(
                    locale.strings.contains_key(accent.label_key),
                    "{lang}: accent {} has no name", accent.key
                );
            }
        }
    }

    /// `set_language` has to move BOTH tables. The shared widgets never touch
    /// Colony's, so if this regresses the theme picker silently stays English.
    #[test]
    fn set_language_moves_the_shared_locale_too() {
        let _guard = locale_guard();

        set_language("fr");
        assert_eq!(colony_ui::i18n::locale(), colony_ui::i18n::Locale::Fr);
        assert_eq!(colony_ui::i18n::t("settings_accent_red"), "Rouge");

        set_language("en");
        assert_eq!(colony_ui::i18n::locale(), colony_ui::i18n::Locale::En);
        assert_eq!(colony_ui::i18n::t("settings_accent_red"), "Red");
    }

    #[test]
    fn t_fmt_substitution() {
        let _guard = locale_guard();

        // Initialize with English for test
        set_language("en");
        let result = t_fmt("apps_found", &[("count", "42")]);
        assert_eq!(result, "42 applications found");
    }
}
