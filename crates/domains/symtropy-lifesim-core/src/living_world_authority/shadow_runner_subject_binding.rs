// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Independent reviewed-subject binding for executable Q2 shadow-runner authority.
//!
//! #701 carries the externally verified product subject inside
//! `VerifiedQualificationProvenance`, while #702 proves that the verifier-accepted
//! semantic claim matches the reviewed receipt. This module closes one further
//! review boundary: reviewers must independently name the exact immutable subject
//! they intended to approve for one exact receipt revision.
//!
//! Subject review is an authority adjunct, not an anchor capability. It cannot mint
//! `ShadowAnchorClaim`, cannot construct `ClosureValidationAnchor`, and cannot mutate
//! canonical ecology.

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
use super::shadow_runner_semantic_provenance::{
    ShadowRunnerSemanticProvenanceError, ShadowRunnerSemanticRegistry,
    VerifierBoundExecutableShadowRunnerPairAuthority,
    VerifierBoundExecutableShadowRunnerQualification,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowRunnerSubjectBindingRegistryKey {
    id: u128,
    version: u32,
}

impl ShadowRunnerSubjectBindingRegistryKey {
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
pub enum ReviewedQualificationSubjectBindingStatus {
    Current,
    Revoked,
    Superseded,
}

/// Independent review statement naming the exact immutable qualification subject
/// accepted for one exact receipt revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedQualificationSubjectBinding {
    receipt: ShadowRunnerQualificationReceiptKey,
    receipt_revision: ShadowRunnerQualificationReceiptRevision,
    subject: QualificationSubjectDigest,
    review_evidence: EvidenceLineageToken,
    status: ReviewedQualificationSubjectBindingStatus,
}

impl ReviewedQualificationSubjectBinding {
    pub fn new(
        receipt: ShadowRunnerQualificationReceiptKey,
        receipt_revision: ShadowRunnerQualificationReceiptRevision,
        subject: QualificationSubjectDigest,
        review_evidence: EvidenceLineageToken,
        status: ReviewedQualificationSubjectBindingStatus,
    ) -> Self {
        Self {
            receipt,
            receipt_revision,
            subject,
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

    pub const fn subject(&self) -> &QualificationSubjectDigest {
        &self.subject
    }

    pub const fn review_evidence(&self) -> EvidenceLineageToken {
        self.review_evidence
    }

    pub const fn status(&self) -> ReviewedQualificationSubjectBindingStatus {
        self.status
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowRunnerSubjectBindingAuthorityStamp {
    key: ShadowRunnerSubjectBindingRegistryKey,
    bindings: BTreeMap<ShadowRunnerQualificationReceiptKey, ReviewedQualificationSubjectBinding>,
}

impl ShadowRunnerSubjectBindingAuthorityStamp {
    pub const fn key(&self) -> ShadowRunnerSubjectBindingRegistryKey {
        self.key
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowRunnerSubjectBindingRegistryBuilder {
    key: ShadowRunnerSubjectBindingRegistryKey,
    bindings: BTreeMap<ShadowRunnerQualificationReceiptKey, ReviewedQualificationSubjectBinding>,
}

impl ShadowRunnerSubjectBindingRegistryBuilder {
    pub const fn new(key: ShadowRunnerSubjectBindingRegistryKey) -> Self {
        Self {
            key,
            bindings: BTreeMap::new(),
        }
    }

    pub fn register_binding(
        &mut self,
        binding: ReviewedQualificationSubjectBinding,
    ) -> Result<(), ShadowRunnerSubjectBindingError> {
        use std::collections::btree_map::Entry;
        match self.bindings.entry(binding.receipt()) {
            Entry::Vacant(entry) => {
                entry.insert(binding);
                Ok(())
            }
            Entry::Occupied(entry) if entry.get() == &binding => Ok(()),
            Entry::Occupied(entry) => Err(ShadowRunnerSubjectBindingError::ConflictingBinding {
                receipt: *entry.key(),
            }),
        }
    }

    pub fn seal(self) -> ShadowRunnerSubjectBindingRegistry {
        let authority = ShadowRunnerSubjectBindingAuthorityStamp {
            key: self.key,
            bindings: self.bindings.clone(),
        };
        ShadowRunnerSubjectBindingRegistry {
            authority,
            bindings: self.bindings,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowRunnerSubjectBindingRegistry {
    authority: ShadowRunnerSubjectBindingAuthorityStamp,
    bindings: BTreeMap<ShadowRunnerQualificationReceiptKey, ReviewedQualificationSubjectBinding>,
}

impl ShadowRunnerSubjectBindingRegistry {
    pub const fn authority_stamp(&self) -> &ShadowRunnerSubjectBindingAuthorityStamp {
        &self.authority
    }

    fn resolve_one(
        &self,
        executable: &VerifierBoundExecutableShadowRunnerQualification,
    ) -> Result<SubjectBoundExecutableShadowRunnerQualification, ShadowRunnerSubjectBindingError>
    {
        let receipt = executable.executable().receipt();
        let binding = self
            .bindings
            .get(&receipt.key())
            .ok_or(ShadowRunnerSubjectBindingError::MissingBinding {
                receipt: receipt.key(),
            })?;

        if binding.status() != ReviewedQualificationSubjectBindingStatus::Current {
            return Err(ShadowRunnerSubjectBindingError::BindingNotCurrent {
                receipt: receipt.key(),
                status: binding.status(),
            });
        }
        if binding.receipt_revision() != receipt.revision() {
            return Err(ShadowRunnerSubjectBindingError::ReceiptRevisionMismatch {
                receipt: receipt.key(),
                expected: receipt.revision(),
                actual: binding.receipt_revision(),
            });
        }
        if binding.subject() != receipt.provenance().subject() {
            return Err(ShadowRunnerSubjectBindingError::SubjectMismatch {
                receipt: receipt.key(),
            });
        }

        Ok(SubjectBoundExecutableShadowRunnerQualification {
            authority: self.authority.clone(),
            executable: executable.clone(),
            binding: binding.clone(),
        })
    }

    /// Resolve subject-bound authority only after the underlying #701/#702 pair
    /// has revalidated against current qualification and semantic registries.
    pub fn resolve_pair_current(
        &self,
        executable_pair: &VerifierBoundExecutableShadowRunnerPairAuthority,
        semantic_registry: &ShadowRunnerSemanticRegistry,
        qualification_registry: &ShadowRunnerQualificationRegistry,
        reference_runner: &ShadowReferenceRunnerQualification,
        coarse_runner: &ShadowCoarseRunnerQualification,
    ) -> Result<SubjectBoundExecutableShadowRunnerPairAuthority, ShadowRunnerSubjectBindingError>
    {
        executable_pair
            .validate_current(
                semantic_registry,
                qualification_registry,
                reference_runner,
                coarse_runner,
            )
            .map_err(ShadowRunnerSubjectBindingError::RunnerSemantics)?;

        Ok(SubjectBoundExecutableShadowRunnerPairAuthority {
            authority: self.authority.clone(),
            executable_pair: executable_pair.clone(),
            reference: self.resolve_one(executable_pair.reference())?,
            coarse: self.resolve_one(executable_pair.coarse())?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectBoundExecutableShadowRunnerQualification {
    authority: ShadowRunnerSubjectBindingAuthorityStamp,
    executable: VerifierBoundExecutableShadowRunnerQualification,
    binding: ReviewedQualificationSubjectBinding,
}

impl SubjectBoundExecutableShadowRunnerQualification {
    pub const fn authority_stamp(&self) -> &ShadowRunnerSubjectBindingAuthorityStamp {
        &self.authority
    }

    pub const fn executable(&self) -> &VerifierBoundExecutableShadowRunnerQualification {
        &self.executable
    }

    pub const fn binding(&self) -> &ReviewedQualificationSubjectBinding {
        &self.binding
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectBoundExecutableShadowRunnerPairAuthority {
    authority: ShadowRunnerSubjectBindingAuthorityStamp,
    executable_pair: VerifierBoundExecutableShadowRunnerPairAuthority,
    reference: SubjectBoundExecutableShadowRunnerQualification,
    coarse: SubjectBoundExecutableShadowRunnerQualification,
}

impl SubjectBoundExecutableShadowRunnerPairAuthority {
    pub const fn authority_stamp(&self) -> &ShadowRunnerSubjectBindingAuthorityStamp {
        &self.authority
    }

    pub const fn executable_pair(&self) -> &VerifierBoundExecutableShadowRunnerPairAuthority {
        &self.executable_pair
    }

    pub const fn reference(&self) -> &SubjectBoundExecutableShadowRunnerQualification {
        &self.reference
    }

    pub const fn coarse(&self) -> &SubjectBoundExecutableShadowRunnerQualification {
        &self.coarse
    }

    pub fn validate_current(
        &self,
        subject_registry: &ShadowRunnerSubjectBindingRegistry,
        semantic_registry: &ShadowRunnerSemanticRegistry,
        qualification_registry: &ShadowRunnerQualificationRegistry,
        reference_runner: &ShadowReferenceRunnerQualification,
        coarse_runner: &ShadowCoarseRunnerQualification,
    ) -> Result<(), ShadowRunnerSubjectBindingError> {
        if subject_registry.authority_stamp() != &self.authority {
            return Err(ShadowRunnerSubjectBindingError::SubjectAuthorityChanged);
        }
        let current = subject_registry.resolve_pair_current(
            &self.executable_pair,
            semantic_registry,
            qualification_registry,
            reference_runner,
            coarse_runner,
        )?;
        if current != *self {
            return Err(ShadowRunnerSubjectBindingError::SubjectBoundPairStale);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShadowRunnerSubjectBindingError {
    ConflictingBinding {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    MissingBinding {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    BindingNotCurrent {
        receipt: ShadowRunnerQualificationReceiptKey,
        status: ReviewedQualificationSubjectBindingStatus,
    },
    ReceiptRevisionMismatch {
        receipt: ShadowRunnerQualificationReceiptKey,
        expected: ShadowRunnerQualificationReceiptRevision,
        actual: ShadowRunnerQualificationReceiptRevision,
    },
    SubjectMismatch {
        receipt: ShadowRunnerQualificationReceiptKey,
    },
    RunnerSemantics(ShadowRunnerSemanticProvenanceError),
    SubjectAuthorityChanged,
    SubjectBoundPairStale,
}

impl fmt::Display for ShadowRunnerSubjectBindingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConflictingBinding { receipt } => write!(
                f,
                "conflicting reviewed qualification-subject binding for receipt {}@{}",
                receipt.id(),
                receipt.version()
            ),
            Self::MissingBinding { receipt } => write!(
                f,
                "qualification receipt {}@{} has no reviewed subject binding",
                receipt.id(),
                receipt.version()
            ),
            Self::BindingNotCurrent { receipt, status } => write!(
                f,
                "reviewed subject binding for qualification receipt {}@{} is {status:?}, not Current",
                receipt.id(),
                receipt.version()
            ),
            Self::ReceiptRevisionMismatch {
                receipt,
                expected,
                actual,
            } => write!(
                f,
                "reviewed subject binding for qualification receipt {}@{} binds revision {}, current receipt is revision {}",
                receipt.id(),
                receipt.version(),
                actual.0,
                expected.0
            ),
            Self::SubjectMismatch { receipt } => write!(
                f,
                "reviewed subject for qualification receipt {}@{} differs from verifier-accepted provenance subject",
                receipt.id(),
                receipt.version()
            ),
            Self::RunnerSemantics(error) => {
                write!(f, "executable runner semantic authority: {error}")
            }
            Self::SubjectAuthorityChanged => {
                write!(f, "reviewed qualification-subject authority changed")
            }
            Self::SubjectBoundPairStale => {
                write!(f, "subject-bound executable runner pair authority is stale")
            }
        }
    }
}

impl Error for ShadowRunnerSubjectBindingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::RunnerSemantics(error) => Some(error),
            _ => None,
        }
    }
}
