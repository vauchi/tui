// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Per-component key mapping — translates key events to actions for each
//! `Component` variant (TextInput, ToggleList, FieldList, etc.).

use crossterm::event::KeyCode;

use vauchi_app::ui::{Component, UserAction};

use super::super::screen_renderer::ScreenRenderState;
use super::KeyResult;

/// Map a key to a component-specific action.
pub(super) fn map_component_key(
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
                            !matches!(field.visibility, vauchi_app::ui::UiFieldVisibility::Hidden);
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
                        state.ensure_visible(idx, 10);
                        KeyResult::Consumed
                    } else {
                        KeyResult::Unhandled
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if sel < contacts.len().saturating_sub(1) {
                        state.component_selections[idx] = sel + 1;
                        state.ensure_visible(idx, 10);
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
                            vauchi_app::ui::SettingsItemKind::Toggle { .. } => {
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

        Component::EditableText { id, value, .. } => match key {
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
            KeyCode::Enter => KeyResult::Action(UserAction::ActionPressed {
                action_id: format!("submit_{id}"),
            }),
            _ => KeyResult::Unhandled,
        },

        Component::InlineConfirm { id, .. } => match key {
            KeyCode::Enter => KeyResult::Action(UserAction::ActionPressed {
                action_id: id.clone(),
            }),
            KeyCode::Esc => KeyResult::Action(UserAction::ActionPressed {
                action_id: "cancel".to_string(),
            }),
            _ => KeyResult::Unhandled,
        },

        Component::Banner { action_id, .. } => match key {
            KeyCode::Enter if !action_id.is_empty() => {
                KeyResult::Action(UserAction::ActionPressed {
                    action_id: action_id.clone(),
                })
            }
            _ => KeyResult::Unhandled,
        },

        Component::Text { .. }
        | Component::InfoPanel { .. }
        | Component::StatusIndicator { .. }
        | Component::QrCode { .. }
        | Component::Divider => KeyResult::Unhandled,
        _ => KeyResult::Unhandled,
    }
}
