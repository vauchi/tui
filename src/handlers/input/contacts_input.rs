// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Contact-related input handlers: contact list, detail, actions, visibility.

use crossterm::event::KeyCode;

use crate::app::{ActionMenuState, App, Screen};

pub(super) fn handle_contacts_keys(app: &mut App, key: KeyCode) {
    // Handle search mode
    if app.contact_search_mode {
        match key {
            KeyCode::Esc => {
                app.contact_search_mode = false;
            }
            KeyCode::Enter => {
                app.contact_search_mode = false;
            }
            KeyCode::Backspace => {
                app.contact_search_query.pop();
                app.selected_contact = 0;
            }
            KeyCode::Char(c) => {
                app.contact_search_query.push(c);
                app.selected_contact = 0;
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
        }
        KeyCode::Char('j') | KeyCode::Down => {
            // Count filtered contacts
            let contacts = app.backend.list_contacts().unwrap_or_default();
            let filtered_count = if app.contact_search_query.is_empty() {
                contacts.len()
            } else {
                let query = app.contact_search_query.to_lowercase();
                contacts
                    .iter()
                    .filter(|c| c.display_name.to_lowercase().contains(&query))
                    .count()
            };
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
            app.goto(Screen::ContactDetail);
        }
        _ => {}
    }
}

pub(super) fn handle_contact_detail_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('j') | KeyCode::Down => {
            // Navigate down through contact fields
            if let Ok(fields) = app.backend.get_contact_fields(app.selected_contact) {
                if app.selected_contact_field < fields.len().saturating_sub(1) {
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
            match app
                .backend
                .get_secondary_actions(app.selected_contact, app.selected_contact_field)
            {
                Ok(actions) if actions.len() > 1 => {
                    app.action_menu_state = ActionMenuState {
                        actions,
                        selected: 0,
                    };
                    app.goto(Screen::ActionMenu);
                }
                Ok(_) => {
                    // Single action — execute directly
                    match app
                        .backend
                        .open_contact_field(app.selected_contact, app.selected_contact_field)
                    {
                        Ok(msg) => app.set_status(msg),
                        Err(e) => app.set_status(format!("Error: {}", e)),
                    }
                }
                Err(e) => app.set_status(format!("Error: {}", e)),
            }
        }
        KeyCode::Char('c') => {
            // Copy field value to clipboard
            match app
                .backend
                .copy_field_to_clipboard(app.selected_contact, app.selected_contact_field)
            {
                Ok(msg) => app.set_status(msg),
                Err(e) => app.set_status(format!("Error: {}", e)),
            }
        }
        KeyCode::Char('v') => {
            // Open visibility settings for this contact
            if let Ok(Some(contact)) = app.backend.get_contact_by_index(app.selected_contact) {
                app.visibility_state.contact_id = Some(contact.id);
                app.visibility_state.selected_field = 0;
                app.goto(Screen::ContactVisibility);
            }
        }
        KeyCode::Char('t') => {
            // Toggle recovery trust
            if let Ok(contacts) = app.backend.list_contacts() {
                if let Some(contact) = contacts.get(app.selected_contact) {
                    match app.backend.toggle_recovery_trust(&contact.id) {
                        Ok(true) => app.set_status("Marked as recovery-trusted"),
                        Ok(false) => app.set_status("Removed recovery trust"),
                        Err(e) => app.set_status(format!("Error: {}", e)),
                    }
                }
            }
        }
        KeyCode::Char('V') => {
            // Validate selected field
            if let Ok(contacts) = app.backend.list_contacts() {
                if let Some(contact) = contacts.get(app.selected_contact) {
                    if let Ok(fields) = app.backend.get_contact_fields(app.selected_contact) {
                        if let Some(field) = fields.get(app.selected_contact_field) {
                            match app.backend.validate_field(
                                &contact.id,
                                &field.label,
                                &field.value,
                            ) {
                                Ok(_) => app.set_status(format!(
                                    "Validated {} for {}",
                                    field.label, contact.display_name
                                )),
                                Err(e) => app.set_status(format!("Error: {}", e)),
                            }
                        } else {
                            app.set_status("No field selected");
                        }
                    } else {
                        app.set_status("No fields available");
                    }
                } else {
                    app.set_status("No contact selected");
                }
            } else {
                app.set_status("No contacts available");
            }
        }
        KeyCode::Char('R') => {
            // Revoke validation on selected field
            if let Ok(contacts) = app.backend.list_contacts() {
                if let Some(contact) = contacts.get(app.selected_contact) {
                    if let Ok(fields) = app.backend.get_contact_fields(app.selected_contact) {
                        if let Some(field) = fields.get(app.selected_contact_field) {
                            match app
                                .backend
                                .revoke_field_validation(&contact.id, &field.label)
                            {
                                Ok(true) => app
                                    .set_status(format!("Revoked validation for {}", field.label)),
                                Ok(false) => app.set_status("No validation to revoke"),
                                Err(e) => app.set_status(format!("Error: {}", e)),
                            }
                        } else {
                            app.set_status("No field selected");
                        }
                    } else {
                        app.set_status("No fields available");
                    }
                } else {
                    app.set_status("No contact selected");
                }
            } else {
                app.set_status("No contacts available");
            }
        }
        KeyCode::Char('f') => {
            // Show fingerprint and verify
            if let Ok(contacts) = app.backend.list_contacts() {
                if let Some(contact) = contacts.get(app.selected_contact) {
                    match app.backend.get_contact_fingerprint(&contact.id) {
                        Ok(fp) => {
                            if fp.is_verified {
                                app.set_status(format!(
                                    "Already verified. Theirs: {}  Ours: {}",
                                    fp.their_fingerprint, fp.our_fingerprint
                                ));
                            } else {
                                match app.backend.verify_contact_fingerprint(&contact.id) {
                                    Ok(()) => app.set_status(format!(
                                        "Verified! Theirs: {}  Ours: {}",
                                        fp.their_fingerprint, fp.our_fingerprint
                                    )),
                                    Err(e) => app.set_status(format!("Error: {}", e)),
                                }
                            }
                        }
                        Err(e) => app.set_status(format!("Error: {}", e)),
                    }
                }
            }
        }
        KeyCode::Char('h') => {
            // Toggle hide/unhide contact
            if let Ok(contacts) = app.backend.list_contacts() {
                if let Some(contact) = contacts.get(app.selected_contact) {
                    match app.backend.is_contact_hidden(&contact.id) {
                        Ok(true) => match app.backend.unhide_contact(&contact.id) {
                            Ok(()) => {
                                app.set_status(format!("{} is now visible", contact.display_name));
                            }
                            Err(e) => app.set_status(format!("Error: {}", e)),
                        },
                        Ok(false) => match app.backend.hide_contact(&contact.id) {
                            Ok(()) => {
                                app.set_status(format!(
                                    "{} hidden from contact list",
                                    contact.display_name
                                ));
                                app.go_back();
                            }
                            Err(e) => app.set_status(format!("Error: {}", e)),
                        },
                        Err(e) => app.set_status(format!("Error: {}", e)),
                    }
                }
            }
        }
        KeyCode::Char('x') | KeyCode::Delete => {
            // Delete contact
            if let Ok(contacts) = app.backend.list_contacts() {
                if let Some(contact) = contacts.get(app.selected_contact) {
                    if app.backend.remove_contact(&contact.id).is_ok() {
                        app.set_status("Contact removed");
                        app.go_back();
                    }
                }
            }
        }
        _ => {}
    }
}

pub(super) fn handle_action_menu_keys(app: &mut App, key: KeyCode) {
    use vauchi_core::contact_card::ContactAction;

    let action_count = app.action_menu_state.actions.len();
    match key {
        KeyCode::Char('j') | KeyCode::Down => {
            if app.action_menu_state.selected < action_count.saturating_sub(1) {
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
                let result = if matches!(action, ContactAction::CopyToClipboard) {
                    app.backend
                        .copy_field_to_clipboard(app.selected_contact, app.selected_contact_field)
                } else {
                    app.backend.execute_action(action)
                };
                match result {
                    Ok(msg) => app.set_status(msg),
                    Err(e) => app.set_status(format!("Error: {}", e)),
                }
            }
            app.go_back();
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            app.go_back();
        }
        _ => {}
    }
}

pub(super) fn handle_visibility_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('j') | KeyCode::Down => {
            if let Some(ref contact_id) = app.visibility_state.contact_id {
                if let Ok(fields) = app.backend.get_contact_visibility(contact_id) {
                    if app.visibility_state.selected_field < fields.len().saturating_sub(1) {
                        app.visibility_state.selected_field += 1;
                    }
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.visibility_state.selected_field > 0 {
                app.visibility_state.selected_field -= 1;
            }
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            // Toggle visibility for selected field
            if let Some(ref contact_id) = app.visibility_state.contact_id.clone() {
                if let Ok(fields) = app.backend.get_contact_visibility(contact_id) {
                    if let Some(field) = fields.get(app.visibility_state.selected_field) {
                        match app
                            .backend
                            .toggle_field_visibility(contact_id, &field.field_label)
                        {
                            Ok(now_visible) => {
                                let status = if now_visible {
                                    "now visible"
                                } else {
                                    "now hidden"
                                };
                                app.set_status(format!("Field {} {}", field.field_label, status));
                            }
                            Err(e) => app.set_status(format!("Error: {}", e)),
                        }
                    }
                }
            }
        }
        _ => {}
    }
}
