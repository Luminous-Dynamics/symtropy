// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Correlation-aware relative-state covariance conditioning for NASA-style 3D-Nc.
//!
//! This module consumes already-mapped Cartesian position/velocity covariances
//! for a primary and secondary object at one common epoch and in one common
//! frame. It assembles the relative-state covariance, including cross-object
//! covariance, and derives the conditional Gaussian blocks used by the
//! pointwise Ncdot kernel.
//!
//! State order is [position; velocity]. The supplied cross covariance is
//! Cov(secondary, primary), so for x_rel = x_secondary - x_primary:
//!
//! P_rel = P_secondary + P_primary - P_secondary,primary
//!         - P_secondary,primary^T.
//!
//! The block partition is P_rel = [[A, B^T], [B, C]], where A is position
//! covariance and B is velocity-position covariance. The derived quantities
//! are A^-1, Q1 = B A^-1, Q2 = C - B A^-1 B^T, and
//! logZ = 0.5 * (3 ln(2π) + ln(det(A))).
//!
//! This reproduces the conditioning algebra used by NASA CARA Pc3D_Hall.m.
//! It does not perform PeakOverlapPos, equinoctial-to-Cartesian covariance
//! mapping, orbital propagation, or covariance realism assessment.
//!
//! NASA source revision: CARA_Analysis_Tools commit
//! 43f0160f94d76a91805d86521c3e8fa0d09ecd68 (November 2026 update).

use crate::KernelError;
use crate::ncdot_pointwise::NcdotPointwiseInputs;
use core::f64::consts::TAU;

/// Fixed-size Cartesian state covariance, with state order [position; velocity].
pub type StateCovariance6 = [[f64; 6]; 6];

/// Inputs for assembling a correlation-aware relative-state Gaussian.
///
/// All matrices must use the same Cartesian frame, epoch and consistent units.
/// The cross covariance orientation is explicitly Cov(secondary, primary).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RelativeNcdotCovarianceInputs {
    /// Primary object's 6×6 Cartesian state covariance.
    pub primary_covariance: StateCovariance6,
    /// Secondary object's 6×6 Cartesian state covariance.
    pub secondary_covariance: StateCovariance6,
    /// Cross-object covariance Cov(secondary, primary), not a correlation coefficient.
    ///
    /// Supply an all-zero matrix only when independence is an explicit modelling
    /// assumption. Its transpose is used for Cov(primary, secondary).
    pub secondary_primary_cross_covariance: StateCovariance6,
    /// Mean relative position, secondary minus primary, in the current frame.
    pub mean_relative_position: [f64; 3],
    /// Mean relative velocity, secondary minus primary, in the current frame.
    pub mean_relative_velocity: [f64; 3],
}

/// NASA differential-correction-parameter inputs for a rank-one cross covariance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DcpCrossCovarianceInputs {
    /// Primary-object DCP sigma value from NASA's covXcorr parameter structure.
    pub primary_sigma: f64,
    /// Primary-object six-element sensitivity vector in the current frame.
    pub primary_sensitivity: [f64; 6],
    /// Secondary-object DCP sigma value from NASA's covXcorr parameter structure.
    pub secondary_sigma: f64,
    /// Secondary-object six-element sensitivity vector in the current frame.
    pub secondary_sensitivity: [f64; 6],
}

/// Relative Gaussian blocks required by the NASA-style Ncdot pointwise kernel.
#[derive(Debug, Clone, PartialEq)]
pub struct RelativeNcdotConditionals {
    /// Assembled relative 6×6 covariance, state order [position; velocity].
    pub relative_state_covariance: StateCovariance6,
    /// Relative position covariance A.
    pub position_covariance: [[f64; 3]; 3],
    /// Inverse relative position covariance A^-1.
    pub inverse_position_covariance: [[f64; 3]; 3],
    /// Conditional velocity regression Q1 = B A^-1.
    pub conditional_velocity_regression: [[f64; 3]; 3],
    /// Conditional velocity covariance Q2 = C - B A^-1 B^T.
    pub conditional_velocity_covariance: [[f64; 3]; 3],
    /// Natural logarithm of det(A).
    pub position_log_determinant: f64,
    /// Negative log of the normalized three-dimensional position density scale.
    ///
    /// This is NASA's -log(Ns0) and equals
    /// 0.5 * (3 ln(2π) + ln(det(A))).
    pub log_position_density_normalizer: f64,
    /// Mean relative position used to build this conditional distribution.
    pub mean_relative_position: [f64; 3],
    /// Mean relative velocity used to build this conditional distribution.
    pub mean_relative_velocity: [f64; 3],
    /// Whether a nonzero cross-object covariance was supplied.
    pub cross_object_correlation_present: bool,
}

impl RelativeNcdotConditionals {
    /// Build the fixed-direction inputs expected by the pointwise Ncdot kernel.
    ///
    /// Direction and hard-body radius are supplied for each sphere evaluation;
    /// all conditional Gaussian blocks and relative means are reused unchanged.
    pub fn pointwise_inputs(
        &self,
        direction: [f64; 3],
        hard_body_radius: f64,
    ) -> NcdotPointwiseInputs {
        NcdotPointwiseInputs {
            direction,
            hard_body_radius,
            mean_relative_position: self.mean_relative_position,
            mean_relative_velocity: self.mean_relative_velocity,
            inverse_position_covariance: self.inverse_position_covariance,
            conditional_velocity_regression: self.conditional_velocity_regression,
            conditional_velocity_covariance: self.conditional_velocity_covariance,
            log_position_density_normalizer: self.log_position_density_normalizer,
        }
    }
}

/// Construct NASA's DCP rank-one cross-object covariance.
///
/// NASA's covariance correction is
/// sigp*sigs*(Gs*Gp^T + Gp*Gs^T) in the relative covariance. Therefore this
/// function returns Cov(secondary, primary) = sigp*sigs*Gs*Gp^T. The caller
/// passes this matrix as secondary_primary_cross_covariance to the assembler.
///
/// DCP sigmas are the scalar quantities defined by NASA's covXcorr interface;
/// they are not silently reinterpreted as a Pearson correlation coefficient.
pub fn dcp_secondary_primary_cross_covariance(
    inputs: DcpCrossCovarianceInputs,
) -> Result<StateCovariance6, KernelError> {
    if !inputs.primary_sigma.is_finite()
        || !inputs.secondary_sigma.is_finite()
        || inputs.primary_sigma < 0.0
        || inputs.secondary_sigma < 0.0
    {
        return Err(KernelError::InvalidDcpSigma);
    }
    if !inputs
        .primary_sensitivity
        .iter()
        .all(|value| value.is_finite())
        || !inputs
            .secondary_sensitivity
            .iter()
            .all(|value| value.is_finite())
    {
        return Err(KernelError::NonFiniteValue);
    }

    let scale = inputs.primary_sigma * inputs.secondary_sigma;
    if !scale.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }

    let mut covariance = [[0.0; 6]; 6];
    for (i, row) in covariance.iter_mut().enumerate() {
        for (j, value) in row.iter_mut().enumerate() {
            *value = scale * inputs.secondary_sensitivity[i] * inputs.primary_sensitivity[j];
            if !value.is_finite() {
                return Err(KernelError::NumericalOverflow);
            }
        }
    }
    Ok(covariance)
}

/// Assemble the joint and relative covariances, then derive NASA's conditional blocks.
///
/// The complete 12×12 joint covariance is validated as positive semidefinite
/// before the relative covariance is formed. This prevents an impossible
/// cross-object covariance from being accepted merely because a downstream
/// relative covariance happens to look plausible.
///
/// The position block A must be strictly positive definite. The conditional
/// velocity Schur complement Q2 must be positive semidefinite. No covariance
/// repair, eigenvalue clipping, diagonal jitter or silent independence fallback
/// is performed.
pub fn build_relative_ncdot_conditionals(
    inputs: RelativeNcdotCovarianceInputs,
) -> Result<RelativeNcdotConditionals, KernelError> {
    validate_finite_vector(inputs.mean_relative_position)?;
    validate_finite_vector(inputs.mean_relative_velocity)?;
    validate_finite_state_covariance(inputs.primary_covariance)?;
    validate_finite_state_covariance(inputs.secondary_covariance)?;
    validate_finite_state_covariance(inputs.secondary_primary_cross_covariance)?;

    validate_symmetric_6(inputs.primary_covariance)?;
    validate_symmetric_6(inputs.secondary_covariance)?;

    let joint = assemble_joint_covariance(
        inputs.primary_covariance,
        inputs.secondary_covariance,
        inputs.secondary_primary_cross_covariance,
    );
    validate_positive_semidefinite(&joint, KernelError::InvalidJointCovariance)?;

    let relative = assemble_relative_covariance(
        inputs.primary_covariance,
        inputs.secondary_covariance,
        inputs.secondary_primary_cross_covariance,
    );
    validate_finite_state_covariance(relative)?;
    validate_symmetric_6(relative)?;
    validate_positive_semidefinite(&relative, KernelError::InvalidJointCovariance)?;

    let position = block_position(relative);
    let velocity_position = block_velocity_position(relative);
    let velocity = block_velocity(relative);
    validate_symmetric_3(position)?;

    let lower = cholesky_3(position)?;
    let inverse_position = inverse_from_cholesky_3(lower)?;
    let position_log_determinant = 2.0 * (lower[0][0].ln() + lower[1][1].ln() + lower[2][2].ln());
    if !position_log_determinant.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }

    let conditional_velocity_regression = multiply_3(velocity_position, inverse_position);
    let regression_times_transpose = multiply_3(
        conditional_velocity_regression,
        transpose_3(velocity_position),
    );
    let mut conditional_velocity_covariance = subtract_3(velocity, regression_times_transpose);
    validate_finite_matrix_3(conditional_velocity_covariance)?;
    validate_symmetric_3(conditional_velocity_covariance)?;
    conditional_velocity_covariance = symmetrize_3(conditional_velocity_covariance);
    validate_positive_semidefinite(
        &conditional_velocity_covariance,
        KernelError::InvalidConditionalVelocityCovariance,
    )?;

    let log_position_density_normalizer = 0.5 * (3.0 * TAU.ln() + position_log_determinant);
    if !log_position_density_normalizer.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }

    let cross_object_correlation_present = inputs
        .secondary_primary_cross_covariance
        .iter()
        .flatten()
        .any(|value| *value != 0.0);

    Ok(RelativeNcdotConditionals {
        relative_state_covariance: relative,
        position_covariance: position,
        inverse_position_covariance: inverse_position,
        conditional_velocity_regression,
        conditional_velocity_covariance,
        position_log_determinant,
        log_position_density_normalizer,
        mean_relative_position: inputs.mean_relative_position,
        mean_relative_velocity: inputs.mean_relative_velocity,
        cross_object_correlation_present,
    })
}

fn assemble_joint_covariance(
    primary: StateCovariance6,
    secondary: StateCovariance6,
    secondary_primary: StateCovariance6,
) -> [[f64; 12]; 12] {
    let mut joint = [[0.0; 12]; 12];
    for i in 0..6 {
        for j in 0..6 {
            joint[i][j] = primary[i][j];
            joint[i + 6][j + 6] = secondary[i][j];
            joint[i + 6][j] = secondary_primary[i][j];
            joint[i][j + 6] = secondary_primary[j][i];
        }
    }
    joint
}

fn assemble_relative_covariance(
    primary: StateCovariance6,
    secondary: StateCovariance6,
    secondary_primary: StateCovariance6,
) -> StateCovariance6 {
    let mut relative = [[0.0; 6]; 6];
    for i in 0..6 {
        for j in 0..6 {
            relative[i][j] =
                primary[i][j] + secondary[i][j] - secondary_primary[i][j] - secondary_primary[j][i];
        }
    }
    relative
}

fn block_position(relative: StateCovariance6) -> [[f64; 3]; 3] {
    let mut block = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            block[i][j] = relative[i][j];
        }
    }
    block
}

fn block_velocity_position(relative: StateCovariance6) -> [[f64; 3]; 3] {
    let mut block = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            block[i][j] = relative[i + 3][j];
        }
    }
    block
}

fn block_velocity(relative: StateCovariance6) -> [[f64; 3]; 3] {
    let mut block = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            block[i][j] = relative[i + 3][j + 3];
        }
    }
    block
}

fn transpose_3(matrix: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut transposed = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            transposed[i][j] = matrix[j][i];
        }
    }
    transposed
}

fn multiply_3(left: [[f64; 3]; 3], right: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut product = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            for k in 0..3 {
                product[i][j] += left[i][k] * right[k][j];
            }
        }
    }
    product
}

fn subtract_3(left: [[f64; 3]; 3], right: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut difference = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            difference[i][j] = left[i][j] - right[i][j];
        }
    }
    difference
}

fn symmetrize_3(matrix: [[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut symmetric = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            symmetric[i][j] = 0.5 * (matrix[i][j] + matrix[j][i]);
        }
    }
    symmetric
}

fn cholesky_3(matrix: [[f64; 3]; 3]) -> Result<[[f64; 3]; 3], KernelError> {
    let mut lower = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..=i {
            let mut residual = matrix[i][j];
            for (left, right) in lower[i].iter().zip(lower[j].iter()).take(j) {
                residual -= left * right;
            }
            if i == j {
                if !residual.is_finite() || residual <= 0.0 {
                    return Err(KernelError::NotPositiveDefinite);
                }
                lower[i][j] = residual.sqrt();
            } else {
                lower[i][j] = residual / lower[j][j];
            }
        }
    }
    if lower.iter().flatten().all(|value| value.is_finite()) {
        Ok(lower)
    } else {
        Err(KernelError::NumericalOverflow)
    }
}

fn inverse_from_cholesky_3(lower: [[f64; 3]; 3]) -> Result<[[f64; 3]; 3], KernelError> {
    let mut inverse = [[0.0; 3]; 3];
    for (column, inverse_row) in inverse.iter_mut().enumerate() {
        let mut forward = [0.0; 3];
        for i in 0..3 {
            let mut value = if i == column { 1.0 } else { 0.0 };
            for k in 0..i {
                value -= lower[i][k] * forward[k];
            }
            forward[i] = value / lower[i][i];
        }

        let mut solution = [0.0; 3];
        for i in (0..3).rev() {
            let mut value = forward[i];
            for k in (i + 1)..3 {
                value -= lower[k][i] * solution[k];
            }
            solution[i] = value / lower[i][i];
        }

        for (row, value) in inverse_row.iter_mut().enumerate() {
            *value = solution[row];
        }
    }
    if inverse.iter().flatten().all(|value| value.is_finite()) {
        Ok(symmetrize_3(inverse))
    } else {
        Err(KernelError::NumericalOverflow)
    }
}

fn validate_finite_vector(vector: [f64; 3]) -> Result<(), KernelError> {
    if vector.iter().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(KernelError::NonFiniteValue)
    }
}

fn validate_finite_state_covariance(covariance: StateCovariance6) -> Result<(), KernelError> {
    if covariance.iter().flatten().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(KernelError::NonFiniteValue)
    }
}

fn validate_finite_matrix_3(matrix: [[f64; 3]; 3]) -> Result<(), KernelError> {
    if matrix.iter().flatten().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(KernelError::NumericalOverflow)
    }
}

fn validate_symmetric_6(matrix: StateCovariance6) -> Result<(), KernelError> {
    let scale = matrix
        .iter()
        .flatten()
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
    let tolerance = 256.0 * f64::EPSILON * scale;
    for (i, row) in matrix.iter().enumerate() {
        for (j, value) in row.iter().enumerate().take(i) {
            if (value - matrix[j][i]).abs() > tolerance {
                return Err(KernelError::NonSymmetricMatrix);
            }
        }
    }
    Ok(())
}

fn validate_symmetric_3(matrix: [[f64; 3]; 3]) -> Result<(), KernelError> {
    let scale = matrix
        .iter()
        .flatten()
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
    let tolerance = 256.0 * f64::EPSILON * scale;
    for (i, row) in matrix.iter().enumerate() {
        for (j, value) in row.iter().enumerate().take(i) {
            if (value - matrix[j][i]).abs() > tolerance {
                return Err(KernelError::NonSymmetricMatrix);
            }
        }
    }
    Ok(())
}

/// Unpivoted LDLᵀ PSD validation. Zero pivots require zero residual cross terms.
///
/// The tolerance scales with the largest absolute matrix entry. Treating a
/// small positive pivot as zero is intentionally conservative: unresolved
/// directions with material cross terms are rejected rather than repaired.
#[allow(clippy::needless_range_loop)]
fn validate_positive_semidefinite<const N: usize>(
    matrix: &[[f64; N]; N],
    error: KernelError,
) -> Result<(), KernelError> {
    let scale = matrix
        .iter()
        .flatten()
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
    if scale == 0.0 {
        return Ok(());
    }
    let tolerance = 512.0 * f64::EPSILON * scale;
    let mut lower = [[0.0; N]; N];
    let mut diagonal = [0.0; N];

    for pivot in 0..N {
        let mut pivot_value = matrix[pivot][pivot];
        for previous in 0..pivot {
            pivot_value -= lower[pivot][previous] * lower[pivot][previous] * diagonal[previous];
        }
        if !pivot_value.is_finite() || pivot_value < -tolerance {
            return Err(error);
        }

        if pivot_value <= tolerance {
            diagonal[pivot] = 0.0;
            for row in (pivot + 1)..N {
                let mut residual = matrix[row][pivot];
                for previous in 0..pivot {
                    residual -= lower[row][previous] * lower[pivot][previous] * diagonal[previous];
                }
                if !residual.is_finite() || residual.abs() > tolerance {
                    return Err(error);
                }
                lower[row][pivot] = 0.0;
            }
        } else {
            diagonal[pivot] = pivot_value;
            for row in (pivot + 1)..N {
                let mut residual = matrix[row][pivot];
                for previous in 0..pivot {
                    residual -= lower[row][previous] * lower[pivot][previous] * diagonal[previous];
                }
                if !residual.is_finite() {
                    return Err(KernelError::NumericalOverflow);
                }
                lower[row][pivot] = residual / pivot_value;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ncdot_pointwise::evaluate_ncdot_pointwise;

    fn zero_covariance() -> StateCovariance6 {
        [[0.0; 6]; 6]
    }

    fn diagonal_covariance(diagonal: [f64; 6]) -> StateCovariance6 {
        let mut covariance = [[0.0; 6]; 6];
        for i in 0..6 {
            covariance[i][i] = diagonal[i];
        }
        covariance
    }

    fn base_inputs() -> RelativeNcdotCovarianceInputs {
        RelativeNcdotCovarianceInputs {
            primary_covariance: diagonal_covariance([4.0, 3.0, 2.0, 1.25, 0.92, 0.58]),
            secondary_covariance: zero_covariance(),
            secondary_primary_cross_covariance: zero_covariance(),
            mean_relative_position: [0.1, -0.2, 0.3],
            mean_relative_velocity: [-1.0, 0.2, 0.1],
        }
    }

    #[test]
    fn independent_objects_recover_known_conditional_blocks_and_logz() {
        let result =
            build_relative_ncdot_conditionals(base_inputs()).expect("independent Gaussian blocks");
        assert_eq!(result.inverse_position_covariance[0][0], 0.25);
        assert!((result.inverse_position_covariance[1][1] - 1.0 / 3.0).abs() < 1.0e-14);
        assert!((result.inverse_position_covariance[2][2] - 0.5).abs() < 1.0e-14);
        assert_eq!(result.conditional_velocity_regression, [[0.0; 3]; 3]);
        assert_eq!(
            result.conditional_velocity_covariance,
            [[1.25, 0.0, 0.0], [0.0, 0.92, 0.0], [0.0, 0.0, 0.58]]
        );
        let logdet = (4.0_f64 * 3.0 * 2.0).ln();
        let expected_logz = 0.5 * (3.0 * TAU.ln() + logdet);
        assert!((result.log_position_density_normalizer - expected_logz).abs() < 1.0e-14);
        assert!(!result.cross_object_correlation_present);
    }

    #[test]
    fn cross_object_covariance_is_subtracted_with_both_orientations() {
        let mut inputs = base_inputs();
        inputs.primary_covariance = diagonal_covariance([2.0; 6]);
        inputs.secondary_covariance = diagonal_covariance([3.0; 6]);
        for i in 0..6 {
            inputs.secondary_primary_cross_covariance[i][i] = 0.25;
        }
        let result =
            build_relative_ncdot_conditionals(inputs).expect("valid correlated joint model");
        for i in 0..6 {
            assert!((result.relative_state_covariance[i][i] - 4.5).abs() < 1.0e-14);
        }
        assert!(result.cross_object_correlation_present);
    }

    #[test]
    fn nonzero_position_velocity_covariance_produces_known_schur_complement() {
        let mut inputs = base_inputs();
        inputs.primary_covariance = zero_covariance();
        let mut p = zero_covariance();
        p[0][0] = 4.0;
        p[1][1] = 3.0;
        p[2][2] = 2.0;
        p[3][0] = 1.0;
        p[4][1] = 0.6;
        p[5][2] = 0.4;
        p[0][3] = 1.0;
        p[1][4] = 0.6;
        p[2][5] = 0.4;
        p[3][3] = 1.25;
        p[4][4] = 0.92;
        p[5][5] = 0.58;
        inputs.primary_covariance = p;
        let result = build_relative_ncdot_conditionals(inputs).expect("valid joint covariance");
        let expected_regression = [[0.25, 0.0, 0.0], [0.0, 0.2, 0.0], [0.0, 0.0, 0.2]];
        for (i, expected_row) in expected_regression.iter().enumerate() {
            for (j, expected) in expected_row.iter().enumerate() {
                assert!((result.conditional_velocity_regression[i][j] - expected).abs() < 1.0e-14);
            }
        }
        assert!((result.conditional_velocity_covariance[0][0] - 1.0).abs() < 1.0e-14);
        assert!((result.conditional_velocity_covariance[1][1] - 0.8).abs() < 1.0e-14);
        assert!((result.conditional_velocity_covariance[2][2] - 0.5).abs() < 1.0e-14);
    }

    #[test]
    fn nasa_dcp_cross_covariance_matches_secondary_primary_outer_product() {
        let cross = dcp_secondary_primary_cross_covariance(DcpCrossCovarianceInputs {
            primary_sigma: 0.2,
            primary_sensitivity: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
            secondary_sigma: 0.3,
            secondary_sensitivity: [-1.0, 1.0, -2.0, 2.0, -3.0, 3.0],
        })
        .expect("valid DCP covariance");
        assert!((cross[0][0] - (-0.06)).abs() < 1.0e-15);
        assert!((cross[1][2] - 0.18).abs() < 1.0e-15);
        assert!((cross[5][4] - 0.9).abs() < 1.0e-15);
    }

    #[test]
    fn dcp_cross_covariance_flows_through_relative_assembly() {
        let mut inputs = base_inputs();
        inputs.primary_covariance = diagonal_covariance([4.0; 6]);
        inputs.secondary_covariance = diagonal_covariance([4.0; 6]);
        let cross = dcp_secondary_primary_cross_covariance(DcpCrossCovarianceInputs {
            primary_sigma: 0.1,
            primary_sensitivity: [0.1; 6],
            secondary_sigma: 0.1,
            secondary_sensitivity: [0.2; 6],
        })
        .expect("valid DCP term");
        inputs.secondary_primary_cross_covariance = cross;
        let result = build_relative_ncdot_conditionals(inputs).expect("valid DCP joint covariance");
        assert!(result.cross_object_correlation_present);
        assert!((result.relative_state_covariance[0][0] - (8.0 - 0.0004)).abs() < 1.0e-14);
        assert!((result.relative_state_covariance[0][1] - (-0.0004)).abs() < 1.0e-14);
    }

    #[test]
    fn impossible_joint_cross_covariance_is_rejected_even_when_relative_block_is_positive() {
        let mut inputs = RelativeNcdotCovarianceInputs {
            primary_covariance: diagonal_covariance([1.0; 6]),
            secondary_covariance: diagonal_covariance([100.0; 6]),
            secondary_primary_cross_covariance: zero_covariance(),
            mean_relative_position: [0.0; 3],
            mean_relative_velocity: [0.0; 3],
        };
        for i in 0..6 {
            inputs.secondary_primary_cross_covariance[i][i] = 10.1;
        }
        let naive_relative_diagonal = 1.0 + 100.0 - 2.0 * 10.1;
        assert!(naive_relative_diagonal > 0.0);
        assert_eq!(
            build_relative_ncdot_conditionals(inputs),
            Err(KernelError::InvalidJointCovariance)
        );
    }

    #[test]
    fn nonsymmetric_marginal_covariance_is_rejected() {
        let mut inputs = base_inputs();
        inputs.primary_covariance[0][1] = 0.1;
        assert_eq!(
            build_relative_ncdot_conditionals(inputs),
            Err(KernelError::NonSymmetricMatrix)
        );
    }

    #[test]
    fn position_covariance_must_be_strictly_positive_definite() {
        let mut inputs = base_inputs();
        inputs.primary_covariance[2][2] = 0.0;
        assert_eq!(
            build_relative_ncdot_conditionals(inputs),
            Err(KernelError::NotPositiveDefinite)
        );
    }

    #[test]
    fn invalid_dcp_sigmas_and_nonfinite_sensitivities_fail_closed() {
        let mut inputs = DcpCrossCovarianceInputs {
            primary_sigma: 1.0,
            primary_sensitivity: [1.0; 6],
            secondary_sigma: 1.0,
            secondary_sensitivity: [1.0; 6],
        };
        inputs.primary_sigma = -1.0;
        assert_eq!(
            dcp_secondary_primary_cross_covariance(inputs),
            Err(KernelError::InvalidDcpSigma)
        );

        inputs.primary_sigma = 1.0;
        inputs.secondary_sensitivity[4] = f64::NAN;
        assert_eq!(
            dcp_secondary_primary_cross_covariance(inputs),
            Err(KernelError::NonFiniteValue)
        );
    }

    #[test]
    fn pointwise_adapter_preserves_means_and_derived_conditionals() {
        let result =
            build_relative_ncdot_conditionals(base_inputs()).expect("valid conditional Gaussian");
        let pointwise = result.pointwise_inputs([1.0, 0.0, 0.0], 2.0);
        assert_eq!(pointwise.direction, [1.0, 0.0, 0.0]);
        assert_eq!(pointwise.hard_body_radius, 2.0);
        assert_eq!(pointwise.mean_relative_position, [0.1, -0.2, 0.3]);
        assert_eq!(pointwise.mean_relative_velocity, [-1.0, 0.2, 0.1]);
        assert_eq!(
            pointwise.inverse_position_covariance,
            result.inverse_position_covariance
        );
        assert_eq!(
            pointwise.conditional_velocity_covariance,
            result.conditional_velocity_covariance
        );
        let report = evaluate_ncdot_pointwise(pointwise).expect("pointwise adapter is consumable");
        assert!(report.log_integrand.is_finite());
    }
}
