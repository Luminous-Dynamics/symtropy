use maritime_service_chain_contract::{ServiceUnit, TickWindow};
use maritime_service_chain_fleet::{
    DemandSelection, FleetObjective, FleetPlan, OperationPlacement,
};
use maritime_traffic_envelope_contract::{
    EvidenceReceipt, EvidenceState, TrafficDisposition, TrafficError, TrafficOccupancy,
    RequiredTrafficUse, TrafficCoverageEntry, TrafficCoverageManifest, TrafficReport,
    TrafficResource, TrafficResourceKind, TrafficViolation, TrafficWindow, TrafficWorkload,
    canonical_occupancy_subject_id, canonical_resource_subject_id,
    canonical_traffic_coverage_subject_id, evaluate_traffic_plan,
};

fn evidence(
    id: &str,
    subject_id: &str,
    subject_revision: &str,
    state: EvidenceState,
) -> EvidenceReceipt {
    EvidenceReceipt {
        evidence_id: id.to_owned(),
        source_id: format!("source-{id}"),
        source_revision: "source-revision-1".to_owned(),
        subject_id: subject_id.to_owned(),
        subject_revision: subject_revision.to_owned(),
        state,
        valid_from_tick: 0,
        valid_through_tick: 200,
        claim_ceiling: "abstract traffic-envelope fixture only".to_owned(),
    }
}

fn resource(
    id: &str,
    revision: &str,
    kind: TrafficResourceKind,
    windows: Vec<TrafficWindow>,
    clearance_before_ticks: u64,
    clearance_after_ticks: u64,
) -> TrafficResource {
    let mut item = TrafficResource {
        corridor_id: "synthetic-corridor".to_owned(),
        time_basis_id: "synthetic-ticks-v1".to_owned(),
        resource_id: id.to_owned(),
        revision: revision.to_owned(),
        kind,
        windows,
        clearance_before_ticks,
        clearance_after_ticks,
        evidence: evidence("resource-evidence", "", revision, EvidenceState::Current),
    };
    item.evidence.subject_id = canonical_resource_subject_id(&item);
    item
}

fn conflict_resource(before: u64, after: u64) -> TrafficResource {
    resource(
        "crossing-zone-1",
        "conflict-r1",
        TrafficResourceKind::ConflictZone,
        vec![TrafficWindow {
            window: TickWindow {
                start_tick: 0,
                end_tick: 200,
            },
            direction_id: None,
        }],
        before,
        after,
    )
}

fn operation(
    demand_id: &str,
    participant_id: &str,
    start_tick: u64,
    end_tick: u64,
) -> OperationPlacement {
    OperationPlacement {
        demand_id: demand_id.to_owned(),
        participant_id: participant_id.to_owned(),
        alternative_id: "alternative-1".to_owned(),
        segment_id: format!("segment-{demand_id}"),
        start_tick,
        end_tick,
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
            service_amount: 10,
            service_unit: ServiceUnit::Tonnes,
        })
        .collect();
    let makespan_tick = operations
        .iter()
        .map(|operation| operation.end_tick)
        .max()
        .unwrap_or(0);
    let total_delivery_ticks = operations
        .iter()
        .map(|operation| u128::from(operation.end_tick))
        .sum();
    FleetPlan {
        selections,
        operations,
        objective: FleetObjective {
            makespan_tick,
            total_delivery_ticks,
            total_idle_wait_ticks: 0,
            water_ml: 0,
            propulsion_wh: 0,
            auxiliary_wh: 0,
            shore_power_import_wh: 0,
            inland_transfer_wh: 0,
        },
    }
}

struct OccupancySpec<'a> {
    id: &'a str,
    resource_id: &'a str,
    route_revision: &'a str,
    operation: OperationPlacement,
    direction_id: Option<&'a str>,
    early_entry_ticks: u64,
    late_clearance_ticks: u64,
    time_basis_id: &'a str,
}

fn occupancy(spec: OccupancySpec<'_>) -> TrafficOccupancy {
    let mut item = TrafficOccupancy {
        corridor_id: "synthetic-corridor".to_owned(),
        occupancy_id: spec.id.to_owned(),
        resource_id: spec.resource_id.to_owned(),
        route_revision: spec.route_revision.to_owned(),
        operation: spec.operation,
        direction_id: spec.direction_id.map(str::to_owned),
        early_entry_ticks: spec.early_entry_ticks,
        late_clearance_ticks: spec.late_clearance_ticks,
        evidence: evidence(
            &format!("occupancy-evidence-{}", spec.id),
            "",
            spec.route_revision,
            EvidenceState::Current,
        ),
    };
    item.evidence.subject_id = canonical_occupancy_subject_id(&item, spec.time_basis_id);
    item
}

fn coverage_manifest(
    operations: &[OperationPlacement],
    resources: &[TrafficResource],
    occupancies: &[TrafficOccupancy],
) -> TrafficCoverageManifest {
    let entries = operations
        .iter()
        .map(|operation| {
            let matching: Vec<&TrafficOccupancy> = occupancies
                .iter()
                .filter(|occupancy| occupancy.operation == *operation)
                .collect();
            let route_revision = matching
                .first()
                .map_or_else(|| "no-resource-route-r1".to_owned(), |item| {
                    item.route_revision.clone()
                });
            let required_uses: Vec<RequiredTrafficUse> = matching
                .iter()
                .map(|item| RequiredTrafficUse {
                    occupancy_id: item.occupancy_id.clone(),
                    resource_id: item.resource_id.clone(),
                    resource_revision: resources
                        .iter()
                        .find(|resource| resource.resource_id == item.resource_id)
                        .map_or_else(|| "missing-resource-revision".to_owned(), |resource| {
                            resource.revision.clone()
                        }),
                    direction_id: item.direction_id.clone(),
                })
                .collect();
            TrafficCoverageEntry {
                operation: operation.clone(),
                route_revision,
                no_resource_basis: if required_uses.is_empty() {
                    Some("Reviewed synthetic projection: operation has no traffic-resource use".to_owned())
                } else {
                    None
                },
                required_uses,
            }
        })
        .collect();
    let mut manifest = TrafficCoverageManifest {
        schema_version: "traffic-coverage-v1".to_owned(),
        corridor_id: "synthetic-corridor".to_owned(),
        time_basis_id: "synthetic-ticks-v1".to_owned(),
        graph_revision: "traffic-graph-r1".to_owned(),
        entries,
        evidence: evidence(
            "traffic-coverage-evidence",
            "",
            "traffic-graph-r1",
            EvidenceState::Current,
        ),
    };
    manifest.evidence.subject_id = canonical_traffic_coverage_subject_id(&manifest);
    manifest
}

fn workload(
    operations: &[OperationPlacement],
    resources: Vec<TrafficResource>,
    occupancies: Vec<TrafficOccupancy>,
) -> TrafficWorkload {
    let coverage = coverage_manifest(operations, &resources, &occupancies);
    TrafficWorkload {
        corridor_id: "synthetic-corridor".to_owned(),
        time_basis_id: "synthetic-ticks-v1".to_owned(),
        decision_tick: 0,
        coverage,
        resources,
        occupancies,
    }
}

fn evaluate(
    operations: Vec<OperationPlacement>,
    resources: Vec<TrafficResource>,
    occupancies: Vec<TrafficOccupancy>,
) -> Result<TrafficReport, TrafficError> {
    let fleet_plan = plan(operations);
    let input = workload(&fleet_plan.operations, resources, occupancies);
    evaluate_traffic_plan(&fleet_plan, &input)
}

#[test]
fn non_overlapping_occupancies_are_admitted_with_half_open_boundaries() {
    let first = operation("demand-a", "participant-a", 10, 12);
    let second = operation("demand-b", "participant-b", 12, 14);
    let occupancies = vec![
        occupancy(OccupancySpec {
            id: "use-a",
            resource_id: "crossing-zone-1",
            route_revision: "route-a",
            operation: first.clone(),
            direction_id: None,
            early_entry_ticks: 0,
            late_clearance_ticks: 0,
            time_basis_id: "synthetic-ticks-v1",
        }),
        occupancy(OccupancySpec {
            id: "use-b",
            resource_id: "crossing-zone-1",
            route_revision: "route-b",
            operation: second.clone(),
            direction_id: None,
            early_entry_ticks: 0,
            late_clearance_ticks: 0,
            time_basis_id: "synthetic-ticks-v1",
        }),
    ];
    let report = evaluate(
        vec![first, second],
        vec![conflict_resource(0, 0)],
        occupancies,
    )
    .expect("valid selected operations");
    assert_eq!(report.disposition, TrafficDisposition::ReadyForComparison);
    assert!(report.violations.is_empty());
}

#[test]
fn nominally_separate_operations_conflict_when_early_entry_and_late_clearance_overlap() {
    let first = operation("demand-a", "participant-a", 10, 12);
    let second = operation("demand-b", "participant-b", 13, 15);
    let occupancies = vec![
        occupancy(OccupancySpec {
            id: "use-a",
            resource_id: "crossing-zone-1",
            route_revision: "route-a",
            operation: first.clone(),
            direction_id: None,
            early_entry_ticks: 1,
            late_clearance_ticks: 1,
            time_basis_id: "synthetic-ticks-v1",
        }),
        occupancy(OccupancySpec {
            id: "use-b",
            resource_id: "crossing-zone-1",
            route_revision: "route-b",
            operation: second.clone(),
            direction_id: None,
            early_entry_ticks: 1,
            late_clearance_ticks: 1,
            time_basis_id: "synthetic-ticks-v1",
        }),
    ];
    let report = evaluate(
        vec![first, second],
        vec![conflict_resource(0, 0)],
        occupancies,
    )
    .expect("valid selected operations");
    assert_eq!(report.disposition, TrafficDisposition::Infeasible);
    assert_eq!(
        report.violations,
        vec![TrafficViolation::Conflict {
            resource_id: "crossing-zone-1".to_owned(),
            first_occupancy_id: "use-a".to_owned(),
            second_occupancy_id: "use-b".to_owned(),
        }]
    );
}

#[test]
fn clearance_guards_are_applied_to_each_half_open_occupancy() {
    let first = operation("demand-a", "participant-a", 10, 12);
    let second = operation("demand-b", "participant-b", 13, 15);
    let occupancies = vec![
        occupancy(OccupancySpec {
            id: "use-a",
            resource_id: "crossing-zone-1",
            route_revision: "route-a",
            operation: first.clone(),
            direction_id: None,
            early_entry_ticks: 0,
            late_clearance_ticks: 0,
            time_basis_id: "synthetic-ticks-v1",
        }),
        occupancy(OccupancySpec {
            id: "use-b",
            resource_id: "crossing-zone-1",
            route_revision: "route-b",
            operation: second.clone(),
            direction_id: None,
            early_entry_ticks: 0,
            late_clearance_ticks: 0,
            time_basis_id: "synthetic-ticks-v1",
        }),
    ];
    let report = evaluate(
        vec![first, second],
        vec![conflict_resource(1, 1)],
        occupancies,
    )
    .expect("valid selected operations");
    assert_eq!(report.disposition, TrafficDisposition::Infeasible);
    assert!(matches!(
        report.violations.as_slice(),
        [TrafficViolation::Conflict { .. }]
    ));
}

#[test]
fn opposite_directions_are_admitted_only_in_their_declared_one_way_windows() {
    let resource = resource(
        "one-way-channel-1",
        "channel-r1",
        TrafficResourceKind::OneWayChannel,
        vec![
            TrafficWindow {
                window: TickWindow {
                    start_tick: 0,
                    end_tick: 10,
                },
                direction_id: Some("eastbound".to_owned()),
            },
            TrafficWindow {
                window: TickWindow {
                    start_tick: 10,
                    end_tick: 20,
                },
                direction_id: Some("westbound".to_owned()),
            },
        ],
        0,
        0,
    );
    let first = operation("demand-a", "participant-a", 2, 4);
    let second = operation("demand-b", "participant-b", 12, 14);
    let occupancies = vec![
        occupancy(OccupancySpec {
            id: "east",
            resource_id: "one-way-channel-1",
            route_revision: "route-a",
            operation: first.clone(),
            direction_id: Some("eastbound"),
            early_entry_ticks: 0,
            late_clearance_ticks: 0,
            time_basis_id: "synthetic-ticks-v1",
        }),
        occupancy(OccupancySpec {
            id: "west",
            resource_id: "one-way-channel-1",
            route_revision: "route-b",
            operation: second.clone(),
            direction_id: Some("westbound"),
            early_entry_ticks: 0,
            late_clearance_ticks: 0,
            time_basis_id: "synthetic-ticks-v1",
        }),
    ];
    let report = evaluate(vec![first, second], vec![resource], occupancies)
        .expect("direction windows are sequential");
    assert_eq!(report.disposition, TrafficDisposition::ReadyForComparison);
}

#[test]
fn opposite_direction_is_rejected_inside_the_wrong_one_way_window() {
    let resource = resource(
        "one-way-channel-1",
        "channel-r1",
        TrafficResourceKind::OneWayChannel,
        vec![TrafficWindow {
            window: TickWindow {
                start_tick: 0,
                end_tick: 20,
            },
            direction_id: Some("eastbound".to_owned()),
        }],
        0,
        0,
    );
    let op = operation("demand-a", "participant-a", 2, 4);
    let use_item = occupancy(OccupancySpec {
        id: "westbound-use",
        resource_id: "one-way-channel-1",
        route_revision: "route-a",
        operation: op.clone(),
        direction_id: Some("westbound"),
        early_entry_ticks: 0,
        late_clearance_ticks: 0,
        time_basis_id: "synthetic-ticks-v1",
    });
    let report = evaluate(vec![op], vec![resource], vec![use_item])
        .expect("well-formed but disallowed direction");
    assert_eq!(report.disposition, TrafficDisposition::Infeasible);
    assert_eq!(
        report.violations,
        vec![TrafficViolation::DirectionNotPermitted(
            "westbound-use".to_owned()
        )]
    );
}

#[test]
fn uncertainty_envelope_must_fit_inside_one_direction_window() {
    let resource = resource(
        "one-way-channel-1",
        "channel-r1",
        TrafficResourceKind::OneWayChannel,
        vec![
            TrafficWindow {
                window: TickWindow {
                    start_tick: 0,
                    end_tick: 10,
                },
                direction_id: Some("eastbound".to_owned()),
            },
            TrafficWindow {
                window: TickWindow {
                    start_tick: 10,
                    end_tick: 20,
                },
                direction_id: Some("westbound".to_owned()),
            },
        ],
        0,
        0,
    );
    let op = operation("demand-a", "participant-a", 9, 11);
    let use_item = occupancy(OccupancySpec {
        id: "cross-window",
        resource_id: "one-way-channel-1",
        route_revision: "route-a",
        operation: op.clone(),
        direction_id: Some("eastbound"),
        early_entry_ticks: 1,
        late_clearance_ticks: 0,
        time_basis_id: "synthetic-ticks-v1",
    });
    let report = evaluate(vec![op], vec![resource], vec![use_item])
        .expect("well-formed but window crossing");
    assert_eq!(report.disposition, TrafficDisposition::Infeasible);
    assert_eq!(
        report.violations,
        vec![TrafficViolation::TrafficWindowInsufficient(
            "cross-window".to_owned()
        )]
    );
}

#[test]
fn stale_traffic_evidence_needs_review_and_revoked_evidence_is_unsupported() {
    let op = operation("demand-a", "participant-a", 10, 12);
    let mut stale = occupancy(OccupancySpec {
        id: "stale-use",
        resource_id: "crossing-zone-1",
        route_revision: "route-a",
        operation: op.clone(),
        direction_id: None,
        early_entry_ticks: 0,
        late_clearance_ticks: 0,
        time_basis_id: "synthetic-ticks-v1",
    });
    stale.evidence.state = EvidenceState::Stale;
    let report = evaluate(vec![op.clone()], vec![conflict_resource(0, 0)], vec![stale])
        .expect("stale evidence is a disposition");
    assert_eq!(report.disposition, TrafficDisposition::NeedsEvidence);

    let mut revoked = occupancy(OccupancySpec {
        id: "revoked-use",
        resource_id: "crossing-zone-1",
        route_revision: "route-a",
        operation: op.clone(),
        direction_id: None,
        early_entry_ticks: 0,
        late_clearance_ticks: 0,
        time_basis_id: "synthetic-ticks-v1",
    });
    revoked.evidence.state = EvidenceState::Revoked;
    let report = evaluate(vec![op], vec![conflict_resource(0, 0)], vec![revoked])
        .expect("revoked evidence is a disposition");
    assert_eq!(report.disposition, TrafficDisposition::Unsupported);
}

#[test]
fn traffic_resource_profile_is_scoped_to_corridor_and_time_basis() {
    let op = operation("demand-a", "participant-a", 10, 12);
    let item = occupancy(OccupancySpec {
        id: "use-a",
        resource_id: "crossing-zone-1",
        route_revision: "route-a",
        operation: op.clone(),
        direction_id: None,
        early_entry_ticks: 0,
        late_clearance_ticks: 0,
        time_basis_id: "synthetic-ticks-v1",
    });
    let mut resource = conflict_resource(0, 0);
    resource.time_basis_id = "different-epoch-v1".to_owned();
    // The scope check happens before the stale subject digest check.
    assert_eq!(
        evaluate(vec![op.clone()], vec![resource], vec![item.clone()]),
        Err(TrafficError::ResourceScopeMismatch(
            "crossing-zone-1".to_owned()
        ))
    );

    let mut item = item;
    item.corridor_id = "another-corridor".to_owned();
    assert_eq!(
        evaluate(vec![op], vec![conflict_resource(0, 0)], vec![item]),
        Err(TrafficError::OccupancyScopeMismatch("use-a".to_owned()))
    );
}

#[test]
fn traffic_resource_profile_must_be_bound_to_exact_revision_and_guards() {
    let op = operation("demand-a", "participant-a", 10, 12);
    let item = occupancy(OccupancySpec {
        id: "use-a",
        resource_id: "crossing-zone-1",
        route_revision: "route-a",
        operation: op.clone(),
        direction_id: None,
        early_entry_ticks: 0,
        late_clearance_ticks: 0,
        time_basis_id: "synthetic-ticks-v1",
    });
    let mut resource = conflict_resource(0, 0);
    resource.clearance_after_ticks = 1;
    assert_eq!(
        evaluate(vec![op], vec![resource], vec![item]),
        Err(TrafficError::ResourceEvidenceSubjectMismatch(
            "crossing-zone-1".to_owned()
        ))
    );
}

#[test]
fn occupancy_evidence_must_bind_to_the_exact_uncertainty_envelope() {
    let op = operation("demand-a", "participant-a", 10, 12);
    let mut item = occupancy(OccupancySpec {
        id: "use-a",
        resource_id: "crossing-zone-1",
        route_revision: "route-a",
        operation: op.clone(),
        direction_id: None,
        early_entry_ticks: 0,
        late_clearance_ticks: 0,
        time_basis_id: "synthetic-ticks-v1",
    });
    item.early_entry_ticks = 1;
    assert_eq!(
        evaluate(vec![op], vec![conflict_resource(0, 0)], vec![item]),
        Err(TrafficError::OccupancyEvidenceSubjectMismatch(
            "use-a".to_owned()
        ))
    );
}

#[test]
fn traffic_occupancy_must_be_the_exact_selected_fleet_operation() {
    let selected = operation("demand-a", "participant-a", 10, 12);
    let mut forged = selected.clone();
    forged.start_tick = 11;
    let item = occupancy(OccupancySpec {
        id: "forged-use",
        resource_id: "crossing-zone-1",
        route_revision: "route-a",
        operation: forged,
        direction_id: None,
        early_entry_ticks: 0,
        late_clearance_ticks: 0,
        time_basis_id: "synthetic-ticks-v1",
    });
    assert_eq!(
        evaluate(vec![selected], vec![conflict_resource(0, 0)], vec![item]),
        Err(TrafficError::PlanOperationMismatch("forged-use".to_owned()))
    );
}

#[test]
fn wrong_participant_or_alternative_is_rejected_even_when_operation_is_present() {
    let selected = operation("demand-a", "participant-a", 10, 12);
    let mut fleet_plan = plan(vec![selected.clone()]);
    fleet_plan.selections[0].participant_id = "different-participant".to_owned();
    let item = occupancy(OccupancySpec {
        id: "use-a",
        resource_id: "crossing-zone-1",
        route_revision: "route-a",
        operation: selected,
        direction_id: None,
        early_entry_ticks: 0,
        late_clearance_ticks: 0,
        time_basis_id: "synthetic-ticks-v1",
    });
    assert_eq!(
        evaluate_traffic_plan(
            &fleet_plan,
            &workload(&fleet_plan.operations, vec![conflict_resource(0, 0)], vec![item])
        ),
        Err(TrafficError::PlanSelectionMismatch("use-a".to_owned()))
    );
}

#[test]
fn stale_occupancy_evidence_after_its_validity_window_needs_review() {
    let op = operation("demand-a", "participant-a", 10, 12);
    let mut item = occupancy(OccupancySpec {
        id: "use-a",
        resource_id: "crossing-zone-1",
        route_revision: "route-a",
        operation: op.clone(),
        direction_id: None,
        early_entry_ticks: 0,
        late_clearance_ticks: 0,
        time_basis_id: "synthetic-ticks-v1",
    });
    item.evidence.valid_through_tick = 10;
    let report = evaluate(vec![op], vec![conflict_resource(0, 0)], vec![item])
        .expect("expiry is a review disposition");
    assert_eq!(report.disposition, TrafficDisposition::NeedsEvidence);
    assert!(report.violations.iter().any(|violation| matches!(
        violation,
        TrafficViolation::EvidenceWindowInsufficient { subject_id, .. }
            if subject_id == "use-a"
    )));
}

#[test]
fn equivalent_input_permutations_produce_identical_reports() {
    let first = operation("demand-a", "participant-a", 10, 12);
    let second = operation("demand-b", "participant-b", 20, 22);
    let occupancies = vec![
        occupancy(OccupancySpec {
            id: "use-a",
            resource_id: "crossing-zone-1",
            route_revision: "route-a",
            operation: first.clone(),
            direction_id: None,
            early_entry_ticks: 0,
            late_clearance_ticks: 0,
            time_basis_id: "synthetic-ticks-v1",
        }),
        occupancy(OccupancySpec {
            id: "use-b",
            resource_id: "crossing-zone-1",
            route_revision: "route-b",
            operation: second.clone(),
            direction_id: None,
            early_entry_ticks: 0,
            late_clearance_ticks: 0,
            time_basis_id: "synthetic-ticks-v1",
        }),
    ];
    let original = workload(
        &[first.clone(), second.clone()],
        vec![conflict_resource(0, 0)],
        occupancies,
    );
    let expected = evaluate_traffic_plan(&plan(vec![first.clone(), second.clone()]), &original)
        .expect("valid workload");

    let mut permuted = original;
    permuted.resources.reverse();
    permuted.occupancies.reverse();
    permuted.coverage.entries.reverse();
    for entry in &mut permuted.coverage.entries {
        entry.required_uses.reverse();
    }
    let actual =
        evaluate_traffic_plan(&plan(vec![second, first]), &permuted).expect("permuted workload");
    assert_eq!(actual, expected);
}

#[test]
fn traffic_windows_must_be_sorted_non_overlapping_and_typed() {
    let overlapping = resource(
        "one-way-channel-1",
        "channel-r1",
        TrafficResourceKind::OneWayChannel,
        vec![
            TrafficWindow {
                window: TickWindow {
                    start_tick: 0,
                    end_tick: 10,
                },
                direction_id: Some("east".to_owned()),
            },
            TrafficWindow {
                window: TickWindow {
                    start_tick: 9,
                    end_tick: 20,
                },
                direction_id: Some("west".to_owned()),
            },
        ],
        0,
        0,
    );
    let op = operation("demand-a", "participant-a", 2, 4);
    let item = occupancy(OccupancySpec {
        id: "use-a",
        resource_id: "one-way-channel-1",
        route_revision: "route-a",
        operation: op.clone(),
        direction_id: Some("east"),
        early_entry_ticks: 0,
        late_clearance_ticks: 0,
        time_basis_id: "synthetic-ticks-v1",
    });
    assert_eq!(
        evaluate(vec![op], vec![overlapping], vec![item]),
        Err(TrafficError::TrafficWindowsOverlap(
            "one-way-channel-1".to_owned()
        ))
    );
}

#[test]
fn omitted_required_occupancy_is_rejected_by_complete_coverage_manifest() {
    let op = operation("demand-a", "participant-a", 10, 12);
    let item = occupancy(OccupancySpec {
        id: "required-use",
        resource_id: "crossing-zone-1",
        route_revision: "route-a",
        operation: op.clone(),
        direction_id: None,
        early_entry_ticks: 0,
        late_clearance_ticks: 0,
        time_basis_id: "synthetic-ticks-v1",
    });
    let mut input = workload(
        std::slice::from_ref(&op),
        vec![conflict_resource(0, 0)],
        vec![item],
    );
    input.occupancies.clear();
    assert_eq!(
        evaluate_traffic_plan(&plan(vec![op]), &input),
        Err(TrafficError::MissingRequiredOccupancy(
            "required-use".to_owned()
        ))
    );
}

#[test]
fn every_selected_operation_must_have_a_coverage_entry() {
    let first = operation("demand-a", "participant-a", 10, 12);
    let second = operation("demand-b", "participant-b", 20, 22);
    let first_use = occupancy(OccupancySpec {
        id: "use-a",
        resource_id: "crossing-zone-1",
        route_revision: "route-a",
        operation: first.clone(),
        direction_id: None,
        early_entry_ticks: 0,
        late_clearance_ticks: 0,
        time_basis_id: "synthetic-ticks-v1",
    });
    let second_use = occupancy(OccupancySpec {
        id: "use-b",
        resource_id: "crossing-zone-1",
        route_revision: "route-b",
        operation: second.clone(),
        direction_id: None,
        early_entry_ticks: 0,
        late_clearance_ticks: 0,
        time_basis_id: "synthetic-ticks-v1",
    });
    let mut input = workload(
        &[first.clone(), second.clone()],
        vec![conflict_resource(0, 0)],
        vec![first_use, second_use],
    );
    input.coverage.entries.pop();
    input.coverage.evidence.subject_id =
        canonical_traffic_coverage_subject_id(&input.coverage);
    assert_eq!(
        evaluate_traffic_plan(&plan(vec![first, second]), &input),
        Err(TrafficError::MissingCoverageOperation(
            "segment-demand-b".to_owned()
        ))
    );
}

#[test]
fn an_extra_occupancy_not_declared_by_the_manifest_is_rejected() {
    let op = operation("demand-a", "participant-a", 10, 12);
    let mut input = workload(
        std::slice::from_ref(&op),
        vec![conflict_resource(0, 0)],
        Vec::new(),
    );
    input.occupancies.push(occupancy(OccupancySpec {
        id: "undeclared-use",
        resource_id: "crossing-zone-1",
        route_revision: "route-a",
        operation: op.clone(),
        direction_id: None,
        early_entry_ticks: 0,
        late_clearance_ticks: 0,
        time_basis_id: "synthetic-ticks-v1",
    }));
    assert_eq!(
        evaluate_traffic_plan(&plan(vec![op]), &input),
        Err(TrafficError::UnmappedOccupancy(
            "undeclared-use".to_owned()
        ))
    );
}

#[test]
fn explicit_reviewed_no_resource_entry_is_required_for_empty_mapping() {
    let op = operation("demand-a", "participant-a", 10, 12);
    let input = workload(std::slice::from_ref(&op), Vec::new(), Vec::new());
    let report = evaluate_traffic_plan(&plan(vec![op.clone()]), &input)
        .expect("reviewed no-resource coverage");
    assert_eq!(report.disposition, TrafficDisposition::ReadyForComparison);
    assert_eq!(report.resources_checked, 0);
    assert_eq!(report.occupancies_checked, 0);

    let mut incomplete = input;
    incomplete.coverage.entries[0].no_resource_basis = None;
    incomplete.coverage.evidence.subject_id =
        canonical_traffic_coverage_subject_id(&incomplete.coverage);
    assert_eq!(
        evaluate_traffic_plan(&plan(vec![op]), &incomplete),
        Err(TrafficError::InvalidCoverageEntry(
            "segment-demand-a".to_owned()
        ))
    );
}

#[test]
fn coverage_manifest_stale_and_revoked_states_never_yield_ready() {
    let op = operation("demand-a", "participant-a", 10, 12);
    let mut stale = workload(std::slice::from_ref(&op), Vec::new(), Vec::new());
    stale.coverage.evidence.state = EvidenceState::Stale;
    let report = evaluate_traffic_plan(&plan(vec![op.clone()]), &stale)
        .expect("stale coverage is a disposition");
    assert_eq!(report.disposition, TrafficDisposition::NeedsEvidence);
    assert!(report.violations.iter().any(|violation| matches!(
        violation,
        TrafficViolation::CoverageEvidenceNotCurrent {
            state: EvidenceState::Stale,
            ..
        }
    )));

    let mut revoked = stale;
    revoked.coverage.evidence.state = EvidenceState::Revoked;
    let report = evaluate_traffic_plan(&plan(vec![op]), &revoked)
        .expect("revoked coverage is a disposition");
    assert_eq!(report.disposition, TrafficDisposition::Unsupported);
}

#[test]
fn coverage_mapping_must_pin_the_exact_traffic_resource_revision() {
    let op = operation("demand-a", "participant-a", 10, 12);
    let item = occupancy(OccupancySpec {
        id: "use-a",
        resource_id: "crossing-zone-1",
        route_revision: "route-a",
        operation: op.clone(),
        direction_id: None,
        early_entry_ticks: 0,
        late_clearance_ticks: 0,
        time_basis_id: "synthetic-ticks-v1",
    });
    let mut input = workload(
        std::slice::from_ref(&op),
        vec![conflict_resource(0, 0)],
        vec![item],
    );
    input.coverage.entries[0].required_uses[0].resource_revision = "old-zone-r0".to_owned();
    input.coverage.evidence.subject_id =
        canonical_traffic_coverage_subject_id(&input.coverage);
    assert_eq!(
        evaluate_traffic_plan(&plan(vec![op]), &input),
        Err(TrafficError::CoverageResourceRevisionMismatch(
            "crossing-zone-1".to_owned()
        ))
    );
}

#[test]
fn changing_the_operation_resource_map_invalidates_the_manifest_identity() {
    let op = operation("demand-a", "participant-a", 10, 12);
    let item = occupancy(OccupancySpec {
        id: "use-a",
        resource_id: "crossing-zone-1",
        route_revision: "route-a",
        operation: op.clone(),
        direction_id: None,
        early_entry_ticks: 0,
        late_clearance_ticks: 0,
        time_basis_id: "synthetic-ticks-v1",
    });
    let mut input = workload(
        std::slice::from_ref(&op),
        vec![conflict_resource(0, 0)],
        vec![item],
    );
    input.coverage.entries[0].required_uses.clear();
    input.coverage.entries[0].no_resource_basis =
        Some("unreviewed changed mapping".to_owned());
    assert_eq!(
        evaluate_traffic_plan(&plan(vec![op]), &input),
        Err(TrafficError::CoverageManifestSubjectMismatch)
    );
}

#[test]
fn effective_occupancy_cannot_extend_before_decision_cutoff() {
    let op = operation("demand-a", "participant-a", 5, 7);
    let item = occupancy(OccupancySpec {
        id: "use-a",
        resource_id: "crossing-zone-1",
        route_revision: "route-a",
        operation: op.clone(),
        direction_id: None,
        early_entry_ticks: 4,
        late_clearance_ticks: 0,
        time_basis_id: "synthetic-ticks-v1",
    });
    let mut input = workload(
        std::slice::from_ref(&op),
        vec![conflict_resource(0, 0)],
        vec![item],
    );
    input.decision_tick = 2;
    assert_eq!(
        evaluate_traffic_plan(&plan(vec![op]), &input),
        Err(TrafficError::InvalidOccupancy("use-a".to_owned()))
    );
}
