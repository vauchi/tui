// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! The terminal keeps Core's opaque restorable state in its data
//! directory and hands it back at the next start, so a language or theme
//! picked in Settings outlives the vault copy (vauchi/private#814).

use vauchi_app::ui::{AppEngine, AppScreen, UserAction, WorkflowEngine};
use vauchi_core::api::Vauchi;
use vauchi_tui::app::App;
use vauchi_tui::app::restorable_state::{hand_back_kept_state, keep_state, kept_state};

fn engine() -> AppEngine {
    AppEngine::new(Vauchi::in_memory().expect("in-memory core"))
}

// @internal
#[test]
fn a_kept_value_comes_back_unchanged() {
    let dir = tempfile::tempdir().unwrap();

    keep_state(dir.path(), r#"{"v":1,"locale":"de","theme_id":null}"#).unwrap();

    assert_eq!(
        kept_state(dir.path()).as_deref(),
        Some(r#"{"v":1,"locale":"de","theme_id":null}"#)
    );
}

// @internal
#[test]
fn a_data_directory_that_kept_nothing_hands_back_nothing() {
    let dir = tempfile::tempdir().unwrap();

    assert_eq!(kept_state(dir.path()), None);
}

// @internal
#[test]
fn an_oversized_or_non_utf8_file_is_not_handed_back() {
    let dir = tempfile::tempdir().unwrap();
    let kept = |bytes: &[u8]| {
        keep_state(dir.path(), "placeholder").unwrap();
        let path = std::fs::read_dir(dir.path())
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        std::fs::write(path, bytes).unwrap();
        kept_state(dir.path())
    };

    assert_eq!(kept(&[b'a'; 4097]), None);
    assert_eq!(kept(&[0xff, 0xfe]), None);
    assert_eq!(kept(&[b'a'; 4096]).map(|s| s.len()), Some(4096));
}

// @internal
#[test]
fn a_pick_is_kept_and_applies_at_the_next_start() {
    let dir = tempfile::tempdir().unwrap();
    let mut app = App::new(engine(), String::new(), dir.path().to_path_buf());
    app.app_engine.navigate_to(AppScreen::SettingsAppearance);
    let _ = app.app_engine.handle_action(UserAction::ListItemSelected {
        component_id: "language".into(),
        item_id: "de".into(),
    });

    app.tick_notifications();
    let mut next_start = engine();
    hand_back_kept_state(&mut next_start, dir.path());

    assert_eq!(next_start.render_context().locale.as_deref(), Some("de"));
}
