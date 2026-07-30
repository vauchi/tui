// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use super::presentation_protocol::PresentationState;
use vauchi_core::{
    AccessibilitySpec, ActionSpec, ActionTone, Command, ContextBar, InteractionId, OverlayKind,
    OverlaySpec, PaneLayout, PresentationProfile, PresentationTokens, SurfaceId, SurfaceLayout,
    SurfaceSpec, WindowClass,
};

fn action(id: &str) -> ActionSpec {
    ActionSpec {
        interaction_id: InteractionId::new(id).unwrap(),
        label: id.into(),
        accessibility_label: id.into(),
        icon_token: None,
        enabled: true,
        tone: ActionTone::Standard,
        shortcut: None,
    }
}

fn surface(revision: u64) -> SurfaceSpec {
    surface_for("surface-primary", revision)
}

fn surface_for(id: &str, revision: u64) -> SurfaceSpec {
    SurfaceSpec {
        surface_id: SurfaceId::new(id).unwrap(),
        revision,
        title: format!("Contacts {revision}"),
        subtitle: None,
        accessibility_label: "Contacts".into(),
        layout: SurfaceLayout::Scroll,
        tokens: PresentationTokens {
            spacing_small: 1,
            spacing_medium: 2,
            spacing_large: 3,
            corner_radius: 1,
            minimum_target_size: 1,
        },
        nodes: vec![vauchi_core::PresentationNode::Text {
            id: None,
            content: "Alice".into(),
            style: vauchi_core::PresentationTextStyle::Body,
            accessibility: AccessibilitySpec::label("Alice"),
        }],
    }
}

// @scenario: generic_presentation_protocol.feature :: Responsive transitions preserve interaction state
#[test]
fn responsive_profiles_preserve_both_surfaces_and_the_active_detail() {
    let mut state = PresentationState::default();
    let primary = surface_for("surface-primary", 7);
    let detail = surface_for("surface-detail", 2);
    state.apply(&[
        Command::ReplaceSurface {
            surface: primary.clone(),
        },
        Command::ReplaceSurface {
            surface: detail.clone(),
        },
        Command::SetPresentationProfile {
            profile: PresentationProfile {
                window_class: WindowClass::Expanded,
                pane_layout: PaneLayout::Split,
                primary_surface: primary.surface_id.clone(),
                detail_surface: Some(detail.surface_id.clone()),
                active_surface: detail.surface_id.clone(),
            },
        },
    ]);

    assert_eq!(state.visible_surfaces().len(), 2);
    assert_eq!(state.surface().unwrap().surface_id, detail.surface_id);

    state.apply(&[Command::SetPresentationProfile {
        profile: PresentationProfile {
            window_class: WindowClass::Compact,
            pane_layout: PaneLayout::Single,
            primary_surface: primary.surface_id.clone(),
            detail_surface: Some(detail.surface_id.clone()),
            active_surface: detail.surface_id.clone(),
        },
    }]);

    assert_eq!(state.visible_surfaces().len(), 1);
    assert_eq!(state.surface().unwrap().surface_id, detail.surface_id);
    assert_eq!(state.retained_surface_count(), 2);
}

// @scenario: generic_presentation_protocol.feature :: Every shell renders the same prepared presentation
#[test]
fn transaction_installs_surface_context_bar_and_overlay_atomically() {
    let mut state = PresentationState::default();
    let surface = surface(3);
    let surface_id = surface.surface_id.clone();

    let effects = state.apply(&[
        Command::ReplaceSurface { surface },
        Command::SetContextBar {
            surface_id: surface_id.clone(),
            revision: 3,
            bar: Box::new(ContextBar {
                back: Some(action("back")),
                navigation: Some(action("navigation")),
                primary: Some(action("primary")),
                secondary: Some(action("secondary")),
            }),
        },
        Command::PresentOverlay {
            surface_id,
            revision: 3,
            overlay: OverlaySpec {
                kind: OverlayKind::Navigation,
                title: Some("Navigate".into()),
                items: vec![action("open-primary")],
            },
        },
    ]);

    assert!(effects.is_empty());
    assert_eq!(state.surface().unwrap().revision, 3);
    assert_eq!(state.context_actions().len(), 4);
    assert_eq!(state.overlay().unwrap().kind, OverlayKind::Navigation);
}

// @scenario: generic_presentation_protocol.feature :: Invalid boundary input fails safely
#[test]
fn stale_chrome_is_rejected_and_effects_are_returned_to_the_shell() {
    let mut state = PresentationState::default();
    let current = surface(4);
    let surface_id = current.surface_id.clone();
    state.apply(&[Command::ReplaceSurface { surface: current }]);

    let effects = state.apply(&[
        Command::SetContextBar {
            surface_id: surface_id.clone(),
            revision: 3,
            bar: Box::new(ContextBar {
                primary: Some(action("stale")),
                ..ContextBar::default()
            }),
        },
        Command::PresentOverlay {
            surface_id,
            revision: 3,
            overlay: OverlaySpec {
                kind: OverlayKind::ActionMenu,
                title: None,
                items: vec![action("stale")],
            },
        },
        Command::ResetApplication,
    ]);

    assert!(state.context_actions().is_empty());
    assert!(state.overlay().is_none());
    assert_eq!(effects, vec![Command::ResetApplication]);
}

// @scenario: generic_presentation_protocol.feature :: Interaction activates its visible pane first
#[test]
fn activation_targets_the_surface_before_sending_the_opaque_interaction() {
    let mut state = PresentationState::default();
    let current = surface(1);
    let surface_id = current.surface_id.clone();
    state.apply(&[
        Command::ReplaceSurface { surface: current },
        Command::SetContextBar {
            surface_id: surface_id.clone(),
            revision: 1,
            bar: Box::new(ContextBar {
                primary: Some(action("opaque.primary")),
                ..ContextBar::default()
            }),
        },
    ]);

    assert_eq!(
        state.activate_context(0),
        vec![
            vauchi_core::Event::SurfaceActivated {
                surface_id: surface_id.clone(),
            },
            vauchi_core::Event::ActionActivated {
                surface_id,
                interaction_id: InteractionId::new("opaque.primary").unwrap(),
            },
        ]
    );
}
