// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! List-style component renderers — contact list, settings group, action list.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};

use crate::theme::TuiTheme;
use crate::ui::widgets::icon::icon_badge;
use vauchi_app::ui::SettingsItemKind;

use super::ScreenRenderState;

/// Render a list component (e.g. `Component::List`) with scrolling and selection indicator.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_contact_list(
    f: &mut Frame,
    area: Rect,
    items_data: &[vauchi_app::ui::Item],
    _searchable: bool,
    is_focused: bool,
    state: &ScreenRenderState,
    component_idx: usize,
    theme: &TuiTheme,
) {
    if items_data.is_empty() {
        // TODO(HUMBLE): W — Empty-state text names Exchange feature (see _private/docs/problems/2026-07-06-desktop-tui-web-domain-shell-violations)
        let empty = Paragraph::new("  No contacts yet. Use Exchange to add one.")
            .style(Style::default().fg(theme.fg_secondary))
            .block(
                Block::default()
                    .title(" Contacts ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(theme.border)),
            );
        f.render_widget(empty, area);
        return;
    }

    let selected = state.selection_for(component_idx);
    // Visible rows = area height minus 2 for borders
    let visible_count = area.height.saturating_sub(2) as usize;
    // Use stored scroll, but clamp so selection is always visible
    let scroll = {
        let s = state.scroll_for(component_idx);
        if selected < s {
            selected
        } else if visible_count > 0 && selected >= s + visible_count {
            selected.saturating_sub(visible_count - 1)
        } else {
            s
        }
    };

    let total = items_data.len();
    let end = (scroll + visible_count).min(total);

    let items: Vec<ListItem> = items_data[scroll..end]
        .iter()
        .enumerate()
        .map(|(vi, item)| {
            let actual_idx = scroll + vi;
            let prefix = if actual_idx == selected { "▸" } else { " " };
            let line = if let Some(sub) = &item.subtitle {
                format!("{} {} {}  {}", prefix, item.initials, item.name, sub)
            } else {
                format!("{} {} {}", prefix, item.initials, item.name)
            };
            let style = if actual_idx == selected && is_focused {
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };
            ListItem::new(line).style(style)
        })
        .collect();

    let border_color = if is_focused {
        theme.accent
    } else {
        theme.border
    };

    // Title with count and scroll indicator
    let title = if total > visible_count {
        format!(" Contacts ({}) ↕ ", total)
    } else {
        format!(" Contacts ({}) ", total)
    };

    let list = List::new(items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    );
    f.render_widget(list, area);
}

/// Render a settings group component.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_settings_group(
    f: &mut Frame,
    area: Rect,
    label: &str,
    items: &[vauchi_app::ui::SettingsItem],
    is_focused: bool,
    state: &ScreenRenderState,
    component_idx: usize,
    theme: &TuiTheme,
) {
    let selected = state.selection_for(component_idx);

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let right = match &item.kind {
                SettingsItemKind::Toggle { enabled } => {
                    if *enabled { "[x]" } else { "[ ]" }.to_string()
                }
                SettingsItemKind::Value { value } => value.clone(),
                SettingsItemKind::Link { detail } => detail.as_deref().unwrap_or("→").to_string(),
                SettingsItemKind::Destructive { label } => label.clone(),
                _ => String::new(),
            };

            let prefix = if i == selected && is_focused {
                "▸"
            } else {
                " "
            };
            let text = format!(" {} {}  {}", prefix, item.label, right);
            let style = if i == selected && is_focused {
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else if matches!(item.kind, SettingsItemKind::Destructive { .. }) {
                Style::default().fg(theme.error)
            } else {
                Style::default().fg(theme.fg)
            };
            ListItem::new(text).style(style)
        })
        .collect();

    let border_color = if is_focused {
        theme.accent
    } else {
        theme.border
    };
    let list = List::new(list_items).block(
        Block::default()
            .title(format!(" {} ", label))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    );
    f.render_widget(list, area);
}

/// Render a sectioned action list component — grouped action rows with
/// section headers (`Section.label` in dim/bold), each item indented
/// underneath. Selection cursors across the full flat item index so
/// up/down navigation crosses section boundaries naturally.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_sectioned_action_list(
    f: &mut Frame,
    area: Rect,
    sections: &[vauchi_app::ui::Section],
    is_focused: bool,
    state: &ScreenRenderState,
    component_idx: usize,
    theme: &TuiTheme,
) {
    let selected = state.selection_for(component_idx);

    // Flatten sections into renderable lines: header → items → header → items.
    // Track the cumulative item index (across sections) so the selected
    // cursor lines up with the flat `UserAction::ListItemSelected` index.
    let mut list_items: Vec<ListItem> = Vec::new();
    let mut flat_item_idx: usize = 0;
    for section in sections {
        // Section header — dim + bold.
        let header = format!(" {}", section.label);
        list_items.push(
            ListItem::new(header).style(
                Style::default()
                    .fg(theme.fg_secondary)
                    .add_modifier(Modifier::BOLD),
            ),
        );

        for item in &section.items {
            let icon = item.icon.as_deref().and_then(icon_badge).unwrap_or("•");
            let detail = item
                .detail
                .as_ref()
                .map(|d| format!("  {}", d))
                .unwrap_or_default();
            let prefix = if flat_item_idx == selected && is_focused {
                "▸"
            } else {
                " "
            };
            let text = format!("   {} {} {}{}", prefix, icon, item.label, detail);
            let style = if flat_item_idx == selected && is_focused {
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };
            list_items.push(ListItem::new(text).style(style));
            flat_item_idx += 1;
        }
    }

    let border_color = if is_focused {
        theme.accent
    } else {
        theme.border
    };
    let list = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    );
    f.render_widget(list, area);
}

/// Render an action list component.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_action_list(
    f: &mut Frame,
    area: Rect,
    items: &[vauchi_app::ui::ActionListItem],
    is_focused: bool,
    state: &ScreenRenderState,
    component_idx: usize,
    theme: &TuiTheme,
) {
    let selected = state.selection_for(component_idx);

    let list_items: Vec<ListItem> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let icon = item.icon.as_deref().and_then(icon_badge).unwrap_or("•");
            let detail = item
                .detail
                .as_ref()
                .map(|d| format!("  {}", d))
                .unwrap_or_default();
            let prefix = if i == selected && is_focused {
                "▸"
            } else {
                " "
            };
            let text = format!(" {} {} {}{}", prefix, icon, item.label, detail);
            let style = if i == selected && is_focused {
                Style::default()
                    .fg(theme.bg)
                    .bg(theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.fg)
            };
            ListItem::new(text).style(style)
        })
        .collect();

    let border_color = if is_focused {
        theme.accent
    } else {
        theme.border
    };
    let list = List::new(list_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color)),
    );
    f.render_widget(list, area);
}
