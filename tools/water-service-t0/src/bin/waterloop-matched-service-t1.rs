// SPDX-License-Identifier: AGPL-3.0-or-later
//! Synthetic-first paired WaterLoop service-credit evaluator.
//! This binary does not ingest or qualify physical water-quality evidence.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::Path;

const INPUT_VERSION: &str = "waterloop-matched-service-t1-input-v0.1";
const ORACLE_VERSION: &str = "waterloop-matched-service-t1-oracle-v0.1";
const BENCHMARK_ID: &str = "WATERLOOP-MATCHED-SERVICE-T1-EXECUTOR-001";
const EXPECTED_PLAN_BLOB_SHA: &str = "3b64690fa02ac47b3bc430a22b1392c65583bba7";
const CASE_COUNT: usize = 19;
const MINIMUM_PAIRED_BLOCKS: usize = 6;

#[derive(Clone, Debug)]
struct Measurement {
    channel_id: String,
    instrument_id: String,
    unit: String,
    boundary: String,
    value: f64,
    standard_uncertainty: f64,
    calibration_ref: String,
    calibration_valid_from_utc: String,
    calibration_valid_until_utc: String,
    method_ref: String,
    raw_record_digest: String,
}

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

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn parse_canonical_utc_seconds(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
        || !bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| [4, 7, 10, 13, 16, 19].contains(&index) || byte.is_ascii_digit())
    {
        return None;
    }
    let year = value[0..4].parse::<i64>().ok()?;
    let month = value[5..7].parse::<i64>().ok()?;
    let day = value[8..10].parse::<i64>().ok()?;
    let hour = value[11..13].parse::<i64>().ok()?;
    let minute = value[14..16].parse::<i64>().ok()?;
    let second = value[17..19].parse::<i64>().ok()?;
    if !(1..=12).contains(&month) || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute) || !(0..=59).contains(&second)
    {
        return None;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if day < 1 || day > days_in_month {
        return None;
    }
    let adjusted_year = year - i64::from(month <= 2);
    let era = if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    } / 400;
    let year_of_era = adjusted_year - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era;
    Some(days * 86_400 + hour * 3_600 + minute * 60 + second)
}

fn valid_canonical_utc(value: &str) -> bool {
    parse_canonical_utc_seconds(value).is_some()
}

fn parse_measurement(
    value: &Value,
    context: &str,
    run_start_utc: &str,
    run_end_utc: &str,
) -> Result<Measurement, String> {
    ensure_keys(
        value,
        &[
            "channel_id",
            "instrument_id",
            "unit",
            "boundary",
            "value",
            "standard_uncertainty",
            "calibration_ref",
            "calibration_valid_from_utc",
            "calibration_valid_until_utc",
            "method_ref",
            "raw_record_digest",
            "validity",
            "time_window_start_utc",
            "time_window_end_utc",
        ],
        context,
    )?;
    let channel_id = string(value, "channel_id", context)?.to_owned();
    let instrument_id = string(value, "instrument_id", context)?.to_owned();
    let unit = string(value, "unit", context)?.to_owned();
    let boundary = string(value, "boundary", context)?.to_owned();
    let measurement_value = number(value, "value", context)?;
    let standard_uncertainty = number(value, "standard_uncertainty", context)?;
    let calibration_ref = string(value, "calibration_ref", context)?.to_owned();
    let calibration_valid_from_utc =
        string(value, "calibration_valid_from_utc", context)?.to_owned();
    let calibration_valid_until_utc =
        string(value, "calibration_valid_until_utc", context)?.to_owned();
    let method_ref = string(value, "method_ref", context)?.to_owned();
    let raw_record_digest = string(value, "raw_record_digest", context)?.to_owned();
    let validity = string(value, "validity", context)?;
    let time_window_start_utc = string(value, "time_window_start_utc", context)?.to_owned();
    let time_window_end_utc = string(value, "time_window_end_utc", context)?.to_owned();

    if channel_id.trim().is_empty()
        || instrument_id.trim().is_empty()
        || calibration_ref.trim().is_empty()
        || method_ref.trim().is_empty()
    {
        return Err(format!(
            "{context}: channel/instrument/calibration/method identity is empty"
        ));
    }
    if measurement_value < 0.0 || standard_uncertainty < 0.0 {
        return Err(format!(
            "{context}: negative value or uncertainty is invalid"
        ));
    }
    if validity != "valid" {
        return Err(format!(
            "{context}: measurement validity is '{validity}', not 'valid'"
        ));
    }
    if !valid_digest(&raw_record_digest) {
        return Err(format!(
            "{context}: raw_record_digest must be 64 hexadecimal characters"
        ));
    }
    if !valid_canonical_utc(&time_window_start_utc)
        || !valid_canonical_utc(&time_window_end_utc)
        || time_window_start_utc >= time_window_end_utc
        || time_window_start_utc != run_start_utc
        || time_window_end_utc != run_end_utc
    {
        return Err(format!(
            "{context}: measurement time window must be valid and equal its run window"
        ));
    }
    if !valid_canonical_utc(&calibration_valid_from_utc)
        || !valid_canonical_utc(&calibration_valid_until_utc)
        || calibration_valid_from_utc > run_start_utc
        || run_end_utc > calibration_valid_until_utc
        || calibration_valid_from_utc >= calibration_valid_until_utc
    {
        return Err(format!(
            "{context}: calibration validity window does not cover the complete run"
        ));
    }
    Ok(Measurement {
        channel_id,
        instrument_id,
        unit,
        boundary,
        value: measurement_value,
        standard_uncertainty,
        calibration_ref,
        calibration_valid_from_utc,
        calibration_valid_until_utc,
        method_ref,
        raw_record_digest,
    })
}

fn expect_measurement<'a>(
    measurements: &'a BTreeMap<String, Measurement>,
    key: &str,
    unit: &str,
    context: &str,
) -> Result<&'a Measurement, String> {
    let measurement = measurements
        .get(key)
        .ok_or_else(|| format!("{context}: missing parsed measurement '{key}'"))?;
    if measurement.unit != unit {
        return Err(format!(
            "{context}: {key} unit '{}' != '{unit}'",
            measurement.unit
        ));
    }
    Ok(measurement)
}

fn empty_block(
    block_id: &str,
    disposition: &str,
    control_energy_kwh: Option<f64>,
    treatment_energy_kwh: Option<f64>,
) -> Value {
    let net_energy_delta_kwh = match (control_energy_kwh, treatment_energy_kwh) {
        (Some(control), Some(treatment)) => Some(treatment - control),
        _ => None,
    };
    json!({
        "block_id": block_id,
        "disposition": disposition,
        "point_credit_l": 0.0,
        "uncertainty_screened_lower_credit_l": 0.0,
        "matched_control_mains_normalized_l": null,
        "matched_control_displacement_l": null,
        "lower_displacement_l": null,
        "control_energy_kwh": control_energy_kwh,
        "energy_kwh": treatment_energy_kwh,
        "net_energy_delta_kwh": net_energy_delta_kwh,
        "energy_per_1000l_lower_credit": null,
        "net_energy_per_1000l_lower_credit": null
    })
}

fn read_energy(run: Option<&Value>, context: &str) -> Option<f64> {
    let run = run?;
    let start = run.get("start_utc")?.as_str()?;
    let end = run.get("end_utc")?.as_str()?;
    let raw = run.get("energy")?;
    parse_measurement(raw, context, start, end)
        .ok()
        .filter(|measurement| {
            measurement.unit == "kWh"
                && measurement.boundary == "whole_treatment_pump_train_electrical_boundary"
        })
        .map(|measurement| measurement.value)
}

fn empty_block_from_input(block: &Value, block_id: &str, disposition: &str) -> Value {
    let control_energy = read_energy(
        block.get("control").filter(|value| !value.is_null()),
        "control.energy",
    );
    let treatment_energy = read_energy(
        block.get("treatment").filter(|value| !value.is_null()),
        "treatment.energy",
    );
    empty_block(block_id, disposition, control_energy, treatment_energy)
}

fn count_duplicate_raw_record_blocks(blocks: &[Value]) -> BTreeSet<String> {
    let measurement_fields = [
        ("control", "mains_fixture"),
        ("control", "energy"),
        ("treatment", "quality_qualified_delivery"),
        ("treatment", "recovered_fixture_use"),
        ("treatment", "mains_fallback"),
        ("treatment", "gross_product"),
        ("treatment", "reject"),
        ("treatment", "purge"),
        ("treatment", "overflow"),
        ("treatment", "energy"),
    ];
    let mut digest_blocks = BTreeMap::<String, BTreeSet<String>>::new();
    for block in blocks {
        let Some(block_id) = block.get("block_id").and_then(Value::as_str) else {
            continue;
        };
        for (arm, field_name) in measurement_fields {
            if let Some(digest) = block
                .get(arm)
                .filter(|value| !value.is_null())
                .and_then(|run| run.get(field_name))
                .and_then(|measurement| measurement.get("raw_record_digest"))
                .and_then(Value::as_str)
            {
                digest_blocks
                    .entry(digest.to_owned())
                    .or_default()
                    .insert(block_id.to_owned());
            }
        }
    }
    let duplicate_digests: BTreeSet<String> = digest_blocks
        .into_iter()
        .filter_map(|(digest, block_ids)| (block_ids.len() > 1).then_some(digest))
        .collect();
    let mut affected_blocks = BTreeSet::new();
    for block in blocks {
        let Some(block_id) = block.get("block_id").and_then(Value::as_str) else {
            continue;
        };
        if measurement_fields.iter().any(|(arm, field_name)| {
            block
                .get(*arm)
                .filter(|value| !value.is_null())
                .and_then(|run| run.get(*field_name))
                .and_then(|measurement| measurement.get("raw_record_digest"))
                .and_then(Value::as_str)
                .is_some_and(|digest| duplicate_digests.contains(digest))
        }) {
            affected_blocks.insert(block_id.to_owned());
        }
    }
    affected_blocks
}

fn count_duplicate_allocations(blocks: &[Value]) -> Result<BTreeSet<String>, String> {
    let mut counts = BTreeMap::<String, usize>::new();
    for (index, block) in blocks.iter().enumerate() {
        let id = string(
            block,
            "recovered_allocation_id",
            &format!("blocks[{index}]"),
        )?;
        if id.trim().is_empty() {
            return Err(format!("blocks[{index}]: recovered_allocation_id is empty"));
        }
        *counts.entry(id.to_owned()).or_default() += 1;
    }
    Ok(counts
        .into_iter()
        .filter_map(|(id, count)| (count > 1).then_some(id))
        .collect())
}

fn eval_block(
    block: &Value,
    case: &Value,
    duplicate_allocations: &BTreeSet<String>,
    duplicate_raw_record_blocks: &BTreeSet<String>,
) -> Result<Value, String> {
    let block_id = string(block, "block_id", "block")?;
    let context = format!("block {block_id}");
    let allocation_id = string(block, "recovered_allocation_id", &context)?;
    if duplicate_allocations.contains(allocation_id) {
        return Ok(empty_block_from_input(
            block,
            block_id,
            "zero_credit_duplicate_recovered_allocation",
        ));
    }
    if duplicate_raw_record_blocks.contains(block_id) {
        return Ok(empty_block_from_input(
            block,
            block_id,
            "zero_credit_duplicate_raw_record_across_blocks",
        ));
    }

    let control_value = field(block, "control", &context)?;
    if control_value.is_null() {
        return Ok(empty_block_from_input(
            block,
            block_id,
            "indeterminate_missing_control",
        ));
    }
    let treatment_value = field(block, "treatment", &context)?;
    if treatment_value.is_null() {
        return Ok(empty_block_from_input(
            block,
            block_id,
            "indeterminate_missing_treatment",
        ));
    }
    let control = control_value;
    let treatment = treatment_value;

    if string(control, "arm_id", &context)? != "CONTROL-MAINS"
        || string(treatment, "arm_id", &context)? != "TREATMENT-RECOVERED"
    {
        return Ok(empty_block_from_input(
            block,
            block_id,
            "zero_credit_arm_identity",
        ));
    }
    let use_profile = string(case, "use_profile_id", "case")?;
    let source_profile = string(case, "source_profile_id", "case")?;
    if string(control, "use_profile_id", &context)? != use_profile
        || string(treatment, "use_profile_id", &context)? != use_profile
    {
        return Ok(empty_block_from_input(
            block,
            block_id,
            "zero_credit_use_profile_mismatch",
        ));
    }
    if string(treatment, "source_profile_id", &context)? != source_profile {
        return Ok(empty_block_from_input(
            block,
            block_id,
            "zero_credit_source_profile_mismatch",
        ));
    }

    let control_run = string(control, "run_id", &context)?;
    let treatment_run = string(treatment, "run_id", &context)?;
    if control_run.trim().is_empty()
        || treatment_run.trim().is_empty()
        || control_run == treatment_run
    {
        return Ok(empty_block_from_input(
            block,
            block_id,
            "zero_credit_arm_identity",
        ));
    }

    let control_digest = string(control, "demand_trace_digest", &context)?;
    let treatment_digest = string(treatment, "demand_trace_digest", &context)?;
    let block_digest = string(block, "demand_trace_digest", &context)?;
    if !valid_digest(control_digest)
        || !valid_digest(treatment_digest)
        || control_digest != treatment_digest
        || control_digest != block_digest
    {
        return Ok(empty_block_from_input(
            block,
            block_id,
            "indeterminate_demand_trace_mismatch",
        ));
    }

    let control_demand = number(control, "measured_demand_l", &context)?;
    let treatment_demand = number(treatment, "measured_demand_l", &context)?;
    if control_demand < 0.0 || treatment_demand < 0.0 {
        return Ok(empty_block_from_input(
            block,
            block_id,
            "indeterminate_demand_not_comparable",
        ));
    }
    let demand_denominator = control_demand.max(treatment_demand).max(1.0);
    let demand_difference_percent =
        (control_demand - treatment_demand).abs() / demand_denominator * 100.0;
    let maximum_difference = number(case, "max_demand_relative_difference_percent", "case")?;
    if demand_difference_percent > maximum_difference {
        return Ok(empty_block_from_input(
            block,
            block_id,
            "indeterminate_demand_not_comparable",
        ));
    }

    let control_start = string(control, "start_utc", &context)?;
    let control_end = string(control, "end_utc", &context)?;
    let treatment_start = string(treatment, "start_utc", &context)?;
    let treatment_end = string(treatment, "end_utc", &context)?;
    if ![control_start, control_end, treatment_start, treatment_end]
        .into_iter()
        .all(valid_canonical_utc)
    {
        return Ok(empty_block_from_input(
            block,
            block_id,
            "indeterminate_invalid_timebase",
        ));
    }
    if control_start >= control_end || treatment_start >= treatment_end {
        return Ok(empty_block_from_input(
            block,
            block_id,
            "indeterminate_invalid_timebase",
        ));
    }
    if !(control_end <= treatment_start || treatment_end <= control_start) {
        return Ok(empty_block_from_input(
            block,
            block_id,
            "indeterminate_overlapping_windows",
        ));
    }
    let control_duration = parse_canonical_utc_seconds(control_end).unwrap()
        - parse_canonical_utc_seconds(control_start).unwrap();
    let treatment_duration = parse_canonical_utc_seconds(treatment_end).unwrap()
        - parse_canonical_utc_seconds(treatment_start).unwrap();
    if control_duration != treatment_duration {
        return Ok(empty_block_from_input(
            block,
            block_id,
            "indeterminate_run_duration_mismatch",
        ));
    }

    let service = field(case, "service_envelope", "case")?;
    let min_pressure = number(service, "min_pressure_kpa", "service_envelope")?;
    let max_pressure = number(service, "max_pressure_kpa", "service_envelope")?;
    let min_flow = number(service, "min_flow_lpm", "service_envelope")?;
    let max_flow = number(service, "max_flow_lpm", "service_envelope")?;
    if min_pressure > max_pressure || min_flow > max_flow {
        return Err("case: invalid service envelope bounds".to_owned());
    }
    let mut service_ranges = Vec::with_capacity(2);
    for run in [control, treatment] {
        let pressure_min = number(run, "service_pressure_min_kpa", &context)?;
        let pressure_max = number(run, "service_pressure_max_kpa", &context)?;
        let flow_min = number(run, "service_flow_min_lpm", &context)?;
        let flow_max = number(run, "service_flow_max_lpm", &context)?;
        if pressure_min > pressure_max
            || flow_min > flow_max
            || pressure_min < min_pressure
            || pressure_max > max_pressure
            || flow_min < min_flow
            || flow_max > max_flow
        {
            return Ok(empty_block_from_input(
                block,
                block_id,
                "zero_credit_service_envelope_violation",
            ));
        }
        service_ranges.push((pressure_min, pressure_max, flow_min, flow_max));
    }
    let max_pressure_difference =
        number(case, "max_service_pressure_difference_kpa", "case")?;
    let max_flow_difference = number(case, "max_service_flow_difference_lpm", "case")?;
    if max_pressure_difference < 0.0 || max_flow_difference < 0.0 {
        return Err("case: service matching tolerances must be non-negative".to_owned());
    }
    let (control_pressure_min, control_pressure_max, control_flow_min, control_flow_max) =
        service_ranges[0];
    let (treatment_pressure_min, treatment_pressure_max, treatment_flow_min, treatment_flow_max) =
        service_ranges[1];
    if (control_pressure_min - treatment_pressure_min).abs() > max_pressure_difference
        || (control_pressure_max - treatment_pressure_max).abs() > max_pressure_difference
        || (control_flow_min - treatment_flow_min).abs() > max_flow_difference
        || (control_flow_max - treatment_flow_max).abs() > max_flow_difference
    {
        return Ok(empty_block_from_input(
            block,
            block_id,
            "indeterminate_service_envelope_mismatch",
        ));
    }

    let quality = field(treatment, "quality_release", &context)?;
    if quality.is_null()
        || string(quality, "status", &context)? != "current"
        || string(quality, "profile_id", &context)? != use_profile
        || string(quality, "source_profile_id", &context)? != source_profile
        || quality.get("independent_review").and_then(Value::as_bool) != Some(true)
        || quality
            .get("release_ref")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        || quality
            .get("method_revision_ref")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        || quality
            .get("independent_reviewer_ref")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        || quality
            .get("sample_set_digest")
            .and_then(Value::as_str)
            .is_none_or(|value| !valid_digest(value))
    {
        return Ok(empty_block_from_input(
            block,
            block_id,
            "zero_credit_quality_not_released_for_profile",
        ));
    }

    let treatment_start = string(treatment, "start_utc", &context)?;
    let treatment_end = string(treatment, "end_utc", &context)?;
    let quality_valid_from = string(quality, "valid_from_utc", &context)?;
    let quality_valid_until = string(quality, "valid_until_utc", &context)?;
    if !valid_canonical_utc(quality_valid_from)
        || !valid_canonical_utc(quality_valid_until)
        || quality_valid_from > treatment_start
        || treatment_end > quality_valid_until
        || quality_valid_from >= quality_valid_until
    {
        return Ok(empty_block_from_input(
            block,
            block_id,
            "zero_credit_quality_release_out_of_validity_window",
        ));
    }

    let raw_measurements = [
        ("control_mains_fixture", control, "mains_fixture"),
        ("control_energy", control, "energy"),
        (
            "quality_qualified_delivery",
            treatment,
            "quality_qualified_delivery",
        ),
        ("recovered_fixture_use", treatment, "recovered_fixture_use"),
        ("mains_fallback", treatment, "mains_fallback"),
        ("gross_product", treatment, "gross_product"),
        ("reject", treatment, "reject"),
        ("purge", treatment, "purge"),
        ("overflow", treatment, "overflow"),
        ("treatment_energy", treatment, "energy"),
    ];
    let mut measurements = BTreeMap::new();
    for (key, run, field_name) in raw_measurements {
        let raw = field(run, field_name, &context)?;
        let run_start = string(run, "start_utc", &context)?;
        let run_end = string(run, "end_utc", &context)?;
        let parsed =
            match parse_measurement(raw, &format!("{context}.{field_name}"), run_start, run_end) {
                Ok(measurement) => measurement,
                Err(error) => {
                    let disposition = if error.contains("calibration validity window") {
                        "indeterminate_measurement_calibration_out_of_validity_window"
                    } else {
                        "indeterminate_invalid_measurement_evidence"
                    };
                    return Ok(empty_block_from_input(block, block_id, disposition));
                }
            };
        measurements.insert(key.to_owned(), parsed);
    }

    let expected_units = [
        ("control_mains_fixture", "L"),
        ("control_energy", "kWh"),
        ("quality_qualified_delivery", "L"),
        ("recovered_fixture_use", "L"),
        ("mains_fallback", "L"),
        ("gross_product", "L"),
        ("reject", "L"),
        ("purge", "L"),
        ("overflow", "L"),
        ("treatment_energy", "kWh"),
    ];
    for (key, unit) in expected_units {
        if expect_measurement(&measurements, key, unit, &context).is_err() {
            return Ok(empty_block_from_input(
                block,
                block_id,
                "indeterminate_invalid_measurement_evidence",
            ));
        }
    }

    let expected_boundaries = [
        ("control_mains_fixture", "declared_fixture_inlet"),
        (
            "control_energy",
            "whole_treatment_pump_train_electrical_boundary",
        ),
        ("quality_qualified_delivery", "treatment_product_outlet"),
        ("recovered_fixture_use", "declared_fixture_inlet"),
        ("mains_fallback", "declared_fixture_inlet"),
        ("gross_product", "treatment_product_outlet"),
        ("reject", "treatment_reject_outlet"),
        ("purge", "purge_backwash_outlet"),
        ("overflow", "storage_overflow_drain"),
        (
            "treatment_energy",
            "whole_treatment_pump_train_electrical_boundary",
        ),
    ];
    if expected_boundaries.iter().any(|(key, boundary)| {
        measurements
            .get(*key)
            .is_none_or(|measurement| measurement.boundary != *boundary)
    }) {
        return Ok(empty_block_from_input(
            block,
            block_id,
            "zero_credit_service_boundary_mismatch",
        ));
    }

    let instrument_ids: BTreeSet<String> = measurements
        .values()
        .map(|measurement| measurement.instrument_id.clone())
        .collect();
    let channel_ids: BTreeSet<String> = measurements
        .values()
        .map(|measurement| measurement.channel_id.clone())
        .collect();
    let raw_digests: BTreeSet<String> = measurements
        .values()
        .map(|measurement| measurement.raw_record_digest.clone())
        .collect();
    if instrument_ids.len() != measurements.len()
        || channel_ids.len() != measurements.len()
        || raw_digests.len() != measurements.len()
    {
        return Ok(empty_block_from_input(
            block,
            block_id,
            "zero_credit_duplicate_meter_or_record_identity",
        ));
    }

    let control_mains = measurements
        .get("control_mains_fixture")
        .expect("parsed above");
    let quality_delivery = measurements
        .get("quality_qualified_delivery")
        .expect("parsed above");
    let recovered_use = measurements
        .get("recovered_fixture_use")
        .expect("parsed above");
    let mains_fallback = measurements.get("mains_fallback").expect("parsed above");
    let treatment_energy = measurements.get("treatment_energy").expect("parsed above");

    if control_demand <= 0.0 {
        return Ok(empty_block_from_input(
            block,
            block_id,
            "indeterminate_demand_not_comparable",
        ));
    }
    // Normalize measured control mains volume to the treatment-arm fixture
    // demand before computing displacement; demand uncertainty is not modeled
    // in this synthetic contract and remains an explicit limitation.
    let matched_control_mains_normalized =
        control_mains.value * (treatment_demand / control_demand);
    let matched_control_displacement =
        (matched_control_mains_normalized - mains_fallback.value).max(0.0);
    let point_credit = quality_delivery
        .value
        .min(recovered_use.value)
        .min(matched_control_displacement);

    let expansion_factor = number(case, "uncertainty_expansion_factor", "case")?;
    if expansion_factor <= 0.0 {
        return Err("case: uncertainty_expansion_factor must be positive".to_owned());
    }
    let quality_lower = (quality_delivery.value
        - expansion_factor * quality_delivery.standard_uncertainty)
        .max(0.0);
    let use_lower =
        (recovered_use.value - expansion_factor * recovered_use.standard_uncertainty).max(0.0);
    // Conservative bound for a difference: add standard uncertainties; do not
    // assume independent calibration errors or a covariance model.
    let demand_ratio = treatment_demand / control_demand;
    let displacement_uncertainty =
        control_mains.standard_uncertainty * demand_ratio + mains_fallback.standard_uncertainty;
    let displacement_lower =
        (matched_control_displacement - expansion_factor * displacement_uncertainty).max(0.0);
    let lower_credit = quality_lower.min(use_lower).min(displacement_lower);

    let disposition = if recovered_use.value == 0.0 {
        "zero_credit_no_recovered_fixture_use"
    } else if matched_control_displacement == 0.0 {
        "zero_credit_no_net_mains_displacement"
    } else if lower_credit == 0.0 && point_credit > 0.0 {
        "zero_credit_uncertainty_dominates_apparent_credit"
    } else {
        "credited"
    };
    let control_energy = measurements
        .get("control_energy")
        .expect("parsed above")
        .value;
    let treatment_energy = treatment_energy.value;
    let net_energy_delta = treatment_energy - control_energy;
    Ok(json!({
        "block_id": block_id,
        "disposition": disposition,
        "point_credit_l": point_credit,
        "uncertainty_screened_lower_credit_l": lower_credit,
        "matched_control_mains_normalized_l": matched_control_mains_normalized,
        "matched_control_displacement_l": matched_control_displacement,
        "lower_displacement_l": displacement_lower,
        "control_energy_kwh": control_energy,
        "energy_kwh": treatment_energy,
        "net_energy_delta_kwh": net_energy_delta,
        "energy_per_1000l_lower_credit": if lower_credit > 0.0 {
            Some(treatment_energy / lower_credit * 1000.0)
        } else {
            None::<f64>
        },
        "net_energy_per_1000l_lower_credit": if lower_credit > 0.0 {
            Some(net_energy_delta / lower_credit * 1000.0)
        } else {
            None::<f64>
        }
    }))
}

fn evaluate_case(case: &Value) -> Result<Value, String> {
    ensure_keys(
        case,
        &[
            "case_id",
            "use_profile_id",
            "source_profile_id",
            "max_demand_relative_difference_percent",
            "max_service_pressure_difference_kpa",
            "max_service_flow_difference_lpm",
            "service_envelope",
            "uncertainty_expansion_factor",
            "blocks",
        ],
        "case",
    )?;
    let case_id = string(case, "case_id", "case")?;
    let blocks = field(case, "blocks", "case")?
        .as_array()
        .ok_or_else(|| format!("{case_id}: blocks must be an array"))?;
    if blocks.len() < MINIMUM_PAIRED_BLOCKS || blocks.len() > 12 {
        return Err(format!(
            "{case_id}: paired block count must be {MINIMUM_PAIRED_BLOCKS}..12"
        ));
    }
    let duplicate_allocations = count_duplicate_allocations(blocks)?;
    let duplicate_raw_record_blocks = count_duplicate_raw_record_blocks(blocks);
    let mut block_ids = BTreeSet::new();
    let mut run_ids = BTreeSet::new();
    let mut outputs = Vec::with_capacity(blocks.len());
    for block in blocks {
        let block_id = string(block, "block_id", "block")?;
        if !block_ids.insert(block_id.to_owned()) {
            return Err(format!("{case_id}: duplicate block_id '{block_id}'"));
        }
        for arm in ["control", "treatment"] {
            if let Some(run_id) = block
                .get(arm)
                .filter(|value| !value.is_null())
                .and_then(|value| value.get("run_id"))
                .and_then(Value::as_str)
            {
                if !run_ids.insert(run_id.to_owned()) {
                    return Err(format!("{case_id}: duplicate run_id '{run_id}'"));
                }
            }
        }
        outputs.push(eval_block(
            block,
            case,
            &duplicate_allocations,
            &duplicate_raw_record_blocks,
        )?);
    }

    let point_credit: f64 = outputs
        .iter()
        .map(|block| block["point_credit_l"].as_f64().unwrap_or(0.0))
        .sum();
    let lower_credit: f64 = outputs
        .iter()
        .map(|block| {
            block["uncertainty_screened_lower_credit_l"]
                .as_f64()
                .unwrap_or(0.0)
        })
        .sum();
    let energy: f64 = outputs
        .iter()
        .map(|block| block["energy_kwh"].as_f64().unwrap_or(0.0))
        .sum();
    let energy_observed_block_count = outputs
        .iter()
        .filter(|block| block["energy_kwh"].is_number())
        .count();
    let control_energy: f64 = outputs
        .iter()
        .map(|block| block["control_energy_kwh"].as_f64().unwrap_or(0.0))
        .sum();
    let control_energy_observed_block_count = outputs
        .iter()
        .filter(|block| block["control_energy_kwh"].is_number())
        .count();
    let net_energy_delta: f64 = outputs
        .iter()
        .map(|block| block["net_energy_delta_kwh"].as_f64().unwrap_or(0.0))
        .sum();
    let net_energy_observed_block_count = outputs
        .iter()
        .filter(|block| block["net_energy_delta_kwh"].is_number())
        .count();
    let credited_block_count = outputs
        .iter()
        .filter(|block| block["disposition"] == "credited")
        .count();
    let mut disposition_counts = BTreeMap::<String, usize>::new();
    for block in &outputs {
        let disposition = block["disposition"]
            .as_str()
            .ok_or_else(|| format!("{case_id}: output disposition missing"))?;
        *disposition_counts
            .entry(disposition.to_owned())
            .or_default() += 1;
    }
    let energy_intensity = (lower_credit > 0.0).then_some(energy / lower_credit * 1000.0);

    Ok(json!({
        "case_id": case_id,
        "paired_block_count": outputs.len(),
        "credited_block_count": credited_block_count,
        "point_credit_l": point_credit,
        "uncertainty_screened_lower_credit_l": lower_credit,
        "energy_kwh": energy,
        "energy_observed_block_count": energy_observed_block_count,
        "control_energy_kwh": control_energy,
        "control_energy_observed_block_count": control_energy_observed_block_count,
        "net_energy_delta_kwh": net_energy_delta,
        "net_energy_observed_block_count": net_energy_observed_block_count,
        "energy_kwh_per_1000l_lower_credit": energy_intensity,
        "net_energy_kwh_per_1000l_lower_credit": (lower_credit > 0.0).then_some(net_energy_delta / lower_credit * 1000.0),
        "block_dispositions": outputs,
        "disposition_counts": disposition_counts,
        "physical_qualification_status": "NotRun",
        "statistical_inference_status": "NotRun",
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
    let schema_properties = field(input_schema, "properties", "input schema")?;
    if string(
        field(
            schema_properties,
            "schema_version",
            "input schema properties",
        )?,
        "const",
        "input schema properties.schema_version",
    )? != INPUT_VERSION
    {
        return Err("input schema version constant mismatch".to_owned());
    }
    let plan_blob = string(input, "frozen_plan_blob_sha", "input")?;
    if plan_blob != EXPECTED_PLAN_BLOB_SHA {
        return Err("input frozen plan blob differs from the reviewed design-only plan".to_owned());
    }
    let minimum_blocks = field(input, "minimum_paired_blocks", "input")?
        .as_u64()
        .ok_or_else(|| "minimum_paired_blocks must be an integer".to_owned())?;
    if minimum_blocks != MINIMUM_PAIRED_BLOCKS as u64 {
        return Err("v0.1 requires six paired blocks".to_owned());
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
        return Err(format!(
            "exactly {CASE_COUNT} input and oracle cases are required"
        ));
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
        let actual = evaluate_case(case)?;
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
        "schema_version": "waterloop-matched-service-t1-evaluation-v0.1",
        "benchmark_id": BENCHMARK_ID,
        "case_count": CASE_COUNT,
        "matched_case_count": CASE_COUNT - mismatch_count,
        "mismatch_count": mismatch_count,
        "mismatches": mismatches,
        "case_results": results,
        "physical_qualification_status": "NotRun",
        "statistical_inference_status": "NotRun",
        "claim_eligibility": "NotEligible",
        "claim_ceiling": "synthetic_matched_service_evidence_contract_only"
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
            "usage: waterloop-matched-service-t1 INPUT_JSON INPUT_SCHEMA ORACLE_JSON RECEIPT_JSON"
                .to_owned(),
        );
    }
    let (input_bytes, input) = read_json(&args[0])?;
    let (schema_bytes, schema) = read_json(&args[1])?;
    let (oracle_bytes, oracle) = read_json(&args[2])?;
    let plan_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/water-service-engineering/waterloop-matched-service-plan.v0.1.json");
    let plan_bytes = fs::read(&plan_path)
        .map_err(|error| format!("read frozen design plan '{}': {error}", plan_path.display()))?;
    let result = evaluate_all(&input, &schema, &oracle)?;
    let success = result["case_count"].as_u64() == Some(CASE_COUNT as u64)
        && result["matched_case_count"].as_u64() == Some(CASE_COUNT as u64)
        && result["mismatch_count"].as_u64() == Some(0);

    let revision = env::var("WATERLOOP_MATCHED_SERVICE_T1_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok()
        .filter(|value| valid_revision(value));
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let lock_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let manifest_bytes = fs::read(&manifest_path)
        .map_err(|error| format!("read {}: {error}", manifest_path.display()))?;
    let lock_bytes =
        fs::read(&lock_path).map_err(|error| format!("read {}: {error}", lock_path.display()))?;
    let implementation = include_bytes!("waterloop-matched-service-t1.rs");

    let receipt = json!({
        "receipt_schema_version": "waterloop-matched-service-t1-receipt-v0.1",
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
        "design_plan_sha256": sha256(&plan_bytes),
        "design_plan_blob_sha": EXPECTED_PLAN_BLOB_SHA,
        "package_manifest_sha256": sha256(&manifest_bytes),
        "cargo_lock_sha256": sha256(&lock_bytes),
        "implementation_sha256": sha256(implementation),
        "case_count": result["case_count"],
        "matched_case_count": result["matched_case_count"],
        "mismatch_count": result["mismatch_count"],
        "mismatches": result["mismatches"],
        "claim_ceiling": "synthetic_matched_service_evidence_contract_only"
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
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt).unwrap_or_default()
    );
    if success {
        Ok(())
    } else {
        Err("matched-service evaluator did not match the frozen oracle".to_owned())
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Err(error) = run(&args) {
        eprintln!("waterloop-matched-service-t1: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Value, Value, Value) {
        let input = serde_json::from_slice(include_bytes!(
            "../../../../research/water-service-engineering/waterloop-matched-service-t1-input.v0.1.json"
        ))
        .unwrap();
        let schema = serde_json::from_slice(include_bytes!(
            "../../../../research/water-service-engineering/waterloop-matched-service-t1-input.schema.v0.1.json"
        ))
        .unwrap();
        let oracle = serde_json::from_slice(include_bytes!(
            "../../../../research/water-service-engineering/waterloop-matched-service-t1-oracle.v0.1.json"
        ))
        .unwrap();
        (input, schema, oracle)
    }

    fn result_case<'a>(result: &'a Value, id: &str) -> &'a Value {
        result["case_results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["case_id"] == id)
            .unwrap()
    }

    #[test]
    fn canonical_nineteen_case_oracle_matches() {
        let (input, schema, oracle) = fixture();
        let result = evaluate_all(&input, &schema, &oracle).unwrap();
        assert_eq!(result["case_count"], 19);
        assert_eq!(result["matched_case_count"], 19);
        assert_eq!(result["mismatch_count"], 0);
        assert_eq!(result["physical_qualification_status"], "NotRun");
        assert_eq!(result["statistical_inference_status"], "NotRun");
        assert_eq!(result["claim_eligibility"], "NotEligible");
    }

    #[test]
    fn valid_paired_block_caps_credit_at_eighty_litres_and_screens_uncertainty() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][0]).unwrap();
        assert_eq!(output["point_credit_l"], 480.0);
        assert_eq!(output["uncertainty_screened_lower_credit_l"], 462.0);
        assert_eq!(output["credited_block_count"], 6);
        assert_eq!(output["control_energy_kwh"], 0.06);
        assert_eq!(output["energy_kwh"], 0.15);
        assert_eq!(output["net_energy_delta_kwh"], 0.09);
        assert_eq!(output["energy_observed_block_count"], 6);
        assert_eq!(output["net_energy_observed_block_count"], 6);
    }

    #[test]
    fn gross_output_cannot_replace_actual_fixture_use() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][1]).unwrap();
        assert_eq!(output["block_dispositions"][0]["point_credit_l"], 0.0);
        assert_eq!(
            output["block_dispositions"][0]["disposition"],
            "zero_credit_no_recovered_fixture_use"
        );
        assert_eq!(output["block_dispositions"][0]["energy_kwh"], 0.025);
        assert_eq!(output["energy_observed_block_count"], 6);
    }

    #[test]
    fn use_profile_mismatch_and_stale_quality_release_both_fail_closed() {
        let (input, _, _) = fixture();
        for index in [2, 3] {
            let output = evaluate_case(&input["cases"][index]).unwrap();
            assert_eq!(
                output["block_dispositions"][0]["disposition"],
                "zero_credit_quality_not_released_for_profile"
            );
            assert_eq!(output["block_dispositions"][0]["point_credit_l"], 0.0);
        }
    }

    #[test]
    fn missing_control_is_indeterminate_and_receives_no_credit() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][4]).unwrap();
        assert_eq!(
            output["block_dispositions"][0]["disposition"],
            "indeterminate_missing_control"
        );
        assert_eq!(output["block_dispositions"][0]["point_credit_l"], 0.0);
    }

    #[test]
    fn fallback_mains_reduces_displacement_credit() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][5]).unwrap();
        assert_eq!(
            output["block_dispositions"][0]["matched_control_mains_normalized_l"],
            100.0
        );
        assert_eq!(
            output["block_dispositions"][0]["matched_control_displacement_l"],
            60.0
        );
        assert_eq!(output["block_dispositions"][0]["point_credit_l"], 60.0);
    }

    #[test]
    fn duplicate_instrument_identity_is_rejected() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][6]).unwrap();
        assert_eq!(
            output["block_dispositions"][0]["disposition"],
            "zero_credit_duplicate_meter_or_record_identity"
        );
    }

    #[test]
    fn every_block_reusing_a_recovered_allocation_is_rejected() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][7]).unwrap();
        assert_eq!(
            output["block_dispositions"][0]["disposition"],
            "zero_credit_duplicate_recovered_allocation"
        );
        assert_eq!(
            output["block_dispositions"][1]["disposition"],
            "zero_credit_duplicate_recovered_allocation"
        );
        assert_eq!(output["block_dispositions"][0]["point_credit_l"], 0.0);
        assert_eq!(output["block_dispositions"][1]["point_credit_l"], 0.0);
    }

    #[test]
    fn reject_stream_cannot_be_relabelled_as_fixture_delivery() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][8]).unwrap();
        assert_eq!(
            output["block_dispositions"][0]["disposition"],
            "zero_credit_service_boundary_mismatch"
        );
    }

    #[test]
    fn service_pressure_or_flow_outside_envelope_receives_no_credit() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][9]).unwrap();
        assert_eq!(
            output["block_dispositions"][0]["disposition"],
            "zero_credit_service_envelope_violation"
        );
    }

    #[test]
    fn overlapping_control_and_treatment_windows_are_indeterminate() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][10]).unwrap();
        assert_eq!(
            output["block_dispositions"][0]["disposition"],
            "indeterminate_overlapping_windows"
        );
    }

    #[test]
    fn mismatched_demand_is_not_a_valid_matched_control() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][11]).unwrap();
        assert_eq!(
            output["block_dispositions"][0]["disposition"],
            "indeterminate_demand_not_comparable"
        );
    }

    #[test]
    fn uncertainty_that_consumes_apparent_credit_yields_zero_lower_credit() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][12]).unwrap();
        let block = &output["block_dispositions"][0];
        assert_eq!(block["point_credit_l"], 80.0);
        assert_eq!(block["uncertainty_screened_lower_credit_l"], 0.0);
        assert_eq!(
            block["disposition"],
            "zero_credit_uncertainty_dominates_apparent_credit"
        );
        assert!(block["energy_per_1000l_lower_credit"].is_null());
    }

    #[test]
    fn control_mains_volume_is_normalized_to_treatment_demand() {
        let (mut input, _, _) = fixture();
        input["cases"][0]["blocks"][0]["treatment"]["measured_demand_l"] = json!(101.0);
        let output = evaluate_case(&input["cases"][0]).unwrap();
        assert_eq!(
            output["block_dispositions"][0]["matched_control_mains_normalized_l"],
            101.0
        );
        assert_eq!(
            output["block_dispositions"][0]["matched_control_displacement_l"],
            91.0
        );
    }

    #[test]
    fn rejected_water_credit_does_not_erase_valid_energy_observations() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][2]).unwrap();
        assert_eq!(output["block_dispositions"][0]["point_credit_l"], 0.0);
        assert_eq!(output["block_dispositions"][0]["energy_kwh"], 0.025);
        assert_eq!(output["block_dispositions"][0]["control_energy_kwh"], 0.01);
        assert_eq!(
            output["block_dispositions"][0]["net_energy_delta_kwh"],
            0.015
        );
    }

    #[test]
    fn raw_record_replayed_across_blocks_rejects_every_affected_block() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][13]).unwrap();
        assert_eq!(
            output["block_dispositions"][0]["disposition"],
            "zero_credit_duplicate_raw_record_across_blocks"
        );
        assert_eq!(
            output["block_dispositions"][1]["disposition"],
            "zero_credit_duplicate_raw_record_across_blocks"
        );
    }

    #[test]
    fn expired_quality_release_never_earns_service_credit() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][14]).unwrap();
        assert_eq!(
            output["block_dispositions"][0]["disposition"],
            "zero_credit_quality_release_out_of_validity_window"
        );
        assert_eq!(output["block_dispositions"][0]["point_credit_l"], 0.0);
    }

    #[test]
    fn expired_instrument_calibration_never_earns_service_credit() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][15]).unwrap();
        assert_eq!(
            output["block_dispositions"][0]["disposition"],
            "indeterminate_measurement_calibration_out_of_validity_window"
        );
        assert_eq!(output["block_dispositions"][0]["point_credit_l"], 0.0);
    }

    #[test]
    fn pressure_must_match_between_control_and_treatment_even_inside_limits() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][16]).unwrap();
        assert_eq!(
            output["block_dispositions"][0]["disposition"],
            "indeterminate_service_envelope_mismatch"
        );
        assert_eq!(output["block_dispositions"][0]["point_credit_l"], 0.0);
    }

    #[test]
    fn flow_must_match_between_control_and_treatment_even_inside_limits() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][17]).unwrap();
        assert_eq!(
            output["block_dispositions"][0]["disposition"],
            "indeterminate_service_envelope_mismatch"
        );
        assert_eq!(output["block_dispositions"][0]["point_credit_l"], 0.0);
    }

    #[test]
    fn paired_run_durations_must_match() {
        let (input, _, _) = fixture();
        let output = evaluate_case(&input["cases"][18]).unwrap();
        assert_eq!(
            output["block_dispositions"][0]["disposition"],
            "indeterminate_run_duration_mismatch"
        );
        assert_eq!(output["block_dispositions"][0]["point_credit_l"], 0.0);
    }

    #[test]
    fn tampered_oracle_is_reported_without_loosening_the_contract() {
        let (input, schema, mut oracle) = fixture();
        oracle["cases"][0]["point_credit_l"] = json!(999.0);
        let output = evaluate_all(&input, &schema, &oracle).unwrap();
        assert_eq!(output["matched_case_count"], 14);
        assert_eq!(output["mismatch_count"], 1);
    }

    #[test]
    fn duplicate_block_ids_are_rejected() {
        let (mut input, _, _) = fixture();
        input["cases"][0]["blocks"][1]["block_id"] = json!("B1");
        assert!(
            evaluate_case(&input["cases"][0])
                .unwrap_err()
                .contains("duplicate block_id")
        );
    }

    #[test]
    fn duplicate_run_ids_are_rejected() {
        let (mut input, _, _) = fixture();
        input["cases"][0]["blocks"][1]["control"]["run_id"] =
            input["cases"][0]["blocks"][0]["control"]["run_id"].clone();
        assert!(
            evaluate_case(&input["cases"][0])
                .unwrap_err()
                .contains("duplicate run_id")
        );
    }
}
