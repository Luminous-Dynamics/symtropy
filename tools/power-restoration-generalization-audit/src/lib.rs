use power_restoration_benchmark_case::{BenchmarkCase, canonical_case_sha256, validate_case};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum DatasetSplit {
    Train,
    Validation,
    Test,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SplitAssignment {
    pub case_id: String,
    pub split: DatasetSplit,
    pub topology_family_id: String,
    pub expected_case_sha256: String,
    pub expected_topology_fingerprint: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SplitManifest {
    pub schema_version: String,
    pub manifest_id: String,
    pub policy_version: String,
    pub split_seed: u64,
    pub assignments: Vec<SplitAssignment>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SplitAuditInput {
    pub manifest: SplitManifest,
    pub cases: Vec<BenchmarkCase>,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CaseSplitFinding {
    pub case_id: String,
    pub split: DatasetSplit,
    pub topology_family_id: String,
    pub case_sha256: String,
    pub topology_fingerprint: String,
    pub result: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SplitLeakageFinding {
    pub leakage_kind: String,
    pub identity: String,
    pub involved_splits: Vec<DatasetSplit>,
    pub case_ids: Vec<String>,
    pub explanation: String,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SplitAuditReport {
    pub manifest_id: String,
    pub manifest_sha256: String,
    pub case_count: usize,
    pub train_case_count: usize,
    pub validation_case_count: usize,
    pub test_case_count: usize,
    pub topology_family_count: usize,
    pub unique_topology_fingerprint_count: usize,
    pub leakage_finding_count: usize,
    pub case_findings: Vec<CaseSplitFinding>,
    pub leakage_findings: Vec<SplitLeakageFinding>,
    pub result_status: String,
    pub claim_boundary: String,
}

#[derive(Clone, Debug)]
struct GraphEdge {
    left: usize,
    right: usize,
    label: String,
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
}

fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn hash_text(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn hash_parts(parts: impl IntoIterator<Item = String>) -> String {
    let canonical = parts.into_iter().collect::<Vec<_>>().join("\n");
    hash_text(&canonical)
}

/// Weisfeiler-Lehman color refinement fingerprint for an attributed, undirected
/// multigraph. Node identifiers are intentionally excluded; source/load/fault
/// placement, branch/switch type and edge state contribute to initial labels.
///
/// This is a leakage-screening fingerprint, not a general graph-isomorphism
/// proof. The manifest must also group semantically related feeder variants
/// under a reviewed topologyFamilyId.
fn graph_fingerprint(
    node_labels: Vec<String>,
    edges: &[GraphEdge],
) -> Result<String, String> {
    if node_labels.is_empty() {
        return Err("topology graph must contain at least one node".to_string());
    }
    if edges.iter().any(|edge| {
        edge.left >= node_labels.len()
            || edge.right >= node_labels.len()
            || edge.left == edge.right
    }) {
        return Err("topology graph has an invalid edge endpoint".to_string());
    }

    let mut colors: Vec<String> = node_labels.iter().map(|label| hash_text(label)).collect();
    for _ in 0..node_labels.len().min(32) {
        let mut next = Vec::with_capacity(colors.len());
        for node_index in 0..colors.len() {
            let mut neighborhood = Vec::new();
            for edge in edges {
                if edge.left == node_index {
                    neighborhood.push(format!("{}:{}", edge.label, colors[edge.right]));
                } else if edge.right == node_index {
                    neighborhood.push(format!("{}:{}", edge.label, colors[edge.left]));
                }
            }
            neighborhood.sort();
            let mut parts = vec![colors[node_index].clone()];
            parts.extend(neighborhood);
            next.push(hash_parts(parts));
        }
        if next == colors {
            break;
        }
        colors = next;
    }

    let mut node_colors = colors;
    node_colors.sort();
    let mut edge_colors = Vec::new();
    for edge in edges {
        let (a, b) = if colors[edge.left] <= colors[edge.right] {
            (&colors[edge.left], &colors[edge.right])
        } else {
            (&colors[edge.right], &colors[edge.left])
        };
        edge_colors.push(format!("{}:{}:{}", edge.label, a, b));
    }
    edge_colors.sort();

    let mut parts = vec![
        format!("node-count={}", node_colors.len()),
        format!("edge-count={}", edge_colors.len()),
    ];
    parts.extend(node_colors.into_iter().map(|color| format!("node:{color}")));
    parts.extend(edge_colors.into_iter().map(|color| format!("edge:{color}")));
    Ok(hash_parts(parts))
}

fn power_graph_fingerprint(case: &BenchmarkCase) -> Result<String, String> {
    let mut node_labels = Vec::new();
    for node in &case.network.nodes {
        let load_count = case.network.loads.iter().filter(|load| load.node_id == node.node_id).count();
        let source_count = case.network.grid_sources.iter().filter(|source| source.node_id == node.node_id).count();
        let fault_incidence = case.faults.iter().filter(|fault| {
            case.network.branches.iter().any(|branch| {
                branch.branch_id == fault.power_branch_id
                    && (branch.from_node_id == node.node_id || branch.to_node_id == node.node_id)
            })
        }).count();
        node_labels.push(format!(
            "power-node;loads={load_count};sources={source_count};fault-incidence={fault_incidence}"
        ));
    }

    let node_index: BTreeMap<&str, usize> = case.network.nodes.iter()
        .enumerate()
        .map(|(index, node)| (node.node_id.as_str(), index))
        .collect();

    let mut edges = Vec::new();
    for branch in &case.network.branches {
        let left = *node_index.get(branch.from_node_id.as_str())
            .ok_or_else(|| format!("branch {} has unknown from node", branch.branch_id))?;
        let right = *node_index.get(branch.to_node_id.as_str())
            .ok_or_else(|| format!("branch {} has unknown to node", branch.branch_id))?;
        let fault_count = case.faults.iter()
            .filter(|fault| fault.power_branch_id == branch.branch_id)
            .count();
        edges.push(GraphEdge {
            left,
            right,
            label: format!(
                "branch;normally-energized={};electrical-data={};fault-count={fault_count}",
                branch.normally_energized,
                branch.electrical.is_some()
            ),
        });
    }
    for switch in &case.network.switches {
        let left = *node_index.get(switch.from_node_id.as_str())
            .ok_or_else(|| format!("switch {} has unknown from node", switch.switch_id))?;
        let right = *node_index.get(switch.to_node_id.as_str())
            .ok_or_else(|| format!("switch {} has unknown to node", switch.switch_id))?;
        edges.push(GraphEdge {
            left,
            right,
            label: format!(
                "switch;initially-closed={};normally-open-tie={}",
                switch.initially_closed, switch.normally_open_tie
            ),
        });
    }

    graph_fingerprint(node_labels, &edges)
}

fn road_graph_fingerprint(case: &BenchmarkCase) -> Result<String, String> {
    let mut node_labels = Vec::new();
    for node in &case.road_network.nodes {
        let crew_count = case.repair_crews.iter()
            .filter(|crew| crew.starting_road_node_id == node.node_id)
            .count();
        let mobile_source_count = case.mobile_power_sources.iter()
            .filter(|source| source.starting_road_node_id == node.node_id)
            .count();
        node_labels.push(format!(
            "road-node;power-interface={};starting-crews={crew_count};starting-mobile-sources={mobile_source_count}",
            !node.power_node_id.is_empty()
        ));
    }

    let node_index: BTreeMap<&str, usize> = case.road_network.nodes.iter()
        .enumerate()
        .map(|(index, node)| (node.node_id.as_str(), index))
        .collect();
    let mut edges = Vec::new();
    for edge in &case.road_network.edges {
        let left = *node_index.get(edge.from_node_id.as_str())
            .ok_or_else(|| format!("road edge {} has unknown from node", edge.edge_id))?;
        let right = *node_index.get(edge.to_node_id.as_str())
            .ok_or_else(|| format!("road edge {} has unknown to node", edge.edge_id))?;
        edges.push(GraphEdge {
            left,
            right,
            label: format!("road-edge;closed-at-start={}", edge.closed_at_start),
        });
    }
    graph_fingerprint(node_labels, &edges)
}

pub fn topology_fingerprint(case: &BenchmarkCase) -> Result<String, String> {
    let power = power_graph_fingerprint(case)?;
    let road = road_graph_fingerprint(case)?;
    let equipment_profile = format!(
        "electrical={:?};faults={};crews={};mobile-sources={};switches={};electrical-settings={}",
        case.model_profile.electrical_data_profile,
        case.faults.len(),
        case.repair_crews.len(),
        case.mobile_power_sources.len(),
        case.network.switches.len(),
        case.network.electrical_settings.is_some()
    );
    Ok(hash_parts([
        format!("power:{power}"),
        format!("road:{road}"),
        equipment_profile,
    ]))
}

pub fn audit_splits(input: &SplitAuditInput) -> Result<SplitAuditReport, Vec<String>> {
    let mut errors = Vec::new();
    let manifest = &input.manifest;
    if manifest.schema_version != "0.1.0"
        || !valid_id(&manifest.manifest_id)
        || manifest.policy_version != "topology-family-holdout-v0.1"
    {
        errors.push("split manifest version, ID or policy version is invalid".to_string());
    }
    if input.cases.is_empty() || manifest.assignments.is_empty() {
        errors.push("split audit requires at least one case and one assignment".to_string());
    }

    let mut cases_by_id = BTreeMap::new();
    for case in &input.cases {
        if cases_by_id.insert(case.case_id.as_str(), case).is_some() {
            errors.push(format!("duplicate benchmark case ID {}", case.case_id));
        }
        for error in validate_case(case) {
            errors.push(format!("case {}: {error}", case.case_id));
        }
    }

    let mut assignments_by_id = BTreeMap::new();
    let mut family_splits = BTreeMap::<&str, BTreeSet<DatasetSplit>>::new();
    let mut fingerprint_splits = BTreeMap::<String, BTreeSet<DatasetSplit>>::new();
    let mut fingerprint_cases = BTreeMap::<String, Vec<String>>::new();
    let mut family_cases = BTreeMap::<String, Vec<String>>::new();
    let mut case_findings = Vec::new();
    let mut train = 0;
    let mut validation = 0;
    let mut test = 0;

    for assignment in &manifest.assignments {
        if !valid_id(&assignment.case_id)
            || !valid_id(&assignment.topology_family_id)
            || !is_lower_hex(&assignment.expected_case_sha256, 64)
            || !is_lower_hex(&assignment.expected_topology_fingerprint, 64)
        {
            errors.push(format!("case assignment {} has invalid IDs or hashes", assignment.case_id));
            continue;
        }
        if assignments_by_id.insert(assignment.case_id.as_str(), assignment).is_some() {
            errors.push(format!("case {} has duplicate split assignments", assignment.case_id));
        }

        match assignment.split {
            DatasetSplit::Train => train += 1,
            DatasetSplit::Validation => validation += 1,
            DatasetSplit::Test => test += 1,
        }

        let Some(case) = cases_by_id.get(assignment.case_id.as_str()) else {
            errors.push(format!("assignment references missing case {}", assignment.case_id));
            continue;
        };
        let case_sha = match canonical_case_sha256(case) {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!("case {} digest failed: {error}", case.case_id));
                continue;
            }
        };
        if case_sha != assignment.expected_case_sha256 {
            errors.push(format!("case {} digest differs from frozen split assignment", case.case_id));
        }
        let fingerprint = match topology_fingerprint(case) {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!("case {} topology fingerprint failed: {error}", case.case_id));
                continue;
            }
        };
        if fingerprint != assignment.expected_topology_fingerprint {
            errors.push(format!("case {} topology fingerprint differs from split manifest", case.case_id));
        }

        family_splits.entry(assignment.topology_family_id.as_str())
            .or_default()
            .insert(assignment.split);
        fingerprint_splits.entry(fingerprint.clone())
            .or_default()
            .insert(assignment.split);
        fingerprint_cases.entry(fingerprint.clone())
            .or_default()
            .push(case.case_id.clone());
        family_cases.entry(assignment.topology_family_id.clone())
            .or_default()
            .push(case.case_id.clone());

        case_findings.push(CaseSplitFinding {
            case_id: case.case_id.clone(),
            split: assignment.split,
            topology_family_id: assignment.topology_family_id.clone(),
            case_sha256: case_sha,
            topology_fingerprint: fingerprint,
            result: "assigned_and_fingerprinted".to_string(),
        });
    }

    if train == 0 || validation == 0 || test == 0 {
        errors.push("train, validation and test splits must all be non-empty".to_string());
    }
    if assignments_by_id.len() != cases_by_id.len() {
        errors.push(format!(
            "split assignment coverage mismatch: {} cases but {} assignments",
            cases_by_id.len(),
            assignments_by_id.len()
        ));
    }

    let mut leakage_findings = Vec::new();
    for (family_id, splits) in &family_splits {
        if splits.len() > 1 {
            let case_ids = family_cases.get(*family_id).cloned().unwrap_or_default();
            leakage_findings.push(SplitLeakageFinding {
                leakage_kind: "topology_family_cross_split".to_string(),
                identity: (*family_id).to_string(),
                involved_splits: splits.iter().copied().collect(),
                case_ids,
                explanation: "A declared topology family appears in more than one data split; all scenarios from a feeder family must be held out together".to_string(),
            });
        }
    }
    for (fingerprint, splits) in &fingerprint_splits {
        if splits.len() > 1 {
            let case_ids = fingerprint_cases.get(fingerprint).cloned().unwrap_or_default();
            leakage_findings.push(SplitLeakageFinding {
                leakage_kind: "structural_topology_fingerprint_cross_split".to_string(),
                identity: fingerprint.clone(),
                involved_splits: splits.iter().copied().collect(),
                case_ids,
                explanation: "The same identifier-invariant power/road graph fingerprint occurs in multiple splits; scenario variants of one structural feeder must not leak across train/validation/test".to_string(),
            });
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let manifest_sha256 = serde_json::to_value(manifest)
        .and_then(|value| serde_json::to_vec(&value))
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|error| vec![format!("split manifest canonicalization failed: {error}")])?;

    Ok(SplitAuditReport {
        manifest_id: manifest.manifest_id.clone(),
        manifest_sha256,
        case_count: input.cases.len(),
        train_case_count: train,
        validation_case_count: validation,
        test_case_count: test,
        topology_family_count: family_splits.len(),
        unique_topology_fingerprint_count: fingerprint_splits.len(),
        leakage_finding_count: leakage_findings.len(),
        case_findings,
        leakage_findings,
        result_status: if leakage_findings.is_empty() {
            "split_assignment_and_topology_fingerprints_valid".to_string()
        } else {
            "topology_leakage_detected".to_string()
        },
        claim_boundary: "Split identity, case digest and identifier-invariant topology fingerprint screening only. The Weisfeiler-Lehman fingerprint is not a complete graph-isomorphism proof; declared topology-family IDs require reviewed provenance. This audit does not prove absence of all semantic, temporal, data, feature or model leakage.".to_string(),
    })
}

pub fn parse_and_audit(input_json: &str) -> Result<SplitAuditReport, Vec<String>> {
    let input: SplitAuditInput = serde_json::from_str(input_json)
        .map_err(|error| vec![format!("split audit input JSON decode error: {error}")])?;
    audit_splits(&input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_case() -> BenchmarkCase {
        serde_json::from_str(include_str!(
            "../../power-restoration-benchmark-case/fixtures/synthetic-restoration-case-v0.1.json"
        )).unwrap()
    }

    fn split_input() -> SplitAuditInput {
        let base = base_case();
        let mut train_case = base.clone();
        train_case.case_id = "train-scenario-a".to_string();
        train_case.network.loads[0].active_power_kw += 10.0;

        let mut validation_case = base.clone();
        validation_case.case_id = "validation-scenario-b".to_string();
        validation_case.network.loads[0].active_power_kw += 20.0;
        validation_case.network.switches[0].to_node_id = "N4".to_string();

        let mut test_case = base.clone();
        test_case.case_id = "test-scenario-c".to_string();
        test_case.network.loads[0].active_power_kw += 30.0;
        test_case.network.switches[1].to_node_id = "N6".to_string();

        let cases = vec![train_case, validation_case, test_case];
        let assignments = cases.iter().enumerate().map(|(index, case)| {
            let split = match index {
                0 => DatasetSplit::Train,
                1 => DatasetSplit::Validation,
                _ => DatasetSplit::Test,
            };
            SplitAssignment {
                case_id: case.case_id.clone(),
                split,
                topology_family_id: format!("family-{index}"),
                expected_case_sha256: canonical_case_sha256(case).unwrap(),
                expected_topology_fingerprint: topology_fingerprint(case).unwrap(),
            }
        }).collect();

        SplitAuditInput {
            manifest: SplitManifest {
                schema_version: "0.1.0".to_string(),
                manifest_id: "synthetic-topology-split-audit-001".to_string(),
                policy_version: "topology-family-holdout-v0.1".to_string(),
                split_seed: 20261120,
                assignments,
            },
            cases,
        }
    }

    #[test]
    fn split_manifest_with_disjoint_topology_families_passes() {
        let report = audit_splits(&split_input()).unwrap();
        assert_eq!(report.case_count, 3);
        assert_eq!(report.train_case_count, 1);
        assert_eq!(report.validation_case_count, 1);
        assert_eq!(report.test_case_count, 1);
        assert_eq!(report.leakage_finding_count, 0);
    }

    #[test]
    fn scenario_variants_of_same_family_cannot_cross_splits() {
        let mut input = split_input();
        input.manifest.assignments[1].topology_family_id =
            input.manifest.assignments[0].topology_family_id.clone();
        let report = audit_splits(&input).unwrap();
        assert!(report.leakage_findings.iter().any(|finding| {
            finding.leakage_kind == "topology_family_cross_split"
        }));
        assert_eq!(report.result_status, "topology_leakage_detected");
    }

    #[test]
    fn same_structural_topology_cannot_cross_splits_even_when_family_ids_differ() {
        let mut input = split_input();
        input.cases[1].network = input.cases[0].network.clone();
        input.cases[2].network = input.cases[0].network.clone();
        for (index, assignment) in input.manifest.assignments.iter_mut().enumerate() {
            assignment.topology_family_id = format!("declared-family-{index}");
            assignment.expected_case_sha256 = canonical_case_sha256(&input.cases[index]).unwrap();
            assignment.expected_topology_fingerprint = topology_fingerprint(&input.cases[index]).unwrap();
        }
        let report = audit_splits(&input).unwrap();
        assert_eq!(report.leakage_finding_count, 1);
        assert_eq!(report.leakage_findings[0].leakage_kind, "structural_topology_fingerprint_cross_split");
    }

    #[test]
    fn changing_topology_fingerprint_is_detected() {
        let mut input = split_input();
        input.manifest.assignments[2].expected_topology_fingerprint = "0".repeat(64);
        assert!(audit_splits(&input).is_err());
    }

    #[test]
    fn case_content_mutation_is_detected() {
        let mut input = split_input();
        input.cases[0].network.loads[0].active_power_kw += 1.0;
        assert!(audit_splits(&input).is_err());
    }

    #[test]
    fn renaming_nodes_does_not_change_structural_fingerprint() {
        let original = base_case();
        let first = topology_fingerprint(&original).unwrap();
        let mut renamed = original.clone();
        for (index, node) in renamed.network.nodes.iter_mut().enumerate() {
            node.node_id = format!("renamed-node-{index}");
        }
        let mapping: BTreeMap<String, String> = original.network.nodes.iter()
            .zip(renamed.network.nodes.iter())
            .map(|(old, new)| (old.node_id.clone(), new.node_id.clone()))
            .collect();
        for (index, branch) in renamed.network.branches.iter_mut().enumerate() {
            let old = &original.network.branches[index];
            branch.from_node_id = mapping[&old.from_node_id].clone();
            branch.to_node_id = mapping[&old.to_node_id].clone();
        }
        for (index, switch) in renamed.network.switches.iter_mut().enumerate() {
            let old = &original.network.switches[index];
            switch.from_node_id = mapping[&old.from_node_id].clone();
            switch.to_node_id = mapping[&old.to_node_id].clone();
        }
        for (index, load) in renamed.network.loads.iter_mut().enumerate() {
            load.node_id = mapping[&original.network.loads[index].node_id].clone();
        }
        for (index, source) in renamed.network.grid_sources.iter_mut().enumerate() {
            source.node_id = mapping[&original.network.grid_sources[index].node_id].clone();
        }
        for (index, road_node) in renamed.road_network.nodes.iter_mut().enumerate() {
            road_node.power_node_id = mapping[&original.road_network.nodes[index].power_node_id].clone();
        }
        assert_eq!(first, topology_fingerprint(&renamed).unwrap());
    }
}
