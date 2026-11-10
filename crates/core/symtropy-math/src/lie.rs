// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Branch-aware differential geometry for SO(3).
//!
//! Rotations are configurations in SO(3); infinitesimal updates are bivectors
//! in its tangent algebra so(3). This module keeps the logarithm's branch
//! decision explicit instead of treating an ambient matrix difference as a
//! globally valid rotation vector.
//!
//! The principal logarithm is not unique at angle pi. At that cut locus, this
//! implementation chooses a deterministic axis sign (largest-magnitude
//! coordinate positive) when the skew part cannot reliably determine the
//! sign, and reports `So3LogBranch::PiCanonical`. Near pi, when the skew part
//! still determines orientation, that orientation is preserved and reported
//! as `So3LogBranch::NearPi`.
//!
//! This is an SO(3)-only P0. It provides branch-aware logarithms, local
//! plus/minus, geodesic interpolation, and left/right exponential Jacobians.
//! It does not claim a complete SO(D) logarithm, tangent transport, or SE(D)
//! adjoint/coadjoint APIs.

use crate::{Bivector, Rotor};
use nalgebra::{SMatrix, SVector};
use std::error::Error;
use std::f64::consts::PI;
use std::fmt;

/// Numerical policy for checked SO(3) logarithms and interpolation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct So3LogConfig {
    /// Maximum absolute orthogonality and determinant residual accepted.
    pub rotation_tolerance: f64,
    /// Angles this close to pi use the near-cut-locus axis extraction path.
    pub pi_branch_tolerance: f64,
    /// Below this skew/axis alignment, the pi-axis sign is treated as ambiguous.
    pub pi_sign_tolerance: f64,
    /// Angles at or below this threshold are treated as identity.
    pub identity_tolerance: f64,
}

impl Default for So3LogConfig {
    fn default() -> Self {
        Self {
            rotation_tolerance: 1.0e-8,
            pi_branch_tolerance: 1.0e-7,
            pi_sign_tolerance: 1.0e-12,
            identity_tolerance: 1.0e-15,
        }
    }
}

/// The branch used to construct an SO(3) logarithm.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum So3LogBranch {
    /// The input is identity to the configured angular tolerance.
    Identity,
    /// The ordinary principal branch, away from the pi cut locus.
    Principal,
    /// Near pi, with the axis sign recovered from the antisymmetric part.
    NearPi,
    /// At the pi cut locus, with a deterministic canonical axis sign.
    PiCanonical,
}

/// Which tangent-space trivialization is used for a local SO(3) update.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum So3Trivialization {
    /// Spatial/left update: R_plus = Exp(delta) R.
    LeftSpatial,
    /// Body/right update: R_plus = R Exp(delta).
    RightBody,
}

/// A checked SO(3) logarithm and its branch metadata.
///
/// `generator` is the bivector whose exponential reconstructs the input
/// rotation using Symtropy's convention `Rotor::from_bivector`.
#[derive(Clone, Debug, PartialEq)]
pub struct So3Log {
    pub generator: Bivector<3>,
    pub angle: f64,
    pub branch: So3LogBranch,
}

/// Failures from checked SO(3) logarithm or geodesic interpolation.
#[derive(Clone, Debug, PartialEq)]
pub enum So3LogError {
    InvalidConfig,
    NonFiniteMatrix,
    NonFiniteParameter,
    NonFiniteTangent,
    InvalidRotation {
        orthogonality_error: f64,
        determinant: f64,
    },
    DegeneratePiAxis,
    SingularExponentialJacobian { angle: f64 },
    IllConditionedExponentialJacobian { angle: f64, condition_estimate: f64 },
    NonFiniteResult,
}

impl fmt::Display for So3LogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig => write!(f, "invalid SO(3) logarithm configuration"),
            Self::NonFiniteMatrix => write!(f, "rotation matrix contains non-finite values"),
            Self::NonFiniteParameter => write!(f, "interpolation parameter must be finite"),
            Self::NonFiniteTangent => write!(f, "SO(3) tangent perturbation must be finite"),
            Self::InvalidRotation {
                orthogonality_error,
                determinant,
            } => write!(
                f,
                "matrix is not in SO(3): orthogonality error {orthogonality_error:e}, determinant {determinant:e}"
            ),
            Self::DegeneratePiAxis => write!(f, "could not recover a stable axis at the pi branch"),
            Self::SingularExponentialJacobian { angle } => write!(f, "SO(3) exponential Jacobian is singular at angle {angle:e}"),
            Self::IllConditionedExponentialJacobian { angle, condition_estimate } => write!(f, "SO(3) exponential Jacobian inverse is ill-conditioned at angle {angle:e} (condition estimate {condition_estimate:e})"),
            Self::NonFiniteResult => write!(f, "SO(3) logarithm produced a non-finite result"),
        }
    }
}

impl Error for So3LogError {}

fn config_is_valid(config: So3LogConfig) -> bool {
    config.rotation_tolerance.is_finite()
        && config.rotation_tolerance > 0.0
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
    rotation: &Rotor<3>,
    tolerance: f64,
) -> Result<(), So3LogError> {
    let matrix = rotation.to_matrix();
    if !matrix.iter().all(|value| value.is_finite()) {
        return Err(So3LogError::NonFiniteMatrix);
    }

    let orthogonality_error = rotation.orthogonality_error();
    let determinant = rotation.determinant();
    if !orthogonality_error.is_finite()
        || !determinant.is_finite()
        || orthogonality_error > tolerance
        || (determinant - 1.0).abs() > tolerance
    {
        return Err(So3LogError::InvalidRotation {
            orthogonality_error,
            determinant,
        });
    }
    Ok(())
}

/// The standard skew matrix K(a) satisfying K(a) v = a cross v.
fn skew_from_axis(axis: &SVector<f64, 3>) -> SMatrix<f64, 3, 3> {
    let mut matrix = SMatrix::<f64, 3, 3>::zeros();
    matrix[(0, 1)] = -axis[2];
    matrix[(0, 2)] = axis[1];
    matrix[(1, 0)] = axis[2];
    matrix[(1, 2)] = -axis[0];
    matrix[(2, 0)] = -axis[1];
    matrix[(2, 1)] = axis[0];
    matrix
}

/// Recover the signless rotation axis from the symmetric part near angle pi.
fn axis_from_pi_matrix(matrix: &SMatrix<f64, 3, 3>) -> Result<SVector<f64, 3>, So3LogError> {
    let mut pivot = 0;
    let mut largest_diagonal = -1.0_f64;
    for (index, row) in matrix.row_iter().enumerate() {
        let candidate = ((row[index] + 1.0) * 0.5).clamp(0.0, 1.0);
        if candidate > largest_diagonal {
            largest_diagonal = candidate;
            pivot = index;
        }
    }

    let pivot_value = largest_diagonal.sqrt();
    if !pivot_value.is_finite() || pivot_value <= 1.0e-15 {
        return Err(So3LogError::DegeneratePiAxis);
    }

    let mut axis = SVector::<f64, 3>::zeros();
    axis[pivot] = pivot_value;
    for (index, component) in axis.iter_mut().enumerate() {
        if index != pivot {
            *component = (matrix[(pivot, index)] + matrix[(index, pivot)])
                / (4.0 * pivot_value);
        }
    }

    let norm = axis.norm();
    if !norm.is_finite() || norm <= 1.0e-15 {
        return Err(So3LogError::DegeneratePiAxis);
    }
    Ok(axis / norm)
}

/// Return a checked principal SO(3) logarithm with explicit cut-locus metadata.
///
/// The matrix must satisfy `R^T R ≈ I` and `det(R) ≈ +1` under `config`.
/// The output bivector obeys `Rotor::from_bivector(&result.generator) ≈ R`.
pub fn so3_log_with_config(
    rotation: &Rotor<3>,
    config: So3LogConfig,
) -> Result<So3Log, So3LogError> {
    if !config_is_valid(config) {
        return Err(So3LogError::InvalidConfig);
    }
    validate_rotation(rotation, config.rotation_tolerance)?;

    let matrix = *rotation.to_matrix();
    // With Symtropy's sign convention, -skew(R) is sin(theta) times
    // the bivector generator plane.
    let skew = (matrix - matrix.transpose()) * 0.5;
    let plane = Bivector::from_matrix(&(-skew));
    let sin_theta = plane.norm();
    let cos_theta = ((matrix.trace() - 1.0) * 0.5).clamp(-1.0, 1.0);
    let angle = sin_theta.atan2(cos_theta);

    if !angle.is_finite() || !sin_theta.is_finite() {
        return Err(So3LogError::NonFiniteResult);
    }

    if angle <= config.identity_tolerance {
        return Ok(So3Log {
            generator: Bivector::zero(),
            angle,
            branch: So3LogBranch::Identity,
        });
    }

    let (generator, branch) = if PI - angle <= config.pi_branch_tolerance {
        let mut axis = axis_from_pi_matrix(&matrix)?;
        let candidate_skew = skew_from_axis(&axis);
        let alignment = candidate_skew
            .iter()
            .zip(skew.iter())
            .map(|(candidate, observed)| candidate * observed)
            .sum::<f64>();

        let branch = if alignment.abs() > config.pi_sign_tolerance {
            if alignment < 0.0 {
                axis = -axis;
            }
            So3LogBranch::NearPi
        } else {
            // At exactly pi, both axis signs produce the same rotation.
            // The largest-magnitude component was made positive by choosing
            // the positive square root for the pivot coordinate.
            So3LogBranch::PiCanonical
        };

        let generator_matrix = -skew_from_axis(&axis) * angle;
        (Bivector::from_matrix(&generator_matrix), branch)
    } else {
        if sin_theta <= f64::MIN_POSITIVE {
            return Err(So3LogError::NonFiniteResult);
        }
        (
            plane.scale(angle / sin_theta),
            So3LogBranch::Principal,
        )
    };

    if !generator.is_finite() {
        return Err(So3LogError::NonFiniteResult);
    }

    Ok(So3Log {
        generator,
        angle,
        branch,
    })
}

/// Return a checked principal SO(3) logarithm using the default numerical policy.
pub fn so3_log(rotation: &Rotor<3>) -> Result<So3Log, So3LogError> {
    so3_log_with_config(rotation, So3LogConfig::default())
}


/// Apply a tangent perturbation using an explicit left/spatial or right/body
/// trivialization. The supplied bivector is an intrinsic so(3) coordinate,
/// not an ambient perturbation of the 3x3 rotation matrix.
pub fn so3_plus_with_config(
    base: &Rotor<3>,
    delta: &Bivector<3>,
    trivialization: So3Trivialization,
    config: So3LogConfig,
) -> Result<Rotor<3>, So3LogError> {
    if !config_is_valid(config) {
        return Err(So3LogError::InvalidConfig);
    }
    validate_rotation(base, config.rotation_tolerance)?;
    if !delta.is_finite() {
        return Err(So3LogError::NonFiniteTangent);
    }

    let increment = Rotor::<3>::from_bivector(delta);
    let result = match trivialization {
        So3Trivialization::LeftSpatial => increment.compose(base),
        So3Trivialization::RightBody => base.compose(&increment),
    };
    validate_rotation(&result, config.rotation_tolerance)?;
    Ok(result)
}

/// Apply a tangent perturbation using the default numerical policy.
pub fn so3_plus(
    base: &Rotor<3>,
    delta: &Bivector<3>,
    trivialization: So3Trivialization,
) -> Result<Rotor<3>, So3LogError> {
    so3_plus_with_config(base, delta, trivialization, So3LogConfig::default())
}

/// Compute the tangent difference that moves `from` to `to`.
///
/// The result is branch-qualified. With left/spatial trivialization it is
/// Log(to * from^-1); with right/body trivialization it is
/// Log(from^-1 * to). It is inverse to the matching `so3_plus` operation
/// within the declared principal-log injectivity region.
pub fn so3_minus_with_config(
    from: &Rotor<3>,
    to: &Rotor<3>,
    trivialization: So3Trivialization,
    config: So3LogConfig,
) -> Result<So3Log, So3LogError> {
    if !config_is_valid(config) {
        return Err(So3LogError::InvalidConfig);
    }
    validate_rotation(from, config.rotation_tolerance)?;
    validate_rotation(to, config.rotation_tolerance)?;
    let relative = match trivialization {
        So3Trivialization::LeftSpatial => to.compose(&from.reverse()),
        So3Trivialization::RightBody => from.reverse().compose(to),
    };
    so3_log_with_config(&relative, config)
}

/// Compute the tangent difference using the default numerical policy.
pub fn so3_minus(
    from: &Rotor<3>,
    to: &Rotor<3>,
    trivialization: So3Trivialization,
) -> Result<So3Log, So3LogError> {
    so3_minus_with_config(from, to, trivialization, So3LogConfig::default())
}

/// Interpolate on SO(3) along the principal geodesic from `start` to `end`.
///
/// `t = 0` and `t = 1` return the original endpoints exactly. Finite values
/// outside [0, 1] are accepted as geodesic extrapolation. Endpoint rotations
/// are validated before relative composition.
pub fn so3_interpolate_with_config(
    start: &Rotor<3>,
    end: &Rotor<3>,
    t: f64,
    config: So3LogConfig,
) -> Result<Rotor<3>, So3LogError> {
    if !t.is_finite() {
        return Err(So3LogError::NonFiniteParameter);
    }
    if !config_is_valid(config) {
        return Err(So3LogError::InvalidConfig);
    }
    validate_rotation(start, config.rotation_tolerance)?;
    validate_rotation(end, config.rotation_tolerance)?;

    if t == 0.0 {
        return Ok(start.clone());
    }
    if t == 1.0 {
        return Ok(end.clone());
    }

    // Rotors compose as matrix products. This is the spatial/left relative
    // increment: R_rel = R_end R_start^T, then R(t) = exp(t Log(R_rel)) R_start.
    let relative = end.compose(&start.reverse());
    let logarithm = so3_log_with_config(&relative, config)?;
    let partial = Rotor::<3>::from_bivector(&logarithm.generator.scale(t));
    let result = partial.compose(start);

    if result.to_matrix().iter().all(|value| value.is_finite()) {
        Ok(result)
    } else {
        Err(So3LogError::NonFiniteResult)
    }
}

/// Interpolate on SO(3) with the default numerical policy.
pub fn so3_interpolate(
    start: &Rotor<3>,
    end: &Rotor<3>,
    t: f64,
) -> Result<Rotor<3>, So3LogError> {
    so3_interpolate_with_config(start, end, t, So3LogConfig::default())
}


/// A left- or right-trivialized differential of the SO(3) exponential map.
///
/// Matrix rows and columns use Symtropy's lexicographic bivector coordinate
/// order [e01, e02, e12]. LeftSpatial maps additive generator perturbations to
/// spatial/left perturbations; RightBody maps them to body/right perturbations.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct So3Jacobian {
    matrix: SMatrix<f64, 3, 3>,
    trivialization: So3Trivialization,
    inverse: bool,
    angle: f64,
}

impl So3Jacobian {
    /// Read the immutable matrix in [e01, e02, e12] row/column order.
    pub fn matrix(&self) -> &SMatrix<f64, 3, 3> {
        &self.matrix
    }

    /// Read the tangent trivialization represented by this map.
    pub fn trivialization(&self) -> So3Trivialization {
        self.trivialization
    }

    /// Return whether this object is the inverse exponential Jacobian.
    pub fn is_inverse(&self) -> bool {
        self.inverse
    }

    /// Read the norm of the exponential-coordinate generator in radians.
    pub fn angle_rad(&self) -> f64 {
        self.angle
    }

    /// Apply this tangent map to a bivector in [e01, e02, e12] order.
    pub fn apply(&self, tangent: &Bivector<3>) -> Result<Bivector<3>, So3LogError> {
        if !tangent.is_finite() {
            return Err(So3LogError::NonFiniteTangent);
        }
        let coordinates = SVector::<f64, 3>::new(
            tangent.get(0, 1),
            tangent.get(0, 2),
            tangent.get(1, 2),
        );
        let mapped = self.matrix * coordinates;
        if !mapped.iter().all(|value| value.is_finite()) {
            return Err(So3LogError::NonFiniteResult);
        }
        let mut result = Bivector::<3>::zero();
        result.set(0, 1, mapped[0]);
        result.set(0, 2, mapped[1]);
        result.set(1, 2, mapped[2]);
        Ok(result)
    }
}

/// Convert [e01, e02, e12] to the conventional SO(3) rotation vector [x,y,z].
fn so3_rotation_vector(generator: &Bivector<3>) -> SVector<f64, 3> {
    SVector::new(
        generator.get(1, 2),
        -generator.get(0, 2),
        generator.get(0, 1),
    )
}

fn so3_bivector_coordinate_map() -> SMatrix<f64, 3, 3> {
    // phi = C * [b01, b02, b12], with C orthogonal.
    let mut c = SMatrix::<f64, 3, 3>::zeros();
    c[(0, 2)] = 1.0;
    c[(1, 1)] = -1.0;
    c[(2, 0)] = 1.0;
    c
}

fn so3_jacobian_coefficients(theta: f64) -> (f64, f64) {
    let theta2 = theta * theta;
    if theta < 1.0e-3 {
        let theta4 = theta2 * theta2;
        let theta6 = theta4 * theta2;
        (
            0.5 - theta2 / 24.0 + theta4 / 720.0 - theta6 / 40_320.0,
            1.0 / 6.0 - theta2 / 120.0 + theta4 / 5_040.0
                - theta6 / 362_880.0,
        )
    } else {
        (
            (1.0 - theta.cos()) / theta2,
            (theta - theta.sin()) / (theta2 * theta),
        )
    }
}

/// Compute the local differential of Exp for the selected trivialization.
///
/// LeftSpatial: Exp(phi + dphi) ~= Exp(J_l(phi) dphi) Exp(phi).
/// RightBody: Exp(phi + dphi) ~= Exp(phi) Exp(J_r(phi) dphi).
///
/// The input and output coordinates use Symtropy's bivector basis. This is a
/// local derivative; it does not differentiate across a logarithm branch.
pub fn so3_exp_jacobian(
    generator: &Bivector<3>,
    trivialization: So3Trivialization,
) -> Result<So3Jacobian, So3LogError> {
    if !generator.is_finite() {
        return Err(So3LogError::NonFiniteTangent);
    }
    let phi = so3_rotation_vector(generator);
    let theta = phi.norm();
    if !theta.is_finite() {
        return Err(So3LogError::NonFiniteResult);
    }
    let (a, b) = so3_jacobian_coefficients(theta);
    let k = skew_from_axis(&phi);
    let identity = SMatrix::<f64, 3, 3>::identity();
    let ad_matrix = match trivialization {
        So3Trivialization::LeftSpatial => identity + k * a + (k * k) * b,
        So3Trivialization::RightBody => identity - k * a + (k * k) * b,
    };
    let coordinate_map = so3_bivector_coordinate_map();
    let matrix = coordinate_map.transpose() * ad_matrix * coordinate_map;
    if !matrix.iter().all(|value| value.is_finite()) {
        return Err(So3LogError::NonFiniteResult);
    }
    Ok(So3Jacobian {
        matrix,
        trivialization,
        inverse: false,
        angle: theta,
    })
}

/// Compute the inverse differential of Exp for the selected trivialization.
///
/// The inverse is singular at nonzero integer multiples of 2π. It fails closed
/// near those singularities instead of returning huge, misleading derivatives.
/// The conservative condition-estimate ceiling is 1e8; this is an API safety
/// threshold, not a universal numerical-analysis constant.
pub fn so3_exp_jacobian_inverse(
    generator: &Bivector<3>,
    trivialization: So3Trivialization,
) -> Result<So3Jacobian, So3LogError> {
    if !generator.is_finite() {
        return Err(So3LogError::NonFiniteTangent);
    }
    let phi = so3_rotation_vector(generator);
    let theta = phi.norm();
    if !theta.is_finite() {
        return Err(So3LogError::NonFiniteResult);
    }

    let c = if theta < 1.0e-3 {
        let theta2 = theta * theta;
        let theta4 = theta2 * theta2;
        let theta6 = theta4 * theta2;
        1.0 / 12.0 + theta2 / 720.0 + theta4 / 30_240.0
            + theta6 / 1_209_600.0
    } else {
        let half_sine = (0.5 * theta).sin().abs();
        let transverse_gain = 2.0 * half_sine / theta;
        let condition_estimate = 1.0 / transverse_gain;
        if !condition_estimate.is_finite() || transverse_gain <= 128.0 * f64::EPSILON {
            return Err(So3LogError::SingularExponentialJacobian { angle: theta });
        }
        if condition_estimate > 1.0e8 {
            return Err(So3LogError::IllConditionedExponentialJacobian {
                angle: theta,
                condition_estimate,
            });
        }
        1.0 / (theta * theta)
            - (0.5 * theta).cos() / (2.0 * theta * (0.5 * theta).sin())
    };

    let k = skew_from_axis(&phi);
    let identity = SMatrix::<f64, 3, 3>::identity();
    let ad_matrix = match trivialization {
        So3Trivialization::LeftSpatial => identity - k * 0.5 + (k * k) * c,
        So3Trivialization::RightBody => identity + k * 0.5 + (k * k) * c,
    };
    let coordinate_map = so3_bivector_coordinate_map();
    let matrix = coordinate_map.transpose() * ad_matrix * coordinate_map;
    if !matrix.iter().all(|value| value.is_finite()) {
        return Err(So3LogError::NonFiniteResult);
    }
    Ok(So3Jacobian {
        matrix,
        trivialization,
        inverse: true,
        angle: theta,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, PI};

    fn matrix_error(left: &Rotor<3>, right: &Rotor<3>) -> f64 {
        left.to_matrix()
            .iter()
            .zip(right.to_matrix().iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0_f64, f64::max)
    }

    fn axis_generator(axis: SVector<f64, 3>, angle: f64) -> Bivector<3> {
        Bivector::from_matrix(&(-skew_from_axis(&axis.normalize()) * angle))
    }

    /// Independent closed-form Rodrigues construction used as an oracle input.
    /// It does not call Rotor::from_bivector or the logarithm implementation.
    fn rodrigues_rotation(axis: SVector<f64, 3>, angle: f64) -> Rotor<3> {
        let axis = axis.normalize();
        let cross = skew_from_axis(&axis);
        let identity = SMatrix::<f64, 3, 3>::identity();
        let matrix = identity * angle.cos()
            + axis * axis.transpose() * (1.0 - angle.cos())
            + cross * angle.sin();
        Rotor::from_matrix(matrix)
    }

    #[test]
    fn round_trips_identity_and_small_angles() {
        for angle in [0.0, 1.0e-12, 1.0e-8, 1.0e-4] {
            let generator = Bivector::<3>::unit_plane(0, 1).scale(angle);
            let rotation = Rotor::from_bivector(&generator);
            let recovered = so3_log(&rotation).expect("valid SO(3) rotation");
            let reconstructed = Rotor::from_bivector(&recovered.generator);
            assert!(matrix_error(&rotation, &reconstructed) < 2.0e-10);
        }
    }

    #[test]
    fn round_trips_generic_axis_and_angle() {
        let axis = SVector::<f64, 3>::new(1.0, -2.0, 3.0).normalize();
        let rotation = Rotor::from_bivector(&axis_generator(axis, 1.234));
        let recovered = so3_log(&rotation).expect("valid SO(3) rotation");
        assert_eq!(recovered.branch, So3LogBranch::Principal);
        let reconstructed = Rotor::from_bivector(&recovered.generator);
        assert!(matrix_error(&rotation, &reconstructed) < 2.0e-10);
    }


    #[test]
    fn log_reconstructs_independent_rodrigues_reference_including_cut_locus() {
        let axis = SVector::<f64, 3>::new(-1.0, 2.0, -3.0).normalize();
        for angle in [1.0e-8, 0.7, std::f64::consts::PI - 1.0e-8, PI] {
            let reference = rodrigues_rotation(axis, angle);
            let logarithm = so3_log(&reference).expect("Rodrigues matrix is a valid rotation");
            let reconstructed = Rotor::from_bivector(&logarithm.generator);
            assert!(
                matrix_error(&reference, &reconstructed) < 5.0e-10,
                "Rodrigues reconstruction failed at angle {angle}: branch={:?}",
                logarithm.branch
            );
        }
    }

    #[test]
    fn exact_pi_uses_and_reports_canonical_axis_branch() {
        let axis = SVector::<f64, 3>::new(1.0, 2.0, 3.0).normalize();
        let rotation = Rotor::from_bivector(&axis_generator(axis, PI));
        let recovered = so3_log(&rotation).expect("pi rotation has a canonical log");
        assert_eq!(recovered.branch, So3LogBranch::PiCanonical);
        assert!((recovered.angle - PI).abs() < 1.0e-10);
        let reconstructed = Rotor::from_bivector(&recovered.generator);
        assert!(matrix_error(&rotation, &reconstructed) < 2.0e-10);
    }

    #[test]
    fn near_pi_preserves_axis_sign_from_skew_part() {
        let axis = SVector::<f64, 3>::new(1.0, 2.0, -3.0).normalize();
        let rotation = Rotor::from_bivector(&axis_generator(axis, PI - 1.0e-9));
        let recovered = so3_log(&rotation).expect("near-pi rotation has a stable branch");
        assert_eq!(recovered.branch, So3LogBranch::NearPi);
        let reconstructed = Rotor::from_bivector(&recovered.generator);
        assert!(matrix_error(&rotation, &reconstructed) < 2.0e-10);
    }

    #[test]
    fn plus_minus_are_inverse_in_both_trivializations() {
        let base = Rotor::<3>::from_bivector(&Bivector::unit_plane(0, 2).scale(0.37));
        let mut delta = Bivector::<3>::unit_plane(0, 1).scale(0.08);
        delta = delta.add(&Bivector::<3>::unit_plane(1, 2).scale(-0.035));

        for trivialization in [
            So3Trivialization::LeftSpatial,
            So3Trivialization::RightBody,
        ] {
            let moved = so3_plus(&base, &delta, trivialization).expect("valid tangent update");
            let recovered = so3_minus(&base, &moved, trivialization)
                .expect("valid tangent difference");
            let expected = Rotor::<3>::from_bivector(&delta);
            let actual = Rotor::<3>::from_bivector(&recovered.generator);
            assert!(matrix_error(&expected, &actual) < 2.0e-10);
        }
    }

    #[test]
    fn interpolation_preserves_endpoints_and_follows_geodesic() {
        let start = Rotor::<3>::from_bivector(&Bivector::unit_plane(0, 2).scale(0.4));
        let end = Rotor::<3>::from_bivector(&Bivector::unit_plane(0, 1).scale(FRAC_PI_2));
        assert_eq!(matrix_error(&so3_interpolate(&start, &end, 0.0).unwrap(), &start), 0.0);
        assert_eq!(matrix_error(&so3_interpolate(&start, &end, 1.0).unwrap(), &end), 0.0);

        let midpoint = so3_interpolate(&start, &end, 0.5).expect("valid interpolation");
        assert!(midpoint.orthogonality_error() < 1.0e-12);
        assert!((midpoint.determinant() - 1.0).abs() < 1.0e-12);
        let relative = end.compose(&start.reverse());
        let midpoint_relative = midpoint.compose(&start.reverse());
        assert!(matrix_error(
            &midpoint_relative.compose(&midpoint_relative),
            &relative
        ) < 2.0e-10);
    }

    #[test]
    fn rejects_non_rotations_and_non_finite_parameters() {
        let invalid = Rotor::from_matrix(SMatrix::<f64, 3, 3>::identity() * 2.0);
        assert!(matches!(
            so3_log(&invalid),
            Err(So3LogError::InvalidRotation { .. })
        ));
        let identity = Rotor::<3>::identity();
        assert!(matches!(
            so3_interpolate(&identity, &identity, f64::NAN),
            Err(So3LogError::NonFiniteParameter)
        ));
    }

    #[test]
    fn rejects_invalid_configuration() {
        let invalid = So3LogConfig {
            rotation_tolerance: -1.0,
            ..So3LogConfig::default()
        };
        assert_eq!(
            so3_log_with_config(&Rotor::<3>::identity(), invalid),
            Err(So3LogError::InvalidConfig)
        );
    }
}


#[cfg(test)]
mod jacobian_tests {
    use super::*;
    use std::f64::consts::{PI, TAU};

    fn max_identity_error(matrix: &SMatrix<f64, 3, 3>) -> f64 {
        let identity = SMatrix::<f64, 3, 3>::identity();
        matrix.iter().zip(identity.iter())
            .map(|(a, b)| (a - b).abs())
            .fold(0.0_f64, f64::max)
    }

    fn central_difference(
        generator: &Bivector<3>,
        direction: &Bivector<3>,
        trivialization: So3Trivialization,
        step: f64,
    ) -> Bivector<3> {
        let plus = generator.add(&direction.scale(step));
        let minus = generator.add(&direction.scale(-step));
        let base_rotation = Rotor::<3>::from_bivector(generator);
        let plus_rotation = Rotor::<3>::from_bivector(&plus);
        let minus_rotation = Rotor::<3>::from_bivector(&minus);

        let plus_relative = match trivialization {
            So3Trivialization::LeftSpatial => plus_rotation.compose(&base_rotation.reverse()),
            So3Trivialization::RightBody => base_rotation.reverse().compose(&plus_rotation),
        };
        let minus_relative = match trivialization {
            So3Trivialization::LeftSpatial => minus_rotation.compose(&base_rotation.reverse()),
            So3Trivialization::RightBody => base_rotation.reverse().compose(&minus_rotation),
        };
        let plus_log = so3_log(&plus_relative).expect("positive perturbation is local");
        let minus_log = so3_log(&minus_relative).expect("negative perturbation is local");
        plus_log.generator.add(&minus_log.generator.scale(-1.0)).scale(0.5 / step)
    }

    #[test]
    fn jacobians_are_identity_at_zero_and_stable_for_small_angles() {
        let zero = Bivector::<3>::zero();
        for trivialization in [
            So3Trivialization::LeftSpatial,
            So3Trivialization::RightBody,
        ] {
            let forward = so3_exp_jacobian(&zero, trivialization).unwrap();
            let inverse = so3_exp_jacobian_inverse(&zero, trivialization).unwrap();
            assert!(max_identity_error(&forward.matrix) < 1.0e-15);
            assert!(max_identity_error(&inverse.matrix) < 1.0e-15);

            let tiny = Bivector::<3>::unit_plane(0, 2).scale(1.0e-8);
            let tiny_forward = so3_exp_jacobian(&tiny, trivialization).unwrap();
            let tiny_inverse = so3_exp_jacobian_inverse(&tiny, trivialization).unwrap();
            assert!(tiny_forward.matrix.iter().all(|v| v.is_finite()));
            assert!(tiny_inverse.matrix.iter().all(|v| v.is_finite()));
            assert!(max_identity_error(&(tiny_inverse.matrix * tiny_forward.matrix)) < 1.0e-8);
        }
    }

    #[test]
    fn left_and_right_jacobians_match_independent_central_differences() {
        let mut generator = Bivector::<3>::zero();
        generator.set(0, 1, 0.41);
        generator.set(0, 2, -0.27);
        generator.set(1, 2, 0.73);

        let mut direction = Bivector::<3>::zero();
        direction.set(0, 1, -0.32);
        direction.set(0, 2, 0.61);
        direction.set(1, 2, 0.19);

        for trivialization in [
            So3Trivialization::LeftSpatial,
            So3Trivialization::RightBody,
        ] {
            let jacobian = so3_exp_jacobian(&generator, trivialization).unwrap();
            let predicted = jacobian.apply(&direction).unwrap();
            let measured = central_difference(&generator, &direction, trivialization, 1.0e-6);
            assert!(
                predicted.add(&measured.scale(-1.0)).norm() < 2.0e-8,
                "finite-difference mismatch for {trivialization:?}: predicted={predicted:?}, measured={measured:?}"
            );
        }
    }

    #[test]
    fn inverse_jacobians_invert_forward_jacobians() {
        let mut generator = Bivector::<3>::zero();
        generator.set(0, 1, 0.9);
        generator.set(0, 2, -0.4);
        generator.set(1, 2, 1.1);

        for trivialization in [
            So3Trivialization::LeftSpatial,
            So3Trivialization::RightBody,
        ] {
            let forward = so3_exp_jacobian(&generator, trivialization).unwrap();
            let inverse = so3_exp_jacobian_inverse(&generator, trivialization).unwrap();
            assert!(max_identity_error(&(inverse.matrix * forward.matrix)) < 2.0e-12);
            assert!(max_identity_error(&(forward.matrix * inverse.matrix)) < 2.0e-12);
        }
    }

    #[test]
    fn inverse_jacobian_reports_singular_and_ill_conditioned_angles() {
        let singular = Bivector::<3>::unit_plane(0, 1).scale(TAU);
        assert!(matches!(
            so3_exp_jacobian_inverse(&singular, So3Trivialization::LeftSpatial),
            Err(So3LogError::SingularExponentialJacobian { .. })
        ));

        let ill_conditioned = Bivector::<3>::unit_plane(0, 1).scale(TAU - 1.0e-9);
        assert!(matches!(
            so3_exp_jacobian_inverse(&ill_conditioned, So3Trivialization::RightBody),
            Err(So3LogError::IllConditionedExponentialJacobian { .. })
        ));

        let cut_locus = Bivector::<3>::unit_plane(0, 1).scale(PI);
        assert!(so3_exp_jacobian_inverse(&cut_locus, So3Trivialization::LeftSpatial).is_ok());
    }

    #[test]
    fn jacobians_reject_non_finite_tangents() {
        let mut invalid = Bivector::<3>::zero();
        invalid.set(0, 1, f64::NAN);
        assert!(matches!(
            so3_exp_jacobian(&invalid, So3Trivialization::LeftSpatial),
            Err(So3LogError::NonFiniteTangent)
        ));
        assert!(matches!(
            so3_exp_jacobian_inverse(&invalid, So3Trivialization::RightBody),
            Err(So3LogError::NonFiniteTangent)
        ));
    }
}

