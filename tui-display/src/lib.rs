// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Terminal rendering and keyboard input for the Vauchi presentation contract.

mod session;
pub mod ui;

pub use session::{SessionView, TerminalSession};
