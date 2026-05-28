// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests for `screen_renderer` — extracted from mod.rs to keep both
//! files under the per-file size limit. Acts as the `tests` child
//! module of `screen_renderer`, referenced from mod.rs via a
//! `cfg(test)` mod-decl.

// INLINE_TEST_REQUIRED: tests need access to pub(crate) screen_renderer
// internals (private helpers like `component_padding_lines` and the
// private `render_components` entrypoint); a tests/ integration target
// cannot see them.

use super::*;

// @internal
#[test]
fn test_render_state_default() {
    let state = ScreenRenderState::default();
    assert_eq!(state.focused_component, 0);
    assert!(state.component_selections.is_empty());
    assert!(state.validation_errors.is_empty());
}

// @internal
#[test]
fn test_render_state_ensure_capacity() {
    let mut state = ScreenRenderState::default();
    state.ensure_capacity(3);
    assert_eq!(state.component_selections.len(), 3);
    assert_eq!(state.selection_for(0), 0);
    assert_eq!(state.selection_for(1), 0);
    assert_eq!(state.selection_for(2), 0);
}

// @internal
#[test]
fn test_render_state_selection_for_out_of_bounds() {
    let state = ScreenRenderState::default();
    assert_eq!(state.selection_for(99), 0);
}

// @internal
#[test]
fn test_render_state_validation_errors() {
    let mut state = ScreenRenderState::default();

    // No error initially
    assert!(state.validation_error_for("name").is_none());

    // Set an error
    state.set_validation_error("name".to_string(), "Required".to_string());
    assert_eq!(state.validation_error_for("name"), Some("Required"));

    // Update the error
    state.set_validation_error("name".to_string(), "Too short".to_string());
    assert_eq!(state.validation_error_for("name"), Some("Too short"));
    assert_eq!(state.validation_errors.len(), 1);

    // Clear the error
    state.clear_validation_error("name");
    assert!(state.validation_error_for("name").is_none());
}

// @internal
#[test]
fn test_render_state_clear_all_errors() {
    let mut state = ScreenRenderState::default();
    state.set_validation_error("a".to_string(), "err1".to_string());
    state.set_validation_error("b".to_string(), "err2".to_string());
    assert_eq!(state.validation_errors.len(), 2);

    state.clear_all_errors();
    assert!(state.validation_errors.is_empty());
}

// @internal
#[test]
fn test_spacing_from_tokens_uses_screen_model_values() {
    // Validates architecture: ScreenModel.tokens flows to render layout
    use vauchi_app::DesignTokens;

    let tokens = DesignTokens::default();
    // Token gap: spacing.sm (8px) / 16px line height → 1 line (min 1)
    assert_eq!(component_padding_lines(&tokens), 1);
    // Content minimum: spacing.xl (32px) / 16px → 2, clamped to min 3
    assert_eq!(content_min_lines(&tokens), 3);
}

// @internal
#[test]
fn test_spacing_from_tokens_respects_custom_values() {
    use vauchi_app::DesignTokens;

    let mut tokens = DesignTokens::default();
    // 16px → 1 line (16/16 = 1)
    tokens.spacing.sm = 16;
    assert_eq!(component_padding_lines(&tokens), 1);

    // 32px → 2 lines
    tokens.spacing.sm = 32;
    assert_eq!(component_padding_lines(&tokens), 2);

    // 48px → 3 lines
    tokens.spacing.sm = 48;
    assert_eq!(component_padding_lines(&tokens), 3);
}

/// Render `components` into a fresh `TestBackend` of size
/// `width × height` and return the buffer text. Mirrors the helper
/// in `dropdown.rs` so renderer dispatch can be exercised in isolation.
fn render_components_to_text(width: u16, height: u16, components: &[Component]) -> String {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    let theme = TuiTheme::default();
    let tokens = DesignTokens::default();
    let state = ScreenRenderState::default();
    terminal
        .draw(|f| {
            render_components(f, f.area(), components, &state, &theme, &tokens);
        })
        .unwrap();
    let buf = terminal.backend().buffer().clone();
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

/// `Component::AvatarPreview` must render *something* identifiable
/// — historically dropped by the catch-all `_` arm in render_components.
/// For TUI a placeholder showing the initials is acceptable.
// @internal
#[test]
fn render_avatar_preview_emits_visible_initials() {
    let components = vec![Component::AvatarPreview {
        id: "avatar".into(),
        image_data: None,
        initials: "AB".into(),
        bg_color: None,
        brightness: 0.0,
        editable: false,
        a11y: None,
    }];
    let text = render_components_to_text(40, 3, &components);
    assert!(
        text.contains("AB") || text.contains("Avatar"),
        "expected avatar surface, got: {text:?}"
    );
}

/// `Component::Slider` must render label + current value — historically
/// dropped by the catch-all `_` arm in render_components.
// @internal
#[test]
fn render_slider_emits_label_and_value() {
    let components = vec![Component::Slider {
        id: "brightness".into(),
        label: "Brightness".into(),
        value: 0.5,
        min: 0.0,
        max: 1.0,
        step: 0.1,
        min_icon: None,
        max_icon: None,
        a11y: None,
    }];
    let text = render_components_to_text(40, 3, &components);
    assert!(text.contains("Brightness"), "expected label, got: {text:?}");
    assert!(
        text.contains("0.5") || text.contains("0.50"),
        "expected current value, got: {text:?}"
    );
}

/// `Component::Indicator` must render its label visibly and pick
/// a kind-driven glyph. Mirrors the iOS chip semantics (filled
/// checkmark / warning / circle / progress) at terminal granularity.
// @internal
#[test]
fn render_indicator_emits_label_and_kind_glyph() {
    use vauchi_app::ui::IndicatorKind;
    for (kind, expected_any_of) in [
        (IndicatorKind::Active, vec!["●", "✓"]),
        (IndicatorKind::Error, vec!["✗", "⚠"]),
        (IndicatorKind::Neutral, vec!["○"]),
        (IndicatorKind::Busy, vec!["◉", "…"]),
    ] {
        let components = vec![Component::Indicator {
            id: "ind".into(),
            label: "Syncing".into(),
            kind,
            action_id: None,
            a11y: None,
        }];
        let text = render_components_to_text(40, 3, &components);
        assert!(
            text.contains("Syncing"),
            "Indicator({kind:?}): expected label, got: {text:?}"
        );
        assert!(
            expected_any_of.iter().any(|g| text.contains(g)),
            "Indicator({kind:?}): expected one of {expected_any_of:?}, got: {text:?}"
        );
    }
}

/// `Component::SectionedActionList` must render every section
/// label plus every item label — losing a section header degrades
/// the rendering to a flat `ActionList` and the discriminant becomes
/// invisible to the user.
// @internal
#[test]
fn render_sectioned_action_list_emits_all_sections_and_items() {
    use vauchi_app::ui::{ActionListItem, Section};
    let components = vec![Component::SectionedActionList {
        id: "more".into(),
        sections: vec![
            Section {
                id: "primary".into(),
                label: "Primary".into(),
                items: vec![ActionListItem {
                    id: "profile".into(),
                    label: "Profile".into(),
                    icon: None,
                    detail: None,
                    a11y: None,
                    info_key: None,
                }],
            },
            Section {
                id: "data".into(),
                label: "Data".into(),
                items: vec![ActionListItem {
                    id: "backup".into(),
                    label: "Backup".into(),
                    icon: None,
                    detail: None,
                    a11y: None,
                    info_key: None,
                }],
            },
        ],
    }];
    let text = render_components_to_text(60, 10, &components);
    for needle in ["Primary", "Profile", "Data", "Backup"] {
        assert!(
            text.contains(needle),
            "SectionedActionList: expected {needle:?}, got: {text:?}"
        );
    }
}

/// Build one labelled instance of every currently-known
/// `Component` variant. Mirrors the enum in
/// `core/vauchi-app/src/ui/component.rs` — keep in sync with
/// `every_known_component_variant_renders_non_empty` below.
fn every_known_component() -> Vec<Component> {
    use vauchi_app::ui::{
        ActionListItem, DropdownOption, Field, IndicatorKind, InfoItem, InputType, Item, QrMode,
        Section, SettingsItem, SettingsItemKind, Status, ToggleItem, UiFieldVisibility,
        VisibilityMode,
    };
    vec![
        Component::Text {
            id: "text".into(),
            content: "Body text".into(),
            style: TextStyle::Body,
        },
        Component::TextInput {
            id: "input".into(),
            label: "Name".into(),
            value: "Alice".into(),
            placeholder: None,
            max_length: None,
            validation_error: None,
            input_type: InputType::Text,
            a11y: None,
            info_key: None,
        },
        Component::ToggleList {
            id: "toggles".into(),
            label: "Groups".into(),
            items: vec![ToggleItem {
                id: "fam".into(),
                label: "Family".into(),
                selected: true,
                subtitle: None,
                a11y: None,
                info_key: None,
            }],
            a11y: None,
        },
        Component::FieldList {
            id: "fields".into(),
            fields: vec![Field {
                id: "email".into(),
                field_type: "email".into(),
                label: "Email".into(),
                value: "a@b.com".into(),
                icon: String::new(),
                visibility: UiFieldVisibility::Shown,
                a11y: None,
            }],
            visibility_mode: VisibilityMode::ShowHide,
            available_groups: vec![],
            a11y: None,
        },
        Component::Preview {
            name: "Alice".into(),
            initials: "A".into(),
            fields: vec![],
            variants: vec![],
            selected_variant: None,
            visible_fields: vec![],
            avatar_data: None,
            a11y: None,
        },
        Component::InfoPanel {
            id: "info".into(),
            icon: None,
            title: "Security".into(),
            items: vec![InfoItem {
                icon: None,
                title: "E2E".into(),
                detail: "Encrypted".into(),
            }],
            a11y: None,
        },
        Component::List {
            id: "contacts".into(),
            items: vec![Item {
                id: "c1".into(),
                name: "Bob".into(),
                subtitle: None,
                avatar_initials: "B".into(),
                status: None,
                actions: vec![],
                a11y: None,
            }],
            searchable: false,
        },
        Component::SettingsGroup {
            id: "settings".into(),
            label: "Privacy".into(),
            items: vec![SettingsItem {
                id: "s1".into(),
                label: "Show toasts".into(),
                kind: SettingsItemKind::Toggle { enabled: true },
                a11y: None,
                info_key: None,
            }],
        },
        Component::ActionList {
            id: "actions".into(),
            items: vec![ActionListItem {
                id: "a1".into(),
                label: "Do thing".into(),
                icon: None,
                detail: None,
                a11y: None,
                info_key: None,
            }],
        },
        Component::StatusIndicator {
            id: "status".into(),
            icon: None,
            title: "Connected".into(),
            detail: Some("relay.example".into()),
            status: Status::Success,
            a11y: None,
        },
        Component::PinInput {
            id: "pin".into(),
            label: "PIN".into(),
            length: 6,
            filled: 0,
            masked: true,
            validation_error: None,
            a11y: None,
        },
        Component::QrCode {
            id: "qr".into(),
            data: "vauchi://test".into(),
            mode: QrMode::Display,
            label: Some("scan me".into()),
            scan_quality: None,
            a11y: None,
        },
        Component::InlineConfirm {
            id: "confirm".into(),
            warning: "Are you sure?".into(),
            confirm_text: "Yes".into(),
            cancel_text: "No".into(),
            destructive: false,
            a11y: None,
        },
        Component::EditableText {
            id: "edit".into(),
            label: "Display name".into(),
            value: "Alice".into(),
            editing: false,
            validation_error: None,
            a11y: None,
            info_key: None,
        },
        Component::Divider,
        Component::Banner {
            text: "Heads up".into(),
            action_label: "OK".into(),
            action_id: "ack".into(),
            a11y: None,
        },
        Component::Dropdown {
            id: "dd".into(),
            label: "Theme".into(),
            selected: Some("dark".into()),
            options: vec![DropdownOption {
                id: "dark".into(),
                label: "Dark".into(),
            }],
            a11y: None,
        },
        Component::AvatarPreview {
            id: "avatar".into(),
            image_data: None,
            initials: "AB".into(),
            bg_color: None,
            brightness: 0.0,
            editable: false,
            a11y: None,
        },
        Component::Slider {
            id: "slider".into(),
            label: "Vol".into(),
            value: 0.5,
            min: 0.0,
            max: 1.0,
            step: 0.1,
            min_icon: None,
            max_icon: None,
            a11y: None,
        },
        Component::Indicator {
            id: "indicator".into(),
            label: "Connected".into(),
            kind: IndicatorKind::Active,
            action_id: None,
            a11y: None,
        },
        Component::SectionedActionList {
            id: "sectioned".into(),
            sections: vec![Section {
                id: "primary".into(),
                label: "Primary".into(),
                items: vec![ActionListItem {
                    id: "a1".into(),
                    label: "Profile".into(),
                    icon: None,
                    detail: None,
                    a11y: None,
                    info_key: None,
                }],
            }],
        },
    ]
}

/// CC-22 reachability gate for `Component`: every currently-known
/// variant must render at least one non-space cell on TUI. Closes
/// the silent-drop class of bug (F1/F3/F4 in the audit) at a
/// structural level — adding a new variant in core without
/// updating this fixture leaves a visible gap that a reviewer can
/// catch from the diff alone.
///
/// Note: `Component` is `#[non_exhaustive]`, so this fixture is
/// hand-maintained. Update `every_known_component()` whenever
/// `core/vauchi-app/src/ui/component.rs` grows a variant.
// @internal
#[test]
fn every_known_component_variant_renders_non_empty() {
    for component in every_known_component() {
        let label = format!("{:?}", component);
        let label_short = label.split_whitespace().next().unwrap_or("?").to_string();
        let text = render_components_to_text(60, 3, &[component]);
        assert!(
            text.chars().any(|c| !c.is_whitespace()),
            "Component variant {label_short} rendered to whitespace only — silent drop"
        );
    }
}
