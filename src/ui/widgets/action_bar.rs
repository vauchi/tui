// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Context-Sensitive Action Bar Widget
//!
//! Displays keybindings and their corresponding actions.
//! Content changes based on the current screen.

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::theme::TuiTheme;

/// A single action item in the action bar.
#[derive(Clone, Debug)]
pub struct ActionItem {
    pub key: String,
    pub label: String,
}

impl ActionItem {
    pub fn new(key: &str, label: &str) -> Self {
        Self {
            key: key.to_string(),
            label: label.to_string(),
        }
    }
}

/// Context-sensitive action bar widget.
pub struct ActionBarWidget<'a> {
    pub items: &'a [ActionItem],
    /// Index of the currently focused item (None if bar is not focused).
    pub focused_index: Option<usize>,
    pub theme: &'a TuiTheme,
}

impl<'a> ActionBarWidget<'a> {
    pub fn render(&self, f: &mut Frame, area: Rect) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let mut spans: Vec<Span> = Vec::new();
        let mut used_width: usize = 0;

        for (i, item) in self.items.iter().enumerate() {
            // Format: "[key] label" with 2-char spacing between items
            let text = format!("[{}] {}", &item.key, item.label);
            let text_width = text.chars().count();
            let spacing = if i > 0 { 2 } else { 0 };

            if used_width + spacing + text_width > area.width as usize {
                break;
            }

            if i > 0 {
                spans.push(Span::raw("  "));
            }

            let is_focused = self.focused_index == Some(i);
            let style = if is_focused {
                Style::default()
                    .fg(self.theme.bg)
                    .bg(self.theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(self.theme.fg_secondary)
            };

            used_width += spacing + text_width;
            spans.push(Span::styled(text, style));
        }

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line).style(Style::default().bg(self.theme.bg));
        f.render_widget(paragraph, area);
    }
}
