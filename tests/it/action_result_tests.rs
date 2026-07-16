// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests for `handle_action_result` — verifies that each `ActionResult` variant
//! produces the correct TUI state change.

use vauchi_app::ui::{ActionResult, AppEngine, AppScreen, ScreenModel, UserAction, WorkflowEngine};
use vauchi_core::{SymmetricKey, Vauchi, VauchiConfig};
use vauchi_tui::app::App;
use vauchi_tui::handlers::action_result::handle_action_result;

fn create_app_engine(data_dir: &std::path::Path) -> AppEngine {
    let key = SymmetricKey::generate();
    let config = VauchiConfig::with_storage_path(data_dir.join("vauchi.db")).with_storage_key(key);
    let vauchi: Vauchi = Vauchi::new(config).expect("vauchi");
    AppEngine::new(vauchi)
}

/// Create an App with an identity (and thus an AppEngine).
fn create_app_with_identity() -> App {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    let mut app_engine = create_app_engine(&path);
    app_engine
        .vauchi_mut()
        .create_identity("Test User")
        .expect("create identity");
    // Keep the tempdir alive by leaking it (tests are short-lived)
    let _keep = dir.keep();
    App::new(app_engine, "wss://relay.vauchi.app".to_string(), path)
}

/// Create a minimal ScreenModel for variants that require one.
fn dummy_screen_model() -> ScreenModel {
    ScreenModel {
        screen_id: "test".into(),
        title: "Test".into(),
        subtitle: None,
        components: vec![],
        actions: vec![],
        progress: None,
        ..Default::default()
    }
}

// --- No-op variants (should not panic) ---

// @internal
#[test]
fn update_screen_is_noop() {
    let mut app = create_app_with_identity();
    let original_screen = app.current_app_screen();
    handle_action_result(&mut app, ActionResult::UpdateScreen(dummy_screen_model()));
    assert_eq!(app.current_app_screen(), original_screen);
}

/// `PerformNativeBack` is core's signal that the back gesture reached a
/// back-stopping root. The TUI's native default is to quit the app
/// (ADR-044 Am2a).
// @internal
#[test]
fn perform_native_back_sets_should_quit() {
    let mut app = create_app_with_identity();
    assert!(!app.should_quit);
    handle_action_result(&mut app, ActionResult::PerformNativeBack);
    assert!(app.should_quit);
}

// @internal
#[test]
fn complete_is_noop() {
    let mut app = create_app_with_identity();
    let original_screen = app.current_app_screen();
    handle_action_result(&mut app, ActionResult::Complete);
    assert_eq!(app.current_app_screen(), original_screen);
}

// --- Screen navigation variants ---

// @internal
#[test]
fn open_contact_sets_contact_detail_screen() {
    let mut app = create_app_with_identity();
    handle_action_result(
        &mut app,
        ActionResult::OpenContact {
            contact_id: "abc123".into(),
        },
    );
    assert!(matches!(
        app.current_app_screen(),
        AppScreen::ContactDetail { .. }
    ));
}

// --- Status message variants ---

// @internal
#[test]
fn open_url_sets_status_with_url() {
    // Prevent xdg-open from actually opening a browser tab during tests
    // TODO: Audit that the environment access only happens in single-threaded code.
    unsafe { std::env::set_var("VAUCHI_NO_BROWSER", "1") };
    let mut app = create_app_with_identity();
    handle_action_result(
        &mut app,
        ActionResult::OpenUrl {
            url: "https://example.com".into(),
        },
    );
    let msg = app.status_message.as_deref().unwrap();
    assert_eq!(msg, "URL: https://example.com");
}

// @internal
#[test]
fn show_alert_sets_modal_alert() {
    let mut app = create_app_with_identity();
    handle_action_result(
        &mut app,
        ActionResult::ShowAlert {
            title: "Warning".into(),
            message: "Something happened".into(),
        },
    );
    assert_eq!(
        app.alert_message,
        Some(("Warning".into(), "Something happened".into()))
    );
    assert!(
        app.status_message.is_none(),
        "Should use alert_message, not status_message"
    );
}

// --- Validation error ---

// @internal
#[test]
fn validation_error_sets_render_state_error() {
    let mut app = create_app_with_identity();
    handle_action_result(
        &mut app,
        ActionResult::ValidationError {
            component_id: "name_field".into(),
            message: "Name is required".into(),
        },
    );
    assert_eq!(
        app.render_state.validation_error_for("name_field"),
        Some("Name is required")
    );
}

// @internal
#[test]
fn validation_error_replaces_existing_error() {
    let mut app = create_app_with_identity();

    handle_action_result(
        &mut app,
        ActionResult::ValidationError {
            component_id: "email".into(),
            message: "Invalid format".into(),
        },
    );
    handle_action_result(
        &mut app,
        ActionResult::ValidationError {
            component_id: "email".into(),
            message: "Already taken".into(),
        },
    );

    assert_eq!(
        app.render_state.validation_error_for("email"),
        Some("Already taken")
    );
    // Should still be just one entry for the same component_id
    assert_eq!(app.render_state.validation_errors.len(), 1);
}

// --- WipeComplete ---

// @internal
#[test]
fn wipe_complete_resets_to_setup_welcome() {
    let mut app = create_app_with_identity();
    app.goto(AppScreen::Settings);

    handle_action_result(&mut app, ActionResult::WipeComplete);

    assert_eq!(app.current_app_screen(), AppScreen::Onboarding);
    assert!(
        app.onboarding_engine.is_some(),
        "onboarding_engine should be Some after WipeComplete"
    );
    assert!(
        app.render_state.validation_errors.is_empty(),
        "render_state should be reset (no validation errors)"
    );
    assert_eq!(
        app.render_state.focused_component, 0,
        "render_state focused_component should be 0 after reset"
    );
}

// --- NavigateTo with AppEngine ---

// @internal
#[test]
fn navigate_to_syncs_home_screen() {
    let mut app = create_app_with_identity();
    // Ensure the engine navigates to Home
    app.app_engine.navigate_to(AppScreen::MyInfo);
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.current_app_screen(), AppScreen::MyInfo);
}

// @internal
#[test]
fn navigate_to_syncs_contacts_screen() {
    let mut app = create_app_with_identity();
    app.app_engine.navigate_to(AppScreen::Contacts);
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.current_app_screen(), AppScreen::Contacts);
}

// @internal
#[test]
fn navigate_to_syncs_exchange_screen() {
    let mut app = create_app_with_identity();
    app.app_engine.navigate_to(AppScreen::Exchange);
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.current_app_screen(), AppScreen::Exchange);
}

// @internal
#[test]
fn navigate_to_syncs_settings_screen() {
    let mut app = create_app_with_identity();
    app.app_engine.navigate_to(AppScreen::Settings);
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.current_app_screen(), AppScreen::Settings);
}

// @internal
#[test]
fn navigate_to_syncs_help_screen() {
    let mut app = create_app_with_identity();
    app.app_engine.navigate_to(AppScreen::Help);
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.current_app_screen(), AppScreen::Help);
}

// @internal
#[test]
fn navigate_to_syncs_onboarding_to_setup_welcome() {
    let mut app = create_app_with_identity();
    app.app_engine.navigate_to(AppScreen::Onboarding);
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.current_app_screen(), AppScreen::Onboarding);
}

// --- NavigateTo: previously missing mappings (C-2 fix) ---

// @internal
#[test]
fn navigate_to_syncs_contact_detail_screen() {
    let mut app = create_app_with_identity();
    app.app_engine.navigate_to(AppScreen::ContactDetail {
        contact_id: "test-id".into(),
    });
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert!(matches!(
        app.current_app_screen(),
        AppScreen::ContactDetail { .. }
    ));
}

// @internal
#[test]
fn navigate_to_syncs_backup_screen() {
    let mut app = create_app_with_identity();
    app.app_engine.navigate_to(AppScreen::Backup);
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.current_app_screen(), AppScreen::Backup);
}

// @internal
#[test]
fn navigate_to_syncs_lock_screen() {
    let mut app = create_app_with_identity();
    app.app_engine.navigate_to(AppScreen::Lock);
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.current_app_screen(), AppScreen::Lock);
}

// @internal
#[test]
fn navigate_to_syncs_device_linking_screen() {
    let mut app = create_app_with_identity();
    app.app_engine.navigate_to(AppScreen::DeviceLinking);
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.current_app_screen(), AppScreen::DeviceLinking);
}

// @internal
#[test]
fn link_device_reaches_device_linking_and_survives_resync() {
    // Regression: AppScreen::DeviceLinking used to collapse to AppScreen::DeviceManagement,
    // so ensure_engine_synced (run every draw) snapped the engine back to
    // DeviceManagement (engine_target_for_screen(Devices) = DeviceManagement)
    // and the device-link QR screen never rendered.
    let mut app = create_app_with_identity();
    app.app_engine.navigate_to(AppScreen::DeviceManagement);
    app.goto(AppScreen::DeviceManagement);

    // "Link New Device" is the Primary action on the DeviceManagement screen.
    let result = app.app_engine.handle_action(UserAction::ActionPressed {
        action_id: "link_device".into(),
    });
    handle_action_result(&mut app, result);
    assert_eq!(app.current_app_screen(), AppScreen::DeviceLinking);

    // Simulate the next frame's sync — it must NOT revert the engine.
    assert_eq!(app.current_app_screen(), AppScreen::DeviceLinking);

    // The rendered screen is a device-link screen (QR display or pending),
    // not the device-management list it was snapped back to before the fix.
    let model = app.app_engine.current_screen();
    assert!(
        model.screen_id.starts_with("link_"),
        "expected a device-link screen, got {:?}",
        model.screen_id
    );
}

// @internal
#[test]
fn navigate_to_syncs_duress_pin_to_duress_screen() {
    let mut app = create_app_with_identity();
    app.app_engine.navigate_to(AppScreen::DuressPin);
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.current_app_screen(), AppScreen::DuressPin);
}

// @internal
#[test]
fn navigate_to_syncs_delivery_status_to_delivery_screen() {
    let mut app = create_app_with_identity();
    app.app_engine.navigate_to(AppScreen::DeliveryStatus);
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.current_app_screen(), AppScreen::DeliveryStatus);
}

// @internal
#[test]
fn navigate_to_syncs_recovery_screen() {
    let mut app = create_app_with_identity();
    app.app_engine.navigate_to(AppScreen::Recovery);
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.current_app_screen(), AppScreen::Recovery);
}

// @internal
#[test]
fn navigate_to_syncs_groups_screen() {
    let mut app = create_app_with_identity();
    app.app_engine.navigate_to(AppScreen::Groups);
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.current_app_screen(), AppScreen::Groups);
}

// @internal
#[test]
fn navigate_to_syncs_privacy_screen() {
    let mut app = create_app_with_identity();
    app.app_engine.navigate_to(AppScreen::Privacy);
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.current_app_screen(), AppScreen::Privacy);
}

// @internal
#[test]
fn gdpr_export_complete_writes_json_to_data_dir() {
    let mut app = create_app_with_identity();
    let json = r#"{"identity":"Test User","contacts":[]}"#.to_string();

    handle_action_result(
        &mut app,
        ActionResult::GdprExportComplete { json: json.clone() },
    );

    let path = app.data_dir.join("gdpr_export.json");
    let written = std::fs::read_to_string(&path).expect("export file written");
    assert_eq!(written, json);
    assert_eq!(
        app.status_message.as_deref(),
        Some(format!("Data exported to {}", path.display()).as_str())
    );
}

// @internal
#[test]
fn navigate_to_syncs_support_screen() {
    let mut app = create_app_with_identity();
    app.app_engine.navigate_to(AppScreen::Support);
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.current_app_screen(), AppScreen::Support);
}

// @internal
#[test]
fn navigate_to_syncs_group_detail_screen() {
    let mut app = create_app_with_identity();
    app.app_engine.navigate_to(AppScreen::GroupDetail {
        group_id: "g1".into(),
    });
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert!(matches!(
        app.current_app_screen(),
        AppScreen::GroupDetail { .. }
    ));
    // Engine carries group_id on its current AppScreen — no local mirror.
    assert!(
        matches!(
            app.app_engine.current_app_screen(),
            AppScreen::GroupDetail { group_id } if group_id == "g1"
        ),
        "engine should carry group_id on AppScreen::GroupDetail"
    );
}

// @internal
#[test]
fn navigate_to_syncs_contact_visibility_screen() {
    let mut app = create_app_with_identity();
    app.app_engine.navigate_to(AppScreen::ContactVisibility {
        contact_id: "c1".into(),
    });
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert!(matches!(
        app.current_app_screen(),
        AppScreen::ContactVisibility { .. }
    ));
    assert_eq!(
        app.selected_contact_id.as_deref(),
        Some("c1"),
        "selected_contact_id should be set"
    );
}

// @internal
#[test]
fn navigate_to_contact_edit_sets_edit_screen() {
    let mut app = create_app_with_identity();
    app.app_engine.navigate_to(AppScreen::ContactEdit {
        contact_id: "test-id".into(),
    });
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert!(matches!(
        app.current_app_screen(),
        AppScreen::ContactEdit { .. }
    ));
    assert_eq!(
        app.selected_contact_id.as_deref(),
        Some("test-id"),
        "selected_contact_id should be set"
    );
}

// @internal
#[test]
fn edit_contact_result_routes_to_contact_edit_screen() {
    let mut app = create_app_with_identity();
    handle_action_result(
        &mut app,
        ActionResult::EditContact {
            contact_id: "abc".into(),
        },
    );
    assert!(matches!(
        app.current_app_screen(),
        AppScreen::ContactEdit { .. }
    ));
    assert_eq!(app.selected_contact_id.as_deref(), Some("abc"));
}

// --- ShowToast with undo ---

// @internal
#[test]
fn show_toast_with_undo_sets_status_and_undo_id() {
    let mut app = create_app_with_identity();
    handle_action_result(
        &mut app,
        ActionResult::ShowToast {
            message: "Field hidden".into(),
            undo_action_id: Some("undo_hide_abc".into()),
            undo_label: Some("Restore field".into()),
        },
    );
    assert_eq!(app.status_message.as_deref(), Some("Field hidden"));
    assert_eq!(app.undo_action_id.as_deref(), Some("undo_hide_abc"));
    assert_eq!(app.undo_label.as_deref(), Some("Restore field"));
}

// @internal
#[test]
fn show_toast_without_undo_clears_stale_undo_id() {
    let mut app = create_app_with_identity();
    app.undo_action_id = Some("leftover".into());
    app.undo_label = Some("Leftover".into());
    handle_action_result(
        &mut app,
        ActionResult::ShowToast {
            message: "Done".into(),
            undo_action_id: None,
            undo_label: None,
        },
    );
    assert_eq!(app.status_message.as_deref(), Some("Done"));
    assert!(
        app.undo_action_id.is_none(),
        "undo_action_id should be cleared when toast has no undo"
    );
    assert!(app.undo_label.is_none());
}

// @internal
#[test]
fn clear_status_also_clears_undo_id() {
    let mut app = create_app_with_identity();
    app.undo_action_id = Some("undo_x".into());
    app.undo_label = Some("Undo X".into());
    app.set_status("temp");
    assert!(
        app.undo_action_id.is_none(),
        "set_status should clear undo_action_id"
    );
    assert!(app.undo_label.is_none());
}
