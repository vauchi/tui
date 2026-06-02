// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Application state types — dialog states, feature-area states, and supporting enums.

use vauchi_core::contact_card::ContactAction;

/// Current screen in the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// My Info screen with contact card
    MyInfo,
    /// Contact list
    Contacts,
    /// Contact detail view
    ContactDetail,
    /// Contact edit form (3-step: fields -> visibility -> preview)
    ContactEdit,
    /// Contact visibility settings
    ContactVisibility,
    /// QR exchange screen
    Exchange,
    /// Settings screen
    Settings,
    /// Help screen
    Help,
    /// Add field dialog
    AddField,
    /// Edit field dialog
    EditField,
    /// Edit display name dialog
    EditName,
    /// Edit relay URL dialog
    EditRelayUrl,
    /// Device management screen
    Devices,
    /// Recovery screen
    Recovery,
    /// Sync status screen
    Sync,
    /// Backup/restore screen
    Backup,
    /// Privacy & GDPR screen
    Privacy,
    /// Support Vauchi screen
    Support,
    /// Delivery status screen
    Delivery,
    /// Activity log and notifications screen
    Activity,
    /// Action menu popup for contact fields
    ActionMenu,
    /// Emergency broadcast configuration screen
    Emergency,
    /// Duress PIN and alert configuration screen
    Duress,
    /// Lock screen (shown on startup when app password is configured)
    Lock,
    /// More menu (Settings, Help, and other infrastructure screens)
    More,
    /// Contact groups management screen
    Groups,
    /// Group detail view
    GroupDetail,
    /// Create-group form dialog (FormDialogType::CreateGroup).
    CreateGroup,
    /// Rename-group form dialog (FormDialogType::RenameGroup).
    RenameGroup,
    // -- SP-21 Onboarding Wizard --
    /// Welcome screen with privacy highlights
    SetupWelcome,
    /// Name input step
    SetupCreateIdentity,
    /// Optional field addition step
    SetupAddFields,
    /// Security explanation step
    SetupSecurity,
    /// Completion / ready screen
    SetupReady,
    // -- SP-12a Merge / Duplicates / Limit --
    /// List of potential duplicate contacts
    ContactDuplicates,
    /// Side-by-side merge preview
    ContactMerge,
    /// Contact limit configuration
    ContactLimit,
    /// MyInfo entry detail view
    MyInfoEntryDetail,
    /// Fingerprint verification screen
    VerifyFingerprint,
    /// Contact import (vCard file path entry)
    ContactImport,
    /// Device replacement wizard
    DeviceReplacement,
}

/// Input mode for text entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    /// Normal navigation mode
    Normal,
    /// Editing text
    Editing,
}

/// Sync status for the UI.
#[derive(Debug, Clone, Default)]
pub struct SyncState {
    /// Whether currently connected to the relay.
    pub connected: bool,
    /// Whether a sync operation is in progress.
    pub is_syncing: bool,
    /// Number of pending outbound updates.
    pub pending_updates: u32,
    /// Last sync result message.
    pub last_result: Option<String>,
    /// Log of sync operations.
    pub sync_log: Vec<String>,
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

/// State for the backup screen.
#[derive(Debug, Default)]
pub struct BackupState {
    pub mode: BackupMode,
    pub password: String,
    pub confirm_password: String,
    pub backup_data: String,
    pub focus: BackupFocus,
}

/// Which backup sub-screen is active: top-level menu, export, or import.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum BackupMode {
    #[default]
    Menu,
    Export,
    Import,
}

/// Tracks which input field is focused in the backup export/import form.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum BackupFocus {
    #[default]
    Password,
    Confirm,
    Data,
}

/// State for the privacy/GDPR screen.
#[derive(Debug, Default)]
pub struct PrivacyState {
    /// Currently selected section index (0=Export, 1=Deletion, 2..=Consent items).
    pub selected_item: usize,
}

/// State for the action menu popup.
#[derive(Debug, Default)]
pub struct ActionMenuState {
    /// Available actions with display labels.
    pub actions: Vec<(String, ContactAction)>,
    /// Currently selected action index.
    pub selected: usize,
}

/// State for the groups management screen — terminal-only navigation
/// state. Group identity comes from `AppScreen::GroupDetail
/// { group_id }`; create/rename go through `FormDialogEngine` via
/// `goto_form_dialog(FormDialogType::CreateGroup | RenameGroup)`.
#[derive(Debug, Default)]
pub struct GroupsState {
    /// Currently selected group index in the list.
    pub selected_group: usize,
    /// Currently selected contact index in group detail view.
    pub selected_contact_in_group: usize,
    /// Search query for filtering groups.
    pub group_search_query: String,
    /// Whether group search mode is active.
    pub group_search_mode: bool,
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

/// Activity log screen state (Phase 2.4).
#[derive(Debug, Default)]
pub struct ActivityState {
    /// Activity log entries.
    pub entries: Vec<vauchi_core::storage::ActivityLogRow>,
    /// Selected entry index.
    pub selected: usize,
}
