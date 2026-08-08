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

    #[cfg(test)]
    pub(crate) fn activate_context(&self, index: usize) -> Vec<Event> {
        self.activation_events(self.context_actions().get(index).copied())
    }

    pub(crate) fn activate_overlay(&self, index: usize) -> Vec<Event> {
        let action = self.overlay().and_then(|overlay| overlay.items.get(index));
        self.activation_events(action)
    }

    /// All list rows from the active surface that can be activated.
    pub(crate) fn surface_list_rows(&self) -> Vec<&vauchi_core::PresentationRow> {
        let Some(surface) = self.surface() else {
            return Vec::new();
        };
        let mut rows = Vec::new();
        collect_list_rows(&surface.nodes, &mut rows);
        rows
    }

    /// Activate a surface list row by its index in `surface_list_rows()`.
    pub(crate) fn activate_surface_row(&self, index: usize) -> Vec<Event> {
        let action = self
            .surface_list_rows()
            .get(index)
            .and_then(|row| row.activation.as_ref());
        self.activation_events(action)
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

fn collect_list_rows<'a>(
    nodes: &'a [vauchi_core::PresentationNode],
    rows: &mut Vec<&'a vauchi_core::PresentationRow>,
) {
    use vauchi_core::PresentationNode;
    for node in nodes {
        match node {
            PresentationNode::List {
                rows: list_rows, ..
            } => {
                for row in list_rows {
                    if row.activation.is_some() {
                        rows.push(row);
                    }
                }
            }
            PresentationNode::Group { children, .. } => collect_list_rows(children, rows),
            _ => {}
        }
    }
}
