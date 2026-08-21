// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use super::presentation_input::{InteractionState, KeyOutcome};
use super::presentation_protocol::PresentationState;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use vauchi_core::{
    AccessibilitySpec, ActionSpec, ActionTone, BindingId, Command, ContextBar, Event, InputValue,
    InteractionId, OverlayKind, OverlaySpec, PresentationInputKind, PresentationNode,
    PresentationRow, PresentationTokens, StandardShortcut, SurfaceId, SurfaceLayout, SurfaceSpec,
};

fn action(id: &str, shortcut: Option<StandardShortcut>) -> ActionSpec {
    ActionSpec {
        interaction_id: InteractionId::new(id).unwrap(),
        label: id.into(),
        accessibility_label: id.into(),
        icon_token: None,
        enabled: true,
        tone: ActionTone::Standard,
        shortcut,
    }
}

fn row(title: &str, activation: Option<ActionSpec>) -> PresentationRow {
    PresentationRow {
        title: title.into(),
        subtitle: None,
        detail: None,
        icon_token: None,
        image_data: None,
        fallback_text: None,
        selected: false,
        enabled: true,
        activation,
        secondary_actions: Vec::new(),
        controls: Vec::new(),
        accessibility: AccessibilitySpec::label(title),
    }
}

fn state_with_input() -> PresentationState {
    let surface_id = SurfaceId::new("onboarding").unwrap();
    let mut state = PresentationState::default();
    state.apply(&[
        Command::ReplaceSurface {
            surface: SurfaceSpec {
                surface_id: surface_id.clone(),
                revision: 1,
                title: "Welcome".into(),
                subtitle: None,
                accessibility_label: "Welcome".into(),
                layout: SurfaceLayout::Fixed,
                tokens: PresentationTokens {
                    spacing_small: 1,
                    spacing_medium: 2,
                    spacing_large: 3,
                    corner_radius: 1,
                    minimum_target_size: 1,
                },
                nodes: vec![PresentationNode::Input {
                    binding_id: BindingId::new("display-name").unwrap(),
                    label: "Name".into(),
                    value: "Al".into(),
                    placeholder: None,
                    input_kind: PresentationInputKind::Text,
                    max_length: Some(80),
                    validation_error: None,
                    enabled: true,
                    accessibility: AccessibilitySpec::label("Name"),
                }],
            },
        },
        Command::SetContextBar {
            surface_id,
            revision: 1,
            bar: Box::new(ContextBar {
                back: Some(action("back", Some(StandardShortcut::Back))),
                navigation: Some(action("navigation", None)),
                primary: Some(action("continue", Some(StandardShortcut::ActivatePrimary))),
                secondary: Some(action("secondary", None)),
            }),
        },
    ]);
    state
}

// @scenario: generic_presentation_protocol.feature :: User interaction returns as an opaque event
#[test]
fn text_input_reports_surface_activation_before_raw_value_change() {
    let state = state_with_input();
    let mut interaction = InteractionState::default();

    assert_eq!(
        interaction.key_outcome(
            &state,
            KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE),
        ),
        KeyOutcome::Events(vec![
            Event::SurfaceActivated {
                surface_id: SurfaceId::new("onboarding").unwrap(),
            },
            Event::ValueChanged {
                surface_id: SurfaceId::new("onboarding").unwrap(),
                binding_id: BindingId::new("display-name").unwrap(),
                value: InputValue::Text("Ali".into()),
            },
        ])
    );
}

// @scenario: generic_presentation_protocol.feature :: User interaction returns as an opaque event
#[test]
fn keyboard_shortcuts_resolve_only_core_minted_context_actions() {
    let state = state_with_input();
    let mut interaction = InteractionState::default();

    for (key, expected) in [
        (
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
            "continue",
        ),
        (
            KeyEvent::new(KeyCode::Char('m'), KeyModifiers::ALT),
            "navigation",
        ),
        (
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::ALT),
            "secondary",
        ),
    ] {
        let KeyOutcome::Events(events) = interaction.key_outcome(&state, key) else {
            panic!("expected events");
        };
        assert!(matches!(
            events.as_slice(),
            [
                Event::SurfaceActivated { .. },
                Event::ActionActivated { interaction_id, .. }
            ] if interaction_id.as_str() == expected
        ));
    }
}

// @scenario: generic_presentation_protocol.feature :: Overlay kinds remain distinct with reduced motion
#[test]
fn escape_dismisses_each_overlay_by_kind_through_core() {
    for kind in [OverlayKind::Navigation, OverlayKind::ActionMenu] {
        let mut state = state_with_input();
        state.apply(&[Command::PresentOverlay {
            surface_id: SurfaceId::new("onboarding").unwrap(),
            revision: 1,
            overlay: OverlaySpec {
                kind,
                title: None,
                items: vec![action("item", None)],
            },
        }]);
        let mut interaction = InteractionState::default();

        assert_eq!(
            interaction.key_outcome(&state, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),),
            KeyOutcome::Events(vec![
                Event::SurfaceActivated {
                    surface_id: SurfaceId::new("onboarding").unwrap(),
                },
                Event::OverlayDismissed {
                    surface_id: SurfaceId::new("onboarding").unwrap(),
                    kind,
                },
            ])
        );
    }
}

fn state_with_action_list() -> PresentationState {
    let surface_id = SurfaceId::new("exchange_mode_selection").unwrap();
    let mut state = PresentationState::default();
    state.apply(&[Command::ReplaceSurface {
        surface: SurfaceSpec {
            surface_id: surface_id.clone(),
            revision: 1,
            title: "Exchange Mode".into(),
            subtitle: Some("Pick a way to connect".into()),
            accessibility_label: "Exchange Mode".into(),
            layout: SurfaceLayout::Scroll,
            tokens: PresentationTokens {
                spacing_small: 1,
                spacing_medium: 2,
                spacing_large: 3,
                corner_radius: 1,
                minimum_target_size: 1,
            },
            nodes: vec![PresentationNode::List {
                id: BindingId::new("modes").unwrap(),
                label: None,
                rows: vec![
                    PresentationRow {
                        title: "Glance".into(),
                        subtitle: Some("Show a QR code".into()),
                        detail: None,
                        icon_token: Some("qrcode".into()),
                        image_data: None,
                        fallback_text: None,
                        selected: false,
                        enabled: true,
                        activation: Some(action("mode:glance", None)),
                        secondary_actions: Vec::new(),
                        controls: Vec::new(),
                        accessibility: AccessibilitySpec::label("Glance"),
                    },
                    PresentationRow {
                        title: "Link".into(),
                        subtitle: Some("Share a link".into()),
                        detail: None,
                        icon_token: Some("link".into()),
                        image_data: None,
                        fallback_text: None,
                        selected: false,
                        enabled: true,
                        activation: Some(action("mode:link", None)),
                        secondary_actions: Vec::new(),
                        controls: Vec::new(),
                        accessibility: AccessibilitySpec::label("Link"),
                    },
                ],
                searchable: false,
                paging: None,
                accessibility: AccessibilitySpec::label("Exchange modes"),
            }],
        },
    }]);
    state
}

// @scenario: contact_exchange.feature :: User selects an exchange mode from the TUI
#[test]
fn down_selects_first_surface_list_row() {
    let state = state_with_action_list();
    let mut interaction = InteractionState::default();

    assert_eq!(
        interaction.key_outcome(&state, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
        KeyOutcome::Consumed
    );
    assert_eq!(interaction.selected_surface_row(&state), Some(0));
}

// @scenario: contact_exchange.feature :: User activates a surface list row with Enter
#[test]
fn enter_activates_selected_surface_list_row() {
    let state = state_with_action_list();
    let mut interaction = InteractionState::default();

    interaction.key_outcome(&state, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    let KeyOutcome::Events(events) =
        interaction.key_outcome(&state, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
    else {
        panic!("expected events");
    };
    assert!(matches!(
        events.as_slice(),
        [
            Event::SurfaceActivated { .. },
            Event::ActionActivated { interaction_id, .. }
        ] if interaction_id.as_str() == "mode:glance"
    ));
}

// @scenario: generic_presentation_protocol.feature :: Contextual controls expose four stable roles
#[test]
fn tab_keeps_cycling_context_actions_while_a_list_is_present() {
    let mut state = state_with_action_list();
    state.apply(&[Command::SetContextBar {
        surface_id: SurfaceId::new("exchange_mode_selection").unwrap(),
        revision: 1,
        bar: Box::new(ContextBar {
            back: Some(action("back", Some(StandardShortcut::Back))),
            navigation: Some(action("navigation", None)),
            primary: Some(action("continue", None)),
            secondary: None,
        }),
    }]);
    let mut interaction = InteractionState::default();

    assert_eq!(
        interaction.key_outcome(&state, KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
        KeyOutcome::Consumed
    );
    assert_eq!(
        interaction.selected_context(),
        1,
        "a list on the surface must not take Tab away from the context bar"
    );
    assert_eq!(interaction.selected_surface_row(&state), None);
}

fn state_with_input_and_action_list() -> PresentationState {
    let surface_id = SurfaceId::new("contact_search").unwrap();
    let mut state = PresentationState::default();
    state.apply(&[Command::ReplaceSurface {
        surface: SurfaceSpec {
            surface_id,
            revision: 1,
            title: "Search".into(),
            subtitle: None,
            accessibility_label: "Search".into(),
            layout: SurfaceLayout::Scroll,
            tokens: PresentationTokens {
                spacing_small: 1,
                spacing_medium: 2,
                spacing_large: 3,
                corner_radius: 1,
                minimum_target_size: 1,
            },
            nodes: vec![
                PresentationNode::Input {
                    binding_id: BindingId::new("query").unwrap(),
                    label: "Phone".into(),
                    value: "+4".into(),
                    placeholder: None,
                    input_kind: PresentationInputKind::Text,
                    max_length: Some(80),
                    validation_error: None,
                    enabled: true,
                    accessibility: AccessibilitySpec::label("Phone"),
                },
                PresentationNode::List {
                    id: BindingId::new("results").unwrap(),
                    label: None,
                    rows: vec![row("Ada", Some(action("open:ada", None)))],
                    searchable: false,
                    paging: None,
                    accessibility: AccessibilitySpec::label("Results"),
                },
            ],
        },
    }]);
    state
}

// @scenario: generic_presentation_protocol.feature :: User interaction returns as an opaque event
#[test]
fn digits_reach_a_focused_input_instead_of_activating_a_row() {
    let state = state_with_input_and_action_list();
    let mut interaction = InteractionState::default();

    let KeyOutcome::Events(events) = interaction.key_outcome(
        &state,
        KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE),
    ) else {
        panic!("expected the digit to edit the focused input");
    };
    assert!(
        matches!(
            events.as_slice(),
            [
                Event::SurfaceActivated { .. },
                Event::ValueChanged {
                    value: InputValue::Text(value),
                    ..
                }
            ] if value == "+41"
        ),
        "typing a digit into a text field must not activate a list row: {events:?}"
    );
}

// @scenario: generic_presentation_protocol.feature :: User interaction returns as an opaque event
#[test]
fn surface_replacement_clears_a_stale_row_selection() {
    let mut state = state_with_action_list();
    let mut interaction = InteractionState::default();

    interaction.key_outcome(
        &state,
        KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE),
    );
    assert_eq!(interaction.selected_surface_row(&state), Some(1));

    state.apply(&[Command::ReplaceSurface {
        surface: SurfaceSpec {
            surface_id: SurfaceId::new("exchange_progress").unwrap(),
            revision: 1,
            title: "Connecting".into(),
            subtitle: None,
            accessibility_label: "Connecting".into(),
            layout: SurfaceLayout::Fixed,
            tokens: PresentationTokens {
                spacing_small: 1,
                spacing_medium: 2,
                spacing_large: 3,
                corner_radius: 1,
                minimum_target_size: 1,
            },
            nodes: vec![PresentationNode::List {
                id: BindingId::new("steps").unwrap(),
                label: None,
                rows: vec![row("Retry", Some(action("exchange:retry", None)))],
                searchable: false,
                paging: None,
                accessibility: AccessibilitySpec::label("Steps"),
            }],
        },
    }]);

    assert_eq!(
        interaction.selected_surface_row(&state),
        None,
        "a selection index from the previous surface must not survive navigation"
    );
}

// @scenario: contact_exchange.feature :: User activates a surface list row by number
#[test]
fn digit_activates_surface_list_row_directly() {
    let state = state_with_action_list();
    let mut interaction = InteractionState::default();

    let KeyOutcome::Events(events) = interaction.key_outcome(
        &state,
        KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE),
    ) else {
        panic!("expected events");
    };
    assert!(matches!(
        events.as_slice(),
        [
            Event::SurfaceActivated { .. },
            Event::ActionActivated { interaction_id, .. }
        ] if interaction_id.as_str() == "mode:link"
    ));
    assert_eq!(interaction.selected_surface_row(&state), Some(1));
}

/// Return in a field is the terminal's submit gesture — there is no
/// pointer to click away with, so it is the only one available. Until a
/// field has been typed into, Return keeps activating the primary
/// action, which `keyboard_shortcuts_resolve_only_core_minted_context_actions`
/// pins.
// @scenario: generic_presentation_protocol.feature :: Return in a field reports a submission
#[test]
fn enter_reports_submission_once_a_field_has_focus() {
    let state = state_with_input();
    let mut interaction = InteractionState::default();

    // Typing focuses the field.
    let KeyOutcome::Events(_) = interaction.key_outcome(
        &state,
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
    ) else {
        panic!("typing must produce events");
    };

    let KeyOutcome::Events(events) =
        interaction.key_outcome(&state, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))
    else {
        panic!("expected events");
    };
    assert!(
        matches!(
            events.as_slice(),
            [Event::InputSubmitted { binding_id, .. }]
                if binding_id.as_str() == "display-name"
        ),
        "Return must report the focused field, got {events:?}"
    );
}

/// Home and End reach the ends of a list without holding a key down.
///
/// With 200 contacts, Up and Down alone make the far end of the list a
/// war of attrition.
// @scenario: contact_exchange.feature :: User activates a surface list row with Enter
#[test]
fn home_and_end_jump_to_the_ends_of_the_list() {
    let state = state_with_action_list();
    let last = state.surface_list_rows().len() - 1;
    let mut interaction = InteractionState::default();

    assert_eq!(
        interaction.key_outcome(&state, KeyEvent::new(KeyCode::End, KeyModifiers::NONE)),
        KeyOutcome::Consumed
    );
    assert_eq!(interaction.selected_surface_row(&state), Some(last));

    assert_eq!(
        interaction.key_outcome(&state, KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)),
        KeyOutcome::Consumed
    );
    assert_eq!(interaction.selected_surface_row(&state), Some(0));
}

/// PageUp and PageDown move by a fixed row step and stop at the ends.
///
/// Deliberately a row count, not a screenful: a row occupies one to three
/// lines depending on whether Core gave it a subtitle and a detail, so the
/// input layer cannot know how many rows a viewport holds without the
/// renderer's line map. A predictable jump beats a guessed one.
// @scenario: contact_exchange.feature :: User activates a surface list row with Enter
#[test]
fn page_keys_move_by_a_step_and_clamp_at_the_ends() {
    let state = state_with_action_list();
    let last = state.surface_list_rows().len() - 1;
    let mut interaction = InteractionState::default();

    // From nothing, PageDown starts at the top rather than jumping blind.
    assert_eq!(
        interaction.key_outcome(&state, KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE)),
        KeyOutcome::Consumed
    );
    assert_eq!(interaction.selected_surface_row(&state), Some(0));

    // A page beyond the end clamps to the last row instead of wrapping —
    // wrapping past the end is how a user loses their place.
    interaction.key_outcome(&state, KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
    assert_eq!(interaction.selected_surface_row(&state), Some(last));

    interaction.key_outcome(&state, KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE));
    assert_eq!(interaction.selected_surface_row(&state), Some(0));
}
