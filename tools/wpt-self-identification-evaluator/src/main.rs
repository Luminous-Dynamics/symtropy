// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fs;
use std::process;

const TOP_LEVEL: &[&str] = &[
    "schema_version",
    "run_id",
    "profile_id",
    "system",
    "conditions",
    "reference",
    "identification",
    "estimates",
    "control",
    "artifacts",
    "qualification",
];

const PARAMETERS: &[&str] = &[
    "L_PRIMARY",
    "L_SECONDARY",
    "MUTUAL_INDUCTANCE",
    "EQUIVALENT_LOAD_RESISTANCE",
];

const ARTIFACT_TYPES: &[&str] = &[
    "RAW_WAVEFORM",
    "REFERENCE_MEASUREMENT",
    "ESTIMATOR_OUTPUT",
    "CONTROLLER_CONFIG",
];

#[derive(Debug, Clone)]
struct RunMetrics {
    run_id: String,
    split: String,
    operating_point_id: String,
    batch_id: String,
    profile_id: String,
    system_id: String,
    identification_method: String,
    errors: BTreeMap<String, f64>,
    zvs_coverage_pct: f64,
    excitation_overhead_pct: f64,
    p99_latency_ms: f64,
}

fn at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(value, |current, key| current.get(*key))
}

fn string_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    at(value, path).and_then(Value::as_str)
}

fn number_at(value: &Value, path: &[&str]) -> Option<f64> {
    at(value, path).and_then(Value::as_f64)
}

fn bool_at(value: &Value, path: &[&str]) -> Option<bool> {
    at(value, path).and_then(Value::as_bool)
}

fn nonempty(value: Option<&str>) -> bool {
    value.is_some_and(|text| !text.trim().is_empty())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_uppercase() || first.is_ascii_digit())
        && chars.all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || "._-".contains(ch))
}

fn exact_keys(
    value: Option<&Value>,
    expected: &[&str],
    path: &str,
    errors: &mut Vec<String>,
) {
    let Some(object) = value.and_then(Value::as_object) else {
        errors.push(format!("{path}: expected object"));
        return;
    };
    for key in expected {
        if !object.contains_key(*key) {
            errors.push(format!("{path}.{key}: required field missing"));
        }
    }
    for key in object.keys() {
        if !expected.contains(&key.as_str()) {
            errors.push(format!("{path}.{key}: unexpected field"));
        }
    }
}

fn exact_array_keys(
    value: Option<&Value>,
    expected: &[&str],
    path: &str,
    errors: &mut Vec<String>,
) {
    let Some(items) = value.and_then(Value::as_array) else {
        errors.push(format!("{path}: expected array"));
        return;
    };
    for (index, item) in items.iter().enumerate() {
        exact_keys(Some(item), expected, &format!("{path}[{index}]"), errors);
    }
}

fn parse_utc_seconds(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return None;
    }
    let part = |start: usize, end: usize| -> Option<i64> {
        std::str::from_utf8(&bytes[start..end]).ok()?.parse().ok()
    };
    let year = part(0, 4)?;
    let month = part(5, 7)?;
    let day = part(8, 10)?;
    let hour = part(11, 13)?;
    let minute = part(14, 16)?;
    let second = part(17, 19)?;
    if !(1..=12).contains(&month)
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute)
        || !(0..=59).contains(&second)
    {
        return None;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if !(1..=month_days).contains(&day) {
        return None;
    }
    let adjusted_year = year - i64::from(month <= 2);
    let era = (if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    }) / 400;
    let year_of_era = adjusted_year - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
    let day_of_era =
        year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    Some(days * 86_400 + hour * 3_600 + minute * 60 + second)
}

fn validate_structure(run: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    exact_keys(Some(run), TOP_LEVEL, "$", &mut errors);
    if string_at(run, &["schema_version"]) != Some("wpt-self-identification-run-v1") {
        errors.push("schema_version: expected wpt-self-identification-run-v1".into());
    }
    match string_at(run, &["run_id"]) {
        Some(id) if valid_identifier(id) => {}
        _ => errors.push("run_id: expected uppercase stable identifier".into()),
    }
    if !nonempty(string_at(run, &["profile_id"])) {
        errors.push("profile_id: nonempty profile ID required".into());
    }

    exact_keys(
        run.get("system"),
        &[
            "system_id",
            "hardware_revision",
            "controller_revision",
            "configuration_sha256",
            "power_rating_w",
            "topology",
        ],
        "system",
        &mut errors,
    );
    exact_keys(
        run.get("conditions"),
        &[
            "split",
            "operating_point_id",
            "direction",
            "frequency_hz",
            "transfer_gap_m",
            "offset_x_m",
            "offset_y_m",
            "offset_z_m",
            "yaw_deg",
            "load_resistance_ohm",
            "output_power_w",
            "ambient_temperature_c",
            "coil_temperature_c",
            "thermal_state",
        ],
        "conditions",
        &mut errors,
    );
    exact_keys(
        run.get("reference"),
        &[
            "instrument_id",
            "instrument_model",
            "method",
            "calibration_id",
            "calibration_valid_from_utc",
            "calibration_due_utc",
            "measurement_uncertainty_pct",
            "window_start_utc",
            "window_end_utc",
            "reference_artifact_sha256",
        ],
        "reference",
        &mut errors,
    );
    exact_keys(
        run.get("identification"),
        &[
            "method",
            "secondary_data_link_required",
            "perturbation_energy_j",
            "useful_transfer_energy_j",
            "reported_overhead_pct",
            "p99_latency_ms",
            "control_deadline_ms",
            "sample_count",
            "batch_id",
        ],
        "identification",
        &mut errors,
    );
    exact_array_keys(
        run.get("estimates"),
        &[
            "parameter",
            "unit",
            "estimate_value",
            "reference_value",
            "relative_error_pct",
            "estimate_uncertainty_pct",
            "coverage_factor",
            "coverage_probability_pct",
            "reference_evidence_kind",
            "estimate_artifact_sha256",
        ],
        "estimates",
        &mut errors,
    );
    exact_keys(
        run.get("control"),
        &[
            "zvs_observation_count",
            "zvs_success_count",
            "reported_zvs_coverage_pct",
            "switching_limit_violation_count",
            "protection_trip_count",
            "unidentifiable_state_fallback",
            "uncertainty_outside_grid_rejected",
            "authority_plane_independent",
            "estimator_can_authorize_energization",
        ],
        "control",
        &mut errors,
    );
    exact_array_keys(
        run.get("artifacts"),
        &["artifact_type", "uri", "sha256", "description"],
        "artifacts",
        &mut errors,
    );
    if !matches!(
        string_at(run, &["qualification"]),
        Some("NOT_RUN" | "INCOMPLETE" | "QUALIFIED" | "REJECTED" | "MODEL_ONLY")
    ) {
        errors.push("qualification: unsupported or missing disposition".into());
    }
    if !run.get("estimates").is_some_and(Value::is_array) {
        errors.push("estimates: expected array".into());
    }
    if !run.get("artifacts").is_some_and(Value::is_array) {
        errors.push("artifacts: expected array".into());
    }
    errors
}

fn validate_qualified(run: &Value) -> Result<RunMetrics, Vec<String>> {
    let mut errors = Vec::new();

    for (path, label) in [
        (&["system", "system_id"][..], "system.system_id"),
        (&["system", "hardware_revision"][..], "system.hardware_revision"),
        (&["system", "controller_revision"][..], "system.controller_revision"),
        (&["system", "topology"][..], "system.topology"),
        (&["identification", "batch_id"][..], "identification.batch_id"),
    ] {
        if !nonempty(string_at(run, path)) {
            errors.push(format!("{label}: required for QUALIFIED"));
        }
    }
    if !string_at(run, &["system", "configuration_sha256"]).is_some_and(valid_sha256) {
        errors.push("system.configuration_sha256: valid SHA-256 required".into());
    }
    if !number_at(run, &["system", "power_rating_w"]).is_some_and(|value| value > 0.0) {
        errors.push("system.power_rating_w: positive declared rating required".into());
    }
    if !matches!(
        string_at(run, &["conditions", "split"]),
        Some("TEST" | "HOLDOUT")
    ) {
        errors.push("conditions.split: physical qualification requires TEST or HOLDOUT split".into());
    }
    if !nonempty(string_at(run, &["conditions", "operating_point_id"])) {
        errors.push("conditions.operating_point_id: stable operating-point ID required".into());
    }
    if !matches!(
        string_at(run, &["conditions", "direction"]),
        Some("GRID_TO_VEHICLE" | "VEHICLE_TO_GRID" | "UNIDIRECTIONAL")
    ) {
        errors.push("conditions.direction: declared power-flow direction required".into());
    }
    if !number_at(run, &["conditions", "frequency_hz"]).is_some_and(|value| value > 0.0) {
        errors.push("conditions.frequency_hz: positive operating frequency required".into());
    }
    if !number_at(run, &["conditions", "transfer_gap_m"]).is_some_and(|value| value >= 0.0) {
        errors.push("conditions.transfer_gap_m: measured transfer gap required".into());
    }
    for axis in ["offset_x_m", "offset_y_m", "offset_z_m", "yaw_deg"] {
        if !number_at(run, &["conditions", axis]).is_some() {
            errors.push(format!("conditions.{axis}: measured alignment value required"));
        }
    }
    if !number_at(run, &["conditions", "ambient_temperature_c"]).is_some()
        || !number_at(run, &["conditions", "coil_temperature_c"]).is_some()
    {
        errors.push("conditions: ambient and coil temperatures required".into());
    }
    if string_at(run, &["conditions", "thermal_state"]) == Some("UNKNOWN") {
        errors.push("conditions.thermal_state: known thermal state required".into());
    }

    let method = string_at(run, &["reference", "method"]);
    if matches!(method, None | Some("UNKNOWN")) {
        errors.push("reference.method: traceable reference method required".into());
    }
    for field in ["instrument_id", "instrument_model", "calibration_id"] {
        if !nonempty(string_at(run, &["reference", field])) {
            errors.push(format!("reference.{field}: required for QUALIFIED"));
        }
    }
    let valid_from = string_at(run, &["reference", "calibration_valid_from_utc"])
        .and_then(parse_utc_seconds);
    let due = string_at(run, &["reference", "calibration_due_utc"]).and_then(parse_utc_seconds);
    let start =
        string_at(run, &["reference", "window_start_utc"]).and_then(parse_utc_seconds);
    let end = string_at(run, &["reference", "window_end_utc"]).and_then(parse_utc_seconds);
    match (valid_from, due, start, end) {
        (Some(from), Some(due), Some(start), Some(end))
            if from <= start && due >= end && due >= from && end > start => {}
        _ => errors.push(
            "reference: calibration validity must cover the ordered UTC measurement window"
                .into(),
        ),
    }
    if !number_at(run, &["reference", "measurement_uncertainty_pct"])
        .is_some_and(|value| value >= 0.0)
    {
        errors.push("reference.measurement_uncertainty_pct: nonnegative uncertainty required".into());
    }
    let reference_digest = string_at(run, &["reference", "reference_artifact_sha256"]);
    if !reference_digest.is_some_and(valid_sha256) {
        errors.push("reference.reference_artifact_sha256: valid digest required".into());
    }

    let estimates = run.get("estimates").and_then(Value::as_array);
    let mut errors_by_parameter = BTreeMap::new();
    match estimates {
        Some(records) if records.len() == PARAMETERS.len() => {
            for (index, estimate) in records.iter().enumerate() {
                let parameter = string_at(estimate, &["parameter"]).unwrap_or("");
                if !PARAMETERS.contains(&parameter) {
                    errors.push(format!("estimates[{index}].parameter: unknown parameter"));
                    continue;
                }
                if errors_by_parameter.contains_key(parameter) {
                    errors.push(format!("estimates[{index}].parameter: duplicate parameter"));
                    continue;
                }
                let expected_unit = if parameter == "EQUIVALENT_LOAD_RESISTANCE" {
                    "OHM"
                } else {
                    "H"
                };
                if string_at(estimate, &["unit"]) != Some(expected_unit) {
                    errors.push(format!(
                        "estimates[{index}].unit: {parameter} requires {expected_unit}"
                    ));
                }
                let value = number_at(estimate, &["estimate_value"]);
                let reference = number_at(estimate, &["reference_value"]);
                let reported = number_at(estimate, &["relative_error_pct"]);
                let uncertainty = number_at(estimate, &["estimate_uncertainty_pct"]);
                let coverage_factor = number_at(estimate, &["coverage_factor"]);
                let coverage_probability = number_at(estimate, &["coverage_probability_pct"]);
                match (value, reference, reported, uncertainty, coverage_factor, coverage_probability) {
                    (Some(value), Some(reference), Some(reported), Some(uncertainty))
                        if value > 0.0 && reference > 0.0 && uncertainty >= 0.0 =>
                    {
                        let calculated = (value - reference).abs() / reference.abs() * 100.0;
                        if (calculated - reported).abs() > 1e-6 {
                            errors.push(format!(
                                "estimates[{index}].relative_error_pct: calculated {calculated:.9}%, reported {reported:.9}%"
                            ));
                        }
                        errors_by_parameter.insert(parameter.to_owned(), calculated);
                    }
                    _ => errors.push(format!(
                        "estimates[{index}]: positive estimate/reference, reported error and nonnegative uncertainty required"
                    )),
                }
                if string_at(estimate, &["reference_evidence_kind"]) != Some("MEASURED") {
                    errors.push(format!(
                        "estimates[{index}].reference_evidence_kind: a QUALIFIED physical run requires MEASURED reference evidence"
                    ));
                }
                if !string_at(estimate, &["estimate_artifact_sha256"])
                    .is_some_and(valid_sha256)
                {
                    errors.push(format!(
                        "estimates[{index}].estimate_artifact_sha256: valid raw estimate digest required"
                    ));
                }
            }
        }
        _ => errors.push(format!(
            "estimates: exactly {} unique parameter estimates required",
            PARAMETERS.len()
        )),
    }
    for parameter in PARAMETERS {
        if !errors_by_parameter.contains_key(*parameter) {
            errors.push(format!("estimates: missing {parameter}"));
        }
    }

    let identification_method = string_at(run, &["identification", "method"]);
    if matches!(identification_method, None | Some("UNKNOWN")) {
        errors.push("identification.method: declared estimator method required".into());
    }
    if bool_at(run, &["identification", "secondary_data_link_required"]).is_none() {
        errors.push("identification.secondary_data_link_required: explicit true/false required".into());
    }
    let perturbation_energy = number_at(run, &["identification", "perturbation_energy_j"]);
    let useful_energy = number_at(run, &["identification", "useful_transfer_energy_j"]);
    let overhead_reported = number_at(run, &["identification", "reported_overhead_pct"]);
    let latency = number_at(run, &["identification", "p99_latency_ms"]);
    let deadline = number_at(run, &["identification", "control_deadline_ms"]);
    match (perturbation_energy, useful_energy, overhead_reported) {
        (Some(perturbation), Some(useful), Some(reported))
            if perturbation >= 0.0 && useful > 0.0 =>
        {
            let overhead = perturbation / useful * 100.0;
            if (overhead - reported).abs() > 1e-6 {
                errors.push(format!(
                    "identification.reported_overhead_pct: calculated {overhead:.9}%, reported {reported:.9}%"
                ));
            }
            if overhead > 0.5 {
                errors.push(format!(
                    "identification: perturbation-energy overhead {overhead:.6}% exceeds proposed 0.500% target"
                ));
            }
        }
        _ => errors.push(
            "identification: perturbation energy, useful transfer energy and reported overhead required"
                .into(),
        ),
    }
    match (latency, deadline) {
        (Some(latency), Some(deadline)) if deadline > 0.0 && latency <= deadline => {}
        _ => errors.push(
            "identification.p99_latency_ms: measured p99 latency must meet the declared positive control deadline"
                .into(),
        ),
    }
    if !at(run, &["identification", "sample_count"])
        .and_then(Value::as_u64)
        .is_some_and(|value| value > 0)
    {
        errors.push("identification.sample_count: positive sample count required".into());
    }
    let batch_id = string_at(run, &["identification", "batch_id"]).unwrap_or("");

    let observation_count = at(run, &["control", "zvs_observation_count"])
        .and_then(Value::as_u64);
    let success_count = at(run, &["control", "zvs_success_count"])
        .and_then(Value::as_u64);
    let reported_coverage = number_at(run, &["control", "reported_zvs_coverage_pct"]);
    let mut zvs_coverage = 0.0;
    match (observation_count, success_count, reported_coverage) {
        (Some(observations), Some(successes), Some(reported))
            if observations > 0 && successes <= observations =>
        {
            zvs_coverage = successes as f64 / observations as f64 * 100.0;
            if (zvs_coverage - reported).abs() > 1e-6 {
                errors.push(format!(
                    "control.reported_zvs_coverage_pct: calculated {zvs_coverage:.6}%, reported {reported:.6}%"
                ));
            }
            if zvs_coverage < 99.0 {
                errors.push(format!(
                    "control: ZVS coverage {zvs_coverage:.6}% is below proposed 99.000% target"
                ));
            }
        }
        _ => errors.push("control: valid ZVS observation/success counts required".into()),
    }
    for field in ["switching_limit_violation_count", "protection_trip_count"] {
        if at(run, &["control", field]).and_then(Value::as_u64) != Some(0) {
            errors.push(format!("control.{field}: must be zero for QUALIFIED"));
        }
    }
    if !matches!(
        string_at(run, &["control", "unidentifiable_state_fallback"]),
        Some("REDUCE_POWER" | "DEENERGIZE" | "HOLD_LAST_SAFE")
    ) {
        errors.push("control.unidentifiable_state_fallback: safe fallback required".into());
    }
    if bool_at(run, &["control", "uncertainty_outside_grid_rejected"]) != Some(true) {
        errors.push("control.uncertainty_outside_grid_rejected: out-of-envelope estimates must be rejected".into());
    }
    if bool_at(run, &["control", "authority_plane_independent"]) != Some(true) {
        errors.push("control.authority_plane_independent: session/receiver authority must be independent of estimator".into());
    }
    if bool_at(run, &["control", "estimator_can_authorize_energization"]) != Some(false) {
        errors.push("control.estimator_can_authorize_energization: estimator must never authorize energization".into());
    }

    let mut artifact_types = BTreeSet::new();
    let mut artifact_digests: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let artifacts = run.get("artifacts").and_then(Value::as_array);
    if let Some(artifacts) = artifacts {
        for (index, artifact) in artifacts.iter().enumerate() {
            let kind = string_at(artifact, &["artifact_type"]).unwrap_or("");
            if ![
                "RAW_WAVEFORM",
                "REFERENCE_MEASUREMENT",
                "ESTIMATOR_OUTPUT",
                "CONTROLLER_CONFIG",
                "REPORT",
                "OTHER",
            ]
            .contains(&kind)
            {
                errors.push(format!(
                    "artifacts[{index}].artifact_type: unknown artifact type"
                ));
            }
            artifact_types.insert(kind.to_owned());
            if !nonempty(string_at(artifact, &["uri"])) {
                errors.push(format!("artifacts[{index}].uri: required"));
            }
            match string_at(artifact, &["sha256"]) {
                Some(digest) if valid_sha256(digest) => {
                    artifact_digests
                        .entry(kind.to_owned())
                        .or_default()
                        .insert(digest.to_owned());
                }
                _ => errors.push(format!(
                    "artifacts[{index}].sha256: valid SHA-256 required"
                )),
            }
            if !nonempty(string_at(artifact, &["description"])) {
                errors.push(format!("artifacts[{index}].description: required"));
            }
        }
    }
    for kind in ARTIFACT_TYPES {
        if !artifact_types.contains(*kind) {
            errors.push(format!("artifacts: missing required {kind} artifact"));
        }
    }
    if !artifact_digests
        .get("REFERENCE_MEASUREMENT")
        .is_some_and(|digests| reference_digest.is_some_and(|digest| digests.contains(digest)))
    {
        errors.push(
            "reference.reference_artifact_sha256: digest must bind a REFERENCE_MEASUREMENT artifact"
                .into(),
        );
    }
    if let Some(digests) = artifact_digests.get("ESTIMATOR_OUTPUT") {
        for (index, estimate) in estimates.unwrap_or(&Vec::new()).iter().enumerate() {
            if !string_at(estimate, &["estimate_artifact_sha256"])
                .is_some_and(|digest| digests.contains(digest))
            {
                errors.push(format!(
                    "estimates[{index}].estimate_artifact_sha256: digest must bind an ESTIMATOR_OUTPUT artifact"
                ));
            }
        }
    } else {
        errors.push("artifacts: ESTIMATOR_OUTPUT digest bundle is required".into());
    }
    let controller_digest = string_at(run, &["system", "configuration_sha256"]);
    if !artifact_digests
        .get("CONTROLLER_CONFIG")
        .is_some_and(|digests| controller_digest.is_some_and(|digest| digests.contains(digest)))
    {
        errors.push(
            "system.configuration_sha256: digest must bind a CONTROLLER_CONFIG artifact".into(),
        );
    }

    if errors.is_empty() {
        Ok(RunMetrics {
            run_id: string_at(run, &["run_id"]).unwrap_or_default().to_owned(),
            split: string_at(run, &["conditions", "split"]).unwrap_or_default().to_owned(),
            operating_point_id: string_at(run, &["conditions", "operating_point_id"])
                .unwrap_or_default()
                .to_owned(),
            batch_id: batch_id.to_owned(),
            profile_id: string_at(run, &["profile_id"]).unwrap_or_default().to_owned(),
            system_id: string_at(run, &["system", "system_id"]).unwrap_or_default().to_owned(),
            identification_method: string_at(run, &["identification", "method"]).unwrap_or_default().to_owned(),
            errors: errors_by_parameter,
            zvs_coverage_pct: zvs_coverage,
            excitation_overhead_pct: perturbation_energy.unwrap_or(0.0)
                / useful_energy.unwrap_or(1.0)
                * 100.0,
            p99_latency_ms: latency.unwrap_or(0.0),
        })
    } else {
        Err(errors)
    }
}

fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        Some((sorted[mid - 1] + sorted[mid]) / 2.0)
    } else {
        Some(sorted[mid])
    }
}

fn percentile_nearest_rank(values: &[f64], percentile: f64) -> Option<f64> {
    if values.is_empty() || !(0.0..=100.0).contains(&percentile) {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = ((percentile / 100.0) * sorted.len() as f64).ceil().max(1.0) as usize;
    Some(sorted[(rank - 1).min(sorted.len() - 1)])
}

fn evaluate_batch(metrics: &[RunMetrics]) -> Vec<String> {
    let mut notices = Vec::new();
    let holdout = metrics
        .iter()
        .filter(|run| run.split == "HOLDOUT")
        .collect::<Vec<_>>();
    if holdout.len() < 30 {
        notices.push(format!(
            "AGGREGATE INCOMPLETE: {} HOLDOUT operating points; at least 30 are required before publishing batch-level P95 claims",
            holdout.len()
        ));
    }
    let group_count = |values: Vec<&str>| values.into_iter().collect::<BTreeSet<_>>().len();
    if group_count(holdout.iter().map(|run| run.batch_id.as_str()).collect()) > 1
        || group_count(holdout.iter().map(|run| run.profile_id.as_str()).collect()) > 1
        || group_count(holdout.iter().map(|run| run.system_id.as_str()).collect()) > 1
        || group_count(
            holdout
                .iter()
                .map(|run| run.identification_method.as_str())
                .collect(),
        ) > 1
    {
        notices.push(
            "AGGREGATE NOT COMPUTED: HOLDOUT records mix batch, profile, system or estimator method; partition these cohorts before calculating percentiles"
                .into(),
        );
        return notices;
    }
    let operating_points = holdout
        .iter()
        .map(|run| run.operating_point_id.as_str())
        .collect::<BTreeSet<_>>();
    if operating_points.len() != holdout.len() {
        notices.push(
            "HOLDOUT set contains repeated operating-point IDs; repeats must not be counted as independent grid coverage"
                .into(),
        );
        return notices;
    }
    for parameter in PARAMETERS {
        let values = holdout
            .iter()
            .filter_map(|run| run.errors.get(*parameter).copied())
            .collect::<Vec<_>>();
        match (median(&values), percentile_nearest_rank(&values, 95.0)) {
            (Some(median), Some(p95)) => {
                notices.push(format!(
                    "{parameter}: HOLDOUT n={} median_abs_relative_error={median:.4}% p95={p95:.4}%",
                    values.len()
                ));
                if median > 5.0 || p95 > 10.0 {
                    notices.push(format!(
                        "AGGREGATE TARGET MISSED: {parameter} requires median <=5% and P95 <=10%"
                    ));
                }
            }
            _ => notices.push(format!("{parameter}: no qualified HOLDOUT estimates")),
        }
    }
    if !holdout.is_empty() {
        let zvs = holdout.iter().map(|run| run.zvs_coverage_pct).collect::<Vec<_>>();
        let overhead = holdout
            .iter()
            .map(|run| run.excitation_overhead_pct)
            .collect::<Vec<_>>();
        let latency = holdout
            .iter()
            .map(|run| run.p99_latency_ms)
            .collect::<Vec<_>>();
        if let (Some(zvs), Some(overhead), Some(latency)) = (
            median(&zvs),
            percentile_nearest_rank(&overhead, 95.0),
            percentile_nearest_rank(&latency, 95.0),
        ) {
            notices.push(format!(
                "HOLDOUT control: median_zvs_coverage={zvs:.4}% p95_excitation_overhead={overhead:.4}% p95_p99_latency={latency:.4}ms"
            ));
        }
    }
    notices
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "wpt-self-identification-evaluator".into());
    let Some(path) = args.next() else {
        eprintln!("usage: {program} <self-identification-runs.jsonl>");
        process::exit(2);
    };
    if args.next().is_some() {
        eprintln!("error: expected exactly one JSONL path");
        process::exit(2);
    }
    let content = fs::read_to_string(&path)?;
    let mut qualified = Vec::new();
    let mut errors_found = false;
    let mut records: Vec<(usize, Value)> = Vec::new();

    // Accept either one pretty-printed JSON run manifest or a JSONL batch.
    // A JSONL file with multiple objects fails whole-file JSON parsing and is
    // then parsed one non-empty line at a time.
    match serde_json::from_str::<Value>(&content) {
        Ok(value) => records.push((1, value)),
        Err(_) => {
            for (line_no, line) in content.lines().enumerate() {
                if line.trim().is_empty() || line.trim_start().starts_with('#') {
                    continue;
                }
                match serde_json::from_str::<Value>(line) {
                    Ok(value) => records.push((line_no + 1, value)),
                    Err(error) => {
                        eprintln!("line {}: invalid JSON: {error}", line_no + 1);
                        errors_found = true;
                    }
                }
            }
        }
    }
    let records_seen = records.len();
    for (line_no, run) in records {
        let mut errors = validate_structure(&run);
        let disposition = string_at(&run, &["qualification"]).unwrap_or("INVALID");
        if errors.is_empty() && disposition == "QUALIFIED" {
            match validate_qualified(&run) {
                Ok(metrics) => {
                    println!(
                        "ACCEPT {} split={} operating_point={} zvs={:.3}% overhead={:.4}% p99={:.3}ms",
                        metrics.run_id,
                        metrics.split,
                        metrics.operating_point_id,
                        metrics.zvs_coverage_pct,
                        metrics.excitation_overhead_pct,
                        metrics.p99_latency_ms
                    );
                    qualified.push(metrics);
                }
                Err(run_errors) => {
                    errors.extend(run_errors);
                }
            }
        } else if errors.is_empty() {
            println!("RECORD {} disposition={disposition}", string_at(&run, &["run_id"]).unwrap_or("UNKNOWN"));
            if disposition == "NOT_RUN" {
                println!("  template only; no measurement or control qualification is implied");
            }
        }
        if !errors.is_empty() {
            eprintln!("REJECT line {}: {} issue(s)", line_no, errors.len());
            for error in errors {
                eprintln!("  - {error}");
            }
            errors_found = true;
        }
    }
    for notice in evaluate_batch(&qualified) {
        println!("{notice}");
    }
    if errors_found {
        process::exit(1);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn template() -> Value {
        serde_json::from_str(include_str!(
            "../../../benchmarks/wpt-empirical/v1/self-identification-run.example.json"
        ))
        .expect("NOT_RUN template parses")
    }

    fn qualified_fixture() -> Value {
        let mut run = template();
        run["run_id"] = json!("SELF-ID-KNOWN-TRUTH-001");
        run["profile_id"] = json!("SYNTHETIC-PROFILE");
        run["system"]["system_id"] = json!("SYNTHETIC-WPT-SYSTEM");
        run["system"]["hardware_revision"] = json!("rev-A");
        run["system"]["controller_revision"] = json!("synthetic-controller-commit");
        run["system"]["configuration_sha256"] = json!("a".repeat(64));
        run["system"]["power_rating_w"] = json!(11_000.0);
        run["system"]["topology"] = json!("synthetic SS compensated IPT");

        run["conditions"]["split"] = json!("HOLDOUT");
        run["conditions"]["operating_point_id"] = json!("GRID-85KHZ-GAP160-ALIGN0-LOAD400");
        run["conditions"]["direction"] = json!("GRID_TO_VEHICLE");
        run["conditions"]["frequency_hz"] = json!(85_000.0);
        run["conditions"]["transfer_gap_m"] = json!(0.16);
        run["conditions"]["offset_x_m"] = json!(0.0);
        run["conditions"]["offset_y_m"] = json!(0.0);
        run["conditions"]["offset_z_m"] = json!(0.0);
        run["conditions"]["yaw_deg"] = json!(0.0);
        run["conditions"]["load_resistance_ohm"] = json!(4.0);
        run["conditions"]["output_power_w"] = json!(11_000.0);
        run["conditions"]["ambient_temperature_c"] = json!(23.0);
        run["conditions"]["coil_temperature_c"] = json!(64.0);
        run["conditions"]["thermal_state"] = json!("THERMAL_SOAK");

        run["reference"]["instrument_id"] = json!("REF-LCR-001");
        run["reference"]["instrument_model"] = json!("synthetic reference impedance analyser");
        run["reference"]["method"] = json!("NETWORK_ANALYZER");
        run["reference"]["calibration_id"] = json!("CAL-REF-001");
        run["reference"]["calibration_valid_from_utc"] = json!("2026-01-01T00:00:00Z");
        run["reference"]["calibration_due_utc"] = json!("2027-01-01T00:00:00Z");
        run["reference"]["measurement_uncertainty_pct"] = json!(0.25);
        run["reference"]["window_start_utc"] = json!("2026-10-10T10:00:00Z");
        run["reference"]["window_end_utc"] = json!("2026-10-10T10:00:10Z");
        run["reference"]["reference_artifact_sha256"] = json!("b".repeat(64));

        run["identification"]["method"] = json!("MULTISINE_PHASE_PERTURBATION");
        run["identification"]["secondary_data_link_required"] = json!(false);
        run["identification"]["perturbation_energy_j"] = json!(10.0);
        run["identification"]["useful_transfer_energy_j"] = json!(10_000.0);
        run["identification"]["reported_overhead_pct"] = json!(0.1);
        run["identification"]["p99_latency_ms"] = json!(2.0);
        run["identification"]["control_deadline_ms"] = json!(10.0);
        run["identification"]["sample_count"] = json!(100);
        run["identification"]["batch_id"] = json!("SYNTHETIC-HOLDOUT-BATCH");

        run["estimates"] = json!([
            {"parameter":"L_PRIMARY","unit":"H","estimate_value":0.000101,"reference_value":0.0001,"relative_error_pct":1.0,"estimate_uncertainty_pct":0.3,"reference_evidence_kind":"MEASURED","estimate_artifact_sha256":"c".repeat(64)},
            {"parameter":"L_SECONDARY","unit":"H","estimate_value":0.000202,"reference_value":0.0002,"relative_error_pct":1.0,"estimate_uncertainty_pct":0.3,"reference_evidence_kind":"MEASURED","estimate_artifact_sha256":"d".repeat(64)},
            {"parameter":"MUTUAL_INDUCTANCE","unit":"H","estimate_value":0.0000808,"reference_value":0.00008,"relative_error_pct":1.0,"estimate_uncertainty_pct":0.4,"reference_evidence_kind":"MEASURED","estimate_artifact_sha256":"e".repeat(64)},
            {"parameter":"EQUIVALENT_LOAD_RESISTANCE","unit":"OHM","estimate_value":4.04,"reference_value":4.0,"relative_error_pct":1.0,"estimate_uncertainty_pct":0.5,"reference_evidence_kind":"MEASURED","estimate_artifact_sha256":"f".repeat(64)}
        ]);

        run["control"]["zvs_observation_count"] = json!(10_000);
        run["control"]["zvs_success_count"] = json!(9_999);
        run["control"]["reported_zvs_coverage_pct"] = json!(99.99);
        run["control"]["switching_limit_violation_count"] = json!(0);
        run["control"]["protection_trip_count"] = json!(0);
        run["control"]["unidentifiable_state_fallback"] = json!("REDUCE_POWER");
        run["control"]["uncertainty_outside_grid_rejected"] = json!(true);
        run["control"]["authority_plane_independent"] = json!(true);
        run["control"]["estimator_can_authorize_energization"] = json!(false);

        run["artifacts"] = json!([
            {"artifact_type":"RAW_WAVEFORM","uri":"artifact://synthetic/raw","sha256":"a".repeat(64),"description":"Synthetic waveform fixture only."},
            {"artifact_type":"REFERENCE_MEASUREMENT","uri":"artifact://synthetic/reference","sha256":"b".repeat(64),"description":"Synthetic reference fixture only."},
            {"artifact_type":"ESTIMATOR_OUTPUT","uri":"artifact://synthetic/estimate-lp","sha256":"c".repeat(64),"description":"Synthetic Lp estimate bundle only."},
            {"artifact_type":"ESTIMATOR_OUTPUT","uri":"artifact://synthetic/estimate-ls","sha256":"d".repeat(64),"description":"Synthetic Ls estimate bundle only."},
            {"artifact_type":"ESTIMATOR_OUTPUT","uri":"artifact://synthetic/estimate-m","sha256":"e".repeat(64),"description":"Synthetic M estimate bundle only."},
            {"artifact_type":"ESTIMATOR_OUTPUT","uri":"artifact://synthetic/estimate-req","sha256":"f".repeat(64),"description":"Synthetic Req estimate bundle only."},
            {"artifact_type":"CONTROLLER_CONFIG","uri":"artifact://synthetic/config","sha256":"a".repeat(64),"description":"Synthetic controller fixture only."}
        ]);
        run["qualification"] = json!("QUALIFIED");
        run
    }

    #[test]
    fn not_run_template_is_valid_and_has_no_measurement_claim() {
        let run = template();
        assert!(validate_structure(&run).is_empty());
        assert_eq!(string_at(&run, &["qualification"]), Some("NOT_RUN"));
        assert!(run["estimates"].as_array().unwrap().is_empty());
    }

    #[test]
    fn synthetic_known_truth_run_passes() {
        let run = qualified_fixture();
        assert!(validate_structure(&run).is_empty());
        assert!(validate_qualified(&run).is_ok());
    }

    #[test]
    fn rejects_reported_relative_error_that_does_not_match_values() {
        let mut run = qualified_fixture();
        run["estimates"][0]["relative_error_pct"] = json!(0.1);
        let errors = validate_qualified(&run).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("relative_error_pct: calculated")));
    }

    #[test]
    fn rejects_calibration_that_expires_before_test() {
        let mut run = qualified_fixture();
        run["reference"]["calibration_due_utc"] = json!("2026-10-09T00:00:00Z");
        let errors = validate_qualified(&run).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("calibration validity")));
    }

    #[test]
    fn rejects_zvs_coverage_below_proposed_target() {
        let mut run = qualified_fixture();
        run["control"]["zvs_success_count"] = json!(9_800);
        run["control"]["reported_zvs_coverage_pct"] = json!(98.0);
        let errors = validate_qualified(&run).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("below proposed 99.000%")));
    }

    #[test]
    fn rejects_perturbation_energy_above_proposed_overhead_target() {
        let mut run = qualified_fixture();
        run["identification"]["perturbation_energy_j"] = json!(100.0);
        run["identification"]["reported_overhead_pct"] = json!(1.0);
        let errors = validate_qualified(&run).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("exceeds proposed 0.500%")));
    }

    #[test]
    fn rejects_estimator_that_can_authorize_energization() {
        let mut run = qualified_fixture();
        run["control"]["estimator_can_authorize_energization"] = json!(true);
        let errors = validate_qualified(&run).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("must never authorize energization")));
    }

    #[test]
    fn rejects_zvs_switching_limit_violation() {
        let mut run = qualified_fixture();
        run["control"]["switching_limit_violation_count"] = json!(1);
        let errors = validate_qualified(&run).unwrap_err();
        assert!(errors.iter().any(|e| e.contains("switching_limit_violation_count")));
    }

    #[test]
    fn holdout_batch_requires_thirty_unique_operating_points() {
        let mut metrics = Vec::new();
        for index in 0..29 {
            metrics.push(RunMetrics {
                run_id: format!("RUN-{index:03}"),
                split: "HOLDOUT".into(),
                operating_point_id: format!("POINT-{index:03}"),
                batch_id: "BATCH-A".into(),
                errors: PARAMETERS.iter().map(|parameter| ((*parameter).to_owned(), 1.0)).collect(),
                zvs_coverage_pct: 99.9,
                excitation_overhead_pct: 0.1,
                p99_latency_ms: 1.0,
            });
        }
        let result = evaluate_batch(&metrics);
        assert!(result.iter().any(|message| message.contains("at least 30 are required")));
    }

    #[test]
    fn rejects_invalid_utc_calendar_date() {
        assert!(parse_utc_seconds("2026-02-29T00:00:00Z").is_none());
        assert!(parse_utc_seconds("2024-02-29T00:00:00Z").is_some());
    }
}
