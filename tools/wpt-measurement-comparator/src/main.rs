// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

fn text<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str)
}

fn n(v: &Value, keys: &[&str]) -> Option<f64> {
    let mut x = v;
    for key in keys {
        x = x.get(*key)?;
    }
    x.as_f64().filter(|x| x.is_finite())
}

fn dimension_for_quantity(q: &str) -> Option<&'static str> {
    match q {
        "electrical_real_power" | "apparent_power" | "reactive_power" | "useful_load_power" => Some("power"),
        "delivered_energy" => Some("energy"),
        "energy_per_distance" => Some("energy_per_distance"),
        "end_to_end_efficiency" | "subsystem_efficiency"
        | "service_energy_replenishment_ratio" | "coupling_coefficient"
        | "quality_factor" | "power_factor" | "uptime_fraction" | "signal_to_noise_ratio"
        | "bit_error_rate" => Some("dimensionless"),
        "electric_field_strength" | "internal_electric_field" => Some("electric_field"),
        "magnetic_field_strength" => Some("magnetic_field"),
        "magnetic_flux_density" => Some("magnetic_flux_density"),
        "specific_absorption_rate" => Some("specific_absorption_rate"),
        "current_density" => Some("current_density"),
        "contact_current" => Some("current"),
        "mutual_inductance" => Some("inductance"),
        "temperature" | "temperature_rise" => Some("temperature"),
        "fault_rate" | "false_trigger_rate" => Some("rate"),
        "control_latency" => Some("time"),
        "data_rate" => Some("data_rate"),
        _ => None,
    }
}

fn dimension_for_unit(unit: &str) -> Option<&'static str> {
    match unit.trim() {
        "%" | "1" | "dB" => Some("dimensionless"),
        "W" | "mW" | "kW" | "MW" | "VA" | "var" => Some("power"),
        "J" | "kJ" | "Wh" | "kWh" => Some("energy"),
        "Wh/km" | "kWh/100km" => Some("energy_per_distance"),
        "V/m" | "kV/m" => Some("electric_field"),
        "A/m" => Some("magnetic_field"),
        "T" | "mT" | "uT" | "μT" | "µT" => Some("magnetic_flux_density"),
        "W/kg" => Some("specific_absorption_rate"),
        "A/m²" | "A/m^2" => Some("current_density"),
        "A" | "mA" => Some("current"),
        "H" | "mH" | "uH" | "μH" | "µH" => Some("inductance"),
        "K" | "degC" | "°C" => Some("temperature"),
        "s" | "ms" | "us" | "μs" | "µs" => Some("time"),
        "Hz" | "kHz" | "MHz" | "GHz" => Some("frequency"),
        "bps" | "kbps" | "Mbps" | "Gbps" => Some("data_rate"),
        "1/s" | "s^-1" | "h^-1" | "1/h" => Some("rate"),
        _ => None,
    }
}

fn validate_measurement(m: &Value) -> Vec<String> {
    let id = text(m, "measurement_id").unwrap_or("<unknown>");
    let mut errors = Vec::new();
    let q = text(m, "quantity_kind").unwrap_or("");
    let purpose = text(m, "measurement_purpose").unwrap_or("");
    let expected_purpose = match q {
        "service_energy_replenishment_ratio" => Some("vehicle_service_outcome"),
        "electric_field_strength" | "magnetic_field_strength" | "magnetic_flux_density"
        | "specific_absorption_rate" | "internal_electric_field" | "current_density"
        | "contact_current" => Some("human_exposure_assessment"),
        "signal_to_noise_ratio" | "bit_error_rate" | "data_rate" => Some("communications_performance"),
        "temperature" | "temperature_rise" => Some("thermal_performance"),
        "uptime_fraction" | "fault_rate" | "false_trigger_rate" | "control_latency" => Some("operational_reliability"),
        _ => None,
    };
    if expected_purpose.is_some_and(|expected| expected != purpose) {
        errors.push(format!("{id}: purpose/quantity mismatch"));
    }
    let raw = m.get("raw_measurement").unwrap_or(&Value::Null);
    let value = raw.get("value").and_then(Value::as_f64);
    let upper = raw.get("upper_value").and_then(Value::as_f64);
    match text(raw, "qualifier").unwrap_or("") {
        "EXACT" | "APPROX" | "GREATER_THAN" | "LESS_THAN" if value.is_none() || upper.is_some() =>
            errors.push(format!("{id}: qualifier/value mismatch")),
        "RANGE" if !matches!((value, upper), (Some(lo), Some(hi)) if hi >= lo) =>
            errors.push(format!("{id}: RANGE requires ordered lower and upper values")),
        "NOT_REPORTED" if value.is_some() || upper.is_some() =>
            errors.push(format!("{id}: NOT_REPORTED must have null values")),
        "EXACT" | "APPROX" | "GREATER_THAN" | "LESS_THAN" | "RANGE" | "NOT_REPORTED" => {}
        other => errors.push(format!("{id}: unsupported qualifier {other}")),
    }

    let raw_unit = text(raw, "unit").unwrap_or("");
    match (dimension_for_quantity(q), dimension_for_unit(raw_unit)) {
        (Some(expected), Some(actual)) if expected == actual => {}
        (Some(expected), Some(actual)) => errors.push(format!(
            "{id}: {q} expects dimension {expected}, but {raw_unit} has {actual}"
        )),
        (Some(_), None) => errors.push(format!("{id}: unsupported raw unit {raw_unit}")),
        (None, _) => errors.push(format!("{id}: unregistered quantity_kind {q}")),
    }

    if let Some(norm) = m.get("normalized_measurement").filter(|v| !v.is_null()) {
        let expected = dimension_for_quantity(q).unwrap_or("");
        let declared = text(norm, "dimension").unwrap_or("");
        let unit = text(norm, "unit").unwrap_or("");
        if expected != declared || dimension_for_unit(unit) != Some(declared) {
            errors.push(format!("{id}: normalized unit/dimension mismatch"));
        }
        if raw_unit == "%" && unit == "1" {
            if let (Some(raw), Some(converted)) = (value, n(m, &["normalized_measurement", "value"])) {
                if (converted - raw / 100.0).abs() > 1e-10 {
                    errors.push(format!("{id}: percent normalization is not raw/100"));
                }
            }
            if text(norm, "conversion_id") != Some("percent_to_fraction_v1") {
                errors.push(format!("{id}: missing percent_to_fraction_v1 conversion id"));
            }
        }
        if matches!(raw_unit, "μT" | "µT" | "uT") && unit == "T" {
            if let (Some(raw), Some(converted)) = (value, n(m, &["normalized_measurement", "value"])) {
                if (converted - raw * 1e-6).abs() > 1e-12 {
                    errors.push(format!("{id}: microtesla normalization is not raw*1e-6"));
                }
            }
            if text(norm, "conversion_id") != Some("microtesla_to_tesla_v1") {
                errors.push(format!("{id}: missing microtesla_to_tesla_v1 conversion id"));
            }
        }
    } else if value.is_some() {
        errors.push(format!("{id}: numeric raw value has no normalized record"));
    }

    if matches!(q, "end_to_end_efficiency" | "subsystem_efficiency" | "service_energy_replenishment_ratio") {
        for v in [value, upper].into_iter().flatten() {
            if !(0.0..=100.0).contains(&v) {
                errors.push(format!("{id}: percentage {v} outside 0..=100"));
            }
        }
    }

    let boundary = m.get("boundary").unwrap_or(&Value::Null);
    if text(boundary, "input_plane_kind").is_none()
        || text(boundary, "output_plane_kind").is_none()
        || text(boundary, "stage_set_status").is_none()
    {
        errors.push(format!("{id}: typed boundary plane fields missing"));
    }
    if q == "service_energy_replenishment_ratio" && text(boundary, "boundary_class") != Some("vehicle_service") {
        errors.push(format!("{id}: service ratio requires vehicle_service boundary"));
    }
    let is_field_quantity = matches!(
        q,
        "electric_field_strength" | "magnetic_field_strength" | "magnetic_flux_density"
            | "specific_absorption_rate" | "internal_electric_field" | "current_density"
            | "contact_current"
    );
    match (is_field_quantity, m.get("field_observation")) {
        (true, Some(Value::Object(_))) => {}
        (true, _) => errors.push(format!("{id}: field quantity requires a field_observation object")),
        (false, Some(Value::Null)) => {}
        (false, None) => errors.push(format!("{id}: field_observation field is required (null for non-field quantities)")),
        (false, Some(_)) => errors.push(format!("{id}: non-field quantity must not carry field_observation metadata")),
    }
    if matches!(q, "electric_field_strength" | "magnetic_field_strength" | "magnetic_flux_density"
        | "specific_absorption_rate" | "internal_electric_field" | "current_density" | "contact_current")
        && text(boundary, "boundary_class") != Some("field_probe")
    {
        errors.push(format!("{id}: field/exposure quantity requires field_probe boundary"));
    }
    errors
}

fn set_of(v: Option<&Value>) -> BTreeSet<String> {
    v.and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str)
            .map(|s| s.trim().to_ascii_lowercase()).collect())
        .unwrap_or_default()
}

fn compare_condition(left: &Value, right: &Value, field: &str, reasons: &mut BTreeSet<String>) {
    match (left.get(field).and_then(Value::as_f64), right.get(field).and_then(Value::as_f64)) {
        (Some(a), Some(b)) => {
            let tolerance = 1e-9_f64.max(a.abs().max(b.abs()) * 1e-6);
            if (a-b).abs() > tolerance {
                reasons.insert("OPERATING_CONDITION_MISMATCH".into());
            }
        }
        _ => { reasons.insert("OPERATING_CONDITION_INCOMPLETE".into()); }
    }
}

fn compare_string_condition(left: &Value, right: &Value, field: &str, reasons: &mut BTreeSet<String>) {
    let controlled = |value: &str| {
        let value = value.trim().to_ascii_lowercase();
        match field {
            "waveform" => matches!(value.as_str(), "sinusoidal" | "square" | "square_wave" | "triangular" | "dc" | "pwm"),
            "alignment" => matches!(value.as_str(), "nominal" | "aligned" | "centered" | "misaligned" | "x_misaligned" | "y_misaligned" | "xy_misaligned" | "misalignment_sweep"),
            "thermal_state" => matches!(value.as_str(), "cold" | "warm" | "hot" | "steady_state" | "transient" | "thermal_soak"),
            "medium" => matches!(value.as_str(), "air" | "water" | "soil" | "vacuum" | "oil" | "concrete" | "indoor_air"),
            _ => false,
        }
    };
    match (text(left, field), text(right, field)) {
        (Some(a), Some(b)) if a == b => {}
        (Some(a), Some(b)) if controlled(a) && controlled(b) => {
            reasons.insert("OPERATING_CONDITION_MISMATCH".into());
        }
        (Some(_), Some(_)) => {
            reasons.insert("OPERATING_CONDITION_UNNORMALIZED".into());
        }
        _ => { reasons.insert("OPERATING_CONDITION_INCOMPLETE".into()); }
    }
}

fn compare_statistic(left: &Value, right: &Value, reasons: &mut BTreeSet<String>) {
    let lkind = text(left, "kind").unwrap_or("");
    let rkind = text(right, "kind").unwrap_or("");
    if lkind == "reported_unspecified" || rkind == "reported_unspecified" || lkind.is_empty() || rkind.is_empty() {
        reasons.insert("STATISTIC_UNSPECIFIED".into());
        return;
    }
    if lkind != rkind {
        reasons.insert("STATISTIC_MISMATCH".into());
        return;
    }
    match (left.get("window_s").and_then(Value::as_f64), right.get("window_s").and_then(Value::as_f64)) {
        (Some(a), Some(b)) if (a-b).abs() <= 1e-9_f64.max(a.abs().max(b.abs()) * 1e-6) => {}
        (Some(_), Some(_)) => { reasons.insert("STATISTIC_MISMATCH".into()); }
        _ => { reasons.insert("STATISTIC_WINDOW_INCOMPLETE".into()); }
    }
    match (left.get("sample_count").and_then(Value::as_u64), right.get("sample_count").and_then(Value::as_u64)) {
        (Some(a), Some(b)) if a == b => {}
        (Some(_), Some(_)) => { reasons.insert("STATISTIC_MISMATCH".into()); }
        _ => { reasons.insert("STATISTIC_WINDOW_INCOMPLETE".into()); }
    }
}

pub fn compare_measurements(left: &Value, right: &Value, intent: &str) -> (String, Vec<String>) {
    let mut reasons = BTreeSet::new();
    let lq = text(left, "quantity_kind").unwrap_or("");
    let rq = text(right, "quantity_kind").unwrap_or("");
    if lq != rq { reasons.insert("QUANTITY_KIND_MISMATCH".into()); }
    if text(left, "measurement_purpose") != text(right, "measurement_purpose") {
        reasons.insert("PURPOSE_MISMATCH".into());
    }
    if intent == "rank_efficiency" && !matches!(lq, "end_to_end_efficiency" | "subsystem_efficiency") {
        reasons.insert("QUANTITY_KIND_MISMATCH".into());
    }

    let lb = left.get("boundary").unwrap_or(&Value::Null);
    let rb = right.get("boundary").unwrap_or(&Value::Null);
    if text(lb, "boundary_class") != text(rb, "boundary_class") {
        reasons.insert("BOUNDARY_CLASS_MISMATCH".into());
    }
    let li = text(lb, "input_plane_kind").unwrap_or("UNKNOWN");
    let ri = text(rb, "input_plane_kind").unwrap_or("UNKNOWN");
    let lo = text(lb, "output_plane_kind").unwrap_or("UNKNOWN");
    let ro = text(rb, "output_plane_kind").unwrap_or("UNKNOWN");
    if [li, ri, lo, ro].contains(&"UNKNOWN") {
        reasons.insert("BOUNDARY_INFORMATION_MISSING".into());
    } else {
        if li != ri { reasons.insert("INPUT_PLANE_MISMATCH".into()); }
        if lo != ro { reasons.insert("OUTPUT_PLANE_MISMATCH".into()); }
    }
    let field_boundary = text(lb, "boundary_class") == Some("field_probe")
        && text(rb, "boundary_class") == Some("field_probe");
    if field_boundary {
        if text(lb, "stage_set_status") != Some("NOT_APPLICABLE")
            || text(rb, "stage_set_status") != Some("NOT_APPLICABLE")
        {
            reasons.insert("BOUNDARY_INFORMATION_MISSING".into());
        }
    } else if text(lb, "stage_set_status") != Some("COMPLETE")
        || text(rb, "stage_set_status") != Some("COMPLETE")
    {
        reasons.insert("BOUNDARY_INFORMATION_MISSING".into());
    } else if set_of(lb.get("included_stages")) != set_of(rb.get("included_stages"))
        || set_of(lb.get("excluded_stages")) != set_of(rb.get("excluded_stages"))
    {
        reasons.insert("STAGE_SET_MISMATCH".into());
    }

    let ls = left.get("statistic").unwrap_or(&Value::Null);
    let rs = right.get("statistic").unwrap_or(&Value::Null);
    compare_statistic(ls, rs, &mut reasons);

    if text(left.get("evidence").unwrap_or(&Value::Null), "kind")
        != text(right.get("evidence").unwrap_or(&Value::Null), "kind")
    {
        reasons.insert("EVIDENCE_KIND_MISMATCH".into());
    }
    for m in [left, right] {
        if text(m, "qualification") != Some("QUALIFIED_RECORD") {
            reasons.insert("SOURCE_QUALIFICATION_INSUFFICIENT".into());
        }
        let uncertainty = m.get("evidence").and_then(|e| e.get("uncertainty")).unwrap_or(&Value::Null);
        if text(uncertainty, "kind") == Some("not_reported") {
            reasons.insert("UNCERTAINTY_NOT_REPORTED".into());
        }
        if text(m.get("evidence").unwrap_or(&Value::Null), "kind") == Some("UNKNOWN") {
            reasons.insert("SOURCE_QUALIFICATION_INSUFFICIENT".into());
        }
    }

    let is_field_comparison = matches!(
        lq,
        "electric_field_strength" | "magnetic_field_strength" | "magnetic_flux_density"
            | "specific_absorption_rate" | "internal_electric_field" | "current_density"
            | "contact_current"
    );
    if is_field_comparison {
        for measurement in [left, right] {
            let field = measurement.get("field_observation").unwrap_or(&Value::Null);
            let position = field.get("position_m").unwrap_or(&Value::Null);
            let complete = field.is_object()
                && text(field, "probe_method").is_some_and(|method| method == "MEASURED" || method == "SIMULATED" || method == "COMBINED")
                && ["x", "y", "z"].iter().all(|axis| position.get(*axis).and_then(Value::as_f64).is_some())
                && text(field, "orientation").is_some()
                && text(field, "reference_frame").is_some()
                && text(field, "spatial_region").is_some()
                && text(field, "averaging_method").is_some_and(|method| method != "not_reported");
            if !complete {
                reasons.insert("FIELD_ASSESSMENT_METADATA_INCOMPLETE".into());
            }
        }
        let left_field = left.get("field_observation").unwrap_or(&Value::Null);
        let right_field = right.get("field_observation").unwrap_or(&Value::Null);
        if text(left_field, "probe_method") != text(right_field, "probe_method") {
            reasons.insert("EVIDENCE_KIND_MISMATCH".into());
        }
        for key in ["orientation", "reference_frame", "spatial_region", "averaging_method"] {
            match (text(left_field, key), text(right_field, key)) {
                (Some(a), Some(b)) if a == b => {}
                (Some(_), Some(_)) => { reasons.insert("FIELD_GEOMETRY_MISMATCH".into()); }
                _ => { reasons.insert("FIELD_ASSESSMENT_METADATA_INCOMPLETE".into()); }
            }
        }
        let lp = left_field.get("position_m").unwrap_or(&Value::Null);
        let rp = right_field.get("position_m").unwrap_or(&Value::Null);
        for axis in ["x", "y", "z"] {
            match (lp.get(axis).and_then(Value::as_f64), rp.get(axis).and_then(Value::as_f64)) {
                (Some(a), Some(b)) if (a-b).abs() <= 1e-9 => {}
                (Some(_), Some(_)) => { reasons.insert("FIELD_GEOMETRY_MISMATCH".into()); }
                _ => { reasons.insert("FIELD_ASSESSMENT_METADATA_INCOMPLETE".into()); }
            }
        }
        compare_condition(left_field, right_field, "averaging_window_s", &mut reasons);
    }

    if intent == "rank_efficiency" || intent == "compare_exposure" {
        let lc = left.get("conditions").unwrap_or(&Value::Null);
        let rc = right.get("conditions").unwrap_or(&Value::Null);
        compare_condition(lc, rc, "frequency_hz", &mut reasons);
        compare_condition(lc, rc, "transfer_gap_m", &mut reasons);
        if intent == "rank_efficiency" {
            compare_condition(lc, rc, "load_power_w", &mut reasons);
        }
        compare_condition(lc, rc, "operating_duration_s", &mut reasons);
        for field in ["waveform", "alignment", "thermal_state", "medium"] {
            compare_string_condition(lc, rc, field, &mut reasons);
        }
    }

    let decisive = [
        "QUANTITY_KIND_MISMATCH","PURPOSE_MISMATCH","BOUNDARY_CLASS_MISMATCH",
        "INPUT_PLANE_MISMATCH","OUTPUT_PLANE_MISMATCH","STAGE_SET_MISMATCH",
        "STATISTIC_MISMATCH","EVIDENCE_KIND_MISMATCH","OPERATING_CONDITION_MISMATCH","FIELD_GEOMETRY_MISMATCH"
    ];
    let disposition = if reasons.iter().any(|r| decisive.contains(&r.as_str())) {
        "NOT_COMPARABLE"
    } else if !reasons.is_empty() {
        "UNRESOLVED"
    } else { "COMPARABLE" };
    (disposition.into(), reasons.into_iter().collect())
}

fn read_jsonl(path: &Path) -> Result<Vec<Value>, Box<dyn Error>> {
    let file = File::open(path)?;
    let mut values = Vec::new();
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() { continue; }
        values.push(serde_json::from_str(&line).map_err(|e| {
            format!("{} line {}: {e}", path.display(), index + 1)
        })?);
    }
    Ok(values)
}

fn unique_index<'a>(rows: &'a [Value], key: &str) -> Result<BTreeMap<String, &'a Value>, Box<dyn Error>> {
    let mut map = BTreeMap::new();
    for row in rows {
        let id = text(row, key).ok_or_else(|| format!("missing {key}"))?;
        if map.insert(id.to_owned(), row).is_some() {
            return Err(format!("duplicate {key} {id}").into());
        }
    }
    Ok(map)
}

pub fn audit(root: &Path) -> Result<Value, Box<dyn Error>> {
    let base = root.join("benchmarks/wpt-empirical");
    let measurements = read_jsonl(&base.join("v1/measurements.jsonl"))?;
    let cases = read_jsonl(&base.join("v1/comparison-cases.jsonl"))?;
    let sources = read_jsonl(&base.join("v0/sources.jsonl"))?;
    let systems = read_jsonl(&base.join("v0/realizations.jsonl"))?;
    let source_index = unique_index(&sources, "source_id")?;
    let system_index = unique_index(&systems, "system_id")?;
    let mut ids = BTreeSet::new();
    let mut measurement_errors = Vec::new();

    for m in &measurements {
        let id = text(m, "measurement_id").unwrap_or("<unknown>");
        if !ids.insert(id.to_owned()) {
            measurement_errors.push(format!("{id}: duplicate measurement ID"));
        }
        measurement_errors.extend(validate_measurement(m));
        if !system_index.contains_key(text(m, "system_id").unwrap_or("")) {
            measurement_errors.push(format!("{id}: unresolved prototype system_id"));
        }
        if let Some(refs) = m.get("source_refs").and_then(Value::as_array) {
            for s in refs {
                if !source_index.contains_key(text(s, "source_id").unwrap_or("")) {
                    measurement_errors.push(format!("{id}: unresolved source reference"));
                }
            }
        } else {
            measurement_errors.push(format!("{id}: source_refs missing"));
        }
    }

    let mut results = Vec::new();
    let mut case_ids = BTreeSet::new();
    let mut mismatches = Vec::new();
    for case in &cases {
        let cid = text(case, "case_id").unwrap_or("<unknown>");
        if !case_ids.insert(cid.to_owned()) {
            mismatches.push(format!("{cid}: duplicate case id"));
            continue;
        }
        let by_id: BTreeMap<&str, &Value> = measurements.iter()
            .filter_map(|m| Some((text(m, "measurement_id")?, m))).collect();
        let left_id = text(case, "left_measurement_id").unwrap_or("");
        let right_id = text(case, "right_measurement_id").unwrap_or("");
        let left = by_id.get(left_id).ok_or_else(|| format!("{cid}: missing measurement {left_id}"))?;
        let right = by_id.get(right_id).ok_or_else(|| format!("{cid}: missing measurement {right_id}"))?;
        let intent = text(case, "comparison_intent").unwrap_or("");
        let (actual, reasons) = compare_measurements(left, right, intent);
        let expected = text(case, "expected_disposition").unwrap_or("");
        let expected_reasons: BTreeSet<String> = case
            .get("expected_reasons")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(Value::as_str).map(str::to_owned).collect())
            .unwrap_or_default();
        let actual_reasons: BTreeSet<String> = reasons.iter().cloned().collect();
        let missing_expected_reasons: Vec<String> = expected_reasons
            .difference(&actual_reasons)
            .cloned()
            .collect();
        let matches = actual == expected && missing_expected_reasons.is_empty();
        if !matches {
            mismatches.push(format!(
                "{cid}: expected {expected} with reasons {:?}, got {actual} with reasons {:?}; missing expected reasons {:?}",
                expected_reasons, reasons, missing_expected_reasons
            ));
        }
        results.push(json!({
            "case_id": cid,
            "actual": actual,
            "expected": expected,
            "matches_expected": matches,
            "reasons": reasons,
            "missing_expected_reasons": missing_expected_reasons
        }));
    }

    let mut counts = BTreeMap::<String, usize>::new();
    for result in &results {
        if let Some(d) = result.get("actual").and_then(Value::as_str) {
            *counts.entry(d.to_owned()).or_default() += 1;
        }
    }
    Ok(json!({
        "measurement_count": measurements.len(),
        "comparison_case_count": cases.len(),
        "source_count": sources.len(),
        "prototype_family_count": systems.len(),
        "unique_measurement_ids": ids.len(),
        "measurement_errors": measurement_errors,
        "disposition_counts": counts,
        "comparison_results": results,
        "expectation_mismatches": mismatches,
        "valid": measurement_errors.is_empty() && mismatches.is_empty()
    }))
}

fn main() -> Result<(), Box<dyn Error>> {
    let root = std::env::args_os().nth(1).map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let report = audit(&root)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if report["valid"] != Value::Bool(true) {
        return Err("WPT typed measurement audit failed".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample(id: &str, percent: f64) -> Value {
        json!({
            "measurement_id": id,
            "system_id": "synthetic",
            "quantity_kind": "subsystem_efficiency",
            "measurement_purpose": "subsystem_characterization",
            "raw_measurement": {
                "value": percent, "upper_value": null, "unit": "%",
                "qualifier": "EXACT", "reported_text": "synthetic test only"
            },
            "normalized_measurement": {
                "value": percent / 100.0, "upper_value": null, "unit": "1",
                "conversion_id": "percent_to_fraction_v1", "dimension": "dimensionless"
            },
            "boundary": {
                "boundary_class": "subsystem",
                "input_plane_kind": "PRIMARY_DC_BUS",
                "output_plane_kind": "SECONDARY_DC_BUS",
                "stage_set_status": "COMPLETE",
                "included_stages": ["inverter", "coupler", "rectifier"],
                "excluded_stages": ["mains", "battery"]
            },
            "statistic": {"kind": "steady_state", "window_s": 60.0, "sample_count": 600},
            "evidence": {
                "kind": "MEASURED",
                "uncertainty": {"kind": "expanded", "lower": 0.1, "upper": 0.1, "coverage_factor": 2.0}
            },
            "qualification": "QUALIFIED_RECORD",
            "conditions": {
                "frequency_hz": 85000.0, "transfer_gap_m": 0.16,
                "load_power_w": 50000.0, "operating_duration_s": 60.0,
                "waveform": "sinusoidal", "alignment": "nominal",
                "thermal_state": "steady_state", "medium": "air"
            }
        })
    }

    #[test]
    fn unit_dimensions_are_not_interchangeable() {
        assert_eq!(dimension_for_unit("μT"), Some("magnetic_flux_density"));
        assert_eq!(dimension_for_unit("V/m"), Some("electric_field"));
        assert_eq!(dimension_for_unit("W"), Some("power"));
        assert_eq!(dimension_for_unit("%"), Some("dimensionless"));
        assert_eq!(dimension_for_quantity("apparent_power"), Some("power"));
        assert_eq!(dimension_for_quantity("reactive_power"), Some("power"));
        assert_eq!(dimension_for_quantity("power_factor"), Some("dimensionless"));
    }

    #[test]
    fn rejects_watts_labelled_as_efficiency() {
        let mut m = sample("wrong-unit", 95.0);
        m["raw_measurement"]["unit"] = json!("W");
        assert!(validate_measurement(&m).iter().any(|e| e.contains("expects dimension")));
    }

    #[test]
    fn rejects_unordered_ranges_and_bad_percent_normalization() {
        let mut m = sample("bad-range", 95.0);
        m["raw_measurement"]["qualifier"] = json!("RANGE");
        m["raw_measurement"]["upper_value"] = json!(94.0);
        m["normalized_measurement"]["value"] = json!(0.95);
        m["normalized_measurement"]["upper_value"] = json!(0.94);
        assert!(!validate_measurement(&m).is_empty());

        let mut wrong = sample("bad-conversion", 95.0);
        wrong["normalized_measurement"]["value"] = json!(95.0);
        assert!(validate_measurement(&wrong).iter().any(|e| e.contains("percent normalization")));
    }

    #[test]
    fn whole_system_and_subsystem_are_not_comparable() {
        let mut left = sample("whole", 95.0);
        let mut right = sample("sub", 96.0);
        left["quantity_kind"] = json!("end_to_end_efficiency");
        left["measurement_purpose"] = json!("system_transfer_performance");
        left["boundary"]["boundary_class"] = json!("whole_system");
        let (disposition, reasons) = compare_measurements(&left, &right, "rank_efficiency");
        assert_eq!(disposition, "NOT_COMPARABLE");
        assert!(reasons.contains(&"QUANTITY_KIND_MISMATCH".to_string()));
        assert!(reasons.contains(&"BOUNDARY_CLASS_MISMATCH".to_string()));
    }

    #[test]
    fn unknown_meter_plane_is_unresolved() {
        let left = sample("left", 95.0);
        let mut right = sample("right", 96.0);
        right["boundary"]["input_plane_kind"] = json!("UNKNOWN");
        let (disposition, reasons) = compare_measurements(&left, &right, "rank_efficiency");
        assert_eq!(disposition, "UNRESOLVED");
        assert!(reasons.contains(&"BOUNDARY_INFORMATION_MISSING".to_string()));
    }

    #[test]
    fn matched_qualified_design_records_are_comparable() {
        let left = sample("left", 95.0);
        let right = sample("right", 96.0);
        let (disposition, reasons) = compare_measurements(&left, &right, "rank_efficiency");
        assert_eq!(disposition, "COMPARABLE");
        assert!(reasons.is_empty());
    }

    fn field_sample() -> Value {
        let mut m = sample("FIELD-KNOWN-TRUTH", 13.86);
        m["quantity_kind"] = json!("magnetic_flux_density");
        m["measurement_purpose"] = json!("human_exposure_assessment");
        m["raw_measurement"]["unit"] = json!("μT");
        m["normalized_measurement"]["value"] = json!(0.00001386);
        m["normalized_measurement"]["unit"] = json!("T");
        m["normalized_measurement"]["conversion_id"] = json!("microtesla_to_tesla_v1");
        m["normalized_measurement"]["dimension"] = json!("magnetic_flux_density");
        m["boundary"]["boundary_class"] = json!("field_probe");
        m["boundary"]["input_plane_kind"] = json!("NOT_APPLICABLE");
        m["boundary"]["output_plane_kind"] = json!("NOT_APPLICABLE");
        m["boundary"]["stage_set_status"] = json!("NOT_APPLICABLE");
        m["field_observation"] = json!({
            "probe_method": "UNKNOWN",
            "probe_model": null,
            "position_m": {"x": null, "y": null, "z": null},
            "orientation": null,
            "reference_frame": null,
            "spatial_region": null,
            "averaging_method": "not_reported",
            "averaging_window_s": null
        });
        m["qualification"] = json!("SOURCE_CLAIM_ONLY");
        m
    }

    #[test]
    fn field_quantity_requires_typed_probe_record_and_unknown_geometry_stays_unresolved() {
        let field = field_sample();
        assert!(validate_measurement(&field).is_empty());

        let (disposition, reasons) = compare_measurements(&field, &field, "compare_exposure");
        assert_eq!(disposition, "UNRESOLVED");
        assert!(reasons.contains(&"FIELD_ASSESSMENT_METADATA_INCOMPLETE".to_string()));
        assert!(reasons.contains(&"SOURCE_QUALIFICATION_INSUFFICIENT".to_string()));
    }

    #[test]
    fn checked_in_corpus_matches_frozen_expected_dispositions() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let report = audit(&root).expect("WPT comparator corpus should load");
        assert_eq!(report["valid"], Value::Bool(true), "{report:#}");
        assert_eq!(report["measurement_count"], json!(10));
        assert_eq!(report["comparison_case_count"], json!(6));
    }
}
