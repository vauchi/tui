// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Emergency broadcast screen — Humble-UI migration (ADR-021 / ADR-043).
//!
//! The screen renders the core `EmergencyBroadcastEngine` and forwards all
//! input through it. This test covers only the TUI's Humble concern — the
//! overview and contacts sub-screens render their components (the contacts
//! step is a `ToggleList` picker over the injected contact pool, not free
//! text). The *behaviour* — recipient selection + message persist, and an
//! empty selection is rejected — is core's and lives in
//! `core/vauchi-app/tests/it/emergency_broadcast_wiring_tests.rs`
//! (CC-24: a frontend test must not assert core state).

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

fn has_status(app: &App, want: &str) -> bool {
    app.app_engine
        .current_screen()
        .components
        .iter()
        .any(|c| matches!(c, Component::StatusIndicator { id, .. } if id == want))
}

fn has_toggle_list(app: &App, want: &str) -> bool {
    app.app_engine
        .current_screen()
        .components
        .iter()
        .any(|c| matches!(c, Component::ToggleList { id, .. } if id == want))
}

// @internal — the EmergencyBroadcast overview + contacts sub-screens render.
// The contacts step is a ToggleList picker (not free text). Selection,
// persistence, and empty-rejection are core's (see
// `emergency_broadcast_wiring_tests`).
#[test]
fn emergency_broadcast_screens_render() {
    let (mut app, _tmp) = emergency_app();

    // Renders the engine overview, not a bespoke widget.
    assert!(has_status(&app, "emergency_status"), "starts on overview");

    // Configure → contacts picker (a ToggleList over available contacts).
    handle_key(&mut app, KeyCode::Enter);
    assert!(
        has_toggle_list(&app, "contact_ids"),
        "configure → contacts step renders the recipient picker"
    );
}
