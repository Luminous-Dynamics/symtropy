//! Generated oracle for the closed-loop execution-to-recovery boundary.
//!
//! The expected decision uses only two explicit rules: a clearance is usable when its completion
//! event is valid and visible, or a lapsed placement has a positive occupancy witness observed
//! exactly at the decision tick and recorded no later than that tick. It does not call internal
//! bridge or recovery predicates to calculate the expected result.

use shipyard_disruption_recovery::{
    Disruption,
    execution::{ExecutionEvent, ExecutionEventKind, ExecutionReplayCutoff, PlannedOperation},
    execution_bridge::{
        ExecutionDrivenRecoveryInput, PlacementClearanceBinding, PlacementOccupancyWitness,
        evaluate_execution_driven_recovery,
    },
};
use shipyard_space_time::{Footprint, Placement, Rect, Scenario, TimeRange};

const CASES: usize = 256;

fn baseline() -> Scenario {
    Scenario {
        yard_bounds: Some(Rect {
            min_x: 0,
            min_y: 0,
            max_x: 10,
            max_y: 10,
        }),
        placements: vec![Placement {
            reservation_id: "block-a-placement".to_owned(),
            block_id: "block-a".to_owned(),
            footprint: Footprint::AxisAlignedRect(Rect {
                min_x: 0,
                min_y: 0,
                max_x: 2,
                max_y: 2,
            }),
            occupied: TimeRange {
                start_tick: 0,
                end_tick: 5,
            },
        }],
        ..Scenario::default()
    }
}

fn plan() -> [PlannedOperation; 1] {
    [PlannedOperation {
        operation_id: "lift-block-a-clear".to_owned(),
        planned_start_tick: 2,
        planned_end_tick: 5,
    }]
}

fn start_event() -> [ExecutionEvent; 1] {
    [ExecutionEvent {
        event_id: "start-clear".to_owned(),
        operation_id: "lift-block-a-clear".to_owned(),
        source_id: "yard-mes".to_owned(),
        evidence_ref: "event://start-clear".to_owned(),
        kind: ExecutionEventKind::Started,
        occurred_tick: 2,
        recorded_tick: 2,
    }]
}

fn prior_disruption(tick: u64) -> Disruption {
    Disruption {
        observed_at_tick: tick,
        new_exclusion_zones: Vec::new(),
        vehicle_outages: Vec::new(),
        placement_occupancy_extensions: Vec::new(),
        explicitly_affected_reservation_ids: Vec::new(),
    }
}

#[test]
fn generated_256_case_oracle_rejects_stale_future_and_late_recorded_witnesses() {
    let mut cases = 0usize;
    let mut expected_acceptances = 0usize;
    let mut actual_acceptances = 0usize;
    let mut false_approvals = 0usize;
    let mut false_rejections = 0usize;

    for index in 0..CASES {
        let decision_tick = 6 + u64::try_from(index % 10).unwrap();
        let mode = index % 4;
        let (occupied_at_tick, recorded_tick, expected_witness_visible) = match mode {
            0 => (decision_tick, decision_tick, true),
            1 => (decision_tick, decision_tick + 1, false),
            2 => (decision_tick - 1, decision_tick, false),
            _ => (decision_tick + 1, decision_tick + 1, false),
        };
        let witnesses = [PlacementOccupancyWitness {
            witness_id: format!("witness-{index}"),
            placement_reservation_id: "block-a-placement".to_owned(),
            source_id: "yard-scanner".to_owned(),
            evidence_ref: format!("scan://block-a/{occupied_at_tick}"),
            occupied_at_tick,
            recorded_tick,
        }];
        let baseline = baseline();
        let plan = plan();
        let events = start_event();
        let bindings = [PlacementClearanceBinding {
            operation_id: "lift-block-a-clear".to_owned(),
            placement_reservation_id: "block-a-placement".to_owned(),
        }];
        let prior = prior_disruption(decision_tick - 1);
        let report = evaluate_execution_driven_recovery(&ExecutionDrivenRecoveryInput {
            baseline: &baseline,
            candidate: &baseline,
            planned_operations: &plan,
            events: &events,
            occupancy_witnesses: &witnesses,
            cutoff: ExecutionReplayCutoff {
                as_of_occurred_tick: decision_tick,
                as_of_recorded_tick: decision_tick,
            },
            decision_tick,
            prior_disruption: &prior,
            bindings: &bindings,
        });

        let expected = expected_witness_visible;
        let actual = report.accepted;
        if expected {
            expected_acceptances += 1;
        }
        if actual {
            actual_acceptances += 1;
        }
        if actual && !expected {
            false_approvals += 1;
        }
        if expected && !actual {
            false_rejections += 1;
        }
        assert_eq!(
            actual,
            expected,
            "case {index}, mode {mode}, decision {decision_tick}, report {report:?}"
        );
        assert!(report.bridge.execution_replay.evidence_valid);
        if expected {
            assert_eq!(
                report.bridge.receipts.len(),
                1,
                "current witness should produce a provenance receipt"
            );
            assert!(report.recovery.is_some());
        } else {
            assert!(report.bridge.disruption.is_none());
            assert!(report.recovery.is_none());
        }
        cases += 1;
    }

    assert_eq!(cases, CASES);
    assert_eq!(expected_acceptances, 64);
    assert_eq!(actual_acceptances, expected_acceptances);
    assert_eq!(false_approvals, 0);
    assert_eq!(false_rejections, 0);
}
