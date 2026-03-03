// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Emergency broadcast and duress mode management.

use anyhow::{Context, Result};

use super::Backend;

// ================================================================
// Emergency Broadcast
// ================================================================

impl Backend {
    /// Get the current emergency broadcast configuration.
    pub fn get_emergency_config(
        &self,
    ) -> Result<Option<vauchi_core::api::EmergencyBroadcastConfig>> {
        self.storage
            .load_emergency_config()
            .context("Failed to load emergency config")
    }

    /// Configure the emergency broadcast system.
    pub fn configure_emergency_broadcast(
        &self,
        contact_ids: Vec<String>,
        message: String,
        include_location: bool,
    ) -> Result<()> {
        let config = vauchi_core::api::EmergencyBroadcastConfig {
            trusted_contact_ids: contact_ids,
            message,
            include_location,
        };
        self.storage
            .save_emergency_config(&config)
            .context("Failed to save emergency config")?;
        Ok(())
    }

    /// Disable the emergency broadcast system.
    pub fn disable_emergency_broadcast(&self) -> Result<()> {
        self.storage
            .delete_emergency_config()
            .context("Failed to delete emergency config")?;
        Ok(())
    }
}

// ================================================================
// Authentication
// ================================================================

/// Result of PIN authentication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthResult {
    /// Normal password — show real contacts.
    Normal,
    /// Duress PIN — show decoy contacts.
    Duress,
    /// PIN didn't match either password.
    Invalid,
}

impl Backend {
    /// Authenticates with a PIN and returns the result.
    ///
    /// Uses `AppPasswordConfig::verify()` to check the PIN against stored
    /// password hash(es). Does not set internal auth mode — the caller
    /// (lock screen handler) decides what to do with the result.
    pub fn authenticate(&self, pin: &str) -> Result<AuthResult> {
        let config = self
            .storage
            .load_password_config()
            .context("Failed to load password config")?
            .ok_or_else(|| anyhow::anyhow!("No app password configured"))?;

        match config.verify(pin) {
            vauchi_core::AuthResult::Normal => Ok(AuthResult::Normal),
            vauchi_core::AuthResult::Duress => Ok(AuthResult::Duress),
            vauchi_core::AuthResult::Invalid => Ok(AuthResult::Invalid),
        }
    }
}

// ================================================================
// Duress Mode & Passwords
// ================================================================

impl Backend {
    /// Sets up an app password for PIN-based lock screen.
    pub fn setup_app_password(&self, password: &str) -> Result<()> {
        let config = vauchi_core::api::AppPasswordConfig::create(password)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        self.storage
            .save_app_password(config.password_hash(), config.password_salt())
            .context("Failed to save app password")?;
        Ok(())
    }

    /// Returns whether an app password is configured.
    pub fn is_password_enabled(&self) -> Result<bool> {
        Ok(self
            .storage
            .load_password_config()
            .context("Failed to load password config")?
            .is_some())
    }

    /// Returns whether duress mode is enabled.
    pub fn is_duress_enabled(&self) -> Result<bool> {
        match self
            .storage
            .load_password_config()
            .context("Failed to load password config")?
        {
            Some(config) => Ok(config.duress_enabled()),
            None => Ok(false),
        }
    }

    /// Sets up a duress PIN. Requires app password to be configured first.
    pub fn setup_duress_password(&self, duress_password: &str) -> Result<()> {
        let mut config = self
            .storage
            .load_password_config()
            .context("Failed to load password config")?
            .ok_or_else(|| anyhow::anyhow!("App password must be set before duress PIN"))?;

        config
            .setup_duress(duress_password)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        let duress_hash = config
            .duress_hash()
            .ok_or_else(|| anyhow::anyhow!("Duress hash not set after setup"))?;
        let duress_salt = config
            .duress_salt()
            .ok_or_else(|| anyhow::anyhow!("Duress salt not set after setup"))?;

        self.storage
            .save_duress_password(duress_hash, duress_salt)
            .context("Failed to save duress password")?;
        Ok(())
    }

    /// Disables duress mode and clears duress hash/salt.
    pub fn disable_duress(&self) -> Result<()> {
        self.storage
            .disable_duress()
            .context("Failed to disable duress")?;
        Ok(())
    }

    /// Loads duress alert settings.
    pub fn load_duress_settings(&self) -> Result<Option<vauchi_core::api::DuressSettings>> {
        self.storage
            .load_duress_settings()
            .context("Failed to load duress settings")
    }

    /// Saves duress alert settings.
    pub fn save_duress_settings(&self, settings: &vauchi_core::api::DuressSettings) -> Result<()> {
        self.storage
            .save_duress_settings(settings)
            .context("Failed to save duress settings")?;
        Ok(())
    }

    /// Deletes duress alert settings.
    pub fn delete_duress_settings(&self) -> Result<()> {
        self.storage
            .delete_duress_settings()
            .context("Failed to delete duress settings")?;
        Ok(())
    }
}
