// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Setup Screen UI
//!
//! Shown when no identity exists. Guides user to create or import an identity.
//! SP-21: Onboarding wizard steps are now rendered via OnboardingEngine.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

/// Draw the legacy setup screen (shown when no identity exists).
pub fn draw(f: &mut Frame, area: Rect, app: &App) {
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
            app.i18n.t("welcome.title"),
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(app.i18n.t("welcome.subtitle")),
        Line::from(""),
        Line::from(Span::styled(
            app.i18n.t("setup.identity_required"),
            Style::default().fg(app.theme.warning),
        )),
    ];

    let welcome = Paragraph::new(welcome_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.accent))
                .title(format!(" {} ", app.i18n.t("setup.title"))),
        );
    f.render_widget(welcome, chunks[1]);

    // Create new identity option
    let create_text = vec![
        Line::from(Span::styled(
            format!("[c] {}", app.i18n.t("setup.create")),
            Style::default()
                .fg(app.theme.success)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            app.i18n.t("setup.create_description"),
            Style::default().fg(app.theme.fg_secondary),
        )),
    ];

    let create = Paragraph::new(create_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(create, chunks[3]);

    // Import backup option
    let import_text = vec![
        Line::from(Span::styled(
            format!("[i] {}", app.i18n.t("setup.import")),
            Style::default()
                .fg(app.theme.warning)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            app.i18n.t("setup.import_description"),
            Style::default().fg(app.theme.fg_secondary),
        )),
    ];

    let import = Paragraph::new(import_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(import, chunks[4]);
}
