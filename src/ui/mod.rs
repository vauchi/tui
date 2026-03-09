// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! UI Rendering

#[allow(dead_code)]
mod contacts;
mod duplicates;
pub mod exchange;
#[allow(dead_code)]
mod home;
mod lock;
#[allow(dead_code)]
mod settings;
mod setup;
pub(crate) mod widgets;

// INLINE_TEST_REQUIRED: Widgets are pub(crate) — snapshot tests must live inside the crate to access them
#[cfg(test)]
mod widget_snapshots;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use vauchi_core::ui::{ScreenModel, WorkflowEngine};

use crate::app::{App, Screen};
use crate::ui::widgets::screen_renderer;

/// Cached screen models computed once per frame to avoid redundant allocations.
struct FrameScreenModels {
    app: Option<ScreenModel>,
    onboarding: Option<ScreenModel>,
    lock: Option<ScreenModel>,
}

/// Helper: renders a pre-computed AppEngine screen model.
fn render_cached_screen(
    f: &mut Frame,
    area: Rect,
    screen_model: Option<&ScreenModel>,
    app: &App,
) -> bool {
    if let Some(model) = screen_model {
        screen_renderer::render_screen(f, area, model, &app.render_state, &app.theme);
        true
    } else {
        false
    }
}

/// Draw the application.
pub fn draw(f: &mut Frame, app: &mut App) {
    // Sync AppEngine to match the current TUI screen before rendering
    if let Some(target) = app.to_app_screen() {
        if let Some(engine) = &mut app.app_engine {
            if *engine.current_app_screen() != target {
                engine.navigate_to(target);
            }
        }
    }

    // Cache screen models once per frame to avoid redundant allocations
    let cached = FrameScreenModels {
        app: app.app_engine.as_ref().map(|e| e.current_screen()),
        onboarding: app.onboarding_engine.as_ref().map(|e| e.current_screen()),
        lock: app.lock_engine.as_ref().map(|e| e.current_screen()),
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Content
            Constraint::Length(3), // Footer/status
        ])
        .split(f.area());

    // Header
    draw_header(f, chunks[0], app, &cached);

    // Content
    match app.screen {
        Screen::Setup => setup::draw(f, chunks[1], app),
        // Engine-driven screens — rendered via AppEngine ScreenModel
        Screen::Home
        | Screen::Contacts
        | Screen::ContactDetail
        | Screen::ContactVisibility
        | Screen::Exchange
        | Screen::Settings
        | Screen::Help
        | Screen::Devices
        | Screen::Recovery
        | Screen::Sync
        | Screen::Delivery
        | Screen::Backup
        | Screen::TorSettings
        | Screen::Privacy
        | Screen::Support
        | Screen::Emergency
        | Screen::Duress
        | Screen::Groups
        | Screen::GroupDetail => {
            render_cached_screen(f, chunks[1], cached.app.as_ref(), app);
        }
        // Form dialogs — TUI-specific text input, not engine-driven
        Screen::AddField => home::draw_add_field(f, chunks[1], app),
        Screen::EditField => home::draw_edit_field(f, chunks[1], app),
        Screen::EditName => settings::draw_edit_name(f, chunks[1], app),
        Screen::EditRelayUrl => settings::draw_edit_relay_url(f, chunks[1], app),
        Screen::Lock => {
            if let Some(model) = &cached.lock {
                screen_renderer::render_screen(f, chunks[1], model, &app.render_state, &app.theme);
            } else {
                lock::draw(f, chunks[1], app);
            }
        }
        Screen::ActionMenu => {
            // Draw contact detail underneath, then overlay action menu
            contacts::draw_detail(f, chunks[1], app);
            draw_action_menu(f, chunks[1], app);
        }
        // SP-21 Onboarding wizard — engine-driven rendering
        Screen::SetupWelcome
        | Screen::SetupCreateIdentity
        | Screen::SetupAddFields
        | Screen::SetupSecurity
        | Screen::SetupReady => {
            if let Some(model) = &cached.onboarding {
                screen_renderer::render_screen(f, chunks[1], model, &app.render_state, &app.theme);
            } else {
                unreachable!("OnboardingEngine always initialized for setup screens");
            }
        }
        // SP-12a Duplicates / Merge / Limit
        Screen::ContactDuplicates => duplicates::draw_duplicates(f, chunks[1], app),
        Screen::ContactMerge => duplicates::draw_merge(f, chunks[1], app),
        Screen::ContactLimit => duplicates::draw_limit(f, chunks[1], app),
    }

    // Footer
    draw_footer(f, chunks[2], app, &cached);
}

fn draw_header(f: &mut Frame, area: Rect, app: &App, cached: &FrameScreenModels) {
    // Use engine title for engine-driven screens (from cached models)
    let engine_title: Option<String> = match app.screen {
        Screen::Home
        | Screen::Contacts
        | Screen::ContactDetail
        | Screen::Exchange
        | Screen::Settings
        | Screen::Help
        | Screen::Backup
        | Screen::Delivery
        | Screen::Devices
        | Screen::Duress
        | Screen::Emergency
        | Screen::Sync
        | Screen::TorSettings
        | Screen::Recovery
        | Screen::Groups
        | Screen::GroupDetail
        | Screen::ContactVisibility
        | Screen::Privacy
        | Screen::Support => cached.app.as_ref().map(|m| m.title.clone()),
        Screen::SetupWelcome
        | Screen::SetupCreateIdentity
        | Screen::SetupAddFields
        | Screen::SetupSecurity
        | Screen::SetupReady => cached
            .onboarding
            .as_ref()
            .map(|m| format!("Vauchi - {}", m.title)),
        Screen::Lock => cached.lock.as_ref().map(|m| m.title.clone()),
        _ => None,
    };

    let title = engine_title.as_deref().unwrap_or(match app.screen {
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
    });

    let header = Paragraph::new(title)
        .style(
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::BOTTOM));

    f.render_widget(header, area);
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App, cached: &FrameScreenModels) {
    // For engine-driven screens, show engine actions in footer (from cached models)
    let engine_footer: Option<String> = match app.screen {
        Screen::Home
        | Screen::Contacts
        | Screen::ContactDetail
        | Screen::Exchange
        | Screen::Settings
        | Screen::Help
        | Screen::Backup
        | Screen::Delivery
        | Screen::Devices
        | Screen::Duress
        | Screen::Emergency
        | Screen::Sync
        | Screen::TorSettings
        | Screen::Recovery
        | Screen::Groups
        | Screen::GroupDetail
        | Screen::ContactVisibility
        | Screen::Privacy
        | Screen::Support => cached.app.as_ref().map(|screen| {
            let actions = screen
                .actions
                .iter()
                .filter(|a| a.enabled)
                .map(|a| {
                    let key = screen_renderer::action_key_hint_pub(&a.id);
                    format!("[{}] {}", key, a.label)
                })
                .collect::<Vec<_>>()
                .join("  ");
            format!("{}  [Esc] back  [q]uit", actions)
        }),
        Screen::SetupWelcome
        | Screen::SetupCreateIdentity
        | Screen::SetupAddFields
        | Screen::SetupSecurity
        | Screen::SetupReady => cached.onboarding.as_ref().map(|screen| {
            screen
                .actions
                .iter()
                .filter(|a| a.enabled)
                .map(|a| {
                    let key = screen_renderer::action_key_hint_pub(&a.id);
                    format!("[{}] {}", key, a.label)
                })
                .collect::<Vec<_>>()
                .join("  ")
                + "  [Esc] back  [q]uit"
        }),
        Screen::Lock => cached.lock.as_ref().map(|screen| {
            screen
                .actions
                .iter()
                .filter(|a| a.enabled)
                .map(|a| {
                    let key = screen_renderer::action_key_hint_pub(&a.id);
                    format!("[{}] {}", key, a.label)
                })
                .collect::<Vec<_>>()
                .join("  ")
        }),
        _ => None,
    };

    let help_text = engine_footer.as_deref().unwrap_or(match app.screen {
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
    });

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
