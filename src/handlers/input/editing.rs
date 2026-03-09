// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Editing mode handler and field/name/URL edit screens.

use crossterm::event::KeyCode;

use crate::app::{AddFieldFocus, App, BackupFocus, InputMode, Screen, SocialPickerState};
use crate::backend::{Backend, FIELD_TYPES};
use vauchi_core::aha_moments::AhaMomentType;

use super::Action;

pub(super) fn handle_editing_mode(app: &mut App, key: KeyCode) -> Action {
    match key {
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Enter => {
            // Submit the input
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Backspace => match app.screen {
            Screen::AddField => match app.add_field_state.focus {
                AddFieldFocus::Label => {
                    app.add_field_state.label.pop();
                }
                AddFieldFocus::Value => {
                    app.add_field_state.value.pop();
                }
                _ => {}
            },
            Screen::EditField => {
                app.edit_field_state.new_value.pop();
            }
            Screen::EditName => {
                app.edit_name_state.new_name.pop();
            }
            Screen::EditRelayUrl => {
                app.edit_relay_url_state.new_url.pop();
            }
            Screen::Backup => match app.backup_state.focus {
                BackupFocus::Password => {
                    app.backup_state.password.pop();
                }
                BackupFocus::Confirm => {
                    app.backup_state.confirm_password.pop();
                }
                BackupFocus::Data => {
                    app.backup_state.backup_data.pop();
                }
            },
            _ => {
                app.input_buffer.pop();
            }
        },
        KeyCode::Char(c) => match app.screen {
            Screen::AddField => match app.add_field_state.focus {
                AddFieldFocus::Label => app.add_field_state.label.push(c),
                AddFieldFocus::Value => app.add_field_state.value.push(c),
                _ => {}
            },
            Screen::EditField => app.edit_field_state.new_value.push(c),
            Screen::EditName => app.edit_name_state.new_name.push(c),
            Screen::EditRelayUrl => app.edit_relay_url_state.new_url.push(c),
            Screen::Backup => match app.backup_state.focus {
                BackupFocus::Password => app.backup_state.password.push(c),
                BackupFocus::Confirm => app.backup_state.confirm_password.push(c),
                BackupFocus::Data => app.backup_state.backup_data.push(c),
            },
            _ => app.input_buffer.push(c),
        },
        _ => {}
    }
    Action::Continue
}

/// Returns true if the currently selected field type is "Social".
fn is_social_type(app: &App) -> bool {
    FIELD_TYPES[app.add_field_state.field_type_index] == "Social"
}

/// Transition from Type to the next focus: Network picker for Social, Label for others.
fn next_focus_after_type(app: &mut App) -> AddFieldFocus {
    if is_social_type(app) {
        // Load social networks into the picker
        let networks = Backend::list_social_networks();
        app.add_field_state.social_picker = SocialPickerState {
            networks: networks
                .into_iter()
                .map(|n| (n.id, n.display_name))
                .collect(),
            selected: 0,
        };
        AddFieldFocus::Network
    } else {
        app.input_mode = InputMode::Editing;
        AddFieldFocus::Label
    }
}

pub(super) fn handle_add_field_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Tab => {
            // Cycle through fields
            app.add_field_state.focus = match app.add_field_state.focus {
                AddFieldFocus::Type => next_focus_after_type(app),
                AddFieldFocus::Network => {
                    // After selecting network, go to Value (label is auto-set)
                    app.input_mode = InputMode::Editing;
                    AddFieldFocus::Value
                }
                AddFieldFocus::Label => {
                    app.input_mode = InputMode::Editing;
                    AddFieldFocus::Value
                }
                AddFieldFocus::Value => {
                    app.input_mode = InputMode::Normal;
                    AddFieldFocus::Type
                }
            };
        }
        KeyCode::Enter => {
            match app.add_field_state.focus {
                AddFieldFocus::Value => {
                    // Submit the field
                    let field_type = Backend::parse_field_type(
                        FIELD_TYPES[app.add_field_state.field_type_index],
                    );
                    if let Err(e) = app.backend.add_field(
                        field_type,
                        &app.add_field_state.label,
                        &app.add_field_state.value,
                    ) {
                        app.set_status(format!("Error: {}", e));
                    } else {
                        app.invalidate_engines();
                        app.set_status("Field added");
                        app.go_back();
                    }
                }
                AddFieldFocus::Network => {
                    // Confirm network selection: set label to network display name,
                    // then move to Value input
                    if let Some((_, display_name)) = app
                        .add_field_state
                        .social_picker
                        .networks
                        .get(app.add_field_state.social_picker.selected)
                    {
                        app.add_field_state.label = display_name.clone();
                    }
                    app.add_field_state.focus = AddFieldFocus::Value;
                    app.input_mode = InputMode::Editing;
                }
                AddFieldFocus::Type => {
                    app.add_field_state.focus = next_focus_after_type(app);
                }
                AddFieldFocus::Label => {
                    app.add_field_state.focus = AddFieldFocus::Value;
                    app.input_mode = InputMode::Editing;
                }
            }
        }
        // Network picker navigation (j/k/Down/Up in Normal mode)
        KeyCode::Char('j') | KeyCode::Down
            if app.add_field_state.focus == AddFieldFocus::Network =>
        {
            let count = app.add_field_state.social_picker.networks.len();
            if app.add_field_state.social_picker.selected < count.saturating_sub(1) {
                app.add_field_state.social_picker.selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up if app.add_field_state.focus == AddFieldFocus::Network => {
            if app.add_field_state.social_picker.selected > 0 {
                app.add_field_state.social_picker.selected -= 1;
            }
        }
        // Type selector left/right
        KeyCode::Left | KeyCode::Char('h') if app.add_field_state.focus == AddFieldFocus::Type => {
            if app.add_field_state.field_type_index > 0 {
                app.add_field_state.field_type_index -= 1;
            }
        }
        KeyCode::Right | KeyCode::Char('l') if app.add_field_state.focus == AddFieldFocus::Type => {
            if app.add_field_state.field_type_index < FIELD_TYPES.len() - 1 {
                app.add_field_state.field_type_index += 1;
            }
        }
        _ => {}
    }
}

pub(super) fn handle_edit_field_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Enter => {
            // Save the edited field
            let label = app.edit_field_state.field_label.clone();
            let new_value = app.edit_field_state.new_value.trim().to_string();
            if new_value.is_empty() {
                app.set_status("Value cannot be empty");
            } else {
                match app.backend.update_field(&label, &new_value) {
                    Ok(()) => {
                        app.invalidate_engines();
                        if let Some(moment) = app.backend.check_aha_moment(AhaMomentType::FirstEdit)
                        {
                            app.set_status(format!("★ {} — {}", moment.title(), moment.message()));
                        } else {
                            app.set_status("Field updated");
                        }
                        app.go_back();
                    }
                    Err(e) => app.set_status(format!("Error: {}", e)),
                }
            }
        }
        KeyCode::Esc => {
            app.go_back();
        }
        _ => {}
    }
}

pub(super) fn handle_edit_name_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Enter => {
            // Save the new display name
            let new_name = app.edit_name_state.new_name.trim().to_string();
            if new_name.is_empty() {
                app.set_status("Name cannot be empty");
            } else {
                match app.backend.update_display_name(&new_name) {
                    Ok(()) => {
                        app.invalidate_engines();
                        app.set_status("Display name updated");
                        app.go_back();
                    }
                    Err(e) => app.set_status(format!("Error: {}", e)),
                }
            }
        }
        KeyCode::Esc => {
            app.go_back();
        }
        _ => {}
    }
}

pub(super) fn handle_edit_relay_url_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Enter => {
            // Save the new relay URL
            let new_url = app.edit_relay_url_state.new_url.trim().to_string();
            if new_url.is_empty() {
                app.set_status("URL cannot be empty");
            } else {
                match app.backend.set_relay_url(&new_url) {
                    Ok(()) => {
                        app.invalidate_engines();
                        app.set_status("Relay URL updated");
                        app.go_back();
                    }
                    Err(e) => app.set_status(format!("Error: {}", e)),
                }
            }
        }
        KeyCode::Esc => {
            app.go_back();
        }
        _ => {}
    }
}
