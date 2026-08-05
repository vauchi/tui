// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Native terminal widgets for Core's generic presentation values.

use ratatui::layout::Flex;
use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use vauchi_core::Command;
use vauchi_core::{ActionSpec, OverlayKind};

use super::presentation_protocol::PresentationState;

mod nodes;
use nodes::append_node_lines;

pub(crate) fn draw(
    frame: &mut Frame,
    area: Rect,
    state: &PresentationState,
    selected_action: usize,
    selected_surface_row: Option<usize>,
) {
    let [content_area, bar_area] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(3)]).areas(area);
    draw_surface(frame, content_area, state, selected_surface_row);
    draw_context_bar(frame, bar_area, state, selected_action);
    if state.overlay().is_some() {
        draw_overlay(frame, area, state, selected_action);
    }
}

pub(crate) fn draw_effect_prompt(frame: &mut Frame, area: Rect, effect: &Command, input: &str) {
    let instruction = match effect {
        Command::FilePickFromUser { .. } => "File path",
        Command::QrRequestScan => "Paste QR data",
        Command::ImagePickFromFile
        | Command::ImagePickFromLibrary
        | Command::ImageCaptureFromCamera => "Image path",
        _ => return,
    };
    let area = centered_rect(72, 24, area);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(format!("{instruction}:")),
            Line::styled(
                format!("> {input}"),
                Style::default().add_modifier(Modifier::REVERSED),
            ),
            Line::styled(
                "Enter confirms · Esc cancels",
                Style::default().add_modifier(Modifier::DIM),
            ),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Core request"),
        ),
        area,
    );
}

pub(crate) fn draw_feedback(
    frame: &mut Frame,
    area: Rect,
    status: Option<&str>,
    alert: Option<&(String, String)>,
) {
    if let Some((title, message)) = alert {
        let area = centered_rect(68, 34, area);
        frame.render_widget(Clear, area);
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(message.clone()),
                Line::default(),
                Line::styled(
                    "Enter or Esc dismisses",
                    Style::default().add_modifier(Modifier::DIM),
                ),
            ])
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .title(title.clone()),
            ),
            area,
        );
    } else if let Some(status) = status {
        let toast_area = Rect::new(
            area.x.saturating_add(2),
            area.y,
            area.width.saturating_sub(4),
            3.min(area.height),
        );
        frame.render_widget(
            Paragraph::new(status.to_string())
                .block(Block::default().borders(Borders::ALL).title("Status")),
            toast_area,
        );
    }
}

fn draw_surface(
    frame: &mut Frame,
    area: Rect,
    state: &PresentationState,
    selected_surface_row: Option<usize>,
) {
    let surfaces = state.visible_surfaces();
    if surfaces.is_empty() {
        frame.render_widget(Paragraph::new("Preparing…"), area);
        return;
    }
    if surfaces.len() == 2 {
        let [primary, detail] =
            Layout::horizontal([Constraint::Percentage(38), Constraint::Percentage(62)])
                .areas(area);
        // The selection indexes the active surface, so highlighting the pane
        // that merely sits first would mark rows the keyboard cannot reach.
        let selection_for = |surface: &vauchi_core::SurfaceSpec| {
            state
                .surface()
                .filter(|active| active.surface_id == surface.surface_id)
                .and(selected_surface_row)
        };
        draw_surface_spec(frame, primary, surfaces[0], selection_for(surfaces[0]));
        draw_surface_spec(frame, detail, surfaces[1], selection_for(surfaces[1]));
    } else {
        draw_surface_spec(frame, area, surfaces[0], selected_surface_row);
    }
}

fn draw_surface_spec(
    frame: &mut Frame,
    area: Rect,
    surface: &vauchi_core::SurfaceSpec,
    selected_surface_row: Option<usize>,
) {
    let mut lines = Vec::new();
    if let Some(subtitle) = &surface.subtitle {
        lines.push(Line::styled(
            subtitle.clone(),
            Style::default().add_modifier(Modifier::DIM),
        ));
        lines.push(Line::default());
    }
    let mut remaining = selected_surface_row;
    for node in &surface.nodes {
        remaining = append_node_lines(node, 0, &mut lines, remaining);
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(surface.title.clone()),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn draw_context_bar(frame: &mut Frame, area: Rect, state: &PresentationState, selected: usize) {
    let Some(bar) = state.context_bar() else {
        frame.render_widget(Block::default().borders(Borders::TOP), area);
        return;
    };
    let roles = [
        (bar.back.as_ref(), "< ", false),
        (bar.navigation.as_ref(), "≡ ", false),
        (bar.primary.as_ref(), "", true),
        (bar.secondary.as_ref(), "… ", false),
    ];
    let spans = roles
        .into_iter()
        .filter_map(|(action, prefix, primary)| action.map(|a| (a, prefix, primary)))
        .enumerate()
        .flat_map(|(index, (action, prefix, primary))| {
            let label = if primary {
                format!("[ {} ]", action.label)
            } else {
                format!("{prefix}{}", action.label)
            };
            let style = action_style(action, index == selected, primary);
            [Span::styled(label, style), Span::raw("   ")]
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(Line::from(spans))
            .block(Block::default().borders(Borders::ALL).title("Commands")),
        area,
    );
}

fn action_style(action: &ActionSpec, selected: bool, primary: bool) -> Style {
    let mut style = if action.enabled {
        Style::default()
    } else {
        Style::default().add_modifier(Modifier::DIM)
    };
    if primary {
        style = style.add_modifier(Modifier::BOLD);
    }
    if selected {
        style = style.add_modifier(Modifier::REVERSED);
    }
    style
}

fn draw_overlay(frame: &mut Frame, area: Rect, state: &PresentationState, selected: usize) {
    let overlay = state.overlay().expect("checked by caller");
    let area = centered_rect(62, 55, area);
    let (fallback_title, border_type) = match overlay.kind {
        OverlayKind::Navigation => ("Navigate", BorderType::Double),
        OverlayKind::ActionMenu => ("Actions", BorderType::Rounded),
        _ => ("Commands", BorderType::Plain),
    };
    let lines = overlay
        .items
        .iter()
        .enumerate()
        .map(|(index, action)| {
            Line::styled(
                format!("{}. {}", index + 1, action.label),
                action_style(action, index == selected, false),
            )
        })
        .collect::<Vec<_>>();
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(border_type)
                .title(
                    overlay
                        .title
                        .clone()
                        .unwrap_or_else(|| fallback_title.into()),
                ),
        ),
        area,
    );
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let [vertical] = Layout::vertical([Constraint::Percentage(percent_y)])
        .flex(Flex::Center)
        .areas(area);
    let [horizontal] = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .areas(vertical);
    horizontal
}
