// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Admission authority for the launcher's v0.1 fixed thermodynamic cadence.
//!
//! Bevy 0.19 uses `Time<Fixed>` for fixed schedules and defaults to 64 Hz
//! (15,625 microseconds). The current HUD close semantics still convert a count
//! of 16 fixed ticks to seconds using that exact cadence. Until the HUD stores
//! accumulated fixed duration instead of tick count, any other configured fixed
//! timestep must fail closed rather than silently misreport rates.

use std::time::Duration;

use bevy::prelude::{Fixed, Time};

use super::thermodynamic::ThermodynamicHudState;
use super::thermodynamic_commit::{
    CommittedThermodynamicTickReceipt, ThermodynamicCommitError,
    finalize_operational_tick_transaction,
};
use super::thermodynamic_runtime::{
    ThermodynamicRuntimeError, ThermodynamicTransactionRuntime,
};
use super::thermodynamic_transaction::{
    ThermodynamicBeginPermit, ThermodynamicConsequenceStatus,
};
use crate::resources::PhysicsWorldRes;
use symtropy_physics::BodyHandle;

pub const THERMODYNAMIC_V01_FIXED_TIMESTEP: Duration = Duration::from_micros(15_625);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ThermodynamicCadenceReceipt {
    pub timestep: Duration,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ThermodynamicCadenceError {
    UnsupportedFixedTimestep {
        expected: Duration,
        actual: Duration,
    },
}

pub fn admit_thermodynamic_cadence(
    fixed_time: &Time<Fixed>,
) -> Result<ThermodynamicCadenceReceipt, ThermodynamicCadenceError> {
    let actual = fixed_time.timestep();
    if actual != THERMODYNAMIC_V01_FIXED_TIMESTEP {
        return Err(ThermodynamicCadenceError::UnsupportedFixedTimestep {
            expected: THERMODYNAMIC_V01_FIXED_TIMESTEP,
            actual,
        });
    }
    Ok(ThermodynamicCadenceReceipt { timestep: actual })
}

#[derive(Debug)]
pub enum CadenceBoundBeginError {
    Cadence(ThermodynamicCadenceError),
    Runtime(ThermodynamicRuntimeError),
}

#[derive(Debug)]
pub enum CadenceBoundCommitError {
    Cadence(ThermodynamicCadenceError),
    Commit(ThermodynamicCommitError),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CadenceBoundThermodynamicTickReceipt {
    pub cadence: ThermodynamicCadenceReceipt,
    pub committed: CommittedThermodynamicTickReceipt,
}

/// Admit cadence before `begin`, so unsupported scheduling cannot reset counters
/// or apply tick-start operational flows under a cadence the current HUD cannot
/// represent honestly.
pub fn begin_thermodynamic_tick_at_admitted_cadence(
    runtime: &mut ThermodynamicTransactionRuntime,
    fixed_time: &Time<Fixed>,
) -> Result<ThermodynamicBeginPermit, CadenceBoundBeginError> {
    admit_thermodynamic_cadence(fixed_time).map_err(CadenceBoundBeginError::Cadence)?;
    runtime
        .begin_next_tick()
        .map_err(CadenceBoundBeginError::Runtime)
}

/// Re-admit the exact fixed cadence before consequence-time close, then bind that
/// cadence into the committed receipt.
///
/// Because v0.1 admits exactly one timestep, successful admission at begin and
/// close implies identical cadence. If configuration changes mid-interval to an
/// unsupported timestep, this function rejects before `prepare_finalize` or any
/// HUD/legacy-close mutation. The tick remains open until the invalid cadence is
/// corrected or an explicit future abort policy is implemented.
pub fn finalize_thermodynamic_tick_at_admitted_cadence(
    runtime: &mut ThermodynamicTransactionRuntime,
    physics: &mut PhysicsWorldRes,
    hud: &mut ThermodynamicHudState,
    handles: &[BodyHandle],
    consequence_status: ThermodynamicConsequenceStatus,
    fixed_time: &Time<Fixed>,
) -> Result<CadenceBoundThermodynamicTickReceipt, CadenceBoundCommitError> {
    let cadence = admit_thermodynamic_cadence(fixed_time)
        .map_err(CadenceBoundCommitError::Cadence)?;
    let committed = finalize_operational_tick_transaction(
        runtime,
        physics,
        hud,
        handles,
        consequence_status,
    )
    .map_err(CadenceBoundCommitError::Commit)?;

    Ok(CadenceBoundThermodynamicTickReceipt { cadence, committed })
}
