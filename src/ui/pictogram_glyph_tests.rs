// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! A terminal cannot draw Core's pictograms, so a `pictogram.*` icon token
//! renders as that pictogram's single-width glyph, the same one the CLI uses
//! (#473). Other icon tokens name native symbols the terminal has no
//! equivalent for: they render nothing, never their own name.

use super::presentation_protocol::PresentationState;
use super::presentation_renderer;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use vauchi_core::{
    AccessibilitySpec, ActionSpec, ActionTone, BindingId, Command, InteractionId,
    PresentationListStyle, PresentationNode, PresentationRow, PresentationTokens, PresentationTone,
    SurfaceId, SurfaceLayout, SurfaceSpec,
};

fn surface(nodes: Vec<PresentationNode>) -> SurfaceSpec {
    SurfaceSpec {
        surface_id: SurfaceId::new("exchange").unwrap(),
        revision: 1,
        title: "Exchange".into(),
        subtitle: None,
        accessibility_label: "Exchange".into(),
        layout: SurfaceLayout::Scroll,
        tokens: PresentationTokens {
            spacing_small: 1,
            spacing_medium: 2,
            spacing_large: 3,
            corner_radius: 1,
            minimum_target_size: 1,
        },
        nodes,
    }
}

fn row(title: &str, icon_token: &str) -> PresentationRow {
    PresentationRow {
        title: title.into(),
        subtitle: None,
        detail: None,
        icon_token: Some(icon_token.into()),
        image_data: None,
        fallback_text: None,
        selected: false,
        enabled: true,
        activation: Some(ActionSpec {
            interaction_id: InteractionId::new(&format!("pick-{title}")).unwrap(),
            label: title.into(),
            accessibility_label: title.into(),
            icon_token: None,
            enabled: true,
            tone: ActionTone::Standard,
            shortcut: None,
        }),
        secondary_actions: Vec::new(),
        controls: Vec::new(),
        accessibility: AccessibilitySpec::label(title),
    }
}

fn list(rows: Vec<PresentationRow>) -> PresentationNode {
    PresentationNode::List {
        style: PresentationListStyle::Rows,
        id: BindingId::new("modes").unwrap(),
        label: None,
        rows,
        searchable: false,
        paging: None,
        accessibility: AccessibilitySpec::label("Modes"),
    }
}

fn rendered(nodes: Vec<PresentationNode>) -> String {
    let mut state = PresentationState::default();
    state.apply(&[Command::ReplaceSurface {
        surface: surface(nodes),
    }]);
    let mut terminal = Terminal::new(TestBackend::new(80, 12)).unwrap();
    terminal
        .draw(|frame| presentation_renderer::draw(frame, frame.area(), &state, 0, None))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

// @internal
#[test]
fn a_pictogram_row_leads_with_its_glyph() {
    let text = rendered(vec![list(vec![
        row("Hover", "pictogram.exchange.hover"),
        row("Cable", "pictogram.exchange.cable"),
    ])]);
    assert!(text.contains("⧉ Hover"), "{text:?}");
    assert!(text.contains("⌁ Cable"), "{text:?}");
    assert!(
        !text.contains("• Hover"),
        "the glyph replaces the bullet: {text:?}"
    );
}

// @internal
#[test]
fn a_pictogram_status_leads_with_its_glyph() {
    let text = rendered(vec![PresentationNode::Status {
        id: None,
        title: "Glance".into(),
        detail: None,
        icon_token: Some("pictogram.exchange.glance".into()),
        badge: None,
        tone: PresentationTone::Neutral,
        activation: None,
        accessibility: AccessibilitySpec::label("Glance"),
    }]);
    assert!(text.contains("◉ Glance"), "{text:?}");
}

// @internal
#[test]
fn a_native_icon_token_keeps_the_bullet_and_is_never_printed() {
    let text = rendered(vec![list(vec![row("Settings", "gearshape")])]);
    assert!(text.contains("• Settings"), "{text:?}");
    assert!(!text.contains("gearshape"), "{text:?}");
}
