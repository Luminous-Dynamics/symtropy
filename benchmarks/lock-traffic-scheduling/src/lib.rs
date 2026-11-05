//! Deterministic, evidence-gated lock traffic scheduling reference evaluator.
//!
//! This crate produces recommendations only. It never controls lock hardware,
//! dispatches a vessel, or grants navigation authority.

use std::collections::BTreeMap;

use inland_waterway_lock_operations::{
    EvidenceState, PassageDisposition, PassageEvaluation, TimeInterval,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    Upstream,
    Downstream,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingPolicy {
    FirstComeFirstServed,
    LeastWaitSlack,
    /// Greedily serves the participant with the least accumulated comparable service units.
    /// This is a policy baseline, not a universal fairness guarantee.
    ParticipantLeastServed,
    DirectionalBatch {
        max_consecutive: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockScheduleProfile {
    pub lock_id: String,
    pub lock_revision: String,
    pub planning_window: TimeInterval,
    pub tick_seconds: u64,
    pub service_unit_label: String,
    pub direction_change_ticks: u64,
    pub maximum_arrival_evidence_age_ticks: u64,
    pub unavailable_windows: Vec<TimeInterval>,
    pub water_budget_ml: u64,
    pub pump_energy_budget_wh: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrivalWindowEvidence {
    pub evidence_id: String,
    pub source_id: String,
    pub source_revision: String,
    /// Points to a validation/calibration receipt from the source model owner.
    pub calibration_receipt_id: String,
    /// Exact arrival interval covered by the calibration/validation receipt.
    pub interval_earliest_tick: u64,
    pub interval_latest_tick: u64,
    /// Must identify the operating regime for which the interval is supported.
    pub operating_regime_id: String,
    pub observed_at_tick: u64,
    pub valid_through_tick: Option<u64>,
    pub state: EvidenceState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassageRequest {
    pub request_id: String,
    pub vessel_id: String,
    pub vessel_revision: String,
    pub compatibility_receipt_id: String,
    pub compatibility: PassageEvaluation,
    pub lock_id: String,
    pub lock_revision: String,
    pub participant_id: String,
    pub service_group: String,
    pub direction: Direction,
    /// Earliest and latest possible arrival ticks, inclusive.
    pub arrival_earliest_tick: u64,
    pub arrival_latest_tick: u64,
    pub arrival_evidence: ArrivalWindowEvidence,
    /// Authorized service-start window, inclusive at both ends.
    pub earliest_start_tick: u64,
    pub latest_start_tick: u64,
    pub maximum_wait_ticks: u64,
    pub service_duration_ticks: u64,
    pub service_units: u64,
    pub water_requirement_ml: u64,
    pub pump_energy_requirement_wh: u64,
    /// Optional hard completion deadline, inclusive.
    pub completion_deadline_tick: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileError {
    InvalidIdentity,
    InvalidPlanningWindow,
    InvalidUnavailableWindow,
    OverlappingUnavailableWindows,
    InvalidPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotScheduledReason {
    InvalidRequest,
    DuplicateRequestId,
    LockBindingMismatch,
    PassageBlocked,
    PassageNeedsReview,
    PassageUnsupported,
    ArrivalEvidenceInvalid,
    ArrivalEvidenceNeedsReview,
    ArrivalEvidenceRevoked,
    LatestStartMissed,
    MaximumWaitExceeded,
    CompletionDeadlineMissed,
    PlanningWindowExhausted,
    WaterBudgetExceeded,
    PumpEnergyBudgetExceeded,
    ArithmeticOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanDisposition {
    AllRequestsScheduled,
    PartialSchedule,
    NoFeasibleSchedule,
    NeedsReview,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledPassage {
    pub request_id: String,
    pub vessel_id: String,
    pub participant_id: String,
    pub service_group: String,
    pub direction: Direction,
    pub arrival_earliest_tick: u64,
    pub start_tick: u64,
    pub end_tick: u64,
    pub waiting_from_earliest_arrival_ticks: u64,
    pub service_units: u64,
    pub water_ml: u64,
    pub pump_energy_wh: u64,
    pub direction_change_before_ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestOutcome {
    pub request_id: String,
    pub participant_id: String,
    pub service_group: String,
    pub scheduled: Option<ScheduledPassage>,
    pub reason: Option<NotScheduledReason>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WaitDistribution {
    pub sample_count: u64,
    /// Nearest-rank median (50th percentile).
    pub p50_ticks: Option<u64>,
    /// Nearest-rank 90th percentile.
    pub p90_ticks: Option<u64>,
    /// Nearest-rank 95th percentile.
    pub p95_ticks: Option<u64>,
    /// Nearest-rank 99th percentile.
    pub p99_ticks: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GroupMetrics {
    pub requests: u64,
    pub scheduled: u64,
    pub unserved: u64,
    pub service_units: u128,
    pub total_wait_ticks: u128,
    pub maximum_wait_ticks: u64,
    pub wait_distribution: WaitDistribution,
    pub water_ml: u128,
    pub pump_energy_wh: u128,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ScheduleMetrics {
    pub scheduled_requests: u64,
    pub unserved_requests: u64,
    pub service_units: u128,
    pub total_wait_ticks: u128,
    pub maximum_wait_ticks: u64,
    pub wait_distribution: WaitDistribution,
    pub total_service_ticks: u128,
    pub total_direction_change_ticks: u128,
    pub total_occupied_ticks: u128,
    pub available_service_ticks: u128,
    pub total_water_ml: u128,
    pub total_pump_energy_wh: u128,
    pub direction_changes: u64,
    pub comparable_arrival_pairs: u64,
    pub inversion_pairs: u64,
    pub by_participant: BTreeMap<String, GroupMetrics>,
    pub by_service_group: BTreeMap<String, GroupMetrics>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulePlan {
    pub disposition: PlanDisposition,
    pub policy: SchedulingPolicy,
    pub schedule: Vec<ScheduledPassage>,
    pub outcomes: Vec<RequestOutcome>,
    pub metrics: ScheduleMetrics,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

/// Compute nearest-rank percentiles without interpolating integer simulation ticks.
fn wait_distribution(samples: &mut [u64]) -> WaitDistribution {
    if samples.is_empty() {
        return WaitDistribution::default();
    }
    samples.sort_unstable();
    let nearest_rank = |percent: usize| {
        let rank = samples.len().saturating_mul(percent).div_ceil(100);
        samples[rank.saturating_sub(1)]
    };
    WaitDistribution {
        sample_count: u64::try_from(samples.len()).unwrap_or(u64::MAX),
        p50_ticks: Some(nearest_rank(50)),
        p90_ticks: Some(nearest_rank(90)),
        p95_ticks: Some(nearest_rank(95)),
        p99_ticks: Some(nearest_rank(99)),
    }
}

fn validate_profile(
    profile: &LockScheduleProfile,
    policy: SchedulingPolicy,
) -> Result<Vec<TimeInterval>, ProfileError> {
    if !nonempty(&profile.lock_id)
        || !nonempty(&profile.lock_revision)
        || !nonempty(&profile.service_unit_label)
        || profile.tick_seconds == 0
    {
        return Err(ProfileError::InvalidIdentity);
    }
    if profile.planning_window.start_tick >= profile.planning_window.end_tick {
        return Err(ProfileError::InvalidPlanningWindow);
    }
    if matches!(
        policy,
        SchedulingPolicy::DirectionalBatch { max_consecutive: 0 }
    ) {
        return Err(ProfileError::InvalidPolicy);
    }

    let mut unavailable = profile.unavailable_windows.clone();
    unavailable.sort_by_key(|window| (window.start_tick, window.end_tick));
    for (index, window) in unavailable.iter().enumerate() {
        if window.start_tick >= window.end_tick {
            return Err(ProfileError::InvalidUnavailableWindow);
        }
        if index > 0 && unavailable[index - 1].end_tick > window.start_tick {
            return Err(ProfileError::OverlappingUnavailableWindows);
        }
    }
    Ok(unavailable)
}

fn arrival_evidence_error(
    profile: &LockScheduleProfile,
    request: &PassageRequest,
) -> Option<NotScheduledReason> {
    let evidence = &request.arrival_evidence;
    if evidence.interval_earliest_tick != request.arrival_earliest_tick
        || evidence.interval_latest_tick != request.arrival_latest_tick
    {
        return Some(NotScheduledReason::ArrivalEvidenceInvalid);
    }
    if !nonempty(&evidence.evidence_id)
        || !nonempty(&evidence.source_id)
        || !nonempty(&evidence.source_revision)
        || !nonempty(&evidence.calibration_receipt_id)
        || !nonempty(&evidence.operating_regime_id)
    {
        return Some(NotScheduledReason::ArrivalEvidenceInvalid);
    }

    match evidence.state {
        EvidenceState::Current => {}
        EvidenceState::Unknown | EvidenceState::Disputed => {
            return Some(NotScheduledReason::ArrivalEvidenceNeedsReview);
        }
        EvidenceState::Revoked => {
            return Some(NotScheduledReason::ArrivalEvidenceRevoked);
        }
    }

    let decision_tick = profile.planning_window.start_tick;
    if evidence.observed_at_tick > decision_tick {
        return Some(NotScheduledReason::ArrivalEvidenceInvalid);
    }
    if decision_tick - evidence.observed_at_tick > profile.maximum_arrival_evidence_age_ticks {
        return Some(NotScheduledReason::ArrivalEvidenceNeedsReview);
    }
    match evidence.valid_through_tick {
        Some(valid_through) if valid_through >= decision_tick => None,
        Some(_) | None => Some(NotScheduledReason::ArrivalEvidenceNeedsReview),
    }
}

fn request_error(
    profile: &LockScheduleProfile,
    request: &PassageRequest,
) -> Option<NotScheduledReason> {
    if !nonempty(&request.request_id)
        || !nonempty(&request.vessel_id)
        || !nonempty(&request.vessel_revision)
        || !nonempty(&request.compatibility_receipt_id)
        || !nonempty(&request.participant_id)
        || !nonempty(&request.service_group)
        || request.arrival_earliest_tick > request.arrival_latest_tick
        || request.earliest_start_tick > request.latest_start_tick
        || request.service_duration_ticks == 0
        || request.service_units == 0
    {
        return Some(NotScheduledReason::InvalidRequest);
    }

    if request.lock_id != profile.lock_id || request.lock_revision != profile.lock_revision {
        return Some(NotScheduledReason::LockBindingMismatch);
    }

    if let Some(reason) = arrival_evidence_error(profile, request) {
        return Some(reason);
    }

    match request.compatibility.disposition {
        PassageDisposition::EligibleForSyntheticPassage => None,
        PassageDisposition::BlockedByHardConstraint => Some(NotScheduledReason::PassageBlocked),
        PassageDisposition::UnknownReviewRequired | PassageDisposition::EvidenceInconsistent => {
            Some(NotScheduledReason::PassageNeedsReview)
        }
        PassageDisposition::UnsupportedOperatingProfile => {
            Some(NotScheduledReason::PassageUnsupported)
        }
    }
}

fn find_start(
    profile: &LockScheduleProfile,
    unavailable: &[TimeInterval],
    earliest_tick: u64,
    duration_ticks: u64,
) -> Option<u64> {
    let mut candidate = earliest_tick.max(profile.planning_window.start_tick);
    loop {
        let end = candidate.checked_add(duration_ticks)?;
        if end > profile.planning_window.end_tick {
            return None;
        }
        let obstruction = unavailable
            .iter()
            .find(|window| candidate < window.end_tick && window.start_tick < end);
        match obstruction {
            Some(window) => candidate = window.end_tick,
            None => return Some(candidate),
        }
    }
}

fn oldest_first(requests: &[PassageRequest], pending: &[usize]) -> Option<usize> {
    pending.iter().copied().min_by_key(|index| {
        (
            requests[*index].arrival_earliest_tick,
            requests[*index].arrival_latest_tick,
            requests[*index].request_id.as_str(),
        )
    })
}

fn latest_permitted_start(request: &PassageRequest) -> u128 {
    let wait_limit =
        u128::from(request.arrival_earliest_tick) + u128::from(request.maximum_wait_ticks);
    let deadline_limit = request
        .completion_deadline_tick
        .map(|deadline| deadline.saturating_sub(request.service_duration_ticks))
        .map(u128::from)
        .unwrap_or(u128::MAX);
    u128::from(request.latest_start_tick)
        .min(wait_limit)
        .min(deadline_limit)
}

fn least_slack_first(
    profile: &LockScheduleProfile,
    unavailable: &[TimeInterval],
    requests: &[PassageRequest],
    pending: &[usize],
    current_direction: Option<Direction>,
    cursor_tick: u64,
) -> Option<usize> {
    pending.iter().copied().min_by_key(|index| {
        let request = &requests[*index];
        let direction_change =
            if current_direction.is_some_and(|direction| direction != request.direction) {
                profile.direction_change_ticks
            } else {
                0
            };
        let projected =
            projected_start(profile, unavailable, cursor_tick, request, direction_change);
        let slack = projected
            .map(|start| latest_permitted_start(request).saturating_sub(u128::from(start)))
            .unwrap_or(u128::MAX);
        (
            slack,
            latest_permitted_start(request),
            request.arrival_earliest_tick,
            request.request_id.as_str(),
        )
    })
}

fn projected_start(
    profile: &LockScheduleProfile,
    unavailable: &[TimeInterval],
    cursor_tick: u64,
    request: &PassageRequest,
    direction_change_ticks: u64,
) -> Option<u64> {
    let after_change = cursor_tick.checked_add(direction_change_ticks)?;
    let earliest = after_change
        .max(request.arrival_latest_tick)
        .max(request.earliest_start_tick);
    find_start(
        profile,
        unavailable,
        earliest,
        request.service_duration_ticks,
    )
}

struct SelectionContext<'a> {
    profile: &'a LockScheduleProfile,
    unavailable: &'a [TimeInterval],
    requests: &'a [PassageRequest],
    pending: &'a [usize],
    current_direction: Option<Direction>,
    current_batch_count: usize,
    cursor_tick: u64,
    served_service_units_by_participant: &'a BTreeMap<String, u128>,
}

fn oldest_request_for_each_participant(
    requests: &[PassageRequest],
    pending: &[usize],
) -> BTreeMap<String, usize> {
    let mut oldest_by_participant = BTreeMap::<String, usize>::new();
    for index in pending.iter().copied() {
        let request = &requests[index];
        match oldest_by_participant.get(&request.participant_id) {
            Some(existing_index) => {
                let existing = &requests[*existing_index];
                let candidate_key = (
                    request.arrival_earliest_tick,
                    request.arrival_latest_tick,
                    request.request_id.as_str(),
                );
                let existing_key = (
                    existing.arrival_earliest_tick,
                    existing.arrival_latest_tick,
                    existing.request_id.as_str(),
                );
                if candidate_key < existing_key {
                    oldest_by_participant.insert(request.participant_id.clone(), index);
                }
            }
            None => {
                oldest_by_participant.insert(request.participant_id.clone(), index);
            }
        }
    }
    oldest_by_participant
}

fn participant_least_service_first(
    requests: &[PassageRequest],
    pending: &[usize],
    served_service_units_by_participant: &BTreeMap<String, u128>,
) -> Option<usize> {
    oldest_request_for_each_participant(requests, pending)
        .into_values()
        .min_by_key(|index| {
            let request = &requests[*index];
            (
                served_service_units_by_participant
                    .get(&request.participant_id)
                    .copied()
                    .unwrap_or(0),
                request.arrival_earliest_tick,
                request.arrival_latest_tick,
                request.participant_id.as_str(),
                request.request_id.as_str(),
            )
        })
}

fn select_next(context: &SelectionContext<'_>, policy: SchedulingPolicy) -> Option<usize> {
    let profile = context.profile;
    let unavailable = context.unavailable;
    let requests = context.requests;
    let pending = context.pending;
    let current_direction = context.current_direction;
    let current_batch_count = context.current_batch_count;
    let cursor_tick = context.cursor_tick;

    match policy {
        SchedulingPolicy::FirstComeFirstServed => oldest_first(requests, pending),
        SchedulingPolicy::LeastWaitSlack => least_slack_first(
            profile,
            unavailable,
            requests,
            pending,
            current_direction,
            cursor_tick,
        ),
        SchedulingPolicy::ParticipantLeastServed => participant_least_service_first(
            requests,
            pending,
            context.served_service_units_by_participant,
        ),
        SchedulingPolicy::DirectionalBatch { max_consecutive } => {
            let Some(direction) = current_direction else {
                return oldest_first(requests, pending);
            };
            if current_batch_count >= max_consecutive {
                let opposite_direction: Vec<usize> = pending
                    .iter()
                    .copied()
                    .filter(|index| requests[*index].direction != direction)
                    .collect();
                return oldest_first(requests, &opposite_direction)
                    .or_else(|| oldest_first(requests, pending));
            }

            let same_direction: Vec<usize> = pending
                .iter()
                .copied()
                .filter(|index| requests[*index].direction == direction)
                .collect();
            let opposite_direction: Vec<usize> = pending
                .iter()
                .copied()
                .filter(|index| requests[*index].direction != direction)
                .collect();

            let Some(same_index) = oldest_first(requests, &same_direction) else {
                return oldest_first(requests, pending);
            };
            let Some(opposite_index) = oldest_first(requests, &opposite_direction) else {
                return Some(same_index);
            };

            let same = &requests[same_index];
            let opposite = &requests[opposite_index];
            let Some(same_start) = projected_start(profile, unavailable, cursor_tick, same, 0)
            else {
                return oldest_first(requests, pending);
            };
            let same_end = same_start.checked_add(same.service_duration_ticks)?;
            let Some(opposite_start) = projected_start(
                profile,
                unavailable,
                same_end,
                opposite,
                profile.direction_change_ticks,
            ) else {
                return Some(same_index);
            };
            let projected_wait = opposite_start.saturating_sub(opposite.arrival_earliest_tick);
            let projected_end = opposite_start.checked_add(opposite.service_duration_ticks)?;
            let would_breach_wait = projected_wait > opposite.maximum_wait_ticks;
            let would_breach_deadline = opposite
                .completion_deadline_tick
                .is_some_and(|deadline| projected_end > deadline);

            if would_breach_wait || would_breach_deadline {
                Some(opposite_index)
            } else {
                Some(same_index)
            }
        }
    }
}

fn available_service_ticks(profile: &LockScheduleProfile, unavailable: &[TimeInterval]) -> u128 {
    let mut unavailable_ticks = 0_u128;
    for window in unavailable {
        let start = window.start_tick.max(profile.planning_window.start_tick);
        let end = window.end_tick.min(profile.planning_window.end_tick);
        if start < end {
            unavailable_ticks += u128::from(end - start);
        }
    }
    u128::from(profile.planning_window.end_tick - profile.planning_window.start_tick)
        .saturating_sub(unavailable_ticks)
}

fn add_group_metrics(
    groups: &mut BTreeMap<String, GroupMetrics>,
    wait_samples: &mut BTreeMap<String, Vec<u64>>,
    key: &str,
    outcome: &RequestOutcome,
) {
    let group = groups.entry(key.to_owned()).or_default();
    group.requests += 1;
    match &outcome.scheduled {
        Some(passage) => {
            group.scheduled += 1;
            group.service_units += u128::from(passage.service_units);
            group.total_wait_ticks += u128::from(passage.waiting_from_earliest_arrival_ticks);
            group.maximum_wait_ticks = group
                .maximum_wait_ticks
                .max(passage.waiting_from_earliest_arrival_ticks);
            wait_samples
                .entry(key.to_owned())
                .or_default()
                .push(passage.waiting_from_earliest_arrival_ticks);
            group.water_ml += u128::from(passage.water_ml);
            group.pump_energy_wh += u128::from(passage.pump_energy_wh);
        }
        None => group.unserved += 1,
    }
}

fn build_metrics(
    profile: &LockScheduleProfile,
    unavailable: &[TimeInterval],
    schedule: &[ScheduledPassage],
    outcomes: &[RequestOutcome],
) -> ScheduleMetrics {
    let mut metrics = ScheduleMetrics {
        available_service_ticks: available_service_ticks(profile, unavailable),
        ..ScheduleMetrics::default()
    };

    let mut global_wait_samples = Vec::with_capacity(schedule.len());
    let mut previous_direction = None;
    for passage in schedule {
        metrics.scheduled_requests += 1;
        metrics.service_units += u128::from(passage.service_units);
        metrics.total_wait_ticks += u128::from(passage.waiting_from_earliest_arrival_ticks);
        global_wait_samples.push(passage.waiting_from_earliest_arrival_ticks);
        metrics.maximum_wait_ticks = metrics
            .maximum_wait_ticks
            .max(passage.waiting_from_earliest_arrival_ticks);
        let service_ticks = u128::from(passage.end_tick - passage.start_tick);
        let direction_change_ticks = u128::from(passage.direction_change_before_ticks);
        metrics.total_service_ticks += service_ticks;
        metrics.total_direction_change_ticks += direction_change_ticks;
        metrics.total_occupied_ticks += service_ticks + direction_change_ticks;
        metrics.total_water_ml += u128::from(passage.water_ml);
        metrics.total_pump_energy_wh += u128::from(passage.pump_energy_wh);
        if previous_direction.is_some_and(|direction| direction != passage.direction) {
            metrics.direction_changes += 1;
        }
        previous_direction = Some(passage.direction);
    }

    let mut participant_wait_samples = BTreeMap::<String, Vec<u64>>::new();
    let mut service_wait_samples = BTreeMap::<String, Vec<u64>>::new();
    for outcome in outcomes {
        if outcome.scheduled.is_none() {
            metrics.unserved_requests += 1;
        }
        add_group_metrics(
            &mut metrics.by_participant,
            &mut participant_wait_samples,
            &outcome.participant_id,
            outcome,
        );
        add_group_metrics(
            &mut metrics.by_service_group,
            &mut service_wait_samples,
            &outcome.service_group,
            outcome,
        );
    }

    metrics.wait_distribution = wait_distribution(&mut global_wait_samples);
    for (key, samples) in &mut participant_wait_samples {
        if let Some(group) = metrics.by_participant.get_mut(key) {
            group.wait_distribution = wait_distribution(samples);
        }
    }
    for (key, samples) in &mut service_wait_samples {
        if let Some(group) = metrics.by_service_group.get_mut(key) {
            group.wait_distribution = wait_distribution(samples);
        }
    }

    for (left, first) in schedule.iter().enumerate() {
        for second in schedule.iter().skip(left + 1) {
            let first_arrival = (first.arrival_earliest_tick, first.request_id.as_str());
            let second_arrival = (second.arrival_earliest_tick, second.request_id.as_str());
            metrics.comparable_arrival_pairs += 1;
            if first_arrival > second_arrival {
                metrics.inversion_pairs += 1;
            }
        }
    }

    metrics
}

/// Create a deterministic recommendation over a single lock and chamber.
///
/// Request input order does not affect canonical schedule or outcomes. The
/// returned plan records every request, including blocked and indeterminate ones.
pub fn schedule(
    profile: &LockScheduleProfile,
    requests: &[PassageRequest],
    policy: SchedulingPolicy,
) -> Result<SchedulePlan, ProfileError> {
    let unavailable = validate_profile(profile, policy)?;
    let mut outcomes = Vec::with_capacity(requests.len());
    let mut request_ids = BTreeMap::<String, usize>::new();
    for request in requests {
        *request_ids.entry(request.request_id.clone()).or_default() += 1;
    }

    let mut pending = Vec::new();
    for (index, request) in requests.iter().enumerate() {
        let reason = if request_ids.get(&request.request_id).copied().unwrap_or(0) > 1 {
            Some(NotScheduledReason::DuplicateRequestId)
        } else {
            request_error(profile, request)
        };
        match reason {
            Some(reason) => outcomes.push(RequestOutcome {
                request_id: request.request_id.clone(),
                participant_id: request.participant_id.clone(),
                service_group: request.service_group.clone(),
                scheduled: None,
                reason: Some(reason),
            }),
            None => pending.push(index),
        }
    }

    let mut schedule = Vec::new();
    let mut cursor_tick = profile.planning_window.start_tick;
    let mut current_direction = None;
    let mut current_batch_count = 0_usize;
    let mut water_used = 0_u128;
    let mut energy_used = 0_u128;
    let mut served_service_units_by_participant = BTreeMap::<String, u128>::new();

    while !pending.is_empty() {
        let selection = SelectionContext {
            profile,
            unavailable: &unavailable,
            requests,
            pending: &pending,
            current_direction,
            current_batch_count,
            cursor_tick,
            served_service_units_by_participant: &served_service_units_by_participant,
        };
        let Some(selected_index) = select_next(&selection, policy) else {
            for index in pending.drain(..) {
                let request = &requests[index];
                outcomes.push(RequestOutcome {
                    request_id: request.request_id.clone(),
                    participant_id: request.participant_id.clone(),
                    service_group: request.service_group.clone(),
                    scheduled: None,
                    reason: Some(NotScheduledReason::PlanningWindowExhausted),
                });
            }
            break;
        };

        let Some(position) = pending.iter().position(|index| *index == selected_index) else {
            return Err(ProfileError::InvalidPolicy);
        };
        let index = pending.remove(position);
        let request = &requests[index];

        let direction_change =
            if current_direction.is_some_and(|direction| direction != request.direction) {
                profile.direction_change_ticks
            } else {
                0
            };
        let Some(after_direction_change) = cursor_tick.checked_add(direction_change) else {
            outcomes.push(RequestOutcome {
                request_id: request.request_id.clone(),
                participant_id: request.participant_id.clone(),
                service_group: request.service_group.clone(),
                scheduled: None,
                reason: Some(NotScheduledReason::ArithmeticOverflow),
            });
            continue;
        };
        let earliest = after_direction_change
            .max(request.arrival_latest_tick)
            .max(request.earliest_start_tick);
        let Some(start_tick) = find_start(
            profile,
            &unavailable,
            earliest,
            request.service_duration_ticks,
        ) else {
            outcomes.push(RequestOutcome {
                request_id: request.request_id.clone(),
                participant_id: request.participant_id.clone(),
                service_group: request.service_group.clone(),
                scheduled: None,
                reason: Some(NotScheduledReason::PlanningWindowExhausted),
            });
            continue;
        };
        let Some(end_tick) = start_tick.checked_add(request.service_duration_ticks) else {
            outcomes.push(RequestOutcome {
                request_id: request.request_id.clone(),
                participant_id: request.participant_id.clone(),
                service_group: request.service_group.clone(),
                scheduled: None,
                reason: Some(NotScheduledReason::ArithmeticOverflow),
            });
            continue;
        };

        let wait_ticks = start_tick.saturating_sub(request.arrival_earliest_tick);
        let reason = if start_tick > request.latest_start_tick {
            Some(NotScheduledReason::LatestStartMissed)
        } else if wait_ticks > request.maximum_wait_ticks {
            Some(NotScheduledReason::MaximumWaitExceeded)
        } else if request
            .completion_deadline_tick
            .is_some_and(|deadline| end_tick > deadline)
        {
            Some(NotScheduledReason::CompletionDeadlineMissed)
        } else if water_used + u128::from(request.water_requirement_ml)
            > u128::from(profile.water_budget_ml)
        {
            Some(NotScheduledReason::WaterBudgetExceeded)
        } else if energy_used + u128::from(request.pump_energy_requirement_wh)
            > u128::from(profile.pump_energy_budget_wh)
        {
            Some(NotScheduledReason::PumpEnergyBudgetExceeded)
        } else {
            None
        };

        if let Some(reason) = reason {
            outcomes.push(RequestOutcome {
                request_id: request.request_id.clone(),
                participant_id: request.participant_id.clone(),
                service_group: request.service_group.clone(),
                scheduled: None,
                reason: Some(reason),
            });
            continue;
        }

        water_used += u128::from(request.water_requirement_ml);
        energy_used += u128::from(request.pump_energy_requirement_wh);
        let served_units = served_service_units_by_participant
            .entry(request.participant_id.clone())
            .or_default();
        *served_units = served_units.saturating_add(u128::from(request.service_units));
        let passage = ScheduledPassage {
            request_id: request.request_id.clone(),
            vessel_id: request.vessel_id.clone(),
            participant_id: request.participant_id.clone(),
            service_group: request.service_group.clone(),
            direction: request.direction,
            arrival_earliest_tick: request.arrival_earliest_tick,
            start_tick,
            end_tick,
            waiting_from_earliest_arrival_ticks: wait_ticks,
            service_units: request.service_units,
            water_ml: request.water_requirement_ml,
            pump_energy_wh: request.pump_energy_requirement_wh,
            direction_change_before_ticks: direction_change,
        };
        schedule.push(passage.clone());
        outcomes.push(RequestOutcome {
            request_id: request.request_id.clone(),
            participant_id: request.participant_id.clone(),
            service_group: request.service_group.clone(),
            scheduled: Some(passage),
            reason: None,
        });
        cursor_tick = end_tick;
        if current_direction == Some(request.direction) {
            current_batch_count += 1;
        } else {
            current_direction = Some(request.direction);
            current_batch_count = 1;
        }
    }

    schedule.sort_by_key(|passage| (passage.start_tick, passage.request_id.clone()));
    outcomes.sort_by(|left, right| left.request_id.cmp(&right.request_id));

    let metrics = build_metrics(profile, &unavailable, &schedule, &outcomes);
    let disposition = if outcomes.iter().any(|outcome| {
        matches!(
            outcome.reason,
            Some(
                NotScheduledReason::PassageNeedsReview
                    | NotScheduledReason::ArrivalEvidenceNeedsReview
                    | NotScheduledReason::ArrivalEvidenceInvalid
            )
        )
    }) {
        PlanDisposition::NeedsReview
    } else if metrics.unserved_requests == 0 {
        PlanDisposition::AllRequestsScheduled
    } else if metrics.scheduled_requests == 0 {
        PlanDisposition::NoFeasibleSchedule
    } else {
        PlanDisposition::PartialSchedule
    };

    Ok(SchedulePlan {
        disposition,
        policy,
        schedule,
        outcomes,
        metrics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use inland_waterway_lock_operations::PassageEvaluation;

    fn eligible() -> PassageEvaluation {
        PassageEvaluation {
            disposition: PassageDisposition::EligibleForSyntheticPassage,
            reasons: Vec::new(),
            required_length_mm: 10,
            required_beam_mm: 5,
            required_depth_mm: 4,
        }
    }

    fn profile() -> LockScheduleProfile {
        LockScheduleProfile {
            lock_id: "lock-01".into(),
            lock_revision: "rev-01".into(),
            planning_window: TimeInterval {
                start_tick: 0,
                end_tick: 1_000,
            },
            tick_seconds: 60,
            service_unit_label: "tonne".into(),
            direction_change_ticks: 5,
            maximum_arrival_evidence_age_ticks: 20,
            unavailable_windows: vec![TimeInterval {
                start_tick: 50,
                end_tick: 60,
            }],
            water_budget_ml: 10_000,
            pump_energy_budget_wh: 5_000,
        }
    }

    fn request(
        request_id: &str,
        arrival: u64,
        direction: Direction,
        water: u64,
        energy: u64,
    ) -> PassageRequest {
        PassageRequest {
            request_id: request_id.into(),
            vessel_id: format!("vessel-{request_id}"),
            vessel_revision: "v1".into(),
            compatibility_receipt_id: format!("compat-{request_id}"),
            compatibility: eligible(),
            lock_id: "lock-01".into(),
            lock_revision: "rev-01".into(),
            participant_id: format!("participant-{request_id}"),
            service_group: "freight".into(),
            direction,
            arrival_earliest_tick: arrival,
            arrival_latest_tick: arrival,
            arrival_evidence: ArrivalWindowEvidence {
                evidence_id: format!("arrival-evidence-{request_id}"),
                source_id: "arrival-model".into(),
                source_revision: "model-v1".into(),
                calibration_receipt_id: "calibration-v1".into(),
                interval_earliest_tick: arrival,
                interval_latest_tick: arrival,
                operating_regime_id: "regime-normal".into(),
                observed_at_tick: 0,
                valid_through_tick: Some(1_000),
                state: EvidenceState::Current,
            },
            earliest_start_tick: arrival,
            latest_start_tick: 900,
            maximum_wait_ticks: 500,
            service_duration_ticks: 10,
            service_units: 10,
            water_requirement_ml: water,
            pump_energy_requirement_wh: energy,
            completion_deadline_tick: Some(950),
        }
    }

    #[test]
    fn fcfs_is_deterministic_under_input_permutation() {
        let requests = vec![
            request("b", 20, Direction::Upstream, 10, 10),
            request("a", 10, Direction::Upstream, 10, 10),
        ];
        let forward = schedule(
            &profile(),
            &requests,
            SchedulingPolicy::FirstComeFirstServed,
        )
        .expect("valid profile");
        let reverse = schedule(
            &profile(),
            &requests.iter().cloned().rev().collect::<Vec<_>>(),
            SchedulingPolicy::FirstComeFirstServed,
        )
        .expect("valid profile");
        assert_eq!(forward, reverse);
        assert_eq!(forward.outcomes.len(), requests.len());
        assert!(
            forward
                .outcomes
                .iter()
                .all(|outcome| outcome.scheduled.is_some())
        );
        assert_eq!(forward.schedule[0].request_id, "a");
        assert_eq!(forward.schedule[1].request_id, "b");
    }

    #[test]
    fn passage_unknown_or_blocked_is_not_scheduled() {
        let mut unknown = request("unknown", 0, Direction::Upstream, 1, 1);
        unknown.compatibility.disposition = PassageDisposition::UnknownReviewRequired;
        let mut blocked = request("blocked", 0, Direction::Upstream, 1, 1);
        blocked.compatibility.disposition = PassageDisposition::BlockedByHardConstraint;
        let plan = schedule(
            &profile(),
            &[unknown, blocked],
            SchedulingPolicy::FirstComeFirstServed,
        )
        .expect("valid profile");
        assert!(plan.schedule.is_empty());
        assert_eq!(plan.disposition, PlanDisposition::NeedsReview);
        assert_eq!(
            plan.outcomes[0].reason,
            Some(NotScheduledReason::PassageBlocked)
        );
        assert_eq!(
            plan.outcomes[1].reason,
            Some(NotScheduledReason::PassageNeedsReview)
        );
    }

    #[test]
    fn lock_revision_mismatch_is_not_scheduled() {
        let mut mismatched = request("v", 0, Direction::Upstream, 1, 1);
        mismatched.lock_revision = "old-rev".into();
        let plan = schedule(
            &profile(),
            &[mismatched],
            SchedulingPolicy::FirstComeFirstServed,
        )
        .expect("valid profile");
        assert!(plan.schedule.is_empty());
        assert_eq!(
            plan.outcomes[0].reason,
            Some(NotScheduledReason::LockBindingMismatch)
        );
    }

    #[test]
    fn unavailable_window_is_respected_using_half_open_intervals() {
        let plan = schedule(
            &profile(),
            &[request("a", 50, Direction::Upstream, 1, 1)],
            SchedulingPolicy::FirstComeFirstServed,
        )
        .expect("valid profile");
        assert_eq!(plan.schedule[0].start_tick, 60);
        assert_eq!(plan.schedule[0].end_tick, 70);
    }

    #[test]
    fn water_and_energy_budgets_are_separate_hard_constraints() {
        let mut p = profile();
        p.water_budget_ml = 15;
        p.pump_energy_budget_wh = 5;
        let plan = schedule(
            &p,
            &[
                request("a", 0, Direction::Upstream, 10, 2),
                request("b", 20, Direction::Upstream, 10, 2),
                request("c", 40, Direction::Upstream, 2, 10),
            ],
            SchedulingPolicy::FirstComeFirstServed,
        )
        .expect("valid profile");
        assert_eq!(plan.metrics.scheduled_requests, 1);
        assert_eq!(
            plan.outcomes[1].reason,
            Some(NotScheduledReason::WaterBudgetExceeded)
        );
        assert_eq!(
            plan.outcomes[2].reason,
            Some(NotScheduledReason::PumpEnergyBudgetExceeded)
        );
    }

    #[test]
    fn least_slack_policy_serves_the_tighter_wait_contract_first() {
        let mut old = request("old", 0, Direction::Upstream, 1, 1);
        old.maximum_wait_ticks = 100;
        let mut urgent = request("urgent", 10, Direction::Upstream, 1, 1);
        urgent.maximum_wait_ticks = 5;
        let plan = schedule(&profile(), &[old, urgent], SchedulingPolicy::LeastWaitSlack)
            .expect("valid profile");
        assert_eq!(plan.schedule[0].request_id, "urgent");
    }

    #[test]
    fn least_slack_policy_accounts_for_completion_deadline_and_service_duration() {
        let mut relaxed = request("relaxed", 0, Direction::Upstream, 1, 1);
        relaxed.latest_start_tick = 800;
        relaxed.maximum_wait_ticks = 500;
        relaxed.completion_deadline_tick = Some(200);

        let mut urgent = request("urgent", 5, Direction::Upstream, 1, 1);
        urgent.latest_start_tick = 700;
        urgent.maximum_wait_ticks = 500;
        urgent.service_duration_ticks = 20;
        urgent.completion_deadline_tick = Some(30);

        let plan = schedule(
            &profile(),
            &[relaxed, urgent],
            SchedulingPolicy::LeastWaitSlack,
        )
        .expect("valid profile");
        assert_eq!(plan.schedule[0].request_id, "urgent");
    }

    #[test]
    fn wait_distribution_uses_nearest_rank_percentiles() {
        let mut p = profile();
        p.unavailable_windows.clear();
        let requests = vec![
            request("a", 0, Direction::Upstream, 1, 1),
            request("b", 1, Direction::Upstream, 1, 1),
            request("c", 2, Direction::Upstream, 1, 1),
            request("d", 3, Direction::Upstream, 1, 1),
        ];
        let plan =
            schedule(&p, &requests, SchedulingPolicy::FirstComeFirstServed).expect("valid profile");
        let dist = plan.metrics.wait_distribution;
        assert_eq!(dist.sample_count, 4);
        assert_eq!(dist.p50_ticks, Some(9));
        assert_eq!(dist.p90_ticks, Some(27));
        assert_eq!(dist.p95_ticks, Some(27));
        assert_eq!(dist.p99_ticks, Some(27));
        assert_eq!(
            plan.metrics.by_service_group["freight"].wait_distribution,
            dist
        );
    }

    #[test]
    fn participant_least_served_rotates_before_a_second_request_from_same_participant() {
        let mut p = profile();
        p.unavailable_windows.clear();
        let mut a_first = request("a-first", 0, Direction::Upstream, 1, 1);
        a_first.participant_id = "participant-a".into();
        let mut a_second = request("a-second", 1, Direction::Upstream, 1, 1);
        a_second.participant_id = "participant-a".into();
        let mut b_first = request("b-first", 2, Direction::Upstream, 1, 1);
        b_first.participant_id = "participant-b".into();
        let requests = vec![a_first, a_second, b_first];
        let plan = schedule(&p, &requests, SchedulingPolicy::ParticipantLeastServed)
            .expect("valid profile");
        assert_eq!(
            plan.schedule
                .iter()
                .map(|passage| passage.request_id.as_str())
                .collect::<Vec<_>>(),
            vec!["a-first", "b-first", "a-second"]
        );
        assert_eq!(
            plan.metrics.by_participant["participant-a"].service_units,
            20
        );
        assert_eq!(
            plan.metrics.by_participant["participant-b"].service_units,
            10
        );
    }

    #[test]
    fn participant_least_served_is_input_order_independent() {
        let mut p = profile();
        p.unavailable_windows.clear();
        let mut a_first = request("a-first", 0, Direction::Upstream, 1, 1);
        a_first.participant_id = "participant-a".into();
        let mut a_second = request("a-second", 1, Direction::Upstream, 1, 1);
        a_second.participant_id = "participant-a".into();
        let mut b_first = request("b-first", 2, Direction::Upstream, 1, 1);
        b_first.participant_id = "participant-b".into();
        let requests = vec![a_first, a_second, b_first];
        let forward = schedule(&p, &requests, SchedulingPolicy::ParticipantLeastServed)
            .expect("valid profile");
        let reverse = schedule(
            &p,
            &requests.iter().cloned().rev().collect::<Vec<_>>(),
            SchedulingPolicy::ParticipantLeastServed,
        )
        .expect("valid profile");
        assert_eq!(forward, reverse);
    }

    #[test]
    fn directional_batch_switches_before_breaching_opposite_queue_wait() {
        let mut p = profile();
        p.unavailable_windows.clear();
        let mut same_a = request("up-a", 0, Direction::Upstream, 1, 1);
        same_a.maximum_wait_ticks = 500;
        let mut same_b = request("up-b", 1, Direction::Upstream, 1, 1);
        same_b.maximum_wait_ticks = 500;
        let mut opposite = request("down-a", 2, Direction::Downstream, 1, 1);
        opposite.maximum_wait_ticks = 20;
        let plan = schedule(
            &p,
            &[same_a, same_b, opposite],
            SchedulingPolicy::DirectionalBatch { max_consecutive: 4 },
        )
        .expect("valid profile");
        assert_eq!(plan.schedule[0].request_id, "up-a");
        assert_eq!(plan.schedule[1].request_id, "down-a");
        assert_eq!(plan.schedule[2].request_id, "up-b");
    }

    #[test]
    fn directional_batch_respects_the_consecutive_passage_limit() {
        let mut p = profile();
        p.unavailable_windows.clear();
        let first = request("up-first", 0, Direction::Upstream, 1, 1);
        let second = request("up-second", 1, Direction::Upstream, 1, 1);
        let opposite = request("down", 100, Direction::Downstream, 1, 1);
        let plan = schedule(
            &p,
            &[first, second, opposite],
            SchedulingPolicy::DirectionalBatch { max_consecutive: 1 },
        )
        .expect("valid profile");
        assert_eq!(plan.schedule[0].request_id, "up-first");
        assert_eq!(plan.schedule[1].request_id, "down");
        assert_eq!(plan.schedule[2].request_id, "up-second");
    }

    #[test]
    fn duplicate_request_ids_are_all_rejected() {
        let duplicate = request("same", 0, Direction::Upstream, 1, 1);
        let plan = schedule(
            &profile(),
            &[duplicate.clone(), duplicate],
            SchedulingPolicy::FirstComeFirstServed,
        )
        .expect("valid profile");
        assert!(plan.schedule.is_empty());
        assert!(
            plan.outcomes
                .iter()
                .all(|outcome| { outcome.reason == Some(NotScheduledReason::DuplicateRequestId) })
        );
    }

    #[test]
    fn empty_or_overlapping_profile_windows_fail_closed() {
        let mut p = profile();
        p.planning_window.end_tick = p.planning_window.start_tick;
        assert_eq!(
            schedule(&p, &[], SchedulingPolicy::FirstComeFirstServed),
            Err(ProfileError::InvalidPlanningWindow)
        );

        let mut p = profile();
        p.unavailable_windows.push(TimeInterval {
            start_tick: 55,
            end_tick: 65,
        });
        assert_eq!(
            schedule(&p, &[], SchedulingPolicy::FirstComeFirstServed),
            Err(ProfileError::OverlappingUnavailableWindows)
        );
    }

    #[test]
    fn maximum_wait_is_a_hard_constraint() {
        let mut p = profile();
        p.unavailable_windows.clear();
        let first = request("first", 0, Direction::Upstream, 1, 1);
        let mut second = request("second", 1, Direction::Upstream, 1, 1);
        second.maximum_wait_ticks = 5;
        let plan = schedule(&p, &[first, second], SchedulingPolicy::FirstComeFirstServed)
            .expect("valid profile");
        assert_eq!(plan.schedule.len(), 1);
        assert_eq!(
            plan.outcomes
                .iter()
                .find(|outcome| outcome.request_id == "second")
                .and_then(|outcome| outcome.reason),
            Some(NotScheduledReason::MaximumWaitExceeded)
        );
    }

    #[test]
    fn latest_start_and_completion_deadline_are_hard_constraints() {
        let mut p = profile();
        p.unavailable_windows.clear();
        let first = request("first", 0, Direction::Upstream, 1, 1);
        let mut late_start = request("late-start", 1, Direction::Upstream, 1, 1);
        late_start.latest_start_tick = 5;
        let mut late_completion = request("late-completion", 2, Direction::Upstream, 1, 1);
        late_completion.completion_deadline_tick = Some(15);
        let plan = schedule(
            &p,
            &[first, late_start, late_completion],
            SchedulingPolicy::FirstComeFirstServed,
        )
        .expect("valid profile");
        assert_eq!(plan.schedule.len(), 1);
        assert_eq!(
            plan.outcomes
                .iter()
                .find(|outcome| outcome.request_id == "late-start")
                .and_then(|outcome| outcome.reason),
            Some(NotScheduledReason::LatestStartMissed)
        );
        assert_eq!(
            plan.outcomes
                .iter()
                .find(|outcome| outcome.request_id == "late-completion")
                .and_then(|outcome| outcome.reason),
            Some(NotScheduledReason::CompletionDeadlineMissed)
        );
    }

    #[test]
    fn arrival_uncertainty_is_scheduled_at_the_latest_arrival_bound() {
        let mut p = profile();
        p.unavailable_windows.clear();
        let mut uncertain = request("uncertain", 0, Direction::Upstream, 1, 1);
        uncertain.arrival_latest_tick = 20;
        uncertain.arrival_evidence.interval_latest_tick = 20;
        let plan = schedule(&p, &[uncertain], SchedulingPolicy::FirstComeFirstServed)
            .expect("valid profile");
        assert_eq!(plan.schedule[0].start_tick, 20);
        assert_eq!(plan.schedule[0].waiting_from_earliest_arrival_ticks, 20);
    }

    #[test]
    fn a_rejected_budget_request_does_not_consume_the_remaining_budget() {
        let mut p = profile();
        p.unavailable_windows.clear();
        p.water_budget_ml = 5;
        let mut too_large = request("large", 0, Direction::Upstream, 10, 1);
        too_large.arrival_earliest_tick = 0;
        let mut fits = request("small", 1, Direction::Upstream, 5, 1);
        fits.arrival_earliest_tick = 1;
        let plan = schedule(
            &p,
            &[too_large, fits],
            SchedulingPolicy::FirstComeFirstServed,
        )
        .expect("valid profile");
        assert_eq!(plan.metrics.scheduled_requests, 1);
        assert_eq!(plan.schedule[0].request_id, "small");
        assert_eq!(
            plan.outcomes
                .iter()
                .find(|outcome| outcome.request_id == "large")
                .and_then(|outcome| outcome.reason),
            Some(NotScheduledReason::WaterBudgetExceeded)
        );
    }

    #[test]
    fn direction_change_delay_is_included_in_the_next_passage_time() {
        let mut p = profile();
        p.unavailable_windows.clear();
        p.direction_change_ticks = 7;
        let first = request("up", 0, Direction::Upstream, 1, 1);
        let second = request("down", 0, Direction::Downstream, 1, 1);
        let plan = schedule(&p, &[first, second], SchedulingPolicy::FirstComeFirstServed)
            .expect("valid profile");
        assert_eq!(plan.schedule[0].end_tick, 10);
        assert_eq!(plan.schedule[1].start_tick, 17);
        assert_eq!(plan.schedule[1].direction_change_before_ticks, 7);
        assert_eq!(plan.metrics.direction_changes, 1);
        assert_eq!(plan.metrics.total_service_ticks, 20);
        assert_eq!(plan.metrics.total_direction_change_ticks, 7);
        assert_eq!(plan.metrics.total_occupied_ticks, 27);
        assert_eq!(plan.metrics.comparable_arrival_pairs, 1);
    }

    #[test]
    fn stale_arrival_evidence_requires_review() {
        let mut p = profile();
        p.maximum_arrival_evidence_age_ticks = 5;
        let mut stale = request("stale", 0, Direction::Upstream, 1, 1);
        stale.arrival_evidence.observed_at_tick = 10;
        // Make the decision time later than the observation to exercise age.
        p.planning_window.start_tick = 20;
        stale.arrival_evidence.observed_at_tick = 10;
        stale.arrival_evidence.valid_through_tick = Some(100);
        let plan =
            schedule(&p, &[stale], SchedulingPolicy::FirstComeFirstServed).expect("valid profile");
        assert_eq!(plan.schedule.len(), 0);
        assert_eq!(plan.disposition, PlanDisposition::NeedsReview);
        assert_eq!(
            plan.outcomes[0].reason,
            Some(NotScheduledReason::ArrivalEvidenceNeedsReview)
        );
    }

    #[test]
    fn mixed_valid_and_stale_requests_preserve_plan_level_review_status() {
        let mut p = profile();
        p.maximum_arrival_evidence_age_ticks = 5;
        p.planning_window.start_tick = 20;
        let mut stale = request("stale", 20, Direction::Upstream, 1, 1);
        stale.arrival_evidence.observed_at_tick = 10;
        let valid = request("valid", 20, Direction::Upstream, 1, 1);
        let plan = schedule(&p, &[stale, valid], SchedulingPolicy::FirstComeFirstServed)
            .expect("valid profile");
        assert_eq!(plan.schedule.len(), 1);
        assert_eq!(plan.disposition, PlanDisposition::NeedsReview);
    }

    #[test]
    fn changed_arrival_interval_without_matching_evidence_is_rejected() {
        let mut changed = request("changed", 0, Direction::Upstream, 1, 1);
        changed.arrival_latest_tick = 5;
        let plan = schedule(
            &profile(),
            &[changed],
            SchedulingPolicy::FirstComeFirstServed,
        )
        .expect("valid profile");
        assert_eq!(
            plan.outcomes[0].reason,
            Some(NotScheduledReason::ArrivalEvidenceInvalid)
        );
    }

    #[test]
    fn future_dated_arrival_evidence_is_rejected_as_inconsistent() {
        let mut future = request("future", 0, Direction::Upstream, 1, 1);
        future.arrival_evidence.observed_at_tick = 1;
        let plan = schedule(
            &profile(),
            &[future],
            SchedulingPolicy::FirstComeFirstServed,
        )
        .expect("valid profile");
        assert_eq!(
            plan.outcomes[0].reason,
            Some(NotScheduledReason::ArrivalEvidenceInvalid)
        );
    }

    #[test]
    fn disputed_and_revoked_arrival_evidence_cannot_be_used() {
        let mut disputed = request("disputed", 0, Direction::Upstream, 1, 1);
        disputed.arrival_evidence.state = EvidenceState::Disputed;
        let mut revoked = request("revoked", 0, Direction::Upstream, 1, 1);
        revoked.arrival_evidence.state = EvidenceState::Revoked;
        let plan = schedule(
            &profile(),
            &[disputed, revoked],
            SchedulingPolicy::FirstComeFirstServed,
        )
        .expect("valid profile");
        assert!(plan.schedule.is_empty());
        assert_eq!(
            plan.outcomes
                .iter()
                .find(|outcome| outcome.request_id == "disputed")
                .and_then(|outcome| outcome.reason),
            Some(NotScheduledReason::ArrivalEvidenceNeedsReview)
        );
        assert_eq!(
            plan.outcomes
                .iter()
                .find(|outcome| outcome.request_id == "revoked")
                .and_then(|outcome| outcome.reason),
            Some(NotScheduledReason::ArrivalEvidenceRevoked)
        );
    }

    #[test]
    fn missing_arrival_calibration_receipt_is_not_accepted() {
        let mut missing = request("missing", 0, Direction::Upstream, 1, 1);
        missing.arrival_evidence.calibration_receipt_id.clear();
        let plan = schedule(
            &profile(),
            &[missing],
            SchedulingPolicy::FirstComeFirstServed,
        )
        .expect("valid profile");
        assert_eq!(
            plan.outcomes[0].reason,
            Some(NotScheduledReason::ArrivalEvidenceInvalid)
        );
    }

    #[test]
    fn unknown_and_unsupported_passages_never_become_schedulable() {
        let mut unknown = request("unknown", 0, Direction::Upstream, 1, 1);
        unknown.compatibility.disposition = PassageDisposition::EvidenceInconsistent;
        let mut unsupported = request("unsupported", 0, Direction::Upstream, 1, 1);
        unsupported.compatibility.disposition = PassageDisposition::UnsupportedOperatingProfile;
        let plan = schedule(
            &profile(),
            &[unknown, unsupported],
            SchedulingPolicy::FirstComeFirstServed,
        )
        .expect("valid profile");
        assert!(plan.schedule.is_empty());
        assert_eq!(
            plan.outcomes
                .iter()
                .find(|outcome| outcome.request_id == "unknown")
                .and_then(|outcome| outcome.reason),
            Some(NotScheduledReason::PassageNeedsReview)
        );
        assert_eq!(
            plan.outcomes
                .iter()
                .find(|outcome| outcome.request_id == "unsupported")
                .and_then(|outcome| outcome.reason),
            Some(NotScheduledReason::PassageUnsupported)
        );
    }

    #[test]
    fn every_request_has_exactly_one_final_outcome() {
        let mut invalid = request("invalid", 0, Direction::Upstream, 1, 1);
        invalid.service_duration_ticks = 0;
        let duplicate_a = request("duplicate", 0, Direction::Upstream, 1, 1);
        let duplicate_b = duplicate_a.clone();
        let blocked = {
            let mut request = request("blocked", 0, Direction::Upstream, 1, 1);
            request.compatibility.disposition = PassageDisposition::BlockedByHardConstraint;
            request
        };
        let valid = request("valid", 0, Direction::Upstream, 1, 1);
        let requests = vec![invalid, duplicate_a, duplicate_b, blocked, valid];
        let plan = schedule(
            &profile(),
            &requests,
            SchedulingPolicy::FirstComeFirstServed,
        )
        .expect("valid profile");
        assert_eq!(plan.outcomes.len(), requests.len());
        assert_eq!(
            plan.outcomes
                .iter()
                .map(|outcome| outcome.request_id.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            4
        );
        assert_eq!(
            plan.metrics.scheduled_requests + plan.metrics.unserved_requests,
            5
        );
    }

    #[test]
    fn inversion_metric_detects_a_policy_order_reversal() {
        let mut early = request("early", 0, Direction::Upstream, 1, 1);
        early.maximum_wait_ticks = 100;
        let mut urgent = request("urgent", 10, Direction::Upstream, 1, 1);
        urgent.maximum_wait_ticks = 5;
        let plan = schedule(
            &profile(),
            &[early, urgent],
            SchedulingPolicy::LeastWaitSlack,
        )
        .expect("valid profile");
        assert_eq!(plan.metrics.inversion_pairs, 1);
    }
}
