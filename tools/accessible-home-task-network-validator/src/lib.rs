// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fail-closed structural and semantic validation for synthetic task networks.
//! This validates consistency of claims, not their truth in a physical home.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

/// Versioned typed-constraint evaluator.
pub mod v03;

pub const VALIDATOR_VERSION: &str = "accessible-home-task-network-validator/0.1.0";
const FIXTURE: &str = include_str!("../../../docs/built-environment/accessible-home-task-network-v0.2.example.json");

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Network {
    pub schema_version: String,
    pub fixture_kind: String,
    pub network_id: String,
    pub claim_boundary: String,
    pub goal: Goal,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub paths: Vec<Path>,
    pub failure_modes: Vec<FailureMode>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Goal {
    pub id: String,
    pub resident_statement: String,
    pub observable_outcome: String,
    pub constraints: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: String,
    pub title: String,
    pub kind: NodeKind,
    pub required: bool,
    pub evidence_state: EvidenceState,
    pub evidence_ref: String,
    pub review_with: String,
    pub note: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    Action,
    Decision,
    Outcome,
    Handoff,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub relation: EdgeRelation,
    pub trigger: Condition,
    pub evidence_state: EvidenceState,
    pub evidence_ref: String,
    pub review_with: String,
    pub note: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EdgeRelation {
    Sequence,
    Requires,
    Alternative,
    Fallback,
    Recovery,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    Normal,
    NetworkUnavailable,
    PowerUnavailable,
    PrimaryDeviceUnavailable,
    SupportUnavailable,
    ResidentSelected,
    Other,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceState {
    VerifiedForDeclaredScope,
    KnownBarrier,
    Unknown,
    Proposed,
    NeedsSiteMeasurement,
    NeedsProfessionalReview,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Assessment {
    BlockedByDeclaredBarrier,
    NotEvaluable,
    EvidenceCompleteForFixture,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Path {
    pub id: String,
    pub title: String,
    pub condition: Condition,
    pub node_ids: Vec<String>,
    pub edge_ids: Vec<String>,
    pub expected_assessment: Assessment,
    pub success_condition: String,
    pub note: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FailureMode {
    pub id: String,
    pub title: String,
    pub trigger: Condition,
    pub affected_node_ids: Vec<String>,
    pub activated_path_ids: Vec<String>,
    pub expected_goal_state: GoalState,
    pub note: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GoalState {
    CandidatePathAvailableForFixture,
    NotEvaluable,
    NoCandidatePathProven,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PathResult {
    pub path_id: String,
    pub condition: Condition,
    pub assessment: Assessment,
    pub expected_assessment: Assessment,
    pub required_node_count: usize,
    pub edge_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct FailureModeResult {
    pub failure_mode_id: String,
    pub trigger: Condition,
    pub activated_path_ids: Vec<String>,
    pub actual_goal_state: GoalState,
    pub expected_goal_state: GoalState,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ValidationReport {
    pub report_schema_version: String,
    pub validator_version: String,
    pub network_id: String,
    pub fixture_kind: String,
    pub input_sha256: String,
    pub node_count: usize,
    pub edge_count: usize,
    pub path_count: usize,
    pub failure_mode_count: usize,
    pub path_results: Vec<PathResult>,
    pub failure_mode_results: Vec<FailureModeResult>,
    pub claim_limit: String,
}

/// Parses the input and emits a receipt only when structural, reference, path-state
/// and failure-mode expectations all agree. Any disagreement fails closed.
pub fn evaluate_bytes(input: &[u8]) -> Result<ValidationReport, Vec<String>> {
    let network: Network = serde_json::from_slice(input)
        .map_err(|error| vec![format!("invalid network JSON/shape: {error}")])?;
    let mut errors = validate_structure(&network);
    if !errors.is_empty() {
        return Err(errors);
    }

    let node_by_id: BTreeMap<&str, &Node> = network.nodes.iter().map(|node| (node.id.as_str(), node)).collect();
    let edge_by_id: BTreeMap<&str, &Edge> = network.edges.iter().map(|edge| (edge.id.as_str(), edge)).collect();
    let path_by_id: BTreeMap<&str, &Path> = network.paths.iter().map(|path| (path.id.as_str(), path)).collect();

    let path_results: Vec<PathResult> = network.paths.iter().map(|path| {
        let actual = assess_path(path, &node_by_id, &edge_by_id);
        if actual != path.expected_assessment {
            errors.push(format!(
                "path '{}' expected {:?}, evaluator produced {:?}",
                path.id, path.expected_assessment, actual
            ));
        }
        PathResult {
            path_id: path.id.clone(),
            condition: path.condition,
            assessment: actual,
            expected_assessment: path.expected_assessment,
            required_node_count: path.node_ids.iter()
                .filter_map(|id| node_by_id.get(id.as_str()))
                .filter(|node| node.required)
                .count(),
            edge_count: path.edge_ids.len(),
        }
    }).collect();

    let assessments: BTreeMap<&str, Assessment> = path_results.iter()
        .map(|result| (result.path_id.as_str(), result.assessment))
        .collect();

    let failure_mode_results: Vec<FailureModeResult> = network.failure_modes.iter().map(|failure| {
        let actual_goal_state = assess_failure_mode(failure, &path_by_id, &assessments);
        if actual_goal_state != failure.expected_goal_state {
            errors.push(format!(
                "failure mode '{}' expected {:?}, evaluator produced {:?}",
                failure.id, failure.expected_goal_state, actual_goal_state
            ));
        }
        FailureModeResult {
            failure_mode_id: failure.id.clone(),
            trigger: failure.trigger,
            activated_path_ids: failure.activated_path_ids.clone(),
            actual_goal_state,
            expected_goal_state: failure.expected_goal_state,
        }
    }).collect();

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(ValidationReport {
        report_schema_version: "0.1.0".to_owned(),
        validator_version: VALIDATOR_VERSION.to_owned(),
        network_id: network.network_id,
        fixture_kind: network.fixture_kind,
        input_sha256: hex_sha256(input),
        node_count: network.nodes.len(),
        edge_count: network.edges.len(),
        path_count: network.paths.len(),
        failure_mode_count: network.failure_modes.len(),
        path_results,
        failure_mode_results,
        claim_limit: "SYNTHETIC CONTRACT ONLY: this receipt validates internal consistency of the declared fixture. Resident constraints are prose in v0.2 and are not machine-verified by this validator. It does not establish the truth of evidence, real-home accessibility, usability, dimensions, device function, security, life-safety, suitability, or legal compliance.".to_owned(),
    })
}

fn validate_structure(network: &Network) -> Vec<String> {
    let mut errors = Vec::new();
    if network.schema_version != "0.2.0" {
        errors.push(format!("unsupported schema_version '{}'", network.schema_version));
    }
    if network.fixture_kind != "synthetic_task_network" {
        errors.push("fixture_kind must be synthetic_task_network; real-home evidence is not accepted".to_owned());
    }
    if !valid_id(&network.network_id) {
        errors.push("network_id is invalid".to_owned());
    }
    if network.claim_boundary.trim().is_empty()
        || !network.claim_boundary.to_ascii_lowercase().contains("synthetic")
    {
        errors.push("claim_boundary must explicitly describe synthetic scope".to_owned());
    }
    require_text(&mut errors, "goal.id", &network.goal.id);
    require_text(&mut errors, "goal.resident_statement", &network.goal.resident_statement);
    require_text(&mut errors, "goal.observable_outcome", &network.goal.observable_outcome);
    if network.goal.constraints.is_empty() {
        errors.push("goal must declare at least one resident constraint".to_owned());
    }
    let mut constraint_set = BTreeSet::new();
    for (index, constraint) in network.goal.constraints.iter().enumerate() {
        require_text(&mut errors, &format!("goal.constraints[{index}]"), constraint);
        if !constraint_set.insert(constraint.trim()) {
            errors.push(format!("duplicate goal constraint at index {index}"));
        }
    }
    if network.nodes.is_empty() || network.edges.is_empty() || network.paths.is_empty() {
        errors.push("network must have nodes, edges and paths".to_owned());
    }

    // IDs share one namespace so a reference cannot ambiguously resolve across kinds.
    let mut all_ids = BTreeSet::new();
    register_id(&mut errors, &mut all_ids, "network", &network.network_id);
    register_id(&mut errors, &mut all_ids, "goal", &network.goal.id);
    for node in &network.nodes {
        register_id(&mut errors, &mut all_ids, "node", &node.id);
        require_text(&mut errors, &format!("node '{}' title", node.id), &node.title);
        require_text(&mut errors, &format!("node '{}' evidence_ref", node.id), &node.evidence_ref);
        require_text(&mut errors, &format!("node '{}' review_with", node.id), &node.review_with);
        require_text(&mut errors, &format!("node '{}' note", node.id), &node.note);
    }
    for edge in &network.edges {
        register_id(&mut errors, &mut all_ids, "edge", &edge.id);
        require_text(&mut errors, &format!("edge '{}' from", edge.id), &edge.from);
        require_text(&mut errors, &format!("edge '{}' to", edge.id), &edge.to);
        require_text(&mut errors, &format!("edge '{}' evidence_ref", edge.id), &edge.evidence_ref);
        require_text(&mut errors, &format!("edge '{}' review_with", edge.id), &edge.review_with);
        require_text(&mut errors, &format!("edge '{}' note", edge.id), &edge.note);
        if matches!(edge.relation, EdgeRelation::Fallback | EdgeRelation::Recovery)
            && edge.trigger == Condition::Normal
        {
            errors.push(format!("fallback/recovery edge '{}' needs a non-normal trigger", edge.id));
        }
    }
    for path in &network.paths {
        register_id(&mut errors, &mut all_ids, "path", &path.id);
        require_text(&mut errors, &format!("path '{}' title", path.id), &path.title);
        require_text(&mut errors, &format!("path '{}' success_condition", path.id), &path.success_condition);
        require_text(&mut errors, &format!("path '{}' note", path.id), &path.note);
    }
    for failure in &network.failure_modes {
        register_id(&mut errors, &mut all_ids, "failure mode", &failure.id);
        require_text(&mut errors, &format!("failure mode '{}' title", failure.id), &failure.title);
        require_text(&mut errors, &format!("failure mode '{}' note", failure.id), &failure.note);
    }
    
    if !errors.is_empty() {
        // Type/ID failures are already actionable; continue only once IDs are safe to index.
        // The complete list is returned below so callers see all first-order defects.
        return errors;
    }

    let node_by_id: BTreeMap<&str, &Node> = network.nodes.iter().map(|node| (node.id.as_str(), node)).collect();
    let edge_by_id: BTreeMap<&str, &Edge> = network.edges.iter().map(|edge| (edge.id.as_str(), edge)).collect();
    let path_by_id: BTreeMap<&str, &Path> = network.paths.iter().map(|path| (path.id.as_str(), path)).collect();

    for edge in &network.edges {
        if !node_by_id.contains_key(edge.from.as_str()) {
            errors.push(format!("edge '{}' has dangling from-node '{}'", edge.id, edge.from));
        }
        if !node_by_id.contains_key(edge.to.as_str()) {
            errors.push(format!("edge '{}' has dangling to-node '{}'", edge.id, edge.to));
        }
    }

    let mut used_nodes = BTreeSet::new();
    let mut used_edges = BTreeSet::new();
    for path in &network.paths {
        if path.node_ids.len() < 2 {
            errors.push(format!("path '{}' must contain at least two ordered nodes", path.id));
        }
        if path.edge_ids.len() + 1 != path.node_ids.len() {
            errors.push(format!("path '{}' must name exactly one edge between each adjacent node pair", path.id));
        }
        if !unique(&path.node_ids) {
            errors.push(format!("path '{}' repeats a node id", path.id));
        }
        if !unique(&path.edge_ids) {
            errors.push(format!("path '{}' repeats an edge id", path.id));
        }
        for node_id in &path.node_ids {
            used_nodes.insert(node_id.as_str());
            if !node_by_id.contains_key(node_id.as_str()) {
                errors.push(format!("path '{}' references missing node '{}'", path.id, node_id));
            }
        }
        for edge_id in &path.edge_ids {
            used_edges.insert(edge_id.as_str());
            if !edge_by_id.contains_key(edge_id.as_str()) {
                errors.push(format!("path '{}' references missing edge '{}'", path.id, edge_id));
            }
        }
        if let Some(last_id) = path.node_ids.last() {
            if let Some(last_node) = node_by_id.get(last_id.as_str()) {
                if last_node.kind != NodeKind::Outcome {
                    errors.push(format!("path '{}' must end at an outcome node", path.id));
                }
            }
        }
        for index in 0..path.edge_ids.len().min(path.node_ids.len().saturating_sub(1)) {
            let Some(edge) = edge_by_id.get(path.edge_ids[index].as_str()) else { continue };
            if edge.from != path.node_ids[index] || edge.to != path.node_ids[index + 1] {
                errors.push(format!(
                    "path '{}' edge '{}' does not connect ordered nodes '{}' -> '{}'",
                    path.id, edge.id, path.node_ids[index], path.node_ids[index + 1]
                ));
            }
            if edge.trigger != Condition::Normal && edge.trigger != path.condition {
                errors.push(format!("path '{}' contains edge '{}' with incompatible trigger", path.id, edge.id));
            }
            if matches!(edge.relation, EdgeRelation::Fallback | EdgeRelation::Recovery)
                && edge.trigger != path.condition
            {
                errors.push(format!("path '{}' fallback/recovery edge '{}' must match path condition", path.id, edge.id));
            }
        }
    }
    for node in &network.nodes {
        if !used_nodes.contains(node.id.as_str()) {
            errors.push(format!("node '{}' is not used by any path", node.id));
        }
    }
    for edge in &network.edges {
        if !used_edges.contains(edge.id.as_str()) {
            errors.push(format!("edge '{}' is not used by any path", edge.id));
        }
    }

    for failure in &network.failure_modes {
        if failure.activated_path_ids.is_empty() {
            errors.push(format!("failure mode '{}' must activate at least one path", failure.id));
        }
        if !unique(&failure.affected_node_ids) {
            errors.push(format!("failure mode '{}' repeats an affected node", failure.id));
        }
        if !unique(&failure.activated_path_ids) {
            errors.push(format!("failure mode '{}' repeats an activated path", failure.id));
        }
        let mut activated = BTreeSet::new();
        for path_id in &failure.activated_path_ids {
            let Some(path) = path_by_id.get(path_id.as_str()) else {
                errors.push(format!("failure mode '{}' references missing path '{}'", failure.id, path_id));
                continue;
            };
            activated.insert(path.id.as_str());
            if path.condition != failure.trigger {
                errors.push(format!(
                    "failure mode '{}' trigger does not match activated path '{}'",
                    failure.id, path.id
                ));
            }
        }
        for node_id in &failure.affected_node_ids {
            if !node_by_id.contains_key(node_id.as_str()) {
                errors.push(format!("failure mode '{}' references missing node '{}'", failure.id, node_id));
                continue;
            }
            let appears_on_degraded_primary = network.paths.iter().any(|path| {
                !activated.contains(path.id.as_str())
                    && path.condition != failure.trigger
                    && path.node_ids.iter().any(|id| id == node_id)
            });
            if !appears_on_degraded_primary {
                errors.push(format!(
                    "failure mode '{}' affected node '{}' does not occur on a non-activated path",
                    failure.id, node_id
                ));
            }
        }
    }
    errors
}

fn assess_path(
    path: &Path,
    nodes: &BTreeMap<&str, &Node>,
    edges: &BTreeMap<&str, &Edge>,
) -> Assessment {
    let mut states = Vec::new();
    for node_id in &path.node_ids {
        if let Some(node) = nodes.get(node_id.as_str()) {
            states.push((node.evidence_state, node.required));
        }
    }
    // Edges within a candidate path are required dependencies in v0.2. Advisory
    // edges would need an explicit field in a future schema version.
    for edge_id in &path.edge_ids {
        if let Some(edge) = edges.get(edge_id.as_str()) {
            states.push((edge.evidence_state, true));
        }
    }
    assess_evidence_states(&states)
}

/// Pure evidence-state rule, exported for exhaustive finite-state/property tests.
pub fn assess_evidence_states(states: &[(EvidenceState, bool)]) -> Assessment {
    let has_required_barrier = states.iter().any(|(state, required)| {
        *required && *state == EvidenceState::KnownBarrier
    });
    if has_required_barrier {
        return Assessment::BlockedByDeclaredBarrier;
    }
    let has_required_gap = states.iter().any(|(state, required)| {
        *required && *state != EvidenceState::VerifiedForDeclaredScope
    });
    if has_required_gap {
        Assessment::NotEvaluable
    } else {
        Assessment::EvidenceCompleteForFixture
    }
}

fn assess_failure_mode(
    failure: &FailureMode,
    paths: &BTreeMap<&str, &Path>,
    assessments: &BTreeMap<&str, Assessment>,
) -> GoalState {
    let states: Vec<Assessment> = failure.activated_path_ids.iter()
        .filter_map(|id| paths.get(id.as_str()).and_then(|_| assessments.get(id.as_str()).copied()))
        .collect();
    if states.iter().any(|state| *state == Assessment::EvidenceCompleteForFixture) {
        GoalState::CandidatePathAvailableForFixture
    } else if states.iter().any(|state| *state == Assessment::NotEvaluable) {
        GoalState::NotEvaluable
    } else {
        GoalState::NoCandidatePathProven
    }
}

fn require_text(errors: &mut Vec<String>, label: &str, value: &str) {
    if value.trim().is_empty() {
        errors.push(format!("{label} must be non-empty"));
    }
}


fn register_id(errors: &mut Vec<String>, all_ids: &mut BTreeSet<String>, kind: &str, id: &str) {
    if !valid_id(id) {
        errors.push(format!("{kind} id '{id}' is invalid"));
    }
    if !all_ids.insert(id.to_owned()) {
        errors.push(format!("duplicate global id '{id}' ({kind})"));
    }
}

fn unique(values: &[String]) -> bool {
    let mut seen = BTreeSet::new();
    values.iter().all(|value| seen.insert(value.as_str()))
}

fn valid_id(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() || bytes.len() > 80 {
        return false;
    }
    if !(bytes[0].is_ascii_lowercase() || bytes[0].is_ascii_digit()) {
        return false;
    }
    bytes.iter().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}

fn hex_sha256(input: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(input);
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn fixture_value() -> Value {
        serde_json::from_str(FIXTURE).expect("repository fixture must be valid JSON")
    }

    fn evaluate_value(value: &Value) -> Result<ValidationReport, Vec<String>> {
        let bytes = serde_json::to_vec(value).expect("test fixture serializes");
        evaluate_bytes(&bytes)
    }

    fn change_evidence(value: &mut Value, node_id: &str, state: &str) {
        let nodes = value["nodes"].as_array_mut().expect("nodes array");
        let node = nodes.iter_mut().find(|node| node["id"].as_str() == Some(node_id)).expect("node exists");
        node["evidence_state"] = Value::String(state.to_owned());
    }

    #[test]
    fn baseline_matches_all_path_and_failure_expectations() {
        let report = evaluate_bytes(FIXTURE.as_bytes()).expect("baseline should qualify");
        assert_eq!(report.node_count, 5);
        assert_eq!(report.edge_count, 5);
        assert_eq!(report.path_count, 2);
        assert_eq!(report.failure_mode_count, 1);
        assert_eq!(report.path_results[0].assessment, Assessment::NotEvaluable);
        assert_eq!(report.path_results[1].assessment, Assessment::EvidenceCompleteForFixture);
        assert_eq!(report.failure_mode_results[0].actual_goal_state, GoalState::CandidatePathAvailableForFixture);
        assert_eq!(report.input_sha256, hex_sha256(FIXTURE.as_bytes()));
        assert!(report.claim_limit.contains("does not establish"));
    }

    #[test]
    fn all_required_evidence_states_obey_fail_closed_rules() {
        let cases = [
            (EvidenceState::VerifiedForDeclaredScope, Assessment::EvidenceCompleteForFixture),
            (EvidenceState::KnownBarrier, Assessment::BlockedByDeclaredBarrier),
            (EvidenceState::Unknown, Assessment::NotEvaluable),
            (EvidenceState::Proposed, Assessment::NotEvaluable),
            (EvidenceState::NeedsSiteMeasurement, Assessment::NotEvaluable),
            (EvidenceState::NeedsProfessionalReview, Assessment::NotEvaluable),
        ];
        for (state, expected) in cases {
            assert_eq!(assess_evidence_states(&[(state, true)]), expected, "state {state:?}");
        }
    }

    #[test]
    fn required_barrier_takes_precedence_over_unknown() {
        assert_eq!(
            assess_evidence_states(&[
                (EvidenceState::Unknown, true),
                (EvidenceState::KnownBarrier, true),
            ]),
            Assessment::BlockedByDeclaredBarrier
        );
    }

    #[test]
    fn optional_unknown_or_barrier_does_not_gate_a_path() {
        for optional_state in [EvidenceState::Unknown, EvidenceState::KnownBarrier, EvidenceState::NeedsProfessionalReview] {
            assert_eq!(
                assess_evidence_states(&[
                    (EvidenceState::VerifiedForDeclaredScope, true),
                    (optional_state, false),
                ]),
                Assessment::EvidenceCompleteForFixture
            );
        }
    }

    #[test]
    fn duplicate_ids_across_different_entity_kinds_are_rejected() {
        let mut value = fixture_value();
        let colliding_id = value["nodes"][0]["id"].clone();
        value["edges"][0]["id"] = colliding_id;
        let error = evaluate_value(&value).unwrap_err().join("\n");
        assert!(error.contains("duplicate global id"));
    }

    #[test]
    fn dangling_edge_endpoint_is_rejected() {
        let mut value = fixture_value();
        value["edges"][0]["to"] = Value::String("missing-node".to_owned());
        let error = evaluate_value(&value).unwrap_err().join("\n");
        assert!(error.contains("dangling to-node"));
    }

    #[test]
    fn dangling_path_reference_is_rejected() {
        let mut value = fixture_value();
        value["paths"][0]["node_ids"][1] = Value::String("missing-node".to_owned());
        let error = evaluate_value(&value).unwrap_err().join("\n");
        assert!(error.contains("references missing node"));
    }

    #[test]
    fn incorrect_edge_order_is_rejected() {
        let mut value = fixture_value();
        value["paths"][0]["edge_ids"].as_array_mut().unwrap().swap(0, 1);
        let error = evaluate_value(&value).unwrap_err().join("\n");
        assert!(error.contains("does not connect ordered nodes"));
    }

    #[test]
    fn edge_count_must_match_adjacent_node_pairs() {
        let mut value = fixture_value();
        value["paths"][0]["edge_ids"].as_array_mut().unwrap().pop();
        let error = evaluate_value(&value).unwrap_err().join("\n");
        assert!(error.contains("exactly one edge between each adjacent node pair"));
    }

    #[test]
    fn outcome_must_terminate_each_path() {
        let mut value = fixture_value();
        value["nodes"][4]["kind"] = Value::String("action".to_owned());
        let error = evaluate_value(&value).unwrap_err().join("\n");
        assert!(error.contains("must end at an outcome node"));
    }

    #[test]
    fn fallback_without_degraded_trigger_is_rejected() {
        let mut value = fixture_value();
        value["edges"][3]["trigger"] = Value::String("normal".to_owned());
        let error = evaluate_value(&value).unwrap_err().join("\n");
        assert!(error.contains("needs a non-normal trigger"));
    }

    #[test]
    fn required_evidence_downgrade_cannot_keep_fixture_complete_status() {
        let mut value = fixture_value();
        change_evidence(&mut value, "local-tactile-signal", "unknown");
        assert_eq!(
            assess_evidence_states(&[(EvidenceState::Unknown, true)]),
            Assessment::NotEvaluable
        );
        let error = evaluate_value(&value).unwrap_err().join("\n");
        assert!(error.contains("expected EvidenceCompleteForFixture"));
    }

    #[test]
    fn optional_unknown_node_does_not_make_a_path_fail() {
        let mut value = fixture_value();
        let node = value["nodes"].as_array_mut().unwrap()
            .iter_mut().find(|node| node["id"].as_str() == Some("local-tactile-signal")).unwrap();
        node["required"] = Value::Bool(false);
        node["evidence_state"] = Value::String("unknown".to_owned());
        let report = evaluate_value(&value).expect("optional node is advisory in this contract");
        assert_eq!(report.path_results[1].assessment, Assessment::EvidenceCompleteForFixture);
    }

    #[test]
    fn failure_mode_cannot_keep_stale_candidate_claim_after_fallback_downgrade() {
        let mut value = fixture_value();
        change_evidence(&mut value, "local-tactile-signal", "needs_professional_review");
        let error = evaluate_value(&value).unwrap_err().join("\n");
        assert!(error.contains("expected EvidenceCompleteForFixture"));
        assert!(error.contains("expected CandidatePathAvailableForFixture"));
    }

    #[test]
    fn unknown_evidence_enum_is_rejected_during_parse() {
        let mut value = fixture_value();
        change_evidence(&mut value, "local-tactile-signal", "probably_safe");
        let bytes = serde_json::to_vec(&value).unwrap();
        let error = evaluate_bytes(&bytes).unwrap_err().join("\n");
        assert!(error.contains("invalid network JSON/shape"));
    }

    #[test]
    fn real_home_kind_is_not_accepted() {
        let mut value = fixture_value();
        value["fixture_kind"] = Value::String("real_home".to_owned());
        let error = evaluate_value(&value).unwrap_err().join("\n");
        assert!(error.contains("fixture_kind must be synthetic_task_network"));
    }

    #[test]
    fn empty_activated_path_set_is_rejected() {
        let mut value = fixture_value();
        value["failure_modes"][0]["activated_path_ids"] = Value::Array(vec![]);
        let error = evaluate_value(&value).unwrap_err().join("\n");
        assert!(error.contains("must activate at least one path"));
    }

    #[test]
    fn deterministic_input_produces_identical_receipt() {
        let first = evaluate_bytes(FIXTURE.as_bytes()).expect("valid baseline");
        let second = evaluate_bytes(FIXTURE.as_bytes()).expect("valid baseline");
        assert_eq!(first, second);
        assert_eq!(serde_json::to_vec(&first).unwrap(), serde_json::to_vec(&second).unwrap());
    }
}


/// Dispatches supported synthetic network schema versions to their matching
/// fail-closed evaluator. Reports are returned as JSON only when expectation
/// checks succeed.
pub fn evaluate_any_bytes(input: &[u8]) -> Result<serde_json::Value, Vec<String>> {
    let header: serde_json::Value = serde_json::from_slice(input)
        .map_err(|error| vec![format!("invalid network JSON: {error}")])?;
    let schema_version = header.get("schema_version")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| vec!["schema_version must be a string".to_owned()])?;

    match schema_version {
        "0.2.0" => {
            let report = evaluate_fixture_bytes(input).map_err(|error| vec![error])?;
            serde_json::to_value(report)
                .map_err(|error| vec![format!("could not serialize v0.2 receipt: {error}")])
        }
        "0.3.0" => {
            let report = v03::evaluate_bytes(input)?;
            serde_json::to_value(report)
                .map_err(|error| vec![format!("could not serialize v0.3 receipt: {error}")])
        }
        other => Err(vec![format!("unsupported schema_version '{other}'")]),
    }
}


#[cfg(test)]
mod version_dispatch_tests {
    use super::*;

    const V02_FIXTURE: &str = include_str!("../../../docs/built-environment/accessible-home-task-network-v0.2.example.json");
    const V03_FIXTURE: &str = include_str!("../../../docs/built-environment/accessible-home-task-network-v0.3.example.json");

    #[test]
    fn cli_dispatch_accepts_v02_network_contract() {
        let report = evaluate_any_bytes(V02_FIXTURE.as_bytes()).expect("v0.2 fixture should dispatch");
        assert_eq!(report["path_count"].as_u64(), Some(2));
    }

    #[test]
    fn cli_dispatch_accepts_v03_typed_constraints_contract() {
        let report = evaluate_any_bytes(V03_FIXTURE.as_bytes()).expect("v0.3 fixture should dispatch");
        assert_eq!(report["constraint_count"].as_u64(), Some(3));
        assert_eq!(report["path_count"].as_u64(), Some(2));
    }

    #[test]
    fn cli_dispatch_rejects_unknown_schema_version() {
        let invalid = br#"{"schema_version":"99.0.0"}"#;
        let error = evaluate_any_bytes(invalid).unwrap_err().join("\n");
        assert!(error.contains("unsupported schema_version"));
    }
}
