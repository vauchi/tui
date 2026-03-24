// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Per-screen visual regression tests using ratatui TestBackend + insta snapshots.
//!
//! Each test creates a Backend with a temp directory, builds an App
//! in the target screen state, renders to a fixed-size terminal buffer,
//! and snapshots the text output.

mod common;

use vauchi_app::ui::LockScreenEngine;
use vauchi_core::contact_card::ContactAction;
use vauchi_tui::app::{
    ActionMenuState, AddFieldFocus, AddFieldState, BackupFocus, BackupMode, BackupState,
    ContactLimitState, DeliveryState, DuplicateEntry, DuplicatesState, DuressState, EditFieldState,
    EditNameState, EditRelayUrlState, EmergencyState, LockState, MergeState, PrivacyState, Screen,
    SyncState, TorState,
};

use common::{
    create_app_with_contacts, create_app_with_identity, create_app_without_identity,
    render_to_string,
};

// =============================================================
// Setup / No Identity
// =============================================================

// @scenario: identity_management:Create new identity on first launch
#[test]
fn test_snapshot_setup_screen() {
    let (mut app, _tmp) = create_app_without_identity();
    assert_eq!(app.screen, Screen::SetupWelcome);
    let output = render_to_string(&mut app);
    assert_snap!("setup_screen", "app_start", "no identity detected", output);
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
    assert_snap!("home_with_fields", "app_start", "identity exists", output);
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
    assert_snap!(
        "contacts_empty",
        "MyInfo",
        "press '2' (Contacts tab)",
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
    assert_snap!(
        "settings",
        "MyInfo",
        "press '5' (More tab) → Settings",
        output
    );
}

// =============================================================
// Help
// =============================================================

#[test]
fn test_snapshot_help() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::Help;
    let output = render_to_string(&mut app);
    assert_snap!("help", "MyInfo", "press '5' (More tab) → Help", output);
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
    assert_snap!("devices", "Settings", "select Devices", output);
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
    assert_snap!("recovery", "Settings", "select Recovery", output);
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
    assert_snap!("sync_idle", "Settings", "select Sync", output);
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
    assert_snap!(
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
    assert_snap!("backup_menu", "Settings", "select Backup", output);
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
    assert_snap!("backup_export", "Backup", "select Create Backup", output);
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
    };
    let output = render_to_string(&mut app);
    assert_snap!(
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
    assert_snap!(
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
    assert_snap!(
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
    assert_snap!(
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
    assert_snap!("tor_settings", "Settings", "select Tor Privacy", output);
}

// @scenario: visibility_control:Configure default visibility
#[test]
fn test_snapshot_privacy() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::Privacy;
    app.privacy_state = PrivacyState::default();
    let output = render_to_string(&mut app);
    assert_snap!("privacy", "Settings", "select Privacy & Data", output);
}

// @scenario: contact_exchange:Generate exchange QR code
#[test]
fn test_snapshot_exchange() {
    let (mut app, _tmp) = create_app_with_identity();
    app.screen = Screen::Exchange;
    let output = render_to_string(&mut app);
    // ADR-031: QR data contains ephemeral keys, redact for deterministic snapshot
    let output = regex::Regex::new(r"V0JF\S+")
        .unwrap()
        .replace_all(&output, "[QR_DATA_REDACTED]")
        .to_string();
    assert_snap!("exchange", "MyInfo", "press '3' (Exchange tab)", output);
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
    assert_snap!("backup_import", "Backup", "select Restore Backup", output);
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
    assert_snap!(
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
    assert_snap!(
        "duress_enabled",
        "Settings",
        "select Duress PIN (configured)",
        output
    );
}

// =============================================================
// Contacts with Seeded Data
// =============================================================

// @scenario: contacts_management:View all contacts
#[test]
fn test_snapshot_contacts_with_entries() {
    let (mut app, _tmp) = create_app_with_contacts(10);
    app.goto(Screen::Contacts);
    let output = render_to_string(&mut app);
    assert_snap!(
        "contacts_with_entries",
        "MyInfo",
        "press '2' (Contacts tab)",
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
    assert_snap!(
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
    assert_snap!(
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
    assert_snap!(
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
    assert_snap!(
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
    assert_snap!(
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
    assert_snap!("contact_edit", "ContactDetail", "press 'e' (Edit)", output);
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
    assert_snap!(
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
    assert_snap!("support", "Settings", "select Support", output);
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
    assert_snap!("delivery", "Settings", "select Delivery", output);
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
    assert_snap!(
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
    assert_snap!("emergency", "Settings", "select Emergency Wipe", output);
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
    assert_snap!(
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
    assert_snap!("groups", "MyInfo", "press Enter (Group View)", output);
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
    assert_snap!("group_detail", "Groups", "press Enter on group", output);
}

// @scenario: contacts_management:View empty group detail
#[test]
fn test_snapshot_group_detail_empty() {
    let (mut app, _tmp) = create_app_with_identity();
    // Create a group with no contacts
    let group = app
        .app_engine
        .vauchi()
        .create_group("Empty Group")
        .expect("create group");
    app.groups_state.selected_group_id = Some(group.id().to_string());
    app.goto(Screen::GroupDetail);
    let output = render_to_string(&mut app);
    assert_snap!(
        "group_detail_empty",
        "Groups",
        "press Enter on empty group",
        output
    );
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
        use vauchi_app::ui::WorkflowEngine;
        // Navigate through IdentityCheck -> LinkChoice -> Welcome -> CreateIdentity
        let _ = engine.handle_action(vauchi_app::ui::UserAction::ActionPressed {
            action_id: "create_new".to_string(),
        });
        let _ = engine.handle_action(vauchi_app::ui::UserAction::ActionPressed {
            action_id: "next".to_string(),
        });
        let _ = engine.handle_action(vauchi_app::ui::UserAction::ActionPressed {
            action_id: "next".to_string(),
        });
    }
    let output = render_to_string(&mut app);
    assert_snap!(
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
        use vauchi_app::ui::WorkflowEngine;
        let _ = engine.handle_action(vauchi_app::ui::UserAction::ActionPressed {
            action_id: "create_new".to_string(),
        });
        let _ = engine.handle_action(vauchi_app::ui::UserAction::ActionPressed {
            action_id: "next".to_string(),
        });
        let _ = engine.handle_action(vauchi_app::ui::UserAction::ActionPressed {
            action_id: "next".to_string(),
        });
        let _ = engine.handle_action(vauchi_app::ui::UserAction::ActionPressed {
            action_id: "next".to_string(),
        });
    }
    let output = render_to_string(&mut app);
    assert_snap!(
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
        use vauchi_app::ui::WorkflowEngine;
        let _ = engine.handle_action(vauchi_app::ui::UserAction::ActionPressed {
            action_id: "create_new".to_string(),
        });
        for _ in 0..4 {
            let _ = engine.handle_action(vauchi_app::ui::UserAction::ActionPressed {
                action_id: "next".to_string(),
            });
        }
    }
    let output = render_to_string(&mut app);
    assert_snap!("setup_security", "SetupAddFields", "press Next", output);
}

// @scenario: identity_management:Onboarding ready step
#[test]
fn test_snapshot_setup_ready() {
    let (mut app, _tmp) = create_app_without_identity();
    app.screen = Screen::SetupReady;
    if let Some(engine) = &mut app.onboarding_engine {
        use vauchi_app::ui::WorkflowEngine;
        let _ = engine.handle_action(vauchi_app::ui::UserAction::ActionPressed {
            action_id: "create_new".to_string(),
        });
        for _ in 0..5 {
            let _ = engine.handle_action(vauchi_app::ui::UserAction::ActionPressed {
                action_id: "next".to_string(),
            });
        }
    }
    let output = render_to_string(&mut app);
    assert_snap!("setup_ready", "SetupSecurity", "press Next", output);
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
    assert_snap!(
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
    assert_snap!(
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
    assert_snap!(
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
    assert_snap!(
        "my_info_entry_detail",
        "MyInfo",
        "press Enter on entry",
        output
    );
}
