// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Emergency broadcast screen key handler.

use crossterm::event::KeyCode;

use crate::app::{App, EmergencyFocus, EmergencyState, InputMode};

pub(super) fn refresh_emergency_state(app: &mut App) {
    let last_broadcast = app.emergency_state.last_broadcast_time;
    match app.app_engine.vauchi().load_emergency_config() {
        Ok(Some(config)) => {
            app.emergency_state = EmergencyState {
                configured: true,
                contact_ids_input: config.trusted_contact_ids.join(", "),
                message_input: config.message,
                include_location: config.include_location,
                trusted_count: config.trusted_contact_ids.len(),
                focus: EmergencyFocus::Status,
                last_broadcast_time: last_broadcast,
            };
        }
        _ => {
            app.emergency_state = EmergencyState {
                last_broadcast_time: last_broadcast,
                ..EmergencyState::default()
            };
        }
    }
}

pub(in crate::handlers::input) fn handle_emergency_keys(app: &mut App, key: KeyCode) {
    match app.emergency_state.focus {
        EmergencyFocus::Status => match key {
            KeyCode::Char('s') => {
                // Send emergency broadcast (with confirmation)
                if !app.emergency_state.configured {
                    app.set_status("Configure emergency broadcast first");
                    return;
                }
                // Rate limit: 60 seconds between broadcasts
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                if let Some(last) = app.emergency_state.last_broadcast_time {
                    if now.saturating_sub(last) < 60 {
                        app.set_status("Alert recently sent. Wait before sending again.");
                        return;
                    }
                }
                app.emergency_state.focus = EmergencyFocus::Confirm;
            }
            KeyCode::Char('c') => {
                // Configure: start editing contact IDs
                if !app.emergency_state.configured {
                    app.emergency_state.message_input =
                        vauchi_core::api::emergency::DEFAULT_EMERGENCY_MESSAGE.to_string();
                }
                app.emergency_state.focus = EmergencyFocus::ContactIds;
                app.input_mode = InputMode::Editing;
            }
            KeyCode::Char('l') => {
                // Toggle location
                app.emergency_state.include_location = !app.emergency_state.include_location;
                if app.emergency_state.configured {
                    let ids: Vec<String> = app
                        .emergency_state
                        .contact_ids_input
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    let _ = app.app_engine.vauchi_mut().configure_emergency_broadcast(
                        ids,
                        app.emergency_state.message_input.clone(),
                        app.emergency_state.include_location,
                    );
                    app.set_status(format!(
                        "Location: {}",
                        if app.emergency_state.include_location {
                            "included"
                        } else {
                            "excluded"
                        }
                    ));
                }
            }
            KeyCode::Char('x') => {
                // Disable emergency broadcast
                if app.emergency_state.configured {
                    match app.app_engine.vauchi_mut().delete_emergency_config() {
                        Ok(()) => {
                            app.emergency_state = EmergencyState::default();
                            app.set_status("Emergency broadcast disabled");
                        }
                        Err(e) => app.set_status(format!("Error: {}", e)),
                    }
                }
            }
            _ => {}
        },
        EmergencyFocus::Confirm => match key {
            KeyCode::Char('y') | KeyCode::Enter => {
                // Confirmed: send broadcast
                match app.app_engine.vauchi_mut().send_emergency_broadcast() {
                    Ok(result) => {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        app.emergency_state.last_broadcast_time = Some(now);
                        app.set_status(format!(
                            "Emergency broadcast sent: {}/{} contacts reached",
                            result.sent, result.total
                        ));
                    }
                    Err(e) => app.set_status(format!("Broadcast failed: {}", e)),
                }
                app.emergency_state.focus = EmergencyFocus::Status;
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                // Cancelled
                app.emergency_state.focus = EmergencyFocus::Status;
                app.set_status("Broadcast cancelled");
            }
            _ => {}
        },
        EmergencyFocus::ContactIds => match key {
            KeyCode::Char(c) => {
                app.emergency_state.contact_ids_input.push(c);
            }
            KeyCode::Backspace => {
                app.emergency_state.contact_ids_input.pop();
            }
            KeyCode::Tab | KeyCode::Enter => {
                // Move to message editing
                app.emergency_state.focus = EmergencyFocus::Message;
            }
            KeyCode::Esc => {
                app.emergency_state.focus = EmergencyFocus::Status;
                app.input_mode = InputMode::Normal;
            }
            _ => {}
        },
        EmergencyFocus::Message => match key {
            KeyCode::Char(c) => {
                app.emergency_state.message_input.push(c);
            }
            KeyCode::Backspace => {
                app.emergency_state.message_input.pop();
            }
            KeyCode::Enter => {
                // Save configuration
                let ids: Vec<String> = app
                    .emergency_state
                    .contact_ids_input
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if ids.is_empty() {
                    app.set_status("At least one contact ID is required");
                } else if ids.len() > vauchi_core::api::MAX_TRUSTED_CONTACTS {
                    app.set_status(format!(
                        "Maximum {} trusted contacts",
                        vauchi_core::api::MAX_TRUSTED_CONTACTS
                    ));
                } else {
                    match app.app_engine.vauchi_mut().configure_emergency_broadcast(
                        ids.clone(),
                        app.emergency_state.message_input.clone(),
                        app.emergency_state.include_location,
                    ) {
                        Ok(()) => {
                            app.emergency_state.configured = true;
                            app.emergency_state.trusted_count = ids.len();
                            app.emergency_state.focus = EmergencyFocus::Status;
                            app.input_mode = InputMode::Normal;
                            app.set_status(format!(
                                "Emergency broadcast configured ({} contacts)",
                                ids.len()
                            ));
                        }
                        Err(e) => app.set_status(format!("Error: {}", e)),
                    }
                }
            }
            KeyCode::Esc => {
                app.emergency_state.focus = EmergencyFocus::Status;
                app.input_mode = InputMode::Normal;
            }
            _ => {}
        },
    }
}
