// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Adapter from Symtropy's native 3D world into the shared observation schema.
//!
//! Semantic identities are caller-supplied. The adapter rejects incomplete mappings
//! instead of treating native body handles as cross-engine identities.

use std::collections::HashMap;
use std::fmt;

use nalgebra::{SMatrix, UnitQuaternion};
use symtropy_physics_diff_core::{
    BodyObservation, ObservationError, ObservationFrame, RunIdentity,
};

use crate::body::BodyHandle;
use crate::world::PhysicsWorld;

/// Errors encountered while converting a native world into a common observation frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeObservationError {
    /// A live body has no declared scenario-level semantic identity.
    MissingSemanticBodyId { handle: BodyHandle },
    /// The source rotor is not a finite, proper 3D rotation matrix.
    InvalidRotation { handle: BodyHandle },
    /// The frame failed common-schema validation.
    InvalidObservation(ObservationError),
}

impl fmt::Display for NativeObservationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSemanticBodyId { handle } => {
                write!(f, "native body {handle:?} has no semantic identity mapping")
            }
            Self::InvalidRotation { handle } => {
                write!(f, "native body {handle:?} has an invalid rotation matrix")
            }
            Self::InvalidObservation(error) => {
                write!(f, "native observation is invalid: {error}")
            }
        }
    }
}

impl std::error::Error for NativeObservationError {}

impl From<ObservationError> for NativeObservationError {
    fn from(value: ObservationError) -> Self {
        Self::InvalidObservation(value)
    }
}

impl PhysicsWorld<3> {
    /// Capture a common-schema frame from the native 3D solver.
    ///
    /// semantic_body_ids must be built from the declared scenario mapping. No fallback
    /// to a native handle value, insertion order, or backend-specific ID is performed.
    /// Contact and energy values remain absent until their accounting conventions are
    /// qualified against the reference backend; an unavailable quantity is not zero.
    pub fn to_observation_frame(
        &self,
        identity: RunIdentity,
        step_index: u64,
        simulation_time_seconds: f64,
        semantic_body_ids: &HashMap<BodyHandle, String>,
    ) -> Result<ObservationFrame, NativeObservationError> {
        let mut frame =
            ObservationFrame::new(identity, step_index, simulation_time_seconds);

        for body in &self.bodies {
            let semantic_body_id = semantic_body_ids
                .get(&body.handle)
                .ok_or(NativeObservationError::MissingSemanticBodyId {
                    handle: body.handle,
                })?;

            if !body.transform.rotation.is_proper_rotation(1.0e-6) {
                return Err(NativeObservationError::InvalidRotation {
                    handle: body.handle,
                });
            }

            let rotation_matrix = body.transform.rotation.to_matrix();
            let rotation_matrix = SMatrix::<f64, 3, 3>::from_fn(|row, column| {
                rotation_matrix[(row, column)]
            });
            let rotation = UnitQuaternion::from_matrix(&rotation_matrix);
            let quaternion = rotation.quaternion();
            let angular = body.angular_velocity;

            // Rotor plane components are dual to conventional XYZ angular velocity:
            // (e12, -e02, e01). Keep this convention explicit at the adapter boundary.
            let angular_velocity_radps = [
                angular.get(1, 2),
                -angular.get(0, 2),
                angular.get(0, 1),
            ];
            let translation = body.transform.translation.0;
            let mut observation = BodyObservation::new(
                semantic_body_id.clone(),
                [translation[0], translation[1], translation[2]],
                [quaternion.w, quaternion.i, quaternion.j, quaternion.k],
                [
                    body.linear_velocity[0],
                    body.linear_velocity[1],
                    body.linear_velocity[2],
                ],
                angular_velocity_radps,
            );
            observation.sleeping = Some(body.sleeping);
            frame.bodies.push(observation);
        }

        frame.validate()?;
        Ok(frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::SVector;
    use std::f64::consts::FRAC_PI_2;
    use symtropy_math::Point;
    use symtropy_math::{Bivector, Rotor};

    fn identity() -> RunIdentity {
        RunIdentity::new(
            "pose-fixture",
            "sha256:pose-fixture",
            "symtropy-native",
            "test-build",
            "default",
            "sha256:config",
            1.0 / 60.0,
        )
    }

    #[test]
    fn native_adapter_preserves_semantic_identity_pose_and_angular_convention() {
        let mut world = PhysicsWorld::<3>::new(SVector::new(0.0, -9.81, 0.0));
        let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
        let body = world.body_mut(handle).expect("body exists");
        body.transform.translation = Point::new([1.0, 2.0, 3.0]);
        body.transform.rotation =
            Rotor::from_plane_angle(&Bivector::<3>::unit_plane(0, 1), FRAC_PI_2);
        body.linear_velocity = SVector::new(1.0, -2.0, 0.5);
        body.angular_velocity.set(1, 2, 3.0);
        body.angular_velocity.set(0, 2, 4.0);
        body.angular_velocity.set(0, 1, 5.0);
        body.sleeping = true;

        let semantic_body_ids =
            HashMap::from([(handle, "scenario/ball-a".to_owned())]);
        let frame = world
            .to_observation_frame(identity(), 7, 7.0 / 60.0, &semantic_body_ids)
            .expect("valid mapped frame");
        assert_eq!(frame.validate(), Ok(()));
        assert_eq!(frame.bodies.len(), 1);

        let body = &frame.bodies[0];
        assert_eq!(body.semantic_body_id, "scenario/ball-a");
        assert_eq!(body.position_m, [1.0, 2.0, 3.0]);
        assert!((body.orientation_wxyz[0] - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-8);
        assert!((body.orientation_wxyz[3] - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-8);
        assert_eq!(body.linear_velocity_mps, [1.0, -2.0, 0.5]);
        assert_eq!(body.angular_velocity_radps, [3.0, -4.0, 5.0]);
        assert_eq!(body.sleeping, Some(true));
        assert_eq!(body.contact_count, None);
        assert_eq!(frame.energy.kinetic_energy_j, None);
    }

    #[test]
    fn native_adapter_rejects_missing_semantic_identity() {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
        let error = world
            .to_observation_frame(identity(), 0, 0.0, &HashMap::new())
            .expect_err("missing scenario identity must not fall back to engine handle");
        assert_eq!(
            error,
            NativeObservationError::MissingSemanticBodyId { handle }
        );
    }

    #[test]
    fn native_adapter_rejects_improper_rotation_matrices() {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
        let body = world.body_mut(handle).expect("body exists");
        body.transform.rotation = Rotor::from_matrix(SMatrix::from_diagonal(&SVector::new(
            1.0, 1.0, -1.0,
        )));
        let ids = HashMap::from([(handle, "scenario/ball-a".to_owned())]);

        assert_eq!(
            world.to_observation_frame(identity(), 0, 0.0, &ids),
            Err(NativeObservationError::InvalidRotation { handle })
        );
    }
}
