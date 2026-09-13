// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Bevy FixedUpdate adapter for the exactly-once thermodynamic transaction stack.
//!
//! This module is the schedule-facing authority. It deliberately separates:
//!
//! 1. cadence admission + operational begin;
//! 2. one consequential 2D physics step (or typed intentional absence in 3D);
//! 3. friction/close readiness;
//! 4. cadence-bound operational close + fixed-tick receipt commit;
//! 5. 2D transform export only after the transaction commits.
//!
//! If finalization is delayed, the adapter records *which side of the consequence
//! boundary the tick reached*. A later FixedUpdate resumes from that exact state;
//! it never resets counters or applies physics twice merely because close evidence
//! was temporarily unavailable.

use bevy::prelude::*;
use symtropy_physics::BodyHandle;
use symtropy_render_bridge::PhysicsBody;

use crate::components::{CrewNpc, Player};
use crate::resources::{EnergyWell, GamePhase, PhysicsWorldRes, SafetyTier};

use super::engine_physics::step_physics_world;
use super::thermodynamic::{
    ThermodynamicHudState, epistemic_offload_factors, maintenance_cost_with_offload,
    regenerate_live_entity_accepted, transfer_from_finite_source,
};
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
    CadenceBeforeConsequence,
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

#[derive(Debug)]
enum PendingThermodynamicTick {
    /// Begin-phase operational mutation happened, but consequence-time physics has
    /// not yet run. This can occur when cadence changes between begin and physics.
    AwaitingConsequence,
    /// Consequence-time processing already happened exactly once, but finalize
    /// could not yet reserve/commit the tick (for example unresolved friction).
    AwaitingOpenClose {
        consequence_status: ThermodynamicConsequenceStatus,
        export_2d_on_commit: bool,
    },
    /// #880 reserved exact tick/status/friction identity and operational close
    /// preflight rejected. The sole continuation token is persisted here.
    AwaitingPreparedClose {
        retry: PreparedOperationalCloseRetry,
        export_2d_on_commit: bool,
    },
    /// An internal invariant consumed the only retry token without producing a
    /// replacement. Runtime remains fail-closed; no new begin or physics is allowed.
    Poisoned,
}

/// Runtime scheduler state. The pending transaction state is intentionally private;
/// callers may inspect receipts/faults but cannot rewrite the resume point.
#[derive(Resource, Debug, Default)]
pub struct ThermodynamicScheduleState {
    pending: Option<PendingThermodynamicTick>,
    last_receipt: Option<CadenceBoundThermodynamicTickReceipt>,
    last_fault: Option<ThermodynamicScheduleFault>,
    fault_count: u64,
}

impl ThermodynamicScheduleState {
    pub fn has_pending_transaction(&self) -> bool {
        self.pending.is_some()
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

    fn note_fault(
        &mut self,
        tick_id: Option<u64>,
        kind: ThermodynamicScheduleFaultKind,
    ) {
        self.last_fault = Some(ThermodynamicScheduleFault { tick_id, kind });
        self.fault_count = self.fault_count.saturating_add(1);
    }

    fn commit_receipt(&mut self, receipt: CadenceBoundThermodynamicTickReceipt) {
        self.pending = None;
        self.last_receipt = Some(receipt);
    }
}

fn apply_operational_begin_flows(
    physics: &mut PhysicsWorldRes,
    agent_data: &[(BodyHandle, Vec3)],
    wells: &mut Query<(&Transform, &mut EnergyWell, &mut Sprite), Without<Player>>,
) {
    let constants = physics.consciousness.constants.clone();
    let handles: Vec<_> = agent_data.iter().map(|(handle, _)| *handle).collect();

    let regen_mult = physics.consciousness.resource_regeneration_multiplier();
    let offload_factors =
        epistemic_offload_factors(physics, &handles, constants.harmony_range);

    for &handle in &handles {
        {
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

    // Legacy operational telemetry only. This records begin-phase maintenance
    // before consequence-time writers run; it is not physical first-law evidence.
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

/// Begin exactly one fixed thermodynamic interval and apply current operational
/// maintenance/offload/well semantics without closing HUD/legacy accounting.
pub fn transactional_thermodynamic_begin_system(
    mut runtime: ResMut<ThermodynamicTransactionRuntime>,
    mut schedule: ResMut<ThermodynamicScheduleState>,
    fixed_time: Res<Time<Fixed>>,
    mut physics: ResMut<PhysicsWorldRes>,
    entities_query: Query<(&PhysicsBody, &Transform), Or<(With<Player>, With<CrewNpc>)>>,
    mut wells: Query<(&Transform, &mut EnergyWell, &mut Sprite), Without<Player>>,
) {
    // A delayed close owns the previous tick. Never reset counters underneath it.
    if schedule.pending.is_some() || runtime.open_tick_id().is_some() {
        return;
    }

    if let Err(error) =
        begin_thermodynamic_tick_at_admitted_cadence(&mut runtime, &fixed_time)
    {
        warn!("thermodynamic fixed-tick begin rejected: {error:?}");
        schedule.note_fault(None, ThermodynamicScheduleFaultKind::BeginRejected);
        return;
    }

    let agent_data: Vec<_> = entities_query
        .iter()
        .map(|(body, transform)| (body.handle, transform.translation))
        .collect();
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
        schedule.note_fault(None, ThermodynamicScheduleFaultKind::FinalizeRuntime);
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
            // #880's prepared retry path should not yield a recoverable runtime
            // error after consuming its sole continuation. Preserve fail-closed
            // state explicitly rather than pretending this tick can restart.
            error!("thermodynamic prepared retry invariant failed: {error:?}");
            schedule.note_fault(Some(tick_id), ThermodynamicScheduleFaultKind::RetryInvariant);
            schedule.pending = Some(PendingThermodynamicTick::Poisoned);
            false
        }
    }
}

fn consequence_for_phase(
    phase: GamePhase,
    physics: &mut PhysicsWorldRes,
    fixed_time: &Time<Fixed>,
) -> Option<(ThermodynamicConsequenceStatus, bool)> {
    match phase {
        GamePhase::Playing => {
            let succeeded = step_physics_world(physics, fixed_time.delta().as_secs_f64());
            Some((
                if succeeded {
                    ThermodynamicConsequenceStatus::Executed
                } else {
                    ThermodynamicConsequenceStatus::Rejected
                },
                succeeded,
            ))
        }
        GamePhase::Playing3D => Some((
            ThermodynamicConsequenceStatus::IntentionallyAbsent,
            false,
        )),
        _ => None,
    }
}

/// Execute or resume the consequence/finalize half of the fixed transaction.
///
/// 2D transforms are exported only after a successful `Executed` transaction
/// commit. The kinematic 3D slice never mirrors stale 2D world positions here.
pub fn transactional_physics_finalize_system(
    mut runtime: ResMut<ThermodynamicTransactionRuntime>,
    mut schedule: ResMut<ThermodynamicScheduleState>,
    fixed_time: Res<Time<Fixed>>,
    phase: Res<State<GamePhase>>,
    mut physics: ResMut<PhysicsWorldRes>,
    mut hud: ResMut<ThermodynamicHudState>,
    agent_query: Query<&PhysicsBody, Or<(With<Player>, With<CrewNpc>)>>,
    mut transform_query: Query<(&PhysicsBody, &mut Transform)>,
) {
    let handles: Vec<_> = agent_query.iter().map(|body| body.handle).collect();

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
        Some(PendingThermodynamicTick::AwaitingConsequence) => {
            if let Err(error) = admit_thermodynamic_cadence(&fixed_time) {
                warn!("thermodynamic cadence rejected before resumed consequence: {error:?}");
                schedule.note_fault(
                    runtime.open_tick_id(),
                    ThermodynamicScheduleFaultKind::CadenceBeforeConsequence,
                );
                schedule.pending = Some(PendingThermodynamicTick::AwaitingConsequence);
                return;
            }
            let Some((status, export)) =
                consequence_for_phase(*phase.get(), &mut physics, &fixed_time)
            else {
                schedule.pending = Some(PendingThermodynamicTick::AwaitingConsequence);
                return;
            };
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
        None => {
            if runtime.open_tick_id().is_none() {
                return;
            }
            if let Err(error) = admit_thermodynamic_cadence(&fixed_time) {
                warn!("thermodynamic cadence rejected before consequence: {error:?}");
                schedule.note_fault(
                    runtime.open_tick_id(),
                    ThermodynamicScheduleFaultKind::CadenceBeforeConsequence,
                );
                schedule.pending = Some(PendingThermodynamicTick::AwaitingConsequence);
                return;
            }
            let Some((status, export)) =
                consequence_for_phase(*phase.get(), &mut physics, &fixed_time)
            else {
                schedule.pending = Some(PendingThermodynamicTick::AwaitingConsequence);
                return;
            };
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
