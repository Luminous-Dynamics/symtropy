// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Checked mutation coordinators for physics network identity.
//!
//! This module provides typed, whole-operation preflight wrappers that make the
//! checked path explicit and fail closed before invoking legacy world mutators.
//! Direct `RigidBody::net_id` writes are crate-controlled; the legacy world
//! mutators remain public compatibility debt until PHYS-ID-01B2.

use std::collections::BTreeSet;

use crate::body::{BodyHandle, NetId, RigidBody};
use crate::world::PhysicsWorld;

/// Bind one previously-unbound body to one `NetId` only after proving the
/// current body/index views are mutually consistent and the requested identity
/// is not owned elsewhere.
///
/// Stable identity is bind-once per body incarnation: `None -> N` is allowed,
/// `N -> N` is idempotent, and `N -> M` is rejected. Identity replacement
/// belongs to an explicit future continuity/reincarnation authority.
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
        if target_handles.len() != 1 || target_handles[0] != handle {
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
        if old_handles.len() != 1 || old_handles[0] != handle {
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

        return Err(NetIdentityMutationError::IdentityReassignmentForbidden {
            handle,
            current: old_id,
            requested: net_id,
        });
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
    IdentityReassignmentForbidden {
        handle: BodyHandle,
        current: NetId,
        requested: NetId,
    },
    LegacyInsertionRejected(String),
}

#[cfg(test)]
mod privileged_corruption_tests {
    use super::*;
    use symtropy_math::Point;

    fn body_at(x: f64) -> RigidBody<3> {
        RigidBody::dynamic_sphere(BodyHandle(0), Point::new([x, 0.0, 0.0]), 0.5, 1.0)
    }

    #[test]
    fn stale_body_identity_rejects_checked_reassignment_without_more_mutation() {
        let mut world = PhysicsWorld::<3>::default();
        let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
        let indexed = NetId(110);
        let stale_body_id = NetId(111);
        let requested = NetId(112);

        world.set_net_id(handle, indexed);
        world.body_mut(handle).expect("body exists").net_id = Some(stale_body_id);

        assert_eq!(
            assign_net_id_checked(&mut world, handle, requested),
            Err(NetIdentityMutationError::CurrentIdentityIndexMissing {
                net_id: stale_body_id,
                handle,
            })
        );
        assert_eq!(world.body(handle).expect("body exists").net_id(), Some(stale_body_id));
        assert_eq!(world.handle_for_net_id(indexed), Some(handle));
        assert_eq!(world.handle_for_net_id(requested), None);
    }

    #[test]
    fn conflicting_embedded_identity_rejects_before_batch_insertion() {
        let mut world = PhysicsWorld::<3>::default();
        let requested = NetId(140);
        let embedded = NetId(141);
        let mut body = body_at(0.0);
        body.net_id = Some(embedded);

        assert_eq!(
            add_bodies_deterministic_checked(&mut world, vec![(requested, body)]),
            Err(NetIdentityMutationError::BodyCarriesConflictingNetId {
                requested,
                embedded,
            })
        );
        assert_eq!(world.body_count(), 0);
        assert_eq!(world.handle_for_net_id(requested), None);
        assert_eq!(world.handle_for_net_id(embedded), None);
    }
}
