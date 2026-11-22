//! Finite-state expected value of sample information for scientific decisions.
//!
//! This module evaluates one-step information acquisition in a finite Bayesian
//! decision problem. It can compare additional computation, posterior sampling,
//! calibration, source documentation, or expert review because all are
//! represented by a declared conditional outcome model.
//!
//! The model is explicit:
//!   p(state), utility(action, state), p(outcome | state, option), and cost.
//!
//! EVSI is the expected optimal utility after observing an option's outcome
//! minus the optimal utility under current information. Net benefit subtracts
//! declared cost in utility-equivalent units.
//!
//! This is an exact finite-state decision oracle, not a nonlinear EIG estimator,
//! multifidelity covariance learner, or multi-step scheduler. Option likelihoods
//! encode dependence on the true state; a receipt is required before their
//! result can influence a qualified decision. Receipt strings are references,
//! not self-authenticating proofs.

use crate::ModelError;

/// Type of information-acquisition action being evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InformationAcquisitionKind {
    LowFidelitySimulation,
    HighFidelitySimulation,
    PosteriorSampling,
    IndependentCalibration,
    SourceDocumentation,
    HumanExpertReview,
    Other,
}

/// Qualification for the source/dependence model used by an acquisition option.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InformationSourceQualification {
    /// A verifiable receipt covers the option's complete likelihood/source model.
    Qualified { receipt_id: String },
    /// The option may be explored, but cannot support a qualified decision.
    Unqualified,
}

/// One possible information acquisition.
///
/// outcome_likelihoods is state-major: rows correspond to state_probabilities,
/// columns correspond to mutually exclusive outcomes. Each row must sum to one.
/// Its likelihood must already include any dependence on existing evidence.
#[derive(Debug, Clone, PartialEq)]
pub struct InformationAcquisitionOption {
    pub option_id: String,
    pub kind: InformationAcquisitionKind,
    /// Resource cost, in the caller's declared cost unit.
    pub cost: f64,
    pub outcome_likelihoods: Vec<Vec<f64>>,
    pub source_qualification: InformationSourceQualification,
}

/// Why a decision policy abstains rather than selecting an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionAbstentionReason {
    NoActionBeatsAbstention,
    NoQualifiedPositiveValueInformation,
}

/// Recommended next step under the finite decision model.
#[derive(Debug, Clone, PartialEq)]
pub enum FiniteDecisionRecommendation {
    AcquireInformation {
        option_id: String,
        kind: InformationAcquisitionKind,
        expected_value_of_sample_information: f64,
        acquisition_cost: f64,
        cost_in_utility_units: f64,
        expected_net_benefit_of_sampling: f64,
        expected_utility_before_information: f64,
        expected_utility_after_information: f64,
        expected_value_of_perfect_information: f64,
        current_best_action_id: Option<String>,
        skipped_unqualified_options: usize,
    },
    SelectAction {
        action_id: String,
        expected_utility: f64,
        expected_value_of_perfect_information: f64,
        skipped_unqualified_options: usize,
    },
    Abstain {
        reason: DecisionAbstentionReason,
        current_best_action_id: Option<String>,
        current_best_action_utility: Option<f64>,
        abstention_utility: f64,
        expected_value_of_perfect_information: f64,
        skipped_unqualified_options: usize,
    },
}

/// Recommend whether to acquire one more source of information, select an
/// action, or abstain.
///
/// Inputs:
/// - action_ids: unique action names.
/// - state_probabilities: prior probabilities over mutually exclusive states.
/// - action_utilities: action-major matrix [action][state]; larger is better.
/// - abstention_utility: utility of taking no irreversible action.
/// - information_options: finite observation models for candidate acquisitions.
/// - remaining_budget: hard resource limit in the cost unit.
/// - cost_to_utility_rate: utility units charged per unit cost.
/// - minimum_net_benefit: minimum net utility required before acquiring.
///
/// For each option, EVSI is computed exactly by summing the unnormalized joint
/// state/outcome probabilities and selecting the best posterior action for each
/// outcome. Unqualified options are skipped rather than allowed to improve a
/// decision score. This is a one-step/myopic policy; after new information is
/// acquired, call the function again using updated state probabilities and
/// remaining budget.
pub fn recommend_finite_decision_step(
    action_ids: &[String],
    state_probabilities: &[f64],
    action_utilities: &[Vec<f64>],
    abstention_utility: f64,
    information_options: &[InformationAcquisitionOption],
    remaining_budget: f64,
    cost_to_utility_rate: f64,
    minimum_net_benefit: f64,
) -> Result<FiniteDecisionRecommendation, ModelError> {
    let state_count = state_probabilities.len();
    let action_count = action_ids.len();
    if state_count == 0
        || action_count == 0
        || action_utilities.len() != action_count
        || action_utilities.iter().any(|row| row.len() != state_count)
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    if action_ids.iter().any(|id| id.trim().is_empty()) {
        return Err(ModelError::InvalidParameter(
            "decision_action_ids_must_be_nonempty",
        ));
    }
    let mut unique_actions = std::collections::BTreeSet::new();
    if action_ids
        .iter()
        .any(|id| !unique_actions.insert(id.as_str()))
    {
        return Err(ModelError::InvalidParameter(
            "decision_action_ids_must_be_unique",
        ));
    }
    if state_probabilities
        .iter()
        .any(|probability| !probability.is_finite())
        || action_utilities
            .iter()
            .flatten()
            .any(|utility| !utility.is_finite())
        || !abstention_utility.is_finite()
    {
        return Err(ModelError::NonFiniteValue("finite_decision_input"));
    }
    if state_probabilities
        .iter()
        .any(|probability| *probability < 0.0)
    {
        return Err(ModelError::InvalidParameter(
            "decision_state_probabilities_must_be_nonnegative",
        ));
    }
    let probability_sum = state_probabilities.iter().sum::<f64>();
    if !probability_sum.is_finite() {
        return Err(ModelError::InvalidParameter(
            "decision_probability_sum_nonfinite",
        ));
    }
    if probability_sum <= 0.0 {
        return Err(ModelError::NonPositiveValue("decision_probability_sum"));
    }
    let probabilities: Vec<f64> = state_probabilities
        .iter()
        .map(|probability| probability / probability_sum)
        .collect();

    if !remaining_budget.is_finite()
        || !cost_to_utility_rate.is_finite()
        || !minimum_net_benefit.is_finite()
    {
        return Err(ModelError::NonFiniteValue("decision_budget_or_cost_rate"));
    }
    if remaining_budget < 0.0 || cost_to_utility_rate < 0.0 || minimum_net_benefit < 0.0 {
        return Err(ModelError::InvalidParameter(
            "decision_budget_cost_rate_and_threshold_must_be_nonnegative",
        ));
    }

    let mut best_action_index = 0usize;
    let mut best_action_utility = f64::NEG_INFINITY;
    for (action_index, utilities) in action_utilities.iter().enumerate() {
        let expected_utility = probabilities
            .iter()
            .zip(utilities.iter())
            .map(|(probability, utility)| probability * utility)
            .sum::<f64>();
        if expected_utility > best_action_utility {
            best_action_utility = expected_utility;
            best_action_index = action_index;
        }
    }

    let utility_with_current_information = best_action_utility.max(abstention_utility);
    let perfect_information_utility = probabilities
        .iter()
        .enumerate()
        .map(|(state_index, probability)| {
            let best_in_state = action_utilities
                .iter()
                .map(|utilities| utilities[state_index])
                .fold(abstention_utility, f64::max);
            probability * best_in_state
        })
        .sum::<f64>();
    let expected_value_of_perfect_information =
        (perfect_information_utility - utility_with_current_information).max(0.0);

    let mut seen_options = std::collections::BTreeSet::new();
    let mut skipped_unqualified_options = 0usize;
    let mut best_option: Option<(&InformationAcquisitionOption, f64, f64, f64)> = None;

    for option in information_options {
        if option.option_id.trim().is_empty() || !seen_options.insert(option.option_id.as_str()) {
            return Err(ModelError::InvalidParameter(
                "information_option_ids_must_be_nonempty_and_unique",
            ));
        }
        if !option.cost.is_finite() {
            return Err(ModelError::NonFiniteValue("information_option_cost"));
        }
        if option.cost < 0.0 {
            return Err(ModelError::InvalidParameter(
                "information_option_cost_must_be_nonnegative",
            ));
        }
        if let InformationSourceQualification::Qualified { receipt_id } =
            &option.source_qualification
        {
            if receipt_id.trim().is_empty() {
                return Err(ModelError::InvalidParameter(
                    "information_option_qualification_receipt_must_be_nonempty",
                ));
            }
        }
        if matches!(
            &option.source_qualification,
            InformationSourceQualification::Unqualified
        ) {
            skipped_unqualified_options += 1;
            continue;
        }
        if option.cost > remaining_budget {
            continue;
        }

        if option.outcome_likelihoods.len() != state_count || option.outcome_likelihoods.is_empty()
        {
            return Err(ModelError::MismatchedDataLengths);
        }
        let outcome_count = option.outcome_likelihoods[0].len();
        if outcome_count == 0
            || option
                .outcome_likelihoods
                .iter()
                .any(|row| row.len() != outcome_count)
        {
            return Err(ModelError::MismatchedDataLengths);
        }
        for row in &option.outcome_likelihoods {
            if row.iter().any(|likelihood| !likelihood.is_finite()) {
                return Err(ModelError::NonFiniteValue(
                    "information_option_outcome_likelihood",
                ));
            }
            if row
                .iter()
                .any(|likelihood| *likelihood < 0.0 || *likelihood > 1.0)
            {
                return Err(ModelError::InvalidParameter(
                    "information_option_likelihood_must_be_in_[0,1]",
                ));
            }
            let row_sum = row.iter().sum::<f64>();
            if (row_sum - 1.0).abs() > 1.0e-10 {
                return Err(ModelError::InvalidParameter(
                    "information_option_likelihood_row_must_sum_to_one",
                ));
            }
        }

        let mut expected_utility_after_information = 0.0;
        for outcome_index in 0..outcome_count {
            let outcome_probability = probabilities
                .iter()
                .enumerate()
                .map(|(state_index, probability)| {
                    probability * option.outcome_likelihoods[state_index][outcome_index]
                })
                .sum::<f64>();
            if outcome_probability <= 0.0 {
                continue;
            }

            let abstention_value_for_outcome = abstention_utility * outcome_probability;
            let mut best_posterior_utility = abstention_value_for_outcome;
            for utilities in action_utilities {
                let unnormalized_posterior_utility = probabilities
                    .iter()
                    .enumerate()
                    .map(|(state_index, probability)| {
                        probability
                            * option.outcome_likelihoods[state_index][outcome_index]
                            * utilities[state_index]
                    })
                    .sum::<f64>();
                best_posterior_utility = best_posterior_utility.max(unnormalized_posterior_utility);
            }
            expected_utility_after_information += best_posterior_utility;
        }

        let raw_evsi = expected_utility_after_information - utility_with_current_information;
        let numerical_tolerance = 1.0e-10 * utility_with_current_information.abs().max(1.0);
        if raw_evsi < -numerical_tolerance {
            return Err(ModelError::InvalidParameter(
                "information_option_reduces_optimal_expected_utility",
            ));
        }
        let evsi = raw_evsi.max(0.0);
        if evsi > expected_value_of_perfect_information + numerical_tolerance {
            return Err(ModelError::InvalidParameter(
                "information_option_evsi_exceeds_evpi",
            ));
        }

        let cost_in_utility_units = option.cost * cost_to_utility_rate;
        if !cost_in_utility_units.is_finite() {
            return Err(ModelError::InvalidParameter(
                "information_option_cost_conversion_nonfinite",
            ));
        }
        let net_benefit = evsi - cost_in_utility_units;
        if net_benefit > minimum_net_benefit
            && best_option
                .as_ref()
                .is_none_or(|(_, _, _, current_net)| net_benefit > *current_net)
        {
            best_option = Some((
                option,
                evsi,
                expected_utility_after_information,
                net_benefit,
            ));
        }
    }

    if let Some((option, evsi, expected_after, net_benefit)) = best_option {
        return Ok(FiniteDecisionRecommendation::AcquireInformation {
            option_id: option.option_id.clone(),
            kind: option.kind,
            expected_value_of_sample_information: evsi,
            acquisition_cost: option.cost,
            cost_in_utility_units: option.cost * cost_to_utility_rate,
            expected_net_benefit_of_sampling: net_benefit,
            expected_utility_before_information: utility_with_current_information,
            expected_utility_after_information: expected_after,
            expected_value_of_perfect_information,
            current_best_action_id: (best_action_utility > abstention_utility)
                .then(|| action_ids[best_action_index].clone()),
            skipped_unqualified_options,
        });
    }

    if best_action_utility > abstention_utility {
        Ok(FiniteDecisionRecommendation::SelectAction {
            action_id: action_ids[best_action_index].clone(),
            expected_utility: best_action_utility,
            expected_value_of_perfect_information,
            skipped_unqualified_options,
        })
    } else {
        Ok(FiniteDecisionRecommendation::Abstain {
            reason: if skipped_unqualified_options == information_options.len()
                && !information_options.is_empty()
            {
                DecisionAbstentionReason::NoQualifiedPositiveValueInformation
            } else {
                DecisionAbstentionReason::NoActionBeatsAbstention
            },
            current_best_action_id: Some(action_ids[best_action_index].clone()),
            current_best_action_utility: Some(best_action_utility),
            abstention_utility,
            expected_value_of_perfect_information,
            skipped_unqualified_options,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actions() -> Vec<String> {
        vec!["intervene".to_owned(), "hold".to_owned()]
    }

    fn options(
        id: &str,
        kind: InformationAcquisitionKind,
        cost: f64,
        likelihoods: Vec<Vec<f64>>,
    ) -> InformationAcquisitionOption {
        InformationAcquisitionOption {
            option_id: id.to_owned(),
            kind,
            cost,
            outcome_likelihoods: likelihoods,
            source_qualification: InformationSourceQualification::Qualified {
                receipt_id: format!("receipt:{id}"),
            },
        }
    }

    #[test]
    fn computes_exact_evsi_and_net_sampling_benefit() {
        let option = options(
            "calibration",
            InformationAcquisitionKind::IndependentCalibration,
            2.0,
            vec![vec![0.9, 0.1], vec![0.2, 0.8]],
        );
        let result = recommend_finite_decision_step(
            &actions(),
            &[0.5, 0.5],
            &[vec![100.0, -30.0], vec![0.0, 0.0]],
            0.0,
            &[option],
            2.0,
            1.0,
            0.0,
        )
        .expect("EVSI recommendation");
        match result {
            FiniteDecisionRecommendation::AcquireInformation {
                option_id,
                expected_value_of_sample_information,
                cost_in_utility_units,
                expected_net_benefit_of_sampling,
                expected_value_of_perfect_information,
                ..
            } => {
                assert_eq!(option_id, "calibration");
                assert!((expected_value_of_sample_information - 7.0).abs() < 1.0e-12);
                assert!((cost_in_utility_units - 2.0).abs() < 1.0e-12);
                assert!((expected_net_benefit_of_sampling - 5.0).abs() < 1.0e-12);
                assert!((expected_value_of_perfect_information - 15.0).abs() < 1.0e-12);
            }
            other => panic!("expected information acquisition, got {other:?}"),
        }
    }

    #[test]
    fn chooses_best_net_value_not_largest_raw_evsi() {
        let expensive_perfect = options(
            "expensive-perfect",
            InformationAcquisitionKind::HighFidelitySimulation,
            14.0,
            vec![vec![1.0, 0.0], vec![0.0, 1.0]],
        );
        let cheap_partial = options(
            "cheap-partial",
            InformationAcquisitionKind::LowFidelitySimulation,
            1.0,
            vec![vec![0.9, 0.1], vec![0.2, 0.8]],
        );
        let result = recommend_finite_decision_step(
            &actions(),
            &[0.5, 0.5],
            &[vec![100.0, -30.0], vec![0.0, 0.0]],
            0.0,
            &[expensive_perfect, cheap_partial],
            14.0,
            1.0,
            0.0,
        )
        .expect("cost-aware comparison");
        assert!(matches!(
            result,
            FiniteDecisionRecommendation::AcquireInformation { option_id, .. }
                if option_id == "cheap-partial"
        ));
    }

    #[test]
    fn recommends_select_when_information_cost_exceeds_evsi() {
        let option = options(
            "overpriced",
            InformationAcquisitionKind::HighFidelitySimulation,
            8.0,
            vec![vec![0.9, 0.1], vec![0.2, 0.8]],
        );
        let result = recommend_finite_decision_step(
            &actions(),
            &[0.5, 0.5],
            &[vec![100.0, -30.0], vec![0.0, 0.0]],
            0.0,
            &[option],
            8.0,
            1.0,
            0.0,
        )
        .expect("select current action");
        assert!(matches!(
            result,
            FiniteDecisionRecommendation::SelectAction { action_id, .. }
                if action_id == "intervene"
        ));
    }

    #[test]
    fn abstains_when_neither_action_nor_information_beats_abstention() {
        let option = options(
            "weak",
            InformationAcquisitionKind::LowFidelitySimulation,
            1.0,
            vec![vec![0.6, 0.4], vec![0.4, 0.6]],
        );
        let result = recommend_finite_decision_step(
            &actions(),
            &[0.5, 0.5],
            &[vec![1.0, -1.0], vec![0.0, 0.0]],
            0.1,
            &[option],
            1.0,
            1.0,
            0.0,
        )
        .expect("abstention");
        assert!(matches!(
            result,
            FiniteDecisionRecommendation::Abstain { .. }
        ));
    }

    #[test]
    fn unqualified_independent_evidence_is_not_used_to_recommend_action() {
        let mut option = options(
            "unqualified",
            InformationAcquisitionKind::IndependentCalibration,
            0.0,
            vec![vec![0.99, 0.01], vec![0.01, 0.99]],
        );
        option.source_qualification = InformationSourceQualification::Unqualified;
        let result = recommend_finite_decision_step(
            &actions(),
            &[0.5, 0.5],
            &[vec![1.0, -1.0], vec![0.0, 0.0]],
            0.1,
            &[option],
            1.0,
            1.0,
            0.0,
        )
        .expect("unqualified evidence ignored");
        assert_eq!(
            result,
            FiniteDecisionRecommendation::Abstain {
                reason: DecisionAbstentionReason::NoQualifiedPositiveValueInformation,
                current_best_action_id: Some("hold".to_owned()),
                current_best_action_utility: Some(0.0),
                abstention_utility: 0.1,
                expected_value_of_perfect_information: 0.45,
                skipped_unqualified_options: 1,
            }
        );
    }

    #[test]
    fn option_likelihoods_must_form_a_conditional_distribution() {
        let bad = options(
            "bad-likelihood",
            InformationAcquisitionKind::LowFidelitySimulation,
            1.0,
            vec![vec![0.4, 0.4], vec![0.5, 0.5]],
        );
        assert_eq!(
            recommend_finite_decision_step(
                &actions(),
                &[0.5, 0.5],
                &[vec![1.0, -1.0], vec![0.0, 0.0]],
                0.0,
                &[bad],
                1.0,
                1.0,
                0.0,
            ),
            Err(ModelError::InvalidParameter(
                "information_option_likelihood_row_must_sum_to_one"
            ))
        );
    }

    #[test]
    fn malformed_probabilities_budgets_and_duplicate_options_fail_closed() {
        assert_eq!(
            recommend_finite_decision_step(
                &actions(),
                &[0.0, 0.0],
                &[vec![1.0, 1.0], vec![0.0, 0.0]],
                0.0,
                &[],
                1.0,
                1.0,
                0.0,
            ),
            Err(ModelError::NonPositiveValue("decision_probability_sum"))
        );
        let option = options(
            "duplicate",
            InformationAcquisitionKind::Other,
            0.1,
            vec![vec![0.5, 0.5], vec![0.5, 0.5]],
        );
        assert_eq!(
            recommend_finite_decision_step(
                &actions(),
                &[0.5, 0.5],
                &[vec![1.0, -1.0], vec![0.0, 0.0]],
                0.0,
                &[option.clone(), option],
                1.0,
                1.0,
                0.0,
            ),
            Err(ModelError::InvalidParameter(
                "information_option_ids_must_be_nonempty_and_unique"
            ))
        );
    }

    #[test]
    fn information_cannot_exceed_perfect_information() {
        let fully_informative = options(
            "perfect",
            InformationAcquisitionKind::IndependentCalibration,
            0.0,
            vec![vec![1.0, 0.0], vec![0.0, 1.0]],
        );
        let result = recommend_finite_decision_step(
            &actions(),
            &[0.5, 0.5],
            &[vec![100.0, -30.0], vec![0.0, 0.0]],
            0.0,
            &[fully_informative],
            0.0,
            1.0,
            0.0,
        )
        .expect("perfect information");
        match result {
            FiniteDecisionRecommendation::AcquireInformation {
                expected_value_of_sample_information,
                expected_value_of_perfect_information,
                ..
            } => assert!(
                (expected_value_of_sample_information - expected_value_of_perfect_information)
                    .abs()
                    < 1.0e-12
            ),
            other => panic!("expected perfect information acquisition, got {other:?}"),
        }
    }
}
