// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! UI Rendering

mod backup;
mod contacts;
mod delivery;
mod devices;
mod duplicates;
mod duress;
mod emergency;
pub mod exchange;
mod gdpr;
mod groups;
mod help;
mod home;
mod lock;
mod recovery;
mod settings;
mod setup;
mod support;
mod sync;
mod tor;
mod visibility;
pub(crate) mod widgets;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};

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
        Screen::Delivery => delivery::draw(f, chunks[1], app),
        Screen::Backup => backup::draw(f, chunks[1], app),
        Screen::TorSettings => tor::draw(f, chunks[1], app),
        Screen::Privacy => gdpr::draw(f, chunks[1], app),
        Screen::Support => support::draw(f, chunks[1], app),
        Screen::Emergency => emergency::draw(f, chunks[1], app),
        Screen::Duress => duress::draw(f, chunks[1], app),
        Screen::Groups => groups::draw(f, chunks[1], app),
        Screen::GroupDetail => groups::draw_detail(f, chunks[1], app),
        Screen::Lock => lock::draw(f, chunks[1], app),
        Screen::ActionMenu => {
            // Draw contact detail underneath, then overlay action menu
            contacts::draw_detail(f, chunks[1], app);
            draw_action_menu(f, chunks[1], app);
        }
        // SP-21 Onboarding wizard
        Screen::SetupWelcome => setup::draw_welcome(f, chunks[1], app),
        Screen::SetupCreateIdentity => setup::draw_create_identity(f, chunks[1], app),
        Screen::SetupAddFields => setup::draw_add_fields(f, chunks[1], app),
        Screen::SetupSecurity => setup::draw_security(f, chunks[1], app),
        Screen::SetupReady => setup::draw_ready(f, chunks[1], app),
        // SP-12a Duplicates / Merge / Limit
        Screen::ContactDuplicates => duplicates::draw_duplicates(f, chunks[1], app),
        Screen::ContactMerge => duplicates::draw_merge(f, chunks[1], app),
        Screen::ContactLimit => duplicates::draw_limit(f, chunks[1], app),
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
        Screen::Delivery => "Delivery",
        Screen::Backup => "Backup & Restore",
        Screen::TorSettings => "Tor Privacy",
        Screen::Privacy => "Privacy & Data",
        Screen::Support => "Support Vauchi",
        Screen::Emergency => "Emergency Broadcast",
        Screen::Duress => "Duress Protection",
        Screen::Groups => "Contact Groups",
        Screen::GroupDetail => "Group Details",
        Screen::Lock => "Vauchi",
        Screen::ActionMenu => "Contact Details",
        Screen::SetupWelcome => "Vauchi - Welcome",
        Screen::SetupCreateIdentity => "Vauchi - Create Identity",
        Screen::SetupAddFields => "Vauchi - Add Fields",
        Screen::SetupSecurity => "Vauchi - Security",
        Screen::SetupReady => "Vauchi - Ready!",
        Screen::ContactDuplicates => "Duplicate Detection",
        Screen::ContactMerge => "Merge Contacts",
        Screen::ContactLimit => "Contact Limit",
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
        Screen::Home => "[c]ontacts  [s]ettings  [g]roups  e[X]change  [a]dd  [e]dit  [x]del  [?]help  [q]uit",
        Screen::Contacts => "[j/k] navigate  [/]search  [d]uplicates  [L]imit  [enter] view  [esc] back  [?]help",
        Screen::ContactDetail => "[j/k] navigate  [c]opy  [o]pen  [v]isibility  [f]ingerprint  [t]rust  [h]ide  [x]delete  [esc] back",
        Screen::ContactVisibility => "[j/k] navigate  [enter/space] toggle  [esc] back",
        Screen::Exchange => "[r]efresh  [esc] back  [?]help",
        Screen::Settings => "[n]ame  [u]relay  [t]or  [b]ackup  [d]evices  [r]ecovery  [e]mergency  [D]uress  [p]rivacy  [s]upport  [esc] back",
        Screen::Help => "[esc/q] close",
        Screen::AddField => "[tab] next  [enter] submit  [esc] cancel",
        Screen::EditField => "[enter] save  [esc] cancel",
        Screen::EditName => "[enter] save  [esc] cancel",
        Screen::EditRelayUrl => "[enter] save  [esc] cancel",
        Screen::Devices => "[j/k] navigate  [l]ink new device  [r]evoke  [esc] back  [?]help",
        Screen::Recovery => "[c]laim  [v]ouch  [s]tatus  [esc] back  [?]help",
        Screen::Sync => "[s]ync now  [t]est connection  [r]efresh pending  [esc] back  [?]help",
        Screen::Delivery => "[r]etry  [c]leanup  [esc] back",
        Screen::Backup => "[e]xport  [i]mport  [esc] back  [?]help",
        Screen::TorSettings => "[e]nable  [d]isable  [o]nion  [n]ew circuit  [x]clear bridges  [esc] back",
        Screen::Privacy => "[j/k] navigate  [e]xport  [d]elete  [c]ancel  [space] toggle consent  [esc] back",
        Screen::Support => "[1] GitHub Sponsors  [2] Liberapay  [esc] back",
        Screen::Emergency => "[c]onfigure  [s]end  [l]ocation toggle  [x]disable  [esc] back",
        Screen::Duress => "[p]in setup  [a]lert config  [l]ocation  [x]disable  [esc] back",
        Screen::Groups => "[j/k] navigate  [/]search  [n]ew group  [enter] view  [d]elete  [esc] back",
        Screen::GroupDetail => "[j/k] navigate  [r]ename  [esc] back",
        Screen::Lock => "[enter] unlock",
        Screen::ActionMenu => "[j/k] navigate  [enter] select  [esc] cancel",
        Screen::SetupWelcome => "[Enter] start  [i]mport backup  [q]uit",
        Screen::SetupCreateIdentity => "[Enter] continue  [Esc] back",
        Screen::SetupAddFields => "[a]dd field  [s/Enter] skip  [Esc] back",
        Screen::SetupSecurity => "[Enter] continue  [Esc] back",
        Screen::SetupReady => "[Enter] go to Home",
        Screen::ContactDuplicates => "[j/k] navigate  [m]erge  [d]ismiss  [esc] back",
        Screen::ContactMerge => "[y] confirm merge  [n/Esc] cancel",
        Screen::ContactLimit => "[e/Enter] edit  [Esc] back",
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

/// Draw the action menu popup centered in the given area.
fn draw_action_menu(f: &mut Frame, area: Rect, app: &App) {
    let actions = &app.action_menu_state.actions;
    if actions.is_empty() {
        return;
    }

    // Calculate popup size
    let max_label_len = actions
        .iter()
        .map(|(label, _)| label.len())
        .max()
        .unwrap_or(20);
    let popup_width = (max_label_len as u16 + 6).min(area.width.saturating_sub(4));
    let popup_height = (actions.len() as u16 + 2).min(area.height.saturating_sub(2));

    // Center the popup
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    // Clear background
    f.render_widget(Clear, popup_area);

    // Build list items
    let items: Vec<ListItem> = actions
        .iter()
        .enumerate()
        .map(|(i, (label, _))| {
            let style = if i == app.action_menu_state.selected {
                Style::default()
                    .fg(app.theme.bg)
                    .bg(app.theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.fg)
            };
            ListItem::new(format!("  {} ", label)).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title(" Actions ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.accent))
            .style(Style::default().bg(app.theme.bg)),
    );

    f.render_widget(list, popup_area);
}
