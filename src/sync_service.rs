// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! TUI-specific relay sync service.
//!
//! Provides sync via core's OHTTP HTTP API and relay connection testing.
//! All logic (message processing, exchange, card updates, ratchets, timing)
//! is handled by `Vauchi::connect()` + `sync()`.

use vauchi_core::api::VauchiSyncOutcome;
use vauchi_core::{Vauchi, VauchiConfig};

/// Result of a sync operation.
#[derive(Debug, Clone)]
pub struct SyncResult {
    /// Number of received (decrypted + applied) updates.
    pub cards_updated: u32,
    /// Number of outbound updates sent.
    pub updates_sent: u32,
    /// Number of acknowledged messages.
    pub acknowledged: u32,
    /// Whether sync completed successfully.
    pub success: bool,
    /// Error message if sync failed.
    pub error: Option<String>,
}

impl SyncResult {
    fn success(cards_updated: u32, updates_sent: u32, acknowledged: u32) -> Self {
        Self {
            cards_updated,
            updates_sent,
            acknowledged,
            success: true,
            error: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            cards_updated: 0,
            updates_sent: 0,
            acknowledged: 0,
            success: false,
            error: Some(msg.into()),
        }
    }

    /// Sync was skipped (e.g. C1/C2 timing) — not a failure.
    fn skipped(msg: impl Into<String>) -> Self {
        Self {
            cards_updated: 0,
            updates_sent: 0,
            acknowledged: 0,
            success: true,
            error: Some(msg.into()),
        }
    }
}

/// Performs a full sync with the relay server.
///
/// Delegates to `Vauchi::connect()` + `sync()` for bidirectional sync
/// over OHTTP-encrypted HTTP.
pub fn sync(vauchi: &mut Vauchi) -> SyncResult {
    if let Err(e) = vauchi.connect() {
        return SyncResult::error(format!("Connection failed: {e}"));
    }

    let outcome = match vauchi.sync() {
        Ok(o) => o,
        Err(e) => return SyncResult::error(format!("Sync failed: {e}")),
    };

    vauchi.disconnect();

    match outcome {
        VauchiSyncOutcome::Ok {
            received,
            sent,
            acknowledged,
            errors,
            ..
        } => {
            let mut result = SyncResult::success(received as u32, sent as u32, acknowledged as u32);
            if !errors.is_empty() {
                result.error = Some(errors.join("; "));
            }
            result
        }
        VauchiSyncOutcome::TooSoon => SyncResult::skipped("Too soon since last sync"),
        VauchiSyncOutcome::NotConnected => SyncResult::error("Not connected to relay"),
        VauchiSyncOutcome::NoIdentity => SyncResult::error("No identity found"),
    }
}

/// Data needed to run sync in a background thread.
///
/// All fields are owned and `Send`, allowing the sync operation to run
/// on a separate thread without borrowing from the main-thread `App`.
pub struct SyncRequest {
    /// Path to the SQLite database file.
    pub storage_path: std::path::PathBuf,
    /// Storage encryption key (cloned from VauchiConfig).
    pub storage_key: vauchi_core::crypto::SymmetricKey,
    /// Relay URL.
    pub relay_url: String,
}

/// Performs a full sync in a self-contained way using owned, `Send` data.
///
/// Creates a fresh `Vauchi` instance on the background thread.
pub fn sync_owned(req: SyncRequest) -> SyncResult {
    let config = VauchiConfig::with_storage_path(req.storage_path)
        .with_relay_url(&req.relay_url)
        .with_storage_key(req.storage_key);
    let mut vauchi = match Vauchi::new(config) {
        Ok(v) => v,
        Err(e) => return SyncResult::error(format!("Vauchi init failed: {}", e)),
    };
    // Identity is loaded from storage automatically by Vauchi::new
    sync(&mut vauchi)
}

/// Tests the relay connection by attempting an HTTP health check.
pub fn test_relay_connection(relay_url: &str) -> anyhow::Result<bool> {
    use vauchi_core::network::{HttpTransport, HttpTransportConfig, ProxyConfig};

    let transport = HttpTransport::new(HttpTransportConfig {
        relay_url: relay_url.to_string(),
        timeout_ms: 5000,
        proxy: ProxyConfig::None,
        allow_direct: true,
    });
    transport
        .health_check()
        .map_err(|e| anyhow::anyhow!("Connection failed: {e}"))?;
    Ok(true)
}

/// Tests the relay connection on a background thread using owned data.
pub fn test_relay_connection_owned(relay_url: String) -> anyhow::Result<bool> {
    test_relay_connection(&relay_url)
}
