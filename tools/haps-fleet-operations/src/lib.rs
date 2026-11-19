//! Deterministic event replay for HAPS fleet service continuity.
//!
//! This evaluator models a supplied time trace. It does not estimate failure
//! probabilities, fit repair distributions, or replace the separate static
//! Bernoulli / semi-Markov fleet-reliability workstream.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

use serde::{Deserialize, Serialize};

const EPSILON: f64 = 1.0e-9;
const RESOURCE_POOLS: [&str; 5] = [
    "maintenance_bay",
    "launch_pad",
    "recovery_slot",
    "reposition_slot",
    "handover_channel",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationsError(String);

impl OperationsError {
    fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for OperationsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for OperationsError {}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleetOperationsCase {
    pub schema_version: String,
    pub case_id: String,
    pub revision: u64,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub horizon_hours: u32,
    pub step_hours: u32,
    pub required_service_capacity_units: f64,
    pub degraded_margin_units: f64,
    pub nodes: Vec<CapacityNode>,
    pub fallback_sources: Vec<FallbackSource>,
    pub events: Vec<CapacityEvent>,
    pub demand_periods: Vec<DemandPeriod>,
    pub resource_limits: Vec<ResourceLimit>,
    pub handover_attempts: Vec<HandoverAttempt>,
    pub expected_oracle: ExpectedOperationsOracle,
    #[serde(default)]
    pub source_evidence_refs: Vec<String>,
    pub assumptions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityNode {
    pub node_id: String,
    pub platform_class: String,
    pub service_region_ref: String,
    pub hazard_domain: String,
    pub base_capacity_units: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FallbackSource {
    pub source_id: String,
    pub source_class: String,
    pub service_region_ref: String,
    pub base_capacity_units: f64,
    pub independence_domain: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityEvent {
    pub event_id: String,
    pub event_type: String,
    pub event_class: String,
    pub start_hour: u32,
    pub end_hour: u32,
    pub capacity_factor: f64,
    pub affected_node_ids: Vec<String>,
    pub affected_fallback_ids: Vec<String>,
    pub shared_cause_id: Option<String>,
    pub resource_pool: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DemandPeriod {
    pub start_hour: u32,
    pub end_hour: u32,
    pub demand_weight: f64,
    pub demand_class: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceLimit {
    pub resource_pool: String,
    pub maximum_concurrent_operations: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HandoverAttempt {
    pub handover_id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub offered_sessions: u64,
    pub prepared_sessions: u64,
    pub accepted_sessions: u64,
    pub completed_sessions: u64,
    pub timed_out_sessions: u64,
    pub p95_interruption_s: f64,
    pub maximum_interruption_s: f64,
    pub state_transfer_failures: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedOperationsOracle {
    pub oracle_status: String,
    pub total_hours: u32,
    pub service_adequate_hours: f64,
    pub strict_service_availability_fraction: f64,
    pub demand_weighted_available_hours: f64,
    pub demand_weighted_total_hours: f64,
    pub demand_weighted_service_availability_fraction: f64,
    pub demand_weighted_capacity_satisfaction_units_hours: f64,
    pub demand_weighted_capacity_satisfaction_fraction: f64,
    pub outage_episode_count: u32,
    pub total_outage_hours: f64,
    pub maximum_contiguous_outage_hours: f64,
    pub degraded_service_hours: f64,
    pub minimum_service_capacity_units: f64,
    pub capacity_deficit_units_hours: f64,
    pub capacity_surplus_units_hours: f64,
    pub maximum_resource_concurrency: BTreeMap<String, u32>,
    pub resource_limit_violations: Vec<String>,
    pub handover_offered_sessions: u64,
    pub handover_prepared_sessions: u64,
    pub handover_accepted_sessions: u64,
    pub handover_completed_sessions: u64,
    pub handover_success_fraction: f64,
    pub handover_p95_interruption_s_max: f64,
    pub handover_maximum_interruption_s_max: f64,
    pub state_transfer_failures: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FleetOperationsReport {
    pub case_id: String,
    pub total_hours: u32,
    pub service_adequate_hours: f64,
    pub strict_service_availability_fraction: f64,
    pub demand_weighted_available_hours: f64,
    pub demand_weighted_total_hours: f64,
    pub demand_weighted_service_availability_fraction: f64,
    pub demand_weighted_capacity_satisfaction_units_hours: f64,
    pub demand_weighted_capacity_satisfaction_fraction: f64,
    pub outage_episode_count: u32,
    pub total_outage_hours: f64,
    pub maximum_contiguous_outage_hours: f64,
    pub degraded_service_hours: f64,
    pub minimum_service_capacity_units: f64,
    pub capacity_deficit_units_hours: f64,
    pub capacity_surplus_units_hours: f64,
    pub maximum_resource_concurrency: BTreeMap<String, u32>,
    pub resource_limit_violations: Vec<String>,
    pub handover_offered_sessions: u64,
    pub handover_prepared_sessions: u64,
    pub handover_accepted_sessions: u64,
    pub handover_completed_sessions: u64,
    pub handover_success_fraction: f64,
    pub handover_p95_interruption_s_max: f64,
    pub handover_maximum_interruption_s_max: f64,
    pub state_transfer_failures: u64,
}

pub fn parse_case_json(bytes: &[u8]) -> Result<FleetOperationsCase, OperationsError> {
    let case: FleetOperationsCase = serde_json::from_slice(bytes)
        .map_err(|error| OperationsError::new(format!("invalid fleet operations JSON: {error}")))?;
    validate_case(&case)?;
    Ok(case)
}

pub fn validate_case(case: &FleetOperationsCase) -> Result<(), OperationsError> {
    if case.schema_version != "haps-fleet-operations-case/v1" {
        return Err(OperationsError::new(
            "unsupported HAPS fleet operations schema_version",
        ));
    }
    if case.case_id.trim().is_empty() || case.revision == 0 {
        return Err(OperationsError::new(
            "case_id must be non-empty and revision positive",
        ));
    }
    if case.evidence_class != "synthetic"
        || case.claim_ceiling != "synthetic_deterministic_event_replay"
    {
        return Err(OperationsError::new(
            "this evaluator accepts synthetic event traces only",
        ));
    }
    if case.horizon_hours == 0 || case.step_hours != 1 {
        return Err(OperationsError::new(
            "horizon_hours must be positive and step_hours must equal 1",
        ));
    }
    if !case.required_service_capacity_units.is_finite()
        || case.required_service_capacity_units <= 0.0
        || !case.degraded_margin_units.is_finite()
        || case.degraded_margin_units < 0.0
    {
        return Err(OperationsError::new(
            "service threshold and degraded margin must be finite and valid",
        ));
    }
    if case.nodes.is_empty() {
        return Err(OperationsError::new("at least one HAPS node is required"));
    }

    let mut node_ids = HashSet::new();
    for node in &case.nodes {
        if node.node_id.trim().is_empty() || !node_ids.insert(node.node_id.as_str()) {
            return Err(OperationsError::new(format!(
                "empty or duplicate node_id: {}",
                node.node_id
            )));
        }
        if node.service_region_ref.trim().is_empty()
            || node.hazard_domain.trim().is_empty()
            || !node.base_capacity_units.is_finite()
            || node.base_capacity_units < 0.0
        {
            return Err(OperationsError::new(format!(
                "invalid node capacity or region: {}",
                node.node_id
            )));
        }
        if !matches!(
            node.platform_class.as_str(),
            "solar_fixed_wing_haps" | "buoyant_airship" | "buoyant_free_balloon"
        ) {
            return Err(OperationsError::new(format!(
                "unsupported platform_class on {}",
                node.node_id
            )));
        }
    }

    let fleet_region = case.nodes[0].service_region_ref.as_str();
    for node in &case.nodes {
        if node.service_region_ref != fleet_region {
            return Err(OperationsError::new(
                "this case aggregates one service region; split multi-region cases before replay",
            ));
        }
    }

    let mut fallback_ids = HashSet::new();
    for fallback in &case.fallback_sources {
        if fallback.source_id.trim().is_empty() || !fallback_ids.insert(fallback.source_id.as_str())
        {
            return Err(OperationsError::new(format!(
                "empty or duplicate fallback source_id: {}",
                fallback.source_id
            )));
        }
        if fallback.service_region_ref.trim().is_empty()
            || fallback.service_region_ref != fleet_region
            || fallback.independence_domain.trim().is_empty()
            || !fallback.base_capacity_units.is_finite()
            || fallback.base_capacity_units < 0.0
        {
            return Err(OperationsError::new(format!(
                "invalid fallback source: {}",
                fallback.source_id
            )));
        }
    }

    let mut event_ids = HashSet::new();
    for event in &case.events {
        if event.event_id.trim().is_empty() || !event_ids.insert(event.event_id.as_str()) {
            return Err(OperationsError::new(format!(
                "empty or duplicate event_id: {}",
                event.event_id
            )));
        }
        let expected_class = match event.event_type.as_str() {
            "launch_transit" | "reposition" | "planned_maintenance" | "recovery_transit" => {
                "planned_operation"
            }
            "unplanned_fault" => "independent_asset",
            "shared_weather_derate" | "shared_weather_outage" => "shared_weather",
            "shared_infrastructure_outage" => "shared_infrastructure",
            "group_handover" => "service_handover",
            _ => {
                return Err(OperationsError::new(format!(
                    "event {} has unsupported event_type",
                    event.event_id
                )));
            }
        };
        if event.event_class != expected_class {
            return Err(OperationsError::new(format!(
                "event {} event_class does not match event_type",
                event.event_id
            )));
        }
        if event.start_hour >= event.end_hour || event.end_hour > case.horizon_hours {
            return Err(OperationsError::new(format!(
                "event {} has invalid half-open interval",
                event.event_id
            )));
        }
        if !event.capacity_factor.is_finite() || !(0.0..=1.0).contains(&event.capacity_factor) {
            return Err(OperationsError::new(format!(
                "event {} capacity_factor must be in [0,1]",
                event.event_id
            )));
        }
        if event.affected_node_ids.is_empty() && event.affected_fallback_ids.is_empty() {
            return Err(OperationsError::new(format!(
                "event {} affects no fleet or fallback capacity",
                event.event_id
            )));
        }
        if event.resource_pool != "none" && !RESOURCE_POOLS.contains(&event.resource_pool.as_str())
        {
            return Err(OperationsError::new(format!(
                "event {} uses unknown resource_pool",
                event.event_id
            )));
        }
        if event.event_class == "shared_weather" || event.event_class == "shared_infrastructure" {
            if event
                .shared_cause_id
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
            {
                return Err(OperationsError::new(format!(
                    "shared event {} lacks shared_cause_id",
                    event.event_id
                )));
            }
        }
        for id in &event.affected_node_ids {
            if !node_ids.contains(id.as_str()) {
                return Err(OperationsError::new(format!(
                    "event {} references unknown HAPS node {id}",
                    event.event_id
                )));
            }
        }
        for id in &event.affected_fallback_ids {
            if !fallback_ids.contains(id.as_str()) {
                return Err(OperationsError::new(format!(
                    "event {} references unknown fallback source {id}",
                    event.event_id
                )));
            }
        }
    }

    let mut sorted_demand = case.demand_periods.iter().collect::<Vec<_>>();
    sorted_demand.sort_by_key(|period| period.start_hour);
    if sorted_demand.is_empty() {
        return Err(OperationsError::new(
            "at least one demand period is required",
        ));
    }
    let mut expected_start = 0;
    for period in sorted_demand {
        if period.start_hour != expected_start
            || period.start_hour >= period.end_hour
            || period.end_hour > case.horizon_hours
            || !period.demand_weight.is_finite()
            || period.demand_weight <= 0.0
        {
            return Err(OperationsError::new(
                "demand periods must form a positive-weight, gap-free, non-overlapping partition",
            ));
        }
        expected_start = period.end_hour;
    }
    if expected_start != case.horizon_hours {
        return Err(OperationsError::new(
            "demand periods do not cover the complete horizon",
        ));
    }

    let mut resource_pools = HashSet::new();
    for limit in &case.resource_limits {
        if !RESOURCE_POOLS.contains(&limit.resource_pool.as_str())
            || limit.maximum_concurrent_operations == 0
            || !resource_pools.insert(limit.resource_pool.as_str())
        {
            return Err(OperationsError::new(format!(
                "invalid or duplicate resource limit: {}",
                limit.resource_pool
            )));
        }
    }
    for pool in RESOURCE_POOLS {
        if !resource_pools.contains(pool) {
            return Err(OperationsError::new(format!(
                "missing resource limit for {pool}"
            )));
        }
    }

    let mut handover_ids = HashSet::new();
    for handover in &case.handover_attempts {
        if handover.handover_id.trim().is_empty()
            || !handover_ids.insert(handover.handover_id.as_str())
        {
            return Err(OperationsError::new(format!(
                "empty or duplicate handover_id: {}",
                handover.handover_id
            )));
        }
        if !node_ids.contains(handover.source_node_id.as_str())
            || !node_ids.contains(handover.target_node_id.as_str())
            || handover.source_node_id == handover.target_node_id
        {
            return Err(OperationsError::new(format!(
                "handover {} has invalid source/target",
                handover.handover_id
            )));
        }
        if handover.completed_sessions > handover.accepted_sessions
            || handover.accepted_sessions > handover.prepared_sessions
            || handover.prepared_sessions > handover.offered_sessions
            || handover.timed_out_sessions
                > handover
                    .accepted_sessions
                    .saturating_sub(handover.completed_sessions)
        {
            return Err(OperationsError::new(format!(
                "handover {} has non-monotonic session counts",
                handover.handover_id
            )));
        }
        if !handover.p95_interruption_s.is_finite()
            || !handover.maximum_interruption_s.is_finite()
            || handover.p95_interruption_s < 0.0
            || handover.maximum_interruption_s < handover.p95_interruption_s
        {
            return Err(OperationsError::new(format!(
                "handover {} has invalid interruption measurements",
                handover.handover_id
            )));
        }
    }
    Ok(())
}

pub fn evaluate_case(case: &FleetOperationsCase) -> Result<FleetOperationsReport, OperationsError> {
    validate_case(case)?;
    let resource_limit_lookup = case
        .resource_limits
        .iter()
        .map(|limit| {
            (
                limit.resource_pool.as_str(),
                limit.maximum_concurrent_operations,
            )
        })
        .collect::<HashMap<_, _>>();

    let mut report = FleetOperationsReport {
        case_id: case.case_id.clone(),
        total_hours: case.horizon_hours,
        service_adequate_hours: 0.0,
        strict_service_availability_fraction: 0.0,
        demand_weighted_available_hours: 0.0,
        demand_weighted_total_hours: 0.0,
        demand_weighted_service_availability_fraction: 0.0,
        demand_weighted_capacity_satisfaction_units_hours: 0.0,
        demand_weighted_capacity_satisfaction_fraction: 0.0,
        outage_episode_count: 0,
        total_outage_hours: 0.0,
        maximum_contiguous_outage_hours: 0.0,
        degraded_service_hours: 0.0,
        minimum_service_capacity_units: f64::INFINITY,
        capacity_deficit_units_hours: 0.0,
        capacity_surplus_units_hours: 0.0,
        maximum_resource_concurrency: RESOURCE_POOLS
            .iter()
            .map(|pool| ((*pool).to_owned(), 0))
            .collect(),
        resource_limit_violations: Vec::new(),
        handover_offered_sessions: 0,
        handover_prepared_sessions: 0,
        handover_accepted_sessions: 0,
        handover_completed_sessions: 0,
        handover_success_fraction: 1.0,
        handover_p95_interruption_s_max: 0.0,
        handover_maximum_interruption_s_max: 0.0,
        state_transfer_failures: 0,
    };

    let mut current_outage_hours = 0.0;
    for hour in 0..case.horizon_hours {
        let mut service_capacity = 0.0;
        for node in &case.nodes {
            let mut factor = 1.0;
            for event in &case.events {
                if event.start_hour <= hour
                    && hour < event.end_hour
                    && event.affected_node_ids.iter().any(|id| id == &node.node_id)
                {
                    factor *= event.capacity_factor;
                }
            }
            service_capacity += node.base_capacity_units * factor;
        }
        for fallback in &case.fallback_sources {
            let mut factor = 1.0;
            for event in &case.events {
                if event.start_hour <= hour
                    && hour < event.end_hour
                    && event
                        .affected_fallback_ids
                        .iter()
                        .any(|id| id == &fallback.source_id)
                {
                    factor *= event.capacity_factor;
                }
            }
            service_capacity += fallback.base_capacity_units * factor;
        }
        report.minimum_service_capacity_units =
            report.minimum_service_capacity_units.min(service_capacity);
        report.capacity_deficit_units_hours +=
            (case.required_service_capacity_units - service_capacity).max(0.0);
        report.capacity_surplus_units_hours +=
            (service_capacity - case.required_service_capacity_units).max(0.0);

        let demand_period = case
            .demand_periods
            .iter()
            .find(|period| period.start_hour <= hour && hour < period.end_hour)
            .ok_or_else(|| OperationsError::new(format!("no demand period for hour {hour}")))?;
        report.demand_weighted_total_hours += demand_period.demand_weight;
        let capacity_fraction = service_capacity / case.required_service_capacity_units;
        if capacity_fraction > 1.0 {
            capacity_fraction = 1.0;
        }
        report.demand_weighted_capacity_satisfaction_units_hours +=
            demand_period.demand_weight * capacity_fraction;

        if service_capacity + EPSILON >= case.required_service_capacity_units {
            report.service_adequate_hours += 1.0;
            report.demand_weighted_available_hours += demand_period.demand_weight;
            if service_capacity < case.required_service_capacity_units + case.degraded_margin_units
            {
                report.degraded_service_hours += 1.0;
            }
            if current_outage_hours > 0.0 {
                report.outage_episode_count += 1;
                report.maximum_contiguous_outage_hours = report
                    .maximum_contiguous_outage_hours
                    .max(current_outage_hours);
                current_outage_hours = 0.0;
            }
        } else {
            report.total_outage_hours += 1.0;
            current_outage_hours += 1.0;
        }

        for pool in RESOURCE_POOLS {
            let count = case
                .events
                .iter()
                .filter(|event| {
                    event.resource_pool == pool && event.start_hour <= hour && hour < event.end_hour
                })
                .count() as u32;
            let max_entry = report
                .maximum_resource_concurrency
                .get_mut(pool)
                .expect("resource pool initialized");
            *max_entry = (*max_entry).max(count);
            if count > resource_limit_lookup[pool] {
                let violation = format!("{pool}@{hour}={count}>{}", resource_limit_lookup[pool]);
                if !report.resource_limit_violations.contains(&violation) {
                    report.resource_limit_violations.push(violation);
                }
            }
        }
    }
    if current_outage_hours > 0.0 {
        report.outage_episode_count += 1;
        report.maximum_contiguous_outage_hours = report
            .maximum_contiguous_outage_hours
            .max(current_outage_hours);
    }
    report.strict_service_availability_fraction =
        report.service_adequate_hours / f64::from(case.horizon_hours);
    report.demand_weighted_service_availability_fraction =
        report.demand_weighted_available_hours / report.demand_weighted_total_hours;
    report.demand_weighted_capacity_satisfaction_fraction = report
        .demand_weighted_capacity_satisfaction_units_hours
        / report.demand_weighted_total_hours;

    for handover in &case.handover_attempts {
        report.handover_offered_sessions += handover.offered_sessions;
        report.handover_prepared_sessions += handover.prepared_sessions;
        report.handover_accepted_sessions += handover.accepted_sessions;
        report.handover_completed_sessions += handover.completed_sessions;
        report.handover_p95_interruption_s_max = report
            .handover_p95_interruption_s_max
            .max(handover.p95_interruption_s);
        report.handover_maximum_interruption_s_max = report
            .handover_maximum_interruption_s_max
            .max(handover.maximum_interruption_s);
        report.state_transfer_failures += handover.state_transfer_failures;
    }
    if report.handover_offered_sessions > 0 {
        report.handover_success_fraction =
            report.handover_completed_sessions as f64 / report.handover_offered_sessions as f64;
    }

    if !report.minimum_service_capacity_units.is_finite() {
        return Err(OperationsError::new(
            "evaluation produced no service capacity",
        ));
    }
    Ok(report)
}

pub fn verify_expected_oracle(
    report: &FleetOperationsReport,
    expected: &ExpectedOperationsOracle,
) -> Result<(), OperationsError> {
    if expected.oracle_status != "independently_calculated" {
        return Err(OperationsError::new(
            "expected_oracle must be independently_calculated",
        ));
    }
    if report.total_hours != expected.total_hours {
        return Err(OperationsError::new("total_hours oracle mismatch"));
    }
    macro_rules! close {
        ($field:ident) => {
            if (report.$field - expected.$field).abs() > 1.0e-8 {
                return Err(OperationsError::new(format!(
                    "{} oracle mismatch",
                    stringify!($field)
                )));
            }
        };
    }
    close!(service_adequate_hours);
    close!(strict_service_availability_fraction);
    close!(demand_weighted_available_hours);
    close!(demand_weighted_total_hours);
    close!(demand_weighted_service_availability_fraction);
    close!(demand_weighted_capacity_satisfaction_units_hours);
    close!(demand_weighted_capacity_satisfaction_fraction);
    close!(total_outage_hours);
    close!(maximum_contiguous_outage_hours);
    close!(degraded_service_hours);
    close!(minimum_service_capacity_units);
    close!(capacity_deficit_units_hours);
    close!(capacity_surplus_units_hours);
    close!(handover_success_fraction);
    close!(handover_p95_interruption_s_max);
    close!(handover_maximum_interruption_s_max);
    if report.outage_episode_count != expected.outage_episode_count
        || report.maximum_resource_concurrency != expected.maximum_resource_concurrency
        || report.resource_limit_violations != expected.resource_limit_violations
        || report.handover_offered_sessions != expected.handover_offered_sessions
        || report.handover_prepared_sessions != expected.handover_prepared_sessions
        || report.handover_accepted_sessions != expected.handover_accepted_sessions
        || report.handover_completed_sessions != expected.handover_completed_sessions
        || report.state_transfer_failures != expected.state_transfer_failures
    {
        return Err(OperationsError::new("discrete operations oracle mismatch"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!(
        "../../../docs/research/aviation/examples/synthetic_haps_fleet_operations_case_v1.json"
    );

    fn case() -> FleetOperationsCase {
        parse_case_json(FIXTURE.as_bytes()).unwrap()
    }

    #[test]
    fn fixture_matches_independent_720_hour_oracle() {
        let case = case();
        let report = evaluate_case(&case).unwrap();
        verify_expected_oracle(&report, &case.expected_oracle).unwrap();
        assert_eq!(report.service_adequate_hours, 708.0);
        assert!((report.strict_service_availability_fraction - 0.9833333333333333).abs() < 1e-12);
        assert!(
            (report.demand_weighted_service_availability_fraction - 0.9365079365079365).abs()
                < 1e-12
        );
        assert!(
            (report.demand_weighted_capacity_satisfaction_fraction - 0.9841269841269841).abs()
                < 1e-12
        );
        assert_eq!(report.total_outage_hours, 12.0);
        assert_eq!(report.maximum_contiguous_outage_hours, 12.0);
    }

    #[test]
    fn weather_derate_alone_is_absorbed_by_diverse_fallback() {
        let mut case = case();
        case.events
            .retain(|event| event.event_id != "shared-ground-core");
        let report = evaluate_case(&case).unwrap();
        assert_eq!(report.total_outage_hours, 0.0);
        assert_eq!(report.strict_service_availability_fraction, 1.0);
        assert!(report.degraded_service_hours >= 10.0);
    }

    #[test]
    fn removing_fallback_independence_creates_the_shared_weather_outage() {
        let mut case = case();
        for source in &mut case.fallback_sources {
            source.base_capacity_units = 0.0;
        }
        let report = evaluate_case(&case).unwrap();
        assert_eq!(report.total_outage_hours, 22.0);
        assert!((report.capacity_deficit_units_hours - 11.0).abs() < 1.0e-8);
    }

    #[test]
    fn overlapping_capacity_factors_multiply() {
        let mut case = case();
        let mut extra = (*case
            .events
            .iter()
            .find(|event| event.event_id == "weather-all-and-ground")
            .unwrap())
        .clone();
        extra.event_id = "second-weather-derate-a".to_owned();
        extra.affected_node_ids = vec!["haps-a".to_owned()];
        extra.affected_fallback_ids.clear();
        extra.capacity_factor = 0.5;
        extra.shared_cause_id = Some("weather-overlap-03".to_owned());
        case.events.push(extra);
        let report = evaluate_case(&case).unwrap();
        assert_eq!(report.minimum_service_capacity_units, 1.25);
        assert_eq!(report.capacity_deficit_units_hours, 14.0);
    }

    #[test]
    fn maintenance_resource_conflicts_are_explicit() {
        let mut case = case();
        case.events.push(CapacityEvent {
            event_id: "overlapping-maintenance-c".to_owned(),
            event_type: "planned_maintenance".to_owned(),
            event_class: "planned_operation".to_owned(),
            start_hour: 100,
            end_hour: 110,
            capacity_factor: 0.0,
            affected_node_ids: vec!["haps-c".to_owned()],
            affected_fallback_ids: Vec::new(),
            shared_cause_id: None,
            resource_pool: "maintenance_bay".to_owned(),
        });
        let report = evaluate_case(&case).unwrap();
        assert_eq!(report.maximum_resource_concurrency["maintenance_bay"], 2);
        assert_eq!(report.resource_limit_violations.len(), 10);
    }

    #[test]
    fn demand_period_gaps_fail_closed() {
        let mut case = case();
        case.demand_periods[1].start_hour = 433;
        assert!(validate_case(&case).is_err());
    }

    #[test]
    fn invalid_handover_counts_fail_closed() {
        let mut case = case();
        case.handover_attempts[0].completed_sessions = 9_901;
        assert!(validate_case(&case).is_err());
    }

    #[test]
    fn event_outside_horizon_fails_closed() {
        let mut case = case();
        case.events[0].end_hour = case.horizon_hours + 1;
        assert!(validate_case(&case).is_err());
    }

    const DIVERSE_FIXTURE: &str = include_str!(
        "../../../docs/research/aviation/examples/synthetic_haps_fleet_weather_diverse_reserve_case_v1.json"
    );

    #[test]
    fn weather_diverse_reserve_preserves_service_during_shared_haps_and_ground_outage() {
        let case = parse_case_json(DIVERSE_FIXTURE.as_bytes()).unwrap();
        let report = evaluate_case(&case).unwrap();
        verify_expected_oracle(&report, &case.expected_oracle).unwrap();
        assert_eq!(report.service_adequate_hours, 720.0);
        assert_eq!(report.strict_service_availability_fraction, 1.0);
        assert_eq!(report.demand_weighted_service_availability_fraction, 1.0);
        assert_eq!(report.demand_weighted_capacity_satisfaction_fraction, 1.0);
        assert_eq!(report.degraded_service_hours, 12.0);
        assert!((report.minimum_service_capacity_units - 2.2).abs() < 1.0e-8);
    }

    #[test]
    fn reserve_sharing_the_weather_event_restores_the_common_cause_outage() {
        let mut case = parse_case_json(DIVERSE_FIXTURE.as_bytes()).unwrap();
        let event = case
            .events
            .iter_mut()
            .find(|event| event.event_id == "weather-main-domain-ground-outage")
            .unwrap();
        event.affected_node_ids.push("haps-d".to_owned());

        let report = evaluate_case(&case).unwrap();
        assert_eq!(report.total_outage_hours, 12.0);
        assert_eq!(report.maximum_contiguous_outage_hours, 12.0);
        assert!((report.minimum_service_capacity_units - 1.6).abs() < 1.0e-8);
        assert!((report.capacity_deficit_units_hours - 4.8).abs() < 1.0e-8);
    }

    #[test]
    fn shared_cause_must_be_named() {
        let mut case = case();
        case.events[8].shared_cause_id = None;
        assert!(validate_case(&case).is_err());
    }

    #[test]
    fn concurrent_planned_maintenance_is_not_hidden_by_service_adequacy() {
        let mut case = case();
        case.events[4].event_id = "fault-a-test".to_owned();
        case.events[4].start_hour = 100;
        case.events[4].end_hour = 110;
        case.events[4].event_type = "unplanned_fault".to_owned();
        case.events[4].event_class = "independent_asset".to_owned();
        case.events[4].resource_pool = "maintenance_bay".to_owned();
        let report = evaluate_case(&case).unwrap();
        assert!(!report.resource_limit_violations.is_empty());
    }
}
