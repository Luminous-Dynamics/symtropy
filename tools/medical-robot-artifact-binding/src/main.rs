use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::{env, process};

const AUDITOR_SOURCE: &[u8] = include_bytes!("main.rs");
const CLAIM_CEILING: &str = "Artifact-byte identity and manifest-integrity checks only. No CAD correctness, simulation fidelity, manufacturability, physical safety, clinical compliance or deployment authorization.";

const ROOT_FIELDS: &[&str] = &[
    "schema_version",
    "tool_id",
    "tool_revision",
    "evidence_class",
    "physical_actuation_authorized",
    "patient_contact_authorized",
    "clinical_function_authorized",
    "artifacts",
    "claim_ceiling",
];

const ARTIFACT_FIELDS: &[&str] = &[
    "artifact_id",
    "relative_path",
    "media_type",
    "byte_length",
    "sha256",
    "role",
];

#[derive(Debug, Clone)]
struct ArtifactSpec {
    artifact_id: String,
    relative_path: String,
    media_type: String,
    byte_length: u64,
    sha256: String,
    role: String,
}

#[derive(Debug)]
struct ManifestSpec {
    tool_id: String,
    tool_revision: String,
    evidence_class: String,
    artifacts: Vec<ArtifactSpec>,
    claim_ceiling: String,
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn valid_sha256(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn field_str<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn reject_unknown_fields(
    object: &serde_json::Map<String, Value>,
    allowed: &[&str],
    context: &str,
    errors: &mut Vec<String>,
) {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            errors.push(format!("{context}: unknown field '{key}'"));
        }
    }
}

fn safe_relative_path(raw: &str) -> Result<PathBuf, &'static str> {
    if raw.trim().is_empty() || raw.contains('\\') {
        return Err("ARTIFACT_PATH_INVALID");
    }
    let path = Path::new(raw);
    if path.is_absolute() {
        return Err("ARTIFACT_PATH_ABSOLUTE");
    }
    let mut normal_components = 0usize;
    for component in path.components() {
        match component {
            Component::Normal(_) => normal_components += 1,
            _ => return Err("ARTIFACT_PATH_INVALID"),
        }
    }
    if normal_components == 0 {
        return Err("ARTIFACT_PATH_INVALID");
    }
    Ok(path.to_path_buf())
}

fn parse_manifest(root: &Value) -> Result<ManifestSpec, Vec<String>> {
    let mut errors = Vec::new();
    let Some(object) = root.as_object() else {
        return Err(vec!["MANIFEST_NOT_OBJECT".to_owned()]);
    };
    reject_unknown_fields(object, ROOT_FIELDS, "manifest", &mut errors);

    if field_str(root, "schema_version") != Some("hmr0-artifact-manifest-v0.1") {
        errors.push("SCHEMA_VERSION_UNSUPPORTED".to_owned());
    }
    for key in ["tool_id", "tool_revision", "evidence_class", "claim_ceiling"] {
        if field_str(root, key).is_none_or(|value| value.trim().is_empty()) {
            errors.push(format!("REQUIRED_STRING_INVALID:{key}"));
        }
    }
    for key in [
        "physical_actuation_authorized",
        "patient_contact_authorized",
        "clinical_function_authorized",
    ] {
        if root.get(key).and_then(Value::as_bool) != Some(false) {
            errors.push(format!("AUTHORIZATION_BOUNDARY_NOT_FALSE:{key}"));
        }
    }
    if field_str(root, "evidence_class") != Some("concept_preview_only") {
        errors.push("EVIDENCE_CLASS_UNSUPPORTED".to_owned());
    }

    let artifacts: &[Value] = match root.get("artifacts").and_then(Value::as_array) {
        Some(artifacts) if !artifacts.is_empty() => artifacts.as_slice(),
        _ => {
            errors.push("ARTIFACT_LIST_MISSING_OR_EMPTY".to_owned());
            &[]
        }
    };
    let mut artifact_ids = BTreeSet::new();
    let mut relative_paths = BTreeSet::new();
    let mut parsed_artifacts = Vec::new();
    let mut has_geometry_preview = false;

    for (index, artifact) in artifacts.iter().enumerate() {
        let context = format!("artifacts[{index}]");
        let Some(artifact_object) = artifact.as_object() else {
            errors.push(format!("{context}: ARTIFACT_NOT_OBJECT"));
            continue;
        };
        reject_unknown_fields(artifact_object, ARTIFACT_FIELDS, &context, &mut errors);

        let artifact_id = field_str(artifact, "artifact_id");
        let relative_path = field_str(artifact, "relative_path");
        let media_type = field_str(artifact, "media_type");
        let sha256 = field_str(artifact, "sha256");
        let role = field_str(artifact, "role");
        let byte_length = artifact.get("byte_length").and_then(Value::as_u64);

        for (key, value) in [
            ("artifact_id", artifact_id),
            ("relative_path", relative_path),
            ("media_type", media_type),
            ("sha256", sha256),
            ("role", role),
        ] {
            if value.is_none_or(|text| text.trim().is_empty()) {
                errors.push(format!("{context}: REQUIRED_STRING_INVALID:{key}"));
            }
        }
        if byte_length.is_none() {
            errors.push(format!("{context}: BYTE_LENGTH_INVALID"));
        }
        if sha256.is_some_and(|value| !valid_sha256(value)) {
            errors.push(format!("{context}: SHA256_INVALID"));
        }
        if let Some(role) = role {
            if !["geometry_preview", "simulation_model", "verification_report"].contains(&role) {
                errors.push(format!("{context}: ARTIFACT_ROLE_UNSUPPORTED"));
            }
            if role == "geometry_preview" {
                has_geometry_preview = true;
            }
        }
        if let Some(path) = relative_path {
            if let Err(reason) = safe_relative_path(path) {
                errors.push(format!("{context}: {reason}"));
            }
            if !relative_paths.insert(path.to_owned()) {
                errors.push(format!("{context}: DUPLICATE_RELATIVE_PATH"));
            }
        }
        if let Some(id) = artifact_id {
            if !artifact_ids.insert(id.to_owned()) {
                errors.push(format!("{context}: DUPLICATE_ARTIFACT_ID"));
            }
        }

        if let (Some(artifact_id), Some(relative_path), Some(media_type), Some(byte_length), Some(sha256), Some(role)) =
            (artifact_id, relative_path, media_type, byte_length, sha256, role)
        {
            if valid_sha256(sha256) && safe_relative_path(relative_path).is_ok() {
                parsed_artifacts.push(ArtifactSpec {
                    artifact_id: artifact_id.to_owned(),
                    relative_path: relative_path.to_owned(),
                    media_type: media_type.to_owned(),
                    byte_length,
                    sha256: sha256.to_owned(),
                    role: role.to_owned(),
                });
            }
        }
    }
    if !has_geometry_preview {
        errors.push("GEOMETRY_PREVIEW_ARTIFACT_REQUIRED".to_owned());
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(ManifestSpec {
        tool_id: field_str(root, "tool_id").unwrap_or_default().to_owned(),
        tool_revision: field_str(root, "tool_revision").unwrap_or_default().to_owned(),
        evidence_class: field_str(root, "evidence_class").unwrap_or_default().to_owned(),
        artifacts: parsed_artifacts,
        claim_ceiling: field_str(root, "claim_ceiling").unwrap_or_default().to_owned(),
    })
}

fn verify_one_artifact(root: &Path, artifact: &ArtifactSpec) -> Value {
    let relative = match safe_relative_path(&artifact.relative_path) {
        Ok(path) => path,
        Err(reason) => {
            return json!({
                "artifact_id": artifact.artifact_id,
                "relative_path": artifact.relative_path,
                "verified": false,
                "finding": reason
            });
        }
    };
    let mut cursor = root.to_path_buf();
    let components: Vec<_> = relative.components().collect();

    for (index, component) in components.iter().enumerate() {
        let Component::Normal(name) = component else {
            return json!({
                "artifact_id": artifact.artifact_id,
                "relative_path": artifact.relative_path,
                "verified": false,
                "finding": "ARTIFACT_PATH_INVALID"
            });
        };
        cursor.push(name);
        let metadata = match fs::symlink_metadata(&cursor) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return json!({
                    "artifact_id": artifact.artifact_id,
                    "relative_path": artifact.relative_path,
                    "verified": false,
                    "finding": "ARTIFACT_MISSING"
                });
            }
            Err(_) => {
                return json!({
                    "artifact_id": artifact.artifact_id,
                    "relative_path": artifact.relative_path,
                    "verified": false,
                    "finding": "ARTIFACT_METADATA_READ_FAILED"
                });
            }
        };
        if metadata.file_type().is_symlink() {
            return json!({
                "artifact_id": artifact.artifact_id,
                "relative_path": artifact.relative_path,
                "verified": false,
                "finding": "SYMLINK_NOT_ALLOWED"
            });
        }
        if index + 1 < components.len() && !metadata.is_dir() {
            return json!({
                "artifact_id": artifact.artifact_id,
                "relative_path": artifact.relative_path,
                "verified": false,
                "finding": "ARTIFACT_PARENT_NOT_DIRECTORY"
            });
        }
        if index + 1 == components.len() && !metadata.is_file() {
            return json!({
                "artifact_id": artifact.artifact_id,
                "relative_path": artifact.relative_path,
                "verified": false,
                "finding": "ARTIFACT_NOT_REGULAR_FILE"
            });
        }
    }

    let canonical_file = match fs::canonicalize(&cursor) {
        Ok(path) => path,
        Err(_) => {
            return json!({
                "artifact_id": artifact.artifact_id,
                "relative_path": artifact.relative_path,
                "verified": false,
                "finding": "ARTIFACT_CANONICALIZATION_FAILED"
            });
        }
    };
    if !canonical_file.starts_with(root) {
        return json!({
            "artifact_id": artifact.artifact_id,
            "relative_path": artifact.relative_path,
            "verified": false,
            "finding": "ARTIFACT_OUTSIDE_ROOT"
        });
    }

    let bytes = match fs::read(&canonical_file) {
        Ok(bytes) => bytes,
        Err(_) => {
            return json!({
                "artifact_id": artifact.artifact_id,
                "relative_path": artifact.relative_path,
                "verified": false,
                "finding": "ARTIFACT_READ_FAILED"
            });
        }
    };
    let observed_sha256 = sha256_hex(&bytes);
    let observed_byte_length = bytes.len() as u64;
    let digest_matches = observed_sha256 == artifact.sha256;
    let length_matches = observed_byte_length == artifact.byte_length;
    let verified = digest_matches && length_matches;

    json!({
        "artifact_id": artifact.artifact_id,
        "relative_path": artifact.relative_path,
        "media_type": artifact.media_type,
        "role": artifact.role,
        "declared_sha256": artifact.sha256,
        "observed_sha256": observed_sha256,
        "declared_byte_length": artifact.byte_length,
        "observed_byte_length": observed_byte_length,
        "digest_matches": digest_matches,
        "byte_length_matches": length_matches,
        "verified": verified,
        "finding": if verified { "ARTIFACT_BYTES_MATCH_MANIFEST" } else { "ARTIFACT_DIGEST_OR_LENGTH_MISMATCH" }
    })
}

fn verify_manifest(
    root_directory: &Path,
    manifest: &Value,
    manifest_bytes: &[u8],
    tested_revision: &str,
) -> Result<(Value, bool), Vec<String>> {
    let spec = parse_manifest(manifest)?;
    let root = fs::canonicalize(root_directory)
        .map_err(|_| vec!["ARTIFACT_ROOT_UNAVAILABLE".to_owned()])?;
    if !root.is_dir() {
        return Err(vec!["ARTIFACT_ROOT_NOT_DIRECTORY".to_owned()]);
    }

    let results: Vec<Value> = spec
        .artifacts
        .iter()
        .map(|artifact| verify_one_artifact(&root, artifact))
        .collect();
    let all_verified = results
        .iter()
        .all(|result| result.get("verified").and_then(Value::as_bool) == Some(true));
    let receipt = json!({
        "receipt_schema": "hmr0-artifact-binding-receipt-v0",
        "tested_revision": tested_revision,
        "tool_id": spec.tool_id,
        "tool_revision": spec.tool_revision,
        "evidence_class": spec.evidence_class,
        "artifact_manifest_sha256": sha256_hex(manifest_bytes),
        "auditor_source_sha256": sha256_hex(AUDITOR_SOURCE),
        "artifact_count": results.len(),
        "verified_artifact_count": results.iter().filter(|result| result.get("verified").and_then(Value::as_bool) == Some(true)).count(),
        "failed_artifact_count": results.len() - results.iter().filter(|result| result.get("verified").and_then(Value::as_bool) == Some(true)).count(),
        "artifacts": results,
        "physical_actuation_authorized": false,
        "patient_contact_authorized": false,
        "clinical_function_authorized": false,
        "claim_ceiling": CLAIM_CEILING,
        "manifest_claim_ceiling": spec.claim_ceiling
    });
    Ok((receipt, all_verified))
}

fn main() {
    let mut args = env::args().skip(1);
    let root_directory = match args.next() {
        Some(path) => PathBuf::from(path),
        None => {
            eprintln!("usage: medical-robot-artifact-binding <artifact-root> <manifest.json>");
            process::exit(2);
        }
    };
    let manifest_path = match args.next() {
        Some(path) => PathBuf::from(path),
        None => {
            eprintln!("usage: medical-robot-artifact-binding <artifact-root> <manifest.json>");
            process::exit(2);
        }
    };
    if args.next().is_some() {
        eprintln!("too many arguments");
        process::exit(2);
    }
    let manifest_bytes = match fs::read(&manifest_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("manifest read failed: {error}");
            process::exit(2);
        }
    };
    let manifest: Value = match serde_json::from_slice(&manifest_bytes) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("manifest JSON rejected: {error}");
            process::exit(2);
        }
    };
    let tested_revision =
        env::var("HMR0_ARTIFACT_TESTED_SHA").unwrap_or_else(|_| "UNBOUND_LOCAL_RUN".to_owned());
    let (receipt, verified) = match verify_manifest(
        &root_directory,
        &manifest,
        &manifest_bytes,
        &tested_revision,
    ) {
        Ok(result) => result,
        Err(errors) => {
            for error in errors {
                eprintln!("REJECT: {error}");
            }
            process::exit(2);
        }
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt).expect("receipt serializes")
    );
    if !verified {
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    fn temp_root(name: &str) -> PathBuf {
        let path = env::temp_dir().join(format!("hmr0-artifact-binding-{}-{name}", process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("temporary root created");
        path
    }

    fn valid_manifest(relative_path: &str, bytes: &[u8]) -> Value {
        json!({
            "schema_version": "hmr0-artifact-manifest-v0.1",
            "tool_id": "HMR0-TEMPORARY-TOOL",
            "tool_revision": "0.1.0",
            "evidence_class": "concept_preview_only",
            "physical_actuation_authorized": false,
            "patient_contact_authorized": false,
            "clinical_function_authorized": false,
            "artifacts": [{
                "artifact_id": "geometry-preview-1",
                "relative_path": relative_path,
                "media_type": "application/vnd.luminous.hmr0.fixture",
                "byte_length": bytes.len(),
                "sha256": sha256_hex(bytes),
                "role": "geometry_preview"
            }],
            "claim_ceiling": "Test fixture bytes only; not CAD validation or physical-safety evidence."
        })
    }

    fn run(root: &Path, manifest: &Value) -> (Value, bool) {
        let bytes = serde_json::to_vec(manifest).expect("manifest serializes");
        verify_manifest(root, manifest, &bytes, "TESTED-SHA").expect("manifest schema valid")
    }

    #[test]
    fn exact_artifact_bytes_verify() {
        let root = temp_root("exact");
        let bytes = b"synthetic geometry fixture; not actual CAD";
        fs::write(root.join("preview.fixture"), bytes).expect("fixture written");
        let (receipt, success) = run(&root, &valid_manifest("preview.fixture", bytes));
        assert!(success);
        assert_eq!(receipt["verified_artifact_count"], 1);
        assert_eq!(receipt["failed_artifact_count"], 0);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn altered_bytes_fail_digest_binding() {
        let root = temp_root("altered");
        fs::write(root.join("preview.fixture"), b"changed! bytes").expect("fixture written");
        let (receipt, success) = run(
            &root,
            &valid_manifest("preview.fixture", b"expected bytes"),
        );
        assert_eq!(b"changed! bytes".len(), b"expected bytes".len());
        assert!(!success);
        assert_eq!(receipt["artifacts"][0]["digest_matches"], false);
        assert_eq!(receipt["artifacts"][0]["byte_length_matches"], true);
        assert_eq!(receipt["artifacts"][0]["finding"], "ARTIFACT_DIGEST_OR_LENGTH_MISMATCH");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn declared_byte_length_mismatch_is_rejected_even_when_digest_matches() {
        let root = temp_root("length-mismatch");
        let bytes = b"synthetic preview";
        fs::write(root.join("preview.fixture"), bytes).expect("fixture written");
        let mut manifest = valid_manifest("preview.fixture", bytes);
        manifest["artifacts"][0]["byte_length"] = json!(bytes.len() + 1);
        let (receipt, success) = run(&root, &manifest);
        assert!(!success);
        assert_eq!(receipt["artifacts"][0]["digest_matches"], true);
        assert_eq!(receipt["artifacts"][0]["byte_length_matches"], false);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn missing_artifact_fails_closed() {
        let root = temp_root("missing");
        let (receipt, success) = run(&root, &valid_manifest("missing.fixture", b"expected"));
        assert!(!success);
        assert_eq!(receipt["artifacts"][0]["finding"], "ARTIFACT_MISSING");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn traversal_path_is_rejected_before_file_access() {
        let root = temp_root("traversal");
        let manifest = valid_manifest("../outside.fixture", b"expected");
        let errors = parse_manifest(&manifest).expect_err("parent traversal must reject");
        assert!(errors.iter().any(|error| error.contains("ARTIFACT_PATH_INVALID")));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn absolute_path_is_rejected() {
        let manifest = valid_manifest("/tmp/outside.fixture", b"expected");
        let errors = parse_manifest(&manifest).expect_err("absolute paths must reject");
        assert!(errors.iter().any(|error| error.contains("ARTIFACT_PATH_ABSOLUTE")));
    }

    #[test]
    fn symlinked_artifact_is_rejected() {
        let root = temp_root("symlink");
        let outside = root.parent().expect("temp parent").join(format!(
            "hmr0-artifact-binding-{}-outside.fixture",
            process::id()
        ));
        fs::write(&outside, b"outside").expect("outside fixture written");
        symlink(&outside, root.join("linked.fixture")).expect("symlink created");
        let (receipt, success) = run(&root, &valid_manifest("linked.fixture", b"outside"));
        assert!(!success);
        assert_eq!(receipt["artifacts"][0]["finding"], "SYMLINK_NOT_ALLOWED");
        let _ = fs::remove_file(outside);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unknown_manifest_fields_are_rejected() {
        let bytes = b"synthetic geometry fixture";
        let mut manifest = valid_manifest("preview.fixture", bytes);
        manifest["unexpected_authority"] = Value::Bool(true);
        let errors = parse_manifest(&manifest).expect_err("unknown field must reject");
        assert!(errors.iter().any(|error| error.contains("unknown field")));
    }

    #[test]
    fn duplicate_artifact_paths_are_rejected() {
        let bytes = b"synthetic geometry fixture";
        let mut manifest = valid_manifest("preview.fixture", bytes);
        let first = manifest["artifacts"][0].clone();
        manifest["artifacts"].as_array_mut().expect("artifact list").push(first);
        let errors = parse_manifest(&manifest).expect_err("duplicate path/id must reject");
        assert!(errors.iter().any(|error| error.contains("DUPLICATE_RELATIVE_PATH")));
        assert!(errors.iter().any(|error| error.contains("DUPLICATE_ARTIFACT_ID")));
    }

    #[test]
    fn non_false_authorization_flags_are_rejected() {
        let bytes = b"synthetic geometry fixture";
        let mut manifest = valid_manifest("preview.fixture", bytes);
        manifest["physical_actuation_authorized"] = Value::Bool(true);
        let errors = parse_manifest(&manifest).expect_err("authorization cannot turn on");
        assert!(errors
            .iter()
            .any(|error| error.contains("AUTHORIZATION_BOUNDARY_NOT_FALSE")));
    }
}
