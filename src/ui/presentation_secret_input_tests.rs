// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Core's lock screen sends the typed app password as the input's value and
//! leaves masking to the shell (`input_kind: Password`), so a terminal that
//! prints values as-is shows the password to whoever reads the screen
//! (vauchi/private#619).

use super::presentation_protocol::PresentationState;
use super::presentation_renderer;
use super::presentation_renderer_tests::titled_surface;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use vauchi_core::{AccessibilitySpec, BindingId, Command, PresentationInputKind, PresentationNode};

fn painted_input(input_kind: PresentationInputKind, value: &str) -> String {
    let mut surface = titled_surface("lock", "Vauchi is Locked");
    surface.nodes = vec![PresentationNode::Input {
        binding_id: BindingId::new("surface.1.pin").unwrap(),
        label: "Password".into(),
        value: value.into(),
        placeholder: None,
        input_kind,
        max_length: Some(128),
        validation_error: None,
        enabled: true,
        accessibility: AccessibilitySpec::label("Password entry"),
    }];
    let mut state = PresentationState::default();
    state.apply(&[Command::ReplaceSurface { surface }]);
    let mut terminal = Terminal::new(TestBackend::new(60, 12)).unwrap();
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
fn a_password_input_paints_one_bullet_per_character_not_the_password() {
    let painted = painted_input(PresentationInputKind::Password, "pass1234");

    assert!(!painted.contains("pass1234"), "{painted}");
    assert!(painted.contains("Password: ••••••••"), "{painted}");
}

// @internal
#[test]
fn a_pin_input_paints_bullets_not_the_digits() {
    let painted = painted_input(PresentationInputKind::Pin, "654321");

    assert!(!painted.contains("654321"), "{painted}");
    assert!(painted.contains("Password: ••••••"), "{painted}");
}

// @internal
#[test]
fn a_text_input_still_paints_its_value() {
    let painted = painted_input(PresentationInputKind::Text, "Alice");

    assert!(painted.contains("Password: Alice"), "{painted}");
}
