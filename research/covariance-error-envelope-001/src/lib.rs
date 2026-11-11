// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! A-posteriori numerical error estimates for small dense coordinate systems.
//!
//! The reported condition number and forward-error expression are estimates,
//! not formally certified interval bounds. Independent high-precision
//! calibration is a separate obligation.

#![allow(clippy::needless_range_loop)]

use std::fmt;

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
pub struct ErrorEnvelopeProfileV1 {
    pub relative_pivot_tolerance: f64,
    pub maximum_condition_inf: f64,
    pub maximum_condition_times_backward_error: f64,
}

impl Default for ErrorEnvelopeProfileV1 {
    fn default() -> Self {
        Self {
            relative_pivot_tolerance: 64.0 * f64::EPSILON,
            maximum_condition_inf: 1.0e8,
            maximum_condition_times_backward_error: 1.0e-2,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SolveErrorEnvelopeV1 {
    pub residual_inf: f64,
    pub matrix_inf: f64,
    pub solution_inf: f64,
    pub rhs_inf: f64,
    pub normwise_backward_error: f64,
    pub componentwise_backward_error: f64,
    pub condition_inf_estimate: f64,
    pub condition_times_backward_error: f64,
    pub forward_error_bound_estimate: Option<f64>,
    pub status: EnvelopeStatusV1,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnvelopeErrorV1 {
    InvalidProfile,
    ZeroDimension,
    NonFiniteMatrix,
    NonFiniteVector,
    SingularPivot {
        column: usize,
        magnitude: f64,
        threshold: f64,
    },
    ArithmeticOverflow,
    DegenerateResidualScale,
}

impl fmt::Display for EnvelopeErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for EnvelopeErrorV1 {}

#[derive(Debug, Clone)]
struct LuFactorV1<const N: usize> {
    lu: Matrix<N>,
    permutation: [usize; N],
}

fn validate_profile(profile: ErrorEnvelopeProfileV1) -> Result<(), EnvelopeErrorV1> {
    if !profile.relative_pivot_tolerance.is_finite()
        || profile.relative_pivot_tolerance <= 0.0
        || !profile.maximum_condition_inf.is_finite()
        || profile.maximum_condition_inf < 1.0
        || !profile
            .maximum_condition_times_backward_error
            .is_finite()
        || profile.maximum_condition_times_backward_error <= 0.0
        || profile.maximum_condition_times_backward_error >= 1.0
    {
        return Err(EnvelopeErrorV1::InvalidProfile);
    }
    Ok(())
}

pub fn vector_inf<const N: usize>(vector: &Vector<N>) -> f64 {
    vector.iter().map(|value| value.abs()).fold(0.0, f64::max)
}

pub fn matrix_inf<const N: usize>(matrix: &Matrix<N>) -> f64 {
    let mut maximum = 0.0_f64;
    for row in 0..N {
        let sum = matrix[row].iter().map(|value| value.abs()).sum::<f64>();
        maximum = maximum.max(sum);
    }
    maximum
}

pub fn transpose<const N: usize>(matrix: &Matrix<N>) -> Matrix<N> {
    let mut result = [[0.0; N]; N];
    for row in 0..N {
        for column in 0..N {
            result[row][column] = matrix[column][row];
        }
    }
    result
}

pub fn multiply<const N: usize>(left: &Matrix<N>, right: &Matrix<N>) -> Matrix<N> {
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

pub fn apply<const N: usize>(matrix: &Matrix<N>, vector: &Vector<N>) -> Vector<N> {
    let mut result = [0.0; N];
    for row in 0..N {
        for column in 0..N {
            result[row] += matrix[row][column] * vector[column];
        }
    }
    result
}

/// (M_y=S^T M_q S), the mass-metric congruence for q = S y.
pub fn transform_metric<const N: usize>(
    mass_q: &Matrix<N>,
    basis: &Matrix<N>,
) -> Matrix<N> {
    multiply(&multiply(&transpose(basis), mass_q), basis)
}

/// (Q_y=S^T Q_q), the covector transformation for q = S y.
pub fn transform_covector<const N: usize>(
    basis: &Matrix<N>,
    force_q: &Vector<N>,
) -> Vector<N> {
    apply(&transpose(basis), force_q)
}

fn factor<const N: usize>(
    matrix: &Matrix<N>,
    profile: ErrorEnvelopeProfileV1,
) -> Result<LuFactorV1<N>, EnvelopeErrorV1> {
    if N == 0 {
        return Err(EnvelopeErrorV1::ZeroDimension);
    }
    for row in matrix {
        for value in row {
            if !value.is_finite() {
                return Err(EnvelopeErrorV1::NonFiniteMatrix);
            }
        }
    }

    let matrix_scale = matrix_inf(matrix);
    if !matrix_scale.is_finite() {
        return Err(EnvelopeErrorV1::ArithmeticOverflow);
    }
    if matrix_scale == 0.0 {
        return Err(EnvelopeErrorV1::SingularPivot {
            column: 0,
            magnitude: 0.0,
            threshold: 0.0,
        });
    }
    let threshold = profile.relative_pivot_tolerance * matrix_scale;
    if !threshold.is_finite() {
        return Err(EnvelopeErrorV1::ArithmeticOverflow);
    }
    let mut lu = *matrix;
    let mut permutation = std::array::from_fn(|index| index);

    for column in 0..N {
        let pivot_row = (column..N)
            .max_by(|left, right| {
                lu[*left][column]
                    .abs()
                    .total_cmp(&lu[*right][column].abs())
            })
            .ok_or(EnvelopeErrorV1::ZeroDimension)?;
        let pivot_magnitude = lu[pivot_row][column].abs();
        if !pivot_magnitude.is_finite() || pivot_magnitude <= threshold {
            return Err(EnvelopeErrorV1::SingularPivot {
                column,
                magnitude: pivot_magnitude,
                threshold,
            });
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
        return Err(EnvelopeErrorV1::ArithmeticOverflow);
    }

    Ok(LuFactorV1 { lu, permutation })
}

impl<const N: usize> LuFactorV1<N> {
    fn solve(&self, rhs: &Vector<N>) -> Result<Vector<N>, EnvelopeErrorV1> {
        if rhs.iter().any(|value| !value.is_finite()) {
            return Err(EnvelopeErrorV1::NonFiniteVector);
        }
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
            return Err(EnvelopeErrorV1::ArithmeticOverflow);
        }
        Ok(solution)
    }

    fn condition_inf(
        &self,
        original: &Matrix<N>,
    ) -> Result<f64, EnvelopeErrorV1> {
        let mut inverse = [[0.0; N]; N];
        for column in 0..N {
            let mut unit = [0.0; N];
            unit[column] = 1.0;
            let inverse_column = self.solve(&unit)?;
            for row in 0..N {
                inverse[row][column] = inverse_column[row];
            }
        }
        let original_norm = matrix_inf(original);
        let inverse_norm = matrix_inf(&inverse);
        let estimate = original_norm * inverse_norm;
        if !original_norm.is_finite() || !inverse_norm.is_finite() || !estimate.is_finite() {
            return Err(EnvelopeErrorV1::ArithmeticOverflow);
        }
        Ok(estimate)
    }
}

/// Independently assess a computed x against the original system A x = b.
///
/// The residual is evaluated from the original, unfactored matrix. The
/// condition estimate is produced by pivoted solves against unit vectors.
pub fn assess_solution<const N: usize>(
    matrix: &Matrix<N>,
    rhs: &Vector<N>,
    computed: &Vector<N>,
    profile: ErrorEnvelopeProfileV1,
) -> Result<SolveErrorEnvelopeV1, EnvelopeErrorV1> {
    validate_profile(profile)?;
    let lu = factor(matrix, profile)?;
    if rhs.iter().any(|value| !value.is_finite())
        || computed.iter().any(|value| !value.is_finite())
    {
        return Err(EnvelopeErrorV1::NonFiniteVector);
    }

    let product = apply(matrix, computed);
    let residual: Vector<N> =
        std::array::from_fn(|index| rhs[index] - product[index]);
    let residual_inf = vector_inf(&residual);
    let matrix_norm = matrix_inf(matrix);
    let solution_norm = vector_inf(computed);
    let rhs_norm = vector_inf(rhs);
    let normwise_scale = matrix_norm * solution_norm + rhs_norm;
    if product.iter().any(|value| !value.is_finite())
        || residual.iter().any(|value| !value.is_finite())
        || !residual_inf.is_finite()
        || !matrix_norm.is_finite()
        || !solution_norm.is_finite()
        || !rhs_norm.is_finite()
        || !normwise_scale.is_finite()
    {
        return Err(EnvelopeErrorV1::ArithmeticOverflow);
    }
    let normwise_backward_error = if normwise_scale == 0.0 {
        if residual_inf == 0.0 {
            0.0
        } else {
            return Err(EnvelopeErrorV1::DegenerateResidualScale);
        }
    } else {
        residual_inf / normwise_scale
    };

    let mut componentwise_backward_error = 0.0_f64;
    for row in 0..N {
        let denominator = rhs[row].abs()
            + (0..N)
                .map(|column| matrix[row][column].abs() * computed[column].abs())
                .sum::<f64>();
        let numerator = residual[row].abs();
        if !denominator.is_finite() || !numerator.is_finite() {
            return Err(EnvelopeErrorV1::ArithmeticOverflow);
        }
        let componentwise = if denominator == 0.0 {
            if numerator == 0.0 {
                0.0
            } else {
                return Err(EnvelopeErrorV1::DegenerateResidualScale);
            }
        } else {
            numerator / denominator
        };
        componentwise_backward_error = componentwise_backward_error.max(componentwise);
    }

    let condition_inf_estimate = lu.condition_inf(matrix)?;
    let condition_times_backward_error =
        condition_inf_estimate * normwise_backward_error;
    let forward_error_bound_estimate = if condition_times_backward_error < 1.0 {
        Some(
            condition_times_backward_error
                / (1.0 - condition_times_backward_error),
        )
    } else {
        None
    };

    let status = if condition_inf_estimate > profile.maximum_condition_inf {
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

    Ok(SolveErrorEnvelopeV1 {
        residual_inf,
        matrix_inf: matrix_norm,
        solution_inf: solution_norm,
        rhs_inf: rhs_norm,
        normwise_backward_error,
        componentwise_backward_error,
        condition_inf_estimate,
        condition_times_backward_error,
        forward_error_bound_estimate,
        status,
    })
}

/// Solve A x = b with partial pivoting, then independently assess the result.
pub fn solve_and_assess<const N: usize>(
    matrix: &Matrix<N>,
    rhs: &Vector<N>,
    profile: ErrorEnvelopeProfileV1,
) -> Result<(Vector<N>, SolveErrorEnvelopeV1), EnvelopeErrorV1> {
    validate_profile(profile)?;
    let lu = factor(matrix, profile)?;
    let solution = lu.solve(rhs)?;
    let envelope = assess_solution(matrix, rhs, &solution, profile)?;
    Ok((solution, envelope))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exact_two_by_two() -> (Matrix<2>, Vector<2>, Vector<2>) {
        ([[4.0, 1.0], [2.0, 3.0]], [1.0, 2.0], [0.1, 0.6])
    }

    #[test]
    fn known_rational_solution_has_condition_estimate_three() {
        let (matrix, rhs, exact) = exact_two_by_two();
        let profile = ErrorEnvelopeProfileV1::default();
        let (computed, envelope) = solve_and_assess(&matrix, &rhs, profile).unwrap();

        assert!((computed[0] - exact[0]).abs() < 1.0e-14);
        assert!((computed[1] - exact[1]).abs() < 1.0e-14);
        assert!((envelope.condition_inf_estimate - 3.0).abs() < 1.0e-14);
        assert_eq!(envelope.status, EnvelopeStatusV1::InformativeEstimate);

        let bound = envelope.forward_error_bound_estimate.unwrap();
        let observed =
            vector_inf(&[computed[0] - exact[0], computed[1] - exact[1]]) / vector_inf(&exact);
        assert!(observed <= bound + 1.0e-14);
    }

    #[test]
    fn deliberately_perturbed_solution_is_covered_by_forward_estimate() {
        let (matrix, rhs, exact) = exact_two_by_two();
        let computed = [exact[0] + 1.0e-3, exact[1]];
        let envelope =
            assess_solution(&matrix, &rhs, &computed, ErrorEnvelopeProfileV1::default()).unwrap();

        let observed = vector_inf(&[
            computed[0] - exact[0],
            computed[1] - exact[1],
        ]) / vector_inf(&exact);
        let bound = envelope.forward_error_bound_estimate.unwrap();
        assert!(observed <= bound);
        assert!(envelope.componentwise_backward_error > envelope.normwise_backward_error);
    }

    #[test]
    fn condition_ladder_abstains_beyond_declared_envelope() {
        let matrix = [[1.0, 0.0], [0.0, 1.0e-9]];
        let rhs = [1.0, 1.0];
        let exact_solution = [1.0, 1.0e9];
        let profile = ErrorEnvelopeProfileV1 {
            maximum_condition_inf: 1.0e8,
            ..ErrorEnvelopeProfileV1::default()
        };
        let envelope = assess_solution(&matrix, &rhs, &exact_solution, profile).unwrap();

        assert!((envelope.condition_inf_estimate - 1.0e9).abs() < 1.0);
        assert_eq!(envelope.status, EnvelopeStatusV1::OutsideConditionEnvelope);
    }

    #[test]
    fn weak_forward_estimate_is_not_admitted_as_informative() {
        let (matrix, rhs, _exact) = exact_two_by_two();
        let perturbed = [0.2, 0.6];
        let profile = ErrorEnvelopeProfileV1 {
            maximum_condition_times_backward_error: 0.1,
            ..ErrorEnvelopeProfileV1::default()
        };
        let envelope = assess_solution(&matrix, &rhs, &perturbed, profile).unwrap();

        assert_eq!(envelope.status, EnvelopeStatusV1::WeakEstimate);
        assert!(envelope.forward_error_bound_estimate.is_some());
    }

    #[test]
    fn vacuous_forward_bound_is_reported_explicitly() {
        let (matrix, rhs, _exact) = exact_two_by_two();
        let computed = [100.0, -100.0];
        let envelope =
            assess_solution(&matrix, &rhs, &computed, ErrorEnvelopeProfileV1::default()).unwrap();

        assert_eq!(envelope.status, EnvelopeStatusV1::VacuousEstimate);
        assert!(envelope.forward_error_bound_estimate.is_none());
    }

    #[test]
    fn singular_and_nonfinite_operators_fail_closed() {
        let singular = [[1.0, 2.0], [2.0, 4.0]];
        assert!(matches!(
            assess_solution(
                &singular,
                &[1.0, 2.0],
                &[1.0, 0.0],
                ErrorEnvelopeProfileV1::default()
            ),
            Err(EnvelopeErrorV1::SingularPivot { .. })
        ));

        let nonfinite = [[f64::NAN, 0.0], [0.0, 1.0]];
        assert_eq!(
            assess_solution(
                &nonfinite,
                &[1.0, 1.0],
                &[1.0, 1.0],
                ErrorEnvelopeProfileV1::default()
            ),
            Err(EnvelopeErrorV1::NonFiniteMatrix)
        );
    }

    #[test]
    fn finite_inputs_whose_matrix_norm_overflows_are_rejected() {
        let matrix = [[f64::MAX, f64::MAX], [0.0, 1.0]];
        assert_eq!(
            assess_solution(
                &matrix,
                &[1.0, 1.0],
                &[0.0, 0.0],
                ErrorEnvelopeProfileV1::default()
            ),
            Err(EnvelopeErrorV1::ArithmeticOverflow)
        );
    }

    #[test]
    fn finite_inputs_whose_residual_overflows_are_rejected() {
        let matrix = [[1.0e200, 0.0], [0.0, 1.0]];
        assert_eq!(
            assess_solution(
                &matrix,
                &[1.0, 1.0],
                &[1.0e200, 0.0],
                ErrorEnvelopeProfileV1::default()
            ),
            Err(EnvelopeErrorV1::ArithmeticOverflow)
        );
    }

    #[test]
    fn normalized_backward_error_is_invariant_under_common_scaling() {
        let (matrix, rhs, exact) = exact_two_by_two();
        let perturbed = [exact[0] + 1.0e-4, exact[1]];
        let first =
            assess_solution(&matrix, &rhs, &perturbed, ErrorEnvelopeProfileV1::default()).unwrap();

        let scale = 1.0e6;
        let scaled_matrix = matrix.map(|row| row.map(|value| value * scale));
        let scaled_rhs = rhs.map(|value| value * scale);
        let second = assess_solution(
            &scaled_matrix,
            &scaled_rhs,
            &perturbed,
            ErrorEnvelopeProfileV1::default(),
        )
        .unwrap();

        assert!((first.normwise_backward_error - second.normwise_backward_error).abs() < 1.0e-15);
        assert!((first.condition_inf_estimate - second.condition_inf_estimate).abs() < 1.0e-12);
    }

    #[test]
    fn dense_coordinate_metric_and_covector_transform_preserve_pairing() {
        let basis = [[1.0, 2.0], [0.0, 1.0]];
        let mass_q = [[2.0, 0.0], [0.0, 3.0]];
        let force_q = [5.0, 7.0];
        let y = [1.0, 2.0];
        let q = apply(&basis, &y);
        let mass_y = transform_metric(&mass_q, &basis);
        let force_y = transform_covector(&basis, &force_q);

        assert_eq!(mass_y, [[2.0, 4.0], [4.0, 11.0]]);
        let physical_pairing = force_q[0] * q[0] + force_q[1] * q[1];
        let transformed_pairing = force_y[0] * y[0] + force_y[1] * y[1];
        assert_eq!(physical_pairing, transformed_pairing);
    }

    #[test]
    fn wrong_metric_congruence_is_detected() {
        let basis = [[1.0, 2.0], [0.0, 1.0]];
        let mass_q = [[2.0, 0.0], [0.0, 3.0]];
        let correct = transform_metric(&mass_q, &basis);
        let wrong = multiply(&multiply(&transpose(&basis), &mass_q), &transpose(&basis));
        assert_ne!(correct, wrong);
    }
}
