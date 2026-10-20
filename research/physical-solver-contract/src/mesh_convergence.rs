// SPDX-License-Identifier: AGPL-3.0-or-later
//! Receipt-backed mesh-refinement diagnostics for one normalized quantity of interest.
//!
//! This module verifies each level's request, receipt, mesh bytes, and normalized
//! observation before computing convergence diagnostics. A numerical convergence
//! PASS is not physical validation, model credibility, or engineering-use approval.

use crate::{
    reference_compare::{MAX_NORMALIZED_BYTES, parse_observation, safe_relative_path},
    sha256_hex, verify_solver_output_artifact_bytes,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

type Check<T> = Result<T, String>;
const STUDY_SCHEMA: &str = "physical-mesh-refinement-study-v0.1";
const REPORT_SCHEMA: &str = "physical-mesh-refinement-report-v0.1";
const MAX_LEVELS: usize = 10;
const GCI_SAFETY_FACTOR: f64 = 1.25;

#[derive(Clone)]
struct ReferenceSpec {
    value: f64,
    unit: String,
    method: String,
    identity: String,
    source_ref: String,
}

#[derive(Clone)]
struct LevelSpec {
    level_id: String,
    h_m: f64,
    element_count: u64,
    dof_count: u64,
    mesh_sha256: String,
    mesh_ref: String,
    request_ref: String,
    receipt_ref: String,
    output_id: String,
}

struct StudySpec {
    study_id: String,
    case_id: String,
    observable_id: String,
    unit: String,
    dimension: u64,
    trend_expectation: String,
    refinement_ratio_relative_tolerance: f64,
    expected_order_min: Option<f64>,
    reference: ReferenceSpec,
    max_fine_absolute_error: Option<f64>,
    max_fine_relative_error: Option<f64>,
    levels: Vec<LevelSpec>,
}

struct LevelObservation {
    spec: LevelSpec,
    request_sha256: String,
    scene_sha256: String,
    scene_revision: u64,
    source_revision: String,
    receipt_sha256: String,
    observation_sha256: String,
    adapter: Value,
    value: f64,
    unit: String,
    artifact_verification_status: String,
    producer_status: String,
}

fn object(value: &Value, at: &str) -> Check<&serde_json::Map<String, Value>> {
    value
        .as_object()
        .ok_or_else(|| format!("{at} must be an object"))
}

fn exact_keys(value: &Value, expected: &[&str], at: &str) -> Check<()> {
    let actual = object(value, at)?;
    let expected: BTreeSet<&str> = expected.iter().copied().collect();
    let actual: BTreeSet<&str> = actual.keys().map(String::as_str).collect();
    let missing: Vec<_> = expected.difference(&actual).copied().collect();
    let unknown: Vec<_> = actual.difference(&expected).copied().collect();
    if !missing.is_empty() || !unknown.is_empty() {
        return Err(format!(
            "{at}: missing keys {missing:?}; unknown keys {unknown:?}"
        ));
    }
    Ok(())
}

fn string(value: &Value, key: &str, at: &str) -> Check<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{at}.{key} must be a string"))
}

fn token(value: &Value, key: &str, at: &str) -> Check<String> {
    let value = string(value, key, at)?;
    if value.is_empty()
        || value.len() > 128
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | ':')
        })
    {
        return Err(format!("{at}.{key} is not a valid token"));
    }
    Ok(value)
}

fn finite_number(value: &Value, key: &str, at: &str) -> Check<f64> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .filter(|number| number.is_finite())
        .ok_or_else(|| format!("{at}.{key} must be a finite number"))
}

fn positive_number(value: &Value, key: &str, at: &str) -> Check<f64> {
    let result = finite_number(value, key, at)?;
    if result <= 0.0 {
        return Err(format!("{at}.{key} must be strictly positive"));
    }
    Ok(result)
}

fn non_negative_number(value: &Value, key: &str, at: &str) -> Check<f64> {
    let result = finite_number(value, key, at)?;
    if result < 0.0 {
        return Err(format!("{at}.{key} must be non-negative"));
    }
    Ok(result)
}

fn positive_integer(value: &Value, key: &str, at: &str) -> Check<u64> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .filter(|number| *number > 0)
        .ok_or_else(|| format!("{at}.{key} must be a positive integer"))
}

fn valid_sha256(value: &str, at: &str) -> Check<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("{at} must be a lowercase SHA-256 digest"));
    }
    Ok(())
}

fn optional_non_negative(value: &Value, key: &str, at: &str) -> Check<Option<f64>> {
    match value.get(key) {
        Some(Value::Null) => Ok(None),
        Some(_) => non_negative_number(value, key, at).map(Some),
        None => Err(format!("{at}.{key} is required (number or null)")),
    }
}

fn parse_study(value: &Value) -> Check<StudySpec> {
    exact_keys(
        value,
        &[
            "schema_version",
            "study_id",
            "case_id",
            "observable_id",
            "unit",
            "dimension",
            "trend_expectation",
            "refinement_ratio_relative_tolerance",
            "expected_order_min",
            "reference",
            "acceptance",
            "levels",
        ],
        "study",
    )?;
    if string(value, "schema_version", "study")? != STUDY_SCHEMA {
        return Err(format!("study.schema_version must equal '{STUDY_SCHEMA}'"));
    }
    let study_id = token(value, "study_id", "study")?;
    let case_id = token(value, "case_id", "study")?;
    let observable_id = token(value, "observable_id", "study")?;
    let unit = string(value, "unit", "study")?;
    if unit.trim().is_empty() || unit.len() > 64 {
        return Err("study.unit must be non-empty and at most 64 bytes".to_owned());
    }
    let dimension = value
        .get("dimension")
        .and_then(Value::as_u64)
        .filter(|number| (1..=3).contains(number))
        .ok_or_else(|| "study.dimension must be 1, 2, or 3".to_owned())?;
    let trend_expectation = string(value, "trend_expectation", "study")?;
    if !["monotone_expected", "not_assumed"].contains(&trend_expectation.as_str()) {
        return Err("study.trend_expectation must be monotone_expected or not_assumed".to_owned());
    }
    let refinement_ratio_relative_tolerance =
        positive_number(value, "refinement_ratio_relative_tolerance", "study")?;
    if refinement_ratio_relative_tolerance > 0.5 {
        return Err("study.refinement_ratio_relative_tolerance must not exceed 0.5".to_owned());
    }
    let expected_order_min = match value.get("expected_order_min") {
        Some(Value::Null) => None,
        Some(_) => {
            let order = positive_number(value, "expected_order_min", "study")?;
            if order > 16.0 {
                return Err("study.expected_order_min must not exceed 16".to_owned());
            }
            Some(order)
        }
        None => return Err("study.expected_order_min must be supplied (number or null)".to_owned()),
    };

    let reference = value
        .get("reference")
        .ok_or_else(|| "study.reference is required".to_owned())?;
    exact_keys(
        reference,
        &["value", "unit", "method", "identity", "source_ref"],
        "study.reference",
    )?;
    let reference_value = finite_number(reference, "value", "study.reference")?;
    let reference_unit = string(reference, "unit", "study.reference")?;
    if reference_unit != unit {
        return Err("study.reference.unit must exactly match study.unit".to_owned());
    }
    let reference_method = string(reference, "method", "study.reference")?;
    if !["analytical_exact", "independent_numerical"].contains(&reference_method.as_str()) {
        return Err(
            "study.reference.method must be analytical_exact or independent_numerical".to_owned(),
        );
    }
    let reference_identity = string(reference, "identity", "study.reference")?;
    let reference_source_ref = string(reference, "source_ref", "study.reference")?;
    if reference_identity.trim().is_empty()
        || reference_identity.len() > 512
        || reference_source_ref.trim().is_empty()
        || reference_source_ref.len() > 2048
    {
        return Err("study.reference identity/source_ref is missing or too long".to_owned());
    }

    let acceptance = value
        .get("acceptance")
        .ok_or_else(|| "study.acceptance is required".to_owned())?;
    exact_keys(
        acceptance,
        &["max_fine_absolute_error", "max_fine_relative_error"],
        "study.acceptance",
    )?;
    let max_fine_absolute_error =
        optional_non_negative(acceptance, "max_fine_absolute_error", "study.acceptance")?;
    let max_fine_relative_error =
        optional_non_negative(acceptance, "max_fine_relative_error", "study.acceptance")?;

    let levels = value
        .get("levels")
        .and_then(Value::as_array)
        .ok_or_else(|| "study.levels must be an array".to_owned())?;
    if !(3..=MAX_LEVELS).contains(&levels.len()) {
        return Err(format!(
            "study.levels must contain between 3 and {MAX_LEVELS} levels"
        ));
    }
    let mut parsed_levels = Vec::with_capacity(levels.len());
    let mut level_ids = BTreeSet::new();
    let mut mesh_digests = BTreeSet::new();
    for (index, level) in levels.iter().enumerate() {
        let at = format!("study.levels[{index}]");
        exact_keys(
            level,
            &[
                "level_id",
                "h_m",
                "element_count",
                "dof_count",
                "mesh_sha256",
                "mesh_ref",
                "request_ref",
                "receipt_ref",
                "output_id",
            ],
            &at,
        )?;
        let level_id = token(level, "level_id", &at)?;
        if !level_ids.insert(level_id.clone()) {
            return Err(format!("{at}.level_id is duplicated"));
        }
        let mesh_sha256 = string(level, "mesh_sha256", &at)?;
        valid_sha256(&mesh_sha256, &format!("{at}.mesh_sha256"))?;
        if !mesh_digests.insert(mesh_sha256.clone()) {
            return Err(format!("{at}.mesh_sha256 is duplicated"));
        }
        let parsed = LevelSpec {
            level_id,
            h_m: positive_number(level, "h_m", &at)?,
            element_count: positive_integer(level, "element_count", &at)?,
            dof_count: positive_integer(level, "dof_count", &at)?,
            mesh_sha256,
            mesh_ref: string(level, "mesh_ref", &at)?,
            request_ref: string(level, "request_ref", &at)?,
            receipt_ref: string(level, "receipt_ref", &at)?,
            output_id: token(level, "output_id", &at)?,
        };
        safe_relative_path(&parsed.mesh_ref)?;
        safe_relative_path(&parsed.request_ref)?;
        safe_relative_path(&parsed.receipt_ref)?;
        parsed_levels.push(parsed);
    }
    for index in 1..parsed_levels.len() {
        let previous = &parsed_levels[index - 1];
        let current = &parsed_levels[index];
        if current.h_m >= previous.h_m {
            return Err(
                "study.levels must be ordered coarse-to-fine with strictly decreasing h_m"
                    .to_owned(),
            );
        }
        if current.element_count <= previous.element_count
            || current.dof_count <= previous.dof_count
        {
            return Err(
                "study.levels must increase both element_count and dof_count at every refinement"
                    .to_owned(),
            );
        }
    }

    Ok(StudySpec {
        study_id,
        case_id,
        observable_id,
        unit,
        dimension,
        trend_expectation,
        refinement_ratio_relative_tolerance,
        expected_order_min,
        reference: ReferenceSpec {
            value: reference_value,
            unit: reference_unit,
            method: reference_method,
            identity: reference_identity,
            source_ref: reference_source_ref,
        },
        max_fine_absolute_error,
        max_fine_relative_error,
        levels: parsed_levels,
    })
}

fn read_rooted_bytes(root: &Path, reference: &str, max_bytes: u64) -> Check<(PathBuf, Vec<u8>)> {
    let relative = safe_relative_path(reference)?;
    let path = fs::canonicalize(root.join(relative))
        .map_err(|error| format!("cannot resolve '{reference}': {error}"))?;
    if !path.starts_with(root) {
        return Err(format!("'{reference}' resolves outside the study root"));
    }
    let metadata =
        fs::metadata(&path).map_err(|error| format!("cannot stat '{reference}': {error}"))?;
    if !metadata.is_file() {
        return Err(format!("'{reference}' is not a regular file"));
    }
    if metadata.len() > max_bytes {
        return Err(format!(
            "'{reference}' exceeds the {max_bytes}-byte input budget"
        ));
    }
    let bytes = fs::read(&path).map_err(|error| format!("cannot read '{reference}': {error}"))?;
    Ok((path, bytes))
}

fn rooted_path(root: &Path, reference: &str) -> Check<PathBuf> {
    let relative = safe_relative_path(reference)?;
    let path = fs::canonicalize(root.join(relative))
        .map_err(|error| format!("cannot resolve '{reference}': {error}"))?;
    if !path.starts_with(root) {
        return Err(format!("'{reference}' resolves outside the study root"));
    }
    let metadata =
        fs::metadata(&path).map_err(|error| format!("cannot stat '{reference}': {error}"))?;
    if !metadata.is_file() {
        return Err(format!("'{reference}' is not a regular file"));
    }
    Ok(path)
}

fn load_level(root: &Path, study: &StudySpec, level: &LevelSpec) -> Check<LevelObservation> {
    // The actual mesh must be a receipt-declared input, not merely a file
    // whose hash was copied into the study manifest.
    let (_, request_bytes) = read_rooted_bytes(root, &level.request_ref, MAX_NORMALIZED_BYTES)?;
    let (receipt_path, receipt_bytes) =
        read_rooted_bytes(root, &level.receipt_ref, MAX_NORMALIZED_BYTES)?;
    let request_sha256 = sha256_hex(&request_bytes);
    let request: Value = serde_json::from_slice(&request_bytes)
        .map_err(|error| format!("level '{}': invalid request JSON: {error}", level.level_id))?;
    let request_id = string(&request, "request_id", "request")?;
    let source_revision = string(&request, "source_revision", "request")?;
    let scene_sha256 = string(&request, "scene_sha256", "request")?;
    valid_sha256(&scene_sha256, "request.scene_sha256")?;
    let scene_revision = request
        .get("scene_revision")
        .and_then(Value::as_u64)
        .ok_or_else(|| "request.scene_revision must be a non-negative integer".to_owned())?;

    let artifact_root = root;
    let artifact_report =
        verify_solver_output_artifact_bytes(&request_bytes, &receipt_bytes, artifact_root)?;
    if artifact_report["output_artifact_verification_status"] != "PASS"
        || artifact_report["output_artifact_bytes_verified"] != true
    {
        return Err(format!(
            "level '{}': receipt-declared output artifact verification did not PASS",
            level.level_id
        ));
    }
    if artifact_report["input_artifact_verification_status"] != "PASS"
        || artifact_report["input_artifact_bytes_verified"] != true
    {
        return Err(format!(
            "level '{}': receipt-bound input artifact bytes did not PASS",
            level.level_id
        ));
    }
    if artifact_report["artifact_verification_status"] != "PASS" {
        return Err(format!(
            "level '{}': combined input/output artifact verification did not PASS",
            level.level_id
        ));
    }
    if artifact_report["producer_status"] != "PASS"
        || artifact_report["physical_result_qualified"] != false
    {
        return Err(format!(
            "level '{}': producer status is not PASS or claim ceiling changed",
            level.level_id
        ));
    }

    let receipt: Value = serde_json::from_slice(&receipt_bytes)
        .map_err(|error| format!("level '{}': invalid receipt JSON: {error}", level.level_id))?;
    let convergence = receipt
        .get("convergence")
        .ok_or_else(|| "receipt.convergence is missing".to_owned())?;
    if string(convergence, "state", "receipt.convergence")? != "converged" {
        return Err(format!(
            "level '{}': receipt does not declare solver convergence",
            level.level_id
        ));
    }
    let input_artifacts = receipt
        .get("input_artifacts")
        .and_then(Value::as_array)
        .ok_or_else(|| "receipt.input_artifacts is required for mesh refinement".to_owned())?;
    let mesh_input = input_artifacts
        .iter()
        .find(|input| input.get("input_id").and_then(Value::as_str) == Some("mesh"))
        .ok_or_else(|| {
            format!(
                "level '{}': receipt does not declare input_id 'mesh'",
                level.level_id
            )
        })?;
    if mesh_input.get("kind").and_then(Value::as_str) != Some("finite_element_mesh")
        || mesh_input.get("sha256").and_then(Value::as_str) != Some(level.mesh_sha256.as_str())
        || mesh_input.get("artifact_ref").and_then(Value::as_str) != Some(level.mesh_ref.as_str())
    {
        return Err(format!(
            "level '{}': study mesh identity does not match receipt-declared mesh input",
            level.level_id
        ));
    }
    let verified_mesh = artifact_report["input_artifact_results"]
        .as_array()
        .and_then(|results| {
            results
                .iter()
                .find(|result| result.get("input_id").and_then(Value::as_str) == Some("mesh"))
        })
        .ok_or_else(|| {
            format!(
                "level '{}': mesh input has no byte-verification record",
                level.level_id
            )
        })?;
    if verified_mesh["status"] != "PASS"
        || verified_mesh["observed_sha256"].as_str() != Some(level.mesh_sha256.as_str())
    {
        return Err(format!(
            "level '{}': receipt-declared mesh input lacks a matching byte-verification PASS",
            level.level_id
        ));
    }
    let adapter = receipt
        .get("adapter")
        .ok_or_else(|| "receipt.adapter is missing".to_owned())?
        .clone();
    let outputs = receipt
        .get("outputs")
        .and_then(Value::as_array)
        .ok_or_else(|| "receipt.outputs must be an array".to_owned())?;
    let selected = outputs
        .iter()
        .find(|output| {
            output.get("output_id").and_then(Value::as_str) == Some(level.output_id.as_str())
        })
        .ok_or_else(|| {
            format!(
                "level '{}': output_id '{}' missing from receipt",
                level.level_id, level.output_id
            )
        })?;
    if selected.get("media_type").and_then(Value::as_str) != Some("application/json") {
        return Err(format!(
            "level '{}': normalized observation output must be application/json",
            level.level_id
        ));
    }
    let artifact_ref = string(selected, "artifact_ref", "receipt output")?;
    let declared_output_sha = string(selected, "sha256", "receipt output")?;
    valid_sha256(&declared_output_sha, "receipt output.sha256")?;
    let artifact_path = rooted_path(root, &artifact_ref)?;
    let metadata = fs::metadata(&artifact_path).map_err(|error| error.to_string())?;
    if metadata.len() > MAX_NORMALIZED_BYTES {
        return Err(format!(
            "level '{}': normalized-observable JSON exceeds the {}-byte parser budget",
            level.level_id, MAX_NORMALIZED_BYTES
        ));
    }
    let normalized_bytes = fs::read(&artifact_path).map_err(|error| {
        format!(
            "level '{}': cannot read normalized output: {error}",
            level.level_id
        )
    })?;
    let observation_sha256 = sha256_hex(&normalized_bytes);
    if observation_sha256 != declared_output_sha {
        return Err(format!(
            "level '{}': normalized output changed after artifact-byte verification",
            level.level_id
        ));
    }
    let result_entry = artifact_report["artifact_results"]
        .as_array()
        .and_then(|results| {
            results.iter().find(|result| {
                result.get("output_id").and_then(Value::as_str) == Some(level.output_id.as_str())
            })
        })
        .ok_or_else(|| {
            format!(
                "level '{}': output has no byte-verification receipt entry",
                level.level_id
            )
        })?;
    if result_entry["status"] != "PASS"
        || result_entry["observed_sha256"].as_str() != Some(observation_sha256.as_str())
    {
        return Err(format!(
            "level '{}': selected output lacks a matching PASS byte-verification record",
            level.level_id
        ));
    }

    let normalized: Value = serde_json::from_slice(&normalized_bytes).map_err(|error| {
        format!(
            "level '{}': invalid normalized observation: {error}",
            level.level_id
        )
    })?;
    let observations = parse_observation(
        &normalized,
        &request_sha256,
        &request_id,
        &scene_sha256,
        scene_revision,
        &study.case_id,
    )?;
    let observation = observations.get(&study.observable_id).ok_or_else(|| {
        format!(
            "level '{}': observable '{}' is missing",
            level.level_id, study.observable_id
        )
    })?;
    if observation.unit != study.unit {
        return Err(format!(
            "level '{}': observable unit '{}' does not match study unit '{}'",
            level.level_id, observation.unit, study.unit
        ));
    }
    let receipt_sha256 = sha256_hex(&receipt_bytes);
    let _receipt_path = receipt_path;

    Ok(LevelObservation {
        spec: level.clone(),
        request_sha256,
        scene_sha256,
        scene_revision,
        source_revision,
        receipt_sha256,
        observation_sha256,
        adapter,
        value: observation.value,
        unit: observation.unit.clone(),
        artifact_verification_status: "PASS".to_owned(),
        producer_status: "PASS".to_owned(),
    })
}

fn error_value(observed: f64, reference: f64) -> Check<(f64, Option<f64>, &'static str)> {
    let absolute_error = (observed - reference).abs();
    if !absolute_error.is_finite() {
        return Err("absolute error overflowed finite range".to_owned());
    }
    let (relative_error, status) = if reference == 0.0 {
        (None, "undefined_zero_reference")
    } else {
        let value = absolute_error / reference.abs();
        if value.is_finite() {
            (Some(value), "computed")
        } else {
            (None, "overflow")
        }
    };
    Ok((absolute_error, relative_error, status))
}

/// Verify a pinned multi-level mesh study and return convergence diagnostics.
///
/// The study root must contain each declared mesh, request, receipt, and all
/// receipt-referenced output artifacts. Files are resolved below the canonical
/// root; the mesh is streamed through SHA-256, while structured inputs/observations
/// are size bounded. Each observation's output bytes are checked against the receipt.
pub fn assess_mesh_refinement_study_bytes(study_bytes: &[u8], study_root: &Path) -> Check<Value> {
    let study_value: Value = serde_json::from_slice(study_bytes)
        .map_err(|error| format!("invalid mesh-study manifest JSON: {error}"))?;
    let study = parse_study(&study_value)?;
    let root = fs::canonicalize(study_root)
        .map_err(|error| format!("cannot canonicalize study root: {error}"))?;
    let study_sha256 = sha256_hex(study_bytes);
    let mut levels = Vec::with_capacity(study.levels.len());
    for level in &study.levels {
        match load_level(&root, &study, level) {
            Ok(loaded) => levels.push(loaded),
            Err(error) => {
                return Ok(unresolved_report(
                    &study,
                    &study_sha256,
                    &levels,
                    &format!("{}: {error}", level.level_id),
                ));
            }
        }
    }

    let first = levels
        .first()
        .expect("at least three levels were validated");
    for level in &levels[1..] {
        if level.source_revision != first.source_revision {
            return Ok(unresolved_report(
                &study,
                &study_sha256,
                &levels,
                "source_revision differs across mesh levels",
            ));
        }
        if level.adapter != first.adapter {
            return Ok(unresolved_report(
                &study,
                &study_sha256,
                &levels,
                "adapter/build/configuration identity differs across mesh levels",
            ));
        }
    }

    let mesh_ratios: Vec<f64> = levels
        .windows(2)
        .map(|pair| pair[0].spec.h_m / pair[1].spec.h_m)
        .collect();
    if mesh_ratios
        .iter()
        .any(|ratio| !ratio.is_finite() || *ratio <= 1.1)
    {
        return Ok(unresolved_report(
            &study,
            &study_sha256,
            &levels,
            "one or more mesh refinement ratios are non-finite or not greater than 1.1",
        ));
    }
    let mean_ratio = mesh_ratios
        .iter()
        .map(|ratio| ratio / mesh_ratios.len() as f64)
        .sum::<f64>();
    let ratio_spread = mesh_ratios
        .iter()
        .map(|ratio| (ratio - mean_ratio).abs() / mean_ratio)
        .fold(0.0_f64, f64::max);
    let geometric_refinement = mean_ratio.is_finite()
        && mean_ratio > 1.1
        && ratio_spread <= study.refinement_ratio_relative_tolerance;

    let mut level_reports = Vec::<Value>::with_capacity(levels.len());
    let mut errors = Vec::<f64>::with_capacity(levels.len());
    let mut relative_errors = Vec::<Option<f64>>::with_capacity(levels.len());
    let mut relative_error_statuses = Vec::<&'static str>::with_capacity(levels.len());
    for level in &levels {
        let (absolute_error, relative_error, relative_error_status) =
            error_value(level.value, study.reference.value)?;
        errors.push(absolute_error);
        relative_errors.push(relative_error);
        relative_error_statuses.push(relative_error_status);
        level_reports.push(json!({
            "level_id": level.spec.level_id,
            "h_m": level.spec.h_m,
            "element_count": level.spec.element_count,
            "dof_count": level.spec.dof_count,
            "mesh_sha256": level.spec.mesh_sha256,
            "request_sha256": level.request_sha256,
            "scene_sha256": level.scene_sha256,
            "scene_revision": level.scene_revision,
            "receipt_sha256": level.receipt_sha256,
            "observation_artifact_sha256": level.observation_sha256,
            "observed_value": level.value,
            "unit": level.unit,
            "absolute_error_vs_reference": absolute_error,
            "relative_error_vs_reference": relative_error,
            "relative_error_defined": relative_error.is_some(),
            "relative_error_status": relative_error_status,
            "artifact_verification_status": level.artifact_verification_status,
            "producer_status": level.producer_status
        }));
    }

    let count = levels.len();
    let coarse = &levels[count - 3];
    let medium = &levels[count - 2];
    let fine = &levels[count - 1];
    let coarse_to_medium = medium.value - coarse.value;
    let medium_to_fine = fine.value - medium.value;
    if !coarse_to_medium.is_finite() || !medium_to_fine.is_finite() {
        return Ok(unresolved_report(
            &study,
            &study_sha256,
            &levels,
            "successive solution differences overflowed",
        ));
    }
    let difference_signs_agree = coarse_to_medium != 0.0
        && medium_to_fine != 0.0
        && coarse_to_medium.is_sign_positive() == medium_to_fine.is_sign_positive();
    let differences_contract = coarse_to_medium.abs() > medium_to_fine.abs();

    let mut convergence_status = "UNRESOLVED";
    let mut diagnostics = Vec::<String>::new();
    let mut observed_order: Option<f64> = None;
    let mut extrapolated_value: Option<f64> = None;
    let mut gci_fine_absolute: Option<f64> = None;
    let mut gci_fine_percent: Option<f64> = None;
    let mut gci_applicability = "UNRESOLVED";

    if !geometric_refinement {
        diagnostics.push(format!(
            "refinement ratios are not systematic: ratios={mesh_ratios:?}, relative spread={ratio_spread}"
        ));
    } else if !difference_signs_agree || !differences_contract {
        let reason = if !difference_signs_agree {
            "finest three solutions are equal or change direction (oscillatory/degenerate)"
        } else {
            "successive solution differences do not contract"
        };
        diagnostics.push(reason.to_owned());
        if study.trend_expectation == "monotone_expected" {
            convergence_status = "FAIL";
        }
    } else {
        let local_ratio =
            (coarse.spec.h_m / medium.spec.h_m) / 2.0 + (medium.spec.h_m / fine.spec.h_m) / 2.0;
        let order = (coarse_to_medium.abs() / medium_to_fine.abs()).ln() / local_ratio.ln();
        if !order.is_finite() || order <= 0.0 {
            diagnostics.push("observed convergence order is not finite and positive".to_owned());
            if study.trend_expectation == "monotone_expected" {
                convergence_status = "FAIL";
            }
        } else {
            observed_order = Some(order);
            let denominator = local_ratio.powf(order) - 1.0;
            if !denominator.is_finite() || denominator <= 0.0 {
                diagnostics.push("Richardson/GCI denominator is invalid".to_owned());
                convergence_status = "UNRESOLVED";
                observed_order = None;
            } else {
                let extrapolated = fine.value + (fine.value - medium.value) / denominator;
                let gci_abs = GCI_SAFETY_FACTOR * (fine.value - medium.value).abs() / denominator;
                if !extrapolated.is_finite() || !gci_abs.is_finite() {
                    diagnostics.push(
                        "Richardson extrapolation or GCI-like estimate overflowed".to_owned(),
                    );
                    convergence_status = "UNRESOLVED";
                    observed_order = None;
                } else {
                    extrapolated_value = Some(extrapolated);
                    gci_fine_absolute = Some(gci_abs);
                    if fine.value != 0.0 {
                        let percent = gci_abs / fine.value.abs() * 100.0;
                        if percent.is_finite() {
                            gci_fine_percent = Some(percent);
                        }
                    }
                    convergence_status = "PASS";
                    gci_applicability = "CONDITIONAL_MONOTONE_GEOMETRIC_REFINEMENT";
                    if let Some(min_order) = study.expected_order_min {
                        if order < min_order {
                            diagnostics.push(format!(
                                "observed order {order} is below expected_order_min {min_order}"
                            ));
                            convergence_status = "FAIL";
                        }
                    }
                }
            }
        }
    }

    let fine_error = *errors.last().expect("at least three levels");
    let fine_relative_error = *relative_errors.last().expect("at least three levels");
    let fine_relative_error_status = *relative_error_statuses
        .last()
        .expect("at least three levels");
    let accuracy_status = match (study.max_fine_absolute_error, study.max_fine_relative_error) {
        (None, None) => "NOT_SPECIFIED",
        (Some(max_abs), Some(max_rel)) => {
            if fine_error > max_abs {
                "FAIL"
            } else if fine_relative_error.is_none() {
                "UNRESOLVED"
            } else if fine_relative_error.is_some_and(|error| error <= max_rel) {
                "PASS"
            } else {
                "FAIL"
            }
        }
        (Some(max_abs), None) => {
            if fine_error <= max_abs {
                "PASS"
            } else {
                "FAIL"
            }
        }
        (None, Some(max_rel)) => {
            if fine_relative_error.is_none() {
                "UNRESOLVED"
            } else if fine_relative_error.is_some_and(|error| error <= max_rel) {
                "PASS"
            } else {
                "FAIL"
            }
        }
    };
    if accuracy_status == "FAIL" {
        diagnostics
            .push("finest-mesh error exceeds the preregistered accuracy acceptance".to_owned());
    }
    if accuracy_status == "UNRESOLVED" {
        diagnostics.push(
            "relative-error acceptance is undefined or cannot be computed reliably".to_owned(),
        );
    }
    let study_status = if convergence_status == "FAIL" || accuracy_status == "FAIL" {
        "FAIL"
    } else if convergence_status != "PASS"
        || accuracy_status == "UNRESOLVED"
        || accuracy_status == "NOT_SPECIFIED"
    {
        "UNRESOLVED"
    } else {
        "PASS"
    };

    Ok(json!({
        "schema_version": REPORT_SCHEMA,
        "study_status": study_status,
        "mesh_convergence_status": convergence_status,
        "fine_mesh_accuracy_status": accuracy_status,
        "study_id": study.study_id,
        "case_id": study.case_id,
        "observable_id": study.observable_id,
        "unit": study.unit,
        "dimension": study.dimension,
        "study_manifest_sha256": study_sha256,
        "reference": {
            "value": study.reference.value,
            "unit": study.reference.unit,
            "method": study.reference.method,
            "identity": study.reference.identity,
            "source_ref": study.reference.source_ref
        },
        "trend_expectation": study.trend_expectation,
        "expected_order_min": study.expected_order_min,
        "mesh_level_count": levels.len(),
        "mesh_refinement_ratios": mesh_ratios,
        "refinement_ratio_relative_spread": ratio_spread,
        "geometric_refinement_status": if geometric_refinement { "PASS" } else { "UNRESOLVED" },
        "observed_order_finest_three": observed_order,
        "richardson_extrapolated_value": extrapolated_value,
        "gci_safety_factor": GCI_SAFETY_FACTOR,
        "gci_fine_absolute": gci_fine_absolute,
        "gci_fine_percent": gci_fine_percent,
        "gci_applicability_status": gci_applicability,
        "fine_mesh_absolute_error": fine_error,
        "fine_mesh_relative_error": fine_relative_error,
        "fine_mesh_relative_error_defined": fine_relative_error.is_some(),
        "fine_mesh_relative_error_status": fine_relative_error_status,
        "maximum_fine_absolute_error": study.max_fine_absolute_error,
        "maximum_fine_relative_error": study.max_fine_relative_error,
        "reference_error_decreases_monotonically": errors.windows(2).all(|pair| pair[1] < pair[0]),
        "all_level_receipt_and_output_bytes_verified": true,
        "solver_build_identity_constant_across_levels": true,
        "physical_result_qualified": false,
        "real_world_validation_status": "UNRESOLVED",
        "diagnostics": diagnostics,
        "levels": level_reports,
        "claim_ceiling": "verified request/receipt/mesh/normalized-output byte lineage and numerical mesh-series diagnostics for this named quantity of interest; conditional Richardson/GCI indicator only; no physical validation, universal convergence proof, model credibility, engineering safety, or intended-use qualification"
    }))
}

fn unresolved_report(
    study: &StudySpec,
    study_sha256: &str,
    loaded: &[LevelObservation],
    diagnostic: &str,
) -> Value {
    json!({
        "schema_version": REPORT_SCHEMA,
        "study_status": "UNRESOLVED",
        "mesh_convergence_status": "UNRESOLVED",
        "fine_mesh_accuracy_status": "UNRESOLVED",
        "study_id": study.study_id,
        "case_id": study.case_id,
        "observable_id": study.observable_id,
        "unit": study.unit,
        "dimension": study.dimension,
        "study_manifest_sha256": study_sha256,
        "reference": {
            "value": study.reference.value,
            "unit": study.reference.unit,
            "method": study.reference.method,
            "identity": study.reference.identity,
            "source_ref": study.reference.source_ref
        },
        "mesh_level_count": study.levels.len(),
        "verified_level_count": loaded.len(),
        "all_level_receipt_and_output_bytes_verified": loaded.len() == study.levels.len(),
        "physical_result_qualified": false,
        "real_world_validation_status": "UNRESOLVED",
        "diagnostics": [diagnostic],
        "levels": loaded.iter().map(|level| json!({
            "level_id": level.spec.level_id,
            "h_m": level.spec.h_m,
            "mesh_sha256": level.spec.mesh_sha256,
            "request_sha256": level.request_sha256,
            "scene_sha256": level.scene_sha256,
            "scene_revision": level.scene_revision,
            "receipt_sha256": level.receipt_sha256,
            "observation_artifact_sha256": level.observation_sha256,
            "observed_value": level.value,
            "unit": level.unit,
            "artifact_verification_status": level.artifact_verification_status,
            "producer_status": level.producer_status
        })).collect::<Vec<_>>(),
        "claim_ceiling": "mesh study incomplete or evidence prerequisites unresolved; no convergence, physical validation, model credibility, safety, or engineering-use claim"
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_root() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "physical-mesh-study-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&root).expect("temporary root");
        root
    }

    fn write_level(
        root: &Path,
        index: usize,
        h_m: f64,
        dof_count: u64,
        element_count: u64,
        observed_value: f64,
    ) -> Value {
        let level_id = ["coarse", "medium", "fine", "very-fine"][index];
        let directory = root.join("levels").join(level_id);
        fs::create_dir_all(&directory).unwrap();
        let mut request: Value = serde_json::from_slice(
            &fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(
                "../../schemas/physical-transformation/fixtures/analytical-bar-request-v0.1.json",
            ))
            .unwrap(),
        )
        .unwrap();
        request["request_id"] = json!(format!("mesh-{level_id}"));
        request["scene_revision"] = json!(index as u64 + 1);
        request["scene_sha256"] = json!(format!("{:0>64x}", index + 1));
        let request_bytes = serde_json::to_vec(&request).unwrap();
        let request_sha = sha256_hex(&request_bytes);
        let source_revision = request["source_revision"].as_str().unwrap();

        let mesh_bytes = format!("synthetic finite-element mesh bytes for level {level_id}");
        fs::write(directory.join("mesh.inp"), &mesh_bytes).unwrap();
        let mesh_sha = sha256_hex(mesh_bytes.as_bytes());

        let observation = json!({
            "schema_version":"physical-normalized-observables-v0.1",
            "request_sha256":request_sha,
            "request_id":request["request_id"],
            "scene_sha256":request["scene_sha256"],
            "scene_revision":request["scene_revision"],
            "case_id":"linear-bar-test",
            "observables":[{"observable_id":"end_displacement_m","value":observed_value,"unit":"m"}]
        });
        let normalized_bytes = serde_json::to_vec(&observation).unwrap();
        let normalized_sha = sha256_hex(&normalized_bytes);
        fs::write(directory.join("normalized.json"), &normalized_bytes).unwrap();
        let aux = br#"{"synthetic_auxiliary":true}"#;
        fs::write(directory.join("stress.json"), aux).unwrap();
        let receipt = json!({
            "schema_version":"physical-solver-receipt-v0.1",
            "request_id":request["request_id"],
            "request_sha256":request_sha,
            "source_revision":source_revision,
            "adapter":{
                "adapter_id":"fixture.ccx",
                "version":"0.1",
                "executable_sha256":"c".repeat(64),
                "configuration_sha256":"d".repeat(64),
                "solver_name":"synthetic-only",
                "solver_version":"0.1",
                "solver_build_ref":"fixture-only:no-solver-executed"
            },
            "input_artifacts":[{
                "input_id":"mesh",
                "kind":"finite_element_mesh",
                "sha256":mesh_sha,
                "media_type":"text/plain",
                "artifact_ref":format!("levels/{level_id}/mesh.inp")
            }],
            "status":"PASS",
            "evidence_class":"synthetic",
            "physics_evaluated":["static_linear_elastic"],
            "physics_omitted":[],
            "convergence":{
                "state":"converged",
                "iterations":1,
                "residual":0.0,
                "mesh_refinement_evaluated":false,
                "details":"Synthetic unit-test receipt; no solver invoked."
            },
            "outputs":[
                {"output_id":"normalized","kind":"displacement","sha256":normalized_sha,"media_type":"application/json","unit":"m","artifact_ref":format!("levels/{level_id}/normalized.json")},
                {"output_id":"aux-stress","kind":"stress","sha256":sha256_hex(aux),"media_type":"application/json","unit":"Pa","artifact_ref":format!("levels/{level_id}/stress.json")}
            ],
            "uncertainty":{"state":"unknown","method":null,"summary":"synthetic test"},
            "validity_envelope":{"supported_geometry":"synthetic","supported_material_model":"synthetic","load_regime":"synthetic","limitations":["no physical result"]},
            "claim_ceiling":"synthetic fixture only",
            "diagnostics":[]
        });
        fs::write(directory.join("request.json"), &request_bytes).unwrap();
        fs::write(
            directory.join("receipt.json"),
            serde_json::to_vec(&receipt).unwrap(),
        )
        .unwrap();

        json!({
            "level_id":level_id,
            "h_m":h_m,
            "element_count":element_count,
            "dof_count":dof_count,
            "mesh_sha256":mesh_sha,
            "mesh_ref":format!("levels/{level_id}/mesh.inp"),
            "request_ref":format!("levels/{level_id}/request.json"),
            "receipt_ref":format!("levels/{level_id}/receipt.json"),
            "output_id":"normalized"
        })
    }

    fn study(root: &Path, values: [f64; 3]) -> Vec<u8> {
        let levels = vec![
            write_level(root, 0, 1.0, 10, 2, values[0]),
            write_level(root, 1, 0.5, 40, 16, values[1]),
            write_level(root, 2, 0.25, 160, 128, values[2]),
        ];
        serde_json::to_vec(&json!({
            "schema_version":STUDY_SCHEMA,
            "study_id":"linear-bar-mesh-study",
            "case_id":"linear-bar-test",
            "observable_id":"end_displacement_m",
            "unit":"m",
            "dimension":1,
            "trend_expectation":"monotone_expected",
            "refinement_ratio_relative_tolerance":0.05,
            "expected_order_min":1.5,
            "reference":{
                "value":0.5,
                "unit":"m",
                "method":"analytical_exact",
                "identity":"closed-form fixture reference",
                "source_ref":"test-oracle://linear-bar"
            },
            "acceptance":{
                "max_fine_absolute_error":0.1,
                "max_fine_relative_error":0.2
            },
            "levels":levels
        }))
        .unwrap()
    }

    #[test]
    fn verifies_all_levels_and_recovers_second_order_convergence() {
        let root = temporary_root();
        let bytes = study(&root, [1.5, 0.75, 0.5625]);
        let report = assess_mesh_refinement_study_bytes(&bytes, &root).unwrap();
        assert_eq!(report["mesh_convergence_status"], "PASS");
        assert_eq!(report["fine_mesh_accuracy_status"], "PASS");
        assert_eq!(report["all_level_receipt_and_output_bytes_verified"], true);
        assert_eq!(report["physical_result_qualified"], false);
        assert!((report["observed_order_finest_three"].as_f64().unwrap() - 2.0).abs() < 1e-10);
        assert!((report["richardson_extrapolated_value"].as_f64().unwrap() - 0.5).abs() < 1e-10);
        assert_eq!(report["gci_safety_factor"], 1.25);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tampered_mesh_bytes_cannot_enter_convergence_analysis() {
        let root = temporary_root();
        let bytes = study(&root, [1.5, 0.75, 0.5625]);
        fs::write(root.join("levels/coarse/mesh.inp"), "tampered mesh bytes").unwrap();
        let report = assess_mesh_refinement_study_bytes(&bytes, &root).unwrap();
        assert_eq!(report["mesh_convergence_status"], "UNRESOLVED");
        assert_eq!(report["all_level_receipt_and_output_bytes_verified"], false);
        assert!(
            report["diagnostics"][0]
                .as_str()
                .unwrap()
                .contains("input artifact bytes")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unit_mismatch_is_rejected_after_byte_verification() {
        let root = temporary_root();
        let bytes = study(&root, [1.5, 0.75, 0.5625]);
        let output_path = root.join("levels/medium/normalized.json");
        let normalized: Value = serde_json::from_slice(&fs::read(&output_path).unwrap()).unwrap();
        let mut tampered = normalized;
        tampered["observables"][0]["unit"] = json!("mm");
        let normalized_bytes = serde_json::to_vec(&tampered).unwrap();
        fs::write(&output_path, &normalized_bytes).unwrap();

        let receipt_path = root.join("levels/medium/receipt.json");
        let mut receipt: Value = serde_json::from_slice(&fs::read(&receipt_path).unwrap()).unwrap();
        receipt["outputs"][0]["sha256"] = json!(sha256_hex(&normalized_bytes));
        fs::write(&receipt_path, serde_json::to_vec(&receipt).unwrap()).unwrap();

        let report = assess_mesh_refinement_study_bytes(&bytes, &root).unwrap();
        assert_eq!(report["mesh_convergence_status"], "UNRESOLVED");
        assert!(report["diagnostics"][0].as_str().unwrap().contains("unit"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn non_uniform_refinement_ratios_withhold_order_and_gci() {
        let root = temporary_root();
        let bytes = study(&root, [1.5, 0.75, 0.5625]);
        let mut manifest: Value = serde_json::from_slice(&bytes).unwrap();
        manifest["levels"][1]["h_m"] = json!(0.4);
        let report =
            assess_mesh_refinement_study_bytes(&serde_json::to_vec(&manifest).unwrap(), &root)
                .unwrap();
        assert_eq!(report["mesh_convergence_status"], "UNRESOLVED");
        assert_eq!(report["observed_order_finest_three"], Value::Null);
        assert_eq!(report["gci_fine_absolute"], Value::Null);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn oscillatory_series_is_unresolved_when_monotonicity_was_not_assumed() {
        let root = temporary_root();
        let bytes = study(&root, [1.5, 0.45, 0.7]);
        let mut manifest: Value = serde_json::from_slice(&bytes).unwrap();
        manifest["trend_expectation"] = json!("not_assumed");
        let report =
            assess_mesh_refinement_study_bytes(&serde_json::to_vec(&manifest).unwrap(), &root)
                .unwrap();
        assert_eq!(report["mesh_convergence_status"], "UNRESOLVED");
        assert_eq!(report["observed_order_finest_three"], Value::Null);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_receipt_mesh_input_declaration_is_unresolved() {
        let root = temporary_root();
        let bytes = study(&root, [1.5, 0.75, 0.5625]);
        let receipt_path = root.join("levels/coarse/receipt.json");
        let mut receipt: Value = serde_json::from_slice(&fs::read(&receipt_path).unwrap()).unwrap();
        receipt.as_object_mut().unwrap().remove("input_artifacts");
        fs::write(&receipt_path, serde_json::to_vec(&receipt).unwrap()).unwrap();
        let report = assess_mesh_refinement_study_bytes(&bytes, &root).unwrap();
        assert_eq!(report["mesh_convergence_status"], "UNRESOLVED");
        assert!(
            report["diagnostics"][0]
                .as_str()
                .unwrap()
                .contains("input artifact bytes")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn receipt_cannot_bind_the_study_to_a_different_mesh_digest() {
        let root = temporary_root();
        let bytes = study(&root, [1.5, 0.75, 0.5625]);
        let receipt_path = root.join("levels/medium/receipt.json");
        let mut receipt: Value = serde_json::from_slice(&fs::read(&receipt_path).unwrap()).unwrap();
        receipt["input_artifacts"][0]["sha256"] = json!("f".repeat(64));
        fs::write(&receipt_path, serde_json::to_vec(&receipt).unwrap()).unwrap();
        let report = assess_mesh_refinement_study_bytes(&bytes, &root).unwrap();
        assert_eq!(report["mesh_convergence_status"], "UNRESOLVED");
        assert!(
            report["diagnostics"][0]
                .as_str()
                .unwrap()
                .contains("input artifact bytes")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn tampered_normalized_bytes_are_rejected_even_if_manifest_is_unchanged() {
        let root = temporary_root();
        let bytes = study(&root, [1.5, 0.75, 0.5625]);
        let path = root.join("levels/fine/normalized.json");
        fs::write(&path, b"not the receipt-declared bytes").unwrap();
        let report = assess_mesh_refinement_study_bytes(&bytes, &root).unwrap();
        assert_eq!(report["mesh_convergence_status"], "UNRESOLVED");
        assert!(
            report["diagnostics"][0]
                .as_str()
                .unwrap()
                .contains("artifact verification")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn oscillatory_series_is_not_given_a_false_convergence_order() {
        let root = temporary_root();
        let bytes = study(&root, [1.5, 0.45, 0.7]);
        let report = assess_mesh_refinement_study_bytes(&bytes, &root).unwrap();
        assert_eq!(report["mesh_convergence_status"], "FAIL");
        assert_eq!(report["observed_order_finest_three"], Value::Null);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn relative_error_is_undefined_for_zero_reference_but_absolute_error_remains() {
        let root = temporary_root();
        let bytes = study(&root, [1.0, 0.25, 0.0625]);
        let mut value: Value = serde_json::from_slice(&bytes).unwrap();
        value["reference"]["value"] = json!(0.0);
        value["acceptance"]["max_fine_absolute_error"] = json!(0.1);
        value["acceptance"]["max_fine_relative_error"] = Value::Null;
        let report =
            assess_mesh_refinement_study_bytes(&serde_json::to_vec(&value).unwrap(), &root)
                .unwrap();
        assert_eq!(report["mesh_convergence_status"], "PASS");
        assert_eq!(report["fine_mesh_relative_error_defined"], false);
        assert_eq!(report["fine_mesh_relative_error"], Value::Null);
        assert_eq!(report["fine_mesh_accuracy_status"], "PASS");

        value["acceptance"]["max_fine_relative_error"] = json!(0.2);
        let report =
            assess_mesh_refinement_study_bytes(&serde_json::to_vec(&value).unwrap(), &root)
                .unwrap();
        assert_eq!(report["fine_mesh_accuracy_status"], "UNRESOLVED");
        fs::remove_dir_all(root).unwrap();
    }
}
