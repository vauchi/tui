// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Key mapping — translates crossterm key events to `UserAction` based on
//! the current `ScreenModel` and render state.

use crossterm::event::KeyCode;

use vauchi_core::ui::{Component, ScreenModel, UserAction};

use super::screen_renderer::ScreenRenderState;

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
    if let Some(current) = screen.components.get(state.focused_component) {
        if !is_focusable(current) {
            if let Some(idx) = find_first_focusable(&screen.components) {
                state.focused_component = idx;
            }
        }
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

/// Map a key to a component-specific action.
fn map_component_key(
    key: KeyCode,
    component: &Component,
    state: &mut ScreenRenderState,
) -> KeyResult {
    match component {
        Component::TextInput { id, value, .. } => match key {
            KeyCode::Char(c) => {
                let mut new_value = value.clone();
                new_value.push(c);
                KeyResult::Action(UserAction::TextChanged {
                    component_id: id.clone(),
                    value: new_value,
                })
            }
            KeyCode::Backspace => {
                let mut new_value = value.clone();
                new_value.pop();
                KeyResult::Action(UserAction::TextChanged {
                    component_id: id.clone(),
                    value: new_value,
                })
            }
            KeyCode::Enter if !value.is_empty() => KeyResult::Action(UserAction::ActionPressed {
                action_id: format!("submit_{id}"),
            }),
            _ => KeyResult::Unhandled,
        },

        Component::ToggleList { id, items, .. } => {
            let idx = state.focused_component;
            let sel = state.selection_for(idx);
            match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    if sel > 0 {
                        state.component_selections[idx] = sel - 1;
                        KeyResult::Consumed
                    } else {
                        // At top of list — let parent handle (move to prev component)
                        KeyResult::Unhandled
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if sel < items.len().saturating_sub(1) {
                        state.component_selections[idx] = sel + 1;
                        KeyResult::Consumed
                    } else {
                        // At bottom of list — let parent handle (move to next component)
                        KeyResult::Unhandled
                    }
                }
                // Space toggles selection; Enter falls through to screen actions (e.g. Continue)
                KeyCode::Char(' ') => {
                    if let Some(item) = items.get(sel) {
                        KeyResult::Action(UserAction::ItemToggled {
                            component_id: id.clone(),
                            item_id: item.id.clone(),
                        })
                    } else {
                        KeyResult::Consumed
                    }
                }
                _ => KeyResult::Unhandled,
            }
        }

        Component::FieldList { fields, .. } => {
            let idx = state.focused_component;
            let sel = state.selection_for(idx);
            match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    if sel > 0 {
                        state.component_selections[idx] = sel - 1;
                        KeyResult::Consumed
                    } else {
                        KeyResult::Unhandled
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if sel < fields.len().saturating_sub(1) {
                        state.component_selections[idx] = sel + 1;
                        KeyResult::Consumed
                    } else {
                        KeyResult::Unhandled
                    }
                }
                KeyCode::Char(' ') => {
                    // Toggle visibility of the selected field
                    if let Some(field) = fields.get(sel) {
                        let currently_visible =
                            !matches!(field.visibility, vauchi_core::ui::UiFieldVisibility::Hidden);
                        KeyResult::Action(UserAction::FieldVisibilityChanged {
                            field_id: field.id.clone(),
                            group_id: None,
                            visible: !currently_visible,
                        })
                    } else {
                        KeyResult::Consumed
                    }
                }
                _ => KeyResult::Unhandled,
            }
        }

        Component::CardPreview {
            group_views,
            selected_group,
            ..
        } => match key {
            KeyCode::Left | KeyCode::Char('[') | KeyCode::Right | KeyCode::Char(']') => {
                if group_views.is_empty() {
                    return KeyResult::Consumed;
                }
                // Find current index and cycle to next/previous
                let current_idx = selected_group
                    .as_ref()
                    .and_then(|sg| group_views.iter().position(|g| &g.group_name == sg))
                    .unwrap_or(0);
                let next_idx = if matches!(key, KeyCode::Left | KeyCode::Char('[')) {
                    if current_idx == 0 {
                        group_views.len() - 1
                    } else {
                        current_idx - 1
                    }
                } else {
                    (current_idx + 1) % group_views.len()
                };
                KeyResult::Action(UserAction::GroupViewSelected {
                    group_name: Some(group_views[next_idx].group_name.clone()),
                })
            }
            _ => KeyResult::Unhandled,
        },

        Component::ContactList { id, contacts, .. } => {
            let idx = state.focused_component;
            let sel = state.selection_for(idx);
            match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    if sel > 0 {
                        state.component_selections[idx] = sel - 1;
                        KeyResult::Consumed
                    } else {
                        KeyResult::Unhandled
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if sel < contacts.len().saturating_sub(1) {
                        state.component_selections[idx] = sel + 1;
                        KeyResult::Consumed
                    } else {
                        KeyResult::Unhandled
                    }
                }
                KeyCode::Enter => {
                    if let Some(contact) = contacts.get(sel) {
                        KeyResult::Action(UserAction::ListItemSelected {
                            component_id: id.clone(),
                            item_id: contact.id.clone(),
                        })
                    } else {
                        KeyResult::Consumed
                    }
                }
                _ => KeyResult::Unhandled,
            }
        }

        Component::SettingsGroup { id, items, .. } => {
            let idx = state.focused_component;
            let sel = state.selection_for(idx);
            match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    if sel > 0 {
                        state.component_selections[idx] = sel - 1;
                        KeyResult::Consumed
                    } else {
                        // At top — let parent move to previous component
                        KeyResult::Unhandled
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if sel < items.len().saturating_sub(1) {
                        state.component_selections[idx] = sel + 1;
                        KeyResult::Consumed
                    } else {
                        // At bottom — let parent move to next component
                        KeyResult::Unhandled
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    if let Some(item) = items.get(sel) {
                        match &item.kind {
                            vauchi_core::ui::SettingsItemKind::Toggle { .. } => {
                                KeyResult::Action(UserAction::SettingsToggled {
                                    component_id: id.clone(),
                                    item_id: item.id.clone(),
                                })
                            }
                            _ => KeyResult::Action(UserAction::ListItemSelected {
                                component_id: id.clone(),
                                item_id: item.id.clone(),
                            }),
                        }
                    } else {
                        KeyResult::Consumed
                    }
                }
                _ => KeyResult::Unhandled,
            }
        }

        Component::ActionList { id, items, .. } => {
            let idx = state.focused_component;
            let sel = state.selection_for(idx);
            match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    if sel > 0 {
                        state.component_selections[idx] = sel - 1;
                        KeyResult::Consumed
                    } else {
                        KeyResult::Unhandled
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if sel < items.len().saturating_sub(1) {
                        state.component_selections[idx] = sel + 1;
                        KeyResult::Consumed
                    } else {
                        KeyResult::Unhandled
                    }
                }
                KeyCode::Enter => {
                    if let Some(item) = items.get(sel) {
                        KeyResult::Action(UserAction::ListItemSelected {
                            component_id: id.clone(),
                            item_id: item.id.clone(),
                        })
                    } else {
                        KeyResult::Consumed
                    }
                }
                _ => KeyResult::Unhandled,
            }
        }

        Component::PinInput { id, .. } => match key {
            KeyCode::Char(c) => KeyResult::Action(UserAction::TextChanged {
                component_id: id.clone(),
                value: c.to_string(),
            }),
            KeyCode::Backspace => KeyResult::Action(UserAction::TextChanged {
                component_id: id.clone(),
                value: String::new(),
            }),
            _ => KeyResult::Unhandled,
        },

        Component::ConfirmationDialog { .. } => match key {
            KeyCode::Enter => KeyResult::Action(UserAction::ActionPressed {
                action_id: "confirm".to_string(),
            }),
            _ => KeyResult::Unhandled,
        },

        Component::Text { .. }
        | Component::InfoPanel { .. }
        | Component::StatusIndicator { .. }
        | Component::QrCode { .. }
        | Component::Divider => KeyResult::Unhandled,
    }
}

/// Map a key to a screen-level action (Enter for primary, `s` for skip, etc.).
fn map_action_key(key: KeyCode, screen: &ScreenModel) -> KeyResult {
    if screen.actions.is_empty() {
        return KeyResult::Unhandled;
    }

    match key {
        KeyCode::Enter => {
            // Find the primary action, or the first enabled action
            let action = screen
                .actions
                .iter()
                .find(|a| a.enabled && matches!(a.style, vauchi_core::ui::ActionStyle::Primary))
                .or_else(|| screen.actions.iter().find(|a| a.enabled));

            if let Some(action) = action {
                KeyResult::Action(UserAction::ActionPressed {
                    action_id: action.id.clone(),
                })
            } else {
                KeyResult::Unhandled
            }
        }
        KeyCode::Char('s') => {
            // Look for skip/secondary action
            let action = screen
                .actions
                .iter()
                .find(|a| a.enabled && (a.id == "skip" || a.id == "skip_to_finish"));

            if let Some(action) = action {
                KeyResult::Action(UserAction::ActionPressed {
                    action_id: action.id.clone(),
                })
            } else {
                KeyResult::Unhandled
            }
        }
        KeyCode::Char('b') => {
            let action = screen
                .actions
                .iter()
                .find(|a| a.enabled && (a.id == "restore_backup" || a.id == "setup_backup"));

            if let Some(action) = action {
                KeyResult::Action(UserAction::ActionPressed {
                    action_id: action.id.clone(),
                })
            } else {
                KeyResult::Unhandled
            }
        }
        KeyCode::Char('e') => {
            let action = screen.actions.iter().find(|a| a.enabled && a.id == "edit");

            if let Some(action) = action {
                KeyResult::Action(UserAction::ActionPressed {
                    action_id: action.id.clone(),
                })
            } else {
                KeyResult::Unhandled
            }
        }
        KeyCode::Char('S') => {
            let action = screen.actions.iter().find(|a| a.enabled && a.id == "scan");

            if let Some(action) = action {
                KeyResult::Action(UserAction::ActionPressed {
                    action_id: action.id.clone(),
                })
            } else {
                KeyResult::Unhandled
            }
        }
        KeyCode::Char('c') => {
            let action = screen
                .actions
                .iter()
                .find(|a| a.enabled && a.id == "create_new");

            if let Some(action) = action {
                KeyResult::Action(UserAction::ActionPressed {
                    action_id: action.id.clone(),
                })
            } else {
                KeyResult::Unhandled
            }
        }
        KeyCode::Char('h') => {
            let action = screen
                .actions
                .iter()
                .find(|a| a.enabled && a.id == "have_identity");

            if let Some(action) = action {
                KeyResult::Action(UserAction::ActionPressed {
                    action_id: action.id.clone(),
                })
            } else {
                KeyResult::Unhandled
            }
        }
        _ => KeyResult::Unhandled,
    }
}

// INLINE_TEST_REQUIRED: tests need access to pub(crate) key_mapping internals
#[cfg(test)]
mod tests {
    use super::*;
    use vauchi_core::ui::*;

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

    fn key_result_debug(result: &KeyResult) -> String {
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

    // ========================================================================
    // H4: 'c' key conflict — create_new vs legacy Contacts navigation
    // ========================================================================

    /// Verify 'c' triggers `create_new` when the action exists on the screen
    /// (e.g., Onboarding screen with a "Create new identity" action).
    #[test]
    fn test_c_key_triggers_create_new_when_action_exists() {
        let screen = make_screen(
            vec![Component::Text {
                id: "welcome".into(),
                content: "Welcome to Vauchi".into(),
                style: TextStyle::Body,
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
                key_result_debug(&other)
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
            key_result_debug(&result)
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
            key_result_debug(&result)
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
            }],
            vec![],
        );

        let mut state = ScreenRenderState::default();

        let result = map_key(KeyCode::Char('c'), &screen, &mut state);
        assert!(
            matches!(result, KeyResult::Unhandled),
            "'c' should be Unhandled on screen with no actions, got {}",
            key_result_debug(&result)
        );
    }
}
