// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Feature screen handlers: exchange, settings, sync, lock.
//!
//! Duress is fully engine-driven (core `DuressPinEngine`); the TUI renders
//! its `ScreenModel` and forwards input — no per-screen handler here.
//! Privacy is likewise engine-driven (core `GdprEngine`) since G3 slice 2.

mod exchange;
mod lock;
mod settings;

pub(super) use exchange::handle_exchange_keys;
pub(super) use lock::handle_lock_keys;
pub(super) use settings::handle_settings_keys;
