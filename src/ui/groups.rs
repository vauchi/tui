// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Contact Groups Screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::app::App;

/// Renders the groups list screen with a create hint and scrollable group list.
pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let groups = app.backend.list_groups().unwrap_or_default();

    // Split area for create hint and list
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    // Hint line for creating new group
    let hint = if app.groups_state.edit_mode {
        "Enter group name (max 50 chars), press Enter to create"
    } else {
        "Press [n] to create a new group"
    };
    let hint_block = Paragraph::new(hint)
        .style(Style::default().fg(app.theme.fg_secondary))
        .block(Block::default().borders(Borders::ALL).title("Info"));
    f.render_widget(hint_block, chunks[0]);

    if groups.is_empty() {
        let empty = Paragraph::new(app.i18n.t("groups.empty"))
            .style(Style::default().fg(app.theme.fg_secondary))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(app.i18n.t("groups.title")),
            );
        f.render_widget(empty, chunks[1]);
        return;
    }

    let items: Vec<ListItem> = groups
        .iter()
        .enumerate()
        .map(|(i, group)| {
            let contact_text = if group.contact_count == 1 {
                "1 contact".to_string()
            } else {
                format!("{} contacts", group.contact_count)
            };
            let content = format!("{} ({})", group.name, contact_text);
            let style = if i == app.groups_state.selected_group {
                Style::default()
                    .fg(app.theme.warning)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(content).style(style)
        })
        .collect();

    let list_title = app
        .i18n
        .t_args("groups.shown", &[("count", &groups.len().to_string())]);
    let list = List::new(items)
        .block(Block::default().title(list_title).borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = ListState::default();
    state.select(Some(app.groups_state.selected_group));
    f.render_stateful_widget(list, chunks[1], &mut state);
}

/// Renders the detail view for a selected group, listing its member contacts.
pub fn draw_detail(f: &mut Frame, area: Rect, app: &App) {
    let groups = app.backend.list_groups().unwrap_or_default();
    let group = groups.get(app.groups_state.selected_group);

    match group {
        Some(g) => {
            let contacts = app.backend.get_contacts_in_group(&g.id).unwrap_or_default();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Group name
                    Constraint::Min(5),    // Contacts
                    Constraint::Length(2), // Help
                ])
                .split(area);

            // Group name
            let name_title = if app.groups_state.edit_mode {
                "Rename Group"
            } else {
                "Group Name"
            };
            let name_text = if app.groups_state.edit_mode {
                app.groups_state.group_name_input.clone()
            } else {
                g.name.clone()
            };
            let name = Paragraph::new(name_text)
                .style(Style::default().add_modifier(Modifier::BOLD))
                .block(Block::default().title(name_title).borders(Borders::ALL));
            f.render_widget(name, chunks[0]);

            // Contacts list
            if contacts.is_empty() {
                let empty = Paragraph::new("No contacts in this group")
                    .style(Style::default().fg(app.theme.fg_secondary))
                    .block(Block::default().title("Contacts").borders(Borders::ALL));
                f.render_widget(empty, chunks[1]);
            } else {
                let items: Vec<ListItem> = contacts
                    .iter()
                    .enumerate()
                    .map(|(i, contact)| {
                        let verified = if contact.verified { "✓" } else { " " };
                        let content = format!(
                            "[{}] {}  ({}...)",
                            verified,
                            contact.display_name,
                            &contact.id[..8]
                        );
                        let style = if i == app.groups_state.selected_contact_in_group {
                            Style::default()
                                .fg(app.theme.warning)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        };
                        ListItem::new(content).style(style)
                    })
                    .collect();

                let list_title = format!("Contacts ({})", contacts.len());
                let list = List::new(items)
                    .block(Block::default().title(list_title).borders(Borders::ALL))
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

                let mut state = ListState::default();
                state.select(Some(app.groups_state.selected_contact_in_group));
                f.render_stateful_widget(list, chunks[1], &mut state);
            }

            // Help line
            let help = if app.groups_state.edit_mode {
                "[enter] save  [esc] cancel"
            } else {
                "[r]ename  [j/k] navigate  [esc] back"
            };
            let help_widget =
                Paragraph::new(help).style(Style::default().fg(app.theme.fg_secondary));
            f.render_widget(help_widget, chunks[2]);
        }
        None => {
            let empty =
                Paragraph::new("Group not found").style(Style::default().fg(app.theme.error));
            f.render_widget(empty, area);
        }
    }
}
