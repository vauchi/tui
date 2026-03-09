// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests for `handle_action_result` — verifies that each `ActionResult` variant
//! produces the correct TUI state change.

use vauchi_core::ui::{ActionResult, AppScreen, ScreenModel};
use vauchi_tui::app::{App, Screen};
use vauchi_tui::backend::Backend;
use vauchi_tui::handlers::action_result::handle_action_result;

/// Create an App with an identity (and thus an AppEngine).
fn create_app_with_identity() -> App {
    let dir = tempfile::tempdir().unwrap();
    let mut backend = Backend::new(dir.path()).unwrap();
    backend.create_identity("Test User").unwrap();
    let path = dir.keep();
    let backend = Backend::new(&path).unwrap();
    App::new(backend)
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
    }
}

// --- No-op variants (should not panic) ---

#[test]
fn update_screen_is_noop() {
    let mut app = create_app_with_identity();
    let original_screen = app.screen;
    handle_action_result(&mut app, ActionResult::UpdateScreen(dummy_screen_model()));
    assert_eq!(app.screen, original_screen);
}

#[test]
fn complete_is_noop() {
    let mut app = create_app_with_identity();
    let original_screen = app.screen;
    handle_action_result(&mut app, ActionResult::Complete);
    assert_eq!(app.screen, original_screen);
}

// --- Screen navigation variants ---

#[test]
fn open_contact_sets_contact_detail_screen() {
    let mut app = create_app_with_identity();
    handle_action_result(
        &mut app,
        ActionResult::OpenContact {
            contact_id: "abc123".into(),
        },
    );
    assert_eq!(app.screen, Screen::ContactDetail);
}

#[test]
fn start_device_link_sets_devices_screen() {
    let mut app = create_app_with_identity();
    handle_action_result(&mut app, ActionResult::StartDeviceLink);
    assert_eq!(app.screen, Screen::Devices);
}

#[test]
fn start_backup_import_sets_backup_screen() {
    let mut app = create_app_with_identity();
    handle_action_result(&mut app, ActionResult::StartBackupImport);
    assert_eq!(app.screen, Screen::Backup);
}

// --- Status message variants ---

#[test]
fn open_url_sets_status_with_url() {
    let mut app = create_app_with_identity();
    handle_action_result(
        &mut app,
        ActionResult::OpenUrl {
            url: "https://example.com".into(),
        },
    );
    assert_eq!(
        app.status_message.as_deref(),
        Some("URL: https://example.com")
    );
}

#[test]
fn show_alert_sets_status_with_message() {
    let mut app = create_app_with_identity();
    handle_action_result(
        &mut app,
        ActionResult::ShowAlert {
            title: "Warning".into(),
            message: "Something happened".into(),
        },
    );
    assert_eq!(app.status_message.as_deref(), Some("Something happened"));
}

#[test]
fn request_camera_sets_not_supported_status() {
    let mut app = create_app_with_identity();
    handle_action_result(&mut app, ActionResult::RequestCamera);
    assert_eq!(
        app.status_message.as_deref(),
        Some("Camera not supported in terminal mode")
    );
}

// --- Validation error ---

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

#[test]
fn wipe_complete_resets_to_setup_welcome() {
    let mut app = create_app_with_identity();
    app.screen = Screen::Settings;

    handle_action_result(&mut app, ActionResult::WipeComplete);

    assert_eq!(app.screen, Screen::SetupWelcome);
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

#[test]
fn navigate_to_syncs_home_screen() {
    let mut app = create_app_with_identity();
    // Ensure the engine navigates to Home
    {
        let engine = app
            .app_engine
            .as_mut()
            .expect("app_engine must be initialized");
        engine.navigate_to(AppScreen::Home);
    }
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.screen, Screen::Home);
}

#[test]
fn navigate_to_syncs_contacts_screen() {
    let mut app = create_app_with_identity();
    {
        let engine = app
            .app_engine
            .as_mut()
            .expect("app_engine must be initialized");
        engine.navigate_to(AppScreen::Contacts);
    }
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.screen, Screen::Contacts);
}

#[test]
fn navigate_to_syncs_exchange_screen() {
    let mut app = create_app_with_identity();
    {
        let engine = app
            .app_engine
            .as_mut()
            .expect("app_engine must be initialized");
        engine.navigate_to(AppScreen::Exchange);
    }
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.screen, Screen::Exchange);
}

#[test]
fn navigate_to_syncs_settings_screen() {
    let mut app = create_app_with_identity();
    {
        let engine = app
            .app_engine
            .as_mut()
            .expect("app_engine must be initialized");
        engine.navigate_to(AppScreen::Settings);
    }
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.screen, Screen::Settings);
}

#[test]
fn navigate_to_syncs_help_screen() {
    let mut app = create_app_with_identity();
    {
        let engine = app
            .app_engine
            .as_mut()
            .expect("app_engine must be initialized");
        engine.navigate_to(AppScreen::Help);
    }
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.screen, Screen::Help);
}

#[test]
fn navigate_to_syncs_onboarding_to_setup_welcome() {
    let mut app = create_app_with_identity();
    {
        let engine = app
            .app_engine
            .as_mut()
            .expect("app_engine must be initialized");
        engine.navigate_to(AppScreen::Onboarding);
    }
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.screen, Screen::SetupWelcome);
}

#[test]
fn navigate_to_without_engine_is_noop() {
    let mut app = create_app_with_identity();
    app.app_engine = None;
    let original_screen = app.screen;
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.screen, original_screen);
}

// --- NavigateTo: previously missing mappings (C-2 fix) ---

#[test]
fn navigate_to_syncs_contact_detail_screen() {
    let mut app = create_app_with_identity();
    {
        let engine = app
            .app_engine
            .as_mut()
            .expect("app_engine must be initialized");
        engine.navigate_to(AppScreen::ContactDetail {
            contact_id: "test-id".into(),
        });
    }
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.screen, Screen::ContactDetail);
}

#[test]
fn navigate_to_syncs_backup_screen() {
    let mut app = create_app_with_identity();
    {
        let engine = app
            .app_engine
            .as_mut()
            .expect("app_engine must be initialized");
        engine.navigate_to(AppScreen::Backup);
    }
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.screen, Screen::Backup);
}

#[test]
fn navigate_to_syncs_lock_screen() {
    let mut app = create_app_with_identity();
    {
        let engine = app
            .app_engine
            .as_mut()
            .expect("app_engine must be initialized");
        engine.navigate_to(AppScreen::Lock);
    }
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.screen, Screen::Lock);
}

#[test]
fn navigate_to_syncs_device_linking_to_devices_screen() {
    let mut app = create_app_with_identity();
    {
        let engine = app
            .app_engine
            .as_mut()
            .expect("app_engine must be initialized");
        engine.navigate_to(AppScreen::DeviceLinking);
    }
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.screen, Screen::Devices);
}

#[test]
fn navigate_to_syncs_duress_pin_to_duress_screen() {
    let mut app = create_app_with_identity();
    {
        let engine = app
            .app_engine
            .as_mut()
            .expect("app_engine must be initialized");
        engine.navigate_to(AppScreen::DuressPin);
    }
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.screen, Screen::Duress);
}

#[test]
fn navigate_to_syncs_emergency_shred_to_emergency_screen() {
    let mut app = create_app_with_identity();
    {
        let engine = app
            .app_engine
            .as_mut()
            .expect("app_engine must be initialized");
        engine.navigate_to(AppScreen::EmergencyShred);
    }
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.screen, Screen::Emergency);
}

#[test]
fn navigate_to_syncs_delivery_status_to_delivery_screen() {
    let mut app = create_app_with_identity();
    {
        let engine = app
            .app_engine
            .as_mut()
            .expect("app_engine must be initialized");
        engine.navigate_to(AppScreen::DeliveryStatus);
    }
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.screen, Screen::Delivery);
}

// --- NavigateTo: Wave 6 Phase A screens ---

#[test]
fn navigate_to_syncs_sync_screen() {
    let mut app = create_app_with_identity();
    {
        let engine = app
            .app_engine
            .as_mut()
            .expect("app_engine must be initialized");
        engine.navigate_to(AppScreen::Sync);
    }
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.screen, Screen::Sync);
}

#[test]
fn navigate_to_syncs_tor_settings_screen() {
    let mut app = create_app_with_identity();
    {
        let engine = app
            .app_engine
            .as_mut()
            .expect("app_engine must be initialized");
        engine.navigate_to(AppScreen::TorSettings);
    }
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.screen, Screen::TorSettings);
}

#[test]
fn navigate_to_syncs_recovery_screen() {
    let mut app = create_app_with_identity();
    {
        let engine = app
            .app_engine
            .as_mut()
            .expect("app_engine must be initialized");
        engine.navigate_to(AppScreen::Recovery);
    }
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.screen, Screen::Recovery);
}

#[test]
fn navigate_to_syncs_groups_screen() {
    let mut app = create_app_with_identity();
    {
        let engine = app
            .app_engine
            .as_mut()
            .expect("app_engine must be initialized");
        engine.navigate_to(AppScreen::Groups);
    }
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.screen, Screen::Groups);
}

#[test]
fn navigate_to_syncs_privacy_screen() {
    let mut app = create_app_with_identity();
    {
        let engine = app
            .app_engine
            .as_mut()
            .expect("app_engine must be initialized");
        engine.navigate_to(AppScreen::Privacy);
    }
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.screen, Screen::Privacy);
}

#[test]
fn navigate_to_syncs_support_screen() {
    let mut app = create_app_with_identity();
    {
        let engine = app
            .app_engine
            .as_mut()
            .expect("app_engine must be initialized");
        engine.navigate_to(AppScreen::Support);
    }
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.screen, Screen::Support);
}

#[test]
fn navigate_to_syncs_group_detail_screen() {
    let mut app = create_app_with_identity();
    {
        let engine = app
            .app_engine
            .as_mut()
            .expect("app_engine must be initialized");
        engine.navigate_to(AppScreen::GroupDetail {
            group_id: "g1".into(),
        });
    }
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.screen, Screen::GroupDetail);
    assert_eq!(
        app.groups_state.selected_group_id.as_deref(),
        Some("g1"),
        "selected_group_id should be set"
    );
}

#[test]
fn navigate_to_syncs_contact_visibility_screen() {
    let mut app = create_app_with_identity();
    {
        let engine = app
            .app_engine
            .as_mut()
            .expect("app_engine must be initialized");
        engine.navigate_to(AppScreen::ContactVisibility {
            contact_id: "c1".into(),
        });
    }
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.screen, Screen::ContactVisibility);
    assert_eq!(
        app.selected_contact_id.as_deref(),
        Some("c1"),
        "selected_contact_id should be set"
    );
}

#[test]
fn navigate_to_contact_edit_is_noop() {
    // ContactEdit has no dedicated TUI Screen — should not change screen
    let mut app = create_app_with_identity();
    let original_screen = app.screen;
    {
        let engine = app
            .app_engine
            .as_mut()
            .expect("app_engine must be initialized");
        engine.navigate_to(AppScreen::ContactEdit {
            contact_id: "test-id".into(),
        });
    }
    handle_action_result(&mut app, ActionResult::NavigateTo(dummy_screen_model()));
    assert_eq!(app.screen, original_screen);
}
