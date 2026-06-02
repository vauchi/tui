// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Editing mode handler and field/name/URL edit screens.

use crossterm::event::KeyCode;

use crate::app::{App, InputMode, Screen};

use super::Action;

/// Editing-mode handler.
///
/// FormDialog screens (AddField, EditField, EditName, EditRelayUrl) are
/// engine-driven via `handle_engine_keys` and never reach this path.
/// This handler covers ContactImport, Backup, and the legacy
/// `input_buffer` fallback.
pub(super) fn handle_editing_mode(app: &mut App, key: KeyCode) -> Action {
    match key {
        KeyCode::Esc => {
            if app.screen == Screen::ContactImport {
                app.goto(Screen::Contacts);
            }
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Enter => {
            if app.screen == Screen::ContactImport {
                handle_import_submit(app);
            }
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Backspace => match app.screen {
            Screen::ContactImport => {
                app.import_state.file_path.pop();
            }
            _ => {
                app.input_buffer.pop();
            }
        },
        KeyCode::Char(c) => match app.screen {
            Screen::ContactImport => app.import_state.file_path.push(c),
            _ => app.input_buffer.push(c),
        },
        _ => {}
    }
    Action::Continue
}

/// Read a vCard file and import contacts via core API.
fn handle_import_submit(app: &mut App) {
    let path = app.import_state.file_path.trim().to_string();
    if path.is_empty() {
        app.import_state.result_message = Some("No file path entered".into());
        app.import_state.success = false;
        return;
    }

    let expanded = if path.starts_with('~') {
        dirs::home_dir()
            .map(|h| h.join(&path[2..]).to_string_lossy().to_string())
            .unwrap_or(path.clone())
    } else {
        path.clone()
    };

    let data = match std::fs::read(&expanded) {
        Ok(d) => d,
        Err(e) => {
            app.import_state.result_message = Some(format!("Cannot read file: {e}"));
            app.import_state.success = false;
            return;
        }
    };

    match app.app_engine.vauchi().import_contacts_from_vcf(&data) {
        Ok(result) => {
            let mut msg = format!("Imported {} contact(s)", result.imported);
            if result.skipped > 0 {
                msg.push_str(&format!(", skipped {}", result.skipped));
            }
            app.import_state.result_message = Some(msg.clone());
            app.import_state.success = true;
            app.set_status(msg);
            app.goto(Screen::Contacts);
        }
        Err(e) => {
            app.import_state.result_message = Some(format!("Import failed: {e}"));
            app.import_state.success = false;
        }
    }
}
