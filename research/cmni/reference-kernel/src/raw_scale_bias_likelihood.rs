//! Raw-scale correlated Gaussian likelihood for multiplicative PFNS bias.
//!
//! Observation model for one experiment:
//!   y ~ Normal(mu .* exp(delta), V)
//!   V = diag(mu) * Sigma_relative * diag(mu)
//!
//! The relative covariance is anchored to the un-biased baseline prediction mu.
//! The multiplicative bias changes only the predicted mean, not the covariance.
//! The baseline mean should already include the declared experiment scale.
//!
//! This is a likelihood primitive only, not the AIACHNE horseshoe prior or BMC.

use crate::ModelError;

#[derive(Debug, Clone, PartialEq)]
pub struct RawScaleGaussianBiasLikelihood {
    pub log_likelihood: f64,
    pub quadratic_form: f64,
    pub log_determinant_covariance: f64,
    pub residual: Vec<f64>,
    pub biased_prediction: Vec<f64>,
}

/// Evaluate a raw-scale correlated Gaussian likelihood for one experiment.
///
/// relative_covariance is the experiment-level covariance of relative errors.
/// baseline_mean includes any separately declared per-experiment scale.
/// bias_log_field is the energy-dependent log-bias delta(E).
///
/// Covariance = diag(baseline_mean) * relative_covariance * diag(baseline_mean).
pub fn raw_scale_gaussian_multiplicative_bias_log_likelihood(
    observed: &[f64],
    baseline_mean: &[f64],
    bias_log_field: &[f64],
    relative_covariance: &[Vec<f64>],
) -> Result<RawScaleGaussianBiasLikelihood, ModelError> {
    let dimension = observed.len();
    if dimension == 0
        || baseline_mean.len() != dimension
        || bias_log_field.len() != dimension
        || relative_covariance.len() != dimension
        || relative_covariance.iter().any(|row| row.len() != dimension)
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    for value in observed {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue("raw_scale_observations"));
        }
        if *value < 0.0 {
            return Err(ModelError::InvalidParameter(
                "raw_scale_observations_must_be_nonnegative",
            ));
        }
    }
    for value in baseline_mean {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue("raw_scale_baseline_mean"));
        }
        if *value <= 0.0 {
            return Err(ModelError::NonPositiveValue("raw_scale_baseline_mean"));
        }
    }
    for value in bias_log_field {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue("raw_scale_bias_log_field"));
        }
    }
    for value in relative_covariance.iter().flatten() {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue("raw_scale_relative_covariance"));
        }
    }

    let covariance_scale = relative_covariance
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    let symmetry_tolerance = 1.0e-10 * covariance_scale.max(f64::MIN_POSITIVE);
    for (row_index, row) in relative_covariance.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate().take(row_index) {
            if (value - relative_covariance[column_index][row_index]).abs() > symmetry_tolerance {
                return Err(ModelError::InvalidCovariance(
                    "raw_scale_relative_covariance_must_be_symmetric",
                ));
            }
        }
    }

    let mut covariance = vec![vec![0.0; dimension]; dimension];
    let mut biased_prediction = vec![0.0; dimension];
    for row in 0..dimension {
        let exponent = bias_log_field[row];
        if exponent > f64::MAX.ln() || exponent < -745.0 {
            return Err(ModelError::InvalidParameter(
                "raw_scale_bias_exponent_out_of_range",
            ));
        }
        let multiplier = exponent.exp();
        let prediction = baseline_mean[row] * multiplier;
        if !multiplier.is_finite() || !prediction.is_finite() || prediction <= 0.0 {
            return Err(ModelError::InvalidParameter(
                "raw_scale_biased_prediction_invalid",
            ));
        }
        biased_prediction[row] = prediction;
        for column in 0..dimension {
            covariance[row][column] =
                baseline_mean[row] * relative_covariance[row][column] * baseline_mean[column];
        }
    }

    let lower = cholesky(&covariance)?;
    let log_determinant_covariance = log_determinant_from_cholesky(&lower)?;
    let residual: Vec<f64> = observed
        .iter()
        .zip(biased_prediction.iter())
        .map(|(value, prediction)| value - prediction)
        .collect();
    let whitened = solve_cholesky(&lower, &residual)?;
    let quadratic_form = dot(&residual, &whitened);
    if !quadratic_form.is_finite() || quadratic_form < -1.0e-9 {
        return Err(ModelError::InvalidCovariance(
            "raw_scale_gaussian_quadratic_form_invalid",
        ));
    }
    let quadratic_form = quadratic_form.max(0.0);
    let log_likelihood = -0.5
        * (dimension as f64 * (2.0 * std::f64::consts::PI).ln()
            + log_determinant_covariance
            + quadratic_form);
    if !log_likelihood.is_finite() {
        return Err(ModelError::InvalidParameter(
            "raw_scale_gaussian_log_likelihood_nonfinite",
        ));
    }

    Ok(RawScaleGaussianBiasLikelihood {
        log_likelihood,
        quadratic_form,
        log_determinant_covariance,
        residual,
        biased_prediction,
    })
}

fn cholesky(matrix: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, ModelError> {
    let dimension = matrix.len();
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
                        "raw_scale_covariance_must_be_positive_definite",
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
            "raw_scale_covariance_solve_nonfinite",
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
            "raw_scale_covariance_logdet_nonfinite",
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
    fn zero_bias_has_zero_residual() {
        let result = raw_scale_gaussian_multiplicative_bias_log_likelihood(
            &[10.0, 20.0],
            &[10.0, 20.0],
            &[0.0, 0.0],
            &[vec![0.04, 0.01], vec![0.01, 0.09]],
        )
        .expect("zero bias");
        assert_eq!(result.biased_prediction, vec![10.0, 20.0]);
        assert_eq!(result.residual, vec![0.0, 0.0]);
        assert!(result.quadratic_form.abs() < 1.0e-20);
        assert!(result.log_likelihood.is_finite());
    }

    #[test]
    fn bias_changes_mean_but_keeps_baseline_covariance() {
        let covariance = vec![vec![0.04, 0.01], vec![0.01, 0.09]];
        let zero = raw_scale_gaussian_multiplicative_bias_log_likelihood(
            &[10.0, 20.0],
            &[10.0, 20.0],
            &[0.0, 0.0],
            &covariance,
        )
        .expect("zero bias");
        let biased = raw_scale_gaussian_multiplicative_bias_log_likelihood(
            &[10.0, 20.0],
            &[10.0, 20.0],
            &[0.1, -0.1],
            &covariance,
        )
        .expect("biased");
        assert_ne!(biased.biased_prediction, zero.biased_prediction);
        assert_eq!(
            biased.log_determinant_covariance,
            zero.log_determinant_covariance
        );
    }

    #[test]
    fn full_relative_covariance_changes_quadratic_form() {
        let diagonal = raw_scale_gaussian_multiplicative_bias_log_likelihood(
            &[11.0, 19.0],
            &[10.0, 20.0],
            &[0.0, 0.0],
            &[vec![0.04, 0.0], vec![0.0, 0.09]],
        )
        .expect("diagonal");
        let correlated = raw_scale_gaussian_multiplicative_bias_log_likelihood(
            &[11.0, 19.0],
            &[10.0, 20.0],
            &[0.0, 0.0],
            &[vec![0.04, 0.02], vec![0.02, 0.09]],
        )
        .expect("correlated");
        assert!((diagonal.quadratic_form - correlated.quadratic_form).abs() > 1.0e-6);
    }

    #[test]
    fn covariance_is_anchored_to_unbiased_baseline_mean() {
        let result = raw_scale_gaussian_multiplicative_bias_log_likelihood(
            &[12.0, 18.0],
            &[10.0, 20.0],
            &[0.2, -0.2],
            &[vec![0.04, 0.01], vec![0.01, 0.09]],
        )
        .expect("biased likelihood");
        let expected_determinant = 140.0_f64.ln();
        assert!((result.log_determinant_covariance - expected_determinant).abs() < 1.0e-10);
    }

    #[test]
    fn rejects_asymmetric_or_non_positive_definite_covariance() {
        assert_eq!(
            raw_scale_gaussian_multiplicative_bias_log_likelihood(
                &[1.0, 2.0],
                &[1.0, 2.0],
                &[0.0, 0.0],
                &[vec![1.0, 0.2], vec![0.1, 1.0]],
            ),
            Err(ModelError::InvalidCovariance(
                "raw_scale_relative_covariance_must_be_symmetric"
            ))
        );
        assert_eq!(
            raw_scale_gaussian_multiplicative_bias_log_likelihood(
                &[1.0, 2.0],
                &[1.0, 2.0],
                &[0.0, 0.0],
                &[vec![1.0, 2.0], vec![2.0, 1.0]],
            ),
            Err(ModelError::InvalidCovariance(
                "raw_scale_covariance_must_be_positive_definite"
            ))
        );
    }

    #[test]
    fn rejects_invalid_dimensions_observations_and_exponents() {
        assert_eq!(
            raw_scale_gaussian_multiplicative_bias_log_likelihood(
                &[1.0],
                &[1.0, 2.0],
                &[0.0],
                &[vec![1.0]],
            ),
            Err(ModelError::MismatchedDataLengths)
        );
        assert_eq!(
            raw_scale_gaussian_multiplicative_bias_log_likelihood(
                &[-1.0],
                &[1.0],
                &[0.0],
                &[vec![1.0]],
            ),
            Err(ModelError::InvalidParameter(
                "raw_scale_observations_must_be_nonnegative"
            ))
        );
        assert_eq!(
            raw_scale_gaussian_multiplicative_bias_log_likelihood(
                &[1.0],
                &[1.0],
                &[1000.0],
                &[vec![1.0]],
            ),
            Err(ModelError::InvalidParameter(
                "raw_scale_bias_exponent_out_of_range"
            ))
        );
    }
}
