// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Decisions the binary makes while starting and pacing its event loop,
//! as functions of their inputs. `main` reads files, the environment and
//! the terminal; nothing here does (vauchi/private#448).

use std::time::{Duration, Instant};

pub const DEFAULT_RELAY_URL: &str = "wss://relay.vauchi.app";

/// The relay URL to use: the configured one, else the environment's, else
/// the default. Blank values count as absent.
pub fn resolve_relay_url(configured: Option<String>, from_env: Option<String>) -> String {
    [configured, from_env]
        .into_iter()
        .flatten()
        .map(|url| url.trim().to_string())
        .find(|url| !url.is_empty())
        .unwrap_or_else(|| DEFAULT_RELAY_URL.to_string())
}

/// How long the loop may wait for input: a short poll while a status is
/// flashing or a background operation runs, a second otherwise, and never
/// past Core's next scheduled wakeup.
pub fn poll_timeout(
    status_flashing: bool,
    background_operation: bool,
    next_wakeup: Option<Instant>,
    now: Instant,
) -> Duration {
    let base = if status_flashing || background_operation {
        Duration::from_millis(100)
    } else {
        Duration::from_secs(1)
    };
    match next_wakeup {
        Some(wakeup) => base.min(wakeup.saturating_duration_since(now)),
        None => base,
    }
}

/// Whether the loop should run Core's heartbeat now: only when no input
/// arrived and the scheduled wakeup has come.
pub fn wakeup_due(event_ready: bool, next_wakeup: Option<Instant>, now: Instant) -> bool {
    !event_ready && next_wakeup.is_some_and(|wakeup| now >= wakeup)
}
