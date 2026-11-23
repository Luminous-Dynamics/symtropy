// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic request admission over prequalified aerial/fallback service slots.
//!
//! This module consumes a bounded capacity envelope from #6909 and externally
//! supplied service opportunities. It does not simulate batteries, route
//! aircraft, select strategic modes, or authorize flight. Unknown/stale
//! permission or weather evidence blocks aerial use.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

const MAX_REQUESTS: usize = 512;
const MAX_AERIAL_OPPORTUNITIES: usize = 512;
const MAX_FALLBACK_OPPORTUNITIES: usize = 512;
const MAX_FLIGHT_WINDOWS: usize = 256;
const MAX_BLOCKED_WINDOWS: usize = 256;
const MAX_HORIZON_SECONDS: u64 = 7 * 24 * 60 * 60;
const MAX_SERVICE_SECONDS: u32 = 24 * 60 * 60;
const MAX_PAYLOAD_UNITS: u16 = 10_000;
const MAX_OPPORTUNITIES_FROM_CAPACITY: u32 = 2_048;
const CLAIM_CEILING: &str = "Deterministic request-to-prequalified-service-slot admission model only; aerial service opportunities and capacity source are supplied inputs. Weather/permission metadata is not authenticated. No route planning, flight/dispatch, physical, market, regulatory or customer-delivery authority.";

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

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RequestDisposition {
    RejectedAfterAcceptanceCutoff,
    CompletedOnTime,
    CompletedLate,
    Unserved,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
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
    pub service_completion_seconds: Option<u64>,
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
    pub unserved_requests: usize,
    pub served_by_aerial: usize,
    pub served_by_fallback: usize,
    pub aerial_opportunities_available: usize,
    pub aerial_opportunities_blocked: usize,
    pub aerial_opportunities_unused: usize,
    pub fallback_opportunities_unused: usize,
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
    let rank = percentile
        .saturating_mul(values.len())
        .div_ceil(100)
        .max(1);
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
        let allowed = (capacity.steady_state_cycle_capacity_upper_bound_per_hour
            * hours
            + capacity.initial_inventory_capacity_allowance_per_hour * hours
            + 1e-9)
            .floor() as u64;
        if u64::from(capacity.max_aerial_opportunities) > allowed {
            add_error(&mut errors, "aerial_opportunity_budget_exceeds_upstream_capacity_envelope");
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
            .checked_add(u64::from(opportunity.max_service_duration_seconds))
            .and_then(|value| value.checked_add(u64::from(opportunity.turnaround_seconds)));
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
                format!("aerial_opportunity_identity_or_bounds_invalid:{}", opportunity.opportunity_id),
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
        add_error(&mut errors, "aerial_opportunity_count_exceeds_capacity_source_budget");
    }

    let mut by_fallback_resource: BTreeMap<&str, Vec<&FallbackServiceOpportunity>> = BTreeMap::new();
    for opportunity in &input.fallback_opportunities {
        let end = opportunity
            .start_seconds
            .checked_add(u64::from(opportunity.max_service_duration_seconds));
        if opportunity.opportunity_id.trim().is_empty()
            || !opportunity_ids.insert(opportunity.opportunity_id.as_str())
            || opportunity.fallback_resource_id.trim().is_empty()
            || opportunity.start_seconds > input.horizon_seconds
            || opportunity.max_service_duration_seconds == 0
            || opportunity.max_service_duration_seconds > MAX_SERVICE_SECONDS
            || opportunity.max_payload_units == 0
            || opportunity.max_payload_units > MAX_PAYLOAD_UNITS
            || end.is_none_or(|value| value > input.horizon_seconds)
        {
            add_error(
                &mut errors,
                format!("fallback_opportunity_identity_or_bounds_invalid:{}", opportunity.opportunity_id),
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
    sorted_windows.sort_by_key(|window| (window.start_seconds, window.end_seconds, window.window_id.as_str()));
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
                format!("flight_window_identity_or_evidence_invalid:{}", window.window_id),
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
                format!("blocked_interval_identity_or_bounds_invalid:{}", interval.interval_id),
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

fn request_priority(
    request: &ServiceRequest,
    policy: AdmissionPolicy,
) -> (u64, u64, String) {
    match policy {
        AdmissionPolicy::FirstComeFirstServed => (
            request.arrival_seconds,
            request.deadline_seconds,
            request.request_id.clone(),
        ),
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
    let mut aerial_opportunities_unused = 0_usize;
    let mut fallback_opportunities_unused = 0_usize;

    for opportunity in &opportunities {
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
                match aerial_block_reason(input, aerial, input.requests[*index].required_service_seconds) {
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
        let wait = opportunity.start_seconds.saturating_sub(request.arrival_seconds);
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

    for request_index in eligible_request_indices {
        let request = &input.requests[request_index];
        if let Some(assignment) = assignments.get(&request.request_id) {
            continue;
        }
        let _ = request;
    }

    for request in &input.requests {
        if let Some(assignment) = assignments.get(&request.request_id) {
            let latency = assignment
                .completion_seconds
                .saturating_sub(request.arrival_seconds);
            let wait = assignment
                .start_seconds
                .saturating_sub(request.arrival_seconds);
            let disposition = if assignment.completion_seconds <= request.deadline_seconds {
                RequestDisposition::CompletedOnTime
            } else {
                RequestDisposition::CompletedLate
            };
            outcomes.insert(
                request.request_id.clone(),
                RequestOutcome {
                    request_id: request.request_id.clone(),
                    disposition,
                    fulfilment_mode: Some(assignment.mode),
                    opportunity_id: Some(assignment.opportunity_id.clone()),
                    arrival_seconds: request.arrival_seconds,
                    service_start_seconds: Some(assignment.start_seconds),
                    service_completion_seconds: Some(assignment.completion_seconds),
                    latency_seconds: Some(latency),
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
    let unserved_requests = accepted_requests - completed_on_time - completed_late;
    let served_by_aerial = outcomes
        .iter()
        .filter(|outcome| outcome.fulfilment_mode == Some(FulfilmentMode::Aerial))
        .count();
    let served_by_fallback = outcomes
        .iter()
        .filter(|outcome| {
            matches!(
                outcome.fulfilment_mode,
                Some(
                    FulfilmentMode::GroundCourier | FulfilmentMode::Locker | FulfilmentMode::Amr
                )
            )
        })
        .count();

    let mut latencies: Vec<u64> = outcomes
        .iter()
        .filter_map(|outcome| outcome.latency_seconds)
        .collect();
    let mut waits: Vec<u64> = outcomes
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

    Ok(ServicePolicyResult {
        policy,
        offered_requests,
        accepted_requests,
        rejected_requests,
        completed_on_time,
        completed_late,
        unserved_requests,
        served_by_aerial,
        served_by_fallback,
        aerial_opportunities_available,
        aerial_opportunities_blocked: blocked_aerial_ids.len(),
        aerial_opportunities_unused,
        fallback_opportunities_unused,
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
        serde_json::from_str(include_str!("../fixtures/sustainable_mobility_service_quality_v0_1.json"))
            .expect("frozen request stream fixture must parse")
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
    fn fixture_reports_both_online_admission_policies() {
        let report = evaluate_service_quality(&fixture()).expect("frozen fixture");
        assert_eq!(report.policy_results.len(), 2);
        assert_eq!(report.policy_results[0].policy, AdmissionPolicy::FirstComeFirstServed);
        assert_eq!(report.policy_results[1].policy, AdmissionPolicy::EarliestDeadlineFirst);
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
                arrival_seconds: 0,
                accept_by_seconds: 0,
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
                turnaround_seconds: 100,
                capacity_reference: "synthetic://capacity/second".to_owned(),
            },
        ];
        input.capacity_source.max_aerial_opportunities = 2;
        input.fallback_opportunities.clear();

        let fcfs = {
            input.policies = vec![AdmissionPolicy::FirstComeFirstServed];
            evaluate_service_quality(&input).expect("FCFS").policy_results.remove(0)
        };
        let edf = {
            input.policies = vec![AdmissionPolicy::EarliestDeadlineFirst];
            evaluate_service_quality(&input).expect("EDF").policy_results.remove(0)
        };
        let first_fcfs = fcfs.outcomes.iter().find(|o| o.service_start_seconds == Some(100)).unwrap();
        let first_edf = edf.outcomes.iter().find(|o| o.service_start_seconds == Some(100)).unwrap();
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
        assert!(errors.iter().any(|error| error.starts_with("flight_window_identity_or_evidence_invalid")));
    }

    #[test]
    fn future_weather_observation_is_rejected_as_information_leakage() {
        let mut input = fixture();
        input.flight_windows[0].evidence.observed_at_seconds = input.flight_windows[0].start_seconds + 1;
        let errors = evaluate_service_quality(&input).expect_err("future observation cannot be known");
        assert!(errors.iter().any(|error| error.starts_with("flight_window_identity_or_evidence_invalid")));
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
                policy.completed_on_time + policy.completed_late + policy.unserved_requests,
                policy.accepted_requests
            );
        }
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
        input.aerial_opportunities[1].start_seconds = input.aerial_opportunities[0].start_seconds + 1;
        let errors = evaluate_service_quality(&input).expect_err("overlapping aircraft use");
        assert!(errors.iter().any(|error| error.starts_with("aerial_opportunities_overlap_on_aircraft")));
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
        input.aerial_opportunities.retain(|op| op.start_seconds > 500);
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
                "fallback-only" => assert_ne!(outcome.fulfilment_mode, Some(FulfilmentMode::Aerial)),
                _ => panic!("unknown request"),
            }
        }
    }
}
