// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests for ADR-044 Am2a nav_actions key dispatch.

use crossterm::event::KeyCode;

use vauchi_app::ui::{ActionStyle, ScreenAction, UserAction};

use super::super::screen_renderer::ScreenRenderState;
use super::tests::{key_result_debug, make_screen_with_nav_actions};
use super::{KeyResult, map_key};

/// ADR-044 Am2a: the reserved `go_back` chrome action dispatches
/// `UserAction::NavigateBack` so the visible back affordance and the
/// system Esc gesture share one code path.
// @internal
#[test]
fn test_nav_action_go_back_dispatches_navigate_back() {
    let screen = make_screen_with_nav_actions(
        vec![],
        vec![],
        vec![ScreenAction {
            id: "go_back".into(),
            label: "Back".into(),
            style: ActionStyle::Secondary,
            enabled: true,
            a11y: None,
        }],
    );

    let mut state = ScreenRenderState::default();
    let result = map_key(KeyCode::Esc, &screen, &mut state);
    assert!(
        matches!(result, KeyResult::Action(UserAction::NavigateBack)),
        "Esc with go_back nav_action must dispatch NavigateBack, got {}",
        key_result_debug(&result)
    );
}

/// ADR-044 Am2a: non-back chrome actions (e.g. `open_settings`) dispatch
/// as ordinary `ActionPressed` so core can resolve them.
// @internal
#[test]
fn test_nav_action_open_settings_dispatches_action_pressed() {
    let screen = make_screen_with_nav_actions(
        vec![],
        vec![],
        vec![ScreenAction {
            id: "open_settings".into(),
            label: "Settings".into(),
            style: ActionStyle::Secondary,
            enabled: true,
            a11y: None,
        }],
    );

    let mut state = ScreenRenderState::default();
    let result = map_key(KeyCode::Char('o'), &screen, &mut state);
    match result {
        KeyResult::Action(UserAction::ActionPressed { action_id }) => {
            assert_eq!(action_id, "open_settings");
        }
        other => panic!(
            "Expected ActionPressed(open_settings), got {}",
            key_result_debug(&other)
        ),
    }
}

/// Disabled nav actions must not be dispatched.
// @internal
#[test]
fn test_disabled_nav_action_is_ignored() {
    let screen = make_screen_with_nav_actions(
        vec![],
        vec![],
        vec![ScreenAction {
            id: "go_back".into(),
            label: "Back".into(),
            style: ActionStyle::Secondary,
            enabled: false,
            a11y: None,
        }],
    );

    let mut state = ScreenRenderState::default();
    let result = map_key(KeyCode::Esc, &screen, &mut state);
    assert!(
        matches!(result, KeyResult::Unhandled),
        "disabled go_back must be ignored, got {}",
        key_result_debug(&result)
    );
}
