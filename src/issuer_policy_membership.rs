// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic membership evaluation for one exact locally trusted
//! issuer-policy snapshot.
//!
//! This module does not verify cryptography and does not construct issuer-authorized
//! physics evidence. It answers only whether one plain query is allowed by one
//! `LocallyTrustedIssuerAuthorizationPolicyV1` snapshot.

use symtropy_physics::issuer_policy::{
    IssuerAttestationSuiteV1, IssuerClaimProfileV1, IssuerCredentialStatusV1,
    IssuerCryptoPolicyProfileV1, IssuerSessionProfileV1,
};

use crate::issuer_policy_trust::LocallyTrustedIssuerAuthorizationPolicyV1;

/// Plain authorization query. These fields are not authenticated merely because
/// this structure exists; later B1 composition must derive them from B0-verified
/// canonical evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IssuerAuthorizationQueryV1 {
    pub credential_id: [u8; 32],
    pub physical_authority_id: u128,
    pub claim_profile: IssuerClaimProfileV1,
    pub session_profile: IssuerSessionProfileV1,
    pub attestation_suite: IssuerAttestationSuiteV1,
    pub crypto_policy_profile: IssuerCryptoPolicyProfileV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssuerPolicyScopeDenyReasonV1 {
    UnknownCredential,
    CredentialDisabled,
    CredentialRevokedCompromised,
    ZeroPhysicalAuthorityId,
    PhysicalAuthorityNotAllowed,
    ClaimProfileNotAllowed,
    SessionProfileNotAllowed,
    AttestationSuiteNotAllowed,
    CryptoPolicyProfileNotAllowed,
}

/// Proof only of exact scope membership under one borrowed locally trusted
/// policy snapshot.
///
/// This is intentionally not an issuer-authorized evidence type. The query remains
/// plain data until a later B1 composition proves it came from a B0-valid claim.
#[derive(Debug)]
pub struct IssuerPolicyScopeMatchV1<'a> {
    trusted_policy: &'a LocallyTrustedIssuerAuthorizationPolicyV1,
    query: IssuerAuthorizationQueryV1,
}

impl<'a> IssuerPolicyScopeMatchV1<'a> {
    pub const fn query(&self) -> IssuerAuthorizationQueryV1 {
        self.query
    }

    pub fn trusted_policy(&self) -> &'a LocallyTrustedIssuerAuthorizationPolicyV1 {
        self.trusted_policy
    }

    pub fn canonical_policy_bytes(&self) -> &[u8] {
        self.trusted_policy.canonical_policy_bytes()
    }

    pub fn policy_revision(&self) -> u64 {
        self.trusted_policy.policy().policy_revision()
    }
}

/// Evaluate one plain query against one exact locally trusted policy snapshot.
///
/// Evaluation order is stable and fail-closed. Successful return establishes only
/// policy scope membership for the supplied field values.
pub fn evaluate_issuer_policy_scope_v1<'a>(
    trusted_policy: &'a LocallyTrustedIssuerAuthorizationPolicyV1,
    query: &IssuerAuthorizationQueryV1,
) -> Result<IssuerPolicyScopeMatchV1<'a>, IssuerPolicyScopeDenyReasonV1> {
    let entries = trusted_policy.policy().entries();
    let entry_index = entries
        .binary_search_by(|entry| entry.credential_id().cmp(&query.credential_id))
        .map_err(|_| IssuerPolicyScopeDenyReasonV1::UnknownCredential)?;
    let entry = &entries[entry_index];

    match entry.status() {
        IssuerCredentialStatusV1::Active => {}
        IssuerCredentialStatusV1::Disabled => {
            return Err(IssuerPolicyScopeDenyReasonV1::CredentialDisabled);
        }
        IssuerCredentialStatusV1::RevokedCompromised => {
            return Err(IssuerPolicyScopeDenyReasonV1::CredentialRevokedCompromised);
        }
    }

    if query.physical_authority_id == 0 {
        return Err(IssuerPolicyScopeDenyReasonV1::ZeroPhysicalAuthorityId);
    }
    if entry
        .physical_authority_ids()
        .binary_search(&query.physical_authority_id)
        .is_err()
    {
        return Err(IssuerPolicyScopeDenyReasonV1::PhysicalAuthorityNotAllowed);
    }
    if entry
        .claim_profiles()
        .binary_search(&query.claim_profile)
        .is_err()
    {
        return Err(IssuerPolicyScopeDenyReasonV1::ClaimProfileNotAllowed);
    }
    if entry
        .session_profiles()
        .binary_search(&query.session_profile)
        .is_err()
    {
        return Err(IssuerPolicyScopeDenyReasonV1::SessionProfileNotAllowed);
    }
    if entry
        .attestation_suites()
        .binary_search(&query.attestation_suite)
        .is_err()
    {
        return Err(IssuerPolicyScopeDenyReasonV1::AttestationSuiteNotAllowed);
    }
    if entry
        .crypto_policy_profiles()
        .binary_search(&query.crypto_policy_profile)
        .is_err()
    {
        return Err(IssuerPolicyScopeDenyReasonV1::CryptoPolicyProfileNotAllowed);
    }

    Ok(IssuerPolicyScopeMatchV1 {
        trusted_policy,
        query: *query,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::issuer_policy_trust::begin_local_issuer_policy_trust_bootstrap_v1;
    use symtropy_physics::issuer_policy::{
        encode_issuer_authorization_policy_v1, IssuerAuthorizationEntryDraftV1,
        IssuerAuthorizationPolicyDraftV1,
    };

    fn trusted_policy(revision: u64) -> LocallyTrustedIssuerAuthorizationPolicyV1 {
        let bytes = encode_issuer_authorization_policy_v1(&IssuerAuthorizationPolicyDraftV1 {
            policy_revision: revision,
            entries: vec![
                IssuerAuthorizationEntryDraftV1 {
                    credential_id: [0x11; 32],
                    status: IssuerCredentialStatusV1::Active,
                    physical_authority_ids: vec![9, 3],
                    claim_profiles: vec![
                        IssuerClaimProfileV1::EndpointSample,
                        IssuerClaimProfileV1::ConsecutivePresence,
                    ],
                    session_profiles: vec![IssuerSessionProfileV1::OsCsprng],
                    attestation_suites: vec![IssuerAttestationSuiteV1::HybridEd25519MlDsa65],
                    crypto_policy_profiles: vec![IssuerCryptoPolicyProfileV1::HybridPqcAuth],
                },
                IssuerAuthorizationEntryDraftV1 {
                    credential_id: [0x22; 32],
                    status: IssuerCredentialStatusV1::Disabled,
                    physical_authority_ids: vec![3],
                    claim_profiles: vec![IssuerClaimProfileV1::EndpointSample],
                    session_profiles: vec![IssuerSessionProfileV1::OsCsprng],
                    attestation_suites: vec![IssuerAttestationSuiteV1::HybridEd25519MlDsa65],
                    crypto_policy_profiles: vec![IssuerCryptoPolicyProfileV1::HybridPqcAuth],
                },
                IssuerAuthorizationEntryDraftV1 {
                    credential_id: [0x33; 32],
                    status: IssuerCredentialStatusV1::RevokedCompromised,
                    physical_authority_ids: vec![3],
                    claim_profiles: vec![IssuerClaimProfileV1::EndpointSample],
                    session_profiles: vec![IssuerSessionProfileV1::OsCsprng],
                    attestation_suites: vec![IssuerAttestationSuiteV1::HybridEd25519MlDsa65],
                    crypto_policy_profiles: vec![IssuerCryptoPolicyProfileV1::HybridPqcAuth],
                },
            ],
        })
        .expect("valid policy");

        begin_local_issuer_policy_trust_bootstrap_v1()
            .trust_exact_policy_bytes(&bytes)
            .expect("explicit local trust bootstrap")
    }

    fn endpoint_only_policy() -> LocallyTrustedIssuerAuthorizationPolicyV1 {
        let bytes = encode_issuer_authorization_policy_v1(&IssuerAuthorizationPolicyDraftV1 {
            policy_revision: 7,
            entries: vec![IssuerAuthorizationEntryDraftV1 {
                credential_id: [0x11; 32],
                status: IssuerCredentialStatusV1::Active,
                physical_authority_ids: vec![3],
                claim_profiles: vec![IssuerClaimProfileV1::EndpointSample],
                session_profiles: vec![IssuerSessionProfileV1::OsCsprng],
                attestation_suites: vec![IssuerAttestationSuiteV1::HybridEd25519MlDsa65],
                crypto_policy_profiles: vec![IssuerCryptoPolicyProfileV1::HybridPqcAuth],
            }],
        })
        .expect("valid endpoint-only policy");

        begin_local_issuer_policy_trust_bootstrap_v1()
            .trust_exact_policy_bytes(&bytes)
            .expect("explicit local trust bootstrap")
    }

    fn allowed_query() -> IssuerAuthorizationQueryV1 {
        IssuerAuthorizationQueryV1 {
            credential_id: [0x11; 32],
            physical_authority_id: 3,
            claim_profile: IssuerClaimProfileV1::EndpointSample,
            session_profile: IssuerSessionProfileV1::OsCsprng,
            attestation_suite: IssuerAttestationSuiteV1::HybridEd25519MlDsa65,
            crypto_policy_profile: IssuerCryptoPolicyProfileV1::HybridPqcAuth,
        }
    }

    #[test]
    fn exact_active_query_matches_and_binds_snapshot() {
        let policy = trusted_policy(7);
        let query = allowed_query();
        let matched = evaluate_issuer_policy_scope_v1(&policy, &query).expect("scope match");
        assert_eq!(matched.query(), query);
        assert_eq!(matched.policy_revision(), 7);
        assert_eq!(matched.canonical_policy_bytes(), policy.canonical_policy_bytes());
    }

    #[test]
    fn either_exact_authority_member_can_match() {
        let policy = trusted_policy(7);
        let mut query = allowed_query();
        query.physical_authority_id = 9;
        assert!(evaluate_issuer_policy_scope_v1(&policy, &query).is_ok());
    }

    #[test]
    fn unknown_disabled_and_revoked_credentials_are_distinct_denials() {
        let policy = trusted_policy(7);
        let mut query = allowed_query();

        query.credential_id = [0x44; 32];
        assert_eq!(
            evaluate_issuer_policy_scope_v1(&policy, &query).unwrap_err(),
            IssuerPolicyScopeDenyReasonV1::UnknownCredential
        );

        query.credential_id = [0x22; 32];
        assert_eq!(
            evaluate_issuer_policy_scope_v1(&policy, &query).unwrap_err(),
            IssuerPolicyScopeDenyReasonV1::CredentialDisabled
        );

        query.credential_id = [0x33; 32];
        assert_eq!(
            evaluate_issuer_policy_scope_v1(&policy, &query).unwrap_err(),
            IssuerPolicyScopeDenyReasonV1::CredentialRevokedCompromised
        );
    }

    #[test]
    fn zero_and_wrong_authority_fail_closed() {
        let policy = trusted_policy(7);
        let mut query = allowed_query();
        query.physical_authority_id = 0;
        assert_eq!(
            evaluate_issuer_policy_scope_v1(&policy, &query).unwrap_err(),
            IssuerPolicyScopeDenyReasonV1::ZeroPhysicalAuthorityId
        );
        query.physical_authority_id = 4;
        assert_eq!(
            evaluate_issuer_policy_scope_v1(&policy, &query).unwrap_err(),
            IssuerPolicyScopeDenyReasonV1::PhysicalAuthorityNotAllowed
        );
    }

    #[test]
    fn wrong_claim_profile_is_denied_exactly() {
        let policy = endpoint_only_policy();
        let mut query = allowed_query();
        query.claim_profile = IssuerClaimProfileV1::ConsecutivePresence;
        assert_eq!(
            evaluate_issuer_policy_scope_v1(&policy, &query).unwrap_err(),
            IssuerPolicyScopeDenyReasonV1::ClaimProfileNotAllowed
        );
    }

    #[test]
    fn exact_query_is_deterministic_under_one_snapshot() {
        let policy = trusted_policy(7);
        let query = allowed_query();
        let a = evaluate_issuer_policy_scope_v1(&policy, &query).expect("match a");
        let b = evaluate_issuer_policy_scope_v1(&policy, &query).expect("match b");
        assert_eq!(a.query(), b.query());
        assert_eq!(a.canonical_policy_bytes(), b.canonical_policy_bytes());
    }

    #[test]
    fn revision_only_change_changes_snapshot_not_query_semantics() {
        let p7 = trusted_policy(7);
        let p8 = trusted_policy(8);
        let query = allowed_query();
        let m7 = evaluate_issuer_policy_scope_v1(&p7, &query).expect("revision 7 match");
        let m8 = evaluate_issuer_policy_scope_v1(&p8, &query).expect("revision 8 match");
        assert_ne!(m7.canonical_policy_bytes(), m8.canonical_policy_bytes());
        assert_eq!(m7.query(), m8.query());
    }
}
