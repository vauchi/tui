// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Maps AppEngine `ActionResult` variants to TUI state changes.

use vauchi_app::ui::{
    ActionResult, AppScreen, Component, DeviceLinkRole, FormDialogType, LockScreenEngine,
    WorkflowEngine,
};

use crate::app::App;

/// Applies an `ActionResult` from AppEngine to TUI state.
///
/// Use this where the action cannot originate from a form dialog (and from
/// tests). Form-dialog dispatch sites call [`handle_action_result_with`]
/// with the dialog kind captured before dispatch.
pub fn handle_action_result(app: &mut App, result: ActionResult) {
    handle_action_result_with(app, result, None);
}

/// As [`handle_action_result`], but `from_form_dialog` carries the dialog
/// kind captured *before* dispatch. Needed because the engine has already
/// navigated back to the parent by the time this runs, so its `AppScreen`
/// no longer carries the kind that drives success feedback.
pub(crate) fn handle_action_result_with(
    app: &mut App,
    result: ActionResult,
    from_form_dialog: Option<FormDialogType>,
) {
    match result {
        ActionResult::UpdateScreen(_) => {
            // Screen model is re-fetched on next draw via app_engine.current_screen()

            // On form screens, auto-advance focus to the first TextInput when the
            // currently focused component is not a TextInput (e.g., after selecting
            // a type in the ToggleList, the engine adds TextInput fields below it).
            if matches!(
                from_form_dialog,
                Some(
                    FormDialogType::AddField { .. }
                        | FormDialogType::EditField { .. }
                        | FormDialogType::EditName { .. }
                        | FormDialogType::EditRelayUrl { .. }
                )
            ) {
                let screen_model = app.app_engine.current_screen();
                let focused = screen_model
                    .components
                    .get(app.render_state.focused_component);
                let focused_is_text_input = matches!(focused, Some(Component::TextInput { .. }));
                if !focused_is_text_input
                    && let Some(idx) = screen_model
                        .components
                        .iter()
                        .position(|c| matches!(c, Component::TextInput { .. }))
                {
                    app.render_state.focused_component = idx;
                }
            }
        }
        ActionResult::NavigateTo(_) => {
            // AppEngine already updated its current screen; the TUI renders
            // from it. Sync only the TUI-local mirror state the engine does
            // not own (selected contact, lock engine).
            app.render_state = Default::default();
            let app_screen = app.app_engine.current_app_screen();
            let contact_id = match app_screen {
                AppScreen::ContactDetail { contact_id }
                | AppScreen::ContactEdit { contact_id }
                | AppScreen::ContactVisibility { contact_id }
                | AppScreen::VerifyFingerprint { contact_id } => Some(contact_id.clone()),
                _ => None,
            };
            let entering_lock = matches!(app_screen, AppScreen::Lock);
            if let Some(contact_id) = contact_id {
                app.selected_contact_id = Some(contact_id);
            }
            // NavigateTo bypasses `goto()`, so lazily create the lock engine
            // here (mirrors `App::ensure_screen_engine` for the goto() path).
            if entering_lock && app.lock_engine.is_none() {
                app.lock_engine = Some(LockScreenEngine::new(
                    vauchi_app::ui::DEFAULT_LOCK_MAX_ATTEMPTS,
                ));
            }
            // Show success feedback when completing a form dialog. The kind
            // is captured pre-dispatch (`from_form_dialog`) because the engine
            // has already navigated back to the parent by the time this runs.
            // TODO(HUMBLE): W — Maps each FormDialogType to success message (see _private/docs/problems/2026-07-06-desktop-tui-web-domain-shell-violations)
            match from_form_dialog {
                Some(FormDialogType::AddField { .. }) => app.set_status("Entry added"),
                Some(FormDialogType::EditField { .. }) => app.set_status("Entry updated"),
                Some(FormDialogType::EditName { .. }) => app.set_status("Name updated"),
                Some(FormDialogType::EditRelayUrl { .. }) => app.set_status("Relay URL updated"),
                Some(FormDialogType::CreateGroup) => app.set_status("Group created"),
                Some(FormDialogType::RenameGroup { .. }) => app.set_status("Group renamed"),
                _ => {}
            }
        }
        ActionResult::OpenContact { contact_id } => {
            app.selected_contact_id = Some(contact_id.clone());
            app.app_engine
                .navigate_to(AppScreen::ContactDetail { contact_id });
        }
        ActionResult::EditContact { contact_id } => {
            app.selected_contact_id = Some(contact_id.clone());
            app.app_engine
                .navigate_to(AppScreen::ContactEdit { contact_id });
            app.render_state = Default::default();
        }
        ActionResult::OpenUrl { url } => {
            // Skip browser open during tests (prevents unwanted tabs)
            if std::env::var("VAUCHI_NO_BROWSER").is_ok() {
                app.set_status(format!("URL: {url}"));
            } else {
                let opener = if cfg!(target_os = "macos") {
                    "open"
                } else if cfg!(target_os = "windows") {
                    "start"
                } else {
                    "xdg-open"
                };
                match std::process::Command::new(opener)
                    .arg(&url)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                {
                    Ok(_) => app.set_status(format!("Opened: {url}")),
                    Err(_) => app.set_status(format!("URL: {url}")),
                }
            }
        }
        ActionResult::ShowAlert { title, message }
        | ActionResult::ShowInfoOverlay {
            title,
            body: message,
        } => {
            app.alert_message = Some((title, message));
        }
        ActionResult::ValidationError {
            component_id,
            message,
        } => {
            app.render_state.set_validation_error(component_id, message);
        }
        ActionResult::Complete => {
            // AppEngine handles completion routing internally
        }
        // TODO(HUMBLE): D — Decides backup blobs copied to clipboard (see _private/docs/problems/2026-07-06-desktop-tui-web-domain-shell-violations)
        ActionResult::BackupExportComplete { data } => {
            // The engine produced the backup blob; the TUI surfaces it by
            // copying to the clipboard (it is too long to display inline).
            match crate::helpers::copy_to_clipboard(&data) {
                Ok(_) => app.set_status("Backup created — copied to clipboard"),
                Err(_) => app.set_status("Backup created (clipboard unavailable)"),
            }
        }
        // TODO(HUMBLE): D — Decides GDPR export filename and writes to disk (see _private/docs/problems/2026-07-06-desktop-tui-web-domain-shell-violations)
        ActionResult::GdprExportComplete { json } => {
            // The engine produced the GDPR export JSON; the TUI writes it to
            // a file under the data dir (mirrors the retired bespoke handler).
            let path = app.data_dir.join("gdpr_export.json");
            match std::fs::write(&path, &json) {
                Ok(()) => app.set_status(format!("Data exported to {}", path.display())),
                Err(e) => app.set_status(format!("Export failed: {e}")),
            }
        }
        // TODO(HUMBLE): D — StartDeviceLink -> navigate_to(AppScreen::DeviceLinking) (see _private/docs/problems/2026-07-06-desktop-tui-web-domain-shell-violations)
        ActionResult::StartDeviceLink { role } => {
            match role {
                // The Devices (DeviceManagement) path is intercepted in core and
                // arrives as NavigateTo(DeviceLinking). StartDeviceLink only
                // reaches the TUI raw from Onboarding / DeviceReplacement, which
                // have no separate native link flow here — drive the engine to
                // the device-link screen so the QR shows there too.
                DeviceLinkRole::Initiator => {
                    app.app_engine
                        .navigate_to(vauchi_app::ui::AppScreen::DeviceLinking);
                    app.render_state = Default::default();
                }
                // TUI has no camera; tell the user to use the CLI join flow.
                DeviceLinkRole::Responder => {
                    app.set_status("QR scan not supported in terminal mode; use 'vauchi device join <qr_data>'");
                }
                _ => {
                    app.set_status("Unknown device-link role");
                }
            }
        }
        ActionResult::RequestCamera => {
            // TUI can't open camera — show status message
            app.set_status("Camera not supported in terminal mode");
        }
        ActionResult::OpenEntryDetail { .. } => {
            // Handled by AppEngine (intercepted before reaching TUI)
        }
        ActionResult::ShowToast {
            message,
            undo_action_id,
        } => {
            app.set_status_with_undo(message, undo_action_id);
        }
        ActionResult::Commands { commands } => {
            handle_exchange_commands(app, commands);
        }
        ActionResult::WipeComplete => {
            app.onboarding_engine = Some(vauchi_app::ui::OnboardingEngine::new());
            app.render_state = Default::default();
            app.invalidate_engines();
            // AppEngine's Vauchi data was wiped — navigate to Onboarding
            app.app_engine
                .navigate_to(vauchi_app::ui::AppScreen::Onboarding);
        }
        ActionResult::VerifyFingerprint { contact_id } => {
            app.selected_contact_id = Some(contact_id.clone());
            app.app_engine
                .navigate_to(AppScreen::VerifyFingerprint { contact_id });
            app.render_state = Default::default();
        }
        // trust-notes-preview (core!368) — not yet implemented in TUI
        ActionResult::PreviewAs { .. } | ActionResult::ShowContactPicker => {}
        _ => {
            // Unknown ActionResult variant — ignore
        }
    }
}

/// Handles Commands from the ADR-031 command/event protocol.
///
/// TUI supports QR display (re-render) and QR scan (text paste prompt).
/// BLE, NFC, and audio are reported as unavailable back to core so the
/// session can fall back to QR.
fn handle_exchange_commands(app: &mut App, commands: Vec<vauchi_core::Command>) {
    use vauchi_core::{Command, Event};

    for cmd in commands {
        match cmd {
            Command::QrDisplay { .. } => {
                // QR data is embedded in the screen model — re-render picks it up
            }
            Command::QrRequestScan => {
                // TUI: prompt user to paste QR data (handled via text input mode)
                app.set_status("Paste the other person's QR code data and press Enter");
                app.exchange_scan_pending = true;
            }
            // Hardware not available in terminal
            Command::BleStartAdvertising { .. }
            | Command::BleStartScanning { .. }
            | Command::BleConnect { .. }
            | Command::BleWriteCharacteristic { .. }
            | Command::BleReadCharacteristic { .. }
            | Command::BleDisconnect => {
                let _ = app
                    .app_engine
                    .handle_hardware_event(Event::HardwareUnavailable {
                        transport: "BLE".into(),
                    });
            }
            Command::NfcActivate { .. } | Command::NfcDeactivate => {
                let _ = app
                    .app_engine
                    .handle_hardware_event(Event::HardwareUnavailable {
                        transport: "NFC".into(),
                    });
            }
            Command::AudioEmitChallenge { .. }
            | Command::AudioListenForResponse { .. }
            | Command::AudioStop => {
                // Audio proximity not available in terminal — silently skip
                // (not fatal, just means no proximity verification)
            }
            // Capture-at-exchange (ADR-051): no location provider in a
            // terminal. Answer with the "location" transport specifically so
            // core clears the pending capture, rather than the generic
            // "unknown" catch-all below (which core wouldn't match to the
            // location capture and would surface as a stray toast).
            Command::LocationRequest { .. } => {
                let _ = app
                    .app_engine
                    .handle_hardware_event(Event::HardwareUnavailable {
                        transport: "location".into(),
                    });
            }
            _ => {
                // Unknown Command — report as unavailable
                let _ = app
                    .app_engine
                    .handle_hardware_event(Event::HardwareUnavailable {
                        transport: "unknown".into(),
                    });
            }
        }
    }
}
