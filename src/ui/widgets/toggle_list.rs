// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! ToggleList widget — renders a `Component::ToggleList` as a Ratatui widget.
//!
//! Displays a list with `[x]`/`[ ]` markers. Arrow keys move selection,
//! Space toggles items. Optional subtitles appear under each item.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::theme::TuiTheme;
use vauchi_core::ui::ToggleItem;

/// State needed to render a toggle list component.
pub struct ToggleListWidget<'a> {
    pub label: &'a str,
    pub items: &'a [ToggleItem],
    pub selected_index: usize,
    pub focused: bool,
    pub theme: &'a TuiTheme,
}

impl<'a> ToggleListWidget<'a> {
    /// Render the toggle list into the given area.
    pub fn render(self, f: &mut Frame, area: Rect) {
        let list_items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .flat_map(|(i, item)| {
                let check = if item.selected { "[x]" } else { "[ ]" };
                let highlight = i == self.selected_index && self.focused;

                let style = if highlight {
                    Style::default()
                        .fg(self.theme.bg)
                        .bg(self.theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else if item.selected {
                    Style::default().fg(self.theme.success)
                } else {
                    Style::default().fg(self.theme.fg)
                };

                let mut lines = vec![Line::from(Span::styled(
                    format!(" {} {}", check, item.label),
                    style,
                ))];

                if let Some(subtitle) = &item.subtitle {
                    let sub_style = if highlight {
                        Style::default().fg(self.theme.bg).bg(self.theme.accent)
                    } else {
                        Style::default().fg(self.theme.fg_secondary)
                    };
                    lines.push(Line::from(Span::styled(
                        format!("     {}", subtitle),
                        sub_style,
                    )));
                }

                // Return a single ListItem with multiple lines
                vec![ListItem::new(lines)]
            })
            .collect();

        let border_style = if self.focused {
            Style::default().fg(self.theme.accent)
        } else {
            Style::default().fg(self.theme.border)
        };

        let list = List::new(list_items).block(
            Block::default()
                .title(format!(" {} ", self.label))
                .borders(Borders::ALL)
                .border_style(border_style),
        );

        let mut state = ListState::default();
        state.select(Some(self.selected_index));
        f.render_stateful_widget(list, area, &mut state);
    }
}
