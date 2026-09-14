include!("current_robustness_matrix_v1.rs");

use symtropy_evolution_core::{AnalysisAuthorityRef, AnalysisContentDigest, AnalysisMethodId};
use symtropy_species_concept_robustness::CrossModelOutcomeDisposition;

fn hardening_auth(label: &str, byte: u8) -> AnalysisAuthorityRef {
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new(label).unwrap(),
        1,
        AnalysisContentDigest::new([byte; 32]),
    )
}

#[test]
fn qualified_independent_contradiction_requires_exact_witness() {
    fixture::with_matrix(
        SpeciesConceptRelationKind::OrthogonalEvidenceFramework,
        PairwiseFaultDomainDisposition::QualifiedSufficientlyDistinct,
        StrictOutcomeCase::Contradicted,
        GeneralOutcomeCase::Contradicted,
        |authority, strict_model, strict_status, general_model, general_status| {
            let report = CrossModelCurrentSpeciesReport::evaluate(
                authority,
                [
                    CurrentModelOutcomeRow::strict_biological(strict_model, strict_status)
                        .unwrap(),
                    CurrentModelOutcomeRow::general_lineage(general_model, general_status).unwrap(),
                ],
            )
            .unwrap();
            assert_eq!(
                report.status,
                CrossModelCurrentSpeciesRobustnessStatus::RobustContradictionAcrossQualifiedIndependentCoverage
            );
            assert_eq!(report.contradiction_independence_witness.len(), 2);
            assert!(report.support_independence_witness.is_empty());
            assert!(report.non_support_independence_witness.is_empty());
        },
    );
}

#[test]
fn explicit_missing_capability_is_not_disagreement_or_negative_vote() {
    fixture::with_matrix(
        SpeciesConceptRelationKind::OrthogonalEvidenceFramework,
        PairwiseFaultDomainDisposition::QualifiedSufficientlyDistinct,
        StrictOutcomeCase::Supported,
        GeneralOutcomeCase::Supported,
        |authority, strict_model, strict_status, general_model, _| {
            let report = CrossModelCurrentSpeciesReport::evaluate(
                authority,
                [
                    CurrentModelOutcomeRow::strict_biological(strict_model, strict_status)
                        .unwrap(),
                    CurrentModelOutcomeRow::missing_current_capability(
                        general_model,
                        hardening_auth("general-current-capability-unavailable", 210),
                    )
                    .unwrap(),
                ],
            )
            .unwrap();
            assert_eq!(
                report.status,
                CrossModelCurrentSpeciesRobustnessStatus::InsufficientIndependentModelCoverage
            );
            assert_eq!(
                report.rows[1].disposition(),
                CrossModelOutcomeDisposition::MissingCurrentCapability
            );
        },
    );
}

#[test]
fn input_order_does_not_change_canonical_report() {
    fixture::with_matrix(
        SpeciesConceptRelationKind::OrthogonalEvidenceFramework,
        PairwiseFaultDomainDisposition::QualifiedSufficientlyDistinct,
        StrictOutcomeCase::Supported,
        GeneralOutcomeCase::Supported,
        |authority, strict_model, strict_status, general_model, general_status| {
            let forward = CrossModelCurrentSpeciesReport::evaluate(
                authority,
                [
                    CurrentModelOutcomeRow::strict_biological(strict_model, strict_status)
                        .unwrap(),
                    CurrentModelOutcomeRow::general_lineage(general_model, general_status).unwrap(),
                ],
            )
            .unwrap();
            let reversed = CrossModelCurrentSpeciesReport::evaluate(
                authority,
                [
                    CurrentModelOutcomeRow::general_lineage(general_model, general_status).unwrap(),
                    CurrentModelOutcomeRow::strict_biological(strict_model, strict_status)
                        .unwrap(),
                ],
            )
            .unwrap();
            assert_eq!(forward, reversed);
            assert_eq!(
                forward.canonical_digest().unwrap(),
                reversed.canonical_digest().unwrap()
            );
        },
    );
}

#[test]
fn dropping_a_contradictory_declared_model_fails_before_aggregation() {
    fixture::with_matrix(
        SpeciesConceptRelationKind::OrthogonalEvidenceFramework,
        PairwiseFaultDomainDisposition::QualifiedSufficientlyDistinct,
        StrictOutcomeCase::Supported,
        GeneralOutcomeCase::Contradicted,
        |authority, strict_model, strict_status, _, _| {
            let result = CrossModelCurrentSpeciesReport::evaluate(
                authority,
                [CurrentModelOutcomeRow::strict_biological(strict_model, strict_status).unwrap()],
            );
            assert!(matches!(
                result,
                Err(CrossModelRobustnessReportError::IncompleteModelCoverage)
            ));
        },
    );
}

#[test]
fn forged_independence_witness_cannot_self_authorize() {
    fixture::with_matrix(
        SpeciesConceptRelationKind::OperationalCriterionWithin,
        PairwiseFaultDomainDisposition::KnownDependent,
        StrictOutcomeCase::Supported,
        GeneralOutcomeCase::Supported,
        |authority, strict_model, strict_status, general_model, general_status| {
            let mut report = CrossModelCurrentSpeciesReport::evaluate(
                authority,
                [
                    CurrentModelOutcomeRow::strict_biological(strict_model, strict_status)
                        .unwrap(),
                    CurrentModelOutcomeRow::general_lineage(general_model, general_status).unwrap(),
                ],
            )
            .unwrap();
            report.support_independence_witness = report
                .rows
                .iter()
                .map(|row| row.conceptual_identity.clone())
                .collect();
            assert!(matches!(
                report.canonical_digest(),
                Err(CrossModelRobustnessReportError::SupportWitnessMismatch)
            ));
        },
    );
}

#[test]
fn pairwise_context_tamper_cannot_self_authorize() {
    fixture::with_matrix(
        SpeciesConceptRelationKind::OrthogonalEvidenceFramework,
        PairwiseFaultDomainDisposition::QualifiedSufficientlyDistinct,
        StrictOutcomeCase::Supported,
        GeneralOutcomeCase::Supported,
        |authority, strict_model, strict_status, general_model, general_status| {
            let mut report = CrossModelCurrentSpeciesReport::evaluate(
                authority,
                [
                    CurrentModelOutcomeRow::strict_biological(strict_model, strict_status)
                        .unwrap(),
                    CurrentModelOutcomeRow::general_lineage(general_model, general_status).unwrap(),
                ],
            )
            .unwrap();
            report.pairwise_context[0].fault_disposition =
                PairwiseFaultDomainDisposition::KnownDependent;
            assert!(matches!(
                report.canonical_digest(),
                Err(CrossModelRobustnessReportError::PairwiseContextMismatch)
            ));
        },
    );
}
