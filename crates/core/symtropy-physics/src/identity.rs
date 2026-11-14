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

use crate::{BodyHandle, NetId};

/// A rejected mutation of the `NetId <-> BodyHandle` identity relation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentityMutationError {
    /// The requested body handle is not present in the physics world.
    UnknownBody { handle: BodyHandle },
    /// One deterministic insertion batch contains the same `NetId` more than once.
    DuplicateNetIdInBatch { net_id: NetId },
    /// The requested `NetId` already belongs to another body in the world.
    NetIdAlreadyAssigned { net_id: NetId, owner: BodyHandle },
}

impl fmt::Display for IdentityMutationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownBody { handle } => {
                write!(f, "body handle {} does not exist", handle.0)
            }
            Self::DuplicateNetIdInBatch { net_id } => {
                write!(f, "NetId({}) appears more than once in the batch", net_id.0)
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
/// existing world ownership. That ordering keeps rejection deterministic and
/// guarantees callers can preflight the entire batch before allocating handles.
///
/// This helper is crate-private until the `PhysicsWorld` mutation methods are
/// wired to the transactional identity contract.
#[allow(dead_code)]
pub(crate) fn validate_batch_net_ids<I, F>(
    ids: I,
    mut owner_for: F,
) -> Result<(), IdentityMutationError>
where
    I: IntoIterator<Item = NetId>,
    F: FnMut(NetId) -> Option<BodyHandle>,
{
    let ids: Vec<NetId> = ids.into_iter().collect();
    let mut seen = BTreeSet::new();

    for net_id in &ids {
        if !seen.insert(*net_id) {
            return Err(IdentityMutationError::DuplicateNetIdInBatch { net_id: *net_id });
        }
    }

    for net_id in ids {
        if let Some(owner) = owner_for(net_id) {
            return Err(IdentityMutationError::NetIdAlreadyAssigned { net_id, owner });
        }
    }

    Ok(())
}

/// Validate one body-to-NetId assignment before changing either lookup direction.
///
/// Reassigning the same ID to its current owner is valid and therefore
/// idempotent. Assigning an ID owned by a different body is rejected.
///
/// This helper is crate-private until the `PhysicsWorld` mutation methods are
/// wired to the transactional identity contract.
#[allow(dead_code)]
pub(crate) fn validate_net_id_assignment(
    handle: BodyHandle,
    body_exists: bool,
    net_id: NetId,
    current_owner: Option<BodyHandle>,
) -> Result<(), IdentityMutationError> {
    if !body_exists {
        return Err(IdentityMutationError::UnknownBody { handle });
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
        assert_ne!(
            IdentityMutationError::NetIdAlreadyAssigned {
                net_id: NetId(7),
                owner: BodyHandle(1),
            },
            IdentityMutationError::NetIdAlreadyAssigned {
                net_id: NetId(7),
                owner: BodyHandle(2),
            }
        );
    }

    #[test]
    fn batch_duplicate_is_rejected_before_existing_owner_lookup() {
        let mut lookups = 0usize;
        let result = validate_batch_net_ids([NetId(7), NetId(7)], |_| {
            lookups += 1;
            Some(BodyHandle(9))
        });

        assert_eq!(
            result,
            Err(IdentityMutationError::DuplicateNetIdInBatch { net_id: NetId(7) })
        );
        assert_eq!(lookups, 0, "batch duplicates should fail in the first pass");
    }

    #[test]
    fn batch_existing_owner_is_rejected_after_internal_uniqueness_passes() {
        let result = validate_batch_net_ids([NetId(7), NetId(8)], |net_id| {
            (net_id == NetId(8)).then_some(BodyHandle(3))
        });

        assert_eq!(
            result,
            Err(IdentityMutationError::NetIdAlreadyAssigned {
                net_id: NetId(8),
                owner: BodyHandle(3),
            })
        );
    }

    #[test]
    fn assignment_requires_existing_body() {
        assert_eq!(
            validate_net_id_assignment(BodyHandle(4), false, NetId(9), None),
            Err(IdentityMutationError::UnknownBody {
                handle: BodyHandle(4),
            })
        );
    }

    #[test]
    fn assignment_to_same_owner_is_idempotent() {
        assert_eq!(
            validate_net_id_assignment(
                BodyHandle(4),
                true,
                NetId(9),
                Some(BodyHandle(4)),
            ),
            Ok(())
        );
    }

    #[test]
    fn assignment_cannot_displace_another_owner() {
        assert_eq!(
            validate_net_id_assignment(
                BodyHandle(4),
                true,
                NetId(9),
                Some(BodyHandle(5)),
            ),
            Err(IdentityMutationError::NetIdAlreadyAssigned {
                net_id: NetId(9),
                owner: BodyHandle(5),
            })
        );
    }
}
