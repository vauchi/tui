// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Feature screen handlers: exchange, settings, devices, recovery, delivery,
//! sync, privacy, support, backup, emergency, duress, lock.

mod backup;
mod delivery;
mod devices;
pub(super) mod duress;
pub(super) mod emergency;
mod exchange;
mod lock;
mod privacy;
mod recovery;
mod settings;
mod support;
mod sync;

pub(super) use backup::handle_backup_keys;
pub(super) use delivery::handle_delivery_keys;
pub(super) use devices::handle_devices_keys;
pub(super) use duress::handle_duress_keys;
pub(super) use emergency::handle_emergency_keys;
pub(super) use exchange::handle_exchange_keys;
pub(super) use lock::handle_lock_keys;
pub(super) use privacy::handle_privacy_keys;
pub(super) use recovery::handle_recovery_keys;
pub(super) use settings::handle_settings_keys;
pub(super) use support::handle_support_keys;
pub(super) use sync::handle_sync_keys;
