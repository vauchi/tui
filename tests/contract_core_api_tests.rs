// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! Contract tests: TUI's expectations of vauchi-core API (PI-04).
//!
//! These tests assert the shape and behavior of vauchi-core types as
//! consumed by the TUI backend. If core changes in a way that breaks
//! these contracts, these tests fail BEFORE the TUI ships.
//!
//! Consumer: vauchi-tui
//! Provider: vauchi-core

use vauchi_core::contact_card::ContactAction;
use vauchi_core::{Contact, ContactCard, ContactField, FieldType, Identity, Storage, SymmetricKey};

// ============================================================
// Storage contracts
// ============================================================

#[test]
fn contract_storage_open_with_symmetric_key() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let key = SymmetricKey::generate();
    let storage = Storage::open(db_path.to_str().unwrap(), key);
    assert!(storage.is_ok());
}

#[test]
fn contract_storage_save_and_load_identity() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let key = SymmetricKey::generate();
    let storage = Storage::open(db_path.to_str().unwrap(), key).unwrap();

    let identity = Identity::create("TuiContractTest");
    let backup_data = b"test-backup-data".to_vec();
    storage
        .save_identity(&backup_data, identity.display_name())
        .unwrap();

    let loaded = storage.load_identity().unwrap();
    assert!(loaded.is_some());
    let (data, name) = loaded.unwrap();
    assert_eq!(data, backup_data);
    assert_eq!(name, "TuiContractTest");
}

#[test]
fn contract_storage_save_and_load_own_card() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let key = SymmetricKey::generate();
    let storage = Storage::open(db_path.to_str().unwrap(), key).unwrap();

    let card = ContactCard::new("TuiTest");
    storage.save_own_card(&card).unwrap();

    let loaded = storage.load_own_card().unwrap();
    assert!(loaded.is_some());
    assert_eq!(loaded.unwrap().display_name(), "TuiTest");
}

#[test]
fn contract_storage_list_contacts_returns_vec() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test.db");
    let key = SymmetricKey::generate();
    let storage = Storage::open(db_path.to_str().unwrap(), key).unwrap();

    let contacts = storage.list_contacts().unwrap();
    assert!(contacts.is_empty());
}

// ============================================================
// Identity contracts
// ============================================================

#[test]
fn contract_identity_create_returns_identity_with_name() {
    let identity = Identity::create("ContractTui");
    assert_eq!(identity.display_name(), "ContractTui");
}

#[test]
fn contract_identity_has_public_id() {
    let identity = Identity::create("ContractTui");
    let pid = identity.public_id();
    assert!(!pid.is_empty());
}

#[test]
fn contract_identity_has_device_id() {
    let identity = Identity::create("ContractTui");
    let did = identity.device_id();
    assert!(!did.is_empty());
}

#[test]
fn contract_identity_has_signing_public_key() {
    let identity = Identity::create("ContractTui");
    let spk = identity.signing_public_key();
    assert!(!spk.is_empty());
}

#[test]
fn contract_identity_set_display_name() {
    let mut identity = Identity::create("OldName");
    identity.set_display_name("NewName");
    assert_eq!(identity.display_name(), "NewName");
}

// ============================================================
// ContactCard contracts
// ============================================================

#[test]
fn contract_contact_card_new_with_display_name() {
    let card = ContactCard::new("TuiCard");
    assert_eq!(card.display_name(), "TuiCard");
    assert!(card.fields().is_empty());
}

#[test]
fn contract_contact_card_add_field() {
    let mut card = ContactCard::new("TuiCard");
    let field = ContactField::new(FieldType::Phone, "Mobile", "+1234567890");
    card.add_field(field).unwrap();
    assert_eq!(card.fields().len(), 1);
    assert_eq!(card.fields()[0].label(), "Mobile");
}

#[test]
fn contract_contact_card_serde_roundtrip() {
    let mut card = ContactCard::new("TuiCard");
    card.add_field(ContactField::new(FieldType::Email, "Work", "a@b.com"))
        .unwrap();

    let json = serde_json::to_string(&card).unwrap();
    let decoded: ContactCard = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.display_name(), "TuiCard");
    assert_eq!(decoded.fields().len(), 1);
}

// ============================================================
// ContactField / FieldType contracts
// ============================================================

#[test]
fn contract_field_type_variants_exist() {
    let types = [
        FieldType::Phone,
        FieldType::Email,
        FieldType::Address,
        FieldType::Website,
        FieldType::Social,
        FieldType::Custom,
    ];
    assert_eq!(types.len(), 6);
}

#[test]
fn contract_contact_field_accessors() {
    let field = ContactField::new(FieldType::Social, "LinkedIn", "linkedin.com/in/test");
    assert_eq!(field.field_type(), FieldType::Social);
    assert_eq!(field.label(), "LinkedIn");
    assert_eq!(field.value(), "linkedin.com/in/test");
}

// ============================================================
// SymmetricKey contracts
// ============================================================

#[test]
fn contract_symmetric_key_generate_returns_key() {
    let key = SymmetricKey::generate();
    assert_eq!(key.as_bytes().len(), 32);
}

#[test]
fn contract_symmetric_key_from_bytes() {
    let bytes = [0xAA; 32];
    let key = SymmetricKey::from_bytes(bytes);
    assert_eq!(key.as_bytes(), &bytes);
}

// ============================================================
// Contact accessors contracts
// ============================================================

#[test]
fn contract_contact_has_expected_accessors() {
    // Contact is typically created through exchange, but we verify the
    // accessor methods exist and return expected types by compiling this.
    fn _assert_contact_shape(c: &Contact) {
        let _: &str = c.id();
        let _: &str = c.display_name();
        let _: &ContactCard = c.card();
        let _: Option<&[u8; 32]> = c.public_key();
        let _: bool = c.is_hidden();
        let _: bool = c.is_blocked();
    }
}

// ============================================================
// Secondary actions contracts (SP-12a)
// ============================================================

#[test]
fn contract_phone_secondary_actions_include_sms() {
    let field = ContactField::new(FieldType::Phone, "Mobile", "+41791234567");
    let actions = field.to_secondary_actions();

    assert!(
        actions.len() >= 3,
        "phone field must have at least 3 secondary actions, got {}",
        actions.len()
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, ContactAction::SendSms(_))),
        "phone secondary actions must include SendSms"
    );
}

#[test]
fn contract_address_secondary_actions_include_directions() {
    let field = ContactField::new(FieldType::Address, "Home", "Bahnhofstrasse 1, Zurich");
    let actions = field.to_secondary_actions();

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, ContactAction::GetDirections(_))),
        "address secondary actions must include GetDirections"
    );
}

#[test]
fn contract_get_directions_variant_exists() {
    // allow(zero_assertions): Compile-time shape check
    let _action = ContactAction::GetDirections("test".to_string());
}

#[test]
fn contract_to_directions_uri_returns_some_for_address() {
    let field = ContactField::new(FieldType::Address, "Office", "Limmatquai 1, Zurich");
    let uri = field.to_directions_uri();
    assert!(
        uri.is_some(),
        "to_directions_uri must return Some for address"
    );
    assert!(
        uri.unwrap().contains("directions"),
        "directions URI must contain 'directions'"
    );
}
