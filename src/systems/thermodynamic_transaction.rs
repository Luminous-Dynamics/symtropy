// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Fixed-tick thermodynamic transaction authority.
//!
//! Additive lifecycle kernel for #824. It does not yet rewrite the launcher's
//! large scheduling/physics adapter; it defines the exact state machine that the
//! later adapter must own around the current operational begin/consequence/close.
//!
//! `Idle/Finalized -> Open -> Finalizing -> Finalized`.
//!
//! Core friction lifecycle evidence is a finalize precondition and is embedded
//! structurally in the final receipt. Unresolved or cross-tick friction evidence
//! blocks finalization.

use bevy::prelude::Resource;
use symtropy_physics::FrictionTransactionJournal;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ThermodynamicConsequenceStatus {
    Executed,
    Rejected,
    IntentionallyAbsent,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct OpenTick {
    tick_id: u64,
    generation: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FinalizingTick {
    tick_id: u64,
    generation: u64,
    consequence_status: ThermodynamicConsequenceStatus,
    friction_snapshot: FrictionTransactionJournal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ActiveTickState {
    Idle,
    Open(OpenTick),
    Finalizing(FinalizingTick),
}

impl Default for ActiveTickState {
    fn default() -> Self {
        Self::Idle
    }
}

/// Receipt proving one fixed thermodynamic interval reached a committed close.
/// The complete friction journal is stored as typed structure rather than a
/// caller-authored digest, binding exact transaction ids and terminal outcomes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThermodynamicTickReceipt {
    pub tick_id: u64,
    pub generation: u64,
    pub consequence_status: ThermodynamicConsequenceStatus,
    pub friction_transactions: FrictionTransactionJournal,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ThermodynamicBeginPermit {
    tick_id: u64,
    generation: u64,
}

impl ThermodynamicBeginPermit {
    pub const fn tick_id(self) -> u64 {
        self.tick_id
    }

    pub const fn generation(self) -> u64 {
        self.generation
    }
}

/// Non-cloneable reservation for one finalize attempt.
#[derive(Debug, PartialEq, Eq)]
pub struct ThermodynamicFinalizePermit {
    tick_id: u64,
    generation: u64,
}

impl ThermodynamicFinalizePermit {
    pub const fn tick_id(&self) -> u64 {
        self.tick_id
    }

    pub const fn generation(&self) -> u64 {
        self.generation
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ThermodynamicTickError {
    DuplicateBegin { tick_id: u64 },
    TickStillOpen { open_tick_id: u64 },
    FinalizeInProgress { tick_id: u64 },
    AlreadyFinalized { tick_id: u64 },
    NonMonotonicBegin {
        requested_tick_id: u64,
        last_finalized_tick_id: u64,
    },
    DirtyFrictionJournal,
    NoOpenTick,
    WrongOpenTick {
        requested_tick_id: u64,
        open_tick_id: u64,
    },
    CrossTickFrictionTransactions { expected_tick_id: u64 },
    PendingFrictionTransactions { count: usize },
    StaleFinalizePermit,
    FrictionJournalChangedAfterPrepare,
    GenerationOverflow,
}

/// Exactly-once lifecycle authority for the launcher's operational thermodynamic
/// interval. This resource owns transaction state only; it does not itself mutate
/// EnergyBudget, HUD telemetry, or the legacy operational ledger.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct ThermodynamicTickAuthority {
    active: ActiveTickState,
    last_finalized: Option<ThermodynamicTickReceipt>,
    next_generation: u64,
}

impl ThermodynamicTickAuthority {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn last_finalized_receipt(&self) -> Option<&ThermodynamicTickReceipt> {
        self.last_finalized.as_ref()
    }

    pub fn open_tick_id(&self) -> Option<u64> {
        match &self.active {
            ActiveTickState::Open(open) => Some(open.tick_id),
            ActiveTickState::Finalizing(finalizing) => Some(finalizing.tick_id),
            ActiveTickState::Idle => None,
        }
    }

    pub fn is_finalize_in_progress(&self) -> bool {
        matches!(self.active, ActiveTickState::Finalizing(_))
    }

    /// Admit one fixed tick before any tick-start operational mutation.
    ///
    /// A new tick requires a freshly rotated empty friction journal. This makes
    /// accidental carry-over from a previously finalized interval fail closed.
    pub fn begin(
        &mut self,
        tick_id: u64,
        friction_journal: &FrictionTransactionJournal,
    ) -> Result<ThermodynamicBeginPermit, ThermodynamicTickError> {
        match &self.active {
            ActiveTickState::Open(open) if open.tick_id == tick_id => {
                return Err(ThermodynamicTickError::DuplicateBegin { tick_id });
            }
            ActiveTickState::Open(open) => {
                return Err(ThermodynamicTickError::TickStillOpen {
                    open_tick_id: open.tick_id,
                });
            }
            ActiveTickState::Finalizing(finalizing) => {
                return Err(ThermodynamicTickError::FinalizeInProgress {
                    tick_id: finalizing.tick_id,
                });
            }
            ActiveTickState::Idle => {}
        }

        if let Some(last) = &self.last_finalized {
            if tick_id == last.tick_id {
                return Err(ThermodynamicTickError::AlreadyFinalized { tick_id });
            }
            if tick_id < last.tick_id {
                return Err(ThermodynamicTickError::NonMonotonicBegin {
                    requested_tick_id: tick_id,
                    last_finalized_tick_id: last.tick_id,
                });
            }
        }

        if !friction_journal.is_empty() {
            return Err(ThermodynamicTickError::DirtyFrictionJournal);
        }

        let generation = self.next_generation;
        self.next_generation = self
            .next_generation
            .checked_add(1)
            .ok_or(ThermodynamicTickError::GenerationOverflow)?;
        self.active = ActiveTickState::Open(OpenTick {
            tick_id,
            generation,
        });

        Ok(ThermodynamicBeginPermit {
            tick_id,
            generation,
        })
    }

    /// Reserve the one allowed finalize attempt.
    ///
    /// Every friction transaction must belong to this exact fixed tick and must
    /// already be terminal (`Promoted` or typed `DiagnosticOnly`).
    pub fn prepare_finalize(
        &mut self,
        tick_id: u64,
        consequence_status: ThermodynamicConsequenceStatus,
        friction_journal: &FrictionTransactionJournal,
    ) -> Result<ThermodynamicFinalizePermit, ThermodynamicTickError> {
        let open = match &self.active {
            ActiveTickState::Idle => {
                if self
                    .last_finalized
                    .as_ref()
                    .is_some_and(|last| last.tick_id == tick_id)
                {
                    return Err(ThermodynamicTickError::AlreadyFinalized { tick_id });
                }
                return Err(ThermodynamicTickError::NoOpenTick);
            }
            ActiveTickState::Finalizing(finalizing) => {
                return Err(ThermodynamicTickError::FinalizeInProgress {
                    tick_id: finalizing.tick_id,
                });
            }
            ActiveTickState::Open(open) => *open,
        };

        if open.tick_id != tick_id {
            return Err(ThermodynamicTickError::WrongOpenTick {
                requested_tick_id: tick_id,
                open_tick_id: open.tick_id,
            });
        }
        if !friction_journal.matches_fixed_tick(tick_id) {
            return Err(ThermodynamicTickError::CrossTickFrictionTransactions {
                expected_tick_id: tick_id,
            });
        }

        let pending = friction_journal.pending_application_count();
        if pending != 0 {
            return Err(ThermodynamicTickError::PendingFrictionTransactions { count: pending });
        }

        self.active = ActiveTickState::Finalizing(FinalizingTick {
            tick_id,
            generation: open.generation,
            consequence_status,
            friction_snapshot: friction_journal.clone(),
        });

        Ok(ThermodynamicFinalizePermit {
            tick_id,
            generation: open.generation,
        })
    }

    fn prepared_finalize<'a>(
        &'a self,
        permit: &ThermodynamicFinalizePermit,
    ) -> Result<&'a FinalizingTick, ThermodynamicTickError> {
        match &self.active {
            ActiveTickState::Finalizing(finalizing)
                if finalizing.tick_id == permit.tick_id
                    && finalizing.generation == permit.generation =>
            {
                Ok(finalizing)
            }
            _ => Err(ThermodynamicTickError::StaleFinalizePermit),
        }
    }

    /// Revalidate the exact prepared friction snapshot immediately before the
    /// outer legacy thermodynamic/HUD close mutates any state.
    pub fn validate_prepared_finalize(
        &self,
        permit: &ThermodynamicFinalizePermit,
        friction_journal: &FrictionTransactionJournal,
    ) -> Result<(), ThermodynamicTickError> {
        let finalizing = self.prepared_finalize(permit)?;
        if !friction_journal.matches_fixed_tick(finalizing.tick_id) {
            return Err(ThermodynamicTickError::CrossTickFrictionTransactions {
                expected_tick_id: finalizing.tick_id,
            });
        }
        if &finalizing.friction_snapshot != friction_journal {
            return Err(ThermodynamicTickError::FrictionJournalChangedAfterPrepare);
        }
        Ok(())
    }

    /// Commit the lifecycle only after the existing close has succeeded.
    ///
    /// The permit is borrowed. If validation fails the caller still owns it and
    /// can call `rollback_finalize` to return this same tick to `Open` without a
    /// duplicate begin/reset. After success, state makes the permit stale.
    pub fn commit_finalize(
        &mut self,
        permit: &ThermodynamicFinalizePermit,
        friction_journal: &FrictionTransactionJournal,
    ) -> Result<ThermodynamicTickReceipt, ThermodynamicTickError> {
        let finalizing = self.prepared_finalize(permit)?.clone();
        if !friction_journal.matches_fixed_tick(finalizing.tick_id) {
            return Err(ThermodynamicTickError::CrossTickFrictionTransactions {
                expected_tick_id: finalizing.tick_id,
            });
        }
        if &finalizing.friction_snapshot != friction_journal {
            return Err(ThermodynamicTickError::FrictionJournalChangedAfterPrepare);
        }

        let receipt = ThermodynamicTickReceipt {
            tick_id: finalizing.tick_id,
            generation: finalizing.generation,
            consequence_status: finalizing.consequence_status,
            friction_transactions: finalizing.friction_snapshot,
        };
        self.active = ActiveTickState::Idle;
        self.last_finalized = Some(receipt.clone());
        Ok(receipt)
    }

    /// Cancel a prepared finalize before the outer close commits any mutation.
    pub fn rollback_finalize(
        &mut self,
        permit: ThermodynamicFinalizePermit,
    ) -> Result<(), ThermodynamicTickError> {
        let finalizing = self.prepared_finalize(&permit)?.clone();
        self.active = ActiveTickState::Open(OpenTick {
            tick_id: finalizing.tick_id,
            generation: finalizing.generation,
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::SVector;
    use symtropy_math::Point;
    use symtropy_physics::{
        BodyHandle, FrictionDiagnosticReason, FrictionTransactionId,
        FrictionTransactionPhase, RigidBody, apply_friction_impulse_once,
        finalize_friction_diagnostic,
    };

    fn pair() -> (RigidBody<3>, RigidBody<3>) {
        let mut a = RigidBody::<3>::dynamic_sphere(
            BodyHandle(1),
            Point::origin(),
            0.5,
            1.0,
        );
        let b = RigidBody::<3>::dynamic_sphere(
            BodyHandle(2),
            Point::origin(),
            0.5,
            1.0,
        );
        a.linear_velocity[0] = 1.0;
        (a, b)
    }

    fn off_center_terminal_journal(fixed_tick: u64) -> FrictionTransactionJournal {
        let mut a = RigidBody::<3>::dynamic_sphere(
            BodyHandle(1),
            Point::new([-1.0, 0.0, 0.0]),
            0.5,
            1.0,
        );
        let mut b = RigidBody::<3>::dynamic_sphere(
            BodyHandle(2),
            Point::new([1.0, 0.0, 0.0]),
            0.5,
            1.0,
        );
        a.linear_velocity[0] = 1.0;
        let mut journal = FrictionTransactionJournal::new();
        let applied = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::from([0.0, 0.5, 0.0]),
            &SVector::from([0.0, -0.1, 0.0]),
            FrictionTransactionId::new(fixed_tick, 0, 0, 0),
            &mut journal,
        )
        .unwrap();
        assert_eq!(
            finalize_friction_diagnostic(&a, &b, &applied, &mut journal).unwrap(),
            FrictionDiagnosticReason::OffCenterUnqualified
        );
        journal
    }

    #[test]
    fn duplicate_begin_does_not_reopen_tick() {
        let journal = FrictionTransactionJournal::new();
        let mut authority = ThermodynamicTickAuthority::new();
        let first = authority.begin(10, &journal).unwrap();
        assert_eq!(first.generation(), 0);
        assert_eq!(
            authority.begin(10, &journal),
            Err(ThermodynamicTickError::DuplicateBegin { tick_id: 10 })
        );
        assert_eq!(authority.open_tick_id(), Some(10));
    }

    #[test]
    fn finalize_before_begin_fails_closed() {
        let journal = FrictionTransactionJournal::new();
        let mut authority = ThermodynamicTickAuthority::new();
        assert_eq!(
            authority.prepare_finalize(10, ThermodynamicConsequenceStatus::Executed, &journal),
            Err(ThermodynamicTickError::NoOpenTick)
        );
    }

    #[test]
    fn begin_next_before_current_finalize_is_rejected() {
        let journal = FrictionTransactionJournal::new();
        let mut authority = ThermodynamicTickAuthority::new();
        authority.begin(10, &journal).unwrap();
        assert_eq!(
            authority.begin(11, &journal),
            Err(ThermodynamicTickError::TickStillOpen { open_tick_id: 10 })
        );
    }

    #[test]
    fn prepare_is_exclusive_until_commit_or_rollback() {
        let journal = FrictionTransactionJournal::new();
        let mut authority = ThermodynamicTickAuthority::new();
        authority.begin(10, &journal).unwrap();
        let permit = authority
            .prepare_finalize(10, ThermodynamicConsequenceStatus::Executed, &journal)
            .unwrap();
        assert_eq!(
            authority.prepare_finalize(10, ThermodynamicConsequenceStatus::Executed, &journal),
            Err(ThermodynamicTickError::FinalizeInProgress { tick_id: 10 })
        );
        authority.rollback_finalize(permit).unwrap();
        assert_eq!(authority.open_tick_id(), Some(10));
    }

    #[test]
    fn rejected_consequence_still_receipts_exactly_one_tick() {
        let journal = FrictionTransactionJournal::new();
        let mut authority = ThermodynamicTickAuthority::new();
        authority.begin(10, &journal).unwrap();
        let permit = authority
            .prepare_finalize(10, ThermodynamicConsequenceStatus::Rejected, &journal)
            .unwrap();
        authority.validate_prepared_finalize(&permit, &journal).unwrap();
        let receipt = authority.commit_finalize(&permit, &journal).unwrap();
        assert_eq!(receipt.tick_id, 10);
        assert_eq!(receipt.consequence_status, ThermodynamicConsequenceStatus::Rejected);
        assert_eq!(receipt.friction_transactions, journal);
        assert_eq!(authority.last_finalized_receipt(), Some(&receipt));
    }

    #[test]
    fn duplicate_finalize_after_commit_is_rejected() {
        let journal = FrictionTransactionJournal::new();
        let mut authority = ThermodynamicTickAuthority::new();
        authority.begin(10, &journal).unwrap();
        let permit = authority
            .prepare_finalize(10, ThermodynamicConsequenceStatus::Executed, &journal)
            .unwrap();
        let receipt = authority.commit_finalize(&permit, &journal).unwrap();
        assert_eq!(
            authority.commit_finalize(&permit, &journal),
            Err(ThermodynamicTickError::StaleFinalizePermit)
        );
        assert_eq!(authority.last_finalized_receipt(), Some(&receipt));
        assert_eq!(
            authority.prepare_finalize(10, ThermodynamicConsequenceStatus::Executed, &journal),
            Err(ThermodynamicTickError::AlreadyFinalized { tick_id: 10 })
        );
    }

    #[test]
    fn stale_finalize_cannot_close_newer_tick() {
        let journal = FrictionTransactionJournal::new();
        let mut authority = ThermodynamicTickAuthority::new();
        authority.begin(10, &journal).unwrap();
        let permit = authority
            .prepare_finalize(10, ThermodynamicConsequenceStatus::Executed, &journal)
            .unwrap();
        authority.commit_finalize(&permit, &journal).unwrap();
        authority.begin(11, &journal).unwrap();
        assert_eq!(
            authority.prepare_finalize(10, ThermodynamicConsequenceStatus::Executed, &journal),
            Err(ThermodynamicTickError::WrongOpenTick {
                requested_tick_id: 10,
                open_tick_id: 11,
            })
        );
    }

    #[test]
    fn pending_friction_application_blocks_finalize() {
        let empty = FrictionTransactionJournal::new();
        let mut authority = ThermodynamicTickAuthority::new();
        authority.begin(10, &empty).unwrap();
        let (mut a, mut b) = pair();
        let mut journal = FrictionTransactionJournal::new();
        let _applied = apply_friction_impulse_once(
            &mut a,
            &mut b,
            &SVector::zeros(),
            &SVector::from([0.5, 0.0, 0.0]),
            FrictionTransactionId::new(10, 0, 0, 0),
            &mut journal,
        )
        .unwrap();
        assert_eq!(
            authority.prepare_finalize(10, ThermodynamicConsequenceStatus::Executed, &journal),
            Err(ThermodynamicTickError::PendingFrictionTransactions { count: 1 })
        );
        assert_eq!(authority.open_tick_id(), Some(10));
    }

    #[test]
    fn terminal_cross_tick_friction_is_rejected() {
        let empty = FrictionTransactionJournal::new();
        let mut authority = ThermodynamicTickAuthority::new();
        authority.begin(10, &empty).unwrap();
        let journal = off_center_terminal_journal(11);
        assert!(journal.is_complete_for_finalize());
        assert_eq!(
            authority.prepare_finalize(10, ThermodynamicConsequenceStatus::Executed, &journal),
            Err(ThermodynamicTickError::CrossTickFrictionTransactions {
                expected_tick_id: 10,
            })
        );
    }

    #[test]
    fn terminal_same_tick_friction_is_embedded_in_receipt() {
        let empty = FrictionTransactionJournal::new();
        let mut authority = ThermodynamicTickAuthority::new();
        authority.begin(10, &empty).unwrap();
        let journal = off_center_terminal_journal(10);
        assert_eq!(journal.pending_application_count(), 0);
        let permit = authority
            .prepare_finalize(10, ThermodynamicConsequenceStatus::Executed, &journal)
            .unwrap();
        let receipt = authority.commit_finalize(&permit, &journal).unwrap();
        assert_eq!(receipt.friction_transactions, journal);
        assert_eq!(
            receipt
                .friction_transactions
                .phase(FrictionTransactionId::new(10, 0, 0, 0)),
            Some(FrictionTransactionPhase::DiagnosticOnly(
                FrictionDiagnosticReason::OffCenterUnqualified
            ))
        );
    }

    #[test]
    fn journal_mutation_after_prepare_can_be_rolled_back_to_open() {
        let mut authority = ThermodynamicTickAuthority::new();
        let mut journal = FrictionTransactionJournal::new();
        authority.begin(10, &journal).unwrap();
        let permit = authority
            .prepare_finalize(10, ThermodynamicConsequenceStatus::Executed, &journal)
            .unwrap();

        journal = off_center_terminal_journal(10);
        assert_eq!(
            authority.validate_prepared_finalize(&permit, &journal),
            Err(ThermodynamicTickError::FrictionJournalChangedAfterPrepare)
        );
        assert_eq!(
            authority.commit_finalize(&permit, &journal),
            Err(ThermodynamicTickError::FrictionJournalChangedAfterPrepare)
        );
        authority.rollback_finalize(permit).unwrap();
        assert_eq!(authority.open_tick_id(), Some(10));
        assert!(!authority.is_finalize_in_progress());
    }

    #[test]
    fn dirty_previous_tick_journal_blocks_next_begin() {
        let dirty = off_center_terminal_journal(9);
        let mut authority = ThermodynamicTickAuthority::new();
        assert_eq!(
            authority.begin(10, &dirty),
            Err(ThermodynamicTickError::DirtyFrictionJournal)
        );
    }

    #[test]
    fn begin_is_monotonic_after_finalize() {
        let journal = FrictionTransactionJournal::new();
        let mut authority = ThermodynamicTickAuthority::new();
        authority.begin(10, &journal).unwrap();
        let permit = authority
            .prepare_finalize(
                10,
                ThermodynamicConsequenceStatus::IntentionallyAbsent,
                &journal,
            )
            .unwrap();
        authority.commit_finalize(&permit, &journal).unwrap();
        assert_eq!(
            authority.begin(9, &journal),
            Err(ThermodynamicTickError::NonMonotonicBegin {
                requested_tick_id: 9,
                last_finalized_tick_id: 10,
            })
        );
        assert_eq!(
            authority.begin(10, &journal),
            Err(ThermodynamicTickError::AlreadyFinalized { tick_id: 10 })
        );
        let next = authority.begin(11, &journal).unwrap();
        assert_eq!(next.generation(), 1);
    }
}