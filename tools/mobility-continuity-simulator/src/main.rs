use jsonschema::draft202012;
use jsonschema::Validator;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

const SCENARIO_SCHEMA_VERSION: &str = "household-mobility-continuity-benchmark/0.1";
const MODEL_CONFIG_VERSION: &str = "mobility-continuity-model-config/0.1";
const MODEL_ID: &str = "deterministic-household-continuity-proxy/0.1";
const HOURS_PER_DAY: f64 = 24.0;
const DAYS_PER_WEEK: f64 = 7.0;

type Errors = Vec<String>;

#[derive(Clone, Debug)]
struct ModelConfig {
    horizon_days: u32,
    failure_event_days: Vec<u32>,
    hands_on_repair_hours_per_event: f64,
    weekly_essential_mobility_budget_r: f64,
    primary_energy_cost_per_essential_trip_r: f64,
    fallback_cost_per_essential_trip_r: f64,
}

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn number_at(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64)
}

fn bool_at(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn schema_validator(schema: &Value, expected_id: &str) -> Result<Validator, String> {
    if text_at(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if text_at(schema, "$id") != Some(expected_id) {
        return Err(format!("schema $id must be {expected_id}"));
    }
    if !draft202012::meta::is_valid(schema) {
        return Err(format!("schema {expected_id} fails Draft 2020-12 meta-schema validation"));
    }
    draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("cannot compile schema {expected_id}: {error}"))
}

fn validate_instance(validator: &Validator, value: &Value, label: &str) -> Result<(), Errors> {
    let errors: Vec<String> = validator
        .iter_errors(value)
        .map(|error| format!("{label}: {error}"))
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn parse_model_config(value: &Value) -> Result<ModelConfig, Errors> {
    let mut errors = Vec::new();
    if text_at(value, "schema_version") != Some(MODEL_CONFIG_VERSION) {
        errors.push(format!("model config schema_version must be {MODEL_CONFIG_VERSION}"));
    }
    if text_at(value, "model_id") != Some(MODEL_ID) {
        errors.push(format!("model config model_id must be {MODEL_ID}"));
    }

    let horizon_days = value.get("horizon_days").and_then(Value::as_u64);
    if horizon_days != Some(364) {
        errors.push("model config horizon_days must be the frozen synthetic value 364".to_owned());
    }

    let failure_event_days: Vec<u32> = match value.get("failure_event_days").and_then(Value::as_array) {
        Some(items) => items
            .iter()
            .filter_map(Value::as_u64)
            .filter_map(|day| u32::try_from(day).ok())
            .collect(),
        None => {
            errors.push("model config failure_event_days must be an array".to_owned());
            Vec::new()
        }
    };
    if failure_event_days.len() != 2 {
        errors.push("model config must contain exactly two valid failure_event_days".to_owned());
    }
    if failure_event_days.windows(2).any(|pair| pair[0] >= pair[1]) {
        errors.push("failure_event_days must be strictly increasing and unique".to_owned());
    }
    if failure_event_days.iter().any(|day| *day >= 364) {
        errors.push("failure_event_days must lie within the 364-day horizon".to_owned());
    }

    let hands_on = number_at(value, "hands_on_repair_hours_per_event");
    if hands_on.is_none_or(|hours| !hours.is_finite() || hours < 0.0 || hours > 168.0) {
        errors.push("hands_on_repair_hours_per_event must be finite and in [0, 168]".to_owned());
    }
    let budget = number_at(value, "weekly_essential_mobility_budget_r");
    if budget.is_none_or(|amount| !amount.is_finite() || amount <= 0.0) {
        errors.push("weekly_essential_mobility_budget_r must be finite and positive".to_owned());
    }
    let primary_cost = number_at(value, "primary_energy_cost_per_essential_trip_r");
    if primary_cost.is_none_or(|amount| !amount.is_finite() || amount <= 0.0) {
        errors.push("primary_energy_cost_per_essential_trip_r must be finite and positive".to_owned());
    }
    let fallback_cost = number_at(value, "fallback_cost_per_essential_trip_r");
    if fallback_cost.is_none_or(|amount| !amount.is_finite() || amount <= 0.0) {
        errors.push("fallback_cost_per_essential_trip_r must be finite and positive".to_owned());
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(ModelConfig {
        horizon_days: 364,
        failure_event_days,
        hands_on_repair_hours_per_event: hands_on.unwrap_or(0.0),
        weekly_essential_mobility_budget_r: budget.unwrap_or(0.0),
        primary_energy_cost_per_essential_trip_r: primary_cost.unwrap_or(0.0),
        fallback_cost_per_essential_trip_r: fallback_cost.unwrap_or(0.0),
    })
}

fn find_by_id<'a>(items: &'a [Value], id: &str) -> Option<&'a Value> {
    items.iter().find(|item| text_at(item, "id") == Some(id))
}

fn merge_intervals(mut intervals: Vec<(f64, f64)>, horizon_hours: f64) -> Vec<(f64, f64)> {
    for interval in &mut intervals {
        interval.0 = interval.0.max(0.0).min(horizon_hours);
        interval.1 = interval.1.max(0.0).min(horizon_hours);
    }
    intervals.retain(|(start, end)| end > start);
    intervals.sort_by(|left, right| {
        left.0
            .partial_cmp(&right.0)
            .unwrap_or(Ordering::Equal)
    });

    let mut merged: Vec<(f64, f64)> = Vec::new();
    for (start, end) in intervals {
        if let Some(last) = merged.last_mut() {
            if start <= last.1 {
                last.1 = last.1.max(end);
                continue;
            }
        }
        merged.push((start, end));
    }
    merged
}

fn is_unavailable(hour: f64, intervals: &[(f64, f64)]) -> bool {
    intervals.iter().any(|(start, end)| hour >= *start && hour < *end)
}

fn nearest_rank(values: &[f64], quantile: f64) -> Option<f64> {
    if values.is_empty() || !(0.0 < quantile && quantile <= 1.0) {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal));
    let rank = ((quantile * sorted.len() as f64).ceil() as usize).max(1);
    sorted.get(rank - 1).copied()
}

fn effective_repair_hours(
    scenario: &Value,
    architecture: &Value,
    model: &ModelConfig,
) -> Result<f64, String> {
    let stress = scenario.get("stress_parameters").ok_or("scenario lacks stress_parameters")?;
    let raw_delay = number_at(stress, "repair_delay_days").ok_or("repair_delay_days missing or non-numeric")?;
    let fill = number_at(stress, "critical_part_fill_fraction")
        .ok_or("critical_part_fill_fraction missing or non-numeric")?;
    let stock_days = number_at(architecture, "stocked_critical_part_coverage_days")
        .ok_or("stocked_critical_part_coverage_days missing or non-numeric")?;

    if !raw_delay.is_finite() || raw_delay < 0.0
        || !fill.is_finite() || !(0.0..=1.0).contains(&fill)
        || !stock_days.is_finite() || stock_days < 0.0
    {
        return Err("invalid repair delay, part-fill fraction, or stock coverage".to_owned());
    }

    // Synthetic sensitivity proxy only: assumed stock coverage offsets part delay
    // in proportion to the declared fill fraction. It is not a supply-chain model.
    let inventory_credit = raw_delay.min(stock_days) * (1.0 - fill);
    let effective_wait_days = (raw_delay - inventory_credit).max(0.0);
    Ok(effective_wait_days * HOURS_PER_DAY + model.hands_on_repair_hours_per_event)
}

fn simulate_scenario(
    scenario: &Value,
    architecture: &Value,
    model: &ModelConfig,
) -> Result<Value, String> {
    let scenario_id = text_at(scenario, "scenario_id").ok_or("scenario_id missing")?;
    let architecture_id = text_at(scenario, "architecture_id").ok_or("architecture_id missing")?;
    let stressor_id = text_at(scenario, "stressor_id").ok_or("stressor_id missing")?;
    if text_at(architecture, "id") != Some(architecture_id) {
        return Err(format!("{scenario_id}: architecture profile does not match scenario"));
    }

    let demand = scenario.get("demand").ok_or("scenario demand missing")?;
    let essential_per_week = demand
        .get("essential_trips_per_week")
        .and_then(Value::as_u64)
        .ok_or("essential_trips_per_week missing or invalid")?;
    if essential_per_week == 0 {
        return Err("essential_trips_per_week must be positive".to_owned());
    }

    let policy = scenario.get("safety_policy").ok_or("safety_policy missing")?;
    let active_safety_hold = bool_at(policy, "active_safety_hold")
        .ok_or("active_safety_hold missing or invalid")?;
    let simulation_qualified = bool_at(policy, "configuration_qualified_for_simulation")
        .ok_or("configuration_qualified_for_simulation missing or invalid")?;
    if text_at(policy, "active_safety_hold_action") != Some("deny_dispatch")
        || text_at(policy, "unqualified_configuration_action") != Some("deny_dispatch")
    {
        return Err(format!("{scenario_id}: fail-closed dispatch policy declaration changed"));
    }

    let arch_fallback_fraction = number_at(architecture, "fallback_capacity_fraction")
        .ok_or("fallback_capacity_fraction missing or invalid")?;
    let fallback_eligible = bool_at(architecture, "fallback_mode_eligible_for_synthetic_test")
        .ok_or("fallback_mode_eligible_for_synthetic_test missing or invalid")?;
    if !arch_fallback_fraction.is_finite() || !(0.0..=1.0).contains(&arch_fallback_fraction) {
        return Err(format!("{scenario_id}: fallback capacity fraction must be within [0, 1]"));
    }
    let fallback_quota_per_week = if fallback_eligible {
        (essential_per_week as f64 * arch_fallback_fraction).floor() as u64
    } else {
        0
    };

    let stress = scenario.get("stress_parameters").ok_or("stress_parameters missing")?;
    let fuel_multiplier = number_at(stress, "fuel_cost_multiplier")
        .ok_or("fuel_cost_multiplier missing or invalid")?;
    let budget_retention = number_at(stress, "household_budget_retention_fraction")
        .ok_or("household_budget_retention_fraction missing or invalid")?;
    if !fuel_multiplier.is_finite() || fuel_multiplier <= 0.0
        || !budget_retention.is_finite() || !(0.0..=1.0).contains(&budget_retention)
    {
        return Err(format!("{scenario_id}: invalid fuel multiplier or budget retention"));
    }

    let horizon_hours = f64::from(model.horizon_days) * HOURS_PER_DAY;
    let demand_count = essential_per_week * u64::from(model.horizon_days) / 7;
    let mut repair_event_durations = Vec::new();
    let mut outage_intervals = Vec::new();

    let primary_gate_open = simulation_qualified && !active_safety_hold;
    if primary_gate_open {
        let repair_hours = effective_repair_hours(scenario, architecture, model)?;
        for day in &model.failure_event_days {
            let start = f64::from(*day) * HOURS_PER_DAY;
            let end = start + repair_hours;
            repair_event_durations.push(repair_hours);
            outage_intervals.push((start, end));
        }
    } else {
        // Fail closed: the primary vehicle is never dispatched for the whole
        // horizon. Independent fallback modes are still counted only when the
        // synthetic fixture explicitly marks them eligible.
        outage_intervals.push((0.0, horizon_hours));
    }
    let merged_outages = merge_intervals(outage_intervals, horizon_hours);
    let out_of_service_hours: f64 = merged_outages.iter().map(|(start, end)| end - start).sum();

    let weeks = usize::try_from(model.horizon_days / 7).map_err(|_| "invalid weeks")?;
    let mut budget_remaining = vec![model.weekly_essential_mobility_budget_r * budget_retention; weeks];
    let mut fallback_used = vec![0_u64; weeks];

    let mut primary_trips = 0_u64;
    let mut fallback_trips = 0_u64;
    let mut unmet_trips = 0_u64;
    let mut primary_spend = 0.0_f64;
    let mut fallback_spend = 0.0_f64;

    for trip_index in 0..demand_count {
        let trip_hour = (trip_index as f64 + 0.5) * horizon_hours / demand_count as f64;
        let week_index = ((trip_hour / (DAYS_PER_WEEK * HOURS_PER_DAY)).floor() as usize)
            .min(weeks.saturating_sub(1));
        let primary_available = primary_gate_open && !is_unavailable(trip_hour, &merged_outages);
        let primary_cost = model.primary_energy_cost_per_essential_trip_r * fuel_multiplier;

        if primary_available
            && budget_remaining[week_index] + 1.0e-9 >= primary_cost
        {
            budget_remaining[week_index] -= primary_cost;
            primary_trips += 1;
            primary_spend += primary_cost;
            continue;
        }

        let fallback_available = fallback_eligible
            && fallback_quota_per_week > 0
            && fallback_used[week_index] < fallback_quota_per_week
            && budget_remaining[week_index] + 1.0e-9 >= model.fallback_cost_per_essential_trip_r;

        if fallback_available {
            budget_remaining[week_index] -= model.fallback_cost_per_essential_trip_r;
            fallback_used[week_index] += 1;
            fallback_trips += 1;
            fallback_spend += model.fallback_cost_per_essential_trip_r;
        } else {
            unmet_trips += 1;
        }
    }

    let fulfilled = primary_trips + fallback_trips;
    let fulfilment_ratio = if demand_count > 0 {
        fulfilled as f64 / demand_count as f64
    } else {
        0.0
    };
    let variable_spend = primary_spend + fallback_spend;
    let spend_per_fulfilled_trip = if fulfilled > 0 {
        Some(variable_spend / fulfilled as f64)
    } else {
        None
    };
    let p50 = nearest_rank(&repair_event_durations, 0.50);
    let p90 = nearest_rank(&repair_event_durations, 0.90);
    let evidence_class = "synthetic_simulated_output";

    Ok(json!({
        "scenario_id": scenario_id,
        "architecture_id": architecture_id,
        "stressor_id": stressor_id,
        "evidence_class": evidence_class,
        "primary_dispatch_gate_open": primary_gate_open,
        "synthetic_fallback_gate_open": fallback_eligible,
        "synthetic_fallback_capacity_fraction": arch_fallback_fraction,
        "fallback_accessibility_fraction": Value::Null,
        "uncertainty_interval_coverage": Value::Null,
        "annual_essential_trip_demand": demand_count,
        "primary_trips_fulfilled": primary_trips,
        "fallback_trips_fulfilled": fallback_trips,
        "essential_trips_fulfilled": fulfilled,
        "unmet_essential_trips": unmet_trips,
        "essential_trip_fulfillment_ratio": fulfilment_ratio,
        "out_of_service_hours": out_of_service_hours,
        "repair_event_count": repair_event_durations.len(),
        "time_to_safe_recovery_p50_hours": p50,
        "time_to_safe_recovery_p90_hours": p90,
        "synthetic_primary_energy_spend_r": primary_spend,
        "synthetic_fallback_mobility_spend_r": fallback_spend,
        "synthetic_variable_mobility_spend_r": variable_spend,
        "synthetic_variable_spend_per_fulfilled_trip_r": spend_per_fulfilled_trip,
        "cost_per_fulfilled_essential_trip_r": Value::Null,
        "whole_life_cost_per_fulfilled_trip_r": Value::Null,
        "whole_life_cost_status": "not_evaluated_missing_capital_financing_insurance_maintenance_parts_and_depreciation_inputs",
        "quantile_interpretation": "descriptive quantiles of two fixed synthetic repair events; not a statistical confidence interval or population estimate"
    }))
}

fn build_report(
    schema_bytes: &[u8],
    corpus_bytes: &[u8],
    model_schema_bytes: &[u8],
    model_bytes: &[u8],
    corpus: &Value,
    model: &ModelConfig,
) -> Result<Value, String> {
    let architectures = corpus.get("architecture_profiles")
        .and_then(Value::as_array)
        .ok_or("architecture_profiles must be an array")?;
    let scenarios = corpus.get("scenarios")
        .and_then(Value::as_array)
        .ok_or("scenarios must be an array")?;
    let mut results = Vec::with_capacity(scenarios.len());

    for scenario in scenarios {
        let architecture_id = text_at(scenario, "architecture_id")
            .ok_or("scenario architecture_id missing")?;
        let architecture = find_by_id(architectures, architecture_id)
            .ok_or_else(|| format!("no architecture profile for {architecture_id}"))?;
        results.push(simulate_scenario(scenario, architecture, model)?);
    }

    let core = json!({
        "report_version": "mobility-continuity-simulation-report/0.1",
        "model_id": MODEL_ID,
        "evidence_class": "synthetic_simulated_output",
        "claim_ceiling": "deterministic proxy simulation under synthetic assumptions; no vehicle, workshop, household, road-safety, legal-compliance, or population claim",
        "input_digests": {
            "scenario_schema_sha256": sha256(schema_bytes),
            "scenario_corpus_sha256": sha256(corpus_bytes),
            "model_config_schema_sha256": sha256(model_schema_bytes),
            "model_config_sha256": sha256(model_bytes)
        },
        "scenario_count": scenarios.len(),
        "results": results,
        "excluded_costs": [
            "vehicle acquisition and depreciation",
            "financing and insurance",
            "maintenance and replacement parts",
            "tax, registration and licensing",
            "downtime consequences and lost income",
            "whole-life lifecycle cost"
        ],
        "limitations": [
            "Failure event times, trip demand, repair durations, budget and per-trip cost values are synthetic deterministic assumptions.",
            "The inventory effect is a simple arithmetic proxy, not a supplier, inventory, workshop queue or reliability model.",
            "Fallback capacity and eligibility are synthetic inputs and do not establish real-world legal availability, accessibility, capacity or service hours.",
            "The model omits vehicle physics, road conditions, safety validation, component interactions, weather, congestion, trip chaining and household heterogeneity.",
            "The descriptive P50/P90 values are computed from two fixed synthetic repair events and are not population estimates or confidence intervals."
        ]
    });
    let canonical = serde_json::to_vec(&core).map_err(|error| format!("cannot serialize report core: {error}"))?;
    let digest = sha256(&canonical);
    let mut report = core;
    report["report_digest_sha256"] = json!(digest);
    Ok(report)
}

fn read_json(path: &PathBuf) -> Result<(Vec<u8>, Value), String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("{} is not valid JSON: {error}", path.display()))?;
    Ok((bytes, value))
}

fn main() -> ExitCode {
    let mut scenario_schema_path = PathBuf::from("schemas/transport/mobility_continuity_scenario_v0_1.schema.json");
    let mut corpus_path = PathBuf::from("fixtures/personal_vehicle/mobility_continuity_scenarios_v0_1.json");
    let mut model_schema_path = PathBuf::from("schemas/transport/mobility_continuity_model_config_v0_1.schema.json");
    let mut model_config_path = PathBuf::from("fixtures/personal_vehicle/mobility_continuity_model_config_v0_1.json");
    let args: Vec<String> = env::args().skip(1).collect();
    let mut index = 0;
    while index < args.len() {
        let target = match args[index].as_str() {
            "--scenario-schema" => Some(&mut scenario_schema_path),
            "--corpus" => Some(&mut corpus_path),
            "--model-schema" => Some(&mut model_schema_path),
            "--model-config" => Some(&mut model_config_path),
            _ => None,
        };
        if let Some(path) = target {
            if index + 1 >= args.len() {
                eprintln!("missing path after {}", args[index]);
                return ExitCode::from(2);
            }
            *path = PathBuf::from(&args[index + 1]);
            index += 2;
        } else {
            eprintln!("unknown argument: {}\nusage: mobility-continuity-simulator [--scenario-schema PATH] [--corpus PATH] [--model-schema PATH] [--model-config PATH]", args[index]);
            return ExitCode::from(2);
        }
    }

    let (scenario_schema_bytes, scenario_schema) = match read_json(&scenario_schema_path) {
        Ok(value) => value,
        Err(error) => { eprintln!("{error}"); return ExitCode::FAILURE; }
    };
    let (corpus_bytes, corpus) = match read_json(&corpus_path) {
        Ok(value) => value,
        Err(error) => { eprintln!("{error}"); return ExitCode::FAILURE; }
    };
    let (model_schema_bytes, model_schema) = match read_json(&model_schema_path) {
        Ok(value) => value,
        Err(error) => { eprintln!("{error}"); return ExitCode::FAILURE; }
    };
    let (model_bytes, model_value) = match read_json(&model_config_path) {
        Ok(value) => value,
        Err(error) => { eprintln!("{error}"); return ExitCode::FAILURE; }
    };

    let scenario_validator = match schema_validator(
        &scenario_schema, "urn:luminous-dynamics:mobility:continuity:scenario:0.1"
    ) {
        Ok(value) => value,
        Err(error) => { eprintln!("{error}"); return ExitCode::FAILURE; }
    };
    let model_validator = match schema_validator(
        &model_schema, "urn:luminous-dynamics:mobility:continuity:model-config:0.1"
    ) {
        Ok(value) => value,
        Err(error) => { eprintln!("{error}"); return ExitCode::FAILURE; }
    };

    if text_at(&corpus, "schema_version") != Some(SCENARIO_SCHEMA_VERSION) {
        eprintln!("scenario corpus schema version mismatch");
        return ExitCode::FAILURE;
    }
    if let Err(errors) = validate_instance(&scenario_validator, &corpus, "scenario corpus") {
        for error in errors { eprintln!("- {error}"); }
        return ExitCode::FAILURE;
    }
    if let Err(errors) = validate_instance(&model_validator, &model_value, "model config") {
        for error in errors { eprintln!("- {error}"); }
        return ExitCode::FAILURE;
    }
    let model = match parse_model_config(&model_value) {
        Ok(value) => value,
        Err(errors) => { for error in errors { eprintln!("- {error}"); } return ExitCode::FAILURE; }
    };
    if text_at(&model_value, "evidence_class") != Some("synthetic_model_assumption") {
        eprintln!("model config must remain marked synthetic_model_assumption");
        return ExitCode::FAILURE;
    }

    match build_report(
        &scenario_schema_bytes, &corpus_bytes, &model_schema_bytes, &model_bytes, &corpus, &model
    ) {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(text) => { println!("{text}"); ExitCode::SUCCESS }
            Err(error) => { eprintln!("cannot serialize report: {error}"); ExitCode::FAILURE }
        },
        Err(error) => { eprintln!("simulation failed: {error}"); ExitCode::FAILURE }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frozen_inputs() -> (Value, Value, Value, ModelConfig) {
        let scenario_schema: Value = serde_json::from_str(include_str!(
            "../../../schemas/transport/mobility_continuity_scenario_v0_1.schema.json"
        )).expect("scenario schema");
        let model_schema: Value = serde_json::from_str(include_str!(
            "../../../schemas/transport/mobility_continuity_model_config_v0_1.schema.json"
        )).expect("model config schema");
        let corpus: Value = serde_json::from_str(include_str!(
            "../../../fixtures/personal_vehicle/mobility_continuity_scenarios_v0_1.json"
        )).expect("scenario corpus");
        let config_value: Value = serde_json::from_str(include_str!(
            "../../../fixtures/personal_vehicle/mobility_continuity_model_config_v0_1.json"
        )).expect("model config");
        let model = parse_model_config(&config_value).expect("model config values");
        (scenario_schema, model_schema, corpus, model)
    }

    fn result_for<'a>(report: &'a Value, architecture: &str, stressor: &str) -> &'a Value {
        report["results"].as_array().expect("results").iter()
            .find(|result| text_at(result, "architecture_id") == Some(architecture)
                && text_at(result, "stressor_id") == Some(stressor))
            .expect("scenario result exists")
    }

    fn report_fixture() -> Value {
        let (scenario_schema, model_schema, corpus, model) = frozen_inputs();
        let scenario_schema_bytes = serde_json::to_vec(&scenario_schema).expect("schema bytes");
        let corpus_bytes = serde_json::to_vec(&corpus).expect("corpus bytes");
        let model_schema_bytes = serde_json::to_vec(&model_schema).expect("model schema bytes");
        let model_bytes: Value = serde_json::from_str(include_str!(
            "../../../fixtures/personal_vehicle/mobility_continuity_model_config_v0_1.json"
        )).expect("model config again");
        let model_bytes = serde_json::to_vec(&model_bytes).expect("model bytes");
        build_report(
            &scenario_schema_bytes, &corpus_bytes, &model_schema_bytes, &model_bytes, &corpus, &model
        ).expect("report builds")
    }

    #[test]
    fn scenario_and_model_config_fixtures_validate_against_draft_2020_12_schemas() {
        let (scenario_schema, model_schema, corpus, model) = frozen_inputs();
        let config_value: Value = serde_json::from_str(include_str!(
            "../../../fixtures/personal_vehicle/mobility_continuity_model_config_v0_1.json"
        )).expect("config fixture");
        let scenario_validator = schema_validator(
            &scenario_schema,
            "urn:luminous-dynamics:mobility:continuity:scenario:0.1"
        ).expect("scenario schema compiles");
        let model_validator = schema_validator(
            &model_schema,
            "urn:luminous-dynamics:mobility:continuity:model-config:0.1"
        ).expect("model schema compiles");
        validate_instance(&scenario_validator, &corpus, "scenario corpus")
            .expect("scenario fixture validates");
        validate_instance(&model_validator, &config_value, "model config")
            .expect("model config validates");
        assert_eq!(model.horizon_days, 364);
    }

    #[test]
    fn produces_one_output_per_frozen_scenario() {
        let report = report_fixture();
        assert_eq!(report["scenario_count"].as_u64(), Some(24));
        assert_eq!(report["results"].as_array().expect("results").len(), 24);
        assert_eq!(report["evidence_class"], "synthetic_simulated_output");
    }

    #[test]
    fn repeated_exact_inputs_produce_identical_report_digest() {
        let first = report_fixture();
        let second = report_fixture();
        assert_eq!(first["report_digest_sha256"], second["report_digest_sha256"]);
        assert_eq!(first, second);
    }

    #[test]
    fn lifecycle_cost_stays_unknown() {
        let report = report_fixture();
        for result in report["results"].as_array().expect("results") {
            assert!(result["cost_per_fulfilled_essential_trip_r"].is_null());
            assert!(result["whole_life_cost_per_fulfilled_trip_r"].is_null());
            assert!(result["fallback_accessibility_fraction"].is_null());
            assert!(result["uncertainty_interval_coverage"].is_null());
            assert_eq!(
                result["whole_life_cost_status"],
                "not_evaluated_missing_capital_financing_insurance_maintenance_parts_and_depreciation_inputs"
            );
        }
    }

    #[test]
    fn combined_shock_is_more_severe_than_nominal_for_baseline() {
        let report = report_fixture();
        let nominal = result_for(&report, "single_vehicle_baseline", "nominal");
        let combined = result_for(
            &report,
            "single_vehicle_baseline",
            "combined_supply_fuel_cashflow_shock"
        );
        assert!(
            combined["essential_trips_fulfilled"].as_u64().unwrap()
                < nominal["essential_trips_fulfilled"].as_u64().unwrap()
        );
        assert!(
            combined["time_to_safe_recovery_p90_hours"].as_f64().unwrap()
                > nominal["time_to_safe_recovery_p90_hours"].as_f64().unwrap()
        );
    }

    #[test]
    fn fuel_price_shock_reduces_baseline_fulfilment() {
        let report = report_fixture();
        let nominal = result_for(&report, "single_vehicle_baseline", "nominal");
        let shocked = result_for(&report, "single_vehicle_baseline", "fuel_price_shock");
        assert!(
            shocked["essential_trips_fulfilled"].as_u64().unwrap()
                < nominal["essential_trips_fulfilled"].as_u64().unwrap()
        );
    }

    #[test]
    fn inventory_proxy_shortens_wait_but_does_not_erase_it() {
        let report = report_fixture();
        let baseline = result_for(&report, "single_vehicle_baseline", "critical_part_unavailability");
        let stocked = result_for(&report, "single_vehicle_repairability_oriented", "critical_part_unavailability");
        let base_p90 = baseline["time_to_safe_recovery_p90_hours"].as_f64().unwrap();
        let stocked_p90 = stocked["time_to_safe_recovery_p90_hours"].as_f64().unwrap();
        assert!(stocked_p90 < base_p90);
        assert!(stocked_p90 > 0.0);
    }

    #[test]
    fn synthetic_fallback_capacity_improves_part_shortage_case() {
        let report = report_fixture();
        let baseline = result_for(&report, "single_vehicle_baseline", "critical_part_unavailability");
        let bundle = result_for(&report, "household_mobility_bundle", "critical_part_unavailability");
        assert_eq!(baseline["fallback_trips_fulfilled"].as_u64(), Some(0));
        assert!(
            bundle["essential_trips_fulfilled"].as_u64().unwrap()
                > baseline["essential_trips_fulfilled"].as_u64().unwrap()
        );
    }

    #[test]
    fn safety_hold_never_dispatches_primary_vehicle() {
        let (_, _, mut corpus, model) = frozen_inputs();
        {
            let scenario = corpus["scenarios"].as_array_mut().expect("array").iter_mut()
                .find(|item| text_at(item, "architecture_id") == Some("single_vehicle_baseline")
                    && text_at(item, "stressor_id") == Some("nominal"))
                .expect("baseline nominal");
            scenario["safety_policy"]["active_safety_hold"] = json!(true);
        }
        let scenario = corpus["scenarios"].as_array().expect("array").iter()
            .find(|item| text_at(item, "architecture_id") == Some("single_vehicle_baseline")
                && text_at(item, "stressor_id") == Some("nominal"))
            .expect("modified baseline nominal")
            .clone();
        let architecture = find_by_id(
            corpus["architecture_profiles"].as_array().expect("array"),
            "single_vehicle_baseline"
        ).expect("architecture");
        let result = simulate_scenario(&scenario, architecture, &model).expect("scenario simulation");
        assert_eq!(result["primary_dispatch_gate_open"], false);
        assert_eq!(result["primary_trips_fulfilled"].as_u64(), Some(0));
        assert_eq!(result["out_of_service_hours"].as_f64(), Some(364.0 * 24.0));
    }

    #[test]
    fn unqualified_configuration_never_dispatches_primary_vehicle() {
        let (_, _, mut corpus, model) = frozen_inputs();
        {
            let scenario = corpus["scenarios"].as_array_mut().expect("array").iter_mut()
                .find(|item| text_at(item, "architecture_id") == Some("single_vehicle_baseline")
                    && text_at(item, "stressor_id") == Some("nominal"))
                .expect("baseline nominal");
            scenario["safety_policy"]["configuration_qualified_for_simulation"] = json!(false);
        }
        let scenario = corpus["scenarios"].as_array().expect("array").iter()
            .find(|item| text_at(item, "architecture_id") == Some("single_vehicle_baseline")
                && text_at(item, "stressor_id") == Some("nominal"))
            .expect("modified baseline nominal")
            .clone();
        let architecture = find_by_id(
            corpus["architecture_profiles"].as_array().expect("array"),
            "single_vehicle_baseline"
        ).expect("architecture");
        let result = simulate_scenario(&scenario, architecture, &model).expect("scenario simulation");
        assert_eq!(result["primary_trips_fulfilled"].as_u64(), Some(0));
        assert_eq!(result["out_of_service_hours"].as_f64(), Some(364.0 * 24.0));
    }

    #[test]
    fn nonqualified_fallback_is_not_counted_as_served() {
        let (_, _, mut corpus, model) = frozen_inputs();
        let scenario = corpus["scenarios"].as_array().expect("array").iter()
            .find(|item| text_at(item, "architecture_id") == Some("household_mobility_bundle")
                && text_at(item, "stressor_id") == Some("critical_part_unavailability"))
            .expect("bundle part shortage")
            .clone();
        {
            let architecture = corpus["architecture_profiles"].as_array_mut().expect("array").iter_mut()
                .find(|item| text_at(item, "id") == Some("household_mobility_bundle"))
                .expect("bundle");
            architecture["fallback_mode_eligible_for_synthetic_test"] = json!(false);
        }
        let architecture = find_by_id(
            corpus["architecture_profiles"].as_array().expect("array"),
            "household_mobility_bundle"
        ).expect("modified bundle");
        let result = simulate_scenario(&scenario, architecture, &model).expect("scenario simulation");
        assert_eq!(result["fallback_trips_fulfilled"].as_u64(), Some(0));
    }

    #[test]
    fn invalid_budget_or_fuel_parameters_fail_closed() {
        let (_, _, mut corpus, model) = frozen_inputs();
        {
            let scenario = corpus["scenarios"].as_array_mut().expect("array").iter_mut()
                .find(|item| text_at(item, "architecture_id") == Some("single_vehicle_baseline")
                    && text_at(item, "stressor_id") == Some("nominal"))
                .expect("baseline nominal");
            scenario["stress_parameters"]["fuel_cost_multiplier"] = json!(0.0);
        }
        let scenario = corpus["scenarios"].as_array().expect("array").iter()
            .find(|item| text_at(item, "architecture_id") == Some("single_vehicle_baseline")
                && text_at(item, "stressor_id") == Some("nominal"))
            .expect("modified baseline nominal")
            .clone();
        let architecture = find_by_id(
            corpus["architecture_profiles"].as_array().expect("array"),
            "single_vehicle_baseline"
        ).expect("architecture");
        assert!(simulate_scenario(&scenario, architecture, &model).is_err());
    }
}
