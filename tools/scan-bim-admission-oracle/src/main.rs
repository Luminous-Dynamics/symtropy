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

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum EvidenceStatus {
    Qualified,
    Unresolved,
    Invalid,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum RegistrationStabilityStatus {
    Stable,
    DriftSuspected,
    Unknown,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct EvidenceScope {
    survey_product_sha256: String,
    asset_id: String,
    base_model_revision: String,
    frame_id: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct EvidenceReceiptRef {
    status: EvidenceStatus,
    receipt_sha256: Option<String>,
    scope: EvidenceScope,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
struct SurveyEvidenceBinding {
    survey_product_sha256: String,
    asset_id: String,
    base_model_revision: String,
    frame_id: String,
    survey_product_status: EvidenceStatus,
    change_estimate: EvidenceReceiptRef,
    registration_stability: RegistrationStabilityStatus,
    registration_stability_receipt: EvidenceReceiptRef,
    independent_checkpoint: EvidenceReceiptRef,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Candidate {
    asset_id: String,
    base_model_revision: String,
    frame_id: String,
    survey_evidence: SurveyEvidenceBinding,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    counterfactuals: Option<CounterfactualReport>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum CounterfactualKind {
    NuisanceInvariance,
    PhysicalChangeSensitivity,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CounterfactualManifest {
    schema_version: String,
    claim_ceiling: String,
    pairs: Vec<CounterfactualPair>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CounterfactualPair {
    pair_id: String,
    kind: CounterfactualKind,
    control_case_id: String,
    treatment_case_id: String,
    intervention_json_pointers: Vec<String>,
    expected_control_disposition: String,
    expected_control_reason_codes: Vec<String>,
    expected_treatment_disposition: String,
    expected_treatment_reason_codes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CounterfactualPairResult {
    pair_id: String,
    kind: CounterfactualKind,
    control_case_id: String,
    treatment_case_id: String,
    intervention_json_pointers: Vec<String>,
    control_disposition: String,
    control_reason_codes: Vec<String>,
    treatment_disposition: String,
    treatment_reason_codes: Vec<String>,
    matches_expected: bool,
}

#[derive(Debug, Serialize)]
struct CounterfactualReport {
    schema_version: String,
    manifest_sha256: String,
    pair_count: usize,
    matched_pairs: usize,
    mismatched_pairs: usize,
    passed: bool,
    claim_ceiling: String,
    pairs: Vec<CounterfactualPairResult>,
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn receipt_scope_matches(receipt: &EvidenceReceiptRef, survey: &SurveyEvidenceBinding) -> bool {
    let scope = &receipt.scope;
    scope.survey_product_sha256 == survey.survey_product_sha256
        && scope.asset_id == survey.asset_id
        && scope.base_model_revision == survey.base_model_revision
        && scope.frame_id == survey.frame_id
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

    // #1607 owns survey/change-estimate computation. This consumer gate
    // requires the product, estimate, registration and checkpoint receipts to
    // share the exact asset/model/frame/survey-product scope.
    let survey_evidence = &candidate.survey_evidence;
    if survey_evidence.asset_id != candidate.asset_id
        || survey_evidence.base_model_revision != candidate.base_model_revision
        || survey_evidence.frame_id != candidate.frame_id
    {
        return decision(
            "STALE_OR_INVALID_EVIDENCE",
            "SURVEY_EVIDENCE_SCOPE_MISMATCH",
        );
    }
    if !is_sha256(&survey_evidence.survey_product_sha256) {
        return decision("STALE_OR_INVALID_EVIDENCE", "SURVEY_PRODUCT_DIGEST_INVALID");
    }
    match survey_evidence.survey_product_status {
        EvidenceStatus::Qualified => {}
        EvidenceStatus::Unresolved => {
            return decision(
                "REQUEST_ADDITIONAL_OBSERVATION",
                "SURVEY_PRODUCT_UNRESOLVED",
            );
        }
        EvidenceStatus::Invalid => {
            return decision(
                "STALE_OR_INVALID_EVIDENCE",
                "UPSTREAM_SURVEY_PRODUCT_INVALID",
            );
        }
    }

    for (receipt, reason) in [
        (
            &survey_evidence.change_estimate,
            "CHANGE_ESTIMATE_RECEIPT_SCOPE_MISMATCH",
        ),
        (
            &survey_evidence.registration_stability_receipt,
            "REGISTRATION_STABILITY_RECEIPT_SCOPE_MISMATCH",
        ),
        (
            &survey_evidence.independent_checkpoint,
            "INDEPENDENT_CHECKPOINT_RECEIPT_SCOPE_MISMATCH",
        ),
    ] {
        if !receipt_scope_matches(receipt, survey_evidence) {
            return decision("STALE_OR_INVALID_EVIDENCE", reason);
        }
    }

    match survey_evidence.change_estimate.status {
        EvidenceStatus::Qualified => {
            if !survey_evidence
                .change_estimate
                .receipt_sha256
                .as_deref()
                .is_some_and(is_sha256)
            {
                return decision(
                    "STALE_OR_INVALID_EVIDENCE",
                    "CHANGE_ESTIMATE_RECEIPT_MISSING_OR_INVALID",
                );
            }
        }
        EvidenceStatus::Unresolved => {
            return decision(
                "REQUEST_ADDITIONAL_OBSERVATION",
                "SURVEY_CHANGE_ESTIMATE_UNRESOLVED",
            );
        }
        EvidenceStatus::Invalid => {
            return decision(
                "STALE_OR_INVALID_EVIDENCE",
                "SURVEY_CHANGE_ESTIMATE_INVALID",
            );
        }
    }

    match survey_evidence.registration_stability {
        RegistrationStabilityStatus::Stable => {
            match survey_evidence.registration_stability_receipt.status {
                EvidenceStatus::Qualified => {
                    if !survey_evidence
                        .registration_stability_receipt
                        .receipt_sha256
                        .as_deref()
                        .is_some_and(is_sha256)
                    {
                        return decision(
                            "STALE_OR_INVALID_EVIDENCE",
                            "REGISTRATION_STABILITY_RECEIPT_MISSING_OR_INVALID",
                        );
                    }
                }
                EvidenceStatus::Unresolved => {
                    return decision(
                        "REQUEST_ADDITIONAL_OBSERVATION",
                        "REGISTRATION_STABILITY_UNRESOLVED",
                    );
                }
                EvidenceStatus::Invalid => {
                    return decision(
                        "STALE_OR_INVALID_EVIDENCE",
                        "REGISTRATION_STABILITY_EVIDENCE_INVALID",
                    );
                }
            }
        },
        RegistrationStabilityStatus::DriftSuspected => {
            return decision(
                "REQUEST_ADDITIONAL_OBSERVATION",
                "REGISTRATION_DRIFT_OR_COMMON_MODE_SHIFT_UNRESOLVED",
            );
        }
        RegistrationStabilityStatus::Unknown => {
            return decision(
                "REQUEST_ADDITIONAL_OBSERVATION",
                "REGISTRATION_STABILITY_UNRESOLVED",
            );
        }
    }

    match survey_evidence.independent_checkpoint.status {
        EvidenceStatus::Qualified => {
            if !survey_evidence
                .independent_checkpoint
                .receipt_sha256
                .as_deref()
                .is_some_and(is_sha256)
            {
                return decision(
                    "STALE_OR_INVALID_EVIDENCE",
                    "INDEPENDENT_CHECKPOINT_EVIDENCE_MISSING_OR_INVALID",
                );
            }
        }
        EvidenceStatus::Unresolved => {
            return decision(
                "REQUEST_ADDITIONAL_OBSERVATION",
                "INDEPENDENT_CHECKPOINT_EVIDENCE_UNRESOLVED",
            );
        }
        EvidenceStatus::Invalid => {
            return decision(
                "STALE_OR_INVALID_EVIDENCE",
                "INDEPENDENT_CHECKPOINT_EVIDENCE_INVALID",
            );
        }
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
        if edge.from == edge.to
            || !requirements.contains_key(edge.from.as_str())
            || !requirements.contains_key(edge.to.as_str())
        {
            return decision("REJECTED_TOPOLOGY", "TOPOLOGY_EDGE_HAS_INVALID_ENDPOINT");
        }
        if edge.relation != "hosts_opening" {
            return decision("REJECTED_TOPOLOGY", "TOPOLOGY_RELATION_UNSUPPORTED");
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
        }
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

    results.sort_by(|left, right| left.case_id.cmp(&right.case_id));
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
        counterfactuals: None,
    })
}

fn json_pointer_tokens(pointer: &str) -> Result<Vec<String>, String> {
    if !pointer.starts_with('/') || pointer.ends_with('/') {
        return Err(format!("invalid JSON pointer: {pointer}"));
    }
    let mut tokens = Vec::new();
    for raw in pointer[1..].split('/') {
        let mut decoded = String::new();
        let mut chars = raw.chars();
        while let Some(character) = chars.next() {
            if character == '~' {
                match chars.next() {
                    Some('0') => decoded.push('~'),
                    Some('1') => decoded.push('/'),
                    _ => return Err(format!("invalid JSON pointer escape: {pointer}")),
                }
            } else {
                decoded.push(character);
            }
        }
        tokens.push(decoded);
    }
    if tokens.is_empty() || tokens.iter().any(String::is_empty) {
        return Err(format!("empty JSON pointer token: {pointer}"));
    }
    Ok(tokens)
}

fn remove_json_pointer(
    value: &mut serde_json::Value,
    pointer: &str,
) -> Result<serde_json::Value, String> {
    let tokens = json_pointer_tokens(pointer)?;
    let (last, parents) = tokens
        .split_last()
        .ok_or_else(|| format!("empty JSON pointer: {pointer}"))?;
    let mut parent = value;
    for token in parents {
        parent = match parent {
            serde_json::Value::Object(map) => map
                .get_mut(token)
                .ok_or_else(|| format!("JSON pointer does not exist: {pointer}"))?,
            serde_json::Value::Array(items) => {
                let index = token
                    .parse::<usize>()
                    .map_err(|_| format!("invalid array index in JSON pointer: {pointer}"))?;
                items
                    .get_mut(index)
                    .ok_or_else(|| format!("JSON pointer does not exist: {pointer}"))?
            }
            _ => return Err(format!("JSON pointer traverses a scalar: {pointer}")),
        };
    }
    match parent {
        serde_json::Value::Object(map) => map
            .remove(last)
            .ok_or_else(|| format!("JSON pointer does not exist: {pointer}")),
        serde_json::Value::Array(items) => {
            let index = last
                .parse::<usize>()
                .map_err(|_| format!("invalid final array index in JSON pointer: {pointer}"))?;
            if index.to_string() != *last || index >= items.len() {
                return Err(format!("invalid or missing final array index: {pointer}"));
            }
            Ok(items.remove(index))
        }
        _ => Err(format!(
            "counterfactual intervention must target a JSON object field or array element: {pointer}"
        )),
    }
}

fn evaluate_counterfactual_manifest(
    corpus_bytes: &[u8],
    manifest_bytes: &[u8],
) -> Result<CounterfactualReport, Box<dyn Error>> {
    let corpus: Corpus = serde_json::from_slice(corpus_bytes)?;
    let corpus_value: serde_json::Value = serde_json::from_slice(corpus_bytes)?;
    let manifest: CounterfactualManifest = serde_json::from_slice(manifest_bytes)?;
    if manifest.schema_version != "0.1" || manifest.claim_ceiling.trim().is_empty() {
        return Err("unsupported or incomplete counterfactual manifest identity".into());
    }
    if manifest.pairs.is_empty() || manifest.pairs.len() > 256 {
        return Err("counterfactual manifest must contain 1..=256 pairs".into());
    }

    let raw_cases: BTreeMap<&str, &serde_json::Value> = corpus_value
        .get("cases")
        .and_then(serde_json::Value::as_array)
        .ok_or("corpus cases array missing")?
        .iter()
        .map(|case| {
            let id = case
                .get("case_id")
                .and_then(serde_json::Value::as_str)
                .ok_or("case_id missing in raw corpus")?;
            Ok((id, case))
        })
        .collect::<Result<_, Box<dyn Error>>>()?;
    let typed_cases: BTreeMap<&str, &Case> = corpus
        .cases
        .iter()
        .map(|case| (case.case_id.as_str(), case))
        .collect();

    let mut pair_ids: BTreeSet<String> = BTreeSet::new();
    let mut treatment_ids: BTreeSet<String> = BTreeSet::new();
    let mut pairs = manifest.pairs;
    pairs.sort_by(|left, right| left.pair_id.cmp(&right.pair_id));
    let mut results = Vec::with_capacity(pairs.len());

    for pair in pairs {
        if pair.pair_id.trim().is_empty() || !pair_ids.insert(pair.pair_id.clone()) {
            return Err(format!(
                "empty or duplicate counterfactual pair id: {}",
                pair.pair_id
            )
            .into());
        }
        if pair.control_case_id == pair.treatment_case_id
            || !treatment_ids.insert(pair.treatment_case_id.clone())
        {
            return Err(format!(
                "counterfactual case reused or self-paired: {}",
                pair.pair_id
            )
            .into());
        }
        if pair.intervention_json_pointers.is_empty() {
            return Err(format!(
                "counterfactual pair has no declared intervention: {}",
                pair.pair_id
            )
            .into());
        }
        if pair.kind == CounterfactualKind::NuisanceInvariance
            && pair.intervention_json_pointers.len() != 1
        {
            return Err(format!(
                "nuisance pair must declare exactly one intervention: {}",
                pair.pair_id
            )
            .into());
        }
        let mut unique_pointers = BTreeSet::new();
        for pointer in &pair.intervention_json_pointers {
            json_pointer_tokens(pointer)?;
            if !unique_pointers.insert(pointer.as_str()) {
                return Err(
                    format!("duplicate intervention pointer in pair: {}", pair.pair_id).into(),
                );
            }
        }

        let control_raw = raw_cases
            .get(pair.control_case_id.as_str())
            .ok_or_else(|| format!("unknown control case in pair {}", pair.pair_id))?;
        let treatment_raw = raw_cases
            .get(pair.treatment_case_id.as_str())
            .ok_or_else(|| format!("unknown treatment case in pair {}", pair.pair_id))?;
        let control = typed_cases
            .get(pair.control_case_id.as_str())
            .ok_or_else(|| format!("typed control case missing in pair {}", pair.pair_id))?;
        let treatment = typed_cases
            .get(pair.treatment_case_id.as_str())
            .ok_or_else(|| format!("typed treatment case missing in pair {}", pair.pair_id))?;

        if control.split_group_id != treatment.split_group_id {
            return Err(
                format!("counterfactual pair crosses split groups: {}", pair.pair_id).into(),
            );
        }
        if control_raw.get("reference") != treatment_raw.get("reference") {
            return Err(format!(
                "counterfactual pair changes reference world: {}",
                pair.pair_id
            )
            .into());
        }

        let mut control_candidate = control_raw
            .get("candidate")
            .cloned()
            .ok_or_else(|| format!("control candidate missing in pair {}", pair.pair_id))?;
        let mut treatment_candidate = treatment_raw
            .get("candidate")
            .cloned()
            .ok_or_else(|| format!("treatment candidate missing in pair {}", pair.pair_id))?;
        for pointer in &pair.intervention_json_pointers {
            let control_value = remove_json_pointer(&mut control_candidate, pointer)?;
            let treatment_value = remove_json_pointer(&mut treatment_candidate, pointer)?;
            let declared_physical_target_set = pair.kind
                == CounterfactualKind::PhysicalChangeSensitivity
                && pointer == "/delta/target_ids"
                && control_value.is_array()
                && treatment_value.is_array();
            if control_value.is_object()
                || treatment_value.is_object()
                || ((control_value.is_array() || treatment_value.is_array())
                    && !declared_physical_target_set)
            {
                return Err(format!(
                    "intervention must identify a scalar/nullable leaf; only the physical-change target_ids field may be an explicitly declared array: {} at {}",
                    pair.pair_id, pointer
                )
                .into());
            }
            if control_value == treatment_value {
                return Err(format!(
                    "declared intervention is not different between pair cases: {} at {}",
                    pair.pair_id, pointer
                )
                .into());
            }
        }
        if control_candidate != treatment_candidate {
            return Err(format!(
                "pair candidates differ outside declared intervention pointer(s): {}",
                pair.pair_id
            )
            .into());
        }

        let (control_disposition, control_reason_codes) = evaluate(control);
        let (treatment_disposition, treatment_reason_codes) = evaluate(treatment);
        match pair.kind {
            CounterfactualKind::NuisanceInvariance => {
                if !matches!(
                    pair.expected_control_disposition.as_str(),
                    "ADMIT_CANDIDATE_DELTA" | "NO_CHANGE_SUPPORTED"
                ) {
                    return Err(format!(
                        "nuisance control must be either a qualified-change or no-change control: {}",
                        pair.pair_id
                    )
                    .into());
                }
                if matches!(
                    pair.expected_treatment_disposition.as_str(),
                    "ADMIT_CANDIDATE_DELTA" | "REAL_CHANGE_CANDIDATE"
                ) {
                    return Err(format!(
                        "nuisance treatment may not expect physical-change admission: {}",
                        pair.pair_id
                    )
                    .into());
                }
                if matches!(
                    treatment_disposition.as_str(),
                    "ADMIT_CANDIDATE_DELTA" | "REAL_CHANGE_CANDIDATE"
                ) {
                    return Err(format!(
                        "nuisance-only treatment yielded a physical-change admission: {}",
                        pair.pair_id
                    )
                    .into());
                }
            }
            CounterfactualKind::PhysicalChangeSensitivity => {
                if pair.expected_control_disposition != "NO_CHANGE_SUPPORTED"
                    || pair.expected_treatment_disposition != "ADMIT_CANDIDATE_DELTA"
                {
                    return Err(format!(
                        "physical-change pair requires no-change control and qualified change treatment: {}",
                        pair.pair_id
                    )
                    .into());
                }
                if control.survey_evidence.survey_product_status != EvidenceStatus::Qualified
                    || treatment.survey_evidence.survey_product_status != EvidenceStatus::Qualified
                    || control.survey_evidence.change_estimate.status != EvidenceStatus::Qualified
                    || treatment.survey_evidence.change_estimate.status != EvidenceStatus::Qualified
                    || control.survey_evidence.registration_stability
                        != RegistrationStabilityStatus::Stable
                    || treatment.survey_evidence.registration_stability
                        != RegistrationStabilityStatus::Stable
                {
                    return Err(format!(
                        "physical-change sensitivity pair requires qualified survey/change/registration evidence: {}",
                        pair.pair_id
                    )
                    .into());
                }
            }
        }

        let matches_expected = control_disposition == pair.expected_control_disposition
            && control_reason_codes == pair.expected_control_reason_codes
            && treatment_disposition == pair.expected_treatment_disposition
            && treatment_reason_codes == pair.expected_treatment_reason_codes
            && control_disposition == control.expected.disposition
            && control_reason_codes == control.expected.reason_codes
            && treatment_disposition == treatment.expected.disposition
            && treatment_reason_codes == treatment.expected.reason_codes;

        results.push(CounterfactualPairResult {
            pair_id: pair.pair_id,
            kind: pair.kind,
            control_case_id: pair.control_case_id,
            treatment_case_id: pair.treatment_case_id,
            intervention_json_pointers: pair.intervention_json_pointers,
            control_disposition,
            control_reason_codes,
            treatment_disposition,
            treatment_reason_codes,
            matches_expected,
        });
    }

    let matched_pairs = results.iter().filter(|pair| pair.matches_expected).count();
    let mismatched_pairs = results.len() - matched_pairs;
    Ok(CounterfactualReport {
        schema_version: manifest.schema_version,
        manifest_sha256: sha256_hex(manifest_bytes),
        pair_count: results.len(),
        matched_pairs,
        mismatched_pairs,
        passed: mismatched_pairs == 0,
        claim_ceiling: manifest.claim_ceiling,
        pairs: results,
    })
}

fn parse_args() -> Result<(PathBuf, PathBuf), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let mut corpus = PathBuf::from("docs/research/built-asset/scan-bim/v0.1/corpus.json");
    let mut counterfactuals =
        PathBuf::from("docs/research/built-asset/scan-bim/v0.1/counterfactual-pairs.json");
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--corpus" => corpus = args.next().ok_or("--corpus requires a path")?.into(),
            "--counterfactuals" => {
                counterfactuals = args
                    .next()
                    .ok_or("--counterfactuals requires a path")?
                    .into()
            }
            "--help" | "-h" => {
                println!("scan-bim-admission-oracle [--corpus PATH] [--counterfactuals PATH]");
                std::process::exit(0);
            }
            unknown => return Err(format!("unknown argument: {unknown}").into()),
        }
    }
    Ok((corpus, counterfactuals))
}

fn run() -> Result<bool, Box<dyn Error>> {
    let (corpus_path, counterfactual_path) = parse_args()?;
    let corpus_bytes = fs::read(corpus_path)?;
    let counterfactual_bytes = fs::read(counterfactual_path)?;
    let mut report = evaluate_corpus(&corpus_bytes)?;
    let counterfactual_report =
        evaluate_counterfactual_manifest(&corpus_bytes, &counterfactual_bytes)?;
    report.passed = report.passed && counterfactual_report.passed;
    report.counterfactuals = Some(counterfactual_report);
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
    const COUNTERFACTUALS: &str =
        include_str!("../../../docs/research/built-asset/scan-bim/v0.1/counterfactual-pairs.json");

    #[test]
    fn frozen_corpus_matches_dispositions_and_reason_codes() {
        let report = evaluate_corpus(FIXTURE.as_bytes()).expect("fixture parses");
        assert_eq!(report.case_count, 40);
        assert_eq!(report.matched_expected_cases, 40);
        assert_eq!(report.mismatched_cases, 0);
        assert!(report.passed);
    }

    #[test]
    fn counterfactual_manifest_matches_all_declared_pairs() {
        let report =
            evaluate_counterfactual_manifest(FIXTURE.as_bytes(), COUNTERFACTUALS.as_bytes())
                .expect("counterfactual fixture validates");
        assert_eq!(report.pair_count, 14);
        assert_eq!(report.matched_pairs, 14);
        assert_eq!(report.mismatched_pairs, 0);
        assert!(report.passed);
    }

    #[test]
    fn corpus_case_order_does_not_change_semantic_reports() {
        let original_cases =
            evaluate_corpus(FIXTURE.as_bytes()).expect("original corpus evaluates");
        let original_pairs =
            evaluate_counterfactual_manifest(FIXTURE.as_bytes(), COUNTERFACTUALS.as_bytes())
                .expect("original pairs evaluate");

        let mut corpus: serde_json::Value = serde_json::from_str(FIXTURE).expect("corpus parses");
        corpus["cases"].as_array_mut().unwrap().reverse();
        let reversed_bytes = serde_json::to_vec(&corpus).expect("corpus serializes");
        let reversed_cases = evaluate_corpus(&reversed_bytes).expect("reversed corpus evaluates");
        let reversed_pairs =
            evaluate_counterfactual_manifest(&reversed_bytes, COUNTERFACTUALS.as_bytes())
                .expect("reversed pairs evaluate");

        assert_eq!(
            serde_json::to_value(original_cases.cases).unwrap(),
            serde_json::to_value(reversed_cases.cases).unwrap()
        );
        assert_eq!(
            serde_json::to_value(original_pairs.pairs).unwrap(),
            serde_json::to_value(reversed_pairs.pairs).unwrap()
        );
        assert_eq!(
            original_cases.matched_expected_cases,
            reversed_cases.matched_expected_cases
        );
        assert_eq!(original_pairs.matched_pairs, reversed_pairs.matched_pairs);
        assert_ne!(original_cases.corpus_sha256, reversed_cases.corpus_sha256);
    }

    #[test]
    fn counterfactual_pair_order_does_not_change_report() {
        let original =
            evaluate_counterfactual_manifest(FIXTURE.as_bytes(), COUNTERFACTUALS.as_bytes())
                .expect("counterfactual fixture validates");
        let mut manifest: serde_json::Value =
            serde_json::from_str(COUNTERFACTUALS).expect("manifest parses");
        manifest["pairs"].as_array_mut().unwrap().reverse();
        let reversed_bytes = serde_json::to_vec(&manifest).expect("manifest serializes");
        let reversed = evaluate_counterfactual_manifest(FIXTURE.as_bytes(), &reversed_bytes)
            .expect("reversed pair order validates");
        let original_value = serde_json::to_value(original).unwrap();
        let reversed_value = serde_json::to_value(reversed).unwrap();
        assert_eq!(original_value["pairs"], reversed_value["pairs"]);
        assert_eq!(original_value["pair_count"], reversed_value["pair_count"]);
        assert_eq!(original_value["passed"], reversed_value["passed"]);
    }

    #[test]
    fn no_change_nuisance_pairs_never_admit_physical_change() {
        let report =
            evaluate_counterfactual_manifest(FIXTURE.as_bytes(), COUNTERFACTUALS.as_bytes())
                .expect("no-change nuisance pairs validate");
        for pair_id in [
            "CF-012-no-change-registration-drift",
            "CF-013-no-change-estimate-unresolved",
            "CF-014-no-change-checkpoint-unresolved",
        ] {
            let pair = report
                .pairs
                .iter()
                .find(|pair| pair.pair_id == pair_id)
                .expect("no-change nuisance pair exists");
            assert_eq!(pair.control_disposition, "NO_CHANGE_SUPPORTED");
            assert_eq!(pair.treatment_disposition, "REQUEST_ADDITIONAL_OBSERVATION");
            assert_ne!(pair.treatment_disposition, "ADMIT_CANDIDATE_DELTA");
            assert_ne!(pair.treatment_disposition, "REAL_CHANGE_CANDIDATE");
        }
    }

    #[test]
    fn physical_change_pair_rejects_unapproved_array_subtrees() {
        let mut manifest: serde_json::Value =
            serde_json::from_str(COUNTERFACTUALS).expect("manifest parses");
        let pair = manifest["pairs"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|pair| pair["pair_id"] == "CF-007-qualified-physical-change-sensitivity")
            .expect("physical-change pair exists");
        pair["intervention_json_pointers"] = serde_json::json!(["/elements"]);
        let bytes = serde_json::to_vec(&manifest).expect("manifest serializes");
        assert!(evaluate_counterfactual_manifest(FIXTURE.as_bytes(), &bytes).is_err());
    }

    #[test]
    fn nuisance_manifest_rejects_array_subtree_interventions() {
        let mut manifest: serde_json::Value =
            serde_json::from_str(COUNTERFACTUALS).expect("manifest parses");
        let pair = manifest["pairs"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|pair| pair["pair_id"] == "CF-001-registration-drift")
            .expect("nuisance pair exists");
        pair["intervention_json_pointers"] = serde_json::json!(["/elements"]);
        let bytes = serde_json::to_vec(&manifest).expect("manifest serializes");
        assert!(evaluate_counterfactual_manifest(FIXTURE.as_bytes(), &bytes).is_err());
    }

    #[test]
    fn final_array_element_json_pointer_is_supported() {
        let mut value = serde_json::json!({"delta": {"target_ids": ["DOOR-A"]}});
        let removed = remove_json_pointer(&mut value, "/delta/target_ids/0")
            .expect("array element pointer removes the declared leaf");
        assert_eq!(removed, serde_json::Value::String("DOOR-A".to_owned()));
        assert_eq!(value, serde_json::json!({"delta": {"target_ids": []}}));
    }

    #[test]
    fn counterfactual_manifest_rejects_subtree_interventions() {
        let mut manifest: serde_json::Value =
            serde_json::from_str(COUNTERFACTUALS).expect("manifest parses");
        let pair = manifest["pairs"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|pair| pair["pair_id"] == "CF-002-change-estimate-unresolved")
            .expect("change-estimate pair exists");
        pair["intervention_json_pointers"] =
            serde_json::json!(["/survey_evidence/change_estimate"]);
        let bytes = serde_json::to_vec(&manifest).expect("manifest serializes");
        assert!(evaluate_counterfactual_manifest(FIXTURE.as_bytes(), &bytes).is_err());
    }

    #[test]
    fn one_factor_pair_rejects_undeclared_candidate_mutation() {
        let mut corpus: serde_json::Value =
            serde_json::from_str(FIXTURE).expect("corpus parses");
        let case = corpus["cases"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|case| case["case_id"] == "SB-026")
            .expect("registration drift fixture");
        case["candidate"]["elements"][1]["deviation_mm"] = serde_json::Value::from(23.0);
        let bytes = serde_json::to_vec(&corpus).expect("corpus serializes");
        assert!(evaluate_counterfactual_manifest(&bytes, COUNTERFACTUALS.as_bytes()).is_err());
    }

    #[test]
    fn nuisance_pair_cannot_claim_physical_change_admission() {
        let mut manifest: serde_json::Value =
            serde_json::from_str(COUNTERFACTUALS).expect("manifest parses");
        manifest["pairs"][0]["expected_treatment_disposition"] =
            serde_json::Value::String("ADMIT_CANDIDATE_DELTA".to_owned());
        let bytes = serde_json::to_vec(&manifest).expect("manifest serializes");
        assert!(evaluate_counterfactual_manifest(FIXTURE.as_bytes(), &bytes).is_err());
    }

    #[test]
    fn counterfactual_expected_labels_only_affect_scoring() {
        let baseline =
            evaluate_counterfactual_manifest(FIXTURE.as_bytes(), COUNTERFACTUALS.as_bytes())
                .expect("baseline manifest validates");
        let baseline_pair = baseline
            .pairs
            .iter()
            .find(|pair| pair.pair_id == "CF-001-registration-drift")
            .expect("registration pair exists");

        let mut manifest: serde_json::Value =
            serde_json::from_str(COUNTERFACTUALS).expect("manifest parses");
        let pair = manifest["pairs"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|pair| pair["pair_id"] == "CF-001-registration-drift")
            .expect("registration pair exists");
        pair["expected_treatment_reason_codes"] =
            serde_json::json!(["DELIBERATELY_WRONG_EXPECTED_REASON"]);
        let bytes = serde_json::to_vec(&manifest).expect("manifest serializes");
        let altered = evaluate_counterfactual_manifest(FIXTURE.as_bytes(), &bytes)
            .expect("bad expected label is a score mismatch, not inference input");
        let altered_pair = altered
            .pairs
            .iter()
            .find(|pair| pair.pair_id == "CF-001-registration-drift")
            .expect("registration pair remains present");

        assert_eq!(
            baseline_pair.control_disposition,
            altered_pair.control_disposition
        );
        assert_eq!(
            baseline_pair.treatment_disposition,
            altered_pair.treatment_disposition
        );
        assert!(!altered_pair.matches_expected);
        assert!(!altered.passed);
    }

    #[test]
    fn duplicate_counterfactual_pair_ids_are_rejected() {
        let mut manifest: serde_json::Value =
            serde_json::from_str(COUNTERFACTUALS).expect("manifest parses");
        let first = manifest["pairs"][0].clone();
        manifest["pairs"].as_array_mut().unwrap().push(first);
        let bytes = serde_json::to_vec(&manifest).expect("manifest serializes");
        assert!(evaluate_counterfactual_manifest(FIXTURE.as_bytes(), &bytes).is_err());
    }

    #[test]
    fn nuisance_evidence_does_not_invent_change_in_no_change_world() {
        let corpus: Corpus = serde_json::from_str(FIXTURE).expect("fixture parses");
        for (case_id, reason) in [
            (
                "SB-038",
                "REGISTRATION_DRIFT_OR_COMMON_MODE_SHIFT_UNRESOLVED",
            ),
            ("SB-039", "SURVEY_CHANGE_ESTIMATE_UNRESOLVED"),
            ("SB-040", "INDEPENDENT_CHECKPOINT_EVIDENCE_UNRESOLVED"),
        ] {
            let case = corpus
                .cases
                .iter()
                .find(|item| item.case_id == case_id)
                .expect("no-change nuisance fixture");
            assert_eq!(evaluate(case).0, "REQUEST_ADDITIONAL_OBSERVATION");
            assert_eq!(evaluate(case).1, vec![reason.to_owned()]);
            assert_ne!(evaluate(case).0, "ADMIT_CANDIDATE_DELTA");
            assert_ne!(evaluate(case).0, "REAL_CHANGE_CANDIDATE");
        }
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
            survey_evidence: case.candidate.survey_evidence.clone(),
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
    fn common_mode_registration_drift_blocks_physical_change_admission() {
        let corpus: Corpus = serde_json::from_str(FIXTURE).expect("fixture parses");
        let case = corpus
            .cases
            .iter()
            .find(|item| item.case_id == "SB-026")
            .expect("common-mode drift fixture");
        assert_eq!(evaluate(case).0, "REQUEST_ADDITIONAL_OBSERVATION");
        assert_eq!(
            evaluate(case).1,
            vec!["REGISTRATION_DRIFT_OR_COMMON_MODE_SHIFT_UNRESOLVED".to_owned()]
        );
    }

    #[test]
    fn missing_independent_checkpoint_receipt_blocks_change_admission() {
        let corpus: Corpus = serde_json::from_str(FIXTURE).expect("fixture parses");
        let case = corpus
            .cases
            .iter()
            .find(|item| item.case_id == "SB-028")
            .expect("missing checkpoint fixture");
        assert_eq!(evaluate(case).0, "STALE_OR_INVALID_EVIDENCE");
        assert_eq!(
            evaluate(case).1,
            vec!["INDEPENDENT_CHECKPOINT_EVIDENCE_MISSING_OR_INVALID".to_owned()]
        );
    }

    #[test]
    fn survey_receipts_must_bind_to_exact_asset_revision_and_frame() {
        let corpus: Corpus = serde_json::from_str(FIXTURE).expect("fixture parses");
        let case = corpus
            .cases
            .iter()
            .find(|item| item.case_id == "SB-029")
            .expect("survey scope mismatch fixture");
        assert_eq!(evaluate(case).0, "STALE_OR_INVALID_EVIDENCE");
        assert_eq!(
            evaluate(case).1,
            vec!["SURVEY_EVIDENCE_SCOPE_MISMATCH".to_owned()]
        );
    }

    #[test]
    fn qualified_upstream_receipts_require_valid_digests() {
        let corpus: Corpus = serde_json::from_str(FIXTURE).expect("fixture parses");
        let change = corpus
            .cases
            .iter()
            .find(|item| item.case_id == "SB-031")
            .expect("missing change receipt fixture");
        assert_eq!(
            evaluate(change).1,
            vec!["CHANGE_ESTIMATE_RECEIPT_MISSING_OR_INVALID".to_owned()]
        );

        let registration = corpus
            .cases
            .iter()
            .find(|item| item.case_id == "SB-032")
            .expect("missing registration receipt fixture");
        assert_eq!(
            evaluate(registration).1,
            vec!["REGISTRATION_STABILITY_RECEIPT_MISSING_OR_INVALID".to_owned()]
        );
    }

    #[test]
    fn unresolved_upstream_products_or_checkpoints_cannot_pass() {
        let corpus: Corpus = serde_json::from_str(FIXTURE).expect("fixture parses");
        let invalid_survey = corpus
            .cases
            .iter()
            .find(|item| item.case_id == "SB-012")
            .expect("invalid survey product fixture");
        assert_eq!(evaluate(invalid_survey).0, "STALE_OR_INVALID_EVIDENCE");
        assert_eq!(
            evaluate(invalid_survey).1,
            vec!["UPSTREAM_SURVEY_PRODUCT_INVALID".to_owned()]
        );

        for (case_id, reason) in [
            ("SB-033", "INDEPENDENT_CHECKPOINT_EVIDENCE_UNRESOLVED"),
            ("SB-034", "SURVEY_PRODUCT_UNRESOLVED"),
        ] {
            let case = corpus
                .cases
                .iter()
                .find(|item| item.case_id == case_id)
                .expect("unresolved evidence fixture");
            assert_eq!(evaluate(case).0, "REQUEST_ADDITIONAL_OBSERVATION");
            assert_eq!(evaluate(case).1, vec![reason.to_owned()]);
        }
    }

    #[test]
    fn each_upstream_receipt_must_match_the_exact_survey_scope() {
        let corpus: Corpus = serde_json::from_str(FIXTURE).expect("fixture parses");
        for (case_id, reason) in [
            ("SB-035", "CHANGE_ESTIMATE_RECEIPT_SCOPE_MISMATCH"),
            ("SB-036", "REGISTRATION_STABILITY_RECEIPT_SCOPE_MISMATCH"),
            ("SB-037", "INDEPENDENT_CHECKPOINT_RECEIPT_SCOPE_MISMATCH"),
        ] {
            let case = corpus
                .cases
                .iter()
                .find(|item| item.case_id == case_id)
                .expect("receipt scope mismatch fixture");
            assert_eq!(evaluate(case).0, "STALE_OR_INVALID_EVIDENCE");
            assert_eq!(evaluate(case).1, vec![reason.to_owned()]);
        }
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
