#![forbid(unsafe_code)]

//! Fault-safe startup and online baseline adaptation.
//!
//! The live reference baseline is immutable. Candidate adaptation data are
//! accumulated in a shadow lane and cannot affect current diagnosis. Any
//! unsupported regime, degraded input, residual excursion, alarm, ambiguity,
//! missing command history or causal-timing failure quarantines the candidate.
//! A completed candidate can only become a review package with independently
//! verified health attestations; this module has no baseline-commit operation.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt;

const RESIDUAL_TOLERANCE: f64 = 1e-9;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BaselineAdaptationPolicy {
    pub minimum_shadow_samples: usize,
    pub maximum_standardized_residual: f64,
    pub maximum_sample_gap_seconds: f64,
    pub minimum_independent_health_attestations: usize,
}

impl BaselineAdaptationPolicy {
    pub fn conservative_startup_shadow() -> Self {
        Self {
            // 90 seconds at LiU-ICE's nominal 20 Hz sample rate.
            // Collection occurs alongside the frozen live baseline, not as a blind period.
            minimum_shadow_samples: 1_800,
            maximum_standardized_residual: 3.0,
            maximum_sample_gap_seconds: 0.10,
            minimum_independent_health_attestations: 2,
        }
    }

    fn validate(&self) -> Result<(), BaselineAdaptationError> {
        if self.minimum_shadow_samples == 0
            || !self.maximum_standardized_residual.is_finite()
            || self.maximum_standardized_residual < 0.0
            || !self.maximum_sample_gap_seconds.is_finite()
            || self.maximum_sample_gap_seconds <= 0.0
            || self.minimum_independent_health_attestations < 2
        {
            return Err(BaselineAdaptationError::InvalidPolicy);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BaselineShadowObservation {
    pub recording_id: String,
    pub sample_index: u64,
    pub time_seconds: f64,
    pub regime_id: String,
    pub input_receipt_sha256: String,
    pub standardized_absolute_residual: f64,
    pub regime_supported_by_frozen_reference: bool,
    pub all_required_signals_current: bool,
    pub command_history_available: bool,
    pub command_response_timing_causally_valid: bool,
    /// Dependency/common-cause status is verified by an upstream evidence service.
    pub dependency_health_verified_upstream: bool,
    pub known_fault_alarm_present: bool,
    pub unknown_fault_alarm_present: bool,
    pub ambiguous_hypothesis_set_present: bool,
    /// Independent common-cause/dependency health check performed upstream.
    pub dependency_health_verification_sha256: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BaselineCandidateStatus {
    CollectingShadowEvidence,
    Quarantined,
    ReadyForIndependentReview,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BaselineQuarantineReason {
    RecordingIdentityChanged,
    NonMonotonicSample,
    SampleSequenceGap,
    InvalidOrMissingEvidenceDigest,
    InvalidTimeOrResidual,
    UnsupportedRegime,
    RequiredSignalDegraded,
    CommandHistoryMissing,
    CommandResponseTimingInvalid,
    ResidualOutsideFrozenEnvelope,
    KnownFaultAlarm,
    UnknownFaultAlarm,
    AmbiguousHypothesisSet,
    DependencyHealthUnverified,
    ObservationGapExceeded,
    InsufficientShadowSamples,
    InsufficientIndependentHealthAttestations,
    DuplicateAttestationIssuer,
    DuplicateAttestationDependencyDomain,
    AttestationDoesNotBindCandidate,
    AttestationNotAuthorityVerified,
    AttestationConclusionNotHealthy,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct IndependentHealthAttestation {
    pub issuer_id: String,
    pub dependency_domain_id: String,
    pub candidate_manifest_sha256: String,
    pub attestation_receipt_sha256: String,
    pub issuer_authority_verification_sha256: String,
    pub conclusion: HealthAttestationConclusion,
    /// Caller-supplied result from the upstream trust/competence service.
    pub issuer_authority_verified_upstream: bool,
    /// Caller-supplied result from the upstream dependency/common-cause review.
    pub independence_verified_upstream: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HealthAttestationConclusion {
    ConfirmedHealthy,
    NotConfirmedHealthy,
    Unknown,
}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct BaselineAdaptationProposal {
    pub schema_version: String,
    pub base_baseline_sha256: String,
    pub model_sha256: String,
    pub regime_id: String,
    pub policy_sha256: String,
    pub candidate_manifest_sha256: String,
    pub sample_count: usize,
    pub first_sample_index: u64,
    pub last_sample_index: u64,
    pub first_time_seconds: f64,
    pub last_time_seconds: f64,
    pub shadow_receipt_chain_sha256: String,
    pub independent_health_attestation_receipt_sha256: Vec<String>,
    pub status: BaselineCandidateStatus,
    /// The frozen live baseline is never mutated by this module.
    pub baseline_mutation_performed: bool,
    /// This module proposes evidence for review; it cannot authorize a baseline change.
    pub baseline_update_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaselineAdaptationError {
    InvalidPolicy,
    EmptyIdentity,
    InvalidBaselineDigest,
    InvalidModelDigest,
    CandidateQuarantined(BaselineQuarantineReason),
    CandidateNotReady,
    InvalidObservation(String),
    ObservationNotAccepted(BaselineQuarantineReason),
    InvalidAttestation,
    ReceiptChainBroken,
    ReceiptDigestMismatch,
    MixedReceiptIdentity,
    Serialization,
}

impl fmt::Display for BaselineAdaptationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPolicy => write!(formatter, "baseline shadow policy is invalid"),
            Self::EmptyIdentity => write!(
                formatter,
                "recording, regime, issuer or dependency-domain identity is empty"
            ),
            Self::InvalidBaselineDigest => write!(
                formatter,
                "baseline digest must be a lowercase SHA-256 digest"
            ),
            Self::InvalidModelDigest => {
                write!(formatter, "model digest must be a lowercase SHA-256 digest")
            }
            Self::CandidateQuarantined(reason) => {
                write!(formatter, "baseline candidate is quarantined: {reason:?}")
            }
            Self::CandidateNotReady => write!(
                formatter,
                "shadow candidate has not met its sample requirement"
            ),
            Self::InvalidObservation(id) => write!(formatter, "invalid shadow observation: {id}"),
            Self::ObservationNotAccepted(reason) => {
                write!(formatter, "shadow observation failed closed: {reason:?}")
            }
            Self::InvalidAttestation => write!(
                formatter,
                "independent health-attestation set is incomplete or invalid"
            ),
            Self::ReceiptChainBroken => write!(
                formatter,
                "shadow observation receipt chain is incomplete or out of order"
            ),
            Self::ReceiptDigestMismatch => write!(
                formatter,
                "shadow observation receipt digest does not match its contents"
            ),
            Self::MixedReceiptIdentity => write!(
                formatter,
                "shadow receipt chain contains mixed baseline, model, recording or regime identities"
            ),
            Self::Serialization => write!(
                formatter,
                "failed to serialize baseline adaptation evidence"
            ),
        }
    }
}

impl std::error::Error for BaselineAdaptationError {}

#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct ShadowObservationReceipt {
    pub base_baseline_sha256: String,
    pub model_sha256: String,
    pub regime_id: String,
    pub recording_id: String,
    pub sample_index: u64,
    pub time_seconds: f64,
    pub input_receipt_sha256: String,
    pub standardized_absolute_residual: f64,
    pub regime_supported_by_frozen_reference: bool,
    pub all_required_signals_current: bool,
    pub command_history_available: bool,
    pub command_response_timing_causally_valid: bool,
    pub dependency_health_verified_upstream: bool,
    pub dependency_health_verification_sha256: String,
    pub known_fault_alarm_present: bool,
    pub unknown_fault_alarm_present: bool,
    pub ambiguous_hypothesis_set_present: bool,
    pub previous_receipt_sha256: Option<String>,
    pub receipt_sha256: String,
}

pub struct BaselineShadowCandidate {
    policy: BaselineAdaptationPolicy,
    policy_sha256: String,
    base_baseline_sha256: String,
    model_sha256: String,
    recording_id: String,
    regime_id: String,
    status: BaselineCandidateStatus,
    quarantine_reason: Option<BaselineQuarantineReason>,
    first_sample_index: Option<u64>,
    last_sample_index: Option<u64>,
    first_time_seconds: Option<f64>,
    last_time_seconds: Option<f64>,
    receipts: Vec<ShadowObservationReceipt>,
    previous_receipt_sha256: Option<String>,
}

impl BaselineShadowCandidate {
    pub fn new(
        policy: BaselineAdaptationPolicy,
        base_baseline_sha256: impl Into<String>,
        model_sha256: impl Into<String>,
        recording_id: impl Into<String>,
        regime_id: impl Into<String>,
    ) -> Result<Self, BaselineAdaptationError> {
        policy.validate()?;
        let base_baseline_sha256 = base_baseline_sha256.into();
        let model_sha256 = model_sha256.into();
        let recording_id = recording_id.into();
        let regime_id = regime_id.into();
        if !is_sha256_hex(&base_baseline_sha256) {
            return Err(BaselineAdaptationError::InvalidBaselineDigest);
        }
        if !is_sha256_hex(&model_sha256) {
            return Err(BaselineAdaptationError::InvalidModelDigest);
        }
        if recording_id.trim().is_empty() || regime_id.trim().is_empty() {
            return Err(BaselineAdaptationError::EmptyIdentity);
        }
        let policy_sha256 = sha256_serializable(b"symtropy-baseline-shadow-policy-v1\0", &policy)?;
        Ok(Self {
            policy,
            policy_sha256,
            base_baseline_sha256,
            model_sha256,
            recording_id,
            regime_id,
            status: BaselineCandidateStatus::CollectingShadowEvidence,
            quarantine_reason: None,
            first_sample_index: None,
            last_sample_index: None,
            first_time_seconds: None,
            last_time_seconds: None,
            receipts: Vec::new(),
            previous_receipt_sha256: None,
        })
    }

    /// Add one sample to the isolated shadow candidate. The live baseline is
    /// not writable through this API. A rejected sample quarantines the entire
    /// candidate so later observations cannot silently restart contaminated
    /// adaptation under the same candidate identity.
    pub fn observe(
        &mut self,
        observation: &BaselineShadowObservation,
    ) -> Result<BaselineCandidateStatus, BaselineAdaptationError> {
        if let Some(reason) = &self.quarantine_reason {
            return Err(BaselineAdaptationError::CandidateQuarantined(
                reason.clone(),
            ));
        }
        let reason = self.validate_observation(observation);
        if let Some(reason) = reason {
            self.status = BaselineCandidateStatus::Quarantined;
            self.quarantine_reason = Some(reason.clone());
            return Err(BaselineAdaptationError::ObservationNotAccepted(reason));
        }

        let receipt = self.make_receipt(observation)?;
        self.first_sample_index
            .get_or_insert(observation.sample_index);
        self.first_time_seconds
            .get_or_insert(observation.time_seconds);
        self.last_sample_index = Some(observation.sample_index);
        self.last_time_seconds = Some(observation.time_seconds);
        self.previous_receipt_sha256 = Some(receipt.receipt_sha256.clone());
        self.receipts.push(receipt);

        self.status = if self.receipts.len() >= self.policy.minimum_shadow_samples {
            BaselineCandidateStatus::ReadyForIndependentReview
        } else {
            BaselineCandidateStatus::CollectingShadowEvidence
        };
        Ok(self.status)
    }

    fn validate_observation(
        &self,
        observation: &BaselineShadowObservation,
    ) -> Option<BaselineQuarantineReason> {
        if observation.recording_id != self.recording_id {
            return Some(BaselineQuarantineReason::RecordingIdentityChanged);
        }
        if observation.regime_id != self.regime_id {
            return Some(BaselineQuarantineReason::UnsupportedRegime);
        }
        if !is_sha256_hex(&observation.input_receipt_sha256)
            || !is_sha256_hex(&observation.dependency_health_verification_sha256)
        {
            return Some(BaselineQuarantineReason::InvalidOrMissingEvidenceDigest);
        }
        if !observation.time_seconds.is_finite()
            || observation.time_seconds < 0.0
            || !observation.standardized_absolute_residual.is_finite()
            || observation.standardized_absolute_residual < 0.0
        {
            return Some(BaselineQuarantineReason::InvalidTimeOrResidual);
        }
        if let Some(previous) = self.last_sample_index {
            if observation.sample_index <= previous {
                return Some(BaselineQuarantineReason::NonMonotonicSample);
            }
            if observation.sample_index != previous.saturating_add(1) {
                return Some(BaselineQuarantineReason::SampleSequenceGap);
            }
        }
        if let Some(previous_time) = self.last_time_seconds {
            let gap = observation.time_seconds - previous_time;
            if gap <= 0.0 {
                return Some(BaselineQuarantineReason::NonMonotonicSample);
            }
            if gap > self.policy.maximum_sample_gap_seconds {
                return Some(BaselineQuarantineReason::ObservationGapExceeded);
            }
        }
        if !observation.regime_supported_by_frozen_reference {
            return Some(BaselineQuarantineReason::UnsupportedRegime);
        }
        if !observation.all_required_signals_current {
            return Some(BaselineQuarantineReason::RequiredSignalDegraded);
        }
        if !observation.command_history_available {
            return Some(BaselineQuarantineReason::CommandHistoryMissing);
        }
        if !observation.command_response_timing_causally_valid {
            return Some(BaselineQuarantineReason::CommandResponseTimingInvalid);
        }
        if observation.standardized_absolute_residual
            > self.policy.maximum_standardized_residual + RESIDUAL_TOLERANCE
        {
            return Some(BaselineQuarantineReason::ResidualOutsideFrozenEnvelope);
        }
        if observation.known_fault_alarm_present {
            return Some(BaselineQuarantineReason::KnownFaultAlarm);
        }
        if observation.unknown_fault_alarm_present {
            return Some(BaselineQuarantineReason::UnknownFaultAlarm);
        }
        if observation.ambiguous_hypothesis_set_present {
            return Some(BaselineQuarantineReason::AmbiguousHypothesisSet);
        }
        if !observation.dependency_health_verified_upstream {
            return Some(BaselineQuarantineReason::DependencyHealthUnverified);
        }
        None
    }

    fn make_receipt(
        &self,
        observation: &BaselineShadowObservation,
    ) -> Result<ShadowObservationReceipt, BaselineAdaptationError> {
        let mut receipt = ShadowObservationReceipt {
            base_baseline_sha256: self.base_baseline_sha256.clone(),
            model_sha256: self.model_sha256.clone(),
            regime_id: self.regime_id.clone(),
            recording_id: observation.recording_id.clone(),
            sample_index: observation.sample_index,
            time_seconds: observation.time_seconds,
            input_receipt_sha256: observation.input_receipt_sha256.clone(),
            standardized_absolute_residual: observation.standardized_absolute_residual,
            regime_supported_by_frozen_reference: observation.regime_supported_by_frozen_reference,
            all_required_signals_current: observation.all_required_signals_current,
            command_history_available: observation.command_history_available,
            command_response_timing_causally_valid: observation
                .command_response_timing_causally_valid,
            dependency_health_verified_upstream: observation.dependency_health_verified_upstream,
            dependency_health_verification_sha256: observation
                .dependency_health_verification_sha256
                .clone(),
            known_fault_alarm_present: observation.known_fault_alarm_present,
            unknown_fault_alarm_present: observation.unknown_fault_alarm_present,
            ambiguous_hypothesis_set_present: observation.ambiguous_hypothesis_set_present,
            previous_receipt_sha256: self.previous_receipt_sha256.clone(),
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = shadow_observation_receipt_sha256(&receipt)?;
        Ok(receipt)
    }

    pub fn status(&self) -> BaselineCandidateStatus {
        self.status
    }

    pub fn quarantine_reason(&self) -> Option<&BaselineQuarantineReason> {
        self.quarantine_reason.as_ref()
    }

    pub fn accepted_shadow_sample_count(&self) -> usize {
        self.receipts.len()
    }

    pub fn frozen_base_baseline_sha256(&self) -> &str {
        &self.base_baseline_sha256
    }

    pub fn proposal(
        &self,
        attestations: &[IndependentHealthAttestation],
    ) -> Result<BaselineAdaptationProposal, BaselineAdaptationError> {
        if self.status != BaselineCandidateStatus::ReadyForIndependentReview
            || self.receipts.len() < self.policy.minimum_shadow_samples
        {
            return Err(BaselineAdaptationError::CandidateNotReady);
        }
        let candidate_manifest_sha256 = self.candidate_manifest_sha256()?;
        validate_attestations(
            attestations,
            &candidate_manifest_sha256,
            self.policy.minimum_independent_health_attestations,
        )?;
        let attestation_receipts = attestations
            .iter()
            .map(|attestation| attestation.attestation_receipt_sha256.clone())
            .collect::<Vec<_>>();
        Ok(BaselineAdaptationProposal {
            schema_version: "symtropy-baseline-adaptation-proposal-v1".into(),
            base_baseline_sha256: self.base_baseline_sha256.clone(),
            model_sha256: self.model_sha256.clone(),
            regime_id: self.regime_id.clone(),
            policy_sha256: self.policy_sha256.clone(),
            candidate_manifest_sha256,
            sample_count: self.receipts.len(),
            first_sample_index: self
                .first_sample_index
                .expect("ready candidate has samples"),
            last_sample_index: self.last_sample_index.expect("ready candidate has samples"),
            first_time_seconds: self
                .first_time_seconds
                .expect("ready candidate has samples"),
            last_time_seconds: self.last_time_seconds.expect("ready candidate has samples"),
            shadow_receipt_chain_sha256: self.shadow_receipt_chain_sha256()?,
            independent_health_attestation_receipt_sha256: attestation_receipts,
            status: BaselineCandidateStatus::ReadyForIndependentReview,
            baseline_mutation_performed: false,
            baseline_update_authorized: false,
        })
    }

    fn candidate_manifest_sha256(&self) -> Result<String, BaselineAdaptationError> {
        let receipt_ids = self
            .receipts
            .iter()
            .map(|receipt| receipt.receipt_sha256.as_str())
            .collect::<Vec<_>>();
        sha256_serializable(
            b"symtropy-baseline-shadow-candidate-v1\0",
            &(
                self.base_baseline_sha256.as_str(),
                self.model_sha256.as_str(),
                self.recording_id.as_str(),
                self.regime_id.as_str(),
                self.policy_sha256.as_str(),
                &receipt_ids,
            ),
        )
    }

    fn shadow_receipt_chain_sha256(&self) -> Result<String, BaselineAdaptationError> {
        sha256_serializable(
            b"symtropy-baseline-shadow-chain-v1\0",
            &self
                .receipts
                .iter()
                .map(|receipt| receipt.receipt_sha256.as_str())
                .collect::<Vec<_>>(),
        )
    }
}

/// Recompute the digest over every decision-relevant field in a shadow receipt.
#[derive(Serialize)]
struct ShadowObservationDigest<'a> {
    base_baseline_sha256: &'a str,
    model_sha256: &'a str,
    regime_id: &'a str,
    recording_id: &'a str,
    sample_index: u64,
    time_seconds: f64,
    input_receipt_sha256: &'a str,
    standardized_absolute_residual: f64,
    regime_supported_by_frozen_reference: bool,
    all_required_signals_current: bool,
    command_history_available: bool,
    command_response_timing_causally_valid: bool,
    dependency_health_verified_upstream: bool,
    dependency_health_verification_sha256: &'a str,
    known_fault_alarm_present: bool,
    unknown_fault_alarm_present: bool,
    ambiguous_hypothesis_set_present: bool,
    previous_receipt_sha256: &'a Option<String>,
}

/// Recompute the digest over every decision-relevant field in a shadow receipt.
pub fn shadow_observation_receipt_sha256(
    receipt: &ShadowObservationReceipt,
) -> Result<String, BaselineAdaptationError> {
    let payload = ShadowObservationDigest {
        base_baseline_sha256: &receipt.base_baseline_sha256,
        model_sha256: &receipt.model_sha256,
        regime_id: &receipt.regime_id,
        recording_id: &receipt.recording_id,
        sample_index: receipt.sample_index,
        time_seconds: receipt.time_seconds,
        input_receipt_sha256: &receipt.input_receipt_sha256,
        standardized_absolute_residual: receipt.standardized_absolute_residual,
        regime_supported_by_frozen_reference: receipt.regime_supported_by_frozen_reference,
        all_required_signals_current: receipt.all_required_signals_current,
        command_history_available: receipt.command_history_available,
        command_response_timing_causally_valid: receipt.command_response_timing_causally_valid,
        dependency_health_verified_upstream: receipt.dependency_health_verified_upstream,
        dependency_health_verification_sha256: &receipt.dependency_health_verification_sha256,
        known_fault_alarm_present: receipt.known_fault_alarm_present,
        unknown_fault_alarm_present: receipt.unknown_fault_alarm_present,
        ambiguous_hypothesis_set_present: receipt.ambiguous_hypothesis_set_present,
        previous_receipt_sha256: &receipt.previous_receipt_sha256,
    };
    sha256_serializable(b"symtropy-baseline-shadow-observation-v3\0", &payload)
}

/// Replay the complete shadow receipt chain and reject any content or order change.
pub fn validate_shadow_receipt_chain(
    receipts: &[ShadowObservationReceipt],
) -> Result<(), BaselineAdaptationError> {
    if receipts.is_empty() {
        return Err(BaselineAdaptationError::ReceiptChainBroken);
    }
    let first = &receipts[0];
    let mut previous_receipt: Option<&str> = None;
    let mut previous_index: Option<u64> = None;
    let mut previous_time: Option<f64> = None;
    for receipt in receipts {
        if receipt.base_baseline_sha256 != first.base_baseline_sha256
            || receipt.model_sha256 != first.model_sha256
            || receipt.regime_id != first.regime_id
            || receipt.recording_id != first.recording_id
        {
            return Err(BaselineAdaptationError::MixedReceiptIdentity);
        }
        if !is_sha256_hex(&receipt.base_baseline_sha256)
            || !is_sha256_hex(&receipt.model_sha256)
            || !is_sha256_hex(&receipt.input_receipt_sha256)
            || !is_sha256_hex(&receipt.dependency_health_verification_sha256)
            || !is_sha256_hex(&receipt.receipt_sha256)
            || !receipt.time_seconds.is_finite()
            || receipt.time_seconds < 0.0
            || !receipt.standardized_absolute_residual.is_finite()
            || receipt.standardized_absolute_residual < 0.0
        {
            return Err(BaselineAdaptationError::ReceiptChainBroken);
        }
        if previous_index.is_some_and(|index| receipt.sample_index != index.saturating_add(1))
            || previous_time.is_some_and(|time| receipt.time_seconds <= time)
        {
            return Err(BaselineAdaptationError::ReceiptChainBroken);
        }
        if receipt.previous_receipt_sha256.as_deref() != previous_receipt {
            return Err(BaselineAdaptationError::ReceiptChainBroken);
        }
        // Integrity check precedes semantic admission checks: a changed decision
        // field is reported as tampering, not merely as a bad original observation.
        if shadow_observation_receipt_sha256(receipt)? != receipt.receipt_sha256 {
            return Err(BaselineAdaptationError::ReceiptDigestMismatch);
        }
        if !receipt.regime_supported_by_frozen_reference
            || !receipt.all_required_signals_current
            || !receipt.command_history_available
            || !receipt.command_response_timing_causally_valid
            || !receipt.dependency_health_verified_upstream
            || receipt.known_fault_alarm_present
            || receipt.unknown_fault_alarm_present
            || receipt.ambiguous_hypothesis_set_present
        {
            return Err(BaselineAdaptationError::ReceiptChainBroken);
        }
        previous_index = Some(receipt.sample_index);
        previous_time = Some(receipt.time_seconds);
        previous_receipt = Some(receipt.receipt_sha256.as_str());
    }
    Ok(())
}

fn validate_attestations(
    attestations: &[IndependentHealthAttestation],
    expected_candidate_manifest_sha256: &str,
    minimum_attestations: usize,
) -> Result<(), BaselineAdaptationError> {
    if attestations.len() < minimum_attestations {
        return Err(BaselineAdaptationError::CandidateQuarantined(
            BaselineQuarantineReason::InsufficientIndependentHealthAttestations,
        ));
    }
    let mut issuers = BTreeSet::new();
    let mut domains = BTreeSet::new();
    let mut receipt_ids = BTreeSet::new();
    for attestation in attestations {
        if attestation.issuer_id.trim().is_empty()
            || attestation.dependency_domain_id.trim().is_empty()
        {
            return Err(BaselineAdaptationError::EmptyIdentity);
        }
        if !is_sha256_hex(&attestation.candidate_manifest_sha256)
            || !is_sha256_hex(&attestation.attestation_receipt_sha256)
            || !is_sha256_hex(&attestation.issuer_authority_verification_sha256)
        {
            return Err(BaselineAdaptationError::InvalidAttestation);
        }
        if !receipt_ids.insert(attestation.attestation_receipt_sha256.as_str()) {
            return Err(BaselineAdaptationError::InvalidAttestation);
        }
        if attestation.candidate_manifest_sha256 != expected_candidate_manifest_sha256 {
            return Err(BaselineAdaptationError::CandidateQuarantined(
                BaselineQuarantineReason::AttestationDoesNotBindCandidate,
            ));
        }
        if !attestation.issuer_authority_verified_upstream
            || !attestation.independence_verified_upstream
        {
            return Err(BaselineAdaptationError::CandidateQuarantined(
                BaselineQuarantineReason::AttestationNotAuthorityVerified,
            ));
        }
        if attestation.conclusion != HealthAttestationConclusion::ConfirmedHealthy {
            return Err(BaselineAdaptationError::CandidateQuarantined(
                BaselineQuarantineReason::AttestationConclusionNotHealthy,
            ));
        }
        if !issuers.insert(attestation.issuer_id.as_str()) {
            return Err(BaselineAdaptationError::CandidateQuarantined(
                BaselineQuarantineReason::DuplicateAttestationIssuer,
            ));
        }
        if !domains.insert(attestation.dependency_domain_id.as_str()) {
            return Err(BaselineAdaptationError::CandidateQuarantined(
                BaselineQuarantineReason::DuplicateAttestationDependencyDomain,
            ));
        }
    }
    Ok(())
}

fn sha256_serializable<T: Serialize>(
    domain: &[u8],
    value: &T,
) -> Result<String, BaselineAdaptationError> {
    let encoded = serde_json::to_vec(value).map_err(|_| BaselineAdaptationError::Serialization)?;
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update((encoded.len() as u64).to_le_bytes());
    hasher.update(encoded);
    Ok(format!("{:x}", hasher.finalize()))
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn policy() -> BaselineAdaptationPolicy {
        BaselineAdaptationPolicy {
            minimum_shadow_samples: 2,
            maximum_standardized_residual: 3.0,
            maximum_sample_gap_seconds: 0.10,
            minimum_independent_health_attestations: 2,
        }
    }

    fn observation(index: u64) -> BaselineShadowObservation {
        BaselineShadowObservation {
            recording_id: "startup-run".into(),
            sample_index: index,
            time_seconds: index as f64 * 0.05,
            regime_id: "wltp".into(),
            input_receipt_sha256: digest(char::from(b'a' + index as u8)),
            standardized_absolute_residual: 0.5,
            regime_supported_by_frozen_reference: true,
            all_required_signals_current: true,
            command_history_available: true,
            command_response_timing_causally_valid: true,
            known_fault_alarm_present: false,
            unknown_fault_alarm_present: false,
            ambiguous_hypothesis_set_present: false,
            dependency_health_verification_sha256: digest('e'),
            dependency_health_verified_upstream: true,
        }
    }

    fn candidate() -> BaselineShadowCandidate {
        BaselineShadowCandidate::new(policy(), digest('b'), digest('c'), "startup-run", "wltp")
            .unwrap()
    }

    fn attestations(candidate_sha256: &str) -> Vec<IndependentHealthAttestation> {
        vec![
            IndependentHealthAttestation {
                issuer_id: "independent_inspection_system".into(),
                dependency_domain_id: "independent_power_domain".into(),
                candidate_manifest_sha256: candidate_sha256.into(),
                attestation_receipt_sha256: digest('f'),
                issuer_authority_verification_sha256: digest('1'),
                conclusion: HealthAttestationConclusion::ConfirmedHealthy,
                issuer_authority_verified_upstream: true,
                independence_verified_upstream: true,
            },
            IndependentHealthAttestation {
                issuer_id: "independent_engineer_review".into(),
                dependency_domain_id: "separate_review_domain".into(),
                candidate_manifest_sha256: candidate_sha256.into(),
                attestation_receipt_sha256: digest('2'),
                issuer_authority_verification_sha256: digest('3'),
                conclusion: HealthAttestationConclusion::ConfirmedHealthy,
                issuer_authority_verified_upstream: true,
                independence_verified_upstream: true,
            },
        ]
    }

    #[test]
    fn adaptation_candidate_never_mutates_the_frozen_live_baseline() {
        let mut candidate = candidate();
        let frozen_baseline = candidate.frozen_base_baseline_sha256().to_owned();
        assert_eq!(
            candidate.observe(&observation(0)).unwrap(),
            BaselineCandidateStatus::CollectingShadowEvidence
        );
        assert_eq!(
            candidate.observe(&observation(1)).unwrap(),
            BaselineCandidateStatus::ReadyForIndependentReview
        );
        let manifest = candidate.candidate_manifest_sha256().unwrap();
        let proposal = candidate.proposal(&attestations(&manifest)).unwrap();
        assert_eq!(proposal.base_baseline_sha256, frozen_baseline);
        assert_eq!(candidate.frozen_base_baseline_sha256(), frozen_baseline);
        assert!(!proposal.baseline_mutation_performed);
        assert!(!proposal.baseline_update_authorized);
    }

    #[test]
    fn startup_fault_or_warmup_anomaly_quarantines_before_candidate_can_be_reviewed() {
        let mut candidate = candidate();
        let mut first = observation(0);
        first.known_fault_alarm_present = true;
        assert_eq!(
            candidate.observe(&first),
            Err(BaselineAdaptationError::ObservationNotAccepted(
                BaselineQuarantineReason::KnownFaultAlarm
            ))
        );
        assert_eq!(candidate.status(), BaselineCandidateStatus::Quarantined);
        assert_eq!(
            candidate.observe(&observation(1)),
            Err(BaselineAdaptationError::CandidateQuarantined(
                BaselineQuarantineReason::KnownFaultAlarm
            ))
        );
    }

    #[test]
    fn quiet_but_out_of_envelope_startup_residual_is_not_learned_as_normal() {
        let mut candidate = candidate();
        let mut first = observation(0);
        first.standardized_absolute_residual = 8.0;
        assert_eq!(
            candidate.observe(&first),
            Err(BaselineAdaptationError::ObservationNotAccepted(
                BaselineQuarantineReason::ResidualOutsideFrozenEnvelope
            ))
        );
        assert_eq!(candidate.accepted_shadow_sample_count(), 0);
    }

    #[test]
    fn unsupported_regime_missing_sensor_and_ambiguous_sets_are_quarantined() {
        for mutate in [0_u8, 1_u8, 2_u8] {
            let mut candidate = candidate();
            let mut input = observation(0);
            match mutate {
                0 => input.regime_supported_by_frozen_reference = false,
                1 => input.all_required_signals_current = false,
                _ => input.ambiguous_hypothesis_set_present = true,
            }
            assert!(matches!(
                candidate.observe(&input),
                Err(BaselineAdaptationError::ObservationNotAccepted(_))
            ));
            assert_eq!(candidate.status(), BaselineCandidateStatus::Quarantined);
        }
    }

    #[test]
    fn skipped_samples_or_large_gaps_are_quarantined() {
        let mut skipped = candidate();
        skipped.observe(&observation(0)).unwrap();
        let mut gap = observation(1);
        gap.time_seconds = 1.0;
        assert_eq!(
            skipped.observe(&gap),
            Err(BaselineAdaptationError::ObservationNotAccepted(
                BaselineQuarantineReason::ObservationGapExceeded
            ))
        );

        let mut sequence = candidate();
        sequence.observe(&observation(0)).unwrap();
        assert_eq!(
            sequence.observe(&observation(2)),
            Err(BaselineAdaptationError::ObservationNotAccepted(
                BaselineQuarantineReason::SampleSequenceGap
            ))
        );
    }

    #[test]
    fn two_independent_attestations_must_bind_the_exact_shadow_candidate() {
        let mut candidate = candidate();
        candidate.observe(&observation(0)).unwrap();
        candidate.observe(&observation(1)).unwrap();
        let manifest = candidate.candidate_manifest_sha256().unwrap();
        let mut evidence = attestations(&manifest);
        evidence[1].candidate_manifest_sha256 = digest('9');
        assert_eq!(
            candidate.proposal(&evidence),
            Err(BaselineAdaptationError::CandidateQuarantined(
                BaselineQuarantineReason::AttestationDoesNotBindCandidate
            ))
        );
    }

    #[test]
    fn same_common_cause_domain_cannot_claim_independent_health() {
        let mut candidate = candidate();
        candidate.observe(&observation(0)).unwrap();
        candidate.observe(&observation(1)).unwrap();
        let manifest = candidate.candidate_manifest_sha256().unwrap();
        let mut evidence = attestations(&manifest);
        evidence[1].dependency_domain_id = evidence[0].dependency_domain_id.clone();
        assert_eq!(
            candidate.proposal(&evidence),
            Err(BaselineAdaptationError::CandidateQuarantined(
                BaselineQuarantineReason::DuplicateAttestationDependencyDomain
            ))
        );
    }

    #[test]
    fn shadow_receipt_chain_binds_residual_and_all_admission_decisions() {
        let mut candidate = candidate();
        candidate.observe(&observation(0)).unwrap();
        candidate.observe(&observation(1)).unwrap();
        let receipts = candidate.receipts.clone();
        assert!(validate_shadow_receipt_chain(&receipts).is_ok());

        let mut changed_residual = receipts.clone();
        changed_residual[0].standardized_absolute_residual += 0.25;
        assert_eq!(
            validate_shadow_receipt_chain(&changed_residual),
            Err(BaselineAdaptationError::ReceiptDigestMismatch)
        );

        let mut changed_admission = receipts.clone();
        changed_admission[1].command_response_timing_causally_valid = false;
        assert_eq!(
            validate_shadow_receipt_chain(&changed_admission),
            Err(BaselineAdaptationError::ReceiptDigestMismatch)
        );

        let mut broken_parent = receipts;
        broken_parent[1].previous_receipt_sha256 = None;
        assert_eq!(
            validate_shadow_receipt_chain(&broken_parent),
            Err(BaselineAdaptationError::ReceiptChainBroken)
        );
    }

    #[test]
    fn no_automatic_baseline_commit_path_exists_in_the_protocol() {
        let protocol: serde_json::Value =
            serde_json::from_str(include_str!("../startup-adaptation-guard-v0.1.json")).unwrap();
        assert_eq!(
            protocol["schema_version"],
            "symtropy-startup-adaptation-guard-v0.1"
        );
        assert!(
            protocol["architecture"]["update_authority"]
                .as_str()
                .unwrap()
                .contains("no commit or activation function")
        );
        assert!(
            protocol["architecture"]["live_reference"]["mutability"]
                .as_str()
                .unwrap()
                .contains("immutable")
        );
        assert_eq!(protocol["status"], "protocol_only_not_executed");
        assert!(
            protocol["research_targets_not_achieved"]
                .as_array()
                .unwrap()
                .iter()
                .all(|target| target["status"] == "aspirational")
        );
    }
}
