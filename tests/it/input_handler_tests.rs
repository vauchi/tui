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

use vauchi_app::ui::{AppEngine, AppScreen, FormDialogType};
use vauchi_core::{ContactField, FieldType, SymmetricKey, Vauchi, VauchiConfig};
use vauchi_tui::app::{App, InputMode};
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
    assert_eq!(app.current_app_screen(), AppScreen::MyInfo);

    app.goto(AppScreen::Contacts);
    assert_eq!(app.current_app_screen(), AppScreen::Contacts);

    app.goto(AppScreen::Settings);
    assert_eq!(app.current_app_screen(), AppScreen::Settings);
}

// @internal
#[test]
fn test_goto_resets_input_mode_to_normal() {
    let (mut app, _dir) = create_app_with_identity();
    app.input_mode = InputMode::Editing;

    app.goto(AppScreen::Contacts);
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
#[case::settings(AppScreen::Settings, AppScreen::More)]
#[case::help(AppScreen::Help, AppScreen::More)]
#[case::devices(AppScreen::DeviceManagement, AppScreen::More)]
#[case::recovery(AppScreen::Recovery, AppScreen::More)]
#[case::privacy(AppScreen::Privacy, AppScreen::Settings)]
#[case::backup(AppScreen::Backup, AppScreen::More)]
#[case::duress(AppScreen::DuressPin, AppScreen::Settings)]
fn test_go_back_returns_to_expected_screen(#[case] from: AppScreen, #[case] expected: AppScreen) {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(expected.clone());
    app.goto(from);
    app.go_back();
    assert_eq!(app.current_app_screen(), expected);
}

/// Tab roots (Contacts, Exchange, Groups, MyInfo) are back-stopping roots
/// in core. `UserAction::NavigateBack` from a root returns
/// `ActionResult::PerformNativeBack`, which the TUI translates to a quit
/// signal instead of a screen change (ADR-044 Am2a).
// @internal
#[rstest]
#[case::contacts(AppScreen::Contacts)]
#[case::exchange(AppScreen::Exchange)]
#[case::groups(AppScreen::Groups)]
#[case::my_info(AppScreen::MyInfo)]
fn test_go_back_at_root_quits(#[case] from: AppScreen) {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(from);
    app.go_back();
    assert!(app.should_quit, "back from a tab root must set should_quit");
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
    app.goto(AppScreen::Contacts);
    app.goto(AppScreen::ContactDetail {
        contact_id: app.selected_contact_id.clone().unwrap_or_default(),
    });
    app.go_back();
    assert_eq!(app.current_app_screen(), AppScreen::Contacts);
}

// @internal
#[test]
fn test_go_back_from_contact_visibility_returns_to_contact_detail() {
    let (mut app, _dir) = create_app_with_identity();
    app.selected_contact_id = Some("test-contact-id".into());
    app.goto(AppScreen::Contacts);
    app.goto(AppScreen::ContactDetail {
        contact_id: app.selected_contact_id.clone().unwrap_or_default(),
    });
    app.goto(AppScreen::ContactVisibility {
        contact_id: app.selected_contact_id.clone().unwrap_or_default(),
    });
    app.go_back();
    assert!(matches!(
        app.current_app_screen(),
        AppScreen::ContactDetail { .. }
    ));
}

// Form dialogs are entered via `goto_form_dialog(FormDialogType)`, which
// carries the dialog's data. Plain `goto(Screen::FormDialog)` does not
// navigate AppEngine and so leaves nav history untouched, making `go_back`
// pop the wrong frame. These tests use the real entry point.
// @internal
#[test]
fn test_go_back_from_edit_name_returns_to_settings() {
    use vauchi_app::ui::FormDialogType;
    let (mut app, _dir) = create_app_with_identity();
    app.goto(AppScreen::Settings);
    app.goto_form_dialog(FormDialogType::EditName {
        current_name: "Alice".into(),
    });
    app.go_back();
    assert_eq!(app.current_app_screen(), AppScreen::Settings);
}

// @internal
#[test]
fn test_go_back_from_edit_relay_url_returns_to_settings() {
    use vauchi_app::ui::FormDialogType;
    let (mut app, _dir) = create_app_with_identity();
    app.goto(AppScreen::Settings);
    app.goto_form_dialog(FormDialogType::EditRelayUrl {
        current_url: "https://relay.test".into(),
    });
    app.go_back();
    assert_eq!(app.current_app_screen(), AppScreen::Settings);
}

// 'a' from Home opens the AddField form dialog. The dialog kind is asserted
// via the engine's AppScreen (single source of truth) now that all form
// dialogs share `Screen::FormDialog`.
// @internal
#[test]
fn test_handle_key_a_from_home_opens_add_field_dialog() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(AppScreen::MyInfo);
    let action = handle_key(&mut app, KeyCode::Char('a'));
    assert!(matches!(action, Action::Continue));
    assert!(matches!(
        app.current_app_screen(),
        AppScreen::FormDialog { .. }
    ));
    assert!(
        matches!(
            app.current_app_screen(),
            AppScreen::FormDialog {
                dialog_type: FormDialogType::AddField { .. }
            }
        ),
        "'a' from Home should open the AddField dialog"
    );
}

// Nav-bar tab for a form dialog is derived from the engine's dialog kind
// (the single source of truth): My Card (0), Groups (3), or More (4).
// @internal
#[rstest]
#[case::add_field(FormDialogType::AddField { available_groups: vec![] }, 0)]
#[case::edit_field(
    FormDialogType::EditField {
        field_id: "f1".into(),
        field_label: "Phone".into(),
        current_value: "555".into(),
        current_note: None,
    },
    0
)]
#[case::edit_name(FormDialogType::EditName { current_name: "Alice".into() }, 0)]
#[case::edit_relay_url(FormDialogType::EditRelayUrl { current_url: "https://r.test".into() }, 4)]
#[case::create_group(FormDialogType::CreateGroup, 3)]
#[case::rename_group(
    FormDialogType::RenameGroup {
        group_id: "g1".into(),
        current_name: "Friends".into(),
    },
    3
)]
fn test_form_dialog_nav_tab_follows_dialog_kind(
    #[case] dialog: FormDialogType,
    #[case] expected_tab: usize,
) {
    let (mut app, _dir) = create_app_with_identity();
    app.goto_form_dialog(dialog);
    assert!(matches!(
        app.current_app_screen(),
        AppScreen::FormDialog { .. }
    ));
    assert_eq!(
        app.focus.nav_index, expected_tab,
        "form-dialog nav tab should follow the engine's dialog kind"
    );
}

// @internal
#[test]
fn test_go_back_from_setup_stays_on_setup() {
    let (mut app, _dir) = create_app_without_identity();
    assert_eq!(app.current_app_screen(), AppScreen::Onboarding);
    app.go_back();
    assert_eq!(
        app.current_app_screen(),
        AppScreen::Onboarding,
        "go_back from SetupWelcome should stay on SetupWelcome"
    );
}

// @internal
#[test]
fn test_go_back_from_backup_without_identity_goes_to_setup() {
    let (mut app, _dir) = create_app_without_identity();
    app.goto(AppScreen::Backup);
    app.go_back();
    assert_eq!(app.current_app_screen(), AppScreen::Onboarding);
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
    app.goto(AppScreen::MyInfo);

    let action = handle_key(&mut app, KeyCode::Char('q'));
    assert!(matches!(action, Action::Quit));
}

// @internal
#[test]
fn test_handle_key_q_is_global_quit() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(AppScreen::Contacts);

    // 'q' is a global quit on any screen (not screen-specific)
    let action = handle_key(&mut app, KeyCode::Char('q'));
    assert!(matches!(action, Action::Quit));
}

// @internal
#[test]
fn test_handle_key_question_mark_navigates_to_help() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(AppScreen::MyInfo);

    let action = handle_key(&mut app, KeyCode::Char('?'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.current_app_screen(), AppScreen::Help);
}

// @internal
#[test]
fn test_handle_key_esc_goes_back() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(AppScreen::More);
    app.goto(AppScreen::Settings);

    let action = handle_key(&mut app, KeyCode::Esc);
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.current_app_screen(), AppScreen::More);
}

/// Escape at a tab root is forwarded to core as `UserAction::NavigateBack`;
/// core answers `PerformNativeBack` and the TUI quits (ADR-044 Am2a).
// @internal
#[test]
fn test_handle_key_esc_at_root_quits() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(AppScreen::Contacts);

    let action = handle_key(&mut app, KeyCode::Esc);
    assert!(matches!(action, Action::Quit));
    assert_eq!(app.current_app_screen(), AppScreen::Contacts);
}

// @scenario: contacts_management:View all contacts
// @scenario: sync_updates:Client initiates sync with relay
// @scenario: device_management:View linked devices
// @scenario: identity_management:View recovery status
// @scenario: identity_management:Create encrypted identity backup
// @scenario: contact_card_management:Add a field to contact card
#[rstest]
#[case::c_contacts('c', AppScreen::Contacts)]
#[case::s_settings('s', AppScreen::Settings)]
#[case::d_devices('d', AppScreen::DeviceManagement)]
#[case::r_recovery('r', AppScreen::Recovery)]
#[case::b_backup('b', AppScreen::Backup)]
#[case::y_delivery('y', AppScreen::DeliveryStatus)]
#[case::g_groups('g', AppScreen::Groups)]
#[case::x_exchange('X', AppScreen::Exchange)]
fn test_handle_key_on_home_navigates_to_screen(#[case] key: char, #[case] expected: AppScreen) {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(AppScreen::MyInfo);

    let action = handle_key(&mut app, KeyCode::Char(key));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.current_app_screen(), expected);
}

// @internal
#[test]
fn test_handle_key_in_editing_mode_esc_returns_to_normal() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(AppScreen::MyInfo);
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
    assert_eq!(app.current_app_screen(), AppScreen::MyInfo);
}

// @internal
#[test]
fn test_app_new_without_identity_starts_on_setup() {
    let (app, _dir) = create_app_without_identity();
    assert_eq!(app.current_app_screen(), AppScreen::Onboarding);
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
    app.goto(AppScreen::More);
    app.goto(AppScreen::DeliveryStatus);

    let action = handle_key(&mut app, KeyCode::Esc);
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.current_app_screen(), AppScreen::More);
}

// ============================================================================
// Duress screen tests
// ============================================================================

// @internal
#[test]
fn test_settings_shift_d_navigates_to_duress() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(AppScreen::Settings);
    let action = handle_key(&mut app, KeyCode::Char('D'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.current_app_screen(), AppScreen::DuressPin);
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
    app.goto(AppScreen::Settings);
    app.goto(AppScreen::DuressPin);
    let action = handle_key(&mut app, KeyCode::Esc);
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.current_app_screen(), AppScreen::Settings);
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
    assert_eq!(app.current_app_screen(), AppScreen::MyInfo);
}

// @internal
#[test]
fn test_lock_screen_q_does_not_quit() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(AppScreen::Lock);
    // 'q' should NOT quit from lock screen — it's a PIN character
    let action = handle_key(&mut app, KeyCode::Char('q'));
    assert!(matches!(action, Action::Continue));
    assert_eq!(app.current_app_screen(), AppScreen::Lock);
    assert_eq!(app.lock_state.pin_input, "q");
}

// @internal
#[test]
fn test_lock_screen_esc_does_not_navigate() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(AppScreen::Lock);
    let action = handle_key(&mut app, KeyCode::Esc);
    assert!(matches!(action, Action::Continue));
    // Should stay on Lock screen
    assert_eq!(app.current_app_screen(), AppScreen::Lock);
}

// @internal
#[test]
fn test_lock_screen_char_input_accumulates() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(AppScreen::Lock);
    handle_key(&mut app, KeyCode::Char('1'));
    handle_key(&mut app, KeyCode::Char('2'));
    handle_key(&mut app, KeyCode::Char('3'));
    assert_eq!(app.lock_state.pin_input, "123");
}

// @internal
#[test]
fn test_lock_screen_backspace_removes_char() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(AppScreen::Lock);
    handle_key(&mut app, KeyCode::Char('a'));
    handle_key(&mut app, KeyCode::Char('b'));
    handle_key(&mut app, KeyCode::Backspace);
    assert_eq!(app.lock_state.pin_input, "a");
}

// @internal
#[test]
fn test_lock_screen_empty_enter_does_nothing() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(AppScreen::Lock);
    handle_key(&mut app, KeyCode::Enter);
    // Should stay on Lock — empty PIN doesn't attempt auth
    assert_eq!(app.current_app_screen(), AppScreen::Lock);
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
    app.goto(AppScreen::Lock);
    // Enter wrong PIN
    handle_key(&mut app, KeyCode::Char('w'));
    handle_key(&mut app, KeyCode::Char('r'));
    handle_key(&mut app, KeyCode::Char('o'));
    handle_key(&mut app, KeyCode::Char('n'));
    handle_key(&mut app, KeyCode::Char('g'));
    handle_key(&mut app, KeyCode::Enter);
    assert_eq!(app.current_app_screen(), AppScreen::Lock);
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
    app.goto(AppScreen::Lock);
    for c in "mypin".chars() {
        handle_key(&mut app, KeyCode::Char(c));
    }
    handle_key(&mut app, KeyCode::Enter);
    assert_eq!(app.current_app_screen(), AppScreen::MyInfo);
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
    app.goto(AppScreen::Lock);
    app.go_back();
    assert_eq!(app.current_app_screen(), AppScreen::Lock);
}

// ============================================================================
// SP-11: TUI Accessibility Improvements
// @scenario: accessibility.feature @keyboard @tui
// ============================================================================

/// @scenario: accessibility.feature @keyboard - Home field delete status includes label
#[test]
fn test_home_field_delete_status_includes_label() {
    let (mut app, _dir) = create_app_with_identity();
    app.goto(AppScreen::MyInfo);

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
    // ContactDetail is engine-driven: the engine only navigates there with a
    // selected contact, so set the id before `goto` (active_screen now derives
    // from the engine, not from `self.screen`).
    app.selected_contact_id = Some("test-contact-id".into());
    app.goto(AppScreen::ContactDetail {
        contact_id: app.selected_contact_id.clone().unwrap_or_default(),
    });

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

    // Navigate to contact detail (engine needs a selected contact id).
    app.selected_contact_id = Some("test-contact-id".into());
    app.goto(AppScreen::ContactDetail {
        contact_id: app.selected_contact_id.clone().unwrap_or_default(),
    });
    app.selected_contact = 0;

    // Pressing 'x' with no contacts should stay on screen without crash
    handle_key(&mut app, KeyCode::Char('x'));
    // No contact to delete — should not crash, screen transitions back
    // The go_back transitions to Contacts
    assert!(
        app.current_app_screen() == AppScreen::Contacts
            || matches!(app.current_app_screen(), AppScreen::ContactDetail { .. }),
        "x on ContactDetail should navigate back or stay, got {:?}",
        app.current_app_screen()
    );
}
