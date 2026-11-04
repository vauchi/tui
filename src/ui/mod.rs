// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Generic terminal projection of Core presentation commands.

// INLINE_TEST_REQUIRED: engine-backed drawing tests inspect the app's private
// display and its locale adapter.
#[cfg(test)]
pub(crate) use vauchi_tui_display::ui::presentation_protocol;
#[cfg(test)]
pub(crate) mod presentation_renderer;
pub mod screen_catalog;
use ratatui::Frame;

use crate::app::App;

pub fn draw_presentation(frame: &mut Frame, app: &App) {
    app.display.draw(
        frame,
        frame.area(),
        vauchi_tui_display::SessionView {
            loading_label: &crate::i18n::I18n::default().t("app.loading"),
            effect_prompt: app
                .presentation_effects
                .front()
                .map(|effect| (effect, app.input_buffer.as_str())),
            status: app.status_message.as_deref(),
            alert: app.alert_message.as_ref(),
        },
    );
}

#[cfg(test)]
mod draw_tests;
#[cfg(test)]
mod locked_app_render_tests;
