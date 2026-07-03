// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Duress PIN screen — Humble-UI migration (ADR-021 / ADR-043).
//!
//! The duress screen renders the core `DuressPinEngine` and forwards all
//! input through it; persistence happens in core when the engine completes.
//! No bespoke domain logic (PIN parsing, `DuressSettings` construction,
//! direct `save_duress_settings`) lives in the TUI.
//!
//! Regression target: the bespoke handler hijacked digit keys via
//! `InputMode::Editing`. On the engine path the global tab-switch keys
//! (`1`–`5`) would navigate away during PIN entry, and the sync-chrome
//! `Indicator` injected at index 0 would capture initial focus and swallow
//! digits. Both are fixed; these tests drive a 6-digit PIN that includes
//! `1`–`5` to prove digits reach the engine.
//!
//! The AppEngine stamps a canonical `screen_id` ("duress_pin") over every
//! duress sub-state (`apply_screen_id_metadata`), so steps are identified by
//! the distinguishing component each sub-screen renders.

use crossterm::event::KeyCode;
use tempfile::TempDir;

use vauchi_app::ui::{AppEngine, AppScreen, Component, WorkflowEngine};
use vauchi_core::{SymmetricKey, Vauchi, VauchiConfig};

use vauchi_tui::app::App;
use vauchi_tui::handlers::handle_key;

/// Create a test App with an identity + app password (duress requires one),
/// parked on the duress screen.
fn duress_app() -> (App, TempDir) {
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
    // Duress setup is gated on an app password (setup_duress_password errors
    // otherwise), mirroring the real Settings → Security flow.
    app_engine
        .vauchi_mut()
        .setup_app_password("app-password-123")
        .expect("app password");
    let mut app = App::new(
        app_engine,
        "wss://relay.vauchi.app".to_string(),
        temp_dir.path().to_path_buf(),
    );
    app.goto(AppScreen::DuressPin);
    (app, temp_dir)
}

fn has_pin_input(app: &App, want: &str) -> bool {
    app.app_engine
        .current_screen()
        .components
        .iter()
        .any(|c| matches!(c, Component::PinInput { id, .. } if id == want))
}

fn has_text_input(app: &App, want: &str) -> bool {
    app.app_engine
        .current_screen()
        .components
        .iter()
        .any(|c| matches!(c, Component::TextInput { id, .. } if id == want))
}

fn has_status(app: &App, want: &str) -> bool {
    app.app_engine
        .current_screen()
        .components
        .iter()
        .any(|c| matches!(c, Component::StatusIndicator { id, .. } if id == want))
}

fn duress_enabled(app: &App) -> bool {
    app.app_engine
        .vauchi()
        .is_duress_enabled()
        .expect("query duress state")
}

// @scenario: duress_mode :: Enable duress password (requires app password)
#[test]
fn duress_setup_flow_runs_through_engine_and_persists_via_core() {
    let (mut app, _tmp) = duress_app();

    // Renders the engine's overview screen, not a bespoke widget.
    assert!(
        has_status(&app, "duress_status"),
        "should start on overview"
    );

    // Primary action ("Set Up PIN") advances the engine to PIN entry.
    handle_key(&mut app, KeyCode::Enter);
    assert!(has_pin_input(&app, "pin"), "configure → enter-pin step");

    // PIN digits — including 1–5 — must reach the PinInput, not switch tabs
    // or get swallowed by the sync-chrome indicator.
    for c in ['1', '2', '3', '4', '5', '6'] {
        handle_key(&mut app, KeyCode::Char(c));
    }
    assert!(
        has_pin_input(&app, "pin"),
        "digit keys must stay on the PIN screen, not navigate away"
    );

    // Continue → confirm step.
    handle_key(&mut app, KeyCode::Enter);
    assert!(
        has_pin_input(&app, "confirm_pin"),
        "continue → confirm step"
    );

    for c in ['1', '2', '3', '4', '5', '6'] {
        handle_key(&mut app, KeyCode::Char(c));
    }
    handle_key(&mut app, KeyCode::Enter);
    assert!(
        has_text_input(&app, "alert_message"),
        "matching PIN → alerts step"
    );

    // Save (alert message left empty) → engine completes → core persists.
    handle_key(&mut app, KeyCode::Enter);

    assert!(
        duress_enabled(&app),
        "duress PIN must be enabled in storage after the humble setup flow"
    );
}

// @internal — engine rejects a mismatched confirmation PIN (no Gherkin scenario)
#[test]
fn duress_mismatched_confirmation_does_not_persist() {
    let (mut app, _tmp) = duress_app();

    handle_key(&mut app, KeyCode::Enter); // configure
    for c in ['1', '2', '3', '4', '5', '6'] {
        handle_key(&mut app, KeyCode::Char(c));
    }
    handle_key(&mut app, KeyCode::Enter); // continue → confirm
    assert!(has_pin_input(&app, "confirm_pin"), "reached confirm step");

    // Enter a non-matching confirmation PIN.
    for c in ['6', '5', '4', '3', '2', '1'] {
        handle_key(&mut app, KeyCode::Char(c));
    }
    handle_key(&mut app, KeyCode::Enter); // continue — engine rejects (no advance)

    assert!(
        has_pin_input(&app, "confirm_pin"),
        "mismatched PIN must keep the user on the confirm screen"
    );
    assert!(
        !duress_enabled(&app),
        "duress must not be enabled when confirmation PIN does not match"
    );
}
