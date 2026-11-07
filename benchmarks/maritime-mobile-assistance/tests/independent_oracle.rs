//! Independent direct checks of tug scheduling for fixed assistance tasks.
//!
//! The reference path sorts tasks and checks each resource use directly. It does
//! not call the production sequence or reposition helpers.

use maritime_mobile_assistance_contract::{
    AssistanceAssignment, AssistanceKind, AssistanceTask, EvidenceReceipt, EvidenceState,
    MobileDisposition, MobileWorkload, RepositionLeg, TugResource, evaluate_mobile_assistance,
};
use maritime_service_chain_contract::TickWindow;
use maritime_service_chain_fleet::OperationPlacement;

fn evidence(id: &str, subject_id: &str, subject_revision: &str) -> EvidenceReceipt {
    EvidenceReceipt {
        evidence_id: id.to_owned(),
        source_id: format!("source-{id}"),
        source_revision: "source-r1".to_owned(),
        subject_id: subject_id.to_owned(),
        subject_revision: subject_revision.to_owned(),
        state: EvidenceState::Current,
        valid_from_tick: 0,
        valid_through_tick: 100,
        claim_ceiling: "independent synthetic oracle fixture".to_owned(),
    }
}

fn tug() -> TugResource {
    TugResource {
        tug_id: "tug-1".to_owned(),
        revision: "tug-r1".to_owned(),
        position_node_id: "origin".to_owned(),
        capability_class_id: "harbour-assist".to_owned(),
        bollard_pull_kn: 100,
        capability_evidence: evidence("tug-capability", "tug-1", "tug-r1"),
        position_evidence: evidence("tug-position", "tug-1", "tug-r1"),
        availability: vec![TickWindow {
            start_tick: 0,
            end_tick: 100,
        }],
    }
}

fn operation(task_id: &str, segment: &str, start: u64) -> OperationPlacement {
    OperationPlacement {
        demand_id: format!("demand-{task_id}"),
        participant_id: format!("participant-{task_id}"),
        alternative_id: "alternative-1".to_owned(),
        segment_id: segment.to_owned(),
        start_tick: start,
        end_tick: start + 2,
        resource_uses: Vec::new(),
    }
}

fn task(task_id: &str, segment: &str, start: u64, location: &str) -> AssistanceTask {
    AssistanceTask {
        task_id: task_id.to_owned(),
        corridor_id: "corridor-1".to_owned(),
        route_revision: "route-r1".to_owned(),
        operation: operation(task_id, segment, start),
        location_node_id: location.to_owned(),
        kind: AssistanceKind::Escort,
        required_tug_count: 1,
        accepted_tug_class_ids: vec!["harbour-assist".to_owned()],
        required_bollard_pull_kn: 30,
        required_pilot_qualification: None,
    }
}

fn reposition(
    resource_id: &str,
    from: &str,
    to: &str,
    duration: u64,
    id: &str,
) -> RepositionLeg {
    RepositionLeg {
        resource_id: resource_id.to_owned(),
        from_node_id: from.to_owned(),
        to_node_id: to.to_owned(),
        duration_ticks: duration,
        route_revision: "route-r1".to_owned(),
        evidence: evidence(
            id,
            &format!("{resource_id}|{from}->{to}"),
            "route-r1",
        ),
    }
}

fn workload(first_start: u64, gap_after_first: u64, reposition_ticks: u64) -> MobileWorkload {
    let first_end = first_start + 2;
    let second_start = first_end + gap_after_first;
    MobileWorkload {
        corridor_id: "corridor-1".to_owned(),
        time_basis_id: "ticks-r1".to_owned(),
        decision_tick: 0,
        tasks: vec![
            task("task-a", "segment-a", first_start, "node-a"),
            task("task-b", "segment-b", second_start, "node-b"),
        ],
        tugs: vec![tug()],
        pilots: Vec::new(),
        reposition_legs: vec![
            reposition("tug-1", "origin", "node-a", 1, "leg-origin-a"),
            reposition("tug-1", "node-a", "node-b", reposition_ticks, "leg-a-b"),
        ],
        assignments: vec![
            AssistanceAssignment {
                task_id: "task-a".to_owned(),
                tug_ids: vec!["tug-1".to_owned()],
                pilot_id: None,
            },
            AssistanceAssignment {
                task_id: "task-b".to_owned(),
                tug_ids: vec!["tug-1".to_owned()],
                pilot_id: None,
            },
        ],
    }
}

/// Independent direct feasibility calculation for this intentionally narrow
/// one-tug / two-task corpus. Reposition is scheduled just in time before each
/// task; availability and evidence windows are checked without production code.
fn reference_one_tug_two_task_feasible(input: &MobileWorkload) -> bool {
    let Some(tug) = input.tugs.iter().find(|item| item.tug_id == "tug-1") else {
        return false;
    };
    if tug.capability_evidence.state != EvidenceState::Current
        || tug.position_evidence.state != EvidenceState::Current
        || tug.capability_evidence.subject_id != tug.tug_id
        || tug.position_evidence.subject_id != tug.tug_id
        || tug.capability_evidence.subject_revision != tug.revision
        || tug.position_evidence.subject_revision != tug.revision
        || tug.position_evidence.valid_from_tick > input.decision_tick
        || tug.position_evidence.valid_through_tick < input.decision_tick
    {
        return false;
    }

    let mut tasks: Vec<&AssistanceTask> = input.tasks.iter().collect();
    tasks.sort_by_key(|item| item.operation.start_tick);
    let mut node = tug.position_node_id.as_str();
    let mut ready = input.decision_tick;

    for task in tasks {
        if task.required_tug_count != 1
            || !task
                .accepted_tug_class_ids
                .iter()
                .any(|class| class == &tug.capability_class_id)
            || task.required_bollard_pull_kn > tug.bollard_pull_kn
            || tug.capability_evidence.state != EvidenceState::Current
            || tug.capability_evidence.valid_from_tick > task.operation.start_tick
            || tug.capability_evidence.valid_through_tick < task.operation.end_tick
        {
            return false;
        }
        if task.operation.start_tick < ready {
            return false;
        }
        if !tug.availability.iter().any(|window| {
            window.start_tick <= task.operation.start_tick
                && window.end_tick >= task.operation.end_tick
        }) {
            return false;
        }

        if node != task.location_node_id {
            let expected_subject = format!("{}|{}->{}", tug.tug_id, node, task.location_node_id);
            let Some(leg) = input.reposition_legs.iter().find(|leg| {
                leg.from_node_id == node
                    && leg.to_node_id == task.location_node_id
                    && leg.route_revision == task.route_revision
                    && leg.evidence.subject_id == expected_subject
                    && leg.evidence.subject_revision == task.route_revision
            }) else {
                return false;
            };
            let Some(departure) = task.operation.start_tick.checked_sub(leg.duration_ticks) else {
                return false;
            };
            if departure < ready {
                return false;
            }
            let arrival = departure + leg.duration_ticks;
            if arrival > task.operation.start_tick
                || leg.evidence.state != EvidenceState::Current
                || leg.evidence.valid_from_tick > departure
                || leg.evidence.valid_through_tick < arrival
                || !tug
                    .availability
                    .iter()
                    .any(|window| window.start_tick <= departure && window.end_tick >= arrival)
            {
                return false;
            }
        }

        node = task.location_node_id.as_str();
        ready = task.operation.end_tick;
    }
    true
}

#[test]
fn production_report_matches_independent_direct_reference_over_frozen_timing_sweep() {
    let mut comparisons = 0;
    for first_start in 1..=8 {
        for gap in 0..=7 {
            for reposition_ticks in 1..=8 {
                let input = workload(first_start, gap, reposition_ticks);
                let expected = reference_one_tug_two_task_feasible(&input);
                let actual = evaluate_mobile_assistance(&input).expect("valid synthetic workload");
                assert_eq!(
                    actual.disposition == MobileDisposition::ReadyForComparison,
                    expected,
                    "mismatch: first_start={first_start}, gap={gap}, reposition={reposition_ticks}"
                );
                comparisons += 1;
            }
        }
    }
    assert_eq!(comparisons, 512);
}
