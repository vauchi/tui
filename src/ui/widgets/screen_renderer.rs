// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! ScreenRenderer — maps a `ScreenModel` to a full terminal layout.
//!
//! Renders: progress bar at top, title/subtitle, components in order,
//! and action hints at the bottom.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::theme::TuiTheme;
use vauchi_core::ui::{ActionStyle, Component, ScreenModel, TextStyle};

use super::card_preview::CardPreviewWidget;
use super::field_list::FieldListWidget;
use super::info_panel::InfoPanelWidget;
use super::text_input::TextInputWidget;
use super::toggle_list::ToggleListWidget;

/// Tracks focus and selection state for the screen renderer.
#[derive(Default)]
pub struct ScreenRenderState {
    /// Index of the currently focused component (for keyboard navigation).
    pub focused_component: usize,
    /// Per-component selection index (e.g., which item is highlighted in a list).
    pub component_selections: Vec<usize>,
    /// Validation errors keyed by component_id.
    pub validation_errors: Vec<(String, String)>,
}

impl ScreenRenderState {
    /// Ensure the selections vector has enough entries for the given component count.
    pub fn ensure_capacity(&mut self, component_count: usize) {
        while self.component_selections.len() < component_count {
            self.component_selections.push(0);
        }
    }

    /// Get the selection index for a given component.
    pub fn selection_for(&self, index: usize) -> usize {
        self.component_selections.get(index).copied().unwrap_or(0)
    }

    /// Get a validation error for a given component_id.
    pub fn validation_error_for(&self, component_id: &str) -> Option<&str> {
        self.validation_errors
            .iter()
            .find(|(id, _)| id == component_id)
            .map(|(_, msg)| msg.as_str())
    }

    /// Set a validation error for a component.
    pub fn set_validation_error(&mut self, component_id: String, message: String) {
        // Replace existing or add new
        if let Some(entry) = self
            .validation_errors
            .iter_mut()
            .find(|(id, _)| *id == component_id)
        {
            entry.1 = message;
        } else {
            self.validation_errors.push((component_id, message));
        }
    }

    /// Clear a validation error for a component.
    pub fn clear_validation_error(&mut self, component_id: &str) {
        self.validation_errors.retain(|(id, _)| id != component_id);
    }

    /// Clear all validation errors.
    pub fn clear_all_errors(&mut self) {
        self.validation_errors.clear();
    }
}

/// Render a complete `ScreenModel` into the given area.
pub fn render_screen(
    f: &mut Frame,
    area: Rect,
    screen: &ScreenModel,
    state: &ScreenRenderState,
    theme: &TuiTheme,
) {
    // Calculate layout heights
    let has_progress = screen.progress.is_some();
    let has_subtitle = screen.subtitle.is_some();
    let has_actions = !screen.actions.is_empty();

    let mut constraints = Vec::new();

    // Progress bar
    if has_progress {
        constraints.push(Constraint::Length(1));
    }

    // Title
    constraints.push(Constraint::Length(if has_subtitle { 3 } else { 2 }));

    // Components area (flexible)
    constraints.push(Constraint::Min(5));

    // Action hints
    if has_actions {
        constraints.push(Constraint::Length(2));
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut chunk_idx = 0;

    // Progress bar
    if let Some(progress) = &screen.progress {
        render_progress(f, chunks[chunk_idx], progress, theme);
        chunk_idx += 1;
    }

    // Title and subtitle
    render_title(
        f,
        chunks[chunk_idx],
        &screen.title,
        screen.subtitle.as_deref(),
        theme,
    );
    chunk_idx += 1;

    // Components
    render_components(f, chunks[chunk_idx], &screen.components, state, theme);
    chunk_idx += 1;

    // Action hints
    if has_actions && chunk_idx < chunks.len() {
        render_action_hints(f, chunks[chunk_idx], &screen.actions, theme);
    }
}

/// Render the step progress indicator.
fn render_progress(
    f: &mut Frame,
    area: Rect,
    progress: &vauchi_core::ui::Progress,
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
fn render_title(f: &mut Frame, area: Rect, title: &str, subtitle: Option<&str>, theme: &TuiTheme) {
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

/// Render all components in the screen model.
fn render_components(
    f: &mut Frame,
    area: Rect,
    components: &[Component],
    state: &ScreenRenderState,
    theme: &TuiTheme,
) {
    if components.is_empty() {
        return;
    }

    // Build constraints: each component gets a portion of the available space
    let constraints: Vec<Constraint> = components
        .iter()
        .map(|c| match c {
            Component::Text { .. } => Constraint::Length(2),
            Component::TextInput { .. } => Constraint::Length(5),
            Component::ToggleList { items, .. } => {
                // Each item is 1-2 lines, plus border
                let item_lines: usize = items
                    .iter()
                    .map(|i| if i.subtitle.is_some() { 2 } else { 1 })
                    .sum();
                Constraint::Length((item_lines as u16 + 2).min(area.height / 2))
            }
            Component::FieldList { fields, .. } => {
                Constraint::Length((fields.len() as u16 + 4).min(area.height / 2))
            }
            Component::CardPreview { group_views, .. } => {
                let extra = if group_views.is_empty() { 0 } else { 2 };
                Constraint::Min(8 + extra)
            }
            Component::InfoPanel { items, .. } => {
                Constraint::Length((items.len() as u16 * 3 + 2).min(area.height / 2))
            }
            Component::Divider => Constraint::Length(1),
        })
        .collect();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    for (i, component) in components.iter().enumerate() {
        let is_focused = i == state.focused_component;
        let chunk = chunks[i];

        match component {
            Component::Text { content, style, .. } => {
                render_text(f, chunk, content, style, theme);
            }
            Component::TextInput {
                id,
                label,
                value,
                placeholder,
                validation_error,
                ..
            } => {
                let error = state
                    .validation_error_for(id)
                    .or(validation_error.as_deref());

                TextInputWidget {
                    label,
                    value,
                    placeholder: placeholder.as_deref(),
                    validation_error: error,
                    focused: is_focused,
                    theme,
                }
                .render(f, chunk);
            }
            Component::ToggleList { label, items, .. } => {
                ToggleListWidget {
                    label,
                    items,
                    selected_index: state.selection_for(i),
                    focused: is_focused,
                    theme,
                }
                .render(f, chunk);
            }
            Component::FieldList {
                fields,
                visibility_mode,
                available_groups,
                ..
            } => {
                FieldListWidget {
                    fields,
                    visibility_mode,
                    available_groups,
                    selected_index: state.selection_for(i),
                    focused: is_focused,
                    theme,
                }
                .render(f, chunk);
            }
            Component::CardPreview {
                name,
                fields,
                group_views,
                selected_group,
            } => {
                CardPreviewWidget {
                    name,
                    fields,
                    group_views,
                    selected_group: selected_group.as_deref(),
                    theme,
                }
                .render(f, chunk);
            }
            Component::InfoPanel {
                title, icon, items, ..
            } => {
                InfoPanelWidget {
                    title,
                    icon: icon.as_deref(),
                    items,
                    theme,
                }
                .render(f, chunk);
            }
            Component::Divider => {
                let divider = Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::default().fg(theme.border));
                f.render_widget(divider, chunk);
            }
        }
    }
}

/// Render a text component with the appropriate style.
fn render_text(f: &mut Frame, area: Rect, content: &str, style: &TextStyle, theme: &TuiTheme) {
    let text_style = match style {
        TextStyle::Title => Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
        TextStyle::Subtitle => Style::default().fg(theme.fg_secondary),
        TextStyle::Body => Style::default().fg(theme.fg),
        TextStyle::Caption => Style::default().fg(theme.fg_secondary),
    };

    let para = Paragraph::new(content).style(text_style);
    f.render_widget(para, area);
}

/// Render action hints at the bottom of the screen.
fn render_action_hints(
    f: &mut Frame,
    area: Rect,
    actions: &[vauchi_core::ui::ScreenAction],
    theme: &TuiTheme,
) {
    let hints: Vec<Span> = actions
        .iter()
        .filter(|a| a.enabled)
        .enumerate()
        .flat_map(|(i, action)| {
            let key_hint = action_key_hint(&action.id);
            let style = match action.style {
                ActionStyle::Primary => Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
                ActionStyle::Secondary => Style::default().fg(theme.fg_secondary),
                ActionStyle::Destructive => Style::default().fg(theme.error),
            };

            let mut spans = Vec::new();
            if i > 0 {
                spans.push(Span::styled("  ", Style::default()));
            }
            spans.push(Span::styled(
                format!("[{}] {}", key_hint, action.label),
                style,
            ));
            spans
        })
        .collect();

    let para = Paragraph::new(Line::from(hints)).alignment(Alignment::Center);
    f.render_widget(para, area);
}

/// Map action IDs to keyboard hints.
fn action_key_hint(action_id: &str) -> &'static str {
    match action_id {
        "get_started" | "continue" | "continue_setup" | "start" => "Enter",
        "restore_backup" | "setup_backup" => "b",
        "skip" | "skip_to_finish" => "s",
        "edit" => "e",
        _ => "Enter",
    }
}
