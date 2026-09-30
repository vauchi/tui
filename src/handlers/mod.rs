// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Generic Core command/event adapters.

mod presentation;

pub use presentation::handle_presentation_key;

// INLINE_TEST_REQUIRED: extracted white-box tests drive the key handler
// against the App's crate-private alert and presentation state.
#[cfg(test)]
mod presentation_tests;

pub enum Action {
    Continue,
    Quit,
}
