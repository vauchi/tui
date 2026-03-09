// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tor privacy mode configuration.

use anyhow::{Context, Result};

use super::Backend;

impl Backend {
    /// Load Tor configuration from storage.
    pub fn load_tor_config(&self) -> Result<vauchi_core::TorConfig> {
        let config = self
            .storage
            .load_or_create_tor_config()
            .context("Failed to load Tor config")?;
        Ok(config)
    }

    /// Enable Tor mode.
    pub fn enable_tor(&self) -> Result<()> {
        let mut config = self.load_tor_config()?;
        config.enabled = true;
        self.storage
            .save_tor_config(&config)
            .context("Failed to save Tor config")?;
        Ok(())
    }

    /// Disable Tor mode.
    pub fn disable_tor(&self) -> Result<()> {
        let mut config = self.load_tor_config()?;
        config.enabled = false;
        self.storage
            .save_tor_config(&config)
            .context("Failed to save Tor config")?;
        Ok(())
    }

    /// Toggle .onion address preference.
    pub fn toggle_prefer_onion(&self) -> Result<bool> {
        let mut config = self.load_tor_config()?;
        config.prefer_onion = !config.prefer_onion;
        self.storage
            .save_tor_config(&config)
            .context("Failed to save Tor config")?;
        Ok(config.prefer_onion)
    }

    /// Clear all bridge addresses.
    pub fn clear_tor_bridges(&self) -> Result<usize> {
        let mut config = self.load_tor_config()?;
        let count = config.bridges.len();
        config.bridges.clear();
        self.storage
            .save_tor_config(&config)
            .context("Failed to save Tor config")?;
        Ok(count)
    }
}
