// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tor Privacy Settings Screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::App;

/// Draw the Tor settings screen.
pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Tor status
            Constraint::Length(6), // Configuration
            Constraint::Min(0),    // Options / bridges
        ])
        .split(area);

    // Load current Tor state
    let tor = &app.tor_state;

    // Tor status block
    let status_style = if tor.enabled {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let status_text = if tor.enabled { "ENABLED" } else { "DISABLED" };

    let status_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Tor Mode:       "),
            Span::styled(status_text, status_style),
        ]),
        Line::from(vec![
            Span::raw("  Prefer .onion:  "),
            Span::styled(
                if tor.prefer_onion { "yes" } else { "no" },
                Style::default().fg(Color::Yellow),
            ),
        ]),
        Line::from(""),
    ];

    let status = Paragraph::new(status_lines)
        .block(Block::default().borders(Borders::ALL).title("Tor Status"));
    f.render_widget(status, chunks[0]);

    // Configuration block
    let config_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("  Circuit rotation:  "),
            Span::styled(
                format!("{}s", tor.circuit_rotation_secs),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::raw("  Bridges:           "),
            Span::styled(
                if tor.bridge_count > 0 {
                    format!("{} configured", tor.bridge_count)
                } else {
                    "none".to_string()
                },
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(""),
    ];

    let config = Paragraph::new(config_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Configuration"),
    );
    f.render_widget(config, chunks[1]);

    // Options
    let mut items = vec![
        ListItem::new(""),
        ListItem::new(Span::styled(
            "  Actions:",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        ListItem::new(""),
    ];

    if tor.enabled {
        items.push(ListItem::new(Span::styled(
            "  [d] Disable Tor mode",
            Style::default().fg(Color::Yellow),
        )));
    } else {
        items.push(ListItem::new(Span::styled(
            "  [e] Enable Tor mode",
            Style::default().fg(Color::Green),
        )));
    }

    items.push(ListItem::new(Span::styled(
        "  [o] Toggle .onion preference",
        Style::default().fg(Color::Yellow),
    )));

    if tor.enabled {
        items.push(ListItem::new(Span::styled(
            "  [n] Request new circuit",
            Style::default().fg(Color::Yellow),
        )));
    }

    items.push(ListItem::new(""));
    items.push(ListItem::new(Span::styled(
        "  Bridges:",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    items.push(ListItem::new(Span::styled(
        "  [x] Clear all bridges",
        Style::default().fg(Color::DarkGray),
    )));

    items.push(ListItem::new(""));
    items.push(ListItem::new(Span::styled(
        "  Use CLI for bridge management: vauchi tor bridges add <addr>",
        Style::default().fg(Color::DarkGray),
    )));

    let options = List::new(items).block(Block::default().borders(Borders::ALL).title("Options"));
    f.render_widget(options, chunks[2]);
}
