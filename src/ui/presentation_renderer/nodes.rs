// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use ratatui::prelude::*;
use vauchi_core::PresentationNode;

pub(super) fn append_node_lines(
    node: &PresentationNode,
    depth: usize,
    lines: &mut Vec<Line<'static>>,
) {
    let indent = "  ".repeat(depth);
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
                append_node_lines(child, depth + 1, lines);
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
                lines.push(Line::from(format!("{indent}• {}", row.title)));
                if let Some(subtitle) = &row.subtitle {
                    lines.push(Line::from(format!("{indent}  {subtitle}")));
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
        PresentationNode::Qr { label, .. } => lines.push(Line::from(format!(
            "{indent}[QR] {}",
            label.as_deref().unwrap_or("")
        ))),
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
}
