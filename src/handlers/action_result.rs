// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Maps AppEngine `ActionResult` variants to TUI state changes.

use vauchi_core::ui::{ActionResult, Component, LockScreenEngine, WorkflowEngine};

use crate::app::{App, Screen};

/// Applies an `ActionResult` from AppEngine to TUI state.
pub fn handle_action_result(app: &mut App, result: ActionResult) {
    match result {
        ActionResult::UpdateScreen(_) => {
            // Screen model is re-fetched on next draw via app_engine.current_screen()

            // On form screens, auto-advance focus to the first TextInput when the
            // currently focused component is not a TextInput (e.g., after selecting
            // a type in the ToggleList, the engine adds TextInput fields below it).
            if matches!(
                app.screen,
                Screen::AddField | Screen::EditField | Screen::EditName | Screen::EditRelayUrl
            ) {
                let screen_model = app.app_engine.current_screen();
                let focused = screen_model
                    .components
                    .get(app.render_state.focused_component);
                let focused_is_text_input = matches!(focused, Some(Component::TextInput { .. }));
                if !focused_is_text_input {
                    if let Some(idx) = screen_model
                        .components
                        .iter()
                        .position(|c| matches!(c, Component::TextInput { .. }))
                    {
                        app.render_state.focused_component = idx;
                    }
                }
            }
        }
        ActionResult::NavigateTo(_) => {
            // AppEngine already updated its internal screen; TUI re-renders on next draw.
            // Sync TUI screen from AppEngine's current screen.
            // Reset render state for the new screen (fresh focus/selection).

            // Show success feedback when navigating back from form dialogs
            let from_screen = app.screen;
            {
                app.render_state = Default::default();
                match app.app_engine.current_app_screen() {
                    vauchi_core::ui::AppScreen::MyInfo => app.screen = Screen::MyInfo,
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
                    vauchi_core::ui::AppScreen::Lock => {
                        if app.lock_engine.is_none() {
                            app.lock_engine = Some(LockScreenEngine::new(5));
                        }
                        app.screen = Screen::Lock;
                    }
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
                    vauchi_core::ui::AppScreen::FormDialog { ref dialog_type } => {
                        // Map form dialog types to the corresponding TUI screens,
                        // syncing legacy TUI state so to_app_screen() stays consistent.
                        use crate::app::{EditFieldState, EditNameState, EditRelayUrlState};
                        use vauchi_core::ui::FormDialogType;
                        match dialog_type {
                            FormDialogType::AddField { .. } => {
                                app.screen = Screen::AddField;
                            }
                            FormDialogType::EditField {
                                field_id,
                                field_label,
                                current_value,
                            } => {
                                app.edit_field_state = EditFieldState {
                                    field_id: field_id.clone(),
                                    field_label: field_label.clone(),
                                    new_value: current_value.clone(),
                                    ..Default::default()
                                };
                                app.screen = Screen::EditField;
                            }
                            FormDialogType::EditName { current_name } => {
                                app.edit_name_state = EditNameState {
                                    new_name: current_name.clone(),
                                };
                                app.screen = Screen::EditName;
                            }
                            FormDialogType::EditRelayUrl { current_url } => {
                                app.edit_relay_url_state = EditRelayUrlState {
                                    new_url: current_url.clone(),
                                };
                                app.screen = Screen::EditRelayUrl;
                            }
                        }
                    }
                    vauchi_core::ui::AppScreen::ContactEdit { contact_id } => {
                        app.selected_contact_id = Some(contact_id.clone());
                        app.screen = Screen::ContactEdit;
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
                    vauchi_core::ui::AppScreen::MyInfoEntryDetail { .. } => {
                        app.screen = Screen::MyInfoEntryDetail;
                    }
                }
            }
            // Show success feedback when completing a form dialog
            match from_screen {
                Screen::AddField => app.set_status("Entry added"),
                Screen::EditField => app.set_status("Entry updated"),
                Screen::EditName => app.set_status("Name updated"),
                Screen::EditRelayUrl => app.set_status("Relay URL updated"),
                _ => {}
            }
        }
        ActionResult::OpenContact { contact_id } => {
            app.selected_contact_id = Some(contact_id);
            app.screen = Screen::ContactDetail;
        }
        ActionResult::EditContact { contact_id } => {
            app.selected_contact_id = Some(contact_id);
            app.screen = Screen::ContactEdit;
            app.render_state = Default::default();
        }
        ActionResult::OpenUrl { url } => {
            // Skip browser open during tests (prevents unwanted tabs)
            if std::env::var("VAUCHI_NO_BROWSER").is_ok() {
                app.set_status(format!("URL: {url}"));
            } else {
                match std::process::Command::new("xdg-open")
                    .arg(&url)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                {
                    Ok(_) => app.set_status(format!("Opened: {url}")),
                    Err(_) => app.set_status(format!("URL: {url}")),
                }
            }
        }
        ActionResult::ShowAlert { title, message } => {
            app.alert_message = Some((title, message));
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
        ActionResult::OpenEntryDetail { .. } => {
            // Handled by AppEngine (intercepted before reaching TUI)
        }
        ActionResult::ShowToast { message, .. } => {
            app.set_status(message);
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
