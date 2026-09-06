// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Organism physiology primitives.
//!
//! This module separates relatively stable metabolic traits from mutable
//! individual physiological state. The legacy [`crate::Metabolism`] type is
//! intentionally preserved for existing callers; conversion helpers provide a
//! compatibility bridge while later ecology work moves toward the split model.

use std::ops::Range;

use crate::{Metabolism, sanitize_unit};

/// Relatively stable metabolic characteristics of an organism or phenotype.
///
/// These values describe capability/tolerance, not the organism's current
/// condition. They are therefore suitable inputs to species/phenotype models.
#[derive(Debug, Clone, PartialEq)]
pub struct MetabolicTraits {
    pub oxygen_need: f32,
    pub heat_tolerance: Range<f32>,
    pub toxin_tolerance: f32,
    pub hunger_rate: f32,
    pub recovery_rate: f32,
}

impl Default for MetabolicTraits {
    fn default() -> Self {
        let legacy = Metabolism::default();
        Self::from(&legacy)
    }
}

impl From<&Metabolism> for MetabolicTraits {
    fn from(value: &Metabolism) -> Self {
        Self {
            oxygen_need: value.oxygen_need,
            heat_tolerance: value.heat_tolerance.clone(),
            toxin_tolerance: value.toxin_tolerance,
            hunger_rate: value.hunger_rate,
            recovery_rate: value.recovery_rate,
        }
    }
}

/// Mutable condition of one organism.
///
/// Unlike [`MetabolicTraits`], these values may change every physiological
/// update and must never be inferred from render state or animation state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysiologyState {
    pub energy: f32,
    pub hydration: f32,
    pub tissue_integrity: f32,
    pub thermal_stress: f32,
    pub toxin_load: f32,
    pub infection_load: f32,
}

impl Default for PhysiologyState {
    fn default() -> Self {
        let legacy = Metabolism::default();
        Self::from(&legacy)
    }
}

impl From<&Metabolism> for PhysiologyState {
    fn from(value: &Metabolism) -> Self {
        Self {
            energy: sanitize_unit(value.energy),
            hydration: sanitize_unit(value.hydration),
            tissue_integrity: 1.0,
            thermal_stress: 0.0,
            toxin_load: 0.0,
            infection_load: 0.0,
        }
    }
}

/// Explainable decomposition of physiological stress.
///
/// Keeping the components separate lets observability and later organism
/// biography systems answer *why* an organism is stressed instead of exposing
/// only an opaque scalar.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PhysiologicalStress {
    pub thermal: f32,
    pub toxin: f32,
    pub oxygen: f32,
    pub hydration: f32,
    pub tissue: f32,
    pub infection: f32,
}

impl PhysiologicalStress {
    /// Bounded aggregate stress for callers that require a scalar viability
    /// input. Component values remain available for explanation/telemetry.
    pub fn total(self) -> f32 {
        sanitize_unit(
            self.thermal + self.toxin + self.oxygen + self.hydration + self.tissue + self.infection,
        )
    }
}

impl PhysiologyState {
    /// Compute current stress from stable traits, mutable physiological state,
    /// and sampled environmental conditions.
    ///
    /// This remains deliberately small and deterministic. Richer habitat
    /// sampling is introduced separately so this type does not become coupled
    /// to a particular field/grid representation.
    pub fn stress_from_fields(
        self,
        traits: &MetabolicTraits,
        heat_c: f32,
        toxin: f32,
        oxygen: f32,
        moisture: f32,
    ) -> PhysiologicalStress {
        let environmental_thermal = if heat_c < traits.heat_tolerance.start {
            (traits.heat_tolerance.start - heat_c) / 20.0
        } else if heat_c > traits.heat_tolerance.end {
            (heat_c - traits.heat_tolerance.end) / 20.0
        } else {
            0.0
        };

        let external_toxin = (toxin - traits.toxin_tolerance).max(0.0);
        let oxygen_deficit = (traits.oxygen_need - oxygen).max(0.0);
        let ambient_dryness = (0.5 - moisture).max(0.0) * 0.5;
        let hydration_deficit = (1.0 - self.hydration).max(0.0) * 0.5;

        PhysiologicalStress {
            thermal: sanitize_unit(environmental_thermal + self.thermal_stress),
            toxin: sanitize_unit(external_toxin + self.toxin_load * 0.5),
            oxygen: sanitize_unit(oxygen_deficit),
            hydration: sanitize_unit(hydration_deficit + ambient_dryness),
            tissue: sanitize_unit((1.0 - self.tissue_integrity).max(0.0) * 0.5),
            infection: sanitize_unit(self.infection_load * 0.5),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_metabolism_splits_traits_from_mutable_state() {
        let legacy = Metabolism {
            energy: 0.42,
            hydration: 0.73,
            oxygen_need: 0.31,
            heat_tolerance: 8.0..29.0,
            toxin_tolerance: 0.17,
            hunger_rate: 0.04,
            recovery_rate: 0.08,
        };

        let traits = MetabolicTraits::from(&legacy);
        let state = PhysiologyState::from(&legacy);

        assert_eq!(traits.oxygen_need, 0.31);
        assert_eq!(traits.heat_tolerance, 8.0..29.0);
        assert_eq!(traits.toxin_tolerance, 0.17);
        assert_eq!(state.energy, 0.42);
        assert_eq!(state.hydration, 0.73);
        assert_eq!(state.tissue_integrity, 1.0);
    }

    #[test]
    fn mutable_condition_does_not_change_metabolic_traits() {
        let traits = MetabolicTraits::default();
        let original = traits.clone();
        let state = PhysiologyState {
            energy: 0.1,
            hydration: 0.2,
            tissue_integrity: 0.4,
            infection_load: 0.7,
            ..PhysiologyState::default()
        };

        assert_eq!(traits, original);
        assert_eq!(state.energy, 0.1);
        assert_eq!(state.hydration, 0.2);
        assert_eq!(state.tissue_integrity, 0.4);
        assert_eq!(state.infection_load, 0.7);
    }

    #[test]
    fn physiological_stress_is_explainable_and_deterministic() {
        let traits = MetabolicTraits {
            oxygen_need: 0.4,
            heat_tolerance: 10.0..30.0,
            toxin_tolerance: 0.2,
            ..MetabolicTraits::default()
        };
        let state = PhysiologyState {
            hydration: 0.5,
            tissue_integrity: 0.8,
            toxin_load: 0.2,
            infection_load: 0.3,
            ..PhysiologyState::default()
        };

        let a = state.stress_from_fields(&traits, 40.0, 0.6, 0.1, 0.2);
        let b = state.stress_from_fields(&traits, 40.0, 0.6, 0.1, 0.2);

        assert_eq!(a, b);
        assert!(a.thermal > 0.0);
        assert!(a.toxin > 0.0);
        assert!(a.oxygen > 0.0);
        assert!(a.hydration > 0.0);
        assert!(a.tissue > 0.0);
        assert!(a.infection > 0.0);
        assert!(a.total() > 0.0);
        assert!(a.total() <= 1.0);
    }

    #[test]
    fn healthy_state_in_suitable_fields_has_no_stress() {
        let traits = MetabolicTraits::default();
        let state = PhysiologyState::default();
        let stress = state.stress_from_fields(&traits, 20.0, 0.0, 1.0, 1.0);

        assert_eq!(stress, PhysiologicalStress::default());
        assert_eq!(stress.total(), 0.0);
    }
}
