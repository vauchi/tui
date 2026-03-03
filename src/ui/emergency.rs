// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Emergency Broadcast Screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, EmergencyFocus};

/// Renders the emergency access screen.
pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Status
            Constraint::Length(3), // Trusted contacts
            Constraint::Length(3), // Message
            Constraint::Length(3), // Location toggle
            Constraint::Min(1),    // Help
        ])
        .split(area);

    // Status
    let status_text = if app.emergency_state.configured {
        format!(
            "{} ({} {})",
            app.i18n.t("emergency.configure"),
            app.emergency_state.trusted_count,
            app.i18n.t("emergency.trusted_contacts"),
        )
    } else {
        app.i18n.t("emergency.description")
    };
    let status_style = if app.emergency_state.configured {
        Style::default().fg(app.theme.success)
    } else {
        Style::default().fg(app.theme.fg_secondary)
    };
    let status = Paragraph::new(status_text).style(status_style).block(
        Block::default()
            .title(app.i18n.t("emergency.title"))
            .borders(Borders::ALL),
    );
    f.render_widget(status, chunks[0]);

    // Trusted contacts input
    let contacts_style = if app.emergency_state.focus == EmergencyFocus::ContactIds {
        Style::default().fg(app.theme.warning)
    } else {
        Style::default()
    };
    let contacts = Paragraph::new(app.emergency_state.contact_ids_input.clone())
        .style(contacts_style)
        .block(
            Block::default()
                .title(app.i18n.t("emergency.trusted_contacts"))
                .borders(Borders::ALL),
        );
    f.render_widget(contacts, chunks[1]);

    // Message input
    let message_style = if app.emergency_state.focus == EmergencyFocus::Message {
        Style::default().fg(app.theme.warning)
    } else {
        Style::default()
    };
    let message = Paragraph::new(app.emergency_state.message_input.clone())
        .style(message_style)
        .block(
            Block::default()
                .title(app.i18n.t("emergency.message_label"))
                .borders(Borders::ALL),
        );
    f.render_widget(message, chunks[2]);

    // Location toggle
    let location_text = if app.emergency_state.include_location {
        format!("{}: ON", app.i18n.t("emergency.include_location"))
    } else {
        format!("{}: OFF", app.i18n.t("emergency.include_location"))
    };
    let location_style = if app.emergency_state.include_location {
        Style::default().fg(app.theme.success)
    } else {
        Style::default().fg(app.theme.fg_secondary)
    };
    let location = Paragraph::new(location_text)
        .style(location_style)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(location, chunks[3]);

    // Confirmation overlay
    if app.emergency_state.focus == EmergencyFocus::Confirm {
        let confirm_text = format!(
            "Send alert to {} contacts? [y] yes  [n/Esc] cancel",
            app.emergency_state.trusted_count
        );
        let confirm = Paragraph::new(confirm_text)
            .style(
                Style::default()
                    .fg(app.theme.warning)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .title(" Confirm ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(app.theme.warning)),
            );
        f.render_widget(confirm, chunks[4]);
        return;
    }

    // Help text
    let help_text = if app.emergency_state.configured {
        match app.emergency_state.focus {
            EmergencyFocus::Status => {
                "[s] send  [c] configure  [l] toggle location  [x] disable  [Esc] back"
            }
            EmergencyFocus::ContactIds => {
                "Enter contact IDs (comma-separated, max 10)  [Tab/Enter] next  [Esc] cancel"
            }
            EmergencyFocus::Message => "Enter alert message  [Enter] save  [Esc] cancel",
            EmergencyFocus::Confirm => unreachable!(),
        }
    } else {
        match app.emergency_state.focus {
            EmergencyFocus::Status => "[c] configure  [Esc] back",
            EmergencyFocus::ContactIds => {
                "Enter contact IDs (comma-separated, max 10)  [Tab/Enter] next  [Esc] cancel"
            }
            EmergencyFocus::Message => "Enter alert message  [Enter] save  [Esc] cancel",
            EmergencyFocus::Confirm => unreachable!(),
        }
    };
    let help = Paragraph::new(help_text).style(Style::default().fg(app.theme.fg_secondary));
    f.render_widget(help, chunks[4]);
}
