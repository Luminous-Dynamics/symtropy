//! Independent exhaustive oracle for tiny, single-lock scheduling instances.
//!
//! This intentionally reimplements the finite scheduling constraints rather than
//! calling the production scheduler or its feasibility/metric helpers. It is a
//! test oracle, not a production optimizer. Factorial enumeration is bounded to
//! tiny fixtures.

use inland_waterway_lock_operations::{
    EvidenceState, PassageDisposition, PassageEvaluation, TimeInterval,
};
use lock_traffic_scheduling::{
    ArrivalWindowEvidence, Direction, LockScheduleProfile, PassageRequest, SchedulingPolicy,
    schedule,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct OracleCandidate {
    accepted_ids: Vec<String>,
    metrics: OracleMetrics,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct OracleMetrics {
    accepted_count: u64,
    service_units: u128,
    total_wait_ticks: u128,
    maximum_wait_ticks: u64,
    water_ml: u128,
    pump_energy_wh: u128,
    direction_changes: u64,
    inversion_pairs: u64,
}

fn independently_find_start(
    profile: &LockScheduleProfile,
    earliest: u64,
    duration: u64,
) -> Option<u64> {
    let mut candidate = earliest.max(profile.planning_window.start_tick);
    loop {
        let end = candidate.checked_add(duration)?;
        if end > profile.planning_window.end_tick {
            return None;
        }

        let mut next_available = None;
        for unavailable in &profile.unavailable_windows {
            let intersects = candidate < unavailable.end_tick && unavailable.start_tick < end;
            if intersects {
                next_available = Some(unavailable.end_tick);
                break;
            }
        }

        match next_available {
            Some(next) => candidate = next,
            None => return Some(candidate),
        }
    }
}

fn arrival_evidence_is_eligible(profile: &LockScheduleProfile, request: &PassageRequest) -> bool {
    let evidence = &request.arrival_evidence;
    let decision_tick = profile.planning_window.start_tick;
    let has_identity = !evidence.evidence_id.trim().is_empty()
        && !evidence.source_id.trim().is_empty()
        && !evidence.source_revision.trim().is_empty()
        && !evidence.calibration_receipt_id.trim().is_empty()
        && !evidence.operating_regime_id.trim().is_empty();
    let current = evidence.state == EvidenceState::Current;
    let not_future = evidence.observed_at_tick <= decision_tick;
    let fresh = decision_tick.saturating_sub(evidence.observed_at_tick)
        <= profile.maximum_arrival_evidence_age_ticks;
    let valid = evidence
        .valid_through_tick
        .is_some_and(|expiry| expiry >= decision_tick);
    has_identity && current && not_future && fresh && valid
}

fn evaluate_order(
    profile: &LockScheduleProfile,
    requests: &[PassageRequest],
    order: &[usize],
) -> OracleCandidate {
    let mut cursor = profile.planning_window.start_tick;
    let mut last_direction = None;
    let mut accepted = Vec::new();
    let mut metrics = OracleMetrics::default();

    for index in order {
        let request = &requests[*index];

        // These eligibility and binding checks are repeated here on purpose.
        if request.compatibility.disposition != PassageDisposition::EligibleForSyntheticPassage
            || request.lock_id != profile.lock_id
            || request.lock_revision != profile.lock_revision
            || request.service_duration_ticks == 0
            || !arrival_evidence_is_eligible(profile, request)
            || request.arrival_earliest_tick > request.arrival_latest_tick
            || request.earliest_start_tick > request.latest_start_tick
        {
            continue;
        }

        let change_ticks = if last_direction.is_some_and(|direction| direction != request.direction)
        {
            profile.direction_change_ticks
        } else {
            0
        };
        let Some(after_change) = cursor.checked_add(change_ticks) else {
            continue;
        };

        let earliest = after_change
            .max(request.arrival_latest_tick)
            .max(request.earliest_start_tick)
            .max(profile.planning_window.start_tick);
        let Some(start) =
            independently_find_start(profile, earliest, request.service_duration_ticks)
        else {
            continue;
        };
        let Some(end) = start.checked_add(request.service_duration_ticks) else {
            continue;
        };

        let wait = start.saturating_sub(request.arrival_earliest_tick);
        let deadline_missed = request
            .completion_deadline_tick
            .is_some_and(|deadline| end > deadline);
        if start > request.latest_start_tick
            || wait > request.maximum_wait_ticks
            || deadline_missed
            || metrics.water_ml + u128::from(request.water_requirement_ml)
                > u128::from(profile.water_budget_ml)
            || metrics.pump_energy_wh + u128::from(request.pump_energy_requirement_wh)
                > u128::from(profile.pump_energy_budget_wh)
        {
            continue;
        }

        if last_direction.is_some_and(|direction| direction != request.direction) {
            metrics.direction_changes += 1;
        }
        last_direction = Some(request.direction);
        cursor = end;

        metrics.accepted_count += 1;
        metrics.service_units += u128::from(request.service_units);
        metrics.total_wait_ticks += u128::from(wait);
        metrics.maximum_wait_ticks = metrics.maximum_wait_ticks.max(wait);
        metrics.water_ml += u128::from(request.water_requirement_ml);
        metrics.pump_energy_wh += u128::from(request.pump_energy_requirement_wh);
        accepted.push(request.request_id.clone());
    }

    for (left, left_id) in accepted.iter().enumerate() {
        let left_request = requests
            .iter()
            .find(|request| request.request_id == *left_id)
            .expect("accepted request exists");
        for right_id in accepted.iter().skip(left + 1) {
            let right_request = requests
                .iter()
                .find(|request| request.request_id == *right_id)
                .expect("accepted request exists");
            let left_key = (left_request.arrival_earliest_tick, left_id.as_str());
            let right_key = (right_request.arrival_earliest_tick, right_id.as_str());
            if left_key > right_key {
                metrics.inversion_pairs += 1;
            }
        }
    }

    OracleCandidate {
        accepted_ids: accepted,
        metrics,
    }
}

fn enumerate_permutations(
    profile: &LockScheduleProfile,
    requests: &[PassageRequest],
    prefix: &mut Vec<usize>,
    remaining: &mut Vec<usize>,
    candidates: &mut Vec<OracleCandidate>,
) {
    if remaining.is_empty() {
        candidates.push(evaluate_order(profile, requests, prefix));
        return;
    }

    for position in 0..remaining.len() {
        let selected = remaining.remove(position);
        prefix.push(selected);
        enumerate_permutations(profile, requests, prefix, remaining, candidates);
        prefix.pop();
        remaining.insert(position, selected);
    }
}

fn exact_frontier(
    profile: &LockScheduleProfile,
    requests: &[PassageRequest],
) -> Vec<OracleCandidate> {
    assert!(
        requests.len() <= 8,
        "the exhaustive oracle is only for tiny cases"
    );
    let mut prefix = Vec::with_capacity(requests.len());
    let mut remaining: Vec<usize> = (0..requests.len()).collect();
    let mut candidates = Vec::new();
    enumerate_permutations(
        profile,
        requests,
        &mut prefix,
        &mut remaining,
        &mut candidates,
    );

    let mut frontier = Vec::new();
    for candidate in candidates {
        let dominated = frontier
            .iter()
            .any(|existing: &OracleCandidate| dominates(&existing.metrics, &candidate.metrics));
        if dominated {
            continue;
        }
        frontier.retain(|existing| !dominates(&candidate.metrics, &existing.metrics));
        frontier.push(candidate);
    }
    frontier
}

fn dominates(left: &OracleMetrics, right: &OracleMetrics) -> bool {
    let no_worse = left.accepted_count >= right.accepted_count
        && left.service_units >= right.service_units
        && left.total_wait_ticks <= right.total_wait_ticks
        && left.maximum_wait_ticks <= right.maximum_wait_ticks
        && left.water_ml <= right.water_ml
        && left.pump_energy_wh <= right.pump_energy_wh
        && left.direction_changes <= right.direction_changes
        && left.inversion_pairs <= right.inversion_pairs;
    let strictly_better = left.accepted_count > right.accepted_count
        || left.service_units > right.service_units
        || left.total_wait_ticks < right.total_wait_ticks
        || left.maximum_wait_ticks < right.maximum_wait_ticks
        || left.water_ml < right.water_ml
        || left.pump_energy_wh < right.pump_energy_wh
        || left.direction_changes < right.direction_changes
        || left.inversion_pairs < right.inversion_pairs;
    no_worse && strictly_better
}

fn passage(disposition: PassageDisposition) -> PassageEvaluation {
    PassageEvaluation {
        disposition,
        reasons: Vec::new(),
        required_length_mm: 1,
        required_beam_mm: 1,
        required_depth_mm: 1,
    }
}

fn request(
    id: &str,
    duration: u64,
    latest_start: u64,
    deadline: u64,
    service_units: u64,
) -> PassageRequest {
    PassageRequest {
        request_id: id.into(),
        vessel_id: format!("vessel-{id}"),
        vessel_revision: "v1".into(),
        compatibility_receipt_id: format!("compat-{id}"),
        compatibility: passage(PassageDisposition::EligibleForSyntheticPassage),
        lock_id: "lock-01".into(),
        lock_revision: "rev-01".into(),
        participant_id: format!("participant-{id}"),
        service_group: "freight".into(),
        direction: Direction::Upstream,
        arrival_earliest_tick: 0,
        arrival_latest_tick: 0,
        arrival_evidence: ArrivalWindowEvidence {
            evidence_id: format!("arrival-evidence-{id}"),
            source_id: "arrival-model".into(),
            source_revision: "model-v1".into(),
            calibration_receipt_id: "calibration-v1".into(),
            operating_regime_id: "regime-normal".into(),
            observed_at_tick: 0,
            valid_through_tick: Some(1_000),
            state: EvidenceState::Current,
        },
        earliest_start_tick: 0,
        latest_start_tick: latest_start,
        maximum_wait_ticks: 100,
        service_duration_ticks: duration,
        service_units,
        water_requirement_ml: 0,
        pump_energy_requirement_wh: 0,
        completion_deadline_tick: Some(deadline),
    }
}

fn profile() -> LockScheduleProfile {
    LockScheduleProfile {
        lock_id: "lock-01".into(),
        lock_revision: "rev-01".into(),
        planning_window: TimeInterval {
            start_tick: 0,
            end_tick: 100,
        },
        tick_seconds: 60,
        service_unit_label: "tonne".into(),
        direction_change_ticks: 0,
        maximum_arrival_evidence_age_ticks: 20,
        unavailable_windows: Vec::new(),
        water_budget_ml: 1_000,
        pump_energy_budget_wh: 1_000,
    }
}

#[test]
fn exact_oracle_finds_order_that_fcfs_misses() {
    // FCFS breaks the equal-arrival tie by request ID and selects the long job.
    // The short-deadline vessel must go first for both to be served.
    let long = request("a-long", 30, 30, 40, 100);
    let short = request("b-short", 10, 0, 10, 50);
    let requests = vec![long, short];
    let p = profile();

    let fcfs =
        schedule(&p, &requests, SchedulingPolicy::FirstComeFirstServed).expect("profile is valid");
    assert_eq!(fcfs.metrics.scheduled_requests, 1);

    let frontier = exact_frontier(&p, &requests);
    assert!(!frontier.is_empty());
    assert_eq!(
        frontier
            .iter()
            .map(|candidate| candidate.metrics.accepted_count)
            .max(),
        Some(2)
    );
    assert!(frontier.iter().any(|candidate| {
        candidate.accepted_ids == vec!["b-short".to_owned(), "a-long".to_owned()]
    }));
}

#[test]
fn oracle_does_not_admit_unknown_or_blocked_passage_evidence() {
    let p = profile();
    let mut unknown = request("unknown", 10, 100, 100, 10);
    unknown.compatibility = passage(PassageDisposition::UnknownReviewRequired);
    let mut blocked = request("blocked", 10, 100, 100, 10);
    blocked.compatibility = passage(PassageDisposition::BlockedByHardConstraint);
    let requests = vec![unknown, blocked];

    let frontier = exact_frontier(&p, &requests);
    assert!(
        frontier
            .iter()
            .all(|candidate| candidate.accepted_ids.is_empty())
    );
}

#[test]
fn oracle_excludes_stale_or_unvalidated_arrival_intervals() {
    let mut p = profile();
    p.planning_window.start_tick = 50;
    p.maximum_arrival_evidence_age_ticks = 5;
    let mut stale = request("stale", 10, 100, 100, 10);
    stale.arrival_evidence.observed_at_tick = 10;
    stale.arrival_evidence.valid_through_tick = Some(100);
    let mut unvalidated = request("unvalidated", 10, 100, 100, 10);
    unvalidated.arrival_evidence.calibration_receipt_id.clear();
    let frontier = exact_frontier(&p, &[stale, unvalidated]);
    assert!(
        frontier
            .iter()
            .all(|candidate| candidate.accepted_ids.is_empty())
    );
}

#[test]
fn oracle_preserves_service_vs_water_tradeoffs_on_the_pareto_front() {
    let mut p = profile();
    p.water_budget_ml = 10;
    let mut high_service = request("high-service", 10, 100, 100, 100);
    high_service.water_requirement_ml = 10;
    let mut low_water = request("low-water", 10, 100, 100, 50);
    low_water.water_requirement_ml = 1;
    let requests = vec![high_service, low_water];

    let frontier = exact_frontier(&p, &requests);
    assert!(frontier.iter().any(|candidate| {
        candidate.accepted_ids == vec!["high-service".to_owned()]
            && candidate.metrics.service_units == 100
            && candidate.metrics.water_ml == 10
    }));
    assert!(frontier.iter().any(|candidate| {
        candidate.accepted_ids == vec!["low-water".to_owned()]
            && candidate.metrics.service_units == 50
            && candidate.metrics.water_ml == 1
    }));
}

#[test]
fn oracle_best_frontier_is_not_assumed_to_equal_a_greedy_policy() {
    let p = profile();
    let requests = vec![request("a", 30, 30, 40, 100), request("b", 10, 0, 10, 50)];
    let frontier = exact_frontier(&p, &requests);
    let fcfs =
        schedule(&p, &requests, SchedulingPolicy::FirstComeFirstServed).expect("profile is valid");

    assert!(
        frontier
            .iter()
            .any(|candidate| candidate.metrics.accepted_count > fcfs.metrics.scheduled_requests)
    );
}

