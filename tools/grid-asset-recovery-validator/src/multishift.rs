use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const TOLERANCE: f64 = 1e-9;

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionCase {
    pub schema_version: String,
    pub case_id: String,
    pub time_origin_utc: String,
    pub as_of_hour: f64,
    pub horizon_end_hour: f64,
    pub tasks: Vec<ExecutionTask>,
    pub resources: Vec<ExecutionResource>,
    pub baseline_commitments: Vec<Commitment>,
    pub proposed_commitments: Vec<Commitment>,
    pub events: Vec<ExecutionEvent>,
    pub planned_segments: Vec<PlannedSegment>,
    #[serde(default)]
    pub reconciliations: Vec<ReconciliationEvent>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionTask {
    pub event_id: String,
    pub task_id: String,
    pub release_hour: f64,
    pub required_effort_hours: f64,
    pub minimum_segment_effort_hours: f64,
    pub preemption_mode: PreemptionMode,
    pub maximum_interruptions: u32,
    #[serde(default)]
    pub checkpoint_effort_hours: Vec<f64>,
    pub paused_resource_policy: PausedResourcePolicy,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub resource_requirements: Vec<ResourceRequirement>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreemptionMode {
    NonPreemptible,
    Checkpointed,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PausedResourcePolicy {
    Release,
    Retain,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRequirement {
    pub resource_id: String,
    pub units: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionResource {
    pub resource_id: String,
    pub capacity_windows: Vec<CapacityWindow>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapacityWindow {
    pub start_hour: f64,
    pub end_hour: f64,
    pub capacity_units: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Commitment {
    pub commitment_id: String,
    pub event_id: String,
    pub task_id: String,
    pub version: u64,
    pub state: CommitmentState,
    #[serde(default)]
    pub frozen_event_ids: Vec<String>,
    pub active_segment_id: Option<String>,
    pub source_evidence_ref: String,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommitmentState {
    InProgress,
    Paused,
    Completed,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionEvent {
    pub event_id: String,
    pub idempotency_key: String,
    pub event_type: ExecutionEventType,
    pub event_id_ref: String,
    pub task_id: String,
    pub segment_id: String,
    pub sequence: u32,
    pub start_hour: f64,
    pub end_hour: f64,
    pub productive_effort_hours: f64,
    pub effective_hour: f64,
    pub observed_hour: f64,
    pub evidence_status: EvidenceStatus,
    pub evidence_class: EvidenceClass,
    pub evidence_ref: String,
    pub interruption_cause: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionEventType {
    WorkSegmentCompleted,
    ProgressSnapshot,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    Current,
    Superseded,
    Stale,
    Conflicted,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    Observed,
    SyntheticFixture,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlannedSegment {
    pub segment_id: String,
    pub event_id: String,
    pub task_id: String,
    pub sequence: u32,
    pub start_hour: f64,
    pub end_hour: f64,
    pub productive_effort_hours: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReconciliationEvent {
    pub event_id: String,
    pub commitment_id: String,
    pub from_version: u64,
    pub to_version: u64,
    pub action: ReconciliationAction,
    pub observed_hour: f64,
    pub evidence_status: EvidenceStatus,
    pub authority_ref: String,
    pub evidence_ref: String,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationAction {
    PauseConfirmed,
    ResumeAuthorized,
    CompletionConfirmed,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionReport {
    pub case_id: String,
    pub result_status: String,
    pub event_deliveries_deduplicated: usize,
    pub tasks: Vec<TaskReport>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaskReport {
    pub event_id: String,
    pub task_id: String,
    pub commitment_state: String,
    pub observed_effort_hours: f64,
    pub planned_effort_hours: f64,
    pub remaining_effort_hours: f64,
    pub interruption_count: usize,
    pub projected_completion_hour: Option<f64>,
}

#[derive(Clone, Debug)]
struct SegmentView {
    segment_id: String,
    event_id: String,
    task_id: String,
    sequence: u32,
    start_hour: f64,
    end_hour: f64,
    effort_hours: f64,
    completed: bool,
    interruption_cause: Option<String>,
}

#[derive(Clone, Debug)]
struct ResourceUse {
    start_hour: f64,
    end_hour: f64,
    units: u32,
    task_key: String,
}

fn task_key(event_id: &str, task_id: &str) -> String {
    format!("{event_id}::{task_id}")
}

fn finite_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn approximately_equal(left: f64, right: f64) -> bool {
    (left - right).abs() <= TOLERANCE
}

fn valid_time_origin(value: &str) -> bool {
    value.ends_with('Z') && value.contains('T') && value.len() >= 20
}

fn validate_event_ledger(
    case: &ExecutionCase,
    errors: &mut Vec<String>,
) -> (Vec<ExecutionEvent>, usize) {
    let mut by_id: BTreeMap<String, ExecutionEvent> = BTreeMap::new();
    let mut idempotency: BTreeMap<String, String> = BTreeMap::new();
    let mut deduplicated = 0usize;

    for event in &case.events {
        if event.event_id.trim().is_empty()
            || event.idempotency_key.trim().is_empty()
            || event.event_id_ref.trim().is_empty()
            || event.task_id.trim().is_empty()
            || event.segment_id.trim().is_empty()
            || event.evidence_ref.trim().is_empty()
        {
            errors.push("execution event has an empty identity or evidence reference".to_string());
            continue;
        }

        if let Some(existing) = by_id.get(&event.event_id) {
            if existing == event {
                deduplicated += 1;
            } else {
                errors.push(format!(
                    "event ID {} was replayed with conflicting content",
                    event.event_id
                ));
            }
            continue;
        }

        if let Some(existing_id) = idempotency.get(&event.idempotency_key) {
            if existing_id != &event.event_id {
                errors.push(format!(
                    "idempotency key {} is bound to multiple event IDs",
                    event.idempotency_key
                ));
                continue;
            }
        } else {
            idempotency.insert(event.idempotency_key.clone(), event.event_id.clone());
        }

        if !finite_nonnegative(event.start_hour)
            || !finite_nonnegative(event.end_hour)
            || !finite_nonnegative(event.productive_effort_hours)
            || !finite_nonnegative(event.effective_hour)
            || !finite_nonnegative(event.observed_hour)
            || event.end_hour <= event.start_hour
            || event.productive_effort_hours <= 0.0
            || event.productive_effort_hours > (event.end_hour - event.start_hour) + TOLERANCE
            || event.observed_hour + TOLERANCE < event.effective_hour
            || event.observed_hour > case.as_of_hour + TOLERANCE
            || event.effective_hour > case.as_of_hour + TOLERANCE
        {
            errors.push(format!("event {} has invalid time or effort values", event.event_id));
        }

        if !matches!(event.evidence_status, EvidenceStatus::Current | EvidenceStatus::Superseded) {
            errors.push(format!(
                "event {} has stale, conflicted or unknown evidence; reconciliation is required",
                event.event_id
            ));
        }
        if event.evidence_status == EvidenceStatus::Superseded
            && event.event_type != ExecutionEventType::ProgressSnapshot
        {
            errors.push(format!(
                "only progress snapshots may be marked superseded: {}",
                event.event_id
            ));
        }

        match event.event_type {
            ExecutionEventType::WorkSegmentCompleted => {
                if !approximately_equal(event.end_hour, event.effective_hour) {
                    errors.push(format!(
                        "completed segment event {} must take effect at its end hour",
                        event.event_id
                    ));
                }
            }
            ExecutionEventType::ProgressSnapshot => {
                if !approximately_equal(event.end_hour, case.as_of_hour)
                    || !approximately_equal(event.effective_hour, case.as_of_hour)
                    || event.evidence_status == EvidenceStatus::Superseded
                        && event.interruption_cause.is_some()
                {
                    errors.push(format!(
                        "progress snapshot {} must represent the current as-of boundary",
                        event.event_id
                    ));
                }
            }
        }

        by_id.insert(event.event_id.clone(), event.clone());
    }

    let mut canonical: Vec<_> = by_id.into_values().collect();
    canonical.sort_by(|left, right| {
        left.effective_hour
            .total_cmp(&right.effective_hour)
            .then_with(|| left.event_id.cmp(&right.event_id))
    });
    (canonical, deduplicated)
}

fn validate_reconciliations(
    case: &ExecutionCase,
    tasks: &BTreeMap<String, &ExecutionTask>,
    errors: &mut Vec<String>,
) -> BTreeMap<String, ReconciliationEvent> {
    let baseline: BTreeMap<String, &Commitment> = case
        .baseline_commitments
        .iter()
        .map(|item| (item.commitment_id.clone(), item))
        .collect();
    let proposed: BTreeMap<String, &Commitment> = case
        .proposed_commitments
        .iter()
        .map(|item| (item.commitment_id.clone(), item))
        .collect();

    if baseline.len() != case.baseline_commitments.len()
        || proposed.len() != case.proposed_commitments.len()
    {
        errors.push("duplicate commitment ID".to_string());
    }
    if baseline.keys().collect::<Vec<_>>() != proposed.keys().collect::<Vec<_>>() {
        errors.push("proposed commitments must preserve every baseline commitment identity".to_string());
    }

    let mut reconciliation_by_commitment = BTreeMap::new();
    let mut event_ids = BTreeSet::new();
    for reconciliation in &case.reconciliations {
        if reconciliation.event_id.trim().is_empty()
            || reconciliation.commitment_id.trim().is_empty()
            || reconciliation.authority_ref.trim().is_empty()
            || reconciliation.evidence_ref.trim().is_empty()
            || !finite_nonnegative(reconciliation.observed_hour)
            || reconciliation.observed_hour > case.as_of_hour + TOLERANCE
            || reconciliation.evidence_status != EvidenceStatus::Current
            || reconciliation.to_version != reconciliation.from_version + 1
            || !event_ids.insert(reconciliation.event_id.as_str())
        {
            errors.push(format!(
                "reconciliation event {} is incomplete, stale or has invalid version/time",
                reconciliation.event_id
            ));
            continue;
        }
        if reconciliation_by_commitment
            .insert(reconciliation.commitment_id.clone(), reconciliation.clone())
            .is_some()
        {
            errors.push(format!(
                "multiple reconciliation events target commitment {}",
                reconciliation.commitment_id
            ));
        }
    }

    for (commitment_id, old) in &baseline {
        let Some(new) = proposed.get(commitment_id) else {
            continue;
        };
        if old.event_id != new.event_id || old.task_id != new.task_id {
            errors.push(format!("commitment {commitment_id} changed task identity"));
            continue;
        }
        if !old
            .frozen_event_ids
            .iter()
            .all(|id| new.frozen_event_ids.contains(id))
        {
            errors.push(format!("commitment {commitment_id} dropped frozen execution events"));
        }

        let changed = old.version != new.version
            || old.state != new.state
            || old.active_segment_id != new.active_segment_id;
        let reconciliation = reconciliation_by_commitment.get(commitment_id);

        if !changed {
            if let Some(event) = reconciliation {
                errors.push(format!(
                    "commitment {commitment_id} has an unnecessary reconciliation {}",
                    event.event_id
                ));
            }
            continue;
        }

        let Some(event) = reconciliation else {
            errors.push(format!(
                "commitment {commitment_id} changed without an explicit reconciliation event"
            ));
            continue;
        };
        if event.from_version != old.version || event.to_version != new.version {
            errors.push(format!(
                "reconciliation {} does not bind the exact commitment versions",
                event.event_id
            ));
        }
        if old.state == CommitmentState::Completed {
            errors.push(format!("completed commitment {commitment_id} is immutable"));
        }

        let action_matches = matches!(
            (old.state, new.state, event.action),
            (
                CommitmentState::InProgress,
                CommitmentState::Paused,
                ReconciliationAction::PauseConfirmed
            ) | (
                CommitmentState::Paused,
                CommitmentState::InProgress,
                ReconciliationAction::ResumeAuthorized
            ) | (
                CommitmentState::InProgress,
                CommitmentState::Completed,
                ReconciliationAction::CompletionConfirmed
            ) | (
                CommitmentState::Paused,
                CommitmentState::Completed,
                ReconciliationAction::CompletionConfirmed
            )
        );
        if !action_matches {
            errors.push(format!(
                "reconciliation {} action does not match the commitment state transition",
                event.event_id
            ));
        }
        if new.frozen_event_ids.len() < old.frozen_event_ids.len() {
            errors.push(format!("commitment {commitment_id} shortened its frozen event ledger"));
        }
        if !tasks.contains_key(&task_key(&new.event_id, &new.task_id)) {
            errors.push(format!("commitment {commitment_id} references an unknown task"));
        }
    }

    for commitment_id in reconciliation_by_commitment.keys() {
        if !baseline.contains_key(commitment_id) {
            errors.push(format!(
                "reconciliation references unknown baseline commitment {commitment_id}"
            ));
        }
    }
    reconciliation_by_commitment
}

fn build_segment_views(
    events: &[ExecutionEvent],
    errors: &mut Vec<String>,
) -> BTreeMap<String, Vec<SegmentView>> {
    let mut completed: BTreeMap<String, &ExecutionEvent> = BTreeMap::new();
    let mut progress: BTreeMap<String, Vec<&ExecutionEvent>> = BTreeMap::new();

    for event in events {
        match event.event_type {
            ExecutionEventType::WorkSegmentCompleted => {
                if completed.insert(event.segment_id.clone(), event).is_some() {
                    errors.push(format!(
                        "multiple completion events reference segment {}",
                        event.segment_id
                    ));
                }
            }
            ExecutionEventType::ProgressSnapshot => {
                progress.entry(event.segment_id.clone()).or_default().push(event);
            }
        }
    }

    let mut segment_ids: BTreeSet<String> = completed.keys().cloned().collect();
    segment_ids.extend(progress.keys().cloned());

    let mut by_task: BTreeMap<String, Vec<SegmentView>> = BTreeMap::new();
    for segment_id in segment_ids {
        let completion = completed.get(&segment_id).copied();
        let snapshots = progress.get(&segment_id).cloned().unwrap_or_default();
        let mut ordered_snapshots = snapshots;
        ordered_snapshots.sort_by(|left, right| {
            left.effective_hour
                .total_cmp(&right.effective_hour)
                .then_with(|| left.event_id.cmp(&right.event_id))
        });

        for pair in ordered_snapshots.windows(2) {
            if pair[1].effective_hour <= pair[0].effective_hour + TOLERANCE
                || pair[1].productive_effort_hours + TOLERANCE
                    < pair[0].productive_effort_hours
                || !approximately_equal(pair[1].start_hour, pair[0].start_hour)
            {
                errors.push(format!(
                    "progress snapshots for segment {segment_id} are not monotonic"
                ));
            }
        }

        if let Some(done) = completion {
            for snapshot in &ordered_snapshots {
                if snapshot.evidence_status != EvidenceStatus::Superseded
                    || snapshot.effective_hour > done.effective_hour + TOLERANCE
                    || snapshot.productive_effort_hours
                        > done.productive_effort_hours + TOLERANCE
                    || !approximately_equal(snapshot.start_hour, done.start_hour)
                {
                    errors.push(format!(
                        "completed segment {segment_id} does not reconcile its prior progress snapshots"
                    ));
                }
            }
            by_task.entry(task_key(&done.event_id_ref, &done.task_id))
                .or_default()
                .push(SegmentView {
                    segment_id: done.segment_id.clone(),
                    event_id: done.event_id_ref.clone(),
                    task_id: done.task_id.clone(),
                    sequence: done.sequence,
                    start_hour: done.start_hour,
                    end_hour: done.end_hour,
                    effort_hours: done.productive_effort_hours,
                    completed: true,
                    interruption_cause: done.interruption_cause.clone(),
                });
        } else if let Some(latest) = ordered_snapshots.last() {
            if latest.evidence_status != EvidenceStatus::Current {
                errors.push(format!(
                    "open segment {segment_id} has no current progress snapshot"
                ));
            }
            if ordered_snapshots[..ordered_snapshots.len().saturating_sub(1)]
                .iter()
                .any(|event| event.evidence_status != EvidenceStatus::Superseded)
            {
                errors.push(format!(
                    "older progress snapshots for segment {segment_id} must be marked superseded"
                ));
            }
            by_task.entry(task_key(&latest.event_id_ref, &latest.task_id))
                .or_default()
                .push(SegmentView {
                    segment_id: latest.segment_id.clone(),
                    event_id: latest.event_id_ref.clone(),
                    task_id: latest.task_id.clone(),
                    sequence: latest.sequence,
                    start_hour: latest.start_hour,
                    end_hour: latest.end_hour,
                    effort_hours: latest.productive_effort_hours,
                    completed: false,
                    interruption_cause: None,
                });
        }
    }

    for segments in by_task.values_mut() {
        segments.sort_by(|left, right| {
            left.sequence
                .cmp(&right.sequence)
                .then_with(|| left.segment_id.cmp(&right.segment_id))
        });
        let mut seen = BTreeSet::new();
        for segment in segments.iter() {
            if !seen.insert(segment.sequence) {
                errors.push(format!(
                    "task {} has duplicate execution segment sequence {}",
                    segment.task_id, segment.sequence
                ));
            }
        }
    }
    by_task
}

fn validate_resource_windows(
    resources: &BTreeMap<String, &ExecutionResource>,
    errors: &mut Vec<String>,
) {
    for (resource_id, resource) in resources {
        if resource.capacity_windows.is_empty() {
            errors.push(format!("resource {resource_id} has no capacity windows"));
            continue;
        }
        let mut windows = resource.capacity_windows.clone();
        windows.sort_by(|left, right| left.start_hour.total_cmp(&right.start_hour));
        let mut previous_end = f64::NEG_INFINITY;
        for window in windows {
            if !finite_nonnegative(window.start_hour)
                || !finite_nonnegative(window.end_hour)
                || window.end_hour <= window.start_hour
                || window.start_hour < previous_end - TOLERANCE
            {
                errors.push(format!("resource {resource_id} has invalid or overlapping capacity windows"));
            }
            previous_end = window.end_hour;
        }
    }
}

fn capacity_at(resource: &ExecutionResource, hour: f64) -> u32 {
    resource
        .capacity_windows
        .iter()
        .find(|window| window.start_hour <= hour && hour < window.end_hour)
        .map(|window| window.capacity_units)
        .unwrap_or(0)
}

fn validate_resource_usage(
    case: &ExecutionCase,
    tasks: &BTreeMap<String, &ExecutionTask>,
    resources: &BTreeMap<String, &ExecutionResource>,
    segment_views: &BTreeMap<String, Vec<SegmentView>>,
    errors: &mut Vec<String>,
) {
    let planned_by_task: BTreeMap<String, Vec<&PlannedSegment>> = {
        let mut map: BTreeMap<String, Vec<&PlannedSegment>> = BTreeMap::new();
        for segment in &case.planned_segments {
            map.entry(task_key(&segment.event_id, &segment.task_id))
                .or_default()
                .push(segment);
        }
        map
    };

    let mut allocations: BTreeMap<String, Vec<ResourceUse>> = BTreeMap::new();
    for (key, task) in tasks {
        let mut intervals: Vec<(f64, f64)> = Vec::new();
        if let Some(views) = segment_views.get(key) {
            intervals.extend(views.iter().map(|s| (s.start_hour, s.end_hour)));
        }
        if let Some(planned) = planned_by_task.get(key) {
            intervals.extend(planned.iter().map(|s| (s.start_hour, s.end_hour)));
        }
        intervals.sort_by(|left, right| left.0.total_cmp(&right.0));

        for requirement in &task.resource_requirements {
            if !resources.contains_key(&requirement.resource_id) {
                errors.push(format!(
                    "{} requires unknown resource {}",
                    key, requirement.resource_id
                ));
                continue;
            }
            if requirement.units == 0 {
                errors.push(format!("{} requires zero units of {}", key, requirement.resource_id));
                continue;
            }

            let mut uses = Vec::new();
            if task.paused_resource_policy == PausedResourcePolicy::Retain && intervals.len() > 1 {
                if let (Some(first), Some(last)) = (intervals.first(), intervals.last()) {
                    uses.push(ResourceUse {
                        start_hour: first.0,
                        end_hour: last.1,
                        units: requirement.units,
                        task_key: key.clone(),
                    });
                }
            } else {
                for (start_hour, end_hour) in &intervals {
                    uses.push(ResourceUse {
                        start_hour: *start_hour,
                        end_hour: *end_hour,
                        units: requirement.units,
                        task_key: key.clone(),
                    });
                }
            }
            allocations
                .entry(requirement.resource_id.clone())
                .or_default()
                .extend(uses);
        }
    }

    for (resource_id, uses) in allocations {
        let Some(resource) = resources.get(&resource_id) else {
            continue;
        };
        let mut boundaries: Vec<f64> = uses
            .iter()
            .flat_map(|item| [item.start_hour, item.end_hour])
            .collect();
        boundaries.extend(
            resource
                .capacity_windows
                .iter()
                .flat_map(|window| [window.start_hour, window.end_hour]),
        );
        boundaries.sort_by(f64::total_cmp);
        boundaries.dedup_by(|left, right| approximately_equal(*left, *right));

        for pair in boundaries.windows(2) {
            let start = pair[0];
            let end = pair[1];
            if end <= start + TOLERANCE {
                continue;
            }
            let midpoint = start + ((end - start) / 2.0);
            let used: u32 = uses
                .iter()
                .filter(|item| item.start_hour <= midpoint && midpoint < item.end_hour)
                .map(|item| item.units)
                .sum();
            let capacity = capacity_at(resource, midpoint);
            if used > capacity {
                let owners: BTreeSet<_> = uses
                    .iter()
                    .filter(|item| item.start_hour <= midpoint && midpoint < item.end_hour)
                    .map(|item| item.task_key.as_str())
                    .collect();
                errors.push(format!(
                    "resource {resource_id} capacity exceeded during [{start}, {end}) by tasks {:?}: used {used}, capacity {capacity}",
                    owners
                ));
            }
        }
    }
}

pub fn validate(case: &ExecutionCase) -> Result<ExecutionReport, Vec<String>> {
    let mut errors = Vec::new();
    if case.schema_version != "0.1.0" {
        errors.push("unsupported execution schemaVersion".to_string());
    }
    if case.case_id.trim().is_empty() || !valid_time_origin(&case.time_origin_utc) {
        errors.push("caseId and UTC timeOriginUtc are required".to_string());
    }
    if !finite_nonnegative(case.as_of_hour)
        || !finite_nonnegative(case.horizon_end_hour)
        || case.horizon_end_hour <= case.as_of_hour
    {
        errors.push("invalid asOfHour or horizonEndHour".to_string());
    }

    let mut tasks_by_key = BTreeMap::new();
    for task in &case.tasks {
        let key = task_key(&task.event_id, &task.task_id);
        if task.event_id.trim().is_empty()
            || task.task_id.trim().is_empty()
            || !finite_nonnegative(task.release_hour)
            || !task.required_effort_hours.is_finite()
            || task.required_effort_hours <= 0.0
            || !task.minimum_segment_effort_hours.is_finite()
            || task.minimum_segment_effort_hours <= 0.0
            || task.minimum_segment_effort_hours > task.required_effort_hours + TOLERANCE
        {
            errors.push(format!("task {key} has invalid identity or effort limits"));
        }
        if tasks_by_key.insert(key.clone(), task).is_some() {
            errors.push(format!("duplicate task identity {key}"));
        }
        if task.preemption_mode == PreemptionMode::NonPreemptible
            && (!task.checkpoint_effort_hours.is_empty() || task.maximum_interruptions != 0)
        {
            errors.push(format!("non-preemptible task {key} cannot declare checkpoints or interruptions"));
        }
        let mut previous_checkpoint = 0.0;
        for checkpoint in &task.checkpoint_effort_hours {
            if !checkpoint.is_finite()
                || *checkpoint <= previous_checkpoint + TOLERANCE
                || *checkpoint >= task.required_effort_hours - TOLERANCE
            {
                errors.push(format!("task {key} has invalid or unordered checkpoints"));
            }
            previous_checkpoint = *checkpoint;
        }
        if task.preemption_mode == PreemptionMode::Checkpointed
            && task.checkpoint_effort_hours.is_empty()
        {
            errors.push(format!("checkpointed task {key} must declare at least one legal checkpoint"));
        }
        let mut dependency_ids = BTreeSet::new();
        for dependency in &task.depends_on {
            if !dependency_ids.insert(dependency) {
                errors.push(format!("task {key} has duplicate dependency {dependency}"));
            }
        }
    }

    let mut resources_by_id = BTreeMap::new();
    for resource in &case.resources {
        if resource.resource_id.trim().is_empty()
            || resources_by_id.insert(resource.resource_id.clone(), resource).is_some()
        {
            errors.push("empty or duplicate resource ID".to_string());
        }
    }
    validate_resource_windows(&resources_by_id, &mut errors);

    let mut event_by_idempotency = BTreeMap::new();
    let (events, deduplicated) = validate_event_ledger(case, &mut errors);
    for event in &events {
        let key = task_key(&event.event_id_ref, &event.task_id);
        if !tasks_by_key.contains_key(&key) {
            errors.push(format!("event {} references unknown task {key}", event.event_id));
        }
        if event.sequence > 0 && event.event_type == ExecutionEventType::ProgressSnapshot {
            // A current progress snapshot may follow any number of completed segments.
        }
        if let Some(previous) = event_by_idempotency.insert(event.idempotency_key.as_str(), event.event_id.as_str()) {
            if previous != event.event_id.as_str() {
                errors.push(format!("event idempotency key {} is reused", event.idempotency_key));
            }
        }
    }

    let mut commitments_baseline = BTreeMap::new();
    for commitment in &case.baseline_commitments {
        if commitment.commitment_id.trim().is_empty()
            || commitment.source_evidence_ref.trim().is_empty()
            || commitment.version == 0
            || commitments_baseline
                .insert(commitment.commitment_id.clone(), commitment)
                .is_some()
        {
            errors.push("baseline commitment has an empty/duplicate identity or invalid version".to_string());
        }
    }
    let mut commitments_proposed = BTreeMap::new();
    for commitment in &case.proposed_commitments {
        if commitment.commitment_id.trim().is_empty()
            || commitment.source_evidence_ref.trim().is_empty()
            || commitment.version == 0
            || commitments_proposed
                .insert(commitment.commitment_id.clone(), commitment)
                .is_some()
        {
            errors.push("proposed commitment has an empty/duplicate identity or invalid version".to_string());
        }
    }

    for commitment in case
        .baseline_commitments
        .iter()
        .chain(case.proposed_commitments.iter())
    {
        let key = task_key(&commitment.event_id, &commitment.task_id);
        if !tasks_by_key.contains_key(&key) {
            errors.push(format!("commitment {} references unknown task {key}", commitment.commitment_id));
        }
        let event_ids_for_task: BTreeSet<_> = events
            .iter()
            .filter(|event| event.event_id_ref == commitment.event_id && event.task_id == commitment.task_id)
            .map(|event| event.event_id.as_str())
            .collect();
        let frozen_ids: BTreeSet<_> = commitment.frozen_event_ids.iter().map(String::as_str).collect();
        if frozen_ids.len() != commitment.frozen_event_ids.len() || frozen_ids != event_ids_for_task {
            errors.push(format!(
                "commitment {} does not preserve the complete immutable event ledger for its task",
                commitment.commitment_id
            ));
        }
    }

    if commitments_baseline.keys().collect::<Vec<_>>() != commitments_proposed.keys().collect::<Vec<_>>() {
        errors.push("proposed plan must retain all baseline commitment IDs".to_string());
    }
    let reconciliation_by_commitment = validate_reconciliations(case, &tasks_by_key, &mut errors);

    let segment_views = build_segment_views(&events, &mut errors);
    let mut planned_by_task: BTreeMap<String, Vec<&PlannedSegment>> = BTreeMap::new();
    let mut planned_ids = BTreeSet::new();

    for segment in &case.planned_segments {
        let key = task_key(&segment.event_id, &segment.task_id);
        if !tasks_by_key.contains_key(&key) {
            errors.push(format!("planned segment {} references unknown task {key}", segment.segment_id));
            continue;
        }
        if segment.segment_id.trim().is_empty()
            || !planned_ids.insert(segment.segment_id.as_str())
            || !finite_nonnegative(segment.start_hour)
            || !finite_nonnegative(segment.end_hour)
            || !finite_nonnegative(segment.productive_effort_hours)
            || segment.end_hour <= segment.start_hour
            || segment.start_hour + TOLERANCE < case.as_of_hour
            || segment.end_hour > case.horizon_end_hour + TOLERANCE
            || segment.productive_effort_hours <= 0.0
            || segment.productive_effort_hours > (segment.end_hour - segment.start_hour) + TOLERANCE
        {
            errors.push(format!("planned segment {} has invalid identity, horizon or effort", segment.segment_id));
        }
        planned_by_task.entry(key).or_default().push(segment);
    }

    let mut reports = Vec::new();
    for (key, task) in &tasks_by_key {
        let views = segment_views.get(key).cloned().unwrap_or_default();
        let planned = planned_by_task.get(key).cloned().unwrap_or_default();
        let mut all_segments: Vec<(u32, f64, f64, f64, bool, Option<String>, String)> = Vec::new();

        for view in &views {
            all_segments.push((
                view.sequence,
                view.start_hour,
                view.end_hour,
                view.effort_hours,
                view.completed,
                view.interruption_cause.clone(),
                view.segment_id.clone(),
            ));
        }
        for segment in &planned {
            all_segments.push((
                segment.sequence,
                segment.start_hour,
                segment.end_hour,
                segment.productive_effort_hours,
                false,
                None,
                segment.segment_id.clone(),
            ));
        }
        all_segments.sort_by(|left, right| left.0.cmp(&right.0));

        let mut sequence_ids = BTreeSet::new();
        let mut prior_end = f64::NEG_INFINITY;
        let mut observed_effort = 0.0;
        let mut planned_effort = 0.0;
        let mut interruption_count = 0usize;
        let mut checkpoint_effort = 0.0;

        for (index, (sequence, start, end, effort, completed, interruption, segment_id)) in
            all_segments.iter().enumerate()
        {
            if !sequence_ids.insert(*sequence) {
                errors.push(format!("task {key} has duplicate segment sequence {sequence}"));
            }
            if *sequence as usize != index {
                errors.push(format!("task {key} segment sequences must be contiguous from zero"));
            }
            if *start + TOLERANCE < task.release_hour
                || *start + TOLERANCE < prior_end
                || *end <= *start
            {
                errors.push(format!("task {key} has overlapping, unordered or pre-release segment {segment_id}"));
            }
            if *start + TOLERANCE < case.as_of_hour && !*completed {
                errors.push(format!("planned segment {segment_id} starts before the as-of boundary"));
            }
            if *completed {
                observed_effort += *effort;
            } else if views.iter().any(|view| view.segment_id == *segment_id && !view.completed) {
                observed_effort += *effort;
            } else {
                planned_effort += *effort;
            }
            prior_end = *end;
            checkpoint_effort += *effort;

            let is_final = index + 1 == all_segments.len();
            if !is_final {
                interruption_count += 1;
                if !*completed || interruption.as_deref().is_none_or(str::is_empty) {
                    errors.push(format!("task {key} has an unrecorded interruption after segment {segment_id}"));
                }
                if task.preemption_mode == PreemptionMode::Checkpointed
                    && !task.checkpoint_effort_hours.iter().any(|value| approximately_equal(*value, checkpoint_effort))
                {
                    errors.push(format!("task {key} paused after {checkpoint_effort} effort hours outside a legal checkpoint"));
                }
            }
            if *completed && *effort + TOLERANCE < task.minimum_segment_effort_hours
                && !approximately_equal(observed_effort + planned_effort, task.required_effort_hours)
            {
                errors.push(format!("completed segment {segment_id} is shorter than the minimum segment effort"));
            }
            if !*completed && task.preemption_mode == PreemptionMode::Checkpointed
                && !is_final
            {
                errors.push(format!("active or planned segment {segment_id} cannot be an interruption boundary"));
            }
        }

        if task.preemption_mode == PreemptionMode::NonPreemptible && all_segments.len() > 1 {
            errors.push(format!("non-preemptible task {key} has more than one execution segment"));
        }
        if interruption_count > task.maximum_interruptions as usize {
            errors.push(format!("task {key} exceeds its maximum interruption count"));
        }
        let total = observed_effort + planned_effort;
        if total > task.required_effort_hours + TOLERANCE {
            errors.push(format!("task {key} over-delivers effort: {total} > {}", task.required_effort_hours));
        }

        let commitment = commitments_proposed
            .values()
            .find(|item| item.event_id == task.event_id && item.task_id == task.task_id);

        let state_name = match commitment.map(|item| item.state) {
            Some(CommitmentState::InProgress) => "in_progress",
            Some(CommitmentState::Paused) => "paused",
            Some(CommitmentState::Completed) => "completed",
            None => "not_started",
        };

        if let Some(commitment) = commitment {
            match commitment.state {
                CommitmentState::InProgress => {
                    let active = views.iter().find(|view| !view.completed);
                    if active.map(|view| view.segment_id.as_str()) != commitment.active_segment_id.as_deref() {
                        errors.push(format!("in-progress commitment {} does not bind its active segment", commitment.commitment_id));
                    }
                    if !planned.is_empty() {
                        errors.push(format!("in-progress task {key} cannot be moved or extended by replanning before an observed pause/checkpoint"));
                    }
                    if active.is_none() {
                        errors.push(format!("in-progress commitment {} lacks a current progress snapshot", commitment.commitment_id));
                    }
                }
                CommitmentState::Paused => {
                    if commitment.active_segment_id.is_some() {
                        errors.push(format!("paused commitment {} still names an active segment", commitment.commitment_id));
                    }
                    if views.iter().any(|view| !view.completed) {
                        errors.push(format!("paused commitment {} contains a current active segment", commitment.commitment_id));
                    }
                    if views.last().is_none_or(|view| view.interruption_cause.as_deref().is_none_or(str::is_empty)) {
                        errors.push(format!("paused commitment {} lacks an observed pause cause", commitment.commitment_id));
                    }
                }
                CommitmentState::Completed => {
                    if commitment.active_segment_id.is_some()
                        || !planned.is_empty()
                        || !approximately_equal(observed_effort, task.required_effort_hours)
                    {
                        errors.push(format!("completed commitment {} does not reconcile all required work", commitment.commitment_id));
                    }
                }
            }
        } else if !views.is_empty() {
            errors.push(format!("task {key} has observed work but no commitment record"));
        }

        if let Some(old) = commitments_baseline.values().find(|item| item.event_id == task.event_id && item.task_id == task.task_id) {
            if let Some(new) = commitments_proposed.get(&old.commitment_id) {
                if old.state != new.state || old.version != new.version {
                    if !reconciliation_by_commitment.contains_key(&old.commitment_id) {
                        errors.push(format!("commitment {} changed without verified reconciliation", old.commitment_id));
                    }
                }
            }
        }

        let remaining = (task.required_effort_hours - total).max(0.0);
        let projected_completion = if remaining <= TOLERANCE {
            all_segments.last().map(|segment| segment.2)
        } else if planned_effort > 0.0 {
            all_segments.last().map(|segment| segment.2)
        } else {
            None
        };

        reports.push(TaskReport {
            event_id: task.event_id.clone(),
            task_id: task.task_id.clone(),
            commitment_state: state_name.to_string(),
            observed_effort_hours: observed_effort,
            planned_effort_hours: planned_effort,
            remaining_effort_hours: remaining,
            interruption_count,
            projected_completion_hour: projected_completion,
        });
    }

    validate_resource_usage(case, &tasks_by_key, &resources_by_id, &segment_views, &mut errors);

    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    reports.sort_by(|left, right| {
        left.event_id
            .cmp(&right.event_id)
            .then_with(|| left.task_id.cmp(&right.task_id))
    });

    Ok(ExecutionReport {
        case_id: case.case_id.clone(),
        result_status: "execution_ledger_and_candidate_schedule_consistent".to_string(),
        event_deliveries_deduplicated: deduplicated,
        tasks: reports,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ExecutionCase {
        ExecutionCase {
            schema_version: "0.1.0".to_string(),
            case_id: "synthetic-multi-shift-recovery-001".to_string(),
            time_origin_utc: "2030-01-01T00:00:00Z".to_string(),
            as_of_hour: 8.0,
            horizon_end_hour: 24.0,
            tasks: vec![
                ExecutionTask {
                    event_id: "storm-001".to_string(),
                    task_id: "repair-pump".to_string(),
                    release_hour: 0.0,
                    required_effort_hours: 8.0,
                    minimum_segment_effort_hours: 2.0,
                    preemption_mode: PreemptionMode::Checkpointed,
                    maximum_interruptions: 2,
                    checkpoint_effort_hours: vec![4.0, 6.0],
                    paused_resource_policy: PausedResourcePolicy::Release,
                    depends_on: vec![],
                    resource_requirements: vec![ResourceRequirement {
                        resource_id: "crew-a".to_string(),
                        units: 1,
                    }],
                },
                ExecutionTask {
                    event_id: "storm-001".to_string(),
                    task_id: "test-commission".to_string(),
                    release_hour: 0.0,
                    required_effort_hours: 2.0,
                    minimum_segment_effort_hours: 1.0,
                    preemption_mode: PreemptionMode::NonPreemptible,
                    maximum_interruptions: 0,
                    checkpoint_effort_hours: vec![],
                    paused_resource_policy: PausedResourcePolicy::Release,
                    depends_on: vec!["repair-pump".to_string()],
                    resource_requirements: vec![ResourceRequirement {
                        resource_id: "crew-a".to_string(),
                        units: 1,
                    }],
                },
            ],
            resources: vec![
                ExecutionResource {
                    resource_id: "crew-a".to_string(),
                    capacity_windows: vec![
                        CapacityWindow { start_hour: 0.0, end_hour: 24.0, capacity_units: 1 },
                    ],
                },
            ],
            baseline_commitments: vec![
                Commitment {
                    commitment_id: "commit-repair-pump".to_string(),
                    event_id: "storm-001".to_string(),
                    task_id: "repair-pump".to_string(),
                    version: 1,
                    state: CommitmentState::Paused,
                    frozen_event_ids: vec!["seg-1-complete".to_string()],
                    active_segment_id: None,
                    source_evidence_ref: "synthetic://commitment/repair-pump/v1".to_string(),
                },
            ],
            proposed_commitments: vec![
                Commitment {
                    commitment_id: "commit-repair-pump".to_string(),
                    event_id: "storm-001".to_string(),
                    task_id: "repair-pump".to_string(),
                    version: 1,
                    state: CommitmentState::Paused,
                    frozen_event_ids: vec!["seg-1-complete".to_string()],
                    active_segment_id: None,
                    source_evidence_ref: "synthetic://commitment/repair-pump/v1".to_string(),
                },
            ],
            events: vec![
                ExecutionEvent {
                    event_id: "seg-1-complete".to_string(),
                    idempotency_key: "storm-001-repair-pump-seg-0".to_string(),
                    event_type: ExecutionEventType::WorkSegmentCompleted,
                    event_id_ref: "storm-001".to_string(),
                    task_id: "repair-pump".to_string(),
                    segment_id: "repair-pump-segment-0".to_string(),
                    sequence: 0,
                    start_hour: 0.0,
                    end_hour: 4.0,
                    productive_effort_hours: 4.0,
                    effective_hour: 4.0,
                    observed_hour: 4.1,
                    evidence_status: EvidenceStatus::Current,
                    evidence_class: EvidenceClass::SyntheticFixture,
                    evidence_ref: "synthetic://segment/repair-pump/0".to_string(),
                    interruption_cause: Some("crew-shift-end".to_string()),
                },
            ],
            planned_segments: vec![
                PlannedSegment {
                    segment_id: "repair-pump-segment-1".to_string(),
                    event_id: "storm-001".to_string(),
                    task_id: "repair-pump".to_string(),
                    sequence: 1,
                    start_hour: 10.0,
                    end_hour: 14.0,
                    productive_effort_hours: 4.0,
                },
                PlannedSegment {
                    segment_id: "test-commission-segment-0".to_string(),
                    event_id: "storm-001".to_string(),
                    task_id: "test-commission".to_string(),
                    sequence: 0,
                    start_hour: 14.0,
                    end_hour: 16.0,
                    productive_effort_hours: 2.0,
                },
            ],
            reconciliations: vec![],
        }
    }

    #[test]
    fn accepts_checkpoint_pause_and_calendar_safe_resume() {
        let report = validate(&fixture()).expect("fixture should be consistent");
        let repair = report.tasks.iter().find(|item| item.task_id == "repair-pump").unwrap();
        assert_eq!(repair.observed_effort_hours, 4.0);
        assert_eq!(repair.planned_effort_hours, 4.0);
        assert_eq!(repair.remaining_effort_hours, 0.0);
        assert_eq!(repair.interruption_count, 1);
        assert_eq!(repair.projected_completion_hour, Some(14.0));
    }

    #[test]
    fn exact_duplicate_event_delivery_is_deduplicated() {
        let mut case = fixture();
        case.events.push(case.events[0].clone());
        case.baseline_commitments[0].frozen_event_ids.push("seg-1-complete".to_string());
        case.proposed_commitments[0].frozen_event_ids.push("seg-1-complete".to_string());
        let report = validate(&case).expect("identical event replay is idempotent");
        assert_eq!(report.event_deliveries_deduplicated, 1);
    }

    #[test]
    fn conflicting_duplicate_event_id_is_rejected() {
        let mut case = fixture();
        let mut conflict = case.events[0].clone();
        conflict.productive_effort_hours = 3.0;
        case.events.push(conflict);
        assert!(validate(&case).is_err());
    }

    #[test]
    fn stale_event_evidence_blocks_replanning() {
        let mut case = fixture();
        case.events[0].evidence_status = EvidenceStatus::Stale;
        assert!(validate(&case).is_err());
    }

    #[test]
    fn progress_after_snapshot_is_rejected() {
        let mut case = fixture();
        case.events[0].observed_hour = 25.0;
        assert!(validate(&case).is_err());
    }

    #[test]
    fn over_delivery_of_effort_is_rejected() {
        let mut case = fixture();
        case.planned_segments[0].productive_effort_hours = 5.0;
        assert!(validate(&case).is_err());
    }

    #[test]
    fn missing_checkpoint_is_rejected() {
        let mut case = fixture();
        case.tasks[0].checkpoint_effort_hours = vec![3.0, 6.0];
        assert!(validate(&case).is_err());
    }

    #[test]
    fn non_preemptible_task_cannot_be_split() {
        let mut case = fixture();
        case.planned_segments.push(PlannedSegment {
            segment_id: "test-commission-segment-1".to_string(),
            event_id: "storm-001".to_string(),
            task_id: "test-commission".to_string(),
            sequence: 1,
            start_hour: 16.0,
            end_hour: 17.0,
            productive_effort_hours: 1.0,
        });
        assert!(validate(&case).is_err());
    }

    #[test]
    fn in_progress_commitment_remains_frozen() {
        let mut case = fixture();
        case.events.push(ExecutionEvent {
            event_id: "repair-progress".to_string(),
            idempotency_key: "repair-progress-key".to_string(),
            event_type: ExecutionEventType::ProgressSnapshot,
            event_id_ref: "storm-001".to_string(),
            task_id: "repair-pump".to_string(),
            segment_id: "repair-pump-active".to_string(),
            sequence: 1,
            start_hour: 4.0,
            end_hour: 8.0,
            productive_effort_hours: 3.0,
            effective_hour: 8.0,
            observed_hour: 8.0,
            evidence_status: EvidenceStatus::Current,
            evidence_class: EvidenceClass::SyntheticFixture,
            evidence_ref: "synthetic://progress/repair-pump".to_string(),
            interruption_cause: None,
        });
        case.baseline_commitments[0].state = CommitmentState::InProgress;
        case.proposed_commitments[0].state = CommitmentState::InProgress;
        case.baseline_commitments[0].active_segment_id = Some("repair-pump-active".to_string());
        case.proposed_commitments[0].active_segment_id = Some("repair-pump-active".to_string());
        case.baseline_commitments[0].frozen_event_ids.push("repair-progress".to_string());
        case.proposed_commitments[0].frozen_event_ids.push("repair-progress".to_string());
        case.planned_segments.retain(|s| s.task_id != "repair-pump");
        let report = validate(&case).expect("in-progress commitment remains frozen");
        let repair = report.tasks.iter().find(|x| x.task_id == "repair-pump").unwrap();
        assert_eq!(repair.observed_effort_hours, 7.0);
        assert_eq!(repair.planned_effort_hours, 0.0);
    }

    #[test]
    fn in_progress_task_cannot_be_silently_rescheduled() {
        let mut case = fixture();
        case.events.push(ExecutionEvent {
            event_id: "repair-progress".to_string(),
            idempotency_key: "repair-progress-key".to_string(),
            event_type: ExecutionEventType::ProgressSnapshot,
            event_id_ref: "storm-001".to_string(),
            task_id: "repair-pump".to_string(),
            segment_id: "repair-pump-active".to_string(),
            sequence: 1,
            start_hour: 4.0,
            end_hour: 8.0,
            productive_effort_hours: 3.0,
            effective_hour: 8.0,
            observed_hour: 8.0,
            evidence_status: EvidenceStatus::Current,
            evidence_class: EvidenceClass::SyntheticFixture,
            evidence_ref: "synthetic://progress/repair-pump".to_string(),
            interruption_cause: None,
        });
        case.baseline_commitments[0].state = CommitmentState::InProgress;
        case.proposed_commitments[0].state = CommitmentState::InProgress;
        case.baseline_commitments[0].active_segment_id = Some("repair-pump-active".to_string());
        case.proposed_commitments[0].active_segment_id = Some("repair-pump-active".to_string());
        case.baseline_commitments[0].frozen_event_ids.push("repair-progress".to_string());
        case.proposed_commitments[0].frozen_event_ids.push("repair-progress".to_string());
        assert!(validate(&case).is_err());
    }

    #[test]
    fn commitment_state_change_requires_reconciliation() {
        let mut case = fixture();
        case.proposed_commitments[0].state = CommitmentState::Completed;
        case.proposed_commitments[0].version = 2;
        assert!(validate(&case).is_err());
    }

    #[test]
    fn resource_capacity_conflict_is_rejected() {
        let mut case = fixture();
        case.planned_segments[1].start_hour = 12.0;
        case.planned_segments[1].end_hour = 14.0;
        case.tasks[1].depends_on.clear();
        assert!(validate(&case).is_err());
    }

    #[test]
    fn released_resource_is_available_between_segments() {
        let report = validate(&fixture()).expect("released crew can be reused between shifts");
        assert_eq!(report.result_status, "execution_ledger_and_candidate_schedule_consistent");
    }

    #[test]
    fn retained_resource_reserves_the_pause_interval() {
        let mut case = fixture();
        case.tasks[0].paused_resource_policy = PausedResourcePolicy::Retain;
        case.planned_segments[0].start_hour = 6.0;
        case.planned_segments[0].end_hour = 10.0;
        case.resources[0].capacity_windows = vec![
            CapacityWindow { start_hour: 0.0, end_hour: 6.0, capacity_units: 1 },
            CapacityWindow { start_hour: 10.0, end_hour: 24.0, capacity_units: 1 },
        ];
        assert!(validate(&case).is_err());
    }

    #[test]
    fn task_dependency_is_respected() {
        let mut case = fixture();
        case.planned_segments[1].start_hour = 13.0;
        case.planned_segments[1].end_hour = 15.0;
        assert!(validate(&case).is_err());
    }

    #[test]
    fn input_order_does_not_change_canonical_report() {
        let original = validate(&fixture()).unwrap();
        let mut reordered = fixture();
        reordered.tasks.reverse();
        reordered.resources.reverse();
        reordered.events.reverse();
        reordered.planned_segments.reverse();
        reordered.baseline_commitments.reverse();
        reordered.proposed_commitments.reverse();
        let replay = validate(&reordered).unwrap();
        assert_eq!(original, replay);
    }
}
