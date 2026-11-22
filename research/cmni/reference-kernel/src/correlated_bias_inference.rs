//! Correlated Gaussian log-ratio Bayesian bias inference.
//!
//! Model:
//!   z = F alpha + X_S beta_S + epsilon
//!   epsilon ~ Normal(0, C_log)
//!
//! Fixed-effect coefficients alpha have proper zero-centred Gaussian priors.
//! Sparse bias coefficients beta have Gaussian slab priors and Bernoulli
//! inclusion indicators. All subsets are enumerated up to 12 bias columns.
//!
//! Unlike the Poisson-count comparator, this path supports full observation
//! covariance and is more suitable for PFNS-shaped relative measurements after
//! a declared log-ratio transformation. It remains an approximation to the
//! raw-scale AIACHNE likelihood, not a reproduction of its BMC / horseshoe model.

use crate::ModelError;

const MAX_GAUSSIAN_BIAS_FEATURES: usize = 12;

#[derive(Debug, Clone, PartialEq)]
pub struct CorrelatedLogBiasResult {
    pub fixed_effect_posterior_mean: Vec<f64>,
    pub bias_coefficient_posterior_mean: Vec<f64>,
    /// Full covariance in [fixed effects, sparse bias coefficients] order.
    pub parameter_posterior_covariance: Vec<Vec<f64>>,
    pub feature_inclusion_probability: Vec<f64>,
    /// Posterior probability per subset; index is the sparse-feature bit mask.
    pub model_posterior_probability: Vec<f64>,
    pub map_fixed_effect_coefficients: Vec<f64>,
    pub map_bias_coefficients: Vec<f64>,
    pub map_feature_mask: usize,
    pub map_model_probability: f64,
    /// Exact model evidence under the declared Gaussian likelihood and priors.
    pub log_model_evidence: f64,
    pub enumerated_model_count: usize,
}

/// Exact Bayesian model averaging over small sparse-feature subsets under a
/// correlated Gaussian log-ratio likelihood.
///
/// log_relative_observations should contain log(observed / baseline) values
/// or another explicitly declared log-space residual. fixed_effect_design may
/// contain per-experiment normalization intercepts or other always-included
/// nuisance terms. sparse_bias_design contains feature-level / energy-basis
/// bias predictors subject to subset selection.
///
/// The covariance is expressed in the same log-ratio space and may be fully
/// correlated. If a source supplies relative covariance on the raw PFNS scale,
/// a first-order delta-method transformation may be appropriate for modest
/// relative uncertainties, but the transformed covariance must be constructed
/// and documented by the caller.
///
/// All coefficients have proper Gaussian priors. This is an exact finite
/// spike-and-slab model average with analytic Gaussian integration, not a
/// horseshoe prior, not Bayesian Monte Carlo, and not a causal attribution.
pub fn fit_sparse_bayesian_correlated_log_bias(
    log_relative_observations: &[f64],
    fixed_effect_design: &[Vec<f64>],
    sparse_bias_design: &[Vec<f64>],
    log_relative_covariance: &[Vec<f64>],
    feature_inclusion_probability: f64,
    fixed_effect_prior_standard_deviation: f64,
    bias_slab_standard_deviation: f64,
) -> Result<CorrelatedLogBiasResult, ModelError> {
    let observation_count = log_relative_observations.len();
    if observation_count == 0
        || fixed_effect_design.len() != observation_count
        || sparse_bias_design.len() != observation_count
        || log_relative_covariance.len() != observation_count
        || log_relative_covariance
            .iter()
            .any(|row| row.len() != observation_count)
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    if !feature_inclusion_probability.is_finite() {
        return Err(ModelError::NonFiniteValue("feature_inclusion_probability"));
    }
    if feature_inclusion_probability <= 0.0 || feature_inclusion_probability >= 1.0 {
        return Err(ModelError::InvalidParameter(
            "feature_inclusion_probability_must_be_between_zero_and_one",
        ));
    }
    for (value, name) in [
        (
            fixed_effect_prior_standard_deviation,
            "fixed_effect_prior_standard_deviation",
        ),
        (bias_slab_standard_deviation, "bias_slab_standard_deviation"),
    ] {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue(name));
        }
        if value <= 0.0 {
            return Err(ModelError::NonPositiveValue(name));
        }
    }

    let fixed_count = fixed_effect_design.first().map_or(0, Vec::len);
    let bias_count = sparse_bias_design.first().map_or(0, Vec::len);
    if fixed_effect_design
        .iter()
        .any(|row| row.len() != fixed_count)
        || sparse_bias_design.iter().any(|row| row.len() != bias_count)
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    if bias_count == 0 {
        return Err(ModelError::MismatchedDataLengths);
    }
    if bias_count > MAX_GAUSSIAN_BIAS_FEATURES {
        return Err(ModelError::InvalidParameter(
            "correlated_log_bias_feature_count_exceeds_exact_enumeration_limit",
        ));
    }
    if log_relative_observations
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(ModelError::NonFiniteValue("log_relative_observations"));
    }
    if fixed_effect_design
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return Err(ModelError::NonFiniteValue("fixed_effect_design"));
    }
    if sparse_bias_design
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return Err(ModelError::NonFiniteValue("sparse_bias_design"));
    }
    if log_relative_covariance
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return Err(ModelError::NonFiniteValue("log_relative_covariance"));
    }

    validate_symmetric(log_relative_covariance)?;
    let covariance_lower = cholesky(log_relative_covariance)?;
    let logdet_covariance = log_determinant_from_cholesky(&covariance_lower)?;
    let inverse_covariance = inverse_from_cholesky(&covariance_lower)?;
    let inverse_times_observations =
        matrix_vector_product(&inverse_covariance, log_relative_observations)?;
    let observation_quadratic = dot(log_relative_observations, &inverse_times_observations);

    let model_count = 1usize << bias_count;
    let log_included = feature_inclusion_probability.ln();
    let log_excluded = (1.0 - feature_inclusion_probability).ln();
    let fixed_precision =
        1.0 / (fixed_effect_prior_standard_deviation * fixed_effect_prior_standard_deviation);
    let bias_precision = 1.0 / (bias_slab_standard_deviation * bias_slab_standard_deviation);
    if !fixed_precision.is_finite()
        || fixed_precision <= 0.0
        || !bias_precision.is_finite()
        || bias_precision <= 0.0
    {
        return Err(ModelError::InvalidParameter(
            "correlated_log_bias_prior_precision_invalid",
        ));
    }

    let mut fits = Vec::with_capacity(model_count);
    for mask in 0..model_count {
        let active = active_features(mask, bias_count);
        let mut fit = fit_gaussian_subset(
            log_relative_observations,
            fixed_effect_design,
            sparse_bias_design,
            &active,
            &inverse_covariance,
            observation_quadratic,
            logdet_covariance,
            fixed_precision,
            bias_precision,
            fixed_effect_prior_standard_deviation,
            bias_slab_standard_deviation,
        )?;
        fit.log_model_evidence +=
            active.len() as f64 * log_included + (bias_count - active.len()) as f64 * log_excluded;
        fits.push(fit);
    }

    let max_log_weight = fits
        .iter()
        .map(|fit| fit.log_model_evidence)
        .fold(f64::NEG_INFINITY, f64::max);
    if !max_log_weight.is_finite() {
        return Err(ModelError::InvalidParameter(
            "correlated_log_bias_model_evidence_nonfinite",
        ));
    }
    let sum_weights = fits
        .iter()
        .map(|fit| (fit.log_model_evidence - max_log_weight).exp())
        .sum::<f64>();
    if !sum_weights.is_finite() || sum_weights <= 0.0 {
        return Err(ModelError::InvalidParameter(
            "correlated_log_bias_model_weights_invalid",
        ));
    }
    let log_model_evidence = max_log_weight + sum_weights.ln();
    let model_probabilities: Vec<f64> = fits
        .iter()
        .map(|fit| (fit.log_model_evidence - max_log_weight).exp() / sum_weights)
        .collect();

    let parameter_count = fixed_count + bias_count;
    let mut parameter_mean = vec![0.0; parameter_count];
    let mut feature_inclusion_probability_out = vec![0.0; bias_count];
    let mut map_parameter_mean = vec![0.0; parameter_count];
    let mut map_feature_mask = 0usize;
    let mut map_model_probability = -1.0_f64;

    for (mask, (fit, probability)) in fits.iter().zip(model_probabilities.iter()).enumerate() {
        for (local, feature) in fit.active_bias_features.iter().enumerate() {
            feature_inclusion_probability_out[*feature] += probability;
            parameter_mean[fixed_count + *feature] +=
                probability * fit.parameter_mean[fixed_count + local];
        }
        for (fixed, value) in fit.parameter_mean.iter().take(fixed_count).enumerate() {
            parameter_mean[fixed] += probability * value;
        }
        if *probability > map_model_probability {
            map_model_probability = *probability;
            map_feature_mask = mask;
            map_parameter_mean.fill(0.0);
            map_parameter_mean[..fixed_count].copy_from_slice(&fit.parameter_mean[..fixed_count]);
            for (local, feature) in fit.active_bias_features.iter().enumerate() {
                map_parameter_mean[fixed_count + *feature] =
                    fit.parameter_mean[fixed_count + local];
            }
        }
    }

    let mut parameter_covariance = vec![vec![0.0; parameter_count]; parameter_count];
    for (fit, probability) in fits.iter().zip(model_probabilities.iter()) {
        let mut full_mean = vec![0.0; parameter_count];
        let mut full_covariance = vec![vec![0.0; parameter_count]; parameter_count];
        full_mean[..fixed_count].copy_from_slice(&fit.parameter_mean[..fixed_count]);
        for (fixed, full_row) in full_covariance.iter_mut().take(fixed_count).enumerate() {
            full_row[..fixed_count]
                .copy_from_slice(&fit.parameter_covariance[fixed][..fixed_count]);
        }
        for (local, feature) in fit.active_bias_features.iter().enumerate() {
            let global = fixed_count + *feature;
            full_mean[global] = fit.parameter_mean[fixed_count + local];
            for fixed in 0..fixed_count {
                full_covariance[fixed][global] =
                    fit.parameter_covariance[fixed][fixed_count + local];
                full_covariance[global][fixed] =
                    fit.parameter_covariance[fixed_count + local][fixed];
            }
            for (local_other, feature_other) in fit.active_bias_features.iter().enumerate() {
                let global_other = fixed_count + *feature_other;
                full_covariance[global][global_other] =
                    fit.parameter_covariance[fixed_count + local][fixed_count + local_other];
            }
        }
        for left in 0..parameter_count {
            for right in 0..parameter_count {
                parameter_covariance[left][right] += probability
                    * (full_covariance[left][right]
                        + (full_mean[left] - parameter_mean[left])
                            * (full_mean[right] - parameter_mean[right]));
            }
        }
    }

    let fixed_effect_posterior_mean = parameter_mean[..fixed_count].to_vec();
    let bias_coefficient_posterior_mean = parameter_mean[fixed_count..].to_vec();
    let map_fixed_effect_coefficients = map_parameter_mean[..fixed_count].to_vec();
    let map_bias_coefficients = map_parameter_mean[fixed_count..].to_vec();

    Ok(CorrelatedLogBiasResult {
        fixed_effect_posterior_mean,
        bias_coefficient_posterior_mean,
        parameter_posterior_covariance: parameter_covariance,
        feature_inclusion_probability: feature_inclusion_probability_out,
        model_posterior_probability: model_probabilities,
        map_fixed_effect_coefficients,
        map_bias_coefficients,
        map_feature_mask,
        map_model_probability,
        log_model_evidence,
        enumerated_model_count: model_count,
    })
}

#[derive(Debug, Clone)]
struct GaussianSubsetFit {
    active_bias_features: Vec<usize>,
    parameter_mean: Vec<f64>,
    parameter_covariance: Vec<Vec<f64>>,
    log_model_evidence: f64,
}

#[allow(clippy::too_many_arguments)]
fn fit_gaussian_subset(
    observations: &[f64],
    fixed_design: &[Vec<f64>],
    bias_design: &[Vec<f64>],
    active_bias_features: &[usize],
    inverse_covariance: &[Vec<f64>],
    observation_quadratic: f64,
    logdet_covariance: f64,
    fixed_precision: f64,
    bias_precision: f64,
    fixed_sd: f64,
    bias_sd: f64,
) -> Result<GaussianSubsetFit, ModelError> {
    let observation_count = observations.len();
    let fixed_count = fixed_design[0].len();
    let active_count = active_bias_features.len();
    let parameter_count = fixed_count + active_count;

    let mut design = vec![vec![0.0; parameter_count]; observation_count];
    for row in 0..observation_count {
        for fixed in 0..fixed_count {
            design[row][fixed] = fixed_design[row][fixed];
        }
        for (local, feature) in active_bias_features.iter().enumerate() {
            design[row][fixed_count + local] = bias_design[row][*feature];
        }
    }

    let mut hessian = vec![vec![0.0; parameter_count]; parameter_count];
    let weighted_design = matrix_product(inverse_covariance, &design)?;
    for left in 0..parameter_count {
        for right in 0..parameter_count {
            hessian[left][right] = (0..observation_count)
                .map(|row| design[row][left] * weighted_design[row][right])
                .sum::<f64>();
        }
    }
    let mut prior_log_determinant = 0.0;
    for (index, row) in hessian.iter_mut().take(fixed_count).enumerate() {
        row[index] += fixed_precision;
        prior_log_determinant += 2.0 * fixed_sd.ln();
    }
    for (index, row) in hessian
        .iter_mut()
        .enumerate()
        .skip(fixed_count)
        .take(active_count)
    {
        row[index] += bias_precision;
        prior_log_determinant += 2.0 * bias_sd.ln();
    }

    let rhs = transpose_matrix_vector_product(
        &design,
        &matrix_vector_product(inverse_covariance, observations)?,
    )?;
    let (parameter_mean, parameter_covariance, logdet_hessian) = if parameter_count == 0 {
        (Vec::new(), Vec::new(), 0.0)
    } else {
        let lower = cholesky(&hessian)?;
        let mean = solve_cholesky(&lower, &rhs)?;
        let covariance = inverse_from_cholesky(&lower)?;
        let logdet = log_determinant_from_cholesky(&lower)?;
        (mean, covariance, logdet)
    };

    let explained_quadratic = if parameter_count == 0 {
        0.0
    } else {
        dot(&rhs, &parameter_mean)
    };
    let residual_quadratic = observation_quadratic - explained_quadratic;
    if !residual_quadratic.is_finite() || residual_quadratic < -1.0e-8 {
        return Err(ModelError::InvalidCovariance(
            "correlated_log_bias_residual_quadratic_invalid",
        ));
    }

    let log_model_evidence = -0.5
        * (observation_count as f64 * (2.0 * std::f64::consts::PI).ln()
            + logdet_covariance
            + prior_log_determinant
            + logdet_hessian
            + residual_quadratic.max(0.0));
    if !log_model_evidence.is_finite() {
        return Err(ModelError::InvalidParameter(
            "correlated_log_bias_log_evidence_nonfinite",
        ));
    }
    Ok(GaussianSubsetFit {
        active_bias_features: active_bias_features.to_vec(),
        parameter_mean,
        parameter_covariance,
        log_model_evidence,
    })
}

fn active_features(mask: usize, feature_count: usize) -> Vec<usize> {
    (0..feature_count)
        .filter(|feature| (mask & (1usize << feature)) != 0)
        .collect()
}

fn validate_symmetric(matrix: &[Vec<f64>]) -> Result<(), ModelError> {
    let scale = matrix
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    let tolerance = 1.0e-10 * scale.max(f64::MIN_POSITIVE);
    for (row_index, row) in matrix.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate().take(row_index) {
            if (value - matrix[column_index][row_index]).abs() > tolerance {
                return Err(ModelError::InvalidCovariance(
                    "log_relative_covariance_must_be_symmetric",
                ));
            }
        }
    }
    Ok(())
}

fn cholesky(matrix: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, ModelError> {
    let dimension = matrix.len();
    if dimension == 0 || matrix.iter().any(|row| row.len() != dimension) {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut lower = vec![vec![0.0; dimension]; dimension];
    for row in 0..dimension {
        for column in 0..=row {
            let mut value = matrix[row][column];
            for (inner, lower_value) in lower[row].iter().enumerate().take(column) {
                value -= lower_value * lower[column][inner];
            }
            if row == column {
                if !value.is_finite() || value <= 0.0 {
                    return Err(ModelError::InvalidCovariance(
                        "log_relative_covariance_must_be_positive_definite",
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
    if rhs.len() != lower.len() {
        return Err(ModelError::MismatchedDataLengths);
    }
    let dimension = lower.len();
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
            "log_relative_covariance_solve_nonfinite",
        ));
    }
    Ok(solution)
}

fn inverse_from_cholesky(lower: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, ModelError> {
    let dimension = lower.len();
    let mut inverse = vec![vec![0.0; dimension]; dimension];
    for column in 0..dimension {
        let mut unit = vec![0.0; dimension];
        unit[column] = 1.0;
        let solution = solve_cholesky(lower, &unit)?;
        for row in 0..dimension {
            inverse[row][column] = solution[row];
        }
    }
    let symmetric_inverse: Vec<Vec<f64>> = inverse
        .iter()
        .enumerate()
        .map(|(row_index, row)| {
            row.iter()
                .enumerate()
                .map(|(column_index, value)| 0.5 * (value + inverse[column_index][row_index]))
                .collect()
        })
        .collect();
    Ok(symmetric_inverse)
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
            "log_relative_covariance_logdet_nonfinite",
        ));
    }
    Ok(value)
}

fn matrix_product(left: &[Vec<f64>], right: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, ModelError> {
    if left.is_empty() || left[0].len() != right.len() {
        return Err(ModelError::MismatchedDataLengths);
    }
    let rows = left.len();
    let inner = right.len();
    let columns = right.first().map_or(0, Vec::len);
    if left.iter().any(|row| row.len() != inner) || right.iter().any(|row| row.len() != columns) {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut result = vec![vec![0.0; columns]; rows];
    for row in 0..rows {
        for index in 0..inner {
            for column in 0..columns {
                result[row][column] += left[row][index] * right[index][column];
            }
        }
    }
    Ok(result)
}

fn matrix_vector_product(matrix: &[Vec<f64>], vector: &[f64]) -> Result<Vec<f64>, ModelError> {
    if matrix.iter().any(|row| row.len() != vector.len()) {
        return Err(ModelError::MismatchedDataLengths);
    }
    Ok(matrix.iter().map(|row| dot(row, vector)).collect())
}

fn transpose_matrix_vector_product(
    matrix: &[Vec<f64>],
    vector: &[f64],
) -> Result<Vec<f64>, ModelError> {
    if matrix.len() != vector.len() {
        return Err(ModelError::MismatchedDataLengths);
    }
    let columns = matrix.first().map_or(0, Vec::len);
    if matrix.iter().any(|row| row.len() != columns) {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut result = vec![0.0; columns];
    for row in 0..matrix.len() {
        for column in 0..columns {
            result[column] += matrix[row][column] * vector[row];
        }
    }
    Ok(result)
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right.iter()).map(|(a, b)| a * b).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correlated_log_model_identifies_planted_bias_with_full_covariance() {
        let result = fit_sparse_bayesian_correlated_log_bias(
            &[0.0, std::f64::consts::LN_2, 0.0, std::f64::consts::LN_2],
            &[vec![], vec![], vec![], vec![]],
            &[vec![0.0], vec![1.0], vec![0.0], vec![1.0]],
            &[
                vec![0.01, 0.002, 0.0, 0.0],
                vec![0.002, 0.01, 0.0, 0.0],
                vec![0.0, 0.0, 0.01, 0.002],
                vec![0.0, 0.0, 0.002, 0.01],
            ],
            0.1,
            2.0,
            1.0,
        )
        .expect("correlated log-bias fit");
        assert!(result.feature_inclusion_probability[0] > 0.99);
        assert!(result.map_bias_coefficients[0] > 0.5);
        assert_eq!(result.map_feature_mask, 1);
        assert_eq!(result.enumerated_model_count, 2);
    }

    #[test]
    fn correlated_log_model_prefers_null_when_residuals_are_zero() {
        let result = fit_sparse_bayesian_correlated_log_bias(
            &[0.0, 0.0, 0.0, 0.0],
            &[vec![], vec![], vec![], vec![]],
            &[vec![0.0], vec![1.0], vec![0.0], vec![1.0]],
            &[
                vec![0.01, 0.002, 0.0, 0.0],
                vec![0.002, 0.01, 0.0, 0.0],
                vec![0.0, 0.0, 0.01, 0.002],
                vec![0.0, 0.0, 0.002, 0.01],
            ],
            0.1,
            2.0,
            1.0,
        )
        .expect("null log-bias fit");
        assert!(result.feature_inclusion_probability[0] < 0.05);
        assert_eq!(result.map_feature_mask, 0);
    }

    #[test]
    fn correlated_log_model_preserves_collinear_feature_ambiguity() {
        let result = fit_sparse_bayesian_correlated_log_bias(
            &[0.0, std::f64::consts::LN_2, 0.0, std::f64::consts::LN_2],
            &[vec![], vec![], vec![], vec![]],
            &[
                vec![0.0, 0.0],
                vec![1.0, 1.0],
                vec![0.0, 0.0],
                vec![1.0, 1.0],
            ],
            &[
                vec![0.01, 0.0, 0.0, 0.0],
                vec![0.0, 0.01, 0.0, 0.0],
                vec![0.0, 0.0, 0.01, 0.0],
                vec![0.0, 0.0, 0.0, 0.01],
            ],
            0.1,
            2.0,
            1.0,
        )
        .expect("collinear log-bias fit");
        assert!(
            (result.feature_inclusion_probability[0] - result.feature_inclusion_probability[1])
                .abs()
                < 1.0e-10
        );
        assert!(result.parameter_posterior_covariance[0][1] < 0.0);
    }

    #[test]
    fn fixed_effects_capture_experiment_normalization_separately() {
        let result = fit_sparse_bayesian_correlated_log_bias(
            &[0.2, 0.8931471805599453, 0.2, 0.8931471805599453],
            &[vec![1.0], vec![1.0], vec![1.0], vec![1.0]],
            &[vec![0.0], vec![1.0], vec![0.0], vec![1.0]],
            &[
                vec![0.01, 0.0, 0.0, 0.0],
                vec![0.0, 0.01, 0.0, 0.0],
                vec![0.0, 0.0, 0.01, 0.0],
                vec![0.0, 0.0, 0.0, 0.01],
            ],
            0.1,
            2.0,
            1.0,
        )
        .expect("fixed plus sparse");
        assert!((result.fixed_effect_posterior_mean[0] - 0.2).abs() < 0.05);
        assert!(result.feature_inclusion_probability[0] > 0.99);
    }

    #[test]
    fn posterior_model_probabilities_normalize_under_correlated_covariance() {
        let result = fit_sparse_bayesian_correlated_log_bias(
            &[0.1, -0.1, 0.05],
            &[vec![1.0], vec![1.0], vec![1.0]],
            &[vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0]],
            &[
                vec![0.04, 0.01, 0.005],
                vec![0.01, 0.05, 0.01],
                vec![0.005, 0.01, 0.03],
            ],
            0.2,
            3.0,
            0.5,
        )
        .expect("correlated model");
        assert!((result.model_posterior_probability.iter().sum::<f64>() - 1.0).abs() < 1.0e-12);
        assert_eq!(result.enumerated_model_count, 4);
        assert!(
            (result.parameter_posterior_covariance[0][1]
                - result.parameter_posterior_covariance[1][0])
                .abs()
                < 1.0e-12
        );
    }

    #[test]
    fn rejects_asymmetric_or_nonpositive_definite_covariance() {
        assert_eq!(
            fit_sparse_bayesian_correlated_log_bias(
                &[0.0, 0.1],
                &[vec![], vec![]],
                &[vec![1.0], vec![0.0]],
                &[vec![1.0, 0.2], vec![0.1, 1.0]],
                0.1,
                1.0,
                1.0,
            ),
            Err(ModelError::InvalidCovariance(
                "log_relative_covariance_must_be_symmetric"
            ))
        );
        assert_eq!(
            fit_sparse_bayesian_correlated_log_bias(
                &[0.0, 0.1],
                &[vec![], vec![]],
                &[vec![1.0], vec![0.0]],
                &[vec![1.0, 2.0], vec![2.0, 1.0]],
                0.1,
                1.0,
                1.0,
            ),
            Err(ModelError::InvalidCovariance(
                "log_relative_covariance_must_be_positive_definite"
            ))
        );
    }

    #[test]
    fn rejects_malformed_designs_and_feature_count_overflow() {
        assert_eq!(
            fit_sparse_bayesian_correlated_log_bias(
                &[0.0],
                &[vec![], vec![]],
                &[vec![1.0]],
                &[vec![1.0]],
                0.1,
                1.0,
                1.0,
            ),
            Err(ModelError::MismatchedDataLengths)
        );
        let features = vec![vec![0.0; 13], vec![0.0; 13]];
        assert_eq!(
            fit_sparse_bayesian_correlated_log_bias(
                &[0.0, 0.0],
                &[vec![], vec![]],
                &features,
                &[vec![1.0, 0.0], vec![0.0, 1.0]],
                0.1,
                1.0,
                1.0,
            ),
            Err(ModelError::InvalidParameter(
                "correlated_log_bias_feature_count_exceeds_exact_enumeration_limit",
            ))
        );
    }

    #[test]
    fn zero_feature_design_is_not_silently_accepted() {
        assert_eq!(
            fit_sparse_bayesian_correlated_log_bias(
                &[0.0],
                &[vec![]],
                &[vec![]],
                &[vec![1.0]],
                0.1,
                1.0,
                1.0,
            ),
            Err(ModelError::MismatchedDataLengths)
        );
    }
}
