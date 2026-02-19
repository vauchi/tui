// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Keyboard Input Dispatch Tests (CRIT-09)
//!
//! Tests for `handle_key` — the entire keyboard dispatch layer.
//! Covers global keys, per-screen navigation, and editing mode.

use crossterm::event::KeyCode;
use tempfile::TempDir;

use vauchi_tui::app::{App, InputMode, Screen};
use vauchi_tui::backend::Backend;
use vauchi_tui::handlers::{handle_key, Action};

/// Create a test App with an initialized identity.
fn create_test_app() -> (App, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let backend = Backend::new(temp_dir.path()).expect("Failed to create backend");
    let mut app = App::new(backend);
    // App starts on Setup screen — create identity to get to Home
    app.backend
        .create_identity("Test User")
        .expect("Failed to create identity");
    app.screen = Screen::Home;
    (app, temp_dir)
}

/// Create a test App without an identity (stays on Setup screen).
fn create_test_app_no_identity() -> (App, TempDir) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let backend = Backend::new(temp_dir.path()).expect("Failed to create backend");
    let app = App::new(backend);
    (app, temp_dir)
}

// ============================================================================
// Global Keys
// ============================================================================

#[test]
fn test_handle_key_q_returns_quit_from_home() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Home;

    let action = handle_key(&mut app, KeyCode::Char('q'));
    assert!(matches!(action, Action::Quit), "q should quit from Home");
}

#[test]
fn test_handle_key_question_mark_navigates_to_help() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Home;

    let action = handle_key(&mut app, KeyCode::Char('?'));
    assert!(
        matches!(action, Action::Continue),
        "? should return Continue"
    );
    assert_eq!(app.screen, Screen::Help, "? should navigate to Help");
}

#[test]
fn test_handle_key_esc_goes_back_from_contacts() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Contacts;

    let action = handle_key(&mut app, KeyCode::Esc);
    assert!(matches!(action, Action::Continue));
    assert_eq!(
        app.screen,
        Screen::Home,
        "Esc from Contacts should go to Home"
    );
}

#[test]
fn test_handle_key_esc_stays_on_setup() {
    let (mut app, _tmp) = create_test_app_no_identity();
    assert_eq!(app.screen, Screen::Setup);

    handle_key(&mut app, KeyCode::Esc);
    assert_eq!(
        app.screen,
        Screen::Setup,
        "Esc from Setup should stay on Setup"
    );
}

// ============================================================================
// Home Screen Navigation
// ============================================================================

#[test]
fn test_handle_key_home_c_navigates_to_contacts() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Home;

    handle_key(&mut app, KeyCode::Char('c'));
    assert_eq!(app.screen, Screen::Contacts);
}

#[test]
fn test_handle_key_home_s_navigates_to_settings() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Home;

    handle_key(&mut app, KeyCode::Char('s'));
    assert_eq!(app.screen, Screen::Settings);
}

#[test]
fn test_handle_key_home_d_navigates_to_devices() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Home;

    handle_key(&mut app, KeyCode::Char('d'));
    assert_eq!(app.screen, Screen::Devices);
}

#[test]
fn test_handle_key_home_a_opens_add_field() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Home;

    handle_key(&mut app, KeyCode::Char('a'));
    assert_eq!(app.screen, Screen::AddField);
}

// ============================================================================
// Settings Screen Navigation
// ============================================================================

#[test]
fn test_handle_key_settings_n_opens_edit_name() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Settings;

    handle_key(&mut app, KeyCode::Char('n'));
    assert_eq!(app.screen, Screen::EditName);
    assert_eq!(
        app.input_mode,
        InputMode::Editing,
        "Edit name should enter editing mode"
    );
}

#[test]
fn test_handle_key_settings_u_opens_edit_relay_url() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Settings;

    handle_key(&mut app, KeyCode::Char('u'));
    assert_eq!(app.screen, Screen::EditRelayUrl);
    assert_eq!(app.input_mode, InputMode::Editing);
}

#[test]
fn test_handle_key_settings_b_navigates_to_backup() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Settings;

    handle_key(&mut app, KeyCode::Char('b'));
    assert_eq!(app.screen, Screen::Backup);
}

#[test]
fn test_handle_key_settings_p_navigates_to_privacy() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Settings;

    handle_key(&mut app, KeyCode::Char('p'));
    assert_eq!(app.screen, Screen::Privacy);
}

// ============================================================================
// Help Screen
// ============================================================================

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

#[test]
fn test_handle_key_help_enter_goes_back() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Help;

    handle_key(&mut app, KeyCode::Enter);
    assert_eq!(app.screen, Screen::Home);
}

// ============================================================================
// Editing Mode
// ============================================================================

#[test]
fn test_handle_key_editing_esc_returns_to_normal() {
    let (mut app, _tmp) = create_test_app();
    app.input_mode = InputMode::Editing;
    app.screen = Screen::Home;

    handle_key(&mut app, KeyCode::Esc);
    assert_eq!(
        app.input_mode,
        InputMode::Normal,
        "Esc in editing mode should return to normal"
    );
}

#[test]
fn test_handle_key_editing_char_appends_to_input_buffer() {
    let (mut app, _tmp) = create_test_app();
    app.input_mode = InputMode::Editing;
    app.screen = Screen::Home;
    app.input_buffer.clear();

    handle_key(&mut app, KeyCode::Char('a'));
    handle_key(&mut app, KeyCode::Char('b'));
    assert_eq!(
        app.input_buffer, "ab",
        "Chars in editing mode should append to input buffer"
    );
}

#[test]
fn test_handle_key_editing_backspace_removes_from_input_buffer() {
    let (mut app, _tmp) = create_test_app();
    app.input_mode = InputMode::Editing;
    app.screen = Screen::Home;
    app.input_buffer = "abc".to_string();

    handle_key(&mut app, KeyCode::Backspace);
    assert_eq!(app.input_buffer, "ab");
}

#[test]
fn test_handle_key_editing_enter_returns_to_normal() {
    let (mut app, _tmp) = create_test_app();
    app.input_mode = InputMode::Editing;
    app.screen = Screen::Home;

    handle_key(&mut app, KeyCode::Enter);
    assert_eq!(app.input_mode, InputMode::Normal);
}

// ============================================================================
// Setup Screen
// ============================================================================

#[test]
fn test_handle_key_setup_c_creates_identity_and_goes_home() {
    let (mut app, _tmp) = create_test_app_no_identity();
    assert_eq!(app.screen, Screen::Setup);

    handle_key(&mut app, KeyCode::Char('c'));
    assert_eq!(
        app.screen,
        Screen::Home,
        "c on Setup should create identity and go to Home"
    );
    assert!(
        app.backend.has_identity(),
        "Identity should exist after creation"
    );
}

#[test]
fn test_handle_key_setup_i_opens_backup_import() {
    let (mut app, _tmp) = create_test_app_no_identity();
    assert_eq!(app.screen, Screen::Setup);

    handle_key(&mut app, KeyCode::Char('i'));
    assert_eq!(app.screen, Screen::Backup);
    // Note: goto() resets input_mode to Normal, overriding the Editing
    // assignment in handle_setup_keys. This appears to be a bug — the
    // intent is to enter editing mode for backup data paste.
    assert_eq!(app.input_mode, InputMode::Normal);
}

// ============================================================================
// Contact Search Mode
// ============================================================================

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

#[test]
fn test_handle_key_search_mode_esc_exits_search() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Contacts;
    app.contact_search_mode = true;

    handle_key(&mut app, KeyCode::Esc);
    assert!(!app.contact_search_mode, "Esc should exit search mode");
}

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

#[test]
fn test_handle_key_home_j_increments_selected_field() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Home;
    // Add two fields so navigation works
    app.backend
        .add_field(
            vauchi_core::contact_card::FieldType::Email,
            "Work",
            "a@b.com",
        )
        .unwrap();
    app.backend
        .add_field(
            vauchi_core::contact_card::FieldType::Phone,
            "Mobile",
            "+1234567890",
        )
        .unwrap();
    app.selected_field = 0;

    handle_key(&mut app, KeyCode::Char('j'));
    assert_eq!(app.selected_field, 1, "j should move selection down");
}

#[test]
fn test_handle_key_home_k_decrements_selected_field() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Home;
    app.selected_field = 1;

    handle_key(&mut app, KeyCode::Char('k'));
    assert_eq!(app.selected_field, 0, "k should move selection up");
}

#[test]
fn test_handle_key_home_k_stays_at_zero() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Home;
    app.selected_field = 0;

    handle_key(&mut app, KeyCode::Char('k'));
    assert_eq!(app.selected_field, 0, "k at index 0 should stay at 0");
}

// ============================================================================
// Search mode blocks global keys
// ============================================================================

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
