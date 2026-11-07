// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    corpus_id: String,
    schema_version: String,
    claim_ceiling: String,
    split_guard: SplitGuard,
    cases: Vec<Case>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SplitGuard {
    training_group_ids: Vec<String>,
    evaluation_group_ids: Vec<String>,
    candidate_input_fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    case_id: String,
    split_group_id: String,
    reference: Reference,
    candidate: Candidate,
    expected: Expected,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reference {
    asset_id: String,
    model_revision: String,
    frame_id: String,
    elements: Vec<ElementRequirement>,
    required_topology: Vec<TopologyEdge>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ElementRequirement {
    element_id: String,
    class: String,
    tolerance_mm: f64,
    min_visible_fraction: f64,
    min_independent_lineages: usize,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
struct TopologyEdge {
    from: String,
    to: String,
    relation: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Candidate {
    asset_id: String,
    base_model_revision: String,
    frame_id: String,
    survey_qualified: bool,
    claims_unobserved_geometry: bool,
    elements: Vec<ObservedElement>,
    topology: Vec<TopologyEdge>,
    delta: ProposedDelta,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ObservedElement {
    element_id: String,
    class: String,
    deviation_mm: f64,
    uncertainty_mm: f64,
    visible_fraction: f64,
    lineage_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProposedDelta {
    operation: String,
    base_model_revision: String,
    target_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Expected {
    disposition: String,
    reason_codes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CaseResult {
    case_id: String,
    disposition: String,
    reason_codes: Vec<String>,
    expected_disposition: String,
    expected_reason_codes: Vec<String>,
    matches_expected: bool,
}

#[derive(Debug, Serialize)]
struct CorpusResult {
    corpus_id: String,
    schema_version: String,
    corpus_sha256: String,
    case_count: usize,
    matched_expected_cases: usize,
    mismatched_cases: usize,
    passed: bool,
    claim_ceiling: String,
    cases: Vec<CaseResult>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decision(disposition: &str, reason: &str) -> (String, Vec<String>) {
    (disposition.to_owned(), vec![reason.to_owned()])
}

fn evaluate(case: &Case) -> (String, Vec<String>) {
    let reference = &case.reference;
    let candidate = &case.candidate;

    if !candidate.survey_qualified {
        return decision(
            "STALE_OR_INVALID_EVIDENCE",
            "UPSTREAM_SURVEY_PRODUCT_NOT_QUALIFIED",
        );
    }
    if candidate.asset_id != reference.asset_id {
        return decision("STALE_OR_INVALID_EVIDENCE", "ASSET_ID_MISMATCH");
    }
    if candidate.base_model_revision != reference.model_revision
        || candidate.delta.base_model_revision != reference.model_revision
    {
        return decision("STALE_OR_INVALID_EVIDENCE", "BASE_MODEL_REVISION_MISMATCH");
    }
    if candidate.frame_id != reference.frame_id {
        return decision("STALE_OR_INVALID_EVIDENCE", "COORDINATE_FRAME_MISMATCH");
    }
    if reference.asset_id.is_empty()
        || reference.model_revision.is_empty()
        || reference.frame_id.is_empty()
    {
        return decision("INVALID_INPUT", "REFERENCE_CONTEXT_INCOMPLETE");
    }
    if candidate.claims_unobserved_geometry {
        return decision(
            "PARTIAL_WITH_UNKNOWN",
            "UNOBSERVED_GEOMETRY_CLAIMED_AS_KNOWN",
        );
    }

    let required_edges: BTreeSet<TopologyEdge> =
        reference.required_topology.iter().cloned().collect();
    if required_edges.len() != reference.required_topology.len() {
        return decision("INVALID_INPUT", "REFERENCE_TOPOLOGY_HAS_DUPLICATE_EDGES");
    }

    if reference.elements.is_empty() {
        return decision("INVALID_INPUT", "REFERENCE_HAS_NO_REQUIRED_ELEMENTS");
    }

    let mut requirements = BTreeMap::new();
    for requirement in &reference.elements {
        if requirement.element_id.is_empty()
            || requirement.class.is_empty()
            || !requirement.tolerance_mm.is_finite()
            || requirement.tolerance_mm < 0.0
            || !requirement.min_visible_fraction.is_finite()
            || !(0.0..=1.0).contains(&requirement.min_visible_fraction)
            || requirement.min_independent_lineages == 0
            || requirements
                .insert(requirement.element_id.as_str(), requirement)
                .is_some()
        {
            return decision(
                "INVALID_INPUT",
                "REFERENCE_REQUIREMENT_INVALID_OR_DUPLICATED",
            );
        }
    }

    let mut observations = BTreeMap::new();
    for observation in &candidate.elements {
        if observation.element_id.is_empty()
            || observation.class.is_empty()
            || !observation.deviation_mm.is_finite()
            || !observation.uncertainty_mm.is_finite()
            || observation.uncertainty_mm < 0.0
            || !observation.visible_fraction.is_finite()
            || !(0.0..=1.0).contains(&observation.visible_fraction)
            || observation.lineage_ids.iter().any(String::is_empty)
            || observations
                .insert(observation.element_id.as_str(), observation)
                .is_some()
        {
            return decision("INVALID_INPUT", "UNCERTAINTY_OR_GEOMETRY_VALUE_INVALID");
        }
        if !requirements.contains_key(observation.element_id.as_str()) {
            return decision(
                "CONFLICTED_CORRESPONDENCE",
                "OBSERVED_ELEMENT_ID_UNRESOLVED",
            );
        }
    }

    for (element_id, requirement) in &requirements {
        let Some(observation) = observations.get(*element_id) else {
            return decision("PARTIAL_WITH_UNKNOWN", "REQUIRED_ELEMENT_NOT_OBSERVED");
        };
        if observation.class != requirement.class {
            return decision(
                "CONFLICTED_CORRESPONDENCE",
                "ELEMENT_CLASS_CONFLICTS_WITH_PERSISTENT_IDENTITY",
            );
        }
        if observation.visible_fraction < requirement.min_visible_fraction {
            return decision(
                "PARTIAL_WITH_UNKNOWN",
                "REQUIRED_ELEMENT_VISIBILITY_INSUFFICIENT",
            );
        }
        let unique_lineages: BTreeSet<&str> =
            observation.lineage_ids.iter().map(String::as_str).collect();
        if unique_lineages.len() < requirement.min_independent_lineages {
            return decision(
                "INSUFFICIENT_INDEPENDENT_EVIDENCE",
                "CORRELATED_LINEAGES_DO_NOT_COUNT_AS_INDEPENDENT",
            );
        }

        let lower = (observation.deviation_mm.abs() - observation.uncertainty_mm).max(0.0);
        let upper = observation.deviation_mm.abs() + observation.uncertainty_mm;
        if lower <= requirement.tolerance_mm && upper > requirement.tolerance_mm {
            return decision(
                "REQUEST_ADDITIONAL_OBSERVATION",
                "UNCERTAINTY_INTERVAL_CROSSES_DECISION_TOLERANCE",
            );
        }
        if lower > requirement.tolerance_mm {
            if candidate.delta.operation != "modify" {
                return decision("REAL_CHANGE_CANDIDATE", "GEOMETRY_OUTSIDE_TOLERANCE");
            }
            if !candidate
                .delta
                .target_ids
                .iter()
                .any(|target| target.as_str() == *element_id)
            {
                return decision(
                    "REAL_CHANGE_CANDIDATE",
                    "CHANGE_NOT_INCLUDED_IN_PROPOSED_DELTA",
                );
            }
        }
    }

    let candidate_edges: BTreeSet<TopologyEdge> = candidate.topology.iter().cloned().collect();
    if candidate_edges.len() != candidate.topology.len() {
        return decision("REJECTED_TOPOLOGY", "DUPLICATE_TOPOLOGY_EDGE");
    }
    if required_edges
        .iter()
        .any(|edge| !candidate_edges.contains(edge))
    {
        return decision("REJECTED_TOPOLOGY", "REQUIRED_TOPOLOGY_EDGE_MISSING");
    }
    for edge in &candidate.topology {
        if edge.relation != "hosts_opening" {
            return decision("REJECTED_TOPOLOGY", "TOPOLOGY_RELATION_UNSUPPORTED");
        }
        if edge.from == edge.to
            || !requirements.contains_key(edge.from.as_str())
            || !requirements.contains_key(edge.to.as_str())
        {
            return decision("REJECTED_TOPOLOGY", "TOPOLOGY_EDGE_HAS_INVALID_ENDPOINT");
        }
    }

    match candidate.delta.operation.as_str() {
        "none" => {
            if !candidate.delta.target_ids.is_empty() {
                return decision("INVALID_INPUT", "NO_CHANGE_DELTA_HAS_TARGETS");
            }
            decision(
                "NO_CHANGE_SUPPORTED",
                "ALL_REQUIRED_ELEMENTS_WITHIN_DECLARED_BOUNDS",
            )
        },
        "modify" => {
            let unique_targets: BTreeSet<&str> = candidate
                .delta
                .target_ids
                .iter()
                .map(String::as_str)
                .collect();
            if unique_targets.len() != candidate.delta.target_ids.len() {
                return decision("INVALID_INPUT", "MODIFY_DELTA_HAS_DUPLICATE_TARGETS");
            }
            if candidate.delta.target_ids.is_empty() {
                return decision("INVALID_INPUT", "MODIFY_DELTA_HAS_NO_TARGETS");
            }
            for target in &candidate.delta.target_ids {
                let Some(requirement) = requirements.get(target.as_str()) else {
                    return decision("CONFLICTED_CORRESPONDENCE", "DELTA_TARGET_ID_UNRESOLVED");
                };
                let Some(observation) = observations.get(target.as_str()) else {
                    return decision("PARTIAL_WITH_UNKNOWN", "DELTA_TARGET_NOT_OBSERVED");
                };
                let lower = (observation.deviation_mm.abs() - observation.uncertainty_mm).max(0.0);
                if lower <= requirement.tolerance_mm {
                    return decision(
                        "REQUEST_ADDITIONAL_OBSERVATION",
                        "DELTA_NOT_SEPARATED_FROM_TOLERANCE",
                    );
                }
            }
            decision(
                "ADMIT_CANDIDATE_DELTA",
                "CHANGE_OUTSIDE_TOLERANCE_WITH_UNCERTAINTY_MARGIN",
            )
        }
        _ => decision("NOT_EVALUATED", "DELTA_OPERATION_OUTSIDE_V0_1_SCOPE"),
    }
}

fn evaluate_corpus(bytes: &[u8]) -> Result<CorpusResult, Box<dyn Error>> {
    let corpus: Corpus = serde_json::from_slice(bytes)?;
    if corpus.corpus_id.trim().is_empty()
        || corpus.schema_version != "0.1"
        || corpus.claim_ceiling.trim().is_empty()
    {
        return Err("unsupported or missing corpus identity/schema version".into());
    }
    if corpus.cases.is_empty() {
        return Err("corpus must contain at least one case".into());
    }
    if corpus.split_guard.training_group_ids.is_empty()
        || corpus.split_guard.evaluation_group_ids.is_empty()
        || corpus.split_guard.candidate_input_fields.is_empty()
    {
        return Err(
            "split guard must declare train groups, evaluation groups and candidate inputs".into(),
        );
    }
    if corpus
        .split_guard
        .training_group_ids
        .iter()
        .chain(corpus.split_guard.evaluation_group_ids.iter())
        .any(|group| group.trim().is_empty())
        || corpus
            .split_guard
            .candidate_input_fields
            .iter()
            .any(|field| field.trim().is_empty())
    {
        return Err("split guard entries must be non-empty".into());
    }
    let unique_candidate_fields: BTreeSet<&str> = corpus
        .split_guard
        .candidate_input_fields
        .iter()
        .map(String::as_str)
        .collect();
    if unique_candidate_fields.len() != corpus.split_guard.candidate_input_fields.len() {
        return Err("candidate input allowlist contains duplicate fields".into());
    }
    let training_groups: BTreeSet<&str> = corpus
        .split_guard
        .training_group_ids
        .iter()
        .map(String::as_str)
        .collect();
    let evaluation_groups: BTreeSet<&str> = corpus
        .split_guard
        .evaluation_group_ids
        .iter()
        .map(String::as_str)
        .collect();
    if training_groups.len() != corpus.split_guard.training_group_ids.len()
        || evaluation_groups.len() != corpus.split_guard.evaluation_group_ids.len()
        || !training_groups.is_disjoint(&evaluation_groups)
    {
        return Err("training and evaluation group identities must be unique and disjoint".into());
    }
    let forbidden_input_markers = [
        "ground_truth",
        "sealed_truth",
        "evaluator_geometry",
        "reference_bbox",
        "reference_bounding_box",
    ];
    if corpus
        .split_guard
        .candidate_input_fields
        .iter()
        .any(|field| {
            let normalized = field.to_ascii_lowercase();
            forbidden_input_markers
                .iter()
                .any(|marker| normalized.contains(*marker))
        })
    {
        return Err("candidate input allowlist exposes evaluator truth or reference bounds".into());
    }

    let mut ids = BTreeSet::new();
    let mut results = Vec::with_capacity(corpus.cases.len());
    for case in &corpus.cases {
        if case.case_id.is_empty() || !ids.insert(case.case_id.as_str()) {
            return Err(format!("empty or duplicate case id: {}", case.case_id).into());
        }
        if !evaluation_groups.contains(case.split_group_id.as_str()) {
            return Err(format!(
                "case {} belongs to a non-evaluation split group: {}",
                case.case_id, case.split_group_id
            )
            .into());
        }
        let (disposition, reason_codes) = evaluate(case);
        let matches_expected =
            disposition == case.expected.disposition && reason_codes == case.expected.reason_codes;
        results.push(CaseResult {
            case_id: case.case_id.clone(),
            disposition,
            reason_codes,
            expected_disposition: case.expected.disposition.clone(),
            expected_reason_codes: case.expected.reason_codes.clone(),
            matches_expected,
        });
    }

    let matched_expected_cases = results.iter().filter(|item| item.matches_expected).count();
    let mismatched_cases = results.len() - matched_expected_cases;
    Ok(CorpusResult {
        corpus_id: corpus.corpus_id,
        schema_version: corpus.schema_version,
        corpus_sha256: sha256_hex(bytes),
        case_count: results.len(),
        matched_expected_cases,
        mismatched_cases,
        passed: mismatched_cases == 0,
        claim_ceiling: corpus.claim_ceiling,
        cases: results,
    })
}

fn parse_args() -> Result<PathBuf, Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let mut corpus = PathBuf::from("docs/research/built-asset/scan-bim/v0.1/corpus.json");
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--corpus" => corpus = args.next().ok_or("--corpus requires a path")?.into(),
            "--help" | "-h" => {
                println!("scan-bim-admission-oracle [--corpus PATH]");
                std::process::exit(0);
            }
            unknown => return Err(format!("unknown argument: {unknown}").into()),
        }
    }
    Ok(corpus)
}

fn run() -> Result<bool, Box<dyn Error>> {
    let corpus_path = parse_args()?;
    let bytes = fs::read(corpus_path)?;
    let report = evaluate_corpus(&bytes)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(report.passed)
}

fn main() {
    match run() {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(error) => {
            eprintln!("scan-bim-admission-oracle: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const FIXTURE: &str =
        include_str!("../../../docs/research/built-asset/scan-bim/v0.1/corpus.json");

    #[test]
    fn frozen_corpus_matches_dispositions_and_reason_codes() {
        let report = evaluate_corpus(FIXTURE.as_bytes()).expect("fixture parses");
        assert_eq!(report.case_count, 25);
        assert_eq!(report.matched_expected_cases, 25);
        assert_eq!(report.mismatched_cases, 0);
        assert!(report.passed);
    }

    #[test]
    fn candidate_order_does_not_change_admission() {
        let corpus: Corpus = serde_json::from_str(FIXTURE).expect("fixture parses");
        let case = corpus
            .cases
            .iter()
            .find(|item| item.case_id == "SB-001")
            .unwrap();
        let original = evaluate(case);
        let mut candidate = Candidate {
            asset_id: case.candidate.asset_id.clone(),
            base_model_revision: case.candidate.base_model_revision.clone(),
            frame_id: case.candidate.frame_id.clone(),
            survey_qualified: case.candidate.survey_qualified,
            claims_unobserved_geometry: case.candidate.claims_unobserved_geometry,
            elements: case
                .candidate
                .elements
                .iter()
                .map(|item| ObservedElement {
                    element_id: item.element_id.clone(),
                    class: item.class.clone(),
                    deviation_mm: item.deviation_mm,
                    uncertainty_mm: item.uncertainty_mm,
                    visible_fraction: item.visible_fraction,
                    lineage_ids: item.lineage_ids.clone(),
                })
                .collect(),
            topology: case.candidate.topology.clone(),
            delta: ProposedDelta {
                operation: case.candidate.delta.operation.clone(),
                base_model_revision: case.candidate.delta.base_model_revision.clone(),
                target_ids: case.candidate.delta.target_ids.clone(),
            },
        };
        candidate.elements.reverse();
        candidate.topology.reverse();
        let mutated = Case {
            case_id: case.case_id.clone(),
            split_group_id: case.split_group_id.clone(),
            reference: Reference {
                asset_id: case.reference.asset_id.clone(),
                model_revision: case.reference.model_revision.clone(),
                frame_id: case.reference.frame_id.clone(),
                elements: case
                    .reference
                    .elements
                    .iter()
                    .map(|item| ElementRequirement {
                        element_id: item.element_id.clone(),
                        class: item.class.clone(),
                        tolerance_mm: item.tolerance_mm,
                        min_visible_fraction: item.min_visible_fraction,
                        min_independent_lineages: item.min_independent_lineages,
                    })
                    .collect(),
                required_topology: case.reference.required_topology.clone(),
            },
            candidate,
            expected: Expected {
                disposition: case.expected.disposition.clone(),
                reason_codes: case.expected.reason_codes.clone(),
            },
        };
        assert_eq!(original, evaluate(&mutated));
    }

    #[test]
    fn expected_labels_do_not_influence_candidate_evaluation() {
        let mut corpus: Corpus = serde_json::from_str(FIXTURE).expect("fixture parses");
        let case = corpus
            .cases
            .iter_mut()
            .find(|item| item.case_id == "SB-001")
            .unwrap();
        let original = evaluate(case);
        case.expected.disposition = "FORCED_PASS".to_owned();
        case.expected.reason_codes = vec!["NOT_USED_FOR_INFERENCE".to_owned()];
        assert_eq!(original, evaluate(case));
    }

    #[test]
    fn duplicate_case_ids_are_rejected() {
        let mut corpus: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        let first = corpus["cases"][0].clone();
        corpus["cases"].as_array_mut().unwrap().push(first);
        assert!(evaluate_corpus(serde_json::to_vec(&corpus).unwrap().as_slice()).is_err());
    }

    #[test]
    fn held_out_groups_and_hidden_bounds_are_guarded() {
        let mut corpus: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        corpus["split_guard"]["candidate_input_fields"]
            .as_array_mut()
            .unwrap()
            .push(Value::String("reference_bounding_box".to_owned()));
        let bytes = serde_json::to_vec(&corpus).expect("fixture serializes");
        assert!(evaluate_corpus(&bytes).is_err());

        let mut corpus: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        corpus["split_guard"]["evaluation_group_ids"] =
            serde_json::json!(["synthetic-building-train-A"]);
        let bytes = serde_json::to_vec(&corpus).expect("fixture serializes");
        assert!(evaluate_corpus(&bytes).is_err());
    }

    #[test]
    fn evaluation_cases_must_belong_to_declared_heldout_groups() {
        let mut corpus: Value = serde_json::from_str(FIXTURE).expect("fixture parses");
        corpus["cases"][0]["split_group_id"] =
            Value::String("synthetic-building-train-A".to_owned());
        let bytes = serde_json::to_vec(&corpus).expect("fixture serializes");
        assert!(evaluate_corpus(&bytes).is_err());
    }

    #[test]
    fn uncertainty_crossing_limit_never_passes_update() {
        let corpus: Corpus = serde_json::from_str(FIXTURE).expect("fixture parses");
        let case = corpus
            .cases
            .iter()
            .find(|item| item.case_id == "SB-005")
            .unwrap();
        assert_eq!(evaluate(case).0, "REQUEST_ADDITIONAL_OBSERVATION");
    }
}
