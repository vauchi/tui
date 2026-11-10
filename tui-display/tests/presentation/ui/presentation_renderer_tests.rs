// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use super::presentation_protocol::PresentationState;
use super::presentation_renderer;
use ratatui_core::backend::TestBackend;
use ratatui_core::style::Modifier;
use ratatui_core::terminal::Terminal;
use vauchi_contract::{
    AccessibilitySpec, ActionSpec, ActionTone, Command, ContextBar, InteractionId, OverlayKind,
    OverlaySpec, PaneLayout, PresentationNode, PresentationProfile, PresentationRow,
    PresentationTextStyle, PresentationTokens, SurfaceId, SurfaceLayout, SurfaceSpec, WindowClass,
};

pub(super) fn action(id: &str, label: &str) -> ActionSpec {
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

pub(super) fn titled_surface(id: &str, title: &str) -> SurfaceSpec {
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
            colors: crate::ui::test_colors::test_colors(),
            fonts: None,
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
                    colors: crate::ui::test_colors::test_colors(),
                    fonts: None,
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
                info: None,
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
                window_title: None,
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

pub(super) fn row(title: &str, activation: Option<ActionSpec>) -> PresentationRow {
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
        info: None,
    }
}

/// Reverse-video runs only — side-by-side panes share terminal rows, so a
/// whole-row view cannot tell which pane carries the highlight.
pub(super) fn highlighted_lines(buffer: &ratatui_core::buffer::Buffer) -> Vec<String> {
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
        search_placeholder: None,
        style: vauchi_contract::PresentationListStyle::Rows,
        id: vauchi_contract::BindingId::new("entries").unwrap(),
        label: None,
        rows: vec![row("Grace", Some(action("open:grace", "Grace")))],
        searchable: false,
        paging: None,
        accessibility: AccessibilitySpec::label("Contacts"),
    }];
    let mut detail = titled_surface("surface-detail", "Ada");
    detail.nodes = vec![PresentationNode::List {
        search_placeholder: None,
        style: vauchi_contract::PresentationListStyle::Rows,
        id: vauchi_contract::BindingId::new("fields").unwrap(),
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
                window_title: None,
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
        search_placeholder: None,
        style: vauchi_contract::PresentationListStyle::Rows,
        id: vauchi_contract::BindingId::new("results").unwrap(),
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

/// A settings-shaped row: Core names the setting in the row title and sends
/// the toggle unlabelled, with no activation of its own.
fn toggle_row(title: &str, on: bool) -> PresentationRow {
    let mut row = row(title, None);
    row.controls = vec![PresentationNode::Toggle {
        binding_id: vauchi_contract::BindingId::new("settings.card_updates").unwrap(),
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
        search_placeholder: None,
        style: vauchi_contract::PresentationListStyle::Rows,
        id: vauchi_contract::BindingId::new("privacy").unwrap(),
        label: None,
        rows: vec![toggle_row("Card Updates", true)],
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
        rendered.contains("[x] Card Updates"),
        "a row's toggle state must be visible — the row title names the \
         setting and Core sends the control unlabelled: {rendered:?}"
    );
}

// @scenario: generic_presentation_protocol.feature :: Every shell renders the same prepared presentation
#[test]
fn a_row_carrying_a_control_can_be_selected() {
    let mut surface = titled_surface("surface-primary", "Settings");
    surface.nodes = vec![PresentationNode::List {
        search_placeholder: None,
        style: vauchi_contract::PresentationListStyle::Rows,
        id: vauchi_contract::BindingId::new("privacy").unwrap(),
        label: None,
        rows: vec![toggle_row("Card Updates", false)],
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
        highlighted.iter().any(|line| line.contains("Card Updates")),
        "a row whose control is the operable thing must take a selection \
         index, or the keyboard can never reach it: {highlighted:?}"
    );
}

// @scenario: generic_presentation_protocol.feature :: Every shell renders the same prepared presentation
#[test]
fn a_rows_status_control_is_shown_as_a_tag_under_its_text() {
    let mut surface = titled_surface("surface-primary", "My Card");
    let mut entry = row("mobile", Some(action("open", "mobile")));
    entry.subtitle = Some("+44 20 7946 0958".into());
    entry.controls = vec![vauchi_contract::PresentationNode::Status {
        id: None,
        title: "Family".into(),
        detail: None,
        icon_token: None,
        badge: None,
        tone: vauchi_contract::PresentationTone::Neutral,
        activation: None,
        accessibility: AccessibilitySpec::label("Family"),
    }];
    surface.nodes = vec![PresentationNode::List {
        search_placeholder: None,
        style: vauchi_contract::PresentationListStyle::Rows,
        id: vauchi_contract::BindingId::new("own_entries").unwrap(),
        label: None,
        rows: vec![entry],
        searchable: false,
        paging: None,
        accessibility: AccessibilitySpec::label("Entries"),
    }];
    let mut state = PresentationState::default();
    state.apply(&[Command::ReplaceSurface { surface }]);

    let backend = TestBackend::new(80, 10);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| presentation_renderer::draw(frame, frame.area(), &state, 0, None))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let lines: Vec<String> = buffer
        .content
        .chunks(buffer.area.width as usize)
        .map(|cells| cells.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect();
    let value_line = lines
        .iter()
        .position(|line| line.contains("+44 20 7946 0958"))
        .unwrap_or_else(|| panic!("the value line is drawn: {lines:?}"));
    let tag_line = &lines[value_line + 1];
    assert_eq!(
        tag_line.find("[Family]"),
        lines[value_line].find("+44 20 7946 0958"),
        "a Status control is the row's state (who sees this entry), drawn as a \
         tag on the line under the row's text, in the text's column, not \
         dropped: {lines:?}"
    );
}

// @scenario: generic_presentation_protocol.feature :: Every shell renders the same prepared presentation
#[test]
fn a_rows_detail_is_shown() {
    let mut surface = titled_surface("surface-primary", "Settings");
    let mut named = row("Display Name", Some(action("edit", "Edit")));
    named.detail = Some("Bob".into());
    surface.nodes = vec![PresentationNode::List {
        search_placeholder: None,
        style: vauchi_contract::PresentationListStyle::Rows,
        id: vauchi_contract::BindingId::new("profile").unwrap(),
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
        search_placeholder: None,
        style: vauchi_contract::PresentationListStyle::Rows,
        id: vauchi_contract::BindingId::new("rows").unwrap(),
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
        search_placeholder: None,
        style: vauchi_contract::PresentationListStyle::Rows,
        id: vauchi_contract::BindingId::new("rows").unwrap(),
        label: None,
        rows: vec![row("Ada", Some(action("open:ada", "Ada")))],
        searchable: false,
        paging: Some(vauchi_contract::PresentationPaging {
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

/// Core's fifth slot explains the surface (vauchi/private#479): it is
/// drawn after the launchers, and its overlay is text to read, not items
/// to pick.
// @scenario: generic_presentation_protocol.feature :: Contextual controls expose four stable roles
#[test]
fn the_info_slot_is_drawn_last_and_its_overlay_shows_the_text() {
    let surface_id = SurfaceId::new("surface-primary").unwrap();
    let mut state = PresentationState::default();
    state.apply(&[
        Command::ReplaceSurface {
            surface: titled_surface("surface-primary", "People"),
        },
        Command::SetContextBar {
            surface_id: surface_id.clone(),
            revision: 1,
            bar: Box::new(ContextBar {
                back: Some(action("back", "Back")),
                navigation: None,
                primary: Some(action("primary", "Add")),
                secondary: Some(action("secondary", "More")),
                info: Some(action("info", "Info")),
            }),
        },
    ]);
    let backend = TestBackend::new(80, 16);
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
    let more = rendered.find("… More").expect("the launcher is drawn");
    let info = rendered.find("(i) Info").expect("the info slot is drawn");
    assert!(more < info, "info comes after the launchers: {rendered}");

    state.apply(&[Command::PresentOverlay {
        surface_id,
        revision: 1,
        overlay: OverlaySpec {
            kind: OverlayKind::Information,
            title: Some("People".into()),
            items: Vec::new(),
            body: Some("Here are the people you have exchanged cards with.".into()),
            close_label: None,
        },
    }]);
    terminal
        .draw(|frame| presentation_renderer::draw(frame, frame.area(), &state, 0, Some(0)))
        .unwrap();
    let rendered = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(
        rendered.contains("Here are the people you have exchanged"),
        "the overlay shows the text: {rendered}"
    );
}

/// A row that offers its own info (vauchi/private#479) must say so, or
/// the keyboard-only `i` shortcut (vauchi/private#516) is undiscoverable.
// @scenario: generic_presentation_protocol.feature :: Contextual controls expose four stable roles
#[test]
fn a_row_with_info_shows_its_i_marker() {
    let mut state = PresentationState::default();
    let mut surface = titled_surface("surface-primary", "Contacts");
    let mut ada = row("Ada", Some(action("open:ada", "Ada")));
    ada.info = Some(action("info:ada", "Info"));
    surface.nodes = vec![PresentationNode::List {
        search_placeholder: None,
        style: vauchi_contract::PresentationListStyle::Rows,
        id: vauchi_contract::BindingId::new("entries").unwrap(),
        label: None,
        rows: vec![ada],
        searchable: false,
        paging: None,
        accessibility: AccessibilitySpec::label("Contacts"),
    }];
    state.apply(&[Command::ReplaceSurface { surface }]);
    let backend = TestBackend::new(80, 16);
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
        rendered.contains("Ada (i)"),
        "the row marks that it carries info: {rendered}"
    );
}

/// Core names the overlay's way out (vauchi/private#479); the hint must
/// use that label instead of the generic phrasing once Core gives one.
// @scenario: generic_presentation_protocol.feature :: Contextual controls expose four stable roles
#[test]
fn an_overlay_hint_names_cores_close_label() {
    let surface_id = SurfaceId::new("surface-primary").unwrap();
    let mut state = PresentationState::default();
    state.apply(&[Command::ReplaceSurface {
        surface: titled_surface("surface-primary", "People"),
    }]);
    state.apply(&[Command::PresentOverlay {
        surface_id,
        revision: 1,
        overlay: OverlaySpec {
            kind: OverlayKind::Information,
            title: Some("People".into()),
            items: Vec::new(),
            body: Some("Here are the people you have exchanged cards with.".into()),
            close_label: Some("Done".into()),
        },
    }]);
    let backend = TestBackend::new(80, 16);
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
        rendered.contains("Esc: Done"),
        "the hint names Core's close label: {rendered}"
    );
}

/// Without a `close_label` (a pre-#479 Core, or an overlay that never set
/// one), the hint keeps the generic phrasing rather than showing nothing.
// @scenario: generic_presentation_protocol.feature :: Contextual controls expose four stable roles
#[test]
fn an_overlay_hint_keeps_the_generic_phrasing_without_a_close_label() {
    let surface_id = SurfaceId::new("surface-primary").unwrap();
    let mut state = PresentationState::default();
    state.apply(&[Command::ReplaceSurface {
        surface: titled_surface("surface-primary", "People"),
    }]);
    state.apply(&[Command::PresentOverlay {
        surface_id,
        revision: 1,
        overlay: OverlaySpec {
            kind: OverlayKind::Information,
            title: Some("People".into()),
            items: Vec::new(),
            body: Some("Here are the people you have exchanged cards with.".into()),
            close_label: None,
        },
    }]);
    let backend = TestBackend::new(80, 16);
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
        rendered.contains("Enter or Esc dismisses"),
        "the hint keeps today's phrasing: {rendered}"
    );
}
