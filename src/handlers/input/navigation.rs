// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Navigation screen handlers: setup, home, help.

use crossterm::event::KeyCode;

use crate::app::{App, InputMode, OnboardingState, Screen};
use crate::ui::widgets::key_mapping::{self, KeyResult};

use super::Action;
use vauchi_app::ui::{ActionResult, AppScreen, FormDialogType, WorkflowEngine};
use vauchi_core::types::AhaMomentType;

pub(super) fn handle_my_info_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('c') => app.goto(Screen::Contacts),
        KeyCode::Char('s') => app.goto(Screen::Settings),
        KeyCode::Char('d') => app.goto(Screen::Devices),
        KeyCode::Char('r') => app.goto(Screen::Recovery),
        KeyCode::Char('n') => app.goto(Screen::Sync),
        KeyCode::Char('y') => app.goto(Screen::Delivery),
        KeyCode::Char('b') => app.goto(Screen::Backup),
        KeyCode::Char('g') => app.goto(Screen::Groups),
        KeyCode::Char('X') => app.goto(Screen::Exchange),
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
                    app.goto(Screen::Exchange);
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
    if app.screen == Screen::SetupWelcome && key == KeyCode::Char('q') {
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
            let result = engine.handle_action(action);
            handle_onboarding_result(app, result);
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
                    app.goto(Screen::Backup);
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
            // Engine state updated — sync TUI screen to engine step
            sync_onboarding_screen(app);
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
            app.goto(Screen::MyInfo);
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

/// Map onboarding engine screen_id to TUI Screen variant.
fn sync_onboarding_screen(app: &mut App) {
    if let Some(engine) = &app.onboarding_engine {
        let screen_id = engine.current_screen().screen_id;
        let tui_screen = match screen_id.as_str() {
            "identity_check" | "welcome" => Screen::SetupWelcome,
            "default_name" => Screen::SetupCreateIdentity,
            "skip_gate" | "groups_setup" | "contact_info" | "preview_card" => {
                Screen::SetupAddFields
            }
            "security_explanation" | "backup_prompt" => Screen::SetupSecurity,
            "ready" => Screen::SetupReady,
            _ => Screen::SetupWelcome,
        };
        // Only reset render state when navigating to a different screen
        if app.active_screen() != tui_screen {
            app.render_state = crate::ui::widgets::screen_renderer::ScreenRenderState::default();
        }
        app.screen = tui_screen;
    }
}

// Legacy handlers (used when engine is not initialized)
pub(super) fn handle_setup_welcome_keys(app: &mut App, key: KeyCode) {
    if app.onboarding_engine.is_some() {
        handle_onboarding_engine_keys(app, key);
        return;
    }
    match key {
        KeyCode::Enter => {
            app.onboarding_state = OnboardingState::default();
            app.goto(Screen::SetupCreateIdentity);
            app.input_mode = InputMode::Editing;
        }
        KeyCode::Char('i') => {
            // Engine-driven: the Backup choose screen lets the user pick
            // Restore (paste-restore via BackupRecoveryEngine).
            app.goto(Screen::Backup);
        }
        _ => {}
    }
}

pub(super) fn handle_setup_create_identity_keys(app: &mut App, key: KeyCode) {
    if app.onboarding_engine.is_some() {
        handle_onboarding_engine_keys(app, key);
        return;
    }
    match key {
        KeyCode::Enter => {
            let name = app.onboarding_state.name_input.trim().to_string();
            if name.is_empty() {
                app.set_status("Please enter your name");
                return;
            }
            match app.app_engine.vauchi_mut().create_identity(&name) {
                Ok(()) => {
                    app.invalidate_engines();
                    app.onboarding_state.identity_created = true;
                    if let Ok(Some(moment)) = app
                        .app_engine
                        .vauchi()
                        .try_trigger_aha_moment(AhaMomentType::CardCreationComplete)
                    {
                        app.set_status(format!("★ {} — {}", moment.title(), moment.message()));
                    }
                    app.goto(Screen::SetupAddFields);
                }
                Err(e) => {
                    app.set_status(format!("Failed to create identity: {}", e));
                }
            }
        }
        KeyCode::Backspace => {
            app.onboarding_state.name_input.pop();
        }
        KeyCode::Char(c) => {
            if app.onboarding_state.name_input.len() < 50 {
                app.onboarding_state.name_input.push(c);
            }
        }
        _ => {}
    }
}

pub(super) fn handle_setup_add_fields_keys(app: &mut App, key: KeyCode) {
    if app.onboarding_engine.is_some() {
        handle_onboarding_engine_keys(app, key);
        return;
    }
    match key {
        KeyCode::Char('a') => {
            let available_groups = app.app_engine.available_groups().into_iter().collect();
            app.goto_form_dialog(FormDialogType::AddField { available_groups });
        }
        KeyCode::Char('s') | KeyCode::Enter => {
            app.goto(Screen::SetupSecurity);
        }
        _ => {}
    }
}

pub(super) fn handle_setup_security_keys(app: &mut App, key: KeyCode) {
    if app.onboarding_engine.is_some() {
        handle_onboarding_engine_keys(app, key);
        return;
    }
    if let KeyCode::Enter = key {
        app.goto(Screen::SetupReady);
    }
}

pub(super) fn handle_setup_ready_keys(app: &mut App, key: KeyCode) {
    if app.onboarding_engine.is_some() {
        handle_onboarding_engine_keys(app, key);
        return;
    }
    if let KeyCode::Enter = key {
        app.onboarding_state = OnboardingState::default();
        app.goto(Screen::MyInfo);
    }
}
