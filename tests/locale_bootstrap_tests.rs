// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Startup i18n bootstrap: a resolved locale directory must end up in the
//! process-global store so `I18n::t` returns real strings instead of
//! "Missing: <key>" (TUI-1: core as a git dependency ships only a 2-key
//! bundled fallback, so runtime loading is the only source of full strings).
//!
//! Separate integration binary on purpose: the locale store is a
//! process-global `RwLock`, and `cargo test` runs one process per test
//! binary — sharing one with the `tests/it` suite would let this test's
//! store contents leak into unrelated tests.

use std::path::{Path, PathBuf};

use vauchi_tui::i18n::{I18n, LocaleSource, apply_locale_source, resolve_locales_dir};

fn sibling_locales_with_en_json(workspace: &Path) -> PathBuf {
    let dir = workspace.join("locales");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("en.json"), "{}").unwrap();
    dir
}

// @internal
#[test]
fn resolve_prefers_env_override_verbatim() {
    let tmp = tempfile::tempdir().unwrap();
    // A workspace-sibling candidate exists but must lose: an explicit
    // override is honored even when unusable — silently routing around a
    // misconfigured VAUCHI_LOCALES_DIR hides the misconfiguration.
    sibling_locales_with_en_json(tmp.path());
    let env_dir = tmp.path().join("does-not-exist");
    let source = resolve_locales_dir(
        Some(env_dir.clone()),
        Some(tmp.path().join("tui")),
        Some(tmp.path().join("bin/vauchi-tui")),
    );
    assert_eq!(source, LocaleSource::Env(env_dir));
}

// @internal
#[test]
fn resolve_uses_workspace_sibling_of_manifest_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let sibling = sibling_locales_with_en_json(tmp.path());
    let source = resolve_locales_dir(None, Some(tmp.path().join("tui")), None);
    assert_eq!(source, LocaleSource::WorkspaceSibling(sibling));
}

// @internal
#[test]
fn resolve_rejects_candidate_without_en_json() {
    let tmp = tempfile::tempdir().unwrap();
    // The directory exists but holds no locale files — an unrelated
    // directory named `locales/` must not be mistaken for a locale catalog.
    std::fs::create_dir_all(tmp.path().join("locales")).unwrap();
    let source = resolve_locales_dir(None, Some(tmp.path().join("tui")), None);
    assert_eq!(source, LocaleSource::BundledFallback);
}

// @internal
#[test]
fn resolve_searches_upward_from_executable() {
    let tmp = tempfile::tempdir().unwrap();
    let app_root = tmp.path().join("app");
    let sibling = sibling_locales_with_en_json(&app_root);
    for exe in [
        app_root.join("target/debug/vauchi-tui"),
        app_root.join("target/x86_64-unknown-linux-gnu/release/vauchi-tui"),
    ] {
        let source = resolve_locales_dir(None, None, Some(exe));
        assert_eq!(source, LocaleSource::ExeRelative(sibling.clone()));
    }
}

// @internal
#[test]
fn resolve_finds_installed_share_vauchi_locales() {
    let tmp = tempfile::tempdir().unwrap();
    let share = tmp.path().join("usr/share/vauchi/locales");
    std::fs::create_dir_all(&share).unwrap();
    std::fs::write(share.join("en.json"), "{}").unwrap();
    let exe = tmp.path().join("usr/bin/vauchi-tui");
    let source = resolve_locales_dir(None, None, Some(exe));
    assert_eq!(source, LocaleSource::ExeRelative(share));
}

// @internal
#[test]
fn resolve_falls_back_to_bundled_when_nothing_found() {
    let tmp = tempfile::tempdir().unwrap();
    let source = resolve_locales_dir(
        None,
        Some(tmp.path().join("tui")),
        Some(tmp.path().join("bin/vauchi-tui")),
    );
    assert_eq!(source, LocaleSource::BundledFallback);
    assert_eq!(source.path(), None);
}

// @internal
#[test]
fn applying_locale_source_loads_runtime_strings() {
    // Pre-condition: with an untouched global store the bundled English
    // fallback answers.
    assert!(!vauchi_app::i18n::is_initialized());
    assert!(I18n::default().t("welcome.title").contains("Vauchi"));

    // A source pointing at a nonexistent directory loads nothing; core's
    // `init` reports `Ok` for it, so `is_initialized` is the only signal.
    apply_locale_source(&LocaleSource::Env(PathBuf::from(
        "/nonexistent/vauchi-locales",
    )));
    assert!(!vauchi_app::i18n::is_initialized());

    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("locales");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("en.json"), r#"{"app.name": "Sentinel Vauchi"}"#).unwrap();

    let source = resolve_locales_dir(Some(dir.clone()), None, None);
    assert_eq!(source, LocaleSource::Env(dir));
    apply_locale_source(&source);

    assert!(vauchi_app::i18n::is_initialized());
    assert_eq!(I18n::default().t("app.name"), "Sentinel Vauchi");
    // Negative case: a loaded locale is authoritative, so an absent key is
    // still reported missing rather than silently falling back.
    assert_eq!(I18n::default().t("no.such.key"), "Missing: no.such.key");
}
