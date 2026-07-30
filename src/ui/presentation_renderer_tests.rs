// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use super::presentation_protocol::PresentationState;
use super::presentation_renderer;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use vauchi_core::{
    AccessibilitySpec, ActionSpec, ActionTone, Command, ContextBar, InteractionId, PaneLayout,
    PresentationNode, PresentationProfile, PresentationTextStyle, PresentationTokens, SurfaceId,
    SurfaceLayout, SurfaceSpec, WindowClass,
};

fn action(id: &str, label: &str) -> ActionSpec {
    ActionSpec {
        interaction_id: InteractionId::new(id).unwrap(),
        label: label.into(),
        accessibility_label: label.into(),
        icon_token: None,
        enabled: true,
        tone: ActionTone::Standard,
        shortcut: None,
    }
}

fn titled_surface(id: &str, title: &str) -> SurfaceSpec {
    SurfaceSpec {
        surface_id: SurfaceId::new(id).unwrap(),
        revision: 1,
        title: title.into(),
        subtitle: None,
        accessibility_label: title.into(),
        layout: SurfaceLayout::Scroll,
        tokens: PresentationTokens {
            spacing_small: 1,
            spacing_medium: 2,
            spacing_large: 3,
            corner_radius: 1,
            minimum_target_size: 1,
        },
        nodes: vec![],
    }
}

// @scenario: generic_presentation_protocol.feature :: Contextual controls expose four stable roles
#[test]
fn renderer_keeps_content_humble_and_contextual_roles_in_one_bottom_strip() {
    let surface_id = SurfaceId::new("surface-primary").unwrap();
    let mut state = PresentationState::default();
    state.apply(&[
        Command::ReplaceSurface {
            surface: SurfaceSpec {
                surface_id: surface_id.clone(),
                revision: 1,
                title: "Contacts".into(),
                subtitle: Some("Core prepared".into()),
                accessibility_label: "Contacts".into(),
                layout: SurfaceLayout::Scroll,
                tokens: PresentationTokens {
                    spacing_small: 1,
                    spacing_medium: 2,
                    spacing_large: 3,
                    corner_radius: 1,
                    minimum_target_size: 1,
                },
                nodes: vec![PresentationNode::Text {
                    id: None,
                    content: "Alice".into(),
                    style: PresentationTextStyle::Body,
                    accessibility: AccessibilitySpec::label("Alice"),
                }],
            },
        },
        Command::SetContextBar {
            surface_id,
            revision: 1,
            bar: Box::new(ContextBar {
                back: Some(action("back", "Back")),
                navigation: Some(action("nav", "Navigate")),
                primary: Some(action("primary", "Continue")),
                secondary: Some(action("secondary", "More")),
            }),
        },
    ]);
    let backend = TestBackend::new(80, 16);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| presentation_renderer::draw(frame, frame.area(), &state, 2))
        .unwrap();

    let buffer = terminal.backend().buffer();
    let rendered = buffer
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    for expected in [
        "Contacts",
        "Core prepared",
        "Alice",
        "< Back",
        "≡ Navigate",
        "[ Continue ]",
        "… More",
    ] {
        assert!(rendered.contains(expected), "missing {expected:?}");
    }
}

// @scenario: generic_presentation_protocol.feature :: Available window drives structural composition
#[test]
fn expanded_profile_renders_primary_and_detail_as_two_native_panes() {
    let primary = titled_surface("surface-primary", "Contacts");
    let detail = titled_surface("surface-detail", "Alice");
    let mut state = PresentationState::default();
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
                primary_surface: primary.surface_id,
                detail_surface: Some(detail.surface_id.clone()),
                active_surface: detail.surface_id,
            },
        },
    ]);
    let backend = TestBackend::new(100, 18);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| presentation_renderer::draw(frame, frame.area(), &state, 0))
        .unwrap();

    let rendered = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains("Contacts"));
    assert!(rendered.contains("Alice"));
}
