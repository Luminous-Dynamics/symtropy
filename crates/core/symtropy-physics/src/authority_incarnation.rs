// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Allocator-instance-local authority-owned temporal-incarnation identity.
//!
//! The identifier in this module distinguishes independently constructed live
//! `PhysicsAuthorityWorld` wrappers that share this exact loaded allocator.
//! It deliberately makes no cross-allocator, cross-process, persistent,
//! cryptographic, or wall-clock identity claim.

use std::sync::Mutex;

/// Opaque identity of one live physics-authority wrapper incarnation within one
/// loaded allocator domain.
///
/// There is no public constructor and no serde authority. Values are minted by
/// the authority wrapper's private allocator and remain stable for the
/// lifetime/moves of that wrapper.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct TemporalIncarnationId(u64);

impl TemporalIncarnationId {
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum TemporalIncarnationAllocationError {
    AllocatorPoisoned,
    Exhausted,
}

// `1` is the first admissible incarnation. This static identifies one allocator
// instance in one loaded copy of this crate. Independently loaded copies may own
// independent statics and therefore are outside this numeric uniqueness domain.
// The allocator conservatively keeps `u64::MAX` as a terminal exhausted sentinel,
// so no successful mint ever needs to wrap the counter or represent a successor
// to MAX.
static NEXT_TEMPORAL_INCARNATION: Mutex<u64> = Mutex::new(1);

pub(crate) fn mint_temporal_incarnation(
) -> Result<TemporalIncarnationId, TemporalIncarnationAllocationError> {
    mint_from(&NEXT_TEMPORAL_INCARNATION)
}

fn mint_from(
    allocator: &Mutex<u64>,
) -> Result<TemporalIncarnationId, TemporalIncarnationAllocationError> {
    let mut next = allocator
        .lock()
        .map_err(|_| TemporalIncarnationAllocationError::AllocatorPoisoned)?;
    let current = *next;
    let successor = current
        .checked_add(1)
        .ok_or(TemporalIncarnationAllocationError::Exhausted)?;
    *next = successor;
    Ok(TemporalIncarnationId(current))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn allocator_never_wraps_at_exhaustion() {
        let allocator = Mutex::new(u64::MAX - 1);

        let final_minted = mint_from(&allocator).unwrap();
        assert_eq!(final_minted.get(), u64::MAX - 1);
        assert_eq!(
            mint_from(&allocator),
            Err(TemporalIncarnationAllocationError::Exhausted)
        );
        assert_eq!(*allocator.lock().unwrap(), u64::MAX);
    }

    #[test]
    fn concurrent_minting_is_unique_within_one_allocator() {
        const COUNT: usize = 64;
        let allocator = Arc::new(Mutex::new(1));
        let mut joins = Vec::with_capacity(COUNT);

        for _ in 0..COUNT {
            let allocator = Arc::clone(&allocator);
            joins.push(thread::spawn(move || mint_from(&allocator).unwrap().get()));
        }

        let mut ids: Vec<_> = joins
            .into_iter()
            .map(|join| join.join().unwrap())
            .collect();
        ids.sort_unstable();
        ids.dedup();

        assert_eq!(ids.len(), COUNT);
        assert_eq!(ids.first().copied(), Some(1));
        assert_eq!(ids.last().copied(), Some(COUNT as u64));
    }
}
