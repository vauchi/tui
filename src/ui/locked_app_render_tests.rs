// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use super::{presentation_protocol::PresentationState, presentation_renderer};
use ratatui::{Terminal, backend::TestBackend};
use vauchi_core::Command;

/// A locked app must not render a navigation affordance here either.
///
/// On Android, the lock screen's context bar listed every destination and
/// tapping one walked past the password
/// (`problems/2026-08-12-android-app-password-bypass`). Core caused it, so
/// the question was whether the other shells inherited it — and "they
/// render what Core sends, so they must" is the same reasoning that had the
/// defect filed as Android-only in the first place.
///
/// So this drives a real locked `AppEngine` rather than a hand-built
/// fixture, and asserts on what the terminal actually paints. It fails if
/// Core starts publishing destinations to a locked surface again, or if
/// this shell ever grows a navigation menu of its own.
// @scenario: generic_presentation_protocol.feature :: Contextual controls expose four stable roles
#[test]
fn a_locked_app_paints_no_navigation_affordance() {
    let mut vauchi = vauchi_core::api::Vauchi::in_memory().expect("in-memory core");
    vauchi.create_identity("Alice").expect("identity");
    let mut engine = vauchi_app::ui::AppEngine::new(vauchi);
    engine
        .vauchi_mut()
        .setup_app_password("app-password-123")
        .expect("app password");
    // Password enabled + identity present is what sends bootstrap to the
    // lock screen, the same path a cold launch takes.
    engine.bootstrap();

    let commands = engine.initial_commands().expect("initial commands");
    let bar_has_navigation = commands.iter().any(
        |command| matches!(command, Command::SetContextBar { bar, .. } if bar.navigation.is_some()),
    );
    assert!(
        !bar_has_navigation,
        "core published a navigation affordance for a locked surface",
    );

    let mut state = PresentationState::default();
    state.apply(&commands);
    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| presentation_renderer::draw(frame, frame.area(), &state, 0, None))
        .unwrap();

    let rendered = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(
        !rendered.contains('≡'),
        "the terminal painted the navigation role button while locked: {rendered:?}",
    );
}
