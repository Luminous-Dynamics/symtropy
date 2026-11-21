//! First-order uncertainty propagation for spectra normalized to a shared reference.
//!
//! AIACHNE preprocessing rescales each experiment using a common pointwise
//! evaluation. Treating the resulting experiment blocks as independent omits
//! covariance induced by uncertainty in that shared reference. This module
//! exposes the normalization Jacobians and the resulting cross-experiment
//! covariance term.
//!
//! For observations y, integration weights w, reference values r, and a
//! projection A from the reference grid to the observation grid:
//!
//!   p = w^T y
//!   q = w^T A r
//!   z = (q / p) y
//!
//! The first-order Jacobians are:
//!
//!   J_y = (q/p) [I - y w^T / p]
//!   J_r = y (w^T A) / p
//!
//! The independent-observation contribution is J_y C_y J_y^T. The shared
//! reference contribution is J_r C_r J_r^T. For experiments j and k that use
//! the same reference, their cross covariance is J_r,j C_r J_r,k^T.
//!
//! This is a delta-method approximation, not exact nonlinear ratio
//! propagation. It deliberately accepts the full reference covariance; callers
//! must not silently invent off-diagonal reference correlations when only
//! marginal uncertainties are available.

use crate::ModelError;

/// Build trapezoidal integration weights for a strictly increasing energy grid.
pub fn trapezoidal_integration_weights(energies: &[f64]) -> Result<Vec<f64>, ModelError> {
    if energies.len() < 2 {
        return Err(ModelError::MismatchedDataLengths);
    }
    if energies.iter().any(|value| !value.is_finite()) {
        return Err(ModelError::NonFiniteValue("trapezoid_energy_grid"));
    }
    if energies.windows(2).any(|pair| pair[1] <= pair[0]) {
        return Err(ModelError::InvalidParameter(
            "trapezoid_energy_grid_must_be_strictly_increasing",
        ));
    }

    let mut weights = vec![0.0; energies.len()];
    weights[0] = 0.5 * (energies[1] - energies[0]);
    for (index, pair) in energies.windows(3).enumerate() {
        weights[index + 1] = 0.5 * (pair[2] - pair[0]);
    }
    let last = energies.len() - 1;
    weights[last] = 0.5 * (energies[last] - energies[last - 1]);
    if weights
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        return Err(ModelError::InvalidParameter("trapezoid_weights_invalid"));
    }
    Ok(weights)
}

/// Construct the piecewise-linear reference-to-observation projection used by
/// one-dimensional linear interpolation. Extrapolation is rejected explicitly.
pub fn linear_reference_projection(
    reference_energies: &[f64],
    observation_energies: &[f64],
) -> Result<Vec<Vec<f64>>, ModelError> {
    if reference_energies.len() < 2 || observation_energies.is_empty() {
        return Err(ModelError::MismatchedDataLengths);
    }
    for grid in [reference_energies, observation_energies] {
        if grid.iter().any(|value| !value.is_finite()) {
            return Err(ModelError::NonFiniteValue("linear_projection_energy_grid"));
        }
        if grid.windows(2).any(|pair| pair[1] <= pair[0]) {
            return Err(ModelError::InvalidParameter(
                "linear_projection_energy_grid_must_be_strictly_increasing",
            ));
        }
    }

    let mut projection = vec![vec![0.0; reference_energies.len()]; observation_energies.len()];
    for (row_index, energy) in observation_energies.iter().enumerate() {
        if *energy < reference_energies[0]
            || *energy > reference_energies[reference_energies.len() - 1]
        {
            return Err(ModelError::InvalidParameter(
                "linear_reference_projection_would_extrapolate",
            ));
        }
        match reference_energies.binary_search_by(|probe| probe.total_cmp(energy)) {
            Ok(exact_index) => projection[row_index][exact_index] = 1.0,
            Err(upper_index) => {
                let lower_index = upper_index - 1;
                let fraction = (energy - reference_energies[lower_index])
                    / (reference_energies[upper_index] - reference_energies[lower_index]);
                projection[row_index][lower_index] = 1.0 - fraction;
                projection[row_index][upper_index] = fraction;
            }
        }
    }
    Ok(projection)
}

/// Piecewise-linear projection and trapezoidal weights over the common
/// reference/observation energy domain.
///
/// This matches the control-flow of the public AIACHNE `get_scale_factor`
/// helper for strictly increasing grids: an interval whose left observation
/// energy is below the reference domain is skipped; an interval whose left
/// endpoint is in-domain but whose right endpoint is outside is rejected
/// because the upstream interpolation would yield NaN for that trapezoid.
#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceOverlapGeometry {
    pub reference_projection: Vec<Vec<f64>>,
    pub integration_weights: Vec<f64>,
    pub included_trapezoid_intervals: usize,
}

pub fn aiachne_compatible_overlap_geometry(
    reference_energies: &[f64],
    observation_energies: &[f64],
) -> Result<ReferenceOverlapGeometry, ModelError> {
    if reference_energies.len() < 2 || observation_energies.len() < 2 {
        return Err(ModelError::MismatchedDataLengths);
    }
    for grid in [reference_energies, observation_energies] {
        if grid.iter().any(|value| !value.is_finite()) {
            return Err(ModelError::NonFiniteValue("overlap_energy_grid"));
        }
        if grid.windows(2).any(|pair| pair[1] <= pair[0]) {
            return Err(ModelError::InvalidParameter(
                "overlap_energy_grid_must_be_strictly_increasing",
            ));
        }
    }

    let reference_min = reference_energies[0];
    let reference_max = reference_energies[reference_energies.len() - 1];
    let mut projection = vec![vec![0.0; reference_energies.len()]; observation_energies.len()];
    for (row_index, energy) in observation_energies.iter().enumerate() {
        if *energy < reference_min || *energy > reference_max {
            continue;
        }
        match reference_energies.binary_search_by(|probe| probe.total_cmp(energy)) {
            Ok(exact_index) => projection[row_index][exact_index] = 1.0,
            Err(upper_index) => {
                let lower_index = upper_index - 1;
                let fraction = (energy - reference_energies[lower_index])
                    / (reference_energies[upper_index] - reference_energies[lower_index]);
                projection[row_index][lower_index] = 1.0 - fraction;
                projection[row_index][upper_index] = fraction;
            }
        }
    }

    let mut integration_weights = vec![0.0; observation_energies.len()];
    let mut included_trapezoid_intervals = 0usize;
    for (index, pair) in observation_energies.windows(2).enumerate() {
        let left_in_reference_domain = pair[0] >= reference_min && pair[0] <= reference_max;
        if !left_in_reference_domain {
            continue;
        }
        let right_in_reference_domain = pair[1] >= reference_min && pair[1] <= reference_max;
        if !right_in_reference_domain {
            return Err(ModelError::InvalidParameter(
                "aiache_reference_interpolation_nan_at_right_trapezoid_endpoint",
            ));
        }
        let half_width = 0.5 * (pair[1] - pair[0]);
        integration_weights[index] += half_width;
        integration_weights[index + 1] += half_width;
        included_trapezoid_intervals += 1;
    }
    if included_trapezoid_intervals == 0 {
        return Err(ModelError::InvalidParameter(
            "aiache_normalization_has_no_reference_overlap_intervals",
        ));
    }
    Ok(ReferenceOverlapGeometry {
        reference_projection: projection,
        integration_weights,
        included_trapezoid_intervals,
    })
}

/// Inputs for one experiment normalized against a shared reference.
pub struct SharedReferenceNormalizationInput<'a> {
    /// Raw, unnormalized observations on this experiment's energy grid.
    pub observations: &'a [f64],
    /// Full covariance of the raw observations.
    pub observation_covariance: &'a [Vec<f64>],
    /// Reference spectrum on its native energy grid.
    pub reference_values: &'a [f64],
    /// Full covariance of the reference spectrum on its native energy grid.
    pub reference_covariance: &'a [Vec<f64>],
    /// Projection from native reference bins to this experiment's bins.
    /// Shape: observations.len() x reference_values.len().
    pub reference_projection: &'a [Vec<f64>],
    /// Integration weights on the experiment grid (for example trapezoid weights).
    pub integration_weights: &'a [f64],
}

/// Normalized spectrum and its decomposed first-order covariance.
#[derive(Debug, Clone, PartialEq)]
pub struct SharedReferenceNormalizationResult {
    pub scale_factor: f64,
    pub reference_integral_on_experiment_grid: f64,
    pub observation_integral: f64,
    pub normalized_values: Vec<f64>,
    /// Derivative of normalized values with respect to raw observations.
    pub observation_jacobian: Vec<Vec<f64>>,
    /// Derivative of normalized values with respect to the native shared reference.
    pub reference_jacobian: Vec<Vec<f64>>,
    /// J_y C_y J_y^T.
    pub observation_covariance_contribution: Vec<Vec<f64>>,
    /// J_r C_r J_r^T.
    pub shared_reference_covariance_contribution: Vec<Vec<f64>>,
    /// Sum of the two covariance contributions for this experiment.
    pub total_covariance: Vec<Vec<f64>>,
}

/// Normalize one experiment and propagate observation plus shared-reference
/// covariance using a first-order delta method.
pub fn normalize_spectrum_with_shared_reference(
    input: SharedReferenceNormalizationInput<'_>,
) -> Result<SharedReferenceNormalizationResult, ModelError> {
    let observation_count = input.observations.len();
    let reference_count = input.reference_values.len();
    if observation_count == 0
        || reference_count == 0
        || input.observation_covariance.len() != observation_count
        || input.reference_covariance.len() != reference_count
        || input.reference_projection.len() != observation_count
        || input.integration_weights.len() != observation_count
        || input
            .observation_covariance
            .iter()
            .any(|row| row.len() != observation_count)
        || input
            .reference_covariance
            .iter()
            .any(|row| row.len() != reference_count)
        || input
            .reference_projection
            .iter()
            .any(|row| row.len() != reference_count)
    {
        return Err(ModelError::MismatchedDataLengths);
    }

    validate_symmetric_finite_covariance(
        input.observation_covariance,
        "normalization_observation_covariance",
    )?;
    validate_symmetric_finite_covariance(
        input.reference_covariance,
        "normalization_reference_covariance",
    )?;
    for value in input.observations {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue("normalization_observations"));
        }
        if *value < 0.0 {
            return Err(ModelError::InvalidParameter(
                "normalization_observations_must_be_nonnegative",
            ));
        }
    }
    for value in input.reference_values {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue("normalization_reference_values"));
        }
        if *value < 0.0 {
            return Err(ModelError::InvalidParameter(
                "normalization_reference_values_must_be_nonnegative",
            ));
        }
    }
    if input
        .integration_weights
        .iter()
        .any(|value| !value.is_finite())
        || input
            .reference_projection
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
    {
        return Err(ModelError::NonFiniteValue(
            "normalization_weights_or_projection",
        ));
    }

    let projected_reference =
        matrix_vector_multiply(input.reference_projection, input.reference_values)?;
    let observation_integral = dot(input.observations, input.integration_weights);
    let reference_integral_on_experiment_grid =
        dot(&projected_reference, input.integration_weights);
    if !observation_integral.is_finite() || observation_integral <= 0.0 {
        return Err(ModelError::NonPositiveValue(
            "normalization_observation_integral",
        ));
    }
    if !reference_integral_on_experiment_grid.is_finite()
        || reference_integral_on_experiment_grid <= 0.0
    {
        return Err(ModelError::NonPositiveValue(
            "normalization_reference_integral",
        ));
    }

    let scale_factor = reference_integral_on_experiment_grid / observation_integral;
    if !scale_factor.is_finite() || scale_factor <= 0.0 {
        return Err(ModelError::InvalidParameter(
            "normalization_scale_factor_invalid",
        ));
    }
    let normalized_values: Vec<f64> = input
        .observations
        .iter()
        .map(|value| scale_factor * value)
        .collect();
    if normalized_values.iter().any(|value| !value.is_finite()) {
        return Err(ModelError::InvalidParameter(
            "normalization_scaled_values_nonfinite",
        ));
    }

    // u = A^T w, so q = u^T r and J_r = y u^T / p.
    let projected_weights =
        transpose_matrix_vector_multiply(input.reference_projection, input.integration_weights)?;
    let mut observation_jacobian = vec![vec![0.0; observation_count]; observation_count];
    for (row_index, row) in observation_jacobian.iter_mut().enumerate() {
        for (column_index, value) in row.iter_mut().enumerate() {
            let identity = if row_index == column_index { 1.0 } else { 0.0 };
            *value = scale_factor
                * (identity
                    - input.observations[row_index] * input.integration_weights[column_index]
                        / observation_integral);
        }
    }

    let mut reference_jacobian = vec![vec![0.0; reference_count]; observation_count];
    for (row_index, row) in reference_jacobian.iter_mut().enumerate() {
        for (column_index, value) in row.iter_mut().enumerate() {
            *value = input.observations[row_index] * projected_weights[column_index]
                / observation_integral;
        }
    }

    let observation_covariance_contribution =
        propagate_covariance(&observation_jacobian, input.observation_covariance)?;
    let shared_reference_covariance_contribution =
        propagate_covariance(&reference_jacobian, input.reference_covariance)?;
    let total_covariance = add_matrices(
        &observation_covariance_contribution,
        &shared_reference_covariance_contribution,
    )?;

    Ok(SharedReferenceNormalizationResult {
        scale_factor,
        reference_integral_on_experiment_grid,
        observation_integral,
        normalized_values,
        observation_jacobian,
        reference_jacobian,
        observation_covariance_contribution,
        shared_reference_covariance_contribution,
        total_covariance,
    })
}

/// Cross covariance between two normalized experiments that use the same
/// uncertain reference. The Jacobians may have different row counts but must
/// share the same native-reference column count.
pub fn shared_reference_cross_covariance(
    left_reference_jacobian: &[Vec<f64>],
    reference_covariance: &[Vec<f64>],
    right_reference_jacobian: &[Vec<f64>],
) -> Result<Vec<Vec<f64>>, ModelError> {
    let reference_count = reference_covariance.len();
    if left_reference_jacobian.is_empty()
        || right_reference_jacobian.is_empty()
        || reference_count == 0
        || reference_covariance
            .iter()
            .any(|row| row.len() != reference_count)
        || left_reference_jacobian
            .iter()
            .any(|row| row.len() != reference_count)
        || right_reference_jacobian
            .iter()
            .any(|row| row.len() != reference_count)
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    validate_symmetric_finite_covariance(
        reference_covariance,
        "shared_reference_cross_covariance",
    )?;
    validate_finite_matrix(left_reference_jacobian, "left_reference_jacobian")?;
    validate_finite_matrix(right_reference_jacobian, "right_reference_jacobian")?;

    let left_times_covariance = matrix_multiply(left_reference_jacobian, reference_covariance)?;
    matrix_multiply(&left_times_covariance, &transpose(right_reference_jacobian))
}

fn validate_symmetric_finite_covariance(
    matrix: &[Vec<f64>],
    name: &'static str,
) -> Result<(), ModelError> {
    let dimension = matrix.len();
    if dimension == 0 || matrix.iter().any(|row| row.len() != dimension) {
        return Err(ModelError::MismatchedDataLengths);
    }
    validate_finite_matrix(matrix, name)?;
    let scale = matrix
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    let tolerance = 1.0e-10 * scale.max(f64::MIN_POSITIVE);
    for (row_index, row) in matrix.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate().take(row_index) {
            if (*value - matrix[column_index][row_index]).abs() > tolerance {
                return Err(ModelError::InvalidCovariance(name));
            }
        }
    }
    validate_positive_semidefinite(matrix, name)?;
    Ok(())
}

fn validate_positive_semidefinite(
    matrix: &[Vec<f64>],
    name: &'static str,
) -> Result<(), ModelError> {
    let dimension = matrix.len();
    let scale = matrix
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    let tolerance = 1.0e-12 * scale.max(f64::MIN_POSITIVE);
    let mut lower = vec![vec![0.0; dimension]; dimension];
    for (row_index, row) in matrix.iter().enumerate() {
        for (column_index, entry) in row.iter().enumerate().take(row_index + 1) {
            let correction: f64 = lower[row_index][..column_index]
                .iter()
                .zip(lower[column_index][..column_index].iter())
                .map(|(left, right)| left * right)
                .sum();
            let value = *entry - correction;
            if row_index == column_index {
                if value < -tolerance {
                    return Err(ModelError::InvalidCovariance(name));
                }
                lower[row_index][column_index] = if value > 0.0 { value.sqrt() } else { 0.0 };
            } else {
                let pivot = lower[column_index][column_index];
                if pivot > 0.0 {
                    lower[row_index][column_index] = value / pivot;
                } else if value.abs() > tolerance {
                    return Err(ModelError::InvalidCovariance(name));
                }
            }
        }
    }
    Ok(())
}

fn validate_finite_matrix(matrix: &[Vec<f64>], name: &'static str) -> Result<(), ModelError> {
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(ModelError::NonFiniteValue(name));
    }
    Ok(())
}

fn matrix_vector_multiply(matrix: &[Vec<f64>], vector: &[f64]) -> Result<Vec<f64>, ModelError> {
    if matrix.is_empty() || matrix.iter().any(|row| row.len() != vector.len()) {
        return Err(ModelError::MismatchedDataLengths);
    }
    Ok(matrix
        .iter()
        .map(|row| row.iter().zip(vector.iter()).map(|(a, b)| *a * *b).sum())
        .collect())
}

fn transpose_matrix_vector_multiply(
    matrix: &[Vec<f64>],
    vector: &[f64],
) -> Result<Vec<f64>, ModelError> {
    if matrix.is_empty() || matrix.len() != vector.len() {
        return Err(ModelError::MismatchedDataLengths);
    }
    let columns = matrix[0].len();
    if columns == 0 || matrix.iter().any(|row| row.len() != columns) {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut result = vec![0.0; columns];
    for (row, weight) in matrix.iter().zip(vector.iter()) {
        for (column, value) in result.iter_mut().enumerate() {
            *value += row[column] * *weight;
        }
    }
    Ok(result)
}

fn transpose(matrix: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let rows = matrix.len();
    let columns = matrix.first().map_or(0, Vec::len);
    let mut result = vec![vec![0.0; rows]; columns];
    for (row_index, row) in matrix.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate() {
            result[column_index][row_index] = *value;
        }
    }
    result
}

fn matrix_multiply(left: &[Vec<f64>], right: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, ModelError> {
    if left.is_empty() || right.is_empty() {
        return Err(ModelError::MismatchedDataLengths);
    }
    let inner_dimension = left[0].len();
    let right_columns = right[0].len();
    if inner_dimension == 0
        || right_columns == 0
        || left.iter().any(|row| row.len() != inner_dimension)
        || right.len() != inner_dimension
        || right.iter().any(|row| row.len() != right_columns)
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut result = vec![vec![0.0; right_columns]; left.len()];
    for (row_index, output_row) in result.iter_mut().enumerate() {
        for (column_index, output) in output_row.iter_mut().enumerate() {
            *output = left[row_index]
                .iter()
                .zip(right.iter().map(|row| row[column_index]))
                .map(|(a, b)| *a * b)
                .sum();
        }
    }
    if result.iter().flatten().any(|value| !value.is_finite()) {
        return Err(ModelError::InvalidCovariance(
            "shared_reference_covariance_propagation_nonfinite",
        ));
    }
    Ok(result)
}

fn propagate_covariance(
    jacobian: &[Vec<f64>],
    covariance: &[Vec<f64>],
) -> Result<Vec<Vec<f64>>, ModelError> {
    let left = matrix_multiply(jacobian, covariance)?;
    matrix_multiply(&left, &transpose(jacobian))
}

fn add_matrices(left: &[Vec<f64>], right: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, ModelError> {
    if left.is_empty()
        || right.is_empty()
        || left.len() != right.len()
        || left.iter().any(|row| row.len() != right[0].len())
        || right.iter().any(|row| row.len() != left[0].len())
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    Ok(left
        .iter()
        .zip(right.iter())
        .map(|(left_row, right_row)| {
            left_row
                .iter()
                .zip(right_row.iter())
                .map(|(a, b)| a + b)
                .collect()
        })
        .collect())
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right.iter()).map(|(a, b)| *a * *b).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trapezoid_weights_match_nonuniform_grid_integral() {
        let weights = trapezoidal_integration_weights(&[1.0, 2.0, 4.0]).expect("weights");
        assert_eq!(weights, vec![0.5, 1.5, 1.0]);
        let integral = dot(&[2.0, 4.0, 8.0], &weights);
        assert_eq!(integral, 15.0);
    }

    #[test]
    fn linear_reference_projection_matches_piecewise_linear_interpolation() {
        let projection =
            linear_reference_projection(&[1.0, 2.0, 4.0], &[1.5, 3.0, 4.0]).expect("projection");
        assert_eq!(
            projection,
            vec![
                vec![0.5, 0.5, 0.0],
                vec![0.0, 0.5, 0.5],
                vec![0.0, 0.0, 1.0],
            ]
        );
    }

    #[test]
    fn linear_reference_projection_rejects_extrapolation_and_bad_grids() {
        assert_eq!(
            linear_reference_projection(&[1.0, 2.0], &[0.5]),
            Err(ModelError::InvalidParameter(
                "linear_reference_projection_would_extrapolate",
            ))
        );
        assert_eq!(
            trapezoidal_integration_weights(&[1.0, 1.0]),
            Err(ModelError::InvalidParameter(
                "trapezoid_energy_grid_must_be_strictly_increasing",
            ))
        );
    }

    #[test]
    fn aiachne_overlap_geometry_skips_only_lower_out_of_domain_left_intervals() {
        let geometry =
            aiachne_compatible_overlap_geometry(&[0.5, 1.0, 2.0, 4.0], &[0.25, 1.0, 2.0, 3.0])
                .expect("overlap geometry");
        assert_eq!(geometry.included_trapezoid_intervals, 2);
        assert_eq!(geometry.integration_weights, vec![0.0, 0.5, 1.0, 0.5]);
        assert_eq!(geometry.reference_projection[0], vec![0.0; 4]);
        assert_eq!(geometry.reference_projection[3], vec![0.0, 0.0, 0.5, 0.5]);
    }

    #[test]
    fn aiachne_overlap_geometry_fails_closed_on_upper_nan_trapezoid_endpoint() {
        assert_eq!(
            aiachne_compatible_overlap_geometry(&[0.5, 1.0, 2.0], &[1.0, 2.0, 3.0],),
            Err(ModelError::InvalidParameter(
                "aiache_reference_interpolation_nan_at_right_trapezoid_endpoint",
            ))
        );
    }

    #[test]
    fn covariance_validation_rejects_indefinite_reference_or_observation_covariance() {
        assert_eq!(
            validate_symmetric_finite_covariance(
                &[vec![1.0, 2.0], vec![2.0, 1.0]],
                "test_covariance"
            ),
            Err(ModelError::InvalidCovariance("test_covariance"))
        );
    }

    #[test]
    fn normalization_jacobians_match_finite_differences() {
        let observed = [2.0, 4.0];
        let reference = [3.0, 5.0];
        let weights = [0.5, 0.5];
        let projection = [vec![1.0, 0.0], vec![0.0, 1.0]];
        let identity_covariance = [vec![1.0, 0.0], vec![0.0, 1.0]];
        let baseline =
            normalize_spectrum_with_shared_reference(SharedReferenceNormalizationInput {
                observations: &observed,
                observation_covariance: &identity_covariance,
                reference_values: &reference,
                reference_covariance: &identity_covariance,
                reference_projection: &projection,
                integration_weights: &weights,
            })
            .expect("baseline normalization");
        let epsilon = 1.0e-6;
        let perturbed_observations = [observed[0] + epsilon, observed[1]];
        let observation_perturbation =
            normalize_spectrum_with_shared_reference(SharedReferenceNormalizationInput {
                observations: &perturbed_observations,
                observation_covariance: &identity_covariance,
                reference_values: &reference,
                reference_covariance: &identity_covariance,
                reference_projection: &projection,
                integration_weights: &weights,
            })
            .expect("observation perturbation");
        let observed_derivative = (observation_perturbation.normalized_values[0]
            - baseline.normalized_values[0])
            / epsilon;
        assert!((observed_derivative - baseline.observation_jacobian[0][0]).abs() < 1.0e-6);

        let perturbed_reference = [reference[0] + epsilon, reference[1]];
        let reference_perturbation =
            normalize_spectrum_with_shared_reference(SharedReferenceNormalizationInput {
                observations: &observed,
                observation_covariance: &identity_covariance,
                reference_values: &perturbed_reference,
                reference_covariance: &identity_covariance,
                reference_projection: &projection,
                integration_weights: &weights,
            })
            .expect("reference perturbation");
        let reference_derivative =
            (reference_perturbation.normalized_values[0] - baseline.normalized_values[0]) / epsilon;
        assert!((reference_derivative - baseline.reference_jacobian[0][0]).abs() < 1.0e-6);
    }

    #[test]
    fn normalization_integral_matches_shared_reference_integral() {
        let observed = [2.0, 4.0];
        let reference = [3.0, 5.0];
        let weights = [0.5, 0.5];
        let input = SharedReferenceNormalizationInput {
            observations: &observed,
            observation_covariance: &[vec![0.04, 0.01], vec![0.01, 0.09]],
            reference_values: &reference,
            reference_covariance: &[vec![0.01, 0.0], vec![0.0, 0.04]],
            reference_projection: &[vec![1.0, 0.0], vec![0.0, 1.0]],
            integration_weights: &weights,
        };
        let result = normalize_spectrum_with_shared_reference(input).expect("normalization");
        let normalized_integral = dot(&result.normalized_values, &weights);
        assert!(
            (normalized_integral - result.reference_integral_on_experiment_grid).abs() < 1.0e-12
        );
        assert!((result.scale_factor - 4.0 / 3.0).abs() < 1.0e-12);
    }

    #[test]
    fn constant_fixed_reference_integral_creates_a_null_covariance_direction() {
        let observed = [2.0, 4.0, 5.0];
        let reference = [3.0, 5.0, 7.0];
        let weights = [0.2, 0.3, 0.5];
        let zero_reference_covariance = vec![vec![0.0; 3]; 3];
        let covariance = vec![
            vec![0.1, 0.01, 0.0],
            vec![0.01, 0.2, 0.02],
            vec![0.0, 0.02, 0.15],
        ];
        let result = normalize_spectrum_with_shared_reference(SharedReferenceNormalizationInput {
            observations: &observed,
            observation_covariance: &covariance,
            reference_values: &reference,
            reference_covariance: &zero_reference_covariance,
            reference_projection: &[
                vec![1.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0],
                vec![0.0, 0.0, 1.0],
            ],
            integration_weights: &weights,
        })
        .expect("normalized covariance");
        let projected: Vec<f64> = result
            .total_covariance
            .iter()
            .map(|row| dot(&weights, row))
            .collect();
        assert!(projected.iter().all(|value| value.abs() < 1.0e-12));
        assert!(result.total_covariance[0][0] > 0.0);
    }

    #[test]
    fn independent_experiments_become_correlated_through_shared_reference() {
        let shared_covariance = vec![vec![0.04, 0.01], vec![0.01, 0.09]];
        let left_jacobian = vec![vec![0.5, 0.5], vec![0.25, 0.25]];
        let right_jacobian = vec![vec![0.2, 0.8], vec![0.1, 0.4], vec![0.3, 1.2]];
        let cross =
            shared_reference_cross_covariance(&left_jacobian, &shared_covariance, &right_jacobian)
                .expect("shared reference cross covariance");
        assert_eq!(cross.len(), 2);
        assert_eq!(cross[0].len(), 3);
        assert!(cross.iter().flatten().all(|value| value.is_finite()));
        assert!(cross.iter().flatten().any(|value| value.abs() > 0.0));
    }

    #[test]
    fn independent_normalized_experiments_share_reference_induced_cross_covariance() {
        let reference = [3.0, 5.0];
        let reference_covariance = [vec![0.04, 0.01], vec![0.01, 0.09]];
        let identity = [vec![0.1, 0.0], vec![0.0, 0.1]];
        let projection = [vec![1.0, 0.0], vec![0.0, 1.0]];
        let weights = [0.5, 0.5];
        let left_observations = [2.0, 4.0];
        let right_observations = [1.0, 6.0];
        let left = normalize_spectrum_with_shared_reference(SharedReferenceNormalizationInput {
            observations: &left_observations,
            observation_covariance: &identity,
            reference_values: &reference,
            reference_covariance: &reference_covariance,
            reference_projection: &projection,
            integration_weights: &weights,
        })
        .expect("left normalization");
        let right = normalize_spectrum_with_shared_reference(SharedReferenceNormalizationInput {
            observations: &right_observations,
            observation_covariance: &identity,
            reference_values: &reference,
            reference_covariance: &reference_covariance,
            reference_projection: &projection,
            integration_weights: &weights,
        })
        .expect("right normalization");

        let cross = shared_reference_cross_covariance(
            &left.reference_jacobian,
            &reference_covariance,
            &right.reference_jacobian,
        )
        .expect("shared-reference cross covariance");
        assert_eq!(cross.len(), 2);
        assert_eq!(cross[0].len(), 2);
        assert!(cross.iter().flatten().any(|value| value.abs() > 1.0e-12));
    }

    #[test]
    fn rejects_bad_projection_dimensions_and_nonfinite_covariance() {
        let observed = [2.0, 4.0];
        let reference = [3.0, 5.0];
        let weights = [0.5, 0.5];
        let wrong_projection = vec![vec![1.0, 0.0]];
        assert_eq!(
            normalize_spectrum_with_shared_reference(SharedReferenceNormalizationInput {
                observations: &observed,
                observation_covariance: &[vec![1.0, 0.0], vec![0.0, 1.0]],
                reference_values: &reference,
                reference_covariance: &[vec![1.0, 0.0], vec![0.0, 1.0]],
                reference_projection: &wrong_projection,
                integration_weights: &weights,
            }),
            Err(ModelError::MismatchedDataLengths)
        );
        assert_eq!(
            normalize_spectrum_with_shared_reference(SharedReferenceNormalizationInput {
                observations: &observed,
                observation_covariance: &[vec![1.0, f64::NAN], vec![f64::NAN, 1.0]],
                reference_values: &reference,
                reference_covariance: &[vec![1.0, 0.0], vec![0.0, 1.0]],
                reference_projection: &[vec![1.0, 0.0], vec![0.0, 1.0]],
                integration_weights: &weights,
            }),
            Err(ModelError::NonFiniteValue(
                "normalization_observation_covariance"
            ))
        );
    }
}
