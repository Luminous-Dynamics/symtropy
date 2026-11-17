use aeroweift_experiment_design::{EstimateSampleUnit, UtilityEstimateReceipt};
use aeroweift_utility_calibration::{
    CalibrationDisposition, CalibrationError, CalibrationFailure, CalibrationPolicy,
    DatasetManifest, DatasetRole, NotCalibratableReason, SamplingDesign, SelectionDecision,
    UtilityOutcome, RealizedUtility, evaluate_calibration,
};

fn digest(ch: char) -> String {
    std::iter::repeat_n(ch, 64).collect()
}

fn unit_ids(prefix: &str, count: usize) -> Vec<String> {
    (0..count).map(|index| format!("{prefix}-{index:03}")).collect()
}

fn strata() -> Vec<String> {
    vec!["crosswind-high".into(), "corridor-edge".into()]
}

fn manifest(
    hash: char,
    role: DatasetRole,
    ids: Vec<String>,
) -> DatasetManifest {
    DatasetManifest {
        manifest_sha256: digest(hash),
        role,
        independent_unit_ids: ids,
        strata: strata(),
    }
}

fn evaluation_ids() -> Vec<String> {
    (0..2)
        .flat_map(|stratum| unit_ids(&format!("eval-{stratum}"), 100))
        .collect()
}

fn policy() -> CalibrationPolicy {
    CalibrationPolicy {
        campaign_sha256: digest('a'),
        model_scope_sha256: digest('b'),
        intended_use_profile_sha256: digest('c'),
        decision_question_id: "minimize-contingency-decision-loss".into(),
        decision_boundary_sha256: digest('d'),
        hypothesis_manifest_sha256: digest('e'),
        robust_uncertainty_set_sha256: digest('f'),
        adaptive_data_manifest_sha256: digest('8'),
        estimator_build_sha256: digest('9'),
        estimate_method_sha256: digest('a'),
        calibration_method_sha256: digest('b'),
        sample_unit: EstimateSampleUnit::Encounter,
        confidence_level: 0.95,
        minimum_lower_bound_coverage: 0.90,
        minimum_units_per_stratum: 90,
        minimum_effective_sample_size: 90.0,
        minimum_selection_propensity: 0.50,
        minimum_mean_lower_bound_sharpness: 0.10,
        maximum_mean_uninformative_upper_bound: 0.20,
        maximum_primary_mean_regret: 0.15,
        minimum_regret_improvement_vs_random: 0.10,
        primary_selector_id: "track-v".into(),
        random_baseline_selector_id: "random-feasible".into(),
        required_selector_ids: vec![
            "track-v".into(),
            "random-feasible".into(),
        ],
        resource_budget_sha256: digest('c'),
        sampling_design: SamplingDesign::ProspectiveRandomized,
        estimator_calibration_manifest: manifest(
            '1',
            DatasetRole::EstimatorCalibration,
            unit_ids("fit", 50),
        ),
        adaptive_development_manifest: manifest(
            '2',
            DatasetRole::AdaptiveDevelopment,
            unit_ids("adaptive", 50),
        ),
        evaluation_manifest: manifest(
            '3',
            DatasetRole::Challenge,
            evaluation_ids(),
        ),
        required_strata: strata(),
    }
}

fn forecast(candidate_id: &str, receipt_char: char) -> UtilityEstimateReceipt {
    UtilityEstimateReceipt {
        receipt_id: format!("utility-receipt-{candidate_id}"),
        payload_sha256: digest(receipt_char),
        issuer_id: "aeroweift-utility-estimator".into(),
        candidate_id: candidate_id.into(),
        campaign_sha256: digest('a'),
        model_scope_sha256: digest('b'),
        intended_use_profile_sha256: digest('c'),
        decision_question_id: "minimize-contingency-decision-loss".into(),
        design_sha256: digest(if candidate_id == "candidate-a" { '1' } else { '2' }),
        scenario_manifest_sha256: digest('d'),
        measurement_plan_sha256: digest('e'),
        decision_boundary_sha256: digest('d'),
        hypothesis_manifest_sha256: digest('e'),
        robust_uncertainty_set_sha256: digest('f'),
        adaptive_data_manifest_sha256: digest('8'),
        estimator_build_sha256: digest('9'),
        estimate_method_sha256: digest('a'),
        confidence_level: 0.95,
        sample_unit: EstimateSampleUnit::Encounter,
        independent_unit_count: 200,
        hypothesis_groups: vec![],
        hypothesis_contrasts: vec![],
        expected_uncertainty_reduction: 0.20,
        expected_model_discrimination: 0.20,
        worst_case_robust_reduction: 0.15,
        decision_relevance: 0.20,
        probability_uninformative: 0.10,
    }
}

fn realized(candidate_id: &str) -> RealizedUtility {
    RealizedUtility {
        uncertainty_reduction: 0.50,
        model_discrimination: 0.60,
        worst_case_robust_reduction: 0.40,
        decision_relevance: if candidate_id == "candidate-a" { 0.80 } else { 0.50 },
        uninformative: false,
    }
}

fn outcomes() -> Vec<UtilityOutcome> {
    let mut result = Vec::new();
    for (stratum_index, stratum) in strata().iter().enumerate() {
        for index in 0..100 {
            let unit = format!("eval-{stratum_index}-{index:03}");
            for candidate_id in ["candidate-a", "candidate-b"] {
                result.push(UtilityOutcome {
                    candidate_id: candidate_id.into(),
                    independent_unit_id: unit.clone(),
                    stratum_id: stratum.clone(),
                    dataset_manifest_sha256: digest('3'),
                    dataset_role: DatasetRole::Challenge,
                    sample_unit: EstimateSampleUnit::Encounter,
                    selection_propensity: Some(1.0),
                    realized: realized(candidate_id),
                });
            }
        }
    }
    result
}

fn decisions(primary_candidate: &str, random_candidate: &str) -> Vec<SelectionDecision> {
    let mut result = Vec::new();
    for unit in evaluation_ids() {
        result.push(SelectionDecision {
            selector_id: "track-v".into(),
            independent_unit_id: unit.clone(),
            candidate_id: primary_candidate.into(),
            resource_budget_sha256: digest('c'),
        });
        result.push(SelectionDecision {
            selector_id: "random-feasible".into(),
            independent_unit_id: unit,
            candidate_id: random_candidate.into(),
            resource_budget_sha256: digest('c'),
        });
    }
    result
}

fn forecasts() -> Vec<(String, UtilityEstimateReceipt)> {
    vec![
        ("candidate-a".into(), forecast("candidate-a", '4')),
        ("candidate-b".into(), forecast("candidate-b", '5')),
    ]
}

fn assess(
    forecasts: &[(String, UtilityEstimateReceipt)],
    observations: &[UtilityOutcome],
    decisions: &[SelectionDecision],
    policy: &CalibrationPolicy,
) -> Result<aeroweift_utility_calibration::CalibrationResult, CalibrationError> {
    evaluate_calibration(forecasts, observations, decisions, policy)
}

#[test]
fn positive_control_calibrates_receipt_bounds_and_paired_decision_value() {
    let result = assess(&forecasts(), &outcomes(), &decisions("candidate-a", "candidate-b"), &policy()).unwrap();
    assert_eq!(
        result.disposition,
        CalibrationDisposition::CalibratedForDeclaredCampaign
    );
    assert_eq!(result.candidate_strata.len(), 4);
    assert_eq!(result.selector_regrets.len(), 2);
    assert_eq!(result.primary_mean_regret, Some(0.0));
    assert_eq!(result.random_mean_regret, Some(0.30));
    assert!(result.primary_mean_regret_upper_bound.unwrap() <= 0.15);
    assert!(result.improvement_over_random_lower_bound.unwrap() >= 0.10);
    assert!(result.failures.is_empty());
    assert!(result.candidate_strata.iter().all(|stratum| {
        stratum.metric_coverage.len() == 4
            && stratum.metric_coverage.iter().all(|metric| metric.coverage.lower >= 0.90)
            && stratum.uninformative_rate.upper <= stratum.submitted_uninformative_upper_bound
    }));
}

#[test]
fn duplicate_estimate_receipt_id_cannot_be_replayed_across_candidates() {
    let mut fs = forecasts();
    fs[1].1.receipt_id = fs[0].1.receipt_id.clone();
    assert_eq!(
        assess(&fs, &outcomes(), &decisions("candidate-a", "candidate-b"), &policy()),
        Err(CalibrationError::DuplicateUtilityEstimateReceiptId)
    );
}

#[test]
fn duplicate_estimate_payload_cannot_be_replayed_across_candidates() {
    let mut fs = forecasts();
    fs[1].1.payload_sha256 = fs[0].1.payload_sha256.clone();
    assert_eq!(
        assess(&fs, &outcomes(), &decisions("candidate-a", "candidate-b"), &policy()),
        Err(CalibrationError::DuplicateUtilityEstimateReceiptId)
    );
}

#[test]
fn calibration_and_evaluation_unit_overlap_is_rejected() {
    let mut frozen = policy();
    frozen.estimator_calibration_manifest.independent_unit_ids[0] =
        frozen.evaluation_manifest.independent_unit_ids[0].clone();
    assert_eq!(
        assess(&forecasts(), &outcomes(), &decisions("candidate-a", "candidate-b"), &frozen),
        Err(CalibrationError::DatasetLeakage)
    );
}

#[test]
fn adaptive_data_receipt_must_match_the_frozen_development_manifest() {
    let mut fs = forecasts();
    fs[0].1.adaptive_data_manifest_sha256 = digest('7');
    assert_eq!(
        assess(&fs, &outcomes(), &decisions("candidate-a", "candidate-b"), &policy()),
        Err(CalibrationError::InvalidForecastReceipt)
    );
}

#[test]
fn challenge_outcomes_cannot_come_from_adaptive_development() {
    let mut records = outcomes();
    records[0].dataset_role = DatasetRole::AdaptiveDevelopment;
    assert_eq!(
        assess(&forecasts(), &records, &decisions("candidate-a", "candidate-b"), &policy()),
        Err(CalibrationError::InvalidOutcome)
    );
}

#[test]
fn challenge_manifest_splice_is_rejected() {
    let mut records = outcomes();
    records[0].dataset_manifest_sha256 = digest('7');
    assert_eq!(
        assess(&forecasts(), &records, &decisions("candidate-a", "candidate-b"), &policy()),
        Err(CalibrationError::InvalidOutcome)
    );
}

#[test]
fn missing_inclusion_propensity_is_not_treated_as_random_sampling() {
    let mut records = outcomes();
    records[0].selection_propensity = None;
    let result = assess(&forecasts(), &records, &decisions("candidate-a", "candidate-b"), &policy()).unwrap();
    assert_eq!(result.disposition, CalibrationDisposition::NotCalibratable);
    assert!(result.not_calibratable_reasons.contains(&NotCalibratableReason::MissingSelectionPropensity));
}

#[test]
fn unknown_adaptive_propensity_forces_abstention() {
    let mut frozen = policy();
    frozen.sampling_design = SamplingDesign::AdaptiveWithUnknownPropensity;
    let result = assess(&forecasts(), &outcomes(), &decisions("candidate-a", "candidate-b"), &frozen).unwrap();
    assert_eq!(result.disposition, CalibrationDisposition::NotCalibratable);
    assert!(result.not_calibratable_reasons.contains(&NotCalibratableReason::UnknownSelectionPropensity));
}

#[test]
fn propensity_below_positivity_floor_forces_abstention() {
    let mut records = outcomes();
    records[0].selection_propensity = Some(0.01);
    let result = assess(&forecasts(), &records, &decisions("candidate-a", "candidate-b"), &policy()).unwrap();
    assert_eq!(result.disposition, CalibrationDisposition::NotCalibratable);
    assert!(result.not_calibratable_reasons.contains(&NotCalibratableReason::InsufficientSelectionOverlap));
}

#[test]
fn incomplete_candidate_pairing_forces_abstention() {
    let mut records = outcomes();
    records.retain(|record| !(record.candidate_id == "candidate-b" && record.independent_unit_id == "eval-0-000"));
    let result = assess(&forecasts(), &records, &decisions("candidate-a", "candidate-b"), &policy()).unwrap();
    assert_eq!(result.disposition, CalibrationDisposition::NotCalibratable);
    assert!(result.not_calibratable_reasons.contains(&NotCalibratableReason::IncompletePairedCampaign));
}

#[test]
fn undercovered_utility_lower_bound_fails_calibration() {
    let mut records = outcomes();
    for record in &mut records {
        if record.candidate_id == "candidate-a"
            && record.stratum_id == "crosswind-high"
            && record.independent_group_id.ends_with("0")
        {
            record.realized.uncertainty_reduction = 0.05;
        }
    }
    let result = assess(&forecasts(), &records, &decisions("candidate-a", "candidate-b"), &policy()).unwrap();
    assert_eq!(result.disposition, CalibrationDisposition::AcceptanceCriteriaFailed);
    assert!(result.failures.contains(&CalibrationFailure::LowerBoundUndercoverage));
}

#[test]
fn uninformative_rate_above_forecast_probability_bound_fails() {
    let mut records = outcomes();
    for record in &mut records {
        if record.candidate_id == "candidate-a"
            && record.stratum_id == "corridor-edge"
            && record.independent_group_id.ends_with("0")
        {
            record.realized.uninformative = true;
        }
    }
    let result = assess(&forecasts(), &records, &decisions("candidate-a", "candidate-b"), &policy()).unwrap();
    assert_eq!(result.disposition, CalibrationDisposition::AcceptanceCriteriaFailed);
    assert!(result.failures.contains(&CalibrationFailure::UninformativeUpperBoundUndercoverage));
}

#[test]
fn high_mean_estimate_bound_is_still_checked_for_sharpness() {
    let mut fs = forecasts();
    fs[0].1.expected_uncertainty_reduction = 0.01;
    fs[0].1.expected_model_discrimination = 0.01;
    fs[0].1.worst_case_robust_reduction = 0.01;
    fs[0].1.decision_relevance = 0.01;
    let result = assess(&fs, &outcomes(), &decisions("candidate-a", "candidate-b"), &policy()).unwrap();
    assert_eq!(result.disposition, CalibrationDisposition::AcceptanceCriteriaFailed);
    assert!(result.failures.contains(&CalibrationFailure::BoundsTooVacuous));
}

#[test]
fn weak_regret_improvement_fails_the_paired_confidence_bound_gate() {
    let mut frozen = policy();
    frozen.minimum_regret_improvement_vs_random = 0.20;
    let result = assess(&forecasts(), &outcomes(), &decisions("candidate-a", "candidate-b"), &frozen).unwrap();
    assert_eq!(result.disposition, CalibrationDisposition::AcceptanceCriteriaFailed);
    assert!(result.improvement_over_random.unwrap() > 0.20);
    assert!(result.improvement_over_random_lower_bound.unwrap() < 0.20);
    assert!(result.failures.contains(&CalibrationFailure::InsufficientImprovementOverRandom));
}

#[test]
fn a_primary_selector_that_loses_to_random_fails_regret_gates() {
    let result = assess(&forecasts(), &outcomes(), &decisions("candidate-b", "candidate-a"), &policy()).unwrap();
    assert_eq!(result.disposition, CalibrationDisposition::AcceptanceCriteriaFailed);
    assert!(result.primary_mean_regret_upper_bound.unwrap() > 0.15);
    assert!(result.improvement_over_random_lower_bound.unwrap() < 0.10);
    assert!(result.failures.contains(&CalibrationFailure::PrimaryDecisionRegretExceeded));
    assert!(result.failures.contains(&CalibrationFailure::InsufficientImprovementOverRandom));
}

#[test]
fn decision_regret_resource_budget_must_match_across_selectors() {
    let mut decisions = decisions("candidate-a", "candidate-b");
    decisions[0].resource_budget_sha256 = digest('9');
    assert_eq!(
        assess(&forecasts(), &outcomes(), &decisions, &policy()),
        Err(CalibrationError::InvalidSelectionDecision)
    );
}

#[test]
fn challenge_input_permutation_preserves_report() {
    let records = outcomes();
    let expected = assess(&forecasts(), &records, &decisions("candidate-a", "candidate-b"), &policy()).unwrap();
    let mut permuted = records;
    permuted.reverse();
    assert_eq!(
        assess(&forecasts(), &permuted, &decisions("candidate-a", "candidate-b"), &policy()).unwrap(),
        expected
    );
}

#[test]
fn duplicate_candidate_outcome_unit_is_rejected() {
    let mut records = outcomes();
    records.push(records[0].clone());
    assert_eq!(
        assess(&forecasts(), &records, &decisions("candidate-a", "candidate-b"), &policy()),
        Err(CalibrationError::DuplicateIndependentOutcome)
    );
}

#[test]
fn decision_coverage_requires_every_selector_for_every_unit() {
    let mut selections = decisions("candidate-a", "candidate-b");
    selections.pop();
    let result = assess(&forecasts(), &outcomes(), &selections, &policy()).unwrap();
    assert_eq!(result.disposition, CalibrationDisposition::NotCalibratable);
    assert!(result.not_calibratable_reasons.contains(&NotCalibratableReason::MissingSelectorDecision));
}
