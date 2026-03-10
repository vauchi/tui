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
            // Reset render state for the new screen (fresh focus/selection).
            {
                app.render_state = Default::default();
                match app.app_engine.current_app_screen() {
                    vauchi_core::ui::AppScreen::Home => app.screen = Screen::Home,
                    vauchi_core::ui::AppScreen::Contacts => app.screen = Screen::Contacts,
                    vauchi_core::ui::AppScreen::Exchange => app.screen = Screen::Exchange,
                    vauchi_core::ui::AppScreen::Settings => app.screen = Screen::Settings,
                    vauchi_core::ui::AppScreen::Help => app.screen = Screen::Help,
                    vauchi_core::ui::AppScreen::Onboarding => {
                        app.screen = Screen::SetupWelcome;
                    }
                    vauchi_core::ui::AppScreen::ContactDetail { contact_id } => {
                        app.selected_contact_id = Some(contact_id.clone());
                        app.screen = Screen::ContactDetail;
                    }
                    vauchi_core::ui::AppScreen::Backup => app.screen = Screen::Backup,
                    vauchi_core::ui::AppScreen::Lock => app.screen = Screen::Lock,
                    vauchi_core::ui::AppScreen::DeviceLinking => app.screen = Screen::Devices,
                    vauchi_core::ui::AppScreen::DuressPin => app.screen = Screen::Duress,
                    vauchi_core::ui::AppScreen::EmergencyShred => app.screen = Screen::Emergency,
                    vauchi_core::ui::AppScreen::DeliveryStatus => app.screen = Screen::Delivery,
                    vauchi_core::ui::AppScreen::Sync => app.screen = Screen::Sync,
                    vauchi_core::ui::AppScreen::TorSettings => app.screen = Screen::TorSettings,
                    vauchi_core::ui::AppScreen::Recovery => app.screen = Screen::Recovery,
                    vauchi_core::ui::AppScreen::Groups => app.screen = Screen::Groups,
                    vauchi_core::ui::AppScreen::Privacy => app.screen = Screen::Privacy,
                    vauchi_core::ui::AppScreen::Support => app.screen = Screen::Support,
                    vauchi_core::ui::AppScreen::GroupDetail { group_id } => {
                        app.groups_state.selected_group_id = Some(group_id.clone());
                        app.screen = Screen::GroupDetail;
                    }
                    vauchi_core::ui::AppScreen::ContactVisibility { contact_id } => {
                        app.selected_contact_id = Some(contact_id.clone());
                        app.screen = Screen::ContactVisibility;
                    }
                    vauchi_core::ui::AppScreen::FormDialog { .. } => {
                        // Form dialogs stay on their current TUI screen (AddField, EditField, etc.)
                        // The engine handles state; TUI screen doesn't change.
                    }
                    // ContactEdit has no dedicated TUI screen yet — edits happen
                    // inline via EditField / EditName dialogs triggered by actions.
                    // Preserve contact_id so fallback to ContactDetail works.
                    vauchi_core::ui::AppScreen::ContactEdit { contact_id } => {
                        app.selected_contact_id = Some(contact_id.clone());
                    }
                    vauchi_core::ui::AppScreen::ContactDuplicates => {
                        app.screen = Screen::ContactDuplicates;
                    }
                    vauchi_core::ui::AppScreen::ContactMerge { .. } => {
                        app.screen = Screen::ContactMerge;
                    }
                    vauchi_core::ui::AppScreen::ContactLimit => {
                        app.screen = Screen::ContactLimit;
                    }
                }
            }
        }
        ActionResult::OpenContact { contact_id } => {
            app.selected_contact_id = Some(contact_id);
            app.screen = Screen::ContactDetail;
        }
        ActionResult::EditContact { contact_id } => {
            // ContactEdit has no dedicated TUI screen yet — route to ContactDetail
            // but preserve contact_id so the detail view shows the right contact.
            app.selected_contact_id = Some(contact_id);
            app.screen = Screen::ContactDetail;
        }
        ActionResult::OpenUrl { url } => {
            app.set_status(format!("URL: {url}"));
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
            app.invalidate_engines();
            // AppEngine's Vauchi data was wiped — navigate to Onboarding
            app.app_engine
                .navigate_to(vauchi_core::ui::AppScreen::Onboarding);
        }
    }
}
