// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::Value;
use std::env;
use std::error::Error;
use std::fs;
use std::process;

const TOP_KEYS: &[&str] = &[
    "schema_version",
    "compatibility_id",
    "vehicle_assembly",
    "ground_assembly",
    "installation",
    "geometry_domain",
    "sampling",
    "coupling_range_metric",
    "evidence",
    "qualification",
    "notes",
];

fn at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(value, |current, key| current.get(*key))
}

fn string_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    at(value, path).and_then(Value::as_str)
}

fn number_at(value: &Value, path: &[&str]) -> Option<f64> {
    at(value, path).and_then(Value::as_f64)
}

fn nonempty(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.trim().is_empty())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn exact_keys(value: Option<&Value>, expected: &[&str], path: &str, errors: &mut Vec<String>) {
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

fn ordered_range(value: &Value, min_key: &str, max_key: &str, path: &str, errors: &mut Vec<String>) {
    match (
        value.get(min_key).and_then(Value::as_f64),
        value.get(max_key).and_then(Value::as_f64),
    ) {
        (Some(minimum), Some(maximum)) if minimum <= maximum => {}
        (Some(_), Some(_)) => errors.push(format!("{path}.{min_key}/{max_key}: minimum exceeds maximum")),
        (None, None) => {}
        _ => errors.push(format!("{path}.{min_key}/{max_key}: range endpoints must both be known or both be null")),
    }
}

fn validate_record(record: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    exact_keys(Some(record), TOP_KEYS, "$", &mut errors);

    if string_at(record, &["schema_version"]) != Some("wpt-geometry-compatibility-v1") {
        errors.push("schema_version: expected wpt-geometry-compatibility-v1".into());
    }
    if !nonempty(string_at(record, &["compatibility_id"])) {
        errors.push("compatibility_id: nonempty stable identifier required".into());
    }
    if !nonempty(string_at(record, &["notes"])) {
        errors.push("notes: nonempty claim and limitation statement required".into());
    }

    exact_keys(
        record.get("vehicle_assembly"),
        &["assembly_id", "generation", "power_rating_w", "mounting_type", "coil_geometry", "ferrite_material", "shielding"],
        "vehicle_assembly",
        &mut errors,
    );
    exact_keys(
        record.get("ground_assembly"),
        &["assembly_id", "generation", "power_rating_w", "mounting_type", "coil_geometry", "ferrite_material", "shielding"],
        "ground_assembly",
        &mut errors,
    );
    exact_keys(
        record.get("installation"),
        &["type", "surface_material", "roadway_or_floor_reference"],
        "installation",
        &mut errors,
    );
    exact_keys(
        record.get("geometry_domain"),
        &[
            "coordinate_frame_id",
            "coil_to_coil_air_gap_min_m",
            "coil_to_coil_air_gap_max_m",
            "vehicle_ground_clearance_min_m",
            "vehicle_ground_clearance_max_m",
            "x_offset_min_m",
            "x_offset_max_m",
            "y_offset_min_m",
            "y_offset_max_m",
            "z_offset_min_m",
            "z_offset_max_m",
            "yaw_min_deg",
            "yaw_max_deg",
        ],
        "geometry_domain",
        &mut errors,
    );
    exact_keys(
        record.get("sampling"),
        &[
            "strategy",
            "unique_positions_sampled",
            "evaluation_point_count",
            "inductance_sample_count",
            "open_circuit_sample_count",
            "short_circuit_sample_count",
            "raw_geometry_data_sha256",
            "instrumentation_reference",
        ],
        "sampling",
        &mut errors,
    );
    exact_keys(
        record.get("coupling_range_metric"),
        &[
            "metric_definition_id",
            "metric_name",
            "comparison_direction",
            "relative_change_percent",
            "qualifier",
            "baseline_compatibility_id",
            "baseline_definition",
            "absolute_region_measure",
            "confidence_interval_lower_percent",
            "confidence_interval_upper_percent",
            "notes",
        ],
        "coupling_range_metric",
        &mut errors,
    );
    exact_keys(
        record.get("evidence"),
        &["kind", "method", "source_doi", "source_locator", "uncertainty_reported"],
        "evidence",
        &mut errors,
    );

    let geometry = record.get("geometry_domain").unwrap_or(&Value::Null);
    for (minimum, maximum, label) in [
        ("coil_to_coil_air_gap_min_m", "coil_to_coil_air_gap_max_m", "coil-to-coil air gap"),
        ("vehicle_ground_clearance_min_m", "vehicle_ground_clearance_max_m", "vehicle ground clearance"),
        ("x_offset_min_m", "x_offset_max_m", "X offset"),
        ("y_offset_min_m", "y_offset_max_m", "Y offset"),
        ("z_offset_min_m", "z_offset_max_m", "Z offset"),
        ("yaw_min_deg", "yaw_max_deg", "yaw"),
    ] {
        ordered_range(geometry, minimum, maximum, "geometry_domain", &mut errors);
        if label == "coil-to-coil air gap" || label == "vehicle ground clearance" {
            for field in [minimum, maximum] {
                if geometry.get(field).is_some_and(|value| {
                    value.as_f64().is_some_and(|number| number < 0.0)
                }) {
                    errors.push(format!("geometry_domain.{field}: {label} cannot be negative"));
                }
            }
        }
    }

    let metric = record.get("coupling_range_metric").unwrap_or(&Value::Null);
    if string_at(metric, &["metric_name"]) != Some("MAGNETIC_COUPLING_OPERATING_RANGE") {
        errors.push("coupling_range_metric.metric_name: expected MAGNETIC_COUPLING_OPERATING_RANGE".into());
    }
    if !nonempty(string_at(metric, &["metric_definition_id"])) {
        errors.push("coupling_range_metric.metric_definition_id: definition required".into());
    }
    if !nonempty(string_at(metric, &["notes"])) {
        errors.push("coupling_range_metric.notes: define the metric and its limitations".into());
    }
    if matches!(
        string_at(metric, &["comparison_direction"]),
        Some("REDUCTION" | "INCREASE" | "NO_CHANGE")
    ) && metric.get("relative_change_percent").is_none_or(Value::is_null)
    {
        errors.push("coupling_range_metric.relative_change_percent: numeric value required for directional change".into());
    }
    if let (Some(lower), Some(upper)) = (
        number_at(metric, &["confidence_interval_lower_percent"]),
        number_at(metric, &["confidence_interval_upper_percent"]),
    ) {
        if lower > upper {
            errors.push("coupling_range_metric: confidence interval lower bound exceeds upper bound".into());
        }
    }

    let qualification = string_at(record, &["qualification"]).unwrap_or("INVALID");
    if !matches!(qualification, "NOT_RUN" | "SOURCE_CLAIM_ONLY" | "INCOMPLETE" | "QUALIFIED" | "REJECTED") {
        errors.push("qualification: unsupported disposition".into());
    }

    if qualification == "QUALIFIED" {
        if !nonempty(string_at(geometry, &["coordinate_frame_id"])) {
            errors.push("geometry_domain.coordinate_frame_id: required for QUALIFIED envelope".into());
        }
        for (minimum, maximum) in [
            ("coil_to_coil_air_gap_min_m", "coil_to_coil_air_gap_max_m"),
            ("vehicle_ground_clearance_min_m", "vehicle_ground_clearance_max_m"),
            ("x_offset_min_m", "x_offset_max_m"),
            ("y_offset_min_m", "y_offset_max_m"),
            ("z_offset_min_m", "z_offset_max_m"),
            ("yaw_min_deg", "yaw_max_deg"),
        ] {
            if !number_at(geometry, &[minimum]).is_some() || !number_at(geometry, &[maximum]).is_some() {
                errors.push(format!("geometry_domain.{minimum}/{maximum}: both endpoints required for QUALIFIED envelope"));
            }
        }

        let positions = at(record, &["sampling", "unique_positions_sampled"]).and_then(Value::as_u64);
        let evaluation_points = at(record, &["sampling", "evaluation_point_count"]).and_then(Value::as_u64);
        let inductance_samples = at(record, &["sampling", "inductance_sample_count"]).and_then(Value::as_u64);
        match (positions, evaluation_points, inductance_samples) {
            (Some(positions), Some(points), Some(samples))
                if positions >= 189 && points >= positions && samples >= points => {}
            _ => errors.push("sampling: QUALIFIED envelope requires >=189 unique positions, evaluation points >= positions, and inductance samples >= evaluation points".into()),
        }

        if !string_at(record, &["sampling", "raw_geometry_data_sha256"]).is_some_and(valid_sha256) {
            errors.push("sampling.raw_geometry_data_sha256: valid raw geometry dataset digest required".into());
        }
        if !nonempty(string_at(record, &["sampling", "instrumentation_reference"])) {
            errors.push("sampling.instrumentation_reference: calibration/instrument reference required".into());
        }
        if !matches!(
            string_at(record, &["evidence", "method"]),
            Some("INSTRUMENTED_INDUCTANCE_MEASUREMENT" | "CALCULATION_FROM_MEASURED_COIL_PARAMETERS")
        ) {
            errors.push("evidence.method: QUALIFIED physical envelope requires measured coil parameters".into());
        }
        if !matches!(
            string_at(record, &["evidence", "kind"]),
            Some("MEASURED" | "AUTHOR_CALCULATED")
        ) {
            errors.push("evidence.kind: QUALIFIED physical envelope cannot be simulation-only".into());
        }
        if !at(record, &["evidence", "uncertainty_reported"]).and_then(Value::as_bool).unwrap_or(false) {
            errors.push("evidence.uncertainty_reported: uncertainty budget required for QUALIFIED envelope".into());
        }
        let reduction = number_at(metric, &["relative_change_percent"]);
        let direction = string_at(metric, &["comparison_direction"]);
        let baseline = string_at(metric, &["baseline_compatibility_id"]);
        if reduction.is_some() && matches!(direction, Some("REDUCTION" | "INCREASE")) && !nonempty(baseline) {
            errors.push("coupling_range_metric.baseline_compatibility_id: named baseline required for relative change qualification".into());
        }
    }

    if record.get("efficiency_percent").is_some() {
        errors.push("efficiency_percent: geometry coupling-range schema cannot carry an efficiency score".into());
    }
    errors
}

fn validate_file(path: &str) -> Result<Vec<(String, Vec<String>)>, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    if let Ok(value) = serde_json::from_str::<Value>(&content) {
        return Ok(vec![(path.to_owned(), validate_record(&value))]);
    }

    let mut results = Vec::new();
    for (index, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line)?;
        results.push((format!("{path}:{}", index + 1), validate_record(&value)));
    }
    Ok(results)
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "wpt-geometry-envelope-validator".into());
    let paths = args.collect::<Vec<_>>();
    if paths.is_empty() {
        eprintln!("usage: {program} <geometry-record.jsonl|template.json> [...]");
        process::exit(2);
    }

    let mut total_errors = 0usize;
    for path in &paths {
        for (label, errors) in validate_file(path)? {
            if errors.is_empty() {
                println!("{label}: valid geometry compatibility record.");
            } else {
                total_errors += errors.len();
                eprintln!("{label}: rejected with {} issue(s):", errors.len());
                for error in errors {
                    eprintln!(" - {error}");
                }
            }
        }
    }
    if total_errors > 0 {
        process::exit(1);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn source_record() -> Value {
        serde_json::from_str(include_str!(
            "../../../benchmarks/wpt-empirical/v1/geometry-compatibility.jsonl"
        ))
        .expect("geometry source record parses")
    }

    fn template() -> Value {
        serde_json::from_str(include_str!(
            "../../../benchmarks/wpt-empirical/v1/geometry-compatibility.example.json"
        ))
        .expect("geometry template parses")
    }

    fn qualified_fixture() -> Value {
        let mut value = template();
        value["compatibility_id"] = json!("SYNTHETIC-WPT4-QUALIFIED-PAIR");
        value["vehicle_assembly"]["assembly_id"] = json!("SYNTHETIC-WPT3-VA");
        value["vehicle_assembly"]["generation"] = json!("WPT3");
        value["vehicle_assembly"]["power_rating_w"] = json!(11_000.0);
        value["vehicle_assembly"]["mounting_type"] = json!("VEHICLE_UNDERBODY");
        value["ground_assembly"]["assembly_id"] = json!("SYNTHETIC-WPT4-GA");
        value["ground_assembly"]["generation"] = json!("WPT4_PROPOSED");
        value["ground_assembly"]["power_rating_w"] = json!(22_000.0);
        value["ground_assembly"]["mounting_type"] = json!("FLUSH_MOUNTED");
        value["installation"]["type"] = json!("FLUSH_MOUNTED");
        value["geometry_domain"]["coordinate_frame_id"] = json!("SYNTHETIC-GA-VA-FRAME-v1");
        for (min, max, low, high) in [
            ("coil_to_coil_air_gap_min_m", "coil_to_coil_air_gap_max_m", 0.10, 0.20),
            ("vehicle_ground_clearance_min_m", "vehicle_ground_clearance_max_m", 0.15, 0.25),
            ("x_offset_min_m", "x_offset_max_m", -0.15, 0.15),
            ("y_offset_min_m", "y_offset_max_m", -0.15, 0.15),
            ("z_offset_min_m", "z_offset_max_m", -0.02, 0.02),
            ("yaw_min_deg", "yaw_max_deg", -5.0, 5.0),
        ] {
            value["geometry_domain"][min] = json!(low);
            value["geometry_domain"][max] = json!(high);
        }
        value["sampling"]["strategy"] = json!("ADAPTIVE_BOUNDARY");
        value["sampling"]["unique_positions_sampled"] = json!(250);
        value["sampling"]["evaluation_point_count"] = json!(500);
        value["sampling"]["inductance_sample_count"] = json!(2_000);
        value["sampling"]["open_circuit_sample_count"] = json!(1_000);
        value["sampling"]["short_circuit_sample_count"] = json!(1_000);
        value["sampling"]["raw_geometry_data_sha256"] = json!("a".repeat(64));
        value["sampling"]["instrumentation_reference"] = json!("SYNTHETIC-CAL-CERT-01");
        value["coupling_range_metric"]["metric_definition_id"] = json!("SYNTHETIC-COUPLING-THRESHOLD-v1");
        value["coupling_range_metric"]["comparison_direction"] = json!("REDUCTION");
        value["coupling_range_metric"]["relative_change_percent"] = json!(2.0);
        value["coupling_range_metric"]["qualifier"] = json!("EXACT");
        value["coupling_range_metric"]["baseline_compatibility_id"] = json!("SYNTHETIC-BASELINE-v1");
        value["evidence"]["kind"] = json!("MEASURED");
        value["evidence"]["method"] = json!("INSTRUMENTED_INDUCTANCE_MEASUREMENT");
        value["evidence"]["source_doi"] = json!("10.0000/synthetic");
        value["evidence"]["source_locator"] = json!("Synthetic unit test only.");
        value["evidence"]["uncertainty_reported"] = json!(true);
        value["qualification"] = json!("QUALIFIED");
        value["notes"] = json!("Synthetic known-truth fixture only; not physical measurements.");
        value
    }

    #[test]
    fn indexed_source_claim_record_is_preserved_without_promotion() {
        assert!(validate_record(&source_record()).is_empty());
        assert_eq!(string_at(&source_record(), &["qualification"]), Some("SOURCE_CLAIM_ONLY"));
    }

    #[test]
    fn not_run_template_is_valid() {
        assert!(validate_record(&template()).is_empty());
    }

    #[test]
    fn synthetic_dense_measured_geometry_record_can_qualify() {
        let errors = validate_record(&qualified_fixture());
        assert!(errors.is_empty(), "{errors:#?}");
    }

    #[test]
    fn rejects_sparse_geometry_grid_for_qualified_record() {
        let mut value = qualified_fixture();
        value["sampling"]["unique_positions_sampled"] = json!(188);
        let errors = validate_record(&value);
        assert!(errors.iter().any(|e| e.contains(">=189 unique positions")));
    }

    #[test]
    fn rejects_missing_raw_geometry_digest() {
        let mut value = qualified_fixture();
        value["sampling"]["raw_geometry_data_sha256"] = Value::Null;
        let errors = validate_record(&value);
        assert!(errors.iter().any(|e| e.contains("raw geometry dataset digest")));
    }

    #[test]
    fn rejects_missing_coordinate_frame() {
        let mut value = qualified_fixture();
        value["geometry_domain"]["coordinate_frame_id"] = Value::Null;
        let errors = validate_record(&value);
        assert!(errors.iter().any(|e| e.contains("coordinate_frame_id")));
    }

    #[test]
    fn rejects_inverted_air_gap_bounds() {
        let mut value = qualified_fixture();
        value["geometry_domain"]["coil_to_coil_air_gap_min_m"] = json!(0.25);
        value["geometry_domain"]["coil_to_coil_air_gap_max_m"] = json!(0.10);
        let errors = validate_record(&value);
        assert!(errors.iter().any(|e| e.contains("minimum exceeds maximum")));
    }

    #[test]
    fn rejects_efficiency_field_in_geometry_record() {
        let mut value = source_record();
        value["efficiency_percent"] = json!(95.0);
        let errors = validate_record(&value);
        assert!(errors.iter().any(|e| e.contains("cannot carry an efficiency score")));
    }

    #[test]
    fn rejects_abstract_extraction_as_qualified_measurement_method() {
        let mut value = qualified_fixture();
        value["evidence"]["method"] = json!("ABSTRACT_EXTRACTION");
        let errors = validate_record(&value);
        assert!(errors.iter().any(|e| e.contains("requires measured coil parameters")));
    }

    #[test]
    fn rejects_unreported_uncertainty_for_qualified_geometry() {
        let mut value = qualified_fixture();
        value["evidence"]["uncertainty_reported"] = json!(false);
        let errors = validate_record(&value);
        assert!(errors.iter().any(|e| e.contains("uncertainty budget required")));
    }

    #[test]
    fn requires_named_baseline_for_relative_qualified_range_claim() {
        let mut value = qualified_fixture();
        value["coupling_range_metric"]["baseline_compatibility_id"] = Value::Null;
        let errors = validate_record(&value);
        assert!(errors.iter().any(|e| e.contains("named baseline required")));
    }
}
