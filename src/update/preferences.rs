//! Preferences-page state and every user preference.
//!
//! The Appearance and Accessibility preferences are colony-ui's
//! [`StandardPreferences`](colony_ui::preferences::StandardPreferences): one
//! [`Change`] message records the change and applies it. Colony's own
//! preferences keep one explicit method each, so each stays greppable by name.

use colony_ui::preferences::Change;
use iced::Task;

use crate::i18n;
use crate::message::Message;
use crate::state::{App, NotificationLevel};

impl App {
    /// Open or close the page. The Colony name and the page's Close button
    /// both send this.
    pub(super) fn toggle_settings(&mut self) -> Task<Message> {
        if self.show_settings {
            self.close_settings();
        } else {
            self.show_settings = true;
        }
        Task::none()
    }

    /// Close the page the way every path out should: back on the first
    /// category, every section folded for the next visit.
    pub(super) fn close_settings(&mut self) {
        self.show_settings = false;
        self.settings_category = 0;
        self.settings_expanded_sections.clear();
    }

    pub(super) fn select_settings_category(&mut self, idx: usize) -> Task<Message> {
        self.settings_category = idx.min(crate::ui::SETTINGS_CATEGORIES.len() - 1);
        Task::none()
    }

    pub(super) fn toggle_settings_section(&mut self, key: &'static str) -> Task<Message> {
        if !self.settings_expanded_sections.remove(key) {
            self.settings_expanded_sections.insert(key.to_string());
        }
        Task::none()
    }

    /// A change from the shared Appearance or Accessibility widgets. It is in
    /// effect the moment it is made: there is no Save button.
    pub(super) fn change_preference(&mut self, change: Change) -> Task<Message> {
        self.standard.update(change);
        self.save_preferences();
        if matches!(change, Change::Theme { .. }) {
            self.push_notification(i18n::t("theme_applied"), NotificationLevel::Info)
        } else {
            Task::none()
        }
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

    pub(super) fn toggle_keyboard_nav(&mut self) -> Task<Message> {
        self.keyboard_nav = !self.keyboard_nav;
        self.save_preferences();
        Task::none()
    }

    pub(super) fn toggle_scan_on_startup(&mut self) -> Task<Message> {
        self.scan_on_startup = !self.scan_on_startup;
        self.save_preferences();
        Task::none()
    }
}
