// SPDX-License-Identifier: AGPL-3.0-or-later
//! Synthetic-first verified water-loss recovery event evaluator.
//! A model alert or closed work order is not proof of water recovered.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::Path;

const INPUT_VERSION: &str = "vwlr-t0-input-v0.1";
const ORACLE_VERSION: &str = "vwlr-t0-oracle-v0.1";
const BENCHMARK_ID: &str = "WATER-VERIFIED-LOSS-RECOVERY-T0-001";
const CASE_COUNT: usize = 13;
const MIN_FOLLOW_UP_HOURS: u64 = 168;

#[derive(Clone, Debug)]
struct LossMeasurement {
    instrument_id: String,
    zone_id: String,
    boundary_id: String,
    topology_revision: String,
    topology_digest: String,
    loss_l_per_day: f64,
    uncertainty_l_per_day: f64,
    pressure_kpa: f64,
    demand_l_per_day: f64,
    window_start_utc: String,
    window_end_utc: String,
    calibration_status: String,
    time_status: String,
    method_ref: String,
    raw_record_digest: String,
}

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

fn ensure_keys(value: &Value, expected: &[&str], context: &str) -> Result<(), String> {
    let object = value.as_object().ok_or_else(|| format!("{context}: must be an object"))?;
    let actual: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let wanted: BTreeSet<&str> = expected.iter().copied().collect();
    if actual != wanted {
        return Err(format!("{context}: key set differs from the frozen v0.1 contract"));
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_utc(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 20
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[13] == b':'
        && bytes[16] == b':'
        && bytes[19] == b'Z'
        && bytes.iter().enumerate().all(|(index, byte)| {
            [4, 7, 10, 13, 16, 19].contains(&index) || byte.is_ascii_digit()
        })
}

fn parse_measurement(value: &Value, context: &str) -> Result<LossMeasurement, String> {
    ensure_keys(
        value,
        &[
            "measurement_id",
            "instrument_id",
            "zone_id",
            "boundary_id",
            "topology_revision",
            "topology_digest",
            "net_loss_l_per_day",
            "standard_uncertainty_l_per_day",
            "pressure_kpa",
            "demand_l_per_day",
            "window_start_utc",
            "window_end_utc",
            "calibration_status",
            "time_status",
            "method_ref",
            "raw_record_digest",
        ],
        context,
    )?;
    let measurement_id = string(value, "measurement_id", context)?;
    let instrument_id = string(value, "instrument_id", context)?.to_owned();
    let zone_id = string(value, "zone_id", context)?.to_owned();
    let boundary_id = string(value, "boundary_id", context)?.to_owned();
    let topology_revision = string(value, "topology_revision", context)?.to_owned();
    let topology_digest = string(value, "topology_digest", context)?.to_owned();
    let loss_l_per_day = number(value, "net_loss_l_per_day", context)?;
    let uncertainty_l_per_day =
        number(value, "standard_uncertainty_l_per_day", context)?;
    let pressure_kpa = number(value, "pressure_kpa", context)?;
    let demand_l_per_day = number(value, "demand_l_per_day", context)?;
    let window_start_utc = string(value, "window_start_utc", context)?.to_owned();
    let window_end_utc = string(value, "window_end_utc", context)?.to_owned();
    let calibration_status = string(value, "calibration_status", context)?.to_owned();
    let time_status = string(value, "time_status", context)?.to_owned();
    let method_ref = string(value, "method_ref", context)?.to_owned();
    let raw_record_digest = string(value, "raw_record_digest", context)?.to_owned();

    if measurement_id.trim().is_empty()
        || instrument_id.trim().is_empty()
        || zone_id.trim().is_empty()
        || boundary_id.trim().is_empty()
        || topology_revision.trim().is_empty()
        || method_ref.trim().is_empty()
    {
        return Err(format!("{context}: measurement identity fields must be non-empty"));
    }
    if loss_l_per_day < 0.0
        || uncertainty_l_per_day < 0.0
        || pressure_kpa < 0.0
        || demand_l_per_day < 0.0
    {
        return Err(format!("{context}: negative measurement/uncertainty is invalid"));
    }
    if !valid_digest(&topology_digest) || !valid_digest(&raw_record_digest) {
        return Err(format!("{context}: topology/raw evidence digest is invalid"));
    }
    if !valid_utc(&window_start_utc)
        || !valid_utc(&window_end_utc)
        || window_start_utc >= window_end_utc
    {
        return Err(format!("{context}: measurement window is invalid"));
    }
    if !["current", "stale", "missing"].contains(&calibration_status.as_str())
        || !["current", "invalid"].contains(&time_status.as_str())
    {
        return Err(format!("{context}: unsupported calibration/time status"));
    }

    Ok(LossMeasurement {
        instrument_id,
        zone_id,
        boundary_id,
        topology_revision,
        topology_digest,
        loss_l_per_day,
        uncertainty_l_per_day,
        pressure_kpa,
        demand_l_per_day,
        window_start_utc,
        window_end_utc,
        calibration_status,
        time_status,
        method_ref,
        raw_record_digest,
    })
}

fn empty_output(event: &Value, disposition: &str) -> Value {
    let inspection_cost = event
        .get("inspection")
        .and_then(|value| value.get("cost_zar"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let crew_hours = event
        .get("inspection")
        .and_then(|value| value.get("crew_hours"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let repair_cost = event
        .get("repair")
        .filter(|value| !value.is_null())
        .and_then(|value| value.get("repair_cost_zar"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    json!({
        "event_id": event.get("event_id").cloned().unwrap_or(Value::Null),
        "disposition": disposition,
        "localization_correct": null,
        "inspection_cost_zar": inspection_cost,
        "crew_hours": crew_hours,
        "repair_cost_zar": repair_cost,
        "point_recovered_l_per_day": null,
        "lower_bound_recovered_l_per_day": null,
        "sustained_recovery_eligible": false
    })
}

fn output_with_measurement(
    event: &Value,
    disposition: &str,
    localization_correct: bool,
    point: f64,
    lower: f64,
    sustained: bool,
) -> Value {
    let mut output = empty_output(event, disposition);
    output["localization_correct"] = json!(localization_correct);
    output["point_recovered_l_per_day"] = json!(point);
    output["lower_bound_recovered_l_per_day"] = json!(lower);
    output["sustained_recovery_eligible"] = json!(sustained);
    output
}

fn output_without_attribution(
    event: &Value,
    disposition: &str,
    localization_correct: bool,
) -> Value {
    let mut output = empty_output(event, disposition);
    output["localization_correct"] = json!(localization_correct);
    output
}

fn duplicate_recovery_claims(events: &[Value]) -> Result<BTreeSet<String>, String> {
    let mut counts = BTreeMap::<String, usize>::new();
    for (index, event) in events.iter().enumerate() {
        let claim_id = string(event, "recovery_claim_id", &format!("events[{index}]"))?;
        if claim_id.trim().is_empty() {
            return Err(format!("events[{index}]: recovery_claim_id is empty"));
        }
        *counts.entry(claim_id.to_owned()).or_default() += 1;
    }
    Ok(counts
        .into_iter()
        .filter_map(|(id, count)| (count > 1).then_some(id))
        .collect())
}

fn relative_difference_percent(left: f64, right: f64) -> f64 {
    (left - right).abs() / left.max(right).max(1.0) * 100.0
}

fn evaluate_event(
    event: &Value,
    minimum_follow_up_hours: u64,
    duplicated_claims: &BTreeSet<String>,
) -> Result<Value, String> {
    let event_id = string(event, "event_id", "event")?;
    let context = format!("event {event_id}");
    let claim_id = string(event, "recovery_claim_id", &context)?;
    if duplicated_claims.contains(claim_id) {
        return Ok(empty_output(event, "duplicate_recovery_claim_rejected"));
    }

    let confirmation = field(event, "field_confirmation", &context)?;
    let confirmation_disposition = string(confirmation, "disposition", &context)?;
    if confirmation_disposition == "confirmed_no_leak" {
        return Ok(empty_output(event, "no_leak_confirmed"));
    }
    if confirmation_disposition != "confirmed_leak" {
        return Ok(empty_output(event, "field_confirmation_indeterminate"));
    }
    if confirmation
        .get("evidence_ref")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Ok(empty_output(event, "field_confirmation_indeterminate"));
    }

    let diagnostic = field(event, "diagnostic", &context)?;
    let predicted_ids = field(diagnostic, "predicted_component_ids", &context)?
        .as_array()
        .ok_or_else(|| format!("{context}: predicted_component_ids must be an array"))?;
    let confirmed_ids = field(confirmation, "confirmed_component_ids", &context)?
        .as_array()
        .ok_or_else(|| format!("{context}: confirmed_component_ids must be an array"))?;
    let predicted: BTreeSet<String> = predicted_ids
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let confirmed: BTreeSet<String> = confirmed_ids
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let localization_correct = !predicted.is_disjoint(&confirmed);

    let repair_value = field(event, "repair", &context)?;
    if repair_value.is_null() {
        return Ok(output_with_measurement(
            event,
            "not_verified_no_repair",
            localization_correct,
            0.0,
            0.0,
            false,
        ));
    }
    let repair_status = string(repair_value, "status", &context)?;
    if !["completed", "partial"].contains(&repair_status) {
        return Ok(output_without_attribution(event, "repair_not_successful", localization_correct));
    }
    let repair_targets = field(repair_value, "target_component_ids", &context)?
        .as_array()
        .ok_or_else(|| format!("{context}: repair target_component_ids must be an array"))?;
    let target_ids: BTreeSet<String> = repair_targets
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    if target_ids.is_disjoint(&confirmed) {
        return Ok(output_with_measurement(
            event,
            "repair_target_mismatch_no_credit",
            localization_correct,
            0.0,
            0.0,
            false,
        ));
    }

    let measurement_keys = ["treated_pre", "treated_post", "control_pre", "control_post"];
    let mut raw = Vec::with_capacity(measurement_keys.len());
    for key in measurement_keys {
        let measurement = event.get(key).filter(|value| !value.is_null());
        let Some(measurement) = measurement else {
            return Ok(output_without_attribution(event, "not_verified_missing_post_or_control", localization_correct));
        };
        raw.push(parse_measurement(measurement, &format!("{context}.{key}"))?);
    }
    let treated_pre = &raw[0];
    let treated_post = &raw[1];
    let control_pre = &raw[2];
    let control_post = &raw[3];

    if raw.iter().any(|measurement| {
        measurement.calibration_status != "current" || measurement.time_status != "current"
    }) {
        return Ok(output_without_attribution(event, "attribution_indeterminate_invalid_measurement", localization_correct));
    }

    let treated_boundary = string(event, "measurement_boundary_id", &context)?;
    let control_boundary = string(event, "control_boundary_id", &context)?;
    let topology = field(event, "topology", &context)?;
    let topology_revision = string(topology, "revision", &context)?;
    let topology_digest = string(topology, "digest", &context)?;
    if !valid_digest(topology_digest) {
        return Err(format!("{context}: event topology digest is invalid"));
    }
    let same_topology = raw.iter().all(|measurement| {
        measurement.topology_revision == topology_revision
            && measurement.topology_digest == topology_digest
    });
    if treated_pre.boundary_id != treated_boundary
        || treated_post.boundary_id != treated_boundary
        || control_pre.boundary_id != control_boundary
        || control_post.boundary_id != control_boundary
        || treated_pre.boundary_id != treated_post.boundary_id
        || control_pre.boundary_id != control_post.boundary_id
        || treated_pre.zone_id != treated_post.zone_id
        || control_pre.zone_id != control_post.zone_id
        || treated_pre.zone_id == control_pre.zone_id
        || !same_topology
    {
        return Ok(output_without_attribution(event, "attribution_indeterminate_boundary_or_topology", localization_correct));
    }

    let identities: BTreeSet<String> = raw
        .iter()
        .map(|measurement| measurement.instrument_id.clone())
        .collect();
    let digests: BTreeSet<String> = raw
        .iter()
        .map(|measurement| measurement.raw_record_digest.clone())
        .collect();
    let methods: BTreeSet<String> = raw
        .iter()
        .map(|measurement| measurement.method_ref.clone())
        .collect();
    if identities.len() != raw.len() || digests.len() != raw.len() || methods.len() != raw.len() {
        return Ok(output_without_attribution(event, "attribution_indeterminate_duplicate_measurement_identity", localization_correct));
    }

    if treated_pre.window_end_utc > treated_post.window_start_utc
        || control_pre.window_end_utc > control_post.window_start_utc
        || treated_pre.window_start_utc != control_pre.window_start_utc
        || treated_pre.window_end_utc != control_pre.window_end_utc
        || treated_post.window_start_utc != control_post.window_start_utc
        || treated_post.window_end_utc != control_post.window_end_utc
    {
        return Ok(output_without_attribution(event, "attribution_indeterminate_time_window", localization_correct));
    }

    let max_pressure_difference =
        number(event, "max_pressure_difference_percent", &context)?;
    let max_demand_difference = number(event, "max_demand_difference_percent", &context)?;
    for (before, after) in [(treated_pre, treated_post), (control_pre, control_post)] {
        if relative_difference_percent(before.pressure_kpa, after.pressure_kpa)
            > max_pressure_difference
            || relative_difference_percent(before.demand_l_per_day, after.demand_l_per_day)
                > max_demand_difference
        {
            return Ok(output_without_attribution(event, "attribution_indeterminate_operating_context", localization_correct));
        }
    }

    let point_raw = (treated_pre.loss_l_per_day - treated_post.loss_l_per_day)
        - (control_pre.loss_l_per_day - control_post.loss_l_per_day);
    let point = point_raw.max(0.0);
    let expansion = number(event, "uncertainty_expansion_factor", &context)?;
    if expansion <= 0.0 {
        return Err(format!("{context}: uncertainty expansion factor must be positive"));
    }
    let uncertainty_sum: f64 = raw
        .iter()
        .map(|measurement| measurement.uncertainty_l_per_day)
        .sum();
    let lower = (point - expansion * uncertainty_sum).max(0.0);
    if point <= 0.0 || lower <= 0.0 {
        return Ok(output_with_measurement(
            event,
            "no_recovery_measured",
            localization_correct,
            point,
            lower,
            false,
        ));
    }

    let follow_up = field(event, "follow_up", &context)?;
    let follow_up_hours = field(follow_up, "window_hours", &context)?
        .as_u64()
        .ok_or_else(|| format!("{context}: follow-up window_hours must be an integer"))?;
    let recurrence = field(follow_up, "recurrence_detected", &context)?
        .as_bool()
        .ok_or_else(|| format!("{context}: recurrence_detected must be boolean"))?;
    let reopen_count = field(follow_up, "reopen_count", &context)?
        .as_u64()
        .ok_or_else(|| format!("{context}: reopen_count must be a non-negative integer"))?;

    if recurrence || reopen_count > 0 {
        return Ok(output_with_measurement(
            event,
            "recurrence_defeats_sustained_claim",
            localization_correct,
            point,
            lower,
            false,
        ));
    }
    if follow_up_hours < minimum_follow_up_hours {
        return Ok(output_with_measurement(
            event,
            "follow_up_window_incomplete",
            localization_correct,
            point,
            lower,
            false,
        ));
    }

    let partial = repair_status == "partial" || confirmed.len() > target_ids.len();
    let disposition = if partial {
        "partial_recovery_verified"
    } else {
        "repair_and_recovery_verified"
    };
    Ok(output_with_measurement(
        event,
        disposition,
        localization_correct,
        point,
        lower,
        !partial,
    ))
}

fn evaluate_case(case: &Value, minimum_follow_up_hours: u64) -> Result<Value, String> {
    ensure_keys(case, &["case_id", "events"], "case")?;
    let case_id = string(case, "case_id", "case")?;
    let events = field(case, "events", "case")?
        .as_array()
        .ok_or_else(|| format!("{case_id}: events must be an array"))?;
    if events.is_empty() || events.len() > 2 {
        return Err(format!("{case_id}: v0.1 requires one or two events per case"));
    }
    let duplicate_claims = duplicate_recovery_claims(events)?;
    let mut event_ids = BTreeSet::new();
    let mut repair_ids = BTreeSet::new();
    let mut output = Vec::with_capacity(events.len());

    for event in events {
        let event_id = string(event, "event_id", "event")?;
        if !event_ids.insert(event_id.to_owned()) {
            return Err(format!("{case_id}: duplicate event_id '{event_id}'"));
        }
        if let Some(repair_id) = event
            .get("repair")
            .filter(|value| !value.is_null())
            .and_then(|value| value.get("repair_id"))
            .and_then(Value::as_str)
        {
            if !repair_ids.insert(repair_id.to_owned()) {
                return Err(format!("{case_id}: duplicate repair_id '{repair_id}'"));
            }
        }
        output.push(evaluate_event(
            event,
            minimum_follow_up_hours,
            &duplicate_claims,
        )?);
    }

    let verified_count = output
        .iter()
        .filter(|event| event["disposition"] == "repair_and_recovery_verified")
        .count();
    let partial_count = output
        .iter()
        .filter(|event| event["disposition"] == "partial_recovery_verified")
        .count();
    let point_recovered: f64 = output
        .iter()
        .map(|event| event["point_recovered_l_per_day"].as_f64().unwrap_or(0.0))
        .sum();
    let lower_recovered: f64 = output
        .iter()
        .map(|event| event["lower_bound_recovered_l_per_day"].as_f64().unwrap_or(0.0))
        .sum();

    Ok(json!({
        "case_id": case_id,
        "event_count": output.len(),
        "events": output,
        "verified_recovery_event_count": verified_count,
        "partial_recovery_event_count": partial_count,
        "lower_bound_recovered_l_per_day": lower_recovered,
        "point_recovered_l_per_day": point_recovered,
        "claim_eligibility": "NotEligible"
    }))
}

fn evaluate_all(input: &Value, input_schema: &Value, oracle: &Value) -> Result<Value, String> {
    if string(input, "$schema", "input")? != "https://json-schema.org/draft/2020-12/schema"
        || string(input_schema, "$schema", "input schema")?
            != "https://json-schema.org/draft/2020-12/schema"
    {
        return Err("input/input schema must declare Draft 2020-12".to_owned());
    }
    if string(input, "schema_version", "input")? != INPUT_VERSION
        || string(input, "benchmark_id", "input")? != BENCHMARK_ID
        || string(input, "evidence_class", "input")? != "synthetic_only"
    {
        return Err("input identity/evidence class mismatch".to_owned());
    }
    let properties = field(input_schema, "properties", "input schema")?;
    if string(
        field(properties, "schema_version", "input schema properties")?,
        "const",
        "input schema version",
    )? != INPUT_VERSION
    {
        return Err("input schema version constant mismatch".to_owned());
    }
    let minimum_follow_up = field(input, "minimum_follow_up_hours", "input")?
        .as_u64()
        .ok_or_else(|| "minimum_follow_up_hours must be an integer".to_owned())?;
    if minimum_follow_up != MIN_FOLLOW_UP_HOURS {
        return Err("v0.1 requires a 168-hour follow-up window".to_owned());
    }

    ensure_keys(
        oracle,
        &["$schema", "schema_version", "benchmark_id", "cases"],
        "oracle",
    )?;
    if string(oracle, "$schema", "oracle")? != "https://json-schema.org/draft/2020-12/schema"
        || string(oracle, "schema_version", "oracle")? != ORACLE_VERSION
        || string(oracle, "benchmark_id", "oracle")? != BENCHMARK_ID
    {
        return Err("oracle identity/schema version mismatch".to_owned());
    }

    let cases = field(input, "cases", "input")?
        .as_array()
        .ok_or_else(|| "input.cases must be an array".to_owned())?;
    let expected_cases = field(oracle, "cases", "oracle")?
        .as_array()
        .ok_or_else(|| "oracle.cases must be an array".to_owned())?;
    if cases.len() != CASE_COUNT || expected_cases.len() != CASE_COUNT {
        return Err(format!("exactly {CASE_COUNT} input and oracle cases are required"));
    }
    let mut expected_by_id = BTreeMap::new();
    for expected in expected_cases {
        let id = string(expected, "case_id", "oracle case")?.to_owned();
        if expected_by_id.insert(id.clone(), expected).is_some() {
            return Err(format!("duplicate oracle case_id '{id}'"));
        }
    }
    let mut input_ids = BTreeSet::new();
    let mut results = Vec::with_capacity(CASE_COUNT);
    let mut mismatches = Vec::new();
    for case in cases {
        let id = string(case, "case_id", "case")?.to_owned();
        if !input_ids.insert(id.clone()) {
            return Err(format!("duplicate input case_id '{id}'"));
        }
        let actual = evaluate_case(case, minimum_follow_up)?;
        let expected = expected_by_id
            .get(&id)
            .ok_or_else(|| format!("input case '{id}' missing from oracle"))?;
        if &actual != *expected {
            mismatches.push(json!({
                "case_id": id,
                "expected": expected,
                "actual": actual.clone()
            }));
        }
        results.push(actual);
    }
    if input_ids != expected_by_id.keys().cloned().collect() {
        return Err("input/oracle case ID sets differ".to_owned());
    }
    let mismatch_count = mismatches.len();
    Ok(json!({
        "schema_version": "vwlr-t0-evaluation-v0.1",
        "benchmark_id": BENCHMARK_ID,
        "case_count": CASE_COUNT,
        "matched_case_count": CASE_COUNT - mismatch_count,
        "mismatch_count": mismatch_count,
        "mismatches": mismatches,
        "case_results": results,
        "physical_qualification_status": "NotRun",
        "statistical_inference_status": "NotRun",
        "claim_eligibility": "NotEligible",
        "claim_ceiling": "synthetic_verified_repair_and_loss_attribution_contract_only"
    }))
}

fn read_json(path: &str) -> Result<(Vec<u8>, Value), String> {
    let bytes = fs::read(path).map_err(|error| format!("cannot read '{path}': {error}"))?;
    let value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid JSON '{path}': {error}"))?;
    Ok((bytes, value))
}

fn valid_revision(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn run(args: &[String]) -> Result<(), String> {
    if args.len() != 4 {
        return Err("usage: vwlr-t0 INPUT_JSON INPUT_SCHEMA ORACLE_JSON RECEIPT_JSON".to_owned());
    }
    let (input_bytes, input) = read_json(&args[0])?;
    let (schema_bytes, schema) = read_json(&args[1])?;
    let (oracle_bytes, oracle) = read_json(&args[2])?;
    let result = evaluate_all(&input, &schema, &oracle)?;
    let success = result["case_count"].as_u64() == Some(CASE_COUNT as u64)
        && result["matched_case_count"].as_u64() == Some(CASE_COUNT as u64)
        && result["mismatch_count"].as_u64() == Some(0);

    let revision = env::var("VWLR_T0_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok()
        .filter(|value| valid_revision(value));
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let lock_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let manifest_bytes = fs::read(&manifest_path)
        .map_err(|error| format!("read {}: {error}", manifest_path.display()))?;
    let lock_bytes = fs::read(&lock_path)
        .map_err(|error| format!("read {}: {error}", lock_path.display()))?;
    let implementation = include_bytes!("vwlr-t0.rs");
    let receipt = json!({
        "receipt_schema_version": "vwlr-t0-receipt-v0.1",
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
        "case_count": result["case_count"],
        "matched_case_count": result["matched_case_count"],
        "mismatch_count": result["mismatch_count"],
        "mismatches": result["mismatches"],
        "claim_ceiling": "synthetic_verified_repair_and_loss_attribution_contract_only"
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
        Err("VWLR evaluator did not match the frozen oracle".to_owned())
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Err(error) = run(&args) {
        eprintln!("vwlr-t0: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Value, Value, Value) {
        let input = serde_json::from_slice(include_bytes!(
            "../../../../docs/research/water-verified-loss-recovery/vwlr_t0_input.v0.1.json"
        ))
        .unwrap();
        let schema = serde_json::from_slice(include_bytes!(
            "../../../../docs/research/water-verified-loss-recovery/vwlr_t0_input.schema.v0.1.json"
        ))
        .unwrap();
        let oracle = serde_json::from_slice(include_bytes!(
            "../../../../docs/research/water-verified-loss-recovery/vwlr_t0_oracle.v0.1.json"
        ))
        .unwrap();
        (input, schema, oracle)
    }

    #[test]
    fn canonical_thirteen_case_oracle_matches() {
        let (input, schema, oracle) = fixture();
        let result = evaluate_all(&input, &schema, &oracle).unwrap();
        assert_eq!(result["case_count"], 13);
        assert_eq!(result["matched_case_count"], 13);
        assert_eq!(result["mismatch_count"], 0);
        assert_eq!(result["physical_qualification_status"], "NotRun");
        assert_eq!(result["statistical_inference_status"], "NotRun");
        assert_eq!(result["claim_eligibility"], "NotEligible");
    }

    #[test]
    fn successful_repair_requires_independent_confirmation_and_post_test() {
        let (input, _, _) = fixture();
        let result = evaluate_case(&input["cases"][0], 168).unwrap();
        assert_eq!(result["verified_recovery_event_count"], 1);
        assert_eq!(result["point_recovered_l_per_day"], 700.0);
        assert_eq!(result["lower_bound_recovered_l_per_day"], 640.0);
    }

    #[test]
    fn closed_work_order_without_post_measurement_is_not_verified() {
        let (input, _, _) = fixture();
        let result = evaluate_case(&input["cases"][1], 168).unwrap();
        assert_eq!(
            result["events"][0]["disposition"],
            "not_verified_missing_post_or_control"
        );
        assert_eq!(result["verified_recovery_event_count"], 0);
    }

    #[test]
    fn partial_repair_receives_only_partial_measured_lower_bound() {
        let (input, _, _) = fixture();
        let result = evaluate_case(&input["cases"][2], 168).unwrap();
        assert_eq!(result["partial_recovery_event_count"], 1);
        assert_eq!(result["lower_bound_recovered_l_per_day"], 340.0);
    }

    #[test]
    fn pressure_or_demand_change_makes_attribution_indeterminate() {
        let (input, _, _) = fixture();
        for index in [3, 4] {
            let result = evaluate_case(&input["cases"][index], 168).unwrap();
            assert_eq!(
                result["events"][0]["disposition"],
                "attribution_indeterminate_operating_context"
            );
            assert_eq!(result["lower_bound_recovered_l_per_day"], 0.0);
        }
    }

    #[test]
    fn stale_meter_calibration_blocks_recovery_credit() {
        let (input, _, _) = fixture();
        let result = evaluate_case(&input["cases"][5], 168).unwrap();
        assert_eq!(
            result["events"][0]["disposition"],
            "attribution_indeterminate_invalid_measurement"
        );
    }

    #[test]
    fn false_pipe_localization_is_preserved_and_wrong_repair_gets_no_credit() {
        let (input, _, _) = fixture();
        let result = evaluate_case(&input["cases"][6], 168).unwrap();
        assert_eq!(result["events"][0]["localization_correct"], false);
        assert_eq!(
            result["events"][0]["disposition"],
            "repair_target_mismatch_no_credit"
        );
        assert_eq!(result["lower_bound_recovered_l_per_day"], 0.0);
    }

    #[test]
    fn false_alarm_is_not_counted_as_a_leak_or_recovery() {
        let (input, _, _) = fixture();
        let result = evaluate_case(&input["cases"][7], 168).unwrap();
        assert_eq!(result["events"][0]["disposition"], "no_leak_confirmed");
        assert_eq!(result["events"][0]["inspection_cost_zar"], 1200.0);
    }

    #[test]
    fn concurrent_leaks_with_only_one_repaired_are_partial_recovery() {
        let (input, _, _) = fixture();
        let result = evaluate_case(&input["cases"][8], 168).unwrap();
        assert_eq!(result["partial_recovery_event_count"], 1);
        assert_eq!(result["lower_bound_recovered_l_per_day"], 240.0);
    }

    #[test]
    fn failed_repair_does_not_receive_recovery_credit() {
        let (input, _, _) = fixture();
        let result = evaluate_case(&input["cases"][9], 168).unwrap();
        assert_eq!(result["events"][0]["disposition"], "repair_not_successful");
    }

    #[test]
    fn recurrence_defeats_sustained_repair_claim() {
        let (input, _, _) = fixture();
        let result = evaluate_case(&input["cases"][10], 168).unwrap();
        assert_eq!(
            result["events"][0]["disposition"],
            "recurrence_defeats_sustained_claim"
        );
        assert_eq!(result["events"][0]["sustained_recovery_eligible"], false);
    }

    #[test]
    fn duplicate_recovery_claims_are_all_rejected() {
        let (input, _, _) = fixture();
        let result = evaluate_case(&input["cases"][11], 168).unwrap();
        for event in result["events"].as_array().unwrap() {
            assert_eq!(event["disposition"], "duplicate_recovery_claim_rejected");
        }
    }

    #[test]
    fn topology_revision_mismatch_is_indeterminate() {
        let (input, _, _) = fixture();
        let result = evaluate_case(&input["cases"][12], 168).unwrap();
        assert_eq!(
            result["events"][0]["disposition"],
            "attribution_indeterminate_boundary_or_topology"
        );
    }

    #[test]
    fn duplicate_event_ids_are_rejected() {
        let (mut input, _, _) = fixture();
        input["cases"][0]["events"][0]["event_id"] = json!("DUPLICATE");
        input["cases"][0]["events"][0]["recovery_claim_id"] = json!("REC-1");
        let mut second = input["cases"][0]["events"][0].clone();
        second["recovery_claim_id"] = json!("REC-2");
        input["cases"][0]["events"]
            .as_array_mut()
            .unwrap()
            .push(second);
        assert!(evaluate_case(&input["cases"][0], 168)
            .unwrap_err()
            .contains("duplicate event_id"));
    }

    #[test]
    fn tampered_oracle_is_reported_without_relaxing_evidence_rules() {
        let (input, schema, mut oracle) = fixture();
        oracle["cases"][0]["lower_bound_recovered_l_per_day"] = json!(9999.0);
        let result = evaluate_all(&input, &schema, &oracle).unwrap();
        assert_eq!(result["matched_case_count"], 12);
        assert_eq!(result["mismatch_count"], 1);
    }
}
