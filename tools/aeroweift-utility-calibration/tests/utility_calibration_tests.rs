use aeroweift_utility_calibration::{
    CalibrationDisposition, CalibrationError, CalibrationPolicy, ChallengeObservation,
    EvidencePartition, IndependentUnit, PredictedUtility, RealizedUtility, UtilityClaim,
    UtilityDimension, evaluate_utility_claim,
};

fn digest(ch: char) -> String {
    std::iter::repeat_n(ch, 64).collect()
}

fn claim() -> UtilityClaim {
    UtilityClaim {
        candidate_id: "aeroweift-candidate-001".into(),
        utility_receipt_id: "utility-receipt-001".into(),
        utility_receipt_payload_sha256: digest('a'),
        campaign_sha256: digest('b'),
        model_scope_sha256: digest('c'),
        intended_use_profile_sha256: digest('d'),
        decision_question_id: "reachability-margin-q1".into(),
        decision_boundary_sha256: digest('e'),
        hypothesis_manifest_sha256: digest('f'),
        robust_uncertainty_set_sha256: digest('1'),
        adaptive_data_manifest_sha256: digest('2'),
        estimator_build_sha256: digest('3'),
        estimate_method_sha256: digest('4'),
        estimate_confidence_level: 0.95,
        independent_unit: IndependentUnit::Encounter,
        estimator_independent_sample_count: 200,
        predicted: PredictedUtility {
            uncertainty_reduction_lower: 0.50,
            model_discrimination_lower: 0.40,
            worst_case_robust_reduction_lower: 0.30,
            decision_relevance_lower: 0.60,
            uninformative_probability_upper: 0.05,
        },
    }
}

fn policy() -> CalibrationPolicy {
    CalibrationPolicy {
        policy_sha256: digest('5'),
        expected_campaign_sha256: digest('b'),
        expected_model_scope_sha256: digest('c'),
        expected_intended_use_profile_sha256: digest('d'),
        expected_decision_boundary_sha256: digest('e'),
        expected_hypothesis_manifest_sha256: digest('f'),
        expected_robust_uncertainty_set_sha256: digest('1'),
        expected_estimator_build_sha256: digest('3'),
        expected_estimate_method_sha256: digest('4'),
        challenge_manifest_sha256: digest('6'),
        evaluator_build_sha256: digest('7'),
        interval_method_sha256: digest('8'),
        required_independent_unit: IndependentUnit::Encounter,
        required_dimensions: vec![
            UtilityDimension::UncertaintyReduction,
            UtilityDimension::ModelDiscrimination,
            UtilityDimension::WorstCaseRobustReduction,
            UtilityDimension::DecisionRelevance,
        ],
        required_strata: vec!["crosswind-high".into(), "corridor-edge".into()],
        minimum_groups_per_stratum: 100,
        one_sided_confidence_level: 0.95,
        minimum_coverage_lower_bound: 0.95,
    }
}

fn observation(index: usize, stratum: &str) -> ChallengeObservation {
    ChallengeObservation {
        observation_id: format!("outcome-{stratum}-{index:03}"),
        candidate_id: claim().candidate_id,
        utility_receipt_payload_sha256: digest('a'),
        measurement_evidence_receipt_sha256: digest('9'),
        challenge_manifest_sha256: digest('6'),
        partition: EvidencePartition::ChallengeSealed,
        independent_group_id: format!("independent-encounter-{stratum}-{index:03}"),
        stratum_id: stratum.into(),
        realized: RealizedUtility {
            uncertainty_reduction: 0.62,
            model_discrimination: 0.54,
            worst_case_robust_reduction: 0.43,
            decision_relevance: 0.72,
            uninformative: false,
        },
    }
}

fn observations() -> Vec<ChallengeObservation> {
    let mut records = Vec::new();
    for stratum in ["crosswind-high", "corridor-edge"] {
        for index in 0..100 {
            records.push(observation(index, stratum));
        }
    }
    records
}

fn assess(
    claim: &UtilityClaim,
    observations: &[ChallengeObservation],
    policy: &CalibrationPolicy,
) -> Result<aeroweift_utility_calibration::CalibrationReport, CalibrationError> {
    evaluate_utility_claim(claim, observations, policy)
}

#[test]
fn positive_control_passes_each_required_stratum_without_pooled_substitution() {
    let result = assess(&claim(), &observations(), &policy()).unwrap();
    assert_eq!(
        result.disposition,
        CalibrationDisposition::CalibratedForDeclaredChallengeSetOnly
    );
    assert_eq!(result.independent_group_count, 200);
    assert_eq!(result.strata.len(), 2);
    assert!(result.strata.iter().all(|stratum| stratum.passed));
    assert!(
        result
            .strata
            .iter()
            .flat_map(|stratum| &stratum.bound_coverage)
            .all(|coverage| coverage.wilson_lower_bound >= 0.95)
    );
    assert!(
        result
            .strata
            .iter()
            .all(|stratum| stratum.uninformative_rate.wilson_upper_bound <= 0.05)
    );
}

#[test]
fn utility_lower_bound_undercoverage_fails_even_when_other_metrics_pass() {
    let mut records = observations();
    for record in records
        .iter_mut()
        .filter(|record| record.stratum_id == "corridor-edge")
        .take(15)
    {
        record.realized.uncertainty_reduction = 0.20;
    }
    let result = assess(&claim(), &records, &policy()).unwrap();
    assert_eq!(result.disposition, CalibrationDisposition::NotCalibrated);
    let corridor = result
        .strata
        .iter()
        .find(|stratum| stratum.stratum_id == "corridor-edge")
        .unwrap();
    assert!(
        !corridor
            .bound_coverage
            .iter()
            .find(|coverage| coverage.dimension == UtilityDimension::UncertaintyReduction)
            .unwrap()
            .passed
    );
}

#[test]
fn high_uninformative_rate_violates_claimed_upper_bound() {
    let mut records = observations();
    for record in records
        .iter_mut()
        .filter(|record| record.stratum_id == "crosswind-high")
        .take(12)
    {
        record.realized.uninformative = true;
    }
    let result = assess(&claim(), &records, &policy()).unwrap();
    assert_eq!(result.disposition, CalibrationDisposition::NotCalibrated);
    let crosswind = result
        .strata
        .iter()
        .find(|stratum| stratum.stratum_id == "crosswind-high")
        .unwrap();
    assert!(!crosswind.uninformative_rate.passed);
    assert!(crosswind.uninformative_rate.wilson_upper_bound > 0.05);
}

#[test]
fn one_weak_stratum_cannot_be_hidden_by_a_strong_pooled_average() {
    let mut records = observations();
    for record in records
        .iter_mut()
        .filter(|record| record.stratum_id == "corridor-edge")
    {
        record.realized.decision_relevance = 0.20;
    }
    let result = assess(&claim(), &records, &policy()).unwrap();
    assert_eq!(result.disposition, CalibrationDisposition::NotCalibrated);
    assert!(result.strata.iter().any(|stratum| !stratum.passed));
}

#[test]
fn insufficient_independent_groups_produces_indeterminate_not_calibrated() {
    let mut records = observations();
    records.retain(|record| {
        record.stratum_id != "corridor-edge"
            || record.independent_group_id.ends_with("000")
            || record.independent_group_id.ends_with("001")
    });
    let result = assess(&claim(), &records, &policy()).unwrap();
    assert_eq!(
        result.disposition,
        CalibrationDisposition::IndeterminateInsufficientData
    );
    assert!(
        result
            .strata
            .iter()
            .find(|stratum| stratum.stratum_id == "corridor-edge")
            .unwrap()
            .sufficient_data
            == false
    );
}

#[test]
fn reusing_an_independent_group_cannot_inflate_sample_count() {
    let mut records = observations();
    let duplicate_group = records[0].independent_group_id.clone();
    records[1].independent_group_id = duplicate_group;
    assert_eq!(
        assess(&claim(), &records, &policy()),
        Err(CalibrationError::DuplicateIndependentGroup)
    );
}

#[test]
fn duplicate_observation_identity_is_rejected() {
    let mut records = observations();
    records[1].observation_id = records[0].observation_id.clone();
    assert_eq!(
        assess(&claim(), &records, &policy()),
        Err(CalibrationError::DuplicateObservationId)
    );
}

#[test]
fn adaptive_or_confirmatory_data_cannot_enter_the_sealed_challenge_set() {
    let mut records = observations();
    records[0].partition = EvidencePartition::AdaptiveDevelopment;
    assert_eq!(
        assess(&claim(), &records, &policy()),
        Err(CalibrationError::SealedPartitionContamination)
    );

    records[0].partition = EvidencePartition::ConfirmatorySealed;
    assert_eq!(
        assess(&claim(), &records, &policy()),
        Err(CalibrationError::SealedPartitionContamination)
    );
}

#[test]
fn observation_from_a_different_candidate_or_utility_receipt_is_rejected() {
    let mut records = observations();
    records[0].candidate_id = "other-candidate".into();
    assert_eq!(
        assess(&claim(), &records, &policy()),
        Err(CalibrationError::ObservationClaimMismatch)
    );

    records[0].candidate_id = claim().candidate_id;
    records[0].utility_receipt_payload_sha256 = digest('9');
    assert_eq!(
        assess(&claim(), &records, &policy()),
        Err(CalibrationError::ObservationClaimMismatch)
    );
}

#[test]
fn challenge_manifest_must_match_the_frozen_sealed_partition() {
    let mut records = observations();
    records[0].challenge_manifest_sha256 = digest('9');
    assert_eq!(
        assess(&claim(), &records, &policy()),
        Err(CalibrationError::ChallengeManifestMismatch)
    );
}

#[test]
fn candidate_claim_must_match_exact_decision_and_model_scope() {
    let mut wrong_scope = claim();
    wrong_scope.decision_boundary_sha256 = digest('9');
    assert_eq!(
        assess(&wrong_scope, &observations(), &policy()),
        Err(CalibrationError::ClaimPolicyMismatch)
    );

    wrong_scope = claim();
    wrong_scope.model_scope_sha256 = digest('9');
    assert_eq!(
        assess(&wrong_scope, &observations(), &policy()),
        Err(CalibrationError::ClaimPolicyMismatch)
    );
}

#[test]
fn estimator_build_and_method_are_policy_bound() {
    let mut wrong = claim();
    wrong.estimator_build_sha256 = digest('9');
    assert_eq!(
        assess(&wrong, &observations(), &policy()),
        Err(CalibrationError::EstimatorMismatch)
    );

    wrong = claim();
    wrong.estimate_method_sha256 = digest('9');
    assert_eq!(
        assess(&wrong, &observations(), &policy()),
        Err(CalibrationError::EstimatorMismatch)
    );
}

#[test]
fn sampling_unit_must_match_policy() {
    let mut wrong = claim();
    wrong.independent_unit = IndependentUnit::Flight;
    assert_eq!(
        assess(&wrong, &observations(), &policy()),
        Err(CalibrationError::SamplingUnitMismatch)
    );
}

#[test]
fn adaptive_estimator_data_manifest_cannot_be_the_challenge_manifest() {
    let mut wrong = claim();
    wrong.adaptive_data_manifest_sha256 = digest('6');
    assert_eq!(
        assess(&wrong, &observations(), &policy()),
        Err(CalibrationError::ClaimPolicyMismatch)
    );
}

#[test]
fn unexpected_strata_are_not_pooled_into_a_known_stratum() {
    let mut records = observations();
    records[0].stratum_id = "unknown-stratum".into();
    assert_eq!(
        assess(&claim(), &records, &policy()),
        Err(CalibrationError::UnexpectedStratum)
    );
}

#[test]
fn missing_required_stratum_is_not_silently_ignored() {
    let records = observations()
        .into_iter()
        .filter(|record| record.stratum_id == "crosswind-high")
        .collect::<Vec<_>>();
    assert_eq!(
        assess(&claim(), &records, &policy()),
        Err(CalibrationError::InvalidStrataOrDimensions)
    );
}

#[test]
fn non_finite_realized_utility_is_rejected() {
    let mut records = observations();
    records[0].realized.decision_relevance = f64::NAN;
    assert_eq!(
        assess(&claim(), &records, &policy()),
        Err(CalibrationError::InvalidObservation)
    );
}

#[test]
fn challenge_input_permutation_preserves_calibration_report() {
    let original = observations();
    let expected = assess(&claim(), &original, &policy()).unwrap();
    let mut permuted = original;
    permuted.reverse();
    assert_eq!(assess(&claim(), &permuted, &policy()).unwrap(), expected);
}

#[test]
fn unsupported_interval_confidence_level_fails_closed() {
    let mut invalid = policy();
    invalid.one_sided_confidence_level = 0.975;
    assert_eq!(
        assess(&claim(), &observations(), &invalid),
        Err(CalibrationError::InvalidPolicy)
    );
}

#[test]
fn confidence_contract_must_not_understate_the_claimed_estimate_level() {
    let mut claim_with_high_level = claim();
    claim_with_high_level.estimate_confidence_level = 0.99;
    let result = assess(&claim_with_high_level, &observations(), &policy());
    assert!(result.is_err());
}

#[test]
fn wilson_bounds_have_expected_limits_for_all_success_and_no_uninformative_outcomes() {
    let report = assess(&claim(), &observations(), &policy()).unwrap();
    for stratum in &report.strata {
        assert!(stratum.bound_coverage.iter().all(|metric| {
            metric.empirical_coverage == 1.0 && metric.wilson_lower_bound > 0.95
        }));
        assert_eq!(stratum.uninformative_rate.empirical_rate, 0.0);
        assert!(stratum.uninformative_rate.wilson_upper_bound < 0.05);
    }
}
