// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Workflow happy-path visual regression tests using ratatui TestBackend + insta snapshots.
//!
//! Each test exercises a multi-step user journey, rendering each step and
//! combining them into a single composite snapshot.

mod common;

use vauchi_tui::app::{AddFieldFocus, AddFieldState, BackupFocus, BackupMode, BackupState, Screen};

use common::{
    create_app_with_contacts, create_app_with_identity, create_app_without_identity,
    render_to_string, workflow_step,
};

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

    // Step 2-5: Advance through onboarding engine
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
            let _ = engine.handle_action(vauchi_core::ui::UserAction::ActionPressed {
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
    // ADR-031: QR data contains ephemeral keys, redact for deterministic snapshot
    let output = regex::Regex::new(r"V0JF\S+")
        .unwrap()
        .replace_all(&output, "[QR_DATA_REDACTED]")
        .to_string();
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
