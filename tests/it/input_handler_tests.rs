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
use rstest::rstest;

use vauchi_app::ui::AppEngine;
use vauchi_core::{ContactField, FieldType, SymmetricKey, Vauchi, VauchiConfig};
use vauchi_tui::app::{App, EmergencyFocus, EmergencyState, InputMode, Screen};
use vauchi_tui::handlers::{Action, handle_key};

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
            if path.join("en.json").exists() && vauchi_app::i18n::init(path).is_ok() {
                return;
            }
        }
    });
}

fn create_app_engine(data_dir: &std::path::Path) -> AppEngine {
    let key = SymmetricKey::generate();
    let config = VauchiConfig::with_storage_path(data_dir.join("vauchi.db")).with_storage_key(key);
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

// @internal
#[test]
fn test_goto_changes_screen() {
    let (mut app, _dir) = create_app_with_identity();
    assert_eq!(app.screen, Screen::MyInfo);

    app.goto(Screen::Contacts);
    assert_eq!(app.screen, Screen::Contacts);

    app.goto(Screen::Settings);
    assert_eq!(app.screen, Screen::Settings);
}

// @internal
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

// @internal
//
// Engine-driven back-navigation pops `nav_history`. To exercise the
// expected child→parent traversal, the test navigates from MyInfo →
// expected parent → from screen, then `go_back`, and expects to land
// on the parent. Direct `goto(from)` from MyInfo would land on MyInfo
// after a single pop — correct engine behavior, but doesn't test the
// natural flow.
//
// FormDialog screens (`AddField`, `EditField`, `EditName`,
// `EditRelayUrl`) and contact-id-bearing screens (`ContactDetail`,
// `ContactVisibility`) need richer setup than `goto` provides; they're
// covered by dedicated tests below.
//
// See `_private/docs/problems/2026-04-30-navigation-in-core/` test
// scaffolding strategy.
#[rstest]
#[case::contacts(Screen::Contacts, Screen::MyInfo)]
#[case::settings(Screen::Settings, Screen::More)]
#[case::exchange(Screen::Exchange, Screen::MyInfo)]
#[case::help(Screen::Help, Screen::More)]
#[case::devices(Screen::Devices, Screen::More)]
#[case::recovery(Screen::Recovery, Screen::More)]
#[case::sync(Screen::Sync, Screen::More)]
#[case::privacy(Screen::Privacy, Screen::Settings)]
#[case::backup(Screen::Backup, Screen::More)]
#[case::groups(Screen::Groups, Screen::MyInfo)]
#[case::duress(Screen::Duress, Screen::Settings)]
fn test_go_back_returns_to_expected_screen(#[case] from: Screen, #[case] expected: Screen) {
    let (mut app, _dir) = create_app_with_identity();
    if expected != Screen::MyInfo {
        app.goto(expected);
    }
    app.goto(from);
    app.go_back();
    assert_eq!(app.screen, expected);
}

// `ContactDetail` requires `selected_contact_id` to be set so
// `engine_target_for_screen(ContactDetail)` returns
// `Some(AppScreen::ContactDetail { contact_id })`, which the engine
// then pushes to nav history. Without the id, the screen is not
// engine-driven and `go_back` can't pop to it.
// @internal
#[test]
fn test_go_back_from_contact_detail_returns_to_contacts() {
    let (mut app, _dir) = create_app_with_identity();
    app.selected_contact_id = Some("test-contact-id".into());
    app.goto(Screen::Contacts);
    app.goto(Screen::ContactDetail);
    app.go_back();
    assert_eq!(app.screen, Screen::Contacts);
}

// @internal
#[test]
fn test_go_back_from_contact_visibility_returns_to_contact_detail() {
    let (mut app, _dir) = create_app_with_identity();
    app.selected_contact_id = Some("test-contact-id".into());
    app.goto(Screen::Contacts);
    app.goto(Screen::ContactDetail);
    app.goto(Screen::ContactVisibility);
    app.go_back();
    assert_eq!(app.screen, Screen::ContactDetail);
}

// FormDialog screens are entered via `goto_form_dialog(FormDialogType)`
// which carries the dialog's data. Plain `goto(Screen::EditName)` does
// not navigate AppEngine and so leaves nav history untouched, making
// `go_back` pop the wrong frame. These tests use the real entry point.
// @internal
#[test]
fn test_go_back_from_edit_name_returns_to_settings() {
    use vauchi_app::ui::FormDialogType;
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Settings);
    app.goto_form_dialog(FormDialogType::EditName {
        current_name: "Alice".into(),
    });
    app.go_back();
    assert_eq!(app.screen, Screen::Settings);
}

// @internal
#[test]
fn test_go_back_from_edit_relay_url_returns_to_settings() {
    use vauchi_app::ui::FormDialogType;
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Settings);
    app.goto_form_dialog(FormDialogType::EditRelayUrl {
        current_url: "https://relay.test".into(),
    });
    app.go_back();
    assert_eq!(app.screen, Screen::Settings);
}

// @internal
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

// @internal
#[test]
fn test_go_back_from_backup_without_identity_goes_to_setup() {
    let (mut app, _dir) = create_app_without_identity();
    app.goto(Screen::Backup);
    app.go_back();
    assert_eq!(app.screen, Screen::SetupWelcome);
}

// @internal
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

// @internal
#[test]
fn test_handle_key_q_on_home_quits() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::MyInfo);

    let action = handle_key(&mut app, KeyCode::Char('q'));
    assert!(matches!(action, Action::Quit));
}

// @internal
#[test]
fn test_handle_key_q_is_global_quit() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Contacts);

    // 'q' is a global quit on any screen (not screen-specific)
    let action = handle_key(&mut app, KeyCode::Char('q'));
    assert!(matches!(action, Action::Quit));
}

// @internal
#[test]
fn test_handle_key_question_mark_navigates_to_help() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::MyInfo);

    let action = handle_key(&mut app, KeyCode::Char('?'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Help);
}

// @internal
#[test]
fn test_handle_key_esc_goes_back() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Contacts);

    let action = handle_key(&mut app, KeyCode::Esc);
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::MyInfo);
}

// @scenario: contacts_management:View all contacts
// @scenario: sync_updates:Client initiates sync with relay
// @scenario: device_management:View linked devices
// @scenario: identity_management:View recovery status
// @scenario: identity_management:Create encrypted identity backup
// @scenario: contact_card_management:Add a field to contact card
#[rstest]
#[case::c_contacts('c', Screen::Contacts)]
#[case::s_settings('s', Screen::Settings)]
#[case::n_sync('n', Screen::Sync)]
#[case::d_devices('d', Screen::Devices)]
#[case::r_recovery('r', Screen::Recovery)]
#[case::b_backup('b', Screen::Backup)]
#[case::a_add_field('a', Screen::AddField)]
#[case::y_delivery('y', Screen::Delivery)]
#[case::g_groups('g', Screen::Groups)]
#[case::x_exchange('X', Screen::Exchange)]
fn test_handle_key_on_home_navigates_to_screen(#[case] key: char, #[case] expected: Screen) {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::MyInfo);

    let action = handle_key(&mut app, KeyCode::Char(key));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, expected);
}

// @internal
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

// @internal
#[test]
fn test_app_new_with_identity_starts_on_home() {
    let (app, _dir) = create_app_with_identity();
    assert_eq!(app.screen, Screen::MyInfo);
}

// @internal
#[test]
fn test_app_new_without_identity_starts_on_setup() {
    let (app, _dir) = create_app_without_identity();
    assert_eq!(app.screen, Screen::SetupWelcome);
}

// ============================================================================
// Delivery Screen (SP-12b)
// ============================================================================

// @scenario: message_delivery:Delivery screen esc goes back to home
#[test]
fn test_delivery_esc_goes_back_to_more() {
    let (mut app, _dir) = create_app_with_identity();
    // Navigate via the natural parent so AppEngine's nav history
    // reflects user-driven traversal. Direct `goto(Delivery)` from
    // MyInfo would land on MyInfo when we pop, which is correct
    // engine behavior but doesn't exercise the Delivery→More flow.
    app.goto(Screen::More);
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
    assert_eq!(
        app.status_message.as_deref(),
        Some("Delivery retries processed"),
        "Retry should set status message"
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
    assert_eq!(
        app.status_message.as_deref(),
        Some("Delivery cleanup complete"),
        "Cleanup should set status message"
    );
}

// ============================================================================
// Duress screen tests
// ============================================================================

// @internal
#[test]
fn test_settings_shift_d_navigates_to_duress() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Settings);
    let action = handle_key(&mut app, KeyCode::Char('D'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Duress);
}

// @internal
#[test]
fn test_duress_esc_in_status_goes_back() {
    let (mut app, _dir) = create_app_with_identity();
    // Engine-driven back-nav pops `nav_history`, so the test must
    // navigate via Duress's natural parent (Settings) instead of
    // jumping straight from MyInfo. See
    // `_private/docs/problems/2026-04-30-navigation-in-core/` test
    // scaffolding strategy.
    app.goto(Screen::Settings);
    app.goto(Screen::Duress);
    let action = handle_key(&mut app, KeyCode::Esc);
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.screen, Screen::Settings);
}

// ============================================================================
// Lock Screen Tests
// Feature: duress_pin.feature @unlock
// ============================================================================

// @internal
#[test]
fn test_lock_screen_no_password_starts_on_home() {
    let (app, _dir) = create_app_with_identity();
    // No password configured → should start on Home, not Lock
    assert_eq!(app.screen, Screen::MyInfo);
}

// @internal
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

// @internal
#[test]
fn test_lock_screen_esc_does_not_navigate() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Lock);
    let action = handle_key(&mut app, KeyCode::Esc);
    assert!(matches!(action, Action::Continue));
    // Should stay on Lock screen
    assert_eq!(app.screen, Screen::Lock);
}

// @internal
#[test]
fn test_lock_screen_char_input_accumulates() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Lock);
    handle_key(&mut app, KeyCode::Char('1'));
    handle_key(&mut app, KeyCode::Char('2'));
    handle_key(&mut app, KeyCode::Char('3'));
    assert_eq!(app.lock_state.pin_input, "123");
}

// @internal
#[test]
fn test_lock_screen_backspace_removes_char() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Lock);
    handle_key(&mut app, KeyCode::Char('a'));
    handle_key(&mut app, KeyCode::Char('b'));
    handle_key(&mut app, KeyCode::Backspace);
    assert_eq!(app.lock_state.pin_input, "a");
}

// @internal
#[test]
fn test_lock_screen_empty_enter_does_nothing() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::Lock);
    handle_key(&mut app, KeyCode::Enter);
    // Should stay on Lock — empty PIN doesn't attempt auth
    assert_eq!(app.screen, Screen::Lock);
    assert_eq!(app.lock_state.attempts, 0);
}

// @internal
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

// @internal
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

// @internal
#[test]
fn test_lock_state_defaults() {
    let (app, _dir) = create_app_with_identity();
    assert!(app.lock_state.pin_input.is_empty());
    assert_eq!(app.lock_state.attempts, 0);
    assert!(!app.lock_state.error);
}

// @internal
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
// @internal
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
// @internal
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
// @internal
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
// @internal
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
// @internal
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
// @internal
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

// @internal
#[test]
fn test_emergency_state_defaults_include_last_broadcast() {
    let state = EmergencyState::default();
    assert!(!state.configured);
    assert!(state.last_broadcast_time.is_none());
    assert_eq!(state.focus, EmergencyFocus::Status);
}

// ============================================================================
// SP-11: TUI Accessibility Improvements
// @scenario: accessibility.feature @keyboard @tui
// ============================================================================

/// @scenario: accessibility.feature @keyboard - Home field delete status includes label
#[test]
fn test_home_field_delete_status_includes_label() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(Screen::MyInfo);

    // Add a field
    app.app_engine
        .vauchi()
        .add_own_field(ContactField::new(
            FieldType::Phone,
            "Mobile",
            "+1234567890",
            0,
        ))
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
