// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Frame-typed tangent/cotangent norms for dense coordinate re-basing.
//!
//! For q = S y:
//!   v_y = S^{-1} v_q
//!   alpha_y = S^T alpha_q
//!   M_y = S^T M_q S
//!
//! Tangent and cotangent norms are induced by M and M^{-1}, respectively.
//! This module is a small dense reference, not a production multibody solver.
//! Its floating-point admission checks are numerical guards, not formal proofs.

#![forbid(unsafe_code)]

use core::marker::PhantomData;
use std::fmt;

pub mod constraint_metric;
pub mod rank_deficient_constraint;

pub type Vector<const N: usize> = [f64; N];
pub type Matrix<const N: usize> = [[f64; N]; N];

pub trait CoordinateFrame: Copy + 'static {}

/// Type-level identity for the multiplier/residual coordinates of a constraint system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstraintSpace;
impl CoordinateFrame for ConstraintSpace {}

pub use constraint_metric::{
    ConstraintMultiplierV1, ConstraintResidualV1, DelassusMetricV1, rebase_constraint_jacobian,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CartesianQ;
impl CoordinateFrame for CartesianQ {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DenseY;
impl CoordinateFrame for DenseY {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tangent<F: CoordinateFrame, const N: usize> {
    components: Vector<N>,
    _frame: PhantomData<F>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cotangent<F: CoordinateFrame, const N: usize> {
    components: Vector<N>,
    _frame: PhantomData<F>,
}

impl<F: CoordinateFrame, const N: usize> Tangent<F, N> {
    pub fn new(components: Vector<N>) -> Result<Self, MetricErrorV1> {
        validate_vector(&components)?;
        Ok(Self {
            components,
            _frame: PhantomData,
        })
    }

    pub fn components(&self) -> &Vector<N> {
        &self.components
    }
}

impl<F: CoordinateFrame, const N: usize> Cotangent<F, N> {
    pub fn new(components: Vector<N>) -> Result<Self, MetricErrorV1> {
        validate_vector(&components)?;
        Ok(Self {
            components,
            _frame: PhantomData,
        })
    }

    pub fn components(&self) -> &Vector<N> {
        &self.components
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricProfileV1 {
    /// Relative threshold against the matrix infinity norm.
    pub relative_pivot_tolerance: f64,
    /// Relative tolerance used only to admit and symmetrize roundoff asymmetry.
    pub relative_symmetry_tolerance: f64,
}

impl Default for MetricProfileV1 {
    fn default() -> Self {
        Self {
            relative_pivot_tolerance: 64.0 * f64::EPSILON,
            relative_symmetry_tolerance: 256.0 * f64::EPSILON,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricAdmissionReceiptV1 {
    pub maximum_relative_symmetry_defect: f64,
    pub minimum_cholesky_pivot: f64,
    pub matrix_inf_norm: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetricErrorV1 {
    ZeroDimension,
    InvalidProfile,
    NonFiniteMatrix,
    NonFiniteVector,
    NonFiniteIntermediate,
    SingularCoordinateTransform { column: usize },
    NonSymmetricMetric { row: usize, column: usize },
    NotPositiveDefinite { pivot: usize },
    NegativeMetricQuadraticForm,
}

impl fmt::Display for MetricErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for MetricErrorV1 {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metric<F: CoordinateFrame, const N: usize> {
    matrix: Matrix<N>,
    cholesky: Matrix<N>,
    receipt: MetricAdmissionReceiptV1,
    _frame: PhantomData<F>,
}

#[derive(Debug, Clone, Copy)]
struct LuFactorV1<const N: usize> {
    lu: Matrix<N>,
    permutation: [usize; N],
}

#[derive(Debug, Clone, Copy)]
pub struct Rebasis<From: CoordinateFrame, To: CoordinateFrame, const N: usize> {
    /// q-components = S * y-components. S maps To tangent coordinates into From.
    matrix: Matrix<N>,
    factor: LuFactorV1<N>,
    _frame_pair: PhantomData<(From, To)>,
}

fn validate_profile(profile: MetricProfileV1) -> Result<(), MetricErrorV1> {
    if !profile.relative_pivot_tolerance.is_finite()
        || profile.relative_pivot_tolerance <= 0.0
        || !profile.relative_symmetry_tolerance.is_finite()
        || profile.relative_symmetry_tolerance < 0.0
    {
        return Err(MetricErrorV1::InvalidProfile);
    }
    Ok(())
}

fn validate_vector<const N: usize>(vector: &Vector<N>) -> Result<(), MetricErrorV1> {
    if N == 0 {
        return Err(MetricErrorV1::ZeroDimension);
    }
    if vector.iter().any(|value| !value.is_finite()) {
        return Err(MetricErrorV1::NonFiniteVector);
    }
    Ok(())
}

pub fn vector_inf<const N: usize>(vector: &Vector<N>) -> f64 {
    vector.iter().map(|value| value.abs()).fold(0.0, f64::max)
}

pub fn vector_euclidean<const N: usize>(vector: &Vector<N>) -> f64 {
    vector
        .iter()
        .fold(0.0_f64, |norm, value| norm.hypot(*value))
}

fn dot<const N: usize>(left: &Vector<N>, right: &Vector<N>) -> f64 {
    (0..N).map(|index| left[index] * right[index]).sum()
}

fn transpose<const N: usize>(matrix: &Matrix<N>) -> Matrix<N> {
    let mut result = [[0.0; N]; N];
    for row in 0..N {
        for column in 0..N {
            result[row][column] = matrix[column][row];
        }
    }
    result
}

#[cfg(test)]
fn multiply<const N: usize>(left: &Matrix<N>, right: &Matrix<N>) -> Matrix<N> {
    let mut result = [[0.0; N]; N];
    for row in 0..N {
        for column in 0..N {
            for inner in 0..N {
                result[row][column] += left[row][inner] * right[inner][column];
            }
        }
    }
    result
}

fn apply<const N: usize>(matrix: &Matrix<N>, vector: &Vector<N>) -> Vector<N> {
    let mut result = [0.0; N];
    for row in 0..N {
        for column in 0..N {
            result[row] += matrix[row][column] * vector[column];
        }
    }
    result
}

fn matrix_inf<const N: usize>(matrix: &Matrix<N>) -> f64 {
    let mut maximum = 0.0_f64;
    for row in matrix {
        let row_sum = row.iter().map(|value| value.abs()).sum::<f64>();
        maximum = maximum.max(row_sum);
    }
    maximum
}

fn factor<const N: usize>(
    matrix: &Matrix<N>,
    relative_pivot_tolerance: f64,
) -> Result<LuFactorV1<N>, MetricErrorV1> {
    if N == 0 {
        return Err(MetricErrorV1::ZeroDimension);
    }
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(MetricErrorV1::NonFiniteMatrix);
    }

    let scale = matrix_inf(matrix);
    if !scale.is_finite() {
        return Err(MetricErrorV1::NonFiniteIntermediate);
    }
    if scale == 0.0 {
        return Err(MetricErrorV1::SingularCoordinateTransform { column: 0 });
    }
    let threshold = relative_pivot_tolerance * scale;
    if !threshold.is_finite() {
        return Err(MetricErrorV1::NonFiniteIntermediate);
    }

    let mut lu = *matrix;
    let mut permutation = std::array::from_fn(|index| index);
    for column in 0..N {
        let pivot_row = (column..N)
            .max_by(|left, right| lu[*left][column].abs().total_cmp(&lu[*right][column].abs()))
            .ok_or(MetricErrorV1::ZeroDimension)?;
        let pivot = lu[pivot_row][column].abs();
        if !pivot.is_finite() || pivot <= threshold {
            return Err(MetricErrorV1::SingularCoordinateTransform { column });
        }
        if pivot_row != column {
            lu.swap(pivot_row, column);
            permutation.swap(pivot_row, column);
        }

        for row in (column + 1)..N {
            lu[row][column] /= lu[column][column];
            for trailing in (column + 1)..N {
                lu[row][trailing] -= lu[row][column] * lu[column][trailing];
            }
        }
        if lu.iter().flatten().any(|value| !value.is_finite()) {
            return Err(MetricErrorV1::NonFiniteIntermediate);
        }
    }

    Ok(LuFactorV1 { lu, permutation })
}

impl<const N: usize> LuFactorV1<N> {
    fn solve(&self, rhs: &Vector<N>) -> Result<Vector<N>, MetricErrorV1> {
        validate_vector(rhs)?;
        let mut solution = std::array::from_fn(|row| rhs[self.permutation[row]]);

        for row in 0..N {
            for column in 0..row {
                solution[row] -= self.lu[row][column] * solution[column];
            }
        }
        for row in (0..N).rev() {
            for column in (row + 1)..N {
                solution[row] -= self.lu[row][column] * solution[column];
            }
            solution[row] /= self.lu[row][row];
        }

        if solution.iter().any(|value| !value.is_finite()) {
            return Err(MetricErrorV1::NonFiniteIntermediate);
        }
        Ok(solution)
    }
}

impl<F: CoordinateFrame, const N: usize> Metric<F, N> {
    pub fn admit(matrix: Matrix<N>, profile: MetricProfileV1) -> Result<Self, MetricErrorV1> {
        validate_profile(profile)?;
        if N == 0 {
            return Err(MetricErrorV1::ZeroDimension);
        }
        if matrix.iter().flatten().any(|value| !value.is_finite()) {
            return Err(MetricErrorV1::NonFiniteMatrix);
        }

        let scale = matrix_inf(&matrix);
        if !scale.is_finite() || scale == 0.0 {
            return Err(MetricErrorV1::NonFiniteIntermediate);
        }

        let mut symmetric = [[0.0; N]; N];
        let mut maximum_relative_symmetry_defect = 0.0_f64;
        for row in 0..N {
            for column in row..N {
                let left = matrix[row][column];
                let right = matrix[column][row];
                let defect = (left - right).abs() / scale;
                if !defect.is_finite() {
                    return Err(MetricErrorV1::NonFiniteIntermediate);
                }
                maximum_relative_symmetry_defect = maximum_relative_symmetry_defect.max(defect);
                if defect > profile.relative_symmetry_tolerance {
                    return Err(MetricErrorV1::NonSymmetricMetric { row, column });
                }
                // Stable midpoint, then mirror exactly so the admitted metric
                // has a single symmetric representation for all downstream use.
                let mean = left + (right - left) * 0.5;
                if !mean.is_finite() {
                    return Err(MetricErrorV1::NonFiniteIntermediate);
                }
                symmetric[row][column] = mean;
                symmetric[column][row] = mean;
            }
        }

        let threshold = profile.relative_pivot_tolerance * scale;
        if !threshold.is_finite() {
            return Err(MetricErrorV1::NonFiniteIntermediate);
        }
        let mut cholesky = [[0.0; N]; N];
        let mut minimum_cholesky_pivot = f64::INFINITY;

        for row in 0..N {
            for column in 0..=row {
                let mut value = symmetric[row][column];
                for (left, right) in cholesky[row]
                    .iter()
                    .zip(cholesky[column].iter())
                    .take(column)
                {
                    value -= left * right;
                }
                if !value.is_finite() {
                    return Err(MetricErrorV1::NonFiniteIntermediate);
                }
                if row == column {
                    if value <= threshold {
                        return Err(MetricErrorV1::NotPositiveDefinite { pivot: row });
                    }
                    let diagonal = value.sqrt();
                    if !diagonal.is_finite() || diagonal == 0.0 {
                        return Err(MetricErrorV1::NotPositiveDefinite { pivot: row });
                    }
                    cholesky[row][column] = diagonal;
                    minimum_cholesky_pivot = minimum_cholesky_pivot.min(value);
                } else {
                    cholesky[row][column] = value / cholesky[column][column];
                    if !cholesky[row][column].is_finite() {
                        return Err(MetricErrorV1::NonFiniteIntermediate);
                    }
                }
            }
        }

        Ok(Self {
            matrix: symmetric,
            cholesky,
            receipt: MetricAdmissionReceiptV1 {
                maximum_relative_symmetry_defect,
                minimum_cholesky_pivot,
                matrix_inf_norm: scale,
            },
            _frame: PhantomData,
        })
    }

    pub fn matrix(&self) -> &Matrix<N> {
        &self.matrix
    }

    pub fn admission_receipt(&self) -> MetricAdmissionReceiptV1 {
        self.receipt
    }

    /// ||v||_M = sqrt(v^T M v), evaluated as ||L^T v||_2 for M = L L^T.
    pub fn tangent_norm(&self, vector: &Tangent<F, N>) -> Result<f64, MetricErrorV1> {
        let mut transformed = [0.0; N];
        for (column, output) in transformed.iter_mut().enumerate() {
            let mut value = 0.0;
            for row in column..N {
                value += self.cholesky[row][column] * vector.components[row];
            }
            if !value.is_finite() {
                return Err(MetricErrorV1::NonFiniteIntermediate);
            }
            *output = value;
        }
        let norm = vector_euclidean(&transformed);
        if norm.is_finite() {
            Ok(norm)
        } else {
            Err(MetricErrorV1::NonFiniteIntermediate)
        }
    }

    /// Apply the inverse square-root factor in Cholesky coordinates:
    /// z = L^-1 alpha, so ||z||_2 = ||alpha||_(M^-1).
    ///
    /// This whitening primitive is also used to construct Y = J L^-T
    /// and the Delassus metric W = Y Y^T without forming M^-1.
    pub fn whiten_cotangent(&self, covector: &Cotangent<F, N>) -> Result<Vector<N>, MetricErrorV1> {
        let mut whitened = [0.0; N];
        for row in 0..N {
            let mut value = covector.components[row];
            for (column, previous) in whitened.iter().take(row).enumerate() {
                value -= self.cholesky[row][column] * previous;
            }
            whitened[row] = value / self.cholesky[row][row];
            if !whitened[row].is_finite() {
                return Err(MetricErrorV1::NonFiniteIntermediate);
            }
        }
        Ok(whitened)
    }

    /// ||alpha||_(M^-1) = sqrt(alpha^T M^-1 alpha), evaluated as ||L^-1 alpha||_2.
    pub fn cotangent_norm(&self, covector: &Cotangent<F, N>) -> Result<f64, MetricErrorV1> {
        let whitened = self.whiten_cotangent(covector)?;
        let norm = vector_euclidean(&whitened);
        if norm.is_finite() {
            Ok(norm)
        } else {
            Err(MetricErrorV1::NonFiniteIntermediate)
        }
    }

    /// Riesz lowering: v ↦ Mv.
    pub fn lower(&self, vector: &Tangent<F, N>) -> Result<Cotangent<F, N>, MetricErrorV1> {
        let components = apply(&self.matrix, &vector.components);
        Cotangent::new(components)
    }

    /// Riesz raising: alpha ↦ M^-1 alpha.
    pub fn raise(&self, covector: &Cotangent<F, N>) -> Result<Tangent<F, N>, MetricErrorV1> {
        let mut intermediate = [0.0; N];
        for row in 0..N {
            let mut value = covector.components[row];
            for (column, previous) in intermediate.iter().take(row).enumerate() {
                value -= self.cholesky[row][column] * previous;
            }
            intermediate[row] = value / self.cholesky[row][row];
            if !intermediate[row].is_finite() {
                return Err(MetricErrorV1::NonFiniteIntermediate);
            }
        }
        let mut components = [0.0; N];
        for row in (0..N).rev() {
            let mut value = intermediate[row];
            for (column, component) in components.iter().enumerate().skip(row + 1) {
                value -= self.cholesky[column][row] * component;
            }
            components[row] = value / self.cholesky[row][row];
            if !components[row].is_finite() {
                return Err(MetricErrorV1::NonFiniteIntermediate);
            }
        }
        Tangent::new(components)
    }
}

impl<From: CoordinateFrame, To: CoordinateFrame, const N: usize> Rebasis<From, To, N> {
    /// Construct from q = S y, where S is nonsingular.
    pub fn new(matrix: Matrix<N>, profile: MetricProfileV1) -> Result<Self, MetricErrorV1> {
        validate_profile(profile)?;
        let factor = factor(&matrix, profile.relative_pivot_tolerance)?;
        Ok(Self {
            matrix,
            factor,
            _frame_pair: PhantomData,
        })
    }

    pub fn matrix(&self) -> &Matrix<N> {
        &self.matrix
    }

    /// Infinity-norm condition estimate for the coordinate matrix S.
    ///
    /// This is a coordinate-representation diagnostic, not a physical
    /// coordinate-invariant condition number.
    pub fn condition_inf_estimate(&self) -> Result<f64, MetricErrorV1> {
        let mut inverse = [[0.0; N]; N];
        for column in 0..N {
            let mut unit = [0.0; N];
            unit[column] = 1.0;
            let inverse_column = self.factor.solve(&unit)?;
            for row in 0..N {
                inverse[row][column] = inverse_column[row];
            }
        }
        let estimate = matrix_inf(&self.matrix) * matrix_inf(&inverse);
        if estimate.is_finite() {
            Ok(estimate)
        } else {
            Err(MetricErrorV1::NonFiniteIntermediate)
        }
    }

    /// Tangent components transform contravariantly: v_y = S^-1 v_q.
    pub fn tangent_to_to(
        &self,
        vector_from: &Tangent<From, N>,
    ) -> Result<Tangent<To, N>, MetricErrorV1> {
        Tangent::new(self.factor.solve(&vector_from.components)?)
    }

    /// Inverse tangent map: v_q = S v_y.
    pub fn tangent_to_from(
        &self,
        vector_to: &Tangent<To, N>,
    ) -> Result<Tangent<From, N>, MetricErrorV1> {
        Tangent::new(apply(&self.matrix, &vector_to.components))
    }

    /// Covector components transform by pullback: alpha_y = S^T alpha_q.
    pub fn cotangent_to_to(
        &self,
        covector_from: &Cotangent<From, N>,
    ) -> Result<Cotangent<To, N>, MetricErrorV1> {
        Cotangent::new(apply(&transpose(&self.matrix), &covector_from.components))
    }

    /// Inverse covector map: alpha_q = S^-T alpha_y.
    pub fn cotangent_to_from(
        &self,
        covector_to: &Cotangent<To, N>,
        profile: MetricProfileV1,
    ) -> Result<Cotangent<From, N>, MetricErrorV1> {
        validate_profile(profile)?;
        let transpose_factor = factor(&transpose(&self.matrix), profile.relative_pivot_tolerance)?;
        Cotangent::new(transpose_factor.solve(&covector_to.components)?)
    }

    /// Metric congruence M_y = S^T M_q S, computed symmetrically.
    pub fn metric_to_to(
        &self,
        metric_from: &Metric<From, N>,
        profile: MetricProfileV1,
    ) -> Result<Metric<To, N>, MetricErrorV1> {
        validate_profile(profile)?;
        // Two dense matrix products keep the congruence at O(N^3):
        // first P = M_q S, then M_y = S^T P. Only the upper triangle of
        // the final symmetric metric is accumulated; it is mirrored once.
        let mut mass_times_basis = [[0.0; N]; N];
        for (row, output_row) in mass_times_basis.iter_mut().enumerate() {
            for (column, output) in output_row.iter_mut().enumerate() {
                let value = metric_from.matrix[row]
                    .iter()
                    .zip(self.matrix.iter())
                    .map(|(mass_value, basis_row)| mass_value * basis_row[column])
                    .sum::<f64>();
                if !value.is_finite() {
                    return Err(MetricErrorV1::NonFiniteIntermediate);
                }
                *output = value;
            }
        }

        let mut upper = [[0.0; N]; N];
        for (row, output_row) in upper.iter_mut().enumerate() {
            for (column, output) in output_row.iter_mut().enumerate().skip(row) {
                let value = self
                    .matrix
                    .iter()
                    .zip(mass_times_basis.iter())
                    .map(|(basis_row, product_row)| basis_row[row] * product_row[column])
                    .sum::<f64>();
                if !value.is_finite() {
                    return Err(MetricErrorV1::NonFiniteIntermediate);
                }
                *output = value;
            }
        }
        let transformed = std::array::from_fn(|row| {
            std::array::from_fn(|column| {
                if row <= column {
                    upper[row][column]
                } else {
                    upper[column][row]
                }
            })
        });
        Metric::admit(transformed, profile)
    }
}

/// Coordinate pairing alpha(v). It must be invariant under dual transformation.
pub fn pairing<F: CoordinateFrame, const N: usize>(
    covector: &Cotangent<F, N>,
    vector: &Tangent<F, N>,
) -> f64 {
    dot(&covector.components, &vector.components)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> MetricProfileV1 {
        MetricProfileV1::default()
    }

    fn cartesian_metric() -> Metric<CartesianQ, 3> {
        Metric::admit(
            [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
            profile(),
        )
        .unwrap()
    }

    fn dense_rebasis() -> Rebasis<CartesianQ, DenseY, 3> {
        // q = S y, a dense integer unit-triangular basis change (det S = 1).
        Rebasis::new(
            [[1.0, 2.0, 1.0], [0.0, 1.0, 3.0], [0.0, 0.0, 1.0]],
            profile(),
        )
        .unwrap()
    }

    fn close(left: f64, right: f64, tolerance: f64) {
        let scale = 1.0_f64.max(left.abs()).max(right.abs());
        assert!(
            (left - right).abs() <= tolerance * scale,
            "{left} != {right}"
        );
    }

    #[test]
    fn coordinate_condition_estimate_is_finite_and_exceeds_one_for_a_shear() {
        let rebasis = dense_rebasis();
        let estimate = rebasis.condition_inf_estimate().unwrap();
        assert!(estimate.is_finite());
        assert!(estimate > 1.0);
    }

    #[test]
    fn dense_rebasis_preserves_tangent_and_cotangent_metric_norms() {
        let metric_q = cartesian_metric();
        let rebasis = dense_rebasis();
        let metric_y = rebasis.metric_to_to(&metric_q, profile()).unwrap();

        let vector_q = Tangent::<CartesianQ, 3>::new([3.0, -1.0, 2.0]).unwrap();
        let vector_y = rebasis.tangent_to_to(&vector_q).unwrap();
        close(
            metric_q.tangent_norm(&vector_q).unwrap(),
            metric_y.tangent_norm(&vector_y).unwrap(),
            2.0e-13,
        );

        let covector_q = Cotangent::<CartesianQ, 3>::new([7.0, -2.0, 4.0]).unwrap();
        let covector_y = rebasis.cotangent_to_to(&covector_q).unwrap();
        close(
            metric_q.cotangent_norm(&covector_q).unwrap(),
            metric_y.cotangent_norm(&covector_y).unwrap(),
            2.0e-13,
        );
    }

    #[test]
    fn primal_dual_pairing_is_invariant_and_round_trips() {
        let rebasis = dense_rebasis();
        let vector_q = Tangent::<CartesianQ, 3>::new([3.0, -1.0, 2.0]).unwrap();
        let covector_q = Cotangent::<CartesianQ, 3>::new([7.0, -2.0, 4.0]).unwrap();

        let vector_y = rebasis.tangent_to_to(&vector_q).unwrap();
        let covector_y = rebasis.cotangent_to_to(&covector_q).unwrap();
        close(
            pairing(&covector_q, &vector_q),
            pairing(&covector_y, &vector_y),
            2.0e-13,
        );

        let vector_q_roundtrip = rebasis.tangent_to_from(&vector_y).unwrap();
        let covector_q_roundtrip = rebasis.cotangent_to_from(&covector_y, profile()).unwrap();
        for index in 0..3 {
            close(
                vector_q.components[index],
                vector_q_roundtrip.components[index],
                2.0e-13,
            );
            close(
                covector_q.components[index],
                covector_q_roundtrip.components[index],
                2.0e-13,
            );
        }
    }

    #[test]
    fn mass_riesz_map_identifies_tangent_and_dual_residual_norms() {
        let metric_q = cartesian_metric();
        let rebasis = dense_rebasis();
        let metric_y = rebasis.metric_to_to(&metric_q, profile()).unwrap();

        let acceleration_q = Tangent::<CartesianQ, 3>::new([0.25, -2.0, 1.5]).unwrap();
        let acceleration_y = rebasis.tangent_to_to(&acceleration_q).unwrap();

        let residual_q = metric_q.lower(&acceleration_q).unwrap();
        let residual_y = metric_y.lower(&acceleration_y).unwrap();

        close(
            metric_q.tangent_norm(&acceleration_q).unwrap(),
            metric_q.cotangent_norm(&residual_q).unwrap(),
            2.0e-13,
        );
        close(
            metric_y.tangent_norm(&acceleration_y).unwrap(),
            metric_y.cotangent_norm(&residual_y).unwrap(),
            2.0e-13,
        );
        close(
            metric_q.cotangent_norm(&residual_q).unwrap(),
            metric_y.cotangent_norm(&residual_y).unwrap(),
            2.0e-13,
        );
    }

    #[test]
    fn dense_rebasing_changes_raw_component_norms_but_not_metric_norms() {
        let metric_q = cartesian_metric();
        let rebasis = dense_rebasis();
        let metric_y = rebasis.metric_to_to(&metric_q, profile()).unwrap();
        let vector_q = Tangent::<CartesianQ, 3>::new([1.0, 2.0, -1.0]).unwrap();
        let vector_y = rebasis.tangent_to_to(&vector_q).unwrap();

        let euclidean_q = vector_euclidean(vector_q.components());
        let euclidean_y = vector_euclidean(vector_y.components());
        let infinity_q = vector_inf(vector_q.components());
        let infinity_y = vector_inf(vector_y.components());

        assert!((euclidean_q - euclidean_y).abs() > 1.0e-3);
        assert!((infinity_q - infinity_y).abs() > 1.0e-3);
        close(
            metric_q.tangent_norm(&vector_q).unwrap(),
            metric_y.tangent_norm(&vector_y).unwrap(),
            2.0e-13,
        );
    }

    #[test]
    fn metric_norms_are_invariant_over_a_deterministic_shear_family() {
        let metric_q = cartesian_metric();
        let vector_q = Tangent::<CartesianQ, 3>::new([0.5, -1.25, 2.0]).unwrap();
        let covector_q = Cotangent::<CartesianQ, 3>::new([2.0, -3.0, 0.75]).unwrap();
        let tangent_reference = metric_q.tangent_norm(&vector_q).unwrap();
        let cotangent_reference = metric_q.cotangent_norm(&covector_q).unwrap();

        for shear in -8..=8 {
            let s = [
                [1.0, shear as f64, 0.0],
                [0.0, 1.0, shear as f64],
                [0.0, 0.0, 1.0],
            ];
            let rebasis = Rebasis::<CartesianQ, DenseY, 3>::new(s, profile()).unwrap();
            let metric_y = rebasis.metric_to_to(&metric_q, profile()).unwrap();
            let vector_y = rebasis.tangent_to_to(&vector_q).unwrap();
            let covector_y = rebasis.cotangent_to_to(&covector_q).unwrap();

            close(
                tangent_reference,
                metric_y.tangent_norm(&vector_y).unwrap(),
                1.0e-11,
            );
            close(
                cotangent_reference,
                metric_y.cotangent_norm(&covector_y).unwrap(),
                1.0e-11,
            );
            close(
                pairing(&covector_q, &vector_q),
                pairing(&covector_y, &vector_y),
                1.0e-11,
            );
        }
    }

    #[test]
    fn wrong_tensor_variance_is_detected_by_pairing_and_norm_checks() {
        let metric_q = cartesian_metric();
        let rebasis = dense_rebasis();
        let metric_y = rebasis.metric_to_to(&metric_q, profile()).unwrap();
        let vector_q = Tangent::<CartesianQ, 3>::new([1.0, 2.0, -1.0]).unwrap();
        let covector_q = Cotangent::<CartesianQ, 3>::new([2.0, -1.0, 3.0]).unwrap();

        let correct_vector_y = rebasis.tangent_to_to(&vector_q).unwrap();
        let correct_covector_y = rebasis.cotangent_to_to(&covector_q).unwrap();

        // Deliberate wrong laws: apply S^T to a tangent and S^-1 to a covector.
        let wrong_vector_y = apply(&transpose(rebasis.matrix()), vector_q.components());
        let wrong_covector_y = rebasis.factor.solve(covector_q.components()).unwrap();

        let correct_pairing = pairing(&correct_covector_y, &correct_vector_y);
        let wrong_pairing = dot(&wrong_covector_y, &wrong_vector_y);

        // This pair of mutually wrong maps still preserves the scalar pairing.
        // Pairing-only tests therefore cannot distinguish the tensor variance.
        close(correct_pairing, wrong_pairing, 2.0e-13);

        let correct_tangent_norm = metric_y.tangent_norm(&correct_vector_y).unwrap();
        let wrong_tangent = Tangent::<DenseY, 3>::new(wrong_vector_y).unwrap();
        let wrong_tangent_norm = metric_y.tangent_norm(&wrong_tangent).unwrap();
        assert!((correct_tangent_norm - wrong_tangent_norm).abs() > 1.0e-3);

        let wrong_covector = Cotangent::<DenseY, 3>::new(wrong_covector_y).unwrap();
        let correct_covector_norm = metric_y.cotangent_norm(&correct_covector_y).unwrap();
        let wrong_covector_norm = metric_y.cotangent_norm(&wrong_covector).unwrap();
        assert!((correct_covector_norm - wrong_covector_norm).abs() > 1.0e-3);
    }

    #[test]
    fn singular_basis_and_non_positive_metric_fail_closed() {
        assert!(matches!(
            Rebasis::<CartesianQ, DenseY, 2>::new([[1.0, 2.0], [2.0, 4.0]], profile()),
            Err(MetricErrorV1::SingularCoordinateTransform { .. })
        ));

        assert!(matches!(
            Metric::<CartesianQ, 2>::admit([[1.0, 2.0], [2.0, 1.0]], profile()),
            Err(MetricErrorV1::NotPositiveDefinite { .. })
        ));
    }

    #[test]
    fn asymmetric_and_nonfinite_metrics_are_rejected() {
        assert!(matches!(
            Metric::<CartesianQ, 2>::admit([[2.0, 1.0], [1.1, 3.0]], profile()),
            Err(MetricErrorV1::NonSymmetricMetric { .. })
        ));
        assert_eq!(
            Metric::<CartesianQ, 2>::admit([[f64::NAN, 0.0], [0.0, 1.0]], profile()),
            Err(MetricErrorV1::NonFiniteMatrix)
        );
    }

    #[test]
    fn wrong_metric_congruence_is_detected() {
        let metric_q = cartesian_metric();
        let rebasis = dense_rebasis();
        let correct = rebasis.metric_to_to(&metric_q, profile()).unwrap();
        let wrong = multiply(
            &multiply(&transpose(rebasis.matrix()), metric_q.matrix()),
            &transpose(rebasis.matrix()),
        );
        let difference = (0..3)
            .flat_map(|row| (0..3).map(move |column| (row, column)))
            .map(|(row, column)| (correct.matrix()[row][column] - wrong[row][column]).abs())
            .fold(0.0_f64, f64::max);
        assert!(difference > 1.0e-3);
    }

    #[test]
    fn six_dof_metric_congruence_matches_independent_two_stage_product() {
        const N: usize = 6;
        let mass = [
            [1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            [0.0, 2.0, 0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 3.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 4.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0, 5.0, 0.0],
            [0.0, 0.0, 0.0, 0.0, 0.0, 6.0],
        ];
        let basis = [
            [1.0, 2.0, -1.0, 0.0, 1.0, 0.0],
            [0.0, 1.0, 3.0, -1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0, 2.0, -1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0, 3.0, -1.0],
            [0.0, 0.0, 0.0, 0.0, 1.0, 2.0],
            [0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
        ];
        let metric_q = Metric::<CartesianQ, N>::admit(mass, profile()).unwrap();
        let rebasis = Rebasis::<CartesianQ, DenseY, N>::new(basis, profile()).unwrap();
        let metric_y = rebasis.metric_to_to(&metric_q, profile()).unwrap();
        let expected = multiply(&multiply(&transpose(&basis), &mass), &basis);

        for row in 0..N {
            for column in 0..N {
                close(metric_y.matrix()[row][column], expected[row][column], 2.0e-13);
            }
        }
    }

    #[test]
    fn admitted_roundoff_asymmetry_is_recorded_not_silently_hidden() {
        let admitted = Metric::<CartesianQ, 2>::admit(
            [[2.0, 1.0], [1.0 + 1.0e-15, 3.0]],
            MetricProfileV1 {
                relative_symmetry_tolerance: 1.0e-12,
                ..profile()
            },
        )
        .unwrap();
        let receipt = admitted.admission_receipt();
        assert!(receipt.maximum_relative_symmetry_defect > 0.0);
        assert!(receipt.maximum_relative_symmetry_defect < 1.0e-12);
        assert_eq!(admitted.matrix()[0][1], admitted.matrix()[1][0]);
    }
}

pub use rank_deficient_constraint::{
    ConstraintQuotientAdmissionReceiptV1, ConstraintQuotientErrorV1,
    ConstraintQuotientProfileV1, ConstraintQuotientSolveReceiptV1,
    ConstraintResidualStatusV1, RankDeficientDelassusQuotientV1,
};
