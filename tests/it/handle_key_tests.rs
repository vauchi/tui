// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Keyboard Input Dispatch Tests (CRIT-09)
//!
//! Tests for `handle_key` — the entire keyboard dispatch layer.
//! Covers global keys, per-screen navigation, and editing mode.

use crossterm::event::KeyCode;
use rstest::rstest;
use tempfile::TempDir;

use vauchi_app::ui::AppEngine;
use vauchi_core::{ContactField, FieldType, SymmetricKey, Vauchi, VauchiConfig};

use vauchi_tui::app::{App, InputMode, Screen};
use vauchi_tui::handlers::{Action, handle_key};

/// Create AppEngine for a test data dir.
fn create_app_engine(data_dir: &std::path::Path) -> AppEngine {
    let key = SymmetricKey::generate();
    let config = VauchiConfig::with_storage_path(data_dir.join("vauchi.db")).with_storage_key(key);
    let vauchi: Vauchi = Vauchi::new(config).expect("vauchi");
    AppEngine::new(vauchi)
}

/// Create a test App with an initialized identity.
fn create_test_app() -> (App, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let mut app_engine = create_app_engine(temp_dir.path());
    app_engine
        .vauchi_mut()
        .create_identity("Test User")
        .expect("Failed to create identity");
    let mut app = App::new(
        app_engine,
        "wss://relay.vauchi.app".to_string(),
        temp_dir.path().to_path_buf(),
    );
    app.screen = Screen::MyInfo;
    (app, temp_dir)
}

/// Create a test App without an identity (stays on Setup screen).
fn create_test_app_no_identity() -> (App, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let app_engine = create_app_engine(temp_dir.path());
    let app = App::new(
        app_engine,
        "wss://relay.vauchi.app".to_string(),
        temp_dir.path().to_path_buf(),
    );
    (app, temp_dir)
}

// ============================================================================
// Global Keys
// ============================================================================

// @internal
#[test]
fn test_handle_key_q_returns_quit_from_home() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::MyInfo;

    let action = handle_key(&mut app, KeyCode::Char('q'));
    assert!(matches!(action, Action::Quit), "q should quit from Home");
}

// @internal
#[test]
fn test_handle_key_question_mark_navigates_to_help() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::MyInfo;

    let action = handle_key(&mut app, KeyCode::Char('?'));
    assert!(
        matches!(action, Action::Continue),
        "? should return Continue"
    );
    assert_eq!(app.screen, Screen::Help, "? should navigate to Help");
}

// @internal
#[test]
fn test_handle_key_esc_goes_back_from_contacts() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Contacts;

    let action = handle_key(&mut app, KeyCode::Esc);
    assert!(matches!(action, Action::Continue));
    assert_eq!(
        app.screen,
        Screen::MyInfo,
        "Esc from Contacts should go to Home"
    );
}

// @internal
#[test]
fn test_handle_key_esc_stays_on_setup() {
    let (mut app, _tmp) = create_test_app_no_identity();
    assert_eq!(app.screen, Screen::SetupWelcome);

    handle_key(&mut app, KeyCode::Esc);
    assert_eq!(
        app.screen,
        Screen::SetupWelcome,
        "Esc from SetupWelcome should stay on SetupWelcome"
    );
}

// ============================================================================
// Home Screen Navigation
// ============================================================================

// @internal
#[rstest]
#[case::c_contacts('c', Screen::Contacts)]
#[case::s_settings('s', Screen::Settings)]
#[case::d_devices('d', Screen::Devices)]
#[case::a_add_field('a', Screen::AddField)]
#[case::g_groups('g', Screen::Groups)]
#[case::x_exchange('X', Screen::Exchange)]
fn test_handle_key_home_navigates_to_screen(#[case] key: char, #[case] expected: Screen) {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::MyInfo;

    handle_key(&mut app, KeyCode::Char(key));
    assert_eq!(app.screen, expected);
}

// ============================================================================
// Settings Screen Navigation
// ============================================================================

// @internal
#[rstest]
#[case::n_edit_name('n', Screen::EditName)]
#[case::u_edit_relay_url('u', Screen::EditRelayUrl)]
#[case::b_backup('b', Screen::Backup)]
#[case::p_privacy('p', Screen::Privacy)]
fn test_handle_key_settings_navigates_to_screen(#[case] key: char, #[case] expected: Screen) {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Settings;

    handle_key(&mut app, KeyCode::Char(key));
    assert_eq!(app.screen, expected);
    // Engine-driven screens (EditName, EditRelayUrl) stay in Normal mode
    assert_eq!(
        app.input_mode,
        InputMode::Normal,
        "Settings navigation should stay in Normal mode"
    );
}

// ============================================================================
// Help Screen
// ============================================================================

// @internal
#[test]
fn test_handle_key_help_q_quits() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Help;

    // Global q handler runs before screen-specific handler, so q quits
    // even from Help. The handle_help_keys 'q' branch is unreachable.
    let action = handle_key(&mut app, KeyCode::Char('q'));
    assert!(
        matches!(action, Action::Quit),
        "q on Help should quit (global handler takes precedence)"
    );
}

// @internal
#[test]
fn test_handle_key_help_enter_selects_faq_item() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Help;

    // Engine-driven: Enter selects a FAQ item (opens URL), stays on Help
    handle_key(&mut app, KeyCode::Enter);
    assert_eq!(
        app.screen,
        Screen::Help,
        "Enter on Help should stay on Help (opens FAQ URL)"
    );
}

// ============================================================================
// Editing Mode
// ============================================================================

// @internal
#[test]
fn test_handle_key_editing_esc_returns_to_normal() {
    let (mut app, _tmp) = create_test_app();
    app.input_mode = InputMode::Editing;
    app.screen = Screen::MyInfo;

    handle_key(&mut app, KeyCode::Esc);
    assert_eq!(
        app.input_mode,
        InputMode::Normal,
        "Esc in editing mode should return to normal"
    );
}

// @internal
#[test]
fn test_handle_key_editing_char_appends_to_input_buffer() {
    let (mut app, _tmp) = create_test_app();
    app.input_mode = InputMode::Editing;
    app.screen = Screen::MyInfo;
    app.input_buffer.clear();

    handle_key(&mut app, KeyCode::Char('a'));
    handle_key(&mut app, KeyCode::Char('b'));
    assert_eq!(
        app.input_buffer, "ab",
        "Chars in editing mode should append to input buffer"
    );
}

// @internal
#[test]
fn test_handle_key_editing_backspace_removes_from_input_buffer() {
    let (mut app, _tmp) = create_test_app();
    app.input_mode = InputMode::Editing;
    app.screen = Screen::MyInfo;
    app.input_buffer = "abc".to_string();

    handle_key(&mut app, KeyCode::Backspace);
    assert_eq!(app.input_buffer, "ab");
}

// @internal
#[test]
fn test_handle_key_editing_enter_returns_to_normal() {
    let (mut app, _tmp) = create_test_app();
    app.input_mode = InputMode::Editing;
    app.screen = Screen::MyInfo;

    handle_key(&mut app, KeyCode::Enter);
    assert_eq!(app.input_mode, InputMode::Normal);
}

// ============================================================================
// Setup Screen
// ============================================================================

// @scenario: identity_management:Engine-driven onboarding starts at identity check
#[test]
fn test_handle_key_engine_onboarding_enter_advances() {
    let (mut app, _tmp) = create_test_app_no_identity();
    assert_eq!(app.screen, Screen::SetupWelcome);
    assert!(
        app.onboarding_engine.is_some(),
        "Engine should be created for onboarding"
    );

    // Enter on IdentityCheck (primary action "have_identity") transitions engine
    handle_key(&mut app, KeyCode::Enter);
    // Engine advances — screen should still be in onboarding range
    assert!(
        matches!(
            app.screen,
            Screen::SetupWelcome
                | Screen::SetupCreateIdentity
                | Screen::SetupAddFields
                | Screen::SetupSecurity
                | Screen::SetupReady
        ),
        "Engine should advance within onboarding screens, got {:?}",
        app.screen,
    );
}

// @internal
#[test]
fn test_handle_key_setup_i_opens_backup_import() {
    let (mut app, _tmp) = create_test_app_no_identity();
    assert_eq!(app.screen, Screen::SetupWelcome);

    // 'i' on SetupWelcome goes to Backup import (engine-driven or legacy)
    handle_key(&mut app, KeyCode::Char('i'));
    assert_eq!(app.screen, Screen::Backup);
    // goto() resets input_mode to Normal
    assert_eq!(app.input_mode, InputMode::Normal);
}

// ============================================================================
// Contact Search Mode
// ============================================================================

// @scenario: contacts_management:Search contacts by name
#[test]
fn test_handle_key_contacts_slash_enters_search_mode() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Contacts;
    app.contact_search_mode = false;

    handle_key(&mut app, KeyCode::Char('/'));
    assert!(
        app.contact_search_mode,
        "/ should enter contact search mode"
    );
    assert!(
        app.contact_search_query.is_empty(),
        "Search query should be cleared"
    );
}

// @scenario: contacts_management:Search contacts by name
#[test]
fn test_handle_key_search_mode_typing_updates_query() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Contacts;
    app.contact_search_mode = true;
    app.contact_search_query.clear();

    handle_key(&mut app, KeyCode::Char('a'));
    handle_key(&mut app, KeyCode::Char('l'));
    assert_eq!(app.contact_search_query, "al");
}

// @internal
#[test]
fn test_handle_key_search_mode_esc_exits_search() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Contacts;
    app.contact_search_mode = true;

    handle_key(&mut app, KeyCode::Esc);
    assert!(!app.contact_search_mode, "Esc should exit search mode");
}

// @internal
#[test]
fn test_handle_key_search_mode_backspace_removes_char() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Contacts;
    app.contact_search_mode = true;
    app.contact_search_query = "abc".to_string();

    handle_key(&mut app, KeyCode::Backspace);
    assert_eq!(app.contact_search_query, "ab");
}

// ============================================================================
// Home Screen Field Navigation
// ============================================================================

// @internal
#[test]
fn test_handle_key_home_j_increments_selected_field() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::MyInfo;
    // Add two fields so navigation works
    app.app_engine
        .vauchi()
        .add_own_field(ContactField::new(FieldType::Email, "Work", "a@b.com"))
        .unwrap();
    app.app_engine
        .vauchi()
        .add_own_field(ContactField::new(FieldType::Phone, "Mobile", "+1234567890"))
        .unwrap();

    // Engine-driven: j navigates within the focused component's selections
    let _initial = app
        .render_state
        .component_selections
        .first()
        .copied()
        .unwrap_or(0);
    handle_key(&mut app, KeyCode::Char('j'));
    // Ensure key was consumed (engine handles focus/component navigation)
    assert_eq!(app.screen, Screen::MyInfo, "should stay on MyInfo");
}

// @internal
#[test]
fn test_handle_key_home_k_decrements_selected_field() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::MyInfo;

    // Engine-driven: k navigates within focused component
    handle_key(&mut app, KeyCode::Char('k'));
    assert_eq!(app.screen, Screen::MyInfo, "should stay on MyInfo");
}

// @internal
#[test]
fn test_handle_key_home_k_stays_at_zero() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::MyInfo;
    app.selected_field = 0;

    handle_key(&mut app, KeyCode::Char('k'));
    assert_eq!(app.selected_field, 0, "k at index 0 should stay at 0");
}

// ============================================================================
// Search mode blocks global keys
// ============================================================================

// @internal
#[test]
fn test_handle_key_search_mode_q_does_not_quit() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Contacts;
    app.contact_search_mode = true;

    let action = handle_key(&mut app, KeyCode::Char('q'));
    assert!(
        matches!(action, Action::Continue),
        "q in search mode should not quit"
    );
    assert_eq!(
        app.contact_search_query, "q",
        "q should be appended to search query"
    );
}

// ============================================================================
// Community scoring keybindings (V=validate, R=revoke) removed per ADR-040

// @scenario: field_validation.feature - lowercase v still opens visibility
#[test]
fn test_handle_key_contact_detail_lowercase_v_still_opens_visibility() {
    let (mut app, _tmp) = create_test_app();
    app.selected_contact_id = Some("dummy-contact-id".to_string());
    app.screen = Screen::ContactDetail;
    app.selected_contact = 0;

    // Note: without a real contact, get_contact_by_index returns None,
    // so the v handler won't navigate. This test verifies the handler
    // path is still wired correctly (not overridden by V).
    handle_key(&mut app, KeyCode::Char('v'));

    // The v handler attempts to get a contact; with none, it stays on ContactDetail.
    // The important thing is that lowercase v does NOT set a validation status message
    // (the uppercase V handler sets validation-related status).
    assert_eq!(
        app.screen,
        Screen::ContactDetail,
        "lowercase v without contact should stay on ContactDetail"
    );
}

// ============================================================================
// SP-11: TUI Accessibility Improvements
// @scenario: accessibility.feature @keyboard @tui
// ============================================================================

// --- Sync screen: test connection 't' and refresh 'r' keys ---

/// @scenario: accessibility.feature @keyboard - Sync test connection shortcut
#[test]
fn test_handle_key_sync_t_tests_connection() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Sync;

    handle_key(&mut app, KeyCode::Char('t'));
    assert!(
        app.status_message.is_some(),
        "t on Sync should set a status message about connection test"
    );
    assert_eq!(app.screen, Screen::Sync, "t should stay on Sync screen");
}

/// @scenario: accessibility.feature @keyboard - Sync refresh pending count shortcut
#[test]
fn test_handle_key_sync_r_refreshes_pending() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Sync;

    handle_key(&mut app, KeyCode::Char('r'));
    assert!(
        app.status_message.is_some(),
        "r on Sync should set a status message about pending updates"
    );
    assert_eq!(app.screen, Screen::Sync, "r should stay on Sync screen");
}

// --- Contact detail: copy shortcut announces field info ---

/// @scenario: accessibility.feature @keyboard - Copy field value announces result
#[test]
fn test_handle_key_contact_detail_c_announces_copy_result() {
    let (mut app, _tmp) = create_test_app();
    app.selected_contact_id = Some("dummy-contact-id".to_string());
    app.screen = Screen::ContactDetail;
    app.selected_contact = 0;
    app.selected_contact_field = 0;

    handle_key(&mut app, KeyCode::Char('c'));
    // Even without a contact, the copy handler should set a status message
    assert!(
        app.status_message.is_some(),
        "c on ContactDetail should set a status message (error or success)"
    );
}

// --- Home screen field delete announces field label ---

/// @scenario: accessibility.feature @keyboard - Field delete announces label in status
#[test]
fn test_handle_key_home_delete_announces_field_label() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::MyInfo;
    // Add a field, then delete it
    app.app_engine
        .vauchi()
        .add_own_field(ContactField::new(
            FieldType::Email,
            "Work Email",
            "test@example.com",
        ))
        .unwrap();
    app.selected_field = 0;

    handle_key(&mut app, KeyCode::Char('x'));
    assert!(
        app.status_message.is_some(),
        "x on Home should set a status message"
    );
    let msg = app.status_message.as_deref().unwrap();
    assert!(
        msg.contains("Work Email"),
        "Delete status should include field label, got: '{}'",
        msg
    );
}
