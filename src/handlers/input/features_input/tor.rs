// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tor settings screen key handler.

use crossterm::event::KeyCode;

use crate::app::App;

/// Refresh the Tor state from Vauchi config.
pub(super) fn refresh_tor_state(app: &mut App) {
    let config = &app.app_engine.vauchi().config().tor;
    app.tor_state.enabled = config.enabled;
    app.tor_state.prefer_onion = config.prefer_onion;
    app.tor_state.circuit_rotation_secs = config.circuit_rotation_secs;
    app.tor_state.bridge_count = config.bridges.len();
}

pub(in crate::handlers::input) fn handle_tor_settings_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('e') => {
            // Enable Tor preference (NOT wired to connections yet)
            if app.tor_state.enabled {
                app.set_status("Tor preference is already enabled (not wired to connections)");
                return;
            }
            match app.app_engine.vauchi_mut().enable_tor() {
                Ok(()) => {
                    app.set_status("Tor preference saved — not yet wired to connections");
                    refresh_tor_state(app);
                }
                Err(e) => app.set_status(format!("Error: {}", e)),
            }
        }
        KeyCode::Char('d') => {
            // Disable Tor preference
            if !app.tor_state.enabled {
                app.set_status("Tor preference is already disabled");
                return;
            }
            match app.app_engine.vauchi_mut().disable_tor() {
                Ok(()) => {
                    app.set_status("Tor preference disabled");
                    refresh_tor_state(app);
                }
                Err(e) => app.set_status(format!("Error: {}", e)),
            }
        }
        KeyCode::Char('o') => {
            // Toggle .onion preference
            match app.app_engine.vauchi_mut().toggle_prefer_onion() {
                Ok(prefer_onion) => {
                    let msg = if prefer_onion {
                        ".onion addresses will be preferred"
                    } else {
                        ".onion preference disabled"
                    };
                    app.set_status(msg);
                    refresh_tor_state(app);
                }
                Err(e) => app.set_status(format!("Error: {}", e)),
            }
        }
        KeyCode::Char('n') => {
            // Request new circuit
            if !app.tor_state.enabled {
                app.set_status("Enable Tor mode first");
                return;
            }
            app.set_status("Circuit rotation not available — Tor not wired to connections");
        }
        KeyCode::Char('x') => {
            // Clear bridges
            match app.app_engine.vauchi_mut().clear_tor_bridges() {
                Ok(0) => app.set_status("No bridges to clear"),
                Ok(count) => {
                    app.set_status(format!("Cleared {} bridge(s)", count));
                    refresh_tor_state(app);
                }
                Err(e) => app.set_status(format!("Error: {}", e)),
            }
        }
        _ => {}
    }
}
