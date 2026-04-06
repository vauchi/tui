// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Miscellaneous component renderers — progress, title, status indicator,
//! PIN input, QR code, confirmation dialog, and action key hints.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::theme::TuiTheme;
use vauchi_app::ui::{QrMode, Status};

/// Render the step progress indicator.
pub(super) fn render_progress(
    f: &mut Frame,
    area: Rect,
    progress: &vauchi_app::ui::Progress,
    theme: &TuiTheme,
) {
    let dots: String = (1..=progress.total_steps)
        .map(|i| {
            if i == progress.current_step {
                "●"
            } else {
                "○"
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    let label = progress
        .label
        .as_ref()
        .map(|l| format!("  {}", l))
        .unwrap_or_default();

    let text = format!(
        "[{}/{}]  {}{}",
        progress.current_step, progress.total_steps, dots, label
    );

    let para = Paragraph::new(text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.accent));

    f.render_widget(para, area);
}

/// Render the screen title and optional subtitle.
pub(super) fn render_title(
    f: &mut Frame,
    area: Rect,
    title: &str,
    subtitle: Option<&str>,
    theme: &TuiTheme,
) {
    let mut lines = vec![Line::from(Span::styled(
        title,
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    ))];

    if let Some(sub) = subtitle {
        lines.push(Line::from(Span::styled(
            sub,
            Style::default().fg(theme.fg_secondary),
        )));
    }

    let para = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(para, area);
}

/// Render a status indicator component.
pub(super) fn render_status_indicator(
    f: &mut Frame,
    area: Rect,
    icon: Option<&str>,
    title: &str,
    detail: Option<&str>,
    status: &Status,
    theme: &TuiTheme,
) {
    let (status_icon, color) = match status {
        Status::Pending => ("○", theme.fg_secondary),
        Status::InProgress => ("◉", theme.accent),
        Status::Success => ("✓", theme.success),
        Status::Failed => ("✗", theme.error),
        Status::Warning => ("⚠", theme.warning),
        _ => ("?", theme.fg_secondary),
    };

    let display_icon = icon.unwrap_or(status_icon);
    let mut text = format!(" {} {}", display_icon, title);
    if let Some(d) = detail {
        text.push_str(&format!(" — {}", d));
    }

    let para = Paragraph::new(text)
        .style(Style::default().fg(color))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        );
    f.render_widget(para, area);
}

/// Render a PIN input component.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_pin_input(
    f: &mut Frame,
    area: Rect,
    label: &str,
    length: usize,
    filled: usize,
    _masked: bool,
    error: Option<&str>,
    is_focused: bool,
    theme: &TuiTheme,
) {
    let dots: String = (0..length)
        .enumerate()
        .map(|(i, _)| if i < filled { "● " } else { "○ " })
        .collect::<String>()
        .trim()
        .to_string();

    let mut lines = vec![
        Line::from(Span::styled(
            label,
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", dots),
            Style::default().fg(theme.accent),
        )),
    ];

    if let Some(err) = error {
        lines.push(Line::from(Span::styled(
            format!("  {}", err),
            Style::default().fg(theme.error),
        )));
    }

    let border_color = if is_focused {
        theme.accent
    } else {
        theme.border
    };
    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    );
    f.render_widget(para, area);
}

/// Render a QR code component (text representation in terminal).
pub(super) fn render_qr_code(
    f: &mut Frame,
    area: Rect,
    data: &str,
    mode: &QrMode,
    label: Option<&str>,
    theme: &TuiTheme,
) {
    let title = match mode {
        QrMode::Display => " QR Code ",
        QrMode::Scan => " Scan QR Code ",
        _ => " QR Code ",
    };

    let content = match mode {
        QrMode::Display => {
            let mut lines = vec![Line::from("")];

            match qrcode::QrCode::new(data) {
                Ok(code) => {
                    let image = code
                        .render::<qrcode::render::unicode::Dense1x2>()
                        .dark_color(qrcode::render::unicode::Dense1x2::Light)
                        .light_color(qrcode::render::unicode::Dense1x2::Dark)
                        .quiet_zone(false)
                        .build();

                    for qr_line in image.lines() {
                        lines.push(Line::from(Span::styled(
                            format!("  {}", qr_line),
                            Style::default().fg(theme.fg),
                        )));
                    }
                }
                Err(_) => {
                    lines.push(Line::from(Span::styled(
                        "  (QR code too large to render)",
                        Style::default().fg(theme.error),
                    )));
                }
            }

            if let Some(l) = label {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    format!("  {}", l),
                    Style::default().fg(theme.fg_secondary),
                )));
            }
            lines
        }
        QrMode::Scan => {
            let mut lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  Point your camera at the QR code",
                    Style::default().fg(theme.fg),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  (Not available in terminal mode)",
                    Style::default().fg(theme.fg_secondary),
                )),
            ];
            if let Some(l) = label {
                lines.push(Line::from(Span::styled(
                    format!("  {}", l),
                    Style::default().fg(theme.fg_secondary),
                )));
            }
            lines
        }
        _ => {
            vec![Line::from(Span::styled(
                "  (QR mode not supported)",
                Style::default().fg(theme.fg_secondary),
            ))]
        }
    };

    let para = Paragraph::new(content).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent)),
    );
    f.render_widget(para, area);
}

/// Render a confirmation dialog component.
pub(super) fn render_confirmation_dialog(
    f: &mut Frame,
    area: Rect,
    title: &str,
    message: &str,
    confirm_text: &str,
    destructive: bool,
    theme: &TuiTheme,
) {
    let confirm_style = if destructive {
        Style::default()
            .fg(theme.error)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    };

    let lines = vec![
        Line::from(Span::styled(
            format!("  {}", title),
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", message),
            Style::default().fg(theme.fg),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("  [Enter] {}  [Esc] Cancel", confirm_text),
            confirm_style,
        )),
    ];

    let border_color = if destructive {
        theme.error
    } else {
        theme.accent
    };
    let para = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    );
    f.render_widget(para, area);
}

/// Public wrapper for rendering a non-destructive confirmation dialog overlay.
pub fn render_discard_confirmation(
    f: &mut Frame,
    area: Rect,
    title: &str,
    message: &str,
    theme: &TuiTheme,
) {
    render_confirmation_dialog(f, area, title, message, "Discard", false, theme);
}

/// Map action IDs to keyboard hints (public accessor for footer rendering).
pub fn action_key_hint_pub(action_id: &str) -> &'static str {
    action_key_hint(action_id)
}

/// Map action IDs to keyboard hints.
fn action_key_hint(action_id: &str) -> &'static str {
    match action_id {
        "get_started" | "continue" | "continue_setup" | "start" | "confirm" | "unlock" | "done"
        | "save" => "Enter",
        "restore_backup" | "setup_backup" | "backup" => "b",
        "skip" | "skip_to_finish" => "s",
        "edit" => "e",
        "add_contact" | "add_field" | "add" => "a",
        "view_all" | "view" => "v",
        "retry" | "retry_all" => "r",
        "cancel" | "back" => "Esc",
        "create_new" => "Enter",
        "have_identity" => "h",
        "delete" | "wipe" | "emergency_wipe" => "x",
        "delete_contact" | "archive_contact" => "d",
        "scan" => "S",
        "enable" | "disable" | "toggle" => "t",
        "toggle_view" => "Enter",
        id if id.starts_with("filter_group") => "g",
        _ => "Enter",
    }
}
