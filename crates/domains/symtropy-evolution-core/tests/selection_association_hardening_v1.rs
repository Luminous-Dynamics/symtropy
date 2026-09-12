// Reuse the complete SEL-07B2 fixture/corpus in this integration-test crate so
// the hardening tests exercise the exact same authority construction path.
include!("selection_association_v1.rs");

#[test]
fn altered_serialized_statistic_is_locally_invalid_before_current_replay() {
    let fixture = fixture(InsufficientSupportPolicy::FailClosed, None);
    let subjects = fixture.subjects();
    let inputs = predictor_inputs(&fixture, false, None);
    let (frame, consequences, evidence, design) =
        validated_frame(&fixture, &subjects, inputs.clone());
    let validated_frame = ValidatedSelectionAnalysisFrame::validate_current(
        &frame,
        &design,
        &evidence,
        &consequences,
        method("association-materialization", 44),
        PredictorRepresentation::BinaryComparisonGroup,
        inputs,
    )
    .unwrap();
    let estimate = execute_binary_viability_association(&validated_frame).unwrap();

    let mut value = serde_json::to_value(&estimate).unwrap();
    value["status"]["Estimated"]["comparison_minus_reference_risk_difference"]["numerator"] =
        serde_json::json!(2);
    let altered: symtropy_evolution_core::ConsequenceAssociationEstimate =
        serde_json::from_value(value).unwrap();

    assert!(matches!(
        altered.canonical_digest(),
        Err(AssociationExecutionError::ArithmeticInvariant)
    ));
}

#[test]
fn descriptive_only_fallback_is_not_silently_aliased_to_reporting() {
    let fixture = fixture(InsufficientSupportPolicy::DescriptiveOnlyFallback, None);
    let subjects = fixture.subjects();
    let inputs = predictor_inputs(&fixture, false, Some(1));
    let (frame, consequences, evidence, design) =
        validated_frame(&fixture, &subjects, inputs.clone());
    let validated_frame = ValidatedSelectionAnalysisFrame::validate_current(
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
        execute_binary_viability_association(&validated_frame),
        Err(AssociationExecutionError::UnsupportedInsufficientSupportPolicy)
    ));
}
