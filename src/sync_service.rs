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
    LegacyExchangeMessage, SimpleAckStatus, SimpleEncryptedUpdate, SimplePayload,
    create_signed_handshake, create_simple_ack, create_simple_envelope, decode_simple_message,
    encode_simple_message,
};
use vauchi_core::network::{MessageType, classify_message};
use vauchi_core::storage::Storage;
use vauchi_core::sync::process_card_updates;
use vauchi_core::{Identity, Vauchi, VauchiConfig};

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
}

/// Performs a full sync with the relay server.
///
/// Connects via WebSocket, receives pending messages, processes them
/// using core functions, and sends outbound updates.
pub fn sync(identity: &Identity, vauchi: &Vauchi, relay_url: &str) -> SyncResult {
    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => return SyncResult::error(format!("Runtime error: {}", e)),
    };
    rt.block_on(sync_async(identity, vauchi, relay_url))
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
/// Creates a fresh `Vauchi` instance on the background thread. Identity
/// is loaded from storage automatically.
pub fn sync_owned(req: SyncRequest) -> SyncResult {
    let config = VauchiConfig {
        storage_path: req.storage_path,
        storage_key: Some(req.storage_key),
        ..VauchiConfig::default()
    };
    let vauchi = match Vauchi::new(config) {
        Ok(v) => v,
        Err(e) => return SyncResult::error(format!("Vauchi init failed: {}", e)),
    };
    // Identity is loaded from storage — reconstruct from bytes for handshake
    let identity = match Identity::from_storage_bytes(&req.identity_bytes) {
        Ok(id) => id,
        Err(e) => return SyncResult::error(format!("Identity restore failed: {}", e)),
    };
    sync(&identity, &vauchi, &req.relay_url)
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

async fn sync_async(identity: &Identity, vauchi: &Vauchi, relay_url: &str) -> SyncResult {
    let storage = vauchi.storage();
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

    // Process legacy exchange messages (ADR-021: crypto in core)
    let legacy_added =
        match process_legacy_exchanges(identity, vauchi, relay_url, received.legacy_exchange).await
        {
            Ok(count) => count,
            Err(e) => return SyncResult::error(format!("Legacy exchange failed: {}", e)),
        };

    // Process encrypted exchange messages (ADR-021: crypto in core)
    let encrypted_added =
        match process_encrypted_exchanges(identity, vauchi, relay_url, received.encrypted_exchange)
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
        cards_updated,
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

    loop {
        let msg = match tokio::time::timeout(Duration::from_secs(1), socket.next()).await {
            Ok(Some(Ok(msg))) => msg,
            Ok(Some(Err(_))) | Ok(None) => break,
            Err(_) => break,
        };

        match msg {
            Message::Binary(data) => {
                let msg_type = classify_message(&data);
                if msg_type == MessageType::EncryptedUpdate
                    && let Ok(envelope) = decode_simple_message(&data)
                    && let SimplePayload::EncryptedUpdate(update) = envelope.payload
                {
                    if LegacyExchangeMessage::is_exchange(&update.ciphertext) {
                        if let Some(exchange) =
                            LegacyExchangeMessage::from_bytes(&update.ciphertext)
                        {
                            legacy_exchange.push(exchange);
                        }
                    } else if EncryptedExchangeMessage::from_bytes(&update.ciphertext).is_ok() {
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
    vauchi: &Vauchi,
    relay_url: &str,
    messages: Vec<LegacyExchangeMessage>,
) -> Result<u32, String> {
    let mut added = 0u32;

    for exchange in messages {
        let identity_key = match parse_hex_key(&exchange.identity_public_key) {
            Some(key) => key,
            None => continue,
        };

        let public_id = hex::encode(identity_key);

        // Response messages update display name of existing contact
        if exchange.is_response {
            if let Ok(Some(mut contact)) = vauchi.get_contact(&public_id)
                && contact.display_name() != exchange.display_name
                && contact.set_display_name(&exchange.display_name).is_ok()
            {
                let _ = vauchi.update_contact(&contact);
            }
            continue;
        }

        let ephemeral_key = match parse_hex_key(&exchange.ephemeral_public_key) {
            Some(key) => key,
            None => continue,
        };

        // ADR-021: all crypto (X3DH, ratchet init) delegated to core
        match vauchi.accept_relay_exchange(&identity_key, &ephemeral_key, &exchange.display_name) {
            Ok(_contact_id) => added += 1,
            Err(_) => continue,
        }

        // Send encrypted response via relay
        let _ =
            send_exchange_response(identity, vauchi, relay_url, &public_id, &ephemeral_key).await;
    }

    Ok(added)
}

async fn process_encrypted_exchanges(
    identity: &Identity,
    vauchi: &Vauchi,
    relay_url: &str,
    encrypted_data: Vec<Vec<u8>>,
) -> Result<u32, String> {
    let mut added = 0u32;

    for data in encrypted_data {
        // ADR-021: all crypto (decrypt, X3DH, ratchet init) delegated to core
        let (contact_id, exchange_key) = match vauchi.accept_encrypted_relay_exchange(&data) {
            Ok(result) => result,
            Err(_) => continue,
        };

        added += 1;

        let _ =
            send_exchange_response(identity, vauchi, relay_url, &contact_id, &exchange_key).await;
    }

    Ok(added)
}

async fn send_exchange_response(
    identity: &Identity,
    vauchi: &Vauchi,
    relay_url: &str,
    recipient_id: &str,
    recipient_exchange_key: &[u8; 32],
) -> Result<(), String> {
    let mut socket = connect_to_relay(relay_url).await?;
    send_handshake(&mut socket, identity, None).await?;

    // ADR-021: exchange message creation delegated to core
    let ciphertext = vauchi
        .create_encrypted_exchange_response(recipient_exchange_key)
        .map_err(|e| format!("Failed to create exchange response: {:?}", e))?;

    let update = SimpleEncryptedUpdate {
        recipient_id: recipient_id.to_string(),
        sender_id: identity.public_id(),
        ciphertext,
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
