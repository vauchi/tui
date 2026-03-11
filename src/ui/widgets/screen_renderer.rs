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
use vauchi_core::ui::{
    ActionStyle, Component, QrMode, ScreenModel, SettingsItemKind, Status, TextStyle,
};

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
    /// Whether content zone has focus (false when ActionBar/NavBar is focused).
    pub content_has_focus: bool,
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
            Component::ContactList { contacts, .. } => {
                Constraint::Length((contacts.len() as u16 + 2).min(area.height / 2).max(4))
            }
            Component::SettingsGroup { items, .. } => {
                Constraint::Length((items.len() as u16 + 2).min(area.height / 2))
            }
            Component::ActionList { items, .. } => {
                Constraint::Length((items.len() as u16 + 2).min(area.height / 2))
            }
            Component::StatusIndicator { .. } => Constraint::Length(3),
            Component::PinInput { .. } => Constraint::Length(5),
            Component::QrCode { .. } => Constraint::Min(8),
            Component::ConfirmationDialog { .. } => Constraint::Min(6),
            Component::Divider => Constraint::Length(1),
        })
        .collect();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    for (i, component) in components.iter().enumerate() {
        let is_focused = state.content_has_focus && i == state.focused_component;
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
            Component::ContactList {
                contacts,
                searchable,
                ..
            } => {
                render_contact_list(f, chunk, contacts, *searchable, is_focused, state, i, theme);
            }
            Component::SettingsGroup { label, items, .. } => {
                render_settings_group(f, chunk, label, items, is_focused, state, i, theme);
            }
            Component::ActionList { items, .. } => {
                render_action_list(f, chunk, items, is_focused, state, i, theme);
            }
            Component::StatusIndicator {
                icon,
                title,
                detail,
                status,
                ..
            } => {
                render_status_indicator(
                    f,
                    chunk,
                    icon.as_deref(),
                    title,
                    detail.as_deref(),
                    status,
                    theme,
                );
            }
            Component::PinInput {
                id,
                label,
                length,
                filled,
                masked,
                validation_error,
            } => {
                let error = state
                    .validation_error_for(id)
                    .or(validation_error.as_deref());
                render_pin_input(
                    f, chunk, label, *length, *filled, *masked, error, is_focused, theme,
                );
            }
            Component::QrCode {
                data, mode, label, ..
            } => {
                render_qr_code(f, chunk, data, mode, label.as_deref(), theme);
            }
            Component::ConfirmationDialog {
                title,
                message,
                confirm_text,
                destructive,
                ..
            } => {
                render_confirmation_dialog(
                    f,
                    chunk,
                    title,
                    message,
                    confirm_text,
                    *destructive,
                    theme,
                );
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

/// Render a contact list component.
#[allow(clippy::too_many_arguments)]
fn render_contact_list(
    f: &mut Frame,
    area: Rect,
    contacts: &[vauchi_core::ui::ContactItem],
    _searchable: bool,
    is_focused: bool,
    state: &ScreenRenderState,
    component_idx: usize,
    theme: &TuiTheme,
) {
    use ratatui::widgets::{List, ListItem};

    let selected = state.selection_for(component_idx);

    let items: Vec<ListItem> = contacts
        .iter()
        .enumerate()
        .map(|(i, contact)| {
            let line = if let Some(sub) = &contact.subtitle {
                format!(" {} {}  {}", contact.avatar_initials, contact.name, sub)
            } else {
                format!(" {} {}", contact.avatar_initials, contact.name)
            };
            let style = if i == selected && is_focused {
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };
            ListItem::new(line).style(style)
        })
        .collect();

    if items.is_empty() {
        let empty = Paragraph::new("  No contacts yet. Use Exchange to add one.")
            .style(Style::default().fg(theme.fg_secondary))
            .block(
                Block::default()
                    .title(" Contacts ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border)),
            );
        f.render_widget(empty, area);
    } else {
        let border_color = if is_focused {
            theme.accent
        } else {
            theme.border
        };
        let list = List::new(items).block(
            Block::default()
                .title(" Contacts ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        );
        f.render_widget(list, area);
    }
}

/// Render a settings group component.
#[allow(clippy::too_many_arguments)]
fn render_settings_group(
    f: &mut Frame,
    area: Rect,
    label: &str,
    items: &[vauchi_core::ui::SettingsItem],
    is_focused: bool,
    state: &ScreenRenderState,
    component_idx: usize,
    theme: &TuiTheme,
) {
    use ratatui::widgets::{List, ListItem};

    let selected = state.selection_for(component_idx);

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let right = match &item.kind {
                SettingsItemKind::Toggle { enabled } => {
                    if *enabled { "[x]" } else { "[ ]" }.to_string()
                }
                SettingsItemKind::Value { value } => value.clone(),
                SettingsItemKind::Link { detail } => detail.as_deref().unwrap_or("→").to_string(),
                SettingsItemKind::Destructive { label } => label.clone(),
            };

            let text = format!("  {}  {}", item.label, right);
            let style = if i == selected && is_focused {
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else if matches!(item.kind, SettingsItemKind::Destructive { .. }) {
                Style::default().fg(theme.error)
            } else {
                Style::default().fg(theme.fg)
            };
            ListItem::new(text).style(style)
        })
        .collect();

    let border_color = if is_focused {
        theme.accent
    } else {
        theme.border
    };
    let list = List::new(list_items).block(
        Block::default()
            .title(format!(" {} ", label))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    );
    f.render_widget(list, area);
}

/// Render an action list component.
#[allow(clippy::too_many_arguments)]
fn render_action_list(
    f: &mut Frame,
    area: Rect,
    items: &[vauchi_core::ui::ActionListItem],
    is_focused: bool,
    state: &ScreenRenderState,
    component_idx: usize,
    theme: &TuiTheme,
) {
    use ratatui::widgets::{List, ListItem};

    let selected = state.selection_for(component_idx);

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let icon = item.icon.as_deref().unwrap_or("•");
            let detail = item
                .detail
                .as_ref()
                .map(|d| format!("  {}", d))
                .unwrap_or_default();
            let text = format!("  {} {}{}", icon, item.label, detail);
            let style = if i == selected && is_focused {
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };
            ListItem::new(text).style(style)
        })
        .collect();

    let border_color = if is_focused {
        theme.accent
    } else {
        theme.border
    };
    let list = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    );
    f.render_widget(list, area);
}

/// Render a status indicator component.
fn render_status_indicator(
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
fn render_pin_input(
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
fn render_qr_code(
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
    };

    let content = match mode {
        QrMode::Display => {
            // In a real terminal, we'd render actual QR. For now, show data info.
            let truncated = if data.len() > 40 {
                format!("{}...", &data[..40])
            } else {
                data.to_string()
            };
            let mut lines = vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  ┌──────────────┐",
                    Style::default().fg(theme.fg_secondary),
                )),
                Line::from(Span::styled(
                    "  │  ▄▄▄▄  ▄▄▄▄ │",
                    Style::default().fg(theme.fg),
                )),
                Line::from(Span::styled(
                    "  │  █  █  █  █ │",
                    Style::default().fg(theme.fg),
                )),
                Line::from(Span::styled(
                    "  │  ▀▀▀▀  ▀▀▀▀ │",
                    Style::default().fg(theme.fg),
                )),
                Line::from(Span::styled(
                    "  └──────────────┘",
                    Style::default().fg(theme.fg_secondary),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    format!("  {}", truncated),
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
fn render_confirmation_dialog(
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

/// Public wrapper for rendering a destructive confirmation dialog overlay.
pub fn render_delete_confirmation(
    f: &mut Frame,
    area: Rect,
    title: &str,
    message: &str,
    theme: &TuiTheme,
) {
    render_confirmation_dialog(f, area, title, message, "Delete", true, theme);
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
        "scan" => "S",
        "enable" | "disable" | "toggle" => "t",
        "toggle_view" => "Enter",
        id if id.starts_with("filter_group") => "g",
        _ => "Enter",
    }
}

// INLINE_TEST_REQUIRED: tests need access to pub(crate) screen_renderer internals
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_state_default() {
        let state = ScreenRenderState::default();
        assert_eq!(state.focused_component, 0);
        assert!(state.component_selections.is_empty());
        assert!(state.validation_errors.is_empty());
    }

    #[test]
    fn test_render_state_ensure_capacity() {
        let mut state = ScreenRenderState::default();
        state.ensure_capacity(3);
        assert_eq!(state.component_selections.len(), 3);
        assert_eq!(state.selection_for(0), 0);
        assert_eq!(state.selection_for(1), 0);
        assert_eq!(state.selection_for(2), 0);
    }

    #[test]
    fn test_render_state_selection_for_out_of_bounds() {
        let state = ScreenRenderState::default();
        assert_eq!(state.selection_for(99), 0);
    }

    #[test]
    fn test_render_state_validation_errors() {
        let mut state = ScreenRenderState::default();

        // No error initially
        assert!(state.validation_error_for("name").is_none());

        // Set an error
        state.set_validation_error("name".to_string(), "Required".to_string());
        assert_eq!(state.validation_error_for("name"), Some("Required"));

        // Update the error
        state.set_validation_error("name".to_string(), "Too short".to_string());
        assert_eq!(state.validation_error_for("name"), Some("Too short"));
        assert_eq!(state.validation_errors.len(), 1);

        // Clear the error
        state.clear_validation_error("name");
        assert!(state.validation_error_for("name").is_none());
    }

    #[test]
    fn test_render_state_clear_all_errors() {
        let mut state = ScreenRenderState::default();
        state.set_validation_error("a".to_string(), "err1".to_string());
        state.set_validation_error("b".to_string(), "err2".to_string());
        assert_eq!(state.validation_errors.len(), 2);

        state.clear_all_errors();
        assert!(state.validation_errors.is_empty());
    }
}
