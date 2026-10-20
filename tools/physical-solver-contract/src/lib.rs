use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ValidationIssue {
    pub path: String,
    pub code: String,
    pub message: String,
}

fn issue(issues: &mut Vec<ValidationIssue>, path: impl Into<String>, code: &str, message: &str) {
    issues.push(ValidationIssue {
        path: path.into(),
        code: code.to_owned(),
        message: message.to_owned(),
    });
}

fn is_lower_hex(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn is_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn expect_string<'a>(
    value: &'a Value,
    key: &str,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) -> Option<&'a str> {
    match value.get(key).and_then(Value::as_str) {
        Some(s) => Some(s),
        None => {
            issue(
                issues,
                format!("{path}.{key}"),
                "type.string",
                "required field must be a string",
            );
            None
        }
    }
}

fn validate_provenance(value: &Value, path: &str, issues: &mut Vec<ValidationIssue>) {
    let Some(provenance) = value.as_object() else {
        issue(issues, path, "type.object", "provenance must be an object");
        return;
    };
    let evidence = provenance.get("evidence_class").and_then(Value::as_str);
    if !matches!(
        evidence,
        Some("synthetic" | "analytical_reference" | "measured" | "derived" | "unknown")
    ) {
        issue(
            issues,
            format!("{path}.evidence_class"),
            "enum.evidence_class",
            "unknown evidence class",
        );
    }
    if !provenance.contains_key("source_ref") {
        issue(
            issues,
            format!("{path}.source_ref"),
            "required",
            "source_ref must be present; use null when unavailable",
        );
    } else if !provenance["source_ref"].is_null() && !provenance["source_ref"].is_string() {
        issue(
            issues,
            format!("{path}.source_ref"),
            "type.string_or_null",
            "source_ref must be a string or null",
        );
    }
    if provenance
        .get("uncertainty_relative")
        .is_some_and(|uncertainty| {
            !uncertainty.is_null()
                && uncertainty
                    .as_f64()
                    .is_none_or(|n| !n.is_finite() || n < 0.0)
        })
    {
        issue(
            issues,
            format!("{path}.uncertainty_relative"),
            "range.nonnegative",
            "uncertainty_relative must be null or a finite non-negative number",
        );
    }
}

const KNOWN_PHYSICS: &[&str] = &[
    "topology",
    "static_linear_elastic",
    "dynamic_response",
    "nonlinear_material",
    "fracture",
    "rigid_contact",
    "granular_contact",
    "thermal_coupling",
];

fn validate_string_set_field(
    value: &Value,
    key: &str,
    path: &str,
    allowed: &[&str],
    issues: &mut Vec<ValidationIssue>,
) -> Option<BTreeSet<String>> {
    let Some(items) = value.get(key).and_then(Value::as_array) else {
        issue(
            issues,
            format!("{path}.{key}"),
            "type.array",
            "field must be an array of unique strings",
        );
        return None;
    };

    let mut seen = BTreeSet::new();
    for (index, item) in items.iter().enumerate() {
        let item_path = format!("{path}.{key}[{index}]");
        match item.as_str() {
            Some(label) if allowed.contains(&label) => {
                if !seen.insert(label.to_owned()) {
                    issue(
                        issues,
                        item_path,
                        "duplicate",
                        "array entries must be unique",
                    );
                }
            }
            Some(_) => issue(
                issues,
                item_path,
                "enum.label",
                "array contains an unknown label",
            ),
            None => issue(
                issues,
                item_path,
                "type.string",
                "array entries must be strings",
            ),
        }
    }
    Some(seen)
}

fn check_unique_tokens(
    values: &[Value],
    path: &str,
    issues: &mut Vec<ValidationIssue>,
    require_nonempty: bool,
) -> BTreeSet<String> {
    if require_nonempty && values.is_empty() {
        issue(issues, path, "array.empty", "array must not be empty");
    }
    let mut seen = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        let item_path = format!("{path}[{index}]");
        match value.as_str() {
            Some(s) if is_token(s) => {
                if !seen.insert(s.to_owned()) {
                    issue(
                        issues,
                        item_path,
                        "duplicate",
                        "duplicate identifiers are not allowed",
                    );
                }
            }
            Some(_) => issue(
                issues,
                item_path,
                "format.identifier",
                "identifier must use ASCII letters, digits, underscore, period, colon, or hyphen",
            ),
            None => issue(
                issues,
                item_path,
                "type.string",
                "identifier must be a string",
            ),
        }
    }
    seen
}

fn validate_nonnegative_number(
    value: &Value,
    key: &str,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
    strictly_positive: bool,
) {
    match value.get(key).and_then(Value::as_f64) {
        Some(number)
            if number.is_finite()
                && if strictly_positive {
                    number > 0.0
                } else {
                    number >= 0.0
                } => {}
        _ => issue(
            issues,
            format!("{path}.{key}"),
            if strictly_positive {
                "range.positive"
            } else {
                "range.nonnegative"
            },
            if strictly_positive {
                "value must be a finite number greater than zero"
            } else {
                "value must be a finite non-negative number"
            },
        ),
    }
}

/// Validates the contract-critical semantic subset of request v0.1.
///
/// This intentionally does not claim generic JSON Schema Draft 2020-12 conformance.
pub fn validate_request_json(value: &Value) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    if !value.is_object() {
        issue(
            &mut issues,
            "$",
            "type.object",
            "request must be a JSON object",
        );
        return issues;
    }

    let request_object = value.as_object().expect("object checked above");
    for key in [
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
    ] {
        if !request_object.contains_key(key) {
            issue(
                &mut issues,
                format!("$.{key}"),
                "required",
                "required field is missing",
            );
        }
    }

    if expect_string(value, "schema_version", "$", &mut issues)
        .is_some_and(|s| s != "physical-solver-request-v0.1")
    {
        issue(
            &mut issues,
            "$.schema_version",
            "version.unsupported",
            "unsupported request schema version",
        );
    }

    if let Some(id) = value.get("request_id").and_then(Value::as_str) {
        if !is_token(id) {
            issue(
                &mut issues,
                "$.request_id",
                "format.identifier",
                "request_id is not a valid contract identifier",
            );
        }
    } else {
        issue(
            &mut issues,
            "$.request_id",
            "type.string",
            "request_id must be a string",
        );
    }

    if let Some(revision) = value.get("source_revision").and_then(Value::as_str) {
        if !is_lower_hex(revision, 40) {
            issue(
                &mut issues,
                "$.source_revision",
                "format.git_revision",
                "source_revision must be a 40-character lowercase Git SHA",
            );
        }
    } else {
        issue(
            &mut issues,
            "$.source_revision",
            "type.string",
            "source_revision must be a string",
        );
    }

    match value.get("scene_revision").and_then(Value::as_u64) {
        Some(_) => {}
        None => issue(
            &mut issues,
            "$.scene_revision",
            "range.integer",
            "scene_revision must be a non-negative integer",
        ),
    }

    if let Some(digest) = value.get("scene_sha256").and_then(Value::as_str) {
        if !is_lower_hex(digest, 64) {
            issue(
                &mut issues,
                "$.scene_sha256",
                "format.sha256",
                "scene_sha256 must be 64 lowercase hexadecimal characters",
            );
        }
    } else {
        issue(
            &mut issues,
            "$.scene_sha256",
            "type.string",
            "scene_sha256 must be a string",
        );
    }

    let fidelity = value.get("model_fidelity").and_then(Value::as_str);
    if !matches!(
        fidelity,
        Some(
            "graph_topology"
                | "analytical_linear_elastic"
                | "continuum_linear_elastic"
                | "continuum_nonlinear"
                | "phase_field_fracture"
                | "rigid_contact"
                | "granular_dem"
        )
    ) {
        issue(
            &mut issues,
            "$.model_fidelity",
            "enum.model_fidelity",
            "unknown model fidelity class",
        );
    }

    let frame = value.get("coordinate_frame_id").and_then(Value::as_str);
    if !frame.is_some_and(is_token) {
        issue(
            &mut issues,
            "$.coordinate_frame_id",
            "format.identifier",
            "coordinate_frame_id must be a valid identifier",
        );
    }

    let Some(units) = value.get("units").and_then(Value::as_object) else {
        issue(
            &mut issues,
            "$.units",
            "type.object",
            "units must be an object",
        );
        return issues;
    };
    for (key, expected) in [
        ("length", "m"),
        ("mass", "kg"),
        ("time", "s"),
        ("force", "N"),
        ("stress", "Pa"),
    ] {
        if units.get(key).and_then(Value::as_str) != Some(expected) {
            issue(
                &mut issues,
                format!("$.units.{key}"),
                "units.si_required",
                "v0.1 solver requests require the declared SI unit",
            );
        }
    }

    let known_physics = [
        "topology",
        "static_linear_elastic",
        "dynamic_response",
        "nonlinear_material",
        "fracture",
        "rigid_contact",
        "granular_contact",
        "thermal_coupling",
    ];
    match value.get("requested_physics").and_then(Value::as_array) {
        Some(items) => {
            if items.is_empty() {
                issue(
                    &mut issues,
                    "$.requested_physics",
                    "array.empty",
                    "at least one physics capability must be requested",
                );
            }
            let mut seen = BTreeSet::new();
            for (index, item) in items.iter().enumerate() {
                match item.as_str() {
                    Some(s) if known_physics.contains(&s) => {
                        if !seen.insert(s) {
                            issue(
                                &mut issues,
                                format!("$.requested_physics[{index}]"),
                                "duplicate",
                                "requested physics entries must be unique",
                            );
                        }
                    }
                    _ => issue(
                        &mut issues,
                        format!("$.requested_physics[{index}]"),
                        "enum.requested_physics",
                        "unknown or non-string physics capability",
                    ),
                }
            }
        }
        None => issue(
            &mut issues,
            "$.requested_physics",
            "type.array",
            "requested_physics must be an array",
        ),
    }

    if let Some(materials) = value.get("materials").and_then(Value::as_array) {
        let mut material_ids = BTreeSet::new();
        for (index, material) in materials.iter().enumerate() {
            let path = format!("$.materials[{index}]");
            let Some(material) = material.as_object() else {
                issue(
                    &mut issues,
                    &path,
                    "type.object",
                    "material must be an object",
                );
                continue;
            };
            match material.get("material_id").and_then(Value::as_str) {
                Some(id) if is_token(id) => {
                    if !material_ids.insert(id.to_owned()) {
                        issue(
                            &mut issues,
                            format!("{path}.material_id"),
                            "duplicate",
                            "material_id must be unique",
                        );
                    }
                }
                _ => issue(
                    &mut issues,
                    format!("{path}.material_id"),
                    "format.identifier",
                    "material_id must be a valid identifier",
                ),
            }
            if let Some(components) = material.get("component_ids").and_then(Value::as_array) {
                check_unique_tokens(
                    components,
                    &format!("{path}.component_ids"),
                    &mut issues,
                    true,
                );
            } else {
                issue(
                    &mut issues,
                    format!("{path}.component_ids"),
                    "type.array",
                    "component_ids must be an array",
                );
            }

            for key in ["youngs_modulus_pa", "density_kg_m3"] {
                match material.get(key) {
                    Some(v) if v.is_null() => {}
                    Some(v) if v.as_f64().is_some_and(|n| n.is_finite() && n > 0.0) => {}
                    _ => issue(
                        &mut issues,
                        format!("{path}.{key}"),
                        "range.positive_or_null",
                        "property must be null or a finite positive number",
                    ),
                }
            }
            match material.get("poisson_ratio") {
                Some(v) if v.is_null() => {}
                Some(v)
                    if v.as_f64()
                        .is_some_and(|n| n.is_finite() && (-1.0..0.5).contains(&n)) => {}
                _ => issue(
                    &mut issues,
                    format!("{path}.poisson_ratio"),
                    "range.poisson_ratio",
                    "Poisson ratio must be null or satisfy -1 <= nu < 0.5",
                ),
            }
            match material.get("provenance") {
                Some(v) => validate_provenance(v, &format!("{path}.provenance"), &mut issues),
                None => issue(
                    &mut issues,
                    format!("{path}.provenance"),
                    "required",
                    "provenance must be present",
                ),
            }
        }
    } else {
        issue(
            &mut issues,
            "$.materials",
            "type.array",
            "materials must be an array",
        );
    }

    if let Some(loads) = value.get("loads").and_then(Value::as_array) {
        let mut load_ids = BTreeSet::new();
        for (index, load) in loads.iter().enumerate() {
            let path = format!("$.loads[{index}]");
            let Some(load) = load.as_object() else {
                issue(&mut issues, &path, "type.object", "load must be an object");
                continue;
            };
            match load.get("load_id").and_then(Value::as_str) {
                Some(id) if is_token(id) => {
                    if !load_ids.insert(id.to_owned()) {
                        issue(
                            &mut issues,
                            format!("{path}.load_id"),
                            "duplicate",
                            "load_id must be unique",
                        );
                    }
                }
                _ => issue(
                    &mut issues,
                    format!("{path}.load_id"),
                    "format.identifier",
                    "load_id must be a valid identifier",
                ),
            }
            if !matches!(
                load.get("kind").and_then(Value::as_str),
                Some("point_force" | "distributed_force" | "pressure" | "body_acceleration")
            ) {
                issue(
                    &mut issues,
                    format!("{path}.kind"),
                    "enum.load_kind",
                    "unknown load kind",
                );
            }
            let load_frame = load.get("coordinate_frame_id").and_then(Value::as_str);
            if !load_frame.is_some_and(is_token) || load_frame != frame {
                issue(
                    &mut issues,
                    format!("{path}.coordinate_frame_id"),
                    "frame.mismatch",
                    "load frame must be a valid identifier matching the request frame",
                );
            }
            match load.get("vector_xyz").and_then(Value::as_array) {
                Some(vector)
                    if vector.len() == 3
                        && vector
                            .iter()
                            .all(|v| v.as_f64().is_some_and(f64::is_finite)) => {}
                _ => issue(
                    &mut issues,
                    format!("{path}.vector_xyz"),
                    "vector.xyz",
                    "vector_xyz must contain exactly three finite numbers",
                ),
            }
            match load.get("provenance") {
                Some(v) => validate_provenance(v, &format!("{path}.provenance"), &mut issues),
                None => issue(
                    &mut issues,
                    format!("{path}.provenance"),
                    "required",
                    "provenance must be present",
                ),
            }
        }
    } else {
        issue(
            &mut issues,
            "$.loads",
            "type.array",
            "loads must be an array",
        );
    }

    if let Some(conditions) = value.get("boundary_conditions").and_then(Value::as_array) {
        let mut condition_ids = BTreeSet::new();
        for (index, condition) in conditions.iter().enumerate() {
            let path = format!("$.boundary_conditions[{index}]");
            let Some(condition) = condition.as_object() else {
                issue(
                    &mut issues,
                    &path,
                    "type.object",
                    "boundary condition must be an object",
                );
                continue;
            };
            match condition.get("condition_id").and_then(Value::as_str) {
                Some(id) if is_token(id) => {
                    if !condition_ids.insert(id.to_owned()) {
                        issue(
                            &mut issues,
                            format!("{path}.condition_id"),
                            "duplicate",
                            "condition_id must be unique",
                        );
                    }
                }
                _ => issue(
                    &mut issues,
                    format!("{path}.condition_id"),
                    "format.identifier",
                    "condition_id must be a valid identifier",
                ),
            }
            if !matches!(
                condition.get("kind").and_then(Value::as_str),
                Some("fixed" | "pinned" | "roller" | "prescribed_displacement" | "support_link")
            ) {
                issue(
                    &mut issues,
                    format!("{path}.kind"),
                    "enum.boundary_condition_kind",
                    "unknown boundary-condition kind",
                );
            }
            match condition.get("dofs").and_then(Value::as_array) {
                Some(dofs) => {
                    let allowed = ["ux", "uy", "uz", "rx", "ry", "rz"];
                    if dofs.is_empty() {
                        issue(
                            &mut issues,
                            format!("{path}.dofs"),
                            "array.empty",
                            "at least one degree of freedom is required",
                        );
                    }
                    let mut seen = BTreeSet::new();
                    for (dof_index, dof) in dofs.iter().enumerate() {
                        match dof.as_str() {
                            Some(s) if allowed.contains(&s) => {
                                if !seen.insert(s) {
                                    issue(
                                        &mut issues,
                                        format!("{path}.dofs[{dof_index}]"),
                                        "duplicate",
                                        "degrees of freedom must be unique",
                                    );
                                }
                            }
                            _ => issue(
                                &mut issues,
                                format!("{path}.dofs[{dof_index}]"),
                                "enum.dof",
                                "unknown degree of freedom",
                            ),
                        }
                    }
                }
                None => issue(
                    &mut issues,
                    format!("{path}.dofs"),
                    "type.array",
                    "dofs must be an array",
                ),
            }
            match condition.get("provenance") {
                Some(v) => validate_provenance(v, &format!("{path}.provenance"), &mut issues),
                None => issue(
                    &mut issues,
                    format!("{path}.provenance"),
                    "required",
                    "provenance must be present",
                ),
            }
        }
    } else {
        issue(
            &mut issues,
            "$.boundary_conditions",
            "type.array",
            "boundary_conditions must be an array",
        );
    }

    let outputs_allowed = [
        "topology",
        "displacement",
        "stress",
        "reaction_force",
        "crack_path",
        "fragment_state",
        "energy_balance",
    ];
    match value.get("required_outputs").and_then(Value::as_array) {
        Some(outputs) => {
            if outputs.is_empty() {
                issue(
                    &mut issues,
                    "$.required_outputs",
                    "array.empty",
                    "at least one output must be required",
                );
            }
            let mut seen = BTreeSet::new();
            for (index, output) in outputs.iter().enumerate() {
                match output.as_str() {
                    Some(s) if outputs_allowed.contains(&s) => {
                        if !seen.insert(s) {
                            issue(
                                &mut issues,
                                format!("$.required_outputs[{index}]"),
                                "duplicate",
                                "required outputs must be unique",
                            );
                        }
                    }
                    _ => issue(
                        &mut issues,
                        format!("$.required_outputs[{index}]"),
                        "enum.required_output",
                        "unknown or non-string required output",
                    ),
                }
            }
        }
        None => issue(
            &mut issues,
            "$.required_outputs",
            "type.array",
            "required_outputs must be an array",
        ),
    }

    if let Some(acceptance) = value.get("acceptance") {
        if !acceptance.is_object() {
            issue(
                &mut issues,
                "$.acceptance",
                "type.object",
                "acceptance must be an object",
            );
        } else {
            validate_nonnegative_number(
                acceptance,
                "relative_tolerance",
                "$.acceptance",
                &mut issues,
                false,
            );
            validate_nonnegative_number(
                acceptance,
                "absolute_tolerance",
                "$.acceptance",
                &mut issues,
                false,
            );
            if !acceptance
                .get("require_convergence")
                .is_some_and(Value::is_boolean)
            {
                issue(
                    &mut issues,
                    "$.acceptance.require_convergence",
                    "type.boolean",
                    "require_convergence must be a boolean",
                );
            }
            if acceptance
                .get("max_iterations")
                .is_some_and(|v| v.as_u64().is_none_or(|n| n == 0))
            {
                issue(
                    &mut issues,
                    "$.acceptance.max_iterations",
                    "range.positive_integer",
                    "max_iterations must be a positive integer",
                );
            }
            if acceptance.get("max_time_step_s").is_some() {
                validate_nonnegative_number(
                    acceptance,
                    "max_time_step_s",
                    "$.acceptance",
                    &mut issues,
                    true,
                );
            }
        }
    } else {
        issue(
            &mut issues,
            "$.acceptance",
            "required",
            "acceptance must be present",
        );
    }

    issues
}

fn compatible_physics(fidelity: &str, physics: &str) -> bool {
    match fidelity {
        "graph_topology" => physics == "topology",
        "analytical_linear_elastic" | "continuum_linear_elastic" => {
            physics == "static_linear_elastic"
        }
        "continuum_nonlinear" => {
            matches!(physics, "static_linear_elastic" | "nonlinear_material")
        }
        "phase_field_fracture" => {
            matches!(physics, "static_linear_elastic" | "fracture")
        }
        "rigid_contact" => physics == "rigid_contact",
        "granular_dem" => physics == "granular_contact",
        _ => false,
    }
}

fn has_text(value: &Value, key: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_str)
        .is_some_and(|s| !s.trim().is_empty())
}

/// Validates the receipt structure and, when supplied, its exact request-byte binding.
///
/// This verifies declared evidence fields and consistency; it cannot prove that an external
/// executable actually ran or that the physical model is valid.
pub fn validate_receipt_json(
    receipt: &Value,
    request: Option<&Value>,
    request_bytes: Option<&[u8]>,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    if !receipt.is_object() {
        issue(
            &mut issues,
            "$",
            "type.object",
            "receipt must be a JSON object",
        );
        return issues;
    }

    for key in [
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
    ] {
        if receipt.get(key).is_none() {
            issue(
                &mut issues,
                format!("$.{key}"),
                "required",
                "required field is missing",
            );
        }
    }

    if expect_string(receipt, "schema_version", "$", &mut issues)
        .is_some_and(|s| s != "physical-solver-receipt-v0.1")
    {
        issue(
            &mut issues,
            "$.schema_version",
            "version.unsupported",
            "unsupported receipt schema version",
        );
    }
    if let Some(id) = receipt.get("request_id").and_then(Value::as_str) {
        if !is_token(id) {
            issue(
                &mut issues,
                "$.request_id",
                "format.identifier",
                "request_id is not a valid contract identifier",
            );
        }
    } else {
        issue(
            &mut issues,
            "$.request_id",
            "type.string",
            "request_id must be a string",
        );
    }
    if let Some(digest) = receipt.get("request_sha256").and_then(Value::as_str) {
        if !is_lower_hex(digest, 64) {
            issue(
                &mut issues,
                "$.request_sha256",
                "format.sha256",
                "request_sha256 must be 64 lowercase hexadecimal characters",
            );
        }
    } else {
        issue(
            &mut issues,
            "$.request_sha256",
            "type.string",
            "request_sha256 must be a string",
        );
    }
    if let Some(revision) = receipt.get("source_revision").and_then(Value::as_str) {
        if !is_lower_hex(revision, 40) {
            issue(
                &mut issues,
                "$.source_revision",
                "format.git_revision",
                "source_revision must be a 40-character lowercase Git SHA",
            );
        }
    } else {
        issue(
            &mut issues,
            "$.source_revision",
            "type.string",
            "source_revision must be a string",
        );
    }

    let status = receipt.get("status").and_then(Value::as_str);
    if !matches!(status, Some("PASS" | "FAIL" | "UNRESOLVED")) {
        issue(
            &mut issues,
            "$.status",
            "enum.status",
            "status must be PASS, FAIL, or UNRESOLVED",
        );
    }
    if !matches!(
        receipt.get("evidence_class").and_then(Value::as_str),
        Some(
            "synthetic" | "analytical_reference" | "simulated" | "measured_comparison" | "unknown"
        )
    ) {
        issue(
            &mut issues,
            "$.evidence_class",
            "enum.evidence_class",
            "unknown receipt evidence class",
        );
    }

    let evaluated = validate_string_set_field(
        receipt,
        "physics_evaluated",
        "$",
        KNOWN_PHYSICS,
        &mut issues,
    );
    let omitted = validate_string_set_field(
        receipt,
        "physics_omitted",
        "$",
        KNOWN_PHYSICS,
        &mut issues,
    );
    if let (Some(evaluated), Some(omitted)) = (&evaluated, &omitted) {
        for physics in evaluated.intersection(omitted) {
            issue(
                &mut issues,
                "$.physics_evaluated",
                "physics.overlap",
                &format!("physics capability '{physics}' cannot be both evaluated and omitted"),
            );
        }
    }

    let Some(adapter) = receipt.get("adapter").and_then(Value::as_object) else {
        issue(
            &mut issues,
            "$.adapter",
            "type.object",
            "adapter must be an object",
        );
        return issues;
    };
    for key in ["adapter_id", "version"] {
        if adapter
            .get(key)
            .and_then(Value::as_str)
            .is_none_or(|s| s.trim().is_empty())
        {
            issue(
                &mut issues,
                format!("$.adapter.{key}"),
                "required.string",
                "adapter identity fields must be non-empty strings",
            );
        }
    }
    for key in ["executable_sha256", "configuration_sha256"] {
        if !adapter
            .get(key)
            .and_then(Value::as_str)
            .is_some_and(|s| is_lower_hex(s, 64))
        {
            issue(
                &mut issues,
                format!("$.adapter.{key}"),
                "format.sha256",
                "adapter digest must be 64 lowercase hexadecimal characters",
            );
        }
    }

    let Some(convergence) = receipt.get("convergence").and_then(Value::as_object) else {
        issue(
            &mut issues,
            "$.convergence",
            "type.object",
            "convergence must be an object",
        );
        return issues;
    };
    let convergence_state = convergence.get("state").and_then(Value::as_str);
    if !matches!(
        convergence_state,
        Some("not_applicable" | "converged" | "not_converged" | "unknown")
    ) {
        issue(
            &mut issues,
            "$.convergence.state",
            "enum.convergence",
            "unknown convergence state",
        );
    }
    if convergence
        .get("iterations")
        .is_some_and(|iterations| !iterations.is_null() && iterations.as_u64().is_none())
    {
        issue(
            &mut issues,
            "$.convergence.iterations",
            "range.integer",
            "iterations must be null or a non-negative integer",
        );
    }
    if convergence.get("residual").is_some_and(|residual| {
        !residual.is_null() && residual.as_f64().is_none_or(|n| !n.is_finite() || n < 0.0)
    }) {
        issue(
            &mut issues,
            "$.convergence.residual",
            "range.nonnegative",
            "residual must be null or a finite non-negative number",
        );
    }
    if !convergence
        .get("mesh_refinement_evaluated")
        .is_some_and(Value::is_boolean)
    {
        issue(
            &mut issues,
            "$.convergence.mesh_refinement_evaluated",
            "type.boolean",
            "mesh_refinement_evaluated must be a boolean",
        );
    }
    if !has_text(&Value::Object(convergence.clone()), "details") {
        issue(
            &mut issues,
            "$.convergence.details",
            "required.string",
            "convergence details must be a non-empty string",
        );
    }

    let Some(outputs) = receipt.get("outputs").and_then(Value::as_array) else {
        issue(
            &mut issues,
            "$.outputs",
            "type.array",
            "outputs must be an array",
        );
        return issues;
    };
    let mut output_ids = BTreeSet::new();
    for (index, output) in outputs.iter().enumerate() {
        let path = format!("$.outputs[{index}]");
        let Some(output) = output.as_object() else {
            issue(
                &mut issues,
                &path,
                "type.object",
                "output must be an object",
            );
            continue;
        };
        for key in ["output_id", "kind", "media_type", "artifact_ref"] {
            if output
                .get(key)
                .and_then(Value::as_str)
                .is_none_or(|s| s.trim().is_empty())
            {
                issue(
                    &mut issues,
                    format!("{path}.{key}"),
                    "required.string",
                    "output field must be a non-empty string",
                );
            }
        }
        match output.get("output_id").and_then(Value::as_str) {
            Some(id) if is_token(id) => {
                if !output_ids.insert(id.to_owned()) {
                    issue(
                        &mut issues,
                        format!("{path}.output_id"),
                        "duplicate",
                        "output_id must be unique",
                    );
                }
            }
            _ => issue(
                &mut issues,
                format!("{path}.output_id"),
                "format.identifier",
                "output_id must be a valid identifier",
            ),
        }
        if !output
            .get("sha256")
            .and_then(Value::as_str)
            .is_some_and(|s| is_lower_hex(s, 64))
        {
            issue(
                &mut issues,
                format!("{path}.sha256"),
                "format.sha256",
                "output sha256 must be 64 lowercase hexadecimal characters",
            );
        }
        if output
            .get("unit")
            .is_some_and(|v| !v.is_null() && !v.is_string())
        {
            issue(
                &mut issues,
                format!("{path}.unit"),
                "type.string_or_null",
                "unit must be a string or null",
            );
        }
    }

    if !has_text(receipt, "claim_ceiling") {
        issue(
            &mut issues,
            "$.claim_ceiling",
            "required.string",
            "claim_ceiling must be non-empty",
        );
    }
    if status == Some("UNRESOLVED") {
        let ceiling = receipt
            .get("claim_ceiling")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if !["unresolved", "not qualified", "not validated", "unknown"]
            .iter()
            .any(|needle| ceiling.contains(needle))
        {
            issue(
                &mut issues,
                "$.claim_ceiling",
                "claim.unresolved_required",
                "UNRESOLVED receipts must say their claim is unresolved or unqualified",
            );
        }
    }
    if status == Some("PASS") {
        if outputs.is_empty() {
            issue(
                &mut issues,
                "$.outputs",
                "pass.requires_outputs",
                "PASS requires at least one output artifact",
            );
        }
        if !matches!(convergence_state, Some("converged" | "not_applicable")) {
            issue(
                &mut issues,
                "$.convergence.state",
                "pass.requires_acceptable_convergence",
                "PASS requires converged or explicitly not-applicable convergence",
            );
        }
        for key in ["solver_name", "solver_version", "solver_build_ref"] {
            if adapter
                .get(key)
                .and_then(Value::as_str)
                .is_none_or(|s| s.trim().is_empty())
            {
                issue(
                    &mut issues,
                    format!("$.adapter.{key}"),
                    "pass.requires_solver_identity",
                    "PASS requires a named solver, version, and build reference",
                );
            }
        }
    }

    if let (Some(request), Some(request_bytes)) = (request, request_bytes) {
        for request_issue in validate_request_json(request) {
            issue(
                &mut issues,
                format!("request{}", request_issue.path.trim_start_matches('$')),
                &request_issue.code,
                &request_issue.message,
            );
        }
        let raw_digest = sha256_hex(request_bytes);
        if receipt.get("request_sha256").and_then(Value::as_str) != Some(raw_digest.as_str()) {
            issue(
                &mut issues,
                "$.request_sha256",
                "binding.request_digest",
                "receipt request_sha256 does not match the exact UTF-8 request bytes",
            );
        }
        for key in ["request_id", "source_revision"] {
            if receipt.get(key) != request.get(key) {
                issue(
                    &mut issues,
                    format!("$.{key}"),
                    "binding.request_field",
                    "receipt field must exactly match the bound request",
                );
            }
        }

        if let Some(physics) = request.get("requested_physics").and_then(Value::as_array) {
            let requested: BTreeSet<String> = physics
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect();

            if let (Some(evaluated_set), Some(omitted_set)) = (&evaluated, &omitted) {
                let reported: BTreeSet<String> = evaluated_set
                    .union(omitted_set)
                    .cloned()
                    .collect();

                for extra in reported.difference(&requested) {
                    issue(
                        &mut issues,
                        "$.physics_evaluated",
                        "physics.partition_extra",
                        &format!("receipt reports unrequested physics capability '{extra}'"),
                    );
                }
                for missing in requested.difference(&reported) {
                    issue(
                        &mut issues,
                        "$.physics_evaluated",
                        "physics.partition_missing",
                        &format!("receipt does not classify requested physics capability '{missing}'"),
                    );
                }

                if status == Some("PASS") {
                    for physics_name in &requested {
                        if !compatible_physics(
                            request
                                .get("model_fidelity")
                                .and_then(Value::as_str)
                                .unwrap_or(""),
                            physics_name,
                        ) {
                            issue(
                                &mut issues,
                                "$.status",
                                "pass.unsupported_physics",
                                "PASS is forbidden when requested physics is incompatible with model fidelity",
                            );
                        }
                        if !evaluated_set.contains(physics_name) || omitted_set.contains(physics_name) {
                            issue(
                                &mut issues,
                                "$.physics_evaluated",
                                "pass.physics_not_evaluated",
                                &format!("PASS requires requested physics '{physics_name}' to be evaluated and not omitted"),
                            );
                        }
                    }
                }
            }
        }

        if status == Some("PASS") {
            if let (Some(required), Some(outputs)) = (
                request.get("required_outputs").and_then(Value::as_array),
                receipt.get("outputs").and_then(Value::as_array),
            ) {
                for (index, required_output) in required.iter().enumerate() {
                    let kind = required_output.as_str().unwrap_or("");
                    if !outputs
                        .iter()
                        .any(|output| output.get("kind").and_then(Value::as_str) == Some(kind))
                    {
                        issue(
                            &mut issues,
                            format!("request.required_outputs[{index}]"),
                            "pass.required_output_missing",
                            "PASS receipt is missing a required output kind",
                        );
                    }
                }
            }
        }
    } else if request.is_some() != request_bytes.is_some() {
        issue(
            &mut issues,
            "$.request_sha256",
            "binding.incomplete",
            "request JSON and its exact source bytes must be supplied together",
        );
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    const BAR_REQUEST: &str = include_str!(
        "../../../schemas/physical-transformation/fixtures/analytical-bar-request-v0.1.json"
    );
    const UNSUPPORTED_REQUEST: &str = include_str!(
        "../../../schemas/physical-transformation/fixtures/unsupported-dynamic-fracture-request-v0.1.json"
    );
    const UNRESOLVED_RECEIPT: &str = include_str!(
        "../../../schemas/physical-transformation/fixtures/unresolved-receipt-v0.1.json"
    );

    #[test]
    fn analytical_bar_request_is_contract_valid_but_not_a_solver_result() {
        let request: Value = serde_json::from_str(BAR_REQUEST).unwrap();
        assert!(validate_request_json(&request).is_empty());
    }

    #[test]
    fn unsupported_physics_request_is_validly_described_not_auto_promoted() {
        let request: Value = serde_json::from_str(UNSUPPORTED_REQUEST).unwrap();
        assert!(validate_request_json(&request).is_empty());
        assert!(!compatible_physics(
            "analytical_linear_elastic",
            "dynamic_response"
        ));
        assert!(!compatible_physics("analytical_linear_elastic", "fracture"));
    }

    #[test]
    fn wrong_units_and_frame_mismatch_are_rejected() {
        let mut request: Value = serde_json::from_str(BAR_REQUEST).unwrap();
        request["units"]["length"] = Value::String("mm".to_owned());
        request["loads"][0]["coordinate_frame_id"] = Value::String("other-frame".to_owned());
        let issues = validate_request_json(&request);
        assert!(issues.iter().any(|i| i.code == "units.si_required"));
        assert!(issues.iter().any(|i| i.code == "frame.mismatch"));
    }

    #[test]
    fn duplicate_load_ids_and_dofs_are_rejected() {
        let mut request: Value = serde_json::from_str(BAR_REQUEST).unwrap();
        let duplicate_load = request["loads"][0].clone();
        request["loads"]
            .as_array_mut()
            .unwrap()
            .push(duplicate_load);
        request["boundary_conditions"][0]["dofs"] = serde_json::json!(["ux", "ux"]);
        let issues = validate_request_json(&request);
        assert!(issues.iter().any(|i| i.code == "duplicate"));
    }

    #[test]
    fn unresolved_receipt_is_well_shaped_without_proving_execution() {
        let receipt: Value = serde_json::from_str(UNRESOLVED_RECEIPT).unwrap();
        assert!(validate_receipt_json(&receipt, None, None).is_empty());
    }

    #[test]
    fn receipt_is_bound_to_exact_request_bytes_and_identity() {
        let mut request: Value = serde_json::from_str(UNSUPPORTED_REQUEST).unwrap();
        let request_bytes = serde_json::to_vec(&request).unwrap();
        let mut receipt: Value = serde_json::from_str(UNRESOLVED_RECEIPT).unwrap();
        receipt["request_sha256"] = Value::String(sha256_hex(&request_bytes));
        receipt["request_id"] = request["request_id"].clone();
        receipt["source_revision"] = request["source_revision"].clone();
        receipt["claim_ceiling"] =
            Value::String("UNRESOLVED: unsupported dynamic fracture is not evaluated.".to_owned());
        assert!(validate_receipt_json(&receipt, Some(&request), Some(&request_bytes)).is_empty());

        request["scene_revision"] = Value::from(2);
        let mismatched_bytes = serde_json::to_vec(&request).unwrap();
        let issues = validate_receipt_json(&receipt, Some(&request), Some(&mismatched_bytes));
        assert!(issues.iter().any(|i| i.code == "binding.request_digest"));
    }

    #[test]
    fn receipt_physics_partition_must_exactly_cover_requested_physics() {
        let request: Value = serde_json::from_str(UNSUPPORTED_REQUEST).unwrap();
        let request_bytes = serde_json::to_vec(&request).unwrap();
        let mut receipt: Value = serde_json::from_str(UNRESOLVED_RECEIPT).unwrap();
        receipt["request_sha256"] = Value::String(sha256_hex(&request_bytes));
        receipt["request_id"] = request["request_id"].clone();
        receipt["source_revision"] = request["source_revision"].clone();

        assert!(validate_receipt_json(&receipt, Some(&request), Some(&request_bytes)).is_empty());

        // Reporting a capability that was not requested must not be silently accepted.
        receipt["physics_evaluated"] = serde_json::json!(["topology"]);
        let extra = validate_receipt_json(&receipt, Some(&request), Some(&request_bytes));
        assert!(extra.iter().any(|i| i.code == "physics.partition_extra"));
        assert!(extra.iter().any(|i| i.code == "physics.partition_missing"));

        // Each requested physics capability must be classified exactly once.
        receipt["physics_evaluated"] = serde_json::json!(["dynamic_response"]);
        receipt["physics_omitted"] = serde_json::json!(["dynamic_response", "fracture"]);
        let overlap = validate_receipt_json(&receipt, Some(&request), Some(&request_bytes));
        assert!(overlap.iter().any(|i| i.code == "physics.overlap"));
    }

    #[test]
    fn physics_lists_reject_unknown_labels_duplicates_and_wrong_types() {
        let mut receipt: Value = serde_json::from_str(UNRESOLVED_RECEIPT).unwrap();

        receipt["physics_evaluated"] = serde_json::json!(["unknown_model"]);
        let unknown = validate_receipt_json(&receipt, None, None);
        assert!(unknown.iter().any(|i| i.code == "enum.label"));

        receipt["physics_evaluated"] = serde_json::json!([]);
        receipt["physics_omitted"] = serde_json::json!(["fracture", "fracture"]);
        let duplicate = validate_receipt_json(&receipt, None, None);
        assert!(duplicate.iter().any(|i| i.code == "duplicate"));

        receipt["physics_omitted"] = Value::Null;
        let wrong_type = validate_receipt_json(&receipt, None, None);
        assert!(wrong_type.iter().any(|i| i.code == "type.array"));
    }

    #[test]
    fn pass_receipt_cannot_claim_unsupported_physics_or_missing_outputs() {
        let request: Value = serde_json::from_str(UNSUPPORTED_REQUEST).unwrap();
        let request_bytes = serde_json::to_vec(&request).unwrap();
        let mut receipt: Value = serde_json::from_str(UNRESOLVED_RECEIPT).unwrap();
        receipt["request_sha256"] = Value::String(sha256_hex(&request_bytes));
        receipt["request_id"] = request["request_id"].clone();
        receipt["source_revision"] = request["source_revision"].clone();
        receipt["status"] = Value::String("PASS".to_owned());
        receipt["claim_ceiling"] = Value::String("Validation pass.".to_owned());
        receipt["convergence"]["state"] = Value::String("converged".to_owned());
        receipt["adapter"]["solver_name"] = Value::String("fictional".to_owned());
        receipt["adapter"]["solver_version"] = Value::String("0.0".to_owned());
        receipt["adapter"]["solver_build_ref"] = Value::String("test-only".to_owned());

        let issues = validate_receipt_json(&receipt, Some(&request), Some(&request_bytes));
        assert!(issues.iter().any(|i| i.code == "pass.requires_outputs"));
        assert!(issues.iter().any(|i| i.code == "pass.unsupported_physics"));
    }
}
