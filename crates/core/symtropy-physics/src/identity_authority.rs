// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Physics-owned live identity authority.
//!
//! This module separates durable physical-world identity from one runtime
//! generation and from ephemeral body handles. It deliberately does not claim
//! persistence or historical continuity across generations.

use serde::{Deserialize, Serialize};

use crate::body::{BodyHandle, NetId, RigidBody};
use crate::identity_mutation::{
    NetIdentityMutationError, add_bodies_deterministic_checked, assign_net_id_checked,
};
use crate::world::PhysicsWorld;

/// Durable identity of the physical authority/persistence lineage that owns a
/// namespace of world generations.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PhysicalAuthorityId(u128);

impl PhysicalAuthorityId {
    pub fn new(value: u128) -> Result<Self, PhysicsIdentityError> {
        if value == 0 {
            return Err(PhysicsIdentityError::ZeroPhysicalAuthorityId);
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> u128 {
        self.0
    }
}

/// Identity of one instantiated world generation within one physical authority
/// lineage.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WorldGenerationId(u128);

impl WorldGenerationId {
    pub fn new(value: u128) -> Result<Self, PhysicsIdentityError> {
        if value == 0 {
            return Err(PhysicsIdentityError::ZeroWorldGenerationId);
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> u128 {
        self.0
    }
}

/// Durable-looking subject reference for one live physics body.
///
/// This reference intentionally contains no [`BodyHandle`]. A body handle is
/// allocated by one runtime `PhysicsWorld` and is not evidence of identity
/// across reconstruction.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PhysicsBodySubject {
    physical_authority_id: PhysicalAuthorityId,
    world_generation_id: WorldGenerationId,
    net_id: NetId,
}

impl PhysicsBodySubject {
    pub const fn new(
        physical_authority_id: PhysicalAuthorityId,
        world_generation_id: WorldGenerationId,
        net_id: NetId,
    ) -> Self {
        Self {
            physical_authority_id,
            world_generation_id,
            net_id,
        }
    }

    pub const fn physical_authority_id(self) -> PhysicalAuthorityId {
        self.physical_authority_id
    }

    pub const fn world_generation_id(self) -> WorldGenerationId {
        self.world_generation_id
    }

    pub const fn net_id(self) -> NetId {
        self.net_id
    }
}

/// Physics-owned binding of one live `PhysicsWorld` to its authority lineage
/// and runtime generation.
///
/// Downstream callers cannot supply a detached claimed authority ID alongside an
/// unrelated world: the identity and world are owned by the same object.
pub struct PhysicsAuthorityWorld<const D: usize> {
    physical_authority_id: PhysicalAuthorityId,
    world_generation_id: WorldGenerationId,
    world: PhysicsWorld<D>,
}

impl<const D: usize> PhysicsAuthorityWorld<D> {
    pub fn new(
        physical_authority_id: PhysicalAuthorityId,
        world_generation_id: WorldGenerationId,
        world: PhysicsWorld<D>,
    ) -> Self {
        Self {
            physical_authority_id,
            world_generation_id,
            world,
        }
    }

    pub const fn physical_authority_id(&self) -> PhysicalAuthorityId {
        self.physical_authority_id
    }

    pub const fn world_generation_id(&self) -> WorldGenerationId {
        self.world_generation_id
    }

    pub fn world(&self) -> &PhysicsWorld<D> {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut PhysicsWorld<D> {
        &mut self.world
    }

    pub fn into_world(self) -> PhysicsWorld<D> {
        self.world
    }

    /// Bind one previously-unbound live body to a stable `NetId` within this
    /// exact physical authority and world generation.
    ///
    /// The underlying mutation follows the checked bind-once law (`None -> N`
    /// allowed, `N -> N` idempotent, `N -> M` rejected). On success, the caller
    /// receives the exact authority/generation-bound subject that was created.
    pub fn bind_net_id(
        &mut self,
        handle: BodyHandle,
        net_id: NetId,
    ) -> Result<PhysicsBodySubject, NetIdentityMutationError> {
        assign_net_id_checked(&mut self.world, handle, net_id)?;

        debug_assert_eq!(self.world.net_id_for_handle(handle), Some(net_id));
        debug_assert_eq!(self.world.handle_for_net_id(net_id), Some(handle));

        Ok(PhysicsBodySubject::new(
            self.physical_authority_id,
            self.world_generation_id,
            net_id,
        ))
    }

    /// Insert a deterministic batch under this exact authority/generation.
    ///
    /// The complete identity set is preflighted before insertion by the checked
    /// coordinator. Successful results pair each ephemeral runtime handle with
    /// the stable authority/generation-bound subject for that inserted body.
    pub fn add_bodies_deterministic(
        &mut self,
        bodies: Vec<(NetId, RigidBody<D>)>,
    ) -> Result<Vec<(BodyHandle, PhysicsBodySubject)>, NetIdentityMutationError> {
        let handles = add_bodies_deterministic_checked(&mut self.world, bodies)?;
        let mut bindings = Vec::with_capacity(handles.len());

        for handle in handles {
            let net_id = self
                .world
                .net_id_for_handle(handle)
                .expect("checked deterministic insertion must bind every returned handle");
            bindings.push((
                handle,
                PhysicsBodySubject::new(
                    self.physical_authority_id,
                    self.world_generation_id,
                    net_id,
                ),
            ));
        }

        Ok(bindings)
    }

    /// Validate one exact body subject against this exact authority/generation.
    ///
    /// Validation does not trust only the `NetId -> BodyHandle` index. It scans
    /// live body state, requires exactly one matching body, and then checks both
    /// forward and reverse indexed views against that body.
    pub fn validate_subject(
        &self,
        subject: PhysicsBodySubject,
    ) -> Result<ValidatedNetBody<'_, D>, PhysicsIdentityError> {
        if subject.physical_authority_id != self.physical_authority_id {
            return Err(PhysicsIdentityError::PhysicalAuthorityMismatch {
                expected: subject.physical_authority_id,
                actual: self.physical_authority_id,
            });
        }
        if subject.world_generation_id != self.world_generation_id {
            return Err(PhysicsIdentityError::WorldGenerationMismatch {
                expected: subject.world_generation_id,
                actual: self.world_generation_id,
            });
        }

        let net_id = subject.net_id;
        let mut unique_handle = None;
        for body in &self.world.bodies {
            if body.net_id() != Some(net_id) {
                continue;
            }
            if unique_handle.replace(body.handle).is_some() {
                return Err(PhysicsIdentityError::AmbiguousNetId { net_id });
            }
        }

        let handle = unique_handle.ok_or(PhysicsIdentityError::UnknownNetId { net_id })?;
        let mapped_handle = self
            .world
            .handle_for_net_id(net_id)
            .ok_or(PhysicsIdentityError::IdentityIndexMissing { net_id, handle })?;
        if mapped_handle != handle {
            return Err(PhysicsIdentityError::IdentityIndexMismatch {
                net_id,
                body_handle: handle,
                mapped_handle,
            });
        }
        if self.world.net_id_for_handle(handle) != Some(net_id) {
            return Err(PhysicsIdentityError::ReverseIdentityMismatch { net_id, handle });
        }

        Ok(ValidatedNetBody {
            authority: self,
            subject,
            handle,
        })
    }
}

/// Non-serializable, lifetime-bound proof that one subject resolves to exactly
/// one consistently indexed live body in one exact authority/world generation.
///
/// Holding this token borrows the authority world immutably, so ordinary safe
/// code cannot obtain `&mut PhysicsAuthorityWorld` until the token is dropped.
pub struct ValidatedNetBody<'a, const D: usize> {
    authority: &'a PhysicsAuthorityWorld<D>,
    subject: PhysicsBodySubject,
    handle: BodyHandle,
}

impl<'a, const D: usize> ValidatedNetBody<'a, D> {
    pub const fn subject(&self) -> PhysicsBodySubject {
        self.subject
    }

    pub const fn net_id(&self) -> NetId {
        self.subject.net_id
    }

    /// Ephemeral runtime handle. This must not be persisted as durable identity.
    pub const fn runtime_handle(&self) -> BodyHandle {
        self.handle
    }

    pub fn body(&self) -> &RigidBody<D> {
        self.authority
            .world
            .body(self.handle)
            .expect("validated body must remain present while authority is immutably borrowed")
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PhysicsIdentityError {
    ZeroPhysicalAuthorityId,
    ZeroWorldGenerationId,
    PhysicalAuthorityMismatch {
        expected: PhysicalAuthorityId,
        actual: PhysicalAuthorityId,
    },
    WorldGenerationMismatch {
        expected: WorldGenerationId,
        actual: WorldGenerationId,
    },
    UnknownNetId {
        net_id: NetId,
    },
    AmbiguousNetId {
        net_id: NetId,
    },
    IdentityIndexMissing {
        net_id: NetId,
        handle: BodyHandle,
    },
    IdentityIndexMismatch {
        net_id: NetId,
        body_handle: BodyHandle,
        mapped_handle: BodyHandle,
    },
    ReverseIdentityMismatch {
        net_id: NetId,
        handle: BodyHandle,
    },
}

#[cfg(test)]
mod privileged_corruption_tests {
    use super::*;
    use symtropy_math::Point;

    #[test]
    fn direct_body_identity_corruption_leaving_stale_index_fails_closed() {
        let indexed_id = NetId(45);
        let mutated_id = NetId(46);
        let authority = PhysicalAuthorityId::new(4001).unwrap();
        let generation = WorldGenerationId::new(1).unwrap();
        let mut raw = PhysicsWorld::<3>::default();
        let handle = raw.add_sphere(Point::origin(), 0.5, 1.0);
        raw.set_net_id(handle, indexed_id);
        raw.body_mut(handle).expect("body exists").net_id = Some(mutated_id);

        let world = PhysicsAuthorityWorld::new(authority, generation, raw);
        let subject = PhysicsBodySubject::new(authority, generation, mutated_id);

        assert!(matches!(
            world.validate_subject(subject),
            Err(PhysicsIdentityError::IdentityIndexMissing {
                net_id: observed,
                handle: observed_handle,
            }) if observed == mutated_id && observed_handle == handle
        ));
    }

    #[test]
    fn duplicate_live_net_id_fails_closed_even_if_index_points_to_one_body() {
        let net_id = NetId(44);
        let authority = PhysicalAuthorityId::new(3001).unwrap();
        let generation = WorldGenerationId::new(1).unwrap();
        let mut raw = PhysicsWorld::<3>::default();
        let first = raw.add_sphere(Point::new([0.0, 0.0, 0.0]), 0.5, 1.0);
        let second = raw.add_sphere(Point::new([1.0, 0.0, 0.0]), 0.5, 1.0);
        raw.set_net_id(first, net_id);
        raw.set_net_id(second, net_id);

        let world = PhysicsAuthorityWorld::new(authority, generation, raw);
        let subject = PhysicsBodySubject::new(authority, generation, net_id);

        assert!(matches!(
            world.validate_subject(subject),
            Err(PhysicsIdentityError::AmbiguousNetId { net_id: observed }) if observed == net_id
        ));
    }
}
