// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process;

type ValidationResult<T> = Result<T, String>;

const MATERIAL_KEYS: &[&str] = &[
    "schema_version",
    "material_id",
    "catalogue_type",
    "material_family",
    "grade",
    "manufacturer",
    "composition_description",
    "operating_frequency_hz_min",
    "operating_frequency_hz_max",
    "operating_temperature_c_min",
    "operating_temperature_c_max",
    "properties",
    "saturation_curve",
    "source_refs",
    "data_status",
    "notes",
];

const PROPERTY_KEYS: &[&str] = &[
    "property_kind",
    "value",
    "unit",
    "frequency_hz",
    "temperature_c",
    "flux_density_t",
    "test_method",
    "evidence_kind",
    "source_locator",
    "uncertainty",
];

const CURVE_POINT_KEYS: &[&str] = &[
    "temperature_c",
    "saturation_flux_density_t",
    "evidence_kind",
    "source_locator",
    "uncertainty",
];

const SOURCE_REF_KEYS: &[&str] = &["source_id", "role", "locator"];

fn exact_keys(value: &Value, expected: &[&str], label: &str) -> ValidationResult<()> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{label}: expected JSON object"))?;
    for key in expected {
        if !object.contains_key(*key) {
            return Err(format!("{label}: missing required field {key}"));
        }
    }
    for key in object.keys() {
        if !expected.contains(&key.as_str()) {
            return Err(format!("{label}: unexpected field {key}"));
        }
    }
    Ok(())
}

fn required_string(value: &Value, key: &str, label: &str) -> ValidationResult<String> {
    let field = value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{label}.{key}: expected string"))?;
    if field.trim().is_empty() {
        return Err(format!("{label}.{key}: must not be empty"));
    }
    Ok(field.to_owned())
}

fn optional_string(value: &Value, key: &str, label: &str) -> ValidationResult<Option<String>> {
    match value.get(key) {
        Some(Value::Null) => Ok(None),
        Some(Value::String(field)) if !field.trim().is_empty() => Ok(Some(field.clone())),
        Some(Value::String(_)) => Err(format!("{label}.{key}: empty string is not a null value")),
        Some(_) => Err(format!("{label}.{key}: expected string or null")),
        None => Err(format!("{label}.{key}: missing field")),
    }
}

fn optional_number(value: &Value, key: &str, label: &str) -> ValidationResult<Option<f64>> {
    match value.get(key) {
        Some(Value::Null) => Ok(None),
        Some(number) => {
            let parsed = number
                .as_f64()
                .ok_or_else(|| format!("{label}.{key}: expected finite number or null"))?;
            if !parsed.is_finite() {
                return Err(format!("{label}.{key}: value must be finite"));
            }
            Ok(Some(parsed))
        }
        None => Err(format!("{label}.{key}: missing field")),
    }
}

fn valid_id(value: &str) -> bool {
    value.starts_with("WPT-MAT-")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'-')
}

fn is_one_of(value: &str, allowed: &[&str]) -> bool {
    allowed.contains(&value)
}

fn read_jsonl(path: &Path) -> Result<Vec<Value>, Box<dyn Error>> {
    let file = File::open(path)?;
    let mut rows = Vec::new();
    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            return Err(format!(
                "{} line {}: blank JSONL rows are not allowed",
                path.display(),
                index + 1
            )
            .into());
        }
        let value: Value = serde_json::from_str(&line).map_err(|error| {
            format!("{} line {}: invalid JSON: {error}", path.display(), index + 1)
        })?;
        rows.push(value);
    }
    if rows.is_empty() {
        return Err(format!("{}: material catalogue is empty", path.display()).into());
    }
    Ok(rows)
}

fn validate_source_refs(
    material: &Value,
    known_sources: &BTreeSet<String>,
    label: &str,
) -> ValidationResult<BTreeSet<String>> {
    let refs = material
        .get("source_refs")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{label}.source_refs: expected array"))?;
    if refs.is_empty() {
        return Err(format!("{label}.source_refs: at least one source reference is required"));
    }
    let mut ids = BTreeSet::new();
    let mut has_datasheet = false;
    for (index, source_ref) in refs.iter().enumerate() {
        let ref_label = format!("{label}.source_refs[{index}]");
        exact_keys(source_ref, SOURCE_REF_KEYS, &ref_label)?;
        let source_id = required_string(source_ref, "source_id", &ref_label)?;
        if !known_sources.contains(&source_id) {
            return Err(format!("{ref_label}: unresolved source_id {source_id}"));
        }
        if !ids.insert(source_id.clone()) {
            return Err(format!("{ref_label}: duplicate source_id {source_id}"));
        }
        let role = required_string(source_ref, "role", &ref_label)?;
        if !is_one_of(
            &role,
            &[
                "PRIMARY_PUBLICATION",
                "MANUFACTURER_DATASHEET",
                "SURVEY_SECONDARY",
                "OTHER",
            ],
        ) {
            return Err(format!("{ref_label}.role: unsupported source role {role}"));
        }
        if role == "MANUFACTURER_DATASHEET" {
            has_datasheet = true;
        }
        required_string(source_ref, "locator", &ref_label)?;
    }

    if material.get("catalogue_type").and_then(Value::as_str) == Some("COMMERCIAL_GRADE")
        && !has_datasheet
    {
        return Err(format!(
            "{label}: COMMERCIAL_GRADE requires at least one MANUFACTURER_DATASHEET reference"
        ));
    }
    Ok(ids)
}

fn validate_property(value: &Value, label: &str) -> ValidationResult<()> {
    exact_keys(value, PROPERTY_KEYS, label)?;
    let property_kind = required_string(value, "property_kind", label)?;
    if !is_one_of(
        &property_kind,
        &[
            "relative_permeability",
            "loss_tangent",
            "core_loss_density",
            "thermal_conductivity",
            "specific_heat",
            "density",
            "electrical_resistivity",
            "tensile_strength",
            "compressive_strength",
            "flexural_strength",
            "curie_temperature",
            "saturation_flux_density",
            "coefficient_thermal_expansion",
            "other",
        ],
    ) {
        return Err(format!("{label}.property_kind: unsupported kind {property_kind}"));
    }
    let measured_value = optional_number(value, "value", label)?;
    let unit = required_string(value, "unit", label)?;
    if measured_value.is_some() && unit == "unknown" {
        return Err(format!("{label}.unit: numeric property requires an explicit unit"));
    }
    for key in ["frequency_hz", "flux_density_t"] {
        if optional_number(value, key, label)?.is_some_and(|number| number < 0.0) {
            return Err(format!("{label}.{key}: must be nonnegative"));
        }
    }
    let evidence_kind = required_string(value, "evidence_kind", label)?;
    if !is_one_of(
        &evidence_kind,
        &[
            "MEASURED",
            "MANUFACTURER_DATASHEET",
            "AUTHOR_CALCULATED",
            "SIMULATED",
            "SECONDARY_REPORTED",
            "UNKNOWN",
        ],
    ) {
        return Err(format!("{label}.evidence_kind: unsupported kind {evidence_kind}"));
    }
    if measured_value.is_some() && evidence_kind == "UNKNOWN" {
        return Err(format!(
            "{label}: numeric property cannot have UNKNOWN evidence provenance"
        ));
    }
    let test_method = optional_string(value, "test_method", label)?;
    if evidence_kind == "MEASURED" && test_method.is_none() {
        return Err(format!("{label}: MEASURED property requires a test_method"));
    }
    required_string(value, "source_locator", label)?;
    if optional_number(value, "uncertainty", label)?.is_some_and(|number| number < 0.0) {
        return Err(format!("{label}.uncertainty: must be nonnegative"));
    }
    Ok(())
}

fn validate_curve_point(value: &Value, label: &str) -> ValidationResult<f64> {
    exact_keys(value, CURVE_POINT_KEYS, label)?;
    let temperature = value
        .get("temperature_c")
        .and_then(Value::as_f64)
        .filter(|number| number.is_finite())
        .ok_or_else(|| format!("{label}.temperature_c: finite number required"))?;
    let saturation = value
        .get("saturation_flux_density_t")
        .and_then(Value::as_f64)
        .filter(|number| number.is_finite() && *number > 0.0)
        .ok_or_else(|| format!("{label}.saturation_flux_density_t: positive tesla value required"))?;
    let evidence_kind = required_string(value, "evidence_kind", label)?;
    if !is_one_of(
        &evidence_kind,
        &[
            "MEASURED",
            "MANUFACTURER_DATASHEET",
            "AUTHOR_CALCULATED",
            "SIMULATED",
            "SECONDARY_REPORTED",
            "UNKNOWN",
        ],
    ) {
        return Err(format!("{label}.evidence_kind: unsupported kind {evidence_kind}"));
    }
    if evidence_kind == "UNKNOWN" {
        return Err(format!("{label}: Bsat(T) point requires attributable evidence"));
    }
    required_string(value, "source_locator", label)?;
    if optional_number(value, "uncertainty", label)?.is_some_and(|number| number < 0.0) {
        return Err(format!("{label}.uncertainty: must be nonnegative"));
    }
    Ok(temperature + saturation * 0.0)
}

fn validate_material(
    material: &Value,
    index: usize,
    known_sources: &BTreeSet<String>,
) -> ValidationResult<String> {
    let label = format!("material record {}", index + 1);
    exact_keys(material, MATERIAL_KEYS, &label)?;
    if required_string(material, "schema_version", &label)? != "wpt-material-v1" {
        return Err(format!("{label}: unsupported schema version"));
    }
    let material_id = required_string(material, "material_id", &label)?;
    if !valid_id(&material_id) {
        return Err(format!("{label}.material_id: expected WPT-MAT- uppercase identifier"));
    }
    let catalogue_type = required_string(material, "catalogue_type", &label)?;
    if !is_one_of(
        &catalogue_type,
        &[
            "MATERIAL_FAMILY",
            "COMMERCIAL_GRADE",
            "CUSTOM_FORMULATION",
            "ASSEMBLY_COMPOSITE",
        ],
    ) {
        return Err(format!("{label}.catalogue_type: unsupported type {catalogue_type}"));
    }
    required_string(material, "material_family", &label)?;
    let grade = optional_string(material, "grade", &label)?;
    let manufacturer = optional_string(material, "manufacturer", &label)?;
    required_string(material, "notes", &label)?;

    if catalogue_type == "COMMERCIAL_GRADE" && (grade.is_none() || manufacturer.is_none()) {
        return Err(format!(
            "{label}: COMMERCIAL_GRADE requires explicit grade and manufacturer"
        ));
    }
    for key in [
        "operating_frequency_hz_min",
        "operating_frequency_hz_max",
    ] {
        if optional_number(material, key, &label)?.is_some_and(|number| number < 0.0) {
            return Err(format!("{label}.{key}: must be nonnegative"));
        }
    }
    let f_min = optional_number(material, "operating_frequency_hz_min", &label)?;
    let f_max = optional_number(material, "operating_frequency_hz_max", &label)?;
    if matches!((f_min, f_max), (Some(min), Some(max)) if min > max) {
        return Err(format!("{label}: minimum operating frequency exceeds maximum"));
    }
    let t_min = optional_number(material, "operating_temperature_c_min", &label)?;
    let t_max = optional_number(material, "operating_temperature_c_max", &label)?;
    if matches!((t_min, t_max), (Some(min), Some(max)) if min > max) {
        return Err(format!("{label}: minimum operating temperature exceeds maximum"));
    }

    let source_ids = validate_source_refs(material, known_sources, &label)?;
    let properties = material
        .get("properties")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{label}.properties: expected array"))?;
    for (property_index, property) in properties.iter().enumerate() {
        validate_property(
            property,
            &format!("{label}.properties[{property_index}]"),
        )?;
    }

    let curve = material
        .get("saturation_curve")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{label}.saturation_curve: expected array"))?;
    let mut previous_temperature = None;
    for (curve_index, point) in curve.iter().enumerate() {
        let temperature =
            validate_curve_point(point, &format!("{label}.saturation_curve[{curve_index}]"))?;
        if previous_temperature.is_some_and(|previous| temperature <= previous) {
            return Err(format!(
                "{label}.saturation_curve: temperatures must be unique and strictly increasing"
            ));
        }
        previous_temperature = Some(temperature);
    }

    let data_status = required_string(material, "data_status", &label)?;
    if !is_one_of(
        &data_status,
        &[
            "UNRESOLVED",
            "PARTIAL",
            "DUAL_REVIEWED",
            "EXPERT_REVIEWED",
            "RETIRED",
        ],
    ) {
        return Err(format!("{label}.data_status: unsupported status {data_status}"));
    }
    if data_status == "UNRESOLVED" && (!properties.is_empty() || !curve.is_empty()) {
        return Err(format!(
            "{label}: UNRESOLVED family record must not contain numeric properties or Bsat(T) points"
        ));
    }
    if data_status == "EXPERT_REVIEWED" && properties.is_empty() && curve.is_empty() {
        return Err(format!(
            "{label}: EXPERT_REVIEWED material requires at least one attributable property"
        ));
    }
    if source_ids.is_empty() {
        return Err(format!("{label}: source provenance is empty"));
    }
    Ok(material_id)
}

pub fn audit(root: &Path) -> Result<Value, Box<dyn Error>> {
    let base = root.join("benchmarks/wpt-empirical");
    let schema_path = base.join("v1/material-catalog.schema.json");
    let schema: Value = serde_json::from_slice(&fs::read(&schema_path)?)?;
    if schema.get("$schema").and_then(Value::as_str)
        != Some("https://json-schema.org/draft/2020-12/schema")
    {
        return Err("material catalogue schema must declare JSON Schema 2020-12".into());
    }
    let source_rows = read_jsonl(&base.join("v0/sources.jsonl"))?;
    let mut known_sources = BTreeSet::new();
    for (index, source) in source_rows.iter().enumerate() {
        let source_id = required_string(source, "source_id", &format!("source record {}", index + 1))?;
        if !known_sources.insert(source_id.clone()) {
            return Err(format!("duplicate source_id {source_id}").into());
        }
    }

    let materials = read_jsonl(&base.join("v1/materials.jsonl"))?;
    let mut ids = BTreeSet::new();
    let mut unresolved_families = 0usize;
    let mut property_count = 0usize;
    let mut curve_point_count = 0usize;
    for (index, material) in materials.iter().enumerate() {
        let material_id = validate_material(material, index, &known_sources)?;
        if !ids.insert(material_id.clone()) {
            return Err(format!("duplicate material_id {material_id}").into());
        }
        if material.get("data_status").and_then(Value::as_str) == Some("UNRESOLVED") {
            unresolved_families += 1;
        }
        property_count += material["properties"].as_array().map_or(0, Vec::len);
        curve_point_count += material["saturation_curve"].as_array().map_or(0, Vec::len);
    }
    Ok(json!({
        "valid": true,
        "material_count": materials.len(),
        "unresolved_family_count": unresolved_families,
        "numeric_property_record_count": property_count,
        "saturation_curve_point_count": curve_point_count,
        "source_count": known_sources.len()
    }))
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "wpt-material-catalog-validator".into());
    let root = args.next().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    if args.next().is_some() {
        eprintln!("usage: {program} [repository-root]");
        process::exit(2);
    }
    match audit(&root) {
        Ok(report) => {
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(())
        }
        Err(error) => {
            eprintln!("WPT material catalogue rejected: {error}");
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repository_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    #[test]
    fn checked_in_material_catalogue_has_resolved_provenance() {
        let report = audit(&repository_root()).expect("material catalogue should validate");
        assert_eq!(report["valid"], Value::Bool(true));
        assert_eq!(report["material_count"], json!(5));
        assert_eq!(report["unresolved_family_count"], json!(5));
        assert_eq!(report["numeric_property_record_count"], json!(0));
        assert_eq!(report["saturation_curve_point_count"], json!(0));
    }

    #[test]
    fn rejects_duplicate_or_non_increasing_temperature_curve_points() {
        let mut point = json!({
            "temperature_c": 25.0,
            "saturation_flux_density_t": 0.53,
            "evidence_kind": "MANUFACTURER_DATASHEET",
            "source_locator": "datasheet page 2",
            "uncertainty": null
        });
        assert!(validate_curve_point(&point, "curve[0]").is_ok());
        point["temperature_c"] = json!(25.0);
        let first = validate_curve_point(&point, "curve[0]").unwrap();
        let second = validate_curve_point(&point, "curve[1]").unwrap();
        assert!(second <= first);
    }

    #[test]
    fn rejects_measured_material_property_without_method() {
        let property = json!({
            "property_kind": "saturation_flux_density",
            "value": 0.53,
            "unit": "T",
            "frequency_hz": null,
            "temperature_c": 25.0,
            "flux_density_t": null,
            "test_method": null,
            "evidence_kind": "MEASURED",
            "source_locator": "table 1",
            "uncertainty": null
        });
        assert!(validate_property(&property, "property").is_err());
    }

    #[test]
    fn rejects_commercial_grade_without_manufacturer_and_datasheet() {
        let material = json!({
            "schema_version": "wpt-material-v1",
            "material_id": "WPT-MAT-GRADE-FAKE-01",
            "catalogue_type": "COMMERCIAL_GRADE",
            "material_family": "Mn-Zn ferrite",
            "grade": "FAKE-01",
            "manufacturer": null,
            "composition_description": null,
            "operating_frequency_hz_min": null,
            "operating_frequency_hz_max": null,
            "operating_temperature_c_min": null,
            "operating_temperature_c_max": null,
            "properties": [],
            "saturation_curve": [],
            "source_refs": [{"source_id":"source","role":"MANUFACTURER_DATASHEET","locator":"datasheet"}],
            "data_status": "UNRESOLVED",
            "notes": "synthetic negative control"
        });
        let sources = BTreeSet::from(["source".to_owned()]);
        assert!(validate_material(&material, 0, &sources).is_err());
    }
}
