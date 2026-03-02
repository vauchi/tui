// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Duress PIN and Alert Configuration Screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, DuressFocus, InputMode};

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Status
            Constraint::Length(3), // PIN setup
            Constraint::Length(3), // Trusted contacts
            Constraint::Length(3), // Message
            Constraint::Length(3), // Location toggle
            Constraint::Min(1),    // Help
        ])
        .split(area);

    // Status
    let status_text = if !app.duress_state.password_enabled {
        "App password required — set one in Settings first".to_string()
    } else if app.duress_state.enabled {
        let alert_info = if app.duress_state.alert_contact_count > 0 {
            format!(
                " — {} alert contact(s) configured",
                app.duress_state.alert_contact_count
            )
        } else {
            " — no alert contacts configured".to_string()
        };
        format!("Duress PIN: ENABLED{}", alert_info)
    } else {
        "Duress PIN: NOT SET".to_string()
    };
    let status_style = if !app.duress_state.password_enabled {
        Style::default().fg(app.theme.error)
    } else if app.duress_state.enabled {
        Style::default().fg(app.theme.success)
    } else {
        Style::default().fg(app.theme.fg_secondary)
    };
    let status = Paragraph::new(status_text).style(status_style).block(
        Block::default()
            .title("Duress Protection")
            .borders(Borders::ALL),
    );
    f.render_widget(status, chunks[0]);

    // PIN setup input
    let pin_style = if app.duress_state.focus == DuressFocus::PinSetup {
        Style::default().fg(app.theme.warning)
    } else {
        Style::default().fg(app.theme.fg_secondary)
    };
    let pin_display = if app.duress_state.focus == DuressFocus::PinSetup {
        let masked: String = "*".repeat(app.duress_state.pin_input.len());
        if app.input_mode == InputMode::Editing {
            format!("{}|", masked)
        } else {
            masked
        }
    } else if app.duress_state.enabled {
        "****".to_string()
    } else {
        "Not configured".to_string()
    };
    let pin = Paragraph::new(pin_display)
        .style(pin_style)
        .block(Block::default().title("Duress PIN").borders(Borders::ALL));
    f.render_widget(pin, chunks[1]);

    // Trusted contacts input (alert settings)
    let contacts_style = if app.duress_state.focus == DuressFocus::ContactIds {
        Style::default().fg(app.theme.warning)
    } else {
        Style::default().fg(app.theme.fg_secondary)
    };
    let contacts = Paragraph::new(app.duress_state.contact_ids_input.clone())
        .style(contacts_style)
        .block(
            Block::default()
                .title("Alert Contacts (comma-separated IDs)")
                .borders(Borders::ALL),
        );
    f.render_widget(contacts, chunks[2]);

    // Message input
    let message_style = if app.duress_state.focus == DuressFocus::Message {
        Style::default().fg(app.theme.warning)
    } else {
        Style::default().fg(app.theme.fg_secondary)
    };
    let message = Paragraph::new(app.duress_state.message_input.clone())
        .style(message_style)
        .block(
            Block::default()
                .title("Alert Message")
                .borders(Borders::ALL),
        );
    f.render_widget(message, chunks[3]);

    // Location toggle
    let location_text = if app.duress_state.include_location {
        "Include Location: ON"
    } else {
        "Include Location: OFF"
    };
    let location_style = if app.duress_state.include_location {
        Style::default().fg(app.theme.success)
    } else {
        Style::default().fg(app.theme.fg_secondary)
    };
    let location = Paragraph::new(location_text)
        .style(location_style)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(location, chunks[4]);

    // Help text
    let help_text = match app.duress_state.focus {
        DuressFocus::Status => {
            if !app.duress_state.password_enabled {
                "[Esc] back"
            } else if app.duress_state.enabled {
                "[a] configure alerts  [l] toggle location  [x] disable  [Esc] back"
            } else {
                "[p] set duress PIN  [Esc] back"
            }
        }
        DuressFocus::PinSetup => {
            "Enter duress PIN (must differ from app password)  [Enter] save  [Esc] cancel"
        }
        DuressFocus::ContactIds => {
            "Enter contact IDs (comma-separated)  [Tab/Enter] next  [Esc] cancel"
        }
        DuressFocus::Message => "Enter alert message  [Enter] save  [Esc] cancel",
    };
    let help = Paragraph::new(help_text).style(Style::default().fg(app.theme.fg_secondary));
    f.render_widget(help, chunks[5]);
}
