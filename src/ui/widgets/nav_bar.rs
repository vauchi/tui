// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Bottom Navigation Bar Widget
//!
//! Persistent bar with 5 tabs: Exchange, MyInfo, Contacts, Settings, Help.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::theme::TuiTheme;

/// A navigation item in the bottom bar.
#[derive(Clone, Debug)]
pub struct NavItem {
    pub label: &'static str,
    pub active: bool,
}

/// Bottom navigation bar widget.
pub struct NavBarWidget<'a> {
    pub items: &'a [NavItem],
    /// Index of the currently focused item (None if bar is not focused).
    pub focused_index: Option<usize>,
    pub theme: &'a TuiTheme,
}

impl<'a> NavBarWidget<'a> {
    pub fn render(&self, f: &mut Frame, area: Rect) {
        if self.items.is_empty() || area.width == 0 || area.height == 0 {
            return;
        }

        let cell_width = area.width as usize / self.items.len();
        let mut spans: Vec<Span> = Vec::new();

        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                spans.push(Span::styled("│", Style::default().fg(self.theme.border)));
            }

            let is_focused = self.focused_index == Some(i);
            let style = if is_focused {
                Style::default()
                    .fg(self.theme.bg)
                    .bg(self.theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else if item.active {
                Style::default()
                    .fg(self.theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(self.theme.fg_secondary)
            };

            let num = i + 1;
            let label = if item.active && !is_focused {
                format!("{} ★ {}", num, item.label)
            } else {
                format!("{} {}", num, item.label)
            };

            // Pad label to fill cell (minus separator)
            let sep_width = if i > 0 { 1 } else { 0 };
            let available = cell_width.saturating_sub(sep_width);
            let padded = center_pad(&label, available);

            spans.push(Span::styled(padded, style));
        }

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line).style(Style::default().bg(self.theme.bg_secondary));
        f.render_widget(paragraph, area);
    }
}

/// Center-pad a string to the given width.
fn center_pad(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        return s.chars().take(width).collect();
    }
    let left = (width - len) / 2;
    let right = width - len - left;
    format!("{}{}{}", " ".repeat(left), s, " ".repeat(right))
}
