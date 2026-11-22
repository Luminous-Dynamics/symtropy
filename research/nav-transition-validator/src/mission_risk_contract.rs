//! Semantic validation for mission-level navigation integrity risk contracts.
//!
//! The validator checks declared scope, integer risk-budget arithmetic,
//! unique check identity and completeness of evidence references. It does not
//! verify artifact contents, calibrate a bound, prove independence, or certify
//! physical safety. A structurally consistent `qualified` claim still needs
//! independent review of the referenced evidence.

use serde_json::Value;
use std::collections::BTreeSet;
use std::fmt;

const RISK_QUANTA_SCALE: &str = "probability_quanta_1e-15";
const MAX_PROBABILITY_QUANTA: u64 = 1_000_000_000_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissionRiskContractIssue {
    pub path: String,
    pub message: String,
}

impl fmt::Display for MissionRiskContractIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissionRiskContractSummary {
    pub mission_profile_id: String,
    pub claim_status: String,
    pub check_count: usize,
    pub qualified_check_count: usize,
    pub mission_budget_quanta: u64,
    pub allocated_quanta: u64,
    pub unallocated_quanta: u64,
    pub check_set_frozen: bool,
    pub structurally_complete: bool,
}

fn issue(issues: &mut Vec<MissionRiskContractIssue>, path: impl Into<String>, message: impl Into<String>) {
    issues.push(MissionRiskContractIssue { path: path.into(), message: message.into() });
}

fn required_object<'a>(
    value: &'a Value,
    key: &str,
    path: &str,
    issues: &mut Vec<MissionRiskContractIssue>,
) -> Option<&'a Value> {
    match value.get(key) {
        Some(found) if found.is_object() => Some(found),
        Some(_) => { issue(issues, format!("{path}.{key}"), "must be an object"); None },
        None => { issue(issues, format!("{path}.{key}"), "missing required object"); None },
    }
}

fn required_array<'a>(
    value: &'a Value,
    key: &str,
    path: &str,
    issues: &mut Vec<MissionRiskContractIssue>,
) -> Option<&'a Vec<Value>> {
    match value.get(key).and_then(Value::as_array) {
        Some(found) if !found.is_empty() => Some(found),
        Some(_) => { issue(issues, format!("{path}.{key}"), "must not be empty"); None },
        None => { issue(issues, format!("{path}.{key}"), "must be an array"); None },
    }
}

fn required_string(
    value: &Value,
    key: &str,
    path: &str,
    issues: &mut Vec<MissionRiskContractIssue>,
) -> Option<String> {
    let field_path = format!("{path}.{key}");
    match value.get(key).and_then(Value::as_str) {
        Some(found) if !found.trim().is_empty() => Some(found.to_owned()),
        Some(_) => { issue(issues, field_path, "must not be empty"); None },
        None => { issue(issues, field_path, "must be a non-empty string"); None },
    }
}

fn required_u64(
    value: &Value,
    key: &str,
    path: &str,
    issues: &mut Vec<MissionRiskContractIssue>,
) -> Option<u64> {
    let field_path = format!("{path}.{key}");
    match value.get(key).and_then(Value::as_u64) {
        Some(found) => Some(found),
        None => { issue(issues, field_path, "must be a non-negative integer"); None },
    }
}

fn required_bool(
    value: &Value,
    key: &str,
    path: &str,
    issues: &mut Vec<MissionRiskContractIssue>,
) -> Option<bool> {
    let field_path = format!("{path}.{key}");
    match value.get(key).and_then(Value::as_bool) {
        Some(found) => Some(found),
        None => { issue(issues, field_path, "must be a boolean"); None },
    }
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn nullable_sha256(value: &Value, key: &str, path: &str, issues: &mut Vec<MissionRiskContractIssue>) -> bool {
    match value.get(key) {
        Some(Value::Null) => false,
        Some(Value::String(digest)) if valid_sha256(digest) => true,
        Some(_) => { issue(issues, format!("{path}.{key}"), "must be null or a 64-character hexadecimal SHA-256 digest"); false },
        None => { issue(issues, format!("{path}.{key}"), "missing required digest field"); false },
    }
}

fn check_bound_evidence(
    evidence: &Value,
    path: &str,
    allocation_quanta: u64,
    mission_domain_ref: &str,
    issues: &mut Vec<MissionRiskContractIssue>,
) -> bool {
    let Some(status) = required_string(evidence, "status", path, issues) else { return false; };
    if status == "unqualified" {
        return false;
    }
    if status != "qualified" {
        issue(issues, format!("{path}.status"), "must be unqualified or qualified");
        return false;
    }

    let method = required_string(evidence, "method", path, issues);
    if !matches!(method.as_deref(), Some("student_t_radial" | "empirical_clopper_pearson" | "analytical_bound" | "reachable_set_bound" | "other")) {
        issue(issues, format!("{path}.method"), "unsupported bound method");
    }
    let upper = required_u64(evidence, "upper_bound_quanta", path, issues);
    if let Some(upper) = upper {
        if upper > allocation_quanta {
            issue(issues, format!("{path}.upper_bound_quanta"), "bound exceeds this check's allocated risk budget");
        }
        if upper >= MAX_PROBABILITY_QUANTA {
            issue(issues, format!("{path}.upper_bound_quanta"), "probability bound must be less than one");
        }
    }

    let confidence = required_u64(evidence, "confidence_quanta", path, issues);
    if let Some(confidence) = confidence {
        if confidence == 0 || confidence >= MAX_PROBABILITY_QUANTA {
            issue(issues, format!("{path}.confidence_quanta"), "confidence must be strictly between zero and one");
        }
    }
    let unit_kind = required_string(evidence, "independent_unit_kind", path, issues);
    if !matches!(unit_kind.as_deref(), Some("mission" | "route_session" | "fault_injection_trial" | "other")) {
        issue(issues, format!("{path}.independent_unit_kind"), "unsupported independent unit kind");
    }
    let unit_count = required_u64(evidence, "independent_unit_count", path, issues);
    if unit_count == Some(0) {
        issue(issues, format!("{path}.independent_unit_count"), "must be greater than zero");
    }
    let failure_count = match evidence.get("failure_count") {
        Some(Value::Null) => None,
        Some(value) => match value.as_u64() {
            Some(count) => Some(count),
            None => {
                issue(issues, format!("{path}.failure_count"), "must be null or a non-negative integer");
                None
            }
        },
        None => {
            issue(issues, format!("{path}.failure_count"), "missing required failure-count field");
            None
        }
    };
    if method.as_deref() == Some("empirical_clopper_pearson") {
        match (failure_count, unit_count) {
            (Some(failures), Some(trials)) if failures > trials => {
                issue(issues, format!("{path}.failure_count"), "cannot exceed the independent trial count");
            }
            (None, _) => {
                issue(issues, format!("{path}.failure_count"), "is required for empirical_clopper_pearson");
            }
            _ => {}
        }
    } else if failure_count.is_some() {
        issue(issues, format!("{path}.failure_count"), "must be null unless the method is empirical_clopper_pearson");
    }

    let independence_ref = required_string(evidence, "independence_basis_ref", path, issues);
    let domain_ref = required_string(evidence, "operational_domain_ref", path, issues);
    if domain_ref.as_deref().is_some_and(|value| value != mission_domain_ref) {
        issue(issues, format!("{path}.operational_domain_ref"), "must match the mission scope operational domain");
    }
    let _method_ref = required_string(evidence, "method_definition_ref", path, issues);
    let calibration_digest = nullable_sha256(evidence, "calibration_split_sha256", path, issues);
    let bound_digest = nullable_sha256(evidence, "bound_result_sha256", path, issues);
    let truth_digest = nullable_sha256(evidence, "truth_oracle_receipt_sha256", path, issues);
    let assumptions_reviewed = required_bool(evidence, "assumptions_reviewed", path, issues);
    let truth_reviewed = required_bool(evidence, "truth_oracle_reviewed", path, issues);
    if assumptions_reviewed == Some(false) {
        issue(issues, format!("{path}.assumptions_reviewed"), "qualified evidence requires explicit assumption review");
    }
    if truth_reviewed == Some(false) {
        issue(issues, format!("{path}.truth_oracle_reviewed"), "qualified evidence requires independent truth/oracle review");
    }
    let evidence_present = method.is_some()
        && upper.is_some()
        && confidence.is_some()
        && unit_kind.is_some()
        && unit_count.is_some_and(|count| count > 0)
        && independence_ref.is_some()
        && domain_ref.as_deref() == Some(mission_domain_ref)
        && calibration_digest
        && bound_digest
        && truth_digest
        && assumptions_reviewed == Some(true)
        && truth_reviewed == Some(true);
    evidence_present && !issues.iter().any(|item| item.path.starts_with(path))
}

/// Validate cross-field invariants after Draft 2020-12 schema validation.
///
/// The function computes exact risk arithmetic using integer quanta (1e-15)
/// so floating-point rounding cannot silently spend more than the mission
/// budget. It checks evidence-reference completeness but does not dereference
/// or independently verify referenced artifacts.
pub fn validate_mission_risk_contract(
    manifest: &Value,
) -> Result<MissionRiskContractSummary, Vec<MissionRiskContractIssue>> {
    let mut issues = Vec::new();
    if !manifest.is_object() {
        issue(&mut issues, "$", "manifest must be an object");
        return Err(issues);
    }
    if required_string(manifest, "schema_version", "$", &mut issues).as_deref() != Some("navigation-mission-risk-contract/v0.1") {
        issue(&mut issues, "$.schema_version", "unsupported schema version");
    }
    let mission_profile_id = required_string(manifest, "mission_profile_id", "$", &mut issues).unwrap_or_default();
    let claim_status = required_string(manifest, "claim_status", "$", &mut issues).unwrap_or_default();
    if !matches!(claim_status.as_str(), "research_only" | "candidate" | "qualified") {
        issue(&mut issues, "$.claim_status", "must be research_only, candidate or qualified");
    }
    let mission_scope = required_object(manifest, "mission_scope", "$", &mut issues);
    let mission_domain_ref = mission_scope
        .and_then(|scope| required_string(scope, "operational_domain_ref", "$.mission_scope", &mut issues))
        .unwrap_or_default();
    if let Some(scope) = mission_scope {
        for field in ["mission_unit", "time_basis_ref", "truth_oracle_scope_ref"] {
            let _ = required_string(scope, field, "$.mission_scope", &mut issues);
        }
    }

    let risk_budget = required_object(manifest, "risk_budget", "$", &mut issues);
    let mut mission_budget_quanta = 0;
    let mut check_set_frozen = false;
    if let Some(budget) = risk_budget {
        let scale = required_string(budget, "scale", "$.risk_budget", &mut issues);
        if scale.as_deref() != Some(RISK_QUANTA_SCALE) {
            issue(&mut issues, "$.risk_budget.scale", "unsupported probability quantum scale");
        }
        mission_budget_quanta = required_u64(budget, "mission_budget_quanta", "$.risk_budget", &mut issues).unwrap_or(0);
        if mission_budget_quanta == 0 || mission_budget_quanta >= MAX_PROBABILITY_QUANTA {
            issue(&mut issues, "$.risk_budget.mission_budget_quanta", "mission budget must be strictly between zero and one");
        }
        check_set_frozen = required_bool(budget, "check_set_frozen", "$.risk_budget", &mut issues).unwrap_or(false);
        let digest_present = nullable_sha256(budget, "check_set_sha256", "$.risk_budget", &mut issues);
        if check_set_frozen && !digest_present {
            issue(&mut issues, "$.risk_budget.check_set_sha256", "a frozen check set requires a SHA-256 digest");
        }
    }

    let checks = required_array(manifest, "checks", "$", &mut issues);
    let mut seen_ids = BTreeSet::new();
    let mut allocated_quanta = 0_u64;
    let mut qualified_check_count = 0_usize;
    let check_count = checks.map_or(0, Vec::len);
    if let Some(checks) = checks {
        for (index, check) in checks.iter().enumerate() {
            let path = format!("$.checks[{index}]");
            if !check.is_object() {
                issue(&mut issues, &path, "must be an object");
                continue;
            }
            let check_id = required_string(check, "check_id", &path, &mut issues).unwrap_or_default();
            if !seen_ids.insert(check_id.clone()) {
                issue(&mut issues, format!("{path}.check_id"), format!("duplicate check ID {check_id}"));
            }
            let _failure_event = required_string(check, "failure_event", &path, &mut issues);
            let allocation = required_u64(check, "allocation_quanta", &path, &mut issues).unwrap_or(0);
            if allocation == 0 || allocation >= MAX_PROBABILITY_QUANTA {
                issue(&mut issues, format!("{path}.allocation_quanta"), "allocation must be strictly between zero and one");
            }
            match allocated_quanta.checked_add(allocation) {
                Some(total) => allocated_quanta = total,
                None => issue(&mut issues, "$.checks", "allocated risk sum overflowed u64"),
            }
            if let Some(evidence) = required_object(check, "bound_evidence", &path, &mut issues) {
                if check_bound_evidence(evidence, &format!("{path}.bound_evidence"), allocation, &mission_domain_ref, &mut issues) {
                    qualified_check_count += 1;
                }
            }
        }
    }
    if allocated_quanta > mission_budget_quanta {
        issue(&mut issues, "$.checks", "sum of per-check allocations exceeds the mission risk budget");
    }
    let all_checks_qualified = check_count > 0 && qualified_check_count == check_count;
    let structurally_complete = all_checks_qualified && check_set_frozen && allocated_quanta <= mission_budget_quanta;
    if claim_status == "qualified" && !structurally_complete {
        issue(&mut issues, "$.claim_status", "qualified mission claim requires every check to have qualified bound evidence and a frozen check-set digest");
    }
    if issues.is_empty() {
        Ok(MissionRiskContractSummary {
            mission_profile_id,
            claim_status,
            check_count,
            qualified_check_count,
            mission_budget_quanta,
            allocated_quanta,
            unallocated_quanta: mission_budget_quanta.saturating_sub(allocated_quanta),
            check_set_frozen,
            structurally_complete,
        })
    } else {
        Err(issues)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> Value {
        serde_json::from_str(include_str!("../../../docs/research/navigation-mission-risk-v0.1/mission_risk_contract.schema.json")).unwrap()
    }
    fn research_fixture() -> Value {
        serde_json::from_str(include_str!("../../../docs/research/navigation-mission-risk-v0.1/examples/urbannav_medium_research_only.json")).unwrap()
    }
    fn qualified_fixture() -> Value {
        let mut value = research_fixture();
        value["claim_status"] = Value::String("qualified".to_owned());
        value["risk_budget"]["check_set_frozen"] = Value::Bool(true);
        value["risk_budget"]["check_set_sha256"] = Value::String("a".repeat(64));
        for check in value["checks"].as_array_mut().unwrap() {
            let evidence = &mut check["bound_evidence"];
            evidence["status"] = Value::String("qualified".to_owned());
            evidence["method"] = Value::String("analytical_bound".to_owned());
            evidence["upper_bound_quanta"] = Value::Number(serde_json::Number::from(400_000_000_000_u64));
            evidence["confidence_quanta"] = Value::Number(serde_json::Number::from(950_000_000_000_000_u64));
            evidence["independent_unit_kind"] = Value::String("mission".to_owned());
            evidence["independent_unit_count"] = Value::Number(serde_json::Number::from(100_u64));
            evidence["failure_count"] = Value::Null;
            evidence["independence_basis_ref"] = Value::String("review/independence/v1".to_owned());
            evidence["operational_domain_ref"] = Value::String("odds/urban-canyon-medium/v0.1".to_owned());
            evidence["method_definition_ref"] = Value::String("methods/analytical-bound/v1".to_owned());
            evidence["calibration_split_sha256"] = Value::String("b".repeat(64));
            evidence["bound_result_sha256"] = Value::String("c".repeat(64));
            evidence["truth_oracle_receipt_sha256"] = Value::String("d".repeat(64));
            evidence["assumptions_reviewed"] = Value::Bool(true);
            evidence["truth_oracle_reviewed"] = Value::Bool(true);
        }
        value
    }

    #[test]
    fn research_only_fixture_passes_schema_and_does_not_claim_qualification() {
        let schema = schema();
        let fixture = research_fixture();
        let validator = jsonschema::draft202012::options().build(&schema).unwrap();
        assert!(validator.is_valid(&fixture));
        let summary = validate_mission_risk_contract(&fixture).unwrap();
        assert_eq!(summary.claim_status, "research_only");
        assert_eq!(summary.check_count, 2);
        assert_eq!(summary.qualified_check_count, 0);
        assert!(!summary.structurally_complete);
        assert_eq!(summary.allocated_quanta, 1_000_000_000_000);
    }

    #[test]
    fn fully_documented_candidate_is_structurally_complete_but_not_a_safety_certificate() {
        let fixture = qualified_fixture();
        let validator = jsonschema::draft202012::options().build(&schema()).unwrap();
        assert!(validator.is_valid(&fixture));
        let summary = validate_mission_risk_contract(&fixture).unwrap();
        assert_eq!(summary.qualified_check_count, 2);
        assert!(summary.structurally_complete);
        assert_eq!(summary.unallocated_quanta, 0);
    }

    #[test]
    fn duplicate_check_ids_are_rejected() {
        let mut fixture = research_fixture();
        fixture["checks"][1]["check_id"] = fixture["checks"][0]["check_id"].clone();
        let errors = validate_mission_risk_contract(&fixture).unwrap_err();
        assert!(errors.iter().any(|error| error.message.contains("duplicate check ID")));
    }

    #[test]
    fn allocation_sum_over_mission_budget_is_rejected() {
        let mut fixture = research_fixture();
        fixture["checks"][1]["allocation_quanta"] = Value::Number(serde_json::Number::from(500_000_000_001_u64));
        let errors = validate_mission_risk_contract(&fixture).unwrap_err();
        assert!(errors.iter().any(|error| error.message.contains("exceeds the mission risk budget")));
    }

    #[test]
    fn qualified_claim_with_unqualified_checks_is_rejected() {
        let mut fixture = research_fixture();
        fixture["claim_status"] = Value::String("qualified".to_owned());
        fixture["risk_budget"]["check_set_frozen"] = Value::Bool(true);
        fixture["risk_budget"]["check_set_sha256"] = Value::String("a".repeat(64));
        let errors = validate_mission_risk_contract(&fixture).unwrap_err();
        assert!(errors.iter().any(|error| error.message.contains("every check")));
    }

    #[test]
    fn qualified_bound_cannot_exceed_its_check_allocation() {
        let mut fixture = qualified_fixture();
        fixture["checks"][0]["bound_evidence"]["upper_bound_quanta"] =
            Value::Number(serde_json::Number::from(500_000_000_001_u64));
        let errors = validate_mission_risk_contract(&fixture).unwrap_err();
        assert!(errors.iter().any(|error| error.message.contains("exceeds this check's allocated risk budget")));
    }

    #[test]
    fn qualified_bound_requires_referenced_calibration_and_truth_receipts() {
        let mut fixture = qualified_fixture();
        fixture["checks"][0]["bound_evidence"]["truth_oracle_receipt_sha256"] = Value::Null;
        let errors = validate_mission_risk_contract(&fixture).unwrap_err();
        assert!(errors.iter().any(|error| error.path.contains("truth_oracle_receipt_sha256")));
    }

    #[test]
    fn evidence_operational_domain_must_match_mission_scope() {
        let mut fixture = qualified_fixture();
        fixture["checks"][0]["bound_evidence"]["operational_domain_ref"] = Value::String("odds/other-domain/v1".to_owned());
        let errors = validate_mission_risk_contract(&fixture).unwrap_err();
        assert!(errors.iter().any(|error| error.message.contains("must match the mission scope")));
    }

    #[test]
    fn empirical_binomial_failure_count_cannot_exceed_independent_trials() {
        let mut fixture = qualified_fixture();
        let evidence = &mut fixture["checks"][0]["bound_evidence"];
        evidence["method"] = Value::String("empirical_clopper_pearson".to_owned());
        evidence["failure_count"] = Value::Number(serde_json::Number::from(101_u64));
        let errors = validate_mission_risk_contract(&fixture).unwrap_err();
        assert!(errors.iter().any(|error| error.message.contains("cannot exceed the independent trial count")));
    }

    #[test]
    fn empirical_binomial_method_requires_an_observed_failure_count() {
        let mut fixture = qualified_fixture();
        fixture["checks"][0]["bound_evidence"]["method"] =
            Value::String("empirical_clopper_pearson".to_owned());
        fixture["checks"][0]["bound_evidence"]["failure_count"] = Value::Null;
        let errors = validate_mission_risk_contract(&fixture).unwrap_err();
        assert!(errors.iter().any(|error| {
            error.message.contains("is required for empirical_clopper_pearson")
        }));
    }

    #[test]
    fn check_set_must_be_frozen_for_qualified_mission_claim() {
        let mut fixture = qualified_fixture();
        fixture["risk_budget"]["check_set_frozen"] = Value::Bool(false);
        let errors = validate_mission_risk_contract(&fixture).unwrap_err();
        assert!(errors.iter().any(|error| error.message.contains("every check")));
    }
}
