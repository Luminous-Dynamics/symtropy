// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Thermodynamic enforcement system.
//!
//! Runs at 64Hz in FixedUpdate. Enforces the 7 thermodynamic rules:
//! 1. Moving costs energy
//! 2. Consciousness costs energy
//! 3. Collision spikes prediction error (handled in PhysicsCallback)
//! 4. Harmony resonance can reduce duplicated processing cost
//! 5. Sanctuary zones emerge (handled in consciousness coupling)
//! 6. Energy depleted = consciousness collapse
//! 7. Energy regenerates from environment (ambient + wells)

use bevy::prelude::*;
use std::collections::HashMap;
use symtropy_physics::BodyHandle;

use crate::components::{CrewNpc, Player};
use crate::resources::{EnergyWell, PhysicsWorldRes, SafetyTier};
use symtropy_render_bridge::PhysicsBody;

/// Marker for entities that have collapsed (zero energy).
#[derive(Component)]
pub struct EnergyCollapsed;

/// Thermodynamic HUD state — accumulates per-tick data for display.
#[derive(Resource, Default)]
pub struct ThermodynamicHudState {
    pub energy_consumed_accumulator: f64,
    pub energy_regenerated_accumulator: f64,
    pub ticks_accumulated: u32,
    /// Per-second rates (updated periodically).
    pub consumed_per_sec: f64,
    pub regenerated_per_sec: f64,
}

/// Main enforcement system. Runs in FixedUpdate.
///
/// Debits consciousness maintenance, applies ambient regeneration,
/// checks for collapse, and handles harmony-resonance cost reduction.
pub fn thermodynamic_enforcement_system(
    mut physics: ResMut<PhysicsWorldRes>,
    mut hud_state: ResMut<ThermodynamicHudState>,
    entities_query: Query<(&PhysicsBody, &Transform), Or<(With<Player>, With<CrewNpc>)>>,
    mut wells: Query<(&Transform, &mut EnergyWell, &mut Sprite), Without<Player>>,
) {
    let constants = physics.consciousness.constants.clone();

    // Collect handles and positions for iteration.
    let agent_data: Vec<_> = entities_query
        .iter()
        .map(|(pb, tf)| (pb.handle, tf.translation))
        .collect();
    let handles: Vec<_> = agent_data.iter().map(|(h, _)| *h).collect();

    // Pre-compute values that don't need mutable access.
    let regen_mult = physics.consciousness.resource_regeneration_multiplier();

    // --- Rule 4: Epistemic offloading reduces duplicated processing cost ---
    //
    // The previous implementation first charged full maintenance and then
    // called `regenerate()` as a refund. That mislabeled a cost reduction as a
    // new energy source, incremented regeneration telemetry, compounded across
    // multiple partners, and could revive a collapsed entity. Instead, derive
    // one bounded offload factor per entity before debiting maintenance.
    let offload_factors = epistemic_offload_factors(&physics, &handles, constants.harmony_range);

    // --- Per-entity costs ---
    for &handle in &handles {
        if let Some(entity) = physics.consciousness.entities.get_mut(&handle) {
            // Reset per-tick counters.
            entity.energy.tick_reset();

            let offload_factor = offload_factors.get(&handle).copied().unwrap_or(0.0);
            if offload_factor > 0.0 {
                // Shared predictive structure accelerates surprise recovery. Apply
                // this once using the strongest partner rather than compounding
                // once per neighbor.
                entity.prediction_error *= 1.0 - offload_factor * 0.1;
                entity.motor_precision = 1.0 / (1.0 + entity.prediction_error);
            }

            // Rule 2: consciousness maintenance cost. Higher Φ costs more.
            // Offloading reduces the duplicated base-processing portion directly;
            // it does not generate replacement energy after the debit.
            let phi = entity.phi();
            let maintenance = maintenance_cost_with_offload(
                constants.consciousness_maintenance_per_tick,
                phi,
                offload_factor,
            );
            entity.energy.consume(maintenance);

            // Rule 7: Ambient regeneration (slow, not enough alone).
            let ambient = constants.ambient_regen_rate * regen_mult;
            entity.energy.regenerate(ambient);

            // Rule 6: Check for collapse.
            if entity.energy.is_collapsed() {
                entity.safety_tier = SafetyTier::Red;
            }
        }
    }

    // --- Energy Wells: finite spatial regeneration sources ---
    //
    // Source depletion must equal energy actually accepted by the destination.
    // A full receiver therefore leaves the well unchanged instead of deleting
    // the offered energy from the modeled world.
    for (well_tf, mut well, mut well_sprite) in &mut wells {
        if !well.is_active() {
            well_sprite.color = Color::srgba(0.2, 0.2, 0.2, 0.15); // dim depleted wells
            continue;
        }

        for &(handle, agent_pos) in &agent_data {
            let dist = agent_pos
                .truncate()
                .distance(well_tf.translation.truncate());
            if dist < well.radius
                && let Some(entity) = physics.consciousness.entities.get_mut(&handle)
            {
                transfer_from_well(&mut well, &mut entity.energy);
            }
        }

        // Visual: pulse alpha based on remaining capacity.
        let frac = well.fraction_remaining() as f32;
        well_sprite.color = Color::srgba(0.1, 0.8 * frac, 0.6 * frac, 0.2 + 0.3 * frac);
    }

    // Record the operational maintenance debit in the legacy telemetry ledger.
    // This ledger is not a complete first-law physics ledger; #40 owns
    // convergence onto the core typed energy-transfer accounting path.
    let total_maintenance: f64 = handles
        .iter()
        .filter_map(|h| physics.consciousness.entities.get(h))
        .map(|e| e.energy.consumed_this_tick)
        .sum();
    physics
        .consciousness
        .ledger
        .record_dissipation(total_maintenance);

    // --- Finalize thermodynamics ---
    let _balance = physics.consciousness.tick_thermodynamics();

    // --- Update HUD state ---
    hud_state.ticks_accumulated += 1;
    for &handle in &handles {
        if let Some(entity) = physics.consciousness.entities.get(&handle) {
            hud_state.energy_consumed_accumulator += entity.energy.consumed_this_tick;
            hud_state.energy_regenerated_accumulator += entity.energy.regenerated_this_tick;
        }
    }
    // Update per-second rates every 16 ticks (~0.25 seconds at 64Hz).
    if hud_state.ticks_accumulated >= 16 {
        let seconds = hud_state.ticks_accumulated as f64 / 64.0;
        hud_state.consumed_per_sec = hud_state.energy_consumed_accumulator / seconds;
        hud_state.regenerated_per_sec = hud_state.energy_regenerated_accumulator / seconds;
        hud_state.energy_consumed_accumulator = 0.0;
        hud_state.energy_regenerated_accumulator = 0.0;
        hud_state.ticks_accumulated = 0;
    }
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

fn maintenance_cost_with_offload(base_cost: f64, phi: f64, offload_factor: f64) -> f64 {
    if !base_cost.is_finite() || base_cost <= 0.0 || !phi.is_finite() {
        return 0.0;
    }

    let raw_cost = base_cost * (1.0 + phi.max(0.0) * 0.5);
    if !raw_cost.is_finite() || raw_cost <= 0.0 {
        return 0.0;
    }

    let factor = if offload_factor.is_finite() {
        offload_factor.clamp(0.0, 1.0)
    } else {
        0.0
    };

    // Preserve the established single-partner refund magnitude, but implement
    // it as a pre-debit reduction and bound it to one strongest-partner effect.
    let discount = base_cost * factor * 0.5;
    (raw_cost - discount).max(0.0)
}

#[cfg(feature = "consciousness-runtime")]
fn transfer_from_well(
    well: &mut EnergyWell,
    energy: &mut symtropy_consciousness_physics::EnergyBudget,
) -> f64 {
    let Some(offered) = finite_well_offer(well) else {
        return 0.0;
    };

    let accepted = energy.regenerate_checked(offered).unwrap_or(0.0);
    commit_well_debit(well, offered, accepted)
}

#[cfg(not(feature = "consciousness-runtime"))]
fn transfer_from_well(
    well: &mut EnergyWell,
    energy: &mut crate::resources::ConsciousnessEnergy,
) -> f64 {
    let Some(offered) = finite_well_offer(well) else {
        return 0.0;
    };

    let before = energy.available;
    energy.regenerate(offered);
    let accepted = energy.available - before;
    commit_well_debit(well, offered, accepted)
}

fn finite_well_offer(well: &EnergyWell) -> Option<f64> {
    if !well.regen_rate.is_finite()
        || well.regen_rate <= 0.0
        || !well.remaining.is_finite()
        || well.remaining <= 0.0
    {
        return None;
    }

    let offered = well.regen_rate.min(well.remaining);
    offered.is_finite().then_some(offered)
}

fn commit_well_debit(well: &mut EnergyWell, offered: f64, accepted: f64) -> f64 {
    if !offered.is_finite()
        || offered <= 0.0
        || !accepted.is_finite()
        || accepted <= 0.0
        || accepted > offered
        || accepted > well.remaining
    {
        return 0.0;
    }

    let next_remaining = well.remaining - accepted;
    if !next_remaining.is_finite() || next_remaining < 0.0 {
        return 0.0;
    }

    well.remaining = next_remaining;
    accepted
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
        0.0
    } else {
        (dot / (mag_a * mag_b)).clamp(0.0, 1.0)
    }
}

/// Visual system: gray out collapsed entities. Run in Update.
pub fn collapse_visual_system(
    physics: Res<PhysicsWorldRes>,
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &PhysicsBody,
        Option<&mut Sprite>,
        Option<&MeshMaterial3d<StandardMaterial>>,
        Option<&EnergyCollapsed>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, body_comp, opt_sprite, opt_mat, collapsed_marker) in &mut query {
        let is_collapsed = physics
            .consciousness
            .entities
            .get(&body_comp.handle)
            .map(|e| e.energy.is_collapsed())
            .unwrap_or(false);

        if is_collapsed && collapsed_marker.is_none() {
            if let Some(mut sprite) = opt_sprite {
                sprite.color = Color::srgba(0.3, 0.3, 0.3, 0.7);
            }
            if let Some(mat_handle) = opt_mat
                && let Some(mut mat) = materials.get_mut(&mat_handle.0)
            {
                mat.base_color = Color::srgba(0.3, 0.3, 0.3, 0.7);
            }
            commands.entity(entity).insert(EnergyCollapsed);
        } else if !is_collapsed && collapsed_marker.is_some() {
            if let Some(mut sprite) = opt_sprite {
                sprite.color = Color::srgba(1.0, 1.0, 1.0, 1.0);
            }
            if let Some(mat_handle) = opt_mat
                && let Some(mut mat) = materials.get_mut(&mat_handle.0)
            {
                mat.base_color = Color::srgba(1.0, 1.0, 1.0, 1.0);
            }
            commands.entity(entity).remove::<EnergyCollapsed>();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offload_reduces_maintenance_without_becoming_a_credit() {
        let base = 0.08;
        let phi = 0.8;
        let raw = maintenance_cost_with_offload(base, phi, 0.0);
        let offloaded = maintenance_cost_with_offload(base, phi, 1.0);

        assert!(offloaded < raw);
        assert!((raw - offloaded - base * 0.5).abs() < 1e-12);
        assert!(offloaded >= 0.0);
    }

    #[test]
    fn offload_factor_is_bounded_to_one_strongest_partner() {
        let base = 0.08;
        let phi = 0.8;
        let fully_offloaded = maintenance_cost_with_offload(base, phi, 1.0);
        let overclaimed = maintenance_cost_with_offload(base, phi, 10.0);
        assert_eq!(overclaimed, fully_offloaded);

        let mut factors = HashMap::new();
        let handle = BodyHandle(7);
        accumulate_strongest_factor(&mut factors, handle, 0.4);
        accumulate_strongest_factor(&mut factors, handle, 0.9);
        accumulate_strongest_factor(&mut factors, handle, 0.6);
        assert_eq!(factors.get(&handle), Some(&0.9));
    }

    #[test]
    fn invalid_harmony_evidence_cannot_create_offload() {
        let mut a = [0.5; 9];
        let b = [0.5; 9];
        a[3] = f64::NAN;
        assert_eq!(harmony_resonance(&a, &b), 0.0);

        let mut factors = HashMap::new();
        accumulate_strongest_factor(&mut factors, BodyHandle(1), f64::NAN);
        assert!(factors.is_empty());
    }

    #[test]
    fn well_debits_only_energy_accepted_by_full_receiver_contract() {
        let mut well = EnergyWell::new(20.0, 10.0, 100.0);
        let remaining_before = well.remaining;
        assert_eq!(commit_well_debit(&mut well, 20.0, 0.0), 0.0);
        assert_eq!(well.remaining, remaining_before);
    }

    #[test]
    fn well_debits_partial_acceptance_exactly() {
        let mut well = EnergyWell::new(20.0, 10.0, 100.0);
        let accepted = commit_well_debit(&mut well, 20.0, 7.5);
        assert_eq!(accepted, 7.5);
        assert_eq!(well.remaining, 92.5);
    }

    #[test]
    fn invalid_or_overaccepted_well_transfer_fails_closed() {
        for accepted in [f64::NAN, f64::INFINITY, -1.0, 21.0] {
            let mut well = EnergyWell::new(20.0, 10.0, 100.0);
            let before = well.remaining;
            assert_eq!(commit_well_debit(&mut well, 20.0, accepted), 0.0);
            assert_eq!(well.remaining, before);
        }
    }

    #[cfg(feature = "consciousness-runtime")]
    #[test]
    fn full_runtime_well_transfer_conserves_source_and_destination_delta() {
        let mut well = EnergyWell::new(20.0, 10.0, 100.0);
        let mut energy = symtropy_consciousness_physics::EnergyBudget::new(100.0);
        energy.consume(10.0);
        energy.tick_reset();

        let destination_before = energy.available;
        let source_before = well.remaining;
        let accepted = transfer_from_well(&mut well, &mut energy);

        assert_eq!(accepted, 10.0);
        assert_eq!(energy.available - destination_before, accepted);
        assert_eq!(source_before - well.remaining, accepted);
        assert_eq!(energy.regenerated_this_tick, accepted);
    }

    #[cfg(feature = "consciousness-runtime")]
    #[test]
    fn full_runtime_full_receiver_does_not_deplete_well() {
        let mut well = EnergyWell::new(20.0, 10.0, 100.0);
        let mut energy = symtropy_consciousness_physics::EnergyBudget::new(100.0);
        let source_before = well.remaining;

        assert_eq!(transfer_from_well(&mut well, &mut energy), 0.0);
        assert_eq!(well.remaining, source_before);
        assert_eq!(energy.regenerated_this_tick, 0.0);
    }

    #[cfg(not(feature = "consciousness-runtime"))]
    #[test]
    fn fallback_well_transfer_conserves_source_and_destination_delta() {
        let mut well = EnergyWell::new(20.0, 10.0, 100.0);
        let mut energy = crate::resources::ConsciousnessEnergy::new(100.0);
        energy.consume(10.0);
        energy.tick_reset();

        let destination_before = energy.available;
        let source_before = well.remaining;
        let accepted = transfer_from_well(&mut well, &mut energy);

        assert_eq!(accepted, 10.0);
        assert_eq!(energy.available - destination_before, accepted);
        assert_eq!(source_before - well.remaining, accepted);
        assert_eq!(energy.regenerated_this_tick, accepted);
    }

    #[cfg(not(feature = "consciousness-runtime"))]
    #[test]
    fn fallback_full_receiver_does_not_deplete_well() {
        let mut well = EnergyWell::new(20.0, 10.0, 100.0);
        let mut energy = crate::resources::ConsciousnessEnergy::new(100.0);
        let source_before = well.remaining;

        assert_eq!(transfer_from_well(&mut well, &mut energy), 0.0);
        assert_eq!(well.remaining, source_before);
        assert_eq!(energy.regenerated_this_tick, 0.0);
    }
}
