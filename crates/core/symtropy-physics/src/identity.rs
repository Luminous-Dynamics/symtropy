// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Network-identity mutation errors for [`crate::PhysicsWorld`].
//!
//! A networked physics body must participate in a one-to-one mapping between
//! [`NetId`] and [`BodyHandle`]. Mutation APIs use these errors to reject
//! operations before world state is changed, so a failed identity mutation is
//! transactional rather than partially committed.

use std::collections::BTreeSet;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::{BodyHandle, NetId};

/// Identifies one authoritative physics-world lineage.
///
/// The numeric value is unique for worlds created in this process. Callers
/// that need deterministic reconstruction can supply an explicit value with
/// PhysicsWorld::new_with_generation. This is separate from NetId: a NetId
/// is only non-reusable within one generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldGenerationId(pub u64);

static NEXT_WORLD_GENERATION: AtomicU64 = AtomicU64::new(1);

impl WorldGenerationId {
    /// Allocate a fresh process-local generation for a newly created world.
    pub(crate) fn fresh() -> Self {
        loop {
            let current = NEXT_WORLD_GENERATION.load(Ordering::Relaxed);
            let next = current.saturating_add(1);
            if NEXT_WORLD_GENERATION
                .compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                return Self(current);
            }
        }
    }

    /// Construct an explicit generation for deterministic import/replay setup.
    ///
    /// Explicit generation values are caller-owned lineage identifiers. The
    /// process-local allocator is advanced past this value so a later default
    /// world cannot accidentally receive the same generation.
    pub fn new(value: u64) -> Self {
        let next = value.saturating_add(1);
        let mut current = NEXT_WORLD_GENERATION.load(Ordering::Relaxed);
        while current < next {
            match NEXT_WORLD_GENERATION.compare_exchange_weak(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(observed) => current = observed,
            }
        }
        Self(value)
    }
}

/// A rejected mutation of the `NetId <-> BodyHandle` identity relation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentityMutationError {
    /// The requested body handle is not present in the physics world.
    UnknownBody { handle: BodyHandle },
    /// One deterministic insertion batch contains the same `NetId` more than once.
    DuplicateNetIdInBatch { net_id: NetId },
    /// The requested `NetId` was retired by a successful removal in this world generation.
    NetIdRetired { net_id: NetId },
    /// The requested `NetId` already belongs to another body in the world.
    NetIdAlreadyAssigned { net_id: NetId, owner: BodyHandle },
}

impl fmt::Display for IdentityMutationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownBody { handle } => write!(f, "body handle {} does not exist", handle.0),
            Self::DuplicateNetIdInBatch { net_id } => {
                write!(f, "NetId({}) appears more than once in the batch", net_id.0)
            }
            Self::NetIdRetired { net_id } => {
                write!(
                    f,
                    "NetId({}) was retired in this world generation",
                    net_id.0
                )
            }
            Self::NetIdAlreadyAssigned { net_id, owner } => write!(
                f,
                "NetId({}) is already assigned to body handle {}",
                net_id.0, owner.0
            ),
        }
    }
}

impl std::error::Error for IdentityMutationError {}

/// Validate every incoming `NetId` before a deterministic batch mutates the world.
///
/// Duplicate IDs inside the incoming batch are reported before conflicts with
/// existing world ownership. Retirement is checked before ownership as well,
/// so a retired identity cannot be rebound even if stale ownership metadata
/// is present.
pub(crate) fn validate_batch_net_ids<I, F, R>(
    ids: I,
    mut owner_for: F,
    mut retired: R,
) -> Result<(), IdentityMutationError>
where
    I: IntoIterator<Item = NetId>,
    F: FnMut(NetId) -> Option<BodyHandle>,
    R: FnMut(NetId) -> bool,
{
    let ids: Vec<NetId> = ids.into_iter().collect();
    let mut seen = BTreeSet::new();

    for net_id in &ids {
        if !seen.insert(*net_id) {
            return Err(IdentityMutationError::DuplicateNetIdInBatch { net_id: *net_id });
        }
    }

    for net_id in ids {
        if retired(net_id) {
            return Err(IdentityMutationError::NetIdRetired { net_id });
        }
        if let Some(owner) = owner_for(net_id) {
            return Err(IdentityMutationError::NetIdAlreadyAssigned { net_id, owner });
        }
    }

    Ok(())
}

/// Validate one body-to-NetId assignment before changing either lookup direction.
///
/// Reassigning the same ID to its current owner is valid and therefore
/// idempotent. A retired ID is never assignable to a different or new body.
pub(crate) fn validate_net_id_assignment(
    handle: BodyHandle,
    body_exists: bool,
    net_id: NetId,
    current_owner: Option<BodyHandle>,
    retired: bool,
) -> Result<(), IdentityMutationError> {
    if !body_exists {
        return Err(IdentityMutationError::UnknownBody { handle });
    }

    if retired && current_owner != Some(handle) {
        return Err(IdentityMutationError::NetIdRetired { net_id });
    }

    if let Some(owner) = current_owner
        && owner != handle
    {
        return Err(IdentityMutationError::NetIdAlreadyAssigned { net_id, owner });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_are_structurally_comparable() {
        assert_eq!(
            IdentityMutationError::DuplicateNetIdInBatch { net_id: NetId(7) },
            IdentityMutationError::DuplicateNetIdInBatch { net_id: NetId(7) }
        );
        assert_eq!(
            IdentityMutationError::NetIdRetired { net_id: NetId(9) },
            IdentityMutationError::NetIdRetired { net_id: NetId(9) }
        );
    }

    #[test]
    fn batch_duplicate_is_rejected_before_existing_owner_lookup() {
        let mut lookups = 0usize;
        let result = validate_batch_net_ids(
            [NetId(7), NetId(7)],
            |_| {
                lookups += 1;
                Some(BodyHandle(9))
            },
            |_| false,
        );

        assert_eq!(
            result,
            Err(IdentityMutationError::DuplicateNetIdInBatch { net_id: NetId(7) })
        );
        assert_eq!(lookups, 0, "batch duplicates should fail in the first pass");
    }

    #[test]
    fn batch_existing_owner_is_rejected_after_internal_uniqueness_passes() {
        let result = validate_batch_net_ids(
            [NetId(7), NetId(8)],
            |net_id| (net_id == NetId(8)).then_some(BodyHandle(3)),
            |_| false,
        );

        assert_eq!(
            result,
            Err(IdentityMutationError::NetIdAlreadyAssigned {
                net_id: NetId(8),
                owner: BodyHandle(3),
            })
        );
    }

    #[test]
    fn retired_id_is_rejected_for_batch() {
        assert_eq!(
            validate_batch_net_ids([NetId(9)], |_| None, |_| true),
            Err(IdentityMutationError::NetIdRetired { net_id: NetId(9) })
        );
    }

    #[test]
    fn assignment_requires_existing_body() {
        assert_eq!(
            validate_net_id_assignment(BodyHandle(4), false, NetId(9), None, false),
            Err(IdentityMutationError::UnknownBody {
                handle: BodyHandle(4),
            })
        );
    }

    #[test]
    fn assignment_to_same_owner_is_idempotent() {
        assert_eq!(
            validate_net_id_assignment(BodyHandle(4), true, NetId(9), Some(BodyHandle(4)), false),
            Ok(())
        );
    }

    #[test]
    fn assignment_cannot_displace_another_owner() {
        assert_eq!(
            validate_net_id_assignment(BodyHandle(4), true, NetId(9), Some(BodyHandle(5)), false),
            Err(IdentityMutationError::NetIdAlreadyAssigned {
                net_id: NetId(9),
                owner: BodyHandle(5),
            })
        );
    }

    #[test]
    fn retired_id_is_rejected_for_assignment() {
        assert_eq!(
            validate_net_id_assignment(BodyHandle(4), true, NetId(9), None, true),
            Err(IdentityMutationError::NetIdRetired { net_id: NetId(9) })
        );
    }
}
