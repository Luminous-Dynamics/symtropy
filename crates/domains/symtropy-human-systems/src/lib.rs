// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! A bounded mathematical reference for the two-process sleep-regulation model.
//!
//! This crate is not a clinically validated human model. It exposes Process S
//! and Process C independently and deliberately does not select sleep/wake
//! behavior, estimate health, or predict task performance.

#![deny(unsafe_code)]

use std::f64::consts::TAU;
use std::fmt;

/// Synthetic observation-process generator and scorer-only truth fixtures.
pub mod observation;

/// Known-truth sampling-rate and task-bandwidth reference fixtures.
pub mod sampling_rate;

/// Typed synthetic action-assignment, delivery and exposure event chain.
pub mod intervention;

/// Typed, fail-closed contracts for cross-subsystem ports and coupling edges.
pub mod coupling;

/// Number of harmonics in the reference circadian expansion.
pub const C_MAX_HARMONICS: usize = 5;

/// Five-harmonic coefficients reported in the individualized two-process
/// performance-model implementation (Achermann et al., 2008).
///
/// These coefficients describe a reference waveform only. They are not
/// individualized, calibrated, or sufficient to identify circadian phase.
pub const REFERENCE_C_HARMONICS: [f64; C_MAX_HARMONICS] = [0.97, 0.22, 0.07, 0.03, 0.001];

/// Process-S kinetic parameters. No population defaults are supplied because
/// wake build-up and sleep decay time constants are individual/model specific.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProcessSParameters {
    /// Wake time constant, in hours; must be finite and greater than zero.
    pub wake_time_constant_hours: f64,
    /// Sleep time constant, in hours; must be finite and greater than zero.
    pub sleep_time_constant_hours: f64,
}

impl ProcessSParameters {
    pub fn new(
        wake_time_constant_hours: f64,
        sleep_time_constant_hours: f64,
    ) -> Result<Self, ModelError> {
        let value = Self {
            wake_time_constant_hours,
            sleep_time_constant_hours,
        };
        value.validate()?;
        Ok(value)
    }

    fn validate(self) -> Result<(), ModelError> {
        if !self.wake_time_constant_hours.is_finite()
            || self.wake_time_constant_hours <= 0.0
            || !self.sleep_time_constant_hours.is_finite()
            || self.sleep_time_constant_hours <= 0.0
        {
            return Err(ModelError::InvalidProcessSTimeConstants);
        }
        Ok(())
    }
}

/// Fixed periodic representation of Process C. Light entrainment and
/// oscillator adaptation are intentionally not included in v0.1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircadianParameters {
    /// Circadian period, in hours; usually specified near 24 h, not assumed.
    pub period_hours: f64,
    /// Phase offset, in hours, inside the sine argument (t + phase).
    pub phase_offset_hours: f64,
    /// Coefficients for harmonics 1 through 5.
    pub harmonics: [f64; C_MAX_HARMONICS],
}

impl CircadianParameters {
    pub fn new(
        period_hours: f64,
        phase_offset_hours: f64,
        harmonics: [f64; C_MAX_HARMONICS],
    ) -> Result<Self, ModelError> {
        let value = Self {
            period_hours,
            phase_offset_hours,
            harmonics,
        };
        value.validate()?;
        Ok(Self {
            phase_offset_hours: phase_offset_hours.rem_euclid(period_hours),
            ..value
        })
    }

    /// Construct the published five-harmonic waveform shape with an explicit
    /// period and phase. This does not calibrate the phase to an individual.
    pub fn reference_five_harmonic(
        period_hours: f64,
        phase_offset_hours: f64,
    ) -> Result<Self, ModelError> {
        Self::new(period_hours, phase_offset_hours, REFERENCE_C_HARMONICS)
    }

    fn validate(self) -> Result<(), ModelError> {
        if !self.period_hours.is_finite()
            || self.period_hours <= 0.0
            || !self.phase_offset_hours.is_finite()
            || self
                .harmonics
                .iter()
                .any(|coefficient| !coefficient.is_finite())
            || self
                .harmonics
                .iter()
                .map(|coefficient| coefficient.abs())
                .sum::<f64>()
                >= f64::MAX / 2.0
        {
            return Err(ModelError::InvalidCircadianParameters);
        }
        Ok(())
    }
}

/// Sleep/wake status for the entire next integration interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SleepWakeState {
    Wake,
    Sleep,
}

/// Current values of the two processes. The values have model-specific scales
/// and must not be confused with clinical measurements.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SleepRegulationSnapshot {
    /// Homeostatic Process S, constrained to [0, 1].
    pub process_s: f64,
    /// Circadian Process C in the configured harmonic scale.
    pub process_c: f64,
    /// Elapsed model time modulo the configured circadian period, in hours.
    pub elapsed_hours_in_period: f64,
}

/// Fail-closed validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelError {
    InvalidInitialProcessS,
    InvalidProcessSTimeConstants,
    InvalidCircadianParameters,
    InvalidElapsedTime,
    NonFiniteState,
}

impl fmt::Display for ModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInitialProcessS => {
                write!(formatter, "initial Process S must be finite and in [0, 1]")
            }
            Self::InvalidProcessSTimeConstants => write!(
                formatter,
                "Process S time constants must be finite and greater than zero"
            ),
            Self::InvalidCircadianParameters => write!(
                formatter,
                "circadian period, phase, and harmonic coefficients must be finite; period must be positive"
            ),
            Self::InvalidElapsedTime => {
                write!(formatter, "elapsed time must be finite and non-negative")
            }
            Self::NonFiniteState => write!(formatter, "model update produced a non-finite state"),
        }
    }
}

impl std::error::Error for ModelError {}

/// Minimal two-process sleep-regulation reference.
///
/// The caller must supply calibrated or explicitly synthetic parameters.
/// Each call to `advance` represents one interval entirely spent awake or
/// asleep; split at every sleep/wake transition to preserve the event history.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SleepRegulationModel {
    process_s: f64,
    elapsed_hours_in_period: f64,
    process_s_parameters: ProcessSParameters,
    circadian_parameters: CircadianParameters,
}

impl SleepRegulationModel {
    pub fn new(
        initial_process_s: f64,
        process_s_parameters: ProcessSParameters,
        circadian_parameters: CircadianParameters,
    ) -> Result<Self, ModelError> {
        if !initial_process_s.is_finite() || !(0.0..=1.0).contains(&initial_process_s) {
            return Err(ModelError::InvalidInitialProcessS);
        }
        process_s_parameters.validate()?;
        circadian_parameters.validate()?;
        let circadian_parameters = CircadianParameters {
            phase_offset_hours: circadian_parameters
                .phase_offset_hours
                .rem_euclid(circadian_parameters.period_hours),
            ..circadian_parameters
        };

        Ok(Self {
            process_s: initial_process_s,
            elapsed_hours_in_period: 0.0,
            process_s_parameters,
            circadian_parameters,
        })
    }

    /// Advance Process S and the independent model clock by an elapsed interval.
    ///
    /// This method does not infer sleep/wake status. The caller supplies the
    /// observed or scenario-declared state that held throughout the interval.
    /// Invalid input is rejected before any state mutation occurs.
    pub fn advance(
        &mut self,
        elapsed_hours: f64,
        state: SleepWakeState,
    ) -> Result<SleepRegulationSnapshot, ModelError> {
        if !elapsed_hours.is_finite() || elapsed_hours < 0.0 {
            return Err(ModelError::InvalidElapsedTime);
        }

        let period = self.circadian_parameters.period_hours;
        let wrapped_duration = elapsed_hours.rem_euclid(period);
        let time_until_wrap = period - self.elapsed_hours_in_period;
        let new_clock = if wrapped_duration >= time_until_wrap {
            wrapped_duration - time_until_wrap
        } else {
            self.elapsed_hours_in_period + wrapped_duration
        };

        let next_process_s = match state {
            SleepWakeState::Wake => {
                let rate = elapsed_hours / self.process_s_parameters.wake_time_constant_hours;
                self.process_s + (1.0 - self.process_s) * -(-rate).exp_m1()
            }
            SleepWakeState::Sleep => {
                let rate = elapsed_hours / self.process_s_parameters.sleep_time_constant_hours;
                self.process_s * (-rate).exp()
            }
        };

        if !new_clock.is_finite() || !next_process_s.is_finite() {
            return Err(ModelError::NonFiniteState);
        }

        // The analytic updates preserve [0, 1]; clamp only floating-point
        // round-off at the boundary.
        self.process_s = next_process_s.clamp(0.0, 1.0);
        self.elapsed_hours_in_period = new_clock;
        self.snapshot()
    }

    pub fn snapshot(&self) -> Result<SleepRegulationSnapshot, ModelError> {
        let mut process_c = 0.0;
        for (index, amplitude) in self
            .circadian_parameters
            .harmonics
            .iter()
            .copied()
            .enumerate()
        {
            let harmonic = (index + 1) as f64;
            let phase_fraction = self.elapsed_hours_in_period
                / self.circadian_parameters.period_hours
                + self.circadian_parameters.phase_offset_hours
                    / self.circadian_parameters.period_hours;
            let angle = TAU * harmonic * phase_fraction;
            process_c += amplitude * angle.sin();
        }

        if !self.process_s.is_finite()
            || !process_c.is_finite()
            || !self.elapsed_hours_in_period.is_finite()
        {
            return Err(ModelError::NonFiniteState);
        }

        Ok(SleepRegulationSnapshot {
            process_s: self.process_s,
            process_c,
            elapsed_hours_in_period: self.elapsed_hours_in_period,
        })
    }

    pub fn process_s(&self) -> f64 {
        self.process_s
    }

    pub fn elapsed_hours_in_period(&self) -> f64 {
        self.elapsed_hours_in_period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(initial_process_s: f64) -> SleepRegulationModel {
        SleepRegulationModel::new(
            initial_process_s,
            ProcessSParameters::new(16.0, 4.0).unwrap(),
            CircadianParameters::reference_five_harmonic(24.0, 3.0).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn process_s_rises_exponentially_during_wake() {
        let mut model = model(0.2);
        let result = model.advance(8.0, SleepWakeState::Wake).unwrap();
        let expected = 1.0 - (1.0 - 0.2) * (-8.0_f64 / 16.0).exp();
        assert!((result.process_s - expected).abs() < 1.0e-12);
        assert!(result.process_s > 0.2 && result.process_s < 1.0);
    }

    #[test]
    fn process_s_decays_exponentially_during_sleep() {
        let mut model = model(0.8);
        let result = model.advance(8.0, SleepWakeState::Sleep).unwrap();
        let expected = 0.8 * (-8.0_f64 / 4.0).exp();
        assert!((result.process_s - expected).abs() < 1.0e-12);
        assert!(result.process_s < 0.8 && result.process_s >= 0.0);
    }

    #[test]
    fn process_s_is_invariant_to_partitioning_a_constant_state_interval() {
        let mut single_step = model(0.35);
        let mut split_steps = model(0.35);
        let single = single_step.advance(7.5, SleepWakeState::Wake).unwrap();
        split_steps.advance(2.0, SleepWakeState::Wake).unwrap();
        let split = split_steps.advance(5.5, SleepWakeState::Wake).unwrap();

        assert!((single.process_s - split.process_s).abs() < 1.0e-12);
        assert!((single.process_c - split.process_c).abs() < 1.0e-12);
        assert!((single.elapsed_hours_in_period - split.elapsed_hours_in_period).abs() < 1.0e-12);
    }

    #[test]
    fn circadian_component_repeats_after_its_declared_period() {
        let mut model = model(0.5);
        let first = model.snapshot().unwrap().process_c;
        let after_period = model.advance(24.0, SleepWakeState::Wake).unwrap();
        assert!((first - after_period.process_c).abs() < 1.0e-12);
    }

    #[test]
    fn circadian_component_is_independent_of_sleep_wake_history() {
        let mut awake = model(0.2);
        let mut asleep = model(0.8);
        let awake_snapshot = awake.advance(9.0, SleepWakeState::Wake).unwrap();
        let asleep_snapshot = asleep.advance(9.0, SleepWakeState::Sleep).unwrap();

        assert_ne!(awake_snapshot.process_s, asleep_snapshot.process_s);
        assert!((awake_snapshot.process_c - asleep_snapshot.process_c).abs() < 1.0e-12);
    }

    #[test]
    fn zero_duration_does_not_change_state() {
        let mut model = model(0.42);
        let before = model.snapshot().unwrap();
        let after = model.advance(0.0, SleepWakeState::Wake).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn invalid_advance_is_rejected_without_mutation() {
        let mut model = model(0.42);
        let before = model.snapshot().unwrap();
        assert_eq!(
            model.advance(f64::NAN, SleepWakeState::Wake),
            Err(ModelError::InvalidElapsedTime)
        );
        assert_eq!(
            model.advance(-1.0, SleepWakeState::Sleep),
            Err(ModelError::InvalidElapsedTime)
        );
        assert_eq!(model.snapshot().unwrap(), before);
    }

    #[test]
    fn invalid_initial_state_and_parameters_are_rejected() {
        assert!(matches!(
            SleepRegulationModel::new(
                1.1,
                ProcessSParameters::new(16.0, 4.0).unwrap(),
                CircadianParameters::reference_five_harmonic(24.0, 0.0).unwrap(),
            ),
            Err(ModelError::InvalidInitialProcessS)
        ));
        assert_eq!(
            ProcessSParameters::new(0.0, 4.0),
            Err(ModelError::InvalidProcessSTimeConstants)
        );
        assert_eq!(
            CircadianParameters::new(0.0, 0.0, REFERENCE_C_HARMONICS),
            Err(ModelError::InvalidCircadianParameters)
        );
    }

    #[test]
    fn circadian_phase_is_normalized_without_changing_periodicity() {
        let normalized = CircadianParameters::new(24.0, 27.0, REFERENCE_C_HARMONICS).unwrap();
        assert!((normalized.phase_offset_hours - 3.0).abs() < 1.0e-12);
    }

    #[test]
    fn non_finite_harmonic_sum_is_rejected_before_model_mutation() {
        assert_eq!(
            CircadianParameters::new(24.0, 0.0, [f64::MAX, f64::MAX, 0.0, 0.0, 0.0],),
            Err(ModelError::InvalidCircadianParameters)
        );
    }

    #[test]
    fn identical_inputs_replay_deterministically() {
        let mut left = model(0.15);
        let mut right = model(0.15);
        for (duration, state) in [
            (10.0, SleepWakeState::Wake),
            (7.0, SleepWakeState::Sleep),
            (1.5, SleepWakeState::Wake),
        ] {
            assert_eq!(
                left.advance(duration, state).unwrap(),
                right.advance(duration, state).unwrap()
            );
        }
    }

    #[test]
    fn process_s_remains_bounded_under_extreme_but_finite_intervals() {
        let mut model = model(0.5);
        assert_eq!(
            model
                .advance(1.0e100, SleepWakeState::Wake)
                .unwrap()
                .process_s,
            1.0
        );
        assert_eq!(
            model
                .advance(1.0e100, SleepWakeState::Sleep)
                .unwrap()
                .process_s,
            0.0
        );
    }
}
