// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Core's screen catalog replayed through the real terminal renderer.
//!
//! The fixture is two batches lifted verbatim from Core's
//! `presentation_contract_v1.json`, so a render proves the shell projects
//! what Core actually emits — not hand-built surfaces.

use std::fs;

use vauchi_tui::ui::screen_catalog::{FULL_FRAME, run_cli};

const TWO_ENTRY_CATALOG: &str =
    include_str!("../../tui-display/tests/fixtures/screen_catalog_two_entries.json");

fn snap_body(snap: &str) -> &str {
    let (_, body) = snap
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n"))
        .expect("snap carries insta-style frontmatter");
    body
}

// @internal
#[test]
fn run_cli_renders_at_the_default_or_requested_size() {
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tui-display/tests/fixtures/screen_catalog_two_entries.json"
    );
    let out = tempfile::tempdir().unwrap();
    let manifest = run_cli(&[fixture.into(), out.path().to_str().unwrap().into()]).unwrap();
    assert_eq!(manifest.frame, FULL_FRAME);
    assert_eq!(manifest.screens.len(), 2);
    let welcome = fs::read_to_string(out.path().join("welcome.snap")).unwrap();
    assert_eq!(
        snap_body(&welcome).lines().count(),
        usize::from(FULL_FRAME.rows)
    );

    let sized = tempfile::tempdir().unwrap();
    run_cli(&[
        fixture.into(),
        sized.path().to_str().unwrap().into(),
        "100".into(),
        "30".into(),
    ])
    .unwrap();
    let welcome = fs::read_to_string(sized.path().join("welcome.snap")).unwrap();
    let rows: Vec<&str> = snap_body(&welcome).lines().collect();
    assert_eq!(rows.len(), 30);
    assert!(rows.iter().all(|row| row.chars().count() == 100));

    let binary_out = tempfile::tempdir().unwrap();
    let process = std::process::Command::new(env!("CARGO_BIN_EXE_vauchi-tui"))
        .args([
            "--render-catalog",
            fixture,
            binary_out.path().to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(process.status.code(), Some(0), "{:?}", process.stderr);
    assert_eq!(
        String::from_utf8(process.stdout).unwrap(),
        format!(
            "[screen-catalog] rendered 2 screen(s) at 100x40 (+ compact 60x24) to {}\n",
            binary_out.path().display(),
        ),
    );
}

// @internal
#[test]
fn run_cli_rejects_catalogs_with_undecodable_commands() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("catalog.json");
    let output = temp.path().join("rendered");
    let mut catalog: serde_json::Value = serde_json::from_str(TWO_ENTRY_CATALOG).unwrap();
    catalog["screens"][0]["commands"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"NotACommandYet": {}}));
    fs::write(&input, serde_json::to_vec(&catalog).unwrap()).unwrap();

    let error = run_cli(&[
        input.to_str().unwrap().into(),
        output.to_str().unwrap().into(),
    ])
    .unwrap_err()
    .to_string();

    assert!(error.contains("undecodable command"), "{error}");
    assert!(error.contains("NotACommandYet"), "{error}");
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(
        manifest["screens"][0]["skipped_commands"][0],
        "NotACommandYet"
    );

    let process = std::process::Command::new(env!("CARGO_BIN_EXE_vauchi-tui"))
        .arg("--render-catalog")
        .arg(&input)
        .arg(temp.path().join("cli-rendered"))
        .output()
        .unwrap();
    assert_eq!(process.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&process.stderr).contains("NotACommandYet"));
}

// @internal
#[test]
fn run_cli_rejects_wrong_argument_counts_and_bad_sizes() {
    let fixture: String = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tui-display/tests/fixtures/screen_catalog_two_entries.json"
    )
    .into();
    let out = tempfile::tempdir().unwrap();
    let out_dir: String = out.path().to_str().unwrap().into();

    let cases: [(Vec<String>, &str); 4] = [
        (vec![fixture.clone()], "--render-catalog"),
        (
            vec![fixture.clone(), out_dir.clone(), "80".into()],
            "--render-catalog",
        ),
        (
            vec![fixture.clone(), out_dir.clone(), "80".into(), "x".into()],
            "rows must be a number",
        ),
        (
            vec![fixture.clone(), out_dir.clone(), "0".into(), "24".into()],
            "frame size must be non-zero",
        ),
    ];
    for (args, expected) in cases {
        let error = run_cli(&args).unwrap_err().to_string();
        assert!(error.contains(expected), "{args:?}: {error}");
    }
    assert!(!out.path().join("welcome.snap").exists());
}
