// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Deterministic identity-integrity preflight for networked physics bodies.
//!
//! `NetId` is the stable network/replay identity while `BodyHandle` is the
//! process-local physics handle. Mutating either relation must preserve a
//! one-to-one mapping. These helpers perform validation without mutating the
//! world so callers can keep batch and reassignment operations transactional.

use std::collections::{BTreeMap, BTreeSet};

use crate::body::{BodyHandle, NetId};

/// Rejected mutation of the `NetId <-> BodyHandle` identity relation.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum NetIdMutationError {
    /// The incoming batch contains the same stable ID more than once.
    DuplicateInBatch(NetId),
    /// The requested stable ID already belongs to another body.
    AlreadyAssigned {
        net_id: NetId,
        existing_handle: BodyHandle,
    },
    /// A mutation targeted a body handle that does not exist in the world.
    UnknownHandle(BodyHandle),
}

/// Validate a complete deterministic insertion batch before any world mutation.
///
/// The incoming IDs are copied into a sorted set so the reported conflict is
/// deterministic even when the caller supplied a different iteration order.
/// No handles are allocated and no mappings are changed by this function.
pub fn preflight_net_id_batch<I>(
    existing: &BTreeMap<NetId, BodyHandle>,
    incoming: I,
) -> Result<(), NetIdMutationError>
where
    I: IntoIterator<Item = NetId>,
{
    let mut ordered = incoming.into_iter().collect::<Vec<_>>();
    ordered.sort_unstable();

    let mut seen = BTreeSet::new();
    for net_id in ordered {
        if !seen.insert(net_id) {
            return Err(NetIdMutationError::DuplicateInBatch(net_id));
        }
        if let Some(&existing_handle) = existing.get(&net_id) {
            return Err(NetIdMutationError::AlreadyAssigned {
                net_id,
                existing_handle,
            });
        }
    }

    Ok(())
}

/// Validate assigning `net_id` to `target_handle` without mutating the map.
///
/// Reassigning the same ID to the same handle is intentionally accepted as an
/// idempotent operation. Assigning an ID owned by a different handle is rejected
/// before the caller removes the target's previous mapping.
pub fn preflight_net_id_assignment(
    existing: &BTreeMap<NetId, BodyHandle>,
    target_handle: BodyHandle,
    net_id: NetId,
) -> Result<(), NetIdMutationError> {
    match existing.get(&net_id).copied() {
        Some(existing_handle) if existing_handle != target_handle => {
            Err(NetIdMutationError::AlreadyAssigned {
                net_id,
                existing_handle,
            })
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_duplicate_is_rejected_deterministically() {
        let existing = BTreeMap::new();

        assert_eq!(
            preflight_net_id_batch(
                &existing,
                [NetId(9), NetId(3), NetId(9), NetId(3)]
            ),
            Err(NetIdMutationError::DuplicateInBatch(NetId(3)))
        );
        assert!(existing.is_empty(), "preflight must not mutate identity state");
    }

    #[test]
    fn batch_conflict_with_existing_reports_current_owner() {
        let existing = BTreeMap::from([
            (NetId(2), BodyHandle(20)),
            (NetId(8), BodyHandle(80)),
        ]);

        assert_eq!(
            preflight_net_id_batch(&existing, [NetId(10), NetId(8), NetId(4)]),
            Err(NetIdMutationError::AlreadyAssigned {
                net_id: NetId(8),
                existing_handle: BodyHandle(80),
            })
        );
        assert_eq!(existing.len(), 2, "preflight must be transactional");
    }

    #[test]
    fn valid_batch_is_accepted_without_mutation() {
        let existing = BTreeMap::from([(NetId(2), BodyHandle(20))]);

        assert_eq!(
            preflight_net_id_batch(&existing, [NetId(7), NetId(3), NetId(5)]),
            Ok(())
        );
        assert_eq!(existing, BTreeMap::from([(NetId(2), BodyHandle(20))]));
    }

    #[test]
    fn assignment_to_same_owner_is_idempotent() {
        let existing = BTreeMap::from([(NetId(7), BodyHandle(4))]);

        assert_eq!(
            preflight_net_id_assignment(&existing, BodyHandle(4), NetId(7)),
            Ok(())
        );
    }

    #[test]
    fn assignment_cannot_steal_another_bodys_id() {
        let existing = BTreeMap::from([(NetId(7), BodyHandle(4))]);

        assert_eq!(
            preflight_net_id_assignment(&existing, BodyHandle(9), NetId(7)),
            Err(NetIdMutationError::AlreadyAssigned {
                net_id: NetId(7),
                existing_handle: BodyHandle(4),
            })
        );
        assert_eq!(existing.get(&NetId(7)), Some(&BodyHandle(4)));
    }
}
