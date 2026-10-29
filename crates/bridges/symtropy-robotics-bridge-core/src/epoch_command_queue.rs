// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Bounded epoch-aware command queue with synchronous epoch and lease checks.
//!
//! Epoch and monotonic deadline are checked at enqueue and again immediately
//! before the synchronous application callback. Advancing the controller epoch
//! purges and returns every pending prior-generation command for audit or
//! host-side cancellation. This reference API does not expose a pop-and-apply
//! later operation.
//!
//! This is an in-process contract only. Real hardware/IPC must enforce the same
//! epoch and deadline at its actual write/application boundary, and retain
//! independent watchdog, stop, and safe-state mechanisms.

use crate::epoch_fence::ControllerEpoch;
use std::collections::{BTreeSet, VecDeque};

/// Fail-closed outcomes from the bounded epoch command queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpochCommandQueueError {
    ZeroCapacity,
    EpochDidNotAdvance {
        current_generation: u64,
        requested_generation: u64,
    },
    StaleEpoch {
        current_generation: u64,
        supplied_generation: u64,
    },
    DeadlineExpired {
        request_id: u64,
        deadline_ns: u64,
        observed_at_ns: u64,
    },
    QueueFull {
        capacity: usize,
    },
    DuplicatePendingRequest {
        generation: u64,
        request_id: u64,
    },
}

/// One queued command, bound to the generation and monotonic expiry deadline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpochQueuedCommand<T> {
    pub epoch: ControllerEpoch,
    pub request_id: u64,
    pub deadline_ns: u64,
    pub command: T,
}

/// Observable result of attempting to apply one queued command.
#[derive(Debug, PartialEq, Eq)]
pub enum EpochCommandApplication<R> {
    Applied {
        epoch: ControllerEpoch,
        request_id: u64,
        deadline_ns: u64,
        result: R,
    },
    RejectedStaleEpoch {
        queued_epoch: ControllerEpoch,
        current_epoch: ControllerEpoch,
        request_id: u64,
    },
    RejectedExpired {
        epoch: ControllerEpoch,
        request_id: u64,
        deadline_ns: u64,
        observed_at_ns: u64,
    },
}

/// Bounded counters useful for a trace or benchmark receipt.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EpochCommandQueueStats {
    pub accepted_enqueues: u64,
    pub stale_epoch_enqueues_rejected: u64,
    pub expired_enqueues_rejected: u64,
    pub queue_full_rejections: u64,
    pub duplicate_pending_request_rejections: u64,
    pub stale_commands_cancelled_on_advance: u64,
    pub stale_commands_rejected_before_application: u64,
    pub expired_commands_rejected_before_application: u64,
}

/// Single-owner queue that checks epoch and lease again at application time.
pub struct EpochCommandQueue<T> {
    current_epoch: ControllerEpoch,
    capacity: usize,
    pending: VecDeque<EpochQueuedCommand<T>>,
    pending_request_ids: BTreeSet<(u64, u64)>,
    stats: EpochCommandQueueStats,
}

impl<T> EpochCommandQueue<T> {
    /// Construct a queue with positive explicit capacity.
    ///
    /// The queue grows only as commands are enqueued, not to the configured
    /// capacity eagerly. Hosts should still choose a small, task-specific bound.
    pub fn new(
        initial_epoch: ControllerEpoch,
        capacity: usize,
    ) -> Result<Self, EpochCommandQueueError> {
        if capacity == 0 {
            return Err(EpochCommandQueueError::ZeroCapacity);
        }
        Ok(Self {
            current_epoch: initial_epoch,
            capacity,
            pending: VecDeque::new(),
            pending_request_ids: BTreeSet::new(),
            stats: EpochCommandQueueStats::default(),
        })
    }

    pub const fn current_epoch(&self) -> ControllerEpoch {
        self.current_epoch
    }

    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }

    pub const fn stats(&self) -> EpochCommandQueueStats {
        self.stats
    }

    /// Queue work tagged for the current epoch if its monotonic lease is live.
    ///
    /// now_ns and deadline_ns must use the same monotonic clock domain.
    /// The upstream sequence/replay guard remains responsible for lifetime
    /// request uniqueness; this queue rejects duplicate IDs only while a prior
    /// command with that ID is pending in the same generation.
    pub fn enqueue_at(
        &mut self,
        epoch: ControllerEpoch,
        request_id: u64,
        deadline_ns: u64,
        now_ns: u64,
        command: T,
    ) -> Result<(), EpochCommandQueueError> {
        if epoch != self.current_epoch {
            self.stats.stale_epoch_enqueues_rejected =
                self.stats.stale_epoch_enqueues_rejected.saturating_add(1);
            return Err(EpochCommandQueueError::StaleEpoch {
                current_generation: self.current_epoch.generation(),
                supplied_generation: epoch.generation(),
            });
        }
        if now_ns >= deadline_ns {
            self.stats.expired_enqueues_rejected =
                self.stats.expired_enqueues_rejected.saturating_add(1);
            return Err(EpochCommandQueueError::DeadlineExpired {
                request_id,
                deadline_ns,
                observed_at_ns: now_ns,
            });
        }
        if self.pending.len() >= self.capacity {
            self.stats.queue_full_rejections = self.stats.queue_full_rejections.saturating_add(1);
            return Err(EpochCommandQueueError::QueueFull {
                capacity: self.capacity,
            });
        }

        let key = (epoch.generation(), request_id);
        if !self.pending_request_ids.insert(key) {
            self.stats.duplicate_pending_request_rejections = self
                .stats
                .duplicate_pending_request_rejections
                .saturating_add(1);
            return Err(EpochCommandQueueError::DuplicatePendingRequest {
                generation: epoch.generation(),
                request_id,
            });
        }

        self.pending.push_back(EpochQueuedCommand {
            epoch,
            request_id,
            deadline_ns,
            command,
        });
        self.stats.accepted_enqueues = self.stats.accepted_enqueues.saturating_add(1);
        Ok(())
    }

    /// Advance monotonically and return every invalidated pending command.
    ///
    /// Returned commands must be recorded as cancelled/dropped and never
    /// forwarded into a controller. Returning them keeps the transition auditable.
    pub fn advance_epoch(
        &mut self,
        next_epoch: ControllerEpoch,
    ) -> Result<Vec<EpochQueuedCommand<T>>, EpochCommandQueueError> {
        if next_epoch <= self.current_epoch {
            return Err(EpochCommandQueueError::EpochDidNotAdvance {
                current_generation: self.current_epoch.generation(),
                requested_generation: next_epoch.generation(),
            });
        }

        self.current_epoch = next_epoch;
        self.pending_request_ids.clear();
        let cancelled: Vec<_> = self.pending.drain(..).collect();
        self.stats.stale_commands_cancelled_on_advance = self
            .stats
            .stale_commands_cancelled_on_advance
            .saturating_add(cancelled.len() as u64);
        Ok(cancelled)
    }

    /// Recheck generation and lease, then synchronously apply one queued command.
    ///
    /// The callback must perform the native write/application step synchronously
    /// and must not defer the command into another unfenced queue. Both timestamps
    /// use the same monotonic clock domain. This queue remains exclusively
    /// borrowed through the callback, serializing epoch transition in this
    /// process. The real native sink must still atomically enforce the epoch.
    ///
    /// One queue item yields one explicit result. Expired or stale items are
    /// consumed and reported; they are never forwarded to the application callback.
    pub fn apply_next_at<R>(
        &mut self,
        now_ns: u64,
        apply: impl FnOnce(ControllerEpoch, u64, u64, &T) -> R,
    ) -> Option<EpochCommandApplication<R>> {
        let queued = self.pending.pop_front()?;
        self.pending_request_ids
            .remove(&(queued.epoch.generation(), queued.request_id));

        if queued.epoch != self.current_epoch {
            self.stats.stale_commands_rejected_before_application = self
                .stats
                .stale_commands_rejected_before_application
                .saturating_add(1);
            return Some(EpochCommandApplication::RejectedStaleEpoch {
                queued_epoch: queued.epoch,
                current_epoch: self.current_epoch,
                request_id: queued.request_id,
            });
        }
        if now_ns >= queued.deadline_ns {
            self.stats.expired_commands_rejected_before_application = self
                .stats
                .expired_commands_rejected_before_application
                .saturating_add(1);
            return Some(EpochCommandApplication::RejectedExpired {
                epoch: queued.epoch,
                request_id: queued.request_id,
                deadline_ns: queued.deadline_ns,
                observed_at_ns: now_ns,
            });
        }

        let result = apply(
            queued.epoch,
            queued.request_id,
            queued.deadline_ns,
            &queued.command,
        );
        Some(EpochCommandApplication::Applied {
            epoch: queued.epoch,
            request_id: queued.request_id,
            deadline_ns: queued.deadline_ns,
            result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn epoch(generation: u64) -> ControllerEpoch {
        ControllerEpoch::new(generation).unwrap()
    }

    #[test]
    fn zero_capacity_is_rejected_and_large_capacity_is_not_eagerly_allocated() {
        assert_eq!(
            EpochCommandQueue::<u64>::new(epoch(1), 0).err(),
            Some(EpochCommandQueueError::ZeroCapacity)
        );
        assert!(
            EpochCommandQueue::<u64>::new(epoch(1), usize::MAX)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn current_epoch_live_command_is_applied_with_bound_identity_and_deadline() {
        let mut queue = EpochCommandQueue::new(epoch(4), 2).unwrap();
        queue
            .enqueue_at(epoch(4), 71, 100, 1, "drive-forward")
            .unwrap();
        let applied = queue.apply_next_at(2, |_, _, _, command| *command);
        assert_eq!(
            applied,
            Some(EpochCommandApplication::Applied {
                epoch: epoch(4),
                request_id: 71,
                deadline_ns: 100,
                result: "drive-forward",
            })
        );
        assert!(queue.is_empty());
    }

    #[test]
    fn stale_epoch_enqueue_is_rejected_without_adding_work() {
        let mut queue = EpochCommandQueue::new(epoch(5), 2).unwrap();
        assert_eq!(
            queue.enqueue_at(epoch(4), 1, 100, 1, "old"),
            Err(EpochCommandQueueError::StaleEpoch {
                current_generation: 5,
                supplied_generation: 4,
            })
        );
        assert!(queue.is_empty());
        assert_eq!(queue.stats().stale_epoch_enqueues_rejected, 1);
    }

    #[test]
    fn already_expired_enqueue_is_rejected() {
        let mut queue = EpochCommandQueue::new(epoch(5), 2).unwrap();
        assert_eq!(
            queue.enqueue_at(epoch(5), 1, 10, 10, "expired"),
            Err(EpochCommandQueueError::DeadlineExpired {
                request_id: 1,
                deadline_ns: 10,
                observed_at_ns: 10,
            })
        );
        assert!(queue.is_empty());
        assert_eq!(queue.stats().expired_enqueues_rejected, 1);
    }

    #[test]
    fn command_expiring_while_queued_never_reaches_application_callback() {
        let mut queue = EpochCommandQueue::new(epoch(1), 2).unwrap();
        queue.enqueue_at(epoch(1), 4, 10, 1, "drive").unwrap();
        let applied = queue.apply_next_at(10, |_, _, _, _| panic!("expired command applied"));
        assert_eq!(
            applied,
            Some(EpochCommandApplication::RejectedExpired {
                epoch: epoch(1),
                request_id: 4,
                deadline_ns: 10,
                observed_at_ns: 10,
            })
        );
        assert_eq!(
            queue.stats().expired_commands_rejected_before_application,
            1
        );
        assert!(queue.is_empty());
    }

    #[test]
    fn epoch_advance_returns_and_purges_every_queued_old_command() {
        let mut queue = EpochCommandQueue::new(epoch(1), 4).unwrap();
        queue.enqueue_at(epoch(1), 10, 100, 1, "left").unwrap();
        queue.enqueue_at(epoch(1), 11, 100, 1, "right").unwrap();

        let cancelled = queue.advance_epoch(epoch(2)).unwrap();
        assert_eq!(cancelled.len(), 2);
        assert!(cancelled.iter().all(|cmd| cmd.epoch == epoch(1)));
        assert_eq!(queue.current_epoch(), epoch(2));
        assert!(queue.is_empty());

        queue.enqueue_at(epoch(2), 10, 100, 1, "new-left").unwrap();
        assert_eq!(
            queue.apply_next_at(2, |_, _, _, command| *command),
            Some(EpochCommandApplication::Applied {
                epoch: epoch(2),
                request_id: 10,
                deadline_ns: 100,
                result: "new-left",
            })
        );
        assert_eq!(queue.stats().stale_commands_cancelled_on_advance, 2);
    }

    #[test]
    fn epoch_must_advance_strictly() {
        let mut queue = EpochCommandQueue::<u8>::new(epoch(3), 1).unwrap();
        for generation in [3, 2] {
            assert_eq!(
                queue.advance_epoch(epoch(generation)).err(),
                Some(EpochCommandQueueError::EpochDidNotAdvance {
                    current_generation: 3,
                    requested_generation: generation,
                })
            );
        }
        assert_eq!(queue.current_epoch(), epoch(3));
    }

    #[test]
    fn capacity_exhaustion_fails_closed_without_evicting_the_oldest_command() {
        let mut queue = EpochCommandQueue::new(epoch(1), 1).unwrap();
        queue.enqueue_at(epoch(1), 1, 100, 1, "first").unwrap();
        assert_eq!(
            queue.enqueue_at(epoch(1), 2, 100, 1, "second"),
            Err(EpochCommandQueueError::QueueFull { capacity: 1 })
        );
        assert_eq!(queue.len(), 1);
        assert_eq!(
            queue.apply_next_at(2, |_, request_id, _, command| (request_id, *command)),
            Some(EpochCommandApplication::Applied {
                epoch: epoch(1),
                request_id: 1,
                deadline_ns: 100,
                result: (1, "first"),
            })
        );
        assert_eq!(queue.stats().queue_full_rejections, 1);
    }

    #[test]
    fn duplicate_pending_identity_is_rejected() {
        let mut queue = EpochCommandQueue::new(epoch(9), 3).unwrap();
        queue.enqueue_at(epoch(9), 44, 100, 1, "first").unwrap();
        assert_eq!(
            queue.enqueue_at(epoch(9), 44, 100, 1, "duplicate"),
            Err(EpochCommandQueueError::DuplicatePendingRequest {
                generation: 9,
                request_id: 44,
            })
        );
        assert_eq!(queue.len(), 1);
        assert_eq!(queue.stats().duplicate_pending_request_rejections, 1);
    }

    #[test]
    fn queued_prior_generation_work_never_reaches_the_application_callback() {
        let mut queue = EpochCommandQueue::new(epoch(20), 4).unwrap();
        queue.enqueue_at(epoch(20), 90, 100, 1, "old-a").unwrap();
        queue.enqueue_at(epoch(20), 91, 100, 1, "old-b").unwrap();
        let cancelled = queue.advance_epoch(epoch(21)).unwrap();
        assert_eq!(cancelled.len(), 2);

        queue.enqueue_at(epoch(21), 92, 100, 1, "current").unwrap();
        let current_epoch = queue.current_epoch();
        let mut applied_ids = Vec::new();
        let applied = queue.apply_next_at(2, |command_epoch, request_id, _, _| {
            assert_eq!(command_epoch, current_epoch);
            applied_ids.push(request_id);
        });
        assert_eq!(
            applied,
            Some(EpochCommandApplication::Applied {
                epoch: epoch(21),
                request_id: 92,
                deadline_ns: 100,
                result: (),
            })
        );
        assert_eq!(applied_ids, vec![92]);
    }

    #[test]
    fn deterministic_256_request_restart_stress_never_applies_a_prior_epoch_or_expired_lease() {
        let mut queue = EpochCommandQueue::new(epoch(1), 8).unwrap();
        let mut applied = Vec::new();

        for request_id in 0..256u64 {
            if request_id > 0 && request_id % 17 == 0 {
                let next = epoch(queue.current_epoch().generation() + 1);
                let cancelled = queue.advance_epoch(next).unwrap();
                assert!(cancelled.iter().all(|command| command.epoch < next));
            }

            let active = queue.current_epoch();
            let supplied = if request_id % 7 == 0 && active.generation() > 1 {
                epoch(active.generation() - 1)
            } else {
                active
            };
            let _ = queue.enqueue_at(supplied, request_id, 10_000, request_id, request_id);

            if request_id % 3 == 0 {
                let at_apply = queue.current_epoch();
                queue.apply_next_at(request_id, |command_epoch, id, deadline, payload| {
                    assert_eq!(command_epoch, at_apply);
                    assert!(request_id < deadline);
                    assert_eq!(id, *payload);
                    applied.push((command_epoch.generation(), id));
                });
            }
        }

        let next = epoch(queue.current_epoch().generation() + 1);
        let cancelled = queue.advance_epoch(next).unwrap();
        assert!(cancelled.iter().all(|command| command.epoch < next));
        assert!(queue.is_empty());
        assert!(
            applied
                .iter()
                .all(|(generation, _)| *generation < next.generation())
        );
        assert!(queue.stats().stale_epoch_enqueues_rejected > 0);
    }
}
