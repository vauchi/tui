// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Sync-related backend methods.

use std::time::Duration;

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use vauchi_core::{
    network::{
        classify_message,
        simple_message::{
            create_device_sync_ack, create_signed_handshake, create_simple_ack,
            create_simple_envelope, decode_simple_message, encode_simple_message,
            LegacyExchangeMessage, SimpleAckStatus, SimpleDeviceSyncMessage, SimpleEncryptedUpdate,
            SimplePayload,
        },
        MessageType,
    },
    sync::{process_card_updates, DeviceSyncOrchestrator, SyncItem},
    Identity,
};

use vauchi_core::exchange::EncryptedExchangeMessage;

use super::Backend;

/// Type alias for the async WebSocket stream.
pub(crate) type WsStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// Result of a sync operation.
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Number of new contacts added from exchange messages.
    pub contacts_added: u32,
    /// Number of contact cards updated.
    pub cards_updated: u32,
    /// Number of outbound updates sent.
    pub updates_sent: u32,
    /// Whether sync completed successfully.
    pub success: bool,
    /// Error message if sync failed.
    pub error: Option<String>,
}

impl SyncResult {
    /// Create a success result.
    pub fn success(contacts_added: u32, cards_updated: u32, updates_sent: u32) -> Self {
        Self {
            contacts_added,
            cards_updated,
            updates_sent,
            success: true,
            error: None,
        }
    }

    /// Create an error result.
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            contacts_added: 0,
            cards_updated: 0,
            updates_sent: 0,
            success: false,
            error: Some(msg.into()),
        }
    }
}

/// Messages received from the relay during sync.
pub(crate) struct ReceivedMessages {
    pub legacy_exchange: Vec<LegacyExchangeMessage>,
    pub encrypted_exchange: Vec<Vec<u8>>,
    pub card_updates: Vec<(String, Vec<u8>)>,
    pub device_sync_messages: Vec<SimpleDeviceSyncMessage>,
}

impl Backend {
    /// Get sync status string for display.
    #[allow(dead_code)]
    pub fn sync_status(&self) -> &'static str {
        if self.identity.is_some() {
            "Ready to sync"
        } else {
            "No identity"
        }
    }

    /// Get count of pending outbound updates.
    pub fn pending_update_count(&self) -> Result<u32> {
        let contacts = self
            .storage
            .list_contacts()
            .context("Failed to list contacts")?;

        let mut total = 0u32;
        for contact in &contacts {
            let pending = self
                .storage
                .get_pending_updates(contact.id())
                .unwrap_or_default();
            total += pending.len() as u32;
        }
        Ok(total)
    }

    /// Perform a full sync with the relay server.
    ///
    /// This connects to the relay, receives pending messages, processes them,
    /// and sends any pending outbound updates. Uses a current-thread tokio
    /// runtime internally for async WebSocket I/O.
    pub fn sync(&self) -> SyncResult {
        let rt = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(e) => return SyncResult::error(format!("Runtime error: {}", e)),
        };
        rt.block_on(self.sync_async())
    }

    /// Async implementation of the full sync pipeline.
    async fn sync_async(&self) -> SyncResult {
        let identity = match &self.identity {
            Some(id) => id,
            None => return SyncResult::error("No identity found. Create an identity first."),
        };

        let relay_url = &self.relay_url;
        let device_id_hex = hex::encode(identity.device_id());

        // Connect to relay (async with timeout)
        let mut socket = match Self::connect_to_relay(relay_url).await {
            Ok(s) => s,
            Err(e) => return SyncResult::error(format!("Connection failed: {}", e)),
        };

        // Send authenticated handshake with device_id for inter-device sync
        if let Err(e) = Self::send_handshake(&mut socket, identity, Some(&device_id_hex)).await {
            return SyncResult::error(format!("Handshake failed: {}", e));
        }

        // Wait for server to send pending messages
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Receive pending messages (async with per-message timeout)
        let received = match Self::receive_pending(&mut socket).await {
            Ok(msgs) => msgs,
            Err(e) => return SyncResult::error(format!("Receive failed: {}", e)),
        };

        // Process legacy exchange messages
        let legacy_added = match self
            .process_legacy_exchanges(identity, received.legacy_exchange)
            .await
        {
            Ok(count) => count,
            Err(e) => return SyncResult::error(format!("Legacy exchange failed: {}", e)),
        };

        // Process encrypted exchange messages
        let encrypted_added = match self
            .process_encrypted_exchanges(identity, received.encrypted_exchange)
            .await
        {
            Ok(count) => count,
            Err(e) => return SyncResult::error(format!("Encrypted exchange failed: {}", e)),
        };

        let contacts_added = legacy_added + encrypted_added;

        // Process card updates (sync — core's secure pipeline)
        let cards_updated =
            match process_card_updates(identity, &self.storage, received.card_updates) {
                Ok(result) => result.processed,
                Err(e) => return SyncResult::error(format!("Card update failed: {}", e)),
            };

        // Process device sync messages (sync)
        let device_synced =
            match self.process_device_sync_messages(identity, received.device_sync_messages) {
                Ok(count) => count,
                Err(e) => return SyncResult::error(format!("Device sync failed: {}", e)),
            };

        // Build device sync envelopes (sync) then send (async)
        let device_envelopes =
            vauchi_core::sync::build_device_sync_envelopes(identity, &self.storage)
                .unwrap_or_default();

        let mut device_sync_sent = 0u32;
        for data in device_envelopes {
            if socket.send(Message::Binary(data)).await.is_ok() {
                device_sync_sent += 1;
            }
        }

        // Collect pending updates (sync) then send (async)
        let pending = self.collect_pending_updates_data(identity);

        let mut updates_sent = 0u32;
        let mut sent_ids = Vec::new();
        for (update_id, data) in pending {
            if socket.send(Message::Binary(data)).await.is_ok() {
                sent_ids.push(update_id);
                updates_sent += 1;
            }
        }

        // Cleanup sent updates
        for id in &sent_ids {
            let _ = self.storage.delete_pending_update(id);
        }

        // Close connection
        let _ = socket.close(None).await;

        SyncResult::success(
            contacts_added,
            cards_updated + device_synced,
            updates_sent + device_sync_sent,
        )
    }

    /// Connect to relay server via async WebSocket with timeout.
    pub(crate) async fn connect_to_relay(relay_url: &str) -> Result<WsStream, String> {
        let (ws_stream, _) = tokio::time::timeout(
            Duration::from_secs(5),
            tokio_tungstenite::connect_async(relay_url),
        )
        .await
        .map_err(|_| "Connection timed out".to_string())?
        .map_err(|e| format!("WebSocket connection failed: {}", e))?;

        Ok(ws_stream)
    }

    /// Send authenticated handshake to relay.
    pub(crate) async fn send_handshake(
        socket: &mut WsStream,
        identity: &Identity,
        device_id: Option<&str>,
    ) -> Result<(), String> {
        let handshake = create_signed_handshake(identity, device_id.map(|s| s.to_string()));
        let envelope = create_simple_envelope(SimplePayload::Handshake(handshake));
        let data = encode_simple_message(&envelope).map_err(|e| format!("Encode error: {}", e))?;
        socket
            .send(Message::Binary(data))
            .await
            .map_err(|e| format!("Send error: {}", e))?;
        Ok(())
    }

    /// Receive pending messages from relay with timeout.
    async fn receive_pending(socket: &mut WsStream) -> Result<ReceivedMessages, String> {
        let mut legacy_exchange = Vec::new();
        let mut encrypted_exchange = Vec::new();
        let mut card_updates = Vec::new();
        let mut device_sync_messages = Vec::new();

        loop {
            let msg = match tokio::time::timeout(Duration::from_secs(1), socket.next()).await {
                Ok(Some(Ok(msg))) => msg,
                Ok(Some(Err(_))) | Ok(None) => break,
                Err(_) => break, // Timeout — no more pending messages
            };

            match msg {
                Message::Binary(data) => {
                    // Use core classify_message() to route by type before full decode
                    let msg_type = classify_message(&data);
                    match msg_type {
                        MessageType::EncryptedUpdate => {
                            if let Ok(envelope) = decode_simple_message(&data) {
                                if let SimplePayload::EncryptedUpdate(update) = envelope.payload {
                                    if LegacyExchangeMessage::is_exchange(&update.ciphertext) {
                                        if let Some(exchange) =
                                            LegacyExchangeMessage::from_bytes(&update.ciphertext)
                                        {
                                            legacy_exchange.push(exchange);
                                        }
                                    } else if EncryptedExchangeMessage::from_bytes(
                                        &update.ciphertext,
                                    )
                                    .is_ok()
                                    {
                                        encrypted_exchange.push(update.ciphertext);
                                    } else {
                                        card_updates.push((update.sender_id, update.ciphertext));
                                    }

                                    let ack = create_simple_ack(
                                        &envelope.message_id,
                                        SimpleAckStatus::ReceivedByRecipient,
                                    );
                                    if let Ok(ack_data) = encode_simple_message(&ack) {
                                        let _ = socket.send(Message::Binary(ack_data)).await;
                                    }
                                }
                            }
                        }
                        MessageType::DeviceSync => {
                            if let Ok(envelope) = decode_simple_message(&data) {
                                if let SimplePayload::DeviceSyncMessage(msg) = envelope.payload {
                                    let version = msg.version;
                                    device_sync_messages.push(msg);

                                    let ack = create_device_sync_ack(&envelope.message_id, version);
                                    if let Ok(ack_data) = encode_simple_message(&ack) {
                                        let _ = socket.send(Message::Binary(ack_data)).await;
                                    }
                                }
                            }
                        }
                        _ => {} // Ack, Handshake, Unknown — not expected inbound
                    }
                }
                Message::Ping(data) => {
                    let _ = socket.send(Message::Pong(data)).await;
                }
                Message::Close(_) => break,
                _ => { /* Ignore other message types */ }
            }
        }

        Ok(ReceivedMessages {
            legacy_exchange,
            encrypted_exchange,
            card_updates,
            device_sync_messages,
        })
    }

    /// Process incoming device sync messages from other devices.
    fn process_device_sync_messages(
        &self,
        identity: &Identity,
        messages: Vec<SimpleDeviceSyncMessage>,
    ) -> Result<u32, String> {
        if messages.is_empty() {
            return Ok(0);
        }

        // Try to load device registry - if none exists, skip
        let registry = match self.storage.load_device_registry() {
            Ok(Some(r)) if r.device_count() > 1 => r,
            _ => return Ok(0),
        };

        let mut orchestrator = DeviceSyncOrchestrator::new(
            &self.storage,
            identity.create_device_info(),
            registry.clone(),
        );

        let mut processed = 0u32;

        for msg in messages {
            // Parse sender device ID
            let sender_device_id: [u8; 32] = match hex::decode(&msg.sender_device_id) {
                Ok(bytes) if bytes.len() == 32 => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    arr
                }
                _ => continue,
            };

            // Find sender in registry
            let sender_device = match registry.find_device(&sender_device_id) {
                Some(d) => d,
                None => continue,
            };

            // Decrypt payload
            let plaintext = match orchestrator
                .decrypt_from_device(&sender_device.exchange_public_key, &msg.encrypted_payload)
            {
                Ok(pt) => pt,
                Err(_) => continue,
            };

            // Parse SyncItems
            let items: Vec<SyncItem> = match serde_json::from_slice(&plaintext) {
                Ok(items) => items,
                Err(_) => continue,
            };

            // Process items with conflict resolution
            let applied = match orchestrator.process_incoming(items) {
                Ok(applied) => applied,
                Err(_) => continue,
            };

            // Apply the items
            for item in &applied {
                let _ = self.apply_sync_item(item);
            }

            if !applied.is_empty() {
                processed += 1;
            }
        }

        Ok(processed)
    }

    /// Apply a single sync item to local storage.
    fn apply_sync_item(&self, item: &SyncItem) -> Result<(), String> {
        match item {
            SyncItem::ContactAdded { contact_data, .. } => {
                if let Ok(contact) = contact_data.to_contact() {
                    self.storage
                        .save_contact(&contact)
                        .map_err(|e| e.to_string())?;
                }
            }
            SyncItem::ContactRemoved { contact_id, .. } => {
                self.storage
                    .delete_contact(contact_id)
                    .map_err(|e| e.to_string())?;
            }
            SyncItem::CardUpdated {
                field_label,
                new_value,
                ..
            } => {
                if let Ok(Some(mut card)) = self.storage.load_own_card() {
                    if card.update_field_value(field_label, new_value).is_ok() {
                        self.storage
                            .save_own_card(&card)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            SyncItem::VisibilityChanged {
                contact_id,
                field_label,
                is_visible,
                ..
            } => {
                if let Some(mut contact) = self
                    .storage
                    .load_contact(contact_id)
                    .map_err(|e| e.to_string())?
                {
                    if *is_visible {
                        contact.visibility_rules_mut().set_everyone(field_label);
                    } else {
                        contact.visibility_rules_mut().set_nobody(field_label);
                    }
                    self.storage
                        .save_contact(&contact)
                        .map_err(|e| e.to_string())?;
                }
            }
            SyncItem::LabelChange { .. }
            | SyncItem::ContactTrustChanged { .. }
            | SyncItem::DeletionScheduled { .. }
            | SyncItem::DeletionCancelled { .. } => {
                // Informational sync items — no local storage action needed
            }
        }
        Ok(())
    }

    /// Collect pending outbound updates as serialized data for async sending.
    ///
    /// Returns `(update_id, serialized_envelope)` pairs. The caller sends
    /// them over the async WebSocket and then deletes the IDs on success.
    fn collect_pending_updates_data(&self, identity: &Identity) -> Vec<(String, Vec<u8>)> {
        let contacts = match self.storage.list_contacts() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let our_id = identity.public_id();
        let mut result = Vec::new();

        for contact in contacts {
            let pending = match self.storage.get_pending_updates(contact.id()) {
                Ok(p) => p,
                Err(_) => continue,
            };

            for update in pending {
                let msg = SimpleEncryptedUpdate {
                    recipient_id: contact.id().to_string(),
                    sender_id: our_id.clone(),
                    ciphertext: update.payload,
                };

                let envelope = create_simple_envelope(SimplePayload::EncryptedUpdate(msg));
                if let Ok(data) = encode_simple_message(&envelope) {
                    result.push((update.id, data));
                }
            }
        }

        result
    }

    /// Test connection to the relay server.
    pub fn test_relay_connection(&self) -> Result<bool> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| anyhow::anyhow!("Runtime error: {}", e))?;

        rt.block_on(async {
            let mut socket = Self::connect_to_relay(&self.relay_url)
                .await
                .map_err(|e| anyhow::anyhow!("Connection failed: {}", e))?;

            let _ = socket.close(None).await;
            Ok(true)
        })
    }
}
