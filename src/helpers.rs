// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! TUI-specific helpers for contact actions, clipboard, and system integration.
//!
//! These functions wrap platform operations (opening URLs, copying to clipboard)
//! that don't belong in vauchi-core.

use vauchi_core::contact_card::ContactAction;

/// Execute a contact action (open URI or copy to clipboard).
///
/// If opening fails, automatically copies the relevant value to the clipboard as a fallback.
pub fn execute_action(action: &ContactAction) -> Result<String, String> {
    match action {
        ContactAction::Call(v) => match open::that(format!("tel:{}", v)) {
            Ok(_) => Ok("Opened dialer".to_string()),
            Err(e) => clipboard_fallback(v, "dialer", e),
        },
        ContactAction::SendSms(v) => match open::that(format!("sms:{}", v)) {
            Ok(_) => Ok("Opened messaging".to_string()),
            Err(e) => clipboard_fallback(v, "messaging", e),
        },
        ContactAction::SendEmail(v) => match open::that(format!("mailto:{}", v)) {
            Ok(_) => Ok("Opened email client".to_string()),
            Err(e) => clipboard_fallback(v, "email client", e),
        },
        ContactAction::OpenUrl(v) => match open::that(v) {
            Ok(_) => Ok("Opened browser".to_string()),
            Err(e) => clipboard_fallback(v, "browser", e),
        },
        ContactAction::OpenMap(v) => {
            let encoded = percent_encode(v);
            match open::that(format!(
                "https://www.openstreetmap.org/search?query={}",
                encoded
            )) {
                Ok(_) => Ok("Opened maps".to_string()),
                Err(e) => clipboard_fallback(v, "maps", e),
            }
        }
        ContactAction::GetDirections(v) => {
            let encoded = percent_encode(v);
            match open::that(format!(
                "https://www.openstreetmap.org/directions?route=&to={}",
                encoded
            )) {
                Ok(_) => Ok("Opened directions".to_string()),
                Err(e) => clipboard_fallback(v, "directions", e),
            }
        }
        ContactAction::CopyToClipboard => {
            Err("CopyToClipboard should be handled by the caller with field value".to_string())
        }
    }
}

/// Copy a value to the system clipboard.
pub fn copy_to_clipboard(value: &str) -> Result<String, String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("Failed to access clipboard: {}", e))?;
    clipboard
        .set_text(value)
        .map_err(|e| format!("Failed to copy: {}", e))?;
    Ok("Copied to clipboard".to_string())
}

/// Open a contact field URI in the system default app, with clipboard fallback.
pub fn open_field_uri(uri: &str, label: &str, action_type: &str) -> String {
    match open::that(uri) {
        Ok(_) => format!("Opened {} in {}", label, action_type),
        Err(e) => match clipboard_fallback(uri, label, e) {
            Ok(msg) => msg,
            Err(msg) => msg,
        },
    }
}

/// Build secondary action labels from contact actions.
pub fn action_label(action: &ContactAction) -> String {
    match action {
        ContactAction::Call(v) => format!("Call {}", v),
        ContactAction::SendSms(v) => format!("Send SMS to {}", v),
        ContactAction::SendEmail(v) => format!("Email {}", v),
        ContactAction::OpenUrl(_) => "Open in Browser".to_string(),
        ContactAction::OpenMap(_) => "Open in Maps".to_string(),
        ContactAction::GetDirections(_) => "Get Directions".to_string(),
        ContactAction::CopyToClipboard => "Copy to Clipboard".to_string(),
    }
}

fn clipboard_fallback(
    value: &str,
    action_name: &str,
    error: std::io::Error,
) -> Result<String, String> {
    match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(value)) {
        Ok(_) => Ok(format!(
            "Could not open {} ({}). Copied to clipboard.",
            action_name, error
        )),
        Err(_) => Ok(format!(
            "Could not open {} ({}). Value: {}",
            action_name, error, value
        )),
    }
}

fn percent_encode(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "-._~,+/".contains(c) {
                c.to_string()
            } else if c == ' ' {
                "%20".to_string()
            } else {
                format!("%{:02X}", c as u32)
            }
        })
        .collect()
}
