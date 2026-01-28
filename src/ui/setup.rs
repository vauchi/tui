// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Setup Screen UI
//!
//! Shown when no identity exists. Guides user to create or import an identity.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

/// Draw the setup screen.
pub fn draw(f: &mut Frame, area: Rect, _app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Spacer
            Constraint::Length(7), // Welcome box
            Constraint::Length(2), // Spacer
            Constraint::Length(5), // Create option
            Constraint::Length(5), // Import option
            Constraint::Min(0),    // Spacer
        ])
        .split(area);

    // Welcome message
    let welcome_text = vec![
        Line::from(Span::styled(
            "Welcome to Vauchi!",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Privacy-focused updatable contact cards."),
        Line::from(""),
        Line::from(Span::styled(
            "You need to set up an identity to continue.",
            Style::default().fg(Color::Yellow),
        )),
    ];

    let welcome = Paragraph::new(welcome_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Setup Required "),
        );
    f.render_widget(welcome, chunks[1]);

    // Create new identity option
    let create_text = vec![
        Line::from(Span::styled(
            "[c] Create New Identity",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Start fresh with a new Vauchi identity",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let create = Paragraph::new(create_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(create, chunks[3]);

    // Import backup option
    let import_text = vec![
        Line::from(Span::styled(
            "[i] Import Backup",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Restore from an existing backup",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let import = Paragraph::new(import_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(import, chunks[4]);
}
