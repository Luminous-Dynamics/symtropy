// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! NPC consciousness via Symthaea's Master Consciousness Equation V2.
//!
//! This is the 7-theory synthesis: IIT (Φ), GWT (broadcast), HOT (recurrence),
//! attention, embodiment, knowledge, and synchrony — all computed in real-time
//! for each NPC. The equation identifies the BOTTLENECK: what's limiting
//! this agent's consciousness right now?
//!
//! The player can see and act on these bottlenecks.

use bevy::prelude::*;
#[cfg(feature = "consciousness-runtime")]
use symthaea_consciousness_equation::{ConsciousnessInputs, MasterConsciousnessEquation};

use crate::components::CrewNpc;
use crate::resources::{BiometricsCtx, LeviathanState, SleepPhase};

/// NPC consciousness state computed by the Master Equation.
#[derive(Component)]
pub struct NpcConsciousness {
    /// The equation engine (carries temporal history).
    #[cfg(feature = "consciousness-runtime")]
    pub equation: MasterConsciousnessEquation,
    /// Current consciousness level [0, 1].
    pub level: f64,
    /// What's limiting this NPC's consciousness right now.
    pub bottleneck: String,
    /// Temporal stability [0, 1] — how consistent their consciousness has been.
    pub stability: f64,
}

impl Default for NpcConsciousness {
    fn default() -> Self {
        Self {
            #[cfg(feature = "consciousness-runtime")]
            equation: MasterConsciousnessEquation::default(),
            level: 0.5,
            bottleneck: "initializing".into(),
            stability: 0.5,
        }
    }
}

/// Player consciousness state.
#[derive(Resource)]
pub struct PlayerConsciousness {
    #[cfg(feature = "consciousness-runtime")]
    pub equation: MasterConsciousnessEquation,
    pub level: f64,
    pub bottleneck: String,
    pub stability: f64,
}

impl Default for PlayerConsciousness {
    fn default() -> Self {
        Self {
            #[cfg(feature = "consciousness-runtime")]
            equation: MasterConsciousnessEquation::default(),
            level: 0.5,
            bottleneck: "awakening".into(),
            stability: 0.5,
        }
    }
}

/// Update player consciousness from biometrics.
pub fn player_consciousness_system(
    biometrics: Res<BiometricsCtx>,
    leviathan: Res<LeviathanState>,
    harmony: Res<crate::systems::harmonies::LocalHarmonyState>,
    mut player_c: ResMut<PlayerConsciousness>,
) {
    let stress = biometrics.encoder.compute_stress_vector();
    let load = biometrics.model.allostatic_load;

    // Map player state to consciousness inputs
    let danger = match leviathan.phase {
        SleepPhase::Dormant => 0.0,
        SleepPhase::Stirring => 0.3,
        SleepPhase::Awake => 0.7,
        SleepPhase::Hunting => 1.0,
    };

    let phi = (1.0f64 - load as f64) * 0.8f64 + 0.2f64;
    let broadcast = (1.0f64 - stress.arousal as f64 * 0.5f64).max(0.1f64);
    let working_memory = (1.0f64 - load as f64 * 0.6f64).max(0.1f64);
    let attention = (stress.arousal as f64 * 0.3f64 + 0.5f64).min(1.0f64);
    let recurrence = (harmony.total_energy as f64 / 8.0f64).min(1.0f64);
    let embodiment = (1.0_f64 - danger * 0.3f64).max(0.2f64);
    let knowledge = (harmony
        .activations
        .iter()
        .filter(|&&a| a as f64 > 0.3f64)
        .count() as f64
        / 8.0f64)
        .min(1.0f64);
    let synchrony = if harmony.is_sanctuary {
        0.9f64
    } else {
        0.4f64 + harmony.total_energy as f64 * 0.05f64
    };

    #[cfg(feature = "consciousness-runtime")]
    {
        let inputs = ConsciousnessInputs {
            phi,
            broadcast,
            working_memory,
            attention,
            recurrence,
            embodiment,
            knowledge,
            synchrony,
        };
        let result = player_c.equation.compute(&inputs);
        player_c.level = result.consciousness_level;
        player_c.bottleneck = result.bottleneck_name.clone();
        player_c.stability = result.temporal_stability;
    }

    #[cfg(not(feature = "consciousness-runtime"))]
    {
        let average = (phi
            + broadcast
            + working_memory
            + attention
            + recurrence
            + embodiment
            + knowledge
            + synchrony)
            / 8.0;
        player_c.level = average.clamp(0.0, 1.0);
        player_c.bottleneck = if load > 0.7 {
            "allostatic_load".into()
        } else if danger > 0.5 {
            "embodiment".into()
        } else {
            "stable".into()
        };
        player_c.stability = (1.0 - load as f64 * 0.5).clamp(0.0, 1.0);
    }
}

/// Update NPC consciousness from game state.
/// Burnout ceiling: allostatic_load > 0.8 caps Phi (ported from multiworld sim).
pub fn npc_consciousness_system(
    mut npcs: Query<(
        &CrewNpc,
        &mut NpcConsciousness,
        Option<&crate::systems::psychology::PsychologicalNeeds>,
    )>,
    leviathan: Res<LeviathanState>,
    harmony: Res<crate::systems::harmonies::LocalHarmonyState>,
) {
    let danger = match leviathan.phase {
        SleepPhase::Dormant => 0.0,
        SleepPhase::Stirring => 0.3,
        SleepPhase::Awake => 0.7,
        SleepPhase::Hunting => 1.0,
    };

    for (npc, mut consciousness, needs) in &mut npcs {
        let caution = npc.caution as f64;
        let load = needs.map(|n| n.allostatic_load).unwrap_or(0.0);

        // Burnout degrades integration capacity
        let burnout_penalty = if load > crate::systems::psychology::BURNOUT_THRESHOLD {
            1.0 - ((load - crate::systems::psychology::BURNOUT_THRESHOLD) * 5.0).min(1.0) as f64
        } else {
            1.0
        };

        let phi = (0.6 + (1.0 - caution) * 0.3) * burnout_penalty;
        let broadcast = (1.0_f64 - danger * 0.4).max(0.2);
        let working_memory = (0.5 + caution * 0.2) * burnout_penalty;
        let attention = 0.5 + (1.0 - caution) * 0.3;
        let recurrence = (harmony.total_energy as f64 / 8.0).min(1.0);
        let embodiment = (1.0_f64 - danger * 0.5).max(0.2);
        let knowledge = 0.5;
        let synchrony = if harmony.is_sanctuary {
            0.8
        } else {
            0.3 + harmony.total_energy as f64 * 0.04
        };

        #[cfg(feature = "consciousness-runtime")]
        {
            let inputs = ConsciousnessInputs {
                phi,
                broadcast,
                working_memory,
                attention,
                recurrence,
                embodiment,
                knowledge,
                synchrony,
            };
            let result = consciousness.equation.compute(&inputs);
            consciousness.level = result.consciousness_level;
            consciousness.bottleneck = if load > crate::systems::psychology::BURNOUT_THRESHOLD {
                "burnout".into()
            } else {
                result.bottleneck_name.clone()
            };
            consciousness.stability = result.temporal_stability;
        }

        #[cfg(not(feature = "consciousness-runtime"))]
        {
            consciousness.level = ((phi
                + broadcast
                + working_memory
                + attention
                + recurrence
                + embodiment
                + knowledge
                + synchrony)
                / 8.0)
                .clamp(0.0, 1.0);
            consciousness.bottleneck = if load > crate::systems::psychology::BURNOUT_THRESHOLD {
                "burnout".into()
            } else if danger > 0.5 {
                "threat".into()
            } else {
                "stable".into()
            };
            consciousness.stability = (1.0 - load as f64 * 0.5).clamp(0.0, 1.0);
        }
    }
}
