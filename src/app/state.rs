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
    /// Unified form-dialog screen. Presentation-only tracking; the dialog
    /// kind lives in the engine's `AppScreen::FormDialog { dialog_type }`,
    /// the single source of truth. Collapses the former per-dialog variants
    /// (AddField/EditField/EditName/EditRelayUrl/CreateGroup/RenameGroup).
    FormDialog,
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
    /// Device-link QR display (engine-driven device-link flow)
    DeviceLinking,
}

impl Screen {
    /// True for screens whose keys are resolved by the generic engine
    /// `ScreenModel` resolver (the humble path). The complement is the only
    /// bespoke-handler set: the setup wizard steps, the lock screen, and the
    /// `ActionMenu`/`ContactImport` overlays — none of which is driven by an
    /// engine `ScreenModel`.
    ///
    /// Single source for the routing gate, replacing a hand-listed
    /// `matches!` that had drifted: `ContactLimit` and `VerifyFingerprint`
    /// are engine-driven but were omitted, so their keys hit the legacy
    /// dispatch and were dropped. Defining the gate by its small complement
    /// makes that class of omission impossible.
    pub(crate) fn routes_through_engine(self) -> bool {
        !matches!(
            self,
            Screen::SetupWelcome
                | Screen::SetupCreateIdentity
                | Screen::SetupAddFields
                | Screen::SetupSecurity
                | Screen::SetupReady
                | Screen::Lock
                | Screen::ActionMenu
                | Screen::ContactImport
        )
    }
}

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
