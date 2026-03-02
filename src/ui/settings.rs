// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Settings Screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, InputMode};

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Display name
            Constraint::Length(4), // Public ID
            Constraint::Length(4), // Relay URL
            Constraint::Min(0),    // Options
        ])
        .margin(1)
        .split(area);

    // Display name (editable)
    let name = app.backend.display_name().unwrap_or("Not set");
    let name_para = Paragraph::new(format!(
        "{}: {}  [press n to edit]",
        app.i18n.t("settings.display_name"),
        name
    ))
    .style(Style::default().fg(app.theme.warning))
    .block(
        Block::default()
            .title(app.i18n.t("settings.identity"))
            .borders(Borders::ALL),
    );
    f.render_widget(name_para, chunks[0]);

    // Public ID
    if let Some(id) = app.backend.public_id() {
        let id_para = Paragraph::new(format!("{}: {}", app.i18n.t("home.public_id"), id))
            .style(Style::default().fg(app.theme.fg_secondary))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(id_para, chunks[1]);
    }

    // Relay URL
    let relay_url = app.backend.relay_url();
    let relay_para = Paragraph::new(format!(
        "{}: {}  [press u to edit]",
        app.i18n.t("settings.relay"),
        relay_url
    ))
    .style(Style::default().fg(app.theme.accent))
    .block(
        Block::default()
            .title(app.i18n.t("settings.sync_server"))
            .borders(Borders::ALL),
    );
    f.render_widget(relay_para, chunks[2]);

    // Options and Help Links
    let options = format!(
        "{options}:\n\
         \x20 [n] {edit_name}\n\
         \x20 [u] {edit_relay}\n\
         \x20 [b] {backup}\n\
         \x20 [d] {devices}\n\
         \x20 [r] {recovery}\n\
         \x20 [t] {tor}\n\
         \x20 [e] {emergency}\n\
         \x20 [D] Duress PIN\n\
         \x20 [p] {privacy}\n\
         \x20 [s] {support}\n\n\
         {help_support}:\n\
         \x20 User Guide:     https://vauchi.app/user-guide\n\
         \x20 FAQ:            https://vauchi.app/faq\n\
         \x20 Report Issue:   https://github.com/vauchi/issues\n\
         \x20 Privacy Policy: https://vauchi.app/privacy\n\n\
         {auto_save}\n\
         {local_encrypted}\n\n\
         Version 1.0.0\n",
        options = app.i18n.t("settings.options"),
        edit_name = app.i18n.t("settings.edit_name"),
        edit_relay = app.i18n.t("settings.edit_relay"),
        backup = app.i18n.t("backup.title"),
        devices = app.i18n.t("devices.title"),
        recovery = app.i18n.t("recovery.title"),
        tor = app.i18n.t("privacy.title"),
        emergency = app.i18n.t("emergency.title"),
        privacy = app.i18n.t("privacy.title"),
        support = app.i18n.t("support.title"),
        help_support = app.i18n.t("settings.help_support"),
        auto_save = app.i18n.t("settings.auto_save"),
        local_encrypted = app.i18n.t("settings.local_encrypted"),
    );

    let help_para = Paragraph::new(options)
        .style(Style::default().fg(app.theme.fg_secondary))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help_para, chunks[3]);
}

pub fn draw_edit_name(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Current name info
            Constraint::Length(3), // Name input
            Constraint::Min(0),    // Spacer
        ])
        .margin(2)
        .split(area);

    let state = &app.edit_name_state;

    // Current name info
    let current_name = app.backend.display_name().unwrap_or("Not set");
    let info_para = Paragraph::new(format!(
        "{}: {}",
        app.i18n.t("settings.current"),
        current_name
    ))
    .style(Style::default().fg(app.theme.fg_secondary))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(info_para, chunks[0]);

    // Name input
    let name_text = if app.input_mode == InputMode::Editing {
        format!("{}|", state.new_name)
    } else {
        state.new_name.clone()
    };
    let name_para = Paragraph::new(name_text)
        .style(Style::default().fg(app.theme.warning))
        .block(
            Block::default()
                .title(app.i18n.t("settings.new_name"))
                .borders(Borders::ALL),
        );
    f.render_widget(name_para, chunks[1]);
}

pub fn draw_edit_relay_url(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Current URL info
            Constraint::Length(3), // URL input
            Constraint::Length(3), // Help text
            Constraint::Min(0),    // Spacer
        ])
        .margin(2)
        .split(area);

    let state = &app.edit_relay_url_state;

    // Current URL info
    let current_url = app.backend.relay_url();
    let info_para = Paragraph::new(format!(
        "{}: {}",
        app.i18n.t("settings.current"),
        current_url
    ))
    .style(Style::default().fg(app.theme.fg_secondary))
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(info_para, chunks[0]);

    // URL input
    let url_text = if app.input_mode == InputMode::Editing {
        format!("{}|", state.new_url)
    } else {
        state.new_url.clone()
    };
    let url_para = Paragraph::new(url_text)
        .style(Style::default().fg(app.theme.accent))
        .block(
            Block::default()
                .title(app.i18n.t("settings.new_relay"))
                .borders(Borders::ALL),
        );
    f.render_widget(url_para, chunks[1]);

    // Help text
    let help_para = Paragraph::new(app.i18n.t("settings.relay_help"))
        .style(Style::default().fg(app.theme.fg_secondary))
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(help_para, chunks[2]);
}
