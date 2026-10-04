// SPDX-FileCopyrightText: 2026 Mattia Egloff <mattia.egloff@pm.me>
//
// SPDX-License-Identifier: GPL-3.0-or-later

//! The `--seed` fixture's shape (vauchi/private#448).

use std::collections::BTreeSet;

use vauchi_core::Vauchi;
use vauchi_tui::demo_seed::seed_demo_data;

fn seeded() -> Vauchi {
    let mut vauchi = Vauchi::in_memory().expect("in-memory core");
    seed_demo_data(&mut vauchi);
    vauchi
}

// @internal
#[test]
fn the_fixture_holds_two_hundred_distinctly_named_contacts() {
    let vauchi = seeded();
    let contacts = vauchi.list_contacts().unwrap();

    assert_eq!(contacts.len(), 200);
    let names: BTreeSet<&str> = contacts.iter().map(|c| c.display_name()).collect();
    assert_eq!(names.len(), 200);
    for name in ["Alice", "Finn", "Alice 2", "Bob 2", "Alice 4"] {
        assert!(names.contains(name), "{name} missing from {names:?}");
    }
    assert!(!names.contains("Alice 1"));
    assert_eq!(vauchi.own_card().unwrap().unwrap().fields().len(), 3);
}

// @internal
#[test]
fn contacts_carry_one_to_six_fields_in_turn() {
    let vauchi = seeded();
    let contacts = vauchi.list_contacts().unwrap();
    let fields = |name: &str| {
        contacts
            .iter()
            .find(|c| c.display_name() == name)
            .unwrap_or_else(|| panic!("{name}"))
            .card()
            .fields()
            .len()
    };

    assert_eq!(
        (
            fields("Alice"),
            fields("Bob"),
            fields("Frank"),
            fields("Grace")
        ),
        (1, 2, 6, 1)
    );
}

// @internal
#[test]
fn contacts_are_dealt_round_robin_into_three_groups() {
    let vauchi = seeded();

    let mut sizes: Vec<(String, usize)> = vauchi
        .list_groups()
        .unwrap()
        .iter()
        .map(|group| {
            (
                group.name().to_string(),
                vauchi.get_group_members(group.id()).unwrap().len(),
            )
        })
        .collect();
    sizes.sort();

    assert_eq!(
        sizes,
        [
            ("Family".to_string(), 67),
            ("Friends".to_string(), 67),
            ("Work".to_string(), 66)
        ]
    );
}

// @internal
#[test]
fn seeding_an_existing_identity_adds_nothing() {
    let mut vauchi = seeded();

    seed_demo_data(&mut vauchi);

    assert_eq!(vauchi.list_contacts().unwrap().len(), 200);
}
