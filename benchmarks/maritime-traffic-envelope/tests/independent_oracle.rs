//! Independent direct interval oracle for robust traffic occupancy.
//!
//! The reference calculation below expands each use arithmetically and checks
//! the single conflict-zone capacity directly. It does not call the production
//! effective-occupancy or conflict-detection helpers.

use maritime_service_chain_contract::{ServiceUnit, TickWindow};
use maritime_service_chain_fleet::{
    DemandSelection, FleetObjective, FleetPlan, OperationPlacement,
};
use maritime_traffic_envelope_contract::{
    EvidenceReceipt, EvidenceState, TrafficDisposition, TrafficOccupancy, TrafficResource,
    TrafficResourceKind, TrafficWindow, TrafficWorkload, canonical_occupancy_subject_id,
    canonical_resource_subject_id, evaluate_traffic_plan,
};

fn receipt(id: &str, subject_id: &str, revision: &str) -> EvidenceReceipt {
    EvidenceReceipt {
        evidence_id: id.to_owned(),
        source_id: format!("source-{id}"),
        source_revision: "oracle-source-r1".to_owned(),
        subject_id: subject_id.to_owned(),
        subject_revision: revision.to_owned(),
        state: EvidenceState::Current,
        valid_from_tick: 0,
        valid_through_tick: 100,
        claim_ceiling: "independent synthetic interval oracle".to_owned(),
    }
}

fn operation(demand: &str, start: u64) -> OperationPlacement {
    OperationPlacement {
        demand_id: demand.to_owned(),
        participant_id: format!("participant-{demand}"),
        alternative_id: "alternative-1".to_owned(),
        segment_id: format!("segment-{demand}"),
        start_tick: start,
        end_tick: start + 2,
        resource_uses: Vec::new(),
    }
}

fn plan(operations: Vec<OperationPlacement>) -> FleetPlan {
    let selections = operations
        .iter()
        .map(|operation| DemandSelection {
            demand_id: operation.demand_id.clone(),
            participant_id: operation.participant_id.clone(),
            alternative_id: operation.alternative_id.clone(),
            delivery_tick: operation.end_tick,
            service_amount: 1,
            service_unit: ServiceUnit::Tonnes,
        })
        .collect();
    let makespan_tick = operations
        .iter()
        .map(|operation| operation.end_tick)
        .max()
        .unwrap_or(0);
    FleetPlan {
        selections,
        operations,
        objective: FleetObjective {
            makespan_tick,
            total_delivery_ticks: 0,
            total_idle_wait_ticks: 0,
            water_ml: 0,
            propulsion_wh: 0,
            auxiliary_wh: 0,
            shore_power_import_wh: 0,
            inland_transfer_wh: 0,
        },
    }
}

fn resource() -> TrafficResource {
    let mut resource = TrafficResource {
        resource_id: "conflict-zone".to_owned(),
        revision: "zone-r1".to_owned(),
        kind: TrafficResourceKind::ConflictZone,
        windows: vec![TrafficWindow {
            window: TickWindow {
                start_tick: 0,
                end_tick: 100,
            },
            direction_id: None,
        }],
        clearance_before_ticks: 1,
        clearance_after_ticks: 1,
        evidence: receipt("zone-evidence", "", "zone-r1"),
    };
    resource.evidence.subject_id = canonical_resource_subject_id(&resource);
    resource
}

fn occupancy(id: &str, op: OperationPlacement, early: u64, late: u64) -> TrafficOccupancy {
    let mut item = TrafficOccupancy {
        occupancy_id: id.to_owned(),
        resource_id: "conflict-zone".to_owned(),
        route_revision: "route-r1".to_owned(),
        operation: op,
        direction_id: None,
        early_entry_ticks: early,
        late_clearance_ticks: late,
        evidence: receipt(&format!("evidence-{id}"), "", "route-r1"),
    };
    item.evidence.subject_id = canonical_occupancy_subject_id(&item, "oracle-time-basis-v1");
    item
}

fn workload(uses: Vec<TrafficOccupancy>) -> TrafficWorkload {
    TrafficWorkload {
        corridor_id: "oracle-corridor".to_owned(),
        time_basis_id: "oracle-time-basis-v1".to_owned(),
        decision_tick: 0,
        resources: vec![resource()],
        occupancies: uses,
    }
}

/// Independent closed-form reference for one exclusive conflict zone.
/// Half-open intervals overlap iff a.start < b.end and b.start < a.end.
fn direct_reference_has_no_conflict(
    first_start: u64,
    second_start: u64,
    early: u64,
    late: u64,
) -> bool {
    let first = (first_start - early - 1, first_start + 2 + late + 1);
    let second = (second_start - early - 1, second_start + 2 + late + 1);
    first.0 >= second.1 || second.0 >= first.1
}

#[test]
fn production_traffic_conflict_disposition_matches_direct_oracle_over_256_cases() {
    let mut comparisons = 0;
    for first_start in 4..=11 {
        for second_start in 4..=11 {
            for early in 0..=1 {
                for late in 0..=1 {
                    let first = operation("demand-a", first_start);
                    let second = operation("demand-b", second_start);
                    let expected =
                        direct_reference_has_no_conflict(first_start, second_start, early, late);
                    let actual = evaluate_traffic_plan(
                        &plan(vec![first.clone(), second.clone()]),
                        &workload(vec![
                            occupancy("use-a", first, early, late),
                            occupancy("use-b", second, early, late),
                        ]),
                    )
                    .expect("frozen synthetic traffic input");

                    assert_eq!(
                        actual.disposition == TrafficDisposition::ReadyForComparison,
                        expected,
                        "mismatch at starts=({first_start},{second_start}) uncertainty=({early},{late})"
                    );
                    comparisons += 1;
                }
            }
        }
    }
    assert_eq!(comparisons, 256);
}
