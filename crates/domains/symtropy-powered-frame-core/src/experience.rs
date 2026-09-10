// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Read-only semantic experience projection for presentation consumers.
//!
//! The owning simulation remains authoritative for world truth, health, protection,
//! relationships, memory, and resource state. This module only accepts already-derived
//! bounded signals, smooths them deterministically, and exposes a compact projection for
//! consumers such as Muse, UI, animation, accessibility, or cinematic direction.

use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct ExperienceSample {
    /// Immediate perceived danger pressure [0, 1].
    pub threat: f64,
    /// Epistemic uncertainty / unresolved surprise [0, 1].
    pub uncertainty: f64,
    /// Perceived ability to affect the current situation [0, 1].
    pub agency: f64,
    /// Derived bodily / embodied strain [0, 1].
    pub physiological_strain: f64,
    /// Pressure from scarce energy, supplies, oxygen, cooling, etc. [0, 1].
    pub resource_pressure: f64,
    /// Instability or imminent failure of protective systems [0, 1].
    pub protection_instability: f64,
    /// Similarity / resonance with important prior experience [0, 1].
    pub memory_resonance: f64,
    /// Felt social connection / support relevant to the present moment [0, 1].
    pub social_connection: f64,
    /// Salience of separation, bereavement, destruction, or other loss [0, 1].
    pub loss: f64,
    /// Causal importance assigned by the owning history/appraisal layer [0, 1].
    pub causal_significance: f64,
}

impl ExperienceSample {
    pub fn validate(&self) -> Result<(), ExperienceError> {
        for (name, value) in self.fields() {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(ExperienceError::OutOfRange(name, value));
            }
        }
        Ok(())
    }

    fn fields(&self) -> [(&'static str, f64); 10] {
        [
            ("threat", self.threat),
            ("uncertainty", self.uncertainty),
            ("agency", self.agency),
            ("physiological_strain", self.physiological_strain),
            ("resource_pressure", self.resource_pressure),
            ("protection_instability", self.protection_instability),
            ("memory_resonance", self.memory_resonance),
            ("social_connection", self.social_connection),
            ("loss", self.loss),
            ("causal_significance", self.causal_significance),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ExperienceFilterConfig {
    /// Fraction of an upward step accepted per update, in (0, 1].
    pub rise_alpha: f64,
    /// Fraction of a downward step accepted per update, in (0, 1].
    pub fall_alpha: f64,
}

impl Default for ExperienceFilterConfig {
    fn default() -> Self {
        Self {
            rise_alpha: 0.35,
            fall_alpha: 0.08,
        }
    }
}

impl ExperienceFilterConfig {
    pub fn validate(&self) -> Result<(), ExperienceError> {
        for (name, value) in [
            ("rise_alpha", self.rise_alpha),
            ("fall_alpha", self.fall_alpha),
        ] {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) || value == 0.0 {
                return Err(ExperienceError::InvalidFilter(name, value));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct NarrativeExperience {
    pub threat: f64,
    pub uncertainty: f64,
    pub agency: f64,
    pub physiological_strain: f64,
    pub resource_pressure: f64,
    pub protection_instability: f64,
    pub memory_resonance: f64,
    pub social_connection: f64,
    pub loss: f64,
    pub causal_significance: f64,
}

impl NarrativeExperience {
    /// Derive a deliberately small musical control surface from semantic experience.
    ///
    /// These values are presentation controls, not physiological measurements. In
    /// particular, this method does not infer neurotransmitter concentrations or
    /// consciousness from gameplay state.
    pub fn muse_projection(&self) -> MuseProjection {
        let arousal = (
            self.threat * 0.35
                + self.uncertainty * 0.15
                + self.physiological_strain * 0.20
                + self.resource_pressure * 0.10
                + self.protection_instability * 0.20
        )
            .clamp(0.0, 1.0);

        let valence = (
            self.agency * 0.35
                + self.social_connection * 0.25
                - self.threat * 0.20
                - self.loss * 0.40
                - self.physiological_strain * 0.15
        )
            .clamp(-1.0, 1.0);

        let prediction_error = (
            self.uncertainty * 0.55
                + self.protection_instability * 0.20
                + self.resource_pressure * 0.15
                + self.threat * 0.10
        )
            .clamp(0.0, 1.0);

        let resonance = (
            self.memory_resonance * 0.55
                + self.causal_significance * 0.30
                + self.loss * 0.15
        )
            .clamp(0.0, 1.0);

        MuseProjection {
            arousal,
            valence,
            prediction_error,
            resonance,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct MuseProjection {
    pub arousal: f64,
    pub valence: f64,
    pub prediction_error: f64,
    pub resonance: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ExperienceProjector {
    config: ExperienceFilterConfig,
    current: NarrativeExperience,
}

impl ExperienceProjector {
    pub fn new(config: ExperienceFilterConfig) -> Result<Self, ExperienceError> {
        config.validate()?;
        Ok(Self {
            config,
            current: NarrativeExperience::default(),
        })
    }

    pub fn current(&self) -> NarrativeExperience {
        self.current
    }

    pub fn update(
        &mut self,
        sample: ExperienceSample,
    ) -> Result<NarrativeExperience, ExperienceError> {
        sample.validate()?;
        let rise = self.config.rise_alpha;
        let fall = self.config.fall_alpha;

        self.current.threat = smooth(self.current.threat, sample.threat, rise, fall);
        self.current.uncertainty = smooth(self.current.uncertainty, sample.uncertainty, rise, fall);
        self.current.agency = smooth(self.current.agency, sample.agency, rise, fall);
        self.current.physiological_strain = smooth(
            self.current.physiological_strain,
            sample.physiological_strain,
            rise,
            fall,
        );
        self.current.resource_pressure = smooth(
            self.current.resource_pressure,
            sample.resource_pressure,
            rise,
            fall,
        );
        self.current.protection_instability = smooth(
            self.current.protection_instability,
            sample.protection_instability,
            rise,
            fall,
        );
        self.current.memory_resonance = smooth(
            self.current.memory_resonance,
            sample.memory_resonance,
            rise,
            fall,
        );
        self.current.social_connection = smooth(
            self.current.social_connection,
            sample.social_connection,
            rise,
            fall,
        );
        self.current.loss = smooth(self.current.loss, sample.loss, rise, fall);
        self.current.causal_significance = smooth(
            self.current.causal_significance,
            sample.causal_significance,
            rise,
            fall,
        );

        Ok(self.current)
    }
}

impl Default for ExperienceProjector {
    fn default() -> Self {
        Self::new(ExperienceFilterConfig::default())
            .expect("default narrative-experience filter must be valid")
    }
}

fn smooth(current: f64, target: f64, rise_alpha: f64, fall_alpha: f64) -> f64 {
    let alpha = if target >= current {
        rise_alpha
    } else {
        fall_alpha
    };
    (current + (target - current) * alpha).clamp(0.0, 1.0)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExperienceError {
    OutOfRange(&'static str, f64),
    InvalidFilter(&'static str, f64),
}

impl fmt::Display for ExperienceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfRange(name, value) => {
                write!(f, "experience signal {name} must be finite and in [0,1], got {value}")
            }
            Self::InvalidFilter(name, value) => {
                write!(f, "experience filter {name} must be finite and in (0,1], got {value}")
            }
        }
    }
}

impl Error for ExperienceError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn high_threat() -> ExperienceSample {
        ExperienceSample {
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
        }
    }

    #[test]
    fn invalid_signal_fails_closed() {
        let mut projector = ExperienceProjector::default();
        let mut sample = high_threat();
        sample.threat = 1.1;
        assert!(matches!(
            projector.update(sample),
            Err(ExperienceError::OutOfRange("threat", _))
        ));
        assert_eq!(projector.current(), NarrativeExperience::default());
    }

    #[test]
    fn rise_is_faster_than_recovery_by_default() {
        let mut projector = ExperienceProjector::default();
        let raised = projector.update(high_threat()).unwrap().threat;
        let recovered = projector
            .update(ExperienceSample::default())
            .unwrap()
            .threat;
        let rise_amount = raised;
        let fall_amount = raised - recovered;
        assert!(rise_amount > fall_amount);
        assert!(recovered > 0.0, "experience should not snap instantly to calm");
    }

    #[test]
    fn musical_projection_is_bounded_and_semantic() {
        let experience = NarrativeExperience {
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
        };
        let muse = experience.muse_projection();
        assert!((0.0..=1.0).contains(&muse.arousal));
        assert!((-1.0..=1.0).contains(&muse.valence));
        assert!((0.0..=1.0).contains(&muse.prediction_error));
        assert!((0.0..=1.0).contains(&muse.resonance));
        assert!(muse.arousal > 0.8);
        assert!(muse.valence < 0.0);
    }

    #[test]
    fn identical_sequences_replay_identically() {
        let mut a = ExperienceProjector::default();
        let mut b = ExperienceProjector::default();
        let samples = [
            ExperienceSample::default(),
            high_threat(),
            ExperienceSample {
                agency: 0.9,
                social_connection: 0.8,
                memory_resonance: 0.7,
                ..ExperienceSample::default()
            },
        ];
        for sample in samples {
            assert_eq!(a.update(sample), b.update(sample));
        }
        assert_eq!(a, b);
    }
}
