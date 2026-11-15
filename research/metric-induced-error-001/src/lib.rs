// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Metric-induced norms for frame-typed tangent and cotangent quantities.
//!
//! This is a bounded dense reference, not a formal floating-point proof.
//! Frame markers prevent accidental mixing of tangent/cotangent values from
//! different coordinate representations at compile time.

use std::marker::PhantomData;

pub type Matrix<const N: usize> = [[f64; N]; N];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CartesianFrame;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DenseGeneralizedFrame;

#[derive(Debug, Clone, PartialEq)]
pub struct Tangent<F, const N: usize> {
    components: [f64; N],
    frame: PhantomData<fn() -> F>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Covector<F, const N: usize> {
    components: [f64; N],
    frame: PhantomData<fn() -> F>,
}

impl<F, const N: usize> Tangent<F, N> {
    pub fn new(components: [f64; N]) -> Result<Self, MetricErrorV1> {
        if components.iter().any(|value| !value.is_finite()) {
            return Err(MetricErrorV1::NonFiniteVector);
        }
        Ok(Self {
            components,
            frame: PhantomData,
        })
    }

    pub fn components(&self) -> &[f64; N] {
        &self.components
    }
}

impl<F, const N: usize> Covector<F, N> {
    pub fn new(components: [f64; N]) -> Result<Self, MetricErrorV1> {
        if components.iter().any(|value| !value.is_finite()) {
            return Err(MetricErrorV1::NonFiniteVector);
        }
        Ok(Self {
            components,
            frame: PhantomData,
        })
    }

    pub fn components(&self) -> &[f64; N] {
        &self.components
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricProfileV1 {
    pub relative_symmetry_tolerance: f64,
    pub relative_pivot_tolerance: f64,
}

impl Default for MetricProfileV1 {
    fn default() -> Self {
        Self {
            relative_symmetry_tolerance: 128.0 * f64::EPSILON,
            relative_pivot_tolerance: 128.0 * f64::EPSILON,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetricErrorV1 {
    InvalidProfile,
    ZeroDimension,
    NonFiniteMetric,
    NonFiniteVector,
    NonFiniteArithmetic,
    MetricNotSymmetric,
    MetricNotPositiveDefinite,
    SingularCoordinateChange,
    CoordinateChangeNotFinite,
    NormNotPositive,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Metric<F, const N: usize> {
    matrix: Matrix<N>,
    admitted_symmetry_defect: f64,
    frame: PhantomData<fn() -> F>,
}

impl<F, const N: usize> Metric<F, N> {
    pub fn new(matrix: Matrix<N>, profile: MetricProfileV1) -> Result<Self, MetricErrorV1> {
        validate_profile(profile)?;
        if N == 0 {
            return Err(MetricErrorV1::ZeroDimension);
        }
        if matrix.iter().flatten().any(|value| !value.is_finite()) {
            return Err(MetricErrorV1::NonFiniteMetric);
        }

        let scale = matrix_inf(&matrix);
        if !scale.is_finite() {
            return Err(MetricErrorV1::NonFiniteArithmetic);
        }
        if scale == 0.0 {
            return Err(MetricErrorV1::MetricNotPositiveDefinite);
        }

        let mut effective = matrix;
        let mut defect = 0.0_f64;
        for row in 0..N {
            for column in (row + 1)..N {
                defect = defect.max((matrix[row][column] - matrix[column][row]).abs() / scale);
                let average = 0.5 * matrix[row][column] + 0.5 * matrix[column][row];
                effective[row][column] = average;
                effective[column][row] = average;
            }
        }
        if defect > profile.relative_symmetry_tolerance {
            return Err(MetricErrorV1::MetricNotSymmetric);
        }

        cholesky(&effective, profile.relative_pivot_tolerance)?;
        Ok(Self {
            matrix: effective,
            admitted_symmetry_defect: defect,
            frame: PhantomData,
        })
    }

    pub fn matrix(&self) -> &Matrix<N> {
        &self.matrix
    }

    pub fn admitted_symmetry_defect(&self) -> f64 {
        self.admitted_symmetry_defect
    }

    pub fn tangent_norm(
        &self,
        tangent: &Tangent<F, N>,
        profile: MetricProfileV1,
    ) -> Result<f64, MetricErrorV1> {
        validate_profile(profile)?;
        let applied = apply(&self.matrix, &tangent.components);
        let squared = dot(&tangent.components, &applied);
        let scale = tangent
            .components
            .iter()
            .zip(applied.iter())
            .map(|(left, right)| (left * right).abs())
            .sum::<f64>();
        checked_sqrt(squared, scale, profile.relative_pivot_tolerance)
    }

    pub fn cotangent_norm(
        &self,
        covector: &Covector<F, N>,
        profile: MetricProfileV1,
    ) -> Result<f64, MetricErrorV1> {
        validate_profile(profile)?;
        let factor = cholesky(&self.matrix, profile.relative_pivot_tolerance)?;
        let raised = solve_cholesky(&factor, &covector.components);
        let squared = dot(&covector.components, &raised);
        let scale = covector
            .components
            .iter()
            .zip(raised.iter())
            .map(|(left, right)| (left * right).abs())
            .sum::<f64>();
        checked_sqrt(squared, scale, profile.relative_pivot_tolerance)
    }

    /// The Riesz map M: T -> T* maps a tangent vector to its metric covector.
    pub fn lower(&self, tangent: &Tangent<F, N>) -> Result<Covector<F, N>, MetricErrorV1> {
        let components = apply(&self.matrix, &tangent.components);
        Covector::new(components)
    }

    /// The inverse Riesz map M^-1: T* -> T raises a covector.
    pub fn raise(
        &self,
        covector: &Covector<F, N>,
        profile: MetricProfileV1,
    ) -> Result<Tangent<F, N>, MetricErrorV1> {
        validate_profile(profile)?;
        let factor = cholesky(&self.matrix, profile.relative_pivot_tolerance)?;
        Tangent::new(solve_cholesky(&factor, &covector.components))
    }

    pub fn pairing(
        &self,
        covector: &Covector<F, N>,
        tangent: &Tangent<F, N>,
    ) -> Result<f64, MetricErrorV1> {
        let value = dot(&covector.components, &tangent.components);
        if value.is_finite() {
            Ok(value)
        } else {
            Err(MetricErrorV1::NonFiniteArithmetic)
        }
    }
}

/// A re-basing stores S for q_from = S y_to.
///
/// Tangents push forward by S; covectors pull back by S^T; metrics pull back
/// by S^T M S. The inverse operations use S^-1 and S^-T.
#[derive(Debug, Clone, PartialEq)]
pub struct CoordinateRebasis<From, To, const N: usize> {
    to_from: Matrix<N>,
    from_to: Matrix<N>,
    condition_inf_estimate: f64,
    _frames: PhantomData<fn() -> (From, To)>,
}

impl<From, To, const N: usize> CoordinateRebasis<From, To, N> {
    pub fn new(to_from: Matrix<N>, profile: MetricProfileV1) -> Result<Self, MetricErrorV1> {
        validate_profile(profile)?;
        if N == 0 {
            return Err(MetricErrorV1::ZeroDimension);
        }
        if to_from.iter().flatten().any(|value| !value.is_finite()) {
            return Err(MetricErrorV1::CoordinateChangeNotFinite);
        }
        let from_to = inverse(&to_from, profile.relative_pivot_tolerance)?;
        let condition_inf_estimate = matrix_inf(&to_from) * matrix_inf(&from_to);
        if !condition_inf_estimate.is_finite() {
            return Err(MetricErrorV1::NonFiniteArithmetic);
        }
        Ok(Self {
            to_from,
            from_to,
            condition_inf_estimate,
            _frames: PhantomData,
        })
    }

    pub fn matrix(&self) -> &Matrix<N> {
        &self.to_from
    }

    /// Coordinate diagnostic only; not a physical uncertainty measure.
    pub fn condition_inf_estimate(&self) -> f64 {
        self.condition_inf_estimate
    }

    pub fn push_tangent_to_from(
        &self,
        tangent_to: &Tangent<To, N>,
    ) -> Result<Tangent<From, N>, MetricErrorV1> {
        Tangent::new(apply(&self.to_from, &tangent_to.components))
    }

    pub fn pull_tangent_to_to(
        &self,
        tangent_from: &Tangent<From, N>,
    ) -> Result<Tangent<To, N>, MetricErrorV1> {
        Tangent::new(apply(&self.from_to, &tangent_from.components))
    }

    pub fn pull_covector_to_to(
        &self,
        covector_from: &Covector<From, N>,
    ) -> Result<Covector<To, N>, MetricErrorV1> {
        Covector::new(apply(&transpose(&self.to_from), &covector_from.components))
    }

    pub fn push_covector_to_from(
        &self,
        covector_to: &Covector<To, N>,
    ) -> Result<Covector<From, N>, MetricErrorV1> {
        Covector::new(apply(&transpose(&self.from_to), &covector_to.components))
    }

    pub fn pull_metric_to_to(
        &self,
        metric_from: &Metric<From, N>,
        profile: MetricProfileV1,
    ) -> Result<Metric<To, N>, MetricErrorV1> {
        let transformed = multiply(
            &multiply(&transpose(&self.to_from), metric_from.matrix()),
            &self.to_from,
        );
        Metric::new(transformed, profile)
    }
}

/// Coordinate infinity norm; retained as a representation-dependent diagnostic.
pub fn coordinate_inf_tangent<F, const N: usize>(value: &Tangent<F, N>) -> f64 {
    value
        .components
        .iter()
        .map(|entry| entry.abs())
        .fold(0.0, f64::max)
}

pub fn coordinate_inf_covector<F, const N: usize>(value: &Covector<F, N>) -> f64 {
    value
        .components
        .iter()
        .map(|entry| entry.abs())
        .fold(0.0, f64::max)
}

/// A primal/cotangent residual for M a = Q.
///
/// Its dual-metric norm equals the metric norm of the acceleration error in
/// exact arithmetic: ||M(a_hat-a)||_(M^-1) = ||a_hat-a||_M.
pub fn riesz_residual<F, const N: usize>(
    metric: &Metric<F, N>,
    approximate_acceleration: &Tangent<F, N>,
    force: &Covector<F, N>,
) -> Result<Covector<F, N>, MetricErrorV1> {
    let applied = metric.lower(approximate_acceleration)?;
    let residual = std::array::from_fn(|index| applied.components[index] - force.components[index]);
    Covector::new(residual)
}

fn validate_profile(profile: MetricProfileV1) -> Result<(), MetricErrorV1> {
    if !profile.relative_symmetry_tolerance.is_finite()
        || profile.relative_symmetry_tolerance < 0.0
        || !profile.relative_pivot_tolerance.is_finite()
        || profile.relative_pivot_tolerance <= 0.0
    {
        return Err(MetricErrorV1::InvalidProfile);
    }
    Ok(())
}

fn checked_sqrt(value: f64, scale: f64, relative_tolerance: f64) -> Result<f64, MetricErrorV1> {
    if !value.is_finite() || !scale.is_finite() {
        return Err(MetricErrorV1::NonFiniteArithmetic);
    }
    let tolerance = relative_tolerance * scale;
    if !tolerance.is_finite() {
        return Err(MetricErrorV1::NonFiniteArithmetic);
    }
    if value < -tolerance {
        return Err(MetricErrorV1::NormNotPositive);
    }
    let result = value.max(0.0).sqrt();
    if result.is_finite() {
        Ok(result)
    } else {
        Err(MetricErrorV1::NonFiniteArithmetic)
    }
}

fn dot<const N: usize>(left: &[f64; N], right: &[f64; N]) -> f64 {
    left.iter().zip(right.iter()).map(|(a, b)| a * b).sum()
}

fn apply<const N: usize>(matrix: &Matrix<N>, vector: &[f64; N]) -> [f64; N] {
    std::array::from_fn(|row| {
        (0..N)
            .map(|column| matrix[row][column] * vector[column])
            .sum()
    })
}

fn matrix_inf<const N: usize>(matrix: &Matrix<N>) -> f64 {
    matrix
        .iter()
        .map(|row| row.iter().map(|value| value.abs()).sum::<f64>())
        .fold(0.0, f64::max)
}

fn transpose<const N: usize>(matrix: &Matrix<N>) -> Matrix<N> {
    std::array::from_fn(|row| std::array::from_fn(|column| matrix[column][row]))
}

fn multiply<const N: usize>(left: &Matrix<N>, right: &Matrix<N>) -> Matrix<N> {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            (0..N)
                .map(|inner| left[row][inner] * right[inner][column])
                .sum()
        })
    })
}

fn cholesky<const N: usize>(
    matrix: &Matrix<N>,
    relative_pivot_tolerance: f64,
) -> Result<Matrix<N>, MetricErrorV1> {
    let scale = matrix_inf(matrix);
    if !scale.is_finite() {
        return Err(MetricErrorV1::NonFiniteArithmetic);
    }
    let threshold = relative_pivot_tolerance * scale;
    if !threshold.is_finite() {
        return Err(MetricErrorV1::NonFiniteArithmetic);
    }
    let mut lower = [[0.0; N]; N];
    for row in 0..N {
        for column in 0..=row {
            let correction = (0..column)
                .map(|inner| lower[row][inner] * lower[column][inner])
                .sum::<f64>();
            let value = matrix[row][column] - correction;
            if !value.is_finite() {
                return Err(MetricErrorV1::NonFiniteArithmetic);
            }
            if row == column {
                if value <= threshold {
                    return Err(MetricErrorV1::MetricNotPositiveDefinite);
                }
                lower[row][column] = value.sqrt();
            } else {
                lower[row][column] = value / lower[column][column];
            }
        }
    }
    Ok(lower)
}

fn solve_cholesky<const N: usize>(lower: &Matrix<N>, rhs: &[f64; N]) -> [f64; N] {
    let mut intermediate = *rhs;
    for row in 0..N {
        for column in 0..row {
            intermediate[row] -= lower[row][column] * intermediate[column];
        }
        intermediate[row] /= lower[row][row];
    }
    let mut solution = intermediate;
    for row in (0..N).rev() {
        for column in (row + 1)..N {
            solution[row] -= lower[column][row] * solution[column];
        }
        solution[row] /= lower[row][row];
    }
    solution
}

fn inverse<const N: usize>(
    matrix: &Matrix<N>,
    relative_pivot_tolerance: f64,
) -> Result<Matrix<N>, MetricErrorV1> {
    let scale = matrix_inf(matrix);
    if !scale.is_finite() || scale == 0.0 {
        return Err(MetricErrorV1::SingularCoordinateChange);
    }
    let threshold = relative_pivot_tolerance * scale;
    if !threshold.is_finite() {
        return Err(MetricErrorV1::NonFiniteArithmetic);
    }
    let mut left = *matrix;
    let mut right = std::array::from_fn(|row| {
        std::array::from_fn(|column| if row == column { 1.0 } else { 0.0 })
    });
    for pivot_column in 0..N {
        let pivot_row = (pivot_column..N)
            .max_by(|a, b| {
                left[*a][pivot_column]
                    .abs()
                    .total_cmp(&left[*b][pivot_column].abs())
            })
            .ok_or(MetricErrorV1::ZeroDimension)?;
        let pivot = left[pivot_row][pivot_column].abs();
        if !pivot.is_finite() || pivot <= threshold {
            return Err(MetricErrorV1::SingularCoordinateChange);
        }
        if pivot_row != pivot_column {
            left.swap(pivot_row, pivot_column);
            right.swap(pivot_row, pivot_column);
        }
        let divisor = left[pivot_column][pivot_column];
        for column in 0..N {
            left[pivot_column][column] /= divisor;
            right[pivot_column][column] /= divisor;
        }
        for row in 0..N {
            if row == pivot_column {
                continue;
            }
            let multiplier = left[row][pivot_column];
            for column in 0..N {
                left[row][column] -= multiplier * left[pivot_column][column];
                right[row][column] -= multiplier * right[pivot_column][column];
            }
        }
    }
    if right.iter().flatten().any(|value| !value.is_finite()) {
        return Err(MetricErrorV1::NonFiniteArithmetic);
    }
    Ok(right)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> MetricProfileV1 {
        MetricProfileV1::default()
    }

    fn cartesian_metric() -> Metric<CartesianFrame, 3> {
        Metric::new(
            [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
            profile(),
        )
        .unwrap()
    }

    fn rebasis() -> CoordinateRebasis<CartesianFrame, DenseGeneralizedFrame, 3> {
        CoordinateRebasis::new(
            [[1.0, 2.0, 1.0], [0.0, 1.0, 1.0], [0.0, 0.0, 1.0]],
            profile(),
        )
        .unwrap()
    }

    #[test]
    fn tangent_norm_is_invariant_under_dense_nonorthogonal_rebasis() {
        let metric_q = cartesian_metric();
        let basis = rebasis();
        let tangent_y = Tangent::<DenseGeneralizedFrame, 3>::new([0.5, -1.0, 2.0]).unwrap();
        let tangent_q = basis.push_tangent_to_from(&tangent_y).unwrap();
        let metric_y = basis.pull_metric_to_to(&metric_q, profile()).unwrap();
        let norm_q = metric_q.tangent_norm(&tangent_q, profile()).unwrap();
        let norm_y = metric_y.tangent_norm(&tangent_y, profile()).unwrap();
        assert!((norm_q - norm_y).abs() < 1.0e-12);
    }

    #[test]
    fn cotangent_dual_norm_is_invariant_under_dense_nonorthogonal_rebasis() {
        let metric_q = cartesian_metric();
        let basis = rebasis();
        let force_q = Covector::<CartesianFrame, 3>::new([3.0, -2.0, 4.0]).unwrap();
        let force_y = basis.pull_covector_to_to(&force_q).unwrap();
        let metric_y = basis.pull_metric_to_to(&metric_q, profile()).unwrap();
        let norm_q = metric_q.cotangent_norm(&force_q, profile()).unwrap();
        let norm_y = metric_y.cotangent_norm(&force_y, profile()).unwrap();
        assert!((norm_q - norm_y).abs() < 1.0e-11);
    }

    #[test]
    fn canonical_pairing_is_invariant_under_primal_dual_transformations() {
        let basis = rebasis();
        let tangent_y = Tangent::<DenseGeneralizedFrame, 3>::new([0.5, -1.0, 2.0]).unwrap();
        let tangent_q = basis.push_tangent_to_from(&tangent_y).unwrap();
        let force_q = Covector::<CartesianFrame, 3>::new([3.0, -2.0, 4.0]).unwrap();
        let force_y = basis.pull_covector_to_to(&force_q).unwrap();
        let pairing_q = dot(tangent_q.components(), force_q.components());
        let pairing_y = dot(tangent_y.components(), force_y.components());
        assert!((pairing_q - pairing_y).abs() < 1.0e-12);
    }

    #[test]
    fn riesz_residual_dual_norm_matches_physical_acceleration_error() {
        let metric = cartesian_metric();
        let exact = Tangent::<CartesianFrame, 3>::new([0.5, -2.0 / 3.0, 0.8]).unwrap();
        let force = metric.lower(&exact).unwrap();
        let approximate =
            Tangent::<CartesianFrame, 3>::new([0.5 + 1.0e-5, -2.0 / 3.0 - 2.0e-5, 0.8 + 3.0e-5])
                .unwrap();
        let error = Tangent::<CartesianFrame, 3>::new(std::array::from_fn(|i| {
            approximate.components()[i] - exact.components()[i]
        }))
        .unwrap();
        let residual = riesz_residual(&metric, &approximate, &force).unwrap();
        let residual_norm = metric.cotangent_norm(&residual, profile()).unwrap();
        let error_norm = metric.tangent_norm(&error, profile()).unwrap();
        assert!((residual_norm - error_norm).abs() < 1.0e-12);
    }

    #[test]
    fn riesz_residual_dual_norm_is_invariant_after_dense_rebasis() {
        let metric_q = cartesian_metric();
        let basis = rebasis();
        let exact_q = Tangent::<CartesianFrame, 3>::new([0.5, -2.0 / 3.0, 0.8]).unwrap();
        let approximate_q =
            Tangent::<CartesianFrame, 3>::new([0.50001, -2.0 / 3.00002, 0.80003]).unwrap();
        let force_q = metric_q.lower(&exact_q).unwrap();

        let residual_q = riesz_residual(&metric_q, &approximate_q, &force_q).unwrap();
        let error_q = Tangent::<CartesianFrame, 3>::new(std::array::from_fn(|i| {
            approximate_q.components()[i] - exact_q.components()[i]
        }))
        .unwrap();
        let residual_norm_q = metric_q.cotangent_norm(&residual_q, profile()).unwrap();
        let error_norm_q = metric_q.tangent_norm(&error_q, profile()).unwrap();
        assert!((residual_norm_q - error_norm_q).abs() < 1.0e-12);

        let exact_y = basis.pull_tangent_to_to(&exact_q).unwrap();
        let approximate_y = basis.pull_tangent_to_to(&approximate_q).unwrap();
        let force_y = basis.pull_covector_to_to(&force_q).unwrap();
        let metric_y = basis.pull_metric_to_to(&metric_q, profile()).unwrap();
        let residual_y = riesz_residual(&metric_y, &approximate_y, &force_y).unwrap();
        let error_y = Tangent::<DenseGeneralizedFrame, 3>::new(std::array::from_fn(|i| {
            approximate_y.components()[i] - exact_y.components()[i]
        }))
        .unwrap();
        let residual_norm_y = metric_y.cotangent_norm(&residual_y, profile()).unwrap();
        let error_norm_y = metric_y.tangent_norm(&error_y, profile()).unwrap();
        assert!((residual_norm_y - error_norm_y).abs() < 1.0e-11);
        assert!((residual_norm_q - residual_norm_y).abs() < 1.0e-11);
        assert!(basis.condition_inf_estimate() > 1.0);
    }

    #[test]
    fn wrong_metric_congruence_is_detected() {
        let metric_q = cartesian_metric();
        let basis = rebasis();
        let correct = basis.pull_metric_to_to(&metric_q, profile()).unwrap();
        let wrong = multiply(
            &multiply(&transpose(basis.matrix()), metric_q.matrix()),
            &transpose(basis.matrix()),
        );
        assert_eq!(
            Metric::<DenseGeneralizedFrame, 3>::new(wrong, profile()).unwrap_err(),
            MetricErrorV1::MetricNotSymmetric
        );
        assert!(
            correct
                .matrix()
                .iter()
                .flatten()
                .all(|value| value.is_finite())
        );
    }

    #[test]
    fn frame_inverse_maps_preserve_tangent_and_covector_types() {
        let basis = rebasis();
        let tangent_q = Tangent::<CartesianFrame, 3>::new([1.0, 2.0, 3.0]).unwrap();
        let tangent_y = basis.pull_tangent_to_to(&tangent_q).unwrap();
        let tangent_roundtrip = basis.push_tangent_to_from(&tangent_y).unwrap();
        for i in 0..3 {
            assert!(
                (tangent_roundtrip.components()[i] - tangent_q.components()[i]).abs() < 1.0e-12
            );
        }
        let force_y = Covector::<DenseGeneralizedFrame, 3>::new([2.0, -1.0, 4.0]).unwrap();
        let force_q = basis.push_covector_to_from(&force_y).unwrap();
        let force_roundtrip = basis.pull_covector_to_to(&force_q).unwrap();
        for i in 0..3 {
            assert!((force_roundtrip.components()[i] - force_y.components()[i]).abs() < 1.0e-12);
        }
    }

    #[test]
    fn ordinary_coordinate_norms_change_while_metric_norm_stays_fixed() {
        let metric_q = cartesian_metric();
        let basis = rebasis();
        let tangent_y = Tangent::<DenseGeneralizedFrame, 3>::new([0.0, 1.0, 0.0]).unwrap();
        let tangent_q = basis.push_tangent_to_from(&tangent_y).unwrap();
        let metric_y = basis.pull_metric_to_to(&metric_q, profile()).unwrap();
        let euclidean_q = coordinate_inf_tangent(&tangent_q);
        let euclidean_y = coordinate_inf_tangent(&tangent_y);
        let physical_q = metric_q.tangent_norm(&tangent_q, profile()).unwrap();
        let physical_y = metric_y.tangent_norm(&tangent_y, profile()).unwrap();
        assert_ne!(euclidean_q, euclidean_y);
        assert!((physical_q - physical_y).abs() < 1.0e-12);
    }

    #[test]
    fn wrong_covector_variance_is_detected_by_pairing_negative_control() {
        let basis = rebasis();
        let tangent_y = Tangent::<DenseGeneralizedFrame, 3>::new([1.0, 2.0, -1.0]).unwrap();
        let tangent_q = basis.push_tangent_to_from(&tangent_y).unwrap();
        let force_q = Covector::<CartesianFrame, 3>::new([2.0, -3.0, 1.0]).unwrap();
        let correct = basis.pull_covector_to_to(&force_q).unwrap();
        let wrong_components = apply(basis.matrix(), force_q.components());
        let wrong = Covector::<DenseGeneralizedFrame, 3>::new(wrong_components).unwrap();
        let correct_pairing = dot(tangent_q.components(), force_q.components());
        let pulled_pairing = dot(tangent_y.components(), correct.components());
        let wrong_pairing = dot(tangent_y.components(), wrong.components());
        assert!((correct_pairing - pulled_pairing).abs() < 1.0e-12);
        assert!((correct_pairing - wrong_pairing).abs() > 1.0e-3);
    }

    #[test]
    fn condition_ladder_preserves_metric_norms_through_dense_shear() {
        let metric_q = cartesian_metric();
        let tangent_q = Tangent::<CartesianFrame, 3>::new([0.0, 1.0, 0.0]).unwrap();
        let force_q = Covector::<CartesianFrame, 3>::new([3.0, -2.0, 4.0]).unwrap();
        let tangent_norm_reference = metric_q.tangent_norm(&tangent_q, profile()).unwrap();
        let covector_norm_reference = metric_q.cotangent_norm(&force_q, profile()).unwrap();

        for shear in [1.0, 10.0, 100.0, 1_000.0] {
            let basis = CoordinateRebasis::<CartesianFrame, DenseGeneralizedFrame, 3>::new(
                [[1.0, shear, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                profile(),
            )
            .unwrap();
            let tangent_y = basis.pull_tangent_to_to(&tangent_q).unwrap();
            let metric_y = basis.pull_metric_to_to(&metric_q, profile()).unwrap();
            let covector_y = basis.pull_covector_to_to(&force_q).unwrap();

            let tangent_norm = metric_y.tangent_norm(&tangent_y, profile()).unwrap();
            let covector_norm = metric_y.cotangent_norm(&covector_y, profile()).unwrap();
            let tangent_relative_defect =
                (tangent_norm - tangent_norm_reference).abs() / tangent_norm_reference;
            let covector_relative_defect =
                (covector_norm - covector_norm_reference).abs() / covector_norm_reference;
            let condition = basis.condition_inf_estimate();

            // The admissible rounding envelope widens with the coordinate
            // condition estimate. Physical quantities remain the comparison target.
            let tolerance = if shear <= 1.0 {
                2.0e-10
            } else if shear <= 10.0 {
                2.0e-8
            } else if shear <= 100.0 {
                2.0e-5
            } else {
                2.0e-2
            };
            assert!(
                tangent_relative_defect < tolerance,
                "tangent norm drift at shear {shear}, condition {condition:e}: {tangent_relative_defect:e}"
            );
            assert!(
                covector_relative_defect < tolerance,
                "cotangent norm drift at shear {shear}, condition {condition:e}: {covector_relative_defect:e}"
            );
        }
    }

    #[test]
    fn nonsymmetric_and_indefinite_metrics_fail_closed() {
        assert_eq!(
            Metric::<CartesianFrame, 2>::new([[2.0, 0.1], [0.0, 3.0]], profile()).unwrap_err(),
            MetricErrorV1::MetricNotSymmetric
        );
        assert_eq!(
            Metric::<CartesianFrame, 2>::new([[1.0, 2.0], [2.0, 1.0]], profile()).unwrap_err(),
            MetricErrorV1::MetricNotPositiveDefinite
        );
    }

    #[test]
    fn singular_coordinate_change_is_rejected() {
        assert_eq!(
            CoordinateRebasis::<CartesianFrame, DenseGeneralizedFrame, 2>::new(
                [[1.0, 2.0], [2.0, 4.0]],
                profile()
            )
            .unwrap_err(),
            MetricErrorV1::SingularCoordinateChange
        );
    }

    #[test]
    fn nonfinite_intermediate_norms_fail_closed() {
        let metric =
            Metric::<CartesianFrame, 2>::new([[1.0e200, 0.0], [0.0, 1.0e200]], profile()).unwrap();
        let huge = Tangent::<CartesianFrame, 2>::new([1.0e200, 0.0]).unwrap();
        assert_eq!(
            metric.tangent_norm(&huge, profile()),
            Err(MetricErrorV1::NonFiniteArithmetic)
        );
    }
}
