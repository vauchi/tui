// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Recovery-related backend methods.

use anyhow::{Context, Result};

use super::Backend;

/// Recovery status information.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RecoveryStatus {
    pub has_active_claim: bool,
    pub voucher_count: u32,
    pub required_vouchers: u32,
    pub claim_expires: Option<String>,
}

impl Backend {
    /// Get recovery status.
    pub fn get_recovery_status(&self) -> Result<RecoveryStatus> {
        // For now, return a stub status
        Ok(RecoveryStatus {
            has_active_claim: false,
            voucher_count: 0,
            required_vouchers: 3,
            claim_expires: None,
        })
    }

    /// Toggle recovery trust for a contact. Returns new trust state.
    pub fn toggle_recovery_trust(&self, contact_id: &str) -> Result<bool> {
        let mut contact = self
            .storage
            .load_contact(contact_id)
            .context("Failed to get contact")?
            .context("Contact not found")?;

        if contact.is_blocked() {
            anyhow::bail!("Blocked contacts cannot be trusted for recovery");
        }

        let new_state = !contact.is_recovery_trusted();
        if new_state {
            contact.trust_for_recovery();
        } else {
            contact.untrust_for_recovery();
        }

        self.storage
            .save_contact(&contact)
            .context("Failed to save contact")?;

        Ok(new_state)
    }

    /// Count contacts that are trusted for recovery.
    pub fn trusted_contact_count(&self) -> Result<usize> {
        let contacts = self
            .storage
            .list_contacts()
            .context("Failed to list contacts")?;
        Ok(contacts.iter().filter(|c| c.is_recovery_trusted()).count())
    }
}
