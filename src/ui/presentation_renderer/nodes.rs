// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use ratatui::prelude::*;
use vauchi_core::PresentationNode;

pub(super) fn append_node_lines(
    node: &PresentationNode,
    depth: usize,
    lines: &mut Vec<Line<'static>>,
    selected_surface_row: Option<usize>,
) -> Option<usize> {
    let indent = "  ".repeat(depth);
    let mut remaining = selected_surface_row;
    match node {
        PresentationNode::Text { content, .. } => {
            lines.push(Line::from(format!("{indent}{content}")));
        }
        PresentationNode::Input {
            label,
            value,
            placeholder,
            validation_error,
            ..
        } => {
            let shown = if value.is_empty() {
                placeholder.as_deref().unwrap_or("")
            } else {
                value
            };
            lines.push(Line::from(format!("{indent}{label}: {shown}")));
            if let Some(error) = validation_error {
                lines.push(Line::styled(
                    format!("{indent}! {error}"),
                    Style::default().add_modifier(Modifier::BOLD),
                ));
            }
        }
        PresentationNode::Toggle { label, value, .. } => lines.push(Line::from(format!(
            "{indent}[{}] {label}",
            if *value { "x" } else { " " }
        ))),
        PresentationNode::Choice {
            label, selected, ..
        } => lines.push(Line::from(format!(
            "{indent}{label}: {}",
            selected.as_deref().unwrap_or("—")
        ))),
        PresentationNode::Group {
            label, children, ..
        } => {
            if let Some(label) = label {
                lines.push(Line::styled(
                    format!("{indent}{label}"),
                    Style::default().add_modifier(Modifier::BOLD),
                ));
            }
            for child in children {
                remaining = append_node_lines(child, depth + 1, lines, remaining);
            }
        }
        PresentationNode::List { label, rows, .. } => {
            if let Some(label) = label {
                lines.push(Line::styled(
                    format!("{indent}{label}"),
                    Style::default().add_modifier(Modifier::BOLD),
                ));
            }
            for row in rows {
                // Only activatable rows are addressable, so only they may
                // consume a selection index — otherwise the highlight drifts
                // away from the row Enter would activate.
                let is_selected = row.activation.is_some() && remaining == Some(0);
                if row.activation.is_some() {
                    remaining = match remaining {
                        Some(0) => None,
                        Some(n) => Some(n - 1),
                        None => None,
                    };
                }

                let title_style = if is_selected {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default()
                };
                lines.push(Line::styled(
                    format!("{indent}• {}", row.title),
                    title_style,
                ));
                if let Some(subtitle) = &row.subtitle {
                    let subtitle_style = if is_selected {
                        Style::default().add_modifier(Modifier::REVERSED)
                    } else {
                        Style::default().add_modifier(Modifier::DIM)
                    };
                    lines.push(Line::styled(
                        format!("{indent}  {subtitle}"),
                        subtitle_style,
                    ));
                }
            }
        }
        PresentationNode::Image { fallback_text, .. } => lines.push(Line::from(format!(
            "{indent}{}",
            fallback_text.as_deref().unwrap_or("[image]")
        ))),
        PresentationNode::Status { title, detail, .. } => {
            lines.push(Line::from(format!("{indent}{title}")));
            if let Some(detail) = detail {
                lines.push(Line::from(format!("{indent}  {detail}")));
            }
        }
        PresentationNode::Qr {
            label,
            payloads,
            purpose,
            ..
        } => {
            lines.push(Line::from(format!(
                "{indent}[QR] {}",
                label.as_deref().unwrap_or("")
            )));
            // In Display mode, render the payload as copyable text so terminal
            // users (and the E2E harness) can read or transcribe it.
            if *purpose == vauchi_core::PresentationQrPurpose::Display {
                if let Some(payload) = payloads.first() {
                    lines.push(Line::from(format!("{indent}  {payload}")));
                }
            }
        }
        PresentationNode::Confirmation { warning, .. } => {
            lines.push(Line::from(format!("{indent}! {warning}")));
        }
        PresentationNode::Slider { label, value, .. } => {
            lines.push(Line::from(format!("{indent}{label}: {value}")));
        }
        PresentationNode::Progress { label, value, .. } => lines.push(Line::from(format!(
            "{indent}{} {}",
            label.as_deref().unwrap_or("Progress"),
            value
                .map(|v| format!("{:.0}%", v * 100.0))
                .unwrap_or_default()
        ))),
        PresentationNode::Divider => lines.push(Line::from(format!("{indent}────────"))),
        _ => {}
    }
    remaining
}
