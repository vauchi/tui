// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! UI Rendering

pub mod exchange;
pub(crate) mod focus;
mod lock;
pub(crate) mod widgets;

// INLINE_TEST_REQUIRED: Widgets are pub(crate) — snapshot tests must live inside the crate to access them
#[cfg(test)]
mod widget_snapshots;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use vauchi_core::ui::{ScreenModel, WorkflowEngine};

use crate::app::{App, Screen};
use crate::ui::focus::FocusZone;
use crate::ui::widgets::action_bar::{ActionBarWidget, ActionItem};
use crate::ui::widgets::nav_bar::{NavBarWidget, NavItem};
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

/// Minimum terminal size for usable rendering.
const MIN_WIDTH: u16 = 40;
const MIN_HEIGHT: u16 = 12;

/// Draw the application.
pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        let msg = format!(
            "Terminal too small ({}x{}). Need {}x{}.",
            area.width, area.height, MIN_WIDTH, MIN_HEIGHT
        );
        let paragraph = Paragraph::new(msg)
            .style(Style::default().fg(app.theme.fg_secondary))
            .alignment(Alignment::Center);
        f.render_widget(paragraph, area);
        return;
    }

    // Auto-clear stale status messages
    app.tick_status();

    // Sync AppEngine to match the current TUI screen before rendering
    if let Some(target) = app.to_app_screen() {
        if *app.app_engine.current_app_screen() != target {
            app.app_engine.navigate_to(target);
        }
    }

    // Cache screen models once per frame to avoid redundant allocations
    let cached = FrameScreenModels {
        app: Some(app.app_engine.current_screen()),
        onboarding: app.onboarding_engine.as_ref().map(|e| e.current_screen()),
        lock: app.lock_engine.as_ref().map(|e| e.current_screen()),
    };

    let has_status = app.status_message.is_some();
    let is_onboarding = matches!(
        app.screen,
        Screen::SetupWelcome
            | Screen::SetupCreateIdentity
            | Screen::SetupAddFields
            | Screen::SetupSecurity
            | Screen::SetupReady
    );
    let show_nav_bar = !is_onboarding && app.screen != Screen::Lock;

    let mut constraints = vec![
        Constraint::Length(3), // Header
        Constraint::Min(0),    // Content
    ];
    if has_status {
        constraints.push(Constraint::Length(1)); // Status message
    }
    constraints.push(Constraint::Length(1)); // Action bar
    if show_nav_bar {
        constraints.push(Constraint::Length(1)); // Nav bar
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(f.area());

    // Build action items from the current screen's ScreenModel
    let action_items = build_action_items(app, &cached);
    let nav_items = if show_nav_bar {
        build_nav_items(app)
    } else {
        Vec::new()
    };

    // Update focus manager counts each frame
    let content_count = cached.app.as_ref().map(|m| m.components.len()).unwrap_or(0);
    app.focus
        .set_counts(content_count, action_items.len(), nav_items.len());

    // Sync content focus state so components dim when bar zones are active
    app.render_state.content_has_focus = app.focus.zone == FocusZone::Content;

    // Header
    draw_header(f, chunks[0], app, &cached);

    // Content
    match app.screen {
        // Engine-driven screens — rendered via AppEngine ScreenModel
        Screen::Home
        | Screen::Contacts
        | Screen::ContactDetail
        | Screen::ContactEdit
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
        | Screen::GroupDetail
        | Screen::AddField
        | Screen::EditField
        | Screen::EditName
        | Screen::EditRelayUrl
        | Screen::ContactDuplicates
        | Screen::ContactMerge
        | Screen::ContactLimit => {
            render_cached_screen(f, chunks[1], cached.app.as_ref(), app);
        }
        Screen::Lock => {
            if let Some(model) = &cached.lock {
                screen_renderer::render_screen(f, chunks[1], model, &app.render_state, &app.theme);
            } else {
                lock::draw(f, chunks[1], app);
            }
        }
        Screen::ActionMenu => {
            // Draw engine-driven contact detail underneath, then overlay action menu
            render_cached_screen(f, chunks[1], cached.app.as_ref(), app);
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
    }

    // Status message (conditional)
    let mut bar_start = 2;
    if has_status {
        if let Some(msg) = &app.status_message {
            let status =
                Paragraph::new(format!(" {}", msg)).style(Style::default().fg(app.theme.accent));
            f.render_widget(status, chunks[bar_start]);
        }
        bar_start += 1;
    }

    // Action bar
    draw_action_bar(f, chunks[bar_start], app, &action_items);

    // Nav bar (hidden during onboarding and lock screen)
    if show_nav_bar {
        draw_nav_bar(f, chunks[bar_start + 1], app, &nav_items);
    }

    // Alert modal overlay (requires user dismissal with Esc/Enter)
    if let Some((title, message)) = &app.alert_message {
        draw_alert_modal(f, area, app, title, message);
    }
}

fn draw_header(f: &mut Frame, area: Rect, app: &App, cached: &FrameScreenModels) {
    // Use engine title for engine-driven screens (from cached models)
    let engine_title: Option<String> = match app.screen {
        Screen::Home
        | Screen::Contacts
        | Screen::ContactDetail
        | Screen::ContactEdit
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
        | Screen::Support
        | Screen::EditName
        | Screen::EditField
        | Screen::EditRelayUrl
        | Screen::AddField
        | Screen::ContactDuplicates
        | Screen::ContactMerge
        | Screen::ContactLimit => cached.app.as_ref().map(|m| m.title.clone()),
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
        Screen::Home => "Vauchi",
        Screen::Contacts => "Contacts",
        Screen::ContactDetail => "Contact Details",
        Screen::ContactEdit => "Edit Contact",
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

/// Build action items for the current screen from the engine ScreenModel.
fn build_action_items(app: &App, cached: &FrameScreenModels) -> Vec<ActionItem> {
    let screen_model = match app.screen {
        Screen::SetupWelcome
        | Screen::SetupCreateIdentity
        | Screen::SetupAddFields
        | Screen::SetupSecurity
        | Screen::SetupReady => cached.onboarding.as_ref(),
        Screen::Lock => cached.lock.as_ref(),
        _ => cached.app.as_ref(),
    };

    let mut items: Vec<ActionItem> = if let Some(model) = screen_model {
        model
            .actions
            .iter()
            .filter(|a| a.enabled)
            .map(|a| {
                let key_str = screen_renderer::action_key_hint_pub(&a.id);
                let key = key_str.chars().next().unwrap_or('?');
                ActionItem::new(key, &a.label)
            })
            .collect()
    } else {
        Vec::new()
    };

    // Add global actions (back/quit) except on Lock and first setup screen
    match app.screen {
        Screen::Lock => {}
        Screen::SetupWelcome => {
            items.push(ActionItem::new('q', "quit"));
        }
        Screen::SetupReady => {
            // No back on final screen
        }
        _ => {
            items.push(ActionItem::new('\u{241B}', "back")); // ␛ for Esc
            items.push(ActionItem::new('q', "quit"));
        }
    }

    items
}

/// Build navigation items for the persistent bottom nav bar.
fn build_nav_items(app: &App) -> Vec<NavItem> {
    let active_tab = match app.screen {
        Screen::Exchange => 0,
        Screen::Contacts
        | Screen::ContactDetail
        | Screen::ContactEdit
        | Screen::ContactVisibility
        | Screen::ContactDuplicates
        | Screen::ContactMerge
        | Screen::ContactLimit => 1,
        Screen::Home => 2,
        Screen::Settings
        | Screen::Devices
        | Screen::Recovery
        | Screen::Sync
        | Screen::Delivery
        | Screen::Backup
        | Screen::TorSettings
        | Screen::Privacy
        | Screen::Support
        | Screen::Emergency
        | Screen::Duress => 3,
        Screen::Help => 4,
        _ => 2, // Default to Home
    };

    vec![
        NavItem {
            label: "Exchange",
            active: active_tab == 0,
        },
        NavItem {
            label: "Contacts",
            active: active_tab == 1,
        },
        NavItem {
            label: "Home",
            active: active_tab == 2,
        },
        NavItem {
            label: "Settings",
            active: active_tab == 3,
        },
        NavItem {
            label: "Help",
            active: active_tab == 4,
        },
    ]
}

/// Render the action bar using ActionBarWidget.
fn draw_action_bar(f: &mut Frame, area: Rect, app: &App, items: &[ActionItem]) {
    let widget = ActionBarWidget {
        items,
        focused_index: app.focus.focused_in(FocusZone::ActionBar),
        theme: &app.theme,
    };
    widget.render(f, area);
}

/// Render the nav bar using NavBarWidget.
fn draw_nav_bar(f: &mut Frame, area: Rect, app: &App, items: &[NavItem]) {
    let widget = NavBarWidget {
        items,
        focused_index: app.focus.focused_in(FocusZone::NavBar),
        theme: &app.theme,
    };
    widget.render(f, area);
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

/// Draw a modal alert dialog centered on screen. Dismissed by Esc/Enter.
fn draw_alert_modal(f: &mut Frame, area: Rect, app: &App, title: &str, message: &str) {
    let lines: Vec<&str> = message.lines().collect();
    let max_line_len = lines.iter().map(|l| l.len()).max().unwrap_or(20);
    let popup_width = ((max_line_len + 6) as u16).clamp(30, area.width.saturating_sub(4));
    let popup_height = ((lines.len() + 4) as u16).clamp(5, area.height.saturating_sub(2));

    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let text = format!("{message}\n\n[Esc/Enter] Dismiss");
    let paragraph = Paragraph::new(text)
        .style(Style::default().fg(app.theme.fg).bg(app.theme.bg))
        .block(
            Block::default()
                .title(format!(" {} ", title))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.accent)),
        )
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(paragraph, popup_area);
}
