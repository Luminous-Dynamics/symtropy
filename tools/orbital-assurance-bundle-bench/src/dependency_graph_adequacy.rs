//! Independent dependency-graph adequacy benchmark.
//!
//! Derives bounded expected dependencies from a transformation grammar and compares
//! them against both consumer-declared artifact inputs and the assurance graph.
//! This is deliberately separate from invalidation propagation: an omitted edge
//! prevents a correct invalidation algorithm from reaching the affected descendant.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const SCHEMA: &str = "luminous.orbital-dependency-graph-adequacy.corpus.v0.1";
const CLAIM_CEILING: &str = "synthetic-dependency-graph-adequacy-only";
const MAX_CASES: usize = 128;
const MAX_PRODUCERS: usize = 4096;
const MAX_CONSUMERS: usize = 4096;
const MAX_EDGES: usize = 16384;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Producer {
    pub node_id: String,
    pub artifact_id: String,
    pub artifact_type: String,
    pub content_sha256: String,
    pub scope_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub node_id: String,
    pub output_artifact_id: String,
    pub operation: String,
    pub input_artifact_ids: Vec<String>,
    pub scope_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct Edge {
    pub parent_node_id: String,
    pub consumer_node_id: String,
    pub artifact_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub disposition: String,
    pub missing_dependency_edges: Vec<String>,
    pub extra_dependency_edges: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub producers: Vec<Producer>,
    pub consumers: Vec<Consumer>,
    pub declared_edges: Vec<Edge>,
    pub expected: Expected,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<Case>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Finding {
    pub code: String,
    pub dependency_edge: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CaseResult {
    pub case_id: String,
    pub disposition: String,
    pub expected_disposition: String,
    pub missing_dependency_edges: Vec<String>,
    pub extra_dependency_edges: Vec<String>,
    pub findings: Vec<Finding>,
    pub producer_count: usize,
    pub consumer_count: usize,
    pub declared_edge_count: usize,
    pub expected_edge_count: usize,
    pub expected_result_matched: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub case_count: usize,
    pub graph_complete_cases: usize,
    pub missing_edge_cases: usize,
    pub common_mode_omission_cases: usize,
    pub consumer_manifest_omission_cases: usize,
    pub extra_edge_cases: usize,
    pub model_inadequate_cases: usize,
    pub source_unavailable_cases: usize,
    pub scope_mismatch_cases: usize,
    pub invalid_graph_cases: usize,
    pub missing_dependency_edge_count: usize,
    pub extra_dependency_edge_count: usize,
    pub expected_result_mismatches: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Evaluation {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub summary: Summary,
    pub cases: Vec<CaseResult>,
}

type EdgeKey = (String, String, String);

fn required_parent_types(operation: &str) -> Option<&'static [&'static str]> {
    match operation {
        "build_closure" => Some(&["domain_manifest"]),
        "build_domain_oracle" => Some(&["domain_manifest"]),
        "evaluate_feasibility" => Some(&["closure_report", "domain_oracle_report"]),
        "select_query_policy" => Some(&["feasibility_report", "domain_oracle_report"]),
        "build_loss_model" => Some(&["closure_report"]),
        "compute_regret" => Some(&["feasibility_report", "loss_model"]),
        "calibrate_regret" => Some(&["regret_report", "loss_model"]),
        "compose_decision_frontier" => Some(&["query_policy", "regret_report", "calibration"]),
        "ingest_campaign" => Some(&[]),
        _ => None,
    }
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() || byte == b' ')
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn edge_label(edge: &EdgeKey) -> String {
    format!("{}->{}#{}", edge.0, edge.1, edge.2)
}

fn add_finding(findings: &mut Vec<Finding>, code: &str, edge: Option<String>, detail: &str) {
    findings.push(Finding {
        code: code.to_owned(),
        dependency_edge: edge,
        detail: detail.to_owned(),
    });
}

fn invalid_graph(case: &Case, detail: &str) -> CaseResult {
    let mut findings = vec![Finding {
        code: "INVALID_GRAPH".to_owned(),
        dependency_edge: None,
        detail: detail.to_owned(),
    }];
    findings.sort_by(|a, b| {
        (&a.code, &a.dependency_edge, &a.detail).cmp(&(&b.code, &b.dependency_edge, &b.detail))
    });
    CaseResult {
        case_id: case.case_id.clone(),
        disposition: "INVALID_GRAPH".to_owned(),
        expected_disposition: case.expected.disposition.clone(),
        missing_dependency_edges: Vec::new(),
        extra_dependency_edges: Vec::new(),
        findings,
        producer_count: case.producers.len(),
        consumer_count: case.consumers.len(),
        declared_edge_count: case.declared_edges.len(),
        expected_edge_count: 0,
        expected_result_matched: case.expected.disposition == "INVALID_GRAPH"
            && case.expected.missing_dependency_edges.is_empty()
            && case.expected.extra_dependency_edges.is_empty(),
    }
}

fn graph_has_cycle(node_ids: &BTreeSet<String>, edges: &BTreeSet<EdgeKey>) -> bool {
    let mut indegree: BTreeMap<&str, usize> = node_ids.iter().map(|id| (id.as_str(), 0)).collect();
    let mut children: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (parent, child, _) in edges {
        if !indegree.contains_key(parent.as_str()) {
            return true;
        }
        let Some(count) = indegree.get_mut(child.as_str()) else {
            return true;
        };
        *count += 1;
        children
            .entry(parent.as_str())
            .or_default()
            .push(child.as_str());
    }
    let mut ready = indegree
        .iter()
        .filter_map(|(node_id, count)| (*count == 0).then_some(*node_id))
        .collect::<BTreeSet<_>>();
    let mut visited = 0usize;
    while let Some(node_id) = ready.pop_first() {
        visited += 1;
        for child in children.get(node_id).into_iter().flatten() {
            let Some(count) = indegree.get_mut(child) else {
                return true;
            };
            *count -= 1;
            if *count == 0 {
                ready.insert(child);
            }
        }
    }
    visited != node_ids.len()
}

fn evaluate_case(case: &Case) -> CaseResult {
    if !valid_id(&case.case_id)
        || case.producers.is_empty()
        || case.producers.len() > MAX_PRODUCERS
        || case.consumers.is_empty()
        || case.consumers.len() > MAX_CONSUMERS
        || case.declared_edges.len() > MAX_EDGES
        || !matches!(
            case.expected.disposition.as_str(),
            "GRAPH_COMPLETE_WITHIN_DECLARED_GRAMMAR"
                | "MISSING_DEPENDENCY_EDGE"
                | "COMMON_MODE_DEPENDENCY_OMISSION"
                | "CONSUMER_MANIFEST_OMISSION"
                | "UNDECLARED_DEPENDENCY_EDGE"
                | "DEPENDENCY_MODEL_INADEQUATE"
                | "DEPENDENCY_SOURCE_UNAVAILABLE"
                | "DEPENDENCY_SCOPE_MISMATCH"
                | "INVALID_GRAPH"
        )
    {
        return invalid_graph(case, "invalid case identity or exceeded bounds");
    }

    let mut producers_by_node = BTreeMap::new();
    let mut producer_by_artifact = BTreeMap::new();
    let mut producers_by_type: BTreeMap<&str, Vec<&Producer>> = BTreeMap::new();
    let mut node_ids = BTreeSet::new();
    for producer in &case.producers {
        if !valid_id(&producer.node_id)
            || !valid_id(&producer.artifact_id)
            || !valid_id(&producer.artifact_type)
            || !valid_sha256(&producer.content_sha256)
            || !valid_sha256(&producer.scope_sha256)
            || producers_by_node
                .insert(producer.node_id.as_str(), producer)
                .is_some()
            || producer_by_artifact
                .insert(producer.artifact_id.as_str(), producer)
                .is_some()
        {
            return invalid_graph(
                case,
                "duplicate producer identity or malformed artifact binding",
            );
        }
        node_ids.insert(producer.node_id.clone());
        producers_by_type
            .entry(producer.artifact_type.as_str())
            .or_default()
            .push(producer);
    }

    let mut consumers_by_node = BTreeMap::new();
    let mut findings = Vec::new();
    for consumer in &case.consumers {
        if !valid_id(&consumer.node_id)
            || !valid_id(&consumer.output_artifact_id)
            || !valid_id(&consumer.operation)
            || !valid_sha256(&consumer.scope_sha256)
            || consumer.input_artifact_ids.iter().any(|id| !valid_id(id))
            || consumers_by_node
                .insert(consumer.node_id.as_str(), consumer)
                .is_some()
        {
            return invalid_graph(
                case,
                "duplicate consumer identity or malformed consumer manifest",
            );
        }
        let Some(output) = producers_by_node.get(consumer.node_id.as_str()) else {
            return invalid_graph(case, "consumer output has no producer record");
        };
        if output.artifact_id != consumer.output_artifact_id {
            return invalid_graph(
                case,
                "consumer output artifact differs from producer manifest",
            );
        }
        if output.scope_sha256 != consumer.scope_sha256 {
            add_finding(
                &mut findings,
                "DEPENDENCY_SCOPE_MISMATCH",
                Some(format!("output:{}", consumer.node_id)),
                "consumer-produced artifact scope differs from its transformation scope",
            );
        }
        let mut inputs = BTreeSet::new();
        if consumer
            .input_artifact_ids
            .iter()
            .any(|id| !inputs.insert(id.as_str()))
        {
            return invalid_graph(
                case,
                "consumer input manifest contains duplicate artifact IDs",
            );
        }
    }

    let mut declared = BTreeSet::new();
    for edge in &case.declared_edges {
        if !valid_id(&edge.parent_node_id)
            || !valid_id(&edge.consumer_node_id)
            || !valid_id(&edge.artifact_id)
            || !producers_by_node.contains_key(edge.parent_node_id.as_str())
            || !consumers_by_node.contains_key(edge.consumer_node_id.as_str())
            || producers_by_node[edge.parent_node_id.as_str()].artifact_id != edge.artifact_id
            || edge.parent_node_id == edge.consumer_node_id
        {
            return invalid_graph(
                case,
                "declared edge has missing endpoint or mismatched artifact",
            );
        }
        let key = (
            edge.parent_node_id.clone(),
            edge.consumer_node_id.clone(),
            edge.artifact_id.clone(),
        );
        if !declared.insert(key) {
            return invalid_graph(case, "declared dependency graph contains duplicate edges");
        }
    }

    if graph_has_cycle(&node_ids, &declared) {
        return invalid_graph(case, "declared dependency graph contains a cycle");
    }

    let mut expected = BTreeSet::new();
    let mut missing = BTreeSet::new();
    let mut extra = BTreeSet::new();

    for consumer in &case.consumers {
        let Some(required_types) = required_parent_types(&consumer.operation) else {
            add_finding(
                &mut findings,
                "DEPENDENCY_MODEL_INADEQUATE",
                None,
                &format!(
                    "operation {} has no registered dependency grammar",
                    consumer.operation
                ),
            );
            continue;
        };

        let input_ids = consumer
            .input_artifact_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        for parent_type in required_types {
            let parents = producers_by_type
                .get(parent_type)
                .cloned()
                .unwrap_or_default();
            if parents.len() != 1 {
                add_finding(
                    &mut findings,
                    "DEPENDENCY_SOURCE_UNAVAILABLE",
                    None,
                    &format!(
                        "operation {} requires exactly one producer of type {}, found {}",
                        consumer.operation,
                        parent_type,
                        parents.len()
                    ),
                );
                continue;
            }
            let parent = parents[0];
            if parent.scope_sha256 != consumer.scope_sha256 {
                add_finding(
                    &mut findings,
                    "DEPENDENCY_SCOPE_MISMATCH",
                    Some(format!("{}->{}", parent.node_id, consumer.node_id)),
                    "producer and consumer scope digests differ",
                );
            }

            let key = (
                parent.node_id.clone(),
                consumer.node_id.clone(),
                parent.artifact_id.clone(),
            );
            expected.insert(key.clone());
            let label = edge_label(&key);
            let manifest_has = input_ids.contains(parent.artifact_id.as_str());
            let graph_has = declared.contains(&key);
            if !manifest_has && !graph_has {
                add_finding(
                    &mut findings,
                    "COMMON_MODE_DEPENDENCY_OMISSION",
                    Some(label.clone()),
                    "both consumer input manifest and declared graph omit a grammar-required parent",
                );
                missing.insert(label);
            } else if !manifest_has {
                add_finding(
                    &mut findings,
                    "CONSUMER_MANIFEST_OMISSION",
                    Some(label.clone()),
                    "consumer input manifest omits a grammar-required artifact while graph edge exists",
                );
            } else if !graph_has {
                add_finding(
                    &mut findings,
                    "MISSING_DEPENDENCY_EDGE",
                    Some(label.clone()),
                    "consumer input manifest declares the artifact but assurance graph omits its edge",
                );
                missing.insert(label);
            }
        }

        for artifact_id in &consumer.input_artifact_ids {
            if !expected.iter().any(|(_, node_id, artifact)| {
                node_id == &consumer.node_id && artifact == artifact_id
            }) {
                add_finding(
                    &mut findings,
                    "UNDECLARED_DEPENDENCY_EDGE",
                    Some(format!("artifact:{}->{}", artifact_id, consumer.node_id)),
                    "consumer declares an input not required by the registered transformation grammar",
                );
            }
        }
    }

    for edge in &declared {
        if !expected.contains(edge) {
            let label = edge_label(edge);
            extra.insert(label.clone());
            add_finding(
                &mut findings,
                "UNDECLARED_DEPENDENCY_EDGE",
                Some(label),
                "assurance graph contains an edge not required by the registered transformation grammar",
            );
        }
    }

    findings.sort_by(|a, b| {
        (&a.code, &a.dependency_edge, &a.detail).cmp(&(&b.code, &b.dependency_edge, &b.detail))
    });
    findings.dedup();

    let codes = findings
        .iter()
        .map(|finding| finding.code.as_str())
        .collect::<BTreeSet<_>>();
    let disposition = if codes.contains("INVALID_GRAPH") {
        "INVALID_GRAPH"
    } else if codes.contains("DEPENDENCY_MODEL_INADEQUATE") {
        "DEPENDENCY_MODEL_INADEQUATE"
    } else if codes.contains("DEPENDENCY_SOURCE_UNAVAILABLE") {
        "DEPENDENCY_SOURCE_UNAVAILABLE"
    } else if codes.contains("DEPENDENCY_SCOPE_MISMATCH") {
        "DEPENDENCY_SCOPE_MISMATCH"
    } else if codes.contains("COMMON_MODE_DEPENDENCY_OMISSION") {
        "COMMON_MODE_DEPENDENCY_OMISSION"
    } else if codes.contains("CONSUMER_MANIFEST_OMISSION") {
        "CONSUMER_MANIFEST_OMISSION"
    } else if codes.contains("MISSING_DEPENDENCY_EDGE") {
        "MISSING_DEPENDENCY_EDGE"
    } else if codes.contains("UNDECLARED_DEPENDENCY_EDGE") {
        "UNDECLARED_DEPENDENCY_EDGE"
    } else {
        "GRAPH_COMPLETE_WITHIN_DECLARED_GRAMMAR"
    };

    let missing_dependency_edges = missing.into_iter().collect::<Vec<_>>();
    let extra_dependency_edges = extra.into_iter().collect::<Vec<_>>();
    let expected_missing = case
        .expected
        .missing_dependency_edges
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let expected_extra = case
        .expected
        .extra_dependency_edges
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let expected_result_matched = disposition == case.expected.disposition
        && expected_missing == missing_dependency_edges.iter().cloned().collect()
        && expected_extra == extra_dependency_edges.iter().cloned().collect()
        && case.expected.missing_dependency_edges.len() == expected_missing.len()
        && case.expected.extra_dependency_edges.len() == expected_extra.len();

    CaseResult {
        case_id: case.case_id.clone(),
        disposition: disposition.to_owned(),
        expected_disposition: case.expected.disposition.clone(),
        missing_dependency_edges,
        extra_dependency_edges,
        findings,
        producer_count: case.producers.len(),
        consumer_count: case.consumers.len(),
        declared_edge_count: declared.len(),
        expected_edge_count: expected.len(),
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
        return Err("unsupported dependency-graph adequacy corpus or claim ceiling".to_owned());
    }

    let mut case_ids = BTreeSet::new();
    let mut cases = Vec::new();
    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err("dependency-graph adequacy case IDs must be unique".to_owned());
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
    let summary = Summary {
        case_count: cases.len(),
        graph_complete_cases: count("GRAPH_COMPLETE_WITHIN_DECLARED_GRAMMAR"),
        missing_edge_cases: count("MISSING_DEPENDENCY_EDGE"),
        common_mode_omission_cases: count("COMMON_MODE_DEPENDENCY_OMISSION"),
        consumer_manifest_omission_cases: count("CONSUMER_MANIFEST_OMISSION"),
        extra_edge_cases: count("UNDECLARED_DEPENDENCY_EDGE"),
        model_inadequate_cases: count("DEPENDENCY_MODEL_INADEQUATE"),
        source_unavailable_cases: count("DEPENDENCY_SOURCE_UNAVAILABLE"),
        scope_mismatch_cases: count("DEPENDENCY_SCOPE_MISMATCH"),
        invalid_graph_cases: count("INVALID_GRAPH"),
        missing_dependency_edge_count: cases
            .iter()
            .map(|case| case.missing_dependency_edges.len())
            .sum(),
        extra_dependency_edge_count: cases
            .iter()
            .map(|case| case.extra_dependency_edges.len())
            .sum(),
        expected_result_mismatches: cases
            .iter()
            .filter(|case| !case.expected_result_matched)
            .count(),
    };
    let result = if summary.expected_result_mismatches == 0
        && summary.graph_complete_cases >= 1
        && summary.common_mode_omission_cases >= 1
    {
        "pass"
    } else {
        "fail"
    };

    Ok(Evaluation {
        result: result.to_owned(),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        summary,
        cases,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Corpus {
        serde_json::from_str(include_str!("../fixtures/v0.1/dependency-graph-cases.json"))
            .expect("dependency graph corpus parses")
    }

    fn case<'a>(evaluation: &'a Evaluation, id: &str) -> &'a CaseResult {
        evaluation
            .cases
            .iter()
            .find(|item| item.case_id == id)
            .expect("case exists")
    }

    #[test]
    fn complete_graph_matches_independent_grammar() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        let result = case(&report, "baseline_complete");
        assert_eq!(result.disposition, "GRAPH_COMPLETE_WITHIN_DECLARED_GRAMMAR");
        assert!(result.expected_result_matched);
    }

    #[test]
    fn missing_graph_edge_is_detected_when_manifest_still_lists_input() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        let result = case(&report, "missing_graph_edge");
        assert_eq!(result.disposition, "MISSING_DEPENDENCY_EDGE");
        assert_eq!(
            result.missing_dependency_edges,
            vec!["domain_oracle->query_policy#artifact-domain-oracle"]
        );
    }

    #[test]
    fn common_mode_omission_is_detected_even_when_both_declarations_agree() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        let result = case(&report, "common_mode_omission");
        assert_eq!(result.disposition, "COMMON_MODE_DEPENDENCY_OMISSION");
        assert_eq!(
            result.missing_dependency_edges,
            vec!["domain_oracle->query_policy#artifact-domain-oracle"]
        );
    }

    #[test]
    fn consumer_manifest_omission_is_separate_from_graph_omission() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        assert_eq!(
            case(&report, "consumer_manifest_omission").disposition,
            "CONSUMER_MANIFEST_OMISSION"
        );
    }

    #[test]
    fn extra_edges_are_visible_as_overapproximation() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        let result = case(&report, "extra_edge");
        assert_eq!(result.disposition, "UNDECLARED_DEPENDENCY_EDGE");
        assert_eq!(
            result.extra_dependency_edges,
            vec!["unrelated_campaign->decision_frontier#artifact-campaign"]
        );
    }

    #[test]
    fn unknown_transform_does_not_become_a_closed_graph() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        assert_eq!(
            case(&report, "unknown_transform").disposition,
            "DEPENDENCY_MODEL_INADEQUATE"
        );
    }

    #[test]
    fn scope_mismatch_blocks_graph_adequacy() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        let result = case(&report, "scope_mismatch");
        assert_eq!(result.disposition, "DEPENDENCY_SCOPE_MISMATCH");
        assert!(result.findings.iter().any(|finding| {
            finding.code == "DEPENDENCY_SCOPE_MISMATCH"
                && finding.dependency_edge.as_deref() == Some("output:domain_oracle")
        }));
    }

    #[test]
    fn input_permutation_is_deterministic() {
        let expected = evaluate(&fixture()).expect("corpus evaluates");
        let mut permuted = fixture();
        for case in &mut permuted.cases {
            case.producers.reverse();
            case.consumers.reverse();
            case.declared_edges.reverse();
            for consumer in &mut case.consumers {
                consumer.input_artifact_ids.reverse();
            }
        }
        let actual = evaluate(&permuted).expect("permuted corpus evaluates");
        assert_eq!(
            serde_json::to_value(actual).expect("actual serializes"),
            serde_json::to_value(expected).expect("expected serializes")
        );
    }
}
