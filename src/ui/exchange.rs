// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Exchange Screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;

pub fn draw(f: &mut Frame, area: Rect, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Instructions
            Constraint::Length(3), // Timer
            Constraint::Min(0),    // QR code
        ])
        .split(area);

    let instructions = Paragraph::new(app.i18n.t("exchange.instruction"))
        .style(Style::default().fg(app.theme.fg_secondary))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(instructions, chunks[0]);

    // Ensure we have a QR code generated
    if app.current_qr.is_none() {
        if let Ok(qr_data) = app.backend.generate_exchange_qr() {
            app.current_qr = Some(qr_data);
        }
    }

    // Check expiration and show timer
    if let Some(ref qr_data) = app.current_qr {
        let remaining = qr_data.remaining_secs();
        let (timer_text, timer_style) = if remaining == 0 {
            (
                app.i18n.t("exchange.expired"),
                Style::default().fg(app.theme.error),
            )
        } else {
            let mins = remaining / 60;
            let secs = remaining % 60;
            let style = if remaining <= 60 {
                Style::default().fg(app.theme.warning)
            } else {
                Style::default().fg(app.theme.success)
            };
            (
                app.i18n.t_args(
                    "exchange.expires_in",
                    &[("time", &format!("{}:{:02}", mins, secs))],
                ),
                style,
            )
        };
        let timer = Paragraph::new(timer_text)
            .style(timer_style)
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::NONE));
        f.render_widget(timer, chunks[1]);

        // Display QR code
        if remaining > 0 {
            let qr_display = render_qr_ascii(&qr_data.data);
            let qr = Paragraph::new(qr_display)
                .style(Style::default().fg(app.theme.fg))
                .block(
                    Block::default()
                        .title(app.i18n.t("exchange.your_qr"))
                        .borders(Borders::ALL),
                );
            f.render_widget(qr, chunks[2]);
        } else {
            let expired = Paragraph::new(app.i18n.t("exchange.expired_hint"))
                .style(Style::default().fg(app.theme.fg_secondary))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .title(app.i18n.t("exchange.qr_code"))
                        .borders(Borders::ALL),
                );
            f.render_widget(expired, chunks[2]);
        }
    } else {
        // Error generating QR
        let error = Paragraph::new(app.i18n.t("exchange.qr_error"))
            .style(Style::default().fg(app.theme.error))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(error, chunks[2]);
    }
}

/// Regenerate the exchange QR code.
pub fn regenerate_qr(app: &mut App) {
    if let Ok(qr_data) = app.backend.generate_exchange_qr() {
        app.current_qr = Some(qr_data);
        app.set_status(app.i18n.t("exchange.refreshed"));
    }
}

/// Render a QR code as ASCII art.
fn render_qr_ascii(data: &str) -> String {
    use qrcode::QrCode;

    match QrCode::new(data) {
        Ok(code) => code
            .render()
            .dark_color('█')
            .light_color(' ')
            .quiet_zone(true)
            .build(),
        Err(_) => "Failed to generate QR code".to_string(),
    }
}
