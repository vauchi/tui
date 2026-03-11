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

use vauchi_core::contact_card::ContactAction;
use vauchi_core::ui::AppEngine;
use vauchi_core::ui::{LockScreenEngine, OnboardingEngine};
use vauchi_core::{
    Contact, ContactCard, ContactField, FieldType, MockTransport, SymmetricKey, Vauchi,
    VauchiConfig,
};
use vauchi_tui::app::{
    ActionMenuState, AddFieldFocus, AddFieldState, App, BackupFocus, BackupMode, BackupState,
    ContactLimitState, DeliveryState, DuplicateEntry, DuplicatesState, DuressState, EditFieldState,
    EditNameState, EditRelayUrlState, EmergencyState, GroupsState, LockState, MergeState,
    OnboardingState, PrivacyState, Screen, SyncState, TorState,
};

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

fn create_app_engine(data_dir: &std::path::Path) -> AppEngine<MockTransport> {
    let key = SymmetricKey::generate();
    let config = VauchiConfig {
        storage_path: data_dir.join("vauchi.db"),
        storage_key: Some(key),
        ..Default::default()
    };
    let vauchi: Vauchi<MockTransport> = Vauchi::new(config).expect("vauchi");
    AppEngine::new(vauchi)
}

/// Create a test app with an identity and some card fields.
fn create_app_with_identity() -> (App, TempDir) {
    ensure_locales_loaded();
    let temp_dir = TempDir::new().expect("temp dir");
    let mut app_engine = create_app_engine(temp_dir.path());
    app_engine
        .vauchi_mut()
        .create_identity("Alice Smith")
        .expect("create identity");
    app_engine
        .vauchi()
        .add_own_field(ContactField::new(
            FieldType::Email,
            "Work",
            "alice@company.com",
        ))
        .expect("add email");
    app_engine
        .vauchi()
        .add_own_field(ContactField::new(
            FieldType::Phone,
            "Mobile",
            "+41 79 123 45 67",
        ))
        .expect("add phone");
    app_engine
        .vauchi()
        .add_own_field(ContactField::new(
            FieldType::Website,
            "Blog",
            "https://alice.example.com",
        ))
        .expect("add website");
    let app = App::new(
        app_engine,
        "wss://relay.vauchi.app".to_string(),
        temp_dir.path().to_path_buf(),
    );
    (app, temp_dir)
}

/// Create a test app without an identity (setup state).
fn create_app_without_identity() -> (App, TempDir) {
    ensure_locales_loaded();
    let temp_dir = TempDir::new().expect("temp dir");
    let app_engine = create_app_engine(temp_dir.path());
    let app = App::new(
        app_engine,
        "wss://relay.vauchi.app".to_string(),
        temp_dir.path().to_path_buf(),
    );
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
    // Replace public IDs: 16+ hex chars (optionally followed by "...")
    let re_public_id = regex::Regex::new(r"[0-9a-f]{16,}(\.\.\.)?").unwrap();
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

/// Assert a snapshot with navigation context metadata.
///
/// `from`: the previous screen, `trigger`: the user action or event that caused this screen.
macro_rules! snap {
    ($name:expr, $from:expr, $trigger:expr, $output:expr) => {
        insta::with_settings!({
            description => concat!($from, " → ", $trigger),
        }, {
            insta::assert_snapshot!($name, $output);
        });
    };
}

/// Render a step in a workflow snapshot with a header separator.
fn workflow_step(step: usize, label: &str, app: &mut App) -> String {
    let output = render_to_string(app);
    format!(
        "═══ Step {} ─ {} {}\n{}",
        step,
        label,
        "═".repeat(60usize.saturating_sub(label.len() + 12)),
        output
    )
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
    snap!("setup_screen", "app_start", "no identity detected", output);
}

// =============================================================
// Home Screen
// =============================================================

// @scenario: contact_card_management:View contact card fields
#[test]
fn test_snapshot_home_with_fields() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::MyInfo;
    let output = render_to_string(&mut app);
    snap!("home_with_fields", "app_start", "identity exists", output);
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
    snap!(
        "contacts_empty",
        "MyInfo",
        "press '3' (Contacts tab)",
        output
    );
}

// =============================================================
// Settings
// =============================================================

#[test]
fn test_snapshot_settings() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::Settings;
    let output = render_to_string(&mut app);
    snap!("settings", "MyInfo", "press '4' (Settings tab)", output);
}

// =============================================================
// Help
// =============================================================

#[test]
fn test_snapshot_help() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::Help;
    let output = render_to_string(&mut app);
    snap!("help", "MyInfo", "press '5' (Help tab)", output);
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
    snap!("devices", "Settings", "select Devices", output);
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
    snap!("recovery", "Settings", "select Recovery", output);
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
    snap!("sync_idle", "Settings", "select Sync", output);
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
    snap!(
        "sync_connected",
        "Sync",
        "sync completed successfully",
        output
    );
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
    snap!("backup_menu", "Settings", "select Backup", output);
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
    snap!("backup_export", "Backup", "select Create Backup", output);
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
    snap!(
        "add_field_dialog",
        "MyInfo",
        "press 'a' (Add Entry)",
        output
    );
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
    snap!(
        "edit_field_dialog",
        "MyInfo",
        "press Enter on field",
        output
    );
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
    snap!(
        "edit_name_dialog",
        "Settings",
        "select Display Name",
        output
    );
}

#[test]
fn test_snapshot_edit_relay_url_dialog() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::EditRelayUrl;
    app.edit_relay_url_state = EditRelayUrlState {
        new_url: "wss://relay.vauchi.app".to_string(),
    };
    let output = render_to_string(&mut app);
    snap!(
        "edit_relay_url_dialog",
        "Settings",
        "select Relay URL",
        output
    );
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
    snap!("tor_settings", "Settings", "select Tor Privacy", output);
}

// @scenario: visibility_control:Configure default visibility
#[test]
fn test_snapshot_privacy() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::Privacy;
    app.privacy_state = PrivacyState::default();
    let output = render_to_string(&mut app);
    snap!("privacy", "Settings", "select Privacy & Data", output);
}

// @scenario: contact_exchange:Generate exchange QR code
#[test]
fn test_snapshot_exchange() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::Exchange;
    let output = render_to_string(&mut app);
    snap!("exchange", "MyInfo", "press '1' (Exchange tab)", output);
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
    snap!("backup_import", "Backup", "select Restore Backup", output);
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
    snap!(
        "duress_not_configured",
        "Settings",
        "select Duress PIN",
        output
    );
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
    snap!(
        "duress_enabled",
        "Settings",
        "select Duress PIN (configured)",
        output
    );
}

// =============================================================
// Contacts with Seeded Data
// =============================================================

/// Seed N fake contacts into a Vauchi instance with groups.
fn seed_contacts(vauchi: &Vauchi<MockTransport>, count: usize) {
    // Create groups
    let family = vauchi.create_group("Family").expect("create group");
    let friends = vauchi.create_group("Friends").expect("create group");
    let work = vauchi.create_group("Work").expect("create group");
    let group_ids = [
        family.id().to_string(),
        friends.id().to_string(),
        work.id().to_string(),
    ];

    let names = [
        "Abigale Schroeder",
        "Ahmed Nikolaus",
        "Andreanne Doyle",
        "Andres Raynor",
        "Augusta Heller",
        "Brady Koss",
        "Carmen Gutierrez",
        "Diana Patel",
        "Eduardo Reyes",
        "Fatima Okonkwo",
        "Gerhard Mueller",
        "Hiroshi Tanaka",
        "Ingrid Svensson",
        "Jorge Castillo",
        "Keiko Watanabe",
    ];

    for (i, name) in names.iter().take(count).enumerate() {
        let area = 200 + (i * 3) % 800;
        let num1 = 100 + (i * 7) % 900;
        let num2 = 1000 + (i * 13) % 9000;
        let phone = format!("+1-{}-{}-{}", area, num1, num2);
        let email = format!("{}@example.com", name.to_lowercase().replace(' ', "."));

        let mut card = ContactCard::new(name);
        card.add_field(ContactField::new(FieldType::Phone, "Mobile", &phone))
            .expect("add phone");
        card.add_field(ContactField::new(FieldType::Email, "Email", &email))
            .expect("add email");

        // Address for ~40%
        if i % 5 < 2 {
            card.add_field(ContactField::new(
                FieldType::Address,
                "Address",
                &format!("{} Main St, Springfield", 100 + i * 10),
            ))
            .expect("add address");
        }

        let mut pk = [0u8; 32];
        pk[0] = (i >> 8) as u8;
        pk[1] = (i & 0xFF) as u8;
        for (j, byte) in pk[2..].iter_mut().enumerate() {
            *byte = ((i * 7 + j) & 0xFF) as u8;
        }

        let contact = Contact::from_exchange(pk, card, SymmetricKey::generate());
        let cid = contact.id().to_string();
        vauchi.add_contact(contact).expect("add contact");

        // Assign to group: ~30% Family, ~40% Friends, ~30% Work
        let gid = match i % 10 {
            0..=2 => &group_ids[0],
            3..=6 => &group_ids[1],
            _ => &group_ids[2],
        };
        vauchi
            .add_contact_to_group(gid, &cid)
            .expect("add to group");
    }
}

/// Create a test app with identity and seeded contacts.
fn create_app_with_contacts(count: usize) -> (App, TempDir) {
    ensure_locales_loaded();
    let temp_dir = TempDir::new().expect("temp dir");
    let mut app_engine = create_app_engine(temp_dir.path());
    app_engine
        .vauchi_mut()
        .create_identity("Alice Smith")
        .expect("create identity");
    app_engine
        .vauchi()
        .add_own_field(ContactField::new(
            FieldType::Email,
            "Work",
            "alice@company.com",
        ))
        .expect("add email");

    seed_contacts(app_engine.vauchi(), count);

    let app = App::new(
        app_engine,
        "wss://relay.vauchi.app".to_string(),
        temp_dir.path().to_path_buf(),
    );
    (app, temp_dir)
}

// @scenario: contacts_management:View all contacts
#[test]
fn test_snapshot_contacts_with_entries() {
    let (mut app, _tmp) = create_app_with_contacts(10);
    app.goto(Screen::Contacts);
    let output = render_to_string(&mut app);
    snap!(
        "contacts_with_entries",
        "MyInfo",
        "press '3' (Contacts tab)",
        output
    );
}

// @scenario: contacts_management:Search contacts by name and field
#[test]
fn test_snapshot_contacts_search_by_name() {
    let (mut app, _tmp) = create_app_with_contacts(10);
    app.goto(Screen::Contacts);
    app.contact_search_query = "Ahmed".to_string();
    vauchi_tui::helpers::dispatch_search(&mut app);
    let output = render_to_string(&mut app);
    snap!(
        "contacts_search_by_name",
        "Contacts",
        "type search 'Ahmed'",
        output
    );
}

// @scenario: contacts_management:Search contacts by field value
#[test]
fn test_snapshot_contacts_search_by_email() {
    let (mut app, _tmp) = create_app_with_contacts(10);
    app.goto(Screen::Contacts);
    app.contact_search_query = "carmen".to_string();
    vauchi_tui::helpers::dispatch_search(&mut app);
    let output = render_to_string(&mut app);
    snap!(
        "contacts_search_by_email",
        "Contacts",
        "type search 'carmen'",
        output
    );
}

// @scenario: contacts_management:Filter contacts by group
#[test]
fn test_snapshot_contacts_group_filter() {
    let (mut app, _tmp) = create_app_with_contacts(10);
    app.goto(Screen::Contacts);
    // Cycle to first group (Family)
    vauchi_tui::helpers::cycle_group_filter(&mut app);
    let output = render_to_string(&mut app);
    snap!(
        "contacts_group_filter_family",
        "Contacts",
        "press 'g' (cycle group filter)",
        output
    );
}

// @scenario: contacts_management:Scroll through contact list
#[test]
fn test_snapshot_contacts_scrolled() {
    let (mut app, _tmp) = create_app_with_contacts(15);
    app.goto(Screen::Contacts);
    app.selected_contact = 5;
    let output = render_to_string(&mut app);
    snap!(
        "contacts_scrolled",
        "Contacts",
        "scroll down to index 5",
        output
    );
}

// =============================================================
// Contact Detail / Edit / Visibility
// =============================================================

// @scenario: contacts_management:View contact detail
#[test]
fn test_snapshot_contact_detail() {
    let (mut app, _tmp) = create_app_with_contacts(5);
    app.goto(Screen::Contacts);
    // Select first contact and navigate to detail
    let contact_id = app
        .app_engine
        .vauchi()
        .list_contacts()
        .expect("list contacts")
        .first()
        .map(|c| c.id().to_string())
        .expect("has contacts");
    app.selected_contact_id = Some(contact_id);
    app.goto(Screen::ContactDetail);
    let output = render_to_string(&mut app);
    snap!(
        "contact_detail",
        "Contacts",
        "press Enter on contact",
        output
    );
}

// @scenario: contacts_management:Edit contact fields
#[test]
fn test_snapshot_contact_edit() {
    let (mut app, _tmp) = create_app_with_contacts(5);
    app.goto(Screen::Contacts);
    let contact_id = app
        .app_engine
        .vauchi()
        .list_contacts()
        .expect("list contacts")
        .first()
        .map(|c| c.id().to_string())
        .expect("has contacts");
    app.selected_contact_id = Some(contact_id);
    app.goto(Screen::ContactEdit);
    let output = render_to_string(&mut app);
    snap!("contact_edit", "ContactDetail", "press 'e' (Edit)", output);
}

// @scenario: visibility_control:View contact visibility settings
#[test]
fn test_snapshot_contact_visibility() {
    let (mut app, _tmp) = create_app_with_contacts(5);
    app.goto(Screen::Contacts);
    let contact_id = app
        .app_engine
        .vauchi()
        .list_contacts()
        .expect("list contacts")
        .first()
        .map(|c| c.id().to_string())
        .expect("has contacts");
    app.selected_contact_id = Some(contact_id);
    app.goto(Screen::ContactVisibility);
    let output = render_to_string(&mut app);
    snap!(
        "contact_visibility",
        "ContactDetail",
        "select Visibility",
        output
    );
}

// =============================================================
// Support
// =============================================================

// @scenario: settings:View support screen
#[test]
fn test_snapshot_support() {
    let (mut app, _tmp) = create_app_with_identity();
    app.goto(Screen::Support);
    let output = render_to_string(&mut app);
    snap!("support", "Settings", "select Support", output);
}

// =============================================================
// Delivery
// =============================================================

// @scenario: delivery:View delivery status
#[test]
fn test_snapshot_delivery() {
    let (mut app, _tmp) = create_app_with_identity();
    app.goto(Screen::Delivery);
    app.delivery_state = DeliveryState {
        queued: 2,
        sent: 5,
        stored: 3,
        delivered: 10,
        failed: 1,
        pending_retries: 1,
        offline_queue_depth: 0,
        last_result: None,
    };
    let output = render_to_string(&mut app);
    snap!("delivery", "Settings", "select Delivery", output);
}

// =============================================================
// Action Menu
// =============================================================

// @scenario: contacts_management:Open action menu on contact field
#[test]
fn test_snapshot_action_menu() {
    let (mut app, _tmp) = create_app_with_contacts(5);
    let contact_id = app
        .app_engine
        .vauchi()
        .list_contacts()
        .expect("list contacts")
        .first()
        .map(|c| c.id().to_string())
        .expect("has contacts");
    app.selected_contact_id = Some(contact_id);
    app.goto(Screen::ContactDetail);
    app.action_menu_state = ActionMenuState {
        actions: vec![
            (
                "Call".to_string(),
                ContactAction::Call("+1-200-100-1000".to_string()),
            ),
            (
                "Send SMS".to_string(),
                ContactAction::SendSms("+1-200-100-1000".to_string()),
            ),
            (
                "Copy".to_string(),
                ContactAction::OpenUrl("+1-200-100-1000".to_string()),
            ),
        ],
        selected: 0,
    };
    app.screen = Screen::ActionMenu;
    let output = render_to_string(&mut app);
    snap!(
        "action_menu",
        "ContactDetail",
        "press Enter on field",
        output
    );
}

// =============================================================
// Emergency
// =============================================================

// @scenario: emergency:View emergency broadcast screen
#[test]
fn test_snapshot_emergency() {
    let (mut app, _tmp) = create_app_with_identity();
    app.goto(Screen::Emergency);
    app.emergency_state = EmergencyState {
        configured: false,
        contact_ids_input: String::new(),
        message_input: String::new(),
        include_location: false,
        trusted_count: 0,
        ..EmergencyState::default()
    };
    let output = render_to_string(&mut app);
    snap!("emergency", "Settings", "select Emergency Wipe", output);
}

// =============================================================
// Lock Screen
// =============================================================

// @scenario: identity_management:Lock screen PIN entry
#[test]
fn test_snapshot_lock_screen() {
    let (mut app, _tmp) = create_app_with_identity();
    app.lock_engine = Some(LockScreenEngine::new(5));
    app.lock_state = LockState {
        pin_input: String::new(),
        attempts: 0,
        error: false,
    };
    app.screen = Screen::Lock;
    let output = render_to_string(&mut app);
    snap!(
        "lock_screen",
        "app_start",
        "app password configured",
        output
    );
}

// =============================================================
// Groups
// =============================================================

// @scenario: contacts_management:View contact groups
#[test]
fn test_snapshot_groups() {
    let (mut app, _tmp) = create_app_with_contacts(10);
    app.goto(Screen::Groups);
    let output = render_to_string(&mut app);
    snap!("groups", "MyInfo", "press Enter (Group View)", output);
}

// @scenario: contacts_management:View group detail
#[test]
fn test_snapshot_group_detail() {
    let (mut app, _tmp) = create_app_with_contacts(10);
    // Get first group ID
    let groups = app.app_engine.vauchi().list_groups().expect("list groups");
    let group_id = groups
        .first()
        .map(|g| g.id().to_string())
        .expect("has groups");
    app.groups_state.selected_group_id = Some(group_id);
    app.goto(Screen::GroupDetail);
    let output = render_to_string(&mut app);
    snap!("group_detail", "Groups", "press Enter on group", output);
}

// =============================================================
// Onboarding Wizard
// =============================================================

// @scenario: identity_management:Onboarding create identity step
#[test]
fn test_snapshot_setup_create_identity() {
    let (mut app, _tmp) = create_app_without_identity();
    // Advance onboarding to CreateIdentity step
    app.screen = Screen::SetupCreateIdentity;
    if let Some(engine) = &mut app.onboarding_engine {
        use vauchi_core::ui::WorkflowEngine;
        // Navigate through IdentityCheck → LinkChoice → Welcome → CreateIdentity
        engine.handle_action(vauchi_core::ui::UserAction::ActionPressed {
            action_id: "create_new".to_string(),
        });
        engine.handle_action(vauchi_core::ui::UserAction::ActionPressed {
            action_id: "next".to_string(),
        });
        engine.handle_action(vauchi_core::ui::UserAction::ActionPressed {
            action_id: "next".to_string(),
        });
    }
    let output = render_to_string(&mut app);
    snap!(
        "setup_create_identity",
        "SetupWelcome",
        "press Enter (Get Started)",
        output
    );
}

// @scenario: identity_management:Onboarding add fields step
#[test]
fn test_snapshot_setup_add_fields() {
    let (mut app, _tmp) = create_app_without_identity();
    app.screen = Screen::SetupAddFields;
    if let Some(engine) = &mut app.onboarding_engine {
        use vauchi_core::ui::WorkflowEngine;
        engine.handle_action(vauchi_core::ui::UserAction::ActionPressed {
            action_id: "create_new".to_string(),
        });
        engine.handle_action(vauchi_core::ui::UserAction::ActionPressed {
            action_id: "next".to_string(),
        });
        engine.handle_action(vauchi_core::ui::UserAction::ActionPressed {
            action_id: "next".to_string(),
        });
        engine.handle_action(vauchi_core::ui::UserAction::ActionPressed {
            action_id: "next".to_string(),
        });
    }
    let output = render_to_string(&mut app);
    snap!(
        "setup_add_fields",
        "SetupCreateIdentity",
        "enter name + press Next",
        output
    );
}

// @scenario: identity_management:Onboarding security step
#[test]
fn test_snapshot_setup_security() {
    let (mut app, _tmp) = create_app_without_identity();
    app.screen = Screen::SetupSecurity;
    if let Some(engine) = &mut app.onboarding_engine {
        use vauchi_core::ui::WorkflowEngine;
        engine.handle_action(vauchi_core::ui::UserAction::ActionPressed {
            action_id: "create_new".to_string(),
        });
        for _ in 0..4 {
            engine.handle_action(vauchi_core::ui::UserAction::ActionPressed {
                action_id: "next".to_string(),
            });
        }
    }
    let output = render_to_string(&mut app);
    snap!("setup_security", "SetupAddFields", "press Next", output);
}

// @scenario: identity_management:Onboarding ready step
#[test]
fn test_snapshot_setup_ready() {
    let (mut app, _tmp) = create_app_without_identity();
    app.screen = Screen::SetupReady;
    if let Some(engine) = &mut app.onboarding_engine {
        use vauchi_core::ui::WorkflowEngine;
        engine.handle_action(vauchi_core::ui::UserAction::ActionPressed {
            action_id: "create_new".to_string(),
        });
        for _ in 0..5 {
            engine.handle_action(vauchi_core::ui::UserAction::ActionPressed {
                action_id: "next".to_string(),
            });
        }
    }
    let output = render_to_string(&mut app);
    snap!("setup_ready", "SetupSecurity", "press Next", output);
}

// =============================================================
// SP-12a: Duplicates / Merge / Limit
// =============================================================

// @scenario: contacts_management:View duplicate contacts
#[test]
fn test_snapshot_contact_duplicates() {
    let (mut app, _tmp) = create_app_with_contacts(10);
    app.duplicates_state = DuplicatesState {
        pairs: vec![
            DuplicateEntry {
                id1: "id1".to_string(),
                name1: "Ahmed Nikolaus".to_string(),
                id2: "id2".to_string(),
                name2: "Ahmed N.".to_string(),
                similarity: 0.92,
            },
            DuplicateEntry {
                id1: "id3".to_string(),
                name1: "Brady Koss".to_string(),
                id2: "id4".to_string(),
                name2: "Brady K.".to_string(),
                similarity: 0.85,
            },
        ],
        selected: 0,
    };
    app.goto(Screen::ContactDuplicates);
    let output = render_to_string(&mut app);
    snap!(
        "contact_duplicates",
        "Contacts",
        "duplicates detected",
        output
    );
}

// @scenario: contacts_management:Merge duplicate contacts
#[test]
fn test_snapshot_contact_merge() {
    let (mut app, _tmp) = create_app_with_contacts(10);
    app.merge_state = MergeState {
        primary_id: "id1".to_string(),
        primary_name: "Ahmed Nikolaus".to_string(),
        primary_fields: vec![
            "+1-203-107-1013".to_string(),
            "ahmed.nikolaus@example.com".to_string(),
        ],
        secondary_id: "id2".to_string(),
        secondary_name: "Ahmed N.".to_string(),
        secondary_fields: vec![
            "+1-203-107-1013".to_string(),
            "ahmed.n@example.com".to_string(),
        ],
    };
    app.goto(Screen::ContactMerge);
    let output = render_to_string(&mut app);
    snap!(
        "contact_merge",
        "ContactDuplicates",
        "select duplicate pair",
        output
    );
}

// @scenario: contacts_management:Configure contact limit
#[test]
fn test_snapshot_contact_limit() {
    let (mut app, _tmp) = create_app_with_contacts(10);
    app.contact_limit_state = ContactLimitState {
        current_limit: 150,
        current_count: 10,
        limit_input: String::new(),
        editing: false,
    };
    app.goto(Screen::ContactLimit);
    let output = render_to_string(&mut app);
    snap!(
        "contact_limit",
        "Contacts",
        "approach contact limit",
        output
    );
}

// =============================================================
// MyInfo Entry Detail
// =============================================================

// @scenario: contact_card_management:View MyInfo entry detail
#[test]
fn test_snapshot_my_info_entry_detail() {
    let (mut app, _tmp) = create_app_with_identity();
    app.goto(Screen::MyInfo);
    // Select first field and navigate to detail
    app.selected_field = 0;
    app.screen = Screen::MyInfoEntryDetail;
    let output = render_to_string(&mut app);
    snap!(
        "my_info_entry_detail",
        "MyInfo",
        "press Enter on entry",
        output
    );
}

// =============================================================
// Workflow Happy-Path Snapshots
// =============================================================

// @workflow: onboarding — first launch to ready state
#[test]
fn test_workflow_onboarding() {
    let (mut app, _tmp) = create_app_without_identity();
    let mut steps = Vec::new();

    // Step 1: Welcome screen (auto-shown when no identity)
    steps.push(workflow_step(
        1,
        "SetupWelcome — no identity detected",
        &mut app,
    ));

    // Step 2–5: Advance through onboarding engine
    let step_labels = [
        "IdentityCheck → press 'Create New'",
        "LinkChoice → press 'Get Started'",
        "Welcome → press 'Next'",
        "CreateIdentity → enter name + press 'Next'",
    ];
    let actions = ["create_new", "next", "next", "next"];
    for (i, (label, action_id)) in step_labels.iter().zip(actions.iter()).enumerate() {
        use vauchi_core::ui::WorkflowEngine;
        if let Some(engine) = &mut app.onboarding_engine {
            engine.handle_action(vauchi_core::ui::UserAction::ActionPressed {
                action_id: action_id.to_string(),
            });
        }
        steps.push(workflow_step(i + 2, label, &mut app));
    }

    let output = steps.join("\n\n");
    insta::with_settings!({
        description => "Onboarding happy path: first launch → identity creation → ready",
    }, {
        insta::assert_snapshot!("workflow_onboarding", output);
    });
}

// @workflow: contact exchange — show QR, view contacts
#[test]
fn test_workflow_exchange() {
    let (mut app, _tmp) = create_app_with_identity();
    let mut steps = Vec::new();

    // Step 1: MyInfo (home screen)
    steps.push(workflow_step(1, "MyInfo — home screen", &mut app));

    // Step 2: Exchange screen
    app.goto(Screen::Exchange);
    steps.push(workflow_step(
        2,
        "Exchange — press '1' (Exchange tab)",
        &mut app,
    ));

    // Step 3: Back to contacts (after exchange completes)
    app.goto(Screen::Contacts);
    steps.push(workflow_step(
        3,
        "Contacts — press '3' (view exchanged contacts)",
        &mut app,
    ));

    let output = steps.join("\n\n");
    insta::with_settings!({
        description => "Exchange happy path: MyInfo → Exchange QR → Contacts",
    }, {
        insta::assert_snapshot!("workflow_exchange", output);
    });
}

// @workflow: add field — MyInfo → AddField → back to MyInfo
#[test]
fn test_workflow_add_field() {
    let (mut app, _tmp) = create_app_with_identity();
    let mut steps = Vec::new();

    // Step 1: MyInfo
    app.screen = Screen::MyInfo;
    steps.push(workflow_step(1, "MyInfo — view card fields", &mut app));

    // Step 2: Add field dialog
    app.screen = Screen::AddField;
    app.add_field_state = AddFieldState {
        field_type_index: 0,
        label: String::new(),
        value: String::new(),
        focus: AddFieldFocus::Type,
        ..Default::default()
    };
    steps.push(workflow_step(
        2,
        "AddField — press 'a' (Add Entry)",
        &mut app,
    ));

    // Step 3: Back to MyInfo (field added)
    app.screen = Screen::MyInfo;
    app.add_field_state = AddFieldState::default();
    steps.push(workflow_step(
        3,
        "MyInfo — field saved, return to card",
        &mut app,
    ));

    let output = steps.join("\n\n");
    insta::with_settings!({
        description => "Add field happy path: MyInfo → AddField → MyInfo",
    }, {
        insta::assert_snapshot!("workflow_add_field", output);
    });
}

// @workflow: contact detail — Contacts → Detail → Edit → back
#[test]
fn test_workflow_contact_detail() {
    let (mut app, _tmp) = create_app_with_contacts(5);
    let mut steps = Vec::new();

    // Step 1: Contacts list
    app.goto(Screen::Contacts);
    steps.push(workflow_step(1, "Contacts — view contact list", &mut app));

    // Step 2: Contact detail
    let contact_id = app
        .app_engine
        .vauchi()
        .list_contacts()
        .expect("list contacts")
        .first()
        .map(|c| c.id().to_string())
        .expect("has contacts");
    app.selected_contact_id = Some(contact_id);
    app.goto(Screen::ContactDetail);
    steps.push(workflow_step(
        2,
        "ContactDetail — press Enter on contact",
        &mut app,
    ));

    // Step 3: Contact edit
    app.goto(Screen::ContactEdit);
    steps.push(workflow_step(3, "ContactEdit — press 'e' (Edit)", &mut app));

    // Step 4: Back to detail
    app.goto(Screen::ContactDetail);
    steps.push(workflow_step(
        4,
        "ContactDetail — press Esc (back from edit)",
        &mut app,
    ));

    let output = steps.join("\n\n");
    insta::with_settings!({
        description => "Contact detail happy path: Contacts → Detail → Edit → Detail",
    }, {
        insta::assert_snapshot!("workflow_contact_detail", output);
    });
}

// @workflow: settings navigation — Settings → sub-screens
#[test]
fn test_workflow_settings() {
    let (mut app, _tmp) = create_app_with_identity();
    let mut steps = Vec::new();

    // Step 1: Settings
    app.goto(Screen::Settings);
    steps.push(workflow_step(
        1,
        "Settings — press '4' (Settings tab)",
        &mut app,
    ));

    // Step 2: Privacy
    app.goto(Screen::Privacy);
    steps.push(workflow_step(
        2,
        "Privacy — select Privacy & Data",
        &mut app,
    ));

    // Step 3: Back to Settings
    app.goto(Screen::Settings);
    steps.push(workflow_step(3, "Settings — press Esc (back)", &mut app));

    // Step 4: Backup
    app.goto(Screen::Backup);
    steps.push(workflow_step(4, "Backup — select Backup", &mut app));

    // Step 5: Back to Settings
    app.goto(Screen::Settings);
    steps.push(workflow_step(5, "Settings — press Esc (back)", &mut app));

    // Step 6: Tor
    app.goto(Screen::TorSettings);
    steps.push(workflow_step(
        6,
        "TorSettings — select Tor Privacy",
        &mut app,
    ));

    let output = steps.join("\n\n");
    insta::with_settings!({
        description => "Settings navigation: Settings → Privacy → Backup → Tor",
    }, {
        insta::assert_snapshot!("workflow_settings", output);
    });
}

// @workflow: backup flow — Settings → Backup → Export
#[test]
fn test_workflow_backup() {
    let (mut app, _tmp) = create_app_with_identity();
    let mut steps = Vec::new();

    // Step 1: Settings
    app.goto(Screen::Settings);
    steps.push(workflow_step(
        1,
        "Settings — press '4' (Settings tab)",
        &mut app,
    ));

    // Step 2: Backup menu
    app.goto(Screen::Backup);
    steps.push(workflow_step(2, "Backup — select Backup", &mut app));

    // Step 3: Export form
    app.backup_state = BackupState {
        mode: BackupMode::Export,
        password: String::new(),
        confirm_password: String::new(),
        backup_data: String::new(),
        focus: BackupFocus::Password,
    };
    steps.push(workflow_step(
        3,
        "BackupExport — select Create Backup",
        &mut app,
    ));

    let output = steps.join("\n\n");
    insta::with_settings!({
        description => "Backup happy path: Settings → Backup menu → Export form",
    }, {
        insta::assert_snapshot!("workflow_backup", output);
    });
}
