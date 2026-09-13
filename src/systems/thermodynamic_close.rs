// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Consequence-time close for one operational thermodynamic interval.
//!
//! This helper contains only post-consequence collapse, legacy operational
//! telemetry close, and HUD sampling. Fixed-tick admission/finalize identity is
//! owned separately by `ThermodynamicTransactionRuntime`.

use symtropy_physics::BodyHandle;

use crate::resources::{PhysicsWorldRes, SafetyTier};
use super::thermodynamic::ThermodynamicHudState;

#[derive(Clone, Debug, PartialEq)]
pub struct OperationalThermodynamicCloseReceipt {
    pub canonical_handles: Vec<BodyHandle>,
    pub collapsed_handles: Vec<BodyHandle>,
    pub sampled_consumed: f64,
    pub sampled_regenerated: f64,
    pub hud_ticks_before: u32,
    pub hud_ticks_after: u32,
    pub rate_window_completed: bool,
    pub consumed_per_sec_after: f64,
    pub regenerated_per_sec_after: f64,
    /// Legacy operational telemetry tick identity only; not physical energy proof.
    pub legacy_tick_count_before: u64,
    pub legacy_tick_count_after: u64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OperationalThermodynamicCloseError {
    DuplicateHandle(BodyHandle),
    MissingOperationalEntity(BodyHandle),
    InvalidHudState,
    InvalidEntityCounters(BodyHandle),
    InvalidLegacyLedgerState,
    LegacyTickCounterExhausted,
    UnrepresentableAccumulator,
    HudTickOverflow,
}

#[derive(Copy, Clone)]
struct StagedHud {
    energy_consumed_accumulator: f64,
    energy_regenerated_accumulator: f64,
    ticks_accumulated: u32,
    consumed_per_sec: f64,
    regenerated_per_sec: f64,
    rate_window_completed: bool,
}

fn finite_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn canonicalize_handles(
    handles: &[BodyHandle],
) -> Result<Vec<BodyHandle>, OperationalThermodynamicCloseError> {
    let mut canonical = handles.to_vec();
    canonical.sort_unstable();
    if let Some(duplicate) = canonical.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(OperationalThermodynamicCloseError::DuplicateHandle(duplicate[0]));
    }
    Ok(canonical)
}

fn validate_legacy_ledger(
    physics: &PhysicsWorldRes,
) -> Result<u64, OperationalThermodynamicCloseError> {
    let ledger = &physics.consciousness.ledger;
    if ledger.tick_count == u64::MAX {
        return Err(OperationalThermodynamicCloseError::LegacyTickCounterExhausted);
    }
    if !finite_nonnegative(ledger.energy_in)
        || !finite_nonnegative(ledger.energy_out)
        || !finite_nonnegative(ledger.boundary_in)
        || !finite_nonnegative(ledger.boundary_out)
        || !ledger.phi_energy_integral.is_finite()
        || !finite_nonnegative(ledger.phi_change_total)
        || !finite_nonnegative(ledger.lifetime_energy)
        || !finite_nonnegative(ledger.lifetime_error)
        || !finite_nonnegative(ledger.lifetime_boundary_in)
        || !finite_nonnegative(ledger.lifetime_boundary_out)
    {
        return Err(OperationalThermodynamicCloseError::InvalidLegacyLedgerState);
    }

    let error = (ledger.energy_in - ledger.energy_out).abs();
    let next_values = [
        ledger.lifetime_energy + ledger.energy_in,
        ledger.lifetime_error + error,
        ledger.lifetime_boundary_in + ledger.boundary_in,
        ledger.lifetime_boundary_out + ledger.boundary_out,
    ];
    if !error.is_finite() || !next_values.iter().all(|value| finite_nonnegative(*value)) {
        return Err(OperationalThermodynamicCloseError::InvalidLegacyLedgerState);
    }
    Ok(ledger.tick_count)
}

fn stage_hud(
    hud: &ThermodynamicHudState,
    sampled_consumed: f64,
    sampled_regenerated: f64,
) -> Result<StagedHud, OperationalThermodynamicCloseError> {
    if !finite_nonnegative(hud.energy_consumed_accumulator)
        || !finite_nonnegative(hud.energy_regenerated_accumulator)
        || !finite_nonnegative(hud.consumed_per_sec)
        || !finite_nonnegative(hud.regenerated_per_sec)
    {
        return Err(OperationalThermodynamicCloseError::InvalidHudState);
    }

    let accumulated_consumed = hud.energy_consumed_accumulator + sampled_consumed;
    let accumulated_regenerated = hud.energy_regenerated_accumulator + sampled_regenerated;
    if !finite_nonnegative(accumulated_consumed)
        || !finite_nonnegative(accumulated_regenerated)
    {
        return Err(OperationalThermodynamicCloseError::UnrepresentableAccumulator);
    }

    let ticks = hud
        .ticks_accumulated
        .checked_add(1)
        .ok_or(OperationalThermodynamicCloseError::HudTickOverflow)?;

    if ticks >= 16 {
        let seconds = f64::from(ticks) / 64.0;
        let consumed_per_sec = accumulated_consumed / seconds;
        let regenerated_per_sec = accumulated_regenerated / seconds;
        if !finite_nonnegative(consumed_per_sec) || !finite_nonnegative(regenerated_per_sec) {
            return Err(OperationalThermodynamicCloseError::UnrepresentableAccumulator);
        }
        Ok(StagedHud {
            energy_consumed_accumulator: 0.0,
            energy_regenerated_accumulator: 0.0,
            ticks_accumulated: 0,
            consumed_per_sec,
            regenerated_per_sec,
            rate_window_completed: true,
        })
    } else {
        Ok(StagedHud {
            energy_consumed_accumulator: accumulated_consumed,
            energy_regenerated_accumulator: accumulated_regenerated,
            ticks_accumulated: ticks,
            consumed_per_sec: hud.consumed_per_sec,
            regenerated_per_sec: hud.regenerated_per_sec,
            rate_window_completed: false,
        })
    }
}

/// Close one operational interval after all consequential work for the tick.
///
/// All fallible census/counter/HUD/legacy-ledger checks occur before any close
/// mutation. The legacy balance remains compatibility telemetry, not first-law
/// evidence.
pub fn close_operational_thermodynamic_tick(
    physics: &mut PhysicsWorldRes,
    hud: &mut ThermodynamicHudState,
    handles: &[BodyHandle],
) -> Result<OperationalThermodynamicCloseReceipt, OperationalThermodynamicCloseError> {
    let canonical_handles = canonicalize_handles(handles)?;
    let legacy_tick_count_before = validate_legacy_ledger(physics)?;
    let mut collapsed_handles = Vec::new();
    let mut sampled_consumed = 0.0;
    let mut sampled_regenerated = 0.0;

    for &handle in &canonical_handles {
        let entity = physics
            .consciousness
            .entities
            .get(&handle)
            .ok_or(OperationalThermodynamicCloseError::MissingOperationalEntity(handle))?;
        let consumed = entity.energy.consumed_this_tick;
        let regenerated = entity.energy.regenerated_this_tick;
        if !finite_nonnegative(consumed) || !finite_nonnegative(regenerated) {
            return Err(OperationalThermodynamicCloseError::InvalidEntityCounters(handle));
        }
        sampled_consumed += consumed;
        sampled_regenerated += regenerated;
        if !finite_nonnegative(sampled_consumed) || !finite_nonnegative(sampled_regenerated) {
            return Err(OperationalThermodynamicCloseError::UnrepresentableAccumulator);
        }
        if entity.energy.is_collapsed() {
            collapsed_handles.push(handle);
        }
    }

    let staged_hud = stage_hud(hud, sampled_consumed, sampled_regenerated)?;
    let hud_ticks_before = hud.ticks_accumulated;

    for &handle in &collapsed_handles {
        physics.consciousness.entities.get_mut(&handle).expect("preflight proved entity").safety_tier = SafetyTier::Red;
    }

    let _legacy_balance = physics.consciousness.tick_thermodynamics();
    let legacy_tick_count_after = physics.consciousness.ledger.tick_count;
    debug_assert_eq!(legacy_tick_count_after, legacy_tick_count_before + 1);

    hud.energy_consumed_accumulator = staged_hud.energy_consumed_accumulator;
    hud.energy_regenerated_accumulator = staged_hud.energy_regenerated_accumulator;
    hud.ticks_accumulated = staged_hud.ticks_accumulated;
    hud.consumed_per_sec = staged_hud.consumed_per_sec;
    hud.regenerated_per_sec = staged_hud.regenerated_per_sec;

    Ok(OperationalThermodynamicCloseReceipt {
        canonical_handles,
        collapsed_handles,
        sampled_consumed,
        sampled_regenerated,
        hud_ticks_before,
        hud_ticks_after: staged_hud.ticks_accumulated,
        rate_window_completed: staged_hud.rate_window_completed,
        consumed_per_sec_after: staged_hud.consumed_per_sec,
        regenerated_per_sec_after: staged_hud.regenerated_per_sec,
        legacy_tick_count_before,
        legacy_tick_count_after,
    })
}