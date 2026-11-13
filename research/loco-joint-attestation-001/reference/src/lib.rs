//! Bind a joint receipt to an upstream RATS/EAT verifier appraisal.
//! This crate does not verify COSE/EAT, manage trust anchors, or prove physical
//! measurement truth. It checks exact packet/source/session bindings and the
//! typed appraisal result emitted by the configured upstream verifier.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceRole {
    DataAcquisition,
    TorqueTransducer,
    ElectricalAnalyzer,
    ThermalSensor,
    SensorGateway,
    ServiceInstrument,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureDisposition {
    Verified,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleDisposition {
    Authorized,
    Unauthorized,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshnessDisposition {
    Fresh,
    Stale,
    Replayed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceDisposition {
    Advance,
    Duplicate,
    Rollback,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootEpochDisposition {
    SameEpoch,
    AuthorizedNewEpoch,
    Rollback,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevocationDisposition {
    NotRevoked,
    Revoked,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppraisalDisposition {
    Pass,
    Fail,
    Inconclusive,
    NotAssessed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JointReceiptContext {
    pub payload_sha256: String,
    pub robot_identity_digest: String,
    pub robot_configuration_digest: String,
    pub source_subject_id: String,
    pub expected_source_role: SourceRole,
    pub actuator_identity_digest: String,
    pub joint_channel_id: String,
    pub acquisition_session_id: String,
    pub boot_epoch_digest: String,
    pub challenge_nonce_digest: String,
    pub audience: String,
    pub calibration_profile_digest: String,
    pub current_tick: u64,
    pub previous_sequence: Option<SequenceWatermark>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceWatermark {
    pub boot_epoch_digest: String,
    pub last_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifierTrustProfile {
    pub verifier_identity: String,
    pub verifier_profile_digest: String,
    pub appraisal_policy_digest: String,
    pub trust_anchor_set_digest: String,
    pub allowed_source_roles: BTreeSet<SourceRole>,
    pub maximum_age_ticks: u64,
}

/// This must be created by the Mycelix RATS/EAT adapter after verifying the
/// verifier-result receipt. Do not populate it from packet self-claims.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifierAppraisalReceipt {
    pub appraisal_result_id: String,
    pub appraisal_receipt_digest: String,
    pub verifier_identity: String,
    pub verifier_profile_digest: String,
    pub appraisal_policy_digest: String,
    pub trust_anchor_set_digest: String,
    pub issuer_identity: String,
    pub issuer_key_epoch_digest: String,
    pub evidence_bytes_digest: String,
    pub payload_sha256: String,
    pub source_subject_id: String,
    pub source_role: SourceRole,
    pub robot_identity_digest: String,
    pub robot_configuration_digest: String,
    pub actuator_identity_digest: String,
    pub joint_channel_id: String,
    pub acquisition_session_id: String,
    pub boot_epoch_digest: String,
    pub challenge_nonce_digest: String,
    pub audience: String,
    pub calibration_profile_digest: String,
    pub sequence_number: u64,
    pub issued_at_tick: u64,
    pub expires_at_tick: u64,
    pub signature: SignatureDisposition,
    pub role_authorization: RoleDisposition,
    pub freshness: FreshnessDisposition,
    pub sequence: SequenceDisposition,
    pub boot_epoch: BootEpochDisposition,
    pub revocation: RevocationDisposition,
    pub appraisal: AppraisalDisposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthenticationDisposition {
    AuthenticatedWithinDeclaredPolicy,
    Unattested,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JointAttestationAssessment {
    pub disposition: AuthenticationDisposition,
    pub payload_sha256: String,
    pub appraisal_receipt_digest: Option<String>,
    pub source_subject_id: Option<String>,
    pub sequence_number: Option<u64>,
    pub physical_validity_assessed: bool,
    pub actuation_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttestationBindingError {
    EmptyIdentity(&'static str),
    PayloadDigestMismatch,
    ScopeMismatch(&'static str),
    VerifierProfileMismatch(&'static str),
    SignatureFailed,
    UnauthorizedSourceRole,
    RevokedCredential,
    StaleOrReplayedEvidence,
    SequenceReplayOrRollback,
    AppraisalFailed,
    InvalidAppraisalInterval,
    EvidenceExpired,
}

fn check_identity(name: &'static str, value: &str) -> Result<(), AttestationBindingError> {
    if value.trim().is_empty() {
        Err(AttestationBindingError::EmptyIdentity(name))
    } else {
        Ok(())
    }
}

fn inconclusive(
    context: &JointReceiptContext,
    appraisal: &VerifierAppraisalReceipt,
) -> JointAttestationAssessment {
    JointAttestationAssessment {
        disposition: AuthenticationDisposition::Inconclusive,
        payload_sha256: context.payload_sha256.clone(),
        appraisal_receipt_digest: Some(appraisal.appraisal_receipt_digest.clone()),
        source_subject_id: Some(appraisal.source_subject_id.clone()),
        sequence_number: Some(appraisal.sequence_number),
        physical_validity_assessed: false,
        actuation_authorized: false,
    }
}

pub fn bind_joint_receipt_attestation(
    context: &JointReceiptContext,
    profile: &VerifierTrustProfile,
    appraisal: Option<&VerifierAppraisalReceipt>,
) -> Result<JointAttestationAssessment, AttestationBindingError> {
    for (name, value) in [
        ("payload_sha256", context.payload_sha256.as_str()),
        (
            "robot_identity_digest",
            context.robot_identity_digest.as_str(),
        ),
        (
            "robot_configuration_digest",
            context.robot_configuration_digest.as_str(),
        ),
        ("source_subject_id", context.source_subject_id.as_str()),
        (
            "actuator_identity_digest",
            context.actuator_identity_digest.as_str(),
        ),
        ("joint_channel_id", context.joint_channel_id.as_str()),
        (
            "acquisition_session_id",
            context.acquisition_session_id.as_str(),
        ),
        ("boot_epoch_digest", context.boot_epoch_digest.as_str()),
        (
            "challenge_nonce_digest",
            context.challenge_nonce_digest.as_str(),
        ),
        ("audience", context.audience.as_str()),
        (
            "calibration_profile_digest",
            context.calibration_profile_digest.as_str(),
        ),
        ("verifier_identity", profile.verifier_identity.as_str()),
        (
            "verifier_profile_digest",
            profile.verifier_profile_digest.as_str(),
        ),
        (
            "appraisal_policy_digest",
            profile.appraisal_policy_digest.as_str(),
        ),
        (
            "trust_anchor_set_digest",
            profile.trust_anchor_set_digest.as_str(),
        ),
    ] {
        check_identity(name, value)?;
    }

    let Some(appraisal) = appraisal else {
        return Ok(JointAttestationAssessment {
            disposition: AuthenticationDisposition::Unattested,
            payload_sha256: context.payload_sha256.clone(),
            appraisal_receipt_digest: None,
            source_subject_id: None,
            sequence_number: None,
            physical_validity_assessed: false,
            actuation_authorized: false,
        });
    };

    for (name, value) in [
        (
            "appraisal_result_id",
            appraisal.appraisal_result_id.as_str(),
        ),
        (
            "appraisal_receipt_digest",
            appraisal.appraisal_receipt_digest.as_str(),
        ),
        ("verifier_identity", appraisal.verifier_identity.as_str()),
        (
            "verifier_profile_digest",
            appraisal.verifier_profile_digest.as_str(),
        ),
        (
            "appraisal_policy_digest",
            appraisal.appraisal_policy_digest.as_str(),
        ),
        (
            "trust_anchor_set_digest",
            appraisal.trust_anchor_set_digest.as_str(),
        ),
        ("issuer_identity", appraisal.issuer_identity.as_str()),
        (
            "issuer_key_epoch_digest",
            appraisal.issuer_key_epoch_digest.as_str(),
        ),
        (
            "evidence_bytes_digest",
            appraisal.evidence_bytes_digest.as_str(),
        ),
        ("payload_sha256", appraisal.payload_sha256.as_str()),
        ("source_subject_id", appraisal.source_subject_id.as_str()),
        (
            "robot_identity_digest",
            appraisal.robot_identity_digest.as_str(),
        ),
        (
            "robot_configuration_digest",
            appraisal.robot_configuration_digest.as_str(),
        ),
        (
            "actuator_identity_digest",
            appraisal.actuator_identity_digest.as_str(),
        ),
        ("joint_channel_id", appraisal.joint_channel_id.as_str()),
        (
            "acquisition_session_id",
            appraisal.acquisition_session_id.as_str(),
        ),
        ("boot_epoch_digest", appraisal.boot_epoch_digest.as_str()),
        (
            "challenge_nonce_digest",
            appraisal.challenge_nonce_digest.as_str(),
        ),
        ("audience", appraisal.audience.as_str()),
        (
            "calibration_profile_digest",
            appraisal.calibration_profile_digest.as_str(),
        ),
    ] {
        check_identity(name, value)?;
    }

    for (field, observed, expected) in [
        (
            "verifier_identity",
            appraisal.verifier_identity.as_str(),
            profile.verifier_identity.as_str(),
        ),
        (
            "verifier_profile_digest",
            appraisal.verifier_profile_digest.as_str(),
            profile.verifier_profile_digest.as_str(),
        ),
        (
            "appraisal_policy_digest",
            appraisal.appraisal_policy_digest.as_str(),
            profile.appraisal_policy_digest.as_str(),
        ),
        (
            "trust_anchor_set_digest",
            appraisal.trust_anchor_set_digest.as_str(),
            profile.trust_anchor_set_digest.as_str(),
        ),
    ] {
        if observed != expected {
            return Err(AttestationBindingError::VerifierProfileMismatch(field));
        }
    }

    for (field, observed, expected) in [
        (
            "payload_sha256",
            appraisal.payload_sha256.as_str(),
            context.payload_sha256.as_str(),
        ),
        (
            "source_subject_id",
            appraisal.source_subject_id.as_str(),
            context.source_subject_id.as_str(),
        ),
        (
            "robot_identity_digest",
            appraisal.robot_identity_digest.as_str(),
            context.robot_identity_digest.as_str(),
        ),
        (
            "robot_configuration_digest",
            appraisal.robot_configuration_digest.as_str(),
            context.robot_configuration_digest.as_str(),
        ),
        (
            "actuator_identity_digest",
            appraisal.actuator_identity_digest.as_str(),
            context.actuator_identity_digest.as_str(),
        ),
        (
            "joint_channel_id",
            appraisal.joint_channel_id.as_str(),
            context.joint_channel_id.as_str(),
        ),
        (
            "acquisition_session_id",
            appraisal.acquisition_session_id.as_str(),
            context.acquisition_session_id.as_str(),
        ),
        (
            "boot_epoch_digest",
            appraisal.boot_epoch_digest.as_str(),
            context.boot_epoch_digest.as_str(),
        ),
        (
            "challenge_nonce_digest",
            appraisal.challenge_nonce_digest.as_str(),
            context.challenge_nonce_digest.as_str(),
        ),
        (
            "audience",
            appraisal.audience.as_str(),
            context.audience.as_str(),
        ),
        (
            "calibration_profile_digest",
            appraisal.calibration_profile_digest.as_str(),
            context.calibration_profile_digest.as_str(),
        ),
    ] {
        if observed != expected {
            if field == "payload_sha256" {
                return Err(AttestationBindingError::PayloadDigestMismatch);
            }
            return Err(AttestationBindingError::ScopeMismatch(field));
        }
    }

    if !profile
        .allowed_source_roles
        .contains(&appraisal.source_role)
        || appraisal.source_role != context.expected_source_role
    {
        return Err(AttestationBindingError::UnauthorizedSourceRole);
    }

    match appraisal.signature {
        SignatureDisposition::Verified => {}
        SignatureDisposition::Failed => return Err(AttestationBindingError::SignatureFailed),
        SignatureDisposition::Unknown => return Ok(inconclusive(context, appraisal)),
    }
    match appraisal.role_authorization {
        RoleDisposition::Authorized => {}
        RoleDisposition::Unauthorized => {
            return Err(AttestationBindingError::UnauthorizedSourceRole);
        }
        RoleDisposition::Unknown => return Ok(inconclusive(context, appraisal)),
    }
    match appraisal.revocation {
        RevocationDisposition::NotRevoked => {}
        RevocationDisposition::Revoked => return Err(AttestationBindingError::RevokedCredential),
        RevocationDisposition::Unknown => return Ok(inconclusive(context, appraisal)),
    }
    match appraisal.freshness {
        FreshnessDisposition::Fresh => {}
        FreshnessDisposition::Stale | FreshnessDisposition::Replayed => {
            return Err(AttestationBindingError::StaleOrReplayedEvidence);
        }
        FreshnessDisposition::Unknown => return Ok(inconclusive(context, appraisal)),
    }
    match appraisal.sequence {
        SequenceDisposition::Advance => {}
        SequenceDisposition::Duplicate | SequenceDisposition::Rollback => {
            return Err(AttestationBindingError::SequenceReplayOrRollback);
        }
        SequenceDisposition::Unknown => return Ok(inconclusive(context, appraisal)),
    }
    match appraisal.boot_epoch {
        BootEpochDisposition::SameEpoch => {}
        BootEpochDisposition::AuthorizedNewEpoch => {
            if context
                .previous_sequence
                .as_ref()
                .is_some_and(|previous| previous.boot_epoch_digest == context.boot_epoch_digest)
            {
                return Err(AttestationBindingError::SequenceReplayOrRollback);
            }
        }
        BootEpochDisposition::Rollback => {
            return Err(AttestationBindingError::SequenceReplayOrRollback);
        }
        BootEpochDisposition::Unknown => return Ok(inconclusive(context, appraisal)),
    }

    if appraisal.expires_at_tick <= appraisal.issued_at_tick
        || context.current_tick < appraisal.issued_at_tick
    {
        return Err(AttestationBindingError::InvalidAppraisalInterval);
    }
    if context.current_tick > appraisal.expires_at_tick
        || context.current_tick - appraisal.issued_at_tick > profile.maximum_age_ticks
    {
        return Err(AttestationBindingError::EvidenceExpired);
    }
    if appraisal.sequence_number == 0 {
        return Err(AttestationBindingError::SequenceReplayOrRollback);
    }
    if let Some(previous) = &context.previous_sequence {
        if previous.boot_epoch_digest == context.boot_epoch_digest
            && appraisal.sequence_number <= previous.last_sequence
        {
            return Err(AttestationBindingError::SequenceReplayOrRollback);
        }
    }
    match appraisal.appraisal {
        AppraisalDisposition::Pass => {}
        AppraisalDisposition::Fail => return Err(AttestationBindingError::AppraisalFailed),
        AppraisalDisposition::Inconclusive | AppraisalDisposition::NotAssessed => {
            return Ok(inconclusive(context, appraisal));
        }
    }

    Ok(JointAttestationAssessment {
        disposition: AuthenticationDisposition::AuthenticatedWithinDeclaredPolicy,
        payload_sha256: context.payload_sha256.clone(),
        appraisal_receipt_digest: Some(appraisal.appraisal_receipt_digest.clone()),
        source_subject_id: Some(appraisal.source_subject_id.clone()),
        sequence_number: Some(appraisal.sequence_number),
        physical_validity_assessed: false,
        actuation_authorized: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> JointReceiptContext {
        JointReceiptContext {
            payload_sha256: "joint-payload-sha256".into(),
            robot_identity_digest: "robot-a1".into(),
            robot_configuration_digest: "robot-config-a1".into(),
            source_subject_id: "daq-device-a1".into(),
            expected_source_role: SourceRole::DataAcquisition,
            actuator_identity_digest: "actuator-joint-1".into(),
            joint_channel_id: "joint-1-torque".into(),
            acquisition_session_id: "session-2026-01".into(),
            boot_epoch_digest: "boot-epoch-7".into(),
            challenge_nonce_digest: "nonce-123".into(),
            audience: "luminous-joint-receipt".into(),
            calibration_profile_digest: "calibration-profile-4".into(),
            current_tick: 110,
            previous_sequence: Some(SequenceWatermark {
                boot_epoch_digest: "boot-epoch-7".into(),
                last_sequence: 40,
            }),
        }
    }

    fn profile() -> VerifierTrustProfile {
        VerifierTrustProfile {
            verifier_identity: "mycelix-verifier".into(),
            verifier_profile_digest: "rats-profile-v1".into(),
            appraisal_policy_digest: "joint-policy-v1".into(),
            trust_anchor_set_digest: "trust-roots-v1".into(),
            allowed_source_roles: [SourceRole::DataAcquisition].into_iter().collect(),
            maximum_age_ticks: 50,
        }
    }

    fn appraisal() -> VerifierAppraisalReceipt {
        VerifierAppraisalReceipt {
            appraisal_result_id: "appraisal-1".into(),
            appraisal_receipt_digest: "appraisal-receipt-sha256".into(),
            verifier_identity: "mycelix-verifier".into(),
            verifier_profile_digest: "rats-profile-v1".into(),
            appraisal_policy_digest: "joint-policy-v1".into(),
            trust_anchor_set_digest: "trust-roots-v1".into(),
            issuer_identity: "instrument-maker".into(),
            issuer_key_epoch_digest: "instrument-key-epoch-3".into(),
            evidence_bytes_digest: "eat-cose-evidence-sha256".into(),
            payload_sha256: "joint-payload-sha256".into(),
            source_subject_id: "daq-device-a1".into(),
            source_role: SourceRole::DataAcquisition,
            robot_identity_digest: "robot-a1".into(),
            robot_configuration_digest: "robot-config-a1".into(),
            actuator_identity_digest: "actuator-joint-1".into(),
            joint_channel_id: "joint-1-torque".into(),
            acquisition_session_id: "session-2026-01".into(),
            boot_epoch_digest: "boot-epoch-7".into(),
            challenge_nonce_digest: "nonce-123".into(),
            audience: "luminous-joint-receipt".into(),
            calibration_profile_digest: "calibration-profile-4".into(),
            sequence_number: 41,
            issued_at_tick: 100,
            expires_at_tick: 150,
            signature: SignatureDisposition::Verified,
            role_authorization: RoleDisposition::Authorized,
            freshness: FreshnessDisposition::Fresh,
            sequence: SequenceDisposition::Advance,
            boot_epoch: BootEpochDisposition::SameEpoch,
            revocation: RevocationDisposition::NotRevoked,
            appraisal: AppraisalDisposition::Pass,
        }
    }

    #[test]
    fn exact_current_packet_device_session_and_verifier_binding_passes() {
        let result =
            bind_joint_receipt_attestation(&context(), &profile(), Some(&appraisal())).unwrap();
        assert_eq!(
            result.disposition,
            AuthenticationDisposition::AuthenticatedWithinDeclaredPolicy
        );
        assert!(!result.physical_validity_assessed);
        assert!(!result.actuation_authorized);
        assert_eq!(result.sequence_number, Some(41));
    }

    #[test]
    fn absent_attestation_remains_explicitly_unattested() {
        let result = bind_joint_receipt_attestation(&context(), &profile(), None).unwrap();
        assert_eq!(result.disposition, AuthenticationDisposition::Unattested);
        assert!(!result.physical_validity_assessed);
        assert!(!result.actuation_authorized);
    }

    #[test]
    fn wrong_packet_digest_is_rejected() {
        let mut receipt = appraisal();
        receipt.payload_sha256 = "another-packet".into();
        assert_eq!(
            bind_joint_receipt_attestation(&context(), &profile(), Some(&receipt)),
            Err(AttestationBindingError::PayloadDigestMismatch)
        );
    }

    #[test]
    fn wrong_robot_configuration_or_joint_channel_is_rejected() {
        let mut receipt = appraisal();
        receipt.robot_configuration_digest = "other-config".into();
        assert_eq!(
            bind_joint_receipt_attestation(&context(), &profile(), Some(&receipt)),
            Err(AttestationBindingError::ScopeMismatch(
                "robot_configuration_digest"
            ))
        );
        let mut receipt = appraisal();
        receipt.joint_channel_id = "joint-2-torque".into();
        assert_eq!(
            bind_joint_receipt_attestation(&context(), &profile(), Some(&receipt)),
            Err(AttestationBindingError::ScopeMismatch("joint_channel_id"))
        );
    }

    #[test]
    fn wrong_source_subject_or_acquisition_session_is_rejected() {
        let mut receipt = appraisal();
        receipt.source_subject_id = "daq-other-robot".into();
        assert_eq!(
            bind_joint_receipt_attestation(&context(), &profile(), Some(&receipt)),
            Err(AttestationBindingError::ScopeMismatch("source_subject_id"))
        );
        let mut receipt = appraisal();
        receipt.acquisition_session_id = "old-session".into();
        assert_eq!(
            bind_joint_receipt_attestation(&context(), &profile(), Some(&receipt)),
            Err(AttestationBindingError::ScopeMismatch(
                "acquisition_session_id"
            ))
        );
    }

    #[test]
    fn verifier_identity_profile_policy_and_trust_anchors_are_pinned() {
        let mut receipt = appraisal();
        receipt.appraisal_policy_digest = "untrusted-policy".into();
        assert_eq!(
            bind_joint_receipt_attestation(&context(), &profile(), Some(&receipt)),
            Err(AttestationBindingError::VerifierProfileMismatch(
                "appraisal_policy_digest"
            ))
        );
    }

    #[test]
    fn failed_signature_or_revoked_key_is_rejected() {
        let mut receipt = appraisal();
        receipt.signature = SignatureDisposition::Failed;
        assert_eq!(
            bind_joint_receipt_attestation(&context(), &profile(), Some(&receipt)),
            Err(AttestationBindingError::SignatureFailed)
        );
        let mut receipt = appraisal();
        receipt.revocation = RevocationDisposition::Revoked;
        assert_eq!(
            bind_joint_receipt_attestation(&context(), &profile(), Some(&receipt)),
            Err(AttestationBindingError::RevokedCredential)
        );
    }

    #[test]
    fn valid_signature_does_not_bypass_unauthorized_source_role() {
        let mut receipt = appraisal();
        receipt.source_role = SourceRole::ServiceInstrument;
        assert_eq!(
            bind_joint_receipt_attestation(&context(), &profile(), Some(&receipt)),
            Err(AttestationBindingError::UnauthorizedSourceRole)
        );
    }

    #[test]
    fn stale_or_replayed_challenge_is_rejected() {
        let mut receipt = appraisal();
        receipt.freshness = FreshnessDisposition::Replayed;
        assert_eq!(
            bind_joint_receipt_attestation(&context(), &profile(), Some(&receipt)),
            Err(AttestationBindingError::StaleOrReplayedEvidence)
        );
        let mut receipt = appraisal();
        receipt.freshness = FreshnessDisposition::Stale;
        assert_eq!(bind_joint_receipt_attestation(&context(), &profile(), Some(&receipt)),
            Err(AttestationBindingError::StaleOrReplayedEvidence));
    }

    #[test]
    fn duplicate_or_rollback_sequence_is_rejected() {
        let mut receipt = appraisal();
        receipt.sequence_number = 40;
        assert_eq!(
            bind_joint_receipt_attestation(&context(), &profile(), Some(&receipt)),
            Err(AttestationBindingError::SequenceReplayOrRollback)
        );
        let mut receipt = appraisal();
        receipt.sequence = SequenceDisposition::Rollback;
        assert_eq!(bind_joint_receipt_attestation(&context(), &profile(), Some(&receipt)),
            Err(AttestationBindingError::SequenceReplayOrRollback));
    }

    #[test]
    fn authorized_boot_epoch_transition_requires_a_distinct_prior_epoch() {
        let mut receipt = appraisal();
        receipt.boot_epoch_digest = "boot-epoch-8".into();
        receipt.boot_epoch = BootEpochDisposition::AuthorizedNewEpoch;
        let mut ctx = context();
        ctx.boot_epoch_digest = "boot-epoch-8".into();
        let result = bind_joint_receipt_attestation(&ctx, &profile(), Some(&receipt)).unwrap();
        assert_eq!(result.disposition, AuthenticationDisposition::AuthenticatedWithinDeclaredPolicy);
        ctx.previous_sequence = Some(SequenceWatermark {
            boot_epoch_digest: "boot-epoch-8".into(),
            last_sequence: 2,
        });
        assert_eq!(
            bind_joint_receipt_attestation(&ctx, &profile(), Some(&receipt)),
            Err(AttestationBindingError::SequenceReplayOrRollback)
        );
    }

    #[test]
    fn stale_or_future_appraisal_interval_is_rejected() {
        let mut receipt = appraisal();
        receipt.expires_at_tick = 105;
        assert_eq!(
            bind_joint_receipt_attestation(&context(), &profile(), Some(&receipt)),
            Err(AttestationBindingError::EvidenceExpired)
        );
        let mut receipt = appraisal();
        receipt.issued_at_tick = 120;
        receipt.expires_at_tick = 150;
        assert_eq!(
            bind_joint_receipt_attestation(&context(), &profile(), Some(&receipt)),
            Err(AttestationBindingError::InvalidAppraisalInterval)
        );
    }

    #[test]
    fn unknown_signature_status_or_appraisal_yields_inconclusive() {
        let mut receipt = appraisal();
        receipt.signature = SignatureDisposition::Unknown;
        let result = bind_joint_receipt_attestation(&context(), &profile(), Some(&receipt)).unwrap();
        assert_eq!(result.disposition, AuthenticationDisposition::Inconclusive);
        let mut receipt = appraisal();
        receipt.appraisal = AppraisalDisposition::Inconclusive;
        let result = bind_joint_receipt_attestation(&context(), &profile(), Some(&receipt)).unwrap();
        assert_eq!(result.disposition, AuthenticationDisposition::Inconclusive);
    }

    #[test]
    fn unknown_role_revocation_or_sequence_evidence_does_not_authenticate() {
        let mut receipt = appraisal();
        receipt.role_authorization = RoleDisposition::Unknown;
        assert_eq!(
            bind_joint_receipt_attestation(&context(), &profile(), Some(&receipt))
                .unwrap()
                .disposition,
            AuthenticationDisposition::Inconclusive
        );
        let mut receipt = appraisal();
        receipt.revocation = RevocationDisposition::Unknown;
        assert_eq!(bind_joint_receipt_attestation(&context(), &profile(), Some(&receipt)).unwrap().disposition,
            AuthenticationDisposition::Inconclusive);
        let mut receipt = appraisal();
        receipt.sequence = SequenceDisposition::Unknown;
        assert_eq!(bind_joint_receipt_attestation(&context(), &profile(), Some(&receipt)).unwrap().disposition,
            AuthenticationDisposition::Inconclusive);
    }

    #[test]
    fn calibration_profile_binding_is_exact_but_does_not_prove_calibration_truth() {
        let mut receipt = appraisal();
        receipt.calibration_profile_digest = "other-calibration".into();
        assert_eq!(
            bind_joint_receipt_attestation(&context(), &profile(), Some(&receipt)),
            Err(AttestationBindingError::ScopeMismatch(
                "calibration_profile_digest"
            ))
        );
        let result = bind_joint_receipt_attestation(&context(), &profile(), Some(&appraisal())).unwrap();
        assert!(!result.physical_validity_assessed);
    }

    #[test]
    fn valid_signature_and_exact_scope_do_not_bypass_failed_appraisal() {
        let mut receipt = appraisal();
        receipt.appraisal = AppraisalDisposition::Fail;
        assert_eq!(
            bind_joint_receipt_attestation(&context(), &profile(), Some(&receipt)),
            Err(AttestationBindingError::AppraisalFailed)
        );
    }

    #[test]
    fn missing_verifier_receipt_identity_is_rejected() {
        let mut receipt = appraisal();
        receipt.appraisal_receipt_digest.clear();
        assert_eq!(
            bind_joint_receipt_attestation(&context(), &profile(), Some(&receipt)),
            Err(AttestationBindingError::EmptyIdentity(
                "appraisal_receipt_digest"
            ))
        );
    }
}
