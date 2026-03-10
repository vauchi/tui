// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Visual regression tests using ratatui TestBackend + insta snapshots.
//!
//! Each test creates a Backend with a temp directory, builds an App
//! in the target screen state, renders to a fixed-size terminal buffer,
//! and snapshots the text output.

use std::sync::Once;

use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;
use tempfile::TempDir;

use vauchi_tui::app::{
    AddFieldFocus, AddFieldState, App, BackupFocus, BackupMode, BackupState, DuressState,
    EditFieldState, EditNameState, EditRelayUrlState, PrivacyState, Screen, SyncState, TorState,
};
use vauchi_tui::backend::Backend;

/// Fixed terminal dimensions for consistent snapshots.
const WIDTH: u16 = 80;
const HEIGHT: u16 = 24;

/// Ensure locale data is loaded before snapshot tests run.
/// Searches for locale JSON files in standard locations (workspace locales repo,
/// core's bundled copy, or VAUCHI_LOCALES_DIR env var).
static INIT_LOCALES: Once = Once::new();

fn ensure_locales_loaded() {
    INIT_LOCALES.call_once(|| {
        let candidates = [
            std::env::var("VAUCHI_LOCALES_DIR").ok(),
            Some("../locales".to_string()),
            Some("../core/vauchi-core/locales".to_string()),
        ];
        for candidate in candidates.iter().flatten() {
            let path = std::path::Path::new(candidate);
            if path.join("en.json").exists() && vauchi_core::i18n::init(path).is_ok() {
                return;
            }
        }
        eprintln!("WARNING: Could not find locale files — snapshots may use fallback strings");
    });
}

/// Create a test backend with an identity and some card fields.
fn create_app_with_identity() -> (App, TempDir) {
    ensure_locales_loaded();
    let temp_dir = TempDir::new().expect("temp dir");
    let mut backend = Backend::new(temp_dir.path()).expect("backend");
    backend
        .create_identity("Alice Smith")
        .expect("create identity");
    backend
        .add_field(vauchi_core::FieldType::Email, "Work", "alice@company.com")
        .expect("add email");
    backend
        .add_field(vauchi_core::FieldType::Phone, "Mobile", "+41 79 123 45 67")
        .expect("add phone");
    backend
        .add_field(
            vauchi_core::FieldType::Website,
            "Blog",
            "https://alice.example.com",
        )
        .expect("add website");
    let app = App::new(backend);
    (app, temp_dir)
}

/// Create a test backend without an identity (setup state).
fn create_app_without_identity() -> (App, TempDir) {
    ensure_locales_loaded();
    let temp_dir = TempDir::new().expect("temp dir");
    let backend = Backend::new(temp_dir.path()).expect("backend");
    let app = App::new(backend);
    (app, temp_dir)
}

/// Render the app to a terminal buffer and extract text content.
/// Non-deterministic values (hex IDs, keys) are redacted for reproducibility.
fn render_to_string(app: &mut App) -> String {
    let backend = TestBackend::new(WIDTH, HEIGHT);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|f| vauchi_tui::ui::draw(f, app))
        .expect("draw");
    let raw = buffer_to_string(terminal.backend().buffer());
    redact_dynamic_values(&raw)
}

/// Replace hex-based IDs and keys with stable placeholders.
fn redact_dynamic_values(s: &str) -> String {
    // Replace truncated public IDs: 16 hex chars followed by "..."
    let re_public_id = regex::Regex::new(r"[0-9a-f]{16}\.\.\.").unwrap();
    let s = re_public_id.replace_all(s, "[PUBLIC_ID]...");
    // Replace truncated device keys: 8 hex chars followed by "..."  e.g. "(83c49a52...)"
    let re_device_key = regex::Regex::new(r"\([0-9a-f]{8}\.\.\.\)").unwrap();
    let s = re_device_key.replace_all(&s, "([DEVICE_KEY]...)");
    // Replace standalone 16-char hex sequences
    let re_hex16 = regex::Regex::new(r"\b[0-9a-f]{16}\b").unwrap();
    let s = re_hex16.replace_all(&s, "[HEX_ID]").to_string();
    // Redact QR code data lines (must contain at least one █) — crypto nonces make these non-deterministic
    let re_qr = regex::Regex::new(r"(?m)^(│\s+)[\s█]*█[\s█]*(\s+│)$").unwrap();
    re_qr.replace_all(&s, "$1[QR_DATA]$2").to_string()
}

/// Convert a ratatui buffer to a plain text string.
fn buffer_to_string(buffer: &Buffer) -> String {
    let mut lines = Vec::new();
    for y in 0..buffer.area.height {
        let mut line = String::new();
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            line.push_str(cell.symbol());
        }
        // Trim trailing whitespace per line
        lines.push(line.trim_end().to_string());
    }
    // Remove trailing empty lines
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

// =============================================================
// Setup / No Identity
// =============================================================

// @scenario: identity_management:Create new identity on first launch
#[test]
fn test_snapshot_setup_screen() {
    let (mut app, _tmp) = create_app_without_identity();
    assert_eq!(app.screen, Screen::SetupWelcome);
    let output = render_to_string(&mut app);
    insta::assert_snapshot!("setup_screen", output);
}

// =============================================================
// Home Screen
// =============================================================

// @scenario: contact_card_management:View contact card fields
#[test]
fn test_snapshot_home_with_fields() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::Home;
    let output = render_to_string(&mut app);
    insta::assert_snapshot!("home_with_fields", output);
}

// =============================================================
// Contacts
// =============================================================

// @scenario: contacts_management:View all contacts
#[test]
fn test_snapshot_contacts_empty() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::Contacts;
    let output = render_to_string(&mut app);
    insta::assert_snapshot!("contacts_empty", output);
}

// =============================================================
// Settings
// =============================================================

#[test]
fn test_snapshot_settings() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::Settings;
    let output = render_to_string(&mut app);
    insta::assert_snapshot!("settings", output);
}

// =============================================================
// Help
// =============================================================

#[test]
fn test_snapshot_help() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::Help;
    let output = render_to_string(&mut app);
    insta::assert_snapshot!("help", output);
}

// =============================================================
// Devices
// =============================================================

// @scenario: device_management:View linked devices
#[test]
fn test_snapshot_devices() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::Devices;
    let output = render_to_string(&mut app);
    insta::assert_snapshot!("devices", output);
}

// =============================================================
// Recovery
// =============================================================

// @scenario: identity_management:View recovery status
#[test]
fn test_snapshot_recovery() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::Recovery;
    let output = render_to_string(&mut app);
    insta::assert_snapshot!("recovery", output);
}

// =============================================================
// Sync
// =============================================================

// @scenario: sync_updates:View sync status
#[test]
fn test_snapshot_sync_idle() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::Sync;
    let output = render_to_string(&mut app);
    insta::assert_snapshot!("sync_idle", output);
}

// @scenario: sync_updates:Client initiates sync with relay
#[test]
fn test_snapshot_sync_connected() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::Sync;
    app.sync_state = SyncState {
        connected: true,
        is_syncing: false,
        pending_updates: 3,
        last_result: Some("Synced: 2 contacts, 1 update".to_string()),
        sync_log: vec![
            "Connected to relay".to_string(),
            "Received 2 contacts".to_string(),
            "Sent 1 update".to_string(),
        ],
    };
    let output = render_to_string(&mut app);
    insta::assert_snapshot!("sync_connected", output);
}

// =============================================================
// Backup
// =============================================================

// @scenario: identity_management:Create encrypted identity backup
#[test]
fn test_snapshot_backup_menu() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::Backup;
    let output = render_to_string(&mut app);
    insta::assert_snapshot!("backup_menu", output);
}

// @scenario: identity_management:Create encrypted identity backup
#[test]
fn test_snapshot_backup_export() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::Backup;
    app.backup_state = BackupState {
        mode: BackupMode::Export,
        password: String::new(),
        confirm_password: String::new(),
        backup_data: String::new(),
        focus: BackupFocus::Password,
    };
    let output = render_to_string(&mut app);
    insta::assert_snapshot!("backup_export", output);
}

// =============================================================
// Dialogs
// =============================================================

// @scenario: contact_card_management:Add a field to contact card
#[test]
fn test_snapshot_add_field_dialog() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::AddField;
    app.add_field_state = AddFieldState {
        field_type_index: 0,
        label: String::new(),
        value: String::new(),
        focus: AddFieldFocus::Type,
        ..Default::default()
    };
    let output = render_to_string(&mut app);
    insta::assert_snapshot!("add_field_dialog", output);
}

// @scenario: contact_card_management:Edit an existing field value
#[test]
fn test_snapshot_edit_field_dialog() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::EditField;
    app.edit_field_state = EditFieldState {
        field_id: "field_001".to_string(),
        field_label: "Mobile".to_string(),
        field_type: "Phone".to_string(),
        new_value: "+41 79 123 45 67".to_string(),
    };
    let output = render_to_string(&mut app);
    insta::assert_snapshot!("edit_field_dialog", output);
}

// @scenario: contact_card_management:Update display name
#[test]
fn test_snapshot_edit_name_dialog() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::EditName;
    app.edit_name_state = EditNameState {
        new_name: "Alice Smith".to_string(),
    };
    let output = render_to_string(&mut app);
    insta::assert_snapshot!("edit_name_dialog", output);
}

#[test]
fn test_snapshot_edit_relay_url_dialog() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::EditRelayUrl;
    app.edit_relay_url_state = EditRelayUrlState {
        new_url: "wss://relay.vauchi.app".to_string(),
    };
    let output = render_to_string(&mut app);
    insta::assert_snapshot!("edit_relay_url_dialog", output);
}

// ============================================================================
// tui-F-018 through tui-F-021: Missing snapshot tests
// ============================================================================

// @scenario: tor_mode:View Tor connection status
#[test]
fn test_snapshot_tor_settings() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::TorSettings;
    app.tor_state = TorState {
        enabled: false,
        prefer_onion: false,
        circuit_rotation_secs: 600,
        bridge_count: 0,
    };
    let output = render_to_string(&mut app);
    insta::assert_snapshot!("tor_settings", output);
}

// @scenario: visibility_control:Configure default visibility
#[test]
fn test_snapshot_privacy() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::Privacy;
    app.privacy_state = PrivacyState::default();
    let output = render_to_string(&mut app);
    insta::assert_snapshot!("privacy", output);
}

// @scenario: contact_exchange:Generate exchange QR code
#[test]
fn test_snapshot_exchange() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::Exchange;
    let output = render_to_string(&mut app);
    insta::assert_snapshot!("exchange", output);
}

// @scenario: identity_management:Restore identity from backup
#[test]
fn test_snapshot_backup_import() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::Backup;
    app.backup_state = BackupState {
        mode: BackupMode::Import,
        password: String::new(),
        confirm_password: String::new(),
        backup_data: String::new(),
        focus: BackupFocus::Data,
    };
    let output = render_to_string(&mut app);
    insta::assert_snapshot!("backup_import", output);
}

// @scenario: duress:View duress configuration
#[test]
fn test_snapshot_duress_not_configured() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::Duress;
    app.duress_state = DuressState {
        password_enabled: false,
        enabled: false,
        ..DuressState::default()
    };
    let output = render_to_string(&mut app);
    insta::assert_snapshot!("duress_not_configured", output);
}

// @scenario: duress:View duress enabled state
#[test]
fn test_snapshot_duress_enabled() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::Duress;
    app.duress_state = DuressState {
        password_enabled: true,
        enabled: true,
        contact_ids_input: "abc123, def456".to_string(),
        message_input: "Duress alert — contact may be under coercion".to_string(),
        include_location: true,
        alert_contact_count: 2,
        ..DuressState::default()
    };
    let output = render_to_string(&mut app);
    insta::assert_snapshot!("duress_enabled", output);
}
