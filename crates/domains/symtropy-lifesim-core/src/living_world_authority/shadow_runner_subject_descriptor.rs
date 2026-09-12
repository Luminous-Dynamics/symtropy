// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Domain-separated qualification-subject authority for Q2 shadow runners.
//!
//! #729 proves that independent review selected the same bare subject digest carried
//! by externally verified provenance. Bare bytes are still ambiguous: the same
//! bytes could be interpreted as a Git commit id, tree id, artifact digest, or a
//! future content identifier. This successor authority requires verifier-side and
//! review-side agreement on the complete descriptor: subject kind, digest grammar,
//! schema version, and the already-authorized digest bytes.
//!
//! This remains evidence authority only. It cannot construct `ShadowAnchorClaim`,
//! cannot construct `ClosureValidationAnchor`, and cannot mutate canonical ecology.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::information::EvidenceLineageToken;

use super::shadow_execution_lineage::ShadowReferenceRunnerQualification;
use super::shadow_paired_execution::ShadowCoarseRunnerQualification;
use super::shadow_runner_qualification::{
    QualificationSubjectDigest, ShadowRunnerQualificationReceiptKey,
    ShadowRunnerQualificationReceiptRevision, ShadowRunnerQualificationRegistry,
};
use super::shadow_runner_semantic_provenance::ShadowRunnerSemanticRegistry;
use super::shadow_runner_subject_binding::{
    ShadowRunnerSubjectBindingError, ShadowRunnerSubjectBindingRegistry,
    SubjectBoundExecutableShadowRunnerPairAuthority, SubjectBoundExecutableShadowRunnerQualification,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum QualificationSubjectKind {
    ProductCommit,
    ProductTree,
    QualificationManifest,
    Artifact,
    Other { namespace: u128, version: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum QualificationDigestGrammar {
    GitSha1ObjectIdV1,
    Sha256V1,
    Blake3V1,
    XeniaContentIdV1,
    Other { namespace: u128, version: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QualificationSubjectDescriptor {
    kind: QualificationSubjectKind,
    digest_grammar: QualificationDigestGrammar,
    schema_version: u32,
    digest: QualificationSubjectDigest,
}

impl QualificationSubjectDescriptor {
    pub fn new(
        kind: QualificationSubjectKind,
        digest_grammar: QualificationDigestGrammar,
        schema_version: u32,
        digest: QualificationSubjectDigest,
    ) -> Self {
        Self {
            kind,
            digest_grammar,
            schema_version,
            digest,
        }
    }

    pub const fn kind(&self) -> QualificationSubjectKind {
        self.kind
    }

    pub const fn digest_grammar(&self) -> QualificationDigestGrammar {
        self.digest_grammar
    }

    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub const fn digest(&self) -> &QualificationSubjectDigest {
        &self.digest
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowSubjectDescriptorVerificationRegistryKey {
    id: u128,
    version: u32,
}

impl ShadowSubjectDescriptorVerificationRegistryKey {
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
pub enum VerifiedQualificationSubjectDescriptorStatus {
    Current,
    Revoked,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedQualificationSubjectDescriptorClaim {
    receipt: ShadowRunnerQualificationReceiptKey,
    receipt_revision: ShadowRunnerQualificationReceiptRevision,
    descriptor: QualificationSubjectDescriptor,
    verification_evidence: EvidenceLineageToken,
    status: VerifiedQualificationSubjectDescriptorStatus,
}

impl VerifiedQualificationSubjectDescriptorClaim {
    pub fn new(
        receipt: ShadowRunnerQualificationReceiptKey,
        receipt_revision: ShadowRunnerQualificationReceiptRevision,
        descriptor: QualificationSubjectDescriptor,
        verification_evidence: EvidenceLineageToken,
        status: VerifiedQualificationSubjectDescriptorStatus,
    ) -> Self {
        Self {
            receipt,
            receipt_revision,
            descriptor,
            verification_evidence,
            status,
        }
    }

    pub const fn receipt(&self) -> ShadowRunnerQualificationReceiptKey {
        self.receipt
    }

    pub const fn receipt_revision(&self) -> ShadowRunnerQualificationReceiptRevision {
        self.receipt_revision
    }

    pub const fn descriptor(&self) -> &QualificationSubjectDescriptor {
        &self.descriptor
    }

    pub const fn verification_evidence(&self) -> EvidenceLineageToken {
        self.verification_evidence
    }

    pub const fn status(&self) -> VerifiedQualificationSubjectDescriptorStatus {
        self.status
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowSubjectDescriptorVerificationAuthorityStamp {
    key: ShadowSubjectDescriptorVerificationRegistryKey,
    claims: BTreeMap<ShadowRunnerQualificationReceiptKey, VerifiedQualificationSubjectDescriptorClaim>,
}

impl ShadowSubjectDescriptorVerificationAuthorityStamp {
    pub const fn key(&self) -> ShadowSubjectDescriptorVerificationRegistryKey {
        self.key
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowSubjectDescriptorVerificationRegistryBuilder {
    key: ShadowSubjectDescriptorVerificationRegistryKey,
    claims: BTreeMap<ShadowRunnerQualificationReceiptKey, VerifiedQualificationSubjectDescriptorClaim>,
}

impl ShadowSubjectDescriptorVerificationRegistryBuilder {
    pub const fn new(key: ShadowSubjectDescriptorVerificationRegistryKey) -> Self {
        Self {
            key,
            claims: BTreeMap::new(),
        }
    }

    pub fn register_claim(
        &mut self,
        claim: VerifiedQualificationSubjectDescriptorClaim,
    ) -> Result<(), ShadowRunnerSubjectDescriptorError> {
        use std::collections::btree_map::Entry;
        match self.claims.entry(claim.receipt()) {
            Entry::Vacant(entry) => {
                entry.insert(claim);
                Ok(())
            }
            Entry::Occupied(entry) if entry.get() == &claim => Ok(()),
            Entry::Occupied(entry) => Err(ShadowRunnerSubjectDescriptorError::ConflictingVerifiedClaim {
                receipt: *entry.key(),
            }),
        }
    }

    pub fn seal(self) -> ShadowSubjectDescriptorVerificationRegistry {
        let authority = ShadowSubjectDescriptorVerificationAuthorityStamp {
            key: self.key,
            claims: self.claims.clone(),
        };
        ShadowSubjectDescriptorVerificationRegistry {
            authority,
            claims: self.claims,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowSubjectDescriptorVerificationRegistry {
    authority: ShadowSubjectDescriptorVerificationAuthorityStamp,
    claims: BTreeMap<ShadowRunnerQualificationReceiptKey, VerifiedQualificationSubjectDescriptorClaim>,
}

impl ShadowSubjectDescriptorVerificationRegistry {
    pub const fn authority_stamp(&self) -> &ShadowSubjectDescriptorVerificationAuthorityStamp {
        &self.authority
    }

    fn resolve_one(
        &self,
        lower: &SubjectBoundExecutableShadowRunnerQualification,
    ) -> Result<VerifiedSubjectDescriptorQualification, ShadowRunnerSubjectDescriptorError> {
        let receipt = lower.executable().executable().receipt();
        let claim = self
            .claims
            .get(&receipt.key())
            .ok_or(ShadowRunnerSubjectDescriptorError::MissingVerifiedClaim {
                receipt: receipt.key(),
            })?;

        if claim.status() != VerifiedQualificationSubjectDescriptorStatus::Current {
            return Err(ShadowRunnerSubjectDescriptorError::VerifiedClaimNotCurrent {
                receipt: receipt.key(),
                status: claim.status(),
            });
        }
        if claim.receipt_revision() != receipt.revision() {
            return Err(ShadowRunnerSubjectDescriptorError::VerifiedReceiptRevisionMismatch {
                receipt: receipt.key(),
                expected: receipt.revision(),
                actual: claim.receipt_revision(),
            });
        }
        if claim.descriptor().digest() != receipt.provenance().subject() {
            return Err(ShadowRunnerSubjectDescriptorError::VerifiedDigestMismatch {
                receipt: receipt.key(),
            });
        }

        Ok(VerifiedSubjectDescriptorQualification {
            authority: self.authority.clone(),
            lower: lower.clone(),
            claim: claim.clone(),
        })
    }

    pub fn resolve_pair(
        &self,
        lower: &SubjectBoundExecutableShadowRunnerPairAuthority,
    ) -> Result<VerifiedSubjectDescriptorPairAuthority, ShadowRunnerSubjectDescriptorError> {
        Ok(VerifiedSubjectDescriptorPairAuthority {
            authority: self.authority.clone(),
            lower: lower.clone(),
            reference: self.resolve_one(lower.reference())?,
            coarse: self.resolve_one(lower.coarse())?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedSubjectDescriptorQualification {
    authority: ShadowSubjectDescriptorVerificationAuthorityStamp,
    lower: SubjectBoundExecutableShadowRunnerQualification,
    claim: VerifiedQualificationSubjectDescriptorClaim,
}

impl VerifiedSubjectDescriptorQualification {
    pub const fn claim(&self) -> &VerifiedQualificationSubjectDescriptorClaim {
        &self.claim
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedSubjectDescriptorPairAuthority {
    authority: ShadowSubjectDescriptorVerificationAuthorityStamp,
    lower: SubjectBoundExecutableShadowRunnerPairAuthority,
    reference: VerifiedSubjectDescriptorQualification,
    coarse: VerifiedSubjectDescriptorQualification,
}

impl VerifiedSubjectDescriptorPairAuthority {
    pub const fn authority_stamp(&self) -> &ShadowSubjectDescriptorVerificationAuthorityStamp {
        &self.authority
    }

    pub const fn lower(&self) -> &SubjectBoundExecutableShadowRunnerPairAuthority {
        &self.lower
    }

    pub const fn reference(&self) -> &VerifiedSubjectDescriptorQualification {
        &self.reference
    }

    pub const fn coarse(&self) -> &VerifiedSubjectDescriptorQualification {
        &self.coarse
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowSubjectDescriptorReviewRegistryKey {
    id: u128,
    version: u32,
}

impl ShadowSubjectDescriptorReviewRegistryKey {
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
pub enum ReviewedQualificationSubjectDescriptorStatus {
    Current,
    Revoked,
    Superseded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedQualificationSubjectDescriptorBinding {
    receipt: ShadowRunnerQualificationReceiptKey,
    receipt_revision: ShadowRunnerQualificationReceiptRevision,
    descriptor: QualificationSubjectDescriptor,
    review_evidence: EvidenceLineageToken,
    status: ReviewedQualificationSubjectDescriptorStatus,
}

impl ReviewedQualificationSubjectDescriptorBinding {
    pub fn new(
        receipt: ShadowRunnerQualificationReceiptKey,
        receipt_revision: ShadowRunnerQualificationReceiptRevision,
        descriptor: QualificationSubjectDescriptor,
        review_evidence: EvidenceLineageToken,
        status: ReviewedQualificationSubjectDescriptorStatus,
    ) -> Self {
        Self {
            receipt,
            receipt_revision,
            descriptor,
            review_evidence,
            status,
        }
    }

    pub const fn receipt(&self) -> ShadowRunnerQualificationReceiptKey {
        self.receipt
    }

    pub const fn receipt_revision(&self) -> ShadowRunnerQualificationReceiptRevision {
        self.receipt_revision
    }

    pub const fn descriptor(&self) -> &QualificationSubjectDescriptor {
        &self.descriptor
    }

    pub const fn review_evidence(&self) -> EvidenceLineageToken {
        self.review_evidence
    }

    pub const fn status(&self) -> ReviewedQualificationSubjectDescriptorStatus {
        self.status
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowSubjectDescriptorReviewAuthorityStamp {
    key: ShadowSubjectDescriptorReviewRegistryKey,
    bindings: BTreeMap<ShadowRunnerQualificationReceiptKey, ReviewedQualificationSubjectDescriptorBinding>,
}

impl ShadowSubjectDescriptorReviewAuthorityStamp {
    pub const fn key(&self) -> ShadowSubjectDescriptorReviewRegistryKey {
        self.key
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowSubjectDescriptorReviewRegistryBuilder {
    key: ShadowSubjectDescriptorReviewRegistryKey,
    bindings: BTreeMap<ShadowRunnerQualificationReceiptKey, ReviewedQualificationSubjectDescriptorBinding>,
}

impl ShadowSubjectDescriptorReviewRegistryBuilder {
    pub const fn new(key: ShadowSubjectDescriptorReviewRegistryKey) -> Self {
        Self {
            key,
            bindings: BTreeMap::new(),
        }
    }

    pub fn register_binding(
        &mut self,
        binding: ReviewedQualificationSubjectDescriptorBinding,
    ) -> Result<(), ShadowRunnerSubjectDescriptorError> {
        use std::collections::btree_map::Entry;
        match self.bindings.entry(binding.receipt()) {
            Entry::Vacant(entry) => {
                entry.insert(binding);
                Ok(())
            }
            Entry::Occupied(entry) if entry.get() == &binding => Ok(()),
            Entry::Occupied(entry) => Err(ShadowRunnerSubjectDescriptorError::ConflictingReviewedBinding {
                receipt: *entry.key(),
            }),
        }
    }

    pub fn seal(self) -> ShadowSubjectDescriptorReviewRegistry {
        let authority = ShadowSubjectDescriptorReviewAuthorityStamp {
            key: self.key,
            bindings: self.bindings.clone(),
        };
        ShadowSubjectDescriptorReviewRegistry {
            authority,
            bindings: self.bindings,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowSubjectDescriptorReviewRegistry {
    authority: ShadowSubjectDescriptorReviewAuthorityStamp,
    bindings: BTreeMap<ShadowRunnerQualificationReceiptKey, ReviewedQualificationSubjectDescriptorBinding>,
}

impl ShadowSubjectDescriptorReviewRegistry {
    pub const fn authority_stamp(&self) -> &ShadowSubjectDescriptorReviewAuthorityStamp {
        &self.authority
    }

    fn resolve_one(
        &self,
        verified: &VerifiedSubjectDescriptorQualification,
    ) -> Result<DescriptorBoundExecutableShadowRunnerQualification, ShadowRunnerSubjectDescriptorError> {
        let receipt = verified.lower.executable().executable().receipt();
        let binding = self
            .bindings
            .get(&receipt.key())
            .ok_or(ShadowRunnerSubjectDescriptorError::MissingReviewedBinding {
                receipt: receipt.key(),
            })?;

        if binding.status() != ReviewedQualificationSubjectDescriptorStatus::Current {
            return Err(ShadowRunnerSubjectDescriptorError::ReviewedBindingNotCurrent {
                receipt: receipt.key(),
                status: binding.status(),
            });
        }
        if binding.receipt_revision() != receipt.revision() {
            return Err(ShadowRunnerSubjectDescriptorError::ReviewedReceiptRevisionMismatch {
                receipt: receipt.key(),
                expected: receipt.revision(),
                actual: binding.receipt_revision(),
            });
        }
        if binding.descriptor() != verified.claim().descriptor() {
            return Err(ShadowRunnerSubjectDescriptorError::DescriptorMismatch {
                receipt: receipt.key(),
            });
        }

        Ok(DescriptorBoundExecutableShadowRunnerQualification {
            verification_authority: verified.authority.clone(),
            review_authority: self.authority.clone(),
            verified: verified.clone(),
            binding: binding.clone(),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn resolve_pair_current(
        &self,
        verification_registry: &ShadowSubjectDescriptorVerificationRegistry,
        lower: &SubjectBoundExecutableShadowRunnerPairAuthority,
        subject_registry: &ShadowRunnerSubjectBindingRegistry,
        semantic_registry: &ShadowRunnerSemanticRegistry,
        qualification_registry: &ShadowRunnerQualificationRegistry,
        reference_runner: &ShadowReferenceRunnerQualification,
        coarse_runner: &ShadowCoarseRunnerQualification,
    ) -> Result<DescriptorBoundExecutableShadowRunnerPairAuthority, ShadowRunnerSubjectDescriptorError> {
        lower
            .validate_current(
                subject_registry,
                semantic_registry,
                qualification_registry,
                reference_runner,
                coarse_runner,
            )
            .map_err(ShadowRunnerSubjectDescriptorError::LowerSubjectAuthority)?;

        let verified = verification_registry.resolve_pair(lower)?;
        Ok(DescriptorBoundExecutableShadowRunnerPairAuthority {
            verification_authority: verification_registry.authority_stamp().clone(),
            review_authority: self.authority.clone(),
            lower: lower.clone(),
            reference: self.resolve_one(verified.reference())?,
            coarse: self.resolve_one(verified.coarse())?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescriptorBoundExecutableShadowRunnerQualification {
    verification_authority: ShadowSubjectDescriptorVerificationAuthorityStamp,
    review_authority: ShadowSubjectDescriptorReviewAuthorityStamp,
    verified: VerifiedSubjectDescriptorQualification,
    binding: ReviewedQualificationSubjectDescriptorBinding,
}

impl DescriptorBoundExecutableShadowRunnerQualification {
    pub const fn descriptor(&self) -> &QualificationSubjectDescriptor {
        self.binding.descriptor()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescriptorBoundExecutableShadowRunnerPairAuthority {
    verification_authority: ShadowSubjectDescriptorVerificationAuthorityStamp,
    review_authority: ShadowSubjectDescriptorReviewAuthorityStamp,
    lower: SubjectBoundExecutableShadowRunnerPairAuthority,
    reference: DescriptorBoundExecutableShadowRunnerQualification,
    coarse: DescriptorBoundExecutableShadowRunnerQualification,
}

impl DescriptorBoundExecutableShadowRunnerPairAuthority {
    pub const fn lower(&self) -> &SubjectBoundExecutableShadowRunnerPairAuthority {
        &self.lower
    }

    pub const fn reference(&self) -> &DescriptorBoundExecutableShadowRunnerQualification {
        &self.reference
    }

    pub const fn coarse(&self) -> &DescriptorBoundExecutableShadowRunnerQualification {
        &self.coarse
    }

    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        verification_registry: &ShadowSubjectDescriptorVerificationRegistry,
        review_registry: &ShadowSubjectDescriptorReviewRegistry,
        subject_registry: &ShadowRunnerSubjectBindingRegistry,
        semantic_registry: &ShadowRunnerSemanticRegistry,
        qualification_registry: &ShadowRunnerQualificationRegistry,
        reference_runner: &ShadowReferenceRunnerQualification,
        coarse_runner: &ShadowCoarseRunnerQualification,
    ) -> Result<(), ShadowRunnerSubjectDescriptorError> {
        if verification_registry.authority_stamp() != &self.verification_authority {
            return Err(ShadowRunnerSubjectDescriptorError::VerificationAuthorityChanged);
        }
        if review_registry.authority_stamp() != &self.review_authority {
            return Err(ShadowRunnerSubjectDescriptorError::ReviewAuthorityChanged);
        }
        let current = review_registry.resolve_pair_current(
            verification_registry,
            &self.lower,
            subject_registry,
            semantic_registry,
            qualification_registry,
            reference_runner,
            coarse_runner,
        )?;
        if current != *self {
            return Err(ShadowRunnerSubjectDescriptorError::DescriptorBoundPairStale);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShadowRunnerSubjectDescriptorError {
    ConflictingVerifiedClaim {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    MissingVerifiedClaim {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    VerifiedClaimNotCurrent {
        receipt: ShadowRunnerQualificationReceiptKey,
        status: VerifiedQualificationSubjectDescriptorStatus,
    },
    VerifiedReceiptRevisionMismatch {
        receipt: ShadowRunnerQualificationReceiptKey,
        expected: ShadowRunnerQualificationReceiptRevision,
        actual: ShadowRunnerQualificationReceiptRevision,
    },
    VerifiedDigestMismatch {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    ConflictingReviewedBinding {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    MissingReviewedBinding {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    ReviewedBindingNotCurrent {
        receipt: ShadowRunnerQualificationReceiptKey,
        status: ReviewedQualificationSubjectDescriptorStatus,
    },
    ReviewedReceiptRevisionMismatch {
        receipt: ShadowRunnerQualificationReceiptKey,
        expected: ShadowRunnerQualificationReceiptRevision,
        actual: ShadowRunnerQualificationReceiptRevision,
    },
    DescriptorMismatch {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    LowerSubjectAuthority(ShadowRunnerSubjectBindingError),
    VerificationAuthorityChanged,
    ReviewAuthorityChanged,
    DescriptorBoundPairStale,
}

impl fmt::Display for ShadowRunnerSubjectDescriptorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConflictingVerifiedClaim { receipt } => write!(
                f,
                "conflicting verified subject-descriptor claim for receipt {}@{}",
                receipt.id(),
                receipt.version()
            ),
            Self::MissingVerifiedClaim { receipt } => write!(
                f,
                "qualification receipt {}@{} has no verified subject-descriptor claim",
                receipt.id(),
                receipt.version()
            ),
            Self::VerifiedClaimNotCurrent { receipt, status } => write!(
                f,
                "verified subject-descriptor claim for receipt {}@{} is {status:?}, not Current",
                receipt.id(),
                receipt.version()
            ),
            Self::VerifiedReceiptRevisionMismatch { receipt, expected, actual } => write!(
                f,
                "verified subject descriptor for receipt {}@{} binds revision {}, current receipt is revision {}",
                receipt.id(),
                receipt.version(),
                actual.0,
                expected.0
            ),
            Self::VerifiedDigestMismatch { receipt } => write!(
                f,
                "verified subject descriptor for receipt {}@{} does not carry the verifier-accepted bare digest",
                receipt.id(),
                receipt.version()
            ),
            Self::ConflictingReviewedBinding { receipt } => write!(
                f,
                "conflicting reviewed subject-descriptor binding for receipt {}@{}",
                receipt.id(),
                receipt.version()
            ),
            Self::MissingReviewedBinding { receipt } => write!(
                f,
                "qualification receipt {}@{} has no reviewed subject-descriptor binding",
                receipt.id(),
                receipt.version()
            ),
            Self::ReviewedBindingNotCurrent { receipt, status } => write!(
                f,
                "reviewed subject-descriptor binding for receipt {}@{} is {status:?}, not Current",
                receipt.id(),
                receipt.version()
            ),
            Self::ReviewedReceiptRevisionMismatch { receipt, expected, actual } => write!(
                f,
                "reviewed subject descriptor for receipt {}@{} binds revision {}, current receipt is revision {}",
                receipt.id(),
                receipt.version(),
                actual.0,
                expected.0
            ),
            Self::DescriptorMismatch { receipt } => write!(
                f,
                "reviewed and verifier-accepted subject descriptors differ for receipt {}@{}",
                receipt.id(),
                receipt.version()
            ),
            Self::LowerSubjectAuthority(error) => write!(f, "bare subject authority: {error}"),
            Self::VerificationAuthorityChanged => {
                write!(f, "verified qualification-subject descriptor authority changed")
            }
            Self::ReviewAuthorityChanged => {
                write!(f, "reviewed qualification-subject descriptor authority changed")
            }
            Self::DescriptorBoundPairStale => {
                write!(f, "descriptor-bound executable runner pair authority is stale")
            }
        }
    }
}

impl Error for ShadowRunnerSubjectDescriptorError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::LowerSubjectAuthority(error) => Some(error),
            _ => None,
        }
    }
}
