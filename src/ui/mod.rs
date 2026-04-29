// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! UI Rendering

pub(crate) mod focus;
pub(crate) mod widgets;

// INLINE_TEST_REQUIRED: Widgets are pub(crate) — snapshot tests must live inside the crate to access them
#[cfg(test)]
mod widget_snapshots;

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use vauchi_app::ui::{ScreenModel, WorkflowEngine};

use crate::app::{App, Screen};
use crate::ui::focus::FocusZone;
use crate::ui::widgets::action_bar::{ActionBarWidget, ActionItem};
use crate::ui::widgets::nav_bar::{NavBarWidget, NavItem};
use crate::ui::widgets::screen_renderer;
use vauchi_app::ui::AppScreen;

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
const MIN_WIDTH: u16 = 60;
const MIN_HEIGHT: u16 = 16;

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
    app.ensure_engine_synced();

    // Cache screen models once per frame to avoid redundant allocations
    let cached = FrameScreenModels {
        app: Some(app.app_engine.current_screen()),
        onboarding: app.onboarding_engine.as_ref().map(|e| e.current_screen()),
        lock: app.lock_engine.as_ref().map(|e| e.current_screen()),
    };

    let current = app.current_app_screen();
    let has_search = current == AppScreen::Contacts && !app.contact_search_query.is_empty();
    let has_status = app.status_message.is_some() || has_search;
    let is_onboarding = current == AppScreen::Onboarding;
    let show_nav_bar = !is_onboarding && current != AppScreen::Lock;

    let mut constraints = vec![
        Constraint::Min(0), // Content
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

    // Content (chunk[0] — header removed). Engine-driven base layer
    // chosen by `current_app_screen()`; TUI-only overlays (ActionMenu,
    // ContactImport) layer on top via the legacy `Screen` enum until
    // they migrate to dedicated overlay state in a follow-up.
    match current {
        AppScreen::Onboarding => {
            if let Some(model) = &cached.onboarding {
                screen_renderer::render_screen(f, chunks[0], model, &app.render_state, &app.theme);
            } else {
                unreachable!("OnboardingEngine always initialized for setup screens");
            }
        }
        AppScreen::Lock => {
            if let Some(model) = &cached.lock {
                screen_renderer::render_screen(f, chunks[0], model, &app.render_state, &app.theme);
            }
        }
        _ => {
            render_cached_screen(f, chunks[0], cached.app.as_ref(), app);
        }
    }
    // TUI-only overlays on top of the engine screen.
    match app.screen {
        Screen::ActionMenu => draw_action_menu(f, chunks[0], app),
        Screen::ContactImport => draw_import_dialog(f, chunks[0], app),
        _ => {}
    }

    // Status message or search indicator (conditional)
    let mut bar_start = 1;
    if has_status {
        if let Some(msg) = &app.status_message {
            // Flash effect: highlight background for first 500ms
            let is_fresh = app
                .status_message_time
                .map(|t| t.elapsed() < std::time::Duration::from_millis(500))
                .unwrap_or(false);
            let style = if is_fresh {
                Style::default()
                    .fg(app.theme.bg)
                    .bg(app.theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.accent)
            };
            let text = if app.undo_action_id.is_some() {
                format!(" {}  [z] Undo", msg)
            } else {
                format!(" {}", msg)
            };
            let status = Paragraph::new(text).style(style);
            f.render_widget(status, chunks[bar_start]);
        } else if has_search {
            let search_style = if app.contact_search_mode {
                Style::default()
                    .fg(app.theme.bg)
                    .bg(app.theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(app.theme.accent)
            };
            let indicator = format!(" Search: {}  [Esc] clear", app.contact_search_query);
            let status = Paragraph::new(indicator).style(search_style);
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

/// Build action items for the current screen from the engine ScreenModel.
fn build_action_items(app: &App, cached: &FrameScreenModels) -> Vec<ActionItem> {
    let screen_model = match app.current_app_screen() {
        AppScreen::Onboarding => cached.onboarding.as_ref(),
        AppScreen::Lock => cached.lock.as_ref(),
        _ => cached.app.as_ref(),
    };

    let mut items: Vec<ActionItem> = if let Some(model) = screen_model {
        // Add Space hint based on focused component type
        let focused = app.render_state.focused_component;
        let focused_is_toggle = model
            .components
            .get(focused)
            .map(|c| matches!(c, vauchi_app::ui::Component::ToggleList { .. }))
            .unwrap_or(false);
        let focused_is_field = model
            .components
            .get(focused)
            .map(|c| matches!(c, vauchi_app::ui::Component::FieldList { .. }))
            .unwrap_or(false);

        let mut action_items: Vec<ActionItem> = Vec::new();

        // Mode indicator for form screens
        let is_form = matches!(app.current_app_screen(), AppScreen::FormDialog { .. });
        if is_form {
            action_items.push(ActionItem::new("EDIT", "").with_active(true));
        }

        if focused_is_toggle {
            action_items.push(ActionItem::new("Space", "select"));
        } else if focused_is_field {
            action_items.push(ActionItem::new("Space", "toggle"));
        }

        // Collapse group filter actions into a single [g] item
        let mut group_filter_shown = false;
        for a in model.actions.iter().filter(|a| a.enabled) {
            if a.id.starts_with("filter_group") {
                if !group_filter_shown {
                    // Find the active group name (Primary style = active filter)
                    let active_group = model
                        .actions
                        .iter()
                        .find(|ga| {
                            ga.id.starts_with("filter_group:")
                                && ga.style == vauchi_app::ui::ActionStyle::Primary
                        })
                        .map(|ga| ga.label.as_str());
                    let label = match active_group {
                        Some(name) => format!("Group: {}", name),
                        None => "Groups".to_string(),
                    };
                    action_items
                        .push(ActionItem::new("g", &label).with_active(active_group.is_some()));
                    group_filter_shown = true;
                }
            } else {
                let key_str = screen_renderer::action_key_hint_pub(&a.id);
                let is_primary = a.style == vauchi_app::ui::ActionStyle::Primary;
                action_items.push(ActionItem::new(key_str, &a.label).with_active(is_primary));
            }
        }

        action_items
    } else {
        Vec::new()
    };

    // Add global actions (back/quit) except on Lock and first setup screen
    match app.screen {
        Screen::Lock => {}
        Screen::SetupWelcome => {
            items.push(ActionItem::new("q", "quit"));
        }
        Screen::SetupReady => {
            // No back on final screen
        }
        _ => {
            items.push(ActionItem::new("?", "help"));
            items.push(ActionItem::new("Esc", "back"));
            items.push(ActionItem::new("q", "quit"));
        }
    }

    items
}

/// Build navigation items for the persistent bottom nav bar.
fn build_nav_items(app: &App) -> Vec<NavItem> {
    let active_tab = match app.screen {
        Screen::MyInfo
        | Screen::MyInfoEntryDetail
        | Screen::AddField
        | Screen::EditField
        | Screen::EditName => 0,
        Screen::Contacts
        | Screen::ContactDetail
        | Screen::ContactEdit
        | Screen::ContactVisibility
        | Screen::ContactDuplicates
        | Screen::ContactMerge
        | Screen::ContactLimit
        | Screen::ContactImport => 1,
        Screen::Exchange => 2,
        Screen::Groups | Screen::GroupDetail => 3,
        Screen::More
        | Screen::Settings
        | Screen::Help
        | Screen::Devices
        | Screen::Recovery
        | Screen::Sync
        | Screen::Activity
        | Screen::Delivery
        | Screen::Backup
        | Screen::Privacy
        | Screen::Support
        | Screen::Emergency
        | Screen::Duress
        | Screen::DeviceReplacement
        | Screen::EditRelayUrl => 4,
        _ => 0, // Default to My Card
    };

    vec![
        NavItem {
            label: "My Card",
            active: active_tab == 0,
        },
        NavItem {
            label: "Contacts",
            active: active_tab == 1,
        },
        NavItem {
            label: "Exchange",
            active: active_tab == 2,
        },
        NavItem {
            label: "Groups",
            active: active_tab == 3,
        },
        NavItem {
            label: "More",
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
            .title(" Actions [Esc] dismiss ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.accent))
            .style(Style::default().bg(app.theme.bg)),
    );

    f.render_widget(list, popup_area);
}

/// Draw the contact import dialog (file path text input).
fn draw_import_dialog(f: &mut Frame, area: Rect, app: &App) {
    let popup_width = 60u16.min(area.width.saturating_sub(4));
    let popup_height = if app.import_state.result_message.is_some() {
        7
    } else {
        5
    };

    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    f.render_widget(Clear, popup_area);

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints(if app.import_state.result_message.is_some() {
            vec![
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ]
        } else {
            vec![Constraint::Length(1), Constraint::Length(1)]
        })
        .margin(1)
        .split(popup_area);

    let block = Block::default()
        .title(" Import vCard [Enter] import [Esc] cancel ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.accent))
        .style(Style::default().bg(app.theme.bg));
    f.render_widget(block, popup_area);

    let label = Paragraph::new("File path:").style(Style::default().fg(app.theme.fg));
    f.render_widget(label, inner[0]);

    let input_style = Style::default()
        .fg(app.theme.fg)
        .bg(app.theme.bg)
        .add_modifier(Modifier::UNDERLINED);
    let input = Paragraph::new(app.import_state.file_path.as_str()).style(input_style);
    f.render_widget(input, inner[1]);

    // Show cursor at end of input
    f.set_cursor_position((
        inner[1].x + app.import_state.file_path.len() as u16,
        inner[1].y,
    ));

    if let Some(msg) = &app.import_state.result_message {
        let style = if app.import_state.success {
            Style::default().fg(app.theme.accent)
        } else {
            Style::default().fg(Color::Red)
        };
        let result = Paragraph::new(msg.as_str()).style(style);
        f.render_widget(result, inner[2]);
    }
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
