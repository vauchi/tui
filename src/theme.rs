// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Theme System for TUI
//!
//! Loads the full 14-theme catalog from the generated `tokens_ansi.json` at
//! compile time and converts hex colors to Ratatui `Color::Rgb` values.

use std::sync::LazyLock;

use ratatui::style::Color;
use serde::Deserialize;
use vauchi_core::theme::{ThemeMode, default_theme};

/// Embedded ANSI token mapping — compiled from `themes/generated/tokens_ansi.json`.
const TOKENS_ANSI_JSON: &[u8] = include_bytes!("../../themes/generated/tokens_ansi.json");

/// A single color entry from the ANSI tokens JSON.
#[derive(Debug, Clone, Deserialize)]
struct AnsiColorEntry {
    hex: String,
}

/// Color set from the ANSI tokens JSON.
#[derive(Debug, Clone, Deserialize)]
struct AnsiColorSet {
    #[serde(rename = "bg-primary")]
    bg_primary: AnsiColorEntry,
    #[serde(rename = "bg-secondary")]
    bg_secondary: AnsiColorEntry,
    #[serde(rename = "text-primary")]
    text_primary: AnsiColorEntry,
    #[serde(rename = "text-secondary")]
    text_secondary: AnsiColorEntry,
    accent: AnsiColorEntry,
    success: AnsiColorEntry,
    error: AnsiColorEntry,
    warning: AnsiColorEntry,
    border: AnsiColorEntry,
}

/// A single theme entry from the ANSI tokens JSON.
#[derive(Debug, Clone, Deserialize)]
struct AnsiThemeEntry {
    id: String,
    name: String,
    mode: ThemeMode,
    colors: AnsiColorSet,
}

/// Parsed theme catalog, loaded once on first access.
static THEME_CATALOG: LazyLock<Vec<AnsiThemeEntry>> = LazyLock::new(|| {
    serde_json::from_slice(TOKENS_ANSI_JSON).unwrap_or_else(|e| {
        eprintln!("Failed to parse embedded tokens_ansi.json: {e}");
        Vec::new()
    })
});

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
        let dt = default_theme();
        Self {
            bg: hex_to_color(&dt.colors.bg_primary),
            bg_secondary: hex_to_color(&dt.colors.bg_secondary),
            fg: hex_to_color(&dt.colors.text_primary),
            fg_secondary: hex_to_color(&dt.colors.text_secondary),
            accent: hex_to_color(&dt.colors.accent),
            success: hex_to_color(&dt.colors.success),
            error: hex_to_color(&dt.colors.error),
            warning: hex_to_color(&dt.colors.warning),
            border: hex_to_color(&dt.colors.border),
        }
    }
}

impl From<&AnsiThemeEntry> for TuiTheme {
    fn from(entry: &AnsiThemeEntry) -> Self {
        let c = &entry.colors;
        Self {
            bg: hex_to_color(&c.bg_primary.hex),
            bg_secondary: hex_to_color(&c.bg_secondary.hex),
            fg: hex_to_color(&c.text_primary.hex),
            fg_secondary: hex_to_color(&c.text_secondary.hex),
            accent: hex_to_color(&c.accent.hex),
            success: hex_to_color(&c.success.hex),
            error: hex_to_color(&c.error.hex),
            warning: hex_to_color(&c.warning.hex),
            border: hex_to_color(&c.border.hex),
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

/// Get a TUI theme by ID from the full catalog.
///
/// Searches all 14 embedded themes. Returns `None` if the ID is not found.
pub fn get_tui_theme(id: &str) -> Option<TuiTheme> {
    THEME_CATALOG
        .iter()
        .find(|t| t.id == id)
        .map(TuiTheme::from)
}

/// Get the default TUI theme based on light/dark preference.
///
/// Returns the first theme matching the requested mode from the catalog,
/// falling back to the hardcoded default if no match is found.
pub fn get_default_tui_theme(prefer_dark: bool) -> TuiTheme {
    let target_mode = if prefer_dark {
        ThemeMode::Dark
    } else {
        ThemeMode::Light
    };

    THEME_CATALOG
        .iter()
        .find(|t| t.mode == target_mode)
        .map(TuiTheme::from)
        .unwrap_or_default()
}

/// Get all available themes as a list of (id, name, mode).
///
/// Returns all 14 themes from the embedded catalog.
pub fn list_themes() -> Vec<(String, String, ThemeMode)> {
    THEME_CATALOG
        .iter()
        .map(|t| (t.id.clone(), t.name.clone(), t.mode))
        .collect()
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
    fn test_hex_to_color_invalid_short() {
        let color = hex_to_color("#fff");
        assert_eq!(color, Color::White);
    }

    #[test]
    fn test_default_theme() {
        let theme = TuiTheme::default();
        // Should match Catppuccin Mocha bg (#1e1e2e)
        assert_eq!(theme.bg, Color::Rgb(30, 30, 46));
    }

    #[test]
    fn test_catalog_has_14_themes() {
        let themes = list_themes();
        assert_eq!(themes.len(), 14, "Expected 14 themes in the catalog");
    }

    #[test]
    fn test_get_tui_theme_catppuccin_mocha() {
        let theme = get_tui_theme("catppuccin-mocha");
        assert!(theme.is_some(), "catppuccin-mocha must be in catalog");
        let theme = theme.unwrap();
        assert_eq!(theme.bg, Color::Rgb(30, 30, 46));
    }

    #[test]
    fn test_get_tui_theme_all_ids_found() {
        let expected_ids = [
            "default-dark",
            "default-light",
            "catppuccin-mocha",
            "catppuccin-latte",
            "catppuccin-frappe",
            "catppuccin-macchiato",
            "dracula",
            "nord",
            "solarized-dark",
            "solarized-light",
            "gruvbox-dark",
            "gruvbox-light",
            "high-contrast",
            "high-contrast-light",
        ];
        for id in &expected_ids {
            assert!(
                get_tui_theme(id).is_some(),
                "Theme '{id}' not found in catalog"
            );
        }
    }

    #[test]
    fn test_get_tui_theme_nonexistent() {
        assert!(get_tui_theme("nonexistent").is_none());
    }

    #[test]
    fn test_get_default_tui_theme_dark() {
        let theme = get_default_tui_theme(true);
        // First dark theme in catalog is default-dark (#1a1a2e)
        assert_eq!(theme.bg, Color::Rgb(26, 26, 46));
    }

    #[test]
    fn test_get_default_tui_theme_light() {
        let theme = get_default_tui_theme(false);
        // First light theme in catalog is default-light (#ffffff)
        assert_eq!(theme.bg, Color::Rgb(255, 255, 255));
    }

    #[test]
    fn test_list_themes_contains_modes() {
        let themes = list_themes();
        let dark_count = themes
            .iter()
            .filter(|(_, _, m)| *m == ThemeMode::Dark)
            .count();
        let light_count = themes
            .iter()
            .filter(|(_, _, m)| *m == ThemeMode::Light)
            .count();
        assert!(dark_count > 0, "Must have at least one dark theme");
        assert!(light_count > 0, "Must have at least one light theme");
    }

    #[test]
    fn test_dracula_theme_colors() {
        let theme = get_tui_theme("dracula").expect("dracula must exist");
        assert_eq!(theme.bg, Color::Rgb(40, 42, 54)); // #282a36
        assert_eq!(theme.accent, Color::Rgb(189, 147, 249)); // #bd93f9
    }

    #[test]
    fn test_high_contrast_theme_colors() {
        let theme = get_tui_theme("high-contrast").expect("high-contrast must exist");
        assert_eq!(theme.bg, Color::Rgb(0, 0, 0)); // #000000
        assert_eq!(theme.fg, Color::Rgb(255, 255, 255)); // #ffffff
    }
}
