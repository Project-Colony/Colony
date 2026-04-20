use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::scan::{AppCategory, AppOrigin, Application};

#[derive(Debug, Clone)]
pub struct Section {
    pub name: String,
    pub icon: String,
    pub filter: SectionFilter,
    pub is_favorites: bool,
}

impl Section {
    pub fn category(&self) -> Option<&AppCategory> {
        self.filter.category.as_ref()
    }
}

#[derive(Debug, Clone)]
pub struct SectionFilter {
    origin: OriginFilter,
    category: Option<AppCategory>,
}

#[derive(Debug, Clone, Copy)]
enum OriginFilter {
    Any,
    WindowsOnly,
    NonWindows,
    ColonyOnly,
    ExternalOnly,
}

impl SectionFilter {
    pub fn matches(&self, app: &Application) -> bool {
        match self.origin {
            OriginFilter::Any => {}
            OriginFilter::WindowsOnly => {
                if app.origin != AppOrigin::Windows {
                    return false;
                }
            }
            OriginFilter::NonWindows => {
                if app.origin == AppOrigin::Windows {
                    return false;
                }
            }
            OriginFilter::ColonyOnly => {
                if app.origin != AppOrigin::Colony {
                    return false;
                }
            }
            OriginFilter::ExternalOnly => {
                if app.origin != AppOrigin::External && app.origin != AppOrigin::Linux {
                    return false;
                }
            }
        }

        if let Some(category) = &self.category {
            if &app.category != category {
                return false;
            }
        }

        true
    }
}

#[derive(Debug, Deserialize)]
struct SectionConfig {
    name: String,
    icon: String,
    origin: Option<String>,
    category: Option<String>,
}

impl SectionConfig {
    fn into_section(self) -> Section {
        let is_favorites = self.name.to_lowercase().contains("favor");
        Section {
            name: self.name,
            icon: self.icon,
            filter: SectionFilter {
                origin: parse_origin(self.origin.as_deref()),
                category: parse_category(self.category.as_deref()),
            },
            is_favorites,
        }
    }
}

pub fn load_sections() -> Vec<Section> {
    let path = Path::new("config/categories.json");
    match fs::read_to_string(path) {
        Ok(contents) => match serde_json::from_str::<Vec<SectionConfig>>(&contents) {
            Ok(configs) => {
                let sections: Vec<Section> = configs.into_iter().map(SectionConfig::into_section).collect();
                if sections.is_empty() {
                    tracing::warn!("Config loaded but no sections found, using defaults.");
                    default_sections()
                } else {
                    sections
                }
            }
            Err(error) => {
                tracing::warn!("Failed to parse {:?}: {error}", path);
                default_sections()
            }
        },
        Err(error) => {
            tracing::warn!("Failed to read {:?}: {error}", path);
            default_sections()
        }
    }
}

fn parse_origin(origin: Option<&str>) -> OriginFilter {
    match origin.map(|value| value.trim().to_lowercase()) {
        Some(value) if value == "windows" || value == "windows_only" => OriginFilter::WindowsOnly,
        Some(value) if value == "non_windows" || value == "nonwindows" => {
            OriginFilter::NonWindows
        }
        Some(value) if value == "linux" || value == "linux_only" => OriginFilter::ExternalOnly,
        Some(value) if value == "colony" => OriginFilter::ColonyOnly,
        Some(value) if value == "external" => OriginFilter::ExternalOnly,
        Some(value) if value == "any" || value == "all" => OriginFilter::Any,
        Some(value) => {
            tracing::warn!("Unknown origin filter '{value}', defaulting to 'any'.");
            OriginFilter::Any
        }
        None => OriginFilter::Any,
    }
}

fn parse_category(category: Option<&str>) -> Option<AppCategory> {
    match category.map(|value| value.trim().to_lowercase()) {
        None => None,
        Some(value) if value == "all" || value == "any" => None,
        Some(value) => match value.as_str() {
            "development" => Some(AppCategory::Development),
            "graphics" => Some(AppCategory::Graphics),
            "network" => Some(AppCategory::Network),
            "office" => Some(AppCategory::Office),
            "multimedia" => Some(AppCategory::Multimedia),
            "system" => Some(AppCategory::System),
            "utility" | "utilities" => Some(AppCategory::Utility),
            "game" | "games" => Some(AppCategory::Game),
            "other" => Some(AppCategory::Other),
            _ => {
                tracing::warn!("Unknown category '{value}', ignoring.");
                None
            }
        },
    }
}

fn default_sections() -> Vec<Section> {
    vec![
        Section {
            name: "All".to_string(),
            icon: "\u{f00a}".to_string(),
            filter: SectionFilter {
                origin: OriginFilter::ColonyOnly,
                category: None,
            },
            is_favorites: false,
        },
        Section {
            name: "Favorites".to_string(),
            icon: "\u{f005}".to_string(),
            filter: SectionFilter {
                origin: OriginFilter::Any,
                category: None,
            },
            is_favorites: true,
        },
        Section {
            name: "Windows".to_string(),
            icon: "\u{f17a}".to_string(),
            filter: SectionFilter {
                origin: OriginFilter::WindowsOnly,
                category: None,
            },
            is_favorites: false,
        },
        Section {
            name: "Linux".to_string(),
            icon: "\u{f17c}".to_string(),
            filter: SectionFilter {
                origin: OriginFilter::ExternalOnly,
                category: None,
            },
            is_favorites: false,
        },
        Section {
            name: "Development".to_string(),
            icon: "\u{f121}".to_string(),
            filter: SectionFilter {
                origin: OriginFilter::ColonyOnly,
                category: Some(AppCategory::Development),
            },
            is_favorites: false,
        },
        Section {
            name: "Graphics".to_string(),
            icon: "\u{f1fc}".to_string(),
            filter: SectionFilter {
                origin: OriginFilter::ColonyOnly,
                category: Some(AppCategory::Graphics),
            },
            is_favorites: false,
        },
        Section {
            name: "Network".to_string(),
            icon: "\u{f0ac}".to_string(),
            filter: SectionFilter {
                origin: OriginFilter::ColonyOnly,
                category: Some(AppCategory::Network),
            },
            is_favorites: false,
        },
        Section {
            name: "Office".to_string(),
            icon: "\u{f0f6}".to_string(),
            filter: SectionFilter {
                origin: OriginFilter::ColonyOnly,
                category: Some(AppCategory::Office),
            },
            is_favorites: false,
        },
        Section {
            name: "Multimedia".to_string(),
            icon: "\u{f008}".to_string(),
            filter: SectionFilter {
                origin: OriginFilter::ColonyOnly,
                category: Some(AppCategory::Multimedia),
            },
            is_favorites: false,
        },
        Section {
            name: "System".to_string(),
            icon: "\u{f085}".to_string(),
            filter: SectionFilter {
                origin: OriginFilter::ColonyOnly,
                category: Some(AppCategory::System),
            },
            is_favorites: false,
        },
        Section {
            name: "Utilities".to_string(),
            icon: "\u{f0ad}".to_string(),
            filter: SectionFilter {
                origin: OriginFilter::ColonyOnly,
                category: Some(AppCategory::Utility),
            },
            is_favorites: false,
        },
        Section {
            name: "Games".to_string(),
            icon: "\u{f11b}".to_string(),
            filter: SectionFilter {
                origin: OriginFilter::ColonyOnly,
                category: Some(AppCategory::Game),
            },
            is_favorites: false,
        },
        Section {
            name: "Other".to_string(),
            icon: "\u{f128}".to_string(),
            filter: SectionFilter {
                origin: OriginFilter::ColonyOnly,
                category: Some(AppCategory::Other),
            },
            is_favorites: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_category_known() {
        assert!(matches!(parse_category(Some("development")), Some(AppCategory::Development)));
        assert!(matches!(parse_category(Some("graphics")), Some(AppCategory::Graphics)));
        assert!(matches!(parse_category(Some("network")), Some(AppCategory::Network)));
        assert!(matches!(parse_category(Some("office")), Some(AppCategory::Office)));
        assert!(matches!(parse_category(Some("multimedia")), Some(AppCategory::Multimedia)));
        assert!(matches!(parse_category(Some("system")), Some(AppCategory::System)));
        assert!(matches!(parse_category(Some("utility")), Some(AppCategory::Utility)));
        assert!(matches!(parse_category(Some("utilities")), Some(AppCategory::Utility)));
        assert!(matches!(parse_category(Some("game")), Some(AppCategory::Game)));
        assert!(matches!(parse_category(Some("games")), Some(AppCategory::Game)));
        assert!(matches!(parse_category(Some("other")), Some(AppCategory::Other)));
    }

    #[test]
    fn parse_category_all_returns_none() {
        assert!(parse_category(Some("all")).is_none());
        assert!(parse_category(Some("any")).is_none());
        assert!(parse_category(None).is_none());
    }

    #[test]
    fn parse_category_unknown_returns_none() {
        assert!(parse_category(Some("foobar")).is_none());
    }

    #[test]
    fn parse_origin_known() {
        assert!(matches!(parse_origin(Some("windows")), OriginFilter::WindowsOnly));
        assert!(matches!(parse_origin(Some("colony")), OriginFilter::ColonyOnly));
        assert!(matches!(parse_origin(Some("any")), OriginFilter::Any));
        assert!(matches!(parse_origin(Some("all")), OriginFilter::Any));
        assert!(matches!(parse_origin(None), OriginFilter::Any));
    }

    #[test]
    fn parse_origin_unknown_defaults_to_any() {
        assert!(matches!(parse_origin(Some("martian")), OriginFilter::Any));
    }

    #[test]
    fn section_filter_matches_any() {
        let filter = SectionFilter {
            origin: OriginFilter::Any,
            category: None,
        };
        let app = Application {
            name: "Test".into(),
            exec: "test".into(),
            icon: None,
            category: AppCategory::Development,
            origin: AppOrigin::Windows,
        };
        assert!(filter.matches(&app));
    }

    #[test]
    fn section_filter_rejects_wrong_origin() {
        let filter = SectionFilter {
            origin: OriginFilter::WindowsOnly,
            category: None,
        };
        let app = Application {
            name: "Test".into(),
            exec: "test".into(),
            icon: None,
            category: AppCategory::Development,
            origin: AppOrigin::Colony,
        };
        assert!(!filter.matches(&app));
    }

    #[test]
    fn section_filter_rejects_wrong_category() {
        let filter = SectionFilter {
            origin: OriginFilter::Any,
            category: Some(AppCategory::Graphics),
        };
        let app = Application {
            name: "Test".into(),
            exec: "test".into(),
            icon: None,
            category: AppCategory::Development,
            origin: AppOrigin::Colony,
        };
        assert!(!filter.matches(&app));
    }

    #[test]
    fn default_sections_not_empty() {
        let sections = default_sections();
        assert!(!sections.is_empty());
        assert_eq!(sections[0].name, "All");
        assert_eq!(sections[1].name, "Favorites");
        assert!(sections[1].is_favorites);
    }

    #[test]
    fn section_config_into_section() {
        let config = SectionConfig {
            name: "Test".to_string(),
            icon: "\u{f00a}".to_string(),
            origin: Some("colony".to_string()),
            category: Some("development".to_string()),
        };
        let section = config.into_section();
        assert_eq!(section.name, "Test");
        assert!(matches!(section.filter.origin, OriginFilter::ColonyOnly));
        assert!(matches!(section.category(), Some(AppCategory::Development)));
    }
}
