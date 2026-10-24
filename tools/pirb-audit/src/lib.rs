// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const REQUIRED_SCENARIOS: [&str; 12] = [
    "pirb-r2-minimum-service-floor-01",
    "pirb-r1-required-acceptance-01",
    "pirb-r0-stale-capacity-01",
    "pirb-r0-conflicting-reports-01",
    "pirb-r4-personal-data-boundary-01",
    "pirb-r3-no-lookahead-01",
    "pirb-r1-refused-participation-01",
    "pirb-r3-worst-case-frontier-01",
    "pirb-r5-displacement-not-net-impact-01",
    "pirb-r5-warning-response-chain-01",
    "pirb-r5-network-spillover-01",
    "pirb-r5-adaptive-stop-rule-01",
];

const ALLOWED_TRACKS: [&str; 6] = ["PIRB-0", "PIRB-1", "PIRB-2", "PIRB-3", "PIRB-4", "PIRB-5"];

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

/// Validates both the schema document and a corpus instance against Draft 2020-12.
///
/// Schema validity is checked first. A malformed schema, malformed corpus, or
/// any instance validation error fails closed. This is structural validation;
/// domain-specific semantic checks remain the responsibility of the auditor.
pub fn validate_corpus_schema(schema_input: &str, corpus_input: &str) -> Result<(), Vec<String>> {
    let schema: Value = serde_json::from_str(schema_input)
        .map_err(|error| vec![format!("schema JSON is invalid: {error}")])?;
    jsonschema::draft202012::meta::validate(&schema).map_err(|error| {
        vec![format!(
            "Draft 2020-12 schema meta-validation failed: {error}"
        )]
    })?;

    let corpus: Value = serde_json::from_str(corpus_input)
        .map_err(|error| vec![format!("corpus JSON is invalid: {error}")])?;
    let validator = jsonschema::draft202012::new(&schema)
        .map_err(|error| vec![format!("cannot build Draft 2020-12 validator: {error}")])?;
    let errors: Vec<String> = validator
        .iter_errors(&corpus)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
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
                "Unknown scenario id; add a versioned evaluator before relying on it.",
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
        let expected_track = match id {
            "pirb-r2-minimum-service-floor-01" => "PIRB-2",
            "pirb-r1-required-acceptance-01" | "pirb-r1-refused-participation-01" => "PIRB-1",
            "pirb-r0-stale-capacity-01" | "pirb-r0-conflicting-reports-01" => "PIRB-0",
            "pirb-r4-personal-data-boundary-01" => "PIRB-4",
            "pirb-r3-no-lookahead-01" | "pirb-r3-worst-case-frontier-01" => "PIRB-3",
            "pirb-r5-displacement-not-net-impact-01"
            | "pirb-r5-warning-response-chain-01"
            | "pirb-r5-network-spillover-01"
            | "pirb-r5-adaptive-stop-rule-01" => "PIRB-5",
            _ => "UNKNOWN",
        };
        if track != Some(expected_track) {
            findings.push(finding(
                "PIRB_TRACK_SCENARIO_MISMATCH",
                &format!("{path}.track"),
                "Reference scenario is assigned to the wrong benchmark track.",
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
            "pirb-r2-minimum-service-floor-01" => {
                audit_service_floor(scenario, &path, &mut findings)
            }
            "pirb-r1-required-acceptance-01" => {
                audit_required_acceptance(scenario, &path, &mut findings)
            }
            "pirb-r0-stale-capacity-01" => audit_stale_evidence(scenario, &path, &mut findings),
            "pirb-r0-conflicting-reports-01" => {
                audit_conflicting_reports(scenario, &path, &mut findings)
            }
            "pirb-r4-personal-data-boundary-01" => {
                audit_privacy_boundary(scenario, &path, &mut findings)
            }
            "pirb-r3-no-lookahead-01" => audit_no_lookahead(scenario, &path, &mut findings),
            "pirb-r1-refused-participation-01" => audit_refusal(scenario, &path, &mut findings),
            "pirb-r3-worst-case-frontier-01" => {
                audit_robust_frontier(scenario, &path, &mut findings)
            }
            "pirb-r5-displacement-not-net-impact-01" => {
                audit_displacement(scenario, &path, &mut findings)
            }
            "pirb-r5-warning-response-chain-01" => {
                audit_warning_response_chain(scenario, &path, &mut findings)
            }
            "pirb-r5-network-spillover-01" => audit_displacement(scenario, &path, &mut findings),
            "pirb-r5-adaptive-stop-rule-01" => {
                audit_adaptive_stop_rules(scenario, &path, &mut findings)
            }
            _ => findings.push(finding(
                "PIRB_EVALUATOR_MISSING",
                &format!("{path}.id"),
                "Known scenario has no evaluator; refusing to treat it as verified.",
            )),
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
    let Some(floors) = scenario
        .pointer("/minimum_service_floor")
        .and_then(Value::as_object)
    else {
        findings.push(finding(
            "PIRB_FLOOR_MISSING",
            path,
            "minimum_service_floor must be an object.",
        ));
        return;
    };
    let Some(candidates) = scenario.get("candidates").and_then(Value::as_array) else {
        findings.push(finding(
            "PIRB_CANDIDATES_MISSING",
            path,
            "candidates must be an array.",
        ));
        return;
    };

    let mut calculated_eligible = Vec::new();
    let mut calculated_rejected = Vec::new();
    let mut candidate_ids = BTreeSet::new();

    for (index, candidate) in candidates.iter().enumerate() {
        let candidate_path = format!("{path}.candidates[{index}]");
        let Some(id) = candidate.get("id").and_then(Value::as_str) else {
            findings.push(finding(
                "PIRB_CANDIDATE_ID_MISSING",
                &candidate_path,
                "Candidate requires a string id.",
            ));
            continue;
        };
        if !candidate_ids.insert(id.to_owned()) {
            findings.push(finding(
                "PIRB_CANDIDATE_ID_DUPLICATE",
                &candidate_path,
                "Candidate ids must be unique.",
            ));
        }

        let Some(allocation) = candidate.get("allocation").and_then(Value::as_object) else {
            findings.push(finding(
                "PIRB_ALLOCATION_MISSING",
                &candidate_path,
                "Candidate allocation must be an object.",
            ));
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
                        "Missing or non-integer allocation/floor values fail the mandatory floor.",
                    ));
                }
            }
        }

        if allocation
            .keys()
            .any(|community| !floors.contains_key(community))
        {
            findings.push(finding(
                "PIRB_ALLOCATION_UNKNOWN_COMMUNITY",
                &format!("{candidate_path}.allocation"),
                "Allocation contains a community without a declared minimum service floor.",
            ));
            feasible = false;
        }

        let computed_total: Option<i64> = allocation
            .values()
            .try_fold(0_i64, |sum, value| sum.checked_add(value.as_i64()?));
        match (
            candidate.get("total").and_then(Value::as_i64),
            computed_total,
        ) {
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
            "Eligible candidates disagree with the computed hard-floor result.",
        ));
    }
    if calculated_rejected != expected_rejected {
        findings.push(finding(
            "PIRB_REJECTION_ORACLE_MISMATCH",
            &format!("{path}.expected.rejected_candidates"),
            "Rejected candidates disagree with the computed hard-floor result.",
        ));
    }
}

fn audit_required_acceptance(scenario: &Value, path: &str, findings: &mut Vec<Finding>) {
    let required = string_list(scenario.get("required_parties"));
    let accepted: BTreeSet<String> = string_list(scenario.get("accepted_by"))
        .into_iter()
        .collect();
    let missing: Vec<_> = required
        .iter()
        .filter(|party| !accepted.contains(*party))
        .cloned()
        .collect();
    let state = scenario.pointer("/expected/state").and_then(Value::as_str);

    if required.is_empty() {
        findings.push(finding(
            "PIRB_REQUIRED_PARTIES_EMPTY",
            path,
            "Agreement requires at least one named party.",
        ));
    }
    if missing.is_empty() {
        findings.push(finding(
            "PIRB_ACCEPTANCE_FIXTURE_NOT_ADVERSARIAL",
            path,
            "Fixture must retain a missing required acceptance.",
        ));
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
    let freshness = scenario
        .pointer("/required_evidence/freshness_status")
        .and_then(Value::as_str);
    let decision = scenario
        .pointer("/expected/decision")
        .and_then(Value::as_str);
    if freshness != Some("stale") {
        findings.push(finding(
            "PIRB_STALENESS_FIXTURE_INVALID",
            path,
            "Reference case must contain stale mandatory evidence.",
        ));
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
        .filter_map(|report| {
            report
                .get("claim")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .collect();
    let state = scenario
        .pointer("/expected/evidence_state")
        .and_then(Value::as_str);

    if claims.len() < 2 {
        findings.push(finding(
            "PIRB_CONFLICT_FIXTURE_INVALID",
            &format!("{path}.reports"),
            "Reference case must include at least two distinct claims.",
        ));
    }
    let all_uncorroborated = scenario
        .get("reports")
        .and_then(Value::as_array)
        .is_some_and(|reports| {
            reports
                .iter()
                .all(|report| report.get("corroborated").and_then(Value::as_bool) == Some(false))
        });
    if !all_uncorroborated {
        findings.push(finding(
            "PIRB_CONFLICT_FIXTURE_CORROBORATION_INVALID",
            &format!("{path}.reports"),
            "This reference case is specifically for conflicting uncorroborated reports.",
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
    let input: BTreeSet<String> = string_list(scenario.get("input_fields"))
        .into_iter()
        .collect();
    let prohibited = string_list(scenario.get("prohibited_shared_fields"));
    let shared: BTreeSet<String> = string_list(scenario.get("proposed_shared_record_fields"))
        .into_iter()
        .collect();
    let has_prohibited_input = prohibited.iter().any(|field| input.contains(field));
    let leaked: Vec<_> = prohibited
        .iter()
        .filter(|field| shared.contains(*field))
        .cloned()
        .collect();
    let decision = scenario
        .pointer("/expected/shared_record_decision")
        .and_then(Value::as_str);

    if prohibited.is_empty() || !has_prohibited_input {
        findings.push(finding(
            "PIRB_PRIVACY_FIXTURE_INVALID",
            path,
            "Reference case must exercise at least one prohibited shared-data field.",
        ));
    }
    if !leaked.is_empty() {
        findings.push(finding(
            "PIRB_SENSITIVE_FIELD_LEAK",
            &format!("{path}.proposed_shared_record_fields"),
            "A prohibited field appears in the proposed shared record.",
        ));
    }
    if decision != Some("reject_or_redact_under_explicit_policy") {
        findings.push(finding(
            "PIRB_PRIVACY_BOUNDARY_BYPASSED",
            &format!("{path}.expected.shared_record_decision"),
            "Prohibited personal fields must be rejected or explicitly redacted.",
        ));
    }
}

fn audit_no_lookahead(scenario: &Value, path: &str, findings: &mut Vec<Finding>) {
    let stages = scenario.get("stages").and_then(Value::as_array);
    let Some(stages) = stages else {
        findings.push(finding(
            "PIRB_STAGES_MISSING",
            path,
            "Adaptive policy requires ordered stages.",
        ));
        return;
    };
    if stages.len() < 2 {
        findings.push(finding(
            "PIRB_STAGES_INSUFFICIENT",
            path,
            "Reference case requires pre- and post-inspection stages.",
        ));
        return;
    }

    let initial = &stages[0];
    let later = &stages[1];
    let initial_inputs: BTreeSet<String> = string_list(initial.get("observable_inputs"))
        .into_iter()
        .collect();
    let later_inputs: BTreeSet<String> = string_list(later.get("observable_inputs"))
        .into_iter()
        .collect();
    let initial_available = initial
        .get("future_observation_available")
        .and_then(Value::as_bool);
    let later_available = later
        .get("future_observation_available")
        .and_then(Value::as_bool);

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
    let accepted: BTreeSet<String> = string_list(scenario.get("accepted_by"))
        .into_iter()
        .collect();
    let refused = string_list(scenario.get("explicit_refusal_by"));
    let state = scenario.pointer("/expected/state").and_then(Value::as_str);

    if refused.is_empty() {
        findings.push(finding(
            "PIRB_REFUSAL_FIXTURE_INVALID",
            path,
            "Reference case must explicitly record refusal.",
        ));
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

fn audit_robust_frontier(scenario: &Value, path: &str, findings: &mut Vec<Finding>) {
    let scenario_ids = string_list(scenario.get("scenario_ids"));
    if scenario_ids.is_empty()
        || scenario_ids.iter().collect::<BTreeSet<_>>().len() != scenario_ids.len()
    {
        findings.push(finding(
            "PIRB_ROBUST_SCENARIOS_INVALID",
            &format!("{path}.scenario_ids"),
            "Robustness scenarios must be non-empty and uniquely named.",
        ));
        return;
    }
    if scenario
        .get("scenario_probabilities_assigned")
        .and_then(Value::as_bool)
        != Some(false)
    {
        findings.push(finding(
            "PIRB_SYNTHETIC_PROBABILITY_CLAIM",
            &format!("{path}.scenario_probabilities_assigned"),
            "Named stress scenarios must not imply empirical probabilities.",
        ));
    }
    let Some(options) = scenario.get("options").and_then(Value::as_array) else {
        findings.push(finding(
            "PIRB_OPTIONS_MISSING",
            path,
            "Robust plan options must be an array.",
        ));
        return;
    };

    let scenario_set: BTreeSet<String> = scenario_ids.into_iter().collect();
    let mut option_ids = BTreeSet::new();
    let mut feasible = Vec::<(String, i64, i64)>::new();
    let mut excluded = Vec::<String>::new();
    let mut worst_by_id = BTreeMap::<String, i64>::new();

    for (index, option) in options.iter().enumerate() {
        let option_path = format!("{path}.options[{index}]");
        let Some(id) = option.get("id").and_then(Value::as_str).map(str::to_owned) else {
            findings.push(finding(
                "PIRB_OPTION_ID_MISSING",
                &option_path,
                "Each option requires a string id.",
            ));
            continue;
        };
        if !option_ids.insert(id.clone()) {
            findings.push(finding(
                "PIRB_OPTION_ID_DUPLICATE",
                &option_path,
                "Option ids must be unique.",
            ));
        }

        let cost = option.get("cost_ticks").and_then(Value::as_i64);
        if cost.is_none_or(|value| value < 0) {
            findings.push(finding(
                "PIRB_OPTION_COST_INVALID",
                &format!("{option_path}.cost_ticks"),
                "Cost must be a non-negative integer tick count.",
            ));
        }

        let Some(gates) = option.get("hard_gates").and_then(Value::as_object) else {
            findings.push(finding(
                "PIRB_OPTION_GATES_MISSING",
                &option_path,
                "Every option must declare its hard gates.",
            ));
            excluded.push(id);
            continue;
        };
        let gate_values_valid = !gates.is_empty()
            && gates
                .values()
                .all(|gate| matches!(gate.as_str(), Some("pass") | Some("fail") | Some("unknown")));
        if !gate_values_valid {
            findings.push(finding(
                "PIRB_OPTION_GATE_INVALID",
                &format!("{option_path}.hard_gates"),
                "Hard gates must be non-empty and use pass/fail/unknown.",
            ));
            excluded.push(id);
            continue;
        }
        let gates_pass = gates.values().all(|gate| gate.as_str() == Some("pass"));

        let Some(gaps) = option
            .get("service_gap_by_scenario")
            .and_then(Value::as_object)
        else {
            findings.push(finding(
                "PIRB_OPTION_GAPS_MISSING",
                &option_path,
                "Every option requires per-scenario service gaps.",
            ));
            excluded.push(id);
            continue;
        };
        let gap_keys: BTreeSet<String> = gaps.keys().cloned().collect();
        if gap_keys != scenario_set {
            findings.push(finding(
                "PIRB_OPTION_SCENARIO_COVERAGE_MISMATCH",
                &format!("{option_path}.service_gap_by_scenario"),
                "Every option must cover exactly the declared stress scenarios.",
            ));
            excluded.push(id);
            continue;
        }
        let parsed_gaps: Option<Vec<i64>> = gaps.values().map(Value::as_i64).collect();
        let Some(parsed_gaps) = parsed_gaps else {
            findings.push(finding(
                "PIRB_OPTION_GAP_INVALID",
                &format!("{option_path}.service_gap_by_scenario"),
                "Service gaps must be integer values.",
            ));
            excluded.push(id);
            continue;
        };
        if parsed_gaps.iter().any(|gap| *gap < 0) {
            findings.push(finding(
                "PIRB_OPTION_GAP_NEGATIVE",
                &format!("{option_path}.service_gap_by_scenario"),
                "Service gaps cannot be negative.",
            ));
            excluded.push(id);
            continue;
        }
        let Some(worst_gap) = parsed_gaps.iter().copied().max() else {
            findings.push(finding(
                "PIRB_OPTION_GAPS_EMPTY",
                &option_path,
                "An option needs at least one scenario gap.",
            ));
            excluded.push(id);
            continue;
        };
        worst_by_id.insert(id.clone(), worst_gap);

        if !gates_pass {
            excluded.push(id);
            continue;
        }
        if let Some(cost) = cost.filter(|value| *value >= 0) {
            feasible.push((id, cost, worst_gap));
        } else {
            excluded.push(id);
        }
    }

    let mut actual_feasible: Vec<String> = feasible.iter().map(|item| item.0.clone()).collect();
    let mut actual_excluded = excluded;
    actual_feasible.sort();
    actual_excluded.sort();
    let mut expected_feasible = string_list(scenario.pointer("/expected/feasible_options"));
    let mut expected_excluded = string_list(scenario.pointer("/expected/excluded_by_hard_gate"));
    expected_feasible.sort();
    expected_excluded.sort();

    if actual_feasible != expected_feasible {
        findings.push(finding(
            "PIRB_FEASIBILITY_ORACLE_MISMATCH",
            &format!("{path}.expected.feasible_options"),
            "Eligible options do not match independently computed hard-gate feasibility.",
        ));
    }
    if actual_excluded != expected_excluded {
        findings.push(finding(
            "PIRB_HARD_GATE_EXCLUSION_MISMATCH",
            &format!("{path}.expected.excluded_by_hard_gate"),
            "Excluded options do not match the independently computed gate result.",
        ));
    }

    let mut frontier = Vec::new();
    for (id, cost, worst_gap) in &feasible {
        let dominated = feasible.iter().any(|(other_id, other_cost, other_gap)| {
            other_id != id
                && *other_cost <= *cost
                && *other_gap <= *worst_gap
                && (*other_cost < *cost || *other_gap < *worst_gap)
        });
        if !dominated {
            frontier.push(id.clone());
        }
    }
    frontier.sort();
    let mut expected_frontier = string_list(scenario.pointer("/expected/pareto_frontier"));
    expected_frontier.sort();
    if frontier != expected_frontier {
        findings.push(finding(
            "PIRB_PARETO_FRONTIER_MISMATCH",
            &format!("{path}.expected.pareto_frontier"),
            "Declared cost/worst-case-gap Pareto frontier is incorrect.",
        ));
    }

    if scenario
        .pointer("/expected/scenario_probabilities_assigned")
        .and_then(Value::as_bool)
        != Some(false)
    {
        findings.push(finding(
            "PIRB_SYNTHETIC_PROBABILITY_CLAIM",
            &format!("{path}.expected.scenario_probabilities_assigned"),
            "Synthetic stress cases must not be represented as probabilistic forecasts.",
        ));
    }
    if let Some(expected_worst) = scenario
        .pointer("/expected/worst_case_service_gap")
        .and_then(Value::as_object)
    {
        for (id, actual) in worst_by_id {
            if expected_worst.get(&id).and_then(Value::as_i64) != Some(actual) {
                findings.push(finding(
                    "PIRB_WORST_CASE_GAP_MISMATCH",
                    &format!("{path}.expected.worst_case_service_gap.{id}"),
                    "Declared worst-case gap differs from the scenario maximum.",
                ));
            }
        }
    } else {
        findings.push(finding(
            "PIRB_WORST_CASE_GAP_MISSING",
            &format!("{path}.expected.worst_case_service_gap"),
            "Expected results must publish the calculated worst-case service gap per option.",
        ));
    }
}

fn audit_displacement(scenario: &Value, path: &str, findings: &mut Vec<Finding>) {
    let outcomes = scenario.get("outcomes").and_then(Value::as_array);
    let Some(outcomes) = outcomes else {
        findings.push(finding(
            "PIRB_OUTCOMES_MISSING",
            path,
            "Before/after outcomes must be an array.",
        ));
        return;
    };
    if outcomes.is_empty() {
        findings.push(finding(
            "PIRB_OUTCOMES_EMPTY",
            path,
            "At least one group outcome is required.",
        ));
        return;
    }

    let mut before_total = 0_i64;
    let mut after_total = 0_i64;
    let mut improved_group = false;
    let mut worsened_group = false;
    let mut improved_groups = Vec::new();
    let mut worsened_groups = Vec::new();

    for (index, outcome) in outcomes.iter().enumerate() {
        let outcome_path = format!("{path}.outcomes[{index}]");
        let group = outcome.get("group").and_then(Value::as_str);
        let Some(group) = group else {
            findings.push(finding(
                "PIRB_OUTCOME_GROUP_MISSING",
                &format!("{outcome_path}.group"),
                "Every group outcome must name its evaluation unit so burden shifts remain visible.",
            ));
            return;
        };
        let before = outcome.get("service_gap_before").and_then(Value::as_i64);
        let after = outcome.get("service_gap_after").and_then(Value::as_i64);
        let (Some(before), Some(after)) = (before, after) else {
            findings.push(finding(
                "PIRB_OUTCOME_VALUE_INVALID",
                &outcome_path,
                "Before/after gaps must be integers.",
            ));
            return;
        };
        if before < 0 || after < 0 {
            findings.push(finding(
                "PIRB_OUTCOME_VALUE_NEGATIVE",
                &outcome_path,
                "Service gaps cannot be negative.",
            ));
            return;
        }
        let Some(next_before) = before_total.checked_add(before) else {
            findings.push(finding(
                "PIRB_OUTCOME_OVERFLOW",
                &outcome_path,
                "Before-gap total overflowed.",
            ));
            return;
        };
        let Some(next_after) = after_total.checked_add(after) else {
            findings.push(finding(
                "PIRB_OUTCOME_OVERFLOW",
                &outcome_path,
                "After-gap total overflowed.",
            ));
            return;
        };
        before_total = next_before;
        after_total = next_after;
        if after < before {
            improved_group = true;
            improved_groups.push(group.to_owned());
        }
        if after > before {
            worsened_group = true;
            worsened_groups.push(group.to_owned());
        }
    }

    improved_groups.sort();
    worsened_groups.sort();
    let displacement_detected = improved_group && worsened_group;
    if scenario.get("design_type").and_then(Value::as_str) == Some("before_after_only")
        && scenario
            .get("causal_claim_requested")
            .and_then(Value::as_bool)
            == Some(true)
        && scenario
            .pointer("/expected/claim_status")
            .and_then(Value::as_str)
            != Some("descriptive_only")
    {
        findings.push(finding(
            "PIRB_CAUSAL_CLAIM_UNSUPPORTED",
            &format!("{path}.expected.claim_status"),
            "Before/after data with concurrent factors cannot establish causality.",
        ));
    }
    if scenario
        .get("concurrent_factors")
        .and_then(Value::as_array)
        .is_none_or(Vec::is_empty)
    {
        findings.push(finding(
            "PIRB_CONCURRENT_FACTORS_UNDECLARED",
            &format!("{path}.concurrent_factors"),
            "This adversarial case requires a disclosed concurrent factor.",
        ));
    }

    if scenario
        .pointer("/expected/aggregate_gap_before")
        .and_then(Value::as_i64)
        != Some(before_total)
        || scenario
            .pointer("/expected/aggregate_gap_after")
            .and_then(Value::as_i64)
            != Some(after_total)
    {
        findings.push(finding(
            "PIRB_AGGREGATE_GAP_MISMATCH",
            &format!("{path}.expected"),
            "Expected aggregate gaps must equal the sum of the declared group outcomes.",
        ));
    }
    if scenario
        .pointer("/expected/displacement_detected")
        .and_then(Value::as_bool)
        != Some(displacement_detected)
    {
        findings.push(finding(
            "PIRB_DISPLACEMENT_MISCLASSIFIED",
            &format!("{path}.expected.displacement_detected"),
            "Expected displacement status must match the per-group outcome changes.",
        ));
    }
    let mut expected_improved_groups = string_list(scenario.pointer("/expected/improved_groups"));
    let mut expected_worsened_groups = string_list(scenario.pointer("/expected/worsened_groups"));
    expected_improved_groups.sort();
    expected_worsened_groups.sort();
    if improved_groups != expected_improved_groups {
        findings.push(finding(
            "PIRB_IMPROVED_GROUPS_MISMATCH",
            &format!("{path}.expected.improved_groups"),
            "The declared groups with reduced service gaps must match the per-group observations.",
        ));
    }
    if worsened_groups != expected_worsened_groups {
        findings.push(finding(
            "PIRB_WORSENED_GROUPS_MISMATCH",
            &format!("{path}.expected.worsened_groups"),
            "The declared groups with increased service gaps must match the per-group observations.",
        ));
    }

    let expected_claim = scenario
        .pointer("/expected/improvement_claim")
        .and_then(Value::as_str);
    if worsened_group
        && after_total < before_total
        && expected_claim != Some("qualified_with_displacement")
    {
        findings.push(finding(
            "PIRB_DISTRIBUTIONAL_HARM_HIDDEN",
            &format!("{path}.expected.improvement_claim"),
            "An aggregate reduction alongside worsened groups must disclose distributional harm and cannot be labelled an unqualified improvement.",
        ));
    }
    if after_total >= before_total && expected_claim != Some("not_supported") {
        findings.push(finding(
            "PIRB_NET_IMPROVEMENT_UNSUPPORTED",
            &format!("{path}.expected.improvement_claim"),
            "No aggregate reduction is observed; an improvement claim is unsupported.",
        ));
    }
}

fn audit_adaptive_stop_rules(scenario: &Value, path: &str, findings: &mut Vec<Finding>) {
    let Some(cases) = scenario.get("stop_rule_cases").and_then(Value::as_array) else {
        findings.push(finding(
            "PIRB_STOP_RULE_CASES_MISSING",
            path,
            "Adaptive stop-rule scenario requires a stop_rule_cases array.",
        ));
        return;
    };
    if cases.is_empty() {
        findings.push(finding(
            "PIRB_STOP_RULE_CASES_EMPTY",
            &format!("{path}.stop_rule_cases"),
            "At least one positive control and one safety stop-rule case are required.",
        ));
        return;
    }

    let mut seen = BTreeSet::new();
    for (index, case) in cases.iter().enumerate() {
        let case_path = format!("{path}.stop_rule_cases[{index}]");
        let Some(case_id) = case.get("case_id").and_then(Value::as_str) else {
            findings.push(finding(
                "PIRB_STOP_RULE_CASE_ID_MISSING",
                &case_path,
                "Every stop-rule case requires a string case_id.",
            ));
            continue;
        };
        if !seen.insert(case_id.to_owned()) {
            findings.push(finding(
                "PIRB_STOP_RULE_CASE_ID_DUPLICATE",
                &format!("{case_path}.case_id"),
                "Stop-rule case identifiers must be unique.",
            ));
        }

        let state = case.get("current_state").and_then(Value::as_str);
        if state != Some("ACTIVE") {
            findings.push(finding(
                "PIRB_STOP_RULE_STATE_INVALID",
                &format!("{case_path}.current_state"),
                "This reference state machine only evaluates explicit ACTIVE interventions.",
            ));
        }

        let owner = case
            .get("accountable_owner_authorized")
            .and_then(Value::as_bool);
        if owner.is_none() {
            findings.push(finding(
                "PIRB_STOP_RULE_AUTHORITY_UNKNOWN",
                &format!("{case_path}.accountable_owner_authorized"),
                "Stop-rule authority status must be explicit; missing authority evidence blocks continuation.",
            ));
        }
        let scope_assessed = case.get("affected_scope_assessed").and_then(Value::as_bool);
        if scope_assessed.is_none() {
            findings.push(finding(
                "PIRB_STOP_RULE_SCOPE_UNKNOWN",
                &format!("{case_path}.affected_scope_assessed"),
                "Whether the declared affected scope was assessed must be explicit.",
            ));
        }
        let evidence_state = case.get("evidence_status").and_then(Value::as_str);
        if !matches!(
            evidence_state,
            Some("verified") | Some("stale") | Some("conflicting") | Some("unknown")
        ) {
            findings.push(finding(
                "PIRB_STOP_RULE_EVIDENCE_STATE_INVALID",
                &format!("{case_path}.evidence_status"),
                "Evidence status must be verified, stale, conflicting, or unknown.",
            ));
        }

        let trigger = case.get("trigger").and_then(Value::as_str);
        let known_trigger = matches!(
            trigger,
            Some("mandatory_floor_breached")
                | Some("mandatory_evidence_stale")
                | Some("mandatory_evidence_conflict")
                | Some("prohibited_sensitive_field")
                | Some("material_spillover_unassessed")
                | Some("metric_definition_changed_after_outcomes")
                | Some("appeal_suppressed")
                | Some("none")
        );
        if !known_trigger {
            findings.push(finding(
                "PIRB_STOP_RULE_TRIGGER_UNKNOWN",
                &format!("{case_path}.trigger"),
                "Unknown or missing triggers fail closed; register a versioned rule before use.",
            ));
        }

        let computed = if state != Some("ACTIVE")
            || owner != Some(true)
            || scope_assessed.is_none()
            || evidence_state != Some("verified")
        {
            "BLOCKED"
        } else {
            match trigger {
                Some("mandatory_floor_breached")
                | Some("mandatory_evidence_stale")
                | Some("mandatory_evidence_conflict")
                | Some("prohibited_sensitive_field") => "BLOCKED",
                Some("material_spillover_unassessed")
                | Some("metric_definition_changed_after_outcomes")
                | Some("appeal_suppressed") => "REVIEW_REQUIRED",
                Some("none") if scope_assessed == Some(true) => "CONTINUE",
                Some("none") => "REVIEW_REQUIRED",
                _ => "BLOCKED",
            }
        };

        let declared = case.get("expected_transition").and_then(Value::as_str);
        if declared != Some(computed) {
            findings.push(finding(
                "PIRB_STOP_RULE_TRANSITION_MISMATCH",
                &format!("{case_path}.expected_transition"),
                &format!(
                    "Computed safe transition is {computed}; the declared transition must match."
                ),
            ));
        }
    }

    if scenario
        .pointer("/expected/case_count")
        .and_then(Value::as_u64)
        != Some(cases.len() as u64)
    {
        findings.push(finding(
            "PIRB_STOP_RULE_CASE_COUNT_MISMATCH",
            &format!("{path}.expected.case_count"),
            "Declared stop-rule case count must equal the evaluated case count.",
        ));
    }

    if scenario
        .pointer("/expected/claim_ceiling")
        .and_then(Value::as_str)
        != Some(
            "Synthetic state-transition conformance only; no real intervention is authorized or controlled by this evaluator.",
        )
    {
        findings.push(finding(
            "PIRB_STOP_RULE_CLAIM_CEILING_INVALID",
            &format!("{path}.expected.claim_ceiling"),
            "Stop-rule evaluation must not be represented as a live control or deployment authorization.",
        ));
    }
}

fn audit_warning_response_chain(scenario: &Value, path: &str, findings: &mut Vec<Finding>) {
    let Some(alerts) = scenario.get("alerts").and_then(Value::as_array) else {
        findings.push(finding(
            "PIRB_ALERTS_MISSING",
            path,
            "Warning-response cases must contain an alerts array.",
        ));
        return;
    };

    let safety_buffer = scenario
        .get("required_safety_buffer_minutes")
        .and_then(Value::as_i64);
    if safety_buffer.is_none_or(|value| value < 0) {
        findings.push(finding(
            "PIRB_RESPONSE_BUFFER_INVALID",
            &format!("{path}.required_safety_buffer_minutes"),
            "A non-negative integer response safety buffer must be declared; missing or invalid reserves fail closed.",
        ));
    }

    let mut seen_ids = BTreeSet::new();
    let mut actionable_ids = Vec::new();
    let mut non_actionable_ids = Vec::new();
    let mut verified_ids = Vec::new();
    let mut unverified_ids = Vec::new();

    for (index, alert) in alerts.iter().enumerate() {
        let alert_path = format!("{path}.alerts[{index}]");
        let Some(id) = alert.get("alert_id").and_then(Value::as_str) else {
            findings.push(finding(
                "PIRB_ALERT_ID_MISSING",
                &alert_path,
                "Every alert requires a string identifier.",
            ));
            continue;
        };
        if !seen_ids.insert(id.to_owned()) {
            findings.push(finding(
                "PIRB_ALERT_ID_DUPLICATE",
                &alert_path,
                "Alert identifiers must be unique.",
            ));
        }

        let lead = alert.get("remaining_lead_minutes").and_then(Value::as_i64);
        let latency = alert
            .get("response_latency_bound_minutes")
            .and_then(Value::as_i64);
        let owner = alert
            .get("accountable_owner_authorized")
            .and_then(Value::as_bool);
        let resource = alert
            .get("response_resource_available")
            .and_then(Value::as_bool);
        let state = alert.get("response_state").and_then(Value::as_str);
        let response_evidence = alert
            .get("response_outcome_independently_verified")
            .and_then(Value::as_bool);

        if lead.is_none_or(|value| value < 0) || latency.is_none_or(|value| value < 0) {
            findings.push(finding(
                "PIRB_ALERT_TIMING_INVALID",
                &alert_path,
                "Lead time and response latency must be non-negative integers.",
            ));
        }
        if owner.is_none() || resource.is_none() {
            findings.push(finding(
                "PIRB_ALERT_CAPABILITY_UNKNOWN",
                &alert_path,
                "Authority and response-resource availability must be explicit.",
            ));
        }
        if !matches!(
            state,
            Some("unassigned") | Some("ready") | Some("dispatched") | Some("verified")
        ) {
            findings.push(finding(
                "PIRB_RESPONSE_STATE_INVALID",
                &format!("{alert_path}.response_state"),
                "Response state must be unassigned, ready, dispatched, or verified.",
            ));
        }
        if response_evidence.is_none() {
            findings.push(finding(
                "PIRB_RESPONSE_EVIDENCE_UNKNOWN",
                &format!("{alert_path}.response_outcome_independently_verified"),
                "Independent response evidence must be explicit.",
            ));
        }

        let actionable = match (owner, resource, lead, latency, safety_buffer) {
            (Some(true), Some(true), Some(lead), Some(latency), Some(buffer)) if buffer >= 0 => {
                latency
                    .checked_add(buffer)
                    .is_some_and(|required_minutes| lead > required_minutes)
            }
            _ => false,
        };
        if actionable {
            actionable_ids.push(id.to_owned());
        } else {
            non_actionable_ids.push(id.to_owned());
        }

        let event_trace_valid = if state == Some("verified") {
            audit_response_events(
                alert.get("response_events"),
                &format!("{alert_path}.response_events"),
                findings,
            )
        } else {
            false
        };
        let independently_verified = state == Some("verified")
            && actionable
            && response_evidence == Some(true)
            && event_trace_valid;
        if independently_verified {
            verified_ids.push(id.to_owned());
        } else {
            unverified_ids.push(id.to_owned());
        }

        if state == Some("verified") && !independently_verified {
            findings.push(finding(
                "PIRB_RESPONSE_VERIFICATION_UNSUPPORTED",
                &alert_path,
                "A verified response requires authority, resources, sufficient lead time, and independent outcome evidence.",
            ));
        }
        if response_evidence == Some(true) && state != Some("verified") {
            findings.push(finding(
                "PIRB_RESPONSE_STATE_EVIDENCE_MISMATCH",
                &alert_path,
                "Independent response evidence requires the verified response state.",
            ));
        }
    }

    actionable_ids.sort();
    non_actionable_ids.sort();
    verified_ids.sort();
    unverified_ids.sort();

    let mut expected_actionable = string_list(scenario.pointer("/expected/actionable_alert_ids"));
    let mut expected_non_actionable =
        string_list(scenario.pointer("/expected/non_actionable_alert_ids"));
    let mut expected_verified = string_list(scenario.pointer("/expected/verified_response_ids"));
    let mut expected_unverified =
        string_list(scenario.pointer("/expected/unverified_response_ids"));
    expected_actionable.sort();
    expected_non_actionable.sort();
    expected_verified.sort();
    expected_unverified.sort();

    if actionable_ids != expected_actionable || non_actionable_ids != expected_non_actionable {
        findings.push(finding(
            "PIRB_ACTIONABILITY_MISMATCH",
            &format!("{path}.expected"),
            "Expected actionable alerts must require authorized ownership, available resources, and lead time greater than response latency.",
        ));
    }
    if verified_ids != expected_verified || unverified_ids != expected_unverified {
        findings.push(finding(
            "PIRB_VERIFIED_RESPONSE_MISMATCH",
            &format!("{path}.expected"),
            "Response verification must agree with response state, timely feasibility, and independent outcome evidence.",
        ));
    }
    if scenario
        .pointer("/expected/prevention_claim")
        .and_then(Value::as_str)
        != Some("not_established_without_impact_design")
    {
        findings.push(finding(
            "PIRB_PREVENTION_CLAIM_UNSUPPORTED",
            &format!("{path}.expected.prevention_claim"),
            "An actionable or verified response is not proof that harm was prevented; causal impact needs a suitable evaluation design.",
        ));
    }
}

fn audit_response_events(value: Option<&Value>, path: &str, findings: &mut Vec<Finding>) -> bool {
    const REQUIRED_EVENTS: [&str; 6] = [
        "alert_issued",
        "acknowledged",
        "dispatched",
        "arrived",
        "action_completed",
        "outcome_verified",
    ];

    let Some(events) = value.and_then(Value::as_array) else {
        findings.push(finding(
            "PIRB_RESPONSE_EVENT_TRACE_MISSING",
            path,
            "A verified response requires a traceable sequence from alert issuance through independent outcome verification.",
        ));
        return false;
    };

    let mut valid = true;
    if events.len() != REQUIRED_EVENTS.len() {
        findings.push(finding(
            "PIRB_RESPONSE_EVENT_SEQUENCE_INVALID",
            path,
            "A response trace must contain exactly the six required lifecycle events in order.",
        ));
        valid = false;
    }

    let mut seen_ids = BTreeSet::new();
    let mut previous_minute: Option<i64> = None;
    for (index, event) in events.iter().enumerate() {
        let event_path = format!("{path}[{index}]");
        let event_id = event.get("event_id").and_then(Value::as_str);
        if event_id.is_none_or(str::is_empty)
            || !seen_ids.insert(event_id.unwrap_or_default().to_owned())
        {
            findings.push(finding(
                "PIRB_RESPONSE_EVENT_ID_INVALID",
                &format!("{event_path}.event_id"),
                "Every response event needs a non-empty, unique event identifier.",
            ));
            valid = false;
        }

        let event_type = event.get("event_type").and_then(Value::as_str);
        if REQUIRED_EVENTS.get(index).copied() != event_type {
            findings.push(finding(
                "PIRB_RESPONSE_EVENT_SEQUENCE_INVALID",
                &format!("{event_path}.event_type"),
                "Response events must follow the declared alert-to-verification lifecycle order.",
            ));
            valid = false;
        }

        let minute = event.get("minute_offset").and_then(Value::as_i64);
        match minute {
            Some(value) if value >= 0 => {
                if previous_minute.is_some_and(|previous| value < previous) {
                    findings.push(finding(
                        "PIRB_RESPONSE_EVENT_TIME_ORDER",
                        &format!("{event_path}.minute_offset"),
                        "Response event times must be non-negative and non-decreasing from alert issuance.",
                    ));
                    valid = false;
                }
                previous_minute = Some(value);
            }
            _ => {
                findings.push(finding(
                    "PIRB_RESPONSE_EVENT_TIME_INVALID",
                    &format!("{event_path}.minute_offset"),
                    "Every response event requires a non-negative integer minute offset.",
                ));
                valid = false;
            }
        }

        if event
            .get("evidence_ref")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            findings.push(finding(
                "PIRB_RESPONSE_EVENT_EVIDENCE_MISSING",
                &format!("{event_path}.evidence_ref"),
                "Every response lifecycle event must reference its supporting evidence record.",
            ));
            valid = false;
        }

        if event
            .get("independent_attestation")
            .and_then(Value::as_bool)
            .is_none()
        {
            findings.push(finding(
                "PIRB_RESPONSE_EVENT_ATTESTATION_UNKNOWN",
                &format!("{event_path}.independent_attestation"),
                "Independence status must be explicit for each response event.",
            ));
            valid = false;
        }
    }

    if events
        .last()
        .and_then(|event| event.get("independent_attestation"))
        .and_then(Value::as_bool)
        != Some(true)
    {
        findings.push(finding(
            "PIRB_RESPONSE_OUTCOME_ATTESTATION_MISSING",
            path,
            "The final outcome-verification event must carry an explicit independent attestation.",
        ));
        valid = false;
    }

    valid
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
    const SCHEMA: &str =
        include_str!("../../../docs/schemas/peace-infrastructure-resilience-v0.1.schema.json");

    fn corpus() -> Value {
        serde_json::from_str(FIXTURE).expect("checked-in PIRB fixture must parse as JSON")
    }

    fn has_code(report: &AuditReport, code: &str) -> bool {
        report.findings.iter().any(|finding| finding.code == code)
    }

    #[test]
    fn evaluator_is_invariant_to_scenario_and_alert_order() {
        let mut value = corpus();
        value["scenarios"].as_array_mut().unwrap().reverse();
        let warning = value
            .get_mut("scenarios")
            .unwrap()
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|scenario| {
                scenario.get("id").and_then(Value::as_str)
                    == Some("pirb-r5-warning-response-chain-01")
            })
            .unwrap();
        warning["alerts"].as_array_mut().unwrap().reverse();

        let report = audit_corpus(&value);
        assert!(report.passed, "{:#?}", report.findings);
    }

    #[test]
    fn duplicated_response_event_id_is_rejected_as_replay_or_collision() {
        let mut value = corpus();
        value["scenarios"][9]["alerts"][0]["response_events"][1]["event_id"] =
            json!("synthetic-event-001");
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_RESPONSE_EVENT_ID_INVALID"));
        assert!(has_code(&report, "PIRB_RESPONSE_VERIFICATION_UNSUPPORTED"));
    }

    #[test]
    fn checked_in_schema_is_valid_and_accepts_reference_corpus() {
        assert!(
            jsonschema::draft202012::meta::validate(
                &serde_json::from_str::<Value>(SCHEMA).expect("schema JSON parses")
            )
            .is_ok(),
            "checked-in Draft 2020-12 schema must validate against its meta-schema"
        );
        assert_eq!(validate_corpus_schema(SCHEMA, FIXTURE), Ok(()));
    }

    #[test]
    fn schema_rejects_corpus_version_mutation() {
        let mut value = corpus();
        value["corpus_version"] = json!("999");
        let mutated = serde_json::to_string(&value).expect("mutation serializes");
        let errors = validate_corpus_schema(SCHEMA, &mutated)
            .expect_err("schema must reject a wrong corpus version");
        assert!(!errors.is_empty());
    }

    #[test]
    fn malformed_schema_fails_closed() {
        let invalid_schema = SCHEMA.replacen(
            r#""type": "object""#,
            r#""type": "not-a-json-schema-type""#,
            1,
        );
        let errors = validate_corpus_schema(&invalid_schema, FIXTURE)
            .expect_err("invalid schema must not be used");
        assert!(!errors.is_empty());
        assert!(errors[0].contains("schema meta-validation failed"));
    }

    #[test]
    fn warning_response_chain_requires_authority_resources_and_lead_time() {
        let report = audit_corpus_json(FIXTURE);
        assert!(report.passed, "{:#?}", report.findings);
        assert_eq!(report.scenario_count, 12);
        let value = corpus();
        assert_eq!(
            value["scenarios"][9]["expected"]["actionable_alert_ids"],
            json!([
                "actionable-verified",
                "ready-not-executed",
                "dispatched-not-verified"
            ])
        );
    }

    #[test]
    fn warning_response_requires_a_declared_safety_buffer() {
        let mut value = corpus();
        value["scenarios"][9]["required_safety_buffer_minutes"] = Value::Null;
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_RESPONSE_BUFFER_INVALID"));
    }

    #[test]
    fn verified_response_requires_monotonic_event_history() {
        let mut value = corpus();
        value["scenarios"][9]["alerts"][0]["response_events"][3]["minute_offset"] = json!(19);
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_RESPONSE_EVENT_TIME_ORDER"));
        assert!(has_code(&report, "PIRB_RESPONSE_VERIFICATION_UNSUPPORTED"));
    }

    #[test]
    fn verified_response_requires_event_provenance() {
        let mut value = corpus();
        value["scenarios"][9]["alerts"][0]["response_events"][4]["evidence_ref"] = json!("");
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_RESPONSE_EVENT_EVIDENCE_MISSING"));
        assert!(has_code(&report, "PIRB_RESPONSE_VERIFICATION_UNSUPPORTED"));
    }

    #[test]
    fn verified_response_requires_independent_outcome_attestation() {
        let mut value = corpus();
        value["scenarios"][9]["alerts"][0]["response_events"][5]["independent_attestation"] =
            json!(false);
        let report = audit_corpus(&value);
        assert!(has_code(
            &report,
            "PIRB_RESPONSE_OUTCOME_ATTESTATION_MISSING"
        ));
        assert!(has_code(&report, "PIRB_RESPONSE_VERIFICATION_UNSUPPORTED"));
    }

    #[test]
    fn warning_without_authorized_owner_is_not_actionable() {
        let mut value = corpus();
        value["scenarios"][9]["alerts"][1]["accountable_owner_authorized"] = json!(true);
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_ACTIONABILITY_MISMATCH"));
    }

    #[test]
    fn verified_response_requires_independent_outcome_evidence() {
        let mut value = corpus();
        value["scenarios"][9]["alerts"][0]["response_outcome_independently_verified"] =
            json!(false);
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_RESPONSE_VERIFICATION_UNSUPPORTED"));
    }

    #[test]
    fn actionable_warning_does_not_prove_prevention() {
        let mut value = corpus();
        value["scenarios"][9]["expected"]["prevention_claim"] = json!("prevention_proven");
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_PREVENTION_CLAIM_UNSUPPORTED"));
    }

    #[test]
    fn reference_corpus_passes_all_semantic_checks() {
        let report = audit_corpus_json(FIXTURE);
        assert!(report.passed, "{:#?}", report.findings);
        assert_eq!(report.scenario_count, 12);
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
        assert!(value["scenarios"].as_array_mut().unwrap().pop().is_some());
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
    fn robust_frontier_excludes_unknown_hard_gate_without_scalarizing_tradeoffs() {
        let report = audit_corpus_json(FIXTURE);
        assert!(report.passed, "{:#?}", report.findings);
        let value = corpus();
        let options = &value["scenarios"][7]["expected"]["pareto_frontier"];
        assert_eq!(options, &json!(["repair_only", "repair_plus_backup"]));
    }

    #[test]
    fn robust_frontier_matches_independent_exhaustive_oracle_grid() {
        let mut value = corpus();
        let mut options = Vec::new();
        let mut feasible_rows = Vec::<(String, i64, i64)>::new();
        let mut expected_worst = serde_json::Map::new();

        // Exhaust the small integer objective space: cost x three named stress gaps.
        for cost in 0_i64..=2 {
            for baseline in 0_i64..=2 {
                for drought in 0_i64..=2 {
                    for supplier in 0_i64..=2 {
                        let id = format!("grid-c{cost}-b{baseline}-d{drought}-s{supplier}");
                        let worst = baseline.max(drought).max(supplier);
                        let gate_unknown = cost == 0 && worst == 0;
                        let hard_gates = if gate_unknown {
                            json!({
                                "civilian_safety": "unknown",
                                "evidence_qualification": "pass"
                            })
                        } else {
                            json!({
                                "civilian_safety": "pass",
                                "evidence_qualification": "pass"
                            })
                        };
                        options.push(json!({
                            "id": id,
                            "cost_ticks": cost,
                            "hard_gates": hard_gates,
                            "service_gap_by_scenario": {
                                "baseline": baseline,
                                "drought": drought,
                                "supplier_disruption": supplier
                            }
                        }));
                        expected_worst.insert(id.clone(), json!(worst));
                        if !gate_unknown {
                            feasible_rows.push((id, cost, worst));
                        }
                    }
                }
            }
        }

        // A second gate-unknown option looks excellent on service/cost but is never feasible.
        let grid_control_id = "grid-c0-b0-d0-s0";
        let excluded_id = "grid-unknown-gate";
        options.push(json!({
            "id": excluded_id,
            "cost_ticks": 0,
            "hard_gates": {
                "civilian_safety": "unknown",
                "evidence_qualification": "pass"
            },
            "service_gap_by_scenario": {
                "baseline": 0,
                "drought": 0,
                "supplier_disruption": 0
            }
        }));
        expected_worst.insert(excluded_id.to_owned(), json!(0));

        // Independent oracle: compute each cost bucket's minimum gap, then retain
        // only local minima that improve on every strictly cheaper bucket.
        let mut minimum_gap_by_cost = BTreeMap::<i64, i64>::new();
        for (_, cost, gap) in &feasible_rows {
            minimum_gap_by_cost
                .entry(*cost)
                .and_modify(|minimum| *minimum = (*minimum).min(*gap))
                .or_insert(*gap);
        }
        let mut expected_frontier = Vec::new();
        for (id, cost, gap) in &feasible_rows {
            let local_minimum = minimum_gap_by_cost[cost];
            let best_cheaper_gap = minimum_gap_by_cost
                .range(..*cost)
                .map(|(_, value)| *value)
                .min();
            if *gap == local_minimum
                && best_cheaper_gap.is_none_or(|cheaper_gap| *gap < cheaper_gap)
            {
                expected_frontier.push(id.clone());
            }
        }
        expected_frontier.sort();

        let mut expected_feasible: Vec<String> =
            feasible_rows.iter().map(|(id, _, _)| id.clone()).collect();
        expected_feasible.sort();
        value["scenarios"][7]["options"] = json!(options);
        value["scenarios"][7]["expected"]["feasible_options"] = json!(expected_feasible);
        value["scenarios"][7]["expected"]["excluded_by_hard_gate"] =
            json!([grid_control_id, excluded_id]);
        value["scenarios"][7]["expected"]["worst_case_service_gap"] = json!(expected_worst);
        value["scenarios"][7]["expected"]["pareto_frontier"] = json!(expected_frontier);

        let report = audit_corpus(&value);
        assert!(report.passed, "{:#?}", report.findings);
    }

    #[test]
    fn incorrect_robust_frontier_is_rejected() {
        let mut value = corpus();
        value["scenarios"][7]["expected"]["pareto_frontier"] = json!(["repair_plus_backup"]);
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_PARETO_FRONTIER_MISMATCH"));
    }

    #[test]
    fn stop_rule_state_space_matches_independent_oracle() {
        let mut value = corpus();
        let triggers = [
            "mandatory_floor_breached",
            "mandatory_evidence_stale",
            "mandatory_evidence_conflict",
            "prohibited_sensitive_field",
            "material_spillover_unassessed",
            "metric_definition_changed_after_outcomes",
            "appeal_suppressed",
            "none",
        ];
        let evidence_states = ["verified", "stale", "conflicting", "unknown"];

        // This table-driven oracle is intentionally expressed differently from
        // the production branch logic: first authority/evidence vetoes, then
        // hard stops, then review triggers, and finally ordinary continuation.
        let oracle = |trigger: &str,
                      evidence: &str,
                      owner_authorized: bool,
                      scope_assessed: bool|
         -> &'static str {
            if !owner_authorized || evidence != "verified" {
                return "BLOCKED";
            }
            match trigger {
                "mandatory_floor_breached"
                | "mandatory_evidence_stale"
                | "mandatory_evidence_conflict"
                | "prohibited_sensitive_field" => "BLOCKED",
                "material_spillover_unassessed"
                | "metric_definition_changed_after_outcomes"
                | "appeal_suppressed" => "REVIEW_REQUIRED",
                "none" if scope_assessed => "CONTINUE",
                "none" => "REVIEW_REQUIRED",
                _ => "BLOCKED",
            }
        };

        let mut generated = Vec::new();
        for (trigger_index, trigger) in triggers.iter().enumerate() {
            for (evidence_index, evidence) in evidence_states.iter().enumerate() {
                for owner_authorized in [false, true] {
                    for scope_assessed in [false, true] {
                        let case_id = format!(
                            "grid-t{trigger_index}-e{evidence_index}-o{owner_authorized}-s{scope_assessed}"
                        );
                        generated.push(json!({
                            "case_id": case_id,
                            "current_state": "ACTIVE",
                            "trigger": trigger,
                            "evidence_status": evidence,
                            "accountable_owner_authorized": owner_authorized,
                            "affected_scope_assessed": scope_assessed,
                            "expected_transition": oracle(
                                trigger,
                                evidence,
                                owner_authorized,
                                scope_assessed
                            )
                        }));
                    }
                }
            }
        }
        assert_eq!(generated.len(), 128);
        value["scenarios"][11]["stop_rule_cases"] = json!(generated);
        value["scenarios"][11]["expected"]["case_count"] = json!(128);
        let report = audit_corpus(&value);
        assert!(report.passed, "{:#?}", report.findings);
    }

    #[test]
    fn stop_rule_reference_corpus_has_explicit_safe_transitions() {
        let report = audit_corpus_json(FIXTURE);
        assert!(report.passed, "{:#?}", report.findings);
        assert_eq!(report.scenario_count, 12);
        let value = corpus();
        let cases = value["scenarios"][11]["stop_rule_cases"]
            .as_array()
            .unwrap();
        assert_eq!(cases[0]["expected_transition"], json!("BLOCKED"));
        assert_eq!(cases[3]["expected_transition"], json!("REVIEW_REQUIRED"));
        assert_eq!(cases[6]["expected_transition"], json!("CONTINUE"));
    }

    #[test]
    fn stop_rule_case_count_mismatch_is_rejected() {
        let mut value = corpus();
        value["scenarios"][11]["expected"]["case_count"] = json!(9);
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_STOP_RULE_CASE_COUNT_MISMATCH"));
    }

    #[test]
    fn mandatory_floor_breach_cannot_continue() {
        let mut value = corpus();
        value["scenarios"][11]["stop_rule_cases"][0]["expected_transition"] = json!("CONTINUE");
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_STOP_RULE_TRANSITION_MISMATCH"));
    }

    #[test]
    fn unassessed_spillover_requires_review() {
        let mut value = corpus();
        value["scenarios"][11]["stop_rule_cases"][3]["expected_transition"] = json!("CONTINUE");
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_STOP_RULE_TRANSITION_MISMATCH"));
    }

    #[test]
    fn unknown_evidence_or_missing_owner_blocks_continuation() {
        let mut value = corpus();
        value["scenarios"][11]["stop_rule_cases"][6]["evidence_status"] = json!("unknown");
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_STOP_RULE_TRANSITION_MISMATCH"));

        let mut value = corpus();
        value["scenarios"][11]["stop_rule_cases"][6]["accountable_owner_authorized"] = json!(false);
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_STOP_RULE_TRANSITION_MISMATCH"));
    }

    #[test]
    fn unregistered_stop_trigger_fails_closed() {
        let mut value = corpus();
        value["scenarios"][11]["stop_rule_cases"][9]["trigger"] = json!("unregistered-trigger");
        value["scenarios"][11]["stop_rule_cases"][9]["expected_transition"] = json!("CONTINUE");
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_STOP_RULE_TRIGGER_UNKNOWN"));
        assert!(has_code(&report, "PIRB_STOP_RULE_TRANSITION_MISMATCH"));
    }

    #[test]
    fn network_aggregate_reduction_still_reports_displaced_burden() {
        let report = audit_corpus_json(FIXTURE);
        assert!(report.passed, "{:#?}", report.findings);
        let value = corpus();
        let scenario = &value["scenarios"][10];
        assert_eq!(scenario["expected"]["aggregate_gap_before"], json!(36));
        assert_eq!(scenario["expected"]["aggregate_gap_after"], json!(34));
        assert_eq!(scenario["expected"]["displacement_detected"], json!(true));
        assert_eq!(
            scenario["expected"]["worsened_groups"],
            json!(["near-community-b", "outer-community-c"])
        );
        assert_eq!(
            scenario["expected"]["improvement_claim"],
            json!("qualified_with_displacement")
        );
    }

    #[test]
    fn network_displacement_cannot_be_hidden_by_aggregate_improvement() {
        let mut value = corpus();
        value["scenarios"][10]["expected"]["improvement_claim"] = json!("improved");
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_DISTRIBUTIONAL_HARM_HIDDEN"));
    }

    #[test]
    fn declared_worsened_groups_must_match_network_outcomes() {
        let mut value = corpus();
        value["scenarios"][10]["expected"]["worsened_groups"] = json!(["near-community-b"]);
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_WORSENED_GROUPS_MISMATCH"));
    }

    #[test]
    fn displaced_harm_cannot_be_reported_as_net_improvement() {
        let report = audit_corpus_json(FIXTURE);
        assert!(report.passed, "{:#?}", report.findings);
        let value = corpus();
        assert_eq!(
            value["scenarios"][8]["expected"]["aggregate_gap_before"],
            json!(15)
        );
        assert_eq!(
            value["scenarios"][8]["expected"]["aggregate_gap_after"],
            json!(15)
        );
        assert_eq!(
            value["scenarios"][8]["expected"]["claim_status"],
            json!("descriptive_only")
        );
    }

    #[test]
    fn displaced_harm_does_not_claim_net_improvement_without_reduction() {
        let mut value = corpus();
        value["scenarios"][8]["expected"]["improvement_claim"] = json!("improved");
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_NET_IMPROVEMENT_UNSUPPORTED"));
    }

    #[test]
    fn before_after_only_data_cannot_support_causal_claim() {
        let mut value = corpus();
        value["scenarios"][8]["expected"]["claim_status"] = json!("causal_effect_established");
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_CAUSAL_CLAIM_UNSUPPORTED"));
    }

    #[test]
    fn sensitive_field_in_proposed_shared_record_is_rejected() {
        let mut value = corpus();
        value["scenarios"][4]["proposed_shared_record_fields"]
            .as_array_mut()
            .unwrap()
            .push(json!("person_name"));
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_SENSITIVE_FIELD_LEAK"));
    }

    #[test]
    fn wrong_benchmark_track_is_rejected() {
        let mut value = corpus();
        value["scenarios"][2]["track"] = json!("PIRB-4");
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_TRACK_SCENARIO_MISMATCH"));
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
