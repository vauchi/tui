// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! ScreenRenderer — maps a `ScreenModel` to a full terminal layout.
//!
//! Renders: progress bar at top, title/subtitle, components in order,
//! and action hints at the bottom.

mod render_lists;
mod render_misc;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::theme::TuiTheme;
use vauchi_app::DesignTokens;
use vauchi_app::ui::{Component, ScreenModel, TextStyle};

use super::card_preview::CardPreviewWidget;
use super::dropdown::DropdownWidget;
use super::field_list::FieldListWidget;
use super::info_panel::InfoPanelWidget;
use super::text_input::TextInputWidget;
use super::toggle_list::ToggleListWidget;

pub use render_misc::action_key_hint_pub;

/// Tracks focus and selection state for the screen renderer.
#[derive(Default)]
pub struct ScreenRenderState {
    /// Index of the currently focused component (for keyboard navigation).
    pub focused_component: usize,
    /// Per-component selection index (e.g., which item is highlighted in a list).
    pub component_selections: Vec<usize>,
    /// Per-component scroll offset for lists longer than the visible area.
    pub scroll_offsets: Vec<usize>,
    /// Validation errors keyed by component_id.
    pub validation_errors: Vec<(String, String)>,
    /// Whether content zone has focus (false when ActionBar/NavBar is focused).
    pub content_has_focus: bool,
}

impl ScreenRenderState {
    /// Ensure the selections and scroll vectors have enough entries for the given component count.
    pub fn ensure_capacity(&mut self, component_count: usize) {
        while self.component_selections.len() < component_count {
            self.component_selections.push(0);
        }
        while self.scroll_offsets.len() < component_count {
            self.scroll_offsets.push(0);
        }
    }

    /// Get the scroll offset for a given component.
    pub fn scroll_for(&self, index: usize) -> usize {
        self.scroll_offsets.get(index).copied().unwrap_or(0)
    }

    /// Adjust scroll offset to keep the selection visible within `visible_count` items.
    pub fn ensure_visible(&mut self, component_idx: usize, visible_count: usize) {
        self.ensure_capacity(component_idx + 1);
        let sel = self.selection_for(component_idx);
        let scroll = self.scroll_offsets[component_idx];
        if sel < scroll {
            self.scroll_offsets[component_idx] = sel;
        } else if sel >= scroll + visible_count {
            self.scroll_offsets[component_idx] = sel - visible_count + 1;
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

// ── Token-to-terminal conversion ──────────────────────────────────
//
// Design tokens are pixel-based. Terminal cells are ~8px wide, ~16px tall.
// These helpers convert token values to character cells.
// Terminal line height divisor (one line ≈ 16 px).
const LINE_HEIGHT_PX: u16 = 16;

/// Component header/footer padding in terminal lines, derived from spacing.sm.
fn component_padding_lines(tokens: &DesignTokens) -> u16 {
    (tokens.spacing.sm / LINE_HEIGHT_PX).max(1)
}

/// Minimum content area in terminal lines, derived from spacing.xl.
fn content_min_lines(tokens: &DesignTokens) -> u16 {
    (tokens.spacing.xl / LINE_HEIGHT_PX).max(3)
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

    let mut constraints = Vec::new();

    // Progress bar
    if has_progress {
        constraints.push(Constraint::Length(1));
    }

    // Title
    constraints.push(Constraint::Length(if has_subtitle { 3 } else { 2 }));

    // Components area (flexible, token-derived minimum)
    constraints.push(Constraint::Min(content_min_lines(&screen.tokens)));

    // (Action hints removed — shown in the persistent action bar instead)

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let mut chunk_idx = 0;

    // Progress bar
    if let Some(progress) = &screen.progress {
        render_misc::render_progress(f, chunks[chunk_idx], progress, theme);
        chunk_idx += 1;
    }

    // Title and subtitle
    render_misc::render_title(
        f,
        chunks[chunk_idx],
        &screen.title,
        screen.subtitle.as_deref(),
        theme,
    );
    chunk_idx += 1;

    // Components
    render_components(
        f,
        chunks[chunk_idx],
        &screen.components,
        state,
        theme,
        &screen.tokens,
    );

    // (Action hints removed — shown in the persistent action bar instead)
}

/// Render all components in the screen model.
fn render_components(
    f: &mut Frame,
    area: Rect,
    components: &[Component],
    state: &ScreenRenderState,
    theme: &TuiTheme,
    tokens: &DesignTokens,
) {
    if components.is_empty() {
        return;
    }

    // Component chrome: 1 line structural + token-derived gap.
    // With default tokens (spacing.sm=8, line_height=16): chrome = 1 + 1 = 2.
    let chrome = 1 + component_padding_lines(tokens);

    // Build constraints: each component gets a portion of the available space.
    // `chrome` replaces the former hardcoded `+ 2` — now token-derived.
    let constraints: Vec<Constraint> = components
        .iter()
        .map(|c| match c {
            Component::Text { .. } => Constraint::Length(chrome),
            Component::TextInput { .. } => Constraint::Length(3 + chrome),
            Component::ToggleList { items, .. } => {
                let item_lines: usize = items
                    .iter()
                    .map(|i| if i.subtitle.is_some() { 2 } else { 1 })
                    .sum();
                Constraint::Length((item_lines as u16 + chrome).min(area.height / 2))
            }
            Component::FieldList { fields, .. } => {
                Constraint::Length((fields.len() as u16 + 2 + chrome).min(area.height / 2))
            }
            Component::Preview { variants, .. } => {
                let extra = if variants.is_empty() { 0 } else { chrome };
                Constraint::Min(8 + extra)
            }
            Component::InfoPanel { items, .. } => {
                Constraint::Length((items.len() as u16 * 3 + chrome).min(area.height / 2))
            }
            Component::List { items, .. } => {
                Constraint::Length((items.len() as u16 + chrome).min(area.height / 2).max(4))
            }
            Component::SettingsGroup { items, .. } => {
                Constraint::Length((items.len() as u16 + chrome).min(area.height / 2))
            }
            Component::ActionList { items, .. } => {
                Constraint::Length((items.len() as u16 + chrome).min(area.height / 2))
            }
            Component::StatusIndicator { .. } => Constraint::Length(3),
            Component::PinInput { .. } => Constraint::Length(5),
            Component::QrCode { .. } => Constraint::Min(14),
            Component::Divider => Constraint::Length(1),
            Component::InlineConfirm { .. } => Constraint::Length(4),
            Component::EditableText { .. } => Constraint::Length(3),
            Component::Banner { .. } => Constraint::Length(2),
            Component::Dropdown { .. } => Constraint::Length(1),
            Component::AvatarPreview { .. } => Constraint::Length(1),
            Component::Slider { .. } => Constraint::Length(1),
            Component::Indicator { .. } => Constraint::Length(1),
            Component::SectionedActionList { sections, .. } => {
                let item_lines: usize = sections
                    .iter()
                    .map(|s| 1 /* header */ + s.items.len())
                    .sum();
                Constraint::Length(((item_lines as u16) + chrome).max(4))
            }
            _ => Constraint::Length(1),
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
            Component::Preview {
                name,
                visible_fields,
                variants,
                selected_variant,
                ..
            } => {
                // ADR-021/043 + 2026-05-21 audit: render the pre-filtered
                // `visible_fields` list from core. The raw `fields` list
                // (still on the wire pending core removal) contains
                // Hidden-visibility entries and must not be rendered.
                CardPreviewWidget {
                    name,
                    fields: visible_fields,
                    variants,
                    selected_variant: selected_variant.as_deref(),
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
            Component::List {
                items, searchable, ..
            } => {
                render_lists::render_contact_list(
                    f,
                    chunk,
                    items,
                    *searchable,
                    is_focused,
                    state,
                    i,
                    theme,
                );
            }
            Component::SettingsGroup { label, items, .. } => {
                render_lists::render_settings_group(
                    f, chunk, label, items, is_focused, state, i, theme,
                );
            }
            Component::ActionList { items, .. } => {
                render_lists::render_action_list(f, chunk, items, is_focused, state, i, theme);
            }
            Component::StatusIndicator {
                icon,
                title,
                detail,
                status,
                ..
            } => {
                render_misc::render_status_indicator(
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
                ..
            } => {
                let error = state
                    .validation_error_for(id)
                    .or(validation_error.as_deref());
                render_misc::render_pin_input(
                    f, chunk, label, *length, *filled, *masked, error, is_focused, theme,
                );
            }
            Component::QrCode {
                data, mode, label, ..
            } => {
                render_misc::render_qr_code(f, chunk, data, mode, label.as_deref(), theme);
            }
            Component::Divider => {
                let divider = Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::default().fg(theme.border));
                f.render_widget(divider, chunk);
            }
            Component::InlineConfirm {
                warning,
                confirm_text,
                cancel_text,
                destructive,
                ..
            } => {
                let text = format!("  ⚠ {warning}  [Enter] {confirm_text}  [Esc] {cancel_text}");
                let color = if *destructive {
                    theme.error
                } else {
                    theme.accent
                };
                let para = Paragraph::new(text).style(Style::default().fg(color));
                f.render_widget(para, chunk);
            }
            Component::EditableText {
                label,
                value,
                editing,
                ..
            } => {
                let indicator = if *editing { "▎" } else { "" };
                let text = format!("  {label}: {value}{indicator}");
                let style = if is_focused {
                    Style::default().fg(theme.accent)
                } else {
                    Style::default().fg(theme.fg)
                };
                let para = Paragraph::new(text).style(style);
                f.render_widget(para, chunk);
            }
            Component::Dropdown {
                label,
                selected,
                options,
                ..
            } => {
                DropdownWidget {
                    label,
                    selected: selected.as_deref(),
                    options,
                    focused: is_focused,
                    theme,
                }
                .render(f, chunk);
            }
            Component::Banner {
                text,
                action_label,
                action_id: _,
                ..
            } => {
                let line = Line::from(vec![
                    Span::styled(
                        format!("  {text}"),
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD),
                    ),
                    if !action_label.is_empty() {
                        Span::styled(
                            format!("  [{action_label}]"),
                            Style::default()
                                .fg(theme.accent)
                                .add_modifier(Modifier::UNDERLINED),
                        )
                    } else {
                        Span::raw("")
                    },
                ]);
                let para = Paragraph::new(line);
                f.render_widget(para, chunk);
            }
            Component::AvatarPreview { initials, .. } => {
                let text = format!("  [Avatar: {}]", initials);
                f.render_widget(
                    Paragraph::new(text).style(Style::default().fg(theme.fg)),
                    chunk,
                );
            }
            Component::Slider { label, value, .. } => {
                let text = format!("  {}: {:.2}", label, value);
                f.render_widget(
                    Paragraph::new(text).style(Style::default().fg(theme.fg)),
                    chunk,
                );
            }
            Component::Indicator {
                label,
                kind,
                action_id,
                ..
            } => {
                render_misc::render_indicator(
                    f,
                    chunk,
                    label,
                    kind,
                    action_id.is_some(),
                    is_focused,
                    theme,
                );
            }
            Component::SectionedActionList { sections, .. } => {
                render_lists::render_sectioned_action_list(
                    f, chunk, sections, is_focused, state, i, theme,
                );
            }
            _ => {
                // Future Component variants — caught by CC-22 reachability
                // test (F5 in 2026-05-03-tui-cli-next-work-audit.md).
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
        _ => Style::default().fg(theme.fg),
    };

    let para = Paragraph::new(content).style(text_style);
    f.render_widget(para, area);
}

// INLINE_TEST_REQUIRED: tests need access to pub(crate) screen_renderer
// internals — extracted to sibling `tests.rs` per file-size baseline
// (CC file-size hard cap 1000 lines).
#[cfg(test)]
mod tests;
