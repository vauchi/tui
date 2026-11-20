// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use crossterm::event::KeyEvent;
use ratatui_core::{layout::Rect, terminal::Frame};
use vauchi_contract::{Command, Event, InputMode, MotionPreference};

use crate::file_pick::PathLine;
use crate::ui::presentation_input::{InteractionState, KeyOutcome};
use crate::ui::presentation_protocol::PresentationState;
use crate::ui::presentation_renderer;

#[derive(Default)]
pub struct TerminalSession {
    presentation: PresentationState,
    interaction: InteractionState,
    reported_window: Option<(u16, u16)>,
    path_line: Option<PathLine>,
    reports: Vec<Event>,
    said_it_answers_picks: bool,
}

#[derive(Default)]
pub struct SessionView<'a> {
    pub loading_label: &'a str,
    pub effect_prompt: Option<(&'a Command, &'a str)>,
    pub status: Option<&'a str>,
    pub alert: Option<&'a (String, String)>,
}

impl TerminalSession {
    /// Returns commands that require the host's native adapters.
    pub fn apply_commands(&mut self, commands: &[Command]) -> Vec<Command> {
        let mut for_the_host = self.presentation.apply(commands);
        for_the_host.retain(|command| !self.answers_pick(command));
        for_the_host
    }

    /// Whether `command` is a pick this display answers itself: a file
    /// pick from a path line, an image pick as unavailable, because it
    /// carries no title to show a line under (vauchi/private#1233).
    fn answers_pick(&mut self, command: &Command) -> bool {
        let unavailable = |transport: &str| Event::HardwareUnavailable {
            transport: transport.into(),
        };
        match command {
            Command::PickFile {
                request_id,
                title,
                max_bytes,
                ..
            } => {
                self.path_line = Some(PathLine::new(request_id.clone(), title.clone(), *max_bytes));
            }
            Command::CancelFilePick { request_id } => {
                if self
                    .path_line
                    .as_ref()
                    .is_some_and(|line| line.answers(request_id))
                {
                    self.path_line = None;
                }
            }
            Command::ImagePickFromFile => self.reports.push(unavailable("image_file")),
            Command::ImagePickFromLibrary => self.reports.push(unavailable("photo_library")),
            _ => return false,
        }
        true
    }

    /// What the display has to tell Core unasked: once, that it answers
    /// file picks, and each pick it could only answer as unavailable.
    pub fn take_reports(&mut self) -> Vec<Event> {
        let mut reports = Vec::new();
        if !std::mem::replace(&mut self.said_it_answers_picks, true) {
            reports.push(Event::FilePickerCapabilitiesChanged {
                correlated_results: true,
            });
        }
        reports.append(&mut self.reports);
        reports
    }

    /// The window to report to Core, or `None` while it is the one last
    /// reported. Core rejects a pane switch until it has one, so the host
    /// asks before each input wait (vauchi/private#1310).
    pub fn window_report(&mut self, columns: u16, rows: u16) -> Option<Event> {
        if self.reported_window == Some((columns, rows)) {
            return None;
        }
        self.reported_window = Some((columns, rows));
        let reduced_motion = std::env::var("VAUCHI_REDUCED_MOTION")
            .is_ok_and(|value| matches!(value.as_str(), "1" | "true" | "yes"));
        Some(Event::PresentationEnvironmentChanged {
            available_width: u32::from(columns),
            available_height: u32::from(rows),
            input_modes: vec![InputMode::Keyboard],
            motion: if reduced_motion {
                MotionPreference::Reduced
            } else {
                MotionPreference::Full
            },
            color_scheme: None,
        })
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> KeyOutcome {
        if let Some(line) = &mut self.path_line {
            let Some(answer) = line.key(key) else {
                return KeyOutcome::Consumed;
            };
            self.path_line = None;
            return KeyOutcome::Events(vec![answer]);
        }
        self.interaction.key_outcome(&self.presentation, key)
    }

    pub fn presentation(&self) -> &PresentationState {
        &self.presentation
    }

    pub fn draw(&self, frame: &mut Frame, area: Rect, view: SessionView<'_>) {
        let selected = if self.presentation.overlay().is_some() {
            self.interaction.selected_overlay()
        } else {
            self.interaction.selected_context()
        };
        presentation_renderer::draw(
            frame,
            area,
            &self.presentation,
            selected,
            self.interaction.selected_surface_target(&self.presentation),
            view.loading_label,
        );
        if let Some((effect, input)) = view.effect_prompt {
            presentation_renderer::draw_effect_prompt(frame, area, effect, input);
        }
        if let Some(line) = &self.path_line {
            presentation_renderer::draw_path_line(frame, area, line.title(), line.typed());
        }
        presentation_renderer::draw_feedback(frame, area, view.status, view.alert);
    }
}
