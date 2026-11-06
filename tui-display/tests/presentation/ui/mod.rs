// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

pub(crate) use vauchi_tui_display::ui::{presentation_input, presentation_protocol};

mod pictogram_glyph_tests;
mod presentation_choice_tests;
mod presentation_input_keys_tests;
mod presentation_input_tests;
mod presentation_protocol_tests;
mod presentation_renderer_edge_tests;
mod presentation_renderer_tests;
mod presentation_secret_input_tests;
pub(crate) mod test_colors;

mod presentation_renderer {
    use super::presentation_protocol::PresentationState;
    use ratatui_core::{layout::Rect, terminal::Frame};
    pub(crate) use vauchi_tui_display::ui::presentation_renderer::{
        draw_effect_prompt, draw_feedback,
    };

    pub(crate) fn draw(
        frame: &mut Frame,
        area: Rect,
        state: &PresentationState,
        selected_action: usize,
        selected_surface_target: Option<usize>,
    ) {
        vauchi_tui_display::ui::presentation_renderer::draw(
            frame,
            area,
            state,
            selected_action,
            selected_surface_target,
            "Loading",
        );
    }
}
