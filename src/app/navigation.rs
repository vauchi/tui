// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Screen navigation — `goto()`, `go_back()`, and nav-index sync.

use vauchi_app::ui::{
    AppScreen, FormDialogType, LockScreenEngine, OnboardingEngine, WorkflowEngine,
};

use super::App;
use super::state::*;
use crate::ui::widgets::screen_renderer::ScreenRenderState;

impl App {
    /// The live `AppScreen` from the engine perspective.
    ///
    /// For engine-driven screens this returns the engine's truth
    /// (`app_engine.current_app_screen()` cloned). The Setup wizard and
    /// Lock screen run on dedicated engines that don't navigate
    /// AppEngine, so their `AppScreen` is synthesized from `self.screen`.
    /// As a fallback (writers that bypass `goto` — notably tests and
    /// `go_back()` — leave the engine out of sync), the per-screen
    /// state is also synthesized from `self.screen`. Once `go_back`
    /// migrates to engine-driven back-nav (Phase 1 T1.4), this fallback
    /// can be deleted.
    /// True while `self.screen` is the onboarding sentinel. During
    /// onboarding `self.screen` stays `SetupWelcome` (the precise step is
    /// derived from the engine); a detour out of onboarding — Backup
    /// restore via 'i' — sets a real screen, clearing this.
    fn screen_is_onboarding_sentinel(&self) -> bool {
        matches!(
            self.screen,
            Screen::SetupWelcome
                | Screen::SetupCreateIdentity
                | Screen::SetupAddFields
                | Screen::SetupSecurity
                | Screen::SetupReady
        )
    }

    pub fn current_app_screen(&self) -> AppScreen {
        if self.screen_is_onboarding_sentinel() {
            return AppScreen::Onboarding;
        }
        if matches!(self.screen, Screen::Lock) {
            return AppScreen::Lock;
        }
        if let Some(app_screen) = self.engine_target_for_screen(self.screen) {
            return app_screen;
        }
        self.app_engine.current_app_screen().clone()
    }

    /// The current TUI `Screen`. Single read accessor for the `screen`
    /// field so external modules never touch the field directly — the
    /// seam Phase 1 flips to an engine-derived value before the field is
    /// deleted (`navigation.rs` remains the only module that names it).
    pub fn active_screen(&self) -> Screen {
        // An open overlay is the active screen from the UI's perspective;
        // the engine's underlying screen stays in `self.screen`.
        if let Some(overlay) = self.overlay {
            return match overlay {
                Overlay::ActionMenu => Screen::ActionMenu,
                Overlay::ContactImport => Screen::ContactImport,
            };
        }
        // During onboarding `self.screen` is the sentinel; the precise step
        // is derived from the engine. A detour out of onboarding falls
        // through to the real screen.
        if self.screen_is_onboarding_sentinel()
            && let Some(ref engine) = self.onboarding_engine
        {
            return Screen::for_onboarding_screen_id(engine.current_screen().screen_id.as_str());
        }
        self.screen
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
        self.clear_contact_search_if_leaving(Screen::FormDialog);
        self.screen = Screen::FormDialog;
        self.input_mode = InputMode::Normal;
        // Navigate the engine first so `sync_nav_index` — which derives the
        // active tab from `AppScreen::FormDialog { dialog_type }` — observes
        // the dialog kind rather than the screen we are leaving.
        self.app_engine
            .navigate_to(AppScreen::FormDialog { dialog_type });
        self.sync_nav_index();
        self.render_state = ScreenRenderState::default();
    }

    /// Navigate to a screen.
    ///
    /// Thin wrapper: updates `self.screen`, navigates AppEngine when the
    /// target maps to an `AppScreen`, ensures any non-AppEngine engine
    /// (Onboarding, Lock) exists, and resets render state.
    pub fn goto(&mut self, screen: Screen) {
        // Navigating to any screen dismisses an open TUI overlay.
        self.overlay = None;
        self.clear_contact_search_if_leaving(screen);
        self.screen = screen;
        self.input_mode = InputMode::Normal;
        self.sync_nav_index();

        if let Some(app_screen) = self.engine_target_for_screen(screen) {
            self.app_engine.navigate_to(app_screen);
            self.render_state = ScreenRenderState::default();
        }
        self.ensure_screen_engine(screen);
    }

    fn clear_contact_search_if_leaving(&mut self, target: Screen) {
        if self.screen == Screen::Contacts && target != Screen::Contacts {
            self.contact_search_mode = false;
            self.contact_search_query.clear();
        }
    }

    /// Lazily creates the dedicated engines that don't live on AppEngine
    /// (Onboarding, Lock). Resets render state when an engine-backed
    /// screen is entered so the renderer starts on a clean slate.
    fn ensure_screen_engine(&mut self, screen: Screen) {
        match screen {
            Screen::SetupWelcome => {
                if self.onboarding_engine.is_none() {
                    self.onboarding_engine = Some(OnboardingEngine::new());
                }
                self.render_state = ScreenRenderState::default();
            }
            Screen::Lock => {
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

    /// Reconcile AppEngine with `self.screen` after writers that bypass
    /// `goto` (notably `go_back()`'s direct `self.screen = Screen::*`
    /// assignments). Once `go_back` migrates to engine-driven back-nav
    /// (Phase 1 T1.4), this can be deleted.
    pub fn ensure_engine_synced(&mut self) {
        if let Some(target) = self.engine_target_for_screen(self.screen)
            && *self.app_engine.current_app_screen() != target
        {
            self.app_engine.navigate_to(target);
        }
    }

    /// Computes the `AppScreen` that AppEngine should be navigated to for
    /// a given TUI `Screen`. Returns `None` for screens that don't map to
    /// an engine-driven AppScreen (Setup wizard steps, Lock, ActionMenu,
    /// ContactImport, MyInfoEntryDetail — engine already has the right
    /// state for the latter).
    fn engine_target_for_screen(&self, screen: Screen) -> Option<AppScreen> {
        match screen {
            Screen::MyInfo => Some(AppScreen::MyInfo),
            Screen::Contacts => Some(AppScreen::Contacts),
            Screen::Exchange => Some(AppScreen::Exchange),
            Screen::Settings => Some(AppScreen::Settings),
            Screen::Help => Some(AppScreen::Help),
            Screen::Backup => Some(AppScreen::Backup),
            Screen::Delivery => Some(AppScreen::DeliveryStatus),
            Screen::Devices => Some(AppScreen::DeviceManagement),
            Screen::Duress => Some(AppScreen::DuressPin),
            Screen::Emergency => Some(AppScreen::EmergencyBroadcast),
            Screen::ContactDetail => {
                self.selected_contact_id
                    .as_ref()
                    .map(|id| AppScreen::ContactDetail {
                        contact_id: id.clone(),
                    })
            }
            Screen::ContactEdit => {
                self.selected_contact_id
                    .as_ref()
                    .map(|id| AppScreen::ContactEdit {
                        contact_id: id.clone(),
                    })
            }
            Screen::Sync => Some(AppScreen::Sync),
            Screen::Activity => Some(AppScreen::ActivityLog),
            Screen::Recovery => Some(AppScreen::Recovery),
            Screen::More => Some(AppScreen::More),
            Screen::Groups => Some(AppScreen::Groups),
            // GroupDetail is engine-driven: the Groups WorkflowEngine
            // emits `NavigateTo(GroupDetail { group_id })` when the user
            // picks a group, and `action_result.rs` syncs `app.screen`.
            Screen::GroupDetail => None,
            Screen::ContactVisibility => {
                self.selected_contact_id
                    .as_ref()
                    .map(|id| AppScreen::ContactVisibility {
                        contact_id: id.clone(),
                    })
            }
            Screen::Privacy => Some(AppScreen::Privacy),
            Screen::Support => Some(AppScreen::Support),
            // `Screen::FormDialog` is entered via `goto_form_dialog`, which
            // navigates the engine to `AppScreen::FormDialog { dialog_type }`
            // directly; it carries its own data and falls through to `None`.
            Screen::DeviceReplacement => Some(AppScreen::DeviceReplacement),
            Screen::DeviceLinking => Some(AppScreen::DeviceLinking),
            Screen::ContactDuplicates => Some(AppScreen::ContactDuplicates),
            // ContactMerge is engine-driven: action_result.rs syncs
            // `app.screen = Screen::ContactMerge` from
            // `AppScreen::ContactMerge { ... }`. No local mirror needed
            // — engine has the primary/secondary contact data.
            Screen::ContactMerge => None,
            Screen::ContactLimit => Some(AppScreen::ContactLimit),
            Screen::VerifyFingerprint => {
                self.selected_contact_id
                    .as_ref()
                    .map(|id| AppScreen::VerifyFingerprint {
                        contact_id: id.clone(),
                    })
            }
            Screen::MyInfoEntryDetail => None,
            _ => None,
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
        match self.screen {
            // TUI-only states (no AppEngine navigation).
            Screen::SetupWelcome | Screen::Lock => {} // stay
            Screen::SetupCreateIdentity => self.goto(Screen::SetupWelcome),
            Screen::SetupAddFields => self.goto(Screen::SetupCreateIdentity),
            Screen::SetupSecurity => self.goto(Screen::SetupAddFields),
            Screen::SetupReady => self.goto(Screen::SetupSecurity),

            // Setup-time `Backup` and `AddField` retain their explicit
            // arms because the bootstrap flow has no engine history to
            // pop back to — the parent is the in-progress wizard step,
            // which AppEngine doesn't track.
            Screen::Backup if !self.app_engine.vauchi().has_identity() => {
                self.goto(Screen::SetupWelcome);
            }
            // Setup-time AddField: the bootstrap wizard (SetupAddFields)
            // opens the AddField form dialog, which has no engine history to
            // pop back to. Guard on the live dialog kind — `identity_created`
            // stays true forever, so post-setup form dialogs must still take
            // the engine-driven branch below.
            Screen::FormDialog
                if self.onboarding_state.identity_created
                    && matches!(
                        self.app_engine.current_app_screen(),
                        AppScreen::FormDialog {
                            dialog_type: FormDialogType::AddField { .. }
                        }
                    ) =>
            {
                self.goto(Screen::SetupAddFields);
            }

            // Engine-driven screens — delegate to AppEngine.
            _ => {
                // Local UI-only state resets that don't survive a parent
                // re-render. These are TUI presentation state, not data.
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
        match self.screen {
            Screen::ContactDetail => self.selected_contact_id = None,
            Screen::ContactEdit => self.render_state = ScreenRenderState::default(),
            Screen::ContactVisibility => self.selected_visibility_field = 0,
            _ => {}
        }
    }

    /// Reverse map `app_engine.current_app_screen()` onto `self.screen`
    /// after engine-driven navigation (`navigate_back`,
    /// engine-emitted `NavigateTo` results). Inverse of
    /// `engine_target_for_screen`. Falls back to MyInfo for screens
    /// AppEngine has but TUI doesn't (or hasn't migrated yet).
    pub(crate) fn sync_screen_from_engine(&mut self) {
        let app_screen = self.app_engine.current_app_screen();
        // `from_app_screen` returns `None` for AppScreen variants the TUI
        // has no top-level screen for (FormDialog overlays pop back through,
        // DeepLinkConsent, future non_exhaustive variants); fall back to
        // MyInfo as the safe landing screen.
        let new_screen = Screen::from_app_screen(app_screen).unwrap_or(Screen::MyInfo);
        let contact_id = Screen::contact_id_of(app_screen);
        if let Some(contact_id) = contact_id {
            self.selected_contact_id = Some(contact_id);
        }
        self.screen = new_screen;
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
        match self.screen {
            // 0: My Card (MyInfo and sub-screens)
            Screen::MyInfo | Screen::MyInfoEntryDetail => Some(0),
            // Form dialogs collapse to one screen; the active tab is derived
            // from the engine's dialog kind (My Card / Groups / More).
            Screen::FormDialog => Some(self.form_dialog_nav_index()),
            // 1: Contacts and sub-screens
            Screen::Contacts
            | Screen::ContactDetail
            | Screen::ContactEdit
            | Screen::ContactVisibility
            | Screen::ContactDuplicates
            | Screen::ContactMerge
            | Screen::ContactLimit
            | Screen::ActionMenu
            | Screen::VerifyFingerprint => Some(1),
            // 2: Exchange
            Screen::Exchange => Some(2),
            // 3: Groups and sub-screens
            Screen::Groups | Screen::GroupDetail => Some(3),
            // 4: More and all infrastructure screens
            Screen::More
            | Screen::Settings
            | Screen::Help
            | Screen::Privacy
            | Screen::Support
            | Screen::Emergency
            | Screen::Duress
            | Screen::Backup
            | Screen::Devices
            | Screen::DeviceReplacement
            | Screen::DeviceLinking
            | Screen::Delivery
            | Screen::Sync
            | Screen::Activity
            | Screen::Recovery => Some(4),
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

impl Screen {
    /// Pure inverse of [`App::engine_target_for_screen`]: the TUI `Screen`
    /// for an engine `AppScreen` discriminant, or `None` for `AppScreen`
    /// variants the TUI has no top-level screen for (overlay form dialogs,
    /// deep-link consent, future `#[non_exhaustive]` variants) — callers
    /// apply their own fallback.
    ///
    /// Single source for the AppScreen->Screen map. It replaced two
    /// hand-maintained copies (`App::sync_screen_from_engine` and
    /// `handlers::action_result`) that had drifted: `MyInfoEntryDetail`
    /// mapped to its own screen in the action-result path but fell back to
    /// `MyInfo` in the back-nav path. Both now agree via this function.
    /// Associated data (`contact_id`, `group_id`, dialog kind) stays on the
    /// engine's `AppScreen`; [`Screen::contact_id_of`] surfaces the one piece
    /// the TUI mirrors locally.
    pub(crate) fn from_app_screen(app_screen: &AppScreen) -> Option<Screen> {
        Some(match app_screen {
            AppScreen::MyInfo => Screen::MyInfo,
            AppScreen::Contacts => Screen::Contacts,
            AppScreen::ContactDetail { .. } => Screen::ContactDetail,
            AppScreen::ContactEdit { .. } => Screen::ContactEdit,
            AppScreen::ContactVisibility { .. } => Screen::ContactVisibility,
            AppScreen::Exchange => Screen::Exchange,
            AppScreen::Settings => Screen::Settings,
            AppScreen::Help => Screen::Help,
            AppScreen::Backup => Screen::Backup,
            AppScreen::Lock => Screen::Lock,
            AppScreen::DeviceLinking => Screen::DeviceLinking,
            AppScreen::DeviceManagement => Screen::Devices,
            AppScreen::DuressPin => Screen::Duress,
            AppScreen::EmergencyBroadcast => Screen::Emergency,
            AppScreen::DeliveryStatus => Screen::Delivery,
            AppScreen::Sync => Screen::Sync,
            AppScreen::Recovery => Screen::Recovery,
            AppScreen::Groups => Screen::Groups,
            AppScreen::GroupDetail { .. } => Screen::GroupDetail,
            AppScreen::More => Screen::More,
            AppScreen::Privacy => Screen::Privacy,
            AppScreen::Support => Screen::Support,
            AppScreen::FormDialog { .. } => Screen::FormDialog,
            AppScreen::ContactDuplicates => Screen::ContactDuplicates,
            AppScreen::ContactMerge { .. } => Screen::ContactMerge,
            AppScreen::ContactLimit => Screen::ContactLimit,
            AppScreen::MyInfoEntryDetail { .. } => Screen::MyInfoEntryDetail,
            AppScreen::VerifyFingerprint { .. } => Screen::VerifyFingerprint,
            AppScreen::ActivityLog => Screen::Activity,
            AppScreen::DeviceReplacement => Screen::DeviceReplacement,
            AppScreen::Onboarding => Screen::SetupWelcome,
            _ => return None,
        })
    }

    /// The contact id the TUI mirrors into `selected_contact_id` for the
    /// `AppScreen` variants that carry one. `None` for every other screen.
    pub(crate) fn contact_id_of(app_screen: &AppScreen) -> Option<String> {
        match app_screen {
            AppScreen::ContactDetail { contact_id }
            | AppScreen::ContactEdit { contact_id }
            | AppScreen::ContactVisibility { contact_id }
            | AppScreen::VerifyFingerprint { contact_id } => Some(contact_id.clone()),
            _ => None,
        }
    }

    /// Map an onboarding engine `screen_id` to its TUI setup `Screen`.
    /// The wizard step is derived from the engine, never stored.
    pub(crate) fn for_onboarding_screen_id(screen_id: &str) -> Screen {
        match screen_id {
            "identity_check" | "welcome" => Screen::SetupWelcome,
            "default_name" => Screen::SetupCreateIdentity,
            "skip_gate" | "groups_setup" | "contact_info" | "preview_card" => {
                Screen::SetupAddFields
            }
            "security_explanation" | "backup_prompt" => Screen::SetupSecurity,
            "ready" => Screen::SetupReady,
            _ => Screen::SetupWelcome,
        }
    }
}

// INLINE_TEST_REQUIRED: round-trip test exercises the private
// App::engine_target_for_screen forward map against pub(crate)
// Screen::from_app_screen — neither is reachable from tests/ (integration
// tests are an external crate and see only `pub`).
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

    /// Every `Screen` that `engine_target_for_screen` maps to an `AppScreen`
    /// must round-trip back to itself through `Screen::from_app_screen`. This
    /// is the seam Phase 0 unified: if a future edit adds a `Screen` variant
    /// (or an `AppScreen` mapping) to one map but not the inverse, the round
    /// trip breaks here instead of drifting silently between the two paths.
    #[test]
    fn engine_mapped_screens_round_trip_through_from_app_screen() {
        let mut app = test_app();
        // Contact-bearing arms of `engine_target_for_screen` read this.
        app.selected_contact_id = Some("test-contact".to_string());

        let engine_mapped = [
            Screen::MyInfo,
            Screen::Contacts,
            Screen::ContactDetail,
            Screen::ContactEdit,
            Screen::ContactVisibility,
            Screen::Exchange,
            Screen::Settings,
            Screen::Help,
            Screen::Backup,
            Screen::Delivery,
            Screen::Devices,
            Screen::Duress,
            Screen::Emergency,
            Screen::Sync,
            Screen::Activity,
            Screen::Recovery,
            Screen::More,
            Screen::Groups,
            Screen::Privacy,
            Screen::Support,
            Screen::DeviceReplacement,
            Screen::DeviceLinking,
            Screen::ContactDuplicates,
            Screen::ContactLimit,
            Screen::VerifyFingerprint,
        ];

        for screen in engine_mapped {
            let app_screen = app
                .engine_target_for_screen(screen)
                .unwrap_or_else(|| panic!("{screen:?} expected to map to an AppScreen"));
            assert_eq!(
                Screen::from_app_screen(&app_screen),
                Some(screen),
                "{screen:?} did not round-trip through from_app_screen",
            );
        }
    }

    #[test]
    fn contact_id_of_extracts_id_for_contact_screens_and_none_otherwise() {
        let detail = AppScreen::ContactDetail {
            contact_id: "alice".to_string(),
        };
        assert_eq!(
            Screen::contact_id_of(&detail),
            Some("alice".to_string()),
            "ContactDetail must surface its contact_id",
        );
        assert_eq!(
            Screen::contact_id_of(&AppScreen::Settings),
            None,
            "Settings carries no contact_id",
        );
    }

    /// `AppScreen` variants the TUI has no top-level `Screen` for must return
    /// `None`; both callers rely on this to fall back (back-nav) or leave
    /// `app.screen` untouched (action-result). `EmergencyShred` is a good
    /// witness: the TUI's Emergency screen maps to `EmergencyBroadcast`, so
    /// `EmergencyShred` has no TUI screen of its own.
    #[test]
    fn from_app_screen_is_none_for_unmapped_screens() {
        assert_eq!(
            Screen::from_app_screen(&AppScreen::EmergencyShred),
            None,
            "EmergencyShred has no top-level TUI screen",
        );
        assert_eq!(
            Screen::from_app_screen(&AppScreen::ChangePassword),
            None,
            "ChangePassword has no top-level TUI screen",
        );
    }

    /// `routes_through_engine` must be true for every engine-driven
    /// screen and false only for the bespoke-handler complement. Locks
    /// the drift the predicate replaced: `ContactLimit` /
    /// `VerifyFingerprint` are engine-driven and were silently dropping
    /// keys before the gate was redefined by its complement.
    #[test]
    fn routes_through_engine_is_false_only_for_bespoke_handler_screens() {
        assert!(Screen::ContactLimit.routes_through_engine());
        assert!(Screen::VerifyFingerprint.routes_through_engine());
        assert!(Screen::MyInfo.routes_through_engine());
        for bespoke in [
            Screen::SetupWelcome,
            Screen::SetupCreateIdentity,
            Screen::SetupAddFields,
            Screen::SetupSecurity,
            Screen::SetupReady,
            Screen::Lock,
            Screen::ActionMenu,
            Screen::ContactImport,
        ] {
            assert!(
                !bespoke.routes_through_engine(),
                "{bespoke:?} must use its bespoke handler, not the engine resolver",
            );
        }
    }

    /// An open overlay reads as the active screen while the engine screen
    /// beneath is untouched, and is dismissed by both back-nav and forward
    /// navigation. Locks brick 3's overlay-aware `active_screen` + the
    /// `go_back`/`goto` overlay handling.
    #[test]
    fn open_overlay_is_active_screen_and_dismissed_by_back_and_navigation() {
        let mut app = test_app();
        app.screen = Screen::ContactDetail;
        app.overlay = Some(Overlay::ActionMenu);
        assert_eq!(app.active_screen(), Screen::ActionMenu);
        assert_eq!(app.screen, Screen::ContactDetail);

        app.go_back();
        assert_eq!(app.overlay, None);
        assert_eq!(app.active_screen(), Screen::ContactDetail);

        app.overlay = Some(Overlay::ActionMenu);
        app.goto(Screen::Settings);
        assert_eq!(app.overlay, None);
        assert_eq!(app.active_screen(), Screen::Settings);
    }

    /// The Backup-restore detour ('i' during onboarding) navigates to a
    /// real screen while the onboarding engine stays alive. The engine's
    /// presence must NOT make the app report onboarding — `self.screen` is
    /// the mode sentinel.
    #[test]
    fn backup_detour_during_onboarding_is_not_reported_as_onboarding() {
        let mut app = test_app();
        app.onboarding_engine = Some(vauchi_app::ui::OnboardingEngine::new());
        app.screen = Screen::SetupWelcome;
        assert_eq!(app.current_app_screen(), AppScreen::Onboarding);

        // 'i' restore detours to Backup with the onboarding engine still alive.
        app.goto(Screen::Backup);
        assert_eq!(app.active_screen(), Screen::Backup);
        assert_eq!(app.current_app_screen(), AppScreen::Backup);
        assert!(
            app.onboarding_engine.is_some(),
            "engine persists across the detour so the user can return",
        );
    }
}
