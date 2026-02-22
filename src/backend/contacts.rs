// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Contact-related backend methods.

use std::sync::Arc;

use anyhow::{Context, Result};

use vauchi_core::api::{ContactManager, EventDispatcher};
use vauchi_core::contact_card::ContactAction;

use super::{Backend, ContactInfo};

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
    pub fn open_contact_field(&self, contact_index: usize, field_index: usize) -> Result<String> {
        let fields = self.get_contact_fields(contact_index)?;
        let field = fields.get(field_index).context("Field not found")?;

        if let Some(ref uri) = field.uri {
            open::that(uri).context("Failed to open URI")?;
            Ok(format!("Opened {} in {}", field.label, field.action_type))
        } else {
            Ok(format!("No action available for {}", field.label))
        }
    }
}
