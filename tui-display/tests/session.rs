// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};
use vauchi_contract::{Command, Event};
use vauchi_tui_display::ui::presentation_input::KeyOutcome;
use vauchi_tui_display::{SessionView, TerminalSession};

fn initial_commands() -> Vec<Command> {
    let fixture: serde_json::Value =
        serde_json::from_str(vauchi_contract::fixtures::presentation_contract_v1_json()).unwrap();
    serde_json::from_value(fixture["initial_commands"].clone()).unwrap()
}

// @scenario: generic_presentation_protocol.feature :: Release contains only the generic action system
#[test]
fn session_renders_commands_and_returns_keyboard_events_without_an_engine() {
    let mut session = TerminalSession::default();
    assert_eq!(
        session.apply_commands(&initial_commands()),
        Vec::<Command>::new()
    );
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|frame| {
            session.draw(
                frame,
                frame.area(),
                SessionView {
                    loading_label: "Loading",
                    ..SessionView::default()
                },
            )
        })
        .unwrap();
    let rendered = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains("Create new identity"), "{rendered}");
    let fixture: serde_json::Value =
        serde_json::from_str(vauchi_contract::fixtures::presentation_contract_v1_json()).unwrap();
    let expected = vec![
        Event::SurfaceActivated {
            surface_id: session.presentation().surface().unwrap().surface_id.clone(),
        },
        serde_json::from_value(fixture["steps"][0]["event"].clone()).unwrap(),
    ];
    assert_eq!(
        session.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        KeyOutcome::Events(expected)
    );
}

// @internal
#[test]
fn session_returns_native_effects_to_the_host_without_executing_them() {
    let mut session = TerminalSession::default();
    let effect = Command::OpenExternalUrl {
        url: "https://example.invalid".into(),
    };
    assert_eq!(
        session.apply_commands(std::slice::from_ref(&effect)),
        vec![effect]
    );
    assert_eq!(session.presentation().surface(), None);
}

// @internal
#[test]
fn session_draws_supplied_loading_copy_and_native_feedback() {
    let session = TerminalSession::default();
    let mut terminal = Terminal::new(TestBackend::new(70, 20)).unwrap();
    terminal
        .draw(|frame| {
            session.draw(
                frame,
                frame.area(),
                SessionView {
                    loading_label: "Caricamento",
                    ..SessionView::default()
                },
            )
        })
        .unwrap();
    let rendered = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.starts_with("Caricamento"), "{rendered}");
    terminal
        .draw(|frame| {
            session.draw(
                frame,
                frame.area(),
                SessionView {
                    status: Some("Prepared status"),
                    ..SessionView::default()
                },
            )
        })
        .unwrap();
    let rendered = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains("Prepared status"), "{rendered}");
    assert!(
        !rendered.contains("Caricamento"),
        "previous frame leaked: {rendered}"
    );
}
