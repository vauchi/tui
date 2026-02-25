// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! UI Rendering

mod backup;
mod contacts;
mod devices;
pub mod exchange;
mod gdpr;
mod help;
mod home;
mod recovery;
mod settings;
mod setup;
mod support;
mod sync;
mod tor;
mod visibility;
mod widgets;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, Screen};

/// Draw the application.
pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer/status
        ])
        .split(f.area());

    // Header
    draw_header(f, chunks[0], app);

    // Content
    match app.screen {
        Screen::Setup => setup::draw(f, chunks[1], app),
        Screen::Home => home::draw(f, chunks[1], app),
        Screen::Contacts => contacts::draw(f, chunks[1], app),
        Screen::ContactDetail => contacts::draw_detail(f, chunks[1], app),
        Screen::ContactVisibility => visibility::draw(f, chunks[1], app),
        Screen::Exchange => exchange::draw(f, chunks[1], app),
        Screen::Settings => settings::draw(f, chunks[1], app),
        Screen::Help => help::draw(f, chunks[1], app),
        Screen::AddField => home::draw_add_field(f, chunks[1], app),
        Screen::EditField => home::draw_edit_field(f, chunks[1], app),
        Screen::EditName => settings::draw_edit_name(f, chunks[1], app),
        Screen::EditRelayUrl => settings::draw_edit_relay_url(f, chunks[1], app),
        Screen::Devices => devices::draw(f, chunks[1], app),
        Screen::Recovery => recovery::draw(f, chunks[1], app),
        Screen::Sync => sync::draw(f, chunks[1], app),
        Screen::Backup => backup::draw(f, chunks[1], app),
        Screen::TorSettings => tor::draw(f, chunks[1], app),
        Screen::Privacy => gdpr::draw(f, chunks[1], app),
        Screen::Support => support::draw(f, chunks[1], app),
    }

    // Footer
    draw_footer(f, chunks[2], app);
}

fn draw_header(f: &mut Frame, area: Rect, app: &App) {
    let title = match app.screen {
        Screen::Setup => "Vauchi - Setup",
        Screen::Home => "Vauchi",
        Screen::Contacts => "Contacts",
        Screen::ContactDetail => "Contact Details",
        Screen::ContactVisibility => "Visibility Settings",
        Screen::Exchange => "Exchange",
        Screen::Settings => "Settings",
        Screen::Help => "Help",
        Screen::AddField => "Add Field",
        Screen::EditField => "Edit Field",
        Screen::EditName => "Edit Display Name",
        Screen::EditRelayUrl => "Edit Relay URL",
        Screen::Devices => "Devices",
        Screen::Recovery => "Recovery",
        Screen::Sync => "Sync",
        Screen::Backup => "Backup & Restore",
        Screen::TorSettings => "Tor Privacy",
        Screen::Privacy => "Privacy & Data",
        Screen::Support => "Support Vauchi",
    };

    let header = Paragraph::new(title)
        .style(
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));

    f.render_widget(header, area);
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let help_text = match app.screen {
        Screen::Setup => "[c]reate new identity  [i]mport backup  [q]uit",
        Screen::Home => "[c]ontacts  [s]ettings  [d]evices  [r]ecovery  sy[n]c  [b]ackup  [a]dd  [e]dit  [x]del  [?]help  [q]uit",
        Screen::Contacts => "[j/k] navigate  [enter] view  [d]elete  [v]erify  [esc] back  [?]help",
        Screen::ContactDetail => "[v]isibility  [x]delete  [esc] back  [?]help",
        Screen::ContactVisibility => "[j/k] navigate  [enter/space] toggle  [esc] back",
        Screen::Exchange => "[r]efresh  [esc] back  [?]help",
        Screen::Settings => "[n]ame  [b]ackup  [d]evices  [r]ecovery  [p]rivacy  [s]upport  [←/→] theme  [esc] back  [?]help",
        Screen::Help => "[esc/q] close",
        Screen::AddField => "[tab] next  [enter] submit  [esc] cancel",
        Screen::EditField => "[enter] save  [esc] cancel",
        Screen::EditName => "[enter] save  [esc] cancel",
        Screen::EditRelayUrl => "[enter] save  [esc] cancel",
        Screen::Devices => "[j/k] navigate  [l]ink new device  [esc] back  [?]help",
        Screen::Recovery => "[c]laim  [v]ouch  [s]tatus  [esc] back  [?]help",
        Screen::Sync => "[s]ync now  [esc] back  [?]help",
        Screen::Backup => "[e]xport  [i]mport  [esc] back  [?]help",
        Screen::TorSettings => "[e]nable  [d]isable  [o]nion  [n]ew circuit  [x]clear bridges  [esc] back",
        Screen::Privacy => "[e]xport  [d]elete  [c]ancel  [j/k] navigate  [space] toggle consent  [esc] back",
        Screen::Support => "[1] GitHub Sponsors  [2] Liberapay  [esc] back",
    };

    let status = if let Some(msg) = &app.status_message {
        format!("{} | {}", msg, help_text)
    } else {
        help_text.to_string()
    };

    let footer = Paragraph::new(status)
        .style(Style::default().fg(app.theme.fg_secondary))
        .block(Block::default().borders(Borders::TOP));

    f.render_widget(footer, area);
}
