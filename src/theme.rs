// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Theme System for TUI
//!
//! Provides Ratatui color conversion from vauchi-core themes.

use ratatui::style::Color;
use vauchi_core::theme::{get_bundled_themes, get_theme_by_id, Theme, ThemeColors, ThemeMode};

/// TUI-compatible theme with Ratatui colors.
#[derive(Debug, Clone)]
pub struct TuiTheme {
    /// Primary background color
    pub bg: Color,
    /// Secondary background color
    pub bg_secondary: Color,
    /// Primary text color
    pub fg: Color,
    /// Secondary text color
    pub fg_secondary: Color,
    /// Accent color
    pub accent: Color,
    /// Success color
    pub success: Color,
    /// Error color
    pub error: Color,
    /// Warning color
    pub warning: Color,
    /// Border color
    pub border: Color,
}

impl Default for TuiTheme {
    fn default() -> Self {
        // Default dark theme colors
        Self {
            bg: Color::Rgb(26, 26, 46),
            bg_secondary: Color::Rgb(22, 33, 62),
            fg: Color::Rgb(238, 238, 238),
            fg_secondary: Color::Rgb(160, 160, 160),
            accent: Color::Rgb(79, 195, 247),
            success: Color::Rgb(76, 175, 80),
            error: Color::Rgb(244, 67, 54),
            warning: Color::Rgb(255, 152, 0),
            border: Color::Rgb(51, 51, 51),
        }
    }
}

impl From<&Theme> for TuiTheme {
    fn from(theme: &Theme) -> Self {
        Self {
            bg: hex_to_color(&theme.colors.bg_primary),
            bg_secondary: hex_to_color(&theme.colors.bg_secondary),
            fg: hex_to_color(&theme.colors.text_primary),
            fg_secondary: hex_to_color(&theme.colors.text_secondary),
            accent: hex_to_color(&theme.colors.accent),
            success: hex_to_color(&theme.colors.success),
            error: hex_to_color(&theme.colors.error),
            warning: hex_to_color(&theme.colors.warning),
            border: hex_to_color(&theme.colors.border),
        }
    }
}

impl From<&ThemeColors> for TuiTheme {
    fn from(colors: &ThemeColors) -> Self {
        Self {
            bg: hex_to_color(&colors.bg_primary),
            bg_secondary: hex_to_color(&colors.bg_secondary),
            fg: hex_to_color(&colors.text_primary),
            fg_secondary: hex_to_color(&colors.text_secondary),
            accent: hex_to_color(&colors.accent),
            success: hex_to_color(&colors.success),
            error: hex_to_color(&colors.error),
            warning: hex_to_color(&colors.warning),
            border: hex_to_color(&colors.border),
        }
    }
}

/// Convert a hex color string to a Ratatui Color.
pub fn hex_to_color(hex: &str) -> Color {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return Color::White;
    }

    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(255);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(255);

    Color::Rgb(r, g, b)
}

/// Get a TUI theme by ID.
pub fn get_tui_theme(id: &str) -> Option<TuiTheme> {
    get_theme_by_id(id).map(|t| TuiTheme::from(&t))
}

/// Get the default TUI theme based on preference.
pub fn get_default_tui_theme(prefer_dark: bool) -> TuiTheme {
    let id = if prefer_dark {
        "default-dark"
    } else {
        "default-light"
    };
    get_tui_theme(id).unwrap_or_default()
}

/// Get all available themes as a list of (id, name, mode).
pub fn list_themes() -> Vec<(String, String, ThemeMode)> {
    get_bundled_themes()
        .into_iter()
        .map(|t| (t.id, t.name, t.mode))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_to_color_valid() {
        let color = hex_to_color("#1e1e2e");
        assert_eq!(color, Color::Rgb(30, 30, 46));
    }

    #[test]
    fn test_hex_to_color_no_hash() {
        let color = hex_to_color("ffffff");
        assert_eq!(color, Color::Rgb(255, 255, 255));
    }

    #[test]
    fn test_default_theme() {
        let theme = TuiTheme::default();
        // Should have some colors
        assert!(matches!(theme.bg, Color::Rgb(_, _, _)));
    }

    #[test]
    fn test_get_tui_theme() {
        let theme = get_tui_theme("catppuccin-mocha");
        assert!(theme.is_some());
    }

    #[test]
    fn test_list_themes() {
        let themes = list_themes();
        assert!(!themes.is_empty());
    }
}
