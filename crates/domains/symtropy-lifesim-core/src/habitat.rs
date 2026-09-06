// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Composable habitat sampling for living-system simulations.
//!
//! The life core intentionally does not define a world-sized 3D ecology grid.
//! Instead, domains expose habitat conditions through a small deterministic
//! query interface. Terrain, basin, atmosphere, canopy, and disturbance layers
//! can therefore compose without becoming dependencies of this crate.

use crate::physiology::{MetabolicTraits, PhysiologicalStress, PhysiologyState};

/// World-space position used by a habitat query.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HabitatPosition {
    pub x_m: f64,
    pub y_m: f64,
    pub z_m: f64,
}

impl HabitatPosition {
    pub const fn new(x_m: f64, y_m: f64, z_m: f64) -> Self {
        Self { x_m, y_m, z_m }
    }
}

/// Deterministic habitat query at one authoritative simulation tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HabitatQuery {
    pub position: HabitatPosition,
    pub simulation_tick: u64,
}

/// Domain-neutral environmental conditions relevant to organism physiology.
///
/// Concentration-like values are intentionally not forced into 0..=1: existing
/// Symtropy field domains use both normalized and absolute scales. Domain
/// adapters own unit conversion before values enter this type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HabitatSample {
    pub temperature_c: f32,
    pub moisture: f32,
    pub nutrient: f32,
    pub toxin: f32,
    pub oxygen: f32,
    pub light: f32,
    pub biomass: f32,
    pub disease: f32,
    pub signal_noise: f32,
}

impl Default for HabitatSample {
    fn default() -> Self {
        Self {
            temperature_c: 20.0,
            moisture: 1.0,
            nutrient: 0.0,
            toxin: 0.0,
            oxygen: 1.0,
            light: 1.0,
            biomass: 0.0,
            disease: 0.0,
            signal_noise: 0.0,
        }
    }
}

impl HabitatSample {
    /// Apply a partial deterministic overlay to this sample.
    ///
    /// Overlays replace only fields they explicitly provide. This is safer than
    /// imposing a universal averaging rule on quantities with different units.
    pub fn apply_override(mut self, overlay: HabitatOverride) -> Self {
        macro_rules! replace {
            ($field:ident) => {
                if let Some(value) = overlay.$field {
                    self.$field = finite_or(self.$field, value);
                }
            };
        }

        replace!(temperature_c);
        replace!(moisture);
        replace!(nutrient);
        replace!(toxin);
        replace!(oxygen);
        replace!(light);
        replace!(biomass);
        replace!(disease);
        replace!(signal_noise);
        self
    }
}

/// Partial habitat contribution from one domain layer.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HabitatOverride {
    pub temperature_c: Option<f32>,
    pub moisture: Option<f32>,
    pub nutrient: Option<f32>,
    pub toxin: Option<f32>,
    pub oxygen: Option<f32>,
    pub light: Option<f32>,
    pub biomass: Option<f32>,
    pub disease: Option<f32>,
    pub signal_noise: Option<f32>,
}

/// Source of a complete habitat sample.
pub trait HabitatSampler {
    fn sample(&self, query: HabitatQuery) -> HabitatSample;
}

/// Source of a partial habitat contribution.
pub trait HabitatOverlay {
    fn sample_override(&self, query: HabitatQuery) -> HabitatOverride;
}

/// Deterministic constant sampler useful for tests, authored baselines, and
/// fallback environments.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstantHabitat(pub HabitatSample);

impl HabitatSampler for ConstantHabitat {
    fn sample(&self, _query: HabitatQuery) -> HabitatSample {
        self.0
    }
}

/// Compose one complete sampler with one partial overlay.
///
/// Additional layers can be composed recursively without this crate knowing
/// anything about terrain, basin, atmosphere, canopy, or rendering types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlayHabitatSampler<Base, Overlay> {
    pub base: Base,
    pub overlay: Overlay,
}

impl<Base, Overlay> HabitatSampler for OverlayHabitatSampler<Base, Overlay>
where
    Base: HabitatSampler,
    Overlay: HabitatOverlay,
{
    fn sample(&self, query: HabitatQuery) -> HabitatSample {
        self.base
            .sample(query)
            .apply_override(self.overlay.sample_override(query))
    }
}

impl PhysiologyState {
    /// Evaluate physiology against a domain-neutral habitat sample.
    pub fn stress_from_habitat(
        self,
        traits: &MetabolicTraits,
        habitat: HabitatSample,
    ) -> PhysiologicalStress {
        self.stress_from_fields(
            traits,
            habitat.temperature_c,
            habitat.toxin,
            habitat.oxygen,
            habitat.moisture,
        )
    }
}

fn finite_or(previous: f32, candidate: f32) -> f32 {
    if candidate.is_finite() {
        candidate
    } else {
        previous
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy)]
    struct DryShade;

    impl HabitatOverlay for DryShade {
        fn sample_override(&self, query: HabitatQuery) -> HabitatOverride {
            assert_eq!(query.simulation_tick, 42);
            HabitatOverride {
                moisture: Some(0.2),
                light: Some(0.1),
                ..HabitatOverride::default()
            }
        }
    }

    #[test]
    fn overlays_change_only_explicit_fields() {
        let base = HabitatSample {
            temperature_c: 24.0,
            moisture: 0.8,
            nutrient: 3.0,
            toxin: 0.1,
            oxygen: 0.9,
            light: 0.7,
            biomass: 9.0,
            disease: 0.05,
            signal_noise: 0.2,
        };
        let sampler = OverlayHabitatSampler {
            base: ConstantHabitat(base),
            overlay: DryShade,
        };
        let query = HabitatQuery {
            position: HabitatPosition::new(10.0, 2.0, -3.0),
            simulation_tick: 42,
        };

        let sampled = sampler.sample(query);

        assert_eq!(sampled.moisture, 0.2);
        assert_eq!(sampled.light, 0.1);
        assert_eq!(sampled.temperature_c, base.temperature_c);
        assert_eq!(sampled.nutrient, base.nutrient);
        assert_eq!(sampled.toxin, base.toxin);
        assert_eq!(sampled.oxygen, base.oxygen);
        assert_eq!(sampled.biomass, base.biomass);
        assert_eq!(sampled.disease, base.disease);
        assert_eq!(sampled.signal_noise, base.signal_noise);
    }

    #[test]
    fn non_finite_overlay_cannot_poison_existing_sample() {
        let base = HabitatSample::default();
        let sampled = base.apply_override(HabitatOverride {
            moisture: Some(f32::NAN),
            temperature_c: Some(f32::INFINITY),
            ..HabitatOverride::default()
        });

        assert_eq!(sampled.moisture, base.moisture);
        assert_eq!(sampled.temperature_c, base.temperature_c);
    }

    #[test]
    fn habitat_stress_matches_direct_physiology_evaluation() {
        let traits = MetabolicTraits {
            oxygen_need: 0.4,
            toxin_tolerance: 0.2,
            heat_tolerance: 10.0..30.0,
            ..MetabolicTraits::default()
        };
        let state = PhysiologyState {
            hydration: 0.5,
            ..PhysiologyState::default()
        };
        let habitat = HabitatSample {
            temperature_c: 40.0,
            moisture: 0.2,
            toxin: 0.6,
            oxygen: 0.1,
            ..HabitatSample::default()
        };

        assert_eq!(
            state.stress_from_habitat(&traits, habitat),
            state.stress_from_fields(
                &traits,
                habitat.temperature_c,
                habitat.toxin,
                habitat.oxygen,
                habitat.moisture,
            )
        );
    }

    #[test]
    fn recursive_overlay_composition_is_deterministic() {
        #[derive(Debug, Clone, Copy)]
        struct ToxinPulse;
        impl HabitatOverlay for ToxinPulse {
            fn sample_override(&self, _query: HabitatQuery) -> HabitatOverride {
                HabitatOverride {
                    toxin: Some(0.7),
                    ..HabitatOverride::default()
                }
            }
        }

        let layered = OverlayHabitatSampler {
            base: OverlayHabitatSampler {
                base: ConstantHabitat(HabitatSample::default()),
                overlay: DryShade,
            },
            overlay: ToxinPulse,
        };
        let query = HabitatQuery {
            position: HabitatPosition::new(0.0, 0.0, 0.0),
            simulation_tick: 42,
        };

        let a = layered.sample(query);
        let b = layered.sample(query);
        assert_eq!(a, b);
        assert_eq!(a.moisture, 0.2);
        assert_eq!(a.light, 0.1);
        assert_eq!(a.toxin, 0.7);
    }
}
