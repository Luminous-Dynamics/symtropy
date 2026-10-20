// SPDX-License-Identifier: AGPL-3.0-or-later
//! Receipt-bound superposition relation for small-strain static linear elasticity.
//! This relation is a code-verification invariant, not physical validation.

use crate::{
    reference_compare::{MAX_NORMALIZED_BYTES, ObservedObservable, parse_observation, safe_relative_path},
    sha256_hex, verify_solver_output_artifact_bytes,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

type Check<T> = Result<T, String>;
const MANIFEST_SCHEMA: &str = "physical-linear-superposition-relation-v0.1";

#[derive(Clone)]
struct OutputPair {
    kind: String,
    a_id: String,
    b_id: String,
    combined_id: String,
}

struct RelationManifest {
    relation_id: String,
    case_id: String,
    request_a_sha256: String,
    request_b_sha256: String,
    request_combined_sha256: String,
    absolute_tolerance: f64,
    relative_tolerance: f64,
    output_pairs: Vec<OutputPair>,
}

struct BoundOutput {
    bytes: Vec<u8>,
    sha256: String,
    declared_unit: String,
    verification_report: Value,
    producer_status: String,
}

/// Assess response(A + B) == response(A) + response(B) for a narrowly declared
/// linear-static model envelope. The exact requests, receipts and selected outputs
/// are hashed and bound; all required output kinds must be scored.
pub fn assess_superposition_relation_bytes(
    request_a_bytes: &[u8],
    receipt_a_bytes: &[u8],
    request_b_bytes: &[u8],
    receipt_b_bytes: &[u8],
    request_combined_bytes: &[u8],
    receipt_combined_bytes: &[u8],
    artifact_root: &Path,
    manifest_bytes: &[u8],
) -> Check<Value> {
    let manifest_sha256 = sha256_hex(manifest_bytes);
    let request_a_sha256 = sha256_hex(request_a_bytes);
    let request_b_sha256 = sha256_hex(request_b_bytes);
    let request_combined_sha256 = sha256_hex(request_combined_bytes);
    let manifest = parse_manifest(manifest_bytes)?;
    let mut report = base_report(
        &manifest,
        &manifest_sha256,
        &request_a_sha256,
        &request_b_sha256,
        &request_combined_sha256,
    );
    if manifest.request_a_sha256 != request_a_sha256
        || manifest.request_b_sha256 != request_b_sha256
        || manifest.request_combined_sha256 != request_combined_sha256
    {
        return Ok(unresolved(report, "relation manifest does not bind all three exact request byte streams"));
    }

    let request_a: Value = match serde_json::from_slice(request_a_bytes) {
        Ok(value) => value,
        Err(error) => return Ok(unresolved(report, format!("request A JSON is invalid: {error}"))),
    };
    let request_b: Value = match serde_json::from_slice(request_b_bytes) {
        Ok(value) => value,
        Err(error) => return Ok(unresolved(report, format!("request B JSON is invalid: {error}"))),
    };
    let request_combined: Value = match serde_json::from_slice(request_combined_bytes) {
        Ok(value) => value,
        Err(error) => return Ok(unresolved(report, format!("combined request JSON is invalid: {error}"))),
    };
    if let Err(reason) = validate_request_relation(&request_a, &request_b, &request_combined) {
        return Ok(unresolved(report, reason));
    }

    let paired_kinds: BTreeSet<&str> = manifest.output_pairs.iter().map(|pair| pair.kind.as_str()).collect();
    let requested_kinds: BTreeSet<&str> = req_array(&request_a, "required_outputs")?
        .iter()
        .map(|value| value.as_str().unwrap_or(""))
        .collect();
    if paired_kinds != requested_kinds {
        return Ok(unresolved(
            report,
            format!("output pairs must cover the exact required-output inventory; requested={requested_kinds:?}, paired={paired_kinds:?}"),
        ));
    }

    let verify_a = match verify_solver_output_artifact_bytes(request_a_bytes, receipt_a_bytes, artifact_root) {
        Ok(value) => value,
        Err(error) => return Ok(unresolved(report, format!("run A receipt/artifact verification failed: {error}"))),
    };
    let verify_b = match verify_solver_output_artifact_bytes(request_b_bytes, receipt_b_bytes, artifact_root) {
        Ok(value) => value,
        Err(error) => return Ok(unresolved(report, format!("run B receipt/artifact verification failed: {error}"))),
    };
    let verify_combined = match verify_solver_output_artifact_bytes(
        request_combined_bytes,
        receipt_combined_bytes,
        artifact_root,
    ) {
        Ok(value) => value,
        Err(error) => return Ok(unresolved(report, format!("combined receipt/artifact verification failed: {error}"))),
    };
    set_verification_fields(&mut report, "a", &verify_a);
    set_verification_fields(&mut report, "b", &verify_b);
    set_verification_fields(&mut report, "combined", &verify_combined);
    for (label, verification) in [
        ("A", &verify_a),
        ("B", &verify_b),
        ("combined", &verify_combined),
    ] {
        if verification["artifact_verification_status"] != "PASS"
            || verification["output_artifact_bytes_verified"] != true
        {
            return Ok(unresolved(
                report,
                format!("run {label} must pass receipt-bound output-byte verification"),
            ));
        }
        if verification["producer_status"] != "PASS" {
            return Ok(unresolved(report, format!("run {label} producer receipt is not PASS")));
        }
    }

    let receipt_a: Value = match serde_json::from_slice(receipt_a_bytes) {
        Ok(value) => value,
        Err(error) => return Ok(unresolved(report, format!("receipt A JSON is invalid: {error}"))),
    };
    let receipt_b: Value = match serde_json::from_slice(receipt_b_bytes) {
        Ok(value) => value,
        Err(error) => return Ok(unresolved(report, format!("receipt B JSON is invalid: {error}"))),
    };
    let receipt_combined: Value = match serde_json::from_slice(receipt_combined_bytes) {
        Ok(value) => value,
        Err(error) => return Ok(unresolved(report, format!("combined receipt JSON is invalid: {error}"))),
    };

    let mut output_results = Vec::with_capacity(manifest.output_pairs.len());
    let mut violation_count = 0usize;
    let mut unresolved_count = 0usize;
    let mut observable_count = 0usize;
    let mut diagnostics = Vec::new();
    for pair in &manifest.output_pairs {
        let output_a = match bind_output(artifact_root, &receipt_a, &verify_a, &pair.a_id, &pair.kind) {
            Ok(value) => value,
            Err(reason) => return Ok(unresolved(report, format!("run A output '{}': {reason}", pair.a_id))),
        };
        let output_b = match bind_output(artifact_root, &receipt_b, &verify_b, &pair.b_id, &pair.kind) {
            Ok(value) => value,
            Err(reason) => return Ok(unresolved(report, format!("run B output '{}': {reason}", pair.b_id))),
        };
        let output_combined = match bind_output(
            artifact_root,
            &receipt_combined,
            &verify_combined,
            &pair.combined_id,
            &pair.kind,
        ) {
            Ok(value) => value,
            Err(reason) => return Ok(unresolved(report, format!("combined output '{}': {reason}", pair.combined_id))),
        };

        let value_a: Value = match serde_json::from_slice(&output_a.bytes) {
            Ok(value) => value,
            Err(error) => return Ok(unresolved(report, format!("run A normalized output JSON is invalid: {error}"))),
        };
        let value_b: Value = match serde_json::from_slice(&output_b.bytes) {
            Ok(value) => value,
            Err(error) => return Ok(unresolved(report, format!("run B normalized output JSON is invalid: {error}"))),
        };
        let value_combined: Value = match serde_json::from_slice(&output_combined.bytes) {
            Ok(value) => value,
            Err(error) => return Ok(unresolved(report, format!("combined normalized output JSON is invalid: {error}"))),
        };

        let observables_a = match parse_observation(
            &value_a,
            &request_a_sha256,
            req_str(&request_a, "request_id")?,
            req_str(&request_a, "scene_sha256")?,
            req_u64(&request_a, "scene_revision")?,
            &manifest.case_id,
        ) {
            Ok(value) => value,
            Err(error) => return Ok(unresolved(report, format!("run A observation is not request-bound/valid: {error}"))),
        };
        let observables_b = match parse_observation(
            &value_b,
            &request_b_sha256,
            req_str(&request_b, "request_id")?,
            req_str(&request_b, "scene_sha256")?,
            req_u64(&request_b, "scene_revision")?,
            &manifest.case_id,
        ) {
            Ok(value) => value,
            Err(error) => return Ok(unresolved(report, format!("run B observation is not request-bound/valid: {error}"))),
        };
        let observables_combined = match parse_observation(
            &value_combined,
            &request_combined_sha256,
            req_str(&request_combined, "request_id")?,
            req_str(&request_combined, "scene_sha256")?,
            req_u64(&request_combined, "scene_revision")?,
            &manifest.case_id,
        ) {
            Ok(value) => value,
            Err(error) => return Ok(unresolved(report, format!("combined observation is not request-bound/valid: {error}"))),
        };

        let inventory_a: BTreeSet<&String> = observables_a.keys().collect();
        let inventory_b: BTreeSet<&String> = observables_b.keys().collect();
        let inventory_combined: BTreeSet<&String> = observables_combined.keys().collect();
        if inventory_a != inventory_b || inventory_a != inventory_combined {
            return Ok(unresolved(report, format!("observable inventory differs across A, B and combined runs for output kind '{}'", pair.kind)));
        }
        for (label, output, observables) in [
            ("A", &output_a, &observables_a),
            ("B", &output_b, &observables_b),
            ("combined", &output_combined, &observables_combined),
        ] {
            if observables.values().any(|observable| observable.unit != output.declared_unit) {
                return Ok(unresolved(report, format!("run {label} normalized units disagree with receipt-declared unit for output kind '{}'", pair.kind)));
            }
        }
        let (results, violations, unresolved_values, count) = compare_observables(
            &observables_a,
            &observables_b,
            &observables_combined,
            manifest.absolute_tolerance,
            manifest.relative_tolerance,
        );
        violation_count += violations;
        unresolved_count += unresolved_values;
        observable_count += count;
        if violations > 0 {
            diagnostics.push(format!(
                "output kind '{}' has {violations} observable(s) outside the superposition tolerance",
                pair.kind
            ));
        }
        if unresolved_values > 0 {
            diagnostics.push(format!(
                "output kind '{}' has {unresolved_values} unresolved observable(s)",
                pair.kind
            ));
        }
        output_results.push(json!({
            "output_kind": pair.kind,
            "output_a_id": pair.a_id,
            "output_b_id": pair.b_id,
            "output_combined_id": pair.combined_id,
            "output_a_sha256": output_a.sha256,
            "output_b_sha256": output_b.sha256,
            "output_combined_sha256": output_combined.sha256,
            "observable_count": count,
            "observable_results": results
        }));
    }

    let status = if unresolved_count > 0 {
        "UNRESOLVED"
    } else if violation_count > 0 {
        "FAIL"
    } else {
        "PASS"
    };
    report["relation_status"] = json!(status);
    report["output_results"] = json!(output_results);
    report["observable_count"] = json!(observable_count);
    report["violation_count"] = json!(violation_count);
    report["unresolved_observable_count"] = json!(unresolved_count);
    report["diagnostics"] = json!(diagnostics);
    report["physical_result_qualified"] = json!(false);
    report["real_world_validation_status"] = json!("UNRESOLVED");
    report["claim_ceiling"] = json!(
        "metamorphic response superposition consistency for receipt-bound observables within the named linear-static request envelope; no solver correctness, physical validation, or engineering-use qualification"
    );
    Ok(report)
}

fn compare_observables(
    a: &BTreeMap<String, ObservedObservable>,
    b: &BTreeMap<String, ObservedObservable>,
    combined: &BTreeMap<String, ObservedObservable>,
    absolute_tolerance: f64,
    relative_tolerance: f64,
) -> (Vec<Value>, usize, usize, usize) {
    let mut results = Vec::with_capacity(a.len());
    let mut violations = 0usize;
    let mut unresolved = 0usize;
    for (id, value_a) in a {
        let value_b = b.get(id).expect("observable inventory checked");
        let value_combined = combined.get(id).expect("observable inventory checked");
        if value_a.unit != value_b.unit || value_a.unit != value_combined.unit {
            results.push(json!({
                "observable_id": id,
                "status": "UNRESOLVED",
                "units": {"a": value_a.unit, "b": value_b.unit, "combined": value_combined.unit},
                "diagnostic": "unit disagreement across A, B and combined results"
            }));
            unresolved += 1;
            continue;
        }
        let expected = value_a.value + value_b.value;
        let error = (value_combined.value - expected).abs();
        let allowed = absolute_tolerance + relative_tolerance * expected.abs();
        if !expected.is_finite() || !error.is_finite() || !allowed.is_finite() {
            results.push(json!({
                "observable_id": id,
                "unit": value_a.unit,
                "status": "UNRESOLVED",
                "expected_combined_value": null,
                "observed_combined_value": value_combined.value,
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
            "unit": value_a.unit,
            "response_a": value_a.value,
            "response_b": value_b.value,
            "expected_combined_value": expected,
            "observed_combined_value": value_combined.value,
            "absolute_error": error,
            "absolute_tolerance": absolute_tolerance,
            "relative_tolerance": relative_tolerance,
            "allowed_absolute_error": allowed,
            "relative_error": relative_error,
            "relative_error_status": relative_status,
            "status": if within { "PASS" } else { "FAIL" }
        }));
    }
    (results, violations, unresolved, a.len())
}

fn validate_request_relation(a: &Value, b: &Value, combined: &Value) -> Check<()> {
    let a_object = a.as_object().ok_or_else(|| "request A must be an object".to_owned())?;
    let b_object = b.as_object().ok_or_else(|| "request B must be an object".to_owned())?;
    let combined_object = combined.as_object().ok_or_else(|| "combined request must be an object".to_owned())?;
    let fidelity = req_str(a, "model_fidelity")?;
    if !["analytical_linear_elastic", "continuum_linear_elastic"].contains(&fidelity) {
        return Err("superposition requires analytical_linear_elastic or continuum_linear_elastic fidelity".to_owned());
    }
    let physics = req_string_array(a, "requested_physics")?;
    if physics.len() != 1 || physics[0] != "static_linear_elastic" {
        return Err("superposition only supports static_linear_elastic as the sole requested physics".to_owned());
    }
    for (label, request) in [("B", b), ("combined", combined)] {
        if req_str(request, "model_fidelity")? != fidelity || req_string_array(request, "requested_physics")? != physics {
            return Err(format!("request {label} does not declare the same eligible fidelity and physics"));
        }
    }
    let ids = [
        req_str(a, "request_id")?,
        req_str(b, "request_id")?,
        req_str(combined, "request_id")?,
    ];
    if ids[0] == ids[1] || ids[0] == ids[2] || ids[1] == ids[2] {
        return Err("A, B and combined runs require distinct request IDs".to_owned());
    }
    for (key, label, request) in [
        ("scene_sha256", "B", b),
        ("scene_sha256", "combined", combined),
    ] {
        if req_str(a, key)? != req_str(request, key)? {
            return Err(format!("request {label} changes scene digest"));
        }
    }
    if req_u64(a, "scene_revision")? != req_u64(b, "scene_revision")?
        || req_u64(a, "scene_revision")? != req_u64(combined, "scene_revision")?
        || req_str(a, "coordinate_frame_id")? != req_str(b, "coordinate_frame_id")?
        || req_str(a, "coordinate_frame_id")? != req_str(combined, "coordinate_frame_id")?
    {
        return Err("A, B and combined requests must bind the same scene revision and coordinate frame".to_owned());
    }
    for (label, object) in [("B", b_object), ("combined", combined_object)] {
        for (key, value) in a_object {
            if key == "request_id" || key == "loads" {
                continue;
            }
            if object.get(key) != Some(value) {
                return Err(format!("request {label} changes non-load field '{key}'"));
            }
        }
        for key in object.keys() {
            if key != "request_id" && key != "loads" && !a_object.contains_key(key) {
                return Err(format!("request {label} adds non-load field '{key}'"));
            }
        }
    }

    let loads_a = req_array(a, "loads")?;
    let loads_b = req_array(b, "loads")?;
    let loads_combined = req_array(combined, "loads")?;
    if loads_a.is_empty() || loads_b.is_empty() || loads_combined.len() != loads_a.len() + loads_b.len() {
        return Err("A and B must have non-empty load inventories and combined must contain exactly their union".to_owned());
    }
    let mut union: BTreeMap<String, Value> = BTreeMap::new();
    for (label, loads) in [("A", loads_a), ("B", loads_b)] {
        for (index, load) in loads.iter().enumerate() {
            if req_str(load, "kind")? != "point_force" {
                return Err(format!("request {label} load {index} is outside v0.1 point_force-only scope"));
            }
            let id = req_token(load, "load_id")?;
            if union.insert(id.clone(), load.clone()).is_some() {
                return Err(format!("load ID '{id}' is duplicated across A and B"));
            }
        }
    }
    let mut combined_ids = BTreeSet::new();
    for (index, load) in loads_combined.iter().enumerate() {
        if req_str(load, "kind")? != "point_force" {
            return Err(format!("combined request load {index} is outside v0.1 point_force-only scope"));
        }
        let id = req_token(load, "load_id")?;
        if !combined_ids.insert(id.clone()) {
            return Err(format!("combined request duplicates load ID '{id}'"));
        }
        if union.get(&id) != Some(load) {
            return Err(format!("combined request load '{id}' is not byte-semantically equal to the union of A and B loads"));
        }
    }
    if combined_ids.len() != union.len() || union.keys().any(|id| !combined_ids.contains(id)) {
        return Err("combined request load inventory is not exactly the union of A and B".to_owned());
    }
    Ok(())
}

fn bind_output(
    root: &Path,
    receipt: &Value,
    verification: &Value,
    output_id: &str,
    expected_kind: &str,
) -> Check<BoundOutput> {
    let outputs = req_array(receipt, "outputs")?;
    let matches: Vec<&Value> = outputs.iter().filter(|output| {
        output.get("output_id").and_then(Value::as_str) == Some(output_id)
    }).collect();
    if matches.len() != 1 {
        return Err(format!("receipt must contain exactly one output ID '{output_id}'"));
    }
    let output = matches[0];
    if req_str(output, "kind")? != expected_kind {
        return Err(format!("output kind is not '{expected_kind}'"));
    }
    let declared_unit = output.get("unit")
        .and_then(Value::as_str)
        .filter(|unit| !unit.trim().is_empty() && unit.len() <= 64)
        .ok_or_else(|| "scored superposition output requires an explicit non-empty unit".to_owned())?
        .to_owned();
    let result_entries = verification.get("artifact_results").and_then(Value::as_array)
        .ok_or_else(|| "artifact verifier omitted artifact_results".to_owned())?;
    let verified = result_entries.iter().find(|entry| {
        entry.get("output_id").and_then(Value::as_str) == Some(output_id)
    }).ok_or_else(|| format!("output '{output_id}' is missing a verifier entry"))?;
    if verified.get("status").and_then(Value::as_str) != Some("PASS") {
        return Err(format!("output '{output_id}' did not pass byte verification"));
    }
    let expected_sha = req_str(output, "sha256")?;
    if req_str(verified, "observed_sha256")? != expected_sha {
        return Err(format!("output '{output_id}' verifier digest differs from receipt"));
    }
    let canonical_root = fs::canonicalize(root).map_err(|error| format!("cannot canonicalize artifact root: {error}"))?;
    let relative = safe_relative_path(req_str(output, "artifact_ref")?)?;
    let candidate = canonical_root.join(relative);
    let canonical = fs::canonicalize(&candidate).map_err(|error| format!("cannot resolve output: {error}"))?;
    if !canonical.starts_with(&canonical_root) {
        return Err("output resolves outside the canonical artifact root".to_owned());
    }
    let metadata = fs::metadata(&canonical).map_err(|error| format!("cannot inspect output: {error}"))?;
    if !metadata.is_file() || metadata.len() > MAX_NORMALIZED_BYTES {
        return Err("normalized output is not a regular file within the parser byte budget".to_owned());
    }
    let bytes = fs::read(&canonical).map_err(|error| format!("cannot read output: {error}"))?;
    let observed_sha = sha256_hex(&bytes);
    if observed_sha != expected_sha {
        return Err("normalized output changed after receipt verification".to_owned());
    }
    Ok(BoundOutput {
        bytes,
        sha256: observed_sha,
        declared_unit,
        verification_report: verification.clone(),
        producer_status: req_str(verification, "producer_status")?.to_owned(),
    })
}

fn set_verification_fields(report: &mut Value, label: &str, verification: &Value) {
    report[format!("{label}_artifact_verification_status")] =
        verification.get("artifact_verification_status").cloned().unwrap_or(Value::Null);
    report[format!("{label}_output_artifact_bytes_verified")] =
        verification.get("output_artifact_bytes_verified").cloned().unwrap_or(Value::Bool(false));
    report[format!("{label}_producer_status")] =
        verification.get("producer_status").cloned().unwrap_or(Value::Null);
}

fn parse_manifest(bytes: &[u8]) -> Check<RelationManifest> {
    let value: Value = serde_json::from_slice(bytes).map_err(|error| format!("invalid superposition manifest JSON: {error}"))?;
    exact_keys(
        &value,
        &[
            "schema_version", "relation_id", "case_id", "request_a_sha256",
            "request_b_sha256", "request_combined_sha256", "absolute_tolerance",
            "relative_tolerance", "output_pairs"
        ],
        "superposition manifest",
    )?;
    if req_str(&value, "schema_version")? != MANIFEST_SCHEMA {
        return Err(format!("manifest.schema_version must equal '{MANIFEST_SCHEMA}'"));
    }
    let relation_id = req_token(&value, "relation_id")?;
    let case_id = req_token(&value, "case_id")?;
    let request_a_sha256 = req_str(&value, "request_a_sha256")?.to_owned();
    let request_b_sha256 = req_str(&value, "request_b_sha256")?.to_owned();
    let request_combined_sha256 = req_str(&value, "request_combined_sha256")?.to_owned();
    for (label, sha) in [
        ("manifest.request_a_sha256", &request_a_sha256),
        ("manifest.request_b_sha256", &request_b_sha256),
        ("manifest.request_combined_sha256", &request_combined_sha256),
    ] {
        valid_sha(sha, label)?;
    }
    let absolute_tolerance = req_number(&value, "absolute_tolerance")?;
    let relative_tolerance = req_number(&value, "relative_tolerance")?;
    if absolute_tolerance < 0.0 || relative_tolerance < 0.0 {
        return Err("superposition tolerances must be finite and non-negative".to_owned());
    }
    let pairs = req_array(&value, "output_pairs")?;
    if pairs.is_empty() {
        return Err("manifest.output_pairs must be non-empty".to_owned());
    }
    let mut output_pairs = Vec::with_capacity(pairs.len());
    let mut kinds = BTreeSet::new();
    let mut a_ids = BTreeSet::new();
    let mut b_ids = BTreeSet::new();
    let mut combined_ids = BTreeSet::new();
    for (index, pair) in pairs.iter().enumerate() {
        let at = format!("manifest.output_pairs[{index}]");
        exact_keys(pair, &["output_kind", "output_a_id", "output_b_id", "output_combined_id"], &at)?;
        let kind = req_token(pair, "output_kind")?;
        let a_id = req_token(pair, "output_a_id")?;
        let b_id = req_token(pair, "output_b_id")?;
        let combined_id = req_token(pair, "output_combined_id")?;
        if !kinds.insert(kind.clone())
            || !a_ids.insert(a_id.clone())
            || !b_ids.insert(b_id.clone())
            || !combined_ids.insert(combined_id.clone())
        {
            return Err(format!("{at} duplicates an output kind or a same-run output ID"));
        }
        output_pairs.push(OutputPair { kind, a_id, b_id, combined_id });
    }
    Ok(RelationManifest {
        relation_id, case_id, request_a_sha256, request_b_sha256,
        request_combined_sha256, absolute_tolerance, relative_tolerance, output_pairs
    })
}

fn base_report(
    manifest: &RelationManifest,
    manifest_sha: &str,
    a_sha: &str,
    b_sha: &str,
    combined_sha: &str,
) -> Value {
    json!({
        "schema_version":"physical-linear-superposition-relation-report-v0.1",
        "relation_status":"UNRESOLVED",
        "relation_id":manifest.relation_id,
        "case_id":manifest.case_id,
        "manifest_sha256":manifest_sha,
        "request_a_sha256":a_sha,
        "request_b_sha256":b_sha,
        "request_combined_sha256":combined_sha,
        "absolute_tolerance":manifest.absolute_tolerance,
        "relative_tolerance":manifest.relative_tolerance,
        "a_artifact_verification_status":"UNRESOLVED",
        "b_artifact_verification_status":"UNRESOLVED",
        "combined_artifact_verification_status":"UNRESOLVED",
        "a_output_artifact_bytes_verified":false,
        "b_output_artifact_bytes_verified":false,
        "combined_output_artifact_bytes_verified":false,
        "a_producer_status":"UNRESOLVED",
        "b_producer_status":"UNRESOLVED",
        "combined_producer_status":"UNRESOLVED",
        "output_results":[],
        "observable_count":0,
        "violation_count":0,
        "unresolved_observable_count":0,
        "diagnostics":[],
        "physical_result_qualified":false,
        "real_world_validation_status":"UNRESOLVED",
        "claim_ceiling":"superposition relation unresolved; no solver or physical qualification is implied"
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
    let object = value.as_object().ok_or_else(|| format!("{at} must be an object"))?;
    let expected: BTreeSet<&str> = expected.iter().copied().collect();
    let actual: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let missing: Vec<_> = expected.difference(&actual).copied().collect();
    let unknown: Vec<_> = actual.difference(&expected).copied().collect();
    if !missing.is_empty() || !unknown.is_empty() {
        return Err(format!("{at}: missing keys {missing:?}; unknown keys {unknown:?}"));
    }
    Ok(())
}

fn req_str<'a>(value: &'a Value, key: &str) -> Check<&'a str> {
    value.get(key).and_then(Value::as_str).ok_or_else(|| format!("{key} must be a string"))
}
fn req_u64(value: &Value, key: &str) -> Check<u64> {
    value.get(key).and_then(Value::as_u64).ok_or_else(|| format!("{key} must be a non-negative integer"))
}
fn req_number(value: &Value, key: &str) -> Check<f64> {
    value.get(key).and_then(Value::as_f64).filter(|number| number.is_finite()).ok_or_else(|| format!("{key} must be finite numeric"))
}
fn req_array<'a>(value: &'a Value, key: &str) -> Check<&'a Vec<Value>> {
    value.get(key).and_then(Value::as_array).ok_or_else(|| format!("{key} must be an array"))
}
fn req_string_array(value: &Value, key: &str) -> Check<Vec<String>> {
    req_array(value, key)?
        .iter()
        .map(|item| item.as_str().map(str::to_owned).ok_or_else(|| format!("{key} entries must be strings")))
        .collect()
}
fn req_token(value: &Value, key: &str) -> Check<String> {
    let value = req_str(value, key)?;
    if value.is_empty()
        || value.len() > 128
        || !value.chars().all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | ':'))
    {
        return Err(format!("{key} must be a stable token"));
    }
    Ok(value.to_owned())
}
fn valid_sha(value: &str, label: &str) -> Check<()> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
        return Err(format!("{label} must be 64 lowercase hexadecimal characters"));
    }
    Ok(())
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
        request_a: Vec<u8>,
        receipt_a: Vec<u8>,
        request_b: Vec<u8>,
        receipt_b: Vec<u8>,
        request_combined: Vec<u8>,
        receipt_combined: Vec<u8>,
        manifest: Vec<u8>,
    }
    impl Drop for Bundle {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn make_bundle(combined_uy: f64) -> Bundle {
        let mut a: Value = serde_json::from_str(include_str!(
            "../../../schemas/physical-transformation/fixtures/analytical-bar-request-v0.1.json"
        )).unwrap();
        a["request_id"] = json!("fixture:superposition-a");
        a["requested_physics"] = json!(["static_linear_elastic"]);
        a["required_outputs"] = json!(["displacement"]);
        a["loads"][0]["kind"] = json!("point_force");
        a["loads"][0]["load_id"] = json!("force-a");
        a["loads"][0]["vector_xyz"] = json!([1000.0, 0.0, 0.0]);

        let mut b = a.clone();
        b["request_id"] = json!("fixture:superposition-b");
        b["loads"][0]["load_id"] = json!("force-b");
        b["loads"][0]["vector_xyz"] = json!([0.0, 1000.0, 0.0]);

        let mut combined = a.clone();
        combined["request_id"] = json!("fixture:superposition-combined");
        combined["loads"] = json!([
            a["loads"][0].clone(),
            b["loads"][0].clone()
        ]);

        let request_a = serde_json::to_vec(&a).unwrap();
        let request_b = serde_json::to_vec(&b).unwrap();
        let request_combined = serde_json::to_vec(&combined).unwrap();
        let sha_a = sha256_hex(&request_a);
        let sha_b = sha256_hex(&request_b);
        let sha_combined = sha256_hex(&request_combined);
        let scene_sha = a["scene_sha256"].as_str().unwrap();
        let scene_revision = a["scene_revision"].as_u64().unwrap();

        let root = std::env::temp_dir().join(format!(
            "symtropy-superposition-{}-{}",
            std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();

        fn observation(request: &Value, request_sha: &str, scene_sha: &str, scene_revision: u64, ux: f64, uy: f64) -> Vec<u8> {
            serde_json::to_vec(&json!({
                "schema_version":"physical-normalized-observables-v0.1",
                "request_sha256":request_sha,
                "request_id":request["request_id"],
                "scene_sha256":scene_sha,
                "scene_revision":scene_revision,
                "case_id":"superposition-synthetic-case",
                "observables":[
                    {"observable_id":"tip_ux","value":ux,"unit":"m"},
                    {"observable_id":"tip_uy","value":uy,"unit":"m"}
                ]
            })).unwrap()
        }
        let bytes_a = observation(&a, &sha_a, scene_sha, scene_revision, 0.01, 0.0);
        let bytes_b = observation(&b, &sha_b, scene_sha, scene_revision, 0.0, 0.02);
        let bytes_combined = observation(&combined, &sha_combined, scene_sha, scene_revision, 0.01, combined_uy);
        fs::write(root.join("a-displacement.json"), &bytes_a).unwrap();
        fs::write(root.join("b-displacement.json"), &bytes_b).unwrap();
        fs::write(root.join("combined-displacement.json"), &bytes_combined).unwrap();

        fn receipt(request: &Value, request_bytes: &[u8], output_id: &str, artifact_ref: &str, output_bytes: &[u8]) -> Vec<u8> {
            serde_json::to_vec(&json!({
                "schema_version":"physical-solver-receipt-v0.1",
                "request_id":request["request_id"],
                "request_sha256":sha256_hex(request_bytes),
                "source_revision":request["source_revision"],
                "adapter":{
                    "adapter_id":"synthetic-superposition-only",
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
                "convergence":{
                    "state":"not_applicable","iterations":null,"residual":null,
                    "mesh_refinement_evaluated":false,
                    "details":"Synthetic relation test; no FEM solver executed."
                },
                "outputs":[{
                    "output_id":output_id,"kind":"displacement",
                    "sha256":sha256_hex(output_bytes),"media_type":"application/json",
                    "unit":"m","artifact_ref":artifact_ref
                }],
                "uncertainty":{"state":"unknown","method":null,"summary":"Synthetic fixture only."},
                "validity_envelope":{
                    "supported_geometry":"synthetic fixture",
                    "supported_material_model":"linear elasticity fixture",
                    "load_regime":"static point forces",
                    "limitations":["No external solver executed."]
                },
                "claim_ceiling":"Synthetic relation fixture only; not physical qualification.",
                "diagnostics":[]
            })).unwrap()
        }
        let receipt_a = receipt(&a, &request_a, "output-a", "a-displacement.json", &bytes_a);
        let receipt_b = receipt(&b, &request_b, "output-b", "b-displacement.json", &bytes_b);
        let receipt_combined = receipt(&combined, &request_combined, "output-combined", "combined-displacement.json", &bytes_combined);

        let manifest = json!({
            "schema_version":MANIFEST_SCHEMA,
            "relation_id":"synthetic-superposition-relation",
            "case_id":"superposition-synthetic-case",
            "request_a_sha256":sha_a,
            "request_b_sha256":sha_b,
            "request_combined_sha256":sha_combined,
            "absolute_tolerance":1e-12,
            "relative_tolerance":1e-10,
            "output_pairs":[{
                "output_kind":"displacement",
                "output_a_id":"output-a",
                "output_b_id":"output-b",
                "output_combined_id":"output-combined"
            }]
        });
        Bundle {
            root,
            request_a,
            receipt_a,
            request_b,
            receipt_b,
            request_combined,
            receipt_combined,
            manifest:serde_json::to_vec(&manifest).unwrap()
        }
    }

    fn assess(bundle: &Bundle) -> Value {
        assess_superposition_relation_bytes(
            &bundle.request_a, &bundle.receipt_a,
            &bundle.request_b, &bundle.receipt_b,
            &bundle.request_combined, &bundle.receipt_combined,
            &bundle.root, &bundle.manifest
        ).unwrap()
    }

    #[test]
    fn exact_synthetic_superposition_passes_without_qualifying_physics() {
        let report = assess(&make_bundle(0.02));
        assert_eq!(report["relation_status"], "PASS");
        assert_eq!(report["observable_count"], 2);
        assert_eq!(report["violation_count"], 0);
        assert_eq!(report["physical_result_qualified"], false);
        assert_eq!(report["real_world_validation_status"], "UNRESOLVED");
    }

    #[test]
    fn wrong_combined_response_fails_superposition() {
        let report = assess(&make_bundle(0.025));
        assert_eq!(report["relation_status"], "FAIL");
        assert_eq!(report["violation_count"], 1);
    }

    #[test]
    fn wrong_manifest_digest_is_unresolved() {
        let mut bundle = make_bundle(0.02);
        let mut manifest: Value = serde_json::from_slice(&bundle.manifest).unwrap();
        manifest["request_b_sha256"] = json!("d".repeat(64));
        bundle.manifest = serde_json::to_vec(&manifest).unwrap();
        assert_eq!(assess(&bundle)["relation_status"], "UNRESOLVED");
    }

    #[test]
    fn combined_load_inventory_must_equal_a_union_b() {
        let mut bundle = make_bundle(0.02);
        let mut combined: Value = serde_json::from_slice(&bundle.request_combined).unwrap();
        combined["loads"] = json!([combined["loads"][0].clone()]);
        bundle.request_combined = serde_json::to_vec(&combined).unwrap();
        let mut manifest: Value = serde_json::from_slice(&bundle.manifest).unwrap();
        manifest["request_combined_sha256"] = json!(sha256_hex(&bundle.request_combined));
        bundle.manifest = serde_json::to_vec(&manifest).unwrap();
        assert_eq!(assess(&bundle)["relation_status"], "UNRESOLVED");
    }

    #[test]
    fn duplicate_load_id_across_a_and_b_is_unresolved() {
        let mut bundle = make_bundle(0.02);
        let mut b: Value = serde_json::from_slice(&bundle.request_b).unwrap();
        b["loads"][0]["load_id"] = json!("force-a");
        bundle.request_b = serde_json::to_vec(&b).unwrap();
        let mut manifest: Value = serde_json::from_slice(&bundle.manifest).unwrap();
        manifest["request_b_sha256"] = json!(sha256_hex(&bundle.request_b));
        bundle.manifest = serde_json::to_vec(&manifest).unwrap();
        assert_eq!(assess(&bundle)["relation_status"], "UNRESOLVED");
    }

    #[test]
    fn material_drift_abstains_instead_of_scoring() {
        let mut bundle = make_bundle(0.02);
        let mut b: Value = serde_json::from_slice(&bundle.request_b).unwrap();
        b["materials"][0]["youngs_modulus_pa"] = json!(150000000000.0);
        bundle.request_b = serde_json::to_vec(&b).unwrap();
        let mut manifest: Value = serde_json::from_slice(&bundle.manifest).unwrap();
        manifest["request_b_sha256"] = json!(sha256_hex(&bundle.request_b));
        bundle.manifest = serde_json::to_vec(&manifest).unwrap();
        assert_eq!(assess(&bundle)["relation_status"], "UNRESOLVED");
    }

    #[test]
    fn nonlinear_fidelity_is_outside_superposition_envelope() {
        let mut bundle = make_bundle(0.02);
        let mut combined: Value = serde_json::from_slice(&bundle.request_combined).unwrap();
        combined["model_fidelity"] = json!("continuum_nonlinear");
        bundle.request_combined = serde_json::to_vec(&combined).unwrap();
        let mut manifest: Value = serde_json::from_slice(&bundle.manifest).unwrap();
        manifest["request_combined_sha256"] = json!(sha256_hex(&bundle.request_combined));
        bundle.manifest = serde_json::to_vec(&manifest).unwrap();
        assert_eq!(assess(&bundle)["relation_status"], "UNRESOLVED");
    }

    #[test]
    fn output_bytes_tamper_is_unresolved() {
        let bundle = make_bundle(0.02);
        fs::write(bundle.root.join("combined-displacement.json"), b"tampered").unwrap();
        let report = assess(&bundle);
        assert_eq!(report["relation_status"], "UNRESOLVED");
        assert_eq!(report["combined_artifact_verification_status"], "FAIL");
    }

    #[test]
    fn unit_disagreement_is_unresolved() {
        let mut bundle = make_bundle(0.02);
        let mut receipt: Value = serde_json::from_slice(&bundle.receipt_b).unwrap();
        receipt["outputs"][0]["unit"] = json!("mm");
        bundle.receipt_b = serde_json::to_vec(&receipt).unwrap();
        assert_eq!(assess(&bundle)["relation_status"], "UNRESOLVED");
    }
}
