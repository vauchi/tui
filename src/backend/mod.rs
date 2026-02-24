// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Backend wrapper for vauchi-core

mod contacts;
mod exchange;
mod identity;
mod recovery;
mod sync;

pub use contacts::{ContactFieldInfo, FieldVisibilityInfo};
pub use exchange::QRData;
pub use recovery::RecoveryStatus;
pub use sync::SyncResult;

use std::path::Path;

use anyhow::{Context, Result};

use vauchi_core::aha_moments::{AhaMoment, AhaMomentTracker, AhaMomentType};
#[cfg(feature = "secure-storage")]
use vauchi_core::storage::secure::{PlatformKeyring, SecureStorage};
use vauchi_core::{
    ContactCard, ContactField, FieldType, Identity, IdentityBackup, Storage, SymmetricKey,
};

#[cfg(not(feature = "secure-storage"))]
use vauchi_core::storage::secure::{FileKeyStorage, SecureStorage};

/// Legacy hardcoded password used before per-installation backup passwords.
const LEGACY_BACKUP_PASSWORD: &str = "vauchi-local-storage";

/// Default relay URL.
const DEFAULT_RELAY_URL: &str = "wss://relay.vauchi.app";

/// Backend for Vauchi operations.
pub struct Backend {
    pub(crate) storage: Storage,
    pub(crate) identity: Option<Identity>,
    pub(crate) backup_data: Option<Vec<u8>>,
    pub(crate) display_name: Option<String>,
    pub(crate) relay_url: String,
    pub(crate) data_dir: std::path::PathBuf,
}

/// Contact card field information for display.
#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub field_type: String,
    pub label: String,
    pub value: String,
}

/// Contact information for display.
#[derive(Debug, Clone)]
pub struct ContactInfo {
    pub id: String,
    pub display_name: String,
    pub verified: bool,
    pub recovery_trusted: bool,
}

/// Loads or generates a per-installation random fallback key from `data_dir/.fallback-key`.
///
/// Used only when the `secure-storage` feature is disabled. Each installation
/// gets a unique random key instead of a hardcoded constant.
#[cfg(not(feature = "secure-storage"))]
fn load_or_generate_fallback_key(data_dir: &Path) -> Result<SymmetricKey> {
    let key_path = data_dir.join(".fallback-key");

    if key_path.exists() {
        let bytes = std::fs::read(&key_path).context("Failed to read fallback key")?;
        if bytes.len() != 32 {
            anyhow::bail!(
                "Invalid fallback key length ({}), expected 32. Delete {} to regenerate.",
                bytes.len(),
                key_path.display()
            );
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        return Ok(SymmetricKey::from_bytes(arr));
    }

    // Generate a new random key
    let key = SymmetricKey::generate();

    // Ensure parent directory exists
    std::fs::create_dir_all(data_dir).context("Failed to create data directory")?;

    std::fs::write(&key_path, key.as_bytes()).context("Failed to write fallback key")?;

    // Set restrictive permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))
            .context("Failed to set fallback key permissions")?;
    }

    Ok(key)
}

/// Loads or generates a per-installation random backup password from `data_dir/.backup-password`.
///
/// Each installation gets a unique random password (32 random bytes, hex-encoded)
/// instead of the old hardcoded `"vauchi-local-storage"` constant.
fn load_or_generate_backup_password(data_dir: &Path) -> Result<String> {
    let password_path = data_dir.join(".backup-password");

    if password_path.exists() {
        let content =
            std::fs::read_to_string(&password_path).context("Failed to read backup password")?;
        let trimmed = content.trim().to_string();
        if trimmed.len() != 64 {
            anyhow::bail!(
                "Invalid backup password length ({}), expected 64 hex chars. Delete {} to regenerate.",
                trimmed.len(),
                password_path.display()
            );
        }
        return Ok(trimmed);
    }

    // Generate a new random password (32 random bytes, hex-encoded = 64 chars)
    let key = SymmetricKey::generate();
    let password: String = key
        .as_bytes()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();

    std::fs::create_dir_all(data_dir).context("Failed to create data directory")?;
    std::fs::write(&password_path, &password).context("Failed to write backup password")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&password_path, std::fs::Permissions::from_mode(0o600))
            .context("Failed to set backup password permissions")?;
    }

    Ok(password)
}

/// Formats a shred report and verification into a display string.
fn format_shred_summary(
    report: &vauchi_core::api::ShredReport,
    verification: &vauchi_core::api::ShredVerification,
) -> String {
    let status = if verification.all_clear {
        "ALL CLEAR"
    } else {
        "WARNING: incomplete"
    };
    format!(
        "Shred complete [{}] — {} contacts notified, purge={}, SMK={}, DB={}",
        status,
        report.contacts_notified,
        report.relay_purge_sent,
        report.smk_destroyed,
        report.sqlite_destroyed,
    )
}

impl Backend {
    /// Returns the per-installation backup password.
    pub(crate) fn backup_password(&self) -> Result<String> {
        load_or_generate_backup_password(&self.data_dir)
    }

    /// Loads or creates the storage encryption key using SecureStorage.
    ///
    /// When the `secure-storage` feature is enabled, uses the OS keychain.
    /// Otherwise, falls back to encrypted file storage.
    #[allow(unused_variables)]
    fn load_or_create_storage_key(data_dir: &Path) -> Result<SymmetricKey> {
        const KEY_NAME: &str = "storage_key";

        #[cfg(feature = "secure-storage")]
        {
            let storage = PlatformKeyring::new("vauchi-tui");
            match storage.load_key(KEY_NAME) {
                Ok(Some(bytes)) if bytes.len() == 32 => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    Ok(SymmetricKey::from_bytes(arr))
                }
                Ok(Some(_)) => {
                    anyhow::bail!("Invalid storage key length in keychain");
                }
                Ok(None) => {
                    // Generate and save new key
                    let key = SymmetricKey::generate();
                    storage
                        .save_key(KEY_NAME, key.as_bytes())
                        .map_err(|e| anyhow::anyhow!("Failed to save key to keychain: {}", e))?;
                    Ok(key)
                }
                Err(e) => {
                    anyhow::bail!("Keychain error: {}", e);
                }
            }
        }

        #[cfg(not(feature = "secure-storage"))]
        {
            let fallback_key = load_or_generate_fallback_key(data_dir)?;

            let key_dir = data_dir.join("keys");
            let storage = FileKeyStorage::new(key_dir, fallback_key);

            match storage.load_key(KEY_NAME) {
                Ok(Some(bytes)) if bytes.len() == 32 => {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&bytes);
                    Ok(SymmetricKey::from_bytes(arr))
                }
                Ok(Some(_)) => {
                    anyhow::bail!("Invalid storage key length");
                }
                Ok(None) => {
                    // Generate and save new key
                    let key = SymmetricKey::generate();
                    storage
                        .save_key(KEY_NAME, key.as_bytes())
                        .map_err(|e| anyhow::anyhow!("Failed to save storage key: {}", e))?;
                    Ok(key)
                }
                Err(e) => {
                    anyhow::bail!("Storage error: {}", e);
                }
            }
        }
    }

    /// Create a new backend.
    pub fn new(data_dir: &Path) -> Result<Self> {
        // Ensure data directory exists
        std::fs::create_dir_all(data_dir).context("Failed to create data directory")?;

        let db_path = data_dir.join("vauchi.db");

        // Generate or load encryption key using SecureStorage
        let key = Self::load_or_create_storage_key(data_dir)?;

        let storage = Storage::open(&db_path, key).context("Failed to open storage")?;

        // Try to load existing identity with migration from legacy password
        let backup_password = load_or_generate_backup_password(data_dir)?;
        let (identity, backup_data, display_name) =
            if let Ok(Some((backup, name))) = storage.load_identity() {
                let backup_obj = IdentityBackup::new(backup.clone());
                match Identity::import_backup(&backup_obj, &backup_password) {
                    Ok(id) => (Some(id), Some(backup), Some(name)),
                    Err(_) => {
                        // Try legacy hardcoded password for migration
                        match Identity::import_backup(&backup_obj, LEGACY_BACKUP_PASSWORD) {
                            Ok(id) => {
                                // Re-export with per-installation password
                                if let Ok(new_backup) = id.export_backup(&backup_password) {
                                    let new_data = new_backup.as_bytes().to_vec();
                                    let _ = storage.save_identity(&new_data, &name);
                                    (Some(id), Some(new_data), Some(name))
                                } else {
                                    (Some(id), Some(backup), Some(name))
                                }
                            }
                            Err(_) => (None, Some(backup), Some(name)),
                        }
                    }
                }
            } else {
                (None, None, None)
            };

        // Load relay URL with fallback hierarchy:
        // 1. User-configured URL (stored in config file)
        // 2. VAUCHI_RELAY_URL environment variable
        // 3. Default: wss://relay.vauchi.app
        let relay_config_path = data_dir.join("relay_url.txt");
        let relay_url = std::fs::read_to_string(&relay_config_path)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                std::env::var("VAUCHI_RELAY_URL")
                    .ok()
                    .filter(|s| !s.is_empty())
            })
            .unwrap_or_else(|| DEFAULT_RELAY_URL.to_string());

        Ok(Backend {
            storage,
            identity,
            backup_data,
            display_name,
            relay_url,
            data_dir: data_dir.to_path_buf(),
        })
    }

    // ========== Card Management ==========

    /// Get the own contact card.
    pub fn get_card(&self) -> Result<Option<ContactCard>> {
        self.storage
            .load_own_card()
            .context("Failed to load own card")
    }

    /// Get card fields for display.
    pub fn get_card_fields(&self) -> Result<Vec<FieldInfo>> {
        let card = self.get_card()?;
        Ok(card
            .map(|c| {
                c.fields()
                    .iter()
                    .map(|f| FieldInfo {
                        field_type: format!("{:?}", f.field_type()),
                        label: f.label().to_string(),
                        value: f.value().to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    /// Add a field to the card.
    pub fn add_field(&self, field_type: FieldType, label: &str, value: &str) -> Result<()> {
        let mut card = self
            .get_card()?
            .unwrap_or_else(|| ContactCard::new(self.display_name().unwrap_or("User")));

        let field = ContactField::new(field_type, label, value);
        card.add_field(field)
            .map_err(|e| anyhow::anyhow!("{}", e))?;

        self.storage
            .save_own_card(&card)
            .context("Failed to save card")?;

        Ok(())
    }

    /// Remove a field from the card.
    pub fn remove_field(&self, field_id: &str) -> Result<()> {
        let mut card = self.get_card()?.context("No card found")?;
        card.remove_field(field_id)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        self.storage
            .save_own_card(&card)
            .context("Failed to save card")?;
        Ok(())
    }

    /// Update a field's value.
    pub fn update_field(&self, field_label: &str, new_value: &str) -> Result<()> {
        let mut card = self.get_card()?.context("No card found")?;

        // Find the field by label and get both ID and type
        let field = card
            .fields()
            .iter()
            .find(|f| f.label() == field_label)
            .map(|f| (f.id().to_string(), f.field_type(), f.label().to_string()));

        if let Some((field_id, field_type, label)) = field {
            // Remove old field by ID and add new one with updated value
            card.remove_field(&field_id)
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            let new_field = ContactField::new(field_type, &label, new_value);
            card.add_field(new_field)
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            self.storage.save_own_card(&card)?;
            Ok(())
        } else {
            anyhow::bail!("Field not found: {}", field_label)
        }
    }

    /// Parse a field type string.
    pub fn parse_field_type(s: &str) -> FieldType {
        match s.to_lowercase().as_str() {
            "email" => FieldType::Email,
            "phone" => FieldType::Phone,
            "website" => FieldType::Website,
            "address" => FieldType::Address,
            "social" => FieldType::Social,
            _ => FieldType::Custom,
        }
    }

    // ========== Settings ==========

    /// Get the relay URL.
    pub fn relay_url(&self) -> &str {
        &self.relay_url
    }

    /// Returns the data directory path.
    pub fn data_dir(&self) -> &std::path::Path {
        &self.data_dir
    }

    /// Set the relay URL.
    pub fn set_relay_url(&mut self, url: &str) -> Result<()> {
        let url = url.trim();
        if url.is_empty() {
            anyhow::bail!("Relay URL cannot be empty");
        }
        if !url.starts_with("wss://") && !url.starts_with("ws://") {
            anyhow::bail!("Relay URL must start with wss:// or ws://");
        }

        // Save to config file
        let relay_config_path = self.data_dir.join("relay_url.txt");
        std::fs::write(&relay_config_path, url).context("Failed to save relay URL")?;

        self.relay_url = url.to_string();
        Ok(())
    }

    // ========== Device Management ==========

    /// List all linked devices.
    pub fn list_devices(&self) -> Result<Vec<DeviceInfo>> {
        let identity = self.identity.as_ref().context("No identity")?;
        let current_device_id = identity.device_id();

        // Try to load device registry from storage
        if let Ok(Some(registry)) = self.storage.load_device_registry() {
            Ok(registry
                .all_devices()
                .iter()
                .enumerate()
                .map(|(i, device)| DeviceInfo {
                    device_index: i as u32,
                    device_name: device.device_name.clone(),
                    public_key_prefix: hex::encode(&device.device_id[..8]),
                    is_current: device.device_id == *current_device_id,
                    is_active: !device.revoked,
                })
                .collect())
        } else {
            // Return current device only
            Ok(vec![DeviceInfo {
                device_index: 0,
                device_name: "This Device".to_string(),
                public_key_prefix: hex::encode(&current_device_id[..8]),
                is_current: true,
                is_active: true,
            }])
        }
    }

    /// Generate device link QR code and data string using the core API.
    ///
    /// Returns `(qr_ascii, data_string, fingerprint)` for display.
    pub fn generate_device_link(&self) -> Result<DeviceLinkResult> {
        let identity = self.identity.as_ref().context("No identity")?;

        let registry = self
            .storage
            .load_device_registry()?
            .unwrap_or_else(|| identity.initial_device_registry());

        let initiator = identity.create_device_link_initiator(registry);
        let qr = initiator.qr();

        Ok(DeviceLinkResult {
            qr_ascii: qr.to_qr_image_string(),
            data_string: qr.to_data_string(),
            fingerprint: qr.identity_fingerprint(),
        })
    }

    /// Revoke a device from the registry by its public key prefix.
    pub fn revoke_device(&self, device_index: usize) -> Result<String> {
        let identity = self.identity.as_ref().context("No identity")?;

        let mut registry = self
            .storage
            .load_device_registry()?
            .context("No device registry found")?;

        let devices = registry.all_devices().to_vec();
        if device_index >= devices.len() {
            anyhow::bail!("Invalid device index: {}", device_index);
        }

        let device = &devices[device_index];

        if device.device_id == *identity.device_id() {
            anyhow::bail!("Cannot revoke the current device");
        }

        if device.revoked {
            anyhow::bail!("Device '{}' is already revoked", device.device_name);
        }

        let device_name = device.device_name.clone();
        registry.revoke_device(&device.device_id, identity.signing_keypair())?;
        self.storage.save_device_registry(&registry)?;

        Ok(device_name)
    }

    // ========== Tor Privacy Mode ==========

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

    /// Add a bridge address.
    pub fn add_tor_bridge(&self, addr: &str) -> Result<()> {
        let mut config = self.load_tor_config()?;
        if !config.bridges.contains(&addr.to_string()) {
            config.bridges.push(addr.to_string());
            self.storage
                .save_tor_config(&config)
                .context("Failed to save Tor config")?;
        }
        Ok(())
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

    // === GDPR Operations ===

    /// Exports all user data for GDPR compliance.
    pub fn export_gdpr_data(&self) -> Result<String> {
        let export = vauchi_core::api::export_all_data(&self.storage)?;
        let json =
            serde_json::to_string_pretty(&export).context("Failed to serialize GDPR export")?;
        Ok(json)
    }

    /// Gets the current deletion state as a display string.
    pub fn get_deletion_status(&self) -> Result<String> {
        let manager = vauchi_core::api::DeletionManager::new(&self.storage);
        let state = manager.deletion_state()?;
        match state {
            vauchi_core::storage::DeletionState::None => Ok("No deletion scheduled".to_string()),
            vauchi_core::storage::DeletionState::Scheduled { execute_at, .. } => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let remaining = execute_at.saturating_sub(now);
                let days = remaining / 86400;
                Ok(format!("Deletion scheduled — {} days remaining", days))
            }
            vauchi_core::storage::DeletionState::Executed { .. } => {
                Ok("Account deleted".to_string())
            }
        }
    }

    /// Schedules account deletion with grace period.
    pub fn schedule_deletion(&self) -> Result<()> {
        let manager = vauchi_core::api::DeletionManager::new(&self.storage);
        manager.schedule_deletion()?;
        Ok(())
    }

    /// Cancels a scheduled deletion.
    pub fn cancel_deletion(&self) -> Result<()> {
        let manager = vauchi_core::api::DeletionManager::new(&self.storage);
        manager.cancel_deletion()?;
        Ok(())
    }

    /// Creates a SecureStorage instance for shred operations.
    #[allow(unused_variables)]
    fn create_secure_storage(&self) -> Result<Box<dyn SecureStorage>> {
        #[cfg(feature = "secure-storage")]
        {
            Ok(Box::new(PlatformKeyring::new("vauchi-tui")))
        }

        #[cfg(not(feature = "secure-storage"))]
        {
            let fallback_key = load_or_generate_fallback_key(&self.data_dir)?;
            let key_dir = self.data_dir.join("keys");
            Ok(Box::new(FileKeyStorage::new(key_dir, fallback_key)))
        }
    }

    /// Creates a connected RelayClient for shred operations.
    fn create_shred_relay_client(
        &self,
        identity_id: &str,
    ) -> Result<vauchi_core::network::RelayClient<vauchi_core::network::WebSocketTransport>> {
        use vauchi_core::network::{
            RelayClient, RelayClientConfig, TransportConfig, WebSocketTransport,
        };
        let transport_config = TransportConfig {
            server_url: self.relay_url.clone(),
            ..TransportConfig::default()
        };
        let config = RelayClientConfig {
            transport: transport_config,
            ..RelayClientConfig::default()
        };
        let transport = WebSocketTransport::new();
        let mut client = RelayClient::new(transport, config, identity_id.to_string());
        client
            .connect()
            .map_err(|e| anyhow::anyhow!("Failed to connect to relay: {}", e))?;
        Ok(client)
    }

    /// Executes a scheduled account deletion after the grace period.
    pub fn execute_deletion(&self) -> Result<String> {
        let identity = self.identity.as_ref().context("No identity loaded")?;

        let manager = vauchi_core::api::DeletionManager::new(&self.storage);
        let state = manager.deletion_state()?;
        let token = match state {
            vauchi_core::storage::DeletionState::Scheduled {
                scheduled_at,
                execute_at,
            } => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                if now < execute_at {
                    let remaining = execute_at.saturating_sub(now);
                    let days = remaining / 86400;
                    anyhow::bail!("Grace period not elapsed. {} days remaining.", days);
                }
                vauchi_core::api::ShredToken::from_created_at(scheduled_at)
            }
            vauchi_core::storage::DeletionState::None => {
                anyhow::bail!("No deletion scheduled.");
            }
            vauchi_core::storage::DeletionState::Executed { .. } => {
                anyhow::bail!("Account already deleted.");
            }
        };

        let secure_storage = self.create_secure_storage()?;
        let identity_id = hex::encode(identity.signing_public_key());
        let shred_manager = vauchi_core::api::ShredManager::new(
            &self.storage,
            secure_storage.as_ref(),
            identity,
            &self.data_dir,
        );

        let mut purge_client = self.create_shred_relay_client(&identity_id)?;
        let mut revocation_client = self.create_shred_relay_client(&identity_id)?;

        let report = shred_manager
            .hard_shred(token, Some(&mut purge_client), Some(&mut revocation_client))
            .map_err(|e| anyhow::anyhow!("Shred failed: {}", e))?;

        let verification = shred_manager.verify_shred();
        Ok(format_shred_summary(&report, &verification))
    }

    /// Emergency immediate deletion — no grace period.
    pub fn panic_shred(&self) -> Result<String> {
        let identity = self.identity.as_ref().context("No identity loaded")?;

        let secure_storage = self.create_secure_storage()?;
        let identity_id = hex::encode(identity.signing_public_key());
        let shred_manager = vauchi_core::api::ShredManager::new(
            &self.storage,
            secure_storage.as_ref(),
            identity,
            &self.data_dir,
        );

        // Best-effort relay connections
        let mut purge_client = self.create_shred_relay_client(&identity_id).ok();
        let mut revocation_client = self.create_shred_relay_client(&identity_id).ok();

        let report = shred_manager
            .panic_shred(
                purge_client
                    .as_mut()
                    .map(|c| c as &mut dyn vauchi_core::api::PurgeSender),
                revocation_client
                    .as_mut()
                    .map(|c| c as &mut dyn vauchi_core::api::RevocationSender),
            )
            .map_err(|e| anyhow::anyhow!("Panic shred failed: {}", e))?;

        let verification = shred_manager.verify_shred();
        Ok(format_shred_summary(&report, &verification))
    }

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

    /// Gets the structured deletion state.
    pub fn deletion_state(&self) -> Result<vauchi_core::storage::DeletionState> {
        let manager = vauchi_core::api::DeletionManager::new(&self.storage);
        let state = manager.deletion_state()?;
        Ok(state)
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

    // ================================================================
    // Aha Moments
    // ================================================================

    /// Load the aha moment tracker from disk.
    fn load_aha_tracker(&self) -> AhaMomentTracker {
        let path = self.data_dir.join("aha_tracker.json");
        match std::fs::read_to_string(&path) {
            Ok(json) => AhaMomentTracker::from_json(&json).unwrap_or_default(),
            Err(_) => AhaMomentTracker::new(),
        }
    }

    /// Save the aha moment tracker to disk.
    fn save_aha_tracker(&self, tracker: &AhaMomentTracker) -> Result<()> {
        let path = self.data_dir.join("aha_tracker.json");
        let json = tracker
            .to_json()
            .context("Failed to serialize aha tracker")?;
        std::fs::write(&path, json).context("Failed to write aha tracker")?;
        Ok(())
    }

    /// Check if an aha moment should fire, returning it if not yet seen.
    /// Automatically persists the tracker after triggering.
    pub fn check_aha_moment(&self, moment_type: AhaMomentType) -> Option<AhaMoment> {
        let mut tracker = self.load_aha_tracker();
        let moment = tracker.try_trigger(moment_type);
        if moment.is_some() {
            let _ = self.save_aha_tracker(&tracker);
        }
        moment
    }

    /// Check if an aha moment should fire with context.
    pub fn check_aha_moment_with_context(
        &self,
        moment_type: AhaMomentType,
        context: String,
    ) -> Option<AhaMoment> {
        let mut tracker = self.load_aha_tracker();
        let moment = tracker.try_trigger_with_context(moment_type, context);
        if moment.is_some() {
            let _ = self.save_aha_tracker(&tracker);
        }
        moment
    }
}

/// Device information for display.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DeviceInfo {
    pub device_index: u32,
    pub device_name: String,
    pub public_key_prefix: String,
    pub is_current: bool,
    pub is_active: bool,
}

/// Result from generating a device link.
#[derive(Debug, Clone)]
pub struct DeviceLinkResult {
    /// ASCII art QR code for terminal display.
    pub qr_ascii: String,
    /// Data string (base64) for copy-paste transport.
    pub data_string: String,
    /// Identity fingerprint for verification.
    pub fingerprint: String,
}

/// Available field types for selection.
pub const FIELD_TYPES: &[&str] = &["Email", "Phone", "Website", "Address", "Social", "Custom"];

// ===========================================================================
// Backend Tests
// Trace: features/identity_management.feature, contact_card_management.feature
// ===========================================================================

// INLINE_TEST_REQUIRED: Tests need access to private Backend fields (storage, identity)
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::TempDir;

    // Mutex to serialize tests that modify VAUCHI_RELAY_URL env var
    static ENV_VAR_MUTEX: Mutex<()> = Mutex::new(());

    /// Create a test backend with isolated data directory.
    fn create_test_backend() -> (Backend, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let backend = Backend::new(temp_dir.path()).expect("Failed to create backend");
        (backend, temp_dir)
    }

    // === Identity Management Tests ===
    // Trace: identity_management.feature

    /// Trace: identity_management.feature - New backend has no identity
    #[test]
    fn test_new_backend_has_no_identity() {
        let (backend, _temp) = create_test_backend();
        assert!(!backend.has_identity());
        assert!(backend.display_name().is_none());
        assert!(backend.public_id().is_none());
    }

    /// Trace: identity_management.feature - Create new identity
    #[test]
    fn test_create_identity() {
        let (mut backend, _temp) = create_test_backend();

        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        assert!(backend.has_identity());
        assert_eq!(backend.display_name(), Some("Alice Smith"));
        assert!(backend.public_id().is_some());
    }

    /// Trace: identity_management.feature - Identity persists across backend instances
    #[test]
    fn test_identity_persistence() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create identity in first backend
        {
            let mut backend = Backend::new(temp_dir.path()).expect("Failed to create backend");
            backend
                .create_identity("Alice Smith")
                .expect("Failed to create identity");
        }

        // Load in second backend
        {
            let backend = Backend::new(temp_dir.path()).expect("Failed to load backend");
            assert!(backend.has_identity());
            assert_eq!(backend.display_name(), Some("Alice Smith"));
        }
    }

    // === Contact Card Management Tests ===
    // Trace: contact_card_management.feature

    /// Trace: contact_card_management.feature - New identity has empty card
    #[test]
    fn test_new_identity_empty_card() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        let fields = backend.get_card_fields().expect("Failed to get fields");
        assert!(fields.is_empty());
    }

    /// Trace: contact_card_management.feature - Add phone field
    #[test]
    fn test_add_phone_field() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        backend
            .add_field(FieldType::Phone, "Mobile", "+1-555-123-4567")
            .expect("Failed to add field");

        let fields = backend.get_card_fields().expect("Failed to get fields");
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].label, "Mobile");
        assert_eq!(fields[0].value, "+1-555-123-4567");
    }

    /// Trace: contact_card_management.feature - Add email field
    #[test]
    fn test_add_email_field() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        backend
            .add_field(FieldType::Email, "Work", "alice@company.com")
            .expect("Failed to add field");

        let fields = backend.get_card_fields().expect("Failed to get fields");
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].label, "Work");
        assert_eq!(fields[0].value, "alice@company.com");
    }

    /// Trace: contact_card_management.feature - Add multiple fields
    #[test]
    fn test_add_multiple_fields() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        backend
            .add_field(FieldType::Phone, "Mobile", "+1-555-123-4567")
            .expect("Failed to add field");
        backend
            .add_field(FieldType::Email, "Work", "alice@company.com")
            .expect("Failed to add field");
        backend
            .add_field(FieldType::Website, "Personal", "https://alice.example.com")
            .expect("Failed to add field");

        let fields = backend.get_card_fields().expect("Failed to get fields");
        assert_eq!(fields.len(), 3);
    }

    /// Trace: contact_card_management.feature - Remove field
    /// Note: Backend.remove_field takes field_id (unique ID), not label
    #[test]
    fn test_remove_field() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        backend
            .add_field(FieldType::Phone, "Mobile", "+1-555-123-4567")
            .expect("Failed to add field");

        // Get the card directly and get the field's unique ID
        let card = backend.get_card().expect("get card").unwrap();
        let field_id = card.fields()[0].id().to_string();
        backend
            .remove_field(&field_id)
            .expect("Failed to remove field");

        let fields = backend.get_card_fields().expect("Failed to get fields");
        assert!(fields.is_empty());
    }

    /// Trace: contact_card_management.feature - Update field value
    /// Note: Backend.update_field takes field label (finds field by label, then modifies)
    #[test]
    fn test_update_field() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        backend
            .add_field(FieldType::Phone, "Mobile", "+1-555-123-4567")
            .expect("Failed to add field");

        // update_field uses label to find and update the field
        backend
            .update_field("Mobile", "+1-555-999-8888")
            .expect("Failed to update field");

        let fields = backend.get_card_fields().expect("Failed to get fields");
        assert_eq!(fields[0].value, "+1-555-999-8888");
    }

    /// Trace: contact_card_management.feature - Update display name
    #[test]
    fn test_update_display_name() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        backend
            .update_display_name("Alice S.")
            .expect("Failed to update name");

        assert_eq!(backend.display_name(), Some("Alice S."));
    }

    /// Trace: contact_card_management.feature - Empty display name rejected
    #[test]
    fn test_empty_display_name_rejected() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        let result = backend.update_display_name("");
        assert!(result.is_err());
        assert_eq!(backend.display_name(), Some("Alice Smith"));
    }

    /// Trace: contact_card_management.feature - Display name too long rejected
    #[test]
    fn test_long_display_name_rejected() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        let long_name = "A".repeat(101);
        let result = backend.update_display_name(&long_name);
        assert!(result.is_err());
    }

    // === Field Type Parsing Tests ===

    #[test]
    fn test_parse_field_type_email() {
        assert!(matches!(
            Backend::parse_field_type("email"),
            FieldType::Email
        ));
        assert!(matches!(
            Backend::parse_field_type("EMAIL"),
            FieldType::Email
        ));
    }

    #[test]
    fn test_parse_field_type_phone() {
        assert!(matches!(
            Backend::parse_field_type("phone"),
            FieldType::Phone
        ));
    }

    #[test]
    fn test_parse_field_type_website() {
        assert!(matches!(
            Backend::parse_field_type("website"),
            FieldType::Website
        ));
    }

    #[test]
    fn test_parse_field_type_address() {
        assert!(matches!(
            Backend::parse_field_type("address"),
            FieldType::Address
        ));
    }

    #[test]
    fn test_parse_field_type_social() {
        assert!(matches!(
            Backend::parse_field_type("social"),
            FieldType::Social
        ));
    }

    #[test]
    fn test_parse_field_type_custom() {
        assert!(matches!(
            Backend::parse_field_type("other"),
            FieldType::Custom
        ));
        assert!(matches!(
            Backend::parse_field_type("unknown"),
            FieldType::Custom
        ));
    }

    // === Contacts Tests ===
    // Trace: contacts_management.feature

    /// Trace: contacts_management.feature - New identity has no contacts
    #[test]
    fn test_new_identity_no_contacts() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        let contacts = backend.list_contacts().expect("Failed to list contacts");
        assert!(contacts.is_empty());
        assert_eq!(backend.contact_count().unwrap(), 0);
    }

    // === Settings Tests ===

    /// Test relay URL configuration
    #[test]
    fn test_relay_url_default() {
        let _lock = ENV_VAR_MUTEX.lock().unwrap();

        // Save existing env var value
        let saved_env = std::env::var("VAUCHI_RELAY_URL").ok();
        // Remove env var for this test
        std::env::remove_var("VAUCHI_RELAY_URL");

        // Create temp dir and backend AFTER removing env var
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let backend = Backend::new(temp_dir.path()).expect("Failed to create backend");
        let relay_url = backend.relay_url().to_string();

        // Restore env var
        if let Some(val) = saved_env {
            std::env::set_var("VAUCHI_RELAY_URL", val);
        }

        assert_eq!(relay_url, "wss://relay.vauchi.app");
    }

    /// Test setting relay URL
    #[test]
    fn test_set_relay_url() {
        let (mut backend, _temp) = create_test_backend();

        backend
            .set_relay_url("wss://custom.relay.example.com")
            .expect("Failed to set relay URL");

        assert_eq!(backend.relay_url(), "wss://custom.relay.example.com");
    }

    /// Test relay URL persistence
    #[test]
    fn test_relay_url_persistence() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Set relay URL in first backend
        {
            let mut backend = Backend::new(temp_dir.path()).expect("Failed to create backend");
            backend
                .set_relay_url("wss://custom.relay.example.com")
                .expect("Failed to set relay URL");
        }

        // Load in second backend
        {
            let backend = Backend::new(temp_dir.path()).expect("Failed to load backend");
            assert_eq!(backend.relay_url(), "wss://custom.relay.example.com");
        }
    }

    /// Test invalid relay URL rejected
    #[test]
    fn test_invalid_relay_url_rejected() {
        let (mut backend, _temp) = create_test_backend();

        let result = backend.set_relay_url("invalid-url");
        assert!(result.is_err());
    }

    /// Test empty relay URL rejected
    #[test]
    fn test_empty_relay_url_rejected() {
        let (mut backend, _temp) = create_test_backend();

        let result = backend.set_relay_url("");
        assert!(result.is_err());
    }

    /// Test VAUCHI_RELAY_URL env var is used when no config file exists
    #[test]
    fn test_relay_url_from_env_var() {
        let _lock = ENV_VAR_MUTEX.lock().unwrap();

        // Save existing env var value
        let saved_env = std::env::var("VAUCHI_RELAY_URL").ok();

        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Set env var before creating backend
        std::env::set_var("VAUCHI_RELAY_URL", "wss://env.relay.example.com");

        let backend = Backend::new(temp_dir.path()).expect("Failed to create backend");
        let relay_url = backend.relay_url().to_string();

        // Restore or remove env var
        match saved_env {
            Some(val) => std::env::set_var("VAUCHI_RELAY_URL", val),
            None => std::env::remove_var("VAUCHI_RELAY_URL"),
        }

        assert_eq!(relay_url, "wss://env.relay.example.com");
    }

    /// Test config file takes precedence over env var
    #[test]
    fn test_config_file_precedence_over_env_var() {
        let _lock = ENV_VAR_MUTEX.lock().unwrap();

        // Save existing env var value
        let saved_env = std::env::var("VAUCHI_RELAY_URL").ok();

        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Write config file
        let relay_config_path = temp_dir.path().join("relay_url.txt");
        std::fs::write(&relay_config_path, "wss://config.relay.example.com")
            .expect("Failed to write config");

        // Set env var
        std::env::set_var("VAUCHI_RELAY_URL", "wss://env.relay.example.com");

        let backend = Backend::new(temp_dir.path()).expect("Failed to create backend");
        let relay_url = backend.relay_url().to_string();

        // Restore or remove env var
        match saved_env {
            Some(val) => std::env::set_var("VAUCHI_RELAY_URL", val),
            None => std::env::remove_var("VAUCHI_RELAY_URL"),
        }

        // Config file should take precedence
        assert_eq!(relay_url, "wss://config.relay.example.com");
    }

    // === Backup Tests ===
    // Trace: identity_management.feature - backup/restore

    /// Trace: identity_management.feature - Export backup
    #[test]
    fn test_export_backup() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        // Use a strong password that meets requirements
        let backup = backend
            .export_backup("Str0ng!P@ssw0rd#2024")
            .expect("Failed to export backup");

        // Backup should be hex-encoded
        assert!(hex::decode(&backup).is_ok());
        assert!(!backup.is_empty());
    }

    /// Trace: identity_management.feature - Import backup
    #[test]
    fn test_import_backup() {
        let backup_data;
        let password = "Str0ng!P@ssw0rd#2024";

        // Create identity and export backup
        {
            let (mut backend1, _temp1) = create_test_backend();
            backend1
                .create_identity("Alice Smith")
                .expect("Failed to create identity");
            backend1
                .add_field(FieldType::Email, "Work", "alice@work.com")
                .expect("Failed to add field");
            backup_data = backend1
                .export_backup(password)
                .expect("Failed to export backup");
        }

        // Import backup into new backend
        let (mut backend2, _temp2) = create_test_backend();
        backend2
            .import_backup(&backup_data, password)
            .expect("Failed to import backup");

        assert!(backend2.has_identity());
        assert_eq!(backend2.display_name(), Some("Alice Smith"));
    }

    /// Trace: identity_management.feature - Import with wrong password fails
    #[test]
    fn test_import_backup_wrong_password() {
        let (mut backend1, _temp1) = create_test_backend();
        backend1
            .create_identity("Alice Smith")
            .expect("Failed to create identity");
        let backup_data = backend1
            .export_backup("C0rrect!P@ssw0rd#2024")
            .expect("Failed to export backup");

        let (mut backend2, _temp2) = create_test_backend();
        let result = backend2.import_backup(&backup_data, "Wr0ng!P@ssw0rd#2024");

        assert!(result.is_err());
    }

    // === Exchange Tests ===
    // Trace: contact_exchange.feature

    /// Trace: contact_exchange.feature - Generate exchange QR
    #[test]
    fn test_generate_exchange_qr() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        let qr = backend
            .generate_exchange_qr()
            .expect("Failed to generate QR");

        assert!(!qr.data.is_empty());
        assert!(qr.expires_in_secs > 0);
        assert!(qr.remaining_secs() <= qr.expires_in_secs);
    }

    // === Device Tests ===
    // Trace: device_management.feature

    /// Trace: device_management.feature - List devices shows current device
    #[test]
    fn test_list_devices() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        let devices = backend.list_devices().expect("Failed to list devices");

        assert_eq!(devices.len(), 1);
        assert!(devices[0].is_current);
        assert!(devices[0].is_active);
    }

    /// Trace: device_management.feature - Generate device link
    #[test]
    fn test_generate_device_link() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        let result = backend
            .generate_device_link()
            .expect("Failed to generate link");

        assert!(
            !result.data_string.is_empty(),
            "data_string must not be empty"
        );
        assert!(!result.qr_ascii.is_empty(), "qr_ascii must not be empty");
        assert!(
            !result.fingerprint.is_empty(),
            "fingerprint must not be empty"
        );
        // Fingerprint follows XXXX-XXXX-XXXX-XXXX pattern
        assert!(
            result.fingerprint.contains('-'),
            "fingerprint must contain dashes"
        );
    }

    /// Trace: device_management.feature - Revoke device requires other devices
    #[test]
    fn test_revoke_device_no_registry() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        let result = backend.revoke_device(0);
        assert!(result.is_err(), "Should fail without device registry");
    }

    /// Trace: device_management.feature - Cannot revoke current device
    #[test]
    fn test_revoke_current_device_rejected() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        // Initialize registry so current device is at index 0
        let identity = backend.identity.as_ref().unwrap();
        let registry = identity.initial_device_registry();
        backend.storage.save_device_registry(&registry).unwrap();

        let result = backend.revoke_device(0);
        assert!(result.is_err(), "Should reject revoking current device");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("current device"),
            "Error should mention current device"
        );
    }

    /// Trace: device_management.feature - Invalid device index rejected
    #[test]
    fn test_revoke_invalid_index() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        let identity = backend.identity.as_ref().unwrap();
        let registry = identity.initial_device_registry();
        backend.storage.save_device_registry(&registry).unwrap();

        let result = backend.revoke_device(99);
        assert!(result.is_err(), "Should reject invalid index");
    }

    // === Sync Tests ===
    // Trace: sync_updates.feature

    /// Trace: sync_updates.feature - Sync status without identity
    #[test]
    fn test_sync_status_no_identity() {
        let (backend, _temp) = create_test_backend();
        assert_eq!(backend.sync_status(), "No identity");
    }

    /// Trace: sync_updates.feature - Sync status with identity
    #[test]
    fn test_sync_status_with_identity() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");
        assert_eq!(backend.sync_status(), "Ready to sync");
    }

    /// Trace: sync_updates.feature - Pending update count starts at zero
    #[test]
    fn test_pending_update_count_zero() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        let count = backend.pending_update_count().expect("Failed to get count");
        assert_eq!(count, 0);
    }

    // === QRData Tests ===

    #[test]
    fn test_qr_data_remaining_secs() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let qr = QRData {
            data: "test".to_string(),
            generated_at: now,
            expires_in_secs: 300,
        };

        // Should have close to 300 seconds remaining
        assert!(qr.remaining_secs() <= 300);
        assert!(qr.remaining_secs() >= 299);
    }

    #[test]
    fn test_qr_data_expired() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let qr = QRData {
            data: "test".to_string(),
            generated_at: now - 400, // 400 seconds ago
            expires_in_secs: 300,    // Expires after 300
        };

        assert_eq!(qr.remaining_secs(), 0);
        assert!(qr.is_expired());
    }

    // === SyncResult Tests ===

    #[test]
    fn test_sync_result_success() {
        let result = SyncResult::success(2, 3, 1);
        assert!(result.success);
        assert_eq!(result.contacts_added, 2);
        assert_eq!(result.cards_updated, 3);
        assert_eq!(result.updates_sent, 1);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_sync_result_error() {
        let result = SyncResult::error("Connection failed");
        assert!(!result.success);
        assert_eq!(result.contacts_added, 0);
        assert_eq!(result.error, Some("Connection failed".to_string()));
    }

    // === GDPR Backend Tests ===
    // Trace: privacy_compliance.feature

    /// Trace: privacy_compliance.feature - Export GDPR data
    #[test]
    fn test_export_gdpr_data() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        let json = backend
            .export_gdpr_data()
            .expect("Failed to export GDPR data");
        assert!(!json.is_empty());
        assert!(json.contains("version"));
    }

    /// Trace: privacy_compliance.feature - Schedule deletion
    #[test]
    fn test_schedule_and_cancel_deletion() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        // Schedule deletion
        backend
            .schedule_deletion()
            .expect("Failed to schedule deletion");

        let status = backend
            .get_deletion_status()
            .expect("Failed to get deletion status");
        assert!(status.contains("scheduled") || status.contains("Deletion"));

        // Cancel deletion
        backend
            .cancel_deletion()
            .expect("Failed to cancel deletion");

        let status = backend
            .get_deletion_status()
            .expect("Failed to get deletion status");
        assert!(status.contains("No deletion"));
    }

    /// Trace: privacy_compliance.feature - Deletion state structured
    #[test]
    fn test_deletion_state_structured() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        let state = backend.deletion_state().expect("Failed to get state");
        assert!(matches!(state, vauchi_core::storage::DeletionState::None));
    }

    /// Trace: privacy_compliance.feature - Consent records empty initially
    #[test]
    fn test_consent_records_empty() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        let records = backend
            .consent_records()
            .expect("Failed to get consent records");
        assert!(records.is_empty());
    }

    /// Trace: privacy_compliance.feature - Grant and revoke consent
    #[test]
    fn test_grant_and_revoke_consent() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        // Grant consent
        backend
            .grant_consent(vauchi_core::api::ConsentType::Analytics)
            .expect("Failed to grant consent");

        let records = backend.consent_records().expect("Failed to get records");
        assert!(!records.is_empty());
        // Find the analytics record (should be granted)
        let analytics: Vec<_> = records
            .iter()
            .filter(|r| matches!(r.consent_type, vauchi_core::api::ConsentType::Analytics))
            .collect();
        assert!(!analytics.is_empty());

        // Revoke consent
        backend
            .revoke_consent(vauchi_core::api::ConsentType::Analytics)
            .expect("Failed to revoke consent");

        // Verify consent status shows revoked
        let status = backend.get_consent_status().expect("Failed to get status");
        assert!(status.contains("Revoked"));
    }

    /// Trace: privacy_compliance.feature - Consent status display
    #[test]
    fn test_consent_status_display() {
        let (mut backend, _temp) = create_test_backend();
        backend
            .create_identity("Alice Smith")
            .expect("Failed to create identity");

        // Initially empty
        let status = backend.get_consent_status().expect("Failed to get status");
        assert_eq!(status, "No consent records");

        // Grant a consent
        backend
            .grant_consent(vauchi_core::api::ConsentType::DataProcessing)
            .expect("Failed to grant consent");

        let status = backend.get_consent_status().expect("Failed to get status");
        assert!(status.contains("Granted"));
    }

    // === Aha Moment Tests ===
    // Trace: aha_moments.feature

    #[test]
    fn test_aha_moment_triggers_on_first_call() {
        let (backend, _temp) = create_test_backend();

        // First call should trigger
        let moment = backend.check_aha_moment(AhaMomentType::CardCreationComplete);
        assert!(moment.is_some());
        assert_eq!(
            moment.unwrap().moment_type,
            AhaMomentType::CardCreationComplete
        );

        // Second call should not trigger (already seen)
        let moment = backend.check_aha_moment(AhaMomentType::CardCreationComplete);
        assert!(moment.is_none());
    }

    #[test]
    fn test_aha_moment_persists_across_loads() {
        let (backend, temp) = create_test_backend();

        // Trigger a moment
        let moment = backend.check_aha_moment(AhaMomentType::FirstEdit);
        assert!(moment.is_some());

        // Create a new backend from the same directory
        let backend2 = Backend::new(temp.path()).expect("Failed to create second backend");

        // Should not trigger again (persisted)
        let moment = backend2.check_aha_moment(AhaMomentType::FirstEdit);
        assert!(moment.is_none());

        // Different moment type should still trigger
        let moment = backend2.check_aha_moment(AhaMomentType::FirstContactAdded);
        assert!(moment.is_some());
    }

    #[test]
    fn test_aha_moment_with_context() {
        let (backend, _temp) = create_test_backend();

        let moment = backend
            .check_aha_moment_with_context(AhaMomentType::FirstContactAdded, "Bob".to_string());
        assert!(moment.is_some());
        let m = moment.unwrap();
        assert!(m.message().contains("Bob"));
    }

    #[test]
    fn test_aha_moment_independent_types() {
        let (backend, _temp) = create_test_backend();

        // Trigger one type
        let moment = backend.check_aha_moment(AhaMomentType::CardCreationComplete);
        assert!(moment.is_some());

        // Other types should still be available
        let moment = backend.check_aha_moment(AhaMomentType::FirstEdit);
        assert!(moment.is_some());
        let moment = backend.check_aha_moment(AhaMomentType::FirstContactAdded);
        assert!(moment.is_some());
        let moment = backend.check_aha_moment(AhaMomentType::FirstUpdateReceived);
        assert!(moment.is_some());
        let moment = backend.check_aha_moment(AhaMomentType::FirstOutboundDelivered);
        assert!(moment.is_some());

        // None should trigger again
        let moment = backend.check_aha_moment(AhaMomentType::CardCreationComplete);
        assert!(moment.is_none());
    }

    // === Recovery Trust Tests ===
    // Trace: features/contact_recovery.feature

    /// Helper: create a test contact and save it to storage.
    fn create_and_save_test_contact(backend: &Backend, name: &str) -> String {
        use vauchi_core::{Contact, ContactCard, SymmetricKey};
        let pk: [u8; 32] = {
            let mut arr = [0u8; 32];
            // Derive unique key from name bytes
            for (i, b) in name.bytes().enumerate() {
                arr[i % 32] ^= b;
            }
            arr
        };
        let card = ContactCard::new(name);
        let shared_key = SymmetricKey::generate();
        let contact = Contact::from_exchange(pk, card, shared_key);
        let id = contact.id().to_string();
        backend
            .storage
            .save_contact(&contact)
            .expect("Failed to save contact");
        id
    }

    /// Trace: contact_recovery.feature line 57 - "Mark contact as trusted"
    #[test]
    fn test_toggle_recovery_trust_on() {
        let (mut backend, _temp) = create_test_backend();
        backend.create_identity("Alice").unwrap();
        let id = create_and_save_test_contact(&backend, "Bob");

        let new_state = backend.toggle_recovery_trust(&id).unwrap();
        assert!(new_state, "Should be trusted after toggling on");

        let contacts = backend.list_contacts().unwrap();
        assert!(contacts[0].recovery_trusted);
    }

    /// Trace: contact_recovery.feature line 64 - "Remove recovery trust"
    #[test]
    fn test_toggle_recovery_trust_off() {
        let (mut backend, _temp) = create_test_backend();
        backend.create_identity("Alice").unwrap();
        let id = create_and_save_test_contact(&backend, "Bob");

        // Trust then untrust
        backend.toggle_recovery_trust(&id).unwrap();
        let new_state = backend.toggle_recovery_trust(&id).unwrap();
        assert!(!new_state, "Should be untrusted after toggling off");

        let contacts = backend.list_contacts().unwrap();
        assert!(!contacts[0].recovery_trusted);
    }

    /// Trace: contact_recovery.feature line 148 - "Removing trust doesn't affect other properties"
    #[test]
    fn test_recovery_trust_preserves_other_properties() {
        let (mut backend, _temp) = create_test_backend();
        backend.create_identity("Alice").unwrap();
        let id = create_and_save_test_contact(&backend, "Bob");

        // Trust, then untrust
        backend.toggle_recovery_trust(&id).unwrap();
        backend.toggle_recovery_trust(&id).unwrap();

        // Verify other properties are preserved
        let contacts = backend.list_contacts().unwrap();
        assert_eq!(contacts[0].display_name, "Bob");
        assert!(!contacts[0].verified); // Default is unverified
    }

    /// Trace: contact_recovery.feature - Trusted contact count
    #[test]
    fn test_trusted_contact_count() {
        let (mut backend, _temp) = create_test_backend();
        backend.create_identity("Alice").unwrap();

        let id1 = create_and_save_test_contact(&backend, "Bob");
        let _id2 = create_and_save_test_contact(&backend, "Carol");
        let id3 = create_and_save_test_contact(&backend, "Dave");

        assert_eq!(backend.trusted_contact_count().unwrap(), 0);

        backend.toggle_recovery_trust(&id1).unwrap();
        assert_eq!(backend.trusted_contact_count().unwrap(), 1);

        backend.toggle_recovery_trust(&id3).unwrap();
        assert_eq!(backend.trusted_contact_count().unwrap(), 2);

        backend.toggle_recovery_trust(&id1).unwrap(); // untrust
        assert_eq!(backend.trusted_contact_count().unwrap(), 1);
    }

    /// Trace: contact_recovery.feature - ContactInfo includes recovery_trusted
    #[test]
    fn test_contact_info_includes_recovery_trusted() {
        let (mut backend, _temp) = create_test_backend();
        backend.create_identity("Alice").unwrap();
        let id = create_and_save_test_contact(&backend, "Bob");

        let contacts = backend.list_contacts().unwrap();
        assert!(!contacts[0].recovery_trusted);

        backend.toggle_recovery_trust(&id).unwrap();
        let contacts = backend.list_contacts().unwrap();
        assert!(contacts[0].recovery_trusted);
    }

    // === Fallback Key Storage Tests ===
    // Trace: Phase 2 security hardening — hardcoded key removal (Item 28)

    /// Verify fallback key is NOT the old hardcoded value
    #[cfg(not(feature = "secure-storage"))]
    #[test]
    fn test_fallback_key_is_random_not_hardcoded() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let key1 = load_or_generate_fallback_key(temp_dir.path()).expect("Failed to generate key");
        let key2 = load_or_generate_fallback_key(temp_dir.path()).expect("Failed to generate key");

        assert_eq!(
            key1.as_bytes(),
            key2.as_bytes(),
            "Same data dir must produce same key"
        );

        // Verify it's NOT the old hardcoded key
        let old_hardcoded: [u8; 32] = [
            0x57, 0x65, 0x62, 0x42, 0x6f, 0x6f, 0x6b, 0x54, // "WebBookT"
            0x55, 0x49, 0x53, 0x74, 0x6f, 0x72, 0x61, 0x67, // "UIStorag"
            0x65, 0x4b, 0x65, 0x79, 0x46, 0x61, 0x6c, 0x6c, // "eKeyFall"
            0x62, 0x61, 0x63, 0x6b, 0x56, 0x31, 0x00, 0x00, // "backV1\0\0"
        ];
        assert_ne!(
            key1.as_bytes(),
            &old_hardcoded,
            "Must not use old hardcoded key"
        );
    }

    /// Verify different installations produce different keys
    #[cfg(not(feature = "secure-storage"))]
    #[test]
    fn test_fallback_key_differs_per_install() {
        let temp1 = TempDir::new().expect("Failed to create temp dir");
        let temp2 = TempDir::new().expect("Failed to create temp dir");

        let key1 = load_or_generate_fallback_key(temp1.path()).expect("Failed to generate key");
        let key2 = load_or_generate_fallback_key(temp2.path()).expect("Failed to generate key");

        assert_ne!(
            key1.as_bytes(),
            key2.as_bytes(),
            "Different installs must produce different keys"
        );
    }

    // === Field Validation Tests ===
    // Trace: features/field_validation.feature

    /// Trace: field_validation.feature - Get status for unvalidated field returns Unverified
    #[test]
    fn test_get_field_validation_status_returns_unverified_for_new_field() {
        let (mut backend, _temp) = create_test_backend();
        backend.create_identity("Alice").unwrap();
        let bob_id = create_and_save_test_contact(&backend, "Bob");

        let status = backend
            .get_field_validation_status(&bob_id, "twitter", "@bob")
            .unwrap();

        assert_eq!(
            status.trust_level,
            vauchi_core::TrustLevel::Unverified,
            "New field should be Unverified"
        );
        assert_eq!(status.count, 0, "Should have zero validations");
        assert!(
            !status.validated_by_me,
            "Current user should not have validated"
        );
    }

    /// Trace: field_validation.feature - Validate a contact's field
    #[test]
    fn test_validate_field_creates_validation() {
        let (mut backend, _temp) = create_test_backend();
        backend.create_identity("Alice").unwrap();
        let bob_id = create_and_save_test_contact(&backend, "Bob");

        let result = backend.validate_field(&bob_id, "twitter", "@bob");
        assert!(
            result.is_ok(),
            "validate_field should succeed: {:?}",
            result
        );

        let status = backend
            .get_field_validation_status(&bob_id, "twitter", "@bob")
            .unwrap();

        assert!(
            status.count >= 1,
            "Should have at least 1 validation after validate_field"
        );
        assert!(
            status.validated_by_me,
            "Current user should show as having validated"
        );
    }

    /// Trace: field_validation.feature - Revoke validation
    #[test]
    fn test_revoke_field_validation_removes_validation() {
        let (mut backend, _temp) = create_test_backend();
        backend.create_identity("Alice").unwrap();
        let bob_id = create_and_save_test_contact(&backend, "Bob");

        // Validate first
        backend.validate_field(&bob_id, "twitter", "@bob").unwrap();

        // Then revoke
        let revoked = backend.revoke_field_validation(&bob_id, "twitter").unwrap();
        assert!(
            revoked,
            "Should return true when revoking existing validation"
        );

        let status = backend
            .get_field_validation_status(&bob_id, "twitter", "@bob")
            .unwrap();

        assert_eq!(status.count, 0, "Should have zero validations after revoke");
        assert!(
            !status.validated_by_me,
            "Current user should not show as having validated after revoke"
        );
    }

    /// Trace: field_validation.feature - Revoke when no validation exists returns false
    #[test]
    fn test_revoke_field_validation_returns_false_when_none_exists() {
        let (mut backend, _temp) = create_test_backend();
        backend.create_identity("Alice").unwrap();
        let bob_id = create_and_save_test_contact(&backend, "Bob");

        let revoked = backend.revoke_field_validation(&bob_id, "twitter").unwrap();
        assert!(
            !revoked,
            "Should return false when no validation exists to revoke"
        );
    }

    /// Trace: field_validation.feature - Cannot validate own field
    #[test]
    fn test_validate_field_rejects_self_validation() {
        let (mut backend, _temp) = create_test_backend();
        backend.create_identity("Alice").unwrap();

        // Get own identity ID (full hex-encoded signing public key)
        let my_id = hex::encode(
            backend
                .identity
                .as_ref()
                .expect("Should have identity")
                .signing_public_key(),
        );

        let result = backend.validate_field(&my_id, "twitter", "@alice");
        assert!(result.is_err(), "Should not be able to validate own field");
    }
}
