// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Metric-induced singular spectra for rectangular operators.
//!
//! For B : T_X -> T_Y, whiten:
//!   C = L_Y^T B L_X^-T
//! For B : T_X -> T_Y*, whiten:
//!   C = L_Y^-1 B L_X^-T
//!
//! A rectangular map can be full row rank yet have a non-trivial domain
//! nullspace. In that case the nonzero singular spectrum is useful, but the
//! whole-domain condition number is undefined. This module reports both
//! separately and never substitutes a pseudoinverse condition for the map's
//! ordinary condition number.

use core::marker::PhantomData;

use super::operator::{OperatorConditionErrorV1, OperatorConditionProfileV1};
use super::{CoordinateFrame, Matrix, Metric};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RectangularOperatorStatusV1 {
    Admitted,
    OutsideConditionEnvelope,
    RankDeficient,
    NonInjective,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RectangularOperatorConditionReceiptV1 {
    /// Singular values in descending order. Exactly min(rows, columns) values
    /// are reported; rank-deficient maps may have zeros at the tail.
    pub singular_values: Vec<f64>,
    pub largest_singular_value: f64,
    pub smallest_singular_value: f64,
    pub smallest_nonzero_singular_value: Option<f64>,
    pub numerical_rank: usize,
    pub domain_dimension: usize,
    pub codomain_dimension: usize,
    pub domain_nullity: usize,
    pub codomain_nullity: usize,
    /// The ordinary operator condition is defined only for full-column-rank
    /// maps (injective maps) in finite-dimensional normed spaces.
    pub condition_estimate: Option<f64>,
    /// Ratio of largest to smallest nonzero singular value. This is the
    /// condition of the restriction to the numerically resolved range; it is
    /// not the whole-domain condition of a non-injective map.
    pub nonzero_spectrum_condition_estimate: Option<f64>,
    pub maximum_normalized_column_correlation: f64,
    pub jacobi_sweeps: usize,
    pub status: RectangularOperatorStatusV1,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectangularTangentToTangentOperator<
    Domain: CoordinateFrame,
    Codomain: CoordinateFrame,
    const K: usize,
    const N: usize,
> {
    /// K-by-N map from domain tangent components into codomain tangent components.
    matrix: [[f64; N]; K],
    _frames: PhantomData<(Domain, Codomain)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectangularTangentToCotangentOperator<
    Domain: CoordinateFrame,
    Codomain: CoordinateFrame,
    const K: usize,
    const N: usize,
> {
    /// K-by-N map from domain tangent components into codomain covector components.
    matrix: [[f64; N]; K],
    _frames: PhantomData<(Domain, Codomain)>,
}

fn validate_rectangular_operator<const K: usize, const N: usize>(
    matrix: &[[f64; N]; K],
) -> Result<(), OperatorConditionErrorV1> {
    if K == 0 || N == 0 {
        return Err(OperatorConditionErrorV1::ZeroDimension);
    }
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(OperatorConditionErrorV1::NonFiniteOperator);
    }
    Ok(())
}

fn validate_rectangular_profile(
    profile: OperatorConditionProfileV1,
) -> Result<(), OperatorConditionErrorV1> {
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

/// Right-whiten A by the domain metric: C = A L_X^-T.
/// Each column is formed by a triangular solve; no inverse is materialized.
fn right_whiten_domain<const K: usize, const N: usize>(
    operator: &[[f64; N]; K],
    domain_lower: &Matrix<N>,
) -> Result<[[f64; N]; K], OperatorConditionErrorV1> {
    let mut result = [[0.0; N]; K];
    for column in 0..N {
        let mut unit = [0.0; N];
        unit[column] = 1.0;
        let basis_column = solve_lower_transpose(domain_lower, &unit)?;
        for row in 0..K {
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

fn whiten_tangent_to_tangent<
    Domain: CoordinateFrame,
    Codomain: CoordinateFrame,
    const K: usize,
    const N: usize,
>(
    operator: &[[f64; N]; K],
    domain: &Metric<Domain, N>,
    codomain: &Metric<Codomain, K>,
) -> Result<[[f64; N]; K], OperatorConditionErrorV1> {
    // C = L_Y^T A L_X^-T.
    let right = right_whiten_domain(operator, &domain.cholesky)?;
    let mut result = [[0.0; N]; K];
    for row in 0..K {
        for column in 0..N {
            let mut value = 0.0;
            for inner in row..K {
                value += codomain.cholesky[inner][row] * right[inner][column];
            }
            if !value.is_finite() {
                return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
            }
            result[row][column] = value;
        }
    }
    Ok(result)
}

fn whiten_tangent_to_cotangent<
    Domain: CoordinateFrame,
    Codomain: CoordinateFrame,
    const K: usize,
    const N: usize,
>(
    operator: &[[f64; N]; K],
    domain: &Metric<Domain, N>,
    codomain: &Metric<Codomain, K>,
) -> Result<[[f64; N]; K], OperatorConditionErrorV1> {
    // C = L_Y^-1 A L_X^-T. Solve L_Y C = A L_X^-T.
    let right = right_whiten_domain(operator, &domain.cholesky)?;
    let mut result = [[0.0; N]; K];
    for column in 0..N {
        for row in 0..K {
            let mut value = right[row][column];
            for inner in 0..row {
                value -= codomain.cholesky[row][inner] * result[inner][column];
            }
            result[row][column] = value / codomain.cholesky[row][row];
            if !result[row][column].is_finite() {
                return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
            }
        }
    }
    Ok(result)
}

fn column_norm<const K: usize, const N: usize>(matrix: &[[f64; N]; K], column: usize) -> f64 {
    matrix
        .iter()
        .fold(0.0_f64, |norm, row| norm.hypot(row[column]))
}

fn normalized_column_correlation<const K: usize, const N: usize>(
    matrix: &[[f64; N]; K],
    left: usize,
    right: usize,
) -> Result<f64, OperatorConditionErrorV1> {
    let left_norm = column_norm(matrix, left);
    let right_norm = column_norm(matrix, right);
    if !left_norm.is_finite() || !right_norm.is_finite() {
        return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
    }
    if left_norm == 0.0 || right_norm == 0.0 {
        return Ok(0.0);
    }
    let mut correlation = 0.0;
    for row in matrix.iter().take(K) {
        correlation += (row[left] / left_norm) * (row[right] / right_norm);
    }
    if correlation.is_finite() {
        Ok(correlation)
    } else {
        Err(OperatorConditionErrorV1::NonFiniteIntermediate)
    }
}

/// One-sided Jacobi SVD for a K-by-N matrix. The working array has N columns;
/// the returned thin singular spectrum contains min(K,N) values. For a wide
/// full-row-rank map, the extra N-K domain-null directions are reported through
/// the explicit domain_nullity field rather than padding the thin spectrum.
fn singular_values_one_sided_jacobi<const K: usize, const N: usize>(
    matrix: &[[f64; N]; K],
    profile: OperatorConditionProfileV1,
) -> Result<(Vec<f64>, f64, usize), OperatorConditionErrorV1> {
    let mut work = *matrix;
    let initial_scale = (0..N).fold(0.0_f64, |maximum, column| {
        maximum.max(column_norm(&work, column))
    });
    if !initial_scale.is_finite() {
        return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
    }
    // Columns below the already-declared numerical-rank threshold represent
    // unresolved null directions. Their normalized correlation is dominated
    // by roundoff and must not prevent convergence of the resolved range.
    let numerical_zero_floor = initial_scale * profile.relative_rank_tolerance;
    if !numerical_zero_floor.is_finite() {
        return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
    }
    let mut sweeps = 0;
    let mut converged = false;

    for sweep in 0..profile.maximum_jacobi_sweeps {
        let mut rotated_any_pair = false;
        for left in 0..N {
            for right in (left + 1)..N {
                let left_norm = column_norm(&work, left);
                let right_norm = column_norm(&work, right);
                if !left_norm.is_finite() || !right_norm.is_finite() {
                    return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
                }
                if left_norm <= numerical_zero_floor || right_norm <= numerical_zero_floor {
                    continue;
                }

                let correlation = normalized_column_correlation(&work, left, right)?;
                if correlation.abs() <= profile.relative_jacobi_tolerance {
                    continue;
                }

                // Scale the 2x2 Gram block before forming the rotation.
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

                for row in work.iter_mut().take(K) {
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
    for left in 0..N {
        for right in (left + 1)..N {
            if column_norm(&work, left) <= numerical_zero_floor
                || column_norm(&work, right) <= numerical_zero_floor
            {
                continue;
            }
            maximum_correlation =
                maximum_correlation.max(normalized_column_correlation(&work, left, right)?.abs());
        }
    }
    if !converged && maximum_correlation > profile.relative_jacobi_tolerance {
        return Err(OperatorConditionErrorV1::JacobiDidNotConverge);
    }

    let mut singular_values: Vec<f64> = (0..N).map(|column| column_norm(&work, column)).collect();
    if singular_values.iter().any(|value| !value.is_finite()) {
        return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
    }
    singular_values.sort_by(|left, right| right.total_cmp(left));

    // Return the conventional thin spectrum after the full N-column
    // orthogonalization has exposed any additional domain-null directions.
    singular_values.truncate(K.min(N));
    Ok((singular_values, maximum_correlation, sweeps))
}

fn assess_whitened<const K: usize, const N: usize>(
    whitened: [[f64; N]; K],
    profile: OperatorConditionProfileV1,
) -> Result<RectangularOperatorConditionReceiptV1, OperatorConditionErrorV1> {
    validate_rectangular_profile(profile)?;
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
    let domain_nullity = N.saturating_sub(numerical_rank);
    let codomain_nullity = K.saturating_sub(numerical_rank);
    let smallest_nonzero = singular_values
        .iter()
        .copied()
        .filter(|value| *value > rank_threshold)
        .last();

    let nonzero_spectrum_condition_estimate = match (smallest_nonzero, largest) {
        (Some(minimum), maximum) if maximum > 0.0 => {
            let condition = maximum / minimum;
            if !condition.is_finite() {
                return Err(OperatorConditionErrorV1::NonFiniteIntermediate);
            }
            Some(condition)
        }
        _ => None,
    };
    let condition_estimate = if N <= K && numerical_rank == N {
        nonzero_spectrum_condition_estimate
    } else {
        None
    };

    let minimum_dimension = K.min(N);
    let status = if numerical_rank < minimum_dimension {
        RectangularOperatorStatusV1::RankDeficient
    } else if nonzero_spectrum_condition_estimate
        .is_some_and(|condition| condition > profile.maximum_condition_estimate)
    {
        RectangularOperatorStatusV1::OutsideConditionEnvelope
    } else if N > K {
        RectangularOperatorStatusV1::NonInjective
    } else {
        RectangularOperatorStatusV1::Admitted
    };

    Ok(RectangularOperatorConditionReceiptV1 {
        singular_values,
        largest_singular_value: largest,
        smallest_singular_value: smallest,
        smallest_nonzero_singular_value: smallest_nonzero,
        numerical_rank,
        domain_dimension: N,
        codomain_dimension: K,
        domain_nullity,
        codomain_nullity,
        condition_estimate,
        nonzero_spectrum_condition_estimate,
        maximum_normalized_column_correlation: maximum_correlation,
        jacobi_sweeps,
        status,
    })
}

impl<Domain: CoordinateFrame, Codomain: CoordinateFrame, const K: usize, const N: usize>
    RectangularTangentToTangentOperator<Domain, Codomain, K, N>
{
    pub fn new(matrix: [[f64; N]; K]) -> Result<Self, OperatorConditionErrorV1> {
        validate_rectangular_operator(&matrix)?;
        Ok(Self {
            matrix,
            _frames: PhantomData,
        })
    }

    pub fn matrix(&self) -> &[[f64; N]; K] {
        &self.matrix
    }

    pub fn assess(
        &self,
        domain_metric: &Metric<Domain, N>,
        codomain_metric: &Metric<Codomain, K>,
        profile: OperatorConditionProfileV1,
    ) -> Result<RectangularOperatorConditionReceiptV1, OperatorConditionErrorV1> {
        let whitened = whiten_tangent_to_tangent(&self.matrix, domain_metric, codomain_metric)?;
        assess_whitened(whitened, profile)
    }
}

impl<Domain: CoordinateFrame, Codomain: CoordinateFrame, const K: usize, const N: usize>
    RectangularTangentToCotangentOperator<Domain, Codomain, K, N>
{
    pub fn new(matrix: [[f64; N]; K]) -> Result<Self, OperatorConditionErrorV1> {
        validate_rectangular_operator(&matrix)?;
        Ok(Self {
            matrix,
            _frames: PhantomData,
        })
    }

    pub fn matrix(&self) -> &[[f64; N]; K] {
        &self.matrix
    }

    pub fn assess(
        &self,
        domain_metric: &Metric<Domain, N>,
        codomain_metric: &Metric<Codomain, K>,
        profile: OperatorConditionProfileV1,
    ) -> Result<RectangularOperatorConditionReceiptV1, OperatorConditionErrorV1> {
        let whitened = whiten_tangent_to_cotangent(&self.matrix, domain_metric, codomain_metric)?;
        assess_whitened(whitened, profile)
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
            "left={left:.16e}, right={right:.16e}"
        );
    }

    fn multiply<const R: usize, const M: usize, const C: usize>(
        left: &[[f64; M]; R],
        right: &[[f64; C]; M],
    ) -> [[f64; C]; R] {
        let mut result = [[0.0; C]; R];
        for row in 0..R {
            for column in 0..C {
                for inner in 0..M {
                    result[row][column] += left[row][inner] * right[inner][column];
                }
            }
        }
        result
    }

    #[test]
    fn full_row_rank_wide_map_reports_domain_nullity_without_fake_condition() {
        let domain = Metric::<CartesianQ, 3>::admit(
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            metric_profile(),
        )
        .unwrap();
        let codomain =
            Metric::<DenseY, 2>::admit([[1.0, 0.0], [0.0, 1.0]], metric_profile()).unwrap();
        let operator = RectangularTangentToTangentOperator::<CartesianQ, DenseY, 2, 3>::new([
            [1.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
        ])
        .unwrap();

        let receipt = operator
            .assess(&domain, &codomain, operator_profile())
            .unwrap();
        assert_eq!(receipt.numerical_rank, 2);
        assert_eq!(receipt.domain_nullity, 1);
        assert_eq!(receipt.codomain_nullity, 0);
        assert_eq!(receipt.condition_estimate, None);
        close(
            receipt.nonzero_spectrum_condition_estimate.unwrap(),
            2.0,
            1.0e-12,
        );
        assert_eq!(receipt.status, RectangularOperatorStatusV1::NonInjective);
    }

    #[test]
    fn rank_deficient_rectangular_map_never_receives_a_whole_map_condition() {
        let domain = Metric::<CartesianQ, 3>::admit(
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            metric_profile(),
        )
        .unwrap();
        let codomain =
            Metric::<DenseY, 2>::admit([[1.0, 0.0], [0.0, 1.0]], metric_profile()).unwrap();
        let operator = RectangularTangentToTangentOperator::<CartesianQ, DenseY, 2, 3>::new([
            [1.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
        ])
        .unwrap();

        let receipt = operator
            .assess(&domain, &codomain, operator_profile())
            .unwrap();
        assert_eq!(receipt.numerical_rank, 1);
        assert_eq!(receipt.domain_nullity, 2);
        assert_eq!(receipt.codomain_nullity, 1);
        assert_eq!(receipt.condition_estimate, None);
        assert_eq!(receipt.status, RectangularOperatorStatusV1::RankDeficient);
    }

    #[test]
    fn tall_full_column_rank_map_has_a_finite_condition_estimate() {
        let domain =
            Metric::<CartesianQ, 2>::admit([[1.0, 0.0], [0.0, 1.0]], metric_profile()).unwrap();
        let codomain = Metric::<DenseY, 3>::admit(
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            metric_profile(),
        )
        .unwrap();
        let operator = RectangularTangentToCotangentOperator::<CartesianQ, DenseY, 3, 2>::new([
            [2.0, 0.0],
            [0.0, 3.0],
            [0.0, 0.0],
        ])
        .unwrap();

        let receipt = operator
            .assess(&domain, &codomain, operator_profile())
            .unwrap();
        assert_eq!(receipt.numerical_rank, 2);
        assert_eq!(receipt.domain_nullity, 0);
        assert_eq!(receipt.codomain_nullity, 1);
        close(receipt.condition_estimate.unwrap(), 1.5, 1.0e-12);
        assert_eq!(receipt.status, RectangularOperatorStatusV1::Admitted);
    }

    #[test]
    fn rectangular_singular_spectrum_is_invariant_under_dense_rebasing() {
        let source_profile = metric_profile();
        let source_operator_profile = operator_profile();
        let domain_q = Metric::<CartesianQ, 3>::admit(
            [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
            source_profile,
        )
        .unwrap();
        let codomain_q =
            Metric::<CartesianQ, 2>::admit([[4.0, 0.0], [0.0, 2.0]], source_profile).unwrap();
        let sx = Rebasis::<CartesianQ, DenseY, 3>::new(
            [[1.0, 2.0, 0.0], [0.0, 1.0, 1.0], [0.0, 0.0, 1.0]],
            source_profile,
        )
        .unwrap();
        let sy = Rebasis::<CartesianQ, DenseY, 2>::new([[1.0, 2.0], [0.0, 1.0]], source_profile)
            .unwrap();
        let domain_y = sx.metric_to_to(&domain_q, source_profile).unwrap();
        let codomain_y = sy.metric_to_to(&codomain_q, source_profile).unwrap();
        let aq = [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0]];
        let sx_matrix = *sx.matrix();
        let sy_inverse = [[1.0, -2.0], [0.0, 1.0]];
        let sy_transpose = [[1.0, 0.0], [2.0, 1.0]];
        let aty_q =
            RectangularTangentToTangentOperator::<CartesianQ, CartesianQ, 2, 3>::new(aq).unwrap();
        let aty_y_matrix = multiply(&multiply(&sy_inverse, &aq), &sx_matrix);
        let aty_y =
            RectangularTangentToTangentOperator::<DenseY, DenseY, 2, 3>::new(aty_y_matrix).unwrap();

        let atc_q =
            RectangularTangentToCotangentOperator::<CartesianQ, CartesianQ, 2, 3>::new(aq).unwrap();
        let atc_y_matrix = multiply(&multiply(&sy_transpose, &aq), &sx_matrix);
        let atc_y =
            RectangularTangentToCotangentOperator::<DenseY, DenseY, 2, 3>::new(atc_y_matrix)
                .unwrap();

        let aty_q_receipt = aty_q
            .assess(&domain_q, &codomain_q, source_operator_profile)
            .unwrap();
        let aty_y_receipt = aty_y
            .assess(&domain_y, &codomain_y, source_operator_profile)
            .unwrap();
        let atc_q_receipt = atc_q
            .assess(&domain_q, &codomain_q, source_operator_profile)
            .unwrap();
        let atc_y_receipt = atc_y
            .assess(&domain_y, &codomain_y, source_operator_profile)
            .unwrap();

        assert_eq!(aty_q_receipt.numerical_rank, aty_y_receipt.numerical_rank);
        assert_eq!(atc_q_receipt.numerical_rank, atc_y_receipt.numerical_rank);
        assert_eq!(aty_q_receipt.singular_values.len(), 2);
        assert_eq!(atc_q_receipt.singular_values.len(), 2);
        for index in 0..2 {
            close(
                aty_q_receipt.singular_values[index],
                aty_y_receipt.singular_values[index],
                5.0e-11,
            );
            close(
                atc_q_receipt.singular_values[index],
                atc_y_receipt.singular_values[index],
                5.0e-11,
            );
        }
        close(
            aty_q_receipt.nonzero_spectrum_condition_estimate.unwrap(),
            aty_y_receipt.nonzero_spectrum_condition_estimate.unwrap(),
            5.0e-11,
        );
        close(
            atc_q_receipt.nonzero_spectrum_condition_estimate.unwrap(),
            atc_y_receipt.nonzero_spectrum_condition_estimate.unwrap(),
            5.0e-11,
        );
        assert_eq!(aty_q_receipt.condition_estimate, None);
        assert_eq!(atc_q_receipt.condition_estimate, None);
    }

    #[test]
    fn wrong_tangent_operator_variance_is_detected() {
        let profile = metric_profile();
        let operator_profile = operator_profile();
        let domain = Metric::<CartesianQ, 3>::admit(
            [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
            profile,
        )
        .unwrap();
        let codomain = Metric::<CartesianQ, 2>::admit([[4.0, 0.0], [0.0, 2.0]], profile).unwrap();
        let sx = Rebasis::<CartesianQ, DenseY, 3>::new(
            [[1.0, 2.0, 0.0], [0.0, 1.0, 1.0], [0.0, 0.0, 1.0]],
            profile,
        )
        .unwrap();
        let sy = Rebasis::<CartesianQ, DenseY, 2>::new([[1.0, 2.0], [0.0, 1.0]], profile).unwrap();
        let domain_y = sx.metric_to_to(&domain, profile).unwrap();
        let codomain_y = sy.metric_to_to(&codomain, profile).unwrap();
        let aq = [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0]];
        let wrong_left = [[1.0, 0.0], [2.0, 1.0]];
        let wrong_matrix = multiply(&multiply(&wrong_left, &aq), sx.matrix());
        let wrong =
            RectangularTangentToTangentOperator::<DenseY, DenseY, 2, 3>::new(wrong_matrix).unwrap();
        let correct = RectangularTangentToTangentOperator::<DenseY, DenseY, 2, 3>::new(multiply(
            &multiply(&[[1.0, -2.0], [0.0, 1.0]], &aq),
            sx.matrix(),
        ))
        .unwrap();
        let wrong_receipt = wrong
            .assess(&domain_y, &codomain_y, operator_profile)
            .unwrap();
        let correct_receipt = correct
            .assess(&domain_y, &codomain_y, operator_profile)
            .unwrap();
        assert!(
            (wrong_receipt.nonzero_spectrum_condition_estimate.unwrap()
                - correct_receipt.nonzero_spectrum_condition_estimate.unwrap())
            .abs()
                > 1.0e-3
        );
    }

    #[test]
    fn nonfinite_and_zero_dimension_inputs_fail_closed() {
        assert!(matches!(
            RectangularTangentToTangentOperator::<CartesianQ, DenseY, 2, 3>::new([
                [1.0, f64::NAN, 0.0],
                [0.0, 1.0, 0.0],
            ]),
            Err(OperatorConditionErrorV1::NonFiniteOperator)
        ));
        assert_eq!(
            RectangularTangentToTangentOperator::<CartesianQ, DenseY, 2, 0>::new([[], []]),
            Err(OperatorConditionErrorV1::ZeroDimension)
        );
    }
}
