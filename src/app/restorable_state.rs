// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Core's opaque restorable state, kept in the data directory between
//! starts (vauchi/private#814).

use std::io;
use std::path::Path;

use vauchi_app::ui::AppEngine;

pub fn keep_state(_data_dir: &Path, _state: &str) -> io::Result<()> {
    Ok(())
}

pub fn kept_state(_data_dir: &Path) -> Option<String> {
    None
}

pub fn hand_back_kept_state(_engine: &mut AppEngine, _data_dir: &Path) {}
