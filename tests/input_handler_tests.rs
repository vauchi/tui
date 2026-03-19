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

use vauchi_core::ui::AppEngine;
use vauchi_core::{ContactField, FieldType, SymmetricKey, Vauchi, VauchiConfig};
use vauchi_tui::app::{App, EmergencyFocus, EmergencyState, InputMode, Screen};
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

fn create_app_engine(data_dir: &std::path::Path) -> AppEngine {
    let key = SymmetricKey::generate();
    let config = VauchiConfig {
        storage_path: data_dir.join("vauchi.db"),
        storage_key: Some(key),
        ..Default::default()
    };
    let vauchi: Vauchi = Vauchi::new(config).expect("vauchi");
    AppEngine::new(vauchi)
}

fn create_app_with_identity() -> (App, TempDir) {
    ensure_locales_loaded();
    let temp_dir = TempDir::new().expect("temp dir");
    let mut app_engine = create_app_engine(temp_dir.path());
    app_engine
        .vauchi_mut()
        .create_identity("Alice Smith")
        .expect("create identity");
    let app = App::new(
        app_engine,
        "wss://relay.vauchi.app".to_string(),
        temp_dir.path().to_path_buf(),
    );
    (app, temp_dir)
}

fn create_app_without_identity() -> (App, TempDir) {
    ensure_locales_loaded();
    let temp_dir = TempDir::new().expect("temp dir");
    let app_engine = create_app_engine(temp_dir.path());
    let app = App::new(
        app_engine,
        "wss://relay.vauchi.app".to_string(),
        temp_dir.path().to_path_buf(),
    );
    (app, temp_dir)
}

// ============================================================================
// tui-F-024: goto, go_back, set_status, clear_status
// ============================================================================

#[test]
fn test_goto_changes_screen() {
    let (mut app, _dir) = create_app_with_identity();
    assert_eq!(app.screen, Screen::MyInfo);

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
    assert_eq!(app.screen, Screen::MyInfo);
}

#[test]
fn test_go_back_from_settings_returns_to_more() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Settings);
    app.go_back();
    assert_eq!(app.screen, Screen::More);
}

#[test]
fn test_go_back_from_exchange_returns_to_home() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Exchange);
    app.go_back();
    assert_eq!(app.screen, Screen::MyInfo);
}

#[test]
fn test_go_back_from_help_returns_to_more() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Help);
    app.go_back();
    assert_eq!(app.screen, Screen::More);
}

#[test]
fn test_go_back_from_devices_returns_to_more() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Devices);
    app.go_back();
    assert_eq!(app.screen, Screen::More);
}

#[test]
fn test_go_back_from_recovery_returns_to_more() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Recovery);
    app.go_back();
    assert_eq!(app.screen, Screen::More);
}

#[test]
fn test_go_back_from_sync_returns_to_more() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Sync);
    app.go_back();
    assert_eq!(app.screen, Screen::More);
}

#[test]
fn test_go_back_from_tor_settings_returns_to_more() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::TorSettings);
    app.go_back();
    assert_eq!(app.screen, Screen::More);
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
    assert_eq!(app.screen, Screen::MyInfo);
}

#[test]
fn test_go_back_from_edit_field_returns_to_home() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::EditField);
    app.go_back();
    assert_eq!(app.screen, Screen::MyInfo);
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
    assert_eq!(app.screen, Screen::SetupWelcome);
    app.go_back();
    assert_eq!(
        app.screen,
        Screen::SetupWelcome,
        "go_back from SetupWelcome should stay on SetupWelcome"
    );
}

#[test]
fn test_go_back_from_backup_with_identity_goes_to_more() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Backup);
    app.go_back();
    assert_eq!(app.screen, Screen::More);
}

#[test]
fn test_go_back_from_backup_without_identity_goes_to_setup() {
    let (mut app, _dir) = create_app_without_identity();
    app.goto(Screen::Backup);
    app.go_back();
    assert_eq!(app.screen, Screen::SetupWelcome);
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
    app.goto(Screen::MyInfo);

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
    app.goto(Screen::MyInfo);

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
    assert_eq!(app.screen, Screen::MyInfo);
}

// @scenario: contacts_management:View all contacts
#[test]
fn test_handle_key_c_on_home_navigates_to_contacts() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::MyInfo);

    let action = handle_key(&mut app, KeyCode::Char('c'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Contacts);
}

#[test]
fn test_handle_key_s_on_home_navigates_to_settings() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::MyInfo);

    let action = handle_key(&mut app, KeyCode::Char('s'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Settings);
}

// @scenario: sync_updates:Client initiates sync with relay
#[test]
fn test_handle_key_n_on_home_navigates_to_sync() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::MyInfo);

    let action = handle_key(&mut app, KeyCode::Char('n'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Sync);
}

// @scenario: device_management:View linked devices
#[test]
fn test_handle_key_d_on_home_navigates_to_devices() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::MyInfo);

    let action = handle_key(&mut app, KeyCode::Char('d'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Devices);
}

// @scenario: identity_management:View recovery status
#[test]
fn test_handle_key_r_on_home_navigates_to_recovery() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::MyInfo);

    let action = handle_key(&mut app, KeyCode::Char('r'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Recovery);
}

// @scenario: identity_management:Create encrypted identity backup
#[test]
fn test_handle_key_b_on_home_navigates_to_backup() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::MyInfo);

    let action = handle_key(&mut app, KeyCode::Char('b'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Backup);
}

// @scenario: contact_card_management:Add a field to contact card
#[test]
fn test_handle_key_a_on_home_navigates_to_add_field() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::MyInfo);

    let action = handle_key(&mut app, KeyCode::Char('a'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::AddField);
}

#[test]
fn test_handle_key_in_editing_mode_esc_returns_to_normal() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::MyInfo);
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
    assert_eq!(app.screen, Screen::MyInfo);
}

#[test]
fn test_app_new_without_identity_starts_on_setup() {
    let (app, _dir) = create_app_without_identity();
    assert_eq!(app.screen, Screen::SetupWelcome);
}

// ============================================================================
// Delivery Screen (SP-12b)
// ============================================================================

// @scenario: message_delivery:Navigate to delivery screen
#[test]
fn test_handle_key_y_on_home_navigates_to_delivery() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::MyInfo);

    let action = handle_key(&mut app, KeyCode::Char('y'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Delivery);
}

// @scenario: message_delivery:Delivery screen esc goes back to home
#[test]
fn test_delivery_esc_goes_back_to_more() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Delivery);

    let action = handle_key(&mut app, KeyCode::Esc);
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::More);
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

// ============================================================================
// Duress screen tests
// ============================================================================

#[test]
fn test_go_back_from_duress_returns_to_settings() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Duress);
    assert_eq!(app.screen, Screen::Duress);
    app.go_back();
    assert_eq!(app.screen, Screen::Settings);
}

#[test]
fn test_settings_shift_d_navigates_to_duress() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Settings);
    let action = handle_key(&mut app, KeyCode::Char('D'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Duress);
}

#[test]
fn test_duress_state_defaults() {
    let (app, _dir) = create_app_with_identity();
    assert!(!app.duress_state.enabled);
    assert!(!app.duress_state.password_enabled);
    assert!(app.duress_state.pin_input.is_empty());
    assert!(app.duress_state.contact_ids_input.is_empty());
    assert_eq!(app.duress_state.alert_contact_count, 0);
}

#[test]
fn test_duress_esc_in_status_goes_back() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Duress);
    let action = handle_key(&mut app, KeyCode::Esc);
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Settings);
}

// ============================================================================
// Lock Screen Tests
// Feature: duress_pin.feature @unlock
// ============================================================================

#[test]
fn test_lock_screen_no_password_starts_on_home() {
    let (app, _dir) = create_app_with_identity();
    // No password configured → should start on Home, not Lock
    assert_eq!(app.screen, Screen::MyInfo);
}

#[test]
fn test_lock_screen_q_does_not_quit() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Lock);
    // 'q' should NOT quit from lock screen — it's a PIN character
    let action = handle_key(&mut app, KeyCode::Char('q'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Lock);
    assert_eq!(app.lock_state.pin_input, "q");
}

#[test]
fn test_lock_screen_esc_does_not_navigate() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Lock);
    let action = handle_key(&mut app, KeyCode::Esc);
    assert!(matches!(action, Action::Continue));
    // Should stay on Lock screen
    assert_eq!(app.screen, Screen::Lock);
}

#[test]
fn test_lock_screen_char_input_accumulates() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Lock);
    handle_key(&mut app, KeyCode::Char('1'));
    handle_key(&mut app, KeyCode::Char('2'));
    handle_key(&mut app, KeyCode::Char('3'));
    assert_eq!(app.lock_state.pin_input, "123");
}

#[test]
fn test_lock_screen_backspace_removes_char() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Lock);
    handle_key(&mut app, KeyCode::Char('a'));
    handle_key(&mut app, KeyCode::Char('b'));
    handle_key(&mut app, KeyCode::Backspace);
    assert_eq!(app.lock_state.pin_input, "a");
}

#[test]
fn test_lock_screen_empty_enter_does_nothing() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Lock);
    handle_key(&mut app, KeyCode::Enter);
    // Should stay on Lock — empty PIN doesn't attempt auth
    assert_eq!(app.screen, Screen::Lock);
    assert_eq!(app.lock_state.attempts, 0);
}

#[test]
fn test_lock_screen_wrong_pin_increments_attempts() {
    let (mut app, _dir) = create_app_with_identity();
    // Set up an app password
    app.app_engine
        .vauchi_mut()
        .setup_app_password("correctpin")
        .unwrap();
    app.goto(Screen::Lock);
    // Enter wrong PIN
    handle_key(&mut app, KeyCode::Char('w'));
    handle_key(&mut app, KeyCode::Char('r'));
    handle_key(&mut app, KeyCode::Char('o'));
    handle_key(&mut app, KeyCode::Char('n'));
    handle_key(&mut app, KeyCode::Char('g'));
    handle_key(&mut app, KeyCode::Enter);
    assert_eq!(app.screen, Screen::Lock);
    assert_eq!(app.lock_state.attempts, 1);
    assert!(app.lock_state.error);
    assert!(app.lock_state.pin_input.is_empty()); // cleared after failure
}

#[test]
fn test_lock_screen_correct_pin_unlocks() {
    let (mut app, _dir) = create_app_with_identity();
    app.app_engine
        .vauchi_mut()
        .setup_app_password("mypin")
        .unwrap();
    app.goto(Screen::Lock);
    for c in "mypin".chars() {
        handle_key(&mut app, KeyCode::Char(c));
    }
    handle_key(&mut app, KeyCode::Enter);
    assert_eq!(app.screen, Screen::MyInfo);
    assert!(!app.lock_state.error);
}

#[test]
fn test_lock_state_defaults() {
    let (app, _dir) = create_app_with_identity();
    assert!(app.lock_state.pin_input.is_empty());
    assert_eq!(app.lock_state.attempts, 0);
    assert!(!app.lock_state.error);
}

#[test]
fn test_lock_go_back_stays_on_lock() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Lock);
    app.go_back();
    assert_eq!(app.screen, Screen::Lock);
}

// ================================================================
// Emergency Broadcast Send + Confirmation Tests
// ================================================================

/// Feature: emergency_broadcast.feature @trigger @confirmation
#[test]
fn test_emergency_send_without_config_shows_error() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Emergency);
    // Not configured — pressing 's' should show error
    handle_key(&mut app, KeyCode::Char('s'));
    assert_eq!(app.emergency_state.focus, EmergencyFocus::Status);
    assert!(app.status_message.is_some());
}

/// Feature: emergency_broadcast.feature @trigger @confirmation
#[test]
fn test_emergency_send_enters_confirm_mode() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Emergency);
    // Configure emergency broadcast
    app.app_engine
        .vauchi_mut()
        .configure_emergency_broadcast(vec!["alice".to_string()], "Help me!".to_string(), false)
        .unwrap();
    app.emergency_state.configured = true;
    app.emergency_state.trusted_count = 1;

    // Press 's' to send
    handle_key(&mut app, KeyCode::Char('s'));
    assert_eq!(app.emergency_state.focus, EmergencyFocus::Confirm);
}

/// Feature: emergency_broadcast.feature @trigger @confirmation
#[test]
fn test_emergency_confirm_cancel_returns_to_status() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Emergency);
    app.emergency_state.configured = true;
    app.emergency_state.focus = EmergencyFocus::Confirm;

    // Press 'n' to cancel
    handle_key(&mut app, KeyCode::Char('n'));
    assert_eq!(app.emergency_state.focus, EmergencyFocus::Status);
}

/// Feature: emergency_broadcast.feature @trigger @confirmation
#[test]
fn test_emergency_confirm_esc_cancels() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Emergency);
    app.emergency_state.configured = true;
    app.emergency_state.focus = EmergencyFocus::Confirm;

    // Press Esc to cancel
    handle_key(&mut app, KeyCode::Esc);
    assert_eq!(app.emergency_state.focus, EmergencyFocus::Status);
}

/// Feature: emergency_broadcast.feature @edge @rate-limiting
#[test]
fn test_emergency_rate_limit_blocks_rapid_resend() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Emergency);
    app.app_engine
        .vauchi_mut()
        .configure_emergency_broadcast(vec!["alice".to_string()], "Help me!".to_string(), false)
        .unwrap();
    app.emergency_state.configured = true;
    app.emergency_state.trusted_count = 1;

    // Simulate a recent broadcast (now - 30 seconds)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    app.emergency_state.last_broadcast_time = Some(now - 30);

    // Press 's' — should be rate-limited, not enter Confirm
    handle_key(&mut app, KeyCode::Char('s'));
    assert_eq!(app.emergency_state.focus, EmergencyFocus::Status);
    assert!(app.status_message.is_some());
}

/// Feature: emergency_broadcast.feature @edge @rate-limiting
#[test]
fn test_emergency_rate_limit_allows_after_cooldown() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Emergency);
    app.app_engine
        .vauchi_mut()
        .configure_emergency_broadcast(vec!["alice".to_string()], "Help me!".to_string(), false)
        .unwrap();
    app.emergency_state.configured = true;
    app.emergency_state.trusted_count = 1;

    // Simulate a broadcast 61 seconds ago (past cooldown)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    app.emergency_state.last_broadcast_time = Some(now - 61);

    // Press 's' — should enter Confirm (past cooldown)
    handle_key(&mut app, KeyCode::Char('s'));
    assert_eq!(app.emergency_state.focus, EmergencyFocus::Confirm);
}

#[test]
fn test_emergency_state_defaults_include_last_broadcast() {
    let state = EmergencyState::default();
    assert!(!state.configured);
    assert!(state.last_broadcast_time.is_none());
    assert_eq!(state.focus, EmergencyFocus::Status);
}

// ============================================================================
// Contact Groups Tests (@groups feature from contacts_management.feature)
// ============================================================================

/// Test go_back from Groups screen returns to Home
#[test]
fn test_go_back_from_groups_returns_to_home() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Groups);
    assert_eq!(app.screen, Screen::Groups);
    app.go_back();
    assert_eq!(app.screen, Screen::MyInfo);
}

// ============================================================================
// SP-11: TUI Accessibility Improvements
// @scenario: accessibility.feature @keyboard @tui
// ============================================================================

/// @scenario: accessibility.feature @keyboard - Home 'g' navigates to Groups
#[test]
fn test_home_g_navigates_to_groups() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::MyInfo);

    let action = handle_key(&mut app, KeyCode::Char('g'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(
        app.screen,
        Screen::Groups,
        "g on Home should navigate to Groups"
    );
}

/// @scenario: accessibility.feature @keyboard - Home 'X' navigates to Exchange
#[test]
fn test_home_uppercase_x_navigates_to_exchange() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::MyInfo);

    let action = handle_key(&mut app, KeyCode::Char('X'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(
        app.screen,
        Screen::Exchange,
        "X on Home should navigate to Exchange"
    );
}

/// @scenario: accessibility.feature @keyboard - Home field delete status includes label
#[test]
fn test_home_field_delete_status_includes_label() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::MyInfo);

    // Add a field
    app.app_engine
        .vauchi()
        .add_own_field(ContactField::new(FieldType::Phone, "Mobile", "+1234567890"))
        .expect("add field");
    app.selected_field = 0;

    handle_key(&mut app, KeyCode::Char('x'));
    let msg = app
        .status_message
        .as_deref()
        .expect("delete should set status");
    assert!(
        msg.contains("Mobile"),
        "Delete status '{}' should contain field label 'Mobile'",
        msg
    );
}

/// @scenario: accessibility.feature @keyboard - Contact detail copy announces result
#[test]
fn test_contact_detail_copy_announces_result() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::ContactDetail);

    handle_key(&mut app, KeyCode::Char('c'));
    assert!(
        app.status_message.is_some(),
        "Copy should announce result in status bar"
    );
}

/// @scenario: accessibility.feature @keyboard - Settings 't' opens Tor settings
#[test]
fn test_settings_t_opens_tor_settings() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Settings);

    let action = handle_key(&mut app, KeyCode::Char('t'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(
        app.screen,
        Screen::TorSettings,
        "t on Settings should navigate to TorSettings"
    );
}

/// @scenario: accessibility.feature @keyboard - Contact delete with no contacts sets status
#[test]
fn test_contact_detail_delete_sets_status() {
    let (mut app, _dir) = create_app_with_identity();

    // Navigate to contact detail with no contacts
    app.goto(Screen::ContactDetail);
    app.selected_contact = 0;

    // Pressing 'x' with no contacts should stay on screen without crash
    handle_key(&mut app, KeyCode::Char('x'));
    // No contact to delete — should not crash, screen transitions back
    // The go_back transitions to Contacts
    assert!(
        app.screen == Screen::Contacts || app.screen == Screen::ContactDetail,
        "x on ContactDetail should navigate back or stay, got {:?}",
        app.screen
    );
}
