// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Keyboard Input Handling

mod contacts_input;
mod editing;
mod features_input;
mod navigation;

use crossterm::event::KeyCode;
use vauchi_app::ui::{AppScreen, Component, UserAction, WorkflowEngine};

use crate::app::{App, InputMode, Screen};
use crate::handlers::action_result::{handle_action_result, handle_action_result_with};
use crate::ui::focus::FocusZone;
use crate::ui::widgets::key_mapping::{self, KeyResult};

use contacts_input::{
    handle_action_menu_keys, handle_contact_detail_keys, handle_contacts_keys, handle_groups_keys,
    handle_visibility_keys,
};
use editing::handle_editing_mode;
use features_input::{
    handle_delivery_keys, handle_devices_keys, handle_exchange_keys, handle_lock_keys,
    handle_privacy_keys, handle_recovery_keys, handle_settings_keys, handle_support_keys,
    handle_sync_keys,
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
    if app.current_app_screen() == AppScreen::Lock {
        handle_lock_keys(app, key);
        return Action::Continue;
    }

    // Toast undo: 'z' triggers UndoPressed when a toast has an undo action
    if key == KeyCode::Char('z')
        && let Some(action_id) = app.undo_action_id.take()
    {
        let result = app
            .app_engine
            .handle_action(UserAction::UndoPressed { action_id });
        app.clear_status();
        handle_action_result(app, result);
        return Action::Continue;
    }

    // Engine-driven onboarding bypasses global keys (engine handles text input)
    if app.onboarding_engine.is_some() && app.current_app_screen() == AppScreen::Onboarding {
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
    if matches!(app.current_app_screen(), AppScreen::FormDialog { .. }) {
        if key == KeyCode::Esc {
            // Send cancel to engine — it handles dirty detection and
            // shows InlineConfirm if the form has unsaved changes (ADR-022).
            let from_fd = app.form_dialog_type();
            let result = app.app_engine.handle_action(UserAction::ActionPressed {
                action_id: "cancel".into(),
            });
            handle_action_result_with(app, result, from_fd);
            return Action::Continue;
        }
        handle_engine_keys(app, key);
        return Action::Continue;
    }

    // Engine-driven contact limit bypasses global keys when editing (digits are input)
    if app.current_app_screen() == AppScreen::ContactLimit {
        handle_engine_keys(app, key);
        return Action::Continue;
    }

    // Engine-driven duress PIN bypasses global keys — the PinInput consumes
    // digits 1–9 that would otherwise hit the global tab-switch shortcuts.
    if app.current_app_screen() == AppScreen::DuressPin {
        handle_engine_keys(app, key);
        return Action::Continue;
    }

    // Engine-driven emergency broadcast bypasses global keys — contact-id and
    // message TextInputs consume characters (incl. digits) that would otherwise
    // hit the global tab-switch shortcuts.
    if app.current_app_screen() == AppScreen::EmergencyBroadcast {
        handle_engine_keys(app, key);
        return Action::Continue;
    }

    // Engine-driven backup bypasses global keys — password and backup-data
    // TextInputs consume characters (incl. digits) that would otherwise hit
    // the global tab-switch shortcuts.
    if app.current_app_screen() == AppScreen::Backup {
        handle_engine_keys(app, key);
        return Action::Continue;
    }

    // Don't process global keys if in contact search mode
    if app.contact_search_mode && app.current_app_screen() == AppScreen::Contacts {
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
            // If an InlineConfirm is on screen, cancel it instead of navigating back
            let screen = app.app_engine.current_screen();
            if let Some(Component::InlineConfirm { id, .. }) = screen
                .components
                .iter()
                .find(|c| matches!(c, Component::InlineConfirm { .. }))
            {
                let from_fd = app.form_dialog_type();
                let result = app.app_engine.handle_action(UserAction::ActionPressed {
                    action_id: format!("cancel_{id}"),
                });
                handle_action_result_with(app, result, from_fd);
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
            | Screen::FormDialog
            | Screen::ContactDuplicates
            | Screen::ContactMerge
            | Screen::MyInfoEntryDetail
            | Screen::Activity
            | Screen::DeviceReplacement
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
        Screen::FormDialog => {
            eprintln!("WARNING: form dialog screen reached legacy handler unexpectedly");
        }
        Screen::Devices => handle_devices_keys(app, key),
        Screen::Recovery => handle_recovery_keys(app, key),
        Screen::Sync => handle_sync_keys(app, key),
        Screen::Delivery => handle_delivery_keys(app, key),
        Screen::Privacy => handle_privacy_keys(app, key),
        Screen::Support => handle_support_keys(app, key),
        Screen::ActionMenu => handle_action_menu_keys(app, key),
        Screen::Groups => handle_groups_keys(app, key),
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
        | Screen::DeviceReplacement
        | Screen::Duress
        | Screen::Emergency
        | Screen::Backup
        | Screen::GroupDetail
        | Screen::More
        | Screen::Activity => {
            eprintln!("WARNING: engine-only screen reached legacy handler unexpectedly");
        }
        Screen::ContactImport => {
            // Handled entirely in editing mode — Esc returns to Contacts
            if key == KeyCode::Esc {
                app.goto(Screen::Contacts);
            }
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
    app.ensure_engine_synced();

    // Get the current screen model from AppEngine
    let screen_model = app.app_engine.current_screen();

    // Map the key to a UserAction via the key_mapping module
    match key_mapping::map_key(key, &screen_model, &mut app.render_state) {
        KeyResult::Action(action) => {
            // On form dialog screens, "cancel" action means go back (don't forward to engine)
            if matches!(app.current_app_screen(), AppScreen::FormDialog { .. })
                && let UserAction::ActionPressed { ref action_id } = action
                && action_id == "cancel"
            {
                app.go_back();
                return;
            }
            // Forward the action to AppEngine. Capture the form-dialog kind
            // first so success feedback survives the engine's back-nav.
            let from_fd = app.form_dialog_type();
            let result = app.app_engine.handle_action(action);
            handle_action_result_with(app, result, from_fd);
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
                // Backup: engine-driven; Esc exits the flow.
                Screen::Backup => {
                    if key == KeyCode::Esc {
                        app.go_back();
                    }
                }
                Screen::Delivery => handle_delivery_keys(app, key),
                Screen::Devices => handle_devices_keys(app, key),
                // Duress: engine-driven; Esc exits the flow (ADR-021/043)
                Screen::Duress => {
                    if key == KeyCode::Esc {
                        app.go_back();
                    }
                }
                // Emergency broadcast: engine-driven; Esc exits the flow.
                Screen::Emergency => {
                    if key == KeyCode::Esc {
                        app.go_back();
                    }
                }
                Screen::Sync => handle_sync_keys(app, key),
                Screen::Recovery => handle_recovery_keys(app, key),
                Screen::Groups => handle_groups_keys(app, key),
                // GroupDetail: engine-driven (members List + rename action); Esc backs out.
                Screen::GroupDetail => {
                    if key == KeyCode::Esc {
                        app.go_back();
                    }
                }
                Screen::ContactVisibility => handle_visibility_keys(app, key),
                Screen::Privacy => handle_privacy_keys(app, key),
                Screen::Support => handle_support_keys(app, key),
                // Form dialogs: Esc goes back (engine handles chars/Enter via key_mapping)
                Screen::FormDialog => {
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
