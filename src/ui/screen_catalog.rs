// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use anyhow::{Context, Result, bail, ensure};
use std::{collections::BTreeSet, fs, path::Path};

pub use vauchi_tui_display::screen_catalog::*;

const USAGE: &str = "usage: --render-catalog <catalog.json> <out-dir> [cols rows]";

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
        bail!(
            "screen catalog contains undecodable command variants: {}",
            skipped.join(", ")
        );
    }
    Ok(())
}
