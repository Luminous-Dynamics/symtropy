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
use symtropy_render_bridge::PhysicsBody;

/// Exactly-once admission guard for the monolithic fixed-tick thermodynamic system.
///
/// Fixed-time elapsed duration is the tick identity because the current system is
/// one begin/effects/finalize operation. Duplicate invocations for the same Bevy
/// Fixed clock value are rejected before reservoir, ledger, or HUD mutation.
/// This does not replace the explicit transaction state machine described in #824.
#[derive(Resource, Debug, Default)]
pub struct ThermodynamicTickGuard {
    last_admitted_elapsed: Option<std::time::Duration>,
    /// Number of fixed-tick invocations admitted since initialization.
    pub admitted_ticks: u64,
    /// Duplicate invocations for an already admitted fixed clock value.
    pub duplicate_invocations: u64,
    /// Out-of-order clock values rejected without changing the watermark.
    pub regressed_invocations: u64,
}

impl ThermodynamicTickGuard {
    /// Admit each strictly increasing Fixed-time identity at most once.
    fn admit(&mut self, elapsed: std::time::Duration) -> bool {
        match self.last_admitted_elapsed {
            None => {
                self.last_admitted_elapsed = Some(elapsed);
                self.admitted_ticks = self.admitted_ticks.saturating_add(1);
                true
            }
            Some(last) if elapsed > last => {
                self.last_admitted_elapsed = Some(elapsed);
                self.admitted_ticks = self.admitted_ticks.saturating_add(1);
                true
            }
            Some(last) if elapsed == last => {
                self.duplicate_invocations = self.duplicate_invocations.saturating_add(1);
                false
            }
            Some(_) => {
                self.regressed_invocations = self.regressed_invocations.saturating_add(1);
                false
            }
        }
    }
}

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
/// checks for collapse, and handles harmony resonance energy transfer.
pub fn thermodynamic_enforcement_system(
    fixed_time: Res<Time<Fixed>>,
    mut tick_guard: ResMut<ThermodynamicTickGuard>,
    mut physics: ResMut<PhysicsWorldRes>,
    mut hud_state: ResMut<ThermodynamicHudState>,
    entities_query: Query<(&PhysicsBody, &Transform), Or<(With<Player>, With<CrewNpc>)>>,
    mut wells: Query<(&Transform, &mut EnergyWell, &mut Sprite), Without<Player>>,
) {
    // Reject duplicate or out-of-order calls before any reservoir, ledger, or HUD side effects.
    if !tick_guard.admit(fixed_time.elapsed()) {
        return;
    }

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
                let regen = well.regen_rate.min(well.remaining);
                well.remaining -= regen;
                if let Some(entity) = physics.consciousness.entities.get_mut(&handle) {
                    entity.energy.regenerate(regen);
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
mod tick_guard_tests {
    use super::*;

    #[test]
    fn fixed_tick_guard_admits_same_identity_once() {
        let mut guard = ThermodynamicTickGuard::default();
        let tick = std::time::Duration::from_millis(16);
        assert!(guard.admit(tick));
        assert!(!guard.admit(tick));
        assert!(!guard.admit(tick));
        assert_eq!(guard.admitted_ticks, 1);
        assert_eq!(guard.duplicate_invocations, 2);
        assert_eq!(guard.regressed_invocations, 0);
    }

    #[test]
    fn fixed_tick_guard_admits_monotonic_identities_once() {
        let mut guard = ThermodynamicTickGuard::default();
        for millis in [16, 32, 48, 64] {
            assert!(guard.admit(std::time::Duration::from_millis(millis)));
            assert!(!guard.admit(std::time::Duration::from_millis(millis)));
        }
        assert_eq!(guard.admitted_ticks, 4);
        assert_eq!(guard.duplicate_invocations, 4);
    }

    #[test]
    fn fixed_tick_guard_rejects_regressed_identity_without_rolling_back_watermark() {
        let mut guard = ThermodynamicTickGuard::default();
        assert!(guard.admit(std::time::Duration::from_millis(32)));
        assert!(!guard.admit(std::time::Duration::from_millis(16)));
        assert!(!guard.admit(std::time::Duration::from_millis(32)));
        assert!(guard.admit(std::time::Duration::from_millis(48)));
        assert_eq!(guard.admitted_ticks, 2);
        assert_eq!(guard.duplicate_invocations, 1);
        assert_eq!(guard.regressed_invocations, 1);
        assert_eq!(
            guard.last_admitted_elapsed,
            Some(std::time::Duration::from_millis(48))
        );
    }

    #[test]
    fn seeded_million_tick_tape_admits_each_unique_tick_once() {
        const UNIQUE_TICKS: u64 = 1_000_000;
        let mut guard = ThermodynamicTickGuard::default();
        let mut state = 0x7f4a_7c15_d1ce_4e5b_u64;
        let mut expected_duplicates = 0_u64;

        for tick in 1..=UNIQUE_TICKS {
            let identity = std::time::Duration::from_nanos(tick);
            assert!(guard.admit(identity), "unique tick {tick} was rejected");

            // Deterministically choose 1-5 duplicate invocations per unique tick.
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let duplicates = (state % 5) + 1;
            expected_duplicates += duplicates;

            for _ in 0..duplicates {
                assert!(!guard.admit(identity), "duplicate tick {tick} was admitted");
            }
        }

        assert_eq!(guard.admitted_ticks, UNIQUE_TICKS);
        assert_eq!(guard.duplicate_invocations, expected_duplicates);
        assert_eq!(guard.regressed_invocations, 0);
    }

    #[test]
    fn zero_elapsed_identity_is_not_a_sentinel() {
        let mut guard = ThermodynamicTickGuard::default();
        let zero = std::time::Duration::ZERO;
        assert!(guard.admit(zero));
        assert!(!guard.admit(zero));
        assert_eq!(guard.admitted_ticks, 1);
        assert_eq!(guard.duplicate_invocations, 1);
    }
}
