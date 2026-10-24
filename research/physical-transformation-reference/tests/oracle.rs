// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration-level acceptance oracle with frozen, explicit expected values.

use physical_transformation_reference::mechanics::{
    axial_bar, cantilever_tip_load, simply_supported_uniform_load,
};
use physical_transformation_reference::{
    Component, Connection, Event, Operation, World, run_reference_fixture,
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

    let expected = vec![vec!["A".to_owned(), "B".to_owned(), "C".to_owned()]];
    assert_eq!(left_first.connected_components(), expected);
    assert_eq!(right_first.connected_components(), expected);
}

#[test]
fn analytical_bar_and_cantilever_match_frozen_known_answers() {
    let bar = axial_bar(1_000.0, 2.0, 0.01, 200.0e9).expect("valid bar case");
    assert!((bar.axial_stress_pa - 100_000.0).abs() <= 1.0e-8);
    assert!((bar.axial_strain - 5.0e-7).abs() <= 1.0e-18);
    assert!((bar.extension_m - 1.0e-6).abs() <= 1.0e-15);

    let cantilever =
        cantilever_tip_load(100.0, 2.0, 200.0e9, 8.0e-6, 0.1).expect("valid cantilever case");
    assert!((cantilever.tip_deflection_m - (1.0 / 6_000.0)).abs() <= 1.0e-15);
    assert!((cantilever.maximum_bending_stress_pa - 2_500_000.0).abs() <= 1.0e-6);
}

#[test]
fn simply_supported_beam_is_checked_against_independent_closed_form_values() {
    let beam = simply_supported_uniform_load(1_000.0, 4.0, 200.0e9, 8.0e-6, 0.1)
        .expect("valid simply supported beam");
    assert!((beam.midspan_deflection_m - (1.0 / 480.0)).abs() <= 1.0e-15);
    assert!((beam.maximum_bending_stress_pa - 25_000_000.0).abs() <= 1.0e-6);
}
