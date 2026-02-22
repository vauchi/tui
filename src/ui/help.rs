// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Help Screen
//!
//! Displays keyboard shortcuts and FAQ from vauchi-core.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use vauchi_core::help::{get_faqs_localized, search_faqs_localized, HelpCategory};
use vauchi_core::i18n::Locale;

use crate::app::App;

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let locale = app.i18n.locale();

    // Build help text with keyboard shortcuts and FAQ summary
    let mut help_text = format!(
        "{}\n{}\n\n{}\n{}\n",
        app.i18n.t("help.title"),
        "=".repeat(16),
        app.i18n.t("help.keyboard_shortcuts"),
        "-".repeat(18),
    );

    // Keyboard shortcuts (labels only — keys are universal)
    help_text.push_str(&format!(
        "{nav}\n\
         \x20 j/↓     {down}\n\
         \x20 k/↑     {up}\n\
         \x20 h/←     {left}\n\
         \x20 l/→     {right}\n\
         \x20 Enter   {select}\n\
         \x20 Esc     {back}\n\
         \x20 Tab     {next_field}\n\n\
         {home}\n\
         \x20 e       {exchange}\n\
         \x20 c       {contacts}\n\
         \x20 s       {settings}\n\
         \x20 a       {add_field}\n\
         \x20 d       {del_field}\n\n\
         {contacts_screen}\n\
         \x20 Enter   {view_detail}\n\n\
         {general}\n\
         \x20 ?       {show_help}\n\
         \x20 q       {quit}\n\n\n\
         {faq_title}\n\
         {faq_sep}\n",
        nav = app.i18n.t("help.navigation"),
        down = app.i18n.t("help.move_down"),
        up = app.i18n.t("help.move_up"),
        left = app.i18n.t("help.move_left"),
        right = app.i18n.t("help.move_right"),
        select = app.i18n.t("help.select"),
        back = app.i18n.t("help.go_back"),
        next_field = app.i18n.t("help.next_field"),
        home = app.i18n.t("nav.home"),
        exchange = app.i18n.t("help.open_exchange"),
        contacts = app.i18n.t("help.view_contacts"),
        settings = app.i18n.t("help.open_settings"),
        add_field = app.i18n.t("help.add_field"),
        del_field = app.i18n.t("help.delete_field"),
        contacts_screen = app.i18n.t("contacts.title"),
        view_detail = app.i18n.t("help.view_details"),
        general = app.i18n.t("help.general"),
        show_help = app.i18n.t("help.show_help"),
        quit = app.i18n.t("help.quit"),
        faq_title = app.i18n.t("help.faq_highlights"),
        faq_sep = "-".repeat(14),
    ));

    // Add top FAQ items from each category (now localized)
    let faqs = get_faqs_localized(locale);
    let categories = [
        HelpCategory::GettingStarted,
        HelpCategory::Privacy,
        HelpCategory::Recovery,
    ];

    for category in categories {
        if let Some(faq) = faqs.iter().find(|f| f.category == category) {
            help_text.push_str(&format!("\n• {}\n", faq.question));
            // Truncate answer for display
            let answer = if faq.answer.len() > 100 {
                format!("{}...", &faq.answer[..100])
            } else {
                faq.answer.clone()
            };
            help_text.push_str(&format!("  {}\n", answer.replace('\n', " ")));
        }
    }

    help_text.push_str(&format!("\n{}\n", app.i18n.t("help.close_hint")));

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(app.theme.fg))
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(app.i18n.t("help.title"))
                .borders(Borders::ALL),
        );

    f.render_widget(help, area);
}

/// Get all FAQ items for display in the given locale.
#[allow(dead_code)]
pub fn get_all_faqs(locale: Locale) -> Vec<(String, String, String)> {
    get_faqs_localized(locale)
        .into_iter()
        .map(|faq| (faq.id, faq.question, faq.answer))
        .collect()
}

/// Search FAQs by query in the given locale.
#[allow(dead_code)]
pub fn search_help(query: &str, locale: Locale) -> Vec<(String, String, String)> {
    search_faqs_localized(query, locale)
        .into_iter()
        .map(|faq| (faq.id, faq.question, faq.answer))
        .collect()
}
