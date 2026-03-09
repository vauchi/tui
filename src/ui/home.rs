// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Home Screen

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::app::{AddFieldFocus, App, InputMode};
use crate::backend::FIELD_TYPES;

/// Icon for a social network ID (used in the picker list).
fn social_icon(network_id: &str) -> &'static str {
    match network_id {
        "twitter" => "🐦",
        "instagram" => "📸",
        "facebook" => "👤",
        "linkedin" => "💼",
        "github" => "🐙",
        "gitlab" => "🦊",
        "youtube" => "🎬",
        "twitch" => "🎮",
        "reddit" => "🗨",
        "mastodon" => "🐘",
        "telegram" => "✈",
        "discord" => "💬",
        "tiktok" => "🎵",
        "spotify" => "🎧",
        "bluesky" => "☁",
        _ => "🔗",
    }
}

/// Renders the home screen showing the user's identity card, fields, and contact count.
pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Welcome
            Constraint::Length(3), // Public ID
            Constraint::Min(0),    // Fields
            Constraint::Length(2), // Contact count
        ])
        .split(area);

    // Welcome message
    let name = app.display_name().unwrap_or("Guest");
    let welcome = if app.backend.has_identity() {
        app.i18n.t_args("home.greeting", &[("name", name)])
    } else {
        app.i18n.t("welcome.title")
    };

    let welcome_para = Paragraph::new(welcome)
        .style(
            Style::default()
                .fg(app.theme.fg)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::NONE));
    f.render_widget(welcome_para, chunks[0]);

    // Public ID
    if let Some(id) = app.backend.public_id() {
        let id_para = Paragraph::new(format!("{}: {}", app.i18n.t("home.public_id"), id))
            .style(Style::default().fg(app.theme.fg_secondary));
        f.render_widget(id_para, chunks[1]);
    }

    // Card fields
    let fields = app.backend.get_card_fields().unwrap_or_default();

    if fields.is_empty() {
        let empty = Paragraph::new(app.i18n.t("home.no_fields"))
            .style(Style::default().fg(app.theme.fg_secondary));
        f.render_widget(empty, chunks[2]);
    } else {
        let items: Vec<ListItem> = fields
            .iter()
            .enumerate()
            .map(|(i, field)| {
                let icon = match field.field_type.as_str() {
                    "Email" => "📧",
                    "Phone" => "📱",
                    "Website" => "🌐",
                    "Address" => "📍",
                    "Social" => "🔗",
                    _ => "📝",
                };

                let content = format!(
                    "{} {} ({})  {}",
                    icon, field.label, field.field_type, field.value
                );
                let style = if i == app.selected_field {
                    Style::default()
                        .fg(app.theme.warning)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(content).style(style)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .title(app.i18n.t("card.title"))
                .borders(Borders::ALL),
        );

        let mut state = ListState::default();
        state.select(Some(app.selected_field));
        f.render_stateful_widget(list, chunks[2], &mut state);
    }

    // Contact count
    let count = app.backend.contact_count().unwrap_or(0);
    let count_text = app
        .i18n
        .t_args("contacts.count", &[("count", &count.to_string())]);
    let count_para = Paragraph::new(count_text).style(Style::default().fg(app.theme.fg_secondary));
    f.render_widget(count_para, chunks[3]);
}

/// Renders the add-field dialog, dispatching to social or generic field forms.
pub fn draw_add_field(f: &mut Frame, area: Rect, app: &App) {
    let is_social = FIELD_TYPES[app.add_field_state.field_type_index] == "Social";

    if is_social {
        draw_add_social_field(f, area, app);
    } else {
        draw_add_generic_field(f, area, app);
    }
}

/// Draw the standard add-field dialog (non-social types).
fn draw_add_generic_field(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Type selector
            Constraint::Length(3), // Label input
            Constraint::Length(3), // Value input
            Constraint::Min(0),    // Spacer
        ])
        .margin(2)
        .split(area);

    let state = &app.add_field_state;

    // Type selector
    draw_type_selector(f, chunks[0], app);

    // Label input
    let label_style = if state.focus == AddFieldFocus::Label {
        Style::default().fg(app.theme.warning)
    } else {
        Style::default()
    };
    let label_text = if state.label.is_empty() && state.focus != AddFieldFocus::Label {
        app.i18n.t("card.enter_label")
    } else if state.focus == AddFieldFocus::Label && app.input_mode == InputMode::Editing {
        format!("{}|", state.label)
    } else {
        state.label.clone()
    };
    let label_para = Paragraph::new(label_text).style(label_style).block(
        Block::default()
            .title(app.i18n.t("card.label"))
            .borders(Borders::ALL),
    );
    f.render_widget(label_para, chunks[1]);

    // Value input
    draw_value_input(f, chunks[2], app);
}

/// Draw the social field dialog with a network picker.
fn draw_add_social_field(f: &mut Frame, area: Rect, app: &App) {
    let state = &app.add_field_state;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Type selector
            Constraint::Min(8),    // Network picker
            Constraint::Length(3), // Value input (username)
            Constraint::Length(2), // Help text
        ])
        .margin(2)
        .split(area);

    // Type selector
    draw_type_selector(f, chunks[0], app);

    // Network picker
    let picker = &state.social_picker;
    let picker_style = if state.focus == AddFieldFocus::Network {
        Style::default()
            .fg(app.theme.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(app.theme.fg_secondary)
    };

    if picker.networks.is_empty() {
        let empty = Paragraph::new("No networks available")
            .style(Style::default().fg(app.theme.fg_secondary))
            .block(
                Block::default()
                    .title("Social Network")
                    .borders(Borders::ALL),
            );
        f.render_widget(empty, chunks[1]);
    } else {
        let items: Vec<ListItem> = picker
            .networks
            .iter()
            .enumerate()
            .map(|(i, (id, display_name))| {
                let icon = social_icon(id);
                let content = format!(" {} {}", icon, display_name);
                let style = if i == picker.selected && state.focus == AddFieldFocus::Network {
                    Style::default()
                        .fg(app.theme.bg)
                        .bg(app.theme.accent)
                        .add_modifier(Modifier::BOLD)
                } else if i == picker.selected {
                    picker_style
                } else {
                    Style::default().fg(app.theme.fg)
                };
                ListItem::new(content).style(style)
            })
            .collect();

        let border_style = if state.focus == AddFieldFocus::Network {
            Style::default().fg(app.theme.accent)
        } else {
            Style::default().fg(app.theme.fg_secondary)
        };

        let list = List::new(items).block(
            Block::default()
                .title("Social Network [j/k to navigate, Enter to select]")
                .borders(Borders::ALL)
                .border_style(border_style),
        );

        let mut list_state = ListState::default();
        list_state.select(Some(picker.selected));
        f.render_stateful_widget(list, chunks[1], &mut list_state);
    }

    // Value input (username/handle)
    let value_title = if !state.label.is_empty() {
        format!("Username ({})", state.label)
    } else {
        "Username".to_string()
    };
    let value_style = if state.focus == AddFieldFocus::Value {
        Style::default().fg(app.theme.warning)
    } else {
        Style::default()
    };
    let value_text = if state.value.is_empty() && state.focus != AddFieldFocus::Value {
        "Enter username or handle".to_string()
    } else if state.focus == AddFieldFocus::Value && app.input_mode == InputMode::Editing {
        format!("{}|", state.value)
    } else {
        state.value.clone()
    };
    let value_para = Paragraph::new(value_text)
        .style(value_style)
        .block(Block::default().title(value_title).borders(Borders::ALL));
    f.render_widget(value_para, chunks[2]);

    // Help text
    let help_text = match state.focus {
        AddFieldFocus::Type => "[←/→] change type  [Tab/Enter] next  [Esc] cancel",
        AddFieldFocus::Network => "[j/k] navigate  [Enter] select  [Tab] skip  [Esc] cancel",
        AddFieldFocus::Label | AddFieldFocus::Value => "[Enter] submit  [Tab] next  [Esc] cancel",
    };
    let help = Paragraph::new(help_text).style(Style::default().fg(app.theme.fg_secondary));
    f.render_widget(help, chunks[3]);
}

/// Draw the field type selector (shared between social and generic).
fn draw_type_selector(f: &mut Frame, area: Rect, app: &App) {
    let state = &app.add_field_state;
    let type_text = format!("< {} >", FIELD_TYPES[state.field_type_index]);
    let type_style = if state.focus == AddFieldFocus::Type {
        Style::default()
            .fg(app.theme.warning)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let type_para = Paragraph::new(type_text).style(type_style).block(
        Block::default()
            .title(app.i18n.t("card.field_type"))
            .borders(Borders::ALL),
    );
    f.render_widget(type_para, area);
}

/// Draw the value input field (shared between social and generic).
fn draw_value_input(f: &mut Frame, area: Rect, app: &App) {
    let state = &app.add_field_state;
    let value_style = if state.focus == AddFieldFocus::Value {
        Style::default().fg(app.theme.warning)
    } else {
        Style::default()
    };
    let value_text = if state.value.is_empty() && state.focus != AddFieldFocus::Value {
        app.i18n.t("card.enter_value")
    } else if state.focus == AddFieldFocus::Value && app.input_mode == InputMode::Editing {
        format!("{}|", state.value)
    } else {
        state.value.clone()
    };
    let value_para = Paragraph::new(value_text).style(value_style).block(
        Block::default()
            .title(app.i18n.t("card.value"))
            .borders(Borders::ALL),
    );
    f.render_widget(value_para, area);
}

/// Renders the edit-field dialog for modifying an existing field's value.
pub fn draw_edit_field(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Field info
            Constraint::Length(3), // Value input
            Constraint::Min(0),    // Spacer
        ])
        .margin(2)
        .split(area);

    let state = &app.edit_field_state;

    // Field info (read-only)
    let info_text = format!("{} ({})", state.field_label, state.field_type);
    let info_para = Paragraph::new(info_text)
        .style(Style::default().fg(app.theme.fg_secondary))
        .block(
            Block::default()
                .title(app.i18n.t("card.field"))
                .borders(Borders::ALL),
        );
    f.render_widget(info_para, chunks[0]);

    // Value input
    let value_text = if app.input_mode == InputMode::Editing {
        format!("{}|", state.new_value)
    } else {
        state.new_value.clone()
    };
    let value_para = Paragraph::new(value_text)
        .style(Style::default().fg(app.theme.warning))
        .block(
            Block::default()
                .title(app.i18n.t("card.new_value"))
                .borders(Borders::ALL),
        );
    f.render_widget(value_para, chunks[1]);
}
