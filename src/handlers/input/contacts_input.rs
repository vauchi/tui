// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Contact-related input handlers: contact list, detail, actions, visibility.

use crossterm::event::KeyCode;
use vauchi_app::ui::{UserAction, WorkflowEngine};

use crate::app::{ActionMenuState, App, ImportState, InputMode, Screen};
use crate::helpers;

pub(super) fn handle_contacts_keys(app: &mut App, key: KeyCode) {
    // Handle search mode
    if app.contact_search_mode {
        match key {
            KeyCode::Esc => {
                app.contact_search_mode = false;
                app.contact_search_query.clear();
                app.selected_contact = 0;
                helpers::dispatch_search(app);
            }
            KeyCode::Enter => {
                app.contact_search_mode = false;
            }
            KeyCode::Backspace => {
                app.contact_search_query.pop();
                app.selected_contact = 0;
                // Sync search to engine for field-aware filtering
                helpers::dispatch_search(app);
            }
            KeyCode::Char(c) => {
                app.contact_search_query.push(c);
                app.selected_contact = 0;
                helpers::dispatch_search(app);
            }
            _ => {}
        }
        return;
    }

    // Normal navigation mode
    match key {
        KeyCode::Char('/') => {
            app.contact_search_mode = true;
            app.contact_search_query.clear();
            app.selected_contact = 0;
            helpers::dispatch_search(app);
        }
        KeyCode::Char('g') => {
            // Cycle through group filters
            helpers::cycle_group_filter(app);
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let screen = app.app_engine.current_screen();
            let filtered_count = screen
                .components
                .iter()
                .find_map(|c| match c {
                    vauchi_app::ui::Component::List { items, .. } => Some(items.len()),
                    _ => None,
                })
                .unwrap_or(0);
            if app.selected_contact < filtered_count.saturating_sub(1) {
                app.selected_contact += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.selected_contact > 0 {
                app.selected_contact -= 1;
            }
        }
        KeyCode::Enter => {
            // Look up contact ID for engine-driven ContactDetail screen
            if let Ok(contacts) = app.app_engine.vauchi().list_contacts()
                && let Some(contact) = contacts.get(app.selected_contact)
            {
                app.selected_contact_id = Some(contact.id().to_string());
            }
            app.goto(Screen::ContactDetail);
        }
        KeyCode::Char('d') | KeyCode::Char('m') => {
            // Open duplicates / merge screen ('d' legacy, 'm' for merge).
            // Engine builds the duplicate list from `find_duplicates()`;
            // the renderer reads from the engine's ScreenModel.
            match app.app_engine.vauchi().find_duplicates() {
                Ok(_) => app.goto(Screen::ContactDuplicates),
                Err(e) => app.set_status(format!("Error: {}", e)),
            }
        }
        KeyCode::Char('i') => {
            app.import_state = ImportState::default();
            app.input_mode = InputMode::Editing;
            app.goto(Screen::ContactImport);
        }
        KeyCode::Char('L') => {
            // Engine owns current_limit / current_count via ContactLimit screen.
            app.goto(Screen::ContactLimit);
        }
        _ => {}
    }
}
pub(super) fn handle_contact_detail_keys(app: &mut App, key: KeyCode) {
    // Resolve the correct contact index from selected_contact_id if available.
    // The engine path sets selected_contact_id (String), but all legacy operations
    // below use selected_contact (usize index). Sync the index to prevent operating
    // on the wrong contact.
    if let Some(ref id) = app.selected_contact_id
        && let Ok(contacts) = app.app_engine.vauchi().list_contacts()
        && let Some(idx) = contacts.iter().position(|c| c.id() == id.as_str())
    {
        app.selected_contact = idx;
    }

    match key {
        KeyCode::Char('j') | KeyCode::Down => {
            // Navigate down through contact fields
            if let Ok(contacts) = app.app_engine.vauchi().list_contacts()
                && let Some(contact) = contacts.get(app.selected_contact)
            {
                let field_count = contact.card().fields().len();
                if app.selected_contact_field < field_count.saturating_sub(1) {
                    app.selected_contact_field += 1;
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            // Navigate up through contact fields
            if app.selected_contact_field > 0 {
                app.selected_contact_field -= 1;
            }
        }
        KeyCode::Enter | KeyCode::Char('o') => {
            // Show action menu for the selected field
            if let Ok(contacts) = app.app_engine.vauchi().list_contacts()
                && let Some(contact) = contacts.get(app.selected_contact)
                && let Some(field) = contact.card().fields().get(app.selected_contact_field)
            {
                let actions: Vec<_> = field
                    .to_secondary_actions()
                    .into_iter()
                    .map(|a| (helpers::action_label(&a), a))
                    .collect();
                if actions.len() > 1 {
                    app.action_menu_state = ActionMenuState {
                        actions,
                        selected: 0,
                    };
                    app.goto(Screen::ActionMenu);
                } else if let Some(uri) = field.to_uri() {
                    let action_type = field.to_action();
                    let type_name = match &action_type {
                        vauchi_core::contact_card::ContactAction::Call(_) => "call",
                        vauchi_core::contact_card::ContactAction::SendSms(_) => "sms",
                        vauchi_core::contact_card::ContactAction::SendEmail(_) => "email",
                        vauchi_core::contact_card::ContactAction::OpenUrl(_) => "web",
                        vauchi_core::contact_card::ContactAction::OpenMap(_) => "map",
                        vauchi_core::contact_card::ContactAction::GetDirections(_) => "directions",
                        vauchi_core::contact_card::ContactAction::CopyToClipboard => "copy",
                        _ => "action",
                    };
                    let msg = helpers::open_field_uri(&uri, field.label(), type_name);
                    app.set_status(msg);
                }
            }
        }
        KeyCode::Char('c') => {
            // Copy field value to clipboard
            if let Ok(contacts) = app.app_engine.vauchi().list_contacts() {
                if let Some(contact) = contacts.get(app.selected_contact) {
                    if let Some(field) = contact.card().fields().get(app.selected_contact_field) {
                        match helpers::copy_to_clipboard(field.value()) {
                            Ok(_) => {
                                app.set_status(format!("Copied {} to clipboard", field.label()))
                            }
                            Err(e) => app.set_status(format!("Error: {}", e)),
                        }
                    } else {
                        app.set_status("No field to copy");
                    }
                } else {
                    app.set_status("No contact selected");
                }
            }
        }
        KeyCode::Char('v') => {
            // Open visibility settings for this contact. The engine's
            // AppScreen::ContactVisibility carries the contact_id; locally
            // we only track the selected field index.
            if let Ok(contacts) = app.app_engine.vauchi().list_contacts()
                && let Some(contact) = contacts.get(app.selected_contact)
            {
                app.selected_contact_id = Some(contact.id().to_string());
                app.selected_visibility_field = 0;
                app.goto(Screen::ContactVisibility);
            }
        }
        KeyCode::Char('t') => {
            // Toggle recovery trust
            if let Ok(contacts) = app.app_engine.vauchi().list_contacts()
                && let Some(contact) = contacts.get(app.selected_contact)
            {
                match app.app_engine.vauchi().toggle_recovery_trust(contact.id()) {
                    Ok(true) => {
                        app.invalidate_engines();
                        app.set_status("Marked as recovery-trusted");
                    }
                    Ok(false) => {
                        app.invalidate_engines();
                        app.set_status("Removed recovery trust");
                    }
                    Err(e) => app.set_status(format!("Error: {}", e)),
                }
            }
        }
        KeyCode::Char('f') => {
            // Show fingerprint and verify
            if let Ok(contacts) = app.app_engine.vauchi().list_contacts()
                && let Some(contact) = contacts.get(app.selected_contact)
            {
                let their_fp = contact.fingerprint();
                let our_fp = app
                    .app_engine
                    .vauchi()
                    .own_fingerprint()
                    .unwrap_or_default();
                if contact.is_fingerprint_verified() {
                    app.set_status(format!(
                        "Already verified. Theirs: {}  Ours: {}",
                        their_fp, our_fp
                    ));
                } else {
                    match app
                        .app_engine
                        .vauchi()
                        .verify_contact_fingerprint(contact.id())
                    {
                        Ok(()) => app.set_status(format!(
                            "Verified! Theirs: {}  Ours: {}",
                            their_fp, our_fp
                        )),
                        Err(e) => app.set_status(format!("Error: {}", e)),
                    }
                }
            }
        }
        KeyCode::Char('b') => {
            // Toggle block/unblock contact
            if let Ok(contacts) = app.app_engine.vauchi().list_contacts()
                && let Some(contact) = contacts.get(app.selected_contact)
            {
                let contact_id = contact.id().to_string();
                let display_name = contact.display_name().to_string();
                if contact.is_blocked() {
                    match app.app_engine.vauchi().unblock_contact(&contact_id) {
                        Ok(()) => {
                            app.invalidate_engines();
                            app.set_status(format!("{} unblocked", display_name));
                        }
                        Err(e) => app.set_status(format!("Error: {}", e)),
                    }
                } else {
                    match app.app_engine.vauchi().block_contact(&contact_id) {
                        Ok(()) => {
                            app.invalidate_engines();
                            app.set_status(format!("{} blocked", display_name));
                            app.go_back();
                        }
                        Err(e) => app.set_status(format!("Error: {}", e)),
                    }
                }
            }
        }
        KeyCode::Char('h') => {
            // Toggle hide/unhide contact
            if let Ok(contacts) = app.app_engine.vauchi().list_contacts()
                && let Some(contact) = contacts.get(app.selected_contact)
            {
                let contact_id = contact.id().to_string();
                let display_name = contact.display_name().to_string();
                if contact.is_hidden() {
                    match app.app_engine.vauchi().unhide_contact(&contact_id) {
                        Ok(()) => {
                            app.set_status(format!("{} is now visible", display_name));
                        }
                        Err(e) => app.set_status(format!("Error: {}", e)),
                    }
                } else {
                    match app.app_engine.vauchi().hide_contact(&contact_id) {
                        Ok(()) => {
                            app.set_status(format!("{} hidden from contact list", display_name));
                            app.go_back();
                        }
                        Err(e) => app.set_status(format!("Error: {}", e)),
                    }
                }
            }
        }
        KeyCode::Char('x') | KeyCode::Delete | KeyCode::Char('d') => {
            // Dispatch delete_contact (imported) or archive_contact (exchanged)
            // via the AppEngine intercept, which validates contact kind.
            let action_id = app
                .app_engine
                .current_screen()
                .actions
                .iter()
                .find(|a| a.enabled && (a.id == "delete_contact" || a.id == "archive_contact"))
                .map(|a| a.id.clone());
            if let Some(id) = action_id {
                let result = app
                    .app_engine
                    .handle_action(UserAction::ActionPressed { action_id: id });
                // AppEngine navigates back internally; sync TUI screen state
                app.go_back();
                // Contact delete/archive — never a form dialog, so no kind.
                crate::handlers::action_result::handle_action_result(app, result);
            }
        }
        _ => {}
    }
}

/// Handle input for the action menu popup.
pub(super) fn handle_action_menu_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('j') | KeyCode::Down => {
            if app.action_menu_state.selected
                < app.action_menu_state.actions.len().saturating_sub(1)
            {
                app.action_menu_state.selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.action_menu_state.selected > 0 {
                app.action_menu_state.selected -= 1;
            }
        }
        KeyCode::Enter => {
            if let Some((_, action)) = app
                .action_menu_state
                .actions
                .get(app.action_menu_state.selected)
            {
                match helpers::execute_action(action) {
                    Ok(msg) => app.set_status(msg),
                    Err(e) => app.set_status(format!("Error: {}", e)),
                }
            }
            app.go_back();
        }
        _ => {}
    }
}

/// Handle input for the visibility settings screen.
pub(super) fn handle_visibility_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('j') | KeyCode::Down => {
            if let Ok(Some(card)) = app.app_engine.vauchi().own_card()
                && app.selected_visibility_field < card.fields().len().saturating_sub(1)
            {
                app.selected_visibility_field += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.selected_visibility_field > 0 {
                app.selected_visibility_field -= 1;
            }
        }
        KeyCode::Char(' ') | KeyCode::Enter => {
            let contact_id = match app.app_engine.current_app_screen() {
                vauchi_app::ui::AppScreen::ContactVisibility { contact_id } => {
                    Some(contact_id.clone())
                }
                _ => app.selected_contact_id.clone(),
            };
            if let Some(contact_id) = contact_id
                && let Ok(Some(card)) = app.app_engine.vauchi().own_card()
                && let Some(field) = card.fields().get(app.selected_visibility_field)
            {
                let field_label = field.label().to_string();
                match app
                    .app_engine
                    .vauchi()
                    .toggle_field_visibility(&contact_id, &field_label)
                {
                    Ok(now_visible) => {
                        let status = if now_visible {
                            "now visible"
                        } else {
                            "now hidden"
                        };
                        app.set_status(format!("Field {} {}", field_label, status));
                    }
                    Err(e) => app.set_status(format!("Error: {}", e)),
                }
            }
        }
        _ => {}
    }
}
