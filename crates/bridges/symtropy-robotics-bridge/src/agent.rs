// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

use bevy::ecs::relationship::Relationship;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use symthaea_bevy_brain::CognitiveBrain;
use symthaea_fep::HapticSemanticBinder;
use symtropy_bevy_core::{BevyPhysics, PhysicsBody};
use symtropy_math::Bivector;

/// Marker for the root entity of a robotic agent (ECS tag component only —
/// for the consciousness-driving agent with `.tick()`/`.phi()`, see
/// [`RoboticAgent`] further down this file).
#[derive(Component, Serialize, Deserialize, Clone, Debug)]
pub struct RoboticAgentTag {
    pub model_name: String,
}

/// Component that maps physical state to semantic HDC space.
#[derive(Component)]
pub struct RoboticHapticBinder {
    pub binder: HapticSemanticBinder,
}

/// Proximity sensor: detects nearby objects within a radius.
#[derive(Component, Serialize, Deserialize, Clone, Debug)]
pub struct RoboticProximitySensor {
    pub radius: f32,
    /// Detected entities and their distances.
    #[serde(skip)]
    pub detections: Vec<(Entity, f32)>,
}

impl Default for RoboticProximitySensor {
    fn default() -> Self {
        Self {
            radius: 5.0,
            detections: Vec::new(),
        }
    }
}

/// Marker for a motorized joint in a robotic kinematic chain.
#[derive(Component, Serialize, Deserialize, Clone, Debug)]
pub struct RoboticJoint {
    pub joint_name: String,
    pub motor_index: usize,
}

pub struct RoboticBrainPlugin;

impl Plugin for RoboticBrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                update_proximity_sensors_system,
                propagate_sensory_input_system,
                apply_motor_commands_system,
            )
                .chain(),
        );
    }
}

/// Other physics bodies visible to a proximity sensor (excludes sensor entities themselves).
type ProximityOthersQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Transform, Entity),
    (Without<RoboticProximitySensor>, With<PhysicsBody>),
>;

/// Updates proximity sensors by querying the physics world or Bevy transforms.
fn update_proximity_sensors_system(
    mut sensors: Query<(&mut RoboticProximitySensor, &Transform, Entity)>,
    others: ProximityOthersQuery,
) {
    for (mut sensor, transform, sensor_entity) in &mut sensors {
        sensor.detections.clear();
        for (other_transform, other_entity) in &others {
            if sensor_entity == other_entity {
                continue;
            }
            let dist = transform.translation.distance(other_transform.translation);
            if dist <= sensor.radius {
                sensor.detections.push((other_entity, dist));
            }
        }
        // Sort by distance
        sensor
            .detections
            .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    }
}

/// Per-agent components consumed when propagating physical state into the brain's LTC input.
type SensoryInputQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut CognitiveBrain,
        &'static Transform,
        &'static PhysicsBody,
        Option<&'static RoboticHapticBinder>,
        Option<&'static RoboticProximitySensor>,
    ),
>;

/// Feeds physical state (pose, velocity, proximity) into the CognitiveBrain's LTC input.
/// Transitions from string-based to HDC-based perception using HapticSemanticBinder.
fn propagate_sensory_input_system(mut query: SensoryInputQuery, physics: Res<BevyPhysics<3>>) {
    for (mut brain, transform, body_comp, haptic_binder, proximity) in &mut query {
        if let Some(hb) = haptic_binder {
            // Base state: [pos(3), rot(4), vel(3), ang_vel(3)] (13 dims)
            let mut state = vec![0.0f32; 13];
            state[0] = transform.translation.x;
            state[1] = transform.translation.y;
            state[2] = transform.translation.z;
            state[3] = transform.rotation.x;
            state[4] = transform.rotation.y;
            state[5] = transform.rotation.z;
            state[6] = transform.rotation.w;

            if let Some(body) = physics.world.body(body_comp.handle) {
                state[7] = body.linear_velocity[0] as f32;
                state[8] = body.linear_velocity[1] as f32;
                state[9] = body.linear_velocity[2] as f32;
                // Bivector 3D has 3 components: xy, xz, yz
                state[10] = body.angular_velocity.get(0, 1) as f32;
                state[11] = body.angular_velocity.get(0, 2) as f32;
                state[12] = body.angular_velocity.get(1, 2) as f32;
            }

            // Optional: Append proximity data (first 3 closest objects: [dist, dx, dy, dz] * 3 = 12 dims)
            // Total dims would be 13 + 12 = 25.
            // For now, keeping it at 13 to match the binder initialization in spawn_robot.
            // Future: Dynamically resize binder or use a fixed large capacity.

            // Project into 16,384D semantic space
            let health = vec![1.0f32; state.len()];
            brain.perception_hv = Some(hb.binder.bind_with_health(&state, &health));
        } else {
            // Legacy path
            let prox_str = if let Some(p) = proximity {
                format!(", proximity: {}", p.detections.len())
            } else {
                String::new()
            };
            brain.perception_input = format!(
                "pos: {:.2}, {:.2}, {:.2}{}",
                transform.translation.x, transform.translation.y, transform.translation.z, prox_str
            );
        }
    }
}

/// Direct-drive proportional torque gain applied per unit of normalized
/// motor output ([-1.0, 1.0] LTC output -> Newton-meters). A full actuator
/// model (gearing, current limits, per-joint PD gains) is out of scope for
/// this generic ECS bridge — platform demos that need real PD position
/// control implement their own controller (e.g.
/// `symtropy-manipulator-demo`'s `admittance_control.rs`) and don't route
/// through this Bevy system at all.
const MOTOR_TORQUE_GAIN: f64 = 5.0;

/// Translates LTC output neurons into torque applied to kinematic joints.
///
/// `RoboticJoint` doesn't currently carry per-joint rotation-axis metadata,
/// so every joint is driven as a single-DOF hinge rotating in the physics
/// world's xy plane (bivector components `(0, 1)`). Multi-axis joints would
/// need an explicit axis/plane field added to `RoboticJoint`.
fn apply_motor_commands_system(
    brains: Query<(&CognitiveBrain, &RoboticAgentTag)>,
    mut joints: Query<(&RoboticJoint, &mut PhysicsBody, &ChildOf)>,
    mut physics: ResMut<BevyPhysics<3>>,
) {
    for (joint, body, parent) in &mut joints {
        if let Ok((brain, _agent)) = brains.get(parent.get()) {
            let output = &brain.motor_output;
            if output.len() > joint.motor_index {
                let target = output[joint.motor_index] as f64;
                if let Some(rb) = physics.world.body_mut(body.handle) {
                    let mut torque = Bivector::zero();
                    torque.set(0, 1, target * MOTOR_TORQUE_GAIN);
                    rb.apply_torque(torque);
                }
            }
        }
    }
}

pub fn spawn_robot(
    commands: &mut Commands,
    name: &str,
    pos: Vec3,
    brain: CognitiveBrain,
    body_handle: symtropy_physics::body::BodyHandle,
) -> Entity {
    // Create a binder for the standard 13-dim state
    let haptic_binder = RoboticHapticBinder {
        binder: HapticSemanticBinder::new(13, 16384),
    };

    commands
        .spawn((
            RoboticAgentTag {
                model_name: name.to_string(),
            },
            brain,
            haptic_binder,
            RoboticProximitySensor::default(),
            PhysicsBody {
                handle: body_handle,
                visual_radius: 0.5,
            },
            Transform::from_translation(pos),
            GlobalTransform::default(),
        ))
        .id()
}

// ---------------------------------------------------------------------
// Headless consciousness-driven RoboticAgent
// ---------------------------------------------------------------------
//
// This is a *different* concept from `RoboticAgentTag` above: it's not a
// Bevy component at all, but the concrete type the platform demos
// (`crates/apps/symtropy-*-demo`) and this crate's `examples/*.rs` Φ
// benchmarks construct directly via `RoboticAgent::new(handle, platform,
// name)` and drive headlessly via `.tick(observation, danger_level)` /
// `.phi()`. It implements the trait of the same name from
// `symtropy-robotics-bridge-core` (re-exported from this crate's root as
// `RoboticAgentTrait` to avoid colliding with this concrete struct) so
// `tick_motor_commands()` / `MotorPlanner` dispatch works generically over
// it. Callers must `use symtropy_robotics_bridge::RoboticAgentTrait;` to
// bring `.tick()` / `.phi()` / `.bottleneck()` / `.can_act()` into scope —
// `RoboticAgent::new()` and the public `caution` / `safety_tier` fields
// don't need it.

use std::collections::{HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use symthaea_consciousness_equation::ConsciousnessInputs;
use symtropy_consciousness_physics::{EntityConsciousness, SafetyTier};
use symtropy_physics::body::BodyHandle;
use symtropy_robotics_bridge_core::RoboticAgent as CoreRoboticAgent;
use symtropy_robotics_bridge_core::platform::PlatformType;

/// EMA smoothing factor for the `caution` accumulator — how quickly caution
/// tracks the current danger level (~6-7 ticks to reach 63% of a step
/// change).
const CAUTION_EMA_ALPHA: f64 = 0.15;

/// Capacity of the short observation ring buffer backing the working-memory,
/// recurrence, and Φ-input derivations. 8 ≈ the classic 7±2 working-memory
/// span; it is also the temporal window over which channel statistics for
/// the Φ proxy are computed.
const OBS_HISTORY_CAP: usize = 8;

/// Half-saturation constant for the knowledge signal: after this many
/// distinct (quantized) states have been visited, `knowledge` = 0.5.
const KNOWLEDGE_HALF_SATURATION: f64 = 16.0;

/// Hard cap on the lifetime distinct-state set so long-running demos can't
/// grow memory without bound (knowledge is already > 0.99 by this point).
const SEEN_STATES_CAP: usize = 4096;

/// Quantization resolution used when hashing observations into "distinct
/// states": each channel is rounded to multiples of 1/STATE_QUANTIZATION
/// before hashing, so tiny sensor jitter doesn't count as a new state.
const STATE_QUANTIZATION: f64 = 8.0;

/// Half-saturation for the differentiation half of the Φ input: a mean
/// per-channel standard deviation of 0.1 across the observation window maps
/// to a differentiation score of 0.5.
const DIFFERENTIATION_HALF_SATURATION: f64 = 0.1;

/// Default energy budget for the internal [`EntityConsciousness`]. Large
/// enough that these headless demo/benchmark agents never hit the
/// low-energy `SafetyTier::Red` collapse path from budget exhaustion —
/// only from Φ itself, which is what the demos/benchmarks are measuring.
const DEFAULT_ENERGY_BUDGET: f64 = 1_000.0;

/// A consciousness-driven robotic agent: wraps the real
/// `MasterConsciousnessEquation` pipeline (via
/// `symtropy_consciousness_physics::EntityConsciousness`) behind the simple
/// `tick(observation, danger) -> motor_gain` contract the platform demos and
/// Φ-gated-safety benchmark examples expect. The equation's inputs are
/// *measured from the agent's own observation stream* (ring buffer +
/// lifetime state statistics — see the `*_signal` helpers and the `tick`
/// docs on the trait impl below), not hardcoded constants.
///
/// Construct with [`RoboticAgent::new`]; drive with `.tick()` / read `.phi()`
/// via the [`symtropy_robotics_bridge_core::RoboticAgent`] trait (import it
/// from this crate's root as `RoboticAgentTrait`).
pub struct RoboticAgent {
    body: BodyHandle,
    platform: PlatformType,
    pub name: String,
    consciousness: EntityConsciousness,
    /// EMA of recent danger levels. Rises under sustained danger, decays
    /// back down once danger subsides — a softer behavioral signal
    /// independent of the harder Φ-gated `safety_tier`.
    pub caution: f64,
    /// Current Φ-gated safety tier (mirrors `self.consciousness.safety_tier`
    /// after each tick; exposed directly so callers don't need
    /// `symtropy-consciousness-physics` in scope just to read it).
    pub safety_tier: SafetyTier,
    /// Ring buffer of the last [`OBS_HISTORY_CAP`] observations (each paired
    /// with its quantized state hash). Backs the working-memory, recurrence,
    /// and Φ-input derivations in [`Self::tick`].
    obs_history: VecDeque<(Vec<f64>, u64)>,
    /// Lifetime set of distinct quantized states visited (capped at
    /// [`SEEN_STATES_CAP`]). Backs the knowledge derivation.
    seen_states: HashSet<u64>,
}

impl RoboticAgent {
    /// Create a new agent for `platform`, tracking physics body `body`, with
    /// a display `name` (logging/telemetry only, no uniqueness requirement).
    pub fn new(body: BodyHandle, platform: PlatformType, name: impl Into<String>) -> Self {
        Self {
            body,
            platform,
            name: name.into(),
            consciousness: EntityConsciousness::new(DEFAULT_ENERGY_BUDGET),
            caution: 0.0,
            safety_tier: SafetyTier::Green,
            obs_history: VecDeque::with_capacity(OBS_HISTORY_CAP),
            seen_states: HashSet::new(),
        }
    }

    /// Hash an observation into a quantized "state" key: each channel is
    /// rounded to the [`STATE_QUANTIZATION`] grid so tiny sensor jitter maps
    /// to the same state while genuinely different configurations don't.
    fn quantized_state(observation: &[f64]) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for v in observation {
            ((v * STATE_QUANTIZATION).round() as i64).hash(&mut h);
        }
        h.finish()
    }

    /// Record the current observation into the ring buffer and the lifetime
    /// distinct-state set.
    fn push_observation(&mut self, observation: &[f64]) {
        let key = Self::quantized_state(observation);
        if self.obs_history.len() == OBS_HISTORY_CAP {
            self.obs_history.pop_front();
        }
        self.obs_history.push_back((observation.to_vec(), key));
        if self.seen_states.len() < SEEN_STATES_CAP {
            self.seen_states.insert(key);
        }
    }

    /// R (recurrence) input: mean similarity (`1 / (1 + RMS distance)`)
    /// between the current observation and the recent observations still held
    /// in the buffer. In the closed perception→action→physics loop each
    /// observation partially reflects the agent's own previous motor output,
    /// so persistent current-vs-recent similarity is a measured (if coarse)
    /// signature of recurrent sensorimotor feedback — not a placebo: it is
    /// genuinely 0.0 on the first tick when no loop has closed yet, and it
    /// moves with what the body actually reports.
    fn recurrence_signal(&self, observation: &[f64]) -> f64 {
        if self.obs_history.is_empty() || observation.is_empty() {
            return 0.0;
        }
        let mut total = 0.0;
        let mut count = 0usize;
        for (past, _) in &self.obs_history {
            let n = past.len().min(observation.len());
            if n == 0 {
                continue;
            }
            let mse = past
                .iter()
                .zip(observation)
                .take(n)
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                / n as f64;
            total += 1.0 / (1.0 + mse.sqrt());
            count += 1;
        }
        if count == 0 {
            0.0
        } else {
            (total / count as f64).clamp(0.0, 1.0)
        }
    }

    /// W (working memory) input: the number of *distinct* quantized states
    /// currently held in the ring buffer, relative to its 7±2-style capacity.
    /// This measures actual working-memory load rather than asserting one: an
    /// agent fed a single unchanging observation genuinely holds one item
    /// (low W), while a varied stream fills the span with distinct items
    /// (W → 1.0).
    fn working_memory_signal(&self) -> f64 {
        let distinct: HashSet<u64> = self.obs_history.iter().map(|(_, k)| *k).collect();
        (distinct.len() as f64 / OBS_HISTORY_CAP as f64).clamp(0.0, 1.0)
    }

    /// K (knowledge) input: saturating function `n / (n + k)` of the number
    /// of distinct quantized states this agent has visited over its lifetime
    /// — an accumulated-experience proxy that starts near 0 for a newborn
    /// agent and grows only through actual exploration of new states, rather
    /// than being pinned to a constant.
    fn knowledge_signal(&self) -> f64 {
        let n = self.seen_states.len() as f64;
        (n / (n + KNOWLEDGE_HALF_SATURATION)).clamp(0.0, 1.0)
    }

    /// Φ input channel: an integration-×-differentiation proxy measured from
    /// the structure of the embodiment channels (`observation[2..]`, matching
    /// the packing convention documented on [`Self::tick`]; falls back to the
    /// whole vector for short observations) over the recent window:
    ///
    /// - **differentiation** — mean per-channel standard deviation over time,
    ///   saturated via [`DIFFERENTIATION_HALF_SATURATION`]: are the body's
    ///   sensory channels carrying information at all, or frozen?
    /// - **integration** — mean |Pearson correlation| across channel pairs
    ///   over the same window (lag-1 autocorrelation when only one channel is
    ///   available): do the channels move together as one coupled system,
    ///   rather than independently?
    ///
    /// Their geometric mean is 0 whenever either is absent — the IIT
    /// intuition that Φ requires both a differentiated repertoire and
    /// integration across parts. This is an explicit *proxy*, not real IIT Φ,
    /// but every term is computed from the agent's own sensory stream; the
    /// external `danger_level` no longer enters this channel at all.
    ///
    /// Public (2026-07-10) so demos can regression-test that their
    /// observation packing gives this channel live structure — the
    /// manipulator demo shipped for weeks with two constant channels that
    /// zeroed it every tick.
    pub fn integration_signal(&self) -> f64 {
        if self.obs_history.len() < 2 {
            return 0.0;
        }
        let ticks = self.obs_history.len();
        let min_len = self
            .obs_history
            .iter()
            .map(|(o, _)| o.len())
            .min()
            .unwrap_or(0);
        if min_len == 0 {
            return 0.0;
        }
        // Embodiment channels per the tick() packing convention; whole vector
        // when the observation is too short to have dedicated ones.
        let (start, end) = if min_len > 2 {
            (2, min_len)
        } else {
            (0, min_len)
        };
        // Per-channel time series over the window, plus (mean, variance).
        let series: Vec<Vec<f64>> = (start..end)
            .map(|c| self.obs_history.iter().map(|(o, _)| o[c]).collect())
            .collect();
        let stats: Vec<(f64, f64)> = series
            .iter()
            .map(|s| {
                let mean = s.iter().sum::<f64>() / ticks as f64;
                let var = s.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / ticks as f64;
                (mean, var)
            })
            .collect();
        let mean_std = stats.iter().map(|(_, v)| v.sqrt()).sum::<f64>() / stats.len() as f64;
        let differentiation = mean_std / (mean_std + DIFFERENTIATION_HALF_SATURATION);

        let mut r_total = 0.0;
        let mut pairs = 0usize;
        for i in 0..series.len() {
            for j in (i + 1)..series.len() {
                let (vi, vj) = (stats[i].1, stats[j].1);
                if vi <= f64::EPSILON || vj <= f64::EPSILON {
                    continue;
                }
                let cov = series[i]
                    .iter()
                    .zip(&series[j])
                    .map(|(a, b)| (a - stats[i].0) * (b - stats[j].0))
                    .sum::<f64>()
                    / ticks as f64;
                r_total += (cov / (vi.sqrt() * vj.sqrt())).abs().min(1.0);
                pairs += 1;
            }
        }
        let integration = if pairs > 0 {
            r_total / pairs as f64
        } else if series.len() == 1 && stats[0].1 > f64::EPSILON {
            // Single varying channel: lag-1 autocorrelation as the coupling
            // proxy (temporal integration instead of cross-channel coupling).
            let s = &series[0];
            let mean = stats[0].0;
            let cov = s
                .windows(2)
                .map(|w| (w[0] - mean) * (w[1] - mean))
                .sum::<f64>()
                / (ticks - 1) as f64;
            (cov / stats[0].1).abs().min(1.0)
        } else {
            0.0
        };

        (differentiation * integration).sqrt().clamp(0.0, 1.0)
    }
}

impl CoreRoboticAgent for RoboticAgent {
    fn body(&self) -> BodyHandle {
        self.body
    }

    fn platform(&self) -> PlatformType {
        self.platform
    }

    /// Run one perception-action tick through the real
    /// `MasterConsciousnessEquation` (via `EntityConsciousness::compute`) and
    /// return the resulting motor gain in `[0, 1]`.
    ///
    /// `observation` is platform-specific (see each demo's
    /// `consciousness_bridge.rs` for its packing convention): `observation[0]`
    /// is treated as a prediction-error-like signal and `observation[2..]` as
    /// embodiment-grounding channels. Every equation input is *derived from
    /// the agent's own running state* (see the `*_signal` helpers above): the
    /// Φ channel from the integration × differentiation structure of the
    /// embodiment channels over a recent window, W from the distinct items
    /// currently held in the observation buffer, R from current-vs-recent
    /// observation similarity, K from the lifetime distinct-state count.
    /// Nothing is hardcoded, and `danger_level` is never used as a stand-in
    /// for Φ (it drives attention narrowing and the `caution` EMA only).
    /// Expect a genuine warm-up transient over roughly the first
    /// [`OBS_HISTORY_CAP`] ticks while the buffers fill — a freshly spawned
    /// agent honestly reports low integration/knowledge.
    fn tick(&mut self, observation: &[f64], danger_level: f64) -> f64 {
        let danger = danger_level.clamp(0.0, 1.0);
        let prediction_error = observation.first().copied().unwrap_or(0.0).clamp(0.0, 2.0);
        let embodiment = if observation.len() > 2 {
            (observation[2..].iter().sum::<f64>() / (observation.len() - 2) as f64).clamp(0.0, 1.0)
        } else {
            0.5
        };

        // Recurrence compares the current observation against history from
        // *before* this tick is recorded (feedback is current-vs-past); the
        // remaining signals include the current observation in their window.
        let recurrence = self.recurrence_signal(observation);
        self.push_observation(observation);
        let working_memory = self.working_memory_signal();
        let knowledge = self.knowledge_signal();
        let integration = self.integration_signal();

        let inputs = ConsciousnessInputs {
            // Structural integration proxy measured from the embodiment
            // channels' own dynamics — NOT `1 - danger`. Danger influences
            // behavior via `attention` and the caution EMA, never by
            // impersonating Φ.
            phi: integration,
            broadcast: (1.0 - prediction_error / 2.0).clamp(0.0, 1.0),
            working_memory,
            attention: (1.0 - danger).clamp(0.0, 1.0),
            recurrence,
            embodiment,
            knowledge,
            synchrony: (1.0 - prediction_error / 2.0).clamp(0.0, 1.0),
        };

        self.consciousness.compute(&inputs);
        self.safety_tier = self.consciousness.safety_tier;

        // Sustained danger raises caution; sustained safety lets it settle.
        self.caution += (danger - self.caution) * CAUTION_EMA_ALPHA;

        self.consciousness.effective_motor_gain().clamp(0.0, 1.0)
    }

    fn phi(&self) -> f64 {
        self.consciousness.phi()
    }

    fn bottleneck(&self) -> &str {
        self.consciousness.bottleneck()
    }

    fn can_act(&self) -> bool {
        self.safety_tier.allows_output()
    }
}

#[cfg(test)]
mod consciousness_agent_tests {
    use super::*;

    #[test]
    fn tick_produces_valid_phi_and_gain() {
        let mut agent = RoboticAgent::new(BodyHandle(0), PlatformType::Manipulator, "test");
        let gain = agent.tick(&[0.01, 0.0, 0.5, 0.5], 0.0);
        assert!((0.0..=1.0).contains(&gain));
        assert!((0.0..=1.0).contains(&agent.phi()));
    }

    #[test]
    fn sustained_danger_raises_caution() {
        let mut agent = RoboticAgent::new(BodyHandle(0), PlatformType::Manipulator, "test");
        let initial = agent.caution;
        for _ in 0..20 {
            agent.tick(&[0.8, 0.95], 0.95);
        }
        assert!(agent.caution > initial);
    }

    #[test]
    fn low_danger_decreases_caution_after_high_danger() {
        let mut agent = RoboticAgent::new(BodyHandle(0), PlatformType::Manipulator, "test");
        for _ in 0..10 {
            agent.tick(&[0.8, 0.95], 0.95);
        }
        let high = agent.caution;
        for _ in 0..50 {
            agent.tick(&[0.01, 0.0], 0.0);
        }
        assert!(agent.caution < high);
    }

    /// A varied observation stream (rich, coupled embodiment dynamics) must
    /// yield strictly higher Φ than a constant stream. The old fabricated
    /// inputs (`phi = 1 - danger`, hardcoded W/R/K) could not distinguish
    /// these two agents at all.
    #[test]
    fn varied_observations_yield_higher_phi_than_constant() {
        let mut varied = RoboticAgent::new(BodyHandle(0), PlatformType::Manipulator, "varied");
        let mut constant = RoboticAgent::new(BodyHandle(1), PlatformType::Manipulator, "constant");
        for i in 0..40 {
            let t = i as f64 * 0.37;
            varied.tick(
                &[
                    0.05,
                    0.0,
                    0.5 + 0.4 * t.sin(),
                    0.5 + 0.4 * (1.7 * t).cos(),
                    0.5 + 0.3 * (0.9 * t + 1.0).sin(),
                ],
                0.0,
            );
            constant.tick(&[0.05, 0.0, 0.5, 0.5, 0.5], 0.0);
        }
        assert!(
            varied.phi() > constant.phi(),
            "varied phi {} should exceed constant phi {}",
            varied.phi(),
            constant.phi()
        );
    }

    /// Regression guard against reintroducing hardcoded inputs: the derived
    /// working_memory / recurrence / knowledge factors must move over time
    /// and must not sit on the old constants (0.7 / 0.6 / 0.5).
    #[test]
    fn derived_inputs_change_over_time() {
        let factors = |agent: &RoboticAgent| {
            agent
                .consciousness
                .result
                .as_ref()
                .expect("tick() computed a result")
                .factors
                .clone()
        };

        let mut agent = RoboticAgent::new(BodyHandle(0), PlatformType::Manipulator, "test");
        agent.tick(&[0.05, 0.0, 0.1, 0.9, 0.3], 0.0);
        let early = factors(&agent);
        for i in 1..50 {
            let t = i as f64 * 0.61;
            agent.tick(
                &[
                    0.05,
                    0.0,
                    0.5 + 0.5 * t.sin(),
                    0.5 + 0.5 * (1.3 * t).cos(),
                    (0.7 * t).sin().abs(),
                ],
                0.0,
            );
        }
        let late = factors(&agent);

        // First tick: single item in the buffer, no closed feedback loop yet,
        // almost no accumulated experience.
        assert!(early.recurrence == 0.0, "no feedback before tick 2");
        assert!(late.working_memory > early.working_memory);
        assert!(late.knowledge > early.knowledge);
        assert!(late.recurrence > early.recurrence);
        assert!(
            late.phi > early.phi,
            "integration grows as the window fills"
        );
        for f in [&early, &late] {
            assert_ne!(f.working_memory, 0.7, "W must not be the old constant");
            assert_ne!(f.recurrence, 0.6, "R must not be the old constant");
            assert_ne!(f.knowledge, 0.5, "K must not be the old constant");
        }
    }

    /// Under maximum danger the old pipeline forced the Φ input to 0.0
    /// (`1 - danger`). The honest pipeline keeps measuring structural
    /// integration from the sensory stream regardless of danger; danger
    /// still reaches the equation through attention narrowing.
    #[test]
    fn phi_input_is_structural_not_danger() {
        let mut agent = RoboticAgent::new(BodyHandle(0), PlatformType::Manipulator, "test");
        for i in 0..30 {
            let t = i as f64 * 0.43;
            agent.tick(
                &[
                    0.05,
                    0.0,
                    0.5 + 0.4 * t.sin(),
                    0.5 + 0.4 * (1.9 * t).cos(),
                    0.5 + 0.3 * (0.8 * t + 0.5).sin(),
                ],
                1.0, // maximum danger, every tick
            );
        }
        let factors = agent
            .consciousness
            .result
            .as_ref()
            .expect("tick() computed a result")
            .factors
            .clone();
        assert!(
            factors.phi > 0.0,
            "Φ input must stay structural under danger (old code forced it to 0.0), got {}",
            factors.phi
        );
        assert!(
            factors.attention == 0.0,
            "danger still drives attention narrowing, got {}",
            factors.attention
        );
    }
}
