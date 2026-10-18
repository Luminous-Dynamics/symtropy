use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const EMBEDDED_CORPUS: &str =
    include_str!("../../../docs/tech/recovery-scenario-envelope-corpus-v0.1.json");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct Corpus {
    schema_version: String,
    algorithm_id: String,
    status: String,
    claim_ceiling: ClaimCeiling,
    profiles: Vec<ScopeProfile>,
    cases: Vec<ScenarioCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ClaimCeiling {
    real_world_measurements: bool,
    predictive_coverage_claimed: bool,
    safety_qualification_claimed: bool,
    expected_actuation: bool,
    execution_authority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ScopeProfile {
    profile_id: String,
    domain: String,
    profile_version: String,
    criteria_regime: String,
    family_taxonomy_version: String,
    covered_failure_families: Vec<String>,
    required_evidence_tags: Vec<String>,
    max_evidence_age_days: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ScenarioCase {
    case_id: String,
    profile_id: String,
    domain: String,
    profile_version: String,
    criteria_regime: String,
    family_taxonomy_version: String,
    classification_review_status: String,
    #[serde(default)]
    classification_record_ref: Option<String>,
    family_classification: String,
    observed_failure_families: Vec<String>,
    #[serde(default)]
    evidence_age_days: Option<u64>,
    observed_evidence_tags: Vec<String>,
    hard_gate_status: String,
    expected_scope_status: String,
    expected_evidence_status: String,
    expected_disposition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct CaseResult {
    case_id: String,
    profile_id: String,
    scope_status: String,
    evidence_status: String,
    hard_gate_status: String,
    disposition: String,
    expected_scope_status: String,
    expected_evidence_status: String,
    expected_disposition: String,
    expected_outcome_matches: bool,
    review_only: bool,
    actuation_authorized: bool,
}

#[derive(Debug, Serialize)]
struct AuditReceipt {
    result: String,
    schema_version: String,
    algorithm_id: String,
    canonical_corpus_sha256: String,
    source_commit: Option<String>,
    profile_count: usize,
    case_count: usize,
    scope_counts: BTreeMap<String, usize>,
    evidence_counts: BTreeMap<String, usize>,
    disposition_counts: BTreeMap<String, usize>,
    expected_outcomes_match: bool,
    all_review_only: bool,
    all_actuation_disabled: bool,
    case_results: Vec<CaseResult>,
    errors: Vec<String>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn is_hex_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn increment(map: &mut BTreeMap<String, usize>, key: &str) {
    *map.entry(key.to_string()).or_insert(0) += 1;
}

fn validate_string_list(
    values: &[String],
    label: &str,
    require_nonempty: bool,
    errors: &mut Vec<String>,
) {
    if require_nonempty && values.is_empty() {
        errors.push(format!("{label}: LIST_MUST_NOT_BE_EMPTY"));
    }

    let mut seen = BTreeSet::new();
    for value in values {
        if !is_nonempty(value) {
            errors.push(format!("{label}: EMPTY_VALUE"));
        } else if !seen.insert(value) {
            errors.push(format!("{label}: DUPLICATE_VALUE:{value}"));
        }
    }
}

fn validate_corpus(corpus: &Corpus, source_commit: Option<&str>, errors: &mut Vec<String>) {
    if corpus.schema_version != "recovery-scenario-envelope-v0.1" {
        errors.push("SCHEMA_VERSION_UNSUPPORTED".to_string());
    }
    if corpus.algorithm_id != "family-held-out-applicability-envelope-audit-v0.1" {
        errors.push("ALGORITHM_ID_UNSUPPORTED".to_string());
    }
    if corpus.status != "synthetic-boundary-fixtures" {
        errors.push("CORPUS_STATUS_UNEXPECTED".to_string());
    }

    if corpus.claim_ceiling.real_world_measurements {
        errors.push("REAL_WORLD_MEASUREMENTS_MUST_NOT_BE_CLAIMED".to_string());
    }
    if corpus.claim_ceiling.predictive_coverage_claimed {
        errors.push("PREDICTIVE_COVERAGE_MUST_NOT_BE_CLAIMED".to_string());
    }
    if corpus.claim_ceiling.safety_qualification_claimed {
        errors.push("SAFETY_QUALIFICATION_MUST_NOT_BE_CLAIMED".to_string());
    }
    if corpus.claim_ceiling.expected_actuation {
        errors.push("ACTUATION_CEILING_MUST_BE_FALSE".to_string());
    }
    if corpus.claim_ceiling.execution_authority != "NO_ACTUATION_AUTHORIZED" {
        errors.push("EXECUTION_AUTHORITY_INVALID".to_string());
    }
    if let Some(commit) = source_commit {
        if !is_hex_sha(commit) {
            errors.push("SOURCE_COMMIT_INVALID".to_string());
        }
    }

    if corpus.profiles.is_empty() {
        errors.push("PROFILES_MISSING_OR_EMPTY".to_string());
    }
    let mut profile_ids = BTreeSet::new();
    for profile in &corpus.profiles {
        if !is_nonempty(&profile.profile_id) {
            errors.push("PROFILE_ID_EMPTY".to_string());
        } else if !profile_ids.insert(&profile.profile_id) {
            errors.push(format!("DUPLICATE_PROFILE_ID:{}", profile.profile_id));
        }
        for (field, value) in [
            ("domain", profile.domain.as_str()),
            ("profile_version", profile.profile_version.as_str()),
            ("criteria_regime", profile.criteria_regime.as_str()),
            (
                "family_taxonomy_version",
                profile.family_taxonomy_version.as_str(),
            ),
        ] {
            if !is_nonempty(value) {
                errors.push(format!(
                    "PROFILE_{}_EMPTY:{}",
                    field.to_ascii_uppercase(),
                    profile.profile_id
                ));
            }
        }
        validate_string_list(
            &profile.covered_failure_families,
            &format!("PROFILE_FAMILIES:{}", profile.profile_id),
            true,
            errors,
        );
        validate_string_list(
            &profile.required_evidence_tags,
            &format!("PROFILE_EVIDENCE:{}", profile.profile_id),
            true,
            errors,
        );
    }

    if corpus.cases.is_empty() {
        errors.push("CASES_MISSING_OR_EMPTY".to_string());
    }
    let mut case_ids = BTreeSet::new();
    for case in &corpus.cases {
        if !is_nonempty(&case.case_id) {
            errors.push("CASE_ID_EMPTY".to_string());
        } else if !case_ids.insert(&case.case_id) {
            errors.push(format!("DUPLICATE_CASE_ID:{}", case.case_id));
        }
        for (field, value) in [
            ("profile_id", case.profile_id.as_str()),
            ("domain", case.domain.as_str()),
            ("profile_version", case.profile_version.as_str()),
            ("criteria_regime", case.criteria_regime.as_str()),
            (
                "family_taxonomy_version",
                case.family_taxonomy_version.as_str(),
            ),
        ] {
            if !is_nonempty(value) {
                errors.push(format!(
                    "CASE_{}_EMPTY:{}",
                    field.to_ascii_uppercase(),
                    case.case_id
                ));
            }
        }
        if !matches!(
            case.family_classification.as_str(),
            "COMPLETE" | "INCOMPLETE" | "UNKNOWN"
        ) {
            errors.push(format!("FAMILY_CLASSIFICATION_INVALID:{}", case.case_id));
        }
        if !matches!(
            case.classification_review_status.as_str(),
            "REVIEWED" | "UNREVIEWED" | "UNKNOWN"
        ) {
            errors.push(format!(
                "CLASSIFICATION_REVIEW_STATUS_INVALID:{}",
                case.case_id
            ));
        }
        validate_string_list(
            &case.observed_failure_families,
            &format!("CASE_FAMILIES:{}", case.case_id),
            true,
            errors,
        );
        validate_string_list(
            &case.observed_evidence_tags,
            &format!("CASE_EVIDENCE:{}", case.case_id),
            false,
            errors,
        );
        if !matches!(case.hard_gate_status.as_str(), "PASS" | "BLOCK" | "UNKNOWN") {
            errors.push(format!("HARD_GATE_STATUS_INVALID:{}", case.case_id));
        }
        if !matches!(
            case.expected_scope_status.as_str(),
            "IN_SCOPE" | "OUT_OF_SCOPE"
        ) {
            errors.push(format!("EXPECTED_SCOPE_STATUS_INVALID:{}", case.case_id));
        }
        if !matches!(
            case.expected_evidence_status.as_str(),
            "SUFFICIENT" | "INSUFFICIENT"
        ) {
            errors.push(format!("EXPECTED_EVIDENCE_STATUS_INVALID:{}", case.case_id));
        }
        if !matches!(
            case.expected_disposition.as_str(),
            "REVIEW_ONLY_CANDIDATE" | "REASSESS_SCOPE" | "HOLD_EVIDENCE" | "BLOCKED_HARD_GATE"
        ) {
            errors.push(format!("EXPECTED_DISPOSITION_INVALID:{}", case.case_id));
        }
    }
}

fn canonical_corpus_sha256(corpus: &Corpus) -> String {
    let mut canonical = corpus.clone();

    for profile in &mut canonical.profiles {
        profile.covered_failure_families.sort();
        profile.required_evidence_tags.sort();
    }
    canonical
        .profiles
        .sort_by(|a, b| a.profile_id.cmp(&b.profile_id));

    for case in &mut canonical.cases {
        case.observed_failure_families.sort();
        case.observed_evidence_tags.sort();
    }
    canonical.cases.sort_by(|a, b| a.case_id.cmp(&b.case_id));

    match serde_json::to_vec(&canonical) {
        Ok(bytes) => sha256_hex(&bytes),
        Err(_) => String::new(),
    }
}

fn evaluate_case(case: &ScenarioCase, profiles: &[ScopeProfile]) -> CaseResult {
    let profile = profiles
        .iter()
        .find(|profile| profile.profile_id == case.profile_id);

    let scope_in_bounds = match profile {
        None => false,
        Some(profile) => {
            case.domain == profile.domain
                && case.profile_version == profile.profile_version
                && case.criteria_regime == profile.criteria_regime
                && case.family_taxonomy_version == profile.family_taxonomy_version
                && case.classification_review_status == "REVIEWED"
                && case
                    .classification_record_ref
                    .as_deref()
                    .map(|reference| {
                        is_nonempty(reference)
                            && reference.starts_with("synthetic://classification-review/")
                    })
                    .unwrap_or(false)
                && case.family_classification == "COMPLETE"
                && !case.observed_failure_families.is_empty()
                && case
                    .observed_failure_families
                    .iter()
                    .all(|family| profile.covered_failure_families.contains(family))
        }
    };

    let scope_status = if scope_in_bounds {
        "IN_SCOPE"
    } else {
        "OUT_OF_SCOPE"
    };

    let evidence_sufficient = match profile {
        None => false,
        Some(profile) => {
            let fresh_enough = case
                .evidence_age_days
                .map(|age| age <= profile.max_evidence_age_days)
                .unwrap_or(false);
            let all_required_present = profile
                .required_evidence_tags
                .iter()
                .all(|required| case.observed_evidence_tags.contains(required));
            fresh_enough && all_required_present
        }
    };
    let evidence_status = if evidence_sufficient {
        "SUFFICIENT"
    } else {
        "INSUFFICIENT"
    };

    let disposition = if scope_status == "OUT_OF_SCOPE" {
        "REASSESS_SCOPE"
    } else if case.hard_gate_status == "BLOCK" {
        "BLOCKED_HARD_GATE"
    } else if case.hard_gate_status == "UNKNOWN" || evidence_status == "INSUFFICIENT" {
        "HOLD_EVIDENCE"
    } else {
        "REVIEW_ONLY_CANDIDATE"
    };

    let expected_outcome_matches = scope_status == case.expected_scope_status
        && evidence_status == case.expected_evidence_status
        && disposition == case.expected_disposition;

    CaseResult {
        case_id: case.case_id.clone(),
        profile_id: case.profile_id.clone(),
        scope_status: scope_status.to_string(),
        evidence_status: evidence_status.to_string(),
        hard_gate_status: case.hard_gate_status.clone(),
        disposition: disposition.to_string(),
        expected_scope_status: case.expected_scope_status.clone(),
        expected_evidence_status: case.expected_evidence_status.clone(),
        expected_disposition: case.expected_disposition.clone(),
        expected_outcome_matches,
        review_only: true,
        actuation_authorized: false,
    }
}

fn audit(corpus: &Corpus, source_commit: Option<String>, mut errors: Vec<String>) -> AuditReceipt {
    validate_corpus(corpus, source_commit.as_deref(), &mut errors);

    let mut case_results: Vec<CaseResult> = corpus
        .cases
        .iter()
        .map(|case| evaluate_case(case, &corpus.profiles))
        .collect();
    case_results.sort_by(|a, b| a.case_id.cmp(&b.case_id));

    let mut scope_counts = BTreeMap::new();
    let mut evidence_counts = BTreeMap::new();
    let mut disposition_counts = BTreeMap::new();
    for result in &case_results {
        increment(&mut scope_counts, &result.scope_status);
        increment(&mut evidence_counts, &result.evidence_status);
        increment(&mut disposition_counts, &result.disposition);
        if !result.expected_outcome_matches {
            errors.push(format!("EXPECTED_OUTCOME_MISMATCH:{}", result.case_id));
        }
    }

    let expected_outcomes_match = case_results
        .iter()
        .all(|result| result.expected_outcome_matches);
    let all_review_only = case_results.iter().all(|result| result.review_only);
    let all_actuation_disabled = !corpus.claim_ceiling.expected_actuation
        && case_results
            .iter()
            .all(|result| !result.actuation_authorized);
    if !all_review_only {
        errors.push("REVIEW_ONLY_CEILING_VIOLATED".to_string());
    }
    if !all_actuation_disabled {
        errors.push("ACTUATION_AUTHORIZED_OUTPUT_FOUND".to_string());
    }
    errors.sort();
    errors.dedup();

    let passed =
        errors.is_empty() && expected_outcomes_match && all_review_only && all_actuation_disabled;

    AuditReceipt {
        result: if passed { "pass" } else { "fail" }.to_string(),
        schema_version: corpus.schema_version.clone(),
        algorithm_id: corpus.algorithm_id.clone(),
        canonical_corpus_sha256: canonical_corpus_sha256(corpus),
        source_commit,
        profile_count: corpus.profiles.len(),
        case_count: corpus.cases.len(),
        scope_counts,
        evidence_counts,
        disposition_counts,
        expected_outcomes_match,
        all_review_only,
        all_actuation_disabled,
        case_results,
        errors,
    }
}

fn usage_error(message: &str) -> ExitCode {
    eprintln!("{message}");
    ExitCode::FAILURE
}

fn main() -> ExitCode {
    let mut input_path: Option<String> = None;
    let mut source_commit: Option<String> = None;
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        if arg == "--source-commit" {
            if source_commit.is_some() {
                return usage_error("--source-commit may be supplied only once");
            }
            match args.next() {
                Some(value) => source_commit = Some(value),
                None => return usage_error("--source-commit requires a 40-character commit SHA"),
            }
        } else if arg.starts_with("--") {
            return usage_error("unsupported command-line option");
        } else if input_path.is_none() {
            input_path = Some(arg);
        } else {
            return usage_error("provide at most one corpus path");
        }
    }

    let raw = match input_path {
        Some(path) => match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => return usage_error(&format!("could not read corpus: {error}")),
        },
        None => EMBEDDED_CORPUS.as_bytes().to_vec(),
    };

    let corpus: Corpus = match serde_json::from_slice(&raw) {
        Ok(corpus) => corpus,
        Err(error) => return usage_error(&format!("CORPUS_JSON_INVALID: {error}")),
    };

    let receipt = audit(&corpus, source_commit, Vec::new());
    match serde_json::to_string_pretty(&receipt) {
        Ok(output) => println!("{output}"),
        Err(error) => return usage_error(&format!("RECEIPT_SERIALIZATION_FAILED: {error}")),
    }

    if receipt.result == "pass" {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Corpus {
        serde_json::from_str(EMBEDDED_CORPUS).expect("embedded corpus must parse")
    }

    fn find_case_mut<'a>(corpus: &'a mut Corpus, case_id: &str) -> &'a mut ScenarioCase {
        corpus
            .cases
            .iter_mut()
            .find(|case| case.case_id == case_id)
            .expect("fixture case must exist")
    }

    fn result_for<'a>(receipt: &'a AuditReceipt, case_id: &str) -> &'a CaseResult {
        receipt
            .case_results
            .iter()
            .find(|result| result.case_id == case_id)
            .expect("evaluated case must exist")
    }

    fn run(corpus: &Corpus) -> AuditReceipt {
        audit(corpus, Some("a".repeat(40)), Vec::new())
    }

    #[test]
    fn frozen_corpus_matches_expected_outcomes() {
        let receipt = run(&corpus());
        assert_eq!(receipt.result, "pass");
        assert_eq!(receipt.profile_count, 3);
        assert_eq!(receipt.case_count, 17);
        assert!(receipt.expected_outcomes_match);
        assert_eq!(receipt.scope_counts.get("IN_SCOPE"), Some(&10));
        assert_eq!(receipt.scope_counts.get("OUT_OF_SCOPE"), Some(&7));
        assert_eq!(receipt.evidence_counts.get("INSUFFICIENT"), Some(&5));
        assert_eq!(
            receipt.disposition_counts.get("REVIEW_ONLY_CANDIDATE"),
            Some(&3)
        );
        assert_eq!(receipt.disposition_counts.get("REASSESS_SCOPE"), Some(&7));
        assert_eq!(receipt.disposition_counts.get("HOLD_EVIDENCE"), Some(&5));
        assert_eq!(
            receipt.disposition_counts.get("BLOCKED_HARD_GATE"),
            Some(&2)
        );
    }

    #[test]
    fn any_uncovered_family_blocks_candidate_promotion() {
        let mut value = corpus();
        let case = find_case_mut(&mut value, "forest-known-family-complete-evidence");
        case.observed_failure_families
            .push("chemical_residue_contamination".to_string());
        let receipt = run(&value);
        let result = result_for(&receipt, "forest-known-family-complete-evidence");
        assert_eq!(result.scope_status, "OUT_OF_SCOPE");
        assert_eq!(result.disposition, "REASSESS_SCOPE");
        assert!(!result.actuation_authorized);
    }

    #[test]
    fn mixed_known_and_novel_families_are_out_of_scope() {
        let receipt = run(&corpus());
        let result = result_for(&receipt, "forest-mixed-known-and-held-out-contamination");
        assert_eq!(result.scope_status, "OUT_OF_SCOPE");
        assert_eq!(result.disposition, "REASSESS_SCOPE");
    }

    #[test]
    fn incomplete_classification_is_out_of_scope() {
        let receipt = run(&corpus());
        assert_eq!(
            result_for(&receipt, "forest-family-classification-incomplete").scope_status,
            "OUT_OF_SCOPE"
        );
        assert_eq!(
            result_for(&receipt, "infrastructure-hazard-classification-unknown").disposition,
            "REASSESS_SCOPE"
        );
    }

    #[test]
    fn criteria_regime_mismatch_is_out_of_scope() {
        let receipt = run(&corpus());
        assert_eq!(
            result_for(&receipt, "forest-criteria-regime-changed").disposition,
            "REASSESS_SCOPE"
        );
    }

    #[test]
    fn profile_version_mismatch_is_out_of_scope() {
        let receipt = run(&corpus());
        assert_eq!(
            result_for(&receipt, "infrastructure-profile-version-mismatch").scope_status,
            "OUT_OF_SCOPE"
        );
    }

    #[test]
    fn unregistered_profile_fails_closed() {
        let receipt = run(&corpus());
        let result = result_for(&receipt, "unknown-profile-id");
        assert_eq!(result.scope_status, "OUT_OF_SCOPE");
        assert_eq!(result.evidence_status, "INSUFFICIENT");
        assert_eq!(result.disposition, "REASSESS_SCOPE");
    }

    #[test]
    fn stale_evidence_is_insufficient() {
        let receipt = run(&corpus());
        assert_eq!(
            result_for(&receipt, "forest-stale-evidence").evidence_status,
            "INSUFFICIENT"
        );
        assert_eq!(
            result_for(&receipt, "pv-stale-inspection").disposition,
            "HOLD_EVIDENCE"
        );
    }

    #[test]
    fn missing_required_evidence_is_insufficient() {
        let receipt = run(&corpus());
        assert_eq!(
            result_for(&receipt, "forest-required-legal-evidence-missing").disposition,
            "HOLD_EVIDENCE"
        );
        assert_eq!(
            result_for(&receipt, "infrastructure-capacity-evidence-missing").evidence_status,
            "INSUFFICIENT"
        );
    }

    #[test]
    fn unknown_hard_gate_is_never_promoted() {
        let receipt = run(&corpus());
        let result = result_for(&receipt, "forest-hard-gate-unknown");
        assert_eq!(result.hard_gate_status, "UNKNOWN");
        assert_eq!(result.disposition, "HOLD_EVIDENCE");
    }

    #[test]
    fn block_gate_is_non_compensatory() {
        let receipt = run(&corpus());
        assert_eq!(
            result_for(&receipt, "forest-habitat-hard-gate-block").disposition,
            "BLOCKED_HARD_GATE"
        );
        assert_eq!(
            result_for(&receipt, "pv-insulation-safety-gate-block").disposition,
            "BLOCKED_HARD_GATE"
        );
    }

    #[test]
    fn out_of_scope_takes_precedence_over_a_block_gate() {
        let mut value = corpus();
        let case = find_case_mut(&mut value, "forest-mixed-known-and-held-out-contamination");
        case.hard_gate_status = "BLOCK".to_string();
        let receipt = run(&value);
        let result = result_for(&receipt, "forest-mixed-known-and-held-out-contamination");
        assert_eq!(result.disposition, "REASSESS_SCOPE");
    }

    #[test]
    fn block_gate_precedes_evidence_hold_when_in_scope() {
        let mut value = corpus();
        let case = find_case_mut(&mut value, "forest-required-legal-evidence-missing");
        case.hard_gate_status = "BLOCK".to_string();
        let receipt = run(&value);
        assert_eq!(
            result_for(&receipt, "forest-required-legal-evidence-missing").disposition,
            "BLOCKED_HARD_GATE"
        );
    }

    #[test]
    fn freshness_boundary_is_inclusive() {
        let mut value = corpus();
        let case = find_case_mut(&mut value, "forest-known-family-complete-evidence");
        case.evidence_age_days = Some(30);
        let receipt = run(&value);
        let result = result_for(&receipt, "forest-known-family-complete-evidence");
        assert_eq!(result.evidence_status, "SUFFICIENT");
        assert_eq!(result.disposition, "REVIEW_ONLY_CANDIDATE");
    }

    #[test]
    fn absent_evidence_age_holds_for_evidence() {
        let mut value = corpus();
        let case = find_case_mut(&mut value, "forest-known-family-complete-evidence");
        case.evidence_age_days = None;
        let receipt = run(&value);
        assert_eq!(
            result_for(&receipt, "forest-known-family-complete-evidence").disposition,
            "HOLD_EVIDENCE"
        );
    }

    #[test]
    fn empty_observed_family_set_is_audit_failure() {
        let mut value = corpus();
        find_case_mut(&mut value, "forest-known-family-complete-evidence")
            .observed_failure_families
            .clear();
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.contains("LIST_MUST_NOT_BE_EMPTY")));
    }

    #[test]
    fn duplicate_case_ids_fail_closed() {
        let mut value = corpus();
        let duplicate = value.cases[0].clone();
        value.cases.push(duplicate);
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("DUPLICATE_CASE_ID")));
    }

    #[test]
    fn duplicate_profile_ids_fail_closed() {
        let mut value = corpus();
        let duplicate = value.profiles[0].clone();
        value.profiles.push(duplicate);
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.starts_with("DUPLICATE_PROFILE_ID")));
    }

    #[test]
    fn empty_profile_family_registry_fails_closed() {
        let mut value = corpus();
        value.profiles[0].covered_failure_families.clear();
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.contains("LIST_MUST_NOT_BE_EMPTY")));
    }

    #[test]
    fn invalid_claim_ceiling_fails_closed() {
        let mut value = corpus();
        value.claim_ceiling.expected_actuation = true;
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(!receipt.all_actuation_disabled);
    }

    #[test]
    fn invalid_source_commit_fails_closed() {
        let receipt = audit(&corpus(), Some("not-a-commit".to_string()), Vec::new());
        assert_eq!(receipt.result, "fail");
        assert!(receipt
            .errors
            .iter()
            .any(|error| error == "SOURCE_COMMIT_INVALID"));
    }

    #[test]
    fn canonical_digest_and_receipt_ignore_row_order() {
        let original = corpus();
        let mut permuted = original.clone();
        permuted.profiles.reverse();
        permuted.cases.reverse();
        for profile in &mut permuted.profiles {
            profile.covered_failure_families.reverse();
            profile.required_evidence_tags.reverse();
        }
        for case in &mut permuted.cases {
            case.observed_failure_families.reverse();
            case.observed_evidence_tags.reverse();
        }
        let first = run(&original);
        let second = run(&permuted);
        assert_eq!(
            first.canonical_corpus_sha256,
            second.canonical_corpus_sha256
        );
        assert_eq!(
            serde_json::to_vec(&first).expect("receipt serializes"),
            serde_json::to_vec(&second).expect("receipt serializes")
        );
    }

    #[test]
    fn results_are_sorted_and_all_outputs_remain_review_only() {
        let receipt = run(&corpus());
        let ids: Vec<&str> = receipt
            .case_results
            .iter()
            .map(|result| result.case_id.as_str())
            .collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted);
        assert!(receipt.all_review_only);
        assert!(receipt.all_actuation_disabled);
        assert!(receipt
            .case_results
            .iter()
            .all(|result| !result.actuation_authorized));
    }

    #[test]
    fn invalid_hard_gate_enum_fails_closed() {
        let mut value = corpus();
        find_case_mut(&mut value, "forest-known-family-complete-evidence").hard_gate_status =
            "MAYBE".to_string();
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.contains("HARD_GATE_STATUS_INVALID")));
    }

    #[test]
    fn unrecognized_failure_family_cannot_be_hidden_by_complete_evidence() {
        let mut value = corpus();
        let case = find_case_mut(&mut value, "pv-known-family-complete-evidence");
        case.observed_failure_families
            .push("delamination_with_hidden_interconnect_corrosion".to_string());
        let receipt = run(&value);
        let result = result_for(&receipt, "pv-known-family-complete-evidence");
        assert_eq!(result.evidence_status, "SUFFICIENT");
        assert_eq!(result.scope_status, "OUT_OF_SCOPE");
        assert_eq!(result.disposition, "REASSESS_SCOPE");
    }

    #[test]
    fn cartesian_shift_matrix_covers_compound_scope_evidence_and_gate_interactions() {
        let base_profiles = corpus().profiles;
        let mut combinations = 0usize;

        for profile in &base_profiles {
            for family_is_known in [true, false] {
                for classification in ["COMPLETE", "INCOMPLETE"] {
                    for regime_matches in [true, false] {
                        for taxonomy_matches in [true, false] {
                            for review_status in ["REVIEWED", "UNREVIEWED"] {
                                for record_is_valid in [true, false] {
                                    for evidence_mode in ["FRESH", "STALE", "MISSING_TAG"] {
                                        for gate in ["PASS", "BLOCK", "UNKNOWN"] {
                                            let observed_family = if family_is_known {
                                                profile.covered_failure_families[0].clone()
                                            } else {
                                                format!("held-out-family-{}", profile.profile_id)
                                            };
                                            let evidence_age_days =
                                                Some(if evidence_mode == "STALE" {
                                                    profile.max_evidence_age_days.saturating_add(1)
                                                } else {
                                                    0
                                                });
                                            let mut evidence_tags =
                                                profile.required_evidence_tags.clone();
                                            if evidence_mode == "MISSING_TAG"
                                                && !evidence_tags.is_empty()
                                            {
                                                evidence_tags.remove(0);
                                            }

                                            let expected_scope = if family_is_known
                                                && classification == "COMPLETE"
                                                && regime_matches
                                                && taxonomy_matches
                                                && review_status == "REVIEWED"
                                                && record_is_valid
                                            {
                                                "IN_SCOPE"
                                            } else {
                                                "OUT_OF_SCOPE"
                                            };
                                            let expected_evidence = if evidence_mode == "FRESH" {
                                                "SUFFICIENT"
                                            } else {
                                                "INSUFFICIENT"
                                            };
                                            let expected_disposition =
                                                if expected_scope == "OUT_OF_SCOPE" {
                                                    "REASSESS_SCOPE"
                                                } else if gate == "BLOCK" {
                                                    "BLOCKED_HARD_GATE"
                                                } else if gate == "UNKNOWN"
                                                    || expected_evidence == "INSUFFICIENT"
                                                {
                                                    "HOLD_EVIDENCE"
                                                } else {
                                                    "REVIEW_ONLY_CANDIDATE"
                                                };

                                            let case = ScenarioCase {
                                                case_id: format!(
                                                    "matrix-{}-{}-{}-{}-{}-{}-{}-{}-{}",
                                                    profile.profile_id,
                                                    family_is_known,
                                                    classification,
                                                    regime_matches,
                                                    taxonomy_matches,
                                                    review_status,
                                                    record_is_valid,
                                                    evidence_mode,
                                                    gate
                                                ),
                                                profile_id: profile.profile_id.clone(),
                                                domain: profile.domain.clone(),
                                                profile_version: if regime_matches {
                                                    profile.profile_version.clone()
                                                } else {
                                                    format!("{}-changed", profile.profile_version)
                                                },
                                                criteria_regime: if regime_matches {
                                                    profile.criteria_regime.clone()
                                                } else {
                                                    format!("{}-changed", profile.criteria_regime)
                                                },
                                                family_taxonomy_version: if taxonomy_matches {
                                                    profile.family_taxonomy_version.clone()
                                                } else {
                                                    format!(
                                                        "{}-changed",
                                                        profile.family_taxonomy_version
                                                    )
                                                },
                                                classification_review_status: review_status
                                                    .to_string(),
                                                classification_record_ref: if record_is_valid {
                                                    Some(format!(
                                                        "synthetic://classification-review/matrix/{}",
                                                        profile.profile_id
                                                    ))
                                                } else {
                                                    None
                                                },
                                                family_classification: classification.to_string(),
                                                observed_failure_families: vec![observed_family],
                                                evidence_age_days,
                                                observed_evidence_tags: evidence_tags,
                                                hard_gate_status: gate.to_string(),
                                                expected_scope_status: expected_scope.to_string(),
                                                expected_evidence_status: expected_evidence
                                                    .to_string(),
                                                expected_disposition: expected_disposition
                                                    .to_string(),
                                            };

                                            let result =
                                                evaluate_case(&case, std::slice::from_ref(profile));
                                            assert_eq!(result.scope_status, expected_scope);
                                            assert_eq!(result.evidence_status, expected_evidence);
                                            assert_eq!(result.disposition, expected_disposition);
                                            assert!(result.expected_outcome_matches);
                                            assert!(result.review_only);
                                            assert!(!result.actuation_authorized);
                                            combinations += 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        assert_eq!(combinations, 1728);
    }

    #[test]
    fn missing_classification_record_forces_scope_reassessment() {
        let mut value = corpus();
        let case = find_case_mut(&mut value, "forest-known-family-complete-evidence");
        case.classification_record_ref = None;
        case.expected_scope_status = "OUT_OF_SCOPE".to_string();
        case.expected_disposition = "REASSESS_SCOPE".to_string();
        let receipt = run(&value);
        let result = result_for(&receipt, "forest-known-family-complete-evidence");
        assert_eq!(result.scope_status, "OUT_OF_SCOPE");
        assert_eq!(result.disposition, "REASSESS_SCOPE");
        assert!(receipt.expected_outcomes_match);
    }

    #[test]
    fn unreviewed_classification_forces_scope_reassessment() {
        let mut value = corpus();
        let case = find_case_mut(&mut value, "forest-known-family-complete-evidence");
        case.classification_review_status = "UNREVIEWED".to_string();
        case.expected_scope_status = "OUT_OF_SCOPE".to_string();
        case.expected_disposition = "REASSESS_SCOPE".to_string();
        let receipt = run(&value);
        assert_eq!(
            result_for(&receipt, "forest-known-family-complete-evidence").disposition,
            "REASSESS_SCOPE"
        );
        assert!(receipt.expected_outcomes_match);
    }

    #[test]
    fn taxonomy_version_mismatch_forces_scope_reassessment() {
        let mut value = corpus();
        let case = find_case_mut(&mut value, "forest-known-family-complete-evidence");
        case.family_taxonomy_version = "forest-hazard-taxonomy-v2".to_string();
        case.expected_scope_status = "OUT_OF_SCOPE".to_string();
        case.expected_disposition = "REASSESS_SCOPE".to_string();
        let receipt = run(&value);
        assert_eq!(
            result_for(&receipt, "forest-known-family-complete-evidence").scope_status,
            "OUT_OF_SCOPE"
        );
        assert!(receipt.expected_outcomes_match);
    }

    #[test]
    fn non_synthetic_classification_reference_is_out_of_scope() {
        let mut value = corpus();
        let case = find_case_mut(&mut value, "forest-known-family-complete-evidence");
        case.classification_record_ref =
            Some("https://unverified.example/classification".to_string());
        case.expected_scope_status = "OUT_OF_SCOPE".to_string();
        case.expected_disposition = "REASSESS_SCOPE".to_string();
        let receipt = run(&value);
        assert_eq!(
            result_for(&receipt, "forest-known-family-complete-evidence").disposition,
            "REASSESS_SCOPE"
        );
        assert!(receipt.expected_outcomes_match);
    }

    #[test]
    fn unknown_classification_review_enum_fails_closed() {
        let mut value = corpus();
        find_case_mut(&mut value, "forest-known-family-complete-evidence")
            .classification_review_status = "TRUST_ME".to_string();
        let receipt = run(&value);
        assert_eq!(receipt.result, "fail");
        assert!(receipt
            .errors
            .iter()
            .any(|error| error.contains("CLASSIFICATION_REVIEW_STATUS_INVALID")));
    }
}
