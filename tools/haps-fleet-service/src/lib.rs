//! Deterministic replay of HAPS fleet service availability event traces.
//!
//! This crate computes demand-qualified service availability from supplied
//! asset states and alternate-layer capacities. It is not a stochastic
//! reliability model, flight-dynamics model, weather model, radio solver,
//! maintenance predictor, battery model, or physical operations authority.

use std::collections::{HashMap, HashSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

const TIME_TOLERANCE_S: f64 = 1.0e-6;
const VALUE_TOLERANCE: f64 = 1.0e-9;
const SECONDS_PER_HOUR: f64 = 3_600.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FleetError {
    Json(String),
    InvalidCase(String),
    InvalidPolicy(String),
    InvalidInterval(String),
    ExpectedMismatch(String),
}

impl fmt::Display for FleetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(message)
            | Self::InvalidCase(message)
            | Self::InvalidPolicy(message)
            | Self::InvalidInterval(message)
            | Self::ExpectedMismatch(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for FleetError {}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleetServiceCase {
    pub schema_version: String,
    pub case_id: String,
    pub revision: u64,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub service_window: ServiceWindow,
    pub service_zones: Vec<ServiceZone>,
    pub platform_inventory: Vec<Platform>,
    pub policies: Vec<FleetPolicy>,
    pub expected_oracle: ExpectedOracle,
    #[serde(default)]
    pub source_evidence_refs: Vec<String>,
    #[serde(default)]
    pub assumptions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceWindow {
    pub duration_s: f64,
    pub interval_partition_required: bool,
    pub service_availability_threshold_fraction: f64,
    pub initial_target_availability_fraction: f64,
    pub stretch_target_availability_fraction: f64,
    pub target_window_s: f64,
    pub time_basis: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceZone {
    pub zone_id: String,
    pub required_capacity_units: f64,
    pub demand_weight: f64,
    pub criticality: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Platform {
    pub platform_id: String,
    pub lift_class: String,
    pub service_capacity_units: f64,
    pub evidence_class: String,
    pub configuration_ref: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleetPolicy {
    pub policy_id: String,
    pub policy_class: String,
    pub fleet_platform_ids: Vec<String>,
    pub required_active_capacity_units: f64,
    pub reserve_platform_ids: Vec<String>,
    pub intervals: Vec<ServiceInterval>,
    pub expected_metrics: ExpectedMetrics,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceInterval {
    pub start_s: f64,
    pub duration_s: f64,
    pub event_class: String,
    pub common_cause_event_id: Option<String>,
    pub platform_states: Vec<PlatformState>,
    pub alternate_service_by_zone: Vec<AlternateService>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformState {
    pub platform_id: String,
    pub state: String,
    pub zone_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AlternateService {
    pub zone_id: String,
    pub capacity_units: f64,
    pub source_layer: String,
    pub common_cause_independence: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedMetrics {
    pub service_availability_fraction: f64,
    pub time_weighted_demand_satisfaction_fraction: f64,
    pub available_hours: f64,
    pub below_threshold_hours: f64,
    pub maximum_contiguous_service_gap_hours: f64,
    pub planned_maintenance_platform_hours: f64,
    pub corrective_repair_platform_hours: f64,
    pub common_cause_grounded_platform_hours: f64,
    pub reserve_platform_hours: f64,
    pub handover_count: usize,
    pub alternate_layer_service_zone_hours: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedOracle {
    pub oracle_status: String,
    pub policy_rankings: Vec<PolicyRanking>,
    pub computed_from_interval_partition: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyRanking {
    pub policy_id: String,
    pub service_availability_fraction: f64,
    pub time_weighted_demand_satisfaction_fraction: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FleetMetrics {
    pub service_availability_fraction: f64,
    pub time_weighted_demand_satisfaction_fraction: f64,
    pub available_hours: f64,
    pub below_threshold_hours: f64,
    pub maximum_contiguous_service_gap_hours: f64,
    pub planned_maintenance_platform_hours: f64,
    pub corrective_repair_platform_hours: f64,
    pub common_cause_grounded_platform_hours: f64,
    pub reserve_platform_hours: f64,
    pub handover_count: usize,
    pub alternate_layer_service_zone_hours: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PolicyReport {
    pub policy_id: String,
    pub metrics: FleetMetrics,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FleetReport {
    pub case_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub duration_s: f64,
    pub policy_reports: Vec<PolicyReport>,
}

#[derive(Debug, Clone)]
struct IntervalResult {
    duration_s: f64,
    delivered_fraction: f64,
    active_by_zone: HashMap<String, String>,
    alternate_zone_capacity: HashMap<String, f64>,
    planned_maintenance_platforms: usize,
    corrective_repair_platforms: usize,
    common_cause_grounded_platforms: usize,
    reserve_platforms: usize,
}

pub fn parse_case_json(bytes: &[u8]) -> Result<FleetServiceCase, FleetError> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|error| FleetError::Json(error.to_string()))?;
    require_keys(
        &value,
        "fleet service case",
        &[
            "schema_version",
            "case_id",
            "revision",
            "evidence_class",
            "claim_ceiling",
            "service_window",
            "service_zones",
            "platform_inventory",
            "policies",
            "expected_oracle",
            "assumptions",
        ],
    )?;
    let case: FleetServiceCase =
        serde_json::from_value(value).map_err(|error| FleetError::Json(error.to_string()))?;
    validate_case(&case)?;
    Ok(case)
}

fn require_keys(value: &Value, context: &str, keys: &[&str]) -> Result<(), FleetError> {
    let object = value.as_object().ok_or_else(|| {
        FleetError::InvalidCase(format!("{context} must be a JSON object"))
    })?;
    for key in keys {
        if !object.contains_key(*key) {
            return Err(FleetError::InvalidCase(format!(
                "{context} is missing required field {key}"
            )));
        }
    }
    Ok(())
}

fn finite_positive(value: f64, label: &str) -> Result<(), FleetError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(FleetError::InvalidCase(format!(
            "{label} must be finite and greater than zero"
        )));
    }
    Ok(())
}

fn finite_nonnegative(value: f64, label: &str) -> Result<(), FleetError> {
    if !value.is_finite() || value < 0.0 {
        return Err(FleetError::InvalidCase(format!(
            "{label} must be finite and non-negative"
        )));
    }
    Ok(())
}

fn validate_case(case: &FleetServiceCase) -> Result<(), FleetError> {
    if case.schema_version != "haps-fleet-service-case/v1" {
        return Err(FleetError::InvalidCase("unsupported schema_version".to_owned()));
    }
    if case.evidence_class == "synthetic" && case.claim_ceiling != "synthetic_fleet_availability_oracle" {
        return Err(FleetError::InvalidCase(
            "synthetic evidence requires synthetic_fleet_availability_oracle claim ceiling".to_owned(),
        ));
    }
    finite_positive(case.service_window.duration_s, "service_window.duration_s")?;
    finite_positive(case.service_window.target_window_s, "service_window.target_window_s")?;
    if !case.service_window.interval_partition_required {
        return Err(FleetError::InvalidCase(
            "interval_partition_required must be true".to_owned(),
        ));
    }
    for (name, value) in [
        ("service_availability_threshold_fraction", case.service_window.service_availability_threshold_fraction),
        ("initial_target_availability_fraction", case.service_window.initial_target_availability_fraction),
        ("stretch_target_availability_fraction", case.service_window.stretch_target_availability_fraction),
    ] {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) || value == 0.0 {
            return Err(FleetError::InvalidCase(format!("{name} must be in (0, 1]")));
        }
    }
    if case.service_window.stretch_target_availability_fraction
        < case.service_window.initial_target_availability_fraction
    {
        return Err(FleetError::InvalidCase(
            "stretch target must not be below initial target".to_owned(),
        ));
    }
    if case.service_zones.is_empty() || case.platform_inventory.is_empty() || case.policies.is_empty() {
        return Err(FleetError::InvalidCase(
            "service zones, platform inventory and policies must be non-empty".to_owned(),
        ));
    }

    let mut zone_ids = HashSet::new();
    for zone in &case.service_zones {
        if zone.zone_id.trim().is_empty() || !zone_ids.insert(zone.zone_id.as_str()) {
            return Err(FleetError::InvalidCase("service zone IDs must be non-empty and unique".to_owned()));
        }
        finite_positive(zone.required_capacity_units, "zone.required_capacity_units")?;
        finite_positive(zone.demand_weight, "zone.demand_weight")?;
    }

    let mut platforms = HashMap::new();
    for platform in &case.platform_inventory {
        if platform.platform_id.trim().is_empty()
            || platforms.insert(platform.platform_id.as_str(), platform).is_some()
        {
            return Err(FleetError::InvalidCase("platform IDs must be non-empty and unique".to_owned()));
        }
        finite_positive(platform.service_capacity_units, "platform.service_capacity_units")?;
    }

    let mut policy_ids = HashSet::new();
    for policy in &case.policies {
        if policy.policy_id.trim().is_empty() || !policy_ids.insert(policy.policy_id.as_str()) {
            return Err(FleetError::InvalidCase("policy IDs must be non-empty and unique".to_owned()));
        }
        validate_policy(case, policy, &platforms, &zone_ids)?;
    }
    if case.expected_oracle.oracle_status == "independently_calculated" {
        if !case.expected_oracle.computed_from_interval_partition {
            return Err(FleetError::ExpectedMismatch(
                "oracle must be computed from the interval partition".to_owned(),
            ));
        }
        if case.expected_oracle.policy_rankings.len() != case.policies.len() {
            return Err(FleetError::ExpectedMismatch(
                "oracle policy ranking count does not match policy count".to_owned(),
            ));
        }
    }
    Ok(())
}

fn validate_policy<'a>(
    case: &FleetServiceCase,
    policy: &FleetPolicy,
    platforms: &HashMap<&'a str, &'a Platform>,
    zone_ids: &HashSet<&str>,
) -> Result<(), FleetError> {
    if policy.fleet_platform_ids.is_empty() {
        return Err(FleetError::InvalidPolicy(format!(
            "{} has no fleet platforms",
            policy.policy_id
        )));
    }
    finite_positive(
        policy.required_active_capacity_units,
        "required_active_capacity_units",
    )?;
    let fleet: HashSet<&str> = policy.fleet_platform_ids.iter().map(String::as_str).collect();
    if fleet.len() != policy.fleet_platform_ids.len() {
        return Err(FleetError::InvalidPolicy(format!(
            "{} has duplicate fleet platform IDs",
            policy.policy_id
        )));
    }
    for id in &fleet {
        if !platforms.contains_key(id) {
            return Err(FleetError::InvalidPolicy(format!(
                "{} references unknown platform {id}",
                policy.policy_id
            )));
        }
    }
    let reserves: HashSet<&str> = policy.reserve_platform_ids.iter().map(String::as_str).collect();
    if reserves.len() != policy.reserve_platform_ids.len() || !reserves.is_subset(&fleet) {
        return Err(FleetError::InvalidPolicy(format!(
            "{} reserve IDs must be unique members of its fleet",
            policy.policy_id
        )));
    }
    if policy.intervals.is_empty() {
        return Err(FleetError::InvalidPolicy(format!(
            "{} has no intervals",
            policy.policy_id
        )));
    }

    let mut elapsed = 0.0;
    for (index, interval) in policy.intervals.iter().enumerate() {
        if !interval.start_s.is_finite()
            || (interval.start_s - elapsed).abs() > TIME_TOLERANCE_S
        {
            return Err(FleetError::InvalidInterval(format!(
                "{} interval {index} is not contiguous: expected start {elapsed}, got {}",
                policy.policy_id, interval.start_s
            )));
        }
        finite_positive(interval.duration_s, "interval.duration_s")?;
        let end = interval.start_s + interval.duration_s;
        if end > case.service_window.duration_s + TIME_TOLERANCE_S {
            return Err(FleetError::InvalidInterval(format!(
                "{} interval {index} exceeds service window",
                policy.policy_id
            )));
        }
        if interval.platform_states.len() != fleet.len() {
            return Err(FleetError::InvalidInterval(format!(
                "{} interval {index} must contain one state for every fleet platform",
                policy.policy_id
            )));
        }

        let mut seen_states = HashSet::new();
        let mut active_zones = HashSet::new();
        let mut grounded_count = 0_usize;
        for state in &interval.platform_states {
            if !fleet.contains(state.platform_id.as_str())
                || !seen_states.insert(state.platform_id.as_str())
            {
                return Err(FleetError::InvalidInterval(format!(
                    "{} interval {index} has an unknown or duplicate platform state",
                    policy.policy_id
                )));
            }

            match state.state.as_str() {
                "active" => {
                    let zone_id = state.zone_id.as_ref().ok_or_else(|| {
                        FleetError::InvalidInterval(format!(
                            "{} interval {index} active platform needs a zone_id",
                            policy.policy_id
                        ))
                    })?;
                    if !zone_ids.contains(zone_id.as_str()) {
                        return Err(FleetError::InvalidInterval(format!(
                            "{} interval {index} references unknown zone {zone_id}",
                            policy.policy_id
                        )));
                    }
                    if !active_zones.insert(zone_id.as_str()) {
                        return Err(FleetError::InvalidInterval(format!(
                            "{} interval {index} assigns more than one platform to zone {zone_id}; this v1 contract expects one active platform per zone",
                            policy.policy_id
                        )));
                    }
                }
                "ready_reserve" => {
                    if state.zone_id.is_some() {
                        return Err(FleetError::InvalidInterval(
                            "ready reserve must not have a service zone".to_owned(),
                        ));
                    }
                    if !reserves.contains(state.platform_id.as_str()) {
                        return Err(FleetError::InvalidInterval(format!(
                            "{} interval {index} marks non-reserve platform {} as ready_reserve",
                            policy.policy_id, state.platform_id
                        )));
                    }
                }
                "scheduled_maintenance" | "corrective_maintenance" | "grounded_weather"
                | "transit" | "launch_recovery" | "retired" => {
                    if state.zone_id.is_some() {
                        return Err(FleetError::InvalidInterval(format!(
                            "non-active state {} cannot be assigned to a service zone",
                            state.state
                        )));
                    }
                    if state.state == "grounded_weather" {
                        grounded_count += 1;
                    }
                }
                other => {
                    return Err(FleetError::InvalidInterval(format!(
                        "unknown platform state {other}"
                    )));
                }
            }
        }
        if seen_states.len() != fleet.len() {
            return Err(FleetError::InvalidInterval(format!(
                "{} interval {index} omits a fleet platform",
                policy.policy_id
            )));
        }

        if interval.event_class == "common_cause_weather" {
            if interval
                .common_cause_event_id
                .as_deref()
                .unwrap_or("")
                .trim()
                .is_empty()
                || grounded_count != fleet.len()
                || !active_zones.is_empty()
            {
                return Err(FleetError::InvalidInterval(format!(
                    "{} interval {index} common-cause weather must ground every platform with a cause ID",
                    policy.policy_id
                )));
            }
        } else if interval.common_cause_event_id.is_some() {
            return Err(FleetError::InvalidInterval(format!(
                "{} interval {index} has a common-cause ID outside a common-cause event",
                policy.policy_id
            )));
        }

        let mut alternate_zones = HashSet::new();
        for alternate in &interval.alternate_service_by_zone {
            if !zone_ids.contains(alternate.zone_id.as_str()) {
                return Err(FleetError::InvalidInterval(format!(
                    "{} interval {index} alternate service references unknown zone {}",
                    policy.policy_id, alternate.zone_id
                )));
            }
            if !alternate_zones.insert(alternate.zone_id.as_str()) {
                return Err(FleetError::InvalidInterval(format!(
                    "{} interval {index} has duplicate alternate service zone",
                    policy.policy_id
                )));
            }
            finite_nonnegative(alternate.capacity_units, "alternate capacity_units")?;
            if interval.event_class == "common_cause_weather"
                && alternate.capacity_units > VALUE_TOLERANCE
                && alternate.common_cause_independence != "independent_for_this_event"
            {
                return Err(FleetError::InvalidInterval(format!(
                    "{} interval {index} counts alternate capacity without an independent-for-this-event declaration",
                    policy.policy_id
                )));
            }
            if alternate.common_cause_independence == "same_common_cause"
                && alternate.capacity_units > VALUE_TOLERANCE
            {
                return Err(FleetError::InvalidInterval(
                    "alternate layer with same_common_cause cannot provide positive capacity during this trace"
                        .to_owned(),
                ));
            }
        }
        elapsed = end;
    }

    if (elapsed - case.service_window.duration_s).abs() > TIME_TOLERANCE_S {
        return Err(FleetError::InvalidPolicy(format!(
            "{} intervals cover {elapsed} seconds, expected {}",
            policy.policy_id, case.service_window.duration_s
        )));
    }
    Ok(())
}

fn compare_expected(policy: &FleetPolicy, actual: &FleetMetrics) -> Result<(), FleetError> {
    let expected = &policy.expected_metrics;
    for (name, got, want) in [
        ("service_availability_fraction", actual.service_availability_fraction, expected.service_availability_fraction),
        ("time_weighted_demand_satisfaction_fraction", actual.time_weighted_demand_satisfaction_fraction, expected.time_weighted_demand_satisfaction_fraction),
        ("available_hours", actual.available_hours, expected.available_hours),
        ("below_threshold_hours", actual.below_threshold_hours, expected.below_threshold_hours),
        ("maximum_contiguous_service_gap_hours", actual.maximum_contiguous_service_gap_hours, expected.maximum_contiguous_service_gap_hours),
        ("planned_maintenance_platform_hours", actual.planned_maintenance_platform_hours, expected.planned_maintenance_platform_hours),
        ("corrective_repair_platform_hours", actual.corrective_repair_platform_hours, expected.corrective_repair_platform_hours),
        ("common_cause_grounded_platform_hours", actual.common_cause_grounded_platform_hours, expected.common_cause_grounded_platform_hours),
        ("reserve_platform_hours", actual.reserve_platform_hours, expected.reserve_platform_hours),
        ("alternate_layer_service_zone_hours", actual.alternate_layer_service_zone_hours, expected.alternate_layer_service_zone_hours),
    ] {
        if !got.is_finite() || (got - want).abs() > 1.0e-8 {
            return Err(FleetError::ExpectedMismatch(format!(
                "{} {name}: computed {got}, expected {want}",
                policy.policy_id
            )));
        }
    }
    if actual.handover_count != expected.handover_count {
        return Err(FleetError::ExpectedMismatch(format!(
            "{} handover_count: computed {}, expected {}",
            policy.policy_id, actual.handover_count, expected.handover_count
        )));
    }
    Ok(())
}

pub fn evaluate_case(case: &FleetServiceCase) -> Result<FleetReport, FleetError> {
    validate_case(case)?;
    let inventory: HashMap<&str, &Platform> = case.platform_inventory.iter()
        .map(|platform| (platform.platform_id.as_str(), platform))
        .collect();
    let mut policy_reports = Vec::new();
    for policy in &case.policies {
        let mut elapsed = 0.0;
        let mut available = 0.0;
        let mut satisfaction_integral = 0.0;
        let mut current_gap = 0.0;
        let mut max_gap = 0.0;
        let mut maintenance = 0.0;
        let mut repair = 0.0;
        let mut weather = 0.0;
        let mut reserve = 0.0;
        let mut handovers = 0_usize;
        let mut alt_hours = 0.0;
        let mut prior = HashMap::<String, String>::new();
        for interval in &policy.intervals {
            let states: HashMap<&str, &PlatformState> = interval.platform_states.iter()
                .map(|state| (state.platform_id.as_str(), state))
                .collect();
            let mut active_by_zone = HashMap::<String, String>::new();
            let mut active_capacity = HashMap::<String, f64>::new();
            for state in states.values() {
                if state.state == "active" {
                    let zone = state.zone_id.as_ref().expect("validated active zone");
                    active_by_zone.insert(zone.clone(), state.platform_id.clone());
                    let capacity = inventory.get(state.platform_id.as_str()).expect("validated inventory").service_capacity_units;
                    *active_capacity.entry(zone.clone()).or_insert(0.0) += capacity;
                }
            }
            let alternate: HashMap<&str, f64> = interval.alternate_service_by_zone.iter()
                .map(|item| (item.zone_id.as_str(), item.capacity_units))
                .collect();
            let total_weight: f64 = case.service_zones.iter().map(|zone| zone.demand_weight).sum();
            let mut delivered = 0.0;
            for zone in &case.service_zones {
                let haps = active_capacity.get(&zone.zone_id).copied().unwrap_or(0.0);
                let alt = alternate.get(zone.zone_id.as_str()).copied().unwrap_or(0.0);
                delivered += zone.demand_weight * ((haps + alt) / zone.required_capacity_units).clamp(0.0, 1.0);
                alt_hours += alt * interval.duration_s / SECONDS_PER_HOUR;
            }
            delivered /= total_weight;
            let interval_hours = interval.duration_s / SECONDS_PER_HOUR;
            if delivered + VALUE_TOLERANCE >= case.service_window.service_availability_threshold_fraction {
                available += interval_hours;
                current_gap = 0.0;
            } else {
                current_gap += interval_hours;
                max_gap = max_gap.max(current_gap);
            }
            satisfaction_integral += delivered * interval.duration_s;
            for state in interval.platform_states.iter() {
                match state.state.as_str() {
                    "scheduled_maintenance" => maintenance += interval_hours,
                    "corrective_maintenance" => repair += interval_hours,
                    "grounded_weather" => weather += interval_hours,
                    "ready_reserve" => reserve += interval_hours,
                    _ => {}
                }
            }
            for (zone, platform_id) in &active_by_zone {
                if let Some(old) = prior.get(zone) {
                    if old != platform_id {
                        handovers += 1;
                    }
                }
            }
            prior = active_by_zone;
            elapsed += interval.duration_s;
        }
        let total_hours = elapsed / SECONDS_PER_HOUR;
        let metrics = FleetMetrics {
            service_availability_fraction: available / total_hours,
            time_weighted_demand_satisfaction_fraction: satisfaction_integral / elapsed,
            available_hours: available,
            below_threshold_hours: total_hours - available,
            maximum_contiguous_service_gap_hours: max_gap,
            planned_maintenance_platform_hours: maintenance,
            corrective_repair_platform_hours: repair,
            common_cause_grounded_platform_hours: weather,
            reserve_platform_hours: reserve,
            handover_count: handovers,
            alternate_layer_service_zone_hours: alt_hours,
        };
        compare_expected(policy, &metrics)?;
        policy_reports.push(PolicyReport { policy_id: policy.policy_id.clone(), metrics });
    }

    for expected in &case.expected_oracle.policy_rankings {
        let report = policy_reports.iter().find(|report| report.policy_id == expected.policy_id)
            .ok_or_else(|| FleetError::ExpectedMismatch(format!("missing oracle policy {}", expected.policy_id)))?;
        if (report.metrics.service_availability_fraction - expected.service_availability_fraction).abs() > 1.0e-8
            || (report.metrics.time_weighted_demand_satisfaction_fraction - expected.time_weighted_demand_satisfaction_fraction).abs() > 1.0e-8
        {
            return Err(FleetError::ExpectedMismatch(format!(
                "policy ranking mismatch for {}", expected.policy_id
            )));
        }
    }

    Ok(FleetReport {
        case_id: case.case_id.clone(),
        evidence_class: case.evidence_class.clone(),
        claim_ceiling: case.claim_ceiling.clone(),
        duration_s: case.service_window.duration_s,
        policy_reports,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!(
        "../../../docs/research/aviation/examples/synthetic_haps_fleet_rotation_common_cause_v1.json"
    );

    fn fixture() -> FleetServiceCase {
        parse_case_json(FIXTURE.as_bytes()).expect("fixture parses")
    }

    #[test]
    fn event_trace_replays_all_three_policy_metrics() {
        let case = fixture();
        let report = evaluate_case(&case).expect("event trace evaluates");
        assert_eq!(report.policy_reports.len(), 3);
        let no_spare = &report.policy_reports[0].metrics;
        assert!((no_spare.service_availability_fraction - (50.0 / 72.0)).abs() < 1.0e-10);
        assert!((no_spare.time_weighted_demand_satisfaction_fraction - (58.0 / 72.0)).abs() < 1.0e-10);
        assert!((no_spare.maximum_contiguous_service_gap_hours - 10.0).abs() < 1.0e-10);
        let rotation = &report.policy_reports[1].metrics;
        assert!((rotation.service_availability_fraction - (66.0 / 72.0)).abs() < 1.0e-10);
        assert_eq!(rotation.handover_count, 4);
        let layered = &report.policy_reports[2].metrics;
        assert_eq!(layered.service_availability_fraction, 1.0);
        assert!((layered.time_weighted_demand_satisfaction_fraction - (71.7 / 72.0)).abs() < 1.0e-10);
        assert_eq!(layered.maximum_contiguous_service_gap_hours, 0.0);
    }

    #[test]
    fn non_contiguous_timeline_is_rejected() {
        let mut case = fixture();
        case.policies[0].intervals[1].start_s += 1.0;
        let error = evaluate_case(&case).expect_err("gap must fail closed");
        assert!(error.to_string().contains("not contiguous"));
    }

    #[test]
    fn weather_common_cause_must_ground_every_fleet_platform() {
        let mut case = fixture();
        case.policies[1].intervals[5].platform_states[0].state = "active".to_owned();
        case.policies[1].intervals[5].platform_states[0].zone_id = Some("zone-north".to_owned());
        let error = evaluate_case(&case).expect_err("partial grounding is not the common-cause case");
        assert!(error.to_string().contains("common-cause weather"));
    }

    #[test]
    fn incomplete_platform_state_vector_is_rejected() {
        let mut case = fixture();
        case.policies[2].intervals[0].platform_states.pop();
        let error = evaluate_case(&case).expect_err("every fleet asset must have a state");
        assert!(error.to_string().contains("one state for every fleet platform"));
    }

    #[test]
    fn correlated_fallback_cannot_be_counted_as_independent() {
        let mut case = fixture();
        case.policies[2].intervals[5].alternate_service_by_zone[0].common_cause_independence = "unknown".to_owned();
        let error = evaluate_case(&case).expect_err("unknown common-cause independence must fail closed");
        assert!(error.to_string().contains("without an independent"));
    }

    #[test]
    fn duplicate_platform_state_is_rejected() {
        let mut case = fixture();
        let duplicate = case.policies[0].intervals[0].platform_states[0].clone();
        case.policies[0].intervals[0].platform_states[1] = duplicate;
        let error = evaluate_case(&case).expect_err("duplicate platform state must fail");
        assert!(error.to_string().contains("unknown or duplicate platform state"));
    }
}
