use symtropy_evolution_core::{
    CausalIdentificationCriterion, CausalIdentificationError, CausalIdentificationTier,
    CausalSelectionIdentification, CausalSelectionIdentificationId, CausalSelectionTarget,
    ComparisonAuthorityId, ComparisonAuthorityRef, IdentificationEvidenceAuthorityId,
    IdentificationEvidenceRef, ValidatedCausalSelectionIdentification,
};

include!("selection_association_v1.rs");

fn comparison_authority() -> ComparisonAuthorityRef {
    ComparisonAuthorityRef::new(
        ComparisonAuthorityId::new("causal-identification-comparison").unwrap(),
        1,
        AnalysisContentDigest::new([101; 32]),
    )
}

fn design_case(
    class: SelectionDesignClass,
    reverse_groups: bool,
) -> (Fixture, SelectionComparisonDesign, ExplicitSelectionAnalysisFrame) {
    let base = fixture(InsufficientSupportPolicy::FailClosed, None);

    let (design, frame) = {
        let subjects = base.subjects();
        let consequences = ValidatedConsequenceLedger::validate_current(
            &base.consequences,
            &base.population,
            &base.schema,
            &base.map,
            &base.census,
            &subjects,
            &base.context,
        )
        .unwrap();
        let evidence = ValidatedSelectionEvidenceLedger::validate_current(
            &base.evidence,
            &consequences,
            &base.schema,
            &base.map,
            &base.census,
            &subjects,
        )
        .unwrap();
        let design = SelectionComparisonDesign::declare(
            SelectionComparisonDesignId::new(format!("causal-identification-{class:?}")).unwrap(),
            &evidence,
            base.design.predictor.clone(),
            SelectionEstimand::ViabilityWindowRiskContrast,
            class,
            Some(comparison_authority()),
            base.design.included_individuals.clone(),
            base.design.exclusions.clone(),
            base.design.eligibility_rule.clone(),
            base.design.phenotype_support.clone(),
            base.design.exposure_support.clone(),
            base.design.phenotype_protocols.clone(),
            base.design.exposure_protocols.clone(),
            ConfoundingPolicy::DesignBasedNoAdjustmentDeclared,
            DeathBeforeEndpointPolicy::NotApplicable,
            CompetingRiskPolicy::NoneDeclared,
            base.design.uncertainty.clone(),
        )
        .unwrap();
        let validated_design =
            ValidatedSelectionComparisonDesign::validate_current(&design, &evidence).unwrap();
        let frame = ExplicitSelectionAnalysisFrame::capture(
            &validated_design,
            &evidence,
            &consequences,
            method("causal-identification-materialization", 102),
            PredictorRepresentation::BinaryComparisonGroup,
            predictor_inputs(&base, reverse_groups, None),
        )
        .unwrap();
        (design, frame)
    };

    (base, design, frame)
}

fn current_frame<'a>(
    base: &'a Fixture,
    design: &'a SelectionComparisonDesign,
    frame: &'a ExplicitSelectionAnalysisFrame,
    reverse_groups: bool,
) -> ValidatedSelectionAnalysisFrame<'a> {
    let subjects = base.subjects();
    let consequences = ValidatedConsequenceLedger::validate_current(
        &base.consequences,
        &base.population,
        &base.schema,
        &base.map,
        &base.census,
        &subjects,
        &base.context,
    )
    .unwrap();
    let evidence = ValidatedSelectionEvidenceLedger::validate_current(
        &base.evidence,
        &consequences,
        &base.schema,
        &base.map,
        &base.census,
        &subjects,
    )
    .unwrap();
    let validated_design =
        ValidatedSelectionComparisonDesign::validate_current(design, &evidence).unwrap();
    ValidatedSelectionAnalysisFrame::validate_current(
        frame,
        &validated_design,
        &evidence,
        &consequences,
        method("causal-identification-materialization", 102),
        PredictorRepresentation::BinaryComparisonGroup,
        predictor_inputs(base, reverse_groups, None),
    )
    .unwrap()
}

fn randomized_criteria() -> Vec<CausalIdentificationCriterion> {
    vec![
        CausalIdentificationCriterion::AssignmentMechanism,
        CausalIdentificationCriterion::AllocationIntegrity,
        CausalIdentificationCriterion::TemporalPrecedence,
        CausalIdentificationCriterion::InterventionFidelity,
        CausalIdentificationCriterion::InterferencePolicy,
        CausalIdentificationCriterion::OutcomeAscertainment,
        CausalIdentificationCriterion::AttritionMissingness,
        CausalIdentificationCriterion::AnalysisPopulationIntegrity,
        CausalIdentificationCriterion::ProtocolFreezeProvenance,
    ]
}

fn simulation_criteria() -> Vec<CausalIdentificationCriterion> {
    let mut values = randomized_criteria();
    values.extend([
        CausalIdentificationCriterion::SimulationReplayIdentity,
        CausalIdentificationCriterion::InterventionIsolation,
        CausalIdentificationCriterion::ScenarioContext,
        CausalIdentificationCriterion::SimulationStochasticityPolicy,
    ]);
    values
}

fn criterion_evidence(
    frame: &ValidatedSelectionAnalysisFrame<'_>,
    criteria: impl IntoIterator<Item = CausalIdentificationCriterion>,
    byte_offset: u8,
) -> Vec<IdentificationEvidenceRef> {
    criteria
        .into_iter()
        .enumerate()
        .map(|(index, criterion)| {
            IdentificationEvidenceRef::new(
                criterion,
                IdentificationEvidenceAuthorityId::new(format!(
                    "causal-criterion-authority-{index}"
                ))
                .unwrap(),
                1,
                AnalysisContentDigest::new([byte_offset.wrapping_add(index as u8); 32]),
                frame.design_digest(),
                frame.frame_digest(),
            )
        })
        .collect()
}

#[test]
fn randomized_intervention_requires_complete_criterion_set_and_replays_fresh_evidence() {
    let (base, design, frame) = design_case(SelectionDesignClass::RandomizedInterventional, false);
    let validated_frame = current_frame(&base, &design, &frame, false);
    let evidence = criterion_evidence(&validated_frame, randomized_criteria(), 110);

    let identification = CausalSelectionIdentification::declare(
        CausalSelectionIdentificationId::new("randomized-identification-v1").unwrap(),
        &validated_frame,
        CausalSelectionTarget::BinaryViabilityRiskDifference,
        evidence.clone(),
    )
    .unwrap();
    assert_eq!(identification.tier, CausalIdentificationTier::InterventionIdentified);
    assert_eq!(identification.criteria.len(), 9);

    let encoded = serde_json::to_vec(&identification).unwrap();
    let restored: CausalSelectionIdentification = serde_json::from_slice(&encoded).unwrap();
    let validated = ValidatedCausalSelectionIdentification::validate_current(
        &restored,
        &validated_frame,
        evidence,
    )
    .unwrap();
    assert_eq!(validated.frame_digest(), validated_frame.frame_digest());
    assert_eq!(validated.design_digest(), validated_frame.design_digest());
    assert_eq!(
        validated.identification_digest(),
        identification.canonical_digest().unwrap()
    );
}

#[test]
fn descriptive_and_matched_designs_cannot_mint_intervention_identification() {
    let descriptive = fixture(InsufficientSupportPolicy::FailClosed, None);
    let descriptive_subjects = descriptive.subjects();
    let inputs = predictor_inputs(&descriptive, false, None);
    let (frame, consequences, evidence, design) =
        validated_frame(&descriptive, &descriptive_subjects, inputs.clone());
    let validated_descriptive = ValidatedSelectionAnalysisFrame::validate_current(
        &frame,
        &design,
        &evidence,
        &consequences,
        method("association-materialization", 44),
        PredictorRepresentation::BinaryComparisonGroup,
        inputs,
    )
    .unwrap();
    assert!(matches!(
        CausalSelectionIdentification::declare(
            CausalSelectionIdentificationId::new("descriptive-causal-attempt").unwrap(),
            &validated_descriptive,
            CausalSelectionTarget::BinaryViabilityRiskDifference,
            [],
        ),
        Err(CausalIdentificationError::UnsupportedDesignClass(
            SelectionDesignClass::DescriptiveAssociation
        ))
    ));

    let (base, matched_design, matched_frame) =
        design_case(SelectionDesignClass::MatchedStratifiedObservational, false);
    let matched = current_frame(&base, &matched_design, &matched_frame, false);
    assert!(matches!(
        CausalSelectionIdentification::declare(
            CausalSelectionIdentificationId::new("matched-causal-attempt").unwrap(),
            &matched,
            CausalSelectionTarget::BinaryViabilityRiskDifference,
            [],
        ),
        Err(CausalIdentificationError::UnsupportedDesignClass(
            SelectionDesignClass::MatchedStratifiedObservational
        ))
    ));
}

#[test]
fn missing_duplicate_and_cross_subject_criteria_fail_closed() {
    let (base, design, frame) = design_case(SelectionDesignClass::RandomizedInterventional, false);
    let validated_frame = current_frame(&base, &design, &frame, false);

    let mut missing = criterion_evidence(&validated_frame, randomized_criteria(), 120);
    missing.retain(|item| item.criterion != CausalIdentificationCriterion::TemporalPrecedence);
    assert!(matches!(
        CausalSelectionIdentification::declare(
            CausalSelectionIdentificationId::new("missing-criterion").unwrap(),
            &validated_frame,
            CausalSelectionTarget::BinaryViabilityRiskDifference,
            missing,
        ),
        Err(CausalIdentificationError::MissingCriterion(
            CausalIdentificationCriterion::TemporalPrecedence
        ))
    ));

    let mut duplicate = criterion_evidence(&validated_frame, randomized_criteria(), 121);
    duplicate.push(duplicate[0].clone());
    assert!(matches!(
        CausalSelectionIdentification::declare(
            CausalSelectionIdentificationId::new("duplicate-criterion").unwrap(),
            &validated_frame,
            CausalSelectionTarget::BinaryViabilityRiskDifference,
            duplicate,
        ),
        Err(CausalIdentificationError::DuplicateCriterion(
            CausalIdentificationCriterion::AssignmentMechanism
        ))
    ));

    let (other_base, other_design, other_frame) =
        design_case(SelectionDesignClass::RandomizedInterventional, true);
    let other_validated = current_frame(&other_base, &other_design, &other_frame, true);
    let wrong_subject = criterion_evidence(&other_validated, randomized_criteria(), 122);
    assert!(matches!(
        CausalSelectionIdentification::declare(
            CausalSelectionIdentificationId::new("wrong-subject").unwrap(),
            &validated_frame,
            CausalSelectionTarget::BinaryViabilityRiskDifference,
            wrong_subject,
        ),
        Err(CausalIdentificationError::CriterionSubjectMismatch(_))
    ));
}

#[test]
fn controlled_simulation_requires_simulation_specific_identification_evidence() {
    let (base, design, frame) =
        design_case(SelectionDesignClass::ControlledSimulationIntervention, false);
    let validated_frame = current_frame(&base, &design, &frame, false);

    let common_only = criterion_evidence(&validated_frame, randomized_criteria(), 130);
    assert!(matches!(
        CausalSelectionIdentification::declare(
            CausalSelectionIdentificationId::new("simulation-common-only").unwrap(),
            &validated_frame,
            CausalSelectionTarget::BinaryViabilityRiskDifference,
            common_only,
        ),
        Err(CausalIdentificationError::MissingCriterion(
            CausalIdentificationCriterion::SimulationReplayIdentity
        ))
    ));

    let complete = criterion_evidence(&validated_frame, simulation_criteria(), 131);
    let identification = CausalSelectionIdentification::declare(
        CausalSelectionIdentificationId::new("simulation-complete").unwrap(),
        &validated_frame,
        CausalSelectionTarget::BinaryViabilityRiskDifference,
        complete,
    )
    .unwrap();
    assert_eq!(identification.criteria.len(), 13);
}

#[test]
fn changed_fresh_criterion_evidence_stales_restored_identification() {
    let (base, design, frame) = design_case(SelectionDesignClass::RandomizedInterventional, false);
    let validated_frame = current_frame(&base, &design, &frame, false);
    let evidence = criterion_evidence(&validated_frame, randomized_criteria(), 140);
    let identification = CausalSelectionIdentification::declare(
        CausalSelectionIdentificationId::new("criterion-drift").unwrap(),
        &validated_frame,
        CausalSelectionTarget::BinaryViabilityRiskDifference,
        evidence,
    )
    .unwrap();

    let encoded = serde_json::to_vec(&identification).unwrap();
    let restored: CausalSelectionIdentification = serde_json::from_slice(&encoded).unwrap();
    let changed = criterion_evidence(&validated_frame, randomized_criteria(), 141);
    assert!(matches!(
        restored.validate_current(&validated_frame, changed),
        Err(CausalIdentificationError::IdentificationReplayMismatch)
    ));
}

fn collect_keys(value: &serde_json::Value, keys: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(object) => {
            for (key, nested) in object {
                keys.push(key.clone());
                collect_keys(nested, keys);
            }
        }
        serde_json::Value::Array(values) => {
            for nested in values {
                collect_keys(nested, keys);
            }
        }
        _ => {}
    }
}

#[test]
fn identification_wire_shape_contains_no_effect_fitness_or_adaptation_claim() {
    let (base, design, frame) = design_case(SelectionDesignClass::RandomizedInterventional, false);
    let validated_frame = current_frame(&base, &design, &frame, false);
    let evidence = criterion_evidence(&validated_frame, randomized_criteria(), 150);
    let identification = CausalSelectionIdentification::declare(
        CausalSelectionIdentificationId::new("wire-boundary").unwrap(),
        &validated_frame,
        CausalSelectionTarget::BinaryViabilityRiskDifference,
        evidence,
    )
    .unwrap();

    let value = serde_json::to_value(identification).unwrap();
    let mut keys = Vec::new();
    collect_keys(&value, &mut keys);
    for forbidden in [
        "effect",
        "effect_estimate",
        "selection_coefficient",
        "fitness",
        "beneficial",
        "deleterious",
        "adaptation",
        "p_value",
        "speciation",
    ] {
        assert!(!keys.iter().any(|key| key == forbidden));
    }
}
