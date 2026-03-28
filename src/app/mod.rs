// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Application State

mod navigation;
pub mod state;

pub use state::*;

use vauchi_app::ui::{AppEngine, AppScreen, LockScreenEngine, OnboardingEngine};
use vauchi_core::api::DeviceLinkResult;

use crate::i18n::I18n;
use crate::sync_service::SyncResult;
use crate::theme::{TuiTheme, get_default_tui_theme, get_tui_theme, list_themes};
use crate::ui::focus::FocusManager;
use crate::ui::widgets::screen_renderer::ScreenRenderState;

/// Path to theme config file.
fn theme_config_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("vauchi").join("theme"))
}

/// Load saved theme ID from config, or detect from VAUCHI_THEME env var,
/// or default to dark.
fn load_saved_theme(theme_ids: &[String]) -> (TuiTheme, usize) {
    // Check env var first
    if let Ok(id) = std::env::var("VAUCHI_THEME")
        && let Some(pos) = theme_ids.iter().position(|t| t == &id)
        && let Some(t) = get_tui_theme(&id)
    {
        return (t, pos);
    }

    // Check saved config
    if let Some(path) = theme_config_path()
        && let Ok(id) = std::fs::read_to_string(&path)
    {
        let id = id.trim();
        if let Some(pos) = theme_ids.iter().position(|t| t == id)
            && let Some(t) = get_tui_theme(id)
        {
            return (t, pos);
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
            // Extract language code from e.g. "de_CH.UTF-8" -> "de"
            let code = val.split('_').next().unwrap_or(&val);
            let code = code.split('.').next().unwrap_or(code);
            if !code.is_empty() && code != "C" && code != "POSIX" {
                return I18n::from_code(code);
            }
        }
    }
    I18n::default()
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
    // -- Core-driven workflow engines --
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
    /// ADR-031: Whether the exchange is waiting for QR scan input (paste).
    pub exchange_scan_pending: bool,
    /// Channel receiver for background sync results.
    pub sync_rx: Option<std::sync::mpsc::Receiver<SyncResult>>,
    /// Channel receiver for background relay connection test results.
    pub relay_test_rx: Option<std::sync::mpsc::Receiver<anyhow::Result<bool>>>,
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
            exchange_scan_pending: false,
            sync_rx: None,
            relay_test_rx: None,
        }
    }

    /// Performs a full sync with the relay server via WebSocket (blocking).
    #[allow(dead_code)]
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
                };
            }
        };
        crate::sync_service::sync(identity, vauchi, &self.relay_url)
    }

    /// Tests the relay connection (blocking — prefer background thread).
    #[allow(dead_code)]
    pub fn test_relay_connection(&self) -> anyhow::Result<bool> {
        crate::sync_service::test_relay_connection(&self.relay_url)
    }

    /// Builds an owned `SyncRequest` that can be sent to a background thread.
    ///
    /// Returns `None` if there is no identity or storage key.
    pub fn build_sync_request(&self) -> Option<crate::sync_service::SyncRequest> {
        let vauchi = self.app_engine.vauchi();
        let identity = vauchi.identity()?;
        let storage_key = vauchi.config().storage_key.clone()?;
        Some(crate::sync_service::SyncRequest {
            identity_bytes: identity.to_storage_bytes(),
            storage_path: vauchi.config().storage_path.clone(),
            storage_key,
            relay_url: self.relay_url.clone(),
        })
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

    /// Applies a completed background sync result to app state.
    pub fn apply_sync_result(&mut self, result: SyncResult) {
        use vauchi_core::types::AhaMomentType;

        self.sync_state.is_syncing = false;

        if result.success {
            self.sync_state.connected = true;
            let summary = format!(
                "+{} contacts, {} updated, {} sent",
                result.contacts_added, result.cards_updated, result.updates_sent
            );
            self.sync_state.last_result = Some(summary.clone());
            self.sync_state
                .sync_log
                .push(format!("Sync complete: {}", summary));
            self.set_status(format!("Sync complete: {}", summary));

            // Update pending count
            self.sync_state.pending_updates =
                self.app_engine.vauchi().pending_update_count().unwrap_or(0);

            // Check for aha moments based on sync results
            if result.contacts_added > 0
                && let Ok(Some(moment)) = self
                    .app_engine
                    .vauchi()
                    .try_trigger_aha_moment(AhaMomentType::FirstContactAdded)
            {
                self.set_status(format!("★ {} — {}", moment.title(), moment.message()));
            }
            if result.cards_updated > 0
                && let Ok(Some(moment)) = self
                    .app_engine
                    .vauchi()
                    .try_trigger_aha_moment(AhaMomentType::FirstUpdateReceived)
            {
                self.set_status(format!("★ {} — {}", moment.title(), moment.message()));
            }
            if result.updates_sent > 0
                && let Ok(Some(moment)) = self
                    .app_engine
                    .vauchi()
                    .try_trigger_aha_moment(AhaMomentType::FirstOutboundDelivered)
            {
                self.set_status(format!("★ {} — {}", moment.title(), moment.message()));
            }

            // Invalidate cached screens since sync may have changed contacts/cards
            self.invalidate_engines();
        } else {
            self.sync_state.connected = false;
            let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
            self.sync_state.last_result = Some(format!("Failed: {}", error_msg));
            self.sync_state
                .sync_log
                .push(format!("Sync failed: {}", error_msg));
            self.set_status(format!(
                "Sync failed: {}. Changes saved locally and will sync when connected.",
                error_msg
            ));
        }
    }

    /// Applies a completed background relay test result to app state.
    pub fn apply_relay_test_result(&mut self, result: anyhow::Result<bool>) {
        match result {
            Ok(true) => {
                self.sync_state.connected = true;
                self.sync_state
                    .sync_log
                    .push("Relay connection test: OK".to_string());
                self.set_status("Relay connection successful!");
            }
            Ok(false) | Err(_) => {
                self.sync_state.connected = false;
                self.sync_state
                    .sync_log
                    .push("Relay connection test: FAILED".to_string());
                self.set_status(
                    "Relay connection failed. Check your network or relay URL in Settings.",
                );
            }
        }
    }

    /// Auto-clear status message after 3 seconds.
    pub fn tick_status(&mut self) {
        if let Some(time) = self.status_message_time
            && time.elapsed() >= std::time::Duration::from_secs(3)
        {
            self.clear_status();
        }
    }
}

// INLINE_TEST_REQUIRED: Tests need access to private detect_locale() and App internals
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use vauchi_app::i18n::Locale;

    /// Env-var tests must be serialised — set_var/remove_var is process-global.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn clear_locale_env() {
        // TODO: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::remove_var("LC_ALL") };
        // TODO: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::remove_var("LC_MESSAGES") };
        // TODO: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::remove_var("LANG") };
    }

    #[test]
    fn test_detect_locale_german() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_locale_env();
        // TODO: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::set_var("LANG", "de_CH.UTF-8") };
        let i18n = detect_locale();
        assert_eq!(i18n.locale(), Locale::German);
        clear_locale_env();
    }

    #[test]
    fn test_detect_locale_french() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_locale_env();
        // TODO: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::set_var("LANG", "fr_FR.UTF-8") };
        let i18n = detect_locale();
        assert_eq!(i18n.locale(), Locale::French);
        clear_locale_env();
    }

    #[test]
    fn test_detect_locale_lc_all_overrides() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_locale_env();
        // TODO: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::set_var("LANG", "en_US.UTF-8") };
        // TODO: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::set_var("LC_ALL", "es_ES.UTF-8") };
        let i18n = detect_locale();
        assert_eq!(i18n.locale(), Locale::Spanish);
        clear_locale_env();
    }

    #[test]
    fn test_detect_locale_fallback_english() {
        let _guard = ENV_LOCK.lock().unwrap();
        clear_locale_env();
        // TODO: Audit that the environment access only happens in single-threaded code.
        unsafe { std::env::set_var("LANG", "C") };
        let i18n = detect_locale();
        assert_eq!(i18n.locale(), Locale::English);
        clear_locale_env();
    }
}
