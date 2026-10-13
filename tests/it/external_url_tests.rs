// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! The program that opens an external URL on each OS (vauchi/private#290).

use vauchi_tui::external_url::opener_command;

const URL: &str = "https://vauchi.app/x#pk=a&n=b c";

// @internal
#[test]
fn windows_opens_urls_without_a_shell_builtin() {
    let (program, args) = opener_command("windows", URL);

    assert_eq!(program, "rundll32");
    assert_eq!(args, vec!["url.dll,FileProtocolHandler", URL]);
}

// @internal
#[test]
fn macos_and_linux_use_their_desktop_openers() {
    assert_eq!(opener_command("macos", URL), ("open", vec![URL]));
    assert_eq!(opener_command("linux", URL), ("xdg-open", vec![URL]));
    assert_eq!(opener_command("freebsd", URL), ("xdg-open", vec![URL]));
}
