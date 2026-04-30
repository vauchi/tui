// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Exchange screen key handler.

use crossterm::event::KeyCode;

use crate::app::App;

pub(in crate::handlers::input) fn handle_exchange_keys(app: &mut App, key: KeyCode) {
    if let KeyCode::Char('r') = key
        && app.generate_exchange_qr().is_ok()
    {
        app.set_status(app.i18n.t("exchange.refreshed"));
    }
}
