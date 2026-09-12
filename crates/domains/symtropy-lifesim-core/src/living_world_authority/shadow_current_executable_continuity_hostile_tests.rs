use super::q2_authority_test_support::Q2AuthorityFixture;
use super::shadow_current_executable_continuity::CurrentExecutableShadowContinuityError;
use super::shadow_execution_continuity::{
    PairedShadowContinuityCertificate, ShadowExecutionContinuityError,
};
use super::shadow_execution_lineage::ShadowExecutionLineageError;
use super::shadow_paired_execution::PairedShadowExecutionError;
use super::shadow_runner_qualification::ShadowRunnerQualificationError;
use super::shadow_runner_semantic_provenance::ShadowRunnerSemanticProvenanceError;

#[test]
fn exact_current_q2_world_certifies() {
    let fixture = Q2AuthorityFixture::new();
    assert!(fixture.certify_current().is_ok());
}

#[test]
fn paired_execution_authority_replacement_rejects() {
    let fixture = Q2AuthorityFixture::new();
    assert!(matches!(
        fixture.certify_with_replacement_paired_authority(),
        Err(CurrentExecutableShadowContinuityError::PairedExecution(
            PairedShadowExecutionError::PairedAuthorityChanged,
        ))
    ));
}

#[test]
fn reference_execution_authority_replacement_rejects() {
    let fixture = Q2AuthorityFixture::new();
    assert!(matches!(
        fixture.certify_with_replacement_reference_authority(),
        Err(CurrentExecutableShadowContinuityError::PairedExecution(
            PairedShadowExecutionError::ReferenceExecutionAuthorityChanged,
        ))
    ));
}

#[test]
fn retained_authority_replacement_rejects_through_reference_currentness() {
    let fixture = Q2AuthorityFixture::new();
    assert!(matches!(
        fixture.certify_with_replacement_retained_authority(),
        Err(CurrentExecutableShadowContinuityError::PairedExecution(
            PairedShadowExecutionError::ReferenceExecution(
                ShadowExecutionLineageError::RetainedAuthorityChanged,
            ),
        ))
    ));
}

#[test]
fn q2_shadow_authority_replacement_rejects_before_continuity_reuse() {
    let fixture = Q2AuthorityFixture::new();
    assert!(matches!(
        fixture.certify_with_replacement_shadow_authority(),
        Err(CurrentExecutableShadowContinuityError::PairedExecution(
            PairedShadowExecutionError::ShadowAuthorityChanged,
        ))
    ));
}

#[test]
fn executable_qualification_authority_replacement_rejects() {
    let fixture = Q2AuthorityFixture::new();
    assert!(matches!(
        fixture.certify_with_replacement_qualification_authority(),
        Err(CurrentExecutableShadowContinuityError::RunnerSemantics(
            ShadowRunnerSemanticProvenanceError::Qualification(
                ShadowRunnerQualificationError::QualificationAuthorityChanged,
            ),
        ))
    ));
}

#[test]
fn semantic_authority_replacement_rejects() {
    let fixture = Q2AuthorityFixture::new();
    assert!(matches!(
        fixture.certify_with_replacement_semantic_authority(),
        Err(CurrentExecutableShadowContinuityError::RunnerSemantics(
            ShadowRunnerSemanticProvenanceError::SemanticAuthorityChanged,
        ))
    ));
}

#[test]
fn cross_run_continuity_rejects_at_structural_boundary() {
    let fixture = Q2AuthorityFixture::new();
    let wrong_coarse = fixture.wrong_coarse_run_transcript();
    assert!(matches!(
        PairedShadowContinuityCertificate::certify(
            fixture.paired(),
            &wrong_coarse,
            fixture.continuity().reference(),
        ),
        Err(ShadowExecutionContinuityError::RunIdentityMismatch)
    ));
}

#[test]
fn cross_evidence_continuity_rejects_at_structural_boundary() {
    let fixture = Q2AuthorityFixture::new();
    let wrong_coarse = fixture.wrong_coarse_evidence_transcript();
    assert!(matches!(
        PairedShadowContinuityCertificate::certify(
            fixture.paired(),
            &wrong_coarse,
            fixture.continuity().reference(),
        ),
        Err(ShadowExecutionContinuityError::EvidenceIdentityMismatch)
    ));
}
