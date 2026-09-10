// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! What a terminal can and cannot do for an in-person exchange. Core
//! keys the exchange picker on these: without a camera, Bluetooth or
//! NFC the picker leads with Link, whose share screen also accepts a
//! pasted peer link (core `link_exchange`).

use vauchi_core::exchange::capability::types::DeviceCapabilities;
use vauchi_core::types::AudioCapability;

/// A terminal session: network only.
pub fn tui_device_capabilities() -> DeviceCapabilities {
    DeviceCapabilities {
        has_camera: false,
        has_ble: false,
        has_nfc: false,
        audio: AudioCapability::None,
        has_accelerometer: false,
        has_internet: true,
        has_usb_port: false,
        ..Default::default()
    }
}
