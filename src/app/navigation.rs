// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Screen navigation — `goto()`, `go_back()`, and nav-index sync.

use vauchi_app::ui::{AppScreen, FormDialogType, LockScreenEngine, OnboardingEngine};

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
    pub fn current_app_screen(&self) -> AppScreen {
        if matches!(
            self.screen,
            Screen::SetupWelcome
                | Screen::SetupCreateIdentity
                | Screen::SetupAddFields
                | Screen::SetupSecurity
                | Screen::SetupReady
        ) {
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

    /// Navigate to a screen.
    ///
    /// Thin wrapper: updates `self.screen`, navigates AppEngine when the
    /// target maps to an `AppScreen`, ensures any non-AppEngine engine
    /// (Onboarding, Lock) exists, and resets render state.
    pub fn goto(&mut self, screen: Screen) {
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
            Screen::Emergency => Some(AppScreen::EmergencyShred),
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
            Screen::GroupDetail => {
                self.groups_state
                    .selected_group_id
                    .as_ref()
                    .map(|id| AppScreen::GroupDetail {
                        group_id: id.clone(),
                    })
            }
            Screen::ContactVisibility => {
                self.selected_contact_id
                    .as_ref()
                    .map(|id| AppScreen::ContactVisibility {
                        contact_id: id.clone(),
                    })
            }
            Screen::Privacy => Some(AppScreen::Privacy),
            Screen::Support => Some(AppScreen::Support),
            Screen::EditName => Some(AppScreen::FormDialog {
                dialog_type: FormDialogType::EditName {
                    current_name: self.edit_name_state.new_name.clone(),
                },
            }),
            Screen::EditField => Some(AppScreen::FormDialog {
                dialog_type: FormDialogType::EditField {
                    field_id: self.edit_field_state.field_id.clone(),
                    field_label: self.edit_field_state.field_label.clone(),
                    current_value: self.edit_field_state.new_value.clone(),
                    current_note: None,
                },
            }),
            Screen::EditRelayUrl => Some(AppScreen::FormDialog {
                dialog_type: FormDialogType::EditRelayUrl {
                    current_url: self.edit_relay_url_state.new_url.clone(),
                },
            }),
            Screen::AddField => {
                let groups = self.app_engine.available_groups().into_iter().collect();
                Some(AppScreen::FormDialog {
                    dialog_type: FormDialogType::AddField {
                        available_groups: groups,
                    },
                })
            }
            Screen::DeviceReplacement => Some(AppScreen::DeviceReplacement),
            Screen::ContactDuplicates => Some(AppScreen::ContactDuplicates),
            Screen::ContactMerge => {
                let s = &self.merge_state;
                Some(AppScreen::ContactMerge {
                    primary_name: s.primary_name.clone(),
                    primary_fields: s.primary_fields.clone(),
                    secondary_name: s.secondary_name.clone(),
                    secondary_fields: s.secondary_fields.clone(),
                })
            }
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
    pub fn go_back(&mut self) {
        match self.screen {
            // Can't go back from onboarding until identity is configured
            Screen::SetupWelcome => {
                // Stay on setup screen
            }
            Screen::SetupCreateIdentity => {
                self.screen = Screen::SetupWelcome;
            }
            Screen::SetupAddFields => {
                self.screen = Screen::SetupCreateIdentity;
            }
            Screen::SetupSecurity => {
                self.screen = Screen::SetupAddFields;
            }
            Screen::SetupReady => {
                self.screen = Screen::SetupSecurity;
            }
            Screen::ContactDuplicates => {
                self.screen = Screen::Contacts;
                self.duplicates_state = DuplicatesState::default();
            }
            Screen::ContactMerge => {
                self.screen = Screen::ContactDuplicates;
                self.merge_state = MergeState::default();
            }
            Screen::ContactLimit => {
                self.screen = Screen::Contacts;
                self.contact_limit_state = ContactLimitState::default();
            }
            // Can't escape the lock screen — must enter PIN
            Screen::Lock => {
                // Stay on lock screen
            }
            Screen::Contacts | Screen::Exchange | Screen::More => {
                self.screen = Screen::MyInfo;
            }
            Screen::Settings
            | Screen::Help
            | Screen::Recovery
            | Screen::Sync
            | Screen::Activity
            | Screen::Delivery => {
                self.screen = Screen::More;
            }
            Screen::Devices => {
                self.device_link_result = None;
                self.screen = Screen::More;
            }
            Screen::DeviceReplacement => {
                self.screen = Screen::More;
            }
            Screen::Privacy => {
                self.screen = Screen::Settings;
                self.privacy_state = PrivacyState::default();
            }
            Screen::Support => self.screen = Screen::Settings,
            Screen::Emergency => {
                self.screen = Screen::Settings;
                self.emergency_state = EmergencyState::default();
            }
            Screen::Duress => {
                self.screen = Screen::Settings;
                self.duress_state = DuressState::default();
            }
            // From Backup, go back to onboarding/setup if no identity, otherwise More
            Screen::Backup => {
                if self.app_engine.vauchi().has_identity() {
                    self.screen = Screen::More;
                } else {
                    self.screen = Screen::SetupWelcome;
                }
            }
            Screen::ContactDetail => {
                self.selected_contact_id = None;
                self.screen = Screen::Contacts;
            }
            Screen::ContactEdit => {
                // Back from edit goes to detail
                self.screen = Screen::ContactDetail;
                self.render_state = Default::default();
            }
            Screen::ContactVisibility => {
                self.screen = Screen::ContactDetail;
                self.visibility_state = VisibilityState::default();
            }
            Screen::VerifyFingerprint => {
                self.screen = Screen::ContactDetail;
            }
            Screen::AddField => {
                // Return to onboarding wizard if we came from there
                self.screen = if self.onboarding_state.identity_created {
                    Screen::SetupAddFields
                } else {
                    Screen::MyInfo
                };
                self.add_field_state = AddFieldState::default();
            }
            Screen::EditField => {
                self.screen = Screen::MyInfo;
                self.edit_field_state = EditFieldState::default();
            }
            Screen::EditName => {
                self.screen = Screen::Settings;
                self.edit_name_state = EditNameState::default();
            }
            Screen::EditRelayUrl => {
                self.screen = Screen::Settings;
                self.edit_relay_url_state = EditRelayUrlState::default();
            }
            Screen::Groups => {
                self.screen = Screen::MyInfo;
                self.groups_state = GroupsState::default();
            }
            Screen::GroupDetail => {
                self.screen = Screen::Groups;
                self.groups_state.show_group_detail = false;
                self.groups_state.selected_contact_in_group = 0;
            }
            Screen::ActionMenu => {
                self.screen = Screen::ContactDetail;
                self.action_menu_state = ActionMenuState::default();
            }
            _ => {}
        }
        self.input_mode = InputMode::Normal;
        self.sync_nav_index();
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
            Screen::MyInfo
            | Screen::MyInfoEntryDetail
            | Screen::AddField
            | Screen::EditField
            | Screen::EditName => Some(0),
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
            | Screen::EditRelayUrl
            | Screen::Backup
            | Screen::Devices
            | Screen::DeviceReplacement
            | Screen::Delivery
            | Screen::Sync
            | Screen::Activity
            | Screen::Recovery => Some(4),
            _ => None,
        }
    }
}
