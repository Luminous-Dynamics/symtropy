//! Small-feature Bayesian variable selection for multiplicative Poisson bias.
//!
//! Observation model:
//!   y_i ~ Poisson(mu_physical_i * exp(x_i^T beta))
//!
//! Each coefficient has a Bernoulli inclusion indicator and a zero-centred
//! Gaussian slab prior. All 2^p feature subsets are enumerated (p <= 12);
//! within each subset, the posterior mode and Hessian are computed and the
//! marginal likelihood is approximated by a Laplace expansion.
//!
//! This is a transparent spike-and-slab comparator, not the AIACHNE horseshoe
//! implementation. It is not suitable for a large unfiltered feature dictionary.
//! Design columns should be scaled using training data only. Physical offsets and
//! raw observations are never changed by this routine.

use crate::ModelError;

const MAX_ENUMERATED_BIAS_FEATURES: usize = 12;
const MAX_NEWTON_ITERATIONS: usize = 100;
const NEWTON_TOLERANCE: f64 = 1.0e-9;

type SubsetEvaluation = (f64, Vec<f64>, Vec<Vec<f64>>, f64);

#[derive(Debug, Clone, PartialEq)]
pub struct SparseBayesianBiasResult {
    pub coefficient_posterior_mean: Vec<f64>,
    pub coefficient_posterior_covariance: Vec<Vec<f64>>,
    pub feature_inclusion_probability: Vec<f64>,
    /// Posterior probability for each subset; index is the feature bit mask.
    pub model_posterior_probability: Vec<f64>,
    /// Posterior mode coefficients for the highest-posterior-probability subset.
    pub map_coefficients: Vec<f64>,
    pub map_feature_mask: usize,
    pub map_model_probability: f64,
    /// Relative log evidence; the common Poisson log-factorial term is omitted.
    pub relative_log_model_evidence: f64,
    pub maximum_optimizer_iterations: usize,
    pub enumerated_model_count: usize,
}

#[derive(Debug, Clone)]
struct SubsetFit {
    coefficients: Vec<f64>,
    covariance: Vec<Vec<f64>>,
    log_joint_evidence: f64,
    iterations: usize,
}

/// Fit a Bayesian sparse log-multiplicative bias model to one flattened set of
/// observation channels. Matrix rows align with observed counts and physical
/// expected counts. Columns encode preregistered experiment features or feature
/// by energy-basis interactions.
///
/// Each feature is independently included with the supplied Bernoulli prior
/// probability. Included coefficients have a Normal(0, slab standard deviation)
/// prior. All subsets are enumerated and the posterior within each subset is
/// approximated by a local Laplace expansion.
///
/// The posterior is conditional on the supplied physical means, design matrix,
/// feature scaling, inclusion prior and slab scale. It does not propagate physical
/// model uncertainty, reference-spectrum covariance, or feature-dictionary
/// uncertainty. Highly correlated features may have diffuse / exchangeable
/// inclusion probabilities; that is evidence of weak attribution, not a reason
/// to select one feature arbitrarily.
pub fn fit_sparse_bayesian_poisson_bias(
    observed_counts: &[f64],
    physical_expected_counts: &[f64],
    design_matrix: &[Vec<f64>],
    feature_inclusion_probability: f64,
    slab_standard_deviation: f64,
) -> Result<SparseBayesianBiasResult, ModelError> {
    if observed_counts.is_empty()
        || observed_counts.len() != physical_expected_counts.len()
        || design_matrix.len() != observed_counts.len()
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
    if !slab_standard_deviation.is_finite() {
        return Err(ModelError::NonFiniteValue("slab_standard_deviation"));
    }
    if slab_standard_deviation <= 0.0 {
        return Err(ModelError::NonPositiveValue("slab_standard_deviation"));
    }

    let feature_count = design_matrix.first().map_or(0, Vec::len);
    if feature_count == 0 || design_matrix.iter().any(|row| row.len() != feature_count) {
        return Err(ModelError::MismatchedDataLengths);
    }
    if feature_count > MAX_ENUMERATED_BIAS_FEATURES {
        return Err(ModelError::InvalidParameter(
            "spike_slab_feature_count_exceeds_exact_enumeration_limit",
        ));
    }

    for (values, name) in [
        (observed_counts, "observed_counts"),
        (physical_expected_counts, "physical_expected_counts"),
    ] {
        if values.iter().any(|value| !value.is_finite()) {
            return Err(ModelError::NonFiniteValue(name));
        }
    }
    if observed_counts.iter().any(|value| *value < 0.0) {
        return Err(ModelError::InvalidParameter(
            "observed_counts_must_be_nonnegative",
        ));
    }
    if physical_expected_counts.iter().any(|value| *value <= 0.0) {
        return Err(ModelError::NonPositiveValue("physical_expected_counts"));
    }
    if design_matrix
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return Err(ModelError::NonFiniteValue("bias_design_matrix"));
    }

    let model_count = 1usize << feature_count;
    let mut fits = Vec::with_capacity(model_count);
    let mut maximum_optimizer_iterations = 0usize;
    let log_included = feature_inclusion_probability.ln();
    let log_excluded = (1.0 - feature_inclusion_probability).ln();

    for mask in 0..model_count {
        let active = active_features(mask, feature_count);
        let mut fit = fit_subset(
            observed_counts,
            physical_expected_counts,
            design_matrix,
            &active,
            slab_standard_deviation,
        )?;
        let included = active.len();
        let excluded = feature_count - included;
        fit.log_joint_evidence += included as f64 * log_included + excluded as f64 * log_excluded;
        maximum_optimizer_iterations = maximum_optimizer_iterations.max(fit.iterations);
        fits.push(fit);
    }

    let max_log_weight = fits
        .iter()
        .map(|fit| fit.log_joint_evidence)
        .fold(f64::NEG_INFINITY, f64::max);
    if !max_log_weight.is_finite() {
        return Err(ModelError::InvalidParameter(
            "sparse_bias_model_evidence_is_nonfinite",
        ));
    }
    let weight_sum = fits
        .iter()
        .map(|fit| (fit.log_joint_evidence - max_log_weight).exp())
        .sum::<f64>();
    if !weight_sum.is_finite() || weight_sum <= 0.0 {
        return Err(ModelError::InvalidParameter(
            "sparse_bias_model_weight_sum_invalid",
        ));
    }
    let relative_log_model_evidence = max_log_weight + weight_sum.ln();

    let probabilities: Vec<f64> = fits
        .iter()
        .map(|fit| (fit.log_joint_evidence - max_log_weight).exp() / weight_sum)
        .collect();

    let mut feature_inclusion = vec![0.0; feature_count];
    let mut coefficient_mean = vec![0.0; feature_count];
    let mut map_coefficients = vec![0.0; feature_count];
    let mut map_feature_mask = 0usize;
    let mut map_model_probability = -1.0_f64;

    for (mask, (fit, probability)) in fits.iter().zip(probabilities.iter()).enumerate() {
        let active = active_features(mask, feature_count);
        let mut full_coefficients = vec![0.0; feature_count];
        for (local, feature) in active.iter().enumerate() {
            full_coefficients[*feature] = fit.coefficients[local];
            feature_inclusion[*feature] += probability;
            coefficient_mean[*feature] += probability * fit.coefficients[local];
        }
        if *probability > map_model_probability {
            map_model_probability = *probability;
            map_feature_mask = mask;
            map_coefficients = full_coefficients;
        }
    }

    // Mixture covariance = within-subset Laplace covariance + between-subset spread.
    let mut coefficient_covariance = vec![vec![0.0; feature_count]; feature_count];
    for (mask, (fit, probability)) in fits.iter().zip(probabilities.iter()).enumerate() {
        let active = active_features(mask, feature_count);
        let mut full_coefficients = vec![0.0; feature_count];
        let mut full_covariance = vec![vec![0.0; feature_count]; feature_count];
        for (local_left, feature_left) in active.iter().enumerate() {
            full_coefficients[*feature_left] = fit.coefficients[local_left];
            for (local_right, feature_right) in active.iter().enumerate() {
                full_covariance[*feature_left][*feature_right] =
                    fit.covariance[local_left][local_right];
            }
        }
        for left in 0..feature_count {
            for right in 0..feature_count {
                let left_delta = full_coefficients[left] - coefficient_mean[left];
                let right_delta = full_coefficients[right] - coefficient_mean[right];
                coefficient_covariance[left][right] +=
                    probability * (full_covariance[left][right] + left_delta * right_delta);
            }
        }
    }

    Ok(SparseBayesianBiasResult {
        coefficient_posterior_mean: coefficient_mean,
        coefficient_posterior_covariance: coefficient_covariance,
        feature_inclusion_probability: feature_inclusion,
        model_posterior_probability: probabilities,
        map_coefficients,
        map_feature_mask,
        map_model_probability,
        relative_log_model_evidence,
        maximum_optimizer_iterations,
        enumerated_model_count: model_count,
    })
}

fn active_features(mask: usize, feature_count: usize) -> Vec<usize> {
    (0..feature_count)
        .filter(|feature| (mask & (1usize << feature)) != 0)
        .collect()
}

fn fit_subset(
    observed: &[f64],
    physical: &[f64],
    design: &[Vec<f64>],
    active: &[usize],
    slab_sd: f64,
) -> Result<SubsetFit, ModelError> {
    let dimension = active.len();
    if dimension == 0 {
        let log_likelihood = observed
            .iter()
            .zip(physical.iter())
            .map(|(count, mean)| count * mean.ln() - mean)
            .sum::<f64>();
        if !log_likelihood.is_finite() {
            return Err(ModelError::InvalidParameter(
                "sparse_bias_null_log_likelihood_nonfinite",
            ));
        }
        return Ok(SubsetFit {
            coefficients: Vec::new(),
            covariance: Vec::new(),
            log_joint_evidence: log_likelihood,
            iterations: 0,
        });
    }

    let mut coefficients = vec![0.0; dimension];
    let prior_precision = 1.0 / (slab_sd * slab_sd);
    if !prior_precision.is_finite() || prior_precision <= 0.0 {
        return Err(ModelError::InvalidParameter("slab_prior_precision_invalid"));
    }
    let mut converged = false;
    let mut iterations = 0usize;

    for iteration in 0..MAX_NEWTON_ITERATIONS {
        iterations = iteration + 1;
        let (objective, gradient, hessian, _) = evaluate_subset(
            observed,
            physical,
            design,
            active,
            &coefficients,
            prior_precision,
        )?;
        let gradient_norm = gradient
            .iter()
            .map(|value| value.abs())
            .fold(0.0_f64, f64::max);
        if gradient_norm <= NEWTON_TOLERANCE {
            converged = true;
            break;
        }

        let lower = cholesky(&hessian)?;
        let step = solve_cholesky(&lower, &gradient)?;
        let directional_decrease = dot(&gradient, &step);
        if !directional_decrease.is_finite() || directional_decrease <= 0.0 {
            return Err(ModelError::InvalidCovariance(
                "sparse_bias_newton_direction_not_descent",
            ));
        }

        let mut accepted = false;
        let mut line_search_scale = 1.0;
        for _ in 0..60 {
            let candidate: Vec<f64> = coefficients
                .iter()
                .zip(step.iter())
                .map(|(value, delta)| value - line_search_scale * delta)
                .collect();
            let candidate_objective = match evaluate_subset(
                observed,
                physical,
                design,
                active,
                &candidate,
                prior_precision,
            ) {
                Ok((value, _, _, _)) => value,
                Err(_) => f64::INFINITY,
            };
            if candidate_objective.is_finite()
                && candidate_objective
                    <= objective - 1.0e-4 * line_search_scale * directional_decrease
            {
                coefficients = candidate;
                accepted = true;
                break;
            }
            line_search_scale *= 0.5;
        }

        if !accepted {
            let step_norm = step.iter().map(|value| value.abs()).fold(0.0_f64, f64::max);
            if step_norm <= NEWTON_TOLERANCE {
                converged = true;
                break;
            }
            return Err(ModelError::InvalidParameter(
                "sparse_bias_newton_line_search_failed",
            ));
        }
        let scaled_step_norm = step
            .iter()
            .map(|value| (line_search_scale * value).abs())
            .fold(0.0_f64, f64::max);
        let coefficient_norm = coefficients
            .iter()
            .map(|value| value.abs())
            .fold(0.0_f64, f64::max);
        if scaled_step_norm <= NEWTON_TOLERANCE * (1.0 + coefficient_norm) {
            converged = true;
            break;
        }
    }

    if !converged {
        return Err(ModelError::InvalidParameter(
            "sparse_bias_newton_did_not_converge",
        ));
    }

    let (_, _, hessian, log_likelihood) = evaluate_subset(
        observed,
        physical,
        design,
        active,
        &coefficients,
        prior_precision,
    )?;
    let lower = cholesky(&hessian)?;
    let log_determinant = 2.0
        * lower
            .iter()
            .enumerate()
            .map(|(index, row)| row[index].ln())
            .sum::<f64>();
    let covariance = inverse_from_cholesky(&lower)?;
    let standardized_norm_sq =
        coefficients.iter().map(|value| value * value).sum::<f64>() * prior_precision;
    let log_joint_evidence = log_likelihood
        - 0.5 * standardized_norm_sq
        - dimension as f64 * slab_sd.ln()
        - 0.5 * log_determinant;

    if !log_joint_evidence.is_finite() {
        return Err(ModelError::InvalidParameter(
            "sparse_bias_laplace_evidence_nonfinite",
        ));
    }
    Ok(SubsetFit {
        coefficients,
        covariance,
        log_joint_evidence,
        iterations,
    })
}

fn evaluate_subset(
    observed: &[f64],
    physical: &[f64],
    design: &[Vec<f64>],
    active: &[usize],
    coefficients: &[f64],
    prior_precision: f64,
) -> Result<SubsetEvaluation, ModelError> {
    let dimension = active.len();
    if coefficients.len() != dimension {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut objective =
        0.5 * prior_precision * coefficients.iter().map(|value| value * value).sum::<f64>();
    let mut log_likelihood = 0.0;
    let mut gradient = vec![0.0; dimension];
    let mut hessian = vec![vec![0.0; dimension]; dimension];

    for row in 0..observed.len() {
        let mut eta = physical[row].ln();
        for (local, feature) in active.iter().enumerate() {
            eta += design[row][*feature] * coefficients[local];
        }
        if !eta.is_finite() || eta > f64::MAX.ln() {
            return Err(ModelError::InvalidParameter(
                "sparse_bias_linear_predictor_out_of_range",
            ));
        }
        let mean = if eta < -745.0 { 0.0 } else { eta.exp() };
        if !mean.is_finite() {
            return Err(ModelError::InvalidParameter(
                "sparse_bias_expected_count_invalid",
            ));
        }
        let count = observed[row];
        objective += mean - count * eta;
        log_likelihood += count * eta - mean;
        let residual = mean - count;
        for left in 0..dimension {
            let left_value = design[row][active[left]];
            gradient[left] += left_value * residual;
            for right in 0..dimension {
                hessian[left][right] += mean * left_value * design[row][active[right]];
            }
        }
    }

    for left in 0..dimension {
        gradient[left] += prior_precision * coefficients[left];
        hessian[left][left] += prior_precision;
    }
    if !objective.is_finite()
        || !log_likelihood.is_finite()
        || gradient.iter().any(|value| !value.is_finite())
        || hessian.iter().flatten().any(|value| !value.is_finite())
    {
        return Err(ModelError::InvalidParameter(
            "sparse_bias_objective_nonfinite",
        ));
    }
    Ok((objective, gradient, hessian, log_likelihood))
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
                        "sparse_bias_hessian_not_positive_definite",
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

fn solve_cholesky(lower: &[Vec<f64>], right_hand_side: &[f64]) -> Result<Vec<f64>, ModelError> {
    let dimension = lower.len();
    if right_hand_side.len() != dimension {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut forward = vec![0.0; dimension];
    for row in 0..dimension {
        let mut value = right_hand_side[row];
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
            "sparse_bias_hessian_solve_nonfinite",
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

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right.iter()).map(|(a, b)| a * b).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planted_single_feature_bias_is_identified() {
        let result = fit_sparse_bayesian_poisson_bias(
            &[20.0, 40.0, 20.0, 40.0],
            &[20.0, 20.0, 20.0, 20.0],
            &[vec![0.0], vec![1.0], vec![0.0], vec![1.0]],
            0.1,
            1.0,
        )
        .expect("planted bias");
        assert!(result.feature_inclusion_probability[0] > 0.99);
        assert!(result.map_coefficients[0] > 0.5);
        assert!(result.map_coefficients[0] < 0.85);
        assert_eq!(result.map_feature_mask, 1);
        assert_eq!(result.enumerated_model_count, 2);
        assert!(result.coefficient_posterior_covariance[0][0] > 0.0);
    }

    #[test]
    fn no_bias_control_prefers_null_model() {
        let result = fit_sparse_bayesian_poisson_bias(
            &[20.0, 20.0, 20.0, 20.0],
            &[20.0, 20.0, 20.0, 20.0],
            &[vec![0.0], vec![1.0], vec![0.0], vec![1.0]],
            0.1,
            1.0,
        )
        .expect("null control");
        assert!(result.feature_inclusion_probability[0] < 0.05);
        assert_eq!(result.map_feature_mask, 0);
    }

    #[test]
    fn collinear_features_remain_ambiguous_in_posterior_inclusion() {
        let result = fit_sparse_bayesian_poisson_bias(
            &[20.0, 40.0, 20.0, 40.0],
            &[20.0, 20.0, 20.0, 20.0],
            &[
                vec![0.0, 0.0],
                vec![1.0, 1.0],
                vec![0.0, 0.0],
                vec![1.0, 1.0],
            ],
            0.1,
            1.0,
        )
        .expect("collinear controls");
        assert!(
            (result.feature_inclusion_probability[0] - result.feature_inclusion_probability[1])
                .abs()
                < 1.0e-10
        );
        assert!(result.coefficient_posterior_covariance[0][1] < 0.0);
    }

    #[test]
    fn zero_feature_prior_mass_is_rejected() {
        assert_eq!(
            fit_sparse_bayesian_poisson_bias(&[1.0], &[1.0], &[vec![1.0]], 0.0, 1.0),
            Err(ModelError::InvalidParameter(
                "feature_inclusion_probability_must_be_between_zero_and_one",
            ))
        );
    }

    #[test]
    fn rejects_nonfinite_or_nonpositive_model_inputs() {
        assert_eq!(
            fit_sparse_bayesian_poisson_bias(&[f64::NAN], &[1.0], &[vec![1.0]], 0.1, 1.0),
            Err(ModelError::NonFiniteValue("observed_counts"))
        );
        assert_eq!(
            fit_sparse_bayesian_poisson_bias(&[1.0], &[0.0], &[vec![1.0]], 0.1, 1.0),
            Err(ModelError::NonPositiveValue("physical_expected_counts"))
        );
        assert_eq!(
            fit_sparse_bayesian_poisson_bias(&[-1.0], &[1.0], &[vec![1.0]], 0.1, 1.0),
            Err(ModelError::InvalidParameter(
                "observed_counts_must_be_nonnegative"
            ))
        );
    }

    #[test]
    fn exact_subset_enumeration_is_bounded() {
        let features = vec![vec![0.0; 13]; 2];
        assert_eq!(
            fit_sparse_bayesian_poisson_bias(&[1.0, 1.0], &[1.0, 1.0], &features, 0.1, 1.0),
            Err(ModelError::InvalidParameter(
                "spike_slab_feature_count_exceeds_exact_enumeration_limit",
            ))
        );
    }

    #[test]
    fn posterior_model_probabilities_normalize() {
        let result = fit_sparse_bayesian_poisson_bias(
            &[15.0, 21.0, 18.0],
            &[20.0, 20.0, 20.0],
            &[vec![0.0, 1.0], vec![1.0, 0.0], vec![1.0, 1.0]],
            0.2,
            0.5,
        )
        .expect("two-feature model");
        assert!((result.model_posterior_probability.iter().sum::<f64>() - 1.0).abs() < 1.0e-12);
        assert_eq!(result.enumerated_model_count, 4);
        assert!(
            (result.coefficient_posterior_covariance[0][1]
                - result.coefficient_posterior_covariance[1][0])
                .abs()
                < 1.0e-12
        );
    }

    #[test]
    fn subset_approximation_reports_relative_log_evidence_and_iteration_count() {
        let result = fit_sparse_bayesian_poisson_bias(
            &[20.0, 40.0, 20.0, 40.0],
            &[20.0, 20.0, 20.0, 20.0],
            &[vec![0.0], vec![1.0], vec![0.0], vec![1.0]],
            0.1,
            1.0,
        )
        .expect("fit");
        assert!(result.relative_log_model_evidence.is_finite());
        assert!(result.maximum_optimizer_iterations > 0);
    }
}
