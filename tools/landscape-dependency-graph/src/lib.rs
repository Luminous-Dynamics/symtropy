// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic-only structural analysis of a landscape's social-ecological-technical
//! dependency graph. This is not a physical model or a causal inference engine.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const FIXTURE_SCHEMA: &str = "symtropy.landscape_dependency_graph_fixture.v0.1";
const REPORT_SCHEMA: &str = "symtropy.landscape_dependency_graph_report.v0.1";
const SUITE_ID: &str = "LANDSCAPE-SETS-DEPENDENCY-GRAPH-V0.1";
const CLAIM_CEILING: &str = "STRUCTURAL_DEPENDENCY_ANALYSIS_ONLY";
const MAX_NODES: usize = 100;
const MAX_DEPENDENCIES: usize = 500;
const MAX_SCENARIOS: usize = 20;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    pub schema_version: String,
    pub suite_id: String,
    pub data_origin: String,
    pub claim_ceiling: String,
    pub dependency_semantics_sha256: String,
    pub graph_id: String,
    pub nodes: Vec<Node>,
    pub dependencies: Vec<Dependency>,
    pub scenarios: Vec<Scenario>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub node_id: String,
    pub label: String,
    pub layer: String,
    pub is_service: bool,
    pub required_dependency_groups: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub edge_id: String,
    pub dependent_node_id: String,
    pub provider_node_id: String,
    pub requirement_group_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub scenario_id: String,
    pub label: String,
    pub failed_node_ids: Vec<String>,
    pub disabled_edge_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProviderAssessment {
    pub edge_id: String,
    pub provider_node_id: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UnsatisfiedGroup {
    pub requirement_group_id: String,
    pub provider_alternatives: Vec<ProviderAssessment>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NodeAssessment {
    pub node_id: String,
    pub layer: String,
    pub is_service: bool,
    pub operational: bool,
    pub self_failed: bool,
    pub unsatisfied_groups: Vec<UnsatisfiedGroup>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScenarioAssessment {
    pub scenario_id: String,
    pub label: String,
    pub injected_failure_count: usize,
    pub service_count: usize,
    pub supported_service_count: usize,
    pub unsupported_service_count: usize,
    pub service_availability_fraction: f64,
    /// Unsupported service count / injected node-or-edge failures; null if no injection.
    pub cascade_amplification: Option<f64>,
    pub unsupported_service_ids: Vec<String>,
    pub node_assessments: Vec<NodeAssessment>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SingleFailureImpact {
    pub failure_kind: String,
    pub target_id: String,
    pub baseline_supported_service_count: usize,
    pub supported_service_count: usize,
    pub lost_service_count: usize,
    pub relative_service_loss_fraction: Option<f64>,
    pub lost_service_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    pub schema_version: String,
    pub suite_id: String,
    pub conformance_status: String,
    pub data_origin: String,
    pub claim_ceiling: String,
    pub graph_id: String,
    pub input_sha256: String,
    pub dependency_semantics_sha256: String,
    pub node_count: usize,
    pub dependency_count: usize,
    pub scenario_count: usize,
    pub service_count: usize,
    pub cross_layer_dependency_count: usize,
    pub baseline_supported_service_count: usize,
    pub single_failure_audit: Vec<SingleFailureImpact>,
    pub scenarios: Vec<ScenarioAssessment>,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|ch| ch.is_ascii_digit() || ('a'..='f').contains(&ch))
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

pub fn validate_fixture(fixture: &Fixture) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if fixture.schema_version != FIXTURE_SCHEMA {
        errors.push(format!("unsupported schema_version: {}", fixture.schema_version));
    }
    if fixture.suite_id != SUITE_ID {
        errors.push(format!("unexpected suite_id: {}", fixture.suite_id));
    }
    if fixture.data_origin != "synthetic_fixture" {
        errors.push("v0.1 accepts synthetic_fixture only".to_owned());
    }
    if fixture.claim_ceiling != CLAIM_CEILING {
        errors.push(format!("claim_ceiling must be {CLAIM_CEILING}"));
    }
    if !valid_digest(&fixture.dependency_semantics_sha256) {
        errors.push("dependency_semantics_sha256 must be a 64-character lowercase SHA-256".to_owned());
    }
    if !nonempty(&fixture.graph_id) {
        errors.push("graph_id must be a non-empty string".to_owned());
    }
    if fixture.nodes.is_empty() || fixture.nodes.len() > MAX_NODES {
        errors.push(format!("nodes must contain 1..={MAX_NODES} entries"));
    }
    if fixture.dependencies.len() > MAX_DEPENDENCIES {
        errors.push(format!("dependencies must not exceed {MAX_DEPENDENCIES}"));
    }
    if fixture.scenarios.is_empty() || fixture.scenarios.len() > MAX_SCENARIOS {
        errors.push(format!("scenarios must contain 1..={MAX_SCENARIOS} entries"));
    }

    let mut node_ids = BTreeSet::new();
    let mut service_count = 0usize;
    for node in &fixture.nodes {
        if !nonempty(&node.node_id) || !nonempty(&node.label) {
            errors.push("node IDs and labels must be non-empty".to_owned());
        }
        if !matches!(node.layer.as_str(), "social" | "ecological" | "technical") {
            errors.push(format!(
                "{}: layer must be social, ecological, or technical",
                node.node_id
            ));
        }
        if !node_ids.insert(node.node_id.as_str()) {
            errors.push(format!("duplicate node_id: {}", node.node_id));
        }
        if node.is_service {
            service_count += 1;
        }
        let mut groups = BTreeSet::new();
        for group in &node.required_dependency_groups {
            if !nonempty(group) {
                errors.push(format!("{}: dependency group ID must be non-empty", node.node_id));
            }
            if !groups.insert(group.as_str()) {
                errors.push(format!(
                    "{}: duplicate required dependency group {group}",
                    node.node_id
                ));
            }
        }
    }
    if service_count == 0 {
        errors.push("graph must define at least one service node".to_owned());
    }

    let mut edge_ids = BTreeSet::new();
    let mut edge_triples = BTreeSet::new();
    let mut groups_with_edges: BTreeSet<(String, String)> = BTreeSet::new();
    let mut adjacency: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for node in &fixture.nodes {
        adjacency.entry(node.node_id.clone()).or_default();
    }
    for edge in &fixture.dependencies {
        if !nonempty(&edge.edge_id) || !nonempty(&edge.requirement_group_id) {
            errors.push("dependency edge IDs and requirement groups must be non-empty".to_owned());
        }
        if !edge_ids.insert(edge.edge_id.as_str()) {
            errors.push(format!("duplicate edge_id: {}", edge.edge_id));
        }
        if !node_ids.contains(edge.dependent_node_id.as_str()) {
            errors.push(format!("{}: unknown dependent node", edge.edge_id));
        }
        if !node_ids.contains(edge.provider_node_id.as_str()) {
            errors.push(format!("{}: unknown provider node", edge.edge_id));
        }
        if edge.dependent_node_id == edge.provider_node_id {
            errors.push(format!("{}: self-dependency is forbidden", edge.edge_id));
        }
        if !edge_triples.insert((
            edge.dependent_node_id.as_str(),
            edge.provider_node_id.as_str(),
            edge.requirement_group_id.as_str(),
        )) {
            errors.push(format!("{}: duplicate dependent/provider/group relation", edge.edge_id));
        }
        if let Some(dependent) = fixture.nodes.iter().find(|n| n.node_id == edge.dependent_node_id) {
            if !dependent.required_dependency_groups.contains(&edge.requirement_group_id) {
                errors.push(format!(
                    "{}: edge group {} is not declared by dependent node {}",
                    edge.edge_id, edge.requirement_group_id, edge.dependent_node_id
                ));
            }
            groups_with_edges.insert((
                edge.dependent_node_id.clone(),
                edge.requirement_group_id.clone(),
            ));
        }
        adjacency
            .entry(edge.dependent_node_id.clone())
            .or_default()
            .push(edge.provider_node_id.clone());
    }
    for node in &fixture.nodes {
        for group in &node.required_dependency_groups {
            if !groups_with_edges.contains(&(node.node_id.clone(), group.clone())) {
                errors.push(format!(
                    "{}: required group {group} has no provider edges",
                    node.node_id
                ));
            }
        }
    }
    let mut temporary = BTreeSet::new();
    let mut permanent = BTreeSet::new();
    for node in &fixture.nodes {
        if graph_has_cycle(&node.node_id, &adjacency, &mut temporary, &mut permanent) {
            errors.push(format!(
                "dependency graph contains a cycle reachable from {}",
                node.node_id
            ));
            break;
        }
    }

    let mut scenario_ids = BTreeSet::new();
    for scenario in &fixture.scenarios {
        if !nonempty(&scenario.scenario_id) || !nonempty(&scenario.label) {
            errors.push("scenario IDs and labels must be non-empty".to_owned());
        }
        if !scenario_ids.insert(scenario.scenario_id.as_str()) {
            errors.push(format!("duplicate scenario_id: {}", scenario.scenario_id));
        }
        let mut failed = BTreeSet::new();
        for node_id in &scenario.failed_node_ids {
            if !node_ids.contains(node_id.as_str()) {
                errors.push(format!("{}: unknown failed node {node_id}", scenario.scenario_id));
            }
            if !failed.insert(node_id.as_str()) {
                errors.push(format!("{}: duplicate failed node {node_id}", scenario.scenario_id));
            }
        }
        let mut disabled = BTreeSet::new();
        for edge_id in &scenario.disabled_edge_ids {
            if !edge_ids.contains(edge_id.as_str()) {
                errors.push(format!("{}: unknown disabled edge {edge_id}", scenario.scenario_id));
            }
            if !disabled.insert(edge_id.as_str()) {
                errors.push(format!("{}: duplicate disabled edge {edge_id}", scenario.scenario_id));
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        errors.sort();
        Err(errors)
    }
}

fn graph_has_cycle(
    node_id: &str,
    adjacency: &BTreeMap<String, Vec<String>>,
    temporary: &mut BTreeSet<String>,
    permanent: &mut BTreeSet<String>,
) -> bool {
    if permanent.contains(node_id) {
        return false;
    }
    if !temporary.insert(node_id.to_owned()) {
        return true;
    }
    if let Some(providers) = adjacency.get(node_id) {
        for provider in providers {
            if graph_has_cycle(provider, adjacency, temporary, permanent) {
                return true;
            }
        }
    }
    temporary.remove(node_id);
    permanent.insert(node_id.to_owned());
    false
}

#[allow(clippy::too_many_arguments)]
fn node_operational(
    node_id: &str,
    nodes: &BTreeMap<&str, &Node>,
    dependencies: &[Dependency],
    failed_nodes: &BTreeSet<String>,
    disabled_edges: &BTreeSet<String>,
    memo: &mut BTreeMap<String, bool>,
    visiting: &mut BTreeSet<String>,
) -> bool {
    if let Some(value) = memo.get(node_id) {
        return *value;
    }
    if failed_nodes.contains(node_id) {
        memo.insert(node_id.to_owned(), false);
        return false;
    }
    if !visiting.insert(node_id.to_owned()) {
        return false;
    }
    let node = nodes[node_id];
    let mut operational = true;
    for group in &node.required_dependency_groups {
        let group_operational = dependencies.iter().any(|edge| {
            edge.dependent_node_id == node_id
                && edge.requirement_group_id == *group
                && !disabled_edges.contains(&edge.edge_id)
                && node_operational(
                    &edge.provider_node_id,
                    nodes,
                    dependencies,
                    failed_nodes,
                    disabled_edges,
                    memo,
                    visiting,
                )
        });
        if !group_operational {
            operational = false;
            break;
        }
    }
    visiting.remove(node_id);
    memo.insert(node_id.to_owned(), operational);
    operational
}

fn evaluate_scenario(fixture: &Fixture, scenario: &Scenario) -> ScenarioAssessment {
    let nodes: BTreeMap<&str, &Node> = fixture
        .nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node))
        .collect();
    let failed_nodes: BTreeSet<String> = scenario.failed_node_ids.iter().cloned().collect();
    let disabled_edges: BTreeSet<String> = scenario.disabled_edge_ids.iter().cloned().collect();
    let mut memo = BTreeMap::new();
    let mut visiting = BTreeSet::new();
    for node in &fixture.nodes {
        node_operational(
            &node.node_id,
            &nodes,
            &fixture.dependencies,
            &failed_nodes,
            &disabled_edges,
            &mut memo,
            &mut visiting,
        );
    }

    let mut node_assessments = Vec::with_capacity(fixture.nodes.len());
    let mut unsupported_service_ids = Vec::new();
    let services: Vec<&Node> = fixture.nodes.iter().filter(|node| node.is_service).collect();
    for node in &fixture.nodes {
        let operational = memo.get(&node.node_id).copied().unwrap_or(false);
        let self_failed = failed_nodes.contains(&node.node_id);
        let mut unsatisfied_groups = Vec::new();
        if !self_failed {
            for group in &node.required_dependency_groups {
                let provider_alternatives: Vec<ProviderAssessment> = fixture
                    .dependencies
                    .iter()
                    .filter(|edge| {
                        edge.dependent_node_id == node.node_id
                            && edge.requirement_group_id == *group
                    })
                    .map(|edge| {
                        let status = if disabled_edges.contains(&edge.edge_id) {
                            "edge_disabled"
                        } else if failed_nodes.contains(&edge.provider_node_id) {
                            "node_failed"
                        } else if memo.get(&edge.provider_node_id).copied().unwrap_or(false) {
                            "operational"
                        } else {
                            "dependency_unavailable"
                        };
                        ProviderAssessment {
                            edge_id: edge.edge_id.clone(),
                            provider_node_id: edge.provider_node_id.clone(),
                            status: status.to_owned(),
                        }
                    })
                    .collect();
                let group_ok = provider_alternatives
                    .iter()
                    .any(|candidate| candidate.status == "operational");
                if !group_ok {
                    unsatisfied_groups.push(UnsatisfiedGroup {
                        requirement_group_id: group.clone(),
                        provider_alternatives,
                    });
                }
            }
        }
        if node.is_service && !operational {
            unsupported_service_ids.push(node.node_id.clone());
        }
        node_assessments.push(NodeAssessment {
            node_id: node.node_id.clone(),
            layer: node.layer.clone(),
            is_service: node.is_service,
            operational,
            self_failed,
            unsatisfied_groups,
        });
    }
    node_assessments.sort_by(|left, right| left.node_id.cmp(&right.node_id));
    unsupported_service_ids.sort();
    let supported_service_count = services.len() - unsupported_service_ids.len();
    let unsupported_service_count = unsupported_service_ids.len();
    let injected_failure_count = failed_nodes.len() + disabled_edges.len();
    let cascade_amplification = if injected_failure_count == 0 {
        None
    } else {
        Some(rounded(
            unsupported_service_count as f64 / injected_failure_count as f64,
        ))
    };
    ScenarioAssessment {
        scenario_id: scenario.scenario_id.clone(),
        label: scenario.label.clone(),
        injected_failure_count,
        service_count: services.len(),
        supported_service_count,
        unsupported_service_count,
        service_availability_fraction: rounded(
            supported_service_count as f64 / services.len() as f64,
        ),
        cascade_amplification,
        unsupported_service_ids,
        node_assessments,
    }
}

fn evaluate_single_failure_audit(fixture: &Fixture) -> (usize, Vec<SingleFailureImpact>) {
    let baseline = Scenario {
        scenario_id: "single-failure-baseline".to_owned(),
        label: "No injected failures".to_owned(),
        failed_node_ids: Vec::new(),
        disabled_edge_ids: Vec::new(),
    };
    let baseline_result = evaluate_scenario(fixture, &baseline);
    let baseline_services: BTreeSet<String> = baseline_result
        .node_assessments
        .iter()
        .filter(|node| node.is_service && node.operational)
        .map(|node| node.node_id.clone())
        .collect();
    let baseline_supported = baseline_services.len();
    let mut impacts = Vec::with_capacity(fixture.nodes.len() + fixture.dependencies.len());

    for node in &fixture.nodes {
        let scenario = Scenario {
            scenario_id: format!("single-node-failure:{}", node.node_id),
            label: format!("Single-node failure: {}", node.label),
            failed_node_ids: vec![node.node_id.clone()],
            disabled_edge_ids: Vec::new(),
        };
        let result = evaluate_scenario(fixture, &scenario);
        let supported: BTreeSet<String> = result
            .node_assessments
            .iter()
            .filter(|item| item.is_service && item.operational)
            .map(|item| item.node_id.clone())
            .collect();
        let lost_service_ids: Vec<String> = baseline_services
            .difference(&supported)
            .cloned()
            .collect();
        let lost_service_count = lost_service_ids.len();
        impacts.push(SingleFailureImpact {
            failure_kind: "node".to_owned(),
            target_id: node.node_id.clone(),
            baseline_supported_service_count: baseline_supported,
            supported_service_count: supported.len(),
            lost_service_count,
            relative_service_loss_fraction: (baseline_supported > 0)
                .then_some(rounded(lost_service_count as f64 / baseline_supported as f64)),
            lost_service_ids,
        });
    }

    for edge in &fixture.dependencies {
        let scenario = Scenario {
            scenario_id: format!("single-edge-failure:{}", edge.edge_id),
            label: format!("Single-edge failure: {}", edge.edge_id),
            failed_node_ids: Vec::new(),
            disabled_edge_ids: vec![edge.edge_id.clone()],
        };
        let result = evaluate_scenario(fixture, &scenario);
        let supported: BTreeSet<String> = result
            .node_assessments
            .iter()
            .filter(|item| item.is_service && item.operational)
            .map(|item| item.node_id.clone())
            .collect();
        let lost_service_ids: Vec<String> = baseline_services
            .difference(&supported)
            .cloned()
            .collect();
        let lost_service_count = lost_service_ids.len();
        impacts.push(SingleFailureImpact {
            failure_kind: "edge".to_owned(),
            target_id: edge.edge_id.clone(),
            baseline_supported_service_count: baseline_supported,
            supported_service_count: supported.len(),
            lost_service_count,
            relative_service_loss_fraction: (baseline_supported > 0)
                .then_some(rounded(lost_service_count as f64 / baseline_supported as f64)),
            lost_service_ids,
        });
    }

    impacts.sort_by(|left, right| {
        right
            .lost_service_count
            .cmp(&left.lost_service_count)
            .then_with(|| left.failure_kind.cmp(&right.failure_kind))
            .then_with(|| left.target_id.cmp(&right.target_id))
    });
    (baseline_supported, impacts)
}

pub fn evaluate_fixture_bytes(
    fixture_bytes: &[u8],
    semantics_bytes: &[u8],
) -> Result<Report, Vec<String>> {
    let fixture: Fixture = serde_json::from_slice(fixture_bytes)
        .map_err(|error| vec![format!("fixture JSON rejected: {error}")])?;
    validate_fixture(&fixture)?;
    let semantics_sha = sha256_hex(semantics_bytes);
    if semantics_sha != fixture.dependency_semantics_sha256 {
        return Err(vec![format!(
            "dependency semantics digest mismatch: fixture says {}, exact semantics bytes hash to {}",
            fixture.dependency_semantics_sha256, semantics_sha
        )]);
    }
    let nodes: BTreeMap<&str, &Node> = fixture
        .nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node))
        .collect();
    let cross_layer_dependency_count = fixture
        .dependencies
        .iter()
        .filter(|edge| {
            nodes[edge.dependent_node_id.as_str()].layer
                != nodes[edge.provider_node_id.as_str()].layer
        })
        .count();
    let service_count = fixture.nodes.iter().filter(|node| node.is_service).count();
    let (baseline_supported_service_count, single_failure_audit) =
        evaluate_single_failure_audit(&fixture);
    let scenarios = fixture
        .scenarios
        .iter()
        .map(|scenario| evaluate_scenario(&fixture, scenario))
        .collect();
    Ok(Report {
        schema_version: REPORT_SCHEMA.to_owned(),
        suite_id: SUITE_ID.to_owned(),
        conformance_status: "PASS_SYNTHETIC_DEPENDENCY_CONFORMANCE_ONLY".to_owned(),
        data_origin: "synthetic_fixture".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        graph_id: fixture.graph_id,
        input_sha256: sha256_hex(fixture_bytes),
        dependency_semantics_sha256: semantics_sha,
        node_count: fixture.nodes.len(),
        dependency_count: fixture.dependencies.len(),
        scenario_count: fixture.scenarios.len(),
        service_count,
        cross_layer_dependency_count,
        baseline_supported_service_count,
        single_failure_audit,
        scenarios,
    })
}

fn rounded(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

pub fn default_fixture_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/landscape/dependency-graph-v0/fixture.json")
}

pub fn default_semantics_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/LANDSCAPE_DEPENDENCY_SEMANTICS_V0.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn fixture_bytes() -> Vec<u8> {
        fs::read(default_fixture_path()).expect("frozen dependency graph fixture")
    }

    fn semantics_bytes() -> Vec<u8> {
        fs::read(default_semantics_path()).expect("frozen semantics file")
    }

    #[test]
    fn frozen_fixture_matches_known_service_availability_and_causes() {
        let report = evaluate_fixture_bytes(&fixture_bytes(), &semantics_bytes()).expect("report");
        assert_eq!(report.conformance_status, "PASS_SYNTHETIC_DEPENDENCY_CONFORMANCE_ONLY");
        assert_eq!(report.node_count, 11);
        assert_eq!(report.dependency_count, 11);
        assert_eq!(report.service_count, 3);
        assert_eq!(report.scenario_count, 6);
        let expected = [
            ("baseline", 3, 0.0),
            ("drought", 2, 1.0),
            ("maintenance_shock", 1, 2.0),
            ("access_disruption", 2, 1.0),
            ("irrigation_link_outage", 2, 1.0),
            ("compound_failure", 0, 1.0),
        ];
        for (scenario_id, supported, cascade) in expected {
            let scenario = report
                .scenarios
                .iter()
                .find(|item| item.scenario_id == scenario_id)
                .expect("scenario");
            assert_eq!(scenario.supported_service_count, supported, "{scenario_id}");
            if scenario_id == "baseline" {
                assert_eq!(scenario.cascade_amplification, None);
            } else {
                assert_eq!(scenario.cascade_amplification, Some(cascade), "{scenario_id}");
            }
        }
        let drought = report.scenarios.iter().find(|item| item.scenario_id == "drought").unwrap();
        let heat = drought.node_assessments.iter().find(|node| node.node_id == "heat_refuge_service").unwrap();
        assert!(heat.operational, "built shade is an alternative provider for the mandatory shade group");
        let tree = drought.node_assessments.iter().find(|node| node.node_id == "tree_ecology_service").unwrap();
        assert!(!tree.operational);
        assert!(tree.unsatisfied_groups.iter().any(|group| group.requirement_group_id == "canopy"));
        let canopy = drought.node_assessments.iter().find(|node| node.node_id == "canopy").unwrap();
        assert!(canopy.unsatisfied_groups.iter().any(|group| group.requirement_group_id == "irrigation"));
    }

    #[test]
    fn scenarios_must_reference_known_nodes_and_edges() {
        let mut fixture: Fixture = serde_json::from_slice(&fixture_bytes()).unwrap();
        fixture.scenarios[0].failed_node_ids.push("not-a-node".to_owned());
        fixture.scenarios[0].disabled_edge_ids.push("not-an-edge".to_owned());
        let errors = validate_fixture(&fixture).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("unknown failed node")));
        assert!(errors.iter().any(|error| error.contains("unknown disabled edge")));
    }

    #[test]
    fn graph_cycles_are_rejected_instead_of_self_supporting() {
        let mut fixture: Fixture = serde_json::from_slice(&fixture_bytes()).unwrap();
        fixture.dependencies.push(Dependency {
            edge_id: "cycle-edge".to_owned(),
            dependent_node_id: "water_source".to_owned(),
            provider_node_id: "irrigation_network".to_owned(),
            requirement_group_id: "undeclared".to_owned(),
        });
        let errors = validate_fixture(&fixture).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("cycle")));
        assert!(errors.iter().any(|error| error.contains("not declared")));
    }

    #[test]
    fn missing_provider_edge_fails_closed() {
        let mut fixture: Fixture = serde_json::from_slice(&fixture_bytes()).unwrap();
        fixture.dependencies.retain(|edge| {
            edge.edge_id != "edge-heat-built-shade" && edge.edge_id != "edge-heat-canopy"
        });
        let errors = validate_fixture(&fixture).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("required group shade has no provider edges")));
    }

    #[test]
    fn scenarios_cannot_be_relabelled_as_observed() {
        let mut fixture: Fixture = serde_json::from_slice(&fixture_bytes()).unwrap();
        fixture.data_origin = "field_observation".to_owned();
        assert!(validate_fixture(&fixture).is_err());
    }

    #[test]
    fn semantics_digest_is_bound_to_exact_bytes() {
        let bytes = fixture_bytes();
        let mut semantics = semantics_bytes();
        semantics.push(b' ');
        assert!(evaluate_fixture_bytes(&bytes, &semantics)
            .unwrap_err()
            .iter()
            .any(|error| error.contains("dependency semantics digest mismatch")));
    }

    #[test]
    fn adding_declared_failures_never_increases_service_availability() {
        let fixture: Fixture = serde_json::from_slice(&fixture_bytes()).unwrap();
        for scenario in &fixture.scenarios {
            let original = evaluate_scenario(&fixture, scenario);
            let mut node_failure = scenario.clone();
            if !node_failure.failed_node_ids.iter().any(|id| id == "soil_infiltration") {
                node_failure.failed_node_ids.push("soil_infiltration".to_owned());
            }
            let after_node_failure = evaluate_scenario(&fixture, &node_failure);
            assert!(
                after_node_failure.supported_service_count <= original.supported_service_count,
                "adding a node failure improved service support in {}",
                scenario.scenario_id
            );

            let mut edge_failure = scenario.clone();
            if !edge_failure.disabled_edge_ids.iter().any(|id| id == "edge-heat-built-shade") {
                edge_failure.disabled_edge_ids.push("edge-heat-built-shade".to_owned());
            }
            let after_edge_failure = evaluate_scenario(&fixture, &edge_failure);
            assert!(
                after_edge_failure.supported_service_count <= original.supported_service_count,
                "disabling an edge improved service support in {}",
                scenario.scenario_id
            );
        }
    }

    #[test]
    fn every_unsupported_node_exposes_a_direct_blocker() {
        let fixture: Fixture = serde_json::from_slice(&fixture_bytes()).unwrap();
        for scenario in &fixture.scenarios {
            let report = evaluate_scenario(&fixture, scenario);
            for node in report.node_assessments.iter().filter(|node| !node.operational) {
                if node.self_failed {
                    continue;
                }
                assert!(
                    !node.unsatisfied_groups.is_empty(),
                    "unsupported node {} has no declared blocker in {}",
                    node.node_id,
                    scenario.scenario_id
                );
                for group in &node.unsatisfied_groups {
                    assert!(
                        group.provider_alternatives.iter().all(|provider| provider.status != "operational"),
                        "unsatisfied group {} has an operational provider in {}",
                        group.requirement_group_id,
                        node.node_id
                    );
                }
            }
        }
    }

    #[test]
    fn declared_node_limit_has_an_exact_boundary() {
        let mut fixture: Fixture = serde_json::from_slice(&fixture_bytes()).unwrap();
        for index in 0..89 {
            fixture.nodes.push(Node {
                node_id: format!("boundary-node-{index:03}"),
                label: format!("Boundary node {index}"),
                layer: "technical".to_owned(),
                is_service: false,
                required_dependency_groups: Vec::new(),
            });
        }
        assert_eq!(fixture.nodes.len(), 100);
        assert!(validate_fixture(&fixture).is_ok());
        fixture.nodes.push(Node {
            node_id: "boundary-node-over-limit".to_owned(),
            label: "Over-limit node".to_owned(),
            layer: "technical".to_owned(),
            is_service: false,
            required_dependency_groups: Vec::new(),
        });
        assert!(
            validate_fixture(&fixture)
                .unwrap_err()
                .iter()
                .any(|error| error.contains("nodes must contain 1..=100 entries"))
        );
    }

    #[test]
    fn single_failure_audit_ranks_declared_critical_dependencies() {
        let report = evaluate_fixture_bytes(&fixture_bytes(), &semantics_bytes()).unwrap();
        assert_eq!(report.baseline_supported_service_count, 3);
        assert_eq!(report.single_failure_audit.len(), 22);
        let maintenance = report
            .single_failure_audit
            .iter()
            .find(|impact| impact.failure_kind == "node" && impact.target_id == "maintenance_crew")
            .expect("maintenance node included in single-failure audit");
        assert_eq!(maintenance.lost_service_count, 2);
        assert_eq!(
            maintenance.lost_service_ids,
            vec!["stormwater_service".to_owned(), "tree_ecology_service".to_owned()]
        );
        assert_eq!(maintenance.relative_service_loss_fraction, Some(0.666667));
        assert!(
            report
                .single_failure_audit
                .windows(2)
                .all(|pair| pair[0].lost_service_count >= pair[1].lost_service_count),
            "single-failure audit must be ordered from largest to smallest service loss"
        );
    }

    #[test]
    fn identical_inputs_emit_identical_report_bytes() {
        let bytes = fixture_bytes();
        let semantics = semantics_bytes();
        let first = evaluate_fixture_bytes(&bytes, &semantics).unwrap();
        let second = evaluate_fixture_bytes(&bytes, &semantics).unwrap();
        assert_eq!(
            serde_json::to_vec(&first).unwrap(),
            serde_json::to_vec(&second).unwrap()
        );
        assert_eq!(first.input_sha256, sha256_hex(&bytes));
    }
}
