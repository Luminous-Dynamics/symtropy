include!("current_robustness_design_v1.rs");

mod strict_current_fixture {
    include!("../../symtropy-evolution-core/tests/current_species_status_v1.rs");

    use super::profile_for;
    use symtropy_evolution_core::{SpeciesConceptAuthority, ValidatedSpeciesConceptAuthority};
    use symtropy_species_concept::{
        SpeciesConceptFamilyDescriptor, ValidatedSpeciesConceptFamilyDescriptor,
    };
    use symtropy_species_concept_robustness::{
        CurrentRobustnessAuthorityError, CurrentSpeciesModelDesignRecord, SpeciesModelDesignRecord,
    };

    pub(super) fn with_current<R>(
        record: &SpeciesModelDesignRecord,
        evidence_source: &str,
        upstream_evidence: &str,
        profile_byte: u8,
        f: impl FnOnce(Result<CurrentSpeciesModelDesignRecord<'_>, CurrentRobustnessAuthorityError>) -> R,
    ) -> R {
        history_fixture::with_current(HistoryCase::Clean, |history_design, _| {
            isolation_fixture::with_current(IsolationCase::Supported, |isolation_design, _| {
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
                let current_design = ValidatedCurrentSpeciesClassificationDesign::validate_current(
                    &raw_design,
                    history_design,
                    isolation_design,
                    &current_model,
                    authority("target-model-applicability", 92),
                )
                .unwrap();
                let concept = SpeciesConceptAuthority::from_strict_biological(&current_model).unwrap();
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
                    evidence_source,
                    upstream_evidence,
                    profile_byte,
                );
                f(CurrentSpeciesModelDesignRecord::validate_strict_biological(
                    record,
                    &current_descriptor,
                    &current_design,
                    profile,
                ))
            })
        })
    }
}

mod general_current_fixture {
    include!("../../symtropy-species-concept-general-lineage/tests/general_lineage_v1.rs");

    use super::profile_for;
    use symtropy_species_concept_general_lineage::{
        general_lineage_family_descriptor, ValidatedGeneralLineageFamilyDescriptor,
    };
    use symtropy_species_concept_robustness::{
        CurrentRobustnessAuthorityError, CurrentSpeciesModelDesignRecord, SpeciesModelDesignRecord,
    };

    pub(super) fn with_current<R>(
        record: &SpeciesModelDesignRecord,
        evidence_source: &str,
        upstream_evidence: &str,
        profile_byte: u8,
        f: impl FnOnce(Result<CurrentSpeciesModelDesignRecord<'_>, CurrentRobustnessAuthorityError>) -> R,
    ) -> R {
        lineage_fixture::with_clean(|history_design, _| {
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
                GeneralLineageClassificationId::new("e2a-general-lineage-current-status").unwrap(),
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
                evidence_source,
                upstream_evidence,
                profile_byte,
            );
            f(CurrentSpeciesModelDesignRecord::validate_general_lineage(
                record,
                &current_descriptor,
                &current_design,
                profile,
            ))
        })
    }
}

fn with_current_relation<R>(
    record: &SemanticRelationRecord,
    assessment: SpeciesConceptRelationAssessment,
    f: impl FnOnce(Result<CurrentSemanticRelationRecord<'_>, CurrentRobustnessAuthorityError>) -> R,
) -> R {
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
    let current_evidence = SpeciesConceptRelationEvidence::evaluate(&current_design, assessment.clone())
        .unwrap();
    let current_relation = ValidatedSpeciesConceptRelationEvidence::validate_current(
        &current_evidence,
        &current_design,
        assessment,
    )
    .unwrap();
    f(CurrentSemanticRelationRecord::validate_current(
        record,
        &current_relation,
    ))
}

#[test]
fn persisted_model_row_requires_fresh_family_replay() {
    let models = independent_models();
    strict_current_fixture::with_current(
        &models[0],
        "strict-sequence-evidence",
        "strict-upstream-authority",
        160,
        |current| assert!(current.is_ok()),
    );
    general_current_fixture::with_current(
        &models[1],
        "general-multichannel-evidence",
        "general-upstream-authority",
        161,
        |current| assert!(current.is_ok()),
    );
}

#[test]
fn profile_qualification_drift_cannot_regain_model_row_authority() {
    let models = independent_models();
    strict_current_fixture::with_current(
        &models[0],
        "strict-sequence-evidence",
        "strict-upstream-authority",
        170,
        |current| {
            assert!(matches!(
                current,
                Err(CurrentRobustnessAuthorityError::ModelRecordReplayMismatch)
            ));
        },
    );
}

#[test]
fn persisted_relation_row_requires_fresh_relation_evidence() {
    let models = independent_models();
    let record = relation_record(
        &models[0],
        &models[1],
        SpeciesConceptRelationKind::OperationalCriterionWithin,
        180,
    );
    with_current_relation(&record, record.evidence.assessment.clone(), |current| {
        assert!(current.is_ok());
    });
}

#[test]
fn relation_qualification_drift_cannot_regain_relation_row_authority() {
    let models = independent_models();
    let record = relation_record(
        &models[0],
        &models[1],
        SpeciesConceptRelationKind::OperationalCriterionWithin,
        180,
    );
    let mut assessment = record.evidence.assessment.clone();
    if let SpeciesConceptRelationAssessment::Qualified {
        qualification_authority,
        ..
    } = &mut assessment
    {
        *qualification_authority = auth("drifted-relation-qualification", 244);
    } else {
        panic!("fixture relation must be qualified");
    }
    with_current_relation(&record, assessment, |current| {
        assert!(matches!(
            current,
            Err(CurrentRobustnessAuthorityError::RelationRecordReplayMismatch)
        ));
    });
}

#[test]
fn top_level_current_authority_requires_fresh_model_and_relation_rows() {
    let models = independent_models();
    let p = policy(
        [
            FaultDomainDimension::EvidenceSourceLineage,
            FaultDomainDimension::UpstreamEvidenceAuthorityLineage,
        ],
        245,
    );
    let relation = relation_record(
        &models[0],
        &models[1],
        SpeciesConceptRelationKind::OperationalCriterionWithin,
        246,
    );
    let assessment = PairwiseFaultDomainAssessment::new(
        &p,
        &models[0].fault_profile,
        &models[1].fault_profile,
        PairwiseFaultDomainDisposition::KnownDependent,
        auth("current-pair-qualification", 247),
    )
    .unwrap();
    let raw = CurrentCrossModelRobustnessDesign::declare(
        CurrentRobustnessDesignId::new("fresh-current-authority").unwrap(),
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

    strict_current_fixture::with_current(
        &models[0],
        "strict-sequence-evidence",
        "strict-upstream-authority",
        160,
        |strict_current| {
            let strict_current = strict_current.unwrap();
            general_current_fixture::with_current(
                &models[1],
                "general-multichannel-evidence",
                "general-upstream-authority",
                161,
                |general_current| {
                    let general_current = general_current.unwrap();
                    with_current_relation(
                        &relation,
                        relation.evidence.assessment.clone(),
                        |current_relation| {
                            let authority = CurrentCrossModelRobustnessAuthority::validate_current(
                                &raw,
                                subject(&models, 248),
                                [strict_current, general_current],
                                [current_relation.unwrap()],
                                p,
                                [assessment],
                                2,
                                2,
                                MissingModelPolicy::ReportInsufficientCoverage,
                            )
                            .unwrap();
                            assert_eq!(
                                authority.design_digest(),
                                raw.canonical_digest().unwrap()
                            );
                        },
                    )
                },
            )
        },
    );
}
