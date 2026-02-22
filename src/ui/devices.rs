// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Device Management Screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};

use crate::app::App;

/// Draw the devices screen.
pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Info + instructions
            Constraint::Min(0),    // Device list
            Constraint::Length(3), // Key hints
        ])
        .split(area);

    // Device info section
    draw_info_section(f, main_chunks[0], app);

    // Device list section
    draw_device_list(f, main_chunks[1], app);

    // Key hints
    draw_key_hints(f, main_chunks[2], app);

    // Overlay: QR code display
    if let Some(ref link_result) = app.device_link_result {
        draw_link_overlay(f, area, link_result);
    }

    // Overlay: Revoke confirmation
    if app.revoke_confirm {
        draw_revoke_confirm(f, area, app);
    }
}

fn draw_info_section(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60), // Device info
            Constraint::Percentage(40), // Instructions
        ])
        .split(area);

    // Device info
    let info_text = if app.backend.has_identity() {
        let device_count = app.backend.list_devices().map(|d| d.len()).unwrap_or(0);
        vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    format!("{}: ", app.i18n.t("devices.count")),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(format!("{}", device_count)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                app.i18n.t("devices.manage_description"),
                Style::default().fg(Color::DarkGray),
            )),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                app.i18n.t("devices.no_identity"),
                Style::default().fg(Color::Yellow),
            )),
            Line::from(""),
            Line::from(Span::styled(
                app.i18n.t("devices.create_first"),
                Style::default().fg(Color::DarkGray),
            )),
        ]
    };

    let info = Paragraph::new(info_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(app.i18n.t("devices.info")),
    );
    f.render_widget(info, chunks[0]);

    // Instructions
    let instructions = Paragraph::new(vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("[l]", Style::default().fg(Color::Yellow)),
            Span::raw(format!(" {}", app.i18n.t("devices.generate_link"))),
        ]),
        Line::from(vec![
            Span::styled("[r]", Style::default().fg(Color::Yellow)),
            Span::raw(format!(" {}", app.i18n.t("devices.revoke"))),
        ]),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(app.i18n.t("devices.actions")),
    );
    f.render_widget(instructions, chunks[1]);
}

fn draw_device_list(f: &mut Frame, area: Rect, app: &App) {
    let devices = app.backend.list_devices().unwrap_or_default();

    if devices.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                app.i18n.t("devices.empty"),
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(""),
            Line::from(Span::raw(app.i18n.t("devices.link_hint"))),
        ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(app.i18n.t("devices.linked")),
        );
        f.render_widget(empty, area);
        return;
    }

    let device_items: Vec<ListItem> = devices
        .iter()
        .enumerate()
        .map(|(idx, device)| {
            let is_selected = idx == app.selected_device;
            let prefix = if is_selected { "› " } else { "  " };

            let status_span = if device.is_current {
                Span::styled(" [this device]", Style::default().fg(Color::Green))
            } else if device.is_active {
                Span::styled(" [active]", Style::default().fg(Color::Blue))
            } else {
                Span::styled(" [revoked]", Style::default().fg(Color::Red))
            };

            let line = Line::from(vec![
                Span::styled(prefix, Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{}. ", device.device_index + 1),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(&device.device_name),
                status_span,
                Span::styled(
                    format!(
                        "  ({}...)",
                        &device.public_key_prefix[..8.min(device.public_key_prefix.len())]
                    ),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);

            let style = if is_selected {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let device_list = List::new(device_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(app.i18n.t("devices.linked")),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    f.render_widget(device_list, area);
}

fn draw_key_hints(f: &mut Frame, area: Rect, app: &App) {
    let hints = if app.device_link_result.is_some() || app.revoke_confirm {
        Paragraph::new(Line::from(vec![
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" Dismiss"),
        ]))
    } else {
        Paragraph::new(Line::from(vec![
            Span::styled("j/↓", Style::default().fg(Color::Yellow)),
            Span::raw(" Down  "),
            Span::styled("k/↑", Style::default().fg(Color::Yellow)),
            Span::raw(" Up  "),
            Span::styled("l", Style::default().fg(Color::Yellow)),
            Span::raw(" Link  "),
            Span::styled("r", Style::default().fg(Color::Yellow)),
            Span::raw(" Revoke  "),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" Back"),
        ]))
    };

    f.render_widget(
        hints
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::TOP)),
        area,
    );
}

/// Draw the device link QR code overlay.
fn draw_link_overlay(f: &mut Frame, area: Rect, link_result: &crate::backend::DeviceLinkResult) {
    // Center overlay at 80% of screen
    let overlay_area = centered_rect(80, 90, area);
    f.render_widget(Clear, overlay_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Title + fingerprint
            Constraint::Min(0),    // QR code
            Constraint::Length(5), // Data string + instructions
        ])
        .split(overlay_area);

    // Title with fingerprint
    let title = Paragraph::new(vec![
        Line::from(Span::styled(
            "Device Link QR Code",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("Fingerprint: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&link_result.fingerprint, Style::default().fg(Color::Yellow)),
        ]),
    ])
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL).title(" Link "));
    f.render_widget(title, chunks[0]);

    // QR code (ASCII art)
    let qr = Paragraph::new(link_result.qr_ascii.as_str())
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title(" QR Code "));
    f.render_widget(qr, chunks[1]);

    // Data string for copy-paste + instructions
    let truncated_data = if link_result.data_string.len() > 60 {
        format!("{}...", &link_result.data_string[..60])
    } else {
        link_result.data_string.clone()
    };

    let info = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Data: ", Style::default().fg(Color::DarkGray)),
            Span::raw(truncated_data),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Scan with new device or copy data string. Press Esc to dismiss.",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .wrap(Wrap { trim: true })
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Instructions "),
    );
    f.render_widget(info, chunks[2]);
}

/// Draw the revoke confirmation overlay.
fn draw_revoke_confirm(f: &mut Frame, area: Rect, app: &App) {
    let devices = app.backend.list_devices().unwrap_or_default();
    let device_name = devices
        .get(app.selected_device)
        .map(|d| d.device_name.as_str())
        .unwrap_or("Unknown");

    let overlay_area = centered_rect(50, 30, area);
    f.render_widget(Clear, overlay_area);

    let confirm = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("Revoke device '{}'?", device_name),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("This device will lose access to your identity."),
        Line::from(""),
        Line::from(vec![
            Span::styled("[y]", Style::default().fg(Color::Red)),
            Span::raw(" Confirm  "),
            Span::styled("[n/Esc]", Style::default().fg(Color::Yellow)),
            Span::raw(" Cancel"),
        ]),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Confirm Revoke ")
            .border_style(Style::default().fg(Color::Red)),
    );
    f.render_widget(confirm, overlay_area);
}

/// Create a centered rectangle with the given percentage of the parent area.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
