// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Thermodynamic enforcement system.
//!
//! Runs at 64Hz in FixedUpdate. Enforces the operational/capability rules:
//! 1. Moving costs operational energy
//! 2. Consciousness maintenance costs operational energy
//! 3. Collision spikes prediction error (handled in PhysicsCallback)
//! 4. Harmony resonance can reduce duplicated processing cost
//! 5. Sanctuary zones emerge (handled in consciousness coupling)
//! 6. Energy depleted = consciousness collapse
//! 7. Operational energy regenerates from allowed sources
//!
//! This module's `EnergyBudget` path is operational/capability authority, not the
//! core physical first-law ledger. Physical energy remains owned by the core
//! `symtropy-physics` energy/thermal/reconciliation stack (#40/#45/#829).

use bevy::prelude::*;
use std::collections::HashMap;
use symtropy_physics::BodyHandle;

use crate::components::{CrewNpc, Player};
use crate::resources::{EnergyWell, PhysicsWorldRes, SafetyTier};
use symtropy_render_bridge::PhysicsBody;

/// Marker for entities that have collapsed (zero energy).
#[derive(Component)]
pub struct EnergyCollapsed;

/// Thermodynamic HUD state — accumulates per-tick operational data for display.
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
/// operational transfer primitive and returns the accepted amount directly. The standalone
/// launcher stub predates that API and returns unit, so this compatibility bridge derives
/// the accepted amount from its already-bounded before/after state. Source-backed systems
/// use this helper rather than duplicating that inference at each call site.
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

/// Apply ambient operational support only to a live reservoir.
///
/// Ambient support is not an authorized collapse-recovery source. A collapsed
/// entity needs an explicit recovery source such as a finite energy well.
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

/// Transfer energy from one finite source into a registered recipient reservoir.
///
/// The source is debited only by the amount the recipient actually accepted. Invalid
/// source state, invalid offer limits, missing recipients and full recipients are no-ops.
/// Unlike ambient support, an explicit finite source is allowed to recover a collapsed
/// operational reservoir.
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

/// Compute one bounded epistemic-offloading factor per entity.
///
/// The strongest in-range resonant partner wins. Multiple partners cannot compound
/// into an accidental operational-energy source.
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

/// Compute this tick's operational maintenance debit.
///
/// Epistemic offloading reduces duplicated work before debit; it is never a
/// regeneration credit. Non-finite Phi receives no discount and is conservatively
/// costed as the maximum normal in-range Phi (`Phi = 1`). Invalid base configuration
/// or unrepresentable arithmetic returns `None` so authority can fail closed.
fn maintenance_cost_with_offload(
    base_cost: f64,
    phi: f64,
    offload_factor: f64,
) -> Option<f64> {
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

/// Main enforcement system. Runs in FixedUpdate.
///
/// Debits operational consciousness maintenance, applies ambient support only to live
/// reservoirs, checks collapse, handles bounded epistemic-offloading cost reduction,
/// and permits explicit finite wells to recover collapsed reservoirs.
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

    // Pre-compute immutable policy evidence before mutating per-entity reservoirs.
    let regen_mult = physics.consciousness.resource_regeneration_multiplier();
    let offload_factors =
        epistemic_offload_factors(&physics, &handles, constants.harmony_range);

    // --- Per-entity operational costs ---
    for &handle in &handles {
        let phi_is_valid;
        let maintenance_valid;

        {
            let Some(entity) = physics.consciousness.entities.get_mut(&handle) else {
                continue;
            };

            // Reset per-tick operational counters.
            entity.energy.tick_reset();

            let phi = entity.phi();
            phi_is_valid = phi.is_finite();
            let offload_factor = if phi_is_valid {
                offload_factors.get(&handle).copied().unwrap_or(0.0)
            } else {
                // Invalid inference evidence may never unlock a favorable discount.
                0.0
            };

            if offload_factor > 0.0 {
                // Shared predictive structure accelerates surprise recovery once per tick,
                // using only the strongest bounded partner effect.
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

            // Rule 2: maintenance is reduced before debit. No synthetic refund occurs.
            let Some(maintenance) = maintenance_cost_with_offload(
                constants.consciousness_maintenance_per_tick,
                phi,
                offload_factor,
            ) else {
                entity.safety_tier = SafetyTier::Red;
                maintenance_valid = false;
                continue;
            };

            let _ = entity.energy.consume(maintenance);
            maintenance_valid = true;

            if !phi_is_valid {
                entity.safety_tier = SafetyTier::Red;
            }
        }

        if !maintenance_valid {
            continue;
        }

        // Rule 7a: ambient support may sustain/refill a live reservoir, but cannot
        // resurrect collapse. It remains an operational policy, not physical heat.
        let ambient = constants.ambient_regen_rate * regen_mult;
        let _ = regenerate_live_entity_accepted(&mut physics, handle, ambient);

        // Rule 6: collapse always removes motor authority for this tick.
        if let Some(entity) = physics.consciousness.entities.get_mut(&handle)
            && entity.energy.is_collapsed()
        {
            entity.safety_tier = SafetyTier::Red;
        }
    }

    // --- Rule 7b: Energy Wells are explicit finite recovery sources ---
    for (well_tf, mut well, mut well_sprite) in &mut wells {
        if !well.is_active() {
            well_sprite.color = Color::srgba(0.2, 0.2, 0.2, 0.15);
            continue;
        }

        for &(handle, agent_pos) in &agent_data {
            let dist = agent_pos
                .truncate()
                .distance(well_tf.translation.truncate());
            if dist.is_finite() && dist < well.radius {
                let regen_rate = well.regen_rate;
                let _accepted = transfer_from_finite_source(
                    &mut physics,
                    handle,
                    &mut well.remaining,
                    regen_rate,
                );
            }
        }

        // Presentation only.
        let frac = well.fraction_remaining() as f32;
        well_sprite.color = Color::srgba(0.1, 0.8 * frac, 0.6 * frac, 0.2 + 0.3 * frac);
    }

    // Legacy operational telemetry only. This is not physical first-law evidence;
    // #45/#51/#829 converge physical accounting onto the core typed ledger.
    let total_maintenance: f64 = handles
        .iter()
        .filter_map(|h| physics.consciousness.entities.get(h))
        .map(|e| e.energy.consumed_this_tick)
        .sum();
    physics
        .consciousness
        .ledger
        .record_dissipation(total_maintenance);

    // Existing monolithic finalize remains for source compatibility in this tranche.
    // #783/#824 separately own same-tick split and exactly-once transaction identity.
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
    fn offload_reduces_maintenance_without_becoming_a_credit() {
        let base = 0.08;
        let phi = 0.8;
        let raw = maintenance_cost_with_offload(base, phi, 0.0).unwrap();
        let offloaded = maintenance_cost_with_offload(base, phi, 1.0).unwrap();

        assert!(offloaded < raw);
        assert!((raw - offloaded - base * 0.5).abs() < 1e-12);
        assert!(offloaded >= 0.0);
    }

    #[test]
    fn invalid_phi_cannot_create_free_maintenance_or_offload_benefit() {
        let base = 0.08;
        let invalid = maintenance_cost_with_offload(base, f64::NAN, 1.0).unwrap();
        let worst_case = maintenance_cost_with_offload(base, 1.0, 0.0).unwrap();
        assert_eq!(invalid, worst_case);
        assert!(invalid > 0.0);
    }

    #[test]
    fn invalid_base_cost_is_not_silently_free() {
        assert_eq!(maintenance_cost_with_offload(f64::NAN, 0.5, 0.0), None);
        assert_eq!(maintenance_cost_with_offload(-1.0, 0.5, 0.0), None);
        assert_eq!(maintenance_cost_with_offload(f64::INFINITY, 0.5, 0.0), None);
    }

    #[test]
    fn offload_factor_is_bounded_to_one_strongest_partner() {
        let mut factors = HashMap::new();
        let handle = BodyHandle(1);
        accumulate_strongest_factor(&mut factors, handle, 0.4);
        accumulate_strongest_factor(&mut factors, handle, 0.8);
        accumulate_strongest_factor(&mut factors, handle, 10.0);
        assert_eq!(factors.get(&handle).copied(), Some(1.0));

        accumulate_strongest_factor(&mut factors, BodyHandle(2), f64::NAN);
        assert!(!factors.contains_key(&BodyHandle(2)));
    }

    #[test]
    fn invalid_harmony_evidence_cannot_create_resonance() {
        let good = [1.0; 9];
        let mut bad = good;
        bad[3] = f64::NAN;
        assert_eq!(harmony_resonance(&bad, &good), 0.0);

        let huge = [f64::MAX; 9];
        assert_eq!(harmony_resonance(&huge, &huge), 0.0);
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
        assert!(
            (regenerate_entity_accepted(&mut physics, BodyHandle(999_999), 10.0) - 0.0).abs()
                < 1e-10
        );
    }

    #[test]
    fn ambient_cannot_revive_collapsed_entity() {
        let (mut physics, handle) = registered_entity_with_headroom(100.0);
        let entity = physics.consciousness.entities.get(&handle).unwrap();
        assert!(entity.energy.is_collapsed());

        let accepted = regenerate_live_entity_accepted(&mut physics, handle, 10.0);
        assert_eq!(accepted, 0.0);
        let entity = physics.consciousness.entities.get(&handle).unwrap();
        assert_eq!(entity.energy.available, 0.0);
        assert!(entity.energy.is_collapsed());
        assert_eq!(entity.energy.regenerated_this_tick, 0.0);
    }

    #[test]
    fn explicit_finite_source_can_recover_collapsed_entity() {
        let (mut physics, handle) = registered_entity_with_headroom(100.0);
        let mut source = 20.0;
        let accepted = transfer_from_finite_source(&mut physics, handle, &mut source, 10.0);

        assert_eq!(accepted, 10.0);
        assert_eq!(source, 10.0);
        let entity = physics.consciousness.entities.get(&handle).unwrap();
        assert_eq!(entity.energy.available, 10.0);
        assert!(!entity.energy.is_collapsed());
        assert_eq!(entity.energy.regenerated_this_tick, 10.0);
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