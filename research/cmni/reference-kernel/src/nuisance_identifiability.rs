//! Nuisance-profiled Fisher information for local identifiability.
//!
//! This computes the Schur complement of a joint Fisher matrix after treating
//! the trailing parameter block as nuisance parameters. It prevents CMNI from
//! reporting information about a physical parameter that is actually absorbed
//! by a detector, calibration, reference, or normalization nuisance parameter.
//!
//! The nuisance block must be well-conditioned and positive definite. Singular
//! nuisance parameterizations must first be reparameterized or constrained by
//! an independently justified anchor; this routine deliberately does not add a
//! ridge or silently use a pseudoinverse.

use crate::ModelError;

/// Profile trailing nuisance parameters out of a joint Fisher-information matrix.
///
/// The first `interest_parameter_count` coordinates are parameters of interest;
/// all remaining coordinates are nuisance parameters. The returned matrix is
/// the local efficient information for the parameters of interest:
///
/// `F_eff = F_ii - F_in F_nn^{-1} F_ni`.
///
/// The tolerance is relative to the largest absolute entry in the nuisance
/// block. Parameters must be scaled to documented dimensionless coordinates
/// before interpreting the tolerance or comparing designs.
pub fn profile_nuisance_fisher_information(
    joint_information: &[Vec<f64>],
    interest_parameter_count: usize,
    relative_pivot_tolerance: f64,
) -> Result<Vec<Vec<f64>>, ModelError> {
    let dimension = joint_information.len();
    if dimension < 2
        || joint_information.iter().any(|row| row.len() != dimension)
        || interest_parameter_count == 0
        || interest_parameter_count >= dimension
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    if !relative_pivot_tolerance.is_finite()
        || relative_pivot_tolerance <= 0.0
        || relative_pivot_tolerance >= 1.0
    {
        return Err(ModelError::InvalidParameter(
            "relative_pivot_tolerance_must_be_between_zero_and_one",
        ));
    }
    if joint_information
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return Err(ModelError::NonFiniteValue("joint_fisher_information"));
    }

    let matrix_scale = joint_information
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    let symmetry_tolerance = 1.0e-10 * matrix_scale.max(f64::MIN_POSITIVE);
    for (row_index, row) in joint_information.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate().take(row_index) {
            if (*value - joint_information[column_index][row_index]).abs() > symmetry_tolerance {
                return Err(ModelError::InvalidCovariance(
                    "joint_fisher_information_must_be_symmetric",
                ));
            }
        }
    }

    let nuisance_count = dimension - interest_parameter_count;
    let nuisance_block: Vec<Vec<f64>> = joint_information
        .iter()
        .skip(interest_parameter_count)
        .map(|row| row[interest_parameter_count..].to_vec())
        .collect();
    let nuisance_scale = nuisance_block
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    if nuisance_scale == 0.0 {
        return Err(ModelError::InvalidCovariance(
            "nuisance_fisher_block_is_singular",
        ));
    }
    let lower = cholesky_with_relative_pivot_tolerance(
        &nuisance_block,
        relative_pivot_tolerance,
        nuisance_scale,
    )?;

    // Solve F_nn X = F_ni one right-hand side at a time.
    let mut solved_cross_block = vec![vec![0.0; interest_parameter_count]; nuisance_count];
    for interest_column in 0..interest_parameter_count {
        let right_hand_side: Vec<f64> = joint_information
            .iter()
            .skip(interest_parameter_count)
            .map(|row| row[interest_column])
            .collect();
        let solution = solve_cholesky(&lower, &right_hand_side)?;
        for (nuisance_row, value) in solution.iter().enumerate() {
            solved_cross_block[nuisance_row][interest_column] = *value;
        }
    }

    let mut profiled = vec![vec![0.0; interest_parameter_count]; interest_parameter_count];
    for (interest_row, output_row) in profiled.iter_mut().enumerate() {
        for (interest_column, output) in output_row.iter_mut().enumerate() {
            let nuisance_correction: f64 = joint_information[interest_row]
                .iter()
                .skip(interest_parameter_count)
                .zip(solved_cross_block.iter())
                .map(|(cross, solved_row)| cross * solved_row[interest_column])
                .sum();
            *output = joint_information[interest_row][interest_column] - nuisance_correction;
        }
    }

    // Remove only antisymmetric floating-point roundoff introduced by the solve.
    let symmetric_profiled: Vec<Vec<f64>> = profiled
        .iter()
        .enumerate()
        .map(|(row_index, row)| {
            row.iter()
                .enumerate()
                .map(|(column_index, value)| {
                    0.5 * (value + profiled[column_index][row_index])
                })
                .collect()
        })
        .collect();
    profiled = symmetric_profiled;
    if profiled.iter().flatten().any(|value| !value.is_finite()) {
        return Err(ModelError::InvalidCovariance(
            "profiled_fisher_information_nonfinite",
        ));
    }
    Ok(profiled)
}

fn cholesky_with_relative_pivot_tolerance(
    matrix: &[Vec<f64>],
    relative_tolerance: f64,
    matrix_scale: f64,
) -> Result<Vec<Vec<f64>>, ModelError> {
    let dimension = matrix.len();
    let mut lower = vec![vec![0.0; dimension]; dimension];

    // Index-based triangular updates make the row/column dependencies explicit.
    #[allow(clippy::needless_range_loop)]
    for row in 0..dimension {
        for column in 0..=row {
            let mut value = matrix[row][column];
            for inner in 0..column {
                value -= lower[row][inner] * lower[column][inner];
            }
            if row == column {
                if !value.is_finite() || value <= relative_tolerance * matrix_scale {
                    return Err(ModelError::InvalidCovariance(
                        "nuisance_fisher_block_is_singular_or_ill_conditioned",
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
    for (row_index, row) in lower.iter().enumerate() {
        let known_sum: f64 = row
            .iter()
            .take(row_index)
            .zip(forward.iter())
            .map(|(coefficient, solved)| coefficient * solved)
            .sum();
        forward[row_index] = (right_hand_side[row_index] - known_sum) / row[row_index];
    }

    let mut solution = vec![0.0; dimension];
    for (row_index, row) in lower.iter().enumerate().rev() {
        let known_sum: f64 = lower
            .iter()
            .skip(row_index + 1)
            .zip(solution.iter().skip(row_index + 1))
            .map(|(later_row, solved)| later_row[row_index] * solved)
            .sum();
        solution[row_index] = (forward[row_index] - known_sum) / row[row_index];
    }
    if solution.iter().any(|value| !value.is_finite()) {
        return Err(ModelError::InvalidCovariance(
            "nuisance_fisher_solve_nonfinite",
        ));
    }
    Ok(solution)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{identifiability_summary, ModelError};

    #[test]
    fn profiling_nuisance_matches_analytic_schur_complement() {
        let joint = vec![vec![5.0, 4.0], vec![4.0, 5.0]];
        let profiled = profile_nuisance_fisher_information(&joint, 1, 1.0e-12).expect("profiled");
        assert!((profiled[0][0] - 1.8).abs() < 1.0e-12);
    }

    #[test]
    fn normalization_and_constant_bias_are_exactly_observationally_equivalent() {
        // A multiplicative normalization and a constant log-bias both change
        // log(mu) with the same derivative. The resulting Fisher matrix has
        // rank one, and profiling one out must leave zero information for the
        // other. This is a mathematical gauge freedom, not a sampler failure.
        let joint = vec![vec![100.0, 100.0], vec![100.0, 100.0]];
        let profiled = profile_nuisance_fisher_information(&joint, 1, 1.0e-12).expect("profiled");
        assert!(profiled[0][0].abs() < 1.0e-12);
        let summary = identifiability_summary(&profiled, 1.0e-10).expect("summary");
        assert_eq!(summary.effective_rank, 0);
        assert_eq!(summary.log_determinant, None);
    }

    #[test]
    fn profiling_multiple_nuisance_parameters_preserves_interest_dimension() {
        let joint = vec![
            vec![10.0, 1.0, 2.0],
            vec![1.0, 8.0, 0.5],
            vec![2.0, 0.5, 9.0],
        ];
        let profiled = profile_nuisance_fisher_information(&joint, 1, 1.0e-12).expect("profiled");
        assert_eq!(profiled.len(), 1);
        assert_eq!(profiled[0].len(), 1);
        assert!(profiled[0][0] > 0.0);
    }

    #[test]
    fn singular_nuisance_block_fails_closed_without_ridge() {
        let joint = vec![vec![2.0, 0.0], vec![0.0, 0.0]];
        assert_eq!(
            profile_nuisance_fisher_information(&joint, 1, 1.0e-12),
            Err(ModelError::InvalidCovariance(
                "nuisance_fisher_block_is_singular",
            ))
        );
    }

    #[test]
    fn profiling_rejects_invalid_partition_and_asymmetric_input() {
        assert_eq!(
            profile_nuisance_fisher_information(&[vec![1.0]], 1, 1.0e-12),
            Err(ModelError::MismatchedDataLengths)
        );
        assert_eq!(
            profile_nuisance_fisher_information(&[vec![1.0, 0.2], vec![0.1, 1.0]], 1, 1.0e-12),
            Err(ModelError::InvalidCovariance(
                "joint_fisher_information_must_be_symmetric",
            ))
        );
    }
}
