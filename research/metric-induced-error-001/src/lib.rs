// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Coordinate-covariant tangent and cotangent norms for small dense SPD metrics.
//!
//! For q = S y:
//!   v_q = S v_y
//!   alpha_y = S^T alpha_q
//!   M_y = S^T M_q S
//! Tangent vectors and cotangent covectors are distinct frame-typed values.

use std::fmt;
use std::marker::PhantomData;

pub type Matrix<const N: usize> = [[f64; N]; N];

pub trait CoordinateFrameV1: Copy + 'static {
    const FRAME_ID: &'static str;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricErrorV1 {
    ZeroDimension,
    NonFiniteInput,
    NonFiniteIntermediate,
    NotSymmetric,
    NotPositiveDefinite,
    SingularBasis,
    NegativeNormSquare,
    InvalidTolerance,
}

impl fmt::Display for MetricErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for MetricErrorV1 {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricProfileV1 {
    pub relative_symmetry_tolerance: f64,
    pub relative_pivot_tolerance: f64,
    pub norm_square_roundoff_tolerance: f64,
}

impl Default for MetricProfileV1 {
    fn default() -> Self {
        Self {
            relative_symmetry_tolerance: 128.0 * f64::EPSILON,
            relative_pivot_tolerance: 64.0 * f64::EPSILON,
            norm_square_roundoff_tolerance: 512.0 * f64::EPSILON,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TangentVectorV1<Frame: CoordinateFrameV1, const N: usize> {
    values: [f64; N],
    frame: PhantomData<Frame>,
}

impl<Frame: CoordinateFrameV1, const N: usize> TangentVectorV1<Frame, N> {
    pub fn new(values: [f64; N]) -> Result<Self, MetricErrorV1> {
        if N == 0 {
            return Err(MetricErrorV1::ZeroDimension);
        }
        if values.iter().any(|value| !value.is_finite()) {
            return Err(MetricErrorV1::NonFiniteInput);
        }
        Ok(Self { values, frame: PhantomData })
    }

    pub fn values(&self) -> &[f64; N] {
        &self.values
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CovectorV1<Frame: CoordinateFrameV1, const N: usize> {
    values: [f64; N],
    frame: PhantomData<Frame>,
}

impl<Frame: CoordinateFrameV1, const N: usize> CovectorV1<Frame, N> {
    pub fn new(values: [f64; N]) -> Result<Self, MetricErrorV1> {
        if N == 0 {
            return Err(MetricErrorV1::ZeroDimension);
        }
        if values.iter().any(|value| !value.is_finite()) {
            return Err(MetricErrorV1::NonFiniteInput);
        }
        Ok(Self { values, frame: PhantomData })
    }

    pub fn values(&self) -> &[f64; N] {
        &self.values
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricV1<Frame: CoordinateFrameV1, const N: usize> {
    matrix: Matrix<N>,
    profile: MetricProfileV1,
    frame: PhantomData<Frame>,
}

fn validate_profile(profile: MetricProfileV1) -> Result<(), MetricErrorV1> {
    if !profile.relative_symmetry_tolerance.is_finite()
        || profile.relative_symmetry_tolerance <= 0.0
        || !profile.relative_pivot_tolerance.is_finite()
        || profile.relative_pivot_tolerance <= 0.0
        || !profile.norm_square_roundoff_tolerance.is_finite()
        || profile.norm_square_roundoff_tolerance <= 0.0
    {
        return Err(MetricErrorV1::InvalidTolerance);
    }
    Ok(())
}

fn matrix_inf<const N: usize>(matrix: &Matrix<N>) -> f64 {
    matrix.iter()
        .map(|row| row.iter().map(|value| value.abs()).sum::<f64>())
        .fold(0.0, f64::max)
}

fn transpose<const N: usize>(matrix: &Matrix<N>) -> Matrix<N> {
    let mut output = [[0.0; N]; N];
    for row in 0..N {
        for column in 0..N {
            output[row][column] = matrix[column][row];
        }
    }
    output
}

fn multiply<const N: usize>(left: &Matrix<N>, right: &Matrix<N>) -> Matrix<N> {
    let mut output = [[0.0; N]; N];
    for row in 0..N {
        for column in 0..N {
            for inner in 0..N {
                output[row][column] += left[row][inner] * right[inner][column];
            }
        }
    }
    output
}

fn apply<const N: usize>(matrix: &Matrix<N>, vector: &[f64; N]) -> [f64; N] {
    let mut output = [0.0; N];
    for row in 0..N {
        for column in 0..N {
            output[row] += matrix[row][column] * vector[column];
        }
    }
    output
}

fn dot<const N: usize>(left: &[f64; N], right: &[f64; N]) -> f64 {
    left.iter().zip(right.iter()).map(|(a, b)| a * b).sum()
}

/// Cholesky factorization A = L L^T with a scale-relative pivot floor.
fn cholesky<const N: usize>(
    matrix: &Matrix<N>,
    profile: MetricProfileV1,
) -> Result<Matrix<N>, MetricErrorV1> {
    if N == 0 {
        return Err(MetricErrorV1::ZeroDimension);
    }
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(MetricErrorV1::NonFiniteInput);
    }
    let scale = matrix_inf(matrix);
    if !scale.is_finite() {
        return Err(MetricErrorV1::NonFiniteIntermediate);
    }
    if scale == 0.0 {
        return Err(MetricErrorV1::NotPositiveDefinite);
    }
    let pivot_floor = profile.relative_pivot_tolerance * scale;
    if !pivot_floor.is_finite() {
        return Err(MetricErrorV1::NonFiniteIntermediate);
    }

    let mut lower = [[0.0; N]; N];
    for row in 0..N {
        for column in 0..=row {
            let mut value = matrix[row][column];
            for inner in 0..column {
                value -= lower[row][inner] * lower[column][inner];
            }
            if !value.is_finite() {
                return Err(MetricErrorV1::NonFiniteIntermediate);
            }
            if row == column {
                if value <= pivot_floor {
                    return Err(MetricErrorV1::NotPositiveDefinite);
                }
                lower[row][column] = value.sqrt();
            } else {
                lower[row][column] = value / lower[column][column];
            }
            if !lower[row][column].is_finite() {
                return Err(MetricErrorV1::NonFiniteIntermediate);
            }
        }
    }
    Ok(lower)
}

fn solve_lower<const N: usize>(
    lower: &Matrix<N>,
    rhs: &[f64; N],
) -> Result<[f64; N], MetricErrorV1> {
    let mut solution = [0.0; N];
    for row in 0..N {
        let mut value = rhs[row];
        for column in 0..row {
            value -= lower[row][column] * solution[column];
        }
        solution[row] = value / lower[row][row];
        if !solution[row].is_finite() {
            return Err(MetricErrorV1::NonFiniteIntermediate);
        }
    }
    Ok(solution)
}

fn solve_upper_from_lower_transpose<const N: usize>(
    lower: &Matrix<N>,
    rhs: &[f64; N],
) -> Result<[f64; N], MetricErrorV1> {
    let mut solution = [0.0; N];
    for row in (0..N).rev() {
        let mut value = rhs[row];
        for column in (row + 1)..N {
            value -= lower[column][row] * solution[column];
        }
        solution[row] = value / lower[row][row];
        if !solution[row].is_finite() {
            return Err(MetricErrorV1::NonFiniteIntermediate);
        }
    }
    Ok(solution)
}

fn solve_spd<const N: usize>(
    matrix: &Matrix<N>,
    rhs: &[f64; N],
    profile: MetricProfileV1,
) -> Result<[f64; N], MetricErrorV1> {
    let lower = cholesky(matrix, profile)?;
    let intermediate = solve_lower(&lower, rhs)?;
    solve_upper_from_lower_transpose(&lower, &intermediate)
}

/// General dense solve with partial pivoting, used for coordinate transforms.
fn solve_dense<const N: usize>(
    matrix: &Matrix<N>,
    rhs: &[f64; N],
    profile: MetricProfileV1,
) -> Result<[f64; N], MetricErrorV1> {
    if N == 0 {
        return Err(MetricErrorV1::ZeroDimension);
    }
    if matrix.iter().flatten().any(|value| !value.is_finite())
        || rhs.iter().any(|value| !value.is_finite())
    {
        return Err(MetricErrorV1::NonFiniteInput);
    }
    let scale = matrix_inf(matrix);
    if !scale.is_finite() {
        return Err(MetricErrorV1::NonFiniteIntermediate);
    }
    if scale == 0.0 {
        return Err(MetricErrorV1::SingularBasis);
    }
    let threshold = profile.relative_pivot_tolerance * scale;
    if !threshold.is_finite() {
        return Err(MetricErrorV1::NonFiniteIntermediate);
    }
    let mut lu = *matrix;
    let mut permutation = std::array::from_fn(|index| index);

    for column in 0..N {
        let pivot_row = (column..N)
            .max_by(|left, right| lu[*left][column].abs().total_cmp(&lu[*right][column].abs()))
            .ok_or(MetricErrorV1::ZeroDimension)?;
        if lu[pivot_row][column].abs() <= threshold {
            return Err(MetricErrorV1::SingularBasis);
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
    }
    if lu.iter().flatten().any(|value| !value.is_finite()) {
        return Err(MetricErrorV1::NonFiniteIntermediate);
    }

    let mut solution = std::array::from_fn(|row| rhs[permutation[row]]);
    for row in 0..N {
        for column in 0..row {
            solution[row] -= lu[row][column] * solution[column];
        }
    }
    for row in (0..N).rev() {
        for column in (row + 1)..N {
            solution[row] -= lu[row][column] * solution[column];
        }
        solution[row] /= lu[row][row];
        if !solution[row].is_finite() {
            return Err(MetricErrorV1::NonFiniteIntermediate);
        }
    }
    Ok(solution)
}

fn validate_basis<const N: usize>(
    basis: &Matrix<N>,
    profile: MetricProfileV1,
) -> Result<(), MetricErrorV1> {
    solve_dense(basis, &[0.0; N], profile).map(|_| ())
}

impl<Frame: CoordinateFrameV1, const N: usize> MetricV1<Frame, N> {
    pub fn new(
        matrix: Matrix<N>,
        profile: MetricProfileV1,
    ) -> Result<Self, MetricErrorV1> {
        validate_profile(profile)?;
        if N == 0 {
            return Err(MetricErrorV1::ZeroDimension);
        }
        if matrix.iter().flatten().any(|value| !value.is_finite()) {
            return Err(MetricErrorV1::NonFiniteInput);
        }
        let scale = matrix_inf(&matrix);
        if !scale.is_finite() || scale == 0.0 {
            return Err(MetricErrorV1::NotPositiveDefinite);
        }
        for row in 0..N {
            for column in 0..N {
                if (matrix[row][column] - matrix[column][row]).abs()
                    > profile.relative_symmetry_tolerance * scale
                {
                    return Err(MetricErrorV1::NotSymmetric);
                }
            }
        }
        cholesky(&matrix, profile)?;
        Ok(Self { matrix, profile, frame: PhantomData })
    }

    pub fn matrix(&self) -> &Matrix<N> {
        &self.matrix
    }

    pub fn frame_id(&self) -> &'static str {
        Frame::FRAME_ID
    }

    pub fn tangent_norm(
        &self,
        vector: &TangentVectorV1<Frame, N>,
    ) -> Result<f64, MetricErrorV1> {
        let product = apply(&self.matrix, &vector.values);
        let square = dot(&vector.values, &product);
        checked_norm(square, self.profile)
    }

    pub fn dual_norm(
        &self,
        covector: &CovectorV1<Frame, N>,
    ) -> Result<f64, MetricErrorV1> {
        let raised = solve_spd(&self.matrix, &covector.values, self.profile)?;
        checked_norm(dot(&covector.values, &raised), self.profile)
    }

    pub fn raise(
        &self,
        covector: &CovectorV1<Frame, N>,
    ) -> Result<TangentVectorV1<Frame, N>, MetricErrorV1> {
        TangentVectorV1::new(solve_spd(&self.matrix, &covector.values, self.profile)?)
    }

    pub fn lower(
        &self,
        vector: &TangentVectorV1<Frame, N>,
    ) -> Result<CovectorV1<Frame, N>, MetricErrorV1> {
        CovectorV1::new(apply(&self.matrix, &vector.values))
    }

    pub fn pairing(
        &self,
        covector: &CovectorV1<Frame, N>,
        vector: &TangentVectorV1<Frame, N>,
    ) -> f64 {
        dot(&covector.values, &vector.values)
    }
}

fn checked_norm(square: f64, profile: MetricProfileV1) -> Result<f64, MetricErrorV1> {
    if !square.is_finite() {
        return Err(MetricErrorV1::NonFiniteIntermediate);
    }
    if square < 0.0 {
        if square.abs() <= profile.norm_square_roundoff_tolerance {
            return Ok(0.0);
        }
        return Err(MetricErrorV1::NegativeNormSquare);
    }
    Ok(square.sqrt())
}

/// Push a tangent vector from From coordinates to To coordinates: v_to = S v_from.
pub fn push_tangent<From: CoordinateFrameV1, To: CoordinateFrameV1, const N: usize>(
    transform_to_from: &Matrix<N>,
    vector_from: &TangentVectorV1<From, N>,
    profile: MetricProfileV1,
) -> Result<TangentVectorV1<To, N>, MetricErrorV1> {
    validate_basis(transform_to_from, profile)?;
    TangentVectorV1::new(apply(transform_to_from, &vector_from.values))
}

/// Pull a covector from To coordinates to From coordinates: alpha_from = S^T alpha_to.
pub fn pull_covector<From: CoordinateFrameV1, To: CoordinateFrameV1, const N: usize>(
    transform_to_from: &Matrix<N>,
    covector_to: &CovectorV1<To, N>,
    profile: MetricProfileV1,
) -> Result<CovectorV1<From, N>, MetricErrorV1> {
    validate_basis(transform_to_from, profile)?;
    CovectorV1::new(apply(&transpose(transform_to_from), &covector_to.values))
}

/// Pull a tangent vector from To coordinates to From coordinates: v_from = S^-1 v_to.
pub fn pull_tangent<From: CoordinateFrameV1, To: CoordinateFrameV1, const N: usize>(
    transform_to_from: &Matrix<N>,
    vector_to: &TangentVectorV1<To, N>,
    profile: MetricProfileV1,
) -> Result<TangentVectorV1<From, N>, MetricErrorV1> {
    TangentVectorV1::new(solve_dense(transform_to_from, &vector_to.values, profile)?)
}

/// Push a covector from From coordinates to To coordinates: alpha_to = S^-T alpha_from.
pub fn push_covector<From: CoordinateFrameV1, To: CoordinateFrameV1, const N: usize>(
    transform_to_from: &Matrix<N>,
    covector_from: &CovectorV1<From, N>,
    profile: MetricProfileV1,
) -> Result<CovectorV1<To, N>, MetricErrorV1> {
    CovectorV1::new(solve_dense(
        &transpose(transform_to_from),
        &covector_from.values,
        profile,
    )?)
}

/// Re-express metric from To coordinates in From coordinates: M_from = S^T M_to S.
pub fn pull_metric<From: CoordinateFrameV1, To: CoordinateFrameV1, const N: usize>(
    transform_to_from: &Matrix<N>,
    metric_to: &MetricV1<To, N>,
    profile: MetricProfileV1,
) -> Result<MetricV1<From, N>, MetricErrorV1> {
    validate_basis(transform_to_from, profile)?;
    let transformed = multiply(
        &multiply(&transpose(transform_to_from), &metric_to.matrix),
        transform_to_from,
    );
    MetricV1::new(transformed, profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct CartesianQ;
    impl CoordinateFrameV1 for CartesianQ {
        const FRAME_ID: &'static str = "cartesian-q";
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct DenseY;
    impl CoordinateFrameV1 for DenseY {
        const FRAME_ID: &'static str = "dense-y";
    }

    type QVec = TangentVectorV1<CartesianQ, 3>;
    type YVec = TangentVectorV1<DenseY, 3>;
    type QCov = CovectorV1<CartesianQ, 3>;
    type YCov = CovectorV1<DenseY, 3>;

    fn q_metric() -> MetricV1<CartesianQ, 3> {
        MetricV1::new(
            [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
            MetricProfileV1::default(),
        )
        .unwrap()
    }

    fn dense_basis() -> Matrix<3> {
        [[1.0, 5.0, -2.0], [0.0, 1.0, 4.0], [0.0, 0.0, 1.0]]
    }

    fn assert_close(left: f64, right: f64, tolerance: f64) {
        assert!(
            (left - right).abs() <= tolerance * left.abs().max(right.abs()).max(1.0),
            "left={left:.17e}, right={right:.17e}, tolerance={tolerance:.3e}"
        );
    }

    #[test]
    fn dense_rebasing_preserves_tangent_and_cotangent_metric_norms() {
        let profile = MetricProfileV1::default();
        let basis = dense_basis();
        let metric_q = q_metric();
        let metric_y: MetricV1<DenseY, 3> = pull_metric(&basis, &metric_q, profile).unwrap();
        let vector_y = YVec::new([0.25, -0.5, 1.25]).unwrap();
        let covector_q = QCov::new([1.5, -0.75, 2.0]).unwrap();
        let vector_q: QVec = push_tangent(&basis, &vector_y, profile).unwrap();
        let covector_y: YCov = pull_covector(&basis, &covector_q, profile).unwrap();

        assert_close(metric_q.tangent_norm(&vector_q).unwrap(), metric_y.tangent_norm(&vector_y).unwrap(), 2.0e-12);
        assert_close(metric_q.dual_norm(&covector_q).unwrap(), metric_y.dual_norm(&covector_y).unwrap(), 2.0e-12);
        assert_close(metric_q.pairing(&covector_q, &vector_q), metric_y.pairing(&covector_y, &vector_y), 2.0e-12);
    }

    #[test]
    fn raw_component_norm_changes_while_physical_norm_is_preserved() {
        let profile = MetricProfileV1::default();
        let basis = dense_basis();
        let metric_q = q_metric();
        let metric_y: MetricV1<DenseY, 3> = pull_metric(&basis, &metric_q, profile).unwrap();
        let vector_y = YVec::new([0.25, -0.5, 1.25]).unwrap();
        let vector_q: QVec = push_tangent(&basis, &vector_y, profile).unwrap();
        let raw_q = vector_q.values().iter().map(|value| value.abs()).fold(0.0, f64::max);
        let raw_y = vector_y.values().iter().map(|value| value.abs()).fold(0.0, f64::max);

        assert!((raw_q - raw_y).abs() > 1.0);
        assert_close(metric_q.tangent_norm(&vector_q).unwrap(), metric_y.tangent_norm(&vector_y).unwrap(), 2.0e-12);
    }

    #[test]
    fn wrong_covector_variance_law_breaks_pairing_invariance() {
        let profile = MetricProfileV1::default();
        let basis = dense_basis();
        let metric_q = q_metric();
        let metric_y: MetricV1<DenseY, 3> = pull_metric(&basis, &metric_q, profile).unwrap();
        let covector_q = QCov::new([1.5, -0.75, 2.0]).unwrap();
        let vector_y = YVec::new([0.25, -0.5, 1.25]).unwrap();
        let vector_q: QVec = push_tangent(&basis, &vector_y, profile).unwrap();
        let correct: YCov = pull_covector(&basis, &covector_q, profile).unwrap();
        let wrong: YCov = YCov::new(apply(&basis, covector_q.values())).unwrap();

        let correct_pairing = metric_y.pairing(&correct, &vector_y);
        let wrong_pairing = dot(wrong.values(), vector_y.values());
        assert!((correct_pairing - wrong_pairing).abs() > 1.0e-3);
        assert_close(correct_pairing, metric_q.pairing(&covector_q, &vector_q), 2.0e-12);
    }

    #[test]
    fn riesz_map_identifies_dual_residual_norm_with_tangent_error() {
        let metric = q_metric();
        let exact = QVec::new([0.5, -1.0, 0.25]).unwrap();
        let computed = QVec::new([0.5001, -0.9998, 0.2499]).unwrap();
        let error = QVec::new(std::array::from_fn(|index| exact.values()[index] - computed.values()[index])).unwrap();
        let residual = metric.lower(&error).unwrap();

        assert_close(metric.tangent_norm(&error).unwrap(), metric.dual_norm(&residual).unwrap(), 2.0e-13);
    }

    #[test]
    fn raise_and_lower_are_mutual_inverses_on_admitted_metric() {
        let metric = q_metric();
        let tangent = QVec::new([0.125, -0.75, 1.5]).unwrap();
        let covector = metric.lower(&tangent).unwrap();
        let recovered = metric.raise(&covector).unwrap();

        for index in 0..3 {
            assert_close(tangent.values()[index], recovered.values()[index], 2.0e-13);
        }
    }

    #[test]
    fn metric_rejects_asymmetry_indefiniteness_and_nonfinite_values() {
        assert_eq!(
            MetricV1::<CartesianQ, 2>::new([[1.0, 0.1], [0.0, 1.0]], MetricProfileV1::default()).unwrap_err(),
            MetricErrorV1::NotSymmetric
        );
        assert_eq!(
            MetricV1::<CartesianQ, 2>::new([[1.0, 2.0], [2.0, 1.0]], MetricProfileV1::default()).unwrap_err(),
            MetricErrorV1::NotPositiveDefinite
        );
        assert_eq!(
            MetricV1::<CartesianQ, 2>::new([[f64::NAN, 0.0], [0.0, 1.0]], MetricProfileV1::default()).unwrap_err(),
            MetricErrorV1::NonFiniteInput
        );
    }

    #[test]
    fn singular_basis_is_rejected_for_inverse_variance_maps() {
        let singular = [[1.0, 2.0, 3.0], [2.0, 4.0, 6.0], [0.0, 0.0, 1.0]];
        let vector = QVec::new([1.0, 2.0, 3.0]).unwrap();
        assert_eq!(
            pull_tangent::<DenseY, CartesianQ, 3>(&singular, &vector, MetricProfileV1::default()).unwrap_err(),
            MetricErrorV1::SingularBasis
        );
    }

    #[test]
    fn force_covector_pullback_and_pushforward_preserve_pairing() {
        let profile = MetricProfileV1::default();
        let basis = dense_basis();
        let covector_y = YCov::new([0.2, -0.7, 1.1]).unwrap();
        let vector_y = YVec::new([1.25, -0.5, 0.75]).unwrap();
        let covector_q: QCov = push_covector::<DenseY, CartesianQ, 3>(&basis, &covector_y, profile).unwrap();
        let vector_q: QVec = push_tangent(&basis, &vector_y, profile).unwrap();
        let restored_y: YCov = pull_covector::<DenseY, CartesianQ, 3>(&basis, &covector_q, profile).unwrap();

        assert_close(dot(covector_y.values(), vector_y.values()), dot(covector_q.values(), vector_q.values()), 2.0e-12);
        for index in 0..3 {
            assert_close(covector_y.values()[index], restored_y.values()[index], 2.0e-12);
        }
    }
}
