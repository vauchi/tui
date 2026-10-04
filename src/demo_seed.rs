// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `--seed` load-testing fixture: one identity, three groups and 200
//! contacts with one to six fields each (vauchi/private#448).

use vauchi_core::api::Vauchi;

/// Seeds the Vauchi instance with demo data for local testing.
///
/// Creates an identity, adds fields, creates groups, and adds fake contacts.
/// Only runs when VAUCHI_SEED=1 and no identity exists yet.
///
/// Runs before the `AppEngine` render loop starts, so the ADR-066 shell
/// boundary (which governs the interactive Command/Event surface) does not
/// apply — this is one-shot dev fixture setup, the same shape as `cli`'s
/// non-interactive subcommands calling `Vauchi` methods directly
/// (`cli/src/commands/init.rs`). No core-owned bulk-seed entry exists to
/// delegate to: `Vauchi::initialize_demo_contact` is a distinct, unrelated
/// onboarding feature (a single placeholder contact), not this tool's
/// 200-contact load-testing fixture
/// (2026-07-06-desktop-tui-web-domain-shell-violations U23).
pub fn seed_demo_data(vauchi: &mut Vauchi) {
    use vauchi_core::contact::Contact;
    use vauchi_core::contact_card::{ContactCard, ContactField, FieldType};
    use vauchi_core::crypto::SymmetricKey;

    if vauchi.create_identity("Demo User").is_err() {
        return;
    }

    // Add own fields
    let own_fields = [
        (FieldType::Phone, "Mobile", "+41 79 123 45 67"),
        (FieldType::Email, "Work", "demo@vauchi.app"),
        (FieldType::Website, "Website", "https://vauchi.app"),
    ];
    let now = vauchi.clock().unix_seconds();
    for (ft, label, value) in own_fields {
        let _ = vauchi.add_own_field(ContactField::new(ft, label, value, now));
    }

    // Create groups
    let family = vauchi.create_group("Family").ok();
    let friends = vauchi.create_group("Friends").ok();
    let work = vauchi.create_group("Work").ok();

    // Base names — combined with numeric suffixes to generate up to 200 contacts
    let base_names = [
        "Alice", "Bob", "Charlie", "Diana", "Eve", "Frank", "Grace", "Hank", "Ivy", "Jack",
        "Karen", "Leo", "Mia", "Noah", "Olivia", "Paul", "Quinn", "Rosa", "Sam", "Tina", "Uma",
        "Victor", "Wendy", "Xavier", "Yuki", "Zara", "Amber", "Brian", "Clara", "David", "Elena",
        "Felix", "Gina", "Hugo", "Iris", "James", "Kira", "Liam", "Maya", "Nora", "Oscar", "Petra",
        "Rafael", "Sofia", "Theo", "Ursula", "Vera", "Walter", "Xena", "Yara", "Zoe", "Aria",
        "Blake", "Cleo", "Dario", "Elsa", "Finn",
    ];

    // Generate 200 names: first 57 as-is, then with numeric suffixes
    let names: Vec<String> = (0..200)
        .map(|i| {
            let base = base_names[i % base_names.len()];
            if i < base_names.len() {
                base.to_string()
            } else {
                format!("{} {}", base, i / base_names.len() + 1)
            }
        })
        .collect();

    // Field templates — each contact gets (i % 6 + 1) fields
    let field_templates: &[(FieldType, &str, &str)] = &[
        (FieldType::Phone, "Mobile", "+41 79 {n} 00"),
        (FieldType::Email, "Personal", "{}@example.com"),
        (FieldType::Phone, "Work", "+41 44 {n} 00"),
        (FieldType::Website, "Website", "https://{}.dev"),
        (FieldType::Email, "Work", "{}@corp.ch"),
        (FieldType::Phone, "Home", "+41 31 {n} 00"),
    ];

    let groups = [&family, &friends, &work];

    for (i, name) in names.iter().enumerate() {
        let mut card = ContactCard::new(name);
        let num_fields = (i % 6) + 1;
        for item in field_templates.iter().take(num_fields) {
            let (ref ft, label, template) = *item;
            // A phone number cannot carry the name: Core rejects letters in
            // it and the contact silently lost those fields.
            let value = template
                .replace("{}", &name.to_lowercase())
                .replace("{n}", &format!("{i:03}"));
            let _ = card.add_field(ContactField::new(ft.clone(), label, &value, now));
        }

        let shared_key = SymmetricKey::generate();
        let extra_key = SymmetricKey::generate();
        let pubkey: [u8; 32] = *extra_key.as_bytes();
        let contact = Contact::from_exchange(
            pubkey,
            card,
            shared_key,
            vauchi_core::clock::SystemClock::shared().unix_seconds(),
        );
        let contact_id = contact.id().to_string();
        if vauchi.add_contact(contact).is_err() {
            continue;
        }

        // Assign to groups round-robin
        if let Some(g) = groups[i % 3] {
            let _ = vauchi.add_contact_to_group(g.id(), &contact_id);
        }
    }
}
