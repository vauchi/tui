// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Exchange-related backend methods.

use std::time::Duration;

use anyhow::{Context, Result};
use futures_util::SinkExt;
use tokio_tungstenite::tungstenite::Message;

use vauchi_core::{
    crypto::ratchet::DoubleRatchetState,
    exchange::{
        EncryptedExchangeMessage, ExchangeEvent, ExchangeSession, ManualConfirmationVerifier,
        X3DHKeyPair,
    },
    network::simple_message::{
        create_simple_envelope, encode_simple_message, LegacyExchangeMessage,
        SimpleEncryptedUpdate, SimplePayload,
    },
    Contact, ContactCard, Identity,
};

use super::Backend;

/// QR code data with expiration info.
#[derive(Debug, Clone)]
pub struct QRData {
    /// The QR code data string.
    pub data: String,
    /// Unix timestamp when the QR was generated.
    pub generated_at: u64,
    /// QR expiration time in seconds.
    pub expires_in_secs: u64,
}

impl QRData {
    /// Calculate remaining seconds until expiration.
    pub fn remaining_secs(&self) -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        let expires_at = self.generated_at + self.expires_in_secs;
        expires_at.saturating_sub(now)
    }

    /// Check if the QR code has expired.
    #[allow(dead_code)]
    pub fn is_expired(&self) -> bool {
        self.remaining_secs() == 0
    }
}

impl Backend {
    /// Generate exchange QR data with expiration info.
    ///
    /// Uses ExchangeSession state machine with ManualConfirmationVerifier
    /// since TUI doesn't have audio hardware for proximity verification.
    pub fn generate_exchange_qr(&self) -> Result<QRData> {
        let identity = self.identity.as_ref().context("No identity")?;

        let our_card = self
            .storage
            .load_own_card()
            .ok()
            .flatten()
            .unwrap_or_else(|| ContactCard::new(identity.display_name()));

        // Reconstruct an owned identity for the session
        let backup_password = self.backup_password()?;
        let backup = identity
            .export_backup(&backup_password)
            .map_err(|e| anyhow::anyhow!("Failed to export identity: {:?}", e))?;
        let identity_owned = Identity::import_backup(&backup, &backup_password)
            .map_err(|e| anyhow::anyhow!("Failed to import identity: {:?}", e))?;

        // Create exchange session for mutual QR exchange
        let verifier = ManualConfirmationVerifier::new();
        let mut session = ExchangeSession::new_qr(identity_owned, our_card, verifier);

        // Generate QR via state machine
        session
            .apply(ExchangeEvent::StartQR)
            .map_err(|e| anyhow::anyhow!("Failed to generate QR: {:?}", e))?;

        let qr = session.qr().context("QR code not generated")?;

        Ok(QRData {
            data: qr.to_data_string(),
            generated_at: qr.timestamp(),
            expires_in_secs: 300, // 5 minutes, matching QR_EXPIRY_SECONDS in vauchi-core
        })
    }

    /// Parse a hex-encoded 32-byte key.
    pub(crate) fn parse_hex_key(hex_str: &str) -> Option<[u8; 32]> {
        let bytes = hex::decode(hex_str).ok()?;
        if bytes.len() != 32 {
            return None;
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Some(arr)
    }

    /// Process legacy plaintext exchange messages.
    pub(crate) async fn process_legacy_exchanges(
        &self,
        identity: &Identity,
        messages: Vec<LegacyExchangeMessage>,
    ) -> Result<u32, String> {
        let mut added = 0u32;
        let our_x3dh = identity.x3dh_keypair();

        for exchange in messages {
            let identity_key = match Self::parse_hex_key(&exchange.identity_public_key) {
                Some(key) => key,
                None => continue,
            };

            let public_id = hex::encode(identity_key);

            // Handle response (update contact name)
            if exchange.is_response {
                if let Ok(Some(mut contact)) = self.storage.load_contact(&public_id) {
                    if contact.display_name() != exchange.display_name
                        && contact.set_display_name(&exchange.display_name).is_ok()
                    {
                        let _ = self.storage.save_contact(&contact);
                    }
                }
                continue;
            }

            // Check if contact exists
            if self
                .storage
                .load_contact(&public_id)
                .map_err(|e| e.to_string())?
                .is_some()
            {
                continue;
            }

            let ephemeral_key = match Self::parse_hex_key(&exchange.ephemeral_public_key) {
                Some(key) => key,
                None => continue,
            };

            // Perform X3DH
            let shared_secret = match vauchi_core::exchange::X3DH::respond(
                &our_x3dh,
                &identity_key,
                &ephemeral_key,
            ) {
                Ok(secret) => secret,
                Err(_) => continue,
            };

            // Create contact
            let card = ContactCard::new(&exchange.display_name);
            let contact = Contact::from_exchange(identity_key, card, shared_secret.clone());
            let contact_id = contact.id().to_string();
            self.storage
                .save_contact(&contact)
                .map_err(|e| e.to_string())?;

            // Initialize ratchet
            let ratchet_dh = X3DHKeyPair::from_bytes(our_x3dh.secret_bytes());
            let ratchet = DoubleRatchetState::initialize_responder(&shared_secret, ratchet_dh);
            let _ = self.storage.save_ratchet_state(&contact_id, &ratchet, true);

            added += 1;

            // Send response
            let _ = self
                .send_exchange_response(identity, &public_id, &ephemeral_key)
                .await;
        }

        Ok(added)
    }

    /// Process encrypted exchange messages.
    pub(crate) async fn process_encrypted_exchanges(
        &self,
        identity: &Identity,
        encrypted_data: Vec<Vec<u8>>,
    ) -> Result<u32, String> {
        let mut added = 0u32;
        let our_x3dh = identity.x3dh_keypair();

        for data in encrypted_data {
            let encrypted_msg = match EncryptedExchangeMessage::from_bytes(&data) {
                Ok(msg) => msg,
                Err(_) => continue,
            };

            let (payload, shared_secret) = match encrypted_msg.decrypt(&our_x3dh) {
                Ok(result) => result,
                Err(_) => continue,
            };

            let public_id = hex::encode(payload.identity_key);

            // Check if contact exists
            if self
                .storage
                .load_contact(&public_id)
                .map_err(|e| e.to_string())?
                .is_some()
            {
                continue;
            }

            // Create contact
            let card = ContactCard::new(&payload.display_name);
            let contact = Contact::from_exchange(payload.identity_key, card, shared_secret.clone());
            let contact_id = contact.id().to_string();
            self.storage
                .save_contact(&contact)
                .map_err(|e| e.to_string())?;

            // Initialize ratchet
            let ratchet_dh = X3DHKeyPair::from_bytes(our_x3dh.secret_bytes());
            let ratchet = DoubleRatchetState::initialize_responder(&shared_secret, ratchet_dh);
            let _ = self
                .storage
                .save_ratchet_state(&contact_id, &ratchet, false);

            added += 1;

            // Send response
            let _ = self
                .send_exchange_response(identity, &public_id, &payload.exchange_key)
                .await;
        }

        Ok(added)
    }

    /// Send exchange response.
    pub(crate) async fn send_exchange_response(
        &self,
        identity: &Identity,
        recipient_id: &str,
        recipient_exchange_key: &[u8; 32],
    ) -> Result<(), String> {
        let mut socket = Self::connect_to_relay(&self.relay_url).await?;

        Self::send_handshake(&mut socket, identity, None).await?;

        let our_id = identity.public_id();
        let our_x3dh = identity.x3dh_keypair();
        let (encrypted_msg, _) = EncryptedExchangeMessage::create(
            &our_x3dh,
            recipient_exchange_key,
            identity.signing_public_key(),
            identity.display_name(),
        )
        .map_err(|e| format!("Failed to encrypt exchange: {:?}", e))?;

        let update = SimpleEncryptedUpdate {
            recipient_id: recipient_id.to_string(),
            sender_id: our_id,
            ciphertext: encrypted_msg.to_bytes(),
        };

        let envelope = create_simple_envelope(SimplePayload::EncryptedUpdate(update));
        let data = encode_simple_message(&envelope).map_err(|e| e.to_string())?;
        socket
            .send(Message::Binary(data))
            .await
            .map_err(|e| e.to_string())?;

        tokio::time::sleep(Duration::from_millis(100)).await;
        let _ = socket.close(None).await;

        Ok(())
    }
}
