// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Verifier-bound semantic claims for executable Q2 shadow runner qualification.
//!
//! `shadow_runner_qualification` resolves reviewed receipts against provider-neutral
//! provenance and bootstrap runner identity. This sibling layer closes one further
//! interpretation gap: the externally verified claim must itself bind the exact
//! semantic-contract version, hostile fixture, emitted transcript and source-state
//! manifest grammar carried by the reviewed receipt.
//!
//! The separation is intentional. A cryptographically valid provenance envelope
//! cannot be upgraded into a stronger scientific proposition merely by attaching a
//! richer reviewed receipt after verification. Both authorities must remain current.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::information::EvidenceLineageToken;

use super::shadow_execution_lineage::ShadowReferenceRunnerQualification;
use super::shadow_paired_execution::ShadowCoarseRunnerQualification;
use super::shadow_runner_qualification::{
    ExecutableShadowRunnerPairAuthority, ExecutableShadowRunnerQualification,
    QualificationFixtureDigest, QualificationTranscriptDigest,
    ReviewedShadowRunnerQualificationStatus, ShadowRunnerQualificationError,
    ShadowRunnerQualificationReceiptKey, ShadowRunnerQualificationReceiptRevision,
    ShadowRunnerQualificationRegistry, ShadowRunnerSemanticContractVersion,
    ShadowStateManifestGrammarFingerprint, VerifiedQualificationProvenance,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowRunnerSemanticRegistryKey {
    id: u128,
    version: u32,
}

impl ShadowRunnerSemanticRegistryKey {
    pub const fn new(id: u128, version: u32) -> Self {
        Self { id, version }
    }

    pub const fn id(self) -> u128 {
        self.id
    }

    pub const fn version(self) -> u32 {
        self.version
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifiedRunnerSemanticClaimStatus {
    Current,
    Revoked,
    Superseded,
}

/// Canonically ingested result of verifying the *semantic contents* of one exact
/// qualification receipt's external evidence.
///
/// This is still evidence-shaped input. Authority comes from the sealed registry
/// plus exact equality with a current executable qualification receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedRunnerSemanticClaim {
    receipt: ShadowRunnerQualificationReceiptKey,
    receipt_revision: ShadowRunnerQualificationReceiptRevision,
    provenance: VerifiedQualificationProvenance,
    semantic_contract: ShadowRunnerSemanticContractVersion,
    fixture: QualificationFixtureDigest,
    transcript: QualificationTranscriptDigest,
    state_manifest_grammar: ShadowStateManifestGrammarFingerprint,
    semantic_verification_evidence: EvidenceLineageToken,
    status: VerifiedRunnerSemanticClaimStatus,
}

impl VerifiedRunnerSemanticClaim {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        receipt: ShadowRunnerQualificationReceiptKey,
        receipt_revision: ShadowRunnerQualificationReceiptRevision,
        provenance: VerifiedQualificationProvenance,
        semantic_contract: ShadowRunnerSemanticContractVersion,
        fixture: QualificationFixtureDigest,
        transcript: QualificationTranscriptDigest,
        state_manifest_grammar: ShadowStateManifestGrammarFingerprint,
        semantic_verification_evidence: EvidenceLineageToken,
        status: VerifiedRunnerSemanticClaimStatus,
    ) -> Self {
        Self {
            receipt,
            receipt_revision,
            provenance,
            semantic_contract,
            fixture,
            transcript,
            state_manifest_grammar,
            semantic_verification_evidence,
            status,
        }
    }

    pub const fn receipt(&self) -> ShadowRunnerQualificationReceiptKey {
        self.receipt
    }

    pub const fn receipt_revision(&self) -> ShadowRunnerQualificationReceiptRevision {
        self.receipt_revision
    }

    pub const fn provenance(&self) -> &VerifiedQualificationProvenance {
        &self.provenance
    }

    pub const fn semantic_contract(&self) -> ShadowRunnerSemanticContractVersion {
        self.semantic_contract
    }

    pub const fn fixture(&self) -> &QualificationFixtureDigest {
        &self.fixture
    }

    pub const fn transcript(&self) -> &QualificationTranscriptDigest {
        &self.transcript
    }

    pub const fn state_manifest_grammar(&self) -> &ShadowStateManifestGrammarFingerprint {
        &self.state_manifest_grammar
    }

    pub const fn semantic_verification_evidence(&self) -> EvidenceLineageToken {
        self.semantic_verification_evidence
    }

    pub const fn status(&self) -> VerifiedRunnerSemanticClaimStatus {
        self.status
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowRunnerSemanticAuthorityStamp {
    key: ShadowRunnerSemanticRegistryKey,
    claims: BTreeMap<ShadowRunnerQualificationReceiptKey, VerifiedRunnerSemanticClaim>,
}

impl ShadowRunnerSemanticAuthorityStamp {
    pub const fn key(&self) -> ShadowRunnerSemanticRegistryKey {
        self.key
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowRunnerSemanticRegistryBuilder {
    key: ShadowRunnerSemanticRegistryKey,
    claims: BTreeMap<ShadowRunnerQualificationReceiptKey, VerifiedRunnerSemanticClaim>,
}

impl ShadowRunnerSemanticRegistryBuilder {
    pub const fn new(key: ShadowRunnerSemanticRegistryKey) -> Self {
        Self {
            key,
            claims: BTreeMap::new(),
        }
    }

    pub fn register_claim(
        &mut self,
        claim: VerifiedRunnerSemanticClaim,
    ) -> Result<(), ShadowRunnerSemanticProvenanceError> {
        use std::collections::btree_map::Entry;
        match self.claims.entry(claim.receipt()) {
            Entry::Vacant(entry) => {
                entry.insert(claim);
                Ok(())
            }
            Entry::Occupied(entry) if entry.get() == &claim => Ok(()),
            Entry::Occupied(entry) => Err(
                ShadowRunnerSemanticProvenanceError::ConflictingSemanticClaim {
                    receipt: *entry.key(),
                },
            ),
        }
    }

    pub fn seal(self) -> ShadowRunnerSemanticRegistry {
        let authority = ShadowRunnerSemanticAuthorityStamp {
            key: self.key,
            claims: self.claims.clone(),
        };
        ShadowRunnerSemanticRegistry {
            authority,
            claims: self.claims,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowRunnerSemanticRegistry {
    authority: ShadowRunnerSemanticAuthorityStamp,
    claims: BTreeMap<ShadowRunnerQualificationReceiptKey, VerifiedRunnerSemanticClaim>,
}

impl ShadowRunnerSemanticRegistry {
    pub const fn authority_stamp(&self) -> &ShadowRunnerSemanticAuthorityStamp {
        &self.authority
    }

    pub fn resolve(
        &self,
        executable: &ExecutableShadowRunnerQualification,
    ) -> Result<VerifierBoundExecutableShadowRunnerQualification, ShadowRunnerSemanticProvenanceError>
    {
        let receipt = executable.receipt();
        if receipt.status() != ReviewedShadowRunnerQualificationStatus::Current {
            return Err(ShadowRunnerSemanticProvenanceError::ReceiptNotCurrent {
                receipt: receipt.key(),
            });
        }
        let claim = self
            .claims
            .get(&receipt.key())
            .ok_or(ShadowRunnerSemanticProvenanceError::MissingSemanticClaim {
                receipt: receipt.key(),
            })?;
        validate_claim_against_receipt(claim, executable)?;

        Ok(VerifierBoundExecutableShadowRunnerQualification {
            authority: self.authority.clone(),
            executable: executable.clone(),
            claim: claim.clone(),
        })
    }

    pub fn resolve_pair(
        &self,
        pair: &ExecutableShadowRunnerPairAuthority,
    ) -> Result<VerifierBoundExecutableShadowRunnerPairAuthority, ShadowRunnerSemanticProvenanceError>
    {
        Ok(VerifierBoundExecutableShadowRunnerPairAuthority {
            semantic_authority: self.authority.clone(),
            executable_pair: pair.clone(),
            reference: self.resolve(pair.reference())?,
            coarse: self.resolve(pair.coarse())?,
        })
    }
}

fn validate_claim_against_receipt(
    claim: &VerifiedRunnerSemanticClaim,
    executable: &ExecutableShadowRunnerQualification,
) -> Result<(), ShadowRunnerSemanticProvenanceError> {
    let receipt = executable.receipt();
    if claim.status() != VerifiedRunnerSemanticClaimStatus::Current {
        return Err(ShadowRunnerSemanticProvenanceError::SemanticClaimNotCurrent {
            receipt: receipt.key(),
            status: claim.status(),
        });
    }
    if claim.receipt_revision() != receipt.revision() {
        return Err(ShadowRunnerSemanticProvenanceError::ReceiptRevisionMismatch {
            receipt: receipt.key(),
            expected: receipt.revision(),
            actual: claim.receipt_revision(),
        });
    }
    if claim.provenance() != receipt.provenance() {
        return Err(ShadowRunnerSemanticProvenanceError::ProvenanceMismatch {
            receipt: receipt.key(),
        });
    }
    if claim.semantic_contract() != receipt.semantic_contract() {
        return Err(ShadowRunnerSemanticProvenanceError::SemanticContractMismatch {
            receipt: receipt.key(),
        });
    }
    if claim.fixture() != receipt.fixture() {
        return Err(ShadowRunnerSemanticProvenanceError::FixtureMismatch {
            receipt: receipt.key(),
        });
    }
    if claim.transcript() != receipt.transcript() {
        return Err(ShadowRunnerSemanticProvenanceError::TranscriptMismatch {
            receipt: receipt.key(),
        });
    }
    if claim.state_manifest_grammar() != receipt.state_manifest_grammar() {
        return Err(ShadowRunnerSemanticProvenanceError::StateManifestGrammarMismatch {
            receipt: receipt.key(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifierBoundExecutableShadowRunnerQualification {
    authority: ShadowRunnerSemanticAuthorityStamp,
    executable: ExecutableShadowRunnerQualification,
    claim: VerifiedRunnerSemanticClaim,
}

impl VerifierBoundExecutableShadowRunnerQualification {
    pub const fn authority_stamp(&self) -> &ShadowRunnerSemanticAuthorityStamp {
        &self.authority
    }

    pub const fn executable(&self) -> &ExecutableShadowRunnerQualification {
        &self.executable
    }

    pub const fn claim(&self) -> &VerifiedRunnerSemanticClaim {
        &self.claim
    }

    pub fn validate_current(
        &self,
        semantic_registry: &ShadowRunnerSemanticRegistry,
    ) -> Result<(), ShadowRunnerSemanticProvenanceError> {
        if semantic_registry.authority_stamp() != &self.authority {
            return Err(ShadowRunnerSemanticProvenanceError::SemanticAuthorityChanged);
        }
        let current = semantic_registry.resolve(&self.executable)?;
        if current != *self {
            return Err(ShadowRunnerSemanticProvenanceError::VerifierBoundQualificationStale);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifierBoundExecutableShadowRunnerPairAuthority {
    semantic_authority: ShadowRunnerSemanticAuthorityStamp,
    executable_pair: ExecutableShadowRunnerPairAuthority,
    reference: VerifierBoundExecutableShadowRunnerQualification,
    coarse: VerifierBoundExecutableShadowRunnerQualification,
}

impl VerifierBoundExecutableShadowRunnerPairAuthority {
    pub const fn semantic_authority(&self) -> &ShadowRunnerSemanticAuthorityStamp {
        &self.semantic_authority
    }

    pub const fn executable_pair(&self) -> &ExecutableShadowRunnerPairAuthority {
        &self.executable_pair
    }

    pub const fn reference(&self) -> &VerifierBoundExecutableShadowRunnerQualification {
        &self.reference
    }

    pub const fn coarse(&self) -> &VerifierBoundExecutableShadowRunnerQualification {
        &self.coarse
    }

    pub fn validate_current(
        &self,
        semantic_registry: &ShadowRunnerSemanticRegistry,
        qualification_registry: &ShadowRunnerQualificationRegistry,
        reference_runner: &ShadowReferenceRunnerQualification,
        coarse_runner: &ShadowCoarseRunnerQualification,
    ) -> Result<(), ShadowRunnerSemanticProvenanceError> {
        self.executable_pair
            .validate_current(qualification_registry, reference_runner, coarse_runner)
            .map_err(ShadowRunnerSemanticProvenanceError::Qualification)?;
        if semantic_registry.authority_stamp() != &self.semantic_authority {
            return Err(ShadowRunnerSemanticProvenanceError::SemanticAuthorityChanged);
        }
        let current = semantic_registry.resolve_pair(&self.executable_pair)?;
        if current != *self {
            return Err(ShadowRunnerSemanticProvenanceError::VerifierBoundQualificationStale);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShadowRunnerSemanticProvenanceError {
    ConflictingSemanticClaim {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    MissingSemanticClaim {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    ReceiptNotCurrent {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    SemanticClaimNotCurrent {
        receipt: ShadowRunnerQualificationReceiptKey,
        status: VerifiedRunnerSemanticClaimStatus,
    },
    ReceiptRevisionMismatch {
        receipt: ShadowRunnerQualificationReceiptKey,
        expected: ShadowRunnerQualificationReceiptRevision,
        actual: ShadowRunnerQualificationReceiptRevision,
    },
    ProvenanceMismatch {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    SemanticContractMismatch {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    FixtureMismatch {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    TranscriptMismatch {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    StateManifestGrammarMismatch {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    Qualification(ShadowRunnerQualificationError),
    SemanticAuthorityChanged,
    VerifierBoundQualificationStale,
}

impl fmt::Display for ShadowRunnerSemanticProvenanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConflictingSemanticClaim { receipt } => write!(
                f,
                "conflicting verifier-bound semantic claim for qualification receipt {}@{}",
                receipt.id(),
                receipt.version()
            ),
            Self::MissingSemanticClaim { receipt } => write!(
                f,
                "qualification receipt {}@{} has no verifier-bound semantic claim",
                receipt.id(),
                receipt.version()
            ),
            Self::ReceiptNotCurrent { receipt } => write!(
                f,
                "qualification receipt {}@{} is not current",
                receipt.id(),
                receipt.version()
            ),
            Self::SemanticClaimNotCurrent { receipt, status } => write!(
                f,
                "semantic claim for qualification receipt {}@{} is {status:?}, not Current",
                receipt.id(),
                receipt.version()
            ),
            Self::ReceiptRevisionMismatch {
                receipt,
                expected,
                actual,
            } => write!(
                f,
                "semantic claim for qualification receipt {}@{} binds revision {}, current receipt is revision {}",
                receipt.id(),
                receipt.version(),
                actual.0,
                expected.0
            ),
            Self::ProvenanceMismatch { receipt } => write!(
                f,
                "semantic claim for qualification receipt {}@{} binds different verified provenance",
                receipt.id(),
                receipt.version()
            ),
            Self::SemanticContractMismatch { receipt } => write!(
                f,
                "semantic claim for qualification receipt {}@{} binds a different runner contract",
                receipt.id(),
                receipt.version()
            ),
            Self::FixtureMismatch { receipt } => write!(
                f,
                "semantic claim for qualification receipt {}@{} binds a different hostile fixture corpus",
                receipt.id(),
                receipt.version()
            ),
            Self::TranscriptMismatch { receipt } => write!(
                f,
                "semantic claim for qualification receipt {}@{} binds a different execution transcript",
                receipt.id(),
                receipt.version()
            ),
            Self::StateManifestGrammarMismatch { receipt } => write!(
                f,
                "semantic claim for qualification receipt {}@{} binds a different source-state manifest grammar",
                receipt.id(),
                receipt.version()
            ),
            Self::Qualification(error) => write!(f, "executable runner qualification: {error}"),
            Self::SemanticAuthorityChanged => {
                write!(f, "verifier-bound runner semantic authority changed")
            }
            Self::VerifierBoundQualificationStale => {
                write!(f, "verifier-bound executable runner qualification is stale")
            }
        }
    }
}

impl Error for ShadowRunnerSemanticProvenanceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Qualification(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::information::RepresentationKey;
    use super::super::shadow_execution_lineage::{
        ShadowReferenceRunnerKey, ShadowReferenceRunnerProfileVersion,
        ShadowReferenceRunnerStatus,
    };
    use super::super::shadow_paired_execution::{
        ShadowCoarseRunnerKey, ShadowCoarseRunnerProfileVersion, ShadowCoarseRunnerStatus,
    };
    use super::super::shadow_runner_qualification::{
        QualificationEvidenceDigest, QualificationPredicateDigest,
        QualificationProducerIdentityDigest, QualificationProviderKey, QualificationSubjectDigest,
        QualificationVerifierPolicyKey, ReviewedShadowRunnerQualificationReceipt,
        ShadowRunnerBinding, ShadowRunnerQualificationPolicy,
        ShadowRunnerQualificationPolicyKey, ShadowRunnerQualificationPolicyStatus,
        ShadowRunnerQualificationReceiptRevision, ShadowRunnerQualificationRegistryBuilder,
        ShadowRunnerQualificationRegistryKey, ReviewedShadowRunnerQualificationStatus,
    };
    use super::super::shadow_validation::{
        ShadowExecutionCapsuleFingerprint, ShadowImplementationFingerprint,
    };

    fn implementation(byte: u8) -> ShadowImplementationFingerprint {
        ShadowImplementationFingerprint::new(vec![byte]).unwrap()
    }

    fn capsule(byte: u8) -> ShadowExecutionCapsuleFingerprint {
        ShadowExecutionCapsuleFingerprint::new(vec![byte]).unwrap()
    }

    fn provenance(subject: u8) -> VerifiedQualificationProvenance {
        VerifiedQualificationProvenance::new(
            QualificationProviderKey::new(1, 1),
            QualificationVerifierPolicyKey::new(2, 1),
            QualificationProducerIdentityDigest::new(vec![3]).unwrap(),
            QualificationSubjectDigest::new(vec![subject]).unwrap(),
            QualificationPredicateDigest::new(vec![4]).unwrap(),
            QualificationEvidenceDigest::new(vec![5]).unwrap(),
            EvidenceLineageToken(6),
        )
    }

    fn policy() -> ShadowRunnerQualificationPolicy {
        ShadowRunnerQualificationPolicy::new(
            ShadowRunnerQualificationPolicyKey::new(10, 1),
            QualificationProviderKey::new(1, 1),
            QualificationVerifierPolicyKey::new(2, 1),
            QualificationProducerIdentityDigest::new(vec![3]).unwrap(),
            ShadowRunnerSemanticContractVersion(1),
            QualificationPredicateDigest::new(vec![4]).unwrap(),
            QualificationFixtureDigest::new(vec![7]).unwrap(),
            ShadowStateManifestGrammarFingerprint::new(vec![8]).unwrap(),
            ShadowRunnerQualificationPolicyStatus::Current,
        )
    }

    fn reference_runner() -> ShadowReferenceRunnerQualification {
        ShadowReferenceRunnerQualification::new(
            ShadowReferenceRunnerKey::new(20, 1),
            ShadowReferenceRunnerProfileVersion(1),
            RepresentationKey::new(21, 1),
            implementation(22),
            Some(capsule(23)),
            EvidenceLineageToken(24),
            ShadowReferenceRunnerStatus::Qualified,
        )
    }

    fn coarse_runner() -> ShadowCoarseRunnerQualification {
        ShadowCoarseRunnerQualification::new(
            ShadowCoarseRunnerKey::new(30, 1),
            ShadowCoarseRunnerProfileVersion(1),
            RepresentationKey::new(31, 1),
            implementation(32),
            Some(capsule(33)),
            EvidenceLineageToken(34),
            ShadowCoarseRunnerStatus::Qualified,
        )
    }

    fn receipt(
        key: u128,
        binding: ShadowRunnerBinding,
        implementation_byte: u8,
        capsule_byte: u8,
        bootstrap_evidence: u128,
        transcript_byte: u8,
        subject_byte: u8,
    ) -> ReviewedShadowRunnerQualificationReceipt {
        ReviewedShadowRunnerQualificationReceipt::new(
            ShadowRunnerQualificationReceiptKey::new(key, 1),
            ShadowRunnerQualificationReceiptRevision(1),
            ShadowRunnerQualificationPolicyKey::new(10, 1),
            binding,
            implementation(implementation_byte),
            Some(capsule(capsule_byte)),
            EvidenceLineageToken(bootstrap_evidence),
            ShadowRunnerSemanticContractVersion(1),
            QualificationFixtureDigest::new(vec![7]).unwrap(),
            QualificationTranscriptDigest::new(vec![transcript_byte]).unwrap(),
            ShadowStateManifestGrammarFingerprint::new(vec![8]).unwrap(),
            provenance(subject_byte),
            ReviewedShadowRunnerQualificationStatus::Current,
        )
    }

    fn qualification_registry() -> ShadowRunnerQualificationRegistry {
        let mut builder = ShadowRunnerQualificationRegistryBuilder::new(
            ShadowRunnerQualificationRegistryKey::new(40, 1),
        );
        builder.register_policy(policy()).unwrap();
        builder
            .register_receipt(receipt(
                41,
                ShadowRunnerBinding::Reference {
                    runner: ShadowReferenceRunnerKey::new(20, 1),
                    profile: ShadowReferenceRunnerProfileVersion(1),
                    representation: RepresentationKey::new(21, 1),
                },
                22,
                23,
                24,
                42,
                43,
            ))
            .unwrap();
        builder
            .register_receipt(receipt(
                51,
                ShadowRunnerBinding::Coarse {
                    runner: ShadowCoarseRunnerKey::new(30, 1),
                    profile: ShadowCoarseRunnerProfileVersion(1),
                    representation: RepresentationKey::new(31, 1),
                },
                32,
                33,
                34,
                52,
                53,
            ))
            .unwrap();
        builder.seal().unwrap()
    }

    fn claim(
        receipt: u128,
        transcript: u8,
        subject: u8,
        status: VerifiedRunnerSemanticClaimStatus,
    ) -> VerifiedRunnerSemanticClaim {
        VerifiedRunnerSemanticClaim::new(
            ShadowRunnerQualificationReceiptKey::new(receipt, 1),
            ShadowRunnerQualificationReceiptRevision(1),
            provenance(subject),
            ShadowRunnerSemanticContractVersion(1),
            QualificationFixtureDigest::new(vec![7]).unwrap(),
            QualificationTranscriptDigest::new(vec![transcript]).unwrap(),
            ShadowStateManifestGrammarFingerprint::new(vec![8]).unwrap(),
            EvidenceLineageToken(60 + receipt),
            status,
        )
    }

    fn semantic_registry(
        key: u128,
        reference_claim: Option<VerifiedRunnerSemanticClaim>,
        coarse_claim: Option<VerifiedRunnerSemanticClaim>,
    ) -> ShadowRunnerSemanticRegistry {
        let mut builder = ShadowRunnerSemanticRegistryBuilder::new(
            ShadowRunnerSemanticRegistryKey::new(key, 1),
        );
        if let Some(claim) = reference_claim {
            builder.register_claim(claim).unwrap();
        }
        if let Some(claim) = coarse_claim {
            builder.register_claim(claim).unwrap();
        }
        builder.seal()
    }

    #[test]
    fn reviewed_receipt_without_semantic_claim_cannot_resolve() {
        let qualifications = qualification_registry();
        let executable = qualifications.resolve_reference(&reference_runner()).unwrap();
        let semantics = semantic_registry(70, None, None);
        assert!(matches!(
            semantics.resolve(&executable),
            Err(ShadowRunnerSemanticProvenanceError::MissingSemanticClaim { .. })
        ));
    }

    #[test]
    fn exact_semantic_pair_resolves_and_revalidates() {
        let qualifications = qualification_registry();
        let reference = reference_runner();
        let coarse = coarse_runner();
        let pair = qualifications.resolve_pair(&reference, &coarse).unwrap();
        let semantics = semantic_registry(
            70,
            Some(claim(41, 42, 43, VerifiedRunnerSemanticClaimStatus::Current)),
            Some(claim(51, 52, 53, VerifiedRunnerSemanticClaimStatus::Current)),
        );
        let bound = semantics.resolve_pair(&pair).unwrap();
        assert_eq!(
            bound.validate_current(&semantics, &qualifications, &reference, &coarse),
            Ok(())
        );
    }

    #[test]
    fn transcript_reinterpretation_rejects() {
        let qualifications = qualification_registry();
        let executable = qualifications.resolve_reference(&reference_runner()).unwrap();
        let semantics = semantic_registry(
            70,
            Some(claim(41, 99, 43, VerifiedRunnerSemanticClaimStatus::Current)),
            None,
        );
        assert!(matches!(
            semantics.resolve(&executable),
            Err(ShadowRunnerSemanticProvenanceError::TranscriptMismatch { .. })
        ));
    }

    #[test]
    fn different_verified_subject_rejects() {
        let qualifications = qualification_registry();
        let executable = qualifications.resolve_reference(&reference_runner()).unwrap();
        let semantics = semantic_registry(
            70,
            Some(claim(41, 42, 99, VerifiedRunnerSemanticClaimStatus::Current)),
            None,
        );
        assert!(matches!(
            semantics.resolve(&executable),
            Err(ShadowRunnerSemanticProvenanceError::ProvenanceMismatch { .. })
        ));
    }

    #[test]
    fn revoked_semantic_claim_rejects() {
        let qualifications = qualification_registry();
        let executable = qualifications.resolve_reference(&reference_runner()).unwrap();
        let semantics = semantic_registry(
            70,
            Some(claim(41, 42, 43, VerifiedRunnerSemanticClaimStatus::Revoked)),
            None,
        );
        assert!(matches!(
            semantics.resolve(&executable),
            Err(ShadowRunnerSemanticProvenanceError::SemanticClaimNotCurrent { .. })
        ));
    }

    #[test]
    fn semantic_authority_change_invalidates_bound_pair() {
        let qualifications = qualification_registry();
        let reference = reference_runner();
        let coarse = coarse_runner();
        let pair = qualifications.resolve_pair(&reference, &coarse).unwrap();
        let semantics = semantic_registry(
            70,
            Some(claim(41, 42, 43, VerifiedRunnerSemanticClaimStatus::Current)),
            Some(claim(51, 52, 53, VerifiedRunnerSemanticClaimStatus::Current)),
        );
        let bound = semantics.resolve_pair(&pair).unwrap();
        let replaced = semantic_registry(
            71,
            Some(claim(41, 42, 43, VerifiedRunnerSemanticClaimStatus::Current)),
            Some(claim(51, 52, 53, VerifiedRunnerSemanticClaimStatus::Current)),
        );
        assert_eq!(
            bound.validate_current(&replaced, &qualifications, &reference, &coarse),
            Err(ShadowRunnerSemanticProvenanceError::SemanticAuthorityChanged)
        );
    }
}
