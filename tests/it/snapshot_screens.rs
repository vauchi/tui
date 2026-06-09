// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Per-screen visual regression tests using ratatui TestBackend + insta snapshots.
//!
//! Each test creates a Backend with a temp directory, builds an App
//! in the target screen state, renders to a fixed-size terminal buffer,
//! and snapshots the text output.

use crate::common;

use vauchi_app::ui::LockScreenEngine;
use vauchi_core::contact_card::ContactAction;
use vauchi_tui::app::{ActionMenuState, LockState, Overlay, Screen, SyncState};

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
    assert_eq!(app.active_screen(), Screen::SetupWelcome);
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
    app.goto(Screen::MyInfo);
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
    app.goto(Screen::Contacts);
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

// @internal
#[test]
fn test_snapshot_settings() {
    let (mut app, _tmp) = create_app_with_identity();
    app.goto(Screen::Settings);
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

// @internal
#[test]
fn test_snapshot_help() {
    let (mut app, _tmp) = create_app_with_identity();
    app.goto(Screen::Help);
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
    app.goto(Screen::Devices);
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
    app.goto(Screen::Recovery);
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
    app.goto(Screen::Sync);
    let output = render_to_string(&mut app);
    assert_snap!("sync_idle", "Settings", "select Sync", output);
}

// @scenario: sync_updates:Client initiates sync with relay
#[test]
fn test_snapshot_sync_connected() {
    let (mut app, _tmp) = create_app_with_identity();
    app.goto(Screen::Sync);
    // Sync screen renders from the engine's ScreenModel; sync_state is not
    // read by the renderer, so this setup is inert (output == sync_idle).
    app.sync_state = SyncState {
        is_syncing: false,
        pending_updates: 3,
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

// Backup is engine-driven (core `BackupRecoveryEngine`); bespoke-widget
// snapshots removed. Coverage via backup_humble_tests + core fixtures.

// =============================================================
// Dialogs
// =============================================================

// @scenario: contact_card_management:Add a field to contact card
#[test]
fn test_snapshot_add_field_dialog() {
    use vauchi_app::ui::FormDialogType;
    let (mut app, _tmp) = create_app_with_identity();
    app.goto_form_dialog(FormDialogType::AddField {
        available_groups: Vec::new(),
    });
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
    use vauchi_app::ui::FormDialogType;
    let (mut app, _tmp) = create_app_with_identity();
    app.goto_form_dialog(FormDialogType::EditField {
        field_id: "field_001".to_string(),
        field_label: "Mobile".to_string(),
        current_value: "+41 79 123 45 67".to_string(),
        current_note: None,
    });
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
    use vauchi_app::ui::FormDialogType;
    let (mut app, _tmp) = create_app_with_identity();
    app.goto_form_dialog(FormDialogType::EditName {
        current_name: "Alice Smith".to_string(),
    });
    let output = render_to_string(&mut app);
    assert_snap!(
        "edit_name_dialog",
        "Settings",
        "select Display Name",
        output
    );
}

// @internal
#[test]
fn test_snapshot_edit_relay_url_dialog() {
    use vauchi_app::ui::FormDialogType;
    let (mut app, _tmp) = create_app_with_identity();
    app.goto_form_dialog(FormDialogType::EditRelayUrl {
        current_url: "wss://relay.vauchi.app".to_string(),
    });
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

// @scenario: visibility_control:Configure default visibility
#[test]
fn test_snapshot_privacy() {
    let (mut app, _tmp) = create_app_with_identity();
    app.goto(Screen::Privacy);
    let output = render_to_string(&mut app);
    assert_snap!("privacy", "Settings", "select Privacy & Data", output);
}

// @scenario: contact_exchange:Generate exchange QR code
#[test]
fn test_snapshot_exchange() {
    let (mut app, _tmp) = create_app_with_identity();
    app.goto(Screen::Exchange);
    // ADR-031: QR image contains ephemeral keys — redacted by redact_dynamic_values in render_to_string
    let output = render_to_string(&mut app);
    assert_snap!("exchange", "MyInfo", "press '3' (Exchange tab)", output);
}

// Duress screens are engine-driven (core `DuressPinEngine`); the TUI renders
// their `ScreenModel` through the shared `screen_renderer`. Behavioral
// coverage lives in `duress_humble_tests.rs`; the engine's own screens are
// golden-fixture tested in core (`engine_golden_fixtures.rs`). The former
// bespoke-widget snapshots were removed with the bespoke duress handler.

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
    app.overlay = Some(Overlay::ActionMenu);
    let output = render_to_string(&mut app);
    assert_snap!(
        "action_menu",
        "ContactDetail",
        "press Enter on field",
        output
    );
}

// Emergency broadcast is engine-driven (core `EmergencyBroadcastEngine`); the
// bespoke-widget snapshot was removed with the bespoke handler. Coverage now
// via emergency_humble_tests + core engine_golden_fixtures.

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
    app.goto(Screen::Lock);
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
    use vauchi_app::ui::AppScreen;
    let (mut app, _tmp) = create_app_with_contacts(10);
    let groups = app.app_engine.vauchi().list_groups().expect("list groups");
    let group_id = groups
        .first()
        .map(|g| g.id().to_string())
        .expect("has groups");
    app.app_engine
        .navigate_to(AppScreen::GroupDetail { group_id });
    app.goto(Screen::GroupDetail);
    let output = render_to_string(&mut app);
    assert_snap!("group_detail", "Groups", "press Enter on group", output);
}

// @scenario: contacts_management:View empty group detail
#[test]
fn test_snapshot_group_detail_empty() {
    use vauchi_app::ui::AppScreen;
    let (mut app, _tmp) = create_app_with_identity();
    let group = app
        .app_engine
        .vauchi()
        .create_group("Empty Group")
        .expect("create group");
    app.app_engine.navigate_to(AppScreen::GroupDetail {
        group_id: group.id().to_string(),
    });
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

// @scenario: identity_management:Onboarding default name step (DefaultName)
#[test]
fn test_snapshot_setup_create_identity() {
    let (mut app, _tmp) = create_app_without_identity();
    app.goto(Screen::SetupCreateIdentity);
    if let Some(engine) = &mut app.onboarding_engine {
        use vauchi_app::ui::WorkflowEngine;
        // IdentityCheck → DefaultName
        let _ = engine.handle_action(vauchi_app::ui::UserAction::ActionPressed {
            action_id: "create_new".to_string(),
        });
    }
    let output = render_to_string(&mut app);
    assert_snap!(
        "setup_create_identity",
        "IdentityCheck",
        "press 'Create New'",
        output
    );
}

// @scenario: identity_management:Onboarding groups setup step (GroupsSetup)
#[test]
fn test_snapshot_setup_groups_setup() {
    let (mut app, _tmp) = create_app_without_identity();
    app.goto(Screen::SetupCreateIdentity);
    if let Some(engine) = &mut app.onboarding_engine {
        use vauchi_app::ui::WorkflowEngine;
        // IdentityCheck → DefaultName
        let _ = engine.handle_action(vauchi_app::ui::UserAction::ActionPressed {
            action_id: "create_new".to_string(),
        });
        // DefaultName: enter name then continue → GroupsSetup
        let _ = engine.handle_action(vauchi_app::ui::UserAction::TextChanged {
            component_id: "display_name".to_string(),
            value: "Alice".to_string(),
        });
        let _ = engine.handle_action(vauchi_app::ui::UserAction::ActionPressed {
            action_id: "continue".to_string(),
        });
    }
    let output = render_to_string(&mut app);
    assert_snap!(
        "setup_groups_setup",
        "DefaultName",
        "enter name + press Continue",
        output
    );
}

// @scenario: identity_management:Onboarding contact info step (ContactInfo, formerly AddFields)
#[test]
fn test_snapshot_setup_add_fields() {
    let (mut app, _tmp) = create_app_without_identity();
    app.goto(Screen::SetupAddFields);
    if let Some(engine) = &mut app.onboarding_engine {
        use vauchi_app::ui::WorkflowEngine;
        // IdentityCheck → DefaultName → GroupsSetup → ContactInfo
        let _ = engine.handle_action(vauchi_app::ui::UserAction::ActionPressed {
            action_id: "create_new".to_string(),
        });
        let _ = engine.handle_action(vauchi_app::ui::UserAction::TextChanged {
            component_id: "display_name".to_string(),
            value: "Alice".to_string(),
        });
        let _ = engine.handle_action(vauchi_app::ui::UserAction::ActionPressed {
            action_id: "continue".to_string(),
        });
        let _ = engine.handle_action(vauchi_app::ui::UserAction::ActionPressed {
            action_id: "skip".to_string(),
        });
    }
    let output = render_to_string(&mut app);
    assert_snap!("setup_add_fields", "GroupsSetup", "press Skip", output);
}

// @scenario: identity_management:Onboarding what-next step (WhatNext, formerly Ready)
#[test]
fn test_snapshot_setup_ready() {
    let (mut app, _tmp) = create_app_without_identity();
    app.goto(Screen::SetupReady);
    if let Some(engine) = &mut app.onboarding_engine {
        use vauchi_app::ui::WorkflowEngine;
        // IdentityCheck → DefaultName → GroupsSetup → ContactInfo → WhatNext
        let _ = engine.handle_action(vauchi_app::ui::UserAction::ActionPressed {
            action_id: "create_new".to_string(),
        });
        let _ = engine.handle_action(vauchi_app::ui::UserAction::TextChanged {
            component_id: "display_name".to_string(),
            value: "Alice".to_string(),
        });
        for _ in 0..3 {
            let _ = engine.handle_action(vauchi_app::ui::UserAction::ActionPressed {
                action_id: "continue".to_string(),
            });
        }
    }
    let output = render_to_string(&mut app);
    assert_snap!("setup_ready", "ContactInfo", "press Continue", output);
}

// =============================================================
// SP-12a: Duplicates / Merge / Limit
// =============================================================

// @scenario: contacts_management:View duplicate contacts
#[test]
fn test_snapshot_contact_duplicates() {
    let (mut app, _tmp) = create_app_with_contacts(10);
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
    use vauchi_app::ui::AppScreen;
    let (mut app, _tmp) = create_app_with_contacts(10);
    // Engine drives ContactMerge navigation (no local mirror state any more);
    // navigate it directly with the same fixture values the old test used.
    app.app_engine.navigate_to(AppScreen::ContactMerge {
        primary_name: "Ahmed Nikolaus".to_string(),
        primary_fields: vec![
            "+1-203-107-1013".to_string(),
            "ahmed.nikolaus@example.com".to_string(),
        ],
        secondary_name: "Ahmed N.".to_string(),
        secondary_fields: vec![
            "+1-203-107-1013".to_string(),
            "ahmed.n@example.com".to_string(),
        ],
    });
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
    app.goto(Screen::MyInfoEntryDetail);
    let output = render_to_string(&mut app);
    assert_snap!(
        "my_info_entry_detail",
        "MyInfo",
        "press Enter on entry",
        output
    );
}
