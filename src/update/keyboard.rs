//! Keyboard navigation: the global key handling for the whole shell.
//!
//! Bindings are matched by physical key so the same layout works on AZERTY and
//! QWERTY without a second table.

use iced::keyboard;
use iced::Task;

use crate::message::Message;
use crate::state::App;

impl App {
    pub(super) fn keyboard_event(&mut self, event: keyboard::Event) -> Task<Message> {
        if !self.keyboard_nav {
            return Task::none();
        }
        if let iced::keyboard::Event::KeyPressed { key, modifiers, .. } = event {
            match key {
                iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) => {
                    if self.show_settings {
                        self.show_settings = false;
                    } else if self.confirm_uninstall.is_some() {
                        self.confirm_uninstall = None;
                    } else if self.show_first_launch {
                        self.show_first_launch = false;
                        self.save_preferences();
                    } else if self.active_colony_repo.is_some() {
                        self.active_colony_repo = None;
                    } else if self.show_github_menu {
                        self.show_github_menu = false;
                    }
                }
                iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab)
                    if !self.show_settings && !self.show_github_menu && !self.show_first_launch =>
                {
                    let len = self.sections.len();
                    if len > 0 {
                        self.sidebar_indicator_from = self.sidebar_indicator_pos();
                        if modifiers.shift() {
                            self.selected_section = if self.selected_section == 0 {
                                len - 1
                            } else {
                                self.selected_section - 1
                            };
                        } else {
                            self.selected_section = (self.selected_section + 1) % len;
                        }
                        self.sidebar_indicator_target = self.selected_section as f32 * 44.0;
                        self.sidebar_indicator_start = Some(std::time::Instant::now());
                        self.active_colony_repo = None;
                        self.save_preferences();
                    }
                }
                iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown)
                    if self.show_settings =>
                {
                    self.settings_category = (self.settings_category + 1).min(5);
                }
                iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp)
                    if self.show_settings =>
                {
                    self.settings_category = self.settings_category.saturating_sub(1);
                }
                // Grid traversal: Down/Up move a highlight over the
                // visible rows (store repos then local apps); Enter
                // activates it. Keys are stable names, not indexes,
                // so a catalog refresh cannot shift the highlight.
                iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown)
                    if !self.show_settings
                        && !self.show_github_menu
                        && !self.show_first_launch
                        && self.active_colony_repo.is_none() =>
                {
                    let keys = self.grid_keys();
                    if !keys.is_empty() {
                        let next = match &self.keyboard_cursor {
                            Some(cur) => keys
                                .iter()
                                .position(|k| k == cur)
                                .map(|i| (i + 1).min(keys.len() - 1))
                                .unwrap_or(0),
                            None => 0,
                        };
                        self.keyboard_cursor = Some(keys[next].clone());
                    }
                }
                iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp)
                    if !self.show_settings
                        && !self.show_github_menu
                        && !self.show_first_launch
                        && self.active_colony_repo.is_none() =>
                {
                    let keys = self.grid_keys();
                    if !keys.is_empty() {
                        let next = match &self.keyboard_cursor {
                            Some(cur) => keys
                                .iter()
                                .position(|k| k == cur)
                                .map(|i| i.saturating_sub(1))
                                .unwrap_or(0),
                            None => 0,
                        };
                        self.keyboard_cursor = Some(keys[next].clone());
                    }
                }
                iced::keyboard::Key::Named(iced::keyboard::key::Named::PageDown)
                    if self.show_settings =>
                {
                    self.settings_category = (self.settings_category + 3).min(5);
                }
                iced::keyboard::Key::Named(iced::keyboard::key::Named::PageUp)
                    if self.show_settings =>
                {
                    self.settings_category = self.settings_category.saturating_sub(3);
                }
                iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter)
                    if !self.show_settings
                        && !self.show_github_menu
                        && !self.show_first_launch
                        && self.active_colony_repo.is_none() =>
                {
                    // Activate the keyboard highlight when there is
                    // one, else fall back to the first store row.
                    let target = self.keyboard_cursor.clone().or_else(|| {
                        self.filtered_colony_repos()
                            .first()
                            .map(|r| format!("repo:{}", r.name))
                    });
                    match target {
                        Some(key) if key.starts_with("repo:") => {
                            self.active_colony_repo = Some(key["repo:".len()..].to_string());
                            // Refresh the (repo, tab) markdown cache —
                            // the detail view reads cached blocks only.
                            self.refresh_detail_markdown();
                        }
                        Some(key) if key.starts_with("app:") => {
                            let name = &key["app:".len()..];
                            if let Some(app) = self.applications.iter().find(|a| a.name == name) {
                                let exec = app.exec.clone();
                                return self.update(Message::LaunchApp(exec));
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        Task::none()
    }
}
