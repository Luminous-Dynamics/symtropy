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

use crate::components::{CrewNpc, Player};
use crate::resources::{EnergyWell, PhysicsWorldRes, SafetyTier};
#[cfg(not(feature = "consciousness-runtime"))]
use crate::resources::ConsciousnessEnergy;
#[cfg(feature = "consciousness-runtime")]
use symtropy_consciousness_physics::EnergyBudget;
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

/// Minimal accepted-energy contract shared by the real and fallback reservoir.
trait EnergyAcceptor {
    fn accept_energy(&mut self, offered: f64) -> f64;
}

#[cfg(feature = "consciousness-runtime")]
impl EnergyAcceptor for EnergyBudget {
    fn accept_energy(&mut self, offered: f64) -> f64 {
        self.regenerate(offered)
    }
}

#[cfg(not(feature = "consciousness-runtime"))]
impl EnergyAcceptor for ConsciousnessEnergy {
    fn accept_energy(&mut self, offered: f64) -> f64 {
        self.regenerate(offered)
    }
}

/// Transfer energy from a finite well only after the recipient reports the
/// amount accepted. Invalid well state is inert and does not invoke the receiver.
fn transfer_well_energy<R: EnergyAcceptor>(well: &mut EnergyWell, recipient: &mut R) -> f64 {
    if !well.regen_rate.is_finite()
        || well.regen_rate <= 0.0
        || !well.remaining.is_finite()
        || well.remaining <= 0.0
        || !well.max_capacity.is_finite()
        || well.max_capacity < 0.0
        || well.remaining > well.max_capacity
    {
        return 0.0;
    }

    let offered = well.regen_rate.min(well.remaining);
    if !offered.is_finite() || offered <= 0.0 {
        return 0.0;
    }

    let accepted = recipient.accept_energy(offered);
    if !accepted.is_finite() || accepted < 0.0 || accepted > offered {
        // The built-in EnergyAcceptor implementations guarantee this bound;
        // keep an impossible response from debiting the finite source.
        return 0.0;
    }

    let remaining = well.remaining - accepted;
    if !remaining.is_finite() || remaining < 0.0 {
        return 0.0;
    }
    well.remaining = remaining;
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
            entity.energy.consume(maintenance);

            // Rule 7: Ambient regeneration (slow, not enough alone)
            let ambient = constants.ambient_regen_rate * regen_mult;
            entity.energy.regenerate(ambient);

            // Rule 6: Check for collapse
            if entity.energy.is_collapsed() {
                entity.safety_tier = SafetyTier::Red;
            }
        }
    }

    // --- Energy Wells: spatial regeneration sources ---
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
                if let Some(entity) = physics.consciousness.entities.get_mut(&handle) {
                    // Debit the source only by the reservoir's actual accepted transfer.
                    transfer_well_energy(&mut well, &mut entity.energy);
                }
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
                    entity.energy.regenerate(
                        constants.consciousness_maintenance_per_tick * offload_factor * 0.5,
                    );
                }
                if let Some(entity) = physics.consciousness.entities.get_mut(&hb) {
                    entity.prediction_error *= 1.0 - offload_factor * 0.1;
                    entity.motor_precision = 1.0 / (1.0 + entity.prediction_error);
                    entity.energy.regenerate(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "consciousness-runtime")]
    type TestReservoir = EnergyBudget;
    #[cfg(not(feature = "consciousness-runtime"))]
    type TestReservoir = ConsciousnessEnergy;

    #[test]
    fn full_reservoir_does_not_debit_well() {
        let mut well = EnergyWell::new(10.0, 10.0, 100.0);
        let mut reservoir = TestReservoir::new(100.0);

        let accepted = transfer_well_energy(&mut well, &mut reservoir);

        assert_eq!(accepted, 0.0);
        assert_eq!(well.remaining, 100.0);
        assert_eq!(reservoir.regenerated_this_tick, 0.0);
    }

    #[test]
    fn partial_reservoir_debits_only_accepted_energy() {
        let mut well = EnergyWell::new(10.0, 10.0, 100.0);
        let mut reservoir = TestReservoir::new(100.0);
        reservoir.available = 99.0;

        let accepted = transfer_well_energy(&mut well, &mut reservoir);

        assert_eq!(accepted, 1.0);
        assert_eq!(well.remaining, 99.0);
        assert_eq!(reservoir.available, 100.0);
        assert_eq!(reservoir.regenerated_this_tick, 1.0);
    }

    #[test]
    fn depleted_reservoir_accepts_transfer_and_debits_same_amount() {
        let mut well = EnergyWell::new(10.0, 10.0, 100.0);
        let mut reservoir = TestReservoir::new(100.0);
        reservoir.available = 0.0;

        let accepted = transfer_well_energy(&mut well, &mut reservoir);

        assert_eq!(accepted, 10.0);
        assert_eq!(well.remaining, 90.0);
        assert_eq!(reservoir.available, 10.0);
        assert_eq!(reservoir.regenerated_this_tick, 10.0);
    }

    #[test]
    fn deterministic_seeded_transfers_preserve_well_plus_reservoir_energy() {
        // 100 seeds × 1,000 independently generated source/recipient states.
        // Integer arithmetic generates repeatable f64 test values without a new dependency.
        for seed in 0_u64..100 {
            let mut state = seed + 1;
            for _case in 0..1_000 {
                let mut sample = || {
                    state = state
                        .wrapping_mul(6_364_136_223_846_793_005)
                        .wrapping_add(1_442_695_040_888_963_407);
                    ((state >> 11) as f64) / ((1_u64 << 53) as f64)
                };

                let well_capacity = 1.0 + sample() * 99.0;
                let well_remaining = sample() * well_capacity;
                let regen_rate = 0.01 + sample() * 20.0;
                let reservoir_capacity = 1.0 + sample() * 99.0;
                let reservoir_available = sample() * reservoir_capacity;

                let mut well = EnergyWell::new(regen_rate, 10.0, well_capacity);
                well.remaining = well_remaining;
                let mut reservoir = TestReservoir::new(reservoir_capacity);
                reservoir.available = reservoir_available;

                let before = well.remaining + reservoir.available;
                let expected = regen_rate
                    .min(well_remaining)
                    .min((reservoir_capacity - reservoir_available).max(0.0));
                let accepted = transfer_well_energy(&mut well, &mut reservoir);
                let after = well.remaining + reservoir.available;
                let tolerance = 1e-10 * before.max(1.0);

                assert!(
                    (accepted - expected).abs() <= tolerance,
                    "seed={seed} expected={expected} accepted={accepted}"
                );
                assert!(
                    (before - after).abs() <= tolerance,
                    "seed={seed} source/recipient energy changed: before={before}, after={after}"
                );
                assert!(
                    (reservoir.regenerated_this_tick - accepted).abs() <= tolerance,
                    "seed={seed} telemetry={} accepted={accepted}",
                    reservoir.regenerated_this_tick
                );
                assert!(
                    (well_remaining - well.remaining - accepted).abs() <= tolerance,
                    "seed={seed} well debit does not match accepted amount"
                );
            }
        }
    }

    #[test]
    fn invalid_reservoir_state_does_not_debit_well() {
        let mut well = EnergyWell::new(10.0, 10.0, 100.0);
        let mut reservoir = TestReservoir::new(100.0);
        reservoir.available = f64::NAN;

        let accepted = transfer_well_energy(&mut well, &mut reservoir);

        assert_eq!(accepted, 0.0);
        assert_eq!(well.remaining, 100.0);
        assert!(reservoir.available.is_nan());
    }

    struct CountingReservoir {
        calls: usize,
    }

    impl EnergyAcceptor for CountingReservoir {
        fn accept_energy(&mut self, offered: f64) -> f64 {
            self.calls += 1;
            offered.min(5.0)
        }
    }

    #[test]
    fn invalid_well_offer_does_not_debit_or_call_recipient() {
        for (regen_rate, remaining, capacity) in [
            (f64::NAN, 10.0, 10.0),
            (f64::INFINITY, 10.0, 10.0),
            (-1.0, 10.0, 10.0),
            (1.0, f64::NAN, 10.0),
            (1.0, 11.0, 10.0),
        ] {
            let mut well = EnergyWell::new(regen_rate, 10.0, capacity);
            well.remaining = remaining;
            let mut recipient = CountingReservoir { calls: 0 };
            let accepted = transfer_well_energy(&mut well, &mut recipient);

            assert_eq!(accepted, 0.0);
            if remaining.is_nan() {
                assert!(well.remaining.is_nan());
            } else {
                assert_eq!(well.remaining, remaining);
            }
            assert_eq!(recipient.calls, 0);
        }
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
