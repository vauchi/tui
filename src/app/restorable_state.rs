// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Core's opaque restorable state, kept in the data directory between
//! starts (vauchi/private#814). The shell never reads the value; Core
//! validates whatever comes back.

use std::io::{self, Read};
use std::path::Path;

use vauchi_app::ui::{AppEngine, RenderContext};

const FILE_NAME: &str = "restorable_state";
/// Core caps the value far below this; the bound only stops a damaged or
/// planted file from being read whole.
const MAX_BYTES: u64 = 4096;

pub fn keep_state(data_dir: &Path, state: &str) -> io::Result<()> {
    let partial = data_dir.join(format!("{FILE_NAME}.partial"));
    std::fs::write(&partial, state)?;
    // A rename replaces the kept value whole, so a crash mid-write leaves
    // the previous one.
    std::fs::rename(partial, data_dir.join(FILE_NAME))
}

pub fn kept_state(data_dir: &Path) -> Option<String> {
    let file = std::fs::File::open(data_dir.join(FILE_NAME)).ok()?;
    if file.metadata().ok()?.len() > MAX_BYTES {
        return None;
    }
    let mut state = String::new();
    file.take(MAX_BYTES + 1).read_to_string(&mut state).ok()?;
    Some(state)
}

pub fn hand_back_kept_state(engine: &mut AppEngine, data_dir: &Path) {
    let ctx = RenderContext {
        restorable_state: kept_state(data_dir),
        ..engine.render_context().clone()
    };
    engine.set_render_context(ctx);
}
