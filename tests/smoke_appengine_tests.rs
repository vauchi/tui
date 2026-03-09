// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Smoke tests for AppEngine-driven screen rendering.
//!
//! Verifies that all 10 engine-driven screens render correctly through the
//! AppEngine → screen_renderer pipeline.

use ratatui::prelude::*;

use vauchi_tui::app::{App, Screen};
use vauchi_tui::backend::Backend;
use vauchi_tui::ui;

fn create_app_with_identity() -> (App, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let mut backend = Backend::new(dir.path()).unwrap();
    backend.create_identity("Smoke Tester").unwrap();
    let backend = Backend::new(dir.path()).unwrap();
    let app = App::new(backend);
    (app, dir)
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
    let (mut app, _dir) = create_app_with_identity();
    app.screen = Screen::Home;
    let output = render_to_string(&mut app, 80, 24);

    // Home screen should have the title and engine-driven content
    assert!(
        output.contains("Home"),
        "Home screen should show title. Got:\n{}",
        output
    );
    // Should show "Add Contact" action from engine
    assert!(
        output.contains("Add Contact"),
        "Home screen should show Add Contact action. Got:\n{}",
        output
    );
}

#[test]
fn smoke_contacts_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.screen = Screen::Contacts;
    let output = render_to_string(&mut app, 80, 24);

    assert!(
        output.contains("Contacts"),
        "Contacts screen should show title. Got:\n{}",
        output
    );
}

#[test]
fn smoke_exchange_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.screen = Screen::Exchange;
    let output = render_to_string(&mut app, 80, 24);

    assert!(
        output.contains("Share Your Code"),
        "Exchange screen should show share prompt. Got:\n{}",
        output
    );
}

#[test]
fn smoke_settings_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.screen = Screen::Settings;
    let output = render_to_string(&mut app, 80, 24);

    assert!(
        output.contains("Settings"),
        "Settings screen should show title. Got:\n{}",
        output
    );
    // Engine-driven settings should show setting groups
    assert!(
        output.contains("Privacy"),
        "Settings screen should show Privacy settings group. Got:\n{}",
        output
    );
}

#[test]
fn smoke_help_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.screen = Screen::Help;
    let output = render_to_string(&mut app, 80, 24);

    assert!(
        output.contains("Help"),
        "Help screen should show title. Got:\n{}",
        output
    );
    // Engine-driven help should show FAQ items
    assert!(
        output.contains("How do I"),
        "Help screen should show FAQ content. Got:\n{}",
        output
    );
}

#[test]
fn smoke_backup_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.screen = Screen::Backup;
    let output = render_to_string(&mut app, 80, 24);

    assert!(
        output.contains("Backup") || output.contains("Recovery"),
        "Backup screen should show Backup or Recovery. Got:\n{}",
        output
    );
}

#[test]
fn smoke_delivery_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.screen = Screen::Delivery;
    let output = render_to_string(&mut app, 80, 24);

    assert!(
        output.contains("Delivery") || output.contains("Delivered"),
        "Delivery screen should show delivery status. Got:\n{}",
        output
    );
}

#[test]
fn smoke_devices_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.screen = Screen::Devices;
    let output = render_to_string(&mut app, 80, 24);

    assert!(
        output.contains("Device") || output.contains("Link"),
        "Devices screen should show device linking content. Got:\n{}",
        output
    );
}

#[test]
fn smoke_duress_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.screen = Screen::Duress;
    let output = render_to_string(&mut app, 80, 24);

    assert!(
        output.contains("Duress") || output.contains("PIN"),
        "Duress screen should show Duress PIN content. Got:\n{}",
        output
    );
}

#[test]
fn smoke_emergency_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.screen = Screen::Emergency;
    let output = render_to_string(&mut app, 80, 24);

    assert!(
        output.contains("Emergency") || output.contains("Wipe"),
        "Emergency screen should show emergency/wipe content. Got:\n{}",
        output
    );
}

#[test]
fn smoke_all_engine_screens_no_panic() {
    let (mut app, _dir) = create_app_with_identity();

    for screen in [
        Screen::Home,
        Screen::Contacts,
        Screen::Exchange,
        Screen::Settings,
        Screen::Help,
        Screen::Backup,
        Screen::Delivery,
        Screen::Devices,
        Screen::Duress,
        Screen::Emergency,
    ] {
        app.screen = screen;
        // Should not panic
        let output = render_to_string(&mut app, 80, 24);
        assert!(!output.is_empty(), "{:?} rendered empty", screen);
    }
}
