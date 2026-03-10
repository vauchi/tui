// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Editing mode handler and field/name/URL edit screens.

use crossterm::event::KeyCode;

use crate::app::{AddFieldFocus, App, BackupFocus, InputMode, Screen};

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
