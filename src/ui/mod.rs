// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Generic terminal projection of Core presentation commands.

pub(crate) mod presentation_input;
pub(crate) mod presentation_protocol;
pub(crate) mod presentation_renderer;

use ratatui::Frame;

use crate::app::App;

pub fn draw_presentation(frame: &mut Frame, app: &App) {
    let selected = if app.presentation.overlay().is_some() {
        app.presentation_interaction.selected_overlay()
    } else {
        app.presentation_interaction.selected_context()
    };
    presentation_renderer::draw(
        frame,
        frame.area(),
        &app.presentation,
        selected,
        app.presentation_interaction
            .selected_surface_row(&app.presentation),
    );
    if let Some(effect) = app.presentation_effects.front() {
        presentation_renderer::draw_effect_prompt(frame, frame.area(), effect, &app.input_buffer);
    }
    presentation_renderer::draw_feedback(
        frame,
        frame.area(),
        app.status_message.as_deref(),
        app.alert_message.as_ref(),
    );
}

// INLINE_TEST_REQUIRED: extracted white-box tests exercise crate-private input,
// reducer, and renderer state without exposing it to consumers.
#[cfg(test)]
#[path = "presentation_input_tests.rs"]
mod presentation_input_tests;

#[cfg(test)]
#[path = "presentation_protocol_tests.rs"]
mod presentation_protocol_tests;

#[cfg(test)]
#[path = "presentation_renderer_tests.rs"]
mod presentation_renderer_tests;
