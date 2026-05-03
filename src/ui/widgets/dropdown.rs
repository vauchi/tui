// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Dropdown widget — renders a `Component::Dropdown` as a single-row
//! `Label: Selected ▼` line. Focused rows highlight the whole row.

use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::theme::TuiTheme;
use vauchi_app::ui::DropdownOption;

const NONE_PLACEHOLDER: &str = "—";

/// State needed to render a dropdown component as a single row.
pub struct DropdownWidget<'a> {
    pub label: &'a str,
    pub selected: Option<&'a str>,
    pub options: &'a [DropdownOption],
    pub focused: bool,
    pub theme: &'a TuiTheme,
}

impl<'a> DropdownWidget<'a> {
    /// Render the dropdown as a single line into the given area.
    pub fn render(self, f: &mut Frame, area: Rect) {
        let selected_label = self
            .selected
            .and_then(|sel_id| self.options.iter().find(|o| o.id == sel_id))
            .map(|o| o.label.as_str())
            .unwrap_or(NONE_PLACEHOLDER);

        let text = format!("  {}: {} ▼", self.label, selected_label);

        let style = if self.focused {
            Style::default()
                .fg(self.theme.bg)
                .bg(self.theme.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.theme.fg)
        };

        f.render_widget(Paragraph::new(text).style(style), area);
    }
}

// INLINE_TEST_REQUIRED: tests construct DropdownWidget which is pub(crate) — must live inside the crate
#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn theme() -> TuiTheme {
        TuiTheme::default()
    }

    fn render_to_buffer<F>(width: u16, height: u16, build: F) -> Buffer
    where
        F: FnOnce(&mut Frame),
    {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(build).unwrap();
        terminal.backend().buffer().clone()
    }

    fn buffer_text(buf: &Buffer) -> String {
        let mut out = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    // @internal
    #[test]
    fn renders_label_selected_label_and_caret() {
        let theme = theme();
        let options = vec![
            DropdownOption {
                id: "dark".into(),
                label: "Dark".into(),
            },
            DropdownOption {
                id: "light".into(),
                label: "Light".into(),
            },
        ];
        let buf = render_to_buffer(40, 1, |f| {
            DropdownWidget {
                label: "Theme",
                selected: Some("dark"),
                options: &options,
                focused: false,
                theme: &theme,
            }
            .render(f, f.area());
        });

        let text = buffer_text(&buf);
        assert!(text.contains("Theme"), "expected label, got: {text:?}");
        assert!(
            text.contains("Dark"),
            "expected selected label, got: {text:?}"
        );
        assert!(text.contains("▼"), "expected caret, got: {text:?}");
    }

    // @internal
    #[test]
    fn renders_placeholder_when_nothing_selected() {
        let theme = theme();
        let options = vec![DropdownOption {
            id: "dark".into(),
            label: "Dark".into(),
        }];
        let buf = render_to_buffer(40, 1, |f| {
            DropdownWidget {
                label: "Theme",
                selected: None,
                options: &options,
                focused: false,
                theme: &theme,
            }
            .render(f, f.area());
        });

        let text = buffer_text(&buf);
        assert!(text.contains("Theme"));
        assert!(
            text.contains(NONE_PLACEHOLDER),
            "expected placeholder for unselected: {text:?}"
        );
    }

    // @internal
    #[test]
    fn renders_placeholder_when_selected_id_not_in_options() {
        let theme = theme();
        let options = vec![DropdownOption {
            id: "dark".into(),
            label: "Dark".into(),
        }];
        let buf = render_to_buffer(40, 1, |f| {
            DropdownWidget {
                label: "Theme",
                selected: Some("system"),
                options: &options,
                focused: false,
                theme: &theme,
            }
            .render(f, f.area());
        });

        let text = buffer_text(&buf);
        assert!(text.contains(NONE_PLACEHOLDER));
    }

    // @internal
    #[test]
    fn focused_row_uses_accent_background() {
        let theme = theme();
        let options = vec![DropdownOption {
            id: "dark".into(),
            label: "Dark".into(),
        }];
        let buf = render_to_buffer(40, 1, |f| {
            DropdownWidget {
                label: "Theme",
                selected: Some("dark"),
                options: &options,
                focused: true,
                theme: &theme,
            }
            .render(f, f.area());
        });

        // Cell over the label glyph — must paint the accent background.
        let cell_bg = buf[(2, 0)].bg;
        assert_eq!(
            cell_bg, theme.accent,
            "focused row must paint accent background"
        );
    }

    // @internal
    #[test]
    fn unfocused_row_uses_default_background() {
        let theme = theme();
        let options = vec![DropdownOption {
            id: "dark".into(),
            label: "Dark".into(),
        }];
        let buf = render_to_buffer(40, 1, |f| {
            DropdownWidget {
                label: "Theme",
                selected: Some("dark"),
                options: &options,
                focused: false,
                theme: &theme,
            }
            .render(f, f.area());
        });

        let cell_bg = buf[(2, 0)].bg;
        assert_ne!(
            cell_bg, theme.accent,
            "unfocused row must not paint accent background"
        );
    }
}
