// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Process-local capability layer for physical-authority and generation namespaces.
//!
//! This module is intentionally narrower than persistent/authenticated identity.
//! It prevents ordinary safe code from manufacturing qualified namespace ownership
//! merely by replaying naked `PhysicalAuthorityId` / `WorldGenerationId` values
//! inside one running process.
//!
//! It does not establish cross-process uniqueness, structural world validity,
//! cryptographic provenance, temporal continuity, or restore continuity.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::identity_authority::{
    PhysicalAuthorityId, PhysicsAuthorityWorld, PhysicsIdentityError, WorldGenerationId,
};
use crate::world::PhysicsWorld;

/// Monotonic process-local allocator for fresh authority-root identifiers.
///
/// `Relaxed` ordering is sufficient here: the theorem needs one atomic
/// modification order / no duplicate returned values, not synchronization of
/// unrelated memory.
static NEXT_LOCAL_AUTHORITY_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LocalAuthorityNamespaceError {
    LocalAuthorityIdExhausted,
    WorldGenerationIdExhausted,
    IdentityInvariantViolation(PhysicsIdentityError),
}

/// Non-cloneable process-local capability representing one physical-authority
/// namespace root.
///
/// The contained numeric `PhysicalAuthorityId` remains reference data. The
/// capability is the stronger value: callers cannot construct it from a naked
/// ID through the production safe API.
///
/// This is deliberately process-local. Persistent/external ownership of the
/// same physical-authority lineage requires a separate authenticated credential
/// theorem.
pub struct LocalQualifiedPhysicalAuthority {
    physical_authority_id: PhysicalAuthorityId,
    next_generation_id: u128,
}

impl LocalQualifiedPhysicalAuthority {
    /// Mint one fresh process-local authority root.
    ///
    /// The allocator is monotonic and checked. It never wraps and never returns
    /// zero. A process restart is outside this theorem and may restart the local
    /// numeric allocator; persistent evidence must therefore not treat this
    /// constructor as cross-process authority.
    pub fn mint() -> Result<Self, LocalAuthorityNamespaceError> {
        let raw = NEXT_LOCAL_AUTHORITY_ID
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map_err(|_| LocalAuthorityNamespaceError::LocalAuthorityIdExhausted)?;

        let physical_authority_id = PhysicalAuthorityId::new(u128::from(raw))
            .map_err(LocalAuthorityNamespaceError::IdentityInvariantViolation)?;

        Ok(Self {
            physical_authority_id,
            next_generation_id: 1,
        })
    }

    pub const fn physical_authority_id(&self) -> PhysicalAuthorityId {
        self.physical_authority_id
    }

    /// Mint one generation capability under this exact local authority root.
    ///
    /// Generation values are monotonic within the root and never reused by this
    /// capability. The root itself is non-cloneable, so ordinary safe code cannot
    /// fork two independent generation allocators for the same qualified root.
    pub fn mint_generation(
        &mut self,
    ) -> Result<LocalQualifiedWorldGeneration, LocalAuthorityNamespaceError> {
        let raw = self.next_generation_id;
        let next = raw
            .checked_add(1)
            .ok_or(LocalAuthorityNamespaceError::WorldGenerationIdExhausted)?;
        let world_generation_id = WorldGenerationId::new(raw)
            .map_err(LocalAuthorityNamespaceError::IdentityInvariantViolation)?;
        self.next_generation_id = next;

        Ok(LocalQualifiedWorldGeneration {
            physical_authority_id: self.physical_authority_id,
            world_generation_id,
        })
    }
}

/// Non-cloneable capability granting one process-local `(A, G)` namespace.
///
/// There is intentionally no public constructor from naked identifiers and no
/// serde authority. The only production constructor is
/// `LocalQualifiedPhysicalAuthority::mint_generation`.
pub struct LocalQualifiedWorldGeneration {
    physical_authority_id: PhysicalAuthorityId,
    world_generation_id: WorldGenerationId,
}

impl LocalQualifiedWorldGeneration {
    pub const fn physical_authority_id(&self) -> PhysicalAuthorityId {
        self.physical_authority_id
    }

    pub const fn world_generation_id(&self) -> WorldGenerationId {
        self.world_generation_id
    }
}

/// Namespace-qualified wrapper around the existing `PhysicsAuthorityWorld`.
///
/// This type proves only that the inner authority/world pair was constructed by
/// consuming one local generation capability. It does **not** certify the
/// structural integrity of an arbitrary supplied `PhysicsWorld`; that remains
/// PHYS-ID-01C/01D (#1005/#1019).
///
/// The wrapper deliberately does not implement `Deref`. Callers must make the
/// downward projection to the legacy authority surface explicit.
pub struct LocalNamespacePhysicsAuthorityWorld<const D: usize> {
    generation: LocalQualifiedWorldGeneration,
    authority_world: PhysicsAuthorityWorld<D>,
}

impl<const D: usize> LocalNamespacePhysicsAuthorityWorld<D> {
    /// Consume exactly one qualified generation capability and bind one raw
    /// physics world into that namespace.
    ///
    /// This operation establishes namespace provenance only. The raw world may
    /// still require a future checked structural-adoption theorem before stronger
    /// evidence claims are admitted.
    pub fn bind(
        generation: LocalQualifiedWorldGeneration,
        world: PhysicsWorld<D>,
    ) -> Self {
        let authority_world = PhysicsAuthorityWorld::new(
            generation.physical_authority_id,
            generation.world_generation_id,
            world,
        );
        Self {
            generation,
            authority_world,
        }
    }

    pub const fn physical_authority_id(&self) -> PhysicalAuthorityId {
        self.generation.physical_authority_id
    }

    pub const fn world_generation_id(&self) -> WorldGenerationId {
        self.generation.world_generation_id
    }

    /// Explicit downward projection to the existing authority surface.
    pub const fn authority_world(&self) -> &PhysicsAuthorityWorld<D> {
        &self.authority_world
    }

    /// Explicit mutable downward projection while retaining this wrapper's
    /// namespace capability.
    pub fn authority_world_mut(&mut self) -> &mut PhysicsAuthorityWorld<D> {
        &mut self.authority_world
    }

    /// Consume the qualified wrapper and return only the weaker legacy
    /// authority object. Namespace qualification is intentionally lost.
    pub fn into_unqualified(self) -> PhysicsAuthorityWorld<D> {
        self.authority_world
    }
}
