use crate::information::{EvidenceLineageToken, RepresentationKey};
use crate::living_world_authority::shadow_execution_lineage::{
    ShadowReferenceRunnerKey, ShadowReferenceRunnerProfileVersion,
    ShadowReferenceRunnerQualification, ShadowReferenceRunnerStatus,
};
use crate::living_world_authority::shadow_paired_execution::{
    ShadowCoarseRunnerKey, ShadowCoarseRunnerProfileVersion, ShadowCoarseRunnerQualification,
    ShadowCoarseRunnerStatus,
};
use crate::living_world_authority::shadow_runner_qualification::{
    QualificationEvidenceDigest, QualificationFixtureDigest, QualificationPredicateDigest,
    QualificationProducerIdentityDigest, QualificationProviderKey, QualificationSubjectDigest,
    QualificationTranscriptDigest, QualificationVerifierPolicyKey,
    ReviewedShadowRunnerQualificationReceipt, ReviewedShadowRunnerQualificationStatus,
    ShadowRunnerBinding, ShadowRunnerQualificationPolicy, ShadowRunnerQualificationPolicyKey,
    ShadowRunnerQualificationPolicyStatus, ShadowRunnerQualificationReceiptKey,
    ShadowRunnerQualificationReceiptRevision, ShadowRunnerQualificationRegistry,
    ShadowRunnerQualificationRegistryBuilder, ShadowRunnerQualificationRegistryKey,
    ShadowRunnerSemanticContractVersion, ShadowStateManifestGrammarFingerprint,
    VerifiedQualificationProvenance,
};
use crate::living_world_authority::shadow_runner_semantic_provenance::{
    ShadowRunnerSemanticRegistry, ShadowRunnerSemanticRegistryBuilder,
    ShadowRunnerSemanticRegistryKey, VerifiedRunnerSemanticClaim,
    VerifiedRunnerSemanticClaimStatus, VerifierBoundExecutableShadowRunnerPairAuthority,
};
use crate::living_world_authority::shadow_runner_subject_binding::{
    ReviewedQualificationSubjectBinding, ReviewedQualificationSubjectBindingStatus,
    ShadowRunnerSubjectBindingRegistry, ShadowRunnerSubjectBindingRegistryBuilder,
    ShadowRunnerSubjectBindingRegistryKey, SubjectBoundExecutableShadowRunnerPairAuthority,
};
use crate::living_world_authority::shadow_validation::{
    ShadowExecutionCapsuleFingerprint, ShadowImplementationFingerprint,
};

pub(super) fn implementation(byte: u8) -> ShadowImplementationFingerprint {
    ShadowImplementationFingerprint::new(vec![byte]).unwrap()
}

pub(super) fn capsule(byte: u8) -> ShadowExecutionCapsuleFingerprint {
    ShadowExecutionCapsuleFingerprint::new(vec![byte]).unwrap()
}

pub(super) fn provenance(subject: u8) -> VerifiedQualificationProvenance {
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

pub(super) fn reference_runner() -> ShadowReferenceRunnerQualification {
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

pub(super) fn coarse_runner() -> ShadowCoarseRunnerQualification {
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

pub(super) fn qualification_registry(key: u128) -> ShadowRunnerQualificationRegistry {
    let mut builder = ShadowRunnerQualificationRegistryBuilder::new(
        ShadowRunnerQualificationRegistryKey::new(key, 1),
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

fn semantic_claim(
    receipt_key: u128,
    transcript: u8,
    subject: u8,
    status: VerifiedRunnerSemanticClaimStatus,
) -> VerifiedRunnerSemanticClaim {
    VerifiedRunnerSemanticClaim::new(
        ShadowRunnerQualificationReceiptKey::new(receipt_key, 1),
        ShadowRunnerQualificationReceiptRevision(1),
        provenance(subject),
        ShadowRunnerSemanticContractVersion(1),
        QualificationFixtureDigest::new(vec![7]).unwrap(),
        QualificationTranscriptDigest::new(vec![transcript]).unwrap(),
        ShadowStateManifestGrammarFingerprint::new(vec![8]).unwrap(),
        EvidenceLineageToken(60 + receipt_key),
        status,
    )
}

pub(super) fn semantic_registry(key: u128) -> ShadowRunnerSemanticRegistry {
    let mut builder = ShadowRunnerSemanticRegistryBuilder::new(
        ShadowRunnerSemanticRegistryKey::new(key, 1),
    );
    builder
        .register_claim(semantic_claim(
            41,
            42,
            43,
            VerifiedRunnerSemanticClaimStatus::Current,
        ))
        .unwrap();
    builder
        .register_claim(semantic_claim(
            51,
            52,
            53,
            VerifiedRunnerSemanticClaimStatus::Current,
        ))
        .unwrap();
    builder.seal()
}

pub(super) fn verifier_bound_pair(
    qualifications: &ShadowRunnerQualificationRegistry,
    semantics: &ShadowRunnerSemanticRegistry,
) -> VerifierBoundExecutableShadowRunnerPairAuthority {
    let executable = qualifications
        .resolve_pair(&reference_runner(), &coarse_runner())
        .unwrap();
    semantics.resolve_pair(&executable).unwrap()
}

pub(super) fn subject_binding(
    receipt: u128,
    revision: u64,
    subject: u8,
    status: ReviewedQualificationSubjectBindingStatus,
) -> ReviewedQualificationSubjectBinding {
    ReviewedQualificationSubjectBinding::new(
        ShadowRunnerQualificationReceiptKey::new(receipt, 1),
        ShadowRunnerQualificationReceiptRevision(revision),
        QualificationSubjectDigest::new(vec![subject]).unwrap(),
        EvidenceLineageToken(100 + receipt),
        status,
    )
}

pub(super) fn subject_registry(
    key: u128,
    reference: ReviewedQualificationSubjectBinding,
    coarse: ReviewedQualificationSubjectBinding,
) -> ShadowRunnerSubjectBindingRegistry {
    let mut builder = ShadowRunnerSubjectBindingRegistryBuilder::new(
        ShadowRunnerSubjectBindingRegistryKey::new(key, 1),
    );
    builder.register_binding(reference).unwrap();
    builder.register_binding(coarse).unwrap();
    builder.seal()
}

pub(super) fn current_subject_registry(key: u128) -> ShadowRunnerSubjectBindingRegistry {
    subject_registry(
        key,
        subject_binding(41, 1, 43, ReviewedQualificationSubjectBindingStatus::Current),
        subject_binding(51, 1, 53, ReviewedQualificationSubjectBindingStatus::Current),
    )
}

pub(super) fn subject_bound_pair(
    qualifications: &ShadowRunnerQualificationRegistry,
    semantics: &ShadowRunnerSemanticRegistry,
    subjects: &ShadowRunnerSubjectBindingRegistry,
) -> SubjectBoundExecutableShadowRunnerPairAuthority {
    let verifier_bound = verifier_bound_pair(qualifications, semantics);
    subjects
        .resolve_pair_current(
            &verifier_bound,
            semantics,
            qualifications,
            &reference_runner(),
            &coarse_runner(),
        )
        .unwrap()
}
