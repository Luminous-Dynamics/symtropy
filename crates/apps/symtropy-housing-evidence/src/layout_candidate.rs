// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Validation of simple home-layout candidates bound to an exact resident brief.
//! Contract validity is not a BIM, architectural, structural, code, or construction approval.

use jsonschema::draft202012;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, fs, path::Path};

const EMBEDDED_SCHEMA: &str =
    include_str!("../../../../docs/built-environment/home-layout-candidate.schema.json");
const GEOMETRY_TOLERANCE_M: f64 = 1.0e-9;
const OVERLAP_AREA_TOLERANCE_M2: f64 = 1.0e-6;

#[derive(Clone, Copy, Debug)]
struct Bounds {
    x_min: f64,
    y_min: f64,
    x_max: f64,
    y_max: f64,
}

impl Bounds {
    fn from_value(value: &Value) -> Option<Self> {
        Some(Self {
            x_min: value.get("x_min")?.as_f64()?,
            y_min: value.get("y_min")?.as_f64()?,
            x_max: value.get("x_max")?.as_f64()?,
            y_max: value.get("y_max")?.as_f64()?,
        })
    }

    fn has_positive_area(self) -> bool {
        self.x_max - self.x_min > GEOMETRY_TOLERANCE_M
            && self.y_max - self.y_min > GEOMETRY_TOLERANCE_M
    }

    fn contains(self, other: Self) -> bool {
        other.x_min >= self.x_min - GEOMETRY_TOLERANCE_M
            && other.y_min >= self.y_min - GEOMETRY_TOLERANCE_M
            && other.x_max <= self.x_max + GEOMETRY_TOLERANCE_M
            && other.y_max <= self.y_max + GEOMETRY_TOLERANCE_M
    }

    fn overlap_area(self, other: Self) -> f64 {
        let width = (self.x_max.min(other.x_max) - self.x_min.max(other.x_min)).max(0.0);
        let height = (self.y_max.min(other.y_max) - self.y_min.max(other.y_min)).max(0.0);
        width * height
    }
}

#[derive(Debug, Serialize)]
pub struct LayoutValidationReport {
    pub validator_version: String,
    pub schema_sha256: String,
    pub brief_sha256: String,
    pub candidate_id: Option<String>,
    pub schema_valid: bool,
    pub brief_valid: bool,
    pub reference_valid: bool,
    pub semantic_valid: bool,
    pub geometry_valid: bool,
    pub contract_status: String,
    pub schema_issues: Vec<String>,
    pub reference_issues: Vec<String>,
    pub semantic_issues: Vec<String>,
    pub geometry_issues: Vec<String>,
    pub limitations: Vec<String>,
}

pub fn validate_paths(
    layout_path: &Path,
    brief_path: &Path,
) -> Result<LayoutValidationReport, String> {
    let layout_bytes = fs::read(layout_path)
        .map_err(|error| format!("cannot read {}: {error}", layout_path.display()))?;
    let brief_bytes = fs::read(brief_path)
        .map_err(|error| format!("cannot read {}: {error}", brief_path.display()))?;
    let layout: Value = serde_json::from_slice(&layout_bytes)
        .map_err(|error| format!("{} is not valid JSON: {error}", layout_path.display()))?;
    let brief: Value = serde_json::from_slice(&brief_bytes)
        .map_err(|error| format!("{} is not valid JSON: {error}", brief_path.display()))?;
    validate_values(&layout, &brief, &brief_bytes)
}

pub fn validate_values(
    layout: &Value,
    brief: &Value,
    brief_bytes: &[u8],
) -> Result<LayoutValidationReport, String> {
    let schema: Value = serde_json::from_str(EMBEDDED_SCHEMA)
        .map_err(|error| format!("embedded layout schema is invalid JSON: {error}"))?;
    if !all_refs_are_local(&schema) {
        return Err("layout schema contains an external reference".to_string());
    }
    draft202012::meta::validate(&schema)
        .map_err(|error| format!("layout schema fails Draft 2020-12 meta-validation: {error}"))?;

    let validator = draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| format!("cannot compile layout schema: {error}"))?;
    let schema_issues: Vec<String> = validator
        .iter_errors(layout)
        .map(|error| format!("schema validation: {error}"))
        .collect();
    let schema_valid = schema_issues.is_empty();

    let brief_report = super::home_design_brief::validate_value(brief)?;
    let brief_valid = brief_report.schema_valid && brief_report.semantic_valid;
    let brief_sha256 = sha256_hex(brief_bytes);
    let mut reference_issues = Vec::new();
    let brief_ref = layout.get("brief_ref");

    if brief_ref.and_then(|value| value.get("brief_id")).and_then(Value::as_str)
        != brief.get("brief_id").and_then(Value::as_str)
    {
        reference_issues.push("brief_ref.brief_id does not match the supplied brief".to_string());
    }
    if brief_ref.and_then(|value| value.get("revision")).and_then(Value::as_u64)
        != brief.get("revision").and_then(Value::as_u64)
    {
        reference_issues.push("brief_ref.revision does not match the supplied brief".to_string());
    }
    if brief_ref.and_then(|value| value.get("sha256")).and_then(Value::as_str)
        != Some(brief_sha256.as_str())
    {
        reference_issues
            .push("brief_ref.sha256 does not match exact supplied brief bytes".to_string());
    }
    let reference_valid = reference_issues.is_empty() && brief_valid;

    let mut semantic_issues = Vec::new();
    let mut geometry_issues = Vec::new();
    if schema_valid {
        check_semantics(layout, brief, &mut semantic_issues, &mut geometry_issues);
    }

    let semantic_valid = semantic_issues.is_empty();
    let geometry_valid = geometry_issues.is_empty();
    let contract_valid =
        schema_valid && brief_valid && reference_valid && semantic_valid && geometry_valid;
    let contract_status = if contract_valid {
        "LAYOUT_CANDIDATE_CONTRACT_VALID"
    } else {
        "LAYOUT_CANDIDATE_CONTRACT_INVALID"
    };

    Ok(LayoutValidationReport {
        validator_version: env!("CARGO_PKG_VERSION").to_string(),
        schema_sha256: sha256_hex(EMBEDDED_SCHEMA.as_bytes()),
        brief_sha256,
        candidate_id: layout
            .get("candidate_id")
            .and_then(Value::as_str)
            .map(str::to_owned),
        schema_valid,
        brief_valid,
        reference_valid,
        semantic_valid,
        geometry_valid,
        contract_status: contract_status.to_string(),
        schema_issues,
        reference_issues,
        semantic_issues,
        geometry_issues,
        limitations: vec![
            "Valid means only contract shape, exact brief linkage, space accounting, and simple rectangle geometry passed.".to_string(),
            "No openings, circulation, visibility, acoustics, daylight, thermal behavior, structure, accessibility compliance, cost, planning rules, permits, or code compliance are evaluated.".to_string(),
            "All same-floor zones must be disjoint; shared/multi-use zones require a future explicit relationship contract.".to_string(),
            "Never describe this result as a buildable, safe, approved, or construction-ready design.".to_string(),
        ],
    })
}

fn all_refs_are_local(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.iter().all(|(key, child)| {
            (key != "$ref"
                || child
                    .as_str()
                    .is_some_and(|reference| reference.starts_with("#/")))
                && all_refs_are_local(child)
        }),
        Value::Array(values) => values.iter().all(all_refs_are_local),
        _ => true,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn check_semantics(
    layout: &Value,
    brief: &Value,
    semantic: &mut Vec<String>,
    geometry: &mut Vec<String>,
) {
    let mut floors = BTreeMap::<String, Bounds>::new();
    let mut floor_ids = BTreeSet::new();
    let mut level_indices = BTreeSet::new();
    let mut elevations = BTreeSet::new();

    for (index, floor) in layout["floors"].as_array().into_iter().flatten().enumerate() {
        let id = floor["floor_id"].as_str().unwrap_or_default();
        if !floor_ids.insert(id.to_owned()) {
            semantic.push(format!("floors[{index}].floor_id duplicates {id:?}"));
        }
        if let Some(level) = floor["level_index"].as_u64() {
            if !level_indices.insert(level) {
                semantic.push(format!(
                    "floors[{index}].level_index duplicates {level}"
                ));
            }
        }
        if let Some(elevation) = floor["elevation_m"].as_f64() {
            if !elevations.insert(elevation.to_bits()) {
                semantic.push(format!(
                    "floors[{index}].elevation_m duplicates another level"
                ));
            }
        }
        if let Some(bounds) = Bounds::from_value(&floor["planning_bounds_m"]) {
            if !bounds.has_positive_area() {
                geometry.push(format!(
                    "floors[{index}] planning bounds have no positive area"
                ));
            }
            floors.insert(id.to_owned(), bounds);
        }
    }

    let mut space_specs = BTreeMap::<String, (String, u64)>::new();
    for space in brief["spaces"].as_array().into_iter().flatten() {
        if let (Some(id), Some(priority), Some(quantity)) = (
            space["space_id"].as_str(),
            space["priority"].as_str(),
            space["quantity"].as_u64(),
        ) {
            space_specs.insert(id.to_owned(), (priority.to_owned(), quantity));
        }
    }

    let mut zone_ids = BTreeSet::new();
    let mut zone_instances = BTreeSet::new();
    let mut zones_per_space = BTreeMap::<String, Vec<u64>>::new();
    let mut positioned = Vec::<(String, String, Bounds)>::new();

    for (index, zone) in layout["zones"].as_array().into_iter().flatten().enumerate() {
        let id = zone["zone_id"].as_str().unwrap_or_default();
        if !zone_ids.insert(id.to_owned()) {
            semantic.push(format!("zones[{index}].zone_id duplicates {id:?}"));
        }
        let space_id = zone["space_id"].as_str().unwrap_or_default();
        let instance = zone["instance_index"].as_u64().unwrap_or_default();
        let floor_id = zone["floor_id"].as_str().unwrap_or_default();

        if let Some((_, quantity)) = space_specs.get(space_id) {
            if instance == 0 || instance > *quantity {
                semantic.push(format!(
                    "zones[{index}] instance {instance} is outside quantity 1..={quantity} for {space_id:?}"
                ));
            }
            if !zone_instances.insert((space_id.to_owned(), instance)) {
                semantic.push(format!(
                    "zones[{index}] duplicates instance {instance} of {space_id:?}"
                ));
            }
            zones_per_space
                .entry(space_id.to_owned())
                .or_default()
                .push(instance);
        } else {
            semantic.push(format!(
                "zones[{index}].space_id references unknown brief space {space_id:?}"
            ));
        }

        let Some(floor_bounds) = floors.get(floor_id).copied() else {
            semantic.push(format!(
                "zones[{index}].floor_id references unknown floor {floor_id:?}"
            ));
            continue;
        };
        let Some(bounds) = Bounds::from_value(&zone["bounds_m"]) else {
            continue;
        };
        if !bounds.has_positive_area() {
            geometry.push(format!(
                "zones[{index}] bounds must have positive width/depth"
            ));
        }
        if !floor_bounds.contains(bounds) {
            geometry.push(format!(
                "zones[{index}] lies outside planning bounds for {floor_id:?}"
            ));
        }
        positioned.push((id.to_owned(), floor_id.to_owned(), bounds));
    }

    let mut omitted = BTreeSet::new();
    for (index, item) in layout["omissions"].as_array().into_iter().flatten().enumerate() {
        let id = item["space_id"].as_str().unwrap_or_default();
        if !space_specs.contains_key(id) {
            semantic.push(format!(
                "omissions[{index}] refers to unknown brief space {id:?}"
            ));
        }
        if !omitted.insert(id.to_owned()) {
            semantic.push(format!("omissions[{index}] duplicates {id:?}"));
        }
        if zones_per_space.contains_key(id) {
            semantic.push(format!("space {id:?} is both included and omitted"));
        }
        if space_specs
            .get(id)
            .is_some_and(|(priority, _)| priority == "must_have")
        {
            semantic.push(format!(
                "must-have space {id:?} cannot be omitted; preserve and report infeasibility"
            ));
        }
    }

    for (id, (priority, quantity)) in &space_specs {
        let count = zones_per_space.get(id).map_or(0, Vec::len) as u64;
        if count > 0 && count != *quantity {
            semantic.push(format!(
                "space {id:?} has {count} zones but requested quantity is {quantity}; partial quantity is unsupported in v1"
            ));
        }
        if priority == "must_have" && count != *quantity {
            semantic.push(format!(
                "must-have space {id:?} requires {quantity} zone instance(s), found {count}"
            ));
        }
        if count == 0 && !omitted.contains(id) {
            semantic.push(format!(
                "space {id:?} has no zone and no explicit omission reason"
            ));
        }
    }

    for left in 0..positioned.len() {
        for right in (left + 1)..positioned.len() {
            let (id_a, floor_a, bounds_a) = &positioned[left];
            let (id_b, floor_b, bounds_b) = &positioned[right];
            if floor_a == floor_b
                && bounds_a.overlap_area(*bounds_b) > OVERLAP_AREA_TOLERANCE_M2
            {
                geometry.push(format!(
                    "zones {id_a:?} and {id_b:?} overlap on floor {floor_a:?}"
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixtures() -> (Value, Value, Vec<u8>) {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../docs/built-environment");
        let layout: Value = serde_json::from_slice(
            &fs::read(root.join("fixtures/example-home-layout-candidate.json"))
                .expect("layout fixture exists"),
        )
        .expect("layout parses");
        let bytes = fs::read(root.join("fixtures/example-home-design-brief.json"))
            .expect("brief fixture exists");
        let brief: Value = serde_json::from_slice(&bytes).expect("brief parses");
        (layout, brief, bytes)
    }

    #[test]
    fn synthetic_fixture_passes_candidate_contract_only() {
        let (layout, brief, bytes) = fixtures();
        let report = validate_values(&layout, &brief, &bytes).expect("schema compiles");
        assert!(report.schema_valid, "{:?}", report.schema_issues);
        assert!(report.brief_valid);
        assert!(report.reference_valid, "{:?}", report.reference_issues);
        assert!(report.semantic_valid, "{:?}", report.semantic_issues);
        assert!(report.geometry_valid, "{:?}", report.geometry_issues);
        assert_eq!(report.contract_status, "LAYOUT_CANDIDATE_CONTRACT_VALID");
    }

    #[test]
    fn changed_brief_digest_is_rejected() {
        let (mut layout, brief, bytes) = fixtures();
        layout["brief_ref"]["sha256"] =
            json!("0000000000000000000000000000000000000000000000000000000000000000");
        let report = validate_values(&layout, &brief, &bytes).expect("schema compiles");
        assert!(!report.reference_valid);
    }

    #[test]
    fn overlaps_and_out_of_bounds_fail_geometry() {
        let (mut layout, brief, bytes) = fixtures();
        layout["zones"][1]["bounds_m"]["x_min"] = json!(5.5);
        let report = validate_values(&layout, &brief, &bytes).expect("schema compiles");
        assert!(!report.geometry_valid);
        assert!(
            report
                .geometry_issues
                .iter()
                .any(|issue| issue.contains("overlap"))
        );

        let (mut layout, brief, bytes) = fixtures();
        layout["zones"][0]["bounds_m"]["x_max"] = json!(21.0);
        let report = validate_values(&layout, &brief, &bytes).expect("schema compiles");
        assert!(
            report
                .geometry_issues
                .iter()
                .any(|issue| issue.contains("outside planning bounds"))
        );
    }

    #[test]
    fn must_have_space_cannot_be_silently_removed() {
        let (mut layout, brief, bytes) = fixtures();
        layout["zones"]
            .as_array_mut()
            .unwrap()
            .retain(|zone| zone["space_id"].as_str() != Some("living"));
        let report = validate_values(&layout, &brief, &bytes).expect("schema compiles");
        assert!(!report.semantic_valid);
        assert!(
            report
                .semantic_issues
                .iter()
                .any(|issue| issue.contains("must-have space"))
        );
    }

    #[test]
    fn unknown_brief_space_is_rejected() {
        let (mut layout, brief, bytes) = fixtures();
        layout["zones"][0]["space_id"] = json!("unrequested-room");
        let report = validate_values(&layout, &brief, &bytes).expect("schema compiles");
        assert!(!report.semantic_valid);
        assert!(
            report
                .semantic_issues
                .iter()
                .any(|issue| issue.contains("unknown brief space"))
        );
    }
}
