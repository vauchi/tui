// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Devices screen key handler.

use crossterm::event::KeyCode;

use crate::app::App;

pub(in crate::handlers::input) fn handle_devices_keys(app: &mut App, key: KeyCode) {
    // Handle revoke confirmation overlay
    if app.revoke_confirm {
        match key {
            KeyCode::Char('y') => {
                app.revoke_confirm = false;
                match app.app_engine.vauchi().revoke_device(app.selected_device) {
                    Ok(name) => app.set_status(format!("Device '{}' revoked", name)),
                    Err(e) => app.set_status(format!("Revoke failed: {}", e)),
                }
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                app.revoke_confirm = false;
            }
            _ => {}
        }
        return;
    }

    // Handle QR overlay — Esc dismisses
    if app.device_link_result.is_some() {
        if key == KeyCode::Esc {
            app.device_link_result = None;
        }
        return;
    }

    match key {
        KeyCode::Char('j') | KeyCode::Down => {
            if let Ok(devices) = app.app_engine.vauchi().list_devices() {
                if app.selected_device < devices.len().saturating_sub(1) {
                    app.selected_device += 1;
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.selected_device > 0 {
                app.selected_device -= 1;
            }
        }
        KeyCode::Char('l') => match app.app_engine.vauchi().generate_device_link() {
            Ok(result) => {
                app.device_link_result = Some(result);
            }
            Err(e) => app.set_status(format!("Error: {}", e)),
        },
        KeyCode::Char('r') => {
            // Check if selected device is current or already revoked
            if let Ok(devices) = app.app_engine.vauchi().list_devices() {
                if let Some(device) = devices.get(app.selected_device) {
                    if device.is_current {
                        app.set_status("Cannot revoke the current device");
                    } else if !device.is_active {
                        app.set_status("Device is already revoked");
                    } else {
                        app.revoke_confirm = true;
                    }
                }
            }
        }
        _ => {}
    }
}
