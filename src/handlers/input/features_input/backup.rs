// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Backup screen key handler.

use crossterm::event::KeyCode;

use crate::app::{App, BackupFocus, BackupMode, InputMode};
use vauchi_core::identity::password::validate_password;

pub(in crate::handlers::input) fn handle_backup_keys(app: &mut App, key: KeyCode) {
    match app.backup_state.mode {
        BackupMode::Menu => match key {
            KeyCode::Char('e') => {
                app.backup_state.mode = BackupMode::Export;
                app.backup_state.password.clear();
                app.backup_state.confirm_password.clear();
                app.backup_state.focus = BackupFocus::Password;
                app.input_mode = InputMode::Editing;
            }
            KeyCode::Char('i') => {
                app.backup_state.mode = BackupMode::Import;
                app.backup_state.backup_data.clear();
                app.backup_state.password.clear();
                app.backup_state.focus = BackupFocus::Data;
                app.input_mode = InputMode::Editing;
            }
            _ => {}
        },
        BackupMode::Export => match key {
            KeyCode::Tab => {
                app.backup_state.focus = match app.backup_state.focus {
                    BackupFocus::Password => BackupFocus::Confirm,
                    BackupFocus::Confirm => BackupFocus::Password,
                    BackupFocus::Data => BackupFocus::Password,
                };
                app.input_mode = InputMode::Editing;
            }
            KeyCode::Enter => {
                // Check passwords match first
                if app.backup_state.password != app.backup_state.confirm_password {
                    app.set_status("Passwords don't match");
                    return;
                }

                // Validate password strength
                match validate_password(&app.backup_state.password) {
                    Ok(_) => {
                        // Password is strong enough, proceed with export
                        match app
                            .app_engine
                            .vauchi()
                            .export_backup(&app.backup_state.password)
                        {
                            Ok(data) => {
                                app.set_status(format!(
                                    "Backup: {}...",
                                    &data[..50.min(data.len())]
                                ));
                                app.backup_state.mode = BackupMode::Menu;
                                app.backup_state = Default::default();
                            }
                            Err(e) => app.set_status(format!("Export error: {}", e)),
                        }
                    }
                    Err(_) => {
                        if app.backup_state.password.len() < 8 {
                            app.set_status("Password must be at least 8 characters");
                        } else {
                            app.set_status("Password too weak. Use a stronger passphrase.");
                        }
                    }
                }
            }
            KeyCode::Esc => {
                app.backup_state.mode = BackupMode::Menu;
                app.input_mode = InputMode::Normal;
            }
            _ => {}
        },
        BackupMode::Import => match key {
            KeyCode::Tab => {
                app.backup_state.focus = match app.backup_state.focus {
                    BackupFocus::Data => BackupFocus::Password,
                    BackupFocus::Password => BackupFocus::Data,
                    BackupFocus::Confirm => BackupFocus::Data,
                };
                app.input_mode = InputMode::Editing;
            }
            KeyCode::Enter => {
                if !app.backup_state.backup_data.is_empty() && !app.backup_state.password.is_empty()
                {
                    match app
                        .app_engine
                        .vauchi_mut()
                        .import_backup(&app.backup_state.backup_data, &app.backup_state.password)
                    {
                        Ok(()) => {
                            app.set_status("Backup imported successfully!");
                            app.backup_state = Default::default();
                            app.go_back();
                        }
                        Err(e) => app.set_status(format!("Import error: {}", e)),
                    }
                } else {
                    app.set_status("Please enter backup data and password");
                }
            }
            KeyCode::Esc => {
                app.backup_state.mode = BackupMode::Menu;
                app.input_mode = InputMode::Normal;
            }
            _ => {}
        },
    }
}
