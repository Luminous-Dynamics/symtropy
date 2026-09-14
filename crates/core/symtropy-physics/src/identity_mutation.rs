// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Checked mutation coordinators for physics network identity.
//!
//! This module owns the low-level, whole-operation preflight mechanics used by
//! [`crate::PhysicsAuthorityWorld`]. The coordinators are crate-internal so
//! downstream safe Rust cannot mutate stable physical identity through a raw
//! authority-free `PhysicsWorld` helper.

use std::collections::BTreeSet;

use crate::body::{BodyHandle, NetId, RigidBody};
use crate::world::PhysicsWorld;

/// Prepared result of validating one stable-identity bind.
///
/// Preparation is pure with respect to the world. A state-changing plan is
/// committed only after the authority layer has broken the previous temporal
/// lineage and entered its mutation-commit taint boundary.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum PreparedNetIdBinding {
    NoChange,
    Change { handle: BodyHandle, net_id: NetId },
}

/// Validate one stable-identity bind without mutating the world.
///
/// Stable identity is bind-once per body incarnation: `None -> N` is allowed,
/// `N -> N` is idempotent, and `N -> M` is rejected. Identity replacement
/// belongs to an explicit future continuity/reincarnation authority.
pub(crate) fn prepare_net_id_binding<const D: usize>(
    world: &PhysicsWorld<D>,
    handle: BodyHandle,
    net_id: NetId,
) -> Result<PreparedNetIdBinding, NetIdentityMutationError> {
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
        return Ok(PreparedNetIdBinding::NoChange);
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

    Ok(PreparedNetIdBinding::Change { handle, net_id })
}

/// Apply an already-prepared stable-identity bind.
///
/// No ordinary recoverable validation branch remains here. If a future change
/// introduces one, it must move back into [`prepare_net_id_binding`] so callers
/// can reject it before entering the authority mutation boundary.
pub(crate) fn commit_prepared_net_id_binding<const D: usize>(
    world: &mut PhysicsWorld<D>,
    prepared: PreparedNetIdBinding,
) {
    if let PreparedNetIdBinding::Change { handle, net_id } = prepared {
        world.set_net_id(handle, net_id);
    }
}

/// Prepared deterministic insertion after whole-request identity preflight.
pub(crate) struct PreparedDeterministicInsertion<const D: usize> {
    bodies: Vec<(NetId, RigidBody<D>)>,
}

impl<const D: usize> PreparedDeterministicInsertion<D> {
    pub(crate) fn is_empty(&self) -> bool {
        self.bodies.is_empty()
    }
}

/// Validate the entire requested identity set without mutating the world.
///
/// This removes the known expected partial-insertion failure mode of the legacy
/// `add_bodies_deterministic`: duplicate/existing IDs are rejected before its
/// insertion loop begins. Handle-allocation capacity and low-level structural
/// atomicity remain PHYS-ID-01E / #1060 responsibilities.
pub(crate) fn prepare_deterministic_insertion<const D: usize>(
    world: &PhysicsWorld<D>,
    bodies: Vec<(NetId, RigidBody<D>)>,
) -> Result<PreparedDeterministicInsertion<D>, NetIdentityMutationError> {
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

    Ok(PreparedDeterministicInsertion { bodies })
}

/// Apply an already-prepared deterministic insertion.
///
/// The legacy world insertion still owns handle allocation until #1060. Any
/// unexpected rejection here indicates that the prepare/commit contract has
/// been violated. Authority callers execute this function inside a mutation
/// taint boundary, so a panic cannot preserve the old temporal lineage.
pub(crate) fn commit_prepared_deterministic_insertion<const D: usize>(
    world: &mut PhysicsWorld<D>,
    prepared: PreparedDeterministicInsertion<D>,
) -> Vec<BodyHandle> {
    world
        .add_bodies_deterministic(prepared.bodies)
        .unwrap_or_else(|error| {
            panic!("prepared deterministic insertion unexpectedly rejected: {error}")
        })
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
    /// Retained for public compatibility with the pre-PHYS-OBS-04B checked
    /// coordinator. Prepared authority commits no longer return this variant;
    /// an unexpected legacy rejection occurs inside the mutation taint boundary.
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
    fn stale_body_identity_rejects_prepare_without_more_mutation() {
        let mut world = PhysicsWorld::<3>::default();
        let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
        let indexed = NetId(110);
        let stale_body_id = NetId(111);
        let requested = NetId(112);

        world.set_net_id(handle, indexed);
        world.body_mut(handle).expect("body exists").net_id = Some(stale_body_id);

        assert!(matches!(
            prepare_net_id_binding(&world, handle, requested),
            Err(NetIdentityMutationError::CurrentIdentityIndexMissing {
                net_id,
                handle: observed_handle,
            }) if net_id == stale_body_id && observed_handle == handle
        ));
        assert_eq!(
            world.body(handle).expect("body exists").net_id(),
            Some(stale_body_id)
        );
        assert_eq!(world.handle_for_net_id(indexed), Some(handle));
        assert_eq!(world.handle_for_net_id(requested), None);
    }

    #[test]
    fn conflicting_embedded_identity_rejects_prepare_before_batch_insertion() {
        let world = PhysicsWorld::<3>::default();
        let requested = NetId(140);
        let embedded = NetId(141);
        let mut body = body_at(0.0);
        body.net_id = Some(embedded);

        assert!(matches!(
            prepare_deterministic_insertion(&world, vec![(requested, body)]),
            Err(NetIdentityMutationError::BodyCarriesConflictingNetId {
                requested: observed_requested,
                embedded: observed_embedded,
            }) if observed_requested == requested && observed_embedded == embedded
        ));
        assert_eq!(world.body_count(), 0);
        assert_eq!(world.handle_for_net_id(requested), None);
        assert_eq!(world.handle_for_net_id(embedded), None);
    }

    #[test]
    fn prepared_identity_change_mutates_only_during_commit() {
        let mut world = PhysicsWorld::<3>::default();
        let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
        let net_id = NetId(150);

        let prepared = prepare_net_id_binding(&world, handle, net_id).unwrap();
        assert_eq!(world.net_id_for_handle(handle), None);

        commit_prepared_net_id_binding(&mut world, prepared);
        assert_eq!(world.net_id_for_handle(handle), Some(net_id));
        assert_eq!(world.handle_for_net_id(net_id), Some(handle));
    }
}
