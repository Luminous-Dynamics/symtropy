// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Known-truth sampling-rate and task-bandwidth fixtures.
//!
//! The reference filter is an ideal spectral low-pass: components above the
//! output Nyquist frequency are removed before sampling. This is an analytical
//! oracle for benchmark construction, not a deployable anti-alias filter.
//! Production filter implementations must be evaluated against this oracle.

use std::f64::consts::TAU;

/// Duration of the known-truth waveform in synthetic seconds.
pub const FIXTURE_DURATION_SECONDS: f64 = 1_200.0;
/// Native rate of the reference waveform, in samples per synthetic second.
pub const NATIVE_SAMPLE_RATE_HZ: f64 = 20.0;
/// Lowest frequency in the task-critical band.
pub const TASK_BAND_LOW_HZ: f64 = 0.05;
/// Highest frequency in the task-critical band.
pub const TASK_BAND_HIGH_HZ: f64 = 0.15;
/// Total ideal power in the two task-critical sinusoidal components.
pub const TASK_BAND_TOTAL_POWER: f64 = 0.17;

/// Rate-sweep conditions, ordered from native to information-destroying.
pub const CANONICAL_SAMPLE_RATES_HZ: [f64; 6] = [20.0, 8.0, 2.0, 0.8, 0.28, 0.08];

/// A component in the known-truth composite signal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalComponent {
    /// Human-readable synthetic component name.
    pub name: &'static str,
    /// Component frequency in cycles per synthetic second (Hz).
    pub frequency_hz: f64,
    /// Peak amplitude in arbitrary signal units.
    pub amplitude: f64,
    /// Phase at t=0, in radians.
    pub phase_radians: f64,
    /// Whether this component is part of the task-critical band.
    pub task_critical: bool,
}

/// Canonical signal components.
pub const SIGNAL_COMPONENTS: [SignalComponent; 4] = [
    SignalComponent {
        name: "slow_trend",
        frequency_hz: 0.01,
        amplitude: 0.8,
        phase_radians: 0.0,
        task_critical: false,
    },
    SignalComponent {
        name: "task_band_low",
        frequency_hz: 0.05,
        amplitude: 0.5,
        phase_radians: 0.3,
        task_critical: true,
    },
    SignalComponent {
        name: "task_band_high",
        frequency_hz: 0.15,
        amplitude: 0.3,
        phase_radians: 0.0,
        task_critical: true,
    },
    SignalComponent {
        name: "above_task_band_decoy",
        frequency_hz: 5.0,
        amplitude: 0.25,
        phase_radians: 0.7,
        task_critical: false,
    },
];

/// Expected information regime at a given sample rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingRegime {
    /// Native sampling reference.
    NativeReference,
    /// Nyquist remains above the entire declared task-critical band.
    InformationPreserving,
    /// Only part of the declared task-critical band remains observable.
    TaskBandPartiallyLost,
    /// Nyquist is below the lowest task-critical component.
    TaskBandLost,
}

/// Exact known-truth profile for a sample rate under the ideal reference filter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SamplingRateProfile {
    /// Samples per synthetic second.
    pub sampling_rate_hz: f64,
    /// Nyquist frequency.
    pub nyquist_hz: f64,
    /// Ideal task-band power fraction retained.
    pub expected_task_band_retention: f64,
    /// Qualitative task-band regime.
    pub regime: SamplingRegime,
}

/// One sample from an ideal low-pass-filtered known-truth signal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalSample {
    /// Capture time in synthetic seconds.
    pub time_seconds: f64,
    /// Signal value in arbitrary units.
    pub value: f64,
}

/// Errors from invalid sampling-rate fixture inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingRateError {
    /// Sampling frequency must be finite and positive.
    InvalidSamplingRate,
    /// Requested duration must be finite and positive.
    InvalidDuration,
    /// The requested number of samples exceeds the supported fixture limit.
    TooManySamples,
}

impl std::fmt::Display for SamplingRateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSamplingRate => {
                write!(formatter, "sampling rate must be finite and positive")
            }
            Self::InvalidDuration => write!(formatter, "duration must be finite and positive"),
            Self::TooManySamples => write!(
                formatter,
                "requested sample count exceeds the fixture limit"
            ),
        }
    }
}

impl std::error::Error for SamplingRateError {}

/// Return the four known-truth components.
pub const fn signal_components() -> &'static [SignalComponent; 4] {
    &SIGNAL_COMPONENTS
}

/// Evaluate the native composite synthetic signal.
///
/// This is an arbitrary-unit mathematical fixture, not a human signal model.
pub fn evaluate_native_signal(time_seconds: f64) -> f64 {
    SIGNAL_COMPONENTS
        .iter()
        .map(|component| evaluate_component(*component, time_seconds))
        .sum()
}

/// Calculate ideal task-band power retention for a sampling rate.
///
/// The oracle uses a brick-wall low-pass at Nyquist. A component exactly at
/// Nyquist is treated as retained here only as a boundary convention; canonical
/// fixture rates avoid that ambiguous boundary.
pub fn task_band_retention(sampling_rate_hz: f64) -> Result<f64, SamplingRateError> {
    let profile = sampling_rate_profile(sampling_rate_hz)?;
    Ok(profile.expected_task_band_retention)
}

/// Construct the known-truth profile for an arbitrary positive sampling rate.
pub fn sampling_rate_profile(
    sampling_rate_hz: f64,
) -> Result<SamplingRateProfile, SamplingRateError> {
    if !sampling_rate_hz.is_finite() || sampling_rate_hz <= 0.0 {
        return Err(SamplingRateError::InvalidSamplingRate);
    }
    let nyquist_hz = sampling_rate_hz / 2.0;
    let retained_power = SIGNAL_COMPONENTS
        .iter()
        .filter(|component| component.task_critical && component.frequency_hz <= nyquist_hz)
        .map(|component| component.amplitude * component.amplitude / 2.0)
        .sum::<f64>();
    let retention = (retained_power / TASK_BAND_TOTAL_POWER).clamp(0.0, 1.0);
    let regime = if (sampling_rate_hz - NATIVE_SAMPLE_RATE_HZ).abs() < f64::EPSILON {
        SamplingRegime::NativeReference
    } else if retention >= 1.0 - f64::EPSILON {
        SamplingRegime::InformationPreserving
    } else if retention > 0.0 {
        SamplingRegime::TaskBandPartiallyLost
    } else {
        SamplingRegime::TaskBandLost
    };
    Ok(SamplingRateProfile {
        sampling_rate_hz,
        nyquist_hz,
        expected_task_band_retention: retention,
        regime,
    })
}

/// Return the canonical rate matrix and analytically derived retention values.
pub fn canonical_rate_profiles() -> [SamplingRateProfile; 6] {
    CANONICAL_SAMPLE_RATES_HZ
        .map(|rate| sampling_rate_profile(rate).expect("canonical sample rates are valid"))
}

/// Generate samples after ideal anti-alias filtering at the target Nyquist.
///
/// This function removes components above Nyquist before sampling, so it
/// cannot produce aliasing. It is an analytical benchmark oracle only. A
/// real-world FIR/IIR/polyphase resampler must be implemented separately and
/// compared against these expected values and the native-rate reference.
pub fn sample_ideal_lowpass(
    sampling_rate_hz: f64,
    duration_seconds: f64,
) -> Result<Vec<SignalSample>, SamplingRateError> {
    if !sampling_rate_hz.is_finite() || sampling_rate_hz <= 0.0 {
        return Err(SamplingRateError::InvalidSamplingRate);
    }
    if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
        return Err(SamplingRateError::InvalidDuration);
    }
    let requested_count = sampling_rate_hz * duration_seconds;
    if !requested_count.is_finite() || requested_count.ceil() > 2_000_000.0 {
        return Err(SamplingRateError::TooManySamples);
    }
    let sample_count = requested_count.ceil() as usize;
    let nyquist_hz = sampling_rate_hz / 2.0;
    let retained: Vec<_> = SIGNAL_COMPONENTS
        .iter()
        .copied()
        .filter(|component| component.frequency_hz <= nyquist_hz)
        .collect();

    Ok((0..sample_count)
        .map(|index| {
            let time_seconds = index as f64 / sampling_rate_hz;
            let value = retained
                .iter()
                .map(|component| evaluate_component(*component, time_seconds))
                .sum();
            SignalSample {
                time_seconds,
                value,
            }
        })
        .collect())
}

fn evaluate_component(component: SignalComponent, time_seconds: f64) -> f64 {
    component.amplitude
        * (TAU * component.frequency_hz * time_seconds + component.phase_radians).sin()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_rate_profiles_match_analytic_information_retention() {
        let profiles = canonical_rate_profiles();
        let expected = [1.0, 1.0, 1.0, 1.0, 25.0 / 34.0, 0.0];
        for (profile, expected_retention) in profiles.iter().zip(expected) {
            assert!((profile.expected_task_band_retention - expected_retention).abs() < 1.0e-12);
        }
        assert_eq!(profiles[0].regime, SamplingRegime::NativeReference);
        assert_eq!(profiles[4].regime, SamplingRegime::TaskBandPartiallyLost);
        assert_eq!(profiles[5].regime, SamplingRegime::TaskBandLost);
    }

    #[test]
    fn rates_above_twice_the_highest_task_frequency_preserve_task_band() {
        for rate in [20.0, 8.0, 2.0, 0.8] {
            let profile = sampling_rate_profile(rate).unwrap();
            assert!(profile.nyquist_hz > TASK_BAND_HIGH_HZ);
            assert_eq!(profile.expected_task_band_retention, 1.0);
        }
    }

    #[test]
    fn crossing_one_component_nyquist_boundary_retains_only_lower_task_component() {
        let profile = sampling_rate_profile(0.28).unwrap();
        assert!((profile.nyquist_hz - 0.14).abs() < 1.0e-12);
        assert!((profile.expected_task_band_retention - 25.0 / 34.0).abs() < 1.0e-12);
        assert_eq!(profile.regime, SamplingRegime::TaskBandPartiallyLost);
    }

    #[test]
    fn sampling_below_twice_the_lowest_task_frequency_removes_task_band() {
        let profile = sampling_rate_profile(0.08).unwrap();
        assert!(profile.nyquist_hz < TASK_BAND_LOW_HZ);
        assert_eq!(profile.expected_task_band_retention, 0.0);
        assert_eq!(profile.regime, SamplingRegime::TaskBandLost);
    }

    #[test]
    fn ideal_lowpass_removes_components_above_nyquist_before_sampling() {
        let native = sample_ideal_lowpass(20.0, FIXTURE_DURATION_SECONDS).unwrap();
        let preserved = sample_ideal_lowpass(0.8, FIXTURE_DURATION_SECONDS).unwrap();
        let partial = sample_ideal_lowpass(0.28, FIXTURE_DURATION_SECONDS).unwrap();
        let lost = sample_ideal_lowpass(0.08, FIXTURE_DURATION_SECONDS).unwrap();

        assert_eq!(native.len(), 24_000);
        assert_eq!(preserved.len(), 960);
        assert_eq!(partial.len(), 336);
        assert_eq!(lost.len(), 96);

        // At 0.28 Hz the 0.15 Hz task component is above Nyquist and removed.
        let time = 0.0;
        let expected_partial = SIGNAL_COMPONENTS[0].amplitude
            * (SIGNAL_COMPONENTS[0].phase_radians).sin()
            + SIGNAL_COMPONENTS[1].amplitude * SIGNAL_COMPONENTS[1].phase_radians.sin();
        assert!((partial[0].value - expected_partial).abs() < 1.0e-12);
        assert!(native[0].value != partial[0].value);
        assert!(preserved[0].value != lost[0].value);
        assert_eq!(partial[0].time_seconds, time);
    }

    #[test]
    fn sampling_times_are_physical_time_not_token_indices() {
        let samples = sample_ideal_lowpass(0.28, 1200.0).unwrap();
        assert!((samples[1].time_seconds - samples[0].time_seconds - 1.0 / 0.28).abs() < 1.0e-12);
        assert!((samples[2].time_seconds - samples[1].time_seconds - 1.0 / 0.28).abs() < 1.0e-12);
    }

    #[test]
    fn invalid_rates_and_durations_fail_closed() {
        assert_eq!(
            task_band_retention(0.0),
            Err(SamplingRateError::InvalidSamplingRate)
        );
        assert_eq!(
            task_band_retention(f64::NAN),
            Err(SamplingRateError::InvalidSamplingRate)
        );
        assert_eq!(
            sample_ideal_lowpass(1.0, 0.0),
            Err(SamplingRateError::InvalidDuration)
        );
        assert_eq!(
            sample_ideal_lowpass(10_000.0, 1_000.0),
            Err(SamplingRateError::TooManySamples)
        );
    }
}
