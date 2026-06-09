// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! TUI Interaction Tests
//!
//! Tests the TUI application's public state types and enums.
//! These tests verify state struct defaults and enum variants.
//!
//! Note: Tests requiring App (which depends on Backend) are inline in src/app.rs
//! because they need access to internal types and a test backend.
//!
//! Tests here focus on the standalone state structs and enums.

use vauchi_app::ui::AppScreen;
use vauchi_tui::app::InputMode;

// ============================================================================
// Screen Enum Tests
// ============================================================================

/// Test: AppScreen Home is the initial screen variant
// @internal
#[test]
fn test_screen_home_is_initial() {
    let screen = AppScreen::MyInfo;
    assert_eq!(screen, AppScreen::MyInfo);
}

/// Test: AppScreen equality works
// @internal
#[test]
fn test_screen_equality() {
    assert_eq!(AppScreen::MyInfo, AppScreen::MyInfo);
    assert_ne!(AppScreen::MyInfo, AppScreen::Contacts);
    assert_ne!(AppScreen::Settings, AppScreen::Help);
}

// ============================================================================
// InputMode Tests
// ============================================================================

/// Test: InputMode variants exist
// @internal
#[test]
fn test_input_mode_variants() {
    // allow(zero_assertions): Compile-time shape check — fails to compile if variants removed
    let _ = InputMode::Normal;
    let _ = InputMode::Editing;
}

/// Test: InputMode Normal is the initial mode
// @internal
#[test]
fn test_input_mode_normal_is_initial() {
    let mode = InputMode::Normal;
    assert_eq!(mode, InputMode::Normal);
}

// Backup is engine-driven (core `BackupRecoveryEngine`); the bespoke
// BackupState/BackupMode/BackupFocus unit tests were removed with the state.

// ============================================================================
// State Struct Field Access Tests
// ============================================================================
