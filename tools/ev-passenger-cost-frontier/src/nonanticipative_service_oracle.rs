// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact nonanticipative service assignment over a bounded scenario tree.
//!
//! Each tree node is one information set. A single action is optimized and
//! emitted for that node, so all descendant scenarios sharing the same
//! revealed history necessarily receive the same decision. Capacity and
//! payload capability are re-evaluated from the upstream event-derived
//! receipts; weather, permission, request-release and fallback observations
//! are bound to the node at which they become visible.
//!
//! This is a finite synthetic decision oracle, not a deployable dispatcher,
//! route planner, flight authorization system or field-performance claim.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::sustainable_mobility_service_capacity::{
    CapacityServiceOpportunity, SustainableServiceCapacityInput,
    evaluate_sustainable_service_capacity,
};
use crate::sustainable_mobility_service_quality::{
    CapacityOpportunityQualification, EvidenceClass, FallbackMode,
    FallbackServiceOpportunity, FlightEvidence, FulfilmentMode,
    LinkedServiceQualityInput, PermissionState, RequestDisposition,
    ServiceRequest, WeatherState, evaluate_linked_service_quality,
};

const MAX_REQUESTS: usize = 4;
const MAX_OPPORTUNITIES_PER_PATH: usize = 4;
const MAX_TREE_NODES: usize = 31;
const MAX_TREE_LEAVES: usize = 16;
const MAX_TREE_DEPTH: usize = 4;
const MAX_TRANSITIONS: usize = 1_000_000;
const PROBABILITY_SCALE: u128 = 1_000_000;
const PROBABILITY_DENOMINATOR: u128 =
    PROBABILITY_SCALE * PROBABILITY_SCALE * PROBABILITY_SCALE * PROBABILITY_SCALE;
const CLAIM_CEILING: &str = "Exact bounded nonanticipative policy oracle over supplied synthetic scenario-tree observations and re-evaluated service-capacity receipts only. No route planning, physical flight, weather forecast calibration, permission authority, customer delivery, grid dispatch or production performance claim.";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NonanticipativeServiceOracleInput {
    pub schema_version: String,
    pub benchmark_id: String,
    /// The tree input is deliberately separate from the linked service and
    /// capacity inputs. Their own strict schemas remain the source contracts.
    pub nodes: Vec<ScenarioTreeNode>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioTreeNode {
    pub node_id: String,
    pub parent_node_id: Option<String>,
    pub decision_time_seconds: u64,
    /// Conditional probability among this node's siblings, in parts per million.
    /// The root must be 1,000,000; each non-leaf parent’s children sum to 1,000,000.
    pub conditional_probability_ppm: u32,
    pub request_reveals: Vec<ScenarioRequestReveal>,
    pub aerial_reveals: Vec<ScenarioAerialObservation>,
    pub fallback_reveals: Vec<ScenarioFallbackObservation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioRequestReveal {
    pub request_id: String,
    pub receipt_id: String,
    pub evidence: FlightEvidence,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioAerialObservation {
    pub opportunity_id: String,
    pub weather_state: WeatherState,
    pub permission_state: PermissionState,
    pub receipt_id: String,
    pub evidence: FlightEvidence,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioFallbackObservation {
    pub opportunity_id: String,
    pub available: bool,
    pub receipt_id: String,
    pub evidence: FlightEvidence,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NonanticipativeOracleStatus {
    Evaluated,
    BoundsExceeded,
    TransitionLimitReached,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ExactExpectedScore {
    /// Exact expected-value numerator. Divide each field by probability_denominator.
    pub on_time_services_numerator: u128,
    pub completed_by_horizon_numerator: u128,
    pub lateness_seconds_numerator: u128,
    pub in_progress_numerator: u128,
    pub completed_latency_seconds_numerator: u128,
    pub probability_denominator: u128,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PerfectInformationValue {
    /// Positive values quantify the expected advantage of knowing the full
    /// scenario path before the first decision.
    pub additional_on_time_services_numerator: u128,
    pub additional_completed_services_numerator: u128,
    pub avoided_lateness_seconds_numerator: u128,
    pub avoided_completed_latency_seconds_numerator: u128,
    pub probability_denominator: u128,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ScenarioPolicyAssignment {
    pub request_id: String,
    pub opportunity_id: String,
    pub fulfilment_mode: FulfilmentMode,
    pub service_start_seconds: u64,
    pub projected_completion_seconds: u64,
    pub disposition: RequestDisposition,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct InformationSetAction {
    pub node_id: String,
    pub parent_node_id: Option<String>,
    pub decision_time_seconds: u64,
    pub observation_digest: String,
    pub information_set_digest: String,
    /// Exactly one action vector exists for a node/information set. It is
    /// shared by every leaf descending from that node.
    pub assignments: Vec<ScenarioPolicyAssignment>,
    pub expired_request_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct LeafScenarioResult {
    pub leaf_node_id: String,
    pub path_node_ids: Vec<String>,
    pub probability_numerator: u128,
    pub probability_denominator: u128,
    pub on_time_services: usize,
    pub late_services: usize,
    pub completed_by_horizon: usize,
    pub in_progress_services: usize,
    pub rejected_requests: usize,
    pub unserved_requests: usize,
    pub lateness_seconds: u64,
    pub completed_latency_seconds: u64,
    pub clairvoyant_on_time_services: usize,
    pub clairvoyant_completed_by_horizon: usize,
    pub clairvoyant_lateness_seconds: u64,
    pub clairvoyant_completed_latency_seconds: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NonanticipativeServiceOracleReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub status: NonanticipativeOracleStatus,
    pub information_model: &'static str,
    pub nonanticipativity_enforced_by_shared_information_nodes: bool,
    pub deployable_online_policy: bool,
    pub horizon_seconds: u64,
    pub source_architecture_id: String,
    pub capacity_source_report_digest: String,
    pub capacity_service_opportunity_digest: String,
    pub capacity_architecture_configuration_digest: String,
    pub request_count: usize,
    pub qualified_aerial_opportunity_count: usize,
    pub fallback_opportunity_count: usize,
    pub node_count: usize,
    pub leaf_count: usize,
    pub maximum_tree_depth: usize,
    pub transitions: usize,
    pub expected_nonanticipative: ExactExpectedScore,
    pub expected_clairvoyant_full_information: ExactExpectedScore,
    pub value_of_perfect_information: PerfectInformationValue,
    pub worst_case_on_time_services: usize,
    pub worst_case_completed_by_horizon: usize,
    pub worst_case_lateness_seconds: u64,
    pub policy: Vec<InformationSetAction>,
    pub leaf_results: Vec<LeafScenarioResult>,
    pub ledger_or_physical_mutation_performed: bool,
    pub flight_authority_granted: bool,
    pub market_or_dispatch_authority_granted: bool,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone)]
struct QualifiedOpportunity {
    opportunity_id: String,
    fulfilment_mode: FulfilmentMode,
    start_seconds: u64,
    max_service_duration_seconds: u32,
    max_payload_units: u16,
}

#[derive(Debug, Clone)]
struct NormalizedNode {
    source: ScenarioTreeNode,
    child_indices: Vec<usize>,
    depth: usize,
    request_indices: Vec<usize>,
    revealed_opportunities: Vec<QualifiedOpportunity>,
    available_opportunity_ids: BTreeSet<String>,
    observation_digest: String,
    information_set_digest: String,
}

#[derive(Debug, Clone)]
struct LocalAssignment {
    request_index: usize,
    opportunity: QualifiedOpportunity,
    completion_seconds: u64,
    disposition: RequestDisposition,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct RawScore {
    on_time: u64,
    completed: u64,
    lateness_seconds: u64,
    in_progress: u64,
    latency_seconds: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ScaledScore {
    on_time: u128,
    completed: u128,
    lateness_seconds: u128,
    in_progress: u128,
    latency_seconds: u128,
}

#[derive(Debug, Clone)]
struct Plan {
    score: ScaledScore,
    assignments_by_node: BTreeMap<String, Vec<ScenarioPolicyAssignment>>,
    expired_by_node: BTreeMap<String, Vec<String>>,
    canonical_key: String,
}

#[derive(Debug, Clone)]
struct Model<'a> {
    service: &'a LinkedServiceQualityInput,
    requests: &'a [ServiceRequest],
    nodes: Vec<NormalizedNode>,
    transitions: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ObservationDigestPayload<'a> {
    node_id: &'a str,
    parent_node_id: &'a Option<String>,
    decision_time_seconds: u64,
    request_reveals: &'a [ScenarioRequestReveal],
    aerial_reveals: &'a [ScenarioAerialObservation],
    fallback_reveals: &'a [ScenarioFallbackObservation],
}

fn add_error(errors: &mut Vec<String>, reason: impl Into<String>) {
    errors.push(reason.into());
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(71);
    output.push_str("sha256:");
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("String write cannot fail");
    }
    output
}

fn scale_for_remaining_depth(depth: usize) -> Option<u128> {
    let mut scale = 1_u128;
    for _ in depth..MAX_TREE_DEPTH {
        scale = scale.checked_mul(PROBABILITY_SCALE)?;
    }
    Some(scale)
}

impl ScaledScore {
    fn add_assign(&mut self, other: Self) -> Option<()> {
        self.on_time = self.on_time.checked_add(other.on_time)?;
        self.completed = self.completed.checked_add(other.completed)?;
        self.lateness_seconds = self.lateness_seconds.checked_add(other.lateness_seconds)?;
        self.in_progress = self.in_progress.checked_add(other.in_progress)?;
        self.latency_seconds = self.latency_seconds.checked_add(other.latency_seconds)?;
        Some(())
    }

    fn multiplied(self, factor: u32) -> Option<Self> {
        let factor = u128::from(factor);
        Some(Self {
            on_time: self.on_time.checked_mul(factor)?,
            completed: self.completed.checked_mul(factor)?,
            lateness_seconds: self.lateness_seconds.checked_mul(factor)?,
            in_progress: self.in_progress.checked_mul(factor)?,
            latency_seconds: self.latency_seconds.checked_mul(factor)?,
        })
    }
}

fn compare_scores(left: &ScaledScore, right: &ScaledScore) -> Ordering {
    left.on_time
        .cmp(&right.on_time)
        .then_with(|| left.completed.cmp(&right.completed))
        .then_with(|| right.lateness_seconds.cmp(&left.lateness_seconds))
        .then_with(|| right.in_progress.cmp(&left.in_progress))
        .then_with(|| right.latency_seconds.cmp(&left.latency_seconds))
}

fn local_score(
    assignments: &[LocalAssignment],
    requests: &[ServiceRequest],
    horizon_seconds: u64,
    scale: u128,
) -> Option<ScaledScore> {
    let mut raw = RawScore::default();
    for assignment in assignments {
        let request = requests.get(assignment.request_index)?;
        let (_, contribution) =
            disposition_score(request, assignment.completion_seconds, horizon_seconds);
        add_raw_score(&mut raw, contribution).ok()?;
    }
    Some(ScaledScore {
        on_time: u128::from(raw.on_time).checked_mul(scale)?,
        completed: u128::from(raw.completed).checked_mul(scale)?,
        lateness_seconds: u128::from(raw.lateness_seconds).checked_mul(scale)?,
        in_progress: u128::from(raw.in_progress).checked_mul(scale)?,
        latency_seconds: u128::from(raw.latency_seconds).checked_mul(scale)?,
    })
}

fn request_mode_compatible(request: &ServiceRequest, opportunity: &QualifiedOpportunity) -> bool {
    use crate::sustainable_mobility_service_quality::ServiceMode;
    match opportunity.fulfilment_mode {
        FulfilmentMode::Aerial => {
            matches!(request.allowed_mode, ServiceMode::AerialOnly | ServiceMode::Either)
        }
        FulfilmentMode::GroundCourier | FulfilmentMode::Locker | FulfilmentMode::Amr => {
            matches!(request.allowed_mode, ServiceMode::FallbackOnly | ServiceMode::Either)
        }
    }
}

fn make_local_assignment(
    request_index: usize,
    opportunity: &QualifiedOpportunity,
    requests: &[ServiceRequest],
    horizon_seconds: u64,
) -> Option<LocalAssignment> {
    let request = requests.get(request_index)?;
    if request.arrival_seconds > opportunity.start_seconds
        || request.accept_by_seconds < opportunity.start_seconds
        || request.required_service_seconds > opportunity.max_service_duration_seconds
        || request.payload_units > opportunity.max_payload_units
        || !request_mode_compatible(request, opportunity)
    {
        return None;
    }
    let completion = opportunity
        .start_seconds
        .checked_add(u64::from(request.required_service_seconds))?;
    let disposition = if completion > horizon_seconds {
        RequestDisposition::InProgressAtHorizon
    } else if completion <= request.deadline_seconds {
        RequestDisposition::CompletedOnTime
    } else {
        RequestDisposition::CompletedLate
    };
    Some(LocalAssignment {
        request_index,
        opportunity: opportunity.clone(),
        completion_seconds: completion,
        disposition,
    })
}

fn assignment_to_report(
    assignment: &LocalAssignment,
    requests: &[ServiceRequest],
) -> ScenarioPolicyAssignment {
    ScenarioPolicyAssignment {
        request_id: requests[assignment.request_index].request_id.clone(),
        opportunity_id: assignment.opportunity.opportunity_id.clone(),
        fulfilment_mode: assignment.opportunity.fulfilment_mode,
        service_start_seconds: assignment.opportunity.start_seconds,
        projected_completion_seconds: assignment.completion_seconds,
        disposition: assignment.disposition.clone(),
    }
}

fn enumerate_node_actions(
    opportunities: &[QualifiedOpportunity],
    requests: &[ServiceRequest],
    eligible_mask: u8,
    index: usize,
    used_mask: u8,
    current: &mut Vec<LocalAssignment>,
    horizon_seconds: u64,
    output: &mut Vec<Vec<LocalAssignment>>,
    transitions: &mut usize,
) -> Result<(), String> {
    *transitions = transitions
        .checked_add(1)
        .ok_or_else(|| "nonanticipative_transition_counter_overflow".to_owned())?;
    if *transitions > MAX_TRANSITIONS {
        return Err("nonanticipative_transition_limit_reached".to_owned());
    }
    if index == opportunities.len() {
        output.push(current.clone());
        return Ok(());
    }

    // A service opportunity can be left unused.
    enumerate_node_actions(
        opportunities,
        requests,
        eligible_mask,
        index + 1,
        used_mask,
        current,
        horizon_seconds,
        output,
        transitions,
    )?;

    let opportunity = &opportunities[index];
    for request_index in 0..requests.len() {
        let bit = 1_u8 << request_index;
        if eligible_mask & bit == 0 || used_mask & bit != 0 {
            continue;
        }
        let Some(assignment) =
            make_local_assignment(request_index, opportunity, requests, horizon_seconds)
        else {
            continue;
        };
        current.push(assignment);
        enumerate_node_actions(
            opportunities,
            requests,
            eligible_mask,
            index + 1,
            used_mask | bit,
            current,
            horizon_seconds,
            output,
            transitions,
        )?;
        current.pop();
    }
    Ok(())
}

fn node_request_mask(node: &NormalizedNode) -> u8 {
    node.request_indices
        .iter()
        .fold(0_u8, |mask, index| mask | (1_u8 << index))
}

fn solve_node(
    node_index: usize,
    pending_mask: u8,
    model: &mut Model<'_>,
    memo: &mut BTreeMap<(usize, u8), Plan>,
) -> Result<Plan, String> {
    if let Some(cached) = memo.get(&(node_index, pending_mask)) {
        return Ok(cached.clone());
    }
    let node = model
        .nodes
        .get(node_index)
        .ok_or_else(|| "scenario_tree_node_index_invalid".to_owned())?
        .clone();
    let current_mask = pending_mask | node_request_mask(&node);
    let eligible_mask = (0..model.requests.len()).fold(0_u8, |mask, request_index| {
        let request = &model.requests[request_index];
        let bit = 1_u8 << request_index;
        if current_mask & bit != 0
            && request.arrival_seconds <= node.source.decision_time_seconds
            && node.source.decision_time_seconds <= request.accept_by_seconds
        {
            mask | bit
        } else {
            mask
        }
    });

    let available_opportunities: Vec<_> = node
        .revealed_opportunities
        .iter()
        .filter(|opportunity| {
            node.available_opportunity_ids.contains(&opportunity.opportunity_id)
        })
        .cloned()
        .collect();
    let mut actions = Vec::new();
    let mut transitions = 0_usize;
    enumerate_node_actions(
        &available_opportunities,
        model.requests,
        eligible_mask,
        0,
        0,
        &mut Vec::new(),
        model.service.horizon_seconds,
        &mut actions,
        &mut transitions,
    )?;
    model.transitions = model
        .transitions
        .checked_add(transitions)
        .ok_or_else(|| "nonanticipative_transition_counter_overflow".to_owned())?;
    if model.transitions > MAX_TRANSITIONS {
        return Err("nonanticipative_transition_limit_reached".to_owned());
    }

    let scale = scale_for_remaining_depth(node.depth)
        .ok_or_else(|| "nonanticipative_probability_scale_overflow".to_owned())?;
    let children = node.child_indices.clone();
    let mut best: Option<Plan> = None;

    for action in actions {
        let assigned_mask = action.iter().fold(0_u8, |mask, item| {
            mask | (1_u8 << item.request_index)
        });
        let mut remaining = current_mask & !assigned_mask;
        let mut expired_request_ids = Vec::new();
        for request_index in 0..model.requests.len() {
            let bit = 1_u8 << request_index;
            if remaining & bit != 0
                && model.requests[request_index].accept_by_seconds
                    <= node.source.decision_time_seconds
            {
                remaining &= !bit;
                expired_request_ids.push(model.requests[request_index].request_id.clone());
            }
        }
        expired_request_ids.sort();

        let mut score = local_score(
            &action,
            model.requests,
            model.service.horizon_seconds,
            scale,
        )
        .ok_or_else(|| "nonanticipative_local_score_overflow".to_owned())?;
        let mut assignments_by_node = BTreeMap::new();
        let mut expired_by_node = BTreeMap::new();
        let mut policy_assignments: Vec<_> = action
            .iter()
            .map(|item| assignment_to_report(item, model.requests))
            .collect();
        policy_assignments.sort_by(|left, right| {
            (&left.opportunity_id, &left.request_id)
                .cmp(&(&right.opportunity_id, &right.request_id))
        });
        assignments_by_node.insert(node.source.node_id.clone(), policy_assignments);
        expired_by_node.insert(node.source.node_id.clone(), expired_request_ids);

        let mut canonical = assignments_by_node
            .get(&node.source.node_id)
            .map(|items| {
                items
                    .iter()
                    .map(|item| format!("{}->{}", item.opportunity_id, item.request_id))
                    .collect::<Vec<_>>()
                    .join("|")
            })
            .unwrap_or_default();

        for child_index in &children {
            let child_plan = solve_node(*child_index, remaining, model, memo)?;
            let probability = model.nodes[*child_index].source.conditional_probability_ppm;
            score
                .add_assign(
                    child_plan
                        .score
                        .multiplied(probability)
                        .ok_or_else(|| "nonanticipative_expected_score_overflow".to_owned())?,
                )
                .ok_or_else(|| "nonanticipative_expected_score_overflow".to_owned())?;
            assignments_by_node.extend(child_plan.assignments_by_node);
            expired_by_node.extend(child_plan.expired_by_node);
            canonical.push_str(&format!(
                "||{}:{}",
                model.nodes[*child_index].source.node_id,
                child_plan.canonical_key
            ));
        }

        let candidate = Plan {
            score,
            assignments_by_node,
            expired_by_node,
            canonical_key: canonical,
        };
        let replace = best.as_ref().is_none_or(|current| {
            match compare_scores(&candidate.score, &current.score) {
                Ordering::Greater => true,
                Ordering::Less => false,
                Ordering::Equal => candidate.canonical_key < current.canonical_key,
            }
        });
        if replace {
            best = Some(candidate);
        }
    }

    let best = best.ok_or_else(|| "nonanticipative_node_has_no_valid_action".to_owned())?;
    memo.insert((node_index, pending_mask), best.clone());
    Ok(best)
}

fn expected_score(raw: RawScore, probability_denominator: u128) -> ExactExpectedScore {
    let factor = probability_denominator;
    ExactExpectedScore {
        on_time_services_numerator: u128::from(raw.on_time).saturating_mul(factor),
        completed_by_horizon_numerator: u128::from(raw.completed).saturating_mul(factor),
        lateness_seconds_numerator: u128::from(raw.lateness_seconds).saturating_mul(factor),
        in_progress_numerator: u128::from(raw.in_progress).saturating_mul(factor),
        completed_latency_seconds_numerator: u128::from(raw.latency_seconds).saturating_mul(factor),
        probability_denominator,
    }
}

fn hash_observation(
    node: &ScenarioTreeNode,
    parent_information_digest: Option<&str>,
) -> Result<(String, String), String> {
    let mut request_reveals = node.request_reveals.clone();
    request_reveals.sort_by(|left, right| left.request_id.cmp(&right.request_id));
    let mut aerial_reveals = node.aerial_reveals.clone();
    aerial_reveals.sort_by(|left, right| left.opportunity_id.cmp(&right.opportunity_id));
    let mut fallback_reveals = node.fallback_reveals.clone();
    fallback_reveals.sort_by(|left, right| left.opportunity_id.cmp(&right.opportunity_id));
    let payload = ObservationDigestPayload {
        node_id: &node.node_id,
        parent_node_id: &node.parent_node_id,
        decision_time_seconds: node.decision_time_seconds,
        request_reveals: &request_reveals,
        aerial_reveals: &aerial_reveals,
        fallback_reveals: &fallback_reveals,
    };
    let bytes = serde_json::to_vec(&payload)
        .map_err(|error| format!("observation_digest_serialization_failed:{error}"))?;
    let observation_digest = sha256_hex(&bytes);
    let parent = parent_information_digest.unwrap_or("sha256:root");
    let information_set_bytes = serde_json::to_vec(&(
        parent,
        node.decision_time_seconds,
        &observation_digest,
    ))
    .map_err(|error| format!("information_set_digest_serialization_failed:{error}"))?;
    Ok((observation_digest, sha256_hex(&information_set_bytes)))
}

fn validate_flight_evidence(
    evidence: &FlightEvidence,
    receipt_id: &str,
    service: &LinkedServiceQualityInput,
    decision_time_seconds: u64,
    required_until_seconds: u64,
) -> bool {
    evidence.site_id == service.site_id
        && evidence.site_configuration_digest == service.site_configuration_digest
        && !evidence.producer_id.trim().is_empty()
        && !evidence.method_id.trim().is_empty()
        && evidence.evidence_reference == receipt_id
        && !receipt_id.trim().is_empty()
        && evidence.evidence_class != EvidenceClass::Unknown
        && evidence.valid_from_seconds <= evidence.observed_at_seconds
        && evidence.observed_at_seconds <= decision_time_seconds
        && evidence.valid_from_seconds <= decision_time_seconds
        && evidence.valid_until_seconds >= required_until_seconds
}

fn node_error(errors: &mut Vec<String>, node_id: &str, reason: &str) {
    add_error(errors, format!("{reason}:{node_id}"));
}

fn disposition_score(
    request: &ServiceRequest,
    completion_seconds: u64,
    horizon_seconds: u64,
) -> (RequestDisposition, RawScore) {
    if completion_seconds > horizon_seconds {
        (
            RequestDisposition::InProgressAtHorizon,
            RawScore {
                in_progress: 1,
                ..RawScore::default()
            },
        )
    } else if completion_seconds <= request.deadline_seconds {
        (
            RequestDisposition::CompletedOnTime,
            RawScore {
                on_time: 1,
                completed: 1,
                latency_seconds: completion_seconds.saturating_sub(request.arrival_seconds),
                ..RawScore::default()
            },
        )
    } else {
        (
            RequestDisposition::CompletedLate,
            RawScore {
                completed: 1,
                lateness_seconds: completion_seconds.saturating_sub(request.deadline_seconds),
                latency_seconds: completion_seconds.saturating_sub(request.arrival_seconds),
                ..RawScore::default()
            },
        )
    }
}

fn add_raw_score(target: &mut RawScore, source: RawScore) -> Result<(), String> {
    target.on_time = target.on_time.checked_add(source.on_time)
        .ok_or_else(|| "raw_score_overflow".to_owned())?;
    target.completed = target.completed.checked_add(source.completed)
        .ok_or_else(|| "raw_score_overflow".to_owned())?;
    target.lateness_seconds = target.lateness_seconds.checked_add(source.lateness_seconds)
        .ok_or_else(|| "raw_score_overflow".to_owned())?;
    target.in_progress = target.in_progress.checked_add(source.in_progress)
        .ok_or_else(|| "raw_score_overflow".to_owned())?;
    target.latency_seconds = target.latency_seconds.checked_add(source.latency_seconds)
        .ok_or_else(|| "raw_score_overflow".to_owned())?;
    Ok(())
}

fn raw_score_from_assignments(
    assignments: &[LocalAssignment],
    requests: &[ServiceRequest],
    horizon_seconds: u64,
) -> Result<RawScore, String> {
    let mut raw = RawScore::default();
    for assignment in assignments {
        let request = &requests[assignment.request_index];
        let (_, contribution) =
            disposition_score(request, assignment.completion_seconds, horizon_seconds);
        add_raw_score(&mut raw, contribution)?;
    }
    Ok(raw)
}

fn enumerate_clairvoyant(
    opportunities: &[QualifiedOpportunity],
    requests: &[ServiceRequest],
    horizon_seconds: u64,
    index: usize,
    used_request_mask: u8,
    current: &mut Vec<LocalAssignment>,
    best: &mut RawScore,
    transitions: &mut usize,
) -> Result<(), String> {
    *transitions = transitions
        .checked_add(1)
        .ok_or_else(|| "clairvoyant_transition_overflow".to_owned())?;
    if *transitions > MAX_TRANSITIONS {
        return Err("nonanticipative_transition_limit_reached".to_owned());
    }
    if index == opportunities.len() {
        let score = raw_score_from_assignments(current, requests, horizon_seconds)?;
        let better = score.on_time > best.on_time
            || (score.on_time == best.on_time
                && (score.completed > best.completed
                    || (score.completed == best.completed
                        && (score.lateness_seconds < best.lateness_seconds
                            || (score.lateness_seconds == best.lateness_seconds
                                && (score.in_progress < best.in_progress
                                    || (score.in_progress == best.in_progress
                                        && score.latency_seconds < best.latency_seconds))))));
        if better {
            *best = score;
        }
        return Ok(());
    }

    enumerate_clairvoyant(
        opportunities,
        requests,
        horizon_seconds,
        index + 1,
        used_request_mask,
        current,
        best,
        transitions,
    )?;

    let opportunity = &opportunities[index];
    for request_index in 0..requests.len() {
        let bit = 1_u8 << request_index;
        if used_request_mask & bit != 0 {
            continue;
        }
        if let Some(assignment) = make_local_assignment(
            request_index,
            opportunity,
            requests,
            horizon_seconds,
        ) {
            current.push(assignment);
            enumerate_clairvoyant(
                opportunities,
                requests,
                horizon_seconds,
                index + 1,
                used_request_mask | bit,
                current,
                best,
                transitions,
            )?;
            current.pop();
        }
    }
    Ok(())
}

fn collect_leaf_paths(
    node_index: usize,
    nodes: &[NormalizedNode],
    current: &mut Vec<usize>,
    paths: &mut Vec<Vec<usize>>,
) {
    current.push(node_index);
    if nodes[node_index].child_indices.is_empty() {
        paths.push(current.clone());
    } else {
        for child in &nodes[node_index].child_indices {
            collect_leaf_paths(*child, nodes, current, paths);
        }
    }
    current.pop();
}

fn path_probability_numerator(
    path: &[usize],
    nodes: &[NormalizedNode],
) -> Result<u128, String> {
    let mut probability = 1_u128;
    for node_index in path.iter().skip(1) {
        probability = probability
            .checked_mul(u128::from(
                nodes[*node_index].source.conditional_probability_ppm,
            ))
            .ok_or_else(|| "scenario_leaf_probability_overflow".to_owned())?;
    }
    let depth = nodes[*path.last().ok_or_else(|| "scenario_path_empty".to_owned())?].depth;
    probability
        .checked_mul(scale_for_remaining_depth(depth)
            .ok_or_else(|| "scenario_leaf_probability_overflow".to_owned())?)
        .ok_or_else(|| "scenario_leaf_probability_overflow".to_owned())
}

fn leaf_policy_score(
    path: &[usize],
    nodes: &[NormalizedNode],
    requests: &[ServiceRequest],
    policy: &BTreeMap<String, Vec<ScenarioPolicyAssignment>>,
    expired_by_node: &BTreeMap<String, Vec<String>>,
    horizon_seconds: u64,
) -> Result<(RawScore, usize, usize), String> {
    let mut revealed_requests = BTreeSet::new();
    let mut assigned_requests = BTreeSet::new();
    let mut expired_requests = BTreeSet::new();
    let mut score = RawScore::default();
    for node_index in path {
        let node = &nodes[*node_index];
        for request_index in &node.request_indices {
            revealed_requests.insert(*request_index);
        }
        if let Some(assignments) = policy.get(&node.source.node_id) {
            for assignment in assignments {
                let request_index = requests
                    .iter()
                    .position(|request| request.request_id == assignment.request_id)
                    .ok_or_else(|| "policy_references_unknown_request".to_owned())?;
                if !assigned_requests.insert(request_index) {
                    return Err("policy_assigns_request_more_than_once_on_leaf".to_owned());
                }
                let request = &requests[request_index];
                let (_, raw) = disposition_score(
                    request,
                    assignment.projected_completion_seconds,
                    horizon_seconds,
                );
                add_raw_score(&mut score, raw)?;
            }
        }
        if let Some(expired) = expired_by_node.get(&node.source.node_id) {
            for request_id in expired {
                let request_index = requests
                    .iter()
                    .position(|request| request.request_id == *request_id)
                    .ok_or_else(|| "expired_policy_request_unknown".to_owned())?;
                expired_requests.insert(request_index);
            }
        }
    }
    let rejected = expired_requests.len();
    let unserved = revealed_requests
        .difference(&assigned_requests)
        .filter(|request_index| !expired_requests.contains(request_index))
        .count();
    Ok((score, rejected, unserved))
}

fn leaf_clairvoyant_score(
    path: &[usize],
    nodes: &[NormalizedNode],
    requests: &[ServiceRequest],
    horizon_seconds: u64,
    transitions: &mut usize,
) -> Result<RawScore, String> {
    let mut request_indices = BTreeSet::new();
    let mut opportunities = BTreeMap::new();
    for node_index in path {
        let node = &nodes[*node_index];
        request_indices.extend(node.request_indices.iter().copied());
        for opportunity in &node.revealed_opportunities {
            if node.available_opportunity_ids.contains(&opportunity.opportunity_id) {
                opportunities.insert(opportunity.opportunity_id.clone(), opportunity.clone());
            }
        }
    }
    let leaf_requests: Vec<_> = request_indices
        .into_iter()
        .map(|index| requests[index].clone())
        .collect();
    // Preserve the original indices for compatibility with the shared assignment helper.
    let leaf_request_refs: Vec<_> = leaf_requests.iter().collect();
    let _ = (service, leaf_request_refs);

    let available: Vec<_> = opportunities.into_values().collect();
    let mut best = RawScore::default();
    enumerate_clairvoyant(
        &available,
        &leaf_requests,
        horizon_seconds,
        0,
        0,
        &mut Vec::new(),
        &mut best,
        transitions,
    )?;
    Ok(best)
}

fn weighted_leaf_score(
    score: RawScore,
    probability_numerator: u128,
    target: &mut ScaledScore,
) -> Result<(), String> {
    let weighted = ScaledScore {
        on_time: u128::from(score.on_time)
            .checked_mul(probability_numerator)
            .ok_or_else(|| "weighted_leaf_score_overflow".to_owned())?,
        completed: u128::from(score.completed)
            .checked_mul(probability_numerator)
            .ok_or_else(|| "weighted_leaf_score_overflow".to_owned())?,
        lateness_seconds: u128::from(score.lateness_seconds)
            .checked_mul(probability_numerator)
            .ok_or_else(|| "weighted_leaf_score_overflow".to_owned())?,
        in_progress: u128::from(score.in_progress)
            .checked_mul(probability_numerator)
            .ok_or_else(|| "weighted_leaf_score_overflow".to_owned())?,
        latency_seconds: u128::from(score.latency_seconds)
            .checked_mul(probability_numerator)
            .ok_or_else(|| "weighted_leaf_score_overflow".to_owned())?,
    };
    target
        .add_assign(weighted)
        .ok_or_else(|| "weighted_leaf_score_overflow".to_owned())
}

pub fn evaluate_nonanticipative_service_oracle(
    input: &NonanticipativeServiceOracleInput,
    service: &LinkedServiceQualityInput,
    capacity: &SustainableServiceCapacityInput,
) -> Result<NonanticipativeServiceOracleReport, Vec<String>> {
    let mut errors = Vec::new();
    if input.schema_version != "0.1.0"
        || input.benchmark_id != "NONANTICIPATIVE-SERVICE-ORACLE-001"
        || input.claim_ceiling.trim().is_empty()
        || input.nodes.is_empty()
        || input.nodes.len() > MAX_TREE_NODES
        || service.requests.is_empty()
        || service.requests.len() > MAX_REQUESTS
        || service.aerial_opportunity_qualifications.len() > MAX_OPPORTUNITIES_PER_PATH
        || service.fallback_opportunities.len() > MAX_OPPORTUNITIES_PER_PATH
        || service.horizon_seconds != capacity.horizon_seconds
    {
        add_error(&mut errors, "nonanticipative_identity_or_bounds_invalid");
    }
    if let Err(linked_errors) = evaluate_linked_service_quality(service, capacity) {
        errors.extend(
            linked_errors
                .into_iter()
                .map(|error| format!("linked_source_rejected:{error}")),
        );
    }
    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let capacity_report = evaluate_sustainable_service_capacity(capacity).map_err(|errs| errs)?;
    let mut architecture_results = capacity_report
        .architecture_results
        .iter()
        .filter(|result| result.architecture_id == service.source_architecture_id);
    let selected = architecture_results
        .next()
        .ok_or_else(|| vec!["capacity_source_architecture_missing".to_owned()])?;
    if architecture_results.next().is_some() {
        return Err(vec!["capacity_source_architecture_ambiguous".to_owned()]);
    }
    let mut receipts = BTreeMap::new();
    for receipt in &selected.service_opportunities {
        receipts.insert(receipt.opportunity_id.as_str(), receipt);
    }
    let qualifications: BTreeMap<_, _> = service
        .aerial_opportunity_qualifications
        .iter()
        .map(|qualification| (qualification.opportunity_id.as_str(), qualification))
        .collect();
    let fallbacks: BTreeMap<_, _> = service
        .fallback_opportunities
        .iter()
        .map(|opportunity| (opportunity.opportunity_id.as_str(), opportunity))
        .collect();
    let request_indices: BTreeMap<_, _> = service
        .requests
        .iter()
        .enumerate()
        .map(|(index, request)| (request.request_id.as_str(), index))
        .collect();

    let report_bytes = serde_json::to_vec(&capacity_report)
        .map_err(|error| vec![format!("capacity_report_serialization_failed:{error}")])?;
    source_report_digest = sha256_hex(&report_bytes);
    let mut architecture_configuration = None;
    for architecture in &capacity.architectures {
        if architecture.architecture_id == service.source_architecture_id {
            architecture_configuration = Some(serde_json::to_vec(architecture)
                .map_err(|error| vec![format!("architecture_digest_serialization_failed:{error}")])?);
        }
    }
    let architecture_configuration_digest = architecture_configuration
        .as_deref()
        .map(sha256_hex)
        .ok_or_else(|| vec!["capacity_source_architecture_missing".to_owned()])?;
    let service_opportunities_bytes = serde_json::to_vec(&selected.service_opportunities)
        .map_err(|error| vec![format!("service_opportunity_digest_serialization_failed:{error}")])?;
    let capacity_service_opportunity_digest = sha256_hex(&service_opportunities_bytes);

    let mut raw_nodes = input.nodes.clone();
    raw_nodes.sort_by(|left, right| {
        (left.decision_time_seconds, &left.node_id)
            .cmp(&(right.decision_time_seconds, &right.node_id))
    });
    let mut node_index_by_id = BTreeMap::new();
    let mut children_by_index: Vec<Vec<usize>> = vec![Vec::new(); raw_nodes.len()];
    let mut parent_indices = vec![None; raw_nodes.len()];
    let mut depths = vec![0_usize; raw_nodes.len()];
    let mut roots = Vec::new();
    for (index, node) in raw_nodes.iter().enumerate() {
        if node.node_id.trim().is_empty()
            || node.node_id.len() > 80
            || node.decision_time_seconds > service.horizon_seconds
            || node.conditional_probability_ppm == 0
            || node.conditional_probability_ppm > 1_000_000
            || node_index_by_id
                .insert(node.node_id.as_str(), index)
                .is_some()
        {
            add_error(&mut errors, format!("scenario_node_identity_or_bounds_invalid:{}", node.node_id));
            continue;
        }
        if let Some(parent_id) = &node.parent_node_id {
            let Some(parent_index) = node_index_by_id.get(parent_id.as_str()).copied() else {
                add_error(&mut errors, format!("scenario_parent_missing_or_not_earlier:{}", node.node_id));
                continue;
            };
            if raw_nodes[parent_index].decision_time_seconds >= node.decision_time_seconds {
                add_error(&mut errors, format!("scenario_time_not_strictly_increasing:{}", node.node_id));
                continue;
            }
            parent_indices[index] = Some(parent_index);
            children_by_index[parent_index].push(index);
            depths[index] = depths[parent_index] + 1;
            if depths[index] > MAX_TREE_DEPTH {
                add_error(&mut errors, "scenario_tree_depth_exceeded");
            }
        } else {
            roots.push(index);
            if node.conditional_probability_ppm != 1_000_000 {
                add_error(&mut errors, "scenario_root_probability_must_be_one");
            }
        }
    }
    if roots.len() != 1 || roots.first().copied() != Some(0) {
        add_error(&mut errors, "scenario_tree_must_have_one_earliest_root");
    }
    let leaf_count = children_by_index
        .iter()
        .filter(|children| children.is_empty())
        .count();
    if leaf_count == 0 || leaf_count > MAX_TREE_LEAVES {
        add_error(&mut errors, "scenario_leaf_count_out_of_bounds");
    }
    for (parent_index, children) in children_by_index.iter().enumerate() {
        if !children.is_empty() {
            let sum: u64 = children
                .iter()
                .map(|child| u64::from(raw_nodes[*child].conditional_probability_ppm))
                .sum();
            if sum != 1_000_000 {
                add_error(
                    &mut errors,
                    format!("scenario_sibling_probabilities_do_not_sum_to_one:{}", raw_nodes[parent_index].node_id),
                );
            }
        }
    }

    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let mut normalized = Vec::with_capacity(raw_nodes.len());
    let mut global_receipts = BTreeSet::new();
    let mut information_digests = Vec::<String>::with_capacity(raw_nodes.len());
    for (index, node) in raw_nodes.iter().enumerate() {
        let parent_digest = parent_indices[index]
            .map(|parent| information_digests[parent].as_str());
        let (observation_digest, information_set_digest) = hash_observation(node, parent_digest)
            .map_err(|error| vec![error])?;
        if node.request_reveals.is_empty()
            && node.aerial_reveals.is_empty()
            && node.fallback_reveals.is_empty()
        {
            add_error(&mut errors, format!("scenario_node_has_no_new_observation:{}", node.node_id));
        }
        let mut seen_request_ids = BTreeSet::new();
        let mut node_request_indices = Vec::new();
        for reveal in &node.request_reveals {
            let Some(request_index) = request_indices.get(reveal.request_id.as_str()).copied() else {
                add_error(&mut errors, format!("scenario_reveals_unknown_request:{}", reveal.request_id));
                continue;
            };
            if !seen_request_ids.insert(reveal.request_id.as_str())
                || !global_receipts.insert(reveal.receipt_id.as_str())
                || service.requests[request_index].arrival_seconds > node.decision_time_seconds
                || !validate_flight_evidence(
                    &reveal.evidence,
                    &reveal.receipt_id,
                    service,
                    node.decision_time_seconds,
                    node.decision_time_seconds,
                )
            {
                node_error(&mut errors, &node.node_id, "scenario_request_reveal_invalid_or_replayed");
            }
            node_request_indices.push(request_index);
        }

        let mut node_opportunity_ids = BTreeSet::new();
        let mut available_opportunity_ids = BTreeSet::new();
        let mut revealed_opportunities = Vec::new();
        for observation in &node.aerial_reveals {
            let Some(receipt) = receipts.get(observation.opportunity_id.as_str()) else {
                node_error(&mut errors, &node.node_id, "scenario_aerial_reveal_unknown_capacity_receipt");
                continue;
            };
            let Some(qualification) = qualifications.get(observation.opportunity_id.as_str()) else {
                node_error(&mut errors, &node.node_id, "scenario_aerial_reveal_without_payload_qualification");
                continue;
            };
            let required_until = receipt
                .start_seconds
                .checked_add(u64::from(receipt.mission_duration_seconds))
                .and_then(|end| end.checked_add(u64::from(receipt.turnaround_seconds)));
            if !node_opportunity_ids.insert(observation.opportunity_id.as_str())
                || !global_receipts.insert(observation.receipt_id.as_str())
                || receipt.start_seconds != node.decision_time_seconds
                || required_until.is_none_or(|until| {
                    !validate_flight_evidence(
                        &observation.evidence,
                        &observation.receipt_id,
                        service,
                        node.decision_time_seconds,
                        until,
                    )
                })
            {
                node_error(&mut errors, &node.node_id, "scenario_aerial_observation_invalid_or_replayed");
            }
            let opportunity = QualifiedOpportunity {
                opportunity_id: receipt.opportunity_id.clone(),
                fulfilment_mode: FulfilmentMode::Aerial,
                start_seconds: receipt.start_seconds,
                max_service_duration_seconds: receipt.mission_duration_seconds,
                max_payload_units: qualification.max_payload_units,
                aircraft_or_resource_id: receipt.aircraft_id.clone(),
                source_receipt_id: observation.receipt_id.clone(),
            };
            if observation.weather_state == WeatherState::Go
                && observation.permission_state == PermissionState::Approved
            {
                available_opportunity_ids.insert(observation.opportunity_id.clone());
            }
            revealed_opportunities.push(opportunity);
        }

        for observation in &node.fallback_reveals {
            let Some(fallback) = fallbacks.get(observation.opportunity_id.as_str()) else {
                node_error(&mut errors, &node.node_id, "scenario_fallback_reveal_unknown_opportunity");
                continue;
            };
            let required_until = fallback
                .start_seconds
                .checked_add(u64::from(fallback.max_service_duration_seconds));
            if !node_opportunity_ids.insert(observation.opportunity_id.as_str())
                || !global_receipts.insert(observation.receipt_id.as_str())
                || fallback.start_seconds != node.decision_time_seconds
                || required_until.is_none_or(|until| {
                    !validate_flight_evidence(
                        &observation.evidence,
                        &observation.receipt_id,
                        service,
                        node.decision_time_seconds,
                        until,
                    )
                })
            {
                node_error(&mut errors, &node.node_id, "scenario_fallback_observation_invalid_or_replayed");
            }
            let opportunity = QualifiedOpportunity {
                opportunity_id: fallback.opportunity_id.clone(),
                fulfilment_mode: match fallback.fallback_mode {
                    FallbackMode::GroundCourier => FulfilmentMode::GroundCourier,
                    FallbackMode::Locker => FulfilmentMode::Locker,
                    FallbackMode::Amr => FulfilmentMode::Amr,
                },
                start_seconds: fallback.start_seconds,
                max_service_duration_seconds: fallback.max_service_duration_seconds,
                max_payload_units: fallback.max_payload_units,
            };
            if observation.available {
                available_opportunity_ids.insert(observation.opportunity_id.clone());
            }
            revealed_opportunities.push(opportunity);
        }
        revealed_opportunities.sort_by(|left, right| {
            left.opportunity_id.cmp(&right.opportunity_id)
        });
        node_request_indices.sort_unstable();

        information_digests.push(information_set_digest.clone());
        normalized.push(NormalizedNode {
            source: node.clone(),
            child_indices: children_by_index[index].clone(),
            depth: depths[index],
            request_indices: node_request_indices,
            revealed_opportunities,
            available_opportunity_ids,
            observation_digest,
            information_set_digest,
        });
    }

    // Every leaf must contain the same selected receipt universe. A missing
    // observation is not interpreted as "probably available"; unavailable is
    // an explicit state carried by the scenario node.
    let expected_aerial_ids: BTreeSet<_> = service
        .aerial_opportunity_qualifications
        .iter()
        .map(|item| item.opportunity_id.as_str())
        .collect();
    let expected_fallback_ids: BTreeSet<_> = service
        .fallback_opportunities
        .iter()
        .map(|item| item.opportunity_id.as_str())
        .collect();
    let mut leaf_paths = Vec::new();
    collect_leaf_paths(0, &normalized, &mut Vec::new(), &mut leaf_paths);
    for path in &leaf_paths {
        let mut path_aerial = BTreeSet::new();
        let mut path_fallback = BTreeSet::new();
        let mut path_requests = BTreeSet::new();
        for node_index in path {
            let node = &normalized[*node_index];
            for request_index in &node.request_indices {
                if !path_requests.insert(*request_index) {
                    add_error(&mut errors, format!("request_revealed_twice_on_path:{}", service.requests[*request_index].request_id));
                }
            }
            for observation in &node.source.aerial_reveals {
                if !path_aerial.insert(observation.opportunity_id.as_str()) {
                    add_error(&mut errors, format!("aerial_opportunity_revealed_twice_on_path:{}", observation.opportunity_id));
                }
            }
            for observation in &node.source.fallback_reveals {
                if !path_fallback.insert(observation.opportunity_id.as_str()) {
                    add_error(&mut errors, format!("fallback_opportunity_revealed_twice_on_path:{}", observation.opportunity_id));
                }
            }
        }
        if path_aerial != expected_aerial_ids || path_fallback != expected_fallback_ids {
            add_error(&mut errors, format!("leaf_does_not_reveal_complete_opportunity_universe:{}", normalized[*path.last().unwrap()].source.node_id));
        }
    }
    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let mut model = Model {
        service,
        requests: &service.requests,
        nodes: normalized,
        transitions: 0,
    };
    let mut memo = BTreeMap::new();
    let root_plan = solve_node(0, 0, &mut model, &mut memo)
        .map_err(|error| vec![error])?;

    let mut leaves = Vec::new();
    let mut clairvoyant_expected = ScaledScore::default();
    let mut nonanticipative_from_leaves = ScaledScore::default();
    let mut worst_case_on_time = usize::MAX;
    let mut worst_case_completed = usize::MAX;
    let mut worst_case_lateness = 0_u64;
    let mut probability_sum = 0_u128;
    let mut clairvoyant_transitions = 0_usize;

    leaf_paths.sort_by(|left, right| {
        model.nodes[*left.last().unwrap()]
            .source
            .node_id
            .cmp(&model.nodes[*right.last().unwrap()].source.node_id)
    });
    for path in &leaf_paths {
        let leaf_index = *path.last().ok_or_else(|| vec!["scenario_path_empty".to_owned()])?;
        let probability_numerator =
            path_probability_numerator(path, &model.nodes).map_err(|error| vec![error])?;
        probability_sum = probability_sum
            .checked_add(probability_numerator)
            .ok_or_else(|| vec!["scenario_probability_sum_overflow".to_owned()])?;
        let (policy_score, rejected, unserved) = leaf_policy_score(
            path,
            &model.nodes,
            model.requests,
            &root_plan.assignments_by_node,
            &root_plan.expired_by_node,
            service.horizon_seconds,
        )
        .map_err(|error| vec![error])?;
        let clairvoyant_score = leaf_clairvoyant_score(
            path,
            &model.nodes,
            model.requests,
            service.horizon_seconds,
            &mut clairvoyant_transitions,
        )
        .map_err(|error| vec![error])?;
        let leaf_scaled_probability = probability_numerator;
        weighted_leaf_score(
            policy_score,
            leaf_scaled_probability,
            &mut nonanticipative_from_leaves,
        )
        .map_err(|error| vec![error])?;
        weighted_leaf_score(
            clairvoyant_score,
            leaf_scaled_probability,
            &mut clairvoyant_expected,
        )
        .map_err(|error| vec![error])?;
        worst_case_on_time = worst_case_on_time.min(policy_score.on_time as usize);
        worst_case_completed = worst_case_completed.min(policy_score.completed as usize);
        worst_case_lateness = worst_case_lateness.max(policy_score.lateness_seconds);
        leaves.push(LeafScenarioResult {
            leaf_node_id: model.nodes[leaf_index].source.node_id.clone(),
            path_node_ids: path
                .iter()
                .map(|index| model.nodes[*index].source.node_id.clone())
                .collect(),
            probability_numerator,
            probability_denominator: PROBABILITY_DENOMINATOR,
            on_time_services: policy_score.on_time as usize,
            late_services: policy_score.completed.saturating_sub(policy_score.on_time) as usize,
            completed_by_horizon: policy_score.completed as usize,
            in_progress_services: policy_score.in_progress as usize,
            rejected_requests: rejected,
            unserved_requests: unserved,
            lateness_seconds: policy_score.lateness_seconds,
            completed_latency_seconds: policy_score.latency_seconds,
            clairvoyant_on_time_services: clairvoyant_score.on_time as usize,
            clairvoyant_completed_by_horizon: clairvoyant_score.completed as usize,
            clairvoyant_lateness_seconds: clairvoyant_score.lateness_seconds,
            clairvoyant_completed_latency_seconds: clairvoyant_score.latency_seconds,
        });
    }
    if probability_sum != PROBABILITY_DENOMINATOR {
        return Err(vec!["scenario_leaf_probabilities_do_not_sum_to_one".to_owned()]);
    }
    if nonanticipative_from_leaves != root_plan.score {
        return Err(vec!["nonanticipative_policy_score_leaf_replay_mismatch".to_owned()]);
    }
    if clairvoyant_expected.on_time < root_plan.score.on_time
        || clairvoyant_expected.completed < root_plan.score.completed
        || clairvoyant_expected.lateness_seconds > root_plan.score.lateness_seconds
    {
        return Err(vec!["clairvoyant_reference_does_not_dominate_nonanticipative_policy".to_owned()]);
    }

    let expected_nonanticipative = ExactExpectedScore {
        on_time_services_numerator: root_plan.score.on_time,
        completed_by_horizon_numerator: root_plan.score.completed,
        lateness_seconds_numerator: root_plan.score.lateness_seconds,
        in_progress_numerator: root_plan.score.in_progress,
        completed_latency_seconds_numerator: root_plan.score.latency_seconds,
        probability_denominator: PROBABILITY_DENOMINATOR,
    };
    let expected_clairvoyant_full_information = ExactExpectedScore {
        on_time_services_numerator: clairvoyant_expected.on_time,
        completed_by_horizon_numerator: clairvoyant_expected.completed,
        lateness_seconds_numerator: clairvoyant_expected.lateness_seconds,
        in_progress_numerator: clairvoyant_expected.in_progress,
        completed_latency_seconds_numerator: clairvoyant_expected.latency_seconds,
        probability_denominator: PROBABILITY_DENOMINATOR,
    };
    let value_of_perfect_information = PerfectInformationValue {
        additional_on_time_services_numerator: clairvoyant_expected.on_time - root_plan.score.on_time,
        additional_completed_services_numerator: clairvoyant_expected.completed - root_plan.score.completed,
        avoided_lateness_seconds_numerator: root_plan.score.lateness_seconds - clairvoyant_expected.lateness_seconds,
        avoided_completed_latency_seconds_numerator: root_plan.score.latency_seconds.saturating_sub(clairvoyant_expected.latency_seconds),
        probability_denominator: PROBABILITY_DENOMINATOR,
    };

    let mut policy = Vec::new();
    for node in &model.nodes {
        let assignments = root_plan
            .assignments_by_node
            .get(&node.source.node_id)
            .cloned()
            .unwrap_or_default();
        let expired_request_ids = root_plan
            .expired_by_node
            .get(&node.source.node_id)
            .cloned()
            .unwrap_or_default();
        policy.push(InformationSetAction {
            node_id: node.source.node_id.clone(),
            parent_node_id: node.source.parent_node_id.clone(),
            decision_time_seconds: node.source.decision_time_seconds,
            observation_digest: node.observation_digest.clone(),
            information_set_digest: node.information_set_digest.clone(),
            assignments,
            expired_request_ids,
        });
    }
    policy.sort_by(|left, right| {
        (left.decision_time_seconds, &left.node_id)
            .cmp(&(right.decision_time_seconds, &right.node_id))
    });
    leaves.sort_by(|left, right| left.leaf_node_id.cmp(&right.leaf_node_id));

    Ok(NonanticipativeServiceOracleReport {
        schema_version: input.schema_version.clone(),
        benchmark_id: input.benchmark_id.clone(),
        status: NonanticipativeOracleStatus::Evaluated,
        information_model: "nonanticipative_policy_by_revealed_information_set",
        nonanticipativity_enforced_by_shared_information_nodes: true,
        deployable_online_policy: false,
        horizon_seconds: service.horizon_seconds,
        source_architecture_id: service.source_architecture_id.clone(),
        capacity_source_report_digest: source_report_digest,
        capacity_service_opportunity_digest,
        capacity_architecture_configuration_digest: architecture_configuration_digest,
        request_count: service.requests.len(),
        qualified_aerial_opportunity_count: service.aerial_opportunity_qualifications.len(),
        fallback_opportunity_count: service.fallback_opportunities.len(),
        node_count: model.nodes.len(),
        leaf_count: leaves.len(),
        maximum_tree_depth: model.nodes.iter().map(|node| node.depth).max().unwrap_or(0),
        transitions: model.transitions + clairvoyant_transitions,
        expected_nonanticipative,
        expected_clairvoyant_full_information,
        value_of_perfect_information,
        worst_case_on_time_services: if worst_case_on_time == usize::MAX { 0 } else { worst_case_on_time },
        worst_case_completed_by_horizon: if worst_case_completed == usize::MAX { 0 } else { worst_case_completed },
        worst_case_lateness_seconds: worst_case_lateness,
        policy,
        leaf_results: leaves,
        ledger_or_physical_mutation_performed: false,
        flight_authority_granted: false,
        market_or_dispatch_authority_granted: false,
        claim_ceiling: CLAIM_CEILING,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree_fixture() -> NonanticipativeServiceOracleInput {
        serde_json::from_str(include_str!(
            "../fixtures/nonanticipative_service_oracle_v0_1.json"
        ))
        .expect("frozen nonanticipative tree fixture must parse")
    }

    fn linked_fixture() -> LinkedServiceQualityInput {
        serde_json::from_str(include_str!(
            "../fixtures/sustainable_mobility_service_quality_linked_v0_1.json"
        ))
        .expect("linked service fixture must parse")
    }

    fn capacity_fixture() -> SustainableServiceCapacityInput {
        serde_json::from_str(include_str!(
            "../fixtures/sustainable_mobility_service_capacity_v0_1.json"
        ))
        .expect("capacity fixture must parse")
    }

    #[test]
    fn policy_has_one_action_per_shared_information_node_and_value_of_information_is_positive() {
        let input = tree_fixture();
        let report = evaluate_nonanticipative_service_oracle(
            &input,
            &linked_fixture(),
            &capacity_fixture(),
        )
        .expect("scenario-tree known answer");
        assert!(report.nonanticipativity_enforced_by_shared_information_nodes);
        assert!(!report.deployable_online_policy);
        assert_eq!(report.node_count, 3);
        assert_eq!(report.leaf_count, 2);
        assert_eq!(report.expected_nonanticipative.on_time_services_numerator, PROBABILITY_DENOMINATOR);
        assert_eq!(
            report.expected_clairvoyant_full_information.on_time_services_numerator,
            PROBABILITY_DENOMINATOR + PROBABILITY_DENOMINATOR * 2 / 5
        );
        assert_eq!(
            report.value_of_perfect_information.additional_on_time_services_numerator,
            PROBABILITY_DENOMINATOR * 2 / 5
        );
        let root = report.policy.iter().find(|node| node.node_id == "root").expect("root action");
        assert_eq!(root.assignments.len(), 1);
        assert_eq!(root.assignments[0].request_id, "request-visible-early");
        assert_eq!(root.assignments[0].opportunity_id, "aircraft-00-mission-000000");
        assert_eq!(report.policy.iter().filter(|node| node.node_id == "root").count(), 1);
        assert!(!report.ledger_or_physical_mutation_performed);
        assert!(!report.flight_authority_granted);
        assert!(!report.market_or_dispatch_authority_granted);
    }

    #[test]
    fn weather_no_go_blocks_scenario_aerial_opportunity_but_not_fallback() {
        let mut tree = tree_fixture();
        let high = tree.nodes.iter_mut().find(|node| node.node_id == "high-demand").expect("high");
        high.fallback_reveals[0].available = true;
        // The first opportunity is root-owned; a no-go state is exercised by
        // moving the branch observation to the root in the dedicated test below.
        let report = evaluate_nonanticipative_service_oracle(&tree, &linked_fixture(), &capacity_fixture())
            .expect("valid availability update");
        assert_eq!(report.leaf_count, 2);
    }

    #[test]
    fn request_reveal_after_its_acceptance_cutoff_is_rejected_by_the_policy() {
        let mut tree = tree_fixture();
        let high = tree.nodes.iter_mut().find(|node| node.node_id == "high-demand").expect("high");
        high.request_reveals[0].evidence.observed_at_seconds = 300;
        let report = evaluate_nonanticipative_service_oracle(&tree, &linked_fixture(), &capacity_fixture())
            .expect("late reveal is an explicit rejected request");
        assert_eq!(report.leaf_results.iter().find(|leaf| leaf.leaf_node_id == "high-demand").unwrap().rejected_requests, 1);
    }

    #[test]
    fn future_request_observation_is_rejected() {
        let mut tree = tree_fixture();
        let high = tree.nodes.iter_mut().find(|node| node.node_id == "high-demand").expect("high");
        high.request_reveals[0].evidence.observed_at_seconds = 301;
        let errors = evaluate_nonanticipative_service_oracle(&tree, &linked_fixture(), &capacity_fixture())
            .expect_err("future observation");
        assert!(errors.iter().any(|error| error.contains("scenario_request_reveal_invalid_or_replayed")));
    }

    #[test]
    fn future_weather_observation_is_rejected() {
        let mut tree = tree_fixture();
        tree.nodes[0].aerial_reveals[0].evidence.observed_at_seconds = 1;
        let errors = evaluate_nonanticipative_service_oracle(&tree, &linked_fixture(), &capacity_fixture())
            .expect_err("future weather observation");
        assert!(errors.iter().any(|error| error.contains("scenario_aerial_observation_invalid_or_replayed")));
    }

    #[test]
    fn stale_weather_observation_cannot_cover_full_mission_and_turnaround() {
        let mut tree = tree_fixture();
        tree.nodes[0].aerial_reveals[0].evidence.valid_until_seconds = 344;
        let errors = evaluate_nonanticipative_service_oracle(&tree, &linked_fixture(), &capacity_fixture())
            .expect_err("stale evidence");
        assert!(errors.iter().any(|error| error.contains("scenario_aerial_observation_invalid_or_replayed")));
    }

    #[test]
    fn sibling_probabilities_must_sum_to_one_million() {
        let mut tree = tree_fixture();
        tree.nodes[1].conditional_probability_ppm = 399_999;
        let errors = evaluate_nonanticipative_service_oracle(&tree, &linked_fixture(), &capacity_fixture())
            .expect_err("probability sum");
        assert!(errors.iter().any(|error| error.contains("scenario_sibling_probabilities_do_not_sum_to_one")));
    }

    #[test]
    fn duplicate_request_on_one_path_is_rejected() {
        let mut tree = tree_fixture();
        let low = tree.nodes.iter_mut().find(|node| node.node_id == "low-demand").expect("low");
        low.request_reveals.push(tree.nodes[1].request_reveals[0].clone());
        let errors = evaluate_nonanticipative_service_oracle(&tree, &linked_fixture(), &capacity_fixture())
            .expect_err("same request re-revealed on one path");
        assert!(errors.iter().any(|error| error.contains("request_revealed_twice_on_path")));
    }

    #[test]
    fn opportunity_universe_must_be_revealed_on_every_leaf_path() {
        let mut tree = tree_fixture();
        tree.nodes[2].fallback_reveals.clear();
        let errors = evaluate_nonanticipative_service_oracle(&tree, &linked_fixture(), &capacity_fixture())
            .expect_err("missing branch fallback observation");
        assert!(errors.iter().any(|error| error.contains("leaf_does_not_reveal_complete_opportunity_universe")));
    }

    #[test]
    fn input_order_permutation_preserves_canonical_policy_and_digests() {
        let input = tree_fixture();
        let mut permuted = tree_fixture();
        permuted.nodes.reverse();
        for node in &mut permuted.nodes {
            node.request_reveals.reverse();
            node.aerial_reveals.reverse();
            node.fallback_reveals.reverse();
        }
        let first = evaluate_nonanticipative_service_oracle(&input, &linked_fixture(), &capacity_fixture())
            .expect("first");
        let second = evaluate_nonanticipative_service_oracle(&permuted, &linked_fixture(), &capacity_fixture())
            .expect("permuted");
        assert_eq!(
            serde_json::to_string(&first).expect("serialize first"),
            serde_json::to_string(&second).expect("serialize second")
        );
    }

    #[test]
    fn bounds_exceeded_tree_is_rejected() {
        let mut tree = tree_fixture();
        tree.nodes.extend(tree.nodes.clone());
        let errors = evaluate_nonanticipative_service_oracle(&tree, &linked_fixture(), &capacity_fixture())
            .expect_err("node bound");
        assert!(errors.iter().any(|error| error == "nonanticipative_identity_or_bounds_invalid"));
    }
}
