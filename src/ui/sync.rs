// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Sync Status Screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Gauge, List, ListItem, Paragraph};

use crate::app::App;

/// Draw the sync screen.
pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // Connection status + Tor indicator
            Constraint::Length(5), // Sync progress
            Constraint::Min(0),    // Sync log
        ])
        .split(area);

    // Connection status - use real values from backend
    let relay_url = app.backend.relay_url();
    let sync_state = &app.sync_state;

    let status_style = if sync_state.connected {
        Style::default().fg(app.theme.success)
    } else if sync_state.is_syncing {
        Style::default().fg(app.theme.warning)
    } else {
        Style::default().fg(app.theme.fg_secondary)
    };

    let status_text = if sync_state.is_syncing {
        app.i18n.t("sync.syncing")
    } else if sync_state.connected {
        app.i18n.t("sync.connected")
    } else {
        app.i18n.t("sync.disconnected")
    };

    // Tor status indicator
    let tor_state = &app.tor_state;
    let tor_indicator = if tor_state.enabled {
        Span::styled("Tor: ON", Style::default().fg(app.theme.success))
    } else {
        Span::styled("Tor: OFF", Style::default().fg(app.theme.fg_secondary))
    };

    let status_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw(format!("  {}: ", app.i18n.t("sync.relay"))),
            Span::styled(relay_url, Style::default().fg(app.theme.accent)),
        ]),
        Line::from(vec![
            Span::raw(format!("  {}: ", app.i18n.t("sync.status"))),
            Span::styled(status_text, status_style),
        ]),
        Line::from(vec![
            Span::raw("  Privacy: "),
            tor_indicator,
            Span::raw(if tor_state.enabled && tor_state.prefer_onion {
                "  (.onion preferred)"
            } else {
                ""
            }),
        ]),
        Line::from(""),
    ];

    let status = Paragraph::new(status_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(app.i18n.t("sync.connection")),
    );
    f.render_widget(status, chunks[0]);

    // Sync progress
    let contact_count = app.backend.contact_count().unwrap_or(0);
    let pending_updates = sync_state.pending_updates;

    let progress_label = if pending_updates > 0 {
        app.i18n.t_args(
            "sync.pending",
            &[
                ("contacts", &contact_count.to_string()),
                ("pending", &pending_updates.to_string()),
            ],
        )
    } else {
        app.i18n
            .t_args("sync.synced", &[("contacts", &contact_count.to_string())])
    };

    // Calculate progress ratio
    let progress_ratio = if sync_state.is_syncing {
        0.5 // Show partial progress while syncing
    } else if pending_updates > 0 {
        0.7 // Mostly synced but has pending
    } else {
        1.0 // Fully synced
    };

    let gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(app.i18n.t("sync.progress")),
        )
        .gauge_style(Style::default().fg(app.theme.accent))
        .ratio(progress_ratio)
        .label(progress_label);

    f.render_widget(gauge, chunks[1]);

    // Sync log / instructions
    let mut log_items: Vec<ListItem> = Vec::new();

    // Add last result if available
    if let Some(ref result) = sync_state.last_result {
        log_items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("  {}: ", app.i18n.t("sync.last_sync")),
                Style::default().fg(app.theme.accent),
            ),
            Span::raw(result.as_str()),
        ])));
        log_items.push(ListItem::new(""));
    }

    // Add sync log entries
    for entry in sync_state.sync_log.iter().rev().take(5) {
        log_items.push(ListItem::new(Span::styled(
            format!("  {}", entry),
            Style::default().fg(app.theme.fg_secondary),
        )));
    }

    if log_items.is_empty() || sync_state.last_result.is_none() {
        log_items.push(ListItem::new(""));
        log_items.push(ListItem::new(Span::styled(
            format!("  {}", app.i18n.t("sync.start_hint")),
            Style::default().fg(app.theme.warning),
        )));
        log_items.push(ListItem::new(""));
        log_items.push(ListItem::new(Span::styled(
            format!("  {}:", app.i18n.t("sync.operations")),
            Style::default().fg(app.theme.accent),
        )));
        log_items.push(ListItem::new(format!(
            "  - {}",
            app.i18n.t("sync.op_connect")
        )));
        log_items.push(ListItem::new(format!(
            "  - {}",
            app.i18n.t("sync.op_receive")
        )));
        log_items.push(ListItem::new(format!(
            "  - {}",
            app.i18n.t("sync.op_process")
        )));
        log_items.push(ListItem::new(format!("  - {}", app.i18n.t("sync.op_send"))));
        log_items.push(ListItem::new(""));
        log_items.push(ListItem::new(Span::styled(
            format!("  {}", app.i18n.t("sync.test_hint")),
            Style::default().fg(app.theme.fg_secondary),
        )));
    }

    let log_list = List::new(log_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(app.i18n.t("sync.info")),
    );
    f.render_widget(log_list, chunks[2]);
}
