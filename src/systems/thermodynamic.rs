// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Fixed-tick thermodynamic enforcement.
//!
//! The thermodynamic lifecycle is deliberately split around the authoritative
//! physics step:
//!
//! `pre-step -> physics callback/solver -> post-step finalization`.
//!
//! This prevents per-tick counters from being finalized before collision-side
//! callback activity exists, while also preventing a post-physics reset from
//! erasing that activity.

use bevy::prelude::*;

use crate::components::{CrewNpc, Player};
use crate::resources::{EnergyWell, PhysicsWorldRes, SafetyTier};
use symtropy_render_bridge::PhysicsBody;

/// Marker for entities that have collapsed (zero energy).
#[derive(Component)]
pub struct EnergyCollapsed;

/// Thermodynamic HUD state — accumulates finalized per-tick data for display.
#[derive(Resource, Default)]
pub struct ThermodynamicHudState {
    pub energy_consumed_accumulator: f64,
    pub energy_regenerated_accumulator: f64,
    pub ticks_accumulated: u32,
    /// Per-second rates (updated periodically).
    pub consumed_per_sec: f64,
    pub regenerated_per_sec: f64,
}

/// FixedUpdate pre-physics thermodynamic phase.
///
/// Owns exactly-once per-tick counter reset and all state that must be available
/// before controller intent is converted into authoritative body motion:
/// maintenance, ambient/well regeneration, collapse/safety state, and
/// resonance/offloading effects.
///
/// This system MUST run before the authoritative physics step. It deliberately
/// does not call `tick_thermodynamics()` and does not sample HUD counters.
pub fn thermodynamic_pre_step_system(
    mut physics: ResMut<PhysicsWorldRes>,
    entities_query: Query<(&PhysicsBody, &Transform), Or<(With<Player>, With<CrewNpc>)>>,
    mut wells: Query<(&Transform, &mut EnergyWell, &mut Sprite), Without<Player>>,
) {
    let constants = physics.consciousness.constants.clone();

    // Positions here represent the authoritative start-of-tick state mirrored
    // to Bevy by the previous fixed tick. The post-step transform sync happens
    // only after this tick's thermodynamic finalization.
    let agent_data: Vec<_> = entities_query
        .iter()
        .map(|(pb, tf)| (pb.handle, tf.translation))
        .collect();
    let handles: Vec<_> = agent_data.iter().map(|(h, _)| *h).collect();

    let regen_mult = physics.consciousness.resource_regeneration_multiplier();

    // --- Per-entity tick opening + costs ---
    for &handle in &handles {
        if let Some(entity) = physics.consciousness.entities.get_mut(&handle) {
            // The only reset in the fixed-tick lifecycle. Callback-side energy
            // activity produced by the later physics step must survive until the
            // post-step finalizer.
            entity.energy.tick_reset();

            // Rule 2: consciousness maintenance cost.
            let phi = entity.phi();
            let maintenance = constants.consciousness_maintenance_per_tick * (1.0 + phi * 0.5);
            entity.energy.consume(maintenance);

            // Rule 7: ambient regeneration.
            let ambient = constants.ambient_regen_rate * regen_mult;
            entity.energy.regenerate(ambient);

            // Rule 6: motor authority must see collapse before player intent is
            // converted into body velocity in the physics-step system.
            if entity.energy.is_collapsed() {
                entity.safety_tier = SafetyTier::Red;
            }
        }
    }

    // --- Energy wells: spatial pre-step regeneration sources ---
    for (well_tf, mut well, mut well_sprite) in &mut wells {
        if !well.is_active() {
            well_sprite.color = Color::srgba(0.2, 0.2, 0.2, 0.15);
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

        let frac = well.fraction_remaining() as f32;
        well_sprite.color = Color::srgba(0.1, 0.8 * frac, 0.6 * frac, 0.2 + 0.3 * frac);
    }

    // Record maintenance after the pre-step consumers have run but before the
    // physics callback can add same-tick collision/prediction costs.
    let total_maintenance: f64 = handles
        .iter()
        .filter_map(|h| physics.consciousness.entities.get(h))
        .map(|e| e.energy.consumed_this_tick)
        .sum();
    physics
        .consciousness
        .ledger
        .record_dissipation(total_maintenance);

    // --- Rule 4: epistemic offloading ---
    // Cooperation reduces processing cost; it does not create energy.
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
                let offload_factor = (resonance - 0.5) * 2.0;

                if let Some(entity) = physics.consciousness.entities.get_mut(&ha) {
                    entity.prediction_error *= 1.0 - offload_factor * 0.1;
                    entity.motor_precision = 1.0 / (1.0 + entity.prediction_error);
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
            }
        }
    }
}

/// FixedUpdate post-physics thermodynamic phase.
///
/// Runs after the authoritative physics step and therefore observes any
/// collision/prediction-energy activity recorded by the physics callback in the
/// same tick. This is the only fixed-tick thermodynamic finalization point.
pub fn thermodynamic_post_step_system(
    mut physics: ResMut<PhysicsWorldRes>,
    mut hud_state: ResMut<ThermodynamicHudState>,
    entities_query: Query<&PhysicsBody, Or<(With<Player>, With<CrewNpc>)>>,
) {
    let handles: Vec<_> = entities_query.iter().map(|pb| pb.handle).collect();

    // Finalize once, after callback-side physics effects exist.
    let _balance = physics.consciousness.tick_thermodynamics();

    let consumed_this_tick: f64 = handles
        .iter()
        .filter_map(|h| physics.consciousness.entities.get(h))
        .map(|entity| entity.energy.consumed_this_tick)
        .sum();
    let regenerated_this_tick: f64 = handles
        .iter()
        .filter_map(|h| physics.consciousness.entities.get(h))
        .map(|entity| entity.energy.regenerated_this_tick)
        .sum();

    accumulate_hud_tick(
        &mut hud_state,
        consumed_this_tick,
        regenerated_this_tick,
    );
}

fn accumulate_hud_tick(hud: &mut ThermodynamicHudState, consumed: f64, regenerated: f64) {
    hud.ticks_accumulated += 1;
    hud.energy_consumed_accumulator += consumed;
    hud.energy_regenerated_accumulator += regenerated;

    if hud.ticks_accumulated >= 16 {
        let seconds = hud.ticks_accumulated as f64 / 64.0;
        hud.consumed_per_sec = hud.energy_consumed_accumulator / seconds;
        hud.regenerated_per_sec = hud.energy_regenerated_accumulator / seconds;
        hud.energy_consumed_accumulator = 0.0;
        hud.energy_regenerated_accumulator = 0.0;
        hud.ticks_accumulated = 0;
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

    #[test]
    fn hud_accumulates_full_finalized_tick_before_rate_rollup() {
        let mut hud = ThermodynamicHudState::default();

        accumulate_hud_tick(&mut hud, 3.5, 1.25);
        assert_eq!(hud.ticks_accumulated, 1);
        assert!((hud.energy_consumed_accumulator - 3.5).abs() < f64::EPSILON);
        assert!((hud.energy_regenerated_accumulator - 1.25).abs() < f64::EPSILON);

        for _ in 1..16 {
            accumulate_hud_tick(&mut hud, 3.5, 1.25);
        }
        assert_eq!(hud.ticks_accumulated, 0);
        assert!((hud.consumed_per_sec - 224.0).abs() < 1.0e-12);
        assert!((hud.regenerated_per_sec - 80.0).abs() < 1.0e-12);
    }

    #[cfg(feature = "consciousness-runtime")]
    #[test]
    fn callback_collision_cost_survives_until_post_step_finalization() {
        use nalgebra::SVector;
        use symtropy_physics::world::PhysicsCallback;
        use symtropy_physics::{BodyHandle, CollisionEvent};

        let mut field = symtropy_consciousness_physics::ConsciousnessField::<2>::new();
        let body_a = BodyHandle(1);
        let body_b = BodyHandle(2);
        field.register(body_a, 100.0, 10.0);
        field.register(body_b, 100.0, 10.0);

        // Model the pre-step contract: one reset followed by a maintenance cost.
        for handle in [body_a, body_b] {
            let entity = field.entities.get_mut(&handle).unwrap();
            entity.energy.tick_reset();
            entity.energy.consume(2.0);
        }

        let before_collision_a = field.entities[&body_a].energy.consumed_this_tick;
        let before_collision_b = field.entities[&body_b].energy.consumed_this_tick;
        assert!((before_collision_a - 2.0).abs() < 1.0e-12);
        assert!((before_collision_b - 2.0).abs() < 1.0e-12);

        let event = CollisionEvent {
            body_a,
            body_b,
            impulse: 10.0,
            normal: SVector::from([1.0, 0.0]),
            depth: 0.1,
        };
        PhysicsCallback::on_collision(&mut field, &event);

        let after_callback_a = field.entities[&body_a].energy.consumed_this_tick;
        let after_callback_b = field.entities[&body_b].energy.consumed_this_tick;
        assert!(after_callback_a > before_collision_a);
        assert!(after_callback_b > before_collision_b);

        // Post-step finalization may drain deferred ledger state, but it must not
        // clear the per-tick counters before the HUD samples them.
        let _balance = field.tick_thermodynamics();
        assert_eq!(
            field.entities[&body_a].energy.consumed_this_tick,
            after_callback_a
        );
        assert_eq!(
            field.entities[&body_b].energy.consumed_this_tick,
            after_callback_b
        );

        // The next tick opening is the operation that clears them.
        field.entities.get_mut(&body_a).unwrap().energy.tick_reset();
        field.entities.get_mut(&body_b).unwrap().energy.tick_reset();
        assert_eq!(field.entities[&body_a].energy.consumed_this_tick, 0.0);
        assert_eq!(field.entities[&body_b].energy.consumed_this_tick, 0.0);
    }
}
