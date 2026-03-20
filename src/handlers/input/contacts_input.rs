// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Contact-related input handlers: contact list, detail, actions, visibility.

use crossterm::event::KeyCode;
use vauchi_core::ui::WorkflowEngine;

use crate::app::{
    ActionMenuState, App, ContactLimitState, DuplicateEntry, DuplicatesState, Screen,
};
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
                    vauchi_core::ui::Component::ContactList { contacts, .. } => {
                        Some(contacts.len())
                    }
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
        KeyCode::Char('d') => {
            // Open duplicates screen
            match app.app_engine.vauchi().find_duplicates() {
                Ok(pairs) => {
                    // Look up display names for duplicate pairs
                    let contacts = app.app_engine.vauchi().list_contacts().unwrap_or_default();
                    let find_name = |id: &str| -> String {
                        contacts
                            .iter()
                            .find(|c| c.id() == id)
                            .map(|c| c.display_name().to_string())
                            .unwrap_or_else(|| id.to_string())
                    };
                    app.duplicates_state = DuplicatesState {
                        pairs: pairs
                            .into_iter()
                            .map(|p| DuplicateEntry {
                                name1: find_name(&p.id1),
                                id1: p.id1,
                                name2: find_name(&p.id2),
                                id2: p.id2,
                                similarity: p.similarity,
                            })
                            .collect(),
                        selected: 0,
                    };
                    app.goto(Screen::ContactDuplicates);
                }
                Err(e) => app.set_status(format!("Error: {}", e)),
            }
        }
        KeyCode::Char('L') => {
            // Open contact limit screen
            let limit = app.app_engine.vauchi().get_contact_limit().unwrap_or(500);
            let count = app.app_engine.vauchi().contact_count().unwrap_or(0);
            app.contact_limit_state = ContactLimitState {
                current_limit: limit,
                current_count: count,
                limit_input: limit.to_string(),
                editing: false,
            };
            app.goto(Screen::ContactLimit);
        }
        _ => {}
    }
}

/// Handle input for the groups list screen.
pub(super) fn handle_groups_keys(app: &mut App, key: KeyCode) {
    // Handle search mode
    if app.groups_state.group_search_mode {
        match key {
            KeyCode::Esc => {
                app.groups_state.group_search_mode = false;
            }
            KeyCode::Enter => {
                app.groups_state.group_search_mode = false;
            }
            KeyCode::Backspace => {
                app.groups_state.group_search_query.pop();
                app.groups_state.selected_group = 0;
            }
            KeyCode::Char(c) => {
                app.groups_state.group_search_query.push(c);
                app.groups_state.selected_group = 0;
            }
            _ => {}
        }
        return;
    }

    // Normal navigation mode
    match key {
        KeyCode::Char('/') => {
            app.groups_state.group_search_mode = true;
            app.groups_state.group_search_query.clear();
            app.groups_state.selected_group = 0;
        }
        KeyCode::Char('n') => {
            // Start creating a new group
            app.groups_state.edit_mode = true;
            app.groups_state.group_name_input.clear();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            if let Ok(groups) = app.app_engine.vauchi().list_groups() {
                let filtered_count = if app.groups_state.group_search_query.is_empty() {
                    groups.len()
                } else {
                    let query = app.groups_state.group_search_query.to_lowercase();
                    groups
                        .iter()
                        .filter(|g| g.name().to_lowercase().contains(&query))
                        .count()
                };
                if app.groups_state.selected_group < filtered_count.saturating_sub(1) {
                    app.groups_state.selected_group += 1;
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.groups_state.selected_group > 0 {
                app.groups_state.selected_group -= 1;
            }
        }
        KeyCode::Enter => {
            app.groups_state.show_group_detail = true;
            app.goto(Screen::GroupDetail);
        }
        KeyCode::Char('d') => {
            app.groups_state.delete_confirm = true;
        }
        _ => {}
    }
}

/// Handle input for the group detail screen.
pub(super) fn handle_group_detail_keys(app: &mut App, key: KeyCode) {
    if app.groups_state.edit_mode {
        match key {
            KeyCode::Esc => {
                app.groups_state.edit_mode = false;
                app.groups_state.group_name_input.clear();
            }
            KeyCode::Enter => {
                // Save the group name (create or rename)
                app.groups_state.edit_mode = false;
                app.groups_state.group_name_input.clear();
            }
            KeyCode::Backspace => {
                app.groups_state.group_name_input.pop();
            }
            KeyCode::Char(c) => {
                if app.groups_state.group_name_input.len() < 50 {
                    app.groups_state.group_name_input.push(c);
                }
            }
            _ => {}
        }
        return;
    }

    match key {
        KeyCode::Char('j') | KeyCode::Down => {
            if let Ok(groups) = app.app_engine.vauchi().list_groups()
                && let Some(group) = groups.get(app.groups_state.selected_group)
                && let Ok(contacts) = app.app_engine.vauchi().get_group_members(group.id())
                && app.groups_state.selected_contact_in_group < contacts.len().saturating_sub(1)
            {
                app.groups_state.selected_contact_in_group += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.groups_state.selected_contact_in_group > 0 {
                app.groups_state.selected_contact_in_group -= 1;
            }
        }
        KeyCode::Char('r') => {
            // Start renaming
            app.groups_state.edit_mode = true;
            if let Ok(groups) = app.app_engine.vauchi().list_groups()
                && let Some(group) = groups.get(app.groups_state.selected_group)
            {
                app.groups_state.group_name_input = group.name().to_string();
            }
        }
        _ => {}
    }
}

pub(super) fn handle_contact_detail_keys(app: &mut App, key: KeyCode) {
    // Handle delete confirmation overlay
    if app.contact_delete_confirm {
        match key {
            KeyCode::Char('y') | KeyCode::Enter => {
                app.contact_delete_confirm = false;
                if let Ok(contacts) = app.app_engine.vauchi().list_contacts()
                    && let Some(contact) = contacts.get(app.selected_contact)
                    && app.app_engine.vauchi().remove_contact(contact.id()).is_ok()
                {
                    app.invalidate_engines();
                    app.set_status("Contact removed");
                    app.go_back();
                }
            }
            _ => {
                app.contact_delete_confirm = false;
            }
        }
        return;
    }

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
            // Open visibility settings for this contact
            if let Ok(contacts) = app.app_engine.vauchi().list_contacts()
                && let Some(contact) = contacts.get(app.selected_contact)
            {
                app.visibility_state.contact_id = Some(contact.id().to_string());
                app.visibility_state.selected_field = 0;
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
        KeyCode::Char('V') => {
            // Validate selected field
            match app.app_engine.vauchi().list_contacts() {
                Ok(contacts) => {
                    if let Some(contact) = contacts.get(app.selected_contact) {
                        if let Some(field) = contact.card().fields().get(app.selected_contact_field)
                        {
                            match app.app_engine.vauchi().validate_field(
                                contact.id(),
                                field.label(),
                                field.value(),
                            ) {
                                Ok(_) => app.set_status(format!(
                                    "Validated {} for {}",
                                    field.label(),
                                    contact.display_name()
                                )),
                                Err(e) => app.set_status(format!("Error: {}", e)),
                            }
                        } else {
                            app.set_status("No field selected");
                        }
                    } else {
                        app.set_status("No contact selected");
                    }
                }
                _ => {
                    app.set_status("No contacts available");
                }
            }
        }
        KeyCode::Char('R') => {
            // Revoke validation on selected field
            match app.app_engine.vauchi().list_contacts() {
                Ok(contacts) => {
                    if let Some(contact) = contacts.get(app.selected_contact) {
                        if let Some(field) = contact.card().fields().get(app.selected_contact_field)
                        {
                            match app
                                .app_engine
                                .vauchi()
                                .revoke_field_validation(contact.id(), field.label())
                            {
                                Ok(true) => app.set_status(format!(
                                    "Revoked validation for {}",
                                    field.label()
                                )),
                                Ok(false) => app.set_status("No validation to revoke"),
                                Err(e) => app.set_status(format!("Error: {}", e)),
                            }
                        } else {
                            app.set_status("No field selected");
                        }
                    } else {
                        app.set_status("No contact selected");
                    }
                }
                _ => {
                    app.set_status("No contacts available");
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
        KeyCode::Char('x') | KeyCode::Delete => {
            // Show delete confirmation
            app.contact_delete_confirm = true;
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
            // Count own card fields for navigation bounds
            if let Ok(Some(card)) = app.app_engine.vauchi().own_card()
                && app.visibility_state.selected_field < card.fields().len().saturating_sub(1)
            {
                app.visibility_state.selected_field += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.visibility_state.selected_field > 0 {
                app.visibility_state.selected_field -= 1;
            }
        }
        KeyCode::Char(' ') | KeyCode::Enter => {
            if let Some(ref contact_id) = app.visibility_state.contact_id
                && let Ok(Some(card)) = app.app_engine.vauchi().own_card()
                && let Some(field) = card.fields().get(app.visibility_state.selected_field)
            {
                let field_label = field.label().to_string();
                match app
                    .app_engine
                    .vauchi()
                    .toggle_field_visibility(contact_id, &field_label)
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
