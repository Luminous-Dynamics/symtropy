// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Checked 3D kinetic-energy views over live [`PhysicsWorld`] state.
//!
//! The production world's historical `total_kinetic_energy()` still uses the
//! generic mean-inertia body metric. This module provides an additive validation
//! path that measures each dynamic [`RigidBody`](crate::body::RigidBody) through
//! `kinetic_energy_3d_checked()`, orders evidence canonically by `BodyHandle`,
//! rejects duplicate identity, and rejects aggregate overflow.

use crate::body::{BodyHandle, RigidBodyEnergyError};
use crate::world::PhysicsWorld;

/// One canonical per-body kinetic-energy measurement.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BodyKineticEnergy3 {
    pub handle: BodyHandle,
    pub joules: f64,
}

/// Failures that make a world-level 3D kinetic-energy view unsuitable as
/// validation evidence.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WorldEnergy3dError {
    DuplicateBodyHandle(BodyHandle),
    Body {
        handle: BodyHandle,
        source: RigidBodyEnergyError,
    },
    UnrepresentableTotal,
}

/// Checked, non-invasive 3D energy measurement for [`PhysicsWorld<3>`].
pub trait PhysicsWorldEnergy3dExt {
    /// Dynamic-body kinetic energies sorted by stable body handle.
    fn body_kinetic_energies_3d_checked(
        &self,
    ) -> Result<Vec<BodyKineticEnergy3>, WorldEnergy3dError>;

    /// Canonical handle-ordered sum of exact represented 3D kinetic energy.
    fn total_kinetic_energy_3d_checked(&self) -> Result<f64, WorldEnergy3dError>;
}

impl PhysicsWorldEnergy3dExt for PhysicsWorld<3> {
    fn body_kinetic_energies_3d_checked(
        &self,
    ) -> Result<Vec<BodyKineticEnergy3>, WorldEnergy3dError> {
        let mut bodies: Vec<_> = self
            .bodies
            .iter()
            .filter(|body| body.is_dynamic())
            .collect();
        bodies.sort_by_key(|body| body.handle);

        for pair in bodies.windows(2) {
            if pair[0].handle == pair[1].handle {
                return Err(WorldEnergy3dError::DuplicateBodyHandle(pair[0].handle));
            }
        }

        let mut measurements = Vec::with_capacity(bodies.len());
        for body in bodies {
            let joules = body
                .kinetic_energy_3d_checked()
                .map_err(|source| WorldEnergy3dError::Body {
                    handle: body.handle,
                    source,
                })?;
            measurements.push(BodyKineticEnergy3 {
                handle: body.handle,
                joules,
            });
        }
        Ok(measurements)
    }

    fn total_kinetic_energy_3d_checked(&self) -> Result<f64, WorldEnergy3dError> {
        let measurements = self.body_kinetic_energies_3d_checked()?;
        let mut total = 0.0_f64;
        for measurement in measurements {
            let next = total + measurement.joules;
            if !next.is_finite() {
                return Err(WorldEnergy3dError::UnrepresentableTotal);
            }
            total = next;
        }
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::angular_dynamics::angular_vector_to_bivector;
    use crate::body::{BodyType, RigidBody};
    use nalgebra::SVector;
    use symtropy_math::{Point, Sphere, Transform};

    fn body(handle: usize, mass: f64, speed_x: f64) -> RigidBody<3> {
        let mut body = RigidBody::new(
            BodyHandle(handle),
            BodyType::Dynamic,
            Transform::<3>::identity(),
            Box::new(Sphere::new(Point::origin(), 1.0)),
            mass,
            SVector::from([1.0, 1.0, 1.0]),
        );
        body.linear_velocity = SVector::from([speed_x, 0.0, 0.0]);
        body.angular_velocity = angular_vector_to_bivector(&SVector::zeros());
        body
    }

    #[test]
    fn measurements_are_canonical_by_handle_not_storage_order() {
        let big = body(2, 2.0, 1.0e8); // 1e16 J
        let small_a = body(0, 2.0, 1.0); // 1 J
        let small_b = body(1, 2.0, 1.0); // 1 J

        let mut world_a = PhysicsWorld::<3>::new(SVector::zeros());
        world_a.bodies = vec![big, small_a, small_b];

        let mut world_b = PhysicsWorld::<3>::new(SVector::zeros());
        world_b.bodies = vec![
            body(0, 2.0, 1.0),
            body(1, 2.0, 1.0),
            body(2, 2.0, 1.0e8),
        ];

        let a = world_a.body_kinetic_energies_3d_checked().unwrap();
        let b = world_b.body_kinetic_energies_3d_checked().unwrap();
        assert_eq!(a, b);
        assert_eq!(
            a.iter().map(|measurement| measurement.handle).collect::<Vec<_>>(),
            vec![BodyHandle(0), BodyHandle(1), BodyHandle(2)]
        );
        assert_eq!(
            world_a.total_kinetic_energy_3d_checked().unwrap(),
            world_b.total_kinetic_energy_3d_checked().unwrap()
        );
    }

    #[test]
    fn duplicate_body_identity_is_rejected() {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        world.bodies = vec![body(7, 1.0, 1.0), body(7, 1.0, 2.0)];
        assert_eq!(
            world.body_kinetic_energies_3d_checked(),
            Err(WorldEnergy3dError::DuplicateBodyHandle(BodyHandle(7)))
        );
    }

    #[test]
    fn malformed_body_error_is_attributed_to_handle() {
        let mut invalid = body(4, 1.0, 1.0);
        invalid.mass = f64::NAN;
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        world.bodies.push(invalid);

        assert_eq!(
            world.total_kinetic_energy_3d_checked(),
            Err(WorldEnergy3dError::Body {
                handle: BodyHandle(4),
                source: RigidBodyEnergyError::InvalidMass,
            })
        );
    }

    #[test]
    fn individually_representable_body_energies_can_still_overflow_world_total() {
        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        world.bodies = vec![
            body(0, f64::MAX, 1.0),
            body(1, f64::MAX, 1.0),
            body(2, f64::MAX, 1.0),
        ];

        let measurements = world.body_kinetic_energies_3d_checked().unwrap();
        assert!(measurements.iter().all(|measurement| measurement.joules.is_finite()));
        assert_eq!(
            world.total_kinetic_energy_3d_checked(),
            Err(WorldEnergy3dError::UnrepresentableTotal)
        );
    }
}
