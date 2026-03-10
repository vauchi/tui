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
    handle_action_menu_keys, handle_contact_detail_keys, handle_contacts_keys,
    handle_group_detail_keys, handle_groups_keys, handle_visibility_keys,
};
use editing::handle_editing_mode;
use features_input::{
    handle_backup_keys, handle_delivery_keys, handle_devices_keys, handle_duress_keys,
    handle_emergency_keys, handle_exchange_keys, handle_lock_keys, handle_privacy_keys,
    handle_recovery_keys, handle_settings_keys, handle_support_keys, handle_sync_keys,
    handle_tor_settings_keys,
};
use navigation::{
    handle_help_keys, handle_home_keys, handle_setup_add_fields_keys,
    handle_setup_create_identity_keys, handle_setup_ready_keys, handle_setup_security_keys,
    handle_setup_welcome_keys,
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
        if let Some(action) = navigation::handle_onboarding_engine_keys(app, key) {
            return action;
        }
        return Action::Continue;
    }

    // Legacy: Onboarding name input bypasses global keys
    if app.screen == Screen::SetupCreateIdentity {
        handle_setup_create_identity_keys(app, key);
        return Action::Continue;
    }

    // Engine-driven form dialogs bypass global keys (text input uses 'q', etc.)
    if matches!(
        app.screen,
        Screen::EditName | Screen::EditField | Screen::EditRelayUrl | Screen::AddField
    ) {
        handle_engine_keys(app, key);
        return Action::Continue;
    }

    // Engine-driven contact limit bypasses global keys when editing (digits are input)
    if app.screen == Screen::ContactLimit {
        handle_engine_keys(app, key);
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
    if matches!(
        app.screen,
        Screen::Home
            | Screen::Contacts
            | Screen::ContactDetail
            | Screen::Exchange
            | Screen::Settings
            | Screen::Help
            | Screen::Backup
            | Screen::Delivery
            | Screen::Devices
            | Screen::Duress
            | Screen::Emergency
            | Screen::Sync
            | Screen::TorSettings
            | Screen::Recovery
            | Screen::Groups
            | Screen::GroupDetail
            | Screen::ContactVisibility
            | Screen::Privacy
            | Screen::Support
            | Screen::EditName
            | Screen::EditField
            | Screen::EditRelayUrl
            | Screen::AddField
            | Screen::ContactDuplicates
            | Screen::ContactMerge
    ) {
        handle_engine_keys(app, key);
        return Action::Continue;
    }

    // Screen-specific keys
    match app.screen {
        Screen::Home => handle_home_keys(app, key),
        Screen::Contacts => handle_contacts_keys(app, key),
        Screen::ContactDetail => handle_contact_detail_keys(app, key),
        Screen::ContactVisibility => handle_visibility_keys(app, key),
        Screen::Exchange => handle_exchange_keys(app, key),
        Screen::Settings => handle_settings_keys(app, key),
        Screen::Help => handle_help_keys(app, key),
        // Form dialogs: handled by engine guard above when app_engine is present
        Screen::AddField | Screen::EditField | Screen::EditName | Screen::EditRelayUrl => {
            unreachable!("Form dialogs handled by engine guard")
        }
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
        // SP-12a screens: handled by engine guard above
        Screen::ContactDuplicates | Screen::ContactMerge | Screen::ContactLimit => {
            unreachable!("SP-12a screens handled by engine guard")
        }
    }

    Action::Continue
}

/// Handle keys for engine-driven screens via AppEngine key mapping.
///
/// Falls back to legacy screen handlers for keys not consumed by the engine
/// (e.g., TUI-specific navigation shortcuts like 's' for Settings).
fn handle_engine_keys(app: &mut App, key: KeyCode) {
    // Ensure AppEngine is synced to the current TUI screen
    if let Some(target) = app.to_app_screen() {
        if *app.app_engine.current_app_screen() != target {
            app.app_engine.navigate_to(target);
        }
    }

    // Get the current screen model from AppEngine
    let screen_model = app.app_engine.current_screen();

    // Map the key to a UserAction via the key_mapping module
    match key_mapping::map_key(key, &screen_model, &mut app.render_state) {
        KeyResult::Action(action) => {
            // Forward the action to AppEngine
            let result = app.app_engine.handle_action(action);
            handle_action_result(app, result);
        }
        KeyResult::Consumed => {
            // Key was handled internally (focus change, etc.)
        }
        KeyResult::Unhandled => {
            // Fall back to legacy handlers for TUI-specific shortcuts
            match app.screen {
                Screen::Home => handle_home_keys(app, key),
                Screen::Contacts => handle_contacts_keys(app, key),
                Screen::ContactDetail => handle_contact_detail_keys(app, key),
                Screen::Exchange => handle_exchange_keys(app, key),
                Screen::Settings => handle_settings_keys(app, key),
                Screen::Help => handle_help_keys(app, key),
                Screen::Backup => handle_backup_keys(app, key),
                Screen::Delivery => handle_delivery_keys(app, key),
                Screen::Devices => handle_devices_keys(app, key),
                Screen::Duress => handle_duress_keys(app, key),
                Screen::Emergency => handle_emergency_keys(app, key),
                Screen::Sync => handle_sync_keys(app, key),
                Screen::TorSettings => handle_tor_settings_keys(app, key),
                Screen::Recovery => handle_recovery_keys(app, key),
                Screen::Groups => handle_groups_keys(app, key),
                Screen::GroupDetail => handle_group_detail_keys(app, key),
                Screen::ContactVisibility => handle_visibility_keys(app, key),
                Screen::Privacy => handle_privacy_keys(app, key),
                Screen::Support => handle_support_keys(app, key),
                // Form dialogs: Esc goes back (engine handles chars/Enter via key_mapping)
                Screen::EditName | Screen::EditField | Screen::EditRelayUrl | Screen::AddField => {
                    if key == KeyCode::Esc {
                        app.go_back();
                    }
                }
                // SP-12a screens: Esc goes back, engine handles other actions
                Screen::ContactDuplicates | Screen::ContactMerge | Screen::ContactLimit => {
                    if key == KeyCode::Esc {
                        app.go_back();
                    }
                }
                _ => {}
            }
        }
    }
}
