// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact, bounded nonanticipative scenario-tree reference for request admission.
//!
//! One action is selected per information-set digest, and that action is shared
//! by every scenario with the same revealed history. This is an offline exact
//! reference policy, not an executing dispatcher or flight authorization.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::sustainable_mobility_service_capacity::{
    CapacityServiceOpportunity, SustainableServiceCapacityInput,
    evaluate_sustainable_service_capacity,
};
use crate::sustainable_mobility_service_quality::{
    FallbackMode, FlightWindow, LinkedServiceQualityInput,
    PermissionState as SourcePermissionState, ServiceMode,
    WeatherState as SourceWeatherState, evaluate_linked_service_quality,
};

const MAX_REQUESTS: usize = 4;
const MAX_OPPORTUNITIES: usize = 4;
const MAX_SCENARIOS: usize = 8;
const MAX_NODES: usize = 32;
const MAX_HORIZON_SECONDS: u64 = 86_400;
const MAX_SEARCH_TRANSITIONS: usize = 250_000;
const PROBABILITY_SCALE: u64 = 1_000_000;

const CLAIM_CEILING: &str = "Bounded synthetic nonanticipative assignment oracle only. Scenario probabilities, revealed observations, opportunity availability and service outcomes are supplied assumptions. No calibrated forecasting, route planning, flight authorization, physical service, market or actuation authority.";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum OpportunityMode {
    Aerial,
    GroundCourier,
    Locker,
    Amr,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AllowedMode {
    AerialOnly,
    FallbackOnly,
    Either,
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

#[derive(Debug, Clone)]
struct ScenarioRequest {
    pub request_id: String,
    pub arrival_seconds: u64,
    pub accept_by_seconds: u64,
    pub deadline_seconds: u64,
    pub required_service_seconds: u32,
    pub payload_units: u16,
    pub allowed_mode: AllowedMode,
}

#[derive(Debug, Clone)]
struct ScenarioOpportunity {
    pub opportunity_id: String,
    pub resource_id: String,
    pub mode: OpportunityMode,
    pub start_seconds: u64,
    pub max_service_duration_seconds: u32,
    pub turnaround_seconds: u32,
    pub max_payload_units: u16,
}

#[derive(Debug, Clone, Serialize)]
struct ObservationPayload {
    parent_information_set_digest: Option<String>,
    decision_time_seconds: u64,
    revealed_request_ids: Vec<String>,
    available_opportunity_ids: Vec<String>,
    weather_state: WeatherState,
    permission_state: PermissionState,
    weather_evidence_digest: String,
    permission_evidence_digest: String,
    flight_window_id: String,
}

#[derive(Debug, Clone, Serialize)]
struct InformationSetPayload {
    parent_information_set_digest: Option<String>,
    observation_digest: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationNode {
    pub node_id: String,
    pub parent_information_set_digest: Option<String>,
    pub information_set_digest: String,
    pub observation_digest: String,
    pub decision_time_seconds: u64,
    /// Cumulative request set known at this information set.
    pub revealed_request_ids: Vec<String>,
    /// Opportunities currently visible and eligible for consideration.
    pub available_opportunity_ids: Vec<String>,
    pub weather_state: WeatherState,
    pub permission_state: PermissionState,
    pub weather_evidence_digest: String,
    pub permission_evidence_digest: String,
    pub flight_window_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioPath {
    pub scenario_id: String,
    pub probability_units: u32,
    /// Ordered node IDs from the common root to this scenario's terminal node.
    pub node_path: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NonanticipativeOracleInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub horizon_seconds: u64,
    pub linked_service_input: LinkedServiceQualityInput,
    pub capacity_input: SustainableServiceCapacityInput,
    pub observation_nodes: Vec<ObservationNode>,
    pub scenarios: Vec<ScenarioPath>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OracleStatus {
    Evaluated,
    NotRunBoundsExceeded,
    TransitionLimitReached,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
pub struct ScenarioScore {
    pub completed_on_time: usize,
    pub completed_late: usize,
    pub completed_by_horizon: usize,
    pub in_progress_at_horizon: usize,
    pub unserved: usize,
    pub total_lateness_seconds: u64,
    pub total_latency_seconds: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
pub struct ExpectedScore {
    /// Probability-weighted count, divided by 1,000,000 for an expectation.
    pub expected_on_time_services_units: u64,
    pub expected_completed_by_horizon_units: u64,
    pub expected_late_services_units: u64,
    pub expected_in_progress_units: u64,
    /// Probability-weighted seconds, divided by 1,000,000 for an expectation.
    pub expected_lateness_seconds_units: u64,
    pub expected_latency_seconds_units: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PolicyAssignment {
    pub request_id: String,
    pub opportunity_id: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct InformationSetDecision {
    pub information_set_digest: String,
    pub decision_time_seconds: u64,
    pub assignments: Vec<PolicyAssignment>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ScenarioResult {
    pub scenario_id: String,
    pub probability_units: u32,
    pub score: ScenarioScore,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PerfectInformationDelta {
    pub expected_on_time_services_units: i64,
    pub expected_completed_by_horizon_units: i64,
    pub expected_late_services_units: i64,
    pub expected_in_progress_units: i64,
    pub expected_lateness_seconds_units: i64,
    pub expected_latency_seconds_units: i64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NonanticipativeOracleReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub status: OracleStatus,
    pub request_count: usize,
    pub opportunity_count: usize,
    pub scenario_count: usize,
    pub information_set_count: usize,
    pub capacity_source_architecture_id: String,
    pub capacity_source_report_digest: String,
    pub capacity_service_opportunity_digest: String,
    pub capacity_architecture_configuration_digest: String,
    pub capacity_opportunities_in_source: usize,
    pub capacity_opportunities_qualified: usize,
    pub search_transition_limit: usize,
    pub search_transitions: usize,
    pub information_model: &'static str,
    pub deployable_online_policy: bool,
    pub nonanticipativity_verified: bool,
    pub nonanticipative_score: Option<ExpectedScore>,
    pub clairvoyant_perfect_information_score: Option<ExpectedScore>,
    pub value_of_perfect_information: Option<PerfectInformationDelta>,
    pub policy: Vec<InformationSetDecision>,
    pub scenario_results: Vec<ScenarioResult>,
    pub dispatch_authority_granted: bool,
    pub flight_authority_granted: bool,
    pub physical_mutation_authority_granted: bool,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone)]
struct InformationSet {
    parent_digest: Option<String>,
    decision_time_seconds: u64,
    revealed_mask: u16,
    available_mask: u16,
    weather_state: WeatherState,
    permission_state: PermissionState,
}

#[derive(Debug, Clone, Default)]
struct SearchState {
    assigned_opportunity_by_request: Vec<Option<usize>>,
    used_opportunity_mask: u16,
    revealed_mask: u16,
}

#[derive(Debug, Clone, Default)]
struct WeightedScore {
    on_time: u64,
    completed: u64,
    late: u64,
    in_progress: u64,
    lateness_seconds: u64,
    latency_seconds: u64,
}

impl WeightedScore {
    fn add_assign(&mut self, other: &Self) {
        self.on_time += other.on_time;
        self.completed += other.completed;
        self.late += other.late;
        self.in_progress += other.in_progress;
        self.lateness_seconds += other.lateness_seconds;
        self.latency_seconds += other.latency_seconds;
    }

    fn add_scenario(&mut self, score: &ScenarioScore, probability_units: u32) {
        let p = u64::from(probability_units);
        self.on_time += (score.completed_on_time as u64) * p;
        self.completed += (score.completed_by_horizon as u64) * p;
        self.late += (score.completed_late as u64) * p;
        self.in_progress += (score.in_progress_at_horizon as u64) * p;
        self.lateness_seconds += score.total_lateness_seconds * p;
        self.latency_seconds += score.total_latency_seconds * p;
    }

    fn public(&self) -> ExpectedScore {
        ExpectedScore {
            expected_on_time_services_units: self.on_time,
            expected_completed_by_horizon_units: self.completed,
            expected_late_services_units: self.late,
            expected_in_progress_units: self.in_progress,
            expected_lateness_seconds_units: self.lateness_seconds,
            expected_latency_seconds_units: self.latency_seconds,
        }
    }
}

#[derive(Debug, Clone)]
struct Solution {
    score: WeightedScore,
    decisions: Vec<InformationSetDecision>,
    canonical_key: String,
}

#[derive(Debug, Clone)]
struct SearchContext {
    transitions: usize,
    limit_reached: bool,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(71);
    output.push_str("sha256:");
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn is_sha256_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn observation_digests(node: &ObservationNode) -> Result<(String, String), String> {
    let mut revealed = node.revealed_request_ids.clone();
    revealed.sort();
    let mut available = node.available_opportunity_ids.clone();
    available.sort();
    let observation = ObservationPayload {
        parent_information_set_digest: node.parent_information_set_digest.clone(),
        decision_time_seconds: node.decision_time_seconds,
        revealed_request_ids: revealed,
        available_opportunity_ids: available,
        weather_state: node.weather_state,
        permission_state: node.permission_state,
        weather_evidence_digest: node.weather_evidence_digest.clone(),
        permission_evidence_digest: node.permission_evidence_digest.clone(),
    };
    let observation_bytes = serde_json::to_vec(&observation)
        .map_err(|error| format!("observation_serialization_failed:{error}"))?;
    let observation_digest = sha256_hex(&observation_bytes);
    let information = InformationSetPayload {
        parent_information_set_digest: node.parent_information_set_digest.clone(),
        observation_digest: observation_digest.clone(),
    };
    let information_bytes = serde_json::to_vec(&information)
        .map_err(|error| format!("information_set_serialization_failed:{error}"))?;
    Ok((observation_digest, sha256_hex(&information_bytes)))
}

#[derive(Debug, Clone)]
struct CapacityMetadata {
    architecture_id: String,
    report_digest: String,
    opportunity_digest: String,
    architecture_digest: String,
    opportunity_count: usize,
    qualified_count: usize,
}

struct DerivedServiceInputs {
    requests: Vec<ScenarioRequest>,
    opportunities: Vec<ScenarioOpportunity>,
    capacity: CapacityMetadata,
}

fn source_weather_state(state: SourceWeatherState) -> WeatherState {
    match state {
        SourceWeatherState::Go => WeatherState::Go,
        SourceWeatherState::NoGo => WeatherState::NoGo,
        SourceWeatherState::Unknown => WeatherState::Unknown,
    }
}

fn source_permission_state(state: SourcePermissionState) -> PermissionState {
    match state {
        SourcePermissionState::Approved => PermissionState::Approved,
        SourcePermissionState::Denied => PermissionState::Denied,
        SourcePermissionState::Unknown => PermissionState::Unknown,
    }
}

fn flight_weather_evidence_digest(window: &FlightWindow) -> Result<String, String> {
    let bytes = serde_json::to_vec(&(
        window.window_id.as_str(),
        window.start_seconds,
        window.end_seconds,
        window.weather_state,
        &window.evidence,
    ))
    .map_err(|error| format!("flight_weather_evidence_serialization_failed:{error}"))?;
    Ok(sha256_hex(&bytes))
}

fn flight_permission_evidence_digest(window: &FlightWindow) -> Result<String, String> {
    let bytes = serde_json::to_vec(&(
        window.window_id.as_str(),
        window.start_seconds,
        window.end_seconds,
        window.permission_state,
        &window.evidence,
    ))
    .map_err(|error| format!("flight_permission_evidence_serialization_failed:{error}"))?;
    Ok(sha256_hex(&bytes))
}

fn validate_node_flight_observation(
    node: &ObservationNode,
    linked_input: &LinkedServiceQualityInput,
) -> Result<(), String> {
    let matches = linked_input
        .flight_windows
        .iter()
        .filter(|window| window.window_id == node.flight_window_id)
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(format!(
            "information_node_flight_window_missing_or_ambiguous:{}",
            node.node_id
        ));
    }
    let window = matches[0];
    let evidence = &window.evidence;
    if window.start_seconds > node.decision_time_seconds
        || node.decision_time_seconds >= window.end_seconds
        || evidence.site_id != linked_input.site_id
        || evidence.site_configuration_digest != linked_input.site_configuration_digest
        || evidence.evidence_class == crate::sustainable_mobility_service_quality::EvidenceClass::Unknown
        || evidence.observed_at_seconds > node.decision_time_seconds
        || evidence.valid_from_seconds > node.decision_time_seconds
        || evidence.valid_until_seconds < node.decision_time_seconds
        || source_weather_state(window.weather_state) != node.weather_state
        || source_permission_state(window.permission_state) != node.permission_state
        || flight_weather_evidence_digest(window)? != node.weather_evidence_digest
        || flight_permission_evidence_digest(window)? != node.permission_evidence_digest
    {
        return Err(format!(
            "information_node_weather_permission_evidence_invalid:{}",
            node.node_id
        ));
    }
    Ok(())
}

fn derive_service_inputs(
    input: &NonanticipativeOracleInput,
) -> Result<DerivedServiceInputs, Vec<String>> {
    let linked_report = evaluate_linked_service_quality(
        &input.linked_service_input,
        &input.capacity_input,
    )?;
    let capacity_report = evaluate_sustainable_service_capacity(&input.capacity_input)?;
    let selected_results = capacity_report
        .architecture_results
        .iter()
        .filter(|result| {
            result.architecture_id == input.linked_service_input.source_architecture_id
        })
        .collect::<Vec<_>>();
    let selected_architectures = input
        .capacity_input
        .architectures
        .iter()
        .filter(|architecture| {
            architecture.architecture_id == input.linked_service_input.source_architecture_id
        })
        .collect::<Vec<_>>();
    if selected_results.len() != 1 || selected_architectures.len() != 1 {
        return Err(vec![
            "nonanticipative_capacity_architecture_missing_or_ambiguous".to_owned(),
        ]);
    }
    let selected = selected_results[0];
    let architecture = selected_architectures[0];
    let report_bytes = serde_json::to_vec(&capacity_report)
        .map_err(|error| vec![format!("capacity_report_serialization_failed:{error}")])?;
    let architecture_bytes = serde_json::to_vec(architecture)
        .map_err(|error| vec![format!("capacity_architecture_serialization_failed:{error}")])?;
    let opportunity_bytes = serde_json::to_vec(&selected.service_opportunities)
        .map_err(|error| vec![format!("capacity_opportunity_serialization_failed:{error}")])?;
    let report_digest = sha256_hex(&report_bytes);
    let architecture_digest = sha256_hex(&architecture_bytes);
    let opportunity_digest = sha256_hex(&opportunity_bytes);
    if linked_report.capacity_source_report_digest != report_digest
        || linked_report.capacity_architecture_configuration_digest != architecture_digest
        || linked_report.capacity_service_opportunity_digest != opportunity_digest
    {
        return Err(vec![
            "linked_capacity_source_digest_reconciliation_failed".to_owned(),
        ]);
    }

    let mut requests = input
        .linked_service_input
        .requests
        .iter()
        .map(|request| ScenarioRequest {
            request_id: request.request_id.clone(),
            arrival_seconds: request.arrival_seconds,
            accept_by_seconds: request.accept_by_seconds,
            deadline_seconds: request.deadline_seconds,
            required_service_seconds: request.required_service_seconds,
            payload_units: request.payload_units,
            allowed_mode: match request.allowed_mode {
                ServiceMode::AerialOnly => AllowedMode::AerialOnly,
                ServiceMode::FallbackOnly => AllowedMode::FallbackOnly,
                ServiceMode::Either => AllowedMode::Either,
            },
        })
        .collect::<Vec<_>>();
    requests.sort_by(|left, right| left.request_id.cmp(&right.request_id));

    let mut opportunities = Vec::new();
    let mut used_opportunity_ids = BTreeSet::new();
    for qualification in &input
        .linked_service_input
        .aerial_opportunity_qualifications
    {
        let Some(receipt) = selected
            .service_opportunities
            .iter()
            .find(|receipt| receipt.opportunity_id == qualification.opportunity_id)
        else {
            return Err(vec![format!(
                "nonanticipative_qualification_not_in_capacity_report:{}",
                qualification.opportunity_id
            )]);
        };
        if !used_opportunity_ids.insert(receipt.opportunity_id.as_str()) {
            return Err(vec![format!(
                "nonanticipative_capacity_receipt_reused:{}",
                receipt.opportunity_id
            )]);
        }
        opportunities.push(ScenarioOpportunity {
            opportunity_id: receipt.opportunity_id.clone(),
            resource_id: receipt.aircraft_id.clone(),
            mode: OpportunityMode::Aerial,
            start_seconds: receipt.start_seconds,
            max_service_duration_seconds: receipt.mission_duration_seconds,
            turnaround_seconds: receipt.turnaround_seconds,
            max_payload_units: qualification.max_payload_units,
        });
    }
    for fallback in &input.linked_service_input.fallback_opportunities {
        if !used_opportunity_ids.insert(fallback.opportunity_id.as_str()) {
            return Err(vec![format!(
                "nonanticipative_opportunity_id_duplicate:{}",
                fallback.opportunity_id
            )]);
        }
        opportunities.push(ScenarioOpportunity {
            opportunity_id: fallback.opportunity_id.clone(),
            resource_id: fallback.fallback_resource_id.clone(),
            mode: match fallback.fallback_mode {
                FallbackMode::GroundCourier => OpportunityMode::GroundCourier,
                FallbackMode::Locker => OpportunityMode::Locker,
                FallbackMode::Amr => OpportunityMode::Amr,
            },
            start_seconds: fallback.start_seconds,
            max_service_duration_seconds: fallback.max_service_duration_seconds,
            turnaround_seconds: 0,
            max_payload_units: fallback.max_payload_units,
        });
    }
    opportunities.sort_by(|left, right| {
        (
            left.start_seconds,
            left.resource_id.as_str(),
            left.opportunity_id.as_str(),
        )
            .cmp(&(
                right.start_seconds,
                right.resource_id.as_str(),
                right.opportunity_id.as_str(),
            ))
    });

    Ok(DerivedServiceInputs {
        requests,
        opportunities,
        capacity: CapacityMetadata {
            architecture_id: selected.architecture_id.clone(),
            report_digest,
            opportunity_digest,
            architecture_digest,
            opportunity_count: selected.service_opportunities.len(),
            qualified_count: linked_report.capacity_opportunities_qualified
                + input.linked_service_input.fallback_opportunities.len(),
        },
    })
}

fn scenario_score(
    state: &SearchState,
    requests: &[ScenarioRequest],
    opportunities: &[ScenarioOpportunity],
    horizon_seconds: u64,
) -> ScenarioScore {
    let mut score = ScenarioScore::default();
    for (index, request) in requests.iter().enumerate() {
        if state.revealed_mask & (1_u16 << index) == 0 {
            continue;
        }
        let Some(opportunity_index) = state.assigned_opportunity_by_request[index] else {
            score.unserved += 1;
            continue;
        };
        let opportunity = &opportunities[opportunity_index];
        let completion = opportunity
            .start_seconds
            .saturating_add(u64::from(request.required_service_seconds));
        if completion > horizon_seconds {
            score.in_progress_at_horizon += 1;
            continue;
        }
        score.completed_by_horizon += 1;
        score.total_latency_seconds += completion.saturating_sub(request.arrival_seconds);
        if completion <= request.deadline_seconds {
            score.completed_on_time += 1;
        } else {
            score.completed_late += 1;
            score.total_lateness_seconds += completion.saturating_sub(request.deadline_seconds);
        }
    }
    score
}

fn mode_allowed(request: &ScenarioRequest, opportunity: &ScenarioOpportunity) -> bool {
    match request.allowed_mode {
        AllowedMode::AerialOnly => opportunity.mode == OpportunityMode::Aerial,
        AllowedMode::FallbackOnly => opportunity.mode != OpportunityMode::Aerial,
        AllowedMode::Either => true,
    }
}

fn pair_eligible(
    request_index: usize,
    opportunity_index: usize,
    node: &InformationSet,
    requests: &[ScenarioRequest],
    opportunities: &[ScenarioOpportunity],
) -> bool {
    let request = &requests[request_index];
    let opportunity = &opportunities[opportunity_index];
    if node.revealed_mask & (1_u16 << request_index) == 0
        || request.arrival_seconds > node.decision_time_seconds
        || request.accept_by_seconds < node.decision_time_seconds
        || opportunity.start_seconds < node.decision_time_seconds
        || opportunity.start_seconds < request.arrival_seconds
        || opportunity.max_service_duration_seconds < request.required_service_seconds
        || opportunity.max_payload_units < request.payload_units
        || !mode_allowed(request, opportunity)
    {
        return false;
    }
    if opportunity.mode == OpportunityMode::Aerial
        && (node.weather_state != WeatherState::Go
            || node.permission_state != PermissionState::Approved)
    {
        return false;
    }
    opportunity
        .start_seconds
        .checked_add(u64::from(request.required_service_seconds))
        .is_some()
}

fn enumerate_actions_for_requests(
    request_cursor: usize,
    eligible_requests: &[usize],
    candidates_by_request: &[Vec<usize>],
    used_opportunity_mask: u16,
    current: &mut Vec<(usize, usize)>,
    output: &mut Vec<Vec<(usize, usize)>>,
) {
    if request_cursor == eligible_requests.len() {
        output.push(current.clone());
        return;
    }
    let request_index = eligible_requests[request_cursor];
    enumerate_actions_for_requests(
        request_cursor + 1,
        eligible_requests,
        candidates_by_request,
        used_opportunity_mask,
        current,
        output,
    );
    for opportunity_index in &candidates_by_request[request_index] {
        let bit = 1_u16 << opportunity_index;
        if used_opportunity_mask & bit != 0 {
            continue;
        }
        current.push((request_index, *opportunity_index));
        enumerate_actions_for_requests(
            request_cursor + 1,
            eligible_requests,
            candidates_by_request,
            used_opportunity_mask | bit,
            current,
            output,
        );
        current.pop();
    }
}

fn enumerate_node_actions(
    node: &InformationSet,
    state: &SearchState,
    requests: &[ScenarioRequest],
    opportunities: &[ScenarioOpportunity],
) -> Vec<Vec<(usize, usize)>> {
    let mut eligible_requests = Vec::new();
    let mut candidates_by_request = vec![Vec::new(); requests.len()];
    for (request_index, _) in requests.iter().enumerate() {
        if node.revealed_mask & (1_u16 << request_index) == 0
            || state.assigned_opportunity_by_request[request_index].is_some()
        {
            continue;
        }
        let mut feasible = Vec::new();
        for (opportunity_index, _) in opportunities.iter().enumerate() {
            if node.available_mask & (1_u16 << opportunity_index) != 0
                && pair_eligible(
                    request_index,
                    opportunity_index,
                    node,
                    requests,
                    opportunities,
                )
            {
                feasible.push(opportunity_index);
            }
        }
        if !feasible.is_empty() {
            eligible_requests.push(request_index);
            candidates_by_request[request_index] = feasible;
        }
    }
    let mut actions = Vec::new();
    enumerate_actions_for_requests(
        0,
        &eligible_requests,
        &candidates_by_request,
        state.used_opportunity_mask,
        &mut Vec::new(),
        &mut actions,
    );
    actions
}

fn score_better(left: &WeightedScore, right: &WeightedScore) -> bool {
    if left.on_time != right.on_time {
        return left.on_time > right.on_time;
    }
    if left.completed != right.completed {
        return left.completed > right.completed;
    }
    if left.lateness_seconds != right.lateness_seconds {
        return left.lateness_seconds < right.lateness_seconds;
    }
    if left.in_progress != right.in_progress {
        return left.in_progress < right.in_progress;
    }
    if left.latency_seconds != right.latency_seconds {
        return left.latency_seconds < right.latency_seconds;
    }
    false
}

fn solve_information_set(
    digest: &str,
    groups: &BTreeMap<String, InformationSet>,
    children: &BTreeMap<String, Vec<String>>,
    terminal_scenarios: &BTreeMap<String, Vec<usize>>,
    scenarios: &[ScenarioPath],
    requests: &[ScenarioRequest],
    opportunities: &[ScenarioOpportunity],
    horizon_seconds: u64,
    parent_state: &SearchState,
    context: &mut SearchContext,
) -> Option<Solution> {
    let node = groups.get(digest)?;
    let mut state = parent_state.clone();
    state.revealed_mask = node.revealed_mask;

    let actions = enumerate_node_actions(node, &state, requests, opportunities);
    let mut best: Option<Solution> = None;

    for action in actions {
        context.transitions = context.transitions.saturating_add(1);
        if context.transitions > MAX_SEARCH_TRANSITIONS {
            context.limit_reached = true;
            return None;
        }

        let mut next_state = state.clone();
        for (request_index, opportunity_index) in &action {
            if next_state.assigned_opportunity_by_request[*request_index].is_some()
                || next_state.used_opportunity_mask & (1_u16 << opportunity_index) != 0
            {
                context.limit_reached = true;
                return None;
            }
            next_state.assigned_opportunity_by_request[*request_index] = Some(*opportunity_index);
            next_state.used_opportunity_mask |= 1_u16 << opportunity_index;
        }

        let mut score = WeightedScore::default();
        if let Some(terminal) = terminal_scenarios.get(digest) {
            for scenario_index in terminal {
                let scenario = &scenarios[*scenario_index];
                let result = scenario_score(&next_state, requests, opportunities, horizon_seconds);
                score.add_scenario(&result, scenario.probability_units);
            }
        }

        let mut decisions = vec![InformationSetDecision {
            information_set_digest: digest.to_owned(),
            decision_time_seconds: node.decision_time_seconds,
            assignments: action
                .iter()
                .map(|(request_index, opportunity_index)| PolicyAssignment {
                    request_id: requests[*request_index].request_id.clone(),
                    opportunity_id: opportunities[*opportunity_index].opportunity_id.clone(),
                })
                .collect(),
        }];
        decisions[0]
            .assignments
            .sort_by(|a, b| a.request_id.cmp(&b.request_id));

        if let Some(child_digests) = children.get(digest) {
            for child_digest in child_digests {
                let Some(child_solution) = solve_information_set(
                    child_digest,
                    groups,
                    children,
                    terminal_scenarios,
                    scenarios,
                    requests,
                    opportunities,
                    horizon_seconds,
                    &next_state,
                    context,
                ) else {
                    return None;
                };
                score.add_assign(&child_solution.score);
                decisions.extend(child_solution.decisions);
            }
        }

        decisions.sort_by(|a, b| a.information_set_digest.cmp(&b.information_set_digest));
        let canonical_key = serde_json::to_string(&decisions).ok()?;
        let candidate = Solution {
            score,
            decisions,
            canonical_key,
        };
        let replace = best.as_ref().is_none_or(|current| {
            score_better(&candidate.score, &current.score)
                || (candidate.score == current.score
                    && candidate.canonical_key < current.canonical_key)
        });
        if replace {
            best = Some(candidate);
        }
    }

    best
}

fn clairvoyant_scenario_score(
    scenario: &ScenarioPath,
    node_by_id: &BTreeMap<String, ObservationNode>,
    requests: &[ScenarioRequest],
    opportunities: &[ScenarioOpportunity],
    horizon_seconds: u64,
    transitions: &mut usize,
) -> Result<ScenarioScore, ()> {
    let mut revealed_mask = 0_u16;
    let mut candidates_by_request = vec![Vec::new(); requests.len()];
    for node_id in &scenario.node_path {
        let node = node_by_id.get(node_id).ok_or(())?;
        for request_id in &node.revealed_request_ids {
            let index = requests.iter().position(|item| item.request_id == *request_id).ok_or(())?;
            revealed_mask |= 1_u16 << index;
        }
        let info = InformationSet {
            parent_digest: node.parent_information_set_digest.clone(),
            decision_time_seconds: node.decision_time_seconds,
            revealed_mask: node.revealed_request_ids.iter().filter_map(|id| requests.iter().position(|r| r.request_id == *id)).fold(0_u16, |mask, i| mask | (1_u16 << i)),
            available_mask: 0,
            weather_state: node.weather_state,
            permission_state: node.permission_state,
        };
        for (opportunity_index, opportunity) in opportunities.iter().enumerate() {
            if node.available_opportunity_ids.iter().any(|id| id == &opportunity.opportunity_id) {
                for (request_index, _) in requests.iter().enumerate() {
                    if pair_eligible(
                        request_index,
                        opportunity_index,
                        &info,
                        requests,
                        opportunities,
                    ) && !candidates_by_request[request_index].contains(&opportunity_index)
                    {
                        candidates_by_request[request_index].push(opportunity_index);
                    }
                }
            }
        }
    }

    let eligible_requests = (0..requests.len())
        .filter(|index| revealed_mask & (1_u16 << index) != 0)
        .collect::<Vec<_>>();
    let mut actions = Vec::new();
    enumerate_actions_for_requests(
        0,
        &eligible_requests,
        &candidates_by_request,
        0,
        &mut Vec::new(),
        &mut actions,
    );
    let mut best: Option<(ScenarioScore, String)> = None;
    for action in actions {
        *transitions = transitions.saturating_add(1);
        if *transitions > MAX_SEARCH_TRANSITIONS {
            return Err(());
        }
        let mut state = SearchState {
            assigned_opportunity_by_request: vec![None; requests.len()],
            used_opportunity_mask: 0,
            revealed_mask,
        };
        for (request_index, opportunity_index) in action {
            state.assigned_opportunity_by_request[request_index] = Some(opportunity_index);
            state.used_opportunity_mask |= 1_u16 << opportunity_index;
        }
        let score = scenario_score(&state, requests, opportunities, horizon_seconds);
        let key = serde_json::to_string(&state.assigned_opportunity_by_request).map_err(|_| ())?;
        let replace = best.as_ref().is_none_or(|(current, current_key)| {
            score.completed_on_time > current.completed_on_time
                || (score.completed_on_time == current.completed_on_time
                    && score.completed_by_horizon > current.completed_by_horizon)
                || (score.completed_on_time == current.completed_on_time
                    && score.completed_by_horizon == current.completed_by_horizon
                    && score.total_lateness_seconds < current.total_lateness_seconds)
                || (score.completed_on_time == current.completed_on_time
                    && score.completed_by_horizon == current.completed_by_horizon
                    && score.total_lateness_seconds == current.total_lateness_seconds
                    && score.in_progress_at_horizon < current.in_progress_at_horizon)
                || (score.completed_on_time == current.completed_on_time
                    && score.completed_by_horizon == current.completed_by_horizon
                    && score.total_lateness_seconds == current.total_lateness_seconds
                    && score.in_progress_at_horizon == current.in_progress_at_horizon
                    && score.total_latency_seconds < current.total_latency_seconds)
                || (score == *current && key < *current_key)
        });
        if replace {
            best = Some((score, key));
        }
    }
    let _ = horizon_seconds;
    best.map(|(score, _)| score).ok_or(())
}

fn expected_score_from_scenarios(
    scenarios: &[ScenarioPath],
    scores: &[ScenarioScore],
) -> ExpectedScore {
    let mut weighted = WeightedScore::default();
    for (scenario, score) in scenarios.iter().zip(scores) {
        weighted.add_scenario(score, scenario.probability_units);
    }
    weighted.public()
}

fn signed_difference(left: u64, right: u64) -> i64 {
    if left >= right {
        i64::try_from(left - right).unwrap_or(i64::MAX)
    } else {
        -i64::try_from(right - left).unwrap_or(i64::MAX)
    }
}

fn value_of_perfect_information(
    clairvoyant: &ExpectedScore,
    nonanticipative: &ExpectedScore,
) -> PerfectInformationDelta {
    PerfectInformationDelta {
        expected_on_time_services_units: signed_difference(
            clairvoyant.expected_on_time_services_units,
            nonanticipative.expected_on_time_services_units,
        ),
        expected_completed_by_horizon_units: signed_difference(
            clairvoyant.expected_completed_by_horizon_units,
            nonanticipative.expected_completed_by_horizon_units,
        ),
        expected_late_services_units: signed_difference(
            nonanticipative.expected_late_services_units,
            clairvoyant.expected_late_services_units,
        ),
        expected_in_progress_units: signed_difference(
            nonanticipative.expected_in_progress_units,
            clairvoyant.expected_in_progress_units,
        ),
        expected_lateness_seconds_units: signed_difference(
            nonanticipative.expected_lateness_seconds_units,
            clairvoyant.expected_lateness_seconds_units,
        ),
        expected_latency_seconds_units: signed_difference(
            nonanticipative.expected_latency_seconds_units,
            clairvoyant.expected_latency_seconds_units,
        ),
    }
}

pub fn evaluate_nonanticipative_oracle(
    input: &NonanticipativeOracleInput,
) -> Result<NonanticipativeOracleReport, Vec<String>> {
    let mut errors = Vec::new();
    if input.schema_version != "0.1.0"
        || input.benchmark_id != "NONANTICIPATIVE-SERVICE-ORACLE-001"
        || input.horizon_seconds == 0
        || input.horizon_seconds > MAX_HORIZON_SECONDS
        || input.claim_ceiling.trim().is_empty()
        || input.requests.is_empty()
        || input.opportunities.is_empty()
        || input.observation_nodes.is_empty()
        || input.scenarios.is_empty()
    {
        add_error(&mut errors, "benchmark_identity_or_empty_input_invalid");
    }
    if input.requests.len() > MAX_REQUESTS
        || input.opportunities.len() > MAX_OPPORTUNITIES
        || input.scenarios.len() > MAX_SCENARIOS
        || input.observation_nodes.len() > MAX_NODES
    {
        return Ok(NonanticipativeOracleReport {
            schema_version: input.schema_version.clone(),
            benchmark_id: input.benchmark_id.clone(),
            status: OracleStatus::NotRunBoundsExceeded,
            request_count: input.requests.len(),
            opportunity_count: input.opportunities.len(),
            scenario_count: input.scenarios.len(),
            information_set_count: 0,
            search_transition_limit: MAX_SEARCH_TRANSITIONS,
            search_transitions: 0,
            information_model: "nonanticipative_information_set_policy",
            deployable_online_policy: false,
            nonanticipativity_verified: false,
            nonanticipative_score: None,
            clairvoyant_perfect_information_score: None,
            value_of_perfect_information: None,
            policy: Vec::new(),
            scenario_results: Vec::new(),
            dispatch_authority_granted: false,
            flight_authority_granted: false,
            physical_mutation_authority_granted: false,
            claim_ceiling: CLAIM_CEILING,
        });
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let mut request_ids = BTreeMap::new();
    for (index, request) in input.requests.iter().enumerate() {
        if request.request_id.trim().is_empty()
            || request.request_id.len() > 80
            || request_ids.insert(request.request_id.as_str(), index).is_some()
            || request.arrival_seconds > input.horizon_seconds
            || request.accept_by_seconds > input.horizon_seconds
            || request.accept_by_seconds < request.arrival_seconds
            || request.deadline_seconds < request.arrival_seconds
            || request.required_service_seconds == 0
            || request.payload_units == 0
        {
            add_error(&mut errors, format!("request_identity_or_bounds_invalid:{}", request.request_id));
        }
    }

    let mut opportunity_ids = BTreeMap::new();
    let mut resource_windows: BTreeMap<&str, Vec<(u64, u64, &str)>> = BTreeMap::new();
    for (index, opportunity) in input.opportunities.iter().enumerate() {
        let end = opportunity
            .start_seconds
            .checked_add(u64::from(opportunity.max_service_duration_seconds))
            .and_then(|value| value.checked_add(u64::from(opportunity.turnaround_seconds)));
        if opportunity.opportunity_id.trim().is_empty()
            || opportunity.resource_id.trim().is_empty()
            || opportunity_ids.insert(opportunity.opportunity_id.as_str(), index).is_some()
            || opportunity.start_seconds >= input.horizon_seconds
            || opportunity.max_service_duration_seconds == 0
            || opportunity.max_payload_units == 0
            || end.is_none_or(|value| value > input.horizon_seconds.saturating_add(MAX_HORIZON_SECONDS))
        {
            add_error(&mut errors, format!("opportunity_identity_or_bounds_invalid:{}", opportunity.opportunity_id));
            continue;
        }
        resource_windows
            .entry(opportunity.resource_id.as_str())
            .or_default()
            .push((opportunity.start_seconds, end.unwrap_or(u64::MAX), opportunity.opportunity_id.as_str()));
    }
    for windows in resource_windows.values_mut() {
        windows.sort_by_key(|item| (item.0, item.1, item.2));
        for pair in windows.windows(2) {
            if pair[0].1 > pair[1].0 {
                add_error(&mut errors, format!("opportunity_resource_overlap:{}:{}", pair[0].2, pair[1].2));
            }
        }
    }

    let mut node_by_id: BTreeMap<String, ObservationNode> = BTreeMap::new();
    let mut groups: BTreeMap<String, InformationSet> = BTreeMap::new();
    for node in &input.observation_nodes {
        if node.node_id.trim().is_empty()
            || node.decision_time_seconds > input.horizon_seconds
            || node.observation_digest.trim().is_empty()
            || node.information_set_digest.trim().is_empty()
            || !is_sha256_digest(&node.weather_evidence_digest)
            || !is_sha256_digest(&node.permission_evidence_digest)
            || node.parent_information_set_digest.as_ref().is_some_and(|digest| !is_sha256_digest(digest))
            || node_by_id.insert(node.node_id.clone(), node.clone()).is_some()
        {
            add_error(&mut errors, format!("observation_node_identity_or_evidence_invalid:{}", node.node_id));
            continue;
        }
        let mut revealed_mask = 0_u16;
        let mut seen_requests = BTreeSet::new();
        for request_id in &node.revealed_request_ids {
            let Some(index) = request_ids.get(request_id.as_str()).copied() else {
                add_error(&mut errors, format!("observation_reveals_unknown_request:{}:{request_id}", node.node_id));
                continue;
            };
            if !seen_requests.insert(request_id.as_str())
                || input.requests[index].arrival_seconds > node.decision_time_seconds
            {
                add_error(&mut errors, format!("observation_request_reveal_invalid:{}:{request_id}", node.node_id));
            }
            revealed_mask |= 1_u16 << index;
        }
        let mut available_mask = 0_u16;
        let mut seen_opportunities = BTreeSet::new();
        for opportunity_id in &node.available_opportunity_ids {
            let Some(index) = opportunity_ids.get(opportunity_id.as_str()).copied() else {
                add_error(&mut errors, format!("observation_available_opportunity_unknown:{}:{opportunity_id}", node.node_id));
                continue;
            };
            if !seen_opportunities.insert(opportunity_id.as_str())
                || input.opportunities[index].start_seconds < node.decision_time_seconds
            {
                add_error(&mut errors, format!("observation_opportunity_availability_invalid:{}:{opportunity_id}", node.node_id));
            }
            available_mask |= 1_u16 << index;
        }
        let (expected_observation, expected_information) = match observation_digests(node) {
            Ok(value) => value,
            Err(error) => {
                add_error(&mut errors, error);
                continue;
            }
        };
        if node.observation_digest != expected_observation
            || node.information_set_digest != expected_information
        {
            add_error(&mut errors, format!("observation_or_information_digest_mismatch:{}", node.node_id));
        }
        let group = InformationSet {
            parent_digest: node.parent_information_set_digest.clone(),
            decision_time_seconds: node.decision_time_seconds,
            revealed_mask,
            available_mask,
            weather_state: node.weather_state,
            permission_state: node.permission_state,
        };
        if let Some(existing) = groups.get(&node.information_set_digest) {
            if existing.parent_digest != group.parent_digest
                || existing.decision_time_seconds != group.decision_time_seconds
                || existing.revealed_mask != group.revealed_mask
                || existing.available_mask != group.available_mask
                || existing.weather_state != group.weather_state
                || existing.permission_state != group.permission_state
            {
                add_error(&mut errors, format!("information_set_digest_collision_or_observation_mismatch:{}", node.information_set_digest));
            }
        } else {
            groups.insert(node.information_set_digest.clone(), group);
        }
    }

    let mut scenario_ids = BTreeSet::new();
    let mut probability_total = 0_u64;
    let mut terminal_scenarios: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut root_digest: Option<String> = None;
    let mut scenario_digests = Vec::new();
    for (scenario_index, scenario) in input.scenarios.iter().enumerate() {
        if scenario.scenario_id.trim().is_empty()
            || !scenario_ids.insert(scenario.scenario_id.as_str())
            || scenario.probability_units == 0
            || scenario.node_path.is_empty()
        {
            add_error(&mut errors, format!("scenario_identity_probability_or_path_invalid:{}", scenario.scenario_id));
            continue;
        }
        probability_total += u64::from(scenario.probability_units);
        let mut previous_digest: Option<String> = None;
        let mut seen_path_nodes = BTreeSet::new();
        let mut path_digests = Vec::new();
        for node_id in &scenario.node_path {
            let Some(node) = node_by_id.get(node_id) else {
                add_error(&mut errors, format!("scenario_references_unknown_node:{}:{node_id}", scenario.scenario_id));
                continue;
            };
            if !seen_path_nodes.insert(node_id.as_str())
                || node.parent_information_set_digest != previous_digest
                || (previous_digest.is_some()
                    && groups
                        .get(&node.information_set_digest)
                        .is_some_and(|current| {
                            groups.get(previous_digest.as_ref().unwrap()).is_none_or(|parent| {
                                parent.decision_time_seconds >= current.decision_time_seconds
                            })
                        }))
            {
                add_error(&mut errors, format!("scenario_path_parent_time_or_cycle_invalid:{}", scenario.scenario_id));
            }
            if previous_digest.is_none() {
                if let Some(existing_root) = &root_digest {
                    if existing_root != &node.information_set_digest {
                        add_error(&mut errors, "scenario_roots_do_not_share_initial_information_set");
                    }
                } else {
                    root_digest = Some(node.information_set_digest.clone());
                }
            }
            previous_digest = Some(node.information_set_digest.clone());
            path_digests.push(node.information_set_digest.clone());
        }
        if let Some(leaf) = path_digests.last() {
            terminal_scenarios
                .entry(leaf.clone())
                .or_default()
                .push(scenario_index);
        }
        scenario_digests.push(path_digests);
    }
    if probability_total != PROBABILITY_SCALE {
        add_error(&mut errors, "scenario_probability_units_must_sum_to_one_million");
    }

    let mut children: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for group in groups.values() {
        if let Some(parent) = &group.parent_digest {
            if let Some(parent_group) = groups.get(parent) {
                if parent_group.decision_time_seconds >= group.decision_time_seconds
                    || group.revealed_mask & parent_group.revealed_mask != parent_group.revealed_mask
                {
                    add_error(
                        &mut errors,
                        format!("information_set_history_not_monotonic:{}", group.digest),
                    );
                }
                children
                    .entry(parent.clone())
                    .or_default()
                    .push(group.digest.clone());
            } else {
                add_error(&mut errors, format!("information_set_parent_missing:{}", group.digest));
            }
        }
    }
    for child_list in children.values_mut() {
        child_list.sort();
        child_list.dedup();
    }
    let referenced: BTreeSet<_> = scenario_digests
        .iter()
        .flat_map(|path| path.iter().cloned())
        .collect();
    if referenced.len() != groups.len()
        || groups.values().any(|group| !referenced.contains(&group.digest))
    {
        add_error(&mut errors, "unreferenced_or_disconnected_information_set");
    }
    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let root = root_digest.ok_or_else(|| vec!["scenario_tree_root_missing".to_owned()])?;
    let initial_state = SearchState {
        assigned_opportunity_by_request: vec![None; input.requests.len()],
        used_opportunity_mask: 0,
        revealed_mask: 0,
    };
    let mut context = SearchContext {
        transitions: 0,
        limit_reached: false,
    };
    let solution = solve_information_set(
        &root,
        &groups,
        &children,
        &terminal_scenarios,
        &input.scenarios,
        &input.requests,
        &input.opportunities,
        input.horizon_seconds,
        &initial_state,
        &mut context,
    );

    if context.limit_reached || solution.is_none() {
        return Ok(NonanticipativeOracleReport {
            schema_version: input.schema_version.clone(),
            benchmark_id: input.benchmark_id.clone(),
            status: OracleStatus::TransitionLimitReached,
            request_count: input.requests.len(),
            opportunity_count: input.opportunities.len(),
            scenario_count: input.scenarios.len(),
            information_set_count: groups.len(),
            search_transition_limit: MAX_SEARCH_TRANSITIONS,
            search_transitions: context.transitions,
            information_model: "nonanticipative_information_set_policy",
            deployable_online_policy: false,
            nonanticipativity_verified: false,
            nonanticipative_score: None,
            clairvoyant_perfect_information_score: None,
            value_of_perfect_information: None,
            policy: Vec::new(),
            scenario_results: Vec::new(),
            dispatch_authority_granted: false,
            flight_authority_granted: false,
            physical_mutation_authority_granted: false,
            claim_ceiling: CLAIM_CEILING,
        });
    }
    let solution = solution.expect("checked above");
    let nonanticipative_score = solution.score.public();

    let mut scenario_results = Vec::new();
    let mut clairvoyant_scores = Vec::new();
    let mut clairvoyant_transitions = 0_usize;
    for (scenario_index, scenario) in input.scenarios.iter().enumerate() {
        let mut state = SearchState {
            assigned_opportunity_by_request: vec![None; input.requests.len()],
            used_opportunity_mask: 0,
            revealed_mask: 0,
        };
        let decisions: BTreeMap<_, _> = solution
            .decisions
            .iter()
            .map(|decision| (decision.information_set_digest.as_str(), decision))
            .collect();
        for digest in &scenario_digests[scenario_index] {
            let Some(decision) = decisions.get(digest.as_str()) else {
                continue;
            };
            let Some(group) = groups.get(digest) else {
                continue;
            };
            state.revealed_mask = group.revealed_mask;
            for assignment in &decision.assignments {
                let request_index = *request_ids.get(assignment.request_id.as_str()).expect("validated request");
                let opportunity_index = *opportunity_ids.get(assignment.opportunity_id.as_str()).expect("validated opportunity");
                state.assigned_opportunity_by_request[request_index] = Some(opportunity_index);
                state.used_opportunity_mask |= 1_u16 << opportunity_index;
            }
        }
        let score = scenario_score(&state, &input.requests, &input.opportunities, input.horizon_seconds);
        scenario_results.push(ScenarioResult {
            scenario_id: scenario.scenario_id.clone(),
            probability_units: scenario.probability_units,
            score: score.clone(),
        });
        let clairvoyant = clairvoyant_scenario_score(
            scenario,
            &node_by_id,
            &input.requests,
            &input.opportunities,
            input.horizon_seconds,
            &mut clairvoyant_transitions,
        )
        .map_err(|_| vec!["clairvoyant_oracle_transition_limit_reached".to_owned()])?;
        clairvoyant_scores.push(clairvoyant);
    }

    let clairvoyant_score = expected_score_from_scenarios(&input.scenarios, &clairvoyant_scores);
    let value_of_perfect_information =
        value_of_perfect_information(&clairvoyant_score, &nonanticipative_score);

    Ok(NonanticipativeOracleReport {
        schema_version: input.schema_version.clone(),
        benchmark_id: input.benchmark_id.clone(),
        status: OracleStatus::Evaluated,
        request_count: input.requests.len(),
        opportunity_count: input.opportunities.len(),
        scenario_count: input.scenarios.len(),
        information_set_count: groups.len(),
        search_transition_limit: MAX_SEARCH_TRANSITIONS,
        search_transitions: context.transitions.saturating_add(clairvoyant_transitions),
        information_model: "nonanticipative_information_set_policy",
        deployable_online_policy: false,
        nonanticipativity_verified: true,
        nonanticipative_score: Some(nonanticipative_score),
        clairvoyant_perfect_information_score: Some(clairvoyant_score.clone()),
        value_of_perfect_information: Some(value_of_perfect_information),
        policy: solution.decisions,
        scenario_results,
        dispatch_authority_granted: false,
        flight_authority_granted: false,
        physical_mutation_authority_granted: false,
        claim_ceiling: CLAIM_CEILING,
    })
}

fn add_error(errors: &mut Vec<String>, message: impl Into<String>) {
    errors.push(message.into());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> NonanticipativeOracleInput {
        serde_json::from_str(include_str!(
            "../fixtures/nonanticipative_service_oracle_v0_1.json"
        ))
        .expect("scenario tree fixture must parse")
    }

    fn finish_hashes(input: &mut NonanticipativeOracleInput) {
        let mut order: Vec<usize> = (0..input.observation_nodes.len()).collect();
        order.sort_by_key(|index| input.observation_nodes[*index].decision_time_seconds);
        let mut remap = BTreeMap::new();
        for index in order {
            let node = &mut input.observation_nodes[index];
            if let Some(parent) = &node.parent_information_set_digest {
                if let Some(new_parent) = remap.get(parent).cloned() {
                    node.parent_information_set_digest = Some(new_parent);
                }
            }
            let old_information_digest = node.information_set_digest.clone();
            let mut revealed = node.revealed_request_ids.clone();
            revealed.sort();
            let mut available = node.available_opportunity_ids.clone();
            available.sort();
            let payload = ObservationPayload {
                parent_information_set_digest: node.parent_information_set_digest.clone(),
                decision_time_seconds: node.decision_time_seconds,
                revealed_request_ids: revealed,
                available_opportunity_ids: available,
                weather_state: node.weather_state,
                permission_state: node.permission_state,
                weather_evidence_digest: node.weather_evidence_digest.clone(),
                permission_evidence_digest: node.permission_evidence_digest.clone(),
                flight_window_id: node.flight_window_id.clone(),
            };
            let observation = serde_json::to_vec(&payload).expect("observation JSON");
            node.observation_digest = sha256_hex(&observation);
            let info = serde_json::to_vec(&InformationSetPayload {
                parent_information_set_digest: node.parent_information_set_digest.clone(),
                observation_digest: node.observation_digest.clone(),
            })
            .expect("information set JSON");
            node.information_set_digest = sha256_hex(&info);
            remap.insert(old_information_digest, node.information_set_digest.clone());
        }
    }

    #[test]
    fn full_tree_policy_uses_shared_root_action_and_reports_perfect_information_gap() {
        let report = evaluate_nonanticipative_oracle(&fixture()).expect("bounded tree evaluates");
        assert_eq!(report.status, OracleStatus::Evaluated);
        assert!(report.nonanticipativity_verified);
        assert_eq!(report.information_set_count, 3);
        assert_eq!(report.scenario_count, 2);
        let root = report.policy.iter().find(|decision| decision.decision_time_seconds == 0).unwrap();
        assert_eq!(root.assignments, vec![PolicyAssignment {
            request_id: "request-early".to_owned(),
            opportunity_id: "slot-late".to_owned(),
        }]);
        let nonanticipative = report.nonanticipative_score.as_ref().unwrap();
        let clairvoyant = report.clairvoyant_perfect_information_score.as_ref().unwrap();
        assert_eq!(nonanticipative.expected_on_time_services_units, 1_500_000);
        assert_eq!(clairvoyant.expected_on_time_services_units, 1_500_000);
        assert_eq!(report.value_of_perfect_information.as_ref().unwrap().expected_latency_seconds_units, 8_500_000);
    }

    #[test]
    fn hidden_future_request_is_not_revealed_at_shared_root() {
        let input = fixture();
        let root = input.observation_nodes.iter()
            .filter(|node| node.decision_time_seconds == 0)
            .collect::<Vec<_>>();
        assert_eq!(root.len(), 2);
        assert_eq!(root[0].information_set_digest, root[1].information_set_digest);
        assert!(!root[0].revealed_request_ids.contains(&"request-later".to_owned()));
    }

    #[test]
    fn aerial_assignment_is_blocked_when_weather_or_permission_is_not_approved() {
        let mut input = fixture();
        input.requests[0].allowed_mode = AllowedMode::FallbackOnly;
        input.requests[1].allowed_mode = AllowedMode::AerialOnly;
        input.opportunities[0].mode = OpportunityMode::Aerial;
        input.observation_nodes[0].weather_state = WeatherState::NoGo;
        input.observation_nodes[1].weather_state = WeatherState::NoGo;
        input.observation_nodes[2].weather_state = WeatherState::NoGo;
        finish_hashes(&mut input);
        let report = evaluate_nonanticipative_oracle(&input)
            .expect("weather no-go removes the aerial action");
        assert_eq!(report.status, OracleStatus::Evaluated);
        assert!(report.policy.iter().all(|decision| {
            decision.assignments.iter().all(|assignment| assignment.request_id != "request-later")
        }));
    }

    #[test]
    fn permission_denied_blocks_aerial_assignment_after_revelation() {
        let mut input = fixture();
        input.requests[0].allowed_mode = AllowedMode::FallbackOnly;
        input.requests[1].allowed_mode = AllowedMode::AerialOnly;
        input.opportunities[0].mode = OpportunityMode::Aerial;
        input.observation_nodes[2].permission_state = PermissionState::Denied;
        finish_hashes(&mut input);
        let report = evaluate_nonanticipative_oracle(&input)
            .expect("permission denial is a valid revealed observation");
        assert!(report.policy.iter().all(|decision| {
            decision.assignments.iter().all(|assignment| assignment.request_id != "request-later")
        }));
    }

    #[test]
    fn one_root_information_set_emits_one_shared_policy_decision() {
        let report = evaluate_nonanticipative_oracle(&fixture()).expect("tree");
        let root = report.policy.iter()
            .filter(|decision| decision.decision_time_seconds == 0)
            .collect::<Vec<_>>();
        assert_eq!(root.len(), 1);
        assert_eq!(
            report.policy.iter().map(|decision| decision.information_set_digest.clone())
                .collect::<BTreeSet<_>>().len(),
            report.policy.len()
        );
    }

    #[test]
    fn same_information_set_with_different_observations_is_rejected() {
        let mut input = fixture();
        input.observation_nodes[1].weather_state = WeatherState::Unknown;
        let root_digest = input.observation_nodes[0].information_set_digest.clone();
        input.observation_nodes[1].information_set_digest = root_digest;
        let errors = evaluate_nonanticipative_oracle(&input)
            .expect_err("same info-set digest must have identical public observations");
        assert!(errors.iter().any(|error| error.contains("digest_mismatch")));
    }

    #[test]
    fn probability_mass_must_sum_to_one_million() {
        let mut input = fixture();
        input.scenarios[0].probability_units = 400_000;
        let errors = evaluate_nonanticipative_oracle(&input)
            .expect_err("probability mass is explicit and normalized");
        assert!(errors.iter().any(|error| {
            error == "scenario_probability_units_must_sum_to_one_million"
        }));
    }

    #[test]
    fn bounds_exceeded_is_explicit_not_a_partial_optimum() {
        let mut input = fixture();
        input.requests.push(input.requests[0].clone());
        input.requests.push(input.requests[0].clone());
        input.requests.push(input.requests[0].clone());
        input.requests.push(input.requests[0].clone());
        input.requests.push(input.requests[0].clone());
        let report = evaluate_nonanticipative_oracle(&input).expect("bounds are report status");
        assert_eq!(report.status, OracleStatus::NotRunBoundsExceeded);
        assert!(report.nonanticipative_score.is_none());
        assert!(report.policy.is_empty());
    }

    #[test]
    fn input_order_permutation_preserves_canonical_policy() {
        let input = fixture();
        let first = evaluate_nonanticipative_oracle(&input).expect("first report");
        let mut permuted = fixture();
        permuted.requests.reverse();
        permuted.opportunities.reverse();
        permuted.observation_nodes.reverse();
        permuted.scenarios.reverse();
        for node in &mut permuted.observation_nodes {
            node.revealed_request_ids.reverse();
            node.available_opportunity_ids.reverse();
        }
        finish_hashes(&mut permuted);
        let second = evaluate_nonanticipative_oracle(&permuted).expect("permuted report");
        assert_eq!(first.policy, second.policy);
        assert_eq!(first.nonanticipative_score, second.nonanticipative_score);
    }
}
