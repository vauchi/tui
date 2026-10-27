// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Core keeps the language and theme pick in the terminal's data
//! directory, so a pick made in Settings applies at the next start
//! (vauchi/private#814).

use std::path::Path;

use vauchi_app::ui::{AppEngine, AppScreen, UserAction, WorkflowEngine};
use vauchi_core::api::Vauchi;
use vauchi_tui::app::App;

fn start_in(data_dir: &Path) -> App {
    let engine = AppEngine::new(Vauchi::in_memory().expect("in-memory core"));
    App::new(engine, String::new(), data_dir.to_path_buf())
}

// @internal
#[test]
fn a_pick_made_in_settings_applies_at_the_next_start() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = start_in(dir.path());
    app.app_engine.navigate_to(AppScreen::SettingsAppearance);
    let _ = app.app_engine.handle_action(UserAction::ListItemSelected {
        component_id: "language".into(),
        item_id: "de".into(),
    });

    let next_start = start_in(dir.path());

    assert_eq!(
        next_start.app_engine.render_context().locale.as_deref(),
        Some("de")
    );
}

// @internal
#[test]
fn a_pick_the_terminal_kept_itself_still_applies() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("restorable_state"),
        r#"{"v":1,"locale":"de","theme_id":null}"#,
    )
    .unwrap();

    let app = start_in(dir.path());

    assert_eq!(
        app.app_engine.render_context().locale.as_deref(),
        Some("de")
    );
}
