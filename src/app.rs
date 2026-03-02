// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Application State

use vauchi_core::contact_card::ContactAction;

use crate::backend::{Backend, DeviceLinkResult, QRData};
use crate::i18n::I18n;
use crate::theme::{get_default_tui_theme, get_tui_theme, list_themes, TuiTheme};

/// Current screen in the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// Setup screen (shown when no identity exists)
    Setup,
    /// Home screen with contact card
    Home,
    /// Contact list
    Contacts,
    /// Contact detail view
    ContactDetail,
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
    /// Tor privacy settings screen
    TorSettings,
    /// Privacy & GDPR screen
    Privacy,
    /// Support Vauchi screen
    Support,
    /// Delivery status screen
    Delivery,
    /// Action menu popup for contact fields
    ActionMenu,
    /// Emergency broadcast configuration screen
    Emergency,
    /// Duress PIN and alert configuration screen
    Duress,
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

/// Tor privacy mode state for the UI.
#[derive(Debug, Clone, Default)]
pub struct TorState {
    /// Whether Tor mode is enabled.
    pub enabled: bool,
    /// Whether .onion addresses are preferred.
    pub prefer_onion: bool,
    /// Circuit rotation interval in seconds.
    pub circuit_rotation_secs: u64,
    /// Number of configured bridges.
    pub bridge_count: usize,
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

/// Application state.
#[allow(dead_code)]
pub struct App {
    /// Vauchi backend
    pub backend: Backend,
    /// Current screen
    pub screen: Screen,
    /// Input mode
    pub input_mode: InputMode,
    /// Whether the app should quit (for future use)
    pub should_quit: bool,
    /// Status message
    pub status_message: Option<String>,
    /// Selected contact index (for contacts list)
    pub selected_contact: usize,
    /// Selected field index (for card fields)
    pub selected_field: usize,
    /// Selected field index in contact detail view
    pub selected_contact_field: usize,
    /// Text input buffer
    pub input_buffer: String,
    /// Add field state
    pub add_field_state: AddFieldState,
    /// Edit field state
    pub edit_field_state: EditFieldState,
    /// Edit name state
    pub edit_name_state: EditNameState,
    /// Edit relay URL state
    pub edit_relay_url_state: EditRelayUrlState,
    /// Visibility screen state
    pub visibility_state: VisibilityState,
    /// Backup screen state
    pub backup_state: BackupState,
    /// Selected device index
    pub selected_device: usize,
    /// Device link result (shown as overlay on Devices screen)
    pub device_link_result: Option<DeviceLinkResult>,
    /// Whether a revoke confirmation is pending
    pub revoke_confirm: bool,
    /// Contact search query
    pub contact_search_query: String,
    /// Contact search mode active
    pub contact_search_mode: bool,
    /// Current exchange QR data (for expiration tracking)
    pub current_qr: Option<QRData>,
    /// Sync state
    pub sync_state: SyncState,
    /// Delivery state
    pub delivery_state: DeliveryState,
    /// Tor privacy mode state
    pub tor_state: TorState,
    /// Privacy/GDPR screen state
    pub privacy_state: PrivacyState,
    /// Action menu state (popup for field actions)
    pub action_menu_state: ActionMenuState,
    /// Emergency broadcast state
    pub emergency_state: EmergencyState,
    /// Duress PIN and alert state
    pub duress_state: DuressState,
    /// Internationalization
    pub i18n: I18n,
    /// Active theme
    pub theme: TuiTheme,
    /// Index of the selected theme in the themes list
    pub theme_index: usize,
    /// Available theme IDs for cycling
    pub theme_ids: Vec<String>,
}

/// State for the add field dialog.
#[derive(Debug, Default)]
pub struct AddFieldState {
    pub field_type_index: usize,
    pub label: String,
    pub value: String,
    pub focus: AddFieldFocus,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AddFieldFocus {
    #[default]
    Type,
    Label,
    Value,
}

/// State for the edit field dialog.
#[derive(Debug, Default)]
pub struct EditFieldState {
    pub field_label: String,
    pub field_type: String,
    pub new_value: String,
}

/// State for the edit name dialog.
#[derive(Debug, Default)]
pub struct EditNameState {
    pub new_name: String,
}

/// State for the edit relay URL dialog.
#[derive(Debug, Default)]
pub struct EditRelayUrlState {
    pub new_url: String,
}

/// State for the visibility screen.
#[derive(Debug, Default)]
pub struct VisibilityState {
    pub contact_id: Option<String>,
    pub selected_field: usize,
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum BackupMode {
    #[default]
    Menu,
    Export,
    Import,
}

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

/// Path to theme config file.
fn theme_config_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("vauchi").join("theme"))
}

/// Load saved theme ID from config, or detect from VAUCHI_THEME env var,
/// or default to dark.
fn load_saved_theme(theme_ids: &[String]) -> (TuiTheme, usize) {
    // Check env var first
    if let Ok(id) = std::env::var("VAUCHI_THEME") {
        if let Some(pos) = theme_ids.iter().position(|t| t == &id) {
            if let Some(t) = get_tui_theme(&id) {
                return (t, pos);
            }
        }
    }

    // Check saved config
    if let Some(path) = theme_config_path() {
        if let Ok(id) = std::fs::read_to_string(&path) {
            let id = id.trim();
            if let Some(pos) = theme_ids.iter().position(|t| t == id) {
                if let Some(t) = get_tui_theme(id) {
                    return (t, pos);
                }
            }
        }
    }

    // Default to dark
    let default_id = "default-dark";
    let index = theme_ids.iter().position(|t| t == default_id).unwrap_or(0);
    (get_default_tui_theme(true), index)
}

/// Save theme preference to config file.
fn save_theme_preference(theme_id: &str) {
    if let Some(path) = theme_config_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, theme_id);
    }
}

/// Detect locale from environment variables (LANG, LC_ALL, LC_MESSAGES).
fn detect_locale() -> I18n {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            // Extract language code from e.g. "de_CH.UTF-8" → "de"
            let code = val.split('_').next().unwrap_or(&val);
            let code = code.split('.').next().unwrap_or(code);
            if !code.is_empty() && code != "C" && code != "POSIX" {
                return I18n::from_code(code);
            }
        }
    }
    I18n::default()
}

impl App {
    /// Create a new application.
    pub fn new(backend: Backend) -> Self {
        // Start on Setup screen if no identity exists
        let initial_screen = if backend.has_identity() {
            Screen::Home
        } else {
            Screen::Setup
        };

        let theme_ids: Vec<String> = list_themes().into_iter().map(|(id, _, _)| id).collect();

        // Load saved theme or detect from environment
        let (theme, theme_index) = load_saved_theme(&theme_ids);

        App {
            backend,
            screen: initial_screen,
            input_mode: InputMode::Normal,
            should_quit: false,
            status_message: None,
            selected_contact: 0,
            selected_field: 0,
            selected_contact_field: 0,
            input_buffer: String::new(),
            add_field_state: AddFieldState::default(),
            edit_field_state: EditFieldState::default(),
            edit_name_state: EditNameState::default(),
            edit_relay_url_state: EditRelayUrlState::default(),
            visibility_state: VisibilityState::default(),
            backup_state: BackupState::default(),
            selected_device: 0,
            device_link_result: None,
            revoke_confirm: false,
            contact_search_query: String::new(),
            contact_search_mode: false,
            current_qr: None,
            sync_state: SyncState::default(),
            delivery_state: DeliveryState::default(),
            tor_state: TorState::default(),
            privacy_state: PrivacyState::default(),
            action_menu_state: ActionMenuState::default(),
            emergency_state: EmergencyState::default(),
            duress_state: DuressState::default(),
            i18n: detect_locale(),
            theme,
            theme_index,
            theme_ids,
        }
    }

    /// Cycle to the next theme.
    pub fn next_theme(&mut self) {
        if self.theme_ids.is_empty() {
            return;
        }
        self.theme_index = (self.theme_index + 1) % self.theme_ids.len();
        if let Some(t) = get_tui_theme(&self.theme_ids[self.theme_index]) {
            self.theme = t;
            save_theme_preference(&self.theme_ids[self.theme_index]);
        }
    }

    /// Cycle to the previous theme.
    pub fn prev_theme(&mut self) {
        if self.theme_ids.is_empty() {
            return;
        }
        self.theme_index = if self.theme_index == 0 {
            self.theme_ids.len() - 1
        } else {
            self.theme_index - 1
        };
        if let Some(t) = get_tui_theme(&self.theme_ids[self.theme_index]) {
            self.theme = t;
            save_theme_preference(&self.theme_ids[self.theme_index]);
        }
    }

    /// Set a status message.
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
    }

    /// Clear the status message.
    #[allow(dead_code)]
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Navigate to a screen.
    pub fn goto(&mut self, screen: Screen) {
        self.screen = screen;
        self.input_mode = InputMode::Normal;
    }

    /// Go back to the previous screen.
    pub fn go_back(&mut self) {
        match self.screen {
            // Can't go back from Setup until identity is configured
            Screen::Setup => {
                // Stay on setup screen
            }
            Screen::Contacts
            | Screen::Exchange
            | Screen::Settings
            | Screen::Help
            | Screen::Recovery
            | Screen::Sync
            | Screen::Delivery
            | Screen::TorSettings => {
                self.screen = Screen::Home;
            }
            Screen::Devices => {
                self.device_link_result = None;
                self.revoke_confirm = false;
                self.screen = Screen::Home;
            }
            Screen::Privacy => {
                self.screen = Screen::Settings;
                self.privacy_state = PrivacyState::default();
            }
            Screen::Support => self.screen = Screen::Settings,
            Screen::Emergency => {
                self.screen = Screen::Settings;
                self.emergency_state = EmergencyState::default();
            }
            Screen::Duress => {
                self.screen = Screen::Settings;
                self.duress_state = DuressState::default();
            }
            // From Backup, go back to Setup if no identity, otherwise Home
            Screen::Backup => {
                if self.backend.has_identity() {
                    self.screen = Screen::Home;
                } else {
                    self.screen = Screen::Setup;
                }
            }
            Screen::ContactDetail => {
                self.screen = Screen::Contacts;
            }
            Screen::ContactVisibility => {
                self.screen = Screen::ContactDetail;
                self.visibility_state = VisibilityState::default();
            }
            Screen::AddField => {
                self.screen = Screen::Home;
                self.add_field_state = AddFieldState::default();
            }
            Screen::EditField => {
                self.screen = Screen::Home;
                self.edit_field_state = EditFieldState::default();
            }
            Screen::EditName => {
                self.screen = Screen::Settings;
                self.edit_name_state = EditNameState::default();
            }
            Screen::EditRelayUrl => {
                self.screen = Screen::Settings;
                self.edit_relay_url_state = EditRelayUrlState::default();
            }
            _ => {}
        }
        self.input_mode = InputMode::Normal;
    }
}

// INLINE_TEST_REQUIRED: Tests need access to private detect_locale() and App internals
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use vauchi_core::i18n::Locale;

    /// Env-var tests must be serialised — set_var/remove_var is process-global.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn clear_locale_env() {
        std::env::remove_var("LC_ALL");
        std::env::remove_var("LC_MESSAGES");
        std::env::remove_var("LANG");
    }

    #[test]
    fn test_detect_locale_german() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_locale_env();
        std::env::set_var("LANG", "de_CH.UTF-8");
        let i18n = detect_locale();
        assert_eq!(i18n.locale(), Locale::German);
        clear_locale_env();
    }

    #[test]
    fn test_detect_locale_french() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_locale_env();
        std::env::set_var("LANG", "fr_FR.UTF-8");
        let i18n = detect_locale();
        assert_eq!(i18n.locale(), Locale::French);
        clear_locale_env();
    }

    #[test]
    fn test_detect_locale_lc_all_overrides() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_locale_env();
        std::env::set_var("LANG", "en_US.UTF-8");
        std::env::set_var("LC_ALL", "es_ES.UTF-8");
        let i18n = detect_locale();
        assert_eq!(i18n.locale(), Locale::Spanish);
        clear_locale_env();
    }

    #[test]
    fn test_detect_locale_fallback_english() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_locale_env();
        std::env::set_var("LANG", "C");
        let i18n = detect_locale();
        assert_eq!(i18n.locale(), Locale::English);
        clear_locale_env();
    }
}
