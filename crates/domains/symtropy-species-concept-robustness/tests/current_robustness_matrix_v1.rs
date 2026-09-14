mod fixture {
    include!("current_robustness_design_v1.rs");

    mod strict_outcome {
        include!("../../symtropy-evolution-core/tests/current_species_status_v1.rs");

        use super::profile_for;
        use symtropy_evolution_core::{SpeciesConceptAuthority, ValidatedSpeciesConceptAuthority};
        use symtropy_species_concept::{
            SpeciesConceptFamilyDescriptor, ValidatedSpeciesConceptFamilyDescriptor,
        };
        use symtropy_species_concept_robustness::{
            CurrentSpeciesModelDesignRecord, SpeciesModelDesignRecord,
        };

        #[derive(Clone, Copy)]
        pub(super) enum Case {
            Supported,
            Contradicted,
            Outside,
        }

        pub(super) fn with_current(
            record: &SpeciesModelDesignRecord,
            case: Case,
            f: impl FnOnce(
                CurrentSpeciesModelDesignRecord<'_>,
                &ValidatedCurrentSpeciesStatus<'_>,
            ),
        ) {
            let (history_case, isolation_case) = match case {
                Case::Supported | Case::Outside => (HistoryCase::Clean, IsolationCase::Supported),
                Case::Contradicted => (HistoryCase::Clean, IsolationCase::Contradicted),
            };
            history_fixture::with_current(history_case, |history_design, history| {
                isolation_fixture::with_current(isolation_case, |isolation_design, isolation| {
                    let source_model = model(90, 91);
                    let current_model = ValidatedBiologicalSpeciesModel::validate_current(
                        &source_model,
                        source_model.validity_domain.clone(),
                        authority("species-model-qualification", 91),
                    )
                    .unwrap();
                    let raw_design = CurrentSpeciesClassificationDesign::declare(
                        CurrentSpeciesClassificationId::new("e2a-strict-current-status").unwrap(),
                        history_design,
                        isolation_design,
                        &current_model,
                        authority("target-model-applicability", 92),
                    )
                    .unwrap();
                    let current_design =
                        ValidatedCurrentSpeciesClassificationDesign::validate_current(
                            &raw_design,
                            history_design,
                            isolation_design,
                            &current_model,
                            authority("target-model-applicability", 92),
                        )
                        .unwrap();
                    let concept = SpeciesConceptAuthority::from_strict_biological(&current_model)
                        .unwrap();
                    let current_concept =
                        ValidatedSpeciesConceptAuthority::validate_current_strict_biological(
                            &concept,
                            &current_model,
                        )
                        .unwrap();
                    let descriptor =
                        SpeciesConceptFamilyDescriptor::from_strict_biological(&current_concept)
                            .unwrap();
                    let current_descriptor =
                        ValidatedSpeciesConceptFamilyDescriptor::validate_current_strict_biological(
                            &descriptor,
                            &current_concept,
                        )
                        .unwrap();
                    let profile = profile_for(
                        current_descriptor.descriptor(),
                        current_descriptor.descriptor_digest(),
                        "strict-bsc",
                        "strict-sequence-evidence",
                        "strict-upstream-authority",
                        160,
                    );
                    let current_record =
                        CurrentSpeciesModelDesignRecord::validate_strict_biological(
                            record,
                            &current_descriptor,
                            &current_design,
                            profile,
                        )
                        .unwrap();
                    let applicability = match case {
                        Case::Supported | Case::Contradicted => {
                            SpeciesModelApplicabilityInput::InsideValidityDomain {
                                evidence: authority("inside-model-domain", 100),
                            }
                        }
                        Case::Outside => SpeciesModelApplicabilityInput::OutsideValidityDomain {
                            evidence: authority("outside-model-domain", 101),
                        },
                    };
                    let evidence = CurrentSpeciesStatusEvidence::evaluate(
                        &current_design,
                        history,
                        isolation,
                        &current_model,
                        applicability.clone(),
                    )
                    .unwrap();
                    let current_status = ValidatedCurrentSpeciesStatus::validate_current(
                        &evidence,
                        &current_design,
                        history,
                        isolation,
                        &current_model,
                        applicability,
                    )
                    .unwrap();
                    f(current_record, &current_status);
                })
            });
        }
    }

    mod general_outcome {
        include!("../../symtropy-species-concept-general-lineage/tests/general_lineage_v1.rs");

        use super::profile_for;
        use symtropy_species_concept_general_lineage::{
            general_lineage_family_descriptor, ValidatedGeneralLineageFamilyDescriptor,
        };
        use symtropy_species_concept_robustness::{
            CurrentSpeciesModelDesignRecord, SpeciesModelDesignRecord,
        };

        #[derive(Clone, Copy)]
        pub(super) enum Case {
            Supported,
            Contradicted,
            Outside,
        }

        pub(super) fn with_current(
            record: &SpeciesModelDesignRecord,
            case: Case,
            f: impl FnOnce(
                CurrentSpeciesModelDesignRecord<'_>,
                &ValidatedGeneralLineageSpeciesEvidence<'_>,
            ),
        ) {
            let run = |history_design: &ValidatedLineageDivergenceHistoryDesign<'_>,
                       history: &ValidatedLineageDivergenceHistory<'_>| {
                let raw_model = model(GeneralLineageReproductiveModePolicy::SexualOrAsexual, 171);
                let current_model = current_model(
                    &raw_model,
                    GeneralLineageReproductiveModePolicy::SexualOrAsexual,
                    171,
                );
                let descriptor = general_lineage_family_descriptor(&current_model).unwrap();
                let current_descriptor = ValidatedGeneralLineageFamilyDescriptor::validate_current(
                    &descriptor,
                    &current_model,
                )
                .unwrap();
                let channels = base_channels();
                let raw_design = GeneralLineageClassificationDesign::declare(
                    GeneralLineageClassificationId::new("e2a-general-lineage-current-status")
                        .unwrap(),
                    history_design,
                    &current_model,
                    auth("general-lineage-target-applicability", 80),
                    channels.clone(),
                    2,
                    GeneralLineageMissingEvidencePolicy::ReportInsufficientIndependentEvidence,
                )
                .unwrap();
                let current_design = current_design(
                    &raw_design,
                    history_design,
                    &current_model,
                    channels,
                    2,
                );
                let profile = profile_for(
                    current_descriptor.descriptor(),
                    current_descriptor.descriptor_digest(),
                    "general-lineage",
                    "general-multichannel-evidence",
                    "general-upstream-authority",
                    161,
                );
                let current_record = CurrentSpeciesModelDesignRecord::validate_general_lineage(
                    record,
                    &current_descriptor,
                    &current_design,
                    profile,
                )
                .unwrap();
                let applicability_input = match case {
                    Case::Supported | Case::Contradicted => applicability(
                        GeneralLineageModelApplicabilityDisposition::InDomain,
                    ),
                    Case::Outside => applicability(
                        GeneralLineageModelApplicabilityDisposition::OutsideModelValidityDomain,
                    ),
                };
                let channel_inputs = || {
                    [external(
                        "ecology",
                        GeneralLineageChannelDisposition::SupportsSeparation,
                        83,
                    )]
                };
                let evidence = GeneralLineageSpeciesEvidence::evaluate(
                    &current_design,
                    history,
                    &current_model,
                    applicability_input.clone(),
                    channel_inputs(),
                )
                .unwrap();
                let current_status = ValidatedGeneralLineageSpeciesEvidence::validate_current(
                    &evidence,
                    &current_design,
                    history,
                    &current_model,
                    applicability_input,
                    channel_inputs(),
                )
                .unwrap();
                f(current_record, &current_status);
            };
            match case {
                Case::Supported | Case::Outside => lineage_fixture::with_clean(run),
                Case::Contradicted => lineage_fixture::with_fusion(run),
            }
        }
    }

    use strict_outcome::Case as StrictCase;
    use general_outcome::Case as GeneralCase;
    use symtropy_species_concept_robustness::{
        CurrentCrossModelRobustnessAuthority, CurrentSemanticRelationRecord,
    };

    fn with_current_relation<R>(
        record: &SemanticRelationRecord,
        f: impl FnOnce(CurrentSemanticRelationRecord<'_>) -> R,
    ) -> R {
        let assessment = record.evidence.assessment.clone();
        let raw_design = &record.evidence.design;
        let current_design = ValidatedSpeciesConceptRelationDesign::validate_current(
            raw_design,
            raw_design.left.clone(),
            raw_design.right.clone(),
            raw_design.scope.clone(),
            raw_design.protocol_authority.clone(),
            raw_design.missing_policy,
        )
        .unwrap();
        let current_evidence =
            SpeciesConceptRelationEvidence::evaluate(&current_design, assessment.clone()).unwrap();
        let current_relation = ValidatedSpeciesConceptRelationEvidence::validate_current(
            &current_evidence,
            &current_design,
            assessment,
        )
        .unwrap();
        f(CurrentSemanticRelationRecord::validate_current(record, &current_relation).unwrap())
    }

    pub(super) fn with_matrix<R>(
        relation_kind: SpeciesConceptRelationKind,
        fault_disposition: PairwiseFaultDomainDisposition,
        strict_case: StrictCase,
        general_case: GeneralCase,
        f: impl FnOnce(
            &CurrentCrossModelRobustnessAuthority<'_>,
            &SpeciesModelDesignRecord,
            &symtropy_evolution_core::ValidatedCurrentSpeciesStatus<'_>,
            &SpeciesModelDesignRecord,
            &symtropy_species_concept_general_lineage::ValidatedGeneralLineageSpeciesEvidence<'_>,
        ) -> R,
    ) -> R {
        let models = independent_models();
        let p = policy(
            [
                FaultDomainDimension::EvidenceSourceLineage,
                FaultDomainDimension::UpstreamEvidenceAuthorityLineage,
            ],
            245,
        );
        let relation = relation_record(&models[0], &models[1], relation_kind, 246);
        let assessment = PairwiseFaultDomainAssessment::new(
            &p,
            &models[0].fault_profile,
            &models[1].fault_profile,
            fault_disposition,
            auth("e2b-pair-qualification", 247),
        )
        .unwrap();
        let raw = CurrentCrossModelRobustnessDesign::declare(
            CurrentRobustnessDesignId::new("e2b-current-robustness-matrix").unwrap(),
            subject(&models, 248),
            models.clone(),
            [relation.clone()],
            p.clone(),
            [assessment.clone()],
            2,
            2,
            MissingModelPolicy::ReportInsufficientCoverage,
        )
        .unwrap();

        strict_outcome::with_current(&models[0], strict_case, |strict_record, strict_status| {
            general_outcome::with_current(
                &models[1],
                general_case,
                |general_record, general_status| {
                    with_current_relation(&relation, |current_relation| {
                        let authority = CurrentCrossModelRobustnessAuthority::validate_current(
                            &raw,
                            subject(&models, 248),
                            [strict_record, general_record],
                            [current_relation],
                            p,
                            [assessment],
                            2,
                            2,
                            MissingModelPolicy::ReportInsufficientCoverage,
                        )
                        .unwrap();
                        f(
                            &authority,
                            &models[0],
                            strict_status,
                            &models[1],
                            general_status,
                        )
                    })
                },
            )
        })
    }

    pub(super) use general_outcome::Case as GeneralOutcomeCase;
    pub(super) use strict_outcome::Case as StrictOutcomeCase;
}

use fixture::{GeneralOutcomeCase, StrictOutcomeCase};
use serde_json::Value;
use symtropy_species_concept_robustness::{
    CrossModelCurrentSpeciesReport, CrossModelCurrentSpeciesRobustnessStatus,
    CrossModelRobustnessReportError, CurrentModelOutcomeRow,
    ValidatedCrossModelCurrentSpeciesReport,
};
use symtropy_species_concept_relations::SpeciesConceptRelationKind;
use symtropy_species_concept_robustness::PairwiseFaultDomainDisposition;

fn collect_keys(value: &Value, keys: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, value) in map {
                keys.push(key.clone());
                collect_keys(value, keys);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_keys(value, keys);
            }
        }
        _ => {}
    }
}

#[test]
fn supporting_nested_models_remain_visible_but_do_not_claim_independent_robustness() {
    fixture::with_matrix(
        SpeciesConceptRelationKind::OperationalCriterionWithin,
        PairwiseFaultDomainDisposition::KnownDependent,
        StrictOutcomeCase::Supported,
        GeneralOutcomeCase::Supported,
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
                CrossModelCurrentSpeciesRobustnessStatus::InsufficientIndependentModelCoverage
            );
            assert!(report.support_independence_witness.is_empty());
            assert_eq!(report.rows.len(), 2);
            assert_eq!(report.pairwise_context.len(), 1);
        },
    );
}

#[test]
fn qualified_non_nested_support_requires_and_records_exact_independence_witness() {
    fixture::with_matrix(
        SpeciesConceptRelationKind::OrthogonalEvidenceFramework,
        PairwiseFaultDomainDisposition::QualifiedSufficientlyDistinct,
        StrictOutcomeCase::Supported,
        GeneralOutcomeCase::Supported,
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
                CrossModelCurrentSpeciesRobustnessStatus::RobustSupportAcrossQualifiedIndependentCoverage
            );
            assert_eq!(report.support_independence_witness.len(), 2);
            assert!(report.contradiction_independence_witness.is_empty());
            assert!(report.support_independence_witness.windows(2).all(|w| w[0] < w[1]));
        },
    );
}

#[test]
fn support_plus_contradiction_is_mixed_not_a_majority_vote() {
    fixture::with_matrix(
        SpeciesConceptRelationKind::OrthogonalEvidenceFramework,
        PairwiseFaultDomainDisposition::QualifiedSufficientlyDistinct,
        StrictOutcomeCase::Supported,
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
                CrossModelCurrentSpeciesRobustnessStatus::MixedSupportAndContradiction
            );
            assert!(report.support_independence_witness.is_empty());
            assert!(report.contradiction_independence_witness.is_empty());
        },
    );
}

#[test]
fn outside_domain_model_is_not_a_negative_vote() {
    fixture::with_matrix(
        SpeciesConceptRelationKind::OrthogonalEvidenceFramework,
        PairwiseFaultDomainDisposition::QualifiedSufficientlyDistinct,
        StrictOutcomeCase::Supported,
        GeneralOutcomeCase::Outside,
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
                CrossModelCurrentSpeciesRobustnessStatus::InsufficientIndependentModelCoverage
            );
            assert!(report.contradiction_independence_witness.is_empty());
        },
    );
}

#[test]
fn all_models_outside_domain_is_explicit() {
    fixture::with_matrix(
        SpeciesConceptRelationKind::OrthogonalEvidenceFramework,
        PairwiseFaultDomainDisposition::QualifiedSufficientlyDistinct,
        StrictOutcomeCase::Outside,
        GeneralOutcomeCase::Outside,
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
                CrossModelCurrentSpeciesRobustnessStatus::AllModelsOutsideValidityDomain
            );
        },
    );
}

#[test]
fn restored_report_requires_fresh_model_status_and_e2a_replay() {
    fixture::with_matrix(
        SpeciesConceptRelationKind::OrthogonalEvidenceFramework,
        PairwiseFaultDomainDisposition::QualifiedSufficientlyDistinct,
        StrictOutcomeCase::Supported,
        GeneralOutcomeCase::Supported,
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
            let restored: CrossModelCurrentSpeciesReport =
                serde_json::from_slice(&serde_json::to_vec(&report).unwrap()).unwrap();
            let current = ValidatedCrossModelCurrentSpeciesReport::validate_current(
                &restored,
                authority,
                [
                    CurrentModelOutcomeRow::strict_biological(strict_model, strict_status)
                        .unwrap(),
                    CurrentModelOutcomeRow::general_lineage(general_model, general_status).unwrap(),
                ],
            )
            .unwrap();
            assert_eq!(current.report_digest(), report.canonical_digest().unwrap());
        },
    );
}

#[test]
fn dropping_a_declared_model_fails_closed() {
    fixture::with_matrix(
        SpeciesConceptRelationKind::OrthogonalEvidenceFramework,
        PairwiseFaultDomainDisposition::QualifiedSufficientlyDistinct,
        StrictOutcomeCase::Supported,
        GeneralOutcomeCase::Supported,
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
fn forged_aggregate_status_fails_local_recomputation() {
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
            report.status = CrossModelCurrentSpeciesRobustnessStatus::RobustSupportAcrossQualifiedIndependentCoverage;
            assert!(matches!(
                report.canonical_digest(),
                Err(CrossModelRobustnessReportError::StatusInvariant)
            ));
        },
    );
}

#[test]
fn wire_shape_contains_no_majority_or_scalar_taxonomy_score() {
    fixture::with_matrix(
        SpeciesConceptRelationKind::OperationalCriterionWithin,
        PairwiseFaultDomainDisposition::KnownDependent,
        StrictOutcomeCase::Supported,
        GeneralOutcomeCase::Supported,
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
            let value = serde_json::to_value(report).unwrap();
            let mut keys = Vec::new();
            collect_keys(&value, &mut keys);
            for forbidden in [
                "majority",
                "majority_count",
                "vote",
                "votes",
                "taxonomy_score",
                "species_score",
                "winner",
                "universal_species_truth",
            ] {
                assert!(!keys.iter().any(|key| key == forbidden));
            }
        },
    );
}
