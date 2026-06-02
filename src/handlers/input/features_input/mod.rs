// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Feature screen handlers: exchange, settings, devices, recovery, delivery,
//! sync, privacy, support, backup, emergency, lock.
//!
//! Duress is fully engine-driven (core `DuressPinEngine`); the TUI renders
//! its `ScreenModel` and forwards input — no per-screen handler here.

mod exchange;
mod lock;
mod privacy;
mod settings;
mod sync;

pub(super) use exchange::handle_exchange_keys;
pub(super) use lock::handle_lock_keys;
pub(super) use privacy::handle_privacy_keys;
pub(super) use settings::handle_settings_keys;
pub(super) use sync::handle_sync_keys;
