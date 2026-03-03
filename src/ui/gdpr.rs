// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Privacy & GDPR Screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::app::App;

/// Renders the GDPR/privacy screen with data export and deletion options.
pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Data Export section
            Constraint::Length(5), // Deletion section
            Constraint::Min(0),    // Consent section
        ])
        .margin(1)
        .split(area);

    // Data Export section
    let export_para = Paragraph::new(
        "Export all your personal data as JSON.\n\
         Press [e] to export.",
    )
    .style(if app.privacy_state.selected_item == 0 {
        Style::default().fg(app.theme.warning)
    } else {
        Style::default().fg(app.theme.fg)
    })
    .block(
        Block::default()
            .title("Data Export (GDPR Art. 20)")
            .borders(Borders::ALL),
    );
    f.render_widget(export_para, chunks[0]);

    // Deletion section
    let deletion_status = app
        .backend
        .get_deletion_status()
        .unwrap_or_else(|_| "Unknown".to_string());
    let deletion_text = format!(
        "Status: {}\n\
         [d] schedule  [c] cancel  [x] execute  [!] panic shred",
        deletion_status
    );
    let deletion_para = Paragraph::new(deletion_text)
        .style(if app.privacy_state.selected_item == 1 {
            Style::default().fg(app.theme.warning)
        } else {
            Style::default().fg(app.theme.fg)
        })
        .block(
            Block::default()
                .title("Account Deletion (GDPR Art. 17)")
                .borders(Borders::ALL),
        );
    f.render_widget(deletion_para, chunks[1]);

    // Consent section
    let consent_items = build_consent_items(app);
    let consent_list = List::new(consent_items)
        .block(
            Block::default()
                .title("Consent Management (GDPR Art. 7)")
                .borders(Borders::ALL),
        )
        .highlight_style(Style::default().fg(app.theme.warning));
    f.render_widget(consent_list, chunks[2]);
}

fn build_consent_items(app: &App) -> Vec<ListItem<'static>> {
    let consent_types = [
        (
            vauchi_core::api::ConsentType::DataProcessing,
            "Data Processing",
            "Required for local operation",
        ),
        (
            vauchi_core::api::ConsentType::ContactSharing,
            "Contact Sharing",
            "Share info with exchanged contacts",
        ),
        (
            vauchi_core::api::ConsentType::Analytics,
            "Analytics",
            "Anonymous usage analytics",
        ),
        (
            vauchi_core::api::ConsentType::RecoveryVouching,
            "Recovery Vouching",
            "Participate in recovery",
        ),
    ];

    consent_types
        .iter()
        .enumerate()
        .map(|(i, (consent_type, name, desc))| {
            let granted = app
                .backend
                .consent_status_for_type(consent_type)
                .map(|s| s.granted)
                .unwrap_or(false);

            let status = if granted { "[x]" } else { "[ ]" };
            let selected = app.privacy_state.selected_item == i + 2;
            let style = if selected {
                Style::default().fg(app.theme.warning)
            } else {
                Style::default().fg(app.theme.fg)
            };

            ListItem::new(format!("  {} {} — {}", status, name, desc)).style(style)
        })
        .collect()
}
