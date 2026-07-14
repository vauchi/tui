// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Application State

mod navigation;
pub mod state;

pub use state::*;

use vauchi_app::ui::{AppEngine, AppScreen, LockScreenEngine, OnboardingEngine};

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
    if let Ok(id) = std::env::var("VAUCHI_THEME")
        && let Some(pos) = theme_ids.iter().position(|t| t == &id)
        && let Some(t) = get_tui_theme(&id)
    {
        return (t, pos);
    }

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
    /// Active TUI-only modal layered over `screen` (action menu, import
    /// dialog). `None` when no overlay is open. See [`Overlay`].
    pub overlay: Option<Overlay>,
    /// Input mode
    pub input_mode: InputMode,
    /// Whether the app should quit (for future use)
    pub should_quit: bool,
    /// Status message
    pub status_message: Option<String>,
    /// When the status message was set (for auto-clear after 3 seconds).
    pub status_message_time: Option<std::time::Instant>,
    /// Undo action ID from the most recent `ShowToast` (cleared with status).
    pub undo_action_id: Option<String>,
    /// Core-owned label for the most recent toast undo action.
    pub undo_label: Option<String>,
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
    /// Selected field index in the ContactVisibility screen (terminal-only).
    /// Contact id comes from `app_engine.current_app_screen()`.
    pub selected_visibility_field: usize,
    /// Contact search query
    pub contact_search_query: String,
    /// Contact search mode active
    pub contact_search_mode: bool,
    /// Sync state
    pub sync_state: SyncState,
    /// Privacy/GDPR screen state
    /// Action menu state (popup for field actions)
    pub action_menu_state: ActionMenuState,
    /// Lock screen state
    pub lock_state: LockState,
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
    /// Contact import state
    pub import_state: ImportState,
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
    /// ADR-044 Am2a: next scheduled wakeup time, derived from the most
    /// recent `Command::ScheduleWakeup` emitted by `on_wakeup()`.
    pub next_wakeup: Option<std::time::Instant>,
    /// Channel receiver for background sync results.
    pub sync_rx: Option<std::sync::mpsc::Receiver<SyncResult>>,
}

impl App {
    /// Create a new application.
    ///
    /// AppEngine owns the single Vauchi<T> instance — all identity/contact/card
    /// operations go through it. Relay sync uses the standalone sync_service module.
    pub fn new(app_engine: AppEngine, relay_url: String, data_dir: std::path::PathBuf) -> Self {
        // Determine initial screen from AppEngine's Vauchi state
        let has_identity = app_engine.vauchi().has_identity();
        let initial_screen: AppScreen = if !has_identity {
            AppScreen::Onboarding
        } else if app_engine.vauchi().is_password_enabled().unwrap_or(false) {
            AppScreen::Lock
        } else {
            // AppEngine's dynamic default: MyInfo (0 contacts) or Contacts (>=1)
            match app_engine.default_screen() {
                AppScreen::Contacts => AppScreen::Contacts,
                _ => AppScreen::MyInfo,
            }
        };

        // Bootstrap AppEngine to the correct initial screen.
        //
        // `set_initial_screen` (core!737) does NOT push the prior screen
        // (`AppScreen::Onboarding`) to `nav_history`. Without it, the
        // user's first `navigate_back` lands on Onboarding instead of
        // their expected parent. Only call this once, during bootstrap.
        // Bootstrap the engine to the initial screen — app_engine is the
        // single source of truth (it tracks Onboarding/Lock too).
        let mut app_engine = app_engine;
        app_engine.set_initial_screen(initial_screen.clone());

        // Create engines for initial screen
        let onboarding_engine = if initial_screen == AppScreen::Onboarding {
            Some(OnboardingEngine::new())
        } else {
            None
        };
        let lock_engine = if initial_screen == AppScreen::Lock {
            Some(LockScreenEngine::new(
                vauchi_app::ui::DEFAULT_LOCK_MAX_ATTEMPTS,
            ))
        } else {
            None
        };

        let theme_ids: Vec<String> = list_themes().into_iter().map(|(id, _, _)| id).collect();

        // Load saved theme or detect from environment
        let (theme, theme_index) = load_saved_theme(&theme_ids);

        App {
            overlay: None,
            input_mode: InputMode::Normal,
            should_quit: false,
            status_message: None,
            status_message_time: None,
            undo_action_id: None,
            undo_label: None,
            alert_message: None,
            selected_contact: 0,
            selected_contact_id: None,
            selected_field: 0,
            selected_contact_field: 0,
            input_buffer: String::new(),
            selected_visibility_field: 0,
            contact_search_query: String::new(),
            contact_search_mode: false,
            sync_state: SyncState::default(),
            action_menu_state: ActionMenuState::default(),
            lock_state: LockState::default(),
            onboarding_state: OnboardingState::default(),
            app_engine,
            onboarding_engine,
            lock_engine,
            render_state: ScreenRenderState::default(),
            focus: FocusManager::new(),
            import_state: ImportState::default(),
            i18n: detect_locale(),
            theme,
            theme_index,
            theme_ids,
            relay_url,
            data_dir,
            exchange_scan_pending: false,
            next_wakeup: None,
            sync_rx: None,
        }
    }

    /// Builds an owned `SyncRequest` that can be sent to a background thread.
    ///
    /// Returns `None` if there is no identity or storage key.
    pub fn build_sync_request(&self) -> Option<crate::sync_service::SyncRequest> {
        let vauchi = self.app_engine.vauchi();
        let _ = vauchi.identity()?; // gate: no request without identity
        let storage_key = vauchi.config().storage_key.clone()?;
        Some(crate::sync_service::SyncRequest {
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
        self.undo_action_id = None;
        self.undo_label = None;
    }

    /// Set a status message with an optional undo action.
    pub fn set_status_with_undo(
        &mut self,
        msg: impl Into<String>,
        undo_action_id: Option<String>,
        undo_label: Option<String>,
    ) {
        self.status_message = Some(msg.into());
        self.status_message_time = Some(std::time::Instant::now());
        self.undo_action_id = undo_action_id;
        self.undo_label = undo_label;
    }

    /// Clear the status message.
    #[allow(dead_code)]
    pub fn clear_status(&mut self) {
        self.status_message = None;
        self.status_message_time = None;
        self.undo_action_id = None;
        self.undo_label = None;
    }

    /// Applies a completed background sync result to app state.
    pub fn apply_sync_result(&mut self, result: SyncResult) {
        use vauchi_core::types::AhaMomentType;

        self.sync_state.is_syncing = false;

        if result.success {
            let summary = format!(
                "{} received, {} sent, {} acked",
                result.cards_updated, result.updates_sent, result.acknowledged
            );
            self.set_status(format!("Sync complete: {}", summary));

            // Update pending count
            self.sync_state.pending_updates =
                self.app_engine.vauchi().pending_update_count().unwrap_or(0);

            // TODO(HUMBLE): D — TUI decides AhaMomentType::FirstUpdateReceived from sync counts (see _private/docs/problems/2026-07-06-desktop-tui-web-domain-shell-violations)
            // Check for aha moments based on sync results
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
            let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
            self.set_status(format!(
                "Sync failed: {}. Changes saved locally and will sync when connected.",
                error_msg
            ));
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

    /// Core-driven wakeup tick (ADR-044 Am2a).
    ///
    /// Replaces the frontend-owned periodic `poll_notifications()` loop with
    /// `AppEngine::on_wakeup()`. Core decides what work is due and emits the
    /// next `Command::ScheduleWakeup`; the TUI only executes the native timer
    /// and re-invokes this method when that schedule fires.
    ///
    /// Emergency and duress alerts are shown as modal alerts (blocking);
    /// core supplies the distinct copy so the recipient can respond to a
    /// coerced sender appropriately. Others (e.g. contact added) are shown
    /// as status messages (toasts).
    // TODO(HUMBLE): D/W — NotificationCategory decides modal alert vs toast (see _private/docs/problems/2026-07-06-desktop-tui-web-domain-shell-violations)
    pub fn tick_notifications(&mut self) {
        use vauchi_app::notification_types::NotificationCategory;
        use vauchi_core::Command;

        let notifications = self.app_engine.on_wakeup();
        for n in notifications {
            match n.category {
                NotificationCategory::EmergencyAlert | NotificationCategory::DuressAlert => {
                    self.alert_message = Some((n.title, n.body));
                }
                NotificationCategory::ContactAdded => {
                    self.set_status(format!("{} — {}", n.title, n.body));
                }
                NotificationCategory::CardUpdate => {
                    self.set_status(format!("{} — {}", n.title, n.body));
                }
            }
        }

        // Drain pending commands for the next wakeup schedule. Core owns *when*
        // the heartbeat is due; the shell only executes the native timer.
        for cmd in self.app_engine.drain_pending_commands() {
            if let Command::ScheduleWakeup { earliest_secs, .. } = cmd {
                self.next_wakeup = Some(
                    std::time::Instant::now()
                        + std::time::Duration::from_secs(earliest_secs.into()),
                );
            }
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

    fn test_app_engine() -> (vauchi_app::ui::AppEngine, tempfile::TempDir) {
        use vauchi_core::{SymmetricKey, Vauchi, VauchiConfig};
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_path_buf();
        let config = VauchiConfig::with_storage_path(path.join("vauchi.db"))
            .with_storage_key(SymmetricKey::generate());
        let mut app_engine = vauchi_app::ui::AppEngine::new(Vauchi::new(config).expect("vauchi"));
        app_engine
            .vauchi_mut()
            .create_identity("Test User")
            .expect("create identity");
        (app_engine, dir)
    }

    /// `tick_notifications` bootstraps the ADR-044 Am2a wakeup loop: it calls
    /// `AppEngine::on_wakeup()`, drains pending commands, and stores the next
    /// `ScheduleWakeup` deadline on `App::next_wakeup`.
    // @internal
    #[test]
    fn tick_notifications_bootstraps_next_wakeup() {
        let (app_engine, _dir) = test_app_engine();
        let mut app = App::new(
            app_engine,
            "wss://relay.vauchi.app".into(),
            std::path::PathBuf::from("."),
        );
        assert!(app.next_wakeup.is_none());

        app.tick_notifications();

        assert!(
            app.next_wakeup.is_some(),
            "on_wakeup must emit a ScheduleWakeup command"
        );
        assert!(
            app.next_wakeup.unwrap() > std::time::Instant::now(),
            "next_wakeup must be in the future"
        );
    }
}
