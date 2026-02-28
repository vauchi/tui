// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Delivery Status Screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::App;

/// Draw the delivery status screen.
pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Delivery counts
            Constraint::Min(0),     // Status / actions log
        ])
        .split(area);

    // Delivery record counts
    let state = &app.delivery_state;

    let count_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Queued:     "),
            Span::styled(state.queued.to_string(), Style::default().fg(app.theme.fg)),
        ]),
        Line::from(vec![
            Span::raw("  Sent:       "),
            Span::styled(state.sent.to_string(), Style::default().fg(app.theme.fg)),
        ]),
        Line::from(vec![
            Span::raw("  Stored:     "),
            Span::styled(
                state.stored.to_string(),
                Style::default().fg(app.theme.accent),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Delivered:  "),
            Span::styled(
                state.delivered.to_string(),
                Style::default().fg(app.theme.success),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Failed:     "),
            Span::styled(
                state.failed.to_string(),
                if state.failed > 0 {
                    Style::default().fg(app.theme.error)
                } else {
                    Style::default().fg(app.theme.fg_secondary)
                },
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("  Pending retries:  "),
            Span::styled(
                state.pending_retries.to_string(),
                Style::default().fg(app.theme.fg_secondary),
            ),
            Span::raw("    Offline queue:  "),
            Span::styled(
                state.offline_queue_depth.to_string(),
                Style::default().fg(app.theme.fg_secondary),
            ),
        ]),
    ];

    let counts = Paragraph::new(count_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Delivery Status "),
    );
    f.render_widget(counts, chunks[0]);

    // Status / actions
    let mut items: Vec<ListItem> = Vec::new();

    if let Some(ref result) = state.last_result {
        items.push(ListItem::new(Line::from(vec![
            Span::styled("  Last action: ", Style::default().fg(app.theme.accent)),
            Span::raw(result.as_str()),
        ])));
        items.push(ListItem::new(""));
    }

    items.push(ListItem::new(Span::styled(
        "  Actions:",
        Style::default()
            .fg(app.theme.accent)
            .add_modifier(Modifier::BOLD),
    )));
    items.push(ListItem::new("  [r] Process due retries"));
    items.push(ListItem::new("  [c] Run cleanup (expire old records)"));
    items.push(ListItem::new("  [Esc] Back to home"));

    let actions = List::new(items).block(Block::default().borders(Borders::ALL).title(" Actions "));
    f.render_widget(actions, chunks[1]);
}
