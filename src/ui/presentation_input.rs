// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Keyboard-to-event adapter for Core's generic presentation protocol.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use vauchi_core::{
    ActionSpec, BindingId, Event, InputValue, PresentationNode, StandardShortcut, SurfaceId,
};

use super::presentation_protocol::PresentationState;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct InteractionState {
    context_index: usize,
    overlay_index: usize,
    /// Selected row and the surface it belongs to, so navigating away cannot
    /// carry an index onto a surface that never had that row.
    surface_row: Option<(SurfaceId, usize)>,
    focused_binding: Option<BindingId>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum KeyOutcome {
    Events(Vec<Event>),
    Consumed,
    Quit,
}

impl InteractionState {
    pub(crate) fn key_outcome(&mut self, state: &PresentationState, key: KeyEvent) -> KeyOutcome {
        if let Some(outcome) = self.overlay_outcome(state, key) {
            return outcome;
        }
        if let Some(outcome) = self.surface_list_outcome(state, key) {
            return outcome;
        }
        if key.code == KeyCode::Esc {
            return self.back_outcome(state);
        }
        if key.code == KeyCode::Enter {
            if let Some(index) = self.selected_surface_row(state) {
                return events_outcome(state.activate_surface_row(index));
            }
            // Return in a field is the terminal's submit gesture, and the
            // only one available here — there is no pointer to click away
            // with. Core decides whether the screen does anything with
            // it; where nothing does, the primary action still runs
            // because Core answers with no command.
            if let Some(outcome) = self.submit_outcome(state) {
                return outcome;
            }
            return events_outcome(
                state.activation_events(state.context_bar().and_then(|bar| bar.primary.as_ref())),
            );
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('z') {
            return events_outcome(
                state.activation_events(shortcut_action(state, StandardShortcut::Undo)),
            );
        }
        if key.modifiers.contains(KeyModifiers::ALT) {
            let action = match key.code {
                KeyCode::Char('m') => state.context_bar().and_then(|bar| bar.navigation.as_ref()),
                KeyCode::Char('a') => state.context_bar().and_then(|bar| bar.secondary.as_ref()),
                _ => None,
            };
            if action.is_some() {
                return events_outcome(state.activation_events(action));
            }
        }
        if key.code == KeyCode::Tab {
            let count = state.context_actions().len();
            if count > 0 {
                self.context_index = (self.context_index + 1) % count;
            }
            return KeyOutcome::Consumed;
        }
        self.value_outcome(state, key)
            .unwrap_or(KeyOutcome::Consumed)
    }

    pub(crate) fn selected_context(&self) -> usize {
        self.context_index
    }

    pub(crate) fn selected_overlay(&self) -> usize {
        self.overlay_index
    }

    pub(crate) fn selected_surface_row(&self, state: &PresentationState) -> Option<usize> {
        let (owner, index) = self.surface_row.as_ref()?;
        let surface = state.surface()?;
        (surface.surface_id == *owner && *index < state.surface_list_rows().len()).then_some(*index)
    }

    fn surface_list_outcome(
        &mut self,
        state: &PresentationState,
        key: KeyEvent,
    ) -> Option<KeyOutcome> {
        let count = state.surface_list_rows().len();
        if count == 0 {
            return None;
        }
        let current = self.selected_surface_row(state);
        let awaits_text = self.focused_input(state).is_some();
        match key.code {
            KeyCode::Up => {
                let previous = current.unwrap_or(0).checked_sub(1).unwrap_or(count - 1);
                self.select_surface_row(state, previous);
                Some(KeyOutcome::Consumed)
            }
            KeyCode::Down => {
                self.select_surface_row(state, current.map_or(0, |index| (index + 1) % count));
                Some(KeyOutcome::Consumed)
            }
            // A row shortcut must never outrank a field waiting for the same
            // keystroke, or digits become untypeable wherever a list is shown.
            KeyCode::Char(digit) if digit.is_ascii_digit() && digit != '0' && !awaits_text => {
                let index = digit.to_digit(10).unwrap_or(1) as usize - 1;
                if index >= count {
                    return Some(KeyOutcome::Consumed);
                }
                self.select_surface_row(state, index);
                Some(events_outcome(state.activate_surface_row(index)))
            }
            _ => None,
        }
    }

    fn select_surface_row(&mut self, state: &PresentationState, index: usize) {
        if let Some(surface) = state.surface() {
            self.surface_row = Some((surface.surface_id.clone(), index));
        }
    }

    fn focused_input(&self, state: &PresentationState) -> Option<InputTarget> {
        find_input(&state.surface()?.nodes, self.focused_binding.as_ref())
    }

    fn overlay_outcome(&mut self, state: &PresentationState, key: KeyEvent) -> Option<KeyOutcome> {
        let overlay = state.overlay()?;
        let count = overlay.items.len();
        match key.code {
            KeyCode::Esc => {
                let surface_id = state.surface()?.surface_id.clone();
                Some(KeyOutcome::Events(vec![
                    Event::SurfaceActivated {
                        surface_id: surface_id.clone(),
                    },
                    Event::OverlayDismissed {
                        surface_id,
                        kind: overlay.kind,
                    },
                ]))
            }
            KeyCode::Up | KeyCode::BackTab => {
                if count > 0 {
                    self.overlay_index = self.overlay_index.checked_sub(1).unwrap_or(count - 1);
                }
                Some(KeyOutcome::Consumed)
            }
            KeyCode::Down | KeyCode::Tab => {
                if count > 0 {
                    self.overlay_index = (self.overlay_index + 1) % count;
                }
                Some(KeyOutcome::Consumed)
            }
            KeyCode::Enter => Some(events_outcome(state.activate_overlay(self.overlay_index))),
            KeyCode::Char(digit) if digit.is_ascii_digit() && digit != '0' => {
                let index = digit.to_digit(10).unwrap_or(1) as usize - 1;
                Some(events_outcome(state.activate_overlay(index)))
            }
            _ => Some(KeyOutcome::Consumed),
        }
    }

    fn back_outcome(&self, state: &PresentationState) -> KeyOutcome {
        let Some(surface_id) = state.surface().map(|surface| surface.surface_id.clone()) else {
            return KeyOutcome::Quit;
        };
        KeyOutcome::Events(vec![
            Event::SurfaceActivated {
                surface_id: surface_id.clone(),
            },
            Event::BackRequested { surface_id },
        ])
    }

    /// Report Return in a focused field. `None` when no field is focused,
    /// so Return keeps activating the primary action on screens without
    /// text entry.
    fn submit_outcome(&self, state: &PresentationState) -> Option<KeyOutcome> {
        let binding_id = self.focused_binding.clone()?;
        let surface_id = state.surface()?.surface_id.clone();
        Some(events_outcome(vec![Event::InputSubmitted {
            surface_id,
            binding_id,
        }]))
    }

    fn value_outcome(&mut self, state: &PresentationState, key: KeyEvent) -> Option<KeyOutcome> {
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return None;
        }
        let input = find_input(&state.surface()?.nodes, self.focused_binding.as_ref())?;
        self.focused_binding = Some(input.binding_id.clone());
        let mut value = input.value;
        match key.code {
            KeyCode::Char(character) if value.chars().count() < input.max_length => {
                value.push(character);
            }
            KeyCode::Backspace => {
                value.pop();
            }
            _ => return None,
        }
        let surface_id = state.surface()?.surface_id.clone();
        Some(KeyOutcome::Events(vec![
            Event::SurfaceActivated {
                surface_id: surface_id.clone(),
            },
            Event::ValueChanged {
                surface_id,
                binding_id: input.binding_id,
                value: InputValue::Text(value),
            },
        ]))
    }
}

fn shortcut_action(state: &PresentationState, shortcut: StandardShortcut) -> Option<&ActionSpec> {
    state
        .context_actions()
        .into_iter()
        .find(|action| action.shortcut == Some(shortcut))
}

fn events_outcome(events: Vec<Event>) -> KeyOutcome {
    if events.is_empty() {
        KeyOutcome::Consumed
    } else {
        KeyOutcome::Events(events)
    }
}

struct InputTarget {
    binding_id: BindingId,
    value: String,
    max_length: usize,
}

fn find_input(nodes: &[PresentationNode], focused: Option<&BindingId>) -> Option<InputTarget> {
    let mut inputs = Vec::new();
    collect_inputs(nodes, &mut inputs);
    inputs
        .iter()
        .find(|input| focused == Some(&input.binding_id))
        .or_else(|| inputs.first())
        .map(|input| InputTarget {
            binding_id: input.binding_id.clone(),
            value: input.value.clone(),
            max_length: input.max_length,
        })
}

fn collect_inputs(nodes: &[PresentationNode], inputs: &mut Vec<InputTarget>) {
    for node in nodes {
        match node {
            PresentationNode::Input {
                binding_id,
                value,
                max_length,
                enabled: true,
                ..
            } => inputs.push(InputTarget {
                binding_id: binding_id.clone(),
                value: value.clone(),
                max_length: max_length.unwrap_or(usize::MAX),
            }),
            PresentationNode::Group { children, .. } => collect_inputs(children, inputs),
            PresentationNode::List { rows, .. } => {
                for row in rows {
                    collect_inputs(&row.controls, inputs);
                }
            }
            _ => {}
        }
    }
}
