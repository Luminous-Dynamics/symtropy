// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![allow(clippy::needless_range_loop)]
// Fixed-size 8x8 transfer algebra uses explicit index loops.

//! Bilocal geodesic transfer operators for convention-explicit ray-bundle work.
//!
//! The state ordering is (xi^0..xi^3, ell^0..ell^3), where xi is a connecting
//! vector and ell is its covariant derivative along the fiducial geodesic.
//! This module currently provides the transfer-operator algebra and the exact
//! flat-spacetime BGO-0 reference. Curvature-driven integration and conversion
//! to canonical (delta x, delta p) variables are separate, not-yet-implemented
//! qualification steps.
//!
//! The canonical bilinear form is Omega = [[0, I], [-I, 0]]. Any curvature
//! generator added later must be derived from the declared Riemann convention
//! and checked for Hamiltonian structure; this type does not guess that sign.

use core::fmt;

const STATE_DIMENSION: usize = 8;
const BLOCK_DIMENSION: usize = 4;

/// A convention or numerical failure in bilocal-operator construction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BilocalOperatorError {
    NonFinite(&'static str),
    InvalidTolerance(f64),
    SingularMatrix { pivot: usize, scaled_magnitude: f64 },
}

impl fmt::Display for BilocalOperatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite(name) => write!(f, "{name} contains a non-finite value"),
            Self::InvalidTolerance(value) => {
                write!(f, "tolerance must be positive and finite, got {value}")
            }
            Self::SingularMatrix {
                pivot,
                scaled_magnitude,
            } => write!(
                f,
                "bilocal transfer is singular at pivot {pivot} (scaled magnitude {scaled_magnitude})"
            ),
        }
    }
}

impl std::error::Error for BilocalOperatorError {}

/// An 8x8 linear transfer between endpoint geodesic-deviation data.
///
/// Matrix blocks are ordered as [[W_XX, W_XK], [W_KX, W_KK]], acting on
/// (xi, ell).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BilocalGeodesicOperator {
    matrix: [[f64; STATE_DIMENSION]; STATE_DIMENSION],
}

impl BilocalGeodesicOperator {
    /// Construct from an 8x8 matrix, rejecting every non-finite entry.
    pub fn from_matrix(
        matrix: [[f64; STATE_DIMENSION]; STATE_DIMENSION],
    ) -> Result<Self, BilocalOperatorError> {
        for row in &matrix {
            for value in row {
                if !value.is_finite() {
                    return Err(BilocalOperatorError::NonFinite("bilocal matrix"));
                }
            }
        }
        Ok(Self { matrix })
    }

    /// Construct from four 4x4 blocks in the declared endpoint ordering.
    pub fn from_blocks(
        w_xx: [[f64; BLOCK_DIMENSION]; BLOCK_DIMENSION],
        w_xk: [[f64; BLOCK_DIMENSION]; BLOCK_DIMENSION],
        w_kx: [[f64; BLOCK_DIMENSION]; BLOCK_DIMENSION],
        w_kk: [[f64; BLOCK_DIMENSION]; BLOCK_DIMENSION],
    ) -> Result<Self, BilocalOperatorError> {
        let mut matrix = [[0.0; STATE_DIMENSION]; STATE_DIMENSION];
        for row in 0..BLOCK_DIMENSION {
            for column in 0..BLOCK_DIMENSION {
                matrix[row][column] = w_xx[row][column];
                matrix[row][column + BLOCK_DIMENSION] = w_xk[row][column];
                matrix[row + BLOCK_DIMENSION][column] = w_kx[row][column];
                matrix[row + BLOCK_DIMENSION][column + BLOCK_DIMENSION] = w_kk[row][column];
            }
        }
        Self::from_matrix(matrix)
    }

    /// Identity endpoint transfer.
    pub const fn identity() -> Self {
        let mut matrix = [[0.0; STATE_DIMENSION]; STATE_DIMENSION];
        let mut index = 0;
        while index < STATE_DIMENSION {
            matrix[index][index] = 1.0;
            index += 1;
        }
        Self { matrix }
    }

    /// Exact BGO-0 transfer in Cartesian Minkowski spacetime.
    ///
    /// For affine interval delta_lambda, xi(lambda + delta_lambda) =
    /// xi(lambda) + delta_lambda * ell(lambda), while ell is constant.
    pub fn minkowski(delta_lambda: f64) -> Result<Self, BilocalOperatorError> {
        if !delta_lambda.is_finite() {
            return Err(BilocalOperatorError::NonFinite("affine interval"));
        }
        let mut matrix = [[0.0; STATE_DIMENSION]; STATE_DIMENSION];
        for index in 0..BLOCK_DIMENSION {
            matrix[index][index] = 1.0;
            matrix[index][index + BLOCK_DIMENSION] = delta_lambda;
            matrix[index + BLOCK_DIMENSION][index + BLOCK_DIMENSION] = 1.0;
        }
        Self::from_matrix(matrix)
    }

    /// Return the complete transfer matrix.
    pub const fn matrix(&self) -> &[[f64; STATE_DIMENSION]; STATE_DIMENSION] {
        &self.matrix
    }

    /// Return one 4x4 block; row_block and column_block are 0 or 1.
    pub fn block(
        &self,
        row_block: usize,
        column_block: usize,
    ) -> Option<[[f64; BLOCK_DIMENSION]; BLOCK_DIMENSION]> {
        if row_block > 1 || column_block > 1 {
            return None;
        }
        let row_offset = row_block * BLOCK_DIMENSION;
        let column_offset = column_block * BLOCK_DIMENSION;
        let mut block = [[0.0; BLOCK_DIMENSION]; BLOCK_DIMENSION];
        for row in 0..BLOCK_DIMENSION {
            for column in 0..BLOCK_DIMENSION {
                block[row][column] = self.matrix[row + row_offset][column + column_offset];
            }
        }
        Some(block)
    }

    /// Compose transfers in propagation order.
    ///
    /// self maps the intermediate event to the final event; earlier maps
    /// the initial event to the intermediate event. The returned transfer is
    /// therefore self * earlier.
    pub fn compose(&self, earlier: &Self) -> Result<Self, BilocalOperatorError> {
        let mut product = [[0.0; STATE_DIMENSION]; STATE_DIMENSION];
        for (row, product_row) in product.iter_mut().enumerate() {
            for (column, product_value) in product_row.iter_mut().enumerate() {
                let mut value = 0.0;
                for inner in 0..STATE_DIMENSION {
                    value = self.matrix[row][inner].mul_add(earlier.matrix[inner][column], value);
                }
                *product_value = value;
            }
        }
        Self::from_matrix(product)
    }

    /// Invert the full 8x8 transfer with scaled partial-pivot Gauss-Jordan.
    ///
    /// A singular projected 2x2 Jacobi block does not imply this full transfer
    /// is singular. The pivot threshold is relative to each candidate row's
    /// scale and is never used to regularize or modify the matrix.
    pub fn inverse(&self, tolerance: f64) -> Result<Self, BilocalOperatorError> {
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err(BilocalOperatorError::InvalidTolerance(tolerance));
        }

        let mut augmented = [[0.0; STATE_DIMENSION * 2]; STATE_DIMENSION];
        let mut row_scales = [0.0_f64; STATE_DIMENSION];
        for row in 0..STATE_DIMENSION {
            for column in 0..STATE_DIMENSION {
                let value = self.matrix[row][column];
                augmented[row][column] = value;
                row_scales[row] = row_scales[row].max(value.abs());
            }
            augmented[row][row + STATE_DIMENSION] = 1.0;
            if row_scales[row] == 0.0 {
                return Err(BilocalOperatorError::SingularMatrix {
                    pivot: row,
                    scaled_magnitude: 0.0,
                });
            }
        }

        for pivot_column in 0..STATE_DIMENSION {
            let mut pivot_row = pivot_column;
            let mut best_scaled: f64 = 0.0;
            for candidate in pivot_column..STATE_DIMENSION {
                let scaled = augmented[candidate][pivot_column].abs() / row_scales[candidate];
                if scaled > best_scaled {
                    best_scaled = scaled;
                    pivot_row = candidate;
                }
            }
            if best_scaled <= tolerance {
                return Err(BilocalOperatorError::SingularMatrix {
                    pivot: pivot_column,
                    scaled_magnitude: best_scaled,
                });
            }
            if pivot_row != pivot_column {
                augmented.swap(pivot_row, pivot_column);
                row_scales.swap(pivot_row, pivot_column);
            }

            let pivot = augmented[pivot_column][pivot_column];
            for value in &mut augmented[pivot_column] {
                *value /= pivot;
            }

            let normalized_pivot_row = augmented[pivot_column];
            for row in 0..STATE_DIMENSION {
                if row == pivot_column {
                    continue;
                }
                let factor = augmented[row][pivot_column];
                for column in 0..(STATE_DIMENSION * 2) {
                    augmented[row][column] =
                        (-factor).mul_add(normalized_pivot_row[column], augmented[row][column]);
                }
            }
        }

        let mut inverse = [[0.0; STATE_DIMENSION]; STATE_DIMENSION];
        for row in 0..STATE_DIMENSION {
            for column in 0..STATE_DIMENSION {
                inverse[row][column] = augmented[row][column + STATE_DIMENSION];
            }
        }
        Self::from_matrix(inverse)
    }

    /// Maximum absolute residual in W^T Omega W - Omega, with
    /// Omega = [[0, I_4], [-I_4, 0]] for the (xi, ell) ordering.
    pub fn symplectic_residual(&self) -> f64 {
        let mut maximum: f64 = 0.0;
        for row in 0..STATE_DIMENSION {
            for column in 0..STATE_DIMENSION {
                let mut value = 0.0;
                for index in 0..BLOCK_DIMENSION {
                    value += self.matrix[index][row] * self.matrix[index + BLOCK_DIMENSION][column]
                        - self.matrix[index + BLOCK_DIMENSION][row] * self.matrix[index][column];
                }
                let expected = if row < BLOCK_DIMENSION && column == row + BLOCK_DIMENSION {
                    1.0
                } else if row >= BLOCK_DIMENSION && column + BLOCK_DIMENSION == row {
                    -1.0
                } else {
                    0.0
                };
                maximum = maximum.max((value - expected).abs());
            }
        }
        maximum
    }

    /// Compare two transfers using the maximum absolute entrywise difference.
    pub fn max_abs_difference(&self, other: &Self) -> f64 {
        let mut maximum: f64 = 0.0;
        for row in 0..STATE_DIMENSION {
            for column in 0..STATE_DIMENSION {
                maximum = maximum.max((self.matrix[row][column] - other.matrix[row][column]).abs());
            }
        }
        maximum
    }
}

#[cfg(test)]
mod tests {
    use super::{BilocalGeodesicOperator, BilocalOperatorError};

    const TOLERANCE: f64 = 1.0e-12;

    #[test]
    fn minkowski_transfer_has_exact_four_block_known_answer() {
        let interval = 0.375;
        let transfer = BilocalGeodesicOperator::minkowski(interval).unwrap();
        let identity = core::array::from_fn(|row| {
            core::array::from_fn(|column| if row == column { 1.0 } else { 0.0 })
        });
        let zero = [[0.0; 4]; 4];
        let mut upper_right = zero;
        for index in 0..4 {
            upper_right[index][index] = interval;
        }

        assert_eq!(transfer.block(0, 0), Some(identity));
        assert_eq!(transfer.block(0, 1), Some(upper_right));
        assert_eq!(transfer.block(1, 0), Some(zero));
        assert_eq!(transfer.block(1, 1), Some(identity));
        assert_eq!(transfer.block(2, 0), None);
        assert_eq!(transfer.block(0, 2), None);
    }

    #[test]
    fn minkowski_transfers_compose_by_affine_interval_addition() {
        let first = BilocalGeodesicOperator::minkowski(0.125).unwrap();
        let second = BilocalGeodesicOperator::minkowski(0.25).unwrap();
        let composed = second.compose(&first).unwrap();
        let expected = BilocalGeodesicOperator::minkowski(0.375).unwrap();
        assert!(composed.max_abs_difference(&expected) <= TOLERANCE);
    }

    #[test]
    fn minkowski_transfer_inverse_is_backward_propagation() {
        let forward = BilocalGeodesicOperator::minkowski(0.75).unwrap();
        let inverse = forward.inverse(TOLERANCE).unwrap();
        let expected = BilocalGeodesicOperator::minkowski(-0.75).unwrap();
        assert!(inverse.max_abs_difference(&expected) <= TOLERANCE);
        let identity = forward.compose(&inverse).unwrap();
        assert!(identity.max_abs_difference(&BilocalGeodesicOperator::identity()) <= TOLERANCE);
    }

    #[test]
    fn minkowski_transfer_preserves_wronskian_symplectic_form() {
        let transfer = BilocalGeodesicOperator::minkowski(12.5).unwrap();
        assert!(transfer.symplectic_residual() <= TOLERANCE);
    }

    #[test]
    fn identity_and_zero_length_transfer_are_identical() {
        let transfer = BilocalGeodesicOperator::minkowski(0.0).unwrap();
        assert_eq!(transfer, BilocalGeodesicOperator::identity());
    }

    #[test]
    fn constructors_reject_nonfinite_values_without_repair() {
        assert_eq!(
            BilocalGeodesicOperator::minkowski(f64::NAN),
            Err(BilocalOperatorError::NonFinite("affine interval"))
        );
        let mut matrix = [[0.0; 8]; 8];
        matrix[3][7] = f64::INFINITY;
        assert_eq!(
            BilocalGeodesicOperator::from_matrix(matrix),
            Err(BilocalOperatorError::NonFinite("bilocal matrix"))
        );
    }

    #[test]
    fn inverse_rejects_singular_matrix_and_invalid_tolerance() {
        let mut singular = [[0.0; 8]; 8];
        for index in 0..7 {
            singular[index][index] = 1.0;
        }
        let operator = BilocalGeodesicOperator::from_matrix(singular).unwrap();
        assert!(matches!(
            operator.inverse(TOLERANCE),
            Err(BilocalOperatorError::SingularMatrix { .. })
        ));
        assert_eq!(
            operator.inverse(0.0),
            Err(BilocalOperatorError::InvalidTolerance(0.0))
        );
    }
}
