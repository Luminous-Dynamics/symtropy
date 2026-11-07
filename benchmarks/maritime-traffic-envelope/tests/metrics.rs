use maritime_service_chain_contract::{ServiceUnit, TickWindow};
use maritime_service_chain_fleet::{
    DemandSelection, FleetObjective, FleetPlan, OperationPlacement,
};
use maritime_traffic_envelope_contract::{
    EvidenceReceipt, EvidenceState, TrafficDisposition, TrafficError, TrafficOccupancy,
    TrafficReport, TrafficResource, TrafficResourceKind, TrafficViolation, TrafficWindow,
    TrafficWorkload, canonical_occupancy_subject_id, canonical_resource_subject_id,
    evaluate_traffic_plan,
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
    item.evidence.subject_id =
        canonical_occupancy_subject_id(&item, spec.time_basis_id);
    item
}

fn workload(
    resources: Vec<TrafficResource>,
    occupancies: Vec<TrafficOccupancy>,
) -> TrafficWorkload {
    TrafficWorkload {
        corridor_id: "synthetic-corridor".to_owned(),
        time_basis_id: "synthetic-ticks-v1".to_owned(),
        decision_tick: 0,
        resources,
        occupancies,
    }
}

fn evaluate(
    operations: Vec<OperationPlacement>,
    resources: Vec<TrafficResource>,
    occupancies: Vec<TrafficOccupancy>,
) -> Result<TrafficReport, TrafficError> {
    evaluate_traffic_plan(&plan(operations), &workload(resources, occupancies))
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
            &workload(vec![conflict_resource(0, 0)], vec![item])
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
    let original = workload(vec![conflict_resource(0, 0)], occupancies);
    let expected = evaluate_traffic_plan(&plan(vec![first.clone(), second.clone()]), &original)
        .expect("valid workload");

    let mut permuted = original;
    permuted.resources.reverse();
    permuted.occupancies.reverse();
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
    let mut input = workload(vec![conflict_resource(0, 0)], vec![item]);
    input.decision_tick = 2;
    assert_eq!(
        evaluate_traffic_plan(&plan(vec![op]), &input),
        Err(TrafficError::InvalidOccupancy("use-a".to_owned()))
    );
}
