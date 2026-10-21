// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! The terminal reports the environment's language tag to Core, so
//! "Follow system" renders in it (vauchi/private#721).

use vauchi_app::i18n::Locale;
use vauchi_app::ui::AppEngine;
use vauchi_core::api::Vauchi;
use vauchi_tui::app::{report_system_locale, system_locale_from};

fn env<'a>(values: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |name| {
        values
            .iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| v.to_string())
    }
}

// @internal
#[test]
fn the_raw_tag_of_the_first_set_variable_is_reported() {
    assert_eq!(
        system_locale_from(env(&[("LANG", "de_DE.UTF-8")])).as_deref(),
        Some("de_DE.UTF-8")
    );
    assert_eq!(
        system_locale_from(env(&[("LC_ALL", "fr_CH.UTF-8"), ("LANG", "de_DE.UTF-8")])).as_deref(),
        Some("fr_CH.UTF-8")
    );
    assert_eq!(
        system_locale_from(env(&[("LC_MESSAGES", "it_IT"), ("LANG", "de_DE")])).as_deref(),
        Some("it_IT")
    );
}

// @internal
#[test]
fn unset_empty_c_and_posix_report_nothing() {
    assert_eq!(system_locale_from(env(&[])), None);
    assert_eq!(system_locale_from(env(&[("LANG", "")])), None);
    assert_eq!(
        system_locale_from(env(&[("LC_ALL", "C"), ("LANG", "POSIX")])),
        None
    );
    assert_eq!(
        system_locale_from(env(&[("LC_ALL", "C.UTF-8"), ("LANG", "de_DE.UTF-8")])).as_deref(),
        Some("de_DE.UTF-8")
    );
}

// @internal
#[test]
fn following_the_system_renders_the_environment_language() {
    let mut engine = AppEngine::new(Vauchi::in_memory().expect("in-memory core"));

    report_system_locale(&mut engine, env(&[("LANG", "de_DE.UTF-8")]));

    assert_eq!(
        engine.render_context().locale,
        None,
        "still following the system"
    );
    assert_eq!(engine.render_context().resolved_locale(), Locale::German);
}
