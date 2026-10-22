// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Explicit, deterministic simulation-tick lifecycle primitives.
//!
//! This module deliberately does not depend on Bevy or perform physics/energy
//! mutations. A caller owns scheduling, canonical state encoding, and digest
//! computation. The state machine only gates legal lifecycle transitions and
//! makes finalization idempotent for the most recently finalized tick.
//!
//! # Contract
//!
//! - Tick IDs are caller-provided monotonic identities, not wall-clock values.
//! - A new tick cannot begin while an earlier tick is unfinished.
//! - Physics may be marked applied only after begin.
//! - Finalization requires the physics-applied phase.
//! - Repeating finalization with the same result digest returns the exact same
//!   receipt. A different digest for the same finalized tick is rejected.
//! - Starting a later tick ends the idempotency window for the prior receipt.
//!
//! A receipt does not prove that a digest is cryptographically sound or that
//! the simulation obeys physical laws. Digests are opaque 32-byte values
//! supplied by the caller; independent verification requires canonical
//! serialization and a separately implemented digest/replay verifier.

/// Monotonic logical identity for one simulation tick.
///
/// The caller must allocate IDs at the simulation boundary. Do not derive a
/// durable tick identity from render-frame time or floating-point elapsed time.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TickId(pub u64);

/// The lifecycle phase currently held by a tick transaction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TickPhase {
    Begun,
    PhysicsApplied,
    Finalized,
}

/// Versioned receipt returned when a tick is finalized.
///
/// The three digests are opaque inputs provided by the caller. This crate does
/// not calculate them and does not claim they are valid hashes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TickReceipt {
    /// Receipt schema version. Increment when the stable receipt shape changes.
    pub schema_version: u16,
    pub tick_id: TickId,
    pub input_digest: [u8; 32],
    pub prior_state_digest: [u8; 32],
    pub result_state_digest: [u8; 32],
}

/// A rejected lifecycle transition. Rejected calls leave transaction state
/// unchanged.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionError {
    NoActiveTick,
    TickAlreadyActive {
        active: TickId,
    },
    WrongTick {
        active: TickId,
        requested: TickId,
    },
    UnexpectedPhase {
        tick_id: TickId,
        expected: TickPhase,
        actual: TickPhase,
    },
    NonMonotonicTick {
        last_finalized: TickId,
        proposed: TickId,
    },
    PriorStateMismatch {
        previous_tick: TickId,
    },
    FinalizationConflict {
        tick_id: TickId,
    },
}

impl core::fmt::Display for TransitionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoActiveTick => write!(f, "no active tick transaction"),
            Self::TickAlreadyActive { active } => {
                write!(f, "tick {} is still active", active.0)
            }
            Self::WrongTick { active, requested } => write!(
                f,
                "active tick is {}, not requested tick {}",
                active.0, requested.0
            ),
            Self::UnexpectedPhase {
                tick_id,
                expected,
                actual,
            } => write!(
                f,
                "tick {} expected phase {:?}, found {:?}",
                tick_id.0, expected, actual
            ),
            Self::NonMonotonicTick {
                last_finalized,
                proposed,
            } => write!(
                f,
                "tick {} is not newer than finalized tick {}",
                proposed.0, last_finalized.0
            ),
            Self::PriorStateMismatch { previous_tick } => write!(
                f,
                "new tick prior-state digest does not match finalized tick {} result",
                previous_tick.0
            ),
            Self::FinalizationConflict { tick_id } => write!(
                f,
                "tick {} was already finalized with a different result digest",
                tick_id.0
            ),
        }
    }
}

impl std::error::Error for TransitionError {}

#[derive(Clone, Copy, Debug)]
struct TickProgress {
    id: TickId,
    phase: TickPhase,
    input_digest: [u8; 32],
    prior_state_digest: [u8; 32],
    receipt: Option<TickReceipt>,
}

/// Fail-closed, single-active-tick transaction state.
///
/// This primitive models lifecycle ordering only. It cannot roll back external
/// side effects after physics has run; callers must keep mutation boundaries
/// explicit and only call `finalize` after their commit boundary is complete.
#[derive(Clone, Debug, Default)]
pub struct TickTransaction {
    current: Option<TickProgress>,
    last_finalized: Option<TickId>,
}

impl TickTransaction {
    /// Begin a tick with caller-computed input and prior-state digests.
    ///
    /// IDs must strictly increase after the first finalized tick. Gaps are
    /// permitted when the prior-state digest still matches the latest receipt;
    /// applications that require contiguous IDs must enforce that policy at
    /// their allocator boundary. To restore from a different checkpoint, start
    /// a new transaction instance after validating that checkpoint explicitly.
    pub fn begin(
        &mut self,
        tick_id: TickId,
        input_digest: [u8; 32],
        prior_state_digest: [u8; 32],
    ) -> Result<(), TransitionError> {
        if let Some(current) = self.current {
            if current.phase != TickPhase::Finalized {
                return Err(TransitionError::TickAlreadyActive { active: current.id });
            }
        }

        if let Some(last_finalized) = self.last_finalized {
            if tick_id <= last_finalized {
                return Err(TransitionError::NonMonotonicTick {
                    last_finalized,
                    proposed: tick_id,
                });
            }
        }

        // Chain continuity is checked separately from monotonic identity. A
        // caller may skip IDs, but may not silently start from a different
        // state without creating a new transaction from an explicit checkpoint.
        if let Some(previous) = self.current {
            if previous.phase == TickPhase::Finalized {
                let previous_receipt = previous
                    .receipt
                    .expect("finalized ticks always carry a receipt");
                if prior_state_digest != previous_receipt.result_state_digest {
                    return Err(TransitionError::PriorStateMismatch {
                        previous_tick: previous.id,
                    });
                }
            }
        }

        self.current = Some(TickProgress {
            id: tick_id,
            phase: TickPhase::Begun,
            input_digest,
            prior_state_digest,
            receipt: None,
        });
        Ok(())
    }

    /// Record that the caller's physics phase completed for this tick.
    pub fn mark_physics_applied(
        &mut self,
        tick_id: TickId,
    ) -> Result<(), TransitionError> {
        let Some(mut current) = self.current else {
            return Err(TransitionError::NoActiveTick);
        };
        if current.id != tick_id {
            return Err(TransitionError::WrongTick {
                active: current.id,
                requested: tick_id,
            });
        }
        if current.phase != TickPhase::Begun {
            return Err(TransitionError::UnexpectedPhase {
                tick_id,
                expected: TickPhase::Begun,
                actual: current.phase,
            });
        }

        current.phase = TickPhase::PhysicsApplied;
        self.current = Some(current);
        Ok(())
    }

    /// Finalize a tick and return its receipt.
    ///
    /// Retrying with the same result digest is idempotent. Retrying with a
    /// different digest is a conflict and does not overwrite the first receipt.
    /// The receipt remains replayable until a later tick begins.
    pub fn finalize(
        &mut self,
        tick_id: TickId,
        result_state_digest: [u8; 32],
    ) -> Result<TickReceipt, TransitionError> {
        let Some(mut current) = self.current else {
            return Err(TransitionError::NoActiveTick);
        };
        if current.id != tick_id {
            return Err(TransitionError::WrongTick {
                active: current.id,
                requested: tick_id,
            });
        }

        if current.phase == TickPhase::Finalized {
            let receipt = current.receipt.expect("finalized ticks always carry a receipt");
            return if receipt.result_state_digest == result_state_digest {
                Ok(receipt)
            } else {
                Err(TransitionError::FinalizationConflict { tick_id })
            };
        }
        if current.phase != TickPhase::PhysicsApplied {
            return Err(TransitionError::UnexpectedPhase {
                tick_id,
                expected: TickPhase::PhysicsApplied,
                actual: current.phase,
            });
        }

        let receipt = TickReceipt {
            schema_version: 1,
            tick_id,
            input_digest: current.input_digest,
            prior_state_digest: current.prior_state_digest,
            result_state_digest,
        };
        current.phase = TickPhase::Finalized;
        current.receipt = Some(receipt);
        self.current = Some(current);
        self.last_finalized = Some(tick_id);
        Ok(receipt)
    }

    /// Return the current tick identity, if a tick has been started.
    pub fn active_tick(&self) -> Option<TickId> {
        self.current.map(|current| current.id)
    }

    /// Return the current lifecycle phase, if a tick has been started.
    pub fn phase(&self) -> Option<TickPhase> {
        self.current.map(|current| current.phase)
    }

    /// Return the newest finalized tick identity, if one exists.
    pub fn last_finalized_tick(&self) -> Option<TickId> {
        self.last_finalized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: u64) -> [u8; 32] {
        let mut bytes = [0_u8; 32];
        bytes[..8].copy_from_slice(&value.to_le_bytes());
        bytes[8..16].copy_from_slice(&value.rotate_left(19).to_le_bytes());
        for (index, byte) in bytes[16..].iter_mut().enumerate() {
            *byte = value
                .wrapping_mul(37)
                .wrapping_add(index as u64)
                .rotate_left((index % 63) as u32) as u8;
        }
        bytes
    }

    #[test]
    fn legal_lifecycle_emits_versioned_receipt() {
        let mut transaction = TickTransaction::default();
        let tick = TickId(42);
        let input = digest(1);
        let prior = digest(2);
        let result = digest(3);

        assert_eq!(transaction.begin(tick, input, prior), Ok(()));
        assert_eq!(transaction.phase(), Some(TickPhase::Begun));
        assert_eq!(
            transaction.mark_physics_applied(tick),
            Ok(())
        );
        assert_eq!(transaction.phase(), Some(TickPhase::PhysicsApplied));

        let receipt = transaction.finalize(tick, result).unwrap();
        assert_eq!(
            receipt,
            TickReceipt {
                schema_version: 1,
                tick_id: tick,
                input_digest: input,
                prior_state_digest: prior,
                result_state_digest: result,
            }
        );
        assert_eq!(transaction.phase(), Some(TickPhase::Finalized));
        assert_eq!(transaction.last_finalized_tick(), Some(tick));
    }

    #[test]
    fn illegal_transitions_fail_without_mutating_state() {
        let mut transaction = TickTransaction::default();
        assert_eq!(
            transaction.mark_physics_applied(TickId(1)),
            Err(TransitionError::NoActiveTick)
        );

        let tick = TickId(7);
        transaction.begin(tick, digest(1), digest(2)).unwrap();
        let before = transaction.phase();

        assert_eq!(
            transaction.begin(TickId(8), digest(3), digest(4)),
            Err(TransitionError::TickAlreadyActive { active: tick })
        );
        assert_eq!(
            transaction.finalize(tick, digest(5)),
            Err(TransitionError::UnexpectedPhase {
                tick_id: tick,
                expected: TickPhase::PhysicsApplied,
                actual: TickPhase::Begun,
            })
        );
        assert_eq!(
            transaction.mark_physics_applied(TickId(8)),
            Err(TransitionError::WrongTick {
                active: tick,
                requested: TickId(8),
            })
        );
        assert_eq!(transaction.phase(), before);
        assert_eq!(transaction.active_tick(), Some(tick));
    }

    #[test]
    fn repeated_finalize_is_idempotent_and_conflicts_fail_closed() {
        let mut transaction = TickTransaction::default();
        let tick = TickId(12);
        transaction.begin(tick, digest(1), digest(2)).unwrap();
        transaction.mark_physics_applied(tick).unwrap();

        let first = transaction.finalize(tick, digest(3)).unwrap();
        let second = transaction.finalize(tick, digest(3)).unwrap();
        assert_eq!(second, first);
        assert_eq!(
            transaction.finalize(tick, digest(999)),
            Err(TransitionError::FinalizationConflict { tick_id: tick })
        );
        assert_eq!(transaction.finalize(tick, digest(3)), Ok(first));
        assert_eq!(transaction.last_finalized_tick(), Some(tick));
    }

    #[test]
    fn finalized_ticks_require_strictly_monotonic_ids() {
        let mut transaction = TickTransaction::default();
        let first = TickId(10);
        transaction.begin(first, digest(1), digest(2)).unwrap();
        transaction.mark_physics_applied(first).unwrap();
        transaction.finalize(first, digest(3)).unwrap();

        for stale in [TickId(10), TickId(9), TickId(0)] {
            assert_eq!(
                transaction.begin(stale, digest(4), digest(5)),
                Err(TransitionError::NonMonotonicTick {
                    last_finalized: first,
                    proposed: stale,
                })
            );
            assert_eq!(transaction.last_finalized_tick(), Some(first));
        }

        // Gaps are a documented allocator policy, not an implicit failure.
        let later = TickId(12);
        assert_eq!(transaction.begin(later, digest(6), digest(3)), Ok(()));
    }

    #[test]
    fn next_tick_must_link_to_the_previous_result_state() {
        let mut transaction = TickTransaction::default();
        let first = TickId(4);
        transaction.begin(first, digest(1), digest(2)).unwrap();
        transaction.mark_physics_applied(first).unwrap();
        let receipt = transaction.finalize(first, digest(3)).unwrap();

        let next = TickId(5);
        assert_eq!(
            transaction.begin(next, digest(4), digest(99)),
            Err(TransitionError::PriorStateMismatch {
                previous_tick: first,
            })
        );
        assert_eq!(transaction.phase(), Some(TickPhase::Finalized));
        assert_eq!(transaction.last_finalized_tick(), Some(first));

        // Retry the begin with the exact committed state; the failed attempt
        // did not mutate the finalized transaction.
        assert_eq!(
            transaction.begin(next, digest(4), receipt.result_state_digest),
            Ok(())
        );
    }

    #[test]
    fn million_tick_replay_is_deterministic_with_seeded_finalize_retries() {
        #[derive(Debug, Eq, PartialEq)]
        struct Summary {
            last_receipt: TickReceipt,
            rolling_check: u64,
            retry_count: u64,
        }

        fn replay() -> Summary {
            const TICKS: u64 = 1_000_000;
            let mut transaction = TickTransaction::default();
            let mut seed = 0x4d59_5df4_d0f3_3173_u64;
            let mut rolling_check = 0_u64;
            let mut retry_count = 0_u64;
            let mut last_receipt = None;

            for counter in 0..TICKS {
                let tick = TickId(counter);
                transaction
                    .begin(tick, digest(counter ^ 0xa5a5), digest(counter))
                    .unwrap();
                transaction.mark_physics_applied(tick).unwrap();
                let result_digest = digest(counter.wrapping_add(1));
                let receipt = transaction.finalize(tick, result_digest).unwrap();

                // Deterministic 1–5 finalize retries per tick. These must not
                // mutate the receipt or advance the transaction.
                seed = seed
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1_442_695_040_888_963_407);
                let retries = ((seed >> 32) % 5) + 1;
                for _ in 0..retries {
                    assert_eq!(transaction.finalize(tick, result_digest), Ok(receipt));
                }
                retry_count += retries;
                let receipt_word = u64::from_le_bytes(
                    receipt.result_state_digest[..8]
                        .try_into()
                        .expect("fixed-size digest prefix"),
                );
                rolling_check = rolling_check.rotate_left(7) ^ receipt_word ^ counter;
                last_receipt = Some(receipt);
            }

            Summary {
                last_receipt: last_receipt.expect("non-empty replay"),
                rolling_check,
                retry_count,
            }
        }

        assert_eq!(replay(), replay());
    }
}
