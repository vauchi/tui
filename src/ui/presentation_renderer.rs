// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use super::presentation_protocol::PresentationState;
use crate::i18n::I18n;
use ratatui::{Frame, layout::Rect};

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
        &I18n::default().t("app.loading"),
    );
}
