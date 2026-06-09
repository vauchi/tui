// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Lock screen key handler.

use crossterm::event::KeyCode;

use crate::app::{App, LockState};

/// Handle lock screen input — PIN entry to unlock the app.
///
/// Feature: duress_pin.feature @unlock
/// The lock screen intercepts all input. Only character entry, backspace,
/// and Enter are processed. Esc stays on the lock screen (no escape).
/// 'q' does NOT quit — it's a PIN character.
pub(in crate::handlers::input) fn handle_lock_keys(app: &mut App, key: KeyCode) {
    use crate::ui::widgets::key_mapping::{self, KeyResult};
    use vauchi_app::ui::{ActionResult, AppScreen, WorkflowEngine};
    use vauchi_core::api::AuthMode;

    // Try engine-driven handling first
    if let Some(engine) = app.lock_engine.as_mut() {
        let screen = engine.current_screen();
        let key_result = key_mapping::map_key(key, &screen, &mut app.render_state);

        match key_result {
            KeyResult::Action(action) => {
                // Sync lock_state from TextChanged actions before forwarding
                match &action {
                    vauchi_app::ui::UserAction::TextChanged {
                        component_id,
                        value: _,
                    } if component_id == "pin" => {
                        // PinInput sends single chars — accumulate in lock_state
                        match key {
                            KeyCode::Char(c) => {
                                app.lock_state.error = false;
                                app.lock_state.pin_input.push(c);
                            }
                            KeyCode::Backspace => {
                                app.lock_state.pin_input.pop();
                                app.lock_state.error = false;
                            }
                            _ => {}
                        }
                        // Feed full accumulated value to engine
                        if let Some(engine) = app.lock_engine.as_mut() {
                            let _ = engine.handle_action(vauchi_app::ui::UserAction::TextChanged {
                                component_id: "pin".into(),
                                value: app.lock_state.pin_input.clone(),
                            });
                        }
                        return;
                    }
                    _ => {}
                }

                let result = engine.handle_action(action);
                match result {
                    ActionResult::Complete => {
                        // Engine says PIN was submitted — verify via Vauchi API
                        let pin = &app.lock_state.pin_input;
                        if pin.is_empty() {
                            return;
                        }
                        match app.app_engine.vauchi_mut().authenticate(pin) {
                            Ok(AuthMode::Normal) => {
                                app.lock_state = LockState::default();
                                app.lock_engine = None;
                                app.goto(AppScreen::MyInfo);
                            }
                            Ok(AuthMode::Duress) => {
                                app.lock_state = LockState::default();
                                app.lock_engine = None;
                                app.goto(AppScreen::MyInfo);
                            }
                            Ok(AuthMode::Unauthenticated) | Ok(_) | Err(_) => {
                                // Invalid PIN, unknown mode, or error
                                app.lock_state.pin_input.clear();
                                app.lock_state.attempts += 1;
                                app.lock_state.error = true;
                                if let Some(engine) = app.lock_engine.as_mut() {
                                    engine.record_failed_attempt();
                                }
                            }
                        }
                    }
                    ActionResult::UpdateScreen(_) => {}
                    ActionResult::ValidationError { message, .. } => {
                        app.set_status(message);
                    }
                    _ => {}
                }
            }
            KeyResult::Consumed => {}
            KeyResult::Unhandled => {}
        }
    } else {
        // Legacy fallback: no engine
        match key {
            KeyCode::Char(c) => {
                app.lock_state.error = false;
                app.lock_state.pin_input.push(c);
            }
            KeyCode::Backspace => {
                app.lock_state.pin_input.pop();
                app.lock_state.error = false;
            }
            KeyCode::Enter => {
                if app.lock_state.pin_input.is_empty() {
                    return;
                }
                match app
                    .app_engine
                    .vauchi_mut()
                    .authenticate(&app.lock_state.pin_input)
                {
                    Ok(AuthMode::Normal) => {
                        app.lock_state = LockState::default();
                        app.goto(AppScreen::MyInfo);
                    }
                    Ok(AuthMode::Duress) => {
                        app.lock_state = LockState::default();
                        app.goto(AppScreen::MyInfo);
                    }
                    Ok(AuthMode::Unauthenticated) | Ok(_) | Err(_) => {
                        app.lock_state.pin_input.clear();
                        app.lock_state.attempts += 1;
                        app.lock_state.error = true;
                    }
                }
            }
            _ => {}
        }
    }
}
