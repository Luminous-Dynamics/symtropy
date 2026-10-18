// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const REQUIRED_SCENARIOS: [&str; 9] = [
    "pirb-r2-minimum-service-floor-01",
    "pirb-r1-required-acceptance-01",
    "pirb-r0-stale-capacity-01",
    "pirb-r0-conflicting-reports-01",
    "pirb-r4-personal-data-boundary-01",
    "pirb-r3-no-lookahead-01",
    "pirb-r1-refused-participation-01",
    "pirb-r3-worst-case-frontier-01",
    "pirb-r5-displacement-not-net-impact-01",
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
        let expected_track = match id {
            "pirb-r2-minimum-service-floor-01" => "PIRB-2",
            "pirb-r1-required-acceptance-01" | "pirb-r1-refused-participation-01" => "PIRB-1",
            "pirb-r0-stale-capacity-01" | "pirb-r0-conflicting-reports-01" => "PIRB-0",
            "pirb-r4-personal-data-boundary-01" => "PIRB-4",
            "pirb-r3-no-lookahead-01" | "pirb-r3-worst-case-frontier-01" => "PIRB-3",
            "pirb-r5-displacement-not-net-impact-01" => "PIRB-5",
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
            "pirb-r2-minimum-service-floor-01" => audit_service_floor(scenario, &path, &mut findings),
            "pirb-r1-required-acceptance-01" => audit_required_acceptance(scenario, &path, &mut findings),
            "pirb-r0-stale-capacity-01" => audit_stale_evidence(scenario, &path, &mut findings),
            "pirb-r0-conflicting-reports-01" => audit_conflicting_reports(scenario, &path, &mut findings),
            "pirb-r4-personal-data-boundary-01" => audit_privacy_boundary(scenario, &path, &mut findings),
            "pirb-r3-no-lookahead-01" => audit_no_lookahead(scenario, &path, &mut findings),
            "pirb-r1-refused-participation-01" => audit_refusal(scenario, &path, &mut findings),
            "pirb-r3-worst-case-frontier-01" => audit_robust_frontier(scenario, &path, &mut findings),
            "pirb-r5-displacement-not-net-impact-01" => audit_displacement(scenario, &path, &mut findings),
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
    let all_uncorroborated = scenario
        .get("reports")
        .and_then(Value::as_array)
        .is_some_and(|reports| {
            reports.iter().all(|report| report.get("corroborated").and_then(Value::as_bool) == Some(false))
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
    let input: BTreeSet<String> = string_list(scenario.get("input_fields")).into_iter().collect();
    let prohibited = string_list(scenario.get("prohibited_shared_fields"));
    let shared: BTreeSet<String> = string_list(scenario.get("proposed_shared_record_fields")).into_iter().collect();
    let has_prohibited_input = prohibited.iter().any(|field| input.contains(field));
    let leaked: Vec<_> = prohibited.iter().filter(|field| shared.contains(*field)).cloned().collect();
    let decision = scenario.pointer("/expected/shared_record_decision").and_then(Value::as_str);

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

fn audit_robust_frontier(scenario: &Value, path: &str, findings: &mut Vec<Finding>) {
    let scenario_ids = string_list(scenario.get("scenario_ids"));
    if scenario_ids.is_empty() || scenario_ids.iter().collect::<BTreeSet<_>>().len() != scenario_ids.len() {
        findings.push(finding(
            "PIRB_ROBUST_SCENARIOS_INVALID",
            &format!("{path}.scenario_ids"),
            "Robustness scenarios must be non-empty and uniquely named.",
        ));
        return;
    }
    if scenario.get("scenario_probabilities_assigned").and_then(Value::as_bool) != Some(false) {
        findings.push(finding(
            "PIRB_SYNTHETIC_PROBABILITY_CLAIM",
            &format!("{path}.scenario_probabilities_assigned"),
            "Named stress scenarios must not imply empirical probabilities.",
        ));
    }
    let Some(options) = scenario.get("options").and_then(Value::as_array) else {
        findings.push(finding("PIRB_OPTIONS_MISSING", path, "Robust plan options must be an array."));
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
            findings.push(finding("PIRB_OPTION_ID_MISSING", &option_path, "Each option requires a string id."));
            continue;
        };
        if !option_ids.insert(id.clone()) {
            findings.push(finding("PIRB_OPTION_ID_DUPLICATE", &option_path, "Option ids must be unique."));
        }

        let cost = option.get("cost_ticks").and_then(Value::as_i64);
        if !cost.is_some_and(|value| value >= 0) {
            findings.push(finding("PIRB_OPTION_COST_INVALID", &format!("{option_path}.cost_ticks"), "Cost must be a non-negative integer tick count."));
        }

        let Some(gates) = option.get("hard_gates").and_then(Value::as_object) else {
            findings.push(finding("PIRB_OPTION_GATES_MISSING", &option_path, "Every option must declare its hard gates."));
            excluded.push(id);
            continue;
        };
        let gate_values_valid = !gates.is_empty() && gates.values().all(|gate| {
            matches!(gate.as_str(), Some("pass") | Some("fail") | Some("unknown"))
        });
        if !gate_values_valid {
            findings.push(finding("PIRB_OPTION_GATE_INVALID", &format!("{option_path}.hard_gates"), "Hard gates must be non-empty and use pass/fail/unknown."));
            excluded.push(id);
            continue;
        }
        let gates_pass = gates.values().all(|gate| gate.as_str() == Some("pass"));

        let Some(gaps) = option.get("service_gap_by_scenario").and_then(Value::as_object) else {
            findings.push(finding("PIRB_OPTION_GAPS_MISSING", &option_path, "Every option requires per-scenario service gaps."));
            excluded.push(id);
            continue;
        };
        let gap_keys: BTreeSet<String> = gaps.keys().cloned().collect();
        if gap_keys != scenario_set {
            findings.push(finding("PIRB_OPTION_SCENARIO_COVERAGE_MISMATCH", &format!("{option_path}.service_gap_by_scenario"), "Every option must cover exactly the declared stress scenarios."));
            excluded.push(id);
            continue;
        }
        let parsed_gaps: Option<Vec<i64>> = gaps.values().map(Value::as_i64).collect();
        let Some(parsed_gaps) = parsed_gaps else {
            findings.push(finding("PIRB_OPTION_GAP_INVALID", &format!("{option_path}.service_gap_by_scenario"), "Service gaps must be integer values."));
            excluded.push(id);
            continue;
        };
        if parsed_gaps.iter().any(|gap| *gap < 0) {
            findings.push(finding("PIRB_OPTION_GAP_NEGATIVE", &format!("{option_path}.service_gap_by_scenario"), "Service gaps cannot be negative."));
            excluded.push(id);
            continue;
        }
        let Some(worst_gap) = parsed_gaps.iter().copied().max() else {
            findings.push(finding("PIRB_OPTION_GAPS_EMPTY", &option_path, "An option needs at least one scenario gap."));
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

    if scenario.pointer("/expected/scenario_probabilities_assigned").and_then(Value::as_bool) != Some(false) {
        findings.push(finding(
            "PIRB_SYNTHETIC_PROBABILITY_CLAIM",
            &format!("{path}.expected.scenario_probabilities_assigned"),
            "Synthetic stress cases must not be represented as probabilistic forecasts.",
        ));
    }
    if let Some(expected_worst) = scenario.pointer("/expected/worst_case_service_gap").and_then(Value::as_object) {
        for (id, actual) in worst_by_id {
            if expected_worst.get(&id).and_then(Value::as_i64) != Some(actual) {
                findings.push(finding(
                    "PIRB_WORST_CASE_GAP_MISMATCH",
                    &format!("{path}.expected.worst_case_service_gap.{id}"),
                    "Declared worst-case gap does not match the maximum across the declared stress cases.",
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
        findings.push(finding("PIRB_OUTCOMES_MISSING", path, "Before/after outcomes must be an array."));
        return;
    };
    if outcomes.is_empty() {
        findings.push(finding("PIRB_OUTCOMES_EMPTY", path, "At least one group outcome is required."));
        return;
    }

    let mut before_total = 0_i64;
    let mut after_total = 0_i64;
    let mut improved_group = false;
    let mut worsened_group = false;

    for (index, outcome) in outcomes.iter().enumerate() {
        let outcome_path = format!("{path}.outcomes[{index}]");
        let before = outcome.get("service_gap_before").and_then(Value::as_i64);
        let after = outcome.get("service_gap_after").and_then(Value::as_i64);
        let (Some(before), Some(after)) = (before, after) else {
            findings.push(finding("PIRB_OUTCOME_VALUE_INVALID", &outcome_path, "Before/after gaps must be integers."));
            return;
        };
        if before < 0 || after < 0 {
            findings.push(finding("PIRB_OUTCOME_VALUE_NEGATIVE", &outcome_path, "Service gaps cannot be negative."));
            return;
        }
        let Some(next_before) = before_total.checked_add(before) else {
            findings.push(finding("PIRB_OUTCOME_OVERFLOW", &outcome_path, "Before-gap total overflowed."));
            return;
        };
        let Some(next_after) = after_total.checked_add(after) else {
            findings.push(finding("PIRB_OUTCOME_OVERFLOW", &outcome_path, "After-gap total overflowed."));
            return;
        };
        before_total = next_before;
        after_total = next_after;
        improved_group |= after < before;
        worsened_group |= after > before;
    }

    let displacement_detected = improved_group && worsened_group;
    if scenario.get("design_type").and_then(Value::as_str) == Some("before_after_only")
        && scenario.get("causal_claim_requested").and_then(Value::as_bool) == Some(true)
        && scenario.pointer("/expected/claim_status").and_then(Value::as_str) != Some("descriptive_only")
    {
        findings.push(finding(
            "PIRB_CAUSAL_CLAIM_UNSUPPORTED",
            &format!("{path}.expected.claim_status"),
            "Before/after-only evidence with concurrent factors cannot by itself establish causal impact.",
        ));
    }
    if scenario.get("concurrent_factors").and_then(Value::as_array).is_none_or(Vec::is_empty) {
        findings.push(finding(
            "PIRB_CONCURRENT_FACTORS_UNDECLARED",
            &format!("{path}.concurrent_factors"),
            "This adversarial case requires a disclosed concurrent factor.",
        ));
    }

    if scenario.pointer("/expected/aggregate_gap_before").and_then(Value::as_i64) != Some(before_total)
        || scenario.pointer("/expected/aggregate_gap_after").and_then(Value::as_i64) != Some(after_total)
    {
        findings.push(finding(
            "PIRB_AGGREGATE_GAP_MISMATCH",
            &format!("{path}.expected"),
            "Expected aggregate gaps must equal the sum of the declared group outcomes.",
        ));
    }
    if scenario.pointer("/expected/displacement_detected").and_then(Value::as_bool) != Some(displacement_detected) {
        findings.push(finding(
            "PIRB_DISPLACEMENT_MISCLASSIFIED",
            &format!("{path}.expected.displacement_detected"),
            "Expected displacement status must match the per-group outcome changes.",
        ));
    }
    if after_total >= before_total
        && scenario.pointer("/expected/improvement_claim").and_then(Value::as_str) != Some("not_supported")
    {
        findings.push(finding(
            "PIRB_NET_IMPROVEMENT_UNSUPPORTED",
            &format!("{path}.expected.improvement_claim"),
            "No aggregate reduction is observed; a net improvement claim is unsupported.",
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
        assert_eq!(report.scenario_count, 9);
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
    fn incorrect_robust_frontier_is_rejected() {
        let mut value = corpus();
        value["scenarios"][7]["expected"]["pareto_frontier"] = json!(["repair_plus_backup"]);
        let report = audit_corpus(&value);
        assert!(has_code(&report, "PIRB_PARETO_FRONTIER_MISMATCH"));
    }

    #[test]
    fn displaced_harm_cannot_be_reported_as_net_improvement() {
        let report = audit_corpus_json(FIXTURE);
        assert!(report.passed, "{:#?}", report.findings);
        let value = corpus();
        assert_eq!(value["scenarios"][8]["expected"]["aggregate_gap_before"], json!(15));
        assert_eq!(value["scenarios"][8]["expected"]["aggregate_gap_after"], json!(15));
        assert_eq!(value["scenarios"][8]["expected"]["claim_status"], json!("descriptive_only"));
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
