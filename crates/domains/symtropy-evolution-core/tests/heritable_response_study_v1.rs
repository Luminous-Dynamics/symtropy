use symtropy_evolution_core::{
    ExpectedHeritableResponseDirection, GenerationEvidenceDisposition,
    GenerationResponseEvidenceInput, GenerationTraitDirection, HeritableResponseContextPolicy,
    HeritableResponseDesignError, HeritableResponseError, HeritableResponseStudy,
    HeritableResponseStudyDesign, HeritableResponseStudyId, HeritableResponseStudyStatus,
    TransmissionEvidenceStatus, ValidatedHeritableResponseStudy,
    ValidatedHeritableResponseStudyDesign,
};

include!("model_specific_selection_v1.rs");

struct OwnedB1Selection {
    base: Fixture,
    design: SelectionComparisonDesign,
    frame: ExplicitSelectionAnalysisFrame,
    association: ConsequenceAssociationEstimate,
    identification: CausalSelectionIdentification,
    effect: CausalViabilityRiskEffect,
    model: ViabilitySelectionTranslationModel,
    estimate: ModelSpecificSelectionEstimate,
    reverse_groups: bool,
    criterion_offset: u8,
    mapping_offset: u8,
    qualification_offset: u8,
}

fn owned_b1_selection(reverse_groups: bool, offset: u8) -> OwnedB1Selection {
    let (base, design, frame, association, identification) = causal_case(reverse_groups, offset);
    let (validated_frame, validated_association, validated_identification) =
        current_causal_capabilities(
            &base,
            &design,
            &frame,
            &association,
            &identification,
            reverse_groups,
            offset,
        );
    let effect = execute_causal_viability_risk_effect(
        CausalSelectionEffectId::new(format!("b1-causal-effect-{offset}")).unwrap(),
        &validated_frame,
        &validated_association,
        &validated_identification,
    )
    .unwrap();
    let validated_effect = ValidatedCausalSelectionEffect::validate_current(
        &effect,
        &validated_frame,
        &validated_association,
        &validated_identification,
    )
    .unwrap();
    let mapping_offset = offset.wrapping_add(10);
    let qualification_offset = offset.wrapping_add(40);
    let (generation, heritable, qualification) =
        translation_authorities(&validated_frame, mapping_offset, qualification_offset);
    let model = ViabilitySelectionTranslationModel::declare(
        SelectionTranslationModelId::new(format!("b1-selection-model-{offset}")).unwrap(),
        &validated_frame,
        generation.clone(),
        heritable.clone(),
        qualification.clone(),
    )
    .unwrap();
    let validated_model = ValidatedSelectionTranslationModel::validate_current(
        &model,
        &validated_frame,
        generation,
        heritable,
        qualification,
    )
    .unwrap();
    let estimate = translate_model_specific_viability_selection(
        ModelSpecificSelectionEstimateId::new(format!("b1-selection-estimate-{offset}")).unwrap(),
        &validated_frame,
        &validated_effect,
        &validated_model,
    )
    .unwrap();
    OwnedB1Selection {
        base,
        design,
        frame,
        association,
        identification,
        effect,
        model,
        estimate,
        reverse_groups,
        criterion_offset: offset,
        mapping_offset,
        qualification_offset,
    }
}

fn current_b1_selection<'a>(case: &'a OwnedB1Selection) -> ValidatedModelSpecificSelectionEstimate<'a> {
    let (validated_frame, validated_association, validated_identification) =
        current_causal_capabilities(
            &case.base,
            &case.design,
            &case.frame,
            &case.association,
            &case.identification,
            case.reverse_groups,
            case.criterion_offset,
        );
    let validated_effect = ValidatedCausalSelectionEffect::validate_current(
        &case.effect,
        &validated_frame,
        &validated_association,
        &validated_identification,
    )
    .unwrap();
    let (generation, heritable, qualification) = translation_authorities(
        &validated_frame,
        case.mapping_offset,
        case.qualification_offset,
    );
    let validated_model = ValidatedSelectionTranslationModel::validate_current(
        &case.model,
        &validated_frame,
        generation,
        heritable,
        qualification,
    )
    .unwrap();
    ValidatedModelSpecificSelectionEstimate::validate_current(
        &case.estimate,
        &validated_frame,
        &validated_effect,
        &validated_model,
    )
    .unwrap()
}

#[derive(Clone)]
struct B1Authorities {
    generation_axis: AnalysisAuthorityRef,
    hereditary_frequency: AnalysisAuthorityRef,
    trait_response: AnalysisAuthorityRef,
    transmission: AnalysisAuthorityRef,
    demography: AnalysisAuthorityRef,
    response_rule: AnalysisAuthorityRef,
}

fn b1_authority(label: &str, byte: u8) -> AnalysisAuthorityRef {
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new(label).unwrap(),
        1,
        AnalysisContentDigest::new([byte; 32]),
    )
}

fn b1_authorities(seed: u8) -> B1Authorities {
    B1Authorities {
        generation_axis: b1_authority("b1-generation-axis", seed),
        hereditary_frequency: b1_authority("b1-hereditary-frequency", seed.wrapping_add(1)),
        trait_response: b1_authority("b1-trait-response", seed.wrapping_add(2)),
        transmission: b1_authority("b1-transmission", seed.wrapping_add(3)),
        demography: b1_authority("b1-demography", seed.wrapping_add(4)),
        response_rule: b1_authority("b1-response-rule", seed.wrapping_add(5)),
    }
}

fn b1_design(
    selection: &ValidatedModelSpecificSelectionEstimate<'_>,
    authorities: &B1Authorities,
) -> HeritableResponseStudyDesign {
    HeritableResponseStudyDesign::declare(
        HeritableResponseStudyId::new("three-generation-response-study").unwrap(),
        selection,
        symtropy_evolution_core::PopulationGeneration(0),
        symtropy_evolution_core::PopulationGeneration(2),
        HeritableResponseContextPolicy::ExactSelectionContext,
        authorities.generation_axis.clone(),
        authorities.hereditary_frequency.clone(),
        authorities.trait_response.clone(),
        authorities.transmission.clone(),
        authorities.demography.clone(),
        authorities.response_rule.clone(),
    )
    .unwrap()
}

fn current_b1_design<'a>(
    design: &'a HeritableResponseStudyDesign,
    selection: &ValidatedModelSpecificSelectionEstimate<'_>,
    authorities: &B1Authorities,
) -> ValidatedHeritableResponseStudyDesign<'a> {
    ValidatedHeritableResponseStudyDesign::validate_current(
        design,
        selection,
        HeritableResponseContextPolicy::ExactSelectionContext,
        authorities.generation_axis.clone(),
        authorities.hereditary_frequency.clone(),
        authorities.trait_response.clone(),
        authorities.transmission.clone(),
        authorities.demography.clone(),
        authorities.response_rule.clone(),
    )
    .unwrap()
}

fn b1_trajectory_digest(case: &OwnedB1Selection, generation: u64) -> symtropy_evolution_core::PopulationTrajectoryPointDigest {
    let census = case.base.census.census_size();
    let copies = census * u64::from(case.base.schema.ploidy);
    let state = symtropy_evolution_core::PopulationGeneticState::from_counts(
        case.base.population.clone(),
        &case.base.schema,
        census,
        std::collections::BTreeMap::from([(
            locus(),
            std::collections::BTreeMap::from([(allele("a0"), copies)]),
        )]),
    )
    .unwrap();
    symtropy_evolution_core::PopulationTrajectoryPoint::declare_reference_start(
        &case.base.schema,
        &state,
        EvolutionExperimentId::new("b1-response-trajectory").unwrap(),
        symtropy_evolution_core::PopulationGeneration(generation),
    )
    .unwrap()
    .canonical_digest()
}

fn b1_input(
    case: &OwnedB1Selection,
    selection: &ValidatedModelSpecificSelectionEstimate<'_>,
    design: &HeritableResponseStudyDesign,
    generation: u64,
    focal_class_count: u64,
    trait_direction: GenerationTraitDirection,
) -> GenerationResponseEvidenceInput {
    GenerationResponseEvidenceInput {
        generation: symtropy_evolution_core::PopulationGeneration(generation),
        population_id: case.base.population.clone(),
        trajectory_point_digest: b1_trajectory_digest(case, generation),
        census_digest: case.base.census.canonical_digest().unwrap(),
        context_digest: selection.estimate().context_digest(),
        focal_class_count,
        census_size: case.base.census.census_size(),
        trait_direction,
        hereditary_frequency_evidence: design.hereditary_frequency_authority.clone(),
        trait_response_evidence: design.trait_response_authority.clone(),
        transmission: if generation == design.start_generation.0 {
            TransmissionEvidenceStatus::NotApplicableAtStudyStart
        } else {
            TransmissionEvidenceStatus::Present {
                authority: design.transmission_authority.clone(),
            }
        },
        demography_evidence: design.demography_accounting_authority.clone(),
        disposition: GenerationEvidenceDisposition::Included,
    }
}

fn directional_inputs(
    case: &OwnedB1Selection,
    selection: &ValidatedModelSpecificSelectionEstimate<'_>,
    design: &HeritableResponseStudyDesign,
) -> Vec<GenerationResponseEvidenceInput> {
    vec![
        b1_input(case, selection, design, 0, 3, GenerationTraitDirection::ReferenceFavored),
        b1_input(case, selection, design, 1, 2, GenerationTraitDirection::ReferenceFavored),
        b1_input(case, selection, design, 2, 1, GenerationTraitDirection::ReferenceFavored),
    ]
}

#[test]
fn complete_directional_study_canonicalizes_input_order_and_replays_current_authority() {
    let case = owned_b1_selection(false, 70);
    let selection = current_b1_selection(&case);
    let authorities = b1_authorities(80);
    let design = b1_design(&selection, &authorities);
    assert_eq!(
        design.expected_direction,
        ExpectedHeritableResponseDirection::ComparisonFrequencyDecrease
    );
    let current_design = current_b1_design(&design, &selection, &authorities);
    let ordered = directional_inputs(&case, &selection, &design);
    let reordered = vec![ordered[2].clone(), ordered[0].clone(), ordered[1].clone()];
    let study = HeritableResponseStudy::capture(&current_design, &selection, reordered).unwrap();
    assert_eq!(study.status, HeritableResponseStudyStatus::DirectionalResponseObserved);
    assert_eq!(
        study.records.iter().map(|record| record.generation.0).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );

    let restored_design: HeritableResponseStudyDesign =
        serde_json::from_slice(&serde_json::to_vec(&design).unwrap()).unwrap();
    let restored_current_design = current_b1_design(&restored_design, &selection, &authorities);
    let restored_study: HeritableResponseStudy =
        serde_json::from_slice(&serde_json::to_vec(&study).unwrap()).unwrap();
    let validated = ValidatedHeritableResponseStudy::validate_current(
        &restored_study,
        &restored_current_design,
        &selection,
        directional_inputs(&case, &selection, &restored_design),
    )
    .unwrap();
    assert_eq!(validated.study_digest(), study.canonical_digest().unwrap());
}

#[test]
fn omitted_or_duplicate_generation_fails_complete_interval_theorem() {
    let case = owned_b1_selection(false, 71);
    let selection = current_b1_selection(&case);
    let authorities = b1_authorities(81);
    let design = b1_design(&selection, &authorities);
    let current_design = current_b1_design(&design, &selection, &authorities);
    let inputs = directional_inputs(&case, &selection, &design);
    assert!(matches!(
        HeritableResponseStudy::capture(
            &current_design,
            &selection,
            vec![inputs[0].clone(), inputs[2].clone()],
        ),
        Err(HeritableResponseError::IncompleteGenerationCoverage)
    ));
    assert!(matches!(
        HeritableResponseStudy::capture(
            &current_design,
            &selection,
            vec![
                inputs[0].clone(),
                inputs[1].clone(),
                inputs[1].clone(),
                inputs[2].clone(),
            ],
        ),
        Err(HeritableResponseError::DuplicateGeneration(_))
    ));
}

#[test]
fn exact_context_policy_rejects_undeclared_context_drift() {
    let case = owned_b1_selection(false, 72);
    let selection = current_b1_selection(&case);
    let authorities = b1_authorities(82);
    let design = b1_design(&selection, &authorities);
    let current_design = current_b1_design(&design, &selection, &authorities);
    let mut inputs = directional_inputs(&case, &selection, &design);
    inputs[1].context_digest = symtropy_evolution_core::EvolutionaryContextRef::new(
        EvolutionaryContextId::new("b1-drifted-context").unwrap(),
        1,
        EvolutionaryContextContentDigest::new([201; 32]),
        ConsequenceWindowId::new("b1-drifted-window").unwrap(),
    )
    .canonical_digest()
    .unwrap();
    assert!(matches!(
        HeritableResponseStudy::capture(&current_design, &selection, inputs),
        Err(HeritableResponseError::UndeclaredContextDrift(_))
    ));
}

#[test]
fn hereditary_only_trait_only_and_reversed_response_are_not_promoted() {
    let case = owned_b1_selection(false, 73);
    let selection = current_b1_selection(&case);
    let authorities = b1_authorities(83);
    let design = b1_design(&selection, &authorities);
    let current_design = current_b1_design(&design, &selection, &authorities);

    let hereditary_only = vec![
        b1_input(&case, &selection, &design, 0, 3, GenerationTraitDirection::Neutral),
        b1_input(&case, &selection, &design, 1, 2, GenerationTraitDirection::Neutral),
        b1_input(&case, &selection, &design, 2, 1, GenerationTraitDirection::Neutral),
    ];
    assert_eq!(
        HeritableResponseStudy::capture(&current_design, &selection, hereditary_only)
            .unwrap()
            .status,
        HeritableResponseStudyStatus::HereditaryOnlyMismatch
    );

    let trait_only = vec![
        b1_input(&case, &selection, &design, 0, 2, GenerationTraitDirection::ReferenceFavored),
        b1_input(&case, &selection, &design, 1, 2, GenerationTraitDirection::ReferenceFavored),
        b1_input(&case, &selection, &design, 2, 2, GenerationTraitDirection::ReferenceFavored),
    ];
    assert_eq!(
        HeritableResponseStudy::capture(&current_design, &selection, trait_only)
            .unwrap()
            .status,
        HeritableResponseStudyStatus::TraitOnlyMismatch
    );

    let reversed = vec![
        b1_input(&case, &selection, &design, 0, 1, GenerationTraitDirection::ComparisonFavored),
        b1_input(&case, &selection, &design, 1, 2, GenerationTraitDirection::ComparisonFavored),
        b1_input(&case, &selection, &design, 2, 3, GenerationTraitDirection::ComparisonFavored),
    ];
    assert_eq!(
        HeritableResponseStudy::capture(&current_design, &selection, reversed)
            .unwrap()
            .status,
        HeritableResponseStudyStatus::ReversedResponse
    );
}

#[test]
fn unavailable_transmission_is_retained_as_insufficient_evidence() {
    let case = owned_b1_selection(false, 74);
    let selection = current_b1_selection(&case);
    let authorities = b1_authorities(84);
    let design = b1_design(&selection, &authorities);
    let current_design = current_b1_design(&design, &selection, &authorities);
    let mut inputs = directional_inputs(&case, &selection, &design);
    inputs[1].transmission = TransmissionEvidenceStatus::Unavailable {
        reason: b1_authority("b1-transmission-unavailable", 207),
    };
    let study = HeritableResponseStudy::capture(&current_design, &selection, inputs).unwrap();
    assert_eq!(study.status, HeritableResponseStudyStatus::InsufficientEvidence);
}

#[test]
fn stale_preregistered_authority_cannot_regain_design_authority() {
    let case = owned_b1_selection(false, 75);
    let selection = current_b1_selection(&case);
    let authorities = b1_authorities(85);
    let design = b1_design(&selection, &authorities);
    let restored: HeritableResponseStudyDesign =
        serde_json::from_slice(&serde_json::to_vec(&design).unwrap()).unwrap();
    assert!(matches!(
        ValidatedHeritableResponseStudyDesign::validate_current(
            &restored,
            &selection,
            HeritableResponseContextPolicy::ExactSelectionContext,
            authorities.generation_axis.clone(),
            authorities.hereditary_frequency.clone(),
            authorities.trait_response.clone(),
            authorities.transmission.clone(),
            authorities.demography.clone(),
            b1_authority("changed-response-rule", 250),
        ),
        Err(HeritableResponseDesignError::ReplayMismatch)
    ));
}

fn collect_b1_keys(value: &serde_json::Value, keys: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, nested) in object {
                keys.push(key.clone());
                collect_b1_keys(nested, keys);
            }
        }
        serde_json::Value::Array(values) => {
            for nested in values {
                collect_b1_keys(nested, keys);
            }
        }
        _ => {}
    }
}

#[test]
fn single_study_wire_shape_does_not_claim_adaptation_or_speciation() {
    let case = owned_b1_selection(false, 76);
    let selection = current_b1_selection(&case);
    let authorities = b1_authorities(86);
    let design = b1_design(&selection, &authorities);
    let current_design = current_b1_design(&design, &selection, &authorities);
    let study = HeritableResponseStudy::capture(
        &current_design,
        &selection,
        directional_inputs(&case, &selection, &design),
    )
    .unwrap();
    let value = serde_json::to_value(study).unwrap();
    let mut keys = Vec::new();
    collect_b1_keys(&value, &mut keys);
    for forbidden in [
        "adaptation",
        "adapted",
        "beneficial",
        "deleterious",
        "reproductive_isolation",
        "speciation",
        "species",
    ] {
        assert!(!keys.iter().any(|key| key == forbidden));
    }
}
