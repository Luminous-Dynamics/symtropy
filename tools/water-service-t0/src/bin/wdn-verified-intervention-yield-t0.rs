// SPDX-License-Identifier: AGPL-3.0-or-later
//! Synthetic-first verified intervention yield evaluator.
//! It records matched-condition observed change, not causal water savings.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::Path;

const INPUT_VERSION: &str = "wdn-verified-intervention-yield-t0-v0.1";
const ORACLE_VERSION: &str = "wdn-verified-intervention-yield-t0-oracle-v0.1";
const BENCHMARK_ID: &str = "WDN-VERIFIED-INTERVENTION-YIELD-001";
const CASE_COUNT: usize = 14;

fn field<'a>(value: &'a Value, key: &str, context: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("{context}: missing '{key}'"))
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

fn ensure_keys(value: &Value, expected: &[&str], context: &str) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{context}: must be an object"))?;
    let actual: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let wanted: BTreeSet<&str> = expected.iter().copied().collect();
    if actual != wanted {
        return Err(format!(
            "{context}: key set differs from the frozen v0.1 contract"
        ));
    }
    Ok(())
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_canonical_utc(value: &str) -> bool {
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

fn round_9(value: f64) -> f64 {
    (value * 1_000_000_000.0).round() / 1_000_000_000.0
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn base_event_output(event: &Value, disposition: &str) -> Result<Value, String> {
    let inspection = event.get("inspection").filter(|value| !value.is_null());
    let repair = event.get("repair").filter(|value| !value.is_null());
    let service = field(event, "service", "event")?;
    let inspection_hours = inspection
        .and_then(|value| value.get("crew_hours"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let repair_hours = repair
        .and_then(|value| value.get("crew_hours"))
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let repair_cost = repair
        .and_then(|value| value.get("cost_minor"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let currency = repair
        .and_then(|value| value.get("currency"))
        .and_then(Value::as_str)
        .unwrap_or("ZAR");
    let service_interruption = number(service, "service_interruption_minutes", "service")?;
    let service_floor_violation = boolean(service, "hard_floor_violation", "service")?;
    Ok(json!({
        "event_id": string(event, "event_id", "event")?,
        "disposition": disposition,
        "alert_asset_id": event
            .get("alert")
            .and_then(|value| value.get("candidate_asset_id"))
            .and_then(Value::as_str),
        "confirmed_asset_id": inspection
            .and_then(|value| value.get("confirmed_asset_id"))
            .and_then(Value::as_str),
        "localization_asset_match": false,
        "independent_confirmation": false,
        "repair_completed": false,
        "independent_post_repair_verification": false,
        "observed_flow_reduction_lps": null,
        "lower_bound_flow_reduction_lps": null,
        "crew_hours": inspection_hours + repair_hours,
        "repair_cost_minor": repair_cost,
        "currency": currency,
        "service_interruption_minutes": service_interruption,
        "service_floor_violation": service_floor_violation
    }))
}

fn set_confirmation(output: &mut Value, event: &Value) -> Result<(), String> {
    let inspection = field(event, "inspection", "event")?;
    let confirmed_asset = inspection
        .get("confirmed_asset_id")
        .and_then(Value::as_str);
    let alert_asset = string(
        field(event, "alert", "event")?,
        "candidate_asset_id",
        "alert",
    )?;
    output["alert_asset_id"] = json!(alert_asset);
    output["confirmed_asset_id"] = confirmed_asset.map_or(Value::Null, |value| json!(value));
    output["localization_asset_match"] =
        json!(confirmed_asset.is_some_and(|asset| asset == alert_asset));
    output["independent_confirmation"] = json!(true);
    Ok(())
}

fn measurement_is_valid(value: &Value, context: &str) -> Result<bool, String> {
    let required = [
        "measurement_id",
        "meter_id",
        "calibration_ref",
        "calibration_status",
        "method_ref",
        "boundary_id",
        "unit",
        "value",
        "standard_uncertainty",
        "pressure_kpa",
        "demand_profile_digest",
        "start_utc",
        "end_utc",
        "raw_record_digest",
        "validity",
    ];
    ensure_keys(value, &required, context)?;
    let value_number = number(value, "value", context)?;
    let uncertainty = number(value, "standard_uncertainty", context)?;
    let pressure = number(value, "pressure_kpa", context)?;
    let digest = string(value, "raw_record_digest", context)?;
    let status = string(value, "validity", context)?;
    let calibration_status = string(value, "calibration_status", context)?;
    Ok(
        value_number >= 0.0
            && uncertainty >= 0.0
            && pressure >= 0.0
            && status == "valid"
            && calibration_status == "current"
            && string(value, "unit", context)? == "L/s"
            && !string(value, "meter_id", context)?.trim().is_empty()
            && !string(value, "calibration_ref", context)?.trim().is_empty()
            && !string(value, "method_ref", context)?.trim().is_empty()
            && !string(value, "boundary_id", context)?.trim().is_empty()
            && valid_digest(digest)
            && valid_digest(string(value, "demand_profile_digest", context)?)
            && valid_canonical_utc(string(value, "start_utc", context)?)
            && valid_canonical_utc(string(value, "end_utc", context)?),
    )
}

fn event_raw_digests(event: &Value) -> Vec<String> {
    ["pre_measurement", "post_measurement"]
        .into_iter()
        .filter_map(|key| event.get(key))
        .filter(|value| !value.is_null())
        .filter_map(|measurement| {
            measurement
                .get("raw_record_digest")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .collect()
}

fn duplicate_raw_event_ids(input: &Value) -> Result<BTreeSet<String>, String> {
    let cases = field(input, "cases", "input")?
        .as_array()
        .ok_or_else(|| "input.cases must be an array".to_owned())?;
    let mut digest_owners = BTreeMap::<String, Vec<String>>::new();
    let mut all_event_ids = BTreeSet::new();
    for case in cases {
        let events = field(case, "events", "case")?
            .as_array()
            .ok_or_else(|| "case.events must be an array".to_owned())?;
        for event in events {
            let event_id = string(event, "event_id", "event")?.to_owned();
            if !all_event_ids.insert(event_id.clone()) {
                return Err(format!("duplicate event_id '{event_id}' across benchmark input"));
            }
            for digest in event_raw_digests(event) {
                digest_owners
                    .entry(digest)
                    .or_default()
                    .push(event_id.clone());
            }
        }
    }
    let duplicate_digests: BTreeSet<String> = digest_owners
        .into_iter()
        .filter_map(|(digest, owners)| (owners.len() > 1).then_some(digest))
        .collect();
    let mut affected = BTreeSet::new();
    for case in cases {
        let events = field(case, "events", "case")?
            .as_array()
            .ok_or_else(|| "case.events must be an array".to_owned())?;
        for event in events {
            if event_raw_digests(event)
                .iter()
                .any(|digest| duplicate_digests.contains(digest))
            {
                affected.insert(string(event, "event_id", "event")?.to_owned());
            }
        }
    }
    Ok(affected)
}

fn evaluate_event(
    event: &Value,
    case: &Value,
    duplicate_events: &BTreeSet<String>,
) -> Result<Value, String> {
    ensure_keys(
        event,
        &[
            "event_id",
            "topology_revision",
            "dma_id",
            "alert",
            "inspection",
            "repair",
            "verification",
            "pre_measurement",
            "post_measurement",
            "confounding_interventions",
            "service",
            "follow_up_days",
        ],
        "event",
    )?;
    let event_id = string(event, "event_id", "event")?;
    if duplicate_events.contains(event_id) {
        return base_event_output(event, "duplicate_raw_record_replay");
    }

    let alert = field(event, "alert", "event")?;
    if string(alert, "event_id", "alert")? != event_id
        || string(alert, "alert_id", "alert")?.trim().is_empty()
        || string(alert, "evidence_ref", "alert")?.trim().is_empty()
        || !valid_canonical_utc(string(alert, "first_seen_utc", "alert")?)
    {
        return base_event_output(event, "indeterminate_inspection_evidence");
    }

    let inspection = field(event, "inspection", "event")?;
    if string(inspection, "event_id", "inspection")? != event_id
        || string(inspection, "status", "inspection")? != "completed"
        || !boolean(inspection, "independent_review", "inspection")?
        || string(inspection, "inspector_id", "inspection")?.trim().is_empty()
        || inspection
            .get("reviewer_id")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        || inspection
            .get("reviewer_id")
            .and_then(Value::as_str)
            == inspection.get("inspector_id").and_then(Value::as_str)
        || inspection
            .get("confirmation_ref")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        || !valid_canonical_utc(string(inspection, "inspected_at", "inspection")?)
        || string(alert, "first_seen_utc", "alert")? > string(inspection, "inspected_at", "inspection")?
        || string(event, "topology_revision", "event")?.trim().is_empty()
        || string(event, "dma_id", "event")?.trim().is_empty()
    {
        return base_event_output(event, "indeterminate_inspection_evidence");
    }

    let finding = string(inspection, "finding", "inspection")?;
    if finding == "confirmed_no_leak" {
        let mut output = base_event_output(event, "false_dispatch_confirmed_no_leak")?;
        output["independent_confirmation"] = json!(true);
        return Ok(output);
    }
    if finding != "confirmed_leak"
        || inspection
            .get("confirmed_asset_id")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
    {
        return base_event_output(event, "indeterminate_inspection_evidence");
    }

    let mut confirmed = base_event_output(event, "indeterminate")?;
    set_confirmation(&mut confirmed, event)?;
    let confirmed_asset = string(inspection, "confirmed_asset_id", "inspection")?;
    let repair_value = field(event, "repair", "event")?;
    if repair_value.is_null() {
        confirmed["disposition"] = json!("repair_not_completed");
        return Ok(confirmed);
    }
    let repair = repair_value;
    if string(repair, "work_order_id", "repair")?.trim().is_empty()
        || string(repair, "state", "repair")? != "completed"
        || repair
            .get("completed_at")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        || repair
            .get("repair_ref")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
    {
        confirmed["disposition"] = json!("repair_not_completed");
        return Ok(confirmed);
    }
    if string(repair, "event_id", "repair")? != event_id
        || string(repair, "asset_id", "repair")? != confirmed_asset
        || !valid_canonical_utc(string(repair, "completed_at", "repair")?)
        || string(inspection, "inspected_at", "inspection")? > string(repair, "completed_at", "repair")?
    {
        confirmed["disposition"] = json!("repair_asset_mismatch");
        return Ok(confirmed);
    }
    confirmed["repair_completed"] = json!(true);

    let verification_value = field(event, "verification", "event")?;
    if verification_value.is_null() {
        confirmed["disposition"] = json!("post_repair_verification_missing_or_not_independent");
        return Ok(confirmed);
    }
    let verification = verification_value;
    let repair_crew = string(repair, "crew_id", "repair")?;
    if string(verification, "event_id", "verification")? != event_id
        || string(verification, "status", "verification")? != "pass"
        || !boolean(verification, "independent", "verification")?
        || string(verification, "verifier_id", "verification")? == repair_crew
        || string(verification, "verifier_id", "verification")? == string(inspection, "inspector_id", "inspection")?
        || verification
            .get("verification_ref")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        || !valid_canonical_utc(string(verification, "verified_at", "verification")?)
    {
        confirmed["disposition"] = json!("post_repair_verification_missing_or_not_independent");
        return Ok(confirmed);
    }
    confirmed["independent_post_repair_verification"] = json!(true);

    let pre_value = field(event, "pre_measurement", "event")?;
    let post_value = field(event, "post_measurement", "event")?;
    if pre_value.is_null() || post_value.is_null() {
        confirmed["disposition"] = json!("indeterminate_measurement_evidence");
        return Ok(confirmed);
    }
    if !measurement_is_valid(pre_value, "pre_measurement")?
        || !measurement_is_valid(post_value, "post_measurement")?
    {
        confirmed["disposition"] = json!("invalid_or_stale_measurement_evidence");
        return Ok(confirmed);
    }

    let pre = pre_value;
    let post = post_value;
    for key in ["meter_id", "calibration_ref", "method_ref", "boundary_id", "unit"] {
        if string(pre, key, "pre_measurement")? != string(post, key, "post_measurement")? {
            confirmed["disposition"] = json!("measurement_identity_or_boundary_mismatch");
            return Ok(confirmed);
        }
    }
    if string(pre, "demand_profile_digest", "pre_measurement")?
        != string(post, "demand_profile_digest", "post_measurement")?
    {
        confirmed["disposition"] = json!("measurement_identity_or_boundary_mismatch");
        return Ok(confirmed);
    }

    let pre_start = string(pre, "start_utc", "pre_measurement")?;
    let pre_end = string(pre, "end_utc", "pre_measurement")?;
    let post_start = string(post, "start_utc", "post_measurement")?;
    let post_end = string(post, "end_utc", "post_measurement")?;
    let completed_at = string(repair, "completed_at", "repair")?;
    let verified_at = string(verification, "verified_at", "verification")?;
    if pre_start >= pre_end
        || post_start >= post_end
        || pre_end > completed_at
        || post_start < completed_at
        || post_end > verified_at
    {
        confirmed["disposition"] = json!("invalid_pre_post_time_order");
        return Ok(confirmed);
    }

    let confounders = field(event, "confounding_interventions", "event")?
        .as_array()
        .ok_or_else(|| "event.confounding_interventions must be an array".to_owned())?;
    if !confounders.is_empty() {
        confirmed["disposition"] = json!("hydraulic_comparison_confounded");
        return Ok(confirmed);
    }

    let pre_pressure = number(pre, "pressure_kpa", "pre_measurement")?;
    let post_pressure = number(post, "pressure_kpa", "post_measurement")?;
    let pressure_denominator = pre_pressure.max(post_pressure);
    if pressure_denominator <= 0.0 {
        confirmed["disposition"] = json!("hydraulic_comparison_pressure_mismatch");
        return Ok(confirmed);
    }
    let pressure_difference_percent =
        (pre_pressure - post_pressure).abs() / pressure_denominator * 100.0;
    let allowed_pressure_difference =
        number(case, "max_pressure_difference_percent", "case")?;
    if pressure_difference_percent > allowed_pressure_difference {
        confirmed["disposition"] = json!("hydraulic_comparison_pressure_mismatch");
        return Ok(confirmed);
    }

    let pre_flow = number(pre, "value", "pre_measurement")?;
    let post_flow = number(post, "value", "post_measurement")?;
    let pre_uncertainty = number(pre, "standard_uncertainty", "pre_measurement")?;
    let post_uncertainty = number(post, "standard_uncertainty", "post_measurement")?;
    let expansion = number(case, "uncertainty_expansion_factor", "case")?;
    if expansion <= 0.0 {
        return Err("case: uncertainty expansion factor must be positive".to_owned());
    }
    let observed_change = round_9(pre_flow - post_flow);
    let lower_change = round_9(
        (observed_change - expansion * (pre_uncertainty + post_uncertainty)).max(0.0),
    );
    confirmed["observed_flow_reduction_lps"] = json!(observed_change);
    confirmed["lower_bound_flow_reduction_lps"] = json!(lower_change);

    let verification_follow_up = string(verification, "follow_up_status", "verification")?;
    if verification_follow_up == "recurrence_observed" {
        confirmed["disposition"] = json!("recurrence_observed");
        return Ok(confirmed);
    }
    let service = field(event, "service", "event")?;
    if boolean(service, "hard_floor_violation", "service")? {
        confirmed["disposition"] = json!("verified_repair_service_floor_violation");
        return Ok(confirmed);
    }
    if lower_change <= 0.0 {
        confirmed["disposition"] = json!("verified_but_flow_reduction_unresolved");
        return Ok(confirmed);
    }
    confirmed["disposition"] = json!("confirmed_repair_verified");
    Ok(confirmed)
}

fn evaluate_case(case: &Value, duplicate_events: &BTreeSet<String>) -> Result<Value, String> {
    ensure_keys(
        case,
        &[
            "case_id",
            "max_pressure_difference_percent",
            "uncertainty_expansion_factor",
            "events",
        ],
        "case",
    )?;
    let case_id = string(case, "case_id", "case")?;
    let events = field(case, "events", "case")?
        .as_array()
        .ok_or_else(|| format!("{case_id}: events must be an array"))?;
    if events.is_empty() || events.len() > 4 {
        return Err(format!("{case_id}: event count must be 1..4"));
    }
    let mut event_ids = BTreeSet::new();
    let mut dispositions = Vec::with_capacity(events.len());
    for event in events {
        let event_id = string(event, "event_id", "event")?;
        if !event_ids.insert(event_id.to_owned()) {
            return Err(format!("{case_id}: duplicate event_id '{event_id}'"));
        }
        dispositions.push(evaluate_event(event, case, duplicate_events)?);
    }

    let count_disposition = |name: &str| {
        dispositions
            .iter()
            .filter(|event| event["disposition"] == name)
            .count()
    };
    let sum_number = |key: &str| {
        dispositions
            .iter()
            .map(|event| event[key].as_f64().unwrap_or(0.0))
            .sum::<f64>()
    };
    let investigation_count = dispositions
        .iter()
        .filter(|event| event["independent_confirmation"] == true)
        .count();
    let confirmed_leak_count = dispositions
        .iter()
        .filter(|event| {
            event["independent_confirmation"] == true
                && !event["confirmed_asset_id"].is_null()
        })
        .count();
    let repair_completed_count = dispositions
        .iter()
        .filter(|event| event["repair_completed"] == true)
        .count();
    let post_repair_verification_count = dispositions
        .iter()
        .filter(|event| event["independent_post_repair_verification"] == true)
        .count();
    let localization_mismatch_count = dispositions
        .iter()
        .filter(|event| {
            event["independent_confirmation"] == true
                && !event["confirmed_asset_id"].is_null()
                && event["localization_asset_match"] == false
        })
        .count();
    let service_floor_count =
        count_disposition("verified_repair_service_floor_violation");
    let verified_repair_count = count_disposition("confirmed_repair_verified")
        + service_floor_count;

    Ok(json!({
        "case_id": case_id,
        "event_count": dispositions.len(),
        "alert_count": dispositions.len(),
        "investigation_count": investigation_count,
        "independent_confirmed_leak_count": confirmed_leak_count,
        "false_dispatch_count": count_disposition("false_dispatch_confirmed_no_leak"),
        "repair_completed_count": repair_completed_count,
        "independent_post_repair_verification_count": post_repair_verification_count,
        "verified_repair_count": verified_repair_count,
        "recurrence_count": count_disposition("recurrence_observed"),
        "localization_asset_mismatch_count": localization_mismatch_count,
        "observed_flow_reduction_lps": round_9(sum_number("observed_flow_reduction_lps")),
        "lower_bound_flow_reduction_lps": round_9(sum_number("lower_bound_flow_reduction_lps")),
        "crew_hours": sum_number("crew_hours"),
        "repair_cost_minor": dispositions
            .iter()
            .map(|event| event["repair_cost_minor"].as_u64().unwrap_or(0))
            .sum::<u64>(),
        "currency": "ZAR",
        "service_floor_violation_count": service_floor_count,
        "event_dispositions": dispositions,
        "causal_attribution_status": "NotRun",
        "annualized_savings_claim": false,
        "claim_eligibility": "NotEligible"
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
        || boolean(input, "annualized_savings_claims_allowed", "input")?
    {
        return Err("input identity/evidence/annualization guard mismatch".to_owned());
    }
    let properties = field(input_schema, "properties", "input schema")?;
    if string(
        field(properties, "schema_version", "schema properties")?,
        "const",
        "schema properties.schema_version",
    )? != INPUT_VERSION
    {
        return Err("input schema version constant mismatch".to_owned());
    }
    if field(input_schema, "additionalProperties", "input schema")?.as_bool() != Some(false) {
        return Err("input schema must be closed".to_owned());
    }
    ensure_keys(
        oracle,
        &["$schema", "schema_version", "benchmark_id", "cases"],
        "oracle",
    )?;
    if string(oracle, "$schema", "oracle")?
        != "https://json-schema.org/draft/2020-12/schema"
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

    let duplicate_events = duplicate_raw_event_ids(input)?;
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
        let actual = evaluate_case(case, &duplicate_events)?;
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
        "schema_version": "wdn-verified-intervention-yield-t0-evaluation-v0.1",
        "benchmark_id": BENCHMARK_ID,
        "case_count": CASE_COUNT,
        "matched_case_count": CASE_COUNT - mismatch_count,
        "mismatch_count": mismatch_count,
        "mismatches": mismatches,
        "case_results": results,
        "causal_attribution_status": "NotRun",
        "annualized_savings_claim": false,
        "claim_eligibility": "NotEligible",
        "claim_ceiling": "synthetic_inspection_repair_outcome_contract_only"
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
        return Err(
            "usage: wdn-verified-intervention-yield-t0 INPUT_JSON INPUT_SCHEMA ORACLE_JSON RECEIPT_JSON"
                .to_owned(),
        );
    }
    let (input_bytes, input) = read_json(&args[0])?;
    let (schema_bytes, schema) = read_json(&args[1])?;
    let (oracle_bytes, oracle) = read_json(&args[2])?;
    let evaluation = evaluate_all(&input, &schema, &oracle)?;
    let success = evaluation["case_count"].as_u64() == Some(CASE_COUNT as u64)
        && evaluation["matched_case_count"].as_u64() == Some(CASE_COUNT as u64)
        && evaluation["mismatch_count"].as_u64() == Some(0);
    let revision = env::var("WDN_VERIFIED_INTERVENTION_YIELD_T0_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok()
        .filter(|value| valid_revision(value));
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let lock_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let manifest_bytes = fs::read(&manifest_path)
        .map_err(|error| format!("read {}: {error}", manifest_path.display()))?;
    let lock_bytes = fs::read(&lock_path)
        .map_err(|error| format!("read {}: {error}", lock_path.display()))?;
    let implementation = include_bytes!("wdn-verified-intervention-yield-t0.rs");
    let receipt = json!({
        "receipt_schema_version": "wdn-verified-intervention-yield-t0-receipt-v0.1",
        "benchmark_id": BENCHMARK_ID,
        "verification_state": if success && revision.is_some() {
            "PassedExactRevision"
        } else if success {
            "PassedLocalUnpinned"
        } else {
            "OracleMismatch"
        },
        "implementation_revision": revision,
        "input_sha256": sha256(&input_bytes),
        "input_schema_sha256": sha256(&schema_bytes),
        "oracle_sha256": sha256(&oracle_bytes),
        "package_manifest_sha256": sha256(&manifest_bytes),
        "cargo_lock_sha256": sha256(&lock_bytes),
        "implementation_sha256": sha256(implementation),
        "case_count": evaluation["case_count"],
        "matched_case_count": evaluation["matched_case_count"],
        "mismatch_count": evaluation["mismatch_count"],
        "mismatches": evaluation["mismatches"],
        "causal_attribution_status": "NotRun",
        "annualized_savings_claim": false,
        "claim_eligibility": "NotEligible",
        "claim_ceiling": "synthetic_inspection_repair_outcome_contract_only"
    });
    let output = Path::new(&args[3]);
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| format!("create receipt directory: {error}"))?;
    }
    fs::write(
        output,
        serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("serialize receipt: {error}"))?,
    )
    .map_err(|error| format!("write receipt '{}': {error}", output.display()))?;
    println!("{}", serde_json::to_string_pretty(&receipt).unwrap_or_default());
    if success {
        Ok(())
    } else {
        Err("verified intervention yield did not match frozen oracle".to_owned())
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Err(error) = run(&args) {
        eprintln!("wdn-verified-intervention-yield-t0: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Value, Value, Value) {
        let input = serde_json::from_slice(include_bytes!(
            "../../../../docs/research/water-intervention/wdn_verified_intervention_yield_input.v0.1.json"
        ))
        .unwrap();
        let schema = serde_json::from_slice(include_bytes!(
            "../../../../docs/research/water-intervention/wdn_verified_intervention_yield_input.schema.v0.1.json"
        ))
        .unwrap();
        let oracle = serde_json::from_slice(include_bytes!(
            "../../../../docs/research/water-intervention/wdn_verified_intervention_yield_oracle.v0.1.json"
        ))
        .unwrap();
        (input, schema, oracle)
    }

    #[test]
    fn canonical_fourteen_case_oracle_matches() {
        let (input, schema, oracle) = fixture();
        let result = evaluate_all(&input, &schema, &oracle).unwrap();
        assert_eq!(result["case_count"], 14);
        assert_eq!(result["matched_case_count"], 14);
        assert_eq!(result["mismatch_count"], 0);
        assert_eq!(result["causal_attribution_status"], "NotRun");
        assert_eq!(result["annualized_savings_claim"], false);
        assert_eq!(result["claim_eligibility"], "NotEligible");
    }

    #[test]
    fn positive_case_requires_independent_confirmation_repair_and_verification() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][0], &BTreeSet::new()).unwrap();
        assert_eq!(output["verified_repair_count"], 1);
        assert_eq!(output["observed_flow_reduction_lps"], 8.0);
        assert_eq!(output["lower_bound_flow_reduction_lps"], 6.8);
    }

    #[test]
    fn confirmed_no_leak_is_a_false_dispatch_not_a_localization_error() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][1], &BTreeSet::new()).unwrap();
        assert_eq!(output["false_dispatch_count"], 1);
        assert_eq!(output["localization_asset_mismatch_count"], 0);
        assert_eq!(output["verified_repair_count"], 0);
    }

    #[test]
    fn alert_wrong_asset_is_not_mistaken_for_correct_localization() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][2], &BTreeSet::new()).unwrap();
        assert_eq!(output["localization_asset_mismatch_count"], 1);
        assert_eq!(output["verified_repair_count"], 1);
    }

    #[test]
    fn work_order_without_completed_repair_is_not_a_repair_success() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][3], &BTreeSet::new()).unwrap();
        assert_eq!(output["event_dispositions"][0]["disposition"], "repair_not_completed");
        assert_eq!(output["verified_repair_count"], 0);
    }

    #[test]
    fn closed_work_order_without_independent_verification_is_not_verified() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][4], &BTreeSet::new()).unwrap();
        assert_eq!(
            output["event_dispositions"][0]["disposition"],
            "post_repair_verification_missing_or_not_independent"
        );
    }

    #[test]
    fn repair_asset_must_match_independent_inspection_asset() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][5], &BTreeSet::new()).unwrap();
        assert_eq!(output["event_dispositions"][0]["disposition"], "repair_asset_mismatch");
    }

    #[test]
    fn pressure_mismatch_blocks_quantitative_change_claim() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][6], &BTreeSet::new()).unwrap();
        assert_eq!(
            output["event_dispositions"][0]["disposition"],
            "hydraulic_comparison_pressure_mismatch"
        );
    }

    #[test]
    fn concurrent_valve_change_blocks_attribution_comparison() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][7], &BTreeSet::new()).unwrap();
        assert_eq!(
            output["event_dispositions"][0]["disposition"],
            "hydraulic_comparison_confounded"
        );
    }

    #[test]
    fn stale_calibration_is_not_admitted() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][8], &BTreeSet::new()).unwrap();
        assert_eq!(
            output["event_dispositions"][0]["disposition"],
            "invalid_or_stale_measurement_evidence"
        );
    }

    #[test]
    fn repeated_raw_record_digest_rejects_every_affected_event() {
        let (input, _, _) = fixture();
        let duplicates = duplicate_raw_event_ids(&input).unwrap();
        assert_eq!(duplicates.len(), 2);
        let output = evaluate_case(&input["cases"][9], &duplicates).unwrap();
        assert_eq!(output["event_dispositions"][0]["disposition"], "duplicate_raw_record_replay");
        assert_eq!(output["event_dispositions"][1]["disposition"], "duplicate_raw_record_replay");
    }

    #[test]
    fn recurrence_overrides_an_immediate_post_repair_pass() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][10], &BTreeSet::new()).unwrap();
        assert_eq!(output["event_dispositions"][0]["disposition"], "recurrence_observed");
        assert_eq!(output["recurrence_count"], 1);
    }

    #[test]
    fn unresolved_inspection_cannot_become_confirmed_leak() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][11], &BTreeSet::new()).unwrap();
        assert_eq!(output["independent_confirmed_leak_count"], 0);
        assert_eq!(
            output["event_dispositions"][0]["disposition"],
            "indeterminate_inspection_evidence"
        );
    }

    #[test]
    fn reduction_inside_uncertainty_has_zero_lower_bound() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][12], &BTreeSet::new()).unwrap();
        assert_eq!(
            output["event_dispositions"][0]["disposition"],
            "verified_but_flow_reduction_unresolved"
        );
        assert_eq!(output["lower_bound_flow_reduction_lps"], 0.0);
    }

    #[test]
    fn service_floor_violation_is_not_hidden_by_verified_repair() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][13], &BTreeSet::new()).unwrap();
        assert_eq!(
            output["event_dispositions"][0]["disposition"],
            "verified_repair_service_floor_violation"
        );
        assert_eq!(output["service_floor_violation_count"], 1);
        assert_eq!(output["verified_repair_count"], 1);
        assert_eq!(output["observed_flow_reduction_lps"], 8.0);
    }

    #[test]
    fn a_tampered_oracle_reports_one_mismatch() {
        let (input, schema, mut oracle) = fixture();
        oracle["cases"][0]["verified_repair_count"] = json!(99);
        let result = evaluate_all(&input, &schema, &oracle).unwrap();
        assert_eq!(result["matched_case_count"], 13);
        assert_eq!(result["mismatch_count"], 1);
    }

    #[test]
    fn duplicate_event_ids_are_rejected_within_a_case() {
        let (mut input, _, _) = fixture();
        input["cases"][9]["events"][1]["event_id"] = input["cases"][9]["events"][0]["event_id"].clone();
        assert!(evaluate_case(&input["cases"][9], &BTreeSet::new()).is_err());
    }
}
