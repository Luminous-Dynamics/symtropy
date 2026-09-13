// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Runtime facade joining fixed-tick thermodynamic authority to friction transactions.
//!
//! The friction journal and canonical physical energy-transfer ledger are privately
//! owned. Solver-facing APIs mint `FrictionTransactionId.fixed_tick` from the
//! currently open thermodynamic tick, so callers cannot relabel mechanical evidence
//! across ticks or replace pending lifecycle/accounting state.

use std::collections::BTreeSet;

use bevy::prelude::Resource;
use nalgebra::SVector;
use symtropy_physics::{
    AppliedFrictionTransaction, BodyHandle, EnergyTransfer, EnergyTransferLedger,
    FrictionApplicationError, FrictionDiagnosticFinalizeError, FrictionDiagnosticReason,
    FrictionEvidenceError, FrictionPromotionError, FrictionPromotionReceipt,
    FrictionSolverCoordinates, FrictionTransactionId, FrictionTransactionJournal,
    HeatPartition, RigidBody, apply_friction_impulse_once, finalize_friction_diagnostic,
    promote_applied_friction_loss_to_heat,
};

use super::thermodynamic_transaction::{
    ThermodynamicBeginPermit, ThermodynamicConsequenceStatus, ThermodynamicFinalizePermit,
    ThermodynamicTickAuthority, ThermodynamicTickError, ThermodynamicTickReceipt,
};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RuntimeFrictionGateError {
    NoOpenTick,
    FinalizeInProgress,
    WrongFixedTick {
        open_tick_id: u64,
        transaction_tick_id: u64,
    },
    UnknownReservation {
        transaction_id: FrictionTransactionId,
    },
    ReservationBodyMismatch {
        expected_a: BodyHandle,
        expected_b: BodyHandle,
        actual_a: BodyHandle,
        actual_b: BodyHandle,
    },
}

#[derive(Debug)]
pub enum RuntimeFrictionError {
    Gate(RuntimeFrictionGateError),
    Application(FrictionApplicationError),
    Diagnostic(FrictionDiagnosticFinalizeError),
    Promotion(FrictionPromotionError),
}

impl From<FrictionApplicationError> for RuntimeFrictionError {
    fn from(value: FrictionApplicationError) -> Self { Self::Application(value) }
}
impl From<FrictionDiagnosticFinalizeError> for RuntimeFrictionError {
    fn from(value: FrictionDiagnosticFinalizeError) -> Self { Self::Diagnostic(value) }
}
impl From<FrictionPromotionError> for RuntimeFrictionError {
    fn from(value: FrictionPromotionError) -> Self { Self::Promotion(value) }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ThermodynamicRuntimeError {
    TickIdExhausted,
    PendingFrictionReservations { count: usize },
    PhysicalLedgerIntervalStillOpen,
    MissingPhysicalLedgerInterval,
    Tick(ThermodynamicTickError),
}
impl From<ThermodynamicTickError> for ThermodynamicRuntimeError {
    fn from(value: ThermodynamicTickError) -> Self { Self::Tick(value) }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ThermodynamicRuntimeTickReceipt {
    pub tick_id: u64,
    pub generation: u64,
    pub consequence_status: ThermodynamicConsequenceStatus,
    pub friction_transactions: FrictionTransactionJournal,
    pub physical_energy_transfers: Vec<EnergyTransfer>,
}

impl ThermodynamicRuntimeTickReceipt {
    fn from_lifecycle(
        lifecycle: ThermodynamicTickReceipt,
        physical_energy_transfers: Vec<EnergyTransfer>,
    ) -> Self {
        Self {
            tick_id: lifecycle.tick_id,
            generation: lifecycle.generation,
            consequence_status: lifecycle.consequence_status,
            friction_transactions: lifecycle.friction_transactions,
            physical_energy_transfers,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum TerminalFrictionOutcome {
    Promoted(FrictionPromotionReceipt),
    Diagnostic(FrictionDiagnosticReason),
}

/// Receipt proving one solver-facing friction request reached a terminal
/// lifecycle state before control returned to later solver mechanics.
#[derive(Clone, Debug, PartialEq)]
pub struct TerminalFrictionReceipt {
    pub transaction_id: FrictionTransactionId,
    pub outcome: TerminalFrictionOutcome,
}

#[derive(Debug, PartialEq)]
pub struct RuntimeFrictionReservation<const D: usize> {
    transaction_id: FrictionTransactionId,
    body_a: BodyHandle,
    body_b: BodyHandle,
    contact_point: SVector<f64, D>,
    impulse_on_b: SVector<f64, D>,
}

impl<const D: usize> RuntimeFrictionReservation<D> {
    pub const fn transaction_id(&self) -> FrictionTransactionId {
        self.transaction_id
    }
}

#[derive(Resource, Debug)]
pub struct ThermodynamicTransactionRuntime {
    authority: ThermodynamicTickAuthority,
    friction_journal: FrictionTransactionJournal,
    reserved_friction: BTreeSet<FrictionTransactionId>,
    physical_ledger: EnergyTransferLedger,
    physical_tick_start: Option<usize>,
    last_runtime_receipt: Option<ThermodynamicRuntimeTickReceipt>,
    next_tick_id: Option<u64>,
}

impl Default for ThermodynamicTransactionRuntime {
    fn default() -> Self {
        Self {
            authority: ThermodynamicTickAuthority::new(),
            friction_journal: FrictionTransactionJournal::new(),
            reserved_friction: BTreeSet::new(),
            physical_ledger: EnergyTransferLedger::new(),
            physical_tick_start: None,
            last_runtime_receipt: None,
            next_tick_id: Some(0),
        }
    }
}

impl ThermodynamicTransactionRuntime {
    pub fn new() -> Self { Self::default() }
    pub fn next_tick_id(&self) -> Option<u64> { self.next_tick_id }
    pub fn open_tick_id(&self) -> Option<u64> { self.authority.open_tick_id() }
    pub fn friction_journal(&self) -> &FrictionTransactionJournal { &self.friction_journal }
    pub fn pending_friction_reservation_count(&self) -> usize { self.reserved_friction.len() }
    pub fn physical_energy_ledger(&self) -> &EnergyTransferLedger { &self.physical_ledger }
    pub fn last_runtime_receipt(&self) -> Option<&ThermodynamicRuntimeTickReceipt> {
        self.last_runtime_receipt.as_ref()
    }

    #[must_use]
    pub fn pending_friction_reservation_ids(&self) -> Vec<FrictionTransactionId> {
        self.reserved_friction.iter().copied().collect()
    }

    pub fn last_finalized_receipt(&self) -> Option<&ThermodynamicTickReceipt> {
        self.authority.last_finalized_receipt()
    }

    fn require_no_pending_reservations(&self) -> Result<(), ThermodynamicRuntimeError> {
        if self.reserved_friction.is_empty() {
            Ok(())
        } else {
            Err(ThermodynamicRuntimeError::PendingFrictionReservations {
                count: self.reserved_friction.len(),
            })
        }
    }

    pub fn begin_next_tick(
        &mut self,
    ) -> Result<ThermodynamicBeginPermit, ThermodynamicRuntimeError> {
        self.require_no_pending_reservations()?;
        let tick_id = self.next_tick_id.ok_or(ThermodynamicRuntimeError::TickIdExhausted)?;
        if self.authority.open_tick_id().is_none() && self.physical_tick_start.is_some() {
            return Err(ThermodynamicRuntimeError::PhysicalLedgerIntervalStillOpen);
        }
        let permit = self.authority.begin(tick_id, &self.friction_journal)?;
        debug_assert!(self.physical_tick_start.is_none());
        self.physical_tick_start = Some(self.physical_ledger.len());
        Ok(permit)
    }

    fn open_friction_tick(&self) -> Result<u64, RuntimeFrictionError> {
        if self.authority.is_finalize_in_progress() {
            return Err(RuntimeFrictionError::Gate(RuntimeFrictionGateError::FinalizeInProgress));
        }
        self.authority.open_tick_id().ok_or(RuntimeFrictionError::Gate(
            RuntimeFrictionGateError::NoOpenTick,
        ))
    }

    fn require_applied_tick<const D: usize>(
        &self,
        applied: &AppliedFrictionTransaction<D>,
    ) -> Result<u64, RuntimeFrictionError> {
        let open_tick_id = self.open_friction_tick()?;
        let transaction_tick_id = applied.transaction_id().fixed_tick;
        if transaction_tick_id != open_tick_id {
            return Err(RuntimeFrictionError::Gate(RuntimeFrictionGateError::WrongFixedTick {
                open_tick_id,
                transaction_tick_id,
            }));
        }
        Ok(open_tick_id)
    }

    fn require_reserved_tick<const D: usize>(
        &self,
        reservation: &RuntimeFrictionReservation<D>,
    ) -> Result<u64, RuntimeFrictionError> {
        let open_tick_id = self.open_friction_tick()?;
        let transaction_tick_id = reservation.transaction_id.fixed_tick;
        if transaction_tick_id != open_tick_id {
            return Err(RuntimeFrictionError::Gate(RuntimeFrictionGateError::WrongFixedTick {
                open_tick_id,
                transaction_tick_id,
            }));
        }
        if !self.reserved_friction.contains(&reservation.transaction_id) {
            return Err(RuntimeFrictionError::Gate(
                RuntimeFrictionGateError::UnknownReservation {
                    transaction_id: reservation.transaction_id,
                },
            ));
        }
        Ok(open_tick_id)
    }

    pub fn reserve_friction_impulse_at<const D: usize>(
        &mut self,
        body_a: BodyHandle,
        body_b: BodyHandle,
        contact_point: &SVector<f64, D>,
        impulse_on_b: &SVector<f64, D>,
        coordinates: FrictionSolverCoordinates,
    ) -> Result<RuntimeFrictionReservation<D>, RuntimeFrictionError> {
        let fixed_tick = self.open_friction_tick()?;
        let transaction_id = FrictionTransactionId::new(
            fixed_tick,
            coordinates.solver_iteration,
            coordinates.contact_sequence,
            coordinates.point_sequence,
        );

        if self.reserved_friction.contains(&transaction_id)
            || self.friction_journal.phase(transaction_id).is_some()
        {
            return Err(RuntimeFrictionError::Application(
                FrictionApplicationError::DuplicateTransaction,
            ));
        }
        if body_a == body_b {
            return Err(RuntimeFrictionError::Application(
                FrictionApplicationError::Evidence(FrictionEvidenceError::SameBody),
            ));
        }
        if !contact_point.iter().all(|value| value.is_finite()) {
            return Err(RuntimeFrictionError::Application(
                FrictionApplicationError::Evidence(FrictionEvidenceError::NonFiniteContactPoint),
            ));
        }
        if !impulse_on_b.iter().all(|value| value.is_finite()) {
            return Err(RuntimeFrictionError::Application(
                FrictionApplicationError::Evidence(FrictionEvidenceError::NonFiniteImpulse),
            ));
        }

        let inserted = self.reserved_friction.insert(transaction_id);
        debug_assert!(inserted);
        Ok(RuntimeFrictionReservation {
            transaction_id,
            body_a,
            body_b,
            contact_point: *contact_point,
            impulse_on_b: *impulse_on_b,
        })
    }

    pub fn cancel_friction_reservation<const D: usize>(
        &mut self,
        reservation: RuntimeFrictionReservation<D>,
    ) -> Result<(), RuntimeFrictionError> {
        self.require_reserved_tick(&reservation)?;
        let removed = self.reserved_friction.remove(&reservation.transaction_id);
        debug_assert!(removed);
        Ok(())
    }

    pub fn apply_reserved_friction_impulse<const D: usize>(
        &mut self,
        body_a: &mut RigidBody<D>,
        body_b: &mut RigidBody<D>,
        reservation: RuntimeFrictionReservation<D>,
    ) -> Result<AppliedFrictionTransaction<D>, RuntimeFrictionError> {
        self.require_reserved_tick(&reservation)?;
        let transaction_id = reservation.transaction_id;

        if body_a.handle != reservation.body_a || body_b.handle != reservation.body_b {
            let removed = self.reserved_friction.remove(&transaction_id);
            debug_assert!(removed);
            return Err(RuntimeFrictionError::Gate(
                RuntimeFrictionGateError::ReservationBodyMismatch {
                    expected_a: reservation.body_a,
                    expected_b: reservation.body_b,
                    actual_a: body_a.handle,
                    actual_b: body_b.handle,
                },
            ));
        }

        let removed = self.reserved_friction.remove(&transaction_id);
        debug_assert!(removed);
        Ok(apply_friction_impulse_once(
            body_a,
            body_b,
            &reservation.contact_point,
            &reservation.impulse_on_b,
            transaction_id,
            &mut self.friction_journal,
        )?)
    }

    pub fn apply_friction_impulse_at<const D: usize>(
        &mut self,
        body_a: &mut RigidBody<D>,
        body_b: &mut RigidBody<D>,
        contact_point: &SVector<f64, D>,
        impulse_on_b: &SVector<f64, D>,
        coordinates: FrictionSolverCoordinates,
    ) -> Result<AppliedFrictionTransaction<D>, RuntimeFrictionError> {
        let reservation = self.reserve_friction_impulse_at(
            body_a.handle,
            body_b.handle,
            contact_point,
            impulse_on_b,
            coordinates,
        )?;
        self.apply_reserved_friction_impulse(body_a, body_b, reservation)
    }

    pub fn apply_friction_impulse<const D: usize>(
        &mut self,
        body_a: &mut RigidBody<D>,
        body_b: &mut RigidBody<D>,
        contact_point: &SVector<f64, D>,
        impulse_on_b: &SVector<f64, D>,
        solver_iteration: u32,
        contact_sequence: u32,
        point_sequence: u32,
    ) -> Result<AppliedFrictionTransaction<D>, RuntimeFrictionError> {
        self.apply_friction_impulse_at(
            body_a,
            body_b,
            contact_point,
            impulse_on_b,
            FrictionSolverCoordinates::new(
                solver_iteration,
                contact_sequence,
                point_sequence,
            ),
        )
    }

    pub fn finalize_friction_diagnostic<const D: usize>(
        &mut self,
        body_a: &RigidBody<D>,
        body_b: &RigidBody<D>,
        applied: &AppliedFrictionTransaction<D>,
    ) -> Result<FrictionDiagnosticReason, RuntimeFrictionError> {
        self.require_applied_tick(applied)?;
        Ok(finalize_friction_diagnostic(
            body_a,
            body_b,
            applied,
            &mut self.friction_journal,
        )?)
    }

    pub fn promote_friction_loss_owned<const D: usize>(
        &mut self,
        body_a: &mut RigidBody<D>,
        body_b: &mut RigidBody<D>,
        applied: &AppliedFrictionTransaction<D>,
        partition: HeatPartition,
    ) -> Result<FrictionPromotionReceipt, RuntimeFrictionError> {
        self.require_applied_tick(applied)?;
        Ok(promote_applied_friction_loss_to_heat(
            body_a,
            body_b,
            applied,
            partition,
            &mut self.physical_ledger,
            &mut self.friction_journal,
        )?)
    }

    /// Atomically execute one friction transaction through a terminal lifecycle
    /// state before returning control to later solver mechanics.
    ///
    /// If terminalization fails, every field touched by this transaction is
    /// restored: A/B linear and angular velocity, A/B thermal state, private
    /// friction journal, private physical ledger, and reservation registry.
    /// Positions and other body fields are not touched by friction mechanics.
    pub fn execute_terminal_friction_impulse_at<const D: usize>(
        &mut self,
        body_a: &mut RigidBody<D>,
        body_b: &mut RigidBody<D>,
        contact_point: &SVector<f64, D>,
        impulse_on_b: &SVector<f64, D>,
        coordinates: FrictionSolverCoordinates,
        partition: HeatPartition,
    ) -> Result<TerminalFrictionReceipt, RuntimeFrictionError> {
        let original_linear_a = body_a.linear_velocity;
        let original_linear_b = body_b.linear_velocity;
        let original_angular_a = body_a.angular_velocity;
        let original_angular_b = body_b.angular_velocity;
        let original_thermal_a = body_a.thermal;
        let original_thermal_b = body_b.thermal;
        let original_journal = self.friction_journal.clone();
        let original_ledger = self.physical_ledger.clone();
        let original_reserved = self.reserved_friction.clone();

        let result = (|| {
            let applied = self.apply_friction_impulse_at(
                body_a,
                body_b,
                contact_point,
                impulse_on_b,
                coordinates,
            )?;
            let transaction_id = applied.transaction_id();

            let outcome = if applied
                .observation()
                .centered_promotable_loss_candidate_joules()
                .is_some()
            {
                TerminalFrictionOutcome::Promoted(self.promote_friction_loss_owned(
                    body_a,
                    body_b,
                    &applied,
                    partition,
                )?)
            } else {
                TerminalFrictionOutcome::Diagnostic(self.finalize_friction_diagnostic(
                    body_a,
                    body_b,
                    &applied,
                )?)
            };

            Ok(TerminalFrictionReceipt {
                transaction_id,
                outcome,
            })
        })();

        if result.is_err() {
            body_a.linear_velocity = original_linear_a;
            body_b.linear_velocity = original_linear_b;
            body_a.angular_velocity = original_angular_a;
            body_b.angular_velocity = original_angular_b;
            body_a.thermal = original_thermal_a;
            body_b.thermal = original_thermal_b;
            self.friction_journal = original_journal;
            self.physical_ledger = original_ledger;
            self.reserved_friction = original_reserved;
        }

        result
    }

    #[cfg(test)]
    pub(crate) fn promote_friction_loss<const D: usize>(
        &mut self,
        body_a: &mut RigidBody<D>,
        body_b: &mut RigidBody<D>,
        applied: &AppliedFrictionTransaction<D>,
        partition: HeatPartition,
        ledger: &mut EnergyTransferLedger,
    ) -> Result<FrictionPromotionReceipt, RuntimeFrictionError> {
        self.require_applied_tick(applied)?;
        Ok(promote_applied_friction_loss_to_heat(
            body_a,
            body_b,
            applied,
            partition,
            ledger,
            &mut self.friction_journal,
        )?)
    }

    pub fn prepare_finalize(
        &mut self,
        consequence_status: ThermodynamicConsequenceStatus,
    ) -> Result<ThermodynamicFinalizePermit, ThermodynamicRuntimeError> {
        self.require_no_pending_reservations()?;
        let tick_id = self.authority.open_tick_id().ok_or(ThermodynamicRuntimeError::Tick(
            ThermodynamicTickError::NoOpenTick,
        ))?;
        if self.physical_tick_start.is_none() {
            return Err(ThermodynamicRuntimeError::MissingPhysicalLedgerInterval);
        }
        Ok(self.authority.prepare_finalize(
            tick_id,
            consequence_status,
            &self.friction_journal,
        )?)
    }

    pub fn validate_prepared_finalize(
        &self,
        permit: &ThermodynamicFinalizePermit,
    ) -> Result<(), ThermodynamicRuntimeError> {
        self.require_no_pending_reservations()?;
        if self.physical_tick_start.is_none() {
            return Err(ThermodynamicRuntimeError::MissingPhysicalLedgerInterval);
        }
        Ok(self.authority.validate_prepared_finalize(permit, &self.friction_journal)?)
    }

    pub fn rollback_finalize(
        &mut self,
        permit: ThermodynamicFinalizePermit,
    ) -> Result<(), ThermodynamicRuntimeError> {
        Ok(self.authority.rollback_finalize(permit)?)
    }

    pub fn commit_finalize_and_rotate(
        &mut self,
        permit: &ThermodynamicFinalizePermit,
    ) -> Result<ThermodynamicRuntimeTickReceipt, ThermodynamicRuntimeError> {
        self.require_no_pending_reservations()?;
        self.authority.validate_prepared_finalize(permit, &self.friction_journal)?;
        let start = self
            .physical_tick_start
            .ok_or(ThermodynamicRuntimeError::MissingPhysicalLedgerInterval)?;
        let physical_energy_transfers = self.physical_ledger.entries()[start..].to_vec();

        let committed_tick = permit.tick_id();
        let lifecycle = self.authority.commit_finalize(permit, &self.friction_journal)?;
        let receipt = ThermodynamicRuntimeTickReceipt::from_lifecycle(
            lifecycle,
            physical_energy_transfers,
        );

        self.friction_journal = FrictionTransactionJournal::new();
        self.physical_tick_start = None;
        self.last_runtime_receipt = Some(receipt.clone());
        debug_assert!(self.reserved_friction.is_empty());
        self.next_tick_id = committed_tick.checked_add(1);
        Ok(receipt)
    }
}
