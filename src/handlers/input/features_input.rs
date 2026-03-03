// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Feature screen handlers: exchange, settings, devices, recovery, delivery,
//! sync, tor, privacy, support, backup, emergency, duress.

use crossterm::event::KeyCode;

use crate::app::App;
use crate::app::{
    BackupFocus, BackupMode, DuressFocus, DuressState, EditNameState, EditRelayUrlState,
    EmergencyFocus, EmergencyState, InputMode, LockState, PrivacyState, Screen,
};
use vauchi_core::aha_moments::AhaMomentType;
use vauchi_core::identity::password::validate_password;

pub(super) fn handle_exchange_keys(app: &mut App, key: KeyCode) {
    use crate::ui::exchange::regenerate_qr;
    if let KeyCode::Char('r') = key {
        regenerate_qr(app);
    }
}

pub(super) fn handle_settings_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('n') | KeyCode::Enter => {
            // Edit display name
            let current_name = app.backend.display_name().unwrap_or("").to_string();
            app.edit_name_state = EditNameState {
                new_name: current_name,
            };
            app.goto(Screen::EditName);
            app.input_mode = InputMode::Editing;
        }
        KeyCode::Char('u') => {
            // Edit relay URL
            let current_url = app.backend.relay_url().to_string();
            app.edit_relay_url_state = EditRelayUrlState {
                new_url: current_url,
            };
            app.goto(Screen::EditRelayUrl);
            app.input_mode = InputMode::Editing;
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

pub(super) fn handle_devices_keys(app: &mut App, key: KeyCode) {
    // Handle revoke confirmation overlay
    if app.revoke_confirm {
        match key {
            KeyCode::Char('y') => {
                app.revoke_confirm = false;
                match app.backend.revoke_device(app.selected_device) {
                    Ok(name) => app.set_status(format!("Device '{}' revoked", name)),
                    Err(e) => app.set_status(format!("Revoke failed: {}", e)),
                }
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                app.revoke_confirm = false;
            }
            _ => {}
        }
        return;
    }

    // Handle QR overlay — Esc dismisses
    if app.device_link_result.is_some() {
        if key == KeyCode::Esc {
            app.device_link_result = None;
        }
        return;
    }

    match key {
        KeyCode::Char('j') | KeyCode::Down => {
            if let Ok(devices) = app.backend.list_devices() {
                if app.selected_device < devices.len().saturating_sub(1) {
                    app.selected_device += 1;
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.selected_device > 0 {
                app.selected_device -= 1;
            }
        }
        KeyCode::Char('l') => match app.backend.generate_device_link() {
            Ok(result) => {
                app.device_link_result = Some(result);
            }
            Err(e) => app.set_status(format!("Error: {}", e)),
        },
        KeyCode::Char('r') => {
            // Check if selected device is current or already revoked
            if let Ok(devices) = app.backend.list_devices() {
                if let Some(device) = devices.get(app.selected_device) {
                    if device.is_current {
                        app.set_status("Cannot revoke the current device");
                    } else if !device.is_active {
                        app.set_status("Device is already revoked");
                    } else {
                        app.revoke_confirm = true;
                    }
                }
            }
        }
        _ => {}
    }
}

pub(super) fn handle_recovery_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('c') => {
            app.set_status("Create claim: use CLI 'vauchi recovery claim <old-pk>'");
        }
        KeyCode::Char('v') => {
            app.set_status("Vouch: use CLI 'vauchi recovery vouch <claim>'");
        }
        KeyCode::Char('s') => match app.backend.get_recovery_status() {
            Ok(status) => {
                let msg = format!(
                    "Recovery: {}/{} vouchers",
                    status.voucher_count, status.required_vouchers
                );
                app.set_status(msg);
            }
            Err(e) => app.set_status(format!("Error: {}", e)),
        },
        _ => {}
    }
}

pub(super) fn handle_delivery_keys(app: &mut App, key: KeyCode) {
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

pub(super) fn handle_sync_keys(app: &mut App, key: KeyCode) {
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

            // Perform sync
            let result = app.backend.sync();

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
                app.sync_state.pending_updates = app.backend.pending_update_count().unwrap_or(0);

                // Check for aha moments based on sync results
                if result.contacts_added > 0 {
                    if let Some(moment) = app
                        .backend
                        .check_aha_moment(AhaMomentType::FirstContactAdded)
                    {
                        app.set_status(format!("★ {} — {}", moment.title(), moment.message()));
                    }
                }
                if result.cards_updated > 0 {
                    if let Some(moment) = app
                        .backend
                        .check_aha_moment(AhaMomentType::FirstUpdateReceived)
                    {
                        app.set_status(format!("★ {} — {}", moment.title(), moment.message()));
                    }
                }
                if result.updates_sent > 0 {
                    if let Some(moment) = app
                        .backend
                        .check_aha_moment(AhaMomentType::FirstOutboundDelivered)
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
                app.set_status(format!("Sync failed: {}", error_msg));
            }
        }
        KeyCode::Char('t') => {
            // Test relay connection
            app.set_status("Testing relay connection...");
            match app.backend.test_relay_connection() {
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
                    app.set_status("Relay connection failed");
                }
            }
        }
        KeyCode::Char('r') => {
            // Refresh pending update count
            app.sync_state.pending_updates = app.backend.pending_update_count().unwrap_or(0);
            app.set_status(format!(
                "{} pending updates",
                app.sync_state.pending_updates
            ));
        }
        _ => {}
    }
}

/// Refresh the Tor state from backend storage.
fn refresh_tor_state(app: &mut App) {
    match app.backend.load_tor_config() {
        Ok(config) => {
            app.tor_state.enabled = config.enabled;
            app.tor_state.prefer_onion = config.prefer_onion;
            app.tor_state.circuit_rotation_secs = config.circuit_rotation_secs;
            app.tor_state.bridge_count = config.bridges.len();
        }
        Err(_) => {
            // Leave defaults (all disabled)
        }
    }
}

pub(super) fn handle_tor_settings_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('e') => {
            // Enable Tor
            if app.tor_state.enabled {
                app.set_status("Tor mode is already enabled");
                return;
            }
            match app.backend.enable_tor() {
                Ok(()) => {
                    app.set_status("Tor mode enabled");
                    refresh_tor_state(app);
                }
                Err(e) => app.set_status(format!("Error: {}", e)),
            }
        }
        KeyCode::Char('d') => {
            // Disable Tor
            if !app.tor_state.enabled {
                app.set_status("Tor mode is already disabled");
                return;
            }
            match app.backend.disable_tor() {
                Ok(()) => {
                    app.set_status("Tor mode disabled");
                    refresh_tor_state(app);
                }
                Err(e) => app.set_status(format!("Error: {}", e)),
            }
        }
        KeyCode::Char('o') => {
            // Toggle .onion preference
            match app.backend.toggle_prefer_onion() {
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
            app.set_status("Circuit rotation requested");
        }
        KeyCode::Char('x') => {
            // Clear bridges
            match app.backend.clear_tor_bridges() {
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

pub(super) fn handle_privacy_keys(app: &mut App, key: KeyCode) {
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
            match app.backend.export_gdpr_data() {
                Ok(json) => {
                    let path = app.backend.data_dir().join("gdpr_export.json");
                    match std::fs::write(&path, &json) {
                        Ok(_) => app.set_status(format!("Data exported to {:?}", path)),
                        Err(e) => app.set_status(format!("Export failed: {}", e)),
                    }
                }
                Err(e) => app.set_status(format!("Export failed: {}", e)),
            }
        }
        KeyCode::Char('d') => {
            // Schedule account deletion
            match app.backend.schedule_deletion() {
                Ok(_) => app.set_status("Account deletion scheduled (7 day grace period)"),
                Err(e) => app.set_status(format!("Schedule failed: {}", e)),
            }
        }
        KeyCode::Char('c') => {
            // Cancel scheduled deletion
            match app.backend.cancel_deletion() {
                Ok(_) => app.set_status("Account deletion cancelled"),
                Err(e) => app.set_status(format!("Cancel failed: {}", e)),
            }
        }
        KeyCode::Char('x') => {
            // Execute scheduled deletion (after grace period)
            match app.backend.execute_deletion() {
                Ok(summary) => app.set_status(format!("DELETED: {}", summary)),
                Err(e) => app.set_status(format!("Execute failed: {}", e)),
            }
        }
        KeyCode::Char('!') => {
            // Emergency panic shred
            match app.backend.panic_shred() {
                Ok(summary) => app.set_status(format!("PANIC SHRED: {}", summary)),
                Err(e) => app.set_status(format!("Panic shred failed: {}", e)),
            }
        }
        KeyCode::Char(' ') | KeyCode::Enter => {
            // Toggle consent for selected item (items 2..5 are consent types)
            let consent_index = app.privacy_state.selected_item;
            if consent_index >= 2 {
                let consent_type = match consent_index - 2 {
                    0 => vauchi_core::api::ConsentType::DataProcessing,
                    1 => vauchi_core::api::ConsentType::ContactSharing,
                    2 => vauchi_core::api::ConsentType::Analytics,
                    _ => vauchi_core::api::ConsentType::RecoveryVouching,
                };

                // Check current state and toggle
                let records = app.backend.consent_records().unwrap_or_default();
                let currently_granted = records
                    .iter()
                    .rfind(|r| r.consent_type == consent_type)
                    .map(|r| r.granted)
                    .unwrap_or(false);

                let result = if currently_granted {
                    app.backend.revoke_consent(consent_type)
                } else {
                    app.backend.grant_consent(consent_type)
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

pub(super) fn handle_support_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('1') => {
            if open::that("https://github.com/sponsors/vauchi").is_err() {
                app.set_status("Could not open browser");
            }
        }
        KeyCode::Char('2') => {
            if open::that("https://liberapay.com/Vauchi/donate").is_err() {
                app.set_status("Could not open browser");
            }
        }
        _ => {}
    }
}

pub(super) fn handle_backup_keys(app: &mut App, key: KeyCode) {
    match app.backup_state.mode {
        BackupMode::Menu => match key {
            KeyCode::Char('e') => {
                app.backup_state.mode = BackupMode::Export;
                app.backup_state.password.clear();
                app.backup_state.confirm_password.clear();
                app.backup_state.focus = BackupFocus::Password;
                app.input_mode = InputMode::Editing;
            }
            KeyCode::Char('i') => {
                app.backup_state.mode = BackupMode::Import;
                app.backup_state.backup_data.clear();
                app.backup_state.password.clear();
                app.backup_state.focus = BackupFocus::Data;
                app.input_mode = InputMode::Editing;
            }
            _ => {}
        },
        BackupMode::Export => match key {
            KeyCode::Tab => {
                app.backup_state.focus = match app.backup_state.focus {
                    BackupFocus::Password => BackupFocus::Confirm,
                    BackupFocus::Confirm => BackupFocus::Password,
                    BackupFocus::Data => BackupFocus::Password,
                };
                app.input_mode = InputMode::Editing;
            }
            KeyCode::Enter => {
                // Check passwords match first
                if app.backup_state.password != app.backup_state.confirm_password {
                    app.set_status("Passwords don't match");
                    return;
                }

                // Validate password strength
                match validate_password(&app.backup_state.password) {
                    Ok(_) => {
                        // Password is strong enough, proceed with export
                        match app.backend.export_backup(&app.backup_state.password) {
                            Ok(data) => {
                                app.set_status(format!(
                                    "Backup: {}...",
                                    &data[..50.min(data.len())]
                                ));
                                app.backup_state.mode = BackupMode::Menu;
                                app.backup_state = Default::default();
                            }
                            Err(e) => app.set_status(format!("Export error: {}", e)),
                        }
                    }
                    Err(_) => {
                        if app.backup_state.password.len() < 8 {
                            app.set_status("Password must be at least 8 characters");
                        } else {
                            app.set_status("Password too weak. Use a stronger passphrase.");
                        }
                    }
                }
            }
            KeyCode::Esc => {
                app.backup_state.mode = BackupMode::Menu;
                app.input_mode = InputMode::Normal;
            }
            _ => {}
        },
        BackupMode::Import => match key {
            KeyCode::Tab => {
                app.backup_state.focus = match app.backup_state.focus {
                    BackupFocus::Data => BackupFocus::Password,
                    BackupFocus::Password => BackupFocus::Data,
                    BackupFocus::Confirm => BackupFocus::Data,
                };
                app.input_mode = InputMode::Editing;
            }
            KeyCode::Enter => {
                if !app.backup_state.backup_data.is_empty() && !app.backup_state.password.is_empty()
                {
                    match app
                        .backend
                        .import_backup(&app.backup_state.backup_data, &app.backup_state.password)
                    {
                        Ok(()) => {
                            app.set_status("Backup imported successfully!");
                            app.backup_state = Default::default();
                            app.go_back();
                        }
                        Err(e) => app.set_status(format!("Import error: {}", e)),
                    }
                } else {
                    app.set_status("Please enter backup data and password");
                }
            }
            KeyCode::Esc => {
                app.backup_state.mode = BackupMode::Menu;
                app.input_mode = InputMode::Normal;
            }
            _ => {}
        },
    }
}

fn refresh_emergency_state(app: &mut App) {
    let last_broadcast = app.emergency_state.last_broadcast_time;
    match app.backend.get_emergency_config() {
        Ok(Some(config)) => {
            app.emergency_state = EmergencyState {
                configured: true,
                contact_ids_input: config.trusted_contact_ids.join(", "),
                message_input: config.message,
                include_location: config.include_location,
                trusted_count: config.trusted_contact_ids.len(),
                focus: EmergencyFocus::Status,
                last_broadcast_time: last_broadcast,
            };
        }
        _ => {
            app.emergency_state = EmergencyState {
                last_broadcast_time: last_broadcast,
                ..EmergencyState::default()
            };
        }
    }
}

pub(super) fn handle_emergency_keys(app: &mut App, key: KeyCode) {
    match app.emergency_state.focus {
        EmergencyFocus::Status => match key {
            KeyCode::Char('s') => {
                // Send emergency broadcast (with confirmation)
                if !app.emergency_state.configured {
                    app.set_status("Configure emergency broadcast first");
                    return;
                }
                // Rate limit: 60 seconds between broadcasts
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                if let Some(last) = app.emergency_state.last_broadcast_time {
                    if now.saturating_sub(last) < 60 {
                        app.set_status("Alert recently sent. Wait before sending again.");
                        return;
                    }
                }
                app.emergency_state.focus = EmergencyFocus::Confirm;
            }
            KeyCode::Char('c') => {
                // Configure: start editing contact IDs
                if !app.emergency_state.configured {
                    app.emergency_state.message_input =
                        vauchi_core::api::emergency::DEFAULT_EMERGENCY_MESSAGE.to_string();
                }
                app.emergency_state.focus = EmergencyFocus::ContactIds;
                app.input_mode = InputMode::Editing;
            }
            KeyCode::Char('l') => {
                // Toggle location
                app.emergency_state.include_location = !app.emergency_state.include_location;
                if app.emergency_state.configured {
                    let ids: Vec<String> = app
                        .emergency_state
                        .contact_ids_input
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    let _ = app.backend.configure_emergency_broadcast(
                        ids,
                        app.emergency_state.message_input.clone(),
                        app.emergency_state.include_location,
                    );
                    app.set_status(format!(
                        "Location: {}",
                        if app.emergency_state.include_location {
                            "included"
                        } else {
                            "excluded"
                        }
                    ));
                }
            }
            KeyCode::Char('x') => {
                // Disable emergency broadcast
                if app.emergency_state.configured {
                    match app.backend.disable_emergency_broadcast() {
                        Ok(()) => {
                            app.emergency_state = EmergencyState::default();
                            app.set_status("Emergency broadcast disabled");
                        }
                        Err(e) => app.set_status(format!("Error: {}", e)),
                    }
                }
            }
            _ => {}
        },
        EmergencyFocus::Confirm => match key {
            KeyCode::Char('y') | KeyCode::Enter => {
                // Confirmed: send broadcast
                match app.backend.send_emergency_broadcast() {
                    Ok((sent, total)) => {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0);
                        app.emergency_state.last_broadcast_time = Some(now);
                        app.set_status(format!(
                            "Emergency broadcast sent: {}/{} contacts reached",
                            sent, total
                        ));
                    }
                    Err(e) => app.set_status(format!("Broadcast failed: {}", e)),
                }
                app.emergency_state.focus = EmergencyFocus::Status;
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                // Cancelled
                app.emergency_state.focus = EmergencyFocus::Status;
                app.set_status("Broadcast cancelled");
            }
            _ => {}
        },
        EmergencyFocus::ContactIds => match key {
            KeyCode::Char(c) => {
                app.emergency_state.contact_ids_input.push(c);
            }
            KeyCode::Backspace => {
                app.emergency_state.contact_ids_input.pop();
            }
            KeyCode::Tab | KeyCode::Enter => {
                // Move to message editing
                app.emergency_state.focus = EmergencyFocus::Message;
            }
            KeyCode::Esc => {
                app.emergency_state.focus = EmergencyFocus::Status;
                app.input_mode = InputMode::Normal;
            }
            _ => {}
        },
        EmergencyFocus::Message => match key {
            KeyCode::Char(c) => {
                app.emergency_state.message_input.push(c);
            }
            KeyCode::Backspace => {
                app.emergency_state.message_input.pop();
            }
            KeyCode::Enter => {
                // Save configuration
                let ids: Vec<String> = app
                    .emergency_state
                    .contact_ids_input
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if ids.is_empty() {
                    app.set_status("At least one contact ID is required");
                } else if ids.len() > vauchi_core::api::MAX_TRUSTED_CONTACTS {
                    app.set_status(format!(
                        "Maximum {} trusted contacts",
                        vauchi_core::api::MAX_TRUSTED_CONTACTS
                    ));
                } else {
                    match app.backend.configure_emergency_broadcast(
                        ids.clone(),
                        app.emergency_state.message_input.clone(),
                        app.emergency_state.include_location,
                    ) {
                        Ok(()) => {
                            app.emergency_state.configured = true;
                            app.emergency_state.trusted_count = ids.len();
                            app.emergency_state.focus = EmergencyFocus::Status;
                            app.input_mode = InputMode::Normal;
                            app.set_status(format!(
                                "Emergency broadcast configured ({} contacts)",
                                ids.len()
                            ));
                        }
                        Err(e) => app.set_status(format!("Error: {}", e)),
                    }
                }
            }
            KeyCode::Esc => {
                app.emergency_state.focus = EmergencyFocus::Status;
                app.input_mode = InputMode::Normal;
            }
            _ => {}
        },
    }
}

fn refresh_duress_state(app: &mut App) {
    let password_enabled = app.backend.is_password_enabled().unwrap_or(false);
    let enabled = app.backend.is_duress_enabled().unwrap_or(false);

    let (contact_ids_input, message_input, include_location, alert_contact_count) =
        match app.backend.load_duress_settings() {
            Ok(Some(settings)) => (
                settings.alert_contact_ids.join(", "),
                settings.alert_message,
                settings.include_location,
                settings.alert_contact_ids.len(),
            ),
            _ => (String::new(), String::new(), false, 0),
        };

    app.duress_state = DuressState {
        password_enabled,
        enabled,
        pin_input: String::new(),
        contact_ids_input,
        message_input,
        include_location,
        alert_contact_count,
        focus: DuressFocus::Status,
    };
}

pub(super) fn handle_duress_keys(app: &mut App, key: KeyCode) {
    match app.duress_state.focus {
        DuressFocus::Status => match key {
            KeyCode::Char('p') if app.duress_state.password_enabled => {
                // Start PIN setup
                app.duress_state.pin_input.clear();
                app.duress_state.focus = DuressFocus::PinSetup;
                app.input_mode = InputMode::Editing;
            }
            KeyCode::Char('a') if app.duress_state.enabled => {
                // Configure alert settings
                app.duress_state.focus = DuressFocus::ContactIds;
                app.input_mode = InputMode::Editing;
            }
            KeyCode::Char('l') if app.duress_state.enabled => {
                // Toggle location
                app.duress_state.include_location = !app.duress_state.include_location;
                if app.duress_state.alert_contact_count > 0 {
                    let ids: Vec<String> = app
                        .duress_state
                        .contact_ids_input
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    let settings = vauchi_core::api::DuressSettings {
                        alert_contact_ids: ids,
                        alert_message: app.duress_state.message_input.clone(),
                        include_location: app.duress_state.include_location,
                    };
                    let _ = app.backend.save_duress_settings(&settings);
                }
                app.set_status(format!(
                    "Location: {}",
                    if app.duress_state.include_location {
                        "included"
                    } else {
                        "excluded"
                    }
                ));
            }
            KeyCode::Char('x') if app.duress_state.enabled => {
                // Disable duress mode
                match app.backend.disable_duress() {
                    Ok(()) => {
                        let _ = app.backend.delete_duress_settings();
                        refresh_duress_state(app);
                        app.set_status("Duress mode disabled");
                    }
                    Err(e) => app.set_status(format!("Error: {}", e)),
                }
            }
            _ => {}
        },
        DuressFocus::PinSetup => match key {
            KeyCode::Char(c) => {
                app.duress_state.pin_input.push(c);
            }
            KeyCode::Backspace => {
                app.duress_state.pin_input.pop();
            }
            KeyCode::Enter => {
                let pin = app.duress_state.pin_input.clone();
                if pin.is_empty() {
                    app.set_status("PIN cannot be empty");
                } else {
                    match app.backend.setup_duress_password(&pin) {
                        Ok(()) => {
                            app.duress_state.enabled = true;
                            app.duress_state.pin_input.clear();
                            app.duress_state.focus = DuressFocus::Status;
                            app.input_mode = InputMode::Normal;
                            app.set_status("Duress PIN configured");
                        }
                        Err(e) => {
                            app.duress_state.pin_input.clear();
                            app.set_status(format!("Error: {}", e));
                        }
                    }
                }
            }
            KeyCode::Esc => {
                app.duress_state.pin_input.clear();
                app.duress_state.focus = DuressFocus::Status;
                app.input_mode = InputMode::Normal;
            }
            _ => {}
        },
        DuressFocus::ContactIds => match key {
            KeyCode::Char(c) => {
                app.duress_state.contact_ids_input.push(c);
            }
            KeyCode::Backspace => {
                app.duress_state.contact_ids_input.pop();
            }
            KeyCode::Tab | KeyCode::Enter => {
                // Move to message editing
                if app.duress_state.message_input.is_empty() {
                    app.duress_state.message_input =
                        "Duress alert — contact may be under coercion".to_string();
                }
                app.duress_state.focus = DuressFocus::Message;
            }
            KeyCode::Esc => {
                app.duress_state.focus = DuressFocus::Status;
                app.input_mode = InputMode::Normal;
            }
            _ => {}
        },
        DuressFocus::Message => match key {
            KeyCode::Char(c) => {
                app.duress_state.message_input.push(c);
            }
            KeyCode::Backspace => {
                app.duress_state.message_input.pop();
            }
            KeyCode::Enter => {
                // Save alert settings
                let ids: Vec<String> = app
                    .duress_state
                    .contact_ids_input
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if ids.is_empty() {
                    app.set_status("At least one contact ID is required");
                } else {
                    let settings = vauchi_core::api::DuressSettings {
                        alert_contact_ids: ids.clone(),
                        alert_message: app.duress_state.message_input.clone(),
                        include_location: app.duress_state.include_location,
                    };
                    match app.backend.save_duress_settings(&settings) {
                        Ok(()) => {
                            app.duress_state.alert_contact_count = ids.len();
                            app.duress_state.focus = DuressFocus::Status;
                            app.input_mode = InputMode::Normal;
                            app.set_status(format!(
                                "Duress alerts configured ({} contacts)",
                                ids.len()
                            ));
                        }
                        Err(e) => app.set_status(format!("Error: {}", e)),
                    }
                }
            }
            KeyCode::Esc => {
                app.duress_state.focus = DuressFocus::Status;
                app.input_mode = InputMode::Normal;
            }
            _ => {}
        },
    }
}

/// Handle lock screen input — PIN entry to unlock the app.
///
/// Feature: duress_pin.feature @unlock
/// The lock screen intercepts all input. Only character entry, backspace,
/// and Enter are processed. Esc stays on the lock screen (no escape).
/// 'q' does NOT quit — it's a PIN character.
pub(super) fn handle_lock_keys(app: &mut App, key: KeyCode) {
    use crate::backend::AuthResult;

    match key {
        KeyCode::Char(c) => {
            app.lock_state.error = false;
            app.lock_state.pin_input.push(c);
        }
        KeyCode::Backspace => {
            app.lock_state.pin_input.pop();
            app.lock_state.error = false;
        }
        KeyCode::Enter => {
            if app.lock_state.pin_input.is_empty() {
                return;
            }
            match app.backend.authenticate(&app.lock_state.pin_input) {
                Ok(AuthResult::Normal) => {
                    app.lock_state = LockState::default();
                    app.goto(Screen::Home);
                }
                Ok(AuthResult::Duress) => {
                    // Duress mode — proceed to Home but contacts will show decoys
                    app.lock_state = LockState::default();
                    app.goto(Screen::Home);
                    // No visual indication of duress mode (by design)
                }
                Ok(AuthResult::Invalid) => {
                    app.lock_state.pin_input.clear();
                    app.lock_state.attempts += 1;
                    app.lock_state.error = true;
                }
                Err(_) => {
                    app.lock_state.pin_input.clear();
                    app.lock_state.error = true;
                }
            }
        }
        // Esc, q, etc. — do nothing on lock screen
        _ => {}
    }
}
