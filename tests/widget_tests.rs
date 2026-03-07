// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests for the TUI component library widgets and key mapping.

use crossterm::event::KeyCode;
use vauchi_core::ui::*;

use vauchi_tui::ui::widgets::key_mapping::{map_key, KeyResult};
use vauchi_tui::ui::widgets::screen_renderer::ScreenRenderState;

// ── ScreenRenderState tests ─────────────────────────────────────────

#[test]
fn test_render_state_default() {
    let state = ScreenRenderState::default();
    assert_eq!(state.focused_component, 0);
    assert!(state.component_selections.is_empty());
    assert!(state.validation_errors.is_empty());
}

#[test]
fn test_render_state_ensure_capacity() {
    let mut state = ScreenRenderState::default();
    state.ensure_capacity(3);
    assert_eq!(state.component_selections.len(), 3);
    assert_eq!(state.selection_for(0), 0);
    assert_eq!(state.selection_for(1), 0);
    assert_eq!(state.selection_for(2), 0);
}

#[test]
fn test_render_state_selection_for_out_of_bounds() {
    let state = ScreenRenderState::default();
    assert_eq!(state.selection_for(99), 0);
}

#[test]
fn test_render_state_validation_errors() {
    let mut state = ScreenRenderState::default();

    // No error initially
    assert!(state.validation_error_for("name").is_none());

    // Set an error
    state.set_validation_error("name".to_string(), "Required".to_string());
    assert_eq!(state.validation_error_for("name"), Some("Required"));

    // Update the error
    state.set_validation_error("name".to_string(), "Too short".to_string());
    assert_eq!(state.validation_error_for("name"), Some("Too short"));
    assert_eq!(state.validation_errors.len(), 1);

    // Clear the error
    state.clear_validation_error("name");
    assert!(state.validation_error_for("name").is_none());
}

#[test]
fn test_render_state_clear_all_errors() {
    let mut state = ScreenRenderState::default();
    state.set_validation_error("a".to_string(), "err1".to_string());
    state.set_validation_error("b".to_string(), "err2".to_string());
    assert_eq!(state.validation_errors.len(), 2);

    state.clear_all_errors();
    assert!(state.validation_errors.is_empty());
}

// ── Key mapping tests ───────────────────────────────────────────────

fn make_screen(components: Vec<Component>, actions: Vec<ScreenAction>) -> ScreenModel {
    ScreenModel {
        screen_id: "test".into(),
        title: "Test".into(),
        subtitle: None,
        components,
        actions,
        progress: None,
    }
}

#[test]
fn test_key_mapping_tab_cycles_focus() {
    let screen = make_screen(
        vec![
            Component::Text {
                id: "t1".into(),
                content: "Hello".into(),
                style: TextStyle::Body,
            },
            Component::Text {
                id: "t2".into(),
                content: "World".into(),
                style: TextStyle::Body,
            },
        ],
        vec![],
    );

    let mut state = ScreenRenderState::default();
    assert_eq!(state.focused_component, 0);

    let result = map_key(KeyCode::Tab, &screen, &mut state);
    assert!(matches!(result, KeyResult::Consumed));
    assert_eq!(state.focused_component, 1);

    let result = map_key(KeyCode::Tab, &screen, &mut state);
    assert!(matches!(result, KeyResult::Consumed));
    assert_eq!(state.focused_component, 0); // wraps around
}

#[test]
fn test_key_mapping_backtab_cycles_focus_backwards() {
    let screen = make_screen(
        vec![
            Component::Text {
                id: "t1".into(),
                content: "A".into(),
                style: TextStyle::Body,
            },
            Component::Text {
                id: "t2".into(),
                content: "B".into(),
                style: TextStyle::Body,
            },
            Component::Text {
                id: "t3".into(),
                content: "C".into(),
                style: TextStyle::Body,
            },
        ],
        vec![],
    );

    let mut state = ScreenRenderState::default();
    assert_eq!(state.focused_component, 0);

    let result = map_key(KeyCode::BackTab, &screen, &mut state);
    assert!(matches!(result, KeyResult::Consumed));
    assert_eq!(state.focused_component, 2); // wraps to end
}

#[test]
fn test_key_mapping_text_input_char() {
    let screen = make_screen(
        vec![Component::TextInput {
            id: "name".into(),
            label: "Name".into(),
            value: "Ali".into(),
            placeholder: None,
            max_length: None,
            validation_error: None,
            input_type: InputType::Text,
        }],
        vec![],
    );

    let mut state = ScreenRenderState::default();

    let result = map_key(KeyCode::Char('c'), &screen, &mut state);
    match result {
        KeyResult::Action(UserAction::TextChanged {
            component_id,
            value,
        }) => {
            assert_eq!(component_id, "name");
            assert_eq!(value, "Alic");
        }
        other => panic!("Expected TextChanged, got {:?}", key_result_debug(&other)),
    }
}

#[test]
fn test_key_mapping_text_input_backspace() {
    let screen = make_screen(
        vec![Component::TextInput {
            id: "name".into(),
            label: "Name".into(),
            value: "Ali".into(),
            placeholder: None,
            max_length: None,
            validation_error: None,
            input_type: InputType::Text,
        }],
        vec![],
    );

    let mut state = ScreenRenderState::default();

    let result = map_key(KeyCode::Backspace, &screen, &mut state);
    match result {
        KeyResult::Action(UserAction::TextChanged {
            component_id,
            value,
        }) => {
            assert_eq!(component_id, "name");
            assert_eq!(value, "Al");
        }
        other => panic!("Expected TextChanged, got {:?}", key_result_debug(&other)),
    }
}

#[test]
fn test_key_mapping_toggle_list_navigation() {
    let screen = make_screen(
        vec![Component::ToggleList {
            id: "groups".into(),
            label: "Groups".into(),
            items: vec![
                ToggleItem {
                    id: "family".into(),
                    label: "Family".into(),
                    selected: false,
                    subtitle: None,
                },
                ToggleItem {
                    id: "friends".into(),
                    label: "Friends".into(),
                    selected: true,
                    subtitle: None,
                },
            ],
        }],
        vec![],
    );

    let mut state = ScreenRenderState::default();
    state.ensure_capacity(1);

    // Move down
    let result = map_key(KeyCode::Down, &screen, &mut state);
    assert!(matches!(result, KeyResult::Consumed));
    assert_eq!(state.selection_for(0), 1);

    // Can't go past end
    let result = map_key(KeyCode::Down, &screen, &mut state);
    assert!(matches!(result, KeyResult::Consumed));
    assert_eq!(state.selection_for(0), 1);

    // Move up
    let result = map_key(KeyCode::Up, &screen, &mut state);
    assert!(matches!(result, KeyResult::Consumed));
    assert_eq!(state.selection_for(0), 0);
}

#[test]
fn test_key_mapping_toggle_list_space_toggles() {
    let screen = make_screen(
        vec![Component::ToggleList {
            id: "groups".into(),
            label: "Groups".into(),
            items: vec![ToggleItem {
                id: "family".into(),
                label: "Family".into(),
                selected: false,
                subtitle: None,
            }],
        }],
        vec![],
    );

    let mut state = ScreenRenderState::default();
    state.ensure_capacity(1);

    let result = map_key(KeyCode::Char(' '), &screen, &mut state);
    match result {
        KeyResult::Action(UserAction::ItemToggled {
            component_id,
            item_id,
        }) => {
            assert_eq!(component_id, "groups");
            assert_eq!(item_id, "family");
        }
        other => panic!("Expected ItemToggled, got {:?}", key_result_debug(&other)),
    }
}

#[test]
fn test_key_mapping_enter_presses_primary_action() {
    let screen = make_screen(
        vec![Component::Text {
            id: "t".into(),
            content: "info".into(),
            style: TextStyle::Body,
        }],
        vec![
            ScreenAction {
                id: "continue".into(),
                label: "Continue".into(),
                style: ActionStyle::Primary,
                enabled: true,
            },
            ScreenAction {
                id: "skip".into(),
                label: "Skip".into(),
                style: ActionStyle::Secondary,
                enabled: true,
            },
        ],
    );

    let mut state = ScreenRenderState::default();

    // Text component doesn't handle Enter, so it falls through to action keys
    let result = map_key(KeyCode::Enter, &screen, &mut state);
    match result {
        KeyResult::Action(UserAction::ActionPressed { action_id }) => {
            assert_eq!(action_id, "continue");
        }
        other => panic!(
            "Expected ActionPressed(continue), got {:?}",
            key_result_debug(&other)
        ),
    }
}

#[test]
fn test_key_mapping_s_presses_skip_action() {
    let screen = make_screen(
        vec![Component::Text {
            id: "t".into(),
            content: "info".into(),
            style: TextStyle::Body,
        }],
        vec![ScreenAction {
            id: "skip".into(),
            label: "Skip".into(),
            style: ActionStyle::Secondary,
            enabled: true,
        }],
    );

    let mut state = ScreenRenderState::default();

    let result = map_key(KeyCode::Char('s'), &screen, &mut state);
    match result {
        KeyResult::Action(UserAction::ActionPressed { action_id }) => {
            assert_eq!(action_id, "skip");
        }
        other => panic!(
            "Expected ActionPressed(skip), got {:?}",
            key_result_debug(&other)
        ),
    }
}

#[test]
fn test_key_mapping_disabled_action_not_triggered() {
    let screen = make_screen(
        vec![Component::Text {
            id: "t".into(),
            content: "info".into(),
            style: TextStyle::Body,
        }],
        vec![ScreenAction {
            id: "continue".into(),
            label: "Continue".into(),
            style: ActionStyle::Primary,
            enabled: false,
        }],
    );

    let mut state = ScreenRenderState::default();

    let result = map_key(KeyCode::Enter, &screen, &mut state);
    assert!(matches!(result, KeyResult::Unhandled));
}

#[test]
fn test_key_mapping_unhandled_key() {
    let screen = make_screen(vec![], vec![]);
    let mut state = ScreenRenderState::default();

    let result = map_key(KeyCode::F(1), &screen, &mut state);
    assert!(matches!(result, KeyResult::Unhandled));
}

#[test]
fn test_key_mapping_field_list_toggle_visibility() {
    let screen = make_screen(
        vec![Component::FieldList {
            id: "fields".into(),
            fields: vec![FieldDisplay {
                id: "field_0".into(),
                field_type: "Email".into(),
                label: "Email".into(),
                value: "a@b.c".into(),
                visibility: UiFieldVisibility::Shown,
            }],
            visibility_mode: VisibilityMode::ShowHide,
            available_groups: vec![],
        }],
        vec![],
    );

    let mut state = ScreenRenderState::default();
    state.ensure_capacity(1);

    let result = map_key(KeyCode::Char(' '), &screen, &mut state);
    match result {
        KeyResult::Action(UserAction::FieldVisibilityChanged {
            field_id,
            group_id,
            visible,
        }) => {
            assert_eq!(field_id, "field_0");
            assert!(group_id.is_none());
            assert!(!visible); // Was Shown, toggling to hidden
        }
        other => panic!(
            "Expected FieldVisibilityChanged, got {:?}",
            key_result_debug(&other)
        ),
    }
}

#[test]
fn test_key_mapping_empty_screen_tab() {
    let screen = make_screen(vec![], vec![]);
    let mut state = ScreenRenderState::default();

    // Tab on empty component list should not panic
    let result = map_key(KeyCode::Tab, &screen, &mut state);
    assert!(matches!(result, KeyResult::Consumed));
    assert_eq!(state.focused_component, 0);
}

// ── Helper ──────────────────────────────────────────────────────────

fn key_result_debug(result: &KeyResult) -> String {
    match result {
        KeyResult::Action(a) => format!("Action({:?})", a),
        KeyResult::Consumed => "Consumed".to_string(),
        KeyResult::Unhandled => "Unhandled".to_string(),
    }
}
