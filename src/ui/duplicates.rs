// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Duplicate Detection, Merge Preview, and Contact Limit Screens (SP-12a)

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::app::App;

/// Renders the duplicates list screen.
pub fn draw_duplicates(f: &mut Frame, area: Rect, app: &App) {
    let pairs = &app.duplicates_state.pairs;

    if pairs.is_empty() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(area);

        let header = Paragraph::new("No duplicate contacts detected.")
            .style(Style::default().fg(app.theme.fg_secondary))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Duplicate Detection "),
            );
        f.render_widget(header, chunks[0]);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(5),    // Duplicate pairs list
            Constraint::Length(2), // Help
        ])
        .split(area);

    let header = Paragraph::new(format!("{} potential duplicate(s) found", pairs.len()))
        .style(
            Style::default()
                .fg(app.theme.warning)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Duplicate Detection "),
        );
    f.render_widget(header, chunks[0]);

    let items: Vec<ListItem> = pairs
        .iter()
        .enumerate()
        .map(|(i, pair)| {
            let similarity_pct = (pair.similarity * 100.0) as u8;
            let content = format!(
                "  {}  <->  {}  ({}% similar)",
                pair.name1, pair.name2, similarity_pct
            );
            let style = if i == app.duplicates_state.selected {
                Style::default()
                    .fg(app.theme.warning)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Potential Duplicates "),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = ListState::default();
    state.select(Some(app.duplicates_state.selected));
    f.render_stateful_widget(list, chunks[1], &mut state);

    let help = Paragraph::new("[j/k] navigate  [m]erge  [d]ismiss  [esc] back")
        .style(Style::default().fg(app.theme.fg_secondary));
    f.render_widget(help, chunks[2]);
}

/// Renders the merge preview screen (side-by-side comparison).
pub fn draw_merge(f: &mut Frame, area: Rect, app: &App) {
    let state = &app.merge_state;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Min(5),    // Side-by-side fields
            Constraint::Length(3), // Confirmation
        ])
        .split(area);

    let title = Paragraph::new(format!(
        "Merge: {} (primary) <- {} (secondary)",
        state.primary_name, state.secondary_name
    ))
    .style(
        Style::default()
            .fg(app.theme.accent)
            .add_modifier(Modifier::BOLD),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Merge Preview "),
    );
    f.render_widget(title, chunks[0]);

    // Side-by-side comparison
    let halves = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    // Primary contact fields
    let primary_text: Vec<Line> = if state.primary_fields.is_empty() {
        vec![Line::from(Span::styled(
            "(no fields)",
            Style::default().fg(app.theme.fg_secondary),
        ))]
    } else {
        state
            .primary_fields
            .iter()
            .map(|f| Line::from(format!("  {}", f)))
            .collect()
    };

    let primary_para = Paragraph::new(primary_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.success))
            .title(format!(" {} (keep) ", state.primary_name)),
    );
    f.render_widget(primary_para, halves[0]);

    // Secondary contact fields
    let secondary_text: Vec<Line> = if state.secondary_fields.is_empty() {
        vec![Line::from(Span::styled(
            "(no fields)",
            Style::default().fg(app.theme.fg_secondary),
        ))]
    } else {
        state
            .secondary_fields
            .iter()
            .map(|f| Line::from(format!("  {}", f)))
            .collect()
    };

    let secondary_para = Paragraph::new(secondary_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.error))
            .title(format!(" {} (remove) ", state.secondary_name)),
    );
    f.render_widget(secondary_para, halves[1]);

    // Confirmation
    let confirm = Paragraph::new(
        "Unique fields from the secondary will be added to the primary. The secondary will be deleted.",
    )
    .style(Style::default().fg(app.theme.warning))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" [y] confirm merge  [n/Esc] cancel "),
    );
    f.render_widget(confirm, chunks[2]);
}

/// Renders the contact limit screen.
pub fn draw_limit(f: &mut Frame, area: Rect, app: &App) {
    let state = &app.contact_limit_state;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Spacer
            Constraint::Length(5), // Info card
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Current count
            Constraint::Length(3), // Limit input
            Constraint::Length(3), // Help
            Constraint::Min(0),    // Spacer
        ])
        .split(area);

    let info_text = vec![
        Line::from(Span::styled(
            "Contact Limit",
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Set a maximum number of contacts to manage storage."),
    ];

    let info = Paragraph::new(info_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.accent))
                .title(" Contact Limit "),
        );
    f.render_widget(info, chunks[1]);

    // Current count
    let usage_ratio = if state.current_limit > 0 {
        format!(
            "{} / {} contacts ({:.0}%)",
            state.current_count,
            state.current_limit,
            (state.current_count as f64 / state.current_limit as f64) * 100.0
        )
    } else {
        format!("{} contacts (no limit)", state.current_count)
    };

    let count_color = if state.current_count >= state.current_limit {
        app.theme.error
    } else if state.current_count as f64 >= state.current_limit as f64 * 0.8 {
        app.theme.warning
    } else {
        app.theme.success
    };

    let count_para = Paragraph::new(usage_ratio)
        .style(Style::default().fg(count_color))
        .block(Block::default().borders(Borders::ALL).title(" Usage "));
    f.render_widget(count_para, chunks[3]);

    // Limit input
    let limit_display = if state.editing {
        format!("{}|", state.limit_input)
    } else {
        state.current_limit.to_string()
    };

    let limit_style = if state.editing {
        Style::default().fg(app.theme.warning)
    } else {
        Style::default()
    };

    let limit_para = Paragraph::new(limit_display).style(limit_style).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(if state.editing {
                Style::default().fg(app.theme.warning)
            } else {
                Style::default()
            })
            .title(" Max Contacts "),
    );
    f.render_widget(limit_para, chunks[4]);

    let help_text = if state.editing {
        "[Enter] save  [Esc] cancel"
    } else {
        "[e/Enter] edit  [Esc] back"
    };
    let help = Paragraph::new(help_text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(app.theme.fg_secondary));
    f.render_widget(help, chunks[5]);
}
