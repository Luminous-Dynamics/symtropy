// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Validates a pinned evaluation scope profile and its declared completeness contract.
//! This module checks presence, identity and local artifact hashes; it does not establish
//! that the profile is adequate or that its evidence artifacts document successful tests.

use crate::jurisdiction_registry;
use jsonschema::draft202012;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path},
};

const EMBEDDED_SCHEMA: &str =
    include_str!("../../../../docs/built-environment/evaluation-scope-profile.schema.json");

#[derive(Debug, Default, Serialize)]
pub struct ScopeProfileValidation {
    pub artifact_verified: bool,
    pub digest_pinned: bool,
    pub profile_valid: bool,
    pub scope_complete: bool,
    pub artifact_digests_verified: usize,
    pub digest_roles: Vec<(String, Option<String>)>,
}

pub fn validate(
    receipt: &Value,
    project_manifest: Option<&Value>,
    artifact_root: &Path,
    expected_sha256: &str,
    artifact_issues: &mut Vec<String>,
    issues: &mut Vec<String>,
) -> Result<ScopeProfileValidation, String> {
    if !is_sha256_hex(expected_sha256) {
        return Err(
            "expected scope-profile SHA-256 must be 64 lowercase hexadecimal characters".to_string(),
        );
    }

    let mut result = ScopeProfileValidation::default();
    let Some(profile_binding) = receipt.get("scope_profile") else {
        issues.push("receipt is missing the evaluation scope profile".to_string());
        return Ok(result);
    };
    let Some(profile_artifact) = profile_binding.get("artifact") else {
        issues.push("receipt scope profile is missing its artifact binding".to_string());
        return Ok(result);
    };

    result.artifact_verified = jurisdiction_registry::verify_artifact(
        Some(profile_artifact),
        artifact_root,
        artifact_issues,
        "receipt evaluation scope profile",
    );
    if result.artifact_verified {
        result.artifact_digests_verified += 1;
    }
    result.digest_roles.push((
        "evaluation scope profile".to_string(),
        profile_artifact.get("sha256").and_then(Value::as_str).map(str::to_owned),
    ));

    let mut gate_ids = BTreeSet::new();
    let mut gate_statuses: BTreeMap<String, String> = BTreeMap::new();
    let mut gate_artifacts_valid = true;
    if let Some(gates) = receipt.get("evaluation_gate_evidence").and_then(Value::as_array) {
        for gate in gates {
            let gate_id = gate.get("gate_id").and_then(Value::as_str).unwrap_or_default();
            let declared_status = gate
                .get("declared_result_status")
                .and_then(Value::as_str)
                .unwrap_or("NOT_EVALUATED")
                .to_owned();
            if !gate_ids.insert(gate_id.to_owned()) {
                gate_artifacts_valid = false;
                issues.push(format!(
                    "receipt duplicates evaluation gate evidence {gate_id:?}"
                ));
            } else {
                gate_statuses.insert(gate_id.to_owned(), declared_status);
            }
            let artifact = gate.get("artifact");
            let verified = jurisdiction_registry::verify_artifact(
                artifact,
                artifact_root,
                artifact_issues,
                &format!("evaluation gate {gate_id:?} evidence"),
            );
            if verified {
                result.artifact_digests_verified += 1;
            } else {
                gate_artifacts_valid = false;
            }
            result.digest_roles.push((
                format!("evaluation gate {gate_id:?}"),
                artifact
                    .and_then(|item| item.get("sha256"))
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            ));
        }
    } else {
        gate_artifacts_valid = false;
    }

    if !result.artifact_verified {
        return Ok(result);
    }

    let relative_path = profile_artifact
        .get("relative_path")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let profile_path = match safe_artifact_path(artifact_root, relative_path) {
        Ok(path) => path,
        Err(error) => {
            issues.push(format!("evaluation scope profile path is unsafe: {error}"));
            return Ok(result);
        }
    };
    let profile_bytes = match fs::read(&profile_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            issues.push(format!("cannot read evaluation scope profile: {error}"));
            return Ok(result);
        }
    };

    let actual_sha256 = sha256_hex(&profile_bytes);
    result.digest_pinned = actual_sha256 == expected_sha256;
    if !result.digest_pinned {
        issues.push(format!(
            "evaluation scope profile does not match out-of-band pin (actual {actual_sha256})"
        ));
    }

    let profile: Value = match serde_json::from_slice(&profile_bytes) {
        Ok(profile) => profile,
        Err(error) => {
            issues.push(format!("evaluation scope profile is not valid JSON: {error}"));
            return Ok(result);
        }
    };

    let schema: Value = serde_json::from_str(EMBEDDED_SCHEMA)
        .map_err(|error| format!("embedded scope-profile schema is invalid JSON: {error}"))?;
    if !all_refs_are_local(&schema) {
        return Err("evaluation scope profile schema contains an external reference".to_string());
    }
    draft202012::meta::validate(&schema).map_err(|error| {
        format!("scope-profile schema fails Draft 2020-12 meta-validation: {error}")
    })?;
    let validator = draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
        .map_err(|error| format!("cannot compile scope-profile schema: {error}"))?;
    let mut profile_issues: Vec<String> = validator
        .iter_errors(&profile)
        .map(|error| format!("scope-profile schema validation: {error}"))
        .collect();

    let mut required_gate_ids = BTreeSet::new();
    if let Some(required_gates) = profile.get("required_gates").and_then(Value::as_array) {
        for gate in required_gates {
            let gate_id = gate.get("gate_id").and_then(Value::as_str).unwrap_or_default();
            if !required_gate_ids.insert(gate_id.to_owned()) {
                profile_issues.push(format!(
                    "scope profile duplicates required gate ID {gate_id:?}"
                ));
            }
        }
    }
    if !profile_issues.is_empty() {
        issues.extend(profile_issues);
        return Ok(result);
    }

    let profile_identity_matches =
        profile["profile_id"] == profile_binding["profile_id"]
            && profile["profile_version"] == profile_binding["profile_version"];
    if !profile_identity_matches {
        issues.push(
            "receipt scope-profile identity/version does not match pinned profile bytes"
                .to_string(),
        );
        return Ok(result);
    }
    result.profile_valid = true;

    let mut project_scope_matches = false;
    let mut required_roles_present = false;
    if let Some(manifest) = project_manifest {
        let project_scope = &manifest["project_scope"];
        let allowed = &profile["allowed_project_scope"];
        project_scope_matches =
            list_contains(&allowed["building_uses"], project_scope["building_use"].as_str())
                && list_contains(
                    &allowed["construction_systems"],
                    project_scope["construction_system"].as_str(),
                )
                && list_contains(&allowed["design_stages"], project_scope["design_stage"].as_str());
        if !project_scope_matches {
            issues.push(
                "project building use, construction system or design stage falls outside pinned scope profile"
                    .to_string(),
            );
        }

        let supplied_roles: BTreeSet<&str> = manifest["additional_inputs"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|input| input["role"].as_str())
            .collect();
        required_roles_present = true;
        for required_role in profile["required_input_roles"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !supplied_roles.contains(required_role) {
                required_roles_present = false;
                issues.push(format!(
                    "project input manifest omits profile-required input role {required_role:?}"
                ));
            }
        }
    } else {
        issues.push(
            "cannot check scope-profile requirements without a valid project input manifest"
                .to_string(),
        );
    }

    let missing_gates: Vec<String> = required_gate_ids
        .difference(&gate_ids)
        .cloned()
        .collect();
    if !missing_gates.is_empty() {
        for missing in &missing_gates {
            issues.push(format!("receipt omits profile-required evaluation gate {missing:?}"));
        }
    }

    let mut required_gate_statuses_pass = true;
    for required_gate_id in &required_gate_ids {
        match gate_statuses.get(required_gate_id).map(String::as_str) {
            Some("CLAIMED_PASS") => {}
            Some(status) => {
                required_gate_statuses_pass = false;
                issues.push(format!(
                    "required evaluation gate {required_gate_id:?} declares {status}, not CLAIMED_PASS"
                ));
            }
            None => required_gate_statuses_pass = false,
        }
    }

    result.scope_complete = result.digest_pinned
        && result.profile_valid
        && project_scope_matches
        && required_roles_present
        && missing_gates.is_empty()
        && required_gate_statuses_pass
        && gate_artifacts_valid;
    Ok(result)
}

fn safe_artifact_path(root: &Path, relative_path: &str) -> Result<std::path::PathBuf, String> {
    let relative = Path::new(relative_path);
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("artifact path must contain only normal relative components".to_string());
    }
    let base = root
        .canonicalize()
        .map_err(|error| format!("cannot resolve artifact root: {error}"))?;
    let resolved = base
        .join(relative)
        .canonicalize()
        .map_err(|error| format!("cannot resolve artifact: {error}"))?;
    if !resolved.starts_with(&base) {
        return Err("artifact path resolves outside the artifact root".to_string());
    }
    Ok(resolved)
}

fn list_contains(values: &Value, expected: Option<&str>) -> bool {
    let Some(expected) = expected else {
        return false;
    };
    values
        .as_array()
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(expected)))
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn all_refs_are_local(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.iter().all(|(key, child)| {
            (key != "$ref"
                || child
                    .as_str()
                    .is_some_and(|reference| reference.starts_with("#/")))
                && all_refs_are_local(child)
        }),
        Value::Array(values) => values.iter().all(all_refs_are_local),
        _ => true,
    }
}
