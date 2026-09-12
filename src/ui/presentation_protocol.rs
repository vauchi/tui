// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
// SPDX-License-Identifier: GPL-3.0-or-later

//! Terminal-owned projection state for Core's generic presentation protocol.

use std::collections::HashMap;

use vauchi_core::{
    ActionSpec, Command, ContextBar, Event, OverlaySpec, PaneLayout, PresentationProfile,
    SurfaceId, SurfaceSpec,
};

#[derive(Default)]
pub(crate) struct PresentationState {
    surfaces: HashMap<SurfaceId, SurfaceSpec>,
    context_bars: HashMap<SurfaceId, (u64, ContextBar)>,
    overlays: HashMap<SurfaceId, (u64, OverlaySpec)>,
    profile: Option<PresentationProfile>,
    last_surface: Option<SurfaceId>,
    native_back_requested: bool,
}

impl PresentationState {
    pub(crate) fn apply(&mut self, commands: &[Command]) -> Vec<Command> {
        let mut effects = Vec::new();
        for command in commands {
            match command {
                Command::ReplaceSurface { surface } => self.replace_surface(surface.clone()),
                Command::SetContextBar {
                    surface_id,
                    revision,
                    bar,
                } if self.is_current_revision(surface_id, *revision) => {
                    self.context_bars
                        .insert(surface_id.clone(), (*revision, (**bar).clone()));
                }
                Command::PresentOverlay {
                    surface_id,
                    revision,
                    overlay,
                } if self.is_current_revision(surface_id, *revision) => {
                    self.overlays
                        .insert(surface_id.clone(), (*revision, overlay.clone()));
                }
                // Core rewrites a repeat PresentOverlay into this so the
                // context-bar buttons toggle. Matching on kind as well as
                // surface keeps a stale dismiss from closing an overlay Core
                // has since replaced.
                Command::DismissOverlay {
                    surface_id, kind, ..
                } if self
                    .overlays
                    .get(surface_id)
                    .is_some_and(|(_, open)| open.kind == *kind) =>
                {
                    self.overlays.remove(surface_id);
                }
                Command::SetContextBar { .. }
                | Command::PresentOverlay { .. }
                | Command::DismissOverlay { .. } => {}
                Command::SetPresentationProfile { profile } => {
                    self.profile = Some(profile.clone());
                }
                Command::PerformNativeBack => self.native_back_requested = true,
                effect => effects.push(effect.clone()),
            }
        }
        effects
    }

    pub(crate) fn surface(&self) -> Option<&SurfaceSpec> {
        self.active_surface_id()
            .and_then(|surface_id| self.surfaces.get(surface_id))
    }

    pub(crate) fn visible_surfaces(&self) -> Vec<&SurfaceSpec> {
        let Some(profile) = &self.profile else {
            return self.surface().into_iter().collect();
        };
        if profile.pane_layout == PaneLayout::Split {
            let mut surfaces = Vec::with_capacity(2);
            if let Some(primary) = self.surfaces.get(&profile.primary_surface) {
                surfaces.push(primary);
            }
            if let Some(detail_id) = &profile.detail_surface
                && let Some(detail) = self.surfaces.get(detail_id)
            {
                surfaces.push(detail);
            }
            surfaces
        } else {
            self.surface().into_iter().collect()
        }
    }

    // INLINE_TEST_REQUIRED: reducer invariants need visibility into retained
    // surfaces and Core-derived profile state without widening production APIs.
    #[cfg(test)]
    pub(crate) fn retained_surface_count(&self) -> usize {
        self.surfaces.len()
    }

    pub(crate) fn context_bar(&self) -> Option<&ContextBar> {
        self.active_surface_id()
            .and_then(|surface_id| self.context_bars.get(surface_id))
            .map(|(_, bar)| bar)
    }

    pub(crate) fn overlay(&self) -> Option<&OverlaySpec> {
        self.active_surface_id()
            .and_then(|surface_id| self.overlays.get(surface_id))
            .map(|(_, overlay)| overlay)
    }

    #[cfg(test)]
    pub(crate) fn profile(&self) -> Option<&PresentationProfile> {
        self.profile.as_ref()
    }

    pub(crate) fn native_back_requested(&self) -> bool {
        self.native_back_requested
    }

    pub(crate) fn context_actions(&self) -> Vec<&ActionSpec> {
        let Some(bar) = self.context_bar() else {
            return Vec::new();
        };
        [
            bar.back.as_ref(),
            bar.navigation.as_ref(),
            bar.primary.as_ref(),
            bar.secondary.as_ref(),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    /// The first activatable `Status` node on the active surface — the
    /// sync chrome chip every other shell taps. Nothing else in the key map
    /// reaches a status node, so this is the terminal's only way to it.
    pub(crate) fn status_activation(&self) -> Option<&ActionSpec> {
        let surface = self.surface()?;
        first_status_activation(&surface.nodes)
    }

    #[cfg(test)]
    pub(crate) fn activate_context(&self, index: usize) -> Vec<Event> {
        self.activation_events(self.context_actions().get(index).copied())
    }

    pub(crate) fn activate_overlay(&self, index: usize) -> Vec<Event> {
        let action = self.overlay().and_then(|overlay| overlay.items.get(index));
        self.activation_events(action)
    }

    /// Everything on the active surface the row keys can land on — list
    /// rows and choices — in the order the renderer paints them, so the
    /// highlight and the keys agree on which thing is next.
    pub(crate) fn surface_targets(&self) -> Vec<SurfaceTarget<'_>> {
        let Some(surface) = self.surface() else {
            return Vec::new();
        };
        let mut targets = Vec::new();
        collect_targets(&surface.nodes, &mut targets);
        targets
    }

    /// Activate a surface target by its index in `surface_targets()`.
    ///
    /// A row whose operable thing is a control reports a value rather than
    /// an action: Core names the setting in the row title and sends the
    /// toggle with no activation of its own, so there is no interaction id
    /// to send and nothing would happen if we sent one. A choice has no
    /// activation either; Enter steps it to its next option.
    pub(crate) fn activate_surface_target(&self, index: usize) -> Vec<Event> {
        let targets = self.surface_targets();
        match targets.get(index) {
            Some(SurfaceTarget::Choice(choice)) => {
                self.value_events(choice.binding_id.clone(), choice.stepped(ChoiceStep::Next))
            }
            Some(SurfaceTarget::Row(row)) => match row_toggle(row) {
                Some((binding_id, value)) => {
                    self.value_events(binding_id, vauchi_core::InputValue::Boolean(!value))
                }
                None => self.activation_events(row.activation.as_ref()),
            },
            None => Vec::new(),
        }
    }

    /// Move the choice at `index` one option along; empty when the target
    /// is not a choice, so the arrows can fall through to whatever else
    /// wants them.
    pub(crate) fn step_surface_choice(&self, index: usize, step: ChoiceStep) -> Vec<Event> {
        let targets = self.surface_targets();
        let Some(SurfaceTarget::Choice(choice)) = targets.get(index) else {
            return Vec::new();
        };
        self.value_events(choice.binding_id.clone(), choice.stepped(step))
    }

    fn value_events(
        &self,
        binding_id: vauchi_core::BindingId,
        value: vauchi_core::InputValue,
    ) -> Vec<Event> {
        let Some(surface_id) = self.active_surface_id() else {
            return Vec::new();
        };
        vec![
            Event::SurfaceActivated {
                surface_id: surface_id.clone(),
            },
            Event::ValueChanged {
                surface_id: surface_id.clone(),
                binding_id,
                value,
            },
        ]
    }

    pub(crate) fn activation_events(&self, action: Option<&ActionSpec>) -> Vec<Event> {
        let (Some(action), Some(surface_id)) = (action, self.active_surface_id()) else {
            return Vec::new();
        };
        if !action.enabled {
            return Vec::new();
        }
        vec![
            Event::SurfaceActivated {
                surface_id: surface_id.clone(),
            },
            Event::ActionActivated {
                surface_id: surface_id.clone(),
                interaction_id: action.interaction_id.clone(),
            },
        ]
    }

    fn active_surface_id(&self) -> Option<&SurfaceId> {
        self.profile
            .as_ref()
            .map(|profile| &profile.active_surface)
            .filter(|surface_id| self.surfaces.contains_key(*surface_id))
            .or(self.last_surface.as_ref())
    }

    fn replace_surface(&mut self, candidate: SurfaceSpec) {
        let surface_id = candidate.surface_id.clone();
        let stale = self
            .surfaces
            .get(&surface_id)
            .is_some_and(|current| current.revision > candidate.revision);
        if !stale {
            self.surfaces.insert(surface_id.clone(), candidate);
            self.context_bars.remove(&surface_id);
            self.overlays.remove(&surface_id);
            self.last_surface = Some(surface_id);
        }
    }

    fn is_current_revision(&self, surface_id: &SurfaceId, revision: u64) -> bool {
        self.surfaces
            .get(surface_id)
            .is_some_and(|surface| surface.revision == revision)
    }
}

fn first_status_activation(nodes: &[vauchi_core::PresentationNode]) -> Option<&ActionSpec> {
    use vauchi_core::PresentationNode;
    nodes.iter().find_map(|node| match node {
        PresentationNode::Status {
            activation: Some(action),
            ..
        } if action.enabled => Some(action),
        PresentationNode::Group { children, .. } => first_status_activation(children),
        _ => None,
    })
}

pub(crate) enum SurfaceTarget<'a> {
    Row(&'a vauchi_core::PresentationRow),
    Choice(ChoiceTarget<'a>),
}

/// An enabled choice with something to pick from.
pub(crate) struct ChoiceTarget<'a> {
    pub(crate) binding_id: &'a vauchi_core::BindingId,
    pub(crate) selected: Option<&'a str>,
    pub(crate) options: &'a [vauchi_core::ChoiceOption],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChoiceStep {
    Previous,
    Next,
}

impl ChoiceTarget<'_> {
    /// The option one step from the current selection, wrapping at both
    /// ends. With nothing selected, Next lands on the first option and
    /// Previous on the last — the ends nearest each key.
    fn stepped(&self, step: ChoiceStep) -> vauchi_core::InputValue {
        let count = self.options.len();
        let current = self
            .selected
            .and_then(|selected| self.options.iter().position(|option| option.id == selected));
        let index = match (step, current) {
            (ChoiceStep::Next, Some(current)) => (current + 1) % count,
            (ChoiceStep::Next, None) => 0,
            (ChoiceStep::Previous, Some(current)) => current.checked_sub(1).unwrap_or(count - 1),
            (ChoiceStep::Previous, None) => count - 1,
        };
        vauchi_core::InputValue::Choice(Some(self.options[index].id.clone()))
    }
}

/// The choice a node offers, if the keyboard can operate it at all.
///
/// Shared by the renderer and the input path so the line the highlight
/// lands on is exactly the choice the arrows step.
pub(crate) fn choice_target(node: &vauchi_core::PresentationNode) -> Option<ChoiceTarget<'_>> {
    match node {
        vauchi_core::PresentationNode::Choice {
            binding_id,
            selected,
            options,
            enabled: true,
            ..
        } if !options.is_empty() => Some(ChoiceTarget {
            binding_id,
            selected: selected.as_deref(),
            options,
        }),
        _ => None,
    }
}

fn collect_targets<'a>(
    nodes: &'a [vauchi_core::PresentationNode],
    targets: &mut Vec<SurfaceTarget<'a>>,
) {
    use vauchi_core::PresentationNode;
    for node in nodes {
        match node {
            PresentationNode::List { rows, .. } => {
                targets.extend(
                    rows.iter()
                        .filter(|row| row_is_addressable(row))
                        .map(SurfaceTarget::Row),
                );
            }
            PresentationNode::Group { children, .. } => collect_targets(children, targets),
            _ => targets.extend(choice_target(node).map(SurfaceTarget::Choice)),
        }
    }
}

/// The enabled toggle a row carries, if any — its binding and current value.
///
/// Shared by the renderer and the input path so the row the highlight lands
/// on is exactly the row Enter operates.
pub(crate) fn row_toggle(
    row: &vauchi_core::PresentationRow,
) -> Option<(vauchi_core::BindingId, bool)> {
    row.controls.iter().find_map(|control| match control {
        vauchi_core::PresentationNode::Toggle {
            binding_id,
            value,
            enabled,
            ..
        } if *enabled => Some((binding_id.clone(), *value)),
        _ => None,
    })
}

/// Whether the keyboard can reach this row at all.
pub(crate) fn row_is_addressable(row: &vauchi_core::PresentationRow) -> bool {
    row.activation.is_some() || row_toggle(row).is_some()
}
