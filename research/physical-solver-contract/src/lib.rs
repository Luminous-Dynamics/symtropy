// SPDX-License-Identifier: AGPL-3.0-or-later
//! Validate request contracts; never mistake request validation for physics.
//!
//! Every receipt produced here has physical_result_status=UNRESOLVED because
//! this crate does not invoke a qualified mechanics solver.

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub type Check<T> = Result<T, String>;
const REQUEST_SCHEMA: &str = "physical-solver-request-v0.1";
const RECEIPT_SCHEMA: &str = "physical-solver-validation-receipt-v0.1";
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
    #[test]
    fn invalid_json_and_revision_fail_closed() {
        assert!(validate_request_bytes(b"{", REV).is_err());
        assert!(
            validate_request_bytes(&fixture("analytical-bar-request-v0.1.json"), "not-a-sha")
                .is_err()
        );
    }
}
