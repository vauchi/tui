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
            KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                if group_views.is_empty() {
                    return KeyResult::Consumed;
                }
                // Find current index and cycle to next
                let current_idx = selected_group
                    .as_ref()
                    .and_then(|sg| group_views.iter().position(|g| &g.group_name == sg))
                    .unwrap_or(0);
                let next_idx = if key == KeyCode::Left {
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

        Component::Text { .. } | Component::InfoPanel { .. } | Component::Divider => {
            KeyResult::Unhandled
        }
    }
}

/// Map a key to a screen-level action (Enter for primary, 's' for skip, etc.).
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
