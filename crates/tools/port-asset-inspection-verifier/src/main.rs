// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn value_at<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = root;
    for part in path.split('.') {
        current = match current {
            Value::Object(object) => object.get(part)?,
            Value::Array(array) => array.get(part.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(current)
}

fn text_at<'a>(root: &'a Value, path: &str) -> Option<&'a str> {
    value_at(root, path)?.as_str()
}

fn number_at(root: &Value, path: &str) -> Option<u64> {
    value_at(root, path)?.as_u64()
}

fn float_at(root: &Value, path: &str) -> Option<f64> {
    value_at(root, path)?.as_f64()
}

fn bool_at(root: &Value, path: &str) -> Option<bool> {
    value_at(root, path)?.as_bool()
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

    for (start, end) in [(0, 4), (5, 7), (8, 10), (11, 13), (14, 16), (17, 19)] {
        if !bytes[start..end].iter().all(u8::is_ascii_digit) {
            return None;
        }
    }

    let year = value[0..4].parse::<i64>().ok()?;
    let month = value[5..7].parse::<i64>().ok()?;
    let day = value[8..10].parse::<i64>().ok()?;
    let hour = value[11..13].parse::<i64>().ok()?;
    let minute = value[14..16].parse::<i64>().ok()?;
    let second = value[17..19].parse::<i64>().ok()?;

    if year == 0
        || !(1..=12).contains(&month)
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute)
        || !(0..=59).contains(&second)
    {
        return None;
    }

    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return None,
    };
    if !(1..=days_in_month).contains(&day) {
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
    let days_since_epoch = era * 146_097 + day_of_era - 719_468;

    Some(
        days_since_epoch * 86_400
            + hour * 3_600
            + minute * 60
            + second,
    )
}

fn timestamp_at(root: &Value, path: &str) -> Option<i64> {
    parse_utc_seconds(text_at(root, path)?)
}

fn evaluate(case: &Value) -> &'static str {
    let cutoff = match timestamp_at(case, "evaluation_cutoff_utc") {
        Some(value) => value,
        None => return "INVALID_DECISION_CUTOFF",
    };

    if text_at(case, "mission.status") != Some("completed_synthetic") {
        return "MISSION_NOT_COMPLETE";
    }
    if text_at(case, "mission.rights_review_status") != Some("synthetic_not_applicable")
        && text_at(case, "mission.rights_review_status") != Some("reviewed_permitted")
    {
        return "DATA_RIGHTS_NOT_REVIEWED";
    }

    let mission_start = match timestamp_at(case, "mission.started_at_utc") {
        Some(value) => value,
        None => return "INVALID_MISSION_TIME",
    };
    let mission_end = match timestamp_at(case, "mission.completed_at_utc") {
        Some(value) => value,
        None => return "INVALID_MISSION_TIME",
    };
    if mission_start > mission_end || mission_end > cutoff {
        return "INVALID_MISSION_TIME";
    }

    let asset_id = match text_at(case, "asset.asset_id") {
        Some(value) => value,
        None => return "ASSET_IDENTITY_MISMATCH",
    };
    let feature_id = match text_at(case, "asset.feature_id") {
        Some(value) => value,
        None => return "FEATURE_IDENTITY_MISMATCH",
    };
    let asset_revision = match text_at(case, "asset.asset_revision") {
        Some(value) => value,
        None => return "ASSET_REVISION_MISMATCH",
    };
    let geometry_revision = match text_at(case, "asset.geometry_revision") {
        Some(value) => value,
        None => return "GEOMETRY_REVISION_MISSING",
    };

    if text_at(case, "mission.required_asset_feature_id") != Some(feature_id) {
        return "FEATURE_IDENTITY_MISMATCH";
    }

    let horizontal_crs = text_at(case, "asset.horizontal_crs");
    if horizontal_crs.is_none_or(str::is_empty)
        || text_at(case, "spatial_reference.horizontal_crs") != horizontal_crs
    {
        return "SPATIAL_REFERENCE_MISSING";
    }

    let vertical_datum = text_at(case, "asset.vertical_datum");
    if vertical_datum.is_none_or(str::is_empty)
        || text_at(case, "spatial_reference.vertical_datum") != vertical_datum
    {
        return "SPATIAL_REFERENCE_MISSING";
    }

    if text_at(case, "spatial_reference.localization_status") != Some("qualified_synthetic") {
        return "LOCALIZATION_UNQUALIFIED";
    }
    if float_at(case, "spatial_reference.pose_error_m")
        .zip(float_at(case, "spatial_reference.max_pose_error_m"))
        .is_none_or(|(observed, maximum)| observed > maximum)
    {
        return "LOCALIZATION_OUT_OF_TOLERANCE";
    }

    if text_at(case, "spatial_reference.geometry_registration_status")
        != Some("registered_synthetic")
        || text_at(case, "coverage.registered_geometry_revision") != Some(geometry_revision)
    {
        return "GEOMETRY_REGISTRATION_MISMATCH";
    }

    let optical_visibility = text_at(case, "environment.optical_visibility_state");
    if optical_visibility == Some("outside_validated_envelope") {
        let sensors = value_at(case, "sensors").and_then(Value::as_array);
        if sensors.is_some_and(|items| {
            items.iter().any(|sensor| {
                sensor.get("modality").and_then(Value::as_str) == Some("optical_imaging")
            })
        }) {
            return "SENSOR_ENVIRONMENT_OUT_OF_DOMAIN";
        }
    }

    let sensors = match value_at(case, "sensors").and_then(Value::as_array) {
        Some(values) => values,
        None => return "SENSOR_SET_MISSING",
    };
    for sensor in sensors {
        let sensor_id = match sensor.get("sensor_id").and_then(Value::as_str) {
            Some(value) => value,
            None => return "SENSOR_METADATA_INCOMPLETE",
        };
        let calibration_valid_until = match sensor
            .get("calibration_valid_until_utc")
            .and_then(Value::as_str)
            .and_then(parse_utc_seconds)
        {
            Some(value) => value,
            None => return "SENSOR_CALIBRATION_EXPIRED",
        };
        if sensor.get("calibration_state").and_then(Value::as_str) != Some("current")
            || calibration_valid_until <= cutoff
        {
            return "SENSOR_CALIBRATION_EXPIRED";
        }
        if sensor.get("health_state").and_then(Value::as_str) != Some("nominal_synthetic") {
            return "SENSOR_HEALTH_UNKNOWN";
        }
        if sensor.get("resolution_m").and_then(Value::as_f64).is_none_or(|v| v <= 0.0)
            || sensor.get("max_range_m").and_then(Value::as_f64).is_none_or(|v| v <= 0.0)
        {
            return "SENSOR_METADATA_INCOMPLETE";
        }
        if sensor.get("platform_id").and_then(Value::as_str)
            != text_at(case, "mission.platform_id")
        {
            return "OBSERVATION_SENSOR_UNBOUND";
        }
        if sensor_id.is_empty() {
            return "SENSOR_METADATA_INCOMPLETE";
        }
    }

    let observations = match value_at(case, "observations").and_then(Value::as_array) {
        Some(values) => values,
        None => return "OBSERVATION_SET_MISSING",
    };
    let mut observation_ids = BTreeSet::new();
    let mut independence_groups = BTreeSet::new();
    let mut result_classes = BTreeSet::new();

    let max_observation_age = match number_at(case, "mission.max_observation_age_s") {
        Some(value) => value as i64,
        None => return "OBSERVATION_FRESHNESS_POLICY_MISSING",
    };

    for observation in observations {
        let observation_id = match observation.get("observation_id").and_then(Value::as_str) {
            Some(value) => value,
            None => return "DUPLICATE_OBSERVATION_ID",
        };
        if !observation_ids.insert(observation_id) {
            return "DUPLICATE_OBSERVATION_ID";
        }

        let sensor_id = match observation.get("sensor_id").and_then(Value::as_str) {
            Some(value) => value,
            None => return "OBSERVATION_SENSOR_UNBOUND",
        };
        let sensor = sensors
            .iter()
            .find(|sensor| sensor.get("sensor_id").and_then(Value::as_str) == Some(sensor_id));
        let Some(sensor) = sensor else {
            return "OBSERVATION_SENSOR_UNBOUND";
        };

        if observation.get("mission_id").and_then(Value::as_str)
            != text_at(case, "mission.mission_id")
            || observation.get("asset_id").and_then(Value::as_str) != Some(asset_id)
        {
            return "ASSET_IDENTITY_MISMATCH";
        }
        if observation.get("asset_feature_id").and_then(Value::as_str) != Some(feature_id) {
            return "FEATURE_IDENTITY_MISMATCH";
        }
        if observation.get("asset_revision").and_then(Value::as_str) != Some(asset_revision) {
            return "ASSET_REVISION_MISMATCH";
        }
        if observation.get("geometry_revision").and_then(Value::as_str) != Some(geometry_revision) {
            return "GEOMETRY_REGISTRATION_MISMATCH";
        }

        let source_time = match observation.get("source_time_utc")
            .and_then(Value::as_str)
            .and_then(parse_utc_seconds)
        {
            Some(value) => value,
            None => return "TEMPORAL_ROLE_INCONSISTENCY",
        };
        let retrieved_time = match observation.get("retrieved_at_utc")
            .and_then(Value::as_str)
            .and_then(parse_utc_seconds)
        {
            Some(value) => value,
            None => return "TEMPORAL_ROLE_INCONSISTENCY",
        };
        if retrieved_time < source_time || source_time > cutoff {
            return "TEMPORAL_ROLE_INCONSISTENCY";
        }
        if cutoff - source_time > max_observation_age {
            return "OBSERVATION_STALE";
        }

        if observation.get("position_reference").and_then(Value::as_str)
            == Some("unknown")
        {
            return "LOCALIZATION_UNQUALIFIED";
        }
        if observation.get("pose_error_m").and_then(Value::as_f64)
            .zip(float_at(case, "spatial_reference.max_pose_error_m"))
            .is_none_or(|(observed, maximum)| observed > maximum)
        {
            return "LOCALIZATION_OUT_OF_TOLERANCE";
        }

        let occlusion = observation.get("occlusion_state").and_then(Value::as_str);
        let result = observation.get("result_class").and_then(Value::as_str);
        if occlusion != Some("clear_synthetic") && result == Some("no_indication") {
            return "OCCLUDED_REGION_NOT_CLEAR";
        }

        let evidence_class = observation.get("evidence_class").and_then(Value::as_str);
        let modality = sensor.get("modality").and_then(Value::as_str);
        if evidence_class == Some("qualified_ndt")
            && !matches!(modality, Some("ultrasonic_thickness" | "laser_metrology"))
        {
            return "EVIDENCE_CLASS_MISMATCH";
        }

        if let Some(group) = observation.get("independence_group").and_then(Value::as_str) {
            independence_groups.insert(group);
        }
        if let Some(result) = result {
            result_classes.insert(result);
        }
    }

    let required_coverage = match float_at(case, "coverage.required_fraction") {
        Some(value) => value,
        None => return "INSPECTION_COVERAGE_INCOMPLETE",
    };
    let observed_coverage = match float_at(case, "coverage.observed_fraction") {
        Some(value) => value,
        None => return "INSPECTION_COVERAGE_INCOMPLETE",
    };
    if text_at(case, "coverage.coverage_status") != Some("complete_synthetic")
        || observed_coverage < required_coverage
    {
        return "INSPECTION_COVERAGE_INCOMPLETE";
    }

    let bathymetry_datum = text_at(case, "bathymetry.vertical_datum");
    if bathymetry_datum != vertical_datum {
        return "VERTICAL_DATUM_MISMATCH";
    }
    if text_at(case, "bathymetry.product") != Some("S-102")
        || text_at(case, "bathymetry.coverage_status") != Some("complete_synthetic")
        || float_at(case, "bathymetry.coverage_fraction").is_none_or(|v| v < 0.99)
    {
        return "BATHYMETRY_COVERAGE_INSUFFICIENT";
    }
    if number_at(case, "bathymetry.sample_count")
        .zip(number_at(case, "bathymetry.minimum_samples"))
        .is_none_or(|(samples, minimum)| samples < minimum)
    {
        return "BATHYMETRY_SAMPLE_SUPPORT_INSUFFICIENT";
    }
    if number_at(case, "bathymetry.survey_sequence")
        .zip(number_at(case, "bathymetry.latest_survey_sequence"))
        .is_some_and(|(current, latest)| current < latest)
    {
        return "SURVEY_REVISION_OUT_OF_ORDER";
    }

    if text_at(case, "assessment.indication_type") == Some("corrosion_thickness_loss") {
        let has_thickness = observations.iter().any(|observation| {
            let sensor_id = observation.get("sensor_id").and_then(Value::as_str);
            let sensor = sensors.iter().find(|item| {
                item.get("sensor_id").and_then(Value::as_str) == sensor_id
            });
            sensor.is_some_and(|item| {
                matches!(
                    item.get("modality").and_then(Value::as_str),
                    Some("ultrasonic_thickness" | "laser_metrology")
                ) && observation.get("measured_value").and_then(Value::as_f64).is_some()
            })
        });
        if !has_thickness {
            return "THICKNESS_EVIDENCE_MISSING";
        }
    }

    if text_at(case, "assessment.claim_type") == Some("structural_capacity") {
        return "STRUCTURAL_CLAIM_UNSUPPORTED";
    }
    if text_at(case, "assessment.confidence_status") != Some("calibrated_synthetic") {
        return "CONFIDENCE_NOT_CALIBRATED";
    }
    if number_at(case, "assessment.state_age_s")
        .zip(number_at(case, "assessment.max_state_age_s"))
        .is_none_or(|(age, maximum)| age > maximum)
    {
        return "ASSESSMENT_STALE";
    }

    if result_classes.len() > 1
        && text_at(case, "assessment.fusion_method") == Some("naive_average")
    {
        return "CONFLICTING_OBSERVATIONS_COLLAPSED";
    }

    if bool_at(case, "assessment.claims_independent_corroboration") == Some(true)
        && independence_groups.len() < observations.len()
    {
        return "FALSE_INDEPENDENCE_CLAIM";
    }

    if bool_at(case, "independent_reference.available") != Some(true)
        || bool_at(case, "independent_reference.blind") != Some(true)
    {
        return "INDEPENDENT_REFERENCE_MISSING";
    }
    if text_at(case, "independent_reference.asset_feature_id") != Some(feature_id)
        || text_at(case, "independent_reference.asset_revision") != Some(asset_revision)
    {
        return "REFERENCE_ASSET_MISMATCH";
    }

    if text_at(case, "independent_reference.disposition") == Some("no_indication")
        && text_at(case, "maintenance.recommendation") == Some("create_work_order")
    {
        return "UNSUPPORTED_WORK_ORDER_RECOMMENDATION";
    }
    if text_at(case, "independent_reference.disposition")
        == Some("indication_confirmed_synthetic")
        && text_at(case, "assessment.disposition") == Some("no_indication")
    {
        return "REFERENCE_ASSESSMENT_CONFLICT";
    }

    if bool_at(case, "maintenance.maintenance_authorized") == Some(true)
        && (text_at(case, "maintenance.approval_status") != Some("approved")
            || text_at(case, "maintenance.approval_ref").is_none_or(str::is_empty)
            || text_at(case, "maintenance.work_order_ref").is_none_or(str::is_empty))
    {
        return "MAINTENANCE_AUTHORITY_MISSING";
    }
    if bool_at(case, "maintenance.repair_required") == Some(true)
        && bool_at(case, "maintenance.repair_completed") == Some(true)
        && text_at(case, "maintenance.post_action_verification") != Some("verified_synthetic")
    {
        return "POST_REPAIR_VERIFICATION_MISSING";
    }
    if bool_at(case, "maintenance.source_writeback") != Some(false) {
        return "SOURCE_WRITEBACK_FORBIDDEN";
    }

    if number_at(case, "commissioning.observed_latency_ms")
        .zip(number_at(case, "commissioning.max_latency_ms"))
        .is_none_or(|(observed, maximum)| observed > maximum)
    {
        return "TWIN_LATENCY_EXCEEDED";
    }
    if text_at(case, "commissioning.projection_write_mode") != Some("read_only") {
        return "SOURCE_WRITEBACK_FORBIDDEN";
    }

    "INSPECTION_CONTRACT_VALID_SYNTHETIC_ONLY"
}

fn apply_mutation(root: &mut Value, mutation: &Value) -> Result<(), String> {
    let path = mutation
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| "mutation has no path".to_owned())?;
    let operation = mutation
        .get("op")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("mutation {path} has no operation"))?;
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return Err("empty mutation path".to_owned());
    }

    let mut current = root;
    for part in &parts[..parts.len() - 1] {
        current = match current {
            Value::Object(object) => object
                .get_mut(*part)
                .ok_or_else(|| format!("missing mutation path prefix {path}"))?,
            Value::Array(array) => array
                .get_mut(part.parse::<usize>()
                    .map_err(|_| format!("invalid array index in mutation path {path}"))?)
                .ok_or_else(|| format!("missing array index in mutation path {path}"))?,
            _ => return Err(format!("non-container in mutation path {path}")),
        };
    }

    let leaf = parts[parts.len() - 1];
    match current {
        Value::Object(object) => match operation {
            "set" => {
                let value = mutation
                    .get("value")
                    .ok_or_else(|| format!("set mutation {path} has no value"))?;
                let slot = object
                    .get_mut(leaf)
                    .ok_or_else(|| format!("missing mutation leaf {path}"))?;
                *slot = value.clone();
                Ok(())
            }
            "remove" => object
                .remove(leaf)
                .map(|_| ())
                .ok_or_else(|| format!("missing mutation leaf {path}")),
            _ => Err(format!("unsupported mutation operation {operation}")),
        },
        Value::Array(array) => {
            let index = leaf
                .parse::<usize>()
                .map_err(|_| format!("invalid array leaf in mutation path {path}"))?;
            match operation {
                "set" => {
                    let value = mutation
                        .get("value")
                        .ok_or_else(|| format!("set mutation {path} has no value"))?;
                    let slot = array
                        .get_mut(index)
                        .ok_or_else(|| format!("missing array leaf {path}"))?;
                    *slot = value.clone();
                    Ok(())
                }
                "remove" => {
                    if index >= array.len() {
                        return Err(format!("missing array leaf {path}"));
                    }
                    array.remove(index);
                    Ok(())
                }
                _ => Err(format!("unsupported mutation operation {operation}")),
            }
        }
        _ => Err(format!("non-container at mutation leaf {path}")),
    }
}

fn verify_corpus(corpus: &Value) -> Result<Value, String> {
    if corpus.get("corpus_id").and_then(Value::as_str)
        != Some("luminous.symtropy.port-asset-inspection")
        || corpus.get("corpus_version").and_then(Value::as_str) != Some("0.1.0")
    {
        return Err("unsupported port asset inspection corpus identity/version".to_owned());
    }

    let base = corpus
        .get("base_cases")
        .and_then(|value| value.get("valid_quay_inspection"))
        .ok_or_else(|| "corpus has no valid_quay_inspection base case".to_owned())?;
    let cases = corpus
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| "corpus cases must be an array".to_owned())?;

    let mut ids = BTreeSet::new();
    let mut results = Vec::with_capacity(cases.len());
    let mut failures = Vec::new();

    for case in cases {
        let id = case
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| "case has no id".to_owned())?;
        if !ids.insert(id) {
            return Err(format!("duplicate case id {id}"));
        }
        if case.get("base").and_then(Value::as_str) != Some("valid_quay_inspection") {
            return Err(format!("case {id} references an unsupported base"));
        }
        let expected = case
            .get("expected_code")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("case {id} has no expected_code"))?;
        let mutations = case
            .get("mutations")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("case {id} mutations must be an array"))?;

        let mut input = base.clone();
        for mutation in mutations {
            apply_mutation(&mut input, mutation)
                .map_err(|error| format!("case {id}: {error}"))?;
        }

        let observed = evaluate(&input);
        let passed = observed == expected;
        if !passed {
            failures.push(json!({
                "case_id": id,
                "expected": expected,
                "observed": observed
            }));
        }
        results.push(json!({
            "case_id": id,
            "expected": expected,
            "observed": observed,
            "passed": passed
        }));
    }

    Ok(json!({
        "status": if failures.is_empty() { "PASS_SYNTHETIC_CORPUS" } else { "FAIL_SYNTHETIC_CORPUS" },
        "corpus_id": corpus.get("corpus_id").and_then(Value::as_str),
        "corpus_version": corpus.get("corpus_version").and_then(Value::as_str),
        "case_count": cases.len(),
        "passed_count": results.iter().filter(|item| item["passed"] == true).count(),
        "failed_count": failures.len(),
        "failures": failures,
        "results": results,
        "claim_ceiling": corpus.get("claim_ceiling").and_then(Value::as_str)
    }))
}

fn main() -> ExitCode {
    let path = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("benchmarks/maritime/port-asset-inspection-corpus-v0.1.json"));
    let bytes = match fs::read(&path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read {}: {error}", path.display());
            return ExitCode::FAILURE;
        }
    };
    let corpus: Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid corpus JSON: {error}");
            return ExitCode::FAILURE;
        }
    };
    let report = match verify_corpus(&corpus) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("corpus verification could not run: {error}");
            return ExitCode::FAILURE;
        }
    };
    println!("{}", serde_json::to_string_pretty(&report).expect("JSON report serializes"));
    if report["failed_count"].as_u64() == Some(0) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::{evaluate, verify_corpus};
    use serde_json::Value;

    const CORPUS: &str =
        include_str!("../../../../benchmarks/maritime/port-asset-inspection-corpus-v0.1.json");

    #[test]
    fn frozen_corpus_matches_all_expected_dispositions() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("frozen corpus parses");
        let report = verify_corpus(&corpus).expect("frozen corpus evaluates");
        assert_eq!(report["case_count"].as_u64(), Some(39));
        assert_eq!(report["passed_count"].as_u64(), Some(39));
        assert_eq!(report["failed_count"].as_u64(), Some(0));
    }

    #[test]
    fn valid_base_is_synthetic_only() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("frozen corpus parses");
        let base = corpus["base_cases"]["valid_quay_inspection"].clone();
        assert_eq!(evaluate(&base), "INSPECTION_CONTRACT_VALID_SYNTHETIC_ONLY");
    }

    #[test]
    fn utc_parser_rejects_impossible_calendar_dates() {
        assert!(parse_utc_seconds("2024-02-29T23:59:59Z").is_some());
        assert!(parse_utc_seconds("2026-11-10T08:00:00Z").is_some());
        assert!(parse_utc_seconds("2026-02-30T08:00:00Z").is_none());
        assert!(parse_utc_seconds("2025-02-29T08:00:00Z").is_none());
        assert!(parse_utc_seconds("2026-11-10T24:00:00Z").is_none());
        assert!(parse_utc_seconds("2026-11-10T08:00:00").is_none());
    }
}
