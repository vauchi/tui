// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! TUI-specific relay sync service.
//!
//! Provides async WebSocket sync and relay connection testing.
//! All core logic (message processing, exchange, device sync) delegates to vauchi-core.
//! This module only handles the WebSocket transport layer.

use std::time::Duration;

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use vauchi_core::exchange::EncryptedExchangeMessage;
use vauchi_core::network::simple_message::{
    create_device_sync_ack, create_signed_handshake, create_simple_ack, create_simple_envelope,
    decode_simple_message, encode_simple_message, LegacyExchangeMessage, SimpleAckStatus,
    SimpleDeviceSyncMessage, SimpleEncryptedUpdate, SimplePayload,
};
use vauchi_core::network::{classify_message, MessageType};
use vauchi_core::sync::{process_card_updates, DeviceSyncOrchestrator, SyncItem};
use vauchi_core::{
    contact_card::ContactCard, crypto::ratchet::DoubleRatchetState, exchange::X3DHKeyPair,
    storage::Storage, Contact, Identity,
};

/// Type alias for the async WebSocket stream.
type WsStream =
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
    fn success(contacts_added: u32, cards_updated: u32, updates_sent: u32) -> Self {
        Self {
            contacts_added,
            cards_updated,
            updates_sent,
            success: true,
            error: None,
        }
    }

    fn error(msg: impl Into<String>) -> Self {
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
struct ReceivedMessages {
    legacy_exchange: Vec<LegacyExchangeMessage>,
    encrypted_exchange: Vec<Vec<u8>>,
    card_updates: Vec<(String, Vec<u8>)>,
    device_sync_messages: Vec<SimpleDeviceSyncMessage>,
}

/// Performs a full sync with the relay server.
///
/// Connects via WebSocket, receives pending messages, processes them
/// using core functions, and sends outbound updates.
pub fn sync(identity: &Identity, storage: &Storage, relay_url: &str) -> SyncResult {
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => return SyncResult::error(format!("Runtime error: {}", e)),
    };
    rt.block_on(sync_async(identity, storage, relay_url))
}

/// Data needed to run sync in a background thread.
///
/// All fields are owned and `Send`, allowing the sync operation to run
/// on a separate thread without borrowing from the main-thread `App`.
/// The background thread reconstructs `Identity` and opens a fresh
/// `Storage` connection (rusqlite `Connection` is not `Send`).
pub struct SyncRequest {
    /// Serialized identity bytes (from `Identity::to_storage_bytes()`).
    pub identity_bytes: Vec<u8>,
    /// Path to the SQLite database file.
    pub storage_path: std::path::PathBuf,
    /// Storage encryption key (cloned from VauchiConfig).
    pub storage_key: vauchi_core::crypto::SymmetricKey,
    /// Relay WebSocket URL.
    pub relay_url: String,
}

/// Performs a full sync in a self-contained way using owned, `Send` data.
///
/// Opens a fresh `Storage` connection and reconstructs `Identity` from
/// serialized bytes, so this can safely run on a background thread.
pub fn sync_owned(req: SyncRequest) -> SyncResult {
    let identity = match Identity::from_storage_bytes(&req.identity_bytes) {
        Ok(id) => id,
        Err(e) => return SyncResult::error(format!("Identity restore failed: {}", e)),
    };
    let storage = match Storage::open(&req.storage_path, req.storage_key) {
        Ok(s) => s,
        Err(e) => return SyncResult::error(format!("Storage open failed: {}", e)),
    };
    sync(&identity, &storage, &req.relay_url)
}

/// Tests the relay connection by opening and closing a WebSocket.
pub fn test_relay_connection(relay_url: &str) -> Result<bool> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| anyhow::anyhow!("Runtime error: {}", e))?;

    rt.block_on(async {
        let mut socket = connect_to_relay(relay_url)
            .await
            .map_err(|e| anyhow::anyhow!("Connection failed: {}", e))?;
        let _ = socket.close(None).await;
        Ok(true)
    })
}

/// Tests the relay connection on a background thread using owned data.
pub fn test_relay_connection_owned(relay_url: String) -> Result<bool> {
    test_relay_connection(&relay_url)
}

// ============================================================
// Async internals
// ============================================================

async fn sync_async(identity: &Identity, storage: &Storage, relay_url: &str) -> SyncResult {
    let device_id_hex = hex::encode(identity.device_id());

    // Connect to relay
    let mut socket = match connect_to_relay(relay_url).await {
        Ok(s) => s,
        Err(e) => return SyncResult::error(format!("Connection failed: {}", e)),
    };

    // Send authenticated handshake
    if let Err(e) = send_handshake(&mut socket, identity, Some(&device_id_hex)).await {
        return SyncResult::error(format!("Handshake failed: {}", e));
    }

    // Wait for server to send pending messages
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Receive pending messages
    let received = match receive_pending(&mut socket).await {
        Ok(msgs) => msgs,
        Err(e) => return SyncResult::error(format!("Receive failed: {}", e)),
    };

    // Process legacy exchange messages
    let legacy_added = match process_legacy_exchanges(
        identity,
        storage,
        relay_url,
        received.legacy_exchange,
    )
    .await
    {
        Ok(count) => count,
        Err(e) => return SyncResult::error(format!("Legacy exchange failed: {}", e)),
    };

    // Process encrypted exchange messages
    let encrypted_added = match process_encrypted_exchanges(
        identity,
        storage,
        relay_url,
        received.encrypted_exchange,
    )
    .await
    {
        Ok(count) => count,
        Err(e) => return SyncResult::error(format!("Encrypted exchange failed: {}", e)),
    };

    let contacts_added = legacy_added + encrypted_added;

    // Process card updates (core's secure pipeline)
    let cards_updated = match process_card_updates(identity, storage, received.card_updates) {
        Ok(result) => result.processed,
        Err(e) => return SyncResult::error(format!("Card update failed: {}", e)),
    };

    // Process device sync messages
    let device_synced =
        match process_device_sync_messages(identity, storage, received.device_sync_messages) {
            Ok(count) => count,
            Err(e) => return SyncResult::error(format!("Device sync failed: {}", e)),
        };

    // Build device sync envelopes then send
    let device_envelopes =
        vauchi_core::sync::build_device_sync_envelopes(identity, storage).unwrap_or_default();

    let mut device_sync_sent = 0u32;
    for data in device_envelopes {
        if socket.send(Message::Binary(data)).await.is_ok() {
            device_sync_sent += 1;
        }
    }

    // Collect pending updates then send
    let pending = collect_pending_updates_data(identity, storage);

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
        let _ = storage.delete_pending_update(id);
    }

    // Close connection
    let _ = socket.close(None).await;

    SyncResult::success(
        contacts_added,
        cards_updated + device_synced,
        updates_sent + device_sync_sent,
    )
}

async fn connect_to_relay(relay_url: &str) -> Result<WsStream, String> {
    let (ws_stream, _) = tokio::time::timeout(
        Duration::from_secs(5),
        tokio_tungstenite::connect_async(relay_url),
    )
    .await
    .map_err(|_| "Connection timed out".to_string())?
    .map_err(|e| format!("WebSocket connection failed: {}", e))?;

    Ok(ws_stream)
}

async fn send_handshake(
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

async fn receive_pending(socket: &mut WsStream) -> Result<ReceivedMessages, String> {
    let mut legacy_exchange = Vec::new();
    let mut encrypted_exchange = Vec::new();
    let mut card_updates = Vec::new();
    let mut device_sync_messages = Vec::new();

    loop {
        let msg = match tokio::time::timeout(Duration::from_secs(1), socket.next()).await {
            Ok(Some(Ok(msg))) => msg,
            Ok(Some(Err(_))) | Ok(None) => break,
            Err(_) => break,
        };

        match msg {
            Message::Binary(data) => {
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
                                } else if EncryptedExchangeMessage::from_bytes(&update.ciphertext)
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
                    _ => {}
                }
            }
            Message::Ping(data) => {
                let _ = socket.send(Message::Pong(data)).await;
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    Ok(ReceivedMessages {
        legacy_exchange,
        encrypted_exchange,
        card_updates,
        device_sync_messages,
    })
}

// ============================================================
// Message processing (delegates to core)
// ============================================================

fn parse_hex_key(hex_str: &str) -> Option<[u8; 32]> {
    let bytes = hex::decode(hex_str).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Some(arr)
}

async fn process_legacy_exchanges(
    identity: &Identity,
    storage: &Storage,
    relay_url: &str,
    messages: Vec<LegacyExchangeMessage>,
) -> Result<u32, String> {
    let mut added = 0u32;
    let our_x3dh = identity.x3dh_keypair();

    for exchange in messages {
        let identity_key = match parse_hex_key(&exchange.identity_public_key) {
            Some(key) => key,
            None => continue,
        };

        let public_id = hex::encode(identity_key);

        if exchange.is_response {
            if let Ok(Some(mut contact)) = storage.load_contact(&public_id) {
                if contact.display_name() != exchange.display_name
                    && contact.set_display_name(&exchange.display_name).is_ok()
                {
                    let _ = storage.save_contact(&contact);
                }
            }
            continue;
        }

        if storage
            .load_contact(&public_id)
            .map_err(|e| e.to_string())?
            .is_some()
        {
            continue;
        }

        let ephemeral_key = match parse_hex_key(&exchange.ephemeral_public_key) {
            Some(key) => key,
            None => continue,
        };

        let shared_secret =
            match vauchi_core::exchange::X3DH::respond(&our_x3dh, &identity_key, &ephemeral_key) {
                Ok(secret) => secret,
                Err(_) => continue,
            };

        let card = ContactCard::new(&exchange.display_name);
        let contact = Contact::from_exchange(identity_key, card, shared_secret.clone());
        let contact_id = contact.id().to_string();
        storage.save_contact(&contact).map_err(|e| e.to_string())?;

        let secret = our_x3dh.secret_bytes();
        let ratchet_dh = X3DHKeyPair::from_bytes(*secret);
        let ratchet = DoubleRatchetState::initialize_responder(&shared_secret, ratchet_dh);
        let _ = storage.save_ratchet_state(&contact_id, &ratchet, true);

        added += 1;

        let _ = send_exchange_response(identity, relay_url, &public_id, &ephemeral_key).await;
    }

    Ok(added)
}

async fn process_encrypted_exchanges(
    identity: &Identity,
    storage: &Storage,
    relay_url: &str,
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

        if storage
            .load_contact(&public_id)
            .map_err(|e| e.to_string())?
            .is_some()
        {
            continue;
        }

        let card = ContactCard::new(&payload.display_name);
        let contact = Contact::from_exchange(payload.identity_key, card, shared_secret.clone());
        let contact_id = contact.id().to_string();
        storage.save_contact(&contact).map_err(|e| e.to_string())?;

        let secret = our_x3dh.secret_bytes();
        let ratchet_dh = X3DHKeyPair::from_bytes(*secret);
        let ratchet = DoubleRatchetState::initialize_responder(&shared_secret, ratchet_dh);
        let _ = storage.save_ratchet_state(&contact_id, &ratchet, false);

        added += 1;

        let _ =
            send_exchange_response(identity, relay_url, &public_id, &payload.exchange_key).await;
    }

    Ok(added)
}

async fn send_exchange_response(
    identity: &Identity,
    relay_url: &str,
    recipient_id: &str,
    recipient_exchange_key: &[u8; 32],
) -> Result<(), String> {
    let mut socket = connect_to_relay(relay_url).await?;
    send_handshake(&mut socket, identity, None).await?;

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
        ciphertext: encrypted_msg
            .to_bytes()
            .map_err(|e| format!("Serialization failed: {:?}", e))?,
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

fn process_device_sync_messages(
    identity: &Identity,
    storage: &Storage,
    messages: Vec<SimpleDeviceSyncMessage>,
) -> Result<u32, String> {
    if messages.is_empty() {
        return Ok(0);
    }

    let registry = match storage.load_device_registry() {
        Ok(Some(r)) if r.device_count() > 1 => r,
        _ => return Ok(0),
    };

    let mut orchestrator =
        DeviceSyncOrchestrator::new(storage, identity.create_device_info(), registry.clone());

    let mut processed = 0u32;

    for msg in messages {
        let sender_device_id: [u8; 32] = match hex::decode(&msg.sender_device_id) {
            Ok(bytes) if bytes.len() == 32 => {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                arr
            }
            _ => continue,
        };

        let sender_device = match registry.find_device(&sender_device_id) {
            Some(d) => d,
            None => continue,
        };

        let plaintext = match orchestrator
            .decrypt_from_device(&sender_device.exchange_public_key, &msg.encrypted_payload)
        {
            Ok(pt) => pt,
            Err(_) => continue,
        };

        let items: Vec<SyncItem> = match serde_json::from_slice(&plaintext) {
            Ok(items) => items,
            Err(_) => continue,
        };

        let applied = match orchestrator.process_incoming(items) {
            Ok(applied) => applied,
            Err(_) => continue,
        };

        for item in &applied {
            let _ = apply_sync_item(storage, item);
        }

        if !applied.is_empty() {
            processed += 1;
        }
    }

    Ok(processed)
}

fn apply_sync_item(storage: &Storage, item: &SyncItem) -> Result<(), String> {
    match item {
        SyncItem::ContactAdded { contact_data, .. } => {
            if let Ok(contact) = contact_data.to_contact() {
                storage.save_contact(&contact).map_err(|e| e.to_string())?;
            }
        }
        SyncItem::ContactRemoved { contact_id, .. } => {
            storage
                .delete_contact(contact_id)
                .map_err(|e| e.to_string())?;
        }
        SyncItem::CardUpdated {
            field_label,
            new_value,
            ..
        } => {
            if let Ok(Some(mut card)) = storage.load_own_card() {
                if card.update_field_value(field_label, new_value).is_ok() {
                    storage.save_own_card(&card).map_err(|e| e.to_string())?;
                }
            }
        }
        SyncItem::VisibilityChanged {
            contact_id,
            field_label,
            is_visible,
            ..
        } => {
            if let Some(mut contact) = storage
                .load_contact(contact_id)
                .map_err(|e| e.to_string())?
            {
                if *is_visible {
                    contact.visibility_rules_mut().set_everyone(field_label);
                } else {
                    contact.visibility_rules_mut().set_nobody(field_label);
                }
                storage.save_contact(&contact).map_err(|e| e.to_string())?;
            }
        }
        SyncItem::LabelChange { .. }
        | SyncItem::ContactTrustChanged { .. }
        | SyncItem::DeletionScheduled { .. }
        | SyncItem::DeletionCancelled { .. } => {}
    }
    Ok(())
}

fn collect_pending_updates_data(identity: &Identity, storage: &Storage) -> Vec<(String, Vec<u8>)> {
    let contacts = match storage.list_contacts() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let our_id = identity.public_id();
    let mut result = Vec::new();

    for contact in contacts {
        let pending = match storage.get_pending_updates(contact.id()) {
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
