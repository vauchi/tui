// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Screen navigation — `goto()`, `go_back()`, and nav-index sync.

use vauchi_app::ui::{AppScreen, FormDialogType, LockScreenEngine, OnboardingEngine};

use super::App;
use super::state::*;
use crate::ui::widgets::screen_renderer::ScreenRenderState;

impl App {
    /// The current `AppScreen` — the engine is the single source of
    /// truth. `app_engine` tracks every screen including Onboarding and
    /// Lock (set at init, navigated by `goto` / onboarding completion /
    /// unlock), so no TUI-local screen mirror is needed.
    pub fn current_app_screen(&self) -> AppScreen {
        // app_engine is the single source of truth — it tracks Onboarding
        // and Lock too (set at init, navigated by goto / completion).
        self.app_engine.current_app_screen().clone()
    }

    /// Navigate to a `FormDialog` AppScreen with explicit `dialog_type`.
    ///
    /// Bypasses the legacy `Screen → AppScreen` mapping so callers no
    /// longer need to populate per-form state structs (`add_field_state`,
    /// `edit_field_state`, etc.) before navigation. The engine's
    /// `FormDialogEngine` owns the form values and is now the single
    /// source of truth for these dialogs.
    pub fn goto_form_dialog(&mut self, dialog_type: FormDialogType) {
        // FormDialogType is `#[non_exhaustive]`; only navigate for the
        // dialog kinds this frontend renders. All collapse to the single
        // `Screen::FormDialog`; the kind lives in the engine's AppScreen.
        match &dialog_type {
            FormDialogType::AddField { .. }
            | FormDialogType::EditField { .. }
            | FormDialogType::EditName { .. }
            | FormDialogType::EditRelayUrl { .. }
            | FormDialogType::CreateGroup
            | FormDialogType::RenameGroup { .. } => {}
            _ => return,
        }
        if self.current_app_screen() == AppScreen::Contacts {
            self.contact_search_mode = false;
            self.contact_search_query.clear();
        }
        self.input_mode = InputMode::Normal;
        // Navigate the engine first so `sync_nav_index` — which derives the
        // active tab from `AppScreen::FormDialog { dialog_type }` — observes
        // the dialog kind rather than the screen we are leaving.
        self.app_engine
            .navigate_to(AppScreen::FormDialog { dialog_type });
        self.sync_nav_index();
        self.render_state = ScreenRenderState::default();
    }

    /// Navigate to an `AppScreen`. Dismisses any overlay, navigates the
    /// engine (the single source of truth), ensures the dedicated
    /// onboarding/lock engine exists, and resets render + nav state.
    pub fn goto(&mut self, target: AppScreen) {
        self.overlay = None;
        self.clear_contact_search_if_leaving(&target);
        self.input_mode = InputMode::Normal;
        self.app_engine.navigate_to(target.clone());
        self.render_state = ScreenRenderState::default();
        self.ensure_screen_engine(&target);
        self.sync_nav_index();
    }

    fn clear_contact_search_if_leaving(&mut self, target: &AppScreen) {
        if self.current_app_screen() == AppScreen::Contacts && *target != AppScreen::Contacts {
            self.contact_search_mode = false;
            self.contact_search_query.clear();
        }
    }

    /// Lazily creates the dedicated engines that don't live on AppEngine
    /// (Onboarding, Lock). Resets render state when such a screen is entered.
    fn ensure_screen_engine(&mut self, target: &AppScreen) {
        match target {
            AppScreen::Onboarding => {
                if self.onboarding_engine.is_none() {
                    self.onboarding_engine = Some(OnboardingEngine::new());
                }
                self.render_state = ScreenRenderState::default();
            }
            AppScreen::Lock => {
                if self.lock_engine.is_none() {
                    self.lock_engine = Some(LockScreenEngine::new(
                        vauchi_app::ui::DEFAULT_LOCK_MAX_ATTEMPTS,
                    ));
                }
                self.render_state = ScreenRenderState::default();
            }
            _ => {}
        }
    }

    /// Go back to the previous screen.
    ///
    /// Engine-driven screens delegate to `app_engine.navigate_back()`,
    /// which pops `nav_history` and restores the prior `AppScreen`. The
    /// resulting `AppScreen` is reverse-mapped onto `self.screen` via
    /// `sync_screen_from_engine`. Setup wizard, Lock, and the
    /// ActionMenu overlay are TUI-only states that AppEngine doesn't
    /// know about, so they keep explicit dispatch.
    ///
    /// Per-screen state-struct resets that the inline arms used to do
    /// (PrivacyState::default, EmergencyState::default, etc.) live on
    /// the engine side now — the engine rebuilds the parent screen
    /// fresh after `navigate_back`. Local UI-only resets that don't
    /// roundtrip through the engine (`selected_contact_id`,
    /// `selected_visibility_field`) clear here.
    pub fn go_back(&mut self) {
        // An open overlay swallows "back": close it, stay on the screen beneath.
        if self.overlay.is_some() {
            self.close_overlay();
            return;
        }
        match self.current_app_screen() {
            // Dedicated-engine states with no engine back-history: "back"
            // is a no-op (onboarding advances via its own engine).
            AppScreen::Onboarding | AppScreen::Lock => {}

            // Bootstrap (no identity yet): Backup-restore and the AddField
            // form dialog have no engine history to pop, so route back to
            // the onboarding flow explicitly.
            AppScreen::Backup if !self.app_engine.vauchi().has_identity() => {
                self.goto(AppScreen::Onboarding);
            }
            AppScreen::FormDialog {
                dialog_type: FormDialogType::AddField { .. },
            } if self.onboarding_state.identity_created
                && !self.app_engine.vauchi().has_identity() =>
            {
                self.goto(AppScreen::Onboarding);
            }

            // Engine-driven screens — delegate to AppEngine.
            _ => {
                self.clear_screen_local_state();
                self.app_engine.navigate_back();
                self.sync_screen_from_engine();
            }
        }
    }

    /// Close any open TUI overlay, resetting its transient state.
    pub fn close_overlay(&mut self) {
        self.overlay = None;
        self.action_menu_state = ActionMenuState::default();
    }

    /// Clear TUI-presentation-only state for the screen we're leaving
    /// via engine-driven back-navigation. Mirrors the inline resets the
    /// old arm-based `go_back` did before delegating to `goto()`.
    fn clear_screen_local_state(&mut self) {
        match self.current_app_screen() {
            AppScreen::ContactDetail { .. } => self.selected_contact_id = None,
            AppScreen::ContactEdit { .. } => self.render_state = ScreenRenderState::default(),
            AppScreen::ContactVisibility { .. } => self.selected_visibility_field = 0,
            _ => {}
        }
    }

    /// Reverse map `app_engine.current_app_screen()` onto `self.screen`
    /// after engine-driven navigation (`navigate_back`,
    /// engine-emitted `NavigateTo` results). Inverse of
    /// `engine_target_for_screen`. Falls back to MyInfo for screens
    /// AppEngine has but TUI doesn't (or hasn't migrated yet).
    pub(crate) fn sync_screen_from_engine(&mut self) {
        let contact_id = match self.app_engine.current_app_screen() {
            AppScreen::ContactDetail { contact_id }
            | AppScreen::ContactEdit { contact_id }
            | AppScreen::ContactVisibility { contact_id }
            | AppScreen::VerifyFingerprint { contact_id } => Some(contact_id.clone()),
            _ => None,
        };
        if let Some(contact_id) = contact_id {
            self.selected_contact_id = Some(contact_id);
        }
        self.input_mode = InputMode::Normal;
        self.sync_nav_index();
        self.render_state = ScreenRenderState::default();
    }

    /// Keep `focus.nav_index` in sync with the current screen so that
    /// Left/Right navigation from Content zone lands on the correct tab.
    pub(crate) fn sync_nav_index(&mut self) {
        if let Some(idx) = self.nav_index_for_screen() {
            self.focus.nav_index = idx;
        }
    }

    /// Maps the current screen (or its parent tab) to a NavBar index.
    /// Returns `None` for screens that don't correspond to a top-level tab
    /// (onboarding, lock, etc.).
    fn nav_index_for_screen(&self) -> Option<usize> {
        match self.current_app_screen() {
            AppScreen::MyInfo | AppScreen::MyInfoEntryDetail { .. } => Some(0),
            AppScreen::FormDialog { .. } => Some(self.form_dialog_nav_index()),
            AppScreen::Contacts
            | AppScreen::ContactDetail { .. }
            | AppScreen::ContactEdit { .. }
            | AppScreen::ContactVisibility { .. }
            | AppScreen::ContactDuplicates
            | AppScreen::ContactMerge { .. }
            | AppScreen::ContactLimit
            | AppScreen::VerifyFingerprint { .. } => Some(1),
            AppScreen::Exchange => Some(2),
            AppScreen::Groups | AppScreen::GroupDetail { .. } => Some(3),
            AppScreen::More
            | AppScreen::Settings
            | AppScreen::Help
            | AppScreen::Privacy
            | AppScreen::Support
            | AppScreen::EmergencyBroadcast
            | AppScreen::DuressPin
            | AppScreen::Backup
            | AppScreen::DeviceManagement
            | AppScreen::DeviceReplacement
            | AppScreen::DeviceLinking
            | AppScreen::DeliveryStatus
            | AppScreen::ActivityLog
            | AppScreen::Recovery => Some(4),
            _ => None,
        }
    }

    /// Active nav-bar tab (0-4) for the current form dialog, derived from
    /// the engine's `AppScreen::FormDialog { dialog_type }` — the single
    /// source of truth now that the per-dialog `Screen` variants are gone.
    /// AddField/EditField/EditName -> My Card (0); CreateGroup/RenameGroup ->
    /// Groups (3); EditRelayUrl -> More (4).
    pub(crate) fn form_dialog_nav_index(&self) -> usize {
        match self.app_engine.current_app_screen() {
            AppScreen::FormDialog { dialog_type } => match dialog_type {
                FormDialogType::AddField { .. }
                | FormDialogType::EditField { .. }
                | FormDialogType::EditName { .. } => 0,
                FormDialogType::CreateGroup | FormDialogType::RenameGroup { .. } => 3,
                FormDialogType::EditRelayUrl { .. } => 4,
                _ => 0,
            },
            _ => 0,
        }
    }

    /// The live form-dialog kind, if the engine is currently on a form
    /// dialog. Captured before dispatching an action so success feedback
    /// survives the engine's back-navigation (the engine has already left
    /// the dialog by the time `handle_action_result` runs).
    pub(crate) fn form_dialog_type(&self) -> Option<FormDialogType> {
        match self.app_engine.current_app_screen() {
            AppScreen::FormDialog { dialog_type } => Some(dialog_type.clone()),
            _ => None,
        }
    }
}

// INLINE_TEST_REQUIRED: exercises go_back / goto / current_app_screen and the
// overlay + onboarding derivation, which need the private App internals
// (unreachable from an external integration-test crate).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use vauchi_app::ui::AppEngine;
    use vauchi_core::{SymmetricKey, Vauchi, VauchiConfig};

    fn test_app() -> App {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_path_buf();
        let config = VauchiConfig::with_storage_path(path.join("vauchi.db"))
            .with_storage_key(SymmetricKey::generate());
        let mut app_engine = AppEngine::new(Vauchi::new(config).expect("vauchi"));
        app_engine
            .vauchi_mut()
            .create_identity("Test User")
            .expect("create identity");
        let _keep = dir.keep();
        App::new(app_engine, "wss://relay.vauchi.app".to_string(), path)
    }

    /// An open overlay is dismissed by both back-nav and forward navigation,
    /// leaving the engine's underlying screen intact.
    // @internal
    #[test]
    fn open_overlay_is_dismissed_by_back_and_navigation() {
        let mut app = test_app();
        app.goto(AppScreen::Settings);
        app.overlay = Some(Overlay::ActionMenu);
        assert_eq!(app.current_app_screen(), AppScreen::Settings);

        app.go_back();
        assert_eq!(app.overlay, None);
        assert_eq!(app.current_app_screen(), AppScreen::Settings);

        app.overlay = Some(Overlay::ActionMenu);
        app.goto(AppScreen::Help);
        assert_eq!(app.overlay, None);
        assert_eq!(app.current_app_screen(), AppScreen::Help);
    }

    /// The Backup-restore detour during onboarding navigates to a real
    /// screen while the onboarding engine stays alive; the app must report
    /// Backup, not Onboarding.
    // @internal
    #[test]
    fn backup_detour_during_onboarding_is_not_reported_as_onboarding() {
        let mut app = test_app();
        app.onboarding_engine = Some(vauchi_app::ui::OnboardingEngine::new());
        app.app_engine.navigate_to(AppScreen::Onboarding);
        assert_eq!(app.current_app_screen(), AppScreen::Onboarding);

        app.goto(AppScreen::Backup);
        assert_eq!(app.current_app_screen(), AppScreen::Backup);
        assert!(app.onboarding_engine.is_some());
    }
}
