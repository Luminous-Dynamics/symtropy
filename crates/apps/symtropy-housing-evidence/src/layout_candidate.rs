// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Validation of simple home-layout candidates bound to an exact resident brief.
//! Contract validity is not a BIM, architectural, structural, code, or construction approval.

use jsonschema::draft202012;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

const EMBEDDED_SCHEMA: &str =
    include_str!("../../../../docs/built-environment/home-layout-candidate.schema.json");
const GEOMETRY_TOLERANCE_M: f64 = 1.0e-9;

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

    fn overlaps(self, other: Self) -> bool {
        let width = (self.x_max.min(other.x_max) - self.x_min.max(other.x_min)).max(0.0);
        let height = (self.y_max.min(other.y_max) - self.y_min.max(other.y_min)).max(0.0);
        width > GEOMETRY_TOLERANCE_M && height > GEOMETRY_TOLERANCE_M
    }
}

/// Measured rectangle dimensions compared with an optional resident target area.
/// Rectangle area is not net usable floor area or a code-compliance measurement.
#[derive(Debug, Serialize)]
pub struct ZoneAreaMetric {
    pub zone_id: String,
    pub space_id: String,
    pub instance_index: u64,
    pub floor_id: String,
    pub width_m: f64,
    pub depth_m: f64,
    pub rectangle_area_m2: f64,
    pub target_area_m2: Option<f64>,
    /// Candidate rectangle area minus target area; informational, never a gate.
    pub delta_from_target_m2: Option<f64>,
}

/// Per-zone comparison against a provenance-bearing project dimension constraint.
#[derive(Debug, Serialize)]
pub struct DimensionConstraintDiagnostic {
    pub constraint_id: String,
    pub space_id: String,
    pub zone_id: Option<String>,
    pub instance_index: Option<u64>,
    pub axis: String,
    pub bound: String,
    pub required_extent_m: f64,
    pub measured_extent_m: Option<f64>,
    pub origin: String,
    pub enforcement: String,
    pub source_reference: Option<String>,
    pub source_edition: Option<String>,
    pub source_location: Option<String>,
    pub status: String,
    pub explanation: String,
}

#[derive(Clone, Debug)]
struct ZonePlacement {
    zone_id: String,
    space_id: String,
    instance_index: u64,
    floor_id: String,
    bounds: Bounds,
}

#[derive(Debug, Serialize)]
pub struct SharedBoundaryPair {
    pub source_zone_id: String,
    pub target_zone_id: String,
    pub floor_id: String,
    pub shared_boundary_length_m: f64,
}

/// Resident adjacency preference diagnostic. These are geometric proxies, not door/circulation proofs.
#[derive(Debug, Serialize)]
pub struct AdjacencyPreferenceDiagnostic {
    pub source_space_id: String,
    pub target_space_id: String,
    pub relationship: String,
    pub source_zone_ids: Vec<String>,
    pub target_zone_ids: Vec<String>,
    pub minimum_rectangle_gap_m: Option<f64>,
    pub shared_boundary_pairs: Vec<SharedBoundaryPair>,
    pub status: String,
    pub explanation: String,
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
    pub zone_area_metrics: Vec<ZoneAreaMetric>,
    pub dimension_constraint_diagnostics: Vec<DimensionConstraintDiagnostic>,
    pub adjacency_preference_diagnostics: Vec<AdjacencyPreferenceDiagnostic>,
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

    if brief_ref
        .and_then(|value| value.get("brief_id"))
        .and_then(Value::as_str)
        != brief.get("brief_id").and_then(Value::as_str)
    {
        reference_issues.push("brief_ref.brief_id does not match the supplied brief".to_string());
    }
    if brief_ref
        .and_then(|value| value.get("revision"))
        .and_then(Value::as_u64)
        != brief.get("revision").and_then(Value::as_u64)
    {
        reference_issues.push("brief_ref.revision does not match the supplied brief".to_string());
    }
    if brief_ref
        .and_then(|value| value.get("sha256"))
        .and_then(Value::as_str)
        != Some(brief_sha256.as_str())
    {
        reference_issues
            .push("brief_ref.sha256 does not match exact supplied brief bytes".to_string());
    }
    let reference_valid = reference_issues.is_empty() && brief_valid;

    let mut semantic_issues = Vec::new();
    let mut geometry_issues = Vec::new();
    let mut zone_area_metrics = Vec::new();
    let mut dimension_constraint_diagnostics = Vec::new();
    let mut adjacency_preference_diagnostics = Vec::new();
    if schema_valid {
        check_semantics(
            layout,
            brief,
            &mut semantic_issues,
            &mut geometry_issues,
            &mut zone_area_metrics,
            &mut dimension_constraint_diagnostics,
            &mut adjacency_preference_diagnostics,
        );
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
        zone_area_metrics,
        dimension_constraint_diagnostics,
        adjacency_preference_diagnostics,
        limitations: vec![
            "Connected-preference results only identify shared rectangle boundaries; they do not prove a doorway, opening, or usable circulation.".to_string(),
            "Near, separate, quiet, visual-connection, and outdoor-access preferences are not assessed by rectangle geometry.".to_string(),
            "Zone rectangle areas are geometric bounding-rectangle areas, not net usable floor areas.".to_string(),
            "Area differences from resident targets are informational; targets are not code minima and do not invalidate a candidate.".to_string(),
            "Dimension constraints compare axis-aligned rectangle X/Y extents, not wall-face clear dimensions or usable room dimensions.".to_string(),
            "Dimension-constraint origin and source fields are declared by the brief author; sources are not authenticated and authority applicability is not determined.".to_string(),
            "Only constraints explicitly marked must_satisfy affect contract validity; advisory mismatches are diagnostics only.".to_string(),
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
    area_metrics: &mut Vec<ZoneAreaMetric>,
    dimension_diagnostics: &mut Vec<DimensionConstraintDiagnostic>,
    adjacency_diagnostics: &mut Vec<AdjacencyPreferenceDiagnostic>,
) {
    let mut floors = BTreeMap::<String, Bounds>::new();
    let mut floor_ids = BTreeSet::new();
    let mut level_indices = BTreeSet::new();
    let mut elevations = BTreeSet::new();

    for (index, floor) in layout["floors"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        let id = floor["floor_id"].as_str().unwrap_or_default();
        if !floor_ids.insert(id.to_owned()) {
            semantic.push(format!("floors[{index}].floor_id duplicates {id:?}"));
        }
        if let Some(level) = floor["level_index"].as_u64()
            && !level_indices.insert(level)
        {
            semantic.push(format!("floors[{index}].level_index duplicates {level}"));
        }
        if let Some(elevation) = floor["elevation_m"].as_f64()
            && !elevations.insert(elevation.to_bits())
        {
            semantic.push(format!(
                "floors[{index}].elevation_m duplicates another level"
            ));
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
    let mut target_areas = BTreeMap::<String, Option<f64>>::new();
    for space in brief["spaces"].as_array().into_iter().flatten() {
        if let (Some(id), Some(priority), Some(quantity)) = (
            space["space_id"].as_str(),
            space["priority"].as_str(),
            space["quantity"].as_u64(),
        ) {
            space_specs.insert(id.to_owned(), (priority.to_owned(), quantity));
            target_areas.insert(
                id.to_owned(),
                space.get("target_area_m2").and_then(Value::as_f64),
            );
        }
    }

    let mut zone_ids = BTreeSet::new();
    let mut zone_instances = BTreeSet::new();
    let mut zones_per_space = BTreeMap::<String, Vec<u64>>::new();
    let mut positioned = Vec::<ZonePlacement>::new();

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
        let width_m = bounds.x_max - bounds.x_min;
        let depth_m = bounds.y_max - bounds.y_min;
        if width_m.is_finite()
            && depth_m.is_finite()
            && width_m > GEOMETRY_TOLERANCE_M
            && depth_m > GEOMETRY_TOLERANCE_M
        {
            let rectangle_area_m2 = width_m * depth_m;
            if rectangle_area_m2.is_finite() {
                let target_area_m2 = target_areas.get(space_id).copied().flatten();
                area_metrics.push(ZoneAreaMetric {
                    zone_id: id.to_owned(),
                    space_id: space_id.to_owned(),
                    instance_index: instance,
                    floor_id: floor_id.to_owned(),
                    width_m,
                    depth_m,
                    rectangle_area_m2,
                    target_area_m2,
                    delta_from_target_m2: target_area_m2
                        .map(|target| rectangle_area_m2 - target),
                });
            }
        }
        positioned.push(ZonePlacement {
            zone_id: id.to_owned(),
            space_id: space_id.to_owned(),
            instance_index: instance,
            floor_id: floor_id.to_owned(),
            bounds,
        });
    }

    let mut omitted = BTreeSet::new();
    for (index, item) in layout["omissions"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
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
            let zone_a = &positioned[left];
            let zone_b = &positioned[right];
            if zone_a.floor_id == zone_b.floor_id
                && zone_a.bounds.overlaps(zone_b.bounds)
            {
                geometry.push(format!(
                    "zones {:?} and {:?} overlap on floor {:?}",
                    zone_a.zone_id, zone_b.zone_id, zone_a.floor_id
                ));
            }
        }
    }

    check_dimension_constraints(brief, &positioned, semantic, dimension_diagnostics);
    check_adjacency_preferences(brief, &positioned, adjacency_diagnostics);
}



fn check_dimension_constraints(
    brief: &Value,
    positions: &[ZonePlacement],
    semantic_issues: &mut Vec<String>,
    diagnostics: &mut Vec<DimensionConstraintDiagnostic>,
) {
    for space in brief["spaces"].as_array().into_iter().flatten() {
        let Some(space_id) = space["space_id"].as_str() else {
            continue;
        };
        for constraint in space["dimension_constraints"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let (
                Some(constraint_id),
                Some(axis),
                Some(bound),
                Some(required_extent_m),
                Some(origin),
                Some(enforcement),
            ) = (
                constraint["constraint_id"].as_str(),
                constraint["axis"].as_str(),
                constraint["bound"].as_str(),
                constraint["value_m"].as_f64(),
                constraint["origin"].as_str(),
                constraint["enforcement"].as_str(),
            )
            else {
                continue;
            };
            if !matches!(axis, "x_extent" | "y_extent")
                || !matches!(bound, "minimum" | "maximum")
            {
                continue;
            }
            let source_reference = constraint["source_reference"]
                .as_str()
                .map(str::to_owned);
            let source_edition = constraint["source_edition"]
                .as_str()
                .map(str::to_owned);
            let source_location = constraint["source_location"]
                .as_str()
                .map(str::to_owned);
            let matching: Vec<&ZonePlacement> = positions
                .iter()
                .filter(|position| position.space_id == space_id)
                .collect();

            if matching.is_empty() {
                let status = "SPACE_INSTANCE_MISSING";
                let explanation =
                    "No zone instance exists for this space, so the declared dimension bound cannot be compared.";
                if enforcement == "must_satisfy" {
                    semantic_issues.push(format!(
                        "must-satisfy dimension constraint {constraint_id:?} for space {space_id:?} cannot be evaluated because no zone instance exists"
                    ));
                }
                diagnostics.push(DimensionConstraintDiagnostic {
                    constraint_id: constraint_id.to_owned(),
                    space_id: space_id.to_owned(),
                    zone_id: None,
                    instance_index: None,
                    axis: axis.to_owned(),
                    bound: bound.to_owned(),
                    required_extent_m,
                    measured_extent_m: None,
                    origin: origin.to_owned(),
                    enforcement: enforcement.to_owned(),
                    source_reference: source_reference.clone(),
                    source_edition: source_edition.clone(),
                    source_location: source_location.clone(),
                    status: status.to_owned(),
                    explanation: explanation.to_owned(),
                });
                continue;
            }

            for position in matching {
                let measured_extent_m = match axis {
                    "x_extent" => position.bounds.x_max - position.bounds.x_min,
                    "y_extent" => position.bounds.y_max - position.bounds.y_min,
                    _ => continue,
                };
                let status = if !measured_extent_m.is_finite()
                    || measured_extent_m <= GEOMETRY_TOLERANCE_M
                {
                    "INVALID_GEOMETRY"
                } else if bound == "minimum"
                    && measured_extent_m + GEOMETRY_TOLERANCE_M < required_extent_m
                {
                    "BELOW_MINIMUM"
                } else if bound == "maximum"
                    && measured_extent_m - GEOMETRY_TOLERANCE_M > required_extent_m
                {
                    "ABOVE_MAXIMUM"
                } else {
                    "WITHIN_LIMIT"
                };
                let explanation = match status {
                    "WITHIN_LIMIT" => "The rectangle extent meets this declared bound within the numeric comparison tolerance; this is not a code or usability finding.",
                    "BELOW_MINIMUM" => "The measured rectangle extent is below the declared minimum; check room orientation, usable dimensions, and constraint applicability.",
                    "ABOVE_MAXIMUM" => "The measured rectangle extent is above the declared maximum; confirm that this is intentional.",
                    "INVALID_GEOMETRY" => "This rectangle extent is not positive and finite, so a meaningful dimension comparison is unavailable.",
                    _ => "Dimension comparison was not assessed.",
                };
                if status != "WITHIN_LIMIT" && enforcement == "must_satisfy" {
                    semantic_issues.push(format!(
                        "must-satisfy dimension constraint {constraint_id:?} for space {space_id:?}, zone {:?}, is {status}: required {required_extent_m} m on {axis}, measured {measured_extent_m} m",
                        position.zone_id
                    ));
                }
                diagnostics.push(DimensionConstraintDiagnostic {
                    constraint_id: constraint_id.to_owned(),
                    space_id: space_id.to_owned(),
                    zone_id: Some(position.zone_id.clone()),
                    instance_index: Some(position.instance_index),
                    axis: axis.to_owned(),
                    bound: bound.to_owned(),
                    required_extent_m,
                    measured_extent_m: Some(measured_extent_m),
                    origin: origin.to_owned(),
                    enforcement: enforcement.to_owned(),
                    source_reference: source_reference.clone(),
                    source_edition: source_edition.clone(),
                    source_location: source_location.clone(),
                    status: status.to_owned(),
                    explanation: explanation.to_owned(),
                });
            }
        }
    }
}

fn check_adjacency_preferences(
    brief: &Value,
    positions: &[ZonePlacement],
    diagnostics: &mut Vec<AdjacencyPreferenceDiagnostic>,
) {
    for space in brief["spaces"].as_array().into_iter().flatten() {
        let Some(source_space_id) = space["space_id"].as_str() else {
            continue;
        };
        for preference in space["adjacency_preferences"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let Some(target_space_id) = preference["other_space_id"].as_str() else {
                continue;
            };
            let relationship = preference["relationship"].as_str().unwrap_or("unknown");
            let source_zone_ids: Vec<String> = positions
                .iter()
                .filter(|zone| zone.space_id == source_space_id)
                .map(|zone| zone.zone_id.clone())
                .collect();
            let target_zone_ids: Vec<String> = positions
                .iter()
                .filter(|zone| zone.space_id == target_space_id)
                .map(|zone| zone.zone_id.clone())
                .collect();

            let mut minimum_rectangle_gap_m: Option<f64> = None;
            let mut shared_boundary_pairs = Vec::new();
            for source in positions.iter().filter(|zone| zone.space_id == source_space_id) {
                for target in positions.iter().filter(|zone| {
                    zone.space_id == target_space_id && zone.floor_id == source.floor_id
                }) {
                    let dx = (source.bounds.x_min - target.bounds.x_max)
                        .max(target.bounds.x_min - source.bounds.x_max)
                        .max(0.0);
                    let dy = (source.bounds.y_min - target.bounds.y_max)
                        .max(target.bounds.y_min - source.bounds.y_max)
                        .max(0.0);
                    let gap = dx.hypot(dy);
                    if gap.is_finite() {
                        minimum_rectangle_gap_m = Some(
                            minimum_rectangle_gap_m.map_or(gap, |previous| previous.min(gap)),
                        );
                    }
                    let vertical_contact = if
                        (source.bounds.x_max - target.bounds.x_min).abs()
                            <= GEOMETRY_TOLERANCE_M
                        || (target.bounds.x_max - source.bounds.x_min).abs()
                            <= GEOMETRY_TOLERANCE_M
                    {
                        (source.bounds.y_max.min(target.bounds.y_max)
                            - source.bounds.y_min.max(target.bounds.y_min))
                            .max(0.0)
                    } else {
                        0.0
                    };
                    let horizontal_contact = if
                        (source.bounds.y_max - target.bounds.y_min).abs()
                            <= GEOMETRY_TOLERANCE_M
                        || (target.bounds.y_max - source.bounds.y_min).abs()
                            <= GEOMETRY_TOLERANCE_M
                    {
                        (source.bounds.x_max.min(target.bounds.x_max)
                            - source.bounds.x_min.max(target.bounds.x_min))
                            .max(0.0)
                    } else {
                        0.0
                    };
                    let shared_length = vertical_contact.max(horizontal_contact);
                    if shared_length > GEOMETRY_TOLERANCE_M {
                        shared_boundary_pairs.push(SharedBoundaryPair {
                            source_zone_id: source.zone_id.clone(),
                            target_zone_id: target.zone_id.clone(),
                            floor_id: source.floor_id.clone(),
                            shared_boundary_length_m: shared_length,
                        });
                    }
                }
            }

            let (status, explanation) = if source_zone_ids.is_empty() || target_zone_ids.is_empty() {
                (
                    "SPACE_INSTANCE_MISSING",
                    "At least one requested space has no zone in this candidate; geometric relation cannot be evaluated.",
                )
            } else if relationship == "connected" && !shared_boundary_pairs.is_empty() {
                (
                    "SHARED_BOUNDARY_PROXY_DETECTED",
                    "Zone rectangles share a boundary, but this does not prove a doorway, opening, accessible route, or circulation clearance.",
                )
            } else if relationship == "connected" {
                (
                    "NO_SHARED_BOUNDARY_PROXY_DETECTED",
                    "No positive-length shared rectangle boundary was detected on a common floor; this does not prove that the intended connection is impossible.",
                )
            } else {
                (
                    "NOT_ASSESSED",
                    "This relationship requires a richer model or an explicit threshold/discipline-specific evaluator.",
                )
            };
            diagnostics.push(AdjacencyPreferenceDiagnostic {
                source_space_id: source_space_id.to_owned(),
                target_space_id: target_space_id.to_owned(),
                relationship: relationship.to_owned(),
                source_zone_ids,
                target_zone_ids,
                minimum_rectangle_gap_m,
                shared_boundary_pairs,
                status: status.to_owned(),
                explanation: explanation.to_owned(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixtures() -> (Value, Value, Vec<u8>) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../docs/built-environment");
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
        assert_eq!(report.zone_area_metrics.len(), 10);
        let living = report
            .zone_area_metrics
            .iter()
            .find(|metric| metric.zone_id == "living-1")
            .expect("living rectangle metric exists");
        assert_eq!(living.width_m, 6.0);
        assert_eq!(living.depth_m, 4.0);
        assert_eq!(living.rectangle_area_m2, 24.0);
        assert_eq!(living.target_area_m2, Some(28.0));
        assert_eq!(living.delta_from_target_m2, Some(-4.0));
        assert_eq!(report.dimension_constraint_diagnostics.len(), 2);
        let x_extent = report
            .dimension_constraint_diagnostics
            .iter()
            .find(|item| item.constraint_id == "living-x-min-resident")
            .expect("resident x-extent constraint is reported");
        assert_eq!(x_extent.status, "WITHIN_LIMIT");
        assert_eq!(x_extent.measured_extent_m, Some(6.0));
        assert_eq!(x_extent.origin, "resident_preference");
        assert_eq!(x_extent.enforcement, "advisory");
        let connected = report
            .adjacency_preference_diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.source_space_id == "living"
                    && diagnostic.target_space_id == "kitchen"
                    && diagnostic.relationship == "connected"
            })
            .expect("living-to-kitchen preference exists");
        assert_eq!(connected.status, "SHARED_BOUNDARY_PROXY_DETECTED");
        assert_eq!(connected.shared_boundary_pairs.len(), 1);
        assert_eq!(connected.shared_boundary_pairs[0].shared_boundary_length_m, 3.0);
        assert!(connected.explanation.contains("does not prove a doorway"));

        let visual = report
            .adjacency_preference_diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.source_space_id == "living"
                    && diagnostic.target_space_id == "courtyard"
                    && diagnostic.relationship == "visual_connection_to"
            })
            .expect("visual-connection preference exists");
        assert_eq!(visual.status, "NOT_ASSESSED");
    }

    #[test]
    fn advisory_dimension_mismatch_is_reported_without_invalidating_candidate() {
        let (mut layout, mut brief, _) = fixtures();
        brief["spaces"][0]["dimension_constraints"][0]["value_m"] = json!(6.5);
        let bytes = serde_json::to_vec(&brief).expect("mutated brief serializes");
        layout["brief_ref"]["sha256"] = json!(sha256_hex(&bytes));

        let report = validate_values(&layout, &brief, &bytes).expect("schema compiles");
        assert_eq!(report.contract_status, "LAYOUT_CANDIDATE_CONTRACT_VALID");
        let diagnostic = report
            .dimension_constraint_diagnostics
            .iter()
            .find(|item| item.constraint_id == "living-x-min-resident")
            .expect("dimension mismatch is reported");
        assert_eq!(diagnostic.status, "BELOW_MINIMUM");
        assert_eq!(diagnostic.measured_extent_m, Some(6.0));
    }

    #[test]
    fn must_satisfy_dimension_mismatch_invalidates_candidate() {
        let (mut layout, mut brief, _) = fixtures();
        brief["spaces"][0]["dimension_constraints"][0]["value_m"] = json!(6.5);
        brief["spaces"][0]["dimension_constraints"][0]["enforcement"] =
            json!("must_satisfy");
        let bytes = serde_json::to_vec(&brief).expect("mutated brief serializes");
        layout["brief_ref"]["sha256"] = json!(sha256_hex(&bytes));

        let report = validate_values(&layout, &brief, &bytes).expect("schema compiles");
        assert_eq!(report.contract_status, "LAYOUT_CANDIDATE_CONTRACT_INVALID");
        assert!(!report.semantic_valid);
        assert!(
            report
                .semantic_issues
                .iter()
                .any(|issue| issue.contains("must-satisfy dimension constraint"))
        );
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
    fn tiny_rectangle_intersection_is_not_hidden_by_area_threshold() {
        let (mut layout, brief, bytes) = fixtures();
        let bounds = &mut layout["zones"][1]["bounds_m"];
        bounds["x_min"] = json!(5.99995);
        bounds["y_min"] = json!(3.99995);
        bounds["x_max"] = json!(6.00005);
        bounds["y_max"] = json!(4.00005);

        let report = validate_values(&layout, &brief, &bytes).expect("schema compiles");
        assert!(!report.geometry_valid);
        assert!(
            report
                .geometry_issues
                .iter()
                .any(|issue| issue.contains("overlap"))
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
