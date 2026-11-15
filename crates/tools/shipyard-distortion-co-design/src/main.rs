use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const BENCHMARK_ID: &str = "SHIPYARD-DISTORTION-CO-DESIGN-001";
const SCHEMA_VERSION: &str = "0.1.0";
const DEFAULT_CORPUS: &str = "benchmarks/shipyard/distortion-co-design/corpus-v0.1.json";
const DEFAULT_SCHEMA: &str = "docs/tech/SHIPYARD_DISTORTION_CO_DESIGN_V0_1.schema.json";
const DEFAULT_ENUMERABLE_CORPUS: &str =
    "benchmarks/shipyard/distortion-co-design/enumerable-oracle-v0.1.json";
const DEFAULT_ENUMERABLE_SCHEMA: &str =
    "docs/tech/SHIPYARD_DISTORTION_CO_DESIGN_ENUMERABLE_ORACLE_V0_1.schema.json";
const EXPECTED_ORACLE_OBJECTIVE_POLICY: &str = concat!(
    "Minimize all four integer objectives lexicographically only for stable serialization; ",
    "Pareto dominance is componentwise across final_geometry_units, production_time_units, ",
    "fixture_cost_units and robustness_risk_units. Lower is better for each synthetic objective."
);

#[derive(Debug, PartialEq, Eq)]
struct Summary {
    cases: usize,
    positive: usize,
    negative: usize,
}

fn str_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn bool_at(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn number_at(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}

fn string_list(value: &Value, key: &str) -> Option<Vec<&str>> {
    value
        .get(key)?
        .as_array()?
        .iter()
        .map(Value::as_str)
        .collect()
}

fn evaluate_case(case: &Value) -> &'static str {
    for field in ["panel_id", "panel_revision"] {
        if str_at(case, field).is_none_or(str::is_empty) {
            return "BLOCK_CONFIGURATION";
        }
    }
    for field in ["fixture_id", "fixture_revision"] {
        if str_at(case, field).is_none_or(str::is_empty) {
            return "BLOCK_FIXTURE_LIMIT";
        }
    }

    let Some(tasks) = string_list(case, "weld_tasks") else {
        return "BLOCK_SEQUENCE";
    };
    let Some(sequence) = string_list(case, "sequence") else {
        return "BLOCK_SEQUENCE";
    };

    let task_set: HashSet<&str> = tasks.iter().copied().collect();
    let sequence_set: HashSet<&str> = sequence.iter().copied().collect();
    if tasks.len() < 2
        || task_set.len() != tasks.len()
        || sequence.len() != tasks.len()
        || sequence_set.len() != sequence.len()
        || sequence_set != task_set
    {
        return "BLOCK_SEQUENCE";
    }

    let positions: HashMap<&str, usize> = sequence
        .iter()
        .enumerate()
        .map(|(index, task)| (*task, index))
        .collect();

    let Some(precedence) = case.get("precedence").and_then(Value::as_array) else {
        return "BLOCK_SEQUENCE";
    };
    for edge in precedence {
        let Some(pair) = edge.as_array() else {
            return "BLOCK_SEQUENCE";
        };
        if pair.len() != 2 {
            return "BLOCK_SEQUENCE";
        }
        let (Some(before), Some(after)) = (pair[0].as_str(), pair[1].as_str()) else {
            return "BLOCK_SEQUENCE";
        };
        let (Some(before_position), Some(after_position)) =
            (positions.get(before), positions.get(after))
        else {
            return "BLOCK_SEQUENCE";
        };
        if before_position >= after_position {
            return "BLOCK_SEQUENCE";
        }
    }

    let (Some(limit), Some(predicted)) = (
        number_at(case, "fixture_reaction_limit_units"),
        number_at(case, "predicted_fixture_reaction_units"),
    ) else {
        return "BLOCK_FIXTURE_LIMIT";
    };
    if limit < 0 || predicted < 0 || predicted > limit {
        return "BLOCK_FIXTURE_LIMIT";
    }

    let Some(release_events) = case.get("release_events").and_then(Value::as_array) else {
        return "BLOCK_RELEASE_STATE";
    };
    let Some(last_release) = release_events.last() else {
        return "BLOCK_RELEASE_STATE";
    };
    let mut previous_order = None;
    for event in release_events {
        let Some(order) = number_at(event, "event_order") else {
            return "BLOCK_RELEASE_STATE";
        };
        if previous_order.is_some_and(|previous| order <= previous) {
            return "BLOCK_RELEASE_STATE";
        }
        previous_order = Some(order);
    }
    if bool_at(last_release, "cooling_complete") != Some(true)
        || bool_at(last_release, "all_restraints_removed") != Some(true)
        || str_at(last_release, "state_after") != Some("fully_released")
    {
        return "BLOCK_RELEASE_STATE";
    }

    let Some(measurement) = case.get("final_measurement").filter(|v| !v.is_null()) else {
        return "BLOCK_FINAL_MEASUREMENT";
    };
    if str_at(measurement, "panel_id") != str_at(case, "panel_id")
        || str_at(measurement, "panel_revision") != str_at(case, "panel_revision")
        || str_at(measurement, "fixture_id") != str_at(case, "fixture_id")
        || str_at(measurement, "phase") != Some("final_free_state")
        || bool_at(measurement, "cooling_complete") != Some(true)
        || bool_at(measurement, "restraints_removed") != Some(true)
        || str_at(measurement, "datum_id").is_none_or(str::is_empty)
        || str_at(measurement, "coordinate_frame").is_none_or(str::is_empty)
        || bool_at(measurement, "uncertainty_present") != Some(true)
        || str_at(measurement, "evidence_class") != Some("synthetic_contract")
    {
        return "BLOCK_FINAL_MEASUREMENT";
    }

    let Some(model) = case.get("model") else {
        return "BLOCK_MODEL_SCOPE";
    };
    if str_at(model, "model_id").is_none_or(str::is_empty)
        || str_at(model, "model_revision").is_none_or(str::is_empty)
        || str_at(model, "scope") != Some("synthetic_toy")
        || str_at(model, "validation_state") != Some("synthetic_contract_only")
        || bool_at(model, "out_of_domain") != Some(false)
        || bool_at(model, "physical_claim") != Some(false)
    {
        return "BLOCK_MODEL_SCOPE";
    }

    if bool_at(case, "future_information_used") != Some(false)
        || bool_at(case, "resource_conflict") != Some(false)
    {
        return "BLOCK_SCHEDULE";
    }

    let Some(objectives) = string_list(case, "objective_vector") else {
        return "BLOCK_OBJECTIVES";
    };
    let objective_set: HashSet<&str> = objectives.iter().copied().collect();
    let required = [
        "final_free_state_geometry",
        "production_time",
        "fixture_cost",
        "worst_case_robustness",
    ];
    if objective_set.len() != objectives.len()
        || !required.iter().all(|item| objective_set.contains(item))
    {
        return "BLOCK_OBJECTIVES";
    }

    if bool_at(case, "as_built_writeback") != Some(false) {
        return "BLOCK_AUTHORITY";
    }

    "SYNTHETIC_PLAN_CONTRACT_VALID"
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OraclePlan {
    plan_id: String,
    final_geometry_units: i64,
    production_time_units: i64,
    fixture_cost_units: i64,
    robustness_risk_units: i64,
}

#[derive(Debug, PartialEq, Eq)]
struct OracleSummary {
    cases: usize,
    feasible_plans: usize,
    pareto_plans: usize,
}

fn nonnegative_number(value: &Value, key: &str) -> Result<i64, String> {
    let number = number_at(value, key).ok_or_else(|| format!("{key} must be an integer"))?;
    if number < 0 {
        return Err(format!("{key} must be non-negative"));
    }
    Ok(number)
}

fn required_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    str_at(value, key)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| format!("{key} must be a non-empty string"))
}

fn add_checked(total: i64, addend: i64) -> Result<i64, String> {
    total
        .checked_add(addend)
        .ok_or_else(|| "synthetic objective overflow".to_owned())
}

fn collect_permutations(
    remaining: &mut Vec<String>,
    prefix: &mut Vec<String>,
    output: &mut Vec<Vec<String>>,
) {
    if remaining.is_empty() {
        output.push(prefix.clone());
        return;
    }
    for index in 0..remaining.len() {
        let item = remaining.remove(index);
        prefix.push(item.clone());
        collect_permutations(remaining, prefix, output);
        prefix.pop();
        remaining.insert(index, item);
    }
}

fn plan_dominates(left: &OraclePlan, right: &OraclePlan) -> bool {
    let left_values = [
        left.final_geometry_units,
        left.production_time_units,
        left.fixture_cost_units,
        left.robustness_risk_units,
    ];
    let right_values = [
        right.final_geometry_units,
        right.production_time_units,
        right.fixture_cost_units,
        right.robustness_risk_units,
    ];
    let mut strictly_better = false;
    for (left_value, right_value) in left_values.into_iter().zip(right_values) {
        if left_value > right_value {
            return false;
        }
        if left_value < right_value {
            strictly_better = true;
        }
    }
    strictly_better
}

fn enumerate_oracle_case(case: &Value) -> Result<(usize, Vec<String>), String> {
    let case_id = required_string(case, "case_id")?;
    let task_values = case
        .get("tasks")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{case_id}: tasks must be an array"))?;
    if !(2..=4).contains(&task_values.len()) {
        return Err(format!(
            "{case_id}: exact oracle supports two to four tasks"
        ));
    }

    let mut task_ids = Vec::new();
    let mut task_metrics = HashMap::new();
    for task in task_values {
        let task_id = required_string(task, "task_id")?.to_owned();
        if task_metrics.contains_key(&task_id) {
            return Err(format!("{case_id}: duplicate task_id {task_id}"));
        }
        let duration = nonnegative_number(task, "duration_units")?;
        let geometry = nonnegative_number(task, "geometry_units")?;
        let risk = nonnegative_number(task, "risk_units")?;
        task_ids.push(task_id.clone());
        task_metrics.insert(task_id, (duration, geometry, risk));
    }

    let mut precedence = Vec::new();
    let mut precedence_seen = HashSet::new();
    let precedence_values = case
        .get("precedence")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{case_id}: precedence must be an array"))?;
    for edge in precedence_values {
        let pair = edge
            .as_array()
            .filter(|pair| pair.len() == 2)
            .ok_or_else(|| format!("{case_id}: precedence edge must have two task IDs"))?;
        let before = pair[0]
            .as_str()
            .ok_or_else(|| format!("{case_id}: precedence before ID must be a string"))?
            .to_owned();
        let after = pair[1]
            .as_str()
            .ok_or_else(|| format!("{case_id}: precedence after ID must be a string"))?
            .to_owned();
        if !task_metrics.contains_key(&before)
            || !task_metrics.contains_key(&after)
            || before == after
            || !precedence_seen.insert((before.clone(), after.clone()))
        {
            return Err(format!("{case_id}: invalid or duplicate precedence edge"));
        }
        precedence.push((before, after));
    }

    let mut transition_metrics = HashMap::new();
    let transition_values = case
        .get("transitions")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{case_id}: transitions must be an array"))?;
    for transition in transition_values {
        let before = required_string(transition, "before")?.to_owned();
        let after = required_string(transition, "after")?.to_owned();
        if !task_metrics.contains_key(&before) || !task_metrics.contains_key(&after) {
            return Err(format!("{case_id}: transition references an unknown task"));
        }
        let metrics = (
            nonnegative_number(transition, "geometry_penalty_units")?,
            nonnegative_number(transition, "time_penalty_units")?,
            nonnegative_number(transition, "risk_penalty_units")?,
        );
        if transition_metrics
            .insert((before, after), metrics)
            .is_some()
        {
            return Err(format!("{case_id}: duplicate transition penalty"));
        }
    }

    let fixture_values = case
        .get("fixtures")
        .and_then(Value::as_array)
        .filter(|fixtures| !fixtures.is_empty())
        .ok_or_else(|| format!("{case_id}: fixtures must be a non-empty array"))?;
    let mut fixtures = Vec::new();
    let mut fixture_ids = HashSet::new();
    for fixture in fixture_values {
        let fixture_id = required_string(fixture, "fixture_id")?.to_owned();
        if !fixture_ids.insert(fixture_id.clone()) {
            return Err(format!("{case_id}: duplicate fixture_id {fixture_id}"));
        }
        let reaction = nonnegative_number(fixture, "reaction_units")?;
        let limit = nonnegative_number(fixture, "reaction_limit_units")?;
        let approved = bool_at(fixture, "approved")
            .ok_or_else(|| format!("{case_id}: fixture approved must be a boolean"))?;
        let geometry = nonnegative_number(fixture, "geometry_bias_units")?;
        let setup = nonnegative_number(fixture, "setup_time_units")?;
        let cost = nonnegative_number(fixture, "lifecycle_cost_units")?;
        let risk = nonnegative_number(fixture, "risk_units")?;
        if approved && reaction <= limit {
            fixtures.push((fixture_id, geometry, setup, cost, risk));
        }
    }

    let release_values = case
        .get("releases")
        .and_then(Value::as_array)
        .filter(|releases| !releases.is_empty())
        .ok_or_else(|| format!("{case_id}: releases must be a non-empty array"))?;
    let mut releases = Vec::new();
    let mut release_ids = HashSet::new();
    for release in release_values {
        let release_id = required_string(release, "release_id")?.to_owned();
        if !release_ids.insert(release_id.clone()) {
            return Err(format!("{case_id}: duplicate release_id {release_id}"));
        }
        let cooling_complete = bool_at(release, "cooling_complete")
            .ok_or_else(|| format!("{case_id}: release cooling_complete must be a boolean"))?;
        let fully_released = bool_at(release, "all_restraints_removed").ok_or_else(|| {
            format!("{case_id}: release all_restraints_removed must be a boolean")
        })?;
        let geometry = nonnegative_number(release, "geometry_penalty_units")?;
        let time = nonnegative_number(release, "time_units")?;
        let risk = nonnegative_number(release, "risk_units")?;
        if cooling_complete && fully_released {
            releases.push((release_id, geometry, time, risk));
        }
    }

    let mut permutations = Vec::new();
    let mut remaining_tasks = task_ids.clone();
    let mut prefix = Vec::new();
    collect_permutations(&mut remaining_tasks, &mut prefix, &mut permutations);

    let mut plans = Vec::new();
    for sequence in permutations {
        let positions: HashMap<&str, usize> = sequence
            .iter()
            .enumerate()
            .map(|(index, task_id)| (task_id.as_str(), index))
            .collect();
        if precedence
            .iter()
            .any(|(before, after)| positions.get(before.as_str()) >= positions.get(after.as_str()))
        {
            continue;
        }

        let mut task_duration = 0i64;
        let mut task_geometry = 0i64;
        let mut task_risk = 0i64;
        for task_id in &sequence {
            let (duration, geometry, risk) = task_metrics
                .get(task_id)
                .copied()
                .ok_or_else(|| format!("{case_id}: sequence references an unknown task"))?;
            task_duration = add_checked(task_duration, duration)?;
            task_geometry = add_checked(task_geometry, geometry)?;
            task_risk = add_checked(task_risk, risk)?;
        }

        let mut transition_geometry = 0i64;
        let mut transition_time = 0i64;
        let mut transition_risk = 0i64;
        for pair in sequence.windows(2) {
            if let Some((geometry, time, risk)) =
                transition_metrics.get(&(pair[0].clone(), pair[1].clone()))
            {
                transition_geometry = add_checked(transition_geometry, *geometry)?;
                transition_time = add_checked(transition_time, *time)?;
                transition_risk = add_checked(transition_risk, *risk)?;
            }
        }

        for (fixture_id, fixture_geometry, fixture_setup, fixture_cost, fixture_risk) in &fixtures {
            for (release_id, release_geometry, release_time, release_risk) in &releases {
                let final_geometry_units = add_checked(
                    add_checked(
                        add_checked(task_geometry, transition_geometry)?,
                        *fixture_geometry,
                    )?,
                    *release_geometry,
                )?;
                let production_time_units = add_checked(
                    add_checked(add_checked(task_duration, transition_time)?, *fixture_setup)?,
                    *release_time,
                )?;
                let robustness_risk_units = add_checked(
                    add_checked(add_checked(task_risk, transition_risk)?, *fixture_risk)?,
                    *release_risk,
                )?;
                plans.push(OraclePlan {
                    plan_id: format!("{case_id}|{}|{fixture_id}|{release_id}", sequence.join(">")),
                    final_geometry_units,
                    production_time_units,
                    fixture_cost_units: *fixture_cost,
                    robustness_risk_units,
                });
            }
        }
    }

    let feasible_count = plans.len();
    let mut pareto_ids: Vec<String> = plans
        .iter()
        .filter(|candidate| {
            !plans
                .iter()
                .any(|other| other.plan_id != candidate.plan_id && plan_dominates(other, candidate))
        })
        .map(|plan| plan.plan_id.clone())
        .collect();
    pareto_ids.sort();
    Ok((feasible_count, pareto_ids))
}

fn validate_enumerable_oracle(
    schema_text: &str,
    corpus_text: &str,
) -> Result<OracleSummary, Vec<String>> {
    let mut errors = Vec::new();
    let schema: Value = match serde_json::from_str(schema_text) {
        Ok(value) => value,
        Err(error) => return Err(vec![format!("oracle schema JSON parse failed: {error}")]),
    };
    let corpus: Value = match serde_json::from_str(corpus_text) {
        Ok(value) => value,
        Err(error) => return Err(vec![format!("oracle corpus JSON parse failed: {error}")]),
    };
    if str_at(&schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        errors.push("enumerable oracle schema must declare Draft 2020-12".to_owned());
    }
    if schema.get("additionalProperties").and_then(Value::as_bool) != Some(false) {
        errors.push("enumerable oracle schema must reject undeclared root fields".to_owned());
    }
    if str_at(&corpus, "objective_policy") != Some(EXPECTED_ORACLE_OBJECTIVE_POLICY) {
        errors.push(
            "enumerable oracle objective policy does not match the frozen lower-is-better vector"
                .to_owned(),
        );
    }
    if str_at(&corpus, "schema_version") != Some("0.1.0") {
        errors.push("enumerable oracle schema version must be 0.1.0".to_owned());
    }
    if str_at(&corpus, "benchmark_id")
        != Some("SHIPYARD-DISTORTION-CO-DESIGN-ENUMERABLE-ORACLE-001")
    {
        errors.push("enumerable oracle benchmark_id mismatch".to_owned());
    }
    let Some(cases) = corpus.get("cases").and_then(Value::as_array) else {
        return Err(vec!["enumerable oracle cases must be an array".to_owned()]);
    };
    if cases.len() != 4 {
        errors.push(format!(
            "enumerable oracle requires four cases, found {}",
            cases.len()
        ));
    }

    let mut case_ids = HashSet::new();
    let mut feasible_total = 0usize;
    let mut pareto_total = 0usize;
    for (index, case) in cases.iter().enumerate() {
        let case_id = match required_string(case, "case_id") {
            Ok(id) => id.to_owned(),
            Err(error) => {
                errors.push(format!("cases[{index}]: {error}"));
                continue;
            }
        };
        if !case_ids.insert(case_id.clone()) {
            errors.push(format!("duplicate enumerable case ID {case_id}"));
        }
        let enumeration = match enumerate_oracle_case(case) {
            Ok(result) => result,
            Err(error) => {
                errors.push(format!("{case_id}: {error}"));
                continue;
            }
        };
        let expected_feasible = case
            .get("feasible_plan_count")
            .and_then(Value::as_u64)
            .map(|number| number as usize);
        if expected_feasible != Some(enumeration.0) {
            errors.push(format!(
                "{case_id}: feasible plan count expected {:?}, computed {}",
                expected_feasible, enumeration.0
            ));
        }
        let mut expected_pareto = match string_list(case, "pareto_plan_ids") {
            Some(ids) => ids.into_iter().map(str::to_owned).collect::<Vec<_>>(),
            None => {
                errors.push(format!("{case_id}: pareto_plan_ids must be a string array"));
                Vec::new()
            }
        };
        expected_pareto.sort();
        if expected_pareto != enumeration.1 {
            errors.push(format!(
                "{case_id}: Pareto plan IDs disagree with exhaustive enumeration"
            ));
        }
        feasible_total += enumeration.0;
        pareto_total += enumeration.1.len();
    }

    if errors.is_empty() {
        Ok(OracleSummary {
            cases: cases.len(),
            feasible_plans: feasible_total,
            pareto_plans: pareto_total,
        })
    } else {
        Err(errors)
    }
}

fn validate_manifest(schema_text: &str, corpus_text: &str) -> Result<Summary, Vec<String>> {
    let mut errors = Vec::new();
    let schema: Value = match serde_json::from_str(schema_text) {
        Ok(value) => value,
        Err(error) => return Err(vec![format!("schema JSON parse failed: {error}")]),
    };
    let corpus: Value = match serde_json::from_str(corpus_text) {
        Ok(value) => value,
        Err(error) => return Err(vec![format!("corpus JSON parse failed: {error}")]),
    };

    if str_at(&schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        errors.push("schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if schema.get("additionalProperties").and_then(Value::as_bool) != Some(false) {
        errors.push("schema must reject undeclared top-level fields".to_owned());
    }
    if str_at(&corpus, "schema_version") != Some(SCHEMA_VERSION) {
        errors.push("corpus schema_version must be 0.1.0".to_owned());
    }
    if str_at(&corpus, "benchmark_id") != Some(BENCHMARK_ID) {
        errors.push("corpus benchmark_id does not match this evaluator".to_owned());
    }
    if str_at(&corpus, "claim_ceiling").is_none_or(str::is_empty) {
        errors.push("claim_ceiling must be explicit".to_owned());
    }

    let Some(cases) = corpus.get("cases").and_then(Value::as_array) else {
        return Err(vec!["cases must be an array".to_owned()]);
    };
    if cases.len() != 31 {
        errors.push(format!("v0.1 requires 31 cases, found {}", cases.len()));
    }

    let mut ids = HashSet::new();
    let mut positive = 0usize;
    let mut negative = 0usize;

    for (index, case) in cases.iter().enumerate() {
        let context = format!("cases[{index}]");
        let Some(case_id) = str_at(case, "case_id") else {
            errors.push(format!("{context}.case_id must be a string"));
            continue;
        };
        if !case_id.starts_with("SDC-") || case_id.len() != 7 {
            errors.push(format!("{context}.case_id must match SDC-NNN"));
        }
        if !ids.insert(case_id.to_owned()) {
            errors.push(format!("duplicate case_id {case_id}"));
        }

        match str_at(case, "case_class") {
            Some("positive_contract_control") => positive += 1,
            Some("negative_control") => negative += 1,
            _ => errors.push(format!("{context}.case_class is unknown")),
        }

        let actual = evaluate_case(case);
        let expected = str_at(case, "expected_disposition");
        if expected != Some(actual) {
            errors.push(format!(
                "{context} {case_id}: expected {}, evaluator returned {actual}",
                expected.unwrap_or("<missing>")
            ));
        }

        if str_at(case, "case_class") == Some("positive_contract_control")
            && actual != "SYNTHETIC_PLAN_CONTRACT_VALID"
        {
            errors.push(format!("{context} positive control did not pass"));
        }
        if str_at(case, "case_class") == Some("negative_control")
            && actual == "SYNTHETIC_PLAN_CONTRACT_VALID"
        {
            errors.push(format!("{context} negative control was falsely accepted"));
        }
    }

    if positive != 2 || negative != 29 {
        errors.push(format!(
            "v0.1 requires 2 positive and 29 negative controls; got {positive}/{negative}"
        ));
    }

    if errors.is_empty() {
        Ok(Summary {
            cases: cases.len(),
            positive,
            negative,
        })
    } else {
        Err(errors)
    }
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let first_arg = args.next();
    let is_oracle_mode = first_arg.as_deref() == Some("--oracle");
    let corpus_path = if is_oracle_mode {
        args.next()
            .unwrap_or_else(|| DEFAULT_ENUMERABLE_CORPUS.to_owned())
    } else {
        first_arg.unwrap_or_else(|| DEFAULT_CORPUS.to_owned())
    };
    let schema_path = if is_oracle_mode {
        args.next()
            .unwrap_or_else(|| DEFAULT_ENUMERABLE_SCHEMA.to_owned())
    } else {
        args.next().unwrap_or_else(|| DEFAULT_SCHEMA.to_owned())
    };
    if args.next().is_some() {
        eprintln!(concat!(
            "usage: shipyard-distortion-co-design [CORPUS_PATH] [SCHEMA_PATH] | ",
            "--oracle [ORACLE_CORPUS] [ORACLE_SCHEMA]"
        ));
        return ExitCode::from(2);
    }

    let schema_text = match fs::read_to_string(&schema_path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("cannot read schema {schema_path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let corpus_text = match fs::read_to_string(&corpus_path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("cannot read corpus {corpus_path}: {error}");
            return ExitCode::FAILURE;
        }
    };

    if is_oracle_mode {
        return match validate_enumerable_oracle(&schema_text, &corpus_text) {
            Ok(summary) => {
                println!(
                    concat!(
                        "PASS: exhaustively enumerated {cases} toy cases, ",
                        "{feasible} feasible plans and {pareto} Pareto plans; ",
                        "symbolic objectives only"
                    ),
                    cases = summary.cases,
                    feasible = summary.feasible_plans,
                    pareto = summary.pareto_plans
                );
                ExitCode::SUCCESS
            }
            Err(errors) => {
                eprintln!(
                    "ORACLE FAIL: {} discrepancy/contract error(s)",
                    errors.len()
                );
                for error in errors {
                    eprintln!("- {error}");
                }
                ExitCode::FAILURE
            }
        };
    }

    match validate_manifest(&schema_text, &corpus_text) {
        Ok(summary) => {
            println!(
                "PASS: {cases} cases ({positive} positive / {negative} negative); \
                 fixture/panel identity bound; no physical prediction claimed",
                cases = summary.cases,
                positive = summary.positive,
                negative = summary.negative
            );
            ExitCode::SUCCESS
        }
        Err(errors) => {
            eprintln!("FAIL: {} contract violation(s)", errors.len());
            for error in errors {
                eprintln!("- {error}");
            }
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHEMA_TEXT: &str =
        include_str!("../../../../docs/tech/SHIPYARD_DISTORTION_CO_DESIGN_V0_1.schema.json");
    const CORPUS_TEXT: &str =
        include_str!("../../../../benchmarks/shipyard/distortion-co-design/corpus-v0.1.json");

    fn corpus_value() -> Value {
        serde_json::from_str(CORPUS_TEXT).expect("corpus must parse")
    }

    const ORACLE_SCHEMA_TEXT: &str = include_str!(
        "../../../../docs/tech/SHIPYARD_DISTORTION_CO_DESIGN_ENUMERABLE_ORACLE_V0_1.schema.json"
    );
    const ORACLE_CORPUS_TEXT: &str = include_str!(
        "../../../../benchmarks/shipyard/distortion-co-design/enumerable-oracle-v0.1.json"
    );

    #[test]
    fn exhaustive_oracle_matches_frozen_counts_and_pareto_sets() {
        let summary = validate_enumerable_oracle(ORACLE_SCHEMA_TEXT, ORACLE_CORPUS_TEXT)
            .expect("enumerable oracle should match every frozen known-answer case");
        assert_eq!(
            summary,
            OracleSummary {
                cases: 4,
                feasible_plans: 37,
                pareto_plans: 15,
            }
        );
    }

    #[test]
    fn exhaustive_oracle_rejects_missing_fixture_approval_state() {
        let mut corpus: Value =
            serde_json::from_str(ORACLE_CORPUS_TEXT).expect("oracle corpus must parse");
        corpus["cases"][0]["fixtures"][0]
            .as_object_mut()
            .expect("fixture object")
            .remove("approved");
        let serialized = serde_json::to_string(&corpus).expect("serialize oracle mutation");
        assert!(validate_enumerable_oracle(ORACLE_SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn exhaustive_oracle_rejects_a_falsified_pareto_answer() {
        let mut corpus: Value =
            serde_json::from_str(ORACLE_CORPUS_TEXT).expect("oracle corpus must parse");
        corpus["cases"][1]["pareto_plan_ids"] = Value::Array(Vec::new());
        let serialized = serde_json::to_string(&corpus).expect("serialize oracle mutation");
        assert!(validate_enumerable_oracle(ORACLE_SCHEMA_TEXT, &serialized).is_err());
    }

    #[test]
    fn all_frozen_controls_match_the_independent_disposition_rules() {
        let summary = validate_manifest(SCHEMA_TEXT, CORPUS_TEXT).expect("corpus should validate");
        assert_eq!(
            summary,
            Summary {
                cases: 31,
                positive: 2,
                negative: 29,
            }
        );
    }

    #[test]
    fn in_fixture_measurement_cannot_replace_final_free_state() {
        let mut corpus = corpus_value();
        corpus["cases"][0]["final_measurement"]["phase"] = Value::String("in_fixture".to_owned());
        assert_eq!(
            evaluate_case(&corpus["cases"][0]),
            "BLOCK_FINAL_MEASUREMENT"
        );
    }

    #[test]
    fn missing_release_cannot_be_treated_as_complete() {
        let mut corpus = corpus_value();
        corpus["cases"][0]["release_events"] = Value::Array(Vec::new());
        assert_eq!(evaluate_case(&corpus["cases"][0]), "BLOCK_RELEASE_STATE");
    }

    #[test]
    fn future_information_is_not_admissible() {
        let mut corpus = corpus_value();
        corpus["cases"][0]["future_information_used"] = Value::Bool(true);
        assert_eq!(evaluate_case(&corpus["cases"][0]), "BLOCK_SCHEDULE");
    }

    #[test]
    fn synthetic_state_cannot_be_written_to_as_built_record() {
        let mut corpus = corpus_value();
        corpus["cases"][0]["as_built_writeback"] = Value::Bool(true);
        assert_eq!(evaluate_case(&corpus["cases"][0]), "BLOCK_AUTHORITY");
    }
}
