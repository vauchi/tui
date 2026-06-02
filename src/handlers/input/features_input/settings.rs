// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Settings screen key handler.

use crossterm::event::KeyCode;

use crate::app::{App, PrivacyState, Screen};
use vauchi_app::ui::FormDialogType;

pub(in crate::handlers::input) fn handle_settings_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('n') | KeyCode::Enter => {
            let current_name = app.display_name().unwrap_or("").to_string();
            app.goto_form_dialog(FormDialogType::EditName { current_name });
        }
        KeyCode::Char('u') => {
            let current_url = app.relay_url.clone();
            app.goto_form_dialog(FormDialogType::EditRelayUrl { current_url });
        }
        KeyCode::Char('b') => app.goto(Screen::Backup),
        KeyCode::Char('d') => app.goto(Screen::Devices),
        KeyCode::Char('r') => app.goto(Screen::Recovery),
        KeyCode::Char('p') | KeyCode::Char('g') => {
            // Open Privacy & Data screen
            app.privacy_state = PrivacyState::default();
            app.goto(Screen::Privacy);
        }
        KeyCode::Char('e') => {
            // Open Emergency Broadcast screen — engine loads config on navigate.
            app.goto(Screen::Emergency);
        }
        KeyCode::Char('D') => {
            // Open Duress PIN screen — engine loads config on navigate.
            app.goto(Screen::Duress);
        }
        KeyCode::Char('s') => {
            // Open Support Vauchi screen
            app.goto(Screen::Support);
        }
        KeyCode::Char(']') | KeyCode::Right => {
            // Next theme
            app.next_theme();
            let id = &app.theme_ids[app.theme_index];
            app.set_status(format!("Theme: {}", id));
        }
        KeyCode::Char('[') | KeyCode::Left => {
            // Previous theme
            app.prev_theme();
            let id = &app.theme_ids[app.theme_index];
            app.set_status(format!("Theme: {}", id));
        }
        _ => {}
    }
}
