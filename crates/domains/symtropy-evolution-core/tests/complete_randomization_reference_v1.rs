use symtropy_evolution_core::{
    AssignmentExchangeabilityAuthorityId, AssignmentMaterializationAuthorityId,
    AssignmentMaterializationQualificationAuthorityId,
    AssignmentMechanismMaterializationQualificationRef, AssignmentMechanismMaterializationRef,
    CompleteRandomizationError, CompleteRandomizationReferenceId,
    CompleteRandomizationReferenceModel, IndividualAssignmentExchangeabilityRef,
    ValidatedCompleteRandomizationReference,
};

include!("causal_identification_v1.rs");

fn randomized_identification_case(
    reverse_groups: bool,
    criterion_offset: u8,
) -> (
    Fixture,
    SelectionComparisonDesign,
    ExplicitSelectionAnalysisFrame,
    CausalSelectionIdentification,
) {
    let (base, design, frame) =
        design_case(SelectionDesignClass::RandomizedInterventional, reverse_groups);
    let validated_frame = current_frame(&base, &design, &frame, reverse_groups);
    let identification = CausalSelectionIdentification::declare(
        CausalSelectionIdentificationId::new("d1-randomized-identification").unwrap(),
        &validated_frame,
        CausalSelectionTarget::BinaryViabilityRiskDifference,
        criterion_evidence(&validated_frame, randomized_criteria(), criterion_offset),
    )
    .unwrap();
    (base, design, frame, identification)
}

fn current_randomized_capabilities<'a>(
    base: &'a Fixture,
    design: &'a SelectionComparisonDesign,
    frame: &'a ExplicitSelectionAnalysisFrame,
    identification: &'a CausalSelectionIdentification,
    reverse_groups: bool,
    criterion_offset: u8,
) -> (
    ValidatedSelectionAnalysisFrame<'a>,
    ValidatedCausalSelectionIdentification<'a>,
) {
    let validated_frame = current_frame(base, design, frame, reverse_groups);
    let validated_identification = ValidatedCausalSelectionIdentification::validate_current(
        identification,
        &validated_frame,
        criterion_evidence(&validated_frame, randomized_criteria(), criterion_offset),
    )
    .unwrap();
    (validated_frame, validated_identification)
}

fn d1_authorities(
    identification: &ValidatedCausalSelectionIdentification<'_>,
    offset: u8,
) -> (
    AssignmentMechanismMaterializationRef,
    IndividualAssignmentExchangeabilityRef,
) {
    (
        AssignmentMechanismMaterializationRef::new(
            AssignmentMaterializationAuthorityId::new(format!(
                "complete-randomization-materialization-{offset}"
            ))
            .unwrap(),
            1,
            AnalysisContentDigest::new([offset; 32]),
            AssignmentMechanismMaterializationQualificationRef::new(
                AssignmentMaterializationQualificationAuthorityId::new(format!(
                    "complete-randomization-materialization-qualification-{offset}"
                ))
                .unwrap(),
                1,
                AnalysisContentDigest::new([offset.wrapping_add(32); 32]),
            ),
            identification.identification_digest(),
        ),
        IndividualAssignmentExchangeabilityRef::new(
            AssignmentExchangeabilityAuthorityId::new(format!(
                "individual-exchangeability-{offset}"
            ))
            .unwrap(),
            1,
            AnalysisContentDigest::new([offset.wrapping_add(64); 32]),
            identification.identification_digest(),
        ),
    )
}

#[test]
fn complete_two_by_two_randomization_has_exact_six_assignment_support() {
    let (base, design, frame, identification) = randomized_identification_case(false, 220);
    let (validated_frame, validated_identification) = current_randomized_capabilities(
        &base,
        &design,
        &frame,
        &identification,
        false,
        220,
    );
    let (materialization, exchangeability) = d1_authorities(&validated_identification, 21);
    let reference = CompleteRandomizationReferenceModel::capture(
        CompleteRandomizationReferenceId::new("complete-randomization-2x2").unwrap(),
        &validated_frame,
        &validated_identification,
        materialization.clone(),
        exchangeability.clone(),
    )
    .unwrap();

    assert_eq!(reference.reference_count, 2);
    assert_eq!(reference.comparison_count, 2);
    assert_eq!(reference.support_size, 6);
    assert_eq!(reference.realized_assignments.len(), 4);
    assert_eq!(reference.realized_assignments[0].group, BinaryComparisonGroup::Reference);
    assert_eq!(reference.realized_assignments[1].group, BinaryComparisonGroup::Reference);
    assert_eq!(reference.realized_assignments[2].group, BinaryComparisonGroup::Comparison);
    assert_eq!(reference.realized_assignments[3].group, BinaryComparisonGroup::Comparison);
    assert_eq!(
        reference.assignment_mechanism,
        identification
            .criteria
            .iter()
            .find(|item| item.criterion == CausalIdentificationCriterion::AssignmentMechanism)
            .unwrap()
            .clone()
    );

    let encoded = serde_json::to_vec(&reference).unwrap();
    let restored: CompleteRandomizationReferenceModel = serde_json::from_slice(&encoded).unwrap();
    let validated = ValidatedCompleteRandomizationReference::validate_current(
        &restored,
        &validated_frame,
        &validated_identification,
        materialization,
        exchangeability,
    )
    .unwrap();
    assert_eq!(
        validated.reference_digest(),
        reference.canonical_digest().unwrap()
    );
}

#[test]
fn reversing_realized_assignment_changes_reference_identity_not_support_size() {
    let (base_a, design_a, frame_a, identification_a) = randomized_identification_case(false, 221);
    let (vf_a, vi_a) = current_randomized_capabilities(
        &base_a,
        &design_a,
        &frame_a,
        &identification_a,
        false,
        221,
    );
    let (mat_a, exch_a) = d1_authorities(&vi_a, 22);
    let reference_a = CompleteRandomizationReferenceModel::capture(
        CompleteRandomizationReferenceId::new("assignment-direction").unwrap(),
        &vf_a,
        &vi_a,
        mat_a,
        exch_a,
    )
    .unwrap();

    let (base_b, design_b, frame_b, identification_b) = randomized_identification_case(true, 222);
    let (vf_b, vi_b) = current_randomized_capabilities(
        &base_b,
        &design_b,
        &frame_b,
        &identification_b,
        true,
        222,
    );
    let (mat_b, exch_b) = d1_authorities(&vi_b, 23);
    let reference_b = CompleteRandomizationReferenceModel::capture(
        CompleteRandomizationReferenceId::new("assignment-direction").unwrap(),
        &vf_b,
        &vi_b,
        mat_b,
        exch_b,
    )
    .unwrap();

    assert_eq!(reference_a.support_size, 6);
    assert_eq!(reference_b.support_size, 6);
    assert_ne!(reference_a.realized_assignments, reference_b.realized_assignments);
    assert_ne!(
        reference_a.canonical_digest().unwrap(),
        reference_b.canonical_digest().unwrap()
    );
}

#[test]
fn materialization_and_exchangeability_authority_are_identification_bound() {
    let (base, design, frame, identification_a) = randomized_identification_case(false, 223);
    let validated_frame = current_frame(&base, &design, &frame, false);
    let validated_a = ValidatedCausalSelectionIdentification::validate_current(
        &identification_a,
        &validated_frame,
        criterion_evidence(&validated_frame, randomized_criteria(), 223),
    )
    .unwrap();

    let evidence_b = criterion_evidence(&validated_frame, randomized_criteria(), 224);
    let identification_b = CausalSelectionIdentification::declare(
        CausalSelectionIdentificationId::new("d1-randomized-identification").unwrap(),
        &validated_frame,
        CausalSelectionTarget::BinaryViabilityRiskDifference,
        evidence_b.clone(),
    )
    .unwrap();
    let validated_b = ValidatedCausalSelectionIdentification::validate_current(
        &identification_b,
        &validated_frame,
        evidence_b,
    )
    .unwrap();

    let (wrong_materialization, wrong_exchangeability) = d1_authorities(&validated_b, 24);
    assert!(matches!(
        CompleteRandomizationReferenceModel::capture(
            CompleteRandomizationReferenceId::new("wrong-subject-materialization").unwrap(),
            &validated_frame,
            &validated_a,
            wrong_materialization,
            d1_authorities(&validated_a, 25).1,
        ),
        Err(CompleteRandomizationError::MaterializationSubjectMismatch)
    ));
    assert!(matches!(
        CompleteRandomizationReferenceModel::capture(
            CompleteRandomizationReferenceId::new("wrong-subject-exchangeability").unwrap(),
            &validated_frame,
            &validated_a,
            d1_authorities(&validated_a, 26).0,
            wrong_exchangeability,
        ),
        Err(CompleteRandomizationError::ExchangeabilitySubjectMismatch)
    ));
}

#[test]
fn materialization_qualification_only_changes_reference_identity() {
    let (base, design, frame, identification) = randomized_identification_case(false, 229);
    let (validated_frame, validated_identification) = current_randomized_capabilities(
        &base,
        &design,
        &frame,
        &identification,
        false,
        229,
    );
    let (materialization_a, exchangeability) = d1_authorities(&validated_identification, 32);
    let mut materialization_b = materialization_a.clone();
    materialization_b.qualification = AssignmentMechanismMaterializationQualificationRef::new(
        AssignmentMaterializationQualificationAuthorityId::new(
            "complete-randomization-materialization-qualification-alternate",
        )
        .unwrap(),
        1,
        AnalysisContentDigest::new([231; 32]),
    );

    let reference_a = CompleteRandomizationReferenceModel::capture(
        CompleteRandomizationReferenceId::new("qualification-only-drift").unwrap(),
        &validated_frame,
        &validated_identification,
        materialization_a,
        exchangeability.clone(),
    )
    .unwrap();
    let reference_b = CompleteRandomizationReferenceModel::capture(
        CompleteRandomizationReferenceId::new("qualification-only-drift").unwrap(),
        &validated_frame,
        &validated_identification,
        materialization_b,
        exchangeability,
    )
    .unwrap();

    assert_eq!(reference_a.realized_assignments, reference_b.realized_assignments);
    assert_eq!(reference_a.support_size, reference_b.support_size);
    assert_ne!(
        reference_a.canonical_digest().unwrap(),
        reference_b.canonical_digest().unwrap()
    );
}

#[test]
fn fresh_d1_authority_drift_stales_persisted_reference_replay() {
    let (base, design, frame, identification) = randomized_identification_case(false, 225);
    let (validated_frame, validated_identification) = current_randomized_capabilities(
        &base,
        &design,
        &frame,
        &identification,
        false,
        225,
    );
    let (materialization, exchangeability) = d1_authorities(&validated_identification, 27);
    let reference = CompleteRandomizationReferenceModel::capture(
        CompleteRandomizationReferenceId::new("authority-drift").unwrap(),
        &validated_frame,
        &validated_identification,
        materialization,
        exchangeability,
    )
    .unwrap();
    let (changed_materialization, changed_exchangeability) =
        d1_authorities(&validated_identification, 28);

    assert!(matches!(
        reference.validate_current(
            &validated_frame,
            &validated_identification,
            changed_materialization,
            changed_exchangeability,
        ),
        Err(CompleteRandomizationError::ReferenceReplayMismatch)
    ));
}

#[test]
fn controlled_simulation_identification_cannot_masquerade_as_physical_complete_randomization() {
    let (base, design, frame) =
        design_case(SelectionDesignClass::ControlledSimulationIntervention, false);
    let validated_frame = current_frame(&base, &design, &frame, false);
    let evidence = criterion_evidence(&validated_frame, simulation_criteria(), 226);
    let identification = CausalSelectionIdentification::declare(
        CausalSelectionIdentificationId::new("simulation-identification-d1").unwrap(),
        &validated_frame,
        CausalSelectionTarget::BinaryViabilityRiskDifference,
        evidence.clone(),
    )
    .unwrap();
    let validated_identification = ValidatedCausalSelectionIdentification::validate_current(
        &identification,
        &validated_frame,
        evidence,
    )
    .unwrap();
    let (materialization, exchangeability) = d1_authorities(&validated_identification, 29);

    assert!(matches!(
        CompleteRandomizationReferenceModel::capture(
            CompleteRandomizationReferenceId::new("simulation-not-randomization").unwrap(),
            &validated_frame,
            &validated_identification,
            materialization,
            exchangeability,
        ),
        Err(CompleteRandomizationError::UnsupportedDesignClass(
            SelectionDesignClass::ControlledSimulationIntervention
        ))
    ));
}

#[test]
fn altered_serialized_realized_assignment_fails_local_reference_validation() {
    let (base, design, frame, identification) = randomized_identification_case(false, 227);
    let (validated_frame, validated_identification) = current_randomized_capabilities(
        &base,
        &design,
        &frame,
        &identification,
        false,
        227,
    );
    let (materialization, exchangeability) = d1_authorities(&validated_identification, 30);
    let reference = CompleteRandomizationReferenceModel::capture(
        CompleteRandomizationReferenceId::new("tampered-assignment").unwrap(),
        &validated_frame,
        &validated_identification,
        materialization,
        exchangeability,
    )
    .unwrap();

    let mut value = serde_json::to_value(reference).unwrap();
    value["realized_assignments"][0]["group"] = serde_json::json!("Comparison");
    let altered: CompleteRandomizationReferenceModel = serde_json::from_value(value).unwrap();
    assert!(matches!(
        altered.canonical_digest(),
        Err(CompleteRandomizationError::AssignmentCountInvariant)
    ));
}

fn collect_randomization_keys(value: &serde_json::Value, keys: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, nested) in object {
                keys.push(key.clone());
                collect_randomization_keys(nested, keys);
            }
        }
        serde_json::Value::Array(values) => {
            for nested in values {
                collect_randomization_keys(nested, keys);
            }
        }
        _ => {}
    }
}

#[test]
fn randomization_reference_wire_shape_contains_mechanism_not_statistical_result() {
    let (base, design, frame, identification) = randomized_identification_case(false, 228);
    let (validated_frame, validated_identification) = current_randomized_capabilities(
        &base,
        &design,
        &frame,
        &identification,
        false,
        228,
    );
    let (materialization, exchangeability) = d1_authorities(&validated_identification, 31);
    let reference = CompleteRandomizationReferenceModel::capture(
        CompleteRandomizationReferenceId::new("wire-reference").unwrap(),
        &validated_frame,
        &validated_identification,
        materialization,
        exchangeability,
    )
    .unwrap();

    let value = serde_json::to_value(reference).unwrap();
    let mut keys = Vec::new();
    collect_randomization_keys(&value, &mut keys);
    for forbidden in [
        "p_value",
        "probability",
        "significance",
        "effect_estimate",
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
