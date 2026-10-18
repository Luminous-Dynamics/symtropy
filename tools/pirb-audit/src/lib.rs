use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeSet;

const REQUIRED_SCENARIOS: [&str; 7] = [
    "pirb-r2-minimum-service-floor-01",
    "pirb-r1-required-acceptance-01",
    "pirb-r0-stale-capacity-01",
    "pirb-r0-conflicting-reports-01",
    "pirb-r4-personal-data-boundary-01",
    "pirb-r3-no-lookahead-01",
    "pirb-r1-refused-participation-01",
];

const ALLOWED_TRACKS: [&str; 6] = [
    "PIRB-0", "PIRB-1", "PIRB-2", "PIRB-3", "PIRB-4", "PIRB-5",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Finding {
    pub code: String,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AuditReport {
    pub benchmark_version: String,
    pub scenario_count: usize,
    pub passed: bool,
    pub findings: Vec<Finding>,
}

impl AuditReport {
    fn new(scenario_count: usize, findings: Vec<Finding>) -> Self {
        Self {
            benchmark_version: "PIRB-0.1".to_owned(),
            scenario_count,
            passed: findings.is_empty(),
            findings,
        }
    }
}

/// Audits the PIRB v0.1 synthetic reference corpus.
///
/// A passing result establishes only that the known synthetic fixtures obey
/// these reference semantics. It does not establish field validity, consent
/// legitimacy, causal impact, or deployment safety.
pub fn audit_corpus_json(input: &str) -> AuditReport {
    match serde_json::from_str::<Value>(input) {
        Ok(corpus) => audit_corpus(&corpus),
        Err(error) => AuditReport::new(
            0,
            vec![Finding {
                code: "PIRB_JSON_INVALID".to_owned(),
                path: "$".to_owned(),
                message: format!("Corpus is not valid JSON: {error}"),
            }],
        ),
    }
}

pub fn audit_corpus(corpus: &Value) -> AuditReport {
    let mut findings = Vec::new();
    let Some(root) = corpus.as_object() else {
        return AuditReport::new(
            0,
            vec![finding(
                "PIRB_ROOT_NOT_OBJECT",
                "$",
                "Corpus root must be a JSON object.",
            )],
        );
    };

    require_equal(
        root.get("corpus_version"),
        &Value::String("0.1".to_owned()),
        "PIRB_VERSION_INVALID",
        "$.corpus_version",
        "Corpus version must be 0.1.",
        &mut findings,
    );
    require_equal(
        root.get("corpus_kind"),
        &Value::String("synthetic_reference_only".to_owned()),
        "PIRB_CORPUS_KIND_INVALID",
        "$.corpus_kind",
        "Only the explicitly synthetic reference corpus is accepted by this audit.",
        &mut findings,
    );
    require_equal(
        root.get("field_evidence"),
        &Value::Bool(false),
        "PIRB_FIELD_EVIDENCE_CLAIM",
        "$.field_evidence",
        "The fixture corpus must not claim to contain field evidence.",
        &mut findings,
    );
    require_equal(
        root.get("quantities_are_policy_thresholds"),
        &Value::Bool(false),
        "PIRB_POLICY_THRESHOLD_CLAIM",
        "$.quantities_are_policy_thresholds",
        "Synthetic quantities must not be represented as policy thresholds.",
        &mut findings,
    );

    let Some(scenarios) = root.get("scenarios").and_then(Value::as_array) else {
        findings.push(finding(
            "PIRB_SCENARIOS_MISSING",
            "$.scenarios",
            "scenarios must be an array.",
        ));
        return AuditReport::new(0, findings);
    };

    let mut seen_ids = BTreeSet::new();
    let mut found_required = BTreeSet::new();

    for (index, scenario) in scenarios.iter().enumerate() {
        let path = format!("$.scenarios[{index}]");
        let Some(object) = scenario.as_object() else {
            findings.push(finding(
                "PIRB_SCENARIO_NOT_OBJECT",
                &path,
                "Each scenario must be a JSON object.",
            ));
            continue;
        };

        let Some(id) = object.get("id").and_then(Value::as_str) else {
            findings.push(finding(
                "PIRB_SCENARIO_ID_MISSING",
                &format!("{path}.id"),
                "Every scenario needs a string id.",
            ));
            continue;
        };

        if !seen_ids.insert(id.to_owned()) {
            findings.push(finding(
                "PIRB_SCENARIO_ID_DUPLICATE",
                &format!("{path}.id"),
                "Scenario identifiers must be unique.",
            ));
        }

        if !REQUIRED_SCENARIOS.contains(&id) {
            findings.push(finding(
                "PIRB_SCENARIO_UNSUPPORTED",
                &format!("{path}.id"),
                "Unknown scenario id: add an explicit versioned evaluator before relying on this case.",
            ));
            continue;
        }
        found_required.insert(id.to_owned());

        let track = object.get("track").and_then(Value::as_str);
        if !track.is_some_and(|value| ALLOWED_TRACKS.contains(&value)) {
            findings.push(finding(
                "PIRB_TRACK_INVALID",
                &format!("{path}.track"),
                "Scenario track must be one of PIRB-0 through PIRB-5.",
            ));
        }

        if object.get("domain").and_then(Value::as_str).is_none() {
            findings.push(finding(
                "PIRB_DOMAIN_MISSING",
                &format!("{path}.domain"),
                "Scenario domain must be a non-empty string.",
            ));
        }

        if !object.get("expected").is_some_and(Value::is_object) {
            findings.push(finding(
                "PIRB_EXPECTED_MISSING",
                &format!("{path}.expected"),
                "Scenario must provide an expected-result object.",
            ));
            continue;
        }

        match id {
            "pirb-r2-minimum-service-floor-01" => audit_service_floor(scenario, &path, &mut findings),
            "pirb-r1-required-acceptance-01" => audit_required_acceptance(scenario, &path, &mut findings),
            "pirb-r0-stale-capacity-01" => audit_stale_evidence(scenario, &path, &mut findings),
            "pirb-r0-conflicting-reports-01" => audit_conflicting_reports(scenario, &path, &mut findings),
            "pirb-r4-personal-data-boundary-01" => audit_privacy_boundary(scenario, &path, &mut findings),
            "pirb-r3-no-lookahead-01" => audit_no_lookahead(scenario, &path, &mut findings),
            "pirb-r1-refused-participation-01" => audit_refusal(scenario, &path, &mut findings),
            _ => unreachable!("known scenario set is exhaustive"),
        }
    }

    for required in REQUIRED_SCENARIOS {
        if !found_required.contains(required) {
            findings.push(finding(
                "PIRB_REQUIRED_SCENARIO_MISSING",
                "$.scenarios",
                &format!("Required reference scenario is missing: {required}"),
            ));
        }
    }

    AuditReport::new(scenarios.len(), findings)
}

fn audit_service_floor(scenario: &Value, path: &str, findings: &mut Vec<Finding>) {
    let Some(floors) = scenario.pointer("/minimum_service_floor").and_then(Value::as_object) else {
        findings.push(finding("PIRB_FLOOR_MISSING", path, "minimum_service_floor must be an object."));
        return;
    };
    let Some(candidates) = scenario.get("candidates").and_then(Value::as_array) else {
        findings.push(finding("PIRB_CANDIDATES_MISSING", path, "candidates must be an array."));
        return;
    };

    let mut calculated_eligible = Vec::new();
    let mut calculated_rejected = Vec::new();
    let mut candidate_ids = BTreeSet::new();

    for (index, candidate) in candidates.iter().enumerate() {
        let candidate_path = format!("{path}.candidates[{index}]");
        let Some(id) = candidate.get("id").and_then(Value::as_str) else {
            findings.push(finding("PIRB_CANDIDATE_ID_MISSING", &candidate_path, "Candidate requires a string id."));
            continue;
        };
        if !candidate_ids.insert(id.to_owned()) {
            findings.push(finding("PIRB_CANDIDATE_ID_DUPLICATE", &candidate_path, "Candidate ids must be unique."));
        }

        let Some(allocation) = candidate.get("allocation").and_then(Value::as_object) else {
            findings.push(finding("PIRB_ALLOCATION_MISSING", &candidate_path, "Candidate allocation must be an object."));
            calculated_rejected.push(id.to_owned());
            continue;
        };

        let mut feasible = true;
        for (community, floor_value) in floors {
            let floor = floor_value.as_i64();
            let amount = allocation.get(community).and_then(Value::as_i64);
            match (floor, amount) {
                (Some(floor), Some(amount)) if amount >= floor => {}
                (Some(_), Some(_)) => feasible = false,
                _ => {
                    feasible = false;
                    findings.push(finding(
                        "PIRB_ALLOCATION_UNVERIFIABLE",
                        &format!("{candidate_path}.allocation.{community}"),
                        "Missing or non-integer allocation/floor values cannot satisfy a mandatory floor.",
                    ));
                }
            }
        }

        if allocation.keys().any(|community| !floors.contains_key(community)) {
            findings.push(finding(
                "PIRB_ALLOCATION_UNKNOWN_COMMUNITY",
                &format!("{candidate_path}.allocation"),
                "Allocation contains a community without a declared minimum service floor.",
            ));
            feasible = false;
        }

        let computed_total: Option<i64> = allocation.values().try_fold(0_i64, |sum, value| {
            sum.checked_add(value.as_i64()?)
        });
        match (candidate.get("total").and_then(Value::as_i64), computed_total) {
            (Some(declared), Some(computed)) if declared == computed => {}
            _ => findings.push(finding(
                "PIRB_ALLOCATION_TOTAL_MISMATCH",
                &format!("{candidate_path}.total"),
                "Declared candidate total must equal the checked sum of its integer allocations.",
            )),
        }

        if feasible {
            calculated_eligible.push(id.to_owned());
        } else {
            calculated_rejected.push(id.to_owned());
        }
    }

    calculated_eligible.sort();
    calculated_rejected.sort();
    let mut expected_eligible = string_list(scenario.pointer("/expected/eligible_candidates"));
    let mut expected_rejected = string_list(scenario.pointer("/expected/rejected_candidates"));
    expected_eligible.sort();
    expected_rejected.sort();

    if calculated_eligible != expected_eligible {
        findings.push(finding(
            "PIRB_ELIGIBILITY_ORACLE_MISMATCH",
            &format!("{path}.expected.eligible_candidates"),
            "Declared eligible candidates do not match independently computed hard-floor eligibility.",
        ));
    }
    if calculated_rejected != expected_rejected {
        findings.push(finding(
            "PIRB_REJECTION_ORACLE_MISMATCH",
            &format!("{path}.expected.rejected_candidates"),
            "Declared rejected candidates do not match independently computed hard-floor eligibility.",
        ));
    }
}

fn audit_required_acceptance(scenario: &Value, path: &str, findings: &mut Vec<Finding>) {
    let required = string_list(scenario.get("required_parties"));
    let accepted: BTreeSet<String> = string_list(scenario.get("accepted_by")).into_iter().collect();
    let missing: Vec<_> = required.iter().filter(|party| !accepted.contains(*party)).cloned().collect();
    let state = scenario.pointer("/expected/state").and_then(Value::as_str);

    if required.is_empty() {
        findings.push(finding("PIRB_REQUIRED_PARTIES_EMPTY", path, "Agreement requires at least one named party."));
    }
    if missing.is_empty() {
        findings.push(finding("PIRB_ACCEPTANCE_FIXTURE_NOT_ADVERSARIAL", path, "This fixture must retain a missing required acceptance."));
    }
    if state != Some("pending_not_active") {
        findings.push(finding(
            "PIRB_PREMATURE_AGREEMENT_ACTIVATION",
            &format!("{path}.expected.state"),
            "Agreement with missing required acceptances must remain pending and inactive.",
        ));
    }
}

fn audit_stale_evidence(scenario: &Value, path: &str, findings: &mut Vec<Finding>) {
    let freshness = scenario.pointer("/required_evidence/freshness_status").and_then(Value::as_str);
    let decision = scenario.pointer("/expected/decision").and_then(Value::as_str);
    if freshness != Some("stale") {
        findings.push(finding("PIRB_STALENESS_FIXTURE_INVALID", path, "Reference case must contain stale mandatory evidence."));
    }
    if decision != Some("blocked_or_unknown") {
        findings.push(finding(
            "PIRB_STALE_EVIDENCE_ACCEPTED",
            &format!("{path}.expected.decision"),
            "A consequential decision must not proceed on stale mandatory evidence.",
        ));
    }
}

fn audit_conflicting_reports(scenario: &Value, path: &str, findings: &mut Vec<Finding>) {
    let claims: BTreeSet<String> = scenario
        .get("reports")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|report| report.get("claim").and_then(Value::as_str).map(str::to_owned))
        .collect();
    let state = scenario.pointer("/expected/evidence_state").and_then(Value::as_str);

    if claims.len() < 2 {
        findings.push(finding(
            "PIRB_CONFLICT_FIXTURE_INVALID",
            &format!("{path}.reports"),
            "Reference case must include at least two distinct claims.",
        ));
    }
    if state != Some("contested_unknown") {
        findings.push(finding(
            "PIRB_CONFLICT_COLLAPSED",
            &format!("{path}.expected.evidence_state"),
            "Contradictory uncorroborated reports must remain contested/unknown.",
        ));
    }
}

fn audit_privacy_boundary(scenario: &Value, path: &str, findings: &mut Vec<Finding>) {
    let input: BTreeSet<String> = string_list(scenario.get("input_fields")).into_iter().collect();
    let prohibited = string_list(scenario.get("prohibited_shared_fields"));
    let has_prohibited_input = prohibited.iter().any(|field| input.contains(field));
    let decision = scenario.pointer("/expected/shared_record_decision").and_then(Value::as_str);

    if prohibited.is_empty() || !has_prohibited_input {
        findings.push(finding(
            "PIRB_PRIVACY_FIXTURE_INVALID",
            path,
            "Reference case must exercise at least one prohibited shared-data field.",
        ));
    }
    if decision != Some("reject_or_redact_under_explicit_policy") {
        findings.push(finding(
            "PIRB_PRIVACY_BOUNDARY_BYPASSED",
            &format!("{path}.expected.shared_record_decision"),
            "Prohibited personal or sensitive fields must be rejected or redacted under explicit policy.",
        ));
    }
}

fn audit_no_lookahead(scenario: &Value, path: &str, findings: &mut Vec<Finding>) {
    let stages = scenario.get("stages").and_then(Value::as_array);
    let Some(stages) = stages else {
        findings.push(finding("PIRB_STAGES_MISSING", path, "Adaptive policy requires ordered stages."));
        return;
    };
    if stages.len() < 2 {
        findings.push(finding("PIRB_STAGES_INSUFFICIENT", path, "Reference case requires pre- and post-inspection stages."));
        return;
    }

    let initial = &stages[0];
    let later = &stages[1];
    let initial_inputs: BTreeSet<String> = string_list(initial.get("observable_inputs")).into_iter().collect();
    let later_inputs: BTreeSet<String> = string_list(later.get("observable_inputs")).into_iter().collect();
    let initial_available = initial.get("future_observation_available").and_then(Value::as_bool);
    let later_available = later.get("future_observation_available").and_then(Value::as_bool);

    if initial_available != Some(false) || initial_inputs.contains("inspection_result") {
        findings.push(finding(
            "PIRB_LOOKAHEAD_LEAK",
            &format!("{path}.stages[0]"),
            "Initial decision uses or claims access to a future inspection result.",
        ));
    }
    if later_available != Some(true) || !later_inputs.contains("inspection_result") {
        findings.push(finding(
            "PIRB_POST_INSPECTION_EVIDENCE_MISSING",
            &format!("{path}.stages[1]"),
            "Post-inspection stage must make the declared inspection result observable.",
        ));
    }
}

fn audit_refusal(scenario: &Value, path: &str, findings: &mut Vec<Finding>) {
    let required = string_list(scenario.get("required_parties"));
    let accepted: BTreeSet<String> = string_list(scenario.get("accepted_by")).into_iter().collect();
    let refused = string_list(scenario.get("explicit_refusal_by"));
    let state = scenario.pointer("/expected/state").and_then(Value::as_str);

    if refused.is_empty() {
        findings.push(finding("PIRB_REFUSAL_FIXTURE_INVALID", path, "Reference case must explicitly record refusal."));
    }
    if refused.iter().any(|party| accepted.contains(party)) {
        findings.push(finding(
            "PIRB_REFUSAL_REWRITTEN_AS_CONSENT",
            &format!("{path}.accepted_by"),
            "A party cannot be both refusing and accepted in the same fixture state.",
        ));
    }
    if required.iter().any(|party| !accepted.contains(party)) && state != Some("not_active") {
        findings.push(finding(
            "PIRB_REFUSED_AGREEMENT_ACTIVATED",
            &format!("{path}.expected.state"),
            "Non-participation or refusal cannot be treated as consent.",
        ));
    }
}

fn string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn require_equal(
    actual: Option<&Value>,
    expected: &Value,
    code: &str,
    path: &str,
    message: &str,
    findings: &mut Vec<Finding>,
) {
    if actual != Some(expected) {
        findings.push(finding(code, path, message));
    }
}

fn finding(code: &str, path: &str, message: &str) -> Finding {
    Finding {
        code: code.to_owned(),
        path: path.to_owned(),
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const FIXTURE: &str =
        include_str!("../../../docs/fixtures/peace-infrastructure-resilience-v0.1.json");

    fn corpus() -> Value {
        serde_json::from_str(FIXTURE).expect("checked-in PIRB fixture must parse as JSON")
    }

    fn has_code(report: &AuditReport, code: &str) -> bool {
        report.findings.iter().any(|finding| finding.code == code)
    }

    #[test]
    fn reference_corpus_passes_all_semantic_checks() {
        let report = audit_corpus_json(FIXTURE);
        assert!(report.passed, "{:#?}", report.findings);
        assert_eq!(report.scenario_count, 7);
    }

    #[test]
    fn malformed_json_fails_closed() {
        let report = audit_corpus_json("{");
        assert!(!report.passed);
        assert!(has_code(&report, "PIRB_JSON_INVALID"));
    }

    #[test]
    fn missing_required_scenario_is_rejected() {
        let mut value = corpus();
        value["scenarios"].as_array_mut().unwrap().pop();
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_REQUIRED_SCENARIO_MISSING"));
    }

    #[test]
    fn duplicate_scenario_id_is_rejected() {
        let mut value = corpus();
        let first = value["scenarios"][0].clone();
        value["scenarios"].as_array_mut().unwrap().push(first);
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_SCENARIO_ID_DUPLICATE"));
    }

    #[test]
    fn field_evidence_claim_is_rejected() {
        let mut value = corpus();
        value["field_evidence"] = json!(true);
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_FIELD_EVIDENCE_CLAIM"));
    }

    #[test]
    fn allocation_cannot_trade_away_a_service_floor() {
        let mut value = corpus();
        value["scenarios"][0]["candidates"][0]["allocation"]["community_a"] = json!(9);
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_ELIGIBILITY_ORACLE_MISMATCH"));
    }

    #[test]
    fn agreement_with_missing_acceptance_cannot_be_active() {
        let mut value = corpus();
        value["scenarios"][1]["expected"]["state"] = json!("active");
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_PREMATURE_AGREEMENT_ACTIVATION"));
    }

    #[test]
    fn stale_capacity_cannot_authorize_action() {
        let mut value = corpus();
        value["scenarios"][2]["expected"]["decision"] = json!("proceed");
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_STALE_EVIDENCE_ACCEPTED"));
    }

    #[test]
    fn conflicting_reports_cannot_be_collapsed_to_known() {
        let mut value = corpus();
        value["scenarios"][3]["expected"]["evidence_state"] = json!("service_available");
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_CONFLICT_COLLAPSED"));
    }

    #[test]
    fn prohibited_fields_cannot_be_approved_for_shared_record() {
        let mut value = corpus();
        value["scenarios"][4]["expected"]["shared_record_decision"] = json!("allow");
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_PRIVACY_BOUNDARY_BYPASSED"));
    }

    #[test]
    fn adaptive_policy_cannot_see_future_inspection_result() {
        let mut value = corpus();
        value["scenarios"][5]["stages"][0]["observable_inputs"]
            .as_array_mut()
            .unwrap()
            .push(json!("inspection_result"));
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_LOOKAHEAD_LEAK"));
    }

    #[test]
    fn refusal_cannot_be_rewritten_as_acceptance() {
        let mut value = corpus();
        value["scenarios"][6]["accepted_by"]
            .as_array_mut()
            .unwrap()
            .push(json!("community-b"));
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_REFUSAL_REWRITTEN_AS_CONSENT"));
    }
}
