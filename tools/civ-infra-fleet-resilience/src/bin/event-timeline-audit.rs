use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, process};

const EMBEDDED_FIXTURE: &str = include_str!(
    "../../../../docs/research/civ-infra-resilience/fixtures/fleet-resilience-v0.1.json"
);
const MAX_AUDIT_TICK: u32 = 256;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    schema_version: String,
    fixture_id: String,
    classification: String,
    claim_ceiling: String,
    warning: String,
    evidence_vocabulary: Vec<String>,
    clock: ClockSpec,
    policies: Vec<PolicySpec>,
    resources: Vec<ResourceSpec>,
    task_graph: Vec<TaskSpec>,
    disturbance_cases: Vec<DisturbanceCase>,
    expected_evaluation_rules: Vec<EvaluationRule>,
    qualification_status: String,
    required_future_checks: Vec<String>,
    tiny_schedule_cases: Vec<TinyScheduleCase>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClockSpec {
    unit: String,
    interval_semantics: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicySpec {
    policy_id: String,
    name: String,
    access_to_unobserved_state: bool,
    #[serde(default)]
    deployable_policy: Option<bool>,
    #[serde(default)]
    purpose: Option<String>,
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
    acceptance_evidence: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DisturbanceCase {
    case_id: String,
    name: String,
    injection_tick: u64,
    observable_tick: u64,
    event: String,
    affected_resources: Vec<String>,
    affected_tasks: Vec<String>,
    invalidates: Vec<String>,
    #[serde(default)]
    recoverability: Option<String>,
    recovery_evidence: Vec<String>,
    required_invariants: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvaluationRule {
    rule_id: String,
    assertion: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TinyScheduleCase {
    case_id: String,
    horizon_ticks: u32,
    tasks: Vec<OracleTask>,
    expected_min_makespan_ticks: Option<u32>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OracleTask {
    task_id: String,
    duration_ticks: u32,
    requires: Vec<String>,
    alternatives: Vec<ScheduleAlternative>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScheduleAlternative {
    alternative_id: String,
    resource_demands: Vec<ResourceDemand>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResourceDemand {
    resource_id: String,
    units: u32,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum DisturbanceKind {
    LocalizationLost,
    CalibrationStale,
    CommunicationOutage,
    RobotFailure,
    LateMaterial,
    MaterialMismatch,
    InspectionRejected,
    GeometryChanged,
    ResourceConflict,
    EnergyReserve,
    UnknownUtility,
    ApprovalDeclined,
    CapabilityStale,
}

impl DisturbanceKind {
    fn for_case(case: &DisturbanceCase) -> Option<Self> {
        let expected = match case.case_id.as_str() {
            "R-001" => ("localization_lost", Self::LocalizationLost),
            "R-002" => ("tool_changed_calibration_stale", Self::CalibrationStale),
            "R-003" => ("communication_outage", Self::CommunicationOutage),
            "R-004" => ("excavator_failure_repairable", Self::RobotFailure),
            "R-005" => ("material_arrives_late", Self::LateMaterial),
            "R-006" => ("wrong_material_delivered", Self::MaterialMismatch),
            "R-007" => ("inspection_rejects_installation", Self::InspectionRejected),
            "R-008" => ("site_geometry_changed_after_schedule", Self::GeometryChanged),
            "R-009" => ("capacity_one_tool_conflict", Self::ResourceConflict),
            "R-010" => ("energy_return_reserve_infeasible", Self::EnergyReserve),
            "R-011" => ("unbounded_unknown_utility", Self::UnknownUtility),
            "R-012" => ("human_approval_declined", Self::ApprovalDeclined),
            "R-013" => ("stale_capability_profile", Self::CapabilityStale),
            _ => return None,
        };
        (case.name == expected.0).then_some(expected.1)
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::LocalizationLost => "localization_lost",
            Self::CalibrationStale => "calibration_stale",
            Self::CommunicationOutage => "communication_outage",
            Self::RobotFailure => "robot_failure",
            Self::LateMaterial => "late_material",
            Self::MaterialMismatch => "material_mismatch",
            Self::InspectionRejected => "inspection_rejected",
            Self::GeometryChanged => "geometry_changed",
            Self::ResourceConflict => "resource_conflict",
            Self::EnergyReserve => "energy_reserve_infeasible",
            Self::UnknownUtility => "unknown_utility",
            Self::ApprovalDeclined => "approval_declined",
            Self::CapabilityStale => "capability_profile_stale",
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct EventTimelineReport {
    schema_version: String,
    fixture_id: String,
    input_sha256: String,
    classification: String,
    timeline_status: String,
    timeline_audit_executed: bool,
    policy_simulation_executed: bool,
    policy_runs_performed: usize,
    counts: Counts,
    event_cases: Vec<EventCaseReport>,
    evaluator_timeline: Vec<EvaluatorTick>,
    policy_timeline: Vec<PolicyTick>,
    claim_ceiling: String,
    explicit_limitations: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct Counts {
    disturbance_cases: usize,
    typed_disturbance_kinds: usize,
    delayed_observations: usize,
    total_observation_delay_ticks: u64,
    maximum_observation_delay_ticks: u64,
    timeline_tick_count: usize,
    referenced_resources: usize,
    referenced_tasks: usize,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct EventCaseReport {
    case_id: String,
    disturbance_kind: String,
    event_name: String,
    injection_tick: u64,
    observable_tick: u64,
    observation_delay_ticks: u64,
    affected_resources: Vec<String>,
    affected_tasks: Vec<String>,
    invalidated_evidence_or_state: Vec<String>,
    recovery_evidence: Vec<String>,
    timeline_status: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct EvaluatorTick {
    tick: u32,
    injected_event_ids: Vec<String>,
    events_injected_but_not_yet_observable: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct PolicyObservation {
    case_id: String,
    disturbance_kind: String,
    event_name: String,
    observed_at_tick: u32,
    affected_resources: Vec<String>,
    affected_tasks: Vec<String>,
    invalidates: Vec<String>,
    recovery_evidence: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct PolicyTick {
    tick: u32,
    newly_observed_event_ids: Vec<String>,
    newly_observed_events: Vec<PolicyObservation>,
    visible_event_ids: Vec<String>,
}

#[derive(Clone, Debug)]
struct QueuedEvent {
    case_id: String,
    kind: DisturbanceKind,
    injection_tick: u32,
    observable_tick: u32,
    event_name: String,
    affected_resources: Vec<String>,
    affected_tasks: Vec<String>,
    invalidates: Vec<String>,
    recovery_evidence: Vec<String>,
}

#[derive(Clone, Debug, Default)]
struct EvaluatorEventQueue {
    injections_by_tick: BTreeMap<u32, Vec<String>>,
}

impl EvaluatorEventQueue {
    fn from_events(events: &[QueuedEvent]) -> Self {
        let mut queue = Self::default();
        for event in events {
            queue
                .injections_by_tick
                .entry(event.injection_tick)
                .or_default()
                .push(event.case_id.clone());
        }
        normalize_queue_buckets(&mut queue.injections_by_tick);
        queue
    }

    fn events_at(&self, tick: u32) -> Vec<String> {
        self.injections_by_tick.get(&tick).cloned().unwrap_or_default()
    }
}

#[derive(Clone, Debug, Default)]
struct PolicyObservationQueue {
    releases_by_tick: BTreeMap<u32, Vec<PolicyObservation>>,
}

impl PolicyObservationQueue {
    fn from_events(events: &[QueuedEvent]) -> Self {
        let mut queue = Self::default();
        for event in events {
            queue
                .releases_by_tick
                .entry(event.observable_tick)
                .or_default()
                .push(PolicyObservation {
                    case_id: event.case_id.clone(),
                    disturbance_kind: event.kind.as_str().to_string(),
                    event_name: event.event_name.clone(),
                    observed_at_tick: event.observable_tick,
                    affected_resources: event.affected_resources.clone(),
                    affected_tasks: event.affected_tasks.clone(),
                    invalidates: event.invalidates.clone(),
                    recovery_evidence: event.recovery_evidence.clone(),
                });
        }
        normalize_observation_buckets(&mut queue.releases_by_tick);
        queue
    }

    fn observations_at(&self, tick: u32) -> Vec<PolicyObservation> {
        self.releases_by_tick.get(&tick).cloned().unwrap_or_default()
    }

    fn final_tick(&self) -> u32 {
        self.releases_by_tick.keys().next_back().copied().unwrap_or(0)
    }
}

fn normalize_observation_buckets(queue: &mut BTreeMap<u32, Vec<PolicyObservation>>) {
    for observations in queue.values_mut() {
        observations.sort_by(|left, right| left.case_id.cmp(&right.case_id));
        observations.dedup_by(|left, right| left.case_id == right.case_id);
    }
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
        eprintln!("usage: event-timeline-audit [fixture.json]");
        process::exit(2);
    }

    match audit_fixture_bytes(&input_bytes) {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("cannot serialize timeline report: {error}");
                process::exit(2);
            }
        },
        Err(errors) => {
            for error in errors {
                eprintln!("timeline audit rejected fixture: {error}");
            }
            process::exit(2);
        }
    }
}

fn audit_fixture_bytes(input_bytes: &[u8]) -> Result<EventTimelineReport, Vec<String>> {
    let fixture: Fixture = serde_json::from_slice(input_bytes).map_err(|error| {
        vec![format!(
            "fixture JSON does not match strict event-timeline schema: {error}"
        )]
    })?;

    let mut errors = validate_fixture(&fixture);
    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let mut events: Vec<QueuedEvent> = fixture
        .disturbance_cases
        .iter()
        .map(|case| QueuedEvent {
            case_id: case.case_id.clone(),
            kind: DisturbanceKind::for_case(case).expect("validated disturbance kind"),
            injection_tick: case.injection_tick as u32,
            observable_tick: case.observable_tick as u32,
            event_name: case.event.clone(),
            affected_resources: sorted_unique(case.affected_resources.clone()),
            affected_tasks: sorted_unique(case.affected_tasks.clone()),
            invalidates: sorted_unique(case.invalidates.clone()),
            recovery_evidence: sorted_unique(case.recovery_evidence.clone()),
        })
        .collect();

    events.sort_by(|left, right| {
        (left.injection_tick, &left.case_id).cmp(&(right.injection_tick, &right.case_id))
    });

    let evaluator_queue = EvaluatorEventQueue::from_events(&events);
    let policy_observation_queue = PolicyObservationQueue::from_events(&events);
    let max_tick = policy_observation_queue.final_tick();
    let mut evaluator_timeline = Vec::with_capacity(max_tick as usize + 1);
    let mut policy_timeline = Vec::with_capacity(max_tick as usize + 1);
    let mut injected_so_far = BTreeSet::<String>::new();
    let mut visible_so_far = BTreeSet::<String>::new();

    for tick in 0..=max_tick {
        let injected_event_ids = evaluator_queue.events_at(tick);
        injected_so_far.extend(injected_event_ids.iter().cloned());

        let newly_observed_events = policy_observation_queue.observations_at(tick);
        let newly_observed_event_ids = newly_observed_events
            .iter()
            .map(|observation| observation.case_id.clone())
            .collect::<Vec<_>>();
        visible_so_far.extend(newly_observed_event_ids.iter().cloned());

        let events_injected_but_not_yet_observable = injected_so_far
            .difference(&visible_so_far)
            .cloned()
            .collect::<Vec<_>>();
        let visible_event_ids = visible_so_far.iter().cloned().collect::<Vec<_>>();

        evaluator_timeline.push(EvaluatorTick {
            tick,
            injected_event_ids,
            events_injected_but_not_yet_observable,
        });
        policy_timeline.push(PolicyTick {
            tick,
            newly_observed_event_ids,
            newly_observed_events,
            visible_event_ids,
        });
    }

    if let Err(mut timeline_errors) = validate_timeline_separation(&events, &evaluator_timeline, &policy_timeline) {
        errors.append(&mut timeline_errors);
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let event_cases = events
        .iter()
        .map(|event| {
            let observation_delay_ticks =
                u64::from(event.observable_tick - event.injection_tick);
            EventCaseReport {
                case_id: event.case_id.clone(),
                disturbance_kind: event.kind.as_str().to_string(),
                event_name: event.event_name.clone(),
                injection_tick: u64::from(event.injection_tick),
                observable_tick: u64::from(event.observable_tick),
                observation_delay_ticks,
                affected_resources: event.affected_resources.clone(),
                affected_tasks: event.affected_tasks.clone(),
                invalidated_evidence_or_state: event.invalidates.clone(),
                recovery_evidence: event.recovery_evidence.clone(),
                timeline_status: if observation_delay_ticks == 0 {
                    "visible_on_injection_tick".to_string()
                } else {
                    "hidden_until_observation_tick".to_string()
                },
            }
        })
        .collect::<Vec<_>>();

    let delayed_observations = events
        .iter()
        .filter(|event| event.observable_tick > event.injection_tick)
        .count();
    let total_observation_delay_ticks = events
        .iter()
        .map(|event| u64::from(event.observable_tick - event.injection_tick))
        .sum();
    let maximum_observation_delay_ticks = events
        .iter()
        .map(|event| u64::from(event.observable_tick - event.injection_tick))
        .max()
        .unwrap_or(0);

    let digest = Sha256::digest(input_bytes);
    Ok(EventTimelineReport {
        schema_version: "luminous.civ_infra.event_timeline_audit.v0.1".to_string(),
        fixture_id: fixture.fixture_id.clone(),
        input_sha256: format!("{digest:x}"),
        classification: fixture.classification.clone(),
        timeline_status: "canonical_event_queue_timing_audited".to_string(),
        timeline_audit_executed: true,
        policy_simulation_executed: false,
        policy_runs_performed: 0,
        counts: Counts {
            disturbance_cases: events.len(),
            typed_disturbance_kinds: events
                .iter()
                .map(|event| event.kind)
                .collect::<BTreeSet<_>>()
                .len(),
            delayed_observations,
            total_observation_delay_ticks,
            maximum_observation_delay_ticks,
            timeline_tick_count: evaluator_timeline.len(),
            referenced_resources: fixture.resources.len(),
            referenced_tasks: fixture.task_graph.len(),
        },
        event_cases,
        evaluator_timeline,
        policy_timeline,
        claim_ceiling: fixture.claim_ceiling.clone(),
        explicit_limitations: vec![
            "This audits event timing and visibility only; it does not execute B0/B1/B2 policies or simulate recovery.".to_string(),
            "The tick schedule and event semantics are synthetic test parameters, not a model of measured physical timing.".to_string(),
            "A policy-visible event is not proof that the underlying evidence is correct, sufficient, or qualified.".to_string(),
            "The hidden evaluator timeline and policy-visible timeline are separate report projections; future policy code must consume only the policy-visible projection.".to_string(),
        ],
    })
}

fn validate_fixture(fixture: &Fixture) -> Vec<String> {
    let mut errors = Vec::new();
    if fixture.schema_version != "civ-infra-fleet-resilience/0.1" {
        errors.push("unsupported schema_version".to_string());
    }
    if fixture.fixture_id.trim().is_empty()
        || fixture.claim_ceiling.trim().is_empty()
        || fixture.warning.trim().is_empty()
    {
        errors.push("fixture identity, claim ceiling, and warning must be explicit".to_string());
    }
    if fixture.classification != "synthetic_only" {
        errors.push("classification must remain synthetic_only".to_string());
    }
    if fixture.qualification_status != "proposed_not_executed" {
        errors.push("qualification_status must remain proposed_not_executed".to_string());
    }
    if fixture.clock.unit != "fixture_tick" || fixture.clock.interval_semantics != "half_open" {
        errors.push("v0.1 timeline requires fixture_tick and half_open interval semantics".to_string());
    }
    if fixture.evidence_vocabulary.is_empty()
        || fixture.policies.is_empty()
        || fixture.resources.is_empty()
        || fixture.task_graph.is_empty()
        || fixture.expected_evaluation_rules.is_empty()
        || fixture.required_future_checks.is_empty()
    {
        errors.push("required top-level contract collections must not be empty".to_string());
    }

    check_unique(
        fixture.evidence_vocabulary.iter().map(String::as_str),
        "evidence vocabulary item",
        &mut errors,
    );
    check_unique(
        fixture.policies.iter().map(|item| item.policy_id.as_str()),
        "policy_id",
        &mut errors,
    );
    check_unique(
        fixture.resources.iter().map(|item| item.resource_id.as_str()),
        "resource_id",
        &mut errors,
    );
    check_unique(
        fixture.task_graph.iter().map(|item| item.task_id.as_str()),
        "task_id",
        &mut errors,
    );
    check_unique(
        fixture.disturbance_cases.iter().map(|item| item.case_id.as_str()),
        "case_id",
        &mut errors,
    );
    check_unique(
        fixture.expected_evaluation_rules.iter().map(|item| item.rule_id.as_str()),
        "evaluation rule id",
        &mut errors,
    );

    let resource_ids: BTreeSet<&str> = fixture
        .resources
        .iter()
        .map(|resource| resource.resource_id.as_str())
        .collect();
    let task_ids: BTreeSet<&str> = fixture
        .task_graph
        .iter()
        .map(|task| task.task_id.as_str())
        .collect();
    let evidence_ids: BTreeSet<&str> = fixture.evidence_vocabulary.iter().map(String::as_str).collect();

    for resource in &fixture.resources {
        if resource.resource_id.trim().is_empty()
            || resource.resource_kind.trim().is_empty()
            || resource.capacity == 0
        {
            errors.push(format!("resource {} has invalid identity, kind, or capacity", resource.resource_id));
        }
    }
    for task in &fixture.task_graph {
        if task.task_id.trim().is_empty() || task.duration_ticks == 0 {
            errors.push("task IDs and durations must be non-empty and positive".to_string());
        }
        for dependency in &task.requires {
            if !task_ids.contains(dependency.as_str()) {
                errors.push(format!("task {} references unknown dependency {}", task.task_id, dependency));
            }
        }
        for evidence in &task.acceptance_evidence {
            if !evidence_ids.contains(evidence.as_str()) {
                errors.push(format!("task {} references unknown acceptance evidence {}", task.task_id, evidence));
            }
        }
    }
    for policy in &fixture.policies {
        if policy.policy_id.trim().is_empty() || policy.name.trim().is_empty() {
            errors.push("policy ID and name must not be empty".to_string());
        }
        if policy.access_to_unobserved_state {
            errors.push(format!("policy {} must not access unobserved state", policy.policy_id));
        }
    }

    if fixture.disturbance_cases.len() != 13 {
        errors.push("frozen v0.1 corpus must contain exactly 13 disturbance cases".to_string());
    }
    for number in 1..=13 {
        let required = format!("R-{number:03}");
        if !fixture.disturbance_cases.iter().any(|case| case.case_id == required) {
            errors.push(format!("required disturbance case {required} is absent"));
        }
    }

    for case in &fixture.disturbance_cases {
        if DisturbanceKind::for_case(case).is_none() {
            errors.push(format!("case {} has an unknown or changed frozen disturbance kind/name", case.case_id));
        }
        if case.event.trim().is_empty() {
            errors.push(format!("case {} has an empty event name", case.case_id));
        }
        if case.observable_tick < case.injection_tick {
            errors.push(format!("case {} is observable before injection", case.case_id));
        }
        if case.injection_tick > u64::from(MAX_AUDIT_TICK)
            || case.observable_tick > u64::from(MAX_AUDIT_TICK)
        {
            errors.push(format!("case {} exceeds the bounded audit horizon", case.case_id));
        }
        if case.recovery_evidence.is_empty() || case.required_invariants.is_empty() {
            errors.push(format!("case {} must declare recovery evidence and invariants", case.case_id));
        }
        for resource_id in &case.affected_resources {
            if !resource_ids.contains(resource_id.as_str()) {
                errors.push(format!("case {} references unknown resource {}", case.case_id, resource_id));
            }
        }
        for task_id in &case.affected_tasks {
            if !task_ids.contains(task_id.as_str()) {
                errors.push(format!("case {} references unknown task {}", case.case_id, task_id));
            }
        }
        for evidence in &case.recovery_evidence {
            if !evidence_ids.contains(evidence.as_str()) {
                errors.push(format!("case {} references unknown recovery evidence {}", case.case_id, evidence));
            }
        }
    }

    if fixture.tiny_schedule_cases.len() != 3 {
        errors.push("frozen v0.1 fixture must contain exactly three tiny schedule cases".to_string());
    }
    for case in &fixture.tiny_schedule_cases {
        if case.case_id.trim().is_empty() || case.horizon_ticks == 0 || case.tasks.is_empty() {
            errors.push("tiny schedule case identity, horizon, and tasks must be non-empty".to_string());
        }
        if let Some(expected) = case.expected_min_makespan_ticks {
            if expected > case.horizon_ticks {
                errors.push(format!("tiny schedule case {} expects makespan beyond horizon", case.case_id));
            }
        }
        for task in &case.tasks {
            if task.task_id.trim().is_empty() || task.duration_ticks == 0 || task.alternatives.is_empty() {
                errors.push(format!("tiny schedule task in {} has invalid identity, duration, or alternatives", case.case_id));
            }
            for alternative in &task.alternatives {
                for demand in &alternative.resource_demands {
                    if demand.resource_id.trim().is_empty() || demand.units == 0 {
                        errors.push(format!("tiny schedule case {} contains invalid resource demand", case.case_id));
                    }
                }
            }
        }
    }

    errors
}

fn validate_timeline_separation(
    events: &[QueuedEvent],
    evaluator_timeline: &[EvaluatorTick],
    policy_timeline: &[PolicyTick],
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if evaluator_timeline.len() != policy_timeline.len() {
        errors.push("evaluator and policy projections have different timeline lengths".to_string());
        return Err(errors);
    }

    for (evaluator, policy) in evaluator_timeline.iter().zip(policy_timeline) {
        if evaluator.tick != policy.tick {
            errors.push("evaluator and policy projections disagree on tick identity".to_string());
            continue;
        }

        for case_id in &policy.visible_event_ids {
            match events.iter().find(|event| &event.case_id == case_id) {
                Some(event) if event.observable_tick <= policy.tick => {}
                Some(_) => errors.push(format!(
                    "policy-visible projection leaks {} before observable_tick at tick {}",
                    case_id, policy.tick
                )),
                None => errors.push(format!("policy-visible projection references unknown event {case_id}")),
            }
        }

        let expected_visible = sorted_unique(
            events
                .iter()
                .filter(|event| event.observable_tick <= policy.tick)
                .map(|event| event.case_id.clone())
                .collect(),
        );
        if policy.visible_event_ids != expected_visible {
            errors.push(format!("policy-visible projection is incomplete or reordered at tick {}", policy.tick));
        }

        let expected_hidden = sorted_unique(
            events
                .iter()
                .filter(|event| event.injection_tick <= evaluator.tick && event.observable_tick > evaluator.tick)
                .map(|event| event.case_id.clone())
                .collect(),
        );
        if evaluator.events_injected_but_not_yet_observable != expected_hidden {
            errors.push(format!("evaluator hidden-event set is incorrect at tick {}", evaluator.tick));
        }

        let expected_observation_queue = PolicyObservationQueue::from_events(events);
        let expected_observations = expected_observation_queue.observations_at(policy.tick);
        let expected_new_observations = expected_observations
            .iter()
            .map(|observation| observation.case_id.clone())
            .collect::<Vec<_>>();
        if policy.newly_observed_events != expected_observations
            || policy.newly_observed_event_ids != expected_new_observations
        {
            errors.push(format!("new observation release set is incorrect at tick {}", policy.tick));
        }
        let expected_injections = sorted_unique(
            events
                .iter()
                .filter(|event| event.injection_tick == evaluator.tick)
                .map(|event| event.case_id.clone())
                .collect(),
        );
        if evaluator.injected_event_ids != expected_injections {
            errors.push(format!("world injection set is incorrect at tick {}", evaluator.tick));
        }

        for newly_observed in &policy.newly_observed_event_ids {
            if !policy.visible_event_ids.contains(newly_observed) {
                errors.push(format!("newly observed event {} is not in visible set at tick {}", newly_observed, policy.tick));
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        errors.sort();
        errors.dedup();
        Err(errors)
    }
}

fn check_unique<'a>(
    values: impl Iterator<Item = &'a str>,
    label: &str,
    errors: &mut Vec<String>,
) {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            errors.push(format!("duplicate {label}: {value}"));
        }
    }
}

fn sorted_unique(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generated_property_events(base_seed: u64, scenario_index: usize) -> Vec<QueuedEvent> {
        let kinds = [
            DisturbanceKind::LocalizationLost,
            DisturbanceKind::CalibrationStale,
            DisturbanceKind::CommunicationOutage,
            DisturbanceKind::RobotFailure,
            DisturbanceKind::LateMaterial,
            DisturbanceKind::MaterialMismatch,
            DisturbanceKind::InspectionRejected,
            DisturbanceKind::GeometryChanged,
            DisturbanceKind::ResourceConflict,
            DisturbanceKind::EnergyReserve,
            DisturbanceKind::UnknownUtility,
            DisturbanceKind::ApprovalDeclined,
            DisturbanceKind::CapabilityStale,
        ];
        let mut state = base_seed
            ^ (scenario_index as u64).wrapping_mul(0x9E3779B97F4A7C15);
        let mut events = Vec::with_capacity(kinds.len());

        for (event_index, kind) in kinds.into_iter().enumerate() {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let injection_tick = (state % 8) as u32;
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let observation_delay = (state % 4) as u32;

            events.push(QueuedEvent {
                case_id: format!("G-{scenario_index:04}-{event_index:02}"),
                kind,
                injection_tick,
                observable_tick: injection_tick + observation_delay,
                event_name: kind.as_str().to_string(),
                affected_resources: vec![format!("resource-{event_index:02}")],
                affected_tasks: vec![format!("task-{event_index:02}")],
                invalidates: vec![format!("state-{event_index:02}")],
                recovery_evidence: vec!["synthetic-recovery-receipt".to_string()],
            });
        }

        events
    }

    #[test]
    fn canonical_corpus_produces_thirteen_typed_events() {
        let report = audit_fixture_bytes(EMBEDDED_FIXTURE.as_bytes()).expect("fixture should audit");
        assert_eq!(report.counts.disturbance_cases, 13);
        assert_eq!(report.counts.typed_disturbance_kinds, 13);
        assert_eq!(report.counts.delayed_observations, 3);
        assert_eq!(report.counts.total_observation_delay_ticks, 4);
        assert_eq!(report.counts.maximum_observation_delay_ticks, 2);
        assert_eq!(report.counts.timeline_tick_count, 8);
        assert!(!report.policy_simulation_executed);
        assert_eq!(report.policy_runs_performed, 0);
    }

    #[test]
    fn changed_geometry_stays_hidden_until_tick_four() {
        let report = audit_fixture_bytes(EMBEDDED_FIXTURE.as_bytes()).expect("fixture should audit");
        let tick_two = report.policy_timeline.iter().find(|tick| tick.tick == 2).unwrap();
        let tick_three = report.policy_timeline.iter().find(|tick| tick.tick == 3).unwrap();
        let tick_four = report.policy_timeline.iter().find(|tick| tick.tick == 4).unwrap();
        assert!(!tick_two.visible_event_ids.iter().any(|id| id == "R-008"));
        assert!(!tick_three.visible_event_ids.iter().any(|id| id == "R-008"));
        assert!(tick_four.visible_event_ids.iter().any(|id| id == "R-008"));

        let evaluator_two = report.evaluator_timeline.iter().find(|tick| tick.tick == 2).unwrap();
        assert!(evaluator_two.injected_event_ids.iter().any(|id| id == "R-008"));
        assert!(evaluator_two.events_injected_but_not_yet_observable.iter().any(|id| id == "R-008"));
    }

    #[test]
    fn policy_observation_payload_omits_hidden_injection_time() {
        let report = audit_fixture_bytes(EMBEDDED_FIXTURE.as_bytes()).expect("fixture should audit");
        let tick_four = report.policy_timeline.iter().find(|tick| tick.tick == 4).unwrap();
        let geometry = tick_four
            .newly_observed_events
            .iter()
            .find(|observation| observation.case_id == "R-008")
            .expect("geometry revision observation should release at tick four");
        assert_eq!(geometry.observed_at_tick, 4);
        assert_eq!(geometry.event_name, "workface_geometry_revision_changed");
        assert!(geometry.invalidates.iter().any(|item| item == "site-snapshot"));
        assert!(geometry.affected_tasks.iter().any(|item| item == "trench-segment-01"));

        let json = serde_json::to_value(&report).expect("report serializes");
        let observations = json["policy_timeline"][4]["newly_observed_events"]
            .as_array()
            .expect("observation array exists");
        let payload = observations
            .iter()
            .find(|value| value["case_id"] == "R-008")
            .expect("R-008 payload exists");
        assert!(payload.get("injection_tick").is_none());
        assert!(payload.get("world_state").is_none());
    }

    #[test]
    fn same_tick_events_are_ordered_deterministically() {
        let report = audit_fixture_bytes(EMBEDDED_FIXTURE.as_bytes()).expect("fixture should audit");
        let tick_two = report.evaluator_timeline.iter().find(|tick| tick.tick == 2).unwrap();
        assert_eq!(
            tick_two.injected_event_ids,
            vec![
                "R-001".to_string(),
                "R-003".to_string(),
                "R-008".to_string(),
                "R-009".to_string(),
                "R-010".to_string(),
            ]
        );
    }

    #[test]
    fn zero_delay_events_are_visible_on_the_injection_tick() {
        let report = audit_fixture_bytes(EMBEDDED_FIXTURE.as_bytes()).expect("fixture should audit");
        let tick_two = report.policy_timeline.iter().find(|tick| tick.tick == 2).unwrap();
        assert!(tick_two.newly_observed_event_ids.iter().any(|id| id == "R-001"));
        assert!(tick_two.visible_event_ids.iter().any(|id| id == "R-001"));
    }

    #[test]
    fn no_policy_projection_uses_future_or_unobserved_events() {
        let report = audit_fixture_bytes(EMBEDDED_FIXTURE.as_bytes()).expect("fixture should audit");
        for tick in &report.policy_timeline {
            for event in &report.event_cases {
                if event.observable_tick > u64::from(tick.tick) {
                    assert!(
                        !tick.visible_event_ids.iter().any(|id| id == &event.case_id),
                        "{} visible at tick {} before observable tick {}",
                        event.case_id,
                        tick.tick,
                        event.observable_tick
                    );
                }
            }
        }
    }

    #[test]
    fn unknown_schema_fields_fail_closed() {
        let mut value: serde_json::Value =
            serde_json::from_slice(EMBEDDED_FIXTURE.as_bytes()).expect("fixture JSON parses");
        value["unexpected_authority"] = serde_json::json!(true);
        let mutated = serde_json::to_vec(&value).expect("mutated fixture serializes");
        assert!(audit_fixture_bytes(&mutated).is_err());
    }

    #[test]
    fn injection_after_observation_is_rejected() {
        let mut value: serde_json::Value =
            serde_json::from_slice(EMBEDDED_FIXTURE.as_bytes()).expect("fixture JSON parses");
        value["disturbance_cases"][2]["observable_tick"] = serde_json::json!(1);
        let mutated = serde_json::to_vec(&value).expect("mutated fixture serializes");
        let errors = audit_fixture_bytes(&mutated).expect_err("impossible timing must reject");
        assert!(errors.iter().any(|error| error.contains("observable before injection")));
    }

    #[test]
    fn fixture_array_order_does_not_change_queue_results() {
        let baseline = audit_fixture_bytes(EMBEDDED_FIXTURE.as_bytes()).expect("fixture should audit");
        let mut value: serde_json::Value =
            serde_json::from_slice(EMBEDDED_FIXTURE.as_bytes()).expect("fixture JSON parses");
        value["disturbance_cases"]
            .as_array_mut()
            .expect("disturbance_cases is an array")
            .reverse();
        let reordered_bytes = serde_json::to_vec(&value).expect("reordered fixture serializes");
        let reordered = audit_fixture_bytes(&reordered_bytes).expect("reordered fixture should audit");

        assert_ne!(baseline.input_sha256, reordered.input_sha256);
        assert_eq!(baseline.event_cases, reordered.event_cases);
        assert_eq!(baseline.evaluator_timeline, reordered.evaluator_timeline);
        assert_eq!(baseline.policy_timeline, reordered.policy_timeline);
    }

    #[test]
    fn duplicate_case_ids_are_rejected() {
        let mut value: serde_json::Value =
            serde_json::from_slice(EMBEDDED_FIXTURE.as_bytes()).expect("fixture JSON parses");
        value["disturbance_cases"][12]["case_id"] = serde_json::json!("R-001");
        let mutated = serde_json::to_vec(&value).expect("mutated fixture serializes");
        let errors = audit_fixture_bytes(&mutated).expect_err("duplicate IDs must fail closed");
        assert!(errors.iter().any(|error| error.contains("duplicate case_id")));
    }

    #[test]
    fn one_thousand_generated_queues_preserve_visibility_and_replay_invariants() {
        let base_seed = 0xC1F1_1A5E_2026_u64;
        let scenario_count = 1_000_usize;

        let mut scenarios_checked = 0_usize;
        for scenario_index in 0..scenario_count {
            let events = generated_property_events(base_seed, scenario_index);
            scenarios_checked += 1;
            let evaluator_queue = EvaluatorEventQueue::from_events(&events);
            let observation_queue = PolicyObservationQueue::from_events(&events);
            let mut reversed_events = events.clone();
            reversed_events.reverse();
            let reversed_evaluator_queue = EvaluatorEventQueue::from_events(&reversed_events);
            let reversed_observation_queue =
                PolicyObservationQueue::from_events(&reversed_events);
            let final_tick = observation_queue.final_tick();
            let mut injected_so_far = BTreeSet::<String>::new();
            let mut visible_so_far = BTreeSet::<String>::new();

            for tick in 0..=final_tick {
                let injected_now = evaluator_queue.events_at(tick);
                let observed_now = observation_queue.observations_at(tick);
                injected_so_far.extend(injected_now);
                visible_so_far.extend(
                    observed_now
                        .iter()
                        .map(|observation| observation.case_id.clone()),
                );

                for event in &events {
                    let expected_visible = event.observable_tick <= tick;
                    assert_eq!(
                        visible_so_far.contains(&event.case_id),
                        expected_visible,
                        "scenario {scenario_index}: {} visibility differs at tick {tick}",
                        event.case_id
                    );

                    let expected_hidden = event.injection_tick <= tick
                        && event.observable_tick > tick;
                    let actual_hidden = injected_so_far.contains(&event.case_id)
                        && !visible_so_far.contains(&event.case_id);
                    assert_eq!(
                        actual_hidden,
                        expected_hidden,
                        "scenario {scenario_index}: {} hidden-event set differs at tick {tick}",
                        event.case_id
                    );
                }

                assert_eq!(
                    evaluator_queue.events_at(tick),
                    reversed_evaluator_queue.events_at(tick),
                    "scenario {scenario_index}: injection order changed after input permutation"
                );
                assert_eq!(
                    observation_queue.observations_at(tick),
                    reversed_observation_queue.observations_at(tick),
                    "scenario {scenario_index}: observation order changed after input permutation"
                );
            }
        }

        assert_eq!(scenarios_checked, 1_000);
    }

    #[test]
    fn report_digest_binds_exact_fixture_bytes() {
        let baseline = audit_fixture_bytes(EMBEDDED_FIXTURE.as_bytes()).expect("fixture should audit");
        let mut whitespace_variant = EMBEDDED_FIXTURE.as_bytes().to_vec();
        whitespace_variant.push(b' ');
        let replay = audit_fixture_bytes(&whitespace_variant).expect("trailing JSON whitespace is valid");
        assert_eq!(baseline.counts.disturbance_cases, replay.counts.disturbance_cases);
        assert_ne!(baseline.input_sha256, replay.input_sha256);
    }
}
