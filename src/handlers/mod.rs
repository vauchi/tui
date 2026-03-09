// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Input Handlers

pub mod action_result;
mod input;

pub use input::{handle_key, Action};
