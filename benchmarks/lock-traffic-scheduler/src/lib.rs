//! Deterministic single-lock traffic scheduling baselines.
//!
//! Output is a recommendation only; this crate cannot authorize passage,
//! dispatch vessels or control gate/pump equipment.

use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeInterval {
    pub start_tick: u64,
    pub end_tick: u64,
}

impl TimeInterval {
    fn is_valid(self) -> bool {
        self.start_tick < self.end_tick
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    Upstream,
    Downstream,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServiceClass {
    Emergency,
    EssentialPassenger,
    TimeSensitiveFreight,
    OrdinaryFreight,
    Recreational,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassageEligibility {
    Qualified,
    Unknown,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockScheduleProfile {
    pub lock_id: String,
    pub revision: String,
    pub valid_through_tick: Option<u64>,
    /// Disjoint half-open lock availability windows, sorted by start tick.
    pub availability: Vec<TimeInterval>,
    pub direction_switch_ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassageRequest {
    pub request_id: String,
    pub participant_id: String,
    pub vessel_id: String,
    pub qualification_reference: String,
    pub authorization_reference: String,
    pub eligibility: PassageEligibility,
    pub arrival_tick: u64,
    pub earliest_start_tick: u64,
    /// Latest allowed service start, inclusive.
    pub latest_start_tick: u64,
    pub maximum_wait_ticks: u64,
    pub service_duration_ticks: u64,
    pub direction: Direction,
    pub service_class: ServiceClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingPolicy {
    FirstComeFirstServed,
    PriorityWithAging { aging_threshold_ticks: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerError {
    InvalidLockIdentity,
    LockProfileExpired,
    UnknownLockProfileCurrentness,
    InvalidAvailabilityWindow,
    OverlappingAvailabilityWindows,
    EmptySchedule,
    InvalidRequestIdentity,
    DuplicateRequestId,
    InvalidRequestWindow,
    ZeroServiceDuration,
    ArithmeticOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnscheduledReason {
    PassageBlockedByQualification,
    PassageQualificationUnknown,
    AuthorizationReferenceMissing,
    NoAvailableLockSlot,
    LatestStartExceeded,
    MaximumWaitExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledPassage {
    pub request_id: String,
    pub participant_id: String,
    pub vessel_id: String,
    pub direction: Direction,
    pub service_class: ServiceClass,
    pub reserved_start_tick: u64,
    pub service_start_tick: u64,
    pub finish_tick: u64,
    pub waiting_ticks: u64,
    pub direction_switch_ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnscheduledPassage {
    pub request_id: String,
    pub reason: UnscheduledReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SchedulingMetrics {
    pub served_count: u64,
    pub unscheduled_count: u64,
    pub total_wait_ticks: u128,
    pub maximum_wait_ticks: u64,
    pub total_lock_service_ticks: u128,
    pub total_direction_switch_ticks: u128,
    pub direction_switch_count: u64,
    pub participant_wait_totals: Vec<(String, u128)>,
    pub service_class_wait_totals: Vec<(ServiceClass, u128)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleResult {
    pub policy: SchedulingPolicy,
    pub scheduled: Vec<ScheduledPassage>,
    pub unscheduled: Vec<UnscheduledPassage>,
    pub metrics: SchedulingMetrics,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_profile(
    profile: &LockScheduleProfile,
    planning_tick: u64,
) -> Result<(), SchedulerError> {
    if !nonempty(&profile.lock_id) || !nonempty(&profile.revision) {
        return Err(SchedulerError::InvalidLockIdentity);
    }
    match profile.valid_through_tick {
        None => return Err(SchedulerError::UnknownLockProfileCurrentness),
        Some(expiry) if expiry < planning_tick => {
            return Err(SchedulerError::LockProfileExpired);
        }
        Some(_) => {}
    }

    let mut previous_end = None;
    for window in &profile.availability {
        if !window.is_valid() {
            return Err(SchedulerError::InvalidAvailabilityWindow);
        }
        if previous_end.is_some_and(|end| window.start_tick < end) {
            return Err(SchedulerError::OverlappingAvailabilityWindows);
        }
        previous_end = Some(window.end_tick);
    }
    Ok(())
}

fn validate_requests(requests: &[PassageRequest]) -> Result<(), SchedulerError> {
    if requests.is_empty() {
        return Err(SchedulerError::EmptySchedule);
    }
    let mut ids = BTreeSet::new();
    for request in requests {
        if !nonempty(&request.request_id)
            || !nonempty(&request.participant_id)
            || !nonempty(&request.vessel_id)
        {
            return Err(SchedulerError::InvalidRequestIdentity);
        }
        if !ids.insert(request.request_id.as_str()) {
            return Err(SchedulerError::DuplicateRequestId);
        }
        if request.earliest_start_tick > request.latest_start_tick
            || request.latest_start_tick < request.arrival_tick
        {
            return Err(SchedulerError::InvalidRequestWindow);
        }
        if request.service_duration_ticks == 0 {
            return Err(SchedulerError::ZeroServiceDuration);
        }
    }
    Ok(())
}

fn ready_tick(request: &PassageRequest) -> u64 {
    request.arrival_tick.max(request.earliest_start_tick)
}

fn find_slot(
    profile: &LockScheduleProfile,
    earliest_reserved_start: u64,
    occupied_duration: u64,
) -> Result<Option<u64>, SchedulerError> {
    for window in &profile.availability {
        let start = earliest_reserved_start.max(window.start_tick);
        let finish = start
            .checked_add(occupied_duration)
            .ok_or(SchedulerError::ArithmeticOverflow)?;
        if finish <= window.end_tick {
            return Ok(Some(start));
        }
    }
    Ok(None)
}

fn choose_candidate(
    pending: &[usize],
    requests: &[PassageRequest],
    current_tick: u64,
    policy: SchedulingPolicy,
) -> Option<usize> {
    let ready = pending
        .iter()
        .copied()
        .filter(|index| ready_tick(&requests[*index]) <= current_tick)
        .collect::<Vec<_>>();

    match policy {
        SchedulingPolicy::FirstComeFirstServed => ready.into_iter().min_by_key(|index| {
            let request = &requests[*index];
            (
                request.arrival_tick,
                ready_tick(request),
                request.request_id.as_str(),
            )
        }),
        SchedulingPolicy::PriorityWithAging { aging_threshold_ticks } => {
            let aged = ready
                .iter()
                .copied()
                .filter(|index| {
                    current_tick.saturating_sub(requests[*index].arrival_tick)
                        >= aging_threshold_ticks
                })
                .collect::<Vec<_>>();
            if !aged.is_empty() {
                aged.into_iter().min_by_key(|index| {
                    (
                        requests[*index].arrival_tick,
                        requests[*index].request_id.as_str(),
                    )
                })
            } else {
                ready.into_iter().min_by_key(|index| {
                    let request = &requests[*index];
                    (
                        request.service_class,
                        request.arrival_tick,
                        request.request_id.as_str(),
                    )
                })
            }
        }
    }
}

fn add_participant_wait(metrics: &mut SchedulingMetrics, participant: &str, wait: u64) {
    if let Some((_, total)) = metrics
        .participant_wait_totals
        .iter_mut()
        .find(|(id, _)| id == participant)
    {
        *total += u128::from(wait);
    } else {
        metrics
            .participant_wait_totals
            .push((participant.to_owned(), u128::from(wait)));
        metrics
            .participant_wait_totals
            .sort_by(|left, right| left.0.cmp(&right.0));
    }
}

fn add_service_wait(
    metrics: &mut SchedulingMetrics,
    service_class: ServiceClass,
    wait: u64,
) {
    if let Some((_, total)) = metrics
        .service_class_wait_totals
        .iter_mut()
        .find(|(class, _)| *class == service_class)
    {
        *total += u128::from(wait);
    } else {
        metrics
            .service_class_wait_totals
            .push((service_class, u128::from(wait)));
        metrics
            .service_class_wait_totals
            .sort_by_key(|(class, _)| *class);
    }
}

/// Build a deterministic single-lock schedule using FCFS or priority with aging.
///
/// All passage eligibility and authorized-window evidence is supplied by upstream
/// owners. This function does not extend windows or create authorization.
pub fn schedule_lock(
    profile: &LockScheduleProfile,
    requests: &[PassageRequest],
    planning_tick: u64,
    policy: SchedulingPolicy,
) -> Result<ScheduleResult, SchedulerError> {
    validate_profile(profile, planning_tick)?;
    validate_requests(requests)?;

    let mut pending = Vec::new();
    let mut unscheduled = Vec::new();

    for (index, request) in requests.iter().enumerate() {
        match request.eligibility {
            PassageEligibility::Blocked => unscheduled.push(UnscheduledPassage {
                request_id: request.request_id.clone(),
                reason: UnscheduledReason::PassageBlockedByQualification,
            }),
            PassageEligibility::Unknown => unscheduled.push(UnscheduledPassage {
                request_id: request.request_id.clone(),
                reason: UnscheduledReason::PassageQualificationUnknown,
            }),
            PassageEligibility::Qualified
                if request.qualification_reference.trim().is_empty() =>
            {
                unscheduled.push(UnscheduledPassage {
                    request_id: request.request_id.clone(),
                    reason: UnscheduledReason::PassageQualificationUnknown,
                });
            }
            PassageEligibility::Qualified if request.authorization_reference.trim().is_empty() => {
                unscheduled.push(UnscheduledPassage {
                    request_id: request.request_id.clone(),
                    reason: UnscheduledReason::AuthorizationReferenceMissing,
                });
            }
            PassageEligibility::Qualified => pending.push(index),
        }
    }

    let mut scheduled = Vec::new();
    let mut metrics = SchedulingMetrics::default();
    let mut current_tick = planning_tick;
    let mut last_direction = None;

    while !pending.is_empty() {
        let Some(index) = choose_candidate(&pending, requests, current_tick, policy) else {
            if let Some(next_ready) = pending
                .iter()
                .map(|index| ready_tick(&requests[*index]))
                .filter(|tick| *tick > current_tick)
                .min()
            {
                current_tick = next_ready;
                continue;
            }
            for index in pending.drain(..) {
                unscheduled.push(UnscheduledPassage {
                    request_id: requests[index].request_id.clone(),
                    reason: UnscheduledReason::NoAvailableLockSlot,
                });
            }
            break;
        };

        pending.retain(|value| *value != index);
        let request = &requests[index];
        let switch_ticks = if last_direction.is_some_and(|direction| direction != request.direction) {
            profile.direction_switch_ticks
        } else {
            0
        };
        let earliest = current_tick.max(ready_tick(request));
        let occupied_duration = switch_ticks
            .checked_add(request.service_duration_ticks)
            .ok_or(SchedulerError::ArithmeticOverflow)?;
        let Some(reserved_start) = find_slot(profile, earliest, occupied_duration)? else {
            unscheduled.push(UnscheduledPassage {
                request_id: request.request_id.clone(),
                reason: UnscheduledReason::NoAvailableLockSlot,
            });
            continue;
        };
        let service_start = reserved_start
            .checked_add(switch_ticks)
            .ok_or(SchedulerError::ArithmeticOverflow)?;
        let finish = service_start
            .checked_add(request.service_duration_ticks)
            .ok_or(SchedulerError::ArithmeticOverflow)?;
        let waiting = service_start.saturating_sub(request.arrival_tick);

        if service_start > request.latest_start_tick {
            unscheduled.push(UnscheduledPassage {
                request_id: request.request_id.clone(),
                reason: UnscheduledReason::LatestStartExceeded,
            });
            continue;
        }
        if waiting > request.maximum_wait_ticks {
            unscheduled.push(UnscheduledPassage {
                request_id: request.request_id.clone(),
                reason: UnscheduledReason::MaximumWaitExceeded,
            });
            continue;
        }

        metrics.served_count = metrics.served_count
            .checked_add(1)
            .ok_or(SchedulerError::ArithmeticOverflow)?;
        metrics.total_wait_ticks = metrics.total_wait_ticks
            .checked_add(u128::from(waiting))
            .ok_or(SchedulerError::ArithmeticOverflow)?;
        metrics.maximum_wait_ticks = metrics.maximum_wait_ticks.max(waiting);
        metrics.total_lock_service_ticks = metrics.total_lock_service_ticks
            .checked_add(u128::from(request.service_duration_ticks))
            .ok_or(SchedulerError::ArithmeticOverflow)?;
        metrics.total_direction_switch_ticks = metrics.total_direction_switch_ticks
            .checked_add(u128::from(switch_ticks))
            .ok_or(SchedulerError::ArithmeticOverflow)?;
        if switch_ticks > 0 {
            metrics.direction_switch_count = metrics.direction_switch_count
                .checked_add(1)
                .ok_or(SchedulerError::ArithmeticOverflow)?;
        }
        add_participant_wait(&mut metrics, &request.participant_id, waiting);
        add_service_wait(&mut metrics, request.service_class, waiting);

        scheduled.push(ScheduledPassage {
            request_id: request.request_id.clone(),
            participant_id: request.participant_id.clone(),
            vessel_id: request.vessel_id.clone(),
            direction: request.direction,
            service_class: request.service_class,
            reserved_start_tick: reserved_start,
            service_start_tick: service_start,
            finish_tick: finish,
            waiting_ticks: waiting,
            direction_switch_ticks: switch_ticks,
        });
        last_direction = Some(request.direction);
        current_tick = finish;
    }

    metrics.unscheduled_count = u64::try_from(unscheduled.len())
        .map_err(|_| SchedulerError::ArithmeticOverflow)?;
    scheduled.sort_by_key(|slot| (slot.service_start_tick, slot.request_id.as_str()));
    unscheduled.sort_by(|left, right| left.request_id.cmp(&right.request_id));

    Ok(ScheduleResult { policy, scheduled, unscheduled, metrics })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> LockScheduleProfile {
        LockScheduleProfile {
            lock_id: "lock-01".into(),
            revision: "lock-r4".into(),
            valid_through_tick: Some(1_000),
            availability: vec![TimeInterval { start_tick: 0, end_tick: 1_000 }],
            direction_switch_ticks: 5,
        }
    }

    fn request(
        id: &str,
        participant: &str,
        arrival: u64,
        direction: Direction,
        class: ServiceClass,
    ) -> PassageRequest {
        PassageRequest {
            request_id: id.into(),
            participant_id: participant.into(),
            vessel_id: format!("vessel-{id}"),
            qualification_reference: format!("passage-receipt-{id}"),
            authorization_reference: format!("authorization-{id}"),
            eligibility: PassageEligibility::Qualified,
            arrival_tick: arrival,
            earliest_start_tick: arrival,
            latest_start_tick: arrival + 500,
            maximum_wait_ticks: 500,
            service_duration_ticks: 10,
            direction,
            service_class: class,
        }
    }

    fn run(requests: &[PassageRequest], policy: SchedulingPolicy) -> ScheduleResult {
        schedule_lock(&profile(), requests, 0, policy).unwrap()
    }

    #[test]
    fn fcfs_orders_by_arrival_then_id() {
        let requests = vec![
            request("b", "p2", 5, Direction::Upstream, ServiceClass::OrdinaryFreight),
            request("a", "p1", 5, Direction::Downstream, ServiceClass::OrdinaryFreight),
            request("c", "p3", 0, Direction::Upstream, ServiceClass::OrdinaryFreight),
        ];
        let result = run(&requests, SchedulingPolicy::FirstComeFirstServed);
        let ids = result.scheduled.iter().map(|slot| slot.request_id.as_str()).collect::<Vec<_>>();
        assert_eq!(ids, vec!["c", "a", "b"]);
    }

    #[test]
    fn metrics_separate_service_wait_and_direction_switches() {
        let requests = vec![
            request("a", "p1", 0, Direction::Upstream, ServiceClass::OrdinaryFreight),
            request("b", "p2", 0, Direction::Downstream, ServiceClass::OrdinaryFreight),
        ];
        let result = run(&requests, SchedulingPolicy::FirstComeFirstServed);
        assert_eq!(result.metrics.served_count, 2);
        assert_eq!(result.metrics.total_wait_ticks, 15);
        assert_eq!(result.metrics.maximum_wait_ticks, 15);
        assert_eq!(result.metrics.total_lock_service_ticks, 20);
        assert_eq!(result.metrics.total_direction_switch_ticks, 5);
        assert_eq!(result.metrics.direction_switch_count, 1);
    }

    #[test]
    fn priority_policy_prefers_essential_passenger_when_ready() {
        let requests = vec![
            request("freight", "p1", 0, Direction::Upstream, ServiceClass::OrdinaryFreight),
            request("ferry", "p2", 0, Direction::Upstream, ServiceClass::EssentialPassenger),
        ];
        let result = run(&requests, SchedulingPolicy::PriorityWithAging { aging_threshold_ticks: 100 });
        assert_eq!(result.scheduled[0].request_id, "ferry");
    }

    #[test]
    fn aging_policy_protects_oldest_request_from_priority_starvation() {
        let requests = vec![
            request("old-freight", "p1", 0, Direction::Upstream, ServiceClass::OrdinaryFreight),
            request("new-ferry", "p2", 50, Direction::Downstream, ServiceClass::EssentialPassenger),
        ];
        let result = schedule_lock(
            &profile(), &requests, 60,
            SchedulingPolicy::PriorityWithAging { aging_threshold_ticks: 40 },
        ).unwrap();
        assert_eq!(result.scheduled[0].request_id, "old-freight");
    }

    #[test]
    fn blocked_passage_never_scheduled() {
        let mut req = request("blocked", "p1", 0, Direction::Upstream, ServiceClass::OrdinaryFreight);
        req.eligibility = PassageEligibility::Blocked;
        let result = run(&[req], SchedulingPolicy::FirstComeFirstServed);
        assert!(result.scheduled.is_empty());
        assert_eq!(result.unscheduled[0].reason, UnscheduledReason::PassageBlockedByQualification);
    }

    #[test]
    fn unknown_qualification_never_scheduled() {
        let mut req = request("unknown", "p1", 0, Direction::Upstream, ServiceClass::Emergency);
        req.eligibility = PassageEligibility::Unknown;
        let result = run(&[req], SchedulingPolicy::FirstComeFirstServed);
        assert_eq!(result.unscheduled[0].reason, UnscheduledReason::PassageQualificationUnknown);
    }

    #[test]
    fn qualification_receipt_is_required() {
        let mut req = request("missing", "p1", 0, Direction::Upstream, ServiceClass::OrdinaryFreight);
        req.qualification_reference.clear();
        assert_eq!(
            run(&[req], SchedulingPolicy::FirstComeFirstServed).unscheduled[0].reason,
            UnscheduledReason::PassageQualificationUnknown
        );
    }

    #[test]
    fn participant_authorization_is_required() {
        let mut req = request("unauthorized", "p1", 0, Direction::Upstream, ServiceClass::OrdinaryFreight);
        req.authorization_reference.clear();
        assert_eq!(
            run(&[req], SchedulingPolicy::FirstComeFirstServed).unscheduled[0].reason,
            UnscheduledReason::AuthorizationReferenceMissing
        );
    }

    #[test]
    fn explicit_earliest_start_is_respected() {
        let mut req = request("delayed", "p1", 0, Direction::Upstream, ServiceClass::OrdinaryFreight);
        req.earliest_start_tick = 30;
        assert_eq!(run(&[req], SchedulingPolicy::FirstComeFirstServed).scheduled[0].service_start_tick, 30);
    }

    #[test]
    fn latest_start_is_a_hard_bound() {
        let mut req = request("late", "p1", 0, Direction::Upstream, ServiceClass::OrdinaryFreight);
        req.latest_start_tick = 5;
        assert_eq!(
            run(&[req], SchedulingPolicy::FirstComeFirstServed).unscheduled[0].reason,
            UnscheduledReason::LatestStartExceeded
        );
    }

    #[test]
    fn maximum_wait_is_a_hard_bound() {
        let mut first = request("first", "p1", 0, Direction::Upstream, ServiceClass::OrdinaryFreight);
        first.service_duration_ticks = 20;
        let mut second = request("second", "p2", 0, Direction::Upstream, ServiceClass::OrdinaryFreight);
        second.maximum_wait_ticks = 5;
        assert_eq!(
            run(&[first, second], SchedulingPolicy::FirstComeFirstServed).unscheduled[0].reason,
            UnscheduledReason::MaximumWaitExceeded
        );
    }

    #[test]
    fn direction_switch_consumes_declared_setup_time() {
        let requests = vec![
            request("a", "p1", 0, Direction::Upstream, ServiceClass::OrdinaryFreight),
            request("b", "p2", 0, Direction::Downstream, ServiceClass::OrdinaryFreight),
        ];
        let result = run(&requests, SchedulingPolicy::FirstComeFirstServed);
        assert_eq!(result.scheduled[1].reserved_start_tick, 10);
        assert_eq!(result.scheduled[1].service_start_tick, 15);
        assert_eq!(result.scheduled[1].finish_tick, 25);
    }

    #[test]
    fn outage_windows_are_respected() {
        let mut p = profile();
        p.availability = vec![
            TimeInterval { start_tick: 0, end_tick: 15 },
            TimeInterval { start_tick: 30, end_tick: 100 },
        ];
        let requests = vec![
            request("a", "p1", 0, Direction::Upstream, ServiceClass::OrdinaryFreight),
            request("b", "p2", 0, Direction::Upstream, ServiceClass::OrdinaryFreight),
        ];
        let result = schedule_lock(&p, &requests, 0, SchedulingPolicy::FirstComeFirstServed).unwrap();
        assert_eq!(result.scheduled[0].service_start_tick, 0);
        assert_eq!(result.scheduled[1].service_start_tick, 30);
    }

    #[test]
    fn too_short_availability_leaves_request_unscheduled() {
        let mut p = profile();
        p.availability = vec![TimeInterval { start_tick: 0, end_tick: 5 }];
        let result = schedule_lock(
            &p,
            &[request("a", "p1", 0, Direction::Upstream, ServiceClass::OrdinaryFreight)],
            0,
            SchedulingPolicy::FirstComeFirstServed,
        ).unwrap();
        assert_eq!(result.unscheduled[0].reason, UnscheduledReason::NoAvailableLockSlot);
    }

    #[test]
    fn direction_setup_must_fit_inside_availability() {
        let mut p = profile();
        p.availability = vec![TimeInterval { start_tick: 0, end_tick: 20 }];
        let requests = vec![
            request("a", "p1", 0, Direction::Upstream, ServiceClass::OrdinaryFreight),
            request("b", "p2", 0, Direction::Downstream, ServiceClass::OrdinaryFreight),
        ];
        let result = schedule_lock(&p, &requests, 0, SchedulingPolicy::FirstComeFirstServed).unwrap();
        assert_eq!(result.scheduled.len(), 1);
        assert_eq!(result.unscheduled[0].request_id, "b");
    }

    #[test]
    fn input_order_does_not_change_priority_schedule() {
        let a = request("a", "p1", 0, Direction::Upstream, ServiceClass::OrdinaryFreight);
        let b = request("b", "p2", 0, Direction::Downstream, ServiceClass::Emergency);
        let c = request("c", "p3", 2, Direction::Upstream, ServiceClass::EssentialPassenger);
        let one = run(&[a.clone(), b.clone(), c.clone()], SchedulingPolicy::PriorityWithAging { aging_threshold_ticks: 100 });
        let two = run(&[c, a, b], SchedulingPolicy::PriorityWithAging { aging_threshold_ticks: 100 });
        assert_eq!(one, two);
    }

    #[test]
    fn participant_and_service_waits_are_separately_reported() {
        let requests = vec![
            request("a", "p1", 0, Direction::Upstream, ServiceClass::OrdinaryFreight),
            request("b", "p2", 0, Direction::Downstream, ServiceClass::EssentialPassenger),
        ];
        let result = run(&requests, SchedulingPolicy::FirstComeFirstServed);
        assert!(result.metrics.participant_wait_totals.iter().any(|(id, wait)| id == "p2" && *wait == 15));
        assert!(result.metrics.service_class_wait_totals.iter().any(|(class, wait)| *class == ServiceClass::EssentialPassenger && *wait == 15));
    }

    #[test]
    fn overlapping_availability_windows_are_rejected() {
        let mut p = profile();
        p.availability = vec![
            TimeInterval { start_tick: 0, end_tick: 20 },
            TimeInterval { start_tick: 15, end_tick: 30 },
        ];
        assert_eq!(
            schedule_lock(&p, &[request("a", "p1", 0, Direction::Upstream, ServiceClass::OrdinaryFreight)], 0, SchedulingPolicy::FirstComeFirstServed),
            Err(SchedulerError::OverlappingAvailabilityWindows)
        );
    }

    #[test]
    fn zero_length_availability_window_is_rejected() {
        let mut p = profile();
        p.availability = vec![TimeInterval { start_tick: 5, end_tick: 5 }];
        assert_eq!(
            schedule_lock(&p, &[request("a", "p1", 0, Direction::Upstream, ServiceClass::OrdinaryFreight)], 0, SchedulingPolicy::FirstComeFirstServed),
            Err(SchedulerError::InvalidAvailabilityWindow)
        );
    }

    #[test]
    fn duplicate_request_id_is_rejected() {
        let a = request("same", "p1", 0, Direction::Upstream, ServiceClass::OrdinaryFreight);
        let b = request("same", "p2", 1, Direction::Downstream, ServiceClass::OrdinaryFreight);
        assert_eq!(
            schedule_lock(&profile(), &[a, b], 0, SchedulingPolicy::FirstComeFirstServed),
            Err(SchedulerError::DuplicateRequestId)
        );
    }

    #[test]
    fn zero_service_duration_is_rejected() {
        let mut req = request("a", "p1", 0, Direction::Upstream, ServiceClass::OrdinaryFreight);
        req.service_duration_ticks = 0;
        assert_eq!(
            schedule_lock(&profile(), &[req], 0, SchedulingPolicy::FirstComeFirstServed),
            Err(SchedulerError::ZeroServiceDuration)
        );
    }

    #[test]
    fn expired_lock_profile_is_rejected() {
        let mut p = profile();
        p.valid_through_tick = Some(99);
        assert_eq!(
            schedule_lock(&p, &[request("a", "p1", 0, Direction::Upstream, ServiceClass::OrdinaryFreight)], 100, SchedulingPolicy::FirstComeFirstServed),
            Err(SchedulerError::LockProfileExpired)
        );
    }

    #[test]
    fn lock_profile_without_currentness_is_rejected() {
        let mut p = profile();
        p.valid_through_tick = None;
        assert_eq!(
            schedule_lock(&p, &[request("a", "p1", 0, Direction::Upstream, ServiceClass::OrdinaryFreight)], 0, SchedulingPolicy::FirstComeFirstServed),
            Err(SchedulerError::UnknownLockProfileCurrentness)
        );
    }
}
