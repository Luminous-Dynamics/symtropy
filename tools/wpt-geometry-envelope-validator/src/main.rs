// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::Value;
use std::collections::BTreeSet;
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
    "qualified_region",
    "qualification_criteria",
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
        &[
            "kind",
            "method",
            "source_doi",
            "source_locator",
            "uncertainty_reported",
            "uncertainty_method",
            "expanded_uncertainty_percent",
            "coverage_factor",
            "coverage_probability_percent",
        ],
        "evidence",
        &mut errors,
    );
    exact_keys(
        record.get("qualified_region"),
        &[
            "metric_scope",
            "evaluated_point_count",
            "qualified_point_count",
            "qualified_fraction",
            "connected_component_count",
            "confidence_level_percent",
            "lower_confidence_bound_fraction",
            "classification_sha256",
            "analysis_sha256",
            "convex_hull_used",
            "disconnected_regions_preserved",
            "notes",
        ],
        "qualified_region",
        &mut errors,
    );
    if let Some(criteria) = record.get("qualification_criteria").and_then(Value::as_array) {
        for (index, criterion) in criteria.iter().enumerate() {
            exact_keys(
                Some(criterion),
                &[
                    "criterion_id",
                    "metric_family",
                    "quantity_kind",
                    "unit",
                    "operator",
                    "lower_threshold",
                    "upper_threshold",
                    "observed_value",
                    "outcome",
                    "hard_gate",
                    "evidence_sha256",
                    "notes",
                ],
                &format!("qualification_criteria[{index}]"),
                &mut errors,
            );
        }
    } else {
        errors.push("qualification_criteria: expected array".into());
    }

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


    for (path, allowed) in [
        (
            &["vehicle_assembly", "generation"][..],
            &["WPT1", "WPT2", "WPT3", "WPT4_PROPOSED", "OTHER", "UNKNOWN"][..],
        ),
        (
            &["ground_assembly", "generation"][..],
            &["WPT1", "WPT2", "WPT3", "WPT4_PROPOSED", "OTHER", "UNKNOWN"][..],
        ),
        (
            &["vehicle_assembly", "mounting_type"][..],
            &["VEHICLE_UNDERBODY", "GROUND_SURFACE", "FLUSH_MOUNTED", "EMBEDDED", "OTHER", "UNKNOWN"][..],
        ),
        (
            &["ground_assembly", "mounting_type"][..],
            &["VEHICLE_UNDERBODY", "GROUND_SURFACE", "FLUSH_MOUNTED", "EMBEDDED", "OTHER", "UNKNOWN"][..],
        ),
        (
            &["installation", "type"][..],
            &["SURFACE_MOUNTED", "FLUSH_MOUNTED", "EMBEDDED", "PORTABLE", "LAB_BENCH", "OTHER", "UNKNOWN"][..],
        ),
        (
            &["sampling", "strategy"][..],
            &["UNIFORM_GRID", "ADAPTIVE_BOUNDARY", "FULL_RANGE_DISCRETE", "OTHER", "UNKNOWN"][..],
        ),
        (
            &["coupling_range_metric", "comparison_direction"][..],
            &["REDUCTION", "INCREASE", "NO_CHANGE", "UNKNOWN"][..],
        ),
        (
            &["coupling_range_metric", "qualifier"][..],
            &["EXACT", "APPROX", "GREATER_THAN", "LESS_THAN", "NOT_REPORTED"][..],
        ),
        (
            &["evidence", "kind"][..],
            &["MEASURED", "AUTHOR_CALCULATED", "SIMULATED", "SECONDARY_REPORTED", "UNKNOWN"][..],
        ),
        (
            &["evidence", "method"][..],
            &["INSTRUMENTED_INDUCTANCE_MEASUREMENT", "CALCULATION_FROM_MEASURED_COIL_PARAMETERS", "NUMERICAL_MODEL", "ABSTRACT_EXTRACTION", "NOT_REPORTED"][..],
        ),
        (
            &["evidence", "uncertainty_method"][..],
            &["GUM", "MONTE_CARLO", "REPEATABILITY", "BOOTSTRAP_SPATIAL", "OTHER", "NOT_REPORTED"][..],
        ),
    ] {
        if !string_at(record, path).is_some_and(|value| allowed.contains(&value)) {
            errors.push(format!("{}: missing or unsupported classification", path.join(".")));
        }
    }
    if !nonempty(string_at(record, &["evidence", "source_locator"])) {
        errors.push("evidence.source_locator: provenance locator required".into());
    }

    let metric_value = number_at(record, &["coupling_range_metric", "relative_change_percent"]);
    let metric_qualifier = string_at(record, &["coupling_range_metric", "qualifier"]);
    match metric_qualifier {
        Some("EXACT" | "APPROX" | "GREATER_THAN" | "LESS_THAN") if metric_value.is_none() => {
            errors.push("coupling_range_metric.relative_change_percent: numeric value required by qualifier".into());
        }
        Some("NOT_REPORTED") if metric_value.is_some() => {
            errors.push("coupling_range_metric.relative_change_percent: NOT_REPORTED requires null value".into());
        }
        _ => {}
    }
    if string_at(record, &["coupling_range_metric", "comparison_direction"]) == Some("REDUCTION")
        && metric_value.is_some_and(|value| value > 100.0)
    {
        errors.push("coupling_range_metric.relative_change_percent: a reduction cannot exceed 100%".into());
    }

    let qualification = string_at(record, &["qualification"]).unwrap_or("INVALID");
    if !matches!(qualification, "NOT_RUN" | "SOURCE_CLAIM_ONLY" | "INCOMPLETE" | "QUALIFIED" | "REJECTED") {
        errors.push("qualification: unsupported disposition".into());
    }

    if qualification == "QUALIFIED" {
        // A qualified envelope is the pointwise pass region, not a convex-hull
        // drawing or a single aligned test point.
        let region = record.get("qualified_region").unwrap_or(&Value::Null);
        let scope = string_at(region, &["metric_scope"]).unwrap_or("UNKNOWN");
        let evaluated = at(region, &["evaluated_point_count"]).and_then(Value::as_u64);
        let qualified = at(region, &["qualified_point_count"]).and_then(Value::as_u64);
        let fraction = number_at(region, &["qualified_fraction"]);
        let components = at(region, &["connected_component_count"]).and_then(Value::as_u64);
        let confidence = number_at(region, &["confidence_level_percent"]);
        let lower_bound = number_at(region, &["lower_confidence_bound_fraction"]);

        match (evaluated, qualified, fraction) {
            (Some(total), Some(passed), Some(fraction))
                if total > 0 && passed > 0 && passed <= total => {
                    let computed = passed as f64 / total as f64;
                    if (fraction - computed).abs() > 1e-6 {
                        errors.push(format!(
                            "qualified_region.qualified_fraction: expected {computed:.9} from passed/evaluated points, got {fraction:.9}"
                        ));
                    }
                }
            _ => errors.push(
                "qualified_region: positive evaluated/qualified point counts and fraction required"
                    .into(),
            ),
        }
        if !matches!(scope, "COUPLING_ONLY" | "POWER_TRANSFER" | "EFFICIENCY" | "THERMAL" | "SAFETY" | "SYSTEM_INTEROPERABILITY") {
            errors.push("qualified_region.metric_scope: known qualification scope required".into());
        }
        if !components.is_some_and(|count| count > 0) {
            errors.push("qualified_region.connected_component_count: at least one connected qualified component required".into());
        }
        match (confidence, lower_bound, fraction) {
            (Some(level), Some(bound), Some(observed))
                if (90.0..=99.99).contains(&level) && bound >= 0.0 && bound <= observed => {}
            _ => errors.push(
                "qualified_region: 90-99.99% confidence level and lower confidence bound <= measured fraction required"
                    .into(),
            ),
        }
        if !string_at(region, &["classification_sha256"]).is_some_and(valid_sha256) {
            errors.push("qualified_region.classification_sha256: pointwise pass/fail label digest required".into());
        }
        if !string_at(region, &["analysis_sha256"]).is_some_and(valid_sha256) {
            errors.push("qualified_region.analysis_sha256: reproducible analysis digest required".into());
        }
        if at(region, &["disconnected_regions_preserved"]).and_then(Value::as_bool) != Some(true) {
            errors.push("qualified_region.disconnected_regions_preserved: preserve the actual connected components; do not merge them into one hull".into());
        }
        if at(region, &["convex_hull_used"]).and_then(Value::as_bool) == Some(true)
            && at(region, &["disconnected_regions_preserved"]).and_then(Value::as_bool) != Some(true)
        {
            errors.push("qualified_region.convex_hull_used: convex hull cannot replace disconnected measured pass regions".into());
        }

        let mut families = BTreeSet::new();
        match record.get("qualification_criteria").and_then(Value::as_array) {
            Some(criteria) if !criteria.is_empty() => {
                let mut criterion_ids = BTreeSet::new();
                for (index, criterion) in criteria.iter().enumerate() {
                    let id = string_at(criterion, &["criterion_id"]).unwrap_or("");
                    let family = string_at(criterion, &["metric_family"]).unwrap_or("");
                    let operator = string_at(criterion, &["operator"]).unwrap_or("");
                    let outcome = string_at(criterion, &["outcome"]).unwrap_or("NOT_RUN");
                    if !nonempty(Some(id)) || !criterion_ids.insert(id.to_owned()) {
                        errors.push(format!("qualification_criteria[{index}].criterion_id: missing or duplicate"));
                    }
                    if !matches!(family, "COUPLING" | "OUTPUT_POWER" | "OUTPUT_REGULATION" | "EFFICIENCY" | "THERMAL" | "SAFETY" | "COMPONENT_STRESS" | "EMC" | "RELIABILITY" | "OTHER") {
                        errors.push(format!("qualification_criteria[{index}].metric_family: invalid metric family"));
                    } else {
                        families.insert(family.to_owned());
                    }
                    if !nonempty(string_at(criterion, &["quantity_kind"]))
                        || !nonempty(string_at(criterion, &["unit"]))
                        || !nonempty(string_at(criterion, &["notes"]))
                    {
                        errors.push(format!("qualification_criteria[{index}]: quantity, unit and notes are required"));
                    }
                    if outcome != "PASS" {
                        errors.push(format!("qualification_criteria[{index}].outcome: every criterion must PASS for QUALIFIED disposition"));
                    }
                    if at(criterion, &["hard_gate"]).and_then(Value::as_bool) != Some(true) {
                        errors.push(format!("qualification_criteria[{index}].hard_gate: qualified envelopes require hard-gate criteria"));
                    }
                    if !string_at(criterion, &["evidence_sha256"]).is_some_and(valid_sha256) {
                        errors.push(format!("qualification_criteria[{index}].evidence_sha256: evidence digest required"));
                    }

                    let observed = criterion.get("observed_value");
                    let observed_number = observed.and_then(Value::as_f64);
                    let lower = number_at(criterion, &["lower_threshold"]);
                    let upper = number_at(criterion, &["upper_threshold"]);
                    let passes = match operator {
                        "GTE" => matches!((observed_number, lower), (Some(value), Some(limit)) if value >= limit),
                        "LTE" => matches!((observed_number, upper), (Some(value), Some(limit)) if value <= limit),
                        "BETWEEN" => matches!((observed_number, lower, upper), (Some(value), Some(minimum), Some(maximum)) if minimum <= value && value <= maximum),
                        "BOOLEAN_PASS" => observed.and_then(Value::as_bool) == Some(true),
                        _ => false,
                    };
                    if !passes {
                        errors.push(format!("qualification_criteria[{index}]: observed value does not satisfy its declared threshold"));
                    }
                }
            }
            _ => errors.push("qualification_criteria: at least one declared, passing criterion required".into()),
        }

        let expected_families: &[&str] = match scope {
            "COUPLING_ONLY" => &["COUPLING"],
            "POWER_TRANSFER" => &["COUPLING", "OUTPUT_POWER", "OUTPUT_REGULATION"],
            "EFFICIENCY" => &["COUPLING", "OUTPUT_POWER", "EFFICIENCY"],
            "THERMAL" => &["COUPLING", "THERMAL", "COMPONENT_STRESS"],
            "SAFETY" => &["COUPLING", "SAFETY", "COMPONENT_STRESS", "EMC"],
            "SYSTEM_INTEROPERABILITY" => &[
                "COUPLING",
                "OUTPUT_POWER",
                "OUTPUT_REGULATION",
                "EFFICIENCY",
                "THERMAL",
                "SAFETY",
                "COMPONENT_STRESS",
                "EMC",
            ],
            _ => &[],
        };
        for family in expected_families {
            if !families.contains(*family) {
                errors.push(format!(
                    "qualified_region.metric_scope: {scope} requires {family} acceptance evidence"
                ));
            }
        }

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
        let region_evaluation_points =
            at(region, &["evaluated_point_count"]).and_then(Value::as_u64);
        if evaluation_points != region_evaluation_points {
            errors.push("qualified_region.evaluated_point_count: must match the sampling evaluation_point_count".into());
        }
        if string_at(record, &["sampling", "strategy"]) == Some("UNKNOWN") {
            errors.push("sampling.strategy: known sampling strategy required for QUALIFIED envelope".into());
        }
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
        let open_samples = at(record, &["sampling", "open_circuit_sample_count"]).and_then(Value::as_u64);
        let short_samples = at(record, &["sampling", "short_circuit_sample_count"]).and_then(Value::as_u64);
        let total_samples = at(record, &["sampling", "inductance_sample_count"]).and_then(Value::as_u64);
        match (open_samples, short_samples, total_samples) {
            (Some(open), Some(short), Some(total))
                if open > 0
                    && short > 0
                    && open.checked_add(short).is_some_and(|sum| sum <= total) => {}
            _ => errors.push("sampling: qualified inductance evidence requires positive open- and short-circuit sample counts included in total samples".into()),
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
        if string_at(record, &["evidence", "uncertainty_method"]) == Some("NOT_REPORTED") {
            errors.push("evidence.uncertainty_method: quantified uncertainty method required for QUALIFIED envelope".into());
        }
        let expanded_uncertainty =
            number_at(record, &["evidence", "expanded_uncertainty_percent"]);
        let coverage_factor = number_at(record, &["evidence", "coverage_factor"]);
        let coverage_probability =
            number_at(record, &["evidence", "coverage_probability_percent"]);
        match (expanded_uncertainty, coverage_factor, coverage_probability) {
            (Some(expanded), Some(factor), Some(probability))
                if expanded >= 0.0 && factor > 0.0 && (90.0..=99.99).contains(&probability) => {}
            _ => errors.push(
                "evidence: QUALIFIED geometry requires expanded uncertainty, coverage factor and 90-99.99% coverage probability"
                    .into(),
            ),
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
    let mut seen_ids = BTreeSet::new();
    for (index, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line)?;
        let mut errors = validate_record(&value);
        let compatibility_id = string_at(&value, &["compatibility_id"]).unwrap_or("");
        if !compatibility_id.is_empty() && !seen_ids.insert(compatibility_id.to_owned()) {
            errors.push(format!(
                "compatibility_id: duplicate record ID {compatibility_id} within {path}"
            ));
        }
        results.push((format!("{path}:{}", index + 1), errors));
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
        value["qualified_region"] = json!({
            "metric_scope": "COUPLING_ONLY",
            "evaluated_point_count": 500,
            "qualified_point_count": 450,
            "qualified_fraction": 0.9,
            "connected_component_count": 1,
            "confidence_level_percent": 95.0,
            "lower_confidence_bound_fraction": 0.88,
            "classification_sha256": "b".repeat(64),
            "analysis_sha256": "c".repeat(64),
            "convex_hull_used": false,
            "disconnected_regions_preserved": true,
            "notes": "Synthetic known-truth region fixture only."
        });
        value["qualification_criteria"] = json!([
            {
                "criterion_id": "COUPLING-MIN",
                "metric_family": "COUPLING",
                "quantity_kind": "coupling_coefficient",
                "unit": "1",
                "operator": "GTE",
                "lower_threshold": 0.05,
                "upper_threshold": null,
                "observed_value": 0.12,
                "outcome": "PASS",
                "hard_gate": true,
                "evidence_sha256": "d".repeat(64),
                "notes": "Synthetic coupling acceptance threshold."
            }
        ]);
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
        value["evidence"]["uncertainty_method"] = json!("BOOTSTRAP_SPATIAL");
        value["evidence"]["expanded_uncertainty_percent"] = json!(1.2);
        value["evidence"]["coverage_factor"] = json!(2.0);
        value["evidence"]["coverage_probability_percent"] = json!(95.0);
        value["qualification"] = json!("QUALIFIED");
        value["notes"] = json!("Synthetic known-truth fixture only; not physical measurements.");
        value
    }


    #[test]
    fn geometry_schema_required_keys_match_rust_validator_contract() {
        let schema: Value = serde_json::from_str(include_str!(
            "../../../benchmarks/wpt-empirical/v1/geometry-compatibility.schema.json"
        ))
        .expect("geometry schema parses");

        let required = |path: &[&str]| -> BTreeSet<String> {
            at(&schema, path)
                .and_then(Value::as_array)
                .expect("required field array")
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        };
        let as_set = |values: &[&str]| -> BTreeSet<String> {
            values.iter().map(|value| (*value).to_owned()).collect()
        };

        assert_eq!(required(&["required"]), as_set(TOP_KEYS));
        assert_eq!(
            required(&["properties", "geometry_domain", "required"]),
            as_set(&[
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
            ])
        );
        assert_eq!(
            required(&["properties", "evidence", "required"]),
            as_set(&[
                "kind",
                "method",
                "source_doi",
                "source_locator",
                "uncertainty_reported",
                "uncertainty_method",
                "expanded_uncertainty_percent",
                "coverage_factor",
                "coverage_probability_percent",
            ])
        );
        assert_eq!(
            required(&["properties", "qualified_region", "required"]),
            as_set(&[
                "metric_scope",
                "evaluated_point_count",
                "qualified_point_count",
                "qualified_fraction",
                "connected_component_count",
                "confidence_level_percent",
                "lower_confidence_bound_fraction",
                "classification_sha256",
                "analysis_sha256",
                "convex_hull_used",
                "disconnected_regions_preserved",
                "notes",
            ])
        );
        assert_eq!(
            required(&["properties", "qualification_criteria", "items", "required"]),
            as_set(&[
                "criterion_id",
                "metric_family",
                "quantity_kind",
                "unit",
                "operator",
                "lower_threshold",
                "upper_threshold",
                "observed_value",
                "outcome",
                "hard_gate",
                "evidence_sha256",
                "notes",
            ])
        );
    }


    #[test]
    fn inductance_sample_count_addition_is_overflow_safe() {
        let mut value = qualified_fixture();
        value["sampling"]["open_circuit_sample_count"] = json!(u64::MAX);
        value["sampling"]["short_circuit_sample_count"] = json!(u64::MAX);
        value["sampling"]["inductance_sample_count"] = json!(u64::MAX);
        let errors = validate_record(&value);
        assert!(errors.iter().any(|e| e.contains("sample counts included in total")));
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
    fn rejects_qualified_fraction_inconsistent_with_point_counts() {
        let mut value = qualified_fixture();
        value["qualified_region"]["qualified_fraction"] = json!(0.5);
        let errors = validate_record(&value);
        assert!(errors.iter().any(|e| e.contains("expected 0.900000000")));
    }

    #[test]
    fn rejects_collapsing_disconnected_regions_into_convex_hull() {
        let mut value = qualified_fixture();
        value["qualified_region"]["connected_component_count"] = json!(2);
        value["qualified_region"]["convex_hull_used"] = json!(true);
        value["qualified_region"]["disconnected_regions_preserved"] = json!(false);
        let errors = validate_record(&value);
        assert!(errors.iter().any(|e| e.contains("preserve the actual connected components")));
    }

    #[test]
    fn system_interoperability_scope_requires_all_hard_gate_families() {
        let mut value = qualified_fixture();
        value["qualified_region"]["metric_scope"] = json!("SYSTEM_INTEROPERABILITY");
        let errors = validate_record(&value);
        assert!(errors.iter().any(|e| e.contains("requires OUTPUT_POWER acceptance evidence")));
        assert!(errors.iter().any(|e| e.contains("requires EFFICIENCY acceptance evidence")));
        assert!(errors.iter().any(|e| e.contains("requires SAFETY acceptance evidence")));
        assert!(errors.iter().any(|e| e.contains("requires EMC acceptance evidence")));
    }

    #[test]
    fn rejects_qualification_criterion_that_does_not_meet_threshold() {
        let mut value = qualified_fixture();
        value["qualification_criteria"][0]["observed_value"] = json!(0.01);
        let errors = validate_record(&value);
        assert!(errors.iter().any(|e| e.contains("does not satisfy its declared threshold")));
    }

    #[test]
    fn rejects_open_short_inductance_sample_count_underreporting() {
        let mut value = qualified_fixture();
        value["sampling"]["open_circuit_sample_count"] = json!(1_500);
        let errors = validate_record(&value);
        assert!(errors.iter().any(|e| e.contains("sample counts included in total")));
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
    fn rejects_qualified_envelope_without_numeric_uncertainty_budget() {
        let mut value = qualified_fixture();
        value["evidence"]["expanded_uncertainty_percent"] = Value::Null;
        let errors = validate_record(&value);
        assert!(errors.iter().any(|e| e.contains("expanded uncertainty, coverage factor")));
    }

    #[test]
    fn rejects_invalid_assembly_generation_label() {
        let mut value = source_record();
        value["vehicle_assembly"]["generation"] = json!("WPT5_CERTIFIED");
        let errors = validate_record(&value);
        assert!(errors.iter().any(|e| e.contains("vehicle_assembly.generation")));
    }

    #[test]
    fn rejects_coupling_range_reduction_above_one_hundred_percent() {
        let mut value = source_record();
        value["coupling_range_metric"]["relative_change_percent"] = json!(120.0);
        let errors = validate_record(&value);
        assert!(errors.iter().any(|e| e.contains("reduction cannot exceed 100%")));
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
