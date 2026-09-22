// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

use super::presentation_protocol::{ChoiceStep, PresentationState};
use serde::Deserialize;
use vauchi_core::{
    AccessibilitySpec, ActionSpec, ActionTone, Command, ContextBar, Event, InteractionId,
    NavigationItem, NavigationSpec, OverlayKind, OverlaySpec, PaneLayout, PresentationProfile,
    PresentationTokens, SurfaceId, SurfaceLayout, SurfaceSpec, WindowClass,
};

// Fixture versions are exact contracts: additive fields require an explicit
// consumer review rather than being ignored silently.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PresentationContractFixture {
    schema_version: u64,
    initial_commands: Vec<Command>,
    steps: Vec<PresentationContractStep>,
    expected_state: ExpectedPresentationState,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PresentationContractStep {
    // The TUI replays Core's commands, but decoding the event still verifies
    // that this consumer agrees with the shell-to-Core wire shape.
    #[serde(rename = "event")]
    _event: Event,
    commands: Vec<Command>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedPresentationState {
    active_surface_id: SurfaceId,
    surface: SurfaceSpec,
    context_bar: ContextBar,
}

fn action(id: &str) -> ActionSpec {
    ActionSpec {
        interaction_id: InteractionId::new(id).unwrap(),
        label: id.into(),
        accessibility_label: id.into(),
        icon_token: None,
        enabled: true,
        tone: ActionTone::Standard,
        shortcut: None,
    }
}

fn surface(revision: u64) -> SurfaceSpec {
    surface_for("surface-primary", revision)
}

fn surface_for(id: &str, revision: u64) -> SurfaceSpec {
    SurfaceSpec {
        surface_id: SurfaceId::new(id).unwrap(),
        revision,
        title: format!("Contacts {revision}"),
        subtitle: None,
        accessibility_label: "Contacts".into(),
        layout: SurfaceLayout::Scroll,
        tokens: PresentationTokens {
            spacing_small: 1,
            spacing_medium: 2,
            spacing_large: 3,
            corner_radius: 1,
            minimum_target_size: 1,
        },
        nodes: vec![vauchi_core::PresentationNode::Text {
            id: None,
            content: "Alice".into(),
            style: vauchi_core::PresentationTextStyle::Body,
            accessibility: AccessibilitySpec::label("Alice"),
        }],
    }
}

// @scenario: generic_presentation_protocol.feature :: Responsive transitions preserve interaction state
#[test]
fn responsive_profiles_preserve_both_surfaces_and_the_active_detail() {
    let mut state = PresentationState::default();
    let primary = surface_for("surface-primary", 7);
    let detail = surface_for("surface-detail", 2);
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
                primary_surface: primary.surface_id.clone(),
                detail_surface: Some(detail.surface_id.clone()),
                active_surface: detail.surface_id.clone(),
            },
        },
    ]);

    assert_eq!(state.visible_surfaces().len(), 2);
    assert_eq!(state.surface().unwrap().surface_id, detail.surface_id);

    state.apply(&[Command::SetPresentationProfile {
        profile: PresentationProfile {
            window_class: WindowClass::Compact,
            pane_layout: PaneLayout::Single,
            primary_surface: primary.surface_id.clone(),
            detail_surface: Some(detail.surface_id.clone()),
            active_surface: detail.surface_id.clone(),
        },
    }]);

    assert_eq!(state.visible_surfaces().len(), 1);
    assert_eq!(state.surface().unwrap().surface_id, detail.surface_id);
    assert_eq!(state.retained_surface_count(), 2);
}

// @scenario: generic_presentation_protocol.feature :: Every shell renders the same prepared presentation
#[test]
fn transaction_installs_surface_context_bar_and_overlay_atomically() {
    let mut state = PresentationState::default();
    let surface = surface(3);
    let surface_id = surface.surface_id.clone();

    let effects = state.apply(&[
        Command::ReplaceSurface { surface },
        Command::SetContextBar {
            surface_id: surface_id.clone(),
            revision: 3,
            bar: Box::new(ContextBar {
                back: Some(action("back")),
                navigation: Some(action("navigation")),
                primary: Some(action("primary")),
                secondary: Some(action("secondary")),
                info: None,
            }),
        },
        Command::PresentOverlay {
            surface_id,
            revision: 3,
            overlay: OverlaySpec {
                kind: OverlayKind::Navigation,
                title: Some("Navigate".into()),
                items: vec![action("open-primary")],
                body: None,
            },
        },
    ]);

    assert!(effects.is_empty());
    assert_eq!(state.surface().unwrap().revision, 3);
    assert_eq!(state.context_actions().len(), 4);
    assert_eq!(state.overlay().unwrap().kind, OverlayKind::Navigation);
}

// Core makes the context-bar menu buttons toggle by rewriting a repeat
// PresentOverlay into DismissOverlay. Every shell has to map it: Android and
// iOS dropped it into a generic effect and the menu never closed on a second
// tap (`vauchi/android!621`). The TUI already scopes overlays per surface, so
// this arm is the only half it was missing.
// @internal
#[test]
fn dismiss_overlay_closes_the_open_overlay() {
    let mut state = PresentationState::default();
    let surface = surface(3);
    let surface_id = surface.surface_id.clone();

    state.apply(&[
        Command::ReplaceSurface { surface },
        Command::PresentOverlay {
            surface_id: surface_id.clone(),
            revision: 3,
            overlay: OverlaySpec {
                kind: OverlayKind::Navigation,
                title: Some("Navigate".into()),
                items: vec![action("open-primary")],
                body: None,
            },
        },
    ]);
    assert_eq!(state.overlay().unwrap().kind, OverlayKind::Navigation);

    let effects = state.apply(&[Command::DismissOverlay {
        surface_id,
        revision: 3,
        kind: OverlayKind::Navigation,
    }]);

    assert!(
        state.overlay().is_none(),
        "a dismissed overlay must leave nothing to render",
    );
    assert!(
        effects.is_empty(),
        "DismissOverlay must be handled, not passed through as an effect: {effects:?}",
    );
}

// @scenario: generic_presentation_protocol.feature :: Every shell renders the same prepared presentation
#[test]
fn tui_consumes_the_core_owned_presentation_contract_fixture() {
    let fixture: PresentationContractFixture =
        serde_json::from_str(vauchi_app::ui::presentation_contract_fixture_json())
            .expect("failed to deserialize Core-owned presentation contract fixture");
    let mut state = PresentationState::default();

    assert_eq!(
        fixture.schema_version, 1,
        "fixture schema changed; re-verify the TUI reducer contract"
    );
    assert!(!fixture.initial_commands.is_empty());
    assert!(!fixture.steps.is_empty());
    assert_eq!(
        fixture.expected_state.surface.surface_id,
        fixture.expected_state.active_surface_id
    );

    let effects = state.apply(&fixture.initial_commands);
    assert!(
        effects.is_empty(),
        "initial fixture batch emitted effects: {effects:?}"
    );
    for (index, step) in fixture.steps.into_iter().enumerate() {
        assert!(!step.commands.is_empty(), "fixture step {index} is empty");
        let effects = state.apply(&step.commands);
        assert!(
            effects.is_empty(),
            "fixture step {index} emitted effects: {effects:?}"
        );
    }

    assert_eq!(
        state.surface().map(|surface| surface.surface_id.as_str()),
        Some(fixture.expected_state.active_surface_id.as_str())
    );
    assert_eq!(state.surface(), Some(&fixture.expected_state.surface));
    assert_eq!(
        state.context_bar(),
        Some(&fixture.expected_state.context_bar)
    );
    assert!(
        state.overlay().is_none(),
        "fixture v1 ends without an active overlay"
    );
}

// @scenario: generic_presentation_protocol.feature :: Invalid boundary input fails safely
#[test]
fn stale_chrome_is_rejected_and_effects_are_returned_to_the_shell() {
    let mut state = PresentationState::default();
    let current = surface(4);
    let surface_id = current.surface_id.clone();
    state.apply(&[Command::ReplaceSurface { surface: current }]);

    let effects = state.apply(&[
        Command::SetContextBar {
            surface_id: surface_id.clone(),
            revision: 3,
            bar: Box::new(ContextBar {
                primary: Some(action("stale")),
                ..ContextBar::default()
            }),
        },
        Command::PresentOverlay {
            surface_id,
            revision: 3,
            overlay: OverlaySpec {
                kind: OverlayKind::ActionMenu,
                title: None,
                items: vec![action("stale")],
                body: None,
            },
        },
        Command::ResetApplication,
    ]);

    assert!(state.context_actions().is_empty());
    assert!(state.overlay().is_none());
    assert_eq!(effects, vec![Command::ResetApplication]);
}

// @scenario: generic_presentation_protocol.feature :: Interaction activates its visible pane first
#[test]
fn activation_targets_the_surface_before_sending_the_opaque_interaction() {
    let mut state = PresentationState::default();
    let current = surface(1);
    let surface_id = current.surface_id.clone();
    state.apply(&[
        Command::ReplaceSurface { surface: current },
        Command::SetContextBar {
            surface_id: surface_id.clone(),
            revision: 1,
            bar: Box::new(ContextBar {
                primary: Some(action("opaque.primary")),
                ..ContextBar::default()
            }),
        },
    ]);

    assert_eq!(
        state.activate_context(0),
        vec![
            vauchi_core::Event::SurfaceActivated {
                surface_id: surface_id.clone(),
            },
            vauchi_core::Event::ActionActivated {
                surface_id,
                interaction_id: InteractionId::new("opaque.primary").unwrap(),
            },
        ]
    );
}

// @scenario: generic_presentation_protocol.feature :: Every shell reports the same generic events
#[test]
fn activating_a_control_row_reports_the_flipped_value() {
    use vauchi_core::{InputValue, PresentationNode, PresentationRow};

    let mut current = surface(1);
    let row = PresentationRow {
        title: "Delivery Receipts".into(),
        subtitle: None,
        detail: None,
        icon_token: None,
        image_data: None,
        fallback_text: None,
        selected: false,
        enabled: true,
        activation: None,
        secondary_actions: Vec::new(),
        controls: vec![PresentationNode::Toggle {
            binding_id: vauchi_core::BindingId::new("settings.delivery_receipts").unwrap(),
            label: String::new(),
            value: true,
            enabled: true,
            accessibility: AccessibilitySpec::label("Delivery Receipts"),
        }],
        accessibility: AccessibilitySpec::label("Delivery Receipts"),
    };
    current.nodes = vec![PresentationNode::List {
        style: vauchi_core::PresentationListStyle::Rows,
        id: vauchi_core::BindingId::new("privacy").unwrap(),
        label: None,
        rows: vec![row],
        searchable: false,
        paging: None,
        accessibility: AccessibilitySpec::label("Privacy"),
    }];
    let mut state = PresentationState::default();
    state.apply(&[Command::ReplaceSurface { surface: current }]);

    let events = state.activate_surface_target(0);
    let changed = events.iter().find_map(|event| match event {
        vauchi_core::Event::ValueChanged {
            binding_id, value, ..
        } => Some((binding_id.clone(), value.clone())),
        _ => None,
    });

    let (binding_id, value) = changed.expect(
        "activating a row whose control is the operable thing must report a \
         value change, not an action",
    );
    assert_eq!(binding_id.as_str(), "settings.delivery_receipts");
    assert_eq!(
        value,
        InputValue::Boolean(false),
        "the reported value is the flipped one"
    );
}

fn nav_item(id: &str, selected: bool) -> NavigationItem {
    NavigationItem {
        interaction_id: InteractionId::new(id).unwrap(),
        label: id.into(),
        accessibility_label: id.into(),
        icon_token: None,
        selected,
        badge_count: 0,
    }
}

// @scenario: generic_presentation_protocol.feature :: Every shell renders the same prepared presentation
/// Core publishes the persistent navigation beside the context bar; a shell
/// that does not consume it leaks it as a native effect and fails the
/// contract fixture (core 0.67.1). Same revision gate and same lifetime as
/// the context bar: stale chrome is dropped, a new surface clears it.
#[test]
fn navigation_is_consumed_for_the_current_revision_and_stale_navigation_is_dropped() {
    let mut state = PresentationState::default();
    let current = surface(4);
    let surface_id = current.surface_id.clone();
    state.apply(&[Command::ReplaceSurface { surface: current }]);

    let effects = state.apply(&[Command::SetNavigation {
        surface_id: surface_id.clone(),
        revision: 4,
        navigation: NavigationSpec {
            items: vec![nav_item("nav.home", true), nav_item("nav.contacts", false)],
        },
    }]);
    assert!(
        effects.is_empty(),
        "navigation must be consumed, got {effects:?}"
    );
    assert_eq!(state.navigation().map(|nav| nav.items.len()), Some(2));

    let effects = state.apply(&[Command::SetNavigation {
        surface_id: surface_id.clone(),
        revision: 3,
        navigation: NavigationSpec::default(),
    }]);
    assert!(effects.is_empty(), "stale chrome is dropped, not echoed");
    assert_eq!(
        state.navigation().map(|nav| nav.items.len()),
        Some(2),
        "a stale SetNavigation must not replace the current one"
    );

    state.apply(&[Command::ReplaceSurface {
        surface: surface(5),
    }]);
    assert!(
        state.navigation().is_none(),
        "a replaced surface starts without navigation until Core publishes it"
    );
}

fn state_showing(nodes: Vec<vauchi_core::PresentationNode>) -> PresentationState {
    let mut spec = surface(1);
    spec.nodes = nodes;
    let mut state = PresentationState::default();
    state.apply(&[Command::ReplaceSurface { surface: spec }]);
    state
}

fn status_chip(enabled: bool) -> vauchi_core::PresentationNode {
    let mut activation = action("status");
    activation.enabled = enabled;
    vauchi_core::PresentationNode::Status {
        id: None,
        title: "Synced".into(),
        detail: None,
        icon_token: None,
        badge: None,
        tone: vauchi_core::PresentationTone::Neutral,
        activation: Some(activation),
        accessibility: AccessibilitySpec::label("Synced"),
    }
}

fn choice(selected: Option<&str>, options: &[&str]) -> vauchi_core::PresentationNode {
    vauchi_core::PresentationNode::Choice {
        binding_id: vauchi_core::BindingId::new("pick").unwrap(),
        label: "Pick".into(),
        selected: selected.map(Into::into),
        options: options
            .iter()
            .map(|id| vauchi_core::ChoiceOption {
                id: (*id).into(),
                label: (*id).into(),
            })
            .collect(),
        enabled: true,
        accessibility: AccessibilitySpec::label("Pick"),
    }
}

fn toggle_row(enabled: bool) -> vauchi_core::PresentationRow {
    vauchi_core::PresentationRow {
        title: "Setting".into(),
        subtitle: None,
        detail: None,
        icon_token: None,
        image_data: None,
        fallback_text: None,
        selected: false,
        enabled: true,
        activation: None,
        secondary_actions: Vec::new(),
        controls: vec![vauchi_core::PresentationNode::Toggle {
            binding_id: vauchi_core::BindingId::new("flag").unwrap(),
            label: "Flag".into(),
            value: false,
            enabled,
            accessibility: AccessibilitySpec::label("Flag"),
        }],
        accessibility: AccessibilitySpec::label("Setting"),
    }
}

fn chosen(events: &[Event]) -> Option<&str> {
    events.iter().find_map(|event| match event {
        Event::ValueChanged {
            value: vauchi_core::InputValue::Choice(Some(id)),
            ..
        } => Some(id.as_str()),
        _ => None,
    })
}

// @internal
#[test]
fn dismissing_an_overlay_of_another_kind_leaves_it_open() {
    let mut state = PresentationState::default();
    let surface = surface(1);
    let surface_id = surface.surface_id.clone();
    state.apply(&[
        Command::ReplaceSurface { surface },
        Command::PresentOverlay {
            surface_id: surface_id.clone(),
            revision: 1,
            overlay: OverlaySpec {
                kind: OverlayKind::ActionMenu,
                title: None,
                body: None,
                items: vec![action("item")],
            },
        },
    ]);

    let effects = state.apply(&[Command::DismissOverlay {
        surface_id,
        revision: 1,
        kind: OverlayKind::Navigation,
    }]);

    assert_eq!(state.overlay().unwrap().kind, OverlayKind::ActionMenu);
    assert!(effects.is_empty(), "{effects:?}");
}

// @internal
#[test]
fn native_back_is_requested_only_after_core_asks_for_it() {
    let mut state = state_showing(Vec::new());
    assert!(!state.native_back_requested());

    state.apply(&[Command::PerformNativeBack]);

    assert!(state.native_back_requested());
}

// @internal
#[test]
fn the_status_chip_is_found_inside_a_group_and_skipped_when_disabled() {
    let grouped = state_showing(vec![vauchi_core::PresentationNode::Group {
        id: None,
        label: None,
        axis: vauchi_core::PresentationAxis::Vertical,
        children: vec![status_chip(true)],
        accessibility: AccessibilitySpec::label("group"),
    }]);
    assert_eq!(
        grouped
            .status_activation()
            .map(|a| a.interaction_id.as_str()),
        Some("status")
    );

    let disabled = state_showing(vec![status_chip(false)]);
    assert!(disabled.status_activation().is_none());
}

// @internal
#[test]
fn stepping_a_choice_wraps_from_either_end() {
    let unselected = state_showing(vec![choice(None, &["a", "b", "c"])]);
    assert_eq!(
        chosen(&unselected.step_surface_choice(0, ChoiceStep::Previous)),
        Some("c")
    );
    assert_eq!(
        chosen(&unselected.step_surface_choice(0, ChoiceStep::Next)),
        Some("a")
    );

    let first = state_showing(vec![choice(Some("a"), &["a", "b", "c"])]);
    assert_eq!(
        chosen(&first.step_surface_choice(0, ChoiceStep::Previous)),
        Some("c")
    );
    assert_eq!(
        chosen(&first.step_surface_choice(0, ChoiceStep::Next)),
        Some("b")
    );
}

// @internal
#[test]
fn a_choice_without_options_is_not_a_target() {
    assert_eq!(
        state_showing(vec![choice(None, &[])])
            .surface_targets()
            .len(),
        0
    );
    assert_eq!(
        state_showing(vec![choice(None, &["only"])])
            .surface_targets()
            .len(),
        1
    );
}

// @internal
#[test]
fn a_row_whose_only_control_is_disabled_cannot_be_reached() {
    let list = |row| vauchi_core::PresentationNode::List {
        style: vauchi_core::PresentationListStyle::Rows,
        id: vauchi_core::BindingId::new("settings").unwrap(),
        label: None,
        rows: vec![row],
        searchable: false,
        paging: None,
        accessibility: AccessibilitySpec::label("Settings"),
    };

    assert_eq!(
        state_showing(vec![list(toggle_row(false))])
            .surface_targets()
            .len(),
        0
    );
    assert_eq!(
        state_showing(vec![list(toggle_row(true))])
            .surface_targets()
            .len(),
        1
    );
}
