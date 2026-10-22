//! Exhaustive 512-case dispatch policy oracle for freeze, churn and geometry budgets.
//!
//! The reference decision is computed directly from the known fixture deltas and policy bits. It
//! does not call policy helper functions or infer expected acceptance from the policy report.

use shipyard_disruption_recovery::{
    Disruption, RecoveryInput,
    policy::{
        DispatchPolicyViolationCode, DispatchStabilityPolicy, evaluate_operational_recovery,
    },
};
use shipyard_space_time::{Footprint, Placement, Rect, Scenario, TimeRange, Transfer};

const CASES: usize = 512;

fn rect(min_x: i64, min_y: i64, max_x: i64, max_y: i64) -> Footprint {
    Footprint::AxisAlignedRect(Rect {
        min_x,
        min_y,
        max_x,
        max_y,
    })
}

fn placement(id: &str, x: i64, y: i64, start: u64, end: u64) -> Placement {
    Placement {
        reservation_id: id.to_owned(),
        block_id: format!("block-{id}"),
        footprint: rect(x, y, x + 2, y + 2),
        occupied: TimeRange {
            start_tick: start,
            end_tick: end,
        },
    }
}

fn transfer(vehicle: &str) -> Transfer {
    Transfer {
        id: "transfer-a".to_owned(),
        block_id: "block-transfer-a".to_owned(),
        corridor: rect(0, 8, 2, 10),
        interval: TimeRange {
            start_tick: 5,
            end_tick: 7,
        },
        vehicle_id: vehicle.to_owned(),
        predecessor_move_ids: Vec::new(),
    }
}

fn scenario(placement_y: i64, vehicle: &str) -> Scenario {
    Scenario {
        yard_bounds: Some(Rect {
            min_x: 0,
            min_y: 0,
            max_x: 20,
            max_y: 20,
        }),
        placements: vec![
            placement("placement-a", 0, placement_y, 4, 10),
            placement("placement-b", 4, 0, 6, 20),
        ],
        transfers: vec![transfer(vehicle)],
        ..Scenario::default()
    }
}

fn empty_disruption() -> Disruption {
    Disruption {
        observed_at_tick: 0,
        new_exclusion_zones: Vec::new(),
        vehicle_outages: Vec::new(),
        placement_occupancy_extensions: Vec::new(),
        explicitly_affected_reservation_ids: Vec::new(),
    }
}

#[test]
fn exhaustive_dispatch_policy_grid_matches_independent_acceptance_predicate() {
    let baseline = scenario(0, "tug-a");
    let mut cases = 0usize;
    let mut expected_accepted = 0usize;
    let mut observed_accepted = 0usize;
    let mut frozen_placement_rejections = 0usize;
    let mut frozen_transfer_rejections = 0usize;
    let mut budget_rejections = 0usize;

    for placement_changed in [false, true] {
        for transfer_changed in [false, true] {
            for freeze_before_tick in 0_u64..8 {
                for placement_budget_bit in 0_usize..2 {
                    for transfer_budget_bit in 0_usize..2 {
                        for unaffected_placement_budget_bit in 0_usize..2 {
                            for rectangle_budget_bit in 0_u64..2 {
                                let candidate = scenario(
                                    if placement_changed { 2 } else { 0 },
                                    if transfer_changed { "tug-b" } else { "tug-a" },
                                );
                                let policy = DispatchStabilityPolicy {
                                    freeze_before_tick,
                                    allow_affected_frozen_changes: false,
                                    max_changed_placement_reservations: placement_budget_bit,
                                    max_changed_transfer_reservations: transfer_budget_bit,
                                    max_unaffected_placement_churn:
                                        unaffected_placement_budget_bit,
                                    max_unaffected_transfer_churn: transfer_budget_bit,
                                    max_time_shift_ticks: 0,
                                    max_rectangle_edge_shift_units: rectangle_budget_bit * 4,
                                };

                                let placement_is_frozen =
                                    placement_changed && 4 < freeze_before_tick;
                                let transfer_is_frozen =
                                    transfer_changed && 5 < freeze_before_tick;
                                let total_rework_budget_passes =
                                    (!placement_changed || placement_budget_bit == 1)
                                        && (!transfer_changed || transfer_budget_bit == 1);
                                let churn_budget_passes =
                                    (!placement_changed || unaffected_placement_budget_bit == 1)
                                        && (!transfer_changed || transfer_budget_bit == 1);
                                let geometry_budget_passes =
                                    !placement_changed || rectangle_budget_bit == 1;
                                let expected = !placement_is_frozen
                                    && !transfer_is_frozen
                                    && total_rework_budget_passes
                                    && churn_budget_passes
                                    && geometry_budget_passes;

                                let input = RecoveryInput {
                                    baseline: baseline.clone(),
                                    candidate,
                                    disruption: empty_disruption(),
                                };
                                let report = evaluate_operational_recovery(&input, &policy);
                                assert!(
                                    report.recovery.accepted,
                                    "fixture should always be physically feasible: {report:?}"
                                );
                                assert_eq!(
                                    report.operationally_acceptable,
                                    expected,
                                    "policy mismatch case={cases}, freeze={freeze_before_tick}, placement_changed={placement_changed}, transfer_changed={transfer_changed}, bits={placement_budget_bit}/{transfer_budget_bit}/{unaffected_placement_budget_bit}/{rectangle_budget_bit}: {:?}",
                                    report.policy_violations
                                );

                                assert_eq!(
                                    report.policy_violations.iter().any(|violation| {
                                        violation.code
                                            == DispatchPolicyViolationCode::FrozenPlacementChanged
                                    }),
                                    placement_is_frozen
                                );
                                assert_eq!(
                                    report.policy_violations.iter().any(|violation| {
                                        violation.code
                                            == DispatchPolicyViolationCode::FrozenTransferChanged
                                    }),
                                    transfer_is_frozen
                                );

                                cases += 1;
                                expected_accepted += usize::from(expected);
                                observed_accepted +=
                                    usize::from(report.operationally_acceptable);
                                frozen_placement_rejections +=
                                    usize::from(placement_is_frozen);
                                frozen_transfer_rejections += usize::from(transfer_is_frozen);
                                budget_rejections +=
                                    usize::from(!expected && !placement_is_frozen && !transfer_is_frozen);
                            }
                        }
                    }
                }
            }
        }
    }

    assert_eq!(cases, CASES);
    assert_eq!(expected_accepted, 191);
    assert_eq!(observed_accepted, expected_accepted);
    assert_eq!(frozen_placement_rejections, 192);
    assert_eq!(frozen_transfer_rejections, 128);
    assert_eq!(budget_rejections, 64);
}
