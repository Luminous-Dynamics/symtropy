use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const SCHEMA_VERSION: &str = "luminous.plasma-dataset-admission.v0.1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseDecision {
    Unknown,
    Restricted,
    EvaluationOnly,
    RedistributionAllowed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LicenseReview {
    pub decision: LicenseDecision,
    pub license_id: String,
    pub evidence_uri: String,
    pub reviewed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactFile {
    pub artifact_id: String,
    pub file_name: String,
    pub reported_size: Option<String>,
    pub expected_size_bytes: Option<u64>,
    pub remote_md5: Option<String>,
    pub remote_sha256: Option<String>,
    pub observed_size_bytes: Option<u64>,
    pub observed_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceManifest {
    pub schema_version: String,
    pub source_id: String,
    pub record_doi: String,
    pub record_uri: String,
    pub version: String,
    pub metadata_captured_at: String,
    pub license_review: Option<LicenseReview>,
    pub artifacts: Vec<ArtifactFile>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AdmissionDisposition {
    ReadyForLocalEvaluation,
    ReadyForRedistribution,
    BlockedPendingEvidence,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditFinding {
    pub code: String,
    pub artifact_ids: Vec<String>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditReportBody {
    pub schema_version: String,
    pub source_id: String,
    pub record_doi: String,
    pub version: String,
    pub disposition: AdmissionDisposition,
    pub local_evaluation_allowed: bool,
    pub redistribution_allowed: bool,
    pub manifest_sha256: String,
    pub findings: Vec<AuditFinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditReport {
    pub body: AuditReportBody,
    pub receipt_sha256: String,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn is_hex(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn finding(code: &str, ids: Vec<String>, detail: &str) -> AuditFinding {
    AuditFinding {
        code: code.to_owned(),
        artifact_ids: ids,
        detail: detail.to_owned(),
    }
}

/// Bind the locally acquired bytes to their exact size and SHA-256.
/// This records local byte identity; it does not authenticate an upstream release.
pub fn record_local_bytes(file: &mut ArtifactFile, bytes: &[u8]) {
    file.observed_size_bytes = Some(bytes.len() as u64);
    file.observed_sha256 = Some(sha256_hex(bytes));
}

pub fn audit_manifest(manifest: &SourceManifest) -> Result<AuditReport, String> {
    let manifest_bytes =
        serde_json::to_vec(manifest).map_err(|error| format!("manifest serialization failed: {error}"))?;
    let manifest_sha256 = sha256_hex(&manifest_bytes);
    let mut findings = Vec::new();
    let mut rejected = false;
    let mut blocked = false;

    if manifest.schema_version != SCHEMA_VERSION {
        findings.push(finding("unsupported_schema_version", vec![], "The manifest schema version is not supported."));
        rejected = true;
    }
    if !non_empty(&manifest.source_id)
        || !non_empty(&manifest.record_doi)
        || !manifest.record_uri.starts_with("https://")
        || !non_empty(&manifest.version)
        || !non_empty(&manifest.metadata_captured_at)
    {
        findings.push(finding("invalid_record_identity", vec![], "Record ID, DOI, HTTPS URI, version and capture timestamp are required."));
        rejected = true;
    }
    if manifest.artifacts.is_empty() {
        findings.push(finding("no_artifacts", vec![], "At least one artifact must be declared."));
        rejected = true;
    }

    let mut artifact_ids = std::collections::BTreeSet::new();
    let mut file_names = std::collections::BTreeSet::new();
    let mut md5_groups: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for artifact in &manifest.artifacts {
        if !non_empty(&artifact.artifact_id)
            || !non_empty(&artifact.file_name)
            || !artifact_ids.insert(artifact.artifact_id.clone())
            || !file_names.insert(artifact.file_name.clone())
        {
            findings.push(finding(
                "invalid_or_duplicate_artifact_identity",
                vec![artifact.artifact_id.clone()],
                "Artifact IDs and original filenames must be non-empty and unique.",
            ));
            rejected = true;
        }

        if artifact.reported_size.as_ref().is_none_or(|size| !non_empty(size)) {
            findings.push(finding(
                "reported_size_missing",
                vec![artifact.artifact_id.clone()],
                "Preserve the source-reported file size or explicitly record why it is unavailable.",
            ));
            blocked = true;
        }

        if let Some(md5) = &artifact.remote_md5 {
            if !is_hex(md5, 32) {
                findings.push(finding(
                    "invalid_remote_md5",
                    vec![artifact.artifact_id.clone()],
                    "A reported MD5 value must contain exactly 32 hexadecimal characters.",
                ));
                rejected = true;
            } else {
                md5_groups
                    .entry(md5.to_ascii_lowercase())
                    .or_default()
                    .push(artifact.artifact_id.clone());
            }
        }
        if let Some(sha256) = &artifact.remote_sha256
            && !is_hex(sha256, 64)
        {
            findings.push(finding(
                "invalid_remote_sha256",
                vec![artifact.artifact_id.clone()],
                "A reported SHA-256 value must contain exactly 64 hexadecimal characters.",
            ));
            rejected = true;
        }
        if let Some(sha256) = &artifact.observed_sha256
            && !is_hex(sha256, 64)
        {
            findings.push(finding(
                "invalid_observed_sha256",
                vec![artifact.artifact_id.clone()],
                "A locally observed SHA-256 value must contain exactly 64 hexadecimal characters.",
            ));
            rejected = true;
        }

        match (artifact.observed_size_bytes, artifact.observed_sha256.as_ref()) {
            (None, None) => {
                findings.push(finding(
                    "artifact_not_locally_acquired",
                    vec![artifact.artifact_id.clone()],
                    "No local byte length or SHA-256 has been recorded.",
                ));
                blocked = true;
            }
            (Some(_), None) | (None, Some(_)) => {
                findings.push(finding(
                    "incomplete_local_observation",
                    vec![artifact.artifact_id.clone()],
                    "Local byte length and SHA-256 must be recorded together.",
                ));
                rejected = true;
            }
            (Some(observed_size), Some(observed_sha256)) => {
                if let Some(expected_size) = artifact.expected_size_bytes
                    && observed_size != expected_size
                {
                    findings.push(finding(
                        "local_size_mismatch",
                        vec![artifact.artifact_id.clone()],
                        "The locally observed byte length does not match the pinned exact source size.",
                    ));
                    rejected = true;
                }
                if let Some(expected_sha256) = &artifact.remote_sha256
                    && !observed_sha256.eq_ignore_ascii_case(expected_sha256)
                {
                    findings.push(finding(
                        "upstream_sha256_mismatch",
                        vec![artifact.artifact_id.clone()],
                        "The locally observed SHA-256 does not match the source-reported SHA-256.",
                    ));
                    rejected = true;
                }
                if artifact.remote_sha256.is_none() {
                    findings.push(finding(
                        "upstream_sha256_unavailable",
                        vec![artifact.artifact_id.clone()],
                        "Local SHA-256 identifies staged bytes but cannot authenticate them against a source-published SHA-256; MD5-only source metadata is not admitted by this version.",
                    ));
                    blocked = true;
                }
            }
        }
    }

    for ids in md5_groups.values_mut() {
        ids.sort();
        ids.dedup();
        if ids.len() > 1 {
            findings.push(finding(
                "duplicate_remote_md5_quarantine",
                ids.clone(),
                "Distinct artifact identities share one source-reported MD5. Quarantine pending byte-level inspection and upstream clarification; do not infer identity.",
            ));
            blocked = true;
        }
    }

    let license_allows_evaluation = match &manifest.license_review {
        None => {
            findings.push(finding("license_review_missing", vec![], "No human-reviewed license decision is recorded; unknown is not permission."));
            blocked = true;
            false
        }
        Some(review) => {
            if !non_empty(&review.license_id)
                || !review.evidence_uri.starts_with("https://")
                || !non_empty(&review.reviewed_at)
            {
                findings.push(finding("license_review_incomplete", vec![], "License ID, HTTPS evidence URI and review timestamp are required."));
                blocked = true;
            }
            match review.decision {
                LicenseDecision::Unknown => {
                    findings.push(finding("license_unknown", vec![], "The explicit license decision is unknown."));
                    blocked = true;
                    false
                }
                LicenseDecision::Restricted => {
                    findings.push(finding("license_restricted", vec![], "The recorded license review does not permit this evaluation."));
                    blocked = true;
                    false
                }
                LicenseDecision::EvaluationOnly | LicenseDecision::RedistributionAllowed => {
                    non_empty(&review.license_id)
                        && review.evidence_uri.starts_with("https://")
                        && non_empty(&review.reviewed_at)
                }
            }
        }
    };

    let local_bytes_complete = manifest.artifacts.iter().all(|artifact| {
        artifact.observed_size_bytes.is_some() && artifact.observed_sha256.as_ref().is_some_and(|value| is_hex(value, 64))
    });
    let local_evaluation_allowed = license_allows_evaluation
        && local_bytes_complete
        && !blocked
        && !rejected;
    let redistribution_allowed = local_evaluation_allowed
        && manifest.license_review.as_ref().is_some_and(|review| {
            review.decision == LicenseDecision::RedistributionAllowed
        });

    let disposition = if rejected {
        AdmissionDisposition::Rejected
    } else if blocked || !local_evaluation_allowed {
        AdmissionDisposition::BlockedPendingEvidence
    } else if redistribution_allowed {
        AdmissionDisposition::ReadyForRedistribution
    } else {
        AdmissionDisposition::ReadyForLocalEvaluation
    };

    let body = AuditReportBody {
        schema_version: SCHEMA_VERSION.to_owned(),
        source_id: manifest.source_id.clone(),
        record_doi: manifest.record_doi.clone(),
        version: manifest.version.clone(),
        disposition,
        local_evaluation_allowed,
        redistribution_allowed,
        manifest_sha256,
        findings,
    };
    let receipt_bytes =
        serde_json::to_vec(&body).map_err(|error| format!("receipt serialization failed: {error}"))?;
    let receipt_sha256 = sha256_hex(&receipt_bytes);

    Ok(AuditReport {
        body,
        receipt_sha256,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_manifest(decision: LicenseDecision) -> SourceManifest {
        let mut artifact = ArtifactFile {
            artifact_id: "artifact-a".to_owned(),
            file_name: "reference.mat".to_owned(),
            reported_size: Some("3 bytes".to_owned()),
            expected_size_bytes: Some(3),
            remote_md5: None,
            remote_sha256: Some("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".to_owned()),
            observed_size_bytes: None,
            observed_sha256: None,
        };
        record_local_bytes(&mut artifact, b"abc");
        SourceManifest {
            schema_version: SCHEMA_VERSION.to_owned(),
            source_id: "fixture-a".to_owned(),
            record_doi: "10.5281/zenodo.example".to_owned(),
            record_uri: "https://zenodo.org/records/example".to_owned(),
            version: "v1".to_owned(),
            metadata_captured_at: "2026-11-19T00:00:00Z".to_owned(),
            license_review: Some(LicenseReview {
                decision,
                license_id: "CC-BY-4.0".to_owned(),
                evidence_uri: "https://zenodo.org/records/example".to_owned(),
                reviewed_at: "2026-11-19".to_owned(),
            }),
            artifacts: vec![artifact],
        }
    }

    #[test]
    fn metadata_fixture_quarantines_duplicate_md5_and_missing_rights() {
        let source = include_str!("../../../../benchmarks/plasma/dataset-admission-pnas-2025-v0.json");
        let manifest: SourceManifest = serde_json::from_str(source).expect("fixture parses");
        let report = audit_manifest(&manifest).expect("audit runs");
        assert_eq!(report.body.disposition, AdmissionDisposition::BlockedPendingEvidence);
        assert!(!report.body.local_evaluation_allowed);
        assert!(!report.body.redistribution_allowed);
        assert!(report.body.findings.iter().any(|item| item.code == "duplicate_remote_md5_quarantine"));
        assert!(report.body.findings.iter().any(|item| item.code == "license_review_missing"));
    }

    #[test]
    fn evaluation_only_never_implies_redistribution() {
        let report = audit_manifest(&base_manifest(LicenseDecision::EvaluationOnly)).expect("audit runs");
        assert_eq!(report.body.disposition, AdmissionDisposition::ReadyForLocalEvaluation);
        assert!(report.body.local_evaluation_allowed);
        assert!(!report.body.redistribution_allowed);
    }

    #[test]
    fn explicit_redistribution_permission_is_separate() {
        let report = audit_manifest(&base_manifest(LicenseDecision::RedistributionAllowed)).expect("audit runs");
        assert_eq!(report.body.disposition, AdmissionDisposition::ReadyForRedistribution);
        assert!(report.body.local_evaluation_allowed);
        assert!(report.body.redistribution_allowed);
    }

    #[test]
    fn missing_license_blocks_admission() {
        let mut manifest = base_manifest(LicenseDecision::EvaluationOnly);
        manifest.license_review = None;
        let report = audit_manifest(&manifest).expect("audit runs");
        assert_eq!(report.body.disposition, AdmissionDisposition::BlockedPendingEvidence);
        assert!(!report.body.local_evaluation_allowed);
    }

    #[test]
    fn unknown_or_restricted_license_blocks_admission() {
        for decision in [LicenseDecision::Unknown, LicenseDecision::Restricted] {
            let report = audit_manifest(&base_manifest(decision)).expect("audit runs");
            assert_eq!(report.body.disposition, AdmissionDisposition::BlockedPendingEvidence);
        }
    }

    #[test]
    fn exact_size_mismatch_is_rejected() {
        let mut manifest = base_manifest(LicenseDecision::EvaluationOnly);
        manifest.artifacts[0].observed_size_bytes = Some(4);
        let report = audit_manifest(&manifest).expect("audit runs");
        assert_eq!(report.body.disposition, AdmissionDisposition::Rejected);
        assert!(!report.body.local_evaluation_allowed);
    }

    #[test]
    fn upstream_sha256_mismatch_is_rejected() {
        let mut manifest = base_manifest(LicenseDecision::EvaluationOnly);
        manifest.artifacts[0].remote_sha256 = Some("00".repeat(32));
        let report = audit_manifest(&manifest).expect("audit runs");
        assert_eq!(report.body.disposition, AdmissionDisposition::Rejected);
    }

    #[test]
    fn repeated_remote_md5_quarantines_without_claiming_byte_identity() {
        let mut manifest = base_manifest(LicenseDecision::EvaluationOnly);
        let mut second = manifest.artifacts[0].clone();
        second.artifact_id = "artifact-b".to_owned();
        second.file_name = "second.mat".to_owned();
        second.remote_md5 = Some("0123456789abcdef0123456789abcdef".to_owned());
        manifest.artifacts[0].remote_md5 = Some("0123456789abcdef0123456789abcdef".to_owned());
        manifest.artifacts.push(second);
        let report = audit_manifest(&manifest).expect("audit runs");
        assert_eq!(report.body.disposition, AdmissionDisposition::BlockedPendingEvidence);
        let duplicate = report.body.findings.iter().find(|item| item.code == "duplicate_remote_md5_quarantine").expect("duplicate finding");
        assert_eq!(duplicate.artifact_ids, vec!["artifact-a".to_owned(), "artifact-b".to_owned()]);
        assert!(!duplicate.detail.contains("identical"));
    }

    #[test]
    fn invalid_digest_syntax_is_rejected() {
        let mut manifest = base_manifest(LicenseDecision::EvaluationOnly);
        manifest.artifacts[0].remote_md5 = Some("not-an-md5".to_owned());
        let report = audit_manifest(&manifest).expect("audit runs");
        assert_eq!(report.body.disposition, AdmissionDisposition::Rejected);
    }

    #[test]
    fn incomplete_local_observation_is_rejected() {
        let mut manifest = base_manifest(LicenseDecision::EvaluationOnly);
        manifest.artifacts[0].observed_sha256 = None;
        let report = audit_manifest(&manifest).expect("audit runs");
        assert_eq!(report.body.disposition, AdmissionDisposition::Rejected);
    }

    #[test]
    fn missing_upstream_sha_is_reported_as_local_integrity_only() {
        let mut manifest = base_manifest(LicenseDecision::EvaluationOnly);
        manifest.artifacts[0].remote_sha256 = None;
        let report = audit_manifest(&manifest).expect("audit runs");
        assert_eq!(report.body.disposition, AdmissionDisposition::BlockedPendingEvidence);
        assert!(!report.body.local_evaluation_allowed);
        assert!(report.body.findings.iter().any(|item| item.code == "upstream_sha256_unavailable"));
    }

    #[test]
    fn receipt_is_deterministic_for_identical_manifest() {
        let manifest = base_manifest(LicenseDecision::EvaluationOnly);
        let first = audit_manifest(&manifest).expect("first audit");
        let second = audit_manifest(&manifest).expect("second audit");
        assert_eq!(first.receipt_sha256, second.receipt_sha256);
        assert_eq!(first, second);
    }

    #[test]
    fn unknown_fields_are_refused() {
        let input = r#"{"schema_version":"luminous.plasma-dataset-admission.v0.1","source_id":"x","record_doi":"10/x","record_uri":"https://example.org","version":"v1","metadata_captured_at":"now","license_review":null,"artifacts":[],"unexpected":true}"#;
        assert!(serde_json::from_str::<SourceManifest>(input).is_err());
    }

    #[test]
    fn local_byte_observation_uses_sha256() {
        let mut artifact = base_manifest(LicenseDecision::EvaluationOnly).artifacts.remove(0);
        record_local_bytes(&mut artifact, b"abc");
        assert_eq!(artifact.observed_size_bytes, Some(3));
        assert_eq!(
            artifact.observed_sha256.as_deref(),
            Some("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        );
    }
}
