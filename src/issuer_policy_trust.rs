// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Explicit local caller trust bootstrap for canonical issuer-authorization policy.
//!
//! This module lives in the application/integration layer deliberately. The
//! low-level physics policy codec proves canonical representation only; it does
//! not grant policy trust. Calling this API is an explicit local caller decision
//! to trust one exact snapshot. It does **not** authenticate which human,
//! operator, deployment principal, or module made that call; stronger caller
//! authorization belongs to PHYS-EVID-03B1B2.

use std::fmt;

use symtropy_physics::issuer_policy::{
    decode_issuer_authorization_policy_v1, DecodedIssuerAuthorizationPolicyV1,
    IssuerPolicyV1Error,
};

pub const LOCAL_ISSUER_POLICY_TRUST_BOOTSTRAP_PROFILE_V1: &str =
    "local-explicit-policy-bootstrap-v1";

/// One-shot local marker representing an explicit caller decision to trust one
/// exact canonical issuer-policy snapshot.
///
/// The marker is intentionally neither `Clone` nor `Copy` and has no
/// serialization surface. It is **not** an unforgeable operator/deployment
/// capability: any code with access to this public integration API may begin a
/// local bootstrap. PHYS-EVID-03B1B2 is reserved for independently proving who
/// was authorized to perform that action.
#[derive(Debug)]
pub struct LocalIssuerPolicyTrustBootstrap {
    _private: (),
}

/// An immutable issuer-policy snapshot trusted by one explicit local caller
/// under `local-explicit-policy-bootstrap-v1`.
///
/// The `LocallyTrusted` name is intentional: this type proves only that an
/// explicit local call selected these exact B1A canonical bytes. It does not
/// authenticate the caller and does not prove deployment authorization, global
/// policy freshness, chronology, signature validity, or issuer authorization.
#[derive(Debug)]
pub struct LocallyTrustedIssuerAuthorizationPolicyV1 {
    canonical_policy_bytes: Vec<u8>,
    policy: DecodedIssuerAuthorizationPolicyV1,
    bootstrap_profile: &'static str,
}

impl LocallyTrustedIssuerAuthorizationPolicyV1 {
    /// Exact canonical B1A bytes selected by the local bootstrap action.
    pub fn canonical_policy_bytes(&self) -> &[u8] {
        &self.canonical_policy_bytes
    }

    /// Read-only admitted policy semantics corresponding to the exact bytes.
    pub fn policy(&self) -> &DecodedIssuerAuthorizationPolicyV1 {
        &self.policy
    }

    /// Stable local bootstrap profile label. This is provenance metadata, not
    /// trusted time, caller identity, or a semantic policy version.
    pub const fn bootstrap_profile(&self) -> &'static str {
        self.bootstrap_profile
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalIssuerPolicyTrustBootstrapError {
    Policy(IssuerPolicyV1Error),
    CanonicalRoundTripMismatch,
}

impl fmt::Display for LocalIssuerPolicyTrustBootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Policy(error) => write!(f, "issuer policy rejected before trust bootstrap: {error}"),
            Self::CanonicalRoundTripMismatch => {
                write!(f, "decoded issuer policy did not round-trip to the exact supplied bytes")
            }
        }
    }
}

impl std::error::Error for LocalIssuerPolicyTrustBootstrapError {}

impl From<IssuerPolicyV1Error> for LocalIssuerPolicyTrustBootstrapError {
    fn from(error: IssuerPolicyV1Error) -> Self {
        Self::Policy(error)
    }
}

/// Begin one explicit local caller policy-trust bootstrap action.
///
/// This function does not inspect a file path, environment variable, mtime, or
/// remote source. The caller must explicitly supply the exact candidate bytes
/// to the returned one-shot marker.
///
/// This public zero-argument constructor intentionally does **not** prove caller
/// identity or deployment/operator authorization. Such provenance must be added
/// by a separate PHYS-EVID-03B1B2 profile when required by the deployment.
pub fn begin_local_issuer_policy_trust_bootstrap_v1() -> LocalIssuerPolicyTrustBootstrap {
    LocalIssuerPolicyTrustBootstrap { _private: () }
}

impl LocalIssuerPolicyTrustBootstrap {
    /// Consume this one-shot marker and trust exactly one already-canonical B1A
    /// policy snapshot for the current local caller context.
    ///
    /// The candidate is decoded with the B1A fail-closed canonical decoder and
    /// then re-encoded. Exact byte equality is required before the locally
    /// trusted wrapper can be constructed.
    pub fn trust_exact_policy_bytes(
        self,
        candidate: &[u8],
    ) -> Result<LocallyTrustedIssuerAuthorizationPolicyV1, LocalIssuerPolicyTrustBootstrapError> {
        let policy = decode_issuer_authorization_policy_v1(candidate)?;
        let canonical_policy_bytes = policy.to_canonical_bytes();
        if canonical_policy_bytes.as_slice() != candidate {
            return Err(LocalIssuerPolicyTrustBootstrapError::CanonicalRoundTripMismatch);
        }

        Ok(LocallyTrustedIssuerAuthorizationPolicyV1 {
            canonical_policy_bytes,
            policy,
            bootstrap_profile: LOCAL_ISSUER_POLICY_TRUST_BOOTSTRAP_PROFILE_V1,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_physics::issuer_policy::{
        encode_issuer_authorization_policy_v1, IssuerAttestationSuiteV1,
        IssuerAuthorizationEntryDraftV1, IssuerAuthorizationPolicyDraftV1,
        IssuerClaimProfileV1, IssuerCredentialStatusV1, IssuerCryptoPolicyProfileV1,
        IssuerSessionProfileV1,
    };

    fn canonical_policy(revision: u64) -> Vec<u8> {
        encode_issuer_authorization_policy_v1(&IssuerAuthorizationPolicyDraftV1 {
            policy_revision: revision,
            entries: vec![IssuerAuthorizationEntryDraftV1 {
                credential_id: [0x11; 32],
                status: IssuerCredentialStatusV1::Active,
                physical_authority_ids: vec![2, 1],
                claim_profiles: vec![
                    IssuerClaimProfileV1::ConsecutivePresence,
                    IssuerClaimProfileV1::EndpointSample,
                ],
                session_profiles: vec![IssuerSessionProfileV1::OsCsprng],
                attestation_suites: vec![IssuerAttestationSuiteV1::HybridEd25519MlDsa65],
                crypto_policy_profiles: vec![IssuerCryptoPolicyProfileV1::HybridPqcAuth],
            }],
        })
        .expect("valid canonical policy")
    }

    #[test]
    fn explicit_bootstrap_retains_exact_canonical_snapshot() {
        let bytes = canonical_policy(7);
        let trusted = begin_local_issuer_policy_trust_bootstrap_v1()
            .trust_exact_policy_bytes(&bytes)
            .expect("explicit bootstrap should accept canonical bytes");

        assert_eq!(trusted.canonical_policy_bytes(), bytes.as_slice());
        assert_eq!(trusted.policy().policy_revision(), 7);
        assert_eq!(
            trusted.bootstrap_profile(),
            LOCAL_ISSUER_POLICY_TRUST_BOOTSTRAP_PROFILE_V1
        );
    }

    #[test]
    fn source_buffer_mutation_after_bootstrap_cannot_mutate_trusted_snapshot() {
        let mut bytes = canonical_policy(7);
        let original = bytes.clone();
        let trusted = begin_local_issuer_policy_trust_bootstrap_v1()
            .trust_exact_policy_bytes(&bytes)
            .expect("bootstrap should succeed");

        bytes[0] ^= 0xff;
        assert_eq!(trusted.canonical_policy_bytes(), original.as_slice());
    }

    #[test]
    fn malformed_or_noncanonical_candidate_never_produces_trusted_type() {
        let mut bytes = canonical_policy(7);
        bytes.push(0);
        assert!(
            begin_local_issuer_policy_trust_bootstrap_v1()
                .trust_exact_policy_bytes(&bytes)
                .is_err()
        );
    }

    #[test]
    fn same_exact_snapshot_bootstrapped_twice_preserves_same_policy_semantics() {
        let bytes = canonical_policy(7);
        let a = begin_local_issuer_policy_trust_bootstrap_v1()
            .trust_exact_policy_bytes(&bytes)
            .expect("first bootstrap");
        let b = begin_local_issuer_policy_trust_bootstrap_v1()
            .trust_exact_policy_bytes(&bytes)
            .expect("second bootstrap");

        assert_eq!(a.canonical_policy_bytes(), b.canonical_policy_bytes());
        assert_eq!(a.policy(), b.policy());
    }

    #[test]
    fn revision_change_is_a_different_snapshot_without_implying_time_order() {
        let a = begin_local_issuer_policy_trust_bootstrap_v1()
            .trust_exact_policy_bytes(&canonical_policy(7))
            .expect("revision 7");
        let b = begin_local_issuer_policy_trust_bootstrap_v1()
            .trust_exact_policy_bytes(&canonical_policy(8))
            .expect("revision 8");

        assert_ne!(a.canonical_policy_bytes(), b.canonical_policy_bytes());
        assert_eq!(a.policy().policy_revision(), 7);
        assert_eq!(b.policy().policy_revision(), 8);
    }
}
