use jsonschema::Validator;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const SCHEMA_ID: &str = "urn:luminous-dynamics:repairability-architecture-decision-engine:0.1.0";
const DECISION_ID: &str = "REPAIRABILITY-DECISION-001";
const EXPECTED_GATES: [&str; 4] = [
    "product_safety",
    "core_wash_function",
    "durability_and_post_repair",
    "market_conformity_route",
];
const EXPECTED_CRITERIA: [(&str, &str); 8] = [
    ("lifecycle_cost_per_useful_service", "minimize"),
    ("restore_time_p90", "minimize"),
    ("first_visit_verified_repair_rate", "maximize"),
    ("energy_per_successful_load", "minimize"),
    ("water_per_successful_load", "minimize"),
    ("discarded_material_per_verified_repair", "minimize"),
    ("added_interface_failure_rate", "minimize"),
    ("target_window_part_fulfilment_rate", "maximize"),
];
const EXPECTED_CANDIDATES: [(&str, &str); 5] = [
    ("W0-RETAIL-CONTROL", "control"),
    ("W1-ACCESSIBLE-CONVENTIONAL", "candidate"),
    ("W2-SERVICE-SPINE", "candidate"),
    ("W3-BEARING-TUB-TRADE", "orthogonal_trade"),
    ("W4-DRIVE-TOPOLOGY", "orthogonal_trade"),
];
const ELIGIBLE_EVIDENCE: [&str; 2] = ["physical_measurement", "independent_replication"];

type Errors = Vec<String>;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DecisionAuditSummary {
    pub(crate) candidate_count: usize,
    pub(crate) hard_gate_count: usize,
    pub(crate) criterion_count: usize,
    pub(crate) observation_count: usize,
    pub(crate) eligible_candidate_count: usize,
    pub(crate) pareto_frontier_count: usize,
    pub(crate) schema_sha256: String,
    pub(crate) decision_sha256: String,
}

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn schema_validator(schema: &Value) -> Result<Validator, String> {
    if text_at(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("decision schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if text_at(schema, "$id") != Some(SCHEMA_ID) {
        return Err(format!("decision schema $id must be {SCHEMA_ID}"));
    }
    if !jsonschema::draft202012::meta::is_valid(schema) {
        return Err(
            "decision schema does not validate against the Draft 2020-12 meta-schema".to_owned(),
        );
    }
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("cannot compile decision schema: {error}"))
}

fn criterion_by_id<'a>(decision: &'a Value, id: &str) -> Option<&'a Value> {
    decision
        .get("criteria")?
        .as_array()?
        .iter()
        .find(|criterion| text_at(criterion, "id") == Some(id))
}

fn observation_by_id<'a>(candidate: &'a Value, id: &str) -> Option<&'a Value> {
    candidate
        .get("observations")?
        .as_array()?
        .iter()
        .find(|observation| text_at(observation, "criterion_id") == Some(id))
}

fn gate_status(candidate: &Value, expected_gate: &str) -> &'static str {
    let Some(gates) = candidate.get("hard_gates").and_then(Value::as_array) else {
        return "unknown";
    };
    let mut matches = gates.iter().filter(|gate| {
        text_at(gate, "gate_id") == Some(expected_gate)
    });
    let Some(gate) = matches.next() else {
        return "unknown";
    };
    if matches.next().is_some() {
        return "unknown";
    }
    match text_at(gate, "status") {
        Some("pass") => "pass",
        Some("fail") => "fail",
        _ => "unknown",
    }
}

fn all_gates_pass(candidate: &Value) -> bool {
    EXPECTED_GATES
        .iter()
        .all(|gate| gate_status(candidate, gate) == "pass")
}

fn any_gate_fails(candidate: &Value) -> bool {
    EXPECTED_GATES
        .iter()
        .any(|gate| gate_status(candidate, gate) == "fail")
}

fn observation_is_rank_eligible(
    observation: Option<&Value>,
    criterion: &Value,
) -> bool {
    if text_at(criterion, "status") != Some("method_frozen") {
        return false;
    }
    let Some(observation) = observation else {
        return false;
    };
    let evidence_class = text_at(observation, "evidence_class").unwrap_or("");
    if !ELIGIBLE_EVIDENCE.contains(&evidence_class) {
        return false;
    }
    if text_at(observation, "unit") != text_at(criterion, "unit") {
        return false;
    }
    if observation.get("method_frozen").and_then(Value::as_bool) != Some(true) {
        return false;
    }
    if text_at(observation, "evidence_id").is_none_or(str::is_empty)
        || text_at(observation, "method_id").is_none_or(str::is_empty)
    {
        return false;
    }
    let (Some(lower), Some(upper)) = (
        observation.get("lower").and_then(Value::as_f64),
        observation.get("upper").and_then(Value::as_f64),
    ) else {
        return false;
    };
    lower.is_finite() && upper.is_finite() && lower <= upper
}

fn all_criteria_measured(decision: &Value, candidate: &Value) -> bool {
    let Some(criteria) = decision.get("criteria").and_then(Value::as_array) else {
        return false;
    };
    let Some(observations) = candidate.get("observations").and_then(Value::as_array) else {
        return false;
    };
    let observed_ids: BTreeSet<&str> = observations
        .iter()
        .filter_map(|observation| text_at(observation, "criterion_id"))
        .collect();
    if observed_ids.len() != observations.len() || observed_ids.len() != criteria.len() {
        return false;
    }
    criteria.iter().all(|criterion| {
        let Some(id) = text_at(criterion, "id") else {
            return false;
        };
        observation_is_rank_eligible(observation_by_id(candidate, id), criterion)
    })
}

fn candidate_status(decision: &Value, candidate: &Value) -> &'static str {
    if any_gate_fails(candidate) {
        return "rejected_hard_gate";
    }
    if !all_gates_pass(candidate) {
        return "blocked_unknown_gate";
    }
    if !all_criteria_measured(decision, candidate) {
        return "insufficient_eligible_evidence";
    }
    "eligible_for_robust_pareto"
}

fn robustly_dominates(decision: &Value, a: &Value, b: &Value) -> bool {
    let Some(criteria) = decision.get("criteria").and_then(Value::as_array) else {
        return false;
    };
    let mut strict_improvement = false;
    for criterion in criteria {
        let Some(id) = text_at(criterion, "id") else {
            return false;
        };
        let (Some(a_obs), Some(b_obs)) = (
            observation_by_id(a, id),
            observation_by_id(b, id),
        ) else {
            return false;
        };
        let (Some(a_lower), Some(a_upper), Some(b_lower), Some(b_upper)) = (
            a_obs.get("lower").and_then(Value::as_f64),
            a_obs.get("upper").and_then(Value::as_f64),
            b_obs.get("lower").and_then(Value::as_f64),
            b_obs.get("upper").and_then(Value::as_f64),
        ) else {
            return false;
        };
        match text_at(criterion, "direction") {
            Some("minimize") => {
                if a_upper > b_lower {
                    return false;
                }
                strict_improvement |= a_upper < b_lower;
            }
            Some("maximize") => {
                if a_lower < b_upper {
                    return false;
                }
                strict_improvement |= a_lower > b_upper;
            }
            _ => return false,
        }
    }
    strict_improvement
}

fn evaluate_decision(decision: &Value) -> Value {
    let candidates = decision
        .get("candidates")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let preliminary: Vec<&str> = candidates
        .iter()
        .map(|candidate| candidate_status(decision, candidate))
        .collect();
    let mut outcomes = Vec::new();
    let mut eligible_count = 0usize;
    let mut frontier_count = 0usize;

    for (index, candidate) in candidates.iter().enumerate() {
        let status = preliminary[index];
        let mut dominated_by = Vec::<String>::new();
        if status == "eligible_for_robust_pareto" {
            eligible_count += 1;
            for (other_index, other) in candidates.iter().enumerate() {
                if index != other_index
                    && preliminary[other_index] == "eligible_for_robust_pareto"
                    && robustly_dominates(decision, other, candidate)
                {
                    if let Some(id) = text_at(other, "id") {
                        dominated_by.push(id.to_owned());
                    }
                }
            }
        }
        let is_frontier = status == "eligible_for_robust_pareto" && dominated_by.is_empty();
        if is_frontier {
            frontier_count += 1;
        }
        outcomes.push(json!({
            "candidate_id": text_at(candidate, "id").unwrap_or("unknown"),
            "status": status,
            "robustly_dominated_by": dominated_by,
            "robust_pareto_frontier": is_frontier
        }));
    }

    json!({
        "policy": "hard_gates_then_interval_robust_pareto",
        "candidate_assessments": outcomes,
        "eligible_candidate_count": eligible_count,
        "robust_pareto_frontier_count": frontier_count,
        "physical_ranking_claim_permitted": eligible_count >= 2,
        "product_winner_declared": false
    })
}

fn audit_values(
    schema: &Value,
    decision: &Value,
    schema_raw: &str,
    decision_raw: &str,
) -> Result<DecisionAuditSummary, Errors> {
    let validator = schema_validator(schema).map_err(|error| vec![error])?;
    let schema_errors: Vec<String> = validator
        .iter_errors(decision)
        .map(|error| error.to_string())
        .collect();
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }

    let mut errors = Vec::new();
    if text_at(decision, "schema_version") != Some("0.1.0")
        || text_at(decision, "decision_id") != Some(DECISION_ID)
    {
        errors.push(
            "decision identity/version must remain REPAIRABILITY-DECISION-001 v0.1.0"
                .to_owned(),
        );
    }
    if text_at(decision, "status") != Some("decision_profile_under_evaluation")
        || text_at(decision, "claim_ceiling")
            != Some("no_architecture_ranked_without_eligible_physical_evidence")
    {
        errors.push("decision profile cannot claim an architectural winner".to_owned());
    }
    let policy = decision.get("decision_policy").unwrap_or(&Value::Null);
    if text_at(policy, "unknown_policy") != Some("unknown_is_not_pass")
        || text_at(policy, "missing_numeric_policy") != Some("unknown_not_zero")
        || text_at(policy, "scoring_policy")
            != Some("non_compensatory_hard_gates_then_robust_pareto")
        || text_at(policy, "dominance_policy")
            != Some("interval_bounds_must_separate_on_every_criterion_and_strictly_improve_at_least_one")
    {
        errors.push(
            "decision policy must preserve non-compensatory gates, unknown-not-zero and interval-robust dominance"
                .to_owned(),
        );
    }

    let Some(gates) = policy.get("hard_gate_ids").and_then(Value::as_array) else {
        return Err(vec!["decision_policy.hard_gate_ids must be an array".to_owned()]);
    };
    let gate_ids: Vec<&str> = gates.iter().filter_map(Value::as_str).collect();
    if gate_ids.as_slice() != EXPECTED_GATES.as_slice() {
        errors.push(format!("hard gate IDs/order must remain {:?}", EXPECTED_GATES));
    }

    let Some(criteria) = decision.get("criteria").and_then(Value::as_array) else {
        return Err(vec!["criteria must be an array".to_owned()]);
    };
    if criteria.len() != EXPECTED_CRITERIA.len() {
        errors.push(format!("decision must contain {} criteria", EXPECTED_CRITERIA.len()));
    }
    let mut criterion_ids = BTreeSet::new();
    for (index, criterion) in criteria.iter().enumerate() {
        let Some(id) = text_at(criterion, "id") else {
            errors.push(format!("criteria[{index}].id must be a string"));
            continue;
        };
        if !criterion_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate criterion ID {id}"));
        }
        if let Some((expected_id, expected_direction)) = EXPECTED_CRITERIA.get(index) {
            if id != *expected_id || text_at(criterion, "direction") != Some(*expected_direction) {
                errors.push(format!(
                    "criteria[{index}] must remain {} with direction {}",
                    expected_id, expected_direction
                ));
            }
        }
        if !matches!(
            text_at(criterion, "status"),
            Some("method_definition_not_yet_frozen" | "method_frozen")
        ) {
            errors.push(format!("{id}: unsupported measurement-method state"));
        }
    }

    let Some(candidates) = decision.get("candidates").and_then(Value::as_array) else {
        return Err(vec!["candidates must be an array".to_owned()]);
    };
    if candidates.len() != EXPECTED_CANDIDATES.len() {
        errors.push(format!(
            "decision must contain {} reviewed washer alternatives",
            EXPECTED_CANDIDATES.len()
        ));
    }
    let mut candidate_ids = BTreeSet::new();
    let mut observation_count = 0usize;
    for (index, candidate) in candidates.iter().enumerate() {
        let Some(id) = text_at(candidate, "id") else {
            errors.push(format!("candidates[{index}].id must be a string"));
            continue;
        };
        if !candidate_ids.insert(id.to_owned()) {
            errors.push(format!("duplicate candidate identity {id}"));
        }
        if let Some((expected_id, expected_role)) = EXPECTED_CANDIDATES.get(index) {
            if id != *expected_id || text_at(candidate, "role") != Some(*expected_role) {
                errors.push(format!(
                    "candidate[{index}] must retain reviewed identity/role {expected_id}/{expected_role}"
                ));
            }
        }
        let candidate_gates = candidate.get("hard_gates").and_then(Value::as_array);
        if candidate_gates.is_none_or(|items| items.len() != EXPECTED_GATES.len()) {
            errors.push(format!("{id}: every hard gate must be represented exactly once"));
        } else if let Some(items) = candidate_gates {
            let found: Vec<&str> = items
                .iter()
                .filter_map(|item| text_at(item, "gate_id"))
                .collect();
            if found.iter().copied().collect::<BTreeSet<_>>().len() != found.len()
                || EXPECTED_GATES.iter().any(|expected| !found.contains(expected))
            {
                errors.push(format!("{id}: hard gate IDs must be unique and complete"));
            }
            for gate in items {
                if text_at(gate, "status") != Some("unknown")
                    && text_at(gate, "evidence_id").is_none_or(str::is_empty)
                {
                    errors.push(format!(
                        "{id}.{}: non-unknown gate state requires an evidence ID",
                        text_at(gate, "gate_id").unwrap_or("unknown_gate")
                    ));
                }
            }
        }

        let observations = candidate.get("observations").and_then(Value::as_array);
        if let Some(observations) = observations {
            observation_count += observations.len();
            let mut seen_observations = BTreeSet::new();
            for observation in observations {
                let Some(metric_id) = text_at(observation, "criterion_id") else {
                    errors.push(format!("{id}: observation criterion_id must be a string"));
                    continue;
                };
                if !seen_observations.insert(metric_id.to_owned()) {
                    errors.push(format!("{id}: duplicate observation for criterion {metric_id}"));
                }
                let Some(criterion) = criterion_by_id(decision, metric_id) else {
                    errors.push(format!("{id}: unknown observation criterion {metric_id}"));
                    continue;
                };
                let evidence_class = text_at(observation, "evidence_class").unwrap_or("");
                if ELIGIBLE_EVIDENCE.contains(&evidence_class) {
                    if observation.get("method_frozen").and_then(Value::as_bool) != Some(true) {
                        errors.push(format!(
                            "{id}.{metric_id}: scored evidence requires a frozen method"
                        ));
                    }
                    if text_at(observation, "evidence_id").is_none_or(str::is_empty)
                        || text_at(observation, "method_id").is_none_or(str::is_empty)
                        || text_at(observation, "evidence_id")
                            .unwrap_or("")
                            .starts_with("SYNTHETIC-")
                    {
                        errors.push(format!(
                            "{id}.{metric_id}: eligible evidence requires a real source receipt and method ID, not synthetic test evidence"
                        ));
                    }
                    if text_at(observation, "unit") != text_at(criterion, "unit") {
                        errors.push(format!("{id}.{metric_id}: unit does not match the criterion"));
                    }
                    if text_at(criterion, "status") != Some("method_frozen") {
                        errors.push(format!(
                            "{id}.{metric_id}: eligible observations require the criterion method to be frozen"
                        ));
                    }
                    let (Some(lower), Some(upper)) = (
                        observation.get("lower").and_then(Value::as_f64),
                        observation.get("upper").and_then(Value::as_f64),
                    ) else {
                        errors.push(format!(
                            "{id}.{metric_id}: eligible evidence requires finite numeric bounds"
                        ));
                        continue;
                    };
                    if !lower.is_finite() || !upper.is_finite() || lower < 0.0 || lower > upper {
                        errors.push(format!(
                            "{id}.{metric_id}: bounds must be finite, non-negative and satisfy lower <= upper"
                        ));
                    }
                    if matches!(
                        metric_id,
                        "first_visit_verified_repair_rate" | "target_window_part_fulfilment_rate"
                    ) && upper > 1.0
                    {
                        errors.push(format!(
                            "{id}.{metric_id}: proportion bounds must not exceed 1.0"
                        ));
                    }
                }
            }
        }
    }
    for (expected, _) in EXPECTED_CANDIDATES {
        if !candidate_ids.contains(expected) {
            errors.push(format!("required candidate {expected} is missing"));
        }
    }

    let experiment = decision.get("next_experiment_policy").unwrap_or(&Value::Null);
    if text_at(experiment, "objective")
        != Some("maximize_decision_relevant_information_per_unit_cost_and_risk")
        || text_at(experiment, "existing_information_value_owner")
            .is_none_or(|value| !value.contains("Symthaea") || !value.contains("EIG/EVSI"))
    {
        errors.push(
            "experiment policy must preserve value-of-information objective and existing Symthaea EIG/EVSI ownership"
                .to_owned(),
        );
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let assessment = evaluate_decision(decision);
    let eligible_candidate_count = assessment["eligible_candidate_count"]
        .as_u64()
        .unwrap_or(0) as usize;
    let pareto_frontier_count = assessment["robust_pareto_frontier_count"]
        .as_u64()
        .unwrap_or(0) as usize;
    Ok(DecisionAuditSummary {
        candidate_count: candidates.len(),
        hard_gate_count: EXPECTED_GATES.len(),
        criterion_count: criteria.len(),
        observation_count,
        eligible_candidate_count,
        pareto_frontier_count,
        schema_sha256: format!("{:x}", Sha256::digest(schema_raw.as_bytes())),
        decision_sha256: format!("{:x}", Sha256::digest(decision_raw.as_bytes())),
    })
}

pub(crate) fn audit_decision_texts(
    schema_raw: &str,
    decision_raw: &str,
) -> Result<(DecisionAuditSummary, Value), Errors> {
    let schema: Value = serde_json::from_str(schema_raw)
        .map_err(|error| vec![format!("decision schema JSON parse failed: {error}")])?;
    let decision: Value = serde_json::from_str(decision_raw)
        .map_err(|error| vec![format!("decision profile JSON parse failed: {error}")])?;
    let summary = audit_values(&schema, &decision, schema_raw, decision_raw)?;
    let assessment = evaluate_decision(&decision);
    Ok((summary, assessment))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SCHEMA: &str = include_str!(
        "../../../schemas/appliances/repairability_architecture_decision_v0_1.schema.json"
    );
    const DECISION: &str = include_str!(
        "../../../fixtures/appliances/repairability_architecture_decision_v0_1.json"
    );

    fn canonical_decision() -> Value {
        serde_json::from_str(DECISION).expect("decision JSON must parse")
    }

    fn synthetic_scored_decision() -> Value {
        let mut decision = canonical_decision();
        for criterion in decision["criteria"].as_array_mut().expect("criteria") {
            criterion["status"] = json!("method_frozen");
        }
        let criteria = decision["criteria"].as_array().expect("criteria").clone();
        for (candidate_index, candidate) in decision["candidates"]
            .as_array_mut()
            .expect("candidates")
            .iter_mut()
            .enumerate()
        {
            for gate in candidate["hard_gates"].as_array_mut().expect("gates") {
                gate["status"] = json!("pass");
                gate["evidence_id"] = json!("SYNTHETIC-TEST-NOT-PHYSICAL");
            }
            let mut observations = Vec::new();
            for (criterion_index, criterion) in criteria.iter().enumerate() {
                let direction = text_at(criterion, "direction").unwrap_or("minimize");
                let criterion_id = text_at(criterion, "id").unwrap_or("");
                let lower_value = if direction == "minimize" {
                    (candidate_index * 10 + criterion_index) as f64
                } else if matches!(
                    criterion_id,
                    "first_visit_verified_repair_rate" | "target_window_part_fulfilment_rate"
                ) {
                    0.95 - (candidate_index as f64 * 0.1)
                } else {
                    100.0 - (candidate_index * 10 + criterion_index) as f64
                };
                observations.push(json!({
                    "criterion_id": text_at(criterion, "id").unwrap_or("unknown"),
                    "lower": lower_value,
                    "upper": lower_value,
                    "unit": text_at(criterion, "unit").unwrap_or("unknown"),
                    "evidence_class": "physical_measurement",
                    "evidence_id": "SYNTHETIC-TEST-NOT-PHYSICAL",
                    "method_id": "SYNTHETIC-TEST-METHOD",
                    "method_frozen": true,
                    "uncertainty_note": "Synthetic algorithm test only; no appliance data."
                }));
            }
            candidate["observations"] = json!(observations);
        }
        decision
    }

    #[test]
    fn canonical_profile_is_valid_but_ranks_no_architecture() {
        let (summary, assessment) = audit_decision_texts(SCHEMA, DECISION)
            .expect("canonical decision profile must pass contract audit");
        assert_eq!(summary.candidate_count, 5);
        assert_eq!(summary.hard_gate_count, 4);
        assert_eq!(summary.criterion_count, 8);
        assert_eq!(summary.observation_count, 0);
        assert_eq!(summary.eligible_candidate_count, 0);
        assert_eq!(summary.pareto_frontier_count, 0);
        assert!(!assessment["physical_ranking_claim_permitted"].as_bool().unwrap());
        for candidate in assessment["candidate_assessments"].as_array().unwrap() {
            assert_eq!(candidate["status"], "blocked_unknown_gate");
        }
    }

    #[test]
    fn failed_safety_gate_rejects_candidate_regardless_of_cost() {
        let mut decision = synthetic_scored_decision();
        decision["candidates"][0]["hard_gates"][0]["status"] = json!("fail");
        assert_eq!(candidate_status(&decision, &decision["candidates"][0]), "rejected_hard_gate");
    }

    #[test]
    fn unknown_hard_gate_blocks_candidate() {
        let mut decision = synthetic_scored_decision();
        decision["candidates"][0]["hard_gates"][0]["status"] = json!("unknown");
        assert_eq!(candidate_status(&decision, &decision["candidates"][0]), "blocked_unknown_gate");
    }

    #[test]
    fn missing_criterion_prevents_comparison() {
        let mut decision = synthetic_scored_decision();
        decision["candidates"][0]["observations"].as_array_mut().unwrap().pop();
        assert_eq!(
            candidate_status(&decision, &decision["candidates"][0]),
            "insufficient_eligible_evidence"
        );
    }

    #[test]
    fn desk_estimate_cannot_qualify_as_physical_measurement() {
        let mut decision = synthetic_scored_decision();
        decision["candidates"][0]["observations"][0]["evidence_class"] = json!("desk_estimate");
        assert_eq!(
            candidate_status(&decision, &decision["candidates"][0]),
            "insufficient_eligible_evidence"
        );
    }

    #[test]
    fn invalid_bounds_prevent_comparison() {
        let mut decision = synthetic_scored_decision();
        decision["candidates"][0]["observations"][0]["lower"] = json!(2.0);
        decision["candidates"][0]["observations"][0]["upper"] = json!(1.0);
        assert_eq!(
            candidate_status(&decision, &decision["candidates"][0]),
            "insufficient_eligible_evidence"
        );
    }

    #[test]
    fn interval_robust_dominance_requires_separation_on_every_criterion() {
        let mut decision = synthetic_scored_decision();
        assert!(robustly_dominates(
            &decision,
            &decision["candidates"][0],
            &decision["candidates"][1]
        ));
    }

    #[test]
    fn overlapping_bounds_do_not_create_a_false_winner() {
        let mut decision = synthetic_scored_decision();
        for observation in decision["candidates"][1]["observations"].as_array_mut().unwrap() {
            observation["lower"] = json!(-1.0);
            observation["upper"] = json!(1.0);
        }
        assert!(!robustly_dominates(
            &decision,
            &decision["candidates"][0],
            &decision["candidates"][1]
        ));
    }

    #[test]
    fn audit_rejects_synthetic_ids_promoted_to_real_evidence() {
        let mut decision = synthetic_scored_decision();
        let raw = serde_json::to_string(&decision).unwrap();
        let schema: Value = serde_json::from_str(SCHEMA).unwrap();
        let schema_raw = serde_json::to_string(&schema).unwrap();
        assert!(audit_values(&schema, &decision, &schema_raw, &raw).is_err());
    }
}
