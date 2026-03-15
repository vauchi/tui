// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Vauchi Terminal UI
//!
//! Interactive terminal application for Vauchi using Ratatui.

use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

use vauchi_core::crypto::SymmetricKey;
#[cfg(feature = "secure-storage")]
use vauchi_core::storage::secure::{PlatformKeyring, SecureStorage};
use vauchi_core::ui::AppEngine;
use vauchi_core::{Vauchi, VauchiConfig};

#[cfg(not(feature = "secure-storage"))]
use vauchi_core::storage::secure::{FileKeyStorage, SecureStorage};

use vauchi_tui::app::App;
use vauchi_tui::handlers;
use vauchi_tui::ui;

/// Default relay URL.
const DEFAULT_RELAY_URL: &str = "wss://relay.vauchi.app";

/// Vauchi — privacy-focused contact card exchange.
///
/// Interactive terminal application for managing encrypted contact cards.
/// Data is end-to-end encrypted and stored locally.
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Data directory path [env: VAUCHI_DATA_DIR]
    #[arg(long, value_name = "PATH")]
    data_dir: Option<PathBuf>,

    /// Relay server URL [env: VAUCHI_RELAY_URL]
    #[arg(long, value_name = "URL")]
    relay_url: Option<String>,

    /// Seed demo data on first run
    #[arg(long)]
    seed: bool,

    /// Validate data integrity and exit
    #[arg(long)]
    check: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Require an interactive terminal for TUI mode
    if !cli.check && !io::stdin().is_terminal() {
        eprintln!("vauchi-tui requires an interactive terminal.");
        eprintln!("Run with --help for usage information.");
        std::process::exit(1);
    }

    // Install panic hook that restores the terminal before printing the panic.
    // Without this, a panic leaves the terminal in raw/alternate-screen mode.
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        restore_terminal();
        original_hook(panic_info);
    }));

    // Resolve data directory: CLI flag > env var > platform default
    let data_dir = cli
        .data_dir
        .or_else(|| std::env::var("VAUCHI_DATA_DIR").ok().map(PathBuf::from))
        .unwrap_or_else(|| {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("vauchi")
        });
    let data_dir_is_new = !data_dir.exists();
    std::fs::create_dir_all(&data_dir).context("Failed to create data directory")?;

    // Restrict data directory to owner-only (0700) on creation.
    // Prevents other users from reading database, keys, or WAL files.
    #[cfg(unix)]
    if data_dir_is_new {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&data_dir, std::fs::Permissions::from_mode(0o700));
    }

    let storage_key = load_or_create_storage_key(&data_dir)?;
    let vauchi_config = VauchiConfig {
        storage_path: data_dir.join("vauchi.db"),
        storage_key: Some(storage_key),
        ..Default::default()
    };
    let relay_url = cli
        .relay_url
        .clone()
        .unwrap_or_else(|| resolve_relay_url(&data_dir));
    let vauchi_config = vauchi_config.with_relay_url(&relay_url);
    let mut vauchi: Vauchi = Vauchi::new(vauchi_config)?;

    // --check: validate data integrity and exit
    if cli.check {
        println!("Data directory: {}", data_dir.display());
        println!("Database: OK (opened successfully)");
        println!(
            "Identity: {}",
            if vauchi.has_identity() {
                "present"
            } else {
                "none"
            }
        );
        return Ok(());
    }

    // Seed with demo data if --seed or VAUCHI_SEED=1 and no identity exists yet
    if (cli.seed || std::env::var("VAUCHI_SEED").is_ok()) && !vauchi.has_identity() {
        seed_demo_data(&mut vauchi);
    }

    // Acquire an exclusive lock to prevent concurrent instances on the same data.
    // Uses flock on a dedicated lock file — the OS auto-releases on crash.
    let lock_path = data_dir.join(".vauchi.lock");
    let lock_file = std::fs::File::create(&lock_path).context("Failed to create lock file")?;
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        let fd = lock_file.as_raw_fd();
        // Try non-blocking exclusive lock
        let ret = unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) };
        if ret != 0 {
            eprintln!("Another vauchi-tui instance is already running on this data directory.");
            eprintln!("Data dir: {}", data_dir.display());
            std::process::exit(1);
        }
    }
    // Keep lock_file alive for the duration of the process (dropped on exit/crash)
    let _lock = lock_file;

    let relay_url = cli
        .relay_url
        .unwrap_or_else(|| resolve_relay_url(&data_dir));
    let app_engine = AppEngine::new(vauchi);

    // Setup terminal (after init — no stray eprintln output in alternate screen)
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the app, capture result so cleanup always runs
    let mut app = App::new(app_engine, relay_url, data_dir);
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal (always runs, even after errors)
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    // Handle any errors
    if let Err(err) = res {
        let msg = format!("{err:?}");
        eprintln!("Error: {msg}");
        if msg.contains("ecryption") || msg.contains("corrupted") || msg.contains("wrong key") {
            let data_dir = std::env::var("VAUCHI_DATA_DIR")
                .ok()
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    dirs::data_dir()
                        .unwrap_or_else(|| PathBuf::from("."))
                        .join("vauchi")
                });
            eprintln!();
            eprintln!("The storage appears corrupted or was encrypted with a different key.");
            eprintln!("To start fresh, delete the data directory:");
            eprintln!();
            eprintln!("  rm -rf {}", data_dir.display());
        }
    }

    Ok(())
}

/// Resolve relay URL with fallback hierarchy:
/// 1. User-configured URL (stored in config file)
/// 2. VAUCHI_RELAY_URL environment variable
/// 3. Default: wss://relay.vauchi.app
fn resolve_relay_url(data_dir: &Path) -> String {
    let relay_config_path = data_dir.join("relay_url.txt");
    std::fs::read_to_string(&relay_config_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            std::env::var("VAUCHI_RELAY_URL")
                .ok()
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| DEFAULT_RELAY_URL.to_string())
}

/// Derives a per-data-dir keychain key name using FNV-1a hash.
///
/// Ensures that TUI instances with different `--data-dir` values get
/// separate OS keychain entries, preventing key collisions (T-M4).
#[cfg(feature = "secure-storage")]
fn keychain_key_name(data_dir: &Path) -> String {
    let path_str = data_dir.to_string_lossy();
    // FNV-1a hash — stable, well-defined algorithm (matches CLI implementation)
    let mut hash: u64 = 0xcbf29ce484222325; // FNV offset basis
    for byte in path_str.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3); // FNV prime
    }
    format!("storage_key_{:016x}", hash)
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

/// Loads or creates the storage encryption key using SecureStorage.
///
/// When the `secure-storage` feature is enabled, uses the OS keychain
/// with a per-data-dir key name (FNV-1a hash of the path) to prevent
/// collisions between TUI instances with different `--data-dir` values.
/// Migrates from the legacy fixed `"storage_key"` entry if present.
///
/// Otherwise, falls back to encrypted file storage.
#[allow(unused_variables)]
fn load_or_create_storage_key(data_dir: &Path) -> Result<SymmetricKey> {
    /// Key name for non-keychain (file-based) storage.
    const KEY_NAME: &str = "storage_key";

    /// Legacy fixed keychain key name (pre per-data-dir fix).
    #[cfg(feature = "secure-storage")]
    const LEGACY_KEY_NAME: &str = "storage_key";

    #[cfg(feature = "secure-storage")]
    {
        let storage = PlatformKeyring::new("vauchi-tui");
        let key_name = keychain_key_name(data_dir);

        match storage.load_key(&key_name) {
            Ok(Some(bytes)) if bytes.len() == 32 => {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                Ok(SymmetricKey::from_bytes(arr))
            }
            Ok(Some(_)) => {
                anyhow::bail!("Invalid storage key length in keychain");
            }
            Ok(None) => {
                // Try migrating from the old fixed key name
                if let Ok(Some(legacy_bytes)) = storage.load_key(LEGACY_KEY_NAME) {
                    if legacy_bytes.len() == 32 {
                        // Migrate: save under new per-dir name, delete legacy entry
                        storage.save_key(&key_name, &legacy_bytes).map_err(|e| {
                            anyhow::anyhow!("Failed to migrate keychain key: {}", e)
                        })?;
                        let _ = storage.delete_key(LEGACY_KEY_NAME);
                        let mut arr = [0u8; 32];
                        arr.copy_from_slice(&legacy_bytes);
                        return Ok(SymmetricKey::from_bytes(arr));
                    }
                }

                // No existing key — generate a new one
                let key = SymmetricKey::generate();
                storage
                    .save_key(&key_name, key.as_bytes())
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

/// Seeds the Vauchi instance with demo data for local testing.
///
/// Creates an identity, adds fields, creates groups, and adds fake contacts.
/// Only runs when VAUCHI_SEED=1 and no identity exists yet.
fn seed_demo_data(vauchi: &mut Vauchi) {
    use vauchi_core::contact::Contact;
    use vauchi_core::contact_card::{ContactCard, ContactField, FieldType};
    use vauchi_core::crypto::SymmetricKey;

    // Create own identity
    if vauchi.create_identity("Demo User").is_err() {
        return;
    }

    // Add own fields
    let own_fields = [
        (FieldType::Phone, "Mobile", "+41 79 123 45 67"),
        (FieldType::Email, "Work", "demo@vauchi.app"),
        (FieldType::Website, "Website", "https://vauchi.app"),
    ];
    for (ft, label, value) in own_fields {
        let _ = vauchi.add_own_field(ContactField::new(ft, label, value));
    }

    // Create groups
    let family = vauchi.create_group("Family").ok();
    let friends = vauchi.create_group("Friends").ok();
    let work = vauchi.create_group("Work").ok();

    // 57 demo contact names
    let names = [
        "Alice", "Bob", "Charlie", "Diana", "Eve", "Frank", "Grace", "Hank", "Ivy", "Jack",
        "Karen", "Leo", "Mia", "Noah", "Olivia", "Paul", "Quinn", "Rosa", "Sam", "Tina", "Uma",
        "Victor", "Wendy", "Xavier", "Yuki", "Zara", "Amber", "Brian", "Clara", "David", "Elena",
        "Felix", "Gina", "Hugo", "Iris", "James", "Kira", "Liam", "Maya", "Nora", "Oscar", "Petra",
        "Rafael", "Sofia", "Theo", "Ursula", "Vera", "Walter", "Xena", "Yara", "Zoe", "Aria",
        "Blake", "Cleo", "Dario", "Elsa", "Finn",
    ];

    // Field templates — each contact gets (i % 6 + 1) fields
    let field_templates: &[(FieldType, &str, &str)] = &[
        (FieldType::Phone, "Mobile", "+41 79 {} 00"),
        (FieldType::Email, "Personal", "{}@example.com"),
        (FieldType::Phone, "Work", "+41 44 {} 00"),
        (FieldType::Website, "Website", "https://{}.dev"),
        (FieldType::Email, "Work", "{}@corp.ch"),
        (FieldType::Phone, "Home", "+41 31 {} 00"),
    ];

    let groups = [&family, &friends, &work];

    for (i, name) in names.iter().enumerate() {
        let mut card = ContactCard::new(name);
        let num_fields = (i % 6) + 1;
        for item in field_templates.iter().take(num_fields) {
            let (ref ft, label, template) = *item;
            let value = template.replace("{}", &name.to_lowercase());
            let _ = card.add_field(ContactField::new(ft.clone(), label, &value));
        }

        let shared_key = SymmetricKey::generate();
        let extra_key = SymmetricKey::generate();
        let pubkey: [u8; 32] = *extra_key.as_bytes();
        let contact = Contact::from_exchange(pubkey, card, shared_key);
        let contact_id = contact.id().to_string();
        if vauchi.add_contact(contact).is_err() {
            continue;
        }

        // Assign to groups round-robin
        if let Some(ref g) = groups[i % 3] {
            let _ = vauchi.add_contact_to_group(g.id(), &contact_id);
        }
    }
}

/// Restore terminal to normal mode. Called from panic hook and normal exit.
fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        // Use short poll timeout when status message is flashing (500ms window),
        // otherwise use longer timeout to minimize idle CPU usage.
        let has_active_flash = app
            .status_message_time
            .map(|t| t.elapsed() < Duration::from_millis(600))
            .unwrap_or(false);
        let poll_timeout = if has_active_flash {
            Duration::from_millis(100)
        } else {
            Duration::from_secs(1)
        };

        if event::poll(poll_timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match handlers::handle_key(app, key.code) {
                        handlers::Action::Quit => return Ok(()),
                        handlers::Action::Continue => {}
                    }
                }
            }
        }
    }
}
