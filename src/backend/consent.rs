// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Consent and privacy record management.

use anyhow::Result;

use super::Backend;

impl Backend {
    /// Gets consent status as a display string.
    pub fn get_consent_status(&self) -> Result<String> {
        let manager = vauchi_core::api::ConsentManager::new(&self.storage);
        let records = manager.export_consent_log()?;
        if records.is_empty() {
            return Ok("No consent records".to_string());
        }
        let lines: Vec<String> = records
            .iter()
            .map(|r| {
                format!(
                    "{:?}: {}",
                    r.consent_type,
                    if r.granted { "Granted" } else { "Revoked" }
                )
            })
            .collect();
        Ok(lines.join("\n"))
    }

    /// Gets all consent records.
    pub fn consent_records(&self) -> Result<Vec<vauchi_core::api::ConsentRecord>> {
        let manager = vauchi_core::api::ConsentManager::new(&self.storage);
        let records = manager.export_consent_log()?;
        Ok(records)
    }

    /// Gets the structured consent status for a specific consent type.
    ///
    /// Delegates to core's ConsentManager to determine grant status and
    /// retrieve the latest consent record metadata (timestamp, policy version).
    pub fn consent_status_for_type(
        &self,
        consent_type: &vauchi_core::api::ConsentType,
    ) -> Result<vauchi_core::api::ConsentStatus> {
        let manager = vauchi_core::api::ConsentManager::new(&self.storage);
        let granted = manager.check(consent_type)?;
        let records = manager.export_consent_log()?;
        let latest = records
            .iter()
            .rev()
            .find(|r| &r.consent_type == consent_type);
        Ok(vauchi_core::api::ConsentStatus {
            granted,
            last_changed_at: latest.map(|r| r.timestamp),
            policy_version: latest.and_then(|r| r.policy_version.clone()),
        })
    }

    /// Grants consent for a specific type.
    pub fn grant_consent(&self, consent_type: vauchi_core::api::ConsentType) -> Result<()> {
        let manager = vauchi_core::api::ConsentManager::new(&self.storage);
        manager.grant(consent_type)?;
        Ok(())
    }

    /// Revokes consent for a specific type.
    pub fn revoke_consent(&self, consent_type: vauchi_core::api::ConsentType) -> Result<()> {
        let manager = vauchi_core::api::ConsentManager::new(&self.storage);
        manager.revoke(consent_type)?;
        Ok(())
    }
}
