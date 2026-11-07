//! Robust intervention selection from frozen model-conditional forecasts.
//!
//! V0 maximizes the worst-pair Gaussian predictive KL per intervention cost.
//! It is a mechanism-discrimination criterion, not exact expected information
//! gain. The caller supplies forecasts made before intervention outcomes are
//! observed and binds them to stable model, candidate, forecast and observable
//! layout digests.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HypothesisPrior {
    pub hypothesis_id: u64,
    pub prior_weight: f64,
    pub model_digest: [u8; 32],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelPrediction {
    pub hypothesis_id: u64,
    pub observable_layout_digest: [u8; 32],
    /// Flattened predictive means in the immutable observable order.
    pub mean: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterventionCandidate {
    pub candidate_id: u64,
    pub cost: f64,
    /// False means a separate safety/transition gate rejected this action.
    pub admissible: bool,
    pub candidate_digest: [u8; 32],
    pub forecast_digest: [u8; 32],
    pub predictions: Vec<ModelPrediction>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DesignConstraints {
    /// Diagonal Gaussian observation variances in the observable layout order.
    pub observation_variances: Vec<f64>,
    pub observable_layout_digest: [u8; 32],
    pub maximum_cost: f64,
    pub minimum_worst_pairwise_kl: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairwiseSeparation {
    pub left_hypothesis_id: u64,
    pub right_hypothesis_id: u64,
    pub gaussian_kl: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CandidateScore {
    pub candidate_id: u64,
    pub cost: f64,
    pub worst_pairwise_kl: f64,
    pub prior_weighted_pairwise_kl: f64,
    pub robust_utility_per_cost: f64,
    pub pairwise_separation: Vec<PairwiseSeparation>,
    pub candidate_digest: [u8; 32],
    pub forecast_digest: [u8; 32],
    pub model_digests: Vec<(u64, [u8; 32])>,
    pub observable_layout_digest: [u8; 32],
}

#[derive(Debug, Clone, PartialEq)]
pub enum DesignDecision {
    Selected {
        score: CandidateScore,
    },
    Abstained {
        closest_candidate: CandidateScore,
        minimum_required_worst_pairwise_kl: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesignError {
    TooFewHypotheses,
    DuplicateHypothesisId,
    InvalidPriorWeight,
    UnnormalizedPrior,
    MissingModelDigest,
    InvalidObservableLayoutDigest,
    InvalidObservationVariances,
    InvalidMaximumCost,
    InvalidMinimumSeparation,
    DuplicateCandidateId,
    InvalidCandidateCost,
    MissingCandidateDigest,
    MissingForecastDigest,
    NoFeasibleCandidate,
    MissingHypothesisForecast,
    DuplicateHypothesisForecast,
    ForecastLayoutMismatch,
    ForecastDimensionMismatch,
    NonFiniteForecast,
    NumericalFailure,
}

impl DesignConstraints {
    pub fn validate(&self) -> Result<(), DesignError> {
        if self.observation_variances.is_empty()
            || !self
                .observation_variances
                .iter()
                .all(|variance| variance.is_finite() && *variance > 0.0)
        {
            return Err(DesignError::InvalidObservationVariances);
        }
        if self.observable_layout_digest.iter().all(|byte| *byte == 0) {
            return Err(DesignError::InvalidObservableLayoutDigest);
        }
        if !self.maximum_cost.is_finite() || self.maximum_cost <= 0.0 {
            return Err(DesignError::InvalidMaximumCost);
        }
        if !self.minimum_worst_pairwise_kl.is_finite() || self.minimum_worst_pairwise_kl < 0.0 {
            return Err(DesignError::InvalidMinimumSeparation);
        }
        Ok(())
    }
}

/// Select an admissible intervention by maximin pairwise predictive separation
/// per unit cost. All hypotheses have positive prior mass, so the worst-pair
/// criterion protects low-probability but still plausible explanations.
///
/// For equal diagonal Gaussian covariance, pairwise KL is:
///
/// ```text
/// 0.5 * sum_k((mean_i[k] - mean_j[k])^2 / variance[k])
/// ```
///
/// The expected diagnostic is prior weighted over unordered pairs, with pair
/// weights normalized to sum to one. Neither quantity is called Bayesian EIG.
pub fn select_intervention(
    hypotheses: &[HypothesisPrior],
    candidates: &[InterventionCandidate],
    constraints: &DesignConstraints,
) -> Result<DesignDecision, DesignError> {
    validate_hypotheses(hypotheses)?;
    constraints.validate()?;
    validate_candidate_catalog(candidates)?;

    let mut scores = Vec::new();
    for candidate in candidates {
        if !candidate.admissible || candidate.cost > constraints.maximum_cost {
            continue;
        }
        scores.push(score_candidate(candidate, hypotheses, constraints)?);
    }
    if scores.is_empty() {
        return Err(DesignError::NoFeasibleCandidate);
    }

    scores.sort_by(|left, right| {
        right
            .robust_utility_per_cost
            .total_cmp(&left.robust_utility_per_cost)
            .then_with(|| left.candidate_id.cmp(&right.candidate_id))
    });
    let best = scores.remove(0);

    if best.worst_pairwise_kl < constraints.minimum_worst_pairwise_kl {
        Ok(DesignDecision::Abstained {
            closest_candidate: best,
            minimum_required_worst_pairwise_kl: constraints.minimum_worst_pairwise_kl,
        })
    } else {
        Ok(DesignDecision::Selected { score: best })
    }
}

fn validate_hypotheses(hypotheses: &[HypothesisPrior]) -> Result<(), DesignError> {
    if hypotheses.len() < 2 {
        return Err(DesignError::TooFewHypotheses);
    }

    let mut seen = BTreeSet::new();
    let mut total_weight = 0.0;
    for hypothesis in hypotheses {
        if !seen.insert(hypothesis.hypothesis_id) {
            return Err(DesignError::DuplicateHypothesisId);
        }
        if !hypothesis.prior_weight.is_finite() || hypothesis.prior_weight <= 0.0 {
            return Err(DesignError::InvalidPriorWeight);
        }
        if hypothesis.model_digest.iter().all(|byte| *byte == 0) {
            return Err(DesignError::MissingModelDigest);
        }
        total_weight += hypothesis.prior_weight;
    }
    if !total_weight.is_finite() || (total_weight - 1.0).abs() > 1.0e-9 {
        return Err(DesignError::UnnormalizedPrior);
    }
    Ok(())
}

fn validate_candidate_catalog(candidates: &[InterventionCandidate]) -> Result<(), DesignError> {
    let mut seen = BTreeSet::new();
    for candidate in candidates {
        if !seen.insert(candidate.candidate_id) {
            return Err(DesignError::DuplicateCandidateId);
        }
        if !candidate.cost.is_finite() || candidate.cost <= 0.0 {
            return Err(DesignError::InvalidCandidateCost);
        }
        if candidate.candidate_digest.iter().all(|byte| *byte == 0) {
            return Err(DesignError::MissingCandidateDigest);
        }
        if candidate.forecast_digest.iter().all(|byte| *byte == 0) {
            return Err(DesignError::MissingForecastDigest);
        }
    }
    Ok(())
}

fn score_candidate(
    candidate: &InterventionCandidate,
    hypotheses: &[HypothesisPrior],
    constraints: &DesignConstraints,
) -> Result<CandidateScore, DesignError> {
    let mut predictions = BTreeMap::new();
    for prediction in &candidate.predictions {
        if predictions
            .insert(prediction.hypothesis_id, prediction)
            .is_some()
        {
            return Err(DesignError::DuplicateHypothesisForecast);
        }
        if prediction.observable_layout_digest != constraints.observable_layout_digest {
            return Err(DesignError::ForecastLayoutMismatch);
        }
        if prediction.mean.len() != constraints.observation_variances.len() {
            return Err(DesignError::ForecastDimensionMismatch);
        }
        if !prediction.mean.iter().all(|value| value.is_finite()) {
            return Err(DesignError::NonFiniteForecast);
        }
    }

    if predictions.len() != hypotheses.len() {
        return Err(DesignError::MissingHypothesisForecast);
    }

    let mut ordered_hypotheses = hypotheses.to_vec();
    ordered_hypotheses.sort_by_key(|hypothesis| hypothesis.hypothesis_id);

    let mut pairwise_separation =
        Vec::with_capacity(ordered_hypotheses.len() * (ordered_hypotheses.len() - 1) / 2);
    let mut worst_pairwise_kl = f64::INFINITY;
    let mut weighted_kl_sum = 0.0;
    let mut pair_weight_sum = 0.0;

    for left_index in 0..ordered_hypotheses.len() {
        for right_index in (left_index + 1)..ordered_hypotheses.len() {
            let left = ordered_hypotheses[left_index];
            let right = ordered_hypotheses[right_index];
            let left_prediction = predictions
                .get(&left.hypothesis_id)
                .ok_or(DesignError::MissingHypothesisForecast)?;
            let right_prediction = predictions
                .get(&right.hypothesis_id)
                .ok_or(DesignError::MissingHypothesisForecast)?;
            let gaussian_kl = diagonal_gaussian_kl(
                &left_prediction.mean,
                &right_prediction.mean,
                &constraints.observation_variances,
            )?;

            worst_pairwise_kl = worst_pairwise_kl.min(gaussian_kl);
            let pair_weight = 2.0 * left.prior_weight * right.prior_weight;
            weighted_kl_sum += pair_weight * gaussian_kl;
            pair_weight_sum += pair_weight;
            pairwise_separation.push(PairwiseSeparation {
                left_hypothesis_id: left.hypothesis_id,
                right_hypothesis_id: right.hypothesis_id,
                gaussian_kl,
            });
        }
    }

    let prior_weighted_pairwise_kl = weighted_kl_sum / pair_weight_sum;
    let robust_utility_per_cost = worst_pairwise_kl / candidate.cost;
    if !worst_pairwise_kl.is_finite()
        || !prior_weighted_pairwise_kl.is_finite()
        || !robust_utility_per_cost.is_finite()
    {
        return Err(DesignError::NumericalFailure);
    }

    Ok(CandidateScore {
        candidate_id: candidate.candidate_id,
        cost: candidate.cost,
        worst_pairwise_kl,
        prior_weighted_pairwise_kl,
        robust_utility_per_cost,
        pairwise_separation,
        candidate_digest: candidate.candidate_digest,
        forecast_digest: candidate.forecast_digest,
        model_digests: ordered_hypotheses
            .iter()
            .map(|hypothesis| (hypothesis.hypothesis_id, hypothesis.model_digest))
            .collect(),
        observable_layout_digest: constraints.observable_layout_digest,
    })
}

fn diagonal_gaussian_kl(
    left_mean: &[f64],
    right_mean: &[f64],
    variances: &[f64],
) -> Result<f64, DesignError> {
    if left_mean.len() != right_mean.len() || left_mean.len() != variances.len() {
        return Err(DesignError::ForecastDimensionMismatch);
    }

    let mut total = 0.0;
    for ((left, right), variance) in left_mean.iter().zip(right_mean).zip(variances) {
        let difference = left - right;
        total += difference * difference / (2.0 * variance);
        if !total.is_finite() {
            return Err(DesignError::NumericalFailure);
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(seed: u8) -> [u8; 32] {
        [seed; 32]
    }

    fn hypotheses() -> Vec<HypothesisPrior> {
        vec![
            HypothesisPrior {
                hypothesis_id: 1,
                prior_weight: 0.5,
                model_digest: digest(1),
            },
            HypothesisPrior {
                hypothesis_id: 2,
                prior_weight: 0.5,
                model_digest: digest(2),
            },
        ]
    }

    fn constraints() -> DesignConstraints {
        DesignConstraints {
            observation_variances: vec![1.0, 1.0, 1.0],
            observable_layout_digest: digest(90),
            maximum_cost: 10.0,
            minimum_worst_pairwise_kl: 0.01,
        }
    }

    fn candidate(
        id: u64,
        cost: f64,
        admissible: bool,
        left: &[f64],
        right: &[f64],
    ) -> InterventionCandidate {
        InterventionCandidate {
            candidate_id: id,
            cost,
            admissible,
            candidate_digest: digest(id as u8 + 10),
            forecast_digest: digest(id as u8 + 20),
            predictions: vec![
                ModelPrediction {
                    hypothesis_id: 1,
                    observable_layout_digest: digest(90),
                    mean: left.to_vec(),
                },
                ModelPrediction {
                    hypothesis_id: 2,
                    observable_layout_digest: digest(90),
                    mean: right.to_vec(),
                },
            ],
        }
    }

    #[test]
    fn baseline_observational_equivalence_mandates_abstention() {
        let candidates = vec![candidate(1, 1.0, true, &[0.0, 1.0, 0.0], &[0.0, 1.0, 0.0])];
        let decision = select_intervention(&hypotheses(), &candidates, &constraints()).unwrap();
        assert!(matches!(
            decision,
            DesignDecision::Abstained {
                closest_candidate: CandidateScore {
                    worst_pairwise_kl: 0.0,
                    ..
                },
                ..
            }
        ));
    }

    #[test]
    fn selector_chooses_phase_pulse_that_separates_independent_and_coupled_models() {
        let candidates = vec![
            candidate(1, 1.0, true, &[0.0, 0.0, 0.0], &[0.0, 0.0, 0.0]),
            candidate(2, 2.0, true, &[1.0, 1.0, 1.0], &[0.0, 0.0, 0.0]),
        ];
        let decision = select_intervention(&hypotheses(), &candidates, &constraints()).unwrap();
        let DesignDecision::Selected { score } = decision else {
            panic!("a discriminating intervention should be selected");
        };
        assert_eq!(score.candidate_id, 2);
        assert!((score.worst_pairwise_kl - 1.5).abs() < 1.0e-12);
        assert_eq!(score.candidate_digest, digest(12));
        assert_eq!(score.forecast_digest, digest(22));
        assert_eq!(score.model_digests.len(), 2);
    }

    #[test]
    fn robust_score_uses_the_least_distinguishable_pair_not_the_mean() {
        let hypotheses = vec![
            HypothesisPrior {
                hypothesis_id: 1,
                prior_weight: 0.8,
                model_digest: digest(1),
            },
            HypothesisPrior {
                hypothesis_id: 2,
                prior_weight: 0.1,
                model_digest: digest(2),
            },
            HypothesisPrior {
                hypothesis_id: 3,
                prior_weight: 0.1,
                model_digest: digest(3),
            },
        ];
        let candidate = InterventionCandidate {
            candidate_id: 1,
            cost: 1.0,
            admissible: true,
            candidate_digest: digest(11),
            forecast_digest: digest(21),
            predictions: vec![
                ModelPrediction {
                    hypothesis_id: 1,
                    observable_layout_digest: digest(90),
                    mean: vec![0.0],
                },
                ModelPrediction {
                    hypothesis_id: 2,
                    observable_layout_digest: digest(90),
                    mean: vec![10.0],
                },
                ModelPrediction {
                    hypothesis_id: 3,
                    observable_layout_digest: digest(90),
                    mean: vec![10.01],
                },
            ],
        };
        let mut constraints = constraints();
        constraints.observation_variances = vec![1.0];
        let decision = select_intervention(&hypotheses, &[candidate], &constraints).unwrap();
        let DesignDecision::Abstained {
            closest_candidate, ..
        } = decision
        else {
            panic!("the weakly separated pair must force abstention");
        };
        assert!(closest_candidate.worst_pairwise_kl < 0.001);
        assert!(closest_candidate.prior_weighted_pairwise_kl > 1.0);
    }

    #[test]
    fn budget_and_admissibility_are_enforced_before_scoring() {
        let candidates = vec![
            candidate(1, 1.0, false, &[0.0, 0.0, 0.0], &[2.0, 2.0, 2.0]),
            candidate(2, 11.0, true, &[0.0, 0.0, 0.0], &[2.0, 2.0, 2.0]),
        ];
        assert_eq!(
            select_intervention(&hypotheses(), &candidates, &constraints()),
            Err(DesignError::NoFeasibleCandidate)
        );
    }

    #[test]
    fn lower_candidate_id_breaks_exact_utility_ties() {
        let candidates = vec![
            candidate(9, 1.0, true, &[1.0, 0.0, 0.0], &[0.0, 0.0, 0.0]),
            candidate(3, 1.0, true, &[0.0, 1.0, 0.0], &[0.0, 0.0, 0.0]),
        ];
        let decision = select_intervention(&hypotheses(), &candidates, &constraints()).unwrap();
        let DesignDecision::Selected { score } = decision else {
            panic!("separating candidates should be selected");
        };
        assert_eq!(score.candidate_id, 3);
    }

    #[test]
    fn prior_weighted_pairwise_score_matches_closed_form() {
        let hypotheses = vec![
            HypothesisPrior {
                hypothesis_id: 1,
                prior_weight: 0.75,
                model_digest: digest(1),
            },
            HypothesisPrior {
                hypothesis_id: 2,
                prior_weight: 0.25,
                model_digest: digest(2),
            },
        ];
        let candidate = candidate(1, 1.0, true, &[0.0, 0.0, 0.0], &[2.0, 0.0, 0.0]);
        let decision = select_intervention(&hypotheses, &[candidate], &constraints()).unwrap();
        let DesignDecision::Selected { score } = decision else {
            panic!("candidate should be selected");
        };
        assert!((score.prior_weighted_pairwise_kl - 2.0).abs() < 1.0e-12);
    }

    #[test]
    fn candidate_cost_changes_robust_utility_per_cost() {
        let candidates = vec![
            candidate(1, 1.0, true, &[0.0, 0.0, 0.0], &[1.0, 0.0, 0.0]),
            candidate(2, 4.0, true, &[0.0, 0.0, 0.0], &[2.0, 0.0, 0.0]),
        ];
        let decision = select_intervention(&hypotheses(), &candidates, &constraints()).unwrap();
        let DesignDecision::Selected { score } = decision else {
            panic!("one candidate should be selected");
        };
        assert_eq!(score.candidate_id, 1);
    }

    #[test]
    fn duplicate_hypothesis_and_candidate_ids_fail_closed() {
        let mut duplicate_hypotheses = hypotheses();
        duplicate_hypotheses[1].hypothesis_id = duplicate_hypotheses[0].hypothesis_id;
        assert_eq!(
            select_intervention(&duplicate_hypotheses, &[], &constraints()),
            Err(DesignError::DuplicateHypothesisId)
        );

        let first = candidate(1, 1.0, true, &[0.0, 0.0, 0.0], &[1.0, 0.0, 0.0]);
        let mut second = first.clone();
        second.candidate_id = first.candidate_id;
        assert_eq!(
            select_intervention(&hypotheses(), &[first, second], &constraints()),
            Err(DesignError::DuplicateCandidateId)
        );
    }

    #[test]
    fn missing_duplicate_or_misaligned_model_forecasts_fail_closed() {
        let mut missing = candidate(1, 1.0, true, &[0.0, 0.0, 0.0], &[1.0, 0.0, 0.0]);
        missing.predictions.pop();
        assert_eq!(
            select_intervention(&hypotheses(), &[missing], &constraints()),
            Err(DesignError::MissingHypothesisForecast)
        );

        let mut duplicate = candidate(2, 1.0, true, &[0.0, 0.0, 0.0], &[1.0, 0.0, 0.0]);
        duplicate.predictions[1].hypothesis_id = 1;
        assert_eq!(
            select_intervention(&hypotheses(), &[duplicate], &constraints()),
            Err(DesignError::DuplicateHypothesisForecast)
        );

        let mut misaligned = candidate(3, 1.0, true, &[0.0, 0.0, 0.0], &[1.0, 0.0, 0.0]);
        misaligned.predictions[1].observable_layout_digest = digest(91);
        assert_eq!(
            select_intervention(&hypotheses(), &[misaligned], &constraints()),
            Err(DesignError::ForecastLayoutMismatch)
        );
    }

    #[test]
    fn invalid_priors_variance_budget_and_digests_fail_closed() {
        let mut invalid_hypotheses = hypotheses();
        invalid_hypotheses[0].prior_weight = f64::NAN;
        assert_eq!(
            select_intervention(&invalid_hypotheses, &[], &constraints()),
            Err(DesignError::InvalidPriorWeight)
        );

        let mut invalid_constraints = constraints();
        invalid_constraints.observation_variances[0] = 0.0;
        assert_eq!(
            select_intervention(&hypotheses(), &[], &invalid_constraints),
            Err(DesignError::InvalidObservationVariances)
        );

        let mut invalid_candidate = candidate(1, 1.0, true, &[0.0, 0.0, 0.0], &[1.0, 0.0, 0.0]);
        invalid_candidate.candidate_digest = [0; 32];
        assert_eq!(
            select_intervention(&hypotheses(), &[invalid_candidate], &constraints()),
            Err(DesignError::MissingCandidateDigest)
        );
    }

    #[test]
    fn mismatched_forecast_dimensions_and_nonfinite_means_fail_closed() {
        let mut dimensions = candidate(1, 1.0, true, &[0.0, 0.0, 0.0], &[1.0, 0.0, 0.0]);
        dimensions.predictions[0].mean.pop();
        assert_eq!(
            select_intervention(&hypotheses(), &[dimensions], &constraints()),
            Err(DesignError::ForecastDimensionMismatch)
        );

        let mut nonfinite = candidate(2, 1.0, true, &[0.0, 0.0, 0.0], &[1.0, 0.0, 0.0]);
        nonfinite.predictions[0].mean[0] = f64::INFINITY;
        assert_eq!(
            select_intervention(&hypotheses(), &[nonfinite], &constraints()),
            Err(DesignError::NonFiniteForecast)
        );
    }

    #[test]
    fn no_feasible_candidate_and_too_few_hypotheses_are_errors_not_guesses() {
        assert_eq!(
            select_intervention(&hypotheses()[..1], &[], &constraints()),
            Err(DesignError::TooFewHypotheses)
        );
        assert_eq!(
            select_intervention(&hypotheses(), &[], &constraints()),
            Err(DesignError::NoFeasibleCandidate)
        );
    }
}
