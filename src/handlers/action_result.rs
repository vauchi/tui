// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Maps AppEngine `ActionResult` variants to TUI state changes.

use vauchi_app::ui::{ActionResult, Component, LockScreenEngine, WorkflowEngine};

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
                if !focused_is_text_input
                    && let Some(idx) = screen_model
                        .components
                        .iter()
                        .position(|c| matches!(c, Component::TextInput { .. }))
                {
                    app.render_state.focused_component = idx;
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
                    vauchi_app::ui::AppScreen::MyInfo => app.screen = Screen::MyInfo,
                    vauchi_app::ui::AppScreen::Contacts => app.screen = Screen::Contacts,
                    vauchi_app::ui::AppScreen::Exchange => app.screen = Screen::Exchange,
                    vauchi_app::ui::AppScreen::Settings => app.screen = Screen::Settings,
                    vauchi_app::ui::AppScreen::Help => app.screen = Screen::Help,
                    vauchi_app::ui::AppScreen::Onboarding => {
                        app.screen = Screen::SetupWelcome;
                    }
                    vauchi_app::ui::AppScreen::ContactDetail { contact_id } => {
                        app.selected_contact_id = Some(contact_id.clone());
                        app.screen = Screen::ContactDetail;
                    }
                    vauchi_app::ui::AppScreen::Backup => app.screen = Screen::Backup,
                    vauchi_app::ui::AppScreen::Lock => {
                        if app.lock_engine.is_none() {
                            app.lock_engine = Some(LockScreenEngine::new(
                                vauchi_app::ui::DEFAULT_LOCK_MAX_ATTEMPTS,
                            ));
                        }
                        app.screen = Screen::Lock;
                    }
                    vauchi_app::ui::AppScreen::DeviceLinking => app.screen = Screen::Devices,
                    vauchi_app::ui::AppScreen::DuressPin => app.screen = Screen::Duress,
                    vauchi_app::ui::AppScreen::EmergencyShred => app.screen = Screen::Emergency,
                    vauchi_app::ui::AppScreen::DeliveryStatus => app.screen = Screen::Delivery,
                    vauchi_app::ui::AppScreen::Sync => app.screen = Screen::Sync,
                    vauchi_app::ui::AppScreen::Recovery => app.screen = Screen::Recovery,
                    vauchi_app::ui::AppScreen::Groups => app.screen = Screen::Groups,
                    vauchi_app::ui::AppScreen::More => app.screen = Screen::More,
                    vauchi_app::ui::AppScreen::Privacy => app.screen = Screen::Privacy,
                    vauchi_app::ui::AppScreen::Support => app.screen = Screen::Support,
                    vauchi_app::ui::AppScreen::GroupDetail { group_id } => {
                        app.groups_state.selected_group_id = Some(group_id.clone());
                        app.screen = Screen::GroupDetail;
                    }
                    vauchi_app::ui::AppScreen::ContactVisibility { contact_id } => {
                        app.selected_contact_id = Some(contact_id.clone());
                        app.screen = Screen::ContactVisibility;
                    }
                    vauchi_app::ui::AppScreen::FormDialog { dialog_type } => {
                        // Map form dialog types to the corresponding TUI screens,
                        // syncing legacy TUI state so to_app_screen() stays consistent.
                        use crate::app::{EditFieldState, EditNameState, EditRelayUrlState};
                        use vauchi_app::ui::FormDialogType;
                        match dialog_type {
                            FormDialogType::AddField { .. } => {
                                app.screen = Screen::AddField;
                            }
                            FormDialogType::EditField {
                                field_id,
                                field_label,
                                current_value,
                                current_note: _,
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
                            _ => {
                                // Unknown FormDialogType — ignore
                            }
                        }
                    }
                    vauchi_app::ui::AppScreen::ContactEdit { contact_id } => {
                        app.selected_contact_id = Some(contact_id.clone());
                        app.screen = Screen::ContactEdit;
                    }
                    vauchi_app::ui::AppScreen::ContactDuplicates => {
                        app.screen = Screen::ContactDuplicates;
                    }
                    vauchi_app::ui::AppScreen::ContactMerge { .. } => {
                        app.screen = Screen::ContactMerge;
                    }
                    vauchi_app::ui::AppScreen::ContactLimit => {
                        app.screen = Screen::ContactLimit;
                    }
                    vauchi_app::ui::AppScreen::MyInfoEntryDetail { .. } => {
                        app.screen = Screen::MyInfoEntryDetail;
                    }
                    vauchi_app::ui::AppScreen::VerifyFingerprint { contact_id } => {
                        app.selected_contact_id = Some(contact_id.clone());
                        app.screen = Screen::VerifyFingerprint;
                    }
                    vauchi_app::ui::AppScreen::ActivityLog => {
                        app.screen = Screen::Activity;
                    }
                    vauchi_app::ui::AppScreen::DeviceReplacement => {
                        app.screen = Screen::DeviceReplacement;
                    }
                    _ => {
                        // Unknown AppScreen variant — stay on current screen
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
                let opener = if cfg!(target_os = "macos") {
                    "open"
                } else if cfg!(target_os = "windows") {
                    "start"
                } else {
                    "xdg-open"
                };
                match std::process::Command::new(opener)
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
        ActionResult::ShowToast {
            message,
            undo_action_id,
        } => {
            app.set_status_with_undo(message, undo_action_id);
        }
        ActionResult::ExchangeCommands { commands } => {
            handle_exchange_commands(app, commands);
        }
        ActionResult::WipeComplete => {
            app.screen = Screen::SetupWelcome;
            app.onboarding_engine = Some(vauchi_app::ui::OnboardingEngine::new());
            app.render_state = Default::default();
            app.invalidate_engines();
            // AppEngine's Vauchi data was wiped — navigate to Onboarding
            app.app_engine
                .navigate_to(vauchi_app::ui::AppScreen::Onboarding);
        }
        ActionResult::VerifyFingerprint { contact_id } => {
            app.selected_contact_id = Some(contact_id);
            app.screen = Screen::VerifyFingerprint;
            app.render_state = Default::default();
        }
        // trust-notes-preview (core!368) — not yet implemented in TUI
        ActionResult::PreviewAs { .. } | ActionResult::ShowContactPicker => {}
        _ => {
            // Unknown ActionResult variant — ignore
        }
    }
}

/// Handles ExchangeCommands from the ADR-031 command/event protocol.
///
/// TUI supports QR display (re-render) and QR scan (text paste prompt).
/// BLE, NFC, and audio are reported as unavailable back to core so the
/// session can fall back to QR.
fn handle_exchange_commands(app: &mut App, commands: Vec<vauchi_core::exchange::ExchangeCommand>) {
    use vauchi_core::exchange::{ExchangeCommand, ExchangeHardwareEvent};

    for cmd in commands {
        match cmd {
            ExchangeCommand::QrDisplay { .. } => {
                // QR data is embedded in the screen model — re-render picks it up
            }
            ExchangeCommand::QrRequestScan => {
                // TUI: prompt user to paste QR data (handled via text input mode)
                app.set_status("Paste the other person's QR code data and press Enter");
                app.exchange_scan_pending = true;
            }
            // Hardware not available in terminal
            ExchangeCommand::BleStartAdvertising { .. }
            | ExchangeCommand::BleStartScanning { .. }
            | ExchangeCommand::BleConnect { .. }
            | ExchangeCommand::BleWriteCharacteristic { .. }
            | ExchangeCommand::BleReadCharacteristic { .. }
            | ExchangeCommand::BleDisconnect => {
                let _ = app.app_engine.handle_hardware_event(
                    ExchangeHardwareEvent::HardwareUnavailable {
                        transport: "BLE".into(),
                    },
                );
            }
            ExchangeCommand::NfcActivate { .. } | ExchangeCommand::NfcDeactivate => {
                let _ = app.app_engine.handle_hardware_event(
                    ExchangeHardwareEvent::HardwareUnavailable {
                        transport: "NFC".into(),
                    },
                );
            }
            ExchangeCommand::AudioEmitChallenge { .. }
            | ExchangeCommand::AudioListenForResponse { .. }
            | ExchangeCommand::AudioStop => {
                // Audio proximity not available in terminal — silently skip
                // (not fatal, just means no proximity verification)
            }
            _ => {
                // Unknown ExchangeCommand — report as unavailable
                let _ = app.app_engine.handle_hardware_event(
                    ExchangeHardwareEvent::HardwareUnavailable {
                        transport: "unknown".into(),
                    },
                );
            }
        }
    }
}
