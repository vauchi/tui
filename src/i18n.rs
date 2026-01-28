// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Internationalization wrapper for TUI
//!
//! Provides convenient string lookup using vauchi-core i18n.

use vauchi_core::i18n::{
    get_available_locales, get_locale_info, get_string, get_string_with_args, Locale, LocaleInfo,
};

/// Current locale state for the TUI.
#[derive(Debug, Clone)]
pub struct I18n {
    /// Current locale
    locale: Locale,
}

impl Default for I18n {
    fn default() -> Self {
        Self {
            locale: Locale::English,
        }
    }
}

impl I18n {
    /// Create a new I18n instance with the specified locale.
    pub fn new(locale: Locale) -> Self {
        Self { locale }
    }

    /// Create from a locale code string.
    pub fn from_code(code: &str) -> Self {
        Self {
            locale: Locale::from_code(code).unwrap_or(Locale::English),
        }
    }

    /// Get the current locale.
    pub fn locale(&self) -> Locale {
        self.locale
    }

    /// Set the current locale.
    pub fn set_locale(&mut self, locale: Locale) {
        self.locale = locale;
    }

    /// Set the locale from a code string.
    pub fn set_locale_code(&mut self, code: &str) {
        if let Some(locale) = Locale::from_code(code) {
            self.locale = locale;
        }
    }

    /// Get a localized string by key.
    pub fn t(&self, key: &str) -> String {
        get_string(self.locale, key)
    }

    /// Get a localized string with argument interpolation.
    pub fn t_args(&self, key: &str, args: &[(&str, &str)]) -> String {
        get_string_with_args(self.locale, key, args)
    }

    /// Get info about the current locale.
    pub fn info(&self) -> LocaleInfo {
        get_locale_info(self.locale)
    }

    /// Get all available locales with their info.
    pub fn available_locales() -> Vec<(Locale, LocaleInfo)> {
        get_available_locales()
            .into_iter()
            .map(|l| (l, get_locale_info(l)))
            .collect()
    }

    /// List locale codes and names.
    pub fn list_locales() -> Vec<(String, String, String)> {
        get_available_locales()
            .into_iter()
            .map(|l| {
                let info = get_locale_info(l);
                (
                    info.code.to_string(),
                    info.name.to_string(),
                    info.english_name.to_string(),
                )
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_locale() {
        let i18n = I18n::default();
        assert_eq!(i18n.locale(), Locale::English);
    }

    #[test]
    fn test_from_code() {
        let i18n = I18n::from_code("de");
        assert_eq!(i18n.locale(), Locale::German);
    }

    #[test]
    fn test_translation() {
        let i18n = I18n::default();
        let welcome = i18n.t("welcome.title");
        assert!(welcome.contains("Vauchi"));
    }

    #[test]
    fn test_translation_german() {
        let i18n = I18n::new(Locale::German);
        let welcome = i18n.t("welcome.title");
        assert!(welcome.contains("Willkommen"));
    }

    #[test]
    fn test_available_locales() {
        let locales = I18n::available_locales();
        assert!(!locales.is_empty());
    }
}
