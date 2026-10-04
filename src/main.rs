// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Vauchi Terminal UI
//!
//! Interactive terminal application for Vauchi using Ratatui.

use std::io::{self, IsTerminal};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;

use vauchi_app::ui::AppEngine;
use vauchi_core::crypto::SymmetricKey;
#[cfg(feature = "secure-storage")]
use vauchi_core::storage::secure::{PlatformKeyring, SecureStorage};
use vauchi_core::{Vauchi, VauchiConfig, VauchiError};

#[cfg(not(feature = "secure-storage"))]
use vauchi_core::storage::secure::{FileKeyStorage, SecureStorage};

use vauchi_tui::app::App;
use vauchi_tui::handlers;
use vauchi_tui::i18n;
use vauchi_tui::startup;
use vauchi_tui::ui;

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

    /// Render every screen of a Core screen catalog to .snap frames and exit
    #[arg(long, num_args = 2..=4, value_names = ["CATALOG", "OUT_DIR", "COLS", "ROWS"])]
    render_catalog: Option<Vec<String>>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Catalog strings arrive already localized by Core, so this needs no
    // locale files, data directory, or terminal.
    if let Some(args) = &cli.render_catalog {
        return vauchi_tui::ui::screen_catalog::run_cli(args);
    }

    // Load runtime locale files before any UI string is rendered; without
    // this every string falls back to core's 2-key bundled set.
    let locale_source = i18n::init_from_environment();
    if !vauchi_app::i18n::is_initialized() {
        match locale_source.path() {
            Some(dir) => eprintln!(
                "vauchi-tui: failed to load locale files from {}; using built-in English strings.",
                dir.display()
            ),
            None => eprintln!(
                "vauchi-tui: no locale directory found; set VAUCHI_LOCALES_DIR to a locale JSON directory."
            ),
        }
    }

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
    let vauchi_config =
        VauchiConfig::with_storage_path(data_dir.join("vauchi.db")).with_storage_key(storage_key);
    let relay_url = cli
        .relay_url
        .clone()
        .unwrap_or_else(|| resolve_relay_url(&data_dir));
    let vauchi_config = vauchi_config.with_relay_url(&relay_url);
    let vauchi_config = vauchi_tui::apply_ohttp_test_overrides(vauchi_config);
    let mut vauchi: Vauchi = match Vauchi::new(vauchi_config) {
        Ok(vauchi) => vauchi,
        Err(err) => {
            eprintln!("Error: {err}");
            if err.is_unreadable_storage() {
                print_storage_recovery_hint(&data_dir);
            }
            std::process::exit(1);
        }
    };

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
        vauchi_tui::demo_seed::seed_demo_data(&mut vauchi);
    }

    // Acquire an exclusive lock to prevent concurrent instances on the same data.
    // Uses flock on a dedicated lock file — the OS auto-releases on crash.
    let lock_path = data_dir.join(".vauchi.lock");
    let lock_file = std::fs::File::create(&lock_path).context("Failed to create lock file")?;
    #[cfg(unix)]
    if lock_file.try_lock().is_err() {
        eprintln!("Another vauchi-tui instance is already running on this data directory.");
        eprintln!("Data dir: {}", data_dir.display());
        std::process::exit(1);
    }
    // Keep lock_file alive for the duration of the process (dropped on exit/crash)
    let _lock = lock_file;

    let relay_url = cli
        .relay_url
        .unwrap_or_else(|| resolve_relay_url(&data_dir));
    let mut app_engine = AppEngine::new(vauchi);
    app_engine.set_device_capabilities(vauchi_tui::app::tui_device_capabilities());

    // Setup terminal (after init — no stray eprintln output in alternate screen)
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the app, capture result so cleanup always runs
    let data_dir_for_recovery_hint = data_dir.clone();
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

    if let Err(err) = res {
        eprintln!("Error: {err:?}");
        // Core owns the error classification (ADR-045 Amendment 1); the
        // TUI owns only this native stderr presentation.
        if err.chain().any(|cause| {
            cause
                .downcast_ref::<VauchiError>()
                .is_some_and(VauchiError::is_unreadable_storage)
        }) {
            print_storage_recovery_hint(&data_dir_for_recovery_hint);
        }
    }

    Ok(())
}

fn print_storage_recovery_hint(data_dir: &Path) {
    eprintln!();
    eprintln!("The storage appears corrupted or was encrypted with a different key.");
    eprintln!("To start fresh, delete the data directory:");
    eprintln!();
    eprintln!("  rm -rf {}", data_dir.display());
}

/// Relay URL: `<data_dir>/relay_url.txt`, then `VAUCHI_RELAY_URL`, then
/// the default.
fn resolve_relay_url(data_dir: &Path) -> String {
    startup::resolve_relay_url(
        std::fs::read_to_string(data_dir.join("relay_url.txt")).ok(),
        std::env::var("VAUCHI_RELAY_URL").ok(),
    )
}

/// Derives a stable per-install keychain key name from the install_id stored
/// at `<data_dir>/install_id`.
///
/// The install_id moves with the data directory on rename — so the OS keychain
/// entry stays reachable even if the user relocates `data_dir`. Two installs
/// with distinct data directories get distinct ids and distinct keychain
/// entries.
#[cfg_attr(not(feature = "secure-storage"), allow(dead_code))]
fn keychain_key_name(data_dir: &Path) -> Result<String> {
    let install_id = vauchi_core::install_id::read_or_create_install_id(data_dir)?;
    Ok(format!("storage_key_{install_id}"))
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
/// When the `secure-storage` feature is enabled, uses the OS keychain with a
/// key name derived from the install_id stored next to the data directory.
/// Otherwise, falls back to encrypted file storage.
///
/// Mirrors `cli::config::CliConfig::storage_key` (`cli/src/config.rs`):
/// core exposes the `SecureStorage`/`PlatformKeyring`/`FileKeyStorage`
/// primitives but has no higher-level open-or-create entry point, since
/// only the shell knows its own keychain service name. Both Rust-native
/// shells resolve the key this way; there is no domain-shaped API to
/// delegate to instead (2026-07-06-desktop-tui-web-domain-shell-violations
/// U22, superseded by ADR-066).
fn load_or_create_storage_key(data_dir: &Path) -> Result<SymmetricKey> {
    #[cfg(feature = "secure-storage")]
    {
        storage_key_from_keychain(data_dir)
    }
    #[cfg(not(feature = "secure-storage"))]
    {
        storage_key_from_file(data_dir)
    }
}

#[cfg(feature = "secure-storage")]
fn storage_key_from_keychain(data_dir: &Path) -> Result<SymmetricKey> {
    let storage = PlatformKeyring::new("vauchi-tui");
    let key_name = keychain_key_name(data_dir)?;

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

/// Key name for non-keychain (file-based) storage.
#[cfg(not(feature = "secure-storage"))]
const FILE_STORAGE_KEY_NAME: &str = "storage_key";

#[cfg(not(feature = "secure-storage"))]
fn storage_key_from_file(data_dir: &Path) -> Result<SymmetricKey> {
    let fallback_key = load_or_generate_fallback_key(data_dir)?;
    let storage = FileKeyStorage::new(data_dir.join("keys"), fallback_key);

    match storage.load_key(FILE_STORAGE_KEY_NAME) {
        Ok(Some(bytes)) if bytes.len() == 32 => {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            Ok(SymmetricKey::from_bytes(arr))
        }
        Ok(Some(_)) => {
            anyhow::bail!("Invalid storage key length");
        }
        Ok(None) => {
            let key = SymmetricKey::generate();
            storage
                .save_key(FILE_STORAGE_KEY_NAME, key.as_bytes())
                .map_err(|e| anyhow::anyhow!("Failed to save storage key: {}", e))?;
            Ok(key)
        }
        Err(e) => {
            anyhow::bail!("Storage error: {}", e);
        }
    }
}

/// Restore terminal to normal mode. Called from panic hook and normal exit.
fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
}

fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    // Bootstrap the core-driven wakeup loop (ADR-044 Am2a). The first call
    // runs any due work and emits the initial `Command::ScheduleWakeup`.
    app.tick_notifications();
    let mut reported_size = (0, 0);

    loop {
        let area = terminal.size()?;
        let current_size = (u32::from(area.width), u32::from(area.height));
        if current_size != reported_size {
            reported_size = current_size;
            app.report_presentation_environment(current_size.0, current_size.1);
        }
        terminal.draw(|f| ui::draw_presentation(f, app))?;

        // Periodic maintenance (ADR-031)
        app.tick_status();

        // Poll background sync result channel (non-blocking).
        if let Some(rx) = &app.sync_rx
            && let Ok(result) = rx.try_recv()
        {
            app.sync_rx = None;
            app.apply_sync_result(result);
        }

        // ADR-044 Am2a: Core's wakeup schedule caps the wait, so `on_wakeup()`
        // runs when it is due; a delayed or coalesced wake is safe because
        // `on_wakeup` is elapsed-based and idempotent.
        let poll_timeout = startup::poll_timeout(
            app.status_is_flashing(),
            app.sync_rx.is_some(),
            app.next_wakeup,
            std::time::Instant::now(),
        );
        let event_ready = event::poll(poll_timeout)?;
        if startup::wakeup_due(event_ready, app.next_wakeup, std::time::Instant::now()) {
            app.tick_notifications();
        }

        if event_ready
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match handlers::handle_presentation_key(app, key) {
                handlers::Action::Quit => return Ok(()),
                handlers::Action::Continue => {}
            }
        }
    }
}

// INLINE_TEST_REQUIRED: keychain_key_name is a private helper of the binary;
// exposing it via tui's lib.rs purely for tests would over-widen the API.
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // @internal
    #[test]
    fn keychain_key_name_is_stable_across_calls() {
        let temp_dir = tempdir().unwrap();
        let name1 = keychain_key_name(temp_dir.path()).unwrap();
        let name2 = keychain_key_name(temp_dir.path()).unwrap();
        assert_eq!(name1, name2);
    }

    // @internal
    #[test]
    fn keychain_key_name_survives_data_dir_rename() {
        // Regression: pre-fix, the key name was derived from `fnv1a(data_dir)`,
        // so renaming the data directory orphaned the OS keychain entry. Now
        // the name is derived from the install_id file, which moves with the
        // data — rename is invisible to the keychain lookup.
        let parent = tempdir().unwrap();
        let original = parent.path().join("original");
        std::fs::create_dir_all(&original).unwrap();
        let name_before = keychain_key_name(&original).unwrap();

        let renamed = parent.path().join("renamed");
        std::fs::rename(&original, &renamed).unwrap();
        let name_after = keychain_key_name(&renamed).unwrap();

        assert_eq!(name_before, name_after);
    }

    // @internal
    #[test]
    fn keychain_key_name_differs_per_data_dir() {
        let dir_a = tempdir().unwrap();
        let dir_b = tempdir().unwrap();
        let name_a = keychain_key_name(dir_a.path()).unwrap();
        let name_b = keychain_key_name(dir_b.path()).unwrap();
        assert_ne!(name_a, name_b);
    }

    #[cfg(not(feature = "secure-storage"))]
    // @internal
    #[test]
    fn the_fallback_key_is_created_once_and_reread() {
        let dir = tempdir().unwrap();

        let first = load_or_generate_fallback_key(dir.path()).unwrap();
        let second = load_or_generate_fallback_key(dir.path()).unwrap();

        assert_eq!(first.as_bytes(), second.as_bytes());
        assert_eq!(
            std::fs::read(dir.path().join(".fallback-key"))
                .unwrap()
                .len(),
            32
        );
    }

    #[cfg(not(feature = "secure-storage"))]
    // @internal
    #[test]
    fn a_fallback_key_of_the_wrong_length_is_refused() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join(".fallback-key"), [7u8; 31]).unwrap();

        let error = load_or_generate_fallback_key(dir.path())
            .unwrap_err()
            .to_string();

        assert!(
            error.contains("Invalid fallback key length (31)"),
            "{error}"
        );
    }

    #[cfg(not(feature = "secure-storage"))]
    // @internal
    #[test]
    fn the_storage_key_is_created_once_and_reread() {
        let dir = tempdir().unwrap();

        let first = load_or_create_storage_key(dir.path()).unwrap();
        let second = load_or_create_storage_key(dir.path()).unwrap();

        assert_eq!(first.as_bytes(), second.as_bytes());
    }

    #[cfg(not(feature = "secure-storage"))]
    // @internal
    #[test]
    fn a_stored_key_of_the_wrong_length_is_refused() {
        let dir = tempdir().unwrap();
        let fallback = load_or_generate_fallback_key(dir.path()).unwrap();
        FileKeyStorage::new(dir.path().join("keys"), fallback)
            .save_key(FILE_STORAGE_KEY_NAME, &[7u8; 31])
            .unwrap();

        let error = load_or_create_storage_key(dir.path())
            .unwrap_err()
            .to_string();

        assert!(error.contains("Invalid storage key length"), "{error}");
    }
}
