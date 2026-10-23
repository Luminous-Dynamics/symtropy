use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, process};

const CORPUS: &str = include_str!("../../../fixtures/transport/mission_resilience_scenarios_v0_1.json");
const CLAIM_CEILING: &str =
    "synthetic_stress_coverage_is_not_route_authorization_or_real_world_safety_validation";
const CAPABILITY_FIELDS: [&str; 6] = [
    "condition_control",
    "securement",
    "accessible_transfer",
    "patient_support",
    "operator_present",
    "recovery_available",
];
const GATE_FIELDS: [&str; 6] = [
    "route_authorization",
    "environmental_protection",
    "hazard_status",
    "recovery_status",
    "operator_status",
    "condition_control_status",
];
const VALID_STATUSES: [&str; 3] = ["passed", "unknown", "failed"];

fn fail(path: &str, message: &str) -> String {
    format!("{path}: {message}")
}

fn exact_keys(value: &Value, expected: &[&str], path: &str) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| fail(path, "must be an object"))?;
    let actual: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let expected: BTreeSet<&str> = expected.iter().copied().collect();
    if actual != expected {
        return Err(fail(
            path,
            &format!(
                "keys differ; expected {:?}, received {:?}",
                expected, actual
            ),
        ));
    }
    Ok(())
}

fn allowed_keys(value: &Value, allowed: &[&str], path: &str) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| fail(path, "must be an object"))?;
    if let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(fail(path, &format!("unknown field {key}")));
    }
    Ok(())
}

fn text<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-empty string"))
}

fn number(value: &Value, key: &str, path: &str) -> Result<f64, String> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .filter(|number| number.is_finite())
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a finite number"))
}

fn integer(value: &Value, key: &str, path: &str) -> Result<i64, String> {
    value
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be an integer"))
}

fn boolean(value: &Value, key: &str, path: &str) -> Result<bool, String> {
    value
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a boolean"))
}

fn validate_statuses(value: &Value, fields: &[&str], path: &str) -> Result<(), String> {
    for field in fields {
        let status = text(value, field, path)?;
        if !VALID_STATUSES.contains(&status) {
            return Err(fail(
                &format!("{path}.{field}"),
                "status must be passed, unknown or failed",
            ));
        }
    }
    Ok(())
}

fn validate_capabilities(value: &Value, path: &str, exact: bool) -> Result<(), String> {
    if exact {
        exact_keys(value, &CAPABILITY_FIELDS, path)?;
    } else {
        allowed_keys(value, &CAPABILITY_FIELDS, path)?;
        if value.as_object().is_none_or(|object| object.is_empty()) {
            return Err(fail(path, "override object must not be empty"));
        }
    }
    for field in value.as_object().unwrap().keys() {
        if !CAPABILITY_FIELDS.contains(&field.as_str()) {
            return Err(fail(path, "contains an unknown capability"));
        }
        if value.get(field).and_then(Value::as_bool).is_none() {
            return Err(fail(&format!("{path}.{field}"), "must be a boolean"));
        }
    }
    Ok(())
}

fn validate_gates(value: &Value, path: &str, exact: bool) -> Result<(), String> {
    if exact {
        exact_keys(value, &GATE_FIELDS, path)?;
        validate_statuses(value, &GATE_FIELDS, path)?;
    } else {
        allowed_keys(value, &GATE_FIELDS, path)?;
        if value.as_object().is_none_or(|object| object.is_empty()) {
            return Err(fail(path, "override object must not be empty"));
        }
        for (field, status) in value.as_object().unwrap() {
            if !GATE_FIELDS.contains(&field.as_str())
                || status.as_str().is_none_or(|status| !VALID_STATUSES.contains(&status))
            {
                return Err(fail(
                    &format!("{path}.{field}"),
                    "unknown gate or invalid status",
                ));
            }
        }
    }
    Ok(())
}

fn validate_fixture(input: &Value) -> Result<(), String> {
    exact_keys(
        input,
        &[
            "schema_version",
            "benchmark_id",
            "provenance",
            "claim_ceiling",
            "missions",
            "plans",
            "stress_cases",
        ],
        "$",
    )?;
    if text(input, "schema_version", "$")? != "0.1"
        || text(input, "benchmark_id", "$")? != "mission-resilience-v0.1"
        || text(input, "claim_ceiling", "$")? != CLAIM_CEILING
    {
        return Err(fail("$", "benchmark identity or claim ceiling mismatch"));
    }

    let provenance = input
        .get("provenance")
        .ok_or_else(|| fail("$.provenance", "missing"))?;
    exact_keys(
        provenance,
        &["kind", "external_real_world_data_included", "notes"],
        "$.provenance",
    )?;
    if text(provenance, "kind", "$.provenance")? != "synthetic_only"
        || boolean(provenance, "external_real_world_data_included", "$.provenance")?
    {
        return Err(fail(
            "$.provenance",
            "only explicitly synthetic fixtures without real-world data are accepted",
        ));
    }
    let _ = text(provenance, "notes", "$.provenance")?;

    let missions = input["missions"]
        .as_array()
        .filter(|items| !items.is_empty())
        .ok_or_else(|| fail("$.missions", "must be a non-empty array"))?;
    let mut mission_ids = BTreeSet::new();
    for (index, mission) in missions.iter().enumerate() {
        let path = format!("$.missions[{index}]");
        exact_keys(
            mission,
            &[
                "mission_id",
                "environment_id",
                "deadline_minutes",
                "minimum_energy_reserve_pct",
                "minimum_condition_margin_c",
                "requirements",
            ],
            &path,
        )?;
        let id = text(mission, "mission_id", &path)?;
        let environment = text(mission, "environment_id", &path)?;
        if !["amazon_basin", "congo_basin", "antarctica", "sahara", "afghanistan"]
            .contains(&environment)
        {
            return Err(fail(&path, "environment_id is not a supported benchmark environment"));
        }
        if !mission_ids.insert(id.to_string()) {
            return Err(fail(&path, "duplicate mission_id"));
        }
        let deadline = integer(mission, "deadline_minutes", &path)?;
        if !(1..=100_000_000).contains(&deadline) {
            return Err(fail(&path, "deadline_minutes must be between 1 and 100000000"));
        }
        let reserve = number(mission, "minimum_energy_reserve_pct", &path)?;
        if !(0.0..=100.0).contains(&reserve) {
            return Err(fail(&path, "minimum energy reserve must be between 0 and 100"));
        }
        let condition_minimum = number(mission, "minimum_condition_margin_c", &path)?;
        if !(-1000.0..=1000.0).contains(&condition_minimum) {
            return Err(fail(&path, "minimum condition margin must be between -1000 and 1000"));
        }
        let requirements = &mission["requirements"];
        let requirement_path = format!("{path}.requirements");
        exact_keys(requirements, &CAPABILITY_FIELDS, &requirement_path)?;
        for field in CAPABILITY_FIELDS {
            let _ = boolean(requirements, field, &requirement_path)?;
        }
    }

    let plans = input["plans"]
        .as_array()
        .filter(|items| !items.is_empty())
        .ok_or_else(|| fail("$.plans", "must be a non-empty array"))?;
    let mut plan_ids = BTreeSet::new();
    let mut plans_per_mission: BTreeMap<String, usize> = BTreeMap::new();
    for (index, plan) in plans.iter().enumerate() {
        let path = format!("$.plans[{index}]");
        exact_keys(
            plan,
            &[
                "plan_id",
                "mission_id",
                "mode_chain",
                "duration_minutes",
                "energy_reserve_pct",
                "condition_margin_c",
                "capabilities",
                "gates",
            ],
            &path,
        )?;
        let id = text(plan, "plan_id", &path)?;
        let mission_id = text(plan, "mission_id", &path)?;
        if !plan_ids.insert(id.to_string()) {
            return Err(fail(&path, "duplicate plan_id"));
        }
        if !mission_ids.contains(mission_id) {
            return Err(fail(&path, "references an unknown mission"));
        }
        *plans_per_mission.entry(mission_id.to_string()).or_default() += 1;
        let mode_chain = plan["mode_chain"]
            .as_array()
            .filter(|items| !items.is_empty())
            .ok_or_else(|| fail(&format!("{path}.mode_chain"), "must be non-empty"))?;
        if mode_chain.iter().any(|mode| mode.as_str().is_none_or(|s| s.trim().is_empty())) {
            return Err(fail(&format!("{path}.mode_chain"), "modes must be non-empty strings"));
        }
        let duration = integer(plan, "duration_minutes", &path)?;
        if !(1..=100_000_000).contains(&duration) {
            return Err(fail(&path, "duration_minutes must be between 1 and 100000000"));
        }
        let reserve = number(plan, "energy_reserve_pct", &path)?;
        if !(0.0..=100.0).contains(&reserve) {
            return Err(fail(&path, "energy reserve must be between 0 and 100"));
        }
        let condition_margin = number(plan, "condition_margin_c", &path)?;
        if !(-1000.0..=1000.0).contains(&condition_margin) {
            return Err(fail(&path, "condition margin must be between -1000 and 1000"));
        }
        validate_capabilities(&plan["capabilities"], &format!("{path}.capabilities"), true)?;
        validate_gates(&plan["gates"], &format!("{path}.gates"), true)?;
    }
    if mission_ids.iter().any(|id| !plans_per_mission.contains_key(id)) {
        return Err(fail("$.plans", "every mission must have at least one candidate plan"));
    }

    let cases = input["stress_cases"]
        .as_array()
        .filter(|items| !items.is_empty())
        .ok_or_else(|| fail("$.stress_cases", "must be a non-empty array"))?;
    let mut case_ids = BTreeSet::new();
    let mut cases_per_mission: BTreeMap<String, usize> = BTreeMap::new();
    for (index, case) in cases.iter().enumerate() {
        let path = format!("$.stress_cases[{index}]");
        exact_keys(
            case,
            &[
                "case_id",
                "mission_id",
                "description",
                "expected_feasible_plan",
                "impacts",
            ],
            &path,
        )?;
        let case_id = text(case, "case_id", &path)?;
        let mission_id = text(case, "mission_id", &path)?;
        if !case_ids.insert(case_id.to_string()) {
            return Err(fail(&path, "duplicate case_id"));
        }
        if !mission_ids.contains(mission_id) {
            return Err(fail(&path, "references an unknown mission"));
        }
        let _ = text(case, "description", &path)?;
        let _ = boolean(case, "expected_feasible_plan", &path)?;
        *cases_per_mission.entry(mission_id.to_string()).or_default() += 1;
        let impacts = case["impacts"]
            .as_array()
            .filter(|items| !items.is_empty())
            .ok_or_else(|| fail(&format!("{path}.impacts"), "must be non-empty"))?;
        let mut impacted_ids = BTreeSet::new();
        for (impact_index, impact) in impacts.iter().enumerate() {
            let impact_path = format!("{path}.impacts[{impact_index}]");
            allowed_keys(
                impact,
                &[
                    "plan_id",
                    "available",
                    "duration_delta_minutes",
                    "energy_reserve_delta_pct",
                    "condition_margin_delta_c",
                    "capability_overrides",
                    "gate_overrides",
                ],
                &impact_path,
            )?;
            for field in [
                "plan_id",
                "available",
                "duration_delta_minutes",
                "energy_reserve_delta_pct",
                "condition_margin_delta_c",
            ] {
                if impact.get(field).is_none() {
                    return Err(fail(&impact_path, &format!("missing required field {field}")));
                }
            }
            let plan_id = text(impact, "plan_id", &impact_path)?;
            if !impacted_ids.insert(plan_id.to_string()) {
                return Err(fail(&impact_path, "duplicate plan impact in one stress case"));
            }
            let plan = plans
                .iter()
                .find(|plan| plan["plan_id"].as_str() == Some(plan_id))
                .ok_or_else(|| fail(&impact_path, "references an unknown plan"))?;
            if plan["mission_id"].as_str() != Some(mission_id) {
                return Err(fail(&impact_path, "impact plan belongs to another mission"));
            }
            let _ = boolean(impact, "available", &impact_path)?;
            let duration_delta = integer(impact, "duration_delta_minutes", &impact_path)?;
            if !(0..=100_000_000).contains(&duration_delta) {
                return Err(fail(&impact_path, "duration delta must be between 0 and 100000000"));
            }
            let energy_delta = number(impact, "energy_reserve_delta_pct", &impact_path)?;
            if !(-100.0..=100.0).contains(&energy_delta) {
                return Err(fail(&impact_path, "energy reserve delta must be between -100 and 100"));
            }
            let condition_delta = number(impact, "condition_margin_delta_c", &impact_path)?;
            if !(-2000.0..=2000.0).contains(&condition_delta) {
                return Err(fail(&impact_path, "condition margin delta must be between -2000 and 2000"));
            }
            if let Some(overrides) = impact.get("capability_overrides") {
                validate_capabilities(
                    overrides,
                    &format!("{impact_path}.capability_overrides"),
                    false,
                )?;
            }
            if let Some(overrides) = impact.get("gate_overrides") {
                validate_gates(overrides, &format!("{impact_path}.gate_overrides"), false)?;
            }
        }
    }
    if mission_ids.iter().any(|id| !cases_per_mission.contains_key(id)) {
        return Err(fail("$.stress_cases", "every mission must have at least one stress case"));
    }
    Ok(())
}

fn overlay_map(
    base: &Value,
    overrides: Option<&Value>,
    fields: &[&str],
    path: &str,
) -> Result<BTreeMap<String, Value>, String> {
    let base_object = base
        .as_object()
        .ok_or_else(|| fail(path, "must be an object"))?;
    let mut output: BTreeMap<String, Value> = base_object
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    if let Some(override_object) = overrides.and_then(Value::as_object) {
        for (key, value) in override_object {
            if !fields.contains(&key.as_str()) {
                return Err(fail(path, "override contains an unknown field"));
            }
            output.insert(key.clone(), value.clone());
        }
    }
    Ok(output)
}

fn assess_plan(
    mission: &Value,
    plan: &Value,
    scenario_id: &str,
    impact: Option<&Value>,
) -> Result<Value, String> {
    let plan_id = text(plan, "plan_id", "$.plans")?;
    let mission_id = text(mission, "mission_id", "$.missions")?;
    let impact_path = format!("scenario[{scenario_id}].plan[{plan_id}]");
    let available = impact
        .map(|value| boolean(value, "available", &impact_path))
        .transpose()?
        .unwrap_or(true);
    let duration_delta = impact
        .map(|value| integer(value, "duration_delta_minutes", &impact_path))
        .transpose()?
        .unwrap_or(0);
    let energy_delta = impact
        .map(|value| number(value, "energy_reserve_delta_pct", &impact_path))
        .transpose()?
        .unwrap_or(0.0);
    let margin_delta = impact
        .map(|value| number(value, "condition_margin_delta_c", &impact_path))
        .transpose()?
        .unwrap_or(0.0);

    let duration = integer(plan, "duration_minutes", "$.plans")? + duration_delta;
    let energy_reserve =
        (number(plan, "energy_reserve_pct", "$.plans")? + energy_delta).clamp(0.0, 100.0);
    let condition_margin = number(plan, "condition_margin_c", "$.plans")? + margin_delta;
    let capabilities = overlay_map(
        &plan["capabilities"],
        impact.and_then(|value| value.get("capability_overrides")),
        &CAPABILITY_FIELDS,
        &impact_path,
    )?;
    let gates = overlay_map(
        &plan["gates"],
        impact.and_then(|value| value.get("gate_overrides")),
        &GATE_FIELDS,
        &impact_path,
    )?;
    let failed_gate_fields: Vec<String> = GATE_FIELDS
        .iter()
        .filter(|field| gates.get(**field).and_then(Value::as_str) == Some("failed"))
        .map(|field| (*field).to_string())
        .collect();
    let unknown_gate_fields: Vec<String> = GATE_FIELDS
        .iter()
        .filter(|field| gates.get(**field).and_then(Value::as_str) == Some("unknown"))
        .map(|field| (*field).to_string())
        .collect();

    let mut constraint_failures = Vec::new();
    if duration > integer(mission, "deadline_minutes", "$.missions")? {
        constraint_failures.push("worst_case_duration_exceeds_deadline".to_string());
    }
    if energy_reserve < number(mission, "minimum_energy_reserve_pct", "$.missions")? {
        constraint_failures.push("energy_reserve_below_required_minimum".to_string());
    }
    if condition_margin < number(mission, "minimum_condition_margin_c", "$.missions")? {
        constraint_failures.push("condition_margin_below_required_minimum".to_string());
    }
    for field in CAPABILITY_FIELDS {
        if boolean(&mission["requirements"], field, "$.missions.requirements")?
            && capabilities.get(field).and_then(Value::as_bool) != Some(true)
        {
            constraint_failures.push(format!("required_capability_missing:{field}"));
        }
    }

    let disposition = if !available {
        "blocked_unavailable"
    } else if !failed_gate_fields.is_empty() {
        "blocked_hard_gate"
    } else if !constraint_failures.is_empty() {
        "blocked_constraints"
    } else if !unknown_gate_fields.is_empty() {
        "needs_gate_evidence"
    } else {
        "eligible_for_human_review"
    };
    let modes: Vec<&str> = plan["mode_chain"]
        .as_array()
        .unwrap()
        .iter()
        .map(|mode| mode.as_str().unwrap())
        .collect();
    Ok(json!({
        "mission_id": mission_id,
        "plan_id": plan_id,
        "scenario_id": scenario_id,
        "available": available,
        "mode_chain": modes,
        "effective_duration_minutes": duration,
        "effective_energy_reserve_pct": energy_reserve,
        "effective_condition_margin_c": condition_margin,
        "effective_capabilities": capabilities,
        "effective_gates": gates,
        "failed_gate_fields": failed_gate_fields,
        "unknown_gate_fields": unknown_gate_fields,
        "constraint_failures": constraint_failures,
        "disposition": disposition
    }))
}

fn metric_number<'a>(assessment: &'a Value, key: &str) -> f64 {
    assessment[key].as_f64().unwrap()
}

fn dominates(left: &Value, right: &Value) -> bool {
    let left_duration = metric_number(left, "effective_duration_minutes");
    let right_duration = metric_number(right, "effective_duration_minutes");
    let left_energy = metric_number(left, "effective_energy_reserve_pct");
    let right_energy = metric_number(right, "effective_energy_reserve_pct");
    let left_margin = metric_number(left, "effective_condition_margin_c");
    let right_margin = metric_number(right, "effective_condition_margin_c");
    let epsilon = 1e-9;
    let no_worse = left_duration <= right_duration + epsilon
        && left_energy + epsilon >= right_energy
        && left_margin + epsilon >= right_margin;
    let strictly_better = left_duration < right_duration - epsilon
        || left_energy > right_energy + epsilon
        || left_margin > right_margin + epsilon;
    no_worse && strictly_better
}

fn pareto_ids(assessments: &[Value]) -> Vec<String> {
    let eligible: Vec<&Value> = assessments
        .iter()
        .filter(|assessment| {
            assessment["disposition"].as_str() == Some("eligible_for_human_review")
        })
        .collect();
    let mut output: Vec<String> = eligible
        .iter()
        .filter(|candidate| {
            !eligible
                .iter()
                .any(|other| other["plan_id"] != candidate["plan_id"] && dominates(other, candidate))
        })
        .map(|candidate| candidate["plan_id"].as_str().unwrap().to_string())
        .collect();
    output.sort();
    output
}

fn eligible_ids(assessments: &[Value]) -> Vec<String> {
    let mut ids: Vec<String> = assessments
        .iter()
        .filter(|assessment| {
            assessment["disposition"].as_str() == Some("eligible_for_human_review")
        })
        .map(|assessment| assessment["plan_id"].as_str().unwrap().to_string())
        .collect();
    ids.sort();
    ids
}

fn percent(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    ((numerator as f64 / denominator as f64) * 10_000.0).round() / 100.0
}

fn audit(input: &Value) -> Result<Value, String> {
    validate_fixture(input)?;
    let mut mission_map: BTreeMap<String, Value> = BTreeMap::new();
    for mission in input["missions"].as_array().unwrap() {
        mission_map.insert(mission["mission_id"].as_str().unwrap().to_string(), mission.clone());
    }
    let mut plans_by_mission: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for plan in input["plans"].as_array().unwrap() {
        plans_by_mission
            .entry(plan["mission_id"].as_str().unwrap().to_string())
            .or_default()
            .push(plan.clone());
    }
    for plans in plans_by_mission.values_mut() {
        plans.sort_by(|left, right| left["plan_id"].as_str().cmp(&right["plan_id"].as_str()));
    }

    let mut baseline_assessments = Vec::new();
    let mut baseline_by_mission: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for (mission_id, plans) in &plans_by_mission {
        let mission = mission_map.get(mission_id).unwrap();
        let mut assessments = Vec::new();
        for plan in plans {
            assessments.push(assess_plan(mission, plan, "baseline", None)?);
        }
        baseline_assessments.extend(assessments.clone());
        baseline_by_mission.insert(mission_id.clone(), assessments);
    }
    baseline_assessments.sort_by(|left, right| {
        left["plan_id"].as_str().cmp(&right["plan_id"].as_str())
    });

    let mut scenario_assessments = Vec::new();
    let mut expected_feasible_case_count = 0usize;
    let mut expected_viability_oracle_match_count = 0usize;
    let mut stress_cases_with_feasible_plan_count = 0usize;
    let mut expected_feasible_cases_with_plan_count = 0usize;
    let mut expected_no_plan_control_count = 0usize;
    let mut expected_no_plan_control_correct_count = 0usize;

    let mut sorted_cases = input["stress_cases"].as_array().unwrap().clone();
    sorted_cases.sort_by(|left, right| left["case_id"].as_str().cmp(&right["case_id"].as_str()));
    for case in &sorted_cases {
        let case_id = case["case_id"].as_str().unwrap();
        let mission_id = case["mission_id"].as_str().unwrap();
        let mission = mission_map.get(mission_id).unwrap();
        let impacts: BTreeMap<String, Value> = case["impacts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|impact| {
                (
                    impact["plan_id"].as_str().unwrap().to_string(),
                    impact.clone(),
                )
            })
            .collect();
        let mut assessments = Vec::new();
        for plan in plans_by_mission.get(mission_id).unwrap() {
            let impact = impacts.get(plan["plan_id"].as_str().unwrap());
            assessments.push(assess_plan(mission, plan, case_id, impact)?);
        }
        let viable_ids = eligible_ids(&assessments);
        let feasible_plan_count = viable_ids.len();
        let actually_feasible = feasible_plan_count > 0;
        let expected_feasible = case["expected_feasible_plan"].as_bool().unwrap();
        let outcome_matches_expectation = actually_feasible == expected_feasible;
        if expected_feasible {
            expected_feasible_case_count += 1;
            if actually_feasible {
                expected_feasible_cases_with_plan_count += 1;
            }
        } else {
            expected_no_plan_control_count += 1;
            if !actually_feasible {
                expected_no_plan_control_correct_count += 1;
            }
        }
        if actually_feasible {
            stress_cases_with_feasible_plan_count += 1;
        }
        if outcome_matches_expectation {
            expected_viability_oracle_match_count += 1;
        }
        scenario_assessments.push(json!({
            "case_id": case_id,
            "mission_id": mission_id,
            "environment_id": mission["environment_id"],
            "description": case["description"],
            "expected_feasible_plan": expected_feasible,
            "feasible_plan_ids": viable_ids,
            "pareto_plan_ids": pareto_ids(&assessments),
            "feasible_plan_count": feasible_plan_count,
            "no_admissible_plan": !actually_feasible,
            "outcome_matches_expectation": outcome_matches_expectation,
            "candidate_assessments": assessments
        }));
    }

    let mut mission_summaries = Vec::new();
    let mut sorted_missions = input["missions"].as_array().unwrap().clone();
    sorted_missions.sort_by(|left, right| left["mission_id"].as_str().cmp(&right["mission_id"].as_str()));
    for mission in &sorted_missions {
        let mission_id = mission["mission_id"].as_str().unwrap();
        let baseline = baseline_by_mission.get(mission_id).unwrap();
        let mission_cases: Vec<&Value> = scenario_assessments
            .iter()
            .filter(|case| case["mission_id"].as_str() == Some(mission_id))
            .collect();
        let viable_cases = mission_cases
            .iter()
            .filter(|case| case["feasible_plan_count"].as_u64().unwrap() > 0)
            .count();
        let expected_viable_cases = mission_cases
            .iter()
            .filter(|case| case["expected_feasible_plan"].as_bool() == Some(true))
            .count();
        let viable_expected_cases = mission_cases
            .iter()
            .filter(|case| {
                case["expected_feasible_plan"].as_bool() == Some(true)
                    && case["feasible_plan_count"].as_u64().unwrap() > 0
            })
            .count();
        mission_summaries.push(json!({
            "mission_id": mission_id,
            "environment_id": mission["environment_id"],
            "candidate_plan_count": plans_by_mission.get(mission_id).unwrap().len(),
            "baseline_feasible_plan_ids": eligible_ids(baseline),
            "baseline_pareto_plan_ids": pareto_ids(baseline),
            "stress_case_count": mission_cases.len(),
            "stress_cases_with_feasible_plan_count": viable_cases,
            "all_stress_case_coverage_percent": percent(viable_cases, mission_cases.len()),
            "expected_recoverable_case_count": expected_viable_cases,
            "expected_recoverable_case_coverage_percent": percent(viable_expected_cases, expected_viable_cases)
        }));
    }

    let mut environment_ids = BTreeSet::new();
    for mission in input["missions"].as_array().unwrap() {
        environment_ids.insert(mission["environment_id"].as_str().unwrap().to_string());
    }
    let mut environment_summaries = Vec::new();
    for environment_id in environment_ids {
        let environment_mission_ids: Vec<String> = mission_map
            .values()
            .filter(|mission| mission["environment_id"].as_str() == Some(environment_id.as_str()))
            .map(|mission| mission["mission_id"].as_str().unwrap().to_string())
            .collect();
        let mut candidate_plan_count = 0usize;
        let mut baseline_feasible_plan_count = 0usize;
        let mut stress_case_count = 0usize;
        let mut stress_cases_with_feasible_plan_count = 0usize;
        let mut expected_recoverable_case_count = 0usize;
        let mut expected_recoverable_case_with_plan_count = 0usize;
        let mut expected_no_plan_control_count = 0usize;
        let mut expected_no_plan_control_correct_count = 0usize;

        for mission_id in &environment_mission_ids {
            candidate_plan_count += plans_by_mission.get(mission_id).unwrap().len();
            baseline_feasible_plan_count +=
                eligible_ids(baseline_by_mission.get(mission_id).unwrap()).len();
            for case in scenario_assessments
                .iter()
                .filter(|case| case["mission_id"].as_str() == Some(mission_id))
            {
                stress_case_count += 1;
                let has_plan = case["feasible_plan_count"].as_u64().unwrap() > 0;
                let expected_plan = case["expected_feasible_plan"].as_bool().unwrap();
                if has_plan {
                    stress_cases_with_feasible_plan_count += 1;
                }
                if expected_plan {
                    expected_recoverable_case_count += 1;
                    if has_plan {
                        expected_recoverable_case_with_plan_count += 1;
                    }
                } else {
                    expected_no_plan_control_count += 1;
                    if !has_plan {
                        expected_no_plan_control_correct_count += 1;
                    }
                }
            }
        }
        environment_summaries.push(json!({
            "environment_id": environment_id,
            "mission_count": environment_mission_ids.len(),
            "candidate_plan_count": candidate_plan_count,
            "baseline_feasible_plan_count": baseline_feasible_plan_count,
            "stress_case_count": stress_case_count,
            "stress_cases_with_feasible_plan_count": stress_cases_with_feasible_plan_count,
            "all_stress_case_coverage_percent": percent(
                stress_cases_with_feasible_plan_count,
                stress_case_count
            ),
            "expected_recoverable_case_count": expected_recoverable_case_count,
            "expected_recoverable_case_coverage_percent": percent(
                expected_recoverable_case_with_plan_count,
                expected_recoverable_case_count
            ),
            "expected_no_plan_control_count": expected_no_plan_control_count,
            "expected_no_plan_control_correct_count": expected_no_plan_control_correct_count
        }));
    }

    let total_cases = sorted_cases.len();
    let baseline_eligible_count = eligible_ids(&baseline_assessments).len();
    let expected_viability_oracle_match_percent =
        percent(expected_viability_oracle_match_count, total_cases);
    Ok(json!({
        "benchmark_id": "mission-resilience-v0.1",
        "claim_ceiling": CLAIM_CEILING,
        "provenance_kind": "synthetic_only",
        "mission_count": mission_map.len(),
        "environment_count": environment_summaries.len(),
        "candidate_plan_count": input["plans"].as_array().unwrap().len(),
        "stress_case_count": total_cases,
        "baseline_eligible_plan_count": baseline_eligible_count,
        "stress_cases_with_feasible_plan_count": stress_cases_with_feasible_plan_count,
        "all_stress_case_coverage_percent": percent(stress_cases_with_feasible_plan_count, total_cases),
        "expected_recoverable_case_count": expected_feasible_case_count,
        "expected_recoverable_case_coverage_percent": percent(
            expected_feasible_cases_with_plan_count,
            expected_feasible_case_count
        ),
        "expected_no_plan_control_count": expected_no_plan_control_count,
        "expected_no_plan_control_correct_count": expected_no_plan_control_correct_count,
        "expected_viability_oracle_match_count": expected_viability_oracle_match_count,
        "expected_viability_oracle_match_percent": expected_viability_oracle_match_percent,
        "weighted_utility_score_produced": false,
        "universal_winner_produced": false,
        "route_or_deployment_authorized": false,
        "patient_care_authorized": false,
        "autonomous_hazard_handling_authorized": false,
        "real_world_validated": false,
        "baseline_assessments": baseline_assessments,
        "scenario_assessments": scenario_assessments,
        "mission_summaries": mission_summaries,
        "environment_summaries": environment_summaries
    }))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() > 1 {
        return Err("usage: mission-resilience-audit [fixture.json]".to_string());
    }
    let source = if let Some(path) = args.first() {
        fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?
    } else {
        CORPUS.to_string()
    };
    let input: Value =
        serde_json::from_str(&source).map_err(|error| format!("invalid JSON: {error}"))?;
    let report = audit(&input)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("cannot serialize report: {error}"))?
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("mission-resilience-audit: {error}");
        process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(CORPUS).unwrap()
    }

    fn report() -> Value {
        audit(&fixture()).unwrap()
    }

    fn case<'a>(report: &'a Value, case_id: &str) -> &'a Value {
        report["scenario_assessments"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["case_id"].as_str() == Some(case_id))
            .unwrap()
    }

    fn assessment<'a>(report: &'a Value, plan_id: &str) -> &'a Value {
        report["baseline_assessments"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["plan_id"].as_str() == Some(plan_id))
            .unwrap()
    }

    #[test]
    fn fixture_counts_and_regression_outputs_are_explicit() {
        let output = report();
        assert_eq!(output["mission_count"], 5);
        assert_eq!(output["environment_count"], 5);
        assert_eq!(output["candidate_plan_count"], 14);
        assert_eq!(output["stress_case_count"], 10);
        assert_eq!(output["baseline_eligible_plan_count"], 8);
        assert_eq!(output["expected_recoverable_case_count"], 6);
        assert_eq!(output["expected_recoverable_case_coverage_percent"], 100.0);
        assert_eq!(output["expected_no_plan_control_count"], 4);
        assert_eq!(output["expected_no_plan_control_correct_count"], 4);
        assert_eq!(output["expected_viability_oracle_match_count"], 10);
        assert_eq!(output["expected_viability_oracle_match_percent"], 100.0);
        assert_eq!(output["route_or_deployment_authorized"], false);
        assert_eq!(output["real_world_validated"], false);
    }

    #[test]
    fn every_stress_case_matches_its_independent_expected_outcome() {
        let output = report();
        assert!(output["scenario_assessments"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| item["outcome_matches_expectation"] == true));
    }

    #[test]
    fn intentionally_unrecoverable_control_has_no_admissible_plan() {
        let output = report();
        let negative = case(&output, "congo-energy-collapse-negative");
        assert_eq!(negative["expected_feasible_plan"], false);
        assert_eq!(negative["no_admissible_plan"], true);
        assert_eq!(negative["feasible_plan_count"], 0);
        assert_eq!(negative["outcome_matches_expectation"], true);
    }

    #[test]
    fn environment_stratification_keeps_afghanistan_unresolved_and_blocked() {
        let output = report();
        let environments = output["environment_summaries"].as_array().unwrap();
        assert_eq!(environments.len(), 5);
        let afghanistan = environments
            .iter()
            .find(|item| item["environment_id"] == "afghanistan")
            .unwrap();
        assert_eq!(afghanistan["baseline_feasible_plan_count"], 0);
        assert_eq!(afghanistan["stress_cases_with_feasible_plan_count"], 0);
        assert_eq!(afghanistan["expected_no_plan_control_count"], 2);
        assert_eq!(afghanistan["expected_no_plan_control_correct_count"], 2);
        assert!(output["baseline_assessments"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|item| item["mission_id"] == "afghanistan-civilian-aid-payload")
            .all(|item| item["disposition"] == "needs_gate_evidence"));
    }

    #[test]
    fn route_or_hazard_gate_failure_never_becomes_eligible() {
        let mut input = fixture();
        input["plans"][0]["gates"]["route_authorization"] = json!("failed");
        let output = audit(&input).unwrap();
        assert_eq!(
            assessment(&output, "amazon-air-river")["disposition"],
            "blocked_hard_gate"
        );
    }

    #[test]
    fn unknown_gate_evidence_fails_closed() {
        let mut input = fixture();
        input["plans"][0]["gates"]["hazard_status"] = json!("unknown");
        let output = audit(&input).unwrap();
        assert_eq!(
            assessment(&output, "amazon-air-river")["disposition"],
            "needs_gate_evidence"
        );
    }

    #[test]
    fn accessibility_degradation_selects_only_the_still_accessible_plan() {
        let output = report();
        let stress = case(&output, "antarctic-accessibility-degradation");
        assert_eq!(stress["feasible_plan_ids"], json!(["antarctic-sled-accessible"]));
        assert_eq!(
            stress["candidate_assessments"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["plan_id"] == "antarctic-tracked-accessible")
                .unwrap()["disposition"],
            "blocked_constraints"
        );
    }

    #[test]
    fn pareto_frontier_keeps_tradeoffs_without_selecting_a_universal_winner() {
        let output = report();
        let amazon = output["mission_summaries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["mission_id"] == "amazon-medical-cold-chain")
            .unwrap();
        assert_eq!(
            amazon["baseline_pareto_plan_ids"],
            json!(["amazon-air-river", "amazon-river-ground"])
        );
        let polar = output["mission_summaries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["mission_id"] == "antarctic-accessible-relief")
            .unwrap();
        assert_eq!(
            polar["baseline_pareto_plan_ids"],
            json!(["antarctic-tracked-accessible"])
        );
        assert_eq!(output["universal_winner_produced"], false);
        assert_eq!(output["weighted_utility_score_produced"], false);
    }

    #[test]
    fn input_collection_order_does_not_change_the_report() {
        let input = fixture();
        let mut reordered = input.clone();
        reordered["missions"].as_array_mut().unwrap().reverse();
        reordered["plans"].as_array_mut().unwrap().reverse();
        reordered["stress_cases"].as_array_mut().unwrap().reverse();
        assert_eq!(
            serde_json::to_string(&audit(&input).unwrap()).unwrap(),
            serde_json::to_string(&audit(&reordered).unwrap()).unwrap()
        );
    }

    #[test]
    fn exhaustive_six_gate_status_matrix_is_fail_closed() {
        let input = fixture();
        let mission = &input["missions"][0];
        let base_plan = input["plans"][0].clone();
        let states = ["passed", "unknown", "failed"];

        for encoded in 0usize..729 {
            let mut cursor = encoded;
            let mut plan = base_plan.clone();
            let mut any_failed = false;
            let mut any_unknown = false;

            for field in GATE_FIELDS {
                let status = states[cursor % states.len()];
                cursor /= states.len();
                plan["gates"][field] = serde_json::json!(status);
                any_failed |= status == "failed";
                any_unknown |= status == "unknown";
            }

            let result = assess_plan(mission, &plan, "generated-gate-matrix", None).unwrap();
            let expected = if any_failed {
                "blocked_hard_gate"
            } else if any_unknown {
                "needs_gate_evidence"
            } else {
                "eligible_for_human_review"
            };
            assert_eq!(result["disposition"], expected, "gate combination {encoded}");
        }
    }

    #[test]
    fn all_sixty_four_capability_combinations_respect_mission_requirements() {
        let input = fixture();
        let mut mission = input["missions"][0].clone();
        for field in CAPABILITY_FIELDS {
            mission["requirements"][field] = serde_json::json!(true);
        }
        let base_plan = input["plans"][0].clone();

        for mask in 0usize..64 {
            let mut plan = base_plan.clone();
            let mut all_present = true;
            for (index, field) in CAPABILITY_FIELDS.iter().enumerate() {
                let enabled = (mask & (1 << index)) != 0;
                plan["capabilities"][*field] = serde_json::json!(enabled);
                all_present &= enabled;
            }

            let result = assess_plan(&mission, &plan, "generated-capability-matrix", None).unwrap();
            let expected = if all_present {
                "eligible_for_human_review"
            } else {
                "blocked_constraints"
            };
            assert_eq!(result["disposition"], expected, "capability mask {mask}");
        }
    }

    #[test]
    fn metric_bounds_reject_overflow_and_implausible_stress_inputs() {
        let mut oversized_duration = fixture();
        oversized_duration["plans"][0]["duration_minutes"] = serde_json::json!(i64::MAX);
        assert!(audit(&oversized_duration)
            .unwrap_err()
            .contains("duration_minutes must be between"));

        let mut oversized_delta = fixture();
        oversized_delta["stress_cases"][0]["impacts"][0]["duration_delta_minutes"] =
            serde_json::json!(100_000_001);
        assert!(audit(&oversized_delta)
            .unwrap_err()
            .contains("duration delta must be between"));
    }

    #[test]
    fn unknown_fields_and_duplicate_stress_impacts_are_rejected() {
        let mut extra_field = fixture();
        extra_field["plans"][0]["optimistic_score"] = json!(999);
        assert!(audit(&extra_field).unwrap_err().contains("keys differ"));
        let mut duplicate_impact = fixture();
        let duplicate = duplicate_impact["stress_cases"][0]["impacts"][0].clone();
        duplicate_impact["stress_cases"][0]["impacts"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(audit(&duplicate_impact)
            .unwrap_err()
            .contains("duplicate plan impact"));
    }
}

