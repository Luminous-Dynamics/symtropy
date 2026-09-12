// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Integration field: the central coupling between Φ (integrated information) and physics.
//!
//! Φ is a formal measure from IIT (Tononi 2004) quantifying how much a system's
//! information exceeds the sum of its parts. It is a mathematical property of
//! causal structure — NOT a claim about subjective experience or sentience.
//!
//! Each entity has an [`EntityConsciousness`] (read: entity integration state) that
//! computes Φ from sensory/behavioral inputs and gates physics via safety tiers.
//! The [`ConsciousnessField`] (read: integration field) aggregates all entities
//! and provides modulation functions called during physics simulation.
//!
//! Operational-budget and semantic couplings in this module are deliberately
//! distinct from the authoritative physical-energy ledger. Collision impulse,
//! harmony, prediction error, and sanctuary attenuation are not Joules unless a
//! separately validated conversion establishes source, destination, and mechanism.

use std::collections::BTreeMap;
use std::collections::VecDeque;

use nalgebra::SVector;
use symthaea_consciousness_equation::{
    ConsciousnessInputs, ConsciousnessResult, MasterConsciousnessEquation,
};

use crate::energy::EnergyBudget;
use crate::harmony_field::{HarmonyField, HarmonySource};
use crate::safety::SafetyTier;
use crate::sanctuary::{SanctuaryConditions, SanctuaryZone};
use crate::thermodynamics::{ThermodynamicConstants, ThermodynamicLedger};
use symtropy_physics::body::BodyHandle;
use symtropy_physics::world::PhysicsCallback;

/// Persistent agent memory for temporal state tracking.
///
/// Gives agents something beyond single-tick perception: remembered well
/// locations, partner history, and hunger urgency. This is NOT learning
/// (no weight updates) — it's episodic memory that the FEP gradient can use.
#[derive(Debug, Clone)]
pub struct AgentMemory {
    /// Known energy well positions (discovered when within range).
    pub known_wells: Vec<SVector<f64, 2>>,
    /// Resonance history with specific agents: (handle_bits, cumulative_resonance, encounters).
    pub partner_history: Vec<(u64, f64, u32)>,
    /// Ticks since last positive energy regeneration.
    pub ticks_since_regen: u64,
    /// Rolling energy average over last 100 ticks (for reward signal).
    pub energy_window: VecDeque<f64>,
    /// Maximum partner history entries.
    pub max_partners: usize,
}

impl Default for AgentMemory {
    fn default() -> Self {
        Self {
            known_wells: Vec::new(),
            partner_history: Vec::new(),
            ticks_since_regen: 0,
            energy_window: VecDeque::with_capacity(100),
            max_partners: 20,
        }
    }
}

impl AgentMemory {
    /// Record an energy observation for the rolling window.
    pub fn record_energy(&mut self, energy_fraction: f64) {
        self.energy_window.push_back(energy_fraction);
        if self.energy_window.len() > 100 {
            self.energy_window.pop_front();
        }
    }

    /// Windowed reward: average energy change over window.
    /// Positive = gaining energy on average. Negative = losing.
    pub fn windowed_reward(&self) -> f64 {
        if self.energy_window.len() < 2 {
            return 0.0;
        }
        let n = self.energy_window.len();
        let half = n / 2;
        let first_half: f64 = self.energy_window.iter().take(half).sum::<f64>() / half as f64;
        let second_half: f64 =
            self.energy_window.iter().skip(half).sum::<f64>() / (n - half) as f64;
        second_half - first_half
    }

    /// Record a discovered well position (deduplicated by proximity).
    pub fn discover_well(&mut self, pos: &SVector<f64, 2>) {
        let dominated = self.known_wells.iter().any(|w| (w - pos).norm() < 10.0);
        if !dominated {
            self.known_wells.push(*pos);
        }
    }

    /// Record a cooperation event with another agent.
    pub fn record_partner(&mut self, handle: symtropy_physics::BodyHandle, resonance: f64) {
        let bits = handle.0 as u64;
        if let Some(entry) = self.partner_history.iter_mut().find(|(h, _, _)| *h == bits) {
            entry.1 += resonance;
            entry.2 += 1;
        } else {
            if self.partner_history.len() >= self.max_partners {
                if let Some(min_idx) = self
                    .partner_history
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, (_, _, c))| *c)
                    .map(|(i, _)| i)
                {
                    self.partner_history.swap_remove(min_idx);
                }
            }
            self.partner_history.push((bits, resonance, 1));
        }
    }

    /// Mean resonance with known partners.
    pub fn mean_partner_resonance(&self) -> f64 {
        if self.partner_history.is_empty() {
            return 0.0;
        }
        let total: f64 = self
            .partner_history
            .iter()
            .map(|(_, res, count)| if *count > 0 { res / *count as f64 } else { 0.0 })
            .sum();
        total / self.partner_history.len() as f64
    }
}

/// Consciousness state for a single entity in the physics world.
pub struct EntityConsciousness {
    pub equation: MasterConsciousnessEquation,
    pub result: Option<ConsciousnessResult>,
    pub safety_tier: SafetyTier,
    pub energy: EnergyBudget,
    pub harmony_activations: [f64; crate::harmony_field::NUM_HARMONIES],
    pub prediction_error: f64,
    pub motor_precision: f64,
    pub prediction_decay: f64,
    pub memory: AgentMemory,
    pub power_drain_watts: f64,
}

impl EntityConsciousness {
    pub fn new(max_energy: f64) -> Self {
        Self {
            equation: MasterConsciousnessEquation::default(),
            result: None,
            safety_tier: SafetyTier::Green,
            energy: EnergyBudget::new(max_energy),
            harmony_activations: [0.0; crate::harmony_field::NUM_HARMONIES],
            prediction_error: 0.0,
            motor_precision: 1.0,
            prediction_decay: 0.05,
            memory: AgentMemory::default(),
            power_drain_watts: 0.0,
        }
    }

    pub fn compute(&mut self, inputs: &ConsciousnessInputs) {
        let result = self.equation.compute(inputs);
        let phi = result.consciousness_level;
        self.safety_tier = if self.energy.is_collapsed() {
            SafetyTier::Red
        } else {
            SafetyTier::from_phi(phi)
        };
        self.result = Some(result);
    }

    pub fn phi(&self) -> f64 {
        self.result
            .as_ref()
            .map(|r| r.consciousness_level)
            .unwrap_or(0.0)
    }

    pub fn bottleneck(&self) -> &str {
        self.result
            .as_ref()
            .map(|r| r.bottleneck_name.as_str())
            .unwrap_or("uncomputed")
    }

    pub fn stillness(&self) -> f64 {
        self.harmony_activations[7]
    }

    pub fn total_harmony_energy(&self) -> f64 {
        self.harmony_activations.iter().sum()
    }

    pub fn effective_motor_gain(&self) -> f64 {
        self.safety_tier.motor_gain() * self.motor_precision
    }

    /// Register collision impulse as semantic prediction-error evidence.
    ///
    /// Impulse is not converted into Joules here. Invalid impulse evidence fails
    /// closed to Red authority without introducing NaN into the state.
    pub fn on_collision(&mut self, impulse_magnitude: f64) {
        if !impulse_magnitude.is_finite() || impulse_magnitude < 0.0 {
            self.prediction_error = if self.prediction_error.is_finite() {
                self.prediction_error.max(2.0)
            } else {
                2.0
            };
            self.motor_precision = 0.0;
            self.safety_tier = SafetyTier::Red;
            return;
        }
        let error_spike = (impulse_magnitude * 0.01).min(2.0);
        self.prediction_error += error_spike;
        self.motor_precision = 1.0 / (1.0 + self.prediction_error);
    }

    pub fn tick_prediction_error(&mut self) {
        if !self.prediction_error.is_finite() || !self.prediction_decay.is_finite() {
            self.prediction_error = 2.0;
            self.motor_precision = 0.0;
            self.safety_tier = SafetyTier::Red;
            return;
        }
        self.prediction_error *= 1.0 - self.prediction_decay;
        if self.prediction_error < 1e-6 {
            self.prediction_error = 0.0;
        }
        self.motor_precision = 1.0 / (1.0 + self.prediction_error);
    }

    pub fn sanctuary_conditions(&self) -> SanctuaryConditions {
        SanctuaryConditions {
            stillness_activation: self.stillness(),
            total_harmony_energy: self.total_harmony_energy(),
            phi: self.phi(),
        }
    }
}

pub struct ConsciousnessField<const D: usize> {
    pub entities: BTreeMap<BodyHandle, EntityConsciousness>,
    pub sanctuaries: BTreeMap<BodyHandle, SanctuaryZone<D>>,
    pub harmony_field: HarmonyField<D>,
    entity_positions: BTreeMap<BodyHandle, SVector<f64, D>>,
    pub phi_gravity_strength: f64,
    pub collective_phi: f64,
    /// Legacy operational telemetry only; not the core physical energy ledger.
    pub ledger: ThermodynamicLedger,
    pub constants: ThermodynamicConstants,
}

impl<const D: usize> ConsciousnessField<D> {
    pub fn new() -> Self {
        Self {
            entities: BTreeMap::new(),
            sanctuaries: BTreeMap::new(),
            harmony_field: HarmonyField::new(),
            entity_positions: BTreeMap::new(),
            phi_gravity_strength: 0.0,
            collective_phi: 0.0,
            ledger: ThermodynamicLedger::new(),
            constants: ThermodynamicConstants::default(),
        }
    }

    pub fn register(&mut self, handle: BodyHandle, max_energy: f64, sanctuary_radius: f64) {
        self.entities
            .insert(handle, EntityConsciousness::new(max_energy));
        self.sanctuaries.insert(
            handle,
            SanctuaryZone::new(symtropy_math::Point::origin(), sanctuary_radius),
        );
    }

    pub fn update_entity(
        &mut self,
        handle: BodyHandle,
        inputs: &ConsciousnessInputs,
        position: symtropy_math::Point<D>,
    ) {
        if let Some(entity) = self.entities.get_mut(&handle) {
            let temp_penalty =
                crate::thermodynamics::smooth_temperature_penalty(entity.energy.temperature);
            let effective_inputs = ConsciousnessInputs {
                phi: inputs.phi * temp_penalty,
                broadcast: inputs.broadcast * temp_penalty,
                working_memory: inputs.working_memory * temp_penalty,
                attention: inputs.attention * temp_penalty,
                recurrence: inputs.recurrence,
                embodiment: inputs.embodiment,
                knowledge: inputs.knowledge,
                synchrony: inputs.synchrony * temp_penalty,
            };

            let phi_before = entity.phi();
            entity.compute(&effective_inputs);
            let phi_after = entity.phi();
            let delta_phi = (phi_after - phi_before).abs();
            if delta_phi > 1e-10 {
                self.ledger.record_phi_change(delta_phi);
            }

            if let Some(sanctuary) = self.sanctuaries.get_mut(&handle) {
                let conditions = entity.sanctuary_conditions();
                sanctuary.update(&conditions, position);
            }
        }
        self.recompute_collective();
    }

    pub fn modulate_force(&self, handle: BodyHandle, force: &SVector<f64, D>) -> SVector<f64, D> {
        let gain = self
            .entities
            .get(&handle)
            .map(|e| e.effective_motor_gain())
            .unwrap_or(1.0);
        force * gain
    }

    /// Collision events feed semantic prediction error only.
    ///
    /// The old `impulse * 0.1` ledger write was dimensionally invalid and has
    /// been removed. Physical collision dissipation must come from measured
    /// pre/post mechanical energy, not impulse magnitude alone.
    pub fn process_collisions(&mut self, events: &[symtropy_physics::CollisionEvent<D>]) {
        for event in events {
            if let Some(entity) = self.entities.get_mut(&event.body_a) {
                entity.on_collision(event.impulse);
            }
            if let Some(entity) = self.entities.get_mut(&event.body_b) {
                entity.on_collision(event.impulse);
            }
        }
    }

    pub fn tick_prediction_errors(&mut self) {
        for entity in self.entities.values_mut() {
            entity.tick_prediction_error();
        }
    }

    /// Modulate an impulse through sanctuary attenuation.
    ///
    /// Absorbed impulse remains an impulse-domain quantity. It is not converted
    /// to heat with an arbitrary `* 0.5` factor.
    pub fn modulate_impulse(&self, impulse: f64, contact_point: &SVector<f64, D>) -> f64 {
        if !impulse.is_finite() || impulse < 0.0 || !contact_point.iter().all(|v| v.is_finite()) {
            return 0.0;
        }
        let mut multiplier: f64 = 1.0;
        for sanctuary in self.sanctuaries.values() {
            let m = sanctuary.impulse_multiplier(contact_point);
            if m.is_finite() {
                multiplier = multiplier.min(m.clamp(0.0, 1.0));
            }
        }
        impulse * multiplier
    }

    pub fn consume_energy(&mut self, handle: BodyHandle, amount: f64) -> f64 {
        let phi = self.phi(handle);
        let dimensional_multiplier = 1.0 + (D as f64 - 3.0).max(0.0) * 0.1;
        let scaled_amount = amount * dimensional_multiplier;

        let consumed = self
            .entities
            .get_mut(&handle)
            .map(|e| e.energy.consume(scaled_amount))
            .unwrap_or(0.0);
        if consumed > 0.0 {
            self.ledger.record_action(consumed, phi);
        }
        consumed
    }

    /// Legacy dissipation-like telemetry only.
    pub fn record_dissipation(&mut self, energy: f64) {
        self.ledger.record_dissipation(energy);
    }

    pub fn tick_thermodynamics(&mut self) -> crate::thermodynamics::TickBalance {
        self.ledger.tick_balance()
    }

    pub fn has_energy(&self, handle: BodyHandle) -> bool {
        self.entities
            .get(&handle)
            .map(|e| e.energy.has_energy())
            .unwrap_or(false)
    }

    pub fn phi(&self, handle: BodyHandle) -> f64 {
        self.entities.get(&handle).map(|e| e.phi()).unwrap_or(0.0)
    }

    pub fn bottleneck(&self, handle: BodyHandle) -> &str {
        self.entities
            .get(&handle)
            .map(|e| e.bottleneck())
            .unwrap_or("unknown")
    }

    pub fn safety_tier(&self, handle: BodyHandle) -> SafetyTier {
        self.entities
            .get(&handle)
            .map(|e| e.safety_tier)
            .unwrap_or(SafetyTier::Green)
    }

    pub fn resource_regeneration_multiplier(&self) -> f64 {
        0.5 + 1.5 * self.collective_phi
    }

    pub fn rebuild_harmony_field(&mut self, positions: &[(BodyHandle, symtropy_math::Point<D>)]) {
        self.harmony_field.sources.clear();
        self.entity_positions.clear();
        for (handle, pos) in positions {
            self.entity_positions.insert(*handle, pos.to_vector());
            if let Some(entity) = self.entities.get(handle) {
                self.harmony_field.sources.push(HarmonySource {
                    position: *pos,
                    activations: entity.harmony_activations,
                    strength: entity.phi().max(0.1),
                    radius: self.constants.harmony_range,
                    created_at: 0.0,
                    propagation_speed: f64::MAX,
                });
            }
        }
    }

    pub fn spread_emotional_contagion(
        &mut self,
        positions: &[(BodyHandle, symtropy_math::Point<D>)],
        dt: f64,
    ) {
        use crate::harmony_field::{EMOTIONAL_CONTAGION_IDX, contagion_update};

        let sources: Vec<(SVector<f64, D>, f64, f64)> = positions
            .iter()
            .filter_map(|(h, pos)| {
                self.entities.get(h).map(|e| {
                    (
                        pos.0,
                        e.harmony_activations[EMOTIONAL_CONTAGION_IDX],
                        e.phi(),
                    )
                })
            })
            .collect();

        for (handle, pos) in positions {
            let self_pos = pos.0;
            if let Some(entity) = self.entities.get_mut(handle) {
                let own_emotion = entity.harmony_activations[EMOTIONAL_CONTAGION_IDX];
                let own_phi = entity.phi();
                let others: Vec<_> = sources
                    .iter()
                    .filter(|(p, _, _)| (p - self_pos).norm() > 1e-6)
                    .cloned()
                    .collect();
                entity.harmony_activations[EMOTIONAL_CONTAGION_IDX] =
                    contagion_update(&self_pos, own_emotion, own_phi, &others, dt);
            }
        }
    }

    /// Apply dimensional leakage to the operational budget with explicit
    /// boundary direction. This is still a modeled gameplay boundary, not the
    /// core physical energy ledger.
    pub fn apply_dimensional_leakage(
        &mut self,
        leakage: &crate::dimensional_leakage::DimensionalLeakage<D>,
        positions: &[(BodyHandle, SVector<f64, D>)],
    ) {
        if !leakage.enabled {
            return;
        }
        for (handle, pos) in positions {
            let effect = leakage.total_effect_at(pos);
            if !effect.is_finite() {
                self.ledger.rejected_event_count = self.ledger.rejected_event_count.saturating_add(1);
                continue;
            }
            if let Some(entity) = self.entities.get_mut(handle) {
                if effect > 0.0 {
                    let before = entity.energy.available;
                    entity.energy.regenerate(effect);
                    let accepted = entity.energy.available - before;
                    if accepted.is_finite() && accepted > 0.0 {
                        self.ledger.record_boundary_inflow(accepted);
                    }
                } else if effect < 0.0 {
                    let consumed = entity.energy.consume(effect.abs());
                    if consumed.is_finite() && consumed > 0.0 {
                        self.ledger.record_boundary_outflow(consumed);
                    }
                }
            }
        }
    }

    fn recompute_collective(&mut self) {
        if self.entities.is_empty() {
            self.collective_phi = 0.0;
            return;
        }
        let sum: f64 = self.entities.values().map(|e| e.phi()).sum();
        self.collective_phi = sum / self.entities.len() as f64;
    }
}

impl<const D: usize> Default for ConsciousnessField<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const D: usize> PhysicsCallback<D> for ConsciousnessField<D> {
    fn modulate_force(&self, body: BodyHandle, force: &SVector<f64, D>) -> SVector<f64, D> {
        let gain = self
            .entities
            .get(&body)
            .map(|e| e.effective_motor_gain())
            .unwrap_or(1.0);
        let mut result = force * gain;

        #[allow(clippy::collapsible_if)]
        if self.phi_gravity_strength > 1e-10 {
            if let (Some(pos), Some(entity)) =
                (self.entity_positions.get(&body), self.entities.get(&body))
            {
                let self_phi = entity.phi();
                if self_phi > 0.01 {
                    let nearby: Vec<_> = self
                        .entity_positions
                        .iter()
                        .filter(|(h, _)| **h != body)
                        .filter_map(|(h, p)| self.entities.get(h).map(|e| (*p, e.phi())))
                        .collect();
                    let gravity = crate::fep_gradient::phi_gravity(
                        pos,
                        self_phi,
                        &nearby,
                        self.phi_gravity_strength,
                    );
                    result += gravity;
                }
            }
        }

        result
    }

    fn modulate_impulse(&self, impulse: f64, contact_point: &SVector<f64, D>) -> f64 {
        ConsciousnessField::modulate_impulse(self, impulse, contact_point)
    }

    fn friction_multiplier(&self, contact_point: &SVector<f64, D>, body: BodyHandle) -> f64 {
        if self.harmony_field.sources.is_empty() {
            return 1.0;
        }
        let entity_harmonies = self
            .entities
            .get(&body)
            .map(|e| e.harmony_activations)
            .unwrap_or([0.0; crate::harmony_field::NUM_HARMONIES]);
        let point = symtropy_math::Point(*contact_point);
        self.harmony_field
            .friction_multiplier(&point, &entity_harmonies)
    }

    fn on_collision(&mut self, event: &symtropy_physics::CollisionEvent<D>) {
        let drain_rate = self.constants.collision_energy_drain;
        if !event.impulse.is_finite() || event.impulse < 0.0 || !drain_rate.is_finite() || drain_rate < 0.0 {
            for handle in [event.body_a, event.body_b] {
                if let Some(entity) = self.entities.get_mut(&handle) {
                    entity.on_collision(f64::NAN);
                }
            }
            return;
        }

        let resonance = {
            let harm_a = self
                .entities
                .get(&event.body_a)
                .map(|e| e.harmony_activations);
            let harm_b = self
                .entities
                .get(&event.body_b)
                .map(|e| e.harmony_activations);
            match (harm_a, harm_b) {
                (Some(a), Some(b)) => crate::harmony_field::HarmonyField::<D>::resonance(&a, &b),
                _ => 0.0,
            }
        };
        let resonance = if resonance.is_finite() {
            resonance.clamp(0.0, 1.0)
        } else {
            0.0
        };

        let phi_a = self.phi(event.body_a);
        let phi_b = self.phi(event.body_b);

        if let Some(entity) = self.entities.get_mut(&event.body_a) {
            let surprise_factor = (1.0 - resonance).max(0.1);
            entity.on_collision(event.impulse * surprise_factor);
            let drain = event.impulse * drain_rate;
            if drain.is_finite() && drain > 0.0 {
                let consumed = entity.energy.consume(drain);
                if consumed.is_finite() && consumed > 0.0 {
                    self.ledger.record_action(consumed, phi_a);
                }
            }
        }
        if let Some(entity) = self.entities.get_mut(&event.body_b) {
            let surprise_factor = (1.0 - resonance).max(0.1);
            entity.on_collision(event.impulse * surprise_factor);
            let drain = event.impulse * drain_rate;
            if drain.is_finite() && drain > 0.0 {
                let consumed = entity.energy.consume(drain);
                if consumed.is_finite() && consumed > 0.0 {
                    self.ledger.record_action(consumed, phi_b);
                }
            }
        }
    }

    fn record_dissipation(&mut self, energy: f64) {
        // The current world solver's dissipation callback is not yet guaranteed
        // to be measured physical energy (some paths use impulse-based proxies).
        // Keep it in legacy telemetry only and do not mutate physical temperature.
        let entropy_multiplier = 1.0 + (D as f64 - 3.0).max(0.0) * 0.05;
        let scaled_dissipation = energy * entropy_multiplier;
        self.ledger.record_dissipation(scaled_dissipation);
    }

    fn record_work(&mut self, body: BodyHandle, work_joules: f64) {
        if !work_joules.is_finite() {
            if let Some(entity) = self.entities.get_mut(&body) {
                entity.safety_tier = SafetyTier::Red;
                entity.motor_precision = 0.0;
            }
            self.ledger.rejected_event_count = self.ledger.rejected_event_count.saturating_add(1);
            return;
        }

        let phi = self.phi(body);
        if let Some(entity) = self.entities.get_mut(&body) {
            if work_joules > 0.0 {
                let actual_consumed = entity.energy.consume(work_joules);
                if actual_consumed.is_finite() && actual_consumed > 0.0 {
                    self.ledger.record_action(actual_consumed, phi);
                }
            } else if work_joules < 0.0 {
                let offered = work_joules.abs() * 0.4;
                if offered.is_finite() && offered > 0.0 {
                    let before = entity.energy.available;
                    entity.energy.regenerate(offered);
                    let accepted = entity.energy.available - before;
                    if accepted.is_finite() && accepted > 0.0 {
                        // This is an operational-boundary credit until a validated
                        // mechanical→stored-energy conversion is modeled physically.
                        self.ledger.record_boundary_inflow(accepted);
                    }
                }
            }
        }
    }

    fn apply_trauma(&mut self, _: &symtropy_physics::CollisionEvent<D>) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_inputs(phi: f64) -> ConsciousnessInputs {
        ConsciousnessInputs {
            phi,
            broadcast: 0.8,
            working_memory: 0.7,
            attention: 0.6,
            recurrence: 0.5,
            embodiment: 0.7,
            knowledge: 0.6,
            synchrony: 0.8,
        }
    }

    fn collision<const D: usize>(a: BodyHandle, b: BodyHandle, impulse: f64) -> symtropy_physics::CollisionEvent<D> {
        symtropy_physics::CollisionEvent {
            body_a: a,
            body_b: b,
            impulse,
            normal: SVector::zeros(),
            depth: 0.0,
        }
    }

    #[test]
    fn register_and_update() {
        let mut field = ConsciousnessField::<3>::new();
        let handle = BodyHandle(0);
        field.register(handle, 100.0, 10.0);
        field.update_entity(handle, &test_inputs(0.8), symtropy_math::Point::origin());
        let phi = field.phi(handle);
        assert!(phi >= 0.0);
        assert!(field.entities.get(&handle).unwrap().result.is_some());
    }

    #[test]
    fn high_inputs_give_more_force_than_low() {
        let mut field = ConsciousnessField::<3>::new();
        let h = BodyHandle(0);
        field.register(h, 100.0, 10.0);
        field.update_entity(h, &test_inputs(0.9), symtropy_math::Point::origin());
        let force = SVector::from([10.0, 0.0, 0.0]);
        let high_force = field.modulate_force(h, &force);
        let low_inputs = ConsciousnessInputs {
            phi: 0.05,
            broadcast: 0.05,
            working_memory: 0.05,
            attention: 0.05,
            recurrence: 0.05,
            embodiment: 0.05,
            knowledge: 0.05,
            synchrony: 0.05,
        };
        field.update_entity(h, &low_inputs, symtropy_math::Point::origin());
        let low_force = field.modulate_force(h, &force);
        assert!(high_force[0] >= low_force[0]);
    }

    #[test]
    fn very_low_inputs_minimal_force() {
        let mut field = ConsciousnessField::<3>::new();
        let h = BodyHandle(0);
        field.register(h, 100.0, 10.0);
        let low_inputs = ConsciousnessInputs {
            phi: 0.01,
            broadcast: 0.01,
            working_memory: 0.01,
            attention: 0.01,
            recurrence: 0.01,
            embodiment: 0.01,
            knowledge: 0.01,
            synchrony: 0.01,
        };
        field.update_entity(h, &low_inputs, symtropy_math::Point::origin());
        let force = SVector::from([10.0, 0.0, 0.0]);
        let modulated = field.modulate_force(h, &force);
        assert!(modulated[0] < 5.0);
    }

    #[test]
    fn invalid_collision_evidence_fails_closed_without_nan() {
        let mut entity = EntityConsciousness::new(100.0);
        entity.on_collision(f64::NAN);
        assert_eq!(entity.safety_tier, SafetyTier::Red);
        assert_eq!(entity.motor_precision, 0.0);
        assert!(entity.prediction_error.is_finite());
    }

    #[test]
    fn sanctuary_dampens_impulse_without_creating_heat_telemetry() {
        let mut field = ConsciousnessField::<3>::new();
        let h = BodyHandle(0);
        field.register(h, 100.0, 10.0);
        field.entities.get_mut(&h).unwrap().harmony_activations = [
            0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.9, 0.0,
        ];
        field.update_entity(h, &test_inputs(0.9), symtropy_math::Point::origin());
        if field.phi(h) <= 0.3 {
            field.sanctuaries.get_mut(&h).unwrap().active = true;
            field.sanctuaries.get_mut(&h).unwrap().dampening = 0.7;
        }
        let before = field.ledger.energy_out;
        let impulse = 100.0;
        let dampened = field.modulate_impulse(impulse, &SVector::zeros());
        assert!(dampened < impulse);
        assert_eq!(field.ledger.energy_out, before);
    }

    #[test]
    fn impulse_outside_sanctuary_unaffected() {
        let mut field = ConsciousnessField::<3>::new();
        let h = BodyHandle(0);
        field.register(h, 100.0, 10.0);
        field.update_entity(h, &test_inputs(0.8), symtropy_math::Point::origin());
        let impulse = 100.0;
        let result = field.modulate_impulse(impulse, &SVector::from([100.0, 0.0, 0.0]));
        assert!((result - impulse).abs() < 1e-10);
    }

    #[test]
    fn energy_consumption() {
        let mut field = ConsciousnessField::<3>::new();
        let h = BodyHandle(0);
        field.register(h, 100.0, 10.0);
        field.update_entity(h, &test_inputs(0.8), symtropy_math::Point::origin());
        let available = field.entities.get(&h).unwrap().energy.available;
        if available > 0.0 {
            assert!(field.has_energy(h));
            let consumed = field.consume_energy(h, available * 0.5);
            assert!(consumed > 0.0);
            field.consume_energy(h, available);
            assert!(!field.has_energy(h));
        }
    }

    #[test]
    fn process_collisions_is_semantic_not_heat_accounting() {
        let mut field = ConsciousnessField::<3>::new();
        let a = BodyHandle(0);
        let b = BodyHandle(1);
        field.register(a, 100.0, 10.0);
        field.register(b, 100.0, 10.0);
        field.process_collisions(&[collision::<3>(a, b, 50.0)]);
        assert_eq!(field.ledger.energy_out, 0.0);
        assert!(field.entities.get(&a).unwrap().prediction_error > 0.0);
        assert!(field.entities.get(&b).unwrap().prediction_error > 0.0);
    }

    #[test]
    fn callback_collision_cost_does_not_invent_temperature_or_dissipation() {
        let mut field = ConsciousnessField::<3>::new();
        let a = BodyHandle(0);
        let b = BodyHandle(1);
        field.register(a, 100.0, 10.0);
        field.register(b, 100.0, 10.0);
        let temp_a = field.entities.get(&a).unwrap().energy.temperature;
        let temp_b = field.entities.get(&b).unwrap().energy.temperature;
        PhysicsCallback::<3>::on_collision(&mut field, &collision::<3>(a, b, 20.0));
        assert_eq!(field.entities.get(&a).unwrap().energy.temperature, temp_a);
        assert_eq!(field.entities.get(&b).unwrap().energy.temperature, temp_b);
        assert_eq!(field.ledger.energy_out, 0.0);
        assert!(field.ledger.energy_in > 0.0);
    }

    #[test]
    fn collective_phi_averages() {
        let mut field = ConsciousnessField::<3>::new();
        field.register(BodyHandle(0), 100.0, 10.0);
        field.register(BodyHandle(1), 100.0, 10.0);
        field.update_entity(BodyHandle(0), &test_inputs(0.9), symtropy_math::Point::origin());
        field.update_entity(BodyHandle(1), &test_inputs(0.9), symtropy_math::Point::origin());
        assert!(field.collective_phi > 0.0);
    }

    #[test]
    fn resource_regen_scales_with_collective() {
        let mut field = ConsciousnessField::<3>::new();
        assert!((field.resource_regeneration_multiplier() - 0.5).abs() < 1e-10);
        field.register(BodyHandle(0), 100.0, 10.0);
        field.update_entity(BodyHandle(0), &test_inputs(0.9), symtropy_math::Point::origin());
        assert!(field.resource_regeneration_multiplier() > 0.5);
    }

    #[test]
    fn unregistered_entity_defaults() {
        let field = ConsciousnessField::<3>::new();
        let h = BodyHandle(99);
        assert_eq!(field.phi(h), 0.0);
        assert_eq!(field.safety_tier(h), SafetyTier::Green);
        assert_eq!(field.bottleneck(h), "unknown");
    }

    #[test]
    fn phi_monotonicity_property() {
        let mut field = ConsciousnessField::<3>::new();
        let h = BodyHandle(0);
        field.register(h, 100.0, 10.0);
        let levels = [0.01, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];
        let mut prev_gain = 0.0;
        for &phi in &levels {
            field.update_entity(h, &test_inputs(phi), symtropy_math::Point::origin());
            let gain = field.entities.get(&h).unwrap().safety_tier.motor_gain();
            assert!(gain >= prev_gain);
            prev_gain = gain;
        }
    }

    #[test]
    fn harmony_friction_reduces_for_resonant_entities() {
        let mut field = ConsciousnessField::<3>::new();
        let h0 = BodyHandle(0);
        let h1 = BodyHandle(1);
        field.register(h0, 100.0, 10.0);
        field.register(h1, 100.0, 10.0);
        field.entities.get_mut(&h0).unwrap().harmony_activations =
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        field.entities.get_mut(&h1).unwrap().harmony_activations =
            [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0];
        field.update_entity(h0, &test_inputs(0.8), symtropy_math::Point::origin());
        field.update_entity(h1, &test_inputs(0.8), symtropy_math::Point::new([5.0, 0.0, 0.0]));
        let positions = [
            (h0, symtropy_math::Point::new([0.0, 0.0, 0.0])),
            (h1, symtropy_math::Point::new([5.0, 0.0, 0.0])),
        ];
        field.rebuild_harmony_field(&positions);
        let contact = SVector::from([2.5, 0.0, 0.0]);
        let mult = PhysicsCallback::<3>::friction_multiplier(&field, &contact, h0);
        assert!(mult < 1.0);
    }

    #[test]
    fn dimensional_leakage_drains_energy_and_records_boundary_outflow() {
        use crate::dimensional_leakage::{DimensionalLeakage, LeakagePoint};
        let mut field = ConsciousnessField::<3>::new();
        let h = BodyHandle(0);
        field.register(h, 100.0, 10.0);
        let energy_before = field.entities.get(&h).unwrap().energy.available;
        let mut leakage = DimensionalLeakage::new();
        leakage.add_point(LeakagePoint::sink(SVector::zeros(), 1.0, 5.0, 50.0));
        leakage.enabled = true;
        field.apply_dimensional_leakage(&leakage, &[(h, SVector::from([1.0, 0.0, 0.0]))]);
        let energy_after = field.entities.get(&h).unwrap().energy.available;
        assert!(energy_after < energy_before);
        assert!((field.ledger.boundary_out - (energy_before - energy_after)).abs() < 1e-10);
        assert_eq!(field.ledger.energy_out, 0.0);
    }

    #[test]
    fn dimensional_leakage_source_records_actual_boundary_inflow() {
        use crate::dimensional_leakage::{DimensionalLeakage, LeakagePoint};
        let mut field = ConsciousnessField::<3>::new();
        let h = BodyHandle(0);
        field.register(h, 100.0, 10.0);
        field.entities.get_mut(&h).unwrap().energy.consume(20.0);
        let before = field.entities.get(&h).unwrap().energy.available;
        let mut leakage = DimensionalLeakage::new();
        leakage.add_point(LeakagePoint::source(SVector::zeros(), 1.0, 5.0, 50.0));
        leakage.enabled = true;
        field.apply_dimensional_leakage(&leakage, &[(h, SVector::from([1.0, 0.0, 0.0]))]);
        let after = field.entities.get(&h).unwrap().energy.available;
        assert!(after > before);
        assert!((field.ledger.boundary_in - (after - before)).abs() < 1e-10);
        assert_eq!(field.ledger.energy_out, 0.0);
    }

    #[test]
    fn dimensional_leakage_disabled_no_effect() {
        use crate::dimensional_leakage::{DimensionalLeakage, LeakagePoint};
        let mut field = ConsciousnessField::<3>::new();
        let h = BodyHandle(0);
        field.register(h, 100.0, 10.0);
        let energy_before = field.entities.get(&h).unwrap().energy.available;
        let mut leakage = DimensionalLeakage::new();
        leakage.add_point(LeakagePoint::sink(SVector::zeros(), 1.0, 5.0, 50.0));
        field.apply_dimensional_leakage(&leakage, &[(h, SVector::zeros())]);
        let energy_after = field.entities.get(&h).unwrap().energy.available;
        assert!((energy_after - energy_before).abs() < 1e-10);
    }

    #[test]
    fn regenerative_work_uses_boundary_credit_not_lifetime_mutation() {
        let mut field = ConsciousnessField::<3>::new();
        let h = BodyHandle(0);
        field.register(h, 100.0, 10.0);
        field.entities.get_mut(&h).unwrap().energy.consume(50.0);
        let lifetime_before = field.ledger.lifetime_energy;
        PhysicsCallback::<3>::record_work(&mut field, h, -20.0);
        assert_eq!(field.ledger.lifetime_energy, lifetime_before);
        assert!(field.ledger.boundary_in > 0.0);
    }

    #[test]
    fn solver_dissipation_telemetry_does_not_mutate_legacy_temperature() {
        let mut field = ConsciousnessField::<3>::new();
        let h = BodyHandle(0);
        field.register(h, 100.0, 10.0);
        let before = field.entities.get(&h).unwrap().energy.temperature;
        PhysicsCallback::<3>::record_dissipation(&mut field, 10.0);
        assert_eq!(field.entities.get(&h).unwrap().energy.temperature, before);
        assert!(field.ledger.energy_out > 0.0);
    }

    #[test]
    fn harmony_friction_neutral_without_rebuild() {
        let field = ConsciousnessField::<3>::new();
        let contact = SVector::zeros();
        let mult = PhysicsCallback::<3>::friction_multiplier(&field, &contact, BodyHandle(0));
        assert!((mult - 1.0).abs() < 1e-10);
    }

    #[test]
    fn emotional_contagion_spreads_between_nearby_entities() {
        use crate::harmony_field::EMOTIONAL_CONTAGION_IDX;
        let mut field = ConsciousnessField::<3>::new();
        let emitter = BodyHandle(0);
        let receiver = BodyHandle(1);
        field.register(emitter, 100.0, 10.0);
        field.register(receiver, 100.0, 10.0);
        field.entities.get_mut(&emitter).unwrap().harmony_activations[EMOTIONAL_CONTAGION_IDX] = 0.9;
        field.entities.get_mut(&receiver).unwrap().harmony_activations[EMOTIONAL_CONTAGION_IDX] = 0.0;
        field.update_entity(emitter, &test_inputs(0.9), symtropy_math::Point::origin());
        field.update_entity(receiver, &test_inputs(0.8), symtropy_math::Point::new([5.0, 0.0, 0.0]));
        field.entities.get_mut(&emitter).unwrap().harmony_activations[EMOTIONAL_CONTAGION_IDX] = 0.9;
        let positions = [
            (emitter, symtropy_math::Point::new([0.0, 0.0, 0.0])),
            (receiver, symtropy_math::Point::new([5.0, 0.0, 0.0])),
        ];
        field.spread_emotional_contagion(&positions, 1.0);
        let receiver_emotion = field.entities.get(&receiver).unwrap().harmony_activations[EMOTIONAL_CONTAGION_IDX];
        assert!(receiver_emotion > 0.0);
    }

    #[test]
    fn emotional_contagion_decays_in_isolation() {
        use crate::harmony_field::EMOTIONAL_CONTAGION_IDX;
        let mut field = ConsciousnessField::<3>::new();
        let h = BodyHandle(0);
        field.register(h, 100.0, 10.0);
        field.entities.get_mut(&h).unwrap().harmony_activations[EMOTIONAL_CONTAGION_IDX] = 0.8;
        let positions = [(h, symtropy_math::Point::<3>::origin())];
        field.spread_emotional_contagion(&positions, 1.0);
        let emotion = field.entities.get(&h).unwrap().harmony_activations[EMOTIONAL_CONTAGION_IDX];
        assert!(emotion < 0.8);
    }

    #[test]
    fn emotional_contagion_does_not_affect_other_harmony_dims() {
        use crate::harmony_field::EMOTIONAL_CONTAGION_IDX;
        let mut field = ConsciousnessField::<3>::new();
        let emitter = BodyHandle(0);
        let receiver = BodyHandle(1);
        field.register(emitter, 100.0, 10.0);
        field.register(receiver, 100.0, 10.0);
        field.entities.get_mut(&receiver).unwrap().harmony_activations[3] = 0.7;
        field.entities.get_mut(&emitter).unwrap().harmony_activations[EMOTIONAL_CONTAGION_IDX] = 0.9;
        let positions = [
            (emitter, symtropy_math::Point::new([0.0, 0.0, 0.0])),
            (receiver, symtropy_math::Point::new([3.0, 0.0, 0.0])),
        ];
        field.spread_emotional_contagion(&positions, 1.0);
        let h3 = field.entities.get(&receiver).unwrap().harmony_activations[3];
        assert!((h3 - 0.7).abs() < 1e-10);
    }
}
