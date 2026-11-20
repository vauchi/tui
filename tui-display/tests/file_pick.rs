// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! A terminal's file picker is a path line. Core asks with `PickFile`; the
//! display shows the line under Core's title, reads the file the person
//! names and answers with its bytes (vauchi/private#1233).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_core::{backend::TestBackend, terminal::Terminal};
use vauchi_contract::{Command, Event, FilePickFailure, FilePickOutcome};
use vauchi_tui_display::ui::presentation_input::KeyOutcome;
use vauchi_tui_display::{SessionView, TerminalSession};

const REQUEST: &str = "pick-1";

fn pick(max_bytes: u64) -> Command {
    Command::PickFile {
        request_id: REQUEST.into(),
        title: "Choose the backup to restore".into(),
        accepted_mime_types: vec![],
        accepted_extensions: vec![],
        max_bytes,
    }
}

fn asked(max_bytes: u64) -> TerminalSession {
    let mut session = TerminalSession::default();
    assert_eq!(
        session.apply_commands(&[pick(max_bytes)]),
        Vec::<Command>::new()
    );
    session
}

fn press(session: &mut TerminalSession, code: KeyCode) -> KeyOutcome {
    session.handle_key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn type_path(session: &mut TerminalSession, path: &std::path::Path) {
    for character in path.to_str().unwrap().chars() {
        assert_eq!(
            press(session, KeyCode::Char(character)),
            KeyOutcome::Consumed
        );
    }
}

fn answered(outcome: FilePickOutcome) -> KeyOutcome {
    KeyOutcome::Events(vec![Event::FilePickCompleted {
        request_id: REQUEST.into(),
        outcome,
    }])
}

fn rendered(session: &TerminalSession) -> String {
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|frame| session.draw(frame, frame.area(), SessionView::default()))
        .unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

// @internal — Core sends `PickFile` only to a display that says it answers it.
#[test]
fn the_display_says_once_that_it_answers_file_picks() {
    let mut session = TerminalSession::default();

    assert_eq!(
        session.take_reports(),
        [Event::FilePickerCapabilitiesChanged {
            correlated_results: true
        }]
    );
    assert_eq!(session.take_reports(), Vec::<Event>::new());
}

// @internal
#[test]
fn a_named_file_is_read_and_returned_under_cores_title() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("backup.vauchi");
    std::fs::write(&path, b"sealed backup").unwrap();
    let mut session = asked(1024);
    assert!(
        rendered(&session).contains("Choose the backup to restore"),
        "the line is shown under the title Core prepared"
    );

    type_path(&mut session, &path);
    let outcome = press(&mut session, KeyCode::Enter);

    assert_eq!(
        outcome,
        answered(FilePickOutcome::Selected {
            bytes: b"sealed backup".to_vec(),
            filename: "backup.vauchi".into(),
        })
    );
    assert!(
        !rendered(&session).contains("Choose the backup to restore"),
        "an answered pick leaves the screen"
    );
}

// @internal — the size limit is Core's, and it holds before the file is read.
#[test]
fn a_file_over_cores_limit_is_refused() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("large.bin");
    std::fs::write(&path, [0u8; 9]).unwrap();
    let mut session = asked(8);

    type_path(&mut session, &path);

    assert_eq!(
        press(&mut session, KeyCode::Enter),
        answered(FilePickOutcome::Failed {
            code: FilePickFailure::TooLarge
        })
    );
}

// @internal
#[test]
fn a_path_that_cannot_be_read_fails_the_pick() {
    let directory = tempfile::tempdir().unwrap();
    let mut session = asked(1024);

    type_path(&mut session, &directory.path().join("absent.vauchi"));

    assert_eq!(
        press(&mut session, KeyCode::Enter),
        answered(FilePickOutcome::Failed {
            code: FilePickFailure::ReadFailed
        })
    );
}

// @internal
#[test]
fn escape_cancels_the_pick_and_an_empty_line_waits() {
    let mut session = asked(1024);

    assert_eq!(press(&mut session, KeyCode::Enter), KeyOutcome::Consumed);
    assert_eq!(
        press(&mut session, KeyCode::Esc),
        answered(FilePickOutcome::Cancelled)
    );
}

// @internal — Core withdraws a pick it no longer waits for.
#[test]
fn a_withdrawn_pick_leaves_the_screen_without_an_answer() {
    let mut session = asked(1024);

    assert_eq!(
        session.apply_commands(&[Command::CancelFilePick {
            request_id: REQUEST.into()
        }]),
        Vec::<Command>::new()
    );

    assert!(!rendered(&session).contains("Choose the backup to restore"));
    assert_eq!(
        session.take_reports().len(),
        1,
        "only the capability report"
    );
}

// @internal — an image pick carries no title to show a line under, so it is
// answered at once and the app goes on.
#[test]
fn picks_the_terminal_has_no_form_for_are_answered_unavailable() {
    let mut session = TerminalSession::default();
    let _capability = session.take_reports();

    assert_eq!(
        session.apply_commands(&[Command::ImagePickFromFile, Command::ImagePickFromLibrary]),
        Vec::<Command>::new()
    );

    assert_eq!(
        session.take_reports(),
        [
            Event::HardwareUnavailable {
                transport: "image_file".into()
            },
            Event::HardwareUnavailable {
                transport: "photo_library".into()
            },
        ]
    );
}
