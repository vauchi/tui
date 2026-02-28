// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Input Handler Tests (CRIT-09 + tui-F-024)
//!
//! Tests for handle_key dispatch, navigation (goto/go_back),
//! status message management, and global key bindings.

use std::sync::Once;
use tempfile::TempDir;

use crossterm::event::KeyCode;

use vauchi_tui::app::{App, InputMode, Screen};
use vauchi_tui::backend::Backend;
use vauchi_tui::handlers::{handle_key, Action};

static INIT_LOCALES: Once = Once::new();

fn ensure_locales_loaded() {
    INIT_LOCALES.call_once(|| {
        let candidates = [
            std::env::var("VAUCHI_LOCALES_DIR").ok(),
            Some("../locales".to_string()),
            Some("../core/vauchi-core/locales".to_string()),
        ];
        for candidate in candidates.iter().flatten() {
            let path = std::path::Path::new(candidate);
            if path.join("en.json").exists() && vauchi_core::i18n::init(path).is_ok() {
                return;
            }
        }
    });
}

fn create_app_with_identity() -> (App, TempDir) {
    ensure_locales_loaded();
    let temp_dir = TempDir::new().expect("temp dir");
    let mut backend = Backend::new(temp_dir.path()).expect("backend");
    backend
        .create_identity("Alice Smith")
        .expect("create identity");
    let app = App::new(backend);
    (app, temp_dir)
}

fn create_app_without_identity() -> (App, TempDir) {
    ensure_locales_loaded();
    let temp_dir = TempDir::new().expect("temp dir");
    let backend = Backend::new(temp_dir.path()).expect("backend");
    let app = App::new(backend);
    (app, temp_dir)
}

// ============================================================================
// tui-F-024: goto, go_back, set_status, clear_status
// ============================================================================

#[test]
fn test_goto_changes_screen() {
    let (mut app, _dir) = create_app_with_identity();
    assert_eq!(app.screen, Screen::Home);

    app.goto(Screen::Contacts);
    assert_eq!(app.screen, Screen::Contacts);

    app.goto(Screen::Settings);
    assert_eq!(app.screen, Screen::Settings);
}

#[test]
fn test_goto_resets_input_mode_to_normal() {
    let (mut app, _dir) = create_app_with_identity();
    app.input_mode = InputMode::Editing;

    app.goto(Screen::Contacts);
    assert_eq!(
        app.input_mode,
        InputMode::Normal,
        "goto should reset input_mode to Normal"
    );
}

#[test]
fn test_go_back_from_contacts_returns_to_home() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Contacts);
    app.go_back();
    assert_eq!(app.screen, Screen::Home);
}

#[test]
fn test_go_back_from_settings_returns_to_home() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Settings);
    app.go_back();
    assert_eq!(app.screen, Screen::Home);
}

#[test]
fn test_go_back_from_exchange_returns_to_home() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Exchange);
    app.go_back();
    assert_eq!(app.screen, Screen::Home);
}

#[test]
fn test_go_back_from_help_returns_to_home() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Help);
    app.go_back();
    assert_eq!(app.screen, Screen::Home);
}

#[test]
fn test_go_back_from_devices_returns_to_home() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Devices);
    app.go_back();
    assert_eq!(app.screen, Screen::Home);
}

#[test]
fn test_go_back_from_recovery_returns_to_home() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Recovery);
    app.go_back();
    assert_eq!(app.screen, Screen::Home);
}

#[test]
fn test_go_back_from_sync_returns_to_home() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Sync);
    app.go_back();
    assert_eq!(app.screen, Screen::Home);
}

#[test]
fn test_go_back_from_tor_settings_returns_to_home() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::TorSettings);
    app.go_back();
    assert_eq!(app.screen, Screen::Home);
}

#[test]
fn test_go_back_from_privacy_returns_to_settings() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Privacy);
    app.go_back();
    assert_eq!(app.screen, Screen::Settings);
}

#[test]
fn test_go_back_from_contact_detail_returns_to_contacts() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::ContactDetail);
    app.go_back();
    assert_eq!(app.screen, Screen::Contacts);
}

#[test]
fn test_go_back_from_contact_visibility_returns_to_detail() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::ContactVisibility);
    app.go_back();
    assert_eq!(app.screen, Screen::ContactDetail);
}

#[test]
fn test_go_back_from_add_field_returns_to_home() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::AddField);
    app.go_back();
    assert_eq!(app.screen, Screen::Home);
}

#[test]
fn test_go_back_from_edit_field_returns_to_home() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::EditField);
    app.go_back();
    assert_eq!(app.screen, Screen::Home);
}

#[test]
fn test_go_back_from_edit_name_returns_to_settings() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::EditName);
    app.go_back();
    assert_eq!(app.screen, Screen::Settings);
}

#[test]
fn test_go_back_from_edit_relay_url_returns_to_settings() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::EditRelayUrl);
    app.go_back();
    assert_eq!(app.screen, Screen::Settings);
}

#[test]
fn test_go_back_from_setup_stays_on_setup() {
    let (mut app, _dir) = create_app_without_identity();
    assert_eq!(app.screen, Screen::Setup);
    app.go_back();
    assert_eq!(
        app.screen,
        Screen::Setup,
        "go_back from Setup should stay on Setup"
    );
}

#[test]
fn test_go_back_from_backup_with_identity_goes_to_home() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Backup);
    app.go_back();
    assert_eq!(app.screen, Screen::Home);
}

#[test]
fn test_go_back_from_backup_without_identity_goes_to_setup() {
    let (mut app, _dir) = create_app_without_identity();
    app.goto(Screen::Backup);
    app.go_back();
    assert_eq!(app.screen, Screen::Setup);
}

#[test]
fn test_set_status_and_clear_status() {
    let (mut app, _dir) = create_app_with_identity();
    assert!(app.status_message.is_none());

    app.set_status("Test message");
    assert_eq!(app.status_message.as_deref(), Some("Test message"));

    app.clear_status();
    assert!(app.status_message.is_none());
}

// ============================================================================
// CRIT-09: handle_key dispatch tests
// ============================================================================

#[test]
fn test_handle_key_q_on_home_quits() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Home);

    let action = handle_key(&mut app, KeyCode::Char('q'));
    assert!(matches!(action, Action::Quit));
}

#[test]
fn test_handle_key_q_is_global_quit() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Contacts);

    // 'q' is a global quit on any screen (not screen-specific)
    let action = handle_key(&mut app, KeyCode::Char('q'));
    assert!(matches!(action, Action::Quit));
}

#[test]
fn test_handle_key_question_mark_navigates_to_help() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Home);

    let action = handle_key(&mut app, KeyCode::Char('?'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Help);
}

#[test]
fn test_handle_key_esc_goes_back() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Contacts);

    let action = handle_key(&mut app, KeyCode::Esc);
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Home);
}

// @scenario: contacts_management:View all contacts
#[test]
fn test_handle_key_c_on_home_navigates_to_contacts() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Home);

    let action = handle_key(&mut app, KeyCode::Char('c'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Contacts);
}

#[test]
fn test_handle_key_s_on_home_navigates_to_settings() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Home);

    let action = handle_key(&mut app, KeyCode::Char('s'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Settings);
}

// @scenario: sync_updates:Client initiates sync with relay
#[test]
fn test_handle_key_n_on_home_navigates_to_sync() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Home);

    let action = handle_key(&mut app, KeyCode::Char('n'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Sync);
}

// @scenario: device_management:View linked devices
#[test]
fn test_handle_key_d_on_home_navigates_to_devices() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Home);

    let action = handle_key(&mut app, KeyCode::Char('d'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Devices);
}

// @scenario: identity_management:View recovery status
#[test]
fn test_handle_key_r_on_home_navigates_to_recovery() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Home);

    let action = handle_key(&mut app, KeyCode::Char('r'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Recovery);
}

// @scenario: identity_management:Create encrypted identity backup
#[test]
fn test_handle_key_b_on_home_navigates_to_backup() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Home);

    let action = handle_key(&mut app, KeyCode::Char('b'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Backup);
}

// @scenario: contact_card_management:Add a field to contact card
#[test]
fn test_handle_key_a_on_home_navigates_to_add_field() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Home);

    let action = handle_key(&mut app, KeyCode::Char('a'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::AddField);
}

#[test]
fn test_handle_key_in_editing_mode_esc_returns_to_normal() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Home);
    app.input_mode = InputMode::Editing;

    let action = handle_key(&mut app, KeyCode::Esc);
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.input_mode, InputMode::Normal);
}

// ============================================================================
// App::new initial screen selection
// ============================================================================

#[test]
fn test_app_new_with_identity_starts_on_home() {
    let (app, _dir) = create_app_with_identity();
    assert_eq!(app.screen, Screen::Home);
}

#[test]
fn test_app_new_without_identity_starts_on_setup() {
    let (app, _dir) = create_app_without_identity();
    assert_eq!(app.screen, Screen::Setup);
}

// ============================================================================
// Delivery Screen (SP-12b)
// ============================================================================

// @scenario: message_delivery:Navigate to delivery screen
#[test]
fn test_handle_key_y_on_home_navigates_to_delivery() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Home);

    let action = handle_key(&mut app, KeyCode::Char('y'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Delivery);
}

// @scenario: message_delivery:Delivery screen esc goes back to home
#[test]
fn test_delivery_esc_goes_back_to_home() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Delivery);

    let action = handle_key(&mut app, KeyCode::Esc);
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Home);
}

// @scenario: message_delivery:Delivery retry key sets status
#[test]
fn test_delivery_r_runs_retry() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Delivery);

    let action = handle_key(&mut app, KeyCode::Char('r'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Delivery);
    assert!(
        app.delivery_state.last_result.is_some(),
        "Retry should set last_result"
    );
}

// @scenario: message_delivery:Delivery cleanup key sets status
#[test]
fn test_delivery_c_runs_cleanup() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Delivery);

    let action = handle_key(&mut app, KeyCode::Char('c'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Delivery);
    assert!(
        app.delivery_state.last_result.is_some(),
        "Cleanup should set last_result"
    );
}

// @scenario: message_delivery:Delivery state has correct defaults
#[test]
fn test_delivery_state_defaults() {
    let (app, _dir) = create_app_with_identity();
    assert_eq!(app.delivery_state.queued, 0);
    assert_eq!(app.delivery_state.delivered, 0);
    assert_eq!(app.delivery_state.failed, 0);
    assert!(app.delivery_state.last_result.is_none());
}
