use crate::information::EvidenceLineageToken;
use crate::living_world_authority::shadow_runner_authority_test_support::{
    coarse_runner, current_subject_registry, qualification_registry, reference_runner,
    semantic_registry, subject_bound_pair,
};
use crate::living_world_authority::shadow_runner_qualification::{
    QualificationSubjectDigest, ShadowRunnerQualificationReceiptKey,
    ShadowRunnerQualificationReceiptRevision,
};
use crate::living_world_authority::shadow_runner_subject_descriptor::{
    DescriptorBoundExecutableShadowRunnerPairAuthority, QualificationDigestGrammar,
    QualificationSubjectDescriptor, QualificationSubjectKind,
    ReviewedQualificationSubjectDescriptorBinding, ReviewedQualificationSubjectDescriptorStatus,
    ShadowRunnerSubjectDescriptorError, ShadowSubjectDescriptorReviewRegistry,
    ShadowSubjectDescriptorReviewRegistryBuilder, ShadowSubjectDescriptorReviewRegistryKey,
    ShadowSubjectDescriptorVerificationRegistry, ShadowSubjectDescriptorVerificationRegistryBuilder,
    ShadowSubjectDescriptorVerificationRegistryKey, VerifiedQualificationSubjectDescriptorClaim,
    VerifiedQualificationSubjectDescriptorStatus,
};

fn descriptor(
    subject: u8,
    kind: QualificationSubjectKind,
    grammar: QualificationDigestGrammar,
    schema_version: u32,
) -> QualificationSubjectDescriptor {
    QualificationSubjectDescriptor::new(
        kind,
        grammar,
        schema_version,
        QualificationSubjectDigest::new(vec![subject]).unwrap(),
    )
}

fn verified_claim(
    receipt: u128,
    subject: u8,
    kind: QualificationSubjectKind,
    grammar: QualificationDigestGrammar,
    schema_version: u32,
    status: VerifiedQualificationSubjectDescriptorStatus,
) -> VerifiedQualificationSubjectDescriptorClaim {
    VerifiedQualificationSubjectDescriptorClaim::new(
        ShadowRunnerQualificationReceiptKey::new(receipt, 1),
        ShadowRunnerQualificationReceiptRevision(1),
        descriptor(subject, kind, grammar, schema_version),
        EvidenceLineageToken(200 + receipt),
        status,
    )
}

fn reviewed_binding(
    receipt: u128,
    subject: u8,
    kind: QualificationSubjectKind,
    grammar: QualificationDigestGrammar,
    schema_version: u32,
    status: ReviewedQualificationSubjectDescriptorStatus,
) -> ReviewedQualificationSubjectDescriptorBinding {
    ReviewedQualificationSubjectDescriptorBinding::new(
        ShadowRunnerQualificationReceiptKey::new(receipt, 1),
        ShadowRunnerQualificationReceiptRevision(1),
        descriptor(subject, kind, grammar, schema_version),
        EvidenceLineageToken(300 + receipt),
        status,
    )
}

fn verification_registry(
    key: u128,
    reference: VerifiedQualificationSubjectDescriptorClaim,
    coarse: VerifiedQualificationSubjectDescriptorClaim,
) -> ShadowSubjectDescriptorVerificationRegistry {
    let mut builder = ShadowSubjectDescriptorVerificationRegistryBuilder::new(
        ShadowSubjectDescriptorVerificationRegistryKey::new(key, 1),
    );
    builder.register_claim(reference).unwrap();
    builder.register_claim(coarse).unwrap();
    builder.seal()
}

fn current_verification_registry(key: u128) -> ShadowSubjectDescriptorVerificationRegistry {
    verification_registry(
        key,
        verified_claim(
            41,
            43,
            QualificationSubjectKind::ProductCommit,
            QualificationDigestGrammar::Sha256V1,
            1,
            VerifiedQualificationSubjectDescriptorStatus::Current,
        ),
        verified_claim(
            51,
            53,
            QualificationSubjectKind::ProductCommit,
            QualificationDigestGrammar::Sha256V1,
            1,
            VerifiedQualificationSubjectDescriptorStatus::Current,
        ),
    )
}

fn review_registry(
    key: u128,
    reference: ReviewedQualificationSubjectDescriptorBinding,
    coarse: ReviewedQualificationSubjectDescriptorBinding,
) -> ShadowSubjectDescriptorReviewRegistry {
    let mut builder = ShadowSubjectDescriptorReviewRegistryBuilder::new(
        ShadowSubjectDescriptorReviewRegistryKey::new(key, 1),
    );
    builder.register_binding(reference).unwrap();
    builder.register_binding(coarse).unwrap();
    builder.seal()
}

fn current_review_registry(key: u128) -> ShadowSubjectDescriptorReviewRegistry {
    review_registry(
        key,
        reviewed_binding(
            41,
            43,
            QualificationSubjectKind::ProductCommit,
            QualificationDigestGrammar::Sha256V1,
            1,
            ReviewedQualificationSubjectDescriptorStatus::Current,
        ),
        reviewed_binding(
            51,
            53,
            QualificationSubjectKind::ProductCommit,
            QualificationDigestGrammar::Sha256V1,
            1,
            ReviewedQualificationSubjectDescriptorStatus::Current,
        ),
    )
}

fn resolve_current(
    verification: &ShadowSubjectDescriptorVerificationRegistry,
    review: &ShadowSubjectDescriptorReviewRegistry,
) -> Result<DescriptorBoundExecutableShadowRunnerPairAuthority, ShadowRunnerSubjectDescriptorError> {
    let qualifications = qualification_registry(40);
    let semantics = semantic_registry(70);
    let subjects = current_subject_registry(80);
    let lower = subject_bound_pair(&qualifications, &semantics, &subjects);
    review.resolve_pair_current(
        verification,
        &lower,
        &subjects,
        &semantics,
        &qualifications,
        &reference_runner(),
        &coarse_runner(),
    )
}

#[test]
fn exact_descriptor_pair_resolves_and_revalidates() {
    let qualifications = qualification_registry(40);
    let semantics = semantic_registry(70);
    let subjects = current_subject_registry(80);
    let lower = subject_bound_pair(&qualifications, &semantics, &subjects);
    let verification = current_verification_registry(90);
    let review = current_review_registry(100);

    let bound = review
        .resolve_pair_current(
            &verification,
            &lower,
            &subjects,
            &semantics,
            &qualifications,
            &reference_runner(),
            &coarse_runner(),
        )
        .unwrap();

    assert_eq!(
        bound.validate_current(
            &verification,
            &review,
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
fn same_bytes_different_subject_kind_rejects() {
    let verification = current_verification_registry(90);
    let review = review_registry(
        100,
        reviewed_binding(
            41,
            43,
            QualificationSubjectKind::ProductTree,
            QualificationDigestGrammar::Sha256V1,
            1,
            ReviewedQualificationSubjectDescriptorStatus::Current,
        ),
        reviewed_binding(
            51,
            53,
            QualificationSubjectKind::ProductCommit,
            QualificationDigestGrammar::Sha256V1,
            1,
            ReviewedQualificationSubjectDescriptorStatus::Current,
        ),
    );
    assert!(matches!(
        resolve_current(&verification, &review),
        Err(ShadowRunnerSubjectDescriptorError::DescriptorMismatch { .. })
    ));
}

#[test]
fn same_bytes_different_digest_grammar_rejects() {
    let verification = current_verification_registry(90);
    let review = review_registry(
        100,
        reviewed_binding(
            41,
            43,
            QualificationSubjectKind::ProductCommit,
            QualificationDigestGrammar::Blake3V1,
            1,
            ReviewedQualificationSubjectDescriptorStatus::Current,
        ),
        reviewed_binding(
            51,
            53,
            QualificationSubjectKind::ProductCommit,
            QualificationDigestGrammar::Sha256V1,
            1,
            ReviewedQualificationSubjectDescriptorStatus::Current,
        ),
    );
    assert!(matches!(
        resolve_current(&verification, &review),
        Err(ShadowRunnerSubjectDescriptorError::DescriptorMismatch { .. })
    ));
}

#[test]
fn same_bytes_different_schema_version_rejects() {
    let verification = current_verification_registry(90);
    let review = review_registry(
        100,
        reviewed_binding(
            41,
            43,
            QualificationSubjectKind::ProductCommit,
            QualificationDigestGrammar::Sha256V1,
            2,
            ReviewedQualificationSubjectDescriptorStatus::Current,
        ),
        reviewed_binding(
            51,
            53,
            QualificationSubjectKind::ProductCommit,
            QualificationDigestGrammar::Sha256V1,
            1,
            ReviewedQualificationSubjectDescriptorStatus::Current,
        ),
    );
    assert!(matches!(
        resolve_current(&verification, &review),
        Err(ShadowRunnerSubjectDescriptorError::DescriptorMismatch { .. })
    ));
}

#[test]
fn verified_descriptor_must_carry_existing_provenance_digest() {
    let verification = verification_registry(
        90,
        verified_claim(
            41,
            99,
            QualificationSubjectKind::ProductCommit,
            QualificationDigestGrammar::Sha256V1,
            1,
            VerifiedQualificationSubjectDescriptorStatus::Current,
        ),
        verified_claim(
            51,
            53,
            QualificationSubjectKind::ProductCommit,
            QualificationDigestGrammar::Sha256V1,
            1,
            VerifiedQualificationSubjectDescriptorStatus::Current,
        ),
    );
    let review = current_review_registry(100);
    assert!(matches!(
        resolve_current(&verification, &review),
        Err(ShadowRunnerSubjectDescriptorError::VerifiedDigestMismatch { .. })
    ));
}

#[test]
fn verification_authority_replacement_invalidates_prior_pair() {
    let qualifications = qualification_registry(40);
    let semantics = semantic_registry(70);
    let subjects = current_subject_registry(80);
    let lower = subject_bound_pair(&qualifications, &semantics, &subjects);
    let verification = current_verification_registry(90);
    let review = current_review_registry(100);
    let bound = review
        .resolve_pair_current(
            &verification,
            &lower,
            &subjects,
            &semantics,
            &qualifications,
            &reference_runner(),
            &coarse_runner(),
        )
        .unwrap();

    let replacement = current_verification_registry(91);
    assert_eq!(
        bound.validate_current(
            &replacement,
            &review,
            &subjects,
            &semantics,
            &qualifications,
            &reference_runner(),
            &coarse_runner(),
        ),
        Err(ShadowRunnerSubjectDescriptorError::VerificationAuthorityChanged)
    );
}

#[test]
fn review_authority_replacement_invalidates_prior_pair() {
    let qualifications = qualification_registry(40);
    let semantics = semantic_registry(70);
    let subjects = current_subject_registry(80);
    let lower = subject_bound_pair(&qualifications, &semantics, &subjects);
    let verification = current_verification_registry(90);
    let review = current_review_registry(100);
    let bound = review
        .resolve_pair_current(
            &verification,
            &lower,
            &subjects,
            &semantics,
            &qualifications,
            &reference_runner(),
            &coarse_runner(),
        )
        .unwrap();

    let replacement = current_review_registry(101);
    assert_eq!(
        bound.validate_current(
            &verification,
            &replacement,
            &subjects,
            &semantics,
            &qualifications,
            &reference_runner(),
            &coarse_runner(),
        ),
        Err(ShadowRunnerSubjectDescriptorError::ReviewAuthorityChanged)
    );
}
