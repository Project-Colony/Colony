mod app_grid;
mod detail;
mod github_panel;
pub mod markdown_blocks;
mod settings;
mod sidebar;
pub mod theme;
pub(crate) mod tutorial;

pub(crate) use settings::SETTINGS_CATEGORIES;
pub(crate) use sidebar::sidebar_width;
pub(crate) use tutorial::{fetch_bounds_task, TutorialBounds, TUTORIAL_LAST_STEP};
