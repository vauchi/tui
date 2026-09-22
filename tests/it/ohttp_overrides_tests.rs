// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! The e2e OHTTP overrides, fed directly instead of through the
//! environment (vauchi/private#448).

use vauchi_core::VauchiConfig;
use vauchi_tui::ohttp_overrides;

// @internal
#[test]
fn a_blank_relay_url_keeps_the_configured_route() {
    let base = VauchiConfig::default();

    let config = ohttp_overrides(VauchiConfig::default(), Some("   ".into()), None);

    assert_eq!(config.relay.ohttp_relay_url, base.relay.ohttp_relay_url);
}

// @internal
#[test]
fn a_relay_url_is_trimmed_and_applied() {
    let config = ohttp_overrides(
        VauchiConfig::default(),
        Some(" https://relay.test ".into()),
        None,
    );

    assert_eq!(
        config.relay.ohttp_relay_url.as_deref(),
        Some("https://relay.test")
    );
}

// @internal
#[test]
fn a_hex_key_replaces_the_bundled_gateway_key_and_bad_hex_is_ignored() {
    let base = VauchiConfig::default();

    let overridden = ohttp_overrides(VauchiConfig::default(), None, Some(" 0a0b ".into()));
    let rejected = ohttp_overrides(VauchiConfig::default(), None, Some("zz".into()));

    assert_eq!(overridden.ohttp.bundled_gateway_key, Some(vec![10, 11]));
    assert_eq!(
        rejected.ohttp.bundled_gateway_key,
        base.ohttp.bundled_gateway_key
    );
}
