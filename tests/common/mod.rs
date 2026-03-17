// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Shared helpers for TUI snapshot tests.
#![allow(dead_code)] // Not all test binaries use every helper

use std::sync::Once;

use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;
use tempfile::TempDir;

use vauchi_core::ui::AppEngine;
use vauchi_core::{
    Contact, ContactCard, ContactField, FieldType, SymmetricKey, Vauchi, VauchiConfig,
};
use vauchi_tui::app::App;

/// Fixed terminal dimensions for consistent snapshots.
pub const WIDTH: u16 = 80;
pub const HEIGHT: u16 = 24;

/// Ensure locale data is loaded before snapshot tests run.
/// Searches for locale JSON files in standard locations (workspace locales repo,
/// core's bundled copy, or VAUCHI_LOCALES_DIR env var).
static INIT_LOCALES: Once = Once::new();

pub fn ensure_locales_loaded() {
    INIT_LOCALES.call_once(|| {
        let candidates = [
            std::env::var("VAUCHI_LOCALES_DIR").ok(),
            Some("../locales".to_string()),
            Some("../core/vauchi-core/locales".to_string()),
        ];
        for candidate in candidates.iter().flatten() {
            let path = std::path::Path::new(candidate);
            if path.join("en.json").exists() && vauchi_core::i18n::init(path).is_ok() {
                return;
            }
        }
        eprintln!("WARNING: Could not find locale files — snapshots may use fallback strings");
    });
}

pub fn create_app_engine(data_dir: &std::path::Path) -> AppEngine {
    let key = SymmetricKey::generate();
    let config = VauchiConfig {
        storage_path: data_dir.join("vauchi.db"),
        storage_key: Some(key),
        ..Default::default()
    };
    let vauchi: Vauchi = Vauchi::new(config).expect("vauchi");
    AppEngine::new(vauchi)
}

/// Create a test app with an identity and some card fields.
pub fn create_app_with_identity() -> (App, TempDir) {
    ensure_locales_loaded();
    let temp_dir = TempDir::new().expect("temp dir");
    let mut app_engine = create_app_engine(temp_dir.path());
    app_engine
        .vauchi_mut()
        .create_identity("Alice Smith")
        .expect("create identity");
    app_engine
        .vauchi()
        .add_own_field(ContactField::new(
            FieldType::Email,
            "Work",
            "alice@company.com",
        ))
        .expect("add email");
    app_engine
        .vauchi()
        .add_own_field(ContactField::new(
            FieldType::Phone,
            "Mobile",
            "+41 79 123 45 67",
        ))
        .expect("add phone");
    app_engine
        .vauchi()
        .add_own_field(ContactField::new(
            FieldType::Website,
            "Blog",
            "https://alice.example.com",
        ))
        .expect("add website");
    let app = App::new(
        app_engine,
        "wss://relay.vauchi.app".to_string(),
        temp_dir.path().to_path_buf(),
    );
    (app, temp_dir)
}

/// Create a test app without an identity (setup state).
pub fn create_app_without_identity() -> (App, TempDir) {
    ensure_locales_loaded();
    let temp_dir = TempDir::new().expect("temp dir");
    let app_engine = create_app_engine(temp_dir.path());
    let app = App::new(
        app_engine,
        "wss://relay.vauchi.app".to_string(),
        temp_dir.path().to_path_buf(),
    );
    (app, temp_dir)
}

/// Render the app to a terminal buffer and extract text content.
/// Non-deterministic values (hex IDs, keys) are redacted for reproducibility.
pub fn render_to_string(app: &mut App) -> String {
    let backend = TestBackend::new(WIDTH, HEIGHT);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|f| vauchi_tui::ui::draw(f, app))
        .expect("draw");
    let raw = buffer_to_string(terminal.backend().buffer());
    redact_dynamic_values(&raw)
}

/// Replace hex-based IDs and keys with stable placeholders.
pub fn redact_dynamic_values(s: &str) -> String {
    // Replace public IDs: 16+ hex chars (optionally followed by "...")
    let re_public_id = regex::Regex::new(r"[0-9a-f]{16,}(\.\.\.)?").unwrap();
    let s = re_public_id.replace_all(s, "[PUBLIC_ID]...");
    // Replace truncated device keys: 8 hex chars followed by "..."  e.g. "(83c49a52...)"
    let re_device_key = regex::Regex::new(r"\([0-9a-f]{8}\.\.\.\)").unwrap();
    let s = re_device_key.replace_all(&s, "([DEVICE_KEY]...)");
    // Replace standalone 16-char hex sequences
    let re_hex16 = regex::Regex::new(r"\b[0-9a-f]{16}\b").unwrap();
    let s = re_hex16.replace_all(&s, "[HEX_ID]").to_string();
    // Redact QR code data lines (must contain at least one block char) — crypto nonces make these non-deterministic
    let re_qr = regex::Regex::new(r"(?m)^(│\s+)[\s█]*█[\s█]*(\s+│)$").unwrap();
    re_qr.replace_all(&s, "$1[QR_DATA]$2").to_string()
}

/// Assert a snapshot with navigation context metadata.
///
/// `from`: the previous screen, `trigger`: the user action or event that caused this screen.
#[macro_export]
macro_rules! assert_snap {
    ($name:expr, $from:expr, $trigger:expr, $output:expr) => {
        insta::with_settings!({
            description => concat!($from, " → ", $trigger),
        }, {
            insta::assert_snapshot!($name, $output);
        });
    };
}

/// Render a step in a workflow snapshot with a header separator.
pub fn workflow_step(step: usize, label: &str, app: &mut App) -> String {
    let output = render_to_string(app);
    format!(
        "═══ Step {} ─ {} {}\n{}",
        step,
        label,
        "═".repeat(60usize.saturating_sub(label.len() + 12)),
        output
    )
}

/// Convert a ratatui buffer to a plain text string.
fn buffer_to_string(buffer: &Buffer) -> String {
    let mut lines = Vec::new();
    for y in 0..buffer.area.height {
        let mut line = String::new();
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            line.push_str(cell.symbol());
        }
        // Trim trailing whitespace per line
        lines.push(line.trim_end().to_string());
    }
    // Remove trailing empty lines
    while lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

/// Seed N fake contacts into a Vauchi instance with groups.
pub fn seed_contacts(vauchi: &Vauchi, count: usize) {
    // Create groups
    let family = vauchi.create_group("Family").expect("create group");
    let friends = vauchi.create_group("Friends").expect("create group");
    let work = vauchi.create_group("Work").expect("create group");
    let group_ids = [
        family.id().to_string(),
        friends.id().to_string(),
        work.id().to_string(),
    ];

    let names = [
        "Abigale Schroeder",
        "Ahmed Nikolaus",
        "Andreanne Doyle",
        "Andres Raynor",
        "Augusta Heller",
        "Brady Koss",
        "Carmen Gutierrez",
        "Diana Patel",
        "Eduardo Reyes",
        "Fatima Okonkwo",
        "Gerhard Mueller",
        "Hiroshi Tanaka",
        "Ingrid Svensson",
        "Jorge Castillo",
        "Keiko Watanabe",
    ];

    for (i, name) in names.iter().take(count).enumerate() {
        let area = 200 + (i * 3) % 800;
        let num1 = 100 + (i * 7) % 900;
        let num2 = 1000 + (i * 13) % 9000;
        let phone = format!("+1-{}-{}-{}", area, num1, num2);
        let email = format!("{}@example.com", name.to_lowercase().replace(' ', "."));

        let mut card = ContactCard::new(name);
        card.add_field(ContactField::new(FieldType::Phone, "Mobile", &phone))
            .expect("add phone");
        card.add_field(ContactField::new(FieldType::Email, "Email", &email))
            .expect("add email");

        // Address for ~40%
        if i % 5 < 2 {
            card.add_field(ContactField::new(
                FieldType::Address,
                "Address",
                &format!("{} Main St, Springfield", 100 + i * 10),
            ))
            .expect("add address");
        }

        let mut pk = [0u8; 32];
        pk[0] = (i >> 8) as u8;
        pk[1] = (i & 0xFF) as u8;
        for (j, byte) in pk[2..].iter_mut().enumerate() {
            *byte = ((i * 7 + j) & 0xFF) as u8;
        }

        let contact = Contact::from_exchange(pk, card, SymmetricKey::generate());
        let cid = contact.id().to_string();
        vauchi.add_contact(contact).expect("add contact");

        // Assign to group: ~30% Family, ~40% Friends, ~30% Work
        let gid = match i % 10 {
            0..=2 => &group_ids[0],
            3..=6 => &group_ids[1],
            _ => &group_ids[2],
        };
        vauchi
            .add_contact_to_group(gid, &cid)
            .expect("add to group");
    }
}

/// Create a test app with identity and seeded contacts.
pub fn create_app_with_contacts(count: usize) -> (App, TempDir) {
    ensure_locales_loaded();
    let temp_dir = TempDir::new().expect("temp dir");
    let mut app_engine = create_app_engine(temp_dir.path());
    app_engine
        .vauchi_mut()
        .create_identity("Alice Smith")
        .expect("create identity");
    app_engine
        .vauchi()
        .add_own_field(ContactField::new(
            FieldType::Email,
            "Work",
            "alice@company.com",
        ))
        .expect("add email");

    seed_contacts(app_engine.vauchi(), count);

    let app = App::new(
        app_engine,
        "wss://relay.vauchi.app".to_string(),
        temp_dir.path().to_path_buf(),
    );
    (app, temp_dir)
}
