// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Application state types — dialog states, feature-area states, and supporting enums.

use vauchi_core::contact_card::ContactAction;

/// A TUI-only modal layered on top of the underlying engine screen. These
/// have no engine `AppScreen` — the engine stays on the screen beneath
/// while the overlay captures input. Tracked separately from `Screen` so
/// the screen stays the engine's truth; the matching `Screen` variant is
/// retired once nothing else references it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    /// Field-action popup over a contact's detail screen.
    ActionMenu,
    /// vCard file-path entry dialog over the contact list.
    ContactImport,
}

/// Input mode for text entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    /// Normal navigation mode
    Normal,
    /// Editing text
    Editing,
}

/// Terminal-only sync status. The Sync screen renders from the engine's
/// `ScreenModel`; these are the only fields the TUI reads — a concurrency
/// guard and an on-demand pending count. The former `connected`,
/// `last_result`, and `sync_log` fields were write-only mirror state and
/// were removed (G2).
#[derive(Debug, Clone, Default)]
pub struct SyncState {
    /// Whether a sync operation is in progress (concurrency guard).
    pub is_syncing: bool,
    /// Number of pending outbound updates (shown on demand via the Sync
    /// screen 'r' refresh; not otherwise rendered).
    pub pending_updates: u32,
}

/// Lock screen state for PIN entry on startup.
#[derive(Debug, Clone, Default)]
pub struct LockState {
    /// Current PIN input (masked in UI).
    pub pin_input: String,
    /// Number of failed attempts.
    pub attempts: u8,
    /// Whether the last attempt failed.
    pub error: bool,
}

/// State for the action menu popup.
#[derive(Debug, Default)]
pub struct ActionMenuState {
    /// Available actions with display labels.
    pub actions: Vec<(String, ContactAction)>,
    /// Currently selected action index.
    pub selected: usize,
}

/// State for the onboarding wizard (SP-21).
#[derive(Debug, Default)]
pub struct OnboardingState {
    /// Display name entered during onboarding.
    pub name_input: String,
    /// Whether identity has been created during this wizard run.
    pub identity_created: bool,
}

/// State for the contact import screen.
#[derive(Debug, Default)]
pub struct ImportState {
    /// File path input buffer.
    pub file_path: String,
    /// Result message after import attempt.
    pub result_message: Option<String>,
    /// Whether the import succeeded (for styling).
    pub success: bool,
}
