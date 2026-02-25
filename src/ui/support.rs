// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Support Vauchi Screen
//!
//! Displays funding links and how donations are used.

use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table, Wrap};

use crate::app::App;

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Description
            Constraint::Length(5), // Funding links
            Constraint::Min(8),    // Where Funds Go table
            Constraint::Length(1), // Footer hint
        ])
        .margin(1)
        .split(area);

    // Description
    let desc = Paragraph::new(app.i18n.t("support.description"))
        .style(Style::default().fg(app.theme.fg))
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .title(app.i18n.t("support.title"))
                .borders(Borders::ALL),
        );
    f.render_widget(desc, chunks[0]);

    // Funding platform links
    let links = Paragraph::new(vec![
        Line::from(""),
        Line::from(format!(
            "  [1] {}  https://github.com/sponsors/vauchi",
            app.i18n.t("support.github_sponsors")
        )),
        Line::from(format!(
            "  [2] {}        https://liberapay.com/Vauchi/donate",
            app.i18n.t("support.liberapay")
        )),
    ])
    .style(Style::default().fg(app.theme.accent));
    f.render_widget(links, chunks[1]);

    // Where Funds Go table
    let header = Row::new(vec!["", ""]).style(
        Style::default()
            .fg(app.theme.accent)
            .add_modifier(Modifier::BOLD),
    );
    let rows = vec![
        Row::new(vec![
            app.i18n.t("support.category_hardware"),
            app.i18n.t("support.purpose_hardware"),
        ]),
        Row::new(vec![
            app.i18n.t("support.category_infrastructure"),
            app.i18n.t("support.purpose_infrastructure"),
        ]),
        Row::new(vec![
            app.i18n.t("support.category_security"),
            app.i18n.t("support.purpose_security"),
        ]),
        Row::new(vec![
            app.i18n.t("support.category_development"),
            app.i18n.t("support.purpose_development"),
        ]),
    ];
    let table = Table::new(rows, [Constraint::Length(20), Constraint::Min(40)])
        .header(header)
        .block(
            Block::default()
                .title(app.i18n.t("support.where_funds_go"))
                .borders(Borders::ALL),
        );
    f.render_widget(table, chunks[2]);

    // Footer hint
    let footer = Paragraph::new("  [1]/[2] open in browser  |  [Esc] back")
        .style(Style::default().fg(app.theme.fg_secondary));
    f.render_widget(footer, chunks[3]);
}
