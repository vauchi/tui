// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Prompts, feedback, scrolling, highlights and the node painter's less
//! common nodes (vauchi/private#448).

use super::presentation_protocol::PresentationState;
use super::presentation_renderer;
use super::presentation_renderer_tests::{action, highlighted_lines, row, titled_surface};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::prelude::Frame;
use vauchi_core::{
    AccessibilitySpec, BindingId, Command, ContextBar, FilePickPurpose, OverlayKind, OverlaySpec,
    PresentationAxis, PresentationImageShape, PresentationInputKind, PresentationListStyle,
    PresentationNode, PresentationQrPurpose, PresentationTextStyle, SurfaceId,
};

fn render(width: u16, height: u16, mut paint: impl FnMut(&mut Frame)) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| paint(frame)).unwrap();
    terminal.backend().buffer().clone()
}

fn text(buffer: &Buffer) -> String {
    buffer.content.iter().map(|cell| cell.symbol()).collect()
}

fn rows(buffer: &Buffer) -> Vec<String> {
    buffer
        .content
        .chunks(usize::from(buffer.area.width))
        .map(|line| line.iter().map(|cell| cell.symbol()).collect())
        .collect()
}

fn state_showing(nodes: Vec<PresentationNode>) -> PresentationState {
    let mut surface = titled_surface("surface-primary", "Surface");
    surface.nodes = nodes;
    let mut state = PresentationState::default();
    state.apply(&[Command::ReplaceSurface { surface }]);
    state
}

fn binding(id: &str) -> BindingId {
    BindingId::new(id).unwrap()
}

fn file_pick() -> Command {
    Command::FilePickFromUser {
        accepted_mime_types: Vec::new(),
        accepted_extensions: Vec::new(),
        purpose: FilePickPurpose::ImportContacts,
    }
}

// @internal
#[test]
fn effect_prompts_name_what_core_asked_for() {
    for (effect, instruction) in [
        (file_pick(), "File path"),
        (Command::QrRequestScan, "Paste QR data"),
        (Command::ImagePickFromFile, "Image path"),
        (Command::ImagePickFromLibrary, "Image path"),
        (Command::ImageCaptureFromCamera, "Image path"),
    ] {
        let buffer = render(80, 24, |frame| {
            presentation_renderer::draw_effect_prompt(frame, frame.area(), &effect, "typed")
        });
        let painted = text(&buffer);
        assert!(painted.contains("Core request"), "{effect:?}: {painted}");
        assert!(
            painted.contains(&format!("{instruction}:")),
            "{effect:?}: {painted}"
        );
        assert!(painted.contains("> typed"), "{effect:?}: {painted}");
    }

    let buffer = render(80, 24, |frame| {
        presentation_renderer::draw_effect_prompt(
            frame,
            frame.area(),
            &Command::PerformNativeBack,
            "typed",
        )
    });
    assert!(!text(&buffer).contains("Core request"));
}

// @internal
#[test]
fn feedback_shows_an_alert_over_a_status_and_a_status_alone() {
    let alert = ("Export failed".to_string(), "disk full".to_string());
    let buffer = render(80, 24, |frame| {
        presentation_renderer::draw_feedback(frame, frame.area(), Some("Saved"), Some(&alert))
    });
    let painted = text(&buffer);
    for expected in ["Export failed", "disk full", "Enter or Esc dismisses"] {
        assert!(painted.contains(expected), "{painted}");
    }
    assert!(!painted.contains("Saved"));

    let buffer = render(80, 24, |frame| {
        presentation_renderer::draw_feedback(frame, frame.area(), Some("Saved"), None)
    });
    let painted = text(&buffer);
    assert!(
        painted.contains("Status") && painted.contains("Saved"),
        "{painted}"
    );
}

// @internal
#[test]
fn scrolling_keeps_the_selected_row_on_screen() {
    let state = state_showing(vec![PresentationNode::List {
        style: PresentationListStyle::Rows,
        id: binding("rows"),
        label: None,
        rows: (0..40)
            .map(|index| row(&format!("Row {index:02}"), Some(action("open", "Open"))))
            .collect(),
        searchable: false,
        paging: None,
        accessibility: AccessibilitySpec::label("Rows"),
    }]);

    let buffer = render(80, 12, |frame| {
        presentation_renderer::draw(frame, frame.area(), &state, 0, Some(30))
    });

    let painted = text(&buffer);
    assert!(painted.contains("Row 30"), "{painted}");
    assert!(
        painted.contains("Row 24"),
        "the selection sits at the bottom: {painted}"
    );
    assert!(!painted.contains("Row 31"), "{painted}");
    assert!(!painted.contains("Row 00"), "{painted}");
}

// @internal
#[test]
fn the_context_bar_highlights_only_the_selected_action() {
    let mut state = state_showing(Vec::new());
    state.apply(&[Command::SetContextBar {
        surface_id: SurfaceId::new("surface-primary").unwrap(),
        revision: 1,
        bar: Box::new(ContextBar {
            back: Some(action("back", "Back")),
            navigation: Some(action("navigation", "Navigate")),
            primary: Some(action("continue", "Continue")),
            secondary: Some(action("more", "More")),
            info: None,
        }),
    }]);

    let buffer = render(80, 16, |frame| {
        presentation_renderer::draw(frame, frame.area(), &state, 2, None)
    });
    assert_eq!(highlighted_lines(&buffer), ["[ Continue ]"]);

    let buffer = render(80, 16, |frame| {
        presentation_renderer::draw(frame, frame.area(), &state, 0, None)
    });
    assert_eq!(highlighted_lines(&buffer), ["< Back"]);
}

// @internal
#[test]
fn overlays_take_their_kinds_frame_and_highlight_the_selected_item() {
    for (kind, fallback_title) in [
        (OverlayKind::Navigation, "Navigate"),
        (OverlayKind::ActionMenu, "Actions"),
    ] {
        let mut state = state_showing(Vec::new());
        state.apply(&[Command::PresentOverlay {
            surface_id: SurfaceId::new("surface-primary").unwrap(),
            revision: 1,
            overlay: OverlaySpec {
                kind,
                title: None,
                body: None,
                items: vec![action("first", "First"), action("second", "Second")],
                close_label: None,
            },
        }]);

        let buffer = render(80, 24, |frame| {
            presentation_renderer::draw(frame, frame.area(), &state, 1, None)
        });

        let painted = text(&buffer);
        assert!(painted.contains(fallback_title), "{kind:?}: {painted}");
        assert!(painted.contains("1. First"), "{kind:?}: {painted}");
        assert_eq!(highlighted_lines(&buffer), ["2. Second"], "{kind:?}");
    }
}

// @internal
#[test]
fn every_node_kind_paints_its_text() {
    let state = state_showing(vec![
        PresentationNode::Input {
            binding_id: binding("name"),
            label: "Name".into(),
            value: String::new(),
            placeholder: Some("type here".into()),
            input_kind: PresentationInputKind::Text,
            max_length: None,
            validation_error: Some("required".into()),
            enabled: true,
            accessibility: AccessibilitySpec::label("Name"),
        },
        PresentationNode::Toggle {
            binding_id: binding("flag"),
            label: "Flag".into(),
            value: true,
            enabled: true,
            accessibility: AccessibilitySpec::label("Flag"),
        },
        PresentationNode::Image {
            id: None,
            data: None,
            fallback_text: Some("[avatar]".into()),
            shape: PresentationImageShape::Circle,
            size: None,
            brightness: 1.0,
            activation: None,
            accessibility: AccessibilitySpec::label("Avatar"),
        },
        PresentationNode::Qr {
            id: binding("qr-display"),
            payloads: vec!["PAYLOAD-1".into()],
            purpose: PresentationQrPurpose::Display,
            label: Some("Scan me".into()),
            placement: None,
            error_correction: None,
            size: None,
            accessibility: AccessibilitySpec::label("QR"),
        },
        PresentationNode::Qr {
            id: binding("qr-capture"),
            payloads: vec!["HIDDEN".into()],
            purpose: PresentationQrPurpose::Capture,
            label: Some("Capture".into()),
            placement: None,
            error_correction: None,
            size: None,
            accessibility: AccessibilitySpec::label("QR"),
        },
        PresentationNode::Confirmation {
            id: binding("confirm"),
            warning: "Really?".into(),
            confirm: action("yes", "Yes"),
            cancel: action("no", "No"),
            accessibility: AccessibilitySpec::label("Confirm"),
        },
        PresentationNode::Slider {
            binding_id: binding("volume"),
            label: "Volume".into(),
            value: 3.0,
            minimum: 0.0,
            maximum: 10.0,
            step: None,
            minimum_icon: None,
            maximum_icon: None,
            accessibility: AccessibilitySpec::label("Volume"),
        },
        PresentationNode::Progress {
            label: Some("Upload".into()),
            value: Some(0.5),
            accessibility: AccessibilitySpec::label("Upload"),
        },
        PresentationNode::Divider,
        PresentationNode::Group {
            id: None,
            label: Some("Section".into()),
            axis: PresentationAxis::Vertical,
            children: vec![PresentationNode::Text {
                id: None,
                content: "Child".into(),
                style: PresentationTextStyle::Body,
                accessibility: AccessibilitySpec::label("Child"),
            }],
            accessibility: AccessibilitySpec::label("Section"),
        },
    ]);

    let buffer = render(80, 24, |frame| {
        presentation_renderer::draw(frame, frame.area(), &state, 0, None)
    });

    let painted = text(&buffer);
    for expected in [
        "Name: type here",
        "! required",
        "[x] Flag",
        "[avatar]",
        "[QR] Scan me",
        "PAYLOAD-1",
        "[QR] Capture",
        "! Really?",
        "Volume: 3",
        "Upload 50%",
        "Section",
    ] {
        assert!(
            painted.contains(expected),
            "missing {expected:?} in {painted}"
        );
    }
    assert!(!painted.contains("HIDDEN"), "{painted}");
    assert!(
        rows(&buffer).iter().any(|line| line.contains("│────────")),
        "the divider is drawn inside the frame: {painted}"
    );
    assert!(
        rows(&buffer).iter().any(|line| line.contains("│  Child")),
        "nested text is indented one level: {painted}"
    );
}
