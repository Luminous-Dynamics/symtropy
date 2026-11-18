//! Deterministic event-schedule replay for HAPS fleet service availability.
//!
//! This module complements, but does not replace, the static Bernoulli
//! common-cause enumerator in lib.rs. It evaluates declared time segments and
//! event windows. It is not a stochastic process, semi-Markov model, weather
//! predictor, flight model, or empirical service-availability estimator.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::ReliabilityError;

const TIME_TOLERANCE_H: f64 = 1.0e-9;
const CAPACITY_TOLERANCE: f64 = 1.0e-9;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FleetServiceAvailabilityCase {
    pub schema_version: String,
    pub case_id: String,
    pub revision: u64,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub evaluation_window_h: f64,
    pub service_contract: ServiceContract,
    pub fleet_policy: ServiceFleetPolicy,
    pub platforms: Vec<ServicePlatform>,
    pub common_cause_events: Vec<ServiceCommonCauseEvent>,
    pub launch_recovery_events: Vec<LaunchRecoveryEvent>,
    pub logistics_limits: ServiceLogisticsLimits,
    pub reserve_state_power_w: ReserveStatePower,
    pub expected_oracle: serde_json::Value,
    #[serde(default)]
    pub source_evidence_refs: Vec<String>,
    pub assumptions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceContract {
    pub required_service_capacity_units: f64,
    pub service_interval_h: f64,
    pub strict_availability_rule: String,
    pub capacity_adequacy_rule: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceFleetPolicy {
    pub policy_id: String,
    pub primary_platform_ids: Vec<String>,
    pub reserve_platform_ids: Vec<String>,
    pub reserve_activation_lead_time_h: f64,
    pub maintenance_rotation_policy: String,
    pub reserve_ready_state: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServicePlatform {
    pub platform_id: String,
    pub platform_family: String,
    pub role: String,
    pub service_capacity_units: f64,
    pub segments: Vec<PlatformStateSegment>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlatformStateSegment {
    pub start_h: f64,
    pub end_h: f64,
    pub state: String,
    pub capacity_fraction: f64,
    pub cause: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceCommonCauseEvent {
    pub event_id: String,
    pub start_h: f64,
    pub end_h: f64,
    pub event_class: String,
    pub affected_platform_ids: Vec<String>,
    pub service_capacity_multiplier: f64,
    pub launch_site_open: bool,
    pub recovery_site_open: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaunchRecoveryEvent {
    pub event_id: String,
    pub platform_id: String,
    pub operation: String,
    pub start_h: f64,
    pub end_h: f64,
    pub purpose: String,
    pub site_available: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReserveStatePower {
    pub ground_ready_power_w: f64,
    pub airborne_holding_power_w: f64,
    pub launching_power_w: f64,
    pub repositioning_power_w: f64,
    pub on_station_power_w: f64,
    pub recovering_power_w: f64,
    pub planned_maintenance_power_w: f64,
    pub unscheduled_repair_power_w: f64,
    pub grounded_weather_power_w: f64,
    pub inactive_power_w: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceLogisticsLimits {
    pub simultaneous_launch_capacity: usize,
    pub simultaneous_recovery_capacity: usize,
    pub maintenance_bays: usize,
    pub ground_ready_spares: usize,
    pub launch_lead_time_h: f64,
    pub recovery_duration_h: f64,
    pub airborne_holding_slots: usize,
    pub simultaneous_reposition_capacity: usize,
    pub reposition_duration_h: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceAvailabilityResult {
    pub strict_service_available_h: f64,
    pub strict_availability_fraction: f64,
    pub capacity_adequate_h: f64,
    pub capacity_adequacy_fraction: f64,
    pub unserved_service_equivalent_h: f64,
    pub strict_outage_h: f64,
    pub maximum_contiguous_outage_h: f64,
    pub outage_intervals: Vec<ServiceOutageInterval>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceOutageInterval {
    pub start_h: f64,
    pub end_h: f64,
    pub duration_h: f64,
    pub minimum_effective_capacity_units: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FleetServiceAvailabilityReport {
    pub case_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub horizon_h: f64,
    pub spare_rotation: ServiceAvailabilityResult,
    pub primary_only_baseline: ServiceAvailabilityResult,
    pub strict_availability_gain_percentage_points: f64,
    pub capacity_adequacy_gain_percentage_points: f64,
    pub launch_operations: usize,
    pub recovery_operations: usize,
    pub reposition_operations: usize,
    pub maintenance_overlap_violations: usize,
}

pub fn parse_service_availability_case_json(
    bytes: &[u8],
) -> Result<FleetServiceAvailabilityCase, ReliabilityError> {
    let case: FleetServiceAvailabilityCase = serde_json::from_slice(bytes)
        .map_err(|error| ReliabilityError::new(format!("invalid fleet service JSON: {error}")))?;
    validate_service_availability_case(&case)?;
    Ok(case)
}

pub fn validate_service_availability_case(
    case: &FleetServiceAvailabilityCase,
) -> Result<(), ReliabilityError> {
    if case.schema_version != "haps-fleet-service-availability/v1" {
        return Err(ReliabilityError::new("unsupported fleet service schema_version"));
    }
    if case.evidence_class != "synthetic"
        || case.claim_ceiling != "synthetic_fleet_availability_oracle"
    {
        return Err(ReliabilityError::new(
            "this first event-schedule evaluator accepts synthetic fixtures only",
        ));
    }
    validate_positive_finite(case.evaluation_window_h, "evaluation_window_h")?;
    validate_positive_finite(
        case.service_contract.required_service_capacity_units,
        "required_service_capacity_units",
    )?;
    validate_positive_finite(case.service_contract.service_interval_h, "service_interval_h")?;
    if case.service_contract.strict_availability_rule
        != "effective_capacity_units >= required_service_capacity_units"
    {
        return Err(ReliabilityError::new("unsupported strict availability rule"));
    }
    if case.service_contract.capacity_adequacy_rule
        != "sum(min(effective_capacity / demand, 1) * interval_hours) / evaluation_window_hours"
    {
        return Err(ReliabilityError::new("unsupported capacity adequacy rule"));
    }
    if case.platforms.is_empty() {
        return Err(ReliabilityError::new("at least one platform is required"));
    }

    let mut platform_indices = HashSet::with_capacity(case.platforms.len());
    for platform in &case.platforms {
        if platform.platform_id.trim().is_empty()
            || !platform_indices.insert(platform.platform_id.as_str())
        {
            return Err(ReliabilityError::new(format!(
                "platform_id is empty or duplicated: {}",
                platform.platform_id
            )));
        }
        if !matches!(platform.role.as_str(), "primary" | "reserve") {
            return Err(ReliabilityError::new(format!(
                "platform {} has an unsupported role",
                platform.platform_id
            )));
        }
        validate_nonnegative_finite(
            platform.service_capacity_units,
            &format!("platform {} service_capacity_units", platform.platform_id),
        )?;
        if platform.segments.is_empty() {
            return Err(ReliabilityError::new(format!(
                "platform {} has no state segments",
                platform.platform_id
            )));
        }
        if (platform.segments[0].start_h).abs() > TIME_TOLERANCE_H {
            return Err(ReliabilityError::new(format!(
                "platform {} timeline must start at hour zero",
                platform.platform_id
            )));
        }
        let final_end = platform.segments.last().map(|segment| segment.end_h).unwrap_or(0.0);
        if (final_end - case.evaluation_window_h).abs() > TIME_TOLERANCE_H {
            return Err(ReliabilityError::new(format!(
                "platform {} timeline must end at evaluation_window_h",
                platform.platform_id
            )));
        }
        for (segment_index, segment) in platform.segments.iter().enumerate() {
            validate_finite(segment.start_h, "segment start_h")?;
            validate_finite(segment.end_h, "segment end_h")?;
            validate_probability(segment.capacity_fraction, "segment capacity_fraction")?;
            if segment.end_h <= segment.start_h
                || segment.start_h < -TIME_TOLERANCE_H
                || segment.end_h > case.evaluation_window_h + TIME_TOLERANCE_H
            {
                return Err(ReliabilityError::new(format!(
                    "platform {} has an invalid segment interval",
                    platform.platform_id
                )));
            }
            if segment_index > 0 {
                let previous_end = platform.segments[segment_index - 1].end_h;
                if (previous_end - segment.start_h).abs() > TIME_TOLERANCE_H {
                    return Err(ReliabilityError::new(format!(
                        "platform {} timeline has a gap or overlap",
                        platform.platform_id
                    )));
                }
            }
            if segment.state == "on_station" {
                if segment.capacity_fraction <= 0.0 {
                    return Err(ReliabilityError::new(format!(
                        "on_station segment on {} must contribute positive capacity",
                        platform.platform_id
                    )));
                }
            } else if segment.capacity_fraction != 0.0 {
                return Err(ReliabilityError::new(format!(
                    "non-station segment on {} must contribute zero capacity",
                    platform.platform_id
                )));
            }
            if !matches!(
                segment.state.as_str(),
                "on_station"
                    | "ground_ready"
                    | "airborne_holding"
                    | "repositioning"
                    | "launching"
                    | "recovering"
                    | "planned_maintenance"
                    | "unscheduled_repair"
                    | "grounded_weather"
                    | "inactive"
            ) {
                return Err(ReliabilityError::new(format!(
                    "platform {} has unknown state {}",
                    platform.platform_id, segment.state
                )));
            }
        }
    }

    let platform_by_id: std::collections::HashMap<&str, &ServicePlatform> = case
        .platforms
        .iter()
        .map(|platform| (platform.platform_id.as_str(), platform))
        .collect();
    let mut primary_ids = HashSet::new();
    for id in &case.fleet_policy.primary_platform_ids {
        let platform = platform_by_id
            .get(id.as_str())
            .ok_or_else(|| ReliabilityError::new(format!("unknown primary platform {id}")))?;
        if platform.role != "primary" || !primary_ids.insert(id.as_str()) {
            return Err(ReliabilityError::new(format!(
                "primary platform {id} has a duplicate ID or mismatched role"
            )));
        }
    }
    if primary_ids.is_empty() {
        return Err(ReliabilityError::new("at least one primary platform is required"));
    }
    let mut reserve_ids = HashSet::new();
    for id in &case.fleet_policy.reserve_platform_ids {
        let platform = platform_by_id
            .get(id.as_str())
            .ok_or_else(|| ReliabilityError::new(format!("unknown reserve platform {id}")))?;
        if platform.role != "reserve" || !reserve_ids.insert(id.as_str()) || primary_ids.contains(id.as_str()) {
            return Err(ReliabilityError::new(format!(
                "reserve platform {id} has a duplicate ID, primary overlap, or mismatched role"
            )));
        }
    }
    validate_nonnegative_finite(
        case.fleet_policy.reserve_activation_lead_time_h,
        "reserve_activation_lead_time_h",
    )?;
    if !matches!(
        case.fleet_policy.maintenance_rotation_policy.as_str(),
        "no_rotation" | "staggered_one_at_a_time" | "risk_aware_rotation"
    ) {
        return Err(ReliabilityError::new("unsupported maintenance rotation policy"));
    }
    if !matches!(
        case.fleet_policy.reserve_ready_state.as_str(),
        "ground_ready" | "airborne_holding"
    ) {
        return Err(ReliabilityError::new("unsupported reserve ready state"));
    }
    let ready_capacity = if case.fleet_policy.reserve_ready_state == "ground_ready" {
        case.logistics_limits.ground_ready_spares
    } else {
        case.logistics_limits.airborne_holding_slots
    };
    if case.fleet_policy.reserve_platform_ids.len() > ready_capacity {
        return Err(ReliabilityError::new(
            "declared reserve platforms exceed ready-state logistics capacity",
        ));
    }

    let mut event_ids = HashSet::new();
    for event in &case.common_cause_events {
        if event.event_id.trim().is_empty() || !event_ids.insert(event.event_id.as_str()) {
            return Err(ReliabilityError::new("common-cause event_id is empty or duplicated"));
        }
        validate_interval(event.start_h, event.end_h, case.evaluation_window_h, "common-cause event")?;
        validate_probability(event.service_capacity_multiplier, "service_capacity_multiplier")?;
        if event.affected_platform_ids.is_empty() {
            return Err(ReliabilityError::new(format!(
                "common-cause event {} affects no platform",
                event.event_id
            )));
        }
        let mut affected = HashSet::new();
        for id in &event.affected_platform_ids {
            if !platform_by_id.contains_key(id.as_str()) || !affected.insert(id.as_str()) {
                return Err(ReliabilityError::new(format!(
                    "common-cause event {} has an unknown or duplicated platform ID {}",
                    event.event_id, id
                )));
            }
        }
        if !matches!(
            event.event_class.as_str(),
            "stratospheric_weather"
                | "launch_site_weather"
                | "shared_ground_infrastructure"
                | "common_component_lot"
                | "communications_backhaul"
                | "other_declared"
        ) {
            return Err(ReliabilityError::new(format!(
                "common-cause event {} has an unknown event_class",
                event.event_id
            )));
        }
    }

    validate_nonnegative_finite(case.logistics_limits.launch_lead_time_h, "launch_lead_time_h")?;
    validate_nonnegative_finite(case.logistics_limits.recovery_duration_h, "recovery_duration_h")?;
    validate_nonnegative_finite(
        case.logistics_limits.reposition_duration_h,
        "reposition_duration_h",
    )?;
    for (label, value) in [
        ("ground_ready_power_w", case.reserve_state_power_w.ground_ready_power_w),
        ("airborne_holding_power_w", case.reserve_state_power_w.airborne_holding_power_w),
        ("launching_power_w", case.reserve_state_power_w.launching_power_w),
        ("repositioning_power_w", case.reserve_state_power_w.repositioning_power_w),
        ("on_station_power_w", case.reserve_state_power_w.on_station_power_w),
        ("recovering_power_w", case.reserve_state_power_w.recovering_power_w),
        ("planned_maintenance_power_w", case.reserve_state_power_w.planned_maintenance_power_w),
        ("unscheduled_repair_power_w", case.reserve_state_power_w.unscheduled_repair_power_w),
        ("grounded_weather_power_w", case.reserve_state_power_w.grounded_weather_power_w),
        ("inactive_power_w", case.reserve_state_power_w.inactive_power_w),
    ] {
        validate_nonnegative_finite(value, label)?;
    }
    let mut operation_ids = HashSet::new();
    for operation in &case.launch_recovery_events {
        if operation.event_id.trim().is_empty() || !operation_ids.insert(operation.event_id.as_str()) {
            return Err(ReliabilityError::new("launch/recovery event_id is empty or duplicated"));
        }
        let platform = platform_by_id
            .get(operation.platform_id.as_str())
            .ok_or_else(|| ReliabilityError::new(format!("unknown operation platform {}", operation.platform_id)))?;
        validate_interval(operation.start_h, operation.end_h, case.evaluation_window_h, "launch/recovery")?;
        if !operation.site_available {
            return Err(ReliabilityError::new(format!(
                "operation {} is declared when its site is unavailable",
                operation.event_id
            )));
        }
        let (required_state, expected_duration, site_check) = match operation.operation.as_str() {
            "launch" => ("launching", case.logistics_limits.launch_lead_time_h, Some(true)),
            "recovery" => ("recovering", case.logistics_limits.recovery_duration_h, Some(false)),
            "reposition" => ("repositioning", case.logistics_limits.reposition_duration_h, None),
            _ => return Err(ReliabilityError::new(format!("unknown operation {}", operation.operation))),
        };
        if (operation.end_h - operation.start_h - expected_duration).abs() > TIME_TOLERANCE_H {
            return Err(ReliabilityError::new(format!(
                "operation {} duration does not match logistics limit",
                operation.event_id
            )));
        }
        let matching_segment = platform.segments.iter().any(|segment| {
            segment.state == required_state
                && (segment.start_h - operation.start_h).abs() <= TIME_TOLERANCE_H
                && (segment.end_h - operation.end_h).abs() <= TIME_TOLERANCE_H
        });
        if !matching_segment {
            return Err(ReliabilityError::new(format!(
                "operation {} does not match a platform {required_state} segment",
                operation.event_id
            )));
        }
        if let Some(launch_or_recovery) = site_check {
            for event in &case.common_cause_events {
                let site_is_open = if launch_or_recovery {
                    event.launch_site_open
                } else {
                    event.recovery_site_open
                };
                let overlaps = operation.start_h < event.end_h && event.start_h < operation.end_h;
                if overlaps && !site_is_open {
                    return Err(ReliabilityError::new(format!(
                        "operation {} overlaps common-cause site closure",
                        operation.event_id
                    )));
                }
            }
        }
    }

    let mut boundaries = vec![0.0, case.evaluation_window_h];
    for platform in &case.platforms {
        for segment in &platform.segments {
            boundaries.push(segment.start_h);
            boundaries.push(segment.end_h);
        }
    }
    for event in &case.common_cause_events {
        boundaries.push(event.start_h);
        boundaries.push(event.end_h);
    }
    boundaries.sort_by(f64::total_cmp);
    boundaries.dedup_by(|a, b| (*a - *b).abs() <= TIME_TOLERANCE_H);
    for pair in boundaries.windows(2) {
        let mid = (pair[0] + pair[1]) / 2.0;
        let maintenance_count = case
            .platforms
            .iter()
            .flat_map(|platform| platform.segments.iter())
            .filter(|segment| {
                mid >= segment.start_h
                    && mid < segment.end_h
                    && matches!(segment.state.as_str(), "planned_maintenance" | "unscheduled_repair")
            })
            .count();
        if maintenance_count > case.logistics_limits.maintenance_bays {
            return Err(ReliabilityError::new(format!(
                "maintenance/repair concurrency {maintenance_count} exceeds bay capacity {}",
                case.logistics_limits.maintenance_bays
            )));
        }
        let launch_count = case
            .launch_recovery_events
            .iter()
            .filter(|event| event.operation == "launch" && mid >= event.start_h && mid < event.end_h)
            .count();
        if launch_count > case.logistics_limits.simultaneous_launch_capacity {
            return Err(ReliabilityError::new(format!(
                "launch concurrency {launch_count} exceeds capacity {}",
                case.logistics_limits.simultaneous_launch_capacity
            )));
        }
        let recovery_count = case
            .launch_recovery_events
            .iter()
            .filter(|event| event.operation == "recovery" && mid >= event.start_h && mid < event.end_h)
            .count();
        if recovery_count > case.logistics_limits.simultaneous_recovery_capacity {
            return Err(ReliabilityError::new(format!(
                "recovery concurrency {recovery_count} exceeds capacity {}",
                case.logistics_limits.simultaneous_recovery_capacity
            )));
        }
        let reposition_count = case
            .launch_recovery_events
            .iter()
            .filter(|event| event.operation == "reposition" && mid >= event.start_h && mid < event.end_h)
            .count();
        if reposition_count > case.logistics_limits.simultaneous_reposition_capacity {
            return Err(ReliabilityError::new(format!(
                "reposition concurrency {reposition_count} exceeds capacity {}",
                case.logistics_limits.simultaneous_reposition_capacity
            )));
        }
    }

    Ok(())
}

pub fn evaluate_service_availability(
    case: &FleetServiceAvailabilityCase,
) -> Result<FleetServiceAvailabilityReport, ReliabilityError> {
    validate_service_availability_case(case)?;
    let all_ids: Vec<&str> = case.platforms.iter().map(|p| p.platform_id.as_str()).collect();
    let primary_ids: Vec<&str> = case
        .fleet_policy
        .primary_platform_ids
        .iter()
        .map(String::as_str)
        .collect();
    let spare_rotation = evaluate_platform_set(case, &all_ids)?;
    let primary_only_baseline = evaluate_platform_set(case, &primary_ids)?;
    let launch_operations = case
        .launch_recovery_events
        .iter()
        .filter(|event| event.operation == "launch")
        .count();
    let recovery_operations = case
        .launch_recovery_events
        .iter()
        .filter(|event| event.operation == "recovery")
        .count();
    let reposition_operations = case
        .launch_recovery_events
        .iter()
        .filter(|event| event.operation == "reposition")
        .count();
    Ok(FleetServiceAvailabilityReport {
        case_id: case.case_id.clone(),
        evidence_class: case.evidence_class.clone(),
        claim_ceiling: case.claim_ceiling.clone(),
        horizon_h: case.evaluation_window_h,
        strict_availability_gain_percentage_points: (spare_rotation.strict_availability_fraction
            - primary_only_baseline.strict_availability_fraction)
            * 100.0,
        capacity_adequacy_gain_percentage_points: (spare_rotation.capacity_adequacy_fraction
            - primary_only_baseline.capacity_adequacy_fraction)
            * 100.0,
        spare_rotation,
        primary_only_baseline,
        launch_operations,
        recovery_operations,
        reposition_operations,
        maintenance_overlap_violations: 0,
    })
}

fn evaluate_platform_set(
    case: &FleetServiceAvailabilityCase,
    platform_ids: &[&str],
) -> Result<ServiceAvailabilityResult, ReliabilityError> {
    let mut boundaries = vec![0.0, case.evaluation_window_h];
    for platform in &case.platforms {
        if platform_ids.contains(&platform.platform_id.as_str()) {
            for segment in &platform.segments {
                boundaries.push(segment.start_h);
                boundaries.push(segment.end_h);
            }
        }
    }
    for event in &case.common_cause_events {
        boundaries.push(event.start_h);
        boundaries.push(event.end_h);
    }
    boundaries.sort_by(f64::total_cmp);
    boundaries.dedup_by(|a, b| (*a - *b).abs() <= TIME_TOLERANCE_H);

    let mut strict_hours = 0.0;
    let mut adequate_hours = 0.0;
    let mut outage_hours = 0.0;
    let mut outages: Vec<ServiceOutageInterval> = Vec::new();
    for pair in boundaries.windows(2) {
        let start = pair[0];
        let end = pair[1];
        let duration = end - start;
        if duration <= TIME_TOLERANCE_H {
            continue;
        }
        let mid = (start + end) / 2.0;
        let mut capacity = 0.0;
        for id in platform_ids {
            let platform = case
                .platforms
                .iter()
                .find(|candidate| candidate.platform_id == *id)
                .ok_or_else(|| ReliabilityError::new(format!("unknown platform {id}")))?;
            if let Some(segment) = platform
                .segments
                .iter()
                .find(|segment| mid >= segment.start_h && mid < segment.end_h)
            {
                if segment.state == "on_station" {
                    let mut multiplier = 1.0;
                    for event in &case.common_cause_events {
                        if mid >= event.start_h
                            && mid < event.end_h
                            && event.affected_platform_ids.iter().any(|affected| affected == *id)
                        {
                            multiplier *= event.service_capacity_multiplier;
                        }
                    }
                    capacity += platform.service_capacity_units * segment.capacity_fraction * multiplier;
                }
            }
        }
        let demand = case.service_contract.required_service_capacity_units;
        let adequacy = (capacity / demand).min(1.0);
        adequate_hours += adequacy * duration;
        if capacity + CAPACITY_TOLERANCE >= demand {
            strict_hours += duration;
        } else {
            outage_hours += duration;
            if let Some(last) = outages.last_mut() {
                if (last.end_h - start).abs() <= TIME_TOLERANCE_H {
                    last.end_h = end;
                    last.duration_h += duration;
                    last.minimum_effective_capacity_units =
                        last.minimum_effective_capacity_units.min(capacity);
                } else {
                    outages.push(ServiceOutageInterval {
                        start_h: start,
                        end_h: end,
                        duration_h: duration,
                        minimum_effective_capacity_units: capacity,
                    });
                }
            } else {
                outages.push(ServiceOutageInterval {
                    start_h: start,
                    end_h: end,
                    duration_h: duration,
                    minimum_effective_capacity_units: capacity,
                });
            }
        }
    }
    let window = case.evaluation_window_h;
    let reserve_ids: HashSet<&str> = case
        .fleet_policy
        .reserve_platform_ids
        .iter()
        .map(String::as_str)
        .collect();
    let mut reserve_strategy_energy_wh = 0.0;
    for platform in &case.platforms {
        if reserve_ids.contains(platform.platform_id.as_str())
            && platform_ids.contains(&platform.platform_id.as_str())
        {
            for segment in &platform.segments {
                let power_w = match segment.state.as_str() {
                    "ground_ready" => case.reserve_state_power_w.ground_ready_power_w,
                    "airborne_holding" => case.reserve_state_power_w.airborne_holding_power_w,
                    "launching" => case.reserve_state_power_w.launching_power_w,
                    "repositioning" => case.reserve_state_power_w.repositioning_power_w,
                    "on_station" => case.reserve_state_power_w.on_station_power_w,
                    "recovering" => case.reserve_state_power_w.recovering_power_w,
                    "planned_maintenance" => case.reserve_state_power_w.planned_maintenance_power_w,
                    "unscheduled_repair" => case.reserve_state_power_w.unscheduled_repair_power_w,
                    "grounded_weather" => case.reserve_state_power_w.grounded_weather_power_w,
                    "inactive" => case.reserve_state_power_w.inactive_power_w,
                    _ => return Err(ReliabilityError::new("unknown reserve state while charging energy")),
                };
                reserve_strategy_energy_wh += power_w * (segment.end_h - segment.start_h);
            }
        }
    }
    Ok(ServiceAvailabilityResult {
        strict_service_available_h: strict_hours,
        strict_availability_fraction: strict_hours / window,
        capacity_adequate_h: adequate_hours,
        capacity_adequacy_fraction: adequate_hours / window,
        unserved_service_equivalent_h: window - adequate_hours,
        strict_outage_h: outage_hours,
        maximum_contiguous_outage_h: outages
            .iter()
            .map(|outage| outage.duration_h)
            .fold(0.0, f64::max),
        reserve_strategy_energy_wh,
        outage_intervals: outages,
    })
}

fn validate_interval(
    start: f64,
    end: f64,
    window: f64,
    label: &str,
) -> Result<(), ReliabilityError> {
    validate_finite(start, &format!("{label} start"))?;
    validate_finite(end, &format!("{label} end"))?;
    if start < -TIME_TOLERANCE_H || end > window + TIME_TOLERANCE_H || end <= start {
        return Err(ReliabilityError::new(format!("{label} interval is outside the evaluation window")));
    }
    Ok(())
}

fn validate_finite(value: f64, label: &str) -> Result<(), ReliabilityError> {
    if !value.is_finite() {
        return Err(ReliabilityError::new(format!("{label} must be finite")));
    }
    Ok(())
}

fn validate_positive_finite(value: f64, label: &str) -> Result<(), ReliabilityError> {
    validate_finite(value, label)?;
    if value <= 0.0 {
        return Err(ReliabilityError::new(format!("{label} must be positive")));
    }
    Ok(())
}

fn validate_nonnegative_finite(value: f64, label: &str) -> Result<(), ReliabilityError> {
    validate_finite(value, label)?;
    if value < 0.0 {
        return Err(ReliabilityError::new(format!("{label} must be non-negative")));
    }
    Ok(())
}

fn validate_probability(value: f64, label: &str) -> Result<(), ReliabilityError> {
    validate_finite(value, label)?;
    if !(0.0..=1.0).contains(&value) {
        return Err(ReliabilityError::new(format!("{label} must lie in [0, 1]")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CASE: &str = include_str!(
        "../../../docs/research/aviation/examples/synthetic_haps_fleet_30day_availability_v1.json"
    );

    fn parse() -> FleetServiceAvailabilityCase {
        parse_service_availability_case_json(CASE.as_bytes()).expect("synthetic fleet case is valid")
    }

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-9,
            "actual {actual}, expected {expected}"
        );
    }

    #[test]
    fn reserve_rotation_matches_hourly_service_oracle() {
        let report = evaluate_service_availability(&parse()).unwrap();
        close(report.spare_rotation.strict_availability_fraction, 0.95);
        close(report.spare_rotation.capacity_adequacy_fraction, 0.9844444444444445);
        close(report.spare_rotation.strict_outage_h, 36.0);
        close(report.spare_rotation.maximum_contiguous_outage_h, 24.0);
        assert_eq!(report.spare_rotation.outage_intervals.len(), 2);
        assert_eq!(report.launch_operations, 3);
        assert_eq!(report.recovery_operations, 3);
        close(report.strict_availability_gain_percentage_points, 8.333333333333329);
        close(report.capacity_adequacy_gain_percentage_points, 2.77777777777778);
        assert_eq!(report.maintenance_overlap_violations, 0);
    }

    #[test]
    fn primary_only_baseline_matches_no_spare_oracle() {
        let report = evaluate_service_availability(&parse()).unwrap();
        close(report.primary_only_baseline.strict_availability_fraction, 0.8666666666666667);
        close(report.primary_only_baseline.capacity_adequacy_fraction, 0.9566666666666667);
        close(report.primary_only_baseline.strict_outage_h, 96.0);
        close(report.primary_only_baseline.maximum_contiguous_outage_h, 24.0);
        assert_eq!(report.primary_only_baseline.outage_intervals.len(), 4);
    }

    #[test]
    fn weather_common_cause_still_defeats_single_spare() {
        let report = evaluate_service_availability(&parse()).unwrap();
        let weather = report.spare_rotation.outage_intervals.last().unwrap();
        assert_eq!(weather.start_h, 600.0);
        assert_eq!(weather.end_h, 624.0);
        close(weather.minimum_effective_capacity_units, 2.1);
    }

    #[test]
    fn parser_rejects_a_timeline_gap() {
        let mut case = parse();
        case.platforms[0].segments[1].start_h += 1.0;
        assert!(validate_service_availability_case(&case).is_err());
    }

    #[test]
    fn parser_rejects_maintenance_bay_overcommitment() {
        let mut case = parse();
        case.platforms[1].segments[1].start_h = 240.0;
        case.platforms[1].segments[1].end_h = 264.0;
        assert!(validate_service_availability_case(&case).is_err());
    }

    #[test]
    fn parser_rejects_launch_during_common_site_closure() {
        let mut case = parse();
        case.common_cause_events[0].launch_site_open = false;
        assert!(validate_service_availability_case(&case).is_err());
    }
}
