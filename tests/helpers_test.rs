// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Unit tests for `helpers.rs` — action labels, execute_action dispatch,
//! copy_to_clipboard error handling, and open_field_uri logic.

use vauchi_core::contact_card::ContactAction;
use vauchi_tui::helpers;

// ─── action_label ───────────────────────────────────────────────────────────

#[test]
fn action_label_call_includes_number() {
    let label = helpers::action_label(&ContactAction::Call("+41 79 000 00 00".into()));
    assert_eq!(label, "Call +41 79 000 00 00");
}

#[test]
fn action_label_send_sms_includes_number() {
    let label = helpers::action_label(&ContactAction::SendSms("+1-555-1234".into()));
    assert_eq!(label, "Send SMS to +1-555-1234");
}

#[test]
fn action_label_send_email_includes_address() {
    let label = helpers::action_label(&ContactAction::SendEmail("alice@example.com".into()));
    assert_eq!(label, "Email alice@example.com");
}

#[test]
fn action_label_open_url_is_generic() {
    let label = helpers::action_label(&ContactAction::OpenUrl("https://example.com".into()));
    assert_eq!(label, "Open in Browser");
}

#[test]
fn action_label_open_map_is_generic() {
    let label = helpers::action_label(&ContactAction::OpenMap("123 Main St".into()));
    assert_eq!(label, "Open in Maps");
}

#[test]
fn action_label_get_directions_is_generic() {
    let label = helpers::action_label(&ContactAction::GetDirections("456 Elm St".into()));
    assert_eq!(label, "Get Directions");
}

#[test]
fn action_label_copy_to_clipboard() {
    let label = helpers::action_label(&ContactAction::CopyToClipboard);
    assert_eq!(label, "Copy to Clipboard");
}

// ─── execute_action ─────────────────────────────────────────────────────────

#[test]
fn execute_action_copy_to_clipboard_returns_caller_error() {
    // CopyToClipboard is a sentinel — the caller must handle it with a field value.
    let result = helpers::execute_action(&ContactAction::CopyToClipboard);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.contains("handled by the caller"),
        "expected caller delegation message, got: {}",
        err
    );
}

// NOTE: execute_action (Call/SMS/Email/OpenUrl/OpenMap/GetDirections),
// copy_to_clipboard, and open_field_uri all call open::that() or
// arboard::Clipboard — real system side effects. Covered by manual
// device testing, not unit tests.

// ─── action_label exhaustiveness ────────────────────────────────────────────

/// Verify every ContactAction variant returns a non-empty label.
#[test]
fn action_label_all_variants_non_empty() {
    let variants: Vec<ContactAction> = vec![
        ContactAction::Call("123".into()),
        ContactAction::SendSms("123".into()),
        ContactAction::SendEmail("a@b.com".into()),
        ContactAction::OpenUrl("https://x.com".into()),
        ContactAction::OpenMap("addr".into()),
        ContactAction::GetDirections("addr".into()),
        ContactAction::CopyToClipboard,
    ];
    for action in &variants {
        let label = helpers::action_label(action);
        assert!(
            !label.is_empty(),
            "label for {:?} should not be empty",
            action
        );
    }
}

/// Verify Call/SMS/Email labels include the value (not just a static string).
#[test]
fn action_label_dynamic_variants_include_value() {
    let phone = "+41 79 123 45 67";
    assert!(
        helpers::action_label(&ContactAction::Call(phone.into())).contains(phone),
        "Call label must include the phone number"
    );

    let email = "test@example.com";
    assert!(
        helpers::action_label(&ContactAction::SendEmail(email.into())).contains(email),
        "Email label must include the email address"
    );

    let sms_num = "+1-555-9999";
    assert!(
        helpers::action_label(&ContactAction::SendSms(sms_num.into())).contains(sms_num),
        "SMS label must include the phone number"
    );
}
