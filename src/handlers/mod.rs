// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Generic Core command/event adapters.

mod presentation;

pub use presentation::handle_presentation_key;

pub enum Action {
    Continue,
    Quit,
}
