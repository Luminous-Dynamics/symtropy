// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![allow(clippy::needless_range_loop)]
// Fixed-size 8x8 transfer algebra uses explicit index loops.

//! Bilocal geodesic transfer operators for convention-explicit ray-bundle work.
//!
//! The state ordering is (xi^0..xi^3, ell^0..ell^3), where xi is a connecting
//! vector and ell is its covariant derivative along the fiducial geodesic.
//! This module provides the transfer-operator algebra, exact flat-spacetime
//! BGO-0 reference, and an endpoint-local map between canonical (delta x, delta p)
//! and covariant (xi, ell) variations. Curvature-driven BGO integration and
//! curved-space cross-formulation qualification remain separate open steps.
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
    InvalidGeometry(&'static str),
    InvalidStepCount,
    TidalSymmetryResidual { residual: f64, tolerance: f64 },
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
            Self::InvalidGeometry(name) => write!(f, "cannot construct endpoint map: {name}"),
            Self::InvalidStepCount => write!(
                f,
                "constant-tidal BGO integration requires at least one step"
            ),
            Self::TidalSymmetryResidual {
                residual,
                tolerance,
            } => write!(
                f,
                "optical tidal matrix symmetry residual {residual} exceeds {tolerance}"
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

/// Endpoint conversion between canonical coordinate variations (delta x, delta p)
/// and covariant geodesic-deviation data (xi, ell).
///
/// The map is defined at one event in one coordinate chart and for one fiducial
/// contravariant tangent k^mu. It is not a chart-independent 8x8 array: callers
/// must build separate maps at both endpoints of a propagated canonical flow.
///
/// Given p_mu = g_mu_nu k^nu and delta p_mu =
/// (partial_sigma g_mu_nu) xi^sigma k^nu + g_mu_nu delta k^nu,
/// this implements xi=delta x and
///
/// ell^mu = delta k^mu + Gamma^mu_(alpha beta) k^alpha xi^beta.
///
/// Therefore S maps (delta x, delta p) -> (xi, ell), and a covariant BGO
/// reconstructed from a canonical fundamental matrix is
/// W = S_final * Phi_canonical * inverse(S_initial).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanonicalCovariantEndpointMap {
    matrix: [[f64; STATE_DIMENSION]; STATE_DIMENSION],
    metric_covariant: [[f64; BLOCK_DIMENSION]; BLOCK_DIMENSION],
    lower_left: [[f64; BLOCK_DIMENSION]; BLOCK_DIMENSION],
}

impl CanonicalCovariantEndpointMap {
    /// Construct the endpoint map from already validated geometry tensors.
    ///
    /// Metric derivatives use d[alpha][mu][nu] = partial_alpha g_mu_nu;
    /// connection uses Gamma[rho][mu][nu]; tangent is k^mu in the same chart.
    /// The metric inverse is computed without regularization.
    pub fn from_geometry(
        metric: &super::metric::MetricComponents4,
        derivatives: &super::connection::MetricDerivatives4,
        connection: &super::connection::LeviCivitaConnection4,
        tangent: [f64; BLOCK_DIMENSION],
    ) -> Result<Self, BilocalOperatorError> {
        if tangent.iter().any(|value| !value.is_finite()) {
            return Err(BilocalOperatorError::NonFinite("endpoint tangent"));
        }
        let inverse = metric
            .inverse()
            .map_err(|_| BilocalOperatorError::InvalidGeometry("metric inverse"))?;
        let covariant = *metric.covariant();
        let d = derivatives.components();
        let gamma = connection.symbols();
        let mut lower_left = [[0.0; BLOCK_DIMENSION]; BLOCK_DIMENSION];

        for mu in 0..BLOCK_DIMENSION {
            for beta in 0..BLOCK_DIMENSION {
                let mut connection_term = 0.0;
                for alpha in 0..BLOCK_DIMENSION {
                    connection_term += gamma[mu][alpha][beta] * tangent[alpha];
                }

                let mut metric_derivative_term = 0.0;
                for nu in 0..BLOCK_DIMENSION {
                    for rho in 0..BLOCK_DIMENSION {
                        metric_derivative_term += inverse[mu][nu] * d[beta][nu][rho] * tangent[rho];
                    }
                }

                lower_left[mu][beta] = connection_term - metric_derivative_term;
            }
        }

        let mut matrix = [[0.0; STATE_DIMENSION]; STATE_DIMENSION];
        for index in 0..BLOCK_DIMENSION {
            matrix[index][index] = 1.0;
            for momentum_index in 0..BLOCK_DIMENSION {
                matrix[index + BLOCK_DIMENSION][momentum_index + BLOCK_DIMENSION] =
                    inverse[index][momentum_index];
            }
            for position_index in 0..BLOCK_DIMENSION {
                matrix[index + BLOCK_DIMENSION][position_index] = lower_left[index][position_index];
            }
        }

        for row in &matrix {
            if row.iter().any(|value| !value.is_finite()) {
                return Err(BilocalOperatorError::NonFinite(
                    "endpoint conversion matrix",
                ));
            }
        }

        Ok(Self {
            matrix,
            metric_covariant: covariant,
            lower_left,
        })
    }

    /// Matrix S taking (delta x, delta p) to (xi, ell).
    pub const fn matrix(&self) -> &[[f64; STATE_DIMENSION]; STATE_DIMENSION] {
        &self.matrix
    }

    /// Convert a canonical variation to covariant deviation data.
    pub fn to_covariant(
        &self,
        canonical: [f64; STATE_DIMENSION],
    ) -> Result<[f64; STATE_DIMENSION], BilocalOperatorError> {
        if canonical.iter().any(|value| !value.is_finite()) {
            return Err(BilocalOperatorError::NonFinite("canonical variation"));
        }
        let mut covariant = [0.0; STATE_DIMENSION];
        for row in 0..STATE_DIMENSION {
            for column in 0..STATE_DIMENSION {
                covariant[row] =
                    self.matrix[row][column].mul_add(canonical[column], covariant[row]);
            }
        }
        if covariant.iter().any(|value| !value.is_finite()) {
            return Err(BilocalOperatorError::NonFinite("covariant variation"));
        }
        Ok(covariant)
    }

    /// Convert covariant deviation data (xi, ell) back to (delta x, delta p).
    ///
    /// This uses the inverse block-triangular relation delta p = g (ell - A xi),
    /// avoiding a second numerical 8x8 inversion.
    pub fn to_canonical(
        &self,
        covariant: [f64; STATE_DIMENSION],
    ) -> Result<[f64; STATE_DIMENSION], BilocalOperatorError> {
        if covariant.iter().any(|value| !value.is_finite()) {
            return Err(BilocalOperatorError::NonFinite("covariant variation"));
        }
        let mut canonical = [0.0; STATE_DIMENSION];
        canonical[..BLOCK_DIMENSION].copy_from_slice(&covariant[..BLOCK_DIMENSION]);
        for momentum_index in 0..BLOCK_DIMENSION {
            let mut value = 0.0;
            for mu in 0..BLOCK_DIMENSION {
                let mut corrected_ell = covariant[mu + BLOCK_DIMENSION];
                for position_index in 0..BLOCK_DIMENSION {
                    corrected_ell -=
                        self.lower_left[mu][position_index] * covariant[position_index];
                }
                value += self.metric_covariant[momentum_index][mu] * corrected_ell;
            }
            canonical[momentum_index + BLOCK_DIMENSION] = value;
        }
        if canonical.iter().any(|value| !value.is_finite()) {
            return Err(BilocalOperatorError::NonFinite("canonical variation"));
        }
        Ok(canonical)
    }

    /// Transform a canonical fundamental matrix Phi between endpoint maps:
    /// W_covariant = S_final * Phi_canonical * inverse(S_initial).
    ///
    /// The returned operator acts on (xi_initial, ell_initial) and produces
    /// (xi_final, ell_final). Both endpoint maps must use their own chart,
    /// metric, connection and fiducial tangent.
    pub fn transform_canonical_flow(
        canonical_flow: [[f64; STATE_DIMENSION]; STATE_DIMENSION],
        initial: &Self,
        final_endpoint: &Self,
    ) -> Result<BilocalGeodesicOperator, BilocalOperatorError> {
        for row in &canonical_flow {
            if row.iter().any(|value| !value.is_finite()) {
                return Err(BilocalOperatorError::NonFinite(
                    "canonical fundamental matrix",
                ));
            }
        }

        let mut transformed = [[0.0; STATE_DIMENSION]; STATE_DIMENSION];
        for column in 0..STATE_DIMENSION {
            let mut covariant_basis = [0.0; STATE_DIMENSION];
            covariant_basis[column] = 1.0;
            let canonical_initial = initial.to_canonical(covariant_basis)?;

            let mut canonical_final = [0.0; STATE_DIMENSION];
            for row in 0..STATE_DIMENSION {
                for inner in 0..STATE_DIMENSION {
                    canonical_final[row] = canonical_flow[row][inner]
                        .mul_add(canonical_initial[inner], canonical_final[row]);
                }
            }

            let covariant_final = final_endpoint.to_covariant(canonical_final)?;
            for row in 0..STATE_DIMENSION {
                transformed[row][column] = covariant_final[row];
            }
        }
        BilocalGeodesicOperator::from_matrix(transformed)
    }
}

/// Integrate the first-order BGO equation for a constant symmetric tidal matrix.
///
/// The frame is declared orthonormal and parallel-propagated, the state is
/// (xi, ell), and the convention is
/// D xi/dlambda = ell, D ell/dlambda = T xi.
///
/// This is a controlled numerical kernel / analytic-limit fixture, not a
/// curvature-derived Schwarzschild or Kerr transport result. It requires a
/// finite symmetric 4x4 tidal matrix and uses classical fixed-step RK4.
pub fn integrate_constant_tidal_fixed_step(
    tidal: [[f64; BLOCK_DIMENSION]; BLOCK_DIMENSION],
    affine_interval: f64,
    steps: usize,
    symmetry_tolerance: f64,
) -> Result<BilocalGeodesicOperator, BilocalOperatorError> {
    if !affine_interval.is_finite() {
        return Err(BilocalOperatorError::NonFinite("affine interval"));
    }
    if !symmetry_tolerance.is_finite() || symmetry_tolerance <= 0.0 {
        return Err(BilocalOperatorError::InvalidTolerance(symmetry_tolerance));
    }
    if steps == 0 {
        return Err(BilocalOperatorError::InvalidStepCount);
    }

    let mut symmetry_residual: f64 = 0.0;
    for row in 0..BLOCK_DIMENSION {
        for column in 0..BLOCK_DIMENSION {
            if !tidal[row][column].is_finite() {
                return Err(BilocalOperatorError::NonFinite("optical tidal matrix"));
            }
            symmetry_residual =
                symmetry_residual.max((tidal[row][column] - tidal[column][row]).abs());
        }
    }
    if symmetry_residual > symmetry_tolerance {
        return Err(BilocalOperatorError::TidalSymmetryResidual {
            residual: symmetry_residual,
            tolerance: symmetry_tolerance,
        });
    }

    fn rhs(
        state: &[[f64; STATE_DIMENSION]; STATE_DIMENSION],
        tidal: &[[f64; BLOCK_DIMENSION]; BLOCK_DIMENSION],
    ) -> [[f64; STATE_DIMENSION]; STATE_DIMENSION] {
        let mut derivative = [[0.0; STATE_DIMENSION]; STATE_DIMENSION];
        for row in 0..BLOCK_DIMENSION {
            for column in 0..STATE_DIMENSION {
                derivative[row][column] = state[row + BLOCK_DIMENSION][column];
                let mut value = 0.0;
                for inner in 0..BLOCK_DIMENSION {
                    value = tidal[row][inner].mul_add(state[inner][column], value);
                }
                derivative[row + BLOCK_DIMENSION][column] = value;
            }
        }
        derivative
    }

    fn add_scaled(
        state: &[[f64; STATE_DIMENSION]; STATE_DIMENSION],
        derivative: &[[f64; STATE_DIMENSION]; STATE_DIMENSION],
        scale: f64,
    ) -> [[f64; STATE_DIMENSION]; STATE_DIMENSION] {
        let mut result = [[0.0; STATE_DIMENSION]; STATE_DIMENSION];
        for row in 0..STATE_DIMENSION {
            for column in 0..STATE_DIMENSION {
                result[row][column] = derivative[row][column].mul_add(scale, state[row][column]);
            }
        }
        result
    }

    let mut state = [[0.0; STATE_DIMENSION]; STATE_DIMENSION];
    for index in 0..STATE_DIMENSION {
        state[index][index] = 1.0;
    }
    let step = affine_interval / steps as f64;
    for _ in 0..steps {
        let k1 = rhs(&state, &tidal);
        let stage2 = add_scaled(&state, &k1, 0.5 * step);
        let k2 = rhs(&stage2, &tidal);
        let stage3 = add_scaled(&state, &k2, 0.5 * step);
        let k3 = rhs(&stage3, &tidal);
        let stage4 = add_scaled(&state, &k3, step);
        let k4 = rhs(&stage4, &tidal);

        for row in 0..STATE_DIMENSION {
            for column in 0..STATE_DIMENSION {
                state[row][column] += step / 6.0
                    * (k1[row][column]
                        + 2.0 * k2[row][column]
                        + 2.0 * k3[row][column]
                        + k4[row][column]);
                if !state[row][column].is_finite() {
                    return Err(BilocalOperatorError::NonFinite(
                        "constant-tidal BGO integration",
                    ));
                }
            }
        }
    }

    BilocalGeodesicOperator::from_matrix(state)
}

#[cfg(test)]
mod tests {
    use super::{BilocalGeodesicOperator, BilocalOperatorError, CanonicalCovariantEndpointMap};
    use crate::{
        MassKg, SchwarzschildBlackHole,
        connection::{LeviCivitaConnection4, MetricDerivatives4},
        metric::{MetricComponents4, schwarzschild_exterior_metric},
    };

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
    #[test]
    fn minkowski_endpoint_map_round_trips_canonical_variations() {
        let metric = MetricComponents4::new([
            [-1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
        .unwrap();
        let derivatives = MetricDerivatives4::zero();
        let connection =
            LeviCivitaConnection4::from_metric_and_derivatives(&metric, &derivatives).unwrap();
        let endpoint = CanonicalCovariantEndpointMap::from_geometry(
            &metric,
            &derivatives,
            &connection,
            [1.0, 1.0, 0.0, 0.0],
        )
        .unwrap();

        let canonical = [0.1, -0.2, 0.3, -0.4, 0.5, -0.6, 0.7, -0.8];
        let covariant = endpoint.to_covariant(canonical).unwrap();
        let recovered = endpoint.to_canonical(covariant).unwrap();
        for index in 0..8 {
            assert!((recovered[index] - canonical[index]).abs() <= TOLERANCE);
        }
        assert_eq!(endpoint.matrix()[0][0], 1.0);
        assert_eq!(endpoint.matrix()[0][4], 0.0);
        assert_eq!(endpoint.matrix()[4][4], -1.0);
    }

    #[test]
    fn schwarzschild_endpoint_map_round_trips_nontrivial_metric_and_connection() {
        let hole = SchwarzschildBlackHole::new(MassKg::new(1.0e30).unwrap());
        let radius = 4.0 * hole.gravitational_radius_m();
        let theta = 1.1;
        let metric = schwarzschild_exterior_metric(hole, radius, theta).unwrap();
        let derivatives = MetricDerivatives4::schwarzschild_exterior(hole, radius, theta).unwrap();
        let connection =
            LeviCivitaConnection4::from_metric_and_derivatives(&metric, &derivatives).unwrap();
        let lapse_squared = 1.0 - hole.horizon_radius_m() / radius;
        // Outgoing radial null tangent normalized by conserved energy E=1.
        let tangent = [1.0 / lapse_squared, 1.0, 0.0, 0.0];
        let endpoint = CanonicalCovariantEndpointMap::from_geometry(
            &metric,
            &derivatives,
            &connection,
            tangent,
        )
        .unwrap();

        let canonical = [0.13, -0.29, 0.31, -0.47, 0.53, -0.61, 0.71, -0.83];
        let covariant = endpoint.to_covariant(canonical).unwrap();
        let recovered = endpoint.to_canonical(covariant).unwrap();
        for index in 0..8 {
            let scale = canonical[index].abs().max(1.0);
            assert!(
                (recovered[index] - canonical[index]).abs() <= 2.0e-12 * scale,
                "index={index}, canonical={}, recovered={}",
                canonical[index],
                recovered[index]
            );
        }
    }

    #[test]
    fn identical_endpoint_maps_transform_canonical_identity_to_covariant_identity() {
        let metric = MetricComponents4::new([
            [-1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
        .unwrap();
        let derivatives = MetricDerivatives4::zero();
        let connection =
            LeviCivitaConnection4::from_metric_and_derivatives(&metric, &derivatives).unwrap();
        let endpoint = CanonicalCovariantEndpointMap::from_geometry(
            &metric,
            &derivatives,
            &connection,
            [1.0, 0.0, 0.0, 0.0],
        )
        .unwrap();
        let mut canonical_identity = [[0.0; 8]; 8];
        for index in 0..8 {
            canonical_identity[index][index] = 1.0;
        }

        let transformed = CanonicalCovariantEndpointMap::transform_canonical_flow(
            canonical_identity,
            &endpoint,
            &endpoint,
        )
        .unwrap();
        assert!(transformed.max_abs_difference(&BilocalGeodesicOperator::identity()) <= TOLERANCE);
    }

    #[test]
    fn endpoint_map_rejects_nonfinite_tangent_and_variations() {
        let metric = MetricComponents4::new([
            [-1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ])
        .unwrap();
        let derivatives = MetricDerivatives4::zero();
        let connection =
            LeviCivitaConnection4::from_metric_and_derivatives(&metric, &derivatives).unwrap();
        assert!(matches!(
            CanonicalCovariantEndpointMap::from_geometry(
                &metric,
                &derivatives,
                &connection,
                [1.0, f64::NAN, 0.0, 0.0]
            ),
            Err(BilocalOperatorError::NonFinite("endpoint tangent"))
        ));

        let endpoint = CanonicalCovariantEndpointMap::from_geometry(
            &metric,
            &derivatives,
            &connection,
            [1.0, 0.0, 0.0, 0.0],
        )
        .unwrap();
        assert!(matches!(
            endpoint.to_covariant([0.0, 0.0, f64::INFINITY, 0.0, 0.0, 0.0, 0.0, 0.0]),
            Err(BilocalOperatorError::NonFinite("canonical variation"))
        ));
    }

    #[test]
    fn constant_zero_tidal_integrator_reduces_to_exact_minkowski_transfer() {
        let zero = [[0.0; 4]; 4];
        let integrated = integrate_constant_tidal_fixed_step(zero, 1.25, 7, TOLERANCE).unwrap();
        let exact = BilocalGeodesicOperator::minkowski(1.25).unwrap();
        assert!(integrated.max_abs_difference(&exact) <= TOLERANCE);
        assert!(integrated.symplectic_residual() <= TOLERANCE);
    }

    #[test]
    fn constant_isotropic_tide_matches_analytic_oscillator_blocks() {
        let omega: f64 = 0.7;
        let interval: f64 = 1.1;
        let mut tidal = [[0.0; 4]; 4];
        for index in 0..4 {
            tidal[index][index] = -omega * omega;
        }
        let integrated =
            integrate_constant_tidal_fixed_step(tidal, interval, 512, TOLERANCE).unwrap();

        let cosine = (omega * interval).cos();
        let sine = (omega * interval).sin();
        for index in 0..4 {
            assert!((integrated.matrix()[index][index] - cosine).abs() <= 1.0e-10);
            assert!((integrated.matrix()[index][index + 4] - sine / omega).abs() <= 1.0e-10);
            assert!((integrated.matrix()[index + 4][index] + omega * sine).abs() <= 1.0e-10);
            assert!((integrated.matrix()[index + 4][index + 4] - cosine).abs() <= 1.0e-10);
        }
        assert!(integrated.symplectic_residual() <= 1.0e-10);
    }

    #[test]
    fn constant_tidal_transfers_compose_under_step_refinement() {
        let mut tidal = [[0.0; 4]; 4];
        tidal[0][0] = -0.49;
        tidal[1][1] = -0.25;
        tidal[2][2] = 0.16;
        tidal[3][3] = 0.04;
        let first = integrate_constant_tidal_fixed_step(tidal, 0.3, 256, TOLERANCE).unwrap();
        let second = integrate_constant_tidal_fixed_step(tidal, 0.4, 256, TOLERANCE).unwrap();
        let combined = second.compose(&first).unwrap();
        let direct = integrate_constant_tidal_fixed_step(tidal, 0.7, 512, TOLERANCE).unwrap();
        assert!(combined.max_abs_difference(&direct) <= 1.0e-9);
        assert!(direct.symplectic_residual() <= 1.0e-9);
    }

    #[test]
    fn constant_tidal_integrator_rejects_asymmetry_nonfinite_and_zero_steps() {
        let mut asymmetric = [[0.0; 4]; 4];
        asymmetric[0][1] = 1.0;
        assert!(matches!(
            integrate_constant_tidal_fixed_step(asymmetric, 1.0, 8, TOLERANCE),
            Err(BilocalOperatorError::TidalSymmetryResidual { .. })
        ));

        let mut nonfinite = [[0.0; 4]; 4];
        nonfinite[0][0] = f64::INFINITY;
        assert_eq!(
            integrate_constant_tidal_fixed_step(nonfinite, 1.0, 8, TOLERANCE),
            Err(BilocalOperatorError::NonFinite("optical tidal matrix"))
        );

        assert_eq!(
            integrate_constant_tidal_fixed_step([[0.0; 4]; 4], 1.0, 0, TOLERANCE),
            Err(BilocalOperatorError::InvalidStepCount)
        );
    }
}
