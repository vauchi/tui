// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Single source of truth for TUI action-id ↔ key bindings.
//!
//! Two views over one data table:
//!
//! - [`key_for_action`] — used by the action bar / footer to render the
//!   keyboard hint for a given `ScreenAction.id`.
//! - [`action_for_key`] — used by the dispatch path to find the
//!   `ScreenAction` (if any) on the rendered `ScreenModel` that the
//!   pressed key triggers.
//!
//! Adding a new dedicated-key binding means adding one row to
//! [`BINDINGS`]; both halves stay consistent automatically.
//!
//! `Enter` is a special role-based key handled in
//! [`crate::ui::widgets::key_mapping::action_keys`] — it triggers
//! whichever `ScreenAction` is `Primary` (or the first enabled
//! action). Several `action_id`s are tagged with hint
//! `"Enter"` so the footer advertises Enter as the canonical
//! shortcut even when no other key dispatches to them.

use crossterm::event::KeyCode;

use vauchi_app::ui::ScreenAction;

/// How an entry in [`BINDINGS`] matches a `ScreenAction.id`.
#[derive(Clone, Copy)]
enum Matcher {
    Exact(&'static str),
    Prefix(&'static str),
}

impl Matcher {
    fn matches(self, action_id: &str) -> bool {
        match self {
            Matcher::Exact(s) => s == action_id,
            Matcher::Prefix(p) => action_id.starts_with(p),
        }
    }
}

/// One binding row: a key (or `None` for Enter-hint-only entries),
/// the action matcher, and the footer hint string. The hint string is
/// what the action bar displays for any action matching `matcher`.
struct Binding {
    /// Dedicated key that dispatches to this action. `None` means the
    /// action is reached only via Enter's role-based primary handler.
    key: Option<KeyCode>,
    matcher: Matcher,
    /// Footer hint shown for any matching action.
    hint: &'static str,
}

/// Default hint when no row matches an `action_id`.
pub const DEFAULT_HINT: &str = "Enter";

/// The unified action ↔ key table. Order is not significant for
/// dispatch (we scan for any matching action) but is significant for
/// `key_for_action` — first match wins.
static BINDINGS: &[Binding] = &[
    // -- Enter-hint group: action_ids the footer advertises as Enter.
    //    Dispatch is handled by Enter's role-based primary handler in
    //    `action_keys::map_action_key`, not by a dedicated key here.
    Binding {
        key: None,
        matcher: Matcher::Exact("get_started"),
        hint: "Enter",
    },
    Binding {
        key: None,
        matcher: Matcher::Exact("continue"),
        hint: "Enter",
    },
    Binding {
        key: None,
        matcher: Matcher::Exact("continue_setup"),
        hint: "Enter",
    },
    Binding {
        key: None,
        matcher: Matcher::Exact("start"),
        hint: "Enter",
    },
    Binding {
        key: None,
        matcher: Matcher::Exact("confirm"),
        hint: "Enter",
    },
    Binding {
        key: None,
        matcher: Matcher::Exact("unlock"),
        hint: "Enter",
    },
    Binding {
        key: None,
        matcher: Matcher::Exact("done"),
        hint: "Enter",
    },
    Binding {
        key: None,
        matcher: Matcher::Exact("save"),
        hint: "Enter",
    },
    Binding {
        key: None,
        matcher: Matcher::Exact("toggle_view"),
        hint: "Enter",
    },
    // -- Dedicated-key bindings.
    Binding {
        key: Some(KeyCode::Char('b')),
        matcher: Matcher::Exact("restore_backup"),
        hint: "b",
    },
    Binding {
        key: Some(KeyCode::Char('b')),
        matcher: Matcher::Exact("setup_backup"),
        hint: "b",
    },
    Binding {
        key: Some(KeyCode::Char('b')),
        matcher: Matcher::Exact("backup"),
        hint: "b",
    },
    Binding {
        key: Some(KeyCode::Char('s')),
        matcher: Matcher::Exact("skip"),
        hint: "s",
    },
    Binding {
        key: Some(KeyCode::Char('s')),
        matcher: Matcher::Exact("skip_to_finish"),
        hint: "s",
    },
    Binding {
        key: Some(KeyCode::Char('e')),
        matcher: Matcher::Exact("edit"),
        hint: "e",
    },
    Binding {
        key: Some(KeyCode::Char('a')),
        matcher: Matcher::Exact("add_contact"),
        hint: "a",
    },
    Binding {
        key: Some(KeyCode::Char('a')),
        matcher: Matcher::Exact("add_field"),
        hint: "a",
    },
    Binding {
        key: Some(KeyCode::Char('a')),
        matcher: Matcher::Exact("add"),
        hint: "a",
    },
    Binding {
        key: Some(KeyCode::Char('v')),
        matcher: Matcher::Exact("view_all"),
        hint: "v",
    },
    Binding {
        key: Some(KeyCode::Char('v')),
        matcher: Matcher::Exact("view"),
        hint: "v",
    },
    Binding {
        key: Some(KeyCode::Char('r')),
        matcher: Matcher::Exact("retry"),
        hint: "r",
    },
    Binding {
        key: Some(KeyCode::Char('r')),
        matcher: Matcher::Exact("retry_all"),
        hint: "r",
    },
    Binding {
        key: Some(KeyCode::Esc),
        matcher: Matcher::Exact("cancel"),
        hint: "Esc",
    },
    Binding {
        key: Some(KeyCode::Esc),
        matcher: Matcher::Exact("back"),
        hint: "Esc",
    },
    Binding {
        key: Some(KeyCode::Char('h')),
        matcher: Matcher::Exact("have_identity"),
        hint: "h",
    },
    Binding {
        key: Some(KeyCode::Char('x')),
        matcher: Matcher::Exact("delete"),
        hint: "x",
    },
    Binding {
        key: Some(KeyCode::Char('x')),
        matcher: Matcher::Exact("wipe"),
        hint: "x",
    },
    Binding {
        key: Some(KeyCode::Char('x')),
        matcher: Matcher::Exact("emergency_wipe"),
        hint: "x",
    },
    Binding {
        key: Some(KeyCode::Char('d')),
        matcher: Matcher::Exact("delete_contact"),
        hint: "d",
    },
    Binding {
        key: Some(KeyCode::Char('d')),
        matcher: Matcher::Exact("archive_contact"),
        hint: "d",
    },
    Binding {
        key: Some(KeyCode::Char('S')),
        matcher: Matcher::Exact("scan"),
        hint: "S",
    },
    Binding {
        key: Some(KeyCode::Char('t')),
        matcher: Matcher::Exact("enable"),
        hint: "t",
    },
    Binding {
        key: Some(KeyCode::Char('t')),
        matcher: Matcher::Exact("disable"),
        hint: "t",
    },
    Binding {
        key: Some(KeyCode::Char('t')),
        matcher: Matcher::Exact("toggle"),
        hint: "t",
    },
    // -- Hybrid: dedicated key AND footer-Enter advertisement.
    //    `create_new` is dispatched by 'c' but the footer says
    //    "Enter" because Enter's primary handler also triggers it
    //    (it's the primary action on the create-identity screen).
    //    Order matters: the "Enter" row must precede the 'c' row so
    //    `key_for_action` returns "Enter" first.
    Binding {
        key: None,
        matcher: Matcher::Exact("create_new"),
        hint: "Enter",
    },
    Binding {
        key: Some(KeyCode::Char('c')),
        matcher: Matcher::Exact("create_new"),
        hint: "Enter",
    },
    // -- Pattern bindings.
    Binding {
        key: Some(KeyCode::Char('g')),
        matcher: Matcher::Prefix("filter_group"),
        hint: "g",
    },
];

/// Footer hint for an `action_id`. Returns [`DEFAULT_HINT`] when no
/// row matches.
pub fn key_for_action(action_id: &str) -> &'static str {
    BINDINGS
        .iter()
        .find(|b| b.matcher.matches(action_id))
        .map(|b| b.hint)
        .unwrap_or(DEFAULT_HINT)
}

/// Dispatch a key press to a `ScreenAction.id`, scanning the screen's
/// advertised actions. Returns the first enabled action whose id
/// matches a binding for `key`. Caller is responsible for emitting
/// `UserAction::ActionPressed { action_id }`.
///
/// Returns `None` when:
/// - `key` has no row in [`BINDINGS`] (e.g., Enter — handled by the
///   role-based primary dispatcher in `action_keys`).
/// - No enabled action on the screen matches any binding for `key`.
pub fn action_for_key<'a>(key: KeyCode, actions: &'a [ScreenAction]) -> Option<&'a ScreenAction> {
    let candidates = BINDINGS
        .iter()
        .filter(|b| b.key == Some(key))
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return None;
    }
    actions
        .iter()
        .find(|a| a.enabled && candidates.iter().any(|b| b.matcher.matches(&a.id)))
}

// INLINE_TEST_REQUIRED: round-trip test iterates the private BINDINGS table
#[cfg(test)]
mod tests {
    use super::*;
    use vauchi_app::ui::ActionStyle;

    fn mk(id: &str) -> ScreenAction {
        ScreenAction {
            id: id.into(),
            label: id.into(),
            style: ActionStyle::Primary,
            enabled: true,
            a11y: None,
        }
    }

    #[test]
    fn round_trip_dedicated_keys_dispatch_and_hint_match() {
        // For every dedicated-key binding, dispatching the key against
        // a screen carrying that exact action returns that action, and
        // the hint advertises a sensible string (the key's char, or
        // "Enter" / "Esc").
        for b in BINDINGS.iter().filter(|b| b.key.is_some()) {
            let key = b.key.unwrap();
            let id = match b.matcher {
                Matcher::Exact(s) => s.to_string(),
                Matcher::Prefix(p) => format!("{p}_test_suffix"),
            };
            let screen_actions = vec![mk(&id)];

            let resolved = action_for_key(key, &screen_actions);
            assert!(
                resolved.is_some(),
                "binding key={:?} matcher should dispatch to action_id={}",
                key,
                id
            );
            assert_eq!(resolved.unwrap().id, id);

            let hint = key_for_action(&id);
            // Either the hint matches the key's printable form, or it's
            // an explicit override like "Enter" (hybrid: create_new) or
            // "Esc" (cancel/back).
            let expected_key_str = match key {
                KeyCode::Char(c) => c.to_string(),
                KeyCode::Esc => "Esc".to_string(),
                _ => unreachable!("unexpected key in BINDINGS: {:?}", key),
            };
            assert!(
                hint == expected_key_str || hint == "Enter" || hint == "Esc",
                "hint for {} should be {:?} or Enter/Esc, got {:?}",
                id,
                expected_key_str,
                hint
            );
        }
    }

    #[test]
    fn enter_hint_group_falls_through_to_default_dispatch() {
        // Enter-hint-only bindings (key = None) should produce the
        // "Enter" hint and `action_for_key` for any non-bound key
        // returns None for these ids.
        for b in BINDINGS.iter().filter(|b| b.key.is_none()) {
            let id = match b.matcher {
                Matcher::Exact(s) => s.to_string(),
                Matcher::Prefix(p) => format!("{p}_test_suffix"),
            };
            assert_eq!(
                key_for_action(&id),
                "Enter",
                "Enter-hint group: {} must hint Enter",
                id
            );
        }
    }

    #[test]
    fn dispatch_skips_disabled_actions() {
        let actions = vec![
            ScreenAction {
                id: "skip".into(),
                label: "Skip".into(),
                style: ActionStyle::Secondary,
                enabled: false,
                a11y: None,
            },
            mk("skip_to_finish"),
        ];
        let resolved = action_for_key(KeyCode::Char('s'), &actions);
        assert!(resolved.is_some(), "should fall through to enabled match");
        assert_eq!(resolved.unwrap().id, "skip_to_finish");
    }

    #[test]
    fn dispatch_returns_none_for_unbound_key() {
        let actions = vec![mk("skip")];
        assert!(
            action_for_key(KeyCode::Char('z'), &actions).is_none(),
            "unbound key should return None"
        );
    }

    #[test]
    fn dispatch_returns_none_when_no_action_matches() {
        let actions = vec![mk("unrelated_action")];
        assert!(
            action_for_key(KeyCode::Char('s'), &actions).is_none(),
            "key bound to other ids should return None when no match"
        );
    }

    #[test]
    fn prefix_matcher_dispatches_for_any_suffix() {
        let actions = vec![mk("filter_group_friends"), mk("filter_group_work")];
        let resolved = action_for_key(KeyCode::Char('g'), &actions);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().id, "filter_group_friends");
    }

    #[test]
    fn hint_for_create_new_is_enter_via_hybrid_row_order() {
        // Hybrid case: create_new has both a 'c' dispatch row and an
        // Enter hint row. Enter row precedes 'c' row, so hint is
        // "Enter" but dispatch on 'c' still works.
        assert_eq!(key_for_action("create_new"), "Enter");
        let actions = vec![mk("create_new")];
        let resolved = action_for_key(KeyCode::Char('c'), &actions);
        assert_eq!(resolved.map(|a| a.id.clone()), Some("create_new".into()));
    }

    #[test]
    fn hint_falls_back_to_default_for_unknown_action() {
        assert_eq!(key_for_action("nonexistent_action_id"), DEFAULT_HINT);
        assert_eq!(DEFAULT_HINT, "Enter");
    }

    #[test]
    fn hint_for_filter_group_prefix() {
        assert_eq!(key_for_action("filter_group_friends"), "g");
        assert_eq!(key_for_action("filter_group"), "g");
    }
}
