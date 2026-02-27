// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Contacts Screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use vauchi_core::TrustLevel;

use crate::app::App;

/// Filter contacts based on search query.
///
/// Delegates to core's find_contact_fuzzy when a query is provided,
/// which combines name substring matching with ID prefix matching.
fn filter_contacts(
    app: &App,
    contacts: &[crate::backend::ContactInfo],
    query: &str,
) -> Vec<(usize, crate::backend::ContactInfo)> {
    if query.is_empty() {
        contacts
            .iter()
            .enumerate()
            .map(|(i, c)| (i, c.clone()))
            .collect()
    } else {
        let fuzzy_results = app.backend.find_contact_fuzzy(query).unwrap_or_default();
        // Map fuzzy results back to original indices for selection tracking
        fuzzy_results
            .into_iter()
            .filter_map(|result| {
                contacts
                    .iter()
                    .position(|c| c.id == result.id)
                    .map(|i| (i, result))
            })
            .collect()
    }
}

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let contacts = app.backend.list_contacts().unwrap_or_default();

    // Split area for search bar and list
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(area);

    // Search bar
    let search_title = if app.contact_search_mode {
        app.i18n.t("contacts.search_active")
    } else {
        app.i18n.t("contacts.search_hint")
    };
    let search_style = if app.contact_search_mode {
        Style::default().fg(app.theme.warning)
    } else {
        Style::default()
    };
    let search_text = if app.contact_search_query.is_empty() && !app.contact_search_mode {
        app.i18n.t("contacts.search_placeholder")
    } else {
        app.contact_search_query.clone()
    };
    let search_bar = Paragraph::new(search_text)
        .style(search_style)
        .block(Block::default().borders(Borders::ALL).title(search_title));
    f.render_widget(search_bar, chunks[0]);

    // Filter contacts
    let filtered = filter_contacts(app, &contacts, &app.contact_search_query);

    if filtered.is_empty() {
        let msg = if contacts.is_empty() {
            app.i18n.t("contacts.empty")
        } else {
            app.i18n.t("contacts.no_match")
        };
        let empty = Paragraph::new(msg)
            .style(Style::default().fg(app.theme.fg_secondary))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(app.i18n.t("contacts.title")),
            );
        f.render_widget(empty, chunks[1]);
        return;
    }

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(display_idx, (_, contact))| {
            let verified = if contact.verified { "✓" } else { " " };
            let trust_marker = if contact.recovery_trusted { " ★" } else { "" };
            let content = format!(
                "[{}] {}{}  ({}...)",
                verified,
                contact.display_name,
                trust_marker,
                &contact.id[..8]
            );
            let style = if display_idx == app.selected_contact {
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
        .t_args("contacts.shown", &[("count", &filtered.len().to_string())]);
    let list = List::new(items)
        .block(Block::default().title(list_title).borders(Borders::ALL))
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = ListState::default();
    state.select(Some(app.selected_contact));
    f.render_stateful_widget(list, chunks[1], &mut state);
}

pub fn draw_detail(f: &mut Frame, area: Rect, app: &App) {
    let contacts = app.backend.list_contacts().unwrap_or_default();
    let contact = contacts.get(app.selected_contact);

    match contact {
        Some(c) => {
            let fields = app
                .backend
                .get_contact_fields(app.selected_contact)
                .unwrap_or_default();

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Name
                    Constraint::Length(3), // Status
                    Constraint::Min(5),    // Fields
                    Constraint::Length(2), // Help
                ])
                .split(area);

            let name = Paragraph::new(c.display_name.clone())
                .style(Style::default().add_modifier(Modifier::BOLD))
                .block(
                    Block::default()
                        .title(app.i18n.t("contacts.name"))
                        .borders(Borders::ALL),
                );
            f.render_widget(name, chunks[0]);

            let mut status_lines = Vec::new();
            if c.verified {
                status_lines.push(Span::styled(
                    app.i18n.t("contacts.verified"),
                    Style::default().fg(app.theme.success),
                ));
            } else {
                status_lines.push(Span::styled(
                    app.i18n.t("contacts.not_verified"),
                    Style::default().fg(app.theme.warning),
                ));
            }
            if c.recovery_trusted {
                status_lines.push(Span::raw("  "));
                status_lines.push(Span::styled(
                    "★ Recovery Trusted",
                    Style::default().fg(app.theme.success),
                ));
            }
            let verified = Paragraph::new(Line::from(status_lines)).block(
                Block::default()
                    .title(app.i18n.t("contacts.status"))
                    .borders(Borders::ALL),
            );
            f.render_widget(verified, chunks[1]);

            // Fields list with selection
            if fields.is_empty() {
                let empty = Paragraph::new(app.i18n.t("contacts.no_info"))
                    .style(Style::default().fg(app.theme.fg_secondary))
                    .block(
                        Block::default()
                            .title(app.i18n.t("contacts.info"))
                            .borders(Borders::ALL),
                    );
                f.render_widget(empty, chunks[2]);
            } else {
                let items: Vec<ListItem> = fields
                    .iter()
                    .enumerate()
                    .map(|(i, field)| {
                        let action_icon = match field.action_type.as_str() {
                            "call" => "📞",
                            "sms" => "💬",
                            "email" => "✉️",
                            "web" => "🌐",
                            "map" | "directions" => "📍",
                            _ => "📋",
                        };

                        // Build trust badge from validation status
                        let (badge_text, badge_color) = app
                            .backend
                            .get_field_validation_status(&c.id, &field.label, &field.value)
                            .map(|status| match status.trust_level {
                                TrustLevel::HighConfidence => (" [verified]", Color::Green),
                                TrustLevel::PartialConfidence => (" [partial]", Color::LightGreen),
                                TrustLevel::LowConfidence => (" [low]", Color::Yellow),
                                TrustLevel::Unverified => (" [unverified]", Color::DarkGray),
                            })
                            .unwrap_or((" [unverified]", Color::DarkGray));

                        let base_style = if i == app.selected_contact_field {
                            Style::default()
                                .fg(app.theme.warning)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        };

                        let line = Line::from(vec![
                            Span::styled(
                                format!("{} {}: {}", action_icon, field.label, field.value),
                                base_style,
                            ),
                            Span::styled(badge_text, Style::default().fg(badge_color)),
                        ]);

                        ListItem::new(line)
                    })
                    .collect();

                let list = List::new(items)
                    .block(
                        Block::default()
                            .title(app.i18n.t("contacts.info"))
                            .borders(Borders::ALL),
                    )
                    .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

                let mut state = ListState::default();
                state.select(Some(app.selected_contact_field));
                f.render_stateful_widget(list, chunks[2], &mut state);
            }

            // Help line
            let help = Paragraph::new(
                "t=trust  v=visibility  V=validate  R=revoke  x=delete  o/Enter=open  Esc=back",
            )
            .style(Style::default().fg(app.theme.fg_secondary));
            f.render_widget(help, chunks[3]);
        }
        None => {
            let empty = Paragraph::new(app.i18n.t("contacts.not_found"))
                .style(Style::default().fg(app.theme.error));
            f.render_widget(empty, area);
        }
    }
}
