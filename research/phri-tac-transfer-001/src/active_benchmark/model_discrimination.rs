//! Synthetic model-discriminating experiment scoring for tactile calibration.
//!
//! This module ranks candidate stimuli by how well their posterior-predictive
//! distributions distinguish competing response-model hypotheses. It is
//! complementary to D-optimal parameter design: D-optimality is conditional on
//! a selected model, while this module scores uncertainty over model identity.
//!
//! Each predictive standard deviation must already include the caller's
//! parameter uncertainty and observation-noise assumptions. This module does
//! not fit physical models, establish reference independence, prove model
//! completeness, or authorize hardware actions. It is a deterministic
//! numerical research primitive, not a metrological qualification method.

#![forbid(unsafe_code)]

const QUADRATURE_BINS: usize = 1024;
const GAUSSIAN_TAIL_SIGMAS: f64 = 8.0;
const SQRT_TWO_PI: f64 = 2.506_628_274_631_000_2;
const LN_TWO: f64 = std::f64::consts::LN_2;
const PRIOR_COMPARISON_TOLERANCE: f64 = 1.0e-12;

#[derive(Clone, Debug, PartialEq)]
pub struct GaussianModelPrediction {
    pub model_id: String,
    /// Unnormalized prior model weight. Weights are normalized per action.
    pub prior_weight: f64,
    /// Posterior-predictive mean for the candidate stimulus.
    pub predictive_mean: f64,
    /// Posterior-predictive standard deviation, including parameter and
    /// observation uncertainty as calculated by the caller.
    pub predictive_std_dev: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidateActionForecast {
    pub action_id: String,
    pub model_predictions: Vec<GaussianModelPrediction>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModelDiscriminationScore {
    pub action_id: String,
    pub model_count: usize,
    pub prior_model_entropy_nats: f64,
    pub expected_information_gain_nats: f64,
    pub minimum_pairwise_js_divergence_nats: f64,
    pub identifiability_floor_nats: f64,
    /// False means at least one pair of candidate models remains too close
    /// under this action. It does not mean that either model is correct.
    pub pairwise_identifiable: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct NormalQuadrature {
    standardized_values: Vec<f64>,
    normalized_weights: Vec<f64>,
}

fn validate_action(action: &CandidateActionForecast) -> Result<(), String> {
    if action.action_id.trim().is_empty() {
        return Err("candidate action ID must not be empty".to_owned());
    }
    if action.model_predictions.len() < 2 {
        return Err("model discrimination requires at least two hypotheses".to_owned());
    }

    for prediction in &action.model_predictions {
        if prediction.model_id.trim().is_empty() {
            return Err("model hypothesis ID must not be empty".to_owned());
        }
        if !prediction.prior_weight.is_finite() || prediction.prior_weight <= 0.0 {
            return Err("model prior weights must be finite and positive".to_owned());
        }
        if !prediction.predictive_mean.is_finite() {
            return Err("predictive means must be finite".to_owned());
        }
        if !prediction.predictive_std_dev.is_finite() || prediction.predictive_std_dev <= 0.0 {
            return Err("predictive standard deviations must be finite and positive".to_owned());
        }
    }

    for left in 0..action.model_predictions.len() {
        for right in (left + 1)..action.model_predictions.len() {
            if action.model_predictions[left].model_id == action.model_predictions[right].model_id {
                return Err("candidate action contains duplicate model IDs".to_owned());
            }
        }
    }
    Ok(())
}

fn prior_signature(action: &CandidateActionForecast) -> Result<Vec<(String, f64)>, String> {
    let total = action
        .model_predictions
        .iter()
        .map(|prediction| prediction.prior_weight)
        .sum::<f64>();
    if !total.is_finite() || total <= 0.0 {
        return Err("model prior total must be finite and positive".to_owned());
    }
    let mut signature: Vec<_> = action
        .model_predictions
        .iter()
        .map(|prediction| (prediction.model_id.clone(), prediction.prior_weight / total))
        .collect();
    if signature
        .iter()
        .any(|(_, prior)| !prior.is_finite() || *prior <= 0.0)
    {
        return Err("normalized model prior weights must remain finite and positive".to_owned());
    }
    signature.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(signature)
}

fn normalized_priors(predictions: &[GaussianModelPrediction]) -> Result<Vec<f64>, String> {
    let total = predictions
        .iter()
        .map(|prediction| prediction.prior_weight)
        .sum::<f64>();
    if !total.is_finite() || total <= 0.0 {
        return Err("model prior total must be finite and positive".to_owned());
    }
    let priors: Vec<_> = predictions
        .iter()
        .map(|prediction| prediction.prior_weight / total)
        .collect();
    if priors
        .iter()
        .any(|prior| !prior.is_finite() || *prior <= 0.0)
    {
        return Err("normalized model prior weights must remain finite and positive".to_owned());
    }
    Ok(priors)
}

fn standard_normal_quadrature() -> NormalQuadrature {
    // Integrate over each model's own standardized coordinate. This avoids a
    // shared physical-support grid missing a narrow distribution when model
    // predictive variances differ substantially.
    let step = (2.0 * GAUSSIAN_TAIL_SIGMAS) / QUADRATURE_BINS as f64;
    let mut standardized_values = Vec::with_capacity(QUADRATURE_BINS);
    let mut normalized_weights = Vec::with_capacity(QUADRATURE_BINS);
    let mut weight_total = 0.0;
    for index in 0..QUADRATURE_BINS {
        let z = -GAUSSIAN_TAIL_SIGMAS + (index as f64 + 0.5) * step;
        let weight = (-0.5 * z * z).exp() / SQRT_TWO_PI * step;
        standardized_values.push(z);
        normalized_weights.push(weight);
        weight_total += weight;
    }
    for weight in &mut normalized_weights {
        *weight /= weight_total;
    }
    NormalQuadrature {
        standardized_values,
        normalized_weights,
    }
}

fn normal_log_density(observation: f64, prediction: &GaussianModelPrediction) -> f64 {
    let standardized = (observation - prediction.predictive_mean) / prediction.predictive_std_dev;
    -prediction.predictive_std_dev.ln() - 0.5 * standardized * standardized - SQRT_TWO_PI.ln()
}

fn log_sum_exp(values: &[f64]) -> f64 {
    let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if maximum == f64::NEG_INFINITY {
        return f64::NEG_INFINITY;
    }
    maximum
        + values
            .iter()
            .map(|value| (value - maximum).exp())
            .sum::<f64>()
            .ln()
}

fn log_add_exp(left: f64, right: f64) -> f64 {
    let maximum = left.max(right);
    if maximum == f64::NEG_INFINITY {
        return f64::NEG_INFINITY;
    }
    maximum + ((left - maximum).exp() + (right - maximum).exp()).ln()
}

fn entropy(probabilities: &[f64]) -> f64 {
    probabilities
        .iter()
        .map(|probability| -probability * probability.ln())
        .sum()
}

fn expected_information_gain(
    predictions: &[GaussianModelPrediction],
    priors: &[f64],
    quadrature: &NormalQuadrature,
) -> f64 {
    let mut information_gain = 0.0;
    for (model_index, prediction) in predictions.iter().enumerate() {
        let mut conditional_kl = 0.0;
        let log_prior_terms: Vec<_> = predictions
            .iter()
            .zip(priors)
            .map(|(model, prior)| (model, prior.ln()))
            .collect();
        for (z, weight) in quadrature
            .standardized_values
            .iter()
            .zip(&quadrature.normalized_weights)
        {
            let observation = prediction.predictive_mean + prediction.predictive_std_dev * z;
            let own_log_density = normal_log_density(observation, prediction);
            let mixture_terms: Vec<_> = log_prior_terms
                .iter()
                .map(|(model, log_prior)| log_prior + normal_log_density(observation, model))
                .collect();
            let log_mixture_density = log_sum_exp(&mixture_terms);
            conditional_kl += weight * (own_log_density - log_mixture_density);
        }
        information_gain += priors[model_index] * conditional_kl;
    }
    information_gain.max(0.0)
}

fn pairwise_js_divergence(
    left: &GaussianModelPrediction,
    right: &GaussianModelPrediction,
    quadrature: &NormalQuadrature,
) -> f64 {
    let mut left_expectation = 0.0;
    for (z, weight) in quadrature
        .standardized_values
        .iter()
        .zip(&quadrature.normalized_weights)
    {
        let observation = left.predictive_mean + left.predictive_std_dev * z;
        let left_log_density = normal_log_density(observation, left);
        let right_log_density = normal_log_density(observation, right);
        let log_midpoint = log_add_exp(left_log_density, right_log_density) - LN_TWO;
        left_expectation += weight * (left_log_density - log_midpoint);
    }

    let mut right_expectation = 0.0;
    for (z, weight) in quadrature
        .standardized_values
        .iter()
        .zip(&quadrature.normalized_weights)
    {
        let observation = right.predictive_mean + right.predictive_std_dev * z;
        let left_log_density = normal_log_density(observation, left);
        let right_log_density = normal_log_density(observation, right);
        let log_midpoint = log_add_exp(left_log_density, right_log_density) - LN_TWO;
        right_expectation += weight * (right_log_density - log_midpoint);
    }

    (0.5 * (left_expectation + right_expectation)).max(0.0)
}

/// Score a single candidate action using deterministic numerical integration
/// over each Gaussian model's standardized predictive distribution.
///
/// Expected information gain is measured in nats. The minimum pairwise
/// Jensen–Shannon divergence is a conservative guard: a stimulus is not
/// called pairwise-identifying when even one currently plausible model pair
/// remains below the declared separation floor.
pub fn score_candidate_action(
    action: &CandidateActionForecast,
    identifiability_floor_nats: f64,
) -> Result<ModelDiscriminationScore, String> {
    validate_action(action)?;
    if !identifiability_floor_nats.is_finite() || identifiability_floor_nats < 0.0 {
        return Err("identifiability floor must be finite and nonnegative".to_owned());
    }

    let priors = normalized_priors(&action.model_predictions)?;
    let quadrature = standard_normal_quadrature();
    let model_count = action.model_predictions.len();
    let prior_model_entropy_nats = entropy(&priors);

    let expected_information_gain_nats =
        expected_information_gain(&action.model_predictions, &priors, &quadrature)
            .min(prior_model_entropy_nats);
    let mut minimum_pairwise_js_divergence_nats = f64::INFINITY;
    for left in 0..model_count {
        for right in (left + 1)..model_count {
            let divergence = pairwise_js_divergence(
                &action.model_predictions[left],
                &action.model_predictions[right],
                &quadrature,
            );
            minimum_pairwise_js_divergence_nats =
                minimum_pairwise_js_divergence_nats.min(divergence);
        }
    }

    if !expected_information_gain_nats.is_finite()
        || !minimum_pairwise_js_divergence_nats.is_finite()
    {
        return Err("model-discrimination score is non-finite".to_owned());
    }

    Ok(ModelDiscriminationScore {
        action_id: action.action_id.clone(),
        model_count,
        prior_model_entropy_nats,
        expected_information_gain_nats,
        minimum_pairwise_js_divergence_nats,
        identifiability_floor_nats,
        pairwise_identifiable: minimum_pairwise_js_divergence_nats >= identifiability_floor_nats,
    })
}

/// Score and deterministically rank candidate actions. All actions must refer
/// to the same model IDs and normalized prior weights so their scores are
/// comparable. Ranking is conservative: minimum pairwise separation first,
/// then expected information gain, then lexical action ID.
pub fn rank_candidate_actions(
    actions: &[CandidateActionForecast],
    identifiability_floor_nats: f64,
) -> Result<Vec<ModelDiscriminationScore>, String> {
    if actions.is_empty() {
        return Err("at least one candidate action is required".to_owned());
    }

    let mut action_ids = Vec::with_capacity(actions.len());
    for action in actions {
        validate_action(action)?;
        if action_ids
            .iter()
            .any(|existing| existing == &action.action_id)
        {
            return Err("candidate action IDs must be unique".to_owned());
        }
        action_ids.push(action.action_id.clone());
    }

    let expected_signature = prior_signature(&actions[0])?;
    for action in actions.iter().skip(1) {
        let signature = prior_signature(action)?;
        if signature.len() != expected_signature.len()
            || signature
                .iter()
                .zip(&expected_signature)
                .any(|(left, right)| {
                    left.0 != right.0 || (left.1 - right.1).abs() > PRIOR_COMPARISON_TOLERANCE
                })
        {
            return Err(
                "candidate actions must use the same model IDs and normalized priors".to_owned(),
            );
        }
    }

    let mut scores: Vec<_> = actions
        .iter()
        .map(|action| score_candidate_action(action, identifiability_floor_nats))
        .collect::<Result<_, _>>()?;
    scores.sort_by(|left, right| {
        right
            .minimum_pairwise_js_divergence_nats
            .total_cmp(&left.minimum_pairwise_js_divergence_nats)
            .then_with(|| {
                right
                    .expected_information_gain_nats
                    .total_cmp(&left.expected_information_gain_nats)
            })
            .then_with(|| left.action_id.cmp(&right.action_id))
    });
    Ok(scores)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(action_id: &str, predictions: &[(&str, f64, f64, f64)]) -> CandidateActionForecast {
        CandidateActionForecast {
            action_id: action_id.to_owned(),
            model_predictions: predictions
                .iter()
                .map(
                    |(model_id, prior_weight, mean, std_dev)| GaussianModelPrediction {
                        model_id: (*model_id).to_owned(),
                        prior_weight: *prior_weight,
                        predictive_mean: *mean,
                        predictive_std_dev: *std_dev,
                    },
                )
                .collect(),
        }
    }

    #[test]
    fn identical_models_are_not_declared_identifiable() {
        let candidate = action(
            "same-observation",
            &[("rational", 1.0, 2.0, 0.2), ("cubic", 1.0, 2.0, 0.2)],
        );
        let score = score_candidate_action(&candidate, 0.01).expect("score");
        assert!(score.expected_information_gain_nats < 1.0e-10);
        assert!(score.minimum_pairwise_js_divergence_nats < 1.0e-10);
        assert!(!score.pairwise_identifiable);
    }

    #[test]
    fn separated_models_produce_information_gain_and_pairwise_separation() {
        let candidate = action(
            "high-temperature-ramp",
            &[
                ("rational", 1.0, 1.0, 0.1),
                ("force-temperature", 1.0, 2.0, 0.1),
            ],
        );
        let score = score_candidate_action(&candidate, 0.05).expect("score");
        assert!(score.expected_information_gain_nats > 0.5);
        assert!(score.minimum_pairwise_js_divergence_nats > 0.5);
        assert!(score.pairwise_identifiable);
        assert!(score.expected_information_gain_nats <= score.prior_model_entropy_nats + 1.0e-9);
    }

    #[test]
    fn weakest_pair_controls_the_robust_identifiability_decision() {
        let candidate = action(
            "ordinary-force-ramp",
            &[
                ("rational", 1.0, 0.0, 0.1),
                ("cubic", 1.0, 0.8, 0.1),
                ("history-interaction", 1.0, 0.02, 0.1),
            ],
        );
        let score = score_candidate_action(&candidate, 0.01).expect("score");
        assert!(score.expected_information_gain_nats > 0.05);
        assert!(score.minimum_pairwise_js_divergence_nats < 0.01);
        assert!(!score.pairwise_identifiable);
    }

    #[test]
    fn candidate_ranking_prefers_the_action_that_separates_all_model_pairs() {
        let ordinary = action(
            "ordinary",
            &[
                ("rational", 1.0, 0.0, 0.1),
                ("cubic", 1.0, 0.02, 0.1),
                ("history", 1.0, 0.8, 0.1),
            ],
        );
        let diagnostic = action(
            "diagnostic",
            &[
                ("rational", 1.0, 0.0, 0.1),
                ("cubic", 1.0, 0.5, 0.1),
                ("history", 1.0, 1.0, 0.1),
            ],
        );
        let scores = rank_candidate_actions(&[ordinary, diagnostic], 0.05).expect("rank");
        assert_eq!(scores[0].action_id, "diagnostic");
        assert!(scores[0].pairwise_identifiable);
        assert!(!scores[1].pairwise_identifiable);
    }

    #[test]
    fn prior_weights_are_normalized_and_scaling_does_not_change_the_score() {
        let first = action(
            "test",
            &[("rational", 1.0, 0.0, 0.2), ("cubic", 2.0, 0.5, 0.2)],
        );
        let second = action(
            "test",
            &[("rational", 10.0, 0.0, 0.2), ("cubic", 20.0, 0.5, 0.2)],
        );
        let left = score_candidate_action(&first, 0.01).expect("left");
        let right = score_candidate_action(&second, 0.01).expect("right");
        assert!(
            (left.expected_information_gain_nats - right.expected_information_gain_nats).abs()
                < 1.0e-12
        );
        assert!(
            (left.minimum_pairwise_js_divergence_nats - right.minimum_pairwise_js_divergence_nats)
                .abs()
                < 1.0e-12
        );
    }

    #[test]
    fn invalid_forecasts_fail_closed() {
        let zero_variance = action(
            "invalid",
            &[("rational", 1.0, 0.0, 0.0), ("cubic", 1.0, 0.5, 0.1)],
        );
        assert!(score_candidate_action(&zero_variance, 0.01).is_err());

        let duplicate_model = action(
            "duplicate",
            &[("rational", 1.0, 0.0, 0.1), ("rational", 1.0, 0.5, 0.1)],
        );
        assert!(score_candidate_action(&duplicate_model, 0.01).is_err());
    }

    #[test]
    fn candidate_actions_must_share_the_same_model_priors() {
        let first = action(
            "first",
            &[("rational", 1.0, 0.0, 0.1), ("cubic", 1.0, 0.5, 0.1)],
        );
        let second = action(
            "second",
            &[("rational", 1.0, 0.0, 0.1), ("cubic", 3.0, 0.5, 0.1)],
        );
        assert!(rank_candidate_actions(&[first, second], 0.01).is_err());
    }

    #[test]
    fn different_predictive_scales_remain_numerically_resolvable() {
        let candidate = action(
            "mixed-scale",
            &[("narrow", 1.0, 0.0, 1.0e-4), ("broad", 1.0, 1.0, 1.0)],
        );
        let score = score_candidate_action(&candidate, 0.01).expect("score");
        assert!(score.expected_information_gain_nats.is_finite());
        assert!(score.minimum_pairwise_js_divergence_nats.is_finite());
        assert!(score.minimum_pairwise_js_divergence_nats > 0.1);
    }
}
