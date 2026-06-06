// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! InfoPanel widget — renders a `Component::InfoPanel` as a Ratatui widget.
//!
//! Shows a styled block with title and list items.
//! Icons are rendered as text prefixes.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::theme::TuiTheme;
use crate::ui::widgets::icon::icon_badge;
use vauchi_app::ui::InfoItem;

/// State needed to render an info panel component.
pub struct InfoPanelWidget<'a> {
    pub title: &'a str,
    pub icon: Option<&'a str>,
    pub items: &'a [InfoItem],
    pub theme: &'a TuiTheme,
}

impl<'a> InfoPanelWidget<'a> {
    /// Render the info panel into the given area.
    pub fn render(self, f: &mut Frame, area: Rect) {
        let mut lines = Vec::new();
        lines.push(Line::from(""));

        for item in self.items {
            let icon_prefix = item
                .icon
                .as_ref()
                .map(|i| format!("{} ", icon_badge(i).unwrap_or("[-]")))
                .unwrap_or_default();

            lines.push(Line::from(Span::styled(
                format!("  {}{}", icon_prefix, item.title),
                Style::default()
                    .fg(self.theme.fg)
                    .add_modifier(Modifier::BOLD),
            )));

            lines.push(Line::from(Span::styled(
                format!("  {}{}", " ".repeat(icon_prefix.len()), item.detail),
                Style::default().fg(self.theme.fg_secondary),
            )));

            lines.push(Line::from(""));
        }

        let panel_icon = self
            .icon
            .map(|i| format!("{} ", icon_badge(i).unwrap_or("[-]")))
            .unwrap_or_default();

        let panel = Paragraph::new(lines).block(
            Block::default()
                .title(format!(" {}{} ", panel_icon, self.title))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(self.theme.accent)),
        );

        f.render_widget(panel, area);
    }
}
