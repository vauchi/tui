// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::ui::presentation_protocol::{
    ChoiceTarget, choice_target, row_is_addressable, row_toggle,
};
use ratatui::prelude::*;
use vauchi_core::PresentationNode;

pub(super) fn append_node_lines(
    node: &PresentationNode,
    depth: usize,
    lines: &mut Vec<Line<'static>>,
    selected_surface_target: Option<usize>,
    // Set to the line index the selection was painted on. The caller needs
    // it to scroll: without it a surface taller than its viewport paints
    // the first screenful and the selection walks off into nothing.
    selected_line: &mut Option<usize>,
    usable_width: usize,
) -> Option<usize> {
    let indent = "  ".repeat(depth);
    let mut remaining = selected_surface_target;
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
        } => {
            let Some(choice) = choice_target(node) else {
                lines.push(Line::from(format!(
                    "{indent}{label}: {}",
                    selected.as_deref().unwrap_or("—")
                )));
                return remaining;
            };
            let is_selected = remaining == Some(0);
            remaining = remaining.and_then(|n| n.checked_sub(1));
            if is_selected {
                *selected_line = Some(lines.len());
            }
            lines.push(choice_line(
                &indent,
                label,
                &choice,
                is_selected,
                usable_width,
            ));
        }
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
                remaining = append_node_lines(
                    child,
                    depth + 1,
                    lines,
                    remaining,
                    selected_line,
                    usable_width,
                );
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
                // Only addressable rows may consume a selection index —
                // otherwise the highlight drifts away from the row Enter
                // would operate. A row carrying a toggle is addressable
                // even though Core gives it no activation of its own.
                let addressable = row_is_addressable(row);
                let is_selected = addressable && remaining == Some(0);
                if addressable {
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
                // The row title names the setting, so Core sends the
                // control unlabelled; rendering it on its own line would
                // print a bare checkbox belonging to nothing.
                let marker = match row_toggle(row) {
                    Some((_, true)) => "[x]".to_string(),
                    Some((_, false)) => "[ ]".to_string(),
                    None => "•".to_string(),
                };
                if is_selected {
                    *selected_line = Some(lines.len());
                }
                lines.push(Line::styled(
                    format!("{indent}{marker} {}", row.title),
                    title_style,
                ));
                let secondary_style = if is_selected {
                    Style::default().add_modifier(Modifier::REVERSED)
                } else {
                    Style::default().add_modifier(Modifier::DIM)
                };
                if let Some(subtitle) = &row.subtitle {
                    lines.push(Line::styled(
                        format!("{indent}  {subtitle}"),
                        secondary_style,
                    ));
                }
                // A settings row's value lives in `detail`, so dropping it
                // hides the very thing the row exists to show. Nested like
                // the subtitle, matching how Status renders its own detail.
                if let Some(detail) = &row.detail {
                    lines.push(Line::styled(format!("{indent}  {detail}"), secondary_style));
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
            // A terminal cannot show a scannable code, so Display-purpose
            // payloads are printed for transcription; e2e/src/device/tui.rs
            // parses this line. Exposure is tracked in
            // backlog/2026-08-05-tui-qr-payload-rendered-plaintext.
            if *purpose == vauchi_core::PresentationQrPurpose::Display
                && let Some(payload) = payloads.first()
            {
                lines.push(Line::from(format!("{indent}  {payload}")));
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

/// `label: A  [ B ]  C` while the options fit, else `label: ‹ B ›` — the
/// selection stays legible in a narrow pane instead of wrapping into
/// noise. A focused choice carries the row highlight plus the keys that
/// step it, since nothing else on screen says how a choice is operated.
fn choice_line(
    indent: &str,
    label: &str,
    choice: &ChoiceTarget<'_>,
    is_selected: bool,
    usable_width: usize,
) -> Line<'static> {
    let label_style = if is_selected {
        Style::default().add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    };
    let picked_style = Style::default().add_modifier(Modifier::BOLD);
    let hint = if is_selected {
        Span::styled("  ←/→ picks", Style::default().add_modifier(Modifier::DIM))
    } else {
        Span::raw("")
    };

    let mut spans = vec![Span::styled(format!("{indent}{label}: "), label_style)];
    let picked = choice
        .selected
        .and_then(|selected| choice.options.iter().find(|option| option.id == selected));
    if picked.is_none() {
        spans.push(Span::styled("[ — ]", picked_style));
    }
    for (index, option) in choice.options.iter().enumerate() {
        if index > 0 || picked.is_none() {
            spans.push(Span::raw("  "));
        }
        if picked.is_some_and(|picked| picked.id == option.id) {
            spans.push(Span::styled(format!("[ {} ]", option.label), picked_style));
        } else {
            spans.push(Span::raw(option.label.clone()));
        }
    }
    spans.push(hint.clone());
    let expanded = Line::from(spans);
    if expanded.width() <= usable_width {
        return expanded;
    }
    Line::from(vec![
        Span::styled(format!("{indent}{label}: "), label_style),
        Span::styled(
            format!("‹ {} ›", picked.map_or("—", |option| option.label.as_str())),
            picked_style,
        ),
        hint,
    ])
}
