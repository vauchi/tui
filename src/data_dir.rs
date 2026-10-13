// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! The data directory holds the database, keys and WAL files.

use std::io;
use std::path::Path;

/// Creates the data directory if needed and makes it owner-only (0700 on
/// Unix) every time, not only on creation: a directory left group- or
/// world-readable by an older version or by hand stays exposed otherwise
/// (vauchi/private#290).
pub fn prepare_data_dir(path: &Path) -> io::Result<()> {
    std::fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}
