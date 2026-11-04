// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use ratatui::{Terminal, backend::TestBackend};
use vauchi_core::Command;
use vauchi_tui_display::ui::{presentation_protocol::PresentationState, presentation_renderer};

// @scenario: generic_presentation_protocol.feature :: Release contains only the generic action system
#[test]
fn display_contract_fixture_replays_and_renders_without_an_engine() {
    let fixture: serde_json::Value =
        serde_json::from_str(vauchi_app::ui::presentation_contract_fixture_json()).unwrap();
    let mut state = PresentationState::default();
    let initial: Vec<Command> =
        serde_json::from_value(fixture["initial_commands"].clone()).unwrap();
    state.apply(&initial);
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|frame| presentation_renderer::draw(frame, frame.area(), &state, 0, None, "Loading"))
        .unwrap();
    let rendered = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains("Create new identity"), "{rendered}");
    for step in fixture["steps"].as_array().unwrap() {
        let commands: Vec<Command> = serde_json::from_value(step["commands"].clone()).unwrap();
        state.apply(&commands);
    }
    assert_eq!(
        serde_json::to_value(state.surface().unwrap()).unwrap(),
        fixture["expected_state"]["surface"]
    );
    assert_eq!(
        serde_json::to_value(state.context_bar().unwrap()).unwrap(),
        fixture["expected_state"]["context_bar"]
    );
}
