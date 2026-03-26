// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Duress PIN screen key handler.

use crossterm::event::KeyCode;

use crate::app::{App, DuressFocus, DuressState, InputMode};

pub(super) fn refresh_duress_state(app: &mut App) {
    let password_enabled = app
        .app_engine
        .vauchi()
        .is_password_enabled()
        .unwrap_or(false);
    let enabled = app.app_engine.vauchi().is_duress_enabled().unwrap_or(false);

    let (contact_ids_input, message_input, include_location, alert_contact_count) =
        match app.app_engine.vauchi().load_duress_settings() {
            Ok(Some(settings)) => (
                settings.alert_contact_ids.join(", "),
                settings.alert_message,
                settings.include_location,
                settings.alert_contact_ids.len(),
            ),
            _ => (String::new(), String::new(), false, 0),
        };

    app.duress_state = DuressState {
        password_enabled,
        enabled,
        pin_input: String::new(),
        contact_ids_input,
        message_input,
        include_location,
        alert_contact_count,
        focus: DuressFocus::Status,
    };
}

pub(in crate::handlers::input) fn handle_duress_keys(app: &mut App, key: KeyCode) {
    match app.duress_state.focus {
        DuressFocus::Status => match key {
            KeyCode::Char('p') if app.duress_state.password_enabled => {
                // Start PIN setup
                app.duress_state.pin_input.clear();
                app.duress_state.focus = DuressFocus::PinSetup;
                app.input_mode = InputMode::Editing;
            }
            KeyCode::Char('a') if app.duress_state.enabled => {
                // Configure alert settings
                app.duress_state.focus = DuressFocus::ContactIds;
                app.input_mode = InputMode::Editing;
            }
            KeyCode::Char('l') if app.duress_state.enabled => {
                // Toggle location
                app.duress_state.include_location = !app.duress_state.include_location;
                if app.duress_state.alert_contact_count > 0 {
                    let ids: Vec<String> = app
                        .duress_state
                        .contact_ids_input
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    let settings = vauchi_core::types::DuressSettings {
                        alert_contact_ids: ids,
                        alert_message: app.duress_state.message_input.clone(),
                        include_location: app.duress_state.include_location,
                    };
                    let _ = app.app_engine.vauchi().save_duress_settings(&settings);
                    app.invalidate_engines();
                }
                app.set_status(format!(
                    "Location: {}",
                    if app.duress_state.include_location {
                        "included"
                    } else {
                        "excluded"
                    }
                ));
            }
            KeyCode::Char('x') if app.duress_state.enabled => {
                // Disable duress mode
                match app.app_engine.vauchi_mut().disable_duress() {
                    Ok(()) => {
                        let _ = app.app_engine.vauchi().delete_duress_settings();
                        app.invalidate_engines();
                        refresh_duress_state(app);
                        app.set_status("Duress mode disabled");
                    }
                    Err(e) => app.set_status(format!("Error: {}", e)),
                }
            }
            _ => {}
        },
        DuressFocus::PinSetup => match key {
            KeyCode::Char(c) => {
                app.duress_state.pin_input.push(c);
            }
            KeyCode::Backspace => {
                app.duress_state.pin_input.pop();
            }
            KeyCode::Enter => {
                let pin = app.duress_state.pin_input.clone();
                if pin.is_empty() {
                    app.set_status("PIN cannot be empty");
                } else {
                    match app.app_engine.vauchi_mut().setup_duress_password(&pin) {
                        Ok(()) => {
                            app.duress_state.enabled = true;
                            app.duress_state.pin_input.clear();
                            app.duress_state.focus = DuressFocus::Status;
                            app.input_mode = InputMode::Normal;
                            app.set_status("Duress PIN configured");
                        }
                        Err(e) => {
                            app.duress_state.pin_input.clear();
                            app.set_status(format!("Error: {}", e));
                        }
                    }
                }
            }
            KeyCode::Esc => {
                app.duress_state.pin_input.clear();
                app.duress_state.focus = DuressFocus::Status;
                app.input_mode = InputMode::Normal;
            }
            _ => {}
        },
        DuressFocus::ContactIds => match key {
            KeyCode::Char(c) => {
                app.duress_state.contact_ids_input.push(c);
            }
            KeyCode::Backspace => {
                app.duress_state.contact_ids_input.pop();
            }
            KeyCode::Tab | KeyCode::Enter => {
                // Move to message editing
                if app.duress_state.message_input.is_empty() {
                    app.duress_state.message_input =
                        "Duress alert — contact may be under coercion".to_string();
                }
                app.duress_state.focus = DuressFocus::Message;
            }
            KeyCode::Esc => {
                app.duress_state.focus = DuressFocus::Status;
                app.input_mode = InputMode::Normal;
            }
            _ => {}
        },
        DuressFocus::Message => match key {
            KeyCode::Char(c) => {
                app.duress_state.message_input.push(c);
            }
            KeyCode::Backspace => {
                app.duress_state.message_input.pop();
            }
            KeyCode::Enter => {
                // Save alert settings
                let ids: Vec<String> = app
                    .duress_state
                    .contact_ids_input
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if ids.is_empty() {
                    app.set_status("At least one contact ID is required");
                } else {
                    let settings = vauchi_core::types::DuressSettings {
                        alert_contact_ids: ids.clone(),
                        alert_message: app.duress_state.message_input.clone(),
                        include_location: app.duress_state.include_location,
                    };
                    match app.app_engine.vauchi().save_duress_settings(&settings) {
                        Ok(()) => {
                            app.invalidate_engines();
                            app.duress_state.alert_contact_count = ids.len();
                            app.duress_state.focus = DuressFocus::Status;
                            app.input_mode = InputMode::Normal;
                            app.set_status(format!(
                                "Duress alerts configured ({} contacts)",
                                ids.len()
                            ));
                        }
                        Err(e) => app.set_status(format!("Error: {}", e)),
                    }
                }
            }
            KeyCode::Esc => {
                app.duress_state.focus = DuressFocus::Status;
                app.input_mode = InputMode::Normal;
            }
            _ => {}
        },
    }
}
