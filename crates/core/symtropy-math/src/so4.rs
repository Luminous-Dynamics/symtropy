// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Checked principal logarithms and interpolation for SO(4).
//!
//! The logarithm is recovered from invariant eigenspaces of the symmetric
//! part (R + R^T)/2. Distinct principal angles produce distinct invariant
//! planes; equal-angle subspaces are handled as one invariant block, keeping
//! the full skew generator. Exact pi eigenspaces use a deterministic
//! coordinate-projection basis and report the canonical cut-locus branch.
//!
//! This implementation is deliberately SO(4)-specific. It does not claim a
//! general SO(D) real-Schur logarithm or tangent-Jacobian family.

use crate::{Bivector, Rotor};
use nalgebra::{SMatrix, SVector, SymmetricEigen};
use std::error::Error;
use std::f64::consts::PI;
use std::fmt;

/// Numerical policy for a checked SO(4) principal logarithm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct So4LogConfig {
    /// Maximum accepted orthogonality and determinant residual.
    pub rotation_tolerance: f64,
    /// Eigenvalues within this absolute tolerance share one invariant block.
    pub eigenspace_tolerance: f64,
    /// Angles this close to pi use cut-locus classification.
    pub pi_branch_tolerance: f64,
    /// Below this sine magnitude, the pi-axis sign is treated as unresolved.
    pub pi_sign_tolerance: f64,
    /// Angles at or below this threshold are treated as identity.
    pub identity_tolerance: f64,
}

impl Default for So4LogConfig {
    fn default() -> Self {
        Self {
            rotation_tolerance: 1.0e-8,
            eigenspace_tolerance: 1.0e-8,
            pi_branch_tolerance: 1.0e-7,
            pi_sign_tolerance: 1.0e-12,
            identity_tolerance: 1.0e-14,
        }
    }
}

/// Classification of the principal SO(4) logarithm branch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum So4LogBranch {
    Identity,
    Principal,
    NearPi,
    PiCanonical,
}

/// Checked SO(4) logarithm with the two principal-plane angle magnitudes.
///
/// The signed plane orientation is retained in generator; the angle list
/// contains the nonnegative principal magnitudes sorted in ascending order.
#[derive(Clone, Debug, PartialEq)]
pub struct So4Log {
    pub generator: Bivector<4>,
    pub principal_angles_rad: [f64; 2],
    pub branch: So4LogBranch,
}

/// Failures from checked SO(4) logarithms and interpolation.
#[derive(Clone, Debug, PartialEq)]
pub enum So4LogError {
    InvalidConfig,
    NonFiniteMatrix,
    NonFiniteParameter,
    InvalidRotation {
        orthogonality_error: f64,
        determinant: f64,
    },
    InvalidInvariantSubspace {
        dimension: usize,
    },
    DegenerateInvariantSubspace,
    NonFiniteResult,
    ReconstructionFailure {
        max_matrix_error: f64,
    },
}

impl fmt::Display for So4LogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig => write!(f, "invalid SO(4) logarithm configuration"),
            Self::NonFiniteMatrix => write!(f, "SO(4) matrix contains non-finite values"),
            Self::NonFiniteParameter => write!(f, "interpolation parameter must be finite"),
            Self::InvalidRotation {
                orthogonality_error,
                determinant,
            } => write!(
                f,
                "matrix is not in SO(4): orthogonality error {orthogonality_error:e}, determinant {determinant:e}"
            ),
            Self::InvalidInvariantSubspace { dimension } => write!(
                f,
                "SO(4) invariant eigenspace has unsupported dimension {dimension}; expected 2 or 4"
            ),
            Self::DegenerateInvariantSubspace => {
                write!(f, "could not construct a stable canonical invariant-plane basis")
            }
            Self::NonFiniteResult => write!(f, "SO(4) logarithm produced a non-finite result"),
            Self::ReconstructionFailure { max_matrix_error } => write!(
                f,
                "SO(4) logarithm failed reconstruction with max matrix error {max_matrix_error:e}"
            ),
        }
    }
}

impl Error for So4LogError {}

fn config_is_valid(config: So4LogConfig) -> bool {
    config.rotation_tolerance.is_finite()
        && config.rotation_tolerance > 0.0
        && config.eigenspace_tolerance.is_finite()
        && config.eigenspace_tolerance > 0.0
        && config.pi_branch_tolerance.is_finite()
        && config.pi_branch_tolerance >= 0.0
        && config.pi_branch_tolerance < PI
        && config.pi_sign_tolerance.is_finite()
        && config.pi_sign_tolerance >= 0.0
        && config.identity_tolerance.is_finite()
        && config.identity_tolerance >= 0.0
        && config.identity_tolerance < PI
}

fn validate_rotation(
    rotation: &Rotor<4>,
    tolerance: f64,
) -> Result<(), So4LogError> {
    let matrix = rotation.to_matrix();
    if !matrix.iter().all(|value| value.is_finite()) {
        return Err(So4LogError::NonFiniteMatrix);
    }
    let orthogonality_error = rotation.orthogonality_error();
    let determinant = rotation.determinant();
    if !orthogonality_error.is_finite()
        || !determinant.is_finite()
        || orthogonality_error > tolerance
        || (determinant - 1.0).abs() > tolerance
    {
        return Err(So4LogError::InvalidRotation {
            orthogonality_error,
            determinant,
        });
    }
    Ok(())
}

/// Build a deterministic orthonormal basis for a projector range by projecting
/// coordinate axes in fixed order and applying Gram-Schmidt.
fn canonical_projector_basis(
    projector: &SMatrix<f64, 4, 4>,
    rank: usize,
    tolerance: f64,
) -> Result<[SVector<f64, 4>; 4], So4LogError> {
    let mut basis = [SVector::<f64, 4>::zeros(); 4];
    let mut count = 0usize;

    for axis in 0..4 {
        let mut candidate = projector.column(axis).into_owned();
        for existing in basis.iter().take(count) {
            candidate -= existing * existing.dot(&candidate);
        }
        let norm = candidate.norm();
        if norm > tolerance {
            basis[count] = candidate / norm;
            count += 1;
            if count == rank {
                break;
            }
        }
    }

    if count != rank {
        return Err(So4LogError::DegenerateInvariantSubspace);
    }
    Ok(basis)
}

/// Checked principal logarithm on SO(4).
///
/// It returns a bivector generator whose exponential reconstructs the supplied
/// rotation. The two principal-plane angle magnitudes are retained even when
/// the angles are equal.
pub fn so4_log_with_config(
    rotation: &Rotor<4>,
    config: So4LogConfig,
) -> Result<So4Log, So4LogError> {
    if !config_is_valid(config) {
        return Err(So4LogError::InvalidConfig);
    }
    validate_rotation(rotation, config.rotation_tolerance)?;

    let matrix = *rotation.to_matrix();
    let symmetric = (matrix + matrix.transpose()) * 0.5;
    let skew = (matrix - matrix.transpose()) * 0.5;
    let eigen = SymmetricEigen::new(symmetric);

    if !eigen.eigenvalues.iter().all(|value| value.is_finite())
        || !eigen.eigenvectors.iter().all(|value| value.is_finite())
    {
        return Err(So4LogError::NonFiniteResult);
    }

    let mut processed = [false; 4];
    let mut generator = Bivector::<4>::zero();
    let mut principal_angles = [0.0_f64; 2];
    let mut angle_count = 0usize;
    let mut branch = So4LogBranch::Identity;

    for seed in 0..4 {
        if processed[seed] {
            continue;
        }

        let seed_value = eigen.eigenvalues[seed];
        let mut indices = [0usize; 4];
        let mut rank = 0usize;
        for index in 0..4 {
            if !processed[index]
                && (eigen.eigenvalues[index] - seed_value).abs()
                    <= config.eigenspace_tolerance
            {
                processed[index] = true;
                indices[rank] = index;
                rank += 1;
            }
        }

        if rank != 2 && rank != 4 {
            return Err(So4LogError::InvalidInvariantSubspace { dimension: rank });
        }

        let mut projector = SMatrix::<f64, 4, 4>::zeros();
        let mut cosine = 0.0_f64;
        for index in indices.iter().take(rank) {
            let vector = eigen.eigenvectors.column(*index).into_owned();
            projector += vector * vector.transpose();
            cosine += eigen.eigenvalues[*index];
        }
        cosine = (cosine / rank as f64).clamp(-1.0, 1.0);

        let basis = canonical_projector_basis(
            &projector,
            rank,
            config.eigenspace_tolerance * 0.1,
        )?;
        let mut basis_matrix = SMatrix::<f64, 4, 4>::zeros();
        for column in 0..rank {
            basis_matrix.set_column(column, &basis[column]);
        }

        // For a rank-2 or equal-angle rank-4 invariant block,
        // S^T S = sin(theta)^2 I on that subspace.
        let restricted_skew = basis_matrix.transpose() * skew * basis_matrix;
        let mut skew_frobenius_squared = 0.0_f64;
        for row in 0..rank {
            for column in 0..rank {
                skew_frobenius_squared +=
                    restricted_skew[(row, column)] * restricted_skew[(row, column)];
            }
        }
        let sine = (skew_frobenius_squared / rank as f64).sqrt();
        let angle = sine.atan2(cosine);

        if !angle.is_finite() || !sine.is_finite() {
            return Err(So4LogError::NonFiniteResult);
        }

        let plane_count = rank / 2;
        for _ in 0..plane_count {
            if angle_count >= 2 {
                return Err(So4LogError::InvalidInvariantSubspace { dimension: rank });
            }
            principal_angles[angle_count] = angle;
            angle_count += 1;
        }

        if angle <= config.identity_tolerance {
            continue;
        }

        if PI - angle <= config.pi_branch_tolerance
            && sine <= config.pi_sign_tolerance
        {
            // At an exact/numerically unresolved pi block, every 2-plane in
            // this -1 eigenspace exponentiates to -I. Choose a deterministic
            // basis from projected coordinate axes and expose the branch.
            for plane in 0..plane_count {
                let canonical_plane =
                    Bivector::<4>::from_wedge(&basis[2 * plane], &basis[2 * plane + 1]);
                generator = generator.add(&canonical_plane.scale(angle));
            }
            branch = So4LogBranch::PiCanonical;
        } else {
            // A rank-4 equal-angle block keeps its complete skew generator:
            // no arbitrary single-plane selection is made.
            let projected_skew =
                basis_matrix * restricted_skew * basis_matrix.transpose();
            let block_generator =
                Bivector::<4>::from_matrix(&(-projected_skew * (angle / sine)));
            generator = generator.add(&block_generator);

            if PI - angle <= config.pi_branch_tolerance
                && branch != So4LogBranch::PiCanonical
            {
                branch = So4LogBranch::NearPi;
            } else if branch == So4LogBranch::Identity {
                branch = So4LogBranch::Principal;
            }
        }
    }

    if angle_count != 2 || !generator.is_finite() {
        return Err(So4LogError::NonFiniteResult);
    }
    principal_angles.sort_by(f64::total_cmp);

    let reconstructed = Rotor::<4>::from_bivector(&generator);
    let max_matrix_error = reconstructed
        .to_matrix()
        .iter()
        .zip(matrix.iter())
        .map(|(left, right)| (left - right).abs())
        .fold(0.0_f64, f64::max);
    let reconstruction_tolerance = config.rotation_tolerance.max(1.0e-10) * 20.0;
    if !max_matrix_error.is_finite() || max_matrix_error > reconstruction_tolerance {
        return Err(So4LogError::ReconstructionFailure { max_matrix_error });
    }

    Ok(So4Log {
        generator,
        principal_angles_rad: principal_angles,
        branch,
    })
}

/// Checked principal SO(4) logarithm with the default numerical policy.
pub fn so4_log(rotation: &Rotor<4>) -> Result<So4Log, So4LogError> {
    so4_log_with_config(rotation, So4LogConfig::default())
}

/// Interpolate on SO(4) along the branch-qualified principal geodesic.
pub fn so4_interpolate_with_config(
    start: &Rotor<4>,
    end: &Rotor<4>,
    t: f64,
    config: So4LogConfig,
) -> Result<Rotor<4>, So4LogError> {
    if !t.is_finite() {
        return Err(So4LogError::NonFiniteParameter);
    }
    if !config_is_valid(config) {
        return Err(So4LogError::InvalidConfig);
    }
    validate_rotation(start, config.rotation_tolerance)?;
    validate_rotation(end, config.rotation_tolerance)?;

    if t == 0.0 {
        return Ok(start.clone());
    }
    if t == 1.0 {
        return Ok(end.clone());
    }

    let relative = end.compose(&start.reverse());
    let logarithm = so4_log_with_config(&relative, config)?;
    let partial = Rotor::<4>::from_bivector(&logarithm.generator.scale(t));
    let result = partial.compose(start);
    validate_rotation(&result, config.rotation_tolerance)?;
    Ok(result)
}

/// Interpolate on SO(4) with the default numerical policy.
pub fn so4_interpolate(
    start: &Rotor<4>,
    end: &Rotor<4>,
    t: f64,
) -> Result<Rotor<4>, So4LogError> {
    so4_interpolate_with_config(start, end, t, So4LogConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, PI};

    fn matrix_error(left: &Rotor<4>, right: &Rotor<4>) -> f64 {
        left.to_matrix()
            .iter()
            .zip(right.to_matrix().iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0_f64, f64::max)
    }

    #[test]
    fn unequal_double_rotation_preserves_both_principal_planes() {
        let mut generator = Bivector::<4>::zero();
        generator.set(0, 1, 0.73);
        generator.set(2, 3, -1.17);
        let rotation = Rotor::from_bivector(&generator);

        let logarithm = so4_log(&rotation).expect("valid unequal double rotation");
        let reconstructed = Rotor::from_bivector(&logarithm.generator);
        assert!(matrix_error(&rotation, &reconstructed) < 2.0e-10);
        assert!((logarithm.generator.get(0, 1) - 0.73).abs() < 2.0e-10);
        assert!((logarithm.generator.get(2, 3) + 1.17).abs() < 2.0e-10);
        assert!((logarithm.principal_angles_rad[0] - 0.73).abs() < 2.0e-10);
        assert!((logarithm.principal_angles_rad[1] - 1.17).abs() < 2.0e-10);
    }

    #[test]
    fn equal_angle_rank_four_block_retains_both_planes() {
        let mut generator = Bivector::<4>::zero();
        generator.set(0, 1, 0.61);
        generator.set(2, 3, -0.61);
        let rotation = Rotor::from_bivector(&generator);

        let logarithm = so4_log(&rotation).expect("valid equal-angle double rotation");
        let reconstructed = Rotor::from_bivector(&logarithm.generator);
        assert!(matrix_error(&rotation, &reconstructed) < 2.0e-10);
        assert!((logarithm.generator.norm() - 2.0_f64.sqrt() * 0.61).abs() < 2.0e-10);
        assert!(logarithm
            .principal_angles_rad
            .iter()
            .all(|angle| (*angle - 0.61).abs() < 2.0e-10));
    }

    #[test]
    fn exact_pi_plane_uses_canonical_branch_and_reconstructs() {
        let mut generator = Bivector::<4>::zero();
        generator.set(0, 1, PI);
        generator.set(2, 3, -0.4);
        let rotation = Rotor::from_bivector(&generator);

        let logarithm = so4_log(&rotation).expect("valid pi plus generic plane rotation");
        assert_eq!(logarithm.branch, So4LogBranch::PiCanonical);
        let reconstructed = Rotor::from_bivector(&logarithm.generator);
        assert!(matrix_error(&rotation, &reconstructed) < 2.0e-10);
        assert!((logarithm.principal_angles_rad[1] - PI).abs() < 2.0e-10);
    }

    #[test]
    fn equal_pi_double_rotation_has_two_canonical_planes() {
        let mut generator = Bivector::<4>::zero();
        generator.set(0, 1, PI);
        generator.set(2, 3, -PI);
        let rotation = Rotor::from_bivector(&generator);

        let logarithm = so4_log(&rotation).expect("valid -identity SO(4) rotation");
        assert_eq!(logarithm.branch, So4LogBranch::PiCanonical);
        assert!(logarithm
            .principal_angles_rad
            .iter()
            .all(|angle| (*angle - PI).abs() < 2.0e-10));
        assert!(matrix_error(&rotation, &Rotor::from_bivector(&logarithm.generator)) < 2.0e-10);
    }

    #[test]
    fn interpolation_handles_unequal_double_rotation() {
        let start = Rotor::<4>::from_bivector(&Bivector::unit_plane(0, 2).scale(0.2));
        let mut relative_generator = Bivector::<4>::zero();
        relative_generator.set(0, 1, FRAC_PI_2);
        relative_generator.set(2, 3, -0.8);
        let relative = Rotor::from_bivector(&relative_generator);
        let end = relative.compose(&start);

        let midpoint = so4_interpolate(&start, &end, 0.5).expect("valid SO(4) interpolation");
        assert!(midpoint.is_proper_rotation(1.0e-10));
        let midpoint_relative = midpoint.compose(&start.reverse());
        let recovered = so4_log(&midpoint_relative).expect("midpoint relative is valid");
        assert!((recovered.principal_angles_rad[0] - 0.4).abs() < 2.0e-10);
        assert!((recovered.principal_angles_rad[1] - FRAC_PI_2 / 2.0).abs() < 2.0e-10);
    }

    #[test]
    fn rejects_invalid_rotations_and_non_finite_interpolation() {
        let invalid = Rotor::from_matrix(SMatrix::<f64, 4, 4>::identity() * 2.0);
        assert!(matches!(
            so4_log(&invalid),
            Err(So4LogError::InvalidRotation { .. })
        ));
        let identity = Rotor::<4>::identity();
        assert!(matches!(
            so4_interpolate(&identity, &identity, f64::NAN),
            Err(So4LogError::NonFiniteParameter)
        ));
    }

    #[test]
    fn identity_and_generic_rotations_round_trip() {
        let identity = Rotor::<4>::identity();
        assert_eq!(so4_log(&identity).unwrap().branch, So4LogBranch::Identity);

        let mut generator = Bivector::<4>::zero();
        generator.set(0, 3, 0.23);
        generator.set(1, 2, 0.91);
        let rotation = Rotor::from_bivector(&generator);
        let recovered = so4_log(&rotation).expect("valid generic SO(4) rotation");
        assert!(matrix_error(&rotation, &Rotor::from_bivector(&recovered.generator)) < 2.0e-10);
    }
}
