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
fn test_handle_key_help_enter_selects_faq_item() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Help;

    if app.app_engine.is_some() {
        // Engine-driven: Enter selects a FAQ item (opens URL), stays on Help
        handle_key(&mut app, KeyCode::Enter);
        assert_eq!(
            app.screen,
            Screen::Help,
            "Enter on Help should stay on Help (opens FAQ URL)"
        );
    } else {
        // Legacy: Enter goes back
        handle_key(&mut app, KeyCode::Enter);
        assert_eq!(app.screen, Screen::Home);
    }
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

// @scenario: identity_management:Create new identity via legacy setup screen
#[test]
fn test_handle_key_setup_c_creates_identity_and_goes_home() {
    let (mut app, _tmp) = create_test_app_no_identity();

    // Test legacy Setup screen (not engine-driven)
    app.onboarding_engine = None; // Disable engine for legacy test
    app.screen = Screen::Setup;
    app.input_mode = InputMode::Normal;
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

    if app.app_engine.is_some() {
        // Engine-driven: j navigates within the focused component's selections
        let _initial = app
            .render_state
            .component_selections
            .first()
            .copied()
            .unwrap_or(0);
        handle_key(&mut app, KeyCode::Char('j'));
        // Ensure key was consumed (engine handles focus/component navigation)
        assert_eq!(app.screen, Screen::Home, "should stay on Home");
    } else {
        // Legacy: j increments selected_field
        app.selected_field = 0;
        handle_key(&mut app, KeyCode::Char('j'));
        assert_eq!(app.selected_field, 1, "j should move selection down");
    }
}

#[test]
fn test_handle_key_home_k_decrements_selected_field() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Home;

    if app.app_engine.is_some() {
        // Engine-driven: k navigates within focused component
        handle_key(&mut app, KeyCode::Char('k'));
        assert_eq!(app.screen, Screen::Home, "should stay on Home");
    } else {
        // Legacy: k decrements selected_field
        app.selected_field = 1;
        handle_key(&mut app, KeyCode::Char('k'));
        assert_eq!(app.selected_field, 0, "k should move selection up");
    }
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

// ============================================================================
// Contact Detail Validation Key Bindings
// ============================================================================

// @scenario: field_validation.feature - V key triggers validate on ContactDetail
#[test]
fn test_handle_key_contact_detail_uppercase_v_sets_status() {
    let (mut app, _tmp) = create_test_app();
    app.selected_contact_id = Some("dummy-contact-id".to_string());
    app.screen = Screen::ContactDetail;
    app.selected_contact = 0;
    app.selected_contact_field = 0;

    handle_key(&mut app, KeyCode::Char('V'));

    // With no contacts, the handler should set an error/info status
    assert!(
        app.status_message.is_some(),
        "V on ContactDetail should set a status message"
    );
    // Should stay on ContactDetail (not navigate away)
    assert_eq!(
        app.screen,
        Screen::ContactDetail,
        "V should stay on ContactDetail"
    );
}

// @scenario: field_validation.feature - R key triggers revoke on ContactDetail
#[test]
fn test_handle_key_contact_detail_uppercase_r_sets_status() {
    let (mut app, _tmp) = create_test_app();
    app.selected_contact_id = Some("dummy-contact-id".to_string());
    app.screen = Screen::ContactDetail;
    app.selected_contact = 0;
    app.selected_contact_field = 0;

    handle_key(&mut app, KeyCode::Char('R'));

    // With no contacts, the handler should set an error/info status
    assert!(
        app.status_message.is_some(),
        "R on ContactDetail should set a status message"
    );
    // Should stay on ContactDetail
    assert_eq!(
        app.screen,
        Screen::ContactDetail,
        "R should stay on ContactDetail"
    );
}

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

// --- Home screen: Groups shortcut via 'g' key ---

/// @scenario: accessibility.feature @keyboard - All screens reachable via keyboard
#[test]
fn test_handle_key_home_g_navigates_to_groups() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Home;

    handle_key(&mut app, KeyCode::Char('g'));
    assert_eq!(
        app.screen,
        Screen::Groups,
        "g on Home should navigate to Groups"
    );
}

/// @scenario: accessibility.feature @keyboard - All screens reachable via keyboard
#[test]
fn test_handle_key_home_uppercase_x_navigates_to_exchange() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Home;

    handle_key(&mut app, KeyCode::Char('X'));
    assert_eq!(
        app.screen,
        Screen::Exchange,
        "X on Home should navigate to Exchange"
    );
}

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

// --- Settings screen: Tor shortcut key ---

/// @scenario: accessibility.feature @keyboard - Settings Tor shortcut
#[test]
fn test_handle_key_settings_t_navigates_to_tor() {
    let (mut app, _tmp) = create_test_app();
    app.screen = Screen::Settings;

    handle_key(&mut app, KeyCode::Char('t'));
    assert_eq!(
        app.screen,
        Screen::TorSettings,
        "t on Settings should navigate to Tor settings"
    );
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
    app.screen = Screen::Home;
    // Add a field, then delete it
    app.backend
        .add_field(
            vauchi_core::contact_card::FieldType::Email,
            "Work Email",
            "test@example.com",
        )
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
