// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Start-up and event-loop pacing decisions (vauchi/private#448).

use std::time::{Duration, Instant};

use vauchi_tui::startup::{DEFAULT_RELAY_URL, poll_timeout, resolve_relay_url, wakeup_due};

// @internal
#[test]
fn the_relay_url_prefers_the_configured_file_then_the_environment() {
    assert_eq!(
        resolve_relay_url(
            Some(" wss://a.example ".into()),
            Some("wss://b.example".into())
        ),
        "wss://a.example"
    );
    assert_eq!(
        resolve_relay_url(Some("   ".into()), Some("wss://b.example".into())),
        "wss://b.example"
    );
    assert_eq!(
        resolve_relay_url(None, Some(String::new())),
        DEFAULT_RELAY_URL
    );
    assert_eq!(resolve_relay_url(None, None), DEFAULT_RELAY_URL);
}

// @internal
#[test]
fn the_loop_polls_fast_while_flashing_or_busy_and_never_past_the_wakeup() {
    let now = Instant::now();

    assert_eq!(
        poll_timeout(false, false, None, now),
        Duration::from_secs(1)
    );
    assert_eq!(
        poll_timeout(true, false, None, now),
        Duration::from_millis(100)
    );
    assert_eq!(
        poll_timeout(false, true, None, now),
        Duration::from_millis(100)
    );
    assert_eq!(
        poll_timeout(false, false, Some(now + Duration::from_millis(300)), now),
        Duration::from_millis(300)
    );
    assert_eq!(
        poll_timeout(true, false, Some(now + Duration::from_millis(300)), now),
        Duration::from_millis(100)
    );
    assert_eq!(
        poll_timeout(false, false, Some(now - Duration::from_secs(1)), now),
        Duration::ZERO
    );
}

// @internal
#[test]
fn the_heartbeat_runs_only_when_idle_and_due() {
    let now = Instant::now();

    assert!(wakeup_due(false, Some(now), now));
    assert!(wakeup_due(false, Some(now - Duration::from_millis(1)), now));
    assert!(!wakeup_due(
        false,
        Some(now + Duration::from_millis(1)),
        now
    ));
    assert!(!wakeup_due(true, Some(now), now));
    assert!(!wakeup_due(false, None, now));
}

// @internal
#[cfg(unix)]
#[test]
fn an_existing_data_directory_is_tightened_to_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let data_dir = root.path().join("vauchi");
    std::fs::create_dir(&data_dir).unwrap();
    std::fs::set_permissions(&data_dir, std::fs::Permissions::from_mode(0o755)).unwrap();

    vauchi_tui::data_dir::prepare_data_dir(&data_dir).unwrap();

    let mode = std::fs::metadata(&data_dir).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o700);
}

// @internal
#[cfg(unix)]
#[test]
fn a_new_data_directory_is_created_owner_only() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let data_dir = root.path().join("nested").join("vauchi");

    vauchi_tui::data_dir::prepare_data_dir(&data_dir).unwrap();

    let mode = std::fs::metadata(&data_dir).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o700);
}

// @internal
#[test]
fn a_data_directory_path_that_is_a_file_is_refused() {
    let root = tempfile::tempdir().unwrap();
    let data_dir = root.path().join("vauchi");
    std::fs::write(&data_dir, b"not a directory").unwrap();

    assert!(vauchi_tui::data_dir::prepare_data_dir(&data_dir).is_err());
    assert_eq!(std::fs::read(&data_dir).unwrap(), b"not a directory");
}
