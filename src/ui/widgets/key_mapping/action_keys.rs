// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Screen-level action key mapping — maps shortcut keys (`Enter`,
//! `s`, `b`, etc.) to `ScreenAction`s advertised by the current
//! screen.
//!
//! `Enter` is special: it dispatches by *role* (the action with
//! `ActionStyle::Primary`, falling back to the first enabled
//! action). All other keys delegate to the unified table in
//! [`super::action_table`], which keeps dispatch and footer hints
//! in sync.

use crossterm::event::KeyCode;

use vauchi_app::ui::{ActionStyle, ScreenModel, UserAction};

use super::KeyResult;
use super::action_table::action_for_key;

/// Map a key to a screen-level action (Enter for primary, `s` for
/// skip, etc.).
pub(super) fn map_action_key(key: KeyCode, screen: &ScreenModel) -> KeyResult {
    if screen.actions.is_empty() {
        return KeyResult::Unhandled;
    }

    if matches!(key, KeyCode::Enter) {
        return enter_dispatch(screen);
    }

    match action_for_key(key, &screen.actions) {
        Some(action) => KeyResult::Action(UserAction::ActionPressed {
            action_id: action.id.clone(),
        }),
        None => KeyResult::Unhandled,
    }
}

/// Enter dispatches the primary action, falling back to the first
/// enabled action. Role-based — not driven by `action_id`.
fn enter_dispatch(screen: &ScreenModel) -> KeyResult {
    let action = screen
        .actions
        .iter()
        .find(|a| a.enabled && matches!(a.style, ActionStyle::Primary))
        .or_else(|| screen.actions.iter().find(|a| a.enabled));

    match action {
        Some(a) => KeyResult::Action(UserAction::ActionPressed {
            action_id: a.id.clone(),
        }),
        None => KeyResult::Unhandled,
    }
}
