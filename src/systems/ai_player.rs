// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! AI Player: Symthaea plays the game.
//!
//! An FEP (Free Energy Principle) agent observes the game and computes an
//! expected-free-energy action-selection signal. Current locomotion is still
//! produced by the continuous free-energy gradient controller; the discrete
//! action selection is retained as advisory telemetry until a controlled
//! closed-loop actuation experiment is validated.
//!
//! Enable with: --ai-player flag

use bevy::prelude::*;
use symthaea_fep::{ActiveInferenceAgent, ActiveInferenceAgentConfig, Observation};

use crate::components::{CrewNpc, FusionCore, NoiseEmitter, Player};
use crate::resources::{
    DungeonSeed, EnergyWell, LeviathanState, PhysicsWorldRes, PlayerInput, SleepPhase,
};
use symtropy_render_bridge::PhysicsBody;

/// AI player state — the consciousness-inspired controller that plays the game.
#[derive(Resource)]
pub struct AiPlayer {
    pub agent: ActiveInferenceAgent,
    pub enabled: bool,
    pub tick: u64,
    pub decisions: u64,
    /// Most recent expected-free-energy action selected by the FEP agent.
    /// Advisory only until a validated action→locomotion mapping is introduced.
    pub last_advisory_action: Option<usize>,
    /// Expected free energy associated with `last_advisory_action`.
    pub last_advisory_efe: Option<f64>,
    /// Dungeon seed used to initialize the stochastic action-selection stream.
    pub action_rng_seed: Option<u64>,
}

impl Default for AiPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl AiPlayer {
    pub fn new() -> Self {
        let config = ActiveInferenceAgentConfig {
            state_dim: 8,
            obs_dim: 8,
            num_actions: 5, // advisory integration vocabulary; not yet directly actuated
            // TD learning requires `last_action` to identify the action that
            // actually caused the next transition. Symtropy's current discrete
            // action is advisory while locomotion comes from the continuous
            // free-energy gradient, so enabling TD here would suppress direct
            // model learning without producing valid TD updates. Keep direct
            // generative-model learning active until #20 closes the real loop.
            enable_model_learning: true,
            enable_td_learning: false,
            ..Default::default()
        };
        Self {
            agent: ActiveInferenceAgent::new(config),
            enabled: false,
            tick: 0,
            decisions: 0,
            last_advisory_action: None,
            last_advisory_efe: None,
            action_rng_seed: None,
        }
    }
}

/// AI player system: observes game state and writes continuous locomotion intent.
///
/// When enabled, the human input system explicitly relinquishes `PlayerInput`
/// ownership, so this system is the sole controller writer for the frame.
///
/// The discrete action returned by `ActiveInferenceAgent::select_action()` is
/// deliberately not passed to `act()` yet. Doing that without a defined and
/// validated action→game-control mapping would teach the TD learner that an
/// abstract action caused whatever the independent gradient controller happened
/// to do. Instead, the selected action and EFE are exposed as advisory telemetry
/// for the future controlled integration experiment.
pub fn ai_player_system(
    mut ai: ResMut<AiPlayer>,
    mut input: ResMut<PlayerInput>,
    physics: Res<PhysicsWorldRes>,
    dungeon_seed: Res<DungeonSeed>,
    leviathan: Res<LeviathanState>,
    player_query: Query<(&Transform, &PhysicsBody), With<Player>>,
    mut noise_query: Query<&mut NoiseEmitter, With<Player>>,
    npc_query: Query<&Transform, (With<CrewNpc>, Without<Player>)>,
    well_query: Query<(&Transform, &EnergyWell), (Without<Player>, Without<CrewNpc>)>,
    core_query: Query<&Transform, (With<FusionCore>, Without<Player>)>,
) {
    if !ai.enabled {
        return;
    }

    // Make stochastic EFE action selection reproducible for a given dungeon
    // seed. This does not alter the continuous gradient controller; it makes
    // advisory action telemetry suitable for paired experiments/replays.
    if ai.action_rng_seed.is_none() {
        let seed = dungeon_seed.0 ^ 0xA17E_1F3E_9C6D_42B5;
        ai.agent.set_rng_seed(seed);
        ai.action_rng_seed = Some(seed);
    }

    ai.tick += 1;

    let Ok((player_tf, player_body)) = player_query.single() else {
        return;
    };
    let player_pos = player_tf.translation.truncate();

    // Observation schema v1:
    // [energy, phi, danger, npc_dir_x, npc_dir_y, well_dir_x, well_dir_y,
    //  npc_proximity]. Keep this ordering stable for reproducible experiments.

    // 1. Energy fraction [0, 1]
    let energy_frac = physics
        .consciousness
        .entities
        .get(&player_body.handle)
        .map(|e| e.energy.fraction_remaining())
        .unwrap_or(1.0);

    // 2. Phi level [0, 1]
    let phi = physics.consciousness.phi(player_body.handle);

    // 3. Danger level [0, 1]
    let danger = match leviathan.phase {
        SleepPhase::Dormant => 0.0,
        SleepPhase::Stirring => 0.3,
        SleepPhase::Awake => 0.7,
        SleepPhase::Hunting => 1.0,
    };

    // 4-5. Direction to nearest NPC (normalized, -1 to 1 for x and y)
    let nearest_npc = npc_query
        .iter()
        .map(|tf| {
            (
                tf.translation.truncate(),
                tf.translation.truncate().distance(player_pos),
            )
        })
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let (npc_dir_x, npc_dir_y, npc_dist_norm) = match nearest_npc {
        Some((npc_pos, dist)) if dist > 1.0 => {
            let dir = (npc_pos - player_pos).normalize();
            (
                dir.x as f64,
                dir.y as f64,
                (1.0 - dist as f64 / 500.0).max(0.0),
            )
        }
        _ => (0.0, 0.0, 1.0), // Already at NPC
    };

    // 6-7. Direction to nearest energy well
    let nearest_well = well_query
        .iter()
        .filter(|(_, w)| w.is_active())
        .map(|(tf, _)| {
            (
                tf.translation.truncate(),
                tf.translation.truncate().distance(player_pos),
            )
        })
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let (well_dir_x, well_dir_y) = match nearest_well {
        Some((well_pos, dist)) if dist > 1.0 => {
            let dir = (well_pos - player_pos).normalize();
            (dir.x as f64, dir.y as f64)
        }
        _ => (0.0, 0.0),
    };

    // Fusion-core direction is currently used by the continuous gradient path
    // below, not by observation schema v1.
    let _core_dir = core_query
        .iter()
        .next()
        .map(|tf| {
            let dir = (tf.translation.truncate() - player_pos).normalize_or_zero();
            (dir.x as f64, dir.y as f64)
        })
        .unwrap_or((0.0, 0.0));

    // === FEP PERCEPTION + ADVISORY ACTION SELECTION ===

    let obs = Observation::new(
        vec![
            energy_frac,
            phi,
            danger,
            npc_dir_x,
            npc_dir_y,
            well_dir_x,
            well_dir_y,
            npc_dist_norm,
        ],
        0.8, // precision
        "game",
    );

    let _perception = ai.agent.perceive(&obs);
    let action_selection = ai.agent.select_action();
    ai.last_advisory_action = Some(action_selection.action);
    ai.last_advisory_efe = Some(action_selection.expected_free_energy);
    ai.decisions += 1;

    // === FREE ENERGY GRADIENT — CURRENT AUTHORITATIVE AI LOCOMOTION POLICY ===

    let pos = nalgebra::SVector::from([player_pos.x as f64, player_pos.y as f64]);

    // Get player's harmony activations
    let harmony = physics
        .consciousness
        .entities
        .get(&player_body.handle)
        .map(|e| e.harmony_activations)
        .unwrap_or([0.5; 9]);

    // Build nearby agents list from NPCs
    let nearby: Vec<_> = npc_query
        .iter()
        .map(|tf| {
            let npc_pos =
                nalgebra::SVector::from([tf.translation.x as f64, tf.translation.y as f64]);
            // Harmony data is not yet plumbed through from the NPC's own
            // consciousness state; every entry uses the neutral 0.5 default.
            // Was a `filter_map` returning unconditional `Some`, i.e. a `map`
            // with extra ceremony — no element was ever filtered out.
            (npc_pos, [0.5f64; 9])
        })
        .collect();

    // Build energy well list
    let wells_data: Vec<_> = well_query
        .iter()
        .filter(|(_, w)| w.is_active())
        .map(|(tf, w)| {
            (
                nalgebra::SVector::from([tf.translation.x as f64, tf.translation.y as f64]),
                w.fraction_remaining(),
            )
        })
        .collect();

    // Danger source (core position, where Leviathan spawns)
    let danger_src: Option<nalgebra::SVector<f64, 2>> = if danger > 0.1 {
        core_query
            .iter()
            .next()
            .map(|tf| nalgebra::SVector::from([tf.translation.x as f64, tf.translation.y as f64]))
    } else {
        None
    };

    let gradient = symtropy_consciousness_physics::fep_gradient::free_energy_gradient(
        &pos,
        energy_frac,
        &harmony,
        &nearby,
        &wells_data,
        danger_src.as_ref(),
        danger,
    );

    let dir = Vec2::new(gradient[0] as f32, gradient[1] as f32);
    let free_energy = ai.agent.current_free_energy();
    let sprinting = energy_frac > 0.5 && danger > 0.5;

    input.direction = dir;
    input.sprinting = sprinting;

    // AI locomotion is subject to the same acoustic gameplay contract as
    // human locomotion. Without this, autonomous movement would be physically
    // real but mechanically silent to the Leviathan.
    if let Ok(mut noise) = noise_query.single_mut() {
        super::player::update_player_noise(&mut noise, dir, sprinting);
    }

    // === SYMTHAEA'S VOICE: raw FEP telemetry, not chatbot ===
    // This is not dialogue. This is a consciousness reporting its thermodynamic state.
    if ai.tick.is_multiple_of(192) {
        let state = if energy_frac <= 0.0 {
            "COLLAPSED. Motor authority zero. Awaiting epistemic offload from proximal agent."
        } else if energy_frac < 0.15 {
            "CRITICAL. Energy below Landauer maintenance floor. Seeking low-entropy zone."
        } else if free_energy > 5.0 {
            "HIGH SURPRISE. Prediction model divergent. Active inference recalibrating."
        } else if danger > 0.5 {
            "THREAT DETECTED. Environmental entropy spike. Minimizing noise signature."
        } else if npc_dist_norm > 0.8 {
            "PROXIMITY. Adjacent agent. Epistemic offloading reduces prediction cost."
        } else if energy_frac > 0.7 {
            "NOMINAL. Energy sufficient. Prediction model converging."
        } else {
            "OPERATING. Sensory processing within maintenance budget."
        };

        eprintln!(
            "[Symthaea] {} | E={:.0}% Phi={:.4} FE={:.2} action={:?} action_EFE={:?} danger={:.1} tick={}",
            state,
            energy_frac * 100.0,
            phi,
            free_energy,
            ai.last_advisory_action,
            ai.last_advisory_efe,
            danger,
            ai.tick,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advisory_mode_keeps_direct_learning_enabled_and_td_disabled() {
        let ai = AiPlayer::new();

        assert!(ai.agent.config.enable_model_learning);
        assert!(!ai.agent.config.enable_td_learning);
        assert!(ai.agent.td_stats().is_none());
        assert!(ai.agent.last_action.is_none());
        assert!(ai.last_advisory_action.is_none());
        assert!(ai.last_advisory_efe.is_none());
        assert!(ai.action_rng_seed.is_none());
    }
}
