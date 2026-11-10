// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic phase oscillators for distinguishing pairwise coupling from
//! synchronization caused by a shared external input.
//!
//! High synchrony is an observable dynamical property, not evidence of
//! communication, collective cognition, life, agency, intelligence, or
//! consciousness.

use std::error::Error;
use std::f64::consts::{PI, TAU};
use std::fmt;

pub const OSCILLATOR_GENERATOR_ID: &str = "OLI-ABIOTIC-OSCILLATOR-001";
pub const OSCILLATOR_EVIDENCE_CLASS: &str = "SYNTHETIC_KNOWN_TRUTH";
pub const MAX_PHASE_INCREMENT_RAD: f64 = 0.25;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhaseOscillatorConfig {
    pub count: usize,
    pub seed: u32,
    pub minimum_natural_frequency: f64,
    pub maximum_natural_frequency: f64,
    pub coupling_strength: f64,
    pub forcing_amplitude: f64,
    pub forcing_frequency: f64,
    pub dt: f64,
}

impl PhaseOscillatorConfig {
    pub const fn reference_population() -> Self {
        Self {
            count: 32,
            seed: 20_261_110,
            minimum_natural_frequency: 0.8,
            maximum_natural_frequency: 1.2,
            coupling_strength: 0.0,
            forcing_amplitude: 0.0,
            forcing_frequency: 1.0,
            dt: 0.01,
        }
    }

    pub fn validate(self) -> Result<(), OscillatorError> {
        if self.count < 2 {
            return Err(OscillatorError::TooFewOscillators);
        }

        for (name, value) in [
            ("minimum_natural_frequency", self.minimum_natural_frequency),
            ("maximum_natural_frequency", self.maximum_natural_frequency),
            ("coupling_strength", self.coupling_strength),
            ("forcing_amplitude", self.forcing_amplitude),
            ("forcing_frequency", self.forcing_frequency),
            ("dt", self.dt),
        ] {
            if !value.is_finite() {
                return Err(OscillatorError::NonFiniteParameter(name));
            }
        }

        if self.minimum_natural_frequency > self.maximum_natural_frequency {
            return Err(OscillatorError::InvalidFrequencyRange);
        }
        if self.coupling_strength < 0.0 {
            return Err(OscillatorError::NegativeParameter("coupling_strength"));
        }
        if self.forcing_amplitude < 0.0 {
            return Err(OscillatorError::NegativeParameter("forcing_amplitude"));
        }
        if self.forcing_frequency < 0.0 {
            return Err(OscillatorError::NegativeParameter("forcing_frequency"));
        }
        if self.dt <= 0.0 {
            return Err(OscillatorError::NonPositiveParameter("dt"));
        }

        // The largest possible instantaneous rate is bounded by the largest
        // natural frequency magnitude plus the coupling and forcing bounds.
        // This is a temporal-resolution contract, not a claim of a global
        // nonlinear stability theorem.
        let maximum_natural = self
            .minimum_natural_frequency
            .abs()
            .max(self.maximum_natural_frequency.abs());
        let maximum_rate = maximum_natural
            + self.forcing_frequency
            + self.coupling_strength
            + self.forcing_amplitude;
        let phase_increment = maximum_rate * self.dt;
        if phase_increment > MAX_PHASE_INCREMENT_RAD {
            return Err(OscillatorError::PhaseResolutionViolation {
                maximum_increment: phase_increment,
                maximum_allowed: MAX_PHASE_INCREMENT_RAD,
            });
        }

        Ok(())
    }

    pub const fn with_coupling(self, coupling_strength: f64) -> Self {
        Self {
            coupling_strength,
            ..self
        }
    }

    pub const fn with_common_forcing(
        self,
        forcing_amplitude: f64,
        forcing_frequency: f64,
    ) -> Self {
        Self {
            forcing_amplitude,
            forcing_frequency,
            ..self
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OscillatorSummary {
    pub count: usize,
    pub steps: usize,
    pub time: f64,
    pub mean_natural_frequency: f64,
    pub order_parameter: f64,
    pub mean_absolute_pairwise_phase_difference: f64,
    /// Time-averaged phase-lag coherence relative to the external driver.
    /// Absent when the external driver is disabled.
    pub common_input_phase_locking: Option<f64>,
    pub mean_phase_lag_to_input: Option<f64>,
    pub coupling_strength: f64,
    pub forcing_amplitude: f64,
    pub forcing_frequency: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OscillatorError {
    TooFewOscillators,
    NonFiniteParameter(&'static str),
    InvalidFrequencyRange,
    NegativeParameter(&'static str),
    NonPositiveParameter(&'static str),
    PhaseResolutionViolation {
        maximum_increment: f64,
        maximum_allowed: f64,
    },
    NonFiniteState {
        oscillator: usize,
    },
}

impl fmt::Display for OscillatorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewOscillators => {
                write!(formatter, "at least two oscillators are required")
            }
            Self::NonFiniteParameter(name) => {
                write!(formatter, "parameter {name} must be finite")
            }
            Self::InvalidFrequencyRange => {
                write!(formatter, "minimum natural frequency exceeds maximum")
            }
            Self::NegativeParameter(name) => {
                write!(formatter, "parameter {name} must be non-negative")
            }
            Self::NonPositiveParameter(name) => {
                write!(formatter, "parameter {name} must be positive")
            }
            Self::PhaseResolutionViolation {
                maximum_increment,
                maximum_allowed,
            } => write!(
                formatter,
                "maximum phase increment {maximum_increment} rad exceeds declared resolution limit {maximum_allowed} rad"
            ),
            Self::NonFiniteState { oscillator } => {
                write!(formatter, "oscillator {oscillator} produced a non-finite phase")
            }
        }
    }
}

impl Error for OscillatorError {}

#[derive(Debug, Clone, PartialEq)]
pub struct PhaseOscillatorSystem {
    config: PhaseOscillatorConfig,
    natural_frequencies: Vec<f64>,
    phases: Vec<f64>,
    steps: usize,
    lag_sine_sum: f64,
    lag_cosine_sum: f64,
    lag_samples: usize,
}

impl PhaseOscillatorSystem {
    pub fn new(config: PhaseOscillatorConfig) -> Result<Self, OscillatorError> {
        config.validate()?;

        let mut rng = DeterministicLcg::new(config.seed);
        let span = config.maximum_natural_frequency - config.minimum_natural_frequency;
        let denominator = (config.count - 1) as f64;

        let natural_frequencies = (0..config.count)
            .map(|index| {
                config.minimum_natural_frequency + span * index as f64 / denominator
            })
            .collect();

        let phases = (0..config.count)
            .map(|_| rng.next_f64() * TAU - PI)
            .collect();

        Ok(Self {
            config,
            natural_frequencies,
            phases,
            steps: 0,
            lag_sine_sum: 0.0,
            lag_cosine_sum: 0.0,
            lag_samples: 0,
        })
    }

    /// Advance one simultaneous explicit-Euler update.
    ///
    /// All derivatives are evaluated from the previous phase vector. A
    /// non-finite candidate phase rejects the entire update without mutation.
    pub fn step(&mut self) -> Result<(), OscillatorError> {
        self.config.validate()?;
        let sine_sum = self.phases.iter().map(|phase| phase.sin()).sum::<f64>();
        let cosine_sum = self.phases.iter().map(|phase| phase.cos()).sum::<f64>();
        let count = self.config.count as f64;
        let time = self.steps as f64 * self.config.dt;

        let mut next = Vec::with_capacity(self.config.count);
        for (index, (phase, natural_frequency)) in self
            .phases
            .iter()
            .copied()
            .zip(self.natural_frequencies.iter().copied())
            .enumerate()
        {
            let coupling = self.config.coupling_strength / count
                * (sine_sum * phase.cos() - cosine_sum * phase.sin());
            let driver_phase = self.config.forcing_frequency * time;
            let common_input = self.config.forcing_amplitude * (driver_phase - phase).sin();
            let derivative = natural_frequency + coupling + common_input;
            let candidate = wrap_phase(phase + self.config.dt * derivative);

            if !candidate.is_finite() {
                return Err(OscillatorError::NonFiniteState {
                    oscillator: index,
                });
            }
            next.push(candidate);
        }

        self.phases = next;
        self.steps += 1;

        // Measure persistent phase lag over time. The instantaneous order
        // parameter alone cannot establish locking to a shared driver because
        // a global phase rotation leaves its magnitude unchanged.
        if self.config.forcing_amplitude > 0.0 {
            let driver_phase = self.config.forcing_frequency
                * self.steps as f64
                * self.config.dt;
            let (sine_sum, cosine_sum) = self.phases.iter().fold((0.0, 0.0), |sum, phase| {
                let relative_phase = phase - driver_phase;
                (sum.0 + relative_phase.sin(), sum.1 + relative_phase.cos())
            });
            self.lag_sine_sum += sine_sum / count;
            self.lag_cosine_sum += cosine_sum / count;
            self.lag_samples += 1;
        }

        Ok(())
    }

    pub fn run(&mut self, steps: usize) -> Result<(), OscillatorError> {
        for _ in 0..steps {
            self.step()?;
        }
        Ok(())
    }

    pub fn phases(&self) -> &[f64] {
        &self.phases
    }

    pub fn natural_frequencies(&self) -> &[f64] {
        &self.natural_frequencies
    }

    pub fn config(&self) -> PhaseOscillatorConfig {
        self.config
    }

    pub fn summary(&self) -> OscillatorSummary {
        let (sine_sum, cosine_sum) = self.phases.iter().fold((0.0, 0.0), |sum, phase| {
            (sum.0 + phase.sin(), sum.1 + phase.cos())
        });
        let count = self.config.count as f64;
        let order_parameter = sine_sum.hypot(cosine_sum) / count;

        let time = self.steps as f64 * self.config.dt;
        let (common_input_phase_locking, mean_phase_lag_to_input) =
            if self.config.forcing_amplitude > 0.0 && self.lag_samples > 0 {
                let denominator = self.lag_samples as f64;
                (
                    Some(
                        self.lag_sine_sum.hypot(self.lag_cosine_sum)
                            / denominator,
                    ),
                    Some(self.lag_sine_sum.atan2(self.lag_cosine_sum)),
                )
            } else {
                (None, None)
            };

        let mut pairwise_difference = 0.0;
        let mut pair_count = 0usize;
        for (left_index, left_phase) in self.phases.iter().copied().enumerate() {
            for right_phase in self.phases.iter().copied().skip(left_index + 1) {
                pairwise_difference += wrap_phase(left_phase - right_phase).abs();
                pair_count += 1;
            }
        }

        OscillatorSummary {
            count: self.config.count,
            steps: self.steps,
            time,
            mean_natural_frequency: mean(&self.natural_frequencies),
            order_parameter,
            mean_absolute_pairwise_phase_difference: pairwise_difference
                / pair_count.max(1) as f64,
            common_input_phase_locking,
            mean_phase_lag_to_input,
            coupling_strength: self.config.coupling_strength,
            forcing_amplitude: self.config.forcing_amplitude,
            forcing_frequency: self.config.forcing_frequency,
        }
    }
}

fn wrap_phase(phase: f64) -> f64 {
    (phase + PI).rem_euclid(TAU) - PI
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len().max(1) as f64
}

struct DeterministicLcg {
    state: u32,
}

impl DeterministicLcg {
    const fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn next_f64(&mut self) -> f64 {
        self.state = self
            .state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        self.state as f64 / 4_294_967_296.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STEPS: usize = 5_000;

    fn reference_system(
        coupling_strength: f64,
        forcing_amplitude: f64,
    ) -> PhaseOscillatorSystem {
        let config = PhaseOscillatorConfig::reference_population()
            .with_coupling(coupling_strength)
            .with_common_forcing(forcing_amplitude, 1.0);
        PhaseOscillatorSystem::new(config).expect("reference configuration is valid")
    }

    #[test]
    fn common_input_can_synchronize_independent_oscillators_without_coupling() {
        let mut system = reference_system(0.0, 1.5);
        system.run(STEPS).expect("common-input run should succeed");
        let summary = system.summary();

        assert_eq!(summary.coupling_strength, 0.0);
        assert_eq!(summary.forcing_amplitude, 1.5);
        assert!(summary.order_parameter > 0.95);
        assert!(
            summary
                .common_input_phase_locking
                .is_some_and(|coherence| coherence > 0.95)
        );
        assert!(
            summary
                .mean_phase_lag_to_input
                .is_some_and(|lag| lag.abs() < 0.05)
        );
    }

    #[test]
    fn pairwise_coupling_can_synchronize_without_common_input() {
        let mut system = reference_system(1.5, 0.0);
        system.run(STEPS).expect("coupled run should succeed");
        let summary = system.summary();

        assert_eq!(summary.coupling_strength, 1.5);
        assert_eq!(summary.forcing_amplitude, 0.0);
        assert!(summary.order_parameter > 0.95);
        assert_eq!(summary.common_input_phase_locking, None);
        assert_eq!(summary.mean_phase_lag_to_input, None);
    }

    #[test]
    fn independent_population_is_less_synchronized_for_reference_profile() {
        let mut system = reference_system(0.0, 0.0);
        system.run(STEPS).expect("independent run should succeed");
        assert!(system.summary().order_parameter < 0.35);
    }

    #[test]
    fn matched_initial_conditions_are_exactly_reproducible() {
        let mut first = reference_system(0.0, 1.5);
        let mut second = reference_system(0.0, 1.5);

        assert_eq!(first.phases(), second.phases());
        assert_eq!(first.natural_frequencies(), second.natural_frequencies());

        first.run(STEPS).expect("first replay should succeed");
        second.run(STEPS).expect("second replay should succeed");

        assert_eq!(first.phases(), second.phases());
        assert_eq!(first.summary(), second.summary());
    }

    #[test]
    fn invalid_phase_resolution_is_rejected() {
        let mut config = PhaseOscillatorConfig::reference_population();
        config.dt = 0.2;
        assert!(matches!(
            config.validate(),
            Err(OscillatorError::PhaseResolutionViolation { .. })
        ));
    }

    #[test]
    fn invalid_oscillator_count_is_rejected() {
        let mut config = PhaseOscillatorConfig::reference_population();
        config.count = 1;
        assert!(matches!(
            config.validate(),
            Err(OscillatorError::TooFewOscillators)
        ));
    }

    #[test]
    fn synchrony_without_coupling_is_not_labelled_communication() {
        let mut system = reference_system(0.0, 1.5);
        system.run(STEPS).expect("common-input run should succeed");
        let summary = system.summary();

        assert!(summary.order_parameter > 0.95);
        assert_eq!(summary.coupling_strength, 0.0);
        assert_eq!(summary.forcing_amplitude, 1.5);
    }
}
