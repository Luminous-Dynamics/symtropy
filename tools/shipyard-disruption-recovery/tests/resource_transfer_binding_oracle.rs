//! Exhaustive generated oracle for resource-to-transfer schedule consistency.
//!
//! The independent reference predicate checks interval containment and tug identity directly. It
//! does not call the production binding validator.

use shipyard_disruption_recovery::{
    Disruption, RecoveryInput,
    resources::{
        JointRecoveryInput, ResourceReservation, ResourceSchedule, ResourceSpatialBinding,
        evaluate_joint_recovery,
    },
};
use shipyard_space_time::{Footprint, Placement, Rect, Scenario, TimeRange, Transfer};

const CASES: usize = 512;

fn next_u32(state: &mut u32) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state
}

fn pick(state: &mut u32, bound: u32) -> u32 {
    next_u32(state) % bound
}

fn spatial_plan() -> Scenario {
    Scenario {
        yard_bounds: Some(Rect {
            min_x: 0,
            min_y: 0,
            max_x: 10,
            max_y: 10,
        }),
        placements: vec![
            Placement {
                reservation_id: "block-a-before".to_owned(),
                block_id: "block-a".to_owned(),
                footprint: Footprint::AxisAlignedRect(Rect {
                    min_x: 0,
                    min_y: 0,
                    max_x: 2,
                    max_y: 2,
                }),
                occupied: TimeRange {
                    start_tick: 0,
                    end_tick: 4,
                },
            },
            Placement {
                reservation_id: "block-a-after".to_owned(),
                block_id: "block-a".to_owned(),
                footprint: Footprint::AxisAlignedRect(Rect {
                    min_x: 2,
                    min_y: 0,
                    max_x: 4,
                    max_y: 2,
                }),
                occupied: TimeRange {
                    start_tick: 6,
                    end_tick: 10,
                },
            },
        ],
        transfers: vec![Transfer {
            id: "move-a".to_owned(),
            block_id: "block-a".to_owned(),
            corridor: Footprint::AxisAlignedRect(Rect {
                min_x: 8,
                min_y: 8,
                max_x: 9,
                max_y: 9,
            }),
            interval: TimeRange {
                start_tick: 4,
                end_tick: 6,
            },
            vehicle_id: "tug-1".to_owned(),
            predecessor_move_ids: Vec::new(),
        }],
        exclusion_zones: Vec::new(),
    }
}

#[test]
fn generated_joint_acceptance_matches_transfer_binding_reference() {
    let mut state = 0x7a9d_1026_u32;
    let spatial = spatial_plan();
    let mut cases = 0usize;
    let mut expected_accepted = 0usize;
    let mut actual_accepted = 0usize;
    let mut false_feasible = 0usize;
    let mut false_infeasible = 0usize;
    let mut interval_cover_cases = 0usize;

    for _case in 0..CASES {
        let start = u64::from(pick(&mut state, 9));
        let end = start + 1 + u64::from(pick(&mut state, 10));
        let resource_id = if pick(&mut state, 2) == 0 {
            "tug-1"
        } else {
            "tug-2"
        };
        let covers_transfer = start <= 4 && end >= 6;
        let expected = covers_transfer && resource_id == "tug-1";
        if covers_transfer {
            interval_cover_cases += 1;
        }
        if expected {
            expected_accepted += 1;
        }

        let resources = ResourceSchedule {
            precedence_constraints: Vec::new(),
            reservations: vec![ResourceReservation {
                reservation_id: "tug-move-a".to_owned(),
                operation_id: "move-a".to_owned(),
                resource_id: resource_id.to_owned(),
                resource_kind: "tug".to_owned(),
                interval: TimeRange {
                    start_tick: start,
                    end_tick: end,
                },
            }],
        };
        let bindings = vec![ResourceSpatialBinding {
            binding_id: "bind-move-a".to_owned(),
            resource_reservation_id: "tug-move-a".to_owned(),
            transfer_id: "move-a".to_owned(),
        }];
        let report = evaluate_joint_recovery(&JointRecoveryInput {
            spatial: RecoveryInput {
                baseline: spatial.clone(),
                candidate: spatial.clone(),
                disruption: Disruption {
                    observed_at_tick: 0,
                    new_exclusion_zones: Vec::new(),
                    vehicle_outages: Vec::new(),
                    placement_occupancy_extensions: Vec::new(),
                    explicitly_affected_reservation_ids: Vec::new(),
                },
            },
            baseline_resources: resources.clone(),
            candidate_resources: resources,
            resource_outages: Vec::new(),
            baseline_bindings: bindings.clone(),
            candidate_bindings: bindings,
        });
        if report.accepted {
            actual_accepted += 1;
        }
        if report.accepted && !expected {
            false_feasible += 1;
        }
        if !report.accepted && expected {
            false_infeasible += 1;
        }
        assert_eq!(
            report.accepted, expected,
            "binding oracle mismatch for start={start}, end={end}, resource={resource_id}: {:?}",
            report.resources.violations
        );
        cases += 1;
    }

    assert_eq!(cases, CASES);
    assert_eq!(expected_accepted, 96);
    assert_eq!(actual_accepted, expected_accepted);
    assert_eq!(interval_cover_cases, 197);
    assert_eq!(false_feasible, 0);
    assert_eq!(false_infeasible, 0);
}
