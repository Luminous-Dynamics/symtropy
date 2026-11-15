use aeroweift_model_credibility::{
    Comparator, DatasetManifest, DatasetSplit, EvidenceModality, MetricEvidence, MetricGate,
    MetricKind, ModelScope, SampleUnit, ValidationDisposition, ValidationError, ValidationPackage,
    ValidationPolicy, evaluate,
};

fn digest(ch: char) -> String {
    std::iter::repeat_n(ch, 64).collect()
}

fn scope() -> ModelScope {
    ModelScope {
        model_artifact_sha256: digest('a'),
        aircraft_configuration_sha256: digest('b'),
        control_stack_sha256: digest('c'),
        sensor_stack_sha256: digest('d'),
        operating_domain_sha256: digest('e'),
        solver_configuration_sha256: digest('f'),
        horizontal_frame_id: "LOCAL-ENU-01".into(),
        vertical_datum_id: "SYNTHETIC-VD-01".into(),
        time_scale_id: "UTC-SYNTHETIC".into(),
        prediction_horizon_ticks: 30,
    }
}

fn manifest(
    digest_char: char,
    split: DatasetSplit,
    flights: &[&str],
    scenarios: &[&str],
    strata: &[&str],
) -> DatasetManifest {
    let model_scope = scope();
    DatasetManifest {
        manifest_sha256: digest(digest_char),
        split,
        operating_domain_sha256: model_scope.operating_domain_sha256,
        aircraft_configuration_sha256: model_scope.aircraft_configuration_sha256,
        control_stack_sha256: model_scope.control_stack_sha256,
        sensor_stack_sha256: model_scope.sensor_stack_sha256,
        horizontal_frame_id: model_scope.horizontal_frame_id,
        vertical_datum_id: model_scope.vertical_datum_id,
        time_scale_id: model_scope.time_scale_id,
        reference_measurement_profile_sha256: digest('8'),
        flight_group_ids: flights.iter().map(|value| (*value).into()).collect(),
        scenario_group_ids: scenarios.iter().map(|value| (*value).into()).collect(),
        strata: strata.iter().map(|value| (*value).into()).collect(),
    }
}

fn policy() -> ValidationPolicy {
    ValidationPolicy {
        policy_manifest_sha256: digest('a'),
        required_evaluator_build_sha256: digest('b'),
        expected_scope: scope(),
        expected_reference_measurement_profile_sha256: digest('8'),
        required_strata: vec!["crosswind-high".into(), "corridor-edge".into()],
        gates: vec![
            MetricGate {
                metric: MetricKind::PositionError,
                sample_unit: SampleUnit::TrajectorySegment,
                stratum_id: "crosswind-high".into(),
                modality: EvidenceModality::ControlledFlight,
                comparator: Comparator::AtMost,
                threshold: 5.0,
                minimum_independent_units: 12,
                confidence_level: 0.95,
                confidence_method_sha256: digest('9'),
            },
            MetricGate {
                metric: MetricKind::PredictionIntervalCoverage,
                sample_unit: SampleUnit::PredictionInterval,
                stratum_id: "crosswind-high".into(),
                modality: EvidenceModality::ControlledFlight,
                comparator: Comparator::AtLeast,
                threshold: 0.95,
                minimum_independent_units: 100,
                confidence_level: 0.95,
                confidence_method_sha256: digest('9'),
            },
            MetricGate {
                metric: MetricKind::OutOfDomainDetectionRecall,
                sample_unit: SampleUnit::Encounter,
                stratum_id: "corridor-edge".into(),
                modality: EvidenceModality::Synthetic,
                comparator: Comparator::AtLeast,
                threshold: 0.90,
                minimum_independent_units: 20,
                confidence_level: 0.95,
                confidence_method_sha256: digest('9'),
            },
        ],
    }
}

fn package() -> ValidationPackage {
    ValidationPackage {
        evaluator_build_sha256: digest('b'),
        policy_manifest_sha256: digest('a'),
        scope: scope(),
        calibration: manifest(
            '1',
            DatasetSplit::Calibration,
            &["cal-flight-1", "cal-flight-2"],
            &["cal-scenario-1", "cal-scenario-2"],
            &["crosswind-high"],
        ),
        development: Some(manifest(
            '2',
            DatasetSplit::Development,
            &["dev-flight-1", "dev-flight-2"],
            &["dev-scenario-1", "dev-scenario-2"],
            &["crosswind-high", "corridor-edge"],
        )),
        confirmatory: manifest(
            '3',
            DatasetSplit::Confirmatory,
            &["test-flight-1", "test-flight-2", "test-flight-3"],
            &["test-scenario-1", "test-scenario-2", "test-scenario-3"],
            &["crosswind-high", "corridor-edge"],
        ),
        metrics: vec![
            MetricEvidence {
                metric: MetricKind::PositionError,
                sample_unit: SampleUnit::TrajectorySegment,
                stratum_id: "crosswind-high".into(),
                modality: EvidenceModality::ControlledFlight,
                confirmatory_manifest_sha256: digest('3'),
                value: 4.2,
                lower_confidence_bound: 4.2,
                upper_confidence_bound: 4.2,
                confidence_level: 0.95,
                confidence_method_sha256: digest('9'),
                independent_unit_count: 14,
            },
            MetricEvidence {
                metric: MetricKind::PredictionIntervalCoverage,
                sample_unit: SampleUnit::PredictionInterval,
                stratum_id: "crosswind-high".into(),
                modality: EvidenceModality::ControlledFlight,
                confirmatory_manifest_sha256: digest('3'),
                value: 0.96,
                lower_confidence_bound: 0.96,
                upper_confidence_bound: 0.96,
                confidence_level: 0.95,
                confidence_method_sha256: digest('9'),
                independent_unit_count: 120,
            },
            MetricEvidence {
                metric: MetricKind::OutOfDomainDetectionRecall,
                sample_unit: SampleUnit::Encounter,
                stratum_id: "corridor-edge".into(),
                modality: EvidenceModality::Synthetic,
                confirmatory_manifest_sha256: digest('3'),
                value: 0.95,
                lower_confidence_bound: 0.95,
                upper_confidence_bound: 0.95,
                confidence_level: 0.95,
                confidence_method_sha256: digest('9'),
                independent_unit_count: 25,
            },
        ],
    }
}

#[test]
fn positive_control_is_admissible_for_review_only() {
    let result = evaluate(&package(), &policy()).unwrap();
    assert_eq!(
        result.disposition,
        ValidationDisposition::AdmissibleForDeclaredDomainReviewOnly
    );
    assert_eq!(result.admitted_metric_count, 3);
    assert_eq!(
        result.covered_strata,
        vec!["corridor-edge".to_string(), "crosswind-high".to_string()]
    );
}

#[test]
fn model_or_operating_domain_change_is_not_covered() {
    let mut candidate = package();
    candidate.scope.aircraft_configuration_sha256 = digest('9');
    let result = evaluate(&candidate, &policy()).unwrap();
    assert_eq!(result.disposition, ValidationDisposition::NotCovered);
    assert_eq!(result.admitted_metric_count, 0);
}

#[test]
fn calibration_and_confirmatory_flight_groups_must_be_disjoint() {
    let mut candidate = package();
    candidate.confirmatory.flight_group_ids[0] = candidate.calibration.flight_group_ids[0].clone();
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::DataLeakage)
    );
}

#[test]
fn calibration_and_confirmatory_scenario_families_must_be_disjoint() {
    let mut candidate = package();
    candidate.confirmatory.scenario_group_ids[0] =
        candidate.calibration.scenario_group_ids[0].clone();
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::DataLeakage)
    );
}

#[test]
fn development_data_must_not_overlap_confirmatory_flights() {
    let mut candidate = package();
    candidate.development.as_mut().unwrap().flight_group_ids[0] =
        candidate.confirmatory.flight_group_ids[0].clone();
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::DataLeakage)
    );
}

#[test]
fn metric_result_must_reference_exact_confirmatory_manifest() {
    let mut candidate = package();
    candidate.metrics[0].confirmatory_manifest_sha256 = digest('8');
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::WrongConfirmatoryManifest)
    );
}

#[test]
fn missing_confirmatory_stratum_is_not_pooled_away() {
    let mut candidate = package();
    candidate
        .confirmatory
        .strata
        .retain(|value| value != "corridor-edge");
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::MissingRequiredStratum)
    );
}

#[test]
fn insufficient_independent_units_fail_the_gate() {
    let mut candidate = package();
    candidate.metrics[0].independent_unit_count = 11;
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::InsufficientIndependentUnits)
    );
}

#[test]
fn pooled_average_cannot_hide_position_error_tail_breach() {
    let mut candidate = package();
    candidate.metrics[0].value = 5.1;
    candidate.metrics[0].lower_confidence_bound = 5.1;
    candidate.metrics[0].upper_confidence_bound = 5.1;
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::MetricOutsideGate)
    );
}

#[test]
fn uncertainty_interval_undercoverage_fails_even_with_good_rmse() {
    let mut candidate = package();
    candidate.metrics[1].value = 0.91;
    candidate.metrics[1].lower_confidence_bound = 0.91;
    candidate.metrics[1].upper_confidence_bound = 0.91;
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::MetricOutsideGate)
    );
}

#[test]
fn evidence_modality_is_part_of_the_gate_identity() {
    let mut candidate = package();
    candidate.metrics[0].modality = EvidenceModality::Synthetic;
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::UnexpectedOrDuplicateMetric)
    );
}

#[test]
fn every_preregistered_metric_gate_requires_a_result() {
    let mut candidate = package();
    candidate.metrics.pop();
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::MissingMetric)
    );
}

#[test]
fn duplicate_metric_records_are_rejected() {
    let mut candidate = package();
    candidate.metrics.push(candidate.metrics[0].clone());
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::UnexpectedOrDuplicateMetric)
    );
}

#[test]
fn dataset_manifest_must_name_the_expected_split() {
    let mut candidate = package();
    candidate.confirmatory.split = DatasetSplit::Development;
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::InvalidPackage)
    );
}

#[test]
fn dataset_manifest_domain_must_match_model_scope() {
    let mut candidate = package();
    candidate.confirmatory.operating_domain_sha256 = digest('8');
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::DatasetScopeMismatch)
    );
}

#[test]
fn non_finite_metric_values_are_rejected() {
    let mut candidate = package();
    candidate.metrics[0].value = f64::NAN;
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::InvalidPackage)
    );
}

#[test]
fn empty_required_strata_cannot_create_a_vacuous_pass() {
    let mut invalid_policy = policy();
    invalid_policy.required_strata.clear();
    assert_eq!(
        evaluate(&package(), &invalid_policy),
        Err(ValidationError::InvalidPolicy)
    );
}

#[test]
fn unrequested_metric_cannot_be_smuggled_into_a_package() {
    let mut candidate = package();
    candidate.metrics.push(MetricEvidence {
        metric: MetricKind::EnergyError,
        sample_unit: SampleUnit::Flight,
        stratum_id: "crosswind-high".into(),
        modality: EvidenceModality::Synthetic,
        confirmatory_manifest_sha256: digest('3'),
        value: 0.1,
        lower_confidence_bound: 0.1,
        upper_confidence_bound: 0.1,
        confidence_level: 0.95,
        confidence_method_sha256: digest('9'),
        independent_unit_count: 30,
    });
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::UnexpectedOrDuplicateMetric)
    );
}

#[test]
fn confirmatory_sensor_stack_must_match_exact_model_scope() {
    let mut candidate = package();
    candidate.confirmatory.sensor_stack_sha256 = digest('7');
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::DatasetScopeMismatch)
    );
}

#[test]
fn confirmatory_reference_measurement_profile_must_be_pinned() {
    let mut candidate = package();
    candidate.confirmatory.reference_measurement_profile_sha256 = digest('6');
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::DatasetScopeMismatch)
    );
}

#[test]
fn confirmatory_time_basis_must_match_model_scope() {
    let mut candidate = package();
    candidate.confirmatory.time_scale_id = "OTHER-TIME-SCALE".into();
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::DatasetScopeMismatch)
    );
}

#[test]
fn evaluator_build_must_match_frozen_validation_policy() {
    let mut candidate = package();
    candidate.evaluator_build_sha256 = digest('c');
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::EvaluatorOrPolicyMismatch)
    );
}

#[test]
fn validation_policy_manifest_must_match_exactly() {
    let mut candidate = package();
    candidate.policy_manifest_sha256 = digest('d');
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::EvaluatorOrPolicyMismatch)
    );
}

#[test]
fn metric_gate_cannot_claim_a_stratum_outside_required_domain() {
    let mut invalid_policy = policy();
    invalid_policy.gates.push(MetricGate {
        metric: MetricKind::EnergyError,
        sample_unit: SampleUnit::Flight,
        stratum_id: "unlisted-stratum".into(),
        modality: EvidenceModality::Synthetic,
        comparator: Comparator::AtMost,
        threshold: 1.0,
        minimum_independent_units: 1,
        confidence_level: 0.95,
        confidence_method_sha256: digest('9'),
    });
    assert_eq!(
        evaluate(&package(), &invalid_policy),
        Err(ValidationError::InvalidPolicy)
    );
}

#[test]
fn error_gate_uses_upper_confidence_bound_not_point_estimate() {
    let mut candidate = package();
    candidate.metrics[0].value = 4.9;
    candidate.metrics[0].lower_confidence_bound = 4.8;
    candidate.metrics[0].upper_confidence_bound = 5.2;
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::MetricOutsideGate)
    );
}

#[test]
fn coverage_gate_uses_lower_confidence_bound_not_point_estimate() {
    let mut candidate = package();
    candidate.metrics[1].value = 0.97;
    candidate.metrics[1].lower_confidence_bound = 0.93;
    candidate.metrics[1].upper_confidence_bound = 0.99;
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::MetricOutsideGate)
    );
}

#[test]
fn metric_interval_method_must_match_frozen_gate() {
    let mut candidate = package();
    candidate.metrics[0].confidence_method_sha256 = digest('7');
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::ConfidenceContractMismatch)
    );
}

#[test]
fn confidence_level_must_match_frozen_gate() {
    let mut candidate = package();
    candidate.metrics[0].confidence_level = 0.90;
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::ConfidenceContractMismatch)
    );
}

#[test]
fn inverted_confidence_interval_is_rejected() {
    let mut candidate = package();
    candidate.metrics[0].lower_confidence_bound = 5.1;
    candidate.metrics[0].upper_confidence_bound = 4.9;
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::InvalidPackage)
    );
}

#[test]
fn confidence_interval_cannot_exclude_its_reported_point_estimate() {
    let mut candidate = package();
    candidate.metrics[0].lower_confidence_bound = 4.0;
    candidate.metrics[0].upper_confidence_bound = 4.1;
    assert_eq!(
        evaluate(&candidate, &policy()),
        Err(ValidationError::InvalidPackage)
    );
}


#[test]
fn increasing_error_upper_bound_never_restores_a_failed_gate() {
    for step in 0..101 {
        let mut candidate = package();
        candidate.metrics[0].value = 4.0;
        candidate.metrics[0].lower_confidence_bound = 3.9;
        candidate.metrics[0].upper_confidence_bound = 5.0 + (step as f64 * 0.01);
        let result = evaluate(&candidate, &policy());
        if step == 0 {
            assert!(result.is_ok());
        } else {
            assert_eq!(result, Err(ValidationError::MetricOutsideGate));
        }
    }
}

#[test]
fn decreasing_coverage_lower_bound_never_restores_a_failed_gate() {
    for step in 0..101 {
        let mut candidate = package();
        candidate.metrics[1].value = 0.98;
        candidate.metrics[1].lower_confidence_bound = 0.95 - (step as f64 * 0.001);
        candidate.metrics[1].upper_confidence_bound = 0.99;
        let result = evaluate(&candidate, &policy());
        if step == 0 {
            assert!(result.is_ok());
        } else {
            assert_eq!(result, Err(ValidationError::MetricOutsideGate));
        }
    }
}

#[test]
fn adding_calibration_overlap_never_improves_validation_status() {
    for index in 0..100 {
        let mut candidate = package();
        let overlap = format!("cal-flight-{index}");
        candidate.calibration.flight_group_ids.push(overlap.clone());
        candidate.confirmatory.flight_group_ids.push(overlap);
        assert_eq!(
            evaluate(&candidate, &policy()),
            Err(ValidationError::DataLeakage)
        );
    }
}

#[test]
fn changing_any_model_identity_dimension_does_not_inherit_old_credit() {
    for dimension in 0..9 {
        let mut candidate = package();
        match dimension {
            0 => candidate.scope.model_artifact_sha256 = digest('8'),
            1 => candidate.scope.aircraft_configuration_sha256 = digest('8'),
            2 => candidate.scope.control_stack_sha256 = digest('8'),
            3 => candidate.scope.sensor_stack_sha256 = digest('8'),
            4 => candidate.scope.operating_domain_sha256 = digest('8'),
            5 => candidate.scope.solver_configuration_sha256 = digest('8'),
            6 => candidate.scope.horizontal_frame_id = "OTHER-FRAME".into(),
            7 => candidate.scope.vertical_datum_id = "OTHER-DATUM".into(),
            _ => candidate.scope.time_scale_id = "OTHER-TIME".into(),
        }
        assert_eq!(
            evaluate(&candidate, &policy()).unwrap().disposition,
            ValidationDisposition::NotCovered
        );
    }
}

#[test]
fn deleting_each_required_metric_never_improves_admissibility() {
    for index in 0..3 {
        let mut candidate = package();
        candidate.metrics.remove(index);
        assert_eq!(
            evaluate(&candidate, &policy()),
            Err(ValidationError::MissingMetric)
        );
    }
}

#[test]
fn confirmatory_scenario_overlap_is_rejected_for_many_mutations() {
    for index in 0..100 {
        let mut candidate = package();
        let overlap = format!("scenario-shared-{index}");
        candidate.calibration.scenario_group_ids.push(overlap.clone());
        candidate.confirmatory.scenario_group_ids.push(overlap);
        assert_eq!(
            evaluate(&candidate, &policy()),
            Err(ValidationError::DataLeakage)
        );
    }
}
