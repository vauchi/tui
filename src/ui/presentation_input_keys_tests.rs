// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Key-by-key edges of the input adapter: wrap-around, empty collections,
//! nested inputs and modifier guards (vauchi/private#448).

use super::presentation_input::{InteractionState, KeyOutcome};
use super::presentation_input_tests::{action, row, state_with_action_list, state_with_input};
use super::presentation_protocol::PresentationState;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use vauchi_core::{
    AccessibilitySpec, BindingId, Command, ContextBar, Event, InputValue, OverlayKind, OverlaySpec,
    PresentationAxis, PresentationInputKind, PresentationListStyle, PresentationNode,
    PresentationRow, PresentationTokens, StandardShortcut, SurfaceId, SurfaceLayout, SurfaceSpec,
};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(character: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL)
}

fn activated_id(outcome: &KeyOutcome) -> Option<String> {
    let KeyOutcome::Events(events) = outcome else {
        return None;
    };
    events.iter().find_map(|event| match event {
        Event::ActionActivated { interaction_id, .. } => Some(interaction_id.as_str().to_string()),
        _ => None,
    })
}

fn changed_text(outcome: &KeyOutcome) -> Option<(BindingId, String)> {
    let KeyOutcome::Events(events) = outcome else {
        return None;
    };
    events.iter().find_map(|event| match event {
        Event::ValueChanged {
            binding_id,
            value: InputValue::Text(text),
            ..
        } => Some((binding_id.clone(), text.clone())),
        _ => None,
    })
}

fn binding(id: &str) -> BindingId {
    BindingId::new(id).unwrap()
}

fn input(id: &str, value: &str, max_length: Option<usize>) -> PresentationNode {
    PresentationNode::Input {
        binding_id: binding(id),
        label: id.into(),
        value: value.into(),
        placeholder: None,
        input_kind: PresentationInputKind::Text,
        max_length,
        validation_error: None,
        enabled: true,
        accessibility: AccessibilitySpec::label(id),
    }
}

fn list(rows: Vec<PresentationRow>) -> PresentationNode {
    PresentationNode::List {
        style: PresentationListStyle::Rows,
        id: binding("list"),
        label: None,
        rows,
        searchable: false,
        paging: None,
        accessibility: AccessibilitySpec::label("list"),
    }
}

fn replace_surface(
    state: &mut PresentationState,
    id: &str,
    revision: u64,
    nodes: Vec<PresentationNode>,
) {
    state.apply(&[Command::ReplaceSurface {
        surface: SurfaceSpec {
            surface_id: SurfaceId::new(id).unwrap(),
            revision,
            title: id.into(),
            subtitle: None,
            accessibility_label: id.into(),
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
}

fn state_with_nodes(nodes: Vec<PresentationNode>) -> PresentationState {
    let mut state = PresentationState::default();
    replace_surface(&mut state, "form", 1, nodes);
    state
}

fn state_with_overlay(items: &[&str]) -> PresentationState {
    let mut state = state_with_action_list();
    state.apply(&[Command::PresentOverlay {
        surface_id: SurfaceId::new("exchange_mode_selection").unwrap(),
        revision: 1,
        overlay: OverlaySpec {
            kind: OverlayKind::ActionMenu,
            title: None,
            body: None,
            items: items.iter().map(|id| action(id, None)).collect(),
            close_label: None,
        },
    }]);
    state
}

// @internal
#[test]
fn overlay_navigation_wraps_in_both_directions() {
    let state = state_with_overlay(&["a", "b", "c"]);
    let mut interaction = InteractionState::default();
    assert_eq!(interaction.selected_overlay(), 0);

    let moves = [
        (KeyCode::Down, 1),
        (KeyCode::Tab, 2),
        (KeyCode::Down, 0),
        (KeyCode::Up, 2),
        (KeyCode::BackTab, 1),
        (KeyCode::Up, 0),
    ];
    for (code, expected) in moves {
        assert_eq!(
            interaction.key_outcome(&state, key(code)),
            KeyOutcome::Consumed
        );
        assert_eq!(interaction.selected_overlay(), expected, "after {code:?}");
    }
}

// @internal
#[test]
fn overlay_without_items_consumes_navigation_without_panicking() {
    let state = state_with_overlay(&[]);
    let mut interaction = InteractionState::default();

    for code in [
        KeyCode::Down,
        KeyCode::Up,
        KeyCode::Enter,
        KeyCode::Char('1'),
    ] {
        assert_eq!(
            interaction.key_outcome(&state, key(code)),
            KeyOutcome::Consumed,
            "{code:?}"
        );
    }
    assert_eq!(interaction.selected_overlay(), 0);
}

// @internal
#[test]
fn enter_activates_the_highlighted_overlay_item() {
    let state = state_with_overlay(&["a", "b", "c"]);
    let mut interaction = InteractionState::default();
    interaction.key_outcome(&state, key(KeyCode::Down));

    let outcome = interaction.key_outcome(&state, key(KeyCode::Enter));
    assert_eq!(activated_id(&outcome).as_deref(), Some("b"));
}

// @internal
#[test]
fn overlay_digits_activate_that_item_and_other_characters_do_nothing() {
    let state = state_with_overlay(&["a", "b", "c"]);
    let mut interaction = InteractionState::default();

    let outcome = interaction.key_outcome(&state, key(KeyCode::Char('2')));
    assert_eq!(activated_id(&outcome).as_deref(), Some("b"));
    assert_eq!(
        interaction.key_outcome(&state, key(KeyCode::Char('0'))),
        KeyOutcome::Consumed
    );
    assert_eq!(
        interaction.key_outcome(&state, key(KeyCode::Char('x'))),
        KeyOutcome::Consumed
    );
    assert_eq!(
        interaction.key_outcome(&state, key(KeyCode::Char('9'))),
        KeyOutcome::Consumed
    );
}

// @internal
#[test]
fn ctrl_z_runs_only_the_undo_shortcut_action() {
    let mut state = state_with_action_list();
    state.apply(&[Command::SetContextBar {
        surface_id: SurfaceId::new("exchange_mode_selection").unwrap(),
        revision: 1,
        bar: Box::new(ContextBar {
            back: Some(action("back", Some(StandardShortcut::Back))),
            navigation: None,
            primary: Some(action("continue", Some(StandardShortcut::ActivatePrimary))),
            secondary: Some(action("undo", Some(StandardShortcut::Undo))),
            info: None,
        }),
    }]);
    let mut interaction = InteractionState::default();

    let outcome = interaction.key_outcome(&state, ctrl('z'));
    assert_eq!(activated_id(&outcome).as_deref(), Some("undo"));
    assert_eq!(
        interaction.key_outcome(&state, ctrl('x')),
        KeyOutcome::Consumed
    );
    assert_eq!(
        interaction.key_outcome(&state, key(KeyCode::Char('z'))),
        KeyOutcome::Consumed
    );
}

// @internal
#[test]
fn tab_without_a_context_bar_is_consumed_and_keeps_index_zero() {
    let state = state_with_action_list();
    let mut interaction = InteractionState::default();
    assert_eq!(interaction.selected_context(), 0);

    assert_eq!(
        interaction.key_outcome(&state, key(KeyCode::Tab)),
        KeyOutcome::Consumed
    );
    assert_eq!(interaction.selected_context(), 0);
}

// @internal
#[test]
fn up_from_no_selection_lands_on_the_last_row() {
    let state = state_with_action_list();
    let mut interaction = InteractionState::default();

    assert_eq!(
        interaction.key_outcome(&state, key(KeyCode::Up)),
        KeyOutcome::Consumed
    );
    assert_eq!(interaction.selected_surface_target(&state), Some(1));
}

// @internal
#[test]
fn a_selection_is_dropped_when_its_row_disappears() {
    let mut state = state_with_action_list();
    let mut interaction = InteractionState::default();
    interaction.key_outcome(&state, key(KeyCode::Down));
    interaction.key_outcome(&state, key(KeyCode::Down));
    assert_eq!(interaction.selected_surface_target(&state), Some(1));

    replace_surface(
        &mut state,
        "exchange_mode_selection",
        2,
        vec![list(vec![row("Only", Some(action("only", None)))])],
    );
    assert_eq!(interaction.selected_surface_target(&state), None);
}

// @internal
#[test]
fn a_selection_is_dropped_when_another_surface_takes_over() {
    let mut state = state_with_action_list();
    let mut interaction = InteractionState::default();
    interaction.key_outcome(&state, key(KeyCode::Down));
    assert_eq!(interaction.selected_surface_target(&state), Some(0));

    replace_surface(
        &mut state,
        "elsewhere",
        1,
        vec![list(vec![
            row("One", Some(action("one", None))),
            row("Two", Some(action("two", None))),
        ])],
    );
    assert_eq!(interaction.selected_surface_target(&state), None);
}

// @internal
#[test]
fn modified_characters_never_edit_a_focused_field() {
    let state = state_with_input();
    let mut interaction = InteractionState::default();

    assert_eq!(
        interaction.key_outcome(&state, ctrl('q')),
        KeyOutcome::Consumed
    );
    assert_eq!(
        interaction.key_outcome(&state, KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT)),
        KeyOutcome::Consumed
    );
}

// @internal
#[test]
fn backspace_removes_the_last_character() {
    let state = state_with_input();
    let mut interaction = InteractionState::default();

    let outcome = interaction.key_outcome(&state, key(KeyCode::Backspace));
    assert_eq!(
        changed_text(&outcome),
        Some((binding("display-name"), "A".into()))
    );
}

// @internal
#[test]
fn typing_stops_at_the_field_maximum_length() {
    let state = state_with_nodes(vec![input("code", "Al", Some(2))]);
    let mut interaction = InteractionState::default();

    assert_eq!(
        interaction.key_outcome(&state, key(KeyCode::Char('x'))),
        KeyOutcome::Consumed
    );

    let roomy = state_with_nodes(vec![input("code", "A", Some(2))]);
    let outcome = interaction.key_outcome(&roomy, key(KeyCode::Char('x')));
    assert_eq!(changed_text(&outcome), Some((binding("code"), "Ax".into())));
}

// @internal
#[test]
fn keystrokes_stay_on_the_focused_field_when_a_second_exists() {
    let state = state_with_nodes(vec![input("first", "", None), input("second", "", None)]);
    let mut interaction = InteractionState::default();

    let outcome = interaction.key_outcome(&state, key(KeyCode::Char('a')));
    assert_eq!(changed_text(&outcome), Some((binding("first"), "a".into())));
    let outcome = interaction.key_outcome(&state, key(KeyCode::Char('b')));
    assert_eq!(changed_text(&outcome), Some((binding("first"), "b".into())));
}

// @internal
#[test]
fn inputs_inside_groups_and_list_rows_receive_keystrokes() {
    let grouped = state_with_nodes(vec![PresentationNode::Group {
        id: None,
        label: None,
        axis: PresentationAxis::Vertical,
        children: vec![input("grouped", "", None)],
        accessibility: AccessibilitySpec::label("group"),
    }]);
    let outcome = InteractionState::default().key_outcome(&grouped, key(KeyCode::Char('g')));
    assert_eq!(
        changed_text(&outcome),
        Some((binding("grouped"), "g".into()))
    );

    let mut in_row = row("Setting", None);
    in_row.controls = vec![input("in-row", "", None)];
    let listed = state_with_nodes(vec![list(vec![in_row])]);
    let outcome = InteractionState::default().key_outcome(&listed, key(KeyCode::Char('r')));
    assert_eq!(
        changed_text(&outcome),
        Some((binding("in-row"), "r".into()))
    );
}
