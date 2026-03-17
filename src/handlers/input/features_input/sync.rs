// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Sync screen key handler.

use crossterm::event::KeyCode;

use crate::app::App;
use vauchi_core::aha_moments::AhaMomentType;

pub(in crate::handlers::input) fn handle_sync_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('s') => {
            // Start sync
            if app.sync_state.is_syncing {
                app.set_status("Sync already in progress");
                return;
            }

            // Mark as syncing
            app.sync_state.is_syncing = true;
            app.sync_state.sync_log.push("Starting sync...".to_string());

            // Perform sync (Backend-specific async WebSocket operation)
            let result = app.sync();

            // Update state based on result
            app.sync_state.is_syncing = false;

            if result.success {
                app.sync_state.connected = true;
                let summary = format!(
                    "+{} contacts, {} updated, {} sent",
                    result.contacts_added, result.cards_updated, result.updates_sent
                );
                app.sync_state.last_result = Some(summary.clone());
                app.sync_state
                    .sync_log
                    .push(format!("Sync complete: {}", summary));
                app.set_status(format!("Sync complete: {}", summary));

                // Update pending count
                app.sync_state.pending_updates =
                    app.app_engine.vauchi().pending_update_count().unwrap_or(0);

                // Check for aha moments based on sync results
                if result.contacts_added > 0 {
                    if let Ok(Some(moment)) = app
                        .app_engine
                        .vauchi()
                        .try_trigger_aha_moment(AhaMomentType::FirstContactAdded)
                    {
                        app.set_status(format!("★ {} — {}", moment.title(), moment.message()));
                    }
                }
                if result.cards_updated > 0 {
                    if let Ok(Some(moment)) = app
                        .app_engine
                        .vauchi()
                        .try_trigger_aha_moment(AhaMomentType::FirstUpdateReceived)
                    {
                        app.set_status(format!("★ {} — {}", moment.title(), moment.message()));
                    }
                }
                if result.updates_sent > 0 {
                    if let Ok(Some(moment)) = app
                        .app_engine
                        .vauchi()
                        .try_trigger_aha_moment(AhaMomentType::FirstOutboundDelivered)
                    {
                        app.set_status(format!("★ {} — {}", moment.title(), moment.message()));
                    }
                }
            } else {
                app.sync_state.connected = false;
                let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
                app.sync_state.last_result = Some(format!("Failed: {}", error_msg));
                app.sync_state
                    .sync_log
                    .push(format!("Sync failed: {}", error_msg));
                app.set_status(format!(
                    "Sync failed: {}. Changes saved locally and will sync when connected.",
                    error_msg
                ));
            }
        }
        KeyCode::Char('t') => {
            // Test relay connection (Backend-specific async WebSocket operation)
            app.set_status("Testing relay connection...");
            match app.test_relay_connection() {
                Ok(true) => {
                    app.sync_state.connected = true;
                    app.sync_state
                        .sync_log
                        .push("Relay connection test: OK".to_string());
                    app.set_status("Relay connection successful!");
                }
                Ok(false) | Err(_) => {
                    app.sync_state.connected = false;
                    app.sync_state
                        .sync_log
                        .push("Relay connection test: FAILED".to_string());
                    app.set_status(
                        "Relay connection failed. Check your network or relay URL in Settings.",
                    );
                }
            }
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
