// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Core's screen catalog replayed through the terminal renderer.
//!
//! Each catalog entry is a batch of `Command`s exactly as Core serializes
//! them. Applying the batch to a fresh [`PresentationState`] and drawing
//! one frame into a `TestBackend` yields the same character grid the
//! interactive shell would show, captured as an insta-style `.snap` so
//! `tests/vrt/snap-to-png.py` can rasterize it unchanged.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use serde::{Deserialize, Serialize};
use vauchi_core::Command;

use super::presentation_protocol::PresentationState;
use super::presentation_renderer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FrameSize {
    pub cols: u16,
    pub rows: u16,
}

pub const FULL_FRAME: FrameSize = FrameSize {
    cols: 100,
    rows: 40,
};
pub const COMPACT_FRAME: FrameSize = FrameSize { cols: 60, rows: 24 };

const COMPACT_DIR: &str = "compact";
const MANIFEST_FILE: &str = "manifest.json";
const USAGE: &str = "usage: --render-catalog <catalog.json> <out-dir> [cols rows]";

#[derive(Debug, Deserialize)]
pub struct ScreenCatalog {
    pub schema_version: u32,
    pub screens: Vec<CatalogScreen>,
}

#[derive(Debug, Deserialize)]
pub struct CatalogScreen {
    pub code_id: String,
    pub title: String,
    #[serde(default)]
    pub locale: String,
    /// Kept as raw JSON so a variant the pinned Core cannot decode skips
    /// that one command instead of failing the whole screen.
    pub commands: Vec<serde_json::Value>,
}

#[derive(Debug)]
pub struct ScreenRender {
    pub frame: String,
    pub skipped_commands: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CatalogManifest {
    pub schema_version: u32,
    pub frame: FrameSize,
    pub compact_frame: FrameSize,
    pub screens: Vec<ManifestScreen>,
}

#[derive(Debug, Serialize)]
pub struct ManifestScreen {
    pub code_id: String,
    pub title: String,
    pub locale: String,
    pub snap: String,
    pub compact_snap: String,
    pub skipped_commands: Vec<String>,
}

pub fn render_screen(screen: &CatalogScreen, size: FrameSize) -> Result<ScreenRender> {
    let mut state = PresentationState::default();
    let mut skipped_commands = Vec::new();
    for value in &screen.commands {
        match serde_json::from_value::<Command>(value.clone()) {
            Ok(command) => {
                state.apply(&[command]);
            }
            Err(_) => skipped_commands.push(variant_name(value)),
        }
    }
    Ok(ScreenRender {
        frame: draw_frame(&state, size)?,
        skipped_commands,
    })
}

pub fn render_catalog(
    catalog: &ScreenCatalog,
    out_dir: &Path,
    size: FrameSize,
) -> Result<CatalogManifest> {
    ensure!(
        catalog.schema_version == 1,
        "unsupported screen catalog schema_version {}",
        catalog.schema_version
    );
    let mut seen = BTreeSet::new();
    for screen in &catalog.screens {
        ensure_plain_file_name(&screen.code_id)?;
        ensure!(
            seen.insert(&screen.code_id),
            "duplicate code_id {:?} in screen catalog",
            screen.code_id
        );
    }
    fs::create_dir_all(out_dir.join(COMPACT_DIR))
        .with_context(|| format!("creating {}", out_dir.display()))?;

    let mut screens = Vec::with_capacity(catalog.screens.len());
    for screen in &catalog.screens {
        let full = render_screen(screen, size)?;
        let compact = render_screen(screen, COMPACT_FRAME)?;
        let snap = format!("{}.snap", screen.code_id);
        let compact_snap = format!("{COMPACT_DIR}/{}.snap", screen.code_id);
        fs::write(out_dir.join(&snap), snap_text(screen, size, &full.frame))?;
        fs::write(
            out_dir.join(&compact_snap),
            snap_text(screen, COMPACT_FRAME, &compact.frame),
        )?;
        screens.push(ManifestScreen {
            code_id: screen.code_id.clone(),
            title: screen.title.clone(),
            locale: screen.locale.clone(),
            snap,
            compact_snap,
            skipped_commands: full.skipped_commands,
        });
    }
    let manifest = CatalogManifest {
        schema_version: catalog.schema_version,
        frame: size,
        compact_frame: COMPACT_FRAME,
        screens,
    };
    fs::write(
        out_dir.join(MANIFEST_FILE),
        serde_json::to_string_pretty(&manifest)?,
    )?;
    Ok(manifest)
}

/// `--render-catalog <catalog.json> <out-dir> [cols rows]`.
pub fn run_cli(args: &[String]) -> Result<()> {
    let [catalog_path, out_dir, size @ ..] = args else {
        bail!("{USAGE}");
    };
    let size = match size {
        [] => FULL_FRAME,
        [cols, rows] => FrameSize {
            cols: cols.parse().context("cols must be a number")?,
            rows: rows.parse().context("rows must be a number")?,
        },
        _ => bail!("{USAGE}"),
    };
    ensure!(
        size.cols > 0 && size.rows > 0,
        "frame size must be non-zero"
    );

    let text =
        fs::read_to_string(catalog_path).with_context(|| format!("reading {catalog_path}"))?;
    let catalog: ScreenCatalog =
        serde_json::from_str(&text).with_context(|| format!("parsing {catalog_path}"))?;
    let manifest = render_catalog(&catalog, Path::new(out_dir), size)?;

    println!(
        "[screen-catalog] rendered {} screen(s) at {}x{} (+ compact {}x{}) to {out_dir}",
        manifest.screens.len(),
        size.cols,
        size.rows,
        COMPACT_FRAME.cols,
        COMPACT_FRAME.rows
    );
    let skipped: BTreeSet<&str> = manifest
        .screens
        .iter()
        .flat_map(|screen| screen.skipped_commands.iter().map(String::as_str))
        .collect();
    if !skipped.is_empty() {
        let skipped: Vec<&str> = skipped.into_iter().collect();
        eprintln!(
            "[screen-catalog] skipped command variants the pinned vauchi-core cannot decode: {}",
            skipped.join(", ")
        );
    }
    Ok(())
}

fn draw_frame(state: &PresentationState, size: FrameSize) -> Result<String> {
    let mut terminal = Terminal::new(TestBackend::new(size.cols, size.rows))?;
    terminal.draw(|frame| presentation_renderer::draw(frame, frame.area(), state, 0, None))?;
    let buffer = terminal.backend().buffer();
    let rows = buffer
        .content
        .chunks(usize::from(buffer.area.width))
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>();
    Ok(rows.join("\n"))
}

fn snap_text(screen: &CatalogScreen, size: FrameSize, frame: &str) -> String {
    let description = serde_json::to_string(&format!(
        "{} — {} ({}x{})",
        screen.code_id, screen.title, size.cols, size.rows
    ))
    .unwrap_or_default();
    format!(
        "---\nsource: screen_catalog\ndescription: {description}\nexpression: frame\n---\n{frame}\n"
    )
}

fn variant_name(command: &serde_json::Value) -> String {
    match command {
        serde_json::Value::Object(map) if map.len() == 1 => {
            map.keys().next().cloned().unwrap_or_default()
        }
        serde_json::Value::String(unit) => unit.clone(),
        other => other.to_string(),
    }
}

/// A code_id names the output file, so anything but a plain name is an
/// escape from the output directory.
fn ensure_plain_file_name(code_id: &str) -> Result<()> {
    ensure!(
        !code_id.is_empty()
            && code_id.len() <= 128
            && code_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
        "code_id {code_id:?} is not a plain file name (letters, digits, '-' and '_' only)"
    );
    Ok(())
}
