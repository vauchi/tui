// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Humble terminal session around Core's application reducer.

pub mod capabilities;
pub use capabilities::tui_device_capabilities;

use std::collections::VecDeque;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use vauchi_app::ui::{AppEngine, AppPresentationError};
use vauchi_core::{Command, Event, InputMode, MotionPreference};

use crate::sync_service::SyncResult;
use crate::ui::presentation_input::InteractionState;
use crate::ui::presentation_protocol::PresentationState;

pub struct App {
    pub app_engine: AppEngine,
    pub(crate) presentation: PresentationState,
    pub(crate) presentation_interaction: InteractionState,
    pub(crate) presentation_effects: VecDeque<Command>,
    pub(crate) input_buffer: String,
    pub(crate) alert_message: Option<(String, String)>,
    pub(crate) status_message: Option<String>,
    status_message_time: Option<Instant>,
    pub(crate) should_quit: bool,
    pub next_wakeup: Option<Instant>,
    pub sync_rx: Option<mpsc::Receiver<SyncResult>>,
    pub data_dir: std::path::PathBuf,
    pub relay_url: String,
    url_opener: fn(&str) -> bool,
}

impl App {
    pub fn new(mut app_engine: AppEngine, relay_url: String, data_dir: std::path::PathBuf) -> Self {
        // Core resolves where to land. Deriving it here from identity and
        // password state made this shell interpret domain state to pick
        // navigation, which ADR-066 reserves to Core — and required
        // importing `AppScreen`, retired from the shell boundary.
        app_engine.bootstrap();
        let commands = app_engine
            .initial_commands()
            .expect("fresh AppEngine must prepare its initial presentation");
        let mut presentation = PresentationState::default();
        let effects = presentation.apply(&commands);
        let mut app = Self {
            app_engine,
            presentation,
            presentation_interaction: InteractionState::default(),
            presentation_effects: VecDeque::new(),
            input_buffer: String::new(),
            alert_message: None,
            status_message: None,
            status_message_time: None,
            should_quit: false,
            next_wakeup: None,
            sync_rx: None,
            data_dir,
            relay_url,
            url_opener: open_external_url,
        };
        for effect in effects {
            app.apply_native_effect(effect);
        }
        app
    }

    pub(crate) fn dispatch_presentation_event(
        &mut self,
        event: Event,
    ) -> Result<(), AppPresentationError> {
        let commands = self.app_engine.dispatch(event)?;
        self.apply_presentation_commands(commands);
        Ok(())
    }

    /// Core prepares the alert for a rejected event (ADR-045 Am1): the
    /// error's own text can echo user input, so it is never shown.
    pub(crate) fn present_rejection(&mut self, error: &AppPresentationError) {
        let commands = self.app_engine.reject_dispatch(error);
        self.apply_presentation_commands(commands);
    }

    pub(crate) fn apply_presentation_commands(&mut self, commands: Vec<Command>) {
        for effect in self.presentation.apply(&commands) {
            self.apply_native_effect(effect);
        }
    }

    pub fn report_presentation_environment(&mut self, available_width: u32, available_height: u32) {
        let reduced_motion = std::env::var("VAUCHI_REDUCED_MOTION")
            .is_ok_and(|value| matches!(value.as_str(), "1" | "true" | "yes"));
        let _ = self.dispatch_presentation_event(Event::PresentationEnvironmentChanged {
            available_width,
            available_height,
            input_modes: vec![InputMode::Keyboard],
            motion: if reduced_motion {
                MotionPreference::Reduced
            } else {
                MotionPreference::Full
            },
        });
    }

    pub fn tick_status(&mut self) {
        self.tick_status_at(Instant::now());
    }

    fn tick_status_at(&mut self, now: Instant) {
        if self
            .status_message_time
            .is_some_and(|time| now.saturating_duration_since(time) >= Duration::from_secs(3))
        {
            self.status_message = None;
            self.status_message_time = None;
        }
    }

    pub fn status_is_flashing(&self) -> bool {
        self.status_is_flashing_at(Instant::now())
    }

    fn status_is_flashing_at(&self, now: Instant) -> bool {
        self.status_message_time
            .is_some_and(|time| now.saturating_duration_since(time) < Duration::from_millis(600))
    }

    pub fn tick_notifications(&mut self) {
        for notification in self.app_engine.on_wakeup() {
            self.set_status(format!("{} — {}", notification.title, notification.body));
        }
        let commands = self.app_engine.drain_pending_commands();
        self.apply_presentation_commands(commands);
    }

    pub fn apply_sync_result(&mut self, result: SyncResult) {
        if result.success {
            self.set_status(format!(
                "Sync complete: {} received, {} sent, {} acknowledged",
                result.cards_updated, result.updates_sent, result.acknowledged
            ));
        } else {
            self.set_status(format!(
                "Sync failed: {}",
                result.error.unwrap_or_else(|| "Unknown error".into())
            ));
        }
        let _ = self.dispatch_presentation_event(Event::PresentationInvalidated);
    }

    pub fn set_url_opener(&mut self, opener: fn(&str) -> bool) {
        self.url_opener = opener;
    }

    fn apply_native_effect(&mut self, effect: Command) {
        match effect {
            Command::PresentAlert { alert } => {
                self.alert_message = Some((alert.title, alert.message));
            }
            Command::ShowToast { toast } => self.set_status(toast.message),
            Command::OpenExternalUrl { url } => {
                if !(self.url_opener)(&url) {
                    self.set_status(format!("Unable to open {url}"));
                }
            }
            Command::PostNotification { notification } => {
                self.set_status(format!("{} — {}", notification.title, notification.body));
            }
            Command::ScheduleWakeup {
                earliest_secs,
                earliest_millis,
                ..
            } => {
                let delay = earliest_millis.map_or_else(
                    || Duration::from_secs(earliest_secs.into()),
                    |millis| Duration::from_millis(millis.into()),
                );
                self.next_wakeup = Some(Instant::now() + delay);
            }
            Command::ExportFile { file } => {
                let destination = self.data_dir.join(&file.suggested_name);
                match std::fs::write(&destination, file.data) {
                    Ok(()) => self.set_status(format!("Saved {}", destination.display())),
                    Err(error) => {
                        self.alert_message = Some(("Export failed".into(), error.to_string()));
                    }
                }
            }
            Command::ResetApplication => self.should_quit = true,
            other => self.presentation_effects.push_back(other),
        }
    }

    fn set_status(&mut self, message: impl Into<String>) {
        self.set_status_at(message, Instant::now());
    }

    fn set_status_at(&mut self, message: impl Into<String>, now: Instant) {
        self.status_message = Some(message.into());
        self.status_message_time = Some(now);
    }
}

fn open_external_url(url: &str) -> bool {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "start"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener)
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .is_ok()
}

// INLINE_TEST_REQUIRED: bootstrap tests inspect the App's private presentation
// state after Core classifies the terminal environment.
#[cfg(test)]
mod tests {
    use super::*;
    use vauchi_core::api::Vauchi;

    fn app() -> App {
        App::new(
            AppEngine::new(Vauchi::in_memory().expect("in-memory core")),
            "wss://relay.vauchi.app".into(),
            std::path::PathBuf::from("."),
        )
    }

    #[test]
    fn bootstrap_uses_core_generic_presentation() {
        assert!(app().presentation.surface().is_some());
    }

    #[test]
    fn environment_is_reduced_by_core_into_a_window_profile() {
        let mut app = app();
        app.report_presentation_environment(840, 600);
        assert_eq!(
            app.presentation.profile().unwrap().window_class,
            vauchi_core::WindowClass::Expanded
        );
    }

    // @internal
    #[test]
    fn a_status_flashes_for_600ms_and_clears_after_three_seconds() {
        let mut app = app();
        let set_at = Instant::now();
        app.set_status_at("Saved", set_at);

        assert!(app.status_is_flashing_at(set_at + Duration::from_millis(599)));
        assert!(!app.status_is_flashing_at(set_at + Duration::from_millis(600)));

        app.tick_status_at(set_at + Duration::from_millis(2999));
        assert_eq!(app.status_message.as_deref(), Some("Saved"));
        app.tick_status_at(set_at + Duration::from_secs(3));
        assert_eq!(app.status_message, None);
        assert!(!app.status_is_flashing_at(set_at + Duration::from_secs(3)));
    }

    // @internal
    #[test]
    fn the_live_clock_variants_read_the_same_status_timing() {
        let mut app = app();
        app.set_status("Fresh");
        assert!(app.status_is_flashing());
        app.tick_status();
        assert_eq!(app.status_message.as_deref(), Some("Fresh"));

        app.set_status_at("Stale", Instant::now() - Duration::from_secs(4));
        assert!(!app.status_is_flashing());
        app.tick_status();
        assert_eq!(app.status_message, None);
    }

    // @internal
    #[test]
    fn without_a_status_nothing_flashes_or_clears() {
        let mut app = app();
        let now = Instant::now();

        assert!(!app.status_is_flashing_at(now));
        app.tick_status_at(now + Duration::from_secs(10));
        assert_eq!(app.status_message, None);
    }

    // @internal
    #[test]
    fn a_sync_result_becomes_a_status_line() {
        let mut app = app();
        app.apply_sync_result(SyncResult {
            cards_updated: 3,
            updates_sent: 2,
            acknowledged: 1,
            success: true,
            error: None,
        });
        assert_eq!(
            app.status_message.as_deref(),
            Some("Sync complete: 3 received, 2 sent, 1 acknowledged")
        );

        app.apply_sync_result(SyncResult::error("boom"));
        assert_eq!(app.status_message.as_deref(), Some("Sync failed: boom"));

        app.apply_sync_result(SyncResult {
            cards_updated: 0,
            updates_sent: 0,
            acknowledged: 0,
            success: false,
            error: None,
        });
        assert_eq!(
            app.status_message.as_deref(),
            Some("Sync failed: Unknown error")
        );
    }

    // @internal
    #[test]
    fn a_url_the_opener_refuses_is_reported_and_an_accepted_one_is_silent() {
        let mut refused = app();
        refused.set_url_opener(|_| false);
        refused.apply_native_effect(Command::OpenExternalUrl {
            url: "https://vauchi.app".into(),
        });
        assert_eq!(
            refused.status_message.as_deref(),
            Some("Unable to open https://vauchi.app")
        );

        let mut accepted = app();
        accepted.set_url_opener(|_| true);
        accepted.apply_native_effect(Command::OpenExternalUrl {
            url: "https://vauchi.app".into(),
        });
        assert_eq!(accepted.status_message, None);
    }

    // @internal
    #[test]
    fn a_wakeup_request_is_scheduled_that_many_seconds_ahead() {
        let mut app = app();
        let before = Instant::now();
        app.apply_native_effect(Command::ScheduleWakeup {
            earliest_secs: 5,
            deadline_secs: 10,
            min_interval_secs: 1,
            earliest_millis: None,
        });
        let after = Instant::now();

        let wakeup = app.next_wakeup.expect("a wakeup is scheduled");
        assert!(wakeup.duration_since(before) >= Duration::from_secs(5));
        assert!(wakeup.duration_since(after) <= Duration::from_secs(5));
    }

    // @internal
    #[test]
    fn a_sub_second_wakeup_request_is_scheduled_in_milliseconds() {
        let mut app = app();
        let before = Instant::now();
        app.apply_native_effect(Command::ScheduleWakeup {
            earliest_secs: 0,
            deadline_secs: 1,
            min_interval_secs: 0,
            earliest_millis: Some(120),
        });
        let after = Instant::now();

        let wakeup = app.next_wakeup.expect("a wakeup is scheduled");
        assert!(wakeup.duration_since(before) >= Duration::from_millis(120));
        assert!(wakeup.duration_since(after) <= Duration::from_millis(120));
    }

    // @internal
    #[test]
    fn the_first_heartbeat_installs_cores_wakeup_schedule() {
        let mut app = app();
        assert_eq!(app.next_wakeup, None);

        app.tick_notifications();

        assert!(app.next_wakeup.is_some());
    }
}
