// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Backup screen — Humble-UI migration (ADR-021 / ADR-043).
//!
//! The screen renders the core `BackupRecoveryEngine` and forwards all input
//! through it; the password capture and export/import run in core. No bespoke
//! domain logic (validation, `validate_password`, direct export/import calls)
//! lives in the TUI. A password digit exercises the AppScreen bypass guard.

use crossterm::event::KeyCode;
use tempfile::TempDir;

use vauchi_app::ui::{AppEngine, AppScreen, Component, WorkflowEngine};
use vauchi_core::{SymmetricKey, Vauchi, VauchiConfig};

use vauchi_tui::app::App;
use vauchi_tui::handlers::handle_key;

fn backup_app() -> (App, TempDir) {
    let temp_dir = TempDir::new().expect("temp dir");
    let key = SymmetricKey::generate();
    let config =
        VauchiConfig::with_storage_path(temp_dir.path().join("vauchi.db")).with_storage_key(key);
    let vauchi: Vauchi = Vauchi::new(config).expect("vauchi");
    let mut app_engine = AppEngine::new(vauchi);
    app_engine
        .vauchi_mut()
        .create_identity("Test User")
        .expect("identity");
    let mut app = App::new(
        app_engine,
        "wss://relay.vauchi.app".to_string(),
        temp_dir.path().to_path_buf(),
    );
    app.goto(AppScreen::Backup);
    (app, temp_dir)
}

fn has_text_input(app: &App, want: &str) -> bool {
    app.app_engine
        .current_screen()
        .components
        .iter()
        .any(|c| matches!(c, Component::TextInput { id, .. } if id == want))
}

fn has_toggle(app: &App, want: &str) -> bool {
    app.app_engine
        .current_screen()
        .components
        .iter()
        .any(|c| matches!(c, Component::ToggleList { id, .. } if id == want))
}

fn type_str(app: &mut App, s: &str) {
    for c in s.chars() {
        handle_key(app, KeyCode::Char(c));
    }
}

// @scenario: identity_management :: Create encrypted identity backup
#[test]
fn create_flow_runs_through_engine_and_exports_via_core() {
    let (mut app, _tmp) = backup_app();

    // Starts on the engine's choose screen.
    assert!(
        has_toggle(&app, "backup_level"),
        "starts on the choose screen"
    );

    // Primary action (Create) → password screen.
    handle_key(&mut app, KeyCode::Enter);
    assert!(has_text_input(&app, "password"), "create → password");

    // Type a password incl. a digit (exercises the bypass guard); Enter advances.
    type_str(&mut app, "pw1secret");
    handle_key(&mut app, KeyCode::Enter); // submit_password → continue
    assert!(has_text_input(&app, "confirm_password"), "→ confirm");

    // Confirm the same password → Enter saves → core exports.
    type_str(&mut app, "pw1secret");
    handle_key(&mut app, KeyCode::Enter); // submit_confirm_password → processing → export

    // The engine produced the backup; the TUI surfaced it (clipboard + status).
    let status = app.status_message.clone().unwrap_or_default();
    assert!(
        status.starts_with("Backup created"),
        "export must surface a 'Backup created' status, got {status:?}"
    );
}

// @internal
#[test]
fn create_password_mismatch_does_not_export() {
    let (mut app, _tmp) = backup_app();
    handle_key(&mut app, KeyCode::Enter); // create → password
    type_str(&mut app, "pw1secret");
    handle_key(&mut app, KeyCode::Enter); // → confirm
    // Mismatched confirmation.
    type_str(&mut app, "different9");
    handle_key(&mut app, KeyCode::Enter); // engine rejects, stays on confirm
    assert!(
        has_text_input(&app, "confirm_password"),
        "mismatch must stay on the confirm screen"
    );
}

// @scenario: identity_management :: Restore identity from backup
#[test]
fn restore_flow_offers_paste_field() {
    let (mut app, _tmp) = backup_app();
    assert!(
        has_toggle(&app, "backup_level"),
        "starts on the choose screen"
    );

    // Restore is the second (secondary) action — reachable via its shortcut
    // key. Drive it through the engine and confirm the paste field appears.
    let restore = app
        .app_engine
        .current_screen()
        .actions
        .iter()
        .find(|a| a.id == "restore")
        .cloned()
        .expect("restore action present");
    let result = app
        .app_engine
        .handle_action(vauchi_app::ui::UserAction::ActionPressed {
            action_id: restore.id,
        });
    vauchi_tui::handlers::action_result::handle_action_result(&mut app, result);
    assert!(
        has_text_input(&app, "backup_data"),
        "restore password screen must offer the backup_data paste field"
    );
}
