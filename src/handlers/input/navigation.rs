// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Navigation screen handlers: setup, home, help.

use crossterm::event::KeyCode;

use crate::app::{App, BackupFocus, BackupMode, EditFieldState, InputMode, Screen};
use vauchi_core::aha_moments::AhaMomentType;

pub(super) fn handle_setup_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('c') => {
            // Create a new identity with a default name
            // User can change it later in settings
            if let Err(e) = app.backend.create_identity("New User") {
                app.set_status(format!("Failed to create identity: {}", e));
            } else {
                if let Some(moment) = app
                    .backend
                    .check_aha_moment(AhaMomentType::CardCreationComplete)
                {
                    app.set_status(format!("★ {} — {}", moment.title(), moment.message()));
                } else {
                    app.set_status("Identity created! You can edit your name in Settings.");
                }
                app.goto(Screen::Home);
            }
        }
        KeyCode::Char('i') => {
            // Go to backup import
            app.backup_state.mode = BackupMode::Import;
            app.backup_state.backup_data.clear();
            app.backup_state.password.clear();
            app.backup_state.focus = BackupFocus::Data;
            app.input_mode = InputMode::Editing;
            app.goto(Screen::Backup);
        }
        _ => {}
    }
}

pub(super) fn handle_home_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('c') => app.goto(Screen::Contacts),
        KeyCode::Char('s') => app.goto(Screen::Settings),
        KeyCode::Char('d') => app.goto(Screen::Devices),
        KeyCode::Char('r') => app.goto(Screen::Recovery),
        KeyCode::Char('n') => app.goto(Screen::Sync),
        KeyCode::Char('y') => app.goto(Screen::Delivery),
        KeyCode::Char('b') => app.goto(Screen::Backup),
        KeyCode::Char('g') => app.goto(Screen::Groups),
        KeyCode::Char('X') => app.goto(Screen::Exchange),
        KeyCode::Char('a') => {
            app.add_field_state = Default::default();
            app.goto(Screen::AddField);
        }
        KeyCode::Char('e') | KeyCode::Enter => {
            // Edit selected field
            if let Ok(fields) = app.backend.get_card_fields() {
                if let Some(field) = fields.get(app.selected_field) {
                    app.edit_field_state = EditFieldState {
                        field_label: field.label.clone(),
                        field_type: field.field_type.clone(),
                        new_value: field.value.clone(),
                    };
                    app.goto(Screen::EditField);
                    app.input_mode = InputMode::Editing;
                } else {
                    // No fields, open Exchange
                    app.goto(Screen::Exchange);
                }
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let fields = app.backend.get_card_fields().unwrap_or_default();
            if app.selected_field < fields.len().saturating_sub(1) {
                app.selected_field += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.selected_field > 0 {
                app.selected_field -= 1;
            }
        }
        KeyCode::Char('x') | KeyCode::Delete => {
            // Delete selected field
            if let Ok(fields) = app.backend.get_card_fields() {
                if let Some(field) = fields.get(app.selected_field) {
                    let label = field.label.clone();
                    if app.backend.remove_field(&field.id).is_ok() {
                        app.set_status(format!("Field removed: {}", label));
                        if app.selected_field > 0 {
                            app.selected_field -= 1;
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

pub(super) fn handle_help_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => {
            app.go_back();
        }
        _ => {}
    }
}
