// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! The environment's language tag, reported to Core unparsed so "Follow
//! system" renders in it; Core alone maps the tag to a language
//! (vauchi/private#721, ADR-066).

use vauchi_app::ui::{AppEngine, RenderContext};

/// The first of `LC_ALL`, `LC_MESSAGES`, `LANG` that names a locale.
/// `C` and `POSIX` name none, so a later variable is consulted, as
/// `detect_locale` in linux-gtk does.
pub fn system_locale_from(value_for: impl Fn(&str) -> Option<String>) -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .filter_map(value_for)
        .find(|tag| {
            let language = tag.split(['_', '.', '@']).next().unwrap_or_default();
            !matches!(language, "" | "C" | "POSIX")
        })
}

pub fn report_system_locale(engine: &mut AppEngine, value_for: impl Fn(&str) -> Option<String>) {
    let ctx = RenderContext {
        system_locale: system_locale_from(value_for),
        ..engine.render_context().clone()
    };
    engine.set_render_context(ctx);
}
