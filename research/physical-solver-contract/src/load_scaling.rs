// SPDX-License-Identifier: AGPL-3.0-or-later
//! Receipt-bound load-scaling metamorphic relation for linear static elasticity.
//! This is numerical code verification, not physical validation.

use crate::{
    reference_compare::{
        MAX_NORMALIZED_BYTES, ObservedObservable, parse_observation, safe_relative_path,
    },
    sha256_hex, verify_solver_output_artifact_bytes,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

type Check<T> = Result<T, String>;
const MANIFEST_SCHEMA: &str = "physical-load-scaling-relation-v0.1";

#[derive(Clone)]
struct OutputPair {
    kind: String,
    baseline_id: String,
    scaled_id: String,
}

struct RelationManifest {
    relation_id: String,
    case_id: String,
    baseline_request_sha256: String,
    scaled_request_sha256: String,
    scale_factor: f64,
    absolute_tolerance: f64,
    relative_tolerance: f64,
    output_pairs: Vec<OutputPair>,
}

struct BoundOutput {
    bytes: Vec<u8>,
    output_sha256: String,
    declared_unit: String,
    verification_report: Value,
    producer_status: String,
}

/// Score a positive load-scaling relation between two receipt-bound linear-static runs.
///
/// Preconditions are checked from the exact requests. Only small-strain static linear
/// elasticity with point-force loads is supported. Every non-load request input must
/// match, each scaled force must equal alpha times its baseline value, and every
/// requested output kind must be present in the relation manifest and compared.
pub fn assess_load_scaling_relation_bytes(
    baseline_request_bytes: &[u8],
    baseline_receipt_bytes: &[u8],
    scaled_request_bytes: &[u8],
    scaled_receipt_bytes: &[u8],
    artifact_root: &Path,
    manifest_bytes: &[u8],
) -> Check<Value> {
    let manifest_sha256 = sha256_hex(manifest_bytes);
    let baseline_request_sha256 = sha256_hex(baseline_request_bytes);
    let scaled_request_sha256 = sha256_hex(scaled_request_bytes);
    let manifest = parse_manifest(manifest_bytes)?;
    let mut report = base_report(
        &manifest,
        &manifest_sha256,
        &baseline_request_sha256,
        &scaled_request_sha256,
    );

    if manifest.baseline_request_sha256 != baseline_request_sha256
        || manifest.scaled_request_sha256 != scaled_request_sha256
    {
        return Ok(unresolved(
            report,
            "relation manifest does not bind the exact baseline and scaled request bytes",
        ));
    }

    let baseline_request: Value = match serde_json::from_slice(baseline_request_bytes) {
        Ok(value) => value,
        Err(error) => {
            return Ok(unresolved(
                report,
                format!("baseline request JSON is invalid: {error}"),
            ));
        }
    };
    let scaled_request: Value = match serde_json::from_slice(scaled_request_bytes) {
        Ok(value) => value,
        Err(error) => {
            return Ok(unresolved(
                report,
                format!("scaled request JSON is invalid: {error}"),
            ));
        }
    };
    if let Err(reason) = validate_request_relation(&baseline_request, &scaled_request, &manifest) {
        return Ok(unresolved(report, reason));
    }

    let paired_kinds: BTreeSet<&str> = manifest
        .output_pairs
        .iter()
        .map(|pair| pair.kind.as_str())
        .collect();
    let requested_kinds: BTreeSet<&str> = req_array(&baseline_request, "required_outputs")?
        .iter()
        .map(|value| value.as_str().unwrap_or(""))
        .collect();
    if paired_kinds != requested_kinds {
        return Ok(unresolved(
            report,
            format!(
                "output pairs must cover the exact required output inventory; requested={requested_kinds:?}, paired={paired_kinds:?}"
            ),
        ));
    }

    let baseline_verification = match verify_solver_output_artifact_bytes(
        baseline_request_bytes,
        baseline_receipt_bytes,
        artifact_root,
    ) {
        Ok(value) => value,
        Err(error) => {
            return Ok(unresolved(
                report,
                format!("baseline receipt/artifact verification failed: {error}"),
            ));
        }
    };
    let scaled_verification = match verify_solver_output_artifact_bytes(
        scaled_request_bytes,
        scaled_receipt_bytes,
        artifact_root,
    ) {
        Ok(value) => value,
        Err(error) => {
            return Ok(unresolved(
                report,
                format!("scaled receipt/artifact verification failed: {error}"),
            ));
        }
    };
    report["baseline_artifact_verification_status"] = baseline_verification
        .get("artifact_verification_status")
        .cloned()
        .unwrap_or(Value::Null);
    report["scaled_artifact_verification_status"] = scaled_verification
        .get("artifact_verification_status")
        .cloned()
        .unwrap_or(Value::Null);
    report["baseline_output_artifact_bytes_verified"] = baseline_verification
        .get("output_artifact_bytes_verified")
        .cloned()
        .unwrap_or(Value::Bool(false));
    report["scaled_output_artifact_bytes_verified"] = scaled_verification
        .get("output_artifact_bytes_verified")
        .cloned()
        .unwrap_or(Value::Bool(false));
    report["baseline_producer_status"] = baseline_verification
        .get("producer_status")
        .cloned()
        .unwrap_or(Value::Null);
    report["scaled_producer_status"] = scaled_verification
        .get("producer_status")
        .cloned()
        .unwrap_or(Value::Null);
    if baseline_verification["artifact_verification_status"] != "PASS"
        || scaled_verification["artifact_verification_status"] != "PASS"
        || baseline_verification["output_artifact_bytes_verified"] != true
        || scaled_verification["output_artifact_bytes_verified"] != true
    {
        return Ok(unresolved(
            report,
            "both runs must pass receipt-bound output-byte verification",
        ));
    }
    if baseline_verification["producer_status"] != "PASS"
        || scaled_verification["producer_status"] != "PASS"
    {
        return Ok(unresolved(
            report,
            "both producer receipts must be PASS before scoring a relation",
        ));
    }

    let baseline_receipt: Value = match serde_json::from_slice(baseline_receipt_bytes) {
        Ok(value) => value,
        Err(error) => {
            return Ok(unresolved(
                report,
                format!("baseline receipt JSON is invalid: {error}"),
            ));
        }
    };
    let scaled_receipt: Value = match serde_json::from_slice(scaled_receipt_bytes) {
        Ok(value) => value,
        Err(error) => {
            return Ok(unresolved(
                report,
                format!("scaled receipt JSON is invalid: {error}"),
            ));
        }
    };

    let mut output_results = Vec::with_capacity(manifest.output_pairs.len());
    let mut violation_count = 0usize;
    let mut unresolved_count = 0usize;
    let mut total_observables = 0usize;
    let mut diagnostics = Vec::new();

    for pair in &manifest.output_pairs {
        let baseline = match bind_output(
            artifact_root,
            &baseline_receipt,
            &baseline_verification,
            &pair.baseline_id,
            &pair.kind,
        ) {
            Ok(value) => value,
            Err(reason) => {
                return Ok(unresolved(
                    report,
                    format!("baseline output '{}': {reason}", pair.baseline_id),
                ));
            }
        };
        let scaled = match bind_output(
            artifact_root,
            &scaled_receipt,
            &scaled_verification,
            &pair.scaled_id,
            &pair.kind,
        ) {
            Ok(value) => value,
            Err(reason) => {
                return Ok(unresolved(
                    report,
                    format!("scaled output '{}': {reason}", pair.scaled_id),
                ));
            }
        };
        let baseline_value: Value = match serde_json::from_slice(&baseline.bytes) {
            Ok(value) => value,
            Err(error) => {
                return Ok(unresolved(
                    report,
                    format!("baseline normalized output JSON is invalid: {error}"),
                ));
            }
        };
        let scaled_value: Value = match serde_json::from_slice(&scaled.bytes) {
            Ok(value) => value,
            Err(error) => {
                return Ok(unresolved(
                    report,
                    format!("scaled normalized output JSON is invalid: {error}"),
                ));
            }
        };
        let baseline_observables = match parse_observation(
            &baseline_value,
            &baseline_request_sha256,
            req_str(&baseline_request, "request_id")?,
            req_str(&baseline_request, "scene_sha256")?,
            req_u64(&baseline_request, "scene_revision")?,
            &manifest.case_id,
        ) {
            Ok(value) => value,
            Err(error) => {
                return Ok(unresolved(
                    report,
                    format!("baseline normalized output is not bound/valid: {error}"),
                ));
            }
        };
        let scaled_observables = match parse_observation(
            &scaled_value,
            &scaled_request_sha256,
            req_str(&scaled_request, "request_id")?,
            req_str(&scaled_request, "scene_sha256")?,
            req_u64(&scaled_request, "scene_revision")?,
            &manifest.case_id,
        ) {
            Ok(value) => value,
            Err(error) => {
                return Ok(unresolved(
                    report,
                    format!("scaled normalized output is not bound/valid: {error}"),
                ));
            }
        };
        if baseline_observables
            .values()
            .any(|observable| observable.unit != baseline.declared_unit)
        {
            return Ok(unresolved(
                report,
                format!(
                    "baseline normalized units disagree with receipt-declared unit for output kind '{}'",
                    pair.kind
                ),
            ));
        }
        if scaled_observables
            .values()
            .any(|observable| observable.unit != scaled.declared_unit)
        {
            return Ok(unresolved(
                report,
                format!(
                    "scaled normalized units disagree with receipt-declared unit for output kind '{}'",
                    pair.kind
                ),
            ));
        }
        if baseline_observables.keys().collect::<BTreeSet<_>>()
            != scaled_observables.keys().collect::<BTreeSet<_>>()
        {
            return Ok(unresolved(
                report,
                format!(
                    "observable inventory differs for output kind '{}'",
                    pair.kind
                ),
            ));
        }
        let (results, violations, unresolved, count) = compare_observables(
            &baseline_observables,
            &scaled_observables,
            manifest.scale_factor,
            manifest.absolute_tolerance,
            manifest.relative_tolerance,
        );
        violation_count += violations;
        unresolved_count += unresolved;
        total_observables += count;
        if violations > 0 {
            diagnostics.push(format!(
                "output kind '{}' has {violations} observable(s) outside the declared load-scaling tolerance",
                pair.kind
            ));
        }
        output_results.push(json!({
            "output_kind": pair.kind,
            "baseline_output_id": pair.baseline_id,
            "scaled_output_id": pair.scaled_id,
            "baseline_output_sha256": baseline.output_sha256,
            "scaled_output_sha256": scaled.output_sha256,
            "baseline_observable_count": baseline_observables.len(),
            "scaled_observable_count": scaled_observables.len(),
            "baseline_artifact_verification_status": baseline.verification_report["artifact_verification_status"],
            "scaled_artifact_verification_status": scaled.verification_report["artifact_verification_status"],
            "baseline_producer_status": baseline.producer_status,
            "scaled_producer_status": scaled.producer_status,
            "observable_results": results
        }));
    }

    let status = if unresolved_count > 0 {
        "UNRESOLVED"
    } else if violation_count == 0 {
        "PASS"
    } else {
        "FAIL"
    };
    report["relation_status"] = json!(status);
    report["output_results"] = json!(output_results);
    report["observable_count"] = json!(total_observables);
    report["violation_count"] = json!(violation_count);
    report["unresolved_observable_count"] = json!(unresolved_count);
    report["diagnostics"] = json!(diagnostics);
    report["physical_result_qualified"] = json!(false);
    report["real_world_validation_status"] = json!("UNRESOLVED");
    report["claim_ceiling"] = json!(
        "metamorphic load-scaling consistency for receipt-bound observables within the named small-strain linear-static request envelope; no solver correctness, physical validation, or engineering-use qualification"
    );
    Ok(report)
}

fn compare_observables(
    baseline: &BTreeMap<String, ObservedObservable>,
    scaled: &BTreeMap<String, ObservedObservable>,
    alpha: f64,
    absolute_tolerance: f64,
    relative_tolerance: f64,
) -> (Vec<Value>, usize, usize, usize) {
    let mut results = Vec::with_capacity(baseline.len());
    let mut violations = 0usize;
    let mut unresolved = 0usize;
    for (id, original) in baseline {
        let actual = scaled.get(id).expect("observable ID inventories checked");
        if original.unit != actual.unit {
            results.push(json!({
                "observable_id": id,
                "unit": actual.unit,
                "status": "UNRESOLVED",
                "diagnostic": format!("unit mismatch: baseline '{}' scaled '{}'", original.unit, actual.unit)
            }));
            unresolved += 1;
            continue;
        }
        let expected = original.value * alpha;
        let error = (actual.value - expected).abs();
        let allowed = absolute_tolerance + relative_tolerance * expected.abs();
        if !expected.is_finite() || !error.is_finite() || !allowed.is_finite() {
            results.push(json!({
                "observable_id": id,
                "unit": original.unit,
                "status": "UNRESOLVED",
                "expected_scaled_value": null,
                "scaled_observed_value": actual.value,
                "absolute_error": null,
                "allowed_absolute_error": null,
                "relative_error": null,
                "relative_error_status": "overflow"
            }));
            unresolved += 1;
            continue;
        }
        let within = error <= allowed;
        if !within {
            violations += 1;
        }
        let (relative_error, relative_status) = if expected == 0.0 {
            (Value::Null, "undefined_zero_expected_value")
        } else {
            let value = error / expected.abs();
            if value.is_finite() {
                (json!(value), "computed")
            } else {
                (Value::Null, "overflow")
            }
        };
        results.push(json!({
            "observable_id": id,
            "unit": original.unit,
            "baseline_value": original.value,
            "scale_factor": alpha,
            "expected_scaled_value": expected,
            "scaled_observed_value": actual.value,
            "absolute_error": error,
            "absolute_tolerance": absolute_tolerance,
            "relative_tolerance": relative_tolerance,
            "allowed_absolute_error": allowed,
            "relative_error": relative_error,
            "relative_error_status": relative_status,
            "status": if within { "PASS" } else { "FAIL" }
        }));
    }
    (results, violations, unresolved, baseline.len())
}

fn validate_request_relation(
    baseline: &Value,
    scaled: &Value,
    manifest: &RelationManifest,
) -> Check<()> {
    let baseline_object = baseline
        .as_object()
        .ok_or_else(|| "baseline request must be an object".to_owned())?;
    let scaled_object = scaled
        .as_object()
        .ok_or_else(|| "scaled request must be an object".to_owned())?;
    let fidelity = req_str(baseline, "model_fidelity")?;
    if !["analytical_linear_elastic", "continuum_linear_elastic"].contains(&fidelity) {
        return Err("load-scaling relation requires analytical_linear_elastic or continuum_linear_elastic fidelity".to_owned());
    }
    let physics = req_string_array(baseline, "requested_physics")?;
    if physics.as_slice() != ["static_linear_elastic"] {
        return Err("load-scaling relation only supports static_linear_elastic as the sole requested physics".to_owned());
    }
    if req_str(scaled, "model_fidelity")? != fidelity
        || req_string_array(scaled, "requested_physics")? != physics
    {
        return Err("baseline and scaled requests must declare the same eligible model fidelity and physics".to_owned());
    }
    if req_str(baseline, "request_id")? == req_str(scaled, "request_id")? {
        return Err("baseline and scaled runs require distinct request IDs".to_owned());
    }
    if req_str(baseline, "scene_sha256")? != req_str(scaled, "scene_sha256")?
        || req_u64(baseline, "scene_revision")? != req_u64(scaled, "scene_revision")?
        || req_str(baseline, "coordinate_frame_id")? != req_str(scaled, "coordinate_frame_id")?
    {
        return Err(
            "baseline and scaled requests must bind the same scene revision and coordinate frame"
                .to_owned(),
        );
    }
    // The only top-level semantic difference permitted is request_id and force vectors.
    for (key, value) in baseline_object {
        if key == "request_id" || key == "loads" {
            continue;
        }
        if scaled_object.get(key) != Some(value) {
            return Err(format!(
                "scaled request changes non-load input field '{key}'"
            ));
        }
    }
    for key in scaled_object.keys() {
        if key != "request_id" && key != "loads" && !baseline_object.contains_key(key) {
            return Err(format!("scaled request adds non-load input field '{key}'"));
        }
    }
    let baseline_loads = req_array(baseline, "loads")?;
    let scaled_loads = req_array(scaled, "loads")?;
    if baseline_loads.is_empty() || baseline_loads.len() != scaled_loads.len() {
        return Err(
            "baseline and scaled requests must contain the same non-empty load inventory"
                .to_owned(),
        );
    }
    for (index, (base_load, scaled_load)) in
        baseline_loads.iter().zip(scaled_loads.iter()).enumerate()
    {
        if req_str(base_load, "kind")? != "point_force"
            || req_str(scaled_load, "kind")? != "point_force"
        {
            return Err(format!(
                "load {index} is outside v0.1 point_force-only scope"
            ));
        }
        let base_obj = base_load
            .as_object()
            .ok_or_else(|| format!("loads[{index}] must be an object"))?;
        let scaled_obj = scaled_load
            .as_object()
            .ok_or_else(|| format!("scaled loads[{index}] must be an object"))?;
        for (key, value) in base_obj {
            if key == "vector_xyz" {
                continue;
            }
            if scaled_obj.get(key) != Some(value) {
                return Err(format!(
                    "scaled load {index} changes load metadata field '{key}'"
                ));
            }
        }
        for key in scaled_obj.keys() {
            if key != "vector_xyz" && !base_obj.contains_key(key) {
                return Err(format!(
                    "scaled load {index} adds load metadata field '{key}'"
                ));
            }
        }
        let base_vector = req_vec3(base_load, "vector_xyz", &format!("loads[{index}]"))?;
        let scaled_vector = req_vec3(scaled_load, "vector_xyz", &format!("scaled loads[{index}]"))?;
        for component in 0..3 {
            let expected = base_vector[component] * manifest.scale_factor;
            if !expected.is_finite() || scaled_vector[component] != expected {
                return Err(format!(
                    "scaled request load {index} component {component} is not exact factor {} times its baseline",
                    manifest.scale_factor
                ));
            }
        }
    }
    Ok(())
}

fn bind_output(
    artifact_root: &Path,
    receipt: &Value,
    verification_report: &Value,
    output_id: &str,
    expected_kind: &str,
) -> Check<BoundOutput> {
    let outputs = req_array(receipt, "outputs")?;
    let matches: Vec<&Value> = outputs
        .iter()
        .filter(|output| output.get("output_id").and_then(Value::as_str) == Some(output_id))
        .collect();
    if matches.len() != 1 {
        return Err(format!(
            "receipt must contain exactly one output ID '{output_id}'"
        ));
    }
    let output = matches[0];
    if req_str(output, "kind")? != expected_kind {
        return Err(format!("output kind is not '{expected_kind}'"));
    }
    let results = verification_report
        .get("artifact_results")
        .and_then(Value::as_array)
        .ok_or_else(|| "artifact verifier omitted artifact_results".to_owned())?;
    let verified = results
        .iter()
        .find(|item| item.get("output_id").and_then(Value::as_str) == Some(output_id))
        .ok_or_else(|| format!("output '{output_id}' is missing a verifier entry"))?;
    if verified.get("status").and_then(Value::as_str) != Some("PASS") {
        return Err(format!(
            "output '{output_id}' did not pass byte verification"
        ));
    }
    let declared_unit = output
        .get("unit")
        .and_then(Value::as_str)
        .filter(|unit| !unit.trim().is_empty() && unit.len() <= 64)
        .ok_or_else(|| "scored load-scaling output requires an explicit non-empty unit".to_owned())?
        .to_owned();
    let expected_sha = req_str(output, "sha256")?;
    if req_str(verified, "observed_sha256")? != expected_sha {
        return Err(format!(
            "output '{output_id}' verifier digest differs from receipt"
        ));
    }
    let root = fs::canonicalize(artifact_root)
        .map_err(|error| format!("cannot canonicalize artifact root: {error}"))?;
    let path = safe_relative_path(req_str(output, "artifact_ref")?)?;
    let candidate = root.join(path);
    let canonical =
        fs::canonicalize(&candidate).map_err(|error| format!("cannot resolve output: {error}"))?;
    if !canonical.starts_with(&root) {
        return Err("output resolves outside the canonical artifact root".to_owned());
    }
    let metadata =
        fs::metadata(&canonical).map_err(|error| format!("cannot inspect output: {error}"))?;
    if !metadata.is_file() || metadata.len() > MAX_NORMALIZED_BYTES {
        return Err(
            "normalized output is not a regular file within the parser byte budget".to_owned(),
        );
    }
    let bytes = fs::read(&canonical).map_err(|error| format!("cannot read output: {error}"))?;
    let output_sha256 = sha256_hex(&bytes);
    if output_sha256 != expected_sha {
        return Err("normalized output bytes changed after receipt verification".to_owned());
    }
    Ok(BoundOutput {
        bytes,
        output_sha256,
        declared_unit,
        verification_report: verification_report.clone(),
        producer_status: req_str(verification_report, "producer_status")?.to_owned(),
    })
}

fn parse_manifest(bytes: &[u8]) -> Check<RelationManifest> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("invalid load-scaling manifest JSON: {error}"))?;
    exact_keys(
        &value,
        &[
            "schema_version",
            "relation_id",
            "case_id",
            "baseline_request_sha256",
            "scaled_request_sha256",
            "load_scale_factor",
            "absolute_tolerance",
            "relative_tolerance",
            "output_pairs",
        ],
        "load-scaling manifest",
    )?;
    if req_str(&value, "schema_version")? != MANIFEST_SCHEMA {
        return Err(format!(
            "manifest.schema_version must equal '{MANIFEST_SCHEMA}'"
        ));
    }
    let relation_id = req_token(&value, "relation_id")?;
    let case_id = req_token(&value, "case_id")?;
    let baseline_request_sha256 = req_str(&value, "baseline_request_sha256")?.to_owned();
    let scaled_request_sha256 = req_str(&value, "scaled_request_sha256")?.to_owned();
    valid_sha(&baseline_request_sha256, "manifest.baseline_request_sha256")?;
    valid_sha(&scaled_request_sha256, "manifest.scaled_request_sha256")?;
    let scale_factor = req_number(&value, "load_scale_factor")?;
    if scale_factor <= 0.0 || scale_factor == 1.0 {
        return Err(
            "manifest.load_scale_factor must be finite, positive and different from 1".to_owned(),
        );
    }
    let absolute_tolerance = req_number(&value, "absolute_tolerance")?;
    let relative_tolerance = req_number(&value, "relative_tolerance")?;
    if absolute_tolerance < 0.0 || relative_tolerance < 0.0 {
        return Err("load-scaling tolerances must be non-negative".to_owned());
    }
    let pairs = req_array(&value, "output_pairs")?;
    if pairs.is_empty() {
        return Err("manifest.output_pairs must be non-empty".to_owned());
    }
    let mut output_pairs = Vec::with_capacity(pairs.len());
    let mut baseline_ids = BTreeSet::new();
    let mut scaled_ids = BTreeSet::new();
    let mut kinds = BTreeSet::new();
    for (index, pair) in pairs.iter().enumerate() {
        let at = format!("manifest.output_pairs[{index}]");
        exact_keys(
            pair,
            &["output_kind", "baseline_output_id", "scaled_output_id"],
            &at,
        )?;
        let kind = req_token(pair, "output_kind")?;
        let baseline_id = req_token(pair, "baseline_output_id")?;
        let scaled_id = req_token(pair, "scaled_output_id")?;
        if !kinds.insert(kind.clone())
            || !baseline_ids.insert(baseline_id.clone())
            || !scaled_ids.insert(scaled_id.clone())
        {
            return Err(format!("{at} duplicates an output kind or output ID"));
        }
        output_pairs.push(OutputPair {
            kind,
            baseline_id,
            scaled_id,
        });
    }
    Ok(RelationManifest {
        relation_id,
        case_id,
        baseline_request_sha256,
        scaled_request_sha256,
        scale_factor,
        absolute_tolerance,
        relative_tolerance,
        output_pairs,
    })
}

fn base_report(
    manifest: &RelationManifest,
    manifest_sha: &str,
    baseline_sha: &str,
    scaled_sha: &str,
) -> Value {
    json!({
        "schema_version":"physical-load-scaling-relation-report-v0.1",
        "relation_status":"UNRESOLVED",
        "relation_id":manifest.relation_id,
        "case_id":manifest.case_id,
        "manifest_sha256":manifest_sha,
        "baseline_request_sha256":baseline_sha,
        "scaled_request_sha256":scaled_sha,
        "scale_factor":manifest.scale_factor,
        "absolute_tolerance":manifest.absolute_tolerance,
        "relative_tolerance":manifest.relative_tolerance,
        "baseline_artifact_verification_status":"UNRESOLVED",
        "scaled_artifact_verification_status":"UNRESOLVED",
        "baseline_output_artifact_bytes_verified":false,
        "scaled_output_artifact_bytes_verified":false,
        "baseline_producer_status":"UNRESOLVED",
        "scaled_producer_status":"UNRESOLVED",
        "output_results":[],
        "observable_count":0,
        "violation_count":0,
        "diagnostics":[],
        "physical_result_qualified":false,
        "real_world_validation_status":"UNRESOLVED",
        "claim_ceiling":"load-scaling relation unresolved; no solver or physical qualification is implied"
    })
}
fn unresolved(mut report: Value, reason: impl Into<String>) -> Value {
    report["relation_status"] = json!("UNRESOLVED");
    report["diagnostics"] = json!([reason.into()]);
    report["physical_result_qualified"] = json!(false);
    report["real_world_validation_status"] = json!("UNRESOLVED");
    report
}
fn exact_keys(value: &Value, expected: &[&str], at: &str) -> Check<()> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{at} must be an object"))?;
    let expected: BTreeSet<&str> = expected.iter().copied().collect();
    let actual: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let missing: Vec<_> = expected.difference(&actual).copied().collect();
    let unknown: Vec<_> = actual.difference(&expected).copied().collect();
    if !missing.is_empty() || !unknown.is_empty() {
        return Err(format!(
            "{at}: missing keys {missing:?}; unknown keys {unknown:?}"
        ));
    }
    Ok(())
}
fn req_str<'a>(value: &'a Value, key: &str) -> Check<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{key} must be a string"))
}
fn req_u64(value: &Value, key: &str) -> Check<u64> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{key} must be a non-negative integer"))
}
fn req_number(value: &Value, key: &str) -> Check<f64> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .filter(|n| n.is_finite())
        .ok_or_else(|| format!("{key} must be finite numeric"))
}
fn req_array<'a>(value: &'a Value, key: &str) -> Check<&'a Vec<Value>> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{key} must be an array"))
}
fn req_string_array(value: &Value, key: &str) -> Check<Vec<String>> {
    req_array(value, key)?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{key} entries must be strings"))
        })
        .collect()
}
fn req_token(value: &Value, key: &str) -> Check<String> {
    let value = req_str(value, key)?;
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':'))
    {
        return Err(format!("{key} must be a stable token"));
    }
    Ok(value.to_owned())
}
fn valid_sha(value: &str, label: &str) -> Check<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(format!(
            "{label} must be 64 lowercase hexadecimal characters"
        ));
    }
    Ok(())
}
fn req_vec3(value: &Value, key: &str, at: &str) -> Check<[f64; 3]> {
    let values = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{at}.{key} must be a three-component vector"))?;
    if values.len() != 3 {
        return Err(format!("{at}.{key} must contain exactly three components"));
    }
    let mut output = [0.0; 3];
    for (index, item) in values.iter().enumerate() {
        output[index] = item
            .as_f64()
            .filter(|n| n.is_finite())
            .ok_or_else(|| format!("{at}.{key}[{index}] must be finite numeric"))?;
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    struct Bundle {
        root: PathBuf,
        baseline_request: Vec<u8>,
        baseline_receipt: Vec<u8>,
        scaled_request: Vec<u8>,
        scaled_receipt: Vec<u8>,
        manifest: Vec<u8>,
    }
    impl Drop for Bundle {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn make_bundle(alpha: f64, scaled_output_multiplier: f64) -> Bundle {
        let mut baseline: Value = serde_json::from_str(include_str!(
            "../../../schemas/physical-transformation/fixtures/analytical-bar-request-v0.1.json"
        ))
        .unwrap();
        baseline["request_id"] = json!("fixture:load-scale-baseline");
        baseline["requested_physics"] = json!(["static_linear_elastic"]);
        baseline["required_outputs"] = json!(["displacement"]);
        baseline["loads"][0]["kind"] = json!("point_force");
        baseline["loads"][0]["vector_xyz"] = json!([1000.0, 0.0, 0.0]);
        let mut scaled = baseline.clone();
        scaled["request_id"] = json!("fixture:load-scale-scaled");
        scaled["loads"][0]["vector_xyz"] = json!([1000.0 * alpha, 0.0, 0.0]);
        let baseline_request = serde_json::to_vec(&baseline).unwrap();
        let scaled_request = serde_json::to_vec(&scaled).unwrap();
        let baseline_sha = sha256_hex(&baseline_request);
        let scaled_sha = sha256_hex(&scaled_request);
        let scene_sha = baseline["scene_sha256"].as_str().unwrap();
        let scene_revision = baseline["scene_revision"].as_u64().unwrap();
        let frame = baseline["coordinate_frame_id"].as_str().unwrap();
        let root = std::env::temp_dir().join(format!(
            "symtropy-load-scaling-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();

        let baseline_obs = json!({
            "schema_version":"physical-normalized-observables-v0.1",
            "request_sha256":baseline_sha,
            "request_id":baseline["request_id"],
            "scene_sha256":scene_sha,
            "scene_revision":scene_revision,
            "case_id":"load-scaling-synthetic-case",
            "observables":[
                {"observable_id":"tip_ux","value":0.01,"unit":"m"},
                {"observable_id":"tip_uy","value":0.0,"unit":"m"}
            ]
        });
        let scaled_obs = json!({
            "schema_version":"physical-normalized-observables-v0.1",
            "request_sha256":scaled_sha,
            "request_id":scaled["request_id"],
            "scene_sha256":scene_sha,
            "scene_revision":scene_revision,
            "case_id":"load-scaling-synthetic-case",
            "observables":[
                {"observable_id":"tip_ux","value":0.01 * alpha * scaled_output_multiplier,"unit":"m"},
                {"observable_id":"tip_uy","value":0.0,"unit":"m"}
            ]
        });
        let b_bytes = serde_json::to_vec(&baseline_obs).unwrap();
        let s_bytes = serde_json::to_vec(&scaled_obs).unwrap();
        fs::write(root.join("baseline-displacement.json"), &b_bytes).unwrap();
        fs::write(root.join("scaled-displacement.json"), &s_bytes).unwrap();

        fn receipt(
            request: &Value,
            request_bytes: &[u8],
            output_id: &str,
            path: &str,
            output_bytes: &[u8],
        ) -> Value {
            json!({
                "schema_version":"physical-solver-receipt-v0.1",
                "request_id":request["request_id"],
                "request_sha256":sha256_hex(request_bytes),
                "source_revision":request["source_revision"],
                "adapter":{
                    "adapter_id":"synthetic-test-only",
                    "version":"0.1.0",
                    "executable_sha256":"b".repeat(64),
                    "configuration_sha256":"c".repeat(64),
                    "solver_name":"synthetic-test-only",
                    "solver_version":"0.1.0",
                    "solver_build_ref":"fixture-only:no-solver-executed"
                },
                "status":"PASS",
                "evidence_class":"synthetic",
                "physics_evaluated":["static_linear_elastic"],
                "physics_omitted":[],
                "convergence":{"state":"not_applicable","iterations":null,"residual":null,"mesh_refinement_evaluated":false,"details":"Synthetic test only; no external solver executed."},
                "outputs":[{
                    "output_id":output_id,
                    "kind":"displacement",
                    "sha256":sha256_hex(output_bytes),
                    "media_type":"application/json",
                    "unit":"m",
                    "artifact_ref":path
                }],
                "uncertainty":{"state":"unknown","method":null,"summary":"Synthetic test only."},
                "validity_envelope":{"supported_geometry":"synthetic fixture","supported_material_model":"synthetic linear elasticity","load_regime":"synthetic static point force","limitations":["No external solver executed."]},
                "claim_ceiling":"Synthetic fixture only; not physically qualified.",
                "diagnostics":[]
            })
        }
        let baseline_receipt = serde_json::to_vec(&receipt(
            &baseline,
            &baseline_request,
            "baseline-displacement",
            "baseline-displacement.json",
            &b_bytes,
        ))
        .unwrap();
        let scaled_receipt = serde_json::to_vec(&receipt(
            &scaled,
            &scaled_request,
            "scaled-displacement",
            "scaled-displacement.json",
            &s_bytes,
        ))
        .unwrap();
        let manifest = json!({
            "schema_version":MANIFEST_SCHEMA,
            "relation_id":"fixture-load-scaling-relation",
            "case_id":"load-scaling-synthetic-case",
            "baseline_request_sha256":baseline_sha,
            "scaled_request_sha256":scaled_sha,
            "load_scale_factor":alpha,
            "absolute_tolerance":1e-12,
            "relative_tolerance":1e-10,
            "output_pairs":[{"output_kind":"displacement","baseline_output_id":"baseline-displacement","scaled_output_id":"scaled-displacement"}]
        });
        Bundle {
            root,
            baseline_request,
            baseline_receipt,
            scaled_request,
            scaled_receipt,
            manifest: serde_json::to_vec(&manifest).unwrap(),
        }
    }

    fn assess(bundle: &Bundle) -> Value {
        assess_load_scaling_relation_bytes(
            &bundle.baseline_request,
            &bundle.baseline_receipt,
            &bundle.scaled_request,
            &bundle.scaled_receipt,
            &bundle.root,
            &bundle.manifest,
        )
        .unwrap()
    }

    #[test]
    fn exact_synthetic_load_scaling_passes_but_does_not_qualify_physics() {
        let report = assess(&make_bundle(2.0, 1.0));
        assert_eq!(report["relation_status"], "PASS");
        assert_eq!(report["observable_count"], 2);
        assert_eq!(report["violation_count"], 0);
        assert_eq!(report["physical_result_qualified"], false);
        assert_eq!(report["real_world_validation_status"], "UNRESOLVED");
    }

    #[test]
    fn wrong_scaled_response_fails_the_relation() {
        let report = assess(&make_bundle(2.0, 1.1));
        assert_eq!(report["relation_status"], "FAIL");
        assert_eq!(report["violation_count"], 1);
    }

    #[test]
    fn exact_zero_expected_value_has_no_invented_relative_error() {
        let report = assess(&make_bundle(2.0, 1.0));
        let outputs = report["output_results"].as_array().unwrap();
        let zero = outputs[0]["observable_results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|value| value["observable_id"] == "tip_uy")
            .unwrap();
        assert_eq!(zero["relative_error"], Value::Null);
        assert_eq!(
            zero["relative_error_status"],
            "undefined_zero_expected_value"
        );
    }

    #[test]
    fn wrong_request_load_factor_is_unresolved() {
        let mut bundle = make_bundle(2.0, 1.0);
        let mut scaled: Value = serde_json::from_slice(&bundle.scaled_request).unwrap();
        scaled["loads"][0]["vector_xyz"] = json!([1500.0, 0.0, 0.0]);
        bundle.scaled_request = serde_json::to_vec(&scaled).unwrap();
        let mut manifest: Value = serde_json::from_slice(&bundle.manifest).unwrap();
        manifest["scaled_request_sha256"] = json!(sha256_hex(&bundle.scaled_request));
        bundle.manifest = serde_json::to_vec(&manifest).unwrap();
        let report = assess(&bundle);
        assert_eq!(report["relation_status"], "UNRESOLVED");
    }

    #[test]
    fn request_manifest_digest_drift_is_unresolved() {
        let mut bundle = make_bundle(2.0, 1.0);
        let mut manifest: Value = serde_json::from_slice(&bundle.manifest).unwrap();
        manifest["baseline_request_sha256"] = json!("d".repeat(64));
        bundle.manifest = serde_json::to_vec(&manifest).unwrap();
        let report = assess(&bundle);
        assert_eq!(report["relation_status"], "UNRESOLVED");
    }

    #[test]
    fn tampered_normalized_artifact_is_unresolved() {
        let bundle = make_bundle(2.0, 1.0);
        fs::write(bundle.root.join("scaled-displacement.json"), b"tampered").unwrap();
        let report = assess(&bundle);
        assert_eq!(report["relation_status"], "UNRESOLVED");
        assert_eq!(report["scaled_artifact_verification_status"], "FAIL");
    }

    #[test]
    fn receipt_unit_must_match_normalized_observables() {
        let mut bundle = make_bundle(2.0, 1.0);
        let mut receipt: Value = serde_json::from_slice(&bundle.baseline_receipt).unwrap();
        receipt["outputs"][0]["unit"] = json!("mm");
        bundle.baseline_receipt = serde_json::to_vec(&receipt).unwrap();
        let report = assess(&bundle);
        assert_eq!(report["relation_status"], "UNRESOLVED");
    }

    #[test]
    fn changed_non_load_input_is_unresolved() {
        let mut bundle = make_bundle(2.0, 1.0);
        let mut scaled: Value = serde_json::from_slice(&bundle.scaled_request).unwrap();
        scaled["materials"][0]["youngs_modulus_pa"] = json!(180000000000.0);
        bundle.scaled_request = serde_json::to_vec(&scaled).unwrap();
        let mut manifest: Value = serde_json::from_slice(&bundle.manifest).unwrap();
        manifest["scaled_request_sha256"] = json!(sha256_hex(&bundle.scaled_request));
        bundle.manifest = serde_json::to_vec(&manifest).unwrap();
        let report = assess(&bundle);
        assert_eq!(report["relation_status"], "UNRESOLVED");
    }

    #[test]
    fn nonlinear_fidelity_is_outside_relation_envelope() {
        let mut bundle = make_bundle(2.0, 1.0);
        let mut baseline: Value = serde_json::from_slice(&bundle.baseline_request).unwrap();
        baseline["model_fidelity"] = json!("continuum_nonlinear");
        bundle.baseline_request = serde_json::to_vec(&baseline).unwrap();
        let mut manifest: Value = serde_json::from_slice(&bundle.manifest).unwrap();
        manifest["baseline_request_sha256"] = json!(sha256_hex(&bundle.baseline_request));
        bundle.manifest = serde_json::to_vec(&manifest).unwrap();
        let report = assess(&bundle);
        assert_eq!(report["relation_status"], "UNRESOLVED");
    }

    #[test]
    fn scale_factor_one_is_not_a_meaningful_scaling_case() {
        assert!(parse_manifest(&make_bundle(2.0, 1.0).manifest).is_ok());
        let mut value: Value = serde_json::from_slice(&make_bundle(2.0, 1.0).manifest).unwrap();
        value["load_scale_factor"] = json!(1.0);
        assert!(parse_manifest(serde_json::to_string(&value).unwrap().as_bytes()).is_err());
    }
}
