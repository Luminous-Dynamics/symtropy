// SPDX-License-Identifier: AGPL-3.0-or-later
//! Subspace MAC for close/repeated modes.
//!
//! A repeated or tightly clustered eigenspace may produce different individual
//! eigenvectors under small mesh changes even when the physical subspace is
//! stable. This module compares equal-dimensional sets of mode-shape samples
//! after they have been expressed on the same coordinates (for mesh studies,
//! use common physical stations and the same rotation scaling policy).

use crate::modal_bar::{Matrix, generalized_symmetric_eigenpairs};
use crate::Check;
use std::f64::consts::FRAC_PI_2;

const MAX_SUBSPACE_DIMENSION: usize = 32;
const MAX_COORDINATE_DIMENSION: usize = 1_000_000;

/// Principal angles and normalized correlation for two equal-dimensional
/// subspaces represented by mode vectors sampled on the same coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct SubspaceMacResult {
    pub subspace_dimension: usize,
    pub coordinate_dimension: usize,
    /// Cosines of principal angles, sorted descending in [0, 1].
    pub principal_cosines: Vec<f64>,
    /// Principal angles in radians, sorted ascending in [0, pi/2].
    pub principal_angles_radians: Vec<f64>,
    /// Mean squared principal cosine; one is identical subspace, zero is
    /// orthogonal subspace. Invariant to an orthogonal basis rotation.
    pub mean_squared_principal_cosine: f64,
    /// Worst-aligned principal direction.
    pub minimum_principal_cosine: f64,
    pub maximum_principal_angle_radians: f64,
    /// Normalized chordal distance: sqrt(mean(1 - cosine^2)).
    pub normalized_chordal_distance: f64,
    pub rank_tolerance: f64,
}

/// Compare two equal-rank modal subspaces using principal angles.
///
/// Each outer entry is one mode vector; every mode must have the same
/// coordinate dimension. The caller is responsible for choosing a common
/// coordinate system and any unit normalization, e.g. scaling rotations by a
/// reference length before comparing them with translational components.
pub fn subspace_mac(
    reference_modes: &[Vec<f64>],
    candidate_modes: &[Vec<f64>],
    rank_tolerance: f64,
) -> Check<SubspaceMacResult> {
    if reference_modes.is_empty()
        || reference_modes.len() != candidate_modes.len()
        || reference_modes.len() > MAX_SUBSPACE_DIMENSION
    {
        return Err(format!(
            "subspace mode count must match and be in 1..={MAX_SUBSPACE_DIMENSION}"
        ));
    }
    if !rank_tolerance.is_finite() || rank_tolerance <= 0.0 || rank_tolerance > 1.0e-3 {
        return Err("rank tolerance must be finite and in (0, 1e-3]".to_owned());
    }
    let coordinate_dimension = reference_modes[0].len();
    if coordinate_dimension == 0 || coordinate_dimension > MAX_COORDINATE_DIMENSION {
        return Err(format!(
            "coordinate dimension must be in 1..={MAX_COORDINATE_DIMENSION}"
        ));
    }
    if reference_modes.iter().chain(candidate_modes).any(|mode| {
        mode.len() != coordinate_dimension || mode.iter().any(|value| !value.is_finite())
    }) {
        return Err("all mode vectors must share a finite coordinate dimension".to_owned());
    }

    let reference_basis = orthonormal_basis(reference_modes, rank_tolerance)?;
    let candidate_basis = orthonormal_basis(candidate_modes, rank_tolerance)?;
    let dimension = reference_basis.len();

    let mut cross = vec![vec![0.0; dimension]; dimension];
    for row in 0..dimension {
        for col in 0..dimension {
            cross[row][col] = dot(&reference_basis[row], &candidate_basis[col]);
        }
    }

    // Singular values of Q_ref^T Q_cand are the principal-angle cosines.
    // Compute eigenvalues of C^T C with the shared symmetric eigensolver.
    let mut gram: Matrix = vec![vec![0.0; dimension]; dimension];
    for row in 0..dimension {
        for col in 0..dimension {
            gram[row][col] = (0..dimension)
                .map(|inner| cross[inner][row] * cross[inner][col])
                .sum();
        }
    }
    let identity: Matrix = (0..dimension)
        .map(|row| (0..dimension).map(|col| if row == col { 1.0 } else { 0.0 }).collect())
        .collect();
    let eigenpairs = generalized_symmetric_eigenpairs(&gram, &identity)?;
    if eigenpairs.len() != dimension {
        return Err("principal-angle eigensolve returned an unexpected eigenvalue count".to_owned());
    }

    let mut squared_cosines = Vec::with_capacity(dimension);
    for (eigenvalue, _) in eigenpairs {
        if !eigenvalue.is_finite() || eigenvalue < -rank_tolerance || eigenvalue > 1.0 + rank_tolerance {
            return Err(format!(
                "principal-cosine square is outside [0,1] beyond tolerance: {eigenvalue:.17e}"
            ));
        }
        squared_cosines.push(eigenvalue.clamp(0.0, 1.0));
    }
    squared_cosines.sort_by(f64::total_cmp);

    let mut principal_cosines = squared_cosines.iter().map(|value| value.sqrt()).collect::<Vec<_>>();
    principal_cosines.reverse();
    let principal_angles_radians = principal_cosines
        .iter()
        .map(|cosine| cosine.clamp(0.0, 1.0).acos().clamp(0.0, FRAC_PI_2))
        .collect::<Vec<_>>();
    let mean_squared_principal_cosine = squared_cosines.iter().sum::<f64>() / dimension as f64;
    let minimum_principal_cosine = *principal_cosines.last().expect("positive subspace dimension");
    let maximum_principal_angle_radians =
        *principal_angles_radians.last().expect("positive subspace dimension");
    let normalized_chordal_distance =
        (squared_cosines.iter().map(|value| 1.0 - value).sum::<f64>() / dimension as f64).sqrt();

    if !mean_squared_principal_cosine.is_finite()
        || !minimum_principal_cosine.is_finite()
        || !maximum_principal_angle_radians.is_finite()
        || !normalized_chordal_distance.is_finite()
    {
        return Err("subspace MAC produced a non-finite diagnostic".to_owned());
    }

    Ok(SubspaceMacResult {
        subspace_dimension: dimension,
        coordinate_dimension,
        principal_cosines,
        principal_angles_radians,
        mean_squared_principal_cosine,
        minimum_principal_cosine,
        maximum_principal_angle_radians,
        normalized_chordal_distance,
        rank_tolerance,
    })
}

fn orthonormal_basis(modes: &[Vec<f64>], rank_tolerance: f64) -> Check<Vec<Vec<f64>>> {
    let mut basis: Vec<Vec<f64>> = Vec::with_capacity(modes.len());
    for (mode_index, source) in modes.iter().enumerate() {
        let source_norm = dot(source, source).sqrt();
        if !source_norm.is_finite() || source_norm <= f64::MIN_POSITIVE {
            return Err(format!("mode {mode_index} is a zero/non-finite vector"));
        }
        let mut vector = source.clone();
        // Two-pass modified Gram-Schmidt reduces loss of orthogonality for
        // nearly dependent vectors without adding a separate factorization.
        for _ in 0..2 {
            for previous in &basis {
                let coefficient = dot(&vector, previous);
                for (value, component) in vector.iter_mut().zip(previous) {
                    *value -= coefficient * component;
                }
            }
        }
        let residual_norm = dot(&vector, &vector).sqrt();
        if !residual_norm.is_finite() || residual_norm <= rank_tolerance * source_norm {
            return Err(format!(
                "mode {mode_index} is rank-deficient on the supplied common coordinates"
            ));
        }
        for value in &mut vector {
            *value /= residual_norm;
        }
        basis.push(vector);
    }
    Ok(basis)
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

#[cfg(test)]
mod tests {
    use super::subspace_mac;

    const TOLERANCE: f64 = 1.0e-12;

    #[test]
    fn subspace_score_is_invariant_to_orthogonal_basis_rotation() {
        let angle: f64 = 0.713;
        let reference = vec![vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0]];
        let candidate = vec![
            vec![angle.cos(), angle.sin(), 0.0],
            vec![-angle.sin(), angle.cos(), 0.0],
        ];
        let score = subspace_mac(&reference, &candidate, TOLERANCE)
            .expect("same subspace under basis rotation");
        assert!(score.principal_cosines.iter().all(|value| (value - 1.0).abs() < 1.0e-12));
        assert!(score.mean_squared_principal_cosine > 1.0 - 1.0e-12);
        assert!(score.normalized_chordal_distance < 1.0e-12);
    }

    #[test]
    fn orthogonal_subspaces_have_zero_principal_cosines() {
        let reference = vec![vec![1.0, 0.0, 0.0, 0.0], vec![0.0, 1.0, 0.0, 0.0]];
        let candidate = vec![vec![0.0, 0.0, 1.0, 0.0], vec![0.0, 0.0, 0.0, 1.0]];
        let score = subspace_mac(&reference, &candidate, TOLERANCE)
            .expect("orthogonal subspaces");
        assert!(score.principal_cosines.iter().all(|value| *value < 1.0e-12));
        assert!(score.mean_squared_principal_cosine < 1.0e-12);
        assert!((score.maximum_principal_angle_radians - FRAC_PI_2).abs() < 1.0e-12);
        assert!((score.normalized_chordal_distance - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn partial_overlap_matches_closed_form_principal_angles() {
        let theta: f64 = 0.4;
        let reference = vec![vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0]];
        let candidate = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, theta.cos(), theta.sin()],
        ];
        let score = subspace_mac(&reference, &candidate, TOLERANCE)
            .expect("partially overlapping subspaces");
        let expected = vec![1.0, theta.cos()];
        for (actual, expected) in score.principal_cosines.iter().zip(expected.iter()) {
            assert!((actual - expected).abs() < 1.0e-11);
        }
        let expected_mean = (1.0 + theta.cos().powi(2)) / 2.0;
        assert!((score.mean_squared_principal_cosine - expected_mean).abs() < 1.0e-11);
        assert!((score.normalized_chordal_distance - theta.sin().abs() / 2.0_f64.sqrt()).abs() < 1.0e-11);
    }

    #[test]
    fn mode_vector_scaling_does_not_change_the_subspace() {
        let reference = vec![vec![2.0, 0.0, 0.0], vec![0.0, 0.5, 0.0]];
        let candidate = vec![vec![0.0, -3.0, 0.0], vec![4.0, 0.0, 0.0]];
        let score = subspace_mac(&reference, &candidate, TOLERANCE)
            .expect("same coordinate subspace");
        assert!(score.mean_squared_principal_cosine > 1.0 - 1.0e-12);
    }

    #[test]
    fn rank_deficient_and_invalid_requests_fail_closed() {
        assert!(subspace_mac(&[vec![1.0, 0.0], vec![2.0, 0.0]], &[vec![1.0, 0.0], vec![0.0, 1.0]], TOLERANCE).is_err());
        assert!(subspace_mac(&[vec![1.0, 0.0]], &[vec![1.0]], TOLERANCE).is_err());
        assert!(subspace_mac(&[vec![1.0, 0.0]], &[vec![1.0, 0.0], vec![0.0, 1.0]], TOLERANCE).is_err());
        assert!(subspace_mac(&[vec![f64::NAN, 0.0]], &[vec![1.0, 0.0]], TOLERANCE).is_err());
        assert!(subspace_mac(&[vec![1.0, 0.0]], &[vec![1.0, 0.0]], 0.0).is_err());
    }
}
