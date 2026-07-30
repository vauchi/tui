// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Live terminal adapter for the Core presentation reducer.

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use vauchi_core::{Command, Event};

use crate::app::App;
use crate::ui::presentation_input::KeyOutcome;

use super::Action;

pub fn handle_presentation_key(app: &mut App, key: KeyEvent) -> Action {
    if app.alert_message.is_some() {
        if matches!(key.code, KeyCode::Esc | KeyCode::Enter) {
            app.alert_message = None;
        }
        return Action::Continue;
    }
    if let Some(action) = handle_interactive_effect(app, key) {
        return action;
    }
    let outcome = app
        .presentation_interaction
        .key_outcome(&app.presentation, key);
    match outcome {
        KeyOutcome::Events(events) => {
            for event in events {
                if let Err(error) = app.dispatch_presentation_event(event) {
                    app.alert_message = Some(("Presentation error".into(), error.to_string()));
                    break;
                }
            }
        }
        KeyOutcome::Quit => return Action::Quit,
        KeyOutcome::Consumed => {}
    }
    if app.presentation.native_back_requested() || app.should_quit {
        Action::Quit
    } else {
        Action::Continue
    }
}

fn handle_interactive_effect(app: &mut App, key: KeyEvent) -> Option<Action> {
    let effect = app.presentation_effects.front()?.clone();
    if !matches!(
        effect,
        Command::FilePickFromUser { .. }
            | Command::QrRequestScan
            | Command::ImagePickFromFile
            | Command::ImagePickFromLibrary
            | Command::ImageCaptureFromCamera
    ) {
        return None;
    }

    match key.code {
        KeyCode::Esc => {
            app.presentation_effects.pop_front();
            app.input_buffer.clear();
            let event = match effect {
                Command::FilePickFromUser { .. } => Event::FilePickCancelledByUser,
                Command::ImagePickFromFile
                | Command::ImagePickFromLibrary
                | Command::ImageCaptureFromCamera => Event::ImagePickCancelled,
                Command::QrRequestScan => Event::HardwareUnavailable {
                    transport: "qr_scan".into(),
                },
                _ => unreachable!("interactive effects are filtered above"),
            };
            dispatch_effect_result(app, event);
        }
        KeyCode::Enter => {
            let value = app.input_buffer.trim().to_string();
            if value.is_empty() {
                return Some(Action::Continue);
            }
            let event = match effect {
                Command::QrRequestScan => Event::QrScanned { data: value },
                Command::FilePickFromUser { .. } => {
                    let bytes = match std::fs::read(&value) {
                        Ok(bytes) => bytes,
                        Err(error) => {
                            app.alert_message =
                                Some(("Unable to read file".into(), error.to_string()));
                            return Some(Action::Continue);
                        }
                    };
                    let filename = std::path::Path::new(&value)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or_default()
                        .to_string();
                    Event::FilePickedFromUser { bytes, filename }
                }
                Command::ImagePickFromFile
                | Command::ImagePickFromLibrary
                | Command::ImageCaptureFromCamera => {
                    let data = match std::fs::read(&value) {
                        Ok(data) => data,
                        Err(error) => {
                            app.alert_message =
                                Some(("Unable to read image".into(), error.to_string()));
                            return Some(Action::Continue);
                        }
                    };
                    Event::ImageReceived { data }
                }
                _ => unreachable!("interactive effects are filtered above"),
            };
            app.presentation_effects.pop_front();
            app.input_buffer.clear();
            dispatch_effect_result(app, event);
        }
        KeyCode::Backspace => {
            app.input_buffer.pop();
        }
        KeyCode::Char(character)
            if !key
                .modifiers
                .intersects(crossterm::event::KeyModifiers::CONTROL) =>
        {
            app.input_buffer.push(character);
        }
        _ => {}
    }
    Some(if app.presentation.native_back_requested() {
        Action::Quit
    } else {
        Action::Continue
    })
}

fn dispatch_effect_result(app: &mut App, event: Event) {
    if let Err(error) = app.dispatch_presentation_event(event) {
        app.alert_message = Some(("Presentation error".into(), error.to_string()));
    }
}
