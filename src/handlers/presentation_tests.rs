// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use vauchi_app::i18n::{Locale, get_string};
use vauchi_app::ui::AppEngine;
use vauchi_core::api::Vauchi;
use vauchi_core::{Command, Event, SurfaceId};

use super::handle_presentation_key;
use crate::app::App;

fn app_showing_a_surface_core_never_prepared() -> App {
    let mut app = App::new(
        AppEngine::new(Vauchi::in_memory().expect("in-memory core")),
        "wss://relay.vauchi.app".into(),
        std::path::PathBuf::from("."),
    );
    app.report_presentation_environment(80, 24);
    let mut ghost = app
        .presentation
        .surface()
        .expect("bootstrapped surface")
        .clone();
    ghost.surface_id = SurfaceId::new("ghost_surface").expect("valid surface id");
    let mut profile = app
        .presentation
        .profile()
        .expect("reported profile")
        .clone();
    profile.active_surface = ghost.surface_id.clone();
    app.apply_presentation_commands(vec![
        Command::ReplaceSurface { surface: ghost },
        Command::SetPresentationProfile { profile },
    ]);
    app
}

// @internal
#[test]
fn rejected_event_shows_core_prepared_alert_instead_of_error_text() {
    let mut app = app_showing_a_surface_core_never_prepared();

    handle_presentation_key(&mut app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    let title = get_string(Locale::English, "error.title");
    let message = get_string(Locale::English, "error.generic");
    assert_ne!(title, "error.title", "locale must resolve the title key");
    assert_ne!(
        message, "error.generic",
        "locale must resolve the message key"
    );
    let (shown_title, shown_message) = app.alert_message.expect("rejection must alert");
    assert_eq!(
        (shown_title.as_str(), shown_message.as_str()),
        (title.as_str(), message.as_str())
    );
    let raw_error = app
        .app_engine
        .dispatch(Event::SurfaceActivated {
            surface_id: SurfaceId::new("ghost_surface").expect("valid surface id"),
        })
        .expect_err("ghost surface stays rejected")
        .to_string();
    assert!(
        !shown_title.contains(&raw_error) && !shown_message.contains(&raw_error),
        "raw error text leaked"
    );
}
