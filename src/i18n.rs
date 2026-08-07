// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Internationalization wrapper for TUI
//!
//! Provides convenient string lookup using vauchi-core i18n.

use std::path::{Path, PathBuf};

use vauchi_app::i18n::{
    Locale, LocaleInfo, get_available_locales, get_locale_info, get_string, get_string_with_args,
};

/// Where runtime locale files were resolved from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocaleSource {
    /// `VAUCHI_LOCALES_DIR` environment variable.
    Env(PathBuf),
    /// `locales/` next to the repo checkout (workspace development layout).
    WorkspaceSibling(PathBuf),
    /// `locales/` (or `share/vauchi/locales/`) found searching upward from
    /// the executable.
    ExeRelative(PathBuf),
    /// No locale directory found; core's bundled English fallback applies.
    BundledFallback,
}

impl LocaleSource {
    /// Directory to load, if any.
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Env(dir) | Self::WorkspaceSibling(dir) | Self::ExeRelative(dir) => Some(dir),
            Self::BundledFallback => None,
        }
    }
}

/// Resolves the runtime locale directory: env override, then the
/// workspace-sibling `locales/`, then an upward search from the executable.
///
/// vauchi-core is consumed as a git dependency, so its build-time locales
/// symlink dangles in cargo git checkouts and the full string catalog must
/// be loaded at runtime — the same contract linux-gtk and vauchi-cabi
/// already follow. Pure: no env reads, no globals. Candidates must contain
/// `en.json` so an unrelated directory that happens to be named `locales/`
/// is not mistaken for a Vauchi locale catalog.
pub fn resolve_locales_dir(
    env_dir: Option<PathBuf>,
    manifest_dir: Option<PathBuf>,
    exe_path: Option<PathBuf>,
) -> LocaleSource {
    if let Some(dir) = env_dir {
        return LocaleSource::Env(dir);
    }

    if let Some(dir) = manifest_dir
        .and_then(|m| m.parent().map(|p| p.join("locales")))
        .filter(|d| has_english_locale(d))
    {
        return LocaleSource::WorkspaceSibling(dir);
    }

    // 8 levels covers `<ws>/.worktrees/<branch>/tui/target/<triple>/<profile>/`
    // (5 pops to the workspace root) with slack; installed binaries find
    // `share/vauchi/locales` within 2.
    if let Some(mut base) = exe_path.and_then(|e| e.parent().map(PathBuf::from)) {
        for _ in 0..8 {
            for candidate in [base.join("locales"), base.join("share/vauchi/locales")] {
                if has_english_locale(&candidate) {
                    return LocaleSource::ExeRelative(candidate);
                }
            }
            if !base.pop() {
                break;
            }
        }
    }

    LocaleSource::BundledFallback
}

fn has_english_locale(dir: &Path) -> bool {
    dir.join("en.json").is_file()
}

/// Loads a resolved source into the global locale store.
/// `BundledFallback` is a no-op; load errors are non-fatal (core keeps the
/// bundled fallback). Callers can detect a failed load via
/// `vauchi_app::i18n::is_initialized()` — core's `init` reports `Ok` even
/// for a directory that yielded no locales.
pub fn apply_locale_source(source: &LocaleSource) {
    if let Some(dir) = source.path() {
        let _ = vauchi_app::i18n::init(dir);
    }
}

/// Startup bootstrap: resolve and load runtime locale files.
/// Returns the source so the caller can report when no strings were loaded.
pub fn init_from_environment() -> LocaleSource {
    let source = resolve_locales_dir(
        std::env::var_os("VAUCHI_LOCALES_DIR").map(PathBuf::from),
        // Compile-time repo path: restricted to debug builds so a release
        // binary never prefers a leftover build tree over installed locales.
        if cfg!(debug_assertions) {
            option_env!("CARGO_MANIFEST_DIR").map(PathBuf::from)
        } else {
            None
        },
        std::env::current_exe().ok(),
    );
    apply_locale_source(&source);
    source
}

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

// INLINE_TEST_REQUIRED: tests use private LOCALE_STORE static and format_locale_key
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
        // bundled_english provides fallback for English
        let i18n = I18n::default();
        let welcome = i18n.t("welcome.title");
        assert!(
            welcome.contains("Vauchi"),
            "Expected 'Vauchi' in welcome, got: {}",
            welcome
        );
    }

    #[test]
    fn test_translation_german() {
        // German requires locale files to be loaded
        // Without them, falls back to English
        let i18n = I18n::new(Locale::German);
        let welcome = i18n.t("welcome.title");
        // Accept either German or English fallback
        assert!(
            welcome.contains("Willkommen") || welcome.contains("Vauchi"),
            "Expected 'Willkommen' or 'Vauchi', got: {}",
            welcome
        );
    }

    #[test]
    fn test_available_locales() {
        let locales = I18n::available_locales();
        assert!(!locales.is_empty());
    }
}
