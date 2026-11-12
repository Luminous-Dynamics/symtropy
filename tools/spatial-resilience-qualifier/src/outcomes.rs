//! Synthetic paired-candidate admission and Pareto-front qualification.
//!
//! This module tests comparison semantics only. It does not simulate a physical building.

use crate::sha256_hex;
use jsonschema::validator_for;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const EMBEDDED_OUTCOME_CORPUS: &str = include_str!(
    "../../../docs/research/spatial-design/SPATIAL_RESILIENCE_PAIRED_OUTCOMES_V0_1.json"
);
pub const EMBEDDED_OUTCOME_SCHEMA: &str = include_str!(
    "../../../docs/research/spatial-design/SPATIAL_RESILIENCE_PAIRED_OUTCOMES_V0_1.schema.json"
);

const EXPECTED_COMPARISON_COUNT: usize = 5;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct OutcomeFailure {
    pub code: String,
    pub path: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CandidateDisposition {
    pub candidate_id: String,
    pub disposition: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ComparisonResult {
    pub comparison_id: String,
    pub status: String,
    pub candidate_dispositions: Vec<CandidateDisposition>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OutcomeSummary {
    pub benchmark_id: Option<String>,
    pub version: Option<String>,
    pub corpus_sha256: String,
    pub schema_sha256: String,
    pub comparisons_seen: usize,
    pub comparisons_passed: usize,
    pub passed: bool,
    pub results: Vec<ComparisonResult>,
    pub failures: Vec<OutcomeFailure>,
}

impl OutcomeSummary {
    fn new(corpus_bytes: &[u8], schema_bytes: &[u8]) -> Self {
        Self {
            benchmark_id: None,
            version: None,
            corpus_sha256: sha256_hex(corpus_bytes),
            schema_sha256: sha256_hex(schema_bytes),
            comparisons_seen: 0,
            comparisons_passed: 0,
            passed: false,
            results: Vec::new(),
            failures: Vec::new(),
        }
    }

    fn fail(&mut self, code: &str, path: impl Into<String>, detail: impl Into<String>) {
        self.failures.push(OutcomeFailure {
            code: code.to_owned(),
            path: path.into(),
            detail: detail.into(),
        });
    }
}

#[derive(Debug, Clone)]
struct Requirement {
    minimum_service_hours: u64,
}

#[derive(Debug, Clone)]
struct Candidate {
    id: String,
    trace_id: String,
    trace_revision: String,
    evidence_state: String,
    hard_constraints_pass: bool,
    uncertainty_width: u64,
    services: BTreeMap<String, u64>,
    energy_units: u64,
    repair_ticks: u64,
    cost_units: u64,
}

fn read_string<'a>(object: &'a serde_json::Map<String, Value>, field: &str) -> Option<&'a str> {
    object.get(field).and_then(Value::as_str)
}

fn read_u64(object: &serde_json::Map<String, Value>, field: &str) -> Option<u64> {
    object.get(field).and_then(Value::as_u64)
}

fn parse_candidate(value: &Value) -> Option<Candidate> {
    let object = value.as_object()?;
    let mut services = BTreeMap::new();
    for service in object.get("services")?.as_array()? {
        let service_object = service.as_object()?;
        let requirement_id = read_string(service_object, "requirement_id")?.to_owned();
        let available_hours = read_u64(service_object, "available_hours")?;
        if services.insert(requirement_id, available_hours).is_some() {
            return None;
        }
    }

    Some(Candidate {
        id: read_string(object, "candidate_id")?.to_owned(),
        trace_id: read_string(object, "trace_id")?.to_owned(),
        trace_revision: read_string(object, "trace_revision")?.to_owned(),
        evidence_state: read_string(object, "evidence_state")?.to_owned(),
        hard_constraints_pass: object.get("hard_constraints_pass")?.as_bool()?,
        uncertainty_width: read_u64(object, "uncertainty_width")?,
        services,
        energy_units: read_u64(object, "energy_units")?,
        repair_ticks: read_u64(object, "repair_ticks")?,
        cost_units: read_u64(object, "cost_units")?,
    })
}

fn dominates(left: &Candidate, right: &Candidate) -> bool {
    let service_no_worse = left.services.iter().all(|(requirement_id, left_hours)| {
        right
            .services
            .get(requirement_id)
            .is_some_and(|right_hours| left_hours >= right_hours)
    });
    let soft_no_worse = left.energy_units <= right.energy_units
        && left.repair_ticks <= right.repair_ticks
        && left.cost_units <= right.cost_units;
    if !service_no_worse || !soft_no_worse {
        return false;
    }

    left.services.iter().any(|(requirement_id, left_hours)| {
        right
            .services
            .get(requirement_id)
            .is_some_and(|right_hours| left_hours > right_hours)
    }) || left.energy_units < right.energy_units
        || left.repair_ticks < right.repair_ticks
        || left.cost_units < right.cost_units
}

fn preliminary_disposition(
    candidate: &Candidate,
    requirements: &BTreeMap<String, Requirement>,
    uncertainty_limit: u64,
) -> Option<&'static str> {
    if candidate.evidence_state != "synthetic_fixture" {
        return Some("blocked_evidence");
    }
    if !candidate.hard_constraints_pass {
        return Some("excluded_hard_constraint");
    }

    let floors_pass = requirements.iter().all(|(id, requirement)| {
        candidate
            .services
            .get(id)
            .is_some_and(|available| *available >= requirement.minimum_service_hours)
    });
    if !floors_pass {
        return Some("excluded_service_floor");
    }
    if candidate.uncertainty_width > uncertainty_limit {
        return Some("excluded_uncertainty");
    }
    None
}

fn compare_one(
    comparison: &Value,
    summary: &mut OutcomeSummary,
    comparison_index: usize,
) -> Option<ComparisonResult> {
    let path = format!("$.comparisons[{comparison_index}]");
    let object = comparison.as_object()?;
    let comparison_id = read_string(object, "id")?.to_owned();
    let expected_status = read_string(object, "expected_status")?;
    let trace_id = read_string(object, "trace_id")?;
    let trace_revision = read_string(object, "trace_revision")?;
    let uncertainty_limit = read_u64(object, "uncertainty_limit")?;

    let mut requirements = BTreeMap::new();
    for (requirement_index, requirement_value) in
        object.get("requirements")?.as_array()?.iter().enumerate()
    {
        let requirement_object = requirement_value.as_object()?;
        let requirement_id = read_string(requirement_object, "requirement_id")?.to_owned();
        let minimum_service_hours = read_u64(requirement_object, "minimum_service_hours")?;
        if requirements
            .insert(
                requirement_id.clone(),
                Requirement {
                    minimum_service_hours,
                },
            )
            .is_some()
        {
            summary.fail(
                "OUTCOME_REQUIREMENT_ID_DUPLICATE",
                format!("{path}.requirements[{requirement_index}]"),
                format!("Requirement ID {requirement_id:?} is duplicated."),
            );
        }
    }
    if requirements.is_empty() {
        summary.fail(
            "OUTCOME_REQUIREMENTS_EMPTY",
            format!("{path}.requirements"),
            "At least one resident/task service floor is required.",
        );
    }

    let candidates_value = object.get("candidates")?.as_array()?;
    let mut candidates = Vec::with_capacity(candidates_value.len());
    let mut candidate_ids = BTreeSet::new();
    let mut service_sets_valid = true;
    let required_service_ids: BTreeSet<String> = requirements.keys().cloned().collect();

    for (candidate_index, candidate_value) in candidates_value.iter().enumerate() {
        let candidate_path = format!("{path}.candidates[{candidate_index}]");
        let Some(candidate) = parse_candidate(candidate_value) else {
            summary.fail(
                "OUTCOME_CANDIDATE_INVALID",
                &candidate_path,
                "Candidate or service vector could not be parsed.",
            );
            continue;
        };
        if !candidate_ids.insert(candidate.id.clone()) {
            summary.fail(
                "OUTCOME_CANDIDATE_ID_DUPLICATE",
                format!("{candidate_path}.candidate_id"),
                format!("Candidate ID {:?} is duplicated.", candidate.id),
            );
        }
        let service_ids: BTreeSet<String> = candidate.services.keys().cloned().collect();
        if service_ids != required_service_ids {
            service_sets_valid = false;
            summary.fail(
                "OUTCOME_SERVICE_VECTOR_MISMATCH",
                format!("{candidate_path}.services"),
                "Candidate must report each declared resident/task service exactly once.",
            );
        }
        candidates.push(candidate);
    }

    if candidates.len() != candidates_value.len() || !service_sets_valid {
        return Some(ComparisonResult {
            comparison_id,
            status: "invalid_input".to_owned(),
            candidate_dispositions: Vec::new(),
        });
    }

    let traces_match = candidates.iter().all(|candidate| {
        candidate.trace_id == trace_id && candidate.trace_revision == trace_revision
    });
    let actual_status = if traces_match {
        "evaluated"
    } else {
        "reject_comparison"
    };

    if actual_status != expected_status {
        summary.fail(
            "OUTCOME_COMPARISON_STATUS_MISMATCH",
            format!("{path}.expected_status"),
            format!("Expected {expected_status:?}, computed {actual_status:?}."),
        );
    }

    let mut dispositions = Vec::with_capacity(candidates.len());
    if !traces_match {
        for candidate in &candidates {
            dispositions.push(CandidateDisposition {
                candidate_id: candidate.id.clone(),
                disposition: "rejected_pair_integrity".to_owned(),
            });
        }
    } else {
        let mut provisional = Vec::with_capacity(candidates.len());
        for candidate in &candidates {
            let preliminary = preliminary_disposition(candidate, &requirements, uncertainty_limit);
            provisional.push(preliminary);
        }

        for (index, candidate) in candidates.iter().enumerate() {
            let disposition = if let Some(reason) = provisional[index] {
                reason.to_owned()
            } else {
                let is_dominated = candidates.iter().enumerate().any(|(other_index, other)| {
                    other_index != index
                        && provisional[other_index].is_none()
                        && dominates(other, candidate)
                });
                if is_dominated {
                    "dominated".to_owned()
                } else {
                    "front".to_owned()
                }
            };
            dispositions.push(CandidateDisposition {
                candidate_id: candidate.id.clone(),
                disposition,
            });
        }
    }

    for (index, (candidate, actual)) in candidates.iter().zip(dispositions.iter()).enumerate() {
        let expected = candidates_value[index]
            .get("expected_disposition")
            .and_then(Value::as_str)
            .unwrap_or("");
        if actual.disposition != expected {
            summary.fail(
                "OUTCOME_DISPOSITION_MISMATCH",
                format!("{path}.candidates[{index}].expected_disposition"),
                format!(
                    "Candidate {:?}: expected {expected:?}, computed {:?}.",
                    candidate.id, actual.disposition
                ),
            );
        }
    }

    Some(ComparisonResult {
        comparison_id,
        status: actual_status.to_owned(),
        candidate_dispositions: dispositions,
    })
}

/// Validate the schema and execute all expected paired-comparison/Pareto cases.
pub fn validate_outcome_fixtures_bytes(corpus_bytes: &[u8], schema_bytes: &[u8]) -> OutcomeSummary {
    let mut summary = OutcomeSummary::new(corpus_bytes, schema_bytes);
    let schema_value: Value = match serde_json::from_slice(schema_bytes) {
        Ok(value) => value,
        Err(error) => {
            summary.fail("OUTCOME_SCHEMA_JSON_INVALID", "$", error.to_string());
            return summary;
        }
    };
    let corpus_value: Value = match serde_json::from_slice(corpus_bytes) {
        Ok(value) => value,
        Err(error) => {
            summary.fail("OUTCOME_CORPUS_JSON_INVALID", "$", error.to_string());
            return summary;
        }
    };

    let validator = match validator_for(&schema_value) {
        Ok(validator) => validator,
        Err(error) => {
            summary.fail("OUTCOME_SCHEMA_COMPILE_FAILED", "$", error.to_string());
            return summary;
        }
    };
    let schema_errors: Vec<_> = validator.iter_errors(&corpus_value).collect();
    if !schema_errors.is_empty() {
        for error in schema_errors {
            summary.fail(
                "OUTCOME_SCHEMA_VALIDATION_FAILED",
                error.instance_path().to_string(),
                error.to_string(),
            );
        }
        return summary;
    }

    let Some(root) = corpus_value.as_object() else {
        summary.fail("OUTCOME_CORPUS_ROOT_INVALID", "$", "Expected an object.");
        return summary;
    };
    summary.benchmark_id = read_string(root, "benchmark_id").map(ToOwned::to_owned);
    summary.version = read_string(root, "version").map(ToOwned::to_owned);
    if summary.benchmark_id.as_deref() != Some("SPATIAL-RESILIENCE-PAIRED-OUTCOMES")
        || summary.version.as_deref() != Some("0.1.0")
    {
        summary.fail(
            "OUTCOME_CORPUS_ID_OR_VERSION_MISMATCH",
            "$",
            "This verifier freezes the paired-outcome corpus version 0.1.0.",
        );
    }

    let Some(comparisons) = root.get("comparisons").and_then(Value::as_array) else {
        summary.fail(
            "OUTCOME_COMPARISONS_MISSING",
            "$.comparisons",
            "Expected an array.",
        );
        return summary;
    };
    summary.comparisons_seen = comparisons.len();
    if comparisons.len() != EXPECTED_COMPARISON_COUNT {
        summary.fail(
            "OUTCOME_COMPARISON_COUNT_MISMATCH",
            "$.comparisons",
            format!("Expected {EXPECTED_COMPARISON_COUNT} comparisons."),
        );
    }

    let mut comparison_ids = BTreeSet::new();
    for (index, comparison) in comparisons.iter().enumerate() {
        let path = format!("$.comparisons[{index}]");
        let case_id = comparison.get("id").and_then(Value::as_str).unwrap_or("");
        if !comparison_ids.insert(case_id.to_owned()) {
            summary.fail(
                "OUTCOME_COMPARISON_ID_DUPLICATE",
                format!("{path}.id"),
                format!("Comparison ID {case_id:?} is duplicated."),
            );
        }
        let expected_id = format!("SPR-P-{:03}", index + 1);
        if case_id != expected_id {
            summary.fail(
                "OUTCOME_COMPARISON_ID_SEQUENCE",
                format!("{path}.id"),
                format!("Expected {expected_id:?}, got {case_id:?}."),
            );
        }

        if let Some(result) = compare_one(comparison, &mut summary, index) {
            let case_failures = summary
                .failures
                .iter()
                .filter(|failure| failure.path.starts_with(&path))
                .count();
            if case_failures == 0 {
                summary.comparisons_passed += 1;
            }
            summary.results.push(result);
        } else {
            summary.fail(
                "OUTCOME_COMPARISON_UNREADABLE",
                &path,
                "Comparison is missing required semantic fields.",
            );
        }
    }

    summary.failures.sort_by(|left, right| {
        (&left.path, &left.code, &left.detail).cmp(&(&right.path, &right.code, &right.detail))
    });
    summary.passed =
        summary.failures.is_empty() && summary.comparisons_passed == summary.comparisons_seen;
    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has_code(summary: &OutcomeSummary, code: &str) -> bool {
        summary.failures.iter().any(|failure| failure.code == code)
    }

    #[test]
    fn embedded_paired_outcomes_match_known_answers() {
        let summary = validate_outcome_fixtures_bytes(
            EMBEDDED_OUTCOME_CORPUS.as_bytes(),
            EMBEDDED_OUTCOME_SCHEMA.as_bytes(),
        );
        assert!(
            summary.passed,
            "unexpected failures: {:?}",
            summary.failures
        );
        assert_eq!(summary.comparisons_seen, 5);
        assert_eq!(summary.comparisons_passed, 5);
        assert_eq!(summary.corpus_sha256.len(), 64);
        assert_eq!(summary.schema_sha256.len(), 64);
    }

    #[test]
    fn broken_individual_service_floor_cannot_be_hidden_by_other_service() {
        let summary = validate_outcome_fixtures_bytes(
            EMBEDDED_OUTCOME_CORPUS.as_bytes(),
            EMBEDDED_OUTCOME_SCHEMA.as_bytes(),
        );
        let result = summary
            .results
            .iter()
            .find(|item| item.comparison_id == "SPR-P-001")
            .unwrap();
        let fragile = result
            .candidate_dispositions
            .iter()
            .find(|item| item.candidate_id == "nominally-efficient")
            .unwrap();
        assert_eq!(fragile.disposition, "excluded_service_floor");
    }

    #[test]
    fn dominated_candidate_does_not_enter_the_front() {
        let summary = validate_outcome_fixtures_bytes(
            EMBEDDED_OUTCOME_CORPUS.as_bytes(),
            EMBEDDED_OUTCOME_SCHEMA.as_bytes(),
        );
        let result = summary
            .results
            .iter()
            .find(|item| item.comparison_id == "SPR-P-001")
            .unwrap();
        let dominated = result
            .candidate_dispositions
            .iter()
            .find(|item| item.candidate_id == "dominated-costly")
            .unwrap();
        assert_eq!(dominated.disposition, "dominated");
    }

    #[test]
    fn trace_revision_mismatch_rejects_the_whole_pair() {
        let summary = validate_outcome_fixtures_bytes(
            EMBEDDED_OUTCOME_CORPUS.as_bytes(),
            EMBEDDED_OUTCOME_SCHEMA.as_bytes(),
        );
        let result = summary
            .results
            .iter()
            .find(|item| item.comparison_id == "SPR-P-003")
            .unwrap();
        assert_eq!(result.status, "reject_comparison");
        assert!(
            result
                .candidate_dispositions
                .iter()
                .all(|item| item.disposition == "rejected_pair_integrity")
        );
    }

    #[test]
    fn equal_objective_vectors_preserve_both_distinct_alternatives() {
        let summary = validate_outcome_fixtures_bytes(
            EMBEDDED_OUTCOME_CORPUS.as_bytes(),
            EMBEDDED_OUTCOME_SCHEMA.as_bytes(),
        );
        let result = summary
            .results
            .iter()
            .find(|item| item.comparison_id == "SPR-P-005")
            .unwrap();
        assert!(
            result
                .candidate_dispositions
                .iter()
                .all(|item| item.disposition == "front")
        );
    }

    #[test]
    fn mutation_of_expected_front_is_detected() {
        let mut corpus: Value =
            serde_json::from_str(EMBEDDED_OUTCOME_CORPUS).expect("valid outcomes");
        corpus["comparisons"][0]["candidates"][0]["expected_disposition"] =
            Value::String("dominated".to_owned());
        let bytes = serde_json::to_vec(&corpus).expect("serialize mutation");
        let summary = validate_outcome_fixtures_bytes(&bytes, EMBEDDED_OUTCOME_SCHEMA.as_bytes());
        assert!(has_code(&summary, "OUTCOME_DISPOSITION_MISMATCH"));
    }

    #[test]
    fn missing_stale_out_of_domain_and_contradictory_evidence_are_blocked() {
        let summary = validate_outcome_fixtures_bytes(
            EMBEDDED_OUTCOME_CORPUS.as_bytes(),
            EMBEDDED_OUTCOME_SCHEMA.as_bytes(),
        );
        let result = summary
            .results
            .iter()
            .find(|item| item.comparison_id == "SPR-P-004")
            .unwrap();
        for candidate_id in [
            "stale-evidence-optimist",
            "missing-evidence-optimist",
            "out-of-domain-evidence",
            "contradictory-evidence",
        ] {
            let candidate = result
                .candidate_dispositions
                .iter()
                .find(|item| item.candidate_id == candidate_id)
                .unwrap();
            assert_eq!(candidate.disposition, "blocked_evidence");
        }
    }

    #[test]
    fn schema_rejects_unknown_candidate_fields() {
        let mut corpus: Value =
            serde_json::from_str(EMBEDDED_OUTCOME_CORPUS).expect("valid outcomes");
        corpus["comparisons"][0]["candidates"][0]["unreviewed_score"] = Value::from(999);
        let bytes = serde_json::to_vec(&corpus).expect("serialize mutation");
        let summary = validate_outcome_fixtures_bytes(&bytes, EMBEDDED_OUTCOME_SCHEMA.as_bytes());
        assert!(has_code(&summary, "OUTCOME_SCHEMA_VALIDATION_FAILED"));
    }

    #[test]
    fn service_vector_must_preserve_every_declared_requirement() {
        let mut corpus: Value =
            serde_json::from_str(EMBEDDED_OUTCOME_CORPUS).expect("valid outcomes");
        corpus["comparisons"][0]["candidates"][0]["services"]
            .as_array_mut()
            .unwrap()
            .pop();
        let bytes = serde_json::to_vec(&corpus).expect("serialize mutation");
        let summary = validate_outcome_fixtures_bytes(&bytes, EMBEDDED_OUTCOME_SCHEMA.as_bytes());
        assert!(has_code(&summary, "OUTCOME_SERVICE_VECTOR_MISMATCH"));
    }
}
