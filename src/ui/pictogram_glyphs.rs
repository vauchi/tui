// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Single-width terminal glyphs for Core's `pictogram.*` icon tokens. The
//! table is vendored from vauchi/assets
//! `pictograms/exchange/generated/terminal.json` (#473), so the TUI and the
//! CLI show the same glyph for a mode.

use std::collections::HashMap;
use std::sync::OnceLock;

const EXCHANGE_TABLE: &str = include_str!("../../assets/pictograms/exchange-terminal.json");

pub(crate) fn glyph(icon_token: Option<&str>) -> Option<&'static str> {
    static TABLE: OnceLock<HashMap<String, String>> = OnceLock::new();
    let table = TABLE.get_or_init(|| serde_json::from_str(EXCHANGE_TABLE).unwrap_or_default());
    table.get(icon_token?).map(String::as_str)
}
