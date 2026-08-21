// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use super::presentation_protocol::PresentationState;
use super::presentation_renderer;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Modifier;
use vauchi_core::{
    AccessibilitySpec, ActionSpec, ActionTone, Command, ContextBar, InteractionId, PaneLayout,
    PresentationNode, PresentationProfile, PresentationRow, PresentationTextStyle,
    PresentationTokens, SurfaceId, SurfaceLayout, SurfaceSpec, WindowClass,
};

fn action(id: &str, label: &str) -> ActionSpec {
    ActionSpec {
        interaction_id: InteractionId::new(id).unwrap(),
        label: label.into(),
        accessibility_label: label.into(),
        icon_token: None,
        enabled: true,
        tone: ActionTone::Standard,
        shortcut: None,
    }
}

fn titled_surface(id: &str, title: &str) -> SurfaceSpec {
    SurfaceSpec {
        surface_id: SurfaceId::new(id).unwrap(),
        revision: 1,
        title: title.into(),
        subtitle: None,
        accessibility_label: title.into(),
        layout: SurfaceLayout::Scroll,
        tokens: PresentationTokens {
            spacing_small: 1,
            spacing_medium: 2,
            spacing_large: 3,
            corner_radius: 1,
            minimum_target_size: 1,
        },
        nodes: vec![],
    }
}

// @scenario: generic_presentation_protocol.feature :: Contextual controls expose four stable roles
#[test]
fn renderer_keeps_content_humble_and_contextual_roles_in_one_bottom_strip() {
    let surface_id = SurfaceId::new("surface-primary").unwrap();
    let mut state = PresentationState::default();
    state.apply(&[
        Command::ReplaceSurface {
            surface: SurfaceSpec {
                surface_id: surface_id.clone(),
                revision: 1,
                title: "Contacts".into(),
                subtitle: Some("Core prepared".into()),
                accessibility_label: "Contacts".into(),
                layout: SurfaceLayout::Scroll,
                tokens: PresentationTokens {
                    spacing_small: 1,
                    spacing_medium: 2,
                    spacing_large: 3,
                    corner_radius: 1,
                    minimum_target_size: 1,
                },
                nodes: vec![PresentationNode::Text {
                    id: None,
                    content: "Alice".into(),
                    style: PresentationTextStyle::Body,
                    accessibility: AccessibilitySpec::label("Alice"),
                }],
            },
        },
        Command::SetContextBar {
            surface_id,
            revision: 1,
            bar: Box::new(ContextBar {
                back: Some(action("back", "Back")),
                navigation: Some(action("nav", "Navigate")),
                primary: Some(action("primary", "Continue")),
                secondary: Some(action("secondary", "More")),
            }),
        },
    ]);
    let backend = TestBackend::new(80, 16);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| presentation_renderer::draw(frame, frame.area(), &state, 2, None))
        .unwrap();

    let buffer = terminal.backend().buffer();
    let rendered = buffer
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    for expected in [
        "Contacts",
        "Core prepared",
        "Alice",
        "< Back",
        "≡ Navigate",
        "[ Continue ]",
        "… More",
    ] {
        assert!(rendered.contains(expected), "missing {expected:?}");
    }
}

// @scenario: generic_presentation_protocol.feature :: Available window drives structural composition
#[test]
fn expanded_profile_renders_primary_and_detail_as_two_native_panes() {
    let primary = titled_surface("surface-primary", "Contacts");
    let detail = titled_surface("surface-detail", "Alice");
    let mut state = PresentationState::default();
    state.apply(&[
        Command::ReplaceSurface {
            surface: primary.clone(),
        },
        Command::ReplaceSurface {
            surface: detail.clone(),
        },
        Command::SetPresentationProfile {
            profile: PresentationProfile {
                window_class: WindowClass::Expanded,
                pane_layout: PaneLayout::Split,
                primary_surface: primary.surface_id,
                detail_surface: Some(detail.surface_id.clone()),
                active_surface: detail.surface_id,
            },
        },
    ]);
    let backend = TestBackend::new(100, 18);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal
        .draw(|frame| presentation_renderer::draw(frame, frame.area(), &state, 0, None))
        .unwrap();

    let rendered = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(rendered.contains("Contacts"));
    assert!(rendered.contains("Alice"));
}

fn row(title: &str, activation: Option<ActionSpec>) -> PresentationRow {
    PresentationRow {
        title: title.into(),
        subtitle: None,
        detail: None,
        icon_token: None,
        image_data: None,
        fallback_text: None,
        selected: false,
        enabled: true,
        activation,
        secondary_actions: Vec::new(),
        controls: Vec::new(),
        accessibility: AccessibilitySpec::label(title),
    }
}

/// Reverse-video runs only — side-by-side panes share terminal rows, so a
/// whole-row view cannot tell which pane carries the highlight.
fn highlighted_lines(buffer: &ratatui::buffer::Buffer) -> Vec<String> {
    buffer
        .content
        .chunks(buffer.area.width as usize)
        .flat_map(|line| {
            let mut runs = Vec::new();
            let mut current = String::new();
            for cell in line {
                if cell.modifier.contains(Modifier::REVERSED) {
                    current.push_str(cell.symbol());
                    continue;
                }
                runs.extend(non_blank(std::mem::take(&mut current)));
            }
            runs.extend(non_blank(current));
            runs
        })
        .collect()
}

fn non_blank(text: String) -> Option<String> {
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

// @scenario: generic_presentation_protocol.feature :: Available window drives structural composition
#[test]
fn split_panes_highlight_only_the_active_surface() {
    let mut primary = titled_surface("surface-primary", "Contacts");
    primary.nodes = vec![PresentationNode::List {
        id: vauchi_core::BindingId::new("entries").unwrap(),
        label: None,
        rows: vec![row("Grace", Some(action("open:grace", "Grace")))],
        searchable: false,
        paging: None,
        accessibility: AccessibilitySpec::label("Contacts"),
    }];
    let mut detail = titled_surface("surface-detail", "Ada");
    detail.nodes = vec![PresentationNode::List {
        id: vauchi_core::BindingId::new("fields").unwrap(),
        label: None,
        rows: vec![row("Email", Some(action("edit:email", "Email")))],
        searchable: false,
        paging: None,
        accessibility: AccessibilitySpec::label("Fields"),
    }];
    let mut state = PresentationState::default();
    state.apply(&[
        Command::ReplaceSurface {
            surface: primary.clone(),
        },
        Command::ReplaceSurface {
            surface: detail.clone(),
        },
        Command::SetPresentationProfile {
            profile: PresentationProfile {
                window_class: WindowClass::Expanded,
                pane_layout: PaneLayout::Split,
                primary_surface: primary.surface_id,
                detail_surface: Some(detail.surface_id.clone()),
                active_surface: detail.surface_id,
            },
        },
    ]);

    let backend = TestBackend::new(100, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| presentation_renderer::draw(frame, frame.area(), &state, 0, Some(0)))
        .unwrap();

    let highlighted = highlighted_lines(terminal.backend().buffer());
    assert!(
        highlighted.iter().any(|line| line.contains("Email")),
        "the selection indexes the active detail surface, so its row must be highlighted: {highlighted:?}"
    );
    assert!(
        !highlighted.iter().any(|line| line.contains("Grace")),
        "the inactive primary pane must not show a selection the keyboard cannot move: {highlighted:?}"
    );
}

// @scenario: generic_presentation_protocol.feature :: Contextual controls expose four stable roles
#[test]
fn highlight_marks_the_row_that_activation_would_reach() {
    let mut surface = titled_surface("surface-primary", "Contacts");
    surface.nodes = vec![PresentationNode::List {
        id: vauchi_core::BindingId::new("results").unwrap(),
        label: None,
        rows: vec![
            row("Unavailable", None),
            row("Ada", Some(action("open:ada", "Ada"))),
        ],
        searchable: false,
        paging: None,
        accessibility: AccessibilitySpec::label("Results"),
    }];
    let mut state = PresentationState::default();
    state.apply(&[Command::ReplaceSurface { surface }]);

    let backend = TestBackend::new(60, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| presentation_renderer::draw(frame, frame.area(), &state, 0, Some(0)))
        .unwrap();

    let highlighted = highlighted_lines(terminal.backend().buffer());
    assert!(
        highlighted.iter().any(|line| line.contains("Ada")),
        "selection index 0 activates the first actionable row, so that row must be the highlighted one: {highlighted:?}"
    );
    assert!(
        !highlighted.iter().any(|line| line.contains("Unavailable")),
        "a row with no activation can never be reached by Enter, so it must never look selected: {highlighted:?}"
    );
}

/// A locked app must not render a navigation affordance here either.
///
/// On Android, the lock screen's context bar listed every destination and
/// tapping one walked past the password
/// (`problems/2026-08-12-android-app-password-bypass`). Core caused it, so
/// the question was whether the other shells inherited it — and "they
/// render what Core sends, so they must" is the same reasoning that had the
/// defect filed as Android-only in the first place.
///
/// So this drives a real locked `AppEngine` rather than a hand-built
/// fixture, and asserts on what the terminal actually paints. It fails if
/// Core starts publishing destinations to a locked surface again, or if
/// this shell ever grows a navigation menu of its own.
// @scenario: generic_presentation_protocol.feature :: Contextual controls expose four stable roles
#[test]
fn a_locked_app_paints_no_navigation_affordance() {
    let mut vauchi = vauchi_core::api::Vauchi::in_memory().expect("in-memory core");
    vauchi.create_identity("Alice").expect("identity");
    let mut engine = vauchi_app::ui::AppEngine::new(vauchi);
    engine
        .vauchi_mut()
        .setup_app_password("app-password-123")
        .expect("app password");
    // Password enabled + identity present is what sends bootstrap to the
    // lock screen, the same path a cold launch takes.
    engine.bootstrap();

    let commands = engine.initial_commands().expect("initial commands");
    let bar_has_navigation = commands.iter().any(
        |command| matches!(command, Command::SetContextBar { bar, .. } if bar.navigation.is_some()),
    );
    assert!(
        !bar_has_navigation,
        "core published a navigation affordance for a locked surface",
    );

    let mut state = PresentationState::default();
    state.apply(&commands);
    let backend = TestBackend::new(80, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| presentation_renderer::draw(frame, frame.area(), &state, 0, None))
        .unwrap();

    let rendered = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(
        !rendered.contains('≡'),
        "the terminal painted the navigation role button while locked: {rendered:?}",
    );
}

/// A settings-shaped row: Core names the setting in the row title and sends
/// the toggle unlabelled, with no activation of its own.
fn toggle_row(title: &str, on: bool) -> PresentationRow {
    let mut row = row(title, None);
    row.controls = vec![PresentationNode::Toggle {
        binding_id: vauchi_core::BindingId::new("settings.delivery_receipts").unwrap(),
        label: String::new(),
        value: on,
        enabled: true,
        accessibility: AccessibilitySpec::label(title),
    }];
    row
}

// @scenario: generic_presentation_protocol.feature :: Every shell renders the same prepared presentation
#[test]
fn a_row_control_renders_its_state() {
    let mut surface = titled_surface("surface-primary", "Settings");
    surface.nodes = vec![PresentationNode::List {
        id: vauchi_core::BindingId::new("privacy").unwrap(),
        label: None,
        rows: vec![toggle_row("Delivery Receipts", true)],
        searchable: false,
        paging: None,
        accessibility: AccessibilitySpec::label("Privacy"),
    }];
    let mut state = PresentationState::default();
    state.apply(&[Command::ReplaceSurface { surface }]);

    let backend = TestBackend::new(80, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| presentation_renderer::draw(frame, frame.area(), &state, 0, None))
        .unwrap();
    let rendered = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();

    assert!(
        rendered.contains("[x] Delivery Receipts"),
        "a row's toggle state must be visible — the row title names the \
         setting and Core sends the control unlabelled: {rendered:?}"
    );
}

// @scenario: generic_presentation_protocol.feature :: Every shell renders the same prepared presentation
#[test]
fn a_row_carrying_a_control_can_be_selected() {
    let mut surface = titled_surface("surface-primary", "Settings");
    surface.nodes = vec![PresentationNode::List {
        id: vauchi_core::BindingId::new("privacy").unwrap(),
        label: None,
        rows: vec![toggle_row("Delivery Receipts", false)],
        searchable: false,
        paging: None,
        accessibility: AccessibilitySpec::label("Privacy"),
    }];
    let mut state = PresentationState::default();
    state.apply(&[Command::ReplaceSurface { surface }]);

    let backend = TestBackend::new(80, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| presentation_renderer::draw(frame, frame.area(), &state, 0, Some(0)))
        .unwrap();

    let highlighted = highlighted_lines(terminal.backend().buffer());
    assert!(
        highlighted
            .iter()
            .any(|line| line.contains("Delivery Receipts")),
        "a row whose control is the operable thing must take a selection \
         index, or the keyboard can never reach it: {highlighted:?}"
    );
}

// @scenario: generic_presentation_protocol.feature :: Every shell renders the same prepared presentation
#[test]
fn a_rows_detail_is_shown() {
    let mut surface = titled_surface("surface-primary", "Settings");
    let mut named = row("Display Name", Some(action("edit", "Edit")));
    named.detail = Some("Bob".into());
    surface.nodes = vec![PresentationNode::List {
        id: vauchi_core::BindingId::new("profile").unwrap(),
        label: None,
        rows: vec![named],
        searchable: false,
        paging: None,
        accessibility: AccessibilitySpec::label("Profile"),
    }];
    let mut state = PresentationState::default();
    state.apply(&[Command::ReplaceSurface { surface }]);

    let backend = TestBackend::new(80, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| presentation_renderer::draw(frame, frame.area(), &state, 0, None))
        .unwrap();
    let rendered = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();

    assert!(
        rendered.contains("Bob"),
        "Core puts a settings row's value in detail, so dropping it hides \
         the value the row exists to show: {rendered:?}"
    );
}

/// A surface taller than its viewport scrolls to keep the selection visible.
///
/// The renderer built every line and rendered a Paragraph with no
/// `.scroll()`, so with 200 seeded contacts only the first screenful was
/// ever painted and Up/Down walked the selection off-screen into nothing.
// @scenario: generic_presentation_protocol.feature :: Every shell renders the same prepared presentation
#[test]
fn a_surface_scrolls_to_keep_the_selection_visible() {
    let mut surface = titled_surface("surface-primary", "Contacts");
    let rows: Vec<_> = (0..40)
        .map(|i| {
            row(
                &format!("Contact{i}"),
                Some(action(&format!("open:{i}"), "Open")),
            )
        })
        .collect();
    surface.nodes = vec![PresentationNode::List {
        id: vauchi_core::BindingId::new("contacts").unwrap(),
        label: None,
        rows,
        searchable: false,
        paging: None,
        accessibility: AccessibilitySpec::label("Contacts"),
    }];
    let mut state = PresentationState::default();
    state.apply(&[Command::ReplaceSurface { surface }]);

    // A viewport far shorter than the list, with a late row selected.
    let backend = TestBackend::new(80, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| presentation_renderer::draw(frame, frame.area(), &state, 0, Some(35)))
        .unwrap();
    let rendered = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();

    assert!(
        rendered.contains("Contact35"),
        "the selected row must be painted, or the rows past the first \
         screenful are unreachable: {rendered:?}"
    );
}

/// A scrolled surface says how much is below the fold.
// @scenario: generic_presentation_protocol.feature :: Every shell renders the same prepared presentation
#[test]
fn a_paged_surface_shows_its_count() {
    let mut surface = titled_surface("surface-primary", "Contacts");
    surface.nodes = vec![PresentationNode::List {
        id: vauchi_core::BindingId::new("contacts").unwrap(),
        label: None,
        rows: vec![row("Ada", Some(action("open:ada", "Ada")))],
        searchable: false,
        paging: Some(vauchi_core::PresentationPaging {
            total_count: 200,
            offset: 0,
            window: 25,
        }),
        accessibility: AccessibilitySpec::label("Contacts"),
    }];
    let mut state = PresentationState::default();
    state.apply(&[Command::ReplaceSurface { surface }]);

    let backend = TestBackend::new(80, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| presentation_renderer::draw(frame, frame.area(), &state, 0, None))
        .unwrap();
    let rendered = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();

    assert!(
        rendered.contains("1-25 of 200"),
        "Core already sends PresentationPaging; without it a scrolled \
         surface gives no hint how much is below the fold: {rendered:?}"
    );
}
