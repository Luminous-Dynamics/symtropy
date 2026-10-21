// SPDX-License-Identifier: AGPL-3.0-or-later
//! Validate request contracts; never mistake request validation for physics.
//!
//! Every receipt produced here has physical_result_status=UNRESOLVED because
//! this crate does not invoke a qualified mechanics solver.

pub mod calculix_displacement;
pub mod calculix_rf_restricted;
pub mod calculix_rr;
pub mod equilibrium;
pub mod mesh_convergence;
pub mod reference_compare;

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::Read,
    path::{Component, Path, PathBuf},
};

pub type Check<T> = Result<T, String>;
const REQUEST_SCHEMA: &str = "physical-solver-request-v0.1";
const RECEIPT_SCHEMA: &str = "physical-solver-validation-receipt-v0.1";
const SOLVER_RESULT_RECEIPT_SCHEMA: &str = "physical-solver-receipt-v0.1";
const PHYSICS: &[&str] = &[
    "topology",
    "static_linear_elastic",
    "dynamic_response",
    "nonlinear_material",
    "fracture",
    "rigid_contact",
    "granular_contact",
    "thermal_coupling",
];
const OUTPUTS: &[&str] = &[
    "topology",
    "displacement",
    "stress",
    "reaction_force",
    "reaction_moment",
    "crack_path",
    "fragment_state",
    "energy_balance",
];
const FIDELITIES: &[&str] = &[
    "graph_topology",
    "analytical_linear_elastic",
    "continuum_linear_elastic",
    "continuum_nonlinear",
    "phase_field_fracture",
    "rigid_contact",
    "granular_dem",
];

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

/// Hash a file incrementally so mesh and field outputs need not fit in RAM.
pub(crate) fn sha256_file(path: &Path) -> Check<(String, u64)> {
    let mut file = File::open(path).map_err(|error| format!("open failed: {error}"))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut byte_length = 0_u64;
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| format!("read failed: {error}"))?;
        if count == 0 {
            break;
        }
        byte_length = byte_length
            .checked_add(count as u64)
            .ok_or_else(|| "artifact byte length overflow".to_owned())?;
        hasher.update(&buffer[..count]);
    }
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    Ok((out, byte_length))
}

pub fn validate_request_bytes(bytes: &[u8], validator_revision: &str) -> Check<Value> {
    validate_hex(validator_revision, 40, "validator revision")?;
    let request: Value =
        serde_json::from_slice(bytes).map_err(|error| format!("invalid request JSON: {error}"))?;
    validate_shape(&request)?;

    let request_id = string(&request, "request_id", "request")?;
    let source_revision = string(&request, "source_revision", "request")?;
    let scene_sha256 = string(&request, "scene_sha256", "request")?;
    let fidelity = string(&request, "model_fidelity", "request")?;
    let top_frame = string(&request, "coordinate_frame_id", "request")?;
    let requested_physics = strings(&request, "requested_physics", "request")?;
    let required_outputs = strings(&request, "required_outputs", "request")?;
    let materials = array(&request, "materials", "request")?;
    let loads = array(&request, "loads", "request")?;
    let boundary_conditions = array(&request, "boundary_conditions", "request")?;

    let allowed: BTreeSet<&str> = match fidelity.as_str() {
        "graph_topology" => ["topology"].into_iter().collect(),
        "analytical_linear_elastic" => ["topology", "static_linear_elastic"].into_iter().collect(),
        _ => BTreeSet::new(),
    };
    let physics_omitted: Vec<String> = requested_physics
        .iter()
        .filter(|physics| !allowed.contains(physics.as_str()))
        .cloned()
        .collect();
    let mut diagnostics = Vec::<String>::new();

    if !["graph_topology", "analytical_linear_elastic"].contains(&fidelity.as_str()) {
        diagnostics.push(format!(
            "no qualified reference adapter exists for model fidelity '{fidelity}'"
        ));
    }
    for physics in &physics_omitted {
        diagnostics.push(format!(
            "requested physics '{physics}' is not supported by this reference capability"
        ));
    }
    if fidelity == "analytical_linear_elastic" {
        if materials.is_empty() {
            diagnostics.push("analytical request has no material declarations".to_owned());
        }
        if loads.is_empty() {
            diagnostics.push("analytical request has no loads".to_owned());
        }
        if boundary_conditions.is_empty() {
            diagnostics.push("analytical request has no boundary conditions".to_owned());
        }
        for material in materials {
            for key in ["youngs_modulus_pa", "poisson_ratio", "density_kg_m3"] {
                if material.get(key).is_none_or(Value::is_null) {
                    diagnostics.push(format!(
                        "material '{}' has unresolved '{key}'",
                        material
                            .get("material_id")
                            .and_then(Value::as_str)
                            .unwrap_or("<unknown>")
                    ));
                }
            }
        }
    }
    for (i, load) in loads.iter().enumerate() {
        let frame = string(load, "coordinate_frame_id", &format!("request.loads[{i}]"))?;
        if frame != top_frame {
            diagnostics.push(format!(
                "load {i} frame '{frame}' differs from '{top_frame}' and has no declared transform"
            ));
        }
    }
    if fidelity == "graph_topology" && required_outputs.iter().any(|v| v != "topology") {
        diagnostics.push("graph_topology cannot produce non-topological outputs".to_owned());
    }
    // This is a request validator, not a physics backend. It never emits PASS
    // for the physical result, even if the request contract itself is valid.
    diagnostics
        .push("no solver was invoked; request validation is not a physical result".to_owned());

    let capability_status = if physics_omitted.is_empty()
        && diagnostics.len() == 1
        && ["graph_topology", "analytical_linear_elastic"].contains(&fidelity.as_str())
    {
        "WITHIN_REFERENCE_REQUEST_ENVELOPE"
    } else {
        "UNRESOLVED"
    };

    Ok(json!({
        "schema_version": RECEIPT_SCHEMA,
        "request_validation": "PASS",
        "request_id": request_id,
        "request_sha256": sha256_hex(bytes),
        "source_revision": source_revision,
        "scene_sha256": scene_sha256,
        "validator_revision": validator_revision,
        "model_fidelity": fidelity,
        "requested_physics": requested_physics,
        "physics_omitted": physics_omitted,
        "capability_status": capability_status,
        "solver_invoked": false,
        "physical_result_status": "UNRESOLVED",
        "required_outputs": required_outputs,
        "diagnostics": diagnostics,
        "claim_ceiling": "request contract validation only; no solver execution, structural qualification, or collapse prediction"
    }))
}

fn validate_shape(request: &Value) -> Check<()> {
    exact_keys(
        request,
        &[
            "schema_version",
            "request_id",
            "source_revision",
            "scene_revision",
            "scene_sha256",
            "model_fidelity",
            "requested_physics",
            "coordinate_frame_id",
            "units",
            "materials",
            "loads",
            "boundary_conditions",
            "required_outputs",
            "acceptance",
        ],
        "request",
    )?;
    if string(request, "schema_version", "request")? != REQUEST_SCHEMA {
        return Err(format!("request.schema_version must be '{REQUEST_SCHEMA}'"));
    }
    token(&string(request, "request_id", "request")?, "request ID")?;
    validate_hex(
        &string(request, "source_revision", "request")?,
        40,
        "source revision",
    )?;
    let _ = request
        .get("scene_revision")
        .and_then(Value::as_u64)
        .ok_or_else(|| "request.scene_revision must be a non-negative integer".to_owned())?;
    validate_hex(
        &string(request, "scene_sha256", "request")?,
        64,
        "scene SHA-256",
    )?;
    let fidelity = string(request, "model_fidelity", "request")?;
    if !FIDELITIES.contains(&fidelity.as_str()) {
        return Err(format!("unknown model fidelity '{fidelity}'"));
    }
    let physics = strings(request, "requested_physics", "request")?;
    enum_array(&physics, PHYSICS, "request.requested_physics")?;
    let outputs = strings(request, "required_outputs", "request")?;
    enum_array(&outputs, OUTPUTS, "request.required_outputs")?;
    token(
        &string(request, "coordinate_frame_id", "request")?,
        "coordinate frame",
    )?;

    let units = object(request, "units", "request")?;
    allowed_keys(
        units,
        &["length", "mass", "time", "force", "stress"],
        &[],
        "request.units",
    )?;
    for (key, required) in [
        ("length", "m"),
        ("mass", "kg"),
        ("time", "s"),
        ("force", "N"),
        ("stress", "Pa"),
    ] {
        if string_map(units, key, "request.units")? != required {
            return Err(format!("request.units.{key} must be '{required}'"));
        }
    }

    validate_materials(array(request, "materials", "request")?)?;
    validate_loads(array(request, "loads", "request")?)?;
    validate_boundary_conditions(array(request, "boundary_conditions", "request")?)?;
    validate_acceptance(object(request, "acceptance", "request")?)?;
    Ok(())
}

fn validate_materials(items: &[Value]) -> Check<()> {
    let mut ids = BTreeSet::new();
    for (i, item) in items.iter().enumerate() {
        let at = format!("request.materials[{i}]");
        exact_keys(
            item,
            &[
                "material_id",
                "component_ids",
                "youngs_modulus_pa",
                "poisson_ratio",
                "density_kg_m3",
                "provenance",
            ],
            &at,
        )?;
        let id = string(item, "material_id", &at)?;
        token(&id, "material ID")?;
        if !ids.insert(id) {
            return Err(format!("{at} has duplicate material ID"));
        }
        let components = strings(item, "component_ids", &at)?;
        if components.is_empty() {
            return Err(format!("{at}.component_ids must not be empty"));
        }
        unique(&components, &format!("{at}.component_ids"))?;
        for key in ["youngs_modulus_pa", "poisson_ratio", "density_kg_m3"] {
            let value = item.get(key).ok_or_else(|| format!("{at} missing {key}"))?;
            if !value.is_null() {
                let n = value
                    .as_f64()
                    .ok_or_else(|| format!("{at}.{key} must be number or null"))?;
                if !n.is_finite() {
                    return Err(format!("{at}.{key} must be finite"));
                }
                if ["youngs_modulus_pa", "density_kg_m3"].contains(&key) && n <= 0.0 {
                    return Err(format!("{at}.{key} must be positive"));
                }
                if key == "poisson_ratio" && !(-1.0..0.5).contains(&n) {
                    return Err(format!("{at}.poisson_ratio must satisfy -1 <= nu < 0.5"));
                }
            }
        }
        let provenance = object(item, "provenance", &at)?;
        allowed_keys(
            provenance,
            &["evidence_class", "source_ref"],
            &["uncertainty_relative"],
            &format!("{at}.provenance"),
        )?;
        let evidence = string_map(provenance, "evidence_class", &format!("{at}.provenance"))?;
        if ![
            "synthetic",
            "analytical_reference",
            "measured",
            "derived",
            "unknown",
        ]
        .contains(&evidence.as_str())
        {
            return Err(format!("{at}.provenance.evidence_class unknown"));
        }
        if provenance
            .get("source_ref")
            .is_none_or(|v| !(v.is_null() || v.is_string()))
        {
            return Err(format!("{at}.provenance.source_ref must be string or null"));
        }
        if let Some(value) = provenance
            .get("uncertainty_relative")
            .filter(|v| !v.is_null())
            && value.as_f64().is_none_or(|n| !n.is_finite() || n < 0.0)
        {
            return Err(format!(
                "{at}.provenance.uncertainty_relative must be finite and non-negative"
            ));
        }
    }
    Ok(())
}

fn validate_loads(items: &[Value]) -> Check<()> {
    let mut ids = BTreeSet::new();
    for (i, item) in items.iter().enumerate() {
        let at = format!("request.loads[{i}]");
        exact_keys(
            item,
            &[
                "load_id",
                "kind",
                "target_id",
                "vector_xyz",
                "coordinate_frame_id",
                "provenance",
            ],
            &at,
        )?;
        let id = string(item, "load_id", &at)?;
        token(&id, "load ID")?;
        if !ids.insert(id) {
            return Err(format!("{at} has duplicate load ID"));
        }
        let kind = string(item, "kind", &at)?;
        if ![
            "point_force",
            "distributed_force",
            "pressure",
            "body_acceleration",
        ]
        .contains(&kind.as_str())
        {
            return Err(format!("{at}.kind unsupported"));
        }
        token(&string(item, "target_id", &at)?, "load target ID")?;
        token(
            &string(item, "coordinate_frame_id", &at)?,
            "load coordinate frame",
        )?;
        let vector = item
            .get("vector_xyz")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{at}.vector_xyz must be array"))?;
        if vector.len() != 3
            || vector
                .iter()
                .any(|v| v.as_f64().is_none_or(|n| !n.is_finite()))
        {
            return Err(format!(
                "{at}.vector_xyz must contain exactly three finite numbers"
            ));
        }
        let provenance = object(item, "provenance", &at)?;
        allowed_keys(
            provenance,
            &["evidence_class", "source_ref"],
            &["uncertainty_relative"],
            &format!("{at}.provenance"),
        )?;
        let evidence = string_map(provenance, "evidence_class", &format!("{at}.provenance"))?;
        if ![
            "synthetic",
            "analytical_reference",
            "measured",
            "derived",
            "unknown",
        ]
        .contains(&evidence.as_str())
        {
            return Err(format!("{at}.provenance.evidence_class unknown"));
        }
    }
    Ok(())
}

fn validate_boundary_conditions(items: &[Value]) -> Check<()> {
    let mut ids = BTreeSet::new();
    for (i, item) in items.iter().enumerate() {
        let at = format!("request.boundary_conditions[{i}]");
        exact_keys(
            item,
            &["condition_id", "target_id", "kind", "dofs", "provenance"],
            &at,
        )?;
        let id = string(item, "condition_id", &at)?;
        token(&id, "boundary condition ID")?;
        if !ids.insert(id) {
            return Err(format!("{at} has duplicate ID"));
        }
        token(&string(item, "target_id", &at)?, "boundary target ID")?;
        let kind = string(item, "kind", &at)?;
        if ![
            "fixed",
            "pinned",
            "roller",
            "prescribed_displacement",
            "support_link",
        ]
        .contains(&kind.as_str())
        {
            return Err(format!("{at}.kind unsupported"));
        }
        let dofs = strings(item, "dofs", &at)?;
        if dofs.is_empty() {
            return Err(format!("{at}.dofs must not be empty"));
        }
        unique(&dofs, &format!("{at}.dofs"))?;
        if dofs
            .iter()
            .any(|v| !["ux", "uy", "uz", "rx", "ry", "rz"].contains(&v.as_str()))
        {
            return Err(format!("{at}.dofs contains unknown DOF"));
        }
        let provenance = object(item, "provenance", &at)?;
        allowed_keys(
            provenance,
            &["evidence_class", "source_ref"],
            &["uncertainty_relative"],
            &format!("{at}.provenance"),
        )?;
        let evidence = string_map(provenance, "evidence_class", &format!("{at}.provenance"))?;
        if ![
            "synthetic",
            "analytical_reference",
            "measured",
            "derived",
            "unknown",
        ]
        .contains(&evidence.as_str())
        {
            return Err(format!("{at}.provenance.evidence_class unknown"));
        }
    }
    Ok(())
}

fn validate_acceptance(items: &Map<String, Value>) -> Check<()> {
    allowed_keys(
        items,
        &[
            "relative_tolerance",
            "absolute_tolerance",
            "require_convergence",
        ],
        &["max_iterations", "max_time_step_s"],
        "request.acceptance",
    )?;
    for key in ["relative_tolerance", "absolute_tolerance"] {
        let n = items
            .get(key)
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("request.acceptance.{key} must be number"))?;
        if !n.is_finite() || n < 0.0 {
            return Err(format!(
                "request.acceptance.{key} must be finite and non-negative"
            ));
        }
    }
    if items
        .get("require_convergence")
        .and_then(Value::as_bool)
        .is_none()
    {
        return Err("request.acceptance.require_convergence must be boolean".to_owned());
    }
    if let Some(v) = items.get("max_iterations")
        && v.as_u64().is_none_or(|n| n == 0)
    {
        return Err("request.acceptance.max_iterations must be positive integer".to_owned());
    }
    if let Some(v) = items.get("max_time_step_s")
        && v.as_f64().is_none_or(|n| !n.is_finite() || n <= 0.0)
    {
        return Err("request.acceptance.max_time_step_s must be positive finite".to_owned());
    }
    Ok(())
}

fn exact_keys(value: &Value, expected: &[&str], at: &str) -> Check<()> {
    let actual = value
        .as_object()
        .ok_or_else(|| format!("{at} must be object"))?;
    allowed_keys(actual, expected, &[], at)
}
fn allowed_keys(
    actual: &Map<String, Value>,
    required: &[&str],
    optional: &[&str],
    at: &str,
) -> Check<()> {
    let mut allowed: BTreeSet<&str> = required.iter().copied().collect();
    allowed.extend(optional.iter().copied());
    let missing: Vec<_> = required
        .iter()
        .copied()
        .filter(|k| !actual.contains_key(*k))
        .collect();
    let unknown: Vec<_> = actual
        .keys()
        .map(String::as_str)
        .filter(|k| !allowed.contains(k))
        .collect();
    if !missing.is_empty() || !unknown.is_empty() {
        return Err(format!(
            "{at}: missing keys {missing:?}; unknown keys {unknown:?}"
        ));
    }
    Ok(())
}
fn object<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a Map<String, Value>> {
    value
        .get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{at}.{key} must be object"))
}
fn array<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a Vec<Value>> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{at}.{key} must be array"))
}
fn string(value: &Value, key: &str, at: &str) -> Check<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{at}.{key} must be string"))
}
fn string_map(value: &Map<String, Value>, key: &str, at: &str) -> Check<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{at}.{key} must be string"))
}
fn strings(value: &Value, key: &str, at: &str) -> Check<Vec<String>> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{at}.{key} must be array"))?
        .iter()
        .enumerate()
        .map(|(i, v)| {
            v.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{at}.{key}[{i}] must be string"))
        })
        .collect()
}
fn enum_array(values: &[String], allowed: &[&str], at: &str) -> Check<()> {
    if values.is_empty() {
        return Err(format!("{at} must not be empty"));
    }
    unique(values, at)?;
    if let Some(bad) = values.iter().find(|v| !allowed.contains(&v.as_str())) {
        return Err(format!("{at} has unsupported value '{bad}'"));
    }
    Ok(())
}
fn unique(values: &[String], at: &str) -> Check<()> {
    let mut seen = BTreeSet::new();
    if let Some(duplicate) = values.iter().find(|v| !seen.insert(v.as_str())) {
        return Err(format!("{at} has duplicate value '{duplicate}'"));
    }
    Ok(())
}
fn token(value: &str, label: &str) -> Check<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | ':'))
    {
        return Err(format!("invalid {label}"));
    }
    Ok(())
}
fn validate_hex(value: &str, length: usize, label: &str) -> Check<()> {
    if value.len() != length
        || !value
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        return Err(format!(
            "{label} must be {length} lowercase hexadecimal characters"
        ));
    }
    Ok(())
}

/// Validate a solver-produced receipt against the exact request bytes.
///
/// This checks structure and cross-field consistency only. It does not authenticate
/// execution, inspect output artifact bytes, or establish physical validity.
pub fn validate_solver_receipt_bytes(request_bytes: &[u8], receipt_bytes: &[u8]) -> Check<Value> {
    let request: Value = serde_json::from_slice(request_bytes)
        .map_err(|error| format!("invalid request JSON: {error}"))?;
    validate_shape(&request)?;

    let receipt: Value = serde_json::from_slice(receipt_bytes)
        .map_err(|error| format!("invalid solver receipt JSON: {error}"))?;
    let receipt_object = receipt
        .as_object()
        .ok_or_else(|| "receipt must be object".to_owned())?;
    allowed_keys(
        receipt_object,
        &[
            "schema_version",
            "request_id",
            "request_sha256",
            "source_revision",
            "adapter",
            "status",
            "evidence_class",
            "physics_evaluated",
            "physics_omitted",
            "convergence",
            "outputs",
            "uncertainty",
            "validity_envelope",
            "claim_ceiling",
            "diagnostics",
        ],
        &["input_artifacts"],
        "receipt",
    )?;

    if string(&receipt, "schema_version", "receipt")? != SOLVER_RESULT_RECEIPT_SCHEMA {
        return Err(format!(
            "receipt.schema_version must be '{SOLVER_RESULT_RECEIPT_SCHEMA}'"
        ));
    }

    let request_id = string(&request, "request_id", "request")?;
    if string(&receipt, "request_id", "receipt")? != request_id {
        return Err("receipt.request_id does not match the bound request".to_owned());
    }
    let source_revision = string(&request, "source_revision", "request")?;
    if string(&receipt, "source_revision", "receipt")? != source_revision {
        return Err("receipt.source_revision does not match the bound request".to_owned());
    }
    let expected_request_sha256 = sha256_hex(request_bytes);
    validate_hex(
        &string(&receipt, "request_sha256", "receipt")?,
        64,
        "receipt request SHA-256",
    )?;
    if string(&receipt, "request_sha256", "receipt")? != expected_request_sha256 {
        return Err("receipt.request_sha256 does not match the exact request bytes".to_owned());
    }

    let adapter = object(&receipt, "adapter", "receipt")?;
    allowed_keys(
        adapter,
        &[
            "adapter_id",
            "version",
            "executable_sha256",
            "configuration_sha256",
        ],
        &["solver_name", "solver_version", "solver_build_ref"],
        "receipt.adapter",
    )?;
    for key in ["adapter_id", "version"] {
        let value = string_map(adapter, key, "receipt.adapter")?;
        if value.trim().is_empty() || value.len() > 128 {
            return Err(format!(
                "receipt.adapter.{key} must be non-empty and at most 128 bytes"
            ));
        }
    }
    for key in ["executable_sha256", "configuration_sha256"] {
        validate_hex(
            &string_map(adapter, key, "receipt.adapter")?,
            64,
            &format!("receipt adapter {key}"),
        )?;
    }
    for key in ["solver_name", "solver_version", "solver_build_ref"] {
        if let Some(value) = adapter.get(key)
            && !value.is_null()
        {
            let text = value
                .as_str()
                .ok_or_else(|| format!("receipt.adapter.{key} must be a string or null"))?;
            if text.len()
                > match key {
                    "solver_name" => 256,
                    "solver_version" => 128,
                    _ => 2048,
                }
            {
                return Err(format!(
                    "receipt.adapter.{key} exceeds its schema length limit"
                ));
            }
        }
    }

    let status = string(&receipt, "status", "receipt")?;
    if !["PASS", "FAIL", "UNRESOLVED"].contains(&status.as_str()) {
        return Err(format!("receipt.status has unsupported value '{status}'"));
    }
    let evidence_class = string(&receipt, "evidence_class", "receipt")?;
    if ![
        "synthetic",
        "analytical_reference",
        "simulated",
        "measured_comparison",
        "unknown",
    ]
    .contains(&evidence_class.as_str())
    {
        return Err(format!(
            "receipt.evidence_class has unsupported value '{evidence_class}'"
        ));
    }

    let physics_evaluated = strings(&receipt, "physics_evaluated", "receipt")?;
    let physics_omitted = strings(&receipt, "physics_omitted", "receipt")?;
    unique(&physics_evaluated, "receipt.physics_evaluated")?;
    unique(&physics_omitted, "receipt.physics_omitted")?;
    for (label, values) in [
        ("receipt.physics_evaluated", &physics_evaluated),
        ("receipt.physics_omitted", &physics_omitted),
    ] {
        if let Some(unknown) = values
            .iter()
            .find(|value| !PHYSICS.contains(&value.as_str()))
        {
            return Err(format!(
                "{label} contains unknown physics label '{unknown}'"
            ));
        }
    }

    let requested_physics = strings(&request, "requested_physics", "request")?;
    let requested: BTreeSet<&str> = requested_physics.iter().map(String::as_str).collect();
    let evaluated: BTreeSet<&str> = physics_evaluated.iter().map(String::as_str).collect();
    let omitted: BTreeSet<&str> = physics_omitted.iter().map(String::as_str).collect();
    if let Some(overlap) = evaluated.intersection(&omitted).next() {
        return Err(format!(
            "receipt classifies physics '{overlap}' as both evaluated and omitted"
        ));
    }
    for physics in evaluated.union(&omitted) {
        if !requested.contains(physics) {
            return Err(format!(
                "receipt classifies unrequested physics '{physics}'"
            ));
        }
    }
    for physics in &requested {
        if !evaluated.contains(physics) && !omitted.contains(physics) {
            return Err(format!(
                "receipt does not classify requested physics '{physics}'"
            ));
        }
    }

    let declared_fidelity = string(&request, "model_fidelity", "request")?;
    for physics in &physics_evaluated {
        if !fidelity_supports_physics(&declared_fidelity, physics) {
            return Err(format!(
                "receipt claims '{physics}' evaluated outside the declared model fidelity '{declared_fidelity}'"
            ));
        }
    }

    let convergence = object(&receipt, "convergence", "receipt")?;
    exact_keys(
        &Value::Object(convergence.clone()),
        &[
            "state",
            "iterations",
            "residual",
            "mesh_refinement_evaluated",
            "details",
        ],
        "receipt.convergence",
    )?;
    let convergence_state = string_map(convergence, "state", "receipt.convergence")?;
    if !["not_applicable", "converged", "not_converged", "unknown"]
        .contains(&convergence_state.as_str())
    {
        return Err(format!(
            "receipt.convergence.state has unsupported value '{convergence_state}'"
        ));
    }
    let iterations = convergence
        .get("iterations")
        .ok_or_else(|| "receipt.convergence.iterations missing".to_owned())?;
    if !iterations.is_null() && iterations.as_u64().is_none() {
        return Err(
            "receipt.convergence.iterations must be null or a non-negative integer".to_owned(),
        );
    }
    let residual = convergence
        .get("residual")
        .ok_or_else(|| "receipt.convergence.residual missing".to_owned())?;
    if !residual.is_null()
        && residual
            .as_f64()
            .is_none_or(|number| !number.is_finite() || number < 0.0)
    {
        return Err(
            "receipt.convergence.residual must be null or finite and non-negative".to_owned(),
        );
    }
    if convergence
        .get("mesh_refinement_evaluated")
        .and_then(Value::as_bool)
        .is_none()
    {
        return Err("receipt.convergence.mesh_refinement_evaluated must be boolean".to_owned());
    }
    let _ = string_map(convergence, "details", "receipt.convergence")?;

    let outputs = array(&receipt, "outputs", "receipt")?;
    let mut output_ids = BTreeSet::new();
    let mut output_kinds = BTreeSet::new();
    for (index, output) in outputs.iter().enumerate() {
        let at = format!("receipt.outputs[{index}]");
        exact_keys(
            output,
            &[
                "output_id",
                "kind",
                "sha256",
                "media_type",
                "unit",
                "artifact_ref",
            ],
            &at,
        )?;
        let output_id = string(output, "output_id", &at)?;
        token(&output_id, "output ID")?;
        if !output_ids.insert(output_id) {
            return Err(format!("{at}.output_id is duplicated"));
        }
        let kind = string(output, "kind", &at)?;
        if kind.is_empty() || kind.len() > 128 || !OUTPUTS.contains(&kind.as_str()) {
            return Err(format!("{at}.kind has unsupported value '{kind}'"));
        }
        output_kinds.insert(kind);
        validate_hex(&string(output, "sha256", &at)?, 64, "output SHA-256")?;
        let media_type = string(output, "media_type", &at)?;
        if media_type.trim().is_empty() || media_type.len() > 128 {
            return Err(format!(
                "{at}.media_type must be non-empty and at most 128 bytes"
            ));
        }
        let unit = output
            .get("unit")
            .ok_or_else(|| format!("{at}.unit missing"))?;
        if !(unit.is_null() || unit.as_str().is_some_and(|text| text.len() <= 64)) {
            return Err(format!(
                "{at}.unit must be a string of at most 64 bytes or null"
            ));
        }
        let artifact_ref = string(output, "artifact_ref", &at)?;
        if artifact_ref.trim().is_empty() || artifact_ref.len() > 2048 {
            return Err(format!(
                "{at}.artifact_ref must be non-empty and at most 2048 bytes"
            ));
        }
    }

    if let Some(input_artifacts) = receipt.get("input_artifacts") {
        let input_artifacts = input_artifacts
            .as_array()
            .ok_or_else(|| "receipt.input_artifacts must be an array when present".to_owned())?;
        let mut input_ids = BTreeSet::new();
        for (index, input) in input_artifacts.iter().enumerate() {
            let at = format!("receipt.input_artifacts[{index}]");
            exact_keys(
                input,
                &["input_id", "kind", "sha256", "media_type", "artifact_ref"],
                &at,
            )?;
            let input_id = string(input, "input_id", &at)?;
            token(&input_id, "input artifact ID")?;
            if !input_ids.insert(input_id) {
                return Err(format!("{at}.input_id is duplicated"));
            }
            let kind = string(input, "kind", &at)?;
            if kind.trim().is_empty() || kind.len() > 128 {
                return Err(format!("{at}.kind must be non-empty and at most 128 bytes"));
            }
            validate_hex(&string(input, "sha256", &at)?, 64, "input artifact SHA-256")?;
            let media_type = string(input, "media_type", &at)?;
            if media_type.trim().is_empty() || media_type.len() > 128 {
                return Err(format!(
                    "{at}.media_type must be non-empty and at most 128 bytes"
                ));
            }
            let artifact_ref = string(input, "artifact_ref", &at)?;
            if artifact_ref.trim().is_empty() || artifact_ref.len() > 2048 {
                return Err(format!(
                    "{at}.artifact_ref must be non-empty and at most 2048 bytes"
                ));
            }
        }
    }

    let uncertainty = object(&receipt, "uncertainty", "receipt")?;
    exact_keys(
        &Value::Object(uncertainty.clone()),
        &["state", "method", "summary"],
        "receipt.uncertainty",
    )?;
    let uncertainty_state = string_map(uncertainty, "state", "receipt.uncertainty")?;
    if !["quantified", "qualitative", "unknown", "not_applicable"]
        .contains(&uncertainty_state.as_str())
    {
        return Err(format!(
            "receipt.uncertainty.state unsupported: '{uncertainty_state}'"
        ));
    }
    let method = uncertainty
        .get("method")
        .ok_or_else(|| "receipt.uncertainty.method missing".to_owned())?;
    if !(method.is_null() || method.as_str().is_some_and(|text| text.len() <= 256)) {
        return Err(
            "receipt.uncertainty.method must be a string of at most 256 bytes or null".to_owned(),
        );
    }
    let _ = string_map(uncertainty, "summary", "receipt.uncertainty")?;

    let validity = object(&receipt, "validity_envelope", "receipt")?;
    exact_keys(
        &Value::Object(validity.clone()),
        &[
            "supported_geometry",
            "supported_material_model",
            "load_regime",
            "limitations",
        ],
        "receipt.validity_envelope",
    )?;
    for key in [
        "supported_geometry",
        "supported_material_model",
        "load_regime",
    ] {
        let value = string_map(validity, key, "receipt.validity_envelope")?;
        if value.trim().is_empty() || value.len() > 2048 {
            return Err(format!(
                "receipt.validity_envelope.{key} is empty or overlong"
            ));
        }
    }
    let limitations = validity
        .get("limitations")
        .and_then(Value::as_array)
        .ok_or_else(|| "receipt.validity_envelope.limitations must be an array".to_owned())?;
    if limitations
        .iter()
        .any(|value| value.as_str().is_none_or(|text| text.len() > 2048))
    {
        return Err(
            "receipt.validity_envelope.limitations must contain strings no longer than 2048 bytes"
                .to_owned(),
        );
    }

    let claim_ceiling = string(&receipt, "claim_ceiling", "receipt")?;
    if claim_ceiling.trim().is_empty() || claim_ceiling.len() > 2048 {
        return Err("receipt.claim_ceiling must be non-empty and at most 2048 bytes".to_owned());
    }
    if status == "UNRESOLVED" {
        let lowercase = claim_ceiling.to_ascii_lowercase();
        if ![
            "unresolved",
            "not qualified",
            "not validated",
            "not a safety",
        ]
        .iter()
        .any(|phrase| lowercase.contains(phrase))
        {
            return Err(
                "UNRESOLVED receipt claim_ceiling must state the unresolved/unqualified limit"
                    .to_owned(),
            );
        }
    }

    let diagnostics = array(&receipt, "diagnostics", "receipt")?;
    if diagnostics
        .iter()
        .any(|value| value.as_str().is_none_or(|text| text.len() > 2048))
    {
        return Err(
            "receipt.diagnostics must contain strings no longer than 2048 bytes".to_owned(),
        );
    }

    if ["FAIL", "UNRESOLVED"].contains(&status.as_str())
        && diagnostics
            .iter()
            .filter_map(Value::as_str)
            .all(|text| text.trim().is_empty())
    {
        return Err(format!(
            "{status} receipt requires at least one non-empty diagnostic"
        ));
    }

    if status == "PASS" {
        validate_pass_request_preconditions(&request)?;
        if outputs.is_empty() {
            return Err("PASS receipt must contain at least one output artifact".to_owned());
        }
        if !["converged", "not_applicable"].contains(&convergence_state.as_str()) {
            return Err("PASS receipt requires converged or not_applicable convergence".to_owned());
        }
        if !physics_omitted.is_empty() {
            return Err(
                "PASS receipt cannot omit any physics requested by the bound request".to_owned(),
            );
        }
        let fidelity = string(&request, "model_fidelity", "request")?;
        for physics in &requested_physics {
            if !fidelity_supports_physics(&fidelity, physics) {
                return Err(format!(
                    "PASS receipt claims '{physics}' outside the declared model fidelity '{fidelity}'"
                ));
            }
        }
        for required in strings(&request, "required_outputs", "request")? {
            if !output_kinds.contains(&required) {
                return Err(format!(
                    "PASS receipt is missing required output kind '{required}'"
                ));
            }
        }
        for key in ["solver_name", "solver_version", "solver_build_ref"] {
            let value = adapter.get(key).and_then(Value::as_str).unwrap_or("");
            if value.trim().is_empty() {
                return Err(format!("PASS receipt requires non-empty adapter.{key}"));
            }
        }
    }

    Ok(json!({
        "schema_version": "physical-solver-receipt-validation-report-v0.1",
        "receipt_contract_validation": "PASS",
        "producer_status": status,
        "request_id": request_id,
        "request_sha256": expected_request_sha256,
        "source_revision": source_revision,
        "physics_partition_exact": true,
        "required_outputs_present": strings(&request, "required_outputs", "request")?.iter().all(|kind| output_kinds.contains(kind)),
        "solver_execution_authenticated": false,
        "output_artifact_bytes_verified": false,
        "physical_result_qualified": false,
        "claim_ceiling": "receipt/request consistency only; producer execution, output bytes, numerical correctness, and physical validity are not authenticated"
    }))
}

/// Verify each receipt-declared output artifact against its exact local file bytes.
///
/// This verifies byte identity only. It deliberately does not authenticate the
/// process that produced the bytes, validate their numerical meaning, or qualify
/// a physical claim. All artifacts must live under an immutable local artifact
/// root for the duration of verification; network references are never fetched.
pub fn verify_solver_output_artifact_bytes(
    request_bytes: &[u8],
    receipt_bytes: &[u8],
    artifact_root: &Path,
) -> Check<Value> {
    let mut report = validate_solver_receipt_bytes(request_bytes, receipt_bytes)?;
    let receipt: Value = serde_json::from_slice(receipt_bytes)
        .map_err(|error| format!("invalid solver receipt JSON: {error}"))?;
    let outputs = array(&receipt, "outputs", "receipt")?;

    let (canonical_root, root_diagnostic) = match fs::canonicalize(artifact_root) {
        Ok(root) if root.is_dir() => (Some(root), None),
        Ok(_) => (None, Some("artifact root is not a directory".to_owned())),
        Err(error) => (
            None,
            Some(format!("could not canonicalize artifact root: {error}")),
        ),
    };

    let mut artifact_results = Vec::<Value>::new();
    let mut diagnostics = Vec::<String>::new();
    let mut has_digest_mismatch = false;
    let mut has_unresolved_artifact = false;

    for (index, output) in outputs.iter().enumerate() {
        let at = format!("receipt.outputs[{index}]");
        let output_id = string(output, "output_id", &at)?;
        let artifact_ref = string(output, "artifact_ref", &at)?;
        let expected_sha256 = string(output, "sha256", &at)?;
        let mut status = "UNRESOLVED";
        let mut observed_sha256 = Value::Null;
        let mut byte_length = Value::Null;
        let diagnostic = if let Some(reason) = root_diagnostic.as_ref() {
            Some(reason.clone())
        } else {
            match safe_artifact_relative_path(&artifact_ref) {
                Err(reason) => Some(reason),
                Ok(relative_path) => {
                    let root = canonical_root
                        .as_ref()
                        .expect("canonical root exists when no root diagnostic exists");
                    let candidate = root.join(relative_path);
                    match fs::canonicalize(&candidate) {
                        Err(error) => Some(format!("artifact cannot be resolved: {error}")),
                        Ok(canonical_path) if !canonical_path.starts_with(root) => {
                            Some("artifact resolves outside the declared artifact root".to_owned())
                        }
                        Ok(canonical_path) => match fs::metadata(&canonical_path) {
                            Err(error) => {
                                Some(format!("artifact metadata cannot be read: {error}"))
                            }
                            Ok(metadata) if !metadata.is_file() => {
                                Some("artifact is not a regular file".to_owned())
                            }
                            Ok(_) => match sha256_file(&canonical_path) {
                                Err(error) => {
                                    Some(format!("artifact bytes cannot be read: {error}"))
                                }
                                Ok((observed, length)) => {
                                    observed_sha256 = json!(observed);
                                    byte_length = json!(length);
                                    if observed == expected_sha256 {
                                        status = "PASS";
                                        None
                                    } else {
                                        status = "FAIL";
                                        has_digest_mismatch = true;
                                        Some(format!(
                                            "SHA-256 mismatch: expected {expected_sha256}, observed {observed}"
                                        ))
                                    }
                                }
                            },
                        },
                    }
                }
            }
        };

        if diagnostic.is_some() && status != "FAIL" {
            has_unresolved_artifact = true;
        }
        if let Some(reason) = diagnostic.as_ref() {
            diagnostics.push(format!("{output_id}: {reason}"));
        }
        artifact_results.push(json!({
            "output_id": output_id,
            "artifact_ref": artifact_ref,
            "status": status,
            "expected_sha256": expected_sha256,
            "observed_sha256": observed_sha256,
            "byte_length": byte_length,
            "diagnostic": diagnostic
        }));
    }

    if outputs.is_empty() {
        diagnostics.push("receipt contains no output artifacts to verify".to_owned());
        has_unresolved_artifact = true;
    }

    let output_status = if has_digest_mismatch {
        "FAIL"
    } else if has_unresolved_artifact {
        "UNRESOLVED"
    } else {
        "PASS"
    };
    let verified_output_count = artifact_results
        .iter()
        .filter(|item| item.get("status").and_then(Value::as_str) == Some("PASS"))
        .count();

    // Input artifacts are optional for legacy receipts, but a mesh-study gate
    // requires them. A receipt-declared input becomes evidence only after its
    // exact bytes are checked under the same canonical root.
    let input_artifacts = receipt
        .get("input_artifacts")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut input_results = Vec::<Value>::new();
    let mut input_diagnostics = Vec::<String>::new();
    let mut has_input_digest_mismatch = false;
    let mut has_input_unresolved = false;
    for (index, input) in input_artifacts.iter().enumerate() {
        let at = format!("receipt.input_artifacts[{index}]");
        let input_id = string(input, "input_id", &at)?;
        let kind = string(input, "kind", &at)?;
        let artifact_ref = string(input, "artifact_ref", &at)?;
        let expected_sha256 = string(input, "sha256", &at)?;
        let mut status = "UNRESOLVED";
        let mut observed_sha256 = Value::Null;
        let mut byte_length = Value::Null;
        let diagnostic = if let Some(reason) = root_diagnostic.as_ref() {
            Some(reason.clone())
        } else {
            match safe_artifact_relative_path(&artifact_ref) {
                Err(reason) => Some(reason),
                Ok(relative_path) => {
                    let root = canonical_root
                        .as_ref()
                        .expect("canonical root exists when no root diagnostic exists");
                    let candidate = root.join(relative_path);
                    match fs::canonicalize(&candidate) {
                        Err(error) => Some(format!("input artifact cannot be resolved: {error}")),
                        Ok(canonical_path) if !canonical_path.starts_with(root) => Some(
                            "input artifact resolves outside the declared artifact root".to_owned(),
                        ),
                        Ok(canonical_path) => match fs::metadata(&canonical_path) {
                            Err(error) => {
                                Some(format!("input artifact metadata cannot be read: {error}"))
                            }
                            Ok(metadata) if !metadata.is_file() => {
                                Some("input artifact is not a regular file".to_owned())
                            }
                            Ok(_) => match sha256_file(&canonical_path) {
                                Err(error) => {
                                    Some(format!("input artifact bytes cannot be read: {error}"))
                                }
                                Ok((observed, length)) => {
                                    observed_sha256 = json!(observed);
                                    byte_length = json!(length);
                                    if observed == expected_sha256 {
                                        status = "PASS";
                                        None
                                    } else {
                                        status = "FAIL";
                                        has_input_digest_mismatch = true;
                                        Some(format!(
                                            "SHA-256 mismatch: expected {expected_sha256}, observed {observed}"
                                        ))
                                    }
                                }
                            },
                        },
                    }
                }
            }
        };
        if diagnostic.is_some() && status != "FAIL" {
            has_input_unresolved = true;
        }
        if let Some(reason) = diagnostic.as_ref() {
            input_diagnostics.push(format!("{input_id}: {reason}"));
        }
        input_results.push(json!({
            "input_id": input_id,
            "kind": kind,
            "artifact_ref": artifact_ref,
            "status": status,
            "expected_sha256": expected_sha256,
            "observed_sha256": observed_sha256,
            "byte_length": byte_length,
            "diagnostic": diagnostic
        }));
    }

    let input_status = if input_artifacts.is_empty() {
        "NOT_DECLARED"
    } else if has_input_digest_mismatch {
        "FAIL"
    } else if has_input_unresolved {
        "UNRESOLVED"
    } else {
        "PASS"
    };
    let artifact_status = if output_status == "FAIL" || input_status == "FAIL" {
        "FAIL"
    } else if output_status == "UNRESOLVED" || input_status == "UNRESOLVED" {
        "UNRESOLVED"
    } else {
        "PASS"
    };
    let verified_input_count = input_results
        .iter()
        .filter(|item| item.get("status").and_then(Value::as_str) == Some("PASS"))
        .count();
    diagnostics.extend(input_diagnostics);

    report["artifact_verification_status"] = json!(artifact_status);
    report["artifact_verification_method"] = json!("local_file_sha256_under_canonical_root");
    report["output_artifact_verification_status"] = json!(output_status);
    report["input_artifact_verification_status"] = json!(input_status);
    report["artifact_results"] = Value::Array(artifact_results);
    report["input_artifact_results"] = Value::Array(input_results);
    report["artifact_diagnostics"] = json!(diagnostics);
    report["verified_output_count"] = json!(verified_output_count);
    report["verified_input_count"] = json!(verified_input_count);
    report["output_artifact_bytes_verified"] = json!(output_status == "PASS");
    report["input_artifact_bytes_verified"] =
        json!(!input_artifacts.is_empty() && input_status == "PASS");
    // Byte identity is not execution identity or physical qualification.
    report["solver_execution_authenticated"] = json!(false);
    report["physical_result_qualified"] = json!(false);
    report["claim_ceiling"] = json!(
        "declared input/output byte identity only; does not authenticate solver execution, numerical correctness, measurement truth, or physical validity"
    );
    Ok(report)
}

fn safe_artifact_relative_path(reference: &str) -> Result<PathBuf, String> {
    if reference.is_empty()
        || reference.trim() != reference
        || reference
            .chars()
            .any(|character| matches!(character, '\\' | ':' | '\0'))
        || reference
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err("artifact_ref must be a normalized relative POSIX path".to_owned());
    }

    let path = Path::new(reference);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("artifact_ref must stay below the declared artifact root".to_owned());
    }
    Ok(path.to_path_buf())
}

fn validate_pass_request_preconditions(request: &Value) -> Check<()> {
    let fidelity = string(request, "model_fidelity", "request")?;
    let request_frame = string(request, "coordinate_frame_id", "request")?;
    for (index, load) in array(request, "loads", "request")?.iter().enumerate() {
        let load_frame = string(
            load,
            "coordinate_frame_id",
            &format!("request.loads[{index}]"),
        )?;
        if load_frame != request_frame {
            return Err(format!(
                "PASS request load {index} uses frame '{load_frame}' without a declared transform to '{request_frame}'"
            ));
        }
    }
    if fidelity == "graph_topology"
        && strings(request, "required_outputs", "request")?
            .iter()
            .any(|output| output != "topology")
    {
        return Err("PASS request asks graph_topology for non-topological outputs".to_owned());
    }
    if fidelity == "analytical_linear_elastic" {
        if array(request, "materials", "request")?.is_empty() {
            return Err("PASS analytical request requires material declarations".to_owned());
        }
        if array(request, "loads", "request")?.is_empty() {
            return Err("PASS analytical request requires at least one load".to_owned());
        }
        if array(request, "boundary_conditions", "request")?.is_empty() {
            return Err("PASS analytical request requires boundary conditions".to_owned());
        }
        for (index, material) in array(request, "materials", "request")?.iter().enumerate() {
            for key in ["youngs_modulus_pa", "poisson_ratio", "density_kg_m3"] {
                if material.get(key).is_none_or(Value::is_null) {
                    return Err(format!(
                        "PASS analytical request has unresolved material property request.materials[{index}].{key}"
                    ));
                }
            }
        }
    }
    Ok(())
}

fn fidelity_supports_physics(fidelity: &str, physics: &str) -> bool {
    match fidelity {
        "graph_topology" => physics == "topology",
        "analytical_linear_elastic" | "continuum_linear_elastic" => {
            matches!(physics, "topology" | "static_linear_elastic")
        }
        "continuum_nonlinear" => {
            matches!(
                physics,
                "topology" | "static_linear_elastic" | "nonlinear_material"
            )
        }
        "phase_field_fracture" => {
            matches!(physics, "topology" | "static_linear_elastic" | "fracture")
        }
        "rigid_contact" => matches!(physics, "topology" | "dynamic_response" | "rigid_contact"),
        "granular_dem" => matches!(
            physics,
            "topology" | "dynamic_response" | "granular_contact"
        ),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};
    const REV: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    fn fixture(name: &str) -> Vec<u8> {
        fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../schemas/physical-transformation/fixtures")
                .join(name),
        )
        .expect("fixture exists")
    }
    #[test]
    fn sha256_known_answer() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
    #[test]
    fn analytical_request_validates_but_physics_remains_unresolved() {
        let out = validate_request_bytes(&fixture("analytical-bar-request-v0.1.json"), REV)
            .expect("request validates");
        assert_eq!(out["request_validation"], "PASS");
        assert_eq!(out["physical_result_status"], "UNRESOLVED");
        assert_eq!(out["solver_invoked"], false);
        assert_eq!(out["physics_omitted"].as_array().unwrap().len(), 0);
        assert_eq!(out["request_sha256"].as_str().unwrap().len(), 64);
    }
    #[test]
    fn unsupported_fracture_is_explicitly_omitted() {
        let out = validate_request_bytes(
            &fixture("unsupported-dynamic-fracture-request-v0.1.json"),
            REV,
        )
        .expect("shape valid");
        assert_eq!(out["physical_result_status"], "UNRESOLVED");
        let omitted = out["physics_omitted"].as_array().unwrap();
        assert!(omitted.iter().any(|v| v == "dynamic_response"));
        assert!(omitted.iter().any(|v| v == "fracture"));
    }
    #[test]
    fn bad_units_duplicate_physics_and_unknown_top_level_keys_fail_closed() {
        let mut value: Value =
            serde_json::from_slice(&fixture("analytical-bar-request-v0.1.json")).unwrap();
        value["units"]["length"] = json!("ft");
        assert!(validate_request_bytes(value.to_string().as_bytes(), REV).is_err());
        let mut value: Value =
            serde_json::from_slice(&fixture("analytical-bar-request-v0.1.json")).unwrap();
        value["requested_physics"] = json!(["static_linear_elastic", "static_linear_elastic"]);
        assert!(validate_request_bytes(value.to_string().as_bytes(), REV).is_err());
        let mut value: Value =
            serde_json::from_slice(&fixture("analytical-bar-request-v0.1.json")).unwrap();
        value["unknown_override"] = json!(true);
        assert!(validate_request_bytes(value.to_string().as_bytes(), REV).is_err());
    }
    #[test]
    fn missing_material_properties_do_not_become_solver_pass() {
        let mut value: Value =
            serde_json::from_slice(&fixture("analytical-bar-request-v0.1.json")).unwrap();
        value["materials"][0]["youngs_modulus_pa"] = Value::Null;
        let out = validate_request_bytes(value.to_string().as_bytes(), REV)
            .expect("null allowed in shape");
        assert_eq!(out["physical_result_status"], "UNRESOLVED");
        assert_eq!(out["capability_status"], "UNRESOLVED");
        assert!(
            out["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|d| d.as_str().unwrap().contains("youngs_modulus_pa"))
        );
    }
    fn solver_receipt(request_bytes: &[u8], status: &str) -> Value {
        let request: Value = serde_json::from_slice(request_bytes).unwrap();
        let requested = request["requested_physics"].as_array().unwrap().clone();
        let pass = status == "PASS";
        let physics_evaluated = if pass {
            Value::Array(requested.clone())
        } else {
            json!([])
        };
        let physics_omitted = if pass {
            json!([])
        } else {
            Value::Array(requested.clone())
        };
        let outputs: Vec<Value> = if pass {
            request["required_outputs"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .map(|(index, output)| {
                    let kind = output.as_str().unwrap().to_owned();
                    json!({
                        "output_id": format!("fixture-output-{index}"),
                        "kind": kind,
                        "sha256": "a".repeat(64),
                        "media_type": "application/json",
                        "unit": null,
                        "artifact_ref": format!("fixture-only:{kind}")
                    })
                })
                .collect()
        } else {
            Vec::new()
        };
        json!({
            "schema_version": SOLVER_RESULT_RECEIPT_SCHEMA,
            "request_id": request["request_id"],
            "request_sha256": sha256_hex(request_bytes),
            "source_revision": request["source_revision"],
            "adapter": {
                "adapter_id": "test.synthetic-adapter",
                "version": "0.1.0",
                "executable_sha256": "b".repeat(64),
                "configuration_sha256": "c".repeat(64),
                "solver_name": if pass { json!("synthetic-test-only") } else { Value::Null },
                "solver_version": if pass { json!("0.1.0") } else { Value::Null },
                "solver_build_ref": if pass { json!("fixture-only:not-a-real-solver") } else { Value::Null }
            },
            "status": status,
            "evidence_class": "synthetic",
            "physics_evaluated": physics_evaluated,
            "physics_omitted": physics_omitted,
            "convergence": {
                "state": "not_applicable",
                "iterations": null,
                "residual": null,
                "mesh_refinement_evaluated": false,
                "details": "Synthetic contract fixture; no solver execution is asserted."
            },
            "outputs": outputs,
            "uncertainty": {
                "state": "unknown",
                "method": null,
                "summary": "No measured data or physical uncertainty estimate is present."
            },
            "validity_envelope": {
                "supported_geometry": "synthetic fixture only",
                "supported_material_model": "synthetic fixture only",
                "load_regime": "synthetic fixture only",
                "limitations": ["Test-only receipt; never engineering evidence."]
            },
            "claim_ceiling": if status == "UNRESOLVED" {
                "UNRESOLVED: contract fixture only."
            } else {
                "Synthetic contract test only; no physical qualification."
            },
            "diagnostics": if status == "PASS" {
                json!([])
            } else {
                json!(["Synthetic fixture disposition; no real solver execution is asserted."])
            }
        })
    }

    #[test]
    fn solver_receipt_checks_exact_request_bytes_without_attesting_execution() {
        let request_bytes = fixture("analytical-bar-request-v0.1.json");
        let receipt_bytes = serde_json::to_vec(&solver_receipt(&request_bytes, "PASS")).unwrap();
        let report = validate_solver_receipt_bytes(&request_bytes, &receipt_bytes)
            .expect("synthetic receipt is internally consistent");
        assert_eq!(report["receipt_contract_validation"], "PASS");
        assert_eq!(report["producer_status"], "PASS");
        assert_eq!(report["physics_partition_exact"], true);
        assert_eq!(report["required_outputs_present"], true);
        assert_eq!(report["solver_execution_authenticated"], false);
        assert_eq!(report["output_artifact_bytes_verified"], false);

        // Whitespace changes the exact request-byte identity even when JSON semantics do not.
        let mut changed_bytes = request_bytes.clone();
        changed_bytes.push(b' ');
        let error = validate_solver_receipt_bytes(&changed_bytes, &receipt_bytes).unwrap_err();
        assert!(error.contains("exact request bytes"));
    }

    #[test]
    fn contract_valid_fail_receipt_remains_a_failed_physical_result() {
        let request_bytes = fixture("analytical-bar-request-v0.1.json");
        let mut receipt = solver_receipt(&request_bytes, "FAIL");
        receipt["diagnostics"] = json!(["Synthetic control: registered criterion failed."]);
        receipt["convergence"]["state"] = json!("not_converged");
        let receipt_bytes = serde_json::to_vec(&receipt).unwrap();

        let report = validate_solver_receipt_bytes(&request_bytes, &receipt_bytes)
            .expect("a well-formed FAIL receipt can be contract-valid");
        assert_eq!(report["receipt_contract_validation"], "PASS");
        assert_eq!(report["producer_status"], "FAIL");
        assert_eq!(report["physical_result_qualified"], false);
        assert_eq!(report["required_outputs_present"], false);
    }

    #[test]
    fn unresolved_receipt_requires_exact_disjoint_physics_partition() {
        let request_bytes = fixture("unsupported-dynamic-fracture-request-v0.1.json");
        let mut receipt = solver_receipt(&request_bytes, "UNRESOLVED");
        let valid = serde_json::to_vec(&receipt).unwrap();
        let report = validate_solver_receipt_bytes(&request_bytes, &valid)
            .expect("unresolved receipt contract is consistent");
        assert_eq!(report["producer_status"], "UNRESOLVED");
        assert_eq!(report["required_outputs_present"], false);
        assert_eq!(report["physical_result_qualified"], false);

        receipt["physics_evaluated"] = json!(["dynamic_response"]);
        let overlap = serde_json::to_vec(&receipt).unwrap();
        assert!(
            validate_solver_receipt_bytes(&request_bytes, &overlap)
                .unwrap_err()
                .contains("both evaluated and omitted")
        );

        receipt["physics_evaluated"] = json!([]);
        receipt["physics_omitted"] = json!(["fracture"]);
        let incomplete = serde_json::to_vec(&receipt).unwrap();
        assert!(
            validate_solver_receipt_bytes(&request_bytes, &incomplete)
                .unwrap_err()
                .contains("does not classify requested physics")
        );

        receipt["physics_omitted"] = json!(["dynamic_response", "fracture", "topology"]);
        let extra = serde_json::to_vec(&receipt).unwrap();
        assert!(
            validate_solver_receipt_bytes(&request_bytes, &extra)
                .unwrap_err()
                .contains("unrequested physics")
        );
    }

    #[test]
    fn pass_receipt_cannot_promote_physics_outside_declared_fidelity() {
        let mut request: Value =
            serde_json::from_slice(&fixture("unsupported-dynamic-fracture-request-v0.1.json"))
                .unwrap();
        // Keep the request structurally valid for a nonlinear solver class while asking
        // for dynamic/fracture physics outside that declared class.
        request["model_fidelity"] = json!("continuum_nonlinear");
        let request_bytes = serde_json::to_vec(&request).unwrap();
        let receipt = solver_receipt(&request_bytes, "PASS");
        let receipt_bytes = serde_json::to_vec(&receipt).unwrap();
        let error = validate_solver_receipt_bytes(&request_bytes, &receipt_bytes).unwrap_err();
        assert!(error.contains("outside the declared model fidelity"));
    }

    #[test]
    fn unresolved_receipt_cannot_claim_out_of_fidelity_physics_was_evaluated() {
        let request_bytes = fixture("unsupported-dynamic-fracture-request-v0.1.json");
        let mut receipt = solver_receipt(&request_bytes, "UNRESOLVED");
        receipt["physics_evaluated"] = json!(["dynamic_response"]);
        receipt["physics_omitted"] = json!(["fracture"]);
        let receipt_bytes = serde_json::to_vec(&receipt).unwrap();
        let error = validate_solver_receipt_bytes(&request_bytes, &receipt_bytes).unwrap_err();
        assert!(error.contains("evaluated outside the declared model fidelity"));
    }

    #[test]
    fn fail_and_unresolved_receipts_need_a_diagnostic() {
        let request_bytes = fixture("analytical-bar-request-v0.1.json");
        let mut receipt = solver_receipt(&request_bytes, "FAIL");
        receipt["diagnostics"] = json!([]);
        let receipt_bytes = serde_json::to_vec(&receipt).unwrap();
        assert!(
            validate_solver_receipt_bytes(&request_bytes, &receipt_bytes)
                .unwrap_err()
                .contains("FAIL receipt requires")
        );

        let mut receipt = solver_receipt(&request_bytes, "UNRESOLVED");
        receipt["diagnostics"] = json!([""]);
        let receipt_bytes = serde_json::to_vec(&receipt).unwrap();
        assert!(
            validate_solver_receipt_bytes(&request_bytes, &receipt_bytes)
                .unwrap_err()
                .contains("UNRESOLVED receipt requires")
        );
    }

    #[test]
    fn pass_receipt_must_include_every_required_output_kind() {
        let request_bytes = fixture("analytical-bar-request-v0.1.json");
        let mut receipt = solver_receipt(&request_bytes, "PASS");
        receipt["outputs"].as_array_mut().unwrap().pop();
        let receipt_bytes = serde_json::to_vec(&receipt).unwrap();
        let error = validate_solver_receipt_bytes(&request_bytes, &receipt_bytes).unwrap_err();
        assert!(error.contains("missing required output kind"));
    }

    #[test]
    fn pass_receipt_cannot_bypass_frame_or_material_gaps() {
        let request_bytes = fixture("analytical-bar-request-v0.1.json");
        let mut request: Value = serde_json::from_slice(&request_bytes).unwrap();
        request["loads"][0]["coordinate_frame_id"] = json!("untransformed-frame");
        let changed_request = serde_json::to_vec(&request).unwrap();
        let receipt = solver_receipt(&changed_request, "PASS");
        let receipt_bytes = serde_json::to_vec(&receipt).unwrap();
        assert!(
            validate_solver_receipt_bytes(&changed_request, &receipt_bytes)
                .unwrap_err()
                .contains("without a declared transform")
        );

        let mut request: Value = serde_json::from_slice(&request_bytes).unwrap();
        request["materials"][0]["youngs_modulus_pa"] = Value::Null;
        let changed_request = serde_json::to_vec(&request).unwrap();
        let receipt = solver_receipt(&changed_request, "PASS");
        let receipt_bytes = serde_json::to_vec(&receipt).unwrap();
        assert!(
            validate_solver_receipt_bytes(&changed_request, &receipt_bytes)
                .unwrap_err()
                .contains("unresolved material property")
        );
    }

    fn temporary_artifact_root() -> PathBuf {
        use std::time::{SystemTime, UNIX_EPOCH};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after UNIX epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "physical-solver-artifact-test-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&root).expect("temporary artifact root is created");
        root
    }

    fn bind_test_artifacts(receipt: &mut Value, root: &Path) {
        for output in receipt["outputs"]
            .as_array_mut()
            .expect("test receipt has output array")
        {
            let output_id = output["output_id"]
                .as_str()
                .expect("output id is string")
                .to_owned();
            let artifact_ref = format!("{output_id}.json");
            let contents =
                format!("{{\"synthetic_fixture_only\":true,\"output_id\":\"{output_id}\"}}\n");
            fs::write(root.join(&artifact_ref), contents.as_bytes())
                .expect("test artifact is written");
            output["artifact_ref"] = json!(artifact_ref);
            output["sha256"] = json!(sha256_hex(contents.as_bytes()));
        }
    }

    #[test]
    fn output_artifact_bytes_verify_without_authenticating_execution_or_physics() {
        let root = temporary_artifact_root();
        let request_bytes = fixture("analytical-bar-request-v0.1.json");
        let mut receipt = solver_receipt(&request_bytes, "PASS");
        bind_test_artifacts(&mut receipt, &root);
        let receipt_bytes = serde_json::to_vec(&receipt).unwrap();

        let report = verify_solver_output_artifact_bytes(&request_bytes, &receipt_bytes, &root)
            .expect("well-formed receipt and local bundle return a verification report");
        assert_eq!(report["receipt_contract_validation"], "PASS");
        assert_eq!(report["producer_status"], "PASS");
        assert_eq!(report["artifact_verification_status"], "PASS");
        assert_eq!(report["output_artifact_bytes_verified"], true);
        assert_eq!(report["solver_execution_authenticated"], false);
        assert_eq!(report["physical_result_qualified"], false);
        assert_eq!(
            report["verified_output_count"],
            receipt["outputs"].as_array().unwrap().len()
        );

        fs::remove_dir_all(root).expect("temporary artifact root is removed");
    }

    #[test]
    fn streamed_hash_matches_reference_across_multiple_chunks() {
        let root = temporary_artifact_root();
        let request_bytes = fixture("analytical-bar-request-v0.1.json");
        let mut receipt = solver_receipt(&request_bytes, "PASS");
        bind_test_artifacts(&mut receipt, &root);

        // Cross multiple 64 KiB reads and include a non-aligned final chunk.
        let contents: Vec<u8> = (0..(3 * 64 * 1024 + 17))
            .map(|index| ((index * 31 + 7) % 256) as u8)
            .collect();
        let first_ref = receipt["outputs"][0]["artifact_ref"]
            .as_str()
            .unwrap()
            .to_owned();
        fs::write(root.join(first_ref), &contents).expect("large test artifact is written");
        receipt["outputs"][0]["sha256"] = json!(sha256_hex(&contents));
        let receipt_bytes = serde_json::to_vec(&receipt).unwrap();

        let report = verify_solver_output_artifact_bytes(&request_bytes, &receipt_bytes, &root)
            .expect("streamed verifier accepts matching multi-chunk artifact bytes");
        assert_eq!(report["artifact_verification_status"], "PASS");
        assert_eq!(report["artifact_results"][0]["status"], "PASS");
        assert_eq!(
            report["artifact_results"][0]["observed_sha256"],
            sha256_hex(&contents)
        );
        assert_eq!(
            report["artifact_results"][0]["byte_length"],
            contents.len() as u64
        );

        fs::remove_dir_all(root).expect("temporary artifact root is removed");
    }

    #[test]
    fn changed_output_bytes_report_failure_not_a_verified_bundle() {
        let root = temporary_artifact_root();
        let request_bytes = fixture("analytical-bar-request-v0.1.json");
        let mut receipt = solver_receipt(&request_bytes, "PASS");
        bind_test_artifacts(&mut receipt, &root);
        let first_ref = receipt["outputs"][0]["artifact_ref"]
            .as_str()
            .unwrap()
            .to_owned();
        fs::write(root.join(first_ref), b"tampered output bytes\n")
            .expect("tampered artifact is written");
        let receipt_bytes = serde_json::to_vec(&receipt).unwrap();

        let report = verify_solver_output_artifact_bytes(&request_bytes, &receipt_bytes, &root)
            .expect("digest mismatches are returned as an explicit FAIL report");
        assert_eq!(report["artifact_verification_status"], "FAIL");
        assert_eq!(report["output_artifact_bytes_verified"], false);
        assert_eq!(report["solver_execution_authenticated"], false);
        assert_eq!(report["physical_result_qualified"], false);
        assert!(
            report["artifact_diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|diagnostic| diagnostic.as_str().unwrap().contains("SHA-256 mismatch"))
        );

        fs::remove_dir_all(root).expect("temporary artifact root is removed");
    }

    #[test]
    fn traversal_references_remain_unresolved() {
        let root = temporary_artifact_root();
        let request_bytes = fixture("analytical-bar-request-v0.1.json");
        let mut receipt = solver_receipt(&request_bytes, "PASS");
        bind_test_artifacts(&mut receipt, &root);
        receipt["outputs"][0]["artifact_ref"] = json!("../outside.json");
        let receipt_bytes = serde_json::to_vec(&receipt).unwrap();

        let report = verify_solver_output_artifact_bytes(&request_bytes, &receipt_bytes, &root)
            .expect("unsafe reference is a verification disposition, not a panic");
        assert_eq!(report["artifact_verification_status"], "UNRESOLVED");
        assert_eq!(report["output_artifact_bytes_verified"], false);
        assert!(
            report["artifact_diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|diagnostic| diagnostic.as_str().unwrap().contains("normalized relative"))
        );

        fs::remove_dir_all(root).expect("temporary artifact root is removed");
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_artifact_cannot_escape_declared_root() {
        let root = temporary_artifact_root();
        let outside = root.with_extension("outside.json");
        let outside_bytes = b"external bytes stay outside the bundle\n";
        fs::write(&outside, outside_bytes).expect("outside test artifact is written");
        std::os::unix::fs::symlink(&outside, root.join("external.json"))
            .expect("symlink test setup succeeds");

        let request_bytes = fixture("analytical-bar-request-v0.1.json");
        let mut receipt = solver_receipt(&request_bytes, "PASS");
        bind_test_artifacts(&mut receipt, &root);
        receipt["outputs"][0]["artifact_ref"] = json!("external.json");
        receipt["outputs"][0]["sha256"] = json!(sha256_hex(outside_bytes));
        let receipt_bytes = serde_json::to_vec(&receipt).unwrap();

        let report = verify_solver_output_artifact_bytes(&request_bytes, &receipt_bytes, &root)
            .expect("symlink escape is rejected as unresolved");
        assert_eq!(report["artifact_verification_status"], "UNRESOLVED");
        assert_eq!(report["output_artifact_bytes_verified"], false);
        assert!(
            report["artifact_diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|diagnostic| diagnostic
                    .as_str()
                    .unwrap()
                    .contains("outside the declared artifact root"))
        );

        fs::remove_dir_all(root).expect("temporary artifact root is removed");
        fs::remove_file(outside).expect("outside test artifact is removed");
    }

    #[test]
    fn receipt_without_output_artifacts_cannot_pass_byte_verification() {
        let root = temporary_artifact_root();
        let request_bytes = fixture("unsupported-dynamic-fracture-request-v0.1.json");
        let receipt = solver_receipt(&request_bytes, "UNRESOLVED");
        let receipt_bytes = serde_json::to_vec(&receipt).unwrap();

        let report = verify_solver_output_artifact_bytes(&request_bytes, &receipt_bytes, &root)
            .expect("a valid unresolved receipt yields an artifact disposition");
        assert_eq!(report["producer_status"], "UNRESOLVED");
        assert_eq!(report["artifact_verification_status"], "UNRESOLVED");
        assert_eq!(report["output_artifact_bytes_verified"], false);

        fs::remove_dir_all(root).expect("temporary artifact root is removed");
    }

    #[test]
    fn invalid_json_and_revision_fail_closed() {
        assert!(validate_request_bytes(b"{", REV).is_err());
        assert!(
            validate_request_bytes(&fixture("analytical-bar-request-v0.1.json"), "not-a-sha")
                .is_err()
        );
    }
}
