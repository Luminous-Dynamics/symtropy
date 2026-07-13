// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Full consciousness pipeline wiring (Option B — NO shortcuts).
//!
//! Uses `symtropy-robotics-bridge`'s `RoboticAgent.tick()` which runs:
//! 1. FEP perceive (observation + precision)
//! 2. FEP select_action()
//! 3. `MasterConsciousnessEquation` computation
//! 4. Safety tier derived from real Phi
//! 5. Motor gain returned
//!
//! The prediction error flows through the actual consciousness pipeline.
//! Phi is never a clamped lerp of PE. Industrial partners can inspect this.

use symtropy_consciousness_physics::safety::SafetyTier;
use symtropy_robotics_bridge::RoboticAgentTrait;
use symtropy_robotics_bridge::agent::RoboticAgent;

/// Run one consciousness tick through the full pipeline.
///
/// Feeds the prediction error and danger level into the RoboticAgent,
/// which computes real Phi via MasterConsciousnessEquation.
///
/// `control_effort` and `stiffness` must be live measurements (normalized
/// to ~[0,1]): the agent's `integration_signal()` computes per-channel
/// differentiation over `observation[2..]`, so constants here zero the
/// Φ input channel every tick — exactly the placeholder bug this
/// signature change removed (2026-07-10).
///
/// Returns (phi, safety_tier, motor_gain).
pub fn consciousness_tick(
    agent: &mut RoboticAgent,
    prediction_error: f32,
    danger_level: f64,
    control_effort: f64,
    stiffness: f64,
) -> (f64, SafetyTier, f64) {
    // Build observation vector from manipulator state.
    // The RoboticAgent expects a slice of f64 observations.
    // We pack: [prediction_error, danger_level, control_effort, stiffness]
    let observation = [
        prediction_error as f64,
        danger_level,
        control_effort,
        stiffness,
    ];

    // Run the full consciousness pipeline:
    // FEP perceive → action select → MasterConsciousnessEquation → SafetyTier
    let motor_gain = agent.tick(&observation, danger_level);

    let phi = agent.phi();
    let tier = agent.safety_tier;

    (phi, tier, motor_gain)
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_physics::body::BodyHandle;
    use symtropy_robotics_bridge::platform::PlatformType;

    #[test]
    fn consciousness_tick_produces_valid_output() {
        let mut agent = RoboticAgent::new(BodyHandle(0), PlatformType::Manipulator, "test-panda");

        let (phi, _tier, gain) = consciousness_tick(&mut agent, 0.01, 0.0, 0.2, 0.8);
        assert!(phi >= 0.0 && phi <= 1.0, "Phi should be in [0,1]: {phi}");
        assert!(
            gain >= 0.0 && gain <= 1.0,
            "Gain should be in [0,1]: {gain}"
        );
    }

    #[test]
    fn high_danger_increases_caution() {
        let mut agent = RoboticAgent::new(BodyHandle(0), PlatformType::Manipulator, "test-panda");

        let initial_caution = agent.caution;

        // Run with high danger — caution should increase
        for i in 0..20 {
            let effort = 0.3 + 0.02 * (i as f64);
            consciousness_tick(&mut agent, 0.8, 0.95, effort, 0.9 - 0.01 * (i as f64));
        }

        assert!(
            agent.caution > initial_caution,
            "High danger should increase caution: initial={initial_caution}, now={}",
            agent.caution
        );
    }

    #[test]
    fn low_danger_decreases_caution() {
        let mut agent = RoboticAgent::new(BodyHandle(0), PlatformType::Manipulator, "test-panda");

        // First increase caution
        for i in 0..10 {
            consciousness_tick(&mut agent, 0.8, 0.95, 0.5 + 0.02 * (i as f64), 0.5);
        }
        let high_caution = agent.caution;

        // Then decrease with low danger
        for i in 0..50 {
            consciousness_tick(&mut agent, 0.01, 0.0, 0.1 + 0.005 * (i as f64), 0.9);
        }

        assert!(
            agent.caution < high_caution,
            "Low danger should decrease caution: high={high_caution}, now={}",
            agent.caution
        );
    }

    #[test]
    fn varying_channels_produce_nonzero_phi_input() {
        // Regression: channels 3/4 used to be hardcoded 0.5s, which made
        // integration_signal()'s per-channel differentiation over
        // observation[2..] exactly 0 → Φ input ≡ 0 every tick. With live
        // varying channels the Φ input must become nonzero once the
        // observation window fills.
        let mut agent = RoboticAgent::new(BodyHandle(0), PlatformType::Manipulator, "test-panda");
        for i in 0..32 {
            let t = i as f64 * 0.3;
            consciousness_tick(
                &mut agent,
                0.05 + 0.02 * t.sin() as f32,
                0.1,
                0.4 + 0.3 * t.sin(),
                0.5 + 0.3 * t.cos(),
            );
        }
        assert!(
            agent.integration_signal() > 0.0,
            "Φ input channel is still structurally zero — placeholder regression"
        );
    }
}
