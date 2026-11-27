// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! AERB candidate-only v0.2 submission and counterfactual-policy validator.
//!
//! This module validates candidate predictions and bounded non-actuating policy
//! graphs. It never receives evaluator truth, actual probe partitions,
//! likelihoods, or evaluator loss rows. It is a schema/contract validator, not
//! a process-security boundary and not an execution-authority path.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use serde::Deserialize;

use super::{CandidateEpisode, OPEN_WORLD_RESIDUAL, SCOPE_CLOSURE_PROBE_ID};

const V02_SCHEMA_VERSION: &str = "0.2";
const V02_BENCHMARK: &str = "AERB";
const MAX_POLICY_NODES: usize = 128;
const MAX_POLICY_DEPTH: usize = 8;
const MAX_PROBE_PREDICTIONS: usize = 64;
const MAX_PREDICTED_OUTCOMES: usize = 128;

const WORLD_IDS: [&str; 8] = [
    "not_dispatched",
    "dispatched_no_effect",
    "applied_once_ack_lost",
    "partial_effect",
    "duplicate_effect",
    "stale_authority_effect",
    "unresolved_command_holder",
    OPEN_WORLD_RESIDUAL,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActuationGuard {
    BlockNewEffect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalDisposition {
    HoldReconcile,
    Abstain,
    RecommendFreshAttempt,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateSubmissionV02 {
    pub schema_version: String,
    pub benchmark: String,
    pub episode_id: String,
    pub candidate_snapshot_sha256: String,
    pub decision_need_sha256: String,
    pub decision_contract_sha256: String,
    pub probe_catalogue_sha256: String,
    pub model_sha256: String,
    pub observer_profile_sha256: String,
    pub actuation_guard: ActuationGuard,
    pub initial_possible_world_support_ids: Vec<String>,
    pub candidate_predicted_probe_models: Vec<ProbePrediction>,
    pub counterfactual_policy: CounterfactualPolicy,
    pub reason_codes: Vec<String>,
    #[serde(default)]
    pub explanation_observation_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbePrediction {
    pub probe_id: String,
    pub outcomes: Vec<PredictedOutcome>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PredictedOutcome {
    pub outcome_id: String,
    pub possible_world_support_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CounterfactualPolicy {
    pub root_node_id: String,
    pub nodes: Vec<PolicyNode>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyNode {
    pub node_id: String,
    pub node_type: String,
    pub counterfactual_only: bool,
    #[serde(default)]
    pub probe_id: Option<String>,
    #[serde(default)]
    pub outcome_branches: Option<Vec<OutcomeBranch>>,
    #[serde(default)]
    pub terminal_disposition: Option<TerminalDisposition>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutcomeBranch {
    pub outcome_id: String,
    pub next_node_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateV02Error(pub String);

impl Display for CandidateV02Error {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for CandidateV02Error {}

fn fail<T>(message: impl Into<String>) -> Result<T, CandidateV02Error> {
    Err(CandidateV02Error(message.into()))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_world(world_id: &str) -> bool {
    WORLD_IDS.contains(&world_id)
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

/// Parse and validate one candidate-visible v0.2 submission against the exact
/// v0.1 decision-cut episode that it claims to describe.
///
/// The validator checks the existing v0.1 snapshot digest profile. That profile
/// is a scoped Serde/sorted-JSON profile, not a claim of RFC 8785 conformance or
/// source authentication.
pub fn validate_candidate_submission_v0_2(
    episode_json: &str,
    submission_json: &str,
) -> Result<CandidateSubmissionV02, CandidateV02Error> {
    let episode: CandidateEpisode = serde_json::from_str(episode_json)
        .map_err(|error| CandidateV02Error(format!("EPISODE_JSON_INVALID: {error}")))?;
    super::validate_episode(&episode)
        .map_err(|error| CandidateV02Error(format!("EPISODE_INVALID: {error}")))?;

    let submission: CandidateSubmissionV02 = serde_json::from_str(submission_json)
        .map_err(|error| CandidateV02Error(format!("SUBMISSION_JSON_INVALID: {error}")))?;

    validate_submission(&episode, episode_json, &submission)?;
    Ok(submission)
}

fn validate_submission(
    episode: &CandidateEpisode,
    episode_json: &str,
    submission: &CandidateSubmissionV02,
) -> Result<(), CandidateV02Error> {
    if submission.schema_version != V02_SCHEMA_VERSION || submission.benchmark != V02_BENCHMARK {
        return fail("UNSUPPORTED_V02_SCHEMA_OR_BENCHMARK");
    }
    if submission.episode_id != episode.episode_id {
        return fail("EPISODE_ID_MISMATCH");
    }
    let snapshot_digest = super::canonical_sorted_json_sha256(episode_json)
        .map_err(|error| CandidateV02Error(format!("SNAPSHOT_DIGEST_FAILED: {error}")))?;
    if submission.candidate_snapshot_sha256 != snapshot_digest {
        return fail("CANDIDATE_SNAPSHOT_DIGEST_MISMATCH");
    }
    for (name, digest) in [
        ("decision_need_sha256", &submission.decision_need_sha256),
        (
            "decision_contract_sha256",
            &submission.decision_contract_sha256,
        ),
        ("probe_catalogue_sha256", &submission.probe_catalogue_sha256),
        ("model_sha256", &submission.model_sha256),
        (
            "observer_profile_sha256",
            &submission.observer_profile_sha256,
        ),
    ] {
        if !valid_digest(digest) {
            return fail(format!("INVALID_DIGEST: {name}"));
        }
    }
    if submission.actuation_guard != ActuationGuard::BlockNewEffect {
        return fail("NEW_EFFECT_GUARD_NOT_BLOCKED");
    }
    if submission.initial_possible_world_support_ids.is_empty() {
        return fail("INITIAL_SUPPORT_EMPTY");
    }

    let mut initial_support = BTreeSet::new();
    for world_id in &submission.initial_possible_world_support_ids {
        if !valid_world(world_id) {
            return fail(format!("UNKNOWN_INITIAL_WORLD: {world_id}"));
        }
        if !initial_support.insert(world_id.clone()) {
            return fail(format!("DUPLICATE_INITIAL_WORLD: {world_id}"));
        }
    }
    if !initial_support.contains(OPEN_WORLD_RESIDUAL) {
        return fail("OPEN_WORLD_RESIDUAL_DROPPED");
    }

    let observation_ids: BTreeSet<&str> = episode
        .observations
        .iter()
        .map(|observation| observation.observation_id.as_str())
        .collect();
    let mut cited_observations = BTreeSet::new();
    for observation_id in &submission.explanation_observation_ids {
        if !observation_ids.contains(observation_id.as_str()) {
            return fail(format!("UNKNOWN_EXPLANATION_OBSERVATION: {observation_id}"));
        }
        if !cited_observations.insert(observation_id.as_str()) {
            return fail(format!(
                "DUPLICATE_EXPLANATION_OBSERVATION: {observation_id}"
            ));
        }
    }
    let mut reason_codes = BTreeSet::new();
    for reason_code in &submission.reason_codes {
        if !nonempty(reason_code) || !reason_codes.insert(reason_code.as_str()) {
            return fail("REASON_CODES_MUST_BE_NONEMPTY_AND_UNIQUE");
        }
    }

    let permitted_probes: BTreeMap<&str, &super::PermittedProbe> = episode
        .permitted_probes
        .iter()
        .map(|probe| (probe.probe_id.as_str(), probe))
        .collect();

    if submission.candidate_predicted_probe_models.is_empty()
        || submission.candidate_predicted_probe_models.len() > MAX_PROBE_PREDICTIONS
    {
        return fail("PREDICTED_PROBE_COUNT_OUT_OF_RANGE");
    }
    let mut predictions = BTreeMap::new();
    for prediction in &submission.candidate_predicted_probe_models {
        if !nonempty(&prediction.probe_id) {
            return fail("EMPTY_PREDICTED_PROBE_ID");
        }
        if !permitted_probes.contains_key(prediction.probe_id.as_str()) {
            return fail(format!(
                "PREDICTED_PROBE_NOT_PERMITTED: {}",
                prediction.probe_id
            ));
        }
        if predictions
            .insert(prediction.probe_id.as_str(), prediction)
            .is_some()
        {
            return fail(format!(
                "DUPLICATE_PREDICTED_PROBE: {}",
                prediction.probe_id
            ));
        }
        validate_probe_prediction(prediction, &initial_support)?;
    }

    validate_policy_graph(
        &submission.counterfactual_policy,
        &initial_support,
        &predictions,
        &permitted_probes,
        episode.current_step,
        episode.decision_deadline_step,
        episode.remaining_probe_budget_units,
    )
}

fn validate_probe_prediction(
    prediction: &ProbePrediction,
    initial_support: &BTreeSet<String>,
) -> Result<(), CandidateV02Error> {
    if prediction.outcomes.is_empty() || prediction.outcomes.len() > MAX_PREDICTED_OUTCOMES {
        return fail(format!(
            "PREDICTED_OUTCOME_COUNT_OUT_OF_RANGE: {}",
            prediction.probe_id
        ));
    }
    let mut outcome_ids = BTreeSet::new();
    let mut covered_worlds = BTreeSet::new();
    let mut has_unresolved_scope = false;
    let mut has_closed_scope = false;

    for outcome in &prediction.outcomes {
        if !nonempty(&outcome.outcome_id) {
            return fail(format!("EMPTY_OUTCOME_ID: {}", prediction.probe_id));
        }
        if !outcome_ids.insert(outcome.outcome_id.as_str()) {
            return fail(format!(
                "DUPLICATE_OUTCOME_ID: {}/{}",
                prediction.probe_id, outcome.outcome_id
            ));
        }
        if outcome.possible_world_support_ids.is_empty() {
            return fail(format!(
                "EMPTY_OUTCOME_SUPPORT: {}/{}",
                prediction.probe_id, outcome.outcome_id
            ));
        }
        let mut local = BTreeSet::new();
        for world_id in &outcome.possible_world_support_ids {
            if !valid_world(world_id) {
                return fail(format!("UNKNOWN_OUTCOME_WORLD: {world_id}"));
            }
            if !initial_support.contains(world_id) {
                return fail(format!(
                    "OUTCOME_WORLD_OUTSIDE_INITIAL_SUPPORT: {}/{}",
                    prediction.probe_id, world_id
                ));
            }
            if !local.insert(world_id.as_str()) {
                return fail(format!(
                    "DUPLICATE_OUTCOME_WORLD: {}/{}",
                    outcome.outcome_id, world_id
                ));
            }
            covered_worlds.insert(world_id.clone());
            if world_id.as_str() == OPEN_WORLD_RESIDUAL {
                has_unresolved_scope = true;
            }
        }
        if !outcome
            .possible_world_support_ids
            .iter()
            .any(|world_id| world_id == OPEN_WORLD_RESIDUAL)
        {
            has_closed_scope = true;
        }
    }
    if &covered_worlds != initial_support {
        return fail(format!(
            "PREDICTED_OUTCOMES_DO_NOT_COVER_INITIAL_SUPPORT: {}",
            prediction.probe_id
        ));
    }
    if prediction.probe_id == SCOPE_CLOSURE_PROBE_ID {
        if !has_unresolved_scope || !has_closed_scope {
            return fail("SCOPE_CLOSURE_MUST_RETAIN_CLOSED_AND_UNRESOLVED_OUTCOMES");
        }
    } else if prediction.outcomes.iter().any(|outcome| {
        !outcome
            .possible_world_support_ids
            .iter()
            .any(|world_id| world_id == OPEN_WORLD_RESIDUAL)
    }) {
        return fail(format!(
            "ORDINARY_PROBE_ELIMINATES_OPEN_WORLD_RESIDUAL: {}",
            prediction.probe_id
        ));
    }
    Ok(())
}

fn validate_policy_graph(
    policy: &CounterfactualPolicy,
    initial_support: &BTreeSet<String>,
    predictions: &BTreeMap<&str, &ProbePrediction>,
    permitted_probes: &BTreeMap<&str, &super::PermittedProbe>,
    current_step: u64,
    decision_deadline_step: u64,
    remaining_budget_units: f64,
) -> Result<(), CandidateV02Error> {
    if policy.nodes.is_empty() || policy.nodes.len() > MAX_POLICY_NODES {
        return fail("POLICY_NODE_COUNT_OUT_OF_RANGE");
    }
    if !nonempty(&policy.root_node_id) {
        return fail("POLICY_ROOT_ID_EMPTY");
    }

    let mut nodes = BTreeMap::new();
    for node in &policy.nodes {
        if !nonempty(&node.node_id) {
            return fail("POLICY_NODE_ID_EMPTY");
        }
        if nodes.insert(node.node_id.as_str(), node).is_some() {
            return fail(format!("DUPLICATE_POLICY_NODE_ID: {}", node.node_id));
        }
        if !node.counterfactual_only {
            return fail(format!(
                "POLICY_NODE_NOT_COUNTERFACTUAL_ONLY: {}",
                node.node_id
            ));
        }
        match node.node_type.as_str() {
            "probe" => {
                if node.probe_id.is_none()
                    || node.outcome_branches.is_none()
                    || node.terminal_disposition.is_some()
                {
                    return fail(format!("MALFORMED_PROBE_NODE: {}", node.node_id));
                }
            }
            "terminal" => {
                if node.probe_id.is_some()
                    || node.outcome_branches.is_some()
                    || node.terminal_disposition.is_none()
                {
                    return fail(format!("MALFORMED_TERMINAL_NODE: {}", node.node_id));
                }
            }
            other => return fail(format!("UNKNOWN_POLICY_NODE_TYPE: {other}")),
        }
    }
    if !nodes.contains_key(policy.root_node_id.as_str()) {
        return fail("POLICY_ROOT_NODE_MISSING");
    }

    let mut incoming = BTreeMap::<&str, usize>::new();
    for node in &policy.nodes {
        if let Some(branches) = &node.outcome_branches {
            let mut branch_ids = BTreeSet::new();
            for branch in branches {
                if !nonempty(&branch.outcome_id) || !nonempty(&branch.next_node_id) {
                    return fail(format!("EMPTY_POLICY_BRANCH_REFERENCE: {}", node.node_id));
                }
                if !branch_ids.insert(branch.outcome_id.as_str()) {
                    return fail(format!(
                        "DUPLICATE_POLICY_BRANCH_OUTCOME: {}/{}",
                        node.node_id, branch.outcome_id
                    ));
                }
                if !nodes.contains_key(branch.next_node_id.as_str()) {
                    return fail(format!(
                        "POLICY_BRANCH_TARGET_MISSING: {}/{}",
                        node.node_id, branch.next_node_id
                    ));
                }
                *incoming.entry(branch.next_node_id.as_str()).or_default() += 1;
            }
        }
    }
    if incoming
        .get(policy.root_node_id.as_str())
        .copied()
        .unwrap_or(0)
        != 0
    {
        return fail("POLICY_ROOT_HAS_INCOMING_EDGE");
    }
    for node_id in nodes.keys() {
        if *node_id != policy.root_node_id.as_str()
            && incoming.get(*node_id).copied().unwrap_or(0) != 1
        {
            return fail(format!("POLICY_NODE_MUST_HAVE_ONE_PARENT: {node_id}"));
        }
    }

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    walk_policy(
        policy.root_node_id.as_str(),
        initial_support,
        &nodes,
        predictions,
        permitted_probes,
        current_step,
        decision_deadline_step,
        remaining_budget_units,
        0.0,
        0,
        &mut visiting,
        &mut visited,
    )?;
    if visited.len() != nodes.len() {
        return fail("POLICY_GRAPH_CONTAINS_UNREACHABLE_NODES");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn walk_policy(
    node_id: &str,
    possible_support: &BTreeSet<String>,
    nodes: &BTreeMap<&str, &PolicyNode>,
    predictions: &BTreeMap<&str, &ProbePrediction>,
    permitted_probes: &BTreeMap<&str, &super::PermittedProbe>,
    current_step: u64,
    decision_deadline_step: u64,
    remaining_budget_units: f64,
    accumulated_cost_units: f64,
    depth: usize,
    visiting: &mut BTreeSet<String>,
    visited: &mut BTreeSet<String>,
) -> Result<(), CandidateV02Error> {
    if depth > MAX_POLICY_DEPTH {
        return fail("POLICY_DEPTH_EXCEEDED");
    }
    if visiting.contains(node_id) {
        return fail(format!("POLICY_GRAPH_CYCLE: {node_id}"));
    }
    if !visited.insert(node_id.to_owned()) {
        return fail(format!("POLICY_NODE_REVISITED: {node_id}"));
    }
    let node = nodes
        .get(node_id)
        .copied()
        .ok_or_else(|| CandidateV02Error(format!("POLICY_NODE_MISSING: {node_id}")))?;

    if node.node_type == "terminal" {
        if possible_support.is_empty() {
            return fail(format!("TERMINAL_WITH_EMPTY_SUPPORT: {node_id}"));
        }
        let disposition = node
            .terminal_disposition
            .ok_or_else(|| {
                CandidateV02Error(format!("TERMINAL_DISPOSITION_MISSING: {node_id}"))
            })?;
        if disposition == TerminalDisposition::RecommendFreshAttempt
            && possible_support
                .iter()
                .any(|world_id| world_id != "not_dispatched" && world_id != "dispatched_no_effect")
        {
            return fail(format!(
                "FRESH_ATTEMPT_RECOMMENDED_WITH_NON_NO_EFFECT_SUPPORT: {node_id}"
            ));
        }
        return Ok(());
    }

    let probe_id = node
        .probe_id
        .as_deref()
        .ok_or_else(|| CandidateV02Error(format!("PROBE_NODE_ID_MISSING: {node_id}")))?;
    let permitted_probe = permitted_probes
        .get(probe_id)
        .copied()
        .ok_or_else(|| CandidateV02Error(format!("POLICY_PROBE_NOT_PERMITTED: {probe_id}")))?;
    let prediction = predictions
        .get(probe_id)
        .copied()
        .ok_or_else(|| CandidateV02Error(format!("POLICY_PROBE_NOT_PREDICTED: {probe_id}")))?;

    let next_cost = accumulated_cost_units + permitted_probe.acquisition_cost_units;
    if !next_cost.is_finite() || next_cost > remaining_budget_units + 1.0e-12 {
        return fail(format!("POLICY_PATH_OVER_BUDGET: {node_id}"));
    }
    let next_step = current_step
        .checked_add(permitted_probe.result_delay_steps)
        .ok_or_else(|| CandidateV02Error(format!("POLICY_TIME_OVERFLOW: {node_id}")))?;
    if next_step > decision_deadline_step {
        return fail(format!("POLICY_PATH_MISSES_DEADLINE: {node_id}"));
    }

    let mut active_outcomes = BTreeMap::<String, BTreeSet<String>>::new();
    for outcome in &prediction.outcomes {
        let support: BTreeSet<String> = outcome
            .possible_world_support_ids
            .iter()
            .filter(|world_id| possible_support.contains(*world_id))
            .cloned()
            .collect();
        if !support.is_empty() {
            active_outcomes.insert(outcome.outcome_id.clone(), support);
        }
    }
    if active_outcomes.is_empty() {
        return fail(format!("POLICY_PROBE_HAS_NO_REACHABLE_OUTCOME: {node_id}"));
    }

    let branches = node
        .outcome_branches
        .as_ref()
        .ok_or_else(|| CandidateV02Error(format!("PROBE_BRANCHES_MISSING: {node_id}")))?;
    let declared_outcomes: BTreeSet<&str> = branches
        .iter()
        .map(|branch| branch.outcome_id.as_str())
        .collect();
    let active_outcome_ids: BTreeSet<&str> = active_outcomes.keys().map(String::as_str).collect();
    if declared_outcomes != active_outcome_ids {
        return fail(format!(
            "POLICY_BRANCHES_DO_NOT_MATCH_REACHABLE_OUTCOMES: {node_id}"
        ));
    }

    visiting.insert(node_id.to_owned());
    for branch in branches {
        let child_support = active_outcomes.get(&branch.outcome_id).ok_or_else(|| {
            CandidateV02Error(format!(
                "POLICY_BRANCH_OUTCOME_UNKNOWN: {}",
                branch.outcome_id
            ))
        })?;
        walk_policy(
            branch.next_node_id.as_str(),
            child_support,
            nodes,
            predictions,
            permitted_probes,
            next_step,
            decision_deadline_step,
            remaining_budget_units,
            next_cost,
            depth + 1,
            visiting,
            visited,
        )?;
    }
    visiting.remove(node_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    const EPISODE_JSON: &str = include_str!("../../fixtures/candidate_episode_synthetic_v0_1.json");
    const SUBMISSION_JSON: &str = include_str!(
        "../../../mobility-rrvr-aerb-probe-value-v0.1/fixtures/candidate_submission_adaptive_synthetic_v0_2.json"
    );

    fn values() -> (Value, Value) {
        (
            serde_json::from_str(EPISODE_JSON).expect("episode fixture"),
            serde_json::from_str(SUBMISSION_JSON).expect("candidate v0.2 fixture"),
        )
    }

    fn reseal_snapshot(episode: &Value, submission: &mut Value) {
        let raw = serde_json::to_string(episode).expect("episode serialization");
        let digest =
            super::super::canonical_sorted_json_sha256(&raw).expect("candidate episode digest");
        submission["candidate_snapshot_sha256"] = json!(digest);
    }

    fn evaluate_values(
        episode: &Value,
        submission: &Value,
    ) -> Result<CandidateSubmissionV02, CandidateV02Error> {
        let episode_json = serde_json::to_string(episode).expect("episode serialization");
        let submission_json = serde_json::to_string(submission).expect("submission serialization");
        validate_candidate_submission_v0_2(&episode_json, &submission_json)
    }

    #[test]
    fn adaptive_fixture_has_blocked_actuation_and_complete_policy_graph() {
        let (episode, submission) = values();
        let candidate = evaluate_values(&episode, &submission).expect("v0.2 fixture validates");
        assert_eq!(candidate.actuation_guard, ActuationGuard::BlockNewEffect);
        assert_eq!(candidate.initial_possible_world_support_ids.len(), 8);
        assert_eq!(candidate.counterfactual_policy.nodes.len(), 7);
    }

    #[test]
    fn hidden_evaluator_fields_are_rejected_by_deny_unknown_fields() {
        let (episode, mut submission) = values();
        submission["hidden_truth_world_id"] = json!("applied_once_ack_lost");
        assert!(evaluate_values(&episode, &submission).is_err());
    }

    #[test]
    fn dropping_open_world_residual_from_initial_support_is_rejected() {
        let (episode, mut submission) = values();
        submission["initial_possible_world_support_ids"]
            .as_array_mut()
            .expect("support array")
            .retain(|world| world.as_str() != Some(OPEN_WORLD_RESIDUAL));
        assert!(evaluate_values(&episode, &submission).is_err());
    }

    #[test]
    fn ordinary_probe_cannot_predict_away_open_world_residual() {
        let (episode, mut submission) = values();
        let outcome = &mut submission["candidate_predicted_probe_models"][0]["outcomes"][0];
        outcome["possible_world_support_ids"]
            .as_array_mut()
            .expect("outcome support")
            .retain(|world| world.as_str() != Some(OPEN_WORLD_RESIDUAL));
        assert!(evaluate_values(&episode, &submission).is_err());
    }

    #[test]
    fn candidate_policy_must_cover_every_reachable_outcome() {
        let (episode, mut submission) = values();
        submission["counterfactual_policy"]["nodes"][0]["outcome_branches"]
            .as_array_mut()
            .expect("root branches")
            .pop();
        assert!(evaluate_values(&episode, &submission).is_err());
    }

    #[test]
    fn candidate_policy_cannot_reference_an_unpermitted_probe() {
        let (episode, mut submission) = values();
        submission["counterfactual_policy"]["nodes"][0]["probe_id"] = json!("not-permitted");
        assert!(evaluate_values(&episode, &submission).is_err());
    }

    #[test]
    fn policy_cycles_are_rejected() {
        let (episode, mut submission) = values();
        submission["counterfactual_policy"]["nodes"][1]["outcome_branches"][0]["next_node_id"] =
            json!("n0");
        assert!(evaluate_values(&episode, &submission).is_err());
    }

    #[test]
    fn cumulative_probe_cost_is_checked_along_each_branch() {
        let (mut episode, mut submission) = values();
        episode["permitted_probes"][0]["acquisition_cost_units"] = json!(3.0);
        reseal_snapshot(&episode, &mut submission);
        assert!(evaluate_values(&episode, &submission).is_err());
    }

    #[test]
    fn cumulative_probe_delay_is_checked_against_the_decision_deadline() {
        let (mut episode, mut submission) = values();
        episode["current_step"] = json!(50);
        reseal_snapshot(&episode, &mut submission);
        assert!(evaluate_values(&episode, &submission).is_err());
    }

    #[test]
    fn candidate_cannot_claim_a_physical_dispatch_terminal_action() {
        let (episode, mut submission) = values();
        submission["counterfactual_policy"]["nodes"][5]["terminal_disposition"] =
            json!("authorize_dispatch");
        assert!(evaluate_values(&episode, &submission).is_err());
    }

    #[test]
    fn candidate_cannot_recommend_fresh_attempt_with_applied_or_open_world_support() {
        let (episode, mut submission) = values();
        submission["counterfactual_policy"]["nodes"][2]["terminal_disposition"] =
            json!("recommend_fresh_attempt");
        assert!(evaluate_values(&episode, &submission).is_err());
    }

    #[test]
    fn duplicate_candidate_reason_codes_are_rejected() {
        let (episode, mut submission) = values();
        submission["reason_codes"] = json!([
            "BLOCK_NEW_EFFECT_UNTIL_INDEPENDENT_FINALITY",
            "BLOCK_NEW_EFFECT_UNTIL_INDEPENDENT_FINALITY"
        ]);
        assert!(evaluate_values(&episode, &submission).is_err());
    }
}
