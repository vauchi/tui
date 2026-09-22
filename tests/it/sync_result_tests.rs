// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! The status line a sync outcome turns into (vauchi/private#448).

use vauchi_core::api::VauchiSyncOutcome;
use vauchi_tui::sync_service::sync_result_from;

fn completed(received: usize, errors: Vec<String>) -> VauchiSyncOutcome {
    VauchiSyncOutcome::Ok {
        received,
        fetched: received,
        rejected: 0,
        unresolved: 0,
        reject_reasons: String::new(),
        sent: 2,
        acknowledged: 1,
        errors,
        version_policy: None,
        aha_moments: Vec::new(),
    }
}

// @internal
#[test]
fn a_clean_sync_reports_its_counts_and_no_error() {
    let result = sync_result_from(completed(3, Vec::new()));

    assert_eq!(
        (
            result.cards_updated,
            result.updates_sent,
            result.acknowledged,
            result.success,
            result.error
        ),
        (3, 2, 1, true, None)
    );
}

// @internal
#[test]
fn sync_errors_are_joined_into_the_status_line() {
    let result = sync_result_from(completed(0, vec!["relay: 500".into(), "decrypt".into()]));

    assert!(result.success);
    assert_eq!(result.error.as_deref(), Some("relay: 500; decrypt"));
}

// @internal
#[test]
fn not_connected_is_a_failure_with_a_reason() {
    let result = sync_result_from(VauchiSyncOutcome::NotConnected);

    assert!(!result.success);
    assert_eq!(result.error.as_deref(), Some("Not connected to relay"));
}
