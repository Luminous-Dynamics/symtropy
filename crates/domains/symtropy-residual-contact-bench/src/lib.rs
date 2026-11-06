//! Evidence-bounded spatial excavation-floor residue evaluation.
//!
//! This crate classifies measured residue thickness intervals inside declared
//! machine support footprints. It is geometric evidence only: it does not infer
//! contact mechanics, friction, traction, stability, ground competence, safety,
//! machine limits, or control actions.
#![forbid(unsafe_code)]

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const MAX_GRID_CELLS: usize = 1_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    pub code: String,
    pub path: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SupportReport {
    pub support_id: String,
    pub acceptance_profile_ref: String,
    pub threshold_mm: f64,
    pub footprint_cell_count: usize,
    pub clear_cells: usize,
    pub obstructed_cells: usize,
    pub ambiguous_cells: usize,
    pub unknown_cells: usize,
    pub clear_fraction: f64,
    pub obstructed_fraction: f64,
    pub ambiguous_fraction: f64,
    pub unknown_fraction: f64,
    pub maximum_thickness_upper_mm: Option<f64>,
    pub disposition: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationReport {
    pub case_id: String,
    pub evidence_class: String,
    pub map_id: String,
    pub frame_id: String,
    pub map_cell_count: usize,
    pub cell_area_mm2: f64,
    pub registration_uncertainty_mm: f64,
    pub residual_volume_lower_mm3: f64,
    pub residual_volume_upper_mm3: f64,
    pub coverage_disposition: String,
    pub support_reports: Vec<SupportReport>,
}

fn push(errors: &mut Vec<Violation>, code: &str, path: &str, detail: impl Into<String>) {
    errors.push(Violation {
        code: code.to_owned(),
        path: path.to_owned(),
        detail: detail.into(),
    });
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn number(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64).filter(|n| n.is_finite())
}

fn coordinate(value: &Value, key: &str) -> Option<u32> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
}

fn validate_cell_measurement(
    cell: &Value,
    path: &str,
    errors: &mut Vec<Violation>,
) {
    let lower = number(cell, "residual_thickness_lower_mm");
    let upper = number(cell, "residual_thickness_upper_mm");

    match (lower, upper) {
        (Some(lo), Some(hi)) if lo >= 0.0 && hi >= lo => {}
        (Some(lo), Some(hi)) if lo < 0.0 || hi < 0.0 => push(
            errors,
            "NEGATIVE_RESIDUAL_THICKNESS",
            path,
            "residual thickness interval must be non-negative",
        ),
        (Some(_), Some(_)) => push(
            errors,
            "REVERSED_RESIDUAL_INTERVAL",
            path,
            "upper residual thickness must be greater than or equal to lower thickness",
        ),
        _ => push(
            errors,
            "INVALID_RESIDUAL_INTERVAL",
            path,
            "both finite lower and upper residual-thickness bounds are required",
        ),
    }

    if text(cell, "evidence_role") != Some("INDEPENDENT_MEASUREMENT") {
        push(
            errors,
            "NON_INDEPENDENT_RESIDUAL_EVIDENCE",
            &format!("{path}.evidence_role"),
            "v0.1 cell measurements must be independently measured; model outputs are not measurement truth",
        );
    }

    for key in ["calibration_ref", "source_ref"] {
        if text(cell, key).is_none() {
            push(
                errors,
                "MISSING_CELL_PROVENANCE",
                &format!("{path}.{key}"),
                "a non-empty calibration and source reference is required",
            );
        }
    }
}

/// Validate the complete semantic contract. This is stricter than JSON parsing:
/// it enforces unique cell identity, complete spatial coverage, frame binding,
/// and support-profile provenance.
pub fn validate_case(case: &Value) -> Vec<Violation> {
    let mut errors = Vec::new();

    if case.get("schema_version").and_then(Value::as_u64) != Some(1) {
        push(
            &mut errors,
            "UNSUPPORTED_SCHEMA_VERSION",
            "$.schema_version",
            "schema_version must be exactly 1",
        );
    }

    if text(case, "case_id").is_none() {
        push(&mut errors, "MISSING_CASE_ID", "$.case_id", "case_id is required");
    }

    if !matches!(text(case, "evidence_class"), Some("SYNTHETIC" | "MEASURED")) {
        push(
            &mut errors,
            "INVALID_EVIDENCE_CLASS",
            "$.evidence_class",
            "evidence_class must be SYNTHETIC or MEASURED",
        );
    }

    let Some(map) = case.get("map") else {
        push(&mut errors, "MISSING_MAP", "$.map", "map is required");
        return errors;
    };

    let map_id = text(map, "map_id");
    let frame_id = text(map, "frame_id");
    for (key, value) in [("map_id", map_id), ("frame_id", frame_id)] {
        if value.is_none() {
            push(
                &mut errors,
                "MISSING_MAP_IDENTITY",
                &format!("$.map.{key}"),
                "non-empty map and frame identity are required",
            );
        }
    }

    for key in [
        "trial_id",
        "task_id",
        "site_id",
        "machine_instance_id",
        "machine_configuration_ref",
    ] {
        if text(map, key).is_none() {
            push(
                &mut errors,
                "MISSING_MAP_CONTEXT",
                &format!("$.map.{key}"),
                "trial, task, site, machine instance and exact configuration identity are required",
            );
        }
    }

    for key in [
        "registration_ref",
        "registration_calibration_ref",
    ] {
        if text(map, key).is_none() {
            push(
                &mut errors,
                "MISSING_REGISTRATION_PROVENANCE",
                &format!("$.map.{key}"),
                "registration evidence and its calibration reference are required",
            );
        }
    }

    let registration_uncertainty = number(map, "registration_uncertainty_mm");
    match registration_uncertainty {
        Some(v) if v >= 0.0 => {}
        _ => push(
            &mut errors,
            "INVALID_REGISTRATION_UNCERTAINTY",
            "$.map.registration_uncertainty_mm",
            "registration uncertainty must be finite and non-negative",
        ),
    }

    if map.get("observed_at_ns").and_then(Value::as_u64).is_none() {
        push(
            &mut errors,
            "INVALID_MAP_TIME",
            "$.map.observed_at_ns",
            "a non-negative integer observation timestamp is required",
        );
    }

    let cell_size = number(map, "cell_size_mm");
    match cell_size {
        Some(v) if v > 0.0 && (v * v).is_finite() && (v * v) > 0.0 => {}
        _ => push(
            &mut errors,
            "INVALID_CELL_SIZE",
            "$.map.cell_size_mm",
            "cell_size_mm must be finite, positive, and have a finite squared area",
        ),
    }

    let width = coordinate(map, "width_cells");
    let height = coordinate(map, "height_cells");
    let expected_count = match (width, height) {
        (Some(w), Some(h)) if w > 0 && h > 0 => {
            let count = (w as usize).checked_mul(h as usize);
            match count {
                Some(n) if n <= MAX_GRID_CELLS => Some(n),
                _ => {
                    push(
                        &mut errors,
                        "GRID_TOO_LARGE",
                        "$.map",
                        "grid dimensions exceed the v0.1 bounded evaluation limit",
                    );
                    None
                }
            }
        }
        _ => {
            push(
                &mut errors,
                "INVALID_GRID_DIMENSIONS",
                "$.map",
                "positive integer width_cells and height_cells are required",
            );
            None
        }
    };

    let Some(cells) = map.get("cells").and_then(Value::as_array) else {
        push(
            &mut errors,
            "MISSING_MAP_CELLS",
            "$.map.cells",
            "cells must be an array",
        );
        return errors;
    };

    let mut seen_cells = BTreeSet::new();
    for (index, cell) in cells.iter().enumerate() {
        let path = format!("$.map.cells[{index}]");
        let (Some(x), Some(y)) = (coordinate(cell, "x"), coordinate(cell, "y")) else {
            push(
                &mut errors,
                "INVALID_CELL_COORDINATE",
                &path,
                "cell x and y must be non-negative u32 coordinates",
            );
            validate_cell_measurement(cell, &path, &mut errors);
            continue;
        };

        if let (Some(w), Some(h)) = (width, height) {
            if x >= w || y >= h {
                push(
                    &mut errors,
                    "CELL_OUT_OF_BOUNDS",
                    &path,
                    "cell coordinate lies outside the declared grid",
                );
            }
        }

        if !seen_cells.insert((x, y)) {
            push(
                &mut errors,
                "DUPLICATE_MAP_CELL",
                &path,
                "each grid coordinate may appear exactly once",
            );
        }

        validate_cell_measurement(cell, &path, &mut errors);
    }

    if let Some(area) = cell_size.map(|v| v * v).filter(|v| *v > 0.0 && v.is_finite()) {
        let mut lower_volume = 0.0;
        let mut upper_volume = 0.0;
        let mut complete_bounds = true;
        for cell in cells {
            match (
                number(cell, "residual_thickness_lower_mm"),
                number(cell, "residual_thickness_upper_mm"),
            ) {
                (Some(lower), Some(upper)) => {
                    lower_volume += area * lower;
                    upper_volume += area * upper;
                }
                _ => complete_bounds = false,
            }
        }
        if complete_bounds && (!lower_volume.is_finite() || !upper_volume.is_finite()) {
            push(
                &mut errors,
                "RESIDUAL_VOLUME_OVERFLOW",
                "$.map.cells",
                "residual-volume interval overflows finite arithmetic",
            );
        }
    }

    if let Some(expected) = expected_count {
        if seen_cells.len() != expected {
            push(
                &mut errors,
                "INCOMPLETE_MAP_COVERAGE",
                "$.map.cells",
                format!(
                    "complete-map profile requires {expected} unique cells, found {}",
                    seen_cells.len()
                ),
            );
        }
    }

    let Some(supports) = case.get("support_profiles").and_then(Value::as_array) else {
        push(
            &mut errors,
            "MISSING_SUPPORT_PROFILES",
            "$.support_profiles",
            "support_profiles must be an array",
        );
        return errors;
    };

    if supports.is_empty() {
        push(
            &mut errors,
            "EMPTY_SUPPORT_PROFILES",
            "$.support_profiles",
            "at least one support footprint is required",
        );
    }

    let mut support_ids = BTreeSet::new();
    for (support_index, support) in supports.iter().enumerate() {
        let path = format!("$.support_profiles[{support_index}]");
        let support_id = text(support, "support_id");

        if let Some(id) = support_id {
            if !support_ids.insert(id.to_owned()) {
                push(
                    &mut errors,
                    "DUPLICATE_SUPPORT_ID",
                    &format!("{path}.support_id"),
                    "support_id values must be unique",
                );
            }
        } else {
            push(
                &mut errors,
                "MISSING_SUPPORT_ID",
                &format!("{path}.support_id"),
                "a non-empty support_id is required",
            );
        }

        if text(support, "frame_id") != frame_id {
            push(
                &mut errors,
                "SUPPORT_FRAME_MISMATCH",
                &format!("{path}.frame_id"),
                "support footprint frame must exactly match the residual map frame",
            );
        }

        for key in ["site_id", "machine_instance_id", "machine_configuration_ref"] {
            if text(support, key) != text(map, key) {
                push(
                    &mut errors,
                    "SUPPORT_CONTEXT_MISMATCH",
                    &format!("{path}.{key}"),
                    "support footprint must bind to the exact map site, machine instance and configuration",
                );
            }
        }

        if text(support, "acceptance_profile_ref").is_none() {
            push(
                &mut errors,
                "MISSING_ACCEPTANCE_PROFILE",
                &format!("{path}.acceptance_profile_ref"),
                "no default threshold exists; an external machine/site acceptance profile is required",
            );
        }

        for key in [
            "maximum_residual_thickness_mm",
            "maximum_registration_uncertainty_mm",
        ] {
            match number(support, key) {
                Some(v) if v >= 0.0 => {}
                _ => push(
                    &mut errors,
                    "INVALID_SUPPORT_THRESHOLD",
                    &format!("{path}.{key}"),
                    "support thresholds must be finite, non-negative and explicit",
                ),
            }
        }

        let Some(footprint) = support.get("footprint_cells").and_then(Value::as_array) else {
            push(
                &mut errors,
                "MISSING_SUPPORT_FOOTPRINT",
                &format!("{path}.footprint_cells"),
                "footprint_cells must be a non-empty array",
            );
            continue;
        };

        if footprint.is_empty() {
            push(
                &mut errors,
                "EMPTY_SUPPORT_FOOTPRINT",
                &format!("{path}.footprint_cells"),
                "a support footprint must contain at least one cell",
            );
        }

        let mut footprint_seen = BTreeSet::new();
        for (cell_index, cell) in footprint.iter().enumerate() {
            let cell_path = format!("{path}.footprint_cells[{cell_index}]");
            let (Some(x), Some(y)) = (coordinate(cell, "x"), coordinate(cell, "y")) else {
                push(
                    &mut errors,
                    "INVALID_FOOTPRINT_COORDINATE",
                    &cell_path,
                    "footprint x and y must be non-negative u32 coordinates",
                );
                continue;
            };

            if let (Some(w), Some(h)) = (width, height) {
                if x >= w || y >= h {
                    push(
                        &mut errors,
                        "FOOTPRINT_OUT_OF_BOUNDS",
                        &cell_path,
                        "support footprint coordinate lies outside the map grid",
                    );
                }
            }

            if !footprint_seen.insert((x, y)) {
                push(
                    &mut errors,
                    "DUPLICATE_FOOTPRINT_CELL",
                    &cell_path,
                    "a support footprint cannot count the same cell more than once",
                );
            }
        }
    }

    errors
}

/// Evaluate a validated case. Missing map cells are an error, never zero-filled.
/// Registration outside a support profile's declared limit yields UNKNOWN for
/// that support rather than a false CLEAR/OBSTRUCTED classification.
pub fn evaluate_case(case: &Value) -> Result<EvaluationReport, Vec<Violation>> {
    let errors = validate_case(case);
    if !errors.is_empty() {
        return Err(errors);
    }

    let map = case.get("map").expect("validated map");
    let cells = map
        .get("cells")
        .and_then(Value::as_array)
        .expect("validated cells array");
    let cell_size = number(map, "cell_size_mm").expect("validated cell size");
    let cell_area = cell_size * cell_size;
    let registration_uncertainty =
        number(map, "registration_uncertainty_mm").expect("validated registration uncertainty");

    let cells_by_coord: BTreeMap<(u32, u32), &Value> = cells
        .iter()
        .map(|cell| {
            (
                (
                    coordinate(cell, "x").expect("validated x coordinate"),
                    coordinate(cell, "y").expect("validated y coordinate"),
                ),
                cell,
            )
        })
        .collect();

    let mut residual_lower = 0.0;
    let mut residual_upper = 0.0;
    for cell in cells {
        residual_lower += cell_area
            * number(cell, "residual_thickness_lower_mm").expect("validated lower bound");
        residual_upper += cell_area
            * number(cell, "residual_thickness_upper_mm").expect("validated upper bound");
    }

    let support_profiles = case
        .get("support_profiles")
        .and_then(Value::as_array)
        .expect("validated support profiles");
    let mut support_reports = Vec::with_capacity(support_profiles.len());

    for support in support_profiles {
        let support_id = text(support, "support_id").expect("validated support id");
        let profile_ref = text(support, "acceptance_profile_ref")
            .expect("validated acceptance profile reference");
        let threshold =
            number(support, "maximum_residual_thickness_mm").expect("validated threshold");
        let maximum_registration_uncertainty =
            number(support, "maximum_registration_uncertainty_mm")
                .expect("validated registration threshold");
        let footprint = support
            .get("footprint_cells")
            .and_then(Value::as_array)
            .expect("validated footprint");

        let mut clear = 0usize;
        let mut obstructed = 0usize;
        let mut ambiguous = 0usize;
        let mut unknown = 0usize;
        let mut maximum_upper: Option<f64> = None;

        if registration_uncertainty > maximum_registration_uncertainty {
            unknown = footprint.len();
        } else {
            for point in footprint {
                let coord = (
                    coordinate(point, "x").expect("validated footprint x"),
                    coordinate(point, "y").expect("validated footprint y"),
                );
                let cell = cells_by_coord.get(&coord).expect("complete validated map");
                let lower = number(cell, "residual_thickness_lower_mm")
                    .expect("validated lower thickness");
                let upper = number(cell, "residual_thickness_upper_mm")
                    .expect("validated upper thickness");
                maximum_upper = Some(maximum_upper.map_or(upper, |current| current.max(upper)));

                if upper <= threshold {
                    clear += 1;
                } else if lower > threshold {
                    obstructed += 1;
                } else {
                    ambiguous += 1;
                }
            }
        }

        let count = footprint.len();
        let denominator = count as f64;
        let disposition = if unknown > 0 {
            "UNKNOWN"
        } else if obstructed > 0 {
            "OBSTRUCTED"
        } else if ambiguous > 0 {
            "AMBIGUOUS"
        } else {
            "CLEAR"
        };

        support_reports.push(SupportReport {
            support_id: support_id.to_owned(),
            acceptance_profile_ref: profile_ref.to_owned(),
            threshold_mm: threshold,
            footprint_cell_count: count,
            clear_cells: clear,
            obstructed_cells: obstructed,
            ambiguous_cells: ambiguous,
            unknown_cells: unknown,
            clear_fraction: clear as f64 / denominator,
            obstructed_fraction: obstructed as f64 / denominator,
            ambiguous_fraction: ambiguous as f64 / denominator,
            unknown_fraction: unknown as f64 / denominator,
            maximum_thickness_upper_mm: maximum_upper,
            disposition: disposition.to_owned(),
        });
    }

    Ok(EvaluationReport {
        case_id: text(case, "case_id").expect("validated case id").to_owned(),
        evidence_class: text(case, "evidence_class")
            .expect("validated evidence class")
            .to_owned(),
        map_id: text(map, "map_id").expect("validated map id").to_owned(),
        frame_id: text(map, "frame_id").expect("validated frame id").to_owned(),
        map_cell_count: cells.len(),
        cell_area_mm2: cell_area,
        registration_uncertainty_mm: registration_uncertainty,
        residual_volume_lower_mm3: residual_lower,
        residual_volume_upper_mm3: residual_upper,
        coverage_disposition: "COMPLETE".to_owned(),
        support_reports,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../fixtures/synthetic-residual-contact-case.v0_1.json"
        ))
        .expect("synthetic fixture parses")
    }

    fn cell_mut(case: &mut Value, x: u64, y: u64) -> &mut Value {
        case["map"]["cells"]
            .as_array_mut()
            .expect("cells")
            .iter_mut()
            .find(|cell| {
                cell.get("x").and_then(Value::as_u64) == Some(x)
                    && cell.get("y").and_then(Value::as_u64) == Some(y)
            })
            .expect("fixture cell")
    }

    fn left_support(report: &EvaluationReport) -> &SupportReport {
        report
            .support_reports
            .iter()
            .find(|support| support.support_id == "left-gripper")
            .expect("left support")
    }

    fn right_support(report: &EvaluationReport) -> &SupportReport {
        report
            .support_reports
            .iter()
            .find(|support| support.support_id == "right-gripper")
            .expect("right support")
    }

    #[test]
    fn synthetic_fixture_is_semantically_valid() {
        assert!(validate_case(&fixture()).is_empty());
    }

    #[test]
    fn synthetic_replay_reports_spatially_distinct_support_exposure() {
        let report = evaluate_case(&fixture()).expect("fixture evaluates");
        let left = left_support(&report);
        let right = right_support(&report);

        assert_eq!(left.disposition, "AMBIGUOUS");
        assert_eq!(left.clear_cells, 2);
        assert_eq!(left.ambiguous_cells, 1);
        assert_eq!(right.disposition, "OBSTRUCTED");
        assert_eq!(right.clear_cells, 1);
        assert_eq!(right.obstructed_cells, 1);
    }

    #[test]
    fn residual_volume_uses_each_map_cell_once() {
        let report = evaluate_case(&fixture()).expect("fixture evaluates");
        assert_eq!(report.residual_volume_lower_mm3, 620_000.0);
        assert_eq!(report.residual_volume_upper_mm3, 850_000.0);
    }

    #[test]
    fn overlapping_support_footprints_do_not_duplicate_map_volume() {
        let mut case = fixture();
        let original = evaluate_case(&case).expect("original evaluates");
        let duplicate_support = case["support_profiles"][0].clone();
        case["support_profiles"]
            .as_array_mut()
            .expect("support array")
            .push(duplicate_support);

        let extended = evaluate_case(&case).expect("overlap remains evaluable");
        assert_eq!(
            original.residual_volume_lower_mm3,
            extended.residual_volume_lower_mm3
        );
        assert_eq!(
            original.residual_volume_upper_mm3,
            extended.residual_volume_upper_mm3
        );
    }

    #[test]
    fn duplicate_map_cell_is_rejected() {
        let mut case = fixture();
        let duplicate = case["map"]["cells"][0].clone();
        case["map"]["cells"].as_array_mut().unwrap().push(duplicate);
        assert!(validate_case(&case)
            .iter()
            .any(|v| v.code == "DUPLICATE_MAP_CELL"));
    }

    #[test]
    fn missing_map_cell_is_not_zero_filled() {
        let mut case = fixture();
        case["map"]["cells"].as_array_mut().unwrap().remove(0);
        assert!(validate_case(&case)
            .iter()
            .any(|v| v.code == "INCOMPLETE_MAP_COVERAGE"));
    }

    #[test]
    fn out_of_bounds_map_cell_is_rejected() {
        let mut case = fixture();
        case["map"]["cells"][0]["x"] = serde_json::json!(4);
        assert!(validate_case(&case)
            .iter()
            .any(|v| v.code == "CELL_OUT_OF_BOUNDS"));
    }

    #[test]
    fn negative_thickness_is_rejected() {
        let mut case = fixture();
        cell_mut(&mut case, 0, 0)["residual_thickness_lower_mm"] = serde_json::json!(-1.0);
        assert!(validate_case(&case)
            .iter()
            .any(|v| v.code == "NEGATIVE_RESIDUAL_THICKNESS"));
    }

    #[test]
    fn reversed_thickness_interval_is_rejected() {
        let mut case = fixture();
        cell_mut(&mut case, 0, 0)["residual_thickness_lower_mm"] = serde_json::json!(4.0);
        cell_mut(&mut case, 0, 0)["residual_thickness_upper_mm"] = serde_json::json!(2.0);
        assert!(validate_case(&case)
            .iter()
            .any(|v| v.code == "REVERSED_RESIDUAL_INTERVAL"));
    }

    #[test]
    fn model_derived_residue_cannot_be_promoted_to_measurement() {
        let mut case = fixture();
        cell_mut(&mut case, 0, 0)["evidence_role"] = serde_json::json!("MODEL_ESTIMATE");
        assert!(validate_case(&case)
            .iter()
            .any(|v| v.code == "NON_INDEPENDENT_RESIDUAL_EVIDENCE"));
    }

    #[test]
    fn missing_cell_calibration_is_rejected() {
        let mut case = fixture();
        cell_mut(&mut case, 0, 0).as_object_mut().unwrap().remove("calibration_ref");
        assert!(validate_case(&case)
            .iter()
            .any(|v| v.code == "MISSING_CELL_PROVENANCE"));
    }

    #[test]
    fn support_frame_must_match_map_frame() {
        let mut case = fixture();
        case["support_profiles"][0]["frame_id"] = serde_json::json!("other-frame");
        assert!(validate_case(&case)
            .iter()
            .any(|v| v.code == "SUPPORT_FRAME_MISMATCH"));
    }

    #[test]
    fn acceptance_profile_is_mandatory() {
        let mut case = fixture();
        case["support_profiles"][0]
            .as_object_mut()
            .unwrap()
            .remove("acceptance_profile_ref");
        assert!(validate_case(&case)
            .iter()
            .any(|v| v.code == "MISSING_ACCEPTANCE_PROFILE"));
    }

    #[test]
    fn negative_support_threshold_is_rejected() {
        let mut case = fixture();
        case["support_profiles"][0]["maximum_residual_thickness_mm"] =
            serde_json::json!(-1.0);
        assert!(validate_case(&case)
            .iter()
            .any(|v| v.code == "INVALID_SUPPORT_THRESHOLD"));
    }

    #[test]
    fn duplicate_footprint_cell_is_rejected() {
        let mut case = fixture();
        let duplicate = case["support_profiles"][0]["footprint_cells"][0].clone();
        case["support_profiles"][0]["footprint_cells"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(validate_case(&case)
            .iter()
            .any(|v| v.code == "DUPLICATE_FOOTPRINT_CELL"));
    }

    #[test]
    fn out_of_bounds_footprint_cell_is_rejected() {
        let mut case = fixture();
        case["support_profiles"][0]["footprint_cells"][0]["x"] = serde_json::json!(4);
        assert!(validate_case(&case)
            .iter()
            .any(|v| v.code == "FOOTPRINT_OUT_OF_BOUNDS"));
    }

    #[test]
    fn excessive_registration_uncertainty_yields_unknown_not_clear() {
        let mut case = fixture();
        case["map"]["registration_uncertainty_mm"] = serde_json::json!(3.0);
        let report = evaluate_case(&case).expect("registration exceedance is reportable");
        let left = left_support(&report);
        assert_eq!(left.disposition, "UNKNOWN");
        assert_eq!(left.unknown_cells, 3);
        assert_eq!(left.clear_cells, 0);
        assert!(left.maximum_thickness_upper_mm.is_none());
    }

    #[test]
    fn interval_touching_threshold_is_clear() {
        let mut case = fixture();
        case["support_profiles"][0]["maximum_residual_thickness_mm"] =
            serde_json::json!(11.0);
        let report = evaluate_case(&case).expect("fixture evaluates");
        assert_eq!(left_support(&report).disposition, "CLEAR");
    }

    #[test]
    fn interval_straddling_threshold_is_ambiguous() {
        let mut case = fixture();
        case["support_profiles"][0]["maximum_residual_thickness_mm"] =
            serde_json::json!(9.5);
        let report = evaluate_case(&case).expect("fixture evaluates");
        assert_eq!(left_support(&report).disposition, "AMBIGUOUS");
    }

    #[test]
    fn interval_wholly_above_threshold_is_obstructed() {
        let mut case = fixture();
        case["support_profiles"][0]["maximum_residual_thickness_mm"] =
            serde_json::json!(3.0);
        let report = evaluate_case(&case).expect("fixture evaluates");
        assert_eq!(left_support(&report).disposition, "OBSTRUCTED");
    }

    #[test]
    fn registration_reference_is_required() {
        let mut case = fixture();
        case["map"].as_object_mut().unwrap().remove("registration_ref");
        assert!(validate_case(&case)
            .iter()
            .any(|v| v.code == "MISSING_REGISTRATION_PROVENANCE"));
    }

    #[test]
    fn zero_or_missing_cell_size_is_rejected() {
        let mut case = fixture();
        case["map"]["cell_size_mm"] = serde_json::json!(0.0);
        assert!(validate_case(&case)
            .iter()
            .any(|v| v.code == "INVALID_CELL_SIZE"));
    }

    #[test]
    fn underflowing_cell_area_is_rejected() {
        let mut case = fixture();
        case["map"]["cell_size_mm"] = serde_json::json!(1.0e-200);
        assert!(validate_case(&case)
            .iter()
            .any(|v| v.code == "INVALID_CELL_SIZE"));
    }

    #[test]
    fn overflowing_residual_volume_is_rejected() {
        let mut case = fixture();
        case["map"]["cell_size_mm"] = serde_json::json!(1.0e100);
        cell_mut(&mut case, 0, 0)["residual_thickness_upper_mm"] =
            serde_json::json!(1.0e200);
        assert!(validate_case(&case)
            .iter()
            .any(|v| v.code == "RESIDUAL_VOLUME_OVERFLOW"));
    }

    #[test]
    fn support_machine_configuration_must_match_map() {
        let mut case = fixture();
        case["support_profiles"][0]["machine_configuration_ref"] =
            serde_json::json!("machine-config:other");
        assert!(validate_case(&case)
            .iter()
            .any(|v| v.code == "SUPPORT_CONTEXT_MISMATCH"));
    }

    #[test]
    fn map_trial_and_machine_identity_are_mandatory() {
        let mut case = fixture();
        case["map"].as_object_mut().unwrap().remove("machine_instance_id");
        assert!(validate_case(&case)
            .iter()
            .any(|v| v.code == "MISSING_MAP_CONTEXT"));
    }

    #[test]
    fn duplicate_support_ids_are_rejected() {
        let mut case = fixture();
        let duplicate = case["support_profiles"][0].clone();
        case["support_profiles"].as_array_mut().unwrap().push(duplicate);
        assert!(validate_case(&case)
            .iter()
            .any(|v| v.code == "DUPLICATE_SUPPORT_ID"));
    }

    #[test]
    fn missing_evidence_class_is_rejected() {
        let mut case = fixture();
        case.as_object_mut().unwrap().remove("evidence_class");
        assert!(validate_case(&case)
            .iter()
            .any(|v| v.code == "INVALID_EVIDENCE_CLASS"));
    }

    #[test]
    fn replay_is_deterministic() {
        let case = fixture();
        let first = evaluate_case(&case).expect("first replay");
        let second = evaluate_case(&case).expect("second replay");
        assert_eq!(first, second);
    }

    #[test]
    fn invalid_cases_return_violations_instead_of_partial_success() {
        let mut case = fixture();
        case["map"]["cells"].as_array_mut().unwrap().remove(0);
        let result = evaluate_case(&case);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .iter()
            .any(|v| v.code == "INCOMPLETE_MAP_COVERAGE"));
    }
}
