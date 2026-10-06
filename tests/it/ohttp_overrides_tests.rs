// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! The relay overrides, fed directly instead of through the environment
//! (vauchi/private#448).

use vauchi_core::VauchiConfig;
use vauchi_tui::ohttp_overrides;

const ANCHOR_HEX: &str = "5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a";

fn custom_relay() -> VauchiConfig {
    VauchiConfig::default().with_relay_url("https://relay.self.example")
}

// @internal
#[test]
fn a_blank_relay_url_keeps_the_configured_route() {
    let base = VauchiConfig::default();

    let config = ohttp_overrides(VauchiConfig::default(), Some("   ".into()), None).unwrap();

    assert_eq!(config.relay.ohttp_relay_url, base.relay.ohttp_relay_url);
}

// @internal
#[test]
fn a_relay_url_is_trimmed_and_applied() {
    let config = ohttp_overrides(
        VauchiConfig::default(),
        Some(" https://relay.test ".into()),
        None,
    )
    .unwrap();

    assert_eq!(
        config.relay.ohttp_relay_url.as_deref(),
        Some("https://relay.test")
    );
}

/// A custom relay's OHTTP anchor travels with its URL (#288, decision
/// 0.9): its gateway keys are then accepted only through its signed chain.
// @internal
#[test]
fn a_relay_anchor_is_set_with_the_relay() {
    let config = ohttp_overrides(custom_relay(), None, Some(format!(" {ANCHOR_HEX} "))).unwrap();

    assert_eq!(config.relay.server_url, "https://relay.self.example");
    assert_eq!(config.relay.ohttp_trust_anchor(), Some([0x5a; 32]));
}

/// DC-01: a malformed anchor stops the TUI instead of being ignored —
/// ignoring it would quietly leave the relay without one.
// @internal
#[test]
fn a_malformed_relay_anchor_is_refused() {
    let not_hex = "zz".repeat(32);
    for value in ["", "abcd", &ANCHOR_HEX[..62], not_hex.as_str()] {
        let result = ohttp_overrides(custom_relay(), None, Some(value.into()));

        assert!(result.is_err(), "{value:?} must be refused");
    }
}

// @internal
#[test]
fn without_an_anchor_a_custom_relay_has_none() {
    let config = ohttp_overrides(custom_relay(), None, None).unwrap();

    assert_eq!(config.relay.ohttp_trust_anchor(), None);
}
