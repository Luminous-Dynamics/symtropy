// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! FixedUpdate adapter for the exactly-once thermodynamic transaction stack.
//!
//! This module is deliberately schedule-facing rather than a second energy model.
//! It preserves the current operational maintenance/offload/well policy, but splits
//! one fixed interval into explicit authority phases:
//!
//! `admit + operational begin -> one consequence -> close/receipt -> representation`.
//!
//! If close is delayed, the exact admitted control-volume census, consequence mode,
//! consequence status, and (when applicable) non-cloneable prepared-close token are
//! retained across FixedUpdate frames. Physics is never re-stepped merely because
//! close evidence was temporarily unavailable.

use std::collections::HashMap;

use bevy::prelude::*;
use symtropy_physics::BodyHandle;
use symtropy_render_bridge::PhysicsBody;

use crate::components::{CrewNpc, Player};
use crate::resources::{EnergyWell, GamePhase, PhysicsWorldRes, SafetyTier};

use super::engine_physics::step_physics_world;
use super::thermodynamic::ThermodynamicHudState;
use super::thermodynamic_cadence::{
    CadenceBoundCommitError, CadenceBoundRetryError, CadenceBoundThermodynamicTickReceipt,
    admit_thermodynamic_cadence, begin_thermodynamic_tick_at_admitted_cadence,
    finalize_thermodynamic_tick_at_admitted_cadence,
    retry_thermodynamic_tick_at_admitted_cadence,
};
use super::thermodynamic_commit::{PreparedOperationalCloseRetry, ThermodynamicCommitError};
use super::thermodynamic_runtime::ThermodynamicTransactionRuntime;
use super::thermodynamic_transaction::ThermodynamicConsequenceStatus;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ThermodynamicScheduleFaultKind {
    BeginRejected,
    BeginCensus,
    CadenceBeforeConsequence,
    CensusBeforeConsequence,
    FinalizeCadence,
    FinalizeRuntime,
    OperationalClose,
    RetryCadence,
    RetryOperationalClose,
    RetryInvariant,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ThermodynamicScheduleFault {
    pub tick_id: Option<u64>,
    pub kind: ThermodynamicScheduleFaultKind,
}

/// Consequence authority is frozen at begin rather than re-read after gameplay
/// state transitions. A delayed 2D tick therefore cannot silently become a 3D
/// intentionally-absent tick, or vice versa.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum BoundConsequenceMode {
    Dynamic2d,
    Kinematic3d,
}

impl BoundConsequenceMode {
    fn from_phase(phase: GamePhase) -> Option<Self> {
        match phase {
            GamePhase::Playing => Some(Self::Dynamic2d),
            GamePhase::Playing3D => Some(Self::Kinematic3d),
            _ => None,
        }
    }
}

#[derive(Debug)]
enum PendingThermodynamicTick {
    /// Begin-phase operational mutation happened, but consequence-time processing
    /// has not yet run. Cadence/census admission is repeated before consequence.
    AwaitingConsequence,
    /// Consequence already happened exactly once; open-tick finalize could not yet
    /// reserve/commit (for example because friction remains non-terminal).
    AwaitingOpenClose {
        consequence_status: ThermodynamicConsequenceStatus,
        export_2d_on_commit: bool,
    },
    /// The fixed-tick runtime already reserved exact status/friction identity and
    /// operational close preflight rejected. This owns the only continuation.
    AwaitingPreparedClose {
        retry: PreparedOperationalCloseRetry,
        export_2d_on_commit: bool,
    },
    /// An internal invariant consumed unique continuation authority without a
    /// replacement. No new begin or consequence is admitted afterward.
    Poisoned,
}

/// Runtime scheduler state. Transaction-resume internals are private so ordinary
/// callers cannot relabel consequence status, replace the census, or skip retry.
#[derive(Resource, Debug, Default)]
pub struct ThermodynamicScheduleState {
    pending: Option<PendingThermodynamicTick>,
    active_handles: Option<Vec<BodyHandle>>,
    active_mode: Option<BoundConsequenceMode>,
    last_receipt: Option<CadenceBoundThermodynamicTickReceipt>,
    last_fault: Option<ThermodynamicScheduleFault>,
    fault_count: u64,
}

impl ThermodynamicScheduleState {
    pub fn has_pending_transaction(&self) -> bool {
        self.pending.is_some()
    }

    pub fn active_handles(&self) -> Option<&[BodyHandle]> {
        self.active_handles.as_deref()
    }

    pub fn last_receipt(&self) -> Option<&CadenceBoundThermodynamicTickReceipt> {
        self.last_receipt.as_ref()
    }

    pub const fn last_fault(&self) -> Option<ThermodynamicScheduleFault> {
        self.last_fault
    }

    pub const fn fault_count(&self) -> u64 {
        self.fault_count
    }

    fn note_fault(&mut self, tick_id: Option<u64>, kind: ThermodynamicScheduleFaultKind) {
        self.last_fault = Some(ThermodynamicScheduleFault { tick_id, kind });
        self.fault_count = self.fault_count.saturating_add(1);
    }

    fn commit_receipt(&mut self, receipt: CadenceBoundThermodynamicTickReceipt) {
        self.pending = None;
        self.active_handles = None;
        self.active_mode = None;
        self.last_receipt = Some(receipt);
    }

    fn poison(&mut self, tick_id: Option<u64>) {
        self.note_fault(tick_id, ThermodynamicScheduleFaultKind::RetryInvariant);
        self.pending = Some(PendingThermodynamicTick::Poisoned);
    }
}

fn canonicalize_handles<I>(handles: I) -> Result<Vec<BodyHandle>, BodyHandle>
where
    I: IntoIterator<Item = BodyHandle>,
{
    let mut canonical: Vec<_> = handles.into_iter().collect();
    canonical.sort_unstable();
    if let Some(pair) = canonical.windows(2).find(|pair| pair[0] == pair[1]) {
        return Err(pair[0]);
    }
    Ok(canonical)
}

fn live_census_matches(
    agent_query: &Query<&PhysicsBody, Or<(With<Player>, With<CrewNpc>)>>,
    admitted: &[BodyHandle],
) -> bool {
    let Ok(current) = canonicalize_handles(agent_query.iter().map(|body| body.handle)) else {
        return false;
    };
    current == admitted
}

fn harmony_resonance(a: &[f64; 9], b: &[f64; 9]) -> f64 {
    let dot = a.iter().zip(b).map(|(a, b)| a * b).sum::<f64>();
    let mag_a = a.iter().map(|v| v * v).sum::<f64>().sqrt();
    let mag_b = b.iter().map(|v| v * v).sum::<f64>().sqrt();

    if !dot.is_finite()
        || !mag_a.is_finite()
        || !mag_b.is_finite()
        || mag_a <= 1e-10
        || mag_b <= 1e-10
    {
        return 0.0;
    }

    let denominator = mag_a * mag_b;
    if !denominator.is_finite() || denominator <= 1e-20 {
        return 0.0;
    }

    let resonance = dot / denominator;
    if resonance.is_finite() {
        resonance.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn accumulate_strongest_factor(
    factors: &mut HashMap<BodyHandle, f64>,
    handle: BodyHandle,
    factor: f64,
) {
    if !factor.is_finite() || factor <= 0.0 {
        return;
    }
    let factor = factor.clamp(0.0, 1.0);
    factors
        .entry(handle)
        .and_modify(|current| *current = current.max(factor))
        .or_insert(factor);
}

fn epistemic_offload_factors(
    physics: &PhysicsWorldRes,
    handles: &[BodyHandle],
    range: f64,
) -> HashMap<BodyHandle, f64> {
    let mut factors = HashMap::new();
    if !range.is_finite() || range <= 0.0 {
        return factors;
    }

    for i in 0..handles.len() {
        for j in (i + 1)..handles.len() {
            let ha = handles[i];
            let hb = handles[j];

            let (pos_a, harmonies_a) = {
                let sanctuary_a = physics.consciousness.sanctuaries.get(&ha);
                let entity_a = physics.consciousness.entities.get(&ha);
                match (sanctuary_a, entity_a) {
                    (Some(s), Some(e)) => (s.center, e.harmony_activations),
                    _ => continue,
                }
            };
            let (pos_b, harmonies_b) = {
                let sanctuary_b = physics.consciousness.sanctuaries.get(&hb);
                let entity_b = physics.consciousness.entities.get(&hb);
                match (sanctuary_b, entity_b) {
                    (Some(s), Some(e)) => (s.center, e.harmony_activations),
                    _ => continue,
                }
            };

            let dist = pos_a.distance(&pos_b);
            if !dist.is_finite() || dist > range {
                continue;
            }

            let resonance = harmony_resonance(&harmonies_a, &harmonies_b);
            if resonance <= 0.5 {
                continue;
            }

            let factor = ((resonance - 0.5) * 2.0).clamp(0.0, 1.0);
            accumulate_strongest_factor(&mut factors, ha, factor);
            accumulate_strongest_factor(&mut factors, hb, factor);
        }
    }

    factors
}

fn maintenance_cost_with_offload(base_cost: f64, phi: f64, offload_factor: f64) -> Option<f64> {
    if !base_cost.is_finite() || base_cost < 0.0 {
        return None;
    }

    let (effective_phi, effective_offload) = if phi.is_finite() {
        let factor = if offload_factor.is_finite() {
            offload_factor.clamp(0.0, 1.0)
        } else {
            0.0
        };
        (phi.clamp(0.0, 1.0), factor)
    } else {
        (1.0, 0.0)
    };

    let raw_cost = base_cost * (1.0 + effective_phi * 0.5);
    let discount = base_cost * effective_offload * 0.5;
    let cost = raw_cost - discount;
    if !raw_cost.is_finite() || !discount.is_finite() || !cost.is_finite() || cost < 0.0 {
        None
    } else {
        Some(cost)
    }
}

fn regenerate_entity_accepted(
    physics: &mut PhysicsWorldRes,
    handle: BodyHandle,
    amount: f64,
) -> f64 {
    let Some(entity) = physics.consciousness.entities.get_mut(&handle) else {
        return 0.0;
    };

    #[cfg(feature = "consciousness-runtime")]
    {
        entity.energy.regenerate(amount)
    }

    #[cfg(not(feature = "consciousness-runtime"))]
    {
        let before = entity.energy.available;
        entity.energy.regenerate(amount);
        let accepted = entity.energy.available - before;
        if accepted.is_finite() && accepted > 0.0 {
            accepted
        } else {
            0.0
        }
    }
}

fn regenerate_live_entity_accepted(
    physics: &mut PhysicsWorldRes,
    handle: BodyHandle,
    amount: f64,
) -> f64 {
    if !amount.is_finite() || amount <= 0.0 {
        return 0.0;
    }
    let Some(entity) = physics.consciousness.entities.get(&handle) else {
        return 0.0;
    };
    if entity.energy.is_collapsed() {
        return 0.0;
    }
    regenerate_entity_accepted(physics, handle, amount)
}

fn transfer_from_finite_source(
    physics: &mut PhysicsWorldRes,
    handle: BodyHandle,
    source_remaining: &mut f64,
    max_offer: f64,
) -> f64 {
    if !source_remaining.is_finite()
        || *source_remaining <= 0.0
        || !max_offer.is_finite()
        || max_offer <= 0.0
    {
        return 0.0;
    }

    let offered = max_offer.min(*source_remaining);
    let accepted = regenerate_entity_accepted(physics, handle, offered);
    if !accepted.is_finite() || accepted <= 0.0 {
        return 0.0;
    }

    let accepted = accepted.min(offered);
    *source_remaining = (*source_remaining - accepted).max(0.0);
    accepted
}

fn apply_operational_begin_flows(
    physics: &mut PhysicsWorldRes,
    agent_data: &[(BodyHandle, Vec3)],
    wells: &mut Query<(&Transform, &mut EnergyWell, &mut Sprite), Without<Player>>,
) {
    let constants = physics.consciousness.constants.clone();
    let handles: Vec<_> = agent_data.iter().map(|(handle, _)| *handle).collect();
    let regen_mult = physics.consciousness.resource_regeneration_multiplier();
    let offload_factors = epistemic_offload_factors(physics, &handles, constants.harmony_range);

    for &handle in &handles {
        {
            // Begin census preflight proves this exists. Keep the conditional as
            // defense in depth against internal mutation between collection/apply.
            let Some(entity) = physics.consciousness.entities.get_mut(&handle) else {
                continue;
            };

            entity.energy.tick_reset();
            let phi = entity.phi();
            let phi_is_valid = phi.is_finite();
            let offload_factor = if phi_is_valid {
                offload_factors.get(&handle).copied().unwrap_or(0.0)
            } else {
                0.0
            };

            if offload_factor > 0.0 {
                if entity.prediction_error.is_finite() {
                    entity.prediction_error *= 1.0 - offload_factor * 0.1;
                    if entity.prediction_error.is_finite() {
                        entity.motor_precision = 1.0 / (1.0 + entity.prediction_error);
                    } else {
                        entity.motor_precision = 0.0;
                        entity.safety_tier = SafetyTier::Red;
                    }
                } else {
                    entity.motor_precision = 0.0;
                    entity.safety_tier = SafetyTier::Red;
                }
            }

            let Some(maintenance) = maintenance_cost_with_offload(
                constants.consciousness_maintenance_per_tick,
                phi,
                offload_factor,
            ) else {
                entity.safety_tier = SafetyTier::Red;
                continue;
            };
            let _ = entity.energy.consume(maintenance);
            if !phi_is_valid {
                entity.safety_tier = SafetyTier::Red;
            }
        }

        let ambient = constants.ambient_regen_rate * regen_mult;
        let _ = regenerate_live_entity_accepted(physics, handle, ambient);
        if let Some(entity) = physics.consciousness.entities.get_mut(&handle)
            && entity.energy.is_collapsed()
        {
            entity.safety_tier = SafetyTier::Red;
        }
    }

    for (well_tf, mut well, mut well_sprite) in wells.iter_mut() {
        if !well.is_active() {
            well_sprite.color = Color::srgba(0.2, 0.2, 0.2, 0.15);
            continue;
        }

        for &(handle, agent_pos) in agent_data {
            let dist = agent_pos
                .truncate()
                .distance(well_tf.translation.truncate());
            if dist.is_finite() && dist < well.radius {
                let regen_rate = well.regen_rate;
                let _ = transfer_from_finite_source(
                    physics,
                    handle,
                    &mut well.remaining,
                    regen_rate,
                );
            }
        }

        let frac = well.fraction_remaining() as f32;
        well_sprite.color = Color::srgba(0.1, 0.8 * frac, 0.6 * frac, 0.2 + 0.3 * frac);
    }

    // Compatibility telemetry only. Physical first-law authority stays in the
    // core typed energy/thermal ledger stack.
    let total_maintenance: f64 = handles
        .iter()
        .filter_map(|handle| physics.consciousness.entities.get(handle))
        .map(|entity| entity.energy.consumed_this_tick)
        .sum();
    physics
        .consciousness
        .ledger
        .record_dissipation(total_maintenance);
}

/// Begin one fixed thermodynamic interval. This system should be gameplay-gated;
/// the finalizer intentionally is not, so an already-open tick can drain after a
/// gameplay state transition.
pub fn transactional_thermodynamic_begin_system(
    mut runtime: ResMut<ThermodynamicTransactionRuntime>,
    mut schedule: ResMut<ThermodynamicScheduleState>,
    fixed_time: Res<Time<Fixed>>,
    phase: Res<State<GamePhase>>,
    mut physics: ResMut<PhysicsWorldRes>,
    entities_query: Query<(&PhysicsBody, &Transform), Or<(With<Player>, With<CrewNpc>)>>,
    mut wells: Query<(&Transform, &mut EnergyWell, &mut Sprite), Without<Player>>,
) {
    if schedule.pending.is_some() || runtime.open_tick_id().is_some() {
        return;
    }

    let Some(bound_mode) = BoundConsequenceMode::from_phase(*phase.get()) else {
        return;
    };

    let mut agent_data: Vec<_> = entities_query
        .iter()
        .map(|(body, transform)| (body.handle, transform.translation))
        .collect();
    agent_data.sort_by_key(|(handle, _)| *handle);

    let handles = match canonicalize_handles(agent_data.iter().map(|(handle, _)| *handle)) {
        Ok(handles) => handles,
        Err(duplicate) => {
            warn!("thermodynamic begin rejected duplicate body handle: {duplicate:?}");
            schedule.note_fault(None, ThermodynamicScheduleFaultKind::BeginCensus);
            return;
        }
    };

    if let Some(handle) = handles
        .iter()
        .find(|handle| !physics.consciousness.entities.contains_key(handle))
    {
        warn!("thermodynamic begin rejected missing operational entity: {handle:?}");
        schedule.note_fault(None, ThermodynamicScheduleFaultKind::BeginCensus);
        return;
    }

    if let Err(error) = begin_thermodynamic_tick_at_admitted_cadence(&mut runtime, &fixed_time) {
        warn!("thermodynamic fixed-tick begin rejected: {error:?}");
        schedule.note_fault(None, ThermodynamicScheduleFaultKind::BeginRejected);
        return;
    }

    schedule.active_handles = Some(handles);
    schedule.active_mode = Some(bound_mode);
    apply_operational_begin_flows(&mut physics, &agent_data, &mut wells);
}

fn export_authoritative_2d_transforms(
    physics: &PhysicsWorldRes,
    query: &mut Query<(&PhysicsBody, &mut Transform)>,
) {
    for (body_component, mut transform) in query.iter_mut() {
        if let Some(body) = physics.world.body(body_component.handle) {
            let position: nalgebra::SVector<f64, 2> = body.position();
            transform.translation.x = position[0] as f32;
            transform.translation.y = position[1] as f32;
        }
    }
}

fn consequence_for_bound_mode(
    mode: BoundConsequenceMode,
    physics: &mut PhysicsWorldRes,
    fixed_time: &Time<Fixed>,
) -> (ThermodynamicConsequenceStatus, bool) {
    match mode {
        BoundConsequenceMode::Dynamic2d => {
            let succeeded = step_physics_world(physics, fixed_time.delta().as_secs_f64());
            (
                if succeeded {
                    ThermodynamicConsequenceStatus::Executed
                } else {
                    ThermodynamicConsequenceStatus::Rejected
                },
                succeeded,
            )
        }
        BoundConsequenceMode::Kinematic3d => (
            ThermodynamicConsequenceStatus::IntentionallyAbsent,
            false,
        ),
    }
}

fn store_open_close_failure(
    schedule: &mut ThermodynamicScheduleState,
    tick_id: u64,
    consequence_status: ThermodynamicConsequenceStatus,
    export_2d_on_commit: bool,
    error: CadenceBoundCommitError,
) {
    match error {
        CadenceBoundCommitError::Cadence(error) => {
            warn!("thermodynamic close cadence rejected: {error:?}");
            schedule.note_fault(Some(tick_id), ThermodynamicScheduleFaultKind::FinalizeCadence);
            schedule.pending = Some(PendingThermodynamicTick::AwaitingOpenClose {
                consequence_status,
                export_2d_on_commit,
            });
        }
        CadenceBoundCommitError::Commit(ThermodynamicCommitError::Runtime(error)) => {
            warn!("thermodynamic close runtime rejected: {error:?}");
            schedule.note_fault(Some(tick_id), ThermodynamicScheduleFaultKind::FinalizeRuntime);
            schedule.pending = Some(PendingThermodynamicTick::AwaitingOpenClose {
                consequence_status,
                export_2d_on_commit,
            });
        }
        CadenceBoundCommitError::Commit(ThermodynamicCommitError::OperationalClose(retry)) => {
            warn!("thermodynamic operational close preflight rejected: {:?}", retry.error());
            schedule.note_fault(Some(tick_id), ThermodynamicScheduleFaultKind::OperationalClose);
            schedule.pending = Some(PendingThermodynamicTick::AwaitingPreparedClose {
                retry,
                export_2d_on_commit,
            });
        }
    }
}

fn try_open_close(
    runtime: &mut ThermodynamicTransactionRuntime,
    schedule: &mut ThermodynamicScheduleState,
    physics: &mut PhysicsWorldRes,
    hud: &mut ThermodynamicHudState,
    handles: &[BodyHandle],
    consequence_status: ThermodynamicConsequenceStatus,
    export_2d_on_commit: bool,
    fixed_time: &Time<Fixed>,
) -> bool {
    let Some(tick_id) = runtime.open_tick_id() else {
        schedule.poison(None);
        return false;
    };

    match finalize_thermodynamic_tick_at_admitted_cadence(
        runtime,
        physics,
        hud,
        handles,
        consequence_status,
        fixed_time,
    ) {
        Ok(receipt) => {
            schedule.commit_receipt(receipt);
            export_2d_on_commit
        }
        Err(error) => {
            store_open_close_failure(
                schedule,
                tick_id,
                consequence_status,
                export_2d_on_commit,
                error,
            );
            false
        }
    }
}

fn try_prepared_retry(
    runtime: &mut ThermodynamicTransactionRuntime,
    schedule: &mut ThermodynamicScheduleState,
    physics: &mut PhysicsWorldRes,
    hud: &mut ThermodynamicHudState,
    handles: &[BodyHandle],
    retry: PreparedOperationalCloseRetry,
    export_2d_on_commit: bool,
    fixed_time: &Time<Fixed>,
) -> bool {
    let tick_id = retry.tick_id();
    match retry_thermodynamic_tick_at_admitted_cadence(
        runtime,
        physics,
        hud,
        handles,
        retry,
        fixed_time,
    ) {
        Ok(receipt) => {
            schedule.commit_receipt(receipt);
            export_2d_on_commit
        }
        Err(CadenceBoundRetryError::Cadence { error, retry }) => {
            warn!("thermodynamic retry cadence rejected: {error:?}");
            schedule.note_fault(Some(tick_id), ThermodynamicScheduleFaultKind::RetryCadence);
            schedule.pending = Some(PendingThermodynamicTick::AwaitingPreparedClose {
                retry,
                export_2d_on_commit,
            });
            false
        }
        Err(CadenceBoundRetryError::Commit(ThermodynamicCommitError::OperationalClose(retry))) => {
            warn!("thermodynamic retry close preflight rejected: {:?}", retry.error());
            schedule.note_fault(
                Some(tick_id),
                ThermodynamicScheduleFaultKind::RetryOperationalClose,
            );
            schedule.pending = Some(PendingThermodynamicTick::AwaitingPreparedClose {
                retry,
                export_2d_on_commit,
            });
            false
        }
        Err(CadenceBoundRetryError::Commit(ThermodynamicCommitError::Runtime(error))) => {
            error!("thermodynamic prepared retry invariant failed: {error:?}");
            schedule.poison(Some(tick_id));
            false
        }
    }
}

fn consequence_admitted(
    runtime: &ThermodynamicTransactionRuntime,
    schedule: &mut ThermodynamicScheduleState,
    fixed_time: &Time<Fixed>,
    agent_query: &Query<&PhysicsBody, Or<(With<Player>, With<CrewNpc>)>>,
    admitted_handles: &[BodyHandle],
) -> bool {
    if let Err(error) = admit_thermodynamic_cadence(fixed_time) {
        warn!("thermodynamic cadence rejected before consequence: {error:?}");
        schedule.note_fault(
            runtime.open_tick_id(),
            ThermodynamicScheduleFaultKind::CadenceBeforeConsequence,
        );
        schedule.pending = Some(PendingThermodynamicTick::AwaitingConsequence);
        return false;
    }

    if !live_census_matches(agent_query, admitted_handles) {
        warn!("thermodynamic live agent census drifted before consequence");
        schedule.note_fault(
            runtime.open_tick_id(),
            ThermodynamicScheduleFaultKind::CensusBeforeConsequence,
        );
        schedule.pending = Some(PendingThermodynamicTick::AwaitingConsequence);
        return false;
    }

    true
}

/// Execute or resume the consequence/finalize half of the fixed transaction.
///
/// This system should run every FixedUpdate, not only during gameplay. When no
/// transaction is active it is a no-op. That allows an already-open tick to close
/// after a game-state transition without permitting a new begin outside gameplay.
pub fn transactional_physics_finalize_system(
    mut runtime: ResMut<ThermodynamicTransactionRuntime>,
    mut schedule: ResMut<ThermodynamicScheduleState>,
    fixed_time: Res<Time<Fixed>>,
    mut physics: ResMut<PhysicsWorldRes>,
    mut hud: ResMut<ThermodynamicHudState>,
    agent_query: Query<&PhysicsBody, Or<(With<Player>, With<CrewNpc>)>>,
    mut transform_query: Query<(&PhysicsBody, &mut Transform)>,
) {
    if runtime.open_tick_id().is_none() && schedule.pending.is_none() {
        if schedule.active_handles.is_some() || schedule.active_mode.is_some() {
            schedule.poison(None);
        }
        return;
    }

    let handles = match schedule.active_handles.clone() {
        Some(handles) => handles,
        None => {
            schedule.poison(runtime.open_tick_id());
            return;
        }
    };
    let mode = match schedule.active_mode {
        Some(mode) => mode,
        None => {
            schedule.poison(runtime.open_tick_id());
            return;
        }
    };

    let export_after_commit = match schedule.pending.take() {
        Some(PendingThermodynamicTick::Poisoned) => {
            schedule.pending = Some(PendingThermodynamicTick::Poisoned);
            return;
        }
        Some(PendingThermodynamicTick::AwaitingPreparedClose {
            retry,
            export_2d_on_commit,
        }) => try_prepared_retry(
            &mut runtime,
            &mut schedule,
            &mut physics,
            &mut hud,
            &handles,
            retry,
            export_2d_on_commit,
            &fixed_time,
        ),
        Some(PendingThermodynamicTick::AwaitingOpenClose {
            consequence_status,
            export_2d_on_commit,
        }) => try_open_close(
            &mut runtime,
            &mut schedule,
            &mut physics,
            &mut hud,
            &handles,
            consequence_status,
            export_2d_on_commit,
            &fixed_time,
        ),
        Some(PendingThermodynamicTick::AwaitingConsequence) | None => {
            if !consequence_admitted(
                &runtime,
                &mut schedule,
                &fixed_time,
                &agent_query,
                &handles,
            ) {
                return;
            }
            let (status, export) = consequence_for_bound_mode(mode, &mut physics, &fixed_time);
            try_open_close(
                &mut runtime,
                &mut schedule,
                &mut physics,
                &mut hud,
                &handles,
                status,
                export,
                &fixed_time,
            )
        }
    };

    if export_after_commit {
        export_authoritative_2d_transforms(&physics, &mut transform_query);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_census_is_sorted_and_rejects_duplicate_identity() {
        assert_eq!(
            canonicalize_handles([BodyHandle(9), BodyHandle(2), BodyHandle(5)]).unwrap(),
            vec![BodyHandle(2), BodyHandle(5), BodyHandle(9)]
        );
        assert_eq!(
            canonicalize_handles([BodyHandle(2), BodyHandle(2)]),
            Err(BodyHandle(2))
        );
    }

    #[test]
    fn gameplay_phase_binds_one_consequence_authority() {
        assert_eq!(
            BoundConsequenceMode::from_phase(GamePhase::Playing),
            Some(BoundConsequenceMode::Dynamic2d)
        );
        assert_eq!(
            BoundConsequenceMode::from_phase(GamePhase::Playing3D),
            Some(BoundConsequenceMode::Kinematic3d)
        );
        assert_eq!(BoundConsequenceMode::from_phase(GamePhase::MainMenu), None);
    }

    #[test]
    fn invalid_or_unrepresentable_maintenance_never_becomes_a_credit() {
        assert_eq!(maintenance_cost_with_offload(f64::NAN, 0.5, 0.5), None);
        assert_eq!(maintenance_cost_with_offload(-1.0, 0.5, 0.5), None);
        let invalid_phi = maintenance_cost_with_offload(1.0, f64::NAN, 1.0).unwrap();
        assert_eq!(invalid_phi, 1.5);
    }
}
