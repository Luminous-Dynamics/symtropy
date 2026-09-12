use crate::living_world_authority::shadow_runner_authority_test_support::{
    coarse_runner, current_subject_registry, qualification_registry, reference_runner,
    semantic_registry, subject_binding, subject_registry, verifier_bound_pair,
};
use crate::living_world_authority::shadow_runner_subject_binding::{
    ReviewedQualificationSubjectBindingStatus, ShadowRunnerSubjectBindingError,
};

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
