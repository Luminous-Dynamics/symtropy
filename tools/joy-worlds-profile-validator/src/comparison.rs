use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashSet};

#[derive(Clone, Debug)]
struct Objective {
    id: String,
    unit: String,
    direction: String,
    dominance_tolerance: f64,
}

#[derive(Clone, Debug)]
struct Observation {
    status: String,
    value: Option<f64>,
    lower_bound: Option<f64>,
    upper_bound: Option<f64>,
    unit: String,
    evidence_ref: String,
}

#[derive(Clone, Debug)]
struct ScenarioCandidate {
    id: String,
    name: String,
    checks: BTreeMap<String, (String, String)>,
    outcomes: BTreeMap<String, Observation>,
}

#[derive(Clone, Debug)]
struct Scenario {
    id: String,
    assumptions_ref: String,
    candidates: BTreeMap<String, ScenarioCandidate>,
}

fn text_field(value: &Value, key: &str, path: &str, errors: &mut Vec<String>) -> String {
    match value.get(key).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()) {
        Some(text) => text.to_owned(),
        None => {
            errors.push(format!("{path}.{key}: required non-empty string"));
            String::new()
        }
    }
}

fn string_list(value: &Value, key: &str, path: &str, minimum: usize, errors: &mut Vec<String>) -> Vec<String> {
    let field_path = format!("{path}.{key}");
    let Some(items) = value.get(key).and_then(Value::as_array) else {
        errors.push(format!("{field_path}: required array of strings"));
        return Vec::new();
    };
    if items.len() < minimum {
        errors.push(format!("{field_path}: expected at least {minimum} item(s)"));
    }
    let mut result = Vec::with_capacity(items.len());
    let mut seen = HashSet::new();
    for (index, item) in items.iter().enumerate() {
        match item.as_str().map(str::trim).filter(|s| !s.is_empty()) {
            Some(text) => {
                if !seen.insert(text.to_owned()) {
                    errors.push(format!("{field_path}[{index}]: duplicate value '{text}'"));
                }
                result.push(text.to_owned());
            }
            None => errors.push(format!("{field_path}[{index}]: expected non-empty string")),
        }
    }
    result
}

fn array_field(value: &Value, key: &str, path: &str, minimum: usize, errors: &mut Vec<String>) -> Vec<Value> {
    let field_path = format!("{path}.{key}");
    let Some(items) = value.get(key).and_then(Value::as_array) else {
        errors.push(format!("{field_path}: required array"));
        return Vec::new();
    };
    if items.len() < minimum {
        errors.push(format!("{field_path}: expected at least {minimum} item(s)"));
    }
    items.clone()
}

fn number_field(value: &Value, key: &str, path: &str, errors: &mut Vec<String>) -> Option<f64> {
    match value.get(key).and_then(Value::as_f64) {
        Some(number) if number.is_finite() => Some(number),
        _ => {
            errors.push(format!("{path}.{key}: expected a finite JSON number"));
            None
        }
    }
}

fn parse_observation(
    raw: &Value,
    objective: &Objective,
    path: &str,
    errors: &mut Vec<String>,
) -> Observation {
    let status = text_field(raw, "status", path, errors);
    let unit = text_field(raw, "unit", path, errors);
    let evidence_ref = text_field(raw, "evidence_ref", path, errors);
    if !unit.is_empty() && unit != objective.unit {
        errors.push(format!(
            "{path}.unit: got '{unit}', expected declared unit '{}'",
            objective.unit
        ));
    }
    if status != "known" && status != "unknown" {
        errors.push(format!("{path}.status: expected 'known' or 'unknown'"));
    }

    if status == "unknown" {
        if raw.get("value").is_some_and(|value| !value.is_null()) {
            errors.push(format!("{path}.value: unknown observations must not provide a numeric value"));
        }
        return Observation {
            status,
            value: None,
            lower_bound: None,
            upper_bound: None,
            unit,
            evidence_ref,
        };
    }

    let value = number_field(raw, "value", path, errors);
    // A missing interval must never silently imply zero uncertainty. Exact
    // point estimates must explicitly set lower_bound == value == upper_bound.
    let lower_bound = number_field(raw, "lower_bound", path, errors);
    let upper_bound = number_field(raw, "upper_bound", path, errors);
    if let (Some(value), Some(lower), Some(upper)) = (value, lower_bound, upper_bound) {
        if lower > value || value > upper || lower > upper {
            errors.push(format!(
                "{path}: bounds must satisfy lower_bound <= value <= upper_bound"
            ));
        }
    }

    Observation {
        status,
        value,
        lower_bound,
        upper_bound,
        unit,
        evidence_ref,
    }
}

fn parse_candidate(
    raw: &Value,
    scenario_id: &str,
    objectives: &[Objective],
    required_constraints: &[String],
    errors: &mut Vec<String>,
) -> ScenarioCandidate {
    let path = format!("$.scenarios[scenario:{scenario_id}].candidates[{}]",
        raw.get("id").and_then(Value::as_str).unwrap_or("<missing-id>"));
    let id = text_field(raw, "id", &path, errors);
    let name = text_field(raw, "name", &path, errors);

    let raw_checks = array_field(raw, "constraint_checks", &path, 1, errors);
    let required: BTreeSet<String> = required_constraints.iter().cloned().collect();
    let mut checks = BTreeMap::new();
    for (index, check) in raw_checks.iter().enumerate() {
        let check_path = format!("{path}.constraint_checks[{index}]");
        let constraint_id = text_field(check, "id", &check_path, errors);
        let status = text_field(check, "status", &check_path, errors);
        let evidence_ref = text_field(check, "evidence_ref", &check_path, errors);
        if !required.contains(&constraint_id) {
            errors.push(format!("{check_path}.id: undeclared hard constraint '{constraint_id}'"));
        }
        if status != "pass" && status != "fail" && status != "unknown" {
            errors.push(format!("{check_path}.status: expected pass, fail, or unknown"));
        }
        if checks.insert(constraint_id.clone(), (status, evidence_ref)).is_some() {
            errors.push(format!("{check_path}.id: duplicate constraint check '{constraint_id}'"));
        }
    }
    for constraint_id in required_constraints {
        if !checks.contains_key(constraint_id) {
            errors.push(format!("{path}.constraint_checks: missing required hard constraint '{constraint_id}'"));
        }
    }

    let mut outcomes = BTreeMap::new();
    match raw.get("outcomes").and_then(Value::as_object) {
        Some(raw_outcomes) => {
            for objective in objectives {
                match raw_outcomes.get(&objective.id) {
                    Some(raw_observation) => {
                        let observation_path = format!("{path}.outcomes.{}", objective.id);
                        let observation = parse_observation(raw_observation, objective, &observation_path, errors);
                        outcomes.insert(objective.id.clone(), observation);
                    }
                    None => errors.push(format!("{path}.outcomes: missing objective '{}'", objective.id)),
                }
            }
            for key in raw_outcomes.keys() {
                if !objectives.iter().any(|objective| objective.id == *key) {
                    errors.push(format!("{path}.outcomes: undeclared objective '{key}'"));
                }
            }
        }
        None => errors.push(format!("{path}.outcomes: required object")),
    }

    ScenarioCandidate { id, name, checks, outcomes }
}

/// Deterministic, synthetic-fixture-only portfolio comparison.
///
/// The engine enforces hard constraints before multi-objective analysis. It
/// deliberately does not make a weighted score and does not qualify real sites.
pub fn compare_scenarios_json(raw: &str) -> Result<Value, Vec<String>> {
    let document: Value = serde_json::from_str(raw)
        .map_err(|error| vec![format!("JSON parse error: {error}")])?;
    let mut errors = Vec::new();

    let schema_version = text_field(&document, "schema_version", "$", &mut errors);
    let artifact_id = text_field(&document, "artifact_id", "$", &mut errors);
    let status = text_field(&document, "status", "$", &mut errors);
    if !schema_version.is_empty() && schema_version != "0.1.0" {
        errors.push("$.schema_version: expected 0.1.0 for this engine version");
    }
    if status != "synthetic_fixture" {
        errors.push("$.status: this initial comparator accepts only explicitly labelled synthetic_fixture inputs");
    }
    string_list(&document, "non_claims", "$", 1, &mut errors);

    let required_constraints = string_list(&document, "required_hard_constraints", "$", 1, &mut errors);

    let raw_objectives = array_field(&document, "objective_dimensions", "$", 2, &mut errors);
    let mut objectives = Vec::new();
    let mut objective_ids = HashSet::new();
    for (index, raw_objective) in raw_objectives.iter().enumerate() {
        let path = format!("$.objective_dimensions[{index}]");
        let id = text_field(raw_objective, "id", &path, &mut errors);
        let unit = text_field(raw_objective, "unit", &path, &mut errors);
        let direction = text_field(raw_objective, "direction", &path, &mut errors);
        let dominance_tolerance = number_field(raw_objective, "dominance_tolerance", &path, &mut errors)
            .unwrap_or(0.0);
        if direction != "maximize" && direction != "minimize" {
            errors.push(format!("{path}.direction: expected maximize or minimize"));
        }
        if dominance_tolerance < 0.0 {
            errors.push(format!("{path}.dominance_tolerance: must be non-negative and expressed in the objective's declared unit"));
        }
        if !id.is_empty() && !objective_ids.insert(id.clone()) {
            errors.push(format!("{path}.id: duplicate objective '{id}'"));
        }
        objectives.push(Objective { id, unit, direction, dominance_tolerance });
    }

    let raw_scenarios = array_field(&document, "scenarios", "$", 2, &mut errors);
    if !errors.is_empty() {
        return Err(errors);
    }

    let mut scenarios = Vec::new();
    let mut scenario_ids = HashSet::new();
    for raw_scenario in &raw_scenarios {
        let scenario_path = format!("$.scenarios[{}]",
            raw_scenario.get("id").and_then(Value::as_str).unwrap_or("<missing-id>"));
        let id = text_field(raw_scenario, "id", &scenario_path, &mut errors);
        let assumptions_ref = text_field(raw_scenario, "assumptions_ref", &scenario_path, &mut errors);
        if !scenario_ids.insert(id.clone()) {
            errors.push(format!("{scenario_path}.id: duplicate scenario id '{id}'"));
        }
        let raw_candidates = array_field(raw_scenario, "candidates", &scenario_path, 1, &mut errors);
        let mut candidates = BTreeMap::new();
        for raw_candidate in &raw_candidates {
            let candidate = parse_candidate(raw_candidate, &id, &objectives, &required_constraints, &mut errors);
            if candidates.insert(candidate.id.clone(), candidate.clone()).is_some() {
                errors.push(format!("{scenario_path}.candidates: duplicate candidate id '{}'", candidate.id));
            }
        }
        scenarios.push(Scenario { id, assumptions_ref, candidates });
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let Some(first_scenario) = scenarios.first() else {
        return Err(vec!["$.scenarios: at least two scenarios are required".to_owned()]);
    };
    let baseline_ids: BTreeSet<String> = first_scenario.candidates.keys().cloned().collect();
    for scenario in &scenarios {
        let ids: BTreeSet<String> = scenario.candidates.keys().cloned().collect();
        if ids != baseline_ids {
            errors.push(format!(
                "$.scenarios[{}].candidates: candidate set differs from the first scenario; paired comparison requires identical candidate IDs",
                scenario.id
            ));
        }
    }
    for candidate_id in &baseline_ids {
        let expected_name = &first_scenario.candidates[candidate_id].name;
        for scenario in &scenarios {
            if let Some(candidate) = scenario.candidates.get(candidate_id) {
                if &candidate.name != expected_name {
                    errors.push(format!(
                        "$.scenarios[{}].candidates[{candidate_id}].name: inconsistent candidate name across scenarios",
                        scenario.id
                    ));
                }
            }
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let mut candidate_results = Vec::new();
    let mut robust_outcomes: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();
    let mut optimistic_outcomes: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();
    let mut statuses: BTreeMap<String, String> = BTreeMap::new();
    let mut failed_by_candidate: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut unknown_by_candidate: BTreeMap<String, Vec<Value>> = BTreeMap::new();

    for candidate_id in &baseline_ids {
        let name = first_scenario.candidates[candidate_id].name.clone();
        let mut failed_constraints = Vec::new();
        let mut unknown_constraints = Vec::new();
        let mut unknown_outcomes = Vec::new();
        let mut scenario_results = Vec::new();

        for scenario in &scenarios {
            let candidate = &scenario.candidates[candidate_id];
            let mut scenario_checks = Vec::new();
            for constraint_id in &required_constraints {
                let (check_status, evidence_ref) = &candidate.checks[constraint_id];
                scenario_checks.push(json!({
                    "constraint_id": constraint_id,
                    "status": check_status,
                    "evidence_ref": evidence_ref
                }));
                if check_status == "fail" {
                    failed_constraints.push(json!({
                        "scenario_id": scenario.id,
                        "constraint_id": constraint_id,
                        "evidence_ref": evidence_ref
                    }));
                } else if check_status == "unknown" {
                    unknown_constraints.push(json!({
                        "scenario_id": scenario.id,
                        "constraint_id": constraint_id,
                        "evidence_ref": evidence_ref
                    }));
                }
            }

            let mut scenario_outcomes = Vec::new();
            for objective in &objectives {
                let observation = &candidate.outcomes[&objective.id];
                if observation.status == "unknown" {
                    unknown_outcomes.push(json!({
                        "scenario_id": scenario.id,
                        "objective_id": objective.id,
                        "evidence_ref": observation.evidence_ref
                    }));
                }
                scenario_outcomes.push(json!({
                    "objective_id": objective.id,
                    "status": observation.status,
                    "value": observation.value,
                    "lower_bound": observation.lower_bound,
                    "upper_bound": observation.upper_bound,
                    "unit": observation.unit,
                    "evidence_ref": observation.evidence_ref
                }));
            }
            scenario_results.push(json!({
                "scenario_id": scenario.id,
                "assumptions_ref": scenario.assumptions_ref,
                "constraint_checks": scenario_checks,
                "outcomes": scenario_outcomes
            }));
        }

        let candidate_status = if !failed_constraints.is_empty() {
            "infeasible"
        } else if !unknown_constraints.is_empty() || !unknown_outcomes.is_empty() {
            "not_evaluable"
        } else {
            "feasible_under_declared_assumptions"
        };
        statuses.insert(candidate_id.clone(), candidate_status.to_owned());
        failed_by_candidate.insert(candidate_id.clone(), failed_constraints.clone());
        let mut all_robust_known = true;
        let mut outcome_envelopes = Vec::new();
        let mut candidate_robust = BTreeMap::new();
        let mut candidate_optimistic = BTreeMap::new();

        for objective in &objectives {
            let observations: Vec<&Observation> = scenarios
                .iter()
                .map(|scenario| &scenario.candidates[candidate_id].outcomes[&objective.id])
                .collect();
            let complete = observations.iter().all(|observation| observation.status == "known");
            if !complete {
                all_robust_known = false;
            }

            let conservative: Vec<f64> = observations.iter().filter_map(|observation| {
                if observation.status != "known" {
                    return None;
                }
                if objective.direction == "maximize" {
                    observation.lower_bound.or(observation.value)
                } else {
                    observation.upper_bound.or(observation.value)
                }
            }).collect();
            let optimistic: Vec<f64> = observations.iter().filter_map(|observation| {
                if observation.status != "known" {
                    return None;
                }
                if objective.direction == "maximize" {
                    observation.upper_bound.or(observation.value)
                } else {
                    observation.lower_bound.or(observation.value)
                }
            }).collect();

            let worst_case = if complete && !conservative.is_empty() {
                Some(if objective.direction == "maximize" {
                    conservative.iter().copied().fold(f64::INFINITY, f64::min)
                } else {
                    conservative.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                })
            } else {
                None
            };
            let best_case = if complete && !optimistic.is_empty() {
                Some(if objective.direction == "maximize" {
                    optimistic.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                } else {
                    optimistic.iter().copied().fold(f64::INFINITY, f64::min)
                })
            } else {
                None
            };
            if let Some(value) = worst_case {
                candidate_robust.insert(objective.id.clone(), value);
            }
            if let Some(value) = best_case {
                candidate_optimistic.insert(objective.id.clone(), value);
            }
            outcome_envelopes.push(json!({
                "objective_id": objective.id,
                "direction": objective.direction,
                "unit": objective.unit,
                "complete_across_scenarios": complete,
                "conservative_worst_case": worst_case,
                "optimistic_envelope": best_case
            }));
        }

        if candidate_status == "feasible_under_declared_assumptions" && all_robust_known {
            robust_outcomes.insert(candidate_id.clone(), candidate_robust);
            optimistic_outcomes.insert(candidate_id.clone(), candidate_optimistic);
        }
        unknown_by_candidate.insert(candidate_id.clone(), {
            let mut items = unknown_constraints.clone();
            items.extend(unknown_outcomes.clone());
            items
        });

        let reason = match candidate_status {
            "infeasible" => "at least one required hard constraint fails in at least one declared scenario",
            "not_evaluable" => "at least one required constraint or outcome is unknown",
            _ => "all encoded hard constraints pass and all declared outcomes are present in the synthetic scenarios",
        };
        candidate_results.push(json!({
            "candidate_id": candidate_id,
            "name": name,
            "status": candidate_status,
            "classification_reason": reason,
            "failed_constraints": failed_constraints,
            "unknown_constraints": unknown_constraints,
            "unknown_outcomes": unknown_outcomes,
            "outcome_envelopes": outcome_envelopes,
            "scenario_results": scenario_results
        }));
    }

    let feasible_ids: Vec<String> = statuses.iter()
        .filter(|(id, status)| status.as_str() == "feasible_under_declared_assumptions" && robust_outcomes.contains_key(*id))
        .map(|(id, _)| id.clone())
        .collect();
    let mut pareto_front_ids = Vec::new();
    let mut dominated_feasible_ids = Vec::new();
    for candidate_id in &feasible_ids {
        let is_dominated = feasible_ids.iter().any(|other_id| {
            other_id != candidate_id && dominates(
                &robust_outcomes[other_id],
                &robust_outcomes[candidate_id],
                &objectives,
            )
        });
        if is_dominated {
            dominated_feasible_ids.push(candidate_id.clone());
        } else {
            pareto_front_ids.push(candidate_id.clone());
        }
    }

    let optimistic_frontier_ids = compute_frontier_ids(&optimistic_outcomes, &objectives);
    let conservative_set: BTreeSet<String> = pareto_front_ids.iter().cloned().collect();
    let optimistic_set: BTreeSet<String> = optimistic_frontier_ids.iter().cloned().collect();
    let stable_frontier_ids: Vec<String> = conservative_set.intersection(&optimistic_set).cloned().collect();
    let frontier_membership_uncertain_ids: Vec<String> = conservative_set
        .symmetric_difference(&optimistic_set)
        .cloned()
        .collect();

    // Single-objective endpoint sweeps identify which measurements can change
    // frontier membership while all other dimensions remain at their conservative
    // envelope. This is a non-probabilistic triage proxy, not formal expected VOI.
    let objective_leverage: Vec<Value> = objectives.iter().map(|objective| {
        let mut hybrid_outcomes = robust_outcomes.clone();
        for (candidate_id, vector) in hybrid_outcomes.iter_mut() {
            if let Some(optimistic_value) = optimistic_outcomes
                .get(candidate_id)
                .and_then(|outcomes| outcomes.get(&objective.id))
            {
                vector.insert(objective.id.clone(), *optimistic_value);
            }
        }
        let hybrid_frontier_ids = compute_frontier_ids(&hybrid_outcomes, &objectives);
        let hybrid_set: BTreeSet<String> = hybrid_frontier_ids.iter().cloned().collect();
        let changed: Vec<String> = conservative_set.symmetric_difference(&hybrid_set).cloned().collect();
        let newly_admitted: Vec<String> = hybrid_set.difference(&conservative_set).cloned().collect();
        let removed: Vec<String> = conservative_set.difference(&hybrid_set).cloned().collect();
        json!({
            "objective_id": objective.id,
            "unit": objective.unit,
            "interpretation": "endpoint-sensitivity heuristic; not expected value of information",
            "frontier_membership_change_count": changed.len(),
            "changed_candidate_ids": changed,
            "newly_admitted_candidate_ids": newly_admitted,
            "removed_candidate_ids": removed,
            "single_objective_optimistic_frontier_ids": hybrid_frontier_ids
        })
    }).collect();

    let mut evidence_gap_triage: Vec<Value> = objectives.iter().map(|objective| {
        let mut unknown_candidate_ids = BTreeSet::new();
        let mut unknown_evidence = Vec::new();
        for scenario in &scenarios {
            for (candidate_id, candidate) in &scenario.candidates {
                let observation = &candidate.outcomes[&objective.id];
                if observation.status == "unknown" {
                    unknown_candidate_ids.insert(candidate_id.clone());
                    unknown_evidence.push(json!({
                        "scenario_id": scenario.id,
                        "candidate_id": candidate_id,
                        "evidence_ref": observation.evidence_ref
                    }));
                }
            }
        }
        let frontier_change_count = objective_leverage.iter()
            .find(|entry| entry["objective_id"] == objective.id)
            .and_then(|entry| entry["frontier_membership_change_count"].as_u64())
            .unwrap_or(0);
        json!({
            "objective_id": objective.id,
            "unit": objective.unit,
            "unknown_observation_count": unknown_evidence.len(),
            "unknown_candidate_ids": unknown_candidate_ids.into_iter().collect::<Vec<_>>(),
            "unknown_evidence": unknown_evidence,
            "frontier_membership_change_count_if_known_interval_is_swept": frontier_change_count,
            "priority_interpretation": "triage heuristic only; does not include probabilities, stakeholder utility weights, measurement cost, or causal information gain"
        })
    }).collect();
    evidence_gap_triage.sort_by(|left, right| {
        let left_leverage = left["frontier_membership_change_count_if_known_interval_is_swept"].as_u64().unwrap_or(0);
        let right_leverage = right["frontier_membership_change_count_if_known_interval_is_swept"].as_u64().unwrap_or(0);
        let leverage_order = right_leverage.cmp(&left_leverage);
        if leverage_order != std::cmp::Ordering::Equal {
            return leverage_order;
        }
        let left_unknown = left["unknown_observation_count"].as_u64().unwrap_or(0);
        let right_unknown = right["unknown_observation_count"].as_u64().unwrap_or(0);
        let unknown_order = right_unknown.cmp(&left_unknown);
        if unknown_order != std::cmp::Ordering::Equal {
            return unknown_order;
        }
        left["objective_id"].as_str().cmp(&right["objective_id"].as_str())
    });

    let excluded_from_pareto: Vec<Value> = statuses.iter()
        .filter(|(id, status)| status.as_str() != "feasible_under_declared_assumptions" || !robust_outcomes.contains_key(*id))
        .map(|(id, status)| json!({
            "candidate_id": id,
            "status": status,
            "failed_constraints": failed_by_candidate.get(id).cloned().unwrap_or_default(),
            "unknown_evidence": unknown_by_candidate.get(id).cloned().unwrap_or_default()
        }))
        .collect();

    let scenario_ids: Vec<String> = scenarios.iter().map(|scenario| scenario.id.clone()).collect();
    Ok(json!({
        "schema_version": "0.1.0",
        "engine": "joy-worlds-deterministic-portfolio-comparator/0.1.0",
        "run_status": "synthetic_decision_support_only",
        "input_artifact_id": artifact_id,
        "input_sha256": format!("{:x}", Sha256::digest(raw.as_bytes())),
        "input_bytes": raw.len(),
        "scenario_ids": scenario_ids,
        "required_hard_constraints": required_constraints,
        "objective_dimensions": objectives.iter().map(|objective| json!({
            "id": objective.id, "unit": objective.unit, "direction": objective.direction,
            "dominance_tolerance": objective.dominance_tolerance
        })).collect::<Vec<_>>(),
        "candidate_results": candidate_results,
        "pareto_front_ids": pareto_front_ids,
        "dominated_feasible_ids": dominated_feasible_ids,
        "excluded_from_pareto": excluded_from_pareto,
        "decision_sensitivity": {
            "method": "endpoint-envelope Pareto sensitivity; no probabilities, stakeholder utility weights or measurement costs are assumed",
            "conservative_frontier_ids": pareto_front_ids,
            "optimistic_envelope_frontier_ids": optimistic_frontier_ids,
            "stable_frontier_ids": stable_frontier_ids,
            "frontier_membership_uncertain_ids": frontier_membership_uncertain_ids,
            "objective_measurement_leverage": objective_leverage,
            "evidence_gap_triage": evidence_gap_triage,
            "caveat": "Optimistic endpoint values for different objectives may come from different scenarios and may not be jointly attainable. Use this only to triage follow-up measurement; it is not a causal estimate or formal value-of-information calculation."
        },
        "methodology": {
            "hard_constraints": "A single fail in any declared scenario makes a candidate infeasible; any unresolved required constraint or outcome makes it not_evaluable unless a known failure already makes it infeasible.",
            "aggregation": "Conservative envelope uses lower bounds for maximize dimensions and upper bounds for minimize dimensions across all scenarios.",
            "ranking": "Pareto dominance only; no weighted aggregate score is constructed. Objective-specific dominance_tolerance values are expressed in each declared unit; there is no hidden universal epsilon.",
            "decision_sensitivity": "Reports conservative-vs-optimistic envelope frontier changes and one-objective-at-a-time endpoint sensitivity. This is a non-probabilistic evidence-priority heuristic, not expected value of information.",
            "paired_comparison": "Every scenario must contain the same candidate IDs with consistent names and declared objective units.",
            "ordering": "Candidate and objective IDs are emitted deterministically in lexical order; scenario results retain input order.",
            "evidence": "Evidence references are carried through, but are not independently authenticated by this engine."
        },
        "non_claims": [
            "Synthetic fixture results do not establish real-world market fit, safety, access compliance, community consent, legal compliance, sustainability or forecast accuracy.",
            "A feasible_under_declared_assumptions result means only that encoded checks passed for the declared synthetic scenarios.",
            "Pareto-front membership is not a universal recommendation or a license to proceed."
        ]
    }))
}

fn compute_frontier_ids(
    outcomes: &BTreeMap<String, BTreeMap<String, f64>>,
    objectives: &[Objective],
) -> Vec<String> {
    let ids: Vec<String> = outcomes.keys().cloned().collect();
    ids.iter()
        .filter(|candidate_id| {
            !ids.iter().any(|other_id| {
                if other_id == *candidate_id {
                    return false;
                }
                let other = outcomes.get(other_id).expect("IDs were collected from this map");
                let candidate = outcomes.get(*candidate_id).expect("IDs were collected from this map");
                dominates(other, candidate, objectives)
            })
        })
        .cloned()
        .collect()
}

fn dominates(a: &BTreeMap<String, f64>, b: &BTreeMap<String, f64>, objectives: &[Objective]) -> bool {
    let mut strictly_better = false;
    for objective in objectives {
        let Some(a_value) = a.get(&objective.id) else { return false };
        let Some(b_value) = b.get(&objective.id) else { return false };
        let no_worse = if objective.direction == "maximize" {
            *a_value >= *b_value - objective.dominance_tolerance
        } else {
            *a_value <= *b_value + objective.dominance_tolerance
        };
        if !no_worse {
            return false;
        }
        let better = if objective.direction == "maximize" {
            *a_value > *b_value + objective.dominance_tolerance
        } else {
            *a_value < *b_value - objective.dominance_tolerance
        };
        strictly_better |= better;
    }
    strictly_better
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn candidate(id: &str, quality: f64, water: f64, capex: f64, resource_status: &str, quality_status: &str) -> Value {
        let constraints = ["rights_and_access", "safety_case", "resource_envelope", "ecological_capacity"];
        let constraint_checks = constraints.iter().map(|constraint_id| {
            let status = if *constraint_id == "resource_envelope" { resource_status } else { "pass" };
            json!({
                "id": constraint_id,
                "status": status,
                "evidence_ref": format!("synthetic://fixture/{id}/{constraint_id}")
            })
        }).collect::<Vec<_>>();
        let quality_observation = if quality_status == "unknown" {
            json!({"status":"unknown","unit":"index_0_100","evidence_ref":format!("synthetic://fixture/{id}/quality-unknown")})
        } else {
            json!({"status":"known","value":quality,"lower_bound":quality-2.0,"upper_bound":quality+2.0,"unit":"index_0_100","evidence_ref":format!("synthetic://fixture/{id}/quality")})
        };
        json!({
            "id": id,
            "name": id,
            "constraint_checks": constraint_checks,
            "outcomes": {
                "visitor_quality": quality_observation,
                "water_m3_year": {"status":"known","value":water,"lower_bound":water*0.9,"upper_bound":water*1.1,"unit":"m3/year","evidence_ref":format!("synthetic://fixture/{id}/water")},
                "capex_musd": {"status":"known","value":capex,"lower_bound":capex*0.9,"upper_bound":capex*1.1,"unit":"USD_millions","evidence_ref":format!("synthetic://fixture/{id}/capex")}
            }
        })
    }

    fn fixture() -> String {
        json!({
            "schema_version":"0.1.0",
            "artifact_id":"test.destination-comparison",
            "status":"synthetic_fixture",
            "non_claims":["No site-feasibility claim."],
            "required_hard_constraints":["rights_and_access","safety_case","resource_envelope","ecological_capacity"],
            "objective_dimensions":[
                {"id":"visitor_quality","unit":"index_0_100","direction":"maximize","dominance_tolerance":0.0},
                {"id":"water_m3_year","unit":"m3/year","direction":"minimize","dominance_tolerance":0.0},
                {"id":"capex_musd","unit":"USD_millions","direction":"minimize","dominance_tolerance":0.0}
            ],
            "scenarios":[
                {"id":"base","assumptions_ref":"synthetic://scenario/base","candidates":[
                    candidate("baseline",30.0,500.0,0.2,"pass","known"),
                    candidate("better-experience",70.0,800.0,5.0,"pass","known"),
                    candidate("unsafe",90.0,4000.0,20.0,"pass","known"),
                    candidate("unknown",50.0,300.0,2.0,"unknown","unknown")
                ]},
                {"id":"water-stress","assumptions_ref":"synthetic://scenario/water-stress","candidates":[
                    candidate("baseline",28.0,500.0,0.2,"pass","known"),
                    candidate("better-experience",60.0,950.0,5.5,"pass","known"),
                    candidate("unsafe",45.0,4500.0,21.0,"fail","known"),
                    candidate("unknown",50.0,300.0,2.0,"unknown","unknown")
                ]}
            ]
        }).to_string()
    }

    #[test]
    fn accepts_fixture_and_keeps_tradeoffs_on_pareto_front() {
        let receipt = compare_scenarios_json(&fixture()).unwrap();
        assert_eq!(receipt["run_status"], "synthetic_decision_support_only");
        assert_eq!(receipt["pareto_front_ids"], json!(["baseline", "better-experience"]));
        assert_eq!(receipt["excluded_from_pareto"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn hard_constraint_failure_excludes_candidate_even_if_other_scores_are_high() {
        let receipt = compare_scenarios_json(&fixture()).unwrap();
        let unsafe_candidate = receipt["candidate_results"].as_array().unwrap().iter()
            .find(|candidate| candidate["candidate_id"] == "unsafe").unwrap();
        assert_eq!(unsafe_candidate["status"], "infeasible");
        assert!(!receipt["pareto_front_ids"].as_array().unwrap().iter().any(|id| id == "unsafe"));
    }

    #[test]
    fn unknown_constraint_or_outcome_is_not_evaluable_not_zero() {
        let receipt = compare_scenarios_json(&fixture()).unwrap();
        let candidate = receipt["candidate_results"].as_array().unwrap().iter()
            .find(|candidate| candidate["candidate_id"] == "unknown").unwrap();
        assert_eq!(candidate["status"], "not_evaluable");
        assert!(!receipt["pareto_front_ids"].as_array().unwrap().iter().any(|id| id == "unknown"));
        assert_eq!(candidate["outcome_envelopes"][0]["conservative_worst_case"], Value::Null);
        let triage = receipt["decision_sensitivity"]["evidence_gap_triage"].as_array().unwrap();
        let quality_gap = triage.iter().find(|entry| entry["objective_id"] == "visitor_quality").unwrap();
        assert_eq!(quality_gap["unknown_observation_count"], 2);
        assert_eq!(quality_gap["unknown_candidate_ids"], json!(["unknown"]));
        assert_eq!(
            quality_gap["priority_interpretation"],
            "triage heuristic only; does not include probabilities, stakeholder utility weights, measurement cost, or causal information gain"
        );
    }

    #[test]
    fn rejects_unpaired_candidate_sets() {
        let mut raw: Value = serde_json::from_str(&fixture()).unwrap();
        raw["scenarios"][1]["candidates"].as_array_mut().unwrap().pop();
        let errors = compare_scenarios_json(&raw.to_string()).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("candidate set differs")));
    }

    #[test]
    fn rejects_objective_unit_mismatch() {
        let mut raw: Value = serde_json::from_str(&fixture()).unwrap();
        raw["scenarios"][0]["candidates"][0]["outcomes"]["water_m3_year"]["unit"] = json!("litres");
        let errors = compare_scenarios_json(&raw.to_string()).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("expected declared unit")));
    }

    #[test]
    fn rejects_negative_objective_dominance_tolerance() {
        let mut raw: Value = serde_json::from_str(&fixture()).unwrap();
        raw["objective_dimensions"][0]["dominance_tolerance"] = json!(-0.01);
        let errors = compare_scenarios_json(&raw.to_string()).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("dominance_tolerance: must be non-negative")));
    }

    #[test]
    fn rejects_objective_missing_dominance_tolerance() {
        let mut raw: Value = serde_json::from_str(&fixture()).unwrap();
        raw["objective_dimensions"][0].as_object_mut().unwrap().remove("dominance_tolerance");
        let errors = compare_scenarios_json(&raw.to_string()).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("objective_dimensions[0].dominance_tolerance")));
    }

    #[test]
    fn rejects_missing_uncertainty_bounds_instead_of_treating_estimate_as_exact() {
        let mut raw: Value = serde_json::from_str(&fixture()).unwrap();
        raw["scenarios"][0]["candidates"][0]["outcomes"]["visitor_quality"]
            .as_object_mut().unwrap().remove("lower_bound");
        let errors = compare_scenarios_json(&raw.to_string()).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("lower_bound: expected a finite JSON number")));
    }

    #[test]
    fn rejects_invalid_uncertainty_bounds() {
        let mut raw: Value = serde_json::from_str(&fixture()).unwrap();
        raw["scenarios"][0]["candidates"][0]["outcomes"]["visitor_quality"]["lower_bound"] = json!(33.0);
        let errors = compare_scenarios_json(&raw.to_string()).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("bounds must satisfy")));
    }

    #[test]
    fn receipt_binds_exact_input_bytes_with_sha256() {
        let raw = fixture();
        let receipt = compare_scenarios_json(&raw).unwrap();
        let expected = format!("{:x}", Sha256::digest(raw.as_bytes()));
        assert_eq!(receipt["input_sha256"], expected);
        assert_eq!(receipt["input_bytes"].as_u64(), Some(raw.len() as u64));
    }

    #[test]
    fn receipt_is_deterministic_for_identical_input() {
        let raw = fixture();
        let first = compare_scenarios_json(&raw).unwrap().to_string();
        let second = compare_scenarios_json(&raw).unwrap().to_string();
        assert_eq!(first, second);
    }


    fn mini_candidate(id: &str, quality: i64, water: i64) -> Value {
        json!({
            "id": id,
            "name": format!("Candidate {id}"),
            "constraint_checks": [
                {"id":"rights","status":"pass","evidence_ref":format!("synthetic://mini/{id}/rights")},
                {"id":"safety","status":"pass","evidence_ref":format!("synthetic://mini/{id}/safety")}
            ],
            "outcomes": {
                "quality": {
                    "status":"known","value":quality,"lower_bound":quality,"upper_bound":quality,
                    "unit":"points","evidence_ref":format!("synthetic://mini/{id}/quality")
                },
                "water": {
                    "status":"known","value":water,"lower_bound":water,"upper_bound":water,
                    "unit":"m3/year","evidence_ref":format!("synthetic://mini/{id}/water")
                }
            }
        })
    }

    fn mini_fixture(values: &[(i64, i64); 3]) -> String {
        let candidates = vec![
            mini_candidate("A", values[0].0, values[0].1),
            mini_candidate("B", values[1].0, values[1].1),
            mini_candidate("C", values[2].0, values[2].1)
        ];
        json!({
            "schema_version":"0.1.0",
            "artifact_id":"test.bounded-pareto-oracle",
            "status":"synthetic_fixture",
            "non_claims":["Bounded mathematical fixture only."],
            "required_hard_constraints":["rights","safety"],
            "objective_dimensions":[
                {"id":"quality","unit":"points","direction":"maximize","dominance_tolerance":0.0},
                {"id":"water","unit":"m3/year","direction":"minimize","dominance_tolerance":0.0}
            ],
            "scenarios":[
                {"id":"base","assumptions_ref":"synthetic://mini/base","candidates":candidates.clone()},
                {"id":"stress","assumptions_ref":"synthetic://mini/stress","candidates":candidates}
            ]
        }).to_string()
    }

    #[test]
    fn all_81_four_constraint_state_combinations_fail_closed() {
        let states = ["pass", "fail", "unknown"];
        for encoded in 0_u32..81 {
            let mut remainder = encoded;
            let mut state_vector = ["pass"; 4];
            for state in &mut state_vector {
                *state = states[(remainder % 3) as usize];
                remainder /= 3;
            }

            let mut raw: Value = serde_json::from_str(&fixture()).unwrap();
            let constraints = [
                "rights_and_access",
                "safety_case",
                "resource_envelope",
                "ecological_capacity"
            ];
            for (index, constraint_id) in constraints.iter().enumerate() {
                let candidate = raw["scenarios"][0]["candidates"].as_array_mut().unwrap()
                    .iter_mut()
                    .find(|candidate| candidate["id"] == "baseline").unwrap();
                let check = candidate["constraint_checks"].as_array_mut().unwrap()
                    .iter_mut()
                    .find(|check| check["id"] == *constraint_id).unwrap();
                check["status"] = json!(state_vector[index]);
            }

            let receipt = compare_scenarios_json(&raw.to_string())
                .unwrap_or_else(|errors| panic!("constraint combination {encoded} rejected: {errors:?}"));
            let candidate = receipt["candidate_results"].as_array().unwrap()
                .iter().find(|candidate| candidate["candidate_id"] == "baseline").unwrap();
            let expected = if state_vector.contains(&"fail") {
                "infeasible"
            } else if state_vector.contains(&"unknown") {
                "not_evaluable"
            } else {
                "feasible_under_declared_assumptions"
            };
            assert_eq!(candidate["status"], expected, "constraint states {state_vector:?}");
            let in_front = receipt["pareto_front_ids"].as_array().unwrap()
                .iter().any(|id| id == "baseline");
            assert_eq!(
                in_front,
                expected == "feasible_under_declared_assumptions",
                "frontier constraint leak for state vector {state_vector:?}"
            );
        }
    }

    #[test]
    fn bounded_exhaustive_pareto_agrees_with_independent_oracle() {
        // Exhaust all 3^6 assignments of three candidate quality/water pairs.
        // The reference front is computed directly from the source values,
        // independently of the engine's aggregation and dominates helper.
        for encoded in 0_u32..729 {
            let mut remainder = encoded;
            let mut digits = [0_i64; 6];
            for digit in &mut digits {
                *digit = (remainder % 3) as i64;
                remainder /= 3;
            }
            let values = [
                (digits[0], digits[1]),
                (digits[2], digits[3]),
                (digits[4], digits[5]),
            ];
            let receipt = compare_scenarios_json(&mini_fixture(&values))
                .unwrap_or_else(|errors| panic!("fixture {encoded} rejected: {errors:?}"));

            let ids = ["A", "B", "C"];
            let mut expected = Vec::new();
            for i in 0..3 {
                let mut dominated = false;
                for j in 0..3 {
                    if i == j {
                        continue;
                    }
                    let no_worse = values[j].0 >= values[i].0 && values[j].1 <= values[i].1;
                    let strictly_better = values[j].0 > values[i].0 || values[j].1 < values[i].1;
                    if no_worse && strictly_better {
                        dominated = true;
                        break;
                    }
                }
                if !dominated {
                    expected.push(ids[i]);
                }
            }
            assert_eq!(
                receipt["pareto_front_ids"],
                json!(expected),
                "independent Pareto oracle mismatch for encoded fixture {encoded} with values {values:?}"
            );
        }
    }

    #[test]
    fn endpoint_sensitivity_identifies_frontier_changing_measurements() {
        let mut raw: Value = serde_json::from_str(&mini_fixture(&[(5, 5), (6, 6), (0, 10)])).unwrap();
        for scenario in raw["scenarios"].as_array_mut().unwrap() {
            let candidate = scenario["candidates"].as_array_mut().unwrap()
                .iter_mut()
                .find(|candidate| candidate["id"] == "A").unwrap();
            candidate["outcomes"]["quality"]["lower_bound"] = json!(1);
            candidate["outcomes"]["quality"]["upper_bound"] = json!(10);
            candidate["outcomes"]["water"]["lower_bound"] = json!(1);
            candidate["outcomes"]["water"]["upper_bound"] = json!(10);
        }

        let receipt = compare_scenarios_json(&raw.to_string()).unwrap();
        assert_eq!(receipt["decision_sensitivity"]["conservative_frontier_ids"], json!(["B"]));
        assert_eq!(receipt["decision_sensitivity"]["optimistic_envelope_frontier_ids"], json!(["A"]));
        assert_eq!(receipt["decision_sensitivity"]["frontier_membership_uncertain_ids"], json!(["A", "B"]));

        let leverage = receipt["decision_sensitivity"]["objective_measurement_leverage"].as_array().unwrap();
        assert_eq!(leverage.len(), 2);
        assert!(leverage.iter().all(|entry| entry["frontier_membership_change_count"].as_u64().unwrap() > 0));
        assert_eq!(
            receipt["decision_sensitivity"]["method"],
            "endpoint-envelope Pareto sensitivity; no probabilities, stakeholder utility weights or measurement costs are assumed"
        );
    }

    #[test]
    fn endpoint_sensitivity_never_reintroduces_infeasible_candidates() {
        let receipt = compare_scenarios_json(&fixture()).unwrap();
        assert!(!receipt["decision_sensitivity"]["conservative_frontier_ids"].as_array().unwrap()
            .iter().any(|id| id == "unsafe"));
        assert!(!receipt["decision_sensitivity"]["optimistic_envelope_frontier_ids"].as_array().unwrap()
            .iter().any(|id| id == "unsafe"));
        assert!(receipt["excluded_from_pareto"].as_array().unwrap()
            .iter().any(|entry| entry["candidate_id"] == "unsafe" && entry["status"] == "infeasible"));
    }

    #[test]
    fn scenario_order_does_not_change_pareto_membership() {
        let mut raw: Value = serde_json::from_str(&fixture()).unwrap();
        let expected = compare_scenarios_json(&raw.to_string()).unwrap()["pareto_front_ids"].clone();
        raw["scenarios"].as_array_mut().unwrap().reverse();
        let actual = compare_scenarios_json(&raw.to_string()).unwrap()["pareto_front_ids"].clone();
        assert_eq!(actual, expected);
    }

    #[test]
    fn rejects_non_synthetic_input_by_default() {
        let mut raw: Value = serde_json::from_str(&fixture()).unwrap();
        raw["status"] = json!("site_validated");
        let errors = compare_scenarios_json(&raw.to_string()).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("only explicitly labelled synthetic_fixture")));
    }
}

        let mut raw: Value = serde_json::from_str(&fixture()).unwrap();
        raw["scenarios"][0]["candidates"][0]["outcomes"]["visitor_quality"]["lower_bound"] = json!(33.0);
        let errors = compare_scenarios_json(&raw.to_string()).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("bounds must satisfy")));
    }

    #[test]
    fn receipt_binds_exact_input_bytes_with_sha256() {
        let raw = fixture();
        let receipt = compare_scenarios_json(&raw).unwrap();
        let expected = format!("{:x}", Sha256::digest(raw.as_bytes()));
        assert_eq!(receipt["input_sha256"], expected);
        assert_eq!(receipt["input_bytes"].as_u64(), Some(raw.len() as u64));
    }

    #[test]
    fn receipt_is_deterministic_for_identical_input() {
        let raw = fixture();
        let first = compare_scenarios_json(&raw).unwrap().to_string();
        let second = compare_scenarios_json(&raw).unwrap().to_string();
        assert_eq!(first, second);
    }


    fn mini_candidate(id: &str, quality: i64, water: i64) -> Value {
        json!({
            "id": id,
            "name": format!("Candidate {id}"),
            "constraint_checks": [
                {"id":"rights","status":"pass","evidence_ref":format!("synthetic://mini/{id}/rights")},
                {"id":"safety","status":"pass","evidence_ref":format!("synthetic://mini/{id}/safety")}
            ],
            "outcomes": {
                "quality": {
                    "status":"known","value":quality,"lower_bound":quality,"upper_bound":quality,
                    "unit":"points","evidence_ref":format!("synthetic://mini/{id}/quality")
                },
                "water": {
                    "status":"known","value":water,"lower_bound":water,"upper_bound":water,
                    "unit":"m3/year","evidence_ref":format!("synthetic://mini/{id}/water")
                }
            }
        })
    }

    fn mini_fixture(values: &[(i64, i64); 3]) -> String {
        let candidates = vec![
            mini_candidate("A", values[0].0, values[0].1),
            mini_candidate("B", values[1].0, values[1].1),
            mini_candidate("C", values[2].0, values[2].1)
        ];
        json!({
            "schema_version":"0.1.0",
            "artifact_id":"test.bounded-pareto-oracle",
            "status":"synthetic_fixture",
            "non_claims":["Bounded mathematical fixture only."],
            "required_hard_constraints":["rights","safety"],
            "objective_dimensions":[
                {"id":"quality","unit":"points","direction":"maximize","dominance_tolerance":0.0},
                {"id":"water","unit":"m3/year","direction":"minimize","dominance_tolerance":0.0}
            ],
            "scenarios":[
                {"id":"base","assumptions_ref":"synthetic://mini/base","candidates":candidates.clone()},
                {"id":"stress","assumptions_ref":"synthetic://mini/stress","candidates":candidates}
            ]
        }).to_string()
    }

    #[test]
    fn all_81_four_constraint_state_combinations_fail_closed() {
        let states = ["pass", "fail", "unknown"];
        for encoded in 0_u32..81 {
            let mut remainder = encoded;
            let mut state_vector = ["pass"; 4];
            for state in &mut state_vector {
                *state = states[(remainder % 3) as usize];
                remainder /= 3;
            }

            let mut raw: Value = serde_json::from_str(&fixture()).unwrap();
            let constraints = [
                "rights_and_access",
                "safety_case",
                "resource_envelope",
                "ecological_capacity"
            ];
            for (index, constraint_id) in constraints.iter().enumerate() {
                let candidate = raw["scenarios"][0]["candidates"].as_array_mut().unwrap()
                    .iter_mut()
                    .find(|candidate| candidate["id"] == "baseline").unwrap();
                let check = candidate["constraint_checks"].as_array_mut().unwrap()
                    .iter_mut()
                    .find(|check| check["id"] == *constraint_id).unwrap();
                check["status"] = json!(state_vector[index]);
            }

            let receipt = compare_scenarios_json(&raw.to_string())
                .unwrap_or_else(|errors| panic!("constraint combination {encoded} rejected: {errors:?}"));
            let candidate = receipt["candidate_results"].as_array().unwrap()
                .iter().find(|candidate| candidate["candidate_id"] == "baseline").unwrap();
            let expected = if state_vector.contains(&"fail") {
                "infeasible"
            } else if state_vector.contains(&"unknown") {
                "not_evaluable"
            } else {
                "feasible_under_declared_assumptions"
            };
            assert_eq!(candidate["status"], expected, "constraint states {state_vector:?}");
            let in_front = receipt["pareto_front_ids"].as_array().unwrap()
                .iter().any(|id| id == "baseline");
            assert_eq!(
                in_front,
                expected == "feasible_under_declared_assumptions",
                "frontier constraint leak for state vector {state_vector:?}"
            );
        }
    }

    #[test]
    fn bounded_exhaustive_pareto_agrees_with_independent_oracle() {
        // Exhaust all 3^6 assignments of three candidate quality/water pairs.
        // The reference front is computed directly from the source values,
        // independently of the engine's aggregation and dominates helper.
        for encoded in 0_u32..729 {
            let mut remainder = encoded;
            let mut digits = [0_i64; 6];
            for digit in &mut digits {
                *digit = (remainder % 3) as i64;
                remainder /= 3;
            }
            let values = [
                (digits[0], digits[1]),
                (digits[2], digits[3]),
                (digits[4], digits[5]),
            ];
            let receipt = compare_scenarios_json(&mini_fixture(&values))
                .unwrap_or_else(|errors| panic!("fixture {encoded} rejected: {errors:?}"));

            let ids = ["A", "B", "C"];
            let mut expected = Vec::new();
            for i in 0..3 {
                let mut dominated = false;
                for j in 0..3 {
                    if i == j {
                        continue;
                    }
                    let no_worse = values[j].0 >= values[i].0 && values[j].1 <= values[i].1;
                    let strictly_better = values[j].0 > values[i].0 || values[j].1 < values[i].1;
                    if no_worse && strictly_better {
                        dominated = true;
                        break;
                    }
                }
                if !dominated {
                    expected.push(ids[i]);
                }
            }
            assert_eq!(
                receipt["pareto_front_ids"],
                json!(expected),
                "independent Pareto oracle mismatch for encoded fixture {encoded} with values {values:?}"
            );
        }
    }

    #[test]
    fn endpoint_sensitivity_identifies_frontier_changing_measurements() {
        let mut raw: Value = serde_json::from_str(&mini_fixture(&[(5, 5), (6, 6), (0, 10)])).unwrap();
        for scenario in raw["scenarios"].as_array_mut().unwrap() {
            let candidate = scenario["candidates"].as_array_mut().unwrap()
                .iter_mut()
                .find(|candidate| candidate["id"] == "A").unwrap();
            candidate["outcomes"]["quality"]["lower_bound"] = json!(1);
            candidate["outcomes"]["quality"]["upper_bound"] = json!(10);
            candidate["outcomes"]["water"]["lower_bound"] = json!(1);
            candidate["outcomes"]["water"]["upper_bound"] = json!(10);
        }

        let receipt = compare_scenarios_json(&raw.to_string()).unwrap();
        assert_eq!(receipt["decision_sensitivity"]["conservative_frontier_ids"], json!(["B"]));
        assert_eq!(receipt["decision_sensitivity"]["optimistic_envelope_frontier_ids"], json!(["A"]));
        assert_eq!(receipt["decision_sensitivity"]["frontier_membership_uncertain_ids"], json!(["A", "B"]));

        let leverage = receipt["decision_sensitivity"]["objective_measurement_leverage"].as_array().unwrap();
        assert_eq!(leverage.len(), 2);
        assert!(leverage.iter().all(|entry| entry["frontier_membership_change_count"].as_u64().unwrap() > 0));
        assert_eq!(
            receipt["decision_sensitivity"]["method"],
            "endpoint-envelope Pareto sensitivity; no probabilities, stakeholder utility weights or measurement costs are assumed"
        );
    }

    #[test]
    fn endpoint_sensitivity_never_reintroduces_infeasible_candidates() {
        let receipt = compare_scenarios_json(&fixture()).unwrap();
        assert!(!receipt["decision_sensitivity"]["conservative_frontier_ids"].as_array().unwrap()
            .iter().any(|id| id == "unsafe"));
        assert!(!receipt["decision_sensitivity"]["optimistic_envelope_frontier_ids"].as_array().unwrap()
            .iter().any(|id| id == "unsafe"));
        assert!(receipt["excluded_from_pareto"].as_array().unwrap()
            .iter().any(|entry| entry["candidate_id"] == "unsafe" && entry["status"] == "infeasible"));
    }

    #[test]
    fn scenario_order_does_not_change_pareto_membership() {
        let mut raw: Value = serde_json::from_str(&fixture()).unwrap();
        let expected = compare_scenarios_json(&raw.to_string()).unwrap()["pareto_front_ids"].clone();
        raw["scenarios"].as_array_mut().unwrap().reverse();
        let actual = compare_scenarios_json(&raw.to_string()).unwrap()["pareto_front_ids"].clone();
        assert_eq!(actual, expected);
    }

    #[test]
    fn rejects_non_synthetic_input_by_default() {
        let mut raw: Value = serde_json::from_str(&fixture()).unwrap();
        raw["status"] = json!("site_validated");
        let errors = compare_scenarios_json(&raw.to_string()).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("only explicitly labelled synthetic_fixture")));
    }
}
