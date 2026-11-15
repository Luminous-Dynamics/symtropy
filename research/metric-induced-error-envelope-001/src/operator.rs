// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Metric-induced operator norms and condition estimates for small dense maps.
//!
//! For G_X = L_X L_X^T and G_Y = L_Y L_Y^T:
//!
//! - tangent -> tangent: B = L_Y^T A L_X^-T;
//! - tangent -> cotangent: B = L_Y^-1 A L_X^-T.
//!
//! The singular values of B are invariant under the corresponding coordinate
//! changes when the operator and both metrics are transformed with the correct
//! tensor variance. A one-sided Jacobi SVD avoids explicitly forming B^T B.
//!
//! This is a bounded square dense f64 reference. It is not an interval-certified
//! condition estimator and does not qualify nonlinear residual-to-state maps.

use core::marker::PhantomData;

use super::{CoordinateFrame, Matrix, Metric, MetricErrorV1};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OperatorConditionProfileV1 {
    pub relative_jacobi_tolerance: f64,
    pub relative_rank_tolerance: f64,
    pub maximum_condition_estimate: f64,
    pub maximum_jacobi_sweeps: usize,
}

impl Default for OperatorConditionProfileV1 {
    fn default() -> Self {
        Self {
            relative_jacobi_tolerance: 64.0 * f64::EPSILON,
            relative_rank_tolerance: 256.0 * f64::EPSILON,
            maximum_condition_estimate: 1.0e10,
            maximum_jacobi_sweeps: 32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorConditionStatusV1 {
    Admitted,
    OutsideConditionEnvelope,
    RankDeficient,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OperatorConditionReceiptV1 {
    /// Singular values of the whitened operator, in descending order.
    pub singular_values: Vec<f64>,
    pub largest_singular_value: f64,
    pub smallest_singular_value: f64,
    pub numerical_rank: usize,
    /// None when the numerical rank is below the square matrix dimension.
    pub condition_estimate: Option<f64>,
    pub maximum_normalized_column_correlation: f64,
    pub jacobi_sweeps: usize,
    pub status: OperatorConditionStatusV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperatorConditionErrorV1 {
    ZeroDimension,
    InvalidProfile,
    NonFiniteOperator,
    NonFiniteIntermediate,
    JacobiDidNotConverge,
    Metric(MetricErrorV1),
}

impl From<MetricErrorV1> for OperatorConditionErrorV1 {
    fn from(value: MetricErrorV1) -> Self {
        Self::Metric(value)
    }
}

impl core::fmt::Display for OperatorConditionErrorV1 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for OperatorConditionErrorV1 {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TangentToTangentOperator<
    Domain: CoordinateFrame,
    Codomain: CoordinateFrame,
    const N: usize,
> {
    matrix: Matrix<N>,
    _frames: PhantomData<(Domain, Codomain)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TangentToCotangentOperator<
    Domain: CoordinateFrame,
    Codomain: CoordinateFrame,
    const N: usize,
> {
    matrix: Matrix<N>,
    _frames: PhantomData<(Domain, Codomain)>,
}

fn validate_operator<const N: usize>(matrix: &Matrix<N>) -> Result<(), OperatorConditionErrorV1> {
    if N == 0 {
        return Err(OperatorConditionErrorV1::ZeroDimension);
    }
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(OperatorConditionErrorV1::NonFiniteOperator);
    }
    Ok(())
}

fn validate_profile(profile: OperatorConditionProfileV1) -> Result<(), OperatorConditionErrorV1> {
    if !profile.relative_jacobi_tolerance.is_finite()
        || profile.relative_jacobi_tolerance <= 0.0
        || profile.relative_jacobi_tolerance >= 1.0
        || !profile.relative_rank_tolerance.is_finite()
        || profile.relative_rank_tolerance <= 0.0
        || profile.relative_rank_tolerance >= 1.0
        || !profile.maximum_condition_estimate.is_finite()
        || profile.maximum_condition_estimate < 1.0
        || profile.maximum_jacobi_sweeps == 0
    {
        return Err(OperatorConditionErrorV1::InvalidProfile);
    }
    Ok(())
}

fn solve_lower_transpose<const N: usize>(
    lower: &Matrix<N>,
    rhs: &[f64; N],
) -> Result<[f64; N], OperatorConditionErrorV1> {
    let mut solution = [0.0; N];
    for row in (0..N).rev() {
        let mut value = rhs[row];
        for column in (row + 1)..N {
            value -= lower[column][row] * solution[column];
        }
        solution[row] = value / lower[row][row];
        if !solution[row].is_finite() {
            return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
        }
    }
    Ok(solution)
}

fn right_whiten_domain<const N: usize>(
    operator: &Matrix<N>,
    domain_lower: &Matrix<N>,
) -> Result<Matrix<N>, OperatorConditionErrorV1> {
    // C = A L_X^-T. Each column is A times the solution of
    // L_X^T z = e_j; no explicit inverse is materialized.
    let mut result = [[0.0; N]; N];
    for column in 0..N {
        let mut unit = [0.0; N];
        unit[column] = 1.0;
        let basis_column = solve_lower_transpose(domain_lower, &unit)?;
        for row in 0..N {
            let mut value = 0.0;
            for inner in 0..N {
                value += operator[row][inner] * basis_column[inner];
            }
            if !value.is_finite() {
                return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
            }
            result[row][column] = value;
        }
    }
    Ok(result)
}

fn whiten_tangent_to_tangent<Domain: CoordinateFrame, Codomain: CoordinateFrame, const N: usize>(
    operator: &Matrix<N>,
    domain: &Metric<Domain, N>,
    codomain: &Metric<Codomain, N>,
) -> Result<Matrix<N>, OperatorConditionErrorV1> {
    // B = L_Y^T A L_X^-T.
    let right_whitened = right_whiten_domain(operator, &domain.cholesky)?;
    let mut result = [[0.0; N]; N];
    for (row, result_row) in result.iter_mut().enumerate() {
        for (column, entry) in result_row.iter_mut().enumerate() {
            let mut value = 0.0;
            for (inner, right_row) in right_whitened.iter().enumerate().skip(row) {
                value += codomain.cholesky[inner][row] * right_row[column];
            }
            if !value.is_finite() {
                return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
            }
            *entry = value;
        }
    }
    Ok(result)
}

fn whiten_tangent_to_cotangent<
    Domain: CoordinateFrame,
    Codomain: CoordinateFrame,
    const N: usize,
>(
    operator: &Matrix<N>,
    domain: &Metric<Domain, N>,
    codomain: &Metric<Codomain, N>,
) -> Result<Matrix<N>, OperatorConditionErrorV1> {
    // B = L_Y^-1 A L_X^-T. Solve L_Y B = A L_X^-T.
    let right_whitened = right_whiten_domain(operator, &domain.cholesky)?;
    let mut result = [[0.0; N]; N];
    for column in 0..N {
        for row in 0..N {
            let mut value = right_whitened[row][column];
            for (inner, result_row) in result.iter().enumerate().take(row) {
                value -= codomain.cholesky[row][inner] * result_row[column];
            }
            result[row][column] = value / codomain.cholesky[row][row];
            if !result[row][column].is_finite() {
                return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
            }
        }
    }
    Ok(result)
}

fn normalized_column_correlation<const N: usize>(
    matrix: &Matrix<N>,
    left: usize,
    right: usize,
) -> Result<f64, OperatorConditionErrorV1> {
    let left_norm = (0..N).fold(0.0_f64, |norm, row| norm.hypot(matrix[row][left]));
    let right_norm = (0..N).fold(0.0_f64, |norm, row| norm.hypot(matrix[row][right]));
    if !left_norm.is_finite() || !right_norm.is_finite() {
        return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
    }
    if left_norm == 0.0 || right_norm == 0.0 {
        return Ok(0.0);
    }

    let mut correlation = 0.0;
    for matrix_row in matrix.iter().take(N) {
        correlation += (matrix_row[left] / left_norm) * (matrix_row[right] / right_norm);
    }
    if correlation.is_finite() {
        Ok(correlation)
    } else {
        Err(OperatorConditionErrorV1::NonFiniteIntermediate)
    }
}

fn singular_values_one_sided_jacobi<const N: usize>(
    matrix: &Matrix<N>,
    profile: OperatorConditionProfileV1,
) -> Result<(Vec<f64>, f64, usize), OperatorConditionErrorV1> {
    let mut work = *matrix;
    let mut sweeps = 0;
    let mut converged = false;

    for sweep in 0..profile.maximum_jacobi_sweeps {
        let mut rotated_any_pair = false;
        for left in 0..N {
            for right in (left + 1)..N {
                let left_norm = (0..N).fold(0.0_f64, |norm, row| norm.hypot(work[row][left]));
                let right_norm = (0..N).fold(0.0_f64, |norm, row| norm.hypot(work[row][right]));
                if !left_norm.is_finite() || !right_norm.is_finite() {
                    return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
                }
                if left_norm == 0.0 || right_norm == 0.0 {
                    continue;
                }

                let correlation = normalized_column_correlation(&work, left, right)?;
                if correlation.abs() <= profile.relative_jacobi_tolerance {
                    continue;
                }

                // Scale the 2x2 Gram block by max(norms)^2 before forming the
                // Jacobi rotation. This avoids squaring large column norms.
                let scale = left_norm.max(right_norm);
                let left_scaled = left_norm / scale;
                let right_scaled = right_norm / scale;
                let alpha = left_scaled * left_scaled;
                let beta = right_scaled * right_scaled;
                let gamma = correlation * left_scaled * right_scaled;
                let delta = 0.5 * (beta - alpha);
                let root = delta.hypot(gamma);
                let denominator = if delta >= 0.0 {
                    delta + root
                } else {
                    delta - root
                };
                let tangent = if denominator == 0.0 {
                    1.0
                } else {
                    gamma / denominator
                };
                let cosine = 1.0 / (1.0 + tangent * tangent).sqrt();
                let sine = cosine * tangent;
                if !cosine.is_finite() || !sine.is_finite() {
                    return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
                }

                for work_row in work.iter_mut().take(N) {
                    let old_left = work_row[left];
                    let old_right = work_row[right];
                    work_row[left] = cosine * old_left - sine * old_right;
                    work_row[right] = sine * old_left + cosine * old_right;
                    if !work_row[left].is_finite() || !work_row[right].is_finite() {
                        return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
                    }
                }
                rotated_any_pair = true;
            }
        }

        sweeps = sweep + 1;
        if !rotated_any_pair {
            converged = true;
            break;
        }
    }

    let mut maximum_correlation = 0.0_f64;
    for left in 0..N {
        for right in (left + 1)..N {
            maximum_correlation =
                maximum_correlation.max(normalized_column_correlation(&work, left, right)?.abs());
        }
    }
    if !converged && maximum_correlation > profile.relative_jacobi_tolerance {
        return Err(OperatorConditionErrorV1::JacobiDidNotConverge);
    }

    let mut singular_values: Vec<f64> = (0..N)
        .map(|column| (0..N).fold(0.0_f64, |norm, row| norm.hypot(work[row][column])))
        .collect();
    if singular_values.iter().any(|value| !value.is_finite()) {
        return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
    }
    singular_values.sort_by(|left, right| right.total_cmp(left));
    Ok((singular_values, maximum_correlation, sweeps))
}

fn assess_whitened<const N: usize>(
    whitened: Matrix<N>,
    profile: OperatorConditionProfileV1,
) -> Result<OperatorConditionReceiptV1, OperatorConditionErrorV1> {
    validate_profile(profile)?;
    let (singular_values, maximum_correlation, jacobi_sweeps) =
        singular_values_one_sided_jacobi(&whitened, profile)?;
    let largest = singular_values.first().copied().unwrap_or(0.0);
    let smallest = singular_values.last().copied().unwrap_or(0.0);
    let rank_threshold = largest * profile.relative_rank_tolerance;
    if !rank_threshold.is_finite() {
        return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
    }
    let numerical_rank = singular_values
        .iter()
        .filter(|value| **value > rank_threshold)
        .count();
    let condition_estimate = if numerical_rank == N && smallest > 0.0 {
        let estimate = largest / smallest;
        if estimate.is_finite() {
            Some(estimate)
        } else {
            return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
        }
    } else {
        None
    };

    let status = match condition_estimate {
        None => OperatorConditionStatusV1::RankDeficient,
        Some(condition) if condition > profile.maximum_condition_estimate => {
            OperatorConditionStatusV1::OutsideConditionEnvelope
        }
        Some(_) => OperatorConditionStatusV1::Admitted,
    };

    Ok(OperatorConditionReceiptV1 {
        singular_values,
        largest_singular_value: largest,
        smallest_singular_value: smallest,
        numerical_rank,
        condition_estimate,
        maximum_normalized_column_correlation: maximum_correlation,
        jacobi_sweeps,
        status,
    })
}

impl<Domain: CoordinateFrame, Codomain: CoordinateFrame, const N: usize>
    TangentToTangentOperator<Domain, Codomain, N>
{
    pub fn new(matrix: Matrix<N>) -> Result<Self, OperatorConditionErrorV1> {
        validate_operator(&matrix)?;
        Ok(Self {
            matrix,
            _frames: PhantomData,
        })
    }

    pub fn matrix(&self) -> &Matrix<N> {
        &self.matrix
    }

    /// Assess A: T_X -> T_Y using ||x||_(G_X) and ||y||_(G_Y).
    pub fn assess(
        &self,
        domain_metric: &Metric<Domain, N>,
        codomain_metric: &Metric<Codomain, N>,
        profile: OperatorConditionProfileV1,
    ) -> Result<OperatorConditionReceiptV1, OperatorConditionErrorV1> {
        let whitened = whiten_tangent_to_tangent(&self.matrix, domain_metric, codomain_metric)?;
        assess_whitened(whitened, profile)
    }
}

impl<Domain: CoordinateFrame, Codomain: CoordinateFrame, const N: usize>
    TangentToCotangentOperator<Domain, Codomain, N>
{
    pub fn new(matrix: Matrix<N>) -> Result<Self, OperatorConditionErrorV1> {
        validate_operator(&matrix)?;
        Ok(Self {
            matrix,
            _frames: PhantomData,
        })
    }

    pub fn matrix(&self) -> &Matrix<N> {
        &self.matrix
    }

    /// Assess A: T_X -> T_Y* using ||x||_(G_X) and ||alpha||_(G_Y^-1).
    pub fn assess(
        &self,
        domain_metric: &Metric<Domain, N>,
        codomain_metric: &Metric<Codomain, N>,
        profile: OperatorConditionProfileV1,
    ) -> Result<OperatorConditionReceiptV1, OperatorConditionErrorV1> {
        let whitened = whiten_tangent_to_cotangent(&self.matrix, domain_metric, codomain_metric)?;
        assess_whitened(whitened, profile)
    }
}

/// A dense rectangular operator matrix with explicit row and column dimensions.
pub type RectMatrix<const ROWS: usize, const COLS: usize> = [[f64; COLS]; ROWS];

/// Spectrum and rank receipt for a rectangular map between metric spaces.
///
/// `singular_values` contains min(rows, columns) values. The condition estimate
/// is the ratio of the largest to smallest *nonzero-space* singular values and
/// is present only when the numerical rank reaches min(rows, columns). Kernel
/// directions and codomain cokernel directions are reported separately.
#[derive(Debug, Clone, PartialEq)]
pub struct RectangularOperatorConditionReceiptV1 {
    pub row_dimension: usize,
    pub column_dimension: usize,
    pub maximum_rank: usize,
    pub singular_values: Vec<f64>,
    pub largest_singular_value: f64,
    pub smallest_singular_value: f64,
    pub numerical_rank: usize,
    pub domain_nullity: usize,
    pub codomain_cokernel_dimension: usize,
    pub condition_estimate: Option<f64>,
    pub maximum_normalized_column_correlation: f64,
    pub jacobi_sweeps: usize,
    pub status: OperatorConditionStatusV1,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TangentToTangentRectOperator<
    Domain: CoordinateFrame,
    Codomain: CoordinateFrame,
    const ROWS: usize,
    const COLS: usize,
> {
    matrix: RectMatrix<ROWS, COLS>,
    _frames: PhantomData<(Domain, Codomain)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TangentToCotangentRectOperator<
    Domain: CoordinateFrame,
    Codomain: CoordinateFrame,
    const ROWS: usize,
    const COLS: usize,
> {
    matrix: RectMatrix<ROWS, COLS>,
    _frames: PhantomData<(Domain, Codomain)>,
}

fn validate_rect_operator<const ROWS: usize, const COLS: usize>(
    matrix: &RectMatrix<ROWS, COLS>,
) -> Result<(), OperatorConditionErrorV1> {
    if ROWS == 0 || COLS == 0 {
        return Err(OperatorConditionErrorV1::ZeroDimension);
    }
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(OperatorConditionErrorV1::NonFiniteOperator);
    }
    Ok(())
}

fn right_whiten_domain_rect<const ROWS: usize, const COLS: usize>(
    operator: &RectMatrix<ROWS, COLS>,
    domain_lower: &Matrix<COLS>,
) -> Result<RectMatrix<ROWS, COLS>, OperatorConditionErrorV1> {
    // C = A L_X^-T. Each column is A times the solution of L_X^T z = e_j.
    let mut result = [[0.0; COLS]; ROWS];
    for column in 0..COLS {
        let mut unit = [0.0; COLS];
        unit[column] = 1.0;
        let basis_column = solve_lower_transpose(domain_lower, &unit)?;
        for (row, output_row) in result.iter_mut().enumerate() {
            let value = (0..COLS)
                .map(|inner| operator[row][inner] * basis_column[inner])
                .sum::<f64>();
            if !value.is_finite() {
                return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
            }
            output_row[column] = value;
        }
    }
    Ok(result)
}

fn whiten_tangent_to_tangent_rect<
    Domain: CoordinateFrame,
    Codomain: CoordinateFrame,
    const ROWS: usize,
    const COLS: usize,
>(
    operator: &RectMatrix<ROWS, COLS>,
    domain: &Metric<Domain, COLS>,
    codomain: &Metric<Codomain, ROWS>,
) -> Result<RectMatrix<ROWS, COLS>, OperatorConditionErrorV1> {
    // B = L_Y^T A L_X^-T.
    let right_whitened = right_whiten_domain_rect(operator, &domain.cholesky)?;
    let mut result = [[0.0; COLS]; ROWS];
    for (row, output_row) in result.iter_mut().enumerate() {
        for (column, output) in output_row.iter_mut().enumerate() {
            let value = (row..ROWS)
                .map(|inner| codomain.cholesky[inner][row] * right_whitened[inner][column])
                .sum::<f64>();
            if !value.is_finite() {
                return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
            }
            *output = value;
        }
    }
    Ok(result)
}

fn whiten_tangent_to_cotangent_rect<
    Domain: CoordinateFrame,
    Codomain: CoordinateFrame,
    const ROWS: usize,
    const COLS: usize,
>(
    operator: &RectMatrix<ROWS, COLS>,
    domain: &Metric<Domain, COLS>,
    codomain: &Metric<Codomain, ROWS>,
) -> Result<RectMatrix<ROWS, COLS>, OperatorConditionErrorV1> {
    // B = L_Y^-1 A L_X^-T. Solve L_Y B = A L_X^-T column by column.
    let right_whitened = right_whiten_domain_rect(operator, &domain.cholesky)?;
    let mut result = [[0.0; COLS]; ROWS];
    for column in 0..COLS {
        for row in 0..ROWS {
            let correction = (0..row)
                .map(|inner| codomain.cholesky[row][inner] * result[inner][column])
                .sum::<f64>();
            let value = (right_whitened[row][column] - correction)
                / codomain.cholesky[row][row];
            if !value.is_finite() {
                return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
            }
            result[row][column] = value;
        }
    }
    Ok(result)
}

fn normalized_column_correlation_dynamic(
    matrix: &[Vec<f64>],
    left: usize,
    right: usize,
) -> Result<f64, OperatorConditionErrorV1> {
    let left_norm = matrix
        .iter()
        .fold(0.0_f64, |norm, row| norm.hypot(row[left]));
    let right_norm = matrix
        .iter()
        .fold(0.0_f64, |norm, row| norm.hypot(row[right]));
    if !left_norm.is_finite() || !right_norm.is_finite() {
        return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
    }
    if left_norm == 0.0 || right_norm == 0.0 {
        return Ok(0.0);
    }
    let correlation = matrix
        .iter()
        .map(|row| (row[left] / left_norm) * (row[right] / right_norm))
        .sum::<f64>();
    if correlation.is_finite() {
        Ok(correlation)
    } else {
        Err(OperatorConditionErrorV1::NonFiniteIntermediate)
    }
}

/// One-sided Jacobi SVD for a dynamic rows-by-columns matrix with columns <= rows.
fn singular_values_one_sided_jacobi_dynamic(
    mut work: Vec<Vec<f64>>,
    profile: OperatorConditionProfileV1,
) -> Result<(Vec<f64>, f64, usize), OperatorConditionErrorV1> {
    let rows = work.len();
    let columns = work.first().map_or(0, Vec::len);
    if rows == 0 || columns == 0 || columns > rows {
        return Err(OperatorConditionErrorV1::ZeroDimension);
    }
    if work.iter().any(|row| row.len() != columns || row.iter().any(|v| !v.is_finite())) {
        return Err(OperatorConditionErrorV1::NonFiniteOperator);
    }

    let mut sweeps = 0;
    let mut converged = false;
    for sweep in 0..profile.maximum_jacobi_sweeps {
        let mut rotated_any_pair = false;
        for left in 0..columns {
            for right in (left + 1)..columns {
                let correlation =
                    normalized_column_correlation_dynamic(&work, left, right)?;
                if correlation.abs() <= profile.relative_jacobi_tolerance {
                    continue;
                }

                let left_norm = work
                    .iter()
                    .fold(0.0_f64, |norm, row| norm.hypot(row[left]));
                let right_norm = work
                    .iter()
                    .fold(0.0_f64, |norm, row| norm.hypot(row[right]));
                if !left_norm.is_finite() || !right_norm.is_finite() {
                    return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
                }
                if left_norm == 0.0 || right_norm == 0.0 {
                    continue;
                }

                // Scale the 2x2 Gram block before forming its Jacobi rotation.
                let scale = left_norm.max(right_norm);
                let left_scaled = left_norm / scale;
                let right_scaled = right_norm / scale;
                let alpha = left_scaled * left_scaled;
                let beta = right_scaled * right_scaled;
                let gamma = correlation * left_scaled * right_scaled;
                let delta = 0.5 * (beta - alpha);
                let root = delta.hypot(gamma);
                let denominator = if delta >= 0.0 {
                    delta + root
                } else {
                    delta - root
                };
                let tangent = if denominator == 0.0 {
                    1.0
                } else {
                    gamma / denominator
                };
                let cosine = 1.0 / (1.0 + tangent * tangent).sqrt();
                let sine = cosine * tangent;
                if !cosine.is_finite() || !sine.is_finite() {
                    return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
                }

                for row in &mut work {
                    let old_left = row[left];
                    let old_right = row[right];
                    row[left] = cosine * old_left - sine * old_right;
                    row[right] = sine * old_left + cosine * old_right;
                    if !row[left].is_finite() || !row[right].is_finite() {
                        return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
                    }
                }
                rotated_any_pair = true;
            }
        }
        sweeps = sweep + 1;
        if !rotated_any_pair {
            converged = true;
            break;
        }
    }

    let mut maximum_correlation = 0.0_f64;
    for left in 0..columns {
        for right in (left + 1)..columns {
            maximum_correlation = maximum_correlation.max(
                normalized_column_correlation_dynamic(&work, left, right)?.abs(),
            );
        }
    }
    if !converged && maximum_correlation > profile.relative_jacobi_tolerance {
        return Err(OperatorConditionErrorV1::JacobiDidNotConverge);
    }

    let mut singular_values: Vec<f64> = (0..columns)
        .map(|column| {
            work.iter()
                .fold(0.0_f64, |norm, row| norm.hypot(row[column]))
        })
        .collect();
    if singular_values.iter().any(|value| !value.is_finite()) {
        return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
    }
    singular_values.sort_by(|left, right| right.total_cmp(left));
    Ok((singular_values, maximum_correlation, sweeps))
}

fn rectangular_singular_values<const ROWS: usize, const COLS: usize>(
    matrix: &RectMatrix<ROWS, COLS>,
    profile: OperatorConditionProfileV1,
) -> Result<(Vec<f64>, f64, usize), OperatorConditionErrorV1> {
    // One-sided Jacobi needs columns <= rows. Transpose wide operators so the
    // returned spectrum always contains exactly min(ROWS, COLS) values.
    let reduced: Vec<Vec<f64>> = if COLS <= ROWS {
        matrix.iter().map(|row| row.to_vec()).collect()
    } else {
        (0..COLS)
            .map(|row| (0..ROWS).map(|column| matrix[column][row]).collect())
            .collect()
    };
    singular_values_one_sided_jacobi_dynamic(reduced, profile)
}

fn assess_rectangular_whitened<const ROWS: usize, const COLS: usize>(
    whitened: RectMatrix<ROWS, COLS>,
    profile: OperatorConditionProfileV1,
) -> Result<RectangularOperatorConditionReceiptV1, OperatorConditionErrorV1> {
    validate_profile(profile)?;
    if ROWS == 0 || COLS == 0 {
        return Err(OperatorConditionErrorV1::ZeroDimension);
    }
    let (singular_values, maximum_correlation, jacobi_sweeps) =
        rectangular_singular_values(&whitened, profile)?;
    let largest = singular_values.first().copied().unwrap_or(0.0);
    let smallest = singular_values.last().copied().unwrap_or(0.0);
    let rank_threshold = largest * profile.relative_rank_tolerance;
    if !rank_threshold.is_finite() {
        return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
    }
    let numerical_rank = singular_values
        .iter()
        .filter(|value| **value > rank_threshold)
        .count();
    let maximum_rank = ROWS.min(COLS);
    let condition_estimate = if numerical_rank == maximum_rank && smallest > 0.0 {
        let estimate = largest / smallest;
        if estimate.is_finite() {
            Some(estimate)
        } else {
            return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
        }
    } else {
        None
    };

    let status = match condition_estimate {
        None => OperatorConditionStatusV1::RankDeficient,
        Some(condition) if condition > profile.maximum_condition_estimate => {
            OperatorConditionStatusV1::OutsideConditionEnvelope
        }
        Some(_) => OperatorConditionStatusV1::Admitted,
    };

    Ok(RectangularOperatorConditionReceiptV1 {
        row_dimension: ROWS,
        column_dimension: COLS,
        maximum_rank,
        singular_values,
        largest_singular_value: largest,
        smallest_singular_value: smallest,
        numerical_rank,
        domain_nullity: COLS - numerical_rank,
        codomain_cokernel_dimension: ROWS - numerical_rank,
        condition_estimate,
        maximum_normalized_column_correlation: maximum_correlation,
        jacobi_sweeps,
        status,
    })
}

impl<Domain: CoordinateFrame, Codomain: CoordinateFrame, const ROWS: usize, const COLS: usize>
    TangentToTangentRectOperator<Domain, Codomain, ROWS, COLS>
{
    pub fn new(
        matrix: RectMatrix<ROWS, COLS>,
    ) -> Result<Self, OperatorConditionErrorV1> {
        validate_rect_operator(&matrix)?;
        Ok(Self {
            matrix,
            _frames: PhantomData,
        })
    }

    pub fn matrix(&self) -> &RectMatrix<ROWS, COLS> {
        &self.matrix
    }

    /// Assess A: T_X -> T_Y using ||x||_(G_X) and ||y||_(G_Y).
    pub fn assess(
        &self,
        domain_metric: &Metric<Domain, COLS>,
        codomain_metric: &Metric<Codomain, ROWS>,
        profile: OperatorConditionProfileV1,
    ) -> Result<RectangularOperatorConditionReceiptV1, OperatorConditionErrorV1> {
        let whitened =
            whiten_tangent_to_tangent_rect(&self.matrix, domain_metric, codomain_metric)?;
        assess_rectangular_whitened(whitened, profile)
    }
}

impl<Domain: CoordinateFrame, Codomain: CoordinateFrame, const ROWS: usize, const COLS: usize>
    TangentToCotangentRectOperator<Domain, Codomain, ROWS, COLS>
{
    pub fn new(
        matrix: RectMatrix<ROWS, COLS>,
    ) -> Result<Self, OperatorConditionErrorV1> {
        validate_rect_operator(&matrix)?;
        Ok(Self {
            matrix,
            _frames: PhantomData,
        })
    }

    pub fn matrix(&self) -> &RectMatrix<ROWS, COLS> {
        &self.matrix
    }

    /// Assess A: T_X -> T_Y* using ||x||_(G_X) and ||alpha||_(G_Y^-1).
    pub fn assess(
        &self,
        domain_metric: &Metric<Domain, COLS>,
        codomain_metric: &Metric<Codomain, ROWS>,
        profile: OperatorConditionProfileV1,
    ) -> Result<RectangularOperatorConditionReceiptV1, OperatorConditionErrorV1> {
        let whitened =
            whiten_tangent_to_cotangent_rect(&self.matrix, domain_metric, codomain_metric)?;
        assess_rectangular_whitened(whitened, profile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CartesianQ, DenseY, MetricProfileV1, Rebasis};

    fn metric_profile() -> MetricProfileV1 {
        MetricProfileV1::default()
    }

    fn operator_profile() -> OperatorConditionProfileV1 {
        OperatorConditionProfileV1 {
            relative_jacobi_tolerance: 1.0e-13,
            relative_rank_tolerance: 1.0e-12,
            maximum_condition_estimate: 1.0e12,
            maximum_jacobi_sweeps: 48,
        }
    }

    fn close(left: f64, right: f64, tolerance: f64) {
        let scale = 1.0_f64.max(left.abs()).max(right.abs());
        assert!(
            (left - right).abs() <= tolerance * scale,
            "left={left:.16e}, right={right:.16e}, tolerance={tolerance:.3e}"
        );
    }

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

    fn transpose<const N: usize>(matrix: &Matrix<N>) -> Matrix<N> {
        let mut result = [[0.0; N]; N];
        for row in 0..N {
            for column in 0..N {
                result[row][column] = matrix[column][row];
            }
        }
        result
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct ConstraintFrame;
    impl CoordinateFrame for ConstraintFrame {}

    fn invert_spd_2(matrix: &[[f64; 2]; 2]) -> [[f64; 2]; 2] {
        let determinant = matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];
        assert!(determinant.is_finite() && determinant > 0.0);
        [
            [matrix[1][1] / determinant, -matrix[0][1] / determinant],
            [-matrix[1][0] / determinant, matrix[0][0] / determinant],
        ]
    }

    #[test]
    fn full_row_rank_constraint_jacobian_is_a_metric_coisometry() {
        use crate::DelassusMetricV1;

        let mass = Metric::<CartesianQ, 3>::admit(
            [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
            metric_profile(),
        )
        .unwrap();
        let jacobian = [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0]];
        let delassus =
            DelassusMetricV1::<CartesianQ, ConstraintFrame, 3, 2>::from_mass_jacobian(
                &mass,
                jacobian,
                metric_profile(),
            )
            .unwrap();
        let constraint_metric = Metric::<ConstraintFrame, 2>::admit(
            invert_spd_2(delassus.matrix()),
            metric_profile(),
        )
        .unwrap();

        let operator =
            TangentToTangentRectOperator::<CartesianQ, ConstraintFrame, 2, 3>::new(jacobian)
                .unwrap();
        let receipt = operator
            .assess(&mass, &constraint_metric, operator_profile())
            .unwrap();

        assert_eq!(receipt.row_dimension, 2);
        assert_eq!(receipt.column_dimension, 3);
        assert_eq!(receipt.maximum_rank, 2);
        assert_eq!(receipt.numerical_rank, 2);
        assert_eq!(receipt.domain_nullity, 1);
        assert_eq!(receipt.codomain_cokernel_dimension, 0);
        assert_eq!(receipt.status, OperatorConditionStatusV1::Admitted);
        close(receipt.singular_values[0], 1.0, 2.0e-12);
        close(receipt.singular_values[1], 1.0, 2.0e-12);
        close(receipt.condition_estimate.unwrap(), 1.0, 2.0e-12);
    }

    #[test]
    fn constraint_jacobian_coisometry_is_invariant_under_dense_coordinate_rebasing() {
        use crate::{DelassusMetricV1, rebase_constraint_jacobian};

        let mass_q = Metric::<CartesianQ, 3>::admit(
            [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
            metric_profile(),
        )
        .unwrap();
        let jacobian_q = [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0]];
        let rebasis = Rebasis::<CartesianQ, DenseY, 3>::new(
            [[1.0, 2.0, 1.0], [0.0, 1.0, 3.0], [0.0, 0.0, 1.0]],
            metric_profile(),
        )
        .unwrap();
        let mass_y = rebasis.metric_to_to(&mass_q, metric_profile()).unwrap();
        let jacobian_y = rebase_constraint_jacobian(&jacobian_q, &rebasis).unwrap();

        let delassus_q =
            DelassusMetricV1::<CartesianQ, ConstraintFrame, 3, 2>::from_mass_jacobian(
                &mass_q,
                jacobian_q,
                metric_profile(),
            )
            .unwrap();
        let delassus_y =
            DelassusMetricV1::<DenseY, ConstraintFrame, 3, 2>::from_mass_jacobian(
                &mass_y,
                jacobian_y,
                metric_profile(),
            )
            .unwrap();
        let constraint_metric_q = Metric::<ConstraintFrame, 2>::admit(
            invert_spd_2(delassus_q.matrix()),
            metric_profile(),
        )
        .unwrap();
        let constraint_metric_y = Metric::<ConstraintFrame, 2>::admit(
            invert_spd_2(delassus_y.matrix()),
            metric_profile(),
        )
        .unwrap();

        let receipt_q = TangentToTangentRectOperator::<CartesianQ, ConstraintFrame, 2, 3>::new(
            jacobian_q,
        )
        .unwrap()
        .assess(&mass_q, &constraint_metric_q, operator_profile())
        .unwrap();
        let receipt_y = TangentToTangentRectOperator::<DenseY, ConstraintFrame, 2, 3>::new(
            jacobian_y,
        )
        .unwrap()
        .assess(&mass_y, &constraint_metric_y, operator_profile())
        .unwrap();

        assert_eq!(receipt_q.numerical_rank, receipt_y.numerical_rank);
        assert_eq!(receipt_q.domain_nullity, receipt_y.domain_nullity);
        for (left, right) in receipt_q
            .singular_values
            .iter()
            .zip(receipt_y.singular_values.iter())
        {
            close(*left, *right, 3.0e-11);
        }
        close(
            receipt_q.condition_estimate.unwrap(),
            receipt_y.condition_estimate.unwrap(),
            3.0e-11,
        );
    }

    #[test]
    fn rank_deficient_rectangular_operator_reports_kernel_and_cokernel_separately() {
        let domain = Metric::<CartesianQ, 3>::admit(
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            metric_profile(),
        )
        .unwrap();
        let codomain = Metric::<ConstraintFrame, 2>::admit(
            [[1.0, 0.0], [0.0, 1.0]],
            metric_profile(),
        )
        .unwrap();
        let operator =
            TangentToTangentRectOperator::<CartesianQ, ConstraintFrame, 2, 3>::new([
                [1.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
            ])
            .unwrap();
        let receipt = operator
            .assess(&domain, &codomain, operator_profile())
            .unwrap();

        assert_eq!(receipt.maximum_rank, 2);
        assert_eq!(receipt.numerical_rank, 1);
        assert_eq!(receipt.domain_nullity, 2);
        assert_eq!(receipt.codomain_cokernel_dimension, 1);
        assert_eq!(receipt.condition_estimate, None);
        assert_eq!(receipt.status, OperatorConditionStatusV1::RankDeficient);
    }

    #[test]
    fn tall_rectangular_operator_uses_transposed_jacobi_path() {
        let domain =
            Metric::<CartesianQ, 2>::admit([[1.0, 0.0], [0.0, 1.0]], metric_profile()).unwrap();
        let codomain =
            Metric::<DenseY, 3>::admit(
                [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                metric_profile(),
            )
            .unwrap();
        let operator =
            TangentToTangentRectOperator::<CartesianQ, DenseY, 3, 2>::new([
                [3.0, 0.0],
                [0.0, 2.0],
                [0.0, 0.0],
            ])
            .unwrap();
        let receipt = operator
            .assess(&domain, &codomain, operator_profile())
            .unwrap();

        assert_eq!(receipt.numerical_rank, 2);
        assert_eq!(receipt.domain_nullity, 0);
        assert_eq!(receipt.codomain_cokernel_dimension, 1);
        close(receipt.singular_values[0], 3.0, 1.0e-12);
        close(receipt.singular_values[1], 2.0, 1.0e-12);
        close(receipt.condition_estimate.unwrap(), 1.5, 1.0e-12);
    }

    #[test]
    fn rectangular_tangent_to_cotangent_uses_codomain_dual_metric() {
        let domain =
            Metric::<CartesianQ, 3>::admit(
                [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                metric_profile(),
            )
            .unwrap();
        let codomain =
            Metric::<DenseY, 2>::admit([[4.0, 0.0], [0.0, 9.0]], metric_profile()).unwrap();
        let operator =
            TangentToCotangentRectOperator::<CartesianQ, DenseY, 2, 3>::new([
                [2.0, 0.0, 0.0],
                [0.0, 6.0, 0.0],
            ])
            .unwrap();
        let receipt = operator
            .assess(&domain, &codomain, operator_profile())
            .unwrap();

        assert_eq!(receipt.numerical_rank, 2);
        close(receipt.singular_values[0], 2.0, 1.0e-12);
        close(receipt.singular_values[1], 2.0, 1.0e-12);
        close(receipt.condition_estimate.unwrap(), 1.0, 1.0e-12);
    }

    #[test]
    fn mass_riesz_operator_has_unit_spectrum_in_paired_metric_spaces() {
        let metric = Metric::<CartesianQ, 3>::admit(
            [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
            metric_profile(),
        )
        .unwrap();
        let operator =
            TangentToCotangentOperator::<CartesianQ, CartesianQ, 3>::new(*metric.matrix()).unwrap();
        let receipt = operator
            .assess(&metric, &metric, operator_profile())
            .unwrap();

        assert_eq!(receipt.status, OperatorConditionStatusV1::Admitted);
        assert_eq!(receipt.numerical_rank, 3);
        close(receipt.condition_estimate.unwrap(), 1.0, 1.0e-12);
        for value in receipt.singular_values {
            close(value, 1.0, 1.0e-12);
        }
    }

    #[test]
    fn tangent_operator_spectrum_matches_known_weighted_diagonal_map() {
        let domain =
            Metric::<CartesianQ, 2>::admit([[1.0, 0.0], [0.0, 4.0]], metric_profile()).unwrap();
        let codomain =
            Metric::<DenseY, 2>::admit([[9.0, 0.0], [0.0, 16.0]], metric_profile()).unwrap();
        let identity =
            TangentToTangentOperator::<CartesianQ, DenseY, 2>::new([[1.0, 0.0], [0.0, 1.0]])
                .unwrap();
        let receipt = identity
            .assess(&domain, &codomain, operator_profile())
            .unwrap();

        close(receipt.singular_values[0], 3.0, 1.0e-12);
        close(receipt.singular_values[1], 2.0, 1.0e-12);
        close(receipt.condition_estimate.unwrap(), 1.5, 1.0e-12);
    }

    #[test]
    fn rank_deficient_operator_never_receives_a_finite_condition_claim() {
        let metric =
            Metric::<CartesianQ, 2>::admit([[1.0, 0.0], [0.0, 1.0]], metric_profile()).unwrap();
        let operator =
            TangentToCotangentOperator::<CartesianQ, CartesianQ, 2>::new([[1.0, 0.0], [0.0, 0.0]])
                .unwrap();
        let receipt = operator
            .assess(&metric, &metric, operator_profile())
            .unwrap();

        assert_eq!(receipt.numerical_rank, 1);
        assert_eq!(receipt.condition_estimate, None);
        assert_eq!(receipt.status, OperatorConditionStatusV1::RankDeficient);
    }

    #[test]
    fn tangent_to_cotangent_singular_spectrum_is_invariant_under_dense_rebasing() {
        let metric_domain_q = Metric::<CartesianQ, 3>::admit(
            [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
            metric_profile(),
        )
        .unwrap();
        let metric_codomain_q = Metric::<CartesianQ, 3>::admit(
            [[4.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 6.0]],
            metric_profile(),
        )
        .unwrap();
        let rebasis = Rebasis::<CartesianQ, DenseY, 3>::new(
            [[1.0, 4.0, -2.0], [0.0, 1.0, 3.0], [0.0, 0.0, 1.0]],
            metric_profile(),
        )
        .unwrap();
        let metric_domain_y = rebasis
            .metric_to_to(&metric_domain_q, metric_profile())
            .unwrap();
        let metric_codomain_y = rebasis
            .metric_to_to(&metric_codomain_q, metric_profile())
            .unwrap();

        let operator_q_matrix = [[2.0, 1.0, -0.5], [0.25, 3.0, 0.75], [-1.0, 0.5, 2.0]];
        // For q = S y and A: T_q -> T_q*, the represented bilinear map is
        // A_y = S^T A_q S. Both spaces change frame, but their variance differs.
        let operator_y_matrix = multiply(
            &multiply(&transpose(rebasis.matrix()), &operator_q_matrix),
            rebasis.matrix(),
        );
        let operator_q =
            TangentToCotangentOperator::<CartesianQ, CartesianQ, 3>::new(operator_q_matrix)
                .unwrap();
        let operator_y =
            TangentToCotangentOperator::<DenseY, DenseY, 3>::new(operator_y_matrix).unwrap();

        let receipt_q = operator_q
            .assess(&metric_domain_q, &metric_codomain_q, operator_profile())
            .unwrap();
        let receipt_y = operator_y
            .assess(&metric_domain_y, &metric_codomain_y, operator_profile())
            .unwrap();

        assert_eq!(receipt_q.numerical_rank, receipt_y.numerical_rank);
        for (left, right) in receipt_q
            .singular_values
            .iter()
            .zip(receipt_y.singular_values.iter())
        {
            close(*left, *right, 2.0e-10);
        }
        close(
            receipt_q.condition_estimate.unwrap(),
            receipt_y.condition_estimate.unwrap(),
            2.0e-10,
        );
    }

    #[test]
    fn wrong_operator_variance_transform_changes_the_condition_receipt() {
        let metric_q = Metric::<CartesianQ, 3>::admit(
            [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
            metric_profile(),
        )
        .unwrap();
        let rebasis = Rebasis::<CartesianQ, DenseY, 3>::new(
            [[1.0, 5.0, 2.0], [0.0, 1.0, 4.0], [0.0, 0.0, 1.0]],
            metric_profile(),
        )
        .unwrap();
        let metric_y = rebasis.metric_to_to(&metric_q, metric_profile()).unwrap();
        let a_q = [[2.0, 1.0, 0.0], [0.25, 3.0, -1.0], [2.0, 0.5, 1.0]];
        let s_t = transpose(rebasis.matrix());
        let correct = multiply(&multiply(&s_t, &a_q), rebasis.matrix());
        let wrong = multiply(&multiply(rebasis.matrix(), &a_q), &s_t);

        let correct_receipt = TangentToCotangentOperator::<DenseY, DenseY, 3>::new(correct)
            .unwrap()
            .assess(&metric_y, &metric_y, operator_profile())
            .unwrap();
        let wrong_receipt = TangentToCotangentOperator::<DenseY, DenseY, 3>::new(wrong)
            .unwrap()
            .assess(&metric_y, &metric_y, operator_profile())
            .unwrap();
        assert!(
            (correct_receipt.condition_estimate.unwrap()
                - wrong_receipt.condition_estimate.unwrap())
            .abs()
                > 1.0e-3
        );
    }
}
