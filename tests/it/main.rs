// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Consolidated integration test binary for vauchi-tui.

#[allow(dead_code)]
#[macro_use]
mod common;
mod action_result_tests;
mod contract_core_api_tests;
mod duress_humble_tests;
mod handle_key_tests;
mod helpers_test;
mod input_handler_tests;
mod qr_rendering_test;
mod smoke_appengine_tests;
mod snapshot_screens;
mod snapshot_workflows;
mod ui_interaction_tests;
