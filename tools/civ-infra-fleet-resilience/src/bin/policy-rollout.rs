use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::{env, fs, process};

const EMBEDDED_FIXTURE: &str = include_str!(
    "../../../../docs/research/civ-infra-resilience/fixtures/policy-rollout-microcase-v0.1.json"
);

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    schema_version: String,
    fixture_id: String,
    classification: String,
    claim_ceiling: String,
    horizon_ticks: u32,
    resources: Vec<ResourceSpec>,
    evidence_vocabulary: Vec<String>,
    tasks: Vec<TaskSpec>,
    disturbance: DisturbanceSpec,
    required_invariants: Vec<String>,
    expected_outcomes: Vec<ExpectedOutcome>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResourceSpec {
    resource_id: String,
    capacity: u32,
    resource_kind: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TaskSpec {
    task_id: String,
    duration_ticks: u32,
    requires: Vec<String>,
    resource_alternatives: Vec<String>,
    acceptance_evidence: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DisturbanceSpec {
    event_id: String,
    injection_tick: u32,
    observable_tick: u32,
    affected_resource: String,
    available_again_tick: u32,
    interrupted_work_is_lost: bool,
    description: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct ExpectedOutcome {
    policy_id: String,
    status: String,
    accepted_synthetic_work: bool,
    makespan_ticks: Option<u32>,
    completed_task_count: usize,
    interrupted_attempt_count: usize,
    resource_reassignments: usize,
    replan_tick: Option<u32>,
}

#[derive(Clone, Debug, Serialize)]
struct RolloutReport {
    schema_version: String,
    fixture_id: String,
    input_sha256: String,
    classification: String,
    validation_status: String,
    benchmark_executed: bool,
    simulated_cases: usize,
    policy_runs_performed: usize,
    event_id: String,
    event_injection_tick: u32,
    event_observable_tick: u32,
    counts: Counts,
    policy_results: Vec<PolicyResult>,
    all_expected_outcomes_matched: bool,
    claim_ceiling: String,
    explicit_limitations: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct Counts {
    resources: usize,
    tasks: usize,
    acceptance_evidence_vocabulary_items: usize,
    required_invariants: usize,
}

#[derive(Clone, Debug, Serialize)]
struct PolicyResult {
    policy_id: String,
    policy_name: String,
    status: String,
    accepted_synthetic_work: bool,
    makespan_ticks: Option<u32>,
    completed_task_count: usize,
    interrupted_attempt_count: usize,
    resource_reassignments: usize,
    event_observed_tick: Option<u32>,
    replan_tick: Option<u32>,
    knowledge_leak_detected: bool,
    final_resource_assignments: Vec<ResourceAssignment>,
}

#[derive(Clone, Debug, Serialize)]
struct ResourceAssignment {
    task_id: String,
    resource_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Policy {
    B0,
    B1,
    B2,
}

impl Policy {
    const ALL: [Self; 3] = [Self::B0, Self::B1, Self::B2];

    fn id(self) -> &'static str {
        match self {
            Self::B0 => "B0",
            Self::B1 => "B1",
            Self::B2 => "B2",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::B0 => "protective-stop baseline",
            Self::B1 => "local repair-and-retry",
            Self::B2 => "observation-gated replan",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TaskState {
    Pending,
    Running,
    Complete,
}

#[derive(Clone, Debug)]
struct TaskRuntime {
    spec: TaskSpec,
    state: TaskState,
    assigned_resource: String,
    remaining_ticks: u32,
    interrupted_attempt_count: usize,
    completed_at: Option<u32>,
}

fn main() {
    let mut args = env::args_os().skip(1);
    let input_bytes = match args.next() {
        Some(path) => match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                eprintln!("cannot read fixture {}: {error}", path.to_string_lossy());
                process::exit(2);
            }
        },
        None => EMBEDDED_FIXTURE.as_bytes().to_vec(),
    };

    if args.next().is_some() {
        eprintln!("usage: policy-rollout [fixture.json]");
        process::exit(2);
    }

    match verify_fixture_bytes(&input_bytes) {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("cannot serialize rollout report: {error}");
                process::exit(2);
            }
        },
        Err(errors) => {
            for error in errors {
                eprintln!("rollout rejected: {error}");
            }
            process::exit(2);
        }
    }
}

fn verify_fixture_bytes(input_bytes: &[u8]) -> Result<RolloutReport, Vec<String>> {
    let fixture: Fixture = serde_json::from_slice(input_bytes).map_err(|error| {
        vec![format!(
            "fixture JSON does not match the strict policy-rollout schema: {error}"
        )]
    })?;

    let mut errors = validate_fixture(&fixture);
    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let policy_results: Vec<PolicyResult> = Policy::ALL
        .into_iter()
        .map(|policy| run_policy(&fixture, policy))
        .collect();

    for expected in &fixture.expected_outcomes {
        match policy_results
            .iter()
            .find(|actual| actual.policy_id == expected.policy_id)
        {
            Some(actual) if matches_expected(actual, expected) => {}
            Some(actual) => errors.push(format!(
                "policy {} differs from frozen outcome: expected {:?}, got {:?}",
                expected.policy_id,
                expected,
                expected_projection(actual)
            )),
            None => errors.push(format!(
                "expected policy {} produced no result",
                expected.policy_id
            )),
        }
    }
    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let digest = Sha256::digest(input_bytes);
    Ok(RolloutReport {
        schema_version: "luminous.civ_infra.policy_rollout_report.v0.1".to_string(),
        fixture_id: fixture.fixture_id.clone(),
        input_sha256: format!("{digest:x}"),
        classification: fixture.classification.clone(),
        validation_status: "single_synthetic_rollout_and_frozen_outcomes_checked".to_string(),
        benchmark_executed: true,
        simulated_cases: 1,
        policy_runs_performed: policy_results.len(),
        event_id: fixture.disturbance.event_id.clone(),
        event_injection_tick: fixture.disturbance.injection_tick,
        event_observable_tick: fixture.disturbance.observable_tick,
        counts: Counts {
            resources: fixture.resources.len(),
            tasks: fixture.tasks.len(),
            acceptance_evidence_vocabulary_items: fixture.evidence_vocabulary.len(),
            required_invariants: fixture.required_invariants.len(),
        },
        policy_results,
        all_expected_outcomes_matched: true,
        claim_ceiling: fixture.claim_ceiling.clone(),
        explicit_limitations: vec![
            "This executes one hand-authored discrete-tick scenario, not the 13-case disturbance corpus.".to_string(),
            "Task durations, repair timing, resource compatibility, and discarded work are synthetic inputs, not measured physical behavior.".to_string(),
            "Synthetic completion tokens do not establish real inspection evidence, safety, engineering conformity, or construction acceptance.".to_string(),
            "B2 is a bounded alternative-selection rule, not a general-purpose planner or dispatch controller.".to_string(),
        ],
    })
}

fn validate_fixture(fixture: &Fixture) -> Vec<String> {
    let mut errors = Vec::new();
    if fixture.schema_version != "civ-infra-policy-rollout/0.1" {
        errors.push("unsupported schema_version".to_string());
    }
    if fixture.fixture_id.trim().is_empty() || fixture.claim_ceiling.trim().is_empty() {
        errors.push("fixture_id and claim_ceiling must be explicit".to_string());
    }
    if fixture.classification != "synthetic_only" {
        errors.push("classification must remain synthetic_only".to_string());
    }
    if fixture.horizon_ticks == 0 || fixture.tasks.is_empty() {
        errors.push("horizon_ticks and task list must be non-empty".to_string());
    }
    if fixture.resources.is_empty() || fixture.evidence_vocabulary.is_empty() {
        errors.push("resources and evidence_vocabulary must be non-empty".to_string());
    }
    if fixture.disturbance.event_id.trim().is_empty()
        || fixture.disturbance.description.trim().is_empty()
    {
        errors.push("disturbance event_id and description must be explicit".to_string());
    }
    if fixture.disturbance.observable_tick < fixture.disturbance.injection_tick {
        errors.push("disturbance is observable before injection".to_string());
    }
    if fixture.disturbance.injection_tick >= fixture.horizon_ticks
        || fixture.disturbance.observable_tick >= fixture.horizon_ticks
        || fixture.disturbance.available_again_tick > fixture.horizon_ticks
        || fixture.disturbance.available_again_tick <= fixture.disturbance.injection_tick
    {
        errors.push(
            "disturbance ticks must fit the declared horizon and recovery interval".to_string(),
        );
    }
    if !fixture.disturbance.interrupted_work_is_lost {
        errors.push(
            "this microcase requires interrupted work to be explicitly discarded".to_string(),
        );
    }
    if fixture.required_invariants.is_empty() {
        errors.push("required_invariants must not be empty".to_string());
    }

    check_unique(
        fixture
            .resources
            .iter()
            .map(|item| item.resource_id.as_str()),
        "resource_id",
        &mut errors,
    );
    check_unique(
        fixture.tasks.iter().map(|item| item.task_id.as_str()),
        "task_id",
        &mut errors,
    );
    check_unique(
        fixture.evidence_vocabulary.iter().map(String::as_str),
        "evidence vocabulary item",
        &mut errors,
    );
    check_unique(
        fixture
            .expected_outcomes
            .iter()
            .map(|item| item.policy_id.as_str()),
        "expected policy_id",
        &mut errors,
    );

    let resource_ids: BTreeSet<&str> = fixture
        .resources
        .iter()
        .map(|resource| resource.resource_id.as_str())
        .collect();
    let task_ids: BTreeSet<&str> = fixture
        .tasks
        .iter()
        .map(|task| task.task_id.as_str())
        .collect();
    let evidence_ids: BTreeSet<&str> = fixture
        .evidence_vocabulary
        .iter()
        .map(String::as_str)
        .collect();

    for resource in &fixture.resources {
        if resource.resource_id.trim().is_empty() || resource.resource_kind.trim().is_empty() {
            errors.push("resource ID and kind must not be empty".to_string());
        }
        if resource.capacity != 1 {
            errors.push(format!(
                "resource {} must have capacity 1 in this microcase",
                resource.resource_id
            ));
        }
    }
    if !resource_ids.contains(fixture.disturbance.affected_resource.as_str()) {
        errors.push("disturbance references an unknown affected resource".to_string());
    }

    for task in &fixture.tasks {
        if task.task_id.trim().is_empty() || task.duration_ticks == 0 {
            errors.push("task IDs and durations must be non-empty and positive".to_string());
        }
        if task.resource_alternatives.is_empty() {
            errors.push(format!("task {} has no resource alternative", task.task_id));
        }
        if task.acceptance_evidence.is_empty() {
            errors.push(format!(
                "task {} has no acceptance evidence token",
                task.task_id
            ));
        }
        check_unique(
            task.requires.iter().map(String::as_str),
            &format!("dependency for {}", task.task_id),
            &mut errors,
        );
        check_unique(
            task.resource_alternatives.iter().map(String::as_str),
            &format!("resource alternative for {}", task.task_id),
            &mut errors,
        );
        for dependency in &task.requires {
            if !task_ids.contains(dependency.as_str()) {
                errors.push(format!(
                    "task {} references missing dependency {}",
                    task.task_id, dependency
                ));
            }
            if dependency == &task.task_id {
                errors.push(format!("task {} depends on itself", task.task_id));
            }
        }
        for resource_id in &task.resource_alternatives {
            if !resource_ids.contains(resource_id.as_str()) {
                errors.push(format!(
                    "task {} references unknown resource {}",
                    task.task_id, resource_id
                ));
            }
        }
        for evidence in &task.acceptance_evidence {
            if !evidence_ids.contains(evidence.as_str()) {
                errors.push(format!(
                    "task {} references unknown acceptance evidence {}",
                    task.task_id, evidence
                ));
            }
        }
    }

    let mut resolved = BTreeSet::<String>::new();
    loop {
        let previous_count = resolved.len();
        for task in &fixture.tasks {
            if task
                .requires
                .iter()
                .all(|dependency| resolved.contains(dependency))
            {
                resolved.insert(task.task_id.clone());
            }
        }
        if resolved.len() == previous_count {
            break;
        }
    }
    if resolved.len() != fixture.tasks.len() {
        errors.push("task dependency graph contains a cycle or unresolved dependency".to_string());
    }

    let actual_policies: BTreeSet<&str> = fixture
        .expected_outcomes
        .iter()
        .map(|outcome| outcome.policy_id.as_str())
        .collect();
    for policy_id in ["B0", "B1", "B2"] {
        if !actual_policies.contains(policy_id) {
            errors.push(format!("missing frozen expected outcome for {policy_id}"));
        }
    }
    if fixture.expected_outcomes.len() != 3 {
        errors.push("microcase must freeze exactly the B0, B1 and B2 outcomes".to_string());
    }
    if fixture.tasks.last().map(|task| task.task_id.as_str()) != Some("inspect-assembly") {
        errors.push("final task must be inspect-assembly for this frozen microcase".to_string());
    }

    for required in [
        "policy_replanning_never_precedes_observation",
        "interrupted_attempt_progress_is_discarded",
        "accepted_synthetic_work_requires_every_task_and_final_inspection",
        "capacity_one_resources_are_never_double_booked",
        "synthetic_completion_does_not_claim_physical_qualification",
    ] {
        if !fixture
            .required_invariants
            .iter()
            .any(|item| item.as_str() == required)
        {
            errors.push(format!("required invariant {required} is absent"));
        }
    }
    errors
}

fn check_unique<'a>(values: impl Iterator<Item = &'a str>, label: &str, errors: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            errors.push(format!("duplicate {label}: {value}"));
        }
    }
}

fn run_policy(fixture: &Fixture, policy: Policy) -> PolicyResult {
    let mut runtime: Vec<TaskRuntime> = fixture
        .tasks
        .iter()
        .cloned()
        .map(|spec| TaskRuntime {
            assigned_resource: spec.resource_alternatives[0].clone(),
            remaining_ticks: spec.duration_ticks,
            spec,
            state: TaskState::Pending,
            interrupted_attempt_count: 0,
            completed_at: None,
        })
        .collect();

    let mut interrupted_attempt_count = 0usize;
    let mut resource_reassignments = 0usize;
    let mut event_observed_tick = None;
    let mut replan_tick = None;
    let mut stopped_by_protective_interlock = false;
    let mut replan_attempted = false;

    for tick in 0..fixture.horizon_ticks {
        if tick == fixture.disturbance.injection_tick {
            for task in &mut runtime {
                if task.state == TaskState::Running
                    && task.assigned_resource == fixture.disturbance.affected_resource
                {
                    task.state = TaskState::Pending;
                    task.remaining_ticks = task.spec.duration_ticks;
                    task.interrupted_attempt_count += 1;
                    interrupted_attempt_count += 1;
                }
            }

            // The equipment-level protective interlock may stop work without
            // exposing the cause to the planning policy before observable_tick.
            if policy == Policy::B0 {
                stopped_by_protective_interlock = true;
                break;
            }
        }

        if tick >= fixture.disturbance.observable_tick && event_observed_tick.is_none() {
            event_observed_tick = Some(tick);
        }

        if policy == Policy::B2 && event_observed_tick.is_some() && !replan_attempted {
            replan_attempted = true;
            let pending_indices: Vec<usize> = runtime
                .iter()
                .enumerate()
                .filter(|(_, task)| {
                    task.state == TaskState::Pending
                        && task.assigned_resource == fixture.disturbance.affected_resource
                })
                .map(|(index, _)| index)
                .collect();

            for index in pending_indices {
                let alternative = runtime[index]
                    .spec
                    .resource_alternatives
                    .iter()
                    .find(|candidate| {
                        candidate.as_str() != fixture.disturbance.affected_resource
                            && resource_is_available(candidate, tick, fixture)
                            && !resource_is_busy(&runtime, index, candidate)
                    })
                    .cloned();

                if let Some(resource_id) = alternative {
                    runtime[index].assigned_resource = resource_id;
                    resource_reassignments += 1;
                    replan_tick = Some(tick);
                }
            }
        }

        for index in 0..runtime.len() {
            if runtime[index].state != TaskState::Pending {
                continue;
            }
            let prerequisites_complete = runtime[index].spec.requires.iter().all(|required| {
                runtime.iter().any(|candidate| {
                    candidate.spec.task_id.as_str() == required.as_str()
                        && candidate.state == TaskState::Complete
                })
            });
            if !prerequisites_complete {
                continue;
            }

            let resource_id = runtime[index].assigned_resource.clone();
            if resource_is_available(&resource_id, tick, fixture)
                && !resource_is_busy(&runtime, index, &resource_id)
            {
                runtime[index].state = TaskState::Running;
            }
        }

        for task in &mut runtime {
            if task.state != TaskState::Running {
                continue;
            }
            task.remaining_ticks -= 1;
            if task.remaining_ticks == 0 {
                task.state = TaskState::Complete;
                task.completed_at = Some(tick + 1);
            }
        }
    }

    let completed_task_count = runtime
        .iter()
        .filter(|task| task.state == TaskState::Complete)
        .count();
    let final_inspection_complete = runtime
        .iter()
        .any(|task| task.spec.task_id == "inspect-assembly" && task.state == TaskState::Complete);
    let accepted_synthetic_work = !stopped_by_protective_interlock
        && completed_task_count == runtime.len()
        && final_inspection_complete;
    let makespan_ticks = if accepted_synthetic_work {
        runtime.iter().filter_map(|task| task.completed_at).max()
    } else {
        None
    };
    let knowledge_leak_detected = replan_tick
        .is_some_and(|decision_tick| decision_tick < fixture.disturbance.observable_tick);
    let final_resource_assignments = runtime
        .iter()
        .map(|task| ResourceAssignment {
            task_id: task.spec.task_id.clone(),
            resource_id: task.assigned_resource.clone(),
        })
        .collect();

    let status = if stopped_by_protective_interlock {
        "protective_stop_no_accepted_output"
    } else if accepted_synthetic_work {
        "synthetic_acceptance_complete"
    } else {
        "horizon_exceeded_no_accepted_output"
    };

    PolicyResult {
        policy_id: policy.id().to_string(),
        policy_name: policy.name().to_string(),
        status: status.to_string(),
        accepted_synthetic_work,
        makespan_ticks,
        completed_task_count,
        interrupted_attempt_count,
        resource_reassignments,
        event_observed_tick,
        replan_tick,
        knowledge_leak_detected,
        final_resource_assignments,
    }
}

fn resource_is_available(resource_id: &str, tick: u32, fixture: &Fixture) -> bool {
    resource_id != fixture.disturbance.affected_resource
        || tick < fixture.disturbance.injection_tick
        || tick >= fixture.disturbance.available_again_tick
}

fn resource_is_busy(runtime: &[TaskRuntime], current_index: usize, resource_id: &str) -> bool {
    runtime.iter().enumerate().any(|(index, task)| {
        index != current_index
            && task.state == TaskState::Running
            && task.assigned_resource == resource_id
    })
}

fn matches_expected(actual: &PolicyResult, expected: &ExpectedOutcome) -> bool {
    actual.policy_id == expected.policy_id
        && actual.status == expected.status
        && actual.accepted_synthetic_work == expected.accepted_synthetic_work
        && actual.makespan_ticks == expected.makespan_ticks
        && actual.completed_task_count == expected.completed_task_count
        && actual.interrupted_attempt_count == expected.interrupted_attempt_count
        && actual.resource_reassignments == expected.resource_reassignments
        && actual.replan_tick == expected.replan_tick
}

fn expected_projection(actual: &PolicyResult) -> ExpectedOutcome {
    ExpectedOutcome {
        policy_id: actual.policy_id.clone(),
        status: actual.status.clone(),
        accepted_synthetic_work: actual.accepted_synthetic_work,
        makespan_ticks: actual.makespan_ticks,
        completed_task_count: actual.completed_task_count,
        interrupted_attempt_count: actual.interrupted_attempt_count,
        resource_reassignments: actual.resource_reassignments,
        replan_tick: actual.replan_tick,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_policy_outcomes_match() {
        let report =
            verify_fixture_bytes(EMBEDDED_FIXTURE.as_bytes()).expect("fixture should verify");
        assert_eq!(report.policy_runs_performed, 3);
        assert!(report.all_expected_outcomes_matched);
        assert_eq!(report.policy_results[0].makespan_ticks, None);
        assert_eq!(report.policy_results[1].makespan_ticks, Some(8));
        assert_eq!(report.policy_results[2].makespan_ticks, Some(6));
    }

    #[test]
    fn replanning_waits_until_the_event_is_observable() {
        let report =
            verify_fixture_bytes(EMBEDDED_FIXTURE.as_bytes()).expect("fixture should verify");
        let b0 = report
            .policy_results
            .iter()
            .find(|item| item.policy_id == "B0")
            .unwrap();
        let b1 = report
            .policy_results
            .iter()
            .find(|item| item.policy_id == "B1")
            .unwrap();
        let b2 = report
            .policy_results
            .iter()
            .find(|item| item.policy_id == "B2")
            .unwrap();

        assert_eq!(b0.event_observed_tick, None);
        assert_eq!(b1.event_observed_tick, Some(2));
        assert_eq!(b2.event_observed_tick, Some(2));
        assert_eq!(b2.replan_tick, Some(2));
        assert!(!b0.knowledge_leak_detected);
        assert!(!b1.knowledge_leak_detected);
        assert!(!b2.knowledge_leak_detected);
    }

    #[test]
    fn synthetic_acceptance_requires_final_inspection() {
        let report =
            verify_fixture_bytes(EMBEDDED_FIXTURE.as_bytes()).expect("fixture should verify");
        for policy in report.policy_results {
            if policy.accepted_synthetic_work {
                assert_eq!(policy.completed_task_count, 4);
                assert!(
                    policy
                        .final_resource_assignments
                        .iter()
                        .any(|assignment| { assignment.task_id == "inspect-assembly" })
                );
            } else {
                assert_eq!(policy.policy_id, "B0");
                assert_eq!(policy.completed_task_count, 0);
            }
        }
    }

    #[test]
    fn unknown_schema_fields_are_rejected() {
        let invalid = EMBEDDED_FIXTURE.replace(
            "\"horizon_ticks\": 12,",
            "\"horizon_ticks\": 12, \"unreviewed_field\": true,",
        );
        assert!(verify_fixture_bytes(invalid.as_bytes()).is_err());
    }

    #[test]
    fn invalid_evidence_references_are_rejected() {
        let invalid = EMBEDDED_FIXTURE.replace(
            "\"prepared-panel-candidate\"",
            "\"undeclared-evidence-token\"",
        );
        assert!(verify_fixture_bytes(invalid.as_bytes()).is_err());
    }
}
