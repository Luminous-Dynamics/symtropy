//! Model-agnostic decision-time action assurance.
//!
//! This crate checks whether a proposal is supported by a specific, fresh,
//! integrity-consistent evidence bundle and whether that same bundle still
//! supports the proposal immediately before execution. It does not actuate,
//! select driving actions, authenticate publishers, or prove semantic safety.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    CameraFront,
    CameraRear,
    CameraLeft,
    CameraRight,
    VehicleState,
    Localization,
    Lidar,
    Radar,
    Map,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceHealth {
    Healthy,
    Degraded,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationDisposition {
    Fresh,
    Delayed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissingReason {
    CameraDrop,
    SensorTimeout,
    TransportLoss,
    PartialObservation,
    ClockFault,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionClass {
    Routine,
    LaneChange,
    Merge,
    Overtake,
    IntersectionCrossing,
    VulnerableRoadUserInteraction,
    EmergencyResponse,
    MinimalRiskFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackDirective {
    HoldForReassessment,
    RequestIndependentFallback,
    EscalateToHumanOperator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorizationMode {
    Nominal,
    Restricted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateDecision {
    Authorized,
    AuthorizedRestricted,
    FallbackRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationSample {
    pub modality: Modality,
    pub sensor_id: String,
    pub sequence: u64,
    pub capture_timestamp_us: u64,
    pub ready_timestamp_us: u64,
    pub delivered_timestamp_us: u64,
    pub clock_id: String,
    pub payload_sha256: String,
    pub calibration_sha256: Option<String>,
    pub health: EvidenceHealth,
    pub disposition: ObservationDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissingEvidenceEvent {
    pub modality: Modality,
    pub sensor_id: String,
    pub sequence: u64,
    pub timestamp_us: u64,
    pub clock_id: String,
    pub reason: MissingReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceBundle {
    pub run_id: String,
    pub episode_id: String,
    pub agent_id: String,
    pub clock_id: String,
    pub samples: Vec<ObservationSample>,
    pub missing_events: Vec<MissingEvidenceEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModalityRequirement {
    pub modality: Modality,
    pub sensor_id: String,
    pub minimum_samples: usize,
    pub max_capture_age_us: u64,
    pub max_processing_delay_us: u64,
    pub max_delivery_delay_us: u64,
    pub max_inter_sample_gap_us: u64,
    pub require_healthy: bool,
    pub allow_delayed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceProfile {
    pub profile_id: String,
    pub allowed_action_classes: Vec<ActionClass>,
    pub requirements: Vec<ModalityRequirement>,
    pub max_inter_sensor_skew_us: u64,
    pub max_proposal_age_us: u64,
    pub max_authorization_lifetime_us: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GatePolicy {
    pub policy_id: String,
    pub nominal: EvidenceProfile,
    pub restricted: Option<EvidenceProfile>,
    pub fallback: FallbackDirective,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionProposal {
    pub run_id: String,
    pub episode_id: String,
    pub agent_id: String,
    pub clock_id: String,
    pub proposal_id: String,
    pub action_class: ActionClass,
    pub action_sha256: String,
    pub source_evidence_bundle_sha256: String,
    pub issued_at_us: u64,
    /// Exclusive upper bound: the proposal is expired when now >= expires_at_us.
    pub expires_at_us: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorizationEnvelope {
    pub schema_version: u32,
    pub policy_id: String,
    pub policy_sha256: String,
    pub profile_id: String,
    pub mode: AuthorizationMode,
    pub proposal_id: String,
    pub proposal_sha256: String,
    pub action_sha256: String,
    pub evidence_bundle_sha256: String,
    pub authorized_at_us: u64,
    /// Exclusive upper bound. Execution at this timestamp is rejected.
    pub not_after_us: u64,
    /// An unkeyed integrity commitment, not an authenticated signature.
    pub envelope_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViolationCode {
    EmptyIdentity,
    IdentityMismatch,
    InvalidPayloadDigest,
    InvalidCalibrationDigest,
    InvalidActionDigest,
    InvalidEvidenceBundleDigest,
    InvalidSequence,
    DuplicateSequence,
    TimestampRegression,
    InvalidTimestampOrder,
    FutureEvidenceTimestamp,
    ClockIdentityMismatch,
    InvalidSensorHealth,
    MissingRequiredSample,
    TooFewSamples,
    StaleObservation,
    ProcessingDelayExceeded,
    DeliveryDelayExceeded,
    InterSampleGapExceeded,
    InterSensorSkewExceeded,
    SensorHealthInsufficient,
    DelayedObservationNotAllowed,
    MissingAfterLatestSample,
    ActionClassNotAllowed,
    ProposalIssuedInFuture,
    ProposalTooOld,
    ProposalExpired,
    ProposalValidityInvalid,
    NoExecutionValidityWindow,
    InvalidProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateEvaluation {
    pub decision: GateDecision,
    pub policy_id: String,
    pub fallback: FallbackDirective,
    pub nominal_violations: Vec<ViolationCode>,
    pub restricted_violations: Vec<ViolationCode>,
    pub envelope: Option<AuthorizationEnvelope>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionRejection {
    EnvelopeCommitmentMismatch,
    PolicyChanged,
    ProposalChanged,
    EvidenceBundleChanged,
    BeforeAuthorization,
    Expired,
    ProfileUnavailable,
    EvidenceNoLongerSatisfies(Vec<ViolationCode>),
}

#[derive(Debug, Clone, Serialize)]
struct EnvelopeCommitment<'a> {
    schema_version: u32,
    policy_id: &'a str,
    policy_sha256: &'a str,
    profile_id: &'a str,
    mode: AuthorizationMode,
    proposal_id: &'a str,
    proposal_sha256: &'a str,
    action_sha256: &'a str,
    evidence_bundle_sha256: &'a str,
    authorized_at_us: u64,
    not_after_us: u64,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut rendered = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut rendered, "{byte:02x}");
    }
    rendered
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn json_sha256<T: Serialize>(value: &T) -> Result<String, String> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| format!("cannot serialize assurance commitment: {error}"))?;
    Ok(sha256_hex(&bytes))
}

/// Compute an order-independent commitment for a bundle. Sample order is
/// canonicalized by modality, sensor ID and sequence; all timestamps, health,
/// disposition, calibration and payload digests remain committed.
pub fn evidence_bundle_sha256(bundle: &EvidenceBundle) -> Result<String, String> {
    let mut canonical = bundle.clone();
    canonical.samples.sort_by(|left, right| {
        (left.modality, left.sensor_id.as_str(), left.sequence)
            .cmp(&(right.modality, right.sensor_id.as_str(), right.sequence))
    });
    canonical.missing_events.sort_by(|left, right| {
        (
            left.modality,
            left.sensor_id.as_str(),
            left.sequence,
            left.timestamp_us,
            left.reason,
        )
            .cmp(&(
                right.modality,
                right.sensor_id.as_str(),
                right.sequence,
                right.timestamp_us,
                right.reason,
            ))
    });
    json_sha256(&canonical)
}

pub fn action_proposal_sha256(proposal: &ActionProposal) -> Result<String, String> {
    json_sha256(proposal)
}

pub fn gate_policy_sha256(policy: &GatePolicy) -> Result<String, String> {
    json_sha256(policy)
}

fn envelope_sha256(envelope: &AuthorizationEnvelope) -> Result<String, String> {
    let commitment = EnvelopeCommitment {
        schema_version: envelope.schema_version,
        policy_id: &envelope.policy_id,
        policy_sha256: &envelope.policy_sha256,
        profile_id: &envelope.profile_id,
        mode: envelope.mode,
        proposal_id: &envelope.proposal_id,
        proposal_sha256: &envelope.proposal_sha256,
        action_sha256: &envelope.action_sha256,
        evidence_bundle_sha256: &envelope.evidence_bundle_sha256,
        authorized_at_us: envelope.authorized_at_us,
        not_after_us: envelope.not_after_us,
    };
    json_sha256(&commitment)
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_profile(profile: &EvidenceProfile) -> Result<(), String> {
    if !nonempty(&profile.profile_id) {
        return Err("profile_id must be non-empty".into());
    }
    if profile.allowed_action_classes.is_empty() || profile.requirements.is_empty() {
        return Err(format!(
            "profile {} must declare action classes and evidence requirements",
            profile.profile_id
        ));
    }
    if profile.max_inter_sensor_skew_us == 0
        || profile.max_proposal_age_us == 0
        || profile.max_authorization_lifetime_us == 0
    {
        return Err(format!(
            "profile {} has a zero time bound",
            profile.profile_id
        ));
    }
    let mut action_classes = profile.allowed_action_classes.clone();
    action_classes.sort();
    action_classes.dedup();
    if action_classes.len() != profile.allowed_action_classes.len() {
        return Err(format!(
            "profile {} contains duplicate action classes",
            profile.profile_id
        ));
    }

    let mut keys = profile
        .requirements
        .iter()
        .map(|requirement| (requirement.modality, requirement.sensor_id.as_str()))
        .collect::<Vec<_>>();
    if keys.iter().any(|(_, sensor_id)| !nonempty(sensor_id)) {
        return Err(format!(
            "profile {} contains an empty sensor ID",
            profile.profile_id
        ));
    }
    keys.sort();
    keys.dedup();
    if keys.len() != profile.requirements.len() {
        return Err(format!(
            "profile {} contains duplicate modality/sensor requirements",
            profile.profile_id
        ));
    }
    for requirement in &profile.requirements {
        if requirement.minimum_samples == 0
            || requirement.max_capture_age_us == 0
            || requirement.max_processing_delay_us == 0
            || requirement.max_delivery_delay_us == 0
            || requirement.max_inter_sample_gap_us == 0
        {
            return Err(format!(
                "profile {} contains a zero evidence bound",
                profile.profile_id
            ));
        }
    }
    Ok(())
}

fn validate_policy(policy: &GatePolicy) -> Result<(), String> {
    if !nonempty(&policy.policy_id) {
        return Err("policy_id must be non-empty".into());
    }
    validate_profile(&policy.nominal)?;
    if let Some(restricted) = &policy.restricted {
        validate_profile(restricted)?;
        if restricted.profile_id == policy.nominal.profile_id {
            return Err("nominal and restricted profiles must have distinct IDs".into());
        }
    }
    Ok(())
}

fn validate_proposal(proposal: &ActionProposal) -> Result<(), String> {
    for (field, value) in [
        ("run_id", proposal.run_id.as_str()),
        ("episode_id", proposal.episode_id.as_str()),
        ("agent_id", proposal.agent_id.as_str()),
        ("clock_id", proposal.clock_id.as_str()),
        ("proposal_id", proposal.proposal_id.as_str()),
    ] {
        if !nonempty(value) {
            return Err(format!("{field} must be non-empty"));
        }
    }
    if !valid_sha256(&proposal.action_sha256) {
        return Err("action_sha256 must be a lowercase SHA-256 digest".into());
    }
    if !valid_sha256(&proposal.source_evidence_bundle_sha256) {
        return Err("source_evidence_bundle_sha256 must be a lowercase SHA-256 digest".into());
    }
    if proposal.expires_at_us <= proposal.issued_at_us {
        return Err("proposal expiry must be later than proposal issue time".into());
    }
    Ok(())
}

fn bundle_integrity_violations(bundle: &EvidenceBundle, now_us: u64) -> Vec<ViolationCode> {
    let mut violations = Vec::new();
    if !nonempty(&bundle.run_id)
        || !nonempty(&bundle.episode_id)
        || !nonempty(&bundle.agent_id)
        || !nonempty(&bundle.clock_id)
    {
        violations.push(ViolationCode::EmptyIdentity);
    }

    let mut by_sensor: BTreeMap<(Modality, String), Vec<&ObservationSample>> = BTreeMap::new();
    for sample in &bundle.samples {
        if !nonempty(&sample.sensor_id) {
            violations.push(ViolationCode::EmptyIdentity);
        }
        if sample.sequence == 0 {
            violations.push(ViolationCode::InvalidSequence);
        }
        if !valid_sha256(&sample.payload_sha256) {
            violations.push(ViolationCode::InvalidPayloadDigest);
        }
        if sample
            .calibration_sha256
            .as_deref()
            .is_some_and(|digest| !valid_sha256(digest))
        {
            violations.push(ViolationCode::InvalidCalibrationDigest);
        }
        if sample.clock_id != bundle.clock_id {
            violations.push(ViolationCode::ClockIdentityMismatch);
        }
        if sample.capture_timestamp_us > sample.ready_timestamp_us
            || sample.ready_timestamp_us > sample.delivered_timestamp_us
        {
            violations.push(ViolationCode::InvalidTimestampOrder);
        }
        if sample.delivered_timestamp_us > now_us {
            violations.push(ViolationCode::FutureEvidenceTimestamp);
        }
        if sample.health == EvidenceHealth::Invalid {
            violations.push(ViolationCode::InvalidSensorHealth);
        }
        by_sensor
            .entry((sample.modality, sample.sensor_id.clone()))
            .or_default()
            .push(sample);
    }

    for samples in by_sensor.values_mut() {
        samples.sort_by_key(|sample| sample.sequence);
        for pair in samples.windows(2) {
            if pair[0].sequence == pair[1].sequence {
                violations.push(ViolationCode::DuplicateSequence);
            }
            if pair[0].capture_timestamp_us >= pair[1].capture_timestamp_us {
                violations.push(ViolationCode::TimestampRegression);
            }
        }
    }

    for event in &bundle.missing_events {
        if !nonempty(&event.sensor_id) || event.sequence == 0 {
            violations.push(ViolationCode::EmptyIdentity);
        }
        if event.clock_id != bundle.clock_id {
            violations.push(ViolationCode::ClockIdentityMismatch);
        }
        if event.timestamp_us > now_us {
            violations.push(ViolationCode::FutureEvidenceTimestamp);
        }
    }

    violations.sort();
    violations.dedup();
    violations
}

fn profile_violations(
    bundle: &EvidenceBundle,
    proposal: &ActionProposal,
    profile: &EvidenceProfile,
    now_us: u64,
) -> (Vec<ViolationCode>, Option<u64>) {
    let mut violations = bundle_integrity_violations(bundle, now_us);
    let mut latest_capture_times = Vec::new();
    let mut evidence_expiry_us = None;

    if !profile.allowed_action_classes.contains(&proposal.action_class) {
        violations.push(ViolationCode::ActionClassNotAllowed);
    }
    if proposal.run_id != bundle.run_id
        || proposal.episode_id != bundle.episode_id
        || proposal.agent_id != bundle.agent_id
        || proposal.clock_id != bundle.clock_id
    {
        violations.push(ViolationCode::IdentityMismatch);
    }
    if proposal.issued_at_us > now_us {
        violations.push(ViolationCode::ProposalIssuedInFuture);
    } else if now_us.saturating_sub(proposal.issued_at_us) > profile.max_proposal_age_us {
        violations.push(ViolationCode::ProposalTooOld);
    }
    if proposal.expires_at_us <= now_us {
        violations.push(ViolationCode::ProposalExpired);
    }

    for requirement in &profile.requirements {
        let mut samples = bundle
            .samples
            .iter()
            .filter(|sample| {
                sample.modality == requirement.modality
                    && sample.sensor_id == requirement.sensor_id
            })
            .collect::<Vec<_>>();
        samples.sort_by_key(|sample| sample.sequence);

        if samples.is_empty() {
            violations.push(ViolationCode::MissingRequiredSample);
            continue;
        }
        if samples.len() < requirement.minimum_samples {
            violations.push(ViolationCode::TooFewSamples);
            continue;
        }

        let recent = &samples[samples.len() - requirement.minimum_samples..];
        let latest = recent[recent.len() - 1];
        latest_capture_times.push(latest.capture_timestamp_us);

        for sample in recent {
            if sample.capture_timestamp_us > now_us {
                violations.push(ViolationCode::FutureEvidenceTimestamp);
                continue;
            }
            let capture_age = now_us.saturating_sub(sample.capture_timestamp_us);
            let processing_delay = sample
                .ready_timestamp_us
                .saturating_sub(sample.capture_timestamp_us);
            let delivery_delay = sample
                .delivered_timestamp_us
                .saturating_sub(sample.capture_timestamp_us);

            if capture_age >= requirement.max_capture_age_us {
                violations.push(ViolationCode::StaleObservation);
            }
            if processing_delay > requirement.max_processing_delay_us {
                violations.push(ViolationCode::ProcessingDelayExceeded);
            }
            if delivery_delay > requirement.max_delivery_delay_us {
                violations.push(ViolationCode::DeliveryDelayExceeded);
            }
            if requirement.require_healthy && sample.health != EvidenceHealth::Healthy {
                violations.push(ViolationCode::SensorHealthInsufficient);
            }
            if !requirement.allow_delayed
                && sample.disposition == ObservationDisposition::Delayed
            {
                violations.push(ViolationCode::DelayedObservationNotAllowed);
            }
        }

        for pair in recent.windows(2) {
            if pair[1].capture_timestamp_us.saturating_sub(pair[0].capture_timestamp_us)
                > requirement.max_inter_sample_gap_us
            {
                violations.push(ViolationCode::InterSampleGapExceeded);
            }
        }

        if bundle.missing_events.iter().any(|event| {
            event.modality == requirement.modality
                && event.sensor_id == requirement.sensor_id
                && event.timestamp_us >= latest.capture_timestamp_us
        }) {
            violations.push(ViolationCode::MissingAfterLatestSample);
        }

        if let Some(expiry) = latest
            .capture_timestamp_us
            .checked_add(requirement.max_capture_age_us)
        {
            evidence_expiry_us = Some(
                evidence_expiry_us.map_or(expiry, |current| current.min(expiry)),
            );
        } else {
            violations.push(ViolationCode::InvalidTimestampOrder);
        }
    }

    if let (Some(minimum), Some(maximum)) = (
        latest_capture_times.iter().min(),
        latest_capture_times.iter().max(),
    ) {
        if maximum.saturating_sub(*minimum) > profile.max_inter_sensor_skew_us {
            violations.push(ViolationCode::InterSensorSkewExceeded);
        }
    }

    violations.sort();
    violations.dedup();
    (violations, evidence_expiry_us)
}

fn make_envelope(
    policy: &GatePolicy,
    policy_sha: &str,
    proposal: &ActionProposal,
    proposal_sha: &str,
    bundle_sha: &str,
    profile: &EvidenceProfile,
    mode: AuthorizationMode,
    now_us: u64,
    evidence_expiry_us: u64,
) -> Result<AuthorizationEnvelope, String> {
    let policy_expiry = now_us
        .checked_add(profile.max_authorization_lifetime_us)
        .ok_or_else(|| "authorization lifetime timestamp overflow".to_string())?;
    let not_after_us = proposal
        .expires_at_us
        .min(policy_expiry)
        .min(evidence_expiry_us);
    if not_after_us <= now_us {
        return Err("no positive execution validity window remains".into());
    }

    let mut envelope = AuthorizationEnvelope {
        schema_version: SCHEMA_VERSION,
        policy_id: policy.policy_id.clone(),
        policy_sha256: policy_sha.to_string(),
        profile_id: profile.profile_id.clone(),
        mode,
        proposal_id: proposal.proposal_id.clone(),
        proposal_sha256: proposal_sha.to_string(),
        action_sha256: proposal.action_sha256.clone(),
        evidence_bundle_sha256: bundle_sha.to_string(),
        authorized_at_us: now_us,
        not_after_us,
        envelope_sha256: String::new(),
    };
    envelope.envelope_sha256 = envelope_sha256(&envelope)?;
    Ok(envelope)
}

/// Evaluate a proposal against the nominal profile first, then an explicitly
/// declared restricted profile. The gate never invents a degraded action.
pub fn evaluate(
    bundle: &EvidenceBundle,
    proposal: &ActionProposal,
    policy: &GatePolicy,
    now_us: u64,
) -> Result<GateEvaluation, String> {
    validate_policy(policy)?;
    validate_proposal(proposal)?;

    let bundle_sha = evidence_bundle_sha256(bundle)?;
    let proposal_sha = action_proposal_sha256(proposal)?;
    let policy_sha = gate_policy_sha256(policy)?;
    let commitment_violation = proposal.source_evidence_bundle_sha256 != bundle_sha;

    let (mut nominal_violations, nominal_evidence_expiry) =
        profile_violations(bundle, proposal, &policy.nominal, now_us);
    if commitment_violation {
        nominal_violations.push(ViolationCode::InvalidEvidenceBundleDigest);
    }
    nominal_violations.sort();
    nominal_violations.dedup();

    if nominal_violations.is_empty() {
        let evidence_expiry = nominal_evidence_expiry
            .ok_or_else(|| "nominal profile has no evidence expiry".to_string())?;
        match make_envelope(
            policy,
            &policy_sha,
            proposal,
            &proposal_sha,
            &bundle_sha,
            &policy.nominal,
            AuthorizationMode::Nominal,
            now_us,
            evidence_expiry,
        ) {
            Ok(envelope) => {
                return Ok(GateEvaluation {
                    decision: GateDecision::Authorized,
                    policy_id: policy.policy_id.clone(),
                    fallback: policy.fallback,
                    nominal_violations,
                    restricted_violations: Vec::new(),
                    envelope: Some(envelope),
                });
            }
            Err(_) => nominal_violations.push(ViolationCode::NoExecutionValidityWindow),
        }
    }

    if let Some(restricted) = &policy.restricted {
        let (mut restricted_violations, restricted_expiry) =
            profile_violations(bundle, proposal, restricted, now_us);
        if commitment_violation {
            restricted_violations.push(ViolationCode::InvalidEvidenceBundleDigest);
        }
        restricted_violations.sort();
        restricted_violations.dedup();

        if restricted_violations.is_empty() {
            let evidence_expiry = restricted_expiry
                .ok_or_else(|| "restricted profile has no evidence expiry".to_string())?;
            match make_envelope(
                policy,
                &policy_sha,
                proposal,
                &proposal_sha,
                &bundle_sha,
                restricted,
                AuthorizationMode::Restricted,
                now_us,
                evidence_expiry,
            ) {
                Ok(envelope) => {
                    return Ok(GateEvaluation {
                        decision: GateDecision::AuthorizedRestricted,
                        policy_id: policy.policy_id.clone(),
                        fallback: policy.fallback,
                        nominal_violations,
                        restricted_violations,
                        envelope: Some(envelope),
                    });
                }
                Err(_) => restricted_violations.push(ViolationCode::NoExecutionValidityWindow),
            }
        }

        Ok(GateEvaluation {
            decision: GateDecision::FallbackRequired,
            policy_id: policy.policy_id.clone(),
            fallback: policy.fallback,
            nominal_violations,
            restricted_violations,
            envelope: None,
        })
    } else {
        Ok(GateEvaluation {
            decision: GateDecision::FallbackRequired,
            policy_id: policy.policy_id.clone(),
            fallback: policy.fallback,
            nominal_violations,
            restricted_violations: Vec::new(),
            envelope: None,
        })
    }
}

/// Revalidate the exact authorization at the point of use. Any changed policy,
/// proposal or evidence bundle invalidates the envelope and requires a new
/// decision; the caller must not execute using a stale commitment.
pub fn revalidate_at_execution(
    envelope: &AuthorizationEnvelope,
    proposal: &ActionProposal,
    bundle: &EvidenceBundle,
    policy: &GatePolicy,
    now_us: u64,
) -> Result<(), ExecutionRejection> {
    let expected_envelope_sha =
        envelope_sha256(envelope).map_err(|_| ExecutionRejection::EnvelopeCommitmentMismatch)?;
    if envelope.schema_version != SCHEMA_VERSION
        || !valid_sha256(&envelope.envelope_sha256)
        || expected_envelope_sha != envelope.envelope_sha256
    {
        return Err(ExecutionRejection::EnvelopeCommitmentMismatch);
    }

    let current_policy_sha =
        gate_policy_sha256(policy).map_err(|_| ExecutionRejection::PolicyChanged)?;
    if current_policy_sha != envelope.policy_sha256 || policy.policy_id != envelope.policy_id {
        return Err(ExecutionRejection::PolicyChanged);
    }
    let current_proposal_sha =
        action_proposal_sha256(proposal).map_err(|_| ExecutionRejection::ProposalChanged)?;
    if current_proposal_sha != envelope.proposal_sha256
        || proposal.proposal_id != envelope.proposal_id
        || proposal.action_sha256 != envelope.action_sha256
    {
        return Err(ExecutionRejection::ProposalChanged);
    }
    let current_bundle_sha =
        evidence_bundle_sha256(bundle).map_err(|_| ExecutionRejection::EvidenceBundleChanged)?;
    if current_bundle_sha != envelope.evidence_bundle_sha256
        || proposal.source_evidence_bundle_sha256 != envelope.evidence_bundle_sha256
    {
        return Err(ExecutionRejection::EvidenceBundleChanged);
    }
    if now_us < envelope.authorized_at_us {
        return Err(ExecutionRejection::BeforeAuthorization);
    }
    if now_us >= envelope.not_after_us || now_us >= proposal.expires_at_us {
        return Err(ExecutionRejection::Expired);
    }

    let profile = match envelope.mode {
        AuthorizationMode::Nominal => {
            if policy.nominal.profile_id != envelope.profile_id {
                return Err(ExecutionRejection::ProfileUnavailable);
            }
            &policy.nominal
        }
        AuthorizationMode::Restricted => policy
            .restricted
            .as_ref()
            .filter(|profile| profile.profile_id == envelope.profile_id)
            .ok_or(ExecutionRejection::ProfileUnavailable)?,
    };
    let (violations, evidence_expiry_us) =
        profile_violations(bundle, proposal, profile, now_us);
    if !violations.is_empty() {
        return Err(ExecutionRejection::EvidenceNoLongerSatisfies(violations));
    }
    let evidence_expiry_us = evidence_expiry_us
        .ok_or(ExecutionRejection::ProfileUnavailable)?;
    let policy_expiry_us = envelope
        .authorized_at_us
        .checked_add(profile.max_authorization_lifetime_us)
        .ok_or(ExecutionRejection::EnvelopeCommitmentMismatch)?;
    let maximum_validity = proposal
        .expires_at_us
        .min(policy_expiry_us)
        .min(evidence_expiry_us);
    if envelope.not_after_us <= envelope.authorized_at_us
        || envelope.not_after_us > maximum_validity
    {
        return Err(ExecutionRejection::EnvelopeCommitmentMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_020_000;

    fn sample(
        modality: Modality,
        sensor_id: &str,
        sequence: u64,
        capture_timestamp_us: u64,
    ) -> ObservationSample {
        ObservationSample {
            modality,
            sensor_id: sensor_id.to_string(),
            sequence,
            capture_timestamp_us,
            ready_timestamp_us: capture_timestamp_us + 500,
            delivered_timestamp_us: capture_timestamp_us + 1_000,
            clock_id: "vehicle-monotonic-1".to_string(),
            payload_sha256: format!("{:064x}", sequence + 1),
            calibration_sha256: Some("c".repeat(64)),
            health: EvidenceHealth::Healthy,
            disposition: ObservationDisposition::Fresh,
        }
    }

    fn bundle() -> EvidenceBundle {
        let mut samples = Vec::new();
        for (modality, sensor_id) in [
            (Modality::CameraFront, "front"),
            (Modality::VehicleState, "ego-state"),
        ] {
            samples.push(sample(modality, sensor_id, 1, 950_000));
            samples.push(sample(modality, sensor_id, 2, 980_000));
            samples.push(sample(modality, sensor_id, 3, 1_000_000));
        }
        EvidenceBundle {
            run_id: "run-1".to_string(),
            episode_id: "episode-1".to_string(),
            agent_id: "ego".to_string(),
            clock_id: "vehicle-monotonic-1".to_string(),
            samples,
            missing_events: Vec::new(),
        }
    }

    fn requirement(
        modality: Modality,
        sensor_id: &str,
        max_age: u64,
        require_healthy: bool,
        allow_delayed: bool,
    ) -> ModalityRequirement {
        ModalityRequirement {
            modality,
            sensor_id: sensor_id.to_string(),
            minimum_samples: 2,
            max_capture_age_us: max_age,
            max_processing_delay_us: 5_000,
            max_delivery_delay_us: 8_000,
            max_inter_sample_gap_us: 60_000,
            require_healthy,
            allow_delayed,
        }
    }

    fn profile(
        profile_id: &str,
        allowed: Vec<ActionClass>,
        max_age: u64,
        require_healthy: bool,
        allow_delayed: bool,
        max_lifetime: u64,
    ) -> EvidenceProfile {
        EvidenceProfile {
            profile_id: profile_id.to_string(),
            allowed_action_classes: allowed,
            requirements: vec![
                requirement(
                    Modality::CameraFront,
                    "front",
                    max_age,
                    require_healthy,
                    allow_delayed,
                ),
                requirement(
                    Modality::VehicleState,
                    "ego-state",
                    max_age,
                    require_healthy,
                    allow_delayed,
                ),
            ],
            max_inter_sensor_skew_us: 20_000,
            max_proposal_age_us: 30_000,
            max_authorization_lifetime_us: max_lifetime,
        }
    }

    fn policy() -> GatePolicy {
        GatePolicy {
            policy_id: "decision-policy-v1".to_string(),
            nominal: profile(
                "nominal-v1",
                vec![
                    ActionClass::Routine,
                    ActionClass::LaneChange,
                    ActionClass::Overtake,
                ],
                100_000,
                true,
                false,
                60_000,
            ),
            restricted: Some(profile(
                "restricted-v1",
                vec![ActionClass::Routine],
                200_000,
                false,
                true,
                20_000,
            )),
            fallback: FallbackDirective::RequestIndependentFallback,
        }
    }

    fn proposal(bundle: &EvidenceBundle) -> ActionProposal {
        ActionProposal {
            run_id: bundle.run_id.clone(),
            episode_id: bundle.episode_id.clone(),
            agent_id: bundle.agent_id.clone(),
            clock_id: bundle.clock_id.clone(),
            proposal_id: "proposal-1".to_string(),
            action_class: ActionClass::LaneChange,
            action_sha256: "a".repeat(64),
            source_evidence_bundle_sha256: evidence_bundle_sha256(bundle).unwrap(),
            issued_at_us: 1_010_000,
            expires_at_us: 1_090_000,
        }
    }

    fn authorized() -> (
        EvidenceBundle,
        ActionProposal,
        GatePolicy,
        AuthorizationEnvelope,
    ) {
        let evidence = bundle();
        let action = proposal(&evidence);
        let policy = policy();
        let result = evaluate(&evidence, &action, &policy, NOW).unwrap();
        assert_eq!(result.decision, GateDecision::Authorized);
        (
            evidence,
            action,
            policy,
            result.envelope.expect("authorized decision must carry an envelope"),
        )
    }

    #[test]
    fn fresh_evidence_authorizes_nominal_action() {
        let (evidence, action, policy, envelope) = authorized();
        assert_eq!(envelope.mode, AuthorizationMode::Nominal);
        assert_eq!(envelope.evidence_bundle_sha256, evidence_bundle_sha256(&evidence).unwrap());
        assert_eq!(envelope.action_sha256, action.action_sha256);
        assert!(revalidate_at_execution(&envelope, &action, &evidence, &policy, NOW + 1).is_ok());
    }

    #[test]
    fn authorization_expiry_is_bounded_by_all_evidence_and_policy_deadlines() {
        let mut evidence = bundle();
        let mut action = proposal(&evidence);
        action.expires_at_us = 1_300_000;
        action.source_evidence_bundle_sha256 = evidence_bundle_sha256(&evidence).unwrap();
        let mut gate_policy = policy();
        gate_policy.nominal.max_authorization_lifetime_us = 300_000;
        let result = evaluate(&evidence, &action, &gate_policy, NOW).unwrap();
        let envelope = result.envelope.unwrap();
        assert_eq!(envelope.not_after_us, 1_100_000);
        assert!(revalidate_at_execution(
            &envelope,
            &action,
            &evidence,
            &gate_policy,
            1_099_999
        )
        .is_ok());
        assert_eq!(
            revalidate_at_execution(
                &envelope,
                &action,
                &evidence,
                &gate_policy,
                1_100_000
            ),
            Err(ExecutionRejection::Expired)
        );
        evidence.samples[0].payload_sha256 = "d".repeat(64);
    }

    #[test]
    fn missing_required_camera_evidence_requires_fallback() {
        let mut evidence = bundle();
        evidence.samples.retain(|sample| sample.modality != Modality::CameraFront);
        let action = proposal(&evidence);
        let result = evaluate(&evidence, &action, &policy(), NOW).unwrap();
        assert_eq!(result.decision, GateDecision::FallbackRequired);
        assert!(result.envelope.is_none());
        assert!(result
            .nominal_violations
            .contains(&ViolationCode::MissingRequiredSample));
    }

    #[test]
    fn stale_evidence_does_not_authorize_high_risk_action() {
        let mut evidence = bundle();
        for sample in evidence
            .samples
            .iter_mut()
            .filter(|sample| sample.modality == Modality::CameraFront)
        {
            sample.capture_timestamp_us = 850_000 + sample.sequence * 10_000;
            sample.ready_timestamp_us = sample.capture_timestamp_us + 500;
            sample.delivered_timestamp_us = sample.capture_timestamp_us + 1_000;
        }
        let action = proposal(&evidence);
        let result = evaluate(&evidence, &action, &policy(), NOW).unwrap();
        assert_eq!(result.decision, GateDecision::FallbackRequired);
        assert!(result
            .nominal_violations
            .contains(&ViolationCode::StaleObservation));
    }

    #[test]
    fn explicitly_restricted_profile_can_authorize_routine_action_under_delay() {
        let mut evidence = bundle();
        for sample in evidence.samples.iter_mut().filter(|sample| {
            sample.modality == Modality::CameraFront && sample.sequence == 3
        }) {
            sample.disposition = ObservationDisposition::Delayed;
        }
        let mut action = proposal(&evidence);
        action.action_class = ActionClass::Routine;
        action.source_evidence_bundle_sha256 = evidence_bundle_sha256(&evidence).unwrap();
        let result = evaluate(&evidence, &action, &policy(), NOW).unwrap();
        assert_eq!(result.decision, GateDecision::AuthorizedRestricted);
        assert_eq!(result.envelope.unwrap().mode, AuthorizationMode::Restricted);
        assert!(result
            .nominal_violations
            .contains(&ViolationCode::DelayedObservationNotAllowed));
    }

    #[test]
    fn restricted_profile_cannot_authorize_overtaking() {
        let mut evidence = bundle();
        for sample in evidence.samples.iter_mut().filter(|sample| {
            sample.modality == Modality::CameraFront && sample.sequence == 3
        }) {
            sample.disposition = ObservationDisposition::Delayed;
        }
        let mut action = proposal(&evidence);
        action.action_class = ActionClass::Overtake;
        action.source_evidence_bundle_sha256 = evidence_bundle_sha256(&evidence).unwrap();
        let result = evaluate(&evidence, &action, &policy(), NOW).unwrap();
        assert_eq!(result.decision, GateDecision::FallbackRequired);
        assert!(result
            .restricted_violations
            .contains(&ViolationCode::ActionClassNotAllowed));
    }

    #[test]
    fn new_observation_after_authorization_invalidates_old_envelope() {
        let (mut evidence, action, gate_policy, envelope) = authorized();
        evidence.samples.push(sample(
            Modality::CameraFront,
            "front",
            4,
            1_010_000,
        ));
        assert_eq!(
            revalidate_at_execution(&envelope, &action, &evidence, &gate_policy, NOW + 1),
            Err(ExecutionRejection::EvidenceBundleChanged)
        );
    }

    #[test]
    fn changed_action_invalidates_old_envelope() {
        let (evidence, mut action, gate_policy, envelope) = authorized();
        action.action_sha256 = "b".repeat(64);
        assert_eq!(
            revalidate_at_execution(&envelope, &action, &evidence, &gate_policy, NOW + 1),
            Err(ExecutionRejection::ProposalChanged)
        );
    }

    #[test]
    fn changed_policy_invalidates_old_envelope() {
        let (evidence, action, mut gate_policy, envelope) = authorized();
        gate_policy.nominal.max_inter_sensor_skew_us -= 1;
        assert_eq!(
            revalidate_at_execution(&envelope, &action, &evidence, &gate_policy, NOW + 1),
            Err(ExecutionRejection::PolicyChanged)
        );
    }

    #[test]
    fn modified_authorization_commitment_is_rejected() {
        let (evidence, action, gate_policy, mut envelope) = authorized();
        envelope.not_after_us += 1;
        assert_eq!(
            revalidate_at_execution(&envelope, &action, &evidence, &gate_policy, NOW + 1),
            Err(ExecutionRejection::EnvelopeCommitmentMismatch)
        );
    }

    #[test]
    fn future_delivery_timestamp_is_rejected() {
        let mut evidence = bundle();
        evidence.samples[0].delivered_timestamp_us = NOW + 1;
        let action = proposal(&evidence);
        let result = evaluate(&evidence, &action, &policy(), NOW).unwrap();
        assert_eq!(result.decision, GateDecision::FallbackRequired);
        assert!(result
            .nominal_violations
            .contains(&ViolationCode::FutureEvidenceTimestamp));
    }

    #[test]
    fn sequence_timestamp_regression_is_rejected() {
        let mut evidence = bundle();
        let low_sequence = evidence.samples.iter_mut().find(|sample| {
            sample.modality == Modality::CameraFront && sample.sequence == 1
        }).unwrap();
        low_sequence.capture_timestamp_us = 990_000;
        low_sequence.ready_timestamp_us = 990_500;
        low_sequence.delivered_timestamp_us = 991_000;
        let action = proposal(&evidence);
        let result = evaluate(&evidence, &action, &policy(), NOW).unwrap();
        assert_eq!(result.decision, GateDecision::FallbackRequired);
        assert!(result
            .nominal_violations
            .contains(&ViolationCode::TimestampRegression));
    }

    #[test]
    fn duplicate_sensor_sequence_is_rejected() {
        let mut evidence = bundle();
        let duplicate = evidence.samples[0].clone();
        evidence.samples.push(duplicate);
        let action = proposal(&evidence);
        let result = evaluate(&evidence, &action, &policy(), NOW).unwrap();
        assert_eq!(result.decision, GateDecision::FallbackRequired);
        assert!(result
            .nominal_violations
            .contains(&ViolationCode::DuplicateSequence));
    }

    #[test]
    fn inter_sensor_clock_skew_is_rejected() {
        let mut evidence = bundle();
        for sample in evidence.samples.iter_mut().filter(|sample| {
            sample.modality == Modality::VehicleState
        }) {
            sample.capture_timestamp_us -= 30_000;
            sample.ready_timestamp_us -= 30_000;
            sample.delivered_timestamp_us -= 30_000;
        }
        let action = proposal(&evidence);
        let result = evaluate(&evidence, &action, &policy(), NOW).unwrap();
        assert_eq!(result.decision, GateDecision::FallbackRequired);
        assert!(result
            .nominal_violations
            .contains(&ViolationCode::InterSensorSkewExceeded));
    }

    #[test]
    fn missing_event_after_latest_sample_invalidates_modality() {
        let mut evidence = bundle();
        evidence.missing_events.push(MissingEvidenceEvent {
            modality: Modality::CameraFront,
            sensor_id: "front".to_string(),
            sequence: 4,
            timestamp_us: 1_005_000,
            clock_id: evidence.clock_id.clone(),
            reason: MissingReason::CameraDrop,
        });
        let action = proposal(&evidence);
        let result = evaluate(&evidence, &action, &policy(), NOW).unwrap();
        assert_eq!(result.decision, GateDecision::FallbackRequired);
        assert!(result
            .nominal_violations
            .contains(&ViolationCode::MissingAfterLatestSample));
    }

    #[test]
    fn invalid_sensor_health_is_rejected_even_in_restricted_profile() {
        let mut evidence = bundle();
        evidence.samples[0].health = EvidenceHealth::Invalid;
        let action = proposal(&evidence);
        let result = evaluate(&evidence, &action, &policy(), NOW).unwrap();
        assert_eq!(result.decision, GateDecision::FallbackRequired);
        assert!(result
            .nominal_violations
            .contains(&ViolationCode::InvalidSensorHealth));
    }

    #[test]
    fn mismatched_evidence_commitment_is_rejected() {
        let evidence = bundle();
        let mut action = proposal(&evidence);
        action.source_evidence_bundle_sha256 = "f".repeat(64);
        let result = evaluate(&evidence, &action, &policy(), NOW).unwrap();
        assert_eq!(result.decision, GateDecision::FallbackRequired);
        assert!(result
            .nominal_violations
            .contains(&ViolationCode::InvalidEvidenceBundleDigest));
    }

    #[test]
    fn mismatched_sensor_clock_identity_is_rejected() {
        let mut evidence = bundle();
        evidence.samples[0].clock_id = "other-clock".to_string();
        let action = proposal(&evidence);
        let result = evaluate(&evidence, &action, &policy(), NOW).unwrap();
        assert_eq!(result.decision, GateDecision::FallbackRequired);
        assert!(result
            .nominal_violations
            .contains(&ViolationCode::ClockIdentityMismatch));
    }

    #[test]
    fn delayed_sample_not_allowed_by_profile_is_rejected() {
        let mut evidence = bundle();
        evidence.samples.iter_mut().find(|sample| {
            sample.modality == Modality::CameraFront && sample.sequence == 3
        }).unwrap().disposition = ObservationDisposition::Delayed;
        let mut gate_policy = policy();
        gate_policy.restricted = None;
        let action = proposal(&evidence);
        let result = evaluate(&evidence, &action, &gate_policy, NOW).unwrap();
        assert_eq!(result.decision, GateDecision::FallbackRequired);
        assert!(result
            .nominal_violations
            .contains(&ViolationCode::DelayedObservationNotAllowed));
    }

    #[test]
    fn canonical_bundle_digest_is_independent_of_input_array_order() {
        let mut evidence = bundle();
        let before = evidence_bundle_sha256(&evidence).unwrap();
        evidence.samples.reverse();
        evidence.missing_events.reverse();
        let after = evidence_bundle_sha256(&evidence).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn proposal_expired_before_evaluation_cannot_be_authorized() {
        let evidence = bundle();
        let mut action = proposal(&evidence);
        action.expires_at_us = NOW;
        action.source_evidence_bundle_sha256 = evidence_bundle_sha256(&evidence).unwrap();
        // Structurally valid interval, but the deadline has already elapsed.
        let result = evaluate(&evidence, &action, &policy(), NOW).unwrap();
        assert_eq!(result.decision, GateDecision::FallbackRequired);
        assert!(result
            .nominal_violations
            .contains(&ViolationCode::ProposalExpired));
    }

    #[test]
    fn bundle_identity_must_match_proposal_identity() {
        let evidence = bundle();
        let mut action = proposal(&evidence);
        action.agent_id = "other-agent".to_string();
        let result = evaluate(&evidence, &action, &policy(), NOW).unwrap();
        assert_eq!(result.decision, GateDecision::FallbackRequired);
        assert!(result
            .nominal_violations
            .contains(&ViolationCode::IdentityMismatch));
    }

    #[test]
    fn execution_before_authorization_is_rejected() {
        let (evidence, action, gate_policy, envelope) = authorized();
        assert_eq!(
            revalidate_at_execution(
                &envelope,
                &action,
                &evidence,
                &gate_policy,
                envelope.authorized_at_us - 1
            ),
            Err(ExecutionRejection::BeforeAuthorization)
        );
    }
}
