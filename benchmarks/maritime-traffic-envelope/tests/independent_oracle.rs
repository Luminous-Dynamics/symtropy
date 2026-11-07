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
    EvidenceReceipt, EvidenceState, RequiredTrafficUse, TrafficCoverageEntry,
    TrafficCoverageManifest, TrafficDisposition, TrafficOccupancy, TrafficResource,
    TrafficResourceKind, TrafficWindow, TrafficWorkload, canonical_occupancy_subject_id,
    canonical_resource_subject_id, canonical_traffic_coverage_subject_id, evaluate_traffic_plan,
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
        corridor_id: "oracle-corridor".to_owned(),
        time_basis_id: "oracle-time-basis-v1".to_owned(),
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
        corridor_id: "oracle-corridor".to_owned(),
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

fn coverage_manifest(
    operations: &[OperationPlacement],
    uses: &[TrafficOccupancy],
) -> TrafficCoverageManifest {
    let entries = operations
        .iter()
        .map(|operation| {
            let matching: Vec<&TrafficOccupancy> = uses
                .iter()
                .filter(|occupancy| occupancy.operation == *operation)
                .collect();
            let route_revision = matching
                .first()
                .map_or_else(|| "no-resource-route-r1".to_owned(), |item| {
                    item.route_revision.clone()
                });
            let required_uses = matching
                .iter()
                .map(|item| RequiredTrafficUse {
                    occupancy_id: item.occupancy_id.clone(),
                    resource_id: item.resource_id.clone(),
                    resource_revision: "zone-r1".to_owned(),
                    direction_id: item.direction_id.clone(),
                })
                .collect();
            TrafficCoverageEntry {
                operation: operation.clone(),
                route_revision,
                required_uses,
                no_resource_basis: None,
            }
        })
        .collect();
    let mut manifest = TrafficCoverageManifest {
        schema_version: "traffic-coverage-v1".to_owned(),
        corridor_id: "oracle-corridor".to_owned(),
        time_basis_id: "oracle-time-basis-v1".to_owned(),
        graph_revision: "graph-r1".to_owned(),
        entries,
        evidence: receipt("coverage-evidence", "", "graph-r1"),
    };
    manifest.evidence.subject_id = canonical_traffic_coverage_subject_id(&manifest);
    manifest
}

fn workload(
    operations: &[OperationPlacement],
    uses: Vec<TrafficOccupancy>,
) -> TrafficWorkload {
    let coverage = coverage_manifest(operations, &uses);
    TrafficWorkload {
        corridor_id: "oracle-corridor".to_owned(),
        time_basis_id: "oracle-time-basis-v1".to_owned(),
        decision_tick: 0,
        coverage,
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

/// Independent roster-to-occupancy completeness calculation. This deliberately
/// compares public fields directly and does not call the production coverage
/// validator or canonical identity helpers.
fn reference_coverage_is_complete(
    plan: &FleetPlan,
    input: &TrafficWorkload,
) -> bool {
    if plan.operations.is_empty()
        || plan.operations.len() != input.coverage.entries.len()
        || input.coverage.corridor_id != input.corridor_id
        || input.coverage.time_basis_id != input.time_basis_id
    {
        return false;
    }

    for operation in &plan.operations {
        let entries: Vec<_> = input
            .coverage
            .entries
            .iter()
            .filter(|entry| entry.operation == *operation)
            .collect();
        if entries.len() != 1 {
            return false;
        }
        let entry = entries[0];
        if entry.required_uses.is_empty() {
            if entry
                .no_resource_basis
                .as_ref()
                .is_none_or(|basis| basis.trim().is_empty())
            {
                return false;
            }
        } else if entry.no_resource_basis.is_some() {
            return false;
        }

        for required in &entry.required_uses {
            let resource_matches = input.resources.iter().any(|resource| {
                resource.resource_id == required.resource_id
                    && resource.revision == required.resource_revision
            });
            if !resource_matches {
                return false;
            }
            let occupancy_matches: Vec<_> = input
                .occupancies
                .iter()
                .filter(|occupancy| occupancy.occupancy_id == required.occupancy_id)
                .collect();
            if occupancy_matches.len() != 1 {
                return false;
            }
            let occupancy = occupancy_matches[0];
            if occupancy.operation != *operation
                || occupancy.resource_id != required.resource_id
                || occupancy.route_revision != entry.route_revision
                || occupancy.direction_id != required.direction_id
            {
                return false;
            }
        }
    }

    input.occupancies.iter().all(|occupancy| {
        input.coverage.entries.iter().any(|entry| {
            entry.operation == occupancy.operation
                && entry.route_revision == occupancy.route_revision
                && entry.required_uses.iter().any(|required| {
                    required.occupancy_id == occupancy.occupancy_id
                        && required.resource_id == occupancy.resource_id
                        && required.direction_id == occupancy.direction_id
                })
        })
    })
}

#[test]
fn production_coverage_validator_matches_direct_roster_oracle_for_all_two_operation_subsets() {
    let first = operation("demand-a", 10);
    let second = operation("demand-b", 20);
    let first_use = occupancy("use-a", first.clone(), 0, 0);
    let second_use = occupancy("use-b", second.clone(), 0, 0);
    let selected = plan(vec![first, second]);
    let baseline = workload(
        &selected.operations,
        vec![first_use, second_use],
    );

    for mask in 0_u8..4 {
        let mut candidate = baseline.clone();
        candidate.occupancies.retain(|item| {
            (item.occupancy_id == "use-a" && mask & 1 != 0)
                || (item.occupancy_id == "use-b" && mask & 2 != 0)
        });

        let expected = reference_coverage_is_complete(&selected, &candidate);
        let actual = evaluate_traffic_plan(&selected, &candidate);
        assert_eq!(
            actual.is_ok(),
            expected,
            "coverage mismatch for two-operation presence mask {mask:02b}"
        );
        if mask == 3 {
            assert_eq!(
                actual.expect("complete operation and occupancy roster").disposition,
                TrafficDisposition::ReadyForComparison
            );
        } else {
            assert_eq!(
                actual,
                Err(
                    maritime_traffic_envelope_contract::TrafficError::MissingRequiredOccupancy(
                        if mask & 1 == 0 { "use-a" } else { "use-b" }.to_owned()
                    )
                )
            );
        }
    }
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
                        &workload(
                            &[first.clone(), second.clone()],
                            vec![
                                occupancy("use-a", first, early, late),
                                occupancy("use-b", second, early, late),
                            ],
                        ),
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
