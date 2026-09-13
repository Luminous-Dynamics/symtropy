// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Runtime facade joining fixed-tick thermodynamic authority to friction transactions.
//!
//! The friction journal is privately owned. Solver-facing APIs mint the
//! `FrictionTransactionId.fixed_tick` from the currently open thermodynamic tick,
//! so callers cannot relabel mechanical evidence across ticks or erase pending
//! journal state by replacing the journal.

use bevy::prelude::Resource;
use nalgebra::SVector;
use symtropy_physics::{
    AppliedFrictionTransaction, EnergyTransferLedger, FrictionApplicationError,
    FrictionDiagnosticFinalizeError, FrictionDiagnosticReason, FrictionPromotionError,
    FrictionPromotionReceipt, FrictionTransactionId, FrictionTransactionJournal, HeatPartition,
    RigidBody, apply_friction_impulse_once, finalize_friction_diagnostic,
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
    Tick(ThermodynamicTickError),
}
impl From<ThermodynamicTickError> for ThermodynamicRuntimeError {
    fn from(value: ThermodynamicTickError) -> Self { Self::Tick(value) }
}

/// Single runtime owner for fixed-tick identity and per-tick friction lifecycle.
#[derive(Resource, Debug)]
pub struct ThermodynamicTransactionRuntime {
    authority: ThermodynamicTickAuthority,
    friction_journal: FrictionTransactionJournal,
    next_tick_id: Option<u64>,
}

impl Default for ThermodynamicTransactionRuntime {
    fn default() -> Self {
        Self {
            authority: ThermodynamicTickAuthority::new(),
            friction_journal: FrictionTransactionJournal::new(),
            next_tick_id: Some(0),
        }
    }
}

impl ThermodynamicTransactionRuntime {
    pub fn new() -> Self { Self::default() }
    pub fn next_tick_id(&self) -> Option<u64> { self.next_tick_id }
    pub fn open_tick_id(&self) -> Option<u64> { self.authority.open_tick_id() }
    pub fn friction_journal(&self) -> &FrictionTransactionJournal { &self.friction_journal }
    pub fn last_finalized_receipt(&self) -> Option<&ThermodynamicTickReceipt> {
        self.authority.last_finalized_receipt()
    }

    pub fn begin_next_tick(
        &mut self,
    ) -> Result<ThermodynamicBeginPermit, ThermodynamicRuntimeError> {
        let tick_id = self.next_tick_id.ok_or(ThermodynamicRuntimeError::TickIdExhausted)?;
        Ok(self.authority.begin(tick_id, &self.friction_journal)?)
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

    /// Apply friction under the open fixed tick. The caller supplies only
    /// solver-local coordinates; this authority supplies the fixed-tick identity.
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
        let fixed_tick = self.open_friction_tick()?;
        let transaction_id = FrictionTransactionId::new(
            fixed_tick,
            solver_iteration,
            contact_sequence,
            point_sequence,
        );
        Ok(apply_friction_impulse_once(
            body_a,
            body_b,
            contact_point,
            impulse_on_b,
            transaction_id,
            &mut self.friction_journal,
        )?)
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

    pub fn promote_friction_loss<const D: usize>(
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
        let tick_id = self.authority.open_tick_id().ok_or(ThermodynamicRuntimeError::Tick(
            ThermodynamicTickError::NoOpenTick,
        ))?;
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
        Ok(self.authority.validate_prepared_finalize(permit, &self.friction_journal)?)
    }

    pub fn rollback_finalize(
        &mut self,
        permit: ThermodynamicFinalizePermit,
    ) -> Result<(), ThermodynamicRuntimeError> {
        Ok(self.authority.rollback_finalize(permit)?)
    }

    /// Commit the receipt and rotate the private friction journal. `u64::MAX`
    /// may be committed as the final tick; afterward no further begin is admitted.
    pub fn commit_finalize_and_rotate(
        &mut self,
        permit: &ThermodynamicFinalizePermit,
    ) -> Result<ThermodynamicTickReceipt, ThermodynamicRuntimeError> {
        self.authority.validate_prepared_finalize(permit, &self.friction_journal)?;
        let committed_tick = permit.tick_id();
        let receipt = self.authority.commit_finalize(permit, &self.friction_journal)?;
        self.friction_journal = FrictionTransactionJournal::new();
        self.next_tick_id = committed_tick.checked_add(1);
        Ok(receipt)
    }
}