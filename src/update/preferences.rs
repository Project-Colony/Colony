//! Settings-panel state and every user preference toggle.
//!
//! These arms share one shape: mutate a field, persist, and occasionally push a
//! side effect into the theme engine or i18n. They are kept as explicit methods
//! rather than a data-driven table so each preference stays greppable by name.

use iced::Task;

use crate::i18n;
use crate::message::Message;
use crate::state::{App, NotificationLevel};
use crate::ui::theme::{
    accent_key_to_color, set_active_accent, set_active_theme, set_high_contrast,
};

impl App {
    pub(super) fn toggle_settings(&mut self) -> Task<Message> {
        self.show_settings = !self.show_settings;
        if !self.show_settings {
            self.settings_category = 0;
        }
        Task::none()
    }

    pub(super) fn select_settings_category(&mut self, idx: usize) -> Task<Message> {
        self.settings_category = idx;
        Task::none()
    }

    pub(super) fn toggle_settings_section(&mut self, key: String) -> Task<Message> {
        if !self.settings_expanded_sections.remove(&key) {
            self.settings_expanded_sections.insert(key);
        }
        Task::none()
    }

    pub(super) fn select_theme_variant(&mut self, theme: String, variant: String) -> Task<Message> {
        self.selected_theme = theme;
        self.selected_variant = variant;
        set_active_theme(&self.selected_theme, &self.selected_variant);
        self.save_preferences();
        self.push_notification(i18n::t("theme_applied"), NotificationLevel::Info)
    }

    pub(super) fn select_accent_color(&mut self, color: String) -> Task<Message> {
        set_active_accent(accent_key_to_color(&color));
        self.selected_accent = color;
        self.auto_accent = false;
        self.save_preferences();
        Task::none()
    }

    pub(super) fn toggle_auto_accent(&mut self) -> Task<Message> {
        self.auto_accent = !self.auto_accent;
        if self.auto_accent {
            set_active_accent(None);
        } else {
            set_active_accent(accent_key_to_color(&self.selected_accent));
        }
        self.save_preferences();
        Task::none()
    }

    pub(super) fn toggle_restore_session(&mut self) -> Task<Message> {
        self.restore_session = !self.restore_session;
        self.save_preferences();
        Task::none()
    }

    pub(super) fn pick_default_view(&mut self, view: String) -> Task<Message> {
        self.default_view = view;
        self.save_preferences();
        Task::none()
    }

    pub(super) fn pick_language(&mut self, language: String) -> Task<Message> {
        self.language = language;
        self.save_preferences();
        // Live swap: every view calls t() per render, so the whole UI
        // re-labels on the next frame - the restart notice is history.
        i18n::set_language(&self.language);
        self.status_message = i18n::t("language_changed");
        Task::none()
    }

    pub(super) fn toggle_auto_check_updates(&mut self) -> Task<Message> {
        self.auto_check_updates = !self.auto_check_updates;
        self.save_preferences();
        Task::none()
    }

    pub(super) fn pick_font_size(&mut self, size: String) -> Task<Message> {
        self.font_size = size;
        self.save_preferences();
        Task::none()
    }

    pub(super) fn toggle_animations(&mut self) -> Task<Message> {
        self.animations = !self.animations;
        self.save_preferences();
        Task::none()
    }

    pub(super) fn toggle_high_contrast(&mut self) -> Task<Message> {
        self.high_contrast = !self.high_contrast;
        set_high_contrast(self.high_contrast);
        self.save_preferences();
        Task::none()
    }

    pub(super) fn pick_text_size_a11y(&mut self, size: String) -> Task<Message> {
        self.text_size_a11y = size;
        self.save_preferences();
        Task::none()
    }

    pub(super) fn toggle_reduce_motion(&mut self) -> Task<Message> {
        self.reduce_motion = !self.reduce_motion;
        self.save_preferences();
        Task::none()
    }

    pub(super) fn toggle_keyboard_nav(&mut self) -> Task<Message> {
        self.keyboard_nav = !self.keyboard_nav;
        self.save_preferences();
        Task::none()
    }

    pub(super) fn toggle_dyslexia_font(&mut self) -> Task<Message> {
        self.dyslexia_font = !self.dyslexia_font;
        self.save_preferences();
        Task::none()
    }

    pub(super) fn toggle_scan_on_startup(&mut self) -> Task<Message> {
        self.scan_on_startup = !self.scan_on_startup;
        self.save_preferences();
        Task::none()
    }
}
