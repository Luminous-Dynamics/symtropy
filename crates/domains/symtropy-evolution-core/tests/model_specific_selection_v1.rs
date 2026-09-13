use symtropy_evolution_core::{
    translate_model_specific_viability_selection, GenerationMappingAuthorityId, GenerationMappingRef,
    HeritableClassMappingAuthorityId, HeritableClassMappingRef, ModelSpecificSelectionEstimate,
    ModelSpecificSelectionEstimateId, RelativeViabilityRatio, RelativeViabilitySelectionQuantity,
    SelectionTranslationError, SelectionTranslationModelId,
    SelectionTranslationQualificationAuthorityId, SelectionTranslationQualificationRef,
    ValidatedModelSpecificSelectionEstimate, ValidatedSelectionTranslationModel,
    ViabilitySelectionTranslationModel,
};

include!("causal_viability_effect_v1.rs");

fn translation_authorities(
    frame: &ValidatedSelectionAnalysisFrame<'_>,
    mapping_offset: u8,
    qualification_offset: u8,
) -> (
    GenerationMappingRef,
    HeritableClassMappingRef,
    SelectionTranslationQualificationRef,
) {
    let predictor_definition_digest = frame.frame().rows[0].predictor_definition_digest;
    (
        GenerationMappingRef::new(
            GenerationMappingAuthorityId::new(format!(
                "generation-stage-mapping-{mapping_offset}"
            ))
            .unwrap(),
            1,
            AnalysisContentDigest::new([mapping_offset; 32]),
            frame.population_id().clone(),
            frame.context_digest(),
        ),
        HeritableClassMappingRef::new(
            HeritableClassMappingAuthorityId::new(format!(
                "heritable-class-mapping-{mapping_offset}"
            ))
            .unwrap(),
            1,
            AnalysisContentDigest::new([mapping_offset.wrapping_add(32); 32]),
            predictor_definition_digest,
            frame.context_digest(),
        ),
        SelectionTranslationQualificationRef::new(
            SelectionTranslationQualificationAuthorityId::new(format!(
                "viability-model-qualification-{qualification_offset}"
            ))
            .unwrap(),
            1,
            AnalysisContentDigest::new([qualification_offset; 32]),
            frame.population_id().clone(),
            predictor_definition_digest,
            frame.context_digest(),
        ),
    )
}

#[test]
fn hereditary_viability_effect_translates_to_exact_relative_viability_minus_one() {
    let (base, design, frame, association, identification) = causal_case(false, 240);
    let (validated_frame, validated_association, validated_identification) =
        current_causal_capabilities(
            &base,
            &design,
            &frame,
            &association,
            &identification,
            false,
            240,
        );
    let effect = execute_causal_viability_risk_effect(
        CausalSelectionEffectId::new("selection-translation-effect").unwrap(),
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

    let (generation, heritable, qualification) =
        translation_authorities(&validated_frame, 50, 90);
    let model = ViabilitySelectionTranslationModel::declare(
        SelectionTranslationModelId::new("single-window-relative-viability-v1").unwrap(),
        &validated_frame,
        generation.clone(),
        heritable.clone(),
        qualification.clone(),
    )
    .unwrap();
    let validated_model = ValidatedSelectionTranslationModel::validate_current(
        &model,
        &validated_frame,
        generation.clone(),
        heritable.clone(),
        qualification.clone(),
    )
    .unwrap();

    let estimate = translate_model_specific_viability_selection(
        ModelSpecificSelectionEstimateId::new("selection-estimate-forward").unwrap(),
        &validated_frame,
        &validated_effect,
        &validated_model,
    )
    .unwrap();

    assert_eq!(estimate.reference_survival.numerator, 1);
    assert_eq!(estimate.reference_survival.denominator, 1);
    assert_eq!(estimate.comparison_survival.numerator, 0);
    assert_eq!(estimate.comparison_survival.denominator, 1);
    assert!(matches!(
        estimate.relative_viability_ratio,
        RelativeViabilityRatio::Finite(value)
            if value.numerator == 0 && value.denominator == 1
    ));
    assert!(matches!(
        estimate.selection_quantity,
        RelativeViabilitySelectionQuantity::Finite(value)
            if value.negative && value.numerator == 1 && value.denominator == 1
    ));

    let model_bytes = serde_json::to_vec(&model).unwrap();
    let restored_model: ViabilitySelectionTranslationModel =
        serde_json::from_slice(&model_bytes).unwrap();
    let current_model = ValidatedSelectionTranslationModel::validate_current(
        &restored_model,
        &validated_frame,
        generation,
        heritable,
        qualification,
    )
    .unwrap();

    let estimate_bytes = serde_json::to_vec(&estimate).unwrap();
    let restored_estimate: ModelSpecificSelectionEstimate =
        serde_json::from_slice(&estimate_bytes).unwrap();
    let current_estimate = ValidatedModelSpecificSelectionEstimate::validate_current(
        &restored_estimate,
        &validated_frame,
        &validated_effect,
        &current_model,
    )
    .unwrap();
    assert_eq!(
        current_estimate.estimate_digest(),
        estimate.canonical_digest().unwrap()
    );
}

#[test]
fn zero_reference_survival_is_typed_positive_infinity_without_continuity_correction() {
    let (base, design, frame, association, identification) = causal_case(true, 241);
    let (validated_frame, validated_association, validated_identification) =
        current_causal_capabilities(
            &base,
            &design,
            &frame,
            &association,
            &identification,
            true,
            241,
        );
    let effect = execute_causal_viability_risk_effect(
        CausalSelectionEffectId::new("selection-translation-infinity").unwrap(),
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
    let (generation, heritable, qualification) =
        translation_authorities(&validated_frame, 51, 91);
    let model = ViabilitySelectionTranslationModel::declare(
        SelectionTranslationModelId::new("single-window-relative-viability-v1").unwrap(),
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
        ModelSpecificSelectionEstimateId::new("selection-estimate-infinity").unwrap(),
        &validated_frame,
        &validated_effect,
        &validated_model,
    )
    .unwrap();

    assert_eq!(estimate.reference_survival.numerator, 0);
    assert_eq!(estimate.comparison_survival.numerator, 1);
    assert_eq!(estimate.relative_viability_ratio, RelativeViabilityRatio::PositiveInfinity);
    assert_eq!(
        estimate.selection_quantity,
        RelativeViabilitySelectionQuantity::PositiveInfinity
    );
}

#[test]
fn qualification_only_drift_changes_selection_evidence_identity_not_numeric_quantity() {
    let (base, design, frame, association, identification) = causal_case(false, 242);
    let (validated_frame, validated_association, validated_identification) =
        current_causal_capabilities(
            &base,
            &design,
            &frame,
            &association,
            &identification,
            false,
            242,
        );
    let effect = execute_causal_viability_risk_effect(
        CausalSelectionEffectId::new("selection-translation-qualified-effect").unwrap(),
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

    let (generation, heritable, qualification_a) =
        translation_authorities(&validated_frame, 52, 92);
    let (_, _, qualification_b) = translation_authorities(&validated_frame, 52, 93);
    let model_a = ViabilitySelectionTranslationModel::declare(
        SelectionTranslationModelId::new("qualification-sensitive-model").unwrap(),
        &validated_frame,
        generation.clone(),
        heritable.clone(),
        qualification_a.clone(),
    )
    .unwrap();
    let model_b = ViabilitySelectionTranslationModel::declare(
        SelectionTranslationModelId::new("qualification-sensitive-model").unwrap(),
        &validated_frame,
        generation.clone(),
        heritable.clone(),
        qualification_b.clone(),
    )
    .unwrap();
    let validated_a = ValidatedSelectionTranslationModel::validate_current(
        &model_a,
        &validated_frame,
        generation.clone(),
        heritable.clone(),
        qualification_a,
    )
    .unwrap();
    let validated_b = ValidatedSelectionTranslationModel::validate_current(
        &model_b,
        &validated_frame,
        generation,
        heritable,
        qualification_b,
    )
    .unwrap();
    let estimate_a = translate_model_specific_viability_selection(
        ModelSpecificSelectionEstimateId::new("qualification-sensitive-estimate").unwrap(),
        &validated_frame,
        &validated_effect,
        &validated_a,
    )
    .unwrap();
    let estimate_b = translate_model_specific_viability_selection(
        ModelSpecificSelectionEstimateId::new("qualification-sensitive-estimate").unwrap(),
        &validated_frame,
        &validated_effect,
        &validated_b,
    )
    .unwrap();

    assert_eq!(estimate_a.selection_quantity, estimate_b.selection_quantity);
    assert_ne!(model_a.canonical_digest().unwrap(), model_b.canonical_digest().unwrap());
    assert_ne!(
        estimate_a.canonical_digest().unwrap(),
        estimate_b.canonical_digest().unwrap()
    );
}

#[test]
fn stale_model_qualification_cannot_regain_current_authority() {
    let (base, design, frame, _association, _identification) = causal_case(false, 243);
    let validated_frame = current_frame(&base, &design, &frame, false);
    let (generation, heritable, qualification_a) =
        translation_authorities(&validated_frame, 53, 94);
    let (_, _, qualification_b) = translation_authorities(&validated_frame, 53, 95);
    let model = ViabilitySelectionTranslationModel::declare(
        SelectionTranslationModelId::new("stale-qualification-model").unwrap(),
        &validated_frame,
        generation.clone(),
        heritable.clone(),
        qualification_a,
    )
    .unwrap();

    let restored: ViabilitySelectionTranslationModel =
        serde_json::from_slice(&serde_json::to_vec(&model).unwrap()).unwrap();
    assert!(matches!(
        ValidatedSelectionTranslationModel::validate_current(
            &restored,
            &validated_frame,
            generation,
            heritable,
            qualification_b,
        ),
        Err(SelectionTranslationError::ModelReplayMismatch)
    ));
}

#[test]
fn altered_serialized_selection_quantity_is_locally_invalid() {
    let (base, design, frame, association, identification) = causal_case(false, 244);
    let (validated_frame, validated_association, validated_identification) =
        current_causal_capabilities(
            &base,
            &design,
            &frame,
            &association,
            &identification,
            false,
            244,
        );
    let effect = execute_causal_viability_risk_effect(
        CausalSelectionEffectId::new("selection-translation-tamper-effect").unwrap(),
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
    let (generation, heritable, qualification) =
        translation_authorities(&validated_frame, 54, 96);
    let model = ViabilitySelectionTranslationModel::declare(
        SelectionTranslationModelId::new("tamper-model").unwrap(),
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
        ModelSpecificSelectionEstimateId::new("tamper-estimate").unwrap(),
        &validated_frame,
        &validated_effect,
        &validated_model,
    )
    .unwrap();

    let mut value = serde_json::to_value(estimate).unwrap();
    value["reference_survival"]["numerator"] = serde_json::json!(0);
    let altered: ModelSpecificSelectionEstimate = serde_json::from_value(value).unwrap();
    assert!(matches!(
        altered.canonical_digest(),
        Err(SelectionTranslationError::ResultArithmeticMismatch)
    ));
}

fn collect_selection_keys(value: &serde_json::Value, keys: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, nested) in object {
                keys.push(key.clone());
                collect_selection_keys(nested, keys);
            }
        }
        serde_json::Value::Array(values) => {
            for nested in values {
                collect_selection_keys(nested, keys);
            }
        }
        _ => {}
    }
}

#[test]
fn model_specific_selection_wire_shape_does_not_claim_generic_fitness_or_adaptation() {
    let (base, design, frame, association, identification) = causal_case(false, 245);
    let (validated_frame, validated_association, validated_identification) =
        current_causal_capabilities(
            &base,
            &design,
            &frame,
            &association,
            &identification,
            false,
            245,
        );
    let effect = execute_causal_viability_risk_effect(
        CausalSelectionEffectId::new("selection-translation-wire-effect").unwrap(),
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
    let (generation, heritable, qualification) =
        translation_authorities(&validated_frame, 55, 97);
    let model = ViabilitySelectionTranslationModel::declare(
        SelectionTranslationModelId::new("wire-model").unwrap(),
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
        ModelSpecificSelectionEstimateId::new("wire-estimate").unwrap(),
        &validated_frame,
        &validated_effect,
        &validated_model,
    )
    .unwrap();

    let value = serde_json::to_value(estimate).unwrap();
    let mut keys = Vec::new();
    collect_selection_keys(&value, &mut keys);
    for forbidden in [
        "fitness",
        "lifetime_fitness",
        "selection_coefficient",
        "beneficial",
        "deleterious",
        "adaptation",
        "reproductive_isolation",
        "speciation",
    ] {
        assert!(!keys.iter().any(|key| key == forbidden));
    }
}
