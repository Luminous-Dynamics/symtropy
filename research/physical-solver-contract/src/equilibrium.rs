// SPDX-License-Identifier: AGPL-3.0-or-later
//! Receipt-bound static force and moment equilibrium checks.
//!
//! This module checks declared resultants for one explicitly bounded linear-static
//! envelope. It is an invariant audit, not a solver and not physical validation.

use crate::{Check, sha256_hex, verify_solver_output_artifact_bytes};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
};

const MANIFEST_SCHEMA: &str = "physical-equilibrium-manifest-v0.1";
const FORCE_SCHEMA: &str = "physical-equilibrium-reaction-forces-v0.1";
const MOMENT_SCHEMA: &str = "physical-equilibrium-reaction-moments-v0.1";
const MAX_REACTION_BYTES: u64 = 8 * 1024 * 1024;

type Vec3 = [f64; 3];

#[derive(Clone, Debug)]
struct AppliedLoad {
    id: String,
    target_id: String,
    force_n: Vec3,
    position_m: Vec3,
    couple_nm: Vec3,
}

#[derive(Clone, Debug)]
struct ReactionForce {
    id: String,
    force_n: Vec3,
    position_m: Vec3,
}

#[derive(Clone, Debug)]
struct ReactionMoment {
    id: String,
    moment_nm: Vec3,
}

#[derive(Clone, Debug)]
struct Manifest {
    audit_id: String,
    case_id: String,
    request_id: String,
    request_sha256: String,
    scene_sha256: String,
    scene_revision: u64,
    coordinate_frame_id: String,
    origin_m: Vec3,
    applied_loads: Vec<AppliedLoad>,
    required_force_ids: BTreeSet<String>,
    required_moment_ids: BTreeSet<String>,
    force_abs_tol_n: f64,
    force_rel_tol: f64,
    moment_abs_tol_nm: f64,
    moment_rel_tol: f64,
    reference_method: String,
    reference_identity: String,
    reference_source_ref: String,
}

/// Verify exact receipt-bound reaction artifacts, then audit force/moment balance.
///
/// The manifest is a separately frozen independent declaration. Its digest and exact
/// request identity are retained in the result, but a digest alone cannot prove when
/// the manifest was authored or that its oracle is independent.
pub fn assess_equilibrium_bytes(
    request_bytes: &[u8],
    receipt_bytes: &[u8],
    artifact_root: &Path,
    manifest_bytes: &[u8],
    reaction_force_output_id: &str,
    reaction_moment_output_id: &str,
) -> Check<Value> {
    let manifest_sha256 = sha256_hex(manifest_bytes);
    let request_sha256 = sha256_hex(request_bytes);
    let manifest = parse_manifest(manifest_bytes)?;
    let mut report = base_report(&manifest, &manifest_sha256, &request_sha256);

    let request: Value = match serde_json::from_slice(request_bytes) {
        Ok(value) => value,
        Err(error) => {
            return Ok(mark_unresolved(
                report,
                format!("invalid request JSON: {error}"),
            ));
        }
    };
    if let Err(reason) = validate_manifest_request_binding(&manifest, &request, &request_sha256) {
        return Ok(mark_unresolved(report, reason));
    }

    let required_outputs = match request.get("required_outputs").and_then(Value::as_array) {
        Some(values) => values,
        None => {
            return Ok(mark_unresolved(
                report,
                "request.required_outputs is missing or invalid",
            ));
        }
    };
    if !required_outputs
        .iter()
        .any(|item| item.as_str() == Some("reaction_force"))
        || !required_outputs
            .iter()
            .any(|item| item.as_str() == Some("reaction_moment"))
    {
        return Ok(mark_unresolved(
            report,
            "request must explicitly require both reaction_force and reaction_moment outputs",
        ));
    }

    let artifact_report =
        match verify_solver_output_artifact_bytes(request_bytes, receipt_bytes, artifact_root) {
            Ok(value) => value,
            Err(error) => {
                return Ok(mark_unresolved(
                    report,
                    format!("receipt/output verification could not complete: {error}"),
                ));
            }
        };
    report["artifact_verification_status"] = artifact_report
        .get("artifact_verification_status")
        .cloned()
        .unwrap_or(Value::Null);
    report["output_artifact_bytes_verified"] = artifact_report
        .get("output_artifact_bytes_verified")
        .cloned()
        .unwrap_or(Value::Bool(false));
    report["producer_status"] = artifact_report
        .get("producer_status")
        .cloned()
        .unwrap_or(Value::Null);
    if artifact_report["artifact_verification_status"] != "PASS"
        || artifact_report["output_artifact_bytes_verified"] != true
    {
        return Ok(mark_unresolved(
            report,
            "receipt-declared artifact byte verification did not PASS",
        ));
    }
    if artifact_report["producer_status"] != "PASS" {
        return Ok(mark_unresolved(
            report,
            "producer receipt status is not PASS; equilibrium is not scoreable",
        ));
    }

    let receipt: Value = match serde_json::from_slice(receipt_bytes) {
        Ok(value) => value,
        Err(error) => {
            return Ok(mark_unresolved(
                report,
                format!("invalid receipt JSON: {error}"),
            ));
        }
    };
    let outputs = match receipt.get("outputs").and_then(Value::as_array) {
        Some(values) => values,
        None => {
            return Ok(mark_unresolved(
                report,
                "receipt.outputs is missing or invalid",
            ));
        }
    };
    let force_output = match find_output(outputs, reaction_force_output_id, "reaction_force") {
        Ok(value) => value,
        Err(reason) => return Ok(mark_unresolved(report, reason)),
    };
    let moment_output = match find_output(outputs, reaction_moment_output_id, "reaction_moment") {
        Ok(value) => value,
        Err(reason) => return Ok(mark_unresolved(report, reason)),
    };

    let force_bytes = match read_verified_output(
        artifact_root,
        force_output,
        &artifact_report,
        reaction_force_output_id,
    ) {
        Ok(value) => value,
        Err(reason) => return Ok(mark_unresolved(report, reason)),
    };
    let moment_bytes = match read_verified_output(
        artifact_root,
        moment_output,
        &artifact_report,
        reaction_moment_output_id,
    ) {
        Ok(value) => value,
        Err(reason) => return Ok(mark_unresolved(report, reason)),
    };
    report["reaction_force_output_sha256"] =
        json!(string_field(force_output, "sha256", "receipt output")?);
    report["reaction_moment_output_sha256"] =
        json!(string_field(moment_output, "sha256", "receipt output")?);

    let forces = match parse_force_output(&force_bytes, &manifest) {
        Ok(value) => value,
        Err(reason) => return Ok(mark_unresolved(report, reason)),
    };
    let moments = match parse_moment_output(&moment_bytes, &manifest) {
        Ok(value) => value,
        Err(reason) => return Ok(mark_unresolved(report, reason)),
    };
    if let Err(reason) = validate_observed_ids(
        &manifest.required_force_ids,
        forces.iter().map(|item| item.id.as_str()),
        "reaction force",
    ) {
        return Ok(mark_unresolved(report, reason));
    }
    if let Err(reason) = validate_observed_ids(
        &manifest.required_moment_ids,
        moments.iter().map(|item| item.id.as_str()),
        "reaction moment",
    ) {
        return Ok(mark_unresolved(report, reason));
    }

    let calculated = match calculate_balance(&manifest, &forces, &moments) {
        Ok(value) => value,
        Err(reason) => return Ok(mark_unresolved(report, reason)),
    };
    let force = balance_dimension(
        calculated.force_residual,
        calculated.force_scale,
        manifest.force_abs_tol_n,
        manifest.force_rel_tol,
        "N",
    );
    let moment = balance_dimension(
        calculated.moment_residual,
        calculated.moment_scale,
        manifest.moment_abs_tol_nm,
        manifest.moment_rel_tol,
        "N*m",
    );
    if force["status"] == "UNRESOLVED" || moment["status"] == "UNRESOLVED" {
        return Ok(mark_unresolved(
            report,
            "force or moment residual calculation was non-finite",
        ));
    }

    let force_status = force["status"]
        .as_str()
        .unwrap_or("UNRESOLVED")
        .to_owned();
    let moment_status = moment["status"]
        .as_str()
        .unwrap_or("UNRESOLVED")
        .to_owned();
    let overall = if force_status == "PASS" && moment_status == "PASS" {
        "PASS"
    } else {
        "FAIL"
    };
    report["audit_status"] = json!(overall);
    report["force_balance"] = force;
    report["moment_balance"] = moment;
    report["applied_load_count"] = json!(manifest.applied_loads.len());
    report["reaction_force_count"] = json!(forces.len());
    report["reaction_moment_count"] = json!(moments.len());
    report["diagnostics"] = if overall == "PASS" {
        json!([])
    } else {
        json!([
            format!("force balance status: {force_status}"),
            format!("moment balance status: {moment_status}")
        ])
    };
    report["physical_result_qualified"] = json!(false);
    report["real_world_validation_status"] = json!("UNRESOLVED");
    report["claim_ceiling"] = json!(
        "declared static-equilibrium audit only; does not establish solver correctness, physical validation, structural safety, or intended-use qualification"
    );
    Ok(report)
}

struct BalanceCalculation {
    force_residual: Vec3,
    force_scale: f64,
    moment_residual: Vec3,
    moment_scale: f64,
}

fn calculate_balance(
    manifest: &Manifest,
    reactions: &[ReactionForce],
    reaction_moments: &[ReactionMoment],
) -> Check<BalanceCalculation> {
    let mut force_residual = [0.0; 3];
    let mut force_scale = 0.0;
    let mut moment_residual = [0.0; 3];
    let mut moment_scale = 0.0;

    for load in &manifest.applied_loads {
        add_in_place(&mut force_residual, load.force_n);
        force_scale += norm(load.force_n);
        let arm = subtract(load.position_m, manifest.origin_m);
        let point_moment = cross(arm, load.force_n);
        add_in_place(&mut moment_residual, point_moment);
        add_in_place(&mut moment_residual, load.couple_nm);
        moment_scale += norm(point_moment) + norm(load.couple_nm);
    }
    for reaction in reactions {
        add_in_place(&mut force_residual, reaction.force_n);
        force_scale += norm(reaction.force_n);
        let arm = subtract(reaction.position_m, manifest.origin_m);
        let point_moment = cross(arm, reaction.force_n);
        add_in_place(&mut moment_residual, point_moment);
        moment_scale += norm(point_moment);
    }
    for reaction in reaction_moments {
        add_in_place(&mut moment_residual, reaction.moment_nm);
        moment_scale += norm(reaction.moment_nm);
    }

    if !force_residual.iter().all(|value| value.is_finite())
        || !moment_residual.iter().all(|value| value.is_finite())
        || !force_scale.is_finite()
        || !moment_scale.is_finite()
    {
        return Err("force/moment accumulation overflowed or became non-finite".to_owned());
    }
    Ok(BalanceCalculation {
        force_residual,
        force_scale,
        moment_residual,
        moment_scale,
    })
}

fn balance_dimension(
    residual: Vec3,
    scale: f64,
    absolute_tolerance: f64,
    relative_tolerance: f64,
    unit: &str,
) -> Value {
    let residual_norm = norm(residual);
    let allowed = absolute_tolerance + relative_tolerance * scale;
    if !residual_norm.is_finite() || !allowed.is_finite() {
        return json!({
            "status": "UNRESOLVED",
            "residual_vector": residual,
            "residual_norm": null,
            "scale": scale,
            "normalized_residual": null,
            "normalized_residual_status": "overflow",
            "absolute_tolerance": absolute_tolerance,
            "relative_tolerance": relative_tolerance,
            "allowed_residual": null,
            "unit": unit
        });
    }
    let normalized = if scale == 0.0 {
        Value::Null
    } else {
        let ratio = residual_norm / scale;
        if ratio.is_finite() {
            json!(ratio)
        } else {
            Value::Null
        }
    };
    let normalized_status = if scale == 0.0 {
        "undefined_zero_scale"
    } else if normalized.is_null() {
        "overflow"
    } else {
        "computed"
    };
    let status = if normalized_status == "overflow" {
        "UNRESOLVED"
    } else if residual_norm <= allowed {
        "PASS"
    } else {
        "FAIL"
    };
    json!({
        "status": status,
        "residual_vector": residual,
        "residual_norm": residual_norm,
        "scale": scale,
        "normalized_residual": normalized,
        "normalized_residual_status": normalized_status,
        "absolute_tolerance": absolute_tolerance,
        "relative_tolerance": relative_tolerance,
        "allowed_residual": allowed,
        "unit": unit
    })
}

fn parse_manifest(bytes: &[u8]) -> Check<Manifest> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("invalid equilibrium manifest JSON: {error}"))?;
    exact_keys(
        &value,
        &[
            "schema_version",
            "audit_id",
            "case_id",
            "request_id",
            "request_sha256",
            "scene_sha256",
            "scene_revision",
            "coordinate_frame_id",
            "origin_m",
            "units",
            "reference",
            "applied_loads",
            "required_reaction_force_ids",
            "required_reaction_moment_ids",
            "tolerances",
        ],
        "manifest",
    )?;
    if string_field(&value, "schema_version", "manifest")? != MANIFEST_SCHEMA {
        return Err(format!(
            "manifest.schema_version must equal {MANIFEST_SCHEMA}"
        ));
    }
    let audit_id = id_field(&value, "audit_id", "manifest")?;
    let case_id = id_field(&value, "case_id", "manifest")?;
    let request_id = id_field(&value, "request_id", "manifest")?;
    let request_sha256 = string_field(&value, "request_sha256", "manifest")?.to_owned();
    validate_sha(&request_sha256, "manifest.request_sha256")?;
    let scene_sha256 = string_field(&value, "scene_sha256", "manifest")?.to_owned();
    validate_sha(&scene_sha256, "manifest.scene_sha256")?;
    let scene_revision = value
        .get("scene_revision")
        .and_then(Value::as_u64)
        .ok_or_else(|| "manifest.scene_revision must be a non-negative integer".to_owned())?;
    let coordinate_frame_id = id_field(&value, "coordinate_frame_id", "manifest")?;
    let origin_m = vec3_field(&value, "origin_m", "manifest")?;

    let units = value
        .get("units")
        .ok_or_else(|| "manifest.units is required".to_owned())?;
    exact_keys(units, &["length", "force", "moment"], "manifest.units")?;
    for (key, expected) in [("length", "m"), ("force", "N"), ("moment", "N*m")] {
        if string_field(units, key, "manifest.units")? != expected {
            return Err(format!("manifest.units.{key} must be '{expected}'"));
        }
    }
    let reference = value
        .get("reference")
        .ok_or_else(|| "manifest.reference is required".to_owned())?;
    exact_keys(
        reference,
        &["method", "identity", "source_ref"],
        "manifest.reference",
    )?;
    let reference_method = nonempty_field(reference, "method", "manifest.reference")?;
    let reference_identity = nonempty_field(reference, "identity", "manifest.reference")?;
    let reference_source_ref = nonempty_field(reference, "source_ref", "manifest.reference")?;

    let loads = value
        .get("applied_loads")
        .and_then(Value::as_array)
        .ok_or_else(|| "manifest.applied_loads must be an array".to_owned())?;
    if loads.is_empty() {
        return Err("manifest.applied_loads must not be empty".to_owned());
    }
    let mut applied_loads = Vec::with_capacity(loads.len());
    let mut load_ids = BTreeSet::new();
    for (index, load) in loads.iter().enumerate() {
        let at = format!("manifest.applied_loads[{index}]");
        exact_keys(
            load,
            &["load_id", "target_id", "force_n", "position_m", "couple_nm"],
            &at,
        )?;
        let id = id_field(load, "load_id", &at)?;
        if !load_ids.insert(id.clone()) {
            return Err(format!("{at}.load_id is duplicated"));
        }
        applied_loads.push(AppliedLoad {
            id,
            target_id: id_field(load, "target_id", &at)?,
            force_n: vec3_field(load, "force_n", &at)?,
            position_m: vec3_field(load, "position_m", &at)?,
            couple_nm: vec3_field(load, "couple_nm", &at)?,
        });
    }
    let required_force_ids = id_set(&value, "required_reaction_force_ids", "manifest")?;
    let required_moment_ids = id_set(&value, "required_reaction_moment_ids", "manifest")?;
    if required_force_ids.is_empty() || required_moment_ids.is_empty() {
        return Err(
            "manifest must require at least one reaction-force and reaction-moment ID".to_owned(),
        );
    }
    let tolerances = value
        .get("tolerances")
        .ok_or_else(|| "manifest.tolerances is required".to_owned())?;
    exact_keys(
        tolerances,
        &[
            "force_absolute_n",
            "force_relative",
            "moment_absolute_nm",
            "moment_relative",
        ],
        "manifest.tolerances",
    )?;
    let force_abs_tol_n = nonnegative_field(tolerances, "force_absolute_n", "manifest.tolerances")?;
    let force_rel_tol = nonnegative_field(tolerances, "force_relative", "manifest.tolerances")?;
    let moment_abs_tol_nm =
        nonnegative_field(tolerances, "moment_absolute_nm", "manifest.tolerances")?;
    let moment_rel_tol = nonnegative_field(tolerances, "moment_relative", "manifest.tolerances")?;

    Ok(Manifest {
        audit_id,
        case_id,
        request_id,
        request_sha256,
        scene_sha256,
        scene_revision,
        coordinate_frame_id,
        origin_m,
        applied_loads,
        required_force_ids,
        required_moment_ids,
        force_abs_tol_n,
        force_rel_tol,
        moment_abs_tol_nm,
        moment_rel_tol,
        reference_method,
        reference_identity,
        reference_source_ref,
    })
}

fn validate_manifest_request_binding(
    manifest: &Manifest,
    request: &Value,
    request_sha256: &str,
) -> Check<()> {
    let request_id = string_field(request, "request_id", "request")?;
    let scene_sha256 = string_field(request, "scene_sha256", "request")?;
    let scene_revision = request
        .get("scene_revision")
        .and_then(Value::as_u64)
        .ok_or_else(|| "request.scene_revision must be a non-negative integer".to_owned())?;
    let frame = string_field(request, "coordinate_frame_id", "request")?;
    if manifest.request_sha256 != request_sha256
        || manifest.request_id != request_id
        || manifest.scene_sha256 != scene_sha256
        || manifest.scene_revision != scene_revision
        || manifest.coordinate_frame_id != frame
    {
        return Err(
            "manifest does not bind the exact request, scene revision, digest and coordinate frame"
                .to_owned(),
        );
    }
    let units = request
        .get("units")
        .ok_or_else(|| "request.units missing".to_owned())?;
    if string_field(units, "length", "request.units")? != "m"
        || string_field(units, "force", "request.units")? != "N"
    {
        return Err("equilibrium auditor v0.1 supports only SI request units (m, N)".to_owned());
    }
    let request_loads = request
        .get("loads")
        .and_then(Value::as_array)
        .ok_or_else(|| "request.loads must be an array".to_owned())?;
    if request_loads.len() != manifest.applied_loads.len() {
        return Err(
            "manifest load inventory differs from the exact request load inventory".to_owned(),
        );
    }
    for applied in &manifest.applied_loads {
        let matching: Vec<&Value> = request_loads
            .iter()
            .filter(|load| load.get("load_id").and_then(Value::as_str) == Some(applied.id.as_str()))
            .collect();
        if matching.len() != 1 {
            return Err(format!(
                "manifest load '{}' does not map to exactly one request load",
                applied.id
            ));
        }
        let request_load = matching[0];
        if string_field(request_load, "kind", "request load")? != "point_force" {
            return Err(format!(
                "request load '{}' is outside the point-force-only equilibrium envelope",
                applied.id
            ));
        }
        if string_field(request_load, "target_id", "request load")? != applied.target_id
            || string_field(request_load, "coordinate_frame_id", "request load")?
                != manifest.coordinate_frame_id
        {
            return Err(format!(
                "manifest load '{}' target/frame differs from the request",
                applied.id
            ));
        }
        let vector = vec3_field(request_load, "vector_xyz", "request load")?;
        if vector != applied.force_n {
            return Err(format!(
                "manifest force for load '{}' differs from the exact request vector",
                applied.id
            ));
        }
    }
    Ok(())
}

fn find_output<'a>(outputs: &'a [Value], output_id: &str, expected_kind: &str) -> Check<&'a Value> {
    let selected: Vec<&Value> = outputs
        .iter()
        .filter(|output| output.get("output_id").and_then(Value::as_str) == Some(output_id))
        .collect();
    if selected.len() != 1 {
        return Err(format!(
            "receipt must contain exactly one output ID '{output_id}'"
        ));
    }
    let output = selected[0];
    if string_field(output, "kind", "receipt output")? != expected_kind {
        return Err(format!(
            "receipt output '{output_id}' must have kind '{expected_kind}'"
        ));
    }
    Ok(output)
}

fn read_verified_output(
    artifact_root: &Path,
    output: &Value,
    verification_report: &Value,
    output_id: &str,
) -> Result<Vec<u8>, String> {
    let results = verification_report
        .get("artifact_results")
        .and_then(Value::as_array)
        .ok_or_else(|| "artifact verifier omitted artifact_results".to_owned())?;
    let verified = results
        .iter()
        .find(|item| item.get("output_id").and_then(Value::as_str) == Some(output_id))
        .ok_or_else(|| {
            format!("output '{output_id}' is absent from the byte-verification report")
        })?;
    if verified.get("status").and_then(Value::as_str) != Some("PASS") {
        return Err(format!(
            "output '{output_id}' did not pass its initial byte verification"
        ));
    }
    let expected_sha = string_field(output, "sha256", "receipt output")?;
    if string_field(verified, "observed_sha256", "artifact verification entry")? != expected_sha {
        return Err(format!(
            "output '{output_id}' verifier digest disagrees with receipt digest"
        ));
    }

    let root = fs::canonicalize(artifact_root)
        .map_err(|error| format!("cannot canonicalize artifact root: {error}"))?;
    let reference = string_field(output, "artifact_ref", "receipt output")?;
    let relative = safe_relative_path(reference)?;
    let candidate = root.join(relative);
    let canonical = fs::canonicalize(&candidate)
        .map_err(|error| format!("cannot resolve output '{output_id}': {error}"))?;
    if !canonical.starts_with(&root) {
        return Err(format!(
            "output '{output_id}' resolves outside artifact root"
        ));
    }
    let metadata = fs::metadata(&canonical)
        .map_err(|error| format!("cannot inspect output '{output_id}': {error}"))?;
    if !metadata.is_file() || metadata.len() > MAX_REACTION_BYTES {
        return Err(format!(
            "output '{output_id}' is not a regular file within the 8 MiB budget"
        ));
    }
    let bytes = fs::read(&canonical)
        .map_err(|error| format!("cannot read output '{output_id}': {error}"))?;
    if sha256_hex(&bytes) != expected_sha {
        return Err(format!(
            "output '{output_id}' changed after initial verification"
        ));
    }
    Ok(bytes)
}

fn parse_force_output(bytes: &[u8], manifest: &Manifest) -> Check<Vec<ReactionForce>> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("reaction-force output is invalid JSON: {error}"))?;
    validate_output_identity(&value, FORCE_SCHEMA, manifest, "reaction-force output")?;
    let records = value
        .get("reaction_forces")
        .and_then(Value::as_array)
        .ok_or_else(|| "reaction_forces must be an array".to_owned())?;
    let mut ids = BTreeSet::new();
    let mut result = Vec::with_capacity(records.len());
    for (index, record) in records.iter().enumerate() {
        let at = format!("reaction_forces[{index}]");
        exact_keys(record, &["reaction_id", "force_n", "position_m"], &at)?;
        let id = id_field(record, "reaction_id", &at)?;
        if !ids.insert(id.clone()) {
            return Err(format!("{at}.reaction_id is duplicated"));
        }
        result.push(ReactionForce {
            id,
            force_n: vec3_field(record, "force_n", &at)?,
            position_m: vec3_field(record, "position_m", &at)?,
        });
    }
    Ok(result)
}

fn parse_moment_output(bytes: &[u8], manifest: &Manifest) -> Check<Vec<ReactionMoment>> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("reaction-moment output is invalid JSON: {error}"))?;
    validate_output_identity(&value, MOMENT_SCHEMA, manifest, "reaction-moment output")?;
    let records = value
        .get("reaction_moments")
        .and_then(Value::as_array)
        .ok_or_else(|| "reaction_moments must be an array".to_owned())?;
    let mut ids = BTreeSet::new();
    let mut result = Vec::with_capacity(records.len());
    for (index, record) in records.iter().enumerate() {
        let at = format!("reaction_moments[{index}]");
        exact_keys(record, &["reaction_id", "moment_nm"], &at)?;
        let id = id_field(record, "reaction_id", &at)?;
        if !ids.insert(id.clone()) {
            return Err(format!("{at}.reaction_id is duplicated"));
        }
        result.push(ReactionMoment {
            id,
            moment_nm: vec3_field(record, "moment_nm", &at)?,
        });
    }
    Ok(result)
}

fn validate_output_identity(
    value: &Value,
    schema: &str,
    manifest: &Manifest,
    at: &str,
) -> Check<()> {
    let mut required = vec![
        "schema_version",
        "case_id",
        "request_id",
        "request_sha256",
        "scene_sha256",
        "scene_revision",
        "coordinate_frame_id",
        "units",
    ];
    required.push(if schema == FORCE_SCHEMA {
        "reaction_forces"
    } else {
        "reaction_moments"
    });
    exact_keys(value, &required, at)?;
    if string_field(value, "schema_version", at)? != schema
        || string_field(value, "case_id", at)? != manifest.case_id
        || string_field(value, "request_id", at)? != manifest.request_id
        || string_field(value, "request_sha256", at)? != manifest.request_sha256
        || string_field(value, "scene_sha256", at)? != manifest.scene_sha256
        || value.get("scene_revision").and_then(Value::as_u64) != Some(manifest.scene_revision)
        || string_field(value, "coordinate_frame_id", at)? != manifest.coordinate_frame_id
    {
        return Err(format!(
            "{at} identity/frame does not match the frozen manifest"
        ));
    }
    let units = value
        .get("units")
        .ok_or_else(|| format!("{at}.units missing"))?;
    let (required, force_or_moment) = if schema == FORCE_SCHEMA {
        (
            vec!["length", "force"],
            vec![("length", "m"), ("force", "N")],
        )
    } else {
        (
            vec!["length", "moment"],
            vec![("length", "m"), ("moment", "N*m")],
        )
    };
    exact_keys(units, &required, &format!("{at}.units"))?;
    for (key, expected) in force_or_moment {
        if string_field(units, key, &format!("{at}.units"))? != expected {
            return Err(format!("{at}.units.{key} must be '{expected}'"));
        }
    }
    Ok(())
}

fn validate_observed_ids<'a>(
    required: &BTreeSet<String>,
    observed: impl Iterator<Item = &'a str>,
    label: &str,
) -> Check<()> {
    let values: BTreeSet<&str> = observed.collect();
    let required_borrowed: BTreeSet<&str> = required.iter().map(String::as_str).collect();
    if values != required_borrowed {
        let missing: Vec<&str> = required_borrowed.difference(&values).copied().collect();
        let extra: Vec<&str> = values.difference(&required_borrowed).copied().collect();
        return Err(format!(
            "{label} inventory mismatch; missing={missing:?}, extra={extra:?}"
        ));
    }
    Ok(())
}

fn base_report(manifest: &Manifest, manifest_sha256: &str, request_sha256: &str) -> Value {
    json!({
        "schema_version": "physical-equilibrium-audit-report-v0.1",
        "audit_status": "UNRESOLVED",
        "audit_id": manifest.audit_id,
        "case_id": manifest.case_id,
        "request_id": manifest.request_id,
        "request_sha256": request_sha256,
        "manifest_sha256": manifest_sha256,
        "scene_sha256": manifest.scene_sha256,
        "scene_revision": manifest.scene_revision,
        "coordinate_frame_id": manifest.coordinate_frame_id,
        "moment_origin_m": manifest.origin_m,
        "reference_method": manifest.reference_method,
        "reference_identity": manifest.reference_identity,
        "reference_source_ref": manifest.reference_source_ref,
        "artifact_verification_status": "UNRESOLVED",
        "output_artifact_bytes_verified": false,
        "producer_status": "UNRESOLVED",
        "physical_result_qualified": false,
        "real_world_validation_status": "UNRESOLVED",
        "diagnostics": [],
        "claim_ceiling": "equilibrium audit unresolved; no solver or physical qualification is implied"
    })
}

fn mark_unresolved(mut report: Value, reason: impl Into<String>) -> Value {
    let reason = reason.into();
    report["audit_status"] = json!("UNRESOLVED");
    report["diagnostics"] = json!([reason]);
    report["physical_result_qualified"] = json!(false);
    report["real_world_validation_status"] = json!("UNRESOLVED");
    report
}

fn exact_keys(value: &Value, expected: &[&str], at: &str) -> Check<()> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{at} must be an object"))?;
    for key in expected {
        if !object.contains_key(*key) {
            return Err(format!("{at}.{key} is required"));
        }
    }
    if let Some(key) = object.keys().find(|key| !expected.contains(&key.as_str())) {
        return Err(format!("{at} contains unsupported field '{key}'"));
    }
    Ok(())
}

fn string_field<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{at}.{key} must be a string"))
}

fn nonempty_field(value: &Value, key: &str, at: &str) -> Check<String> {
    let text = string_field(value, key, at)?;
    if text.trim().is_empty() {
        return Err(format!("{at}.{key} must not be empty"));
    }
    Ok(text.to_owned())
}

fn id_field(value: &Value, key: &str, at: &str) -> Check<String> {
    let text = string_field(value, key, at)?;
    if text.is_empty()
        || text.len() > 128
        || !text
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':'))
    {
        return Err(format!("{at}.{key} is not a valid stable ID"));
    }
    Ok(text.to_owned())
}

fn id_set(value: &Value, key: &str, at: &str) -> Check<BTreeSet<String>> {
    let values = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{at}.{key} must be an array"))?;
    let mut result = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        let text = value
            .as_str()
            .ok_or_else(|| format!("{at}.{key}[{index}] must be a string"))?;
        if text.is_empty()
            || text.len() > 128
            || !text
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':'))
        {
            return Err(format!("{at}.{key}[{index}] is not a valid stable ID"));
        }
        if !result.insert(text.to_owned()) {
            return Err(format!("{at}.{key} contains duplicate ID '{text}'"));
        }
    }
    Ok(result)
}

fn validate_sha(value: &str, label: &str) -> Check<()> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "{label} must be 64 lowercase hexadecimal characters"
        ));
    }
    Ok(())
}

fn nonnegative_field(value: &Value, key: &str, at: &str) -> Check<f64> {
    let number = value
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("{at}.{key} must be numeric"))?;
    if !number.is_finite() || number < 0.0 {
        return Err(format!("{at}.{key} must be finite and non-negative"));
    }
    Ok(number)
}

fn vec3_field(value: &Value, key: &str, at: &str) -> Check<Vec3> {
    let values = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{at}.{key} must be a three-component vector"))?;
    if values.len() != 3 {
        return Err(format!("{at}.{key} must contain exactly three components"));
    }
    let mut vector = [0.0; 3];
    for (index, item) in values.iter().enumerate() {
        let number = item
            .as_f64()
            .ok_or_else(|| format!("{at}.{key}[{index}] must be numeric"))?;
        if !number.is_finite() {
            return Err(format!("{at}.{key}[{index}] must be finite"));
        }
        vector[index] = number;
    }
    Ok(vector)
}

fn safe_relative_path(reference: &str) -> Check<PathBuf> {
    if reference.is_empty()
        || reference.trim() != reference
        || reference.chars().any(|ch| matches!(ch, '\\' | ':' | '\0'))
        || reference
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err("artifact_ref must be a normalized relative POSIX path".to_owned());
    }
    let path = Path::new(reference);
    if path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("artifact_ref must stay below the artifact root".to_owned());
    }
    Ok(path.to_path_buf())
}

fn add_in_place(target: &mut Vec3, value: Vec3) {
    for index in 0..3 {
        target[index] += value[index];
    }
}

fn subtract(left: Vec3, right: Vec3) -> Vec3 {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}

fn cross(left: Vec3, right: Vec3) -> Vec3 {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn norm(vector: Vec3) -> f64 {
    vector[0].hypot(vector[1]).hypot(vector[2])
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
    const REV: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    struct Bundle {
        root: PathBuf,
        request: Vec<u8>,
        receipt: Vec<u8>,
        manifest: Vec<u8>,
    }

    impl Drop for Bundle {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn make_bundle(force_n: Vec3, moment_nm: Vec3) -> Bundle {
        let request_value: Value = serde_json::from_str(include_str!(
            "../../../schemas/physical-transformation/fixtures/analytical-bar-request-v0.1.json"
        ))
        .unwrap();
        let mut request_value = request_value;
        request_value["loads"][0]["target_id"] = json!("cantilever-tip");
        request_value["loads"][0]["vector_xyz"] = json!([0.0, -1000.0, 0.0]);
        request_value["required_outputs"] = json!(["reaction_force", "reaction_moment"]);
        let request = serde_json::to_vec(&request_value).unwrap();
        let request_sha = sha256_hex(&request);
        let request_id = request_value["request_id"].as_str().unwrap();
        let scene_sha = request_value["scene_sha256"].as_str().unwrap();
        let scene_revision = request_value["scene_revision"].as_u64().unwrap();
        let frame = request_value["coordinate_frame_id"].as_str().unwrap();
        let case_id = "cantilever-equilibrium-synthetic";

        let forces = json!({
            "schema_version": FORCE_SCHEMA,
            "case_id": case_id,
            "request_id": request_id,
            "request_sha256": request_sha,
            "scene_sha256": scene_sha,
            "scene_revision": scene_revision,
            "coordinate_frame_id": frame,
            "units": {"length":"m","force":"N"},
            "reaction_forces": [{
                "reaction_id":"fixed-support",
                "force_n":force_n,
                "position_m":[0.0,0.0,0.0]
            }]
        });
        let moments = json!({
            "schema_version": MOMENT_SCHEMA,
            "case_id": case_id,
            "request_id": request_id,
            "request_sha256": request_sha,
            "scene_sha256": scene_sha,
            "scene_revision": scene_revision,
            "coordinate_frame_id": frame,
            "units": {"length":"m","moment":"N*m"},
            "reaction_moments": [{
                "reaction_id":"fixed-support",
                "moment_nm":moment_nm
            }]
        });

        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "symtropy-equilibrium-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        let force_bytes = serde_json::to_vec(&forces).unwrap();
        let moment_bytes = serde_json::to_vec(&moments).unwrap();
        fs::write(root.join("reaction-forces.json"), &force_bytes).unwrap();
        fs::write(root.join("reaction-moments.json"), &moment_bytes).unwrap();
        let receipt_value = json!({
            "schema_version":"physical-solver-receipt-v0.1",
            "request_id":request_id,
            "request_sha256":request_sha,
            "source_revision":request_value["source_revision"],
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
            "convergence":{
                "state":"not_applicable",
                "iterations":null,
                "residual":null,
                "mesh_refinement_evaluated":false,
                "details":"Synthetic test fixture; no solver executed."
            },
            "outputs":[
                {
                    "output_id":"reaction-forces",
                    "kind":"reaction_force",
                    "sha256":sha256_hex(&force_bytes),
                    "media_type":"application/json",
                    "unit":"N",
                    "artifact_ref":"reaction-forces.json"
                },
                {
                    "output_id":"reaction-moments",
                    "kind":"reaction_moment",
                    "sha256":sha256_hex(&moment_bytes),
                    "media_type":"application/json",
                    "unit":"N*m",
                    "artifact_ref":"reaction-moments.json"
                }
            ],
            "uncertainty":{"state":"unknown","method":null,"summary":"Synthetic fixture only."},
            "validity_envelope":{
                "supported_geometry":"synthetic fixture only",
                "supported_material_model":"synthetic fixture only",
                "load_regime":"synthetic fixture only",
                "limitations":["No solver was executed."]
            },
            "claim_ceiling":"Synthetic fixture only; not physical qualification.",
            "diagnostics":[]
        });
        let receipt = serde_json::to_vec(&receipt_value).unwrap();

        let manifest_value = json!({
            "schema_version":MANIFEST_SCHEMA,
            "audit_id":"fixture-equilibrium-audit",
            "case_id":case_id,
            "request_id":request_id,
            "request_sha256":request_sha,
            "scene_sha256":scene_sha,
            "scene_revision":scene_revision,
            "coordinate_frame_id":frame,
            "origin_m":[0.0,0.0,0.0],
            "units":{"length":"m","force":"N","moment":"N*m"},
            "reference":{
                "method":"hand_calculated_synthetic_free_body",
                "identity":"tip force, fixed support force and support couple",
                "source_ref":"test-only:equilibrium-oracle-v1"
            },
            "applied_loads":[{
                "load_id":"tension-001",
                "target_id":"cantilever-tip",
                "force_n":[0.0,-1000.0,0.0],
                "position_m":[2.0,0.0,0.0],
                "couple_nm":[0.0,0.0,0.0]
            }],
            "required_reaction_force_ids":["fixed-support"],
            "required_reaction_moment_ids":["fixed-support"],
            "tolerances":{
                "force_absolute_n":1e-9,
                "force_relative":1e-12,
                "moment_absolute_nm":1e-9,
                "moment_relative":1e-12
            }
        });
        let manifest = serde_json::to_vec(&manifest_value).unwrap();
        Bundle {
            root,
            request,
            receipt,
            manifest,
        }
    }

    fn assess(bundle: &Bundle) -> Value {
        assess_equilibrium_bytes(
            &bundle.request,
            &bundle.receipt,
            &bundle.root,
            &bundle.manifest,
            "reaction-forces",
            "reaction-moments",
        )
        .unwrap()
    }

    #[test]
    fn balanced_synthetic_free_body_passes_without_qualifying_physics() {
        let bundle = make_bundle([0.0, 1000.0, 0.0], [0.0, 0.0, 2000.0]);
        let report = assess(&bundle);
        assert_eq!(report["audit_status"], "PASS");
        assert_eq!(report["force_balance"]["residual_norm"], 0.0);
        assert_eq!(report["moment_balance"]["residual_norm"], 0.0);
        assert_eq!(report["physical_result_qualified"], false);
        assert_eq!(report["real_world_validation_status"], "UNRESOLVED");
    }

    #[test]
    fn inverted_reaction_force_fails_with_expected_residual() {
        let bundle = make_bundle([0.0, -1000.0, 0.0], [0.0, 0.0, 2000.0]);
        let report = assess(&bundle);
        assert_eq!(report["audit_status"], "FAIL");
        assert_eq!(report["force_balance"]["residual_norm"], 2000.0);
    }

    #[test]
    fn omitted_support_couple_fails_with_expected_moment_residual() {
        let bundle = make_bundle([0.0, 1000.0, 0.0], [0.0, 0.0, 0.0]);
        let report = assess(&bundle);
        assert_eq!(report["audit_status"], "FAIL");
        assert_eq!(report["moment_balance"]["residual_norm"], 2000.0);
    }

    #[test]
    fn missing_reaction_record_is_unresolved_not_zero() {
        let mut bundle = make_bundle([0.0, 1000.0, 0.0], [0.0, 0.0, 2000.0]);
        let force_path = bundle.root.join("reaction-forces.json");
        let mut value: Value = serde_json::from_slice(&fs::read(&force_path).unwrap()).unwrap();
        value["reaction_forces"] = json!([]);
        let bytes = serde_json::to_vec(&value).unwrap();
        fs::write(&force_path, &bytes).unwrap();
        let mut receipt: Value = serde_json::from_slice(&bundle.receipt).unwrap();
        receipt["outputs"][0]["sha256"] = json!(sha256_hex(&bytes));
        bundle.receipt = serde_json::to_vec(&receipt).unwrap();
        let report = assess(&bundle);
        assert_eq!(report["audit_status"], "UNRESOLVED");
        assert_eq!(report["physical_result_qualified"], false);
    }

    #[test]
    fn tampered_reaction_bytes_cannot_pass() {
        let bundle = make_bundle([0.0, 1000.0, 0.0], [0.0, 0.0, 2000.0]);
        fs::write(bundle.root.join("reaction-forces.json"), b"tampered").unwrap();
        let report = assess(&bundle);
        assert_eq!(report["audit_status"], "UNRESOLVED");
        assert_eq!(report["artifact_verification_status"], "FAIL");
    }

    #[test]
    fn wrong_frame_or_unit_is_unresolved() {
        let mut bundle = make_bundle([0.0, 1000.0, 0.0], [0.0, 0.0, 2000.0]);
        let path = bundle.root.join("reaction-forces.json");
        let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        value["units"]["force"] = json!("kN");
        let bytes = serde_json::to_vec(&value).unwrap();
        fs::write(&path, &bytes).unwrap();
        let mut receipt: Value = serde_json::from_slice(&bundle.receipt).unwrap();
        receipt["outputs"][0]["sha256"] = json!(sha256_hex(&bytes));
        bundle.receipt = serde_json::to_vec(&receipt).unwrap();
        let report = assess(&bundle);
        assert_eq!(report["audit_status"], "UNRESOLVED");
    }

    #[test]
    fn translating_origin_and_all_points_preserves_force_and_moment_balance() {
        let bundle = make_bundle([0.0, 1000.0, 0.0], [0.0, 0.0, 2000.0]);
        let base = assess(&bundle);
        let shift = [4.0, -3.0, 2.0];
        let mut manifest: Value = serde_json::from_slice(&bundle.manifest).unwrap();
        manifest["origin_m"] = json!(shift);
        let manifest_bytes = serde_json::to_vec(&manifest).unwrap();
        let translated = assess_equilibrium_bytes(
            &bundle.request,
            &bundle.receipt,
            &bundle.root,
            &manifest_bytes,
            "reaction-forces",
            "reaction-moments",
        )
        .unwrap();
        assert_eq!(base["audit_status"], translated["audit_status"]);
        assert_eq!(
            base["force_balance"]["residual_norm"],
            translated["force_balance"]["residual_norm"]
        );
        assert!(
            (base["moment_balance"]["residual_norm"].as_f64().unwrap()
                - translated["moment_balance"]["residual_norm"]
                    .as_f64()
                    .unwrap())
            .abs()
                < 1e-12
        );
    }

    #[test]
    fn mismatched_request_manifest_binding_is_unresolved() {
        let bundle = make_bundle([0.0, 1000.0, 0.0], [0.0, 0.0, 2000.0]);
        let mut manifest: Value = serde_json::from_slice(&bundle.manifest).unwrap();
        manifest["request_sha256"] = json!("d".repeat(64));
        let bytes = serde_json::to_vec(&manifest).unwrap();
        let report = assess_equilibrium_bytes(
            &bundle.request,
            &bundle.receipt,
            &bundle.root,
            &bytes,
            "reaction-forces",
            "reaction-moments",
        )
        .unwrap();
        assert_eq!(report["audit_status"], "UNRESOLVED");
    }
}
