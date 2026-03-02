// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! GDPR data export, account deletion, and emergency shred operations.

use anyhow::{Context, Result};

#[cfg(feature = "secure-storage")]
use vauchi_core::storage::secure::{PlatformKeyring, SecureStorage};

#[cfg(not(feature = "secure-storage"))]
use vauchi_core::storage::secure::{FileKeyStorage, SecureStorage};

#[cfg(not(feature = "secure-storage"))]
use super::load_or_generate_fallback_key;
use super::Backend;

impl Backend {
    /// Exports all user data for GDPR compliance.
    pub fn export_gdpr_data(&self) -> Result<String> {
        let export = vauchi_core::api::export_all_data(&self.storage)?;
        let json =
            serde_json::to_string_pretty(&export).context("Failed to serialize GDPR export")?;
        Ok(json)
    }

    /// Gets the current deletion state as a display string.
    pub fn get_deletion_status(&self) -> Result<String> {
        let manager = vauchi_core::api::DeletionManager::new(&self.storage);
        let state = manager.deletion_state()?;
        match state {
            vauchi_core::storage::DeletionState::None => Ok("No deletion scheduled".to_string()),
            vauchi_core::storage::DeletionState::Scheduled { execute_at, .. } => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let remaining = execute_at.saturating_sub(now);
                let days = remaining / 86400;
                Ok(format!("Deletion scheduled — {} days remaining", days))
            }
            vauchi_core::storage::DeletionState::Executed { .. } => {
                Ok("Account deleted".to_string())
            }
        }
    }

    /// Schedules account deletion with grace period.
    pub fn schedule_deletion(&self) -> Result<()> {
        let manager = vauchi_core::api::DeletionManager::new(&self.storage);
        manager.schedule_deletion()?;
        Ok(())
    }

    /// Cancels a scheduled deletion.
    pub fn cancel_deletion(&self) -> Result<()> {
        let manager = vauchi_core::api::DeletionManager::new(&self.storage);
        manager.cancel_deletion()?;
        Ok(())
    }

    /// Gets the structured deletion state.
    pub fn deletion_state(&self) -> Result<vauchi_core::storage::DeletionState> {
        let manager = vauchi_core::api::DeletionManager::new(&self.storage);
        let state = manager.deletion_state()?;
        Ok(state)
    }

    /// Creates a SecureStorage instance for shred operations.
    #[allow(unused_variables)]
    fn create_secure_storage(&self) -> Result<Box<dyn SecureStorage>> {
        #[cfg(feature = "secure-storage")]
        {
            Ok(Box::new(PlatformKeyring::new("vauchi-tui")))
        }

        #[cfg(not(feature = "secure-storage"))]
        {
            let fallback_key = load_or_generate_fallback_key(&self.data_dir)?;
            let key_dir = self.data_dir.join("keys");
            Ok(Box::new(FileKeyStorage::new(key_dir, fallback_key)))
        }
    }

    /// Creates a connected RelayClient for shred operations.
    fn create_shred_relay_client(
        &self,
        identity_id: &str,
    ) -> Result<vauchi_core::network::RelayClient<vauchi_core::network::WebSocketTransport>> {
        use vauchi_core::network::{
            RelayClient, RelayClientConfig, TransportConfig, WebSocketTransport,
        };
        let transport_config = TransportConfig {
            server_url: self.relay_url.clone(),
            ..TransportConfig::default()
        };
        let config = RelayClientConfig {
            transport: transport_config,
            ..RelayClientConfig::default()
        };
        let transport = WebSocketTransport::new();
        let mut client = RelayClient::new(transport, config, identity_id.to_string());
        client
            .connect()
            .map_err(|e| anyhow::anyhow!("Failed to connect to relay: {}", e))?;
        Ok(client)
    }

    /// Executes a scheduled account deletion after the grace period.
    pub fn execute_deletion(&self) -> Result<String> {
        let identity = self.identity.as_ref().context("No identity loaded")?;

        let manager = vauchi_core::api::DeletionManager::new(&self.storage);
        let state = manager.deletion_state()?;
        let token = match state {
            vauchi_core::storage::DeletionState::Scheduled {
                scheduled_at,
                execute_at,
            } => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                if now < execute_at {
                    let remaining = execute_at.saturating_sub(now);
                    let days = remaining / 86400;
                    anyhow::bail!("Grace period not elapsed. {} days remaining.", days);
                }
                vauchi_core::api::ShredToken::from_created_at(scheduled_at)
            }
            vauchi_core::storage::DeletionState::None => {
                anyhow::bail!("No deletion scheduled.");
            }
            vauchi_core::storage::DeletionState::Executed { .. } => {
                anyhow::bail!("Account already deleted.");
            }
        };

        let secure_storage = self.create_secure_storage()?;
        let identity_id = hex::encode(identity.signing_public_key());
        let shred_manager = vauchi_core::api::ShredManager::new(
            &self.storage,
            secure_storage.as_ref(),
            identity,
            &self.data_dir,
        );

        let mut purge_client = self.create_shred_relay_client(&identity_id)?;
        let mut revocation_client = self.create_shred_relay_client(&identity_id)?;

        let report = shred_manager
            .hard_shred(token, Some(&mut purge_client), Some(&mut revocation_client))
            .map_err(|e| anyhow::anyhow!("Shred failed: {}", e))?;

        let verification = shred_manager.verify_shred();
        Ok(super::format_shred_summary(&report, &verification))
    }

    /// Emergency immediate deletion — no grace period.
    pub fn panic_shred(&self) -> Result<String> {
        let identity = self.identity.as_ref().context("No identity loaded")?;

        let secure_storage = self.create_secure_storage()?;
        let identity_id = hex::encode(identity.signing_public_key());
        let shred_manager = vauchi_core::api::ShredManager::new(
            &self.storage,
            secure_storage.as_ref(),
            identity,
            &self.data_dir,
        );

        // Best-effort relay connections
        let mut purge_client = self.create_shred_relay_client(&identity_id).ok();
        let mut revocation_client = self.create_shred_relay_client(&identity_id).ok();

        let report = shred_manager
            .panic_shred(
                purge_client
                    .as_mut()
                    .map(|c| c as &mut dyn vauchi_core::api::PurgeSender),
                revocation_client
                    .as_mut()
                    .map(|c| c as &mut dyn vauchi_core::api::RevocationSender),
            )
            .map_err(|e| anyhow::anyhow!("Panic shred failed: {}", e))?;

        let verification = shred_manager.verify_shred();
        Ok(super::format_shred_summary(&report, &verification))
    }
}
