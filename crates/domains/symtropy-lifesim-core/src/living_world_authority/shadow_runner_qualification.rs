// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Provider-neutral executable qualification authority for Q2 shadow runners.
//!
//! #563/#567 deliberately carry bootstrap `Qualified` status so structural runner
//! and execution-lineage contracts can be assembled independently. That status is
//! necessary configuration, not proof that an implementation actually obeys the
//! execution semantics required for closure-anchor renewal.
//!
//! This module adds the independent #568 authority layer. External verifiers
//! (GitHub/Sigstore today, Xenia/Nix/local verifiers later) may produce verified
//! provenance, but provenance is not scientific authority by itself. A canonical
//! reviewed policy must bind the exact verifier identity, semantic contract,
//! fixture, manifest grammar and predicate before a receipt can resolve into an
//! executable runner qualification.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::information::{EvidenceLineageToken, RepresentationKey};

use super::shadow_execution_lineage::{
    ShadowReferenceRunnerKey, ShadowReferenceRunnerProfileVersion,
    ShadowReferenceRunnerQualification, ShadowReferenceRunnerStatus,
};
use super::shadow_paired_execution::{
    ShadowCoarseRunnerKey, ShadowCoarseRunnerProfileVersion, ShadowCoarseRunnerQualification,
    ShadowCoarseRunnerStatus,
};
use super::shadow_validation::{
    ShadowExecutionCapsuleFingerprint, ShadowImplementationFingerprint,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowRunnerQualificationRegistryKey {
    id: u128,
    version: u32,
}

impl ShadowRunnerQualificationRegistryKey {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowRunnerQualificationPolicyKey {
    id: u128,
    version: u32,
}

impl ShadowRunnerQualificationPolicyKey {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowRunnerQualificationReceiptKey {
    id: u128,
    version: u32,
}

impl ShadowRunnerQualificationReceiptKey {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowRunnerQualificationReceiptRevision(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QualificationProviderKey {
    id: u128,
    version: u32,
}

impl QualificationProviderKey {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QualificationVerifierPolicyKey {
    id: u128,
    version: u32,
}

impl QualificationVerifierPolicyKey {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowRunnerSemanticContractVersion(pub u32);

macro_rules! opaque_bytes {
    ($name:ident, $error:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(Vec<u8>);

        impl $name {
            pub fn new(
                bytes: impl Into<Vec<u8>>,
            ) -> Result<Self, ShadowRunnerQualificationError> {
                let bytes = bytes.into();
                if bytes.is_empty() {
                    return Err(ShadowRunnerQualificationError::$error);
                }
                Ok(Self(bytes))
            }

            pub fn as_bytes(&self) -> &[u8] {
                &self.0
            }
        }
    };
}

opaque_bytes!(QualificationProducerIdentityDigest, EmptyProducerIdentityDigest);
opaque_bytes!(QualificationSubjectDigest, EmptySubjectDigest);
opaque_bytes!(QualificationPredicateDigest, EmptyPredicateDigest);
opaque_bytes!(QualificationEvidenceDigest, EmptyEvidenceDigest);
opaque_bytes!(QualificationFixtureDigest, EmptyFixtureDigest);
opaque_bytes!(QualificationTranscriptDigest, EmptyTranscriptDigest);
opaque_bytes!(ShadowStateManifestGrammarFingerprint, EmptyStateManifestGrammar);

/// Provider-neutral statement that one external verifier accepted one exact
/// qualification evidence object under one exact verifier policy.
///
/// Construction is evidence ingestion, not authority. The reviewed registry below
/// independently checks every field against canonical policy before the provenance
/// can contribute to executable runner authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedQualificationProvenance {
    provider: QualificationProviderKey,
    verifier_policy: QualificationVerifierPolicyKey,
    producer_identity: QualificationProducerIdentityDigest,
    subject: QualificationSubjectDigest,
    predicate: QualificationPredicateDigest,
    evidence_digest: QualificationEvidenceDigest,
    verification_evidence: EvidenceLineageToken,
}

impl VerifiedQualificationProvenance {
    pub fn new(
        provider: QualificationProviderKey,
        verifier_policy: QualificationVerifierPolicyKey,
        producer_identity: QualificationProducerIdentityDigest,
        subject: QualificationSubjectDigest,
        predicate: QualificationPredicateDigest,
        evidence_digest: QualificationEvidenceDigest,
        verification_evidence: EvidenceLineageToken,
    ) -> Self {
        Self {
            provider,
            verifier_policy,
            producer_identity,
            subject,
            predicate,
            evidence_digest,
            verification_evidence,
        }
    }

    pub const fn provider(&self) -> QualificationProviderKey {
        self.provider
    }

    pub const fn verifier_policy(&self) -> QualificationVerifierPolicyKey {
        self.verifier_policy
    }

    pub const fn producer_identity(&self) -> &QualificationProducerIdentityDigest {
        &self.producer_identity
    }

    pub const fn subject(&self) -> &QualificationSubjectDigest {
        &self.subject
    }

    pub const fn predicate(&self) -> &QualificationPredicateDigest {
        &self.predicate
    }

    pub const fn evidence_digest(&self) -> &QualificationEvidenceDigest {
        &self.evidence_digest
    }

    pub const fn verification_evidence(&self) -> EvidenceLineageToken {
        self.verification_evidence
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowRunnerQualificationPolicyStatus {
    Current,
    Revoked,
    Superseded,
}

/// Reviewed policy describing which externally verified proposition is sufficient
/// to establish the #568 runner-semantic theorem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowRunnerQualificationPolicy {
    key: ShadowRunnerQualificationPolicyKey,
    provider: QualificationProviderKey,
    verifier_policy: QualificationVerifierPolicyKey,
    producer_identity: QualificationProducerIdentityDigest,
    semantic_contract: ShadowRunnerSemanticContractVersion,
    predicate: QualificationPredicateDigest,
    fixture: QualificationFixtureDigest,
    state_manifest_grammar: ShadowStateManifestGrammarFingerprint,
    status: ShadowRunnerQualificationPolicyStatus,
}

impl ShadowRunnerQualificationPolicy {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        key: ShadowRunnerQualificationPolicyKey,
        provider: QualificationProviderKey,
        verifier_policy: QualificationVerifierPolicyKey,
        producer_identity: QualificationProducerIdentityDigest,
        semantic_contract: ShadowRunnerSemanticContractVersion,
        predicate: QualificationPredicateDigest,
        fixture: QualificationFixtureDigest,
        state_manifest_grammar: ShadowStateManifestGrammarFingerprint,
        status: ShadowRunnerQualificationPolicyStatus,
    ) -> Self {
        Self {
            key,
            provider,
            verifier_policy,
            producer_identity,
            semantic_contract,
            predicate,
            fixture,
            state_manifest_grammar,
            status,
        }
    }

    pub const fn key(&self) -> ShadowRunnerQualificationPolicyKey {
        self.key
    }

    pub const fn provider(&self) -> QualificationProviderKey {
        self.provider
    }

    pub const fn verifier_policy(&self) -> QualificationVerifierPolicyKey {
        self.verifier_policy
    }

    pub const fn producer_identity(&self) -> &QualificationProducerIdentityDigest {
        &self.producer_identity
    }

    pub const fn semantic_contract(&self) -> ShadowRunnerSemanticContractVersion {
        self.semantic_contract
    }

    pub const fn predicate(&self) -> &QualificationPredicateDigest {
        &self.predicate
    }

    pub const fn fixture(&self) -> &QualificationFixtureDigest {
        &self.fixture
    }

    pub const fn state_manifest_grammar(&self) -> &ShadowStateManifestGrammarFingerprint {
        &self.state_manifest_grammar
    }

    pub const fn status(&self) -> ShadowRunnerQualificationPolicyStatus {
        self.status
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShadowRunnerBinding {
    Reference {
        runner: ShadowReferenceRunnerKey,
        profile: ShadowReferenceRunnerProfileVersion,
        representation: RepresentationKey,
    },
    Coarse {
        runner: ShadowCoarseRunnerKey,
        profile: ShadowCoarseRunnerProfileVersion,
        representation: RepresentationKey,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewedShadowRunnerQualificationStatus {
    Current,
    Revoked,
    Superseded,
}

/// Human/review-policy accepted executable qualification receipt.
///
/// The receipt binds external provenance to the exact bootstrap runner identity and
/// implementation universe. It cannot be satisfied by bootstrap `Qualified` alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedShadowRunnerQualificationReceipt {
    key: ShadowRunnerQualificationReceiptKey,
    revision: ShadowRunnerQualificationReceiptRevision,
    policy: ShadowRunnerQualificationPolicyKey,
    binding: ShadowRunnerBinding,
    implementation: ShadowImplementationFingerprint,
    execution_capsule: Option<ShadowExecutionCapsuleFingerprint>,
    bootstrap_evidence: EvidenceLineageToken,
    semantic_contract: ShadowRunnerSemanticContractVersion,
    fixture: QualificationFixtureDigest,
    transcript: QualificationTranscriptDigest,
    state_manifest_grammar: ShadowStateManifestGrammarFingerprint,
    provenance: VerifiedQualificationProvenance,
    status: ReviewedShadowRunnerQualificationStatus,
}

impl ReviewedShadowRunnerQualificationReceipt {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        key: ShadowRunnerQualificationReceiptKey,
        revision: ShadowRunnerQualificationReceiptRevision,
        policy: ShadowRunnerQualificationPolicyKey,
        binding: ShadowRunnerBinding,
        implementation: ShadowImplementationFingerprint,
        execution_capsule: Option<ShadowExecutionCapsuleFingerprint>,
        bootstrap_evidence: EvidenceLineageToken,
        semantic_contract: ShadowRunnerSemanticContractVersion,
        fixture: QualificationFixtureDigest,
        transcript: QualificationTranscriptDigest,
        state_manifest_grammar: ShadowStateManifestGrammarFingerprint,
        provenance: VerifiedQualificationProvenance,
        status: ReviewedShadowRunnerQualificationStatus,
    ) -> Self {
        Self {
            key,
            revision,
            policy,
            binding,
            implementation,
            execution_capsule,
            bootstrap_evidence,
            semantic_contract,
            fixture,
            transcript,
            state_manifest_grammar,
            provenance,
            status,
        }
    }

    pub const fn key(&self) -> ShadowRunnerQualificationReceiptKey {
        self.key
    }

    pub const fn revision(&self) -> ShadowRunnerQualificationReceiptRevision {
        self.revision
    }

    pub const fn policy(&self) -> ShadowRunnerQualificationPolicyKey {
        self.policy
    }

    pub const fn binding(&self) -> &ShadowRunnerBinding {
        &self.binding
    }

    pub const fn implementation(&self) -> &ShadowImplementationFingerprint {
        &self.implementation
    }

    pub const fn execution_capsule(&self) -> Option<&ShadowExecutionCapsuleFingerprint> {
        self.execution_capsule.as_ref()
    }

    pub const fn bootstrap_evidence(&self) -> EvidenceLineageToken {
        self.bootstrap_evidence
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

    pub const fn provenance(&self) -> &VerifiedQualificationProvenance {
        &self.provenance
    }

    pub const fn status(&self) -> ReviewedShadowRunnerQualificationStatus {
        self.status
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowRunnerQualificationAuthorityStamp {
    key: ShadowRunnerQualificationRegistryKey,
    policies: BTreeMap<ShadowRunnerQualificationPolicyKey, ShadowRunnerQualificationPolicy>,
    receipts:
        BTreeMap<ShadowRunnerQualificationReceiptKey, ReviewedShadowRunnerQualificationReceipt>,
}

impl ShadowRunnerQualificationAuthorityStamp {
    pub const fn key(&self) -> ShadowRunnerQualificationRegistryKey {
        self.key
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowRunnerQualificationRegistryBuilder {
    key: ShadowRunnerQualificationRegistryKey,
    policies: BTreeMap<ShadowRunnerQualificationPolicyKey, ShadowRunnerQualificationPolicy>,
    receipts:
        BTreeMap<ShadowRunnerQualificationReceiptKey, ReviewedShadowRunnerQualificationReceipt>,
}

impl ShadowRunnerQualificationRegistryBuilder {
    pub const fn new(key: ShadowRunnerQualificationRegistryKey) -> Self {
        Self {
            key,
            policies: BTreeMap::new(),
            receipts: BTreeMap::new(),
        }
    }

    pub fn register_policy(
        &mut self,
        policy: ShadowRunnerQualificationPolicy,
    ) -> Result<(), ShadowRunnerQualificationError> {
        use std::collections::btree_map::Entry;
        match self.policies.entry(policy.key()) {
            Entry::Vacant(entry) => {
                entry.insert(policy);
                Ok(())
            }
            Entry::Occupied(entry) if entry.get() == &policy => Ok(()),
            Entry::Occupied(entry) => Err(ShadowRunnerQualificationError::ConflictingPolicy {
                policy: *entry.key(),
            }),
        }
    }

    pub fn register_receipt(
        &mut self,
        receipt: ReviewedShadowRunnerQualificationReceipt,
    ) -> Result<(), ShadowRunnerQualificationError> {
        use std::collections::btree_map::Entry;
        match self.receipts.entry(receipt.key()) {
            Entry::Vacant(entry) => {
                entry.insert(receipt);
                Ok(())
            }
            Entry::Occupied(entry) if entry.get() == &receipt => Ok(()),
            Entry::Occupied(entry) => Err(ShadowRunnerQualificationError::ConflictingReceipt {
                receipt: *entry.key(),
            }),
        }
    }

    pub fn seal(self) -> Result<ShadowRunnerQualificationRegistry, ShadowRunnerQualificationError> {
        for receipt in self.receipts.values() {
            let policy = self.policies.get(&receipt.policy()).ok_or(
                ShadowRunnerQualificationError::ReceiptReferencesUnknownPolicy {
                    receipt: receipt.key(),
                    policy: receipt.policy(),
                },
            )?;
            validate_receipt_against_policy(receipt, policy)?;
        }

        let authority = ShadowRunnerQualificationAuthorityStamp {
            key: self.key,
            policies: self.policies.clone(),
            receipts: self.receipts.clone(),
        };
        Ok(ShadowRunnerQualificationRegistry {
            authority,
            policies: self.policies,
            receipts: self.receipts,
        })
    }
}

fn validate_receipt_against_policy(
    receipt: &ReviewedShadowRunnerQualificationReceipt,
    policy: &ShadowRunnerQualificationPolicy,
) -> Result<(), ShadowRunnerQualificationError> {
    let provenance = receipt.provenance();
    if provenance.provider() != policy.provider() {
        return Err(ShadowRunnerQualificationError::ProviderMismatch {
            receipt: receipt.key(),
        });
    }
    if provenance.verifier_policy() != policy.verifier_policy() {
        return Err(ShadowRunnerQualificationError::VerifierPolicyMismatch {
            receipt: receipt.key(),
        });
    }
    if provenance.producer_identity() != policy.producer_identity() {
        return Err(ShadowRunnerQualificationError::ProducerIdentityMismatch {
            receipt: receipt.key(),
        });
    }
    if provenance.predicate() != policy.predicate() {
        return Err(ShadowRunnerQualificationError::PredicateMismatch {
            receipt: receipt.key(),
        });
    }
    if receipt.semantic_contract() != policy.semantic_contract() {
        return Err(ShadowRunnerQualificationError::SemanticContractMismatch {
            receipt: receipt.key(),
        });
    }
    if receipt.fixture() != policy.fixture() {
        return Err(ShadowRunnerQualificationError::FixtureMismatch {
            receipt: receipt.key(),
        });
    }
    if receipt.state_manifest_grammar() != policy.state_manifest_grammar() {
        return Err(ShadowRunnerQualificationError::StateManifestGrammarMismatch {
            receipt: receipt.key(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowRunnerQualificationRegistry {
    authority: ShadowRunnerQualificationAuthorityStamp,
    policies: BTreeMap<ShadowRunnerQualificationPolicyKey, ShadowRunnerQualificationPolicy>,
    receipts:
        BTreeMap<ShadowRunnerQualificationReceiptKey, ReviewedShadowRunnerQualificationReceipt>,
}

impl ShadowRunnerQualificationRegistry {
    pub const fn authority_stamp(&self) -> &ShadowRunnerQualificationAuthorityStamp {
        &self.authority
    }

    pub fn resolve_reference(
        &self,
        runner: &ShadowReferenceRunnerQualification,
    ) -> Result<ExecutableShadowRunnerQualification, ShadowRunnerQualificationError> {
        if runner.status() != ShadowReferenceRunnerStatus::Qualified {
            return Err(ShadowRunnerQualificationError::ReferenceBootstrapNotQualified {
                runner: runner.runner(),
                status: runner.status(),
            });
        }

        self.resolve_unique(|receipt| {
            matches!(
                receipt.binding(),
                ShadowRunnerBinding::Reference {
                    runner: receipt_runner,
                    profile,
                    representation,
                } if *receipt_runner == runner.runner()
                    && *profile == runner.profile()
                    && *representation == runner.reference_representation()
            ) && receipt.implementation() == runner.implementation()
                && receipt.execution_capsule() == runner.execution_capsule()
                && receipt.bootstrap_evidence() == runner.qualification_evidence()
        })
    }

    pub fn resolve_coarse(
        &self,
        runner: &ShadowCoarseRunnerQualification,
    ) -> Result<ExecutableShadowRunnerQualification, ShadowRunnerQualificationError> {
        if runner.status() != ShadowCoarseRunnerStatus::Qualified {
            return Err(ShadowRunnerQualificationError::CoarseBootstrapNotQualified {
                runner: runner.runner(),
                status: runner.status(),
            });
        }

        self.resolve_unique(|receipt| {
            matches!(
                receipt.binding(),
                ShadowRunnerBinding::Coarse {
                    runner: receipt_runner,
                    profile,
                    representation,
                } if *receipt_runner == runner.runner()
                    && *profile == runner.profile()
                    && *representation == runner.representation()
            ) && receipt.implementation() == runner.implementation()
                && receipt.execution_capsule() == runner.execution_capsule()
                && receipt.bootstrap_evidence() == runner.qualification_evidence()
        })
    }

    fn resolve_unique(
        &self,
        mut matches_runner: impl FnMut(&ReviewedShadowRunnerQualificationReceipt) -> bool,
    ) -> Result<ExecutableShadowRunnerQualification, ShadowRunnerQualificationError> {
        let mut matches = self.receipts.values().filter(|receipt| {
            if receipt.status() != ReviewedShadowRunnerQualificationStatus::Current {
                return false;
            }
            let Some(policy) = self.policies.get(&receipt.policy()) else {
                return false;
            };
            policy.status() == ShadowRunnerQualificationPolicyStatus::Current
                && matches_runner(receipt)
        });

        let receipt = matches
            .next()
            .ok_or(ShadowRunnerQualificationError::NoExecutableQualification)?;
        if matches.next().is_some() {
            return Err(ShadowRunnerQualificationError::AmbiguousExecutableQualification);
        }

        Ok(ExecutableShadowRunnerQualification {
            authority: self.authority.clone(),
            receipt: receipt.clone(),
        })
    }

    pub fn resolve_pair(
        &self,
        reference: &ShadowReferenceRunnerQualification,
        coarse: &ShadowCoarseRunnerQualification,
    ) -> Result<ExecutableShadowRunnerPairAuthority, ShadowRunnerQualificationError> {
        Ok(ExecutableShadowRunnerPairAuthority {
            authority: self.authority.clone(),
            reference: self.resolve_reference(reference)?,
            coarse: self.resolve_coarse(coarse)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutableShadowRunnerQualification {
    authority: ShadowRunnerQualificationAuthorityStamp,
    receipt: ReviewedShadowRunnerQualificationReceipt,
}

impl ExecutableShadowRunnerQualification {
    pub const fn authority_stamp(&self) -> &ShadowRunnerQualificationAuthorityStamp {
        &self.authority
    }

    pub const fn receipt(&self) -> &ReviewedShadowRunnerQualificationReceipt {
        &self.receipt
    }

    pub fn validate_reference_current(
        &self,
        registry: &ShadowRunnerQualificationRegistry,
        runner: &ShadowReferenceRunnerQualification,
    ) -> Result<(), ShadowRunnerQualificationError> {
        if registry.authority_stamp() != &self.authority {
            return Err(ShadowRunnerQualificationError::QualificationAuthorityChanged);
        }
        let current = registry.resolve_reference(runner)?;
        if current != *self {
            return Err(ShadowRunnerQualificationError::ExecutableQualificationStale);
        }
        Ok(())
    }

    pub fn validate_coarse_current(
        &self,
        registry: &ShadowRunnerQualificationRegistry,
        runner: &ShadowCoarseRunnerQualification,
    ) -> Result<(), ShadowRunnerQualificationError> {
        if registry.authority_stamp() != &self.authority {
            return Err(ShadowRunnerQualificationError::QualificationAuthorityChanged);
        }
        let current = registry.resolve_coarse(runner)?;
        if current != *self {
            return Err(ShadowRunnerQualificationError::ExecutableQualificationStale);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutableShadowRunnerPairAuthority {
    authority: ShadowRunnerQualificationAuthorityStamp,
    reference: ExecutableShadowRunnerQualification,
    coarse: ExecutableShadowRunnerQualification,
}

impl ExecutableShadowRunnerPairAuthority {
    pub const fn authority_stamp(&self) -> &ShadowRunnerQualificationAuthorityStamp {
        &self.authority
    }

    pub const fn reference(&self) -> &ExecutableShadowRunnerQualification {
        &self.reference
    }

    pub const fn coarse(&self) -> &ExecutableShadowRunnerQualification {
        &self.coarse
    }

    pub fn validate_current(
        &self,
        registry: &ShadowRunnerQualificationRegistry,
        reference: &ShadowReferenceRunnerQualification,
        coarse: &ShadowCoarseRunnerQualification,
    ) -> Result<(), ShadowRunnerQualificationError> {
        if registry.authority_stamp() != &self.authority {
            return Err(ShadowRunnerQualificationError::QualificationAuthorityChanged);
        }
        let current = registry.resolve_pair(reference, coarse)?;
        if current != *self {
            return Err(ShadowRunnerQualificationError::ExecutableQualificationStale);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShadowRunnerQualificationError {
    EmptyProducerIdentityDigest,
    EmptySubjectDigest,
    EmptyPredicateDigest,
    EmptyEvidenceDigest,
    EmptyFixtureDigest,
    EmptyTranscriptDigest,
    EmptyStateManifestGrammar,
    ConflictingPolicy {
        policy: ShadowRunnerQualificationPolicyKey,
    },
    ConflictingReceipt {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    ReceiptReferencesUnknownPolicy {
        receipt: ShadowRunnerQualificationReceiptKey,
        policy: ShadowRunnerQualificationPolicyKey,
    },
    ProviderMismatch {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    VerifierPolicyMismatch {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    ProducerIdentityMismatch {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    PredicateMismatch {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    SemanticContractMismatch {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    FixtureMismatch {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    StateManifestGrammarMismatch {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    ReferenceBootstrapNotQualified {
        runner: ShadowReferenceRunnerKey,
        status: ShadowReferenceRunnerStatus,
    },
    CoarseBootstrapNotQualified {
        runner: ShadowCoarseRunnerKey,
        status: ShadowCoarseRunnerStatus,
    },
    NoExecutableQualification,
    AmbiguousExecutableQualification,
    QualificationAuthorityChanged,
    ExecutableQualificationStale,
}

impl fmt::Display for ShadowRunnerQualificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProducerIdentityDigest => write!(f, "qualification producer identity is empty"),
            Self::EmptySubjectDigest => write!(f, "qualification subject digest is empty"),
            Self::EmptyPredicateDigest => write!(f, "qualification predicate digest is empty"),
            Self::EmptyEvidenceDigest => write!(f, "qualification evidence digest is empty"),
            Self::EmptyFixtureDigest => write!(f, "qualification fixture digest is empty"),
            Self::EmptyTranscriptDigest => write!(f, "qualification transcript digest is empty"),
            Self::EmptyStateManifestGrammar => write!(f, "shadow state-manifest grammar fingerprint is empty"),
            Self::ConflictingPolicy { policy } => write!(
                f,
                "conflicting shadow runner qualification policy {}@{}",
                policy.id(),
                policy.version()
            ),
            Self::ConflictingReceipt { receipt } => write!(
                f,
                "conflicting shadow runner qualification receipt {}@{}",
                receipt.id(),
                receipt.version()
            ),
            Self::ReceiptReferencesUnknownPolicy { receipt, policy } => write!(
                f,
                "qualification receipt {}@{} references unknown policy {}@{}",
                receipt.id(),
                receipt.version(),
                policy.id(),
                policy.version()
            ),
            Self::ProviderMismatch { receipt } => write!(
                f,
                "qualification receipt {}@{} uses the wrong provenance provider",
                receipt.id(),
                receipt.version()
            ),
            Self::VerifierPolicyMismatch { receipt } => write!(
                f,
                "qualification receipt {}@{} uses the wrong verifier policy",
                receipt.id(),
                receipt.version()
            ),
            Self::ProducerIdentityMismatch { receipt } => write!(
                f,
                "qualification receipt {}@{} uses the wrong producer identity",
                receipt.id(),
                receipt.version()
            ),
            Self::PredicateMismatch { receipt } => write!(
                f,
                "qualification receipt {}@{} proves the wrong predicate",
                receipt.id(),
                receipt.version()
            ),
            Self::SemanticContractMismatch { receipt } => write!(
                f,
                "qualification receipt {}@{} targets the wrong semantic contract",
                receipt.id(),
                receipt.version()
            ),
            Self::FixtureMismatch { receipt } => write!(
                f,
                "qualification receipt {}@{} used the wrong hostile fixture corpus",
                receipt.id(),
                receipt.version()
            ),
            Self::StateManifestGrammarMismatch { receipt } => write!(
                f,
                "qualification receipt {}@{} used the wrong source-state manifest grammar",
                receipt.id(),
                receipt.version()
            ),
            Self::ReferenceBootstrapNotQualified { runner, status } => write!(
                f,
                "reference runner {}@{} bootstrap state is {status:?}, not Qualified",
                runner.id(),
                runner.version()
            ),
            Self::CoarseBootstrapNotQualified { runner, status } => write!(
                f,
                "coarse runner {}@{} bootstrap state is {status:?}, not Qualified",
                runner.id(),
                runner.version()
            ),
            Self::NoExecutableQualification => write!(
                f,
                "bootstrap runner has no current reviewed executable qualification receipt"
            ),
            Self::AmbiguousExecutableQualification => write!(
                f,
                "bootstrap runner resolves to multiple current executable qualification receipts"
            ),
            Self::QualificationAuthorityChanged => {
                write!(f, "shadow runner qualification authority changed")
            }
            Self::ExecutableQualificationStale => {
                write!(f, "executable shadow runner qualification is stale")
            }
        }
    }
}

impl Error for ShadowRunnerQualificationError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn implementation(byte: u8) -> ShadowImplementationFingerprint {
        ShadowImplementationFingerprint::new(vec![byte]).unwrap()
    }

    fn capsule(byte: u8) -> ShadowExecutionCapsuleFingerprint {
        ShadowExecutionCapsuleFingerprint::new(vec![byte]).unwrap()
    }

    fn policy(status: ShadowRunnerQualificationPolicyStatus) -> ShadowRunnerQualificationPolicy {
        ShadowRunnerQualificationPolicy::new(
            ShadowRunnerQualificationPolicyKey::new(1, 1),
            QualificationProviderKey::new(2, 1),
            QualificationVerifierPolicyKey::new(3, 1),
            QualificationProducerIdentityDigest::new(vec![4]).unwrap(),
            ShadowRunnerSemanticContractVersion(1),
            QualificationPredicateDigest::new(vec![5]).unwrap(),
            QualificationFixtureDigest::new(vec![6]).unwrap(),
            ShadowStateManifestGrammarFingerprint::new(vec![7]).unwrap(),
            status,
        )
    }

    fn provenance() -> VerifiedQualificationProvenance {
        VerifiedQualificationProvenance::new(
            QualificationProviderKey::new(2, 1),
            QualificationVerifierPolicyKey::new(3, 1),
            QualificationProducerIdentityDigest::new(vec![4]).unwrap(),
            QualificationSubjectDigest::new(vec![8]).unwrap(),
            QualificationPredicateDigest::new(vec![5]).unwrap(),
            QualificationEvidenceDigest::new(vec![9]).unwrap(),
            EvidenceLineageToken(10),
        )
    }

    fn reference_runner(
        status: ShadowReferenceRunnerStatus,
        implementation_byte: u8,
        evidence: u128,
    ) -> ShadowReferenceRunnerQualification {
        ShadowReferenceRunnerQualification::new(
            ShadowReferenceRunnerKey::new(11, 1),
            ShadowReferenceRunnerProfileVersion(1),
            RepresentationKey::new(12, 1),
            implementation(implementation_byte),
            Some(capsule(13)),
            EvidenceLineageToken(evidence),
            status,
        )
    }

    fn coarse_runner(
        status: ShadowCoarseRunnerStatus,
        implementation_byte: u8,
        evidence: u128,
    ) -> ShadowCoarseRunnerQualification {
        ShadowCoarseRunnerQualification::new(
            ShadowCoarseRunnerKey::new(21, 1),
            ShadowCoarseRunnerProfileVersion(1),
            RepresentationKey::new(22, 1),
            implementation(implementation_byte),
            Some(capsule(23)),
            EvidenceLineageToken(evidence),
            status,
        )
    }

    fn reference_receipt(
        key: u128,
        status: ReviewedShadowRunnerQualificationStatus,
    ) -> ReviewedShadowRunnerQualificationReceipt {
        ReviewedShadowRunnerQualificationReceipt::new(
            ShadowRunnerQualificationReceiptKey::new(key, 1),
            ShadowRunnerQualificationReceiptRevision(1),
            ShadowRunnerQualificationPolicyKey::new(1, 1),
            ShadowRunnerBinding::Reference {
                runner: ShadowReferenceRunnerKey::new(11, 1),
                profile: ShadowReferenceRunnerProfileVersion(1),
                representation: RepresentationKey::new(12, 1),
            },
            implementation(14),
            Some(capsule(13)),
            EvidenceLineageToken(15),
            ShadowRunnerSemanticContractVersion(1),
            QualificationFixtureDigest::new(vec![6]).unwrap(),
            QualificationTranscriptDigest::new(vec![16]).unwrap(),
            ShadowStateManifestGrammarFingerprint::new(vec![7]).unwrap(),
            provenance(),
            status,
        )
    }

    fn coarse_receipt(
        key: u128,
        status: ReviewedShadowRunnerQualificationStatus,
    ) -> ReviewedShadowRunnerQualificationReceipt {
        ReviewedShadowRunnerQualificationReceipt::new(
            ShadowRunnerQualificationReceiptKey::new(key, 1),
            ShadowRunnerQualificationReceiptRevision(1),
            ShadowRunnerQualificationPolicyKey::new(1, 1),
            ShadowRunnerBinding::Coarse {
                runner: ShadowCoarseRunnerKey::new(21, 1),
                profile: ShadowCoarseRunnerProfileVersion(1),
                representation: RepresentationKey::new(22, 1),
            },
            implementation(24),
            Some(capsule(23)),
            EvidenceLineageToken(25),
            ShadowRunnerSemanticContractVersion(1),
            QualificationFixtureDigest::new(vec![6]).unwrap(),
            QualificationTranscriptDigest::new(vec![26]).unwrap(),
            ShadowStateManifestGrammarFingerprint::new(vec![7]).unwrap(),
            provenance(),
            status,
        )
    }

    fn registry_with(
        policy_status: ShadowRunnerQualificationPolicyStatus,
        receipts: impl IntoIterator<Item = ReviewedShadowRunnerQualificationReceipt>,
    ) -> ShadowRunnerQualificationRegistry {
        let mut builder = ShadowRunnerQualificationRegistryBuilder::new(
            ShadowRunnerQualificationRegistryKey::new(30, 1),
        );
        builder.register_policy(policy(policy_status)).unwrap();
        for receipt in receipts {
            builder.register_receipt(receipt).unwrap();
        }
        builder.seal().unwrap()
    }

    #[test]
    fn bootstrap_qualified_alone_cannot_resolve() {
        let registry = registry_with(ShadowRunnerQualificationPolicyStatus::Current, []);
        let result = registry.resolve_reference(&reference_runner(
            ShadowReferenceRunnerStatus::Qualified,
            14,
            15,
        ));
        assert_eq!(result, Err(ShadowRunnerQualificationError::NoExecutableQualification));
    }

    #[test]
    fn exact_reviewed_reference_receipt_resolves() {
        let registry = registry_with(
            ShadowRunnerQualificationPolicyStatus::Current,
            [reference_receipt(31, ReviewedShadowRunnerQualificationStatus::Current)],
        );
        let runner = reference_runner(ShadowReferenceRunnerStatus::Qualified, 14, 15);
        let executable = registry.resolve_reference(&runner).unwrap();
        assert_eq!(executable.receipt().key(), ShadowRunnerQualificationReceiptKey::new(31, 1));
        assert_eq!(executable.validate_reference_current(&registry, &runner), Ok(()));
    }

    #[test]
    fn implementation_drift_cannot_reuse_receipt() {
        let registry = registry_with(
            ShadowRunnerQualificationPolicyStatus::Current,
            [reference_receipt(31, ReviewedShadowRunnerQualificationStatus::Current)],
        );
        let result = registry.resolve_reference(&reference_runner(
            ShadowReferenceRunnerStatus::Qualified,
            99,
            15,
        ));
        assert_eq!(result, Err(ShadowRunnerQualificationError::NoExecutableQualification));
    }

    #[test]
    fn bootstrap_evidence_drift_cannot_reuse_receipt() {
        let registry = registry_with(
            ShadowRunnerQualificationPolicyStatus::Current,
            [reference_receipt(31, ReviewedShadowRunnerQualificationStatus::Current)],
        );
        let result = registry.resolve_reference(&reference_runner(
            ShadowReferenceRunnerStatus::Qualified,
            14,
            99,
        ));
        assert_eq!(result, Err(ShadowRunnerQualificationError::NoExecutableQualification));
    }

    #[test]
    fn revoked_bootstrap_runner_cannot_resolve() {
        let registry = registry_with(
            ShadowRunnerQualificationPolicyStatus::Current,
            [reference_receipt(31, ReviewedShadowRunnerQualificationStatus::Current)],
        );
        let result = registry.resolve_reference(&reference_runner(
            ShadowReferenceRunnerStatus::Revoked,
            14,
            15,
        ));
        assert!(matches!(
            result,
            Err(ShadowRunnerQualificationError::ReferenceBootstrapNotQualified { .. })
        ));
    }

    #[test]
    fn revoked_receipt_cannot_resolve() {
        let registry = registry_with(
            ShadowRunnerQualificationPolicyStatus::Current,
            [reference_receipt(31, ReviewedShadowRunnerQualificationStatus::Revoked)],
        );
        let result = registry.resolve_reference(&reference_runner(
            ShadowReferenceRunnerStatus::Qualified,
            14,
            15,
        ));
        assert_eq!(result, Err(ShadowRunnerQualificationError::NoExecutableQualification));
    }

    #[test]
    fn revoked_policy_cannot_resolve_current_receipt() {
        let registry = registry_with(
            ShadowRunnerQualificationPolicyStatus::Revoked,
            [reference_receipt(31, ReviewedShadowRunnerQualificationStatus::Current)],
        );
        let result = registry.resolve_reference(&reference_runner(
            ShadowReferenceRunnerStatus::Qualified,
            14,
            15,
        ));
        assert_eq!(result, Err(ShadowRunnerQualificationError::NoExecutableQualification));
    }

    #[test]
    fn ambiguous_current_receipts_fail_closed() {
        let registry = registry_with(
            ShadowRunnerQualificationPolicyStatus::Current,
            [
                reference_receipt(31, ReviewedShadowRunnerQualificationStatus::Current),
                reference_receipt(32, ReviewedShadowRunnerQualificationStatus::Current),
            ],
        );
        let result = registry.resolve_reference(&reference_runner(
            ShadowReferenceRunnerStatus::Qualified,
            14,
            15,
        ));
        assert_eq!(
            result,
            Err(ShadowRunnerQualificationError::AmbiguousExecutableQualification)
        );
    }

    #[test]
    fn wrong_producer_identity_rejects_at_seal() {
        let mut receipt = reference_receipt(31, ReviewedShadowRunnerQualificationStatus::Current);
        receipt.provenance.producer_identity =
            QualificationProducerIdentityDigest::new(vec![99]).unwrap();

        let mut builder = ShadowRunnerQualificationRegistryBuilder::new(
            ShadowRunnerQualificationRegistryKey::new(30, 1),
        );
        builder
            .register_policy(policy(ShadowRunnerQualificationPolicyStatus::Current))
            .unwrap();
        builder.register_receipt(receipt).unwrap();
        assert!(matches!(
            builder.seal(),
            Err(ShadowRunnerQualificationError::ProducerIdentityMismatch { .. })
        ));
    }

    #[test]
    fn wrong_state_manifest_grammar_rejects_at_seal() {
        let mut receipt = reference_receipt(31, ReviewedShadowRunnerQualificationStatus::Current);
        receipt.state_manifest_grammar =
            ShadowStateManifestGrammarFingerprint::new(vec![99]).unwrap();

        let mut builder = ShadowRunnerQualificationRegistryBuilder::new(
            ShadowRunnerQualificationRegistryKey::new(30, 1),
        );
        builder
            .register_policy(policy(ShadowRunnerQualificationPolicyStatus::Current))
            .unwrap();
        builder.register_receipt(receipt).unwrap();
        assert!(matches!(
            builder.seal(),
            Err(ShadowRunnerQualificationError::StateManifestGrammarMismatch { .. })
        ));
    }

    #[test]
    fn exact_reference_and_coarse_receipts_resolve_pair() {
        let registry = registry_with(
            ShadowRunnerQualificationPolicyStatus::Current,
            [
                reference_receipt(31, ReviewedShadowRunnerQualificationStatus::Current),
                coarse_receipt(32, ReviewedShadowRunnerQualificationStatus::Current),
            ],
        );
        let reference = reference_runner(ShadowReferenceRunnerStatus::Qualified, 14, 15);
        let coarse = coarse_runner(ShadowCoarseRunnerStatus::Qualified, 24, 25);
        let pair = registry.resolve_pair(&reference, &coarse).unwrap();
        assert_eq!(pair.validate_current(&registry, &reference, &coarse), Ok(()));
    }
}
