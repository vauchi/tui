// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Component-level visual regression tests for TUI widgets.
//!
//! Each widget is rendered into a `TestBackend` and snapshotted with `insta`.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;

use vauchi_app::ui::{
    Field, InfoItem, PreviewVariant, ToggleItem, UiFieldVisibility, VisibilityMode,
};

use crate::theme::TuiTheme;
use crate::ui::widgets::card_preview::CardPreviewWidget;
use crate::ui::widgets::field_list::FieldListWidget;
use crate::ui::widgets::info_panel::InfoPanelWidget;
use crate::ui::widgets::text_input::TextInputWidget;
use crate::ui::widgets::toggle_list::ToggleListWidget;

/// Convert a ratatui buffer to a plain-text string for snapshot comparison.
fn buffer_to_string(buf: &Buffer) -> String {
    let mut s = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            let cell = &buf[(x, y)];
            s.push_str(cell.symbol());
        }
        s.push('\n');
    }
    s
}

fn test_theme() -> TuiTheme {
    TuiTheme::default()
}

// ── TextInput ──────────────────────────────────────────────

#[test]
fn text_input_empty_focused() {
    let theme = test_theme();
    let backend = TestBackend::new(40, 6);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            TextInputWidget {
                label: "Display Name",
                value: "",
                placeholder: Some("Enter your name"),
                validation_error: None,
                focused: true,
                theme: &theme,
            }
            .render(f, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    insta::assert_snapshot!("text_input_empty_focused", buffer_to_string(&buf));
}

#[test]
// @scenario: accessibility.feature:Contact details are fully announced
fn text_input_with_value() {
    let theme = test_theme();
    let backend = TestBackend::new(40, 6);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            TextInputWidget {
                label: "Display Name",
                value: "Alice",
                placeholder: None,
                validation_error: None,
                focused: false,
                theme: &theme,
            }
            .render(f, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    insta::assert_snapshot!("text_input_with_value", buffer_to_string(&buf));
}

#[test]
// @scenario: accessibility.feature:Notifications are announced
// @scenario: accessibility.feature:Helpful error messages
fn text_input_with_error() {
    let theme = test_theme();
    let backend = TestBackend::new(40, 6);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            TextInputWidget {
                label: "Display Name",
                value: "",
                placeholder: None,
                validation_error: Some("Name is required"),
                focused: true,
                theme: &theme,
            }
            .render(f, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    insta::assert_snapshot!("text_input_with_error", buffer_to_string(&buf));
}

// ── ToggleList ─────────────────────────────────────────────

#[test]
// @scenario: accessibility.feature:Contact list is navigable with screen reader
fn toggle_list_mixed_selection() {
    let theme = test_theme();
    let items = vec![
        ToggleItem {
            id: "family".into(),
            label: "Family".into(),
            selected: true,
            subtitle: Some("Close relatives".into()),
            a11y: None,
            info_key: None,
        },
        ToggleItem {
            id: "friends".into(),
            label: "Friends".into(),
            selected: false,
            subtitle: None,
            a11y: None,
            info_key: None,
        },
        ToggleItem {
            id: "work".into(),
            label: "Coworkers".into(),
            selected: true,
            subtitle: Some("Professional contacts".into()),
            a11y: None,
            info_key: None,
        },
    ];

    let backend = TestBackend::new(40, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            ToggleListWidget {
                label: "Groups",
                items: &items,
                selected_index: 1,
                focused: true,
                theme: &theme,
            }
            .render(f, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    insta::assert_snapshot!("toggle_list_mixed_selection", buffer_to_string(&buf));
}

#[test]
fn toggle_list_unfocused() {
    let theme = test_theme();
    let items = vec![
        ToggleItem {
            id: "a".into(),
            label: "Alpha".into(),
            selected: false,
            subtitle: None,
            a11y: None,
            info_key: None,
        },
        ToggleItem {
            id: "b".into(),
            label: "Beta".into(),
            selected: true,
            subtitle: None,
            a11y: None,
            info_key: None,
        },
    ];

    let backend = TestBackend::new(40, 8);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            ToggleListWidget {
                label: "Options",
                items: &items,
                selected_index: 0,
                focused: false,
                theme: &theme,
            }
            .render(f, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    insta::assert_snapshot!("toggle_list_unfocused", buffer_to_string(&buf));
}

// ── FieldList ──────────────────────────────────────────────

#[test]
fn field_list_show_hide_mode() {
    let theme = test_theme();
    let fields = vec![
        Field {
            id: "1".into(),
            field_type: "Email".into(),
            label: "Personal".into(),
            value: "alice@example.com".into(),
            visibility: UiFieldVisibility::Shown,
            a11y: None,
        },
        Field {
            id: "2".into(),
            field_type: "Phone".into(),
            label: "Mobile".into(),
            value: "+41 79 123 45 67".into(),
            visibility: UiFieldVisibility::Hidden,
            a11y: None,
        },
    ];

    let backend = TestBackend::new(70, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            FieldListWidget {
                fields: &fields,
                visibility_mode: &VisibilityMode::ShowHide,
                available_groups: &[],
                selected_index: 0,
                focused: true,
                theme: &theme,
            }
            .render(f, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    insta::assert_snapshot!("field_list_show_hide_mode", buffer_to_string(&buf));
}

#[test]
fn field_list_per_group_mode() {
    let theme = test_theme();
    let fields = vec![
        Field {
            id: "1".into(),
            field_type: "Email".into(),
            label: "Work".into(),
            value: "alice@corp.com".into(),
            visibility: UiFieldVisibility::Groups(vec!["Coworkers".into()]),
            a11y: None,
        },
        Field {
            id: "2".into(),
            field_type: "Phone".into(),
            label: "Home".into(),
            value: "+41 79 000 00 00".into(),
            visibility: UiFieldVisibility::Groups(vec![]),
            a11y: None,
        },
    ];

    let backend = TestBackend::new(70, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            FieldListWidget {
                fields: &fields,
                visibility_mode: &VisibilityMode::PerGroup,
                available_groups: &["Coworkers".into(), "Family".into()],
                selected_index: 1,
                focused: false,
                theme: &theme,
            }
            .render(f, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    insta::assert_snapshot!("field_list_per_group_mode", buffer_to_string(&buf));
}

#[test]
fn field_list_empty() {
    let theme = test_theme();

    let backend = TestBackend::new(70, 6);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            FieldListWidget {
                fields: &[],
                visibility_mode: &VisibilityMode::ShowHide,
                available_groups: &[],
                selected_index: 0,
                focused: false,
                theme: &theme,
            }
            .render(f, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    insta::assert_snapshot!("field_list_empty", buffer_to_string(&buf));
}

// ── CardPreview ────────────────────────────────────────────

#[test]
fn card_preview_no_groups() {
    let theme = test_theme();
    let fields = vec![
        Field {
            id: "1".into(),
            field_type: "Email".into(),
            label: "Personal".into(),
            value: "alice@example.com".into(),
            visibility: UiFieldVisibility::Shown,
            a11y: None,
        },
        Field {
            id: "2".into(),
            field_type: "Phone".into(),
            label: "Mobile".into(),
            value: "+41 79 123 45 67".into(),
            visibility: UiFieldVisibility::Shown,
            a11y: None,
        },
    ];

    let backend = TestBackend::new(50, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            CardPreviewWidget {
                name: "Alice",
                fields: &fields,
                variants: &[],
                selected_variant: None,
                theme: &theme,
            }
            .render(f, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    insta::assert_snapshot!("card_preview_no_groups", buffer_to_string(&buf));
}

#[test]
fn card_preview_with_groups() {
    let theme = test_theme();
    let variants = vec![
        PreviewVariant {
            variant_id: "Family".into(),
            display_name: "Alice".into(),
            visible_fields: vec![Field {
                id: "1".into(),
                field_type: "Phone".into(),
                label: "Mobile".into(),
                value: "+41 79 123 45 67".into(),
                visibility: UiFieldVisibility::Shown,
                a11y: None,
            }],
        },
        PreviewVariant {
            variant_id: "Coworkers".into(),
            display_name: "A. Smith".into(),
            visible_fields: vec![Field {
                id: "2".into(),
                field_type: "Email".into(),
                label: "Work".into(),
                value: "asmith@corp.com".into(),
                visibility: UiFieldVisibility::Shown,
                a11y: None,
            }],
        },
    ];

    let backend = TestBackend::new(50, 12);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            CardPreviewWidget {
                name: "Alice",
                fields: &[],
                variants: &variants,
                selected_variant: Some("Coworkers"),
                theme: &theme,
            }
            .render(f, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    insta::assert_snapshot!("card_preview_with_groups", buffer_to_string(&buf));
}

#[test]
fn card_preview_no_fields() {
    let theme = test_theme();

    let backend = TestBackend::new(50, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            CardPreviewWidget {
                name: "Empty User",
                fields: &[],
                variants: &[],
                selected_variant: None,
                theme: &theme,
            }
            .render(f, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    insta::assert_snapshot!("card_preview_no_fields", buffer_to_string(&buf));
}

// ── InfoPanel ──────────────────────────────────────────────

#[test]
fn info_panel_with_icons() {
    let theme = test_theme();
    let items = vec![
        InfoItem {
            icon: Some("lock".into()),
            title: "End-to-end encrypted".into(),
            detail: "Your data is always encrypted.".into(),
        },
        InfoItem {
            icon: Some("shield".into()),
            title: "No tracking".into(),
            detail: "We never track your usage.".into(),
        },
        InfoItem {
            icon: None,
            title: "Open source".into(),
            detail: "Fully auditable code.".into(),
        },
    ];

    let backend = TestBackend::new(50, 14);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            InfoPanelWidget {
                title: "Security",
                icon: Some("shield"),
                items: &items,
                theme: &theme,
            }
            .render(f, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    insta::assert_snapshot!("info_panel_with_icons", buffer_to_string(&buf));
}

#[test]
// @scenario: accessibility.feature:Screen reader announces app structure on desktop
fn info_panel_no_icon() {
    let theme = test_theme();
    let items = vec![InfoItem {
        icon: None,
        title: "Welcome".into(),
        detail: "Get started with Vauchi.".into(),
    }];

    let backend = TestBackend::new(50, 8);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|f| {
            let area = f.area();
            InfoPanelWidget {
                title: "Getting Started",
                icon: None,
                items: &items,
                theme: &theme,
            }
            .render(f, area);
        })
        .unwrap();

    let buf = terminal.backend().buffer().clone();
    insta::assert_snapshot!("info_panel_no_icon", buffer_to_string(&buf));
}
