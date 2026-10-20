// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fail-closed capability matching for versioned physical solver backends.
//!
//! Version 0.1 executes only the in-process analytical reference backend.
//! External processes and remote backends can be described in future manifests,
//! but are not selected by this implementation.

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub type Check<T> = Result<T, String>;

const MANIFEST: &str =
    include_str!("../../../schemas/physical-transformation/physical-solver-capabilities-v0.1.json");
const MANIFEST_SCHEMA: &str = "physical-solver-capabilities-v0.1";
const MANIFEST_KEYS: &[&str] = &[
    "schema_version",
    "manifest_id",
    "manifest_version",
    "backends",
];
const BACKEND_KEYS: &[&str] = &[
    "backend_id",
    "backend_version",
    "implementation_kind",
    "dispatch_eligible",
    "qualification_tier",
    "supported_physics",
    "supported_units",
    "cases",
    "validation_evidence",
    "real_world_validation_status",
    "external_solver_invoked",
    "claim_ceiling",
];
const CASE_KEYS: &[&str] = &[
    "case_id",
    "assumption_profile",
    "outputs",
    "known_answer_fixtures",
];
const UNIT_KEYS: &[&str] = &[
    "length",
    "force",
    "area",
    "stress",
    "second_moment",
    "distributed_load",
];
const EXPECTED_UNITS: &[(&str, &str)] = &[
    ("length", "m"),
    ("force", "N"),
    ("area", "m^2"),
    ("stress", "Pa"),
    ("second_moment", "m^4"),
    ("distributed_load", "N/m"),
];
const KNOWN_PHYSICS: &[&str] = &[
    "static_linear_elastic",
    "dynamic_response",
    "fracture",
    "rigid_contact",
    "granular_contact",
    "thermal_coupling",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionStatus {
    Selected,
    Unresolved,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resolution {
    pub status: SelectionStatus,
    pub selected_backend: Option<Value>,
    pub manifest_sha256: String,
    pub outputs: Vec<String>,
    pub physics_omitted: Vec<String>,
    pub diagnostics: Vec<String>,
}

/// Resolve a case against the repository-pinned capability manifest.
///
/// Selection requires exact case/profile and unit compatibility, every requested
/// physics class, and the qualified in-process reference tier. No best-effort
/// fallback is performed when any requirement is unsupported.
pub fn resolve_request(
    case_id: &str,
    assumption_profile: &str,
    requested_physics: &[String],
    units: &Map<String, Value>,
) -> Check<Resolution> {
    let manifest: Value = serde_json::from_str(MANIFEST)
        .map_err(|error| format!("pinned capability manifest JSON is invalid: {error}"))?;
    resolve_against_manifest(
        &manifest,
        case_id,
        assumption_profile,
        requested_physics,
        units,
    )
}

fn resolve_against_manifest(
    manifest: &Value,
    case_id: &str,
    assumption_profile: &str,
    requested_physics: &[String],
    units: &Map<String, Value>,
) -> Check<Resolution> {
    validate_manifest(manifest)?;
    let digest = sha256_hex(MANIFEST.as_bytes());
    if requested_physics.is_empty() {
        return Ok(unresolved(
            digest,
            Vec::new(),
            vec!["requested physics must not be empty".to_owned()],
        ));
    }

    let backends = manifest
        .get("backends")
        .and_then(Value::as_array)
        .ok_or_else(|| "manifest.backends must be an array".to_owned())?;
    let mut compatible = Vec::<(Value, Vec<String>)>::new();
    let mut profile_seen = false;
    let mut unit_compatible_case_seen = false;
    let mut physics_omitted = BTreeSet::new();

    for backend in backends {
        let cases = backend
            .get("cases")
            .and_then(Value::as_array)
            .ok_or_else(|| "manifest backend cases must be an array".to_owned())?;
        let case_record = cases
            .iter()
            .find(|entry| entry.get("case_id").and_then(Value::as_str) == Some(case_id));
        let Some(case_record) = case_record else {
            continue;
        };
        if case_record
            .get("assumption_profile")
            .and_then(Value::as_str)
            != Some(assumption_profile)
        {
            continue;
        }
        profile_seen = true;

        if !units_match(
            units,
            backend
                .get("supported_units")
                .and_then(Value::as_object)
                .ok_or_else(|| "manifest backend supported_units must be an object".to_owned())?,
        ) {
            continue;
        }
        unit_compatible_case_seen = true;

        let supported_physics = string_array(backend, "supported_physics", "backend")?;
        let missing_physics: Vec<String> = requested_physics
            .iter()
            .filter(|physics| !supported_physics.contains(physics))
            .cloned()
            .collect();
        if !missing_physics.is_empty() {
            physics_omitted.extend(missing_physics);
            continue;
        }

        // This dispatcher v0.1 is intentionally not a general external process
        // launcher. An external or remote backend must gain its own audited
        // execution adapter before it becomes dispatch-eligible here.
        let eligible = backend
            .get("dispatch_eligible")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let implementation = string(backend, "implementation_kind", "backend")?;
        let tier = string(backend, "qualification_tier", "backend")?;
        if !eligible
            || implementation != "in_process_reference"
            || tier != "closed_form_equation_contract"
        {
            continue;
        }

        let outputs = case_record
            .get("outputs")
            .and_then(Value::as_array)
            .ok_or_else(|| "manifest case outputs must be an array".to_owned())?
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| "manifest case output must be a string".to_owned())
            })
            .collect::<Check<Vec<_>>>()?;
        compatible.push((backend.clone(), outputs));
    }

    if compatible.len() == 1 {
        let (backend, outputs) = compatible.pop().expect("one compatible backend");
        return Ok(Resolution {
            status: SelectionStatus::Selected,
            selected_backend: Some(backend),
            manifest_sha256: digest,
            outputs,
            physics_omitted: Vec::new(),
            diagnostics: Vec::new(),
        });
    }
    if compatible.len() > 1 {
        return Ok(unresolved(
            digest,
            Vec::new(),
            vec![format!(
                "ambiguous dispatch: {} qualified backends match the complete request",
                compatible.len()
            )],
        ));
    }

    let mut diagnostics = Vec::new();
    if !physics_omitted.is_empty() {
        diagnostics.push("no qualified backend supports every requested physics class".to_owned());
    } else if !profile_seen {
        diagnostics.push(format!(
            "no registered backend supports case '{case_id}' with assumption profile '{assumption_profile}'"
        ));
    } else if !unit_compatible_case_seen {
        diagnostics.push("request units do not match a registered backend capability".to_owned());
    } else {
        diagnostics
            .push("matching backend exists but is not qualified for automatic dispatch".to_owned());
    }
    Ok(unresolved(
        digest,
        physics_omitted.into_iter().collect(),
        diagnostics,
    ))
}

fn unresolved(
    manifest_sha256: String,
    physics_omitted: Vec<String>,
    diagnostics: Vec<String>,
) -> Resolution {
    Resolution {
        status: SelectionStatus::Unresolved,
        selected_backend: None,
        manifest_sha256,
        outputs: Vec::new(),
        physics_omitted,
        diagnostics,
    }
}

fn validate_manifest(manifest: &Value) -> Check<()> {
    exact_keys(manifest, MANIFEST_KEYS, "manifest")?;
    expect_string(manifest, "schema_version", MANIFEST_SCHEMA, "manifest")?;
    nonempty_string(manifest, "manifest_id", "manifest")?;
    nonempty_string(manifest, "manifest_version", "manifest")?;

    let backends = manifest
        .get("backends")
        .and_then(Value::as_array)
        .ok_or_else(|| "manifest.backends must be an array".to_owned())?;
    if backends.is_empty() {
        return Err("manifest.backends must not be empty".to_owned());
    }

    let mut backend_ids = BTreeSet::new();
    for (backend_index, backend) in backends.iter().enumerate() {
        let at = format!("manifest.backends[{backend_index}]");
        exact_keys(backend, BACKEND_KEYS, &at)?;
        let id = nonempty_string(backend, "backend_id", &at)?;
        if !backend_ids.insert(id) {
            return Err(format!("{at}.backend_id is duplicated"));
        }
        nonempty_string(backend, "backend_version", &at)?;
        let implementation = string(backend, "implementation_kind", &at)?;
        if !["in_process_reference", "external_process", "remote_service"]
            .contains(&implementation.as_str())
        {
            return Err(format!("{at}.implementation_kind is unsupported"));
        }
        if backend
            .get("dispatch_eligible")
            .and_then(Value::as_bool)
            .is_none()
        {
            return Err(format!("{at}.dispatch_eligible must be boolean"));
        }
        let tier = string(backend, "qualification_tier", &at)?;
        if ![
            "unqualified",
            "closed_form_equation_contract",
            "numerical_solver_verified",
            "experimentally_validated",
        ]
        .contains(&tier.as_str())
        {
            return Err(format!("{at}.qualification_tier is unsupported"));
        }
        let physics = string_array(backend, "supported_physics", &at)?;
        unique_strings(&physics, &format!("{at}.supported_physics"))?;
        if physics.is_empty() {
            return Err(format!("{at}.supported_physics must not be empty"));
        }
        if let Some(unsupported) = physics
            .iter()
            .find(|physics| !KNOWN_PHYSICS.contains(&physics.as_str()))
        {
            return Err(format!(
                "{at}.supported_physics contains unsupported value '{unsupported}'"
            ));
        }

        let supported_units = backend
            .get("supported_units")
            .and_then(Value::as_object)
            .ok_or_else(|| format!("{at}.supported_units must be an object"))?;
        exact_keys_map(supported_units, UNIT_KEYS, &format!("{at}.supported_units"))?;
        for (key, expected_unit) in EXPECTED_UNITS {
            let actual =
                nonempty_map_string(supported_units, key, &format!("{at}.supported_units"))?;
            if actual != *expected_unit {
                return Err(format!(
                    "{at}.supported_units.{key} must be '{expected_unit}', got '{actual}'"
                ));
            }
        }

        let cases = backend
            .get("cases")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{at}.cases must be an array"))?;
        if cases.is_empty() {
            return Err(format!("{at}.cases must not be empty"));
        }
        let mut case_ids = BTreeSet::new();
        for (case_index, case) in cases.iter().enumerate() {
            let case_at = format!("{at}.cases[{case_index}]");
            exact_keys(case, CASE_KEYS, &case_at)?;
            let case_id = nonempty_string(case, "case_id", &case_at)?;
            if !case_ids.insert(case_id) {
                return Err(format!("{case_at}.case_id is duplicated"));
            }
            nonempty_string(case, "assumption_profile", &case_at)?;
            let outputs = string_array(case, "outputs", &case_at)?;
            if outputs.is_empty() {
                return Err(format!("{case_at}.outputs must not be empty"));
            }
            unique_strings(&outputs, &format!("{case_at}.outputs"))?;
            let fixtures = string_array(case, "known_answer_fixtures", &case_at)?;
            if fixtures.is_empty() {
                return Err(format!("{case_at}.known_answer_fixtures must not be empty"));
            }
        }

        let evidence = string_array(backend, "validation_evidence", &at)?;
        if evidence.is_empty() {
            return Err(format!("{at}.validation_evidence must not be empty"));
        }
        let real_world_status = string(backend, "real_world_validation_status", &at)?;
        if ![
            "UNRESOLVED",
            "PARTIALLY_VALIDATED",
            "VALIDATED_WITHIN_DECLARED_ENVELOPE",
        ]
        .contains(&real_world_status.as_str())
        {
            return Err(format!("{at}.real_world_validation_status is unsupported"));
        }
        let external_solver_invoked = backend
            .get("external_solver_invoked")
            .and_then(Value::as_bool)
            .ok_or_else(|| format!("{at}.external_solver_invoked must be boolean"))?;
        if implementation == "in_process_reference" && external_solver_invoked {
            return Err(format!(
                "{at}.external_solver_invoked must be false for an in-process reference backend"
            ));
        }
        nonempty_string(backend, "claim_ceiling", &at)?;
    }
    Ok(())
}

fn units_match(request: &Map<String, Value>, capability: &Map<String, Value>) -> bool {
    UNIT_KEYS.iter().all(|key| {
        request.get(*key).and_then(Value::as_str) == capability.get(*key).and_then(Value::as_str)
    })
}

fn exact_keys(value: &Value, expected: &[&str], at: &str) -> Check<()> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{at} must be an object"))?;
    exact_keys_map(object, expected, at)
}

fn exact_keys_map(actual: &Map<String, Value>, expected: &[&str], at: &str) -> Check<()> {
    let expected_set: BTreeSet<&str> = expected.iter().copied().collect();
    let actual_set: BTreeSet<&str> = actual.keys().map(String::as_str).collect();
    let missing: Vec<_> = expected_set.difference(&actual_set).copied().collect();
    let unknown: Vec<_> = actual_set.difference(&expected_set).copied().collect();
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

fn nonempty_string(value: &Value, key: &str, at: &str) -> Check<String> {
    let output = string(value, key, at)?;
    if output.trim().is_empty() {
        return Err(format!("{at}.{key} must not be empty"));
    }
    Ok(output)
}

fn nonempty_map_string(value: &Map<String, Value>, key: &str, at: &str) -> Check<String> {
    let output = value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{at}.{key} must be a string"))?;
    if output.trim().is_empty() {
        return Err(format!("{at}.{key} must not be empty"));
    }
    Ok(output)
}

fn expect_string(value: &Value, key: &str, expected: &str, at: &str) -> Check<()> {
    let actual = string(value, key, at)?;
    if actual != expected {
        return Err(format!("{at}.{key} must be '{expected}', got '{actual}'"));
    }
    Ok(())
}

fn string_array(value: &Value, key: &str, at: &str) -> Check<Vec<String>> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{at}.{key} must be an array"))?
        .iter()
        .enumerate()
        .map(|(index, item)| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{at}.{key}[{index}] must be a string"))
        })
        .collect()
}

fn unique_strings(values: &[String], at: &str) -> Check<()> {
    let mut seen = BTreeSet::new();
    if let Some(value) = values.iter().find(|value| !seen.insert(value.as_str())) {
        return Err(format!("{at} contains duplicate value '{value}'"));
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut result = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut result, "{byte:02x}");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn units() -> Map<String, Value> {
        json!({
            "length": "m",
            "force": "N",
            "area": "m^2",
            "stress": "Pa",
            "second_moment": "m^4",
            "distributed_load": "N/m"
        })
        .as_object()
        .expect("unit fixture")
        .clone()
    }

    fn physics(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn axial_bar_resolves_to_only_qualified_in_process_backend() {
        let result = resolve_request(
            "axial_bar",
            "uniform_linear_elastic_axial_bar",
            &physics(&["static_linear_elastic"]),
            &units(),
        )
        .expect("manifest resolves");
        assert_eq!(result.status, SelectionStatus::Selected);
        let backend = result.selected_backend.expect("selected backend");
        assert_eq!(backend["backend_id"], "symtropy-analytical-reference");
        assert_eq!(backend["external_solver_invoked"], false);
        assert_eq!(
            result.outputs,
            vec!["axial_stress_pa", "axial_strain", "extension_m"]
        );
        assert_eq!(result.manifest_sha256.len(), 64);
    }

    #[test]
    fn fracture_request_is_unresolved_without_linear_fallback() {
        let result = resolve_request(
            "axial_bar",
            "uniform_linear_elastic_axial_bar",
            &physics(&["static_linear_elastic", "fracture"]),
            &units(),
        )
        .expect("unsupported request resolves to an abstention");
        assert_eq!(result.status, SelectionStatus::Unresolved);
        assert!(result.selected_backend.is_none());
        assert_eq!(result.physics_omitted, vec!["fracture"]);
    }

    #[test]
    fn unknown_case_and_unit_mismatch_are_unresolved() {
        let unknown = resolve_request(
            "dynamic_collapse",
            "generic_collapse",
            &physics(&["dynamic_response"]),
            &units(),
        )
        .expect("unknown cases abstain");
        assert_eq!(unknown.status, SelectionStatus::Unresolved);
        assert!(unknown.selected_backend.is_none());

        let mut bad_units = units();
        bad_units.insert("length".to_owned(), json!("ft"));
        let mismatch = resolve_request(
            "axial_bar",
            "uniform_linear_elastic_axial_bar",
            &physics(&["static_linear_elastic"]),
            &bad_units,
        )
        .expect("unit mismatch abstains");
        assert_eq!(mismatch.status, SelectionStatus::Unresolved);
        assert!(mismatch.selected_backend.is_none());
    }

    #[test]
    fn ambiguous_registry_is_unresolved() {
        let mut manifest: Value = serde_json::from_str(MANIFEST).expect("pinned manifest");
        let mut duplicate_backend = manifest["backends"][0].clone();
        duplicate_backend["backend_id"] = json!("second-reference-backend");
        manifest["backends"]
            .as_array_mut()
            .unwrap()
            .push(duplicate_backend);
        let result = resolve_against_manifest(
            &manifest,
            "axial_bar",
            "uniform_linear_elastic_axial_bar",
            &physics(&["static_linear_elastic"]),
            &units(),
        )
        .expect("ambiguity is a valid abstention");
        assert_eq!(result.status, SelectionStatus::Unresolved);
        assert!(result.selected_backend.is_none());
        assert!(result.diagnostics[0].contains("ambiguous dispatch"));
    }

    #[test]
    fn external_backend_is_not_executable_in_dispatch_v0_1() {
        let mut manifest: Value = serde_json::from_str(MANIFEST).expect("pinned manifest");
        manifest["backends"][0]["implementation_kind"] = json!("external_process");
        let result = resolve_against_manifest(
            &manifest,
            "axial_bar",
            "uniform_linear_elastic_axial_bar",
            &physics(&["static_linear_elastic"]),
            &units(),
        )
        .expect("unimplemented external execution must abstain");
        assert_eq!(result.status, SelectionStatus::Unresolved);
        assert!(result.selected_backend.is_none());
        assert!(result.diagnostics[0].contains("not qualified for automatic dispatch"));
    }

    #[test]
    fn manifest_unit_claims_must_match_the_si_contract() {
        let mut manifest: Value = serde_json::from_str(MANIFEST).expect("pinned manifest");
        manifest["backends"][0]["supported_units"]["length"] = json!("ft");
        assert!(
            validate_manifest(&manifest)
                .unwrap_err()
                .contains("supported_units.length must be 'm'")
        );
    }

    #[test]
    fn in_process_manifest_entry_cannot_claim_external_solver_invocation() {
        let mut manifest: Value = serde_json::from_str(MANIFEST).expect("pinned manifest");
        manifest["backends"][0]["external_solver_invoked"] = json!(true);
        assert!(
            validate_manifest(&manifest)
                .unwrap_err()
                .contains("must be false for an in-process reference backend")
        );
    }

    #[test]
    fn duplicate_backend_ids_make_the_manifest_invalid() {
        let mut manifest: Value = serde_json::from_str(MANIFEST).expect("pinned manifest");
        let duplicate = manifest["backends"][0].clone();
        manifest["backends"].as_array_mut().unwrap().push(duplicate);
        assert!(
            validate_manifest(&manifest)
                .unwrap_err()
                .contains("backend_id is duplicated")
        );
    }
}
