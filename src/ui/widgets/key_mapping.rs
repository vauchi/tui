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

/// Map a key press to a `UserAction` or internal navigation update.
///
/// Returns `KeyResult::Action` when the key maps to a `UserAction`,
/// `KeyResult::Consumed` when it was handled internally (e.g., focus change),
/// or `KeyResult::Unhandled` when the key is not relevant.
pub fn map_key(key: KeyCode, screen: &ScreenModel, state: &mut ScreenRenderState) -> KeyResult {
    state.ensure_capacity(screen.components.len());

    match key {
        // Navigate focus between components
        KeyCode::Tab => {
            if !screen.components.is_empty() {
                state.focused_component = (state.focused_component + 1) % screen.components.len();
            }
            KeyResult::Consumed
        }
        KeyCode::BackTab => {
            if !screen.components.is_empty() {
                state.focused_component = if state.focused_component == 0 {
                    screen.components.len() - 1
                } else {
                    state.focused_component - 1
                };
            }
            KeyResult::Consumed
        }

        // Component-specific keys, then fall back to action keys
        _ => {
            if let Some(component) = screen.components.get(state.focused_component) {
                let result = map_component_key(key, component, state);
                if matches!(result, KeyResult::Unhandled) {
                    map_action_key(key, screen)
                } else {
                    result
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
            _ => KeyResult::Unhandled,
        },

        Component::ToggleList { id, items, .. } => {
            let idx = state.focused_component;
            let sel = state.selection_for(idx);
            match key {
                KeyCode::Up | KeyCode::Char('k') => {
                    if sel > 0 {
                        state.component_selections[idx] = sel - 1;
                    }
                    KeyResult::Consumed
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if sel < items.len().saturating_sub(1) {
                        state.component_selections[idx] = sel + 1;
                    }
                    KeyResult::Consumed
                }
                KeyCode::Char(' ') | KeyCode::Enter => {
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
                    }
                    KeyResult::Consumed
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if sel < fields.len().saturating_sub(1) {
                        state.component_selections[idx] = sel + 1;
                    }
                    KeyResult::Consumed
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
                    }
                    KeyResult::Consumed
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if sel < contacts.len().saturating_sub(1) {
                        state.component_selections[idx] = sel + 1;
                    }
                    KeyResult::Consumed
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
                    }
                    KeyResult::Consumed
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if sel < items.len().saturating_sub(1) {
                        state.component_selections[idx] = sel + 1;
                    }
                    KeyResult::Consumed
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
                    }
                    KeyResult::Consumed
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if sel < items.len().saturating_sub(1) {
                        state.component_selections[idx] = sel + 1;
                    }
                    KeyResult::Consumed
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
            KeyCode::Char(c) if c.is_ascii_digit() => KeyResult::Action(UserAction::TextChanged {
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
}
