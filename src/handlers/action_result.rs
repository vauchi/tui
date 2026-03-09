// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Maps AppEngine `ActionResult` variants to TUI state changes.

use vauchi_core::ui::ActionResult;

use crate::app::{App, Screen};

/// Applies an `ActionResult` from AppEngine to TUI state.
pub fn handle_action_result(app: &mut App, result: ActionResult) {
    match result {
        ActionResult::UpdateScreen(_) => {
            // Screen model is re-fetched on next draw via app_engine.current_screen()
        }
        ActionResult::NavigateTo(_) => {
            // AppEngine already updated its internal screen; TUI re-renders on next draw.
            // Sync TUI screen from AppEngine's current screen.
            if let Some(engine) = &app.app_engine {
                match engine.current_app_screen() {
                    vauchi_core::ui::AppScreen::Home => app.screen = Screen::Home,
                    vauchi_core::ui::AppScreen::Contacts => app.screen = Screen::Contacts,
                    vauchi_core::ui::AppScreen::Exchange => app.screen = Screen::Exchange,
                    vauchi_core::ui::AppScreen::Settings => app.screen = Screen::Settings,
                    vauchi_core::ui::AppScreen::Help => app.screen = Screen::Help,
                    vauchi_core::ui::AppScreen::Onboarding => {
                        app.screen = Screen::SetupWelcome;
                    }
                    _ => {}
                }
            }
        }
        ActionResult::OpenContact { contact_id: _ } => {
            app.screen = Screen::ContactDetail;
        }
        ActionResult::OpenUrl { url } => {
            let _ = open::that(&url);
        }
        ActionResult::ShowAlert { title: _, message } => {
            app.set_status(message);
        }
        ActionResult::ValidationError {
            component_id,
            message,
        } => {
            app.render_state.set_validation_error(component_id, message);
        }
        ActionResult::Complete => {
            // AppEngine handles completion routing internally
        }
        ActionResult::StartDeviceLink => {
            app.screen = Screen::Devices;
        }
        ActionResult::StartBackupImport => {
            app.screen = Screen::Backup;
        }
        ActionResult::RequestCamera => {
            // TUI can't open camera — show status message
            app.set_status("Camera not supported in terminal mode");
        }
        ActionResult::WipeComplete => {
            app.screen = Screen::SetupWelcome;
            app.onboarding_engine = Some(vauchi_core::ui::OnboardingEngine::new());
            app.render_state = Default::default();
        }
    }
}
