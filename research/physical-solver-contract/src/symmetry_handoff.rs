// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact-byte binding for the Symthaea → Symtropy symmetry evidence handoff.
//!
//! This module verifies only byte identity and manifest/envelope consistency.
//! It does not re-run mathematical qualification, physical equations,
//! step-map equivariance, or scientific-search equivalence.

use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use super::{sha256_hex, Check};

const MANIFEST_SCHEMA: &str = "luminous.symmetry_physical_artifact_manifest.v0.1";

fn object<'a>(value: &'a Value, at: &str) -> Check<&'a serde_json::Map<String, Value>> {
    value
        .as_object()
        .ok_or_else(|| format!("{at} must be an object"))
}

fn field<'a>(value: &'a Value, key: &str, at: &str) -> Check<&'a Value> {
    object(value, at)?
        .get(key)
        .ok_or_else(|| format!("{at}.{key} is required"))
}

fn string(value: &Value, key: &str, at: &str) -> Check<String> {
    field(value, key, at)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("{at}.{key} must be a string"))
}

fn require_exact_keys(value: &Value, allowed: &[&str], at: &str) -> Check<()> {
    let value = object(value, at)?;
    if let Some(key) = value.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(format!("{at} contains unknown field '{key}'"));
    }
    if let Some(key) = allowed.iter().find(|key| !value.contains_key(**key)) {
        return Err(format!("{at}.{key} is required"));
    }
    Ok(())
}

fn digest(value: &Value, key: &str, at: &str) -> Check<String> {
    let value = string(value, key, at)?;
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("{at}.{key} must be a lowercase SHA-256 digest"));
    }
    Ok(value)
}

fn expected_artifacts(handoff: &Value) -> Check<BTreeMap<String, String>> {
    let source = field(handoff, "source_math_receipt", "handoff")?;
    let physical = field(handoff, "physical_problem", "handoff")?;
    let change = field(physical, "coordinate_change", "handoff.physical_problem")?;
    let task = field(handoff, "full_task_semantics", "handoff")?;
    let receiver = field(handoff, "receiver_verification", "handoff")?;

    let mut expected = BTreeMap::new();
    let mut insert = |role: &str, value: String| -> Check<()> {
        if expected.insert(role.to_owned(), value).is_some() {
            return Err(format!("duplicate expected artifact role: {role}"));
        }
        Ok(())
    };

    insert("source_math_receipt", digest(source, "receipt_sha256", "source_math_receipt")?)?;
    insert("generator_set", digest(source, "generator_set_sha256", "source_math_receipt")?)?;
    insert("physical_fixture", digest(physical, "fixture_sha256", "physical_problem")?)?;
    insert("coordinate_transform", digest(change, "transform_sha256", "coordinate_change")?)?;
    insert(
        "transformed_mass_metric",
        digest(change, "transformed_mass_metric_sha256", "coordinate_change")?,
    )?;
    insert("mass_metric", digest(physical, "mass_metric_sha256", "physical_problem")?)?;
    insert(
        "force_covectors",
        digest(physical, "force_covectors_sha256", "physical_problem")?,
    )?;
    insert(
        "constraint_map",
        digest(physical, "constraint_map_sha256", "physical_problem")?,
    )?;
    insert(
        "initial_state",
        digest(physical, "initial_state_sha256", "physical_problem")?,
    )?;
    insert("full_task", digest(task, "task_sha256", "full_task_semantics")?)?;
    insert(
        "candidate_domain",
        digest(task, "candidate_domain_sha256", "full_task_semantics")?,
    )?;
    insert("objective", digest(task, "objective_sha256", "full_task_semantics")?)?;
    insert(
        "feasibility_constraints",
        digest(task, "feasibility_constraints_sha256", "full_task_semantics")?,
    )?;
    insert(
        "initial_and_boundary_conditions",
        digest(task, "initial_and_boundary_conditions_sha256", "full_task_semantics")?,
    )?;
    insert(
        "observable_mapping",
        digest(task, "observable_mapping_sha256", "full_task_semantics")?,
    )?;
    insert(
        "answer_interpretation",
        digest(task, "answer_interpretation_sha256", "full_task_semantics")?,
    )?;
    insert(
        "receiver_receipt",
        digest(receiver, "receiver_receipt_sha256", "receiver_verification")?,
    )?;
    Ok(expected)
}

/// Verify the handoff envelope and every referenced artifact against exact local bytes.
///
/// The manifest schema is luminous.symmetry_physical_artifact_manifest.v0.1.
/// It contains a digest of the exact handoff JSON bytes and exactly one local
/// artifact record for each digest-bearing role in that handoff.
///
/// A PASS means only that the named local bytes match. Upstream and receiver
/// physical statuses are copied verbatim into the report; no authority is
/// promoted by this byte-identity check.
pub fn verify_symmetry_handoff_artifact_bundle(
    handoff_bytes: &[u8],
    manifest_bytes: &[u8],
    artifact_root: &Path,
) -> Check<Value> {
    let handoff_sha256 = sha256_hex(handoff_bytes);
    let manifest_sha256 = sha256_hex(manifest_bytes);
    let handoff: Value = serde_json::from_slice(handoff_bytes)
        .map_err(|error| format!("invalid symmetry handoff JSON: {error}"))?;
    let manifest: Value = serde_json::from_slice(manifest_bytes)
        .map_err(|error| format!("invalid artifact manifest JSON: {error}"))?;

    if string(&handoff, "schema_version", "handoff")?
        != "luminous.symmetry_physical_handoff.v0.1"
    {
        return Err("unsupported symmetry handoff schema_version".to_owned());
    }
    if string(&manifest, "schema_version", "manifest")? != MANIFEST_SCHEMA {
        return Err("unsupported symmetry artifact manifest schema_version".to_owned());
    }

    let fixture_mode = string(&handoff, "fixture_mode", "handoff")?;
    if !["synthetic_example", "admissible_receipt"].contains(&fixture_mode.as_str()) {
        return Err("handoff.fixture_mode has an unsupported value".to_owned());
    }

    let source = field(&handoff, "source_math_receipt", "handoff")?;
    let receiver = field(&handoff, "receiver_verification", "handoff")?;
    let claims = field(&handoff, "claims", "handoff")?;
    let source_status = string(source, "qualification_status", "source_math_receipt")?;
    let receiver_status = string(receiver, "status", "receiver_verification")?;
    let physical_status = string(claims, "physical_step_equivariance_status", "claims")?;
    let physical_authority = string(claims, "physical_equivalence_authority", "claims")?;
    let search_pruning = field(claims, "scientific_search_pruning_authorized", "claims")?
        .as_bool()
        .ok_or_else(|| "claims.scientific_search_pruning_authorized must be boolean".to_owned())?;

    require_exact_keys(
        &manifest,
        &["schema_version", "handoff_sha256", "artifacts"],
        "manifest",
    )?;
    let manifest_handoff_sha256 = digest(&manifest, "handoff_sha256", "manifest")?;
    let handoff_digest_match = manifest_handoff_sha256 == handoff_sha256;
    let expected = expected_artifacts(&handoff)?;
    let entries = field(&manifest, "artifacts", "manifest")?
        .as_array()
        .ok_or_else(|| "manifest.artifacts must be an array".to_owned())?;
    if entries.len() != expected.len() {
        return Err(format!(
            "manifest must contain exactly {} artifact roles; found {}",
            expected.len(),
            entries.len()
        ));
    }

    let mut seen_ids = BTreeSet::new();
    let mut seen_roles = BTreeSet::new();
    let mut seen_paths = BTreeSet::new();
    let (canonical_root, root_error) = match fs::canonicalize(artifact_root) {
        Ok(root) if root.is_dir() => (Some(root), None),
        Ok(_) => (None, Some("artifact root is not a directory".to_owned())),
        Err(error) => (None, Some(format!("cannot canonicalize artifact root: {error}"))),
    };

    let mut results = Vec::with_capacity(entries.len());
    let mut has_failure = !handoff_digest_match || search_pruning;
    let mut has_unresolved = false;

    for (index, entry) in entries.iter().enumerate() {
        let at = format!("manifest.artifacts[{index}]");
        require_exact_keys(
            entry,
            &["artifact_id", "role", "relative_path", "sha256", "byte_length"],
            &at,
        )?;
        let artifact_id = string(entry, "artifact_id", &at)?;
        let role = string(entry, "role", &at)?;
        let relative_path = string(entry, "relative_path", &at)?;
        let declared_sha256 = digest(entry, "sha256", &at)?;
        let declared_length = field(entry, "byte_length", &at)?
            .as_u64()
            .ok_or_else(|| format!("{at}.byte_length must be an unsigned integer"))?;

        if artifact_id.trim().is_empty() {
            return Err(format!("{at}.artifact_id must not be empty"));
        }
        if !seen_ids.insert(artifact_id.clone()) {
            return Err(format!("duplicate artifact_id: {artifact_id}"));
        }
        if !seen_roles.insert(role.clone()) {
            return Err(format!("duplicate artifact role: {role}"));
        }
        if !seen_paths.insert(relative_path.clone()) {
            return Err(format!("duplicate artifact path: {relative_path}"));
        }

        let expected_sha256 = expected
            .get(&role)
            .ok_or_else(|| format!("unknown or unreferenced artifact role: {role}"))?;
        let mut status = "UNRESOLVED";
        let mut observed_sha256 = Value::Null;
        let mut observed_length = Value::Null;
        let diagnostic = if declared_sha256 != *expected_sha256 {
            status = "FAIL";
            has_failure = true;
            Some("manifest digest does not match the digest bound by the handoff envelope".to_owned())
        } else if let Some(reason) = root_error.as_ref() {
            has_unresolved = true;
            Some(reason.clone())
        } else {
            match super::safe_artifact_relative_path(&relative_path) {
                Err(reason) => {
                    has_unresolved = true;
                    Some(reason)
                }
                Ok(relative) => {
                    let root = canonical_root
                        .as_ref()
                        .expect("canonical root exists when no root error exists");
                    let candidate = root.join(relative);
                    match fs::canonicalize(&candidate) {
                        Err(error) => {
                            has_unresolved = true;
                            Some(format!("artifact cannot be resolved: {error}"))
                        }
                        Ok(canonical_path) if !canonical_path.starts_with(root) => {
                            has_unresolved = true;
                            Some("artifact resolves outside the declared root".to_owned())
                        }
                        Ok(canonical_path) => match fs::metadata(&canonical_path) {
                            Err(error) => {
                                has_unresolved = true;
                                Some(format!("artifact metadata cannot be read: {error}"))
                            }
                            Ok(metadata) if !metadata.is_file() => {
                                has_unresolved = true;
                                Some("artifact is not a regular file".to_owned())
                            }
                            Ok(_) => match super::sha256_file(&canonical_path) {
                                Err(error) => {
                                    has_unresolved = true;
                                    Some(format!("artifact bytes cannot be read: {error}"))
                                }
                                Ok((observed, length)) => {
                                    observed_sha256 = Value::String(observed.clone());
                                    observed_length = json!(length);
                                    if observed != declared_sha256 || length != declared_length {
                                        status = "FAIL";
                                        has_failure = true;
                                        Some("artifact digest or byte length mismatch".to_owned())
                                    } else {
                                        status = "PASS";
                                        None
                                    }
                                }
                            },
                        },
                    }
                }
            }
        };

        results.push(json!({
            "artifact_id": artifact_id,
            "role": role,
            "relative_path": relative_path,
            "status": status,
            "expected_sha256": declared_sha256,
            "observed_sha256": observed_sha256,
            "expected_byte_length": declared_length,
            "observed_byte_length": observed_length,
            "diagnostic": diagnostic
        }));
    }

    if seen_roles.len() != expected.len()
        || expected.keys().any(|role| !seen_roles.contains(role))
    {
        return Err("manifest artifact roles do not exactly cover the handoff digest inventory".to_owned());
    }

    let verification_status = if has_failure {
        "FAIL"
    } else if has_unresolved {
        "UNRESOLVED"
    } else {
        "PASS"
    };

    Ok(json!({
        "schema_version": "luminous.symmetry_physical_handoff_byte_verification.v0.1",
        "fixture_mode": fixture_mode,
        "handoff_sha256": handoff_sha256,
        "manifest_sha256": manifest_sha256,
        "handoff_digest_match": handoff_digest_match,
        "verification_status": verification_status,
        "artifact_binding_only": true,
        "physical_claim_authority_created": false,
        "physical_status_promoted": false,
        "source_math_qualification_status": source_status,
        "receiver_physical_status": receiver_status,
        "physical_step_equivariance_status": physical_status,
        "physical_equivalence_authority": physical_authority,
        "scientific_search_pruning_authorized": search_pruning,
        "verified_artifact_count": results.iter().filter(|result| result["status"] == "PASS").count(),
        "artifact_count": results.len(),
        "artifact_results": results
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let stamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "symmetry-handoff-{}-{stamp}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    const ROLES: &[&str] = &[
        "source_math_receipt",
        "generator_set",
        "physical_fixture",
        "coordinate_transform",
        "transformed_mass_metric",
        "mass_metric",
        "force_covectors",
        "constraint_map",
        "initial_state",
        "full_task",
        "candidate_domain",
        "objective",
        "feasibility_constraints",
        "initial_and_boundary_conditions",
        "observable_mapping",
        "answer_interpretation",
        "receiver_receipt",
    ];

    fn artifact_bytes(role: &str) -> Vec<u8> {
        format!("synthetic-artifact-bytes-for-{role}\n").into_bytes()
    }

    fn digest_map() -> BTreeMap<String, String> {
        ROLES
            .iter()
            .map(|role| (role.to_string(), sha256_hex(&artifact_bytes(role))))
            .collect()
    }

    fn sample_handoff(digests: &BTreeMap<String, String>) -> Value {
        let d = |role: &str| digests.get(role).unwrap().clone();
        json!({
            "schema_version": "luminous.symmetry_physical_handoff.v0.1",
            "fixture_mode": "synthetic_example",
            "handoff_id": "test-handoff",
            "source_math_receipt": {
                "repository": "Luminous-Dynamics/symthaea",
                "commit_sha": "0123456789abcdef0123456789abcdef01234567",
                "receipt_id": "test-math-receipt",
                "receipt_sha256": d("source_math_receipt"),
                "checker_id": "test-checker",
                "checker_version": "0.1",
                "profile_id": "test-profile",
                "action_id": "test-action",
                "coordinate_basis_id": "test-basis",
                "metric_id": "test-metric",
                "generator_set_sha256": d("generator_set"),
                "evidence_class": "numeric_lie_basis",
                "qualification_status": "PASS"
            },
            "physical_problem": {
                "case_id": "test-case",
                "fixture_sha256": d("physical_fixture"),
                "coordinate_change": {
                    "kind": "generalized_coordinate_rebasing",
                    "from_basis_id": "q",
                    "to_basis_id": "y",
                    "transform_sha256": d("coordinate_transform"),
                    "transformed_mass_metric_sha256": d("transformed_mass_metric")
                },
                "mass_metric_sha256": d("mass_metric"),
                "force_covectors_sha256": d("force_covectors"),
                "constraint_map_sha256": d("constraint_map"),
                "initial_state_sha256": d("initial_state"),
                "solver_profile_id": "test-solver-profile",
                "integration_method": "midpoint",
                "time_step_seconds": 0.01,
                "units_profile_id": "SI-v1"
            },
            "full_task_semantics": {
                "task_sha256": d("full_task"),
                "candidate_domain_sha256": d("candidate_domain"),
                "objective_sha256": d("objective"),
                "feasibility_constraints_sha256": d("feasibility_constraints"),
                "initial_and_boundary_conditions_sha256": d("initial_and_boundary_conditions"),
                "observable_mapping_sha256": d("observable_mapping"),
                "answer_interpretation_sha256": d("answer_interpretation")
            },
            "receiver_verification": {
                "repository": "Luminous-Dynamics/symtropy",
                "commit_sha": "89abcdef0123456789abcdef0123456789abcdef",
                "checker_id": "test-receiver",
                "checker_version": "0.1",
                "profile_id": "test-receiver-profile",
                "status": "INCONCLUSIVE",
                "receiver_receipt_sha256": d("receiver_receipt"),
                "obligations": {
                    "mass_metric_congruence": "INCONCLUSIVE",
                    "force_covector_duality": "INCONCLUSIVE",
                    "momentum_one_form": "INCONCLUSIVE",
                    "constraint_jacobian": "INCONCLUSIVE",
                    "generator_metric_isometry": "INCONCLUSIVE",
                    "discrete_step_equivariance": "INCONCLUSIVE",
                    "physical_observables": "INCONCLUSIVE",
                    "numerical_error_envelope": "INCONCLUSIVE",
                    "full_task_semantics": "INCONCLUSIVE"
                }
            },
            "claims": {
                "physical_step_equivariance_status": "INCONCLUSIVE",
                "physical_equivalence_authority": "NONE",
                "scientific_search_pruning_authorized": false
            },
            "claim_boundary": {
                "scope": "synthetic byte-verification fixture",
                "limitations": ["No physical correctness is established."]
            }
        })
    }

    fn build_bundle(root: &TestRoot) -> (Vec<u8>, Vec<u8>) {
        let digests = digest_map();
        let handoff_bytes = serde_json::to_vec(&sample_handoff(&digests)).unwrap();
        let mut artifacts = Vec::new();
        for (index, role) in ROLES.iter().enumerate() {
            let relative_path = format!("artifacts/{role}.bin");
            let full_path = root.0.join(&relative_path);
            fs::create_dir_all(full_path.parent().unwrap()).unwrap();
            let bytes = artifact_bytes(role);
            fs::write(&full_path, &bytes).unwrap();
            artifacts.push(json!({
                "artifact_id": format!("artifact-{index:02}"),
                "role": role,
                "relative_path": relative_path,
                "sha256": digests.get(*role).unwrap(),
                "byte_length": bytes.len()
            }));
        }
        let manifest = json!({
            "schema_version": MANIFEST_SCHEMA,
            "handoff_sha256": sha256_hex(&handoff_bytes),
            "artifacts": artifacts
        });
        (handoff_bytes, serde_json::to_vec(&manifest).unwrap())
    }

    #[test]
    fn exact_bytes_pass_without_promoting_physical_or_search_authority() {
        let root = TestRoot::new();
        let (handoff, manifest) = build_bundle(&root);
        let report =
            verify_symmetry_handoff_artifact_bundle(&handoff, &manifest, &root.0).unwrap();
        assert_eq!(report["verification_status"], "PASS");
        assert_eq!(report["artifact_binding_only"], true);
        assert_eq!(report["physical_claim_authority_created"], false);
        assert_eq!(report["physical_status_promoted"], false);
        assert_eq!(report["receiver_physical_status"], "INCONCLUSIVE");
        assert_eq!(report["physical_step_equivariance_status"], "INCONCLUSIVE");
        assert_eq!(report["scientific_search_pruning_authorized"], false);
        assert_eq!(report["verified_artifact_count"], ROLES.len());
    }

    #[test]
    fn changed_artifact_bytes_fail_but_do_not_promote_the_claim() {
        let root = TestRoot::new();
        let (handoff, manifest) = build_bundle(&root);
        fs::write(root.0.join("artifacts/mass_metric.bin"), b"tampered").unwrap();
        let report =
            verify_symmetry_handoff_artifact_bundle(&handoff, &manifest, &root.0).unwrap();
        assert_eq!(report["verification_status"], "FAIL");
        assert_eq!(report["receiver_physical_status"], "INCONCLUSIVE");
        assert_eq!(report["physical_status_promoted"], false);
    }

    #[test]
    fn path_traversal_is_unresolved() {
        let root = TestRoot::new();
        let (handoff, manifest) = build_bundle(&root);
        let mut value: Value = serde_json::from_slice(&manifest).unwrap();
        value["artifacts"][0]["relative_path"] = json!("../outside.bin");
        let report = verify_symmetry_handoff_artifact_bundle(
            &handoff,
            &serde_json::to_vec(&value).unwrap(),
            &root.0,
        )
        .unwrap();
        assert_eq!(report["verification_status"], "UNRESOLVED");
        assert!(report["artifact_results"].as_array().unwrap().iter().any(
            |result| result["status"] == "UNRESOLVED"
        ));
    }

    #[test]
    fn manifest_digest_drift_fails_even_when_file_bytes_match_manifest() {
        let root = TestRoot::new();
        let (handoff, manifest) = build_bundle(&root);
        let mut value: Value = serde_json::from_slice(&manifest).unwrap();
        let entry = value["artifacts"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|entry| entry["role"] == "mass_metric")
            .unwrap();
        entry["sha256"] = json!("0".repeat(64));
        let report = verify_symmetry_handoff_artifact_bundle(
            &handoff,
            &serde_json::to_vec(&value).unwrap(),
            &root.0,
        )
        .unwrap();
        assert_eq!(report["verification_status"], "FAIL");
        assert!(report["artifact_results"].as_array().unwrap().iter().any(
            |entry| entry["role"] == "mass_metric" && entry["status"] == "FAIL"
        ));
    }

    #[cfg(unix)]
    #[test]
    fn symlink_escape_is_unresolved() {
        use std::os::unix::fs::symlink;

        let root = TestRoot::new();
        let outside = TestRoot::new();
        let (handoff, manifest) = build_bundle(&root);
        let outside_file = outside.0.join("external.bin");
        fs::write(&outside_file, b"external").unwrap();
        let escaped = root.0.join("artifacts/source_math_receipt.bin");
        fs::remove_file(&escaped).unwrap();
        symlink(&outside_file, &escaped).unwrap();

        let report =
            verify_symmetry_handoff_artifact_bundle(&handoff, &manifest, &root.0).unwrap();
        assert_eq!(report["verification_status"], "UNRESOLVED");
        assert!(report["artifact_results"].as_array().unwrap().iter().any(
            |entry| entry["role"] == "source_math_receipt" && entry["status"] == "UNRESOLVED"
        ));
    }

    #[test]
    fn hard_pruning_claim_is_never_accepted_by_byte_verifier() {
        let root = TestRoot::new();
        let (handoff, manifest) = build_bundle(&root);
        let mut changed: Value = serde_json::from_slice(&handoff).unwrap();
        changed["claims"]["scientific_search_pruning_authorized"] = json!(true);
        let report = verify_symmetry_handoff_artifact_bundle(
            &serde_json::to_vec(&changed).unwrap(),
            &manifest,
            &root.0,
        )
        .unwrap();
        assert_eq!(report["verification_status"], "FAIL");
        assert_eq!(report["scientific_search_pruning_authorized"], true);
        assert_eq!(report["physical_claim_authority_created"], false);
    }

    #[test]
    fn stale_handoff_bytes_fail_manifest_binding() {
        let root = TestRoot::new();
        let (handoff, manifest) = build_bundle(&root);
        let mut changed: Value = serde_json::from_slice(&handoff).unwrap();
        changed["handoff_id"] = json!("changed-after-manifest");
        let report = verify_symmetry_handoff_artifact_bundle(
            &serde_json::to_vec(&changed).unwrap(),
            &manifest,
            &root.0,
        )
        .unwrap();
        assert_eq!(report["verification_status"], "FAIL");
        assert_eq!(report["handoff_digest_match"], false);
        assert_eq!(report["physical_status_promoted"], false);
    }

    #[test]
    fn manifest_must_cover_exactly_the_declared_digest_roles() {
        let root = TestRoot::new();
        let (handoff, manifest) = build_bundle(&root);
        let mut value: Value = serde_json::from_slice(&manifest).unwrap();
        value["artifacts"].as_array_mut().unwrap().pop();
        assert!(verify_symmetry_handoff_artifact_bundle(
            &handoff,
            &serde_json::to_vec(&value).unwrap(),
            &root.0
        )
        .is_err());
    }
}
