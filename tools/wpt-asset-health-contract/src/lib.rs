// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fail-closed validator for the WPT embedded asset-health evidence contract.
//!
//! This crate implements the deliberately small JSON Schema subset used by
//! asset-observation.schema.json and adds cross-field evidence invariants.
//! It is not a general JSON Schema 2020-12 implementation or a physical
//! condition estimator.

use serde_json::Value;
use std::collections::BTreeSet;

const SCHEMA: &str =
    include_str!("../../../research/wpt-asset-health-v0/asset-observation.schema.json");
const EXAMPLE: &str =
    include_str!("../../../research/wpt-asset-health-v0/asset-observation.synthetic-example.json");
const SCENARIOS: &str = include_str!("../../../research/wpt-asset-health-v0/scenarios.json");
const LIFECYCLE_SCHEMA: &str =
    include_str!("../../../research/wpt-asset-health-v0/asset-lifecycle-outcome.schema.json");
const LIFECYCLE_EXAMPLE: &str = include_str!(
    "../../../research/wpt-asset-health-v0/asset-lifecycle-outcomes.synthetic-example.json"
);

fn fail(path: &str, message: impl AsRef<str>) -> String {
    format!("{path}: {}", message.as_ref())
}

fn matches_type(value: &Value, expected: &str) -> bool {
    match expected {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "number" => value.is_number(),
        "integer" => {
            value.as_i64().is_some()
                || value.as_u64().is_some()
                || value
                    .as_f64()
                    .is_some_and(|number| number.is_finite() && number.fract() == 0.0)
        }
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        _ => false,
    }
}

fn validate_type(schema: &Value, value: &Value, path: &str) -> Result<(), String> {
    let Some(expected) = schema.get("type") else {
        return Ok(());
    };

    let matches = if let Some(name) = expected.as_str() {
        matches_type(value, name)
    } else if let Some(names) = expected.as_array() {
        names
            .iter()
            .any(|name| name.as_str().is_some_and(|name| matches_type(value, name)))
    } else {
        return Err(fail(
            path,
            "schema type must be a string or array of strings",
        ));
    };

    if matches {
        Ok(())
    } else {
        Err(fail(
            path,
            format!("value does not match declared type {expected}"),
        ))
    }
}

fn is_lower_hex_64(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn utc_timestamp_seconds(value: &str) -> Option<i64> {
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

    for (index, byte) in bytes.iter().enumerate() {
        if !matches!(index, 4 | 7 | 10 | 13 | 16 | 19) && !byte.is_ascii_digit() {
            return None;
        }
    }

    let year = value[0..4].parse::<i64>().ok()?;
    let month = value[5..7].parse::<i64>().ok()?;
    let day = value[8..10].parse::<i64>().ok()?;
    let hour = value[11..13].parse::<i64>().ok()?;
    let minute = value[14..16].parse::<i64>().ok()?;
    let second = value[17..19].parse::<i64>().ok()?;
    if year == 0 || !(1..=12).contains(&month) || hour > 23 || minute > 59 || second > 59 {
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
    if !(1..=days_in_month).contains(&day) {
        return None;
    }

    // Howard Hinnant's Gregorian civil-date conversion, with the Unix epoch
    // as day zero. All accepted timestamps use the fixed UTC Z form above.
    let adjusted_year = year - if month <= 2 { 1 } else { 0 };
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year - era * 400;
    let shifted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days_since_epoch = era * 146_097 + day_of_era - 719_468;

    Some(days_since_epoch * 86_400 + hour * 3_600 + minute * 60 + second)
}

fn is_utc_timestamp(value: &str) -> bool {
    utc_timestamp_seconds(value).is_some()
}

fn validate_schema_node(schema: &Value, value: &Value, path: &str) -> Result<(), String> {
    let schema_object = schema
        .as_object()
        .ok_or_else(|| fail(path, "schema node must be an object"))?;

    validate_type(schema, value, path)?;

    if let Some(expected) = schema.get("const")
        && value != expected
    {
        return Err(fail(path, format!("value must equal const {expected}")));
    }

    if let Some(allowed) = schema.get("enum") {
        let values = allowed
            .as_array()
            .ok_or_else(|| fail(path, "schema enum must be an array"))?;
        if !values.contains(value) {
            return Err(fail(path, "value is outside declared enum"));
        }
    }

    if let Some(text) = value.as_str() {
        if let Some(minimum) = schema.get("minLength").and_then(Value::as_u64)
            && text.chars().count() < minimum as usize
        {
            return Err(fail(
                path,
                format!("string is shorter than minLength {minimum}"),
            ));
        }
        if let Some(pattern) = schema.get("pattern").and_then(Value::as_str) {
            if pattern == "^[0-9a-f]{64}$" {
                if !is_lower_hex_64(text) {
                    return Err(fail(
                        path,
                        "string must be 64 lowercase hexadecimal characters",
                    ));
                }
            } else {
                return Err(fail(path, format!("unsupported schema pattern {pattern}")));
            }
        }
        if let Some(format) = schema.get("format").and_then(Value::as_str) {
            if format == "date-time" {
                if !is_utc_timestamp(text) {
                    return Err(fail(path, "timestamp must be UTC YYYY-MM-DDTHH:MM:SSZ"));
                }
            } else {
                return Err(fail(path, format!("unsupported schema format {format}")));
            }
        }
    }

    if let Some(number) = value.as_f64() {
        if let Some(minimum) = schema.get("minimum").and_then(Value::as_f64)
            && number < minimum
        {
            return Err(fail(path, format!("number is below minimum {minimum}")));
        }
        if let Some(maximum) = schema.get("maximum").and_then(Value::as_f64)
            && number > maximum
        {
            return Err(fail(path, format!("number is above maximum {maximum}")));
        }
        if let Some(minimum) = schema.get("exclusiveMinimum").and_then(Value::as_f64)
            && number <= minimum
        {
            return Err(fail(path, format!("number must be greater than {minimum}")));
        }
    }

    if let Some(object) = value.as_object() {
        if let Some(required) = schema.get("required") {
            let required = required
                .as_array()
                .ok_or_else(|| fail(path, "schema required must be an array"))?;
            for field in required {
                let field = field
                    .as_str()
                    .ok_or_else(|| fail(path, "required entries must be strings"))?;
                if !object.contains_key(field) {
                    return Err(fail(path, format!("missing required property {field}")));
                }
            }
        }

        let properties = schema
            .get("properties")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();

        if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
            for key in object.keys() {
                if !properties.contains_key(key) {
                    return Err(fail(path, format!("unknown property {key}")));
                }
            }
        }

        for (name, property_schema) in &properties {
            if let Some(property_value) = object.get(name) {
                validate_schema_node(property_schema, property_value, &format!("{path}.{name}"))?;
            }
        }
    }

    if let Some(array) = value.as_array() {
        if schema.get("uniqueItems") == Some(&Value::Bool(true)) {
            for index in 0..array.len() {
                if array[..index].contains(&array[index]) {
                    return Err(fail(path, "array items must be unique"));
                }
            }
        }

        if let Some(item_schema) = schema.get("items") {
            for (index, item) in array.iter().enumerate() {
                validate_schema_node(item_schema, item, &format!("{path}[{index}]"))?;
            }
        }
    }

    let supported = [
        "$schema",
        "$id",
        "title",
        "description",
        "type",
        "additionalProperties",
        "required",
        "properties",
        "items",
        "const",
        "enum",
        "minLength",
        "minimum",
        "maximum",
        "exclusiveMinimum",
        "uniqueItems",
        "pattern",
        "format",
    ];
    for keyword in schema_object.keys() {
        if !supported.contains(&keyword.as_str()) {
            return Err(fail(path, format!("unsupported schema keyword {keyword}")));
        }
    }

    Ok(())
}

fn object_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a Value, String> {
    value
        .as_object()
        .and_then(|object| object.get(key))
        .ok_or_else(|| fail(path, format!("missing object field {key}")))
}

fn string_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    object_field(value, key, path)?
        .as_str()
        .ok_or_else(|| fail(path, format!("{key} must be a string")))
}

/// Validate one observation using the checked-in schema subset and
/// evidence-specific cross-field invariants.
pub fn validate_observation(schema: &Value, observation: &Value) -> Result<(), String> {
    validate_schema_node(schema, observation, "$")?;

    let evidence_kind = string_field(observation, "evidence_kind", "$")?;
    let exposure = object_field(observation, "exposure", "$")?;
    if let (Some(minimum), Some(maximum)) = (
        exposure.get("ambient_min_c").and_then(Value::as_f64),
        exposure.get("ambient_max_c").and_then(Value::as_f64),
    ) && minimum > maximum
    {
        return Err(fail(
            "$.exposure",
            "ambient_min_c cannot exceed ambient_max_c",
        ));
    }

    let quality_flags = object_field(observation, "quality_flags", "$")?
        .as_array()
        .ok_or_else(|| fail("$.quality_flags", "must be an array"))?;

    let marked_synthetic = quality_flags.iter().any(|flag| flag == "SYNTHETIC_ONLY");
    if evidence_kind == "SYNTHETIC_KNOWN_TRUTH" && !marked_synthetic {
        return Err(fail(
            "$.quality_flags",
            "synthetic evidence must carry SYNTHETIC_ONLY",
        ));
    }
    if evidence_kind != "SYNTHETIC_KNOWN_TRUTH" && marked_synthetic {
        return Err(fail(
            "$.quality_flags",
            "SYNTHETIC_ONLY cannot be attached to non-synthetic evidence",
        ));
    }

    if evidence_kind == "MEASURED" {
        let provenance = object_field(observation, "provenance", "$")?;
        for key in ["instrument_ids", "calibration_receipt_ids"] {
            let entries = object_field(provenance, key, "$.provenance")?
                .as_array()
                .ok_or_else(|| fail("$.provenance", format!("{key} must be an array")))?;
            if entries.is_empty() {
                return Err(fail(
                    "$.provenance",
                    format!("measured evidence requires non-empty {key}"),
                ));
            }
        }
        if object_field(provenance, "raw_artifact_sha256", "$.provenance")?.is_null() {
            return Err(fail(
                "$.provenance.raw_artifact_sha256",
                "measured evidence requires a raw artifact digest",
            ));
        }
    }

    if evidence_kind == "PUBLISHED_MEASURED" {
        let provenance = object_field(observation, "provenance", "$")?;
        let locator = object_field(provenance, "source_locator", "$.provenance")?;
        if locator.as_str().is_none_or(|value| value.trim().is_empty()) {
            return Err(fail(
                "$.provenance.source_locator",
                "published-measured evidence requires a source locator",
            ));
        }
    }

    let asset_context = object_field(observation, "asset_context", "$")?;
    let frame = asset_context
        .get("coordinate_frame_id")
        .and_then(Value::as_str);
    for (group, profile) in [
        ("structural", "spatial_strain_profile"),
        ("structural", "curvature_profile"),
        ("thermal", "spatial_temperature_profile"),
        ("thermal", "spatial_heat_flux_profile"),
    ] {
        let group_value = object_field(observation, group, "$")?;
        let points = object_field(group_value, profile, &format!("$.{group}"))?
            .as_array()
            .ok_or_else(|| fail(&format!("$.{group}.{profile}"), "must be an array"))?;
        if !points.is_empty() && frame.is_none() {
            return Err(fail(
                "$.asset_context.coordinate_frame_id",
                "spatial profiles require a declared asset coordinate frame",
            ));
        }
        if let Some(frame) = frame {
            for (index, point) in points.iter().enumerate() {
                let point_frame = string_field(
                    point,
                    "coordinate_frame_id",
                    &format!("$.{group}.{profile}[{index}]"),
                )?;
                if point_frame != frame {
                    return Err(fail(
                        &format!("$.{group}.{profile}[{index}].coordinate_frame_id"),
                        "spatial point frame differs from asset coordinate frame",
                    ));
                }
            }
        }
    }

    Ok(())
}

/// Validate the checked-in scenario manifest's identity and evidence labels.
pub fn validate_scenarios(scenarios: &Value) -> Result<(), String> {
    let cases = object_field(scenarios, "cases", "$")?
        .as_array()
        .ok_or_else(|| fail("$.cases", "must be an array"))?;
    let mut identifiers = BTreeSet::new();

    for (index, case) in cases.iter().enumerate() {
        let path = format!("$.cases[{index}]");
        let case_id = string_field(case, "case_id", &path)?;
        if !identifiers.insert(case_id) {
            return Err(fail(&path, format!("duplicate case_id {case_id}")));
        }
        let evidence_kind = string_field(case, "evidence_kind", &path)?;
        if ![
            "MEASURED",
            "PUBLISHED_MEASURED",
            "SIMULATED",
            "SYNTHETIC_KNOWN_TRUTH",
            "INFERRED",
        ]
        .contains(&evidence_kind)
        {
            return Err(fail(&path, "unknown evidence_kind"));
        }
    }

    Ok(())
}

/// Validate one time-to-event dataset, preserving right-censored and
/// competing-risk observations as distinct outcomes.
pub fn validate_lifecycle_outcomes(schema: &Value, dataset: &Value) -> Result<(), String> {
    validate_schema_node(schema, dataset, "$")?;

    let records = object_field(dataset, "records", "$")?
        .as_array()
        .ok_or_else(|| fail("$.records", "must be an array"))?;
    if records.is_empty() {
        return Err(fail(
            "$.records",
            "at least one lifecycle outcome is required",
        ));
    }

    let mut outcome_ids = BTreeSet::new();
    let mut asset_ids = BTreeSet::new();
    let mut group_splits = std::collections::BTreeMap::<(String, String), String>::new();

    for (index, record) in records.iter().enumerate() {
        let path = format!("$.records[{index}]");
        let outcome_id = string_field(record, "outcome_id", &path)?;
        let asset_id = string_field(record, "asset_id", &path)?;
        if !outcome_ids.insert(outcome_id.to_string()) {
            return Err(fail(&path, format!("duplicate outcome_id {outcome_id}")));
        }
        if !asset_ids.insert(asset_id.to_string()) {
            return Err(fail(
                &path,
                format!(
                    "duplicate asset_id {asset_id}; one first-event outcome per asset is permitted"
                ),
            ));
        }

        let start = string_field(record, "at_risk_start_utc", &path)?;
        let end = string_field(record, "outcome_or_censor_utc", &path)?;
        let start_seconds = utc_timestamp_seconds(start)
            .ok_or_else(|| fail(&path, "invalid at-risk start timestamp"))?;
        let end_seconds = utc_timestamp_seconds(end)
            .ok_or_else(|| fail(&path, "invalid outcome/censor timestamp"))?;
        if end_seconds <= start_seconds {
            return Err(fail(&path, "outcome/censor time must follow at-risk start"));
        }

        let declared_hours = object_field(record, "duration_hours", &path)?
            .as_f64()
            .ok_or_else(|| fail(&path, "duration_hours must be numeric"))?;
        let derived_hours = (end_seconds - start_seconds) as f64 / 3_600.0;
        if (declared_hours - derived_hours).abs() > 1e-9 {
            return Err(fail(
                &path,
                format!("duration_hours does not match timestamps (expected {derived_hours})"),
            ));
        }

        let outcome_kind = string_field(record, "outcome_kind", &path)?;
        let event_code = object_field(record, "event_code", &path)?;
        let censor_reason = object_field(record, "censor_reason", &path)?;
        let quality_flags = object_field(record, "quality_flags", &path)?
            .as_array()
            .ok_or_else(|| fail(&path, "quality_flags must be an array"))?;
        let has_flag = |name: &str| quality_flags.iter().any(|flag| flag == name);

        match outcome_kind {
            "TARGET_FAILURE" => {
                let code = event_code
                    .as_str()
                    .ok_or_else(|| fail(&path, "target failure requires an event_code"))?;
                if [
                    "PLANNED_REPLACEMENT",
                    "UNRELATED_ASSET_FAILURE",
                    "ROAD_RECONSTRUCTION",
                ]
                .contains(&code)
                {
                    return Err(fail(
                        &path,
                        "competing-event code cannot represent the target failure",
                    ));
                }
                if !censor_reason.is_null() {
                    return Err(fail(&path, "target failure cannot carry a censor_reason"));
                }
                if has_flag("RIGHT_CENSORED") || has_flag("COMPETING_RISK") {
                    return Err(fail(
                        &path,
                        "target failure cannot carry censoring or competing-risk flags",
                    ));
                }
            }
            "RIGHT_CENSORED" => {
                if !event_code.is_null() {
                    return Err(fail(
                        &path,
                        "right-censored record must not carry an event_code",
                    ));
                }
                if censor_reason
                    .as_str()
                    .is_none_or(|reason| reason.trim().is_empty())
                {
                    return Err(fail(
                        &path,
                        "right-censored record requires a censor_reason",
                    ));
                }
                if !has_flag("RIGHT_CENSORED") {
                    return Err(fail(
                        &path,
                        "right-censored record must carry RIGHT_CENSORED",
                    ));
                }
                if has_flag("COMPETING_RISK") {
                    return Err(fail(
                        &path,
                        "right-censored record cannot carry COMPETING_RISK",
                    ));
                }
            }
            "COMPETING_EVENT" => {
                let code = event_code
                    .as_str()
                    .ok_or_else(|| fail(&path, "competing event requires an event_code"))?;
                if ![
                    "PLANNED_REPLACEMENT",
                    "UNRELATED_ASSET_FAILURE",
                    "ROAD_RECONSTRUCTION",
                ]
                .contains(&code)
                {
                    return Err(fail(
                        &path,
                        "competing event requires a declared competing-event code",
                    ));
                }
                if !censor_reason.is_null() {
                    return Err(fail(&path, "competing event cannot carry a censor_reason"));
                }
                if !has_flag("COMPETING_RISK") {
                    return Err(fail(&path, "competing event must carry COMPETING_RISK"));
                }
                if has_flag("RIGHT_CENSORED") {
                    return Err(fail(&path, "competing event cannot carry RIGHT_CENSORED"));
                }
            }
            _ => return Err(fail(&path, "unknown outcome_kind")),
        }

        let evidence_kind = string_field(record, "evidence_kind", &path)?;
        if evidence_kind == "SYNTHETIC_KNOWN_TRUTH" && !has_flag("SYNTHETIC_ONLY") {
            return Err(fail(
                &path,
                "synthetic lifecycle evidence must carry SYNTHETIC_ONLY",
            ));
        }
        if evidence_kind != "SYNTHETIC_KNOWN_TRUTH" && has_flag("SYNTHETIC_ONLY") {
            return Err(fail(
                &path,
                "SYNTHETIC_ONLY cannot be attached to non-synthetic evidence",
            ));
        }
        let source_records = object_field(record, "source_record_ids", &path)?
            .as_array()
            .ok_or_else(|| fail(&path, "source_record_ids must be an array"))?;
        match evidence_kind {
            "MEASURED" => {
                if source_records.is_empty() {
                    return Err(fail(
                        &path,
                        "measured lifecycle outcomes require source_record_ids",
                    ));
                }
                if object_field(record, "raw_artifact_sha256", &path)?.is_null() {
                    return Err(fail(
                        &path,
                        "measured lifecycle outcomes require a raw artifact digest",
                    ));
                }
            }
            "PUBLISHED_MEASURED" => {
                if source_records.is_empty() {
                    return Err(fail(
                        &path,
                        "published-measured lifecycle outcomes require source_record_ids",
                    ));
                }
                let locator = object_field(record, "source_locator", &path)?;
                if locator.as_str().is_none_or(|value| value.trim().is_empty()) {
                    return Err(fail(
                        &path,
                        "published-measured lifecycle outcomes require a source locator",
                    ));
                }
            }
            "SIMULATED" | "INFERRED" | "SYNTHETIC_KNOWN_TRUTH" => {}
            _ => return Err(fail(&path, "unknown lifecycle evidence_kind")),
        }

        let split = string_field(record, "split", &path)?.to_string();
        for group_key in [
            "asset_id",
            "segment_id",
            "material_lot_id",
            "installation_id",
            "route_id",
            "environmental_campaign_id",
        ] {
            let group_value = object_field(record, group_key, &path)?;
            if let Some(value) = group_value
                .as_str()
                .filter(|value| !value.trim().is_empty())
            {
                let key = (group_key.to_string(), value.to_string());
                if let Some(previous_split) = group_splits.get(&key) {
                    if previous_split != &split {
                        return Err(fail(
                            &path,
                            format!(
                                "{group_key} {value} leaks across {previous_split} and {split} splits"
                            ),
                        ));
                    }
                } else {
                    group_splits.insert(key, split.clone());
                }
            }
        }
    }

    Ok(())
}

/// Load the checked-in synthetic survival dataset and its schema.
pub fn bundled_lifecycle_fixtures() -> Result<(Value, Value), String> {
    let schema: Value = serde_json::from_str(LIFECYCLE_SCHEMA)
        .map_err(|error| format!("lifecycle schema JSON: {error}"))?;
    let example: Value = serde_json::from_str(LIFECYCLE_EXAMPLE)
        .map_err(|error| format!("lifecycle example JSON: {error}"))?;
    Ok((schema, example))
}

/// Return the checked-in schema, synthetic example and scenario manifest.
pub fn bundled_fixtures() -> Result<(Value, Value, Value), String> {
    let schema: Value =
        serde_json::from_str(SCHEMA).map_err(|error| format!("schema JSON: {error}"))?;
    let example: Value =
        serde_json::from_str(EXAMPLE).map_err(|error| format!("example JSON: {error}"))?;
    let scenarios: Value =
        serde_json::from_str(SCENARIOS).map_err(|error| format!("scenarios JSON: {error}"))?;
    Ok((schema, example, scenarios))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_schema_integer_type_accepts_integral_float_values() {
        assert!(matches_type(&Value::Null, "null"));
        assert!(matches_type(&Value::from(1.0), "integer"));
        assert!(!matches_type(&Value::from(1.5), "integer"));
    }

    #[test]
    fn measured_lifecycle_outcome_requires_raw_source_evidence() {
        let (schema, mut dataset) = bundled_lifecycle_fixtures().unwrap();
        dataset["records"][0]["evidence_kind"] = Value::String("MEASURED".into());
        dataset["records"][0]["quality_flags"] = Value::Array(vec![]);
        dataset["records"][0]["raw_artifact_sha256"] = Value::Null;
        assert!(
            validate_lifecycle_outcomes(&schema, &dataset)
                .unwrap_err()
                .contains("require a raw artifact digest")
        );
    }

    #[test]
    fn measured_lifecycle_outcome_requires_at_least_one_source_record_id() {
        let (schema, mut dataset) = bundled_lifecycle_fixtures().unwrap();
        dataset["records"][0]["evidence_kind"] = Value::String("MEASURED".into());
        dataset["records"][0]["quality_flags"] = Value::Array(vec![]);
        dataset["records"][0]["source_record_ids"] = Value::Array(vec![]);
        dataset["records"][0]["raw_artifact_sha256"] = Value::String("d".repeat(64));
        assert!(
            validate_lifecycle_outcomes(&schema, &dataset)
                .unwrap_err()
                .contains("measured lifecycle outcomes require source_record_ids")
        );
    }

    #[test]
    fn published_lifecycle_outcome_requires_source_locator() {
        let (schema, mut dataset) = bundled_lifecycle_fixtures().unwrap();
        dataset["records"][0]["evidence_kind"] = Value::String("PUBLISHED_MEASURED".into());
        dataset["records"][0]["quality_flags"] = Value::Array(vec![]);
        dataset["records"][0]["source_locator"] = Value::Null;
        assert!(
            validate_lifecycle_outcomes(&schema, &dataset)
                .unwrap_err()
                .contains("require a source locator")
        );
    }

    #[test]
    fn lifecycle_fixture_keeps_failure_censoring_and_competing_risk_distinct() {
        let (schema, dataset) = bundled_lifecycle_fixtures().unwrap();
        validate_lifecycle_outcomes(&schema, &dataset).unwrap();
        assert_eq!(dataset["records"][0]["outcome_kind"], "TARGET_FAILURE");
        assert_eq!(dataset["records"][1]["outcome_kind"], "RIGHT_CENSORED");
        assert!(dataset["records"][1]["event_code"].is_null());
        assert_eq!(dataset["records"][2]["outcome_kind"], "COMPETING_EVENT");
    }

    #[test]
    fn right_censored_record_cannot_be_relabelled_as_target_failure() {
        let (schema, mut dataset) = bundled_lifecycle_fixtures().unwrap();
        dataset["records"][1]["outcome_kind"] = Value::String("TARGET_FAILURE".into());
        assert!(
            validate_lifecycle_outcomes(&schema, &dataset)
                .unwrap_err()
                .contains("target failure requires an event_code")
        );
    }

    #[test]
    fn repeated_asset_or_segment_groups_cannot_cross_splits() {
        let (schema, mut dataset) = bundled_lifecycle_fixtures().unwrap();
        let mut leaked = dataset["records"][0].clone();
        leaked["outcome_id"] = Value::String("synthetic-outcome-leaked".into());
        leaked["asset_id"] = Value::String("synthetic-asset-leaked".into());
        leaked["segment_id"] = Value::String("synthetic-segment-leaked".into());
        leaked["split"] = Value::String("TEST".into());
        dataset["records"].as_array_mut().unwrap().push(leaked);
        assert!(
            validate_lifecycle_outcomes(&schema, &dataset)
                .unwrap_err()
                .contains("material_lot_id synthetic-lot-a leaks across TRAIN and TEST splits")
        );
    }

    #[test]
    fn lifecycle_duration_must_match_utc_event_or_censor_times() {
        let (schema, mut dataset) = bundled_lifecycle_fixtures().unwrap();
        dataset["records"][1]["duration_hours"] = Value::from(1_000.0);
        assert!(
            validate_lifecycle_outcomes(&schema, &dataset)
                .unwrap_err()
                .contains("duration_hours does not match timestamps")
        );
    }

    #[test]
    fn duplicate_asset_outcomes_are_rejected_for_first_event_estimand() {
        let (schema, mut dataset) = bundled_lifecycle_fixtures().unwrap();
        let mut duplicate = dataset["records"][0].clone();
        duplicate["outcome_id"] = Value::String("another-outcome".into());
        dataset["records"].as_array_mut().unwrap().push(duplicate);
        assert!(
            validate_lifecycle_outcomes(&schema, &dataset)
                .unwrap_err()
                .contains("duplicate asset_id")
        );
    }

    #[test]
    fn target_failure_must_not_be_marked_right_censored() {
        let (schema, mut dataset) = bundled_lifecycle_fixtures().unwrap();
        dataset["records"][0]["quality_flags"] = Value::Array(vec![
            Value::String("SYNTHETIC_ONLY".into()),
            Value::String("RIGHT_CENSORED".into()),
        ]);
        assert!(
            validate_lifecycle_outcomes(&schema, &dataset)
                .unwrap_err()
                .contains("target failure cannot carry censoring")
        );
    }

    #[test]
    fn checked_in_schema_example_and_scenarios_validate() {
        let (schema, example, scenarios) = bundled_fixtures().unwrap();
        validate_observation(&schema, &example).unwrap();
        validate_scenarios(&scenarios).unwrap();
        assert_eq!(
            example["evidence_kind"],
            Value::String("SYNTHETIC_KNOWN_TRUTH".into())
        );
        assert_eq!(scenarios["cases"].as_array().unwrap().len(), 16);
    }

    #[test]
    fn duplicate_quality_flags_are_rejected_by_unique_items_keyword() {
        let (schema, mut example, _) = bundled_fixtures().unwrap();
        example["quality_flags"] = Value::Array(vec![
            Value::String("SYNTHETIC_ONLY".into()),
            Value::String("SYNTHETIC_ONLY".into()),
        ]);
        assert!(
            validate_observation(&schema, &example)
                .unwrap_err()
                .contains("array items must be unique")
        );
    }

    #[test]
    fn validator_fails_closed_on_unimplemented_schema_keywords() {
        let (mut schema, example, _) = bundled_fixtures().unwrap();
        schema["properties"]["record_id"]["minItems"] = Value::from(1);
        assert!(
            validate_observation(&schema, &example)
                .unwrap_err()
                .contains("unsupported schema keyword minItems")
        );
    }

    #[test]
    fn impossible_utc_calendar_date_is_rejected() {
        let (schema, mut example, _) = bundled_fixtures().unwrap();
        example["observed_at_utc"] = Value::String("2026-02-30T00:00:00Z".into());
        assert!(
            validate_observation(&schema, &example)
                .unwrap_err()
                .contains("timestamp must be UTC")
        );
    }

    #[test]
    fn ambient_temperature_minimum_cannot_exceed_maximum() {
        let (schema, mut example, _) = bundled_fixtures().unwrap();
        example["exposure"]["ambient_min_c"] = Value::from(30);
        example["exposure"]["ambient_max_c"] = Value::from(20);
        assert!(
            validate_observation(&schema, &example)
                .unwrap_err()
                .contains("ambient_min_c cannot exceed ambient_max_c")
        );
    }

    #[test]
    fn missing_spatial_profile_is_rejected() {
        let (schema, mut example, _) = bundled_fixtures().unwrap();
        example["structural"]
            .as_object_mut()
            .unwrap()
            .remove("spatial_strain_profile");
        assert!(
            validate_observation(&schema, &example)
                .unwrap_err()
                .contains("spatial_strain_profile")
        );
    }

    #[test]
    fn mismatched_coordinate_frame_is_rejected() {
        let (schema, mut example, _) = bundled_fixtures().unwrap();
        example["thermal"]["spatial_temperature_profile"][0]["coordinate_frame_id"] =
            Value::String("other-frame".into());
        assert!(
            validate_observation(&schema, &example)
                .unwrap_err()
                .contains("differs from asset coordinate frame")
        );
    }

    #[test]
    fn synthetic_evidence_requires_synthetic_only_flag() {
        let (schema, mut example, _) = bundled_fixtures().unwrap();
        example["quality_flags"] = Value::Array(vec![]);
        assert!(
            validate_observation(&schema, &example)
                .unwrap_err()
                .contains("SYNTHETIC_ONLY")
        );
    }

    #[test]
    fn coupling_coefficient_outside_physical_range_is_rejected() {
        let (schema, mut example, _) = bundled_fixtures().unwrap();
        example["electromagnetic"]["coupling_coefficient"] = Value::from(1.01);
        assert!(
            validate_observation(&schema, &example)
                .unwrap_err()
                .contains("above maximum")
        );
    }

    #[test]
    fn unknown_properties_are_rejected() {
        let (schema, mut example, _) = bundled_fixtures().unwrap();
        example["unreviewed_confidence"] = Value::from(0.99);
        assert!(
            validate_observation(&schema, &example)
                .unwrap_err()
                .contains("unknown property")
        );
    }

    #[test]
    fn measured_evidence_requires_calibration_and_raw_data() {
        let (schema, mut example, _) = bundled_fixtures().unwrap();
        example["evidence_kind"] = Value::String("MEASURED".into());
        example["quality_flags"] = Value::Array(vec![]);
        example["provenance"]["instrument_ids"] =
            Value::Array(vec![Value::String("reference-meter-01".into())]);
        example["provenance"]["calibration_receipt_ids"] = Value::Array(vec![]);
        assert!(
            validate_observation(&schema, &example)
                .unwrap_err()
                .contains("calibration_receipt_ids")
        );
    }

    #[test]
    fn missing_spatial_heat_flux_profile_is_rejected() {
        let (schema, mut example, _) = bundled_fixtures().unwrap();
        example["thermal"]
            .as_object_mut()
            .unwrap()
            .remove("spatial_heat_flux_profile");
        assert!(
            validate_observation(&schema, &example)
                .unwrap_err()
                .contains("spatial_heat_flux_profile")
        );
    }

    #[test]
    fn heat_flux_profile_coordinate_frame_is_checked() {
        let (schema, mut example, _) = bundled_fixtures().unwrap();
        example["thermal"]["spatial_heat_flux_profile"][0]["coordinate_frame_id"] =
            Value::String("other-frame".into());
        assert!(
            validate_observation(&schema, &example)
                .unwrap_err()
                .contains("spatial point frame differs from asset coordinate frame")
        );
    }

    #[test]
    fn nonempty_spatial_profile_requires_asset_coordinate_frame() {
        let (schema, mut example, _) = bundled_fixtures().unwrap();
        example["asset_context"]["coordinate_frame_id"] = Value::Null;
        assert!(
            validate_observation(&schema, &example)
                .unwrap_err()
                .contains("require a declared asset coordinate frame")
        );
    }

    #[test]
    fn published_measured_evidence_requires_source_locator() {
        let (schema, mut example, _) = bundled_fixtures().unwrap();
        example["evidence_kind"] = Value::String("PUBLISHED_MEASURED".into());
        example["quality_flags"] = Value::Array(vec![]);
        example["provenance"]["source_locator"] = Value::Null;
        assert!(
            validate_observation(&schema, &example)
                .unwrap_err()
                .contains("requires a source locator")
        );
    }

    #[test]
    fn synthetic_only_flag_is_rejected_for_measured_evidence() {
        let (schema, mut example, _) = bundled_fixtures().unwrap();
        example["evidence_kind"] = Value::String("PUBLISHED_MEASURED".into());
        assert!(
            validate_observation(&schema, &example)
                .unwrap_err()
                .contains("cannot be attached to non-synthetic")
        );
    }

    #[test]
    fn scenarios_require_unique_ids_and_known_evidence_classes() {
        let (_, _, mut scenarios) = bundled_fixtures().unwrap();
        let duplicate = scenarios["cases"][0].clone();
        scenarios["cases"].as_array_mut().unwrap().push(duplicate);
        assert!(
            validate_scenarios(&scenarios)
                .unwrap_err()
                .contains("duplicate case_id")
        );
    }
}
