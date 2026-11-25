// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Distribution-aware maritime service reliability metrics.
//!
//! Mean vessel/terminal time is insufficient when burst arrivals and tail delays
//! dominate schedule reliability. This module reports stage-separated and
//! end-to-end distributions, including p50/p90/p95, maxima, sample standard
//! deviation, coefficient of variation, tail ratio, on-time delivery and
//! per-cohort service floors.
//!
//! Synthetic benchmark/reference implementation only. Not a causal estimator,
//! TOS, operational feed or substitute for qualified source capture, physical
//! feasibility, custody and receiver-acceptance evidence.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};
use std::fmt::Write as FmtWrite;

const ON_TIME_WINDOW_MINUTES: u64 = 120;
const MINIMUM_COHORT_ON_TIME_RATE_MILLI: u16 = 950;
const ARRIVAL_BURST_WINDOW_MINUTES: u64 = 60;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeliveryDisposition {
    ReceiverAccepted,
    NotDelivered,
    Diverted,
    Cancelled,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceObservation {
    pub observation_id: String,
    pub cohort_id: String,
    pub vessel_wait_minutes: u32,
    pub berth_work_minutes: u32,
    pub terminal_dwell_minutes: u32,
    pub inland_transfer_minutes: u32,
    pub receiver_deadline_minutes: u64,
    /// Only populated when an independent receiver record confirms acceptance.
    pub receiver_delivery_minutes: Option<u64>,
    /// Reference to the independently qualified receiver-acceptance receipt.
    pub receiver_acceptance_ref: Option<String>,
    /// Expected units with any other disposition remain in the denominator.
    pub delivery_disposition: DeliveryDisposition,
}

impl ServiceObservation {
    pub fn port_elapsed_minutes(&self) -> u64 {
        u64::from(self.vessel_wait_minutes)
            + u64::from(self.berth_work_minutes)
            + u64::from(self.terminal_dwell_minutes)
    }

    fn staged_elapsed_minutes(&self) -> u64 {
        self.port_elapsed_minutes() + u64::from(self.inland_transfer_minutes)
    }

    pub fn on_time(&self) -> bool {
        self.delivery_disposition == DeliveryDisposition::ReceiverAccepted
            && self
                .receiver_delivery_minutes
                .is_some_and(|minutes| minutes <= self.receiver_deadline_minutes)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VesselArrival {
    pub call_id: String,
    /// Monotonic minutes from a declared benchmark epoch.
    pub arrival_minute: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ServiceMetricError {
    EmptyObservations,
    EmptyObservationIdentity,
    EmptyCohortIdentity,
    DuplicateObservationIdentity,
    ZeroReceiverDeadline,
    ReceiverAcceptanceTimeMissing,
    ReceiverAcceptanceEvidenceMissing,
    NonAcceptedOutcomeHasDeliveryTime,
    NonAcceptedOutcomeHasReceiverEvidence,
    ReceiverDeliveryTimeMismatch,
    EmptyArrivalIdentity,
    DuplicateArrivalIdentity,
    ZeroArrivalWindow,
    ZeroServiceDuration,
    ZeroParallelServers,
    EmptyCohortFloor,
    CohortFloorOutsidePerMille,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DistributionMetrics {
    pub count: usize,
    pub mean_minutes: f64,
    pub p50_minutes: u64,
    pub p90_minutes: u64,
    pub p95_minutes: u64,
    pub maximum_minutes: u64,
    /// Sample standard deviation (n - 1 denominator); None for n < 2.
    pub sample_std_dev_minutes: Option<f64>,
    /// Sample standard deviation divided by mean; None when undefined.
    pub coefficient_of_variation: Option<f64>,
    /// p95 / p50; None when median is zero.
    pub p95_to_p50_ratio: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ServiceMetrics {
    /// Distribution over independently confirmed receiver acceptances only.
    pub distribution: Option<DistributionMetrics>,
    pub expected_count: usize,
    pub receiver_accepted_count: usize,
    pub not_delivered_count: usize,
    pub diverted_count: usize,
    pub cancelled_count: usize,
    pub unknown_count: usize,
    pub on_time_count: usize,
    /// Verified on-time receiver acceptance / all expected units, rounded down.
    pub on_time_rate_milli: u16,
}

impl ServiceMetrics {
    pub fn meets_on_time_floor(&self, floor_per_mille: u16) -> bool {
        self.on_time_rate_milli >= floor_per_mille
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CohortMetrics {
    pub cohort_id: String,
    pub service: ServiceMetrics,
    pub meets_service_floor: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FifoQueueMetrics {
    pub call_count: usize,
    pub max_wait_minutes: u64,
    /// Area under the waiting-queue length curve, in call-minutes.
    pub queue_wait_area_call_minutes: u64,
    pub completion_time_minutes: u64,
    pub wait_distribution: DistributionMetrics,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ServiceReliabilityReport {
    pub observation_count: usize,
    pub vessel_wait: DistributionMetrics,
    pub berth_work: DistributionMetrics,
    pub terminal_dwell: DistributionMetrics,
    pub port_elapsed: DistributionMetrics,
    pub inland_transfer: DistributionMetrics,
    pub end_to_end: ServiceMetrics,
    /// Receipt references for every accepted receiver delivery in this report.
    pub receiver_acceptance_refs: Vec<String>,
    pub cohorts: Vec<CohortMetrics>,
    pub cohort_floor_per_mille: u16,
    pub cohort_floor_violations: usize,
}

fn nearest_rank(sorted: &[u64], percentile: usize) -> u64 {
    let rank = (sorted.len() * percentile).div_ceil(100).max(1);
    sorted[rank - 1]
}

fn distribution(values: &[u64]) -> DistributionMetrics {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let count = sorted.len();
    let sum = sorted.iter().sum::<u64>();
    let mean = sum as f64 / count as f64;
    let variance = if count > 1 {
        let squared = sorted
            .iter()
            .map(|value| {
                let delta = *value as f64 - mean;
                delta * delta
            })
            .sum::<f64>();
        Some(squared / (count - 1) as f64)
    } else {
        None
    };
    let sample_std_dev = variance.map(f64::sqrt);
    let coefficient_of_variation = sample_std_dev.and_then(|std_dev| {
        if mean > 0.0 {
            Some(std_dev / mean)
        } else {
            None
        }
    });
    let p50 = nearest_rank(&sorted, 50);
    let p95 = nearest_rank(&sorted, 95);
    DistributionMetrics {
        count,
        mean_minutes: mean,
        p50_minutes: p50,
        p90_minutes: nearest_rank(&sorted, 90),
        p95_minutes: p95,
        maximum_minutes: sorted[count - 1],
        sample_std_dev_minutes: sample_std_dev,
        coefficient_of_variation,
        p95_to_p50_ratio: (p50 > 0).then_some(p95 as f64 / p50 as f64),
    }
}

fn service_metrics(observations: &[&ServiceObservation]) -> ServiceMetrics {
    let delivered = observations
        .iter()
        .filter_map(|observation| observation.receiver_delivery_minutes)
        .collect::<Vec<_>>();
    let on_time_count = observations
        .iter()
        .filter(|observation| observation.on_time())
        .count();
    let receiver_accepted_count = observations
        .iter()
        .filter(|observation| {
            observation.delivery_disposition == DeliveryDisposition::ReceiverAccepted
        })
        .count();
    let not_delivered_count = observations
        .iter()
        .filter(|observation| {
            observation.delivery_disposition == DeliveryDisposition::NotDelivered
        })
        .count();
    let diverted_count = observations
        .iter()
        .filter(|observation| observation.delivery_disposition == DeliveryDisposition::Diverted)
        .count();
    let cancelled_count = observations
        .iter()
        .filter(|observation| observation.delivery_disposition == DeliveryDisposition::Cancelled)
        .count();
    let unknown_count = observations
        .iter()
        .filter(|observation| observation.delivery_disposition == DeliveryDisposition::Unknown)
        .count();
    let rate = (on_time_count * 1_000 / observations.len()) as u16;
    ServiceMetrics {
        distribution: (!delivered.is_empty()).then(|| distribution(&delivered)),
        expected_count: observations.len(),
        receiver_accepted_count,
        not_delivered_count,
        diverted_count,
        cancelled_count,
        unknown_count,
        on_time_count,
        on_time_rate_milli: rate,
    }
}

impl ServiceReliabilityReport {
    pub fn evaluate(
        observations: &[ServiceObservation],
        cohort_floor_per_mille: u16,
    ) -> Result<Self, ServiceMetricError> {
        if observations.is_empty() {
            return Err(ServiceMetricError::EmptyObservations);
        }
        if cohort_floor_per_mille > 1_000 {
            return Err(ServiceMetricError::CohortFloorOutsidePerMille);
        }
        if cohort_floor_per_mille == 0 {
            return Err(ServiceMetricError::EmptyCohortFloor);
        }

        let mut observation_ids = BTreeSet::new();
        for observation in observations {
            if observation.observation_id.trim().is_empty() {
                return Err(ServiceMetricError::EmptyObservationIdentity);
            }
            if observation.cohort_id.trim().is_empty() {
                return Err(ServiceMetricError::EmptyCohortIdentity);
            }
            if !observation_ids.insert(observation.observation_id.as_str()) {
                return Err(ServiceMetricError::DuplicateObservationIdentity);
            }
            if observation.receiver_deadline_minutes == 0 {
                return Err(ServiceMetricError::ZeroReceiverDeadline);
            }
            match (
                observation.delivery_disposition,
                observation.receiver_delivery_minutes,
                observation.receiver_acceptance_ref.as_deref(),
            ) {
                (DeliveryDisposition::ReceiverAccepted, None, _) => {
                    return Err(ServiceMetricError::ReceiverAcceptanceTimeMissing);
                }
                (DeliveryDisposition::ReceiverAccepted, Some(_), None)
                | (DeliveryDisposition::ReceiverAccepted, Some(_), Some(reference))
                    if reference.trim().is_empty() => {
                    return Err(ServiceMetricError::ReceiverAcceptanceEvidenceMissing);
                }
                (DeliveryDisposition::ReceiverAccepted, Some(minutes), Some(_))
                    if minutes != observation.staged_elapsed_minutes() =>
                {
                    return Err(ServiceMetricError::ReceiverDeliveryTimeMismatch);
                }
                (DeliveryDisposition::NotDelivered
                    | DeliveryDisposition::Diverted
                    | DeliveryDisposition::Cancelled
                    | DeliveryDisposition::Unknown, Some(_), _) =>
                {
                    return Err(ServiceMetricError::NonAcceptedOutcomeHasDeliveryTime);
                }
                (DeliveryDisposition::NotDelivered
                    | DeliveryDisposition::Diverted
                    | DeliveryDisposition::Cancelled
                    | DeliveryDisposition::Unknown, None, Some(_)) =>
                {
                    return Err(ServiceMetricError::NonAcceptedOutcomeHasReceiverEvidence);
                }
                _ => {}
            }
        }

        let vessel_wait = observations
            .iter()
            .map(|item| u64::from(item.vessel_wait_minutes))
            .collect::<Vec<_>>();
        let berth_work = observations
            .iter()
            .map(|item| u64::from(item.berth_work_minutes))
            .collect::<Vec<_>>();
        let terminal_dwell = observations
            .iter()
            .map(|item| u64::from(item.terminal_dwell_minutes))
            .collect::<Vec<_>>();
        let port_elapsed = observations
            .iter()
            .map(ServiceObservation::port_elapsed_minutes)
            .collect::<Vec<_>>();
        let inland_transfer = observations
            .iter()
            .map(|item| u64::from(item.inland_transfer_minutes))
            .collect::<Vec<_>>();
        let mut cohort_rows = BTreeMap::<&str, Vec<&ServiceObservation>>::new();
        for observation in observations {
            cohort_rows
                .entry(observation.cohort_id.as_str())
                .or_default()
                .push(observation);
        }
        let cohorts = cohort_rows
            .into_iter()
            .map(|(cohort_id, cohort_observations)| {
                let service = service_metrics(&cohort_observations);
                CohortMetrics {
                    cohort_id: cohort_id.to_owned(),
                    meets_service_floor: service.meets_on_time_floor(cohort_floor_per_mille),
                    service,
                }
            })
            .collect::<Vec<_>>();

        let cohort_floor_violations = cohorts
            .iter()
            .filter(|cohort| !cohort.meets_service_floor)
            .count();

        let receiver_acceptance_refs = observations
            .iter()
            .filter_map(|observation| observation.receiver_acceptance_ref.clone())
            .collect::<Vec<_>>();

        Ok(Self {
            observation_count: observations.len(),
            vessel_wait: distribution(&vessel_wait),
            berth_work: distribution(&berth_work),
            terminal_dwell: distribution(&terminal_dwell),
            port_elapsed: distribution(&port_elapsed),
            inland_transfer: distribution(&inland_transfer),
            end_to_end: service_metrics(&observations.iter().collect::<Vec<_>>()),
            receiver_acceptance_refs,
            cohorts,
            cohort_floor_per_mille,
            cohort_floor_violations,
        })
    }
}

/// Maximum distinct vessel calls in any inclusive rolling window.
/// This is a burst-load diagnostic, not an independence/cluster estimator.
pub fn maximum_arrivals_in_window(
    arrivals: &[VesselArrival],
    window_minutes: u64,
) -> Result<usize, ServiceMetricError> {
    if window_minutes == 0 {
        return Err(ServiceMetricError::ZeroArrivalWindow);
    }
    let mut call_ids = BTreeSet::new();
    for arrival in arrivals {
        if arrival.call_id.trim().is_empty() {
            return Err(ServiceMetricError::EmptyArrivalIdentity);
        }
        if !call_ids.insert(arrival.call_id.as_str()) {
            return Err(ServiceMetricError::DuplicateArrivalIdentity);
        }
    }
    let mut ordered = arrivals.to_vec();
    ordered.sort_by_key(|arrival| arrival.arrival_minute);
    let mut max_count = 0;
    let mut right = 0;
    for left in 0..ordered.len() {
        right = right.max(left);
        while right < ordered.len()
            && ordered[right].arrival_minute - ordered[left].arrival_minute <= window_minutes
        {
            right += 1;
        }
        max_count = max_count.max(right - left);
    }
    Ok(max_count)
}

/// Replays a deterministic FCFS multi-server queue for a fixed handling
/// duration. This is a transparent queueing baseline, not a terminal optimizer.
pub fn simulate_fcfs_queue(
    arrivals: &[VesselArrival],
    service_duration_minutes: u64,
    parallel_servers: usize,
) -> Result<FifoQueueMetrics, ServiceMetricError> {
    if service_duration_minutes == 0 {
        return Err(ServiceMetricError::ZeroServiceDuration);
    }
    if parallel_servers == 0 {
        return Err(ServiceMetricError::ZeroParallelServers);
    }
    if arrivals.is_empty() {
        return Err(ServiceMetricError::EmptyObservations);
    }
    let mut call_ids = BTreeSet::new();
    for arrival in arrivals {
        if arrival.call_id.trim().is_empty() {
            return Err(ServiceMetricError::EmptyArrivalIdentity);
        }
        if !call_ids.insert(arrival.call_id.as_str()) {
            return Err(ServiceMetricError::DuplicateArrivalIdentity);
        }
    }

    let mut ordered = arrivals.to_vec();
    ordered.sort_by(|left, right| {
        left.arrival_minute
            .cmp(&right.arrival_minute)
            .then_with(|| left.call_id.cmp(&right.call_id))
    });
    let mut server_available = BinaryHeap::new();
    for _ in 0..parallel_servers {
        server_available.push(Reverse(0_u64));
    }

    let mut waits = Vec::with_capacity(ordered.len());
    let mut max_wait = 0_u64;
    let mut queue_wait_area = 0_u64;
    let mut completion_time = 0_u64;
    for arrival in &ordered {
        let Reverse(available_at) = server_available
            .pop()
            .expect("at least one server is configured");
        let start = available_at.max(arrival.arrival_minute);
        let wait = start - arrival.arrival_minute;
        let finish = start + service_duration_minutes;
        server_available.push(Reverse(finish));
        waits.push(wait);
        max_wait = max_wait.max(wait);
        queue_wait_area += wait;
        completion_time = completion_time.max(finish);
    }

    Ok(FifoQueueMetrics {
        call_count: ordered.len(),
        max_wait_minutes: max_wait,
        queue_wait_area_call_minutes: queue_wait_area,
        completion_time_minutes: completion_time,
        wait_distribution: distribution(&waits),
    })
}

fn synthetic_observations(
    scenario: &str,
    stages: &[(u32, u32, u32, u32)],
    cohorts: &[&str],
    deadline: u64,
) -> Vec<ServiceObservation> {
    stages
        .iter()
        .enumerate()
        .map(|(index, (vessel, berth, terminal, inland))| ServiceObservation {
            observation_id: format!("{scenario}-cargo-{index:02}"),
            cohort_id: cohorts.get(index).copied().unwrap_or("ALL").to_owned(),
            vessel_wait_minutes: *vessel,
            berth_work_minutes: *berth,
            terminal_dwell_minutes: *terminal,
            inland_transfer_minutes: *inland,
            receiver_deadline_minutes: deadline,
            receiver_delivery_minutes: Some(
                u64::from(*vessel)
                    + u64::from(*berth)
                    + u64::from(*terminal)
                    + u64::from(*inland),
            ),
            receiver_acceptance_ref: Some(format!("synthetic-receiver-receipt-{scenario}-{index:02}")),
            delivery_disposition: DeliveryDisposition::ReceiverAccepted,
        })
        .collect()
}

fn regular_arrivals(prefix: &str, count: usize) -> Vec<VesselArrival> {
    (0..count)
        .map(|index| VesselArrival {
            call_id: format!("{prefix}-vessel-{index:02}"),
            arrival_minute: index as u64 * 60,
        })
        .collect()
}

fn burst_arrivals(prefix: &str, count: usize) -> Vec<VesselArrival> {
    (0..count)
        .map(|index| VesselArrival {
            call_id: format!("{prefix}-vessel-{index:02}"),
            arrival_minute: if index < count.saturating_sub(2) { 0 } else { 10 },
        })
        .collect()
}

#[derive(Clone, Copy)]
struct Scenario {
    id: &'static str,
    observations: &'static [(u32, u32, u32, u32)],
    cohorts: &'static [&'static str],
    deadline: u64,
    burst: bool,
    include_unresolved_outcomes: bool,
}

const STABLE: [(u32, u32, u32, u32); 20] = [(50, 20, 20, 10); 20];
const MEAN_MATCHED_BURSTY: [(u32, u32, u32, u32); 20] = [
    (40, 15, 15, 10),
    (40, 15, 15, 10),
    (40, 15, 15, 10),
    (40, 15, 15, 10),
    (40, 15, 15, 10),
    (40, 15, 15, 10),
    (40, 15, 15, 10),
    (40, 15, 15, 10),
    (40, 15, 15, 10),
    (40, 15, 15, 10),
    (40, 15, 15, 10),
    (40, 15, 15, 10),
    (40, 15, 15, 10),
    (40, 15, 15, 10),
    (40, 15, 15, 10),
    (40, 15, 15, 10),
    (40, 15, 15, 10),
    (40, 15, 15, 10),
    (100, 60, 70, 50),
    (100, 60, 70, 50),
];
const BASELINE_CHAIN: [(u32, u32, u32, u32); 20] = [(60, 40, 30, 20); 20];
const LOCAL_PORT_GAIN: [(u32, u32, u32, u32); 20] = [(40, 40, 30, 40); 20];
const RECEIVER_REGRESSION: [(u32, u32, u32, u32); 20] = [(40, 40, 30, 80); 20];
const FAIRNESS_COHORTS: [&str; 20] = [
    "PRIORITY_A", "PRIORITY_A", "PRIORITY_A", "PRIORITY_A", "PRIORITY_A",
    "PRIORITY_A", "PRIORITY_A", "PRIORITY_A", "PRIORITY_A", "PRIORITY_A",
    "PRIORITY_A", "PRIORITY_A", "PRIORITY_A", "PRIORITY_A", "PRIORITY_A",
    "PRIORITY_A", "PRIORITY_A", "PRIORITY_A", "SMALL_RECEIVER", "SMALL_RECEIVER",
];

fn scenarios() -> [Scenario; 7] {
    [
        Scenario {
            id: "MEAN_MATCHED_STABLE",
            observations: &STABLE,
            cohorts: &[],
            deadline: 120,
            burst: false,
            include_unresolved_outcomes: false,
        },
        Scenario {
            id: "MEAN_MATCHED_HIGH_VARIABILITY",
            observations: &MEAN_MATCHED_BURSTY,
            cohorts: &[],
            deadline: 120,
            burst: true,
            include_unresolved_outcomes: false,
        },
        Scenario {
            id: "BASELINE_END_TO_END",
            observations: &BASELINE_CHAIN,
            cohorts: &[],
            deadline: 160,
            burst: false,
            include_unresolved_outcomes: false,
        },
        Scenario {
            id: "LOCAL_PORT_GAIN_NO_RECEIVER_GAIN",
            observations: &LOCAL_PORT_GAIN,
            cohorts: &[],
            deadline: 160,
            burst: false,
            include_unresolved_outcomes: false,
        },
        Scenario {
            id: "RECEIVER_REGRESSION",
            observations: &RECEIVER_REGRESSION,
            cohorts: &[],
            deadline: 160,
            burst: true,
            include_unresolved_outcomes: false,
        },
        Scenario {
            id: "FAIRNESS_COHORT_FLOOR_HIDDEN_BY_AGGREGATE",
            observations: &MEAN_MATCHED_BURSTY,
            cohorts: &FAIRNESS_COHORTS,
            deadline: 120,
            burst: true,
            include_unresolved_outcomes: false,
        },
        Scenario {
            id: "EXPECTED_UNITS_WITH_UNRESOLVED_DELIVERY",
            observations: &STABLE,
            cohorts: &[],
            deadline: 120,
            burst: false,
            include_unresolved_outcomes: true,
        },
    ]
}

fn optional_float(value: Option<f64>) -> String {
    value.map_or_else(|| "NA".to_owned(), |number| format!("{number:.6}"))
}

/// Renders deterministic synthetic service-reliability and burst receipts.
pub fn render_csv() -> String {
    let mut output = String::from(
        "scenario_id,n,port_mean_minutes,inland_mean_minutes,end_to_end_mean_minutes,p50_minutes,p90_minutes,p95_minutes,max_minutes,sample_sd_minutes,coefficient_of_variation,p95_to_p50_ratio,on_time_rate_per_mille,cohort_floor_violations,max_arrivals_in_60m,max_fcfs_wait_minutes,queue_wait_area_call_minutes,queue_completion_minutes,claim_ceiling\n",
    );
    for scenario in scenarios() {
        let mut observations = synthetic_observations(
            scenario.id,
            scenario.observations,
            scenario.cohorts,
            scenario.deadline,
        );
        if scenario.include_unresolved_outcomes {
            let not_delivered = observations.len() - 2;
            observations[not_delivered].delivery_disposition = DeliveryDisposition::NotDelivered;
            observations[not_delivered].receiver_delivery_minutes = None;
            observations[not_delivered].receiver_acceptance_ref = None;
            let unknown = observations.len() - 1;
            observations[unknown].delivery_disposition = DeliveryDisposition::Unknown;
            observations[unknown].receiver_delivery_minutes = None;
            observations[unknown].receiver_acceptance_ref = None;
        }
        let report = ServiceReliabilityReport::evaluate(
            &observations,
            MINIMUM_COHORT_ON_TIME_RATE_MILLI,
        )
        .expect("valid synthetic service scenario");
        let arrivals = if scenario.burst {
            burst_arrivals(scenario.id, observations.len())
        } else {
            regular_arrivals(scenario.id, observations.len())
        };
        let burst = maximum_arrivals_in_window(&arrivals, ARRIVAL_BURST_WINDOW_MINUTES)
            .expect("valid synthetic arrivals");
        let queue = simulate_fcfs_queue(&arrivals, 30, 2).expect("valid FCFS queue fixture");
        let delivered = report
            .end_to_end
            .distribution
            .as_ref()
            .expect("each synthetic benchmark scenario has receiver acceptances");
        let _ = writeln!(
            output,
            "{},{},{:.3},{:.3},{:.3},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},SYNTHETIC_ONLY",
            scenario.id,
            report.observation_count,
            report.port_elapsed.mean_minutes,
            report.inland_transfer.mean_minutes,
            delivered.mean_minutes,
            delivered.p50_minutes,
            delivered.p90_minutes,
            delivered.p95_minutes,
            delivered.maximum_minutes,
            optional_float(delivered.sample_std_dev_minutes),
            optional_float(delivered.coefficient_of_variation),
            optional_float(delivered.p95_to_p50_ratio),
            report.end_to_end.on_time_rate_milli,
            report.cohort_floor_violations,
            burst,
            queue.max_wait_minutes,
            queue.queue_wait_area_call_minutes,
            queue.completion_time_minutes,
            report.end_to_end.receiver_accepted_count,
            report.end_to_end.not_delivered_count,
            report.end_to_end.diverted_count,
            report.end_to_end.cancelled_count,
            report.end_to_end.unknown_count
        );
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn equal_mean_pair() -> (ServiceReliabilityReport, ServiceReliabilityReport) {
        let stable = synthetic_observations(
            "stable",
            &STABLE,
            &[],
            ON_TIME_WINDOW_MINUTES,
        );
        let bursty =
            synthetic_observations("bursty", &MEAN_MATCHED_BURSTY, &[], ON_TIME_WINDOW_MINUTES);
        (
            ServiceReliabilityReport::evaluate(&stable, 950).expect("stable report"),
            ServiceReliabilityReport::evaluate(&bursty, 950).expect("bursty report"),
        )
    }

    #[test]
    fn mean_matched_scenarios_expose_hidden_variability_and_tail_loss() {
        let (stable, bursty) = equal_mean_pair();
        assert_eq!(
            stable.end_to_end.distribution.mean_minutes,
            bursty.end_to_end.distribution.mean_minutes
        );
        assert_eq!(stable.end_to_end.distribution.p95_minutes, 100);
        assert_eq!(bursty.end_to_end.distribution.p95_minutes, 280);
        assert_eq!(stable.end_to_end.on_time_rate_milli, 1_000);
        assert_eq!(bursty.end_to_end.on_time_rate_milli, 900);
        assert!(
            bursty.end_to_end.distribution.coefficient_of_variation
                > stable.end_to_end.distribution.coefficient_of_variation
        );
    }

    #[test]
    fn local_port_gain_cannot_be_claimed_as_end_to_end_service_gain() {
        let baseline =
            synthetic_observations("baseline", &BASELINE_CHAIN, &[], 160);
        let optimized =
            synthetic_observations("optimized", &LOCAL_PORT_GAIN, &[], 160);
        let baseline = ServiceReliabilityReport::evaluate(&baseline, 950).expect("baseline");
        let optimized = ServiceReliabilityReport::evaluate(&optimized, 950).expect("optimized");
        assert_eq!(
            optimized.port_elapsed.mean_minutes + 20.0,
            baseline.port_elapsed.mean_minutes
        );
        assert_eq!(
            optimized.end_to_end.distribution.mean_minutes,
            baseline.end_to_end.distribution.mean_minutes
        );
    }

    #[test]
    fn receiver_delay_can_worsen_even_when_port_time_improves() {
        let baseline =
            synthetic_observations("baseline", &BASELINE_CHAIN, &[], 160);
        let regressed =
            synthetic_observations("regressed", &RECEIVER_REGRESSION, &[], 160);
        let baseline = ServiceReliabilityReport::evaluate(&baseline, 950).expect("baseline");
        let regressed = ServiceReliabilityReport::evaluate(&regressed, 950).expect("regressed");
        assert!(
            regressed.port_elapsed.mean_minutes < baseline.port_elapsed.mean_minutes
        );
        assert!(
            regressed.end_to_end.distribution.mean_minutes
                > baseline.end_to_end.distribution.mean_minutes
        );
        assert_eq!(regressed.end_to_end.on_time_rate_milli, 0);
    }

    #[test]
    fn participant_floor_is_per_cohort_not_aggregated_away() {
        let observations = synthetic_observations(
            "fairness",
            &MEAN_MATCHED_BURSTY,
            &FAIRNESS_COHORTS,
            120,
        );
        let report = ServiceReliabilityReport::evaluate(&observations, 950).expect("fairness");
        assert_eq!(report.cohort_floor_violations, 1);
        let small = report
            .cohorts
            .iter()
            .find(|cohort| cohort.cohort_id == "SMALL_RECEIVER")
            .expect("small receiver cohort");
        assert_eq!(small.service.on_time_rate_milli, 0);
        assert!(!small.meets_service_floor);
    }

    #[test]
    fn burst_arrival_metric_exposes_clustered_calls() {
        let regular = maximum_arrivals_in_window(&regular_arrivals("regular", 20), 60)
            .expect("regular arrivals");
        let burst = maximum_arrivals_in_window(&burst_arrivals("burst", 20), 60)
            .expect("burst arrivals");
        assert_eq!(regular, 2);
        assert_eq!(burst, 20);
    }

    #[test]
    fn burst_arrivals_create_fcfs_queue_wait_and_longer_recovery() {
        let regular =
            simulate_fcfs_queue(&regular_arrivals("regular", 20), 30, 2).expect("regular queue");
        let burst =
            simulate_fcfs_queue(&burst_arrivals("burst", 20), 30, 2).expect("burst queue");
        assert_eq!(regular.max_wait_minutes, 0);
        assert_eq!(regular.queue_wait_area_call_minutes, 0);
        assert_eq!(regular.completion_time_minutes, 1_170);
        assert_eq!(burst.max_wait_minutes, 260);
        assert_eq!(burst.queue_wait_area_call_minutes, 2_680);
        assert_eq!(burst.completion_time_minutes, 300);
    }

    #[test]
    fn fcfs_queue_rejects_zero_capacity_and_empty_arrivals() {
        let arrivals = regular_arrivals("queue", 1);
        assert_eq!(
            simulate_fcfs_queue(&arrivals, 30, 0),
            Err(ServiceMetricError::ZeroParallelServers)
        );
        assert_eq!(
            simulate_fcfs_queue(&arrivals, 0, 2),
            Err(ServiceMetricError::ZeroServiceDuration)
        );
        assert_eq!(
            simulate_fcfs_queue(&[], 30, 2),
            Err(ServiceMetricError::EmptyObservations)
        );
    }

    #[test]
    fn arrival_ids_must_be_unique_and_window_must_be_positive() {
        let arrivals = vec![
            VesselArrival {
                call_id: "same".to_owned(),
                arrival_minute: 0,
            },
            VesselArrival {
                call_id: "same".to_owned(),
                arrival_minute: 10,
            },
        ];
        assert_eq!(
            maximum_arrivals_in_window(&arrivals, 60),
            Err(ServiceMetricError::DuplicateArrivalIdentity)
        );
        assert_eq!(
            maximum_arrivals_in_window(&[], 0),
            Err(ServiceMetricError::ZeroArrivalWindow)
        );
    }

    #[test]
    fn invalid_service_rows_are_rejected() {
        assert_eq!(
            ServiceReliabilityReport::evaluate(&[], 950),
            Err(ServiceMetricError::EmptyObservations)
        );
        let mut observations = synthetic_observations("test", &STABLE[..2], &[], 120);
        observations[1].observation_id = observations[0].observation_id.clone();
        assert_eq!(
            ServiceReliabilityReport::evaluate(&observations, 950),
            Err(ServiceMetricError::DuplicateObservationIdentity)
        );
        observations[1].observation_id = "distinct".to_owned();
        observations[0].receiver_deadline_minutes = 0;
        assert_eq!(
            ServiceReliabilityReport::evaluate(&observations, 950),
            Err(ServiceMetricError::ZeroReceiverDeadline)
        );
    }

    #[test]
    fn zero_mean_and_single_observation_leave_cv_undefined() {
        let observations = vec![ServiceObservation {
            observation_id: "zero".to_owned(),
            cohort_id: "cohort".to_owned(),
            vessel_wait_minutes: 0,
            berth_work_minutes: 0,
            terminal_dwell_minutes: 0,
            inland_transfer_minutes: 0,
            receiver_deadline_minutes: 1,
        }];
        let report = ServiceReliabilityReport::evaluate(&observations, 950).expect("report");
        assert_eq!(report.end_to_end.distribution.mean_minutes, 0.0);
        assert_eq!(report.end_to_end.distribution.coefficient_of_variation, None);
        assert_eq!(report.end_to_end.distribution.sample_std_dev_minutes, None);
    }

    #[test]
    fn deterministic_receipt_is_byte_identical_and_has_fixed_schema() {
        let first = render_csv();
        assert_eq!(first, render_csv());
        assert_eq!(first.lines().count(), 7);
        let width = first.lines().next().expect("CSV header").split(',').count();
        assert_eq!(width, 19);
        for row in first.lines().skip(1) {
            assert_eq!(row.split(',').count(), width);
        }
        assert!(first.contains("MEAN_MATCHED_HIGH_VARIABILITY"));
        assert!(first.contains("LOCAL_PORT_GAIN_NO_RECEIVER_GAIN"));
        assert!(first.contains("RECEIVER_REGRESSION"));
        assert!(first.contains("FAIRNESS_COHORT_FLOOR_HIDDEN_BY_AGGREGATE"));
    }
}
