// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Setup Screen UI
//!
//! Shown when no identity exists. Guides user to create or import an identity.
//! SP-21: Extended with a multi-step onboarding wizard.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, InputMode};

/// Helper to render a step progress indicator.
fn progress_indicator(step: u8, total: u8, theme_accent: Color) -> Paragraph<'static> {
    let dots: String = (1..=total)
        .map(|i| if i == step { "●" } else { "○" })
        .collect::<Vec<_>>()
        .join(" ");
    let text = format!("[{}/{}]  {}", step, total, dots);
    Paragraph::new(text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme_accent))
}

/// Draw the legacy setup screen (kept as fallback).
pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Spacer
            Constraint::Length(7), // Welcome box
            Constraint::Length(2), // Spacer
            Constraint::Length(5), // Create option
            Constraint::Length(5), // Import option
            Constraint::Min(0),    // Spacer
        ])
        .split(area);

    // Welcome message
    let welcome_text = vec![
        Line::from(Span::styled(
            app.i18n.t("welcome.title"),
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(app.i18n.t("welcome.subtitle")),
        Line::from(""),
        Line::from(Span::styled(
            app.i18n.t("setup.identity_required"),
            Style::default().fg(app.theme.warning),
        )),
    ];

    let welcome = Paragraph::new(welcome_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.accent))
                .title(format!(" {} ", app.i18n.t("setup.title"))),
        );
    f.render_widget(welcome, chunks[1]);

    // Create new identity option
    let create_text = vec![
        Line::from(Span::styled(
            format!("[c] {}", app.i18n.t("setup.create")),
            Style::default()
                .fg(app.theme.success)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            app.i18n.t("setup.create_description"),
            Style::default().fg(app.theme.fg_secondary),
        )),
    ];

    let create = Paragraph::new(create_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(create, chunks[3]);

    // Import backup option
    let import_text = vec![
        Line::from(Span::styled(
            format!("[i] {}", app.i18n.t("setup.import")),
            Style::default()
                .fg(app.theme.warning)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            app.i18n.t("setup.import_description"),
            Style::default().fg(app.theme.fg_secondary),
        )),
    ];

    let import = Paragraph::new(import_text)
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(import, chunks[4]);
}

// ============================================================================
// SP-21 Onboarding Wizard Steps
// ============================================================================

/// Step 1: Welcome screen with privacy highlights.
pub fn draw_welcome(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Top spacer
            Constraint::Length(9),  // Welcome card
            Constraint::Length(1),  // Spacer
            Constraint::Length(10), // Privacy highlights
            Constraint::Min(0),     // Spacer
            Constraint::Length(1),  // Progress
        ])
        .split(area);

    let welcome_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Welcome to Vauchi",
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Privacy-focused contact cards."),
        Line::from("Exchange in person. Update from anywhere."),
        Line::from(""),
        Line::from(Span::styled(
            "Press Enter to get started, or [i] to import a backup",
            Style::default().fg(app.theme.fg_secondary),
        )),
    ];

    let welcome = Paragraph::new(welcome_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.accent))
                .title(" Getting Started "),
        );
    f.render_widget(welcome, chunks[1]);

    let highlights = vec![
        Line::from(Span::styled(
            "Privacy by Design",
            Style::default()
                .fg(app.theme.success)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  * End-to-end encrypted contact cards"),
        Line::from("  * No phone number or email required"),
        Line::from("  * You control who sees what"),
        Line::from("  * Local-first: your data stays on your device"),
        Line::from("  * Decentralized: no single point of failure"),
        Line::from("  * Open source and auditable"),
    ];

    let highlights_para = Paragraph::new(highlights).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.fg_secondary))
            .title(" Why Vauchi? "),
    );
    f.render_widget(highlights_para, chunks[3]);

    f.render_widget(progress_indicator(1, 5, app.theme.accent), chunks[5]);
}

/// Step 2: Create identity (name input).
pub fn draw_create_identity(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Top spacer
            Constraint::Length(7), // Explanation card
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Name input
            Constraint::Length(1), // Spacer
            Constraint::Length(4), // Hint
            Constraint::Min(0),    // Spacer
            Constraint::Length(1), // Progress
        ])
        .split(area);

    let explain_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Create Your Identity",
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Choose a display name for your contact card."),
        Line::from("This is the name others will see when you exchange cards."),
    ];

    let explain = Paragraph::new(explain_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.accent))
                .title(" Step 2: Identity "),
        );
    f.render_widget(explain, chunks[1]);

    let name_display =
        if app.onboarding_state.name_input.is_empty() && app.input_mode == InputMode::Editing {
            "Type your name...|".to_string()
        } else if app.input_mode == InputMode::Editing {
            format!("{}|", app.onboarding_state.name_input)
        } else {
            app.onboarding_state.name_input.clone()
        };

    let name_para = Paragraph::new(name_display)
        .style(Style::default().fg(app.theme.warning))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.warning))
                .title(" Display Name "),
        );
    f.render_widget(name_para, chunks[3]);

    let hint = Paragraph::new(vec![
        Line::from(Span::styled(
            "You can change this later in Settings.",
            Style::default().fg(app.theme.fg_secondary),
        )),
        Line::from(""),
        Line::from("[Enter] continue  [Esc] back"),
    ])
    .alignment(Alignment::Center);
    f.render_widget(hint, chunks[5]);

    f.render_widget(progress_indicator(2, 5, app.theme.accent), chunks[7]);
}

/// Step 3: Add optional fields.
pub fn draw_add_fields(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2), // Top spacer
            Constraint::Length(7), // Explanation
            Constraint::Length(1), // Spacer
            Constraint::Min(5),    // Current fields
            Constraint::Length(3), // Action hints
            Constraint::Length(1), // Progress
        ])
        .split(area);

    let explain_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Add Contact Information",
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Add fields to your card: email, phone, website, etc."),
        Line::from("You can add, edit, or remove fields later anytime."),
    ];

    let explain = Paragraph::new(explain_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.accent))
                .title(" Step 3: Your Card "),
        );
    f.render_widget(explain, chunks[1]);

    // Show current fields
    let fields = app.backend.get_card_fields().unwrap_or_default();
    let fields_text = if fields.is_empty() {
        vec![
            Line::from(""),
            Line::from(Span::styled(
                "No fields yet. Press [a] to add one.",
                Style::default().fg(app.theme.fg_secondary),
            )),
        ]
    } else {
        let mut lines = vec![Line::from("")];
        for field in &fields {
            lines.push(Line::from(format!(
                "  {} ({}): {}",
                field.label, field.field_type, field.value
            )));
        }
        lines
    };

    let fields_para = Paragraph::new(fields_text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Fields ({}) ", fields.len())),
    );
    f.render_widget(fields_para, chunks[3]);

    let hints = Paragraph::new("[a] add field  [s/Enter] skip to next step  [Esc] back")
        .alignment(Alignment::Center)
        .style(Style::default().fg(app.theme.fg_secondary));
    f.render_widget(hints, chunks[4]);

    f.render_widget(progress_indicator(3, 5, app.theme.accent), chunks[5]);
}

/// Step 4: Security explanation.
pub fn draw_security(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // Top spacer
            Constraint::Length(15), // Security info
            Constraint::Length(1),  // Spacer
            Constraint::Length(3),  // Action hint
            Constraint::Min(0),     // Spacer
            Constraint::Length(1),  // Progress
        ])
        .split(area);

    let security_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Security Overview",
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "How Vauchi protects your data:",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  * Cards are encrypted end-to-end (E2E)"),
        Line::from("  * Keys never leave your device"),
        Line::from("  * Exchanges happen in person via QR code"),
        Line::from("  * Updates are pushed through an encrypted relay"),
        Line::from("  * You can verify contacts with fingerprints"),
        Line::from("  * Optional: set an app password for extra security"),
        Line::from("  * Optional: set a duress PIN (shows fake contacts)"),
        Line::from("  * You can back up and restore your identity"),
    ];

    let security = Paragraph::new(security_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.success))
            .title(" Step 4: Security "),
    );
    f.render_widget(security, chunks[1]);

    let hints = Paragraph::new("[Enter] continue  [Esc] back")
        .alignment(Alignment::Center)
        .style(Style::default().fg(app.theme.fg_secondary));
    f.render_widget(hints, chunks[3]);

    f.render_widget(progress_indicator(4, 5, app.theme.accent), chunks[5]);
}

/// Step 5: Ready / completion.
pub fn draw_ready(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // Top spacer
            Constraint::Length(7),  // Ready card
            Constraint::Length(1),  // Spacer
            Constraint::Length(10), // Shortcuts
            Constraint::Min(0),     // Spacer
            Constraint::Length(1),  // Progress
        ])
        .split(area);

    let ready_text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "You're All Set!",
            Style::default()
                .fg(app.theme.success)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Your identity has been created and is ready to use."),
        Line::from("Exchange cards in person, and stay connected."),
    ];

    let ready = Paragraph::new(ready_text)
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.success))
                .title(" Step 5: Ready! "),
        );
    f.render_widget(ready, chunks[1]);

    let shortcuts = vec![
        Line::from(Span::styled(
            "Quick Reference",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  [X] Exchange cards     (meet someone in person)"),
        Line::from("  [c] View contacts      (see who you've met)"),
        Line::from("  [a] Add field           (phone, email, etc.)"),
        Line::from("  [s] Settings            (name, relay, themes)"),
        Line::from("  [n] Sync               (push/pull updates)"),
        Line::from("  [?] Help               (full keyboard reference)"),
    ];

    let shortcuts_para = Paragraph::new(shortcuts).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Keyboard Shortcuts "),
    );
    f.render_widget(shortcuts_para, chunks[3]);

    f.render_widget(progress_indicator(5, 5, app.theme.accent), chunks[5]);
}
