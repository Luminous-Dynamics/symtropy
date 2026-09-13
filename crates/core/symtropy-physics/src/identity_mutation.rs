// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Checked mutation coordinators for physics network identity.
//!
//! This module does not yet seal the legacy public mutation surfaces. Instead,
//! it provides typed, whole-operation preflight wrappers that make the safe path
//! explicit and fail closed before invoking those legacy mutators.

use std::collections::BTreeSet;

use crate::body::{BodyHandle, NetId, RigidBody};
use crate::world::PhysicsWorld;

/// Assign one `NetId` only after proving the current body/index views are
/// mutually consistent and the requested identity is not owned elsewhere.
pub fn assign_net_id_checked<const D: usize>(
    world: &mut PhysicsWorld<D>,
    handle: BodyHandle,
    net_id: NetId,
) -> Result<(), NetIdentityMutationError> {
    let current = world
        .body(handle)
        .ok_or(NetIdentityMutationError::UnknownHandle { handle })?
        .net_id;

    let target_handles = live_handles_for_net_id(world, net_id);
    if target_handles.len() > 1 {
        return Err(NetIdentityMutationError::AmbiguousLiveNetId { net_id });
    }
    if let Some(existing_handle) = target_handles.first().copied()
        && existing_handle != handle
    {
        return Err(NetIdentityMutationError::NetIdAlreadyBound {
            net_id,
            existing_handle,
        });
    }

    match world.handle_for_net_id(net_id) {
        Some(mapped_handle) if mapped_handle != handle => {
            return Err(NetIdentityMutationError::TargetIndexOccupied {
                net_id,
                mapped_handle,
            });
        }
        Some(mapped_handle) if target_handles.is_empty() => {
            return Err(NetIdentityMutationError::TargetIndexWithoutBody {
                net_id,
                mapped_handle,
            });
        }
        _ => {}
    }

    if current == Some(net_id) {
        if target_handles.as_slice() != [handle] {
            return Err(NetIdentityMutationError::CurrentBodyIdentityMismatch { handle, net_id });
        }
        if world.handle_for_net_id(net_id) != Some(handle)
            || world.net_id_for_handle(handle) != Some(net_id)
        {
            return Err(NetIdentityMutationError::CurrentIdentityIndexMismatch {
                handle,
                net_id,
            });
        }
        return Ok(());
    }

    if let Some(old_id) = current {
        let old_handles = live_handles_for_net_id(world, old_id);
        if old_handles.as_slice() != [handle] {
            return Err(NetIdentityMutationError::CurrentBodyIdentityMismatch {
                handle,
                net_id: old_id,
            });
        }
        match world.handle_for_net_id(old_id) {
            Some(mapped_handle) if mapped_handle == handle => {}
            Some(mapped_handle) => {
                return Err(NetIdentityMutationError::CurrentIndexPointsElsewhere {
                    net_id: old_id,
                    handle,
                    mapped_handle,
                });
            }
            None => {
                return Err(NetIdentityMutationError::CurrentIdentityIndexMissing {
                    net_id: old_id,
                    handle,
                });
            }
        }
    }

    // All fallible identity checks happen before the legacy mutation. Under the
    // current frozen `PhysicsWorld::set_net_id` implementation, no further
    // rejection point remains once this preflight succeeds.
    world.set_net_id(handle, net_id);

    Ok(())
}

/// Insert a deterministic batch only after validating the *entire* requested
/// identity set against both the batch and the existing world.
///
/// This removes the known expected partial-insertion failure mode of the legacy
/// `add_bodies_deterministic`: duplicate/existing IDs are rejected before its
/// insertion loop begins.
pub fn add_bodies_deterministic_checked<const D: usize>(
    world: &mut PhysicsWorld<D>,
    bodies: Vec<(NetId, RigidBody<D>)>,
) -> Result<Vec<BodyHandle>, NetIdentityMutationError> {
    let mut seen = BTreeSet::new();

    for (net_id, body) in &bodies {
        if !seen.insert(*net_id) {
            return Err(NetIdentityMutationError::DuplicateBatchNetId { net_id: *net_id });
        }

        if let Some(embedded) = body.net_id
            && embedded != *net_id
        {
            return Err(NetIdentityMutationError::BodyCarriesConflictingNetId {
                requested: *net_id,
                embedded,
            });
        }

        let live_handles = live_handles_for_net_id(world, *net_id);
        if live_handles.len() > 1 {
            return Err(NetIdentityMutationError::AmbiguousLiveNetId { net_id: *net_id });
        }
        if let Some(existing_handle) = live_handles.first().copied() {
            return Err(NetIdentityMutationError::NetIdAlreadyBound {
                net_id: *net_id,
                existing_handle,
            });
        }
        if let Some(mapped_handle) = world.handle_for_net_id(*net_id) {
            return Err(NetIdentityMutationError::TargetIndexWithoutBody {
                net_id: *net_id,
                mapped_handle,
            });
        }
    }

    world
        .add_bodies_deterministic(bodies)
        .map_err(NetIdentityMutationError::LegacyInsertionRejected)
}

fn live_handles_for_net_id<const D: usize>(
    world: &PhysicsWorld<D>,
    net_id: NetId,
) -> Vec<BodyHandle> {
    world
        .bodies
        .iter()
        .filter(|body| body.net_id == Some(net_id))
        .map(|body| body.handle)
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetIdentityMutationError {
    UnknownHandle {
        handle: BodyHandle,
    },
    DuplicateBatchNetId {
        net_id: NetId,
    },
    BodyCarriesConflictingNetId {
        requested: NetId,
        embedded: NetId,
    },
    AmbiguousLiveNetId {
        net_id: NetId,
    },
    NetIdAlreadyBound {
        net_id: NetId,
        existing_handle: BodyHandle,
    },
    TargetIndexOccupied {
        net_id: NetId,
        mapped_handle: BodyHandle,
    },
    TargetIndexWithoutBody {
        net_id: NetId,
        mapped_handle: BodyHandle,
    },
    CurrentBodyIdentityMismatch {
        handle: BodyHandle,
        net_id: NetId,
    },
    CurrentIdentityIndexMissing {
        net_id: NetId,
        handle: BodyHandle,
    },
    CurrentIndexPointsElsewhere {
        net_id: NetId,
        handle: BodyHandle,
        mapped_handle: BodyHandle,
    },
    CurrentIdentityIndexMismatch {
        handle: BodyHandle,
        net_id: NetId,
    },
    LegacyInsertionRejected(String),
}
