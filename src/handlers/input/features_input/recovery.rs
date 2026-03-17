// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Recovery screen key handler.

use crossterm::event::KeyCode;

use crate::app::App;

pub(in crate::handlers::input) fn handle_recovery_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('c') => {
            app.set_status("Create claim: use CLI 'vauchi recovery claim <old-pk>'");
        }
        KeyCode::Char('v') => {
            app.set_status("Vouch: use CLI 'vauchi recovery vouch <claim>'");
        }
        KeyCode::Char('s') => match app.app_engine.vauchi().get_recovery_readiness() {
            Ok(readiness) => {
                let msg = format!(
                    "Recovery: {}/{} trusted contacts",
                    readiness.trusted_count, readiness.threshold
                );
                app.set_status(msg);
            }
            Err(e) => app.set_status(format!("Error: {}", e)),
        },
        _ => {}
    }
}
