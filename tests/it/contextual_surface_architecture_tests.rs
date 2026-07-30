// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::fs;
use std::path::Path;

// @scenario: generic_presentation_protocol.feature :: Release contains only the generic action system
#[test]
fn live_event_loop_uses_only_the_generic_presentation_boundary() {
    let source = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/main.rs"))
        .expect("TUI main source");
    let run_loop = source
        .split("fn run_app")
        .nth(1)
        .expect("run_app implementation");

    assert!(run_loop.contains("ui::draw_presentation"));
    assert!(run_loop.contains("handlers::handle_presentation_key"));
    assert!(!run_loop.contains("ui::draw(f, app)"));
    assert!(!run_loop.contains("handlers::handle_key(app"));
}

// @internal
#[test]
fn generic_terminal_adapter_handles_interactive_core_effects() {
    let source = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/handlers/presentation.rs"),
    )
    .expect("generic presentation handler");

    for effect in ["FilePickFromUser", "QrRequestScan", "ImagePickFromFile"] {
        assert!(
            source.contains(effect),
            "generic terminal adapter does not handle {effect}"
        );
    }
    assert!(source.contains("FilePickCancelledByUser"));
    assert!(source.contains("ImagePickCancelled"));
}
