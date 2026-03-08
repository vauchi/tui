// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! FieldList widget — renders a `Component::FieldList` as a Ratatui widget.
//!
//! Shows table rows with field type, label, value, and visibility chips.
//! In `ShowHide` mode: "Shown"/"Hidden". In `PerGroup` mode: group names.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Row, Table};

use crate::theme::TuiTheme;
use vauchi_core::ui::{FieldDisplay, UiFieldVisibility, VisibilityMode};

/// State needed to render a field list component.
pub struct FieldListWidget<'a> {
    pub fields: &'a [FieldDisplay],
    pub visibility_mode: &'a VisibilityMode,
    #[allow(dead_code)]
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

        let header = Row::new(vec!["Type", "Label", "Value", "Visibility"])
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

                let style = if i == self.selected_index && self.focused {
                    Style::default()
                        .fg(self.theme.bg)
                        .bg(self.theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else {
                    let vis_color = match &field.visibility {
                        UiFieldVisibility::Shown => self.theme.success,
                        UiFieldVisibility::Hidden => self.theme.fg_secondary,
                        UiFieldVisibility::Groups(g) if g.is_empty() => self.theme.warning,
                        _ => self.theme.fg,
                    };
                    Style::default().fg(vis_color)
                };

                Row::new(vec![
                    field.field_type.clone(),
                    field.label.clone(),
                    field.value.clone(),
                    vis_text,
                ])
                .style(style)
            })
            .collect();

        let widths = [
            Constraint::Length(10),
            Constraint::Length(15),
            Constraint::Min(15),
            Constraint::Length(18),
        ];

        let border_style = if self.focused {
            Style::default().fg(self.theme.accent)
        } else {
            Style::default().fg(self.theme.border)
        };

        let mode_label = match self.visibility_mode {
            VisibilityMode::ShowHide => "Show/Hide",
            VisibilityMode::PerGroup => "Per Group",
        };

        let table = Table::new(rows, widths).header(header).block(
            Block::default()
                .title(format!(" Fields ({}) ", mode_label))
                .borders(Borders::ALL)
                .border_style(border_style),
        );

        f.render_widget(table, area);
    }
}
