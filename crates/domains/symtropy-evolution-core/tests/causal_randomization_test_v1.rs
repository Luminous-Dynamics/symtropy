use symtropy_evolution_core::{
    execute_causal_viability_risk_effect, execute_exact_causal_randomization_test,
    AssignmentMaterializationQualificationAuthorityId,
    AssignmentMechanismMaterializationQualificationRef, CausalRandomizationError,
    CausalRandomizationTestId, CausalRandomizationTestResult, CausalSelectionEffectId,
    CausalViabilityRiskEffect, CompleteRandomizationReferenceModel, ConsequenceAssociationEstimate,
    ValidatedCausalRandomizationTest, ValidatedCausalSelectionEffect,
    CAUSAL_RANDOMIZATION_MAX_ASSIGNMENTS,
};

include!("complete_randomization_reference_v1.rs");

struct D2Fixture {
    base: Fixture,
    design: SelectionComparisonDesign,
    frame: ExplicitSelectionAnalysisFrame,
    identification: CausalSelectionIdentification,
    association: ConsequenceAssociationEstimate,
    effect: CausalViabilityRiskEffect,
    reference: CompleteRandomizationReferenceModel,
    reverse_groups: bool,
    criterion_offset: u8,
    d1_offset: u8,
}

fn d2_fixture(reverse_groups: bool, criterion_offset: u8, d1_offset: u8) -> D2Fixture {
    let (base, design, frame, identification) =
        randomized_identification_case(reverse_groups, criterion_offset);

    let (association, effect, reference) = {
        let (validated_frame, validated_identification) = current_randomized_capabilities(
            &base,
            &design,
            &frame,
            &identification,
            reverse_groups,
            criterion_offset,
        );
        let association = execute_binary_viability_association(&validated_frame).unwrap();
        let validated_association = ValidatedConsequenceAssociation::validate_current(
            &association,
            &validated_frame,
        )
        .unwrap();
        let effect = execute_causal_viability_risk_effect(
            CausalSelectionEffectId::new("d2-causal-effect").unwrap(),
            &validated_frame,
            &validated_association,
            &validated_identification,
        )
        .unwrap();
        let (materialization, exchangeability) =
            d1_authorities(&validated_identification, d1_offset);
        let reference = CompleteRandomizationReferenceModel::capture(
            CompleteRandomizationReferenceId::new("d2-complete-randomization").unwrap(),
            &validated_frame,
            &validated_identification,
            materialization,
            exchangeability,
        )
        .unwrap();
        (association, effect, reference)
    };

    D2Fixture {
        base,
        design,
        frame,
        identification,
        association,
        effect,
        reference,
        reverse_groups,
        criterion_offset,
        d1_offset,
    }
}

fn current_d2_capabilities<'a>(
    fixture: &'a D2Fixture,
) -> (
    ValidatedSelectionAnalysisFrame<'a>,
    ValidatedCausalSelectionIdentification<'a>,
    ValidatedConsequenceAssociation<'a>,
    ValidatedCausalSelectionEffect<'a>,
    ValidatedCompleteRandomizationReference<'a>,
) {
    let (validated_frame, validated_identification) = current_randomized_capabilities(
        &fixture.base,
        &fixture.design,
        &fixture.frame,
        &fixture.identification,
        fixture.reverse_groups,
        fixture.criterion_offset,
    );
    let validated_association = ValidatedConsequenceAssociation::validate_current(
        &fixture.association,
        &validated_frame,
    )
    .unwrap();
    let validated_effect = ValidatedCausalSelectionEffect::validate_current(
        &fixture.effect,
        &validated_frame,
        &validated_association,
        &validated_identification,
    )
    .unwrap();
    let (materialization, exchangeability) =
        d1_authorities(&validated_identification, fixture.d1_offset);
    let validated_reference = ValidatedCompleteRandomizationReference::validate_current(
        &fixture.reference,
        &validated_frame,
        &validated_identification,
        materialization,
        exchangeability,
    )
    .unwrap();
    (
        validated_frame,
        validated_identification,
        validated_association,
        validated_effect,
        validated_reference,
    )
}

#[test]
fn exact_two_by_two_randomization_enumerates_six_assignments_and_yields_one_third() {
    let fixture = d2_fixture(false, 232, 33);
    let (validated_frame, _identification, _association, validated_effect, validated_reference) =
        current_d2_capabilities(&fixture);
    let result = execute_exact_causal_randomization_test(
        CausalRandomizationTestId::new("exact-randomization-2x2").unwrap(),
        &validated_frame,
        &validated_effect,
        &validated_reference,
    )
    .unwrap();

    assert_eq!(result.support_size, 6);
    assert_eq!(result.extreme_assignments, 2);
    assert!(!result.observed_statistic.negative);
    assert_eq!(result.observed_statistic.numerator, 1);
    assert_eq!(result.observed_statistic.denominator, 1);
    assert_eq!(result.two_sided_randomization_probability.numerator, 1);
    assert_eq!(result.two_sided_randomization_probability.denominator, 3);

    let mut minus_one = 0;
    let mut zero = 0;
    let mut plus_one = 0;
    for point in &result.distribution {
        match (
            point.statistic.negative,
            point.statistic.numerator,
            point.statistic.denominator,
        ) {
            (true, 1, 1) => minus_one = point.assignments,
            (false, 0, 1) => zero = point.assignments,
            (false, 1, 1) => plus_one = point.assignments,
            other => panic!("unexpected exact statistic {other:?}"),
        }
    }
    assert_eq!((minus_one, zero, plus_one), (1, 4, 1));

    let encoded = serde_json::to_vec(&result).unwrap();
    let restored: CausalRandomizationTestResult = serde_json::from_slice(&encoded).unwrap();
    let validated = ValidatedCausalRandomizationTest::validate_current(
        &restored,
        &validated_frame,
        &validated_effect,
        &validated_reference,
    )
    .unwrap();
    assert_eq!(validated.result_digest(), result.canonical_digest().unwrap());
}

#[test]
fn causal_randomization_probability_can_equal_b2_fisher_number_without_being_b2_evidence() {
    let fixture = d2_fixture(false, 233, 34);
    let (validated_frame, _identification, _association, validated_effect, validated_reference) =
        current_d2_capabilities(&fixture);
    let result = execute_exact_causal_randomization_test(
        CausalRandomizationTestId::new("d2-vs-b2-equality").unwrap(),
        &validated_frame,
        &validated_effect,
        &validated_reference,
    )
    .unwrap();

    let ConsequenceAssociationStatus::Estimated(descriptive) = &fixture.association.status else {
        panic!("expected B2 descriptive association");
    };
    assert_eq!(
        result.two_sided_randomization_probability.numerator,
        descriptive.exact_reference.two_sided_probability_ordered_p.numerator
    );
    assert_eq!(
        result.two_sided_randomization_probability.denominator,
        descriptive.exact_reference.two_sided_probability_ordered_p.denominator
    );
    assert_eq!(result.two_sided_randomization_probability.numerator, 1);
    assert_eq!(result.two_sided_randomization_probability.denominator, 3);
    assert_eq!(result.randomization_reference_digest(), fixture.reference.canonical_digest().unwrap());
    assert_eq!(result.causal_effect_digest(), fixture.effect.canonical_digest().unwrap());
    assert_eq!(result.identification_digest(), fixture.identification.canonical_digest().unwrap());
}

#[test]
fn reversing_assignment_labels_reverses_observed_statistic_but_preserves_two_sided_probability() {
    let forward = d2_fixture(false, 234, 35);
    let reverse = d2_fixture(true, 235, 36);

    let forward_result = {
        let (frame, _identification, _association, effect, reference) =
            current_d2_capabilities(&forward);
        execute_exact_causal_randomization_test(
            CausalRandomizationTestId::new("label-direction").unwrap(),
            &frame,
            &effect,
            &reference,
        )
        .unwrap()
    };
    let reverse_result = {
        let (frame, _identification, _association, effect, reference) =
            current_d2_capabilities(&reverse);
        execute_exact_causal_randomization_test(
            CausalRandomizationTestId::new("label-direction").unwrap(),
            &frame,
            &effect,
            &reference,
        )
        .unwrap()
    };

    assert!(!forward_result.observed_statistic.negative);
    assert!(reverse_result.observed_statistic.negative);
    assert_eq!(forward_result.observed_statistic.numerator, 1);
    assert_eq!(reverse_result.observed_statistic.numerator, 1);
    assert_eq!(
        forward_result.two_sided_randomization_probability,
        reverse_result.two_sided_randomization_probability
    );
    assert_ne!(
        forward_result.canonical_digest().unwrap(),
        reverse_result.canonical_digest().unwrap()
    );
}

#[test]
fn d1_materialization_qualification_changes_d2_identity_without_changing_probability() {
    let fixture = d2_fixture(false, 236, 37);
    let (validated_frame, validated_identification, _association, validated_effect, _) =
        current_d2_capabilities(&fixture);
    let (materialization_a, exchangeability) =
        d1_authorities(&validated_identification, fixture.d1_offset);
    let mut materialization_b = materialization_a.clone();
    materialization_b.qualification = AssignmentMechanismMaterializationQualificationRef::new(
        AssignmentMaterializationQualificationAuthorityId::new(
            "d2-alternate-materialization-qualification",
        )
        .unwrap(),
        1,
        AnalysisContentDigest::new([237; 32]),
    );

    let reference_a = CompleteRandomizationReferenceModel::capture(
        CompleteRandomizationReferenceId::new("d2-qualified-reference").unwrap(),
        &validated_frame,
        &validated_identification,
        materialization_a.clone(),
        exchangeability.clone(),
    )
    .unwrap();
    let reference_b = CompleteRandomizationReferenceModel::capture(
        CompleteRandomizationReferenceId::new("d2-qualified-reference").unwrap(),
        &validated_frame,
        &validated_identification,
        materialization_b.clone(),
        exchangeability.clone(),
    )
    .unwrap();
    let validated_a = ValidatedCompleteRandomizationReference::validate_current(
        &reference_a,
        &validated_frame,
        &validated_identification,
        materialization_a,
        exchangeability.clone(),
    )
    .unwrap();
    let validated_b = ValidatedCompleteRandomizationReference::validate_current(
        &reference_b,
        &validated_frame,
        &validated_identification,
        materialization_b,
        exchangeability,
    )
    .unwrap();

    let result_a = execute_exact_causal_randomization_test(
        CausalRandomizationTestId::new("qualification-sensitive-d2").unwrap(),
        &validated_frame,
        &validated_effect,
        &validated_a,
    )
    .unwrap();
    let result_b = execute_exact_causal_randomization_test(
        CausalRandomizationTestId::new("qualification-sensitive-d2").unwrap(),
        &validated_frame,
        &validated_effect,
        &validated_b,
    )
    .unwrap();

    assert_eq!(
        result_a.two_sided_randomization_probability,
        result_b.two_sided_randomization_probability
    );
    assert_ne!(
        result_a.randomization_reference_digest(),
        result_b.randomization_reference_digest()
    );
    assert_ne!(
        result_a.canonical_digest().unwrap(),
        result_b.canonical_digest().unwrap()
    );
}

#[test]
fn tampered_serialized_probability_fails_local_validation_before_current_replay() {
    let fixture = d2_fixture(false, 238, 38);
    let (validated_frame, _identification, _association, validated_effect, validated_reference) =
        current_d2_capabilities(&fixture);
    let result = execute_exact_causal_randomization_test(
        CausalRandomizationTestId::new("tampered-d2-probability").unwrap(),
        &validated_frame,
        &validated_effect,
        &validated_reference,
    )
    .unwrap();

    let mut value = serde_json::to_value(result).unwrap();
    value["two_sided_randomization_probability"]["numerator"] = serde_json::json!(1);
    value["two_sided_randomization_probability"]["denominator"] = serde_json::json!(2);
    let altered: CausalRandomizationTestResult = serde_json::from_value(value).unwrap();
    assert!(matches!(
        altered.canonical_digest(),
        Err(CausalRandomizationError::ProbabilityInvariant)
    ));
}

#[test]
fn exact_enumeration_limit_is_explicit_and_never_an_approximation_mode() {
    assert_eq!(CAUSAL_RANDOMIZATION_MAX_ASSIGNMENTS, 100_000);
}

fn collect_d2_keys(value: &serde_json::Value, keys: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, nested) in object {
                keys.push(key.clone());
                collect_d2_keys(nested, keys);
            }
        }
        serde_json::Value::Array(values) => {
            for nested in values {
                collect_d2_keys(nested, keys);
            }
        }
        _ => {}
    }
}

#[test]
fn d2_wire_shape_names_sharp_null_without_downstream_evolutionary_promotion() {
    let fixture = d2_fixture(false, 239, 39);
    let (validated_frame, _identification, _association, validated_effect, validated_reference) =
        current_d2_capabilities(&fixture);
    let result = execute_exact_causal_randomization_test(
        CausalRandomizationTestId::new("wire-d2").unwrap(),
        &validated_frame,
        &validated_effect,
        &validated_reference,
    )
    .unwrap();
    let value = serde_json::to_value(result).unwrap();
    let mut keys = Vec::new();
    collect_d2_keys(&value, &mut keys);
    assert!(keys.iter().any(|key| key == "sharp_null"));
    assert!(keys
        .iter()
        .any(|key| key == "two_sided_randomization_probability"));
    for forbidden in [
        "fisher_p_value",
        "descriptive_p_value",
        "selection_coefficient",
        "fitness",
        "beneficial",
        "deleterious",
        "adaptation",
        "reproductive_isolation",
        "speciation",
    ] {
        assert!(!keys.iter().any(|key| key == forbidden));
    }
}
