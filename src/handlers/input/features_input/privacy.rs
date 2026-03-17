// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Privacy & Data screen key handler.

use crossterm::event::KeyCode;

use crate::app::App;

pub(in crate::handlers::input) fn handle_privacy_keys(app: &mut App, key: KeyCode) {
    // Total items: 0=Export, 1=Deletion, 2..5=Consent types
    let total_items = 6;

    match key {
        KeyCode::Char('j') | KeyCode::Down => {
            if app.privacy_state.selected_item < total_items - 1 {
                app.privacy_state.selected_item += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.privacy_state.selected_item > 0 {
                app.privacy_state.selected_item -= 1;
            }
        }
        KeyCode::Char('e') => {
            // Export GDPR data
            let storage = app.app_engine.vauchi().storage();
            match vauchi_core::api::export_all_data(storage) {
                Ok(export) => match serde_json::to_string_pretty(&export) {
                    Ok(json) => {
                        let path = app.data_dir.join("gdpr_export.json");
                        match std::fs::write(&path, &json) {
                            Ok(_) => app.set_status(format!("Data exported to {:?}", path)),
                            Err(e) => app.set_status(format!("Export failed: {}", e)),
                        }
                    }
                    Err(e) => app.set_status(format!("Export failed: {}", e)),
                },
                Err(e) => app.set_status(format!("Export failed: {}", e)),
            }
        }
        KeyCode::Char('d') => {
            // Schedule identity deletion
            let storage = app.app_engine.vauchi().storage();
            match vauchi_core::api::DeletionManager::new(storage).schedule_deletion() {
                Ok(_) => app.set_status("Identity deletion scheduled (7 day grace period)"),
                Err(e) => app.set_status(format!("Schedule failed: {}", e)),
            }
        }
        KeyCode::Char('c') => {
            // Cancel scheduled deletion
            let storage = app.app_engine.vauchi().storage();
            match vauchi_core::api::DeletionManager::new(storage).cancel_deletion() {
                Ok(_) => app.set_status("Identity deletion cancelled"),
                Err(e) => app.set_status(format!("Cancel failed: {}", e)),
            }
        }
        KeyCode::Char('x') => {
            // Execute scheduled deletion (after grace period)
            let has_identity = app.app_engine.vauchi().identity().is_some();
            if has_identity {
                let vauchi = app.app_engine.vauchi();
                let storage = vauchi.storage();
                let identity = vauchi.identity().unwrap();
                match vauchi_core::api::DeletionManager::new(storage).execute_deletion(identity) {
                    Ok(result) => {
                        let count = result.revocations.len();
                        app.set_status(format!("DELETED: {} revocation(s) generated", count))
                    }
                    Err(e) => app.set_status(format!("Execute failed: {}", e)),
                }
            } else {
                app.set_status("Execute failed: no identity loaded");
            }
        }
        KeyCode::Char('!') => {
            // Emergency panic shred
            match app.app_engine.vauchi_mut().perform_emergency_wipe(true) {
                Ok(()) => app.set_status("PANIC SHRED: All data wiped"),
                Err(e) => app.set_status(format!("Panic shred failed: {}", e)),
            }
        }
        KeyCode::Char(' ') | KeyCode::Enter => {
            // Toggle consent for selected item (items 2..4 are consent types)
            let consent_index = app.privacy_state.selected_item;
            if consent_index >= 2 {
                let consent_type = match consent_index - 2 {
                    0 => vauchi_core::api::ConsentType::DataProcessing,
                    1 => vauchi_core::api::ConsentType::ContactSharing,
                    _ => vauchi_core::api::ConsentType::RecoveryVouching,
                };

                // Check current state and toggle
                let records = app
                    .app_engine
                    .vauchi()
                    .export_consent_log()
                    .unwrap_or_default();
                let currently_granted = records
                    .iter()
                    .rfind(|r| r.consent_type == consent_type)
                    .map(|r| r.granted)
                    .unwrap_or(false);

                let result = if currently_granted {
                    app.app_engine.vauchi().revoke_consent(consent_type)
                } else {
                    app.app_engine.vauchi().grant_consent(consent_type)
                };

                match result {
                    Ok(_) => {
                        let action = if currently_granted {
                            "Revoked"
                        } else {
                            "Granted"
                        };
                        app.set_status(format!("Consent {}", action));
                    }
                    Err(e) => app.set_status(format!("Error: {}", e)),
                }
            }
        }
        _ => {}
    }
}
