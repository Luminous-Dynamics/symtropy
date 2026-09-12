use symtropy_evolution_core::{
    execute_causal_viability_risk_effect, CausalEffectError, CausalSelectionEffectId,
    CausalViabilityRiskEffect, ConsequenceAssociationEstimate, ValidatedCausalSelectionEffect,
};

include!("causal_identification_v1.rs");

fn causal_case(
    reverse_groups: bool,
    criterion_offset: u8,
) -> (
    Fixture,
    SelectionComparisonDesign,
    ExplicitSelectionAnalysisFrame,
    ConsequenceAssociationEstimate,
    CausalSelectionIdentification,
) {
    let (base, design, frame) =
        design_case(SelectionDesignClass::RandomizedInterventional, reverse_groups);
    let validated_frame = current_frame(&base, &design, &frame, reverse_groups);
    let association = execute_binary_viability_association(&validated_frame).unwrap();
    let identification = CausalSelectionIdentification::declare(
        CausalSelectionIdentificationId::new("causal-effect-identification").unwrap(),
        &validated_frame,
        CausalSelectionTarget::BinaryViabilityRiskDifference,
        criterion_evidence(&validated_frame, randomized_criteria(), criterion_offset),
    )
    .unwrap();
    (base, design, frame, association, identification)
}

fn current_causal_capabilities<'a>(
    base: &'a Fixture,
    design: &'a SelectionComparisonDesign,
    frame: &'a ExplicitSelectionAnalysisFrame,
    association: &'a ConsequenceAssociationEstimate,
    identification: &'a CausalSelectionIdentification,
    reverse_groups: bool,
    criterion_offset: u8,
) -> (
    ValidatedSelectionAnalysisFrame<'a>,
    ValidatedConsequenceAssociation<'a>,
    ValidatedCausalSelectionIdentification<'a>,
) {
    let validated_frame = current_frame(base, design, frame, reverse_groups);
    let validated_association =
        ValidatedConsequenceAssociation::validate_current(association, &validated_frame).unwrap();
    let fresh = criterion_evidence(&validated_frame, randomized_criteria(), criterion_offset);
    let validated_identification = ValidatedCausalSelectionIdentification::validate_current(
        identification,
        &validated_frame,
        fresh,
    )
    .unwrap();
    (
        validated_frame,
        validated_association,
        validated_identification,
    )
}

#[test]
fn intervention_identified_effect_is_exact_and_replays_current_authorities() {
    let (base, design, frame, association, identification) = causal_case(false, 160);
    let (validated_frame, validated_association, validated_identification) =
        current_causal_capabilities(
            &base,
            &design,
            &frame,
            &association,
            &identification,
            false,
            160,
        );

    let effect = execute_causal_viability_risk_effect(
        CausalSelectionEffectId::new("causal-viability-effect-v1").unwrap(),
        &validated_frame,
        &validated_association,
        &validated_identification,
    )
    .unwrap();
    assert_eq!(effect.reference_support.total, 2);
    assert_eq!(effect.reference_support.died_during_window, 0);
    assert_eq!(effect.comparison_support.total, 2);
    assert_eq!(effect.comparison_support.died_during_window, 2);
    assert!(!effect.comparison_minus_reference_causal_risk_difference.negative);
    assert_eq!(effect.comparison_minus_reference_causal_risk_difference.numerator, 1);
    assert_eq!(effect.comparison_minus_reference_causal_risk_difference.denominator, 1);
    assert_eq!(effect.frame_digest(), validated_frame.frame_digest());
    assert_eq!(effect.association_digest(), validated_association.estimate_digest());
    assert_eq!(
        effect.identification_digest(),
        validated_identification.identification_digest()
    );

    let encoded = serde_json::to_vec(&effect).unwrap();
    let restored: CausalViabilityRiskEffect = serde_json::from_slice(&encoded).unwrap();
    let validated_effect = ValidatedCausalSelectionEffect::validate_current(
        &restored,
        &validated_frame,
        &validated_association,
        &validated_identification,
    )
    .unwrap();
    assert_eq!(validated_effect.effect_digest(), effect.canonical_digest().unwrap());
}

#[test]
fn reversing_intervention_labels_reverses_exact_causal_effect_direction() {
    let (base_a, design_a, frame_a, association_a, identification_a) = causal_case(false, 170);
    let (vf_a, va_a, vi_a) = current_causal_capabilities(
        &base_a,
        &design_a,
        &frame_a,
        &association_a,
        &identification_a,
        false,
        170,
    );
    let positive = execute_causal_viability_risk_effect(
        CausalSelectionEffectId::new("direction-effect").unwrap(),
        &vf_a,
        &va_a,
        &vi_a,
    )
    .unwrap();

    let (base_b, design_b, frame_b, association_b, identification_b) = causal_case(true, 171);
    let (vf_b, va_b, vi_b) = current_causal_capabilities(
        &base_b,
        &design_b,
        &frame_b,
        &association_b,
        &identification_b,
        true,
        171,
    );
    let negative = execute_causal_viability_risk_effect(
        CausalSelectionEffectId::new("direction-effect").unwrap(),
        &vf_b,
        &va_b,
        &vi_b,
    )
    .unwrap();

    assert!(!positive.comparison_minus_reference_causal_risk_difference.negative);
    assert!(negative.comparison_minus_reference_causal_risk_difference.negative);
    assert_eq!(positive.comparison_minus_reference_causal_risk_difference.numerator, 1);
    assert_eq!(negative.comparison_minus_reference_causal_risk_difference.numerator, 1);
    assert_ne!(positive.canonical_digest().unwrap(), negative.canonical_digest().unwrap());
}

#[test]
fn same_numeric_effect_under_different_identification_qualification_has_different_identity() {
    let (base, design, frame, association, identification_a) = causal_case(false, 180);
    let validated_frame = current_frame(&base, &design, &frame, false);
    let validated_association =
        ValidatedConsequenceAssociation::validate_current(&association, &validated_frame).unwrap();
    let evidence_b = criterion_evidence(&validated_frame, randomized_criteria(), 181);
    let identification_b = CausalSelectionIdentification::declare(
        CausalSelectionIdentificationId::new("causal-effect-identification").unwrap(),
        &validated_frame,
        CausalSelectionTarget::BinaryViabilityRiskDifference,
        evidence_b.clone(),
    )
    .unwrap();
    let validated_a = ValidatedCausalSelectionIdentification::validate_current(
        &identification_a,
        &validated_frame,
        criterion_evidence(&validated_frame, randomized_criteria(), 180),
    )
    .unwrap();
    let validated_b = ValidatedCausalSelectionIdentification::validate_current(
        &identification_b,
        &validated_frame,
        evidence_b,
    )
    .unwrap();

    let effect_a = execute_causal_viability_risk_effect(
        CausalSelectionEffectId::new("same-numeric-effect").unwrap(),
        &validated_frame,
        &validated_association,
        &validated_a,
    )
    .unwrap();
    let effect_b = execute_causal_viability_risk_effect(
        CausalSelectionEffectId::new("same-numeric-effect").unwrap(),
        &validated_frame,
        &validated_association,
        &validated_b,
    )
    .unwrap();

    assert_eq!(
        effect_a.comparison_minus_reference_causal_risk_difference,
        effect_b.comparison_minus_reference_causal_risk_difference
    );
    assert_ne!(effect_a.identification_digest(), effect_b.identification_digest());
    assert_ne!(effect_a.canonical_digest().unwrap(), effect_b.canonical_digest().unwrap());
}

#[test]
fn altered_serialized_causal_point_effect_is_locally_invalid() {
    let (base, design, frame, association, identification) = causal_case(false, 190);
    let (validated_frame, validated_association, validated_identification) =
        current_causal_capabilities(
            &base,
            &design,
            &frame,
            &association,
            &identification,
            false,
            190,
        );
    let effect = execute_causal_viability_risk_effect(
        CausalSelectionEffectId::new("tamper-effect").unwrap(),
        &validated_frame,
        &validated_association,
        &validated_identification,
    )
    .unwrap();

    let mut value = serde_json::to_value(effect).unwrap();
    value["comparison_minus_reference_causal_risk_difference"]["numerator"] =
        serde_json::json!(2);
    let altered: CausalViabilityRiskEffect = serde_json::from_value(value).unwrap();
    assert!(matches!(
        altered.canonical_digest(),
        Err(CausalEffectError::EffectArithmeticInvariant)
    ));
}

#[test]
fn identification_from_another_frame_cannot_authorize_effect() {
    let (base_a, design_a, frame_a, association_a, identification_a) = causal_case(false, 200);
    let (vf_a, va_a, _vi_a) = current_causal_capabilities(
        &base_a,
        &design_a,
        &frame_a,
        &association_a,
        &identification_a,
        false,
        200,
    );

    let (base_b, design_b, frame_b, association_b, identification_b) = causal_case(true, 201);
    let (_vf_b, _va_b, vi_b) = current_causal_capabilities(
        &base_b,
        &design_b,
        &frame_b,
        &association_b,
        &identification_b,
        true,
        201,
    );

    assert!(matches!(
        execute_causal_viability_risk_effect(
            CausalSelectionEffectId::new("cross-frame-effect").unwrap(),
            &vf_a,
            &va_a,
            &vi_b,
        ),
        Err(CausalEffectError::IdentificationFrameMismatch)
    ));
}

fn collect_effect_keys(value: &serde_json::Value, keys: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, nested) in object {
                keys.push(key.clone());
                collect_effect_keys(nested, keys);
            }
        }
        serde_json::Value::Array(values) => {
            for nested in values {
                collect_effect_keys(nested, keys);
            }
        }
        _ => {}
    }
}

#[test]
fn causal_effect_wire_shape_does_not_relabel_descriptive_probability_or_claim_adaptation() {
    let (base, design, frame, association, identification) = causal_case(false, 210);
    let (validated_frame, validated_association, validated_identification) =
        current_causal_capabilities(
            &base,
            &design,
            &frame,
            &association,
            &identification,
            false,
            210,
        );
    let effect = execute_causal_viability_risk_effect(
        CausalSelectionEffectId::new("wire-effect").unwrap(),
        &validated_frame,
        &validated_association,
        &validated_identification,
    )
    .unwrap();

    let value = serde_json::to_value(effect).unwrap();
    let mut keys = Vec::new();
    collect_effect_keys(&value, &mut keys);
    for forbidden in [
        "p_value",
        "probability",
        "exact_reference",
        "hypergeometric",
        "selection_coefficient",
        "fitness",
        "beneficial",
        "deleterious",
        "adaptation",
        "speciation",
    ] {
        assert!(!keys.iter().any(|key| key == forbidden));
    }
}
