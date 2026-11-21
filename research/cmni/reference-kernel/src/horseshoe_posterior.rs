//! Raw-scale horseshoe posterior density and analytic gradients.
//!
//! Likelihood:
//!   y ~ Normal(mu .* exp(X beta), diag(mu) Sigma_relative diag(mu))
//!
//! beta_j | lambda_j, tau ~ Normal(0, tau^2 lambda_j^2)
//! lambda_j ~ HalfCauchy(0, 1)
//! tau ~ caller-selected HalfNormal(0, scale) or HalfCauchy(0, scale).
//!
//! Coordinates are beta, log(lambda), log(tau). Log-scale coordinates include
//! the required Jacobian terms. This module provides the posterior target and
//! analytic gradient only; it does not implement BMC/HMC/NUTS sampling,
//! adaptation, convergence diagnostics or the full 160-columns-per-feature fit.

use crate::ModelError;

#[cfg(test)]
type BaseArgs = (
    Vec<f64>,
    Vec<f64>,
    Vec<Vec<f64>>,
    Vec<f64>,
    Vec<f64>,
    Vec<Vec<f64>>,
);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GlobalScalePrior {
    HalfNormal { scale: f64 },
    HalfCauchy { scale: f64 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawScaleHorseshoePosterior {
    pub log_likelihood: f64,
    pub log_prior: f64,
    pub log_posterior: f64,
    pub coefficient_gradient: Vec<f64>,
    pub log_local_scale_gradient: Vec<f64>,
    pub log_global_scale_gradient: f64,
    pub biased_prediction: Vec<f64>,
}

/// Evaluate raw-scale Gaussian likelihood + regularized horseshoe prior.
///
/// The covariance is anchored to baseline_mean, not the biased prediction.
/// Global tau is sampled on log scale with either half-normal or half-Cauchy
/// prior. Local scales are half-Cauchy. All design columns must be frozen and
/// scaled using training data only.
pub fn raw_scale_horseshoe_log_posterior(
    observed: &[f64],
    baseline_mean: &[f64],
    design_matrix: &[Vec<f64>],
    coefficients: &[f64],
    log_local_scales: &[f64],
    log_global_scale: f64,
    relative_covariance: &[Vec<f64>],
    global_scale_prior: GlobalScalePrior,
) -> Result<RawScaleHorseshoePosterior, ModelError> {
    let observation_count = observed.len();
    let coefficient_count = coefficients.len();
    if observation_count == 0
        || coefficient_count == 0
        || baseline_mean.len() != observation_count
        || design_matrix.len() != observation_count
        || log_local_scales.len() != coefficient_count
        || relative_covariance.len() != observation_count
        || relative_covariance
            .iter()
            .any(|row| row.len() != observation_count)
        || design_matrix
            .iter()
            .any(|row| row.len() != coefficient_count)
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    if !log_global_scale.is_finite() || log_global_scale.abs() > 300.0 {
        return Err(ModelError::InvalidParameter(
            "log_global_scale_out_of_supported_range",
        ));
    }
    if log_local_scales
        .iter()
        .any(|value| !value.is_finite() || value.abs() > 300.0)
    {
        return Err(ModelError::InvalidParameter(
            "log_local_scale_out_of_supported_range",
        ));
    }
    let prior_scale = match global_scale_prior {
        GlobalScalePrior::HalfNormal { scale } | GlobalScalePrior::HalfCauchy { scale } => scale,
    };
    if !prior_scale.is_finite() {
        return Err(ModelError::NonFiniteValue("global_scale_prior_scale"));
    }
    if prior_scale <= 0.0 {
        return Err(ModelError::NonPositiveValue("global_scale_prior_scale"));
    }

    for value in observed {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue("horseshoe_observations"));
        }
        if *value < 0.0 {
            return Err(ModelError::InvalidParameter(
                "horseshoe_observations_must_be_nonnegative",
            ));
        }
    }
    for value in baseline_mean {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue("horseshoe_baseline_mean"));
        }
        if *value <= 0.0 {
            return Err(ModelError::NonPositiveValue("horseshoe_baseline_mean"));
        }
    }
    if design_matrix
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return Err(ModelError::NonFiniteValue("horseshoe_design_matrix"));
    }
    if coefficients.iter().any(|value| !value.is_finite()) {
        return Err(ModelError::NonFiniteValue("horseshoe_coefficients"));
    }
    if relative_covariance
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return Err(ModelError::NonFiniteValue("horseshoe_relative_covariance"));
    }

    let covariance_scale = relative_covariance
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    let symmetry_tolerance = 1.0e-10 * covariance_scale.max(f64::MIN_POSITIVE);
    for row in 0..observation_count {
        for column in 0..row {
            if (relative_covariance[row][column] - relative_covariance[column][row]).abs()
                > symmetry_tolerance
            {
                return Err(ModelError::InvalidCovariance(
                    "horseshoe_relative_covariance_must_be_symmetric",
                ));
            }
        }
    }

    let global_scale = log_global_scale.exp();
    if !global_scale.is_finite() || global_scale <= 0.0 {
        return Err(ModelError::InvalidParameter("global_scale_invalid"));
    }
    let local_scales: Vec<f64> = log_local_scales.iter().map(|value| value.exp()).collect();
    if local_scales
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(ModelError::InvalidParameter("local_scale_invalid"));
    }

    let mut covariance = vec![vec![0.0; observation_count]; observation_count];
    for row in 0..observation_count {
        for column in 0..observation_count {
            covariance[row][column] =
                baseline_mean[row] * relative_covariance[row][column] * baseline_mean[column];
        }
    }
    let lower = cholesky(&covariance)?;
    let logdet = log_determinant_from_cholesky(&lower)?;

    let mut biased_prediction = vec![0.0; observation_count];
    for row in 0..observation_count {
        let delta = dot(&design_matrix[row], coefficients);
        if !delta.is_finite() || delta.abs() > 700.0 {
            return Err(ModelError::InvalidParameter(
                "horseshoe_bias_predictor_out_of_range",
            ));
        }
        biased_prediction[row] = baseline_mean[row] * delta.exp();
        if !biased_prediction[row].is_finite() || biased_prediction[row] <= 0.0 {
            return Err(ModelError::InvalidParameter(
                "horseshoe_biased_prediction_invalid",
            ));
        }
    }
    let residual: Vec<f64> = observed
        .iter()
        .zip(biased_prediction.iter())
        .map(|(y, mean)| y - mean)
        .collect();
    let weighted_residual = solve_cholesky(&lower, &residual)?;
    let quadratic = dot(&residual, &weighted_residual);
    if !quadratic.is_finite() || quadratic < -1.0e-9 {
        return Err(ModelError::InvalidCovariance(
            "horseshoe_likelihood_quadratic_invalid",
        ));
    }
    let log_likelihood = -0.5
        * (observation_count as f64 * (2.0 * std::f64::consts::PI).ln()
            + logdet
            + quadratic.max(0.0));

    let mut coefficient_gradient = vec![0.0; coefficient_count];
    for feature in 0..coefficient_count {
        for row in 0..observation_count {
            coefficient_gradient[feature] +=
                design_matrix[row][feature] * biased_prediction[row] * weighted_residual[row];
        }
    }

    let mut log_prior = 0.0;
    let mut log_local_scale_gradient = vec![0.0; coefficient_count];
    let mut sum_scaled_square = 0.0;
    for feature in 0..coefficient_count {
        let scale = global_scale * local_scales[feature];
        if !scale.is_finite() || scale <= 0.0 {
            return Err(ModelError::InvalidParameter(
                "horseshoe_coefficient_scale_invalid",
            ));
        }
        let standardized = coefficients[feature] / scale;
        let q = standardized * standardized;
        let local_sq = local_scales[feature] * local_scales[feature];
        if !q.is_finite() || !local_sq.is_finite() {
            return Err(ModelError::InvalidParameter("horseshoe_prior_overflow"));
        }
        sum_scaled_square += q;
        log_prior += -0.5 * q - scale.ln() - 0.5 * (2.0 * std::f64::consts::PI).ln();
        // Half-Cauchy local scale plus log-coordinate Jacobian.
        log_prior +=
            (2.0 / std::f64::consts::PI).ln() - (1.0 + local_sq).ln() + log_local_scales[feature];
        coefficient_gradient[feature] -= coefficients[feature] / (scale * scale);
        log_local_scale_gradient[feature] = q - 2.0 * local_sq / (1.0 + local_sq);
    }

    let global_ratio = global_scale / prior_scale;
    let global_ratio_sq = global_ratio * global_ratio;
    if !global_ratio_sq.is_finite() || !sum_scaled_square.is_finite() {
        return Err(ModelError::InvalidParameter(
            "horseshoe_global_prior_overflow",
        ));
    }
    match global_scale_prior {
        GlobalScalePrior::HalfNormal { scale } => {
            log_prior +=
                0.5 * (2.0 / std::f64::consts::PI).ln() - scale.ln() - 0.5 * global_ratio_sq
                    + log_global_scale;
            let coefficient_count_f = coefficient_count as f64;
            let global_gradient = sum_scaled_square - coefficient_count_f + 1.0 - global_ratio_sq;
            if !global_gradient.is_finite() {
                return Err(ModelError::InvalidParameter(
                    "horseshoe_global_gradient_nonfinite",
                ));
            }
            Ok(RawScaleHorseshoePosterior {
                log_likelihood,
                log_prior,
                log_posterior: log_likelihood + log_prior,
                coefficient_gradient,
                log_local_scale_gradient,
                log_global_scale_gradient: global_gradient,
                biased_prediction,
            })
        }
        GlobalScalePrior::HalfCauchy { scale } => {
            log_prior +=
                (2.0 / std::f64::consts::PI).ln() - scale.ln() - (1.0 + global_ratio_sq).ln()
                    + log_global_scale;
            let global_gradient = sum_scaled_square - coefficient_count as f64 + 1.0
                - 2.0 * global_ratio_sq / (1.0 + global_ratio_sq);
            if !global_gradient.is_finite() {
                return Err(ModelError::InvalidParameter(
                    "horseshoe_global_gradient_nonfinite",
                ));
            }
            Ok(RawScaleHorseshoePosterior {
                log_likelihood,
                log_prior,
                log_posterior: log_likelihood + log_prior,
                coefficient_gradient,
                log_local_scale_gradient,
                log_global_scale_gradient: global_gradient,
                biased_prediction,
            })
        }
    }
}

fn cholesky(matrix: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, ModelError> {
    let dimension = matrix.len();
    let mut lower = vec![vec![0.0; dimension]; dimension];
    for row in 0..dimension {
        for column in 0..=row {
            let mut value = matrix[row][column];
            for inner in 0..column {
                value -= lower[row][inner] * lower[column][inner];
            }
            if row == column {
                if !value.is_finite() || value <= 0.0 {
                    return Err(ModelError::InvalidCovariance(
                        "horseshoe_covariance_must_be_positive_definite",
                    ));
                }
                lower[row][column] = value.sqrt();
            } else {
                lower[row][column] = value / lower[column][column];
            }
        }
    }
    Ok(lower)
}

fn solve_cholesky(lower: &[Vec<f64>], rhs: &[f64]) -> Result<Vec<f64>, ModelError> {
    let dimension = lower.len();
    if rhs.len() != dimension {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut forward = vec![0.0; dimension];
    for row in 0..dimension {
        let mut value = rhs[row];
        for column in 0..row {
            value -= lower[row][column] * forward[column];
        }
        forward[row] = value / lower[row][row];
    }
    let mut solution = vec![0.0; dimension];
    for row in (0..dimension).rev() {
        let mut value = forward[row];
        for column in (row + 1)..dimension {
            value -= lower[column][row] * solution[column];
        }
        solution[row] = value / lower[row][row];
    }
    if solution.iter().any(|value| !value.is_finite()) {
        return Err(ModelError::InvalidCovariance(
            "horseshoe_covariance_solve_nonfinite",
        ));
    }
    Ok(solution)
}

fn log_determinant_from_cholesky(lower: &[Vec<f64>]) -> Result<f64, ModelError> {
    let value = 2.0
        * lower
            .iter()
            .enumerate()
            .map(|(index, row)| row[index].ln())
            .sum::<f64>();
    if !value.is_finite() {
        return Err(ModelError::InvalidCovariance(
            "horseshoe_covariance_logdet_nonfinite",
        ));
    }
    Ok(value)
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right.iter()).map(|(a, b)| a * b).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grouped_target_matches_single_global_scale_target_for_one_group() {
        let (observed, baseline, design, coefficients, log_local, covariance) = base_args();
        let log_global = 0.37_f64.ln();
        for prior in [
            GlobalScalePrior::HalfNormal { scale: 1.0 },
            GlobalScalePrior::HalfCauchy { scale: 1.0 },
        ] {
            let single = raw_scale_horseshoe_log_posterior(
                &observed,
                &baseline,
                &design,
                &coefficients,
                &log_local,
                log_global,
                &covariance,
                prior,
            )
            .expect("single-scale target");
            let grouped = grouped_raw_scale_horseshoe_log_posterior(
                GroupedHorseshoePosteriorInput {
                    observed: &observed,
                    baseline_mean: &baseline,
                    design_matrix: &design,
                    coefficients: &coefficients,
                    log_local_scales: &log_local,
                    log_global_scales: &[log_global],
                    global_scale_index_by_coefficient: &vec![0; coefficients.len()],
                    relative_covariance: &covariance,
                    fixed_coefficient_scale: 1.0,
                    global_scale_prior: prior,
                },
            )
            .expect("grouped single-group target");
            assert!((single.log_likelihood - grouped.log_likelihood).abs() < 1.0e-12);
            assert!((single.log_prior - grouped.log_prior).abs() < 1.0e-10);
            assert_eq!(single.biased_prediction, grouped.biased_prediction);
            for (left, right) in single
                .coefficient_gradient
                .iter()
                .zip(grouped.coefficient_gradient.iter())
            {
                assert!((left - right).abs() < 1.0e-10);
            }
            for (left, right) in single
                .log_local_scale_gradient
                .iter()
                .zip(grouped.log_local_scale_gradient.iter())
            {
                assert!((left - right).abs() < 1.0e-10);
            }
            assert!((single.log_global_scale_gradient - grouped.log_global_scale_gradients[0]).abs() < 1.0e-10);
        }
    }

    #[test]
    fn grouped_three_scale_gradients_match_finite_differences() {
        let observed = [12.0, 18.0];
        let baseline = [10.0, 20.0];
        let design = vec![vec![1.0, 0.3, -0.2], vec![0.2, -0.5, 0.4]];
        let coefficients = [0.001, -0.002, 0.003];
        let log_local = [0.8_f64.ln(), 1.1_f64.ln(), 0.9_f64.ln()];
        let log_global = [0.6_f64.ln(), 1.0_f64.ln(), 0.8_f64.ln()];
        let groups = [0, 1, 2];
        let covariance = vec![vec![0.04, 0.005], vec![0.005, 0.09]];
        let prior = GlobalScalePrior::HalfNormal { scale: 1.0 };
        let evaluate = |beta: &[f64], local: &[f64], global: &[f64]| {
            grouped_raw_scale_horseshoe_log_posterior(GroupedHorseshoePosteriorInput {
                observed: &observed,
                baseline_mean: &baseline,
                design_matrix: &design,
                coefficients: beta,
                log_local_scales: local,
                log_global_scales: global,
                global_scale_index_by_coefficient: &groups,
                relative_covariance: &covariance,
                fixed_coefficient_scale: 1.0e-2,
                global_scale_prior: prior,
            })
            .expect("grouped target")
        };
        let base = evaluate(&coefficients, &log_local, &log_global);
        let epsilon = 1.0e-6;

        let mut beta_plus = coefficients;
        beta_plus[1] += epsilon;
        let mut beta_minus = coefficients;
        beta_minus[1] -= epsilon;
        let beta_fd = (evaluate(&beta_plus, &log_local, &log_global).log_posterior
            - evaluate(&beta_minus, &log_local, &log_global).log_posterior)
            / (2.0 * epsilon);
        assert!((beta_fd - base.coefficient_gradient[1]).abs() < 1.0e-4);

        let mut local_plus = log_local;
        local_plus[0] += epsilon;
        let mut local_minus = log_local;
        local_minus[0] -= epsilon;
        let local_fd = (evaluate(&coefficients, &local_plus, &log_global).log_posterior
            - evaluate(&coefficients, &local_minus, &log_global).log_posterior)
            / (2.0 * epsilon);
        assert!((local_fd - base.log_local_scale_gradient[0]).abs() < 1.0e-4);

        let mut global_plus = log_global;
        global_plus[2] += epsilon;
        let mut global_minus = log_global;
        global_minus[2] -= epsilon;
        let global_fd = (evaluate(&coefficients, &log_local, &global_plus).log_posterior
            - evaluate(&coefficients, &log_local, &global_minus).log_posterior)
            / (2.0 * epsilon);
        assert!((global_fd - base.log_global_scale_gradients[2]).abs() < 1.0e-4);
    }

    fn base_args() -> BaseArgs {
        (
            vec![11.0, 19.0],
            vec![10.0, 20.0],
            vec![vec![0.0], vec![1.0]],
            vec![0.1],
            vec![0.0],
            vec![vec![0.04, 0.01], vec![0.01, 0.09]],
        )
    }

    #[test]
    fn posterior_is_finite_for_halfnormal_and_halfcauchy_global_scales() {
        let (y, mu, x, beta, log_lambda, covariance) = base_args();
        for prior in [
            GlobalScalePrior::HalfNormal { scale: 0.5 },
            GlobalScalePrior::HalfCauchy { scale: 0.5 },
        ] {
            let result = raw_scale_horseshoe_log_posterior(
                &y,
                &mu,
                &x,
                &beta,
                &log_lambda,
                -1.0,
                &covariance,
                prior,
            )
            .expect("horseshoe target");
            assert!(result.log_posterior.is_finite());
            assert_eq!(result.coefficient_gradient.len(), 1);
            assert_eq!(result.log_local_scale_gradient.len(), 1);
        }
    }

    #[test]
    fn coefficient_gradient_matches_finite_difference() {
        let (y, mu, x, beta, log_lambda, covariance) = base_args();
        let prior = GlobalScalePrior::HalfNormal { scale: 0.5 };
        let result = raw_scale_horseshoe_log_posterior(
            &y,
            &mu,
            &x,
            &beta,
            &log_lambda,
            -1.0,
            &covariance,
            prior,
        )
        .expect("target");
        let epsilon = 1.0e-6;
        let mut plus = beta.clone();
        let mut minus = beta.clone();
        plus[0] += epsilon;
        minus[0] -= epsilon;
        let lp = raw_scale_horseshoe_log_posterior(
            &y,
            &mu,
            &x,
            &plus,
            &log_lambda,
            -1.0,
            &covariance,
            prior,
        )
        .expect("plus")
        .log_posterior;
        let lm = raw_scale_horseshoe_log_posterior(
            &y,
            &mu,
            &x,
            &minus,
            &log_lambda,
            -1.0,
            &covariance,
            prior,
        )
        .expect("minus")
        .log_posterior;
        let finite_difference = (lp - lm) / (2.0 * epsilon);
        assert!((finite_difference - result.coefficient_gradient[0]).abs() < 1.0e-5);
    }

    #[test]
    fn local_log_scale_gradient_matches_finite_difference() {
        let (y, mu, x, beta, log_lambda, covariance) = base_args();
        let prior = GlobalScalePrior::HalfNormal { scale: 0.5 };
        let result = raw_scale_horseshoe_log_posterior(
            &y,
            &mu,
            &x,
            &beta,
            &log_lambda,
            -1.0,
            &covariance,
            prior,
        )
        .expect("target");
        let epsilon = 1.0e-6;
        let lp = raw_scale_horseshoe_log_posterior(
            &y,
            &mu,
            &x,
            &beta,
            &[log_lambda[0] + epsilon],
            -1.0,
            &covariance,
            prior,
        )
        .expect("plus")
        .log_posterior;
        let lm = raw_scale_horseshoe_log_posterior(
            &y,
            &mu,
            &x,
            &beta,
            &[log_lambda[0] - epsilon],
            -1.0,
            &covariance,
            prior,
        )
        .expect("minus")
        .log_posterior;
        assert!(((lp - lm) / (2.0 * epsilon) - result.log_local_scale_gradient[0]).abs() < 1.0e-5);
    }

    #[test]
    fn global_log_scale_gradient_matches_finite_difference_for_both_priors() {
        let (y, mu, x, beta, log_lambda, covariance) = base_args();
        let epsilon = 1.0e-6;
        for prior in [
            GlobalScalePrior::HalfNormal { scale: 0.5 },
            GlobalScalePrior::HalfCauchy { scale: 0.5 },
        ] {
            let result = raw_scale_horseshoe_log_posterior(
                &y,
                &mu,
                &x,
                &beta,
                &log_lambda,
                -1.0,
                &covariance,
                prior,
            )
            .expect("target");
            let lp = raw_scale_horseshoe_log_posterior(
                &y,
                &mu,
                &x,
                &beta,
                &log_lambda,
                -1.0 + epsilon,
                &covariance,
                prior,
            )
            .expect("plus")
            .log_posterior;
            let lm = raw_scale_horseshoe_log_posterior(
                &y,
                &mu,
                &x,
                &beta,
                &log_lambda,
                -1.0 - epsilon,
                &covariance,
                prior,
            )
            .expect("minus")
            .log_posterior;
            assert!(
                ((lp - lm) / (2.0 * epsilon) - result.log_global_scale_gradient).abs() < 1.0e-5
            );
        }
    }

    #[test]
    fn rejects_invalid_hyperparameters_and_dimensions() {
        let (y, mu, x, beta, log_lambda, covariance) = base_args();
        assert_eq!(
            raw_scale_horseshoe_log_posterior(
                &y,
                &mu,
                &x,
                &beta,
                &log_lambda,
                -1.0,
                &covariance,
                GlobalScalePrior::HalfNormal { scale: 0.0 },
            ),
            Err(ModelError::NonPositiveValue("global_scale_prior_scale"))
        );
        assert_eq!(
            raw_scale_horseshoe_log_posterior(
                &y,
                &mu,
                &x,
                &beta,
                &[],
                -1.0,
                &covariance,
                GlobalScalePrior::HalfNormal { scale: 1.0 },
            ),
            Err(ModelError::MismatchedDataLengths)
        );
    }
}
