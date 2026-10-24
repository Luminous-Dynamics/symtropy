#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

pub const SCHEMA_VERSION: &str = "civ-infra-evidence-bundle/v0.1";
pub const MAX_MANIFEST_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_ARTIFACTS: usize = 10_000;
pub const MAX_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_TOTAL_DECLARED_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_TOTAL_BYTES_READ: u64 = 512 * 1024 * 1024;
const CLAIM_CEILING: &str =
    "integrity_verified_against_manifest_only_not_authenticity_or_physical_validity";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EvidenceBundleManifest {
    pub schema_version: String,
    pub bundle_id: String,
    pub artifacts: Vec<EvidenceArtifact>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EvidenceArtifact {
    pub artifact_id: String,
    pub role: ArtifactRole,
    pub relative_path: String,
    pub expected_byte_len: u64,
    pub expected_sha256: String,
    pub producer: String,
    pub version: String,
    pub applicability_domain_id: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactRole {
    CaseInput,
    ModelArtifact,
    ReferenceArtifact,
    SolverOutput,
    ValidationReport,
    SplitManifest,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactIssue {
    Missing,
    InvalidRelativePath,
    SymlinkDisallowed,
    PathEscape,
    NotDirectory,
    NotRegularFile,
    TooLarge,
    ReadError,
    ChangedDuringRead,
    AggregateByteLimitExceeded,
    ByteLengthMismatch,
    DigestMismatch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactVerification {
    pub artifact_id: String,
    pub role: ArtifactRole,
    pub relative_path: String,
    pub expected_byte_len: u64,
    pub expected_sha256: String,
    pub actual_byte_len: Option<u64>,
    pub actual_sha256: Option<String>,
    pub issues: Vec<ArtifactIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceBundleReport {
    pub schema_version: String,
    pub evaluator_version: String,
    pub bundle_id: String,
    pub input_manifest_sha256: String,
    pub claim_ceiling: String,
    pub integrity_semantics: String,
    pub artifact_count: usize,
    pub verified_artifact_count: usize,
    pub failed_artifact_count: usize,
    pub qualified: bool,
    pub artifacts: Vec<ArtifactVerification>,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Validate and hash the exact manifest bytes, then resolve each artifact beneath root.
/// The manifest is the trust anchor for expected digests; this function establishes byte
/// integrity relative to those declarations, not the authenticity of the declarations.
pub fn verify_bundle(root: &Path, manifest_bytes: &[u8]) -> Result<EvidenceBundleReport, String> {
    if manifest_bytes.len() > MAX_MANIFEST_BYTES {
        return Err(format!(
            "manifest byte count exceeds limit {MAX_MANIFEST_BYTES}"
        ));
    }
    let input_manifest_sha256 = sha256_hex(manifest_bytes);
    let manifest: EvidenceBundleManifest = serde_json::from_slice(manifest_bytes)
        .map_err(|error| format!("invalid manifest JSON: {error}"))?;
    validate_manifest(&manifest)?;

    let canonical_root = fs::canonicalize(root)
        .map_err(|error| format!("cannot resolve bundle root {root:?}: {error}"))?;
    let root_metadata = fs::metadata(&canonical_root)
        .map_err(|error| format!("cannot inspect bundle root {canonical_root:?}: {error}"))?;
    if !root_metadata.is_dir() {
        return Err("bundle root must be a directory".to_owned());
    }

    let mut total_bytes_reserved_for_reads = 0u64;
    let mut artifacts = manifest
        .artifacts
        .iter()
        .map(|artifact| {
            verify_artifact(
                &canonical_root,
                artifact,
                &mut total_bytes_reserved_for_reads,
            )
        })
        .collect::<Vec<_>>();
    artifacts.sort_by(|left, right| left.artifact_id.cmp(&right.artifact_id));

    let verified_artifact_count = artifacts
        .iter()
        .filter(|item| item.issues.is_empty())
        .count();
    let failed_artifact_count = artifacts.len() - verified_artifact_count;
    let qualified = failed_artifact_count == 0 && !artifacts.is_empty();

    Ok(EvidenceBundleReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        evaluator_version: env!("CARGO_PKG_VERSION").to_owned(),
        bundle_id: manifest.bundle_id,
        input_manifest_sha256: input_manifest_sha256,
        claim_ceiling: CLAIM_CEILING.to_owned(),
        integrity_semantics: "the bytes read from each resolved regular file are checked against the manifest's declared byte length and SHA-256. A passing check establishes integrity relative to this manifest only; it does not authenticate the manifest producer, establish chain of custody, validate scientific method, or prove physical correctness.".to_owned(),
        artifact_count: artifacts.len(),
        verified_artifact_count,
        failed_artifact_count,
        qualified,
        artifacts,
    })
}

fn bounded_read_length(
    file_byte_len: u64,
    total_reserved: u64,
    per_file_limit: u64,
    total_limit: u64,
) -> Result<u64, ArtifactIssue> {
    if file_byte_len > per_file_limit {
        return Err(ArtifactIssue::TooLarge);
    }
    let reserve = file_byte_len
        .checked_add(1)
        .ok_or(ArtifactIssue::AggregateByteLimitExceeded)?;
    let total = total_reserved
        .checked_add(reserve)
        .ok_or(ArtifactIssue::AggregateByteLimitExceeded)?;
    if total > total_limit {
        return Err(ArtifactIssue::AggregateByteLimitExceeded);
    }
    Ok(reserve)
}

fn verify_artifact(
    root: &Path,
    artifact: &EvidenceArtifact,
    total_bytes_reserved_for_reads: &mut u64,
) -> ArtifactVerification {
    let mut result = ArtifactVerification {
        artifact_id: artifact.artifact_id.clone(),
        role: artifact.role,
        relative_path: artifact.relative_path.clone(),
        expected_byte_len: artifact.expected_byte_len,
        expected_sha256: artifact.expected_sha256.clone(),
        actual_byte_len: None,
        actual_sha256: None,
        issues: Vec::new(),
    };

    let relative = match validate_relative_path(&artifact.relative_path) {
        Ok(path) => path,
        Err(_) => {
            result.issues.push(ArtifactIssue::InvalidRelativePath);
            return result;
        }
    };

    let mut current = root.to_path_buf();
    let components = relative.components().collect::<Vec<_>>();
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(part) = component else {
            result.issues.push(ArtifactIssue::InvalidRelativePath);
            return result;
        };
        current.push(part);
        let is_final = index + 1 == components.len();
        let metadata = match fs::symlink_metadata(&current) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                result.issues.push(ArtifactIssue::Missing);
                return result;
            }
            Err(_) => {
                result.issues.push(ArtifactIssue::ReadError);
                return result;
            }
        };
        if metadata.file_type().is_symlink() {
            result.issues.push(ArtifactIssue::SymlinkDisallowed);
            return result;
        }
        if !is_final && !metadata.is_dir() {
            result.issues.push(ArtifactIssue::NotDirectory);
            return result;
        }
        if is_final && !metadata.is_file() {
            result.issues.push(ArtifactIssue::NotRegularFile);
            return result;
        }
    }

    let resolved = match fs::canonicalize(&current) {
        Ok(path) => path,
        Err(_) => {
            result.issues.push(ArtifactIssue::ReadError);
            return result;
        }
    };
    if !resolved.starts_with(root) {
        result.issues.push(ArtifactIssue::PathEscape);
        return result;
    }

    let metadata = match fs::metadata(&resolved) {
        Ok(metadata) => metadata,
        Err(_) => {
            result.issues.push(ArtifactIssue::ReadError);
            return result;
        }
    };
    let read_limit = match bounded_read_length(
        metadata.len(),
        *total_bytes_reserved_for_reads,
        MAX_ARTIFACT_BYTES,
        MAX_TOTAL_BYTES_READ,
    ) {
        Ok(limit) => limit,
        Err(issue) => {
            result.actual_byte_len = Some(metadata.len());
            result.issues.push(issue);
            return result;
        }
    };

    let file = match File::open(&resolved) {
        Ok(file) => file,
        Err(_) => {
            result.issues.push(ArtifactIssue::ReadError);
            return result;
        }
    };
    *total_bytes_reserved_for_reads =
        match (*total_bytes_reserved_for_reads).checked_add(read_limit) {
            Some(total) => total,
            None => {
                result
                    .issues
                    .push(ArtifactIssue::AggregateByteLimitExceeded);
                return result;
            }
        };

    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    let mut bounded_reader = file.take(read_limit);
    if bounded_reader.read_to_end(&mut bytes).is_err() {
        result.issues.push(ArtifactIssue::ReadError);
        return result;
    }
    if bytes.len() as u64 > MAX_ARTIFACT_BYTES {
        result.actual_byte_len = Some(bytes.len() as u64);
        result.issues.push(ArtifactIssue::TooLarge);
        return result;
    }
    if bytes.len() as u64 != metadata.len() {
        result.actual_byte_len = Some(bytes.len() as u64);
        result.issues.push(ArtifactIssue::ChangedDuringRead);
        return result;
    }

    let actual_byte_len = bytes.len() as u64;
    let actual_sha256 = sha256_hex(&bytes);
    result.actual_byte_len = Some(actual_byte_len);
    result.actual_sha256 = Some(actual_sha256.clone());
    if actual_byte_len != artifact.expected_byte_len {
        result.issues.push(ArtifactIssue::ByteLengthMismatch);
    }
    if actual_sha256 != artifact.expected_sha256 {
        result.issues.push(ArtifactIssue::DigestMismatch);
    }
    result.issues.sort();
    result
}

fn validate_manifest(manifest: &EvidenceBundleManifest) -> Result<(), String> {
    if manifest.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "unsupported schema_version {:?}",
            manifest.schema_version
        ));
    }
    validate_name(&manifest.bundle_id, "bundle_id")?;
    if manifest.claim_ceiling != CLAIM_CEILING {
        return Err("claim_ceiling must match the fixed integrity-only ceiling".to_owned());
    }
    if manifest.artifacts.is_empty() || manifest.artifacts.len() > MAX_ARTIFACTS {
        return Err(format!("artifact count must be within 1..={MAX_ARTIFACTS}"));
    }

    let mut ids = BTreeSet::new();
    let mut paths = BTreeSet::new();
    let mut total_bytes = 0u64;
    for artifact in &manifest.artifacts {
        validate_name(&artifact.artifact_id, "artifact_id")?;
        validate_name(&artifact.producer, "producer")?;
        validate_name(&artifact.version, "version")?;
        validate_name(&artifact.applicability_domain_id, "applicability_domain_id")?;
        if !ids.insert(artifact.artifact_id.clone()) {
            return Err(format!("duplicate artifact_id {:?}", artifact.artifact_id));
        }
        validate_relative_path(&artifact.relative_path).map_err(|error| {
            format!(
                "invalid relative_path for {:?}: {error}",
                artifact.artifact_id
            )
        })?;
        if !paths.insert(artifact.relative_path.clone()) {
            return Err(format!(
                "duplicate relative_path {:?}",
                artifact.relative_path
            ));
        }
        validate_digest(&artifact.expected_sha256, "expected_sha256")?;
        if artifact.expected_byte_len > MAX_ARTIFACT_BYTES {
            return Err(format!(
                "artifact {:?} exceeds per-artifact byte limit {MAX_ARTIFACT_BYTES}",
                artifact.artifact_id
            ));
        }
        total_bytes = total_bytes
            .checked_add(artifact.expected_byte_len)
            .ok_or_else(|| "declared artifact byte total overflow".to_owned())?;
        if total_bytes > MAX_TOTAL_DECLARED_BYTES {
            return Err(format!(
                "declared artifact bytes exceed total limit {MAX_TOTAL_DECLARED_BYTES}"
            ));
        }
    }
    Ok(())
}

fn validate_relative_path(value: &str) -> Result<PathBuf, String> {
    if value.trim().is_empty() {
        return Err("path must not be empty".to_owned());
    }
    if value.contains('\\') || value.contains(':') || value.starts_with('/') {
        return Err("only normalized relative slash-separated paths are allowed".to_owned());
    }
    let parts = value.split('/').collect::<Vec<_>>();
    if parts
        .iter()
        .any(|part| part.is_empty() || *part == "." || *part == "..")
    {
        return Err(
            "empty, current-directory, or parent-directory components are forbidden".to_owned(),
        );
    }
    let path = Path::new(value);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err("path must contain only normal relative components".to_owned());
    }
    Ok(path.to_path_buf())
}

fn validate_name(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    if value.len() > 1024 {
        return Err(format!("{label} exceeds 1024 UTF-8 bytes"));
    }
    Ok(())
}

fn validate_digest(value: &str, label: &str) -> Result<(), String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "{label} must be a lowercase 64-character SHA-256 digest"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const MANIFEST: &[u8] = include_bytes!("../fixtures/evidence-bundle-v0.1.json");
    const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/bundle-root");

    fn parsed_manifest() -> EvidenceBundleManifest {
        serde_json::from_slice(MANIFEST).expect("frozen manifest parses")
    }

    fn temp_dir() -> PathBuf {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "civ-infra-evidence-bundle-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).expect("temporary root is created");
        path
    }

    fn verify_mutated(manifest: &EvidenceBundleManifest) -> Result<EvidenceBundleReport, String> {
        let bytes = serde_json::to_vec(manifest).expect("manifest serializes");
        verify_bundle(Path::new(ROOT), &bytes)
    }

    #[test]
    fn aggregate_read_budget_includes_the_size_change_probe() {
        assert_eq!(bounded_read_length(10, 0, 20, 30), Ok(11));
        assert_eq!(
            bounded_read_length(10, 20, 20, 30),
            Err(ArtifactIssue::AggregateByteLimitExceeded)
        );
        assert_eq!(
            bounded_read_length(21, 0, 20, 30),
            Err(ArtifactIssue::TooLarge)
        );
    }

    #[test]
    fn aggregate_declared_size_limit_is_checked_before_file_reads() {
        let mut manifest = parsed_manifest();
        let template = manifest.artifacts[0].clone();
        manifest.artifacts.clear();
        for index in 0..9usize {
            let mut artifact = template.clone();
            artifact.artifact_id = format!("oversize-{index:02}");
            artifact.relative_path = format!("missing/artifact-{index:02}.json");
            artifact.expected_byte_len = MAX_ARTIFACT_BYTES;
            manifest.artifacts.push(artifact);
        }
        let error = verify_mutated(&manifest).expect_err("declared aggregate must exceed the cap");
        assert!(
            error.contains("declared artifact bytes exceed total limit"),
            "{error}"
        );
    }

    #[test]
    fn verifies_every_frozen_artifact_against_exact_bytes() {
        let report = verify_bundle(Path::new(ROOT), MANIFEST).expect("manifest is valid");
        assert!(report.qualified);
        assert_eq!(report.artifact_count, 3);
        assert_eq!(report.verified_artifact_count, 3);
        assert_eq!(report.failed_artifact_count, 0);
        assert_eq!(report.input_manifest_sha256, sha256_hex(MANIFEST));
        assert!(
            report
                .artifacts
                .iter()
                .all(|artifact| artifact.issues.is_empty())
        );
    }

    #[test]
    fn digest_and_length_mismatch_are_reported_without_qualification() {
        let mut manifest = parsed_manifest();
        manifest.artifacts[0].expected_sha256 = "0".repeat(64);
        manifest.artifacts[1].expected_byte_len += 1;
        let report = verify_mutated(&manifest).expect("well-formed but false claims are reported");
        assert!(!report.qualified);
        assert_eq!(report.failed_artifact_count, 2);
        assert!(
            report
                .artifacts
                .iter()
                .find(|item| item.artifact_id == "case-input-001")
                .expect("case artifact")
                .issues
                .contains(&ArtifactIssue::DigestMismatch)
        );
        assert!(
            report
                .artifacts
                .iter()
                .find(|item| item.artifact_id == "model-artifact-001")
                .expect("model artifact")
                .issues
                .contains(&ArtifactIssue::ByteLengthMismatch)
        );
    }

    #[test]
    fn missing_artifact_fails_closed_and_stays_visible_in_report() {
        let mut manifest = parsed_manifest();
        manifest.artifacts[0].relative_path = "absent/not-present.json".to_owned();
        let report = verify_mutated(&manifest).expect("missing artifact is a reportable result");
        assert!(!report.qualified);
        assert!(
            report
                .artifacts
                .iter()
                .find(|item| item.artifact_id == "case-input-001")
                .expect("case artifact")
                .issues
                .contains(&ArtifactIssue::Missing)
        );
    }

    #[test]
    fn parent_directory_and_windows_style_paths_are_rejected() {
        for path in [
            "../outside.json",
            "cases/../../outside.json",
            "C:/outside.json",
            "cases\\outside.json",
            "/etc/passwd",
        ] {
            let mut manifest = parsed_manifest();
            manifest.artifacts[0].relative_path = path.to_owned();
            let error = verify_mutated(&manifest).expect_err("unsafe path must be rejected");
            assert!(error.contains("invalid relative_path"), "{path}: {error}");
        }
    }

    #[test]
    fn duplicate_ids_or_paths_are_rejected_before_resolution() {
        let mut manifest = parsed_manifest();
        manifest.artifacts[1].artifact_id = manifest.artifacts[0].artifact_id.clone();
        assert!(
            verify_mutated(&manifest)
                .unwrap_err()
                .contains("duplicate artifact_id")
        );

        let mut manifest = parsed_manifest();
        manifest.artifacts[1].relative_path = manifest.artifacts[0].relative_path.clone();
        assert!(
            verify_mutated(&manifest)
                .unwrap_err()
                .contains("duplicate relative_path")
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_artifacts_are_rejected() {
        use std::os::unix::fs::symlink;

        let temp = temp_dir();
        let root = temp.join("bundle");
        fs::create_dir_all(root.join("cases")).expect("case directory");
        fs::write(temp.join("outside.json"), b"outside").expect("outside file");
        symlink(temp.join("outside.json"), root.join("cases/outside.json")).expect("symlink");
        let mut manifest = parsed_manifest();
        manifest.artifacts.truncate(1);
        manifest.artifacts[0].relative_path = "cases/outside.json".to_owned();
        manifest.artifacts[0].expected_byte_len = 7;
        manifest.artifacts[0].expected_sha256 = sha256_hex(b"outside");
        let bytes = serde_json::to_vec(&manifest).expect("manifest serializes");
        let report = verify_bundle(&root, &bytes).expect("symlink issue is returned in report");
        assert!(!report.qualified);
        assert!(
            report.artifacts[0]
                .issues
                .contains(&ArtifactIssue::SymlinkDisallowed)
        );
        fs::remove_dir_all(temp).expect("temporary tree removed");
    }

    #[test]
    fn report_order_is_stable_even_if_manifest_artifacts_are_reordered() {
        let baseline = verify_bundle(Path::new(ROOT), MANIFEST).expect("baseline");
        let mut manifest = parsed_manifest();
        manifest.artifacts.reverse();
        let bytes = serde_json::to_vec(&manifest).expect("permuted manifest serializes");
        let permuted = verify_bundle(Path::new(ROOT), &bytes).expect("permuted manifest");
        assert_ne!(
            baseline.input_manifest_sha256,
            permuted.input_manifest_sha256
        );
        assert_eq!(
            baseline
                .artifacts
                .iter()
                .map(|a| &a.artifact_id)
                .collect::<Vec<_>>(),
            permuted
                .artifacts
                .iter()
                .map(|a| &a.artifact_id)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            baseline.verified_artifact_count,
            permuted.verified_artifact_count
        );
    }

    #[test]
    fn claims_cannot_smuggle_unknown_probability_fields() {
        let mut value: serde_json::Value =
            serde_json::from_slice(MANIFEST).expect("manifest parses");
        value
            .as_object_mut()
            .expect("object")
            .insert("magic_probability".to_owned(), serde_json::json!(0.99));
        let bytes = serde_json::to_vec(&value).expect("mutated manifest serializes");
        assert!(
            verify_bundle(Path::new(ROOT), &bytes)
                .unwrap_err()
                .contains("unknown field")
        );
    }
}
