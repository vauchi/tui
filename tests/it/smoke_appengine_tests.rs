// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Smoke tests for AppEngine-driven screen rendering.
//!
//! Verifies that all 16 engine-driven screens render correctly through the
//! AppEngine → screen_renderer pipeline.

use ratatui::prelude::*;

use vauchi_app::ui::AppEngine;
use vauchi_core::{SymmetricKey, Vauchi, VauchiConfig};
use vauchi_tui::app::{App, Screen};
use vauchi_tui::ui;

fn create_app_engine(data_dir: &std::path::Path) -> AppEngine {
    let key = SymmetricKey::generate();
    let config = VauchiConfig::with_storage_path(data_dir.join("vauchi.db")).with_storage_key(key);
    let vauchi: Vauchi = Vauchi::new(config).expect("vauchi");
    AppEngine::new(vauchi)
}

fn create_app_with_identity() -> (App, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let mut app_engine = create_app_engine(dir.path());
    app_engine
        .vauchi_mut()
        .create_identity("Smoke Tester")
        .expect("create identity");
    let app = App::new(
        app_engine,
        "wss://relay.vauchi.app".to_string(),
        dir.path().to_path_buf(),
    );
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

// @internal
#[test]
fn smoke_home_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::MyInfo);
    let output = render_to_string(&mut app, 80, 24);

    // Home screen should have the title (display name or "My Info")
    assert!(
        output.contains("Smoke Tester") || output.contains("My Info"),
        "MyInfo screen should show title. Got:\n{}",
        output
    );
    // Should show "Add Entry" action from engine
    assert!(
        output.contains("Add Entry"),
        "MyInfo screen should show Add Entry action. Got:\n{}",
        output
    );
}

// @internal
#[test]
fn smoke_contacts_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Contacts);
    let output = render_to_string(&mut app, 80, 24);

    assert!(
        output.contains("Contacts"),
        "Contacts screen should show title. Got:\n{}",
        output
    );
}

// @internal
#[test]
fn smoke_exchange_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Exchange);
    let output = render_to_string(&mut app, 80, 24);

    assert!(
        output.contains("Share Your Code") || output.contains("Exchange Mode"),
        "Exchange screen should show share prompt or mode selection. Got:\n{}",
        output
    );
}

// @internal
#[test]
fn smoke_settings_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Settings);
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

// @internal
#[test]
fn smoke_help_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Help);
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

// @internal
#[test]
fn smoke_backup_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Backup);
    let output = render_to_string(&mut app, 80, 24);

    assert!(
        output.contains("Backup") || output.contains("Recovery"),
        "Backup screen should show Backup or Recovery. Got:\n{}",
        output
    );
}

// @internal
#[test]
fn smoke_delivery_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Delivery);
    let output = render_to_string(&mut app, 80, 24);

    assert!(
        output.contains("Delivery") || output.contains("Delivered"),
        "Delivery screen should show delivery status. Got:\n{}",
        output
    );
}

// @internal
#[test]
fn smoke_devices_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Devices);
    let output = render_to_string(&mut app, 80, 24);

    assert!(
        output.contains("Device") || output.contains("Link"),
        "Devices screen should show device linking content. Got:\n{}",
        output
    );
}

// @internal
#[test]
fn smoke_duress_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Duress);
    let output = render_to_string(&mut app, 80, 24);

    assert!(
        output.contains("Duress") || output.contains("PIN"),
        "Duress screen should show Duress PIN content. Got:\n{}",
        output
    );
}

// @internal
#[test]
fn smoke_emergency_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Emergency);
    let output = render_to_string(&mut app, 80, 24);

    assert!(
        output.contains("Emergency") || output.contains("Wipe"),
        "Emergency screen should show emergency/wipe content. Got:\n{}",
        output
    );
}

// ── Wave 6 Phase A: new engine smoke tests ───────────────────────────

// @internal
#[test]
fn smoke_sync_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Sync);
    let output = render_to_string(&mut app, 80, 24);
    assert!(
        output.contains("Sync"),
        "Sync screen should show title. Got:\n{}",
        output
    );
}

// @internal
#[test]
fn smoke_recovery_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Recovery);
    let output = render_to_string(&mut app, 80, 24);
    assert!(
        output.contains("Recovery") || output.contains("Quorum"),
        "Recovery screen should show recovery content. Got:\n{}",
        output
    );
}

// @internal
#[test]
fn smoke_groups_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Groups);
    let output = render_to_string(&mut app, 80, 24);
    assert!(
        output.contains("Group"),
        "Groups screen should show group content. Got:\n{}",
        output
    );
}

// @internal
#[test]
fn smoke_privacy_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Privacy);
    let output = render_to_string(&mut app, 80, 24);
    assert!(
        output.contains("Privacy") || output.contains("Data"),
        "Privacy screen should show privacy content. Got:\n{}",
        output
    );
}

// @internal
#[test]
fn smoke_support_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Support);
    let output = render_to_string(&mut app, 80, 24);
    assert!(
        output.contains("Support") || output.contains("Vauchi"),
        "Support screen should show support content. Got:\n{}",
        output
    );
}

// @internal
#[test]
fn smoke_group_detail_screen_renders_via_engine() {
    use vauchi_app::ui::AppScreen;
    let (mut app, _dir) = create_app_with_identity();
    app.app_engine.navigate_to(AppScreen::GroupDetail {
        group_id: "test-group".into(),
    });
    app.goto(Screen::GroupDetail);
    let output = render_to_string(&mut app, 80, 24);
    assert!(
        output.contains("Group") || output.contains("Members"),
        "GroupDetail screen should show group content. Got:\n{}",
        output
    );
}

// @internal
#[test]
fn smoke_contact_visibility_screen_renders_via_engine() {
    let (mut app, _dir) = create_app_with_identity();
    app.selected_contact_id = Some("test-contact".into());
    app.goto(Screen::ContactVisibility);
    let output = render_to_string(&mut app, 80, 24);
    assert!(
        output.contains("Visibility") || output.contains("toggle"),
        "ContactVisibility screen should show visibility content. Got:\n{}",
        output
    );
}

// @internal
#[test]
fn smoke_all_engine_screens_no_panic() {
    let (mut app, _dir) = create_app_with_identity();

    for screen in [
        Screen::MyInfo,
        Screen::Contacts,
        Screen::Exchange,
        Screen::Settings,
        Screen::Help,
        Screen::Backup,
        Screen::Delivery,
        Screen::Devices,
        Screen::Duress,
        Screen::Emergency,
        Screen::Sync,
        Screen::Recovery,
        Screen::Groups,
        Screen::Privacy,
        Screen::Support,
    ] {
        app.goto(screen);
        // Should not panic
        let output = render_to_string(&mut app, 80, 24);
        assert!(!output.is_empty(), "{:?} rendered empty", screen);
    }
}
