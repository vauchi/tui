// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Navigation screen handlers: setup, home, help.

use crossterm::event::KeyCode;

use crate::app::{
    App, BackupFocus, BackupMode, EditFieldState, InputMode, OnboardingState, Screen,
};
use crate::ui::widgets::key_mapping::{self, KeyResult};
use vauchi_core::aha_moments::AhaMomentType;
use vauchi_core::ui::{ActionResult, WorkflowEngine};

pub(super) fn handle_home_keys(app: &mut App, key: KeyCode) {
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
            app.add_field_state = Default::default();
            app.goto(Screen::AddField);
        }
        KeyCode::Char('e') | KeyCode::Enter => {
            // Edit selected field
            if let Ok(fields) = app.backend.get_card_fields() {
                if let Some(field) = fields.get(app.selected_field) {
                    app.edit_field_state = EditFieldState {
                        field_id: field.id.clone(),
                        field_label: field.label.clone(),
                        field_type: field.field_type.clone(),
                        new_value: field.value.clone(),
                    };
                    app.goto(Screen::EditField);
                } else {
                    // No fields, open Exchange
                    app.goto(Screen::Exchange);
                }
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let fields = app.backend.get_card_fields().unwrap_or_default();
            if app.selected_field < fields.len().saturating_sub(1) {
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
            if let Ok(fields) = app.backend.get_card_fields() {
                if let Some(field) = fields.get(app.selected_field) {
                    let label = field.label.clone();
                    if app.backend.remove_field(&field.id).is_ok() {
                        app.invalidate_engines();
                        app.set_status(format!("Field removed: {}", label));
                        if app.selected_field > 0 {
                            app.selected_field -= 1;
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

pub(super) fn handle_help_keys(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => {
            app.go_back();
        }
        _ => {}
    }
}

// ── SP-21 Onboarding Wizard Handlers ──
// Engine-driven: delegate to OnboardingEngine via key_mapping

/// Handle keys for all onboarding screens (engine-driven).
/// Falls back to legacy handlers when engine is not available.
pub(crate) fn handle_onboarding_engine_keys(app: &mut App, key: KeyCode) {
    // Handle Esc globally for onboarding: go back in engine or exit onboarding
    if key == KeyCode::Esc {
        app.go_back();
        return;
    }

    let engine = match app.onboarding_engine.as_mut() {
        Some(e) => e,
        None => return,
    };

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
            if key == KeyCode::Char('i') {
                if let Some(engine) = &app.onboarding_engine {
                    let screen_id = engine.current_screen().screen_id;
                    if screen_id == "welcome" || screen_id == "identity_check" {
                        app.backup_state.mode = BackupMode::Import;
                        app.backup_state.backup_data.clear();
                        app.backup_state.password.clear();
                        app.backup_state.focus = BackupFocus::Data;
                        app.input_mode = InputMode::Editing;
                        app.goto(Screen::Backup);
                    }
                }
            }
        }
    }
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
                    match app.backend.create_identity(&name) {
                        Ok(()) => {
                            app.invalidate_engines();
                            app.onboarding_state.identity_created = true;
                            if let Some(moment) = app
                                .backend
                                .check_aha_moment(AhaMomentType::CardCreationComplete)
                            {
                                app.set_status(format!(
                                    "★ {} — {}",
                                    moment.title(),
                                    moment.message()
                                ));
                            }
                        }
                        Err(e) => {
                            app.set_status(format!("Failed to create identity: {}", e));
                            return;
                        }
                    }
                }
            }
            app.onboarding_engine = None;
            app.onboarding_state = OnboardingState::default();
            app.goto(Screen::Home);
        }
        ActionResult::StartBackupImport => {
            app.backup_state.mode = BackupMode::Import;
            app.backup_state.backup_data.clear();
            app.backup_state.password.clear();
            app.backup_state.focus = BackupFocus::Data;
            app.input_mode = InputMode::Editing;
            app.goto(Screen::Backup);
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
        app.screen = tui_screen;
        app.render_state = crate::ui::widgets::screen_renderer::ScreenRenderState::default();
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
            app.backup_state.mode = BackupMode::Import;
            app.backup_state.backup_data.clear();
            app.backup_state.password.clear();
            app.backup_state.focus = BackupFocus::Data;
            app.input_mode = InputMode::Editing;
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
            match app.backend.create_identity(&name) {
                Ok(()) => {
                    app.invalidate_engines();
                    app.onboarding_state.identity_created = true;
                    if let Some(moment) = app
                        .backend
                        .check_aha_moment(AhaMomentType::CardCreationComplete)
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
            app.add_field_state = Default::default();
            app.goto(Screen::AddField);
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
        app.goto(Screen::Home);
    }
}
