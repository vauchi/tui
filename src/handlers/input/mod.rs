// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Keyboard Input Handling

mod contacts_input;
mod editing;
mod features_input;
mod navigation;

use crossterm::event::KeyCode;
use vauchi_core::ui::WorkflowEngine;

use crate::app::{App, InputMode, Screen};
use crate::handlers::action_result::handle_action_result;
use crate::ui::widgets::key_mapping::{self, KeyResult};

use contacts_input::{
    handle_action_menu_keys, handle_contact_detail_keys, handle_contact_limit_keys,
    handle_contacts_keys, handle_duplicates_keys, handle_group_detail_keys, handle_groups_keys,
    handle_merge_keys, handle_visibility_keys,
};
use editing::{
    handle_add_field_keys, handle_edit_field_keys, handle_edit_name_keys,
    handle_edit_relay_url_keys, handle_editing_mode,
};
use features_input::{
    handle_backup_keys, handle_delivery_keys, handle_devices_keys, handle_duress_keys,
    handle_emergency_keys, handle_exchange_keys, handle_lock_keys, handle_privacy_keys,
    handle_recovery_keys, handle_settings_keys, handle_support_keys, handle_sync_keys,
    handle_tor_settings_keys,
};
use navigation::{
    handle_help_keys, handle_home_keys, handle_setup_add_fields_keys,
    handle_setup_create_identity_keys, handle_setup_keys, handle_setup_ready_keys,
    handle_setup_security_keys, handle_setup_welcome_keys,
};

/// Action to take after handling input.
pub enum Action {
    Continue,
    Quit,
}

/// Handle a key press.
pub fn handle_key(app: &mut App, key: KeyCode) -> Action {
    match app.input_mode {
        InputMode::Normal => handle_normal_mode(app, key),
        InputMode::Editing => handle_editing_mode(app, key),
    }
}

fn handle_normal_mode(app: &mut App, key: KeyCode) -> Action {
    // Lock screen bypasses ALL global keys — PIN chars include 'q', Esc, etc.
    if app.screen == Screen::Lock {
        handle_lock_keys(app, key);
        return Action::Continue;
    }

    // Engine-driven onboarding bypasses global keys (engine handles text input)
    if app.onboarding_engine.is_some()
        && matches!(
            app.screen,
            Screen::SetupWelcome
                | Screen::SetupCreateIdentity
                | Screen::SetupAddFields
                | Screen::SetupSecurity
                | Screen::SetupReady
        )
    {
        navigation::handle_onboarding_engine_keys(app, key);
        return Action::Continue;
    }

    // Legacy: Onboarding name input bypasses global keys
    if app.screen == Screen::SetupCreateIdentity {
        handle_setup_create_identity_keys(app, key);
        return Action::Continue;
    }

    // Contact limit editing bypasses global keys (digits are input)
    if app.screen == Screen::ContactLimit && app.contact_limit_state.editing {
        handle_contact_limit_keys(app, key);
        return Action::Continue;
    }

    // Don't process global keys if in contact search mode
    if app.contact_search_mode && app.screen == Screen::Contacts {
        handle_contacts_keys(app, key);
        return Action::Continue;
    }

    // Global keys
    match key {
        KeyCode::Char('q') => return Action::Quit,
        KeyCode::Char('?') => {
            app.goto(Screen::Help);
            return Action::Continue;
        }
        KeyCode::Esc => {
            app.go_back();
            return Action::Continue;
        }
        _ => {}
    }

    // Engine-driven screens — route through AppEngine key mapping
    if app.app_engine.is_some()
        && matches!(
            app.screen,
            Screen::Home | Screen::Contacts | Screen::Exchange | Screen::Settings | Screen::Help
        )
    {
        handle_engine_keys(app, key);
        return Action::Continue;
    }

    // Screen-specific keys
    match app.screen {
        Screen::Setup => handle_setup_keys(app, key),
        Screen::Home => handle_home_keys(app, key),
        Screen::Contacts => handle_contacts_keys(app, key),
        Screen::ContactDetail => handle_contact_detail_keys(app, key),
        Screen::ContactVisibility => handle_visibility_keys(app, key),
        Screen::Exchange => handle_exchange_keys(app, key),
        Screen::Settings => handle_settings_keys(app, key),
        Screen::Help => handle_help_keys(app, key),
        Screen::AddField => handle_add_field_keys(app, key),
        Screen::EditField => handle_edit_field_keys(app, key),
        Screen::EditName => handle_edit_name_keys(app, key),
        Screen::EditRelayUrl => handle_edit_relay_url_keys(app, key),
        Screen::Devices => handle_devices_keys(app, key),
        Screen::Recovery => handle_recovery_keys(app, key),
        Screen::Sync => handle_sync_keys(app, key),
        Screen::Delivery => handle_delivery_keys(app, key),
        Screen::Backup => handle_backup_keys(app, key),
        Screen::TorSettings => handle_tor_settings_keys(app, key),
        Screen::Privacy => handle_privacy_keys(app, key),
        Screen::Support => handle_support_keys(app, key),
        Screen::ActionMenu => handle_action_menu_keys(app, key),
        Screen::Emergency => handle_emergency_keys(app, key),
        Screen::Duress => handle_duress_keys(app, key),
        Screen::Groups => handle_groups_keys(app, key),
        Screen::GroupDetail => handle_group_detail_keys(app, key),
        Screen::Lock => unreachable!("Lock screen handled before global keys"),
        // SP-21 Onboarding wizard
        Screen::SetupWelcome => handle_setup_welcome_keys(app, key),
        Screen::SetupCreateIdentity => {
            unreachable!("SetupCreateIdentity handled before global keys")
        }
        Screen::SetupAddFields => handle_setup_add_fields_keys(app, key),
        Screen::SetupSecurity => handle_setup_security_keys(app, key),
        Screen::SetupReady => handle_setup_ready_keys(app, key),
        // SP-12a Duplicates / Merge / Limit
        Screen::ContactDuplicates => handle_duplicates_keys(app, key),
        Screen::ContactMerge => handle_merge_keys(app, key),
        Screen::ContactLimit => handle_contact_limit_keys(app, key),
    }

    Action::Continue
}

/// Handle keys for engine-driven screens via AppEngine key mapping.
///
/// Falls back to legacy screen handlers for keys not consumed by the engine
/// (e.g., TUI-specific navigation shortcuts like 's' for Settings).
fn handle_engine_keys(app: &mut App, key: KeyCode) {
    // Ensure AppEngine is synced to the current TUI screen
    if let Some(target) = App::to_app_screen(app.screen) {
        if let Some(engine) = &mut app.app_engine {
            if *engine.current_app_screen() != target {
                engine.navigate_to(target);
            }
        }
    }

    // Get the current screen model from AppEngine
    let screen_model = match &app.app_engine {
        Some(engine) => engine.current_screen(),
        None => return,
    };

    // Map the key to a UserAction via the key_mapping module
    match key_mapping::map_key(key, &screen_model, &mut app.render_state) {
        KeyResult::Action(action) => {
            // Forward the action to AppEngine
            if let Some(engine) = &mut app.app_engine {
                let result = engine.handle_action(action);
                handle_action_result(app, result);
            }
        }
        KeyResult::Consumed => {
            // Key was handled internally (focus change, etc.)
        }
        KeyResult::Unhandled => {
            // Fall back to legacy handlers for TUI-specific shortcuts
            match app.screen {
                Screen::Home => handle_home_keys(app, key),
                Screen::Contacts => handle_contacts_keys(app, key),
                Screen::Exchange => handle_exchange_keys(app, key),
                Screen::Settings => handle_settings_keys(app, key),
                Screen::Help => handle_help_keys(app, key),
                _ => {}
            }
        }
    }
}
