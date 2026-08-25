// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Network-identity mutation errors for [`crate::PhysicsWorld`].
//!
//! A networked physics body must participate in a one-to-one mapping between
//! [`NetId`] and [`BodyHandle`]. Mutation APIs use these errors to reject
//! operations before world state is changed, so a failed identity mutation is
//! transactional rather than partially committed.

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
}
