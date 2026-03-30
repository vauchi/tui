// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Tests for QR code rendering in terminal.

use qrcode::QrCode;

#[test]
fn qr_code_renders_non_empty_unicode_output() {
    let data = "VAUCHI:TEST:ABCDEF1234567890";
    let code = QrCode::new(data).expect("QR generation should not fail");
    let image = code
        .render::<qrcode::render::unicode::Dense1x2>()
        .dark_color(qrcode::render::unicode::Dense1x2::Light)
        .light_color(qrcode::render::unicode::Dense1x2::Dark)
        .quiet_zone(false)
        .build();

    let lines: Vec<&str> = image.lines().collect();
    assert!(
        lines.len() >= 5,
        "QR should have at least 5 lines, got {}",
        lines.len()
    );

    for line in &lines {
        assert!(!line.is_empty(), "QR line should not be empty");
    }

    assert!(
        !image.contains("┌──────────────┐"),
        "Should not contain placeholder box"
    );
}

#[test]
fn qr_code_handles_long_exchange_data() {
    let data = "VAUCHI:EX:".to_string() + &"A".repeat(160);
    let code = QrCode::new(&data).expect("QR generation should not fail for exchange data");
    let image = code
        .render::<qrcode::render::unicode::Dense1x2>()
        .dark_color(qrcode::render::unicode::Dense1x2::Light)
        .light_color(qrcode::render::unicode::Dense1x2::Dark)
        .quiet_zone(false)
        .build();

    assert!(
        image.lines().count() >= 10,
        "Dense QR should have many lines"
    );
}

#[test]
fn qr_code_fails_gracefully_on_oversized_data() {
    let data = "X".repeat(5000);
    let result = QrCode::new(&data);
    assert!(result.is_err(), "Oversized data should fail QR generation");
}
