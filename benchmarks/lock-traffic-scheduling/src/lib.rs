//! Deterministic, evidence-gated lock traffic scheduling reference evaluator.
//!
//! This crate produces recommendations only. It never controls lock hardware,
//! dispatches a vessel, or grants navigation authority.

use std::collections::BTreeMap;

use inland_waterway_lock_operations::{PassageDisposition, PassageEvaluation, TimeInterval};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    Upstream,
    Downstream,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingPolicy {
    FirstComeFirstServed,
    LeastWaitSlack,
    DirectionalBatch { max_consecutive: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockScheduleProfile {
    pub lock_id: String,
    pub lock_revision: String,
    pub planning_window: TimeInterval,
    pub tick_seconds: u64,
    pub service_unit_label: String,
    pub direction_change_ticks: u64,
    pub unavailable_windows: Vec<TimeInterval>,
    pub water_budget_ml: u64,
    pub pump_energy_budget_wh: u64,
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GroupMetrics {
    pub requests: u64,
    pub scheduled: u64,
    pub unserved: u64,
    pub service_units: u128,
    pub total_wait_ticks: u128,
    pub maximum_wait_ticks: u64,
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
    pub total_service_ticks: u128,
    pub available_service_ticks: u128,
    pub total_water_ml: u128,
    pub total_pump_energy_wh: u128,
    pub direction_changes: u64,
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
        SchedulingPolicy::DirectionalBatch {
            max_consecutive: 0
        }
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

    match request.compatibility.disposition {
        PassageDisposition::EligibleForSyntheticPassage => None,
        PassageDisposition::BlockedByHardConstraint => {
            Some(NotScheduledReason::PassageBlocked)
        }
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

fn least_slack_first(requests: &[PassageRequest], pending: &[usize]) -> Option<usize> {
    pending.iter().copied().min_by_key(|index| {
        let request = &requests[*index];
        (
            u128::from(request.arrival_earliest_tick)
                + u128::from(request.maximum_wait_ticks),
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
    find_start(profile, unavailable, earliest, request.service_duration_ticks)
}

fn select_next(
    profile: &LockScheduleProfile,
    unavailable: &[TimeInterval],
    requests: &[PassageRequest],
    pending: &[usize],
    policy: SchedulingPolicy,
    current_direction: Option<Direction>,
    current_batch_count: usize,
    cursor_tick: u64,
) -> Option<usize> {
    match policy {
        SchedulingPolicy::FirstComeFirstServed => oldest_first(requests, pending),
        SchedulingPolicy::LeastWaitSlack => least_slack_first(requests, pending),
        SchedulingPolicy::DirectionalBatch { max_consecutive } => {
            let Some(direction) = current_direction else {
                return oldest_first(requests, pending);
            };
            if current_batch_count >= max_consecutive {
                return oldest_first(requests, pending);
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
            let Some(same_start) =
                projected_start(profile, unavailable, cursor_tick, same, 0)
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

fn available_service_ticks(
    profile: &LockScheduleProfile,
    unavailable: &[TimeInterval],
) -> u128 {
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
    key: &str,
    outcome: &RequestOutcome,
) {
    let group = groups.entry(key.to_owned()).or_default();
    group.requests += 1;
    match &outcome.scheduled {
        Some(passage) => {
            group.scheduled += 1;
            group.service_units += u128::from(passage.service_units);
            group.total_wait_ticks +=
                u128::from(passage.waiting_from_earliest_arrival_ticks);
            group.maximum_wait_ticks = group
                .maximum_wait_ticks
                .max(passage.waiting_from_earliest_arrival_ticks);
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

    let mut previous_direction = None;
    for passage in schedule {
        metrics.scheduled_requests += 1;
        metrics.service_units += u128::from(passage.service_units);
        metrics.total_wait_ticks +=
            u128::from(passage.waiting_from_earliest_arrival_ticks);
        metrics.maximum_wait_ticks = metrics
            .maximum_wait_ticks
            .max(passage.waiting_from_earliest_arrival_ticks);
        metrics.total_service_ticks += u128::from(passage.end_tick - passage.start_tick);
        metrics.total_water_ml += u128::from(passage.water_ml);
        metrics.total_pump_energy_wh += u128::from(passage.pump_energy_wh);
        if previous_direction.is_some_and(|direction| direction != passage.direction) {
            metrics.direction_changes += 1;
        }
        previous_direction = Some(passage.direction);
    }

    for outcome in outcomes {
        if outcome.scheduled.is_none() {
            metrics.unserved_requests += 1;
        }
        add_group_metrics(
            &mut metrics.by_participant,
            &outcome.participant_id,
            outcome,
        );
        add_group_metrics(
            &mut metrics.by_service_group,
            &outcome.service_group,
            outcome,
        );
    }

    for left in 0..schedule.len() {
        for right in (left + 1)..schedule.len() {
            let first = &schedule[left];
            let second = &schedule[right];
            let first_arrival = (first.arrival_earliest_tick, first.request_id.as_str());
            let second_arrival = (second.arrival_earliest_tick, second.request_id.as_str());
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

    while !pending.is_empty() {
        let Some(selected_index) = select_next(
            profile,
            &unavailable,
            requests,
            &pending,
            policy,
            current_direction,
            current_batch_count,
            cursor_tick,
        ) else {
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

        let direction_change = if current_direction.is_some_and(|direction| direction != request.direction) {
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
        schedule.push(ScheduledPassage {
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
    let disposition = if outcomes
        .iter()
        .any(|outcome| outcome.reason == Some(NotScheduledReason::PassageNeedsReview))
    {
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
        let plan = schedule(
            &profile(),
            &[old, urgent],
            SchedulingPolicy::LeastWaitSlack,
        )
        .expect("valid profile");
        assert_eq!(plan.schedule[0].request_id, "urgent");
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
    fn duplicate_request_ids_are_all_rejected() {
        let duplicate = request("same", 0, Direction::Upstream, 1, 1);
        let plan = schedule(
            &profile(),
            &[duplicate.clone(), duplicate],
            SchedulingPolicy::FirstComeFirstServed,
        )
        .expect("valid profile");
        assert!(plan.schedule.is_empty());
        assert!(plan.outcomes.iter().all(|outcome| {
            outcome.reason == Some(NotScheduledReason::DuplicateRequestId)
        }));
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
