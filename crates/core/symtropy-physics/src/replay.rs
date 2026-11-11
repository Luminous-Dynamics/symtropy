// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Determinism helpers: record/replay command streams and bitwise snapshots.
//!
//! This module is intentionally minimal: it provides
//! - a small `WorldCommand` vocabulary for driving a `PhysicsWorld`
//! - a `ReplayTape` of per-tick frames (dt + commands)
//! - `WorldSnapshot`/`BodySnapshot` that capture simulation state as raw `f64` bits
//!
//! The goal is to make it easy to build a replay harness that asserts
//! **bitwise-equal state per tick** across record/replay passes.

use nalgebra::SVector;
use symtropy_math::Bivector;

use crate::body::{BodyHandle, BodyType, RigidBody, RotationalInertiaModel};
use crate::integrator;
use crate::world::PhysicsWorld;

/// Commands that mutate a physics world at a tick boundary.
#[derive(Clone, Debug)]
pub enum WorldCommand<const D: usize> {
    ApplyForce {
        body: BodyHandle,
        force: Box<SVector<f64, D>>,
    },
    ApplyImpulse {
        body: BodyHandle,
        impulse: Box<SVector<f64, D>>,
    },
    SetLinearVelocity {
        body: BodyHandle,
        velocity: Box<SVector<f64, D>>,
    },
    SetAngularVelocity {
        body: BodyHandle,
        velocity: Box<Bivector<D>>,
    },
    Wake {
        body: BodyHandle,
    },
}

/// A single replay frame: `dt` + ordered list of commands to apply before stepping.
#[derive(Clone, Debug)]
pub struct ReplayFrame<const D: usize> {
    pub dt: f64,
    pub commands: Vec<WorldCommand<D>>,
}

/// A full replay tape: ordered frames.
#[derive(Clone, Debug, Default)]
pub struct ReplayTape<const D: usize> {
    pub frames: Vec<ReplayFrame<D>>,
}

impl<const D: usize> ReplayTape<D> {
    pub fn push_frame(&mut self, dt: f64, commands: Vec<WorldCommand<D>>) {
        self.frames.push(ReplayFrame { dt, commands });
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplyCommandError {
    MissingBody(BodyHandle),
}

/// Apply a list of commands to a world.
pub fn apply_commands<const D: usize>(
    world: &mut PhysicsWorld<D>,
    commands: &[WorldCommand<D>],
) -> Result<(), ApplyCommandError> {
    for cmd in commands {
        match cmd {
            WorldCommand::ApplyForce { body, force } => {
                let Some(b) = world.body_mut(*body) else {
                    return Err(ApplyCommandError::MissingBody(*body));
                };
                b.apply_force(**force);
            }
            WorldCommand::ApplyImpulse { body, impulse } => {
                let Some(b) = world.body_mut(*body) else {
                    return Err(ApplyCommandError::MissingBody(*body));
                };
                integrator::apply_impulse(b, &**impulse);
            }
            WorldCommand::SetLinearVelocity { body, velocity } => {
                let Some(b) = world.body_mut(*body) else {
                    return Err(ApplyCommandError::MissingBody(*body));
                };
                b.linear_velocity = **velocity;
            }
            WorldCommand::SetAngularVelocity { body, velocity } => {
                let Some(b) = world.body_mut(*body) else {
                    return Err(ApplyCommandError::MissingBody(*body));
                };
                b.angular_velocity = **velocity;
            }
            WorldCommand::Wake { body } => {
                let Some(b) = world.body_mut(*body) else {
                    return Err(ApplyCommandError::MissingBody(*body));
                };
                b.wake();
            }
        }
    }
    Ok(())
}

/// Bitwise snapshot of a rigid body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BodySnapshot<const D: usize> {
    pub handle: BodyHandle,
    pub body_type: BodyType,
    pub translation: [u64; D],
    pub rotation: [[u64; D]; D],
    pub linear_velocity: [u64; D],
    pub angular_velocity: [[u64; D]; D],
    /// Bitwise dynamic mass and legacy principal-moment fallback.
    pub mass_kg_bits: u64,
    pub inertia_principal_bits: [u64; D],
    pub inverse_inertia_principal_bits: [u64; D],
    /// Complete validated body-frame operator in canonical row-major order.
    pub rotational_inertia_operator_bits: Vec<u64>,
    pub rotational_inertia_model: RotationalInertiaModel,
    pub rotational_inertia_operator_invalidated: bool,
    pub rotational_inertia_operator_mass_bits: Option<u64>,
    pub rotational_inertia_approximate: bool,
    pub sleeping: bool,
    pub sleep_counter: u32,
}

impl<const D: usize> BodySnapshot<D> {
    pub fn from_body(body: &RigidBody<D>) -> Self {
        let translation = std::array::from_fn(|i| body.transform.translation.0[i].to_bits());

        let rot = body.transform.rotation.to_matrix();
        let rotation = std::array::from_fn(|r| std::array::from_fn(|c| rot[(r, c)].to_bits()));

        let linear_velocity = std::array::from_fn(|i| body.linear_velocity[i].to_bits());

        let ang = body.angular_velocity.to_matrix();
        let angular_velocity =
            std::array::from_fn(|r| std::array::from_fn(|c| ang[(r, c)].to_bits()));

        Self {
            handle: body.handle,
            body_type: body.body_type,
            translation,
            rotation,
            linear_velocity,
            angular_velocity,
            mass_kg_bits: body.mass.to_bits(),
            inertia_principal_bits: std::array::from_fn(|i| body.inertia[i].to_bits()),
            inverse_inertia_principal_bits: std::array::from_fn(|i| body.inv_inertia[i].to_bits()),
            rotational_inertia_operator_bits: body
                .rotational_inertia_operator
                .as_ref()
                .map(|operator| operator.as_row_major().iter().map(|value| value.to_bits()).collect())
                .unwrap_or_default(),
            rotational_inertia_model: body.rotational_inertia_model(),
            rotational_inertia_operator_invalidated: body.rotational_inertia_operator_invalidated,
            rotational_inertia_operator_mass_bits: body.rotational_inertia_operator_mass_bits,
            rotational_inertia_approximate: body.rotational_inertia_approximate,
            sleeping: body.sleeping,
            sleep_counter: body.sleep_counter,
        }
    }
}

/// Bitwise snapshot of a collision event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollisionEventSnapshot<const D: usize> {
    pub body_a: BodyHandle,
    pub body_b: BodyHandle,
    pub impulse: u64,
    pub normal: [u64; D],
    pub depth: u64,
}

impl<const D: usize> CollisionEventSnapshot<D> {
    pub fn from_event(event: &crate::contact::CollisionEvent<D>) -> Self {
        Self {
            body_a: event.body_a,
            body_b: event.body_b,
            impulse: event.impulse.to_bits(),
            normal: std::array::from_fn(|i| event.normal[i].to_bits()),
            depth: event.depth.to_bits(),
        }
    }
}

/// Bitwise snapshot of a physics world (bodies + last-step collision events).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldSnapshot<const D: usize> {
    pub bodies: Vec<BodySnapshot<D>>,
    pub collision_events: Vec<CollisionEventSnapshot<D>>,
}

impl<const D: usize> WorldSnapshot<D> {
    pub fn capture(world: &PhysicsWorld<D>) -> Self {
        let mut bodies: Vec<_> = world.bodies.iter().map(BodySnapshot::from_body).collect();
        bodies.sort_by_key(|b| b.handle);

        let mut collision_events: Vec<_> = world
            .collision_events
            .iter()
            .map(CollisionEventSnapshot::from_event)
            .collect();
        collision_events.sort_by_key(|e| (e.body_a, e.body_b, e.impulse, e.depth));

        Self {
            bodies,
            collision_events,
        }
    }
}


#[cfg(test)]
mod rotational_mass_property_replay_tests {
    use super::*;
    use crate::mass_properties::RotationalInertiaOperator;
    use symtropy_math::Point;

    #[test]
    fn body_snapshot_distinguishes_full_operator_coefficients() {
        let operator_a = RotationalInertiaOperator::<3>::try_from_row_major(vec![
            4.0, 0.5, 0.0,
            0.5, 3.0, 0.25,
            0.0, 0.25, 2.0,
        ]).unwrap();
        let operator_b = RotationalInertiaOperator::<3>::try_from_row_major(vec![
            4.0, 0.25, 0.0,
            0.25, 3.0, 0.25,
            0.0, 0.25, 2.0,
        ]).unwrap();
        let a = RigidBody::<3>::dynamic_sphere(
            BodyHandle(7),
            Point::origin(),
            0.1,
            1.0,
        ).with_rotational_inertia_operator(operator_a);
        let b = RigidBody::<3>::dynamic_sphere(
            BodyHandle(7),
            Point::origin(),
            0.1,
            1.0,
        ).with_rotational_inertia_operator(operator_b);

        let snapshot_a = BodySnapshot::from_body(&a);
        let snapshot_b = BodySnapshot::from_body(&b);
        assert_ne!(
            snapshot_a.rotational_inertia_operator_bits,
            snapshot_b.rotational_inertia_operator_bits
        );
        assert_ne!(snapshot_a, snapshot_b);
    }

    #[test]
    fn body_snapshot_records_mass_operator_provenance_mismatch() {
        let operator = RotationalInertiaOperator::<3>::isotropic(1.0).unwrap();
        let mut body = RigidBody::<3>::dynamic_sphere(
            BodyHandle(9),
            Point::origin(),
            0.1,
            1.0,
        ).with_rotational_inertia_operator(operator);
        let valid = BodySnapshot::from_body(&body);

        body.mass = 1.25;
        let stale = BodySnapshot::from_body(&body);
        assert_eq!(valid.rotational_inertia_operator_mass_bits, Some(1.0_f64.to_bits()));
        assert_eq!(stale.rotational_inertia_operator_mass_bits, Some(1.0_f64.to_bits()));
        assert_eq!(stale.mass_kg_bits, 1.25_f64.to_bits());
        assert_eq!(
            stale.rotational_inertia_model,
            RotationalInertiaModel::InvalidPrincipalMoments
        );
        assert_ne!(valid, stale);
    }

    #[test]
    fn body_snapshot_records_inertia_model_and_stale_operator_state() {
        let operator = RotationalInertiaOperator::<3>::isotropic(1.0).unwrap();
        let mut body = RigidBody::<3>::dynamic_sphere(
            BodyHandle(8),
            Point::origin(),
            0.1,
            1.0,
        ).with_rotational_inertia_operator(operator);
        let valid = BodySnapshot::from_body(&body);
        assert_eq!(
            valid.rotational_inertia_model,
            RotationalInertiaModel::ValidatedSpatialBivectorOperator
        );
        assert!(!valid.rotational_inertia_operator_invalidated);

        body.merge_toolhead(
            symtropy_math::Transform::identity(),
            Box::new(symtropy_math::Sphere::unit()),
            0.2,
        );
        let stale = BodySnapshot::from_body(&body);
        assert!(stale.rotational_inertia_operator_invalidated);
        assert!(stale.rotational_inertia_approximate);
        assert_eq!(
            stale.rotational_inertia_model,
            RotationalInertiaModel::InvalidPrincipalMoments
        );
        assert_ne!(valid, stale);
    }
}
