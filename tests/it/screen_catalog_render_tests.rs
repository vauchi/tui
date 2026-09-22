// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Core's screen catalog replayed through the real terminal renderer.
//!
//! The fixture is two batches lifted verbatim from Core's
//! `presentation_contract_v1.json`, so a render proves the shell projects
//! what Core actually emits — not hand-built surfaces.

use std::fs;

use vauchi_tui::ui::screen_catalog::{
    COMPACT_FRAME, FULL_FRAME, ScreenCatalog, render_catalog, render_screen, run_cli,
};

const TWO_ENTRY_CATALOG: &str = include_str!("fixtures/screen_catalog_two_entries.json");

fn two_entry_catalog() -> ScreenCatalog {
    serde_json::from_str(TWO_ENTRY_CATALOG).expect("fixture is a valid catalog")
}

fn snap_body(snap: &str) -> &str {
    let (_, body) = snap
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n"))
        .expect("snap carries insta-style frontmatter");
    body
}

// @internal
#[test]
fn catalog_entries_render_to_distinct_frames_titled_by_their_surface() {
    let out = tempfile::tempdir().unwrap();

    let manifest = render_catalog(&two_entry_catalog(), out.path(), FULL_FRAME).unwrap();

    let welcome = fs::read_to_string(out.path().join("welcome.snap")).unwrap();
    let name = fs::read_to_string(out.path().join("display-name.snap")).unwrap();
    let (welcome, name) = (snap_body(&welcome), snap_body(&name));
    assert!(welcome.contains("Welcome to Vauchi"), "{welcome}");
    assert!(name.contains("What's your name?"), "{name}");
    assert!(
        welcome.contains("Create new identity"),
        "context bar missing: {welcome}"
    );
    assert_ne!(welcome, name);
    for body in [welcome, name] {
        let rows: Vec<&str> = body.lines().collect();
        assert_eq!(rows.len(), usize::from(FULL_FRAME.rows));
        assert!(
            rows.iter()
                .all(|row| row.chars().count() == usize::from(FULL_FRAME.cols))
        );
    }

    let compact = fs::read_to_string(out.path().join("compact").join("welcome.snap")).unwrap();
    let compact_rows: Vec<&str> = snap_body(&compact).lines().collect();
    assert_eq!(compact_rows.len(), usize::from(COMPACT_FRAME.rows));
    assert!(compact_rows[0].chars().count() == usize::from(COMPACT_FRAME.cols));

    let code_ids: Vec<&str> = manifest
        .screens
        .iter()
        .map(|s| s.code_id.as_str())
        .collect();
    assert_eq!(code_ids, ["welcome", "display-name"]);
    let written: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out.path().join("manifest.json")).unwrap())
            .unwrap();
    assert_eq!(written["screens"][1]["code_id"], "display-name");
    assert_eq!(written["screens"][1]["snap"], "display-name.snap");
}

// @internal
#[test]
fn command_variants_the_pinned_core_lacks_are_skipped_and_reported() {
    let mut catalog = two_entry_catalog();
    let screen = &mut catalog.screens[0];
    screen
        .commands
        .push(serde_json::json!({"NotACommandYet": {"surface_id": "onboarding"}}));

    let rendered = render_screen(screen, FULL_FRAME).unwrap();

    assert!(rendered.frame.contains("Welcome to Vauchi"));
    // Real skew (a variant Core main emits that the pinned tag lacks) may
    // precede it; the fake one is always the last command in the batch.
    assert_eq!(rendered.skipped_commands.last().unwrap(), "NotACommandYet");
    assert!(!rendered.skipped_commands.contains(&"ReplaceSurface".into()));
}

// @internal
#[test]
fn code_ids_that_are_not_plain_file_names_are_rejected() {
    let mut catalog = two_entry_catalog();
    catalog.screens[0].code_id = "../escaped".into();
    // A bare tempdir's parent is the runner-wide temp dir, where an earlier
    // run (e.g. a mutant of the name check) may have left escaped.snap.
    let owned_root = tempfile::tempdir().unwrap();
    let out = owned_root.path().join("out");
    std::fs::create_dir(&out).unwrap();

    let result = render_catalog(&catalog, &out, FULL_FRAME);

    assert!(result.is_err());
    assert!(!owned_root.path().join("escaped.snap").exists());
}

// @internal
#[test]
fn run_cli_renders_at_the_default_or_requested_size() {
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/it/fixtures/screen_catalog_two_entries.json"
    );
    let out = tempfile::tempdir().unwrap();
    run_cli(&[fixture.into(), out.path().to_str().unwrap().into()]).unwrap();
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
}

// @internal
#[test]
fn run_cli_rejects_wrong_argument_counts_and_bad_sizes() {
    let fixture: String = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/it/fixtures/screen_catalog_two_entries.json"
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

// @internal
#[test]
fn a_skipped_command_with_several_keys_is_reported_verbatim() {
    let mut catalog = two_entry_catalog();
    let screen = &mut catalog.screens[0];
    screen
        .commands
        .push(serde_json::json!({"Alpha": 1, "Beta": 2}));

    let rendered = render_screen(screen, FULL_FRAME).unwrap();

    assert_eq!(
        rendered.skipped_commands.last().unwrap(),
        r#"{"Alpha":1,"Beta":2}"#
    );
}
