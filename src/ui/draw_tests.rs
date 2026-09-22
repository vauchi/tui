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
