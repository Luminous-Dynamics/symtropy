// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::Value;
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct ValidationError(String);

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for ValidationError {}

type ValidationResult<T> = Result<T, ValidationError>;

fn invalid<T>(message: impl Into<String>) -> ValidationResult<T> {
    Err(ValidationError(message.into()))
}

fn required_fields(value: &Value, schema: &Value, label: &str) -> ValidationResult<()> {
    let object = value
        .as_object()
        .ok_or_else(|| ValidationError(format!("{label}: expected a JSON object")))?;
    let required = schema
        .get("required")
        .and_then(Value::as_array)
        .ok_or_else(|| ValidationError(format!("{label}: schema has no required array")))?;
    for item in required {
        let field = item.as_str().ok_or_else(|| {
            ValidationError(format!("{label}: schema required entry is not a string"))
        })?;
        if !object.contains_key(field) {
            return invalid(format!("{label}: missing required field {field}"));
        }
    }
    Ok(())
}

fn non_empty_string(value: &Value, key: &str, label: &str) -> ValidationResult<String> {
    let text = value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| ValidationError(format!("{label}: {key} must be a string")))?;
    if text.trim().is_empty() {
        return invalid(format!("{label}: {key} must not be empty"));
    }
    Ok(text.to_owned())
}

fn enum_value(value: &Value, key: &str, allowed: &[&str], label: &str) -> ValidationResult<String> {
    let actual = non_empty_string(value, key, label)?;
    if !allowed.contains(&actual.as_str()) {
        return invalid(format!("{label}: {key} has unsupported value {actual}"));
    }
    Ok(actual)
}

fn optional_number(value: &Value, key: &str, label: &str) -> ValidationResult<Option<f64>> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(number) => {
            let parsed = number.as_f64().ok_or_else(|| {
                ValidationError(format!("{label}: {key} must be numeric or null"))
            })?;
            if !parsed.is_finite() {
                return invalid(format!("{label}: {key} must be finite"));
            }
            Ok(Some(parsed))
        }
    }
}

fn validate_qualified_value(
    value: Option<f64>,
    upper: Option<f64>,
    qualifier: &str,
    minimum: f64,
    maximum: Option<f64>,
    label: &str,
) -> ValidationResult<()> {
    let in_range = |number: f64| {
        number.is_finite() && number >= minimum && maximum.is_none_or(|max| number <= max)
    };

    if value.is_some_and(|number| !in_range(number)) {
        return invalid(format!("{label}: value is outside its valid numeric range"));
    }
    if upper.is_some_and(|number| !in_range(number)) {
        return invalid(format!(
            "{label}: upper value is outside its valid numeric range"
        ));
    }

    match qualifier {
        "NOT_REPORTED" => {
            if value.is_some() || upper.is_some() {
                return invalid(format!(
                    "{label}: NOT_REPORTED must not carry a numeric value"
                ));
            }
        }
        "RANGE" => {
            let (Some(lower), Some(upper_value)) = (value, upper) else {
                return invalid(format!("{label}: RANGE requires lower and upper values"));
            };
            if upper_value < lower {
                return invalid(format!(
                    "{label}: range upper value is below its lower value"
                ));
            }
        }
        "EXACT" | "APPROX" | "GREATER_THAN" | "LESS_THAN" => {
            if value.is_none() {
                return invalid(format!(
                    "{label}: qualifier {qualifier} requires a numeric value"
                ));
            }
            if upper.is_some() {
                return invalid(format!("{label}: upper value is only valid for RANGE"));
            }
        }
        _ => return invalid(format!("{label}: unsupported qualifier {qualifier}")),
    }
    Ok(())
}

fn read_jsonl(path: &Path) -> ValidationResult<Vec<Value>> {
    let file = File::open(path)
        .map_err(|error| ValidationError(format!("cannot open {}: {error}", path.display())))?;
    let reader = BufReader::new(file);
    let mut values = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        let line = line.map_err(|error| {
            ValidationError(format!("{} line {}: {error}", path.display(), index + 1))
        })?;
        if line.trim().is_empty() {
            return invalid(format!(
                "{} line {}: blank JSONL records are not allowed",
                path.display(),
                index + 1
            ));
        }
        let value: Value = serde_json::from_str(&line).map_err(|error| {
            ValidationError(format!(
                "{} line {}: invalid JSON: {error}",
                path.display(),
                index + 1
            ))
        })?;
        values.push(value);
    }

    if values.is_empty() {
        return invalid(format!(
            "{}: catalog must contain at least one record",
            path.display()
        ));
    }
    Ok(values)
}

fn validate_sources(records: &[Value], schema: &Value) -> ValidationResult<BTreeSet<String>> {
    let mut ids = BTreeSet::new();
    for (index, record) in records.iter().enumerate() {
        let label = format!("source record {}", index + 1);
        required_fields(record, schema, &label)?;
        if record.get("schema_version").and_then(Value::as_str) != Some("wpt-source-v0") {
            return invalid(format!("{label}: unsupported schema_version"));
        }
        let uri = non_empty_string(record, "uri", &label)?;
        if !(uri.starts_with("https://") || uri.starts_with("http://")) {
            return invalid(format!("{label}: uri must use http or https"));
        }
        non_empty_string(record, "title", &label)?;
        let year = record
            .get("year")
            .and_then(Value::as_i64)
            .ok_or_else(|| ValidationError(format!("{label}: year must be an integer")))?;
        if !(1900..=2100).contains(&year) {
            return invalid(format!("{label}: year is outside the allowed range"));
        }
        let authors = record
            .get("authors")
            .and_then(Value::as_array)
            .ok_or_else(|| ValidationError(format!("{label}: authors must be an array")))?;
        if authors.is_empty()
            || authors
                .iter()
                .any(|x| x.as_str().is_none_or(|s| s.trim().is_empty()))
        {
            return invalid(format!("{label}: authors must contain non-empty strings"));
        }
        let id = non_empty_string(record, "source_id", &label)?;
        if !ids.insert(id.clone()) {
            return invalid(format!("{label}: duplicate source_id {id}"));
        }
        enum_value(
            record,
            "source_kind",
            &[
                "peer_reviewed_article",
                "survey_article",
                "standard",
                "preprint",
                "other",
            ],
            &label,
        )?;
        enum_value(
            record,
            "rights_status",
            &["known_open", "unknown", "restricted", "public_domain"],
            &label,
        )?;
        enum_value(
            record,
            "content_access",
            &["metadata_only", "limited_use", "full_text", "abstract_only"],
            &label,
        )?;
        enum_value(
            record,
            "rights_review",
            &["approved", "pending", "denied"],
            &label,
        )?;
        enum_value(
            record,
            "split",
            &["development", "validation", "test", "holdout"],
            &label,
        )?;
        let locators = record
            .get("locators")
            .and_then(Value::as_array)
            .ok_or_else(|| ValidationError(format!("{label}: locators must be an array")))?;
        if locators.is_empty()
            || locators
                .iter()
                .any(|x| x.as_str().is_none_or(str::is_empty))
        {
            return invalid(format!("{label}: locators must contain non-empty strings"));
        }
    }
    Ok(ids)
}

fn validate_measurement(measurement: &Value, schema: &Value, label: &str) -> ValidationResult<()> {
    required_fields(measurement, schema, label)?;
    enum_value(
        measurement,
        "test_mode",
        &[
            "ELECTRONIC_LOAD",
            "BACK_TO_BACK",
            "BATTERY_LOAD",
            "FIELD_TEST",
            "CONTROL_COMPARISON",
            "SIMULATION",
            "OTHER",
        ],
        label,
    )?;
    enum_value(
        measurement,
        "evidence_kind",
        &[
            "MEASURED",
            "AUTHOR_CALCULATED",
            "SIMULATED",
            "SECONDARY_REPORTED",
            "UNKNOWN",
        ],
        label,
    )?;

    let input = measurement
        .get("input_boundary")
        .ok_or_else(|| ValidationError(format!("{label}: missing input_boundary")))?;
    non_empty_string(input, "measurement_point", label)?;
    let output = measurement
        .get("output_boundary")
        .ok_or_else(|| ValidationError(format!("{label}: missing output_boundary")))?;
    non_empty_string(output, "measurement_point", label)?;

    let power = measurement
        .get("power_out_w")
        .ok_or_else(|| ValidationError(format!("{label}: missing power_out_w")))?;
    let power_qualifier = enum_value(
        power,
        "qualifier",
        &[
            "EXACT",
            "APPROX",
            "GREATER_THAN",
            "LESS_THAN",
            "RANGE",
            "NOT_REPORTED",
        ],
        label,
    )?;
    validate_qualified_value(
        optional_number(power, "value", label)?,
        optional_number(power, "upper_value_w", label)?,
        &power_qualifier,
        0.0,
        None,
        &format!("{label}.power_out_w"),
    )?;

    let efficiency = measurement
        .get("efficiency")
        .ok_or_else(|| ValidationError(format!("{label}: missing efficiency")))?;
    let efficiency_qualifier = enum_value(
        efficiency,
        "qualifier",
        &[
            "EXACT",
            "APPROX",
            "GREATER_THAN",
            "LESS_THAN",
            "RANGE",
            "NOT_REPORTED",
        ],
        label,
    )?;
    non_empty_string(efficiency, "definition", label)?;
    validate_qualified_value(
        optional_number(efficiency, "value_percent", label)?,
        optional_number(efficiency, "upper_value_percent", label)?,
        &efficiency_qualifier,
        0.0,
        Some(100.0),
        &format!("{label}.efficiency"),
    )?;
    if measurement.get("power_in_w_qualifier").is_some() {
        enum_value(
            measurement,
            "power_in_w_qualifier",
            &[
                "EXACT",
                "APPROX",
                "GREATER_THAN",
                "LESS_THAN",
                "RANGE",
                "NOT_REPORTED",
            ],
            label,
        )?;
    }
    for key in ["power_in_w", "transfer_gap_m", "frequency_hz"] {
        if let Some(number) = optional_number(measurement, key, label)? {
            if number < 0.0 || (key == "frequency_hz" && number == 0.0) {
                return invalid(format!(
                    "{label}: {key} must be positive or zero only where meaningful"
                ));
            }
        }
    }
    for boundary_key in ["input_boundary", "output_boundary"] {
        let boundary = measurement
            .get(boundary_key)
            .ok_or_else(|| ValidationError(format!("{label}: missing {boundary_key}")))?;
        for stages_key in ["included_stages", "excludes"] {
            let stages = boundary
                .get(stages_key)
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    ValidationError(format!(
                        "{label}: {boundary_key}.{stages_key} must be an array"
                    ))
                })?;
            if stages
                .iter()
                .any(|x| x.as_str().is_none_or(|s| s.trim().is_empty()))
            {
                return invalid(format!(
                    "{label}: {boundary_key}.{stages_key} must contain non-empty strings"
                ));
            }
        }
    }
    non_empty_string(measurement, "source_locator", label)?;
    Ok(())
}

fn validate_realizations(
    records: &[Value],
    schema: &Value,
    source_ids: &BTreeSet<String>,
) -> ValidationResult<(usize, usize)> {
    let mut system_ids = BTreeSet::new();
    let mut total_measurements = 0usize;
    let mut total_engineering_metrics = 0usize;
    let measurement_schema = schema
        .pointer("/properties/measurements/items")
        .ok_or_else(|| {
            ValidationError("realization schema has no measurement item schema".into())
        })?;

    for (index, record) in records.iter().enumerate() {
        let label = format!("realization record {}", index + 1);
        required_fields(record, schema, &label)?;
        let id = non_empty_string(record, "system_id", &label)?;
        if !system_ids.insert(id.clone()) {
            return invalid(format!("{label}: duplicate system_id {id}"));
        }

        if record.get("schema_version").and_then(Value::as_str) != Some("wpt-realization-v0") {
            return invalid(format!("{label}: unsupported schema_version"));
        }
        enum_value(
            record,
            "technology",
            &[
                "IPT",
                "CPT",
                "RF_MICROWAVE",
                "OPTICAL",
                "EARTH_CONDUCTION",
                "GUIDED_WAVE",
                "HYBRID",
                "OTHER",
            ],
            &label,
        )?;
        let publication = record
            .get("publication")
            .ok_or_else(|| ValidationError(format!("{label}: missing publication")))?;
        non_empty_string(publication, "doi", &label)?;
        non_empty_string(publication, "title", &label)?;
        let publication_year =
            publication
                .get("year")
                .and_then(Value::as_i64)
                .ok_or_else(|| {
                    ValidationError(format!("{label}: publication year must be an integer"))
                })?;
        if !(1900..=2100).contains(&publication_year) {
            return invalid(format!(
                "{label}: publication year is outside the allowed range"
            ));
        }
        let application = record
            .get("application")
            .ok_or_else(|| ValidationError(format!("{label}: missing application")))?;
        enum_value(
            application,
            "domain",
            &[
                "EV_CHARGING",
                "INDUSTRIAL",
                "CONSUMER",
                "BIOMEDICAL",
                "ROBOTICS",
                "GRID",
                "COMMUNICATIONS",
                "RESEARCH",
                "OTHER",
            ],
            &label,
        )?;
        enum_value(
            application,
            "operating_mode",
            &["STATIC", "DYNAMIC", "DOCKED", "PORTABLE", "OTHER"],
            &label,
        )?;
        let prototype = record
            .get("prototype")
            .ok_or_else(|| ValidationError(format!("{label}: missing prototype")))?;
        enum_value(
            prototype,
            "transfer_medium",
            &["AIR", "WATER", "SOIL", "VACUUM", "OTHER", "UNKNOWN"],
            &label,
        )?;
        for key in [
            "nominal_power_w",
            "frequency_hz",
            "transmitter_area_cm2",
            "receiver_area_cm2",
            "gap_min_m",
            "gap_max_m",
        ] {
            if let Some(number) = optional_number(prototype, key, &label)? {
                if number < 0.0 || (key == "frequency_hz" && number == 0.0) {
                    return invalid(format!(
                        "{label}: prototype {key} must be positive or zero only where meaningful"
                    ));
                }
            }
        }
        let gap_min = optional_number(prototype, "gap_min_m", &label)?;
        let gap_max = optional_number(prototype, "gap_max_m", &label)?;
        if matches!((gap_min, gap_max), (Some(min), Some(max)) if min > max) {
            return invalid(format!("{label}: prototype gap_min_m exceeds gap_max_m"));
        }
        enum_value(
            record,
            "data_status",
            &[
                "DEVELOPMENT",
                "DUAL_ANNOTATED",
                "EXPERT_REVIEWED",
                "HELD_OUT",
                "RETIRED",
            ],
            &label,
        )?;

        let refs = record
            .get("source_refs")
            .and_then(Value::as_array)
            .ok_or_else(|| ValidationError(format!("{label}: source_refs must be an array")))?;
        if refs.is_empty() {
            return invalid(format!(
                "{label}: at least one source reference is required"
            ));
        }
        for source_ref in refs {
            let source_id = non_empty_string(source_ref, "source_id", &label)?;
            if !source_ids.contains(&source_id) {
                return invalid(format!("{label}: unresolved source_id {source_id}"));
            }
            enum_value(
                source_ref,
                "role",
                &[
                    "PRIMARY_PUBLICATION",
                    "SURVEY_SECONDARY",
                    "DATASET",
                    "OTHER",
                ],
                &label,
            )?;
            non_empty_string(source_ref, "locator", &label)?;
        }

        let measurements = record
            .get("measurements")
            .and_then(Value::as_array)
            .ok_or_else(|| ValidationError(format!("{label}: measurements must be an array")))?;
        let engineering_metrics_present = record
            .get("engineering_metrics")
            .and_then(Value::as_array)
            .is_some_and(|metrics| !metrics.is_empty());
        if measurements.is_empty() && !engineering_metrics_present {
            return invalid(format!(
                "{label}: at least one source-grounded measurement or engineering metric is required"
            ));
        }
        let mut measurement_ids = BTreeSet::new();
        for (measurement_index, measurement) in measurements.iter().enumerate() {
            let measurement_label = format!("{label}, measurement {}", measurement_index + 1);
            let measurement_id =
                non_empty_string(measurement, "measurement_id", &measurement_label)?;
            if !measurement_ids.insert(measurement_id.clone()) {
                return invalid(format!(
                    "{measurement_label}: duplicate measurement_id {measurement_id}"
                ));
            }
            validate_measurement(measurement, measurement_schema, &measurement_label)?;
            total_measurements += 1;
        }

        let claims = record
            .get("claim_audit")
            .and_then(Value::as_array)
            .ok_or_else(|| ValidationError(format!("{label}: claim_audit must be an array")))?;
        for (claim_index, claim) in claims.iter().enumerate() {
            let claim_label = format!("{label}, claim {}", claim_index + 1);
            non_empty_string(claim, "claim_id", &claim_label)?;
            non_empty_string(claim, "source_locator", &claim_label)?;
            non_empty_string(claim, "claim_text", &claim_label)?;
            non_empty_string(claim, "metric_label", &claim_label)?;
            non_empty_string(claim, "notes", &claim_label)?;
            enum_value(
                claim,
                "status",
                &[
                    "RECONCILED",
                    "DISCREPANCY",
                    "AMBIGUOUS_BOUNDARY",
                    "UNRESOLVED",
                    "SUPPORTED_WITH_LIMITS",
                ],
                &claim_label,
            )?;
            if let Some(value) = optional_number(claim, "value_percent", &claim_label)? {
                if !(0.0..=100.0).contains(&value) {
                    return invalid(format!("{claim_label}: value_percent must be in 0..=100"));
                }
            }
        }

        if let Some(metrics) = record.get("engineering_metrics") {
            let metrics = metrics.as_array().ok_or_else(|| {
                ValidationError(format!("{label}: engineering_metrics must be an array"))
            })?;
            let mut metric_ids = BTreeSet::new();
            for (metric_index, metric) in metrics.iter().enumerate() {
                let metric_label = format!("{label}, engineering metric {}", metric_index + 1);
                let metric_id = non_empty_string(metric, "metric_id", &metric_label)?;
                if !metric_ids.insert(metric_id.clone()) {
                    return invalid(format!("{metric_label}: duplicate metric_id {metric_id}"));
                }
                non_empty_string(metric, "name", &metric_label)?;
                non_empty_string(metric, "unit", &metric_label)?;
                non_empty_string(metric, "source_locator", &metric_label)?;
                non_empty_string(metric, "notes", &metric_label)?;
                enum_value(
                    metric,
                    "evidence_kind",
                    &[
                        "MEASURED",
                        "AUTHOR_CALCULATED",
                        "SIMULATED",
                        "SECONDARY_REPORTED",
                        "UNKNOWN",
                        "ASSUMED",
                    ],
                    &metric_label,
                )?;
                let value = optional_number(metric, "value", &metric_label)?;
                let value_max = optional_number(metric, "value_max", &metric_label)?;
                if matches!((value, value_max), (Some(lower), Some(upper)) if upper < lower) {
                    return invalid(format!("{metric_label}: value_max is below value"));
                }
                total_engineering_metrics += 1;
            }
        }
    }
    Ok((total_measurements, total_engineering_metrics))
}

fn validate(root: &Path) -> ValidationResult<()> {
    let base = root.join("benchmarks/wpt-empirical/v0");
    let source_schema: Value = serde_json::from_reader(
        File::open(base.join("source.schema.json"))
            .map_err(|e| ValidationError(format!("cannot open source schema: {e}")))?,
    )
    .map_err(|e| ValidationError(format!("invalid source schema JSON: {e}")))?;
    let realization_schema: Value = serde_json::from_reader(
        File::open(base.join("realization.schema.json"))
            .map_err(|e| ValidationError(format!("cannot open realization schema: {e}")))?,
    )
    .map_err(|e| ValidationError(format!("invalid realization schema JSON: {e}")))?;
    if source_schema.get("$schema").and_then(Value::as_str)
        != Some("https://json-schema.org/draft/2020-12/schema")
    {
        return invalid("source schema is not declared as JSON Schema 2020-12");
    }
    if realization_schema.get("$schema").and_then(Value::as_str)
        != Some("https://json-schema.org/draft/2020-12/schema")
    {
        return invalid("realization schema is not declared as JSON Schema 2020-12");
    }

    let sources = read_jsonl(&base.join("sources.jsonl"))?;
    let realizations = read_jsonl(&base.join("realizations.jsonl"))?;
    let source_ids = validate_sources(&sources, &source_schema)?;
    let (measurements, engineering_metrics) =
        validate_realizations(&realizations, &realization_schema, &source_ids)?;

    println!("WPT empirical envelope v0: VALID");
    println!("sources: {}", sources.len());
    println!("prototype families: {}", realizations.len());
    println!("measurement records: {measurements}");
    println!("engineering metrics: {engineering_metrics}");
    println!("all source references resolve; no duplicate IDs; qualified ranges are consistent");
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let root = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    validate(&root)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_in_dataset_passes_the_full_integrity_audit() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        validate(&root).expect("checked-in WPT empirical dataset should validate");
    }

    #[test]
    fn engineering_metric_can_explicitly_mark_a_scenario_input_as_assumed() {
        let value = serde_json::json!({ "evidence_kind": "ASSUMED" });
        assert_eq!(
            enum_value(
                &value,
                "evidence_kind",
                &[
                    "MEASURED",
                    "AUTHOR_CALCULATED",
                    "SIMULATED",
                    "SECONDARY_REPORTED",
                    "UNKNOWN",
                    "ASSUMED",
                ],
                "engineering metric"
            )
            .unwrap(),
            "ASSUMED"
        );
    }

    #[test]
    fn claim_audit_can_mark_a_source_supported_with_limits() {
        let value = serde_json::json!({ "status": "SUPPORTED_WITH_LIMITS" });
        assert_eq!(
            enum_value(
                &value,
                "status",
                &[
                    "RECONCILED",
                    "DISCREPANCY",
                    "AMBIGUOUS_BOUNDARY",
                    "UNRESOLVED",
                    "SUPPORTED_WITH_LIMITS",
                ],
                "claim audit"
            )
            .unwrap(),
            "SUPPORTED_WITH_LIMITS"
        );
    }

    #[test]
    fn range_qualifier_requires_ordered_bounds() {
        assert!(
            validate_qualified_value(
                Some(5.0),
                Some(10.0),
                "RANGE",
                0.0,
                Some(100.0),
                "efficiency"
            )
            .is_ok()
        );
        assert!(
            validate_qualified_value(
                Some(10.0),
                Some(5.0),
                "RANGE",
                0.0,
                Some(100.0),
                "efficiency"
            )
            .is_err()
        );
    }

    #[test]
    fn not_reported_requires_null_value() {
        assert!(validate_qualified_value(None, None, "NOT_REPORTED", 0.0, None, "power").is_ok());
        assert!(
            validate_qualified_value(Some(0.0), None, "NOT_REPORTED", 0.0, None, "power").is_err()
        );
    }

    #[test]
    fn efficiency_cannot_exceed_one_hundred_percent() {
        assert!(
            validate_qualified_value(Some(100.1), None, "EXACT", 0.0, Some(100.0), "efficiency")
                .is_err()
        );
        assert!(
            validate_qualified_value(Some(99.9), None, "EXACT", 0.0, Some(100.0), "efficiency")
                .is_ok()
        );
    }

    #[test]
    fn exact_and_approximate_values_require_a_value() {
        assert!(validate_qualified_value(None, None, "APPROX", 0.0, None, "power").is_err());
        assert!(validate_qualified_value(Some(12.0), None, "APPROX", 0.0, None, "power").is_ok());
    }
}
