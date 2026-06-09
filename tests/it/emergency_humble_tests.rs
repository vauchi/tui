// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Emergency broadcast screen — Humble-UI migration (ADR-021 / ADR-043).
//!
//! The screen renders the core `EmergencyBroadcastEngine` and forwards all
//! input through it; contact-id parsing and persistence happen in core. No
//! bespoke domain logic (parsing, rate-limiting, direct API calls) lives in
//! the TUI. Digit IDs (`c1`, `c2`) exercise the AppScreen bypass guard.

use crossterm::event::KeyCode;
use tempfile::TempDir;

use vauchi_app::ui::{AppEngine, AppScreen, Component, WorkflowEngine};
use vauchi_core::{SymmetricKey, Vauchi, VauchiConfig};

use vauchi_tui::app::App;
use vauchi_tui::handlers::handle_key;

fn emergency_app() -> (App, TempDir) {
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
    app.goto(AppScreen::EmergencyBroadcast);
    (app, temp_dir)
}

fn has_toggle(app: &App, want: &str) -> bool {
    app.app_engine
        .current_screen()
        .components
        .iter()
        .any(|c| matches!(c, Component::ToggleList { id, .. } if id == want))
}

fn has_text_input(app: &App, want: &str) -> bool {
    app.app_engine
        .current_screen()
        .components
        .iter()
        .any(|c| matches!(c, Component::TextInput { id, .. } if id == want))
}

// @scenario: emergency_broadcast :: Configure emergency broadcast
#[test]
fn configure_flow_runs_through_engine_and_persists_via_core() {
    let (mut app, _tmp) = emergency_app();

    // Renders the engine overview, not a bespoke widget.
    assert!(has_toggle(&app, "emergency_toggle"), "starts on overview");

    // Configure → contact-ids screen.
    handle_key(&mut app, KeyCode::Enter);
    assert!(has_text_input(&app, "contact_ids"), "configure → contacts");

    // Type IDs incl. digits (exercises the bypass guard), then Enter advances.
    for c in ['c', '1', ',', ' ', 'c', '2'] {
        handle_key(&mut app, KeyCode::Char(c));
    }
    assert!(
        has_text_input(&app, "contact_ids"),
        "digits stay on contacts"
    );
    handle_key(&mut app, KeyCode::Enter); // submit_contact_ids → continue
    assert!(has_text_input(&app, "message"), "continue → message");

    // Message is pre-filled (default); Enter saves → core persists.
    handle_key(&mut app, KeyCode::Enter); // submit_message → save → Complete

    let cfg = app
        .app_engine
        .vauchi()
        .load_emergency_config()
        .expect("query");
    let cfg = cfg.expect("emergency must be configured after the humble flow");
    assert_eq!(
        cfg.trusted_contact_ids,
        vec!["c1".to_string(), "c2".to_string()]
    );
    assert!(!cfg.message.is_empty(), "a default message is persisted");
}

// @internal
#[test]
fn empty_contacts_blocks_advance() {
    let (mut app, _tmp) = emergency_app();
    handle_key(&mut app, KeyCode::Enter); // configure → contacts
    // Continue with no IDs: the engine rejects, staying on the contacts screen.
    handle_key(&mut app, KeyCode::Enter);
    assert!(
        has_text_input(&app, "contact_ids"),
        "empty contacts must not advance to the message screen"
    );
}
