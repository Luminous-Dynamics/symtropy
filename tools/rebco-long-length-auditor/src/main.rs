// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic structural auditor for spatially registered REBCO reel evidence.
//!
//! This tool validates identity, geometry, evidence-state semantics and conservative
//! full-width Ic coverage. It is not a material-property predictor or product-release
//! authority. The default fixture is explicitly synthetic.

use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const DEFAULT_FIXTURE: &str = "docs/research/fixtures/rebco_long_length_synthetic_v0_1.json";
const CLAIM_CEILING: &str = "spatial-genealogy-and-evidence-semantics-only";

#[derive(Debug, Clone)]
struct Reel {
    id: String,
    total_length_mm: f64,
    width_mm: f64,
}

#[derive(Debug, Clone)]
struct Segment {
    id: String,
    longitudinal_start_mm: f64,
    longitudinal_end_mm: f64,
    width_start_mm: f64,
    width_end_mm: f64,
    face: String,
}

#[derive(Debug, Clone, Copy)]
struct Interval {
    start: f64,
    end: f64,
}

#[derive(Debug, Clone)]
struct AuditReport {
    reel_id: String,
    segment_count: usize,
    observation_count: usize,
    evidence_state_counts: BTreeMap<String, usize>,
    ic_direct_transport_qualified_length_mm: f64,
    ic_calibrated_proxy_length_mm: f64,
    ic_inferred_full_width_length_mm: f64,
    ic_direct_transport_unqualified_length_mm: f64,
    ic_proxy_unqualified_length_mm: f64,
    ic_explicitly_unmeasured_full_width_length_mm: f64,
    ic_direct_transport_coverage_fraction: f64,
    ic_not_directly_measured_length_mm: f64,
    acceptance_disposition: &'static str,
}

fn fail(path: &str, message: impl AsRef<str>) -> String {
    format!("{path}: {}", message.as_ref())
}

fn object<'a>(value: &'a Value, path: &str) -> Result<&'a Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| fail(path, "must be an object"))
}

fn array<'a>(value: &'a Value, path: &str) -> Result<&'a Vec<Value>, String> {
    value
        .as_array()
        .ok_or_else(|| fail(path, "must be an array"))
}

fn field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a Value, String> {
    object(value, path)?
        .get(key)
        .ok_or_else(|| fail(path, format!("missing property {key}")))
}

fn string_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    field(value, key, path)?
        .as_str()
        .filter(|item| !item.trim().is_empty())
        .ok_or_else(|| fail(path, format!("{key} must be a non-empty string")))
}

fn number_field(value: &Value, key: &str, path: &str) -> Result<f64, String> {
    let number = field(value, key, path)?
        .as_f64()
        .ok_or_else(|| fail(path, format!("{key} must be a finite number")))?;
    if !number.is_finite() {
        return Err(fail(path, format!("{key} must be finite")));
    }
    Ok(number)
}

fn require_keys(
    value: &Value,
    required: &[&str],
    optional: &[&str],
    path: &str,
) -> Result<(), String> {
    let map = object(value, path)?;
    for key in required {
        if !map.contains_key(*key) {
            return Err(fail(path, format!("missing property {key}")));
        }
    }
    if let Some(key) = map
        .keys()
        .find(|key| !required.contains(&key.as_str()) && !optional.contains(&key.as_str()))
    {
        return Err(fail(path, format!("unknown property {key}")));
    }
    Ok(())
}

fn enum_value(value: &Value, key: &str, allowed: &[&str], path: &str) -> Result<String, String> {
    let text = string_field(value, key, path)?;
    if !allowed.contains(&text) {
        return Err(fail(path, format!("{key} has unsupported value {text:?}")));
    }
    Ok(text.to_string())
}

fn validate_digest(value: &str, path: &str) -> Result<(), String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(fail(path, "must be lowercase 64-character SHA-256 hex"));
    }
    Ok(())
}

fn validate_hash_field(value: &Value, key: &str, path: &str) -> Result<(), String> {
    validate_digest(string_field(value, key, path)?, &format!("{path}.{key}"))
}

fn validate_interval_value(value: &Value, path: &str) -> Result<(), String> {
    if let Some(number) = value.as_f64() {
        if !number.is_finite() {
            return Err(fail(path, "value must be finite"));
        }
        return Ok(());
    }
    let interval = value
        .as_object()
        .ok_or_else(|| fail(path, "must be a number or {lower, upper} interval"))?;
    if interval.len() != 2 || !interval.contains_key("lower") || !interval.contains_key("upper") {
        return Err(fail(path, "interval must contain exactly lower and upper"));
    }
    let lower = interval["lower"]
        .as_f64()
        .ok_or_else(|| fail(path, "interval lower must be finite"))?;
    let upper = interval["upper"]
        .as_f64()
        .ok_or_else(|| fail(path, "interval upper must be finite"))?;
    if !lower.is_finite() || !upper.is_finite() || lower > upper {
        return Err(fail(
            path,
            "interval bounds must be finite and lower <= upper",
        ));
    }
    Ok(())
}

fn validate_conditions(value: &Value, path: &str) -> Result<(), String> {
    const ALLOWED: &[&str] = &[
        "temperature_k",
        "field_magnitude_t",
        "field_vector_t",
        "field_orientation",
        "current_criterion",
        "test_current_a",
        "electric_field_criterion_uv_per_cm",
        "strain",
        "geometry",
        "frequency_hz",
    ];
    let map = object(value, path)?;
    if let Some(key) = map.keys().find(|key| !ALLOWED.contains(&key.as_str())) {
        return Err(fail(path, format!("unknown condition field {key}")));
    }
    for key in [
        "temperature_k",
        "field_magnitude_t",
        "test_current_a",
        "electric_field_criterion_uv_per_cm",
        "frequency_hz",
    ] {
        if let Some(item) = map.get(key) {
            let number = item
                .as_f64()
                .ok_or_else(|| fail(path, format!("{key} must be a finite number")))?;
            let must_be_positive = key == "temperature_k" || key == "frequency_hz";
            if !number.is_finite() || number < 0.0 || (must_be_positive && number == 0.0) {
                return Err(fail(path, format!("{key} is outside its valid range")));
            }
        }
    }
    if let Some(vector) = map.get("field_vector_t") {
        let values = array(vector, &format!("{path}.field_vector_t"))?;
        if values.len() != 3
            || values
                .iter()
                .any(|v| v.as_f64().is_none_or(|n| !n.is_finite()))
        {
            return Err(fail(
                path,
                "field_vector_t must contain three finite numbers",
            ));
        }
    }
    for key in ["field_orientation", "current_criterion", "geometry"] {
        if let Some(item) = map.get(key)
            && item.as_str().is_none_or(|text| text.trim().is_empty())
        {
            return Err(fail(path, format!("{key} must be a non-empty string")));
        }
    }
    if let Some(strain) = map.get("strain")
        && strain.as_f64().is_none_or(|number| !number.is_finite())
    {
        return Err(fail(path, "strain must be a finite number"));
    }
    Ok(())
}

fn validate_uncertainty(value: &Value, path: &str) -> Result<(), String> {
    require_keys(
        value,
        &["type"],
        &[
            "standard_uncertainty",
            "coverage_factor",
            "confidence_level",
            "lower_bound",
            "upper_bound",
            "covariance_group_id",
        ],
        path,
    )?;
    let kind = enum_value(
        value,
        "type",
        &[
            "standard_uncertainty",
            "expanded_uncertainty",
            "interval",
            "qualitative",
            "unknown",
        ],
        path,
    )?;
    if kind == "standard_uncertainty" && !object(value, path)?.contains_key("standard_uncertainty")
    {
        return Err(fail(
            path,
            "standard_uncertainty type requires standard_uncertainty",
        ));
    }
    if kind == "expanded_uncertainty"
        && (!object(value, path)?.contains_key("standard_uncertainty")
            || !object(value, path)?.contains_key("coverage_factor"))
    {
        return Err(fail(
            path,
            "expanded_uncertainty requires standard_uncertainty and coverage_factor",
        ));
    }
    if kind == "interval"
        && (!object(value, path)?.contains_key("lower_bound")
            || !object(value, path)?.contains_key("upper_bound"))
    {
        return Err(fail(
            path,
            "interval uncertainty requires lower_bound and upper_bound",
        ));
    }
    for key in ["standard_uncertainty", "coverage_factor"] {
        if let Some(item) = object(value, path)?.get(key) {
            let number = item
                .as_f64()
                .ok_or_else(|| fail(path, format!("{key} must be finite")))?;
            if !number.is_finite() || number < 0.0 || key == "coverage_factor" && number == 0.0 {
                return Err(fail(path, format!("{key} is outside its valid range")));
            }
        }
    }
    if let Some(item) = object(value, path)?.get("confidence_level") {
        let number = item
            .as_f64()
            .ok_or_else(|| fail(path, "confidence_level must be finite"))?;
        if !number.is_finite() || !(0.0..=1.0).contains(&number) || number == 0.0 {
            return Err(fail(path, "confidence_level must be in (0, 1]"));
        }
    }
    if let (Some(lower), Some(upper)) = (
        object(value, path)?.get("lower_bound"),
        object(value, path)?.get("upper_bound"),
    ) {
        let lower = lower
            .as_f64()
            .ok_or_else(|| fail(path, "lower_bound must be finite"))?;
        let upper = upper
            .as_f64()
            .ok_or_else(|| fail(path, "upper_bound must be finite"))?;
        if !lower.is_finite() || !upper.is_finite() || lower > upper {
            return Err(fail(
                path,
                "uncertainty interval must be finite and lower <= upper",
            ));
        }
    }
    Ok(())
}

fn validate_measurement_system(value: &Value, path: &str) -> Result<(), String> {
    require_keys(
        value,
        &["instrument_id", "calibration_revision"],
        &[
            "calibration_valid_at_measurement",
            "operator_or_automation_id",
        ],
        path,
    )?;
    string_field(value, "instrument_id", path)?;
    string_field(value, "calibration_revision", path)?;
    if let Some(valid) = object(value, path)?.get("calibration_valid_at_measurement")
        && !valid.is_boolean()
    {
        return Err(fail(
            path,
            "calibration_valid_at_measurement must be boolean",
        ));
    }
    if object(value, path)?.contains_key("operator_or_automation_id") {
        string_field(value, "operator_or_automation_id", path)?;
    }
    Ok(())
}

fn interval_union_length(mut intervals: Vec<Interval>) -> f64 {
    if intervals.is_empty() {
        return 0.0;
    }
    intervals.sort_by(|left, right| left.start.total_cmp(&right.start));
    let mut total = 0.0;
    let mut start = intervals[0].start;
    let mut end = intervals[0].end;
    for interval in intervals.into_iter().skip(1) {
        if interval.start <= end {
            end = end.max(interval.end);
        } else {
            total += end - start;
            start = interval.start;
            end = interval.end;
        }
    }
    total + end - start
}

fn rectangles_overlap(left: &Segment, right: &Segment) -> bool {
    left.longitudinal_start_mm < right.longitudinal_end_mm
        && right.longitudinal_start_mm < left.longitudinal_end_mm
        && left.width_start_mm < right.width_end_mm
        && right.width_start_mm < left.width_end_mm
}

fn validate_document(value: &Value) -> Result<AuditReport, String> {
    require_keys(
        value,
        &[
            "schema_version",
            "evidence_class",
            "claim_ceiling",
            "reel",
            "segments",
            "observations",
        ],
        &["application_profiles", "provenance"],
        "$",
    )?;
    if string_field(value, "schema_version", "$")? != "rebco-long-length-spatial-qualification.v0.1"
    {
        return Err(fail("$.schema_version", "unsupported schema version"));
    }
    let evidence_class = enum_value(
        value,
        "evidence_class",
        &["synthetic", "source_reported", "measured", "derived"],
        "$",
    )?;
    if string_field(value, "claim_ceiling", "$")? != CLAIM_CEILING {
        return Err(fail("$.claim_ceiling", "claim ceiling does not match v0.1"));
    }

    let reel_value = field(value, "reel", "$")?;
    require_keys(
        reel_value,
        &[
            "reel_id",
            "lot_id",
            "campaign_id",
            "process_run_id",
            "architecture_id",
            "total_length_mm",
            "width_mm",
            "longitudinal_datum",
            "positive_direction",
            "width_datum",
            "coordinate_units",
        ],
        &["face_convention", "source_manifest_sha256"],
        "$.reel",
    )?;
    let reel_id = string_field(reel_value, "reel_id", "$.reel")?.to_string();
    for key in [
        "lot_id",
        "campaign_id",
        "process_run_id",
        "architecture_id",
        "longitudinal_datum",
        "width_datum",
    ] {
        string_field(reel_value, key, "$.reel")?;
    }
    enum_value(
        reel_value,
        "positive_direction",
        &[
            "winding_direction",
            "unwinding_direction",
            "declared_axis_positive",
        ],
        "$.reel",
    )?;
    if string_field(reel_value, "coordinate_units", "$.reel")? != "mm" {
        return Err(fail("$.reel.coordinate_units", "v0.1 requires millimetres"));
    }
    let total_length_mm = number_field(reel_value, "total_length_mm", "$.reel")?;
    let width_mm = number_field(reel_value, "width_mm", "$.reel")?;
    if total_length_mm <= 0.0 || width_mm <= 0.0 {
        return Err(fail("$.reel", "reel dimensions must be positive"));
    }
    if let Some(digest) = object(reel_value, "$.reel")?.get("source_manifest_sha256") {
        validate_digest(
            digest
                .as_str()
                .ok_or_else(|| fail("$.reel.source_manifest_sha256", "must be a string"))?,
            "$.reel.source_manifest_sha256",
        )?;
    }
    let reel = Reel {
        id: reel_id,
        total_length_mm,
        width_mm,
    };

    let segment_values = array(field(value, "segments", "$")?, "$.segments")?;
    if segment_values.is_empty() {
        return Err(fail("$.segments", "must contain at least one segment"));
    }
    let mut segments = BTreeMap::<String, Segment>::new();
    for (index, item) in segment_values.iter().enumerate() {
        let path = format!("$.segments[{index}]");
        require_keys(
            item,
            &[
                "segment_id",
                "parent_reel_id",
                "longitudinal_start_mm",
                "longitudinal_end_mm",
                "width_start_mm",
                "width_end_mm",
                "face",
                "genealogy_state",
            ],
            &[
                "parent_segment_ids",
                "transformation_event_ids",
                "splice_or_joint_ids",
            ],
            &path,
        )?;
        let id = string_field(item, "segment_id", &path)?.to_string();
        let parent_reel_id = string_field(item, "parent_reel_id", &path)?.to_string();
        if parent_reel_id != reel.id {
            return Err(fail(
                &path,
                "parent_reel_id does not match the declared reel",
            ));
        }
        let segment = Segment {
            id: id.clone(),
            longitudinal_start_mm: number_field(item, "longitudinal_start_mm", &path)?,
            longitudinal_end_mm: number_field(item, "longitudinal_end_mm", &path)?,
            width_start_mm: number_field(item, "width_start_mm", &path)?,
            width_end_mm: number_field(item, "width_end_mm", &path)?,
            face: enum_value(item, "face", &["a", "b", "both", "unknown"], &path)?,
        };
        let genealogy = enum_value(
            item,
            "genealogy_state",
            &[
                "original", "cut", "split", "merged", "spliced", "reworked", "scrapped", "unknown",
            ],
            &path,
        )?;
        if segment.longitudinal_start_mm < 0.0
            || segment.longitudinal_end_mm > reel.total_length_mm
            || segment.longitudinal_start_mm >= segment.longitudinal_end_mm
            || segment.width_start_mm < 0.0
            || segment.width_end_mm > reel.width_mm
            || segment.width_start_mm >= segment.width_end_mm
        {
            return Err(fail(
                &path,
                "segment rectangle lies outside reel bounds or has zero area",
            ));
        }
        for optional_ids in [
            "parent_segment_ids",
            "transformation_event_ids",
            "splice_or_joint_ids",
        ] {
            if let Some(ids) = object(item, &path)?.get(optional_ids) {
                let values = array(ids, &format!("{path}.{optional_ids}"))?;
                let mut unique = BTreeSet::new();
                for id_value in values {
                    let id = id_value
                        .as_str()
                        .filter(|text| !text.trim().is_empty())
                        .ok_or_else(|| {
                            fail(
                                &path,
                                format!("{optional_ids} values must be non-empty strings"),
                            )
                        })?;
                    if !unique.insert(id) {
                        return Err(fail(&path, format!("{optional_ids} contains a duplicate")));
                    }
                }
            }
        }
        if genealogy == "original" && object(item, &path)?.contains_key("parent_segment_ids") {
            let ids = array(
                &item["parent_segment_ids"],
                &format!("{path}.parent_segment_ids"),
            )?;
            if !ids.is_empty() {
                return Err(fail(
                    &path,
                    "original segment cannot declare parent segment IDs",
                ));
            }
        }
        if segments.insert(id, segment).is_some() {
            return Err(fail(&path, "duplicate segment_id"));
        }
    }
    let segment_list: Vec<&Segment> = segments.values().collect();
    for (index, left) in segment_list.iter().enumerate() {
        for right in segment_list.iter().skip(index + 1) {
            if rectangles_overlap(left, right) {
                return Err(fail(
                    "$.segments",
                    format!("segments {} and {} overlap spatially", left.id, right.id),
                ));
            }
        }
    }

    let observation_values = array(field(value, "observations", "$")?, "$.observations")?;
    let mut observation_ids = BTreeSet::new();
    let mut state_counts = BTreeMap::<String, usize>::new();
    let mut ic_direct_transport = Vec::<Interval>::new();
    let mut ic_calibrated_proxy = Vec::<Interval>::new();
    let mut ic_inferred = Vec::<Interval>::new();
    let mut ic_direct_transport_unqualified = Vec::<Interval>::new();
    let mut ic_proxy_unqualified = Vec::<Interval>::new();
    let mut ic_unmeasured = Vec::<Interval>::new();

    for (index, observation) in observation_values.iter().enumerate() {
        let path = format!("$.observations[{index}]");
        require_keys(
            observation,
            &[
                "observation_id",
                "segment_id",
                "property",
                "evidence_state",
                "authority",
                "measurement_basis",
                "coordinate_footprint",
                "source",
            ],
            &[
                "value",
                "unit",
                "method",
                "detection_limit",
                "conditions",
                "uncertainty",
                "measurement_system",
            ],
            &path,
        )?;
        let observation_id = string_field(observation, "observation_id", &path)?.to_string();
        if !observation_ids.insert(observation_id) {
            return Err(fail(&path, "duplicate observation_id"));
        }
        let segment_id = string_field(observation, "segment_id", &path)?;
        let segment = segments
            .get(segment_id)
            .ok_or_else(|| fail(&path, "observation references an unknown segment"))?;
        let property = enum_value(
            observation,
            "property",
            &[
                "ic",
                "jc",
                "je",
                "n_value",
                "tc_onset",
                "tc_zero_resistance",
                "tc_bulk_thermodynamic",
                "layer_thickness",
                "texture_metric",
                "defect_density",
                "crack_presence",
                "delamination",
                "phase_fraction",
                "strain_response",
                "ac_loss",
                "joint_resistance",
                "other_declared",
            ],
            &path,
        )?;
        let state = enum_value(
            observation,
            "evidence_state",
            &[
                "measured",
                "not_detected",
                "unmeasured",
                "inferred",
                "disputed",
                "not_applicable",
            ],
            &path,
        )?;
        let authority = enum_value(
            observation,
            "authority",
            &[
                "direct_measurement",
                "derived_measurement",
                "model_inference",
                "literature_reported",
                "synthetic_fixture",
                "unresolved",
            ],
            &path,
        )?;
        let measurement_basis = enum_value(
            observation,
            "measurement_basis",
            &[
                "direct_transport",
                "noncontact_magnetization_proxy",
                "structural_inspection",
                "model_inference",
                "other",
            ],
            &path,
        )?;
        if evidence_class == "synthetic" && authority != "synthetic_fixture" {
            return Err(fail(
                &path,
                "synthetic corpus requires synthetic_fixture authority",
            ));
        }
        *state_counts.entry(state.clone()).or_insert(0) += 1;

        let footprint = field(observation, "coordinate_footprint", &path)?;
        require_keys(
            footprint,
            &[
                "longitudinal_start_mm",
                "longitudinal_end_mm",
                "width_start_mm",
                "width_end_mm",
                "face",
            ],
            &[
                "spatial_resolution_mm",
                "registration_uncertainty_mm",
                "sampling_spacing_mm",
            ],
            &format!("{path}.coordinate_footprint"),
        )?;
        let footprint_interval = Interval {
            start: number_field(
                footprint,
                "longitudinal_start_mm",
                &format!("{path}.coordinate_footprint"),
            )?,
            end: number_field(
                footprint,
                "longitudinal_end_mm",
                &format!("{path}.coordinate_footprint"),
            )?,
        };
        let width_start = number_field(
            footprint,
            "width_start_mm",
            &format!("{path}.coordinate_footprint"),
        )?;
        let width_end = number_field(
            footprint,
            "width_end_mm",
            &format!("{path}.coordinate_footprint"),
        )?;
        let face = enum_value(
            footprint,
            "face",
            &["a", "b", "both", "unknown"],
            &format!("{path}.coordinate_footprint"),
        )?;
        if footprint_interval.start < segment.longitudinal_start_mm
            || footprint_interval.end > segment.longitudinal_end_mm
            || footprint_interval.start >= footprint_interval.end
            || width_start < segment.width_start_mm
            || width_end > segment.width_end_mm
            || width_start >= width_end
            || footprint_interval.start < 0.0
            || footprint_interval.end > reel.total_length_mm
            || width_start < 0.0
            || width_end > reel.width_mm
        {
            return Err(fail(
                &path,
                "observation footprint lies outside its segment or reel",
            ));
        }
        if segment.face != "both"
            && segment.face != "unknown"
            && face != segment.face
            && face != "both"
        {
            return Err(fail(&path, "observation face does not match segment face"));
        }
        if let Some(resolution) =
            object(footprint, &format!("{path}.coordinate_footprint"))?.get("spatial_resolution_mm")
        {
            let value = resolution
                .as_f64()
                .ok_or_else(|| fail(&path, "spatial_resolution_mm must be finite"))?;
            if !value.is_finite() || value <= 0.0 {
                return Err(fail(&path, "spatial_resolution_mm must be positive"));
            }
        }
        if let Some(spacing) =
            object(footprint, &format!("{path}.coordinate_footprint"))?.get("sampling_spacing_mm")
        {
            let value = spacing
                .as_f64()
                .ok_or_else(|| fail(&path, "sampling_spacing_mm must be finite"))?;
            if !value.is_finite() || value <= 0.0 {
                return Err(fail(&path, "sampling_spacing_mm must be positive"));
            }
        }
        if let Some(uncertainty) = object(footprint, &format!("{path}.coordinate_footprint"))?
            .get("registration_uncertainty_mm")
        {
            let value = uncertainty
                .as_f64()
                .ok_or_else(|| fail(&path, "registration_uncertainty_mm must be finite"))?;
            if !value.is_finite() || value < 0.0 {
                return Err(fail(
                    &path,
                    "registration_uncertainty_mm must be non-negative",
                ));
            }
        }

        match state.as_str() {
            "measured" | "inferred" => {
                if !object(observation, &path)?.contains_key("value") {
                    return Err(fail(&path, "measured/inferred observation requires value"));
                }
                validate_interval_value(&observation["value"], &format!("{path}.value"))?;
                string_field(observation, "unit", &path)?;
                string_field(observation, "method", &path)?;
                validate_conditions(
                    field(observation, "conditions", &path)?,
                    &format!("{path}.conditions"),
                )?;
                validate_uncertainty(
                    field(observation, "uncertainty", &path)?,
                    &format!("{path}.uncertainty"),
                )?;
                validate_measurement_system(
                    field(observation, "measurement_system", &path)?,
                    &format!("{path}.measurement_system"),
                )?;
            }
            "not_detected" => {
                if object(observation, &path)?.contains_key("value") {
                    return Err(fail(
                        &path,
                        "not_detected must not carry a fabricated value",
                    ));
                }
                let limit = number_field(observation, "detection_limit", &path)?;
                if limit < 0.0 {
                    return Err(fail(&path, "detection_limit must be non-negative"));
                }
                string_field(observation, "unit", &path)?;
                string_field(observation, "method", &path)?;
                validate_conditions(
                    field(observation, "conditions", &path)?,
                    &format!("{path}.conditions"),
                )?;
                validate_measurement_system(
                    field(observation, "measurement_system", &path)?,
                    &format!("{path}.measurement_system"),
                )?;
            }
            "unmeasured" | "not_applicable" => {
                if object(observation, &path)?.contains_key("value") {
                    return Err(fail(
                        &path,
                        "unmeasured/not_applicable cannot carry a value",
                    ));
                }
            }
            "disputed" => {
                if let Some(value) = object(observation, &path)?.get("value") {
                    validate_interval_value(value, &format!("{path}.value"))?;
                }
            }
            _ => unreachable!(),
        }

        // Missing or invalid calibration never contributes to qualified coverage.
        // Keep the observation, but account for its length separately.
        let fully_width_covered = (width_start - 0.0).abs() <= f64::EPSILON
            && (width_end - reel.width_mm).abs() <= f64::EPSILON;
        if property == "ic" && fully_width_covered {
            match state.as_str() {
                "measured" => {
                    let calibration_valid = object(
                        field(observation, "measurement_system", &path)?,
                        &format!("{path}.measurement_system"),
                    )?
                    .get("calibration_valid_at_measurement")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                    match (measurement_basis.as_str(), calibration_valid) {
                        ("direct_transport", true) => ic_direct_transport.push(footprint_interval),
                        ("direct_transport", false) => {
                            ic_direct_transport_unqualified.push(footprint_interval)
                        }
                        ("noncontact_magnetization_proxy", true) => {
                            ic_calibrated_proxy.push(footprint_interval)
                        }
                        ("noncontact_magnetization_proxy", false) => {
                            ic_proxy_unqualified.push(footprint_interval)
                        }
                        _ => {}
                    }
                }
                "inferred" => ic_inferred.push(footprint_interval),
                "unmeasured" => ic_unmeasured.push(footprint_interval),
                _ => {}
            }
        }

        let source = field(observation, "source", &path)?;
        require_keys(
            source,
            &["artifact_sha256", "rights_status"],
            &[
                "source_uri",
                "source_locator",
                "review_state",
                "acquisition_time",
            ],
            &format!("{path}.source"),
        )?;
        validate_hash_field(source, "artifact_sha256", &format!("{path}.source"))?;
        enum_value(
            source,
            "rights_status",
            &["public", "restricted", "permission_required", "unknown"],
            &format!("{path}.source"),
        )?;
        if object(source, &format!("{path}.source"))?.contains_key("review_state") {
            enum_value(
                source,
                "review_state",
                &["unreviewed", "reviewed", "disputed", "withdrawn"],
                &format!("{path}.source"),
            )?;
        }
        for key in ["source_uri", "source_locator", "acquisition_time"] {
            if let Some(item) = object(source, &format!("{path}.source"))?.get(key)
                && item.as_str().is_none_or(|text| text.trim().is_empty())
            {
                return Err(fail(
                    &path,
                    format!("source.{key} must be a non-empty string"),
                ));
            }
        }
    }

    if let Some(profiles) = object(value, "$")?.get("application_profiles") {
        for (index, profile) in array(profiles, "$.application_profiles")?
            .iter()
            .enumerate()
        {
            let path = format!("$.application_profiles[{index}]");
            require_keys(
                profile,
                &[
                    "profile_id",
                    "intended_use",
                    "temperature_k",
                    "field_magnitude_t",
                    "field_orientation",
                    "current_criterion",
                    "acceptance_rule_id",
                ],
                &["strain_limit", "geometry_convention"],
                &path,
            )?;
            string_field(profile, "profile_id", &path)?;
            enum_value(
                profile,
                "intended_use",
                &[
                    "high_field_magnet",
                    "power_cable",
                    "busbar",
                    "rf_component",
                    "other_declared",
                ],
                &path,
            )?;
            if number_field(profile, "temperature_k", &path)? <= 0.0
                || number_field(profile, "field_magnitude_t", &path)? < 0.0
            {
                return Err(fail(
                    &path,
                    "application profile temperature/field is invalid",
                ));
            }
            string_field(profile, "field_orientation", &path)?;
            string_field(profile, "current_criterion", &path)?;
            string_field(profile, "acceptance_rule_id", &path)?;
            for key in ["geometry_convention"] {
                if object(profile, &path)?.contains_key(key) {
                    string_field(profile, key, &path)?;
                }
            }
            if object(profile, &path)?.contains_key("strain_limit")
                && number_field(profile, "strain_limit", &path)? < 0.0
            {
                return Err(fail(&path, "strain_limit must be non-negative"));
            }
        }
    }
    if let Some(provenance) = object(value, "$")?.get("provenance") {
        require_keys(
            provenance,
            &["created_at", "producer", "rights_status"],
            &["source_uri", "notes"],
            "$.provenance",
        )?;
        string_field(provenance, "created_at", "$.provenance")?;
        string_field(provenance, "producer", "$.provenance")?;
        enum_value(
            provenance,
            "rights_status",
            &["public", "restricted", "permission_required", "unknown"],
            "$.provenance",
        )?;
    }

    let direct_transport_length = interval_union_length(ic_direct_transport);
    let calibrated_proxy_length = interval_union_length(ic_calibrated_proxy);
    let inferred_length = interval_union_length(ic_inferred);
    let direct_transport_unqualified_length =
        interval_union_length(ic_direct_transport_unqualified);
    let proxy_unqualified_length = interval_union_length(ic_proxy_unqualified);
    let explicit_unmeasured_length = interval_union_length(ic_unmeasured);
    let direct_transport_coverage_fraction =
        (direct_transport_length / reel.total_length_mm).clamp(0.0, 1.0);
    let not_directly_measured = (reel.total_length_mm - direct_transport_length).max(0.0);
    Ok(AuditReport {
        reel_id: reel.id,
        segment_count: segments.len(),
        observation_count: observation_values.len(),
        evidence_state_counts: state_counts,
        ic_direct_transport_qualified_length_mm: direct_transport_length,
        ic_calibrated_proxy_length_mm: calibrated_proxy_length,
        ic_inferred_full_width_length_mm: inferred_length,
        ic_direct_transport_unqualified_length_mm: direct_transport_unqualified_length,
        ic_proxy_unqualified_length_mm: proxy_unqualified_length,
        ic_explicitly_unmeasured_full_width_length_mm: explicit_unmeasured_length,
        ic_direct_transport_coverage_fraction: direct_transport_coverage_fraction,
        ic_not_directly_measured_length_mm: not_directly_measured,
        acceptance_disposition: "NOT_EVALUATED_NO_QUALIFIED_APPLICATION_TEST",
    })
}

fn report_value(report: AuditReport) -> Value {
    json!({
        "schema_version": "rebco-long-length-audit-report.v0.1",
        "reel_id": report.reel_id,
        "segment_count": report.segment_count,
        "observation_count": report.observation_count,
        "evidence_state_counts": report.evidence_state_counts,
        "critical_current_coverage": {
            "direct_transport_qualified_length_mm": report.ic_direct_transport_qualified_length_mm,
            "calibrated_noncontact_proxy_length_mm": report.ic_calibrated_proxy_length_mm,
            "inferred_full_width_length_mm": report.ic_inferred_full_width_length_mm,
            "direct_transport_unqualified_length_mm": report.ic_direct_transport_unqualified_length_mm,
            "proxy_unqualified_length_mm": report.ic_proxy_unqualified_length_mm,
            "explicitly_unmeasured_full_width_length_mm": report.ic_explicitly_unmeasured_full_width_length_mm,
            "direct_transport_coverage_fraction": report.ic_direct_transport_coverage_fraction,
            "not_directly_measured_length_mm": report.ic_not_directly_measured_length_mm
        },
        "acceptance_disposition": report.acceptance_disposition,
        "claim_ceiling": CLAIM_CEILING
    })
}

fn run() -> Result<(), String> {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_FIXTURE.to_string());
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read fixture {path}: {error}"))?;
    let value: Value = serde_json::from_str(&source)
        .map_err(|error| format!("invalid JSON in {path}: {error}"))?;
    let report = validate_document(&value)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report_value(report))
            .map_err(|error| format!("cannot serialize report: {error}"))?
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rebco-long-length-auditor: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../docs/research/fixtures/rebco_long_length_synthetic_v0_1.json"
        ))
        .expect("synthetic fixture must parse")
    }

    #[test]
    fn synthetic_fixture_reports_coverage_without_promoting_acceptance() {
        let report = validate_document(&fixture()).expect("fixture should validate");
        assert_eq!(report.segment_count, 3);
        assert_eq!(report.observation_count, 4);
        assert!((report.ic_direct_transport_qualified_length_mm - 400.0).abs() < 1e-9);
        assert!((report.ic_calibrated_proxy_length_mm - 300.0).abs() < 1e-9);
        assert!((report.ic_direct_transport_coverage_fraction - 0.4).abs() < 1e-9);
        assert!((report.ic_explicitly_unmeasured_full_width_length_mm - 300.0).abs() < 1e-9);
        assert!((report.ic_not_directly_measured_length_mm - 600.0).abs() < 1e-9);
        assert_eq!(
            report.acceptance_disposition,
            "NOT_EVALUATED_NO_QUALIFIED_APPLICATION_TEST"
        );
    }

    #[test]
    fn uncalibrated_noncontact_proxy_does_not_count_as_proxy_coverage() {
        let mut value = fixture();
        value["observations"][3]["measurement_system"]["calibration_valid_at_measurement"] =
            json!(false);
        let report = validate_document(&value).expect("record remains inspectable");
        assert_eq!(report.ic_calibrated_proxy_length_mm, 0.0);
        assert_eq!(report.ic_proxy_unqualified_length_mm, 300.0);
    }

    #[test]
    fn noncontact_proxy_never_counts_as_direct_transport_coverage() {
        let mut value = fixture();
        value["observations"][0]["measurement_basis"] = json!("noncontact_magnetization_proxy");
        let report = validate_document(&value).expect("record remains inspectable");
        assert_eq!(report.ic_direct_transport_qualified_length_mm, 0.0);
        assert_eq!(report.ic_calibrated_proxy_length_mm, 700.0);
    }

    #[test]
    fn rejects_nonpositive_sampling_spacing() {
        let mut value = fixture();
        value["observations"][0]["coordinate_footprint"]["sampling_spacing_mm"] = json!(0.0);
        assert!(
            validate_document(&value)
                .unwrap_err()
                .contains("sampling_spacing_mm must be positive")
        );
    }

    #[test]
    fn rejects_segment_outside_reel_bounds() {
        let mut value = fixture();
        value["segments"][0]["longitudinal_end_mm"] = json!(1100.0);
        assert!(
            validate_document(&value)
                .unwrap_err()
                .contains("outside reel bounds")
        );
    }

    #[test]
    fn rejects_spatially_overlapping_segments() {
        let mut value = fixture();
        value["segments"][1]["longitudinal_start_mm"] = json!(350.0);
        assert!(
            validate_document(&value)
                .unwrap_err()
                .contains("overlap spatially")
        );
    }

    #[test]
    fn rejects_observation_with_unknown_segment() {
        let mut value = fixture();
        value["observations"][0]["segment_id"] = json!("UNKNOWN");
        assert!(
            validate_document(&value)
                .unwrap_err()
                .contains("unknown segment")
        );
    }

    #[test]
    fn rejects_unmeasured_record_with_fabricated_value() {
        let mut value = fixture();
        value["observations"][2]["value"] = json!(0.0);
        assert!(
            validate_document(&value)
                .unwrap_err()
                .contains("cannot carry a value")
        );
    }

    #[test]
    fn rejects_measured_record_without_conditions() {
        let mut value = fixture();
        value["observations"][0]
            .as_object_mut()
            .expect("object")
            .remove("conditions");
        assert!(
            validate_document(&value)
                .unwrap_err()
                .contains("missing property conditions")
        );
    }

    #[test]
    fn rejects_not_detected_without_a_detection_limit() {
        let mut value = fixture();
        value["observations"][1]
            .as_object_mut()
            .expect("object")
            .remove("detection_limit");
        assert!(
            validate_document(&value)
                .unwrap_err()
                .contains("missing property detection_limit")
        );
    }

    #[test]
    fn rejects_noncanonical_source_digest() {
        let mut value = fixture();
        value["observations"][0]["source"]["artifact_sha256"] = json!("NOT-A-DIGEST");
        assert!(validate_document(&value).unwrap_err().contains("SHA-256"));
    }

    #[test]
    fn rejects_measurement_footprint_outside_segment() {
        let mut value = fixture();
        value["observations"][2]["coordinate_footprint"]["longitudinal_start_mm"] = json!(600.0);
        assert!(
            validate_document(&value)
                .unwrap_err()
                .contains("outside its segment")
        );
    }

    #[test]
    fn rejects_duplicate_observation_identifiers() {
        let mut value = fixture();
        let first_id = value["observations"][0]["observation_id"].clone();
        value["observations"][1]["observation_id"] = first_id;
        assert!(
            validate_document(&value)
                .unwrap_err()
                .contains("duplicate observation_id")
        );
    }

    #[test]
    fn missing_calibration_status_does_not_count_as_qualified_coverage() {
        let mut value = fixture();
        value["observations"][0]["measurement_system"]
            .as_object_mut()
            .expect("measurement system object")
            .remove("calibration_valid_at_measurement");
        let report = validate_document(&value).expect("record remains inspectable");
        assert_eq!(report.ic_direct_transport_qualified_length_mm, 0.0);
        assert_eq!(report.ic_direct_transport_unqualified_length_mm, 400.0);
    }

    #[test]
    fn invalid_calibration_does_not_count_as_qualified_coverage() {
        let mut value = fixture();
        value["observations"][0]["measurement_system"]["calibration_valid_at_measurement"] =
            json!(false);
        let report = validate_document(&value).expect("record remains inspectable");
        assert_eq!(report.ic_direct_transport_qualified_length_mm, 0.0);
        assert_eq!(report.ic_direct_transport_unqualified_length_mm, 400.0);
    }

    #[test]
    fn partial_width_ic_does_not_count_as_full_width_coverage() {
        let mut value = fixture();
        value["observations"][0]["coordinate_footprint"]["width_end_mm"] = json!(2.0);
        let report = validate_document(&value).expect("partial-width evidence is valid");
        assert_eq!(report.ic_direct_transport_qualified_length_mm, 0.0);
        assert_eq!(report.ic_direct_transport_coverage_fraction, 0.0);
    }

    #[test]
    fn rejects_unknown_fields_in_closed_contract() {
        let mut value = fixture();
        value["unreviewed_shortcut"] = json!(true);
        assert!(
            validate_document(&value)
                .unwrap_err()
                .contains("unknown property")
        );
    }

    #[test]
    fn synthetic_corpus_cannot_claim_direct_measurement_authority() {
        let mut value = fixture();
        value["observations"][0]["authority"] = json!("direct_measurement");
        assert!(
            validate_document(&value)
                .unwrap_err()
                .contains("synthetic_fixture authority")
        );
    }
}
