// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Thermodynamic enforcement system.
//!
//! Runs at 64Hz in FixedUpdate. Enforces the 7 thermodynamic rules:
//! 1. Moving costs energy
//! 2. Consciousness costs energy
//! 3. Collision spikes prediction error (handled in PhysicsCallback)
//! 4. Harmony resonance transfers energy
//! 5. Sanctuary zones emerge (handled in consciousness coupling)
//! 6. Energy depleted = consciousness collapse
//! 7. Energy regenerates from environment (ambient + wells)

use bevy::prelude::*;
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

/// Offer regeneration to one registered entity and return the amount actually accepted.
///
/// Under `consciousness-runtime`, `EnergyBudget::regenerate` is the authoritative bounded
/// transfer primitive and returns the accepted joules directly. The standalone launcher
/// stub predates that API and returns unit, so this one compatibility bridge derives the
/// accepted amount from its already-bounded before/after state. Source-backed systems use
/// this helper rather than duplicating that inference at each call site.
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

/// Transfer energy from one finite source into a registered recipient reservoir.
///
/// The source is debited only by the amount the recipient actually accepted. Invalid
/// source state, invalid offer limits, missing recipients and full recipients are no-ops.
/// This helper is intentionally independent of `EnergyWell` so other finite stores can
/// reuse the same source-conservation theorem later.
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

    // Defense in depth: the bounded recipient must never accept more than the offer.
    let accepted = accepted.min(offered);
    *source_remaining = (*source_remaining - accepted).max(0.0);
    accepted
}

/// Main enforcement system. Runs in FixedUpdate.
///
/// Debits consciousness maintenance, applies ambient regeneration,
/// checks for collapse, and handles harmony resonance energy transfer.
pub fn thermodynamic_enforcement_system(
    mut physics: ResMut<PhysicsWorldRes>,
    mut hud_state: ResMut<ThermodynamicHudState>,
    entities_query: Query<(&PhysicsBody, &Transform), Or<(With<Player>, With<CrewNpc>)>>,
    mut wells: Query<(&Transform, &mut EnergyWell, &mut Sprite), Without<Player>>,
) {
    let constants = physics.consciousness.constants.clone();

    // Collect handles and positions for iteration
    let agent_data: Vec<_> = entities_query
        .iter()
        .map(|(pb, tf)| (pb.handle, tf.translation))
        .collect();
    let handles: Vec<_> = agent_data.iter().map(|(h, _)| *h).collect();

    // Pre-compute values that don't need mutable access
    let regen_mult = physics.consciousness.resource_regeneration_multiplier();

    // --- Per-entity costs ---
    for &handle in &handles {
        if let Some(entity) = physics.consciousness.entities.get_mut(&handle) {
            // Reset per-tick counters
            entity.energy.tick_reset();

            // Rule 2: Consciousness maintenance cost
            // Higher Φ costs more: base * (1.0 + phi * 0.5)
            let phi = entity.phi();
            let maintenance = constants.consciousness_maintenance_per_tick * (1.0 + phi * 0.5);
            let _ = entity.energy.consume(maintenance);

            // Rule 7: Ambient regeneration (slow, not enough alone).
            // Ambient regeneration is not modeled as a finite source in this slice, so
            // there is no source reservoir to debit by the accepted amount here.
            let ambient = constants.ambient_regen_rate * regen_mult;
            let _ = entity.energy.regenerate(ambient);

            // Rule 6: Check for collapse
            if entity.energy.is_collapsed() {
                entity.safety_tier = SafetyTier::Red;
            }
        }
    }

    // --- Energy Wells: finite source-backed regeneration ---
    for (well_tf, mut well, mut well_sprite) in &mut wells {
        if !well.is_active() {
            well_sprite.color = Color::srgba(0.2, 0.2, 0.2, 0.15); // dim depleted wells
            continue;
        }

        for &(handle, agent_pos) in &agent_data {
            let dist = agent_pos
                .truncate()
                .distance(well_tf.translation.truncate());
            if dist < well.radius {
                let regen_rate = well.regen_rate;
                let _accepted = transfer_from_finite_source(
                    &mut physics,
                    handle,
                    &mut well.remaining,
                    regen_rate,
                );
            }
        }

        // Visual: pulse alpha based on remaining capacity
        let frac = well.fraction_remaining() as f32;
        well_sprite.color = Color::srgba(0.1, 0.8 * frac, 0.6 * frac, 0.2 + 0.3 * frac);
    }

    // Record total maintenance as dissipation
    let total_maintenance: f64 = handles
        .iter()
        .filter_map(|h| physics.consciousness.entities.get(h))
        .map(|e| e.energy.consumed_this_tick)
        .sum();
    physics
        .consciousness
        .ledger
        .record_dissipation(total_maintenance);

    // --- Rule 4: Epistemic offloading (resonance REDUCES COSTS, not generates energy) ---
    // Thermodynamically honest: cooperation doesn't create energy.
    // It reduces the prediction error processing cost for nearby agents.
    // When agents resonate, they share internal models via harmony alignment.
    // This means each agent burns fewer Joules on surprise processing.
    //
    // Implementation: resonant agents get their prediction error decayed FASTER
    // and their consciousness maintenance cost REDUCED (not energy added).
    let range = constants.harmony_range;

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
            if dist > range {
                continue;
            }

            let resonance = harmony_resonance(&harmonies_a, &harmonies_b);
            if resonance > 0.5 {
                let offload_factor = (resonance - 0.5) * 2.0; // [0, 1]

                // Epistemic offloading: accelerate prediction error decay
                // (shared models = faster learning = less energy burned on surprise)
                if let Some(entity) = physics.consciousness.entities.get_mut(&ha) {
                    entity.prediction_error *= 1.0 - offload_factor * 0.1; // 10% faster decay per tick
                    entity.motor_precision = 1.0 / (1.0 + entity.prediction_error);
                    // Refund some of the maintenance cost (predictability reduces processing)
                    let _ = entity.energy.regenerate(
                        constants.consciousness_maintenance_per_tick * offload_factor * 0.5,
                    );
                }
                if let Some(entity) = physics.consciousness.entities.get_mut(&hb) {
                    entity.prediction_error *= 1.0 - offload_factor * 0.1;
                    entity.motor_precision = 1.0 / (1.0 + entity.prediction_error);
                    let _ = entity.energy.regenerate(
                        constants.consciousness_maintenance_per_tick * offload_factor * 0.5,
                    );
                }

                // Collapse recovery: epistemic offloading can't revive a dead agent
                // but mechanical synergy (being physically carried to an energy well) could.
                // For now: collapsed agents need wells, not friends.
                // This is thermodynamically honest — you can't think someone back to life.
            }
        }
    }

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
    // Update per-second rates every 16 ticks (~0.25 seconds at 64Hz)
    if hud_state.ticks_accumulated >= 16 {
        let seconds = hud_state.ticks_accumulated as f64 / 64.0;
        hud_state.consumed_per_sec = hud_state.energy_consumed_accumulator / seconds;
        hud_state.regenerated_per_sec = hud_state.energy_regenerated_accumulator / seconds;
        hud_state.energy_consumed_accumulator = 0.0;
        hud_state.energy_regenerated_accumulator = 0.0;
        hud_state.ticks_accumulated = 0;
    }
}

fn harmony_resonance(a: &[f64; 9], b: &[f64; 9]) -> f64 {
    let dot = a.iter().zip(b).map(|(a, b)| a * b).sum::<f64>();
    let mag_a = a.iter().map(|v| v * v).sum::<f64>().sqrt();
    let mag_b = b.iter().map(|v| v * v).sum::<f64>().sqrt();
    if mag_a <= 1e-10 || mag_b <= 1e-10 {
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

    fn registered_entity_with_headroom(headroom: f64) -> (PhysicsWorldRes, BodyHandle) {
        let mut physics = PhysicsWorldRes::default();
        let handle = physics
            .world
            .add_sphere(symtropy_math::Point::origin(), 1.0, 1.0);
        physics.consciousness.register(handle, 100.0, 10.0);
        if headroom > 0.0 {
            let entity = physics.consciousness.entities.get_mut(&handle).unwrap();
            let _ = entity.energy.consume(headroom.min(100.0));
            entity.energy.tick_reset();
        }
        (physics, handle)
    }

    fn add_registered_entity_with_headroom(
        physics: &mut PhysicsWorldRes,
        headroom: f64,
    ) -> BodyHandle {
        let handle = physics
            .world
            .add_sphere(symtropy_math::Point::origin(), 1.0, 1.0);
        physics.consciousness.register(handle, 100.0, 10.0);
        if headroom > 0.0 {
            let entity = physics.consciousness.entities.get_mut(&handle).unwrap();
            let _ = entity.energy.consume(headroom.min(100.0));
            entity.energy.tick_reset();
        }
        handle
    }

    #[test]
    fn accepted_regeneration_is_zero_for_full_reservoir() {
        let (mut physics, handle) = registered_entity_with_headroom(0.0);
        let accepted = regenerate_entity_accepted(&mut physics, handle, 10.0);
        assert!((accepted - 0.0).abs() < 1e-10);
    }

    #[test]
    fn accepted_regeneration_is_bounded_by_recipient_headroom() {
        let (mut physics, handle) = registered_entity_with_headroom(3.0);
        let accepted = regenerate_entity_accepted(&mut physics, handle, 10.0);
        assert!((accepted - 3.0).abs() < 1e-10);
        let entity = physics.consciousness.entities.get(&handle).unwrap();
        assert!((entity.energy.available - 100.0).abs() < 1e-10);
        assert!((entity.energy.regenerated_this_tick - 3.0).abs() < 1e-10);
    }

    #[test]
    fn accepted_regeneration_is_zero_for_missing_entity() {
        let mut physics = PhysicsWorldRes::default();
        assert!((regenerate_entity_accepted(&mut physics, BodyHandle(999_999), 10.0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn finite_source_is_not_debited_when_recipient_is_full() {
        let (mut physics, handle) = registered_entity_with_headroom(0.0);
        let mut source = 10.0;
        let accepted = transfer_from_finite_source(&mut physics, handle, &mut source, 10.0);
        assert!((accepted - 0.0).abs() < 1e-10);
        assert!((source - 10.0).abs() < 1e-10);
    }

    #[test]
    fn finite_source_loses_exactly_recipient_gain() {
        let (mut physics, handle) = registered_entity_with_headroom(3.0);
        let mut source = 10.0;
        let source_before = source;
        let recipient_before = physics
            .consciousness
            .entities
            .get(&handle)
            .unwrap()
            .energy
            .available;

        let accepted = transfer_from_finite_source(&mut physics, handle, &mut source, 10.0);
        let recipient_after = physics
            .consciousness
            .entities
            .get(&handle)
            .unwrap()
            .energy
            .available;

        assert!((accepted - 3.0).abs() < 1e-10);
        assert!((source_before - source - accepted).abs() < 1e-10);
        assert!((recipient_after - recipient_before - accepted).abs() < 1e-10);
    }

    #[test]
    fn finite_source_conserves_across_multiple_recipients() {
        let mut physics = PhysicsWorldRes::default();
        let first = add_registered_entity_with_headroom(&mut physics, 3.0);
        let second = add_registered_entity_with_headroom(&mut physics, 4.0);
        let mut source = 10.0;
        let source_before = source;

        let accepted_first = transfer_from_finite_source(&mut physics, first, &mut source, 10.0);
        let accepted_second = transfer_from_finite_source(&mut physics, second, &mut source, 10.0);
        let total_accepted = accepted_first + accepted_second;

        assert!((accepted_first - 3.0).abs() < 1e-10);
        assert!((accepted_second - 4.0).abs() < 1e-10);
        assert!((total_accepted - 7.0).abs() < 1e-10);
        assert!((source_before - source - total_accepted).abs() < 1e-10);
        assert!((source - 3.0).abs() < 1e-10);
    }

    #[test]
    fn finite_source_rejects_invalid_offer_or_source_state() {
        let (mut physics, handle) = registered_entity_with_headroom(10.0);

        for offer in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut source = 10.0;
            let accepted = transfer_from_finite_source(&mut physics, handle, &mut source, offer);
            assert!((accepted - 0.0).abs() < 1e-10);
            assert!((source - 10.0).abs() < 1e-10);
        }

        for bad_source in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut source = bad_source;
            let accepted = transfer_from_finite_source(&mut physics, handle, &mut source, 10.0);
            assert!((accepted - 0.0).abs() < 1e-10);
            if bad_source.is_nan() {
                assert!(source.is_nan());
            } else {
                assert_eq!(source, bad_source);
            }
        }
    }
}
