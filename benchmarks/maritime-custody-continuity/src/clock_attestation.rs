// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: MIT

//! Result-centred clock calibration evidence attestation.
//!
//! This module validates an evidence contract around an already-parsed
//! measurement result and a separately verified artifact receipt. It does
//! not implement cryptography, parse certificate bytes, or claim that a
//! signature proves physical clock accuracy. The signature/certificate
//! decision is supplied by a separately audited verifier whose exact
//! implementation, trust store and policy versions are pinned in the receipt.
//!
//! The module checks whether the authenticated claims apply to the expected
//! clock, firmware, measurand, time basis, event time and declared traceability
//! chain. Uncertainty contributions are combined by conservative arithmetic
//! addition; no independence assumption is made.

use std::collections::BTreeSet;

use crate::temporal_order::{
    ClockQualificationRef, ClockSyncState, TemporalOrderError, TimeEvidence, TimeInterval,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CalibrationArtifactReference {
    pub artifact_id: String,
    /// Lowercase SHA-256 digest of the exact immutable artifact bytes.
    pub sha256: String,
    pub media_type: String,
    pub schema_profile_id: String,
    pub schema_profile_version: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VerifierStatus {
    Valid,
    Invalid,
    Unknown,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RevocationStatus {
    Good,
    Revoked,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactVerifierReceipt {
    /// Digest of the exact bytes whose signature and certificate chain were checked.
    pub artifact_sha256: String,
    pub signer_fingerprint: String,
    pub trust_anchor_id: String,
    pub signature_algorithm: String,
    pub signature_time_source_id: String,
    pub signature_time_utc_seconds: i64,
    pub signer_certificate_not_before_utc_seconds: i64,
    pub signer_certificate_not_after_utc_seconds: i64,
    pub key_usage: BTreeSet<String>,
    pub signature_status: VerifierStatus,
    pub certificate_chain_status: VerifierStatus,
    pub revocation_status: RevocationStatus,
    /// Historical time for which the verifier evaluated revocation state.
    pub revocation_evaluated_at_utc_seconds: i64,
    /// End of the evidence freshness interval supplied by the revocation source.
    pub revocation_evidence_valid_until_utc_seconds: i64,
    pub verifier_id: String,
    pub verifier_version: String,
    pub trust_store_id: String,
    pub trust_store_version: String,
    pub policy_id: String,
    pub policy_version: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CalibrationArtifactEvidence {
    pub artifact: CalibrationArtifactReference,
    /// Output of an external, pinned verifier. This crate validates its
    /// bindings and policy identity; it does not independently verify crypto.
    pub verifier: ArtifactVerifierReceipt,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockAttestationPolicy {
    pub policy_id: String,
    pub policy_version: String,
    pub verifier_id: String,
    pub verifier_version: String,
    pub trust_store_id: String,
    pub trust_store_version: String,
    pub trusted_anchor_ids: BTreeSet<String>,
    pub allowed_signature_algorithms: BTreeSet<String>,
    pub allowed_uncertainty_model_ids: BTreeSet<String>,
    pub allowed_synchronization_methods: BTreeSet<String>,
    pub required_key_usage: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectedClockScope {
    pub clock_id: String,
    pub clock_profile_id: String,
    pub firmware_id: String,
    pub measurand_id: String,
    pub time_basis_id: String,
    /// Unit of every timestamp and uncertainty tick, e.g. SI nanosecond.
    pub tick_quantum_id: String,
    pub terminal_reference_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CalibrationMeasurementResult {
    pub result_id: String,
    pub clock_id: String,
    pub clock_profile_id: String,
    pub firmware_id: String,
    pub measurand_id: String,
    pub time_basis_id: String,
    pub tick_quantum_id: String,
    /// First standard in the traceability chain used to calibrate this clock.
    pub reference_standard_id: String,
    /// Declared terminal reference, e.g. a national time/frequency reference.
    pub terminal_reference_id: String,
    pub calibrated_at_ticks: i64,
    pub valid_from_ticks: i64,
    pub valid_until_ticks: i64,
    /// Signed clock-minus-reference offset. Positive means the device clock
    /// reads ahead; normalized reference time = reported device time - offset.
    pub measured_offset_ticks: i64,
    /// Local comparison uncertainty excluding the separately listed
    /// traceability-link contributions. None means the bound is unknown.
    pub measurement_uncertainty_bound_ticks: Option<u64>,
    pub coverage_factor_lexeme: String,
    pub uncertainty_model_id: String,
    pub artifact_evidence: CalibrationArtifactEvidence,
    pub traceability_links: Vec<TraceabilityLink>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceabilityLink {
    pub link_id: String,
    /// The standard calibrated by this link.
    pub calibrated_subject_id: String,
    /// The reference standard against which the subject was compared.
    pub reference_standard_id: String,
    pub measurement_result_id: String,
    pub measurand_id: String,
    pub time_basis_id: String,
    pub tick_quantum_id: String,
    pub measured_at_ticks: i64,
    pub valid_from_ticks: i64,
    pub valid_until_ticks: i64,
    /// A conservative upper bound for this link's non-overlapping contribution.
    pub uncertainty_bound_ticks: Option<u64>,
    pub uncertainty_model_id: String,
    pub artifact_evidence: CalibrationArtifactEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockUseObservation {
    pub observation_id: String,
    /// Exact source-event namespace for the event whose timestamp is qualified.
    pub source_id: String,
    /// Exact source-event revision whose timestamp is being qualified.
    pub source_event_id: String,
    pub source_event_revision: u32,
    pub clock_id: String,
    pub clock_profile_id: String,
    pub firmware_id: String,
    pub time_basis_id: String,
    pub tick_quantum_id: String,
    pub event_time_ticks: i64,
    /// Time at which the in-service state and error components were observed.
    pub observation_time_ticks: i64,
    pub maximum_observation_age_ticks: Option<u64>,
    pub last_sync_ticks: i64,
    pub synchronization_method: String,
    pub sync_state: ClockSyncState,
    /// Maximum age permitted after the last successful comparison while locked.
    pub maximum_sync_age_ticks: Option<u64>,
    /// Maximum time permitted in holdover after the last successful comparison.
    pub maximum_holdover_duration_ticks: Option<u64>,
    pub maximum_offset_error_ticks: Option<u64>,
    pub holdover_drift_error_ticks: Option<u64>,
    pub capture_path_error_ticks: Option<u64>,
    pub rounding_error_ticks: Option<u64>,
    pub artifact_evidence: CalibrationArtifactEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClockAttestationReason {
    MissingRequiredField(&'static str),
    InvalidArtifactDigest,
    ArtifactDigestMismatch,
    VerifierPolicyMismatch,
    UntrustedTrustAnchor,
    DisallowedSignatureAlgorithm,
    SignerCertificateOutsideValidity,
    MissingDocumentSigningKeyUsage,
    DisallowedUncertaintyModel,
    DisallowedSynchronizationMethod,
    InvalidCoverageFactor,
    InvalidSignature,
    InvalidCertificateChain,
    RevokedArtifact,
    RevocationStatusUnknown,
    RevocationEvidenceStale,
    UnsupportedVerifierResult,
    ClockIdentityMismatch,
    ClockProfileMismatch,
    FirmwareMismatch,
    MeasurandMismatch,
    TimeBasisMismatch,
    TimeQuantumMismatch,
    OffsetCorrectionOverflow,
    TerminalReferenceMismatch,
    CalibrationOutsideValidity,
    EmptyTraceabilityChain,
    BrokenTraceabilityChain,
    TraceabilityScopeMismatch,
    TraceabilityOutsideValidity,
    MissingUncertaintyBound(&'static str),
    UncertaintyOverflow,
    ClockAlarmed,
    ClockSyncStateUnknown,
    SyncEvidenceAfterEvent,
    ClockObservationAfterEvent,
    ClockObservationTooOld,
    UncertaintyIntervalOutsideValidity,
    LockedClockSyncTooOld,
    HoldoverLimitExceeded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockAttestationStatus {
    AcceptedForDeclaredScope,
    Unknown,
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UncertaintyContribution {
    pub component_id: String,
    pub upper_bound_ticks: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QualifiedClockCalibrationReceipt {
    result_id: String,
    observation_id: String,
    source_id: String,
    source_event_id: String,
    source_event_revision: u32,
    clock_id: String,
    clock_profile_id: String,
    firmware_id: String,
    measurand_id: String,
    time_basis_id: String,
    tick_quantum_id: String,
    terminal_reference_id: String,
    /// Offset-corrected event timestamp in the declared reference time basis.
    event_time_ticks: i64,
    /// Original clock-reported timestamp before correction.
    reported_event_time_ticks: i64,
    /// Original device-reported timestamp before signed offset correction.
    reported_event_time_ticks: i64,
    observation_time_ticks: i64,
    synchronization_method: String,
    sync_state: ClockSyncState,
    applicable_from_ticks: i64,
    applicable_until_ticks: i64,
    measured_offset_ticks: i64,
    uncertainty_contributions: Vec<UncertaintyContribution>,
    maximum_total_error_ticks: u64,
    calibration_artifact_digests: Vec<String>,
    verifier_id: String,
    verifier_version: String,
    trust_store_id: String,
    trust_store_version: String,
    policy_id: String,
    policy_version: String,
    decision_as_of_utc_seconds: i64,
}

impl QualifiedClockCalibrationReceipt {
    pub fn result_id(&self) -> &str {
        &self.result_id
    }
    pub fn observation_id(&self) -> &str {
        &self.observation_id
    }
    pub fn source_id(&self) -> &str {
        &self.source_id
    }
    pub fn source_event_id(&self) -> &str {
        &self.source_event_id
    }
    pub fn source_event_revision(&self) -> u32 {
        self.source_event_revision
    }
    pub fn clock_id(&self) -> &str {
        &self.clock_id
    }
    pub fn clock_profile_id(&self) -> &str {
        &self.clock_profile_id
    }
    pub fn firmware_id(&self) -> &str {
        &self.firmware_id
    }
    pub fn measurand_id(&self) -> &str {
        &self.measurand_id
    }
    pub fn time_basis_id(&self) -> &str {
        &self.time_basis_id
    }
    pub fn tick_quantum_id(&self) -> &str {
        &self.tick_quantum_id
    }
    pub fn terminal_reference_id(&self) -> &str {
        &self.terminal_reference_id
    }
    pub fn event_time_ticks(&self) -> i64 {
        self.event_time_ticks
    }
    pub fn reported_event_time_ticks(&self) -> i64 {
        self.reported_event_time_ticks
    }
    pub fn observation_time_ticks(&self) -> i64 {
        self.observation_time_ticks
    }
    pub fn synchronization_method(&self) -> &str {
        &self.synchronization_method
    }
    pub fn applicable_from_ticks(&self) -> i64 {
        self.applicable_from_ticks
    }
    pub fn applicable_until_ticks(&self) -> i64 {
        self.applicable_until_ticks
    }
    pub fn measured_offset_ticks(&self) -> i64 {
        self.measured_offset_ticks
    }
    pub fn uncertainty_contributions(&self) -> &[UncertaintyContribution] {
        &self.uncertainty_contributions
    }
    pub fn maximum_total_error_ticks(&self) -> u64 {
        self.maximum_total_error_ticks
    }
    pub fn calibration_artifact_digests(&self) -> &[String] {
        &self.calibration_artifact_digests
    }
    pub fn verifier_id(&self) -> &str {
        &self.verifier_id
    }
    pub fn verifier_version(&self) -> &str {
        &self.verifier_version
    }
    pub fn trust_store_id(&self) -> &str {
        &self.trust_store_id
    }
    pub fn trust_store_version(&self) -> &str {
        &self.trust_store_version
    }
    pub fn policy_id(&self) -> &str {
        &self.policy_id
    }
    pub fn policy_version(&self) -> &str {
        &self.policy_version
    }
    pub fn decision_as_of_utc_seconds(&self) -> i64 {
        self.decision_as_of_utc_seconds
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockAttestationDecision {
    pub status: ClockAttestationStatus,
    pub reason: Option<ClockAttestationReason>,
    pub receipt: Option<QualifiedClockCalibrationReceipt>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClockEvidenceConversionError {
    DecisionNotAccepted,
    MissingAcceptedReceipt,
    MissingCalibrationDigest,
    IntervalOverflow,
    IntervalOutsideValidity,
    InvalidInterval(TemporalOrderError),
    InvalidEvidence(TemporalOrderError),
}

impl ClockAttestationDecision {
    /// Convert only an accepted, opaque clock-attestation receipt into
    /// proof-grade custody event-time evidence. The source-event key and
    /// revision are bound into the exact evidence ID used by the custody
    /// timeline evaluator.
    pub fn to_time_evidence(&self) -> Result<TimeEvidence, ClockEvidenceConversionError> {
        if self.status != ClockAttestationStatus::AcceptedForDeclaredScope {
            return Err(ClockEvidenceConversionError::DecisionNotAccepted);
        }
        let receipt = self
            .receipt
            .as_ref()
            .ok_or(ClockEvidenceConversionError::MissingAcceptedReceipt)?;
        let radius = i64::try_from(receipt.maximum_total_error_ticks)
            .map_err(|_| ClockEvidenceConversionError::IntervalOverflow)?;
        let earliest = receipt
            .event_time_ticks
            .checked_sub(radius)
            .ok_or(ClockEvidenceConversionError::IntervalOverflow)?;
        let latest = receipt
            .event_time_ticks
            .checked_add(radius)
            .ok_or(ClockEvidenceConversionError::IntervalOverflow)?;
        if earliest < receipt.applicable_from_ticks || latest > receipt.applicable_until_ticks {
            return Err(ClockEvidenceConversionError::IntervalOutsideValidity);
        }
        let interval = TimeInterval::new(earliest, latest)
            .map_err(ClockEvidenceConversionError::InvalidInterval)?;
        let calibration_digest = receipt
            .calibration_artifact_digests
            .first()
            .ok_or(ClockEvidenceConversionError::MissingCalibrationDigest)?;
        let event_id = format!(
            "{}:{}:{}:{}:{}",
            receipt.source_id.len(),
            receipt.source_id,
            receipt.source_event_id.len(),
            receipt.source_event_id,
            receipt.source_event_revision
        );
        let qualification = ClockQualificationRef {
            clock_id: receipt.clock_id.clone(),
            profile_id: receipt.clock_profile_id.clone(),
            tick_quantum_id: receipt.tick_quantum_id.clone(),
            calibration_evidence_id: calibration_digest.clone(),
            synchronization_method: receipt.synchronization_method.clone(),
            sync_state: receipt.sync_state,
            calibrated_at: receipt.applicable_from_ticks,
            valid_until: receipt.applicable_until_ticks,
            maximum_total_error_ticks: receipt.maximum_total_error_ticks,
        };
        TimeEvidence::from_clock_qualification(
            event_id,
            receipt.time_basis_id.clone(),
            interval,
            qualification,
        )
        .map_err(ClockEvidenceConversionError::InvalidEvidence)
    }

    fn accepted(receipt: QualifiedClockCalibrationReceipt) -> Self {
        Self {
            status: ClockAttestationStatus::AcceptedForDeclaredScope,
            reason: None,
            receipt: Some(receipt),
        }
    }

    fn unknown(reason: ClockAttestationReason) -> Self {
        Self {
            status: ClockAttestationStatus::Unknown,
            reason: Some(reason),
            receipt: None,
        }
    }

    fn rejected(reason: ClockAttestationReason) -> Self {
        Self {
            status: ClockAttestationStatus::Rejected,
            reason: Some(reason),
            receipt: None,
        }
    }
}

enum ArtifactCheck {
    Accepted,
    Unknown(ClockAttestationReason),
    Rejected(ClockAttestationReason),
}

fn valid_text(value: &str) -> bool {
    !value.is_empty() && value.trim() == value
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn check_artifact_evidence(
    evidence: &CalibrationArtifactEvidence,
    policy: &ClockAttestationPolicy,
    decision_as_of_utc_seconds: i64,
) -> ArtifactCheck {
    let artifact = &evidence.artifact;
    let verifier = &evidence.verifier;

    for (value, field) in [
        (&artifact.artifact_id, "artifact_id"),
        (&artifact.media_type, "media_type"),
        (&artifact.schema_profile_id, "schema_profile_id"),
        (&artifact.schema_profile_version, "schema_profile_version"),
        (&verifier.signer_fingerprint, "signer_fingerprint"),
        (&verifier.trust_anchor_id, "trust_anchor_id"),
        (&verifier.signature_algorithm, "signature_algorithm"),
        (
            &verifier.signature_time_source_id,
            "signature_time_source_id",
        ),
        (&verifier.verifier_id, "verifier_id"),
        (&verifier.verifier_version, "verifier_version"),
        (&verifier.trust_store_id, "trust_store_id"),
        (&verifier.trust_store_version, "trust_store_version"),
        (&verifier.policy_id, "policy_id"),
        (&verifier.policy_version, "policy_version"),
    ] {
        if !valid_text(value) {
            return ArtifactCheck::Unknown(ClockAttestationReason::MissingRequiredField(field));
        }
    }

    if !valid_sha256(&artifact.sha256) {
        return ArtifactCheck::Rejected(ClockAttestationReason::InvalidArtifactDigest);
    }
    if verifier.artifact_sha256 != artifact.sha256 {
        return ArtifactCheck::Rejected(ClockAttestationReason::ArtifactDigestMismatch);
    }
    if verifier.verifier_id != policy.verifier_id
        || verifier.verifier_version != policy.verifier_version
        || verifier.trust_store_id != policy.trust_store_id
        || verifier.trust_store_version != policy.trust_store_version
        || verifier.policy_id != policy.policy_id
        || verifier.policy_version != policy.policy_version
    {
        return ArtifactCheck::Rejected(ClockAttestationReason::VerifierPolicyMismatch);
    }
    if !policy
        .trusted_anchor_ids
        .contains(&verifier.trust_anchor_id)
    {
        return ArtifactCheck::Rejected(ClockAttestationReason::UntrustedTrustAnchor);
    }
    if !policy
        .allowed_signature_algorithms
        .contains(&verifier.signature_algorithm)
    {
        return ArtifactCheck::Rejected(ClockAttestationReason::DisallowedSignatureAlgorithm);
    }
    if verifier.signer_certificate_not_before_utc_seconds > verifier.signature_time_utc_seconds
        || verifier.signature_time_utc_seconds > verifier.signer_certificate_not_after_utc_seconds
        || verifier.signer_certificate_not_before_utc_seconds
            > verifier.signer_certificate_not_after_utc_seconds
    {
        return ArtifactCheck::Rejected(ClockAttestationReason::SignerCertificateOutsideValidity);
    }
    if !verifier.key_usage.contains(&policy.required_key_usage) {
        return ArtifactCheck::Rejected(ClockAttestationReason::MissingDocumentSigningKeyUsage);
    }

    match verifier.signature_status {
        VerifierStatus::Valid => {}
        VerifierStatus::Invalid => {
            return ArtifactCheck::Rejected(ClockAttestationReason::InvalidSignature);
        }
        VerifierStatus::Unknown | VerifierStatus::Unsupported => {
            return ArtifactCheck::Unknown(ClockAttestationReason::UnsupportedVerifierResult);
        }
    }
    match verifier.certificate_chain_status {
        VerifierStatus::Valid => {}
        VerifierStatus::Invalid => {
            return ArtifactCheck::Rejected(ClockAttestationReason::InvalidCertificateChain);
        }
        VerifierStatus::Unknown | VerifierStatus::Unsupported => {
            return ArtifactCheck::Unknown(ClockAttestationReason::UnsupportedVerifierResult);
        }
    }
    match verifier.revocation_status {
        RevocationStatus::Good => {}
        RevocationStatus::Revoked => {
            return ArtifactCheck::Rejected(ClockAttestationReason::RevokedArtifact);
        }
        RevocationStatus::Unknown => {
            return ArtifactCheck::Unknown(ClockAttestationReason::RevocationStatusUnknown);
        }
    }
    if verifier.revocation_evaluated_at_utc_seconds > decision_as_of_utc_seconds
        || verifier.revocation_evidence_valid_until_utc_seconds < decision_as_of_utc_seconds
        || verifier.revocation_evaluated_at_utc_seconds
            > verifier.revocation_evidence_valid_until_utc_seconds
    {
        return ArtifactCheck::Unknown(ClockAttestationReason::RevocationEvidenceStale);
    }

    ArtifactCheck::Accepted
}

fn verify_all_artifacts(
    calibration: &CalibrationMeasurementResult,
    observation: &ClockUseObservation,
    policy: &ClockAttestationPolicy,
    decision_as_of_utc_seconds: i64,
) -> Result<Vec<String>, ClockAttestationDecision> {
    let mut digests = Vec::with_capacity(calibration.traceability_links.len() + 2);
    let mut all = Vec::with_capacity(calibration.traceability_links.len() + 2);
    all.push(&calibration.artifact_evidence);
    for link in &calibration.traceability_links {
        all.push(&link.artifact_evidence);
    }
    all.push(&observation.artifact_evidence);

    for evidence in all {
        match check_artifact_evidence(evidence, policy, decision_as_of_utc_seconds) {
            ArtifactCheck::Accepted => digests.push(evidence.artifact.sha256.clone()),
            ArtifactCheck::Unknown(reason) => {
                return Err(ClockAttestationDecision::unknown(reason));
            }
            ArtifactCheck::Rejected(reason) => {
                return Err(ClockAttestationDecision::rejected(reason));
            }
        }
    }
    Ok(digests)
}

/// Qualify one calibration measurement result for one exact clock-use
/// observation. The returned receipt is conditional on the external verifier
/// identified by the pinned policy; this crate does not verify signatures or
/// parse calibration artifacts itself.
pub fn attest_clock_for_event(
    calibration: &CalibrationMeasurementResult,
    observation: &ClockUseObservation,
    expected: &ExpectedClockScope,
    policy: &ClockAttestationPolicy,
    decision_as_of_utc_seconds: i64,
) -> ClockAttestationDecision {
    for (value, field) in [
        (&expected.clock_id, "expected.clock_id"),
        (&expected.clock_profile_id, "expected.clock_profile_id"),
        (&expected.firmware_id, "expected.firmware_id"),
        (&expected.measurand_id, "expected.measurand_id"),
        (&expected.time_basis_id, "expected.time_basis_id"),
        (&expected.tick_quantum_id, "expected.tick_quantum_id"),
        (
            &expected.terminal_reference_id,
            "expected.terminal_reference_id",
        ),
        (&policy.policy_id, "policy.policy_id"),
        (&policy.policy_version, "policy.policy_version"),
        (&policy.verifier_id, "policy.verifier_id"),
        (&policy.verifier_version, "policy.verifier_version"),
        (&policy.trust_store_id, "policy.trust_store_id"),
        (&policy.trust_store_version, "policy.trust_store_version"),
        (&policy.required_key_usage, "policy.required_key_usage"),
    ] {
        if !valid_text(value) {
            return ClockAttestationDecision::unknown(
                ClockAttestationReason::MissingRequiredField(field),
            );
        }
    }

    if calibration.clock_id != expected.clock_id || observation.clock_id != expected.clock_id {
        return ClockAttestationDecision::rejected(ClockAttestationReason::ClockIdentityMismatch);
    }
    if calibration.clock_profile_id != expected.clock_profile_id
        || observation.clock_profile_id != expected.clock_profile_id
    {
        return ClockAttestationDecision::rejected(ClockAttestationReason::ClockProfileMismatch);
    }
    if calibration.firmware_id != expected.firmware_id
        || observation.firmware_id != expected.firmware_id
    {
        return ClockAttestationDecision::rejected(ClockAttestationReason::FirmwareMismatch);
    }
    if calibration.measurand_id != expected.measurand_id {
        return ClockAttestationDecision::rejected(ClockAttestationReason::MeasurandMismatch);
    }
    if calibration.time_basis_id != expected.time_basis_id
        || observation.time_basis_id != expected.time_basis_id
    {
        return ClockAttestationDecision::rejected(ClockAttestationReason::TimeBasisMismatch);
    }
    if calibration.tick_quantum_id != expected.tick_quantum_id
        || observation.tick_quantum_id != expected.tick_quantum_id
        || calibration
            .traceability_links
            .iter()
            .any(|link| link.tick_quantum_id != expected.tick_quantum_id)
    {
        return ClockAttestationDecision::rejected(ClockAttestationReason::TimeQuantumMismatch);
    }
    if calibration.terminal_reference_id != expected.terminal_reference_id {
        return ClockAttestationDecision::rejected(
            ClockAttestationReason::TerminalReferenceMismatch,
        );
    }
    if !policy
        .allowed_uncertainty_model_ids
        .contains(&calibration.uncertainty_model_id)
        || calibration.traceability_links.iter().any(|link| {
            !policy
                .allowed_uncertainty_model_ids
                .contains(&link.uncertainty_model_id)
        })
    {
        return ClockAttestationDecision::rejected(
            ClockAttestationReason::DisallowedUncertaintyModel,
        );
    }
    if !policy
        .allowed_synchronization_methods
        .contains(&observation.synchronization_method)
    {
        return ClockAttestationDecision::rejected(
            ClockAttestationReason::DisallowedSynchronizationMethod,
        );
    }

    if !valid_text(&calibration.result_id)
        || !valid_text(&calibration.reference_standard_id)
        || !valid_text(&calibration.measurand_id)
        || !valid_text(&calibration.time_basis_id)
        || !valid_text(&calibration.coverage_factor_lexeme)
        || !calibration
            .coverage_factor_lexeme
            .parse::<f64>()
            .is_ok_and(|value| value.is_finite() && value > 0.0)
        || !valid_text(&calibration.uncertainty_model_id)
        || !valid_text(&observation.observation_id)
        || !valid_text(&observation.source_id)
        || !valid_text(&observation.source_event_id)
        || !valid_text(&observation.synchronization_method)
    {
        return ClockAttestationDecision::unknown(ClockAttestationReason::MissingRequiredField(
            "measurement provenance",
        ));
    }

    // Synchronization and capture observations are expressed in the device's
    // reported tick domain. First qualify their freshness against the raw
    // device timestamp, then correct that timestamp into the declared
    // reference time basis using the signed clock-minus-reference offset.
    let reported_event_time = observation.event_time_ticks;
    if observation.observation_time_ticks > reported_event_time {
        return ClockAttestationDecision::rejected(
            ClockAttestationReason::ClockObservationAfterEvent,
        );
    }
    let observation_age = match reported_event_time.checked_sub(observation.observation_time_ticks) {
        Some(age) => age,
        None => {
            return ClockAttestationDecision::rejected(
                ClockAttestationReason::ClockObservationAfterEvent,
            );
        }
    };
    let Some(maximum_observation_age) = observation.maximum_observation_age_ticks else {
        return ClockAttestationDecision::unknown(ClockAttestationReason::MissingUncertaintyBound(
            "maximum observation age",
        ));
    };
    if observation_age > i64::try_from(maximum_observation_age).unwrap_or(i64::MAX) {
        return ClockAttestationDecision::unknown(ClockAttestationReason::ClockObservationTooOld);
    }

    let Some(event_time) = reported_event_time.checked_sub(calibration.measured_offset_ticks)
    else {
        return ClockAttestationDecision::rejected(
            ClockAttestationReason::OffsetCorrectionOverflow,
        );
    };

    if calibration.calibrated_at_ticks > event_time
        || calibration.valid_from_ticks > event_time
        || calibration.valid_until_ticks < event_time
        || calibration.valid_from_ticks > calibration.valid_until_ticks
    {
        return ClockAttestationDecision::rejected(
            ClockAttestationReason::CalibrationOutsideValidity,
        );
    }

    if calibration.traceability_links.is_empty() {
        return ClockAttestationDecision::unknown(ClockAttestationReason::EmptyTraceabilityChain);
    }

    if calibration.traceability_links[0].calibrated_subject_id != calibration.reference_standard_id
        || calibration
            .traceability_links
            .last()
            .is_none_or(|link| link.reference_standard_id != calibration.terminal_reference_id)
    {
        return ClockAttestationDecision::rejected(ClockAttestationReason::BrokenTraceabilityChain);
    }

    let mut traceability_uncertainty_ticks = 0_u64;
    for (index, link) in calibration.traceability_links.iter().enumerate() {
        for (value, field) in [
            (&link.link_id, "traceability.link_id"),
            (&link.calibrated_subject_id, "traceability.subject"),
            (&link.reference_standard_id, "traceability.reference"),
            (
                &link.measurement_result_id,
                "traceability.measurement_result_id",
            ),
            (&link.measurand_id, "traceability.measurand_id"),
            (&link.time_basis_id, "traceability.time_basis_id"),
            (
                &link.uncertainty_model_id,
                "traceability.uncertainty_model_id",
            ),
        ] {
            if !valid_text(value) {
                return ClockAttestationDecision::unknown(
                    ClockAttestationReason::MissingRequiredField(field),
                );
            }
        }
        if link.measurand_id != expected.measurand_id
            || link.time_basis_id != expected.time_basis_id
        {
            return ClockAttestationDecision::rejected(
                ClockAttestationReason::TraceabilityScopeMismatch,
            );
        }
        if link.calibrated_at_ticks > event_time
            || link.measured_at_ticks > event_time
            || link.valid_from_ticks > event_time
            || link.valid_until_ticks < event_time
            || link.valid_from_ticks > link.valid_until_ticks
        {
            return ClockAttestationDecision::rejected(
                ClockAttestationReason::TraceabilityOutsideValidity,
            );
        }
        if index > 0
            && calibration.traceability_links[index - 1].reference_standard_id
                != link.calibrated_subject_id
        {
            return ClockAttestationDecision::rejected(
                ClockAttestationReason::BrokenTraceabilityChain,
            );
        }
        let Some(uncertainty) = link.uncertainty_bound_ticks else {
            return ClockAttestationDecision::unknown(
                ClockAttestationReason::MissingUncertaintyBound("traceability link"),
            );
        };
        let Some(total) = traceability_uncertainty_ticks.checked_add(uncertainty) else {
            return ClockAttestationDecision::rejected(ClockAttestationReason::UncertaintyOverflow);
        };
        traceability_uncertainty_ticks = total;
    }

    let Some(measurement_uncertainty_ticks) = calibration.measurement_uncertainty_bound_ticks
    else {
        return ClockAttestationDecision::unknown(ClockAttestationReason::MissingUncertaintyBound(
            "measurement result",
        ));
    };

    if observation.last_sync_ticks > observation.observation_time_ticks
        || observation.last_sync_ticks > reported_event_time
    {
        return ClockAttestationDecision::rejected(ClockAttestationReason::SyncEvidenceAfterEvent);
    }
    let sync_age = match reported_event_time.checked_sub(observation.last_sync_ticks) {
        Some(age) => age,
        None => {
            return ClockAttestationDecision::rejected(
                ClockAttestationReason::SyncEvidenceAfterEvent,
            );
        }
    };

    match observation.sync_state {
        ClockSyncState::Locked => {
            let Some(maximum_age) = observation.maximum_sync_age_ticks else {
                return ClockAttestationDecision::unknown(
                    ClockAttestationReason::MissingUncertaintyBound("maximum sync age"),
                );
            };
            if sync_age > i64::try_from(maximum_age).unwrap_or(i64::MAX) {
                return ClockAttestationDecision::unknown(
                    ClockAttestationReason::LockedClockSyncTooOld,
                );
            }
        }
        ClockSyncState::Holdover => {
            let Some(maximum_duration) = observation.maximum_holdover_duration_ticks else {
                return ClockAttestationDecision::unknown(
                    ClockAttestationReason::MissingUncertaintyBound("maximum holdover duration"),
                );
            };
            if sync_age > i64::try_from(maximum_duration).unwrap_or(i64::MAX) {
                return ClockAttestationDecision::rejected(
                    ClockAttestationReason::HoldoverLimitExceeded,
                );
            }
        }
        ClockSyncState::Alarmed => {
            return ClockAttestationDecision::rejected(ClockAttestationReason::ClockAlarmed);
        }
        ClockSyncState::Unknown => {
            return ClockAttestationDecision::unknown(
                ClockAttestationReason::ClockSyncStateUnknown,
            );
        }
    }

    let use_components = [
        (
            "maximum_offset_error",
            observation.maximum_offset_error_ticks,
        ),
        (
            "holdover_drift_error",
            observation.holdover_drift_error_ticks,
        ),
        ("capture_path_error", observation.capture_path_error_ticks),
        ("rounding_error", observation.rounding_error_ticks),
    ];
    let mut contributions = Vec::with_capacity(6 + use_components.len());
    contributions.push(UncertaintyContribution {
        component_id: "measurement_result".to_string(),
        upper_bound_ticks: measurement_uncertainty_ticks,
    });
    contributions.push(UncertaintyContribution {
        component_id: "traceability_chain".to_string(),
        upper_bound_ticks: traceability_uncertainty_ticks,
    });

    let mut operational_error_ticks = 0_u64;
    for (component_id, value) in use_components {
        let Some(value) = value else {
            return ClockAttestationDecision::unknown(
                ClockAttestationReason::MissingUncertaintyBound(component_id),
            );
        };
        let Some(total) = operational_error_ticks.checked_add(value) else {
            return ClockAttestationDecision::rejected(ClockAttestationReason::UncertaintyOverflow);
        };
        operational_error_ticks = total;
        contributions.push(UncertaintyContribution {
            component_id: component_id.to_string(),
            upper_bound_ticks: value,
        });
    }

    let Some(calibration_total) =
        measurement_uncertainty_ticks.checked_add(traceability_uncertainty_ticks)
    else {
        return ClockAttestationDecision::rejected(ClockAttestationReason::UncertaintyOverflow);
    };
    let Some(maximum_total_error_ticks) = calibration_total.checked_add(operational_error_ticks)
    else {
        return ClockAttestationDecision::rejected(ClockAttestationReason::UncertaintyOverflow);
    };

    // A proof-grade enclosure must fit entirely inside every calibration and
    // traceability validity interval, not merely have its centre inside.
    let Ok(radius) = i64::try_from(maximum_total_error_ticks) else {
        return ClockAttestationDecision::rejected(ClockAttestationReason::UncertaintyOverflow);
    };
    let Some(earliest) = event_time.checked_sub(radius) else {
        return ClockAttestationDecision::rejected(ClockAttestationReason::UncertaintyOverflow);
    };
    let Some(latest) = event_time.checked_add(radius) else {
        return ClockAttestationDecision::rejected(ClockAttestationReason::UncertaintyOverflow);
    };
    let applicable_from_ticks = calibration
        .traceability_links
        .iter()
        .fold(calibration.valid_from_ticks, |current, link| {
            current.max(link.valid_from_ticks)
        });
    let applicable_until_ticks = calibration
        .traceability_links
        .iter()
        .fold(calibration.valid_until_ticks, |current, link| {
            current.min(link.valid_until_ticks)
        });
    if earliest < applicable_from_ticks || latest > applicable_until_ticks {
        return ClockAttestationDecision::rejected(
            ClockAttestationReason::UncertaintyIntervalOutsideValidity,
        );
    }

    let artifact_digests =
        match verify_all_artifacts(calibration, observation, policy, decision_as_of_utc_seconds) {
            Ok(digests) => digests,
            Err(decision) => return decision,
        };

    ClockAttestationDecision::accepted(QualifiedClockCalibrationReceipt {
        result_id: calibration.result_id.clone(),
        observation_id: observation.observation_id.clone(),
        source_id: observation.source_id.clone(),
        source_event_id: observation.source_event_id.clone(),
        source_event_revision: observation.source_event_revision,
        clock_id: calibration.clock_id.clone(),
        clock_profile_id: calibration.clock_profile_id.clone(),
        firmware_id: calibration.firmware_id.clone(),
        measurand_id: calibration.measurand_id.clone(),
        time_basis_id: calibration.time_basis_id.clone(),
        tick_quantum_id: calibration.tick_quantum_id.clone(),
        terminal_reference_id: calibration.terminal_reference_id.clone(),
        event_time_ticks: event_time,
        reported_event_time_ticks: reported_event_time,
        observation_time_ticks: observation.observation_time_ticks,
        synchronization_method: observation.synchronization_method.clone(),
        sync_state: observation.sync_state,
        applicable_from_ticks,
        applicable_until_ticks,
        measured_offset_ticks: calibration.measured_offset_ticks,
        uncertainty_contributions: contributions,
        maximum_total_error_ticks,
        calibration_artifact_digests: artifact_digests,
        verifier_id: policy.verifier_id.clone(),
        verifier_version: policy.verifier_version.clone(),
        trust_store_id: policy.trust_store_id.clone(),
        trust_store_version: policy.trust_store_version.clone(),
        policy_id: policy.policy_id.clone(),
        policy_version: policy.policy_version.clone(),
        decision_as_of_utc_seconds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(id: &str, digest_byte: char) -> CalibrationArtifactReference {
        CalibrationArtifactReference {
            artifact_id: id.to_string(),
            sha256: std::iter::repeat_n(digest_byte, 64).collect(),
            media_type: "application/pdf".to_string(),
            schema_profile_id: "clock-calibration-certificate".to_string(),
            schema_profile_version: "1.0".to_string(),
        }
    }

    fn evidence(id: &str, digest_byte: char) -> CalibrationArtifactEvidence {
        let artifact = artifact(id, digest_byte);
        CalibrationArtifactEvidence {
            verifier: ArtifactVerifierReceipt {
                artifact_sha256: artifact.sha256.clone(),
                signer_fingerprint: "sha256:signer-fingerprint".to_string(),
                trust_anchor_id: "ZA-TRUST-ROOT-1".to_string(),
                signature_algorithm: "ECDSA-P256-SHA256".to_string(),
                signature_time_source_id: "trusted-timestamp-service-1".to_string(),
                signature_time_utc_seconds: 850,
                signer_certificate_not_before_utc_seconds: 700,
                signer_certificate_not_after_utc_seconds: 1_500,
                key_usage: BTreeSet::from(["digitalSignature".to_string()]),
                signature_status: VerifierStatus::Valid,
                certificate_chain_status: VerifierStatus::Valid,
                revocation_status: RevocationStatus::Good,
                revocation_evaluated_at_utc_seconds: 900,
                revocation_evidence_valid_until_utc_seconds: 2_000,
                verifier_id: "clock-artifact-verifier".to_string(),
                verifier_version: "0.1.0".to_string(),
                trust_store_id: "south-africa-trust-store".to_string(),
                trust_store_version: "2026-11-22".to_string(),
                policy_id: "clock-policy".to_string(),
                policy_version: "1".to_string(),
            },
            artifact,
        }
    }

    fn policy() -> ClockAttestationPolicy {
        ClockAttestationPolicy {
            policy_id: "clock-policy".to_string(),
            policy_version: "1".to_string(),
            verifier_id: "clock-artifact-verifier".to_string(),
            verifier_version: "0.1.0".to_string(),
            trust_store_id: "south-africa-trust-store".to_string(),
            trust_store_version: "2026-11-22".to_string(),
            trusted_anchor_ids: BTreeSet::from(["ZA-TRUST-ROOT-1".to_string()]),
            allowed_signature_algorithms: BTreeSet::from(["ECDSA-P256-SHA256".to_string()]),
            allowed_uncertainty_model_ids: BTreeSet::from([
                "conservative-expanded-bound-v1".to_string(),
                "link-bound-v1".to_string(),
            ]),
            allowed_synchronization_methods: BTreeSet::from(["PTP".to_string()]),
            required_key_usage: "digitalSignature".to_string(),
        }
    }

    fn expected() -> ExpectedClockScope {
        ExpectedClockScope {
            clock_id: "PTP-GM-01".to_string(),
            clock_profile_id: "terminal-clock-profile-v2".to_string(),
            firmware_id: "firmware-7.4.2".to_string(),
            measurand_id: "time-scale-difference".to_string(),
            time_basis_id: "UTC(ZA)".to_string(),
            tick_quantum_id: "SI_NANOSECOND".to_string(),
            terminal_reference_id: "UTC(ZA)".to_string(),
        }
    }

    fn calibration_fixture() -> CalibrationMeasurementResult {
        CalibrationMeasurementResult {
            result_id: "measurement-result-01".to_string(),
            clock_id: "PTP-GM-01".to_string(),
            clock_profile_id: "terminal-clock-profile-v2".to_string(),
            firmware_id: "firmware-7.4.2".to_string(),
            measurand_id: "time-scale-difference".to_string(),
            time_basis_id: "UTC(ZA)".to_string(),
            tick_quantum_id: "SI_NANOSECOND".to_string(),
            reference_standard_id: "WORKING-STD-A".to_string(),
            terminal_reference_id: "UTC(ZA)".to_string(),
            calibrated_at_ticks: 800,
            valid_from_ticks: 800,
            valid_until_ticks: 1_500,
            measured_offset_ticks: 4,
            measurement_uncertainty_bound_ticks: Some(10),
            coverage_factor_lexeme: "2".to_string(),
            uncertainty_model_id: "conservative-expanded-bound-v1".to_string(),
            artifact_evidence: evidence("calibration-result-pdf", 'a'),
            traceability_links: vec![
                TraceabilityLink {
                    link_id: "trace-01".to_string(),
                    calibrated_subject_id: "WORKING-STD-A".to_string(),
                    reference_standard_id: "NMISA-REF-A".to_string(),
                    measurement_result_id: "trace-result-01".to_string(),
                    measurand_id: "time-scale-difference".to_string(),
                    time_basis_id: "UTC(ZA)".to_string(),
                    tick_quantum_id: "SI_NANOSECOND".to_string(),
                    measured_at_ticks: 800,
                    valid_from_ticks: 800,
                    valid_until_ticks: 1_500,
                    uncertainty_bound_ticks: Some(2),
                    uncertainty_model_id: "link-bound-v1".to_string(),
                    artifact_evidence: evidence("trace-link-a", 'b'),
                },
                TraceabilityLink {
                    link_id: "trace-02".to_string(),
                    calibrated_subject_id: "NMISA-REF-A".to_string(),
                    reference_standard_id: "UTC(ZA)".to_string(),
                    measurement_result_id: "trace-result-02".to_string(),
                    measurand_id: "time-scale-difference".to_string(),
                    time_basis_id: "UTC(ZA)".to_string(),
                    tick_quantum_id: "SI_NANOSECOND".to_string(),
                    measured_at_ticks: 800,
                    valid_from_ticks: 800,
                    valid_until_ticks: 1_500,
                    uncertainty_bound_ticks: Some(3),
                    uncertainty_model_id: "link-bound-v1".to_string(),
                    artifact_evidence: evidence("trace-link-b", 'c'),
                },
            ],
        }
    }

    fn observation_fixture() -> ClockUseObservation {
        ClockUseObservation {
            observation_id: "clock-use-1000".to_string(),
            source_id: "WMS_A".to_string(),
            source_event_id: "urn:uuid:custody-event-1000".to_string(),
            source_event_revision: 4,
            clock_id: "PTP-GM-01".to_string(),
            clock_profile_id: "terminal-clock-profile-v2".to_string(),
            firmware_id: "firmware-7.4.2".to_string(),
            time_basis_id: "UTC(ZA)".to_string(),
            tick_quantum_id: "SI_NANOSECOND".to_string(),
            event_time_ticks: 1_000,
            observation_time_ticks: 995,
            maximum_observation_age_ticks: Some(10),
            last_sync_ticks: 990,
            synchronization_method: "PTP".to_string(),
            sync_state: ClockSyncState::Locked,
            maximum_sync_age_ticks: Some(30),
            maximum_holdover_duration_ticks: None,
            maximum_offset_error_ticks: Some(5),
            holdover_drift_error_ticks: Some(1),
            capture_path_error_ticks: Some(2),
            rounding_error_ticks: Some(1),
            artifact_evidence: evidence("clock-use-observation", 'd'),
        }
    }

    fn evaluate(
        calibration: &CalibrationMeasurementResult,
        observation: &ClockUseObservation,
    ) -> ClockAttestationDecision {
        attest_clock_for_event(calibration, observation, &expected(), &policy(), 1_000)
    }

    #[test]
    fn accepted_attestation_converts_to_proof_grade_source_bound_time_evidence() {
        let decision = evaluate(&calibration_fixture(), &observation_fixture());
        let evidence = decision.to_time_evidence().unwrap();
        assert_eq!(
            evidence.event_id(),
            "5:WMS_A:27:urn:uuid:custody-event-1000:4"
        );
        assert_eq!(evidence.time_basis_id(), "UTC(ZA)");
        assert_eq!(
            decision.receipt.as_ref().unwrap().event_time_ticks(),
            996
        );
        assert_eq!(
            decision.receipt.as_ref().unwrap().reported_event_time_ticks(),
            1_000
        );
        assert_eq!(
            decision.receipt.as_ref().unwrap().tick_quantum_id(),
            "SI_NANOSECOND"
        );
        assert_eq!(evidence.interval().earliest(), 972);
        assert_eq!(evidence.interval().latest(), 1_020);
        assert_eq!(
            evidence.bound_kind(),
            crate::temporal_order::TimeBoundKind::QualifiedEnclosure
        );
        assert_eq!(
            evidence.clock_qualification().unwrap().clock_id,
            "PTP-GM-01"
        );
    }

    #[test]
    fn unknown_or_rejected_attestation_cannot_create_proof_grade_evidence() {
        let mut calibration = calibration_fixture();
        calibration.measurement_uncertainty_bound_ticks = None;
        let unknown = evaluate(&calibration, &observation_fixture());
        assert_eq!(
            unknown.to_time_evidence(),
            Err(ClockEvidenceConversionError::DecisionNotAccepted)
        );

        let mut calibration = calibration_fixture();
        calibration.artifact_evidence.verifier.signature_status = VerifierStatus::Invalid;
        let rejected = evaluate(&calibration, &observation_fixture());
        assert_eq!(
            rejected.to_time_evidence(),
            Err(ClockEvidenceConversionError::DecisionNotAccepted)
        );
    }

    #[test]
    fn observation_must_be_captured_before_and_close_to_the_event() {
        let mut observation = observation_fixture();
        observation.observation_time_ticks = 1_001;
        assert_eq!(
            evaluate(&calibration_fixture(), &observation).reason,
            Some(ClockAttestationReason::ClockObservationAfterEvent)
        );

        let mut observation = observation_fixture();
        observation.observation_time_ticks = 900;
        observation.maximum_observation_age_ticks = Some(20);
        assert_eq!(
            evaluate(&calibration_fixture(), &observation).reason,
            Some(ClockAttestationReason::ClockObservationTooOld)
        );
    }

    #[test]
    fn valid_clock_result_and_unbroken_chain_are_accepted_for_declared_scope() {
        let decision = evaluate(&calibration_fixture(), &observation_fixture());
        assert_eq!(
            decision.status,
            ClockAttestationStatus::AcceptedForDeclaredScope
        );
        let receipt = decision.receipt.unwrap();
        assert_eq!(receipt.maximum_total_error_ticks(), 24);
        assert_eq!(receipt.calibration_artifact_digests().len(), 4);
        assert_eq!(receipt.source_id(), "WMS_A");
        assert_eq!(receipt.source_event_id(), "urn:uuid:custody-event-1000");
        assert_eq!(receipt.source_event_revision(), 4);
        assert_eq!(receipt.observation_time_ticks(), 995);
        assert_eq!(receipt.synchronization_method(), "PTP");
        assert_eq!(receipt.applicable_from_ticks(), 800);
        assert_eq!(receipt.applicable_until_ticks(), 1_500);
        assert_eq!(receipt.terminal_reference_id(), "UTC(ZA)");
    }

    #[test]
    fn uncertainty_components_are_added_conservatively_without_independence() {
        let decision = evaluate(&calibration_fixture(), &observation_fixture());
        let receipt = decision.receipt.unwrap();
        let ids: Vec<&str> = receipt
            .uncertainty_contributions()
            .iter()
            .map(|item| item.component_id.as_str())
            .collect();
        assert_eq!(
            ids,
            vec![
                "measurement_result",
                "traceability_chain",
                "maximum_offset_error",
                "holdover_drift_error",
                "capture_path_error",
                "rounding_error"
            ]
        );
    }

    #[test]
    fn explicit_zero_uncertainty_is_distinct_from_unknown() {
        let mut calibration = calibration_fixture();
        calibration.measurement_uncertainty_bound_ticks = Some(0);
        for link in &mut calibration.traceability_links {
            link.uncertainty_bound_ticks = Some(0);
        }
        let decision = evaluate(&calibration, &observation_fixture());
        assert_eq!(
            decision.status,
            ClockAttestationStatus::AcceptedForDeclaredScope
        );
        assert_eq!(decision.receipt.unwrap().maximum_total_error_ticks(), 9);
    }

    #[test]
    fn missing_measurement_uncertainty_is_unknown_not_zero() {
        let mut calibration = calibration_fixture();
        calibration.measurement_uncertainty_bound_ticks = None;
        let decision = evaluate(&calibration, &observation_fixture());
        assert_eq!(decision.status, ClockAttestationStatus::Unknown);
        assert_eq!(
            decision.reason,
            Some(ClockAttestationReason::MissingUncertaintyBound(
                "measurement result"
            ))
        );
        assert!(decision.receipt.is_none());
    }

    #[test]
    fn missing_traceability_link_uncertainty_is_unknown() {
        let mut calibration = calibration_fixture();
        calibration.traceability_links[0].uncertainty_bound_ticks = None;
        let decision = evaluate(&calibration, &observation_fixture());
        assert_eq!(decision.status, ClockAttestationStatus::Unknown);
    }

    #[test]
    fn artifact_digest_mismatch_is_rejected() {
        let mut calibration = calibration_fixture();
        calibration.artifact_evidence.verifier.artifact_sha256 = "0".repeat(64);
        let decision = evaluate(&calibration, &observation_fixture());
        assert_eq!(decision.status, ClockAttestationStatus::Rejected);
        assert_eq!(
            decision.reason,
            Some(ClockAttestationReason::ArtifactDigestMismatch)
        );
    }

    #[test]
    fn invalid_signature_is_rejected() {
        let mut calibration = calibration_fixture();
        calibration.artifact_evidence.verifier.signature_status = VerifierStatus::Invalid;
        let decision = evaluate(&calibration, &observation_fixture());
        assert_eq!(
            decision.reason,
            Some(ClockAttestationReason::InvalidSignature)
        );
    }

    #[test]
    fn unsupported_verifier_result_is_unknown() {
        let mut calibration = calibration_fixture();
        calibration.artifact_evidence.verifier.signature_status = VerifierStatus::Unsupported;
        let decision = evaluate(&calibration, &observation_fixture());
        assert_eq!(decision.status, ClockAttestationStatus::Unknown);
    }

    #[test]
    fn revoked_calibration_artifact_is_rejected() {
        let mut calibration = calibration_fixture();
        calibration.artifact_evidence.verifier.revocation_status = RevocationStatus::Revoked;
        let decision = evaluate(&calibration, &observation_fixture());
        assert_eq!(
            decision.reason,
            Some(ClockAttestationReason::RevokedArtifact)
        );
    }

    #[test]
    fn stale_historical_revocation_evidence_is_unknown() {
        let mut calibration = calibration_fixture();
        calibration
            .artifact_evidence
            .verifier
            .revocation_evidence_valid_until_utc_seconds = 999;
        let decision = evaluate(&calibration, &observation_fixture());
        assert_eq!(
            decision.reason,
            Some(ClockAttestationReason::RevocationEvidenceStale)
        );
    }

    #[test]
    fn signer_certificate_must_be_valid_at_trusted_signing_time() {
        let mut calibration = calibration_fixture();
        calibration
            .artifact_evidence
            .verifier
            .signature_time_utc_seconds = 1_600;
        assert_eq!(
            evaluate(&calibration, &observation_fixture()).reason,
            Some(ClockAttestationReason::SignerCertificateOutsideValidity)
        );
    }

    #[test]
    fn uncertainty_and_synchronization_models_are_policy_pinned() {
        let mut calibration = calibration_fixture();
        calibration.uncertainty_model_id = "unreviewed-model".to_string();
        assert_eq!(
            evaluate(&calibration, &observation_fixture()).reason,
            Some(ClockAttestationReason::DisallowedUncertaintyModel)
        );

        let mut observation = observation_fixture();
        observation.synchronization_method = "UNREVIEWED-NTP".to_string();
        assert_eq!(
            evaluate(&calibration_fixture(), &observation).reason,
            Some(ClockAttestationReason::DisallowedSynchronizationMethod)
        );
    }

    #[test]
    fn invalid_coverage_factor_is_not_accepted() {
        let mut calibration = calibration_fixture();
        calibration.coverage_factor_lexeme = "NaN".to_string();
        assert_eq!(
            evaluate(&calibration, &observation_fixture()).status,
            ClockAttestationStatus::Unknown
        );
    }

    #[test]
    fn untrusted_anchor_and_disallowed_algorithm_are_rejected() {
        let mut calibration = calibration_fixture();
        calibration.artifact_evidence.verifier.trust_anchor_id = "UNTRUSTED".to_string();
        assert_eq!(
            evaluate(&calibration, &observation_fixture()).reason,
            Some(ClockAttestationReason::UntrustedTrustAnchor)
        );

        let mut calibration = calibration_fixture();
        calibration.artifact_evidence.verifier.signature_algorithm = "UNSUPPORTED".to_string();
        assert_eq!(
            evaluate(&calibration, &observation_fixture()).reason,
            Some(ClockAttestationReason::DisallowedSignatureAlgorithm)
        );
    }

    #[test]
    fn device_profile_and_firmware_must_match_expected_scope() {
        let mut calibration = calibration_fixture();
        calibration.clock_id = "OTHER-CLOCK".to_string();
        assert_eq!(
            evaluate(&calibration, &observation_fixture()).reason,
            Some(ClockAttestationReason::ClockIdentityMismatch)
        );

        let mut calibration = calibration_fixture();
        calibration.firmware_id = "firmware-unknown".to_string();
        assert_eq!(
            evaluate(&calibration, &observation_fixture()).reason,
            Some(ClockAttestationReason::FirmwareMismatch)
        );

        let mut observation = observation_fixture();
        observation.clock_profile_id = "other-profile".to_string();
        assert_eq!(
            evaluate(&calibration_fixture(), &observation).reason,
            Some(ClockAttestationReason::ClockProfileMismatch)
        );
    }

    #[test]
    fn negative_clock_offset_moves_corrected_reference_time_forward() {
        let mut calibration = calibration_fixture();
        calibration.measured_offset_ticks = -7;
        let decision = evaluate(&calibration, &observation_fixture());
        assert_eq!(
            decision.receipt.as_ref().unwrap().reported_event_time_ticks(),
            1_000
        );
        assert_eq!(
            decision.receipt.as_ref().unwrap().event_time_ticks(),
            1_007
        );
        let evidence = decision.to_time_evidence().unwrap();
        assert_eq!(evidence.interval().earliest(), 983);
        assert_eq!(evidence.interval().latest(), 1_031);
    }

    #[test]
    fn signed_offset_correction_overflow_is_rejected() {
        let mut calibration = calibration_fixture();
        calibration.measured_offset_ticks = i64::MIN;
        let decision = evaluate(&calibration, &observation_fixture());
        assert_eq!(decision.status, ClockAttestationStatus::Rejected);
        assert_eq!(
            decision.reason,
            Some(ClockAttestationReason::OffsetCorrectionOverflow)
        );
    }

    #[test]
    fn measurand_time_basis_and_terminal_reference_must_match() {
        let mut calibration = calibration_fixture();
        calibration.measurand_id = "frequency".to_string();
        assert_eq!(
            evaluate(&calibration, &observation_fixture()).reason,
            Some(ClockAttestationReason::MeasurandMismatch)
        );

        let mut calibration = calibration_fixture();
        calibration.time_basis_id = "GPS".to_string();
        assert_eq!(
            evaluate(&calibration, &observation_fixture()).reason,
            Some(ClockAttestationReason::TimeBasisMismatch)
        );

        let mut calibration = calibration_fixture();
        calibration.terminal_reference_id = "UTC".to_string();
        assert_eq!(
            evaluate(&calibration, &observation_fixture()).reason,
            Some(ClockAttestationReason::TerminalReferenceMismatch)
        );
    }

    #[test]
    fn calibration_must_cover_the_event_time() {
        let mut calibration = calibration_fixture();
        calibration.valid_until_ticks = 999;
        assert_eq!(
            evaluate(&calibration, &observation_fixture()).reason,
            Some(ClockAttestationReason::CalibrationOutsideValidity)
        );
    }

    #[test]
    fn broken_or_empty_traceability_chain_never_qualifies() {
        let mut calibration = calibration_fixture();
        calibration.traceability_links[1].calibrated_subject_id = "OTHER-STD".to_string();
        assert_eq!(
            evaluate(&calibration, &observation_fixture()).reason,
            Some(ClockAttestationReason::BrokenTraceabilityChain)
        );

        let mut calibration = calibration_fixture();
        calibration.traceability_links.clear();
        assert_eq!(
            evaluate(&calibration, &observation_fixture()).reason,
            Some(ClockAttestationReason::EmptyTraceabilityChain)
        );
    }

    #[test]
    fn traceability_scope_and_validity_are_enforced() {
        let mut calibration = calibration_fixture();
        calibration.traceability_links[0].time_basis_id = "GPS".to_string();
        assert_eq!(
            evaluate(&calibration, &observation_fixture()).reason,
            Some(ClockAttestationReason::TraceabilityScopeMismatch)
        );

        let mut calibration = calibration_fixture();
        calibration.traceability_links[0].valid_until_ticks = 999;
        assert_eq!(
            evaluate(&calibration, &observation_fixture()).reason,
            Some(ClockAttestationReason::TraceabilityOutsideValidity)
        );
    }

    #[test]
    fn alarmed_clock_is_rejected_and_unknown_state_abstains() {
        let mut observation = observation_fixture();
        observation.sync_state = ClockSyncState::Alarmed;
        assert_eq!(
            evaluate(&calibration_fixture(), &observation).reason,
            Some(ClockAttestationReason::ClockAlarmed)
        );

        let mut observation = observation_fixture();
        observation.sync_state = ClockSyncState::Unknown;
        assert_eq!(
            evaluate(&calibration_fixture(), &observation).status,
            ClockAttestationStatus::Unknown
        );
    }

    #[test]
    fn locked_clock_requires_fresh_last_sync_evidence() {
        let mut observation = observation_fixture();
        observation.last_sync_ticks = 900;
        observation.maximum_sync_age_ticks = Some(30);
        assert_eq!(
            evaluate(&calibration_fixture(), &observation).reason,
            Some(ClockAttestationReason::LockedClockSyncTooOld)
        );
    }

    #[test]
    fn holdover_is_accepted_only_within_an_explicit_duration_bound() {
        let mut observation = observation_fixture();
        observation.sync_state = ClockSyncState::Holdover;
        observation.maximum_holdover_duration_ticks = Some(20);
        assert_eq!(
            evaluate(&calibration_fixture(), &observation).status,
            ClockAttestationStatus::AcceptedForDeclaredScope
        );

        observation.maximum_holdover_duration_ticks = Some(5);
        assert_eq!(
            evaluate(&calibration_fixture(), &observation).reason,
            Some(ClockAttestationReason::HoldoverLimitExceeded)
        );
    }

    #[test]
    fn missing_operational_component_abstains() {
        let mut observation = observation_fixture();
        observation.capture_path_error_ticks = None;
        assert_eq!(
            evaluate(&calibration_fixture(), &observation).status,
            ClockAttestationStatus::Unknown
        );
    }

    #[test]
    fn uncertainty_overflow_is_rejected() {
        let mut calibration = calibration_fixture();
        calibration.measurement_uncertainty_bound_ticks = Some(u64::MAX);
        let decision = evaluate(&calibration, &observation_fixture());
        assert_eq!(decision.status, ClockAttestationStatus::Rejected);
        assert_eq!(
            decision.reason,
            Some(ClockAttestationReason::UncertaintyOverflow)
        );
    }

    #[test]
    fn invalid_artifact_digest_is_rejected_before_receipt_acceptance() {
        let mut calibration = calibration_fixture();
        calibration.artifact_evidence.artifact.sha256 = "not-a-sha256".to_string();
        calibration.artifact_evidence.verifier.artifact_sha256 = "not-a-sha256".to_string();
        let decision = evaluate(&calibration, &observation_fixture());
        assert_eq!(
            decision.reason,
            Some(ClockAttestationReason::InvalidArtifactDigest)
        );
    }

    #[test]
    fn verifier_and_trust_store_versions_are_pinned() {
        let mut calibration = calibration_fixture();
        calibration.artifact_evidence.verifier.trust_store_version = "old".to_string();
        assert_eq!(
            evaluate(&calibration, &observation_fixture()).reason,
            Some(ClockAttestationReason::VerifierPolicyMismatch)
        );
    }

    #[test]
    fn missing_document_signing_key_usage_is_rejected() {
        let mut calibration = calibration_fixture();
        calibration.artifact_evidence.verifier.key_usage.clear();
        assert_eq!(
            evaluate(&calibration, &observation_fixture()).reason,
            Some(ClockAttestationReason::MissingDocumentSigningKeyUsage)
        );
    }

    #[test]
    fn every_traceability_artifact_must_be_verified() {
        let mut calibration = calibration_fixture();
        calibration.traceability_links[1]
            .artifact_evidence
            .verifier
            .signature_status = VerifierStatus::Invalid;
        assert_eq!(
            evaluate(&calibration, &observation_fixture()).reason,
            Some(ClockAttestationReason::InvalidSignature)
        );
    }
}
