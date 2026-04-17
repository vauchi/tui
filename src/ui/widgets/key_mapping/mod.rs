// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Key mapping — translates crossterm key events to `UserAction` based on
//! the current `ScreenModel` and render state.

mod action_keys;
mod component_keys;

use crossterm::event::KeyCode;

use vauchi_app::ui::{Component, ScreenModel, UserAction};

use super::screen_renderer::ScreenRenderState;

use action_keys::map_action_key;
use component_keys::map_component_key;

/// Result of handling a key press in the screen renderer context.
pub enum KeyResult {
    /// A `UserAction` to forward to the `WorkflowEngine`.
    Action(UserAction),
    /// Key was consumed for internal navigation (focus/selection change).
    Consumed,
    /// Key was not handled — pass to outer handler.
    Unhandled,
}

/// Returns true if the component is interactive (can receive focus and key input).
fn is_focusable(component: &Component) -> bool {
    !matches!(
        component,
        Component::Text { .. }
            | Component::InfoPanel { .. }
            | Component::StatusIndicator { .. }
            | Component::QrCode { .. }
            | Component::Divider
    )
}

/// Find the next focusable component index in the given direction.
/// Returns `None` if no focusable component exists.
fn find_focusable(components: &[Component], from: usize, forward: bool) -> Option<usize> {
    let len = components.len();
    if len == 0 {
        return None;
    }
    for offset in 1..=len {
        let idx = if forward {
            (from + offset) % len
        } else {
            (from + len - offset) % len
        };
        if is_focusable(&components[idx]) {
            return Some(idx);
        }
    }
    None
}

/// Find the first focusable component index.
fn find_first_focusable(components: &[Component]) -> Option<usize> {
    components.iter().position(is_focusable)
}

/// Map a key press to a `UserAction` or internal navigation update.
///
/// Returns `KeyResult::Action` when the key maps to a `UserAction`,
/// `KeyResult::Consumed` when it was handled internally (e.g., focus change),
/// or `KeyResult::Unhandled` when the key is not relevant.
pub fn map_key(key: KeyCode, screen: &ScreenModel, state: &mut ScreenRenderState) -> KeyResult {
    state.ensure_capacity(screen.components.len());

    // Auto-focus first interactive component if current focus is non-interactive
    if let Some(current) = screen.components.get(state.focused_component)
        && !is_focusable(current)
        && let Some(idx) = find_first_focusable(&screen.components)
    {
        state.focused_component = idx;
    }

    match key {
        // Navigate focus between interactive components
        KeyCode::Tab => {
            if let Some(idx) = find_focusable(&screen.components, state.focused_component, true) {
                state.focused_component = idx;
            }
            KeyResult::Consumed
        }
        KeyCode::BackTab => {
            if let Some(idx) = find_focusable(&screen.components, state.focused_component, false) {
                state.focused_component = idx;
            }
            KeyResult::Consumed
        }

        // Component-specific keys, then fall back to action keys
        _ => {
            if let Some(component) = screen.components.get(state.focused_component) {
                let result = map_component_key(key, component, state);
                match result {
                    KeyResult::Unhandled => {
                        // Up/Down arrows navigate between focusable components
                        // when the focused component doesn't consume them
                        match key {
                            KeyCode::Up => {
                                if let Some(idx) = find_focusable(
                                    &screen.components,
                                    state.focused_component,
                                    false,
                                ) {
                                    state.focused_component = idx;
                                    return KeyResult::Consumed;
                                }
                            }
                            KeyCode::Down => {
                                if let Some(idx) = find_focusable(
                                    &screen.components,
                                    state.focused_component,
                                    true,
                                ) {
                                    state.focused_component = idx;
                                    return KeyResult::Consumed;
                                }
                            }
                            _ => {}
                        }
                        map_action_key(key, screen)
                    }
                    other => other,
                }
            } else {
                map_action_key(key, screen)
            }
        }
    }
}

// INLINE_TEST_REQUIRED: tests need access to pub(crate) key_mapping internals
#[cfg(test)]
mod c_key_tests;
#[cfg(test)]
mod tests {
    use super::*;
    use vauchi_app::ui::*;

    pub(super) fn make_screen(
        components: Vec<Component>,
        actions: Vec<ScreenAction>,
    ) -> ScreenModel {
        ScreenModel {
            screen_id: "test".into(),
            title: "Test".into(),
            subtitle: None,
            components,
            actions,
            progress: None,
            ..Default::default()
        }
    }

    pub(super) fn key_result_debug(result: &KeyResult) -> String {
        match result {
            KeyResult::Action(a) => format!("Action({:?})", a),
            KeyResult::Consumed => "Consumed".to_string(),
            KeyResult::Unhandled => "Unhandled".to_string(),
        }
    }

    fn make_text_input(id: &str) -> Component {
        Component::TextInput {
            id: id.into(),
            label: id.into(),
            value: String::new(),
            placeholder: None,
            max_length: None,
            validation_error: None,
            input_type: InputType::Text,
            a11y: None,
            info_key: None,
        }
    }

    #[test]
    fn test_key_mapping_tab_cycles_focus() {
        let screen = make_screen(vec![make_text_input("a"), make_text_input("b")], vec![]);

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
                make_text_input("a"),
                make_text_input("b"),
                make_text_input("c"),
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
    fn test_key_mapping_tab_skips_non_interactive_components() {
        let screen = make_screen(
            vec![
                Component::Text {
                    id: "label".into(),
                    content: "Label".into(),
                    style: TextStyle::Body,
                },
                make_text_input("input1"),
                Component::Text {
                    id: "info".into(),
                    content: "Info".into(),
                    style: TextStyle::Body,
                },
                make_text_input("input2"),
            ],
            vec![],
        );

        let mut state = ScreenRenderState::default();
        // Auto-focus should jump to index 1 (first TextInput)
        let result = map_key(KeyCode::Tab, &screen, &mut state);
        assert!(matches!(result, KeyResult::Consumed));
        // After auto-focus to 1, Tab goes to next focusable: index 3
        assert_eq!(state.focused_component, 3);

        let result = map_key(KeyCode::Tab, &screen, &mut state);
        assert!(matches!(result, KeyResult::Consumed));
        assert_eq!(state.focused_component, 1); // wraps back to first focusable
    }

    #[test]
    fn test_key_mapping_auto_focus_first_interactive() {
        let screen = make_screen(
            vec![
                Component::Text {
                    id: "title".into(),
                    content: "Title".into(),
                    style: TextStyle::Body,
                },
                make_text_input("name"),
            ],
            vec![],
        );

        let mut state = ScreenRenderState::default();
        assert_eq!(state.focused_component, 0);

        // Typing 'x' should go to the TextInput (auto-focused to index 1)
        let result = map_key(KeyCode::Char('x'), &screen, &mut state);
        assert_eq!(state.focused_component, 1);
        match result {
            KeyResult::Action(UserAction::TextChanged {
                component_id,
                value,
            }) => {
                assert_eq!(component_id, "name");
                assert_eq!(value, "x");
            }
            other => panic!("Expected TextChanged, got {}", key_result_debug(&other)),
        }
    }

    #[test]
    // @scenario: accessibility.feature:Full keyboard navigation on desktop
    // @scenario: accessibility.feature:Focus management during navigation
    // @scenario: accessibility.feature:Text zoom support on desktop
    fn test_key_mapping_arrow_navigates_between_focusable() {
        let screen = make_screen(
            vec![
                make_text_input("a"),
                Component::Text {
                    id: "sep".into(),
                    content: "---".into(),
                    style: TextStyle::Body,
                },
                make_text_input("b"),
            ],
            vec![],
        );

        let mut state = ScreenRenderState::default();
        assert_eq!(state.focused_component, 0);

        // Down arrow: TextInput doesn't consume Down, so it navigates to next focusable
        let result = map_key(KeyCode::Down, &screen, &mut state);
        assert!(matches!(result, KeyResult::Consumed));
        assert_eq!(state.focused_component, 2); // skips Text at index 1

        // Up arrow: back to index 0
        let result = map_key(KeyCode::Up, &screen, &mut state);
        assert!(matches!(result, KeyResult::Consumed));
        assert_eq!(state.focused_component, 0);
    }

    #[test]
    // @scenario: accessibility.feature:Keyboard shortcuts for common actions
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
                a11y: None,
                info_key: None,
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
                a11y: None,
                info_key: None,
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
    // @scenario: accessibility.feature:Arrow key navigation in lists
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
                        a11y: None,
                        info_key: None,
                    },
                    ToggleItem {
                        id: "friends".into(),
                        label: "Friends".into(),
                        selected: true,
                        subtitle: None,
                        a11y: None,
                        info_key: None,
                    },
                ],
                a11y: None,
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
                    a11y: None,
                    info_key: None,
                }],
                a11y: None,
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
                    a11y: None,
                }],
                visibility_mode: VisibilityMode::ShowHide,
                available_groups: vec![],
                a11y: None,
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

    #[test]
    fn test_banner_enter_fires_action_pressed() {
        let screen = make_screen(
            vec![Component::Banner {
                text: "Viewing as Alice".into(),
                action_label: "Exit Preview".into(),
                action_id: "exit-preview".into(),
                a11y: None,
            }],
            vec![],
        );

        let mut state = ScreenRenderState::default();
        let result = map_key(KeyCode::Enter, &screen, &mut state);
        match result {
            KeyResult::Action(UserAction::ActionPressed { action_id }) => {
                assert_eq!(action_id, "exit-preview");
            }
            other => panic!(
                "Expected ActionPressed(exit-preview), got {}",
                key_result_debug(&other)
            ),
        }
    }

    #[test]
    fn test_banner_empty_action_id_enter_is_unhandled() {
        let screen = make_screen(
            vec![Component::Banner {
                text: "Info only".into(),
                action_label: String::new(),
                action_id: String::new(),
                a11y: None,
            }],
            vec![],
        );

        let mut state = ScreenRenderState::default();
        let result = map_key(KeyCode::Enter, &screen, &mut state);
        assert!(
            matches!(result, KeyResult::Unhandled),
            "Enter on banner with empty action_id should be unhandled"
        );
    }

    #[test]
    fn test_banner_non_enter_is_unhandled() {
        let screen = make_screen(
            vec![Component::Banner {
                text: "Preview mode".into(),
                action_label: "Exit".into(),
                action_id: "exit".into(),
                a11y: None,
            }],
            vec![],
        );

        let mut state = ScreenRenderState::default();
        let result = map_key(KeyCode::Char('x'), &screen, &mut state);
        assert!(
            matches!(result, KeyResult::Unhandled),
            "Non-Enter key on banner should be unhandled"
        );
    }

    #[test]
    fn test_inline_confirm_enter_fires_confirm_prefixed_action() {
        let screen = make_screen(
            vec![Component::InlineConfirm {
                id: "delete_group".into(),
                warning: "Are you sure?".into(),
                confirm_text: "Delete".into(),
                cancel_text: "Cancel".into(),
                destructive: true,
                a11y: None,
            }],
            vec![],
        );

        let mut state = ScreenRenderState::default();
        let result = map_key(KeyCode::Enter, &screen, &mut state);
        match result {
            KeyResult::Action(UserAction::ActionPressed { action_id }) => {
                assert_eq!(
                    action_id, "confirm_delete_group",
                    "Enter on InlineConfirm must use confirm_ prefix convention"
                );
            }
            other => panic!(
                "Expected ActionPressed(confirm_delete_group), got {}",
                key_result_debug(&other)
            ),
        }
    }

    #[test]
    fn test_inline_confirm_esc_fires_cancel_prefixed_action() {
        let screen = make_screen(
            vec![Component::InlineConfirm {
                id: "delete_group".into(),
                warning: "Are you sure?".into(),
                confirm_text: "Delete".into(),
                cancel_text: "Cancel".into(),
                destructive: true,
                a11y: None,
            }],
            vec![],
        );

        let mut state = ScreenRenderState::default();
        let result = map_key(KeyCode::Esc, &screen, &mut state);
        match result {
            KeyResult::Action(UserAction::ActionPressed { action_id }) => {
                assert_eq!(
                    action_id, "cancel_delete_group",
                    "Esc on InlineConfirm must use cancel_ prefix convention"
                );
            }
            other => panic!(
                "Expected ActionPressed(cancel_delete_group), got {}",
                key_result_debug(&other)
            ),
        }
    }
}
