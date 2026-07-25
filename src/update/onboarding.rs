//! First-launch welcome flow and the coachmark tutorial.

use iced::Task;

use crate::message::Message;
use crate::state::App;
use crate::ui::TutorialBounds;

impl App {
    pub(super) fn dismiss_first_launch(&mut self) -> Task<Message> {
        self.show_first_launch = false;
        self.welcome_step = 0;
        self.save_preferences();
        Task::none()
    }

    pub(super) fn welcome_next(&mut self) -> Task<Message> {
        const LAST_STEP: u8 = crate::ui::TUTORIAL_LAST_STEP;
        if self.welcome_step >= LAST_STEP {
            self.show_first_launch = false;
            self.welcome_step = 0;
            self.save_preferences();
            Task::none()
        } else {
            self.welcome_step += 1;
            crate::ui::fetch_bounds_task()
        }
    }

    pub(super) fn welcome_back(&mut self) -> Task<Message> {
        self.welcome_step = self.welcome_step.saturating_sub(1);
        crate::ui::fetch_bounds_task()
    }

    pub(super) fn tutorial_bounds_updated(&mut self, bounds: TutorialBounds) -> Task<Message> {
        self.tutorial_bounds = bounds;
        Task::none()
    }

    pub(super) fn welcome_connect_github(&mut self) -> Task<Message> {
        // Close the welcome overlay and jump straight to the GitHub panel so the
        // user can start the device-flow login without an extra "dismiss then
        // navigate" step.
        self.show_first_launch = false;
        self.welcome_step = 0;
        self.show_github_menu = true;
        self.save_preferences();
        Task::none()
    }
}
