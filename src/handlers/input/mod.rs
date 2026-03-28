// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Keyboard Input Handling

mod contacts_input;
mod editing;
mod features_input;
mod navigation;

use crossterm::event::KeyCode;
use vauchi_app::ui::{ActionResult, UserAction, WorkflowEngine};

use crate::app::{App, InputMode, Screen};
use crate::handlers::action_result::handle_action_result;
use crate::ui::focus::FocusZone;
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
};
use navigation::{
    handle_help_keys, handle_my_info_keys, handle_setup_add_fields_keys,
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
    // Alert modal captures all input — dismiss on Esc/Enter
    if app.alert_message.is_some() {
        if matches!(key, KeyCode::Esc | KeyCode::Enter) {
            app.alert_message = None;
        }
        return Action::Continue;
    }

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
        // Handle discard confirmation overlay
        if app.form_discard_confirm {
            match key {
                KeyCode::Char('y') | KeyCode::Enter => {
                    app.form_discard_confirm = false;
                    app.set_status("Changes discarded");
                    app.go_back();
                }
                KeyCode::Esc | KeyCode::Char('n') => {
                    app.form_discard_confirm = false;
                }
                _ => {
                    // Ignore other keys while dialog is open
                }
            }
            return Action::Continue;
        }

        if key == KeyCode::Esc {
            // For AddField: Esc with type selected → deselect type (via engine cancel)
            // For AddField: Esc without type → go back to parent
            // For other forms: go back
            if app.screen == Screen::AddField {
                // Send cancel action to engine — it handles back-step logic
                let result = app.app_engine.handle_action(UserAction::ActionPressed {
                    action_id: "cancel".into(),
                });
                match result {
                    ActionResult::UpdateScreen(_) => {
                        // Engine handled it internally (e.g., deselected type)
                        // Check if form has data after type deselection
                    }
                    _ => {
                        // Engine wants to navigate away
                        if app.app_engine.form_has_data() {
                            app.form_discard_confirm = true;
                        } else {
                            app.go_back();
                        }
                    }
                }
            } else if app.app_engine.form_has_data() {
                app.form_discard_confirm = true;
            } else {
                app.go_back();
            }
            return Action::Continue;
        }
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
        // Direct screen switching: number keys 1-5
        KeyCode::Char('1') => {
            app.focus.zone = FocusZone::Content;
            app.goto(Screen::MyInfo);
            return Action::Continue;
        }
        KeyCode::Char('2') => {
            app.focus.zone = FocusZone::Content;
            app.goto(Screen::Contacts);
            return Action::Continue;
        }
        KeyCode::Char('3') => {
            app.focus.zone = FocusZone::Content;
            app.goto(Screen::Exchange);
            return Action::Continue;
        }
        KeyCode::Char('4') => {
            app.focus.zone = FocusZone::Content;
            app.goto(Screen::Groups);
            return Action::Continue;
        }
        KeyCode::Char('5') => {
            app.focus.zone = FocusZone::Content;
            app.goto(Screen::More);
            return Action::Continue;
        }
        // Shift+Tab: move focus backwards (Content → NavBar → ActionBar → Content)
        KeyCode::BackTab => {
            app.focus.move_up();
            return Action::Continue;
        }
        KeyCode::Esc => {
            // If in a bar zone, return to content first
            if app.focus.zone != FocusZone::Content {
                app.focus.zone = FocusZone::Content;
                return Action::Continue;
            }
            app.go_back();
            return Action::Continue;
        }
        _ => {}
    }

    // Focus zone navigation — when in ActionBar or NavBar, handle arrows and Enter
    if app.focus.zone != FocusZone::Content {
        if let Some(action) = handle_bar_keys(app, key) {
            return action;
        }
        return Action::Continue;
    }

    // Engine-driven screens — route through AppEngine key mapping
    if matches!(
        app.screen,
        Screen::MyInfo
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
            | Screen::Recovery
            | Screen::Groups
            | Screen::GroupDetail
            | Screen::ContactEdit
            | Screen::ContactVisibility
            | Screen::Privacy
            | Screen::Support
            | Screen::EditName
            | Screen::EditField
            | Screen::EditRelayUrl
            | Screen::AddField
            | Screen::ContactDuplicates
            | Screen::ContactMerge
            | Screen::MyInfoEntryDetail
            | Screen::More
    ) {
        handle_engine_keys(app, key);
        return Action::Continue;
    }

    // Screen-specific keys
    match app.screen {
        Screen::MyInfo => handle_my_info_keys(app, key),
        Screen::Contacts => handle_contacts_keys(app, key),
        Screen::ContactDetail => handle_contact_detail_keys(app, key),
        Screen::ContactVisibility => handle_visibility_keys(app, key),
        Screen::Exchange => handle_exchange_keys(app, key),
        Screen::Settings => handle_settings_keys(app, key),
        Screen::Help => handle_help_keys(app, key),
        // Form dialogs: handled by engine guard above when app_engine is present
        Screen::AddField | Screen::EditField | Screen::EditName | Screen::EditRelayUrl => {
            eprintln!("WARNING: form dialog screen reached legacy handler unexpectedly");
        }
        Screen::Devices => handle_devices_keys(app, key),
        Screen::Recovery => handle_recovery_keys(app, key),
        Screen::Sync => handle_sync_keys(app, key),
        Screen::Delivery => handle_delivery_keys(app, key),
        Screen::Backup => handle_backup_keys(app, key),
        Screen::Privacy => handle_privacy_keys(app, key),
        Screen::Support => handle_support_keys(app, key),
        Screen::ActionMenu => handle_action_menu_keys(app, key),
        Screen::Emergency => handle_emergency_keys(app, key),
        Screen::Duress => handle_duress_keys(app, key),
        Screen::Groups => handle_groups_keys(app, key),
        Screen::GroupDetail => handle_group_detail_keys(app, key),
        Screen::Lock => {
            eprintln!("WARNING: Lock screen reached legacy handler unexpectedly");
        }
        // SP-21 Onboarding wizard
        Screen::SetupWelcome => handle_setup_welcome_keys(app, key),
        Screen::SetupCreateIdentity => {
            eprintln!("WARNING: SetupCreateIdentity reached legacy handler unexpectedly");
        }
        Screen::SetupAddFields => handle_setup_add_fields_keys(app, key),
        Screen::SetupSecurity => handle_setup_security_keys(app, key),
        Screen::SetupReady => handle_setup_ready_keys(app, key),
        // Engine-only screens: handled by engine guard above
        Screen::ContactEdit
        | Screen::ContactDuplicates
        | Screen::ContactMerge
        | Screen::ContactLimit
        | Screen::MyInfoEntryDetail
        | Screen::VerifyFingerprint
        | Screen::More => {
            eprintln!("WARNING: engine-only screen reached legacy handler unexpectedly");
        }
    }

    Action::Continue
}

/// Handle keys when focus is in ActionBar or NavBar zone.
///
/// Returns `Some(Action)` if the key was consumed, `None` if unhandled.
fn handle_bar_keys(app: &mut App, key: KeyCode) -> Option<Action> {
    match key {
        // Arrow navigation within and between bars
        KeyCode::Left | KeyCode::Char('h') => {
            app.focus.move_left();
            Some(Action::Continue)
        }
        KeyCode::Right | KeyCode::Char('l') => {
            app.focus.move_right();
            Some(Action::Continue)
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.focus.move_up();
            Some(Action::Continue)
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.focus.move_down();
            Some(Action::Continue)
        }
        // Enter activates the focused item
        KeyCode::Enter => {
            match app.focus.zone {
                FocusZone::NavBar => {
                    // Navigate to the tab at the focused index
                    let target = match app.focus.nav_index {
                        0 => Screen::MyInfo,
                        1 => Screen::Contacts,
                        2 => Screen::Exchange,
                        3 => Screen::Groups,
                        4 => Screen::More,
                        _ => return Some(Action::Continue),
                    };
                    // Cancel any pending state before navigating
                    app.contact_search_mode = false;
                    app.contact_search_query.clear();
                    app.input_mode = InputMode::Normal;
                    app.focus.zone = FocusZone::Content;
                    app.goto(target);
                    Some(Action::Continue)
                }
                FocusZone::ActionBar => {
                    // Action bar Enter triggers the action at the focused index.
                    // The action items are built from the ScreenModel in draw(),
                    // but we can re-derive the action key and dispatch it.
                    app.focus.zone = FocusZone::Content;
                    // For now, action bar activation is handled by the existing
                    // key-based shortcuts (users press the key shown in [key]).
                    Some(Action::Continue)
                }
                FocusZone::Content => None,
            }
        }
        _ => None,
    }
}

/// Handle keys for engine-driven screens via AppEngine key mapping.
///
/// Falls back to legacy screen handlers for keys not consumed by the engine
/// (e.g., TUI-specific navigation shortcuts like 's' for Settings).
fn handle_engine_keys(app: &mut App, key: KeyCode) {
    // Ensure AppEngine is synced to the current TUI screen
    if let Some(target) = app.to_app_screen()
        && *app.app_engine.current_app_screen() != target
    {
        app.app_engine.navigate_to(target);
    }

    // Get the current screen model from AppEngine
    let screen_model = app.app_engine.current_screen();

    // Map the key to a UserAction via the key_mapping module
    match key_mapping::map_key(key, &screen_model, &mut app.render_state) {
        KeyResult::Action(action) => {
            // On form dialog screens, "cancel" action means go back (don't forward to engine)
            if matches!(
                app.screen,
                Screen::AddField | Screen::EditName | Screen::EditField | Screen::EditRelayUrl
            ) && let UserAction::ActionPressed { ref action_id } = action
                && action_id == "cancel"
            {
                app.go_back();
                return;
            }
            // Forward the action to AppEngine
            let result = app.app_engine.handle_action(action);
            handle_action_result(app, result);
        }
        KeyResult::Consumed => {
            // Key was handled internally (focus change, etc.)
        }
        KeyResult::Unhandled => {
            // Tab at bottom of content → move focus to action bar
            if key == KeyCode::Tab {
                app.focus.move_down();
                return;
            }
            // Shift+Tab → move focus backwards
            if key == KeyCode::BackTab {
                app.focus.move_up();
                return;
            }
            // Down at bottom of content → move focus to action bar
            if matches!(key, KeyCode::Down | KeyCode::Char('j')) {
                app.focus.move_down();
                return;
            }
            // Left/Right in content → jump to nav bar for tab navigation
            if matches!(key, KeyCode::Left | KeyCode::Right) {
                app.focus.zone = FocusZone::NavBar;
                if key == KeyCode::Left {
                    // Move left from current nav position
                    if app.focus.nav_index > 0 {
                        app.focus.nav_index -= 1;
                    }
                } else {
                    // Move right from current nav position
                    if app.focus.nav_index < 4 {
                        app.focus.nav_index += 1;
                    }
                }
                return;
            }
            // Fall back to legacy handlers for TUI-specific shortcuts
            match app.screen {
                Screen::MyInfo => handle_my_info_keys(app, key),
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
                // ContactEdit + SP-12a + EntryDetail: Esc goes back
                Screen::ContactEdit
                | Screen::ContactDuplicates
                | Screen::ContactMerge
                | Screen::ContactLimit
                | Screen::MyInfoEntryDetail => {
                    if key == KeyCode::Esc {
                        app.go_back();
                    }
                }
                _ => {}
            }
        }
    }
}
