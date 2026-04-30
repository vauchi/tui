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

/// Delivery status state for the UI.
#[derive(Debug, Clone, Default)]
pub struct DeliveryState {
    /// Number of queued deliveries.
    pub queued: usize,
    /// Number of sent deliveries.
    pub sent: usize,
    /// Number of stored deliveries.
    pub stored: usize,
    /// Number of delivered messages.
    pub delivered: usize,
    /// Number of failed deliveries.
    pub failed: usize,
    /// Number of pending retries.
    pub pending_retries: usize,
    /// Offline queue depth.
    pub offline_queue_depth: usize,
    /// Last action result message.
    pub last_result: Option<String>,
}

/// Emergency broadcast screen state.
#[derive(Debug, Clone, Default)]
pub struct EmergencyState {
    /// Whether a config is currently saved.
    pub configured: bool,
    /// Trusted contact IDs (comma-separated in input).
    pub contact_ids_input: String,
    /// Alert message.
    pub message_input: String,
    /// Whether to include location.
    pub include_location: bool,
    /// Number of trusted contacts (for display).
    pub trusted_count: usize,
    /// Current input focus.
    pub focus: EmergencyFocus,
    /// Timestamp of last broadcast (for rate limiting).
    pub last_broadcast_time: Option<u64>,
}

/// Focus states for the emergency screen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EmergencyFocus {
    #[default]
    /// Viewing status (not editing).
    Status,
    /// Editing contact IDs.
    ContactIds,
    /// Editing message.
    Message,
    /// Confirmation before sending broadcast.
    Confirm,
}

/// Duress PIN and alert configuration screen state.
#[derive(Debug, Clone, Default)]
pub struct DuressState {
    /// Whether an app password is configured (required for duress).
    pub password_enabled: bool,
    /// Whether duress mode is enabled.
    pub enabled: bool,
    /// PIN input for setup.
    pub pin_input: String,
    /// Trusted contact IDs (comma-separated in input).
    pub contact_ids_input: String,
    /// Alert message.
    pub message_input: String,
    /// Whether to include location in alerts.
    pub include_location: bool,
    /// Number of alert contacts (for display).
    pub alert_contact_count: usize,
    /// Current input focus.
    pub focus: DuressFocus,
}

/// Focus states for the duress screen.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DuressFocus {
    #[default]
    /// Viewing status (not editing).
    Status,
    /// Entering a new duress PIN.
    PinSetup,
    /// Editing alert contact IDs.
    ContactIds,
    /// Editing alert message.
    Message,
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

/// State for the groups management screen.
#[derive(Debug, Default)]
pub struct GroupsState {
    /// Currently selected group index in the list.
    pub selected_group: usize,
    /// ID of the currently selected group (for engine routing).
    pub selected_group_id: Option<String>,
    /// Whether to show group detail view.
    pub show_group_detail: bool,
    /// Group edit mode (for creating/renaming).
    pub edit_mode: bool,
    /// Input buffer for group name (when creating or renaming).
    pub group_name_input: String,
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

/// A duplicate pair for the UI.
#[derive(Debug, Clone)]
pub struct DuplicateEntry {
    pub id1: String,
    pub name1: String,
    pub id2: String,
    pub name2: String,
    pub similarity: f64,
}

/// State for the contact duplicates screen (SP-12a).
#[derive(Debug, Default)]
pub struct DuplicatesState {
    /// Detected duplicate pairs.
    pub pairs: Vec<DuplicateEntry>,
    /// Currently selected pair index.
    pub selected: usize,
}

/// State for the contact merge preview screen (SP-12a).
#[derive(Debug, Default, Clone)]
pub struct MergeState {
    /// Primary contact ID.
    pub primary_id: String,
    /// Primary contact name.
    pub primary_name: String,
    /// Primary contact fields.
    pub primary_fields: Vec<String>,
    /// Secondary contact ID.
    pub secondary_id: String,
    /// Secondary contact name.
    pub secondary_name: String,
    /// Secondary contact fields.
    pub secondary_fields: Vec<String>,
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

/// State for the contact limit screen (SP-12a).
#[derive(Debug, Default)]
pub struct ContactLimitState {
    /// Current contact limit.
    pub current_limit: usize,
    /// Current contact count.
    pub current_count: usize,
    /// New limit input buffer.
    pub limit_input: String,
    /// Whether editing is active.
    pub editing: bool,
}

/// Activity log screen state (Phase 2.4).
#[derive(Debug, Default)]
pub struct ActivityState {
    /// Activity log entries.
    pub entries: Vec<vauchi_core::storage::ActivityLogRow>,
    /// Selected entry index.
    pub selected: usize,
}
