// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Keyboard-to-event adapter for Core's generic presentation protocol.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use vauchi_core::{ActionSpec, BindingId, Event, InputValue, PresentationNode, StandardShortcut};

use super::presentation_protocol::PresentationState;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct InteractionState {
    context_index: usize,
    overlay_index: usize,
    /// Selected row index in the active surface's actionable list(s), if any.
    surface_row_index: Option<usize>,
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
            if let Some(index) = self.surface_row_index {
                return events_outcome(state.activate_surface_row(index));
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

    pub(crate) fn selected_surface_row(&self) -> Option<usize> {
        self.surface_row_index
    }

    fn surface_list_outcome(
        &mut self,
        state: &PresentationState,
        key: KeyEvent,
    ) -> Option<KeyOutcome> {
        let rows = state.surface_list_rows();
        let count = rows.len();
        if count == 0 {
            return None;
        }
        match key.code {
            KeyCode::Up | KeyCode::BackTab => {
                self.surface_row_index = Some(
                    self.surface_row_index
                        .unwrap_or(0)
                        .checked_sub(1)
                        .unwrap_or(count - 1),
                );
                Some(KeyOutcome::Consumed)
            }
            KeyCode::Down | KeyCode::Tab => {
                let current = self.surface_row_index.unwrap_or(count - 1);
                self.surface_row_index = Some((current + 1) % count);
                Some(KeyOutcome::Consumed)
            }
            KeyCode::Char(digit) if digit.is_ascii_digit() && digit != '0' => {
                let index = digit.to_digit(10).unwrap_or(1) as usize - 1;
                if index < count {
                    self.surface_row_index = Some(index);
                    Some(events_outcome(state.activate_surface_row(index)))
                } else {
                    Some(KeyOutcome::Consumed)
                }
            }
            _ => None,
        }
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
