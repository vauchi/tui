// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! FieldList widget — renders a `Component::FieldList` as a Ratatui widget.
//!
//! Shows table rows with field type, label, value, and optional visibility chips.
//! `ReadOnly` mode: no visibility column. `ShowHide` mode: "Shown"/"Hidden".
//! `PerGroup` mode: group names.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Row, Table};

use crate::theme::TuiTheme;
use vauchi_app::ui::{FieldDisplay, UiFieldVisibility, VisibilityMode};

/// State needed to render a field list component.
pub struct FieldListWidget<'a> {
    pub fields: &'a [FieldDisplay],
    pub visibility_mode: &'a VisibilityMode,
    pub available_groups: &'a [String],
    pub selected_index: usize,
    pub focused: bool,
    pub theme: &'a TuiTheme,
}

impl<'a> FieldListWidget<'a> {
    /// Render the field list into the given area.
    pub fn render(self, f: &mut Frame, area: Rect) {
        if self.fields.is_empty() {
            let empty = ratatui::widgets::Paragraph::new("  No fields added yet.")
                .style(Style::default().fg(self.theme.fg_secondary))
                .block(
                    Block::default()
                        .title(" Fields ")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(self.theme.border)),
                );
            f.render_widget(empty, area);
            return;
        }

        let is_read_only = matches!(self.visibility_mode, VisibilityMode::ReadOnly);

        let header = if is_read_only {
            Row::new(vec!["Type", "Label", "Value"])
        } else {
            Row::new(vec!["Type", "Label", "Value", "Visibility"])
        }
        .style(
            Style::default()
                .fg(self.theme.accent)
                .add_modifier(Modifier::BOLD),
        )
        .bottom_margin(1);

        let rows: Vec<Row> = self
            .fields
            .iter()
            .enumerate()
            .map(|(i, field)| {
                let style = if i == self.selected_index && self.focused {
                    Style::default()
                        .fg(self.theme.bg)
                        .bg(self.theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else if is_read_only {
                    Style::default().fg(self.theme.fg)
                } else {
                    let vis_color = match &field.visibility {
                        UiFieldVisibility::Shown => self.theme.success,
                        UiFieldVisibility::Hidden => self.theme.fg_secondary,
                        UiFieldVisibility::Groups(g) if g.is_empty() => self.theme.warning,
                        _ => self.theme.fg,
                    };
                    Style::default().fg(vis_color)
                };

                if is_read_only {
                    Row::new(vec![
                        field.field_type.clone(),
                        field.label.clone(),
                        field.value.clone(),
                    ])
                    .style(style)
                } else {
                    let vis_text = match &field.visibility {
                        UiFieldVisibility::Shown => "Shown".to_string(),
                        UiFieldVisibility::Hidden => "Hidden".to_string(),
                        UiFieldVisibility::Groups(groups) => {
                            if groups.is_empty() {
                                "No groups".to_string()
                            } else {
                                groups.join(", ")
                            }
                        }
                    };
                    Row::new(vec![
                        field.field_type.clone(),
                        field.label.clone(),
                        field.value.clone(),
                        vis_text,
                    ])
                    .style(style)
                }
            })
            .collect();

        let border_style = if self.focused {
            Style::default().fg(self.theme.accent)
        } else {
            Style::default().fg(self.theme.border)
        };

        let title = match self.visibility_mode {
            VisibilityMode::ReadOnly => " Fields ".to_string(),
            VisibilityMode::ShowHide => " Fields (Show/Hide) ".to_string(),
            VisibilityMode::PerGroup => {
                if self.available_groups.is_empty() {
                    " Fields (Per Group) ".to_string()
                } else {
                    format!(" Fields ({}) ", self.available_groups.join(", "))
                }
            }
        };

        if is_read_only {
            let widths = [
                Constraint::Length(10),
                Constraint::Length(15),
                Constraint::Min(15),
            ];
            let table = Table::new(rows, widths).header(header).block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style),
            );
            f.render_widget(table, area);
        } else {
            let widths = [
                Constraint::Length(10),
                Constraint::Length(15),
                Constraint::Min(15),
                Constraint::Length(18),
            ];
            let table = Table::new(rows, widths).header(header).block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(border_style),
            );
            f.render_widget(table, area);
        }
    }
}
