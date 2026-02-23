// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Theme System for TUI
//!
//! Provides Ratatui color conversion from vauchi-core themes.

use ratatui::style::Color;
use vauchi_core::theme::{default_theme, Theme, ThemeColors, ThemeMode};

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
        TuiTheme::from(&default_theme())
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
///
/// Returns the default theme if the ID matches, otherwise None.
/// Full theme catalog requires loading themes.json via `load_themes_from_json`.
pub fn get_tui_theme(id: &str) -> Option<TuiTheme> {
    let dt = default_theme();
    if dt.id == id {
        Some(TuiTheme::from(&dt))
    } else {
        None
    }
}

/// Get the default TUI theme based on preference.
///
/// Currently returns Catppuccin Mocha (dark) regardless of preference.
/// Full light/dark selection requires loading themes.json.
pub fn get_default_tui_theme(_prefer_dark: bool) -> TuiTheme {
    TuiTheme::from(&default_theme())
}

/// Get all available themes as a list of (id, name, mode).
///
/// Returns the default theme. Full catalog requires loading themes.json.
pub fn list_themes() -> Vec<(String, String, ThemeMode)> {
    let dt = default_theme();
    vec![(dt.id, dt.name, dt.mode)]
}

// INLINE_TEST_REQUIRED: Tests exercise private hex_to_color helper and TuiTheme conversions
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
        // Should match Catppuccin Mocha bg (#1e1e2e)
        assert_eq!(theme.bg, Color::Rgb(30, 30, 46));
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
