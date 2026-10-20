// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration-level acceptance oracle with frozen, explicit expected values.

use physical_transformation_reference::{
    run_reference_fixture, Component, Connection, Event, Operation, World,
};

#[test]
fn frozen_reference_outcome_matches_independent_expected_values() {
    let observed = run_reference_fixture().expect("reference fixture should execute");
    // These expectations are pinned here, separately from the evaluator's
    // internal self-checks, so the receipt and integration oracle must agree.
    assert_eq!(observed.initial_connected_components, 1);
    assert_eq!(observed.after_cut_connected_components, 2);
    assert_eq!(observed.after_reconnect_connected_components, 1);
    assert_eq!(observed.component_count, 3);
    assert_eq!(observed.connection_count, 2);
    assert!(observed.accepted_balanced_mass_ledger);
    assert!(observed.rejected_imbalanced_mass_ledger);
    assert!(observed.rejected_mutation_was_atomic);
}

#[test]
fn reversing_edge_insertion_order_preserves_final_topology() {
    fn base_world() -> World {
        let mut world = World::default();
        for id in ["A", "B", "C"] {
            world
                .apply(Event {
                    id: format!("install-{id}"),
                    operation: Operation::InstallComponent(Component {
                        id: id.to_owned(),
                        material: "steel".to_owned(),
                        mass_mg: 100,
                    }),
                })
                .unwrap();
        }
        world
    }

    let mut left_first = base_world();
    left_first
        .apply(Event {
            id: "edge-AB".to_owned(),
            operation: Operation::CreateConnection(Connection {
                id: "AB".to_owned(),
                a: "A".to_owned(),
                b: "B".to_owned(),
            }),
        })
        .unwrap();
    left_first
        .apply(Event {
            id: "edge-BC".to_owned(),
            operation: Operation::CreateConnection(Connection {
                id: "BC".to_owned(),
                a: "B".to_owned(),
                b: "C".to_owned(),
            }),
        })
        .unwrap();

    let mut right_first = base_world();
    right_first
        .apply(Event {
            id: "edge-BC".to_owned(),
            operation: Operation::CreateConnection(Connection {
                id: "BC".to_owned(),
                a: "B".to_owned(),
                b: "C".to_owned(),
            }),
        })
        .unwrap();
    right_first
        .apply(Event {
            id: "edge-AB".to_owned(),
            operation: Operation::CreateConnection(Connection {
                id: "AB".to_owned(),
                a: "A".to_owned(),
                b: "B".to_owned(),
            }),
        })
        .unwrap();

    assert_eq!(left_first.connected_components(), vec![vec!["A", "B", "C"]]);
    assert_eq!(right_first.connected_components(), vec![vec!["A", "B", "C"]]);
}
