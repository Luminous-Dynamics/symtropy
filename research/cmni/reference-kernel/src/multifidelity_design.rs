//! Research-only multifidelity and robust-design primitives.
//!
//! These scalar/statistical building blocks are not a complete Bayesian EIG
//! estimator. A fixed control-variate coefficient must be selected from an
//! independent pilot or analytic covariance, not estimated from the same
//! production high-fidelity sample used in the estimator.

use crate::ModelError;

#[derive(Debug, Clone, PartialEq)]
pub struct ControlVariateEstimate {
    pub estimate: f64,
    pub beta: f64,
    pub estimated_variance: f64,
    pub baseline_high_fidelity_variance: f64,
    pub variance_reduction_factor: Option<f64>,
    pub high_fidelity_samples: usize,
    pub low_fidelity_samples: usize,
}

/// Nested two-level approximate-control-variate estimate of a scalar mean.
///
/// Paired high/low values must use common parameter/observation randomness.
/// Extra low-fidelity draws make the low-fidelity mean cheaper to estimate.
/// The beta coefficient must be frozen independently of production samples.
///
/// Estimator: mean(H_paired) + beta * (mean(L_paired) - mean(L_all)).
///
/// Variance uses the nested-sample covariance identity. It measures sampling
/// variance only; it is not a model-discrepancy or bias bound.
pub fn nested_control_variate_estimate(
    high_fidelity_paired: &[f64],
    low_fidelity_paired: &[f64],
    low_fidelity_extra: &[f64],
    beta: f64,
) -> Result<ControlVariateEstimate, ModelError> {
    if high_fidelity_paired.len() < 2
        || high_fidelity_paired.len() != low_fidelity_paired.len()
        || low_fidelity_extra.is_empty()
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    if !beta.is_finite() {
        return Err(ModelError::NonFiniteValue("control_variate_beta"));
    }
    for (values, name) in [
        (high_fidelity_paired, "high_fidelity_samples"),
        (low_fidelity_paired, "low_fidelity_paired_samples"),
        (low_fidelity_extra, "low_fidelity_extra_samples"),
    ] {
        if values.iter().any(|value| !value.is_finite()) {
            return Err(ModelError::NonFiniteValue(name));
        }
    }

    let high_count = high_fidelity_paired.len();
    let low_count = low_fidelity_paired.len() + low_fidelity_extra.len();
    let high_mean = arithmetic_mean(high_fidelity_paired);
    let low_paired_mean = arithmetic_mean(low_fidelity_paired);
    let low_all_sum = low_fidelity_paired
        .iter()
        .chain(low_fidelity_extra.iter())
        .sum::<f64>();
    let low_all_mean = low_all_sum / low_count as f64;

    let high_variance = sample_variance(high_fidelity_paired, high_mean);
    let low_paired_variance = sample_variance(low_fidelity_paired, low_paired_mean);
    let paired_covariance = sample_covariance(
        high_fidelity_paired,
        low_fidelity_paired,
        high_mean,
        low_paired_mean,
    )?;

    let baseline_variance = high_variance / high_count as f64;
    let nested_factor = 1.0 / high_count as f64 - 1.0 / low_count as f64;
    let estimated_variance = baseline_variance
        + nested_factor * (beta * beta * low_paired_variance - 2.0 * beta * paired_covariance);

    if !estimated_variance.is_finite() || estimated_variance < -1.0e-10 * baseline_variance.max(1.0)
    {
        return Err(ModelError::InvalidParameter(
            "control_variate_variance_estimate_invalid",
        ));
    }
    let estimated_variance = estimated_variance.max(0.0);
    let estimate = high_mean + beta * (low_paired_mean - low_all_mean);
    if !estimate.is_finite() {
        return Err(ModelError::InvalidParameter(
            "control_variate_estimate_nonfinite",
        ));
    }
    let variance_reduction_factor = if estimated_variance > 0.0 {
        Some(baseline_variance / estimated_variance)
    } else {
        None
    };

    Ok(ControlVariateEstimate {
        estimate,
        beta,
        estimated_variance,
        baseline_high_fidelity_variance: baseline_variance,
        variance_reduction_factor,
        high_fidelity_samples: high_count,
        low_fidelity_samples: low_count,
    })
}

fn arithmetic_mean(values: &[f64]) -> f64 {
    values
        .iter()
        .map(|value| *value / values.len() as f64)
        .sum::<f64>()
}

fn sample_variance(values: &[f64], mean: f64) -> f64 {
    values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64
}

fn sample_covariance(
    left: &[f64],
    right: &[f64],
    left_mean: f64,
    right_mean: f64,
) -> Result<f64, ModelError> {
    if left.len() < 2 || left.len() != right.len() {
        return Err(ModelError::MismatchedDataLengths);
    }
    Ok(left
        .iter()
        .zip(right.iter())
        .map(|(x, y)| (x - left_mean) * (y - right_mean))
        .sum::<f64>()
        / (left.len() - 1) as f64)
}

#[derive(Debug, Clone, PartialEq)]
pub struct RobustDesignScore {
    pub candidate_index: usize,
    pub worst_case_utility: f64,
    pub mean_utility: f64,
    pub cost: f64,
    pub worst_case_utility_per_cost: f64,
}

/// Score candidate designs over a finite, declared set of nuisance scenarios.
///
/// Rows correspond to designs and columns to nuisance scenarios. The worst-case
/// utility is a finite-scenario maximin objective, not a continuous robust
/// guarantee. The caller must define a defensible scenario set.
pub fn robust_design_scores(
    utilities_by_design: &[Vec<f64>],
    costs: &[f64],
) -> Result<Vec<RobustDesignScore>, ModelError> {
    if utilities_by_design.is_empty() || costs.len() != utilities_by_design.len() {
        return Err(ModelError::MismatchedDataLengths);
    }
    let scenario_count = utilities_by_design[0].len();
    if scenario_count == 0
        || utilities_by_design
            .iter()
            .any(|row| row.len() != scenario_count)
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut scores = Vec::with_capacity(utilities_by_design.len());
    for (index, (utilities, cost)) in utilities_by_design.iter().zip(costs.iter()).enumerate() {
        if !cost.is_finite() {
            return Err(ModelError::NonFiniteValue("design_cost"));
        }
        if *cost <= 0.0 {
            return Err(ModelError::NonPositiveValue("design_cost"));
        }
        if utilities.iter().any(|value| !value.is_finite()) {
            return Err(ModelError::NonFiniteValue("design_scenario_utility"));
        }
        let worst_case_utility = utilities.iter().copied().fold(f64::INFINITY, f64::min);
        let mean_utility = arithmetic_mean(utilities);
        let worst_case_utility_per_cost = worst_case_utility / *cost;
        if !mean_utility.is_finite() || !worst_case_utility_per_cost.is_finite() {
            return Err(ModelError::InvalidParameter(
                "robust_design_score_overflowed",
            ));
        }
        scores.push(RobustDesignScore {
            candidate_index: index,
            worst_case_utility,
            mean_utility,
            cost: *cost,
            worst_case_utility_per_cost,
        });
    }
    Ok(scores)
}

/// Return the finite-scenario maximin design; ties break toward lower cost.
pub fn select_maximin_design(scores: &[RobustDesignScore]) -> Result<usize, ModelError> {
    if scores.is_empty() {
        return Err(ModelError::MismatchedDataLengths);
    }
    for score in scores {
        if !score.worst_case_utility.is_finite()
            || !score.mean_utility.is_finite()
            || !score.cost.is_finite()
            || !score.worst_case_utility_per_cost.is_finite()
        {
            return Err(ModelError::NonFiniteValue("robust_design_score"));
        }
        if score.cost <= 0.0 {
            return Err(ModelError::NonPositiveValue("design_cost"));
        }
    }
    let mut best = &scores[0];
    for candidate in scores.iter().skip(1) {
        if candidate.worst_case_utility > best.worst_case_utility
            || (candidate.worst_case_utility == best.worst_case_utility
                && candidate.cost < best.cost)
        {
            best = candidate;
        }
    }
    Ok(best.candidate_index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfectly_correlated_control_variate_halves_synthetic_variance() {
        let result = nested_control_variate_estimate(
            &[2.0, 4.0, 6.0],
            &[1.0, 2.0, 3.0],
            &[4.0, 5.0, 6.0],
            2.0,
        )
        .expect("ACV estimate");
        assert!((result.estimate - 1.0).abs() < 1.0e-12);
        assert!((result.baseline_high_fidelity_variance - 4.0 / 3.0).abs() < 1.0e-12);
        assert!((result.estimated_variance - 2.0 / 3.0).abs() < 1.0e-12);
        assert!((result.variance_reduction_factor.expect("factor") - 2.0).abs() < 1.0e-12);
        assert_eq!(result.high_fidelity_samples, 3);
        assert_eq!(result.low_fidelity_samples, 6);
    }

    #[test]
    fn zero_beta_reduces_to_high_fidelity_sample_mean() {
        let result = nested_control_variate_estimate(
            &[1.0, 3.0, 5.0],
            &[10.0, 20.0, 30.0],
            &[40.0, 50.0],
            0.0,
        )
        .expect("zero-beta estimate");
        assert_eq!(result.estimate, 3.0);
        assert!(
            (result.estimated_variance - result.baseline_high_fidelity_variance).abs() < 1.0e-12
        );
    }

    #[test]
    fn extra_low_fidelity_samples_do_not_change_target_estimand() {
        let result = nested_control_variate_estimate(
            &[2.0, 4.0, 6.0, 8.0],
            &[1.0, 2.0, 3.0, 4.0],
            &[5.0, 6.0, 7.0, 8.0],
            1.0,
        )
        .expect("ACV estimate");
        assert!(result.estimate.is_finite());
        assert_eq!(result.low_fidelity_samples, 8);
    }

    #[test]
    fn rejects_mismatched_or_insufficient_paired_samples() {
        assert_eq!(
            nested_control_variate_estimate(&[1.0], &[1.0], &[2.0], 1.0),
            Err(ModelError::MismatchedDataLengths)
        );
        assert_eq!(
            nested_control_variate_estimate(&[1.0, 2.0], &[1.0], &[2.0], 1.0),
            Err(ModelError::MismatchedDataLengths)
        );
        assert_eq!(
            nested_control_variate_estimate(&[1.0, 2.0], &[1.0, 2.0], &[], 1.0),
            Err(ModelError::MismatchedDataLengths)
        );
    }

    #[test]
    fn rejects_nonfinite_control_variate_inputs() {
        assert_eq!(
            nested_control_variate_estimate(&[1.0, 2.0], &[1.0, f64::NAN], &[3.0], 1.0),
            Err(ModelError::NonFiniteValue("low_fidelity_paired_samples"))
        );
        assert_eq!(
            nested_control_variate_estimate(&[1.0, 2.0], &[1.0, 2.0], &[3.0], f64::INFINITY),
            Err(ModelError::NonFiniteValue("control_variate_beta"))
        );
    }

    #[test]
    fn robust_design_selects_candidate_with_best_worst_case_utility() {
        let scores = robust_design_scores(
            &[
                vec![4.0, 8.0, 2.0],
                vec![3.0, 3.0, 3.0],
                vec![5.0, 7.0, 1.0],
            ],
            &[1.0, 1.0, 0.5],
        )
        .expect("robust scores");
        assert_eq!(select_maximin_design(&scores), Ok(1));
        assert_eq!(scores[0].worst_case_utility, 2.0);
        assert_eq!(scores[1].worst_case_utility, 3.0);
    }

    #[test]
    fn robust_design_breaks_worst_case_ties_by_cost() {
        let scores =
            robust_design_scores(&[vec![2.0, 4.0], vec![2.0, 2.0]], &[3.0, 1.0]).expect("scores");
        assert_eq!(select_maximin_design(&scores), Ok(1));
    }

    #[test]
    fn robust_design_rejects_invalid_shapes_and_costs() {
        assert_eq!(
            robust_design_scores(&[], &[]),
            Err(ModelError::MismatchedDataLengths)
        );
        assert_eq!(
            robust_design_scores(&[vec![1.0], vec![1.0, 2.0]], &[1.0, 1.0]),
            Err(ModelError::MismatchedDataLengths)
        );
        assert_eq!(
            robust_design_scores(&[vec![1.0]], &[0.0]),
            Err(ModelError::NonPositiveValue("design_cost"))
        );
    }

    #[test]
    fn robust_design_rejects_cost_normalization_overflow() {
        assert_eq!(
            robust_design_scores(&[vec![f64::MAX]], &[f64::MIN_POSITIVE]),
            Err(ModelError::InvalidParameter(
                "robust_design_score_overflowed"
            ))
        );
    }

    #[test]
    fn maximin_selector_rejects_nonfinite_or_invalid_external_scores() {
        let bad = [RobustDesignScore {
            candidate_index: 0,
            worst_case_utility: f64::NAN,
            mean_utility: 1.0,
            cost: 1.0,
            worst_case_utility_per_cost: 1.0,
        }];
        assert_eq!(
            select_maximin_design(&bad),
            Err(ModelError::NonFiniteValue("robust_design_score"))
        );
    }

    #[test]
    fn robust_design_rejects_nonfinite_scenario_values() {
        assert_eq!(
            robust_design_scores(&[vec![1.0, f64::NAN]], &[1.0]),
            Err(ModelError::NonFiniteValue("design_scenario_utility"))
        );
    }
}
