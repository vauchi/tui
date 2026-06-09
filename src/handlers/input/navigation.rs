// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Navigation screen handlers: setup, home, help.

use crossterm::event::KeyCode;

use crate::app::{App, OnboardingState};
use crate::ui::widgets::key_mapping::{self, KeyResult};

use super::Action;
use vauchi_app::ui::{ActionResult, AppScreen, FormDialogType, WorkflowEngine};
use vauchi_core::types::AhaMomentType;

pub(super) fn handle_my_info_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('c') => app.goto(AppScreen::Contacts),
        KeyCode::Char('s') => app.goto(AppScreen::Settings),
        KeyCode::Char('d') => app.goto(AppScreen::DeviceManagement),
        KeyCode::Char('r') => app.goto(AppScreen::Recovery),
        KeyCode::Char('n') => app.goto(AppScreen::Sync),
        KeyCode::Char('y') => app.goto(AppScreen::DeliveryStatus),
        KeyCode::Char('b') => app.goto(AppScreen::Backup),
        KeyCode::Char('g') => app.goto(AppScreen::Groups),
        KeyCode::Char('X') => app.goto(AppScreen::Exchange),
        KeyCode::Char('a') => {
            let available_groups = app.app_engine.available_groups().into_iter().collect();
            app.goto_form_dialog(FormDialogType::AddField { available_groups });
        }
        KeyCode::Char('e') | KeyCode::Enter => {
            if let Ok(Some(card)) = app.app_engine.vauchi().own_card() {
                if let Some(field) = card.fields().get(app.selected_field) {
                    app.goto_form_dialog(FormDialogType::EditField {
                        field_id: field.id().to_string(),
                        field_label: field.label().to_string(),
                        current_value: field.value().to_string(),
                        current_note: None,
                    });
                } else {
                    app.goto(AppScreen::Exchange);
                }
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let field_count = app
                .app_engine
                .vauchi()
                .own_card()
                .ok()
                .flatten()
                .map(|c| c.fields().len())
                .unwrap_or(0);
            if app.selected_field < field_count.saturating_sub(1) {
                app.selected_field += 1;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.selected_field > 0 {
                app.selected_field -= 1;
            }
        }
        KeyCode::Char('x') | KeyCode::Delete => {
            // Delete selected field
            if let Ok(Some(card)) = app.app_engine.vauchi().own_card()
                && let Some(field) = card.fields().get(app.selected_field)
            {
                let label = field.label().to_string();
                let field_id = field.id().to_string();
                if app
                    .app_engine
                    .vauchi()
                    .remove_own_field_by_id(&field_id)
                    .is_ok()
                {
                    app.invalidate_engines();
                    app.set_status(format!("Field removed: {}", label));
                    if app.selected_field > 0 {
                        app.selected_field -= 1;
                    }
                }
            }
        }
        _ => {}
    }
}

// ── SP-21 Onboarding Wizard Handlers ──
// Engine-driven: delegate to OnboardingEngine via key_mapping

/// Handle keys for all onboarding screens (engine-driven).
/// Falls back to legacy handlers when engine is not available.
///
/// Returns `Some(Action::Quit)` when the user presses 'q' on the welcome
/// screen (which has no "back" destination). Returns `None` otherwise.
pub(crate) fn handle_onboarding_engine_keys(app: &mut App, key: KeyCode) -> Option<Action> {
    // Welcome screen: 'q' quits (no back to go to)
    let on_welcome = app
        .onboarding_engine
        .as_ref()
        .map(|e| {
            matches!(
                e.current_screen().screen_id.as_str(),
                "welcome" | "identity_check"
            )
        })
        .unwrap_or(false);
    if on_welcome && key == KeyCode::Char('q') {
        return Some(Action::Quit);
    }

    // Handle Esc for onboarding: go back (no-op on welcome screen)
    if key == KeyCode::Esc {
        app.go_back();
        return None;
    }

    let engine = app.onboarding_engine.as_mut()?;

    let screen = engine.current_screen();
    let key_result = key_mapping::map_key(key, &screen, &mut app.render_state);

    match key_result {
        KeyResult::Action(action) => {
            let screen_before = engine.current_screen().screen_id;
            let result = engine.handle_action(action);
            handle_onboarding_result(app, result);
            // The active step is derived from the engine; reset render
            // focus only when that step actually changed.
            if let Some(engine) = &app.onboarding_engine
                && engine.current_screen().screen_id != screen_before
            {
                app.render_state =
                    crate::ui::widgets::screen_renderer::ScreenRenderState::default();
            }
        }
        KeyResult::Consumed => {
            // Internal navigation (focus/selection change) — just re-render
        }
        KeyResult::Unhandled => {
            // Check for 'i' (import backup) on welcome/identity_check screen
            if key == KeyCode::Char('i')
                && let Some(engine) = &app.onboarding_engine
            {
                let screen_id = engine.current_screen().screen_id;
                if screen_id == "welcome" || screen_id == "identity_check" {
                    // Engine-driven: the Backup choose screen lets the user
                    // pick Restore (paste-restore via BackupRecoveryEngine).
                    app.goto(AppScreen::Backup);
                }
            }
        }
    }
    None
}

/// Handle ActionResult from the onboarding engine.
fn handle_onboarding_result(app: &mut App, result: ActionResult) {
    match result {
        ActionResult::UpdateScreen(_) | ActionResult::NavigateTo(_) => {
            // Engine state updated; the active onboarding step is derived
            // from the engine (`App::active_screen`). Render focus is reset
            // by the caller when the step changes.
        }
        ActionResult::Complete => {
            // Onboarding complete — persist data and go to Home
            if let Some(engine) = &app.onboarding_engine {
                let data = engine.data();
                let name = data.display_name.clone();
                if !name.is_empty() && !app.onboarding_state.identity_created {
                    // Create identity via AppEngine's Vauchi (single source of truth)
                    let identity_result = app
                        .app_engine
                        .vauchi_mut()
                        .create_identity(&name)
                        .map_err(|e| e.to_string());
                    match identity_result {
                        Ok(()) => {
                            app.onboarding_state.identity_created = true;
                            // Navigate AppEngine to Home
                            app.app_engine.navigate_to(AppScreen::MyInfo);
                            if let Ok(Some(moment)) = app
                                .app_engine
                                .vauchi()
                                .try_trigger_aha_moment(AhaMomentType::CardCreationComplete)
                            {
                                app.set_status(format!(
                                    "★ {} — {}",
                                    moment.title(),
                                    moment.message()
                                ));
                            }
                        }
                        Err(e) => {
                            app.set_status(format!("Failed to create identity: {e}"));
                            return;
                        }
                    }
                }
            }
            app.onboarding_engine = None;
            app.onboarding_state = OnboardingState::default();
            app.goto(AppScreen::MyInfo);
        }
        ActionResult::ValidationError {
            component_id,
            message,
        } => {
            app.render_state
                .set_validation_error(component_id, message.clone());
            app.set_status(message);
        }
        _ => {}
    }
}
