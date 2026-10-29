// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Fail-closed, content-addressed qualification receipts for scientific runs.
//!
//! This crate records what a declared verifier checked; it does not prove
//! physical truth or model adequacy on its own. Pass is accepted only for an
//! executed, successful run with complete case accounting, clean source
//! provenance, a declared independent oracle, and all required artifacts.
//!
//! Canonical receipt identity is SHA-256 over the serialized ReceiptPayload
//! only. Volatile envelope metadata (recording time, worker label) is excluded
//! from that digest by design. Payload maps use BTreeMap for stable key order.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

/// Current receipt schema version.
pub const RECEIPT_SCHEMA_VERSION: u32 = 1;

/// High-level verifier verdict. Skipped, NotRun, and Indeterminate are never
/// synonyms for Pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    Pass,
    Fail,
    Indeterminate,
    NotRun,
    Skipped,
    InvalidReceipt,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    /// No command was started.
    NotStarted,
    /// The command finished and its exit code was captured.
    Completed,
    /// The command ran and returned a non-zero exit code.
    Failed,
    /// Execution stopped because a declared timeout expired.
    TimedOut,
    /// The required executable/runner/resource was unavailable.
    Unavailable,
    /// The protocol deliberately did not select this check.
    Skipped,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OracleIndependence {
    /// The oracle was constructed independently of the production helper under test.
    Independent,
    /// The oracle shares one or more relevant helpers with the production path.
    SharedProductionHelpers,
    /// This claim has no external/differential oracle, and must say why.
    NotApplicable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    ScenarioInput,
    Configuration,
    OracleReference,
    Stdout,
    Stderr,
    Result,
    FailureFixture,
    ReferenceData,
    SourceSnapshot,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricClass {
    HardGate,
    Performance,
    Diagnostic,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceIdentity {
    pub repository: String,
    pub commit_sha: String,
    pub working_tree_clean: bool,
    pub cargo_lock_sha256: String,
    pub toolchain: String,
    pub target_triple: String,
    pub feature_flags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScenarioIdentity {
    pub input_sha256: String,
    pub configuration_sha256: String,
    pub units: String,
    pub reference_frame: String,
    /// Explicit dimensions, physical model, numeric ranges and exclusions.
    pub validity_envelope: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OracleIdentity {
    pub name: String,
    pub version: String,
    pub digest_sha256: String,
    pub method: String,
    pub independence: OracleIndependence,
    /// Explain the relationship between oracle code and the implementation.
    pub independence_scope: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseCounts {
    /// Number of cases whose evaluation was actually attempted.
    pub attempted: u64,
    pub passed: u64,
    pub failed: u64,
    /// Attempted cases that did not reach a definitive outcome.
    pub unresolved: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub status: ExecutionStatus,
    /// Executed program plus arguments as an array, never a shell command string.
    pub command: Vec<String>,
    pub exit_code: Option<i32>,
    pub duration_millis: Option<u64>,
    pub cases: CaseCounts,
    /// Required for timeout/unavailable/skipped/indeterminate dispositions.
    pub outcome_reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricRecord {
    pub name: String,
    pub value: Option<f64>,
    pub unit: String,
    pub class: MetricClass,
    /// Used when no value exists or when a metric is diagnostically unresolved.
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactRef {
    pub kind: ArtifactKind,
    pub logical_name: String,
    pub sha256: String,
    pub media_type: String,
    pub size_bytes: u64,
}

impl ArtifactRef {
    pub fn digest_bytes(bytes: &[u8]) -> String {
        sha256_hex(bytes)
    }

    /// Verify a referenced artifact against bytes obtained from the content store.
    pub fn verify_bytes(&self, bytes: &[u8]) -> Result<(), ReceiptError> {
        if !is_sha256(&self.sha256) {
            return Err(ReceiptError::InvalidDigest {
                field: "artifact.sha256",
            });
        }
        let actual = sha256_hex(bytes);
        if actual != self.sha256 {
            return Err(ReceiptError::ArtifactDigestMismatch {
                logical_name: self.logical_name.clone(),
                expected: self.sha256.clone(),
                actual,
            });
        }
        if bytes.len() as u64 != self.size_bytes {
            return Err(ReceiptError::ArtifactSizeMismatch {
                logical_name: self.logical_name.clone(),
                expected: self.size_bytes,
                actual: bytes.len() as u64,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReceiptPayload {
    pub schema_version: u32,
    pub benchmark_id: String,
    pub benchmark_revision: String,
    pub scenario_id: String,
    pub scenario_revision: String,
    pub source: SourceIdentity,
    pub scenario: ScenarioIdentity,
    pub oracle: OracleIdentity,
    pub execution: ExecutionRecord,
    pub disposition: Disposition,
    pub metrics: Vec<MetricRecord>,
    pub artifacts: Vec<ArtifactRef>,
    /// Stable protocol metadata only; volatile execution metadata belongs outside the payload.
    pub protocol_metadata: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VolatileMetadata {
    pub recorded_at_utc: Option<String>,
    pub worker_label: Option<String>,
}

impl Default for VolatileMetadata {
    fn default() -> Self {
        Self {
            recorded_at_utc: None,
            worker_label: None,
        }
    }
}

/// Versioned receipt envelope. receipt_sha256 commits to the payload only;
/// changing volatile metadata does not alter scientific-result identity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QualificationReceipt {
    pub payload: ReceiptPayload,
    pub receipt_sha256: String,
    pub volatile: VolatileMetadata,
}

impl QualificationReceipt {
    pub fn new(
        payload: ReceiptPayload,
        volatile: VolatileMetadata,
    ) -> Result<Self, ReceiptError> {
        payload.validate()?;
        let receipt_sha256 = payload.digest()?;
        Ok(Self {
            payload,
            receipt_sha256,
            volatile,
        })
    }

    /// Revalidate schema semantics and confirm that the payload digest matches.
    pub fn verify(&self) -> Result<(), ReceiptError> {
        self.payload.validate()?;
        let actual = self.payload.digest()?;
        if !is_sha256(&self.receipt_sha256) {
            return Err(ReceiptError::InvalidDigest {
                field: "receipt_sha256",
            });
        }
        if actual != self.receipt_sha256 {
            return Err(ReceiptError::ReceiptDigestMismatch {
                expected: self.receipt_sha256.clone(),
                actual,
            });
        }
        Ok(())
    }

    /// Canonical bytes used for the receipt's scientific identity digest.
    pub fn canonical_payload_bytes(&self) -> Result<Vec<u8>, ReceiptError> {
        self.payload.canonical_bytes()
    }

    /// Verify a known artifact by logical name. A missing artifact is an error,
    /// never an empty/zero-valued substitute.
    pub fn verify_artifact(&self, logical_name: &str, bytes: &[u8]) -> Result<(), ReceiptError> {
        let artifact = self
            .payload
            .artifacts
            .iter()
            .find(|artifact| artifact.logical_name == logical_name)
            .ok_or_else(|| ReceiptError::MissingArtifact {
                logical_name: logical_name.to_string(),
            })?;
        artifact.verify_bytes(bytes)
    }
}

impl ReceiptPayload {
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, ReceiptError> {
        serde_json::to_vec(self).map_err(|_| ReceiptError::SerializationFailure)
    }

    pub fn digest(&self) -> Result<String, ReceiptError> {
        self.validate()?;
        Ok(sha256_hex(&self.canonical_bytes()?))
    }

    pub fn validate(&self) -> Result<(), ReceiptError> {
        if self.schema_version != RECEIPT_SCHEMA_VERSION {
            return Err(ReceiptError::UnsupportedSchema {
                version: self.schema_version,
            });
        }
        for (field, value) in [
            ("benchmark_id", self.benchmark_id.as_str()),
            ("benchmark_revision", self.benchmark_revision.as_str()),
            ("scenario_id", self.scenario_id.as_str()),
            ("scenario_revision", self.scenario_revision.as_str()),
            ("source.repository", self.source.repository.as_str()),
            ("source.toolchain", self.source.toolchain.as_str()),
            ("source.target_triple", self.source.target_triple.as_str()),
            ("scenario.units", self.scenario.units.as_str()),
            ("scenario.reference_frame", self.scenario.reference_frame.as_str()),
            ("oracle.name", self.oracle.name.as_str()),
            ("oracle.version", self.oracle.version.as_str()),
            ("oracle.method", self.oracle.method.as_str()),
            ("oracle.independence_scope", self.oracle.independence_scope.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(ReceiptError::MissingField { field });
            }
        }
        if !is_git_sha(&self.source.commit_sha) {
            return Err(ReceiptError::InvalidDigest {
                field: "source.commit_sha",
            });
        }
        if !self.source.working_tree_clean {
            return Err(ReceiptError::DirtyWorkingTree);
        }
        for (field, digest) in [
            ("source.cargo_lock_sha256", self.source.cargo_lock_sha256.as_str()),
            ("scenario.input_sha256", self.scenario.input_sha256.as_str()),
            (
                "scenario.configuration_sha256",
                self.scenario.configuration_sha256.as_str(),
            ),
            ("oracle.digest_sha256", self.oracle.digest_sha256.as_str()),
        ] {
            if !is_sha256(digest) {
                return Err(ReceiptError::InvalidDigest { field });
            }
        }
        if self.scenario.validity_envelope.is_empty()
            || self.scenario.validity_envelope.iter().any(|item| item.trim().is_empty())
        {
            return Err(ReceiptError::MissingField {
                field: "scenario.validity_envelope",
            });
        }
        if self.execution.command.iter().any(|arg| arg.contains('\0')) {
            return Err(ReceiptError::InvalidCommand);
        }
        if !matches!(
            self.execution.status,
            ExecutionStatus::NotStarted | ExecutionStatus::Unavailable | ExecutionStatus::Skipped
        ) && self.execution.command.is_empty()
        {
            return Err(ReceiptError::MissingField {
                field: "execution.command",
            });
        }
        if self.execution.cases.passed
            .saturating_add(self.execution.cases.failed)
            .saturating_add(self.execution.cases.unresolved)
            > self.execution.cases.attempted
        {
            return Err(ReceiptError::InconsistentCaseCounts);
        }

        match self.execution.status {
            ExecutionStatus::NotStarted | ExecutionStatus::Unavailable | ExecutionStatus::Skipped => {
                if self.execution.exit_code.is_some() {
                    return Err(ReceiptError::InconsistentExecutionStatus);
                }
            }
            ExecutionStatus::Completed => {
                if self.execution.exit_code.is_none() {
                    return Err(ReceiptError::InconsistentExecutionStatus);
                }
                if self.execution.cases.passed
                    + self.execution.cases.failed
                    + self.execution.cases.unresolved
                    != self.execution.cases.attempted
                {
                    return Err(ReceiptError::InconsistentCaseCounts);
                }
            }
            ExecutionStatus::Failed => {
                if self.execution.exit_code.is_none_or(|code| code == 0) {
                    return Err(ReceiptError::InconsistentExecutionStatus);
                }
            }
            ExecutionStatus::TimedOut => {
                if self.execution.exit_code.is_some() {
                    return Err(ReceiptError::InconsistentExecutionStatus);
                }
            }
        }

        let mut logical_names = std::collections::BTreeSet::new();
        for artifact in &self.artifacts {
            if artifact.logical_name.trim().is_empty() || artifact.media_type.trim().is_empty() {
                return Err(ReceiptError::InvalidArtifactReference);
            }
            if !is_sha256(&artifact.sha256) {
                return Err(ReceiptError::InvalidDigest {
                    field: "artifact.sha256",
                });
            }
            if !logical_names.insert(artifact.logical_name.as_str()) {
                return Err(ReceiptError::DuplicateArtifact {
                    logical_name: artifact.logical_name.clone(),
                });
            }
        }

        for metric in &self.metrics {
            if metric.name.trim().is_empty() || metric.unit.trim().is_empty() {
                return Err(ReceiptError::InvalidMetric);
            }
            if let Some(value) = metric.value {
                if !value.is_finite() {
                    return Err(ReceiptError::InvalidMetric);
                }
            } else if metric.reason.as_ref().is_none_or(|reason| reason.trim().is_empty()) {
                return Err(ReceiptError::InvalidMetric);
            }
        }

        match self.disposition {
            Disposition::Pass => self.validate_pass(),
            Disposition::Fail => self.validate_fail(),
            Disposition::Indeterminate => self.validate_indeterminate(),
            Disposition::NotRun => self.validate_not_run(),
            Disposition::Skipped => self.validate_skipped(),
            Disposition::InvalidReceipt => {
                if self
                    .execution
                    .outcome_reason
                    .as_ref()
                    .is_none_or(|reason| reason.trim().is_empty())
                {
                    return Err(ReceiptError::MissingOutcomeReason);
                }
                Ok(())
            }
        }
    }

    fn validate_pass(&self) -> Result<(), ReceiptError> {
        if self.execution.status != ExecutionStatus::Completed || self.execution.exit_code != Some(0) {
            return Err(ReceiptError::DispositionContradictsExecution);
        }
        if self.execution.cases.attempted == 0
            || self.execution.cases.passed != self.execution.cases.attempted
            || self.execution.cases.failed != 0
            || self.execution.cases.unresolved != 0
        {
            return Err(ReceiptError::DispositionContradictsCaseCounts);
        }
        if self.oracle.independence != OracleIndependence::Independent {
            return Err(ReceiptError::OracleNotIndependent);
        }
        let required = [
            ArtifactKind::ScenarioInput,
            ArtifactKind::Configuration,
            ArtifactKind::OracleReference,
            ArtifactKind::Stdout,
            ArtifactKind::Stderr,
            ArtifactKind::Result,
        ];
        for kind in required {
            if !self.artifacts.iter().any(|artifact| artifact.kind == kind) {
                return Err(ReceiptError::MissingArtifactKind { kind });
            }
        }
        self.require_artifact_digest(ArtifactKind::ScenarioInput, &self.scenario.input_sha256)?;
        self.require_artifact_digest(
            ArtifactKind::Configuration,
            &self.scenario.configuration_sha256,
        )?;
        self.require_artifact_digest(ArtifactKind::OracleReference, &self.oracle.digest_sha256)?;
        if self.execution.outcome_reason.as_ref().is_some_and(|reason| !reason.trim().is_empty()) {
            return Err(ReceiptError::PassHasOutcomeReason);
        }
        Ok(())
    }

    fn validate_fail(&self) -> Result<(), ReceiptError> {
        let has_definitive_failure = self.execution.cases.failed > 0
            || self.execution.exit_code.is_some_and(|code| code != 0)
            || self.execution.status == ExecutionStatus::Failed;
        if !has_definitive_failure || self.execution.status == ExecutionStatus::NotStarted {
            return Err(ReceiptError::DispositionContradictsExecution);
        }
        Ok(())
    }

    fn validate_indeterminate(&self) -> Result<(), ReceiptError> {
        let has_unresolved_evidence = self.execution.cases.unresolved > 0
            || matches!(
                self.execution.status,
                ExecutionStatus::TimedOut | ExecutionStatus::Unavailable
            );
        if !has_unresolved_evidence {
            return Err(ReceiptError::DispositionContradictsExecution);
        }
        self.require_outcome_reason()
    }

    fn validate_not_run(&self) -> Result<(), ReceiptError> {
        if self.execution.status != ExecutionStatus::NotStarted
            || self.execution.exit_code.is_some()
            || self.execution.cases.attempted != 0
            || self.execution.cases.passed != 0
            || self.execution.cases.failed != 0
            || self.execution.cases.unresolved != 0
        {
            return Err(ReceiptError::DispositionContradictsExecution);
        }
        self.require_outcome_reason()
    }

    fn validate_skipped(&self) -> Result<(), ReceiptError> {
        if self.execution.status != ExecutionStatus::Skipped
            || self.execution.exit_code.is_some()
            || self.execution.cases.attempted != 0
        {
            return Err(ReceiptError::DispositionContradictsExecution);
        }
        self.require_outcome_reason()
    }

    fn require_outcome_reason(&self) -> Result<(), ReceiptError> {
        if self
            .execution
            .outcome_reason
            .as_ref()
            .is_none_or(|reason| reason.trim().is_empty())
        {
            return Err(ReceiptError::MissingOutcomeReason);
        }
        Ok(())
    }

    fn require_artifact_digest(
        &self,
        kind: ArtifactKind,
        expected: &str,
    ) -> Result<(), ReceiptError> {
        let artifact = self
            .artifacts
            .iter()
            .find(|artifact| artifact.kind == kind)
            .ok_or(ReceiptError::MissingArtifactKind { kind })?;
        if artifact.sha256 != expected {
            return Err(ReceiptError::ArtifactIdentityMismatch { kind });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReceiptError {
    UnsupportedSchema { version: u32 },
    MissingField { field: &'static str },
    InvalidDigest { field: &'static str },
    DirtyWorkingTree,
    InvalidCommand,
    InconsistentExecutionStatus,
    InconsistentCaseCounts,
    InvalidArtifactReference,
    DuplicateArtifact { logical_name: String },
    InvalidMetric,
    DispositionContradictsExecution,
    DispositionContradictsCaseCounts,
    OracleNotIndependent,
    MissingArtifactKind { kind: ArtifactKind },
    MissingArtifact { logical_name: String },
    ArtifactIdentityMismatch { kind: ArtifactKind },
    ArtifactDigestMismatch {
        logical_name: String,
        expected: String,
        actual: String,
    },
    ArtifactSizeMismatch {
        logical_name: String,
        expected: u64,
        actual: u64,
    },
    ReceiptDigestMismatch { expected: String, actual: String },
    MissingOutcomeReason,
    PassHasOutcomeReason,
    SerializationFailure,
}

impl fmt::Display for ReceiptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchema { version } => write!(f, "unsupported receipt schema {version}"),
            Self::MissingField { field } => write!(f, "missing required receipt field {field}"),
            Self::InvalidDigest { field } => write!(f, "invalid digest in {field}"),
            Self::DirtyWorkingTree => write!(f, "qualification receipt requires a clean source tree"),
            Self::InvalidCommand => write!(f, "execution command contains a NUL byte"),
            Self::InconsistentExecutionStatus => write!(f, "execution status and exit code are inconsistent"),
            Self::InconsistentCaseCounts => write!(f, "case counts are inconsistent"),
            Self::InvalidArtifactReference => write!(f, "artifact reference has an empty name or media type"),
            Self::DuplicateArtifact { logical_name } => write!(f, "duplicate artifact logical name {logical_name}"),
            Self::InvalidMetric => write!(f, "metric is non-finite or missing its unit/name/reason"),
            Self::DispositionContradictsExecution => write!(f, "disposition contradicts execution outcome"),
            Self::DispositionContradictsCaseCounts => write!(f, "Pass requires every attempted case to pass"),
            Self::OracleNotIndependent => write!(f, "Pass requires an explicitly independent oracle"),
            Self::MissingArtifactKind { kind } => write!(f, "missing required artifact kind {kind:?}"),
            Self::MissingArtifact { logical_name } => write!(f, "missing artifact {logical_name}"),
            Self::ArtifactIdentityMismatch { kind } => write!(f, "artifact digest does not match declared identity for {kind:?}"),
            Self::ArtifactDigestMismatch { logical_name, .. } => write!(f, "artifact bytes do not match declared digest for {logical_name}"),
            Self::ArtifactSizeMismatch { logical_name, .. } => write!(f, "artifact byte length does not match declared size for {logical_name}"),
            Self::ReceiptDigestMismatch { .. } => write!(f, "receipt payload digest mismatch"),
            Self::MissingOutcomeReason => write!(f, "this disposition requires an explicit outcome reason"),
            Self::PassHasOutcomeReason => write!(f, "Pass cannot carry a failure/skip/unresolved reason"),
            Self::SerializationFailure => write!(f, "failed to serialize canonical receipt payload"),
        }
    }
}

impl Error for ReceiptError {}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_git_sha(value: &str) -> bool {
    (value.len() == 40 || value.len() == 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::{
        ArtifactKind, ArtifactRef, CaseCounts, Disposition, ExecutionRecord, ExecutionStatus,
        MetricClass, MetricRecord, OracleIdentity, OracleIndependence, QualificationReceipt,
        ReceiptError, ReceiptPayload, ScenarioIdentity, SourceIdentity, VolatileMetadata,
        RECEIPT_SCHEMA_VERSION, sha256_hex,
    };
    use std::collections::BTreeMap;

    fn artifact(kind: ArtifactKind, logical_name: &str, bytes: &[u8]) -> ArtifactRef {
        ArtifactRef {
            kind,
            logical_name: logical_name.to_string(),
            sha256: sha256_hex(bytes),
            media_type: "application/octet-stream".to_string(),
            size_bytes: bytes.len() as u64,
        }
    }

    fn valid_payload() -> ReceiptPayload {
        let input = b"frozen-scenario-v1";
        let config = b"frozen-config-v1";
        let oracle = b"independent-oracle-v1";
        let stdout = b"qualification run stdout";
        let stderr = b"";
        let result = b"{\"passed\":3,\"failed\":0}";

        ReceiptPayload {
            schema_version: RECEIPT_SCHEMA_VERSION,
            benchmark_id: "periodic.wrap.v1".to_string(),
            benchmark_revision: "corpus-1".to_string(),
            scenario_id: "wrap-fixed-seed".to_string(),
            scenario_revision: "1".to_string(),
            source: SourceIdentity {
                repository: "Luminous-Dynamics/symtropy".to_string(),
                commit_sha: "0123456789abcdef0123456789abcdef01234567".to_string(),
                working_tree_clean: true,
                cargo_lock_sha256: sha256_hex(b"lock-v1"),
                toolchain: "rustc 1.96.0".to_string(),
                target_triple: "x86_64-unknown-linux-gnu".to_string(),
                feature_flags: vec!["default".to_string()],
            },
            scenario: ScenarioIdentity {
                input_sha256: sha256_hex(input),
                configuration_sha256: sha256_hex(config),
                units: "m,s".to_string(),
                reference_frame: "orthogonal-R2-periodic-x".to_string(),
                validity_envelope: vec![
                    "D=2".to_string(),
                    "x-period=10".to_string(),
                    "analytic sphere CCD".to_string(),
                ],
            },
            oracle: OracleIdentity {
                name: "direct-shift-oracle".to_string(),
                version: "1".to_string(),
                digest_sha256: sha256_hex(oracle),
                method: "enumerate translated Euclidean images".to_string(),
                independence: OracleIndependence::Independent,
                independence_scope: "does not call production image-range helper".to_string(),
            },
            execution: ExecutionRecord {
                status: ExecutionStatus::Completed,
                command: vec![
                    "cargo".to_string(),
                    "test".to_string(),
                    "-p".to_string(),
                    "symtropy-physics".to_string(),
                ],
                exit_code: Some(0),
                duration_millis: Some(1234),
                cases: CaseCounts {
                    attempted: 3,
                    passed: 3,
                    failed: 0,
                    unresolved: 0,
                },
                outcome_reason: None,
            },
            disposition: Disposition::Pass,
            metrics: vec![MetricRecord {
                name: "minimum_image_error".to_string(),
                value: Some(0.0),
                unit: "m".to_string(),
                class: MetricClass::HardGate,
                reason: None,
            }],
            artifacts: vec![
                artifact(ArtifactKind::ScenarioInput, "scenario.bin", input),
                artifact(ArtifactKind::Configuration, "config.bin", config),
                artifact(ArtifactKind::OracleReference, "oracle.bin", oracle),
                artifact(ArtifactKind::Stdout, "stdout.bin", stdout),
                artifact(ArtifactKind::Stderr, "stderr.bin", stderr),
                artifact(ArtifactKind::Result, "result.json", result),
            ],
            protocol_metadata: BTreeMap::new(),
        }
    }

    fn volatile(label: &str) -> VolatileMetadata {
        VolatileMetadata {
            recorded_at_utc: Some("2026-10-29T00:00:00Z".to_string()),
            worker_label: Some(label.to_string()),
        }
    }

    #[test]
    fn valid_pass_receipt_has_stable_payload_digest() {
        let payload = valid_payload();
        let receipt_a = QualificationReceipt::new(payload.clone(), volatile("runner-a")).unwrap();
        let mut volatile_b = volatile("runner-b");
        volatile_b.recorded_at_utc = Some("2026-10-30T00:00:00Z".to_string());
        let receipt_b = QualificationReceipt::new(payload, volatile_b).unwrap();

        assert_eq!(receipt_a.receipt_sha256, receipt_b.receipt_sha256);
        receipt_a.verify().unwrap();
        receipt_b.verify().unwrap();
    }

    #[test]
    fn changed_artifact_bytes_fail_digest_verification() {
        let receipt = QualificationReceipt::new(valid_payload(), volatile("runner")).unwrap();
        assert!(matches!(
            receipt.verify_artifact("result.json", b"{\"passed\":3,\"failed\":1}"),
            Err(ReceiptError::ArtifactDigestMismatch { .. })
        ));
    }

    #[test]
    fn missing_required_raw_output_blocks_pass() {
        let mut payload = valid_payload();
        payload.artifacts.retain(|artifact| artifact.kind != ArtifactKind::Stderr);
        assert_eq!(
            QualificationReceipt::new(payload, volatile("runner")).unwrap_err(),
            ReceiptError::MissingArtifactKind { kind: ArtifactKind::Stderr }
        );
    }

    #[test]
    fn not_run_cannot_be_labeled_pass() {
        let mut payload = valid_payload();
        payload.disposition = Disposition::Pass;
        payload.execution.status = ExecutionStatus::NotStarted;
        payload.execution.command.clear();
        payload.execution.exit_code = None;
        payload.execution.duration_millis = None;
        payload.execution.cases = CaseCounts::default();
        payload.execution.outcome_reason = Some("runner unavailable".to_string());
        assert_eq!(
            QualificationReceipt::new(payload, volatile("runner")).unwrap_err(),
            ReceiptError::DispositionContradictsExecution
        );
    }

    #[test]
    fn skipped_and_timeout_are_not_passes() {
        let mut skipped = valid_payload();
        skipped.disposition = Disposition::Skipped;
        skipped.execution.status = ExecutionStatus::Skipped;
        skipped.execution.command.clear();
        skipped.execution.exit_code = None;
        skipped.execution.duration_millis = None;
        skipped.execution.cases = CaseCounts::default();
        skipped.execution.outcome_reason = Some("not selected in this matrix".to_string());
        QualificationReceipt::new(skipped, volatile("runner")).unwrap();

        let mut timeout = valid_payload();
        timeout.disposition = Disposition::Indeterminate;
        timeout.execution.status = ExecutionStatus::TimedOut;
        timeout.execution.exit_code = None;
        timeout.execution.outcome_reason = Some("hard timeout reached".to_string());
        timeout.execution.cases = CaseCounts {
            attempted: 3,
            passed: 2,
            failed: 0,
            unresolved: 1,
        };
        QualificationReceipt::new(timeout, volatile("runner")).unwrap();
    }

    #[test]
    fn pass_requires_independent_oracle_and_complete_case_accounting() {
        let mut payload = valid_payload();
        payload.oracle.independence = OracleIndependence::SharedProductionHelpers;
        assert_eq!(
            QualificationReceipt::new(payload, volatile("runner")).unwrap_err(),
            ReceiptError::OracleNotIndependent
        );

        let mut payload = valid_payload();
        payload.execution.cases.failed = 1;
        payload.execution.cases.passed = 2;
        payload.disposition = Disposition::Pass;
        assert_eq!(
            QualificationReceipt::new(payload, volatile("runner")).unwrap_err(),
            ReceiptError::DispositionContradictsCaseCounts
        );
    }

    #[test]
    fn reordered_metadata_keys_have_identical_digest() {
        let mut first = valid_payload();
        first.protocol_metadata.insert("b".to_string(), "2".to_string());
        first.protocol_metadata.insert("a".to_string(), "1".to_string());

        let mut second = valid_payload();
        second.protocol_metadata.insert("a".to_string(), "1".to_string());
        second.protocol_metadata.insert("b".to_string(), "2".to_string());

        let first = QualificationReceipt::new(first, volatile("one")).unwrap();
        let second = QualificationReceipt::new(second, volatile("two")).unwrap();
        assert_eq!(first.receipt_sha256, second.receipt_sha256);
    }

    #[test]
    fn tampering_with_payload_is_detected() {
        let mut receipt = QualificationReceipt::new(valid_payload(), volatile("runner")).unwrap();
        receipt.payload.benchmark_revision = "corrupted".to_string();
        assert!(matches!(
            receipt.verify(),
            Err(ReceiptError::ReceiptDigestMismatch { .. })
                | Err(ReceiptError::DispositionContradictsExecution)
        ));
    }

    #[test]
    fn non_finite_metrics_are_rejected_before_serialization() {
        let mut payload = valid_payload();
        payload.metrics[0].value = Some(f64::NAN);
        assert_eq!(
            QualificationReceipt::new(payload, volatile("runner")).unwrap_err(),
            ReceiptError::InvalidMetric
        );
    }
}
