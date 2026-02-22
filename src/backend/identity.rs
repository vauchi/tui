// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Identity-related backend methods.

use anyhow::{Context, Result};

use vauchi_core::{Identity, IdentityBackup};

use super::Backend;

impl Backend {
    /// Check if identity exists.
    pub fn has_identity(&self) -> bool {
        self.identity.is_some() || self.backup_data.is_some()
    }

    /// Create a new identity.
    #[allow(dead_code)]
    pub fn create_identity(&mut self, name: &str) -> Result<()> {
        let password = self.backup_password()?;
        let identity = Identity::create(name);
        let backup = identity
            .export_backup(&password)
            .map_err(|e| anyhow::anyhow!("Failed to create backup: {:?}", e))?;
        let backup_data = backup.as_bytes().to_vec();

        self.storage
            .save_identity(&backup_data, name)
            .context("Failed to save identity")?;

        self.identity = Some(identity);
        self.backup_data = Some(backup_data);
        self.display_name = Some(name.to_string());
        Ok(())
    }

    /// Get the display name.
    pub fn display_name(&self) -> Option<&str> {
        self.identity
            .as_ref()
            .map(|i| i.display_name())
            .or(self.display_name.as_deref())
    }

    /// Get the public ID (truncated).
    pub fn public_id(&self) -> Option<String> {
        self.identity.as_ref().map(|i| {
            let full = i.public_id();
            format!("{}...", &full[..16.min(full.len())])
        })
    }

    /// Update the display name.
    pub fn update_display_name(&mut self, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            anyhow::bail!("Display name cannot be empty");
        }
        if name.len() > 100 {
            anyhow::bail!("Display name cannot exceed 100 characters");
        }

        // Update identity
        let password = self.backup_password()?;
        if let Some(ref mut identity) = self.identity {
            identity.set_display_name(name);

            // Re-export backup with updated identity
            let backup = identity
                .export_backup(&password)
                .map_err(|e| anyhow::anyhow!("Failed to create backup: {:?}", e))?;
            let backup_data = backup.as_bytes().to_vec();
            self.storage.save_identity(&backup_data, name)?;
            self.backup_data = Some(backup_data);
        }

        // Update card display name
        let mut card = self
            .get_card()?
            .unwrap_or_else(|| vauchi_core::ContactCard::new(name));
        card.set_display_name(name)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        self.storage.save_own_card(&card)?;

        self.display_name = Some(name.to_string());
        Ok(())
    }

    /// Export identity backup with password.
    pub fn export_backup(&self, password: &str) -> Result<String> {
        let identity = self.identity.as_ref().context("No identity")?;
        let backup = identity
            .export_backup(password)
            .map_err(|e| anyhow::anyhow!("Export failed: {:?}", e))?;
        Ok(hex::encode(backup.as_bytes()))
    }

    /// Import identity from backup with password.
    pub fn import_backup(&mut self, backup_data: &str, password: &str) -> Result<()> {
        let bytes = hex::decode(backup_data.trim()).context("Invalid hex data")?;
        let backup = IdentityBackup::new(bytes.clone());
        let identity = Identity::import_backup(&backup, password)
            .map_err(|e| anyhow::anyhow!("Import failed: {:?}", e))?;

        let name = identity.display_name().to_string();
        self.storage
            .save_identity(&bytes, &name)
            .context("Failed to save identity")?;

        self.identity = Some(identity);
        self.backup_data = Some(bytes);
        self.display_name = Some(name);
        Ok(())
    }
}
