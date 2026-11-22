//! Fail-closed structural gate for externally verified UWB ranging sessions.
//!
//! This module does not implement STS, AES, key derivation, packet parsing,
//! signatures or hardware attestation. It consumes a verification receipt from
//! a trusted protocol/secure-element verifier, checks that the receipt is bound
//! to the expected peer/session/measurement, enforces freshness and replay
//! monotonicity, and only then issues a short-lived software grant.
//!
//! Replay state is in-memory in this research implementation. Production use
//! requires durable monotonic state or a trusted hardware epoch across process
//! restart; clearing this validator must never silently reset replay protection.

use std::collections::{HashMap, HashSet};
use std::fmt;

/// STS configuration reported by the protocol/secure ranging verifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StsMode {
    Static,
    Provisioned,
    Dynamic,
}

/// Result returned by an external trusted cryptographic verifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CryptoVerificationResult {
    Verified,
    Invalid,
    Unavailable,
}

/// Externally produced cryptographic verification receipt.
///
/// A caller must obtain this from a trusted IEEE 802.15.4z / FiRa implementation
/// or hardware-backed verifier. This module checks structure and bindings; it
/// does not cryptographically authenticate these fields itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CryptoVerificationReceipt {
    pub verifier_id: u32,
    pub result: CryptoVerificationResult,
    pub protocol_profile_id: u32,
    pub time_domain_id: u32,
    pub initiator_id: u32,
    pub responder_id: u32,
    pub session_id: [u8; 16],
    pub sub_session_id: u32,
    pub key_epoch: u64,
    pub measurement_sequence: u64,
    pub nonce: [u8; 16],
    pub transcript_digest: [u8; 32],
    pub receipt_digest: [u8; 32],
    /// Verification time in the policy's trusted monotonic time domain.
    pub verified_at_ns: u64,
}

/// Session and measurement metadata presented to the navigation boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RangingSessionEvidence {
    pub protocol_profile_id: u32,
    pub time_domain_id: u32,
    pub initiator_id: u32,
    pub responder_id: u32,
    pub session_id: [u8; 16],
    pub sub_session_id: u32,
    pub key_epoch: u64,
    pub measurement_sequence: u64,
    pub sts_mode: StsMode,
    pub nonce: [u8; 16],
    pub not_before_ns: u64,
    pub expires_at_ns: u64,
    pub verification: CryptoVerificationReceipt,
}

/// Requirements bound to the local deployment and trusted verifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionEvidencePolicy {
    pub trusted_verifier_id: u32,
    pub expected_protocol_profile_id: u32,
    pub expected_time_domain_id: u32,
    pub expected_initiator_id: u32,
    pub expected_responder_id: u32,
    pub maximum_session_validity_ns: u64,
    pub maximum_verification_receipt_age_ns: u64,
}

/// A grant emitted only after structural, freshness and replay checks succeed.
///
/// The grant does not itself establish physical LOS, accurate range, correct
/// antenna calibration, or a qualified navigation state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifiedRangingSessionGrant {
    pub protocol_profile_id: u32,
    pub initiator_id: u32,
    pub responder_id: u32,
    pub session_id: [u8; 16],
    pub sub_session_id: u32,
    pub key_epoch: u64,
    pub measurement_sequence: u64,
    pub sts_mode: StsMode,
    pub transcript_digest: [u8; 32],
    pub verification_receipt_digest: [u8; 32],
    pub valid_until_ns: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionEvidenceError {
    InvalidPolicy,
    WrongProtocolProfile,
    WrongTimeDomain,
    WrongInitiator,
    WrongResponder,
    InvalidSessionIdentity,
    InvalidNonce,
    InvalidValidityWindow,
    SessionNotYetValid,
    SessionExpired,
    SessionLifetimeExceeded,
    UntrustedVerifier,
    CryptographicVerificationFailed,
    ReceiptBindingMismatch,
    MissingTranscriptDigest,
    MissingReceiptDigest,
    VerificationReceiptFromFuture,
    VerificationReceiptStale,
    WeakStsMode,
    KeyEpochRollback,
    MeasurementSequenceReplay,
    NonceReplay,
    VerificationReceiptReplay,
}

impl fmt::Display for SessionEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPolicy => write!(f, "session evidence policy contains an invalid identity or time bound"),
            Self::WrongProtocolProfile => write!(f, "ranging session does not match the required protocol profile"),
            Self::WrongTimeDomain => write!(f, "ranging session time bounds use an untrusted time domain"),
            Self::WrongInitiator => write!(f, "ranging initiator identity does not match policy"),
            Self::WrongResponder => write!(f, "ranging responder identity does not match policy"),
            Self::InvalidSessionIdentity => write!(f, "ranging session, sub-session, key epoch or sequence is invalid"),
            Self::InvalidNonce => write!(f, "ranging measurement nonce is empty"),
            Self::InvalidValidityWindow => write!(f, "ranging session validity window is invalid"),
            Self::SessionNotYetValid => write!(f, "ranging session is not yet valid"),
            Self::SessionExpired => write!(f, "ranging session has expired"),
            Self::SessionLifetimeExceeded => write!(f, "ranging session exceeds the configured maximum lifetime"),
            Self::UntrustedVerifier => write!(f, "cryptographic verification receipt is not from the trusted verifier"),
            Self::CryptographicVerificationFailed => write!(f, "external cryptographic verifier did not return Verified"),
            Self::ReceiptBindingMismatch => write!(f, "cryptographic receipt is not bound to the presented ranging event"),
            Self::MissingTranscriptDigest => write!(f, "cryptographic transcript digest is missing"),
            Self::MissingReceiptDigest => write!(f, "verification receipt digest is missing"),
            Self::VerificationReceiptFromFuture => write!(f, "verification receipt time is in the future"),
            Self::VerificationReceiptStale => write!(f, "cryptographic verification receipt exceeds its freshness horizon"),
            Self::WeakStsMode => write!(f, "static STS mode is not accepted by this secure-ranging policy"),
            Self::KeyEpochRollback => write!(f, "key epoch moved backwards for this peer pair"),
            Self::MeasurementSequenceReplay => write!(f, "measurement sequence is not strictly increasing within the session"),
            Self::NonceReplay => write!(f, "ranging nonce has already been accepted"),
            Self::VerificationReceiptReplay => write!(f, "cryptographic verification receipt has already been accepted"),
        }
    }
}

impl std::error::Error for SessionEvidenceError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct PeerPair {
    initiator_id: u32,
    responder_id: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct SessionStream {
    peers: PeerPair,
    session_id: [u8; 16],
    sub_session_id: u32,
}

#[derive(Default)]
pub struct SessionEvidenceValidator {
    latest_key_epoch_by_peer_pair: HashMap<PeerPair, u64>,
    latest_sequence_by_session_stream: HashMap<SessionStream, u64>,
    seen_nonces: HashSet<[u8; 16]>,
    seen_receipt_digests: HashSet<[u8; 32]>,
}

impl SessionEvidenceValidator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Validate one measurement and advance replay state only on success.
    ///
    /// now_ns must be supplied by the trusted monotonic clock identified by
    /// policy.expected_time_domain_id. The policy is pinned by the caller.
    pub fn validate(
        &mut self,
        policy: SessionEvidencePolicy,
        evidence: RangingSessionEvidence,
        now_ns: u64,
    ) -> Result<VerifiedRangingSessionGrant, SessionEvidenceError> {
        validate_policy(policy)?;

        if evidence.protocol_profile_id != policy.expected_protocol_profile_id {
            return Err(SessionEvidenceError::WrongProtocolProfile);
        }
        if evidence.time_domain_id != policy.expected_time_domain_id {
            return Err(SessionEvidenceError::WrongTimeDomain);
        }
        if evidence.initiator_id != policy.expected_initiator_id {
            return Err(SessionEvidenceError::WrongInitiator);
        }
        if evidence.responder_id != policy.expected_responder_id {
            return Err(SessionEvidenceError::WrongResponder);
        }
        if evidence.session_id.iter().all(|byte| *byte == 0)
            || evidence.sub_session_id == 0
            || evidence.key_epoch == 0
            || evidence.measurement_sequence == 0
        {
            return Err(SessionEvidenceError::InvalidSessionIdentity);
        }
        if evidence.nonce.iter().all(|byte| *byte == 0) {
            return Err(SessionEvidenceError::InvalidNonce);
        }
        if evidence.expires_at_ns <= evidence.not_before_ns {
            return Err(SessionEvidenceError::InvalidValidityWindow);
        }
        let session_lifetime = evidence.expires_at_ns - evidence.not_before_ns;
        if session_lifetime > policy.maximum_session_validity_ns {
            return Err(SessionEvidenceError::SessionLifetimeExceeded);
        }
        if now_ns < evidence.not_before_ns {
            return Err(SessionEvidenceError::SessionNotYetValid);
        }
        if now_ns >= evidence.expires_at_ns {
            return Err(SessionEvidenceError::SessionExpired);
        }
        if evidence.verification.verifier_id != policy.trusted_verifier_id {
            return Err(SessionEvidenceError::UntrustedVerifier);
        }
        if evidence.verification.result != CryptoVerificationResult::Verified {
            return Err(SessionEvidenceError::CryptographicVerificationFailed);
        }
        if evidence.sts_mode == StsMode::Static {
            return Err(SessionEvidenceError::WeakStsMode);
        }
        if evidence.verification.protocol_profile_id != evidence.protocol_profile_id
            || evidence.verification.time_domain_id != evidence.time_domain_id
            || evidence.verification.initiator_id != evidence.initiator_id
            || evidence.verification.responder_id != evidence.responder_id
            || evidence.verification.session_id != evidence.session_id
            || evidence.verification.sub_session_id != evidence.sub_session_id
            || evidence.verification.key_epoch != evidence.key_epoch
            || evidence.verification.measurement_sequence != evidence.measurement_sequence
            || evidence.verification.nonce != evidence.nonce
        {
            return Err(SessionEvidenceError::ReceiptBindingMismatch);
        }
        if evidence.verification.transcript_digest.iter().all(|byte| *byte == 0) {
            return Err(SessionEvidenceError::MissingTranscriptDigest);
        }
        if evidence.verification.receipt_digest.iter().all(|byte| *byte == 0) {
            return Err(SessionEvidenceError::MissingReceiptDigest);
        }
        if evidence.verification.verified_at_ns > now_ns {
            return Err(SessionEvidenceError::VerificationReceiptFromFuture);
        }
        if now_ns - evidence.verification.verified_at_ns
            > policy.maximum_verification_receipt_age_ns
        {
            return Err(SessionEvidenceError::VerificationReceiptStale);
        }
        if evidence.verification.verified_at_ns < evidence.not_before_ns
            || evidence.verification.verified_at_ns >= evidence.expires_at_ns
        {
            return Err(SessionEvidenceError::ReceiptBindingMismatch);
        }

        let peers = PeerPair {
            initiator_id: evidence.initiator_id,
            responder_id: evidence.responder_id,
        };
        let stream = SessionStream {
            peers,
            session_id: evidence.session_id,
            sub_session_id: evidence.sub_session_id,
        };

        if self
            .latest_key_epoch_by_peer_pair
            .get(&peers)
            .is_some_and(|latest| evidence.key_epoch < *latest)
        {
            return Err(SessionEvidenceError::KeyEpochRollback);
        }
        if self
            .latest_sequence_by_session_stream
            .get(&stream)
            .is_some_and(|latest| evidence.measurement_sequence <= *latest)
        {
            return Err(SessionEvidenceError::MeasurementSequenceReplay);
        }
        if self.seen_nonces.contains(&evidence.nonce) {
            return Err(SessionEvidenceError::NonceReplay);
        }
        if self
            .seen_receipt_digests
            .contains(&evidence.verification.receipt_digest)
        {
            return Err(SessionEvidenceError::VerificationReceiptReplay);
        }

        // Commit replay state only after every validation succeeds.
        self.latest_key_epoch_by_peer_pair
            .entry(peers)
            .and_modify(|latest| *latest = (*latest).max(evidence.key_epoch))
            .or_insert(evidence.key_epoch);
        self.latest_sequence_by_session_stream
            .insert(stream, evidence.measurement_sequence);
        self.seen_nonces.insert(evidence.nonce);
        self.seen_receipt_digests
            .insert(evidence.verification.receipt_digest);

        Ok(VerifiedRangingSessionGrant {
            protocol_profile_id: evidence.protocol_profile_id,
            initiator_id: evidence.initiator_id,
            responder_id: evidence.responder_id,
            session_id: evidence.session_id,
            sub_session_id: evidence.sub_session_id,
            key_epoch: evidence.key_epoch,
            measurement_sequence: evidence.measurement_sequence,
            sts_mode: evidence.sts_mode,
            transcript_digest: evidence.verification.transcript_digest,
            verification_receipt_digest: evidence.verification.receipt_digest,
            valid_until_ns: evidence.expires_at_ns,
        })
    }
}

fn validate_policy(policy: SessionEvidencePolicy) -> Result<(), SessionEvidenceError> {
    if policy.trusted_verifier_id == 0
        || policy.expected_protocol_profile_id == 0
        || policy.expected_time_domain_id == 0
        || policy.expected_initiator_id == 0
        || policy.expected_responder_id == 0
        || policy.expected_initiator_id == policy.expected_responder_id
        || policy.maximum_session_validity_ns == 0
        || policy.maximum_verification_receipt_age_ns == 0
    {
        return Err(SessionEvidenceError::InvalidPolicy);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> SessionEvidencePolicy {
        SessionEvidencePolicy {
            trusted_verifier_id: 9,
            expected_protocol_profile_id: 41,
            expected_time_domain_id: 7,
            expected_initiator_id: 10,
            expected_responder_id: 20,
            maximum_session_validity_ns: 1_000,
            maximum_verification_receipt_age_ns: 50,
        }
    }

    fn evidence(sequence: u64, nonce_value: u8, receipt_value: u8) -> RangingSessionEvidence {
        let session_id = [1_u8; 16];
        let nonce = [nonce_value; 16];
        let transcript_digest = [2_u8; 32];
        let receipt_digest = [receipt_value; 32];
        RangingSessionEvidence {
            protocol_profile_id: 41,
            time_domain_id: 7,
            initiator_id: 10,
            responder_id: 20,
            session_id,
            sub_session_id: 2,
            key_epoch: 3,
            measurement_sequence: sequence,
            sts_mode: StsMode::Dynamic,
            nonce,
            not_before_ns: 100,
            expires_at_ns: 200,
            verification: CryptoVerificationReceipt {
                verifier_id: 9,
                result: CryptoVerificationResult::Verified,
                protocol_profile_id: 41,
                time_domain_id: 7,
                initiator_id: 10,
                responder_id: 20,
                session_id,
                sub_session_id: 2,
                key_epoch: 3,
                measurement_sequence: sequence,
                nonce,
                transcript_digest,
                receipt_digest,
                verified_at_ns: 140,
            },
        }
    }

    #[test]
    fn accepts_fresh_dynamic_sts_evidence_from_trusted_verifier() {
        let mut validator = SessionEvidenceValidator::new();
        let grant = validator.validate(policy(), evidence(1, 3, 4), 150).unwrap();
        assert_eq!(grant.initiator_id, 10);
        assert_eq!(grant.responder_id, 20);
        assert_eq!(grant.sts_mode, StsMode::Dynamic);
        assert_eq!(grant.valid_until_ns, 200);
    }

    #[test]
    fn accepts_provisioned_sts_when_policy_allows_secure_sts() {
        let mut validator = SessionEvidenceValidator::new();
        let mut event = evidence(1, 3, 4);
        event.sts_mode = StsMode::Provisioned;
        let grant = validator.validate(policy(), event, 150).unwrap();
        assert_eq!(grant.sts_mode, StsMode::Provisioned);
    }

    #[test]
    fn rejects_static_sts_mode() {
        let mut validator = SessionEvidenceValidator::new();
        let mut event = evidence(1, 3, 4);
        event.sts_mode = StsMode::Static;
        assert_eq!(
            validator.validate(policy(), event, 150),
            Err(SessionEvidenceError::WeakStsMode)
        );
    }

    #[test]
    fn rejects_wrong_verifier_or_nonverified_result() {
        let mut validator = SessionEvidenceValidator::new();
        let mut event = evidence(1, 3, 4);
        event.verification.verifier_id = 8;
        assert_eq!(
            validator.validate(policy(), event, 150),
            Err(SessionEvidenceError::UntrustedVerifier)
        );

        let mut event = evidence(1, 3, 4);
        event.verification.result = CryptoVerificationResult::Unavailable;
        assert_eq!(
            validator.validate(policy(), event, 150),
            Err(SessionEvidenceError::CryptographicVerificationFailed)
        );
    }

    #[test]
    fn rejects_identity_and_protocol_profile_mismatch() {
        let mut validator = SessionEvidenceValidator::new();
        let mut event = evidence(1, 3, 4);
        event.responder_id = 21;
        assert_eq!(
            validator.validate(policy(), event, 150),
            Err(SessionEvidenceError::WrongResponder)
        );

        let mut event = evidence(1, 3, 4);
        event.protocol_profile_id = 5;
        assert_eq!(
            validator.validate(policy(), event, 150),
            Err(SessionEvidenceError::WrongProtocolProfile)
        );
    }

    #[test]
    fn rejects_mismatched_cryptographic_receipt_binding() {
        let mut validator = SessionEvidenceValidator::new();
        let mut event = evidence(1, 3, 4);
        event.verification.responder_id = 99;
        assert_eq!(
            validator.validate(policy(), event, 150),
            Err(SessionEvidenceError::ReceiptBindingMismatch)
        );
    }

    #[test]
    fn rejects_expired_future_and_overlong_session_windows() {
        let mut validator = SessionEvidenceValidator::new();
        assert_eq!(
            validator.validate(policy(), evidence(1, 3, 4), 200),
            Err(SessionEvidenceError::SessionExpired)
        );
        assert_eq!(
            validator.validate(policy(), evidence(1, 3, 4), 99),
            Err(SessionEvidenceError::SessionNotYetValid)
        );

        let mut event = evidence(1, 3, 4);
        event.expires_at_ns = 1_101;
        assert_eq!(
            validator.validate(policy(), event, 150),
            Err(SessionEvidenceError::SessionLifetimeExceeded)
        );
    }

    #[test]
    fn rejects_stale_or_future_verification_receipt() {
        let mut validator = SessionEvidenceValidator::new();
        let mut event = evidence(1, 3, 4);
        event.verification.verified_at_ns = 99;
        assert_eq!(
            validator.validate(policy(), event, 149),
            Err(SessionEvidenceError::ReceiptBindingMismatch)
        );

        let mut event = evidence(1, 3, 4);
        event.verification.verified_at_ns = 151;
        assert_eq!(
            validator.validate(policy(), event, 150),
            Err(SessionEvidenceError::VerificationReceiptFromFuture)
        );

        let mut event = evidence(1, 3, 4);
        event.verification.verified_at_ns = 80;
        event.not_before_ns = 70;
        assert_eq!(
            validator.validate(policy(), event, 150),
            Err(SessionEvidenceError::VerificationReceiptStale)
        );
    }

    #[test]
    fn rejects_nonce_receipt_and_measurement_sequence_replay() {
        let mut validator = SessionEvidenceValidator::new();
        validator.validate(policy(), evidence(1, 3, 4), 150).unwrap();

        assert_eq!(
            validator.validate(policy(), evidence(2, 3, 5), 150),
            Err(SessionEvidenceError::NonceReplay)
        );
        assert_eq!(
            validator.validate(policy(), evidence(2, 4, 4), 150),
            Err(SessionEvidenceError::VerificationReceiptReplay)
        );
        assert_eq!(
            validator.validate(policy(), evidence(1, 5, 6), 150),
            Err(SessionEvidenceError::MeasurementSequenceReplay)
        );
    }

    #[test]
    fn rejects_key_epoch_rollback_but_accepts_forward_rotation() {
        let mut validator = SessionEvidenceValidator::new();
        validator.validate(policy(), evidence(1, 3, 4), 150).unwrap();

        let mut forward = evidence(2, 4, 5);
        forward.key_epoch = 4;
        forward.verification.key_epoch = 4;
        validator.validate(policy(), forward, 150).unwrap();

        let mut rollback = evidence(3, 5, 6);
        rollback.key_epoch = 2;
        rollback.verification.key_epoch = 2;
        assert_eq!(
            validator.validate(policy(), rollback, 150),
            Err(SessionEvidenceError::KeyEpochRollback)
        );
    }

    #[test]
    fn rejected_evidence_does_not_advance_replay_state() {
        let mut validator = SessionEvidenceValidator::new();
        let mut invalid = evidence(1, 3, 4);
        invalid.verification.result = CryptoVerificationResult::Invalid;
        assert_eq!(
            validator.validate(policy(), invalid, 150),
            Err(SessionEvidenceError::CryptographicVerificationFailed)
        );
        assert!(validator.validate(policy(), evidence(1, 3, 4), 150).is_ok());
    }
}
