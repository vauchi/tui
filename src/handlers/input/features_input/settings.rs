// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Settings screen key handler.

use crossterm::event::KeyCode;

use crate::app::{App, EditNameState, EditRelayUrlState, PrivacyState, Screen};

use super::duress::refresh_duress_state;
use super::emergency::refresh_emergency_state;
use super::tor::refresh_tor_state;

pub(in crate::handlers::input) fn handle_settings_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('n') | KeyCode::Enter => {
            // Edit display name — engine-driven via FormDialogEngine
            let current_name = app.display_name().unwrap_or("").to_string();
            app.edit_name_state = EditNameState {
                new_name: current_name,
            };
            app.goto(Screen::EditName);
        }
        KeyCode::Char('u') => {
            // Edit relay URL — engine-driven via FormDialogEngine
            let current_url = app.relay_url.clone();
            app.edit_relay_url_state = EditRelayUrlState {
                new_url: current_url,
            };
            app.goto(Screen::EditRelayUrl);
        }
        KeyCode::Char('b') => app.goto(Screen::Backup),
        KeyCode::Char('d') => app.goto(Screen::Devices),
        KeyCode::Char('r') => app.goto(Screen::Recovery),
        KeyCode::Char('t') => {
            // Load current Tor config and navigate to Tor settings
            refresh_tor_state(app);
            app.goto(Screen::TorSettings);
        }
        KeyCode::Char('p') | KeyCode::Char('g') => {
            // Open Privacy & Data screen
            app.privacy_state = PrivacyState::default();
            app.goto(Screen::Privacy);
        }
        KeyCode::Char('e') => {
            // Open Emergency Broadcast screen
            refresh_emergency_state(app);
            app.goto(Screen::Emergency);
        }
        KeyCode::Char('D') => {
            // Open Duress PIN screen
            refresh_duress_state(app);
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
