//! Globally solved maximization of a positive-semidefinite quadratic over a
//! Euclidean trust region.
//!
//! The solver uses the secular equation and explicitly handles the hard case
//! where the linear term is orthogonal to the top eigenspace. It is intended as
//! a small, auditable exact reference, not a large-scale optimizer.

#![allow(clippy::needless_range_loop)]

use crate::network::{symmetric_eigen, NetworkError};
use std::fmt;

const SYMMETRY_TOLERANCE: f64 = 1.0e-11;
const EIGEN_TOLERANCE: f64 = 1.0e-10;
const RESIDUAL_TOLERANCE: f64 = 1.0e-8;
const MAX_ROOT_ITERATIONS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustRegionDisposition {
    RegularBoundaryOptimum,
    HardCaseBoundaryOptimum,
    FlatObjective,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrustRegionSolutionV1 {
    pub point: Vec<f64>,
    pub multiplier: f64,
    pub largest_eigenvalue: f64,
    pub objective_without_constant: f64,
    pub point_norm: f64,
    pub radius: f64,
    pub stationarity_residual: f64,
    pub feasibility_residual: f64,
    pub secular_residual: f64,
    pub iterations: usize,
    pub disposition: TrustRegionDisposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustRegionError {
    EmptyMatrix,
    DimensionMismatch,
    NonSquareMatrix,
    NonFiniteInput,
    AsymmetricMatrix,
    IndefiniteMatrix,
    InvalidRadius,
    EigensolverFailure,
    RootBracketFailure,
    KktResidualTooLarge,
}

impl fmt::Display for TrustRegionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyMatrix => write!(f, "trust-region matrix must be non-empty"),
            Self::DimensionMismatch => write!(f, "trust-region matrix and linear term dimensions differ"),
            Self::NonSquareMatrix => write!(f, "trust-region matrix must be square"),
            Self::NonFiniteInput => write!(f, "trust-region input contains a non-finite value"),
            Self::AsymmetricMatrix => write!(f, "trust-region matrix is not symmetric within tolerance"),
            Self::IndefiniteMatrix => write!(f, "trust-region matrix is not positive semidefinite"),
            Self::InvalidRadius => write!(f, "trust-region radius must be finite and non-negative"),
            Self::EigensolverFailure => write!(f, "trust-region eigendecomposition failed"),
            Self::RootBracketFailure => write!(f, "could not bracket the trust-region secular root"),
            Self::KktResidualTooLarge => write!(f, "trust-region solution failed its KKT residual checks"),
        }
    }
}

impl std::error::Error for TrustRegionError {}

/// Globally maximize 0.5 * x^T A x + g^T x subject to ||x|| <= radius.
///
/// The returned multiplier satisfies (lambda I - A) x = g and
/// lambda >= lambda_max(A). The hard case is handled explicitly when g has
/// no material component in the top eigenspace and the lower-eigenspace
/// solution leaves unused radius.
pub fn maximize_psd_quadratic_on_ball(
    matrix: &[Vec<f64>],
    linear: &[f64],
    radius: f64,
) -> Result<TrustRegionSolutionV1, TrustRegionError> {
    let n = matrix.len();
    if n == 0 {
        return Err(TrustRegionError::EmptyMatrix);
    }
    if linear.len() != n {
        return Err(TrustRegionError::DimensionMismatch);
    }
    if matrix.iter().any(|row| row.len() != n) {
        return Err(TrustRegionError::NonSquareMatrix);
    }
    if !radius.is_finite() || radius <= 0.0 {
        return Err(TrustRegionError::InvalidRadius);
    }
    if matrix.iter().flatten().any(|value| !value.is_finite())
        || linear.iter().any(|value| !value.is_finite())
    {
        return Err(TrustRegionError::NonFiniteInput);
    }

    let scale = matrix
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(1.0_f64, f64::max);
    for row in 0..n {
        for column in 0..n {
            if (matrix[row][column] - matrix[column][row]).abs()
                > SYMMETRY_TOLERANCE * scale
            {
                return Err(TrustRegionError::AsymmetricMatrix);
            }
        }
    }

    let (eigenvalues, eigenvectors) = symmetric_eigen(matrix.to_vec())
        .map_err(|_: NetworkError| TrustRegionError::EigensolverFailure)?;
    if eigenvalues.len() != n
        || eigenvectors.len() != n
        || eigenvectors.iter().any(|row| row.len() != n)
        || eigenvalues.iter().any(|value| !value.is_finite())
        || eigenvectors.iter().flatten().any(|value| !value.is_finite())
    {
        return Err(TrustRegionError::EigensolverFailure);
    }

    let eigen_scale = eigenvalues
        .iter()
        .map(|value| value.abs())
        .fold(1.0_f64, f64::max);
    let psd_tolerance = EIGEN_TOLERANCE * eigen_scale;
    if eigenvalues.iter().any(|value| *value < -psd_tolerance) {
        return Err(TrustRegionError::IndefiniteMatrix);
    }
    let largest_eigenvalue = eigenvalues
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max)
        .max(0.0);
    let eigen_tolerance = EIGEN_TOLERANCE * (1.0 + largest_eigenvalue);
    let top_indices: Vec<usize> = eigenvalues
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            ((*value - largest_eigenvalue).abs() <= eigen_tolerance).then_some(index)
        })
        .collect();

    let mut rotated_linear = vec![0.0; n];
    for column in 0..n {
        rotated_linear[column] = (0..n)
            .map(|row| eigenvectors[row][column] * linear[row])
            .sum();
    }
    let linear_norm = vector_norm(linear);
    let linear_tolerance = EIGEN_TOLERANCE * (1.0 + linear_norm);
    let top_linear_norm = top_indices
        .iter()
        .map(|index| rotated_linear[*index] * rotated_linear[*index])
        .sum::<f64>()
        .sqrt();

    let matrix_norm = eigenvalues
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    if matrix_norm <= linear_tolerance && linear_norm <= linear_tolerance {
        return Ok(TrustRegionSolutionV1 {
            point: vec![0.0; n],
            multiplier: 0.0,
            largest_eigenvalue,
            objective_without_constant: 0.0,
            point_norm: 0.0,
            radius,
            stationarity_residual: 0.0,
            feasibility_residual: 0.0,
            secular_residual: 0.0,
            iterations: 0,
            disposition: TrustRegionDisposition::FlatObjective,
        });
    }

    let top_mask: Vec<bool> = (0..n)
        .map(|index| top_indices.contains(&index))
        .collect();
    let lower_at_top: Vec<f64> = (0..n)
        .map(|index| {
            if top_mask[index] {
                0.0
            } else {
                rotated_linear[index] / (largest_eigenvalue - eigenvalues[index])
            }
        })
        .collect();
    let lower_norm = vector_norm(&lower_at_top);
    let radius_tolerance = RESIDUAL_TOLERANCE * (1.0 + radius);
    let hard_case = top_linear_norm <= linear_tolerance
        && lower_norm <= radius + radius_tolerance;

    let (multiplier, point_eigen, iterations, disposition) = if hard_case {
        let mut solution = lower_at_top;
        let remaining = (radius * radius - lower_norm * lower_norm)
            .max(0.0)
            .sqrt();
        let chosen_top = *top_indices
            .first()
            .ok_or(TrustRegionError::EigensolverFailure)?;
        solution[chosen_top] = remaining;
        (
            largest_eigenvalue,
            solution,
            0,
            TrustRegionDisposition::HardCaseBoundaryOptimum,
        )
    } else {
        let norm_at = |lambda: f64| -> f64 {
            (0..n)
                .map(|index| {
                    let denominator = lambda - eigenvalues[index];
                    let value = rotated_linear[index] / denominator;
                    value * value
                })
                .sum::<f64>()
                .sqrt()
        };
        let mut low = largest_eigenvalue;
        let mut high = largest_eigenvalue + 1.0 + linear_norm / radius;
        if !high.is_finite() || high <= low {
            high = low + 1.0;
        }
        let mut bracketed = norm_at(high) <= radius;
        for _ in 0..128 {
            if bracketed {
                break;
            }
            high = largest_eigenvalue + 2.0 * (high - largest_eigenvalue);
            if !high.is_finite() {
                break;
            }
            bracketed = norm_at(high) <= radius;
        }
        if !bracketed {
            return Err(TrustRegionError::RootBracketFailure);
        }
        let mut iterations = 0;
        for _ in 0..MAX_ROOT_ITERATIONS {
            iterations += 1;
            let middle = low + 0.5 * (high - low);
            if middle <= low || middle >= high {
                break;
            }
            if norm_at(middle) > radius {
                low = middle;
            } else {
                high = middle;
            }
            if (high - low).abs() <= 1.0e-13 * (1.0 + high.abs()) {
                break;
            }
        }
        let multiplier = high;
        let solution: Vec<f64> = (0..n)
            .map(|index| rotated_linear[index] / (multiplier - eigenvalues[index]))
            .collect();
        (
            multiplier,
            solution,
            iterations,
            TrustRegionDisposition::RegularBoundaryOptimum,
        )
    };

    let mut point = vec![0.0; n];
    for row in 0..n {
        point[row] = (0..n)
            .map(|column| eigenvectors[row][column] * point_eigen[column])
            .sum();
    }
    let matrix_point = matrix_vector_product(matrix, &point);
    let stationarity: Vec<f64> = matrix_point
        .iter()
        .zip(linear.iter())
        .zip(point.iter())
        .map(|((ax, gx), x)| ax + gx - multiplier * x)
        .collect();
    let stationarity_residual = vector_norm(&stationarity);
    let point_norm = vector_norm(&point);
    let feasibility_residual = (point_norm - radius).max(0.0);
    let secular_residual = (point_norm - radius).abs();
    let objective_without_constant = 0.5 * dot(&point, &matrix_point) + dot(linear, &point);
    if !objective_without_constant.is_finite() || !multiplier.is_finite() {
        return Err(TrustRegionError::NonFiniteInput);
    }
    let residual_scale = 1.0 + vector_norm(linear) + matrix_norm * (1.0 + radius);
    if stationarity_residual > RESIDUAL_TOLERANCE * residual_scale
        || feasibility_residual > radius_tolerance
        || secular_residual > radius_tolerance
        || multiplier + eigen_tolerance < largest_eigenvalue
    {
        return Err(TrustRegionError::KktResidualTooLarge);
    }

    Ok(TrustRegionSolutionV1 {
        point,
        multiplier,
        largest_eigenvalue,
        objective_without_constant,
        point_norm,
        radius,
        stationarity_residual,
        feasibility_residual,
        secular_residual,
        iterations,
        disposition,
    })
}

fn vector_norm(values: &[f64]) -> f64 {
    dot(values, values).sqrt()
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right.iter()).map(|(a, b)| a * b).sum()
}

fn matrix_vector_product(matrix: &[Vec<f64>], vector: &[f64]) -> Vec<f64> {
    matrix
        .iter()
        .map(|row| row.iter().zip(vector.iter()).map(|(a, b)| a * b).sum())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn objective(matrix: &[Vec<f64>], linear: &[f64], point: &[f64]) -> f64 {
        let product = matrix_vector_product(matrix, point);
        0.5 * dot(point, &product) + dot(linear, point)
    }

    #[test]
    fn regular_solution_matches_independent_dense_angular_reference() {
        let matrix = vec![vec![3.0, 0.4], vec![0.4, 1.0]];
        let linear = vec![0.3, -0.2];
        let radius = 1.7;
        let solution = maximize_psd_quadratic_on_ball(&matrix, &linear, radius).unwrap();
        let mut dense_best = f64::NEG_INFINITY;
        for index in 0..200_000 {
            let angle = std::f64::consts::TAU * index as f64 / 200_000.0;
            let point = [radius * angle.cos(), radius * angle.sin()];
            dense_best = dense_best.max(objective(&matrix, &linear, &[point[0], point[1]]));
        }
        assert_eq!(solution.disposition, TrustRegionDisposition::RegularBoundaryOptimum);
        assert!((solution.point_norm - radius).abs() < 1.0e-8);
        assert!(objective(&matrix, &linear, &solution.point) + 1.0e-8 >= dense_best);
        assert!((solution.objective_without_constant - dense_best).abs() < 1.0e-7);
        assert!(solution.stationarity_residual < 1.0e-8);
    }

    #[test]
    fn hard_case_completes_radius_in_top_eigenspace() {
        let matrix = vec![vec![3.0, 0.0], vec![0.0, 1.0]];
        let linear = vec![0.0, 1.0];
        let radius = 2.0;
        let solution = maximize_psd_quadratic_on_ball(&matrix, &linear, radius).unwrap();
        let lower = 0.5;
        let top = (radius * radius - lower * lower).sqrt();
        assert_eq!(solution.disposition, TrustRegionDisposition::HardCaseBoundaryOptimum);
        assert!((solution.point[0].abs() - top).abs() < 1.0e-8);
        assert!((solution.point[1] - lower).abs() < 1.0e-8);
        assert!((solution.multiplier - 3.0).abs() < 1.0e-10);
        assert!(solution.stationarity_residual < 1.0e-8);
    }

    #[test]
    fn repeated_top_eigenspace_uses_deterministic_completion() {
        let matrix = vec![
            vec![2.0, 0.0, 0.0],
            vec![0.0, 2.0, 0.0],
            vec![0.0, 0.0, 0.0],
        ];
        let linear = vec![0.0, 0.0, 0.5];
        let solution = maximize_psd_quadratic_on_ball(&matrix, &linear, 1.0).unwrap();
        assert_eq!(solution.disposition, TrustRegionDisposition::HardCaseBoundaryOptimum);
        assert!(solution.point[0] > 0.0);
        assert!(solution.point[1].abs() < 1.0e-10);
        assert!((solution.point[2] - 0.25).abs() < 1.0e-8);
        assert!((solution.point_norm - 1.0).abs() < 1.0e-8);
    }

    #[test]
    fn zero_curvature_with_linear_term_selects_boundary_direction() {
        let matrix = vec![vec![0.0, 0.0], vec![0.0, 0.0]];
        let linear = vec![3.0, 4.0];
        let solution = maximize_psd_quadratic_on_ball(&matrix, &linear, 2.0).unwrap();
        assert_eq!(solution.disposition, TrustRegionDisposition::RegularBoundaryOptimum);
        assert!((solution.point[0] - 1.2).abs() < 1.0e-8);
        assert!((solution.point[1] - 1.6).abs() < 1.0e-8);
        assert!((solution.multiplier - 2.5).abs() < 1.0e-8);
    }

    #[test]
    fn flat_objective_returns_zero_without_fabricating_direction() {
        let matrix = vec![vec![0.0, 0.0], vec![0.0, 0.0]];
        let linear = vec![0.0, 0.0];
        let solution = maximize_psd_quadratic_on_ball(&matrix, &linear, 3.0).unwrap();
        assert_eq!(solution.disposition, TrustRegionDisposition::FlatObjective);
        assert_eq!(solution.point, vec![0.0, 0.0]);
        assert_eq!(solution.objective_without_constant, 0.0);
        assert_eq!(solution.point_norm, 0.0);
    }

    #[test]
    fn invalid_indefinite_asymmetric_and_nonfinite_inputs_reject() {
        assert_eq!(
            maximize_psd_quadratic_on_ball(&[], &[], 1.0),
            Err(TrustRegionError::EmptyMatrix)
        );
        assert_eq!(
            maximize_psd_quadratic_on_ball(&[vec![1.0, 0.0]], &[1.0, 2.0], 1.0),
            Err(TrustRegionError::DimensionMismatch)
        );
        assert_eq!(
            maximize_psd_quadratic_on_ball(
                &[vec![1.0, 0.2], vec![0.0, 1.0]],
                &[0.0, 0.0],
                1.0
            ),
            Err(TrustRegionError::AsymmetricMatrix)
        );
        assert_eq!(
            maximize_psd_quadratic_on_ball(
                &[vec![1.0, 0.0], vec![0.0, -0.1]],
                &[0.0, 0.0],
                1.0
            ),
            Err(TrustRegionError::IndefiniteMatrix)
        );
        assert_eq!(
            maximize_psd_quadratic_on_ball(
                &[vec![1.0, 0.0], vec![0.0, 1.0]],
                &[f64::NAN, 0.0],
                1.0
            ),
            Err(TrustRegionError::NonFiniteInput)
        );
    }
}
