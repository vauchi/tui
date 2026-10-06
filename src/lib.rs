// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

pub mod app;
pub mod demo_seed;
pub mod handlers;
pub mod i18n;
pub mod startup;
pub mod sync_service;
pub mod theme;
pub mod ui;

use vauchi_core::VauchiConfig;

/// Apply the relay overrides to a config, mirroring the CLI
/// (`cli/src/commands/common.rs`). `VAUCHI_OHTTP_RELAY_URL` sets the OHTTP
/// route (e2e/dev). `relay_anchor` — the `--relay-anchor` flag, else
/// `VAUCHI_RELAY_ANCHOR` — is the relay's OHTTP trust anchor (#288): its
/// gateway keys are then accepted only through its signed chain. No key is
/// compiled in, so e2e hands a local relay's test anchor here.
pub fn apply_ohttp_test_overrides(
    config: VauchiConfig,
    relay_anchor: Option<String>,
) -> Result<VauchiConfig, String> {
    ohttp_overrides(
        config,
        std::env::var("VAUCHI_OHTTP_RELAY_URL").ok(),
        relay_anchor.or_else(|| std::env::var("VAUCHI_RELAY_ANCHOR").ok()),
    )
}

/// The overrides without the environment: `relay_url` replaces the OHTTP
/// route unless blank; `anchor_hex` (64 hex characters) becomes the
/// configured relay's anchor. A malformed anchor is refused, not ignored:
/// ignoring it would quietly leave the relay without one (DC-01).
pub fn ohttp_overrides(
    mut config: VauchiConfig,
    relay_url: Option<String>,
    anchor_hex: Option<String>,
) -> Result<VauchiConfig, String> {
    if let Some(url) = relay_url
        .as_deref()
        .map(str::trim)
        .filter(|url| !url.is_empty())
    {
        config = config.with_ohttp_relay_url(url);
    }
    if let Some(hex) = anchor_hex {
        let mut anchor = [0u8; 32];
        hex::decode_to_slice(hex.trim(), &mut anchor)
            .map_err(|_| "a relay anchor is 64 hex characters".to_string())?;
        let server_url = config.relay.server_url.clone();
        config = config.with_relay(server_url, anchor);
    }
    Ok(config)
}
