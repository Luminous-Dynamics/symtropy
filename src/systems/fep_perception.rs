// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Ephemeral observer-local inputs for the launcher FEP agent.
//!
//! This module does not own world truth. It stores bounded observations with an
//! adapter-local generation, confidence, and staleness horizon. The authoritative
//! settlement and Leviathan systems remain elsewhere; forgetting or uncertainty here
//! can change an NPC's behavior but cannot change the simulated world.

use bevy::prelude::Vec2;

pub const FULL_SCALE_BPS: u16 = 10_000;
pub const NEUTRAL_INFRASTRUCTURE_PRIOR_BPS: u16 = 5_000;
pub const NO_LOCAL_DANGER_CUE_BPS: u16 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PerceivedScalar {
    pub value_bps: u16,
    pub confidence_bps: u16,
    pub observed_generation: u64,
    pub stale_after_generation: u64,
}

impl PerceivedScalar {
    pub fn new(
        value: f64,
        confidence: f64,
        generation: u64,
        lifetime_generations: u64,
    ) -> Self {
        Self {
            value_bps: quantize_unit(value),
            confidence_bps: quantize_unit(confidence),
            observed_generation: generation,
            stale_after_generation: generation.saturating_add(lifetime_generations),
        }
    }

    pub fn is_stale(self, generation: u64) -> bool {
        generation > self.stale_after_generation
    }

    /// Blend a still-fresh observation with an explicit prior according to confidence.
    /// Stale observations contribute nothing and resolve to the prior.
    pub fn effective_value(self, generation: u64, prior_bps: u16) -> f64 {
        if self.is_stale(generation) {
            return unit_from_bps(prior_bps);
        }
        let confidence = u32::from(self.confidence_bps);
        let inverse = u32::from(FULL_SCALE_BPS) - confidence;
        let blended = (u32::from(self.value_bps) * confidence
            + u32::from(prior_bps) * inverse)
            / u32::from(FULL_SCALE_BPS);
        unit_from_bps(blended as u16)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PerceivedWorldFrame {
    /// Adapter-local update generation. This is deliberately not a world/simulation tick.
    pub generation: u64,
    pub local_danger_cue: Option<PerceivedScalar>,
    pub local_water: Option<PerceivedScalar>,
    pub local_power: Option<PerceivedScalar>,
}

impl PerceivedWorldFrame {
    pub fn advance(&mut self) -> u64 {
        self.generation = self.generation.saturating_add(1);
        self.generation
    }

    pub fn observe_danger(&mut self, value: f64, confidence: f64, lifetime: u64) {
        self.local_danger_cue = Some(PerceivedScalar::new(
            value,
            confidence,
            self.generation,
            lifetime,
        ));
    }

    pub fn observe_water(&mut self, value: f64, confidence: f64, lifetime: u64) {
        self.local_water = Some(PerceivedScalar::new(
            value,
            confidence,
            self.generation,
            lifetime,
        ));
    }

    pub fn observe_power(&mut self, value: f64, confidence: f64, lifetime: u64) {
        self.local_power = Some(PerceivedScalar::new(
            value,
            confidence,
            self.generation,
            lifetime,
        ));
    }

    /// Strength of locally observed danger/risk cues. Zero means no current local cue;
    /// it does **not** mean the hidden global threat state is known to be safe.
    pub fn danger_signal(&self) -> f64 {
        self.local_danger_cue.map_or(
            unit_from_bps(NO_LOCAL_DANGER_CUE_BPS),
            |observation| {
                observation.effective_value(self.generation, NO_LOCAL_DANGER_CUE_BPS)
            },
        )
    }

    /// Local infrastructure estimates fall back to a neutral 0.5 prior when unknown.
    pub fn water_estimate(&self) -> f64 {
        self.local_water.map_or(
            unit_from_bps(NEUTRAL_INFRASTRUCTURE_PRIOR_BPS),
            |observation| {
                observation.effective_value(
                    self.generation,
                    NEUTRAL_INFRASTRUCTURE_PRIOR_BPS,
                )
            },
        )
    }

    pub fn power_estimate(&self) -> f64 {
        self.local_power.map_or(
            unit_from_bps(NEUTRAL_INFRASTRUCTURE_PRIOR_BPS),
            |observation| {
                observation.effective_value(
                    self.generation,
                    NEUTRAL_INFRASTRUCTURE_PRIOR_BPS,
                )
            },
        )
    }
}

/// One world-facing sample that a launcher adapter has decided is locally observable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalScalarSample {
    pub position: Vec2,
    pub value: f64,
}

/// Select the strongest spatially credible local sample without depending on query order.
///
/// Confidence falls linearly with distance. Ties in confidence are resolved by the
/// larger bounded value, making the result independent of source iteration order.
pub fn strongest_local_sample(
    observer: Vec2,
    samples: &[LocalScalarSample],
    range: f32,
) -> Option<(f64, f64)> {
    if !range.is_finite() || range <= 0.0 {
        return None;
    }

    let mut best: Option<(u16, u16)> = None;
    for sample in samples {
        let distance = observer.distance(sample.position);
        if !distance.is_finite() || distance > range {
            continue;
        }
        let confidence = 1.0 - f64::from(distance / range);
        let candidate = (quantize_unit(confidence), quantize_unit(sample.value));
        if best.is_none_or(|current| candidate > current) {
            best = Some(candidate);
        }
    }

    best.map(|(confidence_bps, value_bps)| {
        (unit_from_bps(value_bps), unit_from_bps(confidence_bps))
    })
}

/// Strongest locally audible/observable noise-risk cue.
///
/// This intentionally measures a *local cue*, not Leviathan phase. No local noise
/// does not prove global safety; it only yields zero cue strength for this channel.
pub fn local_noise_risk_cue(
    observer: Vec2,
    samples: &[LocalScalarSample],
    range: f32,
) -> f64 {
    strongest_local_sample(observer, samples, range)
        .map(|(value, confidence)| value * confidence)
        .unwrap_or(0.0)
        .clamp(0.0, 1.0)
}

fn quantize_unit(value: f64) -> u16 {
    if !value.is_finite() {
        return 0;
    }
    (value.clamp(0.0, 1.0) * f64::from(FULL_SCALE_BPS)).round() as u16
}

fn unit_from_bps(value: u16) -> f64 {
    f64::from(value.min(FULL_SCALE_BPS)) / f64::from(FULL_SCALE_BPS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_observation_returns_explicit_prior() {
        let observation = PerceivedScalar::new(1.0, 1.0, 10, 2);
        assert_eq!(observation.effective_value(12, 5_000), 1.0);
        assert_eq!(observation.effective_value(13, 5_000), 0.5);
    }

    #[test]
    fn confidence_blends_observation_with_prior() {
        let observation = PerceivedScalar::new(1.0, 0.5, 1, 10);
        assert_eq!(observation.effective_value(2, 5_000), 0.75);
    }

    #[test]
    fn unknown_infrastructure_uses_neutral_prior_but_danger_means_no_local_cue() {
        let frame = PerceivedWorldFrame::default();
        assert_eq!(frame.water_estimate(), 0.5);
        assert_eq!(frame.power_estimate(), 0.5);
        assert_eq!(frame.danger_signal(), 0.0);
    }

    #[test]
    fn strongest_sample_is_independent_of_iteration_order() {
        let observer = Vec2::ZERO;
        let samples = [
            LocalScalarSample {
                position: Vec2::new(10.0, 0.0),
                value: 0.2,
            },
            LocalScalarSample {
                position: Vec2::new(2.0, 0.0),
                value: 0.8,
            },
        ];
        let reversed = [samples[1], samples[0]];
        assert_eq!(
            strongest_local_sample(observer, &samples, 20.0),
            strongest_local_sample(observer, &reversed, 20.0)
        );
    }

    #[test]
    fn out_of_range_sample_is_not_observed() {
        let samples = [LocalScalarSample {
            position: Vec2::new(100.0, 0.0),
            value: 1.0,
        }];
        assert_eq!(strongest_local_sample(Vec2::ZERO, &samples, 50.0), None);
    }

    #[test]
    fn noise_cue_is_local_and_distance_attenuated() {
        let samples = [LocalScalarSample {
            position: Vec2::new(5.0, 0.0),
            value: 1.0,
        }];
        let near = local_noise_risk_cue(Vec2::ZERO, &samples, 20.0);
        let far = local_noise_risk_cue(Vec2::new(-10.0, 0.0), &samples, 20.0);
        assert!(near > far);
    }

    #[test]
    fn frame_updates_do_not_change_world_truth_types() {
        let mut frame = PerceivedWorldFrame::default();
        frame.advance();
        frame.observe_water(0.2, 1.0, 10);
        frame.observe_power(0.8, 1.0, 10);
        frame.observe_danger(0.6, 1.0, 2);
        assert_eq!(frame.water_estimate(), 0.2);
        assert_eq!(frame.power_estimate(), 0.8);
        assert_eq!(frame.danger_signal(), 0.6);
    }
}
