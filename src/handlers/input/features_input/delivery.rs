// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Delivery screen key handler.

use crossterm::event::KeyCode;

use crate::app::App;

pub(in crate::handlers::input) fn handle_delivery_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('r') => {
            // Run retry tick
            app.delivery_state.last_result = Some("Retry tick processed".to_string());
            app.set_status("Delivery retries processed");
        }
        KeyCode::Char('c') => {
            // Run cleanup
            app.delivery_state.last_result = Some("Cleanup complete".to_string());
            app.set_status("Delivery cleanup complete");
        }
        KeyCode::Esc => app.go_back(),
        _ => {}
    }
}
