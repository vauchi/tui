// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests for 'c' key conflict — create_new vs legacy Contacts navigation.

use crossterm::event::KeyCode;
use vauchi_app::ui::*;

use super::tests::make_screen;
use super::{KeyResult, map_key};
use crate::ui::widgets::screen_renderer::ScreenRenderState;

/// Verify 'c' triggers `create_new` when the action exists on the screen
/// (e.g., Onboarding screen with a "Create new identity" action).
#[test]
fn test_c_key_triggers_create_new_when_action_exists() {
    let screen = make_screen(
        vec![Component::Text {
            id: "welcome".into(),
            content: "Welcome to Vauchi".into(),
            style: TextStyle::Body,
            accessible_label: None,
            accessible_hint: None,
        }],
        vec![ScreenAction {
            id: "create_new".into(),
            label: "Create New Identity".into(),
            style: ActionStyle::Primary,
            enabled: true,
        }],
    );

    let mut state = ScreenRenderState::default();

    let result = map_key(KeyCode::Char('c'), &screen, &mut state);
    match result {
        KeyResult::Action(UserAction::ActionPressed { action_id }) => {
            assert_eq!(
                action_id, "create_new",
                "'c' should trigger create_new action when present"
            );
        }
        other => panic!(
            "Expected ActionPressed(create_new), got {}",
            super::tests::key_result_debug(&other)
        ),
    }
}

/// Verify 'c' returns `Unhandled` when no `create_new` action exists
/// (e.g., Home screen), allowing the legacy nav handler to pick it up
/// for Contacts navigation.
#[test]
fn test_c_key_unhandled_when_no_create_new_action() {
    // Simulate a Home-like screen: has actions, but none with id "create_new"
    let screen = make_screen(
        vec![Component::Text {
            id: "home_info".into(),
            content: "Your card".into(),
            style: TextStyle::Body,
            accessible_label: None,
            accessible_hint: None,
        }],
        vec![ScreenAction {
            id: "add_field".into(),
            label: "Add Field".into(),
            style: ActionStyle::Secondary,
            enabled: true,
        }],
    );

    let mut state = ScreenRenderState::default();

    let result = map_key(KeyCode::Char('c'), &screen, &mut state);
    assert!(
        matches!(result, KeyResult::Unhandled),
        "'c' should be Unhandled when no create_new action exists, got {}",
        super::tests::key_result_debug(&result)
    );
}

/// Verify 'c' returns `Unhandled` when `create_new` action exists but is disabled.
#[test]
fn test_c_key_unhandled_when_create_new_disabled() {
    let screen = make_screen(
        vec![Component::Text {
            id: "info".into(),
            content: "info".into(),
            style: TextStyle::Body,
            accessible_label: None,
            accessible_hint: None,
        }],
        vec![ScreenAction {
            id: "create_new".into(),
            label: "Create New Identity".into(),
            style: ActionStyle::Primary,
            enabled: false,
        }],
    );

    let mut state = ScreenRenderState::default();

    let result = map_key(KeyCode::Char('c'), &screen, &mut state);
    assert!(
        matches!(result, KeyResult::Unhandled),
        "'c' should be Unhandled when create_new is disabled, got {}",
        super::tests::key_result_debug(&result)
    );
}

/// Verify 'c' returns `Unhandled` on a screen with no actions at all
/// (empty actions list).
#[test]
fn test_c_key_unhandled_on_screen_with_no_actions() {
    let screen = make_screen(
        vec![Component::Text {
            id: "t".into(),
            content: "text".into(),
            style: TextStyle::Body,
            accessible_label: None,
            accessible_hint: None,
        }],
        vec![],
    );

    let mut state = ScreenRenderState::default();

    let result = map_key(KeyCode::Char('c'), &screen, &mut state);
    assert!(
        matches!(result, KeyResult::Unhandled),
        "'c' should be Unhandled on screen with no actions, got {}",
        super::tests::key_result_debug(&result)
    );
}
