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
    /// Maximum reconstruction error after exponentiating the recovered log.
    pub reconstruction_tolerance: f64,
}

impl Default for So4LogConfig {
    fn default() -> Self {
        Self {
            rotation_tolerance: 1.0e-8,
            eigenspace_tolerance: 1.0e-12,
            pi_branch_tolerance: 1.0e-7,
            pi_sign_tolerance: 1.0e-12,
            identity_tolerance: 1.0e-14,
            reconstruction_tolerance: 1.0e-10,
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
        && config.reconstruction_tolerance.is_finite()
        && config.reconstruction_tolerance > 0.0
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

        let seed_vector = eigen.eigenvectors.column(seed).into_owned();
        let seed_value = eigen.eigenvalues[seed];
        let seed_skew_magnitude = (skew * seed_vector).norm();
        let mut indices = [0usize; 4];
        let mut rank = 0usize;
        for index in 0..4 {
            if processed[index] {
                continue;
            }
            let candidate_vector = eigen.eigenvectors.column(index).into_owned();
            let candidate_skew_magnitude = (skew * candidate_vector).norm();
            // The symmetric-part eigenvalue is cos(theta); the skew magnitude
            // is |sin(theta)|. Use both invariants. Near pi, distinct angles
            // can have indistinguishable cosines while their sine magnitudes
            // remain well separated.
            if (eigen.eigenvalues[index] - seed_value).abs()
                <= config.eigenspace_tolerance
                && (candidate_skew_magnitude - seed_skew_magnitude).abs()
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
    if !max_matrix_error.is_finite()
        || max_matrix_error > config.reconstruction_tolerance
    {
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
    fn near_pi_planes_with_nearly_equal_cosines_remain_separate() {
        let mut generator = Bivector::<4>::zero();
        generator.set(0, 1, PI - 1.0e-7);
        generator.set(2, 3, -(PI - 2.0e-7));
        let rotation = Rotor::from_bivector(&generator);

        let logarithm = so4_log(&rotation)
            .expect("near-pi planes must be separated by the skew invariant");
        assert!((logarithm.principal_angles_rad[0] - (PI - 2.0e-7)).abs() < 2.0e-10);
        assert!((logarithm.principal_angles_rad[1] - (PI - 1.0e-7)).abs() < 2.0e-10);
        assert!((logarithm.generator.get(0, 1) - (PI - 1.0e-7)).abs() < 2.0e-10);
        assert!((logarithm.generator.get(2, 3) - (PI - 2.0e-7)).abs() < 2.0e-10);
        assert!(matrix_error(&rotation, &Rotor::from_bivector(&logarithm.generator)) < 1.0e-10);
    }

    #[test]
    fn near_but_distinct_principal_angles_remain_separate_invariant_planes() {
        let mut generator = Bivector::<4>::zero();
        generator.set(0, 1, 0.7);
        generator.set(2, 3, 0.700_000_01);
        let rotation = Rotor::from_bivector(&generator);

        let logarithm = so4_log(&rotation).expect("near-equal angles remain resolvable");
        assert!((logarithm.principal_angles_rad[0] - 0.7).abs() < 2.0e-10);
        assert!((logarithm.principal_angles_rad[1] - 0.700_000_01).abs() < 2.0e-10);
        assert!(matrix_error(&rotation, &Rotor::from_bivector(&logarithm.generator)) <= 1.0e-10);
    }

    #[test]
    fn deterministic_random_so4_logs_reconstruct_across_plane_combinations() {
        let mut state = 0x7a31_5eed_d15c_a11e_u64;
        let mut next_angle = || {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            let bits = state.wrapping_mul(0x2545_f491_4f6c_dd1d);
            let unit = (bits >> 11) as f64 / ((1_u64 << 53) as f64);
            (2.0 * unit - 1.0) * 2.8
        };

        for case in 0..512 {
            let mut generator = Bivector::<4>::zero();
            generator.set(0, 1, next_angle());
            generator.set(0, 2, next_angle());
            generator.set(0, 3, next_angle());
            generator.set(1, 2, next_angle());
            generator.set(1, 3, next_angle());
            generator.set(2, 3, next_angle());
            let rotation = Rotor::from_bivector(&generator);
            let logarithm = so4_log(&rotation)
                .unwrap_or_else(|error| panic!("case {case} failed: {error}"));
            let reconstructed = Rotor::from_bivector(&logarithm.generator);
            assert!(
                matrix_error(&rotation, &reconstructed)
                    <= So4LogConfig::default().reconstruction_tolerance,
                "case {case} reconstruction error {}",
                matrix_error(&rotation, &reconstructed)
            );
        }
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

    fn next_random(state: &mut u64) -> f64 {
        *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = *state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^= value >> 31;
        (value >> 11) as f64 / ((1_u64 << 53) as f64)
    }

    fn random_orthogonal_matrix(state: &mut u64) -> SMatrix<f64, 4, 4> {
        let mut columns = Vec::with_capacity(4);
        for _ in 0..4 {
            let mut candidate = SVector::<f64, 4>::new(
                2.0 * next_random(state) - 1.0,
                2.0 * next_random(state) - 1.0,
                2.0 * next_random(state) - 1.0,
                2.0 * next_random(state) - 1.0,
            );
            for previous in &columns {
                candidate -= previous * previous.dot(&candidate);
            }
            let norm = candidate.norm();
            assert!(norm > 1.0e-8, "deterministic QR fixture became degenerate");
            columns.push(candidate / norm);
        }

        let mut orthogonal = SMatrix::<f64, 4, 4>::zeros();
        for (index, column) in columns.iter().enumerate() {
            orthogonal.set_column(index, column);
        }
        if orthogonal.determinant() < 0.0 {
            let last_column = -orthogonal.column(3).into_owned();
            orthogonal.set_column(3, &last_column);
        }
        orthogonal
    }

    fn independent_double_rotation(
        first_angle: f64,
        second_angle: f64,
    ) -> SMatrix<f64, 4, 4> {
        let mut rotation = SMatrix::<f64, 4, 4>::identity();
        for (first, second, angle) in [
            (0usize, 1usize, first_angle),
            (2usize, 3usize, second_angle),
        ] {
            let cosine = angle.cos();
            let sine = angle.sin();
            rotation[(first, first)] = cosine;
            rotation[(first, second)] = -sine;
            rotation[(second, first)] = sine;
            rotation[(second, second)] = cosine;
        }
        rotation
    }

    #[test]
    fn randomized_conjugated_double_rotations_reconstruct_both_planes() {
        let mut rng = 0x243f_6a88_85a3_08d3_u64;
        for case in 0..512 {
            let mut first_angle = 0.03 + 2.9 * next_random(&mut rng);
            let mut second_angle = 0.03 + 2.9 * next_random(&mut rng);
            if (first_angle - second_angle).abs() < 0.04 {
                second_angle = (second_angle + 0.11).min(3.0);
            }
            let first_sign = if next_random(&mut rng) < 0.5 { -1.0 } else { 1.0 };
            let second_sign = if next_random(&mut rng) < 0.5 { -1.0 } else { 1.0 };
            first_angle *= first_sign;
            second_angle *= second_sign;

            // Construct the reference rotation with independent 2x2
            // trigonometric blocks, then conjugate by an independent
            // deterministic orthogonal basis. No Rotor exponential is used
            // to create the reference matrix.
            let basis = random_orthogonal_matrix(&mut rng);
            let canonical = independent_double_rotation(first_angle, second_angle);
            let reference_matrix = basis * canonical * basis.transpose();
            let reference = Rotor::<4>::from_matrix(reference_matrix);

            let logarithm = so4_log(&reference)
                .unwrap_or_else(|error| panic!("case {case} failed: {error}"));
            assert_eq!(logarithm.branch, So4LogBranch::Principal);
            let reconstructed = Rotor::<4>::from_bivector(&logarithm.generator);
            assert!(
                matrix_error(&reference, &reconstructed) < 3.0e-9,
                "reconstruction error in randomized case {case}"
            );

            let expected = [first_angle.abs(), second_angle.abs()];
            let mut expected_sorted = expected;
            expected_sorted.sort_by(f64::total_cmp);
            assert!(
                (logarithm.principal_angles_rad[0] - expected_sorted[0]).abs() < 3.0e-9,
                "first principal-angle mismatch in case {case}"
            );
            assert!(
                (logarithm.principal_angles_rad[1] - expected_sorted[1]).abs() < 3.0e-9,
                "second principal-angle mismatch in case {case}"
            );
        }
    }

}
