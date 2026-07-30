// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Humble terminal session around Core's application reducer.

use std::collections::VecDeque;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use vauchi_app::ui::{AppEngine, AppPresentationError, AppScreen};
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
        let initial_screen = if !app_engine.vauchi().has_identity() {
            AppScreen::Onboarding
        } else if app_engine.vauchi().is_password_enabled().unwrap_or(false) {
            AppScreen::Lock
        } else {
            app_engine.default_screen()
        };
        app_engine.set_initial_screen(initial_screen);
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
        if self
            .status_message_time
            .is_some_and(|time| time.elapsed() >= Duration::from_secs(3))
        {
            self.status_message = None;
            self.status_message_time = None;
        }
    }

    pub fn status_is_flashing(&self) -> bool {
        self.status_message_time
            .is_some_and(|time| time.elapsed() < Duration::from_millis(600))
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
        self.app_engine.invalidate_all();
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
            Command::ScheduleWakeup { earliest_secs, .. } => {
                self.next_wakeup = Some(Instant::now() + Duration::from_secs(earliest_secs.into()));
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
        self.status_message = Some(message.into());
        self.status_message_time = Some(Instant::now());
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
}
