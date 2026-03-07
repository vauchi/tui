// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! CardPreview widget — renders a `Component::CardPreview` as a Ratatui widget.
//!
//! Shows a bordered box with card content. Tab or arrow keys switch between
//! group views when available.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};

use crate::theme::TuiTheme;
use vauchi_core::ui::{FieldDisplay, GroupCardView};

/// State needed to render a card preview component.
pub struct CardPreviewWidget<'a> {
    pub name: &'a str,
    pub fields: &'a [FieldDisplay],
    pub group_views: &'a [GroupCardView],
    pub selected_group: Option<&'a str>,
    pub theme: &'a TuiTheme,
}

impl<'a> CardPreviewWidget<'a> {
    /// Render the card preview into the given area.
    pub fn render(self, f: &mut Frame, area: Rect) {
        let has_groups = !self.group_views.is_empty();

        let chunks = if has_groups {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2), // Group tabs
                    Constraint::Min(5),    // Card content
                ])
                .split(area)
        } else {
            Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(5)])
                .split(area)
        };

        // Group tabs (if applicable)
        let (card_area, display_name, display_fields) = if has_groups {
            let tab_titles: Vec<&str> = self
                .group_views
                .iter()
                .map(|g| g.group_name.as_str())
                .collect();

            let selected_idx = self
                .selected_group
                .and_then(|sg| tab_titles.iter().position(|t| *t == sg))
                .unwrap_or(0);

            let tabs = Tabs::new(
                tab_titles
                    .iter()
                    .map(|t| Line::from(*t))
                    .collect::<Vec<_>>(),
            )
            .select(selected_idx)
            .style(Style::default().fg(self.theme.fg_secondary))
            .highlight_style(
                Style::default()
                    .fg(self.theme.accent)
                    .add_modifier(Modifier::BOLD),
            )
            .divider("|");

            f.render_widget(tabs, chunks[0]);

            let view = &self.group_views[selected_idx];
            (
                chunks[1],
                view.display_name.as_str(),
                view.visible_fields.as_slice(),
            )
        } else {
            (chunks[0], self.name, self.fields)
        };

        // Card content
        let mut lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  {}", display_name),
                Style::default()
                    .fg(self.theme.accent)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];

        if display_fields.is_empty() {
            lines.push(Line::from(Span::styled(
                "  No visible fields",
                Style::default().fg(self.theme.fg_secondary),
            )));
        } else {
            for field in display_fields {
                let icon = field_icon(&field.field_type);
                lines.push(Line::from(format!(
                    "  {} {} ({}): {}",
                    icon, field.label, field.field_type, field.value
                )));
            }
        }

        lines.push(Line::from(""));

        let card = Paragraph::new(lines).block(
            Block::default()
                .title(" Card Preview ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(self.theme.accent)),
        );

        f.render_widget(card, card_area);
    }
}

/// Map field type to a text icon.
fn field_icon(field_type: &str) -> &'static str {
    match field_type {
        "Email" => "[E]",
        "Phone" => "[P]",
        "Website" => "[W]",
        "Address" => "[A]",
        "Social" => "[S]",
        _ => "[?]",
    }
}
