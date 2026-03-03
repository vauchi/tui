// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Keyboard Input Handling

mod contacts_input;
mod editing;
mod features_input;
mod navigation;

use crossterm::event::KeyCode;

use crate::app::{App, InputMode, Screen};

use contacts_input::{
    handle_action_menu_keys, handle_contact_detail_keys, handle_contacts_keys,
    handle_group_detail_keys, handle_groups_keys, handle_visibility_keys,
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
use navigation::{handle_help_keys, handle_home_keys, handle_setup_keys};

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
    }

    Action::Continue
}
