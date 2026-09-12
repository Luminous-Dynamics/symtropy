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
    ShadowRunnerSubjectBindingError, ShadowRunnerSubjectBindingRegistry,
    ShadowRunnerSubjectBindingRegistryBuilder, ShadowRunnerSubjectBindingRegistryKey,
};
use crate::living_world_authority::shadow_validation::{
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

fn qualification_registry(key: u128) -> ShadowRunnerQualificationRegistry {
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

fn semantic_registry(key: u128) -> ShadowRunnerSemanticRegistry {
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

fn verifier_bound_pair(
    qualifications: &ShadowRunnerQualificationRegistry,
    semantics: &ShadowRunnerSemanticRegistry,
) -> VerifierBoundExecutableShadowRunnerPairAuthority {
    let executable = qualifications
        .resolve_pair(&reference_runner(), &coarse_runner())
        .unwrap();
    semantics.resolve_pair(&executable).unwrap()
}

fn subject_binding(
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

fn subject_registry(
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

fn current_subject_registry(key: u128) -> ShadowRunnerSubjectBindingRegistry {
    subject_registry(
        key,
        subject_binding(41, 1, 43, ReviewedQualificationSubjectBindingStatus::Current),
        subject_binding(51, 1, 53, ReviewedQualificationSubjectBindingStatus::Current),
    )
}

#[test]
fn exact_subject_pair_resolves_and_revalidates() {
    let qualifications = qualification_registry(40);
    let semantics = semantic_registry(70);
    let executable = verifier_bound_pair(&qualifications, &semantics);
    let subjects = current_subject_registry(80);

    let bound = subjects
        .resolve_pair_current(
            &executable,
            &semantics,
            &qualifications,
            &reference_runner(),
            &coarse_runner(),
        )
        .unwrap();

    assert_eq!(
        bound.validate_current(
            &subjects,
            &semantics,
            &qualifications,
            &reference_runner(),
            &coarse_runner(),
        ),
        Ok(())
    );
}

#[test]
fn wrong_reviewed_subject_rejects() {
    let qualifications = qualification_registry(40);
    let semantics = semantic_registry(70);
    let executable = verifier_bound_pair(&qualifications, &semantics);
    let subjects = subject_registry(
        80,
        subject_binding(41, 1, 99, ReviewedQualificationSubjectBindingStatus::Current),
        subject_binding(51, 1, 53, ReviewedQualificationSubjectBindingStatus::Current),
    );

    assert!(matches!(
        subjects.resolve_pair_current(
            &executable,
            &semantics,
            &qualifications,
            &reference_runner(),
            &coarse_runner(),
        ),
        Err(ShadowRunnerSubjectBindingError::SubjectMismatch { .. })
    ));
}

#[test]
fn old_receipt_revision_rejects() {
    let qualifications = qualification_registry(40);
    let semantics = semantic_registry(70);
    let executable = verifier_bound_pair(&qualifications, &semantics);
    let subjects = subject_registry(
        80,
        subject_binding(41, 0, 43, ReviewedQualificationSubjectBindingStatus::Current),
        subject_binding(51, 1, 53, ReviewedQualificationSubjectBindingStatus::Current),
    );

    assert!(matches!(
        subjects.resolve_pair_current(
            &executable,
            &semantics,
            &qualifications,
            &reference_runner(),
            &coarse_runner(),
        ),
        Err(ShadowRunnerSubjectBindingError::ReceiptRevisionMismatch { .. })
    ));
}

#[test]
fn revoked_subject_binding_rejects() {
    let qualifications = qualification_registry(40);
    let semantics = semantic_registry(70);
    let executable = verifier_bound_pair(&qualifications, &semantics);
    let subjects = subject_registry(
        80,
        subject_binding(41, 1, 43, ReviewedQualificationSubjectBindingStatus::Revoked),
        subject_binding(51, 1, 53, ReviewedQualificationSubjectBindingStatus::Current),
    );

    assert!(matches!(
        subjects.resolve_pair_current(
            &executable,
            &semantics,
            &qualifications,
            &reference_runner(),
            &coarse_runner(),
        ),
        Err(ShadowRunnerSubjectBindingError::BindingNotCurrent { .. })
    ));
}

#[test]
fn subject_authority_replacement_invalidates_prior_pair() {
    let qualifications = qualification_registry(40);
    let semantics = semantic_registry(70);
    let executable = verifier_bound_pair(&qualifications, &semantics);
    let subjects = current_subject_registry(80);
    let bound = subjects
        .resolve_pair_current(
            &executable,
            &semantics,
            &qualifications,
            &reference_runner(),
            &coarse_runner(),
        )
        .unwrap();

    let replacement = current_subject_registry(81);
    assert_eq!(
        bound.validate_current(
            &replacement,
            &semantics,
            &qualifications,
            &reference_runner(),
            &coarse_runner(),
        ),
        Err(ShadowRunnerSubjectBindingError::SubjectAuthorityChanged)
    );
}

#[test]
fn qualification_authority_replacement_rejects_before_subject_resolution() {
    let qualifications = qualification_registry(40);
    let semantics = semantic_registry(70);
    let executable = verifier_bound_pair(&qualifications, &semantics);
    let subjects = current_subject_registry(80);

    let replacement = qualification_registry(41);
    assert!(matches!(
        subjects.resolve_pair_current(
            &executable,
            &semantics,
            &replacement,
            &reference_runner(),
            &coarse_runner(),
        ),
        Err(ShadowRunnerSubjectBindingError::RunnerSemantics(_))
    ));
}

#[test]
fn semantic_authority_replacement_rejects_before_subject_resolution() {
    let qualifications = qualification_registry(40);
    let semantics = semantic_registry(70);
    let executable = verifier_bound_pair(&qualifications, &semantics);
    let subjects = current_subject_registry(80);

    let replacement = semantic_registry(71);
    assert!(matches!(
        subjects.resolve_pair_current(
            &executable,
            &replacement,
            &qualifications,
            &reference_runner(),
            &coarse_runner(),
        ),
        Err(ShadowRunnerSubjectBindingError::RunnerSemantics(_))
    ));
}
