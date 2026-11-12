// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Metric-induced, coordinate-covariant error measures for finite-dimensional systems.
//!
//! The API distinguishes tangent vectors from cotangent covectors and binds each
//! value to a coordinate-frame marker. Dense re-basing therefore has explicit
//! pushforward, pullback, and metric-congruence operations.
//!
//! The induced operator norm is evaluated after metric whitening. The resulting
//! condition estimate and forward-error envelope are floating-point estimates,
//! not interval-certified bounds.

use std::fmt;
use std::marker::PhantomData;

pub type Vector<const N: usize> = [f64; N];
pub type Matrix<const N: usize> = [[f64; N]; N];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeStatusV1 {
    InformativeEstimate,
    WeakEstimate,
    VacuousEstimate,
    OutsideConditionEnvelope,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricEnvelopeProfileV1 {
    pub relative_pivot_tolerance: f64,
    pub symmetry_tolerance: f64,
    pub maximum_condition: f64,
    pub maximum_condition_times_backward_error: f64,
}

impl Default for MetricEnvelopeProfileV1 {
    fn default() -> Self {
        Self {
            relative_pivot_tolerance: 128.0 * f64::EPSILON,
            symmetry_tolerance: 512.0 * f64::EPSILON,
            maximum_condition: 1.0e8,
            maximum_condition_times_backward_error: 1.0e-2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetricErrorV1 {
    ZeroDimension,
    InvalidProfile,
    NonFiniteMatrix,
    NonFiniteVector,
    NonsymmetricMetric,
    NonPositiveDefiniteMetric,
    SingularCoordinateChange,
    SingularOperator,
    ArithmeticOverflow,
    EigenIterationLimit,
}

impl fmt::Display for MetricErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for MetricErrorV1 {}

fn validate_profile(profile: MetricEnvelopeProfileV1) -> Result<(), MetricErrorV1> {
    if !profile.relative_pivot_tolerance.is_finite()
        || profile.relative_pivot_tolerance <= 0.0
        || !profile.symmetry_tolerance.is_finite()
        || profile.symmetry_tolerance <= 0.0
        || !profile.maximum_condition.is_finite()
        || profile.maximum_condition < 1.0
        || !profile.maximum_condition_times_backward_error.is_finite()
        || profile.maximum_condition_times_backward_error <= 0.0
        || profile.maximum_condition_times_backward_error >= 1.0
    {
        return Err(MetricErrorV1::InvalidProfile);
    }
    Ok(())
}

pub fn dot<const N: usize>(left: &Vector<N>, right: &Vector<N>) -> f64 {
    left.iter()
        .zip(right.iter())
        .map(|(a, b)| a * b)
        .sum()
}

pub fn vector_inf<const N: usize>(vector: &Vector<N>) -> f64 {
    vector.iter().map(|value| value.abs()).fold(0.0, f64::max)
}

pub fn matrix_inf<const N: usize>(matrix: &Matrix<N>) -> f64 {
    matrix
        .iter()
        .map(|row| row.iter().map(|value| value.abs()).sum::<f64>())
        .fold(0.0, f64::max)
}

pub fn transpose<const N: usize>(matrix: &Matrix<N>) -> Matrix<N> {
    let mut result = [[0.0; N]; N];
    for (row, result_row) in result.iter_mut().enumerate() {
        for (column, value) in result_row.iter_mut().enumerate() {
            *value = matrix[column][row];
        }
    }
    result
}

pub fn multiply<const N: usize>(left: &Matrix<N>, right: &Matrix<N>) -> Matrix<N> {
    let mut result = [[0.0; N]; N];
    for (row, result_row) in result.iter_mut().enumerate() {
        for (column, result_value) in result_row.iter_mut().enumerate() {
            let mut sum = 0.0;
            for (inner, left_value) in left[row].iter().enumerate() {
                sum += left_value * right[inner][column];
            }
            *result_value = sum;
        }
    }
    result
}

pub fn apply<const N: usize>(matrix: &Matrix<N>, vector: &Vector<N>) -> Vector<N> {
    let mut result = [0.0; N];
    for (row, result_value) in result.iter_mut().enumerate() {
        *result_value = dot(&matrix[row], vector);
    }
    result
}

fn cholesky<const N: usize>(
    matrix: &Matrix<N>,
    profile: MetricEnvelopeProfileV1,
) -> Result<Matrix<N>, MetricErrorV1> {
    if N == 0 {
        return Err(MetricErrorV1::ZeroDimension);
    }
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(MetricErrorV1::NonFiniteMatrix);
    }

    let scale = matrix_inf(matrix);
    if !scale.is_finite() {
        return Err(MetricErrorV1::ArithmeticOverflow);
    }
    let symmetry_limit = profile.symmetry_tolerance * scale.max(1.0);
    for (row, values) in matrix.iter().enumerate() {
        for (column, value) in values.iter().enumerate() {
            if (*value - matrix[column][row]).abs() > symmetry_limit {
                return Err(MetricErrorV1::NonsymmetricMetric);
            }
        }
    }

    let pivot_limit = profile.relative_pivot_tolerance * scale;
    let mut lower = [[0.0; N]; N];
    for row in 0..N {
        for column in 0..=row {
            let correction = (0..column)
                .map(|inner| lower[row][inner] * lower[column][inner])
                .sum::<f64>();
            let value = matrix[row][column] - correction;
            if row == column {
                if !value.is_finite() || value <= pivot_limit {
                    return Err(MetricErrorV1::NonPositiveDefiniteMetric);
                }
                lower[row][column] = value.sqrt();
            } else {
                lower[row][column] = value / lower[column][column];
            }
        }
    }
    if lower.iter().flatten().any(|value| !value.is_finite()) {
        return Err(MetricErrorV1::ArithmeticOverflow);
    }
    Ok(lower)
}

fn inverse_lower<const N: usize>(
    lower: &Matrix<N>,
) -> Result<Matrix<N>, MetricErrorV1> {
    let mut inverse = [[0.0; N]; N];
    for column in 0..N {
        for row in 0..N {
            let mut value = if row == column { 1.0 } else { 0.0 };
            for inner in 0..row {
                value -= lower[row][inner] * inverse[inner][column];
            }
            if lower[row][row] == 0.0 || !lower[row][row].is_finite() {
                return Err(MetricErrorV1::NonPositiveDefiniteMetric);
            }
            inverse[row][column] = value / lower[row][row];
        }
    }
    if inverse.iter().flatten().any(|value| !value.is_finite()) {
        return Err(MetricErrorV1::ArithmeticOverflow);
    }
    Ok(inverse)
}

fn jacobi_eigenvalues<const N: usize>(
    symmetric: &Matrix<N>,
) -> Result<Vector<N>, MetricErrorV1> {
    if N == 0 {
        return Err(MetricErrorV1::ZeroDimension);
    }
    if symmetric.iter().flatten().any(|value| !value.is_finite()) {
        return Err(MetricErrorV1::NonFiniteMatrix);
    }
    let mut matrix = *symmetric;
    let scale = matrix_inf(&matrix).max(1.0);
    let tolerance = 256.0 * f64::EPSILON * scale;
    let maximum_iterations = 128usize
        .checked_mul(N)
        .and_then(|value| value.checked_mul(N))
        .ok_or(MetricErrorV1::ArithmeticOverflow)?;

    for _ in 0..maximum_iterations {
        let mut pivot = None;
        let mut maximum = 0.0_f64;
        for (row, values) in matrix.iter().enumerate() {
            for (column, value) in values.iter().enumerate().skip(row + 1) {
                if value.abs() > maximum {
                    maximum = value.abs();
                    pivot = Some((row, column));
                }
            }
        }
        if maximum <= tolerance {
            return Ok(std::array::from_fn(|index| matrix[index][index]));
        }
        let (p, q) = pivot.ok_or(MetricErrorV1::EigenIterationLimit)?;
        let off_diagonal = matrix[p][q];
        let tau = (matrix[q][q] - matrix[p][p]) / (2.0 * off_diagonal);
        let sign = if tau >= 0.0 { 1.0 } else { -1.0 };
        let tangent = sign / (tau.abs() + (1.0 + tau * tau).sqrt());
        let cosine = 1.0 / (1.0 + tangent * tangent).sqrt();
        let sine = tangent * cosine;
        let pp = matrix[p][p] - tangent * off_diagonal;
        let qq = matrix[q][q] + tangent * off_diagonal;

        for index in 0..N {
            if index != p && index != q {
                let ip = matrix[index][p];
                let iq = matrix[index][q];
                matrix[index][p] = cosine * ip - sine * iq;
                matrix[p][index] = matrix[index][p];
                matrix[index][q] = sine * ip + cosine * iq;
                matrix[q][index] = matrix[index][q];
            }
        }
        matrix[p][p] = pp;
        matrix[q][q] = qq;
        matrix[p][q] = 0.0;
        matrix[q][p] = 0.0;
    }
    Err(MetricErrorV1::EigenIterationLimit)
}

pub struct TangentVectorV1<const N: usize, Frame> {
    values: Vector<N>,
    frame: PhantomData<fn() -> Frame>,
}

impl<const N: usize, Frame> Clone for TangentVectorV1<N, Frame> {
    fn clone(&self) -> Self {
        Self::new(self.values)
    }
}

impl<const N: usize, Frame> std::fmt::Debug for TangentVectorV1<N, Frame> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TangentVectorV1").field(&self.values).finish()
    }
}

impl<const N: usize, Frame> PartialEq for TangentVectorV1<N, Frame> {
    fn eq(&self, other: &Self) -> bool {
        self.values == other.values
    }
}

impl<const N: usize, Frame> TangentVectorV1<N, Frame> {
    pub fn new(values: Vector<N>) -> Self {
        Self {
            values,
            frame: PhantomData,
        }
    }

    pub fn values(&self) -> &Vector<N> {
        &self.values
    }
}

pub struct CotangentCovectorV1<const N: usize, Frame> {
    values: Vector<N>,
    frame: PhantomData<fn() -> Frame>,
}

impl<const N: usize, Frame> Clone for CotangentCovectorV1<N, Frame> {
    fn clone(&self) -> Self {
        Self::new(self.values)
    }
}

impl<const N: usize, Frame> std::fmt::Debug for CotangentCovectorV1<N, Frame> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CotangentCovectorV1")
            .field(&self.values)
            .finish()
    }
}

impl<const N: usize, Frame> PartialEq for CotangentCovectorV1<N, Frame> {
    fn eq(&self, other: &Self) -> bool {
        self.values == other.values
    }
}

impl<const N: usize, Frame> CotangentCovectorV1<N, Frame> {
    pub fn new(values: Vector<N>) -> Self {
        Self {
            values,
            frame: PhantomData,
        }
    }

    pub fn values(&self) -> &Vector<N> {
        &self.values
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetricTensorV1<const N: usize, Frame> {
    matrix: Matrix<N>,
    lower: Matrix<N>,
    inverse_lower: Matrix<N>,
    frame: PhantomData<fn() -> Frame>,
}

impl<const N: usize, Frame> MetricTensorV1<N, Frame> {
    pub fn new(
        matrix: Matrix<N>,
        profile: MetricEnvelopeProfileV1,
    ) -> Result<Self, MetricErrorV1> {
        validate_profile(profile)?;
        let lower = cholesky(&matrix, profile)?;
        let inverse_lower = inverse_lower(&lower)?;
        Ok(Self {
            matrix,
            lower,
            inverse_lower,
            frame: PhantomData,
        })
    }

    pub fn matrix(&self) -> &Matrix<N> {
        &self.matrix
    }

    pub fn tangent_norm(
        &self,
        tangent: &TangentVectorV1<N, Frame>,
    ) -> Result<f64, MetricErrorV1> {
        if tangent.values.iter().any(|value| !value.is_finite()) {
            return Err(MetricErrorV1::NonFiniteVector);
        }
        let mapped = apply(&transpose(&self.lower), &tangent.values);
        let norm_squared = dot(&mapped, &mapped);
        checked_sqrt(norm_squared)
    }

    pub fn cotangent_dual_norm(
        &self,
        covector: &CotangentCovectorV1<N, Frame>,
    ) -> Result<f64, MetricErrorV1> {
        if covector.values.iter().any(|value| !value.is_finite()) {
            return Err(MetricErrorV1::NonFiniteVector);
        }
        let mapped = apply(&self.inverse_lower, &covector.values);
        let norm_squared = dot(&mapped, &mapped);
        checked_sqrt(norm_squared)
    }

    pub fn riesz_map(
        &self,
        tangent: &TangentVectorV1<N, Frame>,
    ) -> Result<CotangentCovectorV1<N, Frame>, MetricErrorV1> {
        if tangent.values.iter().any(|value| !value.is_finite()) {
            return Err(MetricErrorV1::NonFiniteVector);
        }
        Ok(CotangentCovectorV1::new(apply(&self.matrix, &tangent.values)))
    }
}

fn checked_sqrt(value: f64) -> Result<f64, MetricErrorV1> {
    if !value.is_finite() {
        return Err(MetricErrorV1::ArithmeticOverflow);
    }
    if value < -1024.0 * f64::EPSILON {
        return Err(MetricErrorV1::NonPositiveDefiniteMetric);
    }
    Ok(value.max(0.0).sqrt())
}

pub struct CoordinateChangeV1<const N: usize, FromFrame, ToFrame> {
    from_from_to: Matrix<N>,
    from_to_from: Matrix<N>,
    profile: MetricEnvelopeProfileV1,
    frames: PhantomData<fn() -> (FromFrame, ToFrame)>,
}

impl<const N: usize, FromFrame, ToFrame> Clone
    for CoordinateChangeV1<N, FromFrame, ToFrame>
{
    fn clone(&self) -> Self {
        Self {
            from_from_to: self.from_from_to,
            from_to_from: self.from_to_from,
            profile: self.profile,
            frames: PhantomData,
        }
    }
}

impl<const N: usize, FromFrame, ToFrame> CoordinateChangeV1<N, FromFrame, ToFrame> {
    /// Construct S with x_from = S x_to. The inverse is computed and checked.
    pub fn new(
        from_from_to: Matrix<N>,
        profile: MetricEnvelopeProfileV1,
    ) -> Result<Self, MetricErrorV1> {
        validate_profile(profile)?;
        if N == 0 {
            return Err(MetricErrorV1::ZeroDimension);
        }
        if from_from_to.iter().flatten().any(|value| !value.is_finite()) {
            return Err(MetricErrorV1::NonFiniteMatrix);
        }
        let scale = matrix_inf(&from_from_to);
        if !scale.is_finite() {
            return Err(MetricErrorV1::ArithmeticOverflow);
        }
        let mut augmented = from_from_to;
        let mut inverse = [[0.0; N]; N];
        for index in 0..N {
            inverse[index][index] = 1.0;
        }
        // Gauss-Jordan elimination with scaled partial pivoting.
        let threshold = profile.relative_pivot_tolerance * scale;
        for column in 0..N {
            let pivot_row = (column..N)
                .max_by(|left, right| {
                    augmented[*left][column]
                        .abs()
                        .total_cmp(&augmented[*right][column].abs())
                })
                .ok_or(MetricErrorV1::ZeroDimension)?;
            let magnitude = augmented[pivot_row][column].abs();
            if !magnitude.is_finite() || magnitude <= threshold {
                return Err(MetricErrorV1::SingularCoordinateChange);
            }
            if pivot_row != column {
                augmented.swap(pivot_row, column);
                inverse.swap(pivot_row, column);
            }
            let pivot = augmented[column][column];
            for value in augmented[column].iter_mut() {
                *value /= pivot;
            }
            for value in inverse[column].iter_mut() {
                *value /= pivot;
            }
            for row in 0..N {
                if row != column {
                    let factor = augmented[row][column];
                    for entry in 0..N {
                        augmented[row][entry] -= factor * augmented[column][entry];
                        inverse[row][entry] -= factor * inverse[column][entry];
                    }
                }
            }
        }
        if inverse.iter().flatten().any(|value| !value.is_finite()) {
            return Err(MetricErrorV1::ArithmeticOverflow);
        }
        Ok(Self {
            from_from_to,
            from_to_from: inverse,
            profile,
            frames: PhantomData,
        })
    }

    pub fn matrix(&self) -> &Matrix<N> {
        &self.from_from_to
    }

    pub fn push_tangent_to_from(
        &self,
        tangent_to: &TangentVectorV1<N, ToFrame>,
    ) -> TangentVectorV1<N, FromFrame> {
        TangentVectorV1::new(apply(&self.from_from_to, &tangent_to.values))
    }

    pub fn pull_covector_to_to(
        &self,
        covector_from: &CotangentCovectorV1<N, FromFrame>,
    ) -> CotangentCovectorV1<N, ToFrame> {
        CotangentCovectorV1::new(apply(
            &transpose(&self.from_from_to),
            &covector_from.values,
        ))
    }

    pub fn pull_metric_to_to(
        &self,
        metric_from: &MetricTensorV1<N, FromFrame>,
    ) -> Result<MetricTensorV1<N, ToFrame>, MetricErrorV1> {
        let transformed = multiply(
            &multiply(&transpose(&self.from_from_to), &metric_from.matrix),
            &self.from_from_to,
        );
        MetricTensorV1::new(transformed, self.profile)
    }

    /// Pull a bilinear map T_from -> T*_from back to the destination frame.
    /// For x_from = S x_to, A_to = S^T A_from S.
    pub fn pull_bilinear_map_to_to(
        &self,
        map_from: &BilinearMapV1<N, FromFrame>,
    ) -> BilinearMapV1<N, ToFrame> {
        let transformed = multiply(
            &multiply(&transpose(&self.from_from_to), &map_from.matrix),
            &self.from_from_to,
        );
        BilinearMapV1::new(transformed)
    }

    pub fn pull_tangent_to_to(
        &self,
        tangent_from: &TangentVectorV1<N, FromFrame>,
    ) -> TangentVectorV1<N, ToFrame> {
        TangentVectorV1::new(apply(&self.from_to_from, &tangent_from.values))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BilinearMapV1<const N: usize, Frame> {
    matrix: Matrix<N>,
    frame: PhantomData<fn() -> Frame>,
}

impl<const N: usize, Frame> BilinearMapV1<N, Frame> {
    pub fn new(matrix: Matrix<N>) -> Self {
        Self {
            matrix,
            frame: PhantomData,
        }
    }

    pub fn matrix(&self) -> &Matrix<N> {
        &self.matrix
    }

    pub fn apply(
        &self,
        tangent: &TangentVectorV1<N, Frame>,
    ) -> CotangentCovectorV1<N, Frame> {
        CotangentCovectorV1::new(apply(&self.matrix, &tangent.values))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OperatorMeasureV1 {
    pub induced_operator_norm: f64,
    pub inverse_operator_norm: f64,
    pub condition_estimate: f64,
}

fn operator_measure<const N: usize, DomainFrame, CodomainFrame>(
    map: &BilinearMapV1<N, DomainFrame>,
    domain_metric: &MetricTensorV1<N, DomainFrame>,
    codomain_metric: &MetricTensorV1<N, CodomainFrame>,
) -> Result<OperatorMeasureV1, MetricErrorV1> {
    let codomain_whitener = codomain_metric.inverse_lower;
    let domain_unwhitener_t = transpose(&domain_metric.inverse_lower);
    let whitened = multiply(
        &multiply(&codomain_whitener, &map.matrix),
        &domain_unwhitener_t,
    );
    let gram = multiply(&transpose(&whitened), &whitened);
    let eigenvalues = jacobi_eigenvalues(&gram)?;
    let maximum = eigenvalues
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let minimum = eigenvalues.iter().copied().fold(f64::INFINITY, f64::min);
    if !maximum.is_finite() || !minimum.is_finite() {
        return Err(MetricErrorV1::ArithmeticOverflow);
    }
    if maximum <= 0.0 || minimum <= 0.0 {
        return Err(MetricErrorV1::SingularOperator);
    }
    let induced_operator_norm = maximum.sqrt();
    let inverse_operator_norm = 1.0 / minimum.sqrt();
    let condition_estimate = induced_operator_norm * inverse_operator_norm;
    if !condition_estimate.is_finite() {
        return Err(MetricErrorV1::ArithmeticOverflow);
    }
    Ok(OperatorMeasureV1 {
        induced_operator_norm,
        inverse_operator_norm,
        condition_estimate,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetricErrorEnvelopeV1 {
    pub residual_dual_norm: f64,
    pub solution_tangent_norm: f64,
    pub rhs_dual_norm: f64,
    pub induced_operator_norm: f64,
    pub condition_estimate: f64,
    pub normwise_backward_error: f64,
    pub condition_times_backward_error: f64,
    pub forward_error_bound_estimate: Option<f64>,
    pub status: EnvelopeStatusV1,
}

/// Assess A x = b where A maps the domain tangent space to the codomain
/// cotangent space. The residual is therefore a codomain covector.
pub fn assess_solution<const N: usize, DomainFrame, CodomainFrame>(
    map: &BilinearMapV1<N, DomainFrame>,
    domain_metric: &MetricTensorV1<N, DomainFrame>,
    codomain_metric: &MetricTensorV1<N, CodomainFrame>,
    rhs: &CotangentCovectorV1<N, CodomainFrame>,
    computed: &TangentVectorV1<N, DomainFrame>,
    profile: MetricEnvelopeProfileV1,
) -> Result<MetricErrorEnvelopeV1, MetricErrorV1> {
    validate_profile(profile)?;
    let measure = operator_measure(map, domain_metric, codomain_metric)?;
    let predicted = map.apply(computed);
    let residual_values: Vector<N> =
        std::array::from_fn(|index| rhs.values[index] - predicted.values[index]);
    let residual = CotangentCovectorV1::<N, CodomainFrame>::new(residual_values);
    let residual_dual_norm = codomain_metric.cotangent_dual_norm(&residual)?;
    let solution_tangent_norm = domain_metric.tangent_norm(computed)?;
    let rhs_dual_norm = codomain_metric.cotangent_dual_norm(rhs)?;
    let denominator =
        measure.induced_operator_norm * solution_tangent_norm + rhs_dual_norm;
    if !denominator.is_finite() || denominator <= 0.0 {
        return Err(MetricErrorV1::ArithmeticOverflow);
    }
    let normwise_backward_error = residual_dual_norm / denominator;
    let condition_times_backward_error = measure.condition_estimate * normwise_backward_error;
    if !normwise_backward_error.is_finite()
        || !condition_times_backward_error.is_finite()
    {
        return Err(MetricErrorV1::ArithmeticOverflow);
    }
    let forward_error_bound_estimate = if condition_times_backward_error < 1.0 {
        Some(
            2.0 * condition_times_backward_error
                / (1.0 - condition_times_backward_error),
        )
    } else {
        None
    };
    let status = if measure.condition_estimate > profile.maximum_condition {
        EnvelopeStatusV1::OutsideConditionEnvelope
    } else if forward_error_bound_estimate.is_none() {
        EnvelopeStatusV1::VacuousEstimate
    } else if condition_times_backward_error
        > profile.maximum_condition_times_backward_error
    {
        EnvelopeStatusV1::WeakEstimate
    } else {
        EnvelopeStatusV1::InformativeEstimate
    };
    Ok(MetricErrorEnvelopeV1 {
        residual_dual_norm,
        solution_tangent_norm,
        rhs_dual_norm,
        induced_operator_norm: measure.induced_operator_norm,
        condition_estimate: measure.condition_estimate,
        normwise_backward_error,
        condition_times_backward_error,
        forward_error_bound_estimate,
        status,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Cartesian;
    #[derive(Debug)]
    struct Dense;

    fn profile() -> MetricEnvelopeProfileV1 {
        MetricEnvelopeProfileV1::default()
    }

    fn close(left: f64, right: f64, tolerance: f64) {
        assert!(
            (left - right).abs() <= tolerance,
            "left={left:.17e}, right={right:.17e}, tolerance={tolerance:.3e}"
        );
    }

    fn base_metric() -> MetricTensorV1<3, Cartesian> {
        MetricTensorV1::new(
            [[2.0, 0.1, 0.0], [0.1, 3.0, 0.2], [0.0, 0.2, 4.0]],
            profile(),
        )
        .unwrap()
    }

    fn dense_change() -> CoordinateChangeV1<3, Cartesian, Dense> {
        CoordinateChangeV1::new(
            [[1.0, 2.0, 0.5], [0.0, 1.0, 1.5], [0.0, 0.0, 1.0]],
            profile(),
        )
        .unwrap()
    }

    #[test]
    fn tangent_and_cotangent_norms_are_invariant_under_dense_rebasing() {
        let metric_q = base_metric();
        let change = dense_change();
        let metric_y = change.pull_metric_to_to(&metric_q).unwrap();

        let tangent_y = TangentVectorV1::<3, Dense>::new([0.25, -1.0, 2.0]);
        let tangent_q = change.push_tangent_to_from(&tangent_y);
        close(
            metric_q.tangent_norm(&tangent_q).unwrap(),
            metric_y.tangent_norm(&tangent_y).unwrap(),
            2.0e-13,
        );

        let covector_q = CotangentCovectorV1::<3, Cartesian>::new([1.5, -0.25, 0.75]);
        let covector_y = change.pull_covector_to_to(&covector_q);
        close(
            metric_q.cotangent_dual_norm(&covector_q).unwrap(),
            metric_y.cotangent_dual_norm(&covector_y).unwrap(),
            2.0e-13,
        );
    }

    #[test]
    fn tangent_covector_pairing_is_preserved() {
        let change = dense_change();
        let tangent_y = TangentVectorV1::<3, Dense>::new([1.0, -0.5, 2.0]);
        let tangent_q = change.push_tangent_to_from(&tangent_y);
        let covector_q = CotangentCovectorV1::<3, Cartesian>::new([2.0, 3.0, -1.0]);
        let covector_y = change.pull_covector_to_to(&covector_q);
        close(
            dot(covector_q.values(), tangent_q.values()),
            dot(covector_y.values(), tangent_y.values()),
            1.0e-13,
        );
    }

    #[test]
    fn riesz_map_is_an_isometry_between_tangent_and_dual_cotangent_norms() {
        let metric = base_metric();
        let tangent = TangentVectorV1::<3, Cartesian>::new([0.3, -1.1, 2.0]);
        let covector = metric.riesz_map(&tangent).unwrap();
        close(
            metric.tangent_norm(&tangent).unwrap(),
            metric.cotangent_dual_norm(&covector).unwrap(),
            2.0e-13,
        );
    }

    #[test]
    fn metric_induced_operator_norm_and_condition_are_rebase_invariant() {
        let metric_q = base_metric();
        let change = dense_change();
        let metric_y = change.pull_metric_to_to(&metric_q).unwrap();
        let operator_q = BilinearMapV1::<3, Cartesian>::new([
            [4.0, 1.0, 0.5],
            [0.25, 2.0, 0.0],
            [0.0, 0.75, 3.0],
        ]);
        let operator_y = change.pull_bilinear_map_to_to(&operator_q);

        let q_measure = operator_measure(&operator_q, &metric_q, &metric_q).unwrap();
        let y_measure = operator_measure(&operator_y, &metric_y, &metric_y).unwrap();
        close(
            q_measure.induced_operator_norm,
            y_measure.induced_operator_norm,
            2.0e-10,
        );
        close(
            q_measure.condition_estimate,
            y_measure.condition_estimate,
            2.0e-9,
        );
    }

    #[test]
    fn backward_error_envelope_is_invariant_under_dense_rebasing() {
        let metric_q = base_metric();
        let change = dense_change();
        let metric_y = change.pull_metric_to_to(&metric_q).unwrap();
        let operator_q = BilinearMapV1::<3, Cartesian>::new([
            [4.0, 1.0, 0.5],
            [0.25, 2.0, 0.0],
            [0.0, 0.75, 3.0],
        ]);
        let operator_y = change.pull_bilinear_map_to_to(&operator_q);
        let exact_q = TangentVectorV1::<3, Cartesian>::new([0.5, -1.0, 2.0]);
        let exact_y = change.pull_tangent_to_to(&exact_q);
        let rhs_q = operator_q.apply(&exact_q);
        let rhs_y = operator_y.apply(&exact_y);
        let computed_q =
            TangentVectorV1::<3, Cartesian>::new([0.5 + 1.0e-5, -1.0, 2.0]);
        let computed_y = change.pull_tangent_to_to(&computed_q);

        let q_envelope = assess_solution(
            &operator_q,
            &metric_q,
            &metric_q,
            &rhs_q,
            &computed_q,
            profile(),
        )
        .unwrap();
        let y_envelope = assess_solution(
            &operator_y,
            &metric_y,
            &metric_y,
            &rhs_y,
            &computed_y,
            profile(),
        )
        .unwrap();
        close(
            q_envelope.normwise_backward_error,
            y_envelope.normwise_backward_error,
            2.0e-11,
        );
        close(
            q_envelope.forward_error_bound_estimate.unwrap(),
            y_envelope.forward_error_bound_estimate.unwrap(),
            2.0e-9,
        );
    }

    #[test]
    fn raw_coordinate_infinity_norm_changes_while_physical_norm_does_not() {
        let metric_q = base_metric();
        let change = dense_change();
        let metric_y = change.pull_metric_to_to(&metric_q).unwrap();
        let tangent_y = TangentVectorV1::<3, Dense>::new([1.0, -0.25, 0.5]);
        let tangent_q = change.push_tangent_to_from(&tangent_y);
        let raw_q = vector_inf(tangent_q.values());
        let raw_y = vector_inf(tangent_y.values());
        assert!((raw_q - raw_y).abs() > 0.5);
        close(
            metric_q.tangent_norm(&tangent_q).unwrap(),
            metric_y.tangent_norm(&tangent_y).unwrap(),
            2.0e-13,
        );
    }

    #[test]
    fn wrong_covector_transform_is_detected_by_dual_norm_and_pairing() {
        let metric_q = base_metric();
        let change = dense_change();
        let metric_y = change.pull_metric_to_to(&metric_q).unwrap();
        let covector_q = CotangentCovectorV1::<3, Cartesian>::new([0.5, 1.0, -0.75]);
        let wrong_values = apply(
            &change.from_to_from,
            covector_q.values(),
        );
        let correct = change.pull_covector_to_to(&covector_q);
        assert!(
            (wrong_values[0] - correct.values()[0]).abs()
                + (wrong_values[1] - correct.values()[1]).abs()
                + (wrong_values[2] - correct.values()[2]).abs()
                > 0.1
        );
        let wrong = CotangentCovectorV1::<3, Dense>::new(wrong_values);
        assert!(
            (metric_y.cotangent_dual_norm(&wrong).unwrap()
                - metric_q.cotangent_dual_norm(&covector_q).unwrap())
            .abs()
                > 0.1
        );
    }

    #[test]
    fn non_positive_definite_and_singular_coordinate_metrics_fail_closed() {
        assert!(matches!(
            MetricTensorV1::<2, Cartesian>::new([[1.0, 2.0], [2.0, 1.0]], profile()),
            Err(MetricErrorV1::NonPositiveDefiniteMetric)
        ));
        assert!(matches!(
            CoordinateChangeV1::<2, Cartesian, Dense>::new(
                [[1.0, 2.0], [2.0, 4.0]],
                profile()
            ),
            Err(MetricErrorV1::SingularCoordinateChange)
        ));
    }
}
