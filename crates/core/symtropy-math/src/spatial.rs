// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Frame-explicit SE(3) spatial algebra for rigid-body motion and force.
//!
//! A Transform<3> maps body/local coordinates into spatial/world coordinates:
//! p_world = R * p_body + translation. Twists use the [linear, angular]
//! ordering, with angular coordinates represented by Symtropy's bivector
//! basis [e01, e02, e12]. The spatial linear component is the translational
//! se(3) coordinate (p_dot - omega cross p), not simply p_dot. Wrenches use
//! the dual [force, torque] ordering, with torque taken about each frame origin.
//!
//! The body-to-spatial twist map is Ad_T. The matching body-to-spatial wrench
//! map is Ad_T^(-T), so their dual power pairing is invariant. These typed
//! operations do not create or replace the repository's global frame registry;
//! callers remain responsible for binding the transform to the correct frame
//! identities and point of application.

use crate::lie::{so3_log, So3LogError};
use crate::{Bivector, Transform};
use nalgebra::{SMatrix, SVector};
use std::error::Error;
use std::fmt;

/// Errors from checked SE(3) spatial transformations.
#[derive(Clone, Debug, PartialEq)]
pub enum SpatialAlgebraError {
    InvalidRotation(So3LogError),
    NonFiniteTransformTranslation,
    NonFiniteTwist,
    NonFiniteWrench,
    NonFiniteResult,
}

impl fmt::Display for SpatialAlgebraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRotation(error) => write!(f, "invalid SE(3) rotation: {error}"),
            Self::NonFiniteTransformTranslation => {
                write!(f, "SE(3) translation contains non-finite values")
            }
            Self::NonFiniteTwist => write!(f, "twist contains non-finite values"),
            Self::NonFiniteWrench => write!(f, "wrench contains non-finite values"),
            Self::NonFiniteResult => write!(f, "SE(3) spatial transform produced non-finite values"),
        }
    }
}

impl Error for SpatialAlgebraError {}

impl From<So3LogError> for SpatialAlgebraError {
    fn from(value: So3LogError) -> Self {
        Self::InvalidRotation(value)
    }
}

/// Body-frame twist, ordered as linear then angular velocity.
///
/// The angular bivector is expressed in the body coordinate basis. Its
/// components are radians per second in the plane ordering [e01, e02, e12].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyTwist3 {
    pub linear: SVector<f64, 3>,
    pub angular: Bivector<3>,
}

impl BodyTwist3 {
    pub fn new(linear: SVector<f64, 3>, angular: Bivector<3>) -> Self {
        Self { linear, angular }
    }

    pub fn is_finite(&self) -> bool {
        self.linear.iter().all(|value| value.is_finite()) && self.angular.is_finite()
    }

    /// Instantaneous power paired with a body-frame wrench.
    pub fn power_with(&self, wrench: &BodyWrench3) -> f64 {
        self.linear.dot(&wrench.force) + bivector_pair(&self.angular, &wrench.torque)
    }
}

/// Spatial/world-frame twist, ordered as linear then angular velocity.
/// The linear part is the spatial se(3) coordinate, not the point velocity p_dot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpatialTwist3 {
    pub linear: SVector<f64, 3>,
    pub angular: Bivector<3>,
}

impl SpatialTwist3 {
    pub fn new(linear: SVector<f64, 3>, angular: Bivector<3>) -> Self {
        Self { linear, angular }
    }

    pub fn is_finite(&self) -> bool {
        self.linear.iter().all(|value| value.is_finite()) && self.angular.is_finite()
    }

    /// Instantaneous power paired with a spatial/world-frame wrench.
    pub fn power_with(&self, wrench: &SpatialWrench3) -> f64 {
        self.linear.dot(&wrench.force) + bivector_pair(&self.angular, &wrench.torque)
    }
}

/// Body-frame wrench, ordered as force then torque, with moment about the body origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyWrench3 {
    pub force: SVector<f64, 3>,
    pub torque: Bivector<3>,
}

impl BodyWrench3 {
    pub fn new(force: SVector<f64, 3>, torque: Bivector<3>) -> Self {
        Self { force, torque }
    }

    pub fn is_finite(&self) -> bool {
        self.force.iter().all(|value| value.is_finite()) && self.torque.is_finite()
    }
}

/// Spatial/world-frame wrench, ordered as force then torque, with moment about the world origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpatialWrench3 {
    pub force: SVector<f64, 3>,
    pub torque: Bivector<3>,
}

impl SpatialWrench3 {
    pub fn new(force: SVector<f64, 3>, torque: Bivector<3>) -> Self {
        Self { force, torque }
    }

    pub fn is_finite(&self) -> bool {
        self.force.iter().all(|value| value.is_finite()) && self.torque.is_finite()
    }
}

fn cross3(left: &SVector<f64, 3>, right: &SVector<f64, 3>) -> SVector<f64, 3> {
    SVector::new(
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    )
}

fn cross_matrix(vector: &SVector<f64, 3>) -> SMatrix<f64, 3, 3> {
    let mut matrix = SMatrix::<f64, 3, 3>::zeros();
    matrix[(0, 1)] = -vector[2];
    matrix[(0, 2)] = vector[1];
    matrix[(1, 0)] = vector[2];
    matrix[(1, 2)] = -vector[0];
    matrix[(2, 0)] = -vector[1];
    matrix[(2, 1)] = vector[0];
    matrix
}

fn bivector_to_angular_vector(angular: &Bivector<3>) -> SVector<f64, 3> {
    SVector::new(
        angular.get(1, 2),
        -angular.get(0, 2),
        angular.get(0, 1),
    )
}

fn angular_vector_to_bivector(angular: &SVector<f64, 3>) -> Bivector<3> {
    let mut result = Bivector::<3>::zero();
    result.set(0, 1, angular[2]);
    result.set(0, 2, -angular[1]);
    result.set(1, 2, angular[0]);
    result
}

fn bivector_pair(left: &Bivector<3>, right: &Bivector<3>) -> f64 {
    left.get(0, 1) * right.get(0, 1)
        + left.get(0, 2) * right.get(0, 2)
        + left.get(1, 2) * right.get(1, 2)
}

fn angular_coordinate_map() -> SMatrix<f64, 3, 3> {
    // [wx, wy, wz] = C * [b01, b02, b12].
    let mut c = SMatrix::<f64, 3, 3>::zeros();
    c[(0, 2)] = 1.0;
    c[(1, 1)] = -1.0;
    c[(2, 0)] = 1.0;
    c
}

fn validate_transform(transform: &Transform<3>) -> Result<(), SpatialAlgebraError> {
    if !transform.translation.0.iter().all(|value| value.is_finite()) {
        return Err(SpatialAlgebraError::NonFiniteTransformTranslation);
    }
    so3_log(&transform.rotation)?;
    Ok(())
}

impl Transform<3> {
    /// Return the 6x6 body-to-spatial twist adjoint matrix.
    ///
    /// Coordinates are [linear, angular-bivector], with the angular block
    /// ordered [e01, e02, e12]. This is Ad_T for T mapping body to world.
    pub fn body_to_spatial_adjoint_matrix(
        &self,
    ) -> Result<SMatrix<f64, 6, 6>, SpatialAlgebraError> {
        validate_transform(self)?;
        let rotation = *self.rotation.to_matrix();
        let translation_cross = cross_matrix(&self.translation.0);
        let coordinate_map = angular_coordinate_map();
        let angular_block = coordinate_map.transpose() * rotation * coordinate_map;
        let coupling_block = translation_cross * rotation * coordinate_map;

        let mut matrix = SMatrix::<f64, 6, 6>::zeros();
        for row in 0..3 {
            for column in 0..3 {
                matrix[(row, column)] = rotation[(row, column)];
                matrix[(row, column + 3)] = coupling_block[(row, column)];
                matrix[(row + 3, column + 3)] = angular_block[(row, column)];
            }
        }
        if matrix.iter().all(|value| value.is_finite()) {
            Ok(matrix)
        } else {
            Err(SpatialAlgebraError::NonFiniteResult)
        }
    }

    /// Return the 6x6 body-to-spatial wrench coadjoint matrix Ad_T^(-T).
    ///
    /// This is the dual of body-to-spatial twist transport and preserves
    /// instantaneous power under the [linear, angular] / [force, torque]
    /// pairing.
    pub fn body_to_spatial_coadjoint_matrix(
        &self,
    ) -> Result<SMatrix<f64, 6, 6>, SpatialAlgebraError> {
        validate_transform(self)?;
        let rotation = *self.rotation.to_matrix();
        let translation_cross = cross_matrix(&self.translation.0);
        let coordinate_map = angular_coordinate_map();
        let angular_block = coordinate_map.transpose() * rotation * coordinate_map;
        let torque_force_block =
            coordinate_map.transpose() * translation_cross * rotation;

        let mut matrix = SMatrix::<f64, 6, 6>::zeros();
        for row in 0..3 {
            for column in 0..3 {
                matrix[(row, column)] = rotation[(row, column)];
                matrix[(row + 3, column)] = torque_force_block[(row, column)];
                matrix[(row + 3, column + 3)] = angular_block[(row, column)];
            }
        }
        if matrix.iter().all(|value| value.is_finite()) {
            Ok(matrix)
        } else {
            Err(SpatialAlgebraError::NonFiniteResult)
        }
    }

    /// Transform a body-frame twist into spatial/world coordinates using Ad_T.
    pub fn body_twist_to_spatial(
        &self,
        twist: &BodyTwist3,
    ) -> Result<SpatialTwist3, SpatialAlgebraError> {
        validate_transform(self)?;
        if !twist.is_finite() {
            return Err(SpatialAlgebraError::NonFiniteTwist);
        }
        let angular_body = bivector_to_angular_vector(&twist.angular);
        let angular_spatial = self.rotation.rotate_vector(&angular_body);
        let linear_spatial = self.rotation.rotate_vector(&twist.linear)
            + cross3(&self.translation.0, &angular_spatial);
        let result = SpatialTwist3::new(
            linear_spatial,
            angular_vector_to_bivector(&angular_spatial),
        );
        if result.is_finite() {
            Ok(result)
        } else {
            Err(SpatialAlgebraError::NonFiniteResult)
        }
    }

    /// Transform a spatial/world-frame twist into body coordinates using Ad_T^-1.
    pub fn spatial_twist_to_body(
        &self,
        twist: &SpatialTwist3,
    ) -> Result<BodyTwist3, SpatialAlgebraError> {
        validate_transform(self)?;
        if !twist.is_finite() {
            return Err(SpatialAlgebraError::NonFiniteTwist);
        }
        let angular_spatial = bivector_to_angular_vector(&twist.angular);
        let angular_body = self.rotation.reverse().rotate_vector(&angular_spatial);
        let shifted_linear = twist.linear - cross3(&self.translation.0, &angular_spatial);
        let linear_body = self.rotation.reverse().rotate_vector(&shifted_linear);
        let result = BodyTwist3::new(linear_body, angular_vector_to_bivector(&angular_body));
        if result.is_finite() {
            Ok(result)
        } else {
            Err(SpatialAlgebraError::NonFiniteResult)
        }
    }

    /// Transform a body-frame wrench into spatial coordinates using Ad_T^(-T).
    pub fn body_wrench_to_spatial(
        &self,
        wrench: &BodyWrench3,
    ) -> Result<SpatialWrench3, SpatialAlgebraError> {
        validate_transform(self)?;
        if !wrench.is_finite() {
            return Err(SpatialAlgebraError::NonFiniteWrench);
        }
        let force_spatial = self.rotation.rotate_vector(&wrench.force);
        let torque_body = bivector_to_angular_vector(&wrench.torque);
        let torque_spatial = cross3(&self.translation.0, &force_spatial)
            + self.rotation.rotate_vector(&torque_body);
        let result =
            SpatialWrench3::new(force_spatial, angular_vector_to_bivector(&torque_spatial));
        if result.is_finite() {
            Ok(result)
        } else {
            Err(SpatialAlgebraError::NonFiniteResult)
        }
    }

    /// Transform a spatial/world-frame wrench into body coordinates.
    pub fn spatial_wrench_to_body(
        &self,
        wrench: &SpatialWrench3,
    ) -> Result<BodyWrench3, SpatialAlgebraError> {
        validate_transform(self)?;
        if !wrench.is_finite() {
            return Err(SpatialAlgebraError::NonFiniteWrench);
        }
        let force_body = self.rotation.reverse().rotate_vector(&wrench.force);
        let shifted_torque = bivector_to_angular_vector(&wrench.torque)
            - cross3(&self.translation.0, &wrench.force);
        let torque_body = self.rotation.reverse().rotate_vector(&shifted_torque);
        let result = BodyWrench3::new(force_body, angular_vector_to_bivector(&torque_body));
        if result.is_finite() {
            Ok(result)
        } else {
            Err(SpatialAlgebraError::NonFiniteResult)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Point, Rotor};
    use std::f64::consts::FRAC_PI_2;

    fn fixture() -> Transform<3> {
        let mut generator = Bivector::<3>::zero();
        generator.set(0, 1, 0.31);
        generator.set(0, 2, -0.22);
        generator.set(1, 2, 0.47);
        Transform {
            translation: Point::new([1.3, -0.7, 2.1]),
            rotation: Rotor::from_bivector(&generator),
        }
    }

    fn assert_bivector_close(left: &Bivector<3>, right: &Bivector<3>, tolerance: f64) {
        assert!(left.add(&right.scale(-1.0)).norm() < tolerance);
    }

    #[test]
    fn body_to_spatial_twist_and_wrench_preserve_power() {
        let transform = fixture();
        let mut angular = Bivector::<3>::zero();
        angular.set(0, 1, 0.4);
        angular.set(0, 2, -0.6);
        angular.set(1, 2, 0.2);
        let twist = BodyTwist3::new(SVector::new(0.3, -1.1, 0.7), angular);

        let mut torque = Bivector::<3>::zero();
        torque.set(0, 1, -0.5);
        torque.set(0, 2, 0.9);
        torque.set(1, 2, 1.2);
        let wrench = BodyWrench3::new(SVector::new(4.0, 1.5, -2.0), torque);

        let spatial_twist = transform.body_twist_to_spatial(&twist).unwrap();
        let spatial_wrench = transform.body_wrench_to_spatial(&wrench).unwrap();
        let body_power = twist.power_with(&wrench);
        let spatial_power = spatial_twist.power_with(&spatial_wrench);
        assert!((body_power - spatial_power).abs() < 1.0e-12);
    }

    #[test]
    fn body_spatial_conversions_round_trip() {
        let transform = fixture();
        let mut angular = Bivector::<3>::zero();
        angular.set(0, 1, -0.7);
        angular.set(0, 2, 0.13);
        angular.set(1, 2, 0.52);
        let twist = BodyTwist3::new(SVector::new(-0.4, 0.2, 1.7), angular);
        let recovered_twist = transform
            .spatial_twist_to_body(&transform.body_twist_to_spatial(&twist).unwrap())
            .unwrap();
        assert!((twist.linear - recovered_twist.linear).norm() < 1.0e-12);
        assert_bivector_close(&twist.angular, &recovered_twist.angular, 1.0e-12);

        let mut torque = Bivector::<3>::zero();
        torque.set(0, 1, 0.5);
        torque.set(0, 2, -0.11);
        torque.set(1, 2, 0.8);
        let wrench = BodyWrench3::new(SVector::new(2.0, -3.0, 0.4), torque);
        let recovered_wrench = transform
            .spatial_wrench_to_body(&transform.body_wrench_to_spatial(&wrench).unwrap())
            .unwrap();
        assert!((wrench.force - recovered_wrench.force).norm() < 1.0e-12);
        assert_bivector_close(&wrench.torque, &recovered_wrench.torque, 1.0e-12);
    }

    #[test]
    fn adjoint_and_coadjoint_matrices_are_dual() {
        let transform = fixture();
        let adjoint = transform.body_to_spatial_adjoint_matrix().unwrap();
        let coadjoint = transform.body_to_spatial_coadjoint_matrix().unwrap();
        let identity = SMatrix::<f64, 6, 6>::identity();
        assert!((adjoint.transpose() * coadjoint - identity).norm() < 2.0e-12);
    }

    #[test]
    fn pure_translation_produces_expected_spatial_coupling() {
        let transform = Transform::<3>::from_translation(Point::new([1.0, 0.0, 0.0]));
        let mut angular = Bivector::<3>::zero();
        // omega_z = 1 -> spatial linear twist includes p cross omega = -y.
        angular.set(0, 1, 1.0);
        let twist = BodyTwist3::new(SVector::zeros(), angular);
        let spatial = transform.body_twist_to_spatial(&twist).unwrap();
        assert!(spatial.linear[0].abs() < 1.0e-12);
        assert!((spatial.linear[1] + 1.0).abs() < 1.0e-12);
        assert_bivector_close(&spatial.angular, &angular, 1.0e-12);

        // A pure x-force translated to x=1 has no induced moment.
        let wrench = BodyWrench3::new(SVector::new(1.0, 0.0, 0.0), Bivector::zero());
        let spatial_wrench = transform.body_wrench_to_spatial(&wrench).unwrap();
        assert_bivector_close(&spatial_wrench.torque, &Bivector::zero(), 1.0e-12);
    }

    #[test]
    fn invalid_pose_or_non_finite_inputs_are_rejected() {
        let mut invalid_rotation = Rotor::<3>::identity();
        invalid_rotation = Rotor::from_matrix(*invalid_rotation.to_matrix() * 2.0);
        let invalid_pose = Transform {
            translation: Point::origin(),
            rotation: invalid_rotation,
        };
        let twist = BodyTwist3::new(SVector::zeros(), Bivector::zero());
        assert!(matches!(
            invalid_pose.body_twist_to_spatial(&twist),
            Err(SpatialAlgebraError::InvalidRotation(_))
        ));

        let valid_pose = fixture();
        let bad_twist = BodyTwist3::new(SVector::new(f64::NAN, 0.0, 0.0), Bivector::zero());
        assert_eq!(
            valid_pose.body_twist_to_spatial(&bad_twist),
            Err(SpatialAlgebraError::NonFiniteTwist)
        );
        let bad_wrench =
            BodyWrench3::new(SVector::new(0.0, f64::INFINITY, 0.0), Bivector::zero());
        assert_eq!(
            valid_pose.body_wrench_to_spatial(&bad_wrench),
            Err(SpatialAlgebraError::NonFiniteWrench)
        );
    }

    #[test]
    fn rotation_and_translation_together_are_not_assumed_commutative() {
        let transform = fixture();
        let mut angular = Bivector::<3>::zero();
        angular.set(0, 1, 0.3);
        let twist = BodyTwist3::new(SVector::new(1.0, 0.0, 0.0), angular);
        let spatial = transform.body_twist_to_spatial(&twist).unwrap();
        let back = transform.spatial_twist_to_body(&spatial).unwrap();
        assert!((back.linear - twist.linear).norm() < 1.0e-12);
        assert_bivector_close(&back.angular, &twist.angular, 1.0e-12);

        let quarter_turn = Transform {
            translation: Point::new([0.0, 1.0, 0.0]),
            rotation: Rotor::from_plane_angle(
                &Bivector::<3>::unit_plane(0, 1),
                FRAC_PI_2,
            ),
        };
        let quarter_spatial = quarter_turn.body_twist_to_spatial(&twist).unwrap();
        assert!(quarter_spatial.is_finite());
        assert!((quarter_spatial.linear[0] - 1.0).abs() < 1.0e-12);
        assert!((quarter_spatial.linear[1] - 1.0).abs() < 1.0e-12);
    }
}



#[cfg(test)]
mod deterministic_sweep_tests {
    use super::*;
    use crate::{Point, Rotor};

    fn next_signed(state: &mut u64) -> f64 {
        *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = *state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^= value >> 31;
        (value as f64 / u64::MAX as f64) * 2.0 - 1.0
    }

    fn next_bivector(state: &mut u64, scale: f64) -> Bivector<3> {
        let mut result = Bivector::<3>::zero();
        result.set(0, 1, next_signed(state) * scale);
        result.set(0, 2, next_signed(state) * scale);
        result.set(1, 2, next_signed(state) * scale);
        result
    }

    fn six_vector(linear: &SVector<f64, 3>, angular: &Bivector<3>) -> SVector<f64, 6> {
        SVector::new(
            linear[0],
            linear[1],
            linear[2],
            angular.get(0, 1),
            angular.get(0, 2),
            angular.get(1, 2),
        )
    }

    #[test]
    fn deterministic_scale_sweep_preserves_dual_power_and_inverse_maps() {
        let mut state = 0x5345_335F_4144_4A4F_u64;
        let length_scales = [1.0e-6, 1.0, 1.0e6];

        for case in 0..384 {
            let length_scale = length_scales[case % length_scales.len()];
            let translation = SVector::<f64, 3>::new(
                next_signed(&mut state) * length_scale,
                next_signed(&mut state) * length_scale,
                next_signed(&mut state) * length_scale,
            );
            let rotation = Rotor::<3>::from_bivector(&next_bivector(&mut state, 0.45));
            let transform = Transform {
                translation: Point(translation),
                rotation,
            };

            let body_twist = BodyTwist3::new(
                SVector::new(
                    next_signed(&mut state),
                    next_signed(&mut state),
                    next_signed(&mut state),
                ),
                next_bivector(&mut state, 0.8),
            );
            let body_wrench = BodyWrench3::new(
                SVector::new(
                    next_signed(&mut state),
                    next_signed(&mut state),
                    next_signed(&mut state),
                ),
                next_bivector(&mut state, length_scale),
            );

            let spatial_twist = transform.body_twist_to_spatial(&body_twist).unwrap();
            let spatial_wrench = transform.body_wrench_to_spatial(&body_wrench).unwrap();
            let body_power = body_twist.power_with(&body_wrench);
            let spatial_power = spatial_twist.power_with(&spatial_wrench);
            let power_tolerance = 2.0e-11 * (1.0 + body_power.abs() + spatial_power.abs());
            assert!(
                (body_power - spatial_power).abs() <= power_tolerance,
                "power duality failed: case={case}, scale={length_scale:e}, body={body_power:e}, spatial={spatial_power:e}"
            );

            let recovered_twist = transform.spatial_twist_to_body(&spatial_twist).unwrap();
            let twist_tolerance = 2.0e-9 * (1.0 + body_twist.linear.norm());
            assert!(
                (body_twist.linear - recovered_twist.linear).norm() <= twist_tolerance,
                "twist round trip failed: case={case}, scale={length_scale:e}"
            );
            assert!(
                body_twist
                    .angular
                    .add(&recovered_twist.angular.scale(-1.0))
                    .norm()
                    <= 2.0e-10
            );

            let recovered_wrench = transform.spatial_wrench_to_body(&spatial_wrench).unwrap();
            assert!(
                (body_wrench.force - recovered_wrench.force).norm() <= 2.0e-10,
                "force round trip failed: case={case}"
            );
            let torque_tolerance = 2.0e-9 * (1.0 + body_wrench.torque.norm());
            assert!(
                body_wrench
                    .torque
                    .add(&recovered_wrench.torque.scale(-1.0))
                    .norm()
                    <= torque_tolerance,
                "torque round trip failed: case={case}, scale={length_scale:e}"
            );
        }
    }

    #[test]
    fn six_by_six_matrices_match_typed_adjoint_and_coadjoint_actions() {
        let transform = Transform {
            translation: Point::new([0.7, -1.2, 0.4]),
            rotation: Rotor::from_bivector(&{
                let mut value = Bivector::<3>::zero();
                value.set(0, 1, 0.21);
                value.set(0, 2, -0.33);
                value.set(1, 2, 0.49);
                value
            }),
        };
        let mut angular = Bivector::<3>::zero();
        angular.set(0, 1, -0.6);
        angular.set(0, 2, 0.2);
        angular.set(1, 2, 0.7);
        let twist = BodyTwist3::new(SVector::new(0.2, -0.8, 1.1), angular);
        let mut torque = Bivector::<3>::zero();
        torque.set(0, 1, 0.1);
        torque.set(0, 2, -0.4);
        torque.set(1, 2, 0.9);
        let wrench = BodyWrench3::new(SVector::new(1.2, 0.4, -0.5), torque);

        let adjoint_action = transform.body_to_spatial_adjoint_matrix().unwrap()
            * six_vector(&twist.linear, &twist.angular);
        let typed_twist = transform.body_twist_to_spatial(&twist).unwrap();
        assert!((adjoint_action - six_vector(&typed_twist.linear, &typed_twist.angular)).norm() < 1.0e-12);

        let coadjoint_action = transform.body_to_spatial_coadjoint_matrix().unwrap()
            * six_vector(&wrench.force, &wrench.torque);
        let typed_wrench = transform.body_wrench_to_spatial(&wrench).unwrap();
        assert!((coadjoint_action - six_vector(&typed_wrench.force, &typed_wrench.torque)).norm() < 1.0e-12);
    }
}

