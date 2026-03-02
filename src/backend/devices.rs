// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Device management: listing, linking, and revocation.

use anyhow::{Context, Result};

use super::Backend;

/// Device information for display.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DeviceInfo {
    pub device_index: u32,
    pub device_name: String,
    pub public_key_prefix: String,
    pub is_current: bool,
    pub is_active: bool,
}

/// Result from generating a device link.
#[derive(Debug, Clone)]
pub struct DeviceLinkResult {
    /// ASCII art QR code for terminal display.
    pub qr_ascii: String,
    /// Data string (base64) for copy-paste transport.
    pub data_string: String,
    /// Identity fingerprint for verification.
    pub fingerprint: String,
}

impl Backend {
    /// List all linked devices.
    pub fn list_devices(&self) -> Result<Vec<DeviceInfo>> {
        let identity = self.identity.as_ref().context("No identity")?;
        let current_device_id = identity.device_id();

        // Try to load device registry from storage
        if let Ok(Some(registry)) = self.storage.load_device_registry() {
            Ok(registry
                .all_devices()
                .iter()
                .enumerate()
                .map(|(i, device)| DeviceInfo {
                    device_index: i as u32,
                    device_name: device.device_name.clone(),
                    public_key_prefix: hex::encode(&device.device_id[..8]),
                    is_current: device.device_id == *current_device_id,
                    is_active: !device.revoked,
                })
                .collect())
        } else {
            // Return current device only
            Ok(vec![DeviceInfo {
                device_index: 0,
                device_name: "This Device".to_string(),
                public_key_prefix: hex::encode(&current_device_id[..8]),
                is_current: true,
                is_active: true,
            }])
        }
    }

    /// Generate device link QR code and data string using the core API.
    ///
    /// Returns `(qr_ascii, data_string, fingerprint)` for display.
    pub fn generate_device_link(&self) -> Result<DeviceLinkResult> {
        let identity = self.identity.as_ref().context("No identity")?;

        let registry = self
            .storage
            .load_device_registry()?
            .unwrap_or_else(|| identity.initial_device_registry());

        let initiator = identity.create_device_link_initiator(registry);
        let qr = initiator.qr();

        Ok(DeviceLinkResult {
            qr_ascii: qr.to_qr_image_string(),
            data_string: qr.to_data_string(),
            fingerprint: qr.identity_fingerprint(),
        })
    }

    /// Revoke a device from the registry by its public key prefix.
    pub fn revoke_device(&self, device_index: usize) -> Result<String> {
        let identity = self.identity.as_ref().context("No identity")?;

        let mut registry = self
            .storage
            .load_device_registry()?
            .context("No device registry found")?;

        let devices = registry.all_devices().to_vec();
        if device_index >= devices.len() {
            anyhow::bail!("Invalid device index: {}", device_index);
        }

        let device = &devices[device_index];

        if device.device_id == *identity.device_id() {
            anyhow::bail!("Cannot revoke the current device");
        }

        if device.revoked {
            anyhow::bail!("Device '{}' is already revoked", device.device_name);
        }

        let device_name = device.device_name.clone();
        registry.revoke_device(&device.device_id, identity.signing_keypair())?;
        self.storage.save_device_registry(&registry)?;

        Ok(device_name)
    }
}
