// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Hands a URL to the desktop's opener.

/// The program and arguments that open `url` on `target_os` (a
/// `std::env::consts::OS` value). The URL is always one argument, never
/// passed through a shell: `start` is a `cmd.exe` builtin, not a program,
/// and `cmd` would read `&` in a URL as a command separator
/// (vauchi/private#290).
pub fn opener_command<'a>(target_os: &str, url: &'a str) -> (&'static str, Vec<&'a str>) {
    match target_os {
        "windows" => ("rundll32", vec!["url.dll,FileProtocolHandler", url]),
        "macos" => ("open", vec![url]),
        _ => ("xdg-open", vec![url]),
    }
}

/// Opens `url` with the desktop's opener; false when it could not start.
pub fn open(url: &str) -> bool {
    let (program, args) = opener_command(std::env::consts::OS, url);
    std::process::Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .is_ok()
}
