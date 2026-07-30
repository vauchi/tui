// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use super::presentation_input::{InteractionState, KeyOutcome};
use super::presentation_protocol::PresentationState;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use vauchi_core::{
    AccessibilitySpec, ActionSpec, ActionTone, BindingId, Command, ContextBar, Event, InputValue,
    InteractionId, OverlayKind, OverlaySpec, PresentationInputKind, PresentationNode,
    PresentationTokens, StandardShortcut, SurfaceId, SurfaceLayout, SurfaceSpec,
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
