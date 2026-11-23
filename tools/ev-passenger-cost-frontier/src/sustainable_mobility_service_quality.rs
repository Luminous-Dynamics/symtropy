// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic request admission over prequalified aerial/fallback service slots.
//!
//! This module consumes a bounded capacity envelope from #6909 and externally
//! supplied service opportunities. It does not simulate batteries, route
//! aircraft, select strategic modes, or authorize flight. Unknown/stale
//! permission or weather evidence blocks aerial use.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::sustainable_mobility_service_capacity::{
    CapacityServiceOpportunity, SustainableServiceCapacityInput,
    evaluate_sustainable_service_capacity,
};

const MAX_REQUESTS: usize = 512;
const MAX_AERIAL_OPPORTUNITIES: usize = 512;
const MAX_FALLBACK_OPPORTUNITIES: usize = 512;
const MAX_FLIGHT_WINDOWS: usize = 256;
const MAX_BLOCKED_WINDOWS: usize = 256;
const MAX_HORIZON_SECONDS: u64 = 7 * 24 * 60 * 60;
const MAX_SERVICE_SECONDS: u32 = 24 * 60 * 60;
const MAX_PAYLOAD_UNITS: u16 = 10_000;
const MAX_OPPORTUNITIES_FROM_CAPACITY: u32 = 2_048;
const CLAIM_CEILING: &str = "Deterministic request-to-service-slot admission model only. Legacy mode accepts supplied opportunities; linked mode derives aerial schedule fields from a re-evaluated synthetic capacity event model. Payload/weather/permission evidence is supplied and unauthenticated. No route planning, flight/dispatch, physical, market, regulatory or customer-delivery authority.";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceMode {
    AerialOnly,
    FallbackOnly,
    Either,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionPolicy {
    FirstComeFirstServed,
    EarliestDeadlineFirst,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WeatherState {
    Go,
    NoGo,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionState {
    Approved,
    Denied,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    Synthetic,
    Measured,
    IndependentTest,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FallbackMode {
    GroundCourier,
    Locker,
    Amr,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BlockedWindowKind {
    Maintenance,
    SiteClosed,
    NoGo,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacitySource {
    pub source_report_digest: String,
    pub source_architecture_id: String,
    pub horizon_seconds: u64,
    pub steady_state_cycle_capacity_upper_bound_per_hour: f64,
    pub initial_inventory_capacity_allowance_per_hour: f64,
    pub max_aerial_opportunities: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceRequest {
    pub request_id: String,
    pub arrival_seconds: u64,
    /// Latest time at which the request is accepted into this service system.
    pub accept_by_seconds: u64,
    /// Latest desired completion time. A job may start by this time and finish late.
    pub deadline_seconds: u64,
    pub required_service_seconds: u32,
    pub payload_units: u16,
    pub service_class: String,
    pub allowed_mode: ServiceMode,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AerialServiceOpportunity {
    pub opportunity_id: String,
    pub aircraft_id: String,
    pub start_seconds: u64,
    pub max_service_duration_seconds: u32,
    pub max_payload_units: u16,
    pub turnaround_seconds: u32,
    pub capacity_reference: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FallbackServiceOpportunity {
    pub opportunity_id: String,
    pub fallback_resource_id: String,
    pub fallback_mode: FallbackMode,
    pub start_seconds: u64,
    pub max_service_duration_seconds: u32,
    pub max_payload_units: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FlightEvidence {
    pub site_id: String,
    pub site_configuration_digest: String,
    pub producer_id: String,
    pub method_id: String,
    pub evidence_reference: String,
    pub evidence_class: EvidenceClass,
    /// Observation time; must be no later than the flight-window decision time.
    pub observed_at_seconds: u64,
    pub valid_from_seconds: u64,
    pub valid_until_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FlightWindow {
    pub window_id: String,
    pub start_seconds: u64,
    pub end_seconds: u64,
    pub weather_state: WeatherState,
    pub permission_state: PermissionState,
    pub evidence: FlightEvidence,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BlockedInterval {
    pub interval_id: String,
    pub start_seconds: u64,
    pub end_seconds: u64,
    pub kind: BlockedWindowKind,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceQualityInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub site_id: String,
    pub site_configuration_digest: String,
    pub horizon_seconds: u64,
    pub capacity_source: CapacitySource,
    pub requests: Vec<ServiceRequest>,
    pub aerial_opportunities: Vec<AerialServiceOpportunity>,
    pub fallback_opportunities: Vec<FallbackServiceOpportunity>,
    pub flight_windows: Vec<FlightWindow>,
    pub blocked_intervals: Vec<BlockedInterval>,
    pub policies: Vec<AdmissionPolicy>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PayloadQualificationEvidence {
    pub capacity_opportunity_id: String,
    pub aircraft_id: String,
    pub site_id: String,
    pub site_configuration_digest: String,
    pub architecture_configuration_digest: String,
    pub producer_id: String,
    pub method_id: String,
    pub evidence_reference: String,
    pub evidence_class: EvidenceClass,
    pub observed_at_seconds: u64,
    pub valid_from_seconds: u64,
    pub valid_until_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityOpportunityQualification {
    /// The only capacity identifier supplied by this input. Schedule fields
    /// are resolved from the event-derived upstream report.
    pub opportunity_id: String,
    pub max_payload_units: u16,
    pub evidence: PayloadQualificationEvidence,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LinkedServiceQualityInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub site_id: String,
    pub site_configuration_digest: String,
    pub horizon_seconds: u64,
    pub source_architecture_id: String,
    pub requests: Vec<ServiceRequest>,
    pub aerial_opportunity_qualifications: Vec<CapacityOpportunityQualification>,
    pub fallback_opportunities: Vec<FallbackServiceOpportunity>,
    pub flight_windows: Vec<FlightWindow>,
    pub blocked_intervals: Vec<BlockedInterval>,
    pub policies: Vec<AdmissionPolicy>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExactOracleStatus {
    Evaluated,
    NotRunBoundsExceeded,
    NodeLimitReached,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExactOracleScore {
    pub completed_on_time: usize,
    pub completed_late: usize,
    pub completed_by_horizon: usize,
    pub in_progress_at_horizon: usize,
    pub total_lateness_seconds: u64,
    pub total_latency_seconds: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExactOracleAssignment {
    pub request_id: String,
    pub opportunity_id: String,
    pub fulfilment_mode: FulfilmentMode,
    pub service_start_seconds: u64,
    pub projected_completion_seconds: u64,
    pub disposition: RequestDisposition,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExactOraclePolicyGap {
    pub policy: AdmissionPolicy,
    pub on_time_completion_gap: usize,
    pub completed_by_horizon_gap: usize,
    pub lateness_excess_seconds: u64,
    pub latency_excess_seconds: u64,
    pub in_progress_excess: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExactOracleReport {
    pub status: ExactOracleStatus,
    pub request_bound: usize,
    pub opportunity_bound: usize,
    pub search_node_limit: usize,
    pub search_nodes: usize,
    pub best_score: Option<ExactOracleScore>,
    pub best_assignments: Vec<ExactOracleAssignment>,
    pub policy_gaps: Vec<ExactOraclePolicyGap>,
    pub objective: &'static str,
}

const MAX_EXACT_ORACLE_REQUESTS: usize = 8;
const MAX_EXACT_ORACLE_OPPORTUNITIES: usize = 8;
const MAX_EXACT_ORACLE_NODES: usize = 1_000_000;

#[derive(Debug, Clone)]
struct ExactOracleCandidate {
    request_index: usize,
    completion_seconds: u64,
    mode: FulfilmentMode,
}

#[derive(Debug, Clone)]
struct ExactOracleBest {
    score: ExactOracleScore,
    assignments: Vec<Option<ExactOracleAssignment>>,
    canonical_key: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LinkedServiceQualityReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub horizon_seconds: u64,
    pub capacity_source_architecture_id: String,
    pub capacity_source_report_digest: String,
    pub capacity_service_opportunity_digest: String,
    pub capacity_architecture_configuration_digest: String,
    pub capacity_opportunities_in_source: usize,
    pub capacity_opportunities_qualified: usize,
    pub policy_results: Vec<ServicePolicyResult>,
    pub exact_oracle: ExactOracleReport,
    pub service_quality_is_physical_performance_claim: bool,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RequestDisposition {
    RejectedAfterAcceptanceCutoff,
    CompletedOnTime,
    CompletedLate,
    InProgressAtHorizon,
    Unserved,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FulfilmentMode {
    Aerial,
    GroundCourier,
    Locker,
    Amr,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RequestOutcome {
    pub request_id: String,
    pub disposition: RequestDisposition,
    pub fulfilment_mode: Option<FulfilmentMode>,
    pub opportunity_id: Option<String>,
    pub arrival_seconds: u64,
    pub service_start_seconds: Option<u64>,
    /// Present only when the service actually completed within the model horizon.
    pub service_completion_seconds: Option<u64>,
    /// A projected completion beyond the horizon is not counted as a completed service.
    pub projected_completion_seconds: Option<u64>,
    pub latency_seconds: Option<u64>,
    pub queue_wait_seconds: Option<u64>,
    pub denial_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ServicePolicyResult {
    pub policy: AdmissionPolicy,
    pub offered_requests: usize,
    pub accepted_requests: usize,
    pub rejected_requests: usize,
    pub completed_on_time: usize,
    pub completed_late: usize,
    pub in_progress_requests: usize,
    pub unserved_requests: usize,
    /// Completed services only; in-progress assignments are reported separately.
    pub served_by_aerial: usize,
    pub served_by_fallback: usize,
    pub in_progress_by_aerial: usize,
    pub in_progress_by_fallback: usize,
    pub aerial_opportunities_available: usize,
    pub aerial_opportunities_blocked: usize,
    pub aerial_opportunities_unused: usize,
    pub fallback_opportunities_unused: usize,
    pub max_waiting_requests_peak: usize,
    pub completed_requests_per_hour: f64,
    pub aerial_opportunity_utilization_percent: f64,
    pub fallback_opportunity_utilization_percent: f64,
    pub fallback_share_of_completed_percent: f64,
    pub on_time_rate_of_offered_percent: f64,
    pub on_time_rate_of_accepted_percent: f64,
    pub late_rate_of_accepted_percent: f64,
    pub unserved_rate_of_accepted_percent: f64,
    pub p50_latency_seconds: Option<u64>,
    pub p95_latency_seconds: Option<u64>,
    pub p99_latency_seconds: Option<u64>,
    pub mean_latency_seconds: Option<f64>,
    pub mean_queue_wait_seconds: Option<f64>,
    pub max_queue_wait_seconds: u64,
    pub denial_reason_counts: BTreeMap<String, usize>,
    pub outcomes: Vec<RequestOutcome>,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ServiceQualityReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub horizon_seconds: u64,
    pub capacity_source_architecture_id: String,
    pub capacity_source_report_digest: String,
    pub policy_results: Vec<ServicePolicyResult>,
    pub service_quality_is_physical_performance_claim: bool,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone)]
struct ScheduledOpportunity {
    start_seconds: u64,
    is_aerial: bool,
    index: usize,
    opportunity_id: String,
}

#[derive(Debug, Clone)]
struct Assignment {
    opportunity_id: String,
    mode: FulfilmentMode,
    start_seconds: u64,
    completion_seconds: u64,
}

fn add_error(errors: &mut Vec<String>, reason: impl Into<String>) {
    errors.push(reason.into());
}

fn is_sha256_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn overlap(start_a: u64, end_a: u64, start_b: u64, end_b: u64) -> bool {
    start_a < end_b && start_b < end_a
}

fn nearest_rank(values: &mut [u64], percentile: usize) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let rank = percentile.saturating_mul(values.len()).div_ceil(100).max(1);
    values.get(rank - 1).copied()
}

fn validate_input(input: &ServiceQualityInput) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();

    if input.schema_version != "0.1.0"
        || input.benchmark_id != "SUSTAINABLE-MOBILITY-SERVICE-QUALITY-001"
        || input.site_id.trim().is_empty()
        || input.site_configuration_digest.trim().is_empty()
        || input.horizon_seconds == 0
        || input.horizon_seconds > MAX_HORIZON_SECONDS
        || input.claim_ceiling.trim().is_empty()
        || input.requests.is_empty()
        || input.requests.len() > MAX_REQUESTS
        || input.aerial_opportunities.len() > MAX_AERIAL_OPPORTUNITIES
        || input.fallback_opportunities.len() > MAX_FALLBACK_OPPORTUNITIES
        || input.flight_windows.is_empty()
        || input.flight_windows.len() > MAX_FLIGHT_WINDOWS
        || input.blocked_intervals.len() > MAX_BLOCKED_WINDOWS
        || input.policies.is_empty()
        || input.policies.len() > 2
    {
        add_error(&mut errors, "benchmark_identity_or_input_bounds_invalid");
    }

    let capacity = &input.capacity_source;
    if !is_sha256_digest(&capacity.source_report_digest)
        || capacity.source_architecture_id.trim().is_empty()
        || capacity.horizon_seconds != input.horizon_seconds
        || !capacity
            .steady_state_cycle_capacity_upper_bound_per_hour
            .is_finite()
        || capacity
            .steady_state_cycle_capacity_upper_bound_per_hour
            .is_sign_negative()
        || !capacity
            .initial_inventory_capacity_allowance_per_hour
            .is_finite()
        || capacity
            .initial_inventory_capacity_allowance_per_hour
            .is_sign_negative()
        || capacity.max_aerial_opportunities > MAX_OPPORTUNITIES_FROM_CAPACITY
    {
        add_error(&mut errors, "capacity_source_identity_or_bounds_invalid");
    } else {
        let hours = input.horizon_seconds as f64 / 3_600.0;
        let allowed = (capacity.steady_state_cycle_capacity_upper_bound_per_hour * hours
            + capacity.initial_inventory_capacity_allowance_per_hour * hours
            + 1e-9)
            .floor() as u64;
        if u64::from(capacity.max_aerial_opportunities) > allowed {
            add_error(
                &mut errors,
                "aerial_opportunity_budget_exceeds_upstream_capacity_envelope",
            );
        }
    }

    let mut request_ids = BTreeSet::new();
    for request in &input.requests {
        if request.request_id.trim().is_empty()
            || request.request_id.len() > 80
            || !request_ids.insert(request.request_id.as_str())
            || request.arrival_seconds > input.horizon_seconds
            || request.accept_by_seconds > input.horizon_seconds
            || request.deadline_seconds < request.arrival_seconds
            || request.accept_by_seconds > request.deadline_seconds
            || request.deadline_seconds > input.horizon_seconds
            || request.required_service_seconds == 0
            || request.required_service_seconds > MAX_SERVICE_SECONDS
            || request.payload_units == 0
            || request.payload_units > MAX_PAYLOAD_UNITS
            || request.service_class.trim().is_empty()
            || request.service_class.len() > 80
        {
            add_error(
                &mut errors,
                format!("request_identity_or_bounds_invalid:{}", request.request_id),
            );
        }
    }

    let mut opportunity_ids = BTreeSet::new();
    let mut capacity_references = BTreeSet::new();
    let mut by_aircraft: BTreeMap<&str, Vec<&AerialServiceOpportunity>> = BTreeMap::new();
    for opportunity in &input.aerial_opportunities {
        let end = opportunity
            .start_seconds
            .checked_add(u64::from(opportunity.max_service_duration_seconds));
        if opportunity.opportunity_id.trim().is_empty()
            || !opportunity_ids.insert(opportunity.opportunity_id.as_str())
            || opportunity.aircraft_id.trim().is_empty()
            || opportunity.start_seconds > input.horizon_seconds
            || opportunity.max_service_duration_seconds == 0
            || opportunity.max_service_duration_seconds > MAX_SERVICE_SECONDS
            || opportunity.max_payload_units == 0
            || opportunity.max_payload_units > MAX_PAYLOAD_UNITS
            || opportunity.turnaround_seconds > MAX_SERVICE_SECONDS
            || opportunity.capacity_reference.trim().is_empty()
            || !capacity_references.insert(opportunity.capacity_reference.as_str())
            || end.is_none_or(|value| value > input.horizon_seconds)
        {
            add_error(
                &mut errors,
                format!(
                    "aerial_opportunity_identity_or_bounds_invalid:{}",
                    opportunity.opportunity_id
                ),
            );
        }
        by_aircraft
            .entry(opportunity.aircraft_id.as_str())
            .or_default()
            .push(opportunity);
    }
    for (aircraft_id, opportunities) in &mut by_aircraft {
        opportunities.sort_by_key(|item| (item.start_seconds, item.opportunity_id.as_str()));
        for pair in opportunities.windows(2) {
            let busy_until = pair[0]
                .start_seconds
                .saturating_add(u64::from(pair[0].max_service_duration_seconds))
                .saturating_add(u64::from(pair[0].turnaround_seconds));
            if pair[1].start_seconds < busy_until {
                add_error(
                    &mut errors,
                    format!("aerial_opportunities_overlap_on_aircraft:{aircraft_id}"),
                );
            }
        }
    }
    if input.aerial_opportunities.len() > capacity.max_aerial_opportunities as usize {
        add_error(
            &mut errors,
            "aerial_opportunity_count_exceeds_capacity_source_budget",
        );
    }

    let mut by_fallback_resource: BTreeMap<&str, Vec<&FallbackServiceOpportunity>> =
        BTreeMap::new();
    for opportunity in &input.fallback_opportunities {
        if opportunity.opportunity_id.trim().is_empty()
            || !opportunity_ids.insert(opportunity.opportunity_id.as_str())
            || opportunity.fallback_resource_id.trim().is_empty()
            || opportunity.start_seconds > input.horizon_seconds
            || opportunity.max_service_duration_seconds == 0
            || opportunity.max_service_duration_seconds > MAX_SERVICE_SECONDS
            || opportunity.max_payload_units == 0
            || opportunity.max_payload_units > MAX_PAYLOAD_UNITS
        {
            add_error(
                &mut errors,
                format!(
                    "fallback_opportunity_identity_or_bounds_invalid:{}",
                    opportunity.opportunity_id
                ),
            );
        }
        by_fallback_resource
            .entry(opportunity.fallback_resource_id.as_str())
            .or_default()
            .push(opportunity);
    }
    for (resource_id, opportunities) in &mut by_fallback_resource {
        opportunities.sort_by_key(|item| (item.start_seconds, item.opportunity_id.as_str()));
        for pair in opportunities.windows(2) {
            let busy_until = pair[0]
                .start_seconds
                .saturating_add(u64::from(pair[0].max_service_duration_seconds));
            if pair[1].start_seconds < busy_until {
                add_error(
                    &mut errors,
                    format!("fallback_opportunities_overlap_on_resource:{resource_id}"),
                );
            }
        }
    }

    let mut flight_window_ids = BTreeSet::new();
    let mut sorted_windows: Vec<_> = input.flight_windows.iter().collect();
    sorted_windows.sort_by_key(|window| {
        (
            window.start_seconds,
            window.end_seconds,
            window.window_id.as_str(),
        )
    });
    for window in &sorted_windows {
        if window.window_id.trim().is_empty()
            || !flight_window_ids.insert(window.window_id.as_str())
            || window.start_seconds >= window.end_seconds
            || window.end_seconds > input.horizon_seconds
            || window.evidence.site_id != input.site_id
            || window.evidence.site_configuration_digest != input.site_configuration_digest
            || window.evidence.producer_id.trim().is_empty()
            || window.evidence.method_id.trim().is_empty()
            || window.evidence.evidence_reference.trim().is_empty()
            || window.evidence.observed_at_seconds > window.start_seconds
            || window.evidence.valid_from_seconds > window.evidence.observed_at_seconds
            || window.evidence.valid_from_seconds > window.start_seconds
            || window.evidence.valid_until_seconds < window.end_seconds
        {
            add_error(
                &mut errors,
                format!(
                    "flight_window_identity_or_evidence_invalid:{}",
                    window.window_id
                ),
            );
        }
    }
    for pair in sorted_windows.windows(2) {
        if pair[1].start_seconds < pair[0].end_seconds {
            add_error(&mut errors, "flight_windows_overlap_or_are_ambiguous");
        }
    }

    let mut blocked_ids = BTreeSet::new();
    for interval in &input.blocked_intervals {
        if interval.interval_id.trim().is_empty()
            || !blocked_ids.insert(interval.interval_id.as_str())
            || interval.start_seconds >= interval.end_seconds
            || interval.end_seconds > input.horizon_seconds
        {
            add_error(
                &mut errors,
                format!(
                    "blocked_interval_identity_or_bounds_invalid:{}",
                    interval.interval_id
                ),
            );
        }
    }

    let mut policies = BTreeSet::new();
    for policy in &input.policies {
        if !policies.insert(*policy) {
            add_error(&mut errors, "admission_policy_duplicate");
        }
    }

    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }
    Ok(())
}

fn flight_window_for(
    input: &ServiceQualityInput,
    start_seconds: u64,
    completion_seconds: u64,
) -> Result<Option<&FlightWindow>, String> {
    let matches: Vec<_> = input
        .flight_windows
        .iter()
        .filter(|window| {
            window.start_seconds <= start_seconds && completion_seconds <= window.end_seconds
        })
        .collect();
    if matches.len() > 1 {
        return Err("flight_window_ambiguous_for_aerial_opportunity".to_owned());
    }
    Ok(matches.first().copied())
}

fn aerial_block_reason(
    input: &ServiceQualityInput,
    opportunity: &AerialServiceOpportunity,
    request_duration: u32,
) -> Result<Option<String>, String> {
    let Some(completion) = opportunity
        .start_seconds
        .checked_add(u64::from(request_duration))
    else {
        return Err("aerial_service_completion_overflow".to_owned());
    };
    let Some(window) = flight_window_for(input, opportunity.start_seconds, completion)? else {
        return Ok(Some("no_flight_window_covers_service_interval".to_owned()));
    };

    if window.evidence.observed_at_seconds > opportunity.start_seconds {
        return Err("future_weather_or_permission_evidence_leakage".to_owned());
    }
    if window.evidence.evidence_class == EvidenceClass::Unknown {
        return Ok(Some("flight_evidence_unknown".to_owned()));
    }
    if window.weather_state != WeatherState::Go {
        return Ok(Some(match window.weather_state {
            WeatherState::NoGo => "weather_no_go".to_owned(),
            WeatherState::Unknown => "weather_unknown".to_owned(),
            WeatherState::Go => unreachable!(),
        }));
    }
    if window.permission_state != PermissionState::Approved {
        return Ok(Some(match window.permission_state {
            PermissionState::Denied => "permission_denied".to_owned(),
            PermissionState::Unknown => "permission_unknown".to_owned(),
            PermissionState::Approved => unreachable!(),
        }));
    }

    let occupancy_end = completion
        .checked_add(u64::from(opportunity.turnaround_seconds))
        .ok_or_else(|| "aerial_occupancy_end_overflow".to_owned())?;
    if input.blocked_intervals.iter().any(|interval| {
        overlap(
            opportunity.start_seconds,
            occupancy_end,
            interval.start_seconds,
            interval.end_seconds,
        )
    }) {
        return Ok(Some("maintenance_or_site_blocked_interval".to_owned()));
    }
    Ok(None)
}

fn request_priority(request: &ServiceRequest, policy: AdmissionPolicy) -> (u64, u64, String) {
    match policy {
        AdmissionPolicy::FirstComeFirstServed => {
            (request.arrival_seconds, 0, request.request_id.clone())
        }
        AdmissionPolicy::EarliestDeadlineFirst => (
            request.deadline_seconds,
            request.arrival_seconds,
            request.request_id.clone(),
        ),
    }
}

fn mode_for_fallback(mode: FallbackMode) -> FulfilmentMode {
    match mode {
        FallbackMode::GroundCourier => FulfilmentMode::GroundCourier,
        FallbackMode::Locker => FulfilmentMode::Locker,
        FallbackMode::Amr => FulfilmentMode::Amr,
    }
}

fn evaluate_policy(
    input: &ServiceQualityInput,
    policy: AdmissionPolicy,
) -> Result<ServicePolicyResult, Vec<String>> {
    let mut outcomes: BTreeMap<String, RequestOutcome> = BTreeMap::new();
    let mut eligible_request_indices = Vec::new();
    let mut denial_reason_counts = BTreeMap::new();

    for (index, request) in input.requests.iter().enumerate() {
        if request.arrival_seconds > request.accept_by_seconds {
            outcomes.insert(
                request.request_id.clone(),
                RequestOutcome {
                    request_id: request.request_id.clone(),
                    disposition: RequestDisposition::RejectedAfterAcceptanceCutoff,
                    fulfilment_mode: None,
                    opportunity_id: None,
                    arrival_seconds: request.arrival_seconds,
                    service_start_seconds: None,
                    service_completion_seconds: None,
                    projected_completion_seconds: None,
                    latency_seconds: None,
                    queue_wait_seconds: None,
                    denial_reason: Some("arrival_after_acceptance_cutoff".to_owned()),
                },
            );
        } else {
            eligible_request_indices.push(index);
        }
    }

    let mut opportunities = Vec::new();
    for (index, opportunity) in input.aerial_opportunities.iter().enumerate() {
        opportunities.push(ScheduledOpportunity {
            start_seconds: opportunity.start_seconds,
            is_aerial: true,
            index,
            opportunity_id: opportunity.opportunity_id.clone(),
        });
    }
    for (index, opportunity) in input.fallback_opportunities.iter().enumerate() {
        opportunities.push(ScheduledOpportunity {
            start_seconds: opportunity.start_seconds,
            is_aerial: false,
            index,
            opportunity_id: opportunity.opportunity_id.clone(),
        });
    }
    opportunities.sort_by_key(|opportunity| {
        (
            opportunity.start_seconds,
            if opportunity.is_aerial { 0_u8 } else { 1_u8 },
            opportunity.opportunity_id.clone(),
        )
    });

    let mut assignments: BTreeMap<String, Assignment> = BTreeMap::new();
    let mut blocked_aerial_ids = BTreeSet::new();
    let mut aerial_opportunities_available = 0_usize;
    let mut max_waiting_requests_peak = 0_usize;
    let mut aerial_opportunities_unused = 0_usize;
    let mut fallback_opportunities_unused = 0_usize;

    for opportunity in &opportunities {
        let waiting_now = input
            .requests
            .iter()
            .filter(|request| {
                request.arrival_seconds <= request.accept_by_seconds
                    && request.arrival_seconds <= opportunity.start_seconds
                    && opportunity.start_seconds <= request.deadline_seconds
                    && !assignments.contains_key(&request.request_id)
            })
            .count();
        max_waiting_requests_peak = max_waiting_requests_peak.max(waiting_now);

        let mut candidates: Vec<usize> = eligible_request_indices
            .iter()
            .copied()
            .filter(|index| {
                let request = &input.requests[*index];
                !assignments.contains_key(&request.request_id)
                    && request.arrival_seconds <= opportunity.start_seconds
                    && opportunity.start_seconds <= request.deadline_seconds
                    && (if opportunity.is_aerial {
                        matches!(
                            request.allowed_mode,
                            ServiceMode::AerialOnly | ServiceMode::Either
                        )
                    } else {
                        matches!(
                            request.allowed_mode,
                            ServiceMode::FallbackOnly | ServiceMode::Either
                        )
                    })
            })
            .filter(|index| {
                if opportunity.is_aerial {
                    let aerial = &input.aerial_opportunities[opportunity.index];
                    aerial.max_service_duration_seconds
                        >= input.requests[*index].required_service_seconds
                        && aerial.max_payload_units >= input.requests[*index].payload_units
                } else {
                    let fallback = &input.fallback_opportunities[opportunity.index];
                    fallback.max_service_duration_seconds
                        >= input.requests[*index].required_service_seconds
                        && fallback.max_payload_units >= input.requests[*index].payload_units
                }
            })
            .collect();

        if opportunity.is_aerial {
            if candidates.is_empty() {
                aerial_opportunities_unused += 1;
                continue;
            }
            let aerial = &input.aerial_opportunities[opportunity.index];
            let mut any_qualified = false;
            let mut reasons = BTreeSet::new();
            for index in &candidates {
                match aerial_block_reason(
                    input,
                    aerial,
                    input.requests[*index].required_service_seconds,
                ) {
                    Ok(None) => any_qualified = true,
                    Ok(Some(reason)) => {
                        reasons.insert(reason);
                    }
                    Err(error) => return Err(vec![error]),
                }
            }
            if !any_qualified {
                blocked_aerial_ids.insert(aerial.opportunity_id.clone());
                let _ = reasons;
                continue;
            }
            aerial_opportunities_available += 1;
        }

        candidates.retain(|index| {
            if opportunity.is_aerial {
                aerial_block_reason(
                    input,
                    &input.aerial_opportunities[opportunity.index],
                    input.requests[*index].required_service_seconds,
                )
                .is_ok_and(|result| result.is_none())
            } else {
                true
            }
        });
        candidates.sort_by_key(|index| request_priority(&input.requests[*index], policy));
        let Some(request_index) = candidates.first().copied() else {
            if opportunity.is_aerial {
                aerial_opportunities_unused += 1;
            } else {
                fallback_opportunities_unused += 1;
            }
            continue;
        };

        let request = &input.requests[request_index];
        let (opportunity_id, mode, duration) = if opportunity.is_aerial {
            let aerial = &input.aerial_opportunities[opportunity.index];
            (
                aerial.opportunity_id.clone(),
                FulfilmentMode::Aerial,
                request.required_service_seconds,
            )
        } else {
            let fallback = &input.fallback_opportunities[opportunity.index];
            (
                fallback.opportunity_id.clone(),
                mode_for_fallback(fallback.fallback_mode),
                request.required_service_seconds,
            )
        };
        let completion = opportunity
            .start_seconds
            .checked_add(u64::from(duration))
            .ok_or_else(|| vec!["service_completion_time_overflow".to_owned()])?;
        let latency = completion.saturating_sub(request.arrival_seconds);
        let wait = opportunity
            .start_seconds
            .saturating_sub(request.arrival_seconds);
        assignments.insert(
            request.request_id.clone(),
            Assignment {
                opportunity_id,
                mode,
                start_seconds: opportunity.start_seconds,
                completion_seconds: completion,
            },
        );
        let _ = (latency, wait);
    }

    for request in &input.requests {
        if let Some(assignment) = assignments.get(&request.request_id) {
            let latency = assignment
                .completion_seconds
                .saturating_sub(request.arrival_seconds);
            let wait = assignment
                .start_seconds
                .saturating_sub(request.arrival_seconds);
            let disposition = if assignment.completion_seconds > input.horizon_seconds {
                RequestDisposition::InProgressAtHorizon
            } else if assignment.completion_seconds <= request.deadline_seconds {
                RequestDisposition::CompletedOnTime
            } else {
                RequestDisposition::CompletedLate
            };
            let completed = disposition != RequestDisposition::InProgressAtHorizon;
            outcomes.insert(
                request.request_id.clone(),
                RequestOutcome {
                    request_id: request.request_id.clone(),
                    disposition,
                    fulfilment_mode: Some(assignment.mode),
                    opportunity_id: Some(assignment.opportunity_id.clone()),
                    arrival_seconds: request.arrival_seconds,
                    service_start_seconds: Some(assignment.start_seconds),
                    service_completion_seconds: completed.then_some(assignment.completion_seconds),
                    projected_completion_seconds: Some(assignment.completion_seconds),
                    latency_seconds: completed.then_some(latency),
                    queue_wait_seconds: Some(wait),
                    denial_reason: None,
                },
            );
        } else if let Some(outcome) = outcomes.get(&request.request_id) {
            let _ = outcome;
        } else {
            let reason = if request.arrival_seconds > request.accept_by_seconds {
                "arrival_after_acceptance_cutoff".to_owned()
            } else if input.aerial_opportunities.iter().any(|opportunity| {
                opportunity.start_seconds <= request.deadline_seconds
                    && request.allowed_mode != ServiceMode::FallbackOnly
                    && blocked_aerial_ids.contains(&opportunity.opportunity_id)
            }) {
                "aerial_weather_permission_or_site_blocked".to_owned()
            } else {
                "capacity_unavailable_before_deadline".to_owned()
            };
            outcomes.insert(
                request.request_id.clone(),
                RequestOutcome {
                    request_id: request.request_id.clone(),
                    disposition: RequestDisposition::Unserved,
                    fulfilment_mode: None,
                    opportunity_id: None,
                    arrival_seconds: request.arrival_seconds,
                    service_start_seconds: None,
                    service_completion_seconds: None,
                    projected_completion_seconds: None,
                    latency_seconds: None,
                    queue_wait_seconds: None,
                    denial_reason: Some(reason),
                },
            );
        }
    }

    for outcome in outcomes.values() {
        if let Some(reason) = &outcome.denial_reason {
            *denial_reason_counts.entry(reason.clone()).or_insert(0) += 1;
        }
    }

    let outcomes: Vec<_> = outcomes.into_values().collect();
    let offered_requests = input.requests.len();
    let rejected_requests = outcomes
        .iter()
        .filter(|outcome| outcome.disposition == RequestDisposition::RejectedAfterAcceptanceCutoff)
        .count();
    let accepted_requests = offered_requests - rejected_requests;
    let completed_on_time = outcomes
        .iter()
        .filter(|outcome| outcome.disposition == RequestDisposition::CompletedOnTime)
        .count();
    let completed_late = outcomes
        .iter()
        .filter(|outcome| outcome.disposition == RequestDisposition::CompletedLate)
        .count();
    let in_progress_requests = outcomes
        .iter()
        .filter(|outcome| outcome.disposition == RequestDisposition::InProgressAtHorizon)
        .count();
    let unserved_requests =
        accepted_requests - completed_on_time - completed_late - in_progress_requests;
    let served_by_aerial = outcomes
        .iter()
        .filter(|outcome| {
            outcome.fulfilment_mode == Some(FulfilmentMode::Aerial)
                && matches!(
                    outcome.disposition,
                    RequestDisposition::CompletedOnTime | RequestDisposition::CompletedLate
                )
        })
        .count();
    let served_by_fallback = outcomes
        .iter()
        .filter(|outcome| {
            matches!(
                outcome.fulfilment_mode,
                Some(FulfilmentMode::GroundCourier | FulfilmentMode::Locker | FulfilmentMode::Amr)
            ) && matches!(
                outcome.disposition,
                RequestDisposition::CompletedOnTime | RequestDisposition::CompletedLate
            )
        })
        .count();
    let in_progress_by_aerial = outcomes
        .iter()
        .filter(|outcome| {
            outcome.fulfilment_mode == Some(FulfilmentMode::Aerial)
                && outcome.disposition == RequestDisposition::InProgressAtHorizon
        })
        .count();
    let in_progress_by_fallback = outcomes
        .iter()
        .filter(|outcome| {
            matches!(
                outcome.fulfilment_mode,
                Some(FulfilmentMode::GroundCourier | FulfilmentMode::Locker | FulfilmentMode::Amr)
            ) && outcome.disposition == RequestDisposition::InProgressAtHorizon
        })
        .count();

    let mut latencies: Vec<u64> = outcomes
        .iter()
        .filter_map(|outcome| outcome.latency_seconds)
        .collect();
    let waits: Vec<u64> = outcomes
        .iter()
        .filter_map(|outcome| outcome.queue_wait_seconds)
        .collect();
    let mean_latency_seconds = if latencies.is_empty() {
        None
    } else {
        Some(latencies.iter().map(|value| *value as f64).sum::<f64>() / latencies.len() as f64)
    };
    let mean_queue_wait_seconds = if waits.is_empty() {
        None
    } else {
        Some(waits.iter().map(|value| *value as f64).sum::<f64>() / waits.len() as f64)
    };
    let max_queue_wait_seconds = waits.iter().copied().max().unwrap_or(0);
    let p50_latency_seconds = nearest_rank(&mut latencies, 50);
    let p95_latency_seconds = nearest_rank(&mut latencies, 95);
    let p99_latency_seconds = nearest_rank(&mut latencies, 99);

    let percent = |numerator: usize, denominator: usize| -> f64 {
        if denominator == 0 {
            0.0
        } else {
            numerator as f64 * 100.0 / denominator as f64
        }
    };
    let total_completed = completed_on_time + completed_late;
    let horizon_hours = input.horizon_seconds as f64 / 3_600.0;

    Ok(ServicePolicyResult {
        policy,
        offered_requests,
        accepted_requests,
        rejected_requests,
        completed_on_time,
        completed_late,
        in_progress_requests,
        unserved_requests,
        served_by_aerial,
        served_by_fallback,
        in_progress_by_aerial,
        in_progress_by_fallback,
        aerial_opportunities_available,
        aerial_opportunities_blocked: blocked_aerial_ids.len(),
        aerial_opportunities_unused,
        fallback_opportunities_unused,
        max_waiting_requests_peak,
        completed_requests_per_hour: total_completed as f64 / horizon_hours,
        aerial_opportunity_utilization_percent: percent(
            served_by_aerial + in_progress_by_aerial,
            input.aerial_opportunities.len(),
        ),
        fallback_opportunity_utilization_percent: percent(
            served_by_fallback + in_progress_by_fallback,
            input.fallback_opportunities.len(),
        ),
        fallback_share_of_completed_percent: percent(served_by_fallback, total_completed),
        on_time_rate_of_offered_percent: percent(completed_on_time, offered_requests),
        on_time_rate_of_accepted_percent: percent(completed_on_time, accepted_requests),
        late_rate_of_accepted_percent: percent(completed_late, accepted_requests),
        unserved_rate_of_accepted_percent: percent(unserved_requests, accepted_requests),
        p50_latency_seconds,
        p95_latency_seconds,
        p99_latency_seconds,
        mean_latency_seconds,
        mean_queue_wait_seconds,
        max_queue_wait_seconds,
        denial_reason_counts,
        outcomes,
        claim_ceiling: CLAIM_CEILING,
    })
}

fn exact_score(assignments: &[Option<ExactOracleAssignment>], input: &ServiceQualityInput) -> ExactOracleScore {
    let mut completed_on_time = 0_usize;
    let mut completed_late = 0_usize;
    let mut in_progress_at_horizon = 0_usize;
    let mut total_lateness_seconds = 0_u64;
    let mut total_latency_seconds = 0_u64;

    let request_by_id: BTreeMap<&str, &ServiceRequest> = input
        .requests
        .iter()
        .map(|request| (request.request_id.as_str(), request))
        .collect();

    for assignment in assignments.iter().flatten() {
        let Some(request) = request_by_id.get(assignment.request_id.as_str()) else {
            continue;
        };
        if assignment.projected_completion_seconds > input.horizon_seconds {
            in_progress_at_horizon += 1;
            continue;
        }

        total_latency_seconds = total_latency_seconds
            .saturating_add(
                assignment
                    .projected_completion_seconds
                    .saturating_sub(request.arrival_seconds),
            );
        if assignment.projected_completion_seconds <= request.deadline_seconds {
            completed_on_time += 1;
        } else {
            completed_late += 1;
            total_lateness_seconds = total_lateness_seconds.saturating_add(
                assignment
                    .projected_completion_seconds
                    .saturating_sub(request.deadline_seconds),
            );
        }
    }

    ExactOracleScore {
        completed_on_time,
        completed_late,
        completed_by_horizon: completed_on_time + completed_late,
        in_progress_at_horizon,
        total_lateness_seconds,
        total_latency_seconds,
    }
}

fn exact_score_is_better(
    candidate: &ExactOracleScore,
    candidate_key: &[String],
    current: &ExactOracleScore,
    current_key: &[String],
) -> bool {
    if candidate.completed_on_time != current.completed_on_time {
        return candidate.completed_on_time > current.completed_on_time;
    }
    if candidate.completed_by_horizon != current.completed_by_horizon {
        return candidate.completed_by_horizon > current.completed_by_horizon;
    }
    if candidate.total_lateness_seconds != current.total_lateness_seconds {
        return candidate.total_lateness_seconds < current.total_lateness_seconds;
    }
    if candidate.in_progress_at_horizon != current.in_progress_at_horizon {
        return candidate.in_progress_at_horizon < current.in_progress_at_horizon;
    }
    if candidate.total_latency_seconds != current.total_latency_seconds {
        return candidate.total_latency_seconds < current.total_latency_seconds;
    }
    candidate_key < current_key
}

fn exact_assignment_key(assignments: &[Option<ExactOracleAssignment>]) -> Vec<String> {
    assignments
        .iter()
        .map(|assignment| match assignment {
            Some(item) => format!("{}=>{}", item.opportunity_id, item.request_id),
            None => "~".to_owned(),
        })
        .collect()
}

fn exact_policy_gap(
    policy_result: &ServicePolicyResult,
    oracle: &ExactOracleScore,
    input: &ServiceQualityInput,
) -> ExactOraclePolicyGap {
    let request_by_id: BTreeMap<&str, &ServiceRequest> = input
        .requests
        .iter()
        .map(|request| (request.request_id.as_str(), request))
        .collect();
    let mut policy_lateness_seconds = 0_u64;
    let mut policy_latency_seconds = 0_u64;

    for outcome in &policy_result.outcomes {
        let Some(request) = request_by_id.get(outcome.request_id.as_str()) else {
            continue;
        };
        match outcome.disposition {
            RequestDisposition::CompletedOnTime | RequestDisposition::CompletedLate => {
                if let Some(completion) = outcome.service_completion_seconds {
                    policy_latency_seconds = policy_latency_seconds
                        .saturating_add(completion.saturating_sub(request.arrival_seconds));
                    if outcome.disposition == RequestDisposition::CompletedLate {
                        policy_lateness_seconds = policy_lateness_seconds
                            .saturating_add(completion.saturating_sub(request.deadline_seconds));
                    }
                }
            }
            RequestDisposition::RejectedAfterAcceptanceCutoff
            | RequestDisposition::InProgressAtHorizon
            | RequestDisposition::Unserved => {}
        }
    }

    ExactOraclePolicyGap {
        policy: policy_result.policy,
        on_time_completion_gap: oracle
            .completed_on_time
            .saturating_sub(policy_result.completed_on_time),
        completed_by_horizon_gap: oracle
            .completed_by_horizon
            .saturating_sub(policy_result.completed_on_time + policy_result.completed_late),
        lateness_excess_seconds: policy_lateness_seconds
            .saturating_sub(oracle.total_lateness_seconds),
        latency_excess_seconds: policy_latency_seconds
            .saturating_sub(oracle.total_latency_seconds),
        in_progress_excess: policy_result
            .in_progress_requests
            .saturating_sub(oracle.in_progress_at_horizon),
    }
}

struct ExactOracleSearch<'a> {
    input: &'a ServiceQualityInput,
    opportunities: &'a [ScheduledOpportunity],
    candidates_by_opportunity: &'a [Vec<ExactOracleCandidate>],
    nodes: usize,
    limit_reached: bool,
    current: Vec<Option<ExactOracleAssignment>>,
    used_requests: BTreeSet<usize>,
    best: Option<ExactOracleBest>,
}

impl ExactOracleSearch<'_> {
    fn visit(&mut self, opportunity_index: usize) {
        if self.limit_reached {
            return;
        }
        self.nodes = self.nodes.saturating_add(1);
        if self.nodes > MAX_EXACT_ORACLE_NODES {
            self.limit_reached = true;
            return;
        }

        if opportunity_index == self.opportunities.len() {
            let score = exact_score(&self.current, self.input);
            let key = exact_assignment_key(&self.current);
            let replace = self.best.as_ref().is_none_or(|current| {
                exact_score_is_better(&score, &key, &current.score, &current.canonical_key)
            });
            if replace {
                self.best = Some(ExactOracleBest {
                    score,
                    assignments: self.current.clone(),
                    canonical_key: key,
                });
            }
            return;
        }

        // Leaving a capacity opportunity unused is always a valid choice.
        self.current.push(None);
        self.visit(opportunity_index + 1);
        self.current.pop();

        let candidates = self.candidates_by_opportunity[opportunity_index].clone();
        let opportunity = self.opportunities[opportunity_index].clone();
        for candidate in candidates {
            if !self.used_requests.insert(candidate.request_index) {
                continue;
            }
            let request = &self.input.requests[candidate.request_index];
            self.current.push(Some(ExactOracleAssignment {
                request_id: request.request_id.clone(),
                opportunity_id: opportunity.opportunity_id.clone(),
                fulfilment_mode: candidate.mode,
                service_start_seconds: opportunity.start_seconds,
                projected_completion_seconds: candidate.completion_seconds,
                disposition: if candidate.completion_seconds > self.input.horizon_seconds {
                    RequestDisposition::InProgressAtHorizon
                } else if candidate.completion_seconds <= request.deadline_seconds {
                    RequestDisposition::CompletedOnTime
                } else {
                    RequestDisposition::CompletedLate
                },
            }));
            self.visit(opportunity_index + 1);
            self.current.pop();
            self.used_requests.remove(&candidate.request_index);
            if self.limit_reached {
                return;
            }
        }
    }
}

fn exact_assignment_oracle(
    input: &ServiceQualityInput,
    policy_results: &[ServicePolicyResult],
) -> Result<ExactOracleReport, Vec<String>> {
    let opportunity_count = input.aerial_opportunities.len() + input.fallback_opportunities.len();
    if input.requests.len() > MAX_EXACT_ORACLE_REQUESTS
        || opportunity_count > MAX_EXACT_ORACLE_OPPORTUNITIES
    {
        return Ok(ExactOracleReport {
            status: ExactOracleStatus::NotRunBoundsExceeded,
            request_bound: MAX_EXACT_ORACLE_REQUESTS,
            opportunity_bound: MAX_EXACT_ORACLE_OPPORTUNITIES,
            search_node_limit: MAX_EXACT_ORACLE_NODES,
            search_nodes: 0,
            best_score: None,
            best_assignments: Vec::new(),
            policy_gaps: Vec::new(),
            objective: "Lexicographically maximize on-time completions, then completed services; minimize total lateness, in-progress-at-horizon assignments and total completed-request latency; finally choose the canonical opportunity/request assignment.",
        });
    }

    let mut opportunities = Vec::with_capacity(opportunity_count);
    for (index, opportunity) in input.aerial_opportunities.iter().enumerate() {
        opportunities.push(ScheduledOpportunity {
            start_seconds: opportunity.start_seconds,
            is_aerial: true,
            index,
            opportunity_id: opportunity.opportunity_id.clone(),
        });
    }
    for (index, opportunity) in input.fallback_opportunities.iter().enumerate() {
        opportunities.push(ScheduledOpportunity {
            start_seconds: opportunity.start_seconds,
            is_aerial: false,
            index,
            opportunity_id: opportunity.opportunity_id.clone(),
        });
    }
    opportunities.sort_by_key(|opportunity| {
        (
            opportunity.start_seconds,
            if opportunity.is_aerial { 0_u8 } else { 1_u8 },
            opportunity.opportunity_id.clone(),
        )
    });

    let mut candidates_by_opportunity = Vec::with_capacity(opportunities.len());
    for opportunity in &opportunities {
        let mut candidates = Vec::new();
        for (request_index, request) in input.requests.iter().enumerate() {
            if request.arrival_seconds > request.accept_by_seconds
                || request.arrival_seconds > opportunity.start_seconds
                || opportunity.start_seconds > request.deadline_seconds
                || (opportunity.is_aerial
                    && !matches!(
                        request.allowed_mode,
                        ServiceMode::AerialOnly | ServiceMode::Either
                    ))
                || (!opportunity.is_aerial
                    && !matches!(
                        request.allowed_mode,
                        ServiceMode::FallbackOnly | ServiceMode::Either
                    ))
            {
                continue;
            }

            let (max_duration, max_payload, mode) = if opportunity.is_aerial {
                let aerial = &input.aerial_opportunities[opportunity.index];
                (
                    aerial.max_service_duration_seconds,
                    aerial.max_payload_units,
                    FulfilmentMode::Aerial,
                )
            } else {
                let fallback = &input.fallback_opportunities[opportunity.index];
                (
                    fallback.max_service_duration_seconds,
                    fallback.max_payload_units,
                    mode_for_fallback(fallback.fallback_mode),
                )
            };
            if max_duration < request.required_service_seconds
                || max_payload < request.payload_units
            {
                continue;
            }
            if opportunity.is_aerial {
                let aerial = &input.aerial_opportunities[opportunity.index];
                match aerial_block_reason(input, aerial, request.required_service_seconds) {
                    Ok(None) => {}
                    Ok(Some(_)) => continue,
                    Err(error) => return Err(vec![error]),
                }
            }
            let completion_seconds = opportunity
                .start_seconds
                .checked_add(u64::from(request.required_service_seconds))
                .ok_or_else(|| vec!["exact_oracle_service_completion_overflow".to_owned()])?;
            candidates.push(ExactOracleCandidate {
                request_index,
                completion_seconds,
                mode,
            });
        }
        candidates.sort_by_key(|candidate| {
            (
                input.requests[candidate.request_index].request_id.clone(),
                candidate.request_index,
            )
        });
        candidates_by_opportunity.push(candidates);
    }

    let mut search = ExactOracleSearch {
        input,
        opportunities: &opportunities,
        candidates_by_opportunity: &candidates_by_opportunity,
        nodes: 0,
        limit_reached: false,
        current: Vec::with_capacity(opportunities.len()),
        used_requests: BTreeSet::new(),
        best: None,
    };
    search.visit(0);

    if search.limit_reached {
        return Ok(ExactOracleReport {
            status: ExactOracleStatus::NodeLimitReached,
            request_bound: MAX_EXACT_ORACLE_REQUESTS,
            opportunity_bound: MAX_EXACT_ORACLE_OPPORTUNITIES,
            search_node_limit: MAX_EXACT_ORACLE_NODES,
            search_nodes: search.nodes,
            best_score: None,
            best_assignments: Vec::new(),
            policy_gaps: Vec::new(),
            objective: "Lexicographically maximize on-time completions, then completed services; minimize total lateness, in-progress-at-horizon assignments and total completed-request latency; finally choose the canonical opportunity/request assignment.",
        });
    }

    let best = search
        .best
        .ok_or_else(|| vec!["exact_oracle_failed_to_emit_empty_assignment".to_owned()])?;
    let policy_gaps = policy_results
        .iter()
        .map(|policy| exact_policy_gap(policy, &best.score, input))
        .collect();

    Ok(ExactOracleReport {
        status: ExactOracleStatus::Evaluated,
        request_bound: MAX_EXACT_ORACLE_REQUESTS,
        opportunity_bound: MAX_EXACT_ORACLE_OPPORTUNITIES,
        search_node_limit: MAX_EXACT_ORACLE_NODES,
        search_nodes: search.nodes,
        best_score: Some(best.score),
        best_assignments: best.assignments.into_iter().flatten().collect(),
        policy_gaps,
        objective: "Lexicographically maximize on-time completions, then completed services; minimize total lateness, in-progress-at-horizon assignments and total completed-request latency; finally choose the canonical opportunity/request assignment.",
    })
}

fn sha256_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        write!(&mut hex, "{byte:02x}").expect("writing to String cannot fail");
    }
    format!("sha256:{hex}")
}

fn linked_report_claim_ceiling() -> &'static str {
    "Synthetic request admission over event-derived capacity receipts and separately supplied payload/weather/permission evidence only; no route planning, flight authorization, physical service, market or regulatory authority."
}

/// Re-run the upstream capacity evaluator and resolve aerial opportunities
/// exclusively from its event-derived receipt sequence. This mode deliberately
/// accepts no user-authored aircraft IDs, start times, mission durations or
/// turnaround values.
pub fn evaluate_linked_service_quality(
    input: &LinkedServiceQualityInput,
    capacity_input: &SustainableServiceCapacityInput,
) -> Result<LinkedServiceQualityReport, Vec<String>> {
    let mut errors = Vec::new();
    if input.schema_version != "0.1.0"
        || input.benchmark_id != "SUSTAINABLE-MOBILITY-SERVICE-QUALITY-001"
        || input.site_id.trim().is_empty()
        || input.site_configuration_digest.trim().is_empty()
        || input.source_architecture_id.trim().is_empty()
        || input.horizon_seconds == 0
        || input.horizon_seconds > MAX_HORIZON_SECONDS
        || input.claim_ceiling.trim().is_empty()
        || input.aerial_opportunity_qualifications.len() > MAX_AERIAL_OPPORTUNITIES
        || input.requests.is_empty()
        || input.requests.len() > MAX_REQUESTS
        || input.fallback_opportunities.len() > MAX_FALLBACK_OPPORTUNITIES
        || input.flight_windows.is_empty()
        || input.flight_windows.len() > MAX_FLIGHT_WINDOWS
        || input.blocked_intervals.len() > MAX_BLOCKED_WINDOWS
        || input.policies.is_empty()
        || input.policies.len() > 2
        || capacity_input.horizon_seconds != input.horizon_seconds
    {
        add_error(&mut errors, "linked_service_identity_or_bounds_invalid");
    }

    let capacity_report =
        evaluate_sustainable_service_capacity(capacity_input).map_err(|capacity_errors| {
            capacity_errors
                .into_iter()
                .map(|error| format!("capacity_source_rejected:{error}"))
                .collect::<Vec<_>>()
        })?;
    if capacity_report.schema_version != "0.1.0"
        || capacity_report.benchmark_id != "SUSTAINABLE-MOBILITY-SERVICE-CAPACITY-001"
        || capacity_report.horizon_seconds != input.horizon_seconds
        || capacity_report.capacity_is_physical_performance_claim
    {
        add_error(
            &mut errors,
            "capacity_source_report_identity_or_horizon_invalid",
        );
    }

    let selected: Vec<_> = capacity_report
        .architecture_results
        .iter()
        .filter(|result| result.architecture_id == input.source_architecture_id)
        .collect();
    if selected.len() != 1 {
        add_error(
            &mut errors,
            "capacity_source_architecture_missing_or_ambiguous",
        );
    }
    let architectures: Vec<_> = capacity_input
        .architectures
        .iter()
        .filter(|architecture| architecture.architecture_id == input.source_architecture_id)
        .collect();
    if architectures.len() != 1 {
        add_error(
            &mut errors,
            "capacity_source_configuration_missing_or_ambiguous",
        );
    }
    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let selected = selected[0];
    let architecture = architectures[0];
    if selected.horizon_seconds != input.horizon_seconds
        || selected.service_opportunities.len() as u64 != selected.completed_sorties
        || selected.service_opportunities.len() > MAX_AERIAL_OPPORTUNITIES
    {
        add_error(
            &mut errors,
            "capacity_service_opportunity_count_or_horizon_invalid",
        );
    }

    let source_report_bytes = serde_json::to_vec(&capacity_report)
        .map_err(|error| vec![format!("capacity_report_serialization_failed:{error}")])?;
    let source_report_digest = sha256_digest(&source_report_bytes);
    let configuration_bytes = serde_json::to_vec(architecture).map_err(|error| {
        vec![format!(
            "capacity_configuration_serialization_failed:{error}"
        )]
    })?;
    let configuration_digest = sha256_digest(&configuration_bytes);
    let opportunity_bytes = serde_json::to_vec(&selected.service_opportunities)
        .map_err(|error| vec![format!("capacity_opportunity_serialization_failed:{error}")])?;
    let opportunity_digest = sha256_digest(&opportunity_bytes);
    if opportunity_digest != selected.service_opportunity_digest {
        add_error(&mut errors, "capacity_service_opportunity_digest_mismatch");
    }

    let mut source_ids = BTreeSet::new();
    let mut source_sequences: BTreeMap<&str, Vec<&CapacityServiceOpportunity>> = BTreeMap::new();
    for opportunity in &selected.service_opportunities {
        let mission_end = opportunity
            .start_seconds
            .checked_add(u64::from(opportunity.mission_duration_seconds));
        if opportunity.opportunity_id.trim().is_empty()
            || opportunity.aircraft_id.trim().is_empty()
            || !source_ids.insert(opportunity.opportunity_id.as_str())
            || opportunity.mission_duration_seconds == 0
            || opportunity.turnaround_seconds > MAX_SERVICE_SECONDS
            || mission_end.is_none_or(|end| end > input.horizon_seconds)
        {
            add_error(
                &mut errors,
                format!(
                    "capacity_receipt_identity_or_bounds_invalid:{}",
                    opportunity.opportunity_id
                ),
            );
        }
        source_sequences
            .entry(opportunity.aircraft_id.as_str())
            .or_default()
            .push(opportunity);
    }
    for (aircraft_id, opportunities) in &mut source_sequences {
        opportunities.sort_by_key(|item| (item.mission_sequence, item.start_seconds));
        for (index, opportunity) in opportunities.iter().enumerate() {
            if usize::try_from(opportunity.mission_sequence).ok() != Some(index) {
                add_error(
                    &mut errors,
                    format!("capacity_receipt_sequence_gap:{aircraft_id}"),
                );
            }
        }
        for pair in opportunities.windows(2) {
            let next_earliest = pair[0]
                .start_seconds
                .checked_add(u64::from(pair[0].mission_duration_seconds))
                .and_then(|end| end.checked_add(u64::from(pair[0].turnaround_seconds)));
            if next_earliest.is_none_or(|earliest| pair[1].start_seconds < earliest) {
                add_error(
                    &mut errors,
                    format!("capacity_receipts_overlap_on_aircraft:{aircraft_id}"),
                );
            }
        }
    }

    let mut qualification_ids = BTreeSet::new();
    let mut payload_evidence_references = BTreeSet::new();
    for qualification in &input.aerial_opportunity_qualifications {
        let Some(receipt) = selected
            .service_opportunities
            .iter()
            .find(|receipt| receipt.opportunity_id == qualification.opportunity_id)
        else {
            add_error(
                &mut errors,
                format!(
                    "aerial_qualification_references_unknown_capacity_opportunity:{}",
                    qualification.opportunity_id
                ),
            );
            continue;
        };
        let evidence = &qualification.evidence;
        let occupancy_end = receipt
            .start_seconds
            .checked_add(u64::from(receipt.mission_duration_seconds))
            .and_then(|end| end.checked_add(u64::from(receipt.turnaround_seconds)));
        if qualification.opportunity_id.trim().is_empty()
            || !qualification_ids.insert(qualification.opportunity_id.as_str())
            || qualification.max_payload_units == 0
            || qualification.max_payload_units > MAX_PAYLOAD_UNITS
            || evidence.capacity_opportunity_id != receipt.opportunity_id
            || evidence.aircraft_id != receipt.aircraft_id
            || evidence.site_id != input.site_id
            || evidence.site_configuration_digest != input.site_configuration_digest
            || evidence.architecture_configuration_digest != configuration_digest
            || evidence.producer_id.trim().is_empty()
            || evidence.method_id.trim().is_empty()
            || evidence.evidence_reference.trim().is_empty()
            || !payload_evidence_references.insert(evidence.evidence_reference.as_str())
            || evidence.evidence_class == EvidenceClass::Unknown
            || evidence.valid_from_seconds > evidence.observed_at_seconds
            || evidence.observed_at_seconds > receipt.start_seconds
            || evidence.valid_from_seconds > receipt.start_seconds
            || occupancy_end.is_none_or(|end| evidence.valid_until_seconds < end)
        {
            add_error(
                &mut errors,
                format!(
                    "aerial_payload_qualification_invalid:{}",
                    qualification.opportunity_id
                ),
            );
        }
    }

    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let mut aerial_opportunities =
        Vec::with_capacity(input.aerial_opportunity_qualifications.len());
    for qualification in &input.aerial_opportunity_qualifications {
        let receipt = selected
            .service_opportunities
            .iter()
            .find(|receipt| receipt.opportunity_id == qualification.opportunity_id)
            .expect("all qualifications were resolved above");
        aerial_opportunities.push(AerialServiceOpportunity {
            opportunity_id: receipt.opportunity_id.clone(),
            aircraft_id: receipt.aircraft_id.clone(),
            start_seconds: receipt.start_seconds,
            max_service_duration_seconds: receipt.mission_duration_seconds,
            max_payload_units: qualification.max_payload_units,
            turnaround_seconds: receipt.turnaround_seconds,
            capacity_reference: format!("{source_report_digest}#{}", receipt.opportunity_id),
        });
    }
    aerial_opportunities.sort_by_key(|opportunity| {
        (
            opportunity.start_seconds,
            opportunity.aircraft_id.clone(),
            opportunity.opportunity_id.clone(),
        )
    });

    let qualified_opportunity_count = aerial_opportunities.len();
    let legacy_input = ServiceQualityInput {
        schema_version: input.schema_version.clone(),
        benchmark_id: input.benchmark_id.clone(),
        site_id: input.site_id.clone(),
        site_configuration_digest: input.site_configuration_digest.clone(),
        horizon_seconds: input.horizon_seconds,
        capacity_source: CapacitySource {
            source_report_digest: source_report_digest.clone(),
            source_architecture_id: input.source_architecture_id.clone(),
            horizon_seconds: selected.horizon_seconds,
            steady_state_cycle_capacity_upper_bound_per_hour: selected
                .steady_state_cycle_capacity_upper_bound_per_hour,
            initial_inventory_capacity_allowance_per_hour: selected
                .initial_inventory_capacity_allowance_per_hour,
            max_aerial_opportunities: u32::try_from(qualified_opportunity_count)
                .map_err(|_| vec!["qualified_opportunity_count_overflow".to_owned()])?,
        },
        requests: input.requests.clone(),
        aerial_opportunities,
        fallback_opportunities: input.fallback_opportunities.clone(),
        flight_windows: input.flight_windows.clone(),
        blocked_intervals: input.blocked_intervals.clone(),
        policies: input.policies.clone(),
        claim_ceiling: input.claim_ceiling.clone(),
    };
    let evaluated = evaluate_service_quality(&legacy_input).map_err(|errors| errors)?;
    let exact_oracle =
        exact_assignment_oracle(&legacy_input, &evaluated.policy_results)?;

    Ok(LinkedServiceQualityReport {
        schema_version: input.schema_version.clone(),
        benchmark_id: input.benchmark_id.clone(),
        horizon_seconds: input.horizon_seconds,
        capacity_source_architecture_id: input.source_architecture_id.clone(),
        capacity_source_report_digest: source_report_digest,
        capacity_service_opportunity_digest: opportunity_digest,
        capacity_architecture_configuration_digest: configuration_digest,
        capacity_opportunities_in_source: selected.service_opportunities.len(),
        capacity_opportunities_qualified: qualified_opportunity_count,
        policy_results: evaluated.policy_results,
        exact_oracle,
        service_quality_is_physical_performance_claim: false,
        claim_ceiling: linked_report_claim_ceiling(),
    })
}

pub fn evaluate_service_quality(
    input: &ServiceQualityInput,
) -> Result<ServiceQualityReport, Vec<String>> {
    validate_input(input)?;
    let mut policy_results = Vec::new();
    for policy in &input.policies {
        match evaluate_policy(input, *policy) {
            Ok(result) => policy_results.push(result),
            Err(errors) => return Err(errors),
        }
    }
    policy_results.sort_by_key(|result| match result.policy {
        AdmissionPolicy::FirstComeFirstServed => 0_u8,
        AdmissionPolicy::EarliestDeadlineFirst => 1_u8,
    });
    Ok(ServiceQualityReport {
        schema_version: input.schema_version.clone(),
        benchmark_id: input.benchmark_id.clone(),
        horizon_seconds: input.horizon_seconds,
        capacity_source_architecture_id: input.capacity_source.source_architecture_id.clone(),
        capacity_source_report_digest: input.capacity_source.source_report_digest.clone(),
        policy_results,
        service_quality_is_physical_performance_claim: false,
        claim_ceiling: CLAIM_CEILING,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ServiceQualityInput {
        serde_json::from_str(include_str!(
            "../fixtures/sustainable_mobility_service_quality_v0_1.json"
        ))
        .expect("frozen request stream fixture must parse")
    }

    fn linked_fixture() -> LinkedServiceQualityInput {
        serde_json::from_str(include_str!(
            "../fixtures/sustainable_mobility_service_quality_linked_v0_1.json"
        ))
        .expect("frozen linked request fixture must parse")
    }

    fn capacity_fixture() -> SustainableServiceCapacityInput {
        serde_json::from_str(include_str!(
            "../fixtures/sustainable_mobility_service_capacity_v0_1.json"
        ))
        .expect("frozen capacity source fixture must parse")
    }

    fn linked_report() -> LinkedServiceQualityReport {
        evaluate_linked_service_quality(&linked_fixture(), &capacity_fixture())
            .expect("linked capacity receipt fixture")
    }

    fn evaluate(policy: AdmissionPolicy) -> ServicePolicyResult {
        let mut input = fixture();
        input.policies = vec![policy];
        evaluate_service_quality(&input)
            .expect("fixture must evaluate")
            .policy_results
            .remove(0)
    }

    #[test]
    fn fcfs_uses_arrival_order_not_deadline_order_for_same_arrival() {
        let mut input = fixture();
        input.requests = vec![
            ServiceRequest {
                request_id: "a-later-deadline".to_owned(),
                arrival_seconds: 0,
                accept_by_seconds: 0,
                deadline_seconds: 2_000,
                required_service_seconds: 600,
                payload_units: 1,
                service_class: "standard".to_owned(),
                allowed_mode: ServiceMode::AerialOnly,
            },
            ServiceRequest {
                request_id: "z-earlier-deadline".to_owned(),
                arrival_seconds: 0,
                accept_by_seconds: 0,
                deadline_seconds: 500,
                required_service_seconds: 600,
                payload_units: 1,
                service_class: "urgent".to_owned(),
                allowed_mode: ServiceMode::AerialOnly,
            },
        ];
        input.aerial_opportunities = vec![
            AerialServiceOpportunity {
                opportunity_id: "slot-0".to_owned(),
                aircraft_id: "aircraft-01".to_owned(),
                start_seconds: 0,
                max_service_duration_seconds: 600,
                max_payload_units: 100,
                turnaround_seconds: 45,
                capacity_reference: "synthetic://slot-0".to_owned(),
            },
            AerialServiceOpportunity {
                opportunity_id: "slot-1".to_owned(),
                aircraft_id: "aircraft-01".to_owned(),
                start_seconds: 700,
                max_service_duration_seconds: 600,
                max_payload_units: 100,
                turnaround_seconds: 45,
                capacity_reference: "synthetic://slot-1".to_owned(),
            },
        ];
        input.fallback_opportunities.clear();
        input.capacity_source.max_aerial_opportunities = 2;

        let fcfs = {
            input.policies = vec![AdmissionPolicy::FirstComeFirstServed];
            evaluate_service_quality(&input)
                .expect("FCFS")
                .policy_results
                .remove(0)
        };
        let edf = {
            input.policies = vec![AdmissionPolicy::EarliestDeadlineFirst];
            evaluate_service_quality(&input)
                .expect("EDF")
                .policy_results
                .remove(0)
        };
        assert_eq!(fcfs.outcomes[0].request_id, "a-later-deadline");
        assert_eq!(fcfs.outcomes[0].opportunity_id.as_deref(), Some("slot-0"));
        assert_eq!(edf.outcomes[0].request_id, "a-later-deadline");
        assert_eq!(edf.outcomes[1].request_id, "z-earlier-deadline");
        assert_eq!(edf.outcomes[1].opportunity_id.as_deref(), Some("slot-0"));
    }

    #[test]
    fn linked_fixture_uses_event_derived_capacity_and_reports_bound_digests() {
        let report = linked_report();
        assert_eq!(
            report.capacity_source_architecture_id,
            "fixed_dock4_reference_profile"
        );
        assert_eq!(report.capacity_opportunities_in_source, 38);
        assert_eq!(report.capacity_opportunities_qualified, 7);
        assert!(report.capacity_source_report_digest.starts_with("sha256:"));
        assert_eq!(report.capacity_source_report_digest.len(), 71);
        assert!(
            report
                .capacity_service_opportunity_digest
                .starts_with("sha256:")
        );
        assert_eq!(
            report.capacity_architecture_configuration_digest,
            "sha256:090dc6e97d0cf64936d64749e05a5041a474eb3fe8712fb1247eeaaf09db8024"
        );
        assert_eq!(report.policy_results.len(), 2);
        assert_eq!(report.policy_results[0].completed_on_time, 6);
        assert_eq!(report.policy_results[0].completed_late, 2);
        assert_eq!(report.policy_results[0].unserved_requests, 3);
        assert_eq!(report.policy_results[1].completed_on_time, 7);
        assert_eq!(report.policy_results[1].completed_late, 1);
        assert_eq!(report.policy_results[1].unserved_requests, 3);
    }

    #[test]
    fn linked_fcfs_and_edf_consume_only_source_receipt_ids() {
        let report = linked_report();
        let fcfs = &report.policy_results[0];
        let edf = &report.policy_results[1];
        let fcfs_first = fcfs
            .outcomes
            .iter()
            .find(|item| item.request_id == "request-first-arrival")
            .unwrap();
        let fcfs_urgent = fcfs
            .outcomes
            .iter()
            .find(|item| item.request_id == "request-tight-deadline")
            .unwrap();
        let edf_first = edf
            .outcomes
            .iter()
            .find(|item| item.request_id == "request-first-arrival")
            .unwrap();
        let edf_urgent = edf
            .outcomes
            .iter()
            .find(|item| item.request_id == "request-tight-deadline")
            .unwrap();
        assert_eq!(
            fcfs_first.opportunity_id.as_deref(),
            Some("aircraft-00-mission-000000")
        );
        assert_eq!(
            fcfs_urgent.opportunity_id.as_deref(),
            Some("aircraft-00-mission-000001")
        );
        assert_eq!(fcfs_first.service_start_seconds, Some(0));
        assert_eq!(fcfs_urgent.service_start_seconds, Some(345));
        assert_eq!(
            edf_urgent.opportunity_id.as_deref(),
            Some("aircraft-00-mission-000000")
        );
        assert_eq!(
            edf_first.opportunity_id.as_deref(),
            Some("aircraft-00-mission-000001")
        );
    }

    #[test]
    fn unknown_capacity_receipt_id_is_rejected() {
        let mut input = linked_fixture();
        input.aerial_opportunity_qualifications[0].opportunity_id =
            "aircraft-00-mission-999999".to_owned();
        let errors = evaluate_linked_service_quality(&input, &capacity_fixture())
            .expect_err("request input cannot invent a capacity opportunity");
        assert!(errors.iter().any(|error| {
            error.contains("aerial_qualification_references_unknown_capacity_opportunity")
        }));
    }

    #[test]
    fn payload_qualification_must_match_exact_aircraft_and_configuration() {
        let input = linked_fixture();
        let capacity = capacity_fixture();
        let mut wrong_aircraft = linked_fixture();
        wrong_aircraft.aerial_opportunity_qualifications[0]
            .evidence
            .aircraft_id = "aircraft-99".to_owned();
        let errors = evaluate_linked_service_quality(&wrong_aircraft, &capacity)
            .expect_err("payload qualification is aircraft-bound");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("aerial_payload_qualification_invalid"))
        );

        let mut wrong_configuration = input;
        wrong_configuration.aerial_opportunity_qualifications[0]
            .evidence
            .architecture_configuration_digest = "sha256:stale-configuration".to_owned();
        let errors = evaluate_linked_service_quality(&wrong_configuration, &capacity)
            .expect_err("payload qualification is configuration-bound");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("aerial_payload_qualification_invalid"))
        );
    }

    #[test]
    fn payload_qualification_must_cover_entire_receipt_occupancy() {
        let mut input = linked_fixture();
        let receipt = "aircraft-00-mission-000021";
        let qualification = input
            .aerial_opportunity_qualifications
            .iter_mut()
            .find(|item| item.opportunity_id == receipt)
            .expect("late receipt qualification");
        qualification.evidence.valid_until_seconds = 15_500;
        let errors = evaluate_linked_service_quality(&input, &capacity_fixture())
            .expect_err("evidence expires before mission plus turnaround ends");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("aerial_payload_qualification_invalid"))
        );
    }

    #[test]
    fn payload_evidence_is_bound_to_the_exact_capacity_opportunity_id() {
        let mut input = linked_fixture();
        input.aerial_opportunity_qualifications[1]
            .evidence
            .capacity_opportunity_id = input.aerial_opportunity_qualifications[0]
            .opportunity_id
            .clone();
        let errors = evaluate_linked_service_quality(&input, &capacity_fixture())
            .expect_err("evidence receipt must name the exact source opportunity");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("aerial_payload_qualification_invalid"))
        );
    }

    #[test]
    fn payload_evidence_is_bound_to_the_exact_site_configuration() {
        let mut input = linked_fixture();
        input.aerial_opportunity_qualifications[0].evidence.site_id = "another-site".to_owned();
        let errors = evaluate_linked_service_quality(&input, &capacity_fixture())
            .expect_err("payload qualification cannot be replayed at another site");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("aerial_payload_qualification_invalid"))
        );

        let mut input = linked_fixture();
        input.aerial_opportunity_qualifications[0]
            .evidence
            .site_configuration_digest = "sha256:another-site-config".to_owned();
        let errors = evaluate_linked_service_quality(&input, &capacity_fixture())
            .expect_err("payload qualification cannot cross site configurations");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("aerial_payload_qualification_invalid"))
        );
    }

    #[test]
    fn payload_evidence_reference_cannot_be_replayed_for_another_opportunity() {
        let mut input = linked_fixture();
        input.aerial_opportunity_qualifications[1]
            .evidence
            .evidence_reference = input.aerial_opportunity_qualifications[0]
            .evidence
            .evidence_reference
            .clone();
        let errors = evaluate_linked_service_quality(&input, &capacity_fixture())
            .expect_err("one payload receipt cannot qualify two service opportunities");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("aerial_payload_qualification_invalid"))
        );
    }

    #[test]
    fn architecture_or_horizon_mismatch_is_rejected() {
        let mut input = linked_fixture();
        input.source_architecture_id = "unmodelled-architecture".to_owned();
        let errors = evaluate_linked_service_quality(&input, &capacity_fixture())
            .expect_err("unknown architecture");
        assert!(
            errors
                .iter()
                .any(|error| { error == "capacity_source_architecture_missing_or_ambiguous" })
        );

        let mut input = linked_fixture();
        input.horizon_seconds -= 1;
        let errors = evaluate_linked_service_quality(&input, &capacity_fixture())
            .expect_err("capacity and request horizons must be identical");
        assert!(
            errors
                .iter()
                .any(|error| error == "linked_service_identity_or_bounds_invalid")
        );
    }

    #[test]
    fn linked_output_is_invariant_to_request_and_qualification_input_order() {
        let input = linked_fixture();
        let first = evaluate_linked_service_quality(&input, &capacity_fixture())
            .expect("first linked report");
        let mut permuted = linked_fixture();
        permuted.requests.reverse();
        permuted.aerial_opportunity_qualifications.reverse();
        permuted.fallback_opportunities.reverse();
        permuted.flight_windows.reverse();
        permuted.blocked_intervals.reverse();
        permuted.policies.reverse();
        let second = evaluate_linked_service_quality(&permuted, &capacity_fixture())
            .expect("permuted linked report");
        assert_eq!(
            serde_json::to_string(&first).expect("serialize first"),
            serde_json::to_string(&second).expect("serialize second")
        );
    }

    #[test]
    fn linked_capacity_mutation_changes_configuration_digest_and_invalidates_old_qualification() {
        let input = linked_fixture();
        let mut capacity = capacity_fixture();
        capacity.architectures[1].charge_duration_seconds += 60;
        let errors = evaluate_linked_service_quality(&input, &capacity)
            .expect_err("changing the architecture requires new payload qualification evidence");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("aerial_payload_qualification_invalid"))
        );
    }

    #[test]
    fn fixture_reports_both_online_admission_policies() {
        let report = evaluate_service_quality(&fixture()).expect("frozen fixture");
        assert_eq!(report.policy_results.len(), 2);
        assert_eq!(
            report.policy_results[0].policy,
            AdmissionPolicy::FirstComeFirstServed
        );
        assert_eq!(
            report.policy_results[1].policy,
            AdmissionPolicy::EarliestDeadlineFirst
        );
        for result in report.policy_results {
            assert_eq!(
                result.completed_on_time + result.completed_late + result.unserved_requests,
                result.accepted_requests
            );
            assert_eq!(
                result.served_by_aerial + result.served_by_fallback,
                result.completed_on_time + result.completed_late
            );
        }
    }

    #[test]
    fn earliest_deadline_first_serves_tight_deadline_before_longer_job() {
        let mut input = fixture();
        input.requests = vec![
            ServiceRequest {
                request_id: "long".to_owned(),
                arrival_seconds: 0,
                accept_by_seconds: 0,
                deadline_seconds: 5_000,
                required_service_seconds: 600,
                payload_units: 1,
                service_class: "standard".to_owned(),
                allowed_mode: ServiceMode::AerialOnly,
            },
            ServiceRequest {
                request_id: "urgent".to_owned(),
                arrival_seconds: 30,
                accept_by_seconds: 30,
                deadline_seconds: 2_000,
                required_service_seconds: 600,
                payload_units: 1,
                service_class: "urgent".to_owned(),
                allowed_mode: ServiceMode::AerialOnly,
            },
        ];
        input.aerial_opportunities = vec![
            AerialServiceOpportunity {
                opportunity_id: "first".to_owned(),
                aircraft_id: "a1".to_owned(),
                start_seconds: 100,
                max_service_duration_seconds: 600,
                max_payload_units: 100,
                turnaround_seconds: 100,
                capacity_reference: "synthetic://capacity/first".to_owned(),
            },
            AerialServiceOpportunity {
                opportunity_id: "second".to_owned(),
                aircraft_id: "a1".to_owned(),
                start_seconds: 900,
                max_service_duration_seconds: 600,
                max_payload_units: 100,
                turnaround_seconds: 100,
                capacity_reference: "synthetic://capacity/second".to_owned(),
            },
        ];
        input.capacity_source.max_aerial_opportunities = 2;
        input.fallback_opportunities.clear();

        let fcfs = {
            input.policies = vec![AdmissionPolicy::FirstComeFirstServed];
            evaluate_service_quality(&input)
                .expect("FCFS")
                .policy_results
                .remove(0)
        };
        let edf = {
            input.policies = vec![AdmissionPolicy::EarliestDeadlineFirst];
            evaluate_service_quality(&input)
                .expect("EDF")
                .policy_results
                .remove(0)
        };
        let first_fcfs = fcfs
            .outcomes
            .iter()
            .find(|o| o.service_start_seconds == Some(100))
            .unwrap();
        let first_edf = edf
            .outcomes
            .iter()
            .find(|o| o.service_start_seconds == Some(100))
            .unwrap();
        assert_eq!(first_fcfs.request_id, "long");
        assert_eq!(first_edf.request_id, "urgent");
    }

    #[test]
    fn unknown_permission_fails_closed_but_fallback_can_serve_request() {
        let mut input = fixture();
        input.requests = vec![ServiceRequest {
            request_id: "fallback".to_owned(),
            arrival_seconds: 0,
            accept_by_seconds: 0,
            deadline_seconds: 2_000,
            required_service_seconds: 600,
            payload_units: 1,
            service_class: "standard".to_owned(),
            allowed_mode: ServiceMode::Either,
        }];
        input.flight_windows[0].permission_state = PermissionState::Unknown;
        input.aerial_opportunities.truncate(1);
        input.fallback_opportunities.truncate(1);
        input.capacity_source.max_aerial_opportunities = 1;
        let result = evaluate_service_quality(&input).expect("fallback service");
        let outcome = &result.policy_results[0].outcomes[0];
        assert_eq!(outcome.fulfilment_mode, Some(FulfilmentMode::GroundCourier));
    }

    #[test]
    fn stale_weather_or_permission_evidence_is_rejected() {
        let mut input = fixture();
        input.flight_windows[0].evidence.valid_until_seconds = 1;
        let errors = evaluate_service_quality(&input).expect_err("window evidence expires early");
        assert!(
            errors
                .iter()
                .any(|error| error.starts_with("flight_window_identity_or_evidence_invalid"))
        );
    }

    #[test]
    fn future_weather_observation_is_rejected_as_information_leakage() {
        let mut input = fixture();
        input.flight_windows[0].evidence.observed_at_seconds =
            input.flight_windows[0].start_seconds + 1;
        let errors =
            evaluate_service_quality(&input).expect_err("future observation cannot be known");
        assert!(
            errors
                .iter()
                .any(|error| error.starts_with("flight_window_identity_or_evidence_invalid"))
        );
    }

    #[test]
    fn maintenance_interval_blocks_aerial_but_not_fallback() {
        let mut input = fixture();
        input.blocked_intervals = vec![BlockedInterval {
            interval_id: "maintenance".to_owned(),
            start_seconds: 0,
            end_seconds: 3_000,
            kind: BlockedWindowKind::Maintenance,
        }];
        input.requests = vec![ServiceRequest {
            request_id: "maintenance-test".to_owned(),
            arrival_seconds: 0,
            accept_by_seconds: 0,
            deadline_seconds: 2_000,
            required_service_seconds: 600,
            payload_units: 1,
            service_class: "standard".to_owned(),
            allowed_mode: ServiceMode::Either,
        }];
        input.aerial_opportunities.truncate(1);
        input.fallback_opportunities.truncate(1);
        input.capacity_source.max_aerial_opportunities = 1;
        let result = evaluate_service_quality(&input).expect("blocked aerial / fallback");
        assert_eq!(
            result.policy_results[0].outcomes[0].fulfilment_mode,
            Some(FulfilmentMode::GroundCourier)
        );
    }

    #[test]
    fn acceptance_cutoff_rejects_late_arrival() {
        let mut input = fixture();
        input.requests = vec![ServiceRequest {
            request_id: "late-arrival".to_owned(),
            arrival_seconds: 101,
            accept_by_seconds: 100,
            deadline_seconds: 500,
            required_service_seconds: 100,
            payload_units: 1,
            service_class: "standard".to_owned(),
            allowed_mode: ServiceMode::Either,
        }];
        input.aerial_opportunities.clear();
        input.fallback_opportunities.clear();
        let result = evaluate_service_quality(&input).expect("late request is a valid rejection");
        assert_eq!(
            result.policy_results[0].outcomes[0].disposition,
            RequestDisposition::RejectedAfterAcceptanceCutoff
        );
    }

    #[test]
    fn late_completion_is_counted_separately_from_unserved_requests() {
        let report = evaluate_service_quality(&fixture()).expect("frozen fixture");
        for policy in report.policy_results {
            assert_eq!(policy.completed_late, 1);
            assert!(policy.outcomes.iter().any(|outcome| {
                outcome.request_id == "request-late-completion"
                    && outcome.disposition == RequestDisposition::CompletedLate
                    && outcome.service_start_seconds == Some(15_000)
                    && outcome.service_completion_seconds == Some(15_600)
            }));
            assert_eq!(
                policy.completed_on_time
                    + policy.completed_late
                    + policy.in_progress_requests
                    + policy.unserved_requests,
                policy.accepted_requests
            );
        }
    }

    #[test]
    fn fallback_service_crossing_the_horizon_is_in_progress_not_completed() {
        let mut input = fixture();
        input.requests = vec![ServiceRequest {
            request_id: "fallback-crosses-horizon".to_owned(),
            arrival_seconds: 28_600,
            accept_by_seconds: 28_650,
            deadline_seconds: 28_800,
            required_service_seconds: 600,
            payload_units: 1,
            service_class: "standard".to_owned(),
            allowed_mode: ServiceMode::FallbackOnly,
        }];
        input.aerial_opportunities.clear();
        input.fallback_opportunities = vec![FallbackServiceOpportunity {
            opportunity_id: "van-crosses-horizon".to_owned(),
            fallback_resource_id: "van-01".to_owned(),
            fallback_mode: FallbackMode::GroundCourier,
            start_seconds: 28_700,
            max_service_duration_seconds: 600,
            max_payload_units: 100,
        }];
        input.capacity_source.max_aerial_opportunities = 0;
        input.policies = vec![AdmissionPolicy::FirstComeFirstServed];

        let result = evaluate_service_quality(&input).expect("bounded in-progress service");
        let policy = &result.policy_results[0];
        let outcome = &policy.outcomes[0];
        assert_eq!(outcome.disposition, RequestDisposition::InProgressAtHorizon);
        assert_eq!(outcome.service_start_seconds, Some(28_700));
        assert_eq!(outcome.service_completion_seconds, None);
        assert_eq!(outcome.projected_completion_seconds, Some(29_300));
        assert_eq!(outcome.latency_seconds, None);
        assert_eq!(policy.completed_on_time, 0);
        assert_eq!(policy.completed_late, 0);
        assert_eq!(policy.in_progress_requests, 1);
        assert_eq!(policy.in_progress_by_fallback, 1);
        assert_eq!(policy.served_by_fallback, 0);
        assert_eq!(policy.unserved_requests, 0);
        assert_eq!(policy.completed_requests_per_hour, 0.0);
    }

    #[test]
    fn linked_exact_oracle_finds_edf_optimum_and_quantifies_fcfs_regret() {
        let mut input = linked_fixture();
        input.requests.truncate(2);
        input.aerial_opportunity_qualifications.truncate(2);
        input.fallback_opportunities.clear();

        let report = evaluate_linked_service_quality(&input, &capacity_fixture())
            .expect("small linked exact-oracle fixture");
        let exact = report.exact_oracle;
        assert_eq!(exact.status, ExactOracleStatus::Evaluated);
        let score = exact.best_score.expect("exact score");
        assert_eq!(score.completed_on_time, 2);
        assert_eq!(score.completed_late, 0);
        assert_eq!(score.completed_by_horizon, 2);
        assert_eq!(score.in_progress_at_horizon, 0);
        assert_eq!(score.total_lateness_seconds, 0);
        assert_eq!(score.total_latency_seconds, 825);
        assert_eq!(exact.best_assignments.len(), 2);

        let fcfs = exact
            .policy_gaps
            .iter()
            .find(|gap| gap.policy == AdmissionPolicy::FirstComeFirstServed)
            .expect("FCFS gap");
        assert_eq!(fcfs.on_time_completion_gap, 1);
        assert_eq!(fcfs.completed_by_horizon_gap, 0);
        assert_eq!(fcfs.lateness_excess_seconds, 185);
        assert_eq!(fcfs.latency_excess_seconds, 0);

        let edf = exact
            .policy_gaps
            .iter()
            .find(|gap| gap.policy == AdmissionPolicy::EarliestDeadlineFirst)
            .expect("EDF gap");
        assert_eq!(edf.on_time_completion_gap, 0);
        assert_eq!(edf.completed_by_horizon_gap, 0);
        assert_eq!(edf.lateness_excess_seconds, 0);
        assert_eq!(edf.latency_excess_seconds, 0);
        assert_eq!(
            exact.best_assignments[0].request_id,
            "request-tight-deadline"
        );
        assert_eq!(
            exact.best_assignments[1].request_id,
            "request-first-arrival"
        );
    }

    #[test]
    fn linked_large_fixture_explicitly_skips_exact_oracle_above_bounds() {
        let report = linked_report();
        assert_eq!(
            report.exact_oracle.status,
            ExactOracleStatus::NotRunBoundsExceeded
        );
        assert!(report.exact_oracle.best_score.is_none());
        assert!(report.exact_oracle.best_assignments.is_empty());
        assert!(report.exact_oracle.policy_gaps.is_empty());
    }

    #[test]
    fn payload_capacity_is_a_hard_service_constraint() {
        let mut input = fixture();
        input.requests = vec![ServiceRequest {
            request_id: "oversize".to_owned(),
            arrival_seconds: 0,
            accept_by_seconds: 0,
            deadline_seconds: 2_000,
            required_service_seconds: 300,
            payload_units: 101,
            service_class: "oversize".to_owned(),
            allowed_mode: ServiceMode::AerialOnly,
        }];
        input.aerial_opportunities.truncate(1);
        input.fallback_opportunities.clear();
        input.capacity_source.max_aerial_opportunities = 1;
        let result = evaluate_service_quality(&input).expect("oversize request is safely unserved");
        assert_eq!(result.policy_results[0].unserved_requests, 1);
        assert_eq!(
            result.policy_results[0].outcomes[0].disposition,
            RequestDisposition::Unserved
        );
    }

    #[test]
    fn overlapping_aerial_opportunities_on_one_aircraft_are_rejected() {
        let mut input = fixture();
        input.aerial_opportunities[1].start_seconds =
            input.aerial_opportunities[0].start_seconds + 1;
        let errors = evaluate_service_quality(&input).expect_err("overlapping aircraft use");
        assert!(
            errors
                .iter()
                .any(|error| error.starts_with("aerial_opportunities_overlap_on_aircraft"))
        );
    }

    #[test]
    fn aerial_opportunity_budget_cannot_exceed_upstream_capacity() {
        let mut input = fixture();
        input.capacity_source.max_aerial_opportunities = 99;
        let errors = evaluate_service_quality(&input).expect_err("upstream capacity bound");
        assert!(errors.iter().any(|error| {
            error == "aerial_opportunity_budget_exceeds_upstream_capacity_envelope"
        }));
    }

    #[test]
    fn no_service_start_after_deadline() {
        let mut input = fixture();
        input.requests = vec![ServiceRequest {
            request_id: "missed".to_owned(),
            arrival_seconds: 0,
            accept_by_seconds: 0,
            deadline_seconds: 500,
            required_service_seconds: 100,
            payload_units: 1,
            service_class: "standard".to_owned(),
            allowed_mode: ServiceMode::AerialOnly,
        }];
        input
            .aerial_opportunities
            .retain(|op| op.start_seconds > 500);
        input.fallback_opportunities.clear();
        let result = evaluate_service_quality(&input).expect("unserved request");
        assert_eq!(result.policy_results[0].unserved_requests, 1);
    }

    #[test]
    fn input_order_permutation_preserves_policy_outputs() {
        let input = fixture();
        let mut permuted = fixture();
        permuted.requests.reverse();
        permuted.aerial_opportunities.reverse();
        permuted.fallback_opportunities.reverse();
        permuted.flight_windows.reverse();
        let first = evaluate_service_quality(&input).expect("canonical first");
        let second = evaluate_service_quality(&permuted).expect("canonical second");
        assert_eq!(
            serde_json::to_string(&first).expect("first JSON"),
            serde_json::to_string(&second).expect("second JSON")
        );
    }

    #[test]
    fn mode_constraints_are_respected() {
        let mut input = fixture();
        input.requests = vec![
            ServiceRequest {
                request_id: "aerial-only".to_owned(),
                arrival_seconds: 0,
                accept_by_seconds: 0,
                deadline_seconds: 2_000,
                required_service_seconds: 600,
                payload_units: 1,
                service_class: "standard".to_owned(),
                allowed_mode: ServiceMode::AerialOnly,
            },
            ServiceRequest {
                request_id: "fallback-only".to_owned(),
                arrival_seconds: 0,
                accept_by_seconds: 0,
                deadline_seconds: 2_000,
                required_service_seconds: 600,
                payload_units: 1,
                service_class: "standard".to_owned(),
                allowed_mode: ServiceMode::FallbackOnly,
            },
        ];
        let result = evaluate_service_quality(&input).expect("mode constraints");
        for outcome in &result.policy_results[0].outcomes {
            match outcome.request_id.as_str() {
                "aerial-only" => assert_eq!(outcome.fulfilment_mode, Some(FulfilmentMode::Aerial)),
                "fallback-only" => {
                    assert_ne!(outcome.fulfilment_mode, Some(FulfilmentMode::Aerial))
                }
                _ => panic!("unknown request"),
            }
        }
    }
}
