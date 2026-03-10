// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! TextInput widget — renders a `Component::TextInput` as a Ratatui widget.
//!
//! Shows a label above the input field, current value with cursor,
//! and a validation error below (in red) when present.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::theme::TuiTheme;

/// State needed to render a text input component.
pub struct TextInputWidget<'a> {
    pub label: &'a str,
    pub value: &'a str,
    pub placeholder: Option<&'a str>,
    pub validation_error: Option<&'a str>,
    pub focused: bool,
    pub theme: &'a TuiTheme,
}

impl<'a> TextInputWidget<'a> {
    /// Render the text input into the given area.
    ///
    /// Layout: label (1 line) + input box (3 lines) + error (1 line, optional).
    /// Returns the total height consumed.
    pub fn render(self, f: &mut Frame, area: Rect) -> u16 {
        let has_error = self.validation_error.is_some();
        let total_height = if has_error { 5 } else { 4 };

        if area.height < 3 {
            return 0;
        }

        let constraints = if has_error {
            vec![
                Constraint::Length(1), // Label
                Constraint::Length(3), // Input box
                Constraint::Length(1), // Error
            ]
        } else {
            vec![
                Constraint::Length(1), // Label
                Constraint::Length(3), // Input box
            ]
        };

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        // Label
        let label_style = if self.focused {
            Style::default()
                .fg(self.theme.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.theme.fg_secondary)
        };
        let label_para = Paragraph::new(self.label).style(label_style);
        f.render_widget(label_para, chunks[0]);

        // Input field with cursor
        let display_value = if self.value.is_empty() {
            if self.focused {
                "█".to_string()
            } else {
                self.placeholder.unwrap_or("").to_string()
            }
        } else if self.focused {
            format!("{}█", self.value)
        } else {
            self.value.to_string()
        };

        let border_color = if self.focused {
            if has_error {
                self.theme.error
            } else {
                self.theme.accent
            }
        } else {
            self.theme.border
        };

        let text_style = if self.value.is_empty() && !self.focused {
            Style::default().fg(self.theme.fg_secondary)
        } else {
            Style::default().fg(self.theme.fg)
        };

        // Show a "type here" hint for unfocused empty inputs
        let block_title = if self.focused {
            " ✎ editing "
        } else if self.value.is_empty() {
            " ⌨ type here "
        } else {
            ""
        };

        let input_para = Paragraph::new(display_value).style(text_style).block(
            Block::default()
                .title(block_title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        );
        f.render_widget(input_para, chunks[1]);

        // Validation error
        if let Some(error) = self.validation_error {
            let error_para = Paragraph::new(error).style(Style::default().fg(self.theme.error));
            f.render_widget(error_para, chunks[2]);
        }

        total_height
    }
}
