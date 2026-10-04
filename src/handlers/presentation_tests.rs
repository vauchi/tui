// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use vauchi_app::i18n::{Locale, get_string};
use vauchi_app::ui::AppEngine;
use vauchi_core::api::Vauchi;
use vauchi_core::{Command, Event, SurfaceId};

use super::{Action, handle_presentation_key};
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

fn fresh_app() -> App {
    App::new(
        AppEngine::new(Vauchi::in_memory().expect("in-memory core")),
        "wss://relay.vauchi.app".into(),
        std::path::PathBuf::from("."),
    )
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn type_text(app: &mut App, text: &str) {
    for character in text.chars() {
        handle_presentation_key(app, key(KeyCode::Char(character)));
    }
}

fn file_pick() -> Command {
    Command::FilePickFromUser {
        accepted_mime_types: Vec::new(),
        accepted_extensions: Vec::new(),
        purpose: vauchi_core::FilePickPurpose::ImportContacts,
    }
}

// @internal
#[test]
fn typing_fills_the_request_prompt_and_control_chords_do_not() {
    let mut app = fresh_app();
    app.presentation_effects.push_back(Command::QrRequestScan);

    type_text(&mut app, "ab");
    handle_presentation_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
    );
    assert_eq!(app.input_buffer, "ab");

    handle_presentation_key(&mut app, key(KeyCode::Backspace));
    assert_eq!(app.input_buffer, "a");
}

// @internal
#[test]
fn escape_cancels_whichever_request_is_pending() {
    for effect in [
        Command::QrRequestScan,
        file_pick(),
        Command::ImagePickFromFile,
        Command::ImagePickFromLibrary,
        Command::ImageCaptureFromCamera,
    ] {
        let mut app = fresh_app();
        app.presentation_effects.push_back(effect.clone());
        type_text(&mut app, "half");

        let action = handle_presentation_key(&mut app, key(KeyCode::Esc));

        assert!(matches!(action, Action::Continue), "{effect:?}");
        assert!(app.presentation_effects.is_empty(), "{effect:?}");
        assert_eq!(app.input_buffer, "", "{effect:?}");
    }
}

// @internal
#[test]
fn enter_with_nothing_typed_keeps_the_request_waiting() {
    let mut app = fresh_app();
    app.presentation_effects.push_back(Command::QrRequestScan);

    handle_presentation_key(&mut app, key(KeyCode::Enter));

    assert_eq!(app.presentation_effects.len(), 1);
}

// @internal
#[test]
fn enter_submits_the_typed_qr_payload() {
    let mut app = fresh_app();
    app.presentation_effects.push_back(Command::QrRequestScan);
    type_text(&mut app, "not-a-vauchi-code");

    handle_presentation_key(&mut app, key(KeyCode::Enter));

    assert!(app.presentation_effects.is_empty());
    assert_eq!(app.input_buffer, "");
}

// @internal
#[test]
fn enter_reads_the_named_file_or_image_and_reports_a_missing_one() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("contacts.vcf");
    std::fs::write(&path, b"BEGIN:VCARD").unwrap();

    for (effect, missing_title) in [
        (file_pick(), "Unable to read file"),
        (Command::ImagePickFromFile, "Unable to read image"),
        (Command::ImagePickFromLibrary, "Unable to read image"),
        (Command::ImageCaptureFromCamera, "Unable to read image"),
    ] {
        let mut app = fresh_app();
        app.presentation_effects.push_back(effect.clone());
        type_text(&mut app, path.to_str().unwrap());
        handle_presentation_key(&mut app, key(KeyCode::Enter));
        assert!(app.presentation_effects.is_empty(), "{effect:?}");
        assert_eq!(app.input_buffer, "", "{effect:?}");

        let mut app = fresh_app();
        app.presentation_effects.push_back(effect.clone());
        type_text(&mut app, dir.path().join("missing").to_str().unwrap());
        handle_presentation_key(&mut app, key(KeyCode::Enter));
        assert_eq!(app.presentation_effects.len(), 1, "{effect:?}");
        assert_eq!(
            app.alert_message.as_ref().map(|(title, _)| title.as_str()),
            Some(missing_title),
            "{effect:?}"
        );
    }
}

// @internal
#[test]
fn a_set_quit_flag_ends_the_loop_on_the_next_key() {
    let mut app = fresh_app();
    app.should_quit = true;

    let action = handle_presentation_key(&mut app, key(KeyCode::Char('x')));

    assert!(matches!(action, Action::Quit));
}
