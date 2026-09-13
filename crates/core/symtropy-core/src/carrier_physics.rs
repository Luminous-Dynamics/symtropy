// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! ECON-05C resolves an economic carrier binding to exactly one live physics body.
//!
//! `BodyHandle` is runtime-local allocation state and is therefore never stored as
//! durable economic authority here. `NetId` is the physics world's stable network
//! identity, but this module still treats it as a live-resolution key rather than
//! proof of persistence across save/load. The opaque physical-authority identifier
//! binds the reference to the same external authority named by ECON-05A.

use crate::economic::{AssetId, CausalId};
use crate::freight_load::CarrierCargoBinding;
use crate::physics::body::{BodyHandle, NetId};
use crate::physics::world::PhysicsWorld;

/// Durable-looking reference data used to request a *live* physical resolution.
///
/// This type contains no `BodyHandle`: handles are allocated by each `PhysicsWorld`
/// instance and reset with that runtime. `physical_authority_id` is intentionally
/// opaque. Callers should use it to identify the exact world/session/persistence
/// authority that owns the `NetId` namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarrierPhysicsRef {
    carrier_asset_id: AssetId,
    physical_authority_id: CausalId,
    net_id: NetId,
}

impl CarrierPhysicsRef {
    pub fn new(
        carrier_asset_id: AssetId,
        physical_authority_id: CausalId,
        net_id: NetId,
    ) -> Self {
        Self {
            carrier_asset_id,
            physical_authority_id,
            net_id,
        }
    }

    pub fn carrier_asset_id(&self) -> &AssetId {
        &self.carrier_asset_id
    }

    pub fn physical_authority_id(&self) -> &CausalId {
        &self.physical_authority_id
    }

    pub const fn net_id(&self) -> NetId {
        self.net_id
    }
}

/// Resolve a carrier reference to exactly one live physics body.
///
/// Resolution is deliberately defensive. It does not trust only
/// `PhysicsWorld::handle_for_net_id`: the current physics API permits direct
/// mutation of `RigidBody::net_id`, and its legacy `set_net_id` method can create
/// duplicate IDs. We therefore scan body truth and require all three views to
/// agree:
///
/// 1. exactly one live body contains `NetId`;
/// 2. `NetId -> BodyHandle` map resolves to that body;
/// 3. `BodyHandle -> NetId` resolves back to the same ID.
///
/// The returned `BodyHandle` is runtime-only and must not be persisted as carrier
/// identity or copied into economic evidence.
pub fn resolve_live_carrier<const D: usize>(
    world: &PhysicsWorld<D>,
    cargo_binding: &CarrierCargoBinding,
    physics_ref: &CarrierPhysicsRef,
) -> Result<BodyHandle, CarrierPhysicsError> {
    if cargo_binding.carrier_asset_id() != physics_ref.carrier_asset_id() {
        return Err(CarrierPhysicsError::CarrierAssetMismatch {
            economic: cargo_binding.carrier_asset_id().clone(),
            physical: physics_ref.carrier_asset_id().clone(),
        });
    }
    if cargo_binding.physical_authority_id() != physics_ref.physical_authority_id() {
        return Err(CarrierPhysicsError::PhysicalAuthorityMismatch {
            economic: cargo_binding.physical_authority_id().clone(),
            physical: physics_ref.physical_authority_id().clone(),
        });
    }

    let net_id = physics_ref.net_id();
    let mut unique_handle = None;
    for body in &world.bodies {
        if body.net_id != Some(net_id) {
            continue;
        }
        if unique_handle.replace(body.handle).is_some() {
            return Err(CarrierPhysicsError::AmbiguousNetId { net_id });
        }
    }

    let handle = unique_handle.ok_or(CarrierPhysicsError::UnknownNetId { net_id })?;
    let mapped_handle = world
        .handle_for_net_id(net_id)
        .ok_or(CarrierPhysicsError::IdentityIndexMissing { net_id, handle })?;
    if mapped_handle != handle {
        return Err(CarrierPhysicsError::IdentityIndexMismatch {
            net_id,
            body_handle: handle,
            mapped_handle,
        });
    }
    if world.net_id_for_handle(handle) != Some(net_id) {
        return Err(CarrierPhysicsError::ReverseIdentityMismatch { net_id, handle });
    }

    Ok(handle)
}

/// Fail-closed live carrier identity errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CarrierPhysicsError {
    CarrierAssetMismatch {
        economic: AssetId,
        physical: AssetId,
    },
    PhysicalAuthorityMismatch {
        economic: CausalId,
        physical: CausalId,
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
