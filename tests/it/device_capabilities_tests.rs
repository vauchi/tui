// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use vauchi_core::types::AudioCapability;
use vauchi_tui::app::tui_device_capabilities;

// @scenario: exchange :: Camera-less devices are offered Link first
#[test]
fn a_terminal_reports_itself_as_network_only() {
    let caps = tui_device_capabilities();
    assert!(!caps.has_camera);
    assert!(!caps.has_ble);
    assert!(!caps.has_nfc);
    assert!(!caps.has_accelerometer);
    assert!(matches!(caps.audio, AudioCapability::None));
    assert!(caps.has_internet);
}
