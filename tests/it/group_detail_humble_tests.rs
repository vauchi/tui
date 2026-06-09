// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Group-detail screen — Humble-UI migration (ADR-021 / ADR-043).
//!
//! The screen renders the core `GroupDetailEngine`; the bespoke
//! `handle_group_detail_keys` (member-list cursor + `'r'` rename) is gone.
//! The engine provides the equivalents: a focusable `Component::List` of
//! members (navigated via the shared `map_key`) and a `rename` action that
//! the AppEngine routes to the rename `FormDialog`.

use tempfile::TempDir;

use vauchi_app::ui::{AppEngine, AppScreen, Component, WorkflowEngine};
use vauchi_core::{SymmetricKey, Vauchi, VauchiConfig};

use vauchi_tui::app::{App, Screen};

fn group_detail_app() -> (App, TempDir) {
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
    let group = app_engine
        .vauchi()
        .create_group("Friends")
        .expect("create group");
    let mut app = App::new(
        app_engine,
        "wss://relay.vauchi.app".to_string(),
        temp_dir.path().to_path_buf(),
    );
    // Navigate the live engine to this group's detail (mirrors selecting it
    // from the Groups list); read the engine screen directly without
    // re-syncing, which would re-derive the group from list context.
    app.app_engine.navigate_to(AppScreen::GroupDetail {
        group_id: group.id().to_string(),
    });
    app.goto(Screen::GroupDetail);
    (app, temp_dir)
}

// @internal
#[test]
fn group_detail_engine_provides_member_list_and_rename() {
    let (app, _tmp) = group_detail_app();
    let screen = app.app_engine.current_screen();

    // Member navigation is the engine's focusable List, not bespoke cursor state.
    assert!(
        screen
            .components
            .iter()
            .any(|c| matches!(c, Component::List { .. })),
        "GroupDetail must render a member List for map_key navigation"
    );

    // Rename is an engine action (routed to the rename FormDialog by the
    // AppEngine), replacing the bespoke `'r'` shortcut.
    assert!(
        screen.actions.iter().any(|a| a.id == "rename"),
        "GroupDetail must expose a 'rename' action"
    );
}
