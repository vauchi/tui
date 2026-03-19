// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Sync screen key handler.

use crossterm::event::KeyCode;

use crate::app::App;

pub(in crate::handlers::input) fn handle_sync_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('s') => {
            // Start sync (background thread — non-blocking)
            if app.sync_state.is_syncing {
                app.set_status("Sync already in progress");
                return;
            }

            let req = match app.build_sync_request() {
                Some(r) => r,
                None => {
                    app.set_status("No identity — cannot sync");
                    return;
                }
            };

            // Mark as syncing before spawning
            app.sync_state.is_syncing = true;
            app.sync_state.sync_log.push("Starting sync...".to_string());
            app.set_status("Syncing...");

            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let result = crate::sync_service::sync_owned(req);
                let _ = tx.send(result);
            });
            app.sync_rx = Some(rx);
        }
        KeyCode::Char('t') => {
            // Test relay connection (background thread — non-blocking)
            if app.sync_state.is_syncing {
                app.set_status("Sync in progress — wait before testing");
                return;
            }

            app.set_status("Testing relay connection...");

            let relay_url = app.relay_url.clone();
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let result = crate::sync_service::test_relay_connection_owned(relay_url);
                let _ = tx.send(result);
            });
            app.relay_test_rx = Some(rx);
        }
        KeyCode::Char('r') => {
            // Refresh pending update count
            app.sync_state.pending_updates =
                app.app_engine.vauchi().pending_update_count().unwrap_or(0);
            app.set_status(format!(
                "{} pending updates",
                app.sync_state.pending_updates
            ));
        }
        _ => {}
    }
}
