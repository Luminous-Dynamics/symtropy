//! Prepared raw-scale horseshoe model with cached observation covariance factorization.
//!
//! The covariance and experiment geometry are fixed during posterior sampling.
//! This object factors diag(mu) * Sigma_relative * diag(mu) once, then reuses
//! that Cholesky factor for every posterior / gradient evaluation.

use crate::{GlobalScalePrior, ModelError, RawScaleHorseshoePosterior};

#[derive(Debug, Clone, PartialEq)]
pub struct PreparedRawScaleHorseshoeModel {
    observed: Vec<f64>,
    baseline_mean: Vec<f64>,
    design_matrix: Vec<Vec<f64>>,
    covariance_lower: Vec<Vec<f64>>,
    covariance_log_determinant: f64,
}

impl PreparedRawScaleHorseshoeModel {
    pub fn new(
        observed: &[f64],
        baseline_mean: &[f64],
        design_matrix: &[Vec<f64>],
        relative_covariance: &[Vec<f64>],
    ) -> Result<Self, ModelError> {
        let observation_count = observed.len();
        if observation_count == 0
            || baseline_mean.len() != observation_count
            || design_matrix.len() != observation_count
            || relative_covariance.len() != observation_count
            || relative_covariance
                .iter()
                .any(|row| row.len() != observation_count)
        {
            return Err(ModelError::MismatchedDataLengths);
        }
        let coefficient_count = design_matrix.first().map_or(0, Vec::len);
        if coefficient_count == 0
            || design_matrix
                .iter()
                .any(|row| row.len() != coefficient_count)
        {
            return Err(ModelError::MismatchedDataLengths);
        }
        for value in observed {
            if !value.is_finite() {
                return Err(ModelError::NonFiniteValue(
                    "prepared_horseshoe_observations",
                ));
            }
            if *value < 0.0 {
                return Err(ModelError::InvalidParameter(
                    "prepared_horseshoe_observations_must_be_nonnegative",
                ));
            }
        }
        for value in baseline_mean {
            if !value.is_finite() {
                return Err(ModelError::NonFiniteValue(
                    "prepared_horseshoe_baseline_mean",
                ));
            }
            if *value <= 0.0 {
                return Err(ModelError::NonPositiveValue(
                    "prepared_horseshoe_baseline_mean",
                ));
            }
        }
        if design_matrix
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
        {
            return Err(ModelError::NonFiniteValue(
                "prepared_horseshoe_design_matrix",
            ));
        }
        if relative_covariance
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
        {
            return Err(ModelError::NonFiniteValue(
                "prepared_horseshoe_relative_covariance",
            ));
        }
        let scale = relative_covariance
            .iter()
            .flatten()
            .map(|value| value.abs())
            .fold(0.0_f64, f64::max);
        let tolerance = 1.0e-10 * scale.max(f64::MIN_POSITIVE);
        for (row_index, row) in relative_covariance.iter().enumerate() {
            for (column_index, value) in row.iter().enumerate().take(row_index) {
                if (value - relative_covariance[column_index][row_index]).abs() > tolerance {
                    return Err(ModelError::InvalidCovariance(
                        "prepared_horseshoe_relative_covariance_must_be_symmetric",
                    ));
                }
            }
        }

        let mut covariance = vec![vec![0.0; observation_count]; observation_count];
        for row in 0..observation_count {
            for column in 0..observation_count {
                covariance[row][column] =
                    baseline_mean[row] * relative_covariance[row][column] * baseline_mean[column];
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

    pub fn observation_count(&self) -> usize {
        self.observed.len()
    }

    pub fn coefficient_count(&self) -> usize {
        self.design_matrix[0].len()
    }

    /// Evaluate the raw-scale horseshoe posterior and analytic gradient.
    /// Covariance Cholesky is cached from construction.
    pub fn evaluate(
        &self,
        coefficients: &[f64],
        log_local_scales: &[f64],
        log_global_scale: f64,
        global_scale_prior: GlobalScalePrior,
    ) -> Result<RawScaleHorseshoePosterior, ModelError> {
        let coefficient_count = self.coefficient_count();
        let observation_count = self.observation_count();
        if coefficients.len() != coefficient_count || log_local_scales.len() != coefficient_count {
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
        if coefficients.iter().any(|value| !value.is_finite()) {
            return Err(ModelError::NonFiniteValue("horseshoe_coefficients"));
        }
        let prior_scale = match global_scale_prior {
            GlobalScalePrior::HalfNormal { scale } | GlobalScalePrior::HalfCauchy { scale } => {
                scale
            }
        };
        if !prior_scale.is_finite() {
            return Err(ModelError::NonFiniteValue("global_scale_prior_scale"));
        }
        if prior_scale <= 0.0 {
            return Err(ModelError::NonPositiveValue("global_scale_prior_scale"));
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

        let mut biased_prediction = vec![0.0; observation_count];
        for (row, prediction) in biased_prediction.iter_mut().enumerate() {
            let delta = dot(&self.design_matrix[row], coefficients);
            if !delta.is_finite() || delta.abs() > 700.0 {
                return Err(ModelError::InvalidParameter(
                    "horseshoe_bias_predictor_out_of_range",
                ));
            }
            *prediction = self.baseline_mean[row] * delta.exp();
            if !prediction.is_finite() || *prediction <= 0.0 {
                return Err(ModelError::InvalidParameter(
                    "horseshoe_biased_prediction_invalid",
                ));
            }
        }
        let residual: Vec<f64> = self
            .observed
            .iter()
            .zip(biased_prediction.iter())
            .map(|(y, mean)| y - mean)
            .collect();
        let weighted_residual = solve_cholesky(&self.covariance_lower, &residual)?;
        let quadratic = dot(&residual, &weighted_residual);
        if !quadratic.is_finite() || quadratic < -1.0e-9 {
            return Err(ModelError::InvalidCovariance(
                "horseshoe_likelihood_quadratic_invalid",
            ));
        }
        let log_likelihood = -0.5
            * (observation_count as f64 * (2.0 * std::f64::consts::PI).ln()
                + self.covariance_log_determinant
                + quadratic.max(0.0));

        let log_mean_gradient: Vec<f64> = biased_prediction
            .iter()
            .zip(weighted_residual.iter())
            .map(|(prediction, residual)| prediction * residual)
            .collect();
        let mut coefficient_gradient = vec![0.0; coefficient_count];
        for (feature, gradient) in coefficient_gradient.iter_mut().enumerate() {
            for (design_row, log_gradient) in
                self.design_matrix.iter().zip(log_mean_gradient.iter())
            {
                *gradient += design_row[feature] * log_gradient;
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
            log_prior += -0.5 * q - scale.ln() - 0.5 * (2.0 * std::f64::consts::PI).ln()
                + (2.0 / std::f64::consts::PI).ln()
                - (1.0 + local_sq).ln()
                + log_local_scales[feature];
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
        let log_global_scale_gradient = match global_scale_prior {
            GlobalScalePrior::HalfNormal { scale } => {
                log_prior +=
                    0.5 * (2.0 / std::f64::consts::PI).ln() - scale.ln() - 0.5 * global_ratio_sq
                        + log_global_scale;
                sum_scaled_square - coefficient_count as f64 + 1.0 - global_ratio_sq
            }
            GlobalScalePrior::HalfCauchy { scale } => {
                log_prior +=
                    (2.0 / std::f64::consts::PI).ln() - scale.ln() - (1.0 + global_ratio_sq).ln()
                        + log_global_scale;
                sum_scaled_square - coefficient_count as f64 + 1.0
                    - 2.0 * global_ratio_sq / (1.0 + global_ratio_sq)
            }
        };
        if !log_likelihood.is_finite()
            || !log_prior.is_finite()
            || !log_global_scale_gradient.is_finite()
            || coefficient_gradient.iter().any(|value| !value.is_finite())
            || log_local_scale_gradient
                .iter()
                .any(|value| !value.is_finite())
        {
            return Err(ModelError::InvalidParameter(
                "prepared_horseshoe_posterior_nonfinite",
            ));
        }

        Ok(RawScaleHorseshoePosterior {
            log_likelihood,
            log_prior,
            log_posterior: log_likelihood + log_prior,
            coefficient_gradient,
            log_mean_gradient,
            log_local_scale_gradient,
            log_global_scale_gradient,
            biased_prediction,
        })
    }
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
                        "prepared_horseshoe_covariance_must_be_positive_definite",
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
            "prepared_horseshoe_solve_nonfinite",
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
            "prepared_horseshoe_logdet_nonfinite",
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
    fn prepared_model_matches_unprepared_posterior_target() {
        let y = [11.0, 19.0];
        let mu = [10.0, 20.0];
        let x = [vec![0.0], vec![1.0]];
        let beta = [0.1];
        let local = [0.0];
        let covariance = [vec![0.04, 0.01], vec![0.01, 0.09]];
        let prior = GlobalScalePrior::HalfNormal { scale: 0.5 };
        let prepared =
            PreparedRawScaleHorseshoeModel::new(&y, &mu, &x, &covariance).expect("prepared");
        let actual = prepared
            .evaluate(&beta, &local, -1.0, prior)
            .expect("evaluate");
        let expected = crate::raw_scale_horseshoe_log_posterior(
            &y,
            &mu,
            &x,
            &beta,
            &local,
            -1.0,
            &covariance,
            prior,
        )
        .expect("unprepared");
        assert!((actual.log_posterior - expected.log_posterior).abs() < 1.0e-12);
        assert_eq!(actual.coefficient_gradient, expected.coefficient_gradient);
        assert_eq!(
            actual.log_local_scale_gradient,
            expected.log_local_scale_gradient
        );
        assert_eq!(
            actual.log_global_scale_gradient,
            expected.log_global_scale_gradient
        );
    }

    #[test]
    fn prepared_model_rejects_invalid_covariance_at_construction() {
        assert_eq!(
            PreparedRawScaleHorseshoeModel::new(
                &[1.0, 2.0],
                &[1.0, 2.0],
                &[vec![1.0], vec![1.0]],
                &[vec![1.0, 2.0], vec![2.0, 1.0]],
            ),
            Err(ModelError::InvalidCovariance(
                "prepared_horseshoe_covariance_must_be_positive_definite",
            ))
        );
    }

    #[test]
    fn prepared_evaluation_rejects_coefficient_dimension_change() {
        let model = PreparedRawScaleHorseshoeModel::new(
            &[1.0, 2.0],
            &[1.0, 2.0],
            &[vec![1.0], vec![1.0]],
            &[vec![0.1, 0.0], vec![0.0, 0.1]],
        )
        .expect("model");
        assert_eq!(
            model.evaluate(
                &[0.1, 0.2],
                &[0.0, 0.0],
                -1.0,
                GlobalScalePrior::HalfNormal { scale: 1.0 },
            ),
            Err(ModelError::MismatchedDataLengths)
        );
    }
}
