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

/// A candidate location that has already crossed an explicit presentation boundary.
///
/// `salience` is observer-facing evidence, not the hidden cause behind it. For example,
/// an engineer may receive a visible low-output cue from a junction without receiving the
/// private failure bit that produced it; a medic may receive a coarse distress presentation
/// without receiving another resident's exact allostatic state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalTargetSample {
    pub position: Vec2,
    pub salience: f64,
}

/// Result of observer-local target selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalTargetObservation {
    pub position: Vec2,
    pub salience: f64,
    pub confidence: f64,
}

/// Select the strongest distance-attenuated local sample without depending on query order.
///
/// Confidence falls linearly with distance. Zero-confidence boundary samples are rejected
/// rather than refreshing memory with no information. Remaining candidates are ranked by
/// bounded `value × confidence`, then confidence, then value. The tuple ordering prevents a
/// nearby zero-valued sample from masking a slightly farther strong signal.
pub fn strongest_local_sample(
    observer: Vec2,
    samples: &[LocalScalarSample],
    range: f32,
) -> Option<(f64, f64)> {
    if !valid_observer_range(observer, range) {
        return None;
    }

    let mut best: Option<(u16, u16, u16)> = None;
    for sample in samples {
        if !sample.position.x.is_finite() || !sample.position.y.is_finite() {
            continue;
        }
        let distance = observer.distance(sample.position);
        if !distance.is_finite() || distance > range {
            continue;
        }
        let confidence_bps = quantize_unit(1.0 - f64::from(distance / range));
        if confidence_bps == 0 {
            continue;
        }
        let value_bps = quantize_unit(sample.value);
        let signal_bps = attenuated_signal_bps(value_bps, confidence_bps);
        let candidate = (signal_bps, confidence_bps, value_bps);
        if best.is_none_or(|current| candidate > current) {
            best = Some(candidate);
        }
    }

    best.map(|(_, confidence_bps, value_bps)| {
        (unit_from_bps(value_bps), unit_from_bps(confidence_bps))
    })
}

/// Select an observer-local actionable target from already-presented cues.
///
/// Hidden world flags must be projected into `salience` by a capability-appropriate
/// presentation adapter before they reach this function. Candidates outside `range` are
/// not considered. Ties are broken by coordinates, so ECS/query iteration order cannot
/// choose a different target for an otherwise identical frame.
pub fn strongest_local_target(
    observer: Vec2,
    samples: &[LocalTargetSample],
    range: f32,
) -> Option<LocalTargetObservation> {
    if !valid_observer_range(observer, range) {
        return None;
    }

    let mut best: Option<(LocalTargetObservation, (u16, u16, u16))> = None;
    for sample in samples {
        if !sample.position.x.is_finite() || !sample.position.y.is_finite() {
            continue;
        }
        let distance = observer.distance(sample.position);
        if !distance.is_finite() || distance > range {
            continue;
        }

        let confidence_bps = quantize_unit(1.0 - f64::from(distance / range));
        let salience_bps = quantize_unit(sample.salience);
        let signal_bps = attenuated_signal_bps(salience_bps, confidence_bps);
        if signal_bps == 0 {
            continue;
        }

        let observation = LocalTargetObservation {
            position: sample.position,
            salience: unit_from_bps(salience_bps),
            confidence: unit_from_bps(confidence_bps),
        };
        let rank = (signal_bps, confidence_bps, salience_bps);
        let should_replace = best.as_ref().is_none_or(|(current, current_rank)| {
            rank > *current_rank
                || (rank == *current_rank
                    && coordinate_precedes(observation.position, current.position))
        });
        if should_replace {
            best = Some((observation, rank));
        }
    }

    best.map(|(observation, _)| observation)
}

/// Project private allostatic state into a deliberately coarse outward distress cue.
///
/// This is a presentation boundary, not permission to disclose the underlying scalar.
/// Multiple internal values map to the same tier so observers cannot recover exact private
/// state from the cue. Values below the existing authored-help threshold present no cue.
pub fn presented_distress_cue(allostatic_load: f32) -> f64 {
    if !allostatic_load.is_finite() {
        return 0.0;
    }
    match allostatic_load.clamp(0.0, 1.0) {
        load if load < 0.4 => 0.0,
        load if load < 0.65 => 0.33,
        load if load < 0.8 => 0.66,
        _ => 1.0,
    }
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

fn valid_observer_range(observer: Vec2, range: f32) -> bool {
    observer.x.is_finite() && observer.y.is_finite() && range.is_finite() && range > 0.0
}

fn attenuated_signal_bps(value_bps: u16, confidence_bps: u16) -> u16 {
    ((u32::from(value_bps) * u32::from(confidence_bps)) / u32::from(FULL_SCALE_BPS)) as u16
}

fn coordinate_precedes(candidate: Vec2, current: Vec2) -> bool {
    candidate.x.total_cmp(&current.x).is_lt()
        || (candidate.x.total_cmp(&current.x).is_eq()
            && candidate.y.total_cmp(&current.y).is_lt())
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
    fn zero_confidence_boundary_sample_is_not_an_observation() {
        let samples = [LocalScalarSample {
            position: Vec2::new(20.0, 0.0),
            value: 1.0,
        }];
        assert_eq!(strongest_local_sample(Vec2::ZERO, &samples, 20.0), None);
    }

    #[test]
    fn nearby_silent_sample_does_not_mask_farther_strong_signal() {
        let samples = [
            LocalScalarSample {
                position: Vec2::ZERO,
                value: 0.0,
            },
            LocalScalarSample {
                position: Vec2::new(5.0, 0.0),
                value: 1.0,
            },
        ];
        let cue = local_noise_risk_cue(Vec2::ZERO, &samples, 20.0);
        assert!(cue > 0.7);
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
    fn target_selection_is_local_and_iteration_order_independent() {
        let observer = Vec2::ZERO;
        let samples = [
            LocalTargetSample {
                position: Vec2::new(80.0, 0.0),
                salience: 1.0,
            },
            LocalTargetSample {
                position: Vec2::new(10.0, 0.0),
                salience: 0.8,
            },
        ];
        let reversed = [samples[1], samples[0]];
        assert_eq!(
            strongest_local_target(observer, &samples, 100.0),
            strongest_local_target(observer, &reversed, 100.0)
        );
        assert_eq!(
            strongest_local_target(observer, &samples, 50.0)
                .expect("near candidate is observable")
                .position,
            Vec2::new(10.0, 0.0)
        );
    }

    #[test]
    fn target_ties_use_coordinate_order_not_query_order() {
        let observer = Vec2::ZERO;
        let samples = [
            LocalTargetSample {
                position: Vec2::new(3.0, 4.0),
                salience: 1.0,
            },
            LocalTargetSample {
                position: Vec2::new(-3.0, 4.0),
                salience: 1.0,
            },
        ];
        let reversed = [samples[1], samples[0]];
        let a = strongest_local_target(observer, &samples, 10.0).unwrap();
        let b = strongest_local_target(observer, &reversed, 10.0).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.position, Vec2::new(-3.0, 4.0));
    }

    #[test]
    fn zero_salience_or_out_of_range_targets_are_not_actionable() {
        let samples = [
            LocalTargetSample {
                position: Vec2::new(1.0, 0.0),
                salience: 0.0,
            },
            LocalTargetSample {
                position: Vec2::new(100.0, 0.0),
                salience: 1.0,
            },
        ];
        assert_eq!(strongest_local_target(Vec2::ZERO, &samples, 50.0), None);
    }

    #[test]
    fn distress_projection_is_coarse_and_hides_exact_private_load() {
        assert_eq!(presented_distress_cue(0.2), 0.0);
        assert_eq!(presented_distress_cue(0.41), presented_distress_cue(0.60));
        assert_eq!(presented_distress_cue(0.70), 0.66);
        assert_eq!(presented_distress_cue(0.95), 1.0);
        assert_eq!(presented_distress_cue(f32::NAN), 0.0);
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