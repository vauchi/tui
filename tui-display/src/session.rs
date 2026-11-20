// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use crossterm::event::KeyEvent;
use ratatui_core::{layout::Rect, terminal::Frame};
use vauchi_contract::{Command, Event, InputMode, MotionPreference};

use crate::ui::presentation_input::{InteractionState, KeyOutcome};
use crate::ui::presentation_protocol::PresentationState;
use crate::ui::presentation_renderer;

#[derive(Default)]
pub struct TerminalSession {
    presentation: PresentationState,
    interaction: InteractionState,
    reported_window: Option<(u16, u16)>,
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
        self.presentation.apply(commands)
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
        presentation_renderer::draw_feedback(frame, area, view.status, view.alert);
    }
}
