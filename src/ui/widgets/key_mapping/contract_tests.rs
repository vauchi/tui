// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crossterm::event::KeyCode;
use vauchi_app::ui::{Component, UserAction};

use super::super::screen_renderer::ScreenRenderState;
use super::{
    KeyResult, map_key,
    tests::{key_result_debug, make_screen},
};

fn inline_confirm() -> Component {
    Component::InlineConfirm {
        id: "delete_group".into(),
        warning: "Are you sure?".into(),
        confirm_text: "Delete".into(),
        cancel_text: "Cancel".into(),
        confirm_action_id: "core-delete-group".into(),
        cancel_action_id: "core-keep-group".into(),
        destructive: true,
        a11y: None,
    }
}

#[test]
fn inline_confirm_enter_forwards_core_confirm_action() {
    let screen = make_screen(vec![inline_confirm()], vec![]);
    let mut state = ScreenRenderState::default();
    let result = map_key(KeyCode::Enter, &screen, &mut state);
    match result {
        KeyResult::Action(UserAction::ActionPressed { action_id }) => {
            assert_eq!(
                action_id, "core-delete-group",
                "Enter on InlineConfirm must forward the core action id verbatim"
            );
        }
        other => panic!(
            "Expected ActionPressed(core-delete-group), got {}",
            key_result_debug(&other)
        ),
    }
}

#[test]
fn inline_confirm_esc_forwards_core_cancel_action() {
    let screen = make_screen(vec![inline_confirm()], vec![]);
    let mut state = ScreenRenderState::default();
    let result = map_key(KeyCode::Esc, &screen, &mut state);
    match result {
        KeyResult::Action(UserAction::ActionPressed { action_id }) => {
            assert_eq!(
                action_id, "core-keep-group",
                "Esc on InlineConfirm must forward the core action id verbatim"
            );
        }
        other => panic!(
            "Expected ActionPressed(core-keep-group), got {}",
            key_result_debug(&other)
        ),
    }
}

fn editable_text(editing: bool) -> Component {
    Component::EditableText {
        id: "display-name".into(),
        label: "Display name".into(),
        value: "Alice".into(),
        edit_text: "Rename".into(),
        save_text: "Keep name".into(),
        cancel_text: "Discard".into(),
        edit_action_id: "core-start-rename".into(),
        save_action_id: "core-save-rename".into(),
        cancel_action_id: "core-cancel-rename".into(),
        editing,
        validation_error: None,
        a11y: None,
        info_key: None,
    }
}

// @internal
#[test]
fn editable_text_forwards_core_transition_actions() {
    let mut state = ScreenRenderState::default();
    for (component, key, expected) in [
        (editable_text(false), KeyCode::Enter, "core-start-rename"),
        (editable_text(true), KeyCode::Enter, "core-save-rename"),
        (editable_text(true), KeyCode::Esc, "core-cancel-rename"),
    ] {
        let screen = make_screen(vec![component], vec![]);
        let result = map_key(key, &screen, &mut state);
        assert!(matches!(
            result,
            KeyResult::Action(UserAction::ActionPressed { action_id })
                if action_id == expected
        ));
    }
}

// @internal
#[test]
fn editable_text_ignores_text_input_until_core_enters_edit_mode() {
    let screen = make_screen(vec![editable_text(false)], vec![]);
    let mut state = ScreenRenderState::default();
    assert!(matches!(
        map_key(KeyCode::Char('x'), &screen, &mut state),
        KeyResult::Unhandled
    ));
}
