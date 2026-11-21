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
    /// Derivative of the log likelihood with respect to each observation's log mean.
    pub log_mean_gradient: Vec<f64>,
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

    let log_mean_gradient: Vec<f64> = biased_prediction
        .iter()
        .zip(weighted_residual.iter())
        .map(|(prediction, residual)| prediction * residual)
        .collect();
    let mut coefficient_gradient = vec![0.0; coefficient_count];
    for feature in 0..coefficient_count {
        for row in 0..observation_count {
            coefficient_gradient[feature] += design_matrix[row][feature] * log_mean_gradient[row];
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
                log_mean_gradient,
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
                log_mean_gradient,
                log_local_scale_gradient,
                log_global_scale_gradient: global_gradient,
                biased_prediction,
            })
        }
    }
}

/// Inputs for a raw-scale horseshoe target with separate global scales.
/// Each coefficient is assigned to exactly one global shrinkage group.
pub struct GroupedHorseshoePosteriorInput<'a> {
    pub observed: &'a [f64],
    pub baseline_mean: &'a [f64],
    pub design_matrix: &'a [Vec<f64>],
    pub coefficients: &'a [f64],
    pub log_local_scales: &'a [f64],
    pub log_global_scales: &'a [f64],
    pub global_scale_index_by_coefficient: &'a [usize],
    /// Log experiment scales. Empty means the caller supplies an already-scaled baseline.
    /// Otherwise each observation is assigned through experiment_scale_index_by_observation.
    pub log_experiment_scales: &'a [f64],
    /// One scale-group index per observation when log_experiment_scales is non-empty.
    pub experiment_scale_index_by_observation: &'a [usize],
    pub relative_covariance: &'a [Vec<f64>],
    /// Fixed multiplicative factor on coefficient scales (AIACHNE tau_scale).
    pub fixed_coefficient_scale: f64,
    pub global_scale_prior: GlobalScalePrior,
}

/// Result of a grouped raw-scale horseshoe posterior evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupedRawScaleHorseshoePosterior {
    pub log_likelihood: f64,
    pub log_prior: f64,
    pub log_posterior: f64,
    pub coefficient_gradient: Vec<f64>,
    pub log_local_scale_gradient: Vec<f64>,
    pub log_global_scale_gradients: Vec<f64>,
    pub log_experiment_scale_gradients: Vec<f64>,
    pub biased_prediction: Vec<f64>,
}

/// Evaluate a horseshoe target with one global scale per coefficient group.
///
/// This supports the AIACHNE short/medium/long structure when the coefficient
/// mapping is generated by `aiachne_global_scale_index_by_coefficient`.
/// It evaluates the same raw-scale likelihood and applies a separate
/// half-normal or half-Cauchy global prior to each group.
pub fn grouped_raw_scale_horseshoe_log_posterior(
    input: GroupedHorseshoePosteriorInput<'_>,
) -> Result<GroupedRawScaleHorseshoePosterior, ModelError> {
    let coefficient_count = input.coefficients.len();
    let observation_count = input.observed.len();
    let group_count = input.log_global_scales.len();
    let experiment_scale_count = input.log_experiment_scales.len();
    if coefficient_count == 0
        || observation_count == 0
        || group_count == 0
        || input.log_local_scales.len() != coefficient_count
        || input.global_scale_index_by_coefficient.len() != coefficient_count
        || (experiment_scale_count == 0 && !input.experiment_scale_index_by_observation.is_empty())
        || (experiment_scale_count > 0
            && input.experiment_scale_index_by_observation.len() != observation_count)
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    if !input.fixed_coefficient_scale.is_finite() {
        return Err(ModelError::NonFiniteValue("fixed_coefficient_scale"));
    }
    if input.fixed_coefficient_scale <= 0.0 {
        return Err(ModelError::NonPositiveValue("fixed_coefficient_scale"));
    }
    let prior_scale = match input.global_scale_prior {
        GlobalScalePrior::HalfNormal { scale } | GlobalScalePrior::HalfCauchy { scale } => scale,
    };
    if !prior_scale.is_finite() {
        return Err(ModelError::NonFiniteValue("global_scale_prior_scale"));
    }
    if prior_scale <= 0.0 {
        return Err(ModelError::NonPositiveValue("global_scale_prior_scale"));
    }
    if input
        .log_global_scales
        .iter()
        .any(|value| !value.is_finite() || value.abs() > 300.0)
    {
        return Err(ModelError::InvalidParameter(
            "grouped_log_global_scale_out_of_supported_range",
        ));
    }

    let mut group_counts = vec![0usize; group_count];
    for group_index in input.global_scale_index_by_coefficient {
        if *group_index >= group_count {
            return Err(ModelError::InvalidParameter(
                "coefficient_global_scale_index_out_of_range",
            ));
        }
        group_counts[*group_index] += 1;
    }
    if group_counts.contains(&0) {
        return Err(ModelError::InvalidParameter(
            "every_global_scale_group_must_own_coefficients",
        ));
    }

    // AIACHNE's original interpolation_horseshoe model has one multiplicative
    // data_scale per experiment with Normal(1, 0.01), bounded to [0.5, 1.5].
    // Parameterize it in log coordinates and include the log-Jacobian.
    let mut experiment_scale_by_observation = vec![1.0; observation_count];
    let mut experiment_scale_counts = vec![0usize; experiment_scale_count];
    let mut experiment_scale_values = Vec::with_capacity(experiment_scale_count);
    for log_scale in input.log_experiment_scales {
        if !log_scale.is_finite() || log_scale.abs() > 300.0 {
            return Err(ModelError::InvalidParameter(
                "log_experiment_scale_out_of_supported_range",
            ));
        }
        let scale = log_scale.exp();
        if !scale.is_finite() || !(0.5..=1.5).contains(&scale) {
            return Err(ModelError::InvalidParameter(
                "experiment_scale_outside_aiache_bounds",
            ));
        }
        experiment_scale_values.push(scale);
    }
    if experiment_scale_count > 0 {
        for (observation_index, scale_index) in input
            .experiment_scale_index_by_observation
            .iter()
            .enumerate()
        {
            if *scale_index >= experiment_scale_count {
                return Err(ModelError::InvalidParameter(
                    "experiment_scale_index_out_of_range",
                ));
            }
            experiment_scale_by_observation[observation_index] =
                experiment_scale_values[*scale_index];
            experiment_scale_counts[*scale_index] += 1;
        }
        if experiment_scale_counts.contains(&0) {
            return Err(ModelError::InvalidParameter(
                "every_experiment_scale_must_own_observations",
            ));
        }
    }

    // The likelihood covariance is anchored to the unscaled physical mean.
    // Divide the relative covariance by the observation scale pairs so that
    // the base target reconstructs that unchanged covariance after scaling mu.
    let scaled_baseline: Vec<f64> = input
        .baseline_mean
        .iter()
        .zip(experiment_scale_by_observation.iter())
        .map(|(mean, scale)| mean * scale)
        .collect();
    if scaled_baseline.len() != observation_count {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut scaled_relative_covariance = input.relative_covariance.to_vec();
    if scaled_relative_covariance.len() != observation_count
        || scaled_relative_covariance
            .iter()
            .any(|row| row.len() != observation_count)
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    for (row_index, row) in scaled_relative_covariance.iter_mut().enumerate() {
        for (column_index, value) in row.iter_mut().enumerate() {
            *value /= experiment_scale_by_observation[row_index]
                * experiment_scale_by_observation[column_index];
        }
    }

    // Reuse the independently tested likelihood and log-coordinate local-prior
    // path with unit global scales. Replace its coefficient/global-prior terms
    // with the corresponding grouped terms below.
    let base = raw_scale_horseshoe_log_posterior(
        input.observed,
        &scaled_baseline,
        input.design_matrix,
        input.coefficients,
        input.log_local_scales,
        0.0,
        &scaled_relative_covariance,
        GlobalScalePrior::HalfNormal { scale: 1.0 },
    )?;

    let local_scales: Vec<f64> = input
        .log_local_scales
        .iter()
        .map(|value| value.exp())
        .collect();
    let global_scales: Vec<f64> = input
        .log_global_scales
        .iter()
        .map(|value| value.exp())
        .collect();
    if local_scales
        .iter()
        .chain(global_scales.iter())
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(ModelError::InvalidParameter(
            "grouped_horseshoe_scale_invalid",
        ));
    }

    let mut coefficient_gradient = base.coefficient_gradient.clone();
    let mut log_local_scale_gradient = vec![0.0; coefficient_count];
    let mut log_global_scale_gradients = vec![0.0; group_count];
    let mut log_experiment_scale_gradients = vec![0.0; experiment_scale_count];
    let mut sum_scaled_squares = vec![0.0; group_count];
    let mut log_prior = 0.0;

    // Stan's data_scale ~ normal(1, 0.01), constrained to [0.5, 1.5].
    // Since this target uses log(scale) coordinates, include the log-Jacobian.
    const EXPERIMENT_SCALE_PRIOR_STD: f64 = 0.01;
    for (scale_index, scale) in experiment_scale_values.iter().enumerate() {
        let standardized = (scale - 1.0) / EXPERIMENT_SCALE_PRIOR_STD;
        log_prior += -0.5 * standardized * standardized
            - EXPERIMENT_SCALE_PRIOR_STD.ln()
            - 0.5 * (2.0 * std::f64::consts::PI).ln()
            + input.log_experiment_scales[scale_index];
        log_experiment_scale_gradients[scale_index] =
            -(scale - 1.0) * scale / EXPERIMENT_SCALE_PRIOR_STD.powi(2) + 1.0;
    }
    for (observation_index, scale_index) in input
        .experiment_scale_index_by_observation
        .iter()
        .enumerate()
    {
        if experiment_scale_count > 0 {
            log_experiment_scale_gradients[*scale_index] +=
                base.log_mean_gradient[observation_index];
        }
    }

    for feature in 0..coefficient_count {
        let group_index = input.global_scale_index_by_coefficient[feature];
        let local = local_scales[feature];
        let global = global_scales[group_index];
        let scale = input.fixed_coefficient_scale * global * local;
        if !scale.is_finite() || scale <= 0.0 {
            return Err(ModelError::InvalidParameter(
                "grouped_horseshoe_coefficient_scale_invalid",
            ));
        }
        let coefficient = input.coefficients[feature];
        let standardized = coefficient / scale;
        let q = standardized * standardized;
        let local_sq = local * local;
        if !q.is_finite() || !local_sq.is_finite() {
            return Err(ModelError::InvalidParameter(
                "grouped_horseshoe_prior_overflow",
            ));
        }

        sum_scaled_squares[group_index] += q;
        log_prior += -0.5 * q - scale.ln() - 0.5 * (2.0 * std::f64::consts::PI).ln();
        // Half-Cauchy local scale with the log-coordinate Jacobian.
        log_prior += (2.0 / std::f64::consts::PI).ln() - (1.0 + local_sq).ln()
            + input.log_local_scales[feature];

        // Remove the base unit-global coefficient prior, then apply the grouped
        // coefficient prior with fixed_coefficient_scale * tau[group] * lambda.
        coefficient_gradient[feature] += coefficient / (local * local);
        coefficient_gradient[feature] -= coefficient / (scale * scale);
        log_local_scale_gradient[feature] = q - 2.0 * local_sq / (1.0 + local_sq);
    }

    for group_index in 0..group_count {
        let global = global_scales[group_index];
        let ratio = global / prior_scale;
        let ratio_sq = ratio * ratio;
        if !ratio_sq.is_finite() || !sum_scaled_squares[group_index].is_finite() {
            return Err(ModelError::InvalidParameter(
                "grouped_horseshoe_global_prior_overflow",
            ));
        }
        match input.global_scale_prior {
            GlobalScalePrior::HalfNormal { scale } => {
                log_prior += 0.5 * (2.0 / std::f64::consts::PI).ln() - scale.ln() - 0.5 * ratio_sq
                    + input.log_global_scales[group_index];
                log_global_scale_gradients[group_index] =
                    sum_scaled_squares[group_index] - group_counts[group_index] as f64 + 1.0
                        - ratio_sq;
            }
            GlobalScalePrior::HalfCauchy { scale } => {
                log_prior += (2.0 / std::f64::consts::PI).ln() - scale.ln() - (1.0 + ratio_sq).ln()
                    + input.log_global_scales[group_index];
                log_global_scale_gradients[group_index] =
                    sum_scaled_squares[group_index] - group_counts[group_index] as f64 + 1.0
                        - 2.0 * ratio_sq / (1.0 + ratio_sq);
            }
        }
    }

    if !log_prior.is_finite()
        || coefficient_gradient.iter().any(|value| !value.is_finite())
        || log_local_scale_gradient
            .iter()
            .any(|value| !value.is_finite())
        || log_global_scale_gradients
            .iter()
            .any(|value| !value.is_finite())
    {
        return Err(ModelError::InvalidParameter(
            "grouped_horseshoe_posterior_nonfinite",
        ));
    }

    Ok(GroupedRawScaleHorseshoePosterior {
        log_likelihood: base.log_likelihood,
        log_prior,
        log_posterior: base.log_likelihood + log_prior,
        coefficient_gradient,
        log_local_scale_gradient,
        log_global_scale_gradients,
        log_experiment_scale_gradients,
        biased_prediction: base.biased_prediction,
    })
}

/// Prepared grouped horseshoe model with a covariance Cholesky factor cached once.
///
/// Experiment scales alter only the predicted mean; the observation covariance
/// remains anchored to the unscaled physical baseline. The factorization can
/// therefore be reused across every posterior and leapfrog evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedGroupedRawScaleHorseshoeModel {
    observed: Vec<f64>,
    baseline_mean: Vec<f64>,
    design_matrix: Vec<Vec<f64>>,
    covariance_lower: Vec<Vec<f64>>,
    covariance_log_determinant: f64,
}

impl PreparedGroupedRawScaleHorseshoeModel {
    pub fn new(
        observed: &[f64],
        baseline_mean: &[f64],
        design_matrix: &[Vec<f64>],
        relative_covariance: &[Vec<f64>],
    ) -> Result<Self, ModelError> {
        let n = observed.len();
        if n == 0
            || baseline_mean.len() != n
            || design_matrix.len() != n
            || relative_covariance.len() != n
            || relative_covariance.iter().any(|row| row.len() != n)
        {
            return Err(ModelError::MismatchedDataLengths);
        }
        let p = design_matrix.first().map_or(0, Vec::len);
        if p == 0 || design_matrix.iter().any(|row| row.len() != p) {
            return Err(ModelError::MismatchedDataLengths);
        }
        if observed.iter().any(|v| !v.is_finite() || *v < 0.0) {
            return Err(ModelError::InvalidParameter(
                "prepared_grouped_observations_must_be_finite_nonnegative",
            ));
        }
        if baseline_mean.iter().any(|v| !v.is_finite() || *v <= 0.0) {
            return Err(ModelError::InvalidParameter(
                "prepared_grouped_baseline_mean_must_be_finite_positive",
            ));
        }
        if design_matrix.iter().flatten().any(|v| !v.is_finite()) {
            return Err(ModelError::NonFiniteValue("prepared_grouped_design_matrix"));
        }
        if relative_covariance.iter().flatten().any(|v| !v.is_finite()) {
            return Err(ModelError::NonFiniteValue(
                "prepared_grouped_relative_covariance",
            ));
        }
        let scale = relative_covariance
            .iter()
            .flatten()
            .map(|v| v.abs())
            .fold(0.0_f64, f64::max);
        let tolerance = 1.0e-10 * scale.max(f64::MIN_POSITIVE);
        for (row_index, row) in relative_covariance.iter().enumerate() {
            for (column_index, value) in row.iter().enumerate().take(row_index) {
                if (*value - relative_covariance[column_index][row_index]).abs() > tolerance {
                    return Err(ModelError::InvalidCovariance(
                        "prepared_grouped_relative_covariance_must_be_symmetric",
                    ));
                }
            }
        }

        let mut covariance = vec![vec![0.0; n]; n];
        for (row_index, row) in covariance.iter_mut().enumerate() {
            for (column_index, value) in row.iter_mut().enumerate() {
                *value = baseline_mean[row_index]
                    * relative_covariance[row_index][column_index]
                    * baseline_mean[column_index];
            }
        }
        let covariance_lower = cholesky(&covariance)?;
        let covariance_log_determinant = log_determinant_from_cholesky(&covariance_lower)?;
        Ok(Self {
            observed: observed.to_vec(),
            baseline_mean: baseline_mean.to_vec(),
            design_matrix: design_matrix.to_vec(),
            covariance_lower,
            covariance_log_determinant,
        })
    }

    pub fn evaluate(
        &self,
        coefficients: &[f64],
        log_local_scales: &[f64],
        log_global_scales: &[f64],
        global_scale_index_by_coefficient: &[usize],
        log_experiment_scales: &[f64],
        experiment_scale_index_by_observation: &[usize],
        fixed_coefficient_scale: f64,
        global_scale_prior: GlobalScalePrior,
    ) -> Result<GroupedRawScaleHorseshoePosterior, ModelError> {
        let n = self.observed.len();
        let p = self.design_matrix[0].len();
        let groups = log_global_scales.len();
        let experiment_groups = log_experiment_scales.len();
        if coefficients.len() != p
            || log_local_scales.len() != p
            || global_scale_index_by_coefficient.len() != p
            || groups == 0
            || (experiment_groups == 0 && !experiment_scale_index_by_observation.is_empty())
            || (experiment_groups > 0 && experiment_scale_index_by_observation.len() != n)
        {
            return Err(ModelError::MismatchedDataLengths);
        }
        if !fixed_coefficient_scale.is_finite() || fixed_coefficient_scale <= 0.0 {
            return Err(ModelError::InvalidParameter(
                "prepared_grouped_fixed_coefficient_scale_invalid",
            ));
        }
        if coefficients.iter().any(|v| !v.is_finite())
            || log_local_scales
                .iter()
                .chain(log_global_scales.iter())
                .chain(log_experiment_scales.iter())
                .any(|v| !v.is_finite() || v.abs() > 300.0)
        {
            return Err(ModelError::InvalidParameter(
                "prepared_grouped_parameter_out_of_supported_range",
            ));
        }
        let prior_scale = match global_scale_prior {
            GlobalScalePrior::HalfNormal { scale } | GlobalScalePrior::HalfCauchy { scale } => {
                scale
            }
        };
        if !prior_scale.is_finite() || prior_scale <= 0.0 {
            return Err(ModelError::InvalidParameter(
                "prepared_grouped_global_prior_scale_invalid",
            ));
        }

        let local_scales: Vec<f64> = log_local_scales.iter().map(|v| v.exp()).collect();
        let global_scales: Vec<f64> = log_global_scales.iter().map(|v| v.exp()).collect();
        if local_scales
            .iter()
            .chain(global_scales.iter())
            .any(|v| !v.is_finite() || *v <= 0.0)
        {
            return Err(ModelError::InvalidParameter(
                "prepared_grouped_scale_invalid",
            ));
        }
        let mut group_counts = vec![0usize; groups];
        for group in global_scale_index_by_coefficient {
            if *group >= groups {
                return Err(ModelError::InvalidParameter(
                    "prepared_grouped_coefficient_group_out_of_range",
                ));
            }
            group_counts[*group] += 1;
        }
        if group_counts.contains(&0) {
            return Err(ModelError::InvalidParameter(
                "prepared_grouped_global_scale_group_empty",
            ));
        }

        let mut scale_by_observation = vec![1.0; n];
        let mut experiment_scales = Vec::with_capacity(experiment_groups);
        let mut experiment_counts = vec![0usize; experiment_groups];
        for log_scale in log_experiment_scales {
            let scale = log_scale.exp();
            if !scale.is_finite() || !(0.5..=1.5).contains(&scale) {
                return Err(ModelError::InvalidParameter(
                    "experiment_scale_outside_aiache_bounds",
                ));
            }
            experiment_scales.push(scale);
        }
        if experiment_groups > 0 {
            for (obs_index, group) in experiment_scale_index_by_observation.iter().enumerate() {
                if *group >= experiment_groups {
                    return Err(ModelError::InvalidParameter(
                        "prepared_grouped_experiment_scale_index_out_of_range",
                    ));
                }
                scale_by_observation[obs_index] = experiment_scales[*group];
                experiment_counts[*group] += 1;
            }
            if experiment_counts.contains(&0) {
                return Err(ModelError::InvalidParameter(
                    "prepared_grouped_experiment_scale_group_empty",
                ));
            }
        }

        let mut prediction = vec![0.0; n];
        for (row_index, value) in prediction.iter_mut().enumerate() {
            let delta: f64 = self.design_matrix[row_index]
                .iter()
                .zip(coefficients.iter())
                .map(|(basis, beta)| basis * beta)
                .sum();
            if !delta.is_finite() || delta.abs() > 700.0 {
                return Err(ModelError::InvalidParameter(
                    "prepared_grouped_bias_predictor_out_of_range",
                ));
            }
            *value = self.baseline_mean[row_index] * scale_by_observation[row_index] * delta.exp();
            if !value.is_finite() || *value <= 0.0 {
                return Err(ModelError::InvalidParameter(
                    "prepared_grouped_biased_prediction_invalid",
                ));
            }
        }

        let residual: Vec<f64> = self
            .observed
            .iter()
            .zip(prediction.iter())
            .map(|(y, mu)| y - mu)
            .collect();
        let weighted = solve_cholesky(&self.covariance_lower, &residual)?;
        let quadratic = dot(&residual, &weighted);
        if !quadratic.is_finite() || quadratic < -1.0e-9 {
            return Err(ModelError::InvalidCovariance(
                "prepared_grouped_likelihood_quadratic_invalid",
            ));
        }
        let log_likelihood = -0.5
            * (n as f64 * (2.0 * std::f64::consts::PI).ln()
                + self.covariance_log_determinant
                + quadratic.max(0.0));
        let log_mean_gradient: Vec<f64> = prediction
            .iter()
            .zip(weighted.iter())
            .map(|(mu, r)| mu * r)
            .collect();

        let mut coefficient_gradient = vec![0.0; p];
        for (feature, gradient) in coefficient_gradient.iter_mut().enumerate() {
            *gradient = self
                .design_matrix
                .iter()
                .zip(log_mean_gradient.iter())
                .map(|(row, value)| row[feature] * value)
                .sum();
        }

        let mut log_prior = 0.0;
        let mut local_gradient = vec![0.0; p];
        let mut global_gradient = vec![0.0; groups];
        let mut experiment_gradient = vec![0.0; experiment_groups];
        let mut group_sum_q = vec![0.0; groups];
        const DATA_SCALE_STD: f64 = 0.01;
        for (index, scale) in experiment_scales.iter().enumerate() {
            let z = (scale - 1.0) / DATA_SCALE_STD;
            log_prior +=
                -0.5 * z * z - DATA_SCALE_STD.ln() - 0.5 * (2.0 * std::f64::consts::PI).ln()
                    + log_experiment_scales[index];
            experiment_gradient[index] = -(scale - 1.0) * scale / DATA_SCALE_STD.powi(2) + 1.0;
        }
        for (obs_index, group) in experiment_scale_index_by_observation.iter().enumerate() {
            if experiment_groups > 0 {
                experiment_gradient[*group] += log_mean_gradient[obs_index];
            }
        }

        for feature in 0..p {
            let group = global_scale_index_by_coefficient[feature];
            let local = local_scales[feature];
            let global = global_scales[group];
            let scale = fixed_coefficient_scale * global * local;
            if !scale.is_finite() || scale <= 0.0 {
                return Err(ModelError::InvalidParameter(
                    "prepared_grouped_coefficient_scale_invalid",
                ));
            }
            let beta = coefficients[feature];
            let q = (beta / scale).powi(2);
            let local_sq = local * local;
            if !q.is_finite() || !local_sq.is_finite() {
                return Err(ModelError::InvalidParameter(
                    "prepared_grouped_prior_overflow",
                ));
            }
            group_sum_q[group] += q;
            log_prior += -0.5 * q - scale.ln() - 0.5 * (2.0 * std::f64::consts::PI).ln()
                + (2.0 / std::f64::consts::PI).ln()
                - (1.0 + local_sq).ln()
                + log_local_scales[feature];
            coefficient_gradient[feature] -= beta / (scale * scale);
            local_gradient[feature] = q - 2.0 * local_sq / (1.0 + local_sq);
        }

        for group in 0..groups {
            let ratio = global_scales[group] / prior_scale;
            let ratio_sq = ratio * ratio;
            if !ratio_sq.is_finite() || !group_sum_q[group].is_finite() {
                return Err(ModelError::InvalidParameter(
                    "prepared_grouped_global_prior_overflow",
                ));
            }
            match global_scale_prior {
                GlobalScalePrior::HalfNormal { scale } => {
                    log_prior +=
                        0.5 * (2.0 / std::f64::consts::PI).ln() - scale.ln() - 0.5 * ratio_sq
                            + log_global_scales[group];
                    global_gradient[group] =
                        group_sum_q[group] - group_counts[group] as f64 + 1.0 - ratio_sq;
                }
                GlobalScalePrior::HalfCauchy { scale } => {
                    log_prior +=
                        (2.0 / std::f64::consts::PI).ln() - scale.ln() - (1.0 + ratio_sq).ln()
                            + log_global_scales[group];
                    global_gradient[group] = group_sum_q[group] - group_counts[group] as f64 + 1.0
                        - 2.0 * ratio_sq / (1.0 + ratio_sq);
                }
            }
        }
        if !log_likelihood.is_finite()
            || !log_prior.is_finite()
            || coefficient_gradient.iter().any(|v| !v.is_finite())
            || local_gradient.iter().any(|v| !v.is_finite())
            || global_gradient.iter().any(|v| !v.is_finite())
            || experiment_gradient.iter().any(|v| !v.is_finite())
        {
            return Err(ModelError::InvalidParameter(
                "prepared_grouped_horseshoe_posterior_nonfinite",
            ));
        }
        Ok(GroupedRawScaleHorseshoePosterior {
            log_likelihood,
            log_prior,
            log_posterior: log_likelihood + log_prior,
            coefficient_gradient,
            log_local_scale_gradient: local_gradient,
            log_global_scale_gradients: global_gradient,
            log_experiment_scale_gradients: experiment_gradient,
            biased_prediction: prediction,
        })
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
    fn prepared_grouped_model_matches_unprepared_posterior_and_gradients() {
        let observed = [12.0, 18.0];
        let baseline = [10.0, 20.0];
        let design = vec![vec![1.0, 0.3, -0.2], vec![0.2, -0.5, 0.4]];
        let coefficients = [0.001, -0.002, 0.003];
        let log_local = [0.8_f64.ln(), 1.1_f64.ln(), 0.9_f64.ln()];
        let log_global = [0.6_f64.ln(), 1.0_f64.ln(), 0.8_f64.ln()];
        let groups = [0, 1, 2];
        let log_experiment = [1.02_f64.ln()];
        let experiment_map = [0, 0];
        let covariance = vec![vec![0.04, 0.005], vec![0.005, 0.09]];
        let prior = GlobalScalePrior::HalfNormal { scale: 1.0 };

        let prepared =
            PreparedGroupedRawScaleHorseshoeModel::new(&observed, &baseline, &design, &covariance)
                .expect("prepared grouped model");
        let prepared_result = prepared
            .evaluate(
                &coefficients,
                &log_local,
                &log_global,
                &groups,
                &log_experiment,
                &experiment_map,
                1.0e-2,
                prior,
            )
            .expect("prepared posterior");
        let unprepared_result =
            grouped_raw_scale_horseshoe_log_posterior(GroupedHorseshoePosteriorInput {
                observed: &observed,
                baseline_mean: &baseline,
                design_matrix: &design,
                coefficients: &coefficients,
                log_local_scales: &log_local,
                log_global_scales: &log_global,
                global_scale_index_by_coefficient: &groups,
                log_experiment_scales: &log_experiment,
                experiment_scale_index_by_observation: &experiment_map,
                relative_covariance: &covariance,
                fixed_coefficient_scale: 1.0e-2,
                global_scale_prior: prior,
            })
            .expect("unprepared grouped target");

        assert!(
            (prepared_result.log_likelihood - unprepared_result.log_likelihood).abs() < 1.0e-10
        );
        assert!((prepared_result.log_posterior - unprepared_result.log_posterior).abs() < 1.0e-10);
        assert_eq!(
            prepared_result.biased_prediction,
            unprepared_result.biased_prediction
        );
        assert_eq!(
            prepared_result.coefficient_gradient,
            unprepared_result.coefficient_gradient
        );
        assert_eq!(
            prepared_result.log_local_scale_gradient,
            unprepared_result.log_local_scale_gradient
        );
        assert_eq!(
            prepared_result.log_global_scale_gradients,
            unprepared_result.log_global_scale_gradients
        );
        assert_eq!(
            prepared_result.log_experiment_scale_gradients,
            unprepared_result.log_experiment_scale_gradients
        );
    }

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
            let grouped =
                grouped_raw_scale_horseshoe_log_posterior(GroupedHorseshoePosteriorInput {
                    observed: &observed,
                    baseline_mean: &baseline,
                    design_matrix: &design,
                    coefficients: &coefficients,
                    log_local_scales: &log_local,
                    log_global_scales: &[log_global],
                    global_scale_index_by_coefficient: &vec![0; coefficients.len()],
                    log_experiment_scales: &[],
                    experiment_scale_index_by_observation: &[],
                    relative_covariance: &covariance,
                    fixed_coefficient_scale: 1.0,
                    global_scale_prior: prior,
                })
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
            assert!(
                (single.log_global_scale_gradient - grouped.log_global_scale_gradients[0]).abs()
                    < 1.0e-10
            );
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
                log_experiment_scales: &[],
                experiment_scale_index_by_observation: &[],
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

    #[test]
    fn experiment_scale_gradient_matches_finite_difference_and_prior_bounds() {
        let observed = [12.0, 18.0];
        let baseline = [10.0, 20.0];
        let design = vec![vec![1.0, 0.3, -0.2], vec![0.2, -0.5, 0.4]];
        let coefficients = [0.001, -0.002, 0.003];
        let log_local = [0.8_f64.ln(), 1.1_f64.ln(), 0.9_f64.ln()];
        let log_global = [0.6_f64.ln(), 1.0_f64.ln(), 0.8_f64.ln()];
        let groups = [0, 1, 2];
        let experiment_map = [0, 0];
        let covariance = vec![vec![0.04, 0.005], vec![0.005, 0.09]];
        let prior = GlobalScalePrior::HalfNormal { scale: 1.0 };
        let evaluate = |log_scale: f64| {
            grouped_raw_scale_horseshoe_log_posterior(GroupedHorseshoePosteriorInput {
                observed: &observed,
                baseline_mean: &baseline,
                design_matrix: &design,
                coefficients: &coefficients,
                log_local_scales: &log_local,
                log_global_scales: &log_global,
                global_scale_index_by_coefficient: &groups,
                log_experiment_scales: &[log_scale],
                experiment_scale_index_by_observation: &experiment_map,
                relative_covariance: &covariance,
                fixed_coefficient_scale: 1.0e-2,
                global_scale_prior: prior,
            })
            .expect("experiment scale target")
        };
        let log_scale = 1.02_f64.ln();
        let result = evaluate(log_scale);
        let epsilon = 1.0e-6;
        let finite_difference = (evaluate(log_scale + epsilon).log_posterior
            - evaluate(log_scale - epsilon).log_posterior)
            / (2.0 * epsilon);
        assert!((finite_difference - result.log_experiment_scale_gradients[0]).abs() < 1.0e-4);
    }

    #[test]
    fn grouped_target_rejects_experiment_scale_outside_aiache_bounds() {
        let observed = [12.0, 18.0];
        let baseline = [10.0, 20.0];
        let design = vec![vec![1.0, 0.3, -0.2], vec![0.2, -0.5, 0.4]];
        let coefficients = [0.001, -0.002, 0.003];
        let log_local = [0.8_f64.ln(), 1.1_f64.ln(), 0.9_f64.ln()];
        let log_global = [0.6_f64.ln(), 1.0_f64.ln(), 0.8_f64.ln()];
        let groups = [0, 1, 2];
        let experiment_map = [0, 0];
        let covariance = vec![vec![0.04, 0.005], vec![0.005, 0.09]];
        assert_eq!(
            grouped_raw_scale_horseshoe_log_posterior(GroupedHorseshoePosteriorInput {
                observed: &observed,
                baseline_mean: &baseline,
                design_matrix: &design,
                coefficients: &coefficients,
                log_local_scales: &log_local,
                log_global_scales: &log_global,
                global_scale_index_by_coefficient: &groups,
                log_experiment_scales: &[0.4_f64.ln()],
                experiment_scale_index_by_observation: &experiment_map,
                relative_covariance: &covariance,
                fixed_coefficient_scale: 1.0e-2,
                global_scale_prior: GlobalScalePrior::HalfNormal { scale: 1.0 },
            }),
            Err(ModelError::InvalidParameter(
                "experiment_scale_outside_aiache_bounds",
            ))
        );
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
