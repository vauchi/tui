// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Screen-level action key mapping — maps shortcut keys (`Enter`, `s`, `b`, etc.)
//! to `ScreenAction`s advertised by the current screen.

use crossterm::event::KeyCode;

use vauchi_core::ui::{ScreenModel, UserAction};

use super::KeyResult;

/// Map a key to a screen-level action (Enter for primary, `s` for skip, etc.).
pub(super) fn map_action_key(key: KeyCode, screen: &ScreenModel) -> KeyResult {
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
