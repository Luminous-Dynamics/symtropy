use symtropy_evolution_core::{AnalysisAuthorityRef, AnalysisContentDigest, AnalysisMethodId};
use symtropy_species_concept_robustness::*;
use symtropy_species_concept_relations::{
    RelationMissingPolicy, SpeciesConceptRelationAssertion, SpeciesConceptRelationAssessment,
    SpeciesConceptRelationDesign, SpeciesConceptRelationDesignId, SpeciesConceptRelationDirection,
    SpeciesConceptRelationEvidence, SpeciesConceptRelationKind, SpeciesConceptRelationScopeDigest,
    SpeciesConceptRelationScopeId, SpeciesConceptRelationScopeRef,
    SpeciesConceptSemanticMappingDigest, ValidatedSpeciesConceptRelationDesign,
    ValidatedSpeciesConceptRelationEvidence,
};

fn auth(label: &str, byte: u8) -> AnalysisAuthorityRef {
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new(label).unwrap(),
        1,
        AnalysisContentDigest::new([byte; 32]),
    )
}

fn fault_id(value: &str) -> FaultDomainId {
    FaultDomainId::new(value).unwrap()
}

fn profile_for(
    descriptor: &symtropy_species_concept::SpeciesConceptFamilyDescriptor,
    descriptor_digest: symtropy_species_concept::SpeciesConceptFamilyDescriptorDigest,
    prefix: &str,
    evidence_source: &str,
    upstream_evidence: &str,
    byte: u8,
) -> ModelFaultDomainProfile {
    ModelFaultDomainProfile::new(
        ModelFaultDomainProfileId::new(format!("{prefix}-profile")).unwrap(),
        descriptor.conceptual_identity.clone(),
        descriptor_digest,
        fault_id(&format!("{prefix}-qualification-organization")),
        fault_id(&format!("{prefix}-qualification-process")),
        fault_id(evidence_source),
        fault_id(&format!("{prefix}-implementation-toolchain")),
        fault_id(upstream_evidence),
        fault_id(&format!("{prefix}-semantic-mapping")),
        auth(&format!("{prefix}-profile-qualification"), byte),
    )
    .unwrap()
}

mod strict_fixture {
    include!("../../symtropy-evolution-core/tests/current_species_status_v1.rs");

    use super::profile_for;
    use symtropy_evolution_core::{SpeciesConceptAuthority, ValidatedSpeciesConceptAuthority};
    use symtropy_species_concept::{
        SpeciesConceptFamilyDescriptor, ValidatedSpeciesConceptFamilyDescriptor,
    };
    use symtropy_species_concept_robustness::SpeciesModelDesignRecord;

    pub(super) fn record(
        evidence_source: &str,
        upstream_evidence: &str,
        profile_byte: u8,
    ) -> SpeciesModelDesignRecord {
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

                let authority_model = SpeciesConceptAuthority::from_strict_biological(&current_model)
                    .unwrap();
                let current_authority =
                    ValidatedSpeciesConceptAuthority::validate_current_strict_biological(
                        &authority_model,
                        &current_model,
                    )
                    .unwrap();
                let descriptor =
                    SpeciesConceptFamilyDescriptor::from_strict_biological(&current_authority)
                        .unwrap();
                let current_descriptor =
                    ValidatedSpeciesConceptFamilyDescriptor::validate_current_strict_biological(
                        &descriptor,
                        &current_authority,
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
                SpeciesModelDesignRecord::strict_biological(
                    &current_descriptor,
                    &current_design,
                    profile,
                )
                .unwrap()
            })
        })
    }
}

mod general_fixture {
    include!("../../symtropy-species-concept-general-lineage/tests/general_lineage_v1.rs");

    use super::profile_for;
    use symtropy_species_concept_general_lineage::{
        general_lineage_family_descriptor, ValidatedGeneralLineageFamilyDescriptor,
    };
    use symtropy_species_concept_robustness::SpeciesModelDesignRecord;

    pub(super) fn record(
        evidence_source: &str,
        upstream_evidence: &str,
        profile_byte: u8,
    ) -> SpeciesModelDesignRecord {
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
            SpeciesModelDesignRecord::general_lineage(
                &current_descriptor,
                &current_design,
                profile,
            )
            .unwrap()
        })
    }
}

fn relation_record(
    first: &SpeciesModelDesignRecord,
    second: &SpeciesModelDesignRecord,
    kind: SpeciesConceptRelationKind,
    byte: u8,
) -> SemanticRelationRecord {
    let first_identity = first.conceptual_identity().clone();
    let second_identity = second.conceptual_identity().clone();
    let scope = SpeciesConceptRelationScopeRef::new(
        SpeciesConceptRelationScopeId::new("e2a-current-species-semantics").unwrap(),
        1,
        SpeciesConceptRelationScopeDigest::new([byte; 32]),
    )
    .unwrap();
    let raw_design = SpeciesConceptRelationDesign::declare(
        SpeciesConceptRelationDesignId::new("e2a-family-relation").unwrap(),
        first_identity.clone(),
        second_identity.clone(),
        scope.clone(),
        auth("e2a-relation-protocol", byte.wrapping_add(1)),
        RelationMissingPolicy::FailClosed,
    )
    .unwrap();
    let current_design = ValidatedSpeciesConceptRelationDesign::validate_current(
        &raw_design,
        first_identity.clone(),
        second_identity.clone(),
        scope,
        auth("e2a-relation-protocol", byte.wrapping_add(1)),
        RelationMissingPolicy::FailClosed,
    )
    .unwrap();

    let direction = if kind == SpeciesConceptRelationKind::OperationalCriterionWithin {
        let strict_is_left = raw_design.left.family_id.as_str() == "strict-biological-species";
        if strict_is_left {
            SpeciesConceptRelationDirection::LeftToRight
        } else {
            SpeciesConceptRelationDirection::RightToLeft
        }
    } else {
        SpeciesConceptRelationDirection::Symmetric
    };
    let assessment = SpeciesConceptRelationAssessment::Qualified {
        assertion: SpeciesConceptRelationAssertion::new(kind, direction).unwrap(),
        semantic_mapping_digest: SpeciesConceptSemanticMappingDigest::new([
            byte.wrapping_add(2);
            32
        ]),
        scientific_reference_authority: auth(
            "e2a-relation-scientific-reference",
            byte.wrapping_add(3),
        ),
        qualification_authority: auth(
            "e2a-relation-qualification",
            byte.wrapping_add(4),
        ),
    };
    let evidence = SpeciesConceptRelationEvidence::evaluate(&current_design, assessment.clone())
        .unwrap();
    let current = ValidatedSpeciesConceptRelationEvidence::validate_current(
        &evidence,
        &current_design,
        assessment,
    )
    .unwrap();
    SemanticRelationRecord::from_current(&current).unwrap()
}

fn policy(dimensions: impl IntoIterator<Item = FaultDomainDimension>, byte: u8) -> FaultDomainIndependencePolicy {
    FaultDomainIndependencePolicy::declare(
        dimensions,
        auth("e2a-fault-policy-qualification", byte),
    )
    .unwrap()
}

fn subject(models: &[SpeciesModelDesignRecord], byte: u8) -> SharedEvidenceSubject {
    SharedEvidenceSubject::new(
        models[0].lineage_a.clone(),
        models[0].lineage_b.clone(),
        models[0].lineage_history_design_digest,
        auth("e2a-common-evidence-universe", byte),
    )
    .unwrap()
}

fn design_with(
    models: Vec<SpeciesModelDesignRecord>,
    relation_kind: SpeciesConceptRelationKind,
    policy: FaultDomainIndependencePolicy,
    disposition: PairwiseFaultDomainDisposition,
) -> Result<CurrentCrossModelRobustnessDesign, RobustnessDesignError> {
    let relation = relation_record(&models[0], &models[1], relation_kind, 180);
    let assessment = PairwiseFaultDomainAssessment::new(
        &policy,
        &models[0].fault_profile,
        &models[1].fault_profile,
        disposition,
        auth("e2a-pair-fault-qualification", 190),
    )?;
    CurrentCrossModelRobustnessDesign::declare(
        CurrentRobustnessDesignId::new("e2a-two-family-design").unwrap(),
        subject(&models, 191),
        models,
        [relation],
        policy,
        [assessment],
        2,
        2,
        MissingModelPolicy::ReportInsufficientCoverage,
    )
}

fn independent_models() -> Vec<SpeciesModelDesignRecord> {
    vec![
        strict_fixture::record("strict-sequence-evidence", "strict-upstream-authority", 160),
        general_fixture::record(
            "general-multichannel-evidence",
            "general-upstream-authority",
            161,
        ),
    ]
}

#[test]
fn real_strict_and_general_lineage_designs_share_subject_without_identical_criteria() {
    let models = independent_models();
    assert_eq!(models[0].lineage_a, models[1].lineage_a);
    assert_eq!(models[0].lineage_b, models[1].lineage_b);
    assert_eq!(
        models[0].lineage_history_design_digest,
        models[1].lineage_history_design_digest
    );
    assert_ne!(models[0].classification_design, models[1].classification_design);

    let p = policy(
        [
            FaultDomainDimension::EvidenceSourceLineage,
            FaultDomainDimension::UpstreamEvidenceAuthorityLineage,
        ],
        192,
    );
    let design = design_with(
        models,
        SpeciesConceptRelationKind::OperationalCriterionWithin,
        p,
        PairwiseFaultDomainDisposition::QualifiedSufficientlyDistinct,
    )
    .unwrap();
    assert_eq!(design.models.len(), 2);
}

#[test]
fn nested_semantics_blocks_independent_coverage_even_when_fault_domains_differ() {
    let models = independent_models();
    let first = models[0].conceptual_identity().clone();
    let second = models[1].conceptual_identity().clone();
    let design = design_with(
        models,
        SpeciesConceptRelationKind::OperationalCriterionWithin,
        policy(
            [
                FaultDomainDimension::EvidenceSourceLineage,
                FaultDomainDimension::UpstreamEvidenceAuthorityLineage,
            ],
            193,
        ),
        PairwiseFaultDomainDisposition::QualifiedSufficientlyDistinct,
    )
    .unwrap();
    assert!(!design.pair_is_eligible_for_independent_coverage(&first, &second));
}

#[test]
fn potentially_non_nested_plus_qualified_fault_diversity_is_only_pair_eligibility() {
    let models = independent_models();
    let first = models[0].conceptual_identity().clone();
    let second = models[1].conceptual_identity().clone();
    let design = design_with(
        models,
        SpeciesConceptRelationKind::OrthogonalEvidenceFramework,
        policy(
            [
                FaultDomainDimension::EvidenceSourceLineage,
                FaultDomainDimension::UpstreamEvidenceAuthorityLineage,
            ],
            194,
        ),
        PairwiseFaultDomainDisposition::QualifiedSufficientlyDistinct,
    )
    .unwrap();
    assert!(design.pair_is_eligible_for_independent_coverage(&first, &second));
}

#[test]
fn shared_required_evidence_fault_domain_cannot_be_qualified_independent() {
    let models = vec![
        strict_fixture::record("shared-evidence-source", "strict-upstream", 162),
        general_fixture::record("shared-evidence-source", "general-upstream", 163),
    ];
    let p = policy(
        [
            FaultDomainDimension::EvidenceSourceLineage,
            FaultDomainDimension::UpstreamEvidenceAuthorityLineage,
        ],
        195,
    );
    assert!(matches!(
        PairwiseFaultDomainAssessment::new(
            &p,
            &models[0].fault_profile,
            &models[1].fault_profile,
            PairwiseFaultDomainDisposition::QualifiedSufficientlyDistinct,
            auth("e2a-pair-fault-qualification", 196),
        ),
        Err(RobustnessDesignError::QualifiedIndependenceSharesRequiredDimension(
            FaultDomainDimension::EvidenceSourceLineage
        ))
    ));
}

#[test]
fn independence_policy_requires_evidence_dimension_and_at_least_two_dimensions() {
    assert!(matches!(
        FaultDomainIndependencePolicy::declare(
            [FaultDomainDimension::QualificationProcess],
            auth("policy", 197),
        ),
        Err(RobustnessDesignError::FaultPolicyTooFewDimensions)
    ));
    assert!(matches!(
        FaultDomainIndependencePolicy::declare(
            [
                FaultDomainDimension::QualificationOrganization,
                FaultDomainDimension::ImplementationToolchainLineage,
            ],
            auth("policy", 198),
        ),
        Err(RobustnessDesignError::FaultPolicyMissingEvidenceDimension)
    ));
}

#[test]
fn duplicate_conceptual_identity_cannot_inflate_model_count() {
    let strict = strict_fixture::record("strict-evidence", "strict-upstream", 164);
    let duplicate = strict.clone();
    let models = vec![strict, duplicate];
    assert!(matches!(
        CurrentCrossModelRobustnessDesign::declare(
            CurrentRobustnessDesignId::new("duplicate-family").unwrap(),
            subject(&models, 199),
            models,
            [],
            policy(
                [
                    FaultDomainDimension::EvidenceSourceLineage,
                    FaultDomainDimension::UpstreamEvidenceAuthorityLineage,
                ],
                200,
            ),
            [],
            2,
            2,
            MissingModelPolicy::ReportInsufficientCoverage,
        ),
        Err(RobustnessDesignError::DuplicateConceptualIdentity)
    ));
}

#[test]
fn model_order_is_non_semantic_and_canonical() {
    let models = independent_models();
    let p = policy(
        [
            FaultDomainDimension::EvidenceSourceLineage,
            FaultDomainDimension::UpstreamEvidenceAuthorityLineage,
        ],
        201,
    );
    let normal = design_with(
        models.clone(),
        SpeciesConceptRelationKind::OperationalCriterionWithin,
        p.clone(),
        PairwiseFaultDomainDisposition::KnownDependent,
    )
    .unwrap();
    let reversed = design_with(
        models.into_iter().rev().collect(),
        SpeciesConceptRelationKind::OperationalCriterionWithin,
        p,
        PairwiseFaultDomainDisposition::KnownDependent,
    )
    .unwrap();
    assert_eq!(normal.canonical_digest().unwrap(), reversed.canonical_digest().unwrap());
}

#[test]
fn missing_relation_or_fault_assessment_coverage_is_rejected() {
    let models = independent_models();
    let p = policy(
        [
            FaultDomainDimension::EvidenceSourceLineage,
            FaultDomainDimension::UpstreamEvidenceAuthorityLineage,
        ],
        202,
    );
    let relation = relation_record(
        &models[0],
        &models[1],
        SpeciesConceptRelationKind::OperationalCriterionWithin,
        203,
    );
    let assessment = PairwiseFaultDomainAssessment::new(
        &p,
        &models[0].fault_profile,
        &models[1].fault_profile,
        PairwiseFaultDomainDisposition::KnownDependent,
        auth("pair", 204),
    )
    .unwrap();

    assert!(matches!(
        CurrentCrossModelRobustnessDesign::declare(
            CurrentRobustnessDesignId::new("missing-relation").unwrap(),
            subject(&models, 205),
            models.clone(),
            [],
            p.clone(),
            [assessment.clone()],
            2,
            2,
            MissingModelPolicy::ReportInsufficientCoverage,
        ),
        Err(RobustnessDesignError::IncompleteRelationCoverage)
    ));
    assert!(matches!(
        CurrentCrossModelRobustnessDesign::declare(
            CurrentRobustnessDesignId::new("missing-fault").unwrap(),
            subject(&models, 205),
            models,
            [relation],
            p,
            [],
            2,
            2,
            MissingModelPolicy::ReportInsufficientCoverage,
        ),
        Err(RobustnessDesignError::IncompleteFaultAssessmentCoverage)
    ));
}

#[test]
fn policy_drift_stales_pair_assessment() {
    let models = independent_models();
    let old_policy = policy(
        [
            FaultDomainDimension::EvidenceSourceLineage,
            FaultDomainDimension::UpstreamEvidenceAuthorityLineage,
        ],
        206,
    );
    let old_assessment = PairwiseFaultDomainAssessment::new(
        &old_policy,
        &models[0].fault_profile,
        &models[1].fault_profile,
        PairwiseFaultDomainDisposition::QualifiedSufficientlyDistinct,
        auth("pair", 207),
    )
    .unwrap();
    let new_policy = policy(
        [
            FaultDomainDimension::QualificationProcess,
            FaultDomainDimension::EvidenceSourceLineage,
        ],
        208,
    );
    let relation = relation_record(
        &models[0],
        &models[1],
        SpeciesConceptRelationKind::OrthogonalEvidenceFramework,
        209,
    );
    assert!(matches!(
        CurrentCrossModelRobustnessDesign::declare(
            CurrentRobustnessDesignId::new("policy-drift").unwrap(),
            subject(&models, 210),
            models,
            [relation],
            new_policy,
            [old_assessment],
            2,
            2,
            MissingModelPolicy::ReportInsufficientCoverage,
        ),
        Err(RobustnessDesignError::FaultAssessmentPolicyMismatch)
    ));
}

#[test]
fn restored_source_kind_swap_is_rejected() {
    let models = independent_models();
    let general_design = serde_json::to_value(&models[1].classification_design).unwrap();
    let mut value = serde_json::to_value(&models[0]).unwrap();
    value["classification_design"] = general_design;
    let forged: SpeciesModelDesignRecord = serde_json::from_value(value).unwrap();
    let forged_models = vec![forged, models[1].clone()];
    assert!(matches!(
        CurrentCrossModelRobustnessDesign::declare(
            CurrentRobustnessDesignId::new("source-kind-swap").unwrap(),
            subject(&forged_models, 211),
            forged_models,
            [],
            policy(
                [
                    FaultDomainDimension::EvidenceSourceLineage,
                    FaultDomainDimension::UpstreamEvidenceAuthorityLineage,
                ],
                212,
            ),
            [],
            2,
            2,
            MissingModelPolicy::ReportInsufficientCoverage,
        ),
        Err(RobustnessDesignError::ClassificationSourceKindMismatch)
    ));
}

#[test]
fn serde_restore_requires_fresh_exact_replay() {
    let models = independent_models();
    let p = policy(
        [
            FaultDomainDimension::EvidenceSourceLineage,
            FaultDomainDimension::UpstreamEvidenceAuthorityLineage,
        ],
        213,
    );
    let relation = relation_record(
        &models[0],
        &models[1],
        SpeciesConceptRelationKind::OperationalCriterionWithin,
        214,
    );
    let assessment = PairwiseFaultDomainAssessment::new(
        &p,
        &models[0].fault_profile,
        &models[1].fault_profile,
        PairwiseFaultDomainDisposition::KnownDependent,
        auth("pair", 215),
    )
    .unwrap();
    let raw = CurrentCrossModelRobustnessDesign::declare(
        CurrentRobustnessDesignId::new("serde-replay").unwrap(),
        subject(&models, 216),
        models.clone(),
        [relation.clone()],
        p.clone(),
        [assessment.clone()],
        2,
        2,
        MissingModelPolicy::ReportInsufficientCoverage,
    )
    .unwrap();
    let restored: CurrentCrossModelRobustnessDesign =
        serde_json::from_slice(&serde_json::to_vec(&raw).unwrap()).unwrap();
    let current = ValidatedCurrentCrossModelRobustnessDesign::validate_current(
        &restored,
        subject(&models, 216),
        models,
        [relation],
        p,
        [assessment],
        2,
        2,
        MissingModelPolicy::ReportInsufficientCoverage,
    )
    .unwrap();
    assert_eq!(current.design_digest(), restored.canonical_digest().unwrap());
}

#[test]
fn wire_shape_is_outcome_free() {
    let models = independent_models();
    let raw = design_with(
        models,
        SpeciesConceptRelationKind::OperationalCriterionWithin,
        policy(
            [
                FaultDomainDimension::EvidenceSourceLineage,
                FaultDomainDimension::UpstreamEvidenceAuthorityLineage,
            ],
            217,
        ),
        PairwiseFaultDomainDisposition::KnownDependent,
    )
    .unwrap();
    let text = serde_json::to_string(&raw).unwrap();
    for forbidden in [
        "SupportedUnderModel",
        "ContradictedUnderModel",
        "SupportedUnderGeneralLineageModel",
        "current_species_status",
        "speciation_transition",
        "majority_species",
        "universal_taxonomy_truth",
    ] {
        assert!(!text.contains(forbidden));
    }
}
