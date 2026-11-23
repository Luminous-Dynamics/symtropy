// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Nonanticipative service-policy certificate checker.
//!
//! This module checks a supplied policy mapping against the information that
//! was revealed at each decision node. It does not optimize or execute a
//! policy. Nodes sharing the same information-set digest must choose exactly
//! the same action. Requests and opportunities must be present in the node's
//! observed state before they can be assigned.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

const MAX_NODES: usize = 128;
const MAX_SCENARIOS: usize = 16;
const MAX_REQUESTS_PER_NODE: usize = 64;
const MAX_OPPORTUNITIES_PER_NODE: usize = 64;
const MAX_SERVICE_SECONDS: u32 = 86_400;
const MAX_PAYLOAD_UNITS: u32 = 1_000_000;
const CLAIM_CEILING: &str = "Finite scenario-tree nonanticipativity and information-eligibility checks only; no stochastic calibration, optimization, online policy execution, route planning, flight authorization, dispatch, physical mutation or market authority.";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AvailableRequest {
    pub request_id: String,
    pub released_at_seconds: u64,
    pub accept_by_seconds: u64,
    pub required_service_seconds: u32,
    pub payload_units: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AvailableOpportunity {
    pub opportunity_id: String,
    pub start_seconds: u64,
    pub end_seconds: u64,
    pub max_service_duration_seconds: u32,
    pub max_payload_units: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PolicyAction {
    Hold,
    Assign {
        request_id: String,
        opportunity_id: String,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PolicyDecisionNode {
    pub node_id: String,
    pub parent_node_id: Option<String>,
    pub decision_time_seconds: u64,
    pub information_set_digest: String,
    pub observation_digest: String,
    pub available_requests: Vec<AvailableRequest>,
    pub available_opportunities: Vec<AvailableOpportunity>,
    pub action: PolicyAction,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ScenarioLeaf {
    pub scenario_id: String,
    pub terminal_node_id: String,
    pub weight_numerator: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NonanticipativePolicyInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub site_id: String,
    pub root_node_id: String,
    pub probability_denominator: u32,
    pub nodes: Vec<PolicyDecisionNode>,
    pub scenarios: Vec<ScenarioLeaf>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct InformationSetDecision {
    pub information_set_digest: String,
    pub observation_digest: String,
    pub equivalent_node_count: usize,
    pub action: PolicyAction,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ScenarioPathCertificate {
    pub scenario_id: String,
    pub terminal_node_id: String,
    pub path_node_ids: Vec<String>,
    pub assigned_request_ids: Vec<String>,
    pub assigned_opportunity_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NonanticipativePolicyReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub site_id: String,
    pub root_node_id: String,
    pub node_count: usize,
    pub information_set_count: usize,
    pub scenario_count: usize,
    pub total_assignment_decisions: usize,
    pub hold_decisions: usize,
    pub probability_mass_numerator: u32,
    pub probability_mass_denominator: u32,
    pub nonanticipativity_verified: bool,
    pub future_information_leakage_detected: bool,
    pub online_policy_execution_performed: bool,
    pub dispatch_authority_granted: bool,
    pub physical_mutation_authority_granted: bool,
    pub information_set_decisions: Vec<InformationSetDecision>,
    pub scenario_paths: Vec<ScenarioPathCertificate>,
    pub findings: Vec<String>,
    pub claim_ceiling: &'static str,
}

fn add_error(errors: &mut Vec<String>, message: impl Into<String>) {
    errors.push(message.into());
}

fn is_sha256_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_node_observations(
    node: &PolicyDecisionNode,
    errors: &mut Vec<String>,
) {
    if node.node_id.trim().is_empty()
        || !is_sha256_digest(&node.information_set_digest)
        || !is_sha256_digest(&node.observation_digest)
    {
        add_error(
            errors,
            format!("node_identity_or_digest_invalid:{}", node.node_id),
        );
    }

    if node.available_requests.len() > MAX_REQUESTS_PER_NODE
        || node.available_opportunities.len() > MAX_OPPORTUNITIES_PER_NODE
    {
        add_error(
            errors,
            format!("node_observation_bound_exceeded:{}", node.node_id),
        );
    }

    let mut request_ids = BTreeSet::new();
    for request in &node.available_requests {
        if request.request_id.trim().is_empty()
            || !request_ids.insert(request.request_id.as_str())
            || request.released_at_seconds > node.decision_time_seconds
            || request.accept_by_seconds < node.decision_time_seconds
            || request.required_service_seconds == 0
            || request.required_service_seconds > MAX_SERVICE_SECONDS
            || request.payload_units > MAX_PAYLOAD_UNITS
        {
            add_error(
                errors,
                format!(
                    "request_not_available_or_invalid_at_information_set:{}:{}",
                    node.node_id, request.request_id
                ),
            );
        }
    }

    let mut opportunity_ids = BTreeSet::new();
    for opportunity in &node.available_opportunities {
        if opportunity.opportunity_id.trim().is_empty()
            || !opportunity_ids.insert(opportunity.opportunity_id.as_str())
            || opportunity.start_seconds < node.decision_time_seconds
            || opportunity.end_seconds <= opportunity.start_seconds
            || opportunity.max_service_duration_seconds == 0
            || opportunity.max_service_duration_seconds > MAX_SERVICE_SECONDS
            || opportunity.max_payload_units > MAX_PAYLOAD_UNITS
        {
            add_error(
                errors,
                format!(
                    "opportunity_not_available_or_invalid_at_information_set:{}:{}",
                    node.node_id, opportunity.opportunity_id
                ),
            );
        }
    }

    if let PolicyAction::Assign {
        request_id,
        opportunity_id,
    } = &node.action
    {
        let request = node
            .available_requests
            .iter()
            .find(|request| request.request_id == *request_id);
        let opportunity = node
            .available_opportunities
            .iter()
            .find(|opportunity| opportunity.opportunity_id == *opportunity_id);

        match (request, opportunity) {
            (Some(request), Some(opportunity)) => {
                let Some(completion) = opportunity
                    .start_seconds
                    .checked_add(u64::from(request.required_service_seconds))
                else {
                    add_error(
                        errors,
                        format!("assignment_completion_overflow:{}:{request_id}", node.node_id),
                    );
                    return;
                };
                if request.released_at_seconds > node.decision_time_seconds
                    || request.accept_by_seconds < node.decision_time_seconds
                {
                    add_error(
                        errors,
                        format!("assignment_uses_unrevealed_or_expired_request:{}:{request_id}", node.node_id),
                    );
                }
                if opportunity.start_seconds < node.decision_time_seconds
                    || completion > opportunity.end_seconds
                    || request.required_service_seconds
                        > opportunity.max_service_duration_seconds
                    || request.payload_units > opportunity.max_payload_units
                {
                    add_error(
                        errors,
                        format!("assignment_outside_observed_opportunity_envelope:{}:{opportunity_id}", node.node_id),
                    );
                }
            }
            (None, _) => add_error(
                errors,
                format!("assignment_request_not_in_observed_information_set:{}:{request_id}", node.node_id),
            ),
            (_, None) => add_error(
                errors,
                format!("assignment_opportunity_not_in_observed_information_set:{}:{opportunity_id}", node.node_id),
            ),
        }
    }
}

fn path_to_root(
    terminal_node_id: &str,
    node_map: &BTreeMap<&str, &PolicyDecisionNode>,
) -> Result<Vec<String>, String> {
    let mut reverse_path = Vec::new();
    let mut current = terminal_node_id;
    let mut visited = BTreeSet::new();

    loop {
        if !visited.insert(current.to_owned()) {
            return Err(format!("scenario_path_cycle:{terminal_node_id}"));
        }
        let node = node_map
            .get(current)
            .ok_or_else(|| format!("scenario_terminal_or_parent_missing:{current}"))?;
        reverse_path.push(node.node_id.clone());
        match node.parent_node_id.as_deref() {
            Some(parent) => current = parent,
            None => break,
        }
        if reverse_path.len() > MAX_NODES {
            return Err(format!("scenario_path_exceeds_node_limit:{terminal_node_id}"));
        }
    }

    reverse_path.reverse();
    Ok(reverse_path)
}

pub fn evaluate_nonanticipative_policy(
    input: &NonanticipativePolicyInput,
) -> Result<NonanticipativePolicyReport, Vec<String>> {
    let mut errors = Vec::new();
    if input.schema_version != "0.1.0"
        || input.benchmark_id != "NONANTICIPATIVE-SERVICE-POLICY-001"
        || input.site_id.trim().is_empty()
        || input.root_node_id.trim().is_empty()
        || input.probability_denominator == 0
        || input.claim_ceiling.trim().is_empty()
        || input.nodes.is_empty()
        || input.nodes.len() > MAX_NODES
        || input.scenarios.is_empty()
        || input.scenarios.len() > MAX_SCENARIOS
    {
        add_error(&mut errors, "policy_input_identity_or_bounds_invalid");
    }

    let mut node_map = BTreeMap::new();
    let mut root_ids = BTreeSet::new();
    let mut child_counts: BTreeMap<String, usize> = BTreeMap::new();

    for node in &input.nodes {
        if node_map.insert(node.node_id.as_str(), node).is_some() {
            add_error(
                &mut errors,
                format!("duplicate_policy_node_id:{}", node.node_id),
            );
        }
        validate_node_observations(node, &mut errors);

        match node.parent_node_id.as_deref() {
            None => {
                root_ids.insert(node.node_id.as_str());
            }
            Some(parent_id) => {
                if parent_id == node.node_id {
                    add_error(
                        &mut errors,
                        format!("policy_node_self_parent:{}", node.node_id),
                    );
                }
                *child_counts.entry(parent_id.to_owned()).or_default() += 1;
            }
        }
    }

    if root_ids.len() != 1 || !root_ids.contains(input.root_node_id.as_str()) {
        add_error(&mut errors, "policy_tree_must_have_exact_declared_root");
    }

    for node in &input.nodes {
        if let Some(parent_id) = node.parent_node_id.as_deref() {
            match node_map.get(parent_id) {
                Some(parent) => {
                    if parent.decision_time_seconds > node.decision_time_seconds {
                        add_error(
                            &mut errors,
                            format!("policy_node_time_precedes_parent:{}", node.node_id),
                        );
                    }
                }
                None => add_error(
                    &mut errors,
                    format!("policy_parent_node_missing:{}:{parent_id}", node.node_id),
                ),
            }
        }
    }

    // Reachability from the declared root proves that no disconnected cycle
    // or orphaned scenario fragment is silently accepted.
    let mut reachable = BTreeSet::new();
    let mut stack = vec![input.root_node_id.as_str()];
    while let Some(node_id) = stack.pop() {
        if !reachable.insert(node_id.to_owned()) {
            continue;
        }
        for node in &input.nodes {
            if node.parent_node_id.as_deref() == Some(node_id) {
                stack.push(node.node_id.as_str());
            }
        }
    }
    if reachable.len() != input.nodes.len() {
        add_error(&mut errors, "policy_tree_contains_unreachable_or_cyclic_nodes");
    }

    // Nodes with the same information-set digest must have the same observed
    // state and the same action. This is the nonanticipativity certificate.
    let mut information_sets: BTreeMap<
        String,
        (String, Vec<AvailableRequest>, Vec<AvailableOpportunity>, PolicyAction, usize),
    > = BTreeMap::new();

    for node in &input.nodes {
        let mut requests = node.available_requests.clone();
        requests.sort_by(|a, b| a.request_id.cmp(&b.request_id));
        let mut opportunities = node.available_opportunities.clone();
        opportunities.sort_by(|a, b| a.opportunity_id.cmp(&b.opportunity_id));

        match information_sets.get_mut(&node.information_set_digest) {
            None => {
                information_sets.insert(
                    node.information_set_digest.clone(),
                    (
                        node.observation_digest.clone(),
                        requests,
                        opportunities,
                        node.action.clone(),
                        1,
                    ),
                );
            }
            Some((observation, known_requests, known_opportunities, action, count)) => {
                *count += 1;
                if *observation != node.observation_digest
                    || *known_requests != requests
                    || *known_opportunities != opportunities
                {
                    add_error(
                        &mut errors,
                        format!("information_set_digest_observation_mismatch:{}", node.node_id),
                    );
                }
                if *action != node.action {
                    add_error(
                        &mut errors,
                        format!("nonanticipative_action_conflict:{}", node.node_id),
                    );
                }
            }
        }
    }

    let mut sorted_scenarios = input.scenarios.clone();
    sorted_scenarios.sort_by(|a, b| a.scenario_id.cmp(&b.scenario_id));
    let mut scenario_ids = BTreeSet::new();
    let mut terminal_ids = BTreeSet::new();
    let mut probability_mass = 0_u32;
    let mut scenario_paths = Vec::new();

    for scenario in &sorted_scenarios {
        if scenario.scenario_id.trim().is_empty()
            || !scenario_ids.insert(scenario.scenario_id.as_str())
            || !terminal_ids.insert(scenario.terminal_node_id.as_str())
            || scenario.weight_numerator == 0
        {
            add_error(&mut errors, "scenario_leaf_identity_or_weight_invalid");
            continue;
        }
        probability_mass = match probability_mass.checked_add(scenario.weight_numerator) {
            Some(value) => value,
            None => {
                add_error(&mut errors, "scenario_probability_mass_overflow");
                probability_mass
            }
        };

        let path = match path_to_root(&scenario.terminal_node_id, &node_map) {
            Ok(path) => path,
            Err(error) => {
                add_error(&mut errors, error);
                continue;
            }
        };
        if path.first().map(String::as_str) != Some(input.root_node_id.as_str()) {
            add_error(
                &mut errors,
                format!("scenario_path_wrong_root:{}", scenario.scenario_id),
            );
            continue;
        }
        if child_counts
            .get(&scenario.terminal_node_id)
            .copied()
            .unwrap_or(0)
            != 0
        {
            add_error(
                &mut errors,
                format!("scenario_terminal_node_has_children:{}", scenario.terminal_node_id),
            );
        }

        let mut assigned_requests = BTreeSet::new();
        let mut assigned_opportunities = BTreeSet::new();
        for node_id in &path {
            let node = node_map[node_id.as_str()];
            if let PolicyAction::Assign {
                request_id,
                opportunity_id,
            } = &node.action
            {
                if !assigned_requests.insert(request_id.clone()) {
                    add_error(
                        &mut errors,
                        format!("request_assigned_twice_on_scenario_path:{}:{request_id}", scenario.scenario_id),
                    );
                }
                if !assigned_opportunities.insert(opportunity_id.clone()) {
                    add_error(
                        &mut errors,
                        format!("opportunity_assigned_twice_on_scenario_path:{}:{opportunity_id}", scenario.scenario_id),
                    );
                }
            }
        }

        scenario_paths.push(ScenarioPathCertificate {
            scenario_id: scenario.scenario_id.clone(),
            terminal_node_id: scenario.terminal_node_id.clone(),
            path_node_ids: path,
            assigned_request_ids: assigned_requests.into_iter().collect(),
            assigned_opportunity_ids: assigned_opportunities.into_iter().collect(),
        });
    }

    let tree_leaves: BTreeSet<&str> = input
        .nodes
        .iter()
        .filter(|node| child_counts.get(&node.node_id).copied().unwrap_or(0) == 0)
        .map(|node| node.node_id.as_str())
        .collect();
    if tree_leaves != terminal_ids {
        add_error(&mut errors, "scenario_leaf_coverage_does_not_match_tree_leaves");
    }
    if probability_mass != input.probability_denominator {
        add_error(
            &mut errors,
            format!(
                "scenario_probability_mass_mismatch:{}:{}",
                probability_mass, input.probability_denominator
            ),
        );
    }

    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let mut information_set_decisions = Vec::new();
    for (digest, (observation, _, _, action, count)) in information_sets {
        information_set_decisions.push(InformationSetDecision {
            information_set_digest: digest,
            observation_digest: observation,
            equivalent_node_count: count,
            action,
        });
    }

    scenario_paths.sort_by(|a, b| a.scenario_id.cmp(&b.scenario_id));
    let total_assignment_decisions = input
        .nodes
        .iter()
        .filter(|node| matches!(&node.action, PolicyAction::Assign { .. }))
        .count();
    let hold_decisions = input
        .nodes
        .iter()
        .filter(|node| matches!(&node.action, PolicyAction::Hold))
        .count();

    Ok(NonanticipativePolicyReport {
        schema_version: input.schema_version.clone(),
        benchmark_id: input.benchmark_id.clone(),
        site_id: input.site_id.clone(),
        root_node_id: input.root_node_id.clone(),
        node_count: input.nodes.len(),
        information_set_count: information_set_decisions.len(),
        scenario_count: input.scenarios.len(),
        total_assignment_decisions,
        hold_decisions,
        probability_mass_numerator: probability_mass,
        probability_mass_denominator: input.probability_denominator,
        nonanticipativity_verified: true,
        future_information_leakage_detected: false,
        online_policy_execution_performed: false,
        dispatch_authority_granted: false,
        physical_mutation_authority_granted: false,
        information_set_decisions,
        scenario_paths,
        findings: vec![
            "all_equivalent_information_sets_have_identical_observations_and_actions".to_owned(),
            "each_assignment_uses_only_a_request_and_opportunity_in_the_node_observation".to_owned(),
            "no_request_or_opportunity_is_assigned_twice_on_any_scenario_path".to_owned(),
            "scenario_probability_mass_is_explicit_and_sums_to_the_declared_denominator".to_owned(),
            "this_report_validates_a_policy_certificate_and_does_not_optimize_or_execute_it".to_owned(),
        ],
        claim_ceiling: CLAIM_CEILING,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> NonanticipativePolicyInput {
        serde_json::from_str(include_str!("../fixtures/nonanticipative_service_policy_v0_1.json"))
            .expect("frozen nonanticipative policy fixture must parse")
    }

    #[test]
    fn common_information_sets_share_one_action_and_revealed_branches_can_diverge() {
        let report =
            evaluate_nonanticipative_policy(&fixture()).expect("policy certificate is valid");
        assert_eq!(report.node_count, 5);
        assert_eq!(report.information_set_count, 4);
        assert_eq!(report.scenario_count, 2);
        assert_eq!(report.total_assignment_decisions, 3);
        assert_eq!(report.hold_decisions, 2);
        assert!(report.nonanticipativity_verified);
        assert!(!report.future_information_leakage_detected);
        assert!(!report.online_policy_execution_performed);
        assert!(!report.dispatch_authority_granted);
        assert!(!report.physical_mutation_authority_granted);
        assert_eq!(report.scenario_paths.len(), 2);
        assert_eq!(
            report
                .information_set_decisions
                .iter()
                .find(|decision| decision.information_set_digest == format!("sha256:{}", "c".repeat(64)))
                .expect("shared pre-reveal information set")
                .equivalent_node_count,
            2
        );
    }

    #[test]
    fn different_actions_at_indistinguishable_nodes_are_rejected() {
        let mut input = fixture();
        input.nodes.iter_mut().find(|node| node.node_id == "blind-b").unwrap().action =
            PolicyAction::Assign {
                request_id: "request-a".to_owned(),
                opportunity_id: "opportunity-1".to_owned(),
            };
        let errors = evaluate_nonanticipative_policy(&input)
            .expect_err("equivalent observations must have identical actions");
        assert!(errors
            .iter()
            .any(|error| error.contains("nonanticipative_action_conflict")));
    }

    #[test]
    fn future_request_cannot_be_used_before_its_reveal_time() {
        let mut input = fixture();
        let root = input.nodes.iter_mut().find(|node| node.node_id == "root").unwrap();
        root.action = PolicyAction::Assign {
            request_id: "request-a".to_owned(),
            opportunity_id: "opportunity-0".to_owned(),
        };
        let errors = evaluate_nonanticipative_policy(&input)
            .expect_err("future request absent from root information set");
        assert!(errors.iter().any(|error| {
            error.contains("assignment_request_not_in_observed_information_set")
        }));
    }

    #[test]
    fn unknown_opportunity_cannot_be_assigned() {
        let mut input = fixture();
        input.nodes.iter_mut().find(|node| node.node_id == "root").unwrap().action =
            PolicyAction::Assign {
                request_id: "request-early".to_owned(),
                opportunity_id: "not-observed".to_owned(),
            };
        let errors = evaluate_nonanticipative_policy(&input)
            .expect_err("unobserved opportunity cannot be used");
        assert!(errors.iter().any(|error| {
            error.contains("assignment_opportunity_not_in_observed_information_set")
        }));
    }

    #[test]
    fn duplicate_request_or_opportunity_on_one_path_is_rejected() {
        let mut input = fixture();
        let leaf = input
            .nodes
            .iter_mut()
            .find(|node| node.node_id == "revealed-a")
            .unwrap();
        leaf.action = PolicyAction::Assign {
            request_id: "request-early".to_owned(),
            opportunity_id: "opportunity-0".to_owned(),
        };
        leaf.available_requests.push(AvailableRequest {
            request_id: "request-early".to_owned(),
            released_at_seconds: 0,
            accept_by_seconds: 500,
            required_service_seconds: 100,
            payload_units: 1,
        });
        leaf.available_opportunities.push(AvailableOpportunity {
            opportunity_id: "opportunity-0".to_owned(),
            start_seconds: 200,
            end_seconds: 400,
            max_service_duration_seconds: 200,
            max_payload_units: 10,
        });
        let errors = evaluate_nonanticipative_policy(&input)
            .expect_err("same request and opportunity were assigned earlier on path");
        assert!(errors.iter().any(|error| {
            error.contains("request_assigned_twice_on_scenario_path")
        }));
        assert!(errors.iter().any(|error| {
            error.contains("opportunity_assigned_twice_on_scenario_path")
        }));
    }

    #[test]
    fn shared_information_set_with_mismatched_observation_digest_is_rejected() {
        let mut input = fixture();
        let node = input.nodes.iter_mut().find(|node| node.node_id == "blind-b").unwrap();
        node.observation_digest = format!("sha256:{}", "e".repeat(64));
        let errors = evaluate_nonanticipative_policy(&input)
            .expect_err("same information set cannot refer to different observations");
        assert!(errors.iter().any(|error| {
            error.contains("information_set_digest_observation_mismatch")
        }));
    }

    #[test]
    fn scenario_probability_mass_must_be_exact() {
        let mut input = fixture();
        input.scenarios[0].weight_numerator = 2;
        let errors = evaluate_nonanticipative_policy(&input)
            .expect_err("scenario weights must sum to denominator");
        assert!(errors
            .iter()
            .any(|error| error.contains("scenario_probability_mass_mismatch")));
    }

    #[test]
    fn parent_time_cannot_move_backwards() {
        let mut input = fixture();
        input.nodes.iter_mut().find(|node| node.node_id == "blind-a").unwrap().decision_time_seconds = 0;
        let errors = evaluate_nonanticipative_policy(&input)
            .expect_err("a child decision cannot precede its parent");
        assert!(errors
            .iter()
            .any(|error| error.contains("policy_node_time_precedes_parent")));
    }

    #[test]
    fn input_permutation_preserves_canonical_report() {
        let input = fixture();
        let mut permuted = fixture();
        permuted.nodes.reverse();
        permuted.scenarios.reverse();
        for node in &mut permuted.nodes {
            node.available_requests.reverse();
            node.available_opportunities.reverse();
        }
        let first = evaluate_nonanticipative_policy(&input).expect("first");
        let second = evaluate_nonanticipative_policy(&permuted).expect("permuted");
        assert_eq!(
            serde_json::to_string(&first).expect("serialize first"),
            serde_json::to_string(&second).expect("serialize second")
        );
    }
}
