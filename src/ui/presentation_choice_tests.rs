// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! A `Choice` node is one selected option among several. Every other shell
//! renders it as an exclusive selection and reports the pick with the
//! generic choice value; the terminal must reach and operate it too.

use super::presentation_input::{InteractionState, KeyOutcome};
use super::presentation_protocol::PresentationState;
use super::presentation_renderer;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Modifier;
use vauchi_core::{
    AccessibilitySpec, ActionSpec, ActionTone, BindingId, ChoiceOption, Command, Event, InputValue,
    InteractionId, PresentationNode, PresentationRow, PresentationTokens, SurfaceId, SurfaceLayout,
    SurfaceSpec,
};

fn option(id: &str, label: &str) -> ChoiceOption {
    ChoiceOption {
        id: id.into(),
        label: label.into(),
    }
}

fn choice(binding: &str, label: &str, selected: Option<&str>) -> PresentationNode {
    PresentationNode::Choice {
        binding_id: BindingId::new(binding).unwrap(),
        label: label.into(),
        selected: selected.map(Into::into),
        options: vec![
            option("follow_system", "System"),
            option("dark", "Dark"),
            option("light", "Light"),
        ],
        enabled: true,
        accessibility: AccessibilitySpec::label(label),
    }
}

fn action_row(title: &str, id: &str) -> PresentationRow {
    PresentationRow {
        title: title.into(),
        subtitle: None,
        detail: None,
        icon_token: None,
        image_data: None,
        fallback_text: None,
        selected: false,
        enabled: true,
        activation: Some(ActionSpec {
            interaction_id: InteractionId::new(id).unwrap(),
            label: title.into(),
            accessibility_label: title.into(),
            icon_token: None,
            enabled: true,
            tone: ActionTone::Standard,
            shortcut: None,
        }),
        secondary_actions: Vec::new(),
        controls: Vec::new(),
        accessibility: AccessibilitySpec::label(title),
    }
}

fn state_with(nodes: Vec<PresentationNode>) -> PresentationState {
    let mut state = PresentationState::default();
    state.apply(&[Command::ReplaceSurface {
        surface: SurfaceSpec {
            surface_id: SurfaceId::new("settings").unwrap(),
            revision: 1,
            title: "Settings".into(),
            subtitle: None,
            accessibility_label: "Settings".into(),
            layout: SurfaceLayout::Scroll,
            tokens: PresentationTokens {
                spacing_small: 1,
                spacing_medium: 2,
                spacing_large: 3,
                corner_radius: 1,
                minimum_target_size: 1,
            },
            nodes,
        },
    }]);
    state
}

fn render(state: &PresentationState, width: u16, selected: Option<usize>) -> (String, Vec<String>) {
    let mut terminal = Terminal::new(TestBackend::new(width, 8)).unwrap();
    terminal
        .draw(|frame| presentation_renderer::draw(frame, frame.area(), state, 0, selected))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let plain = buffer.content.iter().map(|cell| cell.symbol()).collect();
    let bold = buffer
        .content
        .chunks(buffer.area.width as usize)
        .flat_map(|line| {
            let mut runs = Vec::new();
            let mut current = String::new();
            for cell in line {
                if cell.modifier.contains(Modifier::BOLD) {
                    current.push_str(cell.symbol());
                } else if !current.trim().is_empty() {
                    runs.push(std::mem::take(&mut current).trim().to_string());
                } else {
                    current.clear();
                }
            }
            if !current.trim().is_empty() {
                runs.push(current.trim().to_string());
            }
            runs
        })
        .collect();
    (plain, bold)
}

fn choice_events(outcome: KeyOutcome) -> Option<String> {
    let KeyOutcome::Events(events) = outcome else {
        return None;
    };
    match events.as_slice() {
        [
            Event::SurfaceActivated { .. },
            Event::ValueChanged {
                binding_id,
                value: InputValue::Choice(Some(picked)),
                ..
            },
        ] if binding_id.as_str() == "theme" => Some(picked.clone()),
        other => panic!("expected a choice value change, got {other:?}"),
    }
}

// @scenario: generic_presentation_protocol.feature :: Every shell renders the same prepared presentation
#[test]
fn a_choice_shows_every_option_and_brackets_the_selected_one() {
    let state = state_with(vec![choice("theme", "Theme", Some("dark"))]);

    let (plain, bold) = render(&state, 80, None);

    assert!(
        plain.contains("Theme: System  [ Dark ]  Light"),
        "options must be visible by label, with the selection marked, \
         or the user cannot tell what else there is to pick: {plain:?}"
    );
    assert!(
        bold.iter().any(|run| run.contains("[ Dark ]")),
        "the selected option must stand out: {bold:?}"
    );
}

// @scenario: generic_presentation_protocol.feature :: Every shell renders the same prepared presentation
#[test]
fn a_choice_too_wide_for_its_pane_shows_only_the_selection() {
    let state = state_with(vec![choice("theme", "Theme", Some("dark"))]);

    let (plain, _) = render(&state, 24, None);

    assert!(
        plain.contains("Theme: ‹ Dark ›"),
        "when the options do not fit, the selection alone must still be \
         legible instead of wrapping into noise: {plain:?}"
    );
}

// @scenario: generic_presentation_protocol.feature :: Every shell renders the same prepared presentation
#[test]
fn an_unset_choice_renders_a_placeholder() {
    let state = state_with(vec![choice("theme", "Theme", None)]);

    let (plain, _) = render(&state, 80, None);

    assert!(
        plain.contains("Theme: [ — ]  System  Dark  Light"),
        "{plain:?}"
    );
}

// @scenario: generic_presentation_protocol.feature :: Every shell renders the same prepared presentation
#[test]
fn a_selected_choice_is_highlighted_with_its_key_hint() {
    let state = state_with(vec![choice("theme", "Theme", Some("dark"))]);

    let (plain, _) = render(&state, 80, Some(0));
    let mut terminal = Terminal::new(TestBackend::new(80, 8)).unwrap();
    terminal
        .draw(|frame| presentation_renderer::draw(frame, frame.area(), &state, 0, Some(0)))
        .unwrap();
    let reversed: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .filter(|cell| cell.modifier.contains(Modifier::REVERSED))
        .map(|cell| cell.symbol())
        .collect();

    assert!(
        reversed.contains("Theme"),
        "the focused choice must carry the selection highlight: {reversed:?}"
    );
    assert!(
        plain.contains("←/→ picks"),
        "the keys that operate a focused choice must be discoverable: {plain:?}"
    );
}

// @scenario: generic_presentation_protocol.feature :: User interaction returns as an opaque event
#[test]
fn right_reports_the_next_option_as_a_choice_value() {
    let state = state_with(vec![choice("theme", "Theme", Some("dark"))]);
    let mut interaction = InteractionState::default();
    interaction.key_outcome(&state, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

    let picked = choice_events(
        interaction.key_outcome(&state, KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
    );

    assert_eq!(picked.as_deref(), Some("light"));
}

// @scenario: generic_presentation_protocol.feature :: User interaction returns as an opaque event
#[test]
fn left_wraps_from_the_first_option_to_the_last() {
    let state = state_with(vec![choice("theme", "Theme", Some("follow_system"))]);
    let mut interaction = InteractionState::default();
    interaction.key_outcome(&state, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

    let picked = choice_events(
        interaction.key_outcome(&state, KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
    );

    assert_eq!(picked.as_deref(), Some("light"));
}

// @scenario: generic_presentation_protocol.feature :: User interaction returns as an opaque event
#[test]
fn enter_cycles_a_selected_choice_forward() {
    let state = state_with(vec![choice("theme", "Theme", None)]);
    let mut interaction = InteractionState::default();
    interaction.key_outcome(&state, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

    let picked = choice_events(
        interaction.key_outcome(&state, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
    );

    assert_eq!(
        picked.as_deref(),
        Some("follow_system"),
        "an unset choice steps onto its first option"
    );
}

// @scenario: generic_presentation_protocol.feature :: User interaction returns as an opaque event
#[test]
fn arrows_leave_a_selected_list_row_alone() {
    let state = state_with(vec![PresentationNode::List {
        id: BindingId::new("rows").unwrap(),
        label: None,
        rows: vec![action_row("Display Name", "edit")],
        searchable: false,
        paging: None,
        accessibility: AccessibilitySpec::label("Profile"),
    }]);
    let mut interaction = InteractionState::default();
    interaction.key_outcome(&state, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

    let outcome =
        interaction.key_outcome(&state, KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));

    assert_eq!(outcome, KeyOutcome::Consumed);
}

// @scenario: generic_presentation_protocol.feature :: User interaction returns as an opaque event
#[test]
fn a_choice_takes_its_place_in_the_surface_order() {
    let state = state_with(vec![
        PresentationNode::List {
            id: BindingId::new("rows").unwrap(),
            label: None,
            rows: vec![action_row("Display Name", "edit")],
            searchable: false,
            paging: None,
            accessibility: AccessibilitySpec::label("Profile"),
        },
        PresentationNode::Group {
            id: None,
            label: Some("Appearance".into()),
            axis: vauchi_core::PresentationAxis::Vertical,
            children: vec![choice("theme", "Theme", Some("dark"))],
            accessibility: AccessibilitySpec::label("Appearance"),
        },
    ]);
    let mut interaction = InteractionState::default();
    interaction.key_outcome(&state, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    interaction.key_outcome(&state, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

    let picked = choice_events(
        interaction.key_outcome(&state, KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
    );

    assert_eq!(
        picked.as_deref(),
        Some("light"),
        "Down must walk rows and choices in the order they are painted"
    );
    let (_, bold) = render(&state, 80, Some(1));
    assert!(
        bold.iter().any(|run| run.contains("[ Dark ]")),
        "the second target the keys reach is the choice the renderer paints second: {bold:?}"
    );
}

// @scenario: generic_presentation_protocol.feature :: User interaction returns as an opaque event
#[test]
fn a_disabled_choice_is_not_reachable() {
    let PresentationNode::Choice {
        binding_id,
        label,
        selected,
        options,
        accessibility,
        ..
    } = choice("theme", "Theme", Some("dark"))
    else {
        unreachable!()
    };
    let state = state_with(vec![PresentationNode::Choice {
        binding_id,
        label,
        selected,
        options,
        enabled: false,
        accessibility,
    }]);
    let mut interaction = InteractionState::default();

    interaction.key_outcome(&state, KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
    let outcome =
        interaction.key_outcome(&state, KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));

    assert_eq!(outcome, KeyOutcome::Consumed);
}
