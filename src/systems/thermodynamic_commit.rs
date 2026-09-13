// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Composition of fixed-tick lifecycle authority with operational close semantics.

use symtropy_physics::BodyHandle;

use crate::resources::PhysicsWorldRes;

use super::thermodynamic::ThermodynamicHudState;
use super::thermodynamic_close::{
    OperationalThermodynamicCloseError, OperationalThermodynamicCloseReceipt,
    close_operational_thermodynamic_tick,
};
use super::thermodynamic_runtime::{
    ThermodynamicRuntimeError, ThermodynamicTransactionRuntime,
};
use super::thermodynamic_transaction::{
    ThermodynamicConsequenceStatus, ThermodynamicFinalizePermit, ThermodynamicTickReceipt,
};

#[derive(Clone, Debug, PartialEq)]
pub struct CommittedThermodynamicTickReceipt {
    pub transaction: ThermodynamicTickReceipt,
    pub operational_close: OperationalThermodynamicCloseReceipt,
}

/// Non-cloneable continuation for a close that failed preflight after finalize
/// identity/status/friction evidence had already been reserved.
///
/// Keeping the runtime in `Finalizing` prevents the same consequential tick from
/// being reopened and relabeled with a different `ThermodynamicConsequenceStatus`.
#[derive(Debug)]
pub struct PreparedOperationalCloseRetry {
    permit: ThermodynamicFinalizePermit,
    error: OperationalThermodynamicCloseError,
}

impl PreparedOperationalCloseRetry {
    pub const fn error(&self) -> OperationalThermodynamicCloseError {
        self.error
    }

    pub const fn tick_id(&self) -> u64 {
        self.permit.tick_id()
    }
}

#[derive(Debug)]
pub enum ThermodynamicCommitError {
    Runtime(ThermodynamicRuntimeError),
    OperationalClose(PreparedOperationalCloseRetry),
}

impl From<ThermodynamicRuntimeError> for ThermodynamicCommitError {
    fn from(value: ThermodynamicRuntimeError) -> Self {
        Self::Runtime(value)
    }
}

fn finish_prepared_close(
    runtime: &mut ThermodynamicTransactionRuntime,
    physics: &mut PhysicsWorldRes,
    hud: &mut ThermodynamicHudState,
    handles: &[BodyHandle],
    permit: ThermodynamicFinalizePermit,
) -> Result<CommittedThermodynamicTickReceipt, ThermodynamicCommitError> {
    runtime
        .validate_prepared_finalize(&permit)
        .expect("exclusive prepared runtime must retain its exact friction snapshot");

    let operational_close = match close_operational_thermodynamic_tick(physics, hud, handles) {
        Ok(receipt) => receipt,
        Err(error) => {
            return Err(ThermodynamicCommitError::OperationalClose(
                PreparedOperationalCloseRetry { permit, error },
            ));
        }
    };

    let transaction = runtime
        .commit_finalize_and_rotate(&permit)
        .expect("exclusive validated runtime cannot change between close and commit");

    Ok(CommittedThermodynamicTickReceipt {
        transaction,
        operational_close,
    })
}

/// Close and commit one fixed thermodynamic transaction.
///
/// Ordering is strictly:
///
/// ```text
/// reserve tick + consequence status + friction snapshot
/// -> validate
/// -> preflight + commit operational close
/// -> commit typed fixed-tick receipt
/// -> rotate private friction journal
/// ```
///
/// If operational-close preflight rejects, the runtime intentionally remains in
/// `Finalizing`. The returned retry token owns the only permit that can complete
/// that exact reservation. Retry cannot supply a replacement consequence status.
/// This prevents a failed close from reopening already-observed history under a
/// more favorable label.
///
/// This is scheduler-level in-memory atomicity only. Process-crash consistency
/// across close/receipt still requires persisted WAL/checkpoint semantics.
pub fn finalize_operational_tick_transaction(
    runtime: &mut ThermodynamicTransactionRuntime,
    physics: &mut PhysicsWorldRes,
    hud: &mut ThermodynamicHudState,
    handles: &[BodyHandle],
    consequence_status: ThermodynamicConsequenceStatus,
) -> Result<CommittedThermodynamicTickReceipt, ThermodynamicCommitError> {
    let permit = runtime.prepare_finalize(consequence_status)?;
    finish_prepared_close(runtime, physics, hud, handles, permit)
}

/// Retry a preflight-rejected close without reopening or relabeling the tick.
pub fn retry_operational_tick_transaction(
    runtime: &mut ThermodynamicTransactionRuntime,
    physics: &mut PhysicsWorldRes,
    hud: &mut ThermodynamicHudState,
    handles: &[BodyHandle],
    retry: PreparedOperationalCloseRetry,
) -> Result<CommittedThermodynamicTickReceipt, ThermodynamicCommitError> {
    finish_prepared_close(runtime, physics, hud, handles, retry.permit)
}