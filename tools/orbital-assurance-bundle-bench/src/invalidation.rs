//! Dependency-scoped invalidation for versioned assurance claims.
//!
//! This adapter consumes immutable artifact identities and exact dependency pins.
//! It computes which descendants must be requalified after a source or scope change.
//! It never grants operational authority or validates physical truth.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const SCHEMA: &str = "luminous.orbital-assurance-invalidation.corpus.v0.1";
const CLAIM_CEILING: &str = "synthetic-dependency-invalidation-only";
const MAX_CASES: usize = 128;
const MAX_NODES: usize = 4096;
const MAX_DEPENDENCIES: usize = 16384;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DependencyPin {
    pub node_id: String,
    pub expected_content_sha256: String,
    pub expected_scope_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EvidenceNode {
    pub node_id: String,
    pub node_type: String,
    /// Digest of the exact artifact used when this node was produced.
    pub content_sha256: String,
    /// Digest of the artifact currently observed at the source.
    pub current_content_sha256: String,
    /// Scope identity bound when this node was produced.
    pub scope_sha256: String,
    /// Current scope identity observed at the source.
    pub current_scope_sha256: String,
    /// Exact versions of upstream nodes consumed by this node.
    pub dependencies: Vec<DependencyPin>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InvalidationCase {
    pub case_id: String,
    pub required_root_node_ids: Vec<String>,
    pub nodes: Vec<EvidenceNode>,
    pub expected_disposition: String,
    pub expected_invalidated_node_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<InvalidationCase>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct NodeResult {
    pub node_id: String,
    pub status: String,
    pub invalidated_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CaseResult {
    pub case_id: String,
    pub disposition: String,
    pub required_root_node_ids: Vec<String>,
    pub invalidated_required_root_node_ids: Vec<String>,
    pub invalidated_node_ids: Vec<String>,
    pub current_node_count: usize,
    pub invalidated_node_count: usize,
    pub nodes: Vec<NodeResult>,
    pub authority_granted: bool,
    pub expected_result_matched: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub case_count: usize,
    pub current_cases: usize,
    pub requalification_required_cases: usize,
    pub invalid_graph_cases: usize,
    pub source_changed_nodes: usize,
    pub scope_changed_nodes: usize,
    pub dependency_version_mismatch_nodes: usize,
    pub upstream_invalidated_nodes: usize,
    pub invalidated_required_roots: usize,
    pub expected_result_mismatches: usize,
    pub authority_grants: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Evaluation {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub summary: Summary,
    pub cases: Vec<CaseResult>,
    pub authority_granted: bool,
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.is_ascii()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn invalid_graph(case: &InvalidationCase, detail: &str) -> CaseResult {
    let mut node_ids = case
        .nodes
        .iter()
        .map(|node| node.node_id.clone())
        .collect::<Vec<_>>();
    node_ids.sort();
    node_ids.dedup();
    let invalidated_required_root_node_ids = case
        .required_root_node_ids
        .iter()
        .filter(|id| node_ids.contains(id))
        .cloned()
        .collect::<Vec<_>>();
    CaseResult {
        case_id: case.case_id.clone(),
        disposition: "INVALID_GRAPH".to_owned(),
        required_root_node_ids: case.required_root_node_ids.clone(),
        invalidated_required_root_node_ids,
        invalidated_node_ids: node_ids.clone(),
        current_node_count: 0,
        invalidated_node_count: node_ids.len(),
        nodes: node_ids
            .iter()
            .map(|node_id| NodeResult {
                node_id: node_id.clone(),
                status: "INVALID_GRAPH".to_owned(),
                invalidated_by: vec![detail.to_owned()],
            })
            .collect(),
        authority_granted: false,
        expected_result_matched: case.expected_disposition == "INVALID_GRAPH"
            && case
                .expected_invalidated_node_ids
                .iter()
                .collect::<BTreeSet<_>>()
                == node_ids.iter().collect::<BTreeSet<_>>(),
    }
}

fn evaluate_case(case: &InvalidationCase) -> CaseResult {
    if !valid_id(&case.case_id)
        || case.nodes.is_empty()
        || case.nodes.len() > MAX_NODES
        || case.required_root_node_ids.is_empty()
        || case.required_root_node_ids.len() > MAX_NODES
        || case.expected_invalidated_node_ids.len() > MAX_NODES
        || !matches!(
            case.expected_disposition.as_str(),
            "CURRENT" | "REQUALIFICATION_REQUIRED" | "INVALID_GRAPH"
        )
    {
        return invalid_graph(case, "invalid case identity or bounds");
    }

    let mut node_index = BTreeMap::new();
    let mut dependency_count = 0usize;
    for (index, node) in case.nodes.iter().enumerate() {
        if !valid_id(&node.node_id)
            || !valid_id(&node.node_type)
            || !valid_sha256(&node.content_sha256)
            || !valid_sha256(&node.current_content_sha256)
            || !valid_sha256(&node.scope_sha256)
            || !valid_sha256(&node.current_scope_sha256)
            || node.dependencies.len() > MAX_NODES
            || node_index.insert(node.node_id.as_str(), index).is_some()
        {
            return invalid_graph(case, "duplicate node or malformed node identity");
        }
        dependency_count += node.dependencies.len();
        if dependency_count > MAX_DEPENDENCIES {
            return invalid_graph(case, "dependency inventory exceeds bound");
        }
    }

    let mut roots = BTreeSet::new();
    for root_id in &case.required_root_node_ids {
        if !valid_id(root_id)
            || !roots.insert(root_id.as_str())
            || !node_index.contains_key(root_id.as_str())
        {
            return invalid_graph(case, "required root is missing, empty or duplicated");
        }
    }

    let mut children: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut indegree: BTreeMap<&str, usize> = case
        .nodes
        .iter()
        .map(|node| (node.node_id.as_str(), node.dependencies.len()))
        .collect();

    for node in &case.nodes {
        let mut dependency_ids = BTreeSet::new();
        for dependency in &node.dependencies {
            if !valid_id(&dependency.node_id)
                || !valid_sha256(&dependency.expected_content_sha256)
                || !valid_sha256(&dependency.expected_scope_sha256)
                || !dependency_ids.insert(dependency.node_id.as_str())
                || !node_index.contains_key(dependency.node_id.as_str())
                || dependency.node_id == node.node_id
            {
                return invalid_graph(case, "missing, duplicated or malformed dependency pin");
            }
            children
                .entry(dependency.node_id.as_str())
                .or_default()
                .push(node.node_id.as_str());
        }
    }

    // Kahn's algorithm: all nodes must belong to a DAG, including disconnected evidence.
    let mut ready = indegree
        .iter()
        .filter_map(|(node_id, count)| (*count == 0).then_some(*node_id))
        .collect::<BTreeSet<_>>();
    let mut visited = 0usize;
    while let Some(node_id) = ready.pop_first() {
        visited += 1;
        for child in children.get(node_id).into_iter().flatten() {
            let count = indegree
                .get_mut(child)
                .expect("child was added from a validated node");
            *count -= 1;
            if *count == 0 {
                ready.insert(child);
            }
        }
    }
    if visited != case.nodes.len() {
        return invalid_graph(case, "dependency graph contains a cycle");
    }

    let mut statuses: BTreeMap<&str, String> = BTreeMap::new();
    let mut invalidated_by: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    let mut queue = VecDeque::new();

    // First identify version changes at source nodes.
    for node in &case.nodes {
        let status = if node.content_sha256 != node.current_content_sha256 {
            Some("SOURCE_CHANGED")
        } else if node.scope_sha256 != node.current_scope_sha256 {
            Some("SCOPE_CHANGED")
        } else {
            None
        };
        if let Some(status) = status {
            statuses.insert(node.node_id.as_str(), status.to_owned());
            invalidated_by
                .entry(node.node_id.as_str())
                .or_default()
                .insert(node.node_id.clone());
            queue.push_back(node.node_id.as_str());
        }
    }

    // Then identify consumers whose exact dependency pins no longer match current parents.
    for node in &case.nodes {
        for dependency in &node.dependencies {
            let parent = &case.nodes[*node_index
                .get(dependency.node_id.as_str())
                .expect("dependency existence was checked")];
            let mismatch = dependency.expected_content_sha256 != parent.current_content_sha256
                || dependency.expected_scope_sha256 != parent.current_scope_sha256;
            if mismatch {
                statuses
                    .entry(node.node_id.as_str())
                    .or_insert_with(|| "DEPENDENCY_VERSION_MISMATCH".to_owned());
                invalidated_by
                    .entry(node.node_id.as_str())
                    .or_default()
                    .insert(parent.node_id.clone());
                queue.push_back(node.node_id.as_str());
            }
        }
    }

    // Propagate invalidation transitively through the reverse dependency graph.
    let mut propagated = BTreeSet::new();
    while let Some(parent_id) = queue.pop_front() {
        if !propagated.insert(parent_id) {
            continue;
        }
        for child_id in children.get(parent_id).into_iter().flatten() {
            invalidated_by
                .entry(child_id)
                .or_default()
                .insert(parent_id.to_owned());
            if !statuses.contains_key(child_id) {
                statuses.insert(child_id, "UPSTREAM_INVALIDATED".to_owned());
                queue.push_back(child_id);
            } else if !propagated.contains(child_id) {
                queue.push_back(child_id);
            }
        }
    }

    let mut node_results = case
        .nodes
        .iter()
        .map(|node| {
            let status = statuses
                .get(node.node_id.as_str())
                .cloned()
                .unwrap_or_else(|| "CURRENT".to_owned());
            let causes = invalidated_by
                .get(node.node_id.as_str())
                .map(|values| values.iter().cloned().collect())
                .unwrap_or_default();
            NodeResult {
                node_id: node.node_id.clone(),
                status,
                invalidated_by: causes,
            }
        })
        .collect::<Vec<_>>();
    node_results.sort_by(|left, right| left.node_id.cmp(&right.node_id));

    let invalidated_node_ids = node_results
        .iter()
        .filter(|node| node.status != "CURRENT")
        .map(|node| node.node_id.clone())
        .collect::<Vec<_>>();
    let invalidated_required_root_node_ids = case
        .required_root_node_ids
        .iter()
        .filter(|root| statuses.contains_key(root.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let disposition = if invalidated_required_root_node_ids.is_empty() {
        "CURRENT"
    } else {
        "REQUALIFICATION_REQUIRED"
    };

    let expected_invalidated = case
        .expected_invalidated_node_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let actual_invalidated = invalidated_node_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut expected_roots = case.required_root_node_ids.clone();
    expected_roots.sort();
    expected_roots.dedup();
    let expected_result_matched = expected_roots.len() == case.required_root_node_ids.len()
        && expected_invalidated == actual_invalidated
        && disposition == case.expected_disposition;

    CaseResult {
        case_id: case.case_id.clone(),
        disposition: disposition.to_owned(),
        required_root_node_ids: case.required_root_node_ids.clone(),
        invalidated_required_root_node_ids,
        invalidated_node_count: invalidated_node_ids.len(),
        current_node_count: node_results.len() - invalidated_node_ids.len(),
        invalidated_node_ids,
        nodes: node_results,
        authority_granted: false,
        expected_result_matched,
    }
}

pub fn evaluate(corpus: &Corpus) -> Result<Evaluation, String> {
    if corpus.schema != SCHEMA
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.is_empty()
        || corpus.cases.len() > MAX_CASES
    {
        return Err("unsupported assurance-invalidation corpus or claim ceiling".to_owned());
    }

    let mut case_ids = BTreeSet::new();
    let mut cases = Vec::new();
    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err("assurance-invalidation case IDs must be unique".to_owned());
        }
        cases.push(evaluate_case(case));
    }
    cases.sort_by(|left, right| left.case_id.cmp(&right.case_id));

    let count = |disposition: &str| {
        cases
            .iter()
            .filter(|case| case.disposition == disposition)
            .count()
    };
    let node_status_count = |status: &str| {
        cases
            .iter()
            .flat_map(|case| &case.nodes)
            .filter(|node| node.status == status)
            .count()
    };
    let summary = Summary {
        case_count: cases.len(),
        current_cases: count("CURRENT"),
        requalification_required_cases: count("REQUALIFICATION_REQUIRED"),
        invalid_graph_cases: count("INVALID_GRAPH"),
        source_changed_nodes: node_status_count("SOURCE_CHANGED"),
        scope_changed_nodes: node_status_count("SCOPE_CHANGED"),
        dependency_version_mismatch_nodes: node_status_count("DEPENDENCY_VERSION_MISMATCH"),
        upstream_invalidated_nodes: node_status_count("UPSTREAM_INVALIDATED"),
        invalidated_required_roots: cases
            .iter()
            .map(|case| case.invalidated_required_root_node_ids.len())
            .sum(),
        expected_result_mismatches: cases
            .iter()
            .filter(|case| !case.expected_result_matched)
            .count(),
        authority_grants: cases.iter().filter(|case| case.authority_granted).count(),
    };
    let pass = summary.expected_result_mismatches == 0 && summary.authority_grants == 0;

    Ok(Evaluation {
        result: if pass { "pass" } else { "fail" }.to_owned(),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        summary,
        cases,
        authority_granted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Corpus {
        serde_json::from_str(include_str!("../fixtures/v0.1/invalidation-cases.json"))
            .expect("invalidation corpus parses")
    }

    fn case<'a>(evaluation: &'a Evaluation, id: &str) -> &'a CaseResult {
        evaluation
            .cases
            .iter()
            .find(|item| item.case_id == id)
            .expect("invalidation case exists")
    }

    #[test]
    fn unchanged_dependency_graph_remains_current() {
        let result = evaluate(&fixture()).expect("fixture evaluates");
        assert_eq!(case(&result, "baseline_current").disposition, "CURRENT");
        assert!(
            case(&result, "baseline_current")
                .invalidated_node_ids
                .is_empty()
        );
    }

    #[test]
    fn domain_change_invalidates_every_dependent_decision_claim() {
        let result = evaluate(&fixture()).expect("fixture evaluates");
        let item = case(&result, "domain_snapshot_changed");
        assert_eq!(item.disposition, "REQUALIFICATION_REQUIRED");
        for expected in [
            "domain",
            "closure",
            "domain_oracle",
            "feasibility",
            "query_policy",
            "loss_model",
            "regret",
            "calibration",
            "decision_frontier",
        ] {
            assert!(item.invalidated_node_ids.iter().any(|id| id == expected));
        }
        assert!(
            !item
                .invalidated_node_ids
                .iter()
                .any(|id| id == "unrelated_campaign")
        );
    }

    #[test]
    fn loss_model_change_invalidates_regret_and_calibration_but_not_feasibility() {
        let result = evaluate(&fixture()).expect("fixture evaluates");
        let item = case(&result, "loss_model_changed");
        assert_eq!(item.disposition, "REQUALIFICATION_REQUIRED");
        for expected in ["loss_model", "regret", "calibration", "decision_frontier"] {
            assert!(item.invalidated_node_ids.iter().any(|id| id == expected));
        }
        for preserved in ["closure", "feasibility", "query_policy", "domain_oracle"] {
            assert!(!item.invalidated_node_ids.iter().any(|id| id == preserved));
        }
    }

    #[test]
    fn unrelated_evidence_change_does_not_invalidate_decision_frontier() {
        let result = evaluate(&fixture()).expect("fixture evaluates");
        let item = case(&result, "unrelated_evidence_changed");
        assert_eq!(item.disposition, "CURRENT");
        assert_eq!(
            item.invalidated_node_ids,
            vec!["unrelated_campaign".to_owned()]
        );
        assert!(item.invalidated_required_root_node_ids.is_empty());
    }

    #[test]
    fn scope_change_invalidates_dependent_claims_even_when_content_digest_is_unchanged() {
        let result = evaluate(&fixture()).expect("fixture evaluates");
        let item = case(&result, "domain_scope_changed");
        assert_eq!(item.disposition, "REQUALIFICATION_REQUIRED");
        assert!(
            item.invalidated_node_ids
                .iter()
                .any(|id| id == "decision_frontier")
        );
        assert!(
            item.nodes
                .iter()
                .any(|node| { node.node_id == "domain" && node.status == "SCOPE_CHANGED" })
        );
    }

    #[test]
    fn missing_dependency_and_cycles_fail_closed() {
        let result = evaluate(&fixture()).expect("fixture evaluates");
        assert_eq!(
            case(&result, "missing_dependency").disposition,
            "INVALID_GRAPH"
        );
        assert_eq!(
            case(&result, "dependency_cycle").disposition,
            "INVALID_GRAPH"
        );
    }

    #[test]
    fn input_permutation_does_not_change_invalidation_result() {
        let original = fixture();
        let expected = evaluate(&original).expect("original evaluates");
        let mut permuted = fixture();
        for case in &mut permuted.cases {
            case.nodes.reverse();
            case.required_root_node_ids.reverse();
            case.expected_invalidated_node_ids.reverse();
            for node in &mut case.nodes {
                node.dependencies.reverse();
            }
        }
        let actual = evaluate(&permuted).expect("permuted evaluates");
        assert_eq!(
            serde_json::to_value(actual).expect("actual serializes"),
            serde_json::to_value(expected).expect("expected serializes")
        );
    }

    #[test]
    fn no_path_grants_authority() {
        let result = evaluate(&fixture()).expect("fixture evaluates");
        assert!(!result.authority_granted);
        assert!(result.cases.iter().all(|case| !case.authority_granted));
    }
}
