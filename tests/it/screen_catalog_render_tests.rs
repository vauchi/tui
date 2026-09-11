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
    COMPACT_FRAME, FULL_FRAME, ScreenCatalog, render_catalog, render_screen,
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
    let out = tempfile::tempdir().unwrap();

    let result = render_catalog(&catalog, out.path(), FULL_FRAME);

    assert!(result.is_err());
    assert!(!out.path().parent().unwrap().join("escaped.snap").exists());
}
