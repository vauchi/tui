// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

pub mod app;
pub mod handlers;
pub mod i18n;
pub mod sync_service;
pub mod theme;
pub mod ui;

use vauchi_core::VauchiConfig;

/// Apply the e2e test-only OHTTP overrides to a config, mirroring the CLI
/// (`cli/src/commands/common.rs`). `VAUCHI_OHTTP_RELAY_URL` sets the OHTTP
/// route; `VAUCHI_OVERRIDE_BUNDLED_OHTTP_KEY_HEX` injects a locally-spawned
/// relay's ephemeral gateway key so the TUI can encap to a key that relay can
/// decrypt. Both are test/dev-only and WARN-loud — production must never set
/// them. The key override changes only which bytes are used; it does not
/// enable direct fetch, so ADR-037 holds. Without this the TUI carries the
/// compiled-in production key and cannot reach a local test relay (see the
/// backlog record 2026-09-10-tui-link-initiator-does-not-complete-handshake).
pub fn apply_ohttp_test_overrides(mut config: VauchiConfig) -> VauchiConfig {
    if let Ok(url) = std::env::var("VAUCHI_OHTTP_RELAY_URL")
        && !url.trim().is_empty()
    {
        config = config.with_ohttp_relay_url(url.trim());
    }
    if let Ok(hex) = std::env::var("VAUCHI_OVERRIDE_BUNDLED_OHTTP_KEY_HEX") {
        match hex::decode(hex.trim()) {
            Ok(bytes) => {
                eprintln!(
                    "OHTTP bundled key overridden via \
                     VAUCHI_OVERRIDE_BUNDLED_OHTTP_KEY_HEX ({} bytes) — \
                     must NOT be set in production",
                    bytes.len()
                );
                config.ohttp.bundled_gateway_key = Some(bytes);
            }
            Err(e) => {
                eprintln!("VAUCHI_OVERRIDE_BUNDLED_OHTTP_KEY_HEX is not valid hex: {e}");
            }
        }
    }
    config
}
