// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Support screen key handler.

use crossterm::event::KeyCode;

use crate::app::App;

pub(in crate::handlers::input) fn handle_support_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('1') => {
            if open::that("https://github.com/sponsors/vauchi").is_err() {
                app.set_status("Could not open browser");
            }
        }
        KeyCode::Char('2') => {
            if open::that("https://liberapay.com/Vauchi/donate").is_err() {
                app.set_status("Could not open browser");
            }
        }
        _ => {}
    }
}
