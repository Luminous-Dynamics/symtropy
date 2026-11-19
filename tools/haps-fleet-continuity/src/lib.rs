//! Deterministic event replay for HAPS fleet and service continuity.
//!
//! This crate separates HAPS-only availability from end-to-end critical/full
//! service. It applies explicit common-cause weather and network events to
//! policy-specific maintenance and reserve schedules. It is not a stochastic
//! reliability estimator, weather forecast, aircraft dynamics, RF propagation,
//! battery, or thermal solver.

use std::collections::{HashMap, HashSet};
use std::fmt;

use serde::{Deserialize, Serialize};

const EPSILON: f64 = 1.0e-9;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FleetError {
    Json(String),
    InvalidCase(String),
    UnknownPlatform(String),
    InvalidEvent(String),
    OracleMismatch(String),
}

impl fmt::Display for FleetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(message)
            | Self::InvalidCase(message)
            | Self::UnknownPlatform(message)
            | Self::InvalidEvent(message)
            | Self::OracleMismatch(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for FleetError {}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleetContinuityCase {
    pub schema_version: String,
    pub case_id: String,
    pub revision: u64,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub timebase: Timebase,
    pub service_contract: ServiceContract,
    pub fleet: Vec<FleetPlatform>,
    pub common_cause_events: Vec<CommonCauseEvent>,
    pub policy_cases: Vec<PolicyCase>,
    pub expected_oracle: ExpectedOracle,
    pub source_evidence_refs: Vec<String>,
    pub assumptions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Timebase {
    pub horizon_hours: f64,
    pub epoch_utc: Option<String>,
    pub time_step_hours: f64,
    pub interval_convention: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceContract {
    pub service_id: String,
    pub critical_demand_capacity_units: f64,
    pub full_demand_capacity_units: f64,
    pub normal_terrestrial_capacity_units: f64,
    pub normal_satellite_capacity_units: f64,
    pub capacity_interpretation: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleetPlatform {
    pub platform_id: String,
    pub lift_class: String,
    pub service_capacity_units: f64,
    pub initial_state: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommonCauseEvent {
    pub event_id: String,
    pub event_class: String,
    pub start_hour: f64,
    pub duration_hours: f64,
    pub scope: String,
    pub affected_platform_ids: Option<Vec<String>>,
    pub haps_capacity_factor: f64,
    pub terrestrial_capacity_override_units: Option<f64>,
    pub satellite_capacity_override_units: Option<f64>,
    pub parent_event_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyCase {
    pub policy_id: String,
    pub policy_class: String,
    pub description: String,
    pub operating_platform_ids: Vec<String>,
    pub platform_unavailability_events: Vec<PlatformUnavailabilityEvent>,
    pub reserve_dispatch_events: Vec<ReserveDispatchEvent>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformUnavailabilityEvent {
    pub event_id: String,
    pub platform_id: String,
    pub event_class: String,
    pub start_hour: f64,
    pub duration_hours: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReserveDispatchEvent {
    pub platform_id: String,
    pub requested_at_hour: f64,
    pub on_station_at_hour: f64,
    pub released_at_hour: f64,
    pub dispatch_delay_hours: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedOracle {
    pub oracle_status: String,
    pub policy_results: Vec<FleetPolicyResult>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FleetPolicyResult {
    pub policy_id: String,
    pub haps_only_full_service_availability_fraction: f64,
    pub critical_service_availability_fraction: f64,
    pub full_service_availability_fraction: f64,
    pub useful_service_capacity_fraction: f64,
    pub full_service_degraded_hours: f64,
    pub critical_service_outage_hours: f64,
    pub full_service_degradation_episode_count: u64,
    pub max_full_service_degradation_hours: f64,
    pub max_critical_service_outage_hours: f64,
    pub planned_maintenance_hours: f64,
    pub reserve_dispatch_delay_hours: f64,
    pub minimum_on_station_haps_capacity_units: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FleetContinuityReport {
    pub case_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub evaluated_hours: f64,
    pub time_step_hours: f64,
    pub policy_results: Vec<FleetPolicyResult>,
}

pub fn parse_case(input: &str) -> Result<FleetContinuityCase, FleetError> {
    serde_json::from_str(input).map_err(|error| FleetError::Json(error.to_string()))
}

pub fn evaluate_case(input: &str) -> Result<FleetContinuityReport, FleetError> {
    let case = parse_case(input)?;
    evaluate(&case)
}

pub fn evaluate(case: &FleetContinuityCase) -> Result<FleetContinuityReport, FleetError> {
    validate_case(case)?;

    let horizon = case.timebase.horizon_hours;
    let step = case.timebase.time_step_hours;
    let steps = (horizon / step).round() as usize;
    let platforms: HashMap<&str, &FleetPlatform> = case
        .fleet
        .iter()
        .map(|platform| (platform.platform_id.as_str(), platform))
        .collect();

    let mut results = Vec::with_capacity(case.policy_cases.len());
    for policy in &case.policy_cases {
        results.push(evaluate_policy(case, policy, &platforms, steps)?);
    }

    Ok(FleetContinuityReport {
        case_id: case.case_id.clone(),
        evidence_class: case.evidence_class.clone(),
        claim_ceiling: case.claim_ceiling.clone(),
        evaluated_hours: horizon,
        time_step_hours: step,
        policy_results: results,
    })
}

pub fn verify_expected(
    report: &FleetContinuityReport,
    expected: &ExpectedOracle,
) -> Result<(), FleetError> {
    if expected.oracle_status != "independently_calculated" {
        return Err(FleetError::OracleMismatch(
            "expected oracle is not marked independently_calculated".to_owned(),
        ));
    }

    if report.policy_results.len() != expected.policy_results.len() {
        return Err(FleetError::OracleMismatch(format!(
            "policy count differs: report={}, expected={}",
            report.policy_results.len(),
            expected.policy_results.len()
        )));
    }

    for (actual, reference) in report.policy_results.iter().zip(&expected.policy_results) {
        if actual.policy_id != reference.policy_id {
            return Err(FleetError::OracleMismatch(format!(
                "policy order/id differs: report={}, expected={}",
                actual.policy_id, reference.policy_id
            )));
        }

        compare_metric(
            &actual.policy_id,
            "haps_only_full_service_availability_fraction",
            actual.haps_only_full_service_availability_fraction,
            reference.haps_only_full_service_availability_fraction,
        )?;
        compare_metric(
            &actual.policy_id,
            "critical_service_availability_fraction",
            actual.critical_service_availability_fraction,
            reference.critical_service_availability_fraction,
        )?;
        compare_metric(
            &actual.policy_id,
            "full_service_availability_fraction",
            actual.full_service_availability_fraction,
            reference.full_service_availability_fraction,
        )?;
        compare_metric(
            &actual.policy_id,
            "useful_service_capacity_fraction",
            actual.useful_service_capacity_fraction,
            reference.useful_service_capacity_fraction,
        )?;
        compare_metric(
            &actual.policy_id,
            "full_service_degraded_hours",
            actual.full_service_degraded_hours,
            reference.full_service_degraded_hours,
        )?;
        compare_metric(
            &actual.policy_id,
            "critical_service_outage_hours",
            actual.critical_service_outage_hours,
            reference.critical_service_outage_hours,
        )?;
        if actual.full_service_degradation_episode_count
            != reference.full_service_degradation_episode_count
        {
            return Err(FleetError::OracleMismatch(format!(
                "{}.full_service_degradation_episode_count: {} != {}",
                actual.policy_id,
                actual.full_service_degradation_episode_count,
                reference.full_service_degradation_episode_count
            )));
        }
        compare_metric(
            &actual.policy_id,
            "max_full_service_degradation_hours",
            actual.max_full_service_degradation_hours,
            reference.max_full_service_degradation_hours,
        )?;
        compare_metric(
            &actual.policy_id,
            "max_critical_service_outage_hours",
            actual.max_critical_service_outage_hours,
            reference.max_critical_service_outage_hours,
        )?;
        compare_metric(
            &actual.policy_id,
            "planned_maintenance_hours",
            actual.planned_maintenance_hours,
            reference.planned_maintenance_hours,
        )?;
        compare_metric(
            &actual.policy_id,
            "reserve_dispatch_delay_hours",
            actual.reserve_dispatch_delay_hours,
            reference.reserve_dispatch_delay_hours,
        )?;
        compare_metric(
            &actual.policy_id,
            "minimum_on_station_haps_capacity_units",
            actual.minimum_on_station_haps_capacity_units,
            reference.minimum_on_station_haps_capacity_units,
        )?;
    }

    Ok(())
}

fn compare_metric(
    policy_id: &str,
    metric_name: &str,
    actual: f64,
    expected: f64,
) -> Result<(), FleetError> {
    if !actual.is_finite() || !expected.is_finite() || (actual - expected).abs() > EPSILON {
        return Err(FleetError::OracleMismatch(format!(
            "{policy_id}.{metric_name}: {actual} != {expected}"
        )));
    }
    Ok(())
}

fn validate_case(case: &FleetContinuityCase) -> Result<(), FleetError> {
    if case.schema_version != "haps-fleet-continuity-case/v1" {
        return Err(FleetError::InvalidCase(
            "unsupported fleet continuity schema_version".to_owned(),
        ));
    }
    if case.evidence_class != "synthetic" || case.claim_ceiling != "synthetic_event_replay" {
        return Err(FleetError::InvalidCase(
            "this evaluator only accepts the synthetic_event_replay evidence boundary".to_owned(),
        ));
    }
    if case.timebase.interval_convention != "half_open_start_inclusive_end_exclusive" {
        return Err(FleetError::InvalidCase(
            "unsupported interval convention".to_owned(),
        ));
    }

    let horizon = case.timebase.horizon_hours;
    let step = case.timebase.time_step_hours;
    if !horizon.is_finite() || !step.is_finite() || horizon <= 0.0 || step <= 0.0 {
        return Err(FleetError::InvalidCase(
            "timebase horizon and step must be finite and positive".to_owned(),
        ));
    }
    let steps = horizon / step;
    if (steps - steps.round()).abs() > EPSILON {
        return Err(FleetError::InvalidCase(
            "horizon_hours must be an integer multiple of time_step_hours".to_owned(),
        ));
    }

    let critical = case.service_contract.critical_demand_capacity_units;
    let full = case.service_contract.full_demand_capacity_units;
    if !critical.is_finite()
        || !full.is_finite()
        || critical <= 0.0
        || full < critical
        || !case
            .service_contract
            .normal_terrestrial_capacity_units
            .is_finite()
        || !case
            .service_contract
            .normal_satellite_capacity_units
            .is_finite()
        || case.service_contract.normal_terrestrial_capacity_units < 0.0
        || case.service_contract.normal_satellite_capacity_units < 0.0
    {
        return Err(FleetError::InvalidCase(
            "service capacity thresholds or fallback capacities are invalid".to_owned(),
        ));
    }

    let mut platform_ids = HashSet::new();
    for platform in &case.fleet {
        if platform.platform_id.trim().is_empty()
            || !platform_ids.insert(platform.platform_id.as_str())
            || !platform.service_capacity_units.is_finite()
            || platform.service_capacity_units < 0.0
            || !["on_station", "standby_ready"].contains(&platform.initial_state.as_str())
        {
            return Err(FleetError::InvalidCase(
                "fleet IDs must be unique and service capacities finite/non-negative".to_owned(),
            ));
        }
    }
    if platform_ids.is_empty() {
        return Err(FleetError::InvalidCase("fleet cannot be empty".to_owned()));
    }

    let mut common_ids = HashMap::new();
    for event in &case.common_cause_events {
        validate_interval(
            &event.event_id,
            event.start_hour,
            event.duration_hours,
            horizon,
        )?;
        validate_grid_alignment(
            &event.event_id,
            event.start_hour,
            event.duration_hours,
            step,
        )?;
        if !event.haps_capacity_factor.is_finite()
            || !(0.0..=1.0).contains(&event.haps_capacity_factor)
            || ![
                "all_haps_in_service_region",
                "network_layer",
                "declared_platform_subset",
            ]
            .contains(&event.scope.as_str())
            || event
                .terrestrial_capacity_override_units
                .is_some_and(|value| !value.is_finite() || value < 0.0)
            || event
                .satellite_capacity_override_units
                .is_some_and(|value| !value.is_finite() || value < 0.0)
        {
            return Err(FleetError::InvalidEvent(format!(
                "{} has invalid capacity factors/overrides",
                event.event_id
            )));
        }
        if event.scope == "declared_platform_subset" {
            let affected = event.affected_platform_ids.as_ref().ok_or_else(|| {
                FleetError::InvalidEvent(format!(
                    "{} must declare affected_platform_ids",
                    event.event_id
                ))
            })?;
            for id in affected {
                if !platform_ids.contains(id.as_str()) {
                    return Err(FleetError::UnknownPlatform(id.clone()));
                }
            }
        }
        if common_ids.insert(event.event_id.as_str(), event).is_some() {
            return Err(FleetError::InvalidEvent(format!(
                "duplicate common-cause event id {}",
                event.event_id
            )));
        }
    }
    for event in &case.common_cause_events {
        let mut visited_parent_ids = HashSet::new();
        visited_parent_ids.insert(event.event_id.as_str());
        let mut next_parent_id = event.parent_event_id.as_deref();
        while let Some(parent_id) = next_parent_id {
            if !visited_parent_ids.insert(parent_id) {
                return Err(FleetError::InvalidEvent(format!(
                    "{} participates in a common-cause parent cycle",
                    event.event_id
                )));
            }
            let parent = common_ids.get(parent_id).ok_or_else(|| {
                FleetError::InvalidEvent(format!(
                    "{} references unknown parent event {}",
                    event.event_id, parent_id
                ))
            })?;
            let event_end = event.start_hour + event.duration_hours;
            let parent_end = parent.start_hour + parent.duration_hours;
            if event.start_hour + EPSILON < parent.start_hour
                || event_end > parent_end + EPSILON
            {
                return Err(FleetError::InvalidEvent(format!(
                    "{} is not contained by parent event {}",
                    event.event_id, parent_id
                )));
            }
            next_parent_id = parent.parent_event_id.as_deref();
        }
    }

    let mut policy_ids = HashSet::new();
    for policy in &case.policy_cases {
        if policy.policy_id.trim().is_empty()
            || !policy_ids.insert(policy.policy_id.as_str())
            || policy.operating_platform_ids.is_empty()
        {
            return Err(FleetError::InvalidCase(
                "policy IDs must be unique and each policy must operate at least one platform"
                    .to_owned(),
            ));
        }
        let mut operating = HashSet::new();
        for id in &policy.operating_platform_ids {
            if !platform_ids.contains(id.as_str()) {
                return Err(FleetError::UnknownPlatform(id.clone()));
            }
            if !operating.insert(id.as_str()) {
                return Err(FleetError::InvalidCase(format!(
                    "{} repeats operating platform {}",
                    policy.policy_id, id
                )));
            }
        }

        let reserved_ids: HashSet<&str> = policy
            .reserve_dispatch_events
            .iter()
            .map(|event| event.platform_id.as_str())
            .collect();
        for platform_id in &policy.operating_platform_ids {
            if platforms[platform_id.as_str()].initial_state != "on_station" {
                return Err(FleetError::InvalidEvent(format!(
                    "{} lists {} as operating but its initial_state is not on_station",
                    policy.policy_id, platform_id
                )));
            }
        }

        let mut per_platform_events: HashMap<&str, Vec<(f64, f64, &str)>> = HashMap::new();
        for event in &policy.platform_unavailability_events {
            if !platform_ids.contains(event.platform_id.as_str()) {
                return Err(FleetError::UnknownPlatform(event.platform_id.clone()));
            }
            if !operating.contains(event.platform_id.as_str())
                && !reserved_ids.contains(event.platform_id.as_str())
            {
                return Err(FleetError::InvalidEvent(format!(
                    "{} applies an unavailability event to a platform that is neither operating nor dispatched as reserve",
                    event.event_id
                )));
            }
            validate_interval(
                &event.event_id,
                event.start_hour,
                event.duration_hours,
                horizon,
            )?;
            validate_grid_alignment(
                &event.event_id,
                event.start_hour,
                event.duration_hours,
                step,
            )?;
            per_platform_events
                .entry(event.platform_id.as_str())
                .or_default()
                .push((
                    event.start_hour,
                    event.start_hour + event.duration_hours,
                    event.event_id.as_str(),
                ));
        }
        for (platform_id, events) in per_platform_events.iter_mut() {
            events.sort_by(|left, right| left.0.total_cmp(&right.0));
            for pair in events.windows(2) {
                if pair[1].0 < pair[0].1 - EPSILON {
                    return Err(FleetError::InvalidEvent(format!(
                        "{} has overlapping unavailability events on {}: {} and {}",
                        policy.policy_id, platform_id, pair[0].2, pair[1].2
                    )));
                }
            }
        }

        let mut reserve_service_windows: HashMap<&str, Vec<(f64, f64)>> = HashMap::new();
        for dispatch in &policy.reserve_dispatch_events {
            if !platform_ids.contains(dispatch.platform_id.as_str()) {
                return Err(FleetError::UnknownPlatform(dispatch.platform_id.clone()));
            }
            if platforms[dispatch.platform_id.as_str()].initial_state != "standby_ready" {
                return Err(FleetError::InvalidEvent(format!(
                    "{} reserve {} is not in standby_ready initial state",
                    policy.policy_id, dispatch.platform_id
                )));
            }
            if operating.contains(dispatch.platform_id.as_str()) {
                return Err(FleetError::InvalidEvent(format!(
                    "{} reserve {} is already in operating_platform_ids",
                    policy.policy_id, dispatch.platform_id
                )));
            }
            let values = [
                dispatch.requested_at_hour,
                dispatch.on_station_at_hour,
                dispatch.released_at_hour,
                dispatch.dispatch_delay_hours,
            ];
            if values.iter().any(|value| !value.is_finite())
                || dispatch.requested_at_hour < 0.0
                || dispatch.on_station_at_hour < dispatch.requested_at_hour
                || dispatch.released_at_hour <= dispatch.on_station_at_hour + EPSILON
                || dispatch.released_at_hour > horizon
                || (dispatch.on_station_at_hour
                    - dispatch.requested_at_hour
                    - dispatch.dispatch_delay_hours)
                    .abs()
                    > EPSILON
                || [
                    dispatch.requested_at_hour,
                    dispatch.on_station_at_hour,
                    dispatch.released_at_hour,
                ]
                .iter()
                .any(|value| !is_grid_aligned(*value, step))
            {
                return Err(FleetError::InvalidEvent(format!(
                    "{} has an invalid reserve dispatch interval/delay",
                    policy.policy_id
                )));
            }
            reserve_service_windows
                .entry(dispatch.platform_id.as_str())
                .or_default()
                .push((dispatch.on_station_at_hour, dispatch.released_at_hour));
        }
        for (platform_id, windows) in reserve_service_windows.iter_mut() {
            windows.sort_by(|left, right| left.0.total_cmp(&right.0));
            for pair in windows.windows(2) {
                if pair[1].0 < pair[0].1 - EPSILON {
                    return Err(FleetError::InvalidEvent(format!(
                        "{} has overlapping reserve dispatch windows for {}",
                        policy.policy_id, platform_id
                    )));
                }
            }
        }
    }

    if case.policy_cases.len() < 2 {
        return Err(FleetError::InvalidCase(
            "at least two policies are required for matched comparison".to_owned(),
        ));
    }

    Ok(())
}

fn is_grid_aligned(value: f64, step: f64) -> bool {
    ((value / step) - (value / step).round()).abs() <= EPSILON
}

fn validate_grid_alignment(
    event_id: &str,
    start: f64,
    duration: f64,
    step: f64,
) -> Result<(), FleetError> {
    if !is_grid_aligned(start, step) || !is_grid_aligned(duration, step) {
        return Err(FleetError::InvalidEvent(format!(
            "{event_id} is not aligned to the declared time_step_hours"
        )));
    }
    Ok(())
}

fn validate_interval(
    event_id: &str,
    start: f64,
    duration: f64,
    horizon: f64,
) -> Result<(), FleetError> {
    if !start.is_finite()
        || !duration.is_finite()
        || start < 0.0
        || duration <= 0.0
        || start + duration > horizon + EPSILON
    {
        return Err(FleetError::InvalidEvent(format!(
            "{event_id} has an invalid interval outside the evaluation horizon"
        )));
    }
    Ok(())
}

fn evaluate_policy(
    case: &FleetContinuityCase,
    policy: &PolicyCase,
    platforms: &HashMap<&str, &FleetPlatform>,
    steps: usize,
) -> Result<FleetPolicyResult, FleetError> {
    let horizon = case.timebase.horizon_hours;
    let step = case.timebase.time_step_hours;
    let critical_threshold = case.service_contract.critical_demand_capacity_units;
    let full_threshold = case.service_contract.full_demand_capacity_units;

    let mut critical_service_steps = 0usize;
    let mut full_service_steps = 0usize;
    let mut haps_full_service_steps = 0usize;
    let mut useful_capacity_hours = 0.0;
    let mut minimum_haps_capacity = f64::INFINITY;
    let mut full_degradation_steps = 0usize;
    let mut critical_outage_steps = 0usize;
    let mut full_episode_count = 0u64;
    let mut current_full_episode = 0usize;
    let mut max_full_episode = 0usize;
    let mut current_critical_outage = 0usize;
    let mut max_critical_outage = 0usize;
    let mut planned_maintenance_hours = 0.0;
    let reserve_dispatch_delay_hours = policy
        .reserve_dispatch_events
        .iter()
        .map(|event| event.dispatch_delay_hours)
        .sum::<f64>();

    for event in &policy.platform_unavailability_events {
        if event.event_class == "planned_maintenance" {
            planned_maintenance_hours += event.duration_hours;
        }
    }

    for index in 0..steps {
        let time = index as f64 * step;
        let mut active_platform_ids: Vec<&str> = policy
            .operating_platform_ids
            .iter()
            .map(String::as_str)
            .collect();
        for dispatch in &policy.reserve_dispatch_events {
            if time + EPSILON >= dispatch.on_station_at_hour
                && time < dispatch.released_at_hour - EPSILON
            {
                active_platform_ids.push(dispatch.platform_id.as_str());
            }
        }

        let mut haps_capacity = 0.0;
        for platform_id in active_platform_ids {
            let unavailable = policy.platform_unavailability_events.iter().any(|event| {
                event.platform_id == platform_id
                    && time + EPSILON >= event.start_hour
                    && time < event.start_hour + event.duration_hours - EPSILON
            });
            if unavailable {
                continue;
            }
            let mut platform_factor = 1.0;
            for event in &case.common_cause_events {
                let in_interval = time + EPSILON >= event.start_hour
                    && time < event.start_hour + event.duration_hours - EPSILON;
                if !in_interval {
                    continue;
                }
                match event.scope.as_str() {
                    "all_haps_in_service_region" => {
                        platform_factor *= event.haps_capacity_factor;
                    }
                    "declared_platform_subset"
                        if event
                            .affected_platform_ids
                            .as_deref()
                            .unwrap_or_default()
                            .iter()
                            .any(|affected| affected == platform_id) =>
                    {
                        platform_factor *= event.haps_capacity_factor;
                    }
                    _ => {}
                }
            }
            haps_capacity +=
                platforms[platform_id].service_capacity_units * platform_factor;
        }

        let mut terrestrial_capacity =
            case.service_contract.normal_terrestrial_capacity_units;
        let mut satellite_capacity = case.service_contract.normal_satellite_capacity_units;
        for event in &case.common_cause_events {
            let in_interval = time + EPSILON >= event.start_hour
                && time < event.start_hour + event.duration_hours - EPSILON;
            if !in_interval {
                continue;
            }
            if let Some(value) = event.terrestrial_capacity_override_units {
                terrestrial_capacity = terrestrial_capacity.min(value);
            }
            if let Some(value) = event.satellite_capacity_override_units {
                satellite_capacity = satellite_capacity.min(value);
            }
        }

        let total_capacity = haps_capacity + terrestrial_capacity + satellite_capacity;
        minimum_haps_capacity = minimum_haps_capacity.min(haps_capacity);

        if haps_capacity + EPSILON >= full_threshold {
            haps_full_service_steps += 1;
        }

        if total_capacity + EPSILON >= critical_threshold {
            critical_service_steps += 1;
            if current_critical_outage > 0 {
                max_critical_outage = max_critical_outage.max(current_critical_outage);
                current_critical_outage = 0;
            }
        } else {
            critical_outage_steps += 1;
            current_critical_outage += 1;
            max_critical_outage = max_critical_outage.max(current_critical_outage);
        }

        if total_capacity + EPSILON >= full_threshold {
            full_service_steps += 1;
            if current_full_episode > 0 {
                full_episode_count += 1;
                max_full_episode = max_full_episode.max(current_full_episode);
                current_full_episode = 0;
            }
        } else {
            full_degradation_steps += 1;
            current_full_episode += 1;
            max_full_episode = max_full_episode.max(current_full_episode);
        }

        useful_capacity_hours +=
            (total_capacity.min(full_threshold) / full_threshold) * step;
    }

    if current_full_episode > 0 {
        full_episode_count += 1;
        max_full_episode = max_full_episode.max(current_full_episode);
    }
    max_critical_outage = max_critical_outage.max(current_critical_outage);

    Ok(FleetPolicyResult {
        policy_id: policy.policy_id.clone(),
        haps_only_full_service_availability_fraction: haps_full_service_steps as f64
            / steps as f64,
        critical_service_availability_fraction: critical_service_steps as f64 / steps as f64,
        full_service_availability_fraction: full_service_steps as f64 / steps as f64,
        useful_service_capacity_fraction: useful_capacity_hours / horizon,
        full_service_degraded_hours: full_degradation_steps as f64 * step,
        critical_service_outage_hours: critical_outage_steps as f64 * step,
        full_service_degradation_episode_count: full_episode_count,
        max_full_service_degradation_hours: max_full_episode as f64 * step,
        max_critical_service_outage_hours: max_critical_outage as f64 * step,
        planned_maintenance_hours,
        reserve_dispatch_delay_hours,
        minimum_on_station_haps_capacity_units: minimum_haps_capacity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!(
        "../../../docs/research/aviation/examples/synthetic_haps_fleet_continuity_30day_v1.json"
    );

    #[test]
    fn thirty_day_hourly_replay_matches_all_three_oracles() {
        let case = parse_case(FIXTURE).expect("fixture parses");
        let report = evaluate(&case).expect("fixture evaluates");
        verify_expected(&report, &case.expected_oracle).expect("oracle matches");

        assert_eq!(report.evaluated_hours, 720.0);
        assert_eq!(report.policy_results.len(), 3);
    }

    #[test]
    fn cold_spare_reduces_full_service_maintenance_gap_but_not_shared_weather_gap() {
        let report = evaluate_case(FIXTURE).expect("fixture evaluates");
        let no_spare = &report.policy_results[0];
        let cold_spare = &report.policy_results[1];

        assert_eq!(no_spare.full_service_degraded_hours, 72.0);
        assert_eq!(cold_spare.full_service_degraded_hours, 36.0);
        assert_eq!(no_spare.critical_service_outage_hours, 6.0);
        assert_eq!(cold_spare.critical_service_outage_hours, 6.0);
    }

    #[test]
    fn staggered_rotation_removes_maintenance_overlap_from_full_service_gaps() {
        let report = evaluate_case(FIXTURE).expect("fixture evaluates");
        let cold_spare = &report.policy_results[1];
        let staggered = &report.policy_results[2];

        assert_eq!(cold_spare.full_service_degraded_hours, 36.0);
        assert_eq!(staggered.full_service_degraded_hours, 24.0);
        assert_eq!(staggered.full_service_degradation_episode_count, 1);
        assert_eq!(staggered.max_full_service_degradation_hours, 24.0);
    }

    #[test]
    fn critical_and_full_service_are_distinct_during_weather_fallback() {
        let report = evaluate_case(FIXTURE).expect("fixture evaluates");
        let staggered = &report.policy_results[2];

        assert!(staggered.critical_service_availability_fraction
            > staggered.full_service_availability_fraction);
        assert!((staggered.critical_service_availability_fraction - 0.9916666666666667).abs()
            < EPSILON);
        assert!((staggered.full_service_availability_fraction - 0.9666666666666667).abs()
            < EPSILON);
    }

    #[test]
    fn unknown_platform_reference_fails_closed() {
        let mut case = parse_case(FIXTURE).expect("fixture parses");
        case.policy_cases[0]
            .operating_platform_ids
            .push("not-a-platform".to_owned());

        assert!(matches!(
            evaluate(&case),
            Err(FleetError::UnknownPlatform(_))
        ));
    }

    #[test]
    fn common_cause_gateway_event_must_be_nested_in_weather_event() {
        let mut case = parse_case(FIXTURE).expect("fixture parses");
        case.common_cause_events[1].parent_event_id =
            Some("missing-weather-parent".to_owned());

        assert!(matches!(
            evaluate(&case),
            Err(FleetError::InvalidEvent(_))
        ));
    }

    #[test]
    fn event_outside_horizon_fails_closed() {
        let mut case = parse_case(FIXTURE).expect("fixture parses");
        case.common_cause_events[0].start_hour = 719.0;
        case.common_cause_events[0].duration_hours = 2.0;

        assert!(matches!(
            evaluate(&case),
            Err(FleetError::InvalidEvent(_))
        ));
    }

    #[test]
    fn subset_event_does_not_remove_capacity_from_an_already_unavailable_platform() {
        let mut base = parse_case(FIXTURE).expect("fixture parses");
        base.policy_cases[0]
            .platform_unavailability_events
            .push(PlatformUnavailabilityEvent {
                event_id: "extra-maintenance-a".to_owned(),
                platform_id: "haps-a".to_owned(),
                event_class: "planned_maintenance".to_owned(),
                start_hour: 0.0,
                duration_hours: 24.0,
            });
        let without_subset_event = evaluate(&base).expect("base evaluates");

        let mut with_subset_event = base.clone();
        with_subset_event.common_cause_events.push(CommonCauseEvent {
            event_id: "a-only-weather".to_owned(),
            event_class: "regional_weather_restriction".to_owned(),
            start_hour: 0.0,
            duration_hours: 24.0,
            scope: "declared_platform_subset".to_owned(),
            affected_platform_ids: Some(vec!["haps-a".to_owned()]),
            haps_capacity_factor: 0.0,
            terrestrial_capacity_override_units: None,
            satellite_capacity_override_units: None,
            parent_event_id: None,
        });
        let with_subset_event = evaluate(&with_subset_event).expect("subset case evaluates");

        let baseline = &without_subset_event.policy_results[0];
        let affected = &with_subset_event.policy_results[0];
        assert_eq!(
            baseline.full_service_availability_fraction,
            affected.full_service_availability_fraction
        );
        assert_eq!(
            baseline.useful_service_capacity_fraction,
            affected.useful_service_capacity_fraction
        );
    }

    #[test]
    fn common_cause_event_order_does_not_change_service_replay() {
        let base = parse_case(FIXTURE).expect("fixture parses");
        let baseline = evaluate(&base).expect("baseline evaluates");
        let mut reordered = base;
        reordered.common_cause_events.reverse();
        let replay = evaluate(&reordered).expect("reordered events evaluate");

        assert_eq!(baseline.policy_results, replay.policy_results);
    }

    #[test]
    fn common_cause_parent_cycles_fail_closed() {
        let mut case = parse_case(FIXTURE).expect("fixture parses");
        case.common_cause_events[0].parent_event_id =
            Some("gateway-satellite-degradation-01".to_owned());

        assert!(matches!(
            evaluate(&case),
            Err(FleetError::InvalidEvent(_))
        ));
    }

    #[test]
    fn reserve_dispatch_windows_for_same_platform_cannot_overlap() {
        let mut case = parse_case(FIXTURE).expect("fixture parses");
        case.policy_cases[1].reserve_dispatch_events.push(ReserveDispatchEvent {
            platform_id: "haps-d".to_owned(),
            requested_at_hour: 150.0,
            on_station_at_hour: 162.0,
            released_at_hour: 200.0,
            dispatch_delay_hours: 12.0,
        });

        assert!(matches!(
            evaluate(&case),
            Err(FleetError::InvalidEvent(_))
        ));
    }

    #[test]
    fn standby_platform_must_have_an_explicit_dispatch_interval() {
        let mut case = parse_case(FIXTURE).expect("fixture parses");
        case.policy_cases[2].reserve_dispatch_events.clear();

        assert!(matches!(
            evaluate(&case),
            Err(FleetError::InvalidEvent(_))
        ));
    }

    #[test]
    fn event_times_must_align_to_the_declared_time_grid() {
        let mut case = parse_case(FIXTURE).expect("fixture parses");
        case.common_cause_events[0].start_hour = 360.5;

        assert!(matches!(
            evaluate(&case),
            Err(FleetError::InvalidEvent(_))
        ));
    }
}
