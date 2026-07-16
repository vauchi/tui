// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Miscellaneous component renderers — progress, title, status indicator,
//! PIN input, QR code, confirmation dialog, and action key hints.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::theme::TuiTheme;
use crate::ui::widgets::icon::icon_badge;
use vauchi_app::ui::{IndicatorKind, QrMode, Status};

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

/// Render an `Indicator` component — single-line chrome chip with a
/// kind-driven glyph, label, and optional "press" hint when tappable.
///
/// Semantic mapping mirrors the iOS chip:
/// - `Active`  → ● (success / emphasis)
/// - `Error`   → ✗ (error / attention-required)
/// - `Neutral` → ○ (muted / informational)
/// - `Busy`    → ◉ (animated counterpart at terminal granularity)
pub(super) fn render_indicator(
    f: &mut Frame,
    area: Rect,
    label: &str,
    kind: &IndicatorKind,
    tappable: bool,
    is_focused: bool,
    theme: &TuiTheme,
) {
    let (glyph, color) = match kind {
        IndicatorKind::Active => ("●", theme.success),
        IndicatorKind::Error => ("✗", theme.error),
        IndicatorKind::Neutral => ("○", theme.fg_secondary),
        IndicatorKind::Busy => ("◉", theme.accent),
        _ => ("•", theme.fg_secondary),
    };

    let mut style = Style::default().fg(color);
    if tappable {
        style = style.add_modifier(Modifier::BOLD);
    }
    if is_focused && tappable {
        style = style.bg(theme.bg_secondary);
    }

    let mut line_spans = vec![Span::styled(format!(" {glyph} "), style)];
    line_spans.push(Span::styled(label.to_string(), style));
    if tappable {
        line_spans.push(Span::styled(
            "  [Enter]".to_string(),
            Style::default().fg(theme.fg_secondary),
        ));
    }
    let para = Paragraph::new(Line::from(line_spans));
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

    let display_icon = icon.and_then(icon_badge).unwrap_or(status_icon);
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
    // Core supplies the localized QR label (ADR-044 generic-component
    // contract); the mode discriminant selects presentation only.
    let title = label.map(|l| format!(" {l} ")).unwrap_or_default();

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

            lines
        }
        QrMode::Scan => {
            vec![
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
            ]
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

/// Map action IDs to keyboard hints (public accessor for footer rendering).
///
/// Delegates to the unified action↔key table in
/// `crate::ui::widgets::key_mapping::action_table` so footer hints
/// and dispatch share one source of truth.
pub fn action_key_hint_pub(action_id: &str) -> &'static str {
    crate::ui::widgets::key_mapping::action_table::key_for_action(action_id)
}
