// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Screen-level action key mapping — maps shortcut keys (`Enter`,
//! `s`, `b`, etc.) to `ScreenAction`s advertised by the current
//! screen and to global chrome actions in `nav_actions`.
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
/// skip, etc.). Falls through to [`map_nav_action_key`] for chrome
/// actions advertised in `nav_actions` (ADR-044 Am2a).
pub(super) fn map_action_key(key: KeyCode, screen: &ScreenModel) -> KeyResult {
    if !screen.actions.is_empty() {
        if matches!(key, KeyCode::Enter) {
            let result = enter_dispatch(screen);
            if !matches!(result, KeyResult::Unhandled) {
                return result;
            }
        }

        if let Some(action) = action_for_key(key, &screen.actions) {
            return KeyResult::Action(UserAction::ActionPressed {
                action_id: action.id.clone(),
            });
        }
    }

    map_nav_action_key(key, screen)
}

/// Map a key to a global chrome action from `ScreenModel::nav_actions`
/// (e.g. the back affordance or the settings gear on the home screen).
///
/// The reserved `go_back` id dispatches the system-back gesture so the
/// visible chrome and the Escape key share one code path. All other nav
/// actions forward as `ActionPressed` so core can resolve them.
pub(super) fn map_nav_action_key(key: KeyCode, screen: &ScreenModel) -> KeyResult {
    use super::action_table::key_for_action;

    let target = screen.nav_actions.iter().find(|a| {
        if !a.enabled {
            return false;
        }
        let hint = key_for_action(&a.id);
        // The back affordance is always bound to Escape in the TUI.
        if a.id == "go_back" {
            return key == KeyCode::Esc;
        }
        key_to_hint(key) == hint
    });

    match target {
        Some(action) if action.id == "go_back" => KeyResult::Action(UserAction::NavigateBack),
        Some(action) => KeyResult::Action(UserAction::ActionPressed {
            action_id: action.id.clone(),
        }),
        None => KeyResult::Unhandled,
    }
}

/// Convert a `KeyCode` to the short hint string used in `action_table`.
fn key_to_hint(key: KeyCode) -> &'static str {
    match key {
        KeyCode::Char(c) => {
            // Safety: we only ever bind single ASCII chars in action_table.
            // Returning a static slice avoids an allocation on every lookup.
            match c {
                'a' => "a",
                'b' => "b",
                'c' => "c",
                'd' => "d",
                'e' => "e",
                'g' => "g",
                'h' => "h",
                'i' => "i",
                'n' => "n",
                'o' => "o",
                'r' => "r",
                's' => "s",
                't' => "t",
                'v' => "v",
                'x' => "x",
                '!' => "!",
                'S' => "S",
                _ => "",
            }
        }
        KeyCode::Esc => "Esc",
        _ => "",
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
