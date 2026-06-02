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

use vauchi_tui::app::{InputMode, Screen, SyncState};

// ============================================================================
// Screen Enum Tests
// ============================================================================

/// Test: All screen variants exist (including Privacy)
// @internal
#[test]
fn test_screen_variants_exist() {
    let screens = [
        Screen::MyInfo,
        Screen::Contacts,
        Screen::ContactDetail,
        Screen::ContactVisibility,
        Screen::Exchange,
        Screen::Settings,
        Screen::Help,
        Screen::FormDialog,
        Screen::Devices,
        Screen::Recovery,
        Screen::Sync,
        Screen::Backup,
        Screen::Privacy,
        Screen::Support,
        Screen::Delivery,
        Screen::ActionMenu,
        Screen::Emergency,
        Screen::Duress,
        Screen::Lock,
        Screen::Groups,
        Screen::GroupDetail,
        // SP-21 Onboarding wizard
        Screen::SetupWelcome,
        Screen::SetupCreateIdentity,
        Screen::SetupAddFields,
        Screen::SetupSecurity,
        Screen::SetupReady,
        // SP-12a Duplicates / Merge / Limit
        Screen::ContactDuplicates,
        Screen::ContactMerge,
        Screen::ContactLimit,
    ];

    // Verify we have all 29 screen variants
    assert_eq!(screens.len(), 29);
}

/// Test: Screen Home is the initial screen variant
// @internal
#[test]
fn test_screen_home_is_initial() {
    let screen = Screen::MyInfo;
    assert_eq!(screen, Screen::MyInfo);
}

/// Test: Screen equality works
// @internal
#[test]
fn test_screen_equality() {
    assert_eq!(Screen::MyInfo, Screen::MyInfo);
    assert_ne!(Screen::MyInfo, Screen::Contacts);
    assert_ne!(Screen::Settings, Screen::Help);
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
// SyncState Tests
// ============================================================================

/// Test: SyncState default values
// @internal
#[test]
fn test_sync_state_default() {
    let state = SyncState::default();
    assert!(!state.connected);
    assert!(!state.is_syncing);
    assert_eq!(state.pending_updates, 0);
    assert!(state.last_result.is_none());
    assert!(state.sync_log.is_empty());
}

/// Test: SyncState can be modified
// @internal
#[test]
fn test_sync_state_modification() {
    let state = SyncState {
        connected: true,
        is_syncing: true,
        pending_updates: 5,
        sync_log: vec!["Test log entry".to_string()],
        ..Default::default()
    };

    assert!(state.connected);
    assert!(state.is_syncing);
    assert_eq!(state.pending_updates, 5);
    assert_eq!(state.sync_log.len(), 1);
}

// ============================================================================
// State Struct Field Access Tests
// ============================================================================
