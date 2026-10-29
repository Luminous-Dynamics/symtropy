// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Authority interface for restart-safe controller generations.
//!
//! The store trait is deliberately a contract, not a persistence implementation.
//! A production implementation must provide durable, linearizable compare-and-
//! advance semantics across processes and host restarts. It must never restore
//! an old generation after rollback/failover, and must fail closed if it cannot
//! establish the current generation.
//!
//! Checking that an epoch is current is a point-in-time read. The actual command
//! sink must enforce the fencing token atomically with command acceptance (or
//! have a single authoritative owner that serializes those operations); a local
//! preflight read alone cannot eliminate a check/use race.

use crate::epoch_fence::ControllerEpoch;

/// Stable failures returned by a controller-generation store.
///
/// A persistent implementation should map storage-specific conditions into
/// these fail-closed classes instead of converting missing/corrupt state into a
/// fresh generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EpochAuthorityStoreError {
    Unavailable,
    MissingController,
    CompareAndAdvanceConflict {
        expected_generation: u64,
        observed_generation: Option<u64>,
    },
    CorruptState,
    PersistenceFailure,
}

/// Store contract required by the controller epoch authority.
///
/// `compare_and_advance` must atomically compare the durable current generation
/// with `expected_generation`, durably commit exactly expected + 1, and only
/// then return the new value. It must reject stale contenders. Implementations
/// must not use a process-local counter as the authoritative generation.
///
/// The trait cannot prove its implementation is durable or linearizable; each
/// concrete backend requires its own fault-injection and recovery qualification.
pub trait ControllerEpochStore: Send + Sync {
    fn read_generation(&self, controller_id: &str) -> Result<u64, EpochAuthorityStoreError>;

    fn compare_and_advance(
        &self,
        controller_id: &str,
        expected_generation: u64,
    ) -> Result<u64, EpochAuthorityStoreError>;
}

/// Fail-closed errors raised by the authority wrapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EpochAuthorityError {
    EmptyControllerId,
    InvalidStoredGeneration {
        generation: u64,
    },
    GenerationOverflow,
    UnexpectedAdvanceResult {
        expected_generation: u64,
        returned_generation: u64,
    },
    SupersededDuringActivation {
        claimed_generation: u64,
        observed_generation: u64,
    },
    StaleEpoch {
        expected_current_generation: u64,
        supplied_generation: u64,
    },
    Store(EpochAuthorityStoreError),
}

/// Binds one controller identity to a durable epoch-store contract.
///
/// This wrapper contains no local fallback counter and no "start at one" path.
/// An uninitialized, unavailable, corrupt, or rolled-back store is an error,
/// never permission to invent a generation.
pub struct ControllerEpochAuthority<S> {
    controller_id: String,
    store: S,
}

impl<S: ControllerEpochStore> ControllerEpochAuthority<S> {
    pub fn new(
        controller_id: impl Into<String>,
        store: S,
    ) -> Result<Self, EpochAuthorityError> {
        let controller_id = controller_id.into();
        if controller_id.trim().is_empty() {
            return Err(EpochAuthorityError::EmptyControllerId);
        }
        Ok(Self {
            controller_id,
            store,
        })
    }

    pub fn controller_id(&self) -> &str {
        &self.controller_id
    }

    /// Read the authoritative current generation.
    pub fn current_epoch(&self) -> Result<ControllerEpoch, EpochAuthorityError> {
        self.read_valid_generation()
    }

    /// Atomically claim the next generation from the store.
    ///
    /// A successful result means the configured store claims it durably advanced
    /// by exactly one. A read-after-advance check detects immediate supersession.
    /// The caller must still ensure that native command acceptance enforces the
    /// generation atomically; this method does not stop an older controller.
    pub fn claim_next_epoch(&self) -> Result<ControllerEpoch, EpochAuthorityError> {
        let current = self.read_valid_generation()?;
        let expected_next = current
            .generation()
            .checked_add(1)
            .ok_or(EpochAuthorityError::GenerationOverflow)?;
        let next = ControllerEpoch::new(expected_next)
            .map_err(|_| EpochAuthorityError::InvalidStoredGeneration {
                generation: expected_next,
            })?;

        let returned = self
            .store
            .compare_and_advance(&self.controller_id, current.generation())
            .map_err(EpochAuthorityError::Store)?;
        if returned != expected_next {
            return Err(EpochAuthorityError::UnexpectedAdvanceResult {
                expected_generation: expected_next,
                returned_generation: returned,
            });
        }

        let observed = self.read_valid_generation()?;
        if observed != next {
            return Err(EpochAuthorityError::SupersededDuringActivation {
                claimed_generation: next.generation(),
                observed_generation: observed.generation(),
            });
        }
        Ok(next)
    }

    /// Fail closed unless the supplied epoch is current at the time of the read.
    ///
    /// This is not a substitute for the command sink checking the fencing token
    /// atomically when it accepts a command.
    pub fn ensure_current(
        &self,
        supplied: ControllerEpoch,
    ) -> Result<(), EpochAuthorityError> {
        let current = self.read_valid_generation()?;
        if current != supplied {
            return Err(EpochAuthorityError::StaleEpoch {
                expected_current_generation: current.generation(),
                supplied_generation: supplied.generation(),
            });
        }
        Ok(())
    }

    pub fn store(&self) -> &S {
        &self.store
    }

    fn read_valid_generation(&self) -> Result<ControllerEpoch, EpochAuthorityError> {
        let generation = self
            .store
            .read_generation(&self.controller_id)
            .map_err(EpochAuthorityError::Store)?;
        ControllerEpoch::new(generation).map_err(|_| {
            EpochAuthorityError::InvalidStoredGeneration { generation }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct TestStore {
        generations: Mutex<BTreeMap<String, u64>>,
        unavailable: AtomicBool,
        corrupt_read: AtomicBool,
        wrong_advance_result: AtomicBool,
        supersede_after_advance: AtomicBool,
    }

    impl TestStore {
        fn with_generation(controller_id: &str, generation: u64) -> Arc<Self> {
            let mut generations = BTreeMap::new();
            generations.insert(controller_id.to_owned(), generation);
            Arc::new(Self {
                generations: Mutex::new(generations),
                unavailable: AtomicBool::new(false),
                corrupt_read: AtomicBool::new(false),
                wrong_advance_result: AtomicBool::new(false),
                supersede_after_advance: AtomicBool::new(false),
            })
        }

        fn set_generation(&self, controller_id: &str, generation: u64) {
            self.generations
                .lock()
                .unwrap()
                .insert(controller_id.to_owned(), generation);
        }

        fn set_unavailable(&self, value: bool) {
            self.unavailable.store(value, Ordering::SeqCst);
        }
    }

    impl ControllerEpochStore for TestStore {
        fn read_generation(&self, controller_id: &str) -> Result<u64, EpochAuthorityStoreError> {
            if self.unavailable.load(Ordering::SeqCst) {
                return Err(EpochAuthorityStoreError::Unavailable);
            }
            if self.corrupt_read.load(Ordering::SeqCst) {
                return Ok(0);
            }
            self.generations
                .lock()
                .unwrap()
                .get(controller_id)
                .copied()
                .ok_or(EpochAuthorityStoreError::MissingController)
        }

        fn compare_and_advance(
            &self,
            controller_id: &str,
            expected_generation: u64,
        ) -> Result<u64, EpochAuthorityStoreError> {
            if self.unavailable.load(Ordering::SeqCst) {
                return Err(EpochAuthorityStoreError::Unavailable);
            }

            let mut generations = self.generations.lock().unwrap();
            let observed = generations.get(controller_id).copied();
            if observed != Some(expected_generation) {
                return Err(EpochAuthorityStoreError::CompareAndAdvanceConflict {
                    expected_generation,
                    observed_generation: observed,
                });
            }
            let next = expected_generation.checked_add(1).ok_or(
                EpochAuthorityStoreError::CorruptState,
            )?;
            generations.insert(controller_id.to_owned(), next);

            if self.supersede_after_advance.swap(false, Ordering::SeqCst) {
                let superseding = next
                    .checked_add(1)
                    .ok_or(EpochAuthorityStoreError::CorruptState)?;
                generations.insert(controller_id.to_owned(), superseding);
            }
            if self.wrong_advance_result.swap(false, Ordering::SeqCst) {
                return Ok(expected_generation);
            }
            Ok(next)
        }
    }

    fn authority(
        store: Arc<TestStore>,
    ) -> ControllerEpochAuthority<Arc<TestStore>> {
        ControllerEpochAuthority::new("left-wheel-controller", store).unwrap()
    }

    fn epoch(generation: u64) -> ControllerEpoch {
        ControllerEpoch::new(generation).unwrap()
    }

    #[test]
    fn controller_identity_must_be_nonempty() {
        let store = TestStore::with_generation("controller", 1);
        assert_eq!(
            ControllerEpochAuthority::new("  ", store).err(),
            Some(EpochAuthorityError::EmptyControllerId)
        );
    }

    #[test]
    fn claims_exactly_next_persisted_generation() {
        let store = TestStore::with_generation("left-wheel-controller", 41);
        let authority = authority(store.clone());

        assert_eq!(authority.current_epoch(), Ok(epoch(41)));
        assert_eq!(authority.claim_next_epoch(), Ok(epoch(42)));
        assert_eq!(authority.current_epoch(), Ok(epoch(42)));
    }

    #[test]
    fn missing_controller_state_never_falls_back_to_generation_one() {
        let store = TestStore::with_generation("different-controller", 9);
        let authority = authority(store);
        assert_eq!(
            authority.current_epoch(),
            Err(EpochAuthorityError::Store(
                EpochAuthorityStoreError::MissingController
            ))
        );
        assert_eq!(
            authority.claim_next_epoch(),
            Err(EpochAuthorityError::Store(
                EpochAuthorityStoreError::MissingController
            ))
        );
    }

    #[test]
    fn unavailable_authority_fails_closed_for_reads_and_advances() {
        let store = TestStore::with_generation("left-wheel-controller", 2);
        store.set_unavailable(true);
        let authority = authority(store);

        assert_eq!(
            authority.current_epoch(),
            Err(EpochAuthorityError::Store(
                EpochAuthorityStoreError::Unavailable
            ))
        );
        assert_eq!(
            authority.claim_next_epoch(),
            Err(EpochAuthorityError::Store(
                EpochAuthorityStoreError::Unavailable
            ))
        );
    }

    #[test]
    fn corrupt_zero_generation_fails_closed() {
        let store = TestStore::with_generation("left-wheel-controller", 2);
        store.corrupt_read.store(true, Ordering::SeqCst);
        let authority = authority(store);
        assert_eq!(
            authority.current_epoch(),
            Err(EpochAuthorityError::InvalidStoredGeneration { generation: 0 })
        );
    }

    #[test]
    fn unexpected_compare_and_advance_result_is_rejected() {
        let store = TestStore::with_generation("left-wheel-controller", 2);
        store.wrong_advance_result.store(true, Ordering::SeqCst);
        let authority = authority(store);
        assert_eq!(
            authority.claim_next_epoch(),
            Err(EpochAuthorityError::UnexpectedAdvanceResult {
                expected_generation: 3,
                returned_generation: 2,
            })
        );
    }

    #[test]
    fn activation_detects_being_superseded_by_another_owner() {
        let store = TestStore::with_generation("left-wheel-controller", 2);
        store.supersede_after_advance.store(true, Ordering::SeqCst);
        let authority = authority(store);
        assert_eq!(
            authority.claim_next_epoch(),
            Err(EpochAuthorityError::SupersededDuringActivation {
                claimed_generation: 3,
                observed_generation: 4,
            })
        );
    }

    #[test]
    fn stale_epoch_is_rejected_after_store_advances() {
        let store = TestStore::with_generation("left-wheel-controller", 5);
        let authority = authority(store.clone());
        let old_epoch = authority.current_epoch().unwrap();

        assert_eq!(authority.claim_next_epoch(), Ok(epoch(6)));
        assert_eq!(
            authority.ensure_current(old_epoch),
            Err(EpochAuthorityError::StaleEpoch {
                expected_current_generation: 6,
                supplied_generation: 5,
            })
        );
        assert_eq!(authority.ensure_current(epoch(6)), Ok(()));
    }

    #[test]
    fn generation_cannot_wrap_at_u64_max() {
        let store = TestStore::with_generation("left-wheel-controller", u64::MAX);
        let authority = authority(store.clone());
        assert_eq!(
            authority.claim_next_epoch(),
            Err(EpochAuthorityError::GenerationOverflow)
        );
        assert_eq!(authority.current_epoch(), Ok(epoch(u64::MAX)));
    }

    #[test]
    fn external_rollback_is_seen_as_a_changed_generation_not_accepted_as_current() {
        let store = TestStore::with_generation("left-wheel-controller", 8);
        let authority = authority(store.clone());
        let old = authority.current_epoch().unwrap();
        assert_eq!(authority.claim_next_epoch(), Ok(epoch(9)));

        // Simulates a faulty backend restored from an old snapshot. This module
        // can detect the mismatch with its retained token, but cannot prevent
        // the backend itself from rolling back; production qualification must.
        store.set_generation("left-wheel-controller", 8);
        assert_eq!(
            authority.ensure_current(epoch(9)),
            Err(EpochAuthorityError::StaleEpoch {
                expected_current_generation: 8,
                supplied_generation: 9,
            })
        );
        assert_eq!(
            authority.ensure_current(old),
            Err(EpochAuthorityError::StaleEpoch {
                expected_current_generation: 8,
                supplied_generation: 8,
            })
        );
    }

    #[test]
    fn two_authorities_sharing_one_store_observe_one_monotonic_sequence() {
        let store = TestStore::with_generation("left-wheel-controller", 10);
        let first = authority(store.clone());
        let second = authority(store);

        let first_epoch = first.claim_next_epoch().unwrap();
        let second_epoch = second.claim_next_epoch().unwrap();
        assert_eq!(first_epoch, epoch(11));
        assert_eq!(second_epoch, epoch(12));
        assert_eq!(first.ensure_current(first_epoch), Err(EpochAuthorityError::StaleEpoch {
            expected_current_generation: 12,
            supplied_generation: 11,
        }));
        assert_eq!(second.ensure_current(second_epoch), Ok(()));
    }
}
