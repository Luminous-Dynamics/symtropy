include!("causal_viability_effect_v1.rs");

#[test]
fn changing_only_identification_qualification_changes_causal_effect_identity() {
    let (base, design, frame) =
        design_case(SelectionDesignClass::RandomizedInterventional, false);
    let validated_frame = current_frame(&base, &design, &frame, false);
    let association = execute_binary_viability_association(&validated_frame).unwrap();
    let validated_association =
        ValidatedConsequenceAssociation::validate_current(&association, &validated_frame).unwrap();

    let evidence_a = criterion_evidence(&validated_frame, randomized_criteria(), 220);
    let mut evidence_b = evidence_a.clone();
    for (index, item) in evidence_b.iter_mut().enumerate() {
        item.qualification = IdentificationEvidenceQualificationRef::new(
            IdentificationQualificationAuthorityId::new(format!(
                "alternate-causal-qualification-{index}"
            ))
            .unwrap(),
            2,
            AnalysisContentDigest::new([230u8.wrapping_add(index as u8); 32]),
        );
    }

    let identification_a = CausalSelectionIdentification::declare(
        CausalSelectionIdentificationId::new("qualification-only-identification").unwrap(),
        &validated_frame,
        CausalSelectionTarget::BinaryViabilityRiskDifference,
        evidence_a.clone(),
    )
    .unwrap();
    let identification_b = CausalSelectionIdentification::declare(
        CausalSelectionIdentificationId::new("qualification-only-identification").unwrap(),
        &validated_frame,
        CausalSelectionTarget::BinaryViabilityRiskDifference,
        evidence_b.clone(),
    )
    .unwrap();

    let validated_a = ValidatedCausalSelectionIdentification::validate_current(
        &identification_a,
        &validated_frame,
        evidence_a,
    )
    .unwrap();
    let validated_b = ValidatedCausalSelectionIdentification::validate_current(
        &identification_b,
        &validated_frame,
        evidence_b,
    )
    .unwrap();

    let effect_a = execute_causal_viability_risk_effect(
        CausalSelectionEffectId::new("qualification-only-effect").unwrap(),
        &validated_frame,
        &validated_association,
        &validated_a,
    )
    .unwrap();
    let effect_b = execute_causal_viability_risk_effect(
        CausalSelectionEffectId::new("qualification-only-effect").unwrap(),
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
