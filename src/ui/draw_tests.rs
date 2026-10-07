// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use vauchi_app::ui::AppEngine;
use vauchi_core::api::Vauchi;

use super::draw_presentation;
use crate::app::App;

// @internal
#[test]
fn drawing_paints_the_active_surface_title() {
    let mut app = App::new(
        AppEngine::new(Vauchi::in_memory().expect("in-memory core")),
        "wss://relay.vauchi.app".into(),
        std::path::PathBuf::from("."),
    );
    app.report_presentation_environment(80, 24);
    let title = app
        .presentation
        .surface()
        .expect("bootstrapped surface")
        .title
        .clone();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();

    terminal
        .draw(|frame| draw_presentation(frame, &app))
        .unwrap();

    let painted: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(painted.contains(&title), "{title:?} not in {painted:?}");
}

// Before Core's first surface arrives the TUI shows a placeholder; its
// words come from the locale catalog, not a hardcoded English literal
// (vauchi/private#543).
// @internal
#[test]
fn the_placeholder_before_the_first_surface_comes_from_the_locale() {
    let state = super::presentation_protocol::PresentationState::default();
    let expected = crate::i18n::I18n::default().t("app.loading");
    let mut terminal = Terminal::new(TestBackend::new(40, 5)).unwrap();

    terminal
        .draw(|frame| super::presentation_renderer::draw(frame, frame.area(), &state, 0, None))
        .unwrap();

    let painted: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert_ne!(expected, "app.loading", "the locale must resolve the key");
    assert!(
        painted.contains(&expected),
        "{expected:?} not in {painted:?}"
    );
}
