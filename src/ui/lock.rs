// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Lock Screen UI
//!
//! Shown on startup when an app password is configured. Accepts PIN entry
//! with masked display. No visual indication of duress mode existence.
//!
//! Feature: duress_pin.feature @unlock

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

/// Draw the lock screen.
pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(2),    // Top spacer
            Constraint::Length(5), // Lock icon / title
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // PIN input
            Constraint::Length(2), // Error / attempts
            Constraint::Min(2),    // Bottom spacer
        ])
        .split(area);

    // Title
    let title = Paragraph::new(vec![
        Line::from(Span::styled(
            "Vauchi is locked",
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Enter your PIN to unlock"),
    ])
    .alignment(Alignment::Center)
    .block(Block::default());
    f.render_widget(title, chunks[1]);

    // PIN input (masked with dots)
    let masked: String = "*".repeat(app.lock_state.pin_input.len());
    let pin_display = if masked.is_empty() {
        "Enter PIN...".to_string()
    } else {
        masked
    };

    let input_style = if app.lock_state.error {
        Style::default().fg(app.theme.error)
    } else {
        Style::default().fg(app.theme.fg)
    };

    let pin_input = Paragraph::new(Line::from(Span::styled(pin_display, input_style)))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(if app.lock_state.error {
                    Style::default().fg(app.theme.error)
                } else {
                    Style::default().fg(app.theme.border)
                })
                .title(" PIN "),
        );
    f.render_widget(pin_input, chunks[3]);

    // Error message / attempt count
    if app.lock_state.error {
        let msg = format!("Invalid PIN (attempt {})", app.lock_state.attempts);
        let error = Paragraph::new(Span::styled(msg, Style::default().fg(app.theme.error)))
            .alignment(Alignment::Center);
        f.render_widget(error, chunks[4]);
    }
}
