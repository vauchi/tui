// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Application State

use vauchi_core::contact_card::ContactAction;
use vauchi_core::ui::{AppEngine, AppScreen, FormDialogType, LockScreenEngine, OnboardingEngine};

use vauchi_core::api::DeviceLinkResult;

use crate::i18n::I18n;
use crate::sync_service::SyncResult;
use crate::theme::{get_default_tui_theme, get_tui_theme, list_themes, TuiTheme};
use crate::ui::focus::FocusManager;
use crate::ui::widgets::screen_renderer::ScreenRenderState;

/// Current screen in the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// My Info screen with contact card
    MyInfo,
    /// Contact list
    Contacts,
    /// Contact detail view
    ContactDetail,
    /// Contact edit form (3-step: fields → visibility → preview)
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
    /// Lock screen (shown on startup when app password is configured)
    Lock,
    /// Contact groups management screen
    Groups,
    /// Group detail view
    GroupDetail,
    // ── SP-21 Onboarding Wizard ──
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
    // ── SP-12a Merge / Duplicates / Limit ──
    /// List of potential duplicate contacts
    ContactDuplicates,
    /// Side-by-side merge preview
    ContactMerge,
    /// Contact limit configuration
    ContactLimit,
    /// MyInfo entry detail view
    MyInfoEntryDetail,
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

/// Application state.
#[allow(dead_code)]
pub struct App {
    /// Current screen
    pub screen: Screen,
    /// Input mode
    pub input_mode: InputMode,
    /// Whether the app should quit (for future use)
    pub should_quit: bool,
    /// Status message
    pub status_message: Option<String>,
    /// When the status message was set (for auto-clear after 3 seconds).
    pub status_message_time: Option<std::time::Instant>,
    /// Modal alert message (title, body) — requires user dismissal.
    pub alert_message: Option<(String, String)>,
    /// Selected contact index (for contacts list)
    pub selected_contact: usize,
    /// Selected contact ID (for engine-driven ContactDetail screen)
    pub selected_contact_id: Option<String>,
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
    /// Whether a contact delete confirmation is pending
    pub contact_delete_confirm: bool,
    /// Whether a form discard confirmation is pending (Escape with unsaved data)
    pub form_discard_confirm: bool,
    /// Current exchange QR data (for expiration tracking)
    pub current_qr: Option<vauchi_core::api::ExchangeQrData>,
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
    /// Lock screen state
    pub lock_state: LockState,
    /// Groups management state
    pub groups_state: GroupsState,
    /// Onboarding wizard state (SP-21)
    pub onboarding_state: OnboardingState,
    // ── Core-driven workflow engines ──
    /// Unified AppEngine orchestrator — single owner of Vauchi<T>.
    /// All identity/contact/card operations go through this.
    pub app_engine: AppEngine,
    /// Onboarding engine (core-driven state machine)
    pub onboarding_engine: Option<OnboardingEngine>,
    /// Lock screen engine (core-driven state machine)
    pub lock_engine: Option<LockScreenEngine>,
    /// Render state for engine-driven screens (focus, selections)
    pub render_state: ScreenRenderState,
    /// Focus manager for keyboard navigation across Content/ActionBar/NavBar zones.
    pub focus: FocusManager,
    /// Duplicates detection state (SP-12a)
    pub duplicates_state: DuplicatesState,
    /// Merge preview state (SP-12a)
    pub merge_state: MergeState,
    /// Contact limit state (SP-12a)
    pub contact_limit_state: ContactLimitState,
    /// Internationalization
    pub i18n: I18n,
    /// Active theme
    pub theme: TuiTheme,
    /// Index of the selected theme in the themes list
    pub theme_index: usize,
    /// Available theme IDs for cycling
    pub theme_ids: Vec<String>,
    /// Relay server URL (loaded from config/env/default at startup)
    pub relay_url: String,
    /// Data directory path (TUI storage location)
    pub data_dir: std::path::PathBuf,
}

/// State for the add field dialog.
#[derive(Debug, Default)]
pub struct AddFieldState {
    pub field_type_index: usize,
    pub label: String,
    pub value: String,
    pub focus: AddFieldFocus,
    /// Social network picker state (used when field type is Social).
    pub social_picker: SocialPickerState,
}

/// State for the social network picker.
#[derive(Debug, Default)]
pub struct SocialPickerState {
    /// Available networks (id, display_name) pairs, sorted by display name.
    pub networks: Vec<(String, String)>,
    /// Currently selected network index.
    pub selected: usize,
}

/// Tracks which input field is focused in the add-field dialog.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum AddFieldFocus {
    #[default]
    Type,
    /// Social network picker (only shown when field type is Social).
    Network,
    Label,
    Value,
}

/// State for the edit field dialog.
#[derive(Debug, Default)]
pub struct EditFieldState {
    pub field_id: String,
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
    /// Whether a delete confirmation is pending.
    pub delete_confirm: bool,
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
    ///
    /// AppEngine owns the single Vauchi<T> instance — all identity/contact/card
    /// operations go through it. Relay sync uses the standalone sync_service module.
    pub fn new(app_engine: AppEngine, relay_url: String, data_dir: std::path::PathBuf) -> Self {
        // Determine initial screen from AppEngine's Vauchi state
        let has_identity = app_engine.vauchi().has_identity();
        let initial_screen = if !has_identity {
            Screen::SetupWelcome
        } else if app_engine.vauchi().is_password_enabled().unwrap_or(false) {
            Screen::Lock
        } else {
            // Use AppEngine's dynamic default: MyInfo (0 contacts) or Contacts (>=1)
            match app_engine.default_screen() {
                AppScreen::Contacts => Screen::Contacts,
                _ => Screen::MyInfo,
            }
        };

        // Navigate AppEngine to the correct initial screen
        let mut app_engine = app_engine;
        if has_identity {
            let target = match initial_screen {
                Screen::Lock => AppScreen::Lock,
                Screen::Contacts => AppScreen::Contacts,
                _ => AppScreen::MyInfo,
            };
            app_engine.navigate_to(target);
        }

        // Create engines for initial screen
        let onboarding_engine = if initial_screen == Screen::SetupWelcome {
            Some(OnboardingEngine::new())
        } else {
            None
        };
        let lock_engine = if initial_screen == Screen::Lock {
            Some(LockScreenEngine::new(5))
        } else {
            None
        };

        let theme_ids: Vec<String> = list_themes().into_iter().map(|(id, _, _)| id).collect();

        // Load saved theme or detect from environment
        let (theme, theme_index) = load_saved_theme(&theme_ids);

        App {
            screen: initial_screen,
            input_mode: InputMode::Normal,
            should_quit: false,
            status_message: None,
            status_message_time: None,
            alert_message: None,
            selected_contact: 0,
            selected_contact_id: None,
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
            contact_delete_confirm: false,
            form_discard_confirm: false,
            current_qr: None,
            sync_state: SyncState::default(),
            delivery_state: DeliveryState::default(),
            tor_state: TorState::default(),
            privacy_state: PrivacyState::default(),
            action_menu_state: ActionMenuState::default(),
            emergency_state: EmergencyState::default(),
            duress_state: DuressState::default(),
            lock_state: LockState::default(),
            groups_state: GroupsState::default(),
            onboarding_state: OnboardingState::default(),
            app_engine,
            onboarding_engine,
            lock_engine,
            render_state: ScreenRenderState::default(),
            focus: FocusManager::new(),
            duplicates_state: DuplicatesState::default(),
            merge_state: MergeState::default(),
            contact_limit_state: ContactLimitState::default(),
            i18n: detect_locale(),
            theme,
            theme_index,
            theme_ids,
            relay_url,
            data_dir,
        }
    }

    /// Performs a full sync with the relay server via WebSocket.
    pub fn sync(&self) -> SyncResult {
        let vauchi = self.app_engine.vauchi();
        let identity = match vauchi.identity() {
            Some(id) => id,
            None => {
                return SyncResult {
                    contacts_added: 0,
                    cards_updated: 0,
                    updates_sent: 0,
                    success: false,
                    error: Some("No identity".into()),
                }
            }
        };
        crate::sync_service::sync(identity, vauchi.storage(), &self.relay_url)
    }

    /// Tests the relay connection.
    pub fn test_relay_connection(&self) -> anyhow::Result<bool> {
        crate::sync_service::test_relay_connection(&self.relay_url)
    }

    /// Generates exchange QR data via Vauchi API.
    pub fn generate_exchange_qr(
        &self,
    ) -> vauchi_core::api::VauchiResult<vauchi_core::api::ExchangeQrData> {
        self.app_engine.vauchi().generate_exchange_qr()
    }

    /// Returns the user's display name from the single Vauchi instance.
    pub fn display_name(&self) -> Option<&str> {
        self.app_engine
            .vauchi()
            .identity()
            .map(|i| i.display_name())
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

    /// Invalidates all cached AppEngine screens after a mutation.
    pub fn invalidate_engines(&mut self) {
        self.app_engine.invalidate_all();
    }

    /// Set a status message.
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
        self.status_message_time = Some(std::time::Instant::now());
    }

    /// Clear the status message.
    #[allow(dead_code)]
    pub fn clear_status(&mut self) {
        self.status_message = None;
        self.status_message_time = None;
    }

    /// Auto-clear status message after 3 seconds.
    pub fn tick_status(&mut self) {
        if let Some(time) = self.status_message_time {
            if time.elapsed() >= std::time::Duration::from_secs(3) {
                self.clear_status();
            }
        }
    }

    /// Navigate to a screen.
    pub fn goto(&mut self, screen: Screen) {
        // Clear contact search when leaving Contacts screen
        if self.screen == Screen::Contacts && screen != Screen::Contacts {
            self.contact_search_mode = false;
            self.contact_search_query.clear();
        }
        self.screen = screen;
        self.input_mode = InputMode::Normal;
        self.sync_nav_index();

        // Navigate AppEngine for engine-driven screens
        if let Some(app_screen) = self.to_app_screen() {
            self.app_engine.navigate_to(app_screen);
            self.render_state = ScreenRenderState::default();
        }

        // Create engines for engine-backed screens
        match screen {
            Screen::SetupWelcome => {
                if self.onboarding_engine.is_none() {
                    self.onboarding_engine = Some(OnboardingEngine::new());
                }
                self.render_state = ScreenRenderState::default();
            }
            Screen::Lock => {
                if self.lock_engine.is_none() {
                    self.lock_engine = Some(LockScreenEngine::new(5));
                }
                self.render_state = ScreenRenderState::default();
            }
            _ => {}
        }
    }

    /// Maps TUI Screen to core AppScreen for engine-driven screens.
    pub fn to_app_screen(&self) -> Option<AppScreen> {
        match self.screen {
            Screen::MyInfo => Some(AppScreen::MyInfo),
            Screen::Contacts => Some(AppScreen::Contacts),
            Screen::Exchange => Some(AppScreen::Exchange),
            Screen::Settings => Some(AppScreen::Settings),
            Screen::Help => Some(AppScreen::Help),
            Screen::Backup => Some(AppScreen::Backup),
            Screen::Delivery => Some(AppScreen::DeliveryStatus),
            Screen::Devices => Some(AppScreen::DeviceLinking),
            Screen::Duress => Some(AppScreen::DuressPin),
            Screen::Emergency => Some(AppScreen::EmergencyShred),
            Screen::ContactDetail => {
                self.selected_contact_id
                    .as_ref()
                    .map(|id| AppScreen::ContactDetail {
                        contact_id: id.clone(),
                    })
            }
            Screen::ContactEdit => {
                self.selected_contact_id
                    .as_ref()
                    .map(|id| AppScreen::ContactEdit {
                        contact_id: id.clone(),
                    })
            }
            Screen::Sync => Some(AppScreen::Sync),
            Screen::TorSettings => Some(AppScreen::TorSettings),
            Screen::Recovery => Some(AppScreen::Recovery),
            Screen::Groups => Some(AppScreen::Groups),
            Screen::GroupDetail => {
                self.groups_state
                    .selected_group_id
                    .as_ref()
                    .map(|id| AppScreen::GroupDetail {
                        group_id: id.clone(),
                    })
            }
            Screen::ContactVisibility => {
                self.selected_contact_id
                    .as_ref()
                    .map(|id| AppScreen::ContactVisibility {
                        contact_id: id.clone(),
                    })
            }
            Screen::Privacy => Some(AppScreen::Privacy),
            Screen::Support => Some(AppScreen::Support),
            Screen::EditName => Some(AppScreen::FormDialog {
                dialog_type: FormDialogType::EditName {
                    current_name: self.edit_name_state.new_name.clone(),
                },
            }),
            Screen::EditField => Some(AppScreen::FormDialog {
                dialog_type: FormDialogType::EditField {
                    field_id: self.edit_field_state.field_id.clone(),
                    field_label: self.edit_field_state.field_label.clone(),
                    current_value: self.edit_field_state.new_value.clone(),
                },
            }),
            Screen::EditRelayUrl => Some(AppScreen::FormDialog {
                dialog_type: FormDialogType::EditRelayUrl {
                    current_url: self.edit_relay_url_state.new_url.clone(),
                },
            }),
            Screen::AddField => {
                // Load available groups for the add field form
                let groups = self.app_engine.available_groups().into_iter().collect();
                Some(AppScreen::FormDialog {
                    dialog_type: FormDialogType::AddField {
                        available_groups: groups,
                    },
                })
            }
            Screen::ContactDuplicates => Some(AppScreen::ContactDuplicates),
            Screen::ContactMerge => {
                let s = &self.merge_state;
                Some(AppScreen::ContactMerge {
                    primary_name: s.primary_name.clone(),
                    primary_fields: s.primary_fields.clone(),
                    secondary_name: s.secondary_name.clone(),
                    secondary_fields: s.secondary_fields.clone(),
                })
            }
            Screen::ContactLimit => Some(AppScreen::ContactLimit),
            // MyInfoEntryDetail is engine-driven; AppEngine already has the right screen
            Screen::MyInfoEntryDetail => None,
            _ => None,
        }
    }

    /// Go back to the previous screen.
    pub fn go_back(&mut self) {
        match self.screen {
            // Can't go back from onboarding until identity is configured
            Screen::SetupWelcome => {
                // Stay on setup screen
            }
            Screen::SetupCreateIdentity => {
                self.screen = Screen::SetupWelcome;
            }
            Screen::SetupAddFields => {
                self.screen = Screen::SetupCreateIdentity;
            }
            Screen::SetupSecurity => {
                self.screen = Screen::SetupAddFields;
            }
            Screen::SetupReady => {
                self.screen = Screen::SetupSecurity;
            }
            Screen::ContactDuplicates => {
                self.screen = Screen::Contacts;
                self.duplicates_state = DuplicatesState::default();
            }
            Screen::ContactMerge => {
                self.screen = Screen::ContactDuplicates;
                self.merge_state = MergeState::default();
            }
            Screen::ContactLimit => {
                self.screen = Screen::Contacts;
                self.contact_limit_state = ContactLimitState::default();
            }
            // Can't escape the lock screen — must enter PIN
            Screen::Lock => {
                // Stay on lock screen
            }
            Screen::Contacts
            | Screen::Exchange
            | Screen::Settings
            | Screen::Help
            | Screen::Recovery
            | Screen::Sync
            | Screen::Delivery
            | Screen::TorSettings => {
                self.screen = Screen::MyInfo;
            }
            Screen::Devices => {
                self.device_link_result = None;
                self.revoke_confirm = false;
                self.screen = Screen::MyInfo;
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
            // From Backup, go back to onboarding/setup if no identity, otherwise Home
            Screen::Backup => {
                if self.app_engine.vauchi().has_identity() {
                    self.screen = Screen::MyInfo;
                } else {
                    self.screen = Screen::SetupWelcome;
                }
            }
            Screen::ContactDetail => {
                self.selected_contact_id = None;
                self.contact_delete_confirm = false;
                self.screen = Screen::Contacts;
            }
            Screen::ContactEdit => {
                // Back from edit goes to detail
                self.screen = Screen::ContactDetail;
                self.render_state = Default::default();
            }
            Screen::ContactVisibility => {
                self.screen = Screen::ContactDetail;
                self.visibility_state = VisibilityState::default();
            }
            Screen::AddField => {
                // Return to onboarding wizard if we came from there
                self.screen = if self.onboarding_state.identity_created {
                    Screen::SetupAddFields
                } else {
                    Screen::MyInfo
                };
                self.add_field_state = AddFieldState::default();
            }
            Screen::EditField => {
                self.screen = Screen::MyInfo;
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
            Screen::Groups => {
                self.screen = Screen::MyInfo;
                self.groups_state = GroupsState::default();
            }
            Screen::GroupDetail => {
                self.screen = Screen::Groups;
                self.groups_state.show_group_detail = false;
                self.groups_state.selected_contact_in_group = 0;
            }
            Screen::ActionMenu => {
                self.screen = Screen::ContactDetail;
                self.action_menu_state = ActionMenuState::default();
            }
            _ => {}
        }
        self.input_mode = InputMode::Normal;
        self.sync_nav_index();
    }

    /// Keep `focus.nav_index` in sync with the current screen so that
    /// Left/Right navigation from Content zone lands on the correct tab.
    fn sync_nav_index(&mut self) {
        if let Some(idx) = self.nav_index_for_screen() {
            self.focus.nav_index = idx;
        }
    }

    /// Maps the current screen (or its parent tab) to a NavBar index.
    /// Returns `None` for screens that don't correspond to a top-level tab
    /// (onboarding, lock, etc.).
    fn nav_index_for_screen(&self) -> Option<usize> {
        match self.screen {
            // Direct tab screens
            Screen::Exchange => Some(0),
            Screen::MyInfo
            | Screen::MyInfoEntryDetail
            | Screen::AddField
            | Screen::EditField
            | Screen::Groups
            | Screen::GroupDetail => Some(1),
            Screen::Contacts
            | Screen::ContactDetail
            | Screen::ContactEdit
            | Screen::ContactVisibility
            | Screen::ContactDuplicates
            | Screen::ContactMerge
            | Screen::ContactLimit
            | Screen::ActionMenu => Some(2),
            Screen::Settings
            | Screen::Privacy
            | Screen::Support
            | Screen::Emergency
            | Screen::Duress
            | Screen::EditName
            | Screen::EditRelayUrl
            | Screen::Backup
            | Screen::Devices
            | Screen::Delivery
            | Screen::TorSettings
            | Screen::Sync
            | Screen::Recovery => Some(3),
            Screen::Help => Some(4),
            _ => None,
        }
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
