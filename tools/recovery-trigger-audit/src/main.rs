use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const EMBEDDED_CORPUS: &str =
    include_str!("../../../docs/tech/recovery-trigger-corpus-v0.1.json");

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
struct TriggerResult {
    case_id: String,
    trigger_id: String,
    domain_profile_id: String,
    previous_state: String,
    resulting_state: String,
    outcome: String,
    reason_code: String,
    review_action: String,
    actuation_authorized: bool,
}

#[derive(Debug, Serialize)]
struct AuditReceipt {
    result: String,
    schema_version: String,
    algorithm_id: String,
    corpus_sha256: String,
    source_commit: Option<String>,
    trigger_count: usize,
    unique_case_count: usize,
    domain_profile_count: usize,
    outcome_counts: BTreeMap<String, usize>,
    expected_outcomes_match: bool,
    all_review_only: bool,
    all_plan_only: bool,
    trigger_results: Vec<TriggerResult>,
    errors: Vec<String>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect::<Vec<_>>().join("")
}

fn nonempty_string<'a>(value: Option<&'a Value>) -> Option<&'a str> {
    value.and_then(Value::as_str).filter(|value| !value.trim().is_empty())
}

fn is_hex_sha(value: &str, digits: usize) -> bool {
    value.len() == digits && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn count(map: &mut BTreeMap<String, usize>, key: impl Into<String>) {
    *map.entry(key.into()).or_insert(0) += 1;
}

fn finite_number(value: Option<&Value>) -> Option<f64> {
    let value = value.and_then(Value::as_f64)?;
    if value.is_finite() { Some(value) } else { None }
}

fn compare(value: f64, operator: &str, threshold: f64) -> Option<bool> {
    match operator {
        "gte" => Some(value >= threshold),
        "gt" => Some(value > threshold),
        "lte" => Some(value <= threshold),
        "lt" => Some(value < threshold),
        _ => None,
    }
}

fn validate_profile(profile: &Value, label: &str, errors: &mut Vec<String>) {
    for field in ["trigger_id", "domain_profile_id", "criteria_version", "unit"] {
        if nonempty_string(profile.get(field)).is_none() {
            errors.push(format!("{label}: PROFILE_{}_MISSING", field.to_ascii_uppercase()));
        }
    }
    if nonempty_string(profile.get("decision_scope")) != Some("review_only") {
        errors.push(format!("{label}: DECISION_SCOPE_MUST_BE_REVIEW_ONLY"));
    }
    if nonempty_string(profile.get("action_on_trigger")) != Some("HUMAN_REVIEW_OR_REPLAN") {
        errors.push(format!("{label}: TRIGGER_ACTION_MUST_BE_REVIEW_OR_REPLAN"));
    }
    if nonempty_string(profile.get("action_on_clear")) != Some("HUMAN_REVIEW_STATE_TRANSITION") {
        errors.push(format!("{label}: CLEAR_ACTION_MUST_BE_REVIEW_ONLY"));
    }

    let enter = profile.get("enter");
    let exit = profile.get("exit");
    for (side, predicate) in [("ENTER", enter), ("EXIT", exit)] {
        let operator = nonempty_string(predicate.and_then(|v| v.get("operator")));
        if !matches!(operator, Some("gte" | "gt" | "lte" | "lt")) {
            errors.push(format!("{label}: {side}_OPERATOR_UNSUPPORTED"));
        }
        if finite_number(predicate.and_then(|v| v.get("threshold"))).is_none() {
            errors.push(format!("{label}: {side}_THRESHOLD_INVALID"));
        }
    }
    let minimum_coverage = finite_number(profile.get("minimum_coverage_ratio"));
    if minimum_coverage.map(|value| !(0.0..=1.0).contains(&value)).unwrap_or(true) {
        errors.push(format!("{label}: MINIMUM_COVERAGE_RATIO_INVALID"));
    }
    for field in ["max_observation_age_minutes", "required_response_lead_time_minutes"] {
        match profile.get(field).and_then(Value::as_i64) {
            Some(value) if value >= 0 => {}
            _ => errors.push(format!("{label}: {}_INVALID", field.to_ascii_uppercase())),
        }
    }
}

fn evaluate_case(case: &Value, errors: &mut Vec<String>) -> Option<TriggerResult> {
    let case_id = nonempty_string(case.get("id"))?.to_string();
    let profile = case.get("trigger_profile")?;
    let observation = case.get("observation")?;
    let label = case_id.as_str();

    validate_profile(profile, label, errors);
    let trigger_id = nonempty_string(profile.get("trigger_id")).unwrap_or("TRIGGER_ID_MISSING").to_string();
    let domain_profile_id = nonempty_string(profile.get("domain_profile_id")).unwrap_or("DOMAIN_PROFILE_MISSING").to_string();
    let profile_version = nonempty_string(profile.get("criteria_version")).unwrap_or("CRITERIA_VERSION_MISSING");
    let current_version = nonempty_string(case.get("current_decision_regime_version")).unwrap_or("CURRENT_REGIME_MISSING");
    let previous_state = nonempty_string(observation.get("previous_state")).unwrap_or("STATE_MISSING").to_string();
    let review_action = nonempty_string(profile.get("action_on_trigger")).unwrap_or("REVIEW_ACTION_MISSING").to_string();

    let (outcome, reason_code, resulting_state) = if current_version != profile_version {
        (
            "REASSESS_CRITERIA_REGIME",
            "CURRENT_DECISION_REGIME_VERSION_MISMATCH",
            "requires_reassessment",
        )
    } else {
        let expected_unit = nonempty_string(profile.get("unit")).unwrap_or("UNIT_MISSING");
        let observed_unit = nonempty_string(observation.get("unit")).unwrap_or("UNIT_MISSING");
        if observed_unit != expected_unit {
            ("HOLD_UNIT_MISMATCH", "OBSERVATION_UNIT_DIFFERS_FROM_TRIGGER_PROFILE", "held")
        } else {
            let age = observation.get("age_minutes").and_then(Value::as_i64);
            let max_age = profile.get("max_observation_age_minutes").and_then(Value::as_i64).unwrap_or(-1);
            if age.map(|value| value < 0).unwrap_or(true) {
                ("HOLD_INVALID_OBSERVATION", "OBSERVATION_AGE_INVALID", "held")
            } else if age.map(|value| value > max_age).unwrap_or(true) {
                ("HOLD_STALE_EVIDENCE", "OBSERVATION_EXCEEDS_MAX_AGE", "held")
            } else {
                let coverage = finite_number(observation.get("coverage_ratio"));
                let min_coverage = finite_number(profile.get("minimum_coverage_ratio")).unwrap_or(1.0);
                if coverage.map(|value| value < 0.0 || value > 1.0).unwrap_or(true) {
                    ("HOLD_INVALID_OBSERVATION", "COVERAGE_RATIO_INVALID", "held")
                } else if coverage.map(|value| value < min_coverage).unwrap_or(true) {
                    ("HOLD_INSUFFICIENT_COVERAGE", "OBSERVATION_COVERAGE_BELOW_MINIMUM", "held")
                } else {
                    let value = finite_number(observation.get("value"));
                    let remaining = observation.get("remaining_time_to_boundary_minutes").and_then(Value::as_i64);
                    let required_lead = profile.get("required_response_lead_time_minutes").and_then(Value::as_i64).unwrap_or(-1);
                    let enter_operator = nonempty_string(profile.get("enter").and_then(|v| v.get("operator"))).unwrap_or("invalid");
                    let enter_threshold = finite_number(profile.get("enter").and_then(|v| v.get("threshold")));
                    let exit_operator = nonempty_string(profile.get("exit").and_then(|v| v.get("operator"))).unwrap_or("invalid");
                    let exit_threshold = finite_number(profile.get("exit").and_then(|v| v.get("threshold")));

                    if value.is_none() || remaining.map(|x| x < 0).unwrap_or(true) || required_lead < 0 {
                        ("HOLD_INVALID_OBSERVATION", "OBSERVATION_VALUE_OR_TIMING_INVALID", "held")
                    } else {
                        let value = value.unwrap();
                        let is_active = match previous_state.as_str() {
                            "inactive" => match (enter_threshold, compare(value, enter_operator, enter_threshold.unwrap_or(0.0))) {
                                (Some(_), Some(true)) => true,
                                (Some(_), Some(false)) => false,
                                _ => {
                                    errors.push(format!("{label}: ENTER_PREDICATE_INVALID"));
                                    false
                                }
                            },
                            "active" => match (exit_threshold, compare(value, exit_operator, exit_threshold.unwrap_or(0.0))) {
                                (Some(_), Some(clear)) => !clear,
                                _ => {
                                    errors.push(format!("{label}: EXIT_PREDICATE_INVALID"));
                                    false
                                }
                            },
                            _ => {
                                errors.push(format!("{label}: PREVIOUS_STATE_UNSUPPORTED"));
                                false
                            }
                        };

                        if !matches!(previous_state.as_str(), "inactive" | "active") {
                            ("HOLD_INVALID_PRIOR_STATE", "PRIOR_TRIGGER_STATE_INVALID", "held")
                        } else if !is_active && previous_state == "inactive" {
                            ("NO_TRIGGER", "ENTRY_THRESHOLD_NOT_MET", "inactive")
                        } else if !is_active && previous_state == "active" {
                            ("TRIGGER_CLEARED_REVIEW_REQUIRED", "EXIT_CONDITION_SATISFIED_REVIEW_STATE_TRANSITION", "inactive")
                        } else if remaining.unwrap() < required_lead {
                            ("TRIGGER_LATE_REASSESSMENT_REQUIRED", "REMAINING_TIME_BELOW_REQUIRED_RESPONSE_LEAD", "active")
                        } else if previous_state == "active" {
                            ("TRIGGER_ACTIVE_REVIEW_REQUIRED", "EXIT_THRESHOLD_NOT_MET", "active")
                        } else {
                            ("TRIGGER_ACTIVE_REVIEW_REQUIRED", "ENTRY_THRESHOLD_MET_REVIEW_ONLY", "active")
                        }
                    }
                }
            }
        }
    };

    if case.get("expected_actuation").and_then(Value::as_bool) != Some(false) {
        errors.push(format!("{label}: EXPECTED_ACTUATION_MUST_BE_FALSE"));
    }
    if nonempty_string(observation.get("evidence_ref"))
        .map(|value| value.starts_with("synthetic://"))
        != Some(true)
    {
        errors.push(format!("{label}: SYNTHETIC_EVIDENCE_REFERENCE_INVALID"));
    }
    if nonempty_string(observation.get("capture_id")).is_none() {
        errors.push(format!("{label}: CAPTURE_ID_MISSING"));
    }
    if nonempty_string(case.get("expected_outcome")) != Some(outcome) {
        errors.push(format!("{label}: EXPECTED_OUTCOME_MISMATCH"));
    }
    if nonempty_string(case.get("expected_reason_code")) != Some(reason_code) {
        errors.push(format!("{label}: EXPECTED_REASON_CODE_MISMATCH"));
    }

    Some(TriggerResult {
        case_id,
        trigger_id,
        domain_profile_id,
        previous_state,
        resulting_state: resulting_state.to_string(),
        outcome: outcome.to_string(),
        reason_code: reason_code.to_string(),
        review_action,
        actuation_authorized: false,
    })
}

fn make_receipt(
    raw: &[u8],
    source_commit: Option<String>,
    parsed: Option<Value>,
    parse_error: Option<String>,
) -> AuditReceipt {
    let had_parse_error = parse_error.is_some();
    let mut errors = Vec::new();
    let mut schema_version = "unavailable".to_string();
    let mut algorithm_id = "unavailable".to_string();
    let mut case_ids = BTreeSet::new();
    let mut profiles = BTreeSet::new();
    let mut trigger_results = Vec::new();
    let mut outcome_counts = BTreeMap::new();
    let mut trigger_count = 0usize;
    let mut all_review_only = true;
    let mut all_plan_only = false;

    if let Some(error) = parse_error {
        errors.push(format!("CORPUS_JSON_INVALID: {error}"));
    }
    if let Some(commit) = source_commit.as_deref() {
        if !is_hex_sha(commit, 40) {
            errors.push("SOURCE_COMMIT_INVALID".to_string());
        }
    }

    if let Some(root) = parsed.as_ref() {
        schema_version = nonempty_string(root.get("schema_version")).unwrap_or("unavailable").to_string();
        algorithm_id = nonempty_string(root.get("algorithm_id")).unwrap_or("unavailable").to_string();

        if schema_version != "recovery-trigger-contract-v0.1" {
            errors.push("SCHEMA_VERSION_UNSUPPORTED".to_string());
        }
        if algorithm_id != "versioned-signpost-trigger-evaluator-v0.1" {
            errors.push("ALGORITHM_ID_UNSUPPORTED".to_string());
        }
        if nonempty_string(root.get("status")) != Some("synthetic-boundary-fixtures") {
            errors.push("CORPUS_STATUS_UNEXPECTED".to_string());
        }
        let claim = root.get("claim_ceiling");
        all_plan_only = claim.and_then(|v| v.get("expected_actuation")).and_then(Value::as_bool) == Some(false);
        if !all_plan_only {
            errors.push("TOP_LEVEL_ACTUATION_CEILING_INVALID".to_string());
        }
        if claim.and_then(|v| v.get("real_world_measurements")).and_then(Value::as_bool) != Some(false) {
            errors.push("REAL_WORLD_MEASUREMENTS_MUST_NOT_BE_CLAIMED".to_string());
        }
        if claim.and_then(|v| v.get("field_accuracy_claimed")).and_then(Value::as_bool) != Some(false) {
            errors.push("FIELD_ACCURACY_MUST_NOT_BE_CLAIMED".to_string());
        }
        if nonempty_string(claim.and_then(|v| v.get("execution_authority"))) != Some("NO_ACTUATION_AUTHORIZED") {
            errors.push("EXECUTION_AUTHORITY_INVALID".to_string());
        }

        let cases: Vec<&Value> = match root.get("cases").and_then(Value::as_array) {
            Some(cases) if !cases.is_empty() => cases.iter().collect(),
            _ => {
                errors.push("CASES_MISSING_OR_EMPTY".to_string());
                Vec::new()
            }
        };
        trigger_count = cases.len();
        match root.get("trigger_count").and_then(Value::as_u64) {
            Some(declared) if declared as usize == trigger_count => {}
            Some(declared) => errors.push(format!("TRIGGER_COUNT_MISMATCH: declared {declared}, actual {trigger_count}")),
            None => errors.push("TRIGGER_COUNT_MISSING".to_string()),
        }

        for (index, case) in cases.iter().enumerate() {
            let label = format!("CASE_{}", index + 1);
            match nonempty_string(case.get("id")) {
                Some(id) => {
                    if !case_ids.insert(id.to_string()) {
                        errors.push(format!("{id}: DUPLICATE_CASE_ID"));
                    }
                }
                None => errors.push(format!("{label}_ID_MISSING")),
            }
            match nonempty_string(case.get("domain_profile_id")) {
                Some(profile) => { profiles.insert(profile.to_string()); }
                None => errors.push(format!("{label}_DOMAIN_PROFILE_MISSING")),
            }
            let profile = case.get("trigger_profile");
            match nonempty_string(profile.and_then(|v| v.get("domain_profile_id"))) {
                Some(profile_id) if nonempty_string(case.get("domain_profile_id")) == Some(profile_id) => {}
                _ => errors.push(format!("{label}_PROFILE_BINDING_MISMATCH")),
            }
            match nonempty_string(case.get("id")) {
                Some(id) => match evaluate_case(case, &mut errors) {
                    Some(result) => {
                        count(&mut outcome_counts, result.outcome.clone());
                        if nonempty_string(profile.and_then(|v| v.get("decision_scope")) ) != Some("review_only") {
                            all_review_only = false;
                        }
                        trigger_results.push(result);
                    }
                    None => errors.push(format!("{id}: CASE_EVALUATION_FAILED")),
                },
                None => {}
            }
        }
        if case_ids.len() != trigger_count {
            errors.push("CASE_ID_COVERAGE_MISMATCH".to_string());
        }
    } else if !had_parse_error {
        errors.push("CORPUS_VALUE_MISSING".to_string());
    }

    trigger_results.sort_by(|a, b| a.case_id.cmp(&b.case_id));
    let expected_outcomes_match = trigger_results.len() == trigger_count
        && errors.iter().all(|error| !error.contains("EXPECTED_OUTCOME_MISMATCH") && !error.contains("EXPECTED_REASON_CODE_MISMATCH"));
    AuditReceipt {
        result: if errors.is_empty() { "pass" } else { "fail" }.to_string(),
        schema_version,
        algorithm_id,
        corpus_sha256: sha256_hex(raw),
        source_commit,
        trigger_count,
        unique_case_count: case_ids.len(),
        domain_profile_count: profiles.len(),
        outcome_counts,
        expected_outcomes_match,
        all_review_only,
        all_plan_only,
        trigger_results,
        errors,
    }
}

fn run() -> Result<(), String> {
    let mut path: Option<String> = None;
    let mut source_commit: Option<String> = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--source-commit" => source_commit = Some(args.next().ok_or_else(|| "--source-commit requires a value".to_string())?),
            "--help" | "-h" => {
                println!("Usage: recovery-trigger-audit <corpus.json> [--source-commit <40-hex-sha>]");
                return Ok(());
            }
            _ if arg.starts_with('-') => return Err(format!("unknown argument: {arg}")),
            _ if path.is_none() => path = Some(arg),
            _ => return Err("provide exactly one corpus path".to_string()),
        }
    }
    let path = path.ok_or_else(|| "missing corpus path; use --help for usage".to_string())?;
    let bytes = fs::read(&path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let receipt = match serde_json::from_slice::<Value>(&bytes) {
        Ok(value) => make_receipt(&bytes, source_commit, Some(value), None),
        Err(error) => make_receipt(&bytes, source_commit, None, Some(error.to_string())),
    };
    println!("{}", serde_json::to_string_pretty(&receipt).map_err(|error| format!("receipt serialization failed: {error}"))?);
    if receipt.result != "pass" {
        return Err("trigger contract audit failed".to_string());
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("recovery-trigger-audit: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn embedded() -> Value {
        serde_json::from_str(EMBEDDED_CORPUS).expect("trigger corpus must parse")
    }

    fn audit(value: Value) -> AuditReceipt {
        let raw = serde_json::to_vec(&value).expect("test input serializes");
        make_receipt(&raw, None, Some(value), None)
    }

    #[test]
    fn frozen_corpus_matches_all_expected_boundaries() {
        let receipt = make_receipt(EMBEDDED_CORPUS.as_bytes(), Some("d".repeat(40)), Some(embedded()), None);
        assert_eq!(receipt.result, "pass", "{:?}", receipt.errors);
        assert_eq!(receipt.trigger_count, 12);
        assert_eq!(receipt.unique_case_count, 12);
        assert_eq!(receipt.domain_profile_count, 3);
        assert!(receipt.expected_outcomes_match);
        assert!(receipt.all_review_only);
        assert!(receipt.all_plan_only);
        assert_eq!(receipt.claim_ceiling, "NO_ACTUATION_AUTHORIZED");
        assert_eq!(receipt.outcome_counts.get("TRIGGER_ACTIVE_REVIEW_REQUIRED"), Some(&5));
        assert_eq!(receipt.outcome_counts.get("NO_TRIGGER"), Some(&1));
        assert_eq!(receipt.outcome_counts.get("TRIGGER_CLEARED_REVIEW_REQUIRED"), Some(&1));
    }

    #[test]
    fn exact_entry_threshold_is_inclusive() {
        let receipt = audit(embedded());
        let result = receipt.trigger_results.iter().find(|r| r.case_id == "TRG-001-FOREST-ENTER-AT-BOUNDARY").unwrap();
        assert_eq!(result.outcome, "TRIGGER_ACTIVE_REVIEW_REQUIRED");
    }

    #[test]
    fn exact_exit_threshold_is_not_clear_when_exit_is_strict_lt() {
        let receipt = audit(embedded());
        let result = receipt.trigger_results.iter().find(|r| r.case_id == "TRG-004-FOREST-EXIT-AT-BOUNDARY").unwrap();
        assert_eq!(result.resulting_state, "active");
        assert_eq!(result.outcome, "TRIGGER_ACTIVE_REVIEW_REQUIRED");
    }

    #[test]
    fn strictly_below_exit_threshold_clears_to_review_state() {
        let receipt = audit(embedded());
        let result = receipt.trigger_results.iter().find(|r| r.case_id == "TRG-005-FOREST-EXIT-BELOW-BOUNDARY").unwrap();
        assert_eq!(result.resulting_state, "inactive");
        assert_eq!(result.outcome, "TRIGGER_CLEARED_REVIEW_REQUIRED");
    }

    #[test]
    fn age_equal_to_maximum_is_fresh() {
        let mut corpus = embedded();
        corpus["cases"][0]["observation"]["age_minutes"] = json!(60);
        let receipt = audit(corpus);
        let result = receipt.trigger_results.iter().find(|r| r.case_id == "TRG-001-FOREST-ENTER-AT-BOUNDARY").unwrap();
        assert_eq!(result.outcome, "TRIGGER_ACTIVE_REVIEW_REQUIRED");
    }

    #[test]
    fn stale_observation_holds_even_if_trigger_value_is_high() {
        let receipt = audit(embedded());
        let result = receipt.trigger_results.iter().find(|r| r.case_id == "TRG-006-FOREST-STALE-OBSERVATION").unwrap();
        assert_eq!(result.outcome, "HOLD_STALE_EVIDENCE");
    }

    #[test]
    fn insufficient_coverage_holds_even_if_trigger_value_is_high() {
        let receipt = audit(embedded());
        let result = receipt.trigger_results.iter().find(|r| r.case_id == "TRG-007-FOREST-INSUFFICIENT-COVERAGE").unwrap();
        assert_eq!(result.outcome, "HOLD_INSUFFICIENT_COVERAGE");
    }

    #[test]
    fn criteria_regime_change_forces_reassessment_before_threshold_evaluation() {
        let receipt = audit(embedded());
        let result = receipt.trigger_results.iter().find(|r| r.case_id == "TRG-008-CRITERIA-REGIME-CHANGED").unwrap();
        assert_eq!(result.outcome, "REASSESS_CRITERIA_REGIME");
    }

    #[test]
    fn unit_mismatch_does_not_implicitly_convert() {
        let receipt = audit(embedded());
        let result = receipt.trigger_results.iter().find(|r| r.case_id == "TRG-009-UNIT-MISMATCH").unwrap();
        assert_eq!(result.outcome, "HOLD_UNIT_MISMATCH");
    }

    #[test]
    fn active_trigger_detected_after_required_lead_time_is_late() {
        let receipt = audit(embedded());
        let result = receipt.trigger_results.iter().find(|r| r.case_id == "TRG-010-TRIGGER-TOO-LATE").unwrap();
        assert_eq!(result.outcome, "TRIGGER_LATE_REASSESSMENT_REQUIRED");
    }

    #[test]
    fn the_same_contract_accepts_distinct_pv_and_infrastructure_units() {
        let receipt = audit(embedded());
        assert!(receipt.trigger_results.iter().any(|r| r.domain_profile_id == "photovoltaic_second_life_trigger_v0"));
        assert!(receipt.trigger_results.iter().any(|r| r.domain_profile_id == "infrastructure_component_trigger_v0"));
    }

    #[test]
    fn unsupported_prior_state_fails_closed() {
        let mut corpus = embedded();
        corpus["cases"][0]["observation"]["previous_state"] = json!("unknown");
        let receipt = audit(corpus);
        let result = receipt.trigger_results.iter().find(|r| r.case_id == "TRG-001-FOREST-ENTER-AT-BOUNDARY").unwrap();
        assert_eq!(result.outcome, "HOLD_INVALID_PRIOR_STATE");
        assert!(receipt.errors.iter().any(|e| e.contains("PREVIOUS_STATE_UNSUPPORTED")));
    }

    #[test]
    fn negative_or_missing_age_fails_closed() {
        let mut corpus = embedded();
        corpus["cases"][0]["observation"]["age_minutes"] = json!(-1);
        let receipt = audit(corpus);
        let result = receipt.trigger_results.iter().find(|r| r.case_id == "TRG-001-FOREST-ENTER-AT-BOUNDARY").unwrap();
        assert_eq!(result.outcome, "HOLD_INVALID_OBSERVATION");
        assert_eq!(result.reason_code, "OBSERVATION_AGE_INVALID");
    }

    #[test]
    fn invalid_coverage_above_one_fails_closed() {
        let mut corpus = embedded();
        corpus["cases"][0]["observation"]["coverage_ratio"] = json!(1.2);
        let receipt = audit(corpus);
        let result = receipt.trigger_results.iter().find(|r| r.case_id == "TRG-001-FOREST-ENTER-AT-BOUNDARY").unwrap();
        assert_eq!(result.outcome, "HOLD_INVALID_OBSERVATION");
        assert_eq!(result.reason_code, "COVERAGE_RATIO_INVALID");
    }

    #[test]
    fn trigger_does_not_authorize_actuation() {
        let mut corpus = embedded();
        corpus["cases"][0]["trigger_profile"]["action_on_trigger"] = json!("execute_felling");
        let receipt = audit(corpus);
        assert!(!receipt.all_review_only);
        assert!(receipt.errors.iter().any(|e| e.contains("TRIGGER_ACTION_MUST_BE_REVIEW_OR_REPLAN")));
    }

    #[test]
    fn duplicate_case_ids_are_rejected() {
        let mut corpus = embedded();
        corpus["cases"][1]["id"] = corpus["cases"][0]["id"].clone();
        let receipt = audit(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("DUPLICATE_CASE_ID")));
    }

    #[test]
    fn expected_outcome_mismatch_fails_receipt() {
        let mut corpus = embedded();
        corpus["cases"][0]["expected_outcome"] = json!("NO_TRIGGER");
        let receipt = audit(corpus);
        assert_eq!(receipt.result, "fail");
        assert!(receipt.errors.iter().any(|e| e.contains("EXPECTED_OUTCOME_MISMATCH")));
    }

    #[test]
    fn source_commit_must_be_40_hex_characters() {
        let receipt = make_receipt(EMBEDDED_CORPUS.as_bytes(), Some("bad-sha".to_string()), Some(embedded()), None);
        assert!(receipt.errors.iter().any(|e| e.contains("SOURCE_COMMIT_INVALID")));
    }
}
