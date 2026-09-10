// SPDX-License-Identifier: AGPL-3.0-or-later
//! Read-only bridge from semantic player/world experience into Muse presentation state.
//!
//! This module is deliberately downstream of simulation authority. Health, protection,
//! resources, memory, relationships, and world truth are owned elsewhere; callers may
//! only submit an already-derived bounded `ExperienceSample`. Muse cannot mutate those
//! authoritative systems through this bridge.

use bevy::prelude::*;
use symtropy_powered_frame_core::experience::{
    ExperienceError, ExperienceProjector, ExperienceSample, NarrativeExperience,
};

#[derive(Resource, Debug, Clone, Copy, PartialEq, Default)]
pub struct MuseExperience {
    projector: ExperienceProjector,
}

impl MuseExperience {
    pub fn ingest(
        &mut self,
        sample: ExperienceSample,
    ) -> Result<NarrativeExperience, ExperienceError> {
        self.projector.update(sample)
    }

    pub fn current(&self) -> NarrativeExperience {
        self.projector.current()
    }
}

pub struct MusePlugin;

impl Plugin for MusePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MuseExperience>();
    }
}

/// Convert semantic experience to the subset of Muse's current `MusicalState`
/// that has a direct presentation-level interpretation.
///
/// We intentionally leave dopamine, serotonin, noradrenaline, and
/// `consciousness_level` at Muse defaults: game-semantic experience is not evidence of
/// neurotransmitter concentrations or consciousness. `resonance` remains explicit in
/// the wrapper so the real Muse integration can use it for motif/history handling.
#[cfg(feature = "muse-audio")]
pub fn musical_frame_from_experience(experience: NarrativeExperience) -> MuseExperienceFrame {
    let projection = experience.muse_projection();
    let mut state = symthaea_muse::MusicalState::default();
    state.arousal = projection.arousal as f32;
    state.valence = projection.valence as f32;
    state.prediction_error = projection.prediction_error as f32;

    MuseExperienceFrame {
        state,
        resonance: projection.resonance as f32,
    }
}

#[cfg(feature = "muse-audio")]
#[derive(Debug, Clone)]
pub struct MuseExperienceFrame {
    pub state: symthaea_muse::MusicalState,
    /// Semantic memory/history resonance [0, 1], kept separate from neuro-style fields.
    pub resonance: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_smooths_bounded_samples_without_world_access() {
        let mut bridge = MuseExperience::default();
        let first = bridge
            .ingest(ExperienceSample {
                threat: 1.0,
                uncertainty: 0.8,
                agency: 0.2,
                physiological_strain: 0.7,
                resource_pressure: 0.6,
                protection_instability: 0.9,
                memory_resonance: 0.4,
                social_connection: 0.3,
                loss: 0.2,
                causal_significance: 0.7,
            })
            .unwrap();
        assert!(first.threat > 0.0 && first.threat < 1.0);
        assert_eq!(bridge.current(), first);
    }

    #[cfg(feature = "muse-audio")]
    #[test]
    fn muse_mapping_does_not_infer_neuro_or_consciousness_values() {
        let defaults = symthaea_muse::MusicalState::default();
        let frame = musical_frame_from_experience(NarrativeExperience {
            threat: 1.0,
            uncertainty: 1.0,
            agency: 0.0,
            physiological_strain: 1.0,
            resource_pressure: 1.0,
            protection_instability: 1.0,
            memory_resonance: 1.0,
            social_connection: 0.0,
            loss: 1.0,
            causal_significance: 1.0,
        });
        assert!(frame.state.arousal > defaults.arousal);
        assert!(frame.state.valence < defaults.valence);
        assert!(frame.state.prediction_error > defaults.prediction_error);
        assert_eq!(frame.state.dopamine, defaults.dopamine);
        assert_eq!(frame.state.serotonin, defaults.serotonin);
        assert_eq!(frame.state.noradrenaline, defaults.noradrenaline);
        assert_eq!(
            frame.state.consciousness_level,
            defaults.consciousness_level
        );
        assert!(frame.resonance > 0.5);
    }
}
