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

use std::collections::HashSet;

use nalgebra::SVector;
use symtropy_math::Bivector;

use crate::body::{BodyHandle, BodyType, NetId, RigidBody};
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

/// Optional runtime identity for one body in a [WorldSnapshotWithRuntimeIds].
///
/// NetId is a scoped runtime hint, not a canonical engineering subject.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BodyRuntimeIdentitySnapshot {
    pub handle: BodyHandle,
    pub net_id: Option<NetId>,
}

/// Additive identity-enriched snapshot that leaves [WorldSnapshot] and
/// [BodySnapshot] layout unchanged for existing consumers.
///
/// body_identities is aligned with snapshot.bodies and retains explicit
/// None values so absence is not confused with a missing sidecar entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldSnapshotWithRuntimeIds<const D: usize> {
    pub snapshot: WorldSnapshot<D>,
    pub body_identities: Vec<BodyRuntimeIdentitySnapshot>,
}

/// Structural failures in an identity-enriched snapshot sidecar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeIdentitySnapshotError {
    BodyCountMismatch {
        snapshot_count: usize,
        identity_count: usize,
    },
    BodyHandleMismatch {
        index: usize,
        snapshot_handle: BodyHandle,
        identity_handle: BodyHandle,
    },
    DuplicateBodyHandle(BodyHandle),
    DuplicateNetId(NetId),
}

impl<const D: usize> WorldSnapshotWithRuntimeIds<D> {
    /// Check that the identity sidecar is one-to-one and aligned with the
    /// ordered body list in the ordinary snapshot.
    ///
    /// A NetId is unique within one PhysicsWorld by the runtime lookup-map
    /// contract. This validates the captured sidecar, not the authority or
    /// world-generation scope of the identifier.
    pub fn validate_alignment(&self) -> Result<(), RuntimeIdentitySnapshotError> {
        if self.snapshot.bodies.len() != self.body_identities.len() {
            return Err(RuntimeIdentitySnapshotError::BodyCountMismatch {
                snapshot_count: self.snapshot.bodies.len(),
                identity_count: self.body_identities.len(),
            });
        }

        let mut handles = HashSet::with_capacity(self.body_identities.len());
        let mut net_ids = HashSet::with_capacity(self.body_identities.len());
        for (index, (body, identity)) in self
            .snapshot
            .bodies
            .iter()
            .zip(&self.body_identities)
            .enumerate()
        {
            if body.handle != identity.handle {
                return Err(RuntimeIdentitySnapshotError::BodyHandleMismatch {
                    index,
                    snapshot_handle: body.handle,
                    identity_handle: identity.handle,
                });
            }
            if !handles.insert(identity.handle) {
                return Err(RuntimeIdentitySnapshotError::DuplicateBodyHandle(
                    identity.handle,
                ));
            }
            if let Some(net_id) = identity.net_id
                && !net_ids.insert(net_id)
            {
                return Err(RuntimeIdentitySnapshotError::DuplicateNetId(net_id));
            }
        }
        Ok(())
    }
}

impl<const D: usize> WorldSnapshot<D> {
    /// Capture the existing bitwise snapshot plus an aligned optional-NetId
    /// sidecar without changing the existing snapshot struct layout.
    pub fn capture_with_runtime_ids(world: &PhysicsWorld<D>) -> WorldSnapshotWithRuntimeIds<D> {
        let snapshot = Self::capture(world);
        let body_identities = snapshot
            .bodies
            .iter()
            .map(|body| BodyRuntimeIdentitySnapshot {
                handle: body.handle,
                net_id: world.body(body.handle).and_then(|source| source.net_id),
            })
            .collect();
        WorldSnapshotWithRuntimeIds {
            snapshot,
            body_identities,
        }
    }

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
mod tests {
    use super::*;
    use nalgebra::SVector;
    use symtropy_math::Point;

    #[test]
    fn runtime_identity_sidecar_preserves_present_and_absent_net_ids() {
        let mut world = PhysicsWorld::<2>::new(SVector::zeros());
        let identified = world.add_sphere(Point::new([0.0, 0.0]), 1.0, 1.0);
        let unidentified = world.add_sphere(Point::new([3.0, 0.0]), 1.0, 1.0);
        world.set_net_id(identified, NetId(77));

        let snapshot = WorldSnapshot::capture_with_runtime_ids(&world);
        assert_eq!(snapshot.snapshot.bodies.len(), 2);
        assert_eq!(snapshot.body_identities.len(), 2);
        assert_eq!(
            snapshot.body_identities[0].handle,
            snapshot.snapshot.bodies[0].handle
        );
        assert_eq!(
            snapshot.body_identities[1].handle,
            snapshot.snapshot.bodies[1].handle
        );

        let identified_identity = snapshot
            .body_identities
            .iter()
            .find(|identity| identity.handle == identified)
            .expect("identified sidecar entry");
        let unidentified_identity = snapshot
            .body_identities
            .iter()
            .find(|identity| identity.handle == unidentified)
            .expect("unidentified sidecar entry");

        assert_eq!(identified_identity.net_id, Some(NetId(77)));
        assert_eq!(unidentified_identity.net_id, None);
    }

    #[test]
    fn runtime_identity_sidecar_validates_when_aligned_and_unique() {
        let mut world = PhysicsWorld::<2>::new(SVector::zeros());
        let first = world.add_sphere(Point::new([0.0, 0.0]), 1.0, 1.0);
        let second = world.add_sphere(Point::new([3.0, 0.0]), 1.0, 1.0);
        world.set_net_id(first, NetId(1));
        world.set_net_id(second, NetId(2));

        let snapshot = WorldSnapshot::capture_with_runtime_ids(&world);
        assert_eq!(snapshot.validate_alignment(), Ok(()));
    }

    #[test]
    fn runtime_identity_sidecar_rejects_body_count_mismatch() {
        let mut world = PhysicsWorld::<2>::new(SVector::zeros());
        world.add_sphere(Point::new([0.0, 0.0]), 1.0, 1.0);
        let mut snapshot = WorldSnapshot::capture_with_runtime_ids(&world);
        snapshot.body_identities.clear();
        assert_eq!(
            snapshot.validate_alignment(),
            Err(RuntimeIdentitySnapshotError::BodyCountMismatch {
                snapshot_count: 1,
                identity_count: 0
            })
        );
    }

    #[test]
    fn runtime_identity_sidecar_rejects_misaligned_handle() {
        let mut world = PhysicsWorld::<2>::new(SVector::zeros());
        world.add_sphere(Point::new([0.0, 0.0]), 1.0, 1.0);
        world.add_sphere(Point::new([3.0, 0.0]), 1.0, 1.0);
        let mut snapshot = WorldSnapshot::capture_with_runtime_ids(&world);
        snapshot.body_identities.swap(0, 1);
        assert!(matches!(
            snapshot.validate_alignment(),
            Err(RuntimeIdentitySnapshotError::BodyHandleMismatch { index: 0, .. })
        ));
    }

    #[test]
    fn runtime_identity_sidecar_rejects_duplicate_net_ids() {
        let mut world = PhysicsWorld::<2>::new(SVector::zeros());
        let first = world.add_sphere(Point::new([0.0, 0.0]), 1.0, 1.0);
        let second = world.add_sphere(Point::new([3.0, 0.0]), 1.0, 1.0);
        world.set_net_id(first, NetId(99));
        // Model a corrupted state introduced through the public mutable body
        // API. The sidecar validator must surface it even when setter APIs
        // reject duplicate assignments.
        world.body_mut(second).expect("second body").net_id = Some(NetId(99));

        let snapshot = WorldSnapshot::capture_with_runtime_ids(&world);
        assert_eq!(
            snapshot.validate_alignment(),
            Err(RuntimeIdentitySnapshotError::DuplicateNetId(NetId(99)))
        );
    }

    #[test]
    fn runtime_identity_capture_leaves_existing_snapshot_unchanged() {
        let mut world = PhysicsWorld::<2>::new(SVector::zeros());
        let handle = world.add_sphere(Point::new([0.0, 0.0]), 1.0, 1.0);
        world.set_net_id(handle, NetId(42));

        let ordinary = WorldSnapshot::capture(&world);
        let enriched = WorldSnapshot::capture_with_runtime_ids(&world);
        assert_eq!(ordinary, enriched.snapshot);
        assert_eq!(ordinary.bodies.len(), enriched.body_identities.len());
    }
}
