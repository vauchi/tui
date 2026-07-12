// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Shared mapping from core's semantic icon tokens to terminal-safe
//! glyphs.
//!
//! Core emits SF-Symbol-style icon tokens on `ScreenModel` items
//! (`qrcode`, `sparkles`, `shield`, …); each frontend maps those names
//! to its native symbol set. The TUI has no symbol set, so it renders a
//! small ASCII badge for the tokens it recognizes. Unknown tokens return
//! `None` so every call site can pick its own generic fallback (list
//! rows fall back to a bullet, status indicators to their status glyph)
//! — the raw token must never reach the screen.

/// Map a core icon token to a terminal-safe badge glyph.
///
/// Returns `Some(badge)` for a recognized token and `None` for anything
/// else. Callers own the fallback so the raw token is never rendered
/// verbatim — see the module docs.
pub(crate) fn icon_badge(token: &str) -> Option<&'static str> {
    Some(match token {
        "lock" => "[*]",
        "refresh" => "[~]",
        "people" | "group" => "[#]",
        "shield" => "[S]",
        "server" => "[@]",
        "key" => "[K]",
        "check" => "[v]",
        "share" => "[>]",
        "edit" => "[/]",
        "warning" => "[!]",
        "devices" => "[D]",
        "backup" | "drive" => "[B]",
        "card" | "id_card" => "[C]",
        "eye" => "[o]",
        "folder" => "[F]",
        "lifebuoy" => "[R]",
        "swap" => "[X]",
        "checkmark.seal" => "[Y]",
        _ => return None,
    })
}

// INLINE_TEST_REQUIRED: icon_badge is pub(crate); the tests/ integration
// target (a separate crate) cannot reach it, so the mapping must be
// exercised inline.
#[cfg(test)]
mod tests {
    use super::*;

    // @internal
    #[test]
    fn known_tokens_map_to_badges() {
        assert_eq!(icon_badge("lock"), Some("[*]"));
        assert_eq!(icon_badge("shield"), Some("[S]"));
        assert_eq!(icon_badge("group"), Some("[#]"));
    }

    // @internal
    #[test]
    fn unknown_tokens_return_none() {
        assert_eq!(icon_badge("definitely-not-a-known-icon"), None);
        // Exchange-method tokens have no ASCII badge — callers fall back.
        assert_eq!(icon_badge("qrcode"), None);
        assert_eq!(icon_badge("sparkles"), None);
    }

    // @internal
    #[test]
    fn no_badge_echoes_its_token() {
        // A badge must never equal the token it maps — that would be a
        // raw-identifier leak. Covers the full known set.
        for token in [
            "lock",
            "refresh",
            "people",
            "group",
            "shield",
            "server",
            "key",
            "check",
            "share",
            "edit",
            "warning",
            "devices",
            "backup",
            "drive",
            "card",
            "id_card",
            "eye",
            "folder",
            "lifebuoy",
            "swap",
            "checkmark.seal",
        ] {
            assert_ne!(
                icon_badge(token),
                Some(token),
                "token {token:?} leaked verbatim"
            );
        }
    }
}
