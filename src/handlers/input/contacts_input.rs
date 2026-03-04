// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Contact-related input handlers: contact list, detail, actions, visibility.

use crossterm::event::KeyCode;

use crate::app::{
    ActionMenuState, App, ContactLimitState, DuplicateEntry, DuplicatesState, InputMode,
    MergeState, Screen,
};

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
        KeyCode::Char('d') => {
            // Open duplicates screen
            match app.backend.find_duplicates() {
                Ok(pairs) => {
                    app.duplicates_state = DuplicatesState {
                        pairs: pairs
                            .into_iter()
                            .map(|p| DuplicateEntry {
                                id1: p.id1,
                                name1: p.name1,
                                id2: p.id2,
                                name2: p.name2,
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
            let limit = app.backend.get_contact_limit().unwrap_or(500);
            let count = app.backend.contact_count().unwrap_or(0);
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
            if let Ok(groups) = app.backend.list_groups() {
                let filtered_count = if app.groups_state.group_search_query.is_empty() {
                    groups.len()
                } else {
                    let query = app.groups_state.group_search_query.to_lowercase();
                    groups
                        .iter()
                        .filter(|g| g.name.to_lowercase().contains(&query))
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
            if let Ok(groups) = app.backend.list_groups() {
                if let Some(group) = groups.get(app.groups_state.selected_group) {
                    if let Ok(contacts) = app.backend.get_contacts_in_group(&group.id) {
                        if app.groups_state.selected_contact_in_group
                            < contacts.len().saturating_sub(1)
                        {
                            app.groups_state.selected_contact_in_group += 1;
                        }
                    }
                }
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
            if let Ok(groups) = app.backend.list_groups() {
                if let Some(group) = groups.get(app.groups_state.selected_group) {
                    app.groups_state.group_name_input = group.name.clone();
                }
            }
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
                match app.backend.execute_action(action) {
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
        KeyCode::Char(' ') | KeyCode::Enter => {
            if let Some(ref contact_id) = app.visibility_state.contact_id {
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

// ── SP-12a Duplicate / Merge / Limit Handlers ──

pub(super) fn handle_duplicates_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('j') | KeyCode::Down => {
            if app.duplicates_state.selected < app.duplicates_state.pairs.len().saturating_sub(1) {
                app.duplicates_state.selected += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.duplicates_state.selected > 0 {
                app.duplicates_state.selected -= 1;
            }
        }
        KeyCode::Char('m') => {
            // Open merge preview for selected pair
            if let Some(pair) = app
                .duplicates_state
                .pairs
                .get(app.duplicates_state.selected)
            {
                let primary_fields = app
                    .backend
                    .get_contact_fields_by_id(&pair.id1)
                    .unwrap_or_default();
                let secondary_fields = app
                    .backend
                    .get_contact_fields_by_id(&pair.id2)
                    .unwrap_or_default();
                app.merge_state = MergeState {
                    primary_id: pair.id1.clone(),
                    primary_name: pair.name1.clone(),
                    primary_fields,
                    secondary_id: pair.id2.clone(),
                    secondary_name: pair.name2.clone(),
                    secondary_fields,
                };
                app.goto(Screen::ContactMerge);
            }
        }
        KeyCode::Char('d') => {
            // Dismiss selected pair
            if let Some(pair) = app
                .duplicates_state
                .pairs
                .get(app.duplicates_state.selected)
            {
                let id1 = pair.id1.clone();
                let id2 = pair.id2.clone();
                match app.backend.dismiss_duplicate(&id1, &id2) {
                    Ok(()) => {
                        app.duplicates_state
                            .pairs
                            .remove(app.duplicates_state.selected);
                        if app.duplicates_state.selected > 0
                            && app.duplicates_state.selected >= app.duplicates_state.pairs.len()
                        {
                            app.duplicates_state.selected -= 1;
                        }
                        app.set_status("Duplicate dismissed");
                    }
                    Err(e) => app.set_status(format!("Error: {}", e)),
                }
            }
        }
        _ => {}
    }
}

pub(super) fn handle_merge_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('y') => {
            // Confirm merge
            let primary_id = app.merge_state.primary_id.clone();
            let secondary_id = app.merge_state.secondary_id.clone();
            match app.backend.merge_contacts(&primary_id, &secondary_id) {
                Ok(name) => {
                    app.set_status(format!("Merged contacts into {}", name));
                    // Remove the pair from duplicates list
                    app.duplicates_state.pairs.retain(|p| {
                        !((p.id1 == primary_id && p.id2 == secondary_id)
                            || (p.id1 == secondary_id && p.id2 == primary_id))
                    });
                    if app.duplicates_state.selected > 0
                        && app.duplicates_state.selected >= app.duplicates_state.pairs.len()
                    {
                        app.duplicates_state.selected =
                            app.duplicates_state.pairs.len().saturating_sub(1);
                    }
                    app.merge_state = MergeState::default();
                    app.goto(Screen::ContactDuplicates);
                }
                Err(e) => app.set_status(format!("Merge failed: {}", e)),
            }
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            // Cancel merge
            app.go_back();
        }
        _ => {}
    }
}

pub(super) fn handle_contact_limit_keys(app: &mut App, key: KeyCode) {
    if app.contact_limit_state.editing {
        match key {
            KeyCode::Char(c) if c.is_ascii_digit() => {
                if app.contact_limit_state.limit_input.len() < 6 {
                    app.contact_limit_state.limit_input.push(c);
                }
            }
            KeyCode::Backspace => {
                app.contact_limit_state.limit_input.pop();
            }
            KeyCode::Enter => {
                if let Ok(limit) = app.contact_limit_state.limit_input.parse::<usize>() {
                    if limit < 1 {
                        app.set_status("Limit must be at least 1");
                    } else {
                        match app.backend.set_contact_limit(limit) {
                            Ok(()) => {
                                app.contact_limit_state.current_limit = limit;
                                app.contact_limit_state.editing = false;
                                app.input_mode = InputMode::Normal;
                                app.set_status(format!("Contact limit set to {}", limit));
                            }
                            Err(e) => app.set_status(format!("Error: {}", e)),
                        }
                    }
                } else {
                    app.set_status("Invalid number");
                }
            }
            KeyCode::Esc => {
                app.contact_limit_state.editing = false;
                app.contact_limit_state.limit_input =
                    app.contact_limit_state.current_limit.to_string();
                app.input_mode = InputMode::Normal;
            }
            _ => {}
        }
    } else {
        match key {
            KeyCode::Char('e') | KeyCode::Enter => {
                app.contact_limit_state.editing = true;
                app.input_mode = InputMode::Editing;
            }
            _ => {}
        }
    }
}
