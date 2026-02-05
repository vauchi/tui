// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Recovery Screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

/// Draw the recovery screen.
pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(9), // Status
            Constraint::Length(8), // Actions
            Constraint::Min(0),    // Info
            Constraint::Length(3), // Key hints
        ])
        .split(area);

    // Recovery status - dynamic from backend
    draw_status_section(f, chunks[0], app);

    // Recovery actions
    draw_actions_section(f, chunks[1], app);

    // Recovery info
    draw_info_section(f, chunks[2], app);

    // Key hints
    draw_key_hints(f, chunks[3]);
}

fn draw_status_section(f: &mut Frame, area: Rect, app: &App) {
    let status_result = app.backend.get_recovery_status();

    let status_text = match status_result {
        Ok(status) => {
            if status.has_active_claim {
                let progress = if status.required_vouchers > 0 {
                    (status.voucher_count as f64 / status.required_vouchers as f64).min(1.0)
                } else {
                    0.0
                };

                let progress_color = if progress >= 1.0 {
                    Color::Green
                } else if progress >= 0.5 {
                    Color::Yellow
                } else {
                    Color::Red
                };

                let expiry_text = status
                    .claim_expires
                    .clone()
                    .unwrap_or_else(|| app.i18n.t("recovery.no_expiration"));

                vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        app.i18n.t("recovery.claim_active"),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(vec![
                        Span::raw(format!("  {}: ", app.i18n.t("recovery.vouchers"))),
                        Span::styled(
                            format!("{}/{}", status.voucher_count, status.required_vouchers),
                            Style::default().fg(progress_color),
                        ),
                    ]),
                    Line::from(vec![
                        Span::raw("  Expires: "),
                        Span::styled(expiry_text, Style::default().fg(Color::DarkGray)),
                    ]),
                    Line::from(""),
                ]
            } else {
                vec![
                    Line::from(""),
                    Line::from(Span::styled(
                        app.i18n.t("recovery.title"),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("  {}", app.i18n.t("recovery.no_active_claim")),
                        Style::default().fg(Color::DarkGray),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("  {}", app.i18n.t("recovery.create_hint")),
                        Style::default().fg(Color::DarkGray),
                    )),
                    Line::from(""),
                ]
            }
        }
        Err(_) => {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    app.i18n.t("recovery.title"),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {}", app.i18n.t("recovery.load_error")),
                    Style::default().fg(Color::Red),
                )),
                Line::from(""),
            ]
        }
    };

    let status = Paragraph::new(status_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(app.i18n.t("sync.status")),
    );
    f.render_widget(status, area);
}

fn draw_actions_section(f: &mut Frame, area: Rect, app: &App) {
    let has_identity = app.backend.has_identity();

    let actions_text = if has_identity {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled("  [c]", Style::default().fg(Color::Yellow)),
                Span::raw(format!(" {}", app.i18n.t("recovery.create_claim"))),
            ]),
            Line::from(vec![
                Span::styled("  [v]", Style::default().fg(Color::Yellow)),
                Span::raw(format!(" {}", app.i18n.t("recovery.vouch"))),
            ]),
            Line::from(vec![
                Span::styled("  [s]", Style::default().fg(Color::Yellow)),
                Span::raw(format!(" {}", app.i18n.t("recovery.check_status"))),
            ]),
            Line::from(""),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  {}", app.i18n.t("devices.no_identity")),
                Style::default().fg(Color::Yellow),
            )),
            Line::from(""),
            Line::from(Span::styled(
                format!("  {}", app.i18n.t("devices.create_first")),
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
        ]
    };

    let actions = Paragraph::new(actions_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(app.i18n.t("devices.actions")),
    );
    f.render_widget(actions, area);
}

fn draw_info_section(f: &mut Frame, area: Rect, app: &App) {
    let info_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            app.i18n.t("recovery.how_it_works"),
            Style::default().fg(Color::Cyan),
        )),
        Line::from(""),
        Line::from(format!("  1. {}", app.i18n.t("recovery.step1"))),
        Line::from(format!("  2. {}", app.i18n.t("recovery.step2"))),
        Line::from(format!("  3. {}", app.i18n.t("recovery.step3"))),
        Line::from(format!("  4. {}", app.i18n.t("recovery.step4"))),
        Line::from(format!("  5. {}", app.i18n.t("recovery.step5"))),
        Line::from(""),
        Line::from(Span::styled(
            app.i18n.t("recovery.note"),
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            "Use CLI for detailed recovery workflow: vauchi recovery --help",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
    ];

    let info = Paragraph::new(info_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(app.i18n.t("recovery.information")),
    );
    f.render_widget(info, area);
}

fn draw_key_hints(f: &mut Frame, area: Rect) {
    let hints = Paragraph::new(Line::from(vec![
        Span::styled("c", Style::default().fg(Color::Yellow)),
        Span::raw(" Claim  "),
        Span::styled("v", Style::default().fg(Color::Yellow)),
        Span::raw(" Vouch  "),
        Span::styled("s", Style::default().fg(Color::Yellow)),
        Span::raw(" Status  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" Back"),
    ]))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::TOP));

    f.render_widget(hints, area);
}
