// SPDX-License-Identifier: AGPL-3.0-or-later
//! Synthetic-first diagnosis-to-verified-intervention utility evaluator.
//! Read-only: no dispatch, SCADA write, repair authorization or actuation.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::Path;

const INPUT_VERSION: &str = "water-verified-intervention-utility-t1-input-v0.1";
const ORACLE_VERSION: &str = "water-verified-intervention-utility-t1-oracle-v0.1";
const BENCHMARK_ID: &str = "WATER-VERIFIED-INTERVENTION-UTILITY-001";
const SCENARIO_COUNT: usize = 12;

fn field<'a>(value: &'a Value, key: &str, context: &str) -> Result<&'a Value, String> {
    value.get(key).ok_or_else(|| format!("{context}: missing '{key}'"))
}

fn string<'a>(value: &'a Value, key: &str, context: &str) -> Result<&'a str, String> {
    field(value, key, context)?
        .as_str()
        .ok_or_else(|| format!("{context}: '{key}' must be a string"))
}

fn number(value: &Value, key: &str, context: &str) -> Result<f64, String> {
    let result = field(value, key, context)?
        .as_f64()
        .ok_or_else(|| format!("{context}: '{key}' must be numeric"))?;
    if !result.is_finite() {
        return Err(format!("{context}: '{key}' must be finite"));
    }
    Ok(result)
}

fn boolean(value: &Value, key: &str, context: &str) -> Result<bool, String> {
    field(value, key, context)?
        .as_bool()
        .ok_or_else(|| format!("{context}: '{key}' must be boolean"))
}

fn strings(value: &Value, context: &str) -> Result<Vec<String>, String> {
    let items = value
        .as_array()
        .ok_or_else(|| format!("{context}: must be an array"))?;
    let mut output = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let item = item
            .as_str()
            .ok_or_else(|| format!("{context}[{index}]: must be a string"))?;
        if item.trim().is_empty() {
            return Err(format!("{context}[{index}]: empty identity"));
        }
        if output.iter().any(|existing| existing == item) {
            return Err(format!("{context}: duplicate identity '{item}'"));
        }
        output.push(item.to_owned());
    }
    Ok(output)
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn read_json(path: &str) -> Result<(Vec<u8>, Value), String> {
    let bytes = fs::read(path).map_err(|error| format!("cannot read '{path}': {error}"))?;
    let value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid JSON '{path}': {error}"))?;
    Ok((bytes, value))
}

fn ensure_keys(value: &Value, expected: &[&str], context: &str) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{context}: must be an object"))?;
    let actual: std::collections::BTreeSet<&str> =
        object.keys().map(String::as_str).collect();
    let wanted: std::collections::BTreeSet<&str> = expected.iter().copied().collect();
    if actual != wanted {
        return Err(format!("{context}: key set differs from frozen v0.1 contract"));
    }
    Ok(())
}

fn array<'a>(value: &'a Value, key: &str, context: &str) -> Result<&'a Vec<Value>, String> {
    field(value, key, context)?
        .as_array()
        .ok_or_else(|| format!("{context}: '{key}' must be an array"))
}

fn optional_number(value: &Value, key: &str, context: &str) -> Result<Option<f64>, String> {
    let raw = field(value, key, context)?;
    if raw.is_null() {
        return Ok(None);
    }
    let value = raw
        .as_f64()
        .ok_or_else(|| format!("{context}: '{key}' must be numeric or null"))?;
    if !value.is_finite() {
        return Err(format!("{context}: '{key}' must be finite"));
    }
    Ok(Some(value))
}

#[derive(Clone)]
struct Measurement {
    leak_flow: f64,
    uncertainty: f64,
    demand: f64,
    pressure: f64,
    pump_state: String,
    valve_state: String,
    topology_revision: String,
    instrument_id: String,
    calibration_ref: String,
    method_ref: String,
    raw_digest: String,
    start: u64,
    end: u64,
}

fn parse_measurement(value: &Value, context: &str) -> Result<Measurement, String> {
    ensure_keys(
        value,
        &[
            "leak_flow_m3_day",
            "standard_uncertainty_m3_day",
            "demand_m3_day",
            "pressure_kpa",
            "pump_state",
            "valve_state",
            "topology_revision",
            "instrument_id",
            "calibration_ref",
            "method_ref",
            "raw_record_digest",
            "validity",
            "window_start_minute",
            "window_end_minute",
        ],
        context,
    )?;
    let leak_flow = number(value, "leak_flow_m3_day", context)?;
    let uncertainty = number(value, "standard_uncertainty_m3_day", context)?;
    let demand = number(value, "demand_m3_day", context)?;
    let pressure = number(value, "pressure_kpa", context)?;
    let start = field(value, "window_start_minute", context)?
        .as_u64()
        .ok_or_else(|| format!("{context}: window_start_minute must be a nonnegative integer"))?;
    let end = field(value, "window_end_minute", context)?
        .as_u64()
        .ok_or_else(|| format!("{context}: window_end_minute must be a nonnegative integer"))?;
    let validity = string(value, "validity", context)?;
    let raw_digest = string(value, "raw_record_digest", context)?.to_owned();
    let measurement = Measurement {
        leak_flow,
        uncertainty,
        demand,
        pressure,
        pump_state: string(value, "pump_state", context)?.to_owned(),
        valve_state: string(value, "valve_state", context)?.to_owned(),
        topology_revision: string(value, "topology_revision", context)?.to_owned(),
        instrument_id: string(value, "instrument_id", context)?.to_owned(),
        calibration_ref: string(value, "calibration_ref", context)?.to_owned(),
        method_ref: string(value, "method_ref", context)?.to_owned(),
        raw_digest,
        start,
        end,
    };
    if leak_flow < 0.0 || uncertainty < 0.0 || demand < 0.0 || pressure < 0.0 {
        return Err(format!("{context}: negative measurement value or uncertainty"));
    }
    if validity != "valid" {
        return Err(format!("{context}: validity '{validity}' is not 'valid'"));
    }
    if !valid_digest(&measurement.raw_digest) {
        return Err(format!("{context}: raw_record_digest must be 64 hex characters"));
    }
    if measurement.instrument_id.trim().is_empty()
        || measurement.calibration_ref.trim().is_empty()
        || measurement.method_ref.trim().is_empty()
        || measurement.topology_revision.trim().is_empty()
        || measurement.pump_state.trim().is_empty()
        || measurement.valve_state.trim().is_empty()
    {
        return Err(format!("{context}: measurement provenance identity is empty"));
    }
    if start >= end {
        return Err(format!("{context}: measurement window is empty or reversed"));
    }
    Ok(measurement)
}

fn empty_followup(day: u64, status: &str) -> Value {
    json!({
        "day": day,
        "status": status,
        "point_reduction_m3_day": null,
        "lower_reduction_m3_day": null
    })
}

fn verify_followups(
    scenario: &Value,
    repair_consistent: bool,
    input: &Value,
) -> Result<Vec<Value>, String> {
    let context = string(scenario, "scenario_id", "scenario")?;
    let verification = field(scenario, "verification", context)?;
    let followups = array(verification, "followups", context)?;
    let baseline_value = field(verification, "baseline", context)?;
    let baseline = if baseline_value.is_null() {
        None
    } else {
        Some(parse_measurement(baseline_value, &format!("{context}.baseline"))?)
    };
    let repair = field(scenario, "repair", context)?;
    let repair_minute = if repair_consistent {
        Some(
            field(repair, "repair_minute", context)?
                .as_u64()
                .ok_or_else(|| format!("{context}: repaired run requires repair_minute"))?,
        )
    } else {
        None
    };
    let mut by_day = std::collections::BTreeMap::<u64, &Value>::new();
    let mut post_digest_days = std::collections::BTreeMap::<String, Vec<u64>>::new();
    for followup in followups {
        let day = field(followup, "day", context)?
            .as_u64()
            .ok_or_else(|| format!("{context}: follow-up day must be integer"))?;
        if ![7, 30, 90].contains(&day) || by_day.insert(day, followup).is_some() {
            return Err(format!("{context}: duplicate or unsupported follow-up day"));
        }
        if let Some(digest) = followup
            .get("measurement")
            .and_then(|measurement| measurement.get("raw_record_digest"))
            .and_then(Value::as_str)
        {
            post_digest_days
                .entry(digest.to_owned())
                .or_default()
                .push(day);
        }
    }

    let demand_tolerance = number(input, "maximum_demand_difference_percent", "input")?;
    let pressure_tolerance = number(input, "maximum_pressure_difference_percent", "input")?;
    let expansion = number(input, "uncertainty_expansion_factor", "input")?;
    let mut output = Vec::with_capacity(3);

    for day in [7_u64, 30, 90] {
        if !repair_consistent {
            output.push(empty_followup(day, "not_applicable"));
            continue;
        }
        let Some(pre) = baseline.as_ref() else {
            output.push(empty_followup(day, "not_observed"));
            continue;
        };
        let Some(followup) = by_day.get(&day) else {
            output.push(empty_followup(day, "not_observed"));
            continue;
        };
        let post_value = field(followup, "measurement", context)?;
        let post = match parse_measurement(post_value, &format!("{context}.day{day}")) {
            Ok(value) => value,
            Err(_) => {
                output.push(empty_followup(day, "invalid_measurement"));
                continue;
            }
        };
        if post_digest_days
            .get(&post.raw_digest)
            .is_some_and(|days| days.len() > 1)
        {
            output.push(empty_followup(day, "indeterminate_evidence_scope"));
            continue;
        }
        if repair_minute.is_some_and(|minute| pre.end > minute || post.start < minute) {
            output.push(empty_followup(day, "indeterminate_evidence_scope"));
            continue;
        }
        if pre.instrument_id != post.instrument_id
            || pre.calibration_ref != post.calibration_ref
            || pre.method_ref != post.method_ref
            || pre.topology_revision != post.topology_revision
            || pre.pump_state != post.pump_state
            || pre.valve_state != post.valve_state
            || pre.raw_digest == post.raw_digest
        {
            output.push(empty_followup(day, "indeterminate_evidence_scope"));
            continue;
        }
        let demand_delta =
            (pre.demand - post.demand).abs() / pre.demand.max(post.demand).max(1.0) * 100.0;
        let pressure_delta =
            (pre.pressure - post.pressure).abs() / pre.pressure.max(post.pressure).max(1.0)
                * 100.0;
        if demand_delta > demand_tolerance
            || pressure_delta > pressure_tolerance
            || !(pre.end <= post.start || post.end <= pre.start)
        {
            output.push(empty_followup(day, "indeterminate_operating_conditions"));
            continue;
        }
        let point = pre.leak_flow - post.leak_flow;
        let lower = (point - expansion * (pre.uncertainty + post.uncertainty)).max(0.0);
        let status = if lower > 0.0 {
            "verified_positive_lower_bound"
        } else {
            "no_positive_lower_bound"
        };
        output.push(json!({
            "day": day,
            "status": status,
            "point_reduction_m3_day": point,
            "lower_reduction_m3_day": lower
        }));
    }
    Ok(output)
}

fn quantile_nearest_rank(values: &mut [f64], probability: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let rank = (probability * values.len() as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(values.len() - 1);
    Some(values[index])
}

fn evaluate_scenario(scenario: &Value, input: &Value) -> Result<Value, String> {
    let id = string(scenario, "scenario_id", "scenario")?;
    let truth = string(scenario, "event_truth", id)?;
    if !["leak", "non_leak"].contains(&truth) {
        return Err(format!("{id}: unsupported event_truth '{truth}'"));
    }
    let event_start = field(scenario, "event_start_minute", id)?
        .as_i64()
        .ok_or_else(|| format!("{id}: event_start_minute must be integer"))?;
    let truth_assets = strings(field(scenario, "ground_truth_asset_ids", id)?, &format!("{id}.ground_truth_asset_ids"))?;

    let observability = field(scenario, "observability", id)?;
    let observation_disposition = string(observability, "disposition", id)?;
    let topology_valid = boolean(observability, "topology_valid", id)?;
    let estimator_scope_valid = boolean(observability, "estimator_scope_valid", id)?;
    let directly_or_scoped = observation_disposition == "directly_observed_current"
        || (observation_disposition == "estimator_qualified_for_declared_scope"
            && estimator_scope_valid);
    let state_eligible = directly_or_scoped && topology_valid;

    let diagnostic = field(scenario, "diagnostic", id)?;
    let detected = boolean(diagnostic, "detected", id)?;
    let detected_minute = optional_number(diagnostic, "detected_minute", id)?;
    let localization_level = string(diagnostic, "localization_level", id)?;
    let candidates = strings(
        field(diagnostic, "candidate_asset_ids", id)?,
        &format!("{id}.candidate_asset_ids"),
    )?;
    let action = string(diagnostic, "action", id)?;
    if !["inspect", "prioritize", "abstain"].contains(&action) {
        return Err(format!("{id}: invalid diagnostic action"));
    }
    let pipe_output = localization_level == "pipe" || localization_level == "within_pipe";
    let pipe_claim_issued = detected && action != "abstain" && pipe_output;
    let pipe_eligible = pipe_claim_issued && state_eligible && candidates.len() == 1;
    let false_safe_pipe_claim = pipe_claim_issued && !pipe_eligible;
    let localization_correct = detected
        && pipe_eligible
        && truth_assets.len() == 1
        && candidates[0] == truth_assets[0];

    let abstained = action == "abstain";
    let true_positive = detected && truth == "leak" && !abstained;
    let false_positive = detected && truth == "non_leak" && !abstained;
    let false_negative = truth == "leak" && (!detected || abstained);

    let inspection = field(scenario, "inspection", id)?;
    let inspection_status = string(inspection, "status", id)?;
    let confirmed_assets = strings(
        field(inspection, "confirmed_asset_ids", id)?,
        &format!("{id}.confirmed_asset_ids"),
    )?;
    let inspection_confirmed = inspection_status == "confirmed_leak"
        && truth == "leak"
        && confirmed_assets.iter().any(|asset| truth_assets.contains(asset));
    let false_dispatch = inspection_status == "no_leak" && truth == "non_leak";
    let crew_hours = number(inspection, "crew_hours", id)?
        + number(inspection, "travel_hours", id)?;
    let inspection_cost = number(inspection, "inspection_cost", id)?;
    let inspection_cost = inspection_cost.max(0.0);
    let confirmation_minute = optional_number(inspection, "confirmed_minute", id)?;
    let detection_delay = detected_minute.map(|minute| minute - event_start as f64);
    let confirmation_delay = if inspection_confirmed {
        confirmation_minute.map(|minute| minute - event_start as f64)
    } else {
        None
    };

    let repair = field(scenario, "repair", id)?;
    let repair_status = string(repair, "status", id)?;
    let repair_asset = field(repair, "asset_id", id)?;
    let repair_asset = repair_asset.as_str();
    let repair_consistent = inspection_confirmed
        && repair_status == "repaired"
        && repair_asset.is_some_and(|asset| confirmed_assets.iter().any(|confirmed| confirmed == asset));
    let repair_cost = number(repair, "repair_cost", id)?;
    let verification = verify_followups(scenario, repair_consistent, input)?;
    let reduction = |day: u64, field: &str| -> Option<f64> {
        verification
            .iter()
            .find(|item| item["day"].as_u64() == Some(day))
            .and_then(|item| item[field].as_f64())
    };

    Ok(json!({
        "scenario_id": id,
        "true_positive": true_positive,
        "false_positive": false_positive,
        "false_negative": false_negative,
        "abstained": abstained,
        "pipe_claim_eligible": pipe_eligible,
        "false_safe_pipe_claim": false_safe_pipe_claim,
        "localization_correct": localization_correct,
        "inspection_confirmed": inspection_confirmed,
        "false_dispatch": false_dispatch,
        "crew_hours": crew_hours,
        "inspection_cost": inspection_cost,
        "repair_cost": repair_cost,
        "detection_delay_min": detection_delay,
        "confirmation_delay_min": confirmation_delay,
        "repair_consistent": repair_consistent,
        "repair_verification": verification,
        "verified_point_reduction_7d_m3_day": reduction(7, "point_reduction_m3_day"),
        "verified_point_reduction_30d_m3_day": reduction(30, "point_reduction_m3_day"),
        "verified_point_reduction_90d_m3_day": reduction(90, "point_reduction_m3_day"),
        "verified_lower_reduction_7d_m3_day": reduction(7, "lower_reduction_m3_day"),
        "verified_lower_reduction_30d_m3_day": reduction(30, "lower_reduction_m3_day"),
        "verified_lower_reduction_90d_m3_day": reduction(90, "lower_reduction_m3_day")
    }))
}

fn evaluate_all(input: &Value, input_schema: &Value, oracle: &Value) -> Result<Value, String> {
    if string(input, "$schema", "input")? != "https://json-schema.org/draft/2020-12/schema"
        || string(input_schema, "$schema", "input schema")?
            != "https://json-schema.org/draft/2020-12/schema"
    {
        return Err("input and input schema must declare Draft 2020-12".to_owned());
    }
    if string(input, "schema_version", "input")? != INPUT_VERSION
        || string(input, "benchmark_id", "input")? != BENCHMARK_ID
        || string(input, "evidence_class", "input")? != "synthetic_only"
    {
        return Err("input identity/evidence class mismatch".to_owned());
    }
    let schema_properties = field(input_schema, "properties", "input schema")?;
    if string(
        field(schema_properties, "schema_version", "input schema properties")?,
        "const",
        "input schema properties.schema_version",
    )? != INPUT_VERSION
    {
        return Err("input schema version constant mismatch".to_owned());
    }
    let scenarios = array(input, "scenarios", "input")?;
    if scenarios.len() != SCENARIO_COUNT {
        return Err(format!("exactly {SCENARIO_COUNT} input scenarios are required"));
    }
    ensure_keys(
        oracle,
        &[
            "$schema",
            "schema_version",
            "benchmark_id",
            "submission_id",
            "arm_id",
            "aggregate",
            "scenarios",
            "physical_qualification_status",
            "statistical_inference_status",
            "claim_eligibility",
        ],
        "oracle",
    )?;
    if string(oracle, "$schema", "oracle")? != "https://json-schema.org/draft/2020-12/schema"
        || string(oracle, "schema_version", "oracle")? != ORACLE_VERSION
        || string(oracle, "benchmark_id", "oracle")? != BENCHMARK_ID
        || string(oracle, "submission_id", "oracle")?
            != string(input, "submission_id", "input")?
        || string(oracle, "arm_id", "oracle")? != string(input, "arm_id", "input")?
    {
        return Err("oracle identity does not match input".to_owned());
    }
    if string(oracle, "physical_qualification_status", "oracle")? != "NotRun"
        || string(oracle, "statistical_inference_status", "oracle")? != "NotRun"
        || string(oracle, "claim_eligibility", "oracle")? != "NotEligible"
    {
        return Err("oracle claim ceiling must remain NotRun/NotEligible".to_owned());
    }
    let expected_scenarios = array(oracle, "scenarios", "oracle")?;
    if expected_scenarios.len() != SCENARIO_COUNT {
        return Err(format!("exactly {SCENARIO_COUNT} oracle scenarios are required"));
    }
    let mut expected_by_id = std::collections::BTreeMap::new();
    for expected in expected_scenarios {
        let id = string(expected, "scenario_id", "oracle scenario")?.to_owned();
        if expected_by_id.insert(id.clone(), expected).is_some() {
            return Err(format!("duplicate oracle scenario_id '{id}'"));
        }
    }
    let mut input_ids = std::collections::BTreeSet::new();
    let mut results = Vec::with_capacity(SCENARIO_COUNT);
    let mut mismatches = Vec::new();
    for scenario in scenarios {
        let id = string(scenario, "scenario_id", "scenario")?.to_owned();
        if !input_ids.insert(id.clone()) {
            return Err(format!("duplicate input scenario_id '{id}'"));
        }
        let actual = evaluate_scenario(scenario, input)?;
        let expected = expected_by_id
            .get(&id)
            .ok_or_else(|| format!("input scenario '{id}' absent from oracle"))?;
        if &actual != *expected {
            mismatches.push(json!({
                "scenario_id": id,
                "expected": expected,
                "actual": actual.clone()
            }));
        }
        results.push(actual);
    }
    if input_ids != expected_by_id.keys().cloned().collect() {
        return Err("input/oracle scenario ID sets differ".to_owned());
    }

    let count = |predicate: fn(&Value) -> bool| -> usize {
        results.iter().filter(|scenario| predicate(scenario)).count()
    };
    let sum = |key: &str| -> f64 {
        results
            .iter()
            .map(|scenario| scenario[key].as_f64().unwrap_or(0.0))
            .sum()
    };
    let mut detection_delays: Vec<f64> = results
        .iter()
        .filter_map(|scenario| scenario["detection_delay_min"].as_f64())
        .collect();
    let mut confirmation_delays: Vec<f64> = results
        .iter()
        .filter_map(|scenario| scenario["confirmation_delay_min"].as_f64())
        .collect();
    let crew_hours = sum("crew_hours");
    let confirmed_count = count(|scenario| scenario["inspection_confirmed"] == true);
    let aggregate = json!({
        "scenario_count": results.len(),
        "leak_event_count": scenarios
            .iter()
            .filter(|scenario| scenario["event_truth"] == "leak")
            .count(),
        "true_positive_count": count(|scenario| scenario["true_positive"] == true),
        "false_positive_count": count(|scenario| scenario["false_positive"] == true),
        "false_negative_count": count(|scenario| scenario["false_negative"] == true),
        "abstention_count": count(|scenario| scenario["abstained"] == true),
        "false_safe_pipe_claim_count": count(|scenario| scenario["false_safe_pipe_claim"] == true),
        "localization_correct_count": count(|scenario| scenario["localization_correct"] == true),
        "inspection_confirmed_leak_count": confirmed_count,
        "false_dispatch_count": count(|scenario| scenario["false_dispatch"] == true),
        "crew_hours": crew_hours,
        "confirmed_leaks_per_crew_day": if crew_hours > 0.0 {
            Some(confirmed_count as f64 / (crew_hours / 8.0))
        } else {
            None::<f64>
        },
        "median_detection_delay_min": quantile_nearest_rank(&mut detection_delays, 0.5),
        "p95_detection_delay_min": quantile_nearest_rank(&mut detection_delays, 0.95),
        "median_confirmation_delay_min": quantile_nearest_rank(&mut confirmation_delays, 0.5),
        "p95_confirmation_delay_min": quantile_nearest_rank(&mut confirmation_delays, 0.95),
        "verified_lower_reduction_7d_m3_day": sum("verified_lower_reduction_7d_m3_day"),
        "verified_lower_reduction_30d_m3_day": sum("verified_lower_reduction_30d_m3_day"),
        "verified_lower_reduction_90d_m3_day": sum("verified_lower_reduction_90d_m3_day")
    });
    let mismatch_count = mismatches.len();
    Ok(json!({
        "schema_version": "water-verified-intervention-utility-t1-evaluation-v0.1",
        "benchmark_id": BENCHMARK_ID,
        "submission_id": string(input, "submission_id", "input")?,
        "arm_id": string(input, "arm_id", "input")?,
        "scenario_count": SCENARIO_COUNT,
        "matched_scenario_count": SCENARIO_COUNT - mismatch_count,
        "mismatch_count": mismatch_count,
        "mismatches": mismatches,
        "aggregate": aggregate,
        "scenarios": results,
        "physical_qualification_status": "NotRun",
        "statistical_inference_status": "NotRun",
        "claim_eligibility": "NotEligible",
        "claim_ceiling": "synthetic_diagnosis_to_verified_intervention_contract_only"
    }))
}

fn valid_revision(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn run(args: &[String]) -> Result<(), String> {
    if args.len() != 4 {
        return Err(
            "usage: water-verified-intervention-utility-t1 INPUT_JSON INPUT_SCHEMA ORACLE_JSON RECEIPT_JSON"
                .to_owned(),
        );
    }
    let (input_bytes, input) = read_json(&args[0])?;
    let (schema_bytes, input_schema) = read_json(&args[1])?;
    let (oracle_bytes, oracle) = read_json(&args[2])?;
    let result = evaluate_all(&input, &input_schema, &oracle)?;
    let success = result["scenario_count"].as_u64() == Some(SCENARIO_COUNT as u64)
        && result["matched_scenario_count"].as_u64() == Some(SCENARIO_COUNT as u64)
        && result["mismatch_count"].as_u64() == Some(0);

    let revision = env::var("WATER_VERIFIED_INTERVENTION_UTILITY_T1_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok()
        .filter(|value| valid_revision(value));
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let lock_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let manifest_bytes = fs::read(&manifest_path)
        .map_err(|error| format!("read {}: {error}", manifest_path.display()))?;
    let lock_bytes = fs::read(&lock_path)
        .map_err(|error| format!("read {}: {error}", lock_path.display()))?;
    let implementation = include_bytes!("water-verified-intervention-utility-t1.rs");
    let receipt = json!({
        "receipt_schema_version": "water-verified-intervention-utility-t1-receipt-v0.1",
        "benchmark_id": BENCHMARK_ID,
        "verification_state": if success && revision.is_some() {
            "PassedExactRevision"
        } else if success {
            "PassedLocalUnpinned"
        } else {
            "OracleMismatch"
        },
        "physical_qualification_status": "NotRun",
        "statistical_inference_status": "NotRun",
        "claim_eligibility": "NotEligible",
        "implementation_revision": revision,
        "input_sha256": sha256(&input_bytes),
        "input_schema_sha256": sha256(&schema_bytes),
        "oracle_sha256": sha256(&oracle_bytes),
        "package_manifest_sha256": sha256(&manifest_bytes),
        "cargo_lock_sha256": sha256(&lock_bytes),
        "implementation_sha256": sha256(implementation),
        "scenario_count": result["scenario_count"],
        "matched_scenario_count": result["matched_scenario_count"],
        "mismatch_count": result["mismatch_count"],
        "mismatches": result["mismatches"],
        "aggregate": result["aggregate"],
        "claim_ceiling": "synthetic_diagnosis_to_verified_intervention_contract_only"
    });
    let output = Path::new(&args[3]);
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create receipt directory: {error}"))?;
    }
    fs::write(
        output,
        serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("serialize receipt: {error}"))?,
    )
    .map_err(|error| format!("write receipt '{}': {error}", output.display()))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt).unwrap_or_default()
    );
    if success {
        Ok(())
    } else {
        Err("verified-intervention evaluator did not match frozen oracle".to_owned())
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Err(error) = run(&args) {
        eprintln!("water-verified-intervention-utility-t1: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Value, Value, Value) {
        let input = serde_json::from_slice(include_bytes!(
            "../../../../research/water-service-engineering/water-verified-intervention-utility-t1-input.v0.1.json"
        ))
        .unwrap();
        let schema = serde_json::from_slice(include_bytes!(
            "../../../../research/water-service-engineering/water-verified-intervention-utility-t1-input.schema.v0.1.json"
        ))
        .unwrap();
        let oracle = serde_json::from_slice(include_bytes!(
            "../../../../research/water-service-engineering/water-verified-intervention-utility-t1-oracle.v0.1.json"
        ))
        .unwrap();
        (input, schema, oracle)
    }

    fn result_scenario<'a>(result: &'a Value, id: &str) -> &'a Value {
        result["scenarios"]
            .as_array()
            .unwrap()
            .iter()
            .find(|scenario| scenario["scenario_id"] == id)
            .unwrap()
    }

    #[test]
    fn canonical_twelve_scenario_oracle_matches() {
        let (input, schema, oracle) = fixture();
        let result = evaluate_all(&input, &schema, &oracle).unwrap();
        assert_eq!(result["scenario_count"], 12);
        assert_eq!(result["matched_scenario_count"], 12);
        assert_eq!(result["mismatch_count"], 0);
        assert_eq!(result["aggregate"]["false_safe_pipe_claim_count"], 3);
        assert_eq!(result["physical_qualification_status"], "NotRun");
        assert_eq!(result["statistical_inference_status"], "NotRun");
        assert_eq!(result["claim_eligibility"], "NotEligible");
    }

    #[test]
    fn unobservable_structural_only_and_scope_mismatch_pipe_claims_are_false_safe() {
        let (input, _, _) = fixture();
        let ids = [
            "IVU-T1-002-unobservable-false-pipe-claim",
            "IVU-T1-005-structural-only-multiple-pipes",
            "IVU-T1-006-estimator-scope-mismatch",
        ];
        for id in ids {
            let output = evaluate_scenario(
                input["scenarios"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|scenario| scenario["scenario_id"] == id)
                    .unwrap(),
                &input,
            )
            .unwrap();
            assert_eq!(output["false_safe_pipe_claim"], true);
            assert_eq!(output["pipe_claim_eligible"], false);
        }
    }

    #[test]
    fn no_leak_pressure_change_is_false_dispatch_not_true_positive() {
        let (input, _, _) = fixture();
        let output = evaluate_scenario(&input["scenarios"][2], &input).unwrap();
        assert_eq!(output["true_positive"], false);
        assert_eq!(output["false_positive"], true);
        assert_eq!(output["false_dispatch"], true);
    }

    #[test]
    fn missed_leak_is_false_negative_and_abstention_is_not_a_true_positive() {
        let (input, _, _) = fixture();
        let output = evaluate_scenario(&input["scenarios"][3], &input).unwrap();
        assert_eq!(output["false_negative"], true);
        assert_eq!(output["true_positive"], false);
        assert_eq!(output["abstained"], true);
    }

    #[test]
    fn seven_day_repair_does_not_claim_thirty_or_ninety_day_persistence() {
        let (input, schema, oracle) = fixture();
        let result = evaluate_all(&input, &schema, &oracle).unwrap();
        let output = result_scenario(
            &result,
            "IVU-T1-007-repair-only-seven-day-followup",
        );
        assert_eq!(output["verified_lower_reduction_7d_m3_day"], 6.2);
        assert!(output["verified_lower_reduction_30d_m3_day"].is_null());
        assert!(output["verified_lower_reduction_90d_m3_day"].is_null());
    }

    #[test]
    fn unmatched_pressure_makes_post_repair_verification_indeterminate() {
        let (input, schema, oracle) = fixture();
        let result = evaluate_all(&input, &schema, &oracle).unwrap();
        let output = result_scenario(
            &result,
            "IVU-T1-008-post-pressure-mismatch",
        );
        assert_eq!(
            output["repair_verification"][0]["status"],
            "indeterminate_operating_conditions"
        );
    }

    #[test]
    fn reused_raw_pre_post_record_is_not_verification() {
        let (input, schema, oracle) = fixture();
        let result = evaluate_all(&input, &schema, &oracle).unwrap();
        let output = result_scenario(
            &result,
            "IVU-T1-009-replayed-raw-record",
        );
        assert_eq!(
            output["repair_verification"][0]["status"],
            "indeterminate_evidence_scope"
        );
    }

    #[test]
    fn repair_asset_mismatch_does_not_earn_repair_or_verification_credit() {
        let (input, schema, oracle) = fixture();
        let result = evaluate_all(&input, &schema, &oracle).unwrap();
        let output = result_scenario(
            &result,
            "IVU-T1-010-repair-asset-mismatch",
        );
        assert_eq!(output["repair_consistent"], false);
        assert_eq!(output["repair_verification"][0]["status"], "not_applicable");
    }

    #[test]
    fn zone_localization_and_missing_post_repair_evidence_do_not_create_pipe_or_saving_claims() {
        let (input, schema, oracle) = fixture();
        let result = evaluate_all(&input, &schema, &oracle).unwrap();
        let output = result_scenario(
            &result,
            "IVU-T1-011-zone-inspection-no-post-repair-evidence",
        );
        assert_eq!(output["pipe_claim_eligible"], false);
        assert!(output["verified_lower_reduction_7d_m3_day"].is_null());
    }

    #[test]
    fn prioritization_without_inspection_has_no_confirmed_repair_or_savings() {
        let (input, schema, oracle) = fixture();
        let result = evaluate_all(&input, &schema, &oracle).unwrap();
        let output = result_scenario(
            &result,
            "IVU-T1-012-prioritized-not-inspected",
        );
        assert_eq!(output["inspection_confirmed"], false);
        assert_eq!(output["repair_consistent"], false);
        assert!(output["verified_lower_reduction_7d_m3_day"].is_null());
    }

    #[test]
    fn abstained_leak_is_not_counted_as_a_true_positive_or_pipe_claim() {
        let (input, _, _) = fixture();
        let mut scenario = input["scenarios"][0].clone();
        scenario["diagnostic"]["action"] = json!("abstain");
        let output = evaluate_scenario(&scenario, &input).unwrap();
        assert_eq!(output["true_positive"], false);
        assert_eq!(output["false_negative"], true);
        assert_eq!(output["pipe_claim_eligible"], false);
        assert_eq!(output["localization_correct"], false);
    }

    #[test]
    fn post_repair_followup_before_repair_time_is_not_credited() {
        let (input, _, _) = fixture();
        let mut scenario = input["scenarios"][0].clone();
        scenario["verification"]["followups"][0]["measurement"]["window_start_minute"] =
            json!(100);
        scenario["verification"]["followups"][0]["measurement"]["window_end_minute"] =
            json!(150);
        let output = evaluate_scenario(&scenario, &input).unwrap();
        assert_eq!(
            output["repair_verification"][0]["status"],
            "indeterminate_evidence_scope"
        );
    }

    #[test]
    fn replayed_post_measurement_between_followup_days_invalidates_both() {
        let (input, _, _) = fixture();
        let mut scenario = input["scenarios"][0].clone();
        scenario["verification"]["followups"][1]["measurement"]["raw_record_digest"] =
            scenario["verification"]["followups"][0]["measurement"]["raw_record_digest"].clone();
        let output = evaluate_scenario(&scenario, &input).unwrap();
        assert_eq!(
            output["repair_verification"][0]["status"],
            "indeterminate_evidence_scope"
        );
        assert_eq!(
            output["repair_verification"][1]["status"],
            "indeterminate_evidence_scope"
        );
    }

    #[test]
    fn altered_oracle_reports_exact_mismatch_without_changing_eligibility() {
        let (input, schema, mut oracle) = fixture();
        oracle["scenarios"][0]["verified_lower_reduction_7d_m3_day"] = json!(999.0);
        let result = evaluate_all(&input, &schema, &oracle).unwrap();
        assert_eq!(result["matched_scenario_count"], 11);
        assert_eq!(result["mismatch_count"], 1);
    }
}
