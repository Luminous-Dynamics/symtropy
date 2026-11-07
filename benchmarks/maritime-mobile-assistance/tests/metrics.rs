use maritime_mobile_assistance_contract::{
    AssistanceAssignment, AssistanceKind, AssistanceTask, EvidenceReceipt, EvidenceState,
    MobileDisposition, MobileError, MobileViolation, MobileWorkload, PilotResource, RepositionLeg,
    TugResource, evaluate_mobile_assistance, evaluate_mobile_assistance_for_plan,
};
use maritime_service_chain_contract::{ServiceUnit, TickWindow};
use maritime_service_chain_fleet::{
    DemandSelection, FleetObjective, FleetPlan, OperationPlacement,
};

fn receipt(
    id: &str,
    state: EvidenceState,
    start: u64,
    end: u64,
    subject_id: &str,
    subject_revision: &str,
) -> EvidenceReceipt {
    EvidenceReceipt {
        evidence_id: id.to_owned(),
        source_id: format!("source-{id}"),
        source_revision: "source-revision-1".to_owned(),
        subject_id: subject_id.to_owned(),
        subject_revision: subject_revision.to_owned(),
        state,
        valid_from_tick: start,
        valid_through_tick: end,
        claim_ceiling: "synthetic mobile-assistance fixture only".to_owned(),
    }
}

fn tug(id: &str, node: &str, bollard_pull_kn: u32) -> TugResource {
    TugResource {
        tug_id: id.to_owned(),
        revision: "tug-config-1".to_owned(),
        position_node_id: node.to_owned(),
        capability_class_id: "harbour-assist".to_owned(),
        bollard_pull_kn,
        capability_evidence: receipt(
            &format!("{id}-capability"),
            EvidenceState::Current,
            0,
            40,
            id,
            "tug-config-1",
        ),
        position_evidence: receipt(
            &format!("{id}-position"),
            EvidenceState::Current,
            0,
            0,
            id,
            "tug-config-1",
        ),
        availability: vec![TickWindow {
            start_tick: 0,
            end_tick: 40,
        }],
    }
}

fn pilot(id: &str, node: &str, qualifications: &[&str]) -> PilotResource {
    PilotResource {
        pilot_id: id.to_owned(),
        revision: "pilot-roster-1".to_owned(),
        position_node_id: node.to_owned(),
        qualification_ids: qualifications
            .iter()
            .map(|item| (*item).to_owned())
            .collect(),
        qualification_evidence: receipt(
            &format!("{id}-qualification"),
            EvidenceState::Current,
            0,
            40,
            id,
            "pilot-roster-1",
        ),
        position_evidence: receipt(
            &format!("{id}-position"),
            EvidenceState::Current,
            0,
            0,
            id,
            "pilot-roster-1",
        ),
        availability: vec![TickWindow {
            start_tick: 0,
            end_tick: 40,
        }],
    }
}

struct TaskSpec<'a> {
    segment_id: &'a str,
    start_tick: u64,
    end_tick: u64,
    location: &'a str,
    tug_count: u8,
    required_pull: u32,
    pilot_qualification: Option<&'a str>,
}

fn task(task_id: &str, spec: TaskSpec<'_>) -> AssistanceTask {
    AssistanceTask {
        task_id: task_id.to_owned(),
        corridor_id: "synthetic-corridor".to_owned(),
        route_revision: "route-r1".to_owned(),
        operation: OperationPlacement {
            demand_id: format!("demand-{task_id}"),
            participant_id: format!("participant-{task_id}"),
            alternative_id: "alternative-a".to_owned(),
            segment_id: spec.segment_id.to_owned(),
            start_tick: spec.start_tick,
            end_tick: spec.end_tick,
            resource_uses: Vec::new(),
        },
        location_node_id: spec.location.to_owned(),
        kind: AssistanceKind::Berthing,
        required_tug_count: spec.tug_count,
        accepted_tug_class_ids: if spec.tug_count == 0 {
            Vec::new()
        } else {
            vec!["harbour-assist".to_owned()]
        },
        required_bollard_pull_kn: spec.required_pull,
        required_pilot_qualification: spec.pilot_qualification.map(str::to_owned),
    }
}

fn leg(resource_id: &str, from: &str, to: &str, duration: u64) -> RepositionLeg {
    RepositionLeg {
        resource_id: resource_id.to_owned(),
        from_node_id: from.to_owned(),
        to_node_id: to.to_owned(),
        duration_ticks: duration,
        route_revision: "route-r1".to_owned(),
        evidence: receipt(
            &format!("reposition-{resource_id}-{from}-{to}"),
            EvidenceState::Current,
            0,
            40,
            &format!("{resource_id}|{from}->{to}"),
            "route-r1",
        ),
    }
}

fn fleet_plan(input: &MobileWorkload) -> FleetPlan {
    let mut selections = std::collections::BTreeMap::new();
    for task in &input.tasks {
        selections
            .entry(task.operation.demand_id.clone())
            .or_insert_with(|| DemandSelection {
                demand_id: task.operation.demand_id.clone(),
                participant_id: task.operation.participant_id.clone(),
                alternative_id: task.operation.alternative_id.clone(),
                delivery_tick: task.operation.end_tick,
                service_amount: 1,
                service_unit: ServiceUnit::Tonnes,
            });
    }

    FleetPlan {
        selections: selections.into_values().collect(),
        operations: input
            .tasks
            .iter()
            .map(|task| task.operation.clone())
            .collect(),
        objective: FleetObjective {
            makespan_tick: input
                .tasks
                .iter()
                .map(|task| task.operation.end_tick)
                .max()
                .unwrap_or(0),
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

fn workload() -> MobileWorkload {
    MobileWorkload {
        corridor_id: "synthetic-corridor".to_owned(),
        time_basis_id: "synthetic-ticks-v1".to_owned(),
        decision_tick: 0,
        tasks: vec![
            task(
                "task-a",
                TaskSpec {
                    segment_id: "segment-a",
                    start_tick: 5,
                    end_tick: 7,
                    location: "pilot-point",
                    tug_count: 1,
                    required_pull: 30,
                    pilot_qualification: Some("container"),
                },
            ),
            task(
                "task-b",
                TaskSpec {
                    segment_id: "segment-b",
                    start_tick: 12,
                    end_tick: 14,
                    location: "berth-1",
                    tug_count: 1,
                    required_pull: 30,
                    pilot_qualification: Some("container"),
                },
            ),
        ],
        tugs: vec![tug("tug-01", "anchorage", 40)],
        pilots: vec![pilot("pilot-01", "pilot-point", &["container"])],
        reposition_legs: vec![
            leg("tug-01", "anchorage", "pilot-point", 2),
            leg("tug-01", "pilot-point", "berth-1", 4),
            leg("pilot-01", "pilot-point", "berth-1", 1),
        ],
        assignments: vec![
            AssistanceAssignment {
                task_id: "task-a".to_owned(),
                tug_ids: vec!["tug-01".to_owned()],
                pilot_id: Some("pilot-01".to_owned()),
            },
            AssistanceAssignment {
                task_id: "task-b".to_owned(),
                tug_ids: vec!["tug-01".to_owned()],
                pilot_id: Some("pilot-01".to_owned()),
            },
        ],
    }
}

#[test]
fn fleet_plan_binding_accepts_only_exact_selected_operation_placements() {
    let input = workload();
    let plan = fleet_plan(&input);
    let report = evaluate_mobile_assistance_for_plan(&input, &plan)
        .expect("assistance exactly binds to the fleet plan");
    assert_eq!(report.disposition, MobileDisposition::ReadyForComparison);
}

#[test]
fn fleet_plan_binding_rejects_forged_operation_window() {
    let input = workload();
    let mut plan = fleet_plan(&input);
    plan.operations[0].start_tick += 1;
    assert_eq!(
        evaluate_mobile_assistance_for_plan(&input, &plan),
        Err(MobileError::FleetOperationNotFound("task-a".to_owned()))
    );
}

#[test]
fn fleet_plan_binding_rejects_unselected_alternative() {
    let input = workload();
    let mut plan = fleet_plan(&input);
    plan.selections[0].alternative_id = "different-alternative".to_owned();
    assert_eq!(
        evaluate_mobile_assistance_for_plan(&input, &plan),
        Err(MobileError::FleetSelectionMismatch(
            "demand-task-a".to_owned()
        ))
    );
}

#[test]
fn fleet_plan_binding_rejects_missing_demand_selection() {
    let input = workload();
    let mut plan = fleet_plan(&input);
    plan.selections
        .retain(|selection| selection.demand_id != "demand-task-a");
    assert_eq!(
        evaluate_mobile_assistance_for_plan(&input, &plan),
        Err(MobileError::FleetSelectionMismatch("demand-task-a".to_owned()))
    );
}

#[test]
fn named_tug_and_pilot_are_feasible_when_capability_availability_and_repositioning_fit() {
    let report = evaluate_mobile_assistance(&workload()).expect("valid workload");
    assert_eq!(report.disposition, MobileDisposition::ReadyForComparison);
    assert!(report.violations.is_empty());
    assert_eq!(report.tasks_checked, 2);
    assert_eq!(report.assignments_checked, 2);
}

#[test]
fn tug_and_pilot_use_distinct_reposition_durations_for_the_same_waypoint_pair() {
    let input = workload();
    let tug_leg = input
        .reposition_legs
        .iter()
        .find(|item| item.resource_id == "tug-01" && item.from_node_id == "pilot-point")
        .expect("tug reposition leg");
    let pilot_leg = input
        .reposition_legs
        .iter()
        .find(|item| item.resource_id == "pilot-01" && item.from_node_id == "pilot-point")
        .expect("pilot reposition leg");
    assert_eq!(tug_leg.duration_ticks, 4);
    assert_eq!(pilot_leg.duration_ticks, 1);

    let report = evaluate_mobile_assistance(&input).expect("resource-specific movement durations");
    assert_eq!(report.disposition, MobileDisposition::ReadyForComparison);
}

#[test]
fn tug_reposition_evidence_cannot_be_reused_for_a_pilot() {
    let mut input = workload();
    input
        .reposition_legs
        .retain(|item| item.resource_id != "pilot-01");
    let report = evaluate_mobile_assistance(&input).expect("missing pilot-specific leg");
    assert_eq!(report.disposition, MobileDisposition::NeedsEvidence);
    assert!(
        report
            .violations
            .contains(&MobileViolation::MissingRepositionLeg {
                resource_id: "pilot-01".to_owned(),
                from_node_id: "pilot-point".to_owned(),
                to_node_id: "berth-1".to_owned(),
            })
    );
}

#[test]
fn one_named_tug_cannot_serve_overlapping_assistance_tasks() {
    let mut input = workload();
    input.tasks[1].operation.start_tick = 6;
    input.tasks[1].operation.end_tick = 8;
    let report = evaluate_mobile_assistance(&input).expect("valid workload");
    assert_eq!(report.disposition, MobileDisposition::Infeasible);
    assert!(
        report
            .violations
            .contains(&MobileViolation::ResourceOverlap("tug-01".to_owned()))
    );
}

#[test]
fn reposition_time_must_fit_after_the_previous_service_finishes() {
    let mut input = workload();
    input.tasks[1].operation.start_tick = 9;
    input.tasks[1].operation.end_tick = 11;
    let report = evaluate_mobile_assistance(&input).expect("valid workload");
    assert_eq!(report.disposition, MobileDisposition::Infeasible);
    assert!(
        report
            .violations
            .contains(&MobileViolation::RepositionTimeInsufficient(
                "tug-01".to_owned()
            ))
    );
}

#[test]
fn insufficient_bollard_pull_is_infeasible_not_an_optimizer_penalty() {
    let mut input = workload();
    input.tugs[0].bollard_pull_kn = 20;
    let report = evaluate_mobile_assistance(&input).expect("valid workload");
    assert_eq!(report.disposition, MobileDisposition::Infeasible);
    assert!(
        report
            .violations
            .contains(&MobileViolation::InsufficientBollardPull(
                "task-a".to_owned()
            ))
    );
}

#[test]
fn joint_tug_requirement_needs_distinct_named_units() {
    let mut input = workload();
    input.tasks[0].required_tug_count = 2;
    input.tugs.push(tug("tug-02", "anchorage", 20));
    let report = evaluate_mobile_assistance(&input).expect("valid workload");
    assert_eq!(report.disposition, MobileDisposition::Infeasible);
    assert!(
        report
            .violations
            .contains(&MobileViolation::InsufficientTugCount("task-a".to_owned()))
    );
}

#[test]
fn wrong_tug_class_is_rejected_even_when_bollard_pull_is_sufficient() {
    let mut input = workload();
    input.tugs[0].capability_class_id = "escort-only".to_owned();
    let report = evaluate_mobile_assistance(&input).expect("valid workload");
    assert_eq!(report.disposition, MobileDisposition::Infeasible);
    assert!(
        report
            .violations
            .contains(&MobileViolation::TugClassMismatch("task-a".to_owned()))
    );
}

#[test]
fn unqualified_pilot_is_rejected() {
    let mut input = workload();
    input.pilots[0].qualification_ids = vec!["tanker".to_owned()];
    let report = evaluate_mobile_assistance(&input).expect("valid workload");
    assert_eq!(report.disposition, MobileDisposition::Infeasible);
    assert!(
        report
            .violations
            .contains(&MobileViolation::PilotQualificationMismatch(
                "task-a".to_owned()
            ))
    );
}

#[test]
fn required_pilot_cannot_be_silently_omitted() {
    let mut input = workload();
    input.assignments[0].pilot_id = None;
    let report = evaluate_mobile_assistance(&input).expect("valid workload");
    assert_eq!(report.disposition, MobileDisposition::Infeasible);
    assert!(
        report
            .violations
            .contains(&MobileViolation::MissingPilotAssignment(
                "task-a".to_owned()
            ))
    );
}

#[test]
fn missing_reposition_evidence_abstains_instead_of_assuming_zero_travel() {
    let mut input = workload();
    input.reposition_legs.remove(1);
    let report = evaluate_mobile_assistance(&input).expect("valid workload");
    assert_eq!(report.disposition, MobileDisposition::NeedsEvidence);
    assert!(
        report
            .violations
            .contains(&MobileViolation::MissingRepositionLeg {
                resource_id: "tug-01".to_owned(),
                from_node_id: "pilot-point".to_owned(),
                to_node_id: "berth-1".to_owned(),
            })
    );
}

#[test]
fn stale_capability_evidence_needs_review() {
    let mut input = workload();
    input.tugs[0].capability_evidence.state = EvidenceState::Stale;
    let report = evaluate_mobile_assistance(&input).expect("valid workload");
    assert_eq!(report.disposition, MobileDisposition::NeedsEvidence);
    assert!(
        report
            .violations
            .contains(&MobileViolation::EvidenceStateNotCurrent {
                resource_id: "tug-01".to_owned(),
                evidence_id: "tug-01-capability".to_owned(),
                state: EvidenceState::Stale,
            })
    );
}

#[test]
fn revoked_position_evidence_is_unsupported() {
    let mut input = workload();
    input.tugs[0].position_evidence.state = EvidenceState::Revoked;
    let report = evaluate_mobile_assistance(&input).expect("valid workload");
    assert_eq!(report.disposition, MobileDisposition::Unsupported);
}

#[test]
fn reposition_data_must_match_the_task_route_revision() {
    let mut input = workload();
    input.reposition_legs[1].route_revision = "route-old".to_owned();
    input.reposition_legs[1].evidence.subject_revision = "route-old".to_owned();
    let report = evaluate_mobile_assistance(&input).expect("valid workload");
    assert_eq!(report.disposition, MobileDisposition::NeedsEvidence);
    assert!(
        report
            .violations
            .contains(&MobileViolation::RepositionRouteRevisionMismatch(
                "tug-01".to_owned()
            ))
    );
}

#[test]
fn half_open_service_handoff_is_allowed_when_the_resource_stays_at_the_same_node() {
    let mut input = workload();
    input.tasks[1].operation.start_tick = 7;
    input.tasks[1].operation.end_tick = 9;
    input.tasks[1].location_node_id = "pilot-point".to_owned();
    input
        .reposition_legs
        .retain(|item| item.from_node_id != "pilot-point" || item.to_node_id != "berth-1");
    let report = evaluate_mobile_assistance(&input).expect("valid workload");
    assert_eq!(report.disposition, MobileDisposition::ReadyForComparison);
}

#[test]
fn resource_availability_must_cover_the_operation_window() {
    let mut input = workload();
    input.tugs[0].availability = vec![TickWindow {
        start_tick: 0,
        end_tick: 10,
    }];
    let report = evaluate_mobile_assistance(&input).expect("valid workload");
    assert_eq!(report.disposition, MobileDisposition::Infeasible);
    assert!(
        report
            .violations
            .contains(&MobileViolation::ResourceUnavailable("tug-01".to_owned()))
    );
}

#[test]
fn duplicate_task_assignments_are_structural_errors() {
    let mut input = workload();
    input.assignments.push(input.assignments[0].clone());
    assert_eq!(
        evaluate_mobile_assistance(&input),
        Err(MobileError::DuplicateAssignment("task-a".to_owned()))
    );
}

#[test]
fn equivalent_input_permutations_produce_identical_reports() {
    let original = workload();
    let expected = evaluate_mobile_assistance(&original).expect("valid workload");

    let mut permuted = original;
    permuted.tasks.reverse();
    permuted.tugs.reverse();
    permuted.pilots.reverse();
    permuted.reposition_legs.reverse();
    permuted.assignments.reverse();

    assert_eq!(
        evaluate_mobile_assistance(&permuted).expect("permuted workload"),
        expected
    );
}

#[test]
fn capability_evidence_must_bind_to_the_exact_tug_identity_and_revision() {
    let mut input = workload();
    input.tugs[0].capability_evidence.subject_id = "another-tug".to_owned();
    assert_eq!(
        evaluate_mobile_assistance(&input),
        Err(MobileError::InvalidEvidenceReceipt(
            "tug-01-capability".to_owned()
        ))
    );
}

#[test]
fn reposition_evidence_must_bind_to_the_exact_directed_route_leg() {
    let mut input = workload();
    input.reposition_legs[0].evidence.subject_id = "anchorage->different-node".to_owned();
    assert_eq!(
        evaluate_mobile_assistance(&input),
        Err(MobileError::InvalidEvidenceReceipt(
            "reposition-tug-01-anchorage-pilot-point".to_owned()
        ))
    );
}

#[test]
fn unsupported_reposition_evidence_does_not_become_a_ready_assignment() {
    let mut input = workload();
    input.reposition_legs[0].evidence.state = EvidenceState::Unsupported;
    let report = evaluate_mobile_assistance(&input).expect("valid workload");
    assert_eq!(report.disposition, MobileDisposition::Unsupported);
}
