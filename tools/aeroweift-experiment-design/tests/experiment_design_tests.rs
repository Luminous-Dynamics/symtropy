use aeroweift_experiment_design::{
    AuthorizationState, DesignDisposition, ExperimentCandidate, ExperimentDesignPolicy,
    ExperimentModality, ExperimentObjective, HypothesisGroup, RejectionReason, ResourceBudget,
    ResourceCost, SemanticEquivalence, SelectionEvidenceRole, select_experiments,
};

fn digest(ch: char) -> String {
    std::iter::repeat_n(ch, 64).collect()
}

fn policy() -> ExperimentDesignPolicy {
    ExperimentDesignPolicy {
        campaign_sha256: digest('b'),
        model_scope_sha256: digest('c'),
        intended_use_profile_sha256: digest('d'),
        decision_question_id: "occupancy-boundary-q1".into(),
        objective: ExperimentObjective::ModelDiscrimination,
        policy_sha256: digest('a'),
        maximum_cost_units: 100,
        minimum_decision_relevance: 0.40,
        minimum_worst_case_robust_reduction: 0.20,
        minimum_model_discrimination: 0.30,
        maximum_probability_uninformative: 0.80,
        minimum_estimate_confidence_level: 0.95,
        required_estimator_build_sha256: digest('e'),
        required_estimate_method_sha256: digest('f'),
        resource_budget: ResourceBudget {
            setup_minutes: 100,
            facility_minutes: 100,
            cpu_seconds: 10_000,
            accelerator_seconds: 10_000,
            energy_joules: 100_000,
            adjudication_person_minutes: 100,
        },
    }
}

fn candidate(id: &str) -> ExperimentCandidate {
    ExperimentCandidate {
        candidate_id: id.into(),
        campaign_sha256: digest('b'),
        model_scope_sha256: digest('c'),
        intended_use_profile_sha256: digest('d'),
        design_sha256: digest('1'),
        scenario_manifest_sha256: digest('2'),
        measurement_plan_sha256: digest('3'),
        decision_question_id: "occupancy-boundary-q1".into(),
        objective: ExperimentObjective::ModelDiscrimination,
        modality: ExperimentModality::Simulation,
        requires_external_authorization: false,
        authorization: AuthorizationState::NotRequired,
        authorization_receipt_sha256: None,
        safety_constraints_clear: true,
        safety_gate_receipt_sha256: Some(digest('4')),
        within_declared_envelope: true,
        envelope_receipt_sha256: Some(digest('5')),
        independent_reference_available: true,
        independent_reference_receipt_sha256: Some(digest('6')),
        semantic_equivalence: SemanticEquivalence::Established,
        semantic_equivalence_receipt_sha256: Some(digest('7')),
        selection_evidence_role: SelectionEvidenceRole::AdaptiveDevelopment,
        hypothesis_groups: vec![
            HypothesisGroup { group_id: "model-family-a".into() },
            HypothesisGroup { group_id: "model-family-b".into() },
        ],
        expected_uncertainty_reduction: 0.70,
        expected_model_discrimination: 0.80,
        worst_case_robust_reduction: 0.50,
        decision_relevance: 0.90,
        probability_uninformative: 0.10,
        estimate_confidence_level: 0.95,
        estimate_method_sha256: digest('f'),
        estimator_build_sha256: digest('e'),
        cost_units: 20,
        resource_cost: ResourceCost {
            setup_minutes: 10,
            facility_minutes: 10,
            cpu_seconds: 500,
            accelerator_seconds: 100,
            energy_joules: 1_000,
            adjudication_person_minutes: 5,
        },
    }
}

fn reason(result: &aeroweift_experiment_design::ExperimentDesignResult, id: &str) -> Vec<RejectionReason> {
    result
        .rejected
        .iter()
        .find(|candidate| candidate.candidate_id == id)
        .map(|candidate| candidate.reasons.clone())
        .unwrap_or_default()
}

#[test]
fn positive_control_returns_a_pareto_front() {
    let result = select_experiments(&[candidate("baseline")], &policy()).unwrap();
    assert_eq!(result.disposition, DesignDisposition::ParetoFrontAvailable);
    assert_eq!(result.pareto_front.len(), 1);
    assert_eq!(result.pareto_front[0].candidate_id, "baseline");
    assert!(result.rejected.is_empty());
}

#[test]
fn dominated_candidate_is_removed_without_weighted_scoring() {
    let best = candidate("best");
    let mut inferior = candidate("dominated");
    inferior.expected_uncertainty_reduction = 0.50;
    inferior.expected_model_discrimination = 0.60;
    inferior.worst_case_robust_reduction = 0.30;
    inferior.decision_relevance = 0.70;
    inferior.probability_uninformative = 0.20;
    inferior.cost_units = 30;

    let result = select_experiments(&[best, inferior], &policy()).unwrap();
    assert_eq!(result.pareto_front.len(), 1);
    assert_eq!(result.pareto_front[0].candidate_id, "best");
}

#[test]
fn real_cost_utility_tradeoff_preserves_multiple_pareto_options() {
    let mut cheap = candidate("cheap");
    cheap.expected_uncertainty_reduction = 0.50;
    cheap.expected_model_discrimination = 0.60;
    cheap.worst_case_robust_reduction = 0.35;
    cheap.decision_relevance = 0.70;
    cheap.probability_uninformative = 0.20;
    cheap.cost_units = 10;

    let mut expensive = candidate("expensive");
    expensive.cost_units = 70;

    let result = select_experiments(&[expensive, cheap], &policy()).unwrap();
    assert_eq!(result.pareto_front.len(), 2);
    assert_eq!(
        result.pareto_front.iter().map(|x| x.candidate_id.as_str()).collect::<Vec<_>>(),
        vec!["cheap", "expensive"]
    );
}

#[test]
fn high_information_but_irrelevant_test_is_rejected() {
    let mut irrelevant = candidate("irrelevant");
    irrelevant.expected_uncertainty_reduction = 0.99;
    irrelevant.expected_model_discrimination = 0.99;
    irrelevant.decision_relevance = 0.10;

    let result = select_experiments(&[irrelevant], &policy()).unwrap();
    assert_eq!(result.disposition, DesignDisposition::AbstainNoAdmissibleExperiment);
    assert!(reason(&result, "irrelevant").contains(&RejectionReason::InsufficientDecisionRelevance));
}

#[test]
fn flight_test_without_explicit_authorization_is_never_selected() {
    let mut flight = candidate("flight");
    flight.modality = ExperimentModality::ControlledFlight;
    flight.requires_external_authorization = true;
    flight.authorization = AuthorizationState::Unknown;

    let result = select_experiments(&[flight], &policy()).unwrap();
    assert!(result.pareto_front.is_empty());
    assert!(reason(&result, "flight").contains(&RejectionReason::AuthorizationNotGranted));
}

#[test]
fn explicit_denial_blocks_a_flight_test() {
    let mut flight = candidate("flight-denied");
    flight.modality = ExperimentModality::ControlledFlight;
    flight.requires_external_authorization = true;
    flight.authorization = AuthorizationState::NotAuthorized;

    let result = select_experiments(&[flight], &policy()).unwrap();
    assert!(reason(&result, "flight-denied").contains(&RejectionReason::AuthorizationNotGranted));
}

#[test]
fn uncleared_safety_constraint_blocks_selection() {
    let mut unsafe_candidate = candidate("safety-not-clear");
    unsafe_candidate.safety_constraints_clear = false;

    let result = select_experiments(&[unsafe_candidate], &policy()).unwrap();
    assert!(reason(&result, "safety-not-clear").contains(&RejectionReason::SafetyConstraintNotClear));
}

#[test]
fn outside_envelope_candidate_is_rejected() {
    let mut outside = candidate("outside");
    outside.within_declared_envelope = false;

    let result = select_experiments(&[outside], &policy()).unwrap();
    assert!(reason(&result, "outside").contains(&RejectionReason::OutsideDeclaredEnvelope));
}

#[test]
fn missing_independent_reference_is_rejected() {
    let mut ungrounded = candidate("no-reference");
    ungrounded.independent_reference_available = false;

    let result = select_experiments(&[ungrounded], &policy()).unwrap();
    assert!(reason(&result, "no-reference").contains(&RejectionReason::MissingIndependentReference));
}

#[test]
fn cross_implementation_test_requires_semantic_equivalence() {
    let mut comparison = candidate("cross-code");
    comparison.modality = ExperimentModality::CrossImplementation;
    comparison.semantic_equivalence = SemanticEquivalence::Unknown;

    let result = select_experiments(&[comparison], &policy()).unwrap();
    assert!(reason(&result, "cross-code").contains(&RejectionReason::SemanticEquivalenceNotEstablished));
}

#[test]
fn cross_implementation_test_is_admissible_after_semantic_equivalence() {
    let mut comparison = candidate("cross-code-matched");
    comparison.modality = ExperimentModality::CrossImplementation;
    comparison.semantic_equivalence = SemanticEquivalence::Established;

    let result = select_experiments(&[comparison], &policy()).unwrap();
    assert_eq!(result.pareto_front[0].candidate_id, "cross-code-matched");
}

#[test]
fn confirmatory_or_challenge_outcomes_cannot_drive_adaptive_selection() {
    let mut contaminated = candidate("sealed-data");
    contaminated.selection_evidence_role = SelectionEvidenceRole::ChallengeSealed;

    let result = select_experiments(&[contaminated], &policy()).unwrap();
    assert!(reason(&result, "sealed-data").contains(&RejectionReason::SealedEvidenceUsedForSelection));
}

#[test]
fn correlated_hypotheses_do_not_count_as_independent_models() {
    let mut false_diversity = candidate("correlated");
    false_diversity.hypothesis_groups = vec![
        HypothesisGroup { group_id: "shared-family".into() },
        HypothesisGroup { group_id: "shared-family".into() },
    ];

    let result = select_experiments(&[false_diversity], &policy()).unwrap();
    assert!(reason(&result, "correlated").contains(&RejectionReason::InsufficientIndependentHypothesisGroups));
}

#[test]
fn genuinely_distinct_hypothesis_groups_can_support_discrimination() {
    let result = select_experiments(&[candidate("independent-models")], &policy()).unwrap();
    assert_eq!(result.pareto_front.len(), 1);
}

#[test]
fn model_discrimination_threshold_is_an_explicit_gate() {
    let mut weak = candidate("weak-discrimination");
    weak.expected_model_discrimination = 0.10;

    let result = select_experiments(&[weak], &policy()).unwrap();
    assert!(reason(&result, "weak-discrimination").contains(&RejectionReason::InsufficientModelDiscrimination));
}

#[test]
fn robust_reduction_threshold_is_an_explicit_gate() {
    let mut brittle = candidate("brittle");
    brittle.worst_case_robust_reduction = 0.10;

    let result = select_experiments(&[brittle], &policy()).unwrap();
    assert!(reason(&result, "brittle").contains(&RejectionReason::InsufficientRobustReduction));
}

#[test]
fn cost_budget_is_a_hard_constraint() {
    let mut costly = candidate("costly");
    costly.cost_units = 101;

    let result = select_experiments(&[costly], &policy()).unwrap();
    assert!(reason(&result, "costly").contains(&RejectionReason::OverBudget));
}

#[test]
fn non_finite_utility_is_rejected_not_ranked() {
    let mut malformed = candidate("nan");
    malformed.expected_uncertainty_reduction = f64::NAN;

    let result = select_experiments(&[malformed], &policy()).unwrap();
    assert!(reason(&result, "nan").contains(&RejectionReason::InvalidUtilityVector));
}

#[test]
fn out_of_range_uninformative_probability_is_rejected() {
    let mut malformed = candidate("bad-probability");
    malformed.probability_uninformative = 1.1;

    let result = select_experiments(&[malformed], &policy()).unwrap();
    assert!(reason(&result, "bad-probability").contains(&RejectionReason::InvalidUtilityVector));
}

#[test]
fn parameter_identification_does_not_require_two_independent_model_families() {
    let mut parameter_policy = policy();
    parameter_policy.objective = ExperimentObjective::ParameterIdentification;
    let mut parameter_test = candidate("one-family-parameter-test");
    parameter_test.objective = ExperimentObjective::ParameterIdentification;
    parameter_test.hypothesis_groups = vec![HypothesisGroup {
        group_id: "one-model-family".into(),
    }];
    parameter_test.expected_model_discrimination = 0.0;

    let result = select_experiments(&[parameter_test], &parameter_policy).unwrap();
    assert_eq!(result.disposition, DesignDisposition::ParetoFrontAvailable);
}

#[test]
fn decision_question_mismatch_is_not_borrowed_for_a_different_claim() {
    let mut wrong_question = candidate("wrong-question");
    wrong_question.decision_question_id = "different-question".into();

    let result = select_experiments(&[wrong_question], &policy()).unwrap();
    assert!(reason(&result, "wrong-question").contains(&RejectionReason::QuestionOrObjectiveMismatch));
}

#[test]
fn objective_mismatch_is_not_borrowed_for_a_different_question_type() {
    let mut wrong_objective = candidate("wrong-objective");
    wrong_objective.objective = ExperimentObjective::ParameterIdentification;

    let result = select_experiments(&[wrong_objective], &policy()).unwrap();
    assert!(reason(&result, "wrong-objective").contains(&RejectionReason::QuestionOrObjectiveMismatch));
}

#[test]
fn candidate_input_permutation_preserves_exact_result() {
    let first = candidate("first");
    let mut second = candidate("second");
    second.expected_uncertainty_reduction = 0.55;
    second.expected_model_discrimination = 0.65;
    second.worst_case_robust_reduction = 0.35;
    second.decision_relevance = 0.75;
    second.probability_uninformative = 0.20;
    second.cost_units = 15;

    let candidates = vec![first, second];
    let expected = select_experiments(&candidates, &policy()).unwrap();
    let mut reversed = candidates;
    reversed.reverse();
    assert_eq!(select_experiments(&reversed, &policy()).unwrap(), expected);
}

#[test]
fn duplicate_candidate_identity_fails_closed() {
    let duplicate = vec![candidate("same-id"), candidate("same-id")];
    assert_eq!(
        select_experiments(&duplicate, &policy()),
        Err(aeroweift_experiment_design::DesignError::DuplicateCandidateId)
    );
}

#[test]
fn invalid_policy_digest_or_threshold_fails_closed() {
    let mut invalid = policy();
    invalid.policy_sha256 = "not-a-digest".into();
    assert_eq!(
        select_experiments(&[candidate("candidate")], &invalid),
        Err(aeroweift_experiment_design::DesignError::InvalidPolicy)
    );

    let mut invalid_threshold = policy();
    invalid_threshold.minimum_decision_relevance = f64::NAN;
    assert_eq!(
        select_experiments(&[candidate("candidate")], &invalid_threshold),
        Err(aeroweift_experiment_design::DesignError::InvalidPolicy)
    );
}

#[test]
fn reject_all_is_an_abstention_not_a_successful_recommendation() {
    let mut candidate = candidate("no-safe-experiment");
    candidate.safety_constraints_clear = false;
    candidate.within_declared_envelope = false;
    candidate.requires_external_authorization = true;
    candidate.authorization = AuthorizationState::Unknown;

    let result = select_experiments(&[candidate], &policy()).unwrap();
    assert_eq!(result.disposition, DesignDisposition::AbstainNoAdmissibleExperiment);
    assert!(result.pareto_front.is_empty());
    assert!(result.rejected[0].reasons.contains(&RejectionReason::AuthorizationNotGranted));
    assert!(result.rejected[0].reasons.contains(&RejectionReason::SafetyConstraintNotClear));
}


#[test]
fn model_discrimination_requires_semantically_comparable_outputs() {
    let mut unmatched = candidate("unmatched-models");
    unmatched.semantic_equivalence = SemanticEquivalence::NotRequired;
    let result = select_experiments(&[unmatched], &policy()).unwrap();
    assert!(reason(&result, "unmatched-models").contains(&RejectionReason::SemanticEquivalenceNotEstablished));
}

#[test]
fn excessive_probability_of_uninformative_result_is_a_hard_gate() {
    let mut low_value = candidate("likely-uninformative");
    low_value.probability_uninformative = 0.95;
    let result = select_experiments(&[low_value], &policy()).unwrap();
    assert!(reason(&result, "likely-uninformative").contains(&RejectionReason::TooLikelyUninformative));
}

#[test]
fn empty_hypothesis_correlation_group_is_rejected() {
    let mut malformed = candidate("empty-group");
    malformed.hypothesis_groups[0].group_id.clear();
    let result = select_experiments(&[malformed], &policy()).unwrap();
    assert!(reason(&result, "empty-group").contains(&RejectionReason::InvalidUtilityVector));
}


#[test]
fn controlled_flight_always_requires_authorization_even_if_candidate_says_not_required() {
    let mut flight = candidate("flight-no-flag-bypass");
    flight.modality = ExperimentModality::ControlledFlight;
    flight.requires_external_authorization = false;
    flight.authorization = AuthorizationState::NotRequired;
    flight.authorization_receipt_sha256 = None;
    let result = select_experiments(&[flight], &policy()).unwrap();
    assert!(reason(&result, "flight-no-flag-bypass").contains(&RejectionReason::AuthorizationNotGranted));
}

#[test]
fn affirmative_safety_gate_without_owner_receipt_is_rejected() {
    let mut candidate = candidate("no-safety-receipt");
    candidate.safety_gate_receipt_sha256 = None;
    let result = select_experiments(&[candidate], &policy()).unwrap();
    assert!(reason(&result, "no-safety-receipt").contains(&RejectionReason::MissingEvidenceReceipt));
}

#[test]
fn affirmative_reference_gate_without_owner_receipt_is_rejected() {
    let mut candidate = candidate("no-reference-receipt");
    candidate.independent_reference_receipt_sha256 = None;
    let result = select_experiments(&[candidate], &policy()).unwrap();
    assert!(reason(&result, "no-reference-receipt").contains(&RejectionReason::MissingEvidenceReceipt));
}

#[test]
fn candidate_from_another_model_scope_cannot_borrow_this_campaigns_credit() {
    let mut candidate = candidate("other-model");
    candidate.model_scope_sha256 = digest('8');
    let result = select_experiments(&[candidate], &policy()).unwrap();
    assert!(reason(&result, "other-model").contains(&RejectionReason::QuestionOrObjectiveMismatch));
}

#[test]
fn estimator_build_and_confidence_method_are_pinned() {
    let mut candidate = candidate("other-estimator");
    candidate.estimator_build_sha256 = digest('9');
    let result = select_experiments(&[candidate], &policy()).unwrap();
    assert!(reason(&result, "other-estimator").contains(&RejectionReason::ConfidenceContractMismatch));
}

#[test]
fn resource_dimensions_are_hard_non_substitutable_budgets() {
    let mut candidate = candidate("facility-over-budget");
    candidate.resource_cost.facility_minutes = 101;
    let result = select_experiments(&[candidate], &policy()).unwrap();
    assert!(reason(&result, "facility-over-budget").contains(&RejectionReason::OverBudget));
}

#[test]
fn pareto_front_preserves_tradeoffs_across_resource_dimensions() {
    let mut low_energy = candidate("low-energy");
    low_energy.resource_cost.energy_joules = 100;
    low_energy.resource_cost.facility_minutes = 80;
    low_energy.expected_uncertainty_reduction = 0.60;

    let mut low_facility = candidate("low-facility");
    low_facility.resource_cost.energy_joules = 5_000;
    low_facility.resource_cost.facility_minutes = 5;
    low_facility.expected_uncertainty_reduction = 0.75;

    let result = select_experiments(&[low_energy, low_facility], &policy()).unwrap();
    assert_eq!(result.pareto_front.len(), 2);
}

#[test]
fn confirmatory_and_challenge_partitions_remain_sealed() {
    for (id, role) in [
        ("confirmatory-sealed", SelectionEvidenceRole::ConfirmatorySealed),
        ("challenge-sealed", SelectionEvidenceRole::ChallengeSealed),
    ] {
        let mut candidate = candidate(id);
        candidate.selection_evidence_role = role;
        let result = select_experiments(&[candidate], &policy()).unwrap();
        assert!(reason(&result, id).contains(&RejectionReason::SealedEvidenceUsedForSelection));
    }
}
