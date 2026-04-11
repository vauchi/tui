// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Devices screen key handler.
//!
//! Device listing and revocation are now driven by `DeviceManagementEngine`
//! in core (ADR-022 InlineConfirm for irrevocable actions). This handler
//! maps TUI keys to engine actions.

use crossterm::event::KeyCode;

use crate::app::App;

pub(in crate::handlers::input) fn handle_devices_keys(app: &mut App, key: KeyCode) {
    // Handle QR overlay — Esc dismisses
    if app.device_link_result.is_some() {
        if key == KeyCode::Esc {
            app.device_link_result = None;
        }
        return;
    }

    // 'l' triggers device linking; all other keys (j/k/r/Enter/Esc) are handled
    // by the ScreenModel renderer's component-level key mapping (InlineConfirm, ActionList).
    if let KeyCode::Char('l') = key {
        match app.app_engine.vauchi().generate_device_link() {
            Ok(result) => {
                app.device_link_result = Some(result);
            }
            Err(e) => app.set_status(format!("Error: {}", e)),
        }
    }
}
