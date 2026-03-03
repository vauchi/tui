// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Contact-related backend methods.

use std::sync::Arc;

use anyhow::{Context, Result};

use vauchi_core::api::{ContactManager, EventDispatcher};
use vauchi_core::contact_card::ContactAction;

use super::{Backend, ContactInfo};

/// Fingerprint information for manual verification.
#[derive(Debug, Clone)]
pub struct FingerprintInfo {
    /// The contact's formatted fingerprint (groups of 4 uppercase hex chars).
    pub their_fingerprint: String,
    /// Our own formatted fingerprint (groups of 4 uppercase hex chars).
    pub our_fingerprint: String,
    /// Whether this contact's fingerprint is already verified.
    pub is_verified: bool,
}

/// Field visibility information for display.
#[derive(Debug, Clone)]
pub struct FieldVisibilityInfo {
    pub field_label: String,
    pub can_see: bool,
}

/// Contact field information for display.
#[derive(Debug, Clone)]
pub struct ContactFieldInfo {
    pub label: String,
    pub value: String,
    #[allow(dead_code)]
    pub field_type: String,
    pub action_type: String,
    pub uri: Option<String>,
}

impl Backend {
    /// List all contacts.
    pub fn list_contacts(&self) -> Result<Vec<ContactInfo>> {
        let contacts = self
            .storage
            .list_contacts()
            .context("Failed to list contacts")?;

        Ok(contacts
            .into_iter()
            .map(|c| ContactInfo {
                id: c.id().to_string(),
                display_name: c.display_name().to_string(),
                verified: c.is_fingerprint_verified(),
                recovery_trusted: c.is_recovery_trusted(),
            })
            .collect())
    }

    /// Fuzzy-find contacts by display name or ID prefix.
    ///
    /// Delegates to core's ContactManager::find_contact_fuzzy, which combines
    /// case-insensitive name substring matching with ID prefix matching.
    pub fn find_contact_fuzzy(&self, query: &str) -> Result<Vec<ContactInfo>> {
        let events = Arc::new(EventDispatcher::new());
        let manager = ContactManager::new(&self.storage, events);
        let contacts = manager
            .find_contact_fuzzy(query)
            .context("Failed to fuzzy-find contacts")?;
        Ok(contacts
            .into_iter()
            .map(|c| ContactInfo {
                id: c.id().to_string(),
                display_name: c.display_name().to_string(),
                verified: c.is_fingerprint_verified(),
                recovery_trusted: c.is_recovery_trusted(),
            })
            .collect())
    }

    /// Get contact count.
    pub fn contact_count(&self) -> Result<usize> {
        let contacts = self
            .storage
            .list_contacts()
            .context("Failed to list contacts")?;
        Ok(contacts.len())
    }

    /// Get a contact by index.
    pub fn get_contact_by_index(&self, index: usize) -> Result<Option<ContactInfo>> {
        let contacts = self.list_contacts()?;
        Ok(contacts.get(index).cloned())
    }

    /// Get visibility info for a contact (what fields they can see).
    pub fn get_contact_visibility(&self, contact_id: &str) -> Result<Vec<FieldVisibilityInfo>> {
        let contact = self
            .storage
            .load_contact(contact_id)
            .context("Failed to get contact")?
            .context("Contact not found")?;

        let card = self.get_card()?.unwrap_or_else(|| {
            vauchi_core::ContactCard::new(self.display_name().unwrap_or("User"))
        });

        let rules = contact.visibility_rules();

        Ok(card
            .fields()
            .iter()
            .map(|field| {
                let can_see = rules.can_see(field.label(), contact_id);
                FieldVisibilityInfo {
                    field_label: field.label().to_string(),
                    can_see,
                }
            })
            .collect())
    }

    /// Toggle visibility of a field for a contact.
    pub fn toggle_field_visibility(&self, contact_id: &str, field_label: &str) -> Result<bool> {
        let mut contact = self
            .storage
            .load_contact(contact_id)
            .context("Failed to get contact")?
            .context("Contact not found")?;

        let current_can_see = contact.visibility_rules().can_see(field_label, contact_id);

        // Toggle: if currently visible, set to nobody; if hidden, set to everyone
        if current_can_see {
            contact.visibility_rules_mut().set_nobody(field_label);
        } else {
            contact.visibility_rules_mut().set_everyone(field_label);
        }

        let new_can_see = !current_can_see;

        self.storage
            .save_contact(&contact)
            .context("Failed to save contact")?;

        Ok(new_can_see)
    }

    /// Remove a contact by ID.
    pub fn remove_contact(&self, contact_id: &str) -> Result<()> {
        self.storage
            .delete_contact(contact_id)
            .context("Failed to delete contact")?;
        Ok(())
    }

    /// Get fields for a contact by index.
    pub fn get_contact_fields(&self, contact_index: usize) -> Result<Vec<ContactFieldInfo>> {
        let contacts = self
            .storage
            .list_contacts()
            .context("Failed to list contacts")?;

        let contact = contacts.get(contact_index).context("Contact not found")?;

        Ok(contact
            .card()
            .fields()
            .iter()
            .map(|f| {
                let action = f.to_action();
                let action_type = match &action {
                    ContactAction::Call(_) => "call",
                    ContactAction::SendSms(_) => "sms",
                    ContactAction::SendEmail(_) => "email",
                    ContactAction::OpenUrl(_) => "web",
                    ContactAction::OpenMap(_) => "map",
                    ContactAction::GetDirections(_) => "directions",
                    ContactAction::CopyToClipboard => "copy",
                };
                ContactFieldInfo {
                    label: f.label().to_string(),
                    value: f.value().to_string(),
                    field_type: format!("{:?}", f.field_type()),
                    action_type: action_type.to_string(),
                    uri: f.to_uri(),
                }
            })
            .collect())
    }

    /// Open a contact field in the system default app.
    ///
    /// If opening fails, automatically copies the field value to the clipboard as a fallback.
    pub fn open_contact_field(&self, contact_index: usize, field_index: usize) -> Result<String> {
        let fields = self.get_contact_fields(contact_index)?;
        let field = fields.get(field_index).context("Field not found")?;

        if let Some(ref uri) = field.uri {
            match open::that(uri) {
                Ok(_) => Ok(format!("Opened {} in {}", field.label, field.action_type)),
                Err(e) => {
                    // Fallback: copy value to clipboard
                    match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(&field.value)) {
                        Ok(_) => Ok(format!(
                            "Could not open {} ({}). Copied to clipboard.",
                            field.label, e
                        )),
                        Err(_) => Ok(format!(
                            "Could not open {} ({}). Value: {}",
                            field.label, e, field.value
                        )),
                    }
                }
            }
        } else {
            Ok(format!("No action available for {}", field.label))
        }
    }

    /// Get secondary actions for a contact field.
    pub fn get_secondary_actions(
        &self,
        contact_index: usize,
        field_index: usize,
    ) -> Result<Vec<(String, ContactAction)>> {
        let contacts = self
            .storage
            .list_contacts()
            .context("Failed to list contacts")?;
        let contact = contacts.get(contact_index).context("Contact not found")?;
        let field = contact
            .card()
            .fields()
            .get(field_index)
            .context("Field not found")?;

        Ok(field
            .to_secondary_actions()
            .into_iter()
            .map(|a| {
                let label = match &a {
                    ContactAction::Call(v) => format!("Call {}", v),
                    ContactAction::SendSms(v) => format!("Send SMS to {}", v),
                    ContactAction::SendEmail(v) => format!("Email {}", v),
                    ContactAction::OpenUrl(_) => "Open in Browser".to_string(),
                    ContactAction::OpenMap(_) => "Open in Maps".to_string(),
                    ContactAction::GetDirections(_) => "Get Directions".to_string(),
                    ContactAction::CopyToClipboard => "Copy to Clipboard".to_string(),
                };
                (label, a)
            })
            .collect())
    }

    /// Attempts to copy a value to the clipboard as a fallback when opening fails.
    fn clipboard_fallback(value: &str, action_name: &str, error: std::io::Error) -> Result<String> {
        match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(value)) {
            Ok(_) => Ok(format!(
                "Could not open {} ({}). Copied to clipboard.",
                action_name, error
            )),
            Err(_) => Ok(format!(
                "Could not open {} ({}). Value: {}",
                action_name, error, value
            )),
        }
    }

    /// Execute a contact action (open URI or copy to clipboard).
    ///
    /// If opening fails, automatically copies the relevant value to the clipboard as a fallback.
    pub fn execute_action(&self, action: &ContactAction) -> Result<String> {
        match action {
            ContactAction::Call(v) => match open::that(format!("tel:{}", v)) {
                Ok(_) => Ok("Opened dialer".to_string()),
                Err(e) => Self::clipboard_fallback(v, "dialer", e),
            },
            ContactAction::SendSms(v) => match open::that(format!("sms:{}", v)) {
                Ok(_) => Ok("Opened messaging".to_string()),
                Err(e) => Self::clipboard_fallback(v, "messaging", e),
            },
            ContactAction::SendEmail(v) => match open::that(format!("mailto:{}", v)) {
                Ok(_) => Ok("Opened email client".to_string()),
                Err(e) => Self::clipboard_fallback(v, "email client", e),
            },
            ContactAction::OpenUrl(v) => match open::that(v) {
                Ok(_) => Ok("Opened browser".to_string()),
                Err(e) => Self::clipboard_fallback(v, "browser", e),
            },
            ContactAction::OpenMap(v) => {
                let encoded: String = v
                    .chars()
                    .map(|c| {
                        if c.is_ascii_alphanumeric() || "-._~,+/".contains(c) {
                            c.to_string()
                        } else if c == ' ' {
                            "%20".to_string()
                        } else {
                            format!("%{:02X}", c as u32)
                        }
                    })
                    .collect();
                match open::that(format!(
                    "https://www.openstreetmap.org/search?query={}",
                    encoded
                )) {
                    Ok(_) => Ok("Opened maps".to_string()),
                    Err(e) => Self::clipboard_fallback(v, "maps", e),
                }
            }
            ContactAction::GetDirections(v) => {
                let encoded: String = v
                    .chars()
                    .map(|c| {
                        if c.is_ascii_alphanumeric() || "-._~,+/".contains(c) {
                            c.to_string()
                        } else if c == ' ' {
                            "%20".to_string()
                        } else {
                            format!("%{:02X}", c as u32)
                        }
                    })
                    .collect();
                match open::that(format!(
                    "https://www.openstreetmap.org/directions?route=&to={}",
                    encoded
                )) {
                    Ok(_) => Ok("Opened directions".to_string()),
                    Err(e) => Self::clipboard_fallback(v, "directions", e),
                }
            }
            ContactAction::CopyToClipboard => {
                anyhow::bail!("CopyToClipboard should be handled by the caller with field value")
            }
        }
    }

    /// Copy a field value to the system clipboard.
    pub fn copy_field_to_clipboard(
        &self,
        contact_index: usize,
        field_index: usize,
    ) -> Result<String> {
        let fields = self.get_contact_fields(contact_index)?;
        let field = fields.get(field_index).context("Field not found")?;

        let mut clipboard =
            arboard::Clipboard::new().context("Failed to access system clipboard")?;
        clipboard
            .set_text(&field.value)
            .context("Failed to copy to clipboard")?;

        Ok(format!("Copied {} to clipboard", field.label))
    }

    // ========== Field Validation ==========

    /// Get the validation status for a contact's field.
    ///
    /// Returns aggregated validation information including count, trust level,
    /// and whether the current user has validated this field.
    pub fn get_field_validation_status(
        &self,
        contact_id: &str,
        field_id: &str,
        field_value: &str,
    ) -> Result<vauchi_core::ValidationStatus> {
        let validations = self
            .storage
            .load_validations_for_field(contact_id, field_id)
            .context("Failed to load validations")?;

        let my_id = self
            .identity
            .as_ref()
            .map(|id| hex::encode(id.signing_public_key()));

        let contacts = self
            .storage
            .list_contacts()
            .context("Failed to list contacts")?;

        // Exclude blocked contacts' validations
        let blocked: std::collections::HashSet<String> = contacts
            .iter()
            .filter(|c| c.is_blocked())
            .map(|c| c.id().to_string())
            .collect();

        // Build validator metadata for trust weighting
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before UNIX epoch")
            .as_secs();

        let validator_meta: std::collections::HashMap<String, vauchi_core::ValidatorMeta> =
            contacts
                .iter()
                .map(|c| {
                    let age_days = (now.saturating_sub(c.exchange_timestamp())) / 86400;
                    (
                        c.id().to_string(),
                        vauchi_core::ValidatorMeta {
                            contact_age_days: age_days,
                            fingerprint_verified: c.is_fingerprint_verified(),
                        },
                    )
                })
                .collect();

        let status = vauchi_core::ValidationStatus::from_validations_weighted(
            &validations,
            field_value,
            my_id.as_deref(),
            &blocked,
            &validator_meta,
        );

        Ok(status)
    }

    /// Validate a contact's field.
    ///
    /// Creates a signed validation record and stores it locally.
    /// Also queues the validation for delivery to the contact via sync.
    pub fn validate_field(
        &self,
        contact_id: &str,
        field_id: &str,
        field_value: &str,
    ) -> Result<vauchi_core::ProfileValidation> {
        let identity = self.identity.as_ref().context("No identity")?;

        // Check we're not validating our own field
        let my_id = hex::encode(identity.signing_public_key());
        if contact_id == my_id {
            anyhow::bail!("Cannot validate your own field");
        }

        // Check we haven't already validated this field
        let validator_id = hex::encode(identity.signing_public_key());
        if self
            .storage
            .has_validated(contact_id, field_id, &validator_id)
            .context("Failed to check validation status")?
        {
            anyhow::bail!("You have already validated this field");
        }

        // Create signed validation
        let validation = vauchi_core::ProfileValidation::create_signed(
            identity,
            field_id,
            field_value,
            contact_id,
        );

        // Store it
        self.storage
            .save_validation(&validation)
            .context("Failed to save validation")?;

        // Queue for delivery to the validated contact
        if let Ok(validation_bytes) = serde_json::to_vec(&validation) {
            let sync_manager = vauchi_core::SyncManager::new(&self.storage);
            let _ = sync_manager.queue_validation_delivery(contact_id, validation_bytes);
        }

        Ok(validation)
    }

    /// Revoke the current user's validation of a field.
    ///
    /// Returns true if a validation was revoked, false if none existed.
    pub fn revoke_field_validation(&self, contact_id: &str, field_id: &str) -> Result<bool> {
        let identity = self.identity.as_ref().context("No identity")?;

        let validator_id = hex::encode(identity.signing_public_key());
        let deleted = self
            .storage
            .delete_validation(contact_id, field_id, &validator_id)
            .context("Failed to delete validation")?;

        // Queue revocation for delivery
        if deleted {
            let revocation_info = serde_json::json!({
                "contact_id": contact_id,
                "field_id": field_id,
                "validator_id": validator_id,
            });
            if let Ok(revocation_bytes) = serde_json::to_vec(&revocation_info) {
                let sync_manager = vauchi_core::SyncManager::new(&self.storage);
                let _ = sync_manager.queue_validation_revocation(contact_id, revocation_bytes);
            }
        }

        Ok(deleted)
    }

    /// Get fingerprint information for a contact (for manual verification).
    ///
    /// Returns both the contact's and our own fingerprint as formatted strings
    /// (groups of 4 uppercase hex chars) using the core `Contact::fingerprint()` API.
    pub fn get_contact_fingerprint(&self, contact_id: &str) -> Result<FingerprintInfo> {
        let identity = self.identity.as_ref().context("No identity")?;

        let contact = self
            .storage
            .load_contact(contact_id)
            .context("Failed to load contact")?
            .context("Contact not found")?;

        let their_fingerprint = contact.fingerprint();

        // Format our own fingerprint the same way as Contact::fingerprint()
        let our_hex = hex::encode(identity.signing_public_key());
        let our_fingerprint = our_hex
            .chars()
            .collect::<Vec<_>>()
            .chunks(4)
            .map(|c| c.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join(" ")
            .to_uppercase();

        Ok(FingerprintInfo {
            their_fingerprint,
            our_fingerprint,
            is_verified: contact.is_fingerprint_verified(),
        })
    }

    /// Hide a contact from the default contact list.
    pub fn hide_contact(&self, contact_id: &str) -> Result<()> {
        let mut contact = self
            .storage
            .load_contact(contact_id)
            .context("Failed to load contact")?
            .context("Contact not found")?;

        contact.hide();

        self.storage
            .save_contact(&contact)
            .context("Failed to save contact")?;

        Ok(())
    }

    /// Unhide a previously hidden contact.
    pub fn unhide_contact(&self, contact_id: &str) -> Result<()> {
        let mut contact = self
            .storage
            .load_contact(contact_id)
            .context("Failed to load contact")?
            .context("Contact not found")?;

        contact.unhide();

        self.storage
            .save_contact(&contact)
            .context("Failed to save contact")?;

        Ok(())
    }

    /// List hidden contacts.
    pub fn list_hidden_contacts(&self) -> Result<Vec<ContactInfo>> {
        let contacts = self
            .storage
            .list_contacts()
            .context("Failed to list contacts")?;

        Ok(contacts
            .into_iter()
            .filter(|c| c.is_hidden())
            .map(|c| ContactInfo {
                id: c.id().to_string(),
                display_name: c.display_name().to_string(),
                verified: c.is_fingerprint_verified(),
                recovery_trusted: c.is_recovery_trusted(),
            })
            .collect())
    }

    /// Check if a contact is hidden.
    pub fn is_contact_hidden(&self, contact_id: &str) -> Result<bool> {
        let contact = self
            .storage
            .load_contact(contact_id)
            .context("Failed to load contact")?
            .context("Contact not found")?;

        Ok(contact.is_hidden())
    }

    /// Mark a contact's fingerprint as verified.
    pub fn verify_contact_fingerprint(&self, contact_id: &str) -> Result<()> {
        let mut contact = self
            .storage
            .load_contact(contact_id)
            .context("Failed to load contact")?
            .context("Contact not found")?;

        contact.mark_fingerprint_verified();

        self.storage
            .save_contact(&contact)
            .context("Failed to save contact")?;

        Ok(())
    }
}
