// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Smoke tests for AppEngine-driven screen rendering.
//!
//! Verifies that all 5 migrated screens (Home, Contacts, Exchange, Settings, Help)
//! render correctly through the AppEngine → screen_renderer pipeline.

use ratatui::prelude::*;

use vauchi_tui::app::{App, Screen};
use vauchi_tui::backend::Backend;
use vauchi_tui::ui;

fn create_app_with_identity() -> App {
    let dir = tempfile::tempdir().unwrap();
    let mut backend = Backend::new(dir.path()).unwrap();
    backend.create_identity("Smoke Tester").unwrap();
    // Leak the tempdir so it lives long enough
    let path = dir.into_path();
    let backend = Backend::new(&path).unwrap();
    App::new(backend)
}

fn render_to_string(app: &mut App, width: u16, height: u16) -> String {
    let backend = ratatui::backend::TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui::draw(f, app)).unwrap();
    let buf = terminal.backend().buffer().clone();
    let mut output = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            output.push(buf[(x, y)].symbol().chars().next().unwrap_or(' '));
        }
        output.push('\n');
    }
    output
}

#[test]
fn smoke_home_screen_renders_via_engine() {
    let mut app = create_app_with_identity();
    app.screen = Screen::Home;
    let output = render_to_string(&mut app, 80, 24);

    // Home screen should have the title and engine-driven content
    assert!(
        output.contains("Home") || output.contains("Vauchi"),
        "Home screen should show title. Got:\n{}",
        output
    );
    // Should show "Add Contact" action from engine
    assert!(
        output.contains("Add Contact") || output.contains("add"),
        "Home screen should show Add Contact action. Got:\n{}",
        output
    );
}

#[test]
fn smoke_contacts_screen_renders_via_engine() {
    let mut app = create_app_with_identity();
    app.screen = Screen::Contacts;
    let output = render_to_string(&mut app, 80, 24);

    assert!(
        output.contains("Contacts") || output.contains("contact"),
        "Contacts screen should show title. Got:\n{}",
        output
    );
}

#[test]
fn smoke_exchange_screen_renders_via_engine() {
    let mut app = create_app_with_identity();
    app.screen = Screen::Exchange;
    let output = render_to_string(&mut app, 80, 24);

    assert!(
        output.contains("Exchange") || output.contains("QR"),
        "Exchange screen should show title or QR content. Got:\n{}",
        output
    );
}

#[test]
fn smoke_settings_screen_renders_via_engine() {
    let mut app = create_app_with_identity();
    app.screen = Screen::Settings;
    let output = render_to_string(&mut app, 80, 24);

    assert!(
        output.contains("Settings") || output.contains("Profile"),
        "Settings screen should show title or profile group. Got:\n{}",
        output
    );
    // Engine-driven settings should show setting groups
    assert!(
        output.contains("Privacy") || output.contains("Security") || output.contains("Network"),
        "Settings screen should show settings groups. Got:\n{}",
        output
    );
}

#[test]
fn smoke_help_screen_renders_via_engine() {
    let mut app = create_app_with_identity();
    app.screen = Screen::Help;
    let output = render_to_string(&mut app, 80, 24);

    assert!(
        output.contains("Help") || output.contains("FAQ"),
        "Help screen should show title. Got:\n{}",
        output
    );
    // Engine-driven help should show FAQ items
    assert!(
        output.contains("How do I") || output.contains("Getting Started"),
        "Help screen should show FAQ content. Got:\n{}",
        output
    );
}

#[test]
fn smoke_all_migrated_screens_no_panic() {
    let mut app = create_app_with_identity();

    for screen in [
        Screen::Home,
        Screen::Contacts,
        Screen::Exchange,
        Screen::Settings,
        Screen::Help,
    ] {
        app.screen = screen;
        // Should not panic
        let output = render_to_string(&mut app, 80, 24);
        assert!(!output.is_empty(), "{:?} rendered empty", screen);
    }
}
