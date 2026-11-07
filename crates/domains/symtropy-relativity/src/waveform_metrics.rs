// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Noise-weighted frequency-domain waveform comparison.
//!
//! This module compares already time-aligned complex strains on an identical
//! uniform frequency grid with a caller-supplied one-sided PSD. It does not
//! window, whiten, interpolate, resample, or maximize over time.
//!
//! Discrete convention:
//! (a|b) = 4 * df * sum_k conjugate(a_k) * b_k / S_n(f_k).
//!
//! A caller must pin the time-alignment convention, PSD, frequency band,
//! waveform mode, total-mass scaling, and reference waveform resolution.

use core::fmt;

/// A complex strain sample in one frequency bin.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ComplexStrain {
    pub re: f64,
    pub im: f64,
}

impl ComplexStrain {
    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    pub fn amplitude(self) -> f64 {
        self.re.hypot(self.im)
    }

    pub fn phase_rad(self) -> f64 {
        self.im.atan2(self.re)
    }

    fn is_finite(self) -> bool {
        self.re.is_finite() && self.im.is_finite()
    }
}

/// Frequency-domain strain on a strictly increasing uniform frequency grid.
#[derive(Debug, Clone, Copy)]
pub struct FrequencyDomainStrain<'a> {
    pub frequencies_hz: &'a [f64],
    pub samples: &'a [ComplexStrain],
}

/// One-sided noise power spectral density on a frequency grid.
#[derive(Debug, Clone, Copy)]
pub struct OneSidedPsd<'a> {
    pub frequencies_hz: &'a [f64],
    pub values_strain2_per_hz: &'a [f64],
}

/// Explicit inclusive frequency band and phase-diagnostic amplitude floor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaveformComparisonBand {
    pub minimum_hz: f64,
    pub maximum_hz: f64,
    /// Bins with reference amplitude <= this fraction of the in-band peak
    /// are omitted from phase diagnostics, but not from the inner product.
    pub reference_phase_floor_fraction: f64,
}

/// Per-bin diagnostic after removal of the best constant phase offset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrequencyBinError {
    pub frequency_hz: f64,
    /// Candidate/reference amplitude ratio minus one, when defined.
    pub relative_amplitude_error: Option<f64>,
    /// Wrapped candidate-reference phase residual after constant-phase
    /// alignment, in [-pi, pi], when both amplitudes exceed the floor.
    pub phase_residual_rad: Option<f64>,
}

/// Summary of noise-weighted waveform comparison.
#[derive(Debug, Clone, PartialEq)]
pub struct WaveformComparison {
    /// Real normalized inner product before constant-phase maximization.
    pub real_overlap_before_phase_maximization: f64,
    /// Magnitude of the normalized complex inner product. Constant phase is
    /// maximized analytically; time shift is not maximized.
    pub phase_maximized_overlap: f64,
    pub phase_maximized_mismatch: f64,
    /// Argument of conjugate(reference) * candidate, in radians. Apply its
    /// negative to the candidate to align a single constant phase.
    pub best_constant_phase_offset_rad: f64,
    /// sqrt[sum w*(|candidate|-|reference|)^2 / sum w*|reference|^2].
    pub relative_amplitude_l2_error: f64,
    /// Reference-amplitude/PSD-weighted RMS phase residual after removing
    /// the reported constant phase offset. None when no bins pass the floor.
    pub weighted_phase_residual_rms_rad: Option<f64>,
    pub phase_diagnostic_bins: usize,
    pub bins_in_band: usize,
    pub frequency_spacing_hz: f64,
    pub frequency_bin_errors: Vec<FrequencyBinError>,
}

/// Errors in waveform/PSD validation or metric calculation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WaveformMetricError {
    TooFewFrequencyBins,
    StrainLengthMismatch,
    PsdLengthMismatch,
    NonFiniteFrequency { index: usize },
    NonIncreasingFrequencyGrid { index: usize },
    NonUniformFrequencyGrid { index: usize },
    FrequencyGridMismatch { index: usize },
    NonFiniteStrain { index: usize },
    NonFinitePsd { index: usize },
    NonPositivePsd { index: usize },
    InvalidFrequencyBand,
    InvalidPhaseFloor,
    FewerThanTwoBinsInBand,
    ZeroReferenceNorm,
    ZeroCandidateNorm,
    NonFiniteAccumulation(&'static str),
    OverlapOutsideUnitInterval(f64),
}

impl fmt::Display for WaveformMetricError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewFrequencyBins => write!(f, "at least two frequency bins are required"),
            Self::StrainLengthMismatch => {
                write!(f, "reference and candidate strain lengths differ")
            }
            Self::PsdLengthMismatch => write!(f, "PSD length differs from strain grid length"),
            Self::NonFiniteFrequency { index } => {
                write!(f, "frequency at index {index} is non-finite")
            }
            Self::NonIncreasingFrequencyGrid { index } => write!(
                f,
                "frequency grid is not strictly increasing at index {index}"
            ),
            Self::NonUniformFrequencyGrid { index } => {
                write!(f, "frequency grid is not uniform at interval {index}")
            }
            Self::FrequencyGridMismatch { index } => {
                write!(f, "frequency grids differ at index {index}")
            }
            Self::NonFiniteStrain { index } => {
                write!(f, "strain sample at index {index} is non-finite")
            }
            Self::NonFinitePsd { index } => write!(f, "PSD at index {index} is non-finite"),
            Self::NonPositivePsd { index } => write!(f, "PSD at index {index} must be positive"),
            Self::InvalidFrequencyBand => write!(
                f,
                "frequency band must be finite, nonnegative, and increasing"
            ),
            Self::InvalidPhaseFloor => {
                write!(f, "phase floor fraction must be finite and in [0,1)")
            }
            Self::FewerThanTwoBinsInBand => {
                write!(f, "frequency band must include at least two bins")
            }
            Self::ZeroReferenceNorm => write!(f, "reference waveform has zero in-band norm"),
            Self::ZeroCandidateNorm => write!(f, "candidate waveform has zero in-band norm"),
            Self::NonFiniteAccumulation(name) => write!(f, "{name} became non-finite"),
            Self::OverlapOutsideUnitInterval(value) => write!(
                f,
                "normalized overlap {value} violates the Cauchy-Schwarz bound"
            ),
        }
    }
}

impl std::error::Error for WaveformMetricError {}

#[derive(Debug, Clone, Copy, Default)]
struct CompensatedSum {
    sum: f64,
    correction: f64,
}

impl CompensatedSum {
    fn add(&mut self, value: f64) {
        let next = self.sum + value;
        if self.sum.abs() >= value.abs() {
            self.correction += (self.sum - next) + value;
        } else {
            self.correction += (value - next) + self.sum;
        }
        self.sum = next;
    }

    fn total(self) -> f64 {
        self.sum + self.correction
    }
}

fn close_grid_value(a: f64, b: f64) -> bool {
    (a - b).abs() <= 16.0 * f64::EPSILON * a.abs().max(b.abs()).max(1.0)
}

fn wrap_phase(value: f64) -> f64 {
    value.sin().atan2(value.cos())
}

/// Compare aligned frequency-domain strains using a one-sided PSD.
///
/// The reference, candidate and PSD must use the same uniform frequency grid.
/// The PSD must be positive and finite. The caller must time-align the strains
/// using an externally declared convention. Only a constant phase is
/// maximized analytically. No time shift, interpolation, resampling, windowing
/// or PSD estimation is performed.
pub fn compare_frequency_domain(
    reference: FrequencyDomainStrain<'_>,
    candidate: FrequencyDomainStrain<'_>,
    psd: OneSidedPsd<'_>,
    band: WaveformComparisonBand,
) -> Result<WaveformComparison, WaveformMetricError> {
    let frequencies = reference.frequencies_hz;
    let candidate_frequencies = candidate.frequencies_hz;
    let reference_samples = reference.samples;
    let candidate_samples = candidate.samples;
    let psd_frequencies = psd.frequencies_hz;
    let psd_values = psd.values_strain2_per_hz;

    let len = frequencies.len();
    if len < 2 {
        return Err(WaveformMetricError::TooFewFrequencyBins);
    }
    if reference_samples.len() != len
        || candidate_samples.len() != len
        || candidate_frequencies.len() != len
    {
        return Err(WaveformMetricError::StrainLengthMismatch);
    }
    if psd_frequencies.len() != len || psd_values.len() != len {
        return Err(WaveformMetricError::PsdLengthMismatch);
    }
    if !band.minimum_hz.is_finite()
        || !band.maximum_hz.is_finite()
        || band.minimum_hz < 0.0
        || band.maximum_hz <= band.minimum_hz
    {
        return Err(WaveformMetricError::InvalidFrequencyBand);
    }
    if !band.reference_phase_floor_fraction.is_finite()
        || !(0.0..1.0).contains(&band.reference_phase_floor_fraction)
    {
        return Err(WaveformMetricError::InvalidPhaseFloor);
    }

    for index in 0..len {
        let frequency = frequencies[index];
        if !frequency.is_finite()
            || !candidate_frequencies[index].is_finite()
            || !psd_frequencies[index].is_finite()
        {
            return Err(WaveformMetricError::NonFiniteFrequency { index });
        }
        if !close_grid_value(frequency, candidate_frequencies[index])
            || !close_grid_value(frequency, psd_frequencies[index])
        {
            return Err(WaveformMetricError::FrequencyGridMismatch { index });
        }
        if index > 0 && frequency <= frequencies[index - 1] {
            return Err(WaveformMetricError::NonIncreasingFrequencyGrid { index });
        }
        if !reference_samples[index].is_finite() || !candidate_samples[index].is_finite() {
            return Err(WaveformMetricError::NonFiniteStrain { index });
        }
        if !psd_values[index].is_finite() {
            return Err(WaveformMetricError::NonFinitePsd { index });
        }
        if psd_values[index] <= 0.0 {
            return Err(WaveformMetricError::NonPositivePsd { index });
        }
    }

    let delta_f = frequencies[1] - frequencies[0];
    if !delta_f.is_finite() || delta_f <= 0.0 {
        return Err(WaveformMetricError::NonIncreasingFrequencyGrid { index: 1 });
    }
    for index in 1..(len - 1) {
        let interval = frequencies[index + 1] - frequencies[index];
        if !close_grid_value(interval, delta_f) {
            return Err(WaveformMetricError::NonUniformFrequencyGrid { index });
        }
    }

    let mut selected = Vec::new();
    let mut maximum_reference_amplitude: f64 = 0.0;
    for index in 0..len {
        let frequency = frequencies[index];
        if frequency >= band.minimum_hz && frequency <= band.maximum_hz {
            let reference_amplitude = reference_samples[index].amplitude();
            let candidate_amplitude = candidate_samples[index].amplitude();
            if !reference_amplitude.is_finite() || !candidate_amplitude.is_finite() {
                return Err(WaveformMetricError::NonFiniteAccumulation(
                    "waveform amplitude",
                ));
            }
            maximum_reference_amplitude = maximum_reference_amplitude.max(reference_amplitude);
            selected.push(index);
        }
    }
    if selected.len() < 2 {
        return Err(WaveformMetricError::FewerThanTwoBinsInBand);
    }

    let phase_floor = maximum_reference_amplitude * band.reference_phase_floor_fraction;

    let mut reference_norm = CompensatedSum::default();
    let mut candidate_norm = CompensatedSum::default();
    let mut inner_real = CompensatedSum::default();
    let mut inner_imag = CompensatedSum::default();
    let mut amplitude_error_numerator = CompensatedSum::default();
    let mut amplitude_error_denominator = CompensatedSum::default();

    for &index in &selected {
        let a = reference_samples[index];
        let b = candidate_samples[index];
        let noise = psd_values[index];
        let weight = 4.0 * delta_f / noise;

        let a2 = a.re * a.re + a.im * a.im;
        let b2 = b.re * b.re + b.im * b.im;
        let real = a.re * b.re + a.im * b.im;
        let imaginary = a.re * b.im - a.im * b.re;

        reference_norm.add(weight * a2);
        candidate_norm.add(weight * b2);
        inner_real.add(weight * real);
        inner_imag.add(weight * imaginary);

        let amplitude_difference = b.amplitude() - a.amplitude();
        amplitude_error_numerator.add(weight * amplitude_difference * amplitude_difference);
        amplitude_error_denominator.add(weight * a2);
    }

    let norm_a = reference_norm.total();
    let norm_b = candidate_norm.total();
    let inner_re = inner_real.total();
    let inner_im = inner_imag.total();
    let amplitude_numerator = amplitude_error_numerator.total();
    let amplitude_denominator = amplitude_error_denominator.total();

    if !norm_a.is_finite()
        || !norm_b.is_finite()
        || !inner_re.is_finite()
        || !inner_im.is_finite()
        || !amplitude_numerator.is_finite()
        || !amplitude_denominator.is_finite()
    {
        return Err(WaveformMetricError::NonFiniteAccumulation(
            "waveform inner product",
        ));
    }
    if norm_a <= 0.0 || amplitude_denominator <= 0.0 {
        return Err(WaveformMetricError::ZeroReferenceNorm);
    }
    if norm_b <= 0.0 {
        return Err(WaveformMetricError::ZeroCandidateNorm);
    }

    let normalization = norm_a.sqrt() * norm_b.sqrt();
    if !normalization.is_finite() || normalization <= 0.0 {
        return Err(WaveformMetricError::NonFiniteAccumulation(
            "waveform normalization",
        ));
    }
    let real_overlap = inner_re / normalization;
    let raw_complex_overlap = inner_re.hypot(inner_im) / normalization;
    if !real_overlap.is_finite() || !raw_complex_overlap.is_finite() {
        return Err(WaveformMetricError::NonFiniteAccumulation(
            "normalized overlap",
        ));
    }

    // The exact Cauchy-Schwarz bound is <= 1. Permit only a tiny f64
    // roundoff excursion; do not silently clip a materially invalid result.
    if raw_complex_overlap > 1.0 + 128.0 * f64::EPSILON {
        return Err(WaveformMetricError::OverlapOutsideUnitInterval(
            raw_complex_overlap,
        ));
    }
    let overlap = raw_complex_overlap.min(1.0);
    let best_phase = inner_im.atan2(inner_re);
    let amplitude_l2 = (amplitude_numerator / amplitude_denominator).sqrt();
    if !amplitude_l2.is_finite() {
        return Err(WaveformMetricError::NonFiniteAccumulation(
            "relative amplitude error",
        ));
    }

    let mut phase_numerator = CompensatedSum::default();
    let mut phase_denominator = CompensatedSum::default();
    let mut phase_count = 0usize;
    let mut bin_errors = Vec::with_capacity(selected.len());

    for &index in &selected {
        let a = reference_samples[index];
        let b = candidate_samples[index];
        let a_amp = a.amplitude();
        let b_amp = b.amplitude();
        let relative_amplitude_error = if a_amp > phase_floor && a_amp > 0.0 {
            Some(b_amp / a_amp - 1.0)
        } else {
            None
        };

        let phase_residual = if a_amp > phase_floor && a_amp > 0.0 && b_amp > 0.0 {
            let residual = wrap_phase(b.phase_rad() - a.phase_rad() - best_phase);
            let phase_weight = (4.0 * delta_f / psd_values[index]) * a_amp * a_amp;
            phase_numerator.add(phase_weight * residual * residual);
            phase_denominator.add(phase_weight);
            phase_count += 1;
            Some(residual)
        } else {
            None
        };

        bin_errors.push(FrequencyBinError {
            frequency_hz: frequencies[index],
            relative_amplitude_error,
            phase_residual_rad: phase_residual,
        });
    }

    let phase_denominator_value = phase_denominator.total();
    let phase_numerator_value = phase_numerator.total();
    if !phase_denominator_value.is_finite() || !phase_numerator_value.is_finite() {
        return Err(WaveformMetricError::NonFiniteAccumulation(
            "phase residual accumulation",
        ));
    }
    let phase_rms = if phase_count > 0 && phase_denominator_value > 0.0 {
        Some((phase_numerator_value / phase_denominator_value).sqrt())
    } else {
        None
    };
    if phase_rms.is_some_and(|value| !value.is_finite()) {
        return Err(WaveformMetricError::NonFiniteAccumulation(
            "phase residual RMS",
        ));
    }

    Ok(WaveformComparison {
        real_overlap_before_phase_maximization: real_overlap,
        phase_maximized_overlap: overlap,
        phase_maximized_mismatch: 1.0 - overlap,
        best_constant_phase_offset_rad: best_phase,
        relative_amplitude_l2_error: amplitude_l2,
        weighted_phase_residual_rms_rad: phase_rms,
        phase_diagnostic_bins: phase_count,
        bins_in_band: selected.len(),
        frequency_spacing_hz: delta_f,
        frequency_bin_errors: bin_errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FREQUENCIES: [f64; 5] = [20.0, 30.0, 40.0, 50.0, 60.0];
    const PSD: [f64; 5] = [2.0, 2.0, 2.0, 2.0, 2.0];

    fn series<'a>(values: &'a [ComplexStrain; 5]) -> FrequencyDomainStrain<'a> {
        FrequencyDomainStrain {
            frequencies_hz: &FREQUENCIES,
            samples: values,
        }
    }

    fn psd<'a>(values: &'a [f64; 5]) -> OneSidedPsd<'a> {
        OneSidedPsd {
            frequencies_hz: &FREQUENCIES,
            values_strain2_per_hz: values,
        }
    }

    fn base() -> [ComplexStrain; 5] {
        [
            ComplexStrain::new(1.0, 0.2),
            ComplexStrain::new(0.8, -0.1),
            ComplexStrain::new(0.6, 0.3),
            ComplexStrain::new(0.4, -0.2),
            ComplexStrain::new(0.2, 0.1),
        ]
    }

    fn band() -> WaveformComparisonBand {
        WaveformComparisonBand {
            minimum_hz: 20.0,
            maximum_hz: 60.0,
            reference_phase_floor_fraction: 1e-10,
        }
    }

    #[test]
    fn identical_waveforms_have_zero_mismatch_amplitude_and_phase_error() {
        let values = base();
        let result =
            compare_frequency_domain(series(&values), series(&values), psd(&PSD), band()).unwrap();
        assert!(result.phase_maximized_mismatch < 1e-14);
        assert!(result.relative_amplitude_l2_error < 1e-14);
        assert!(result.weighted_phase_residual_rms_rad.unwrap() < 1e-14);
        assert_eq!(result.bins_in_band, 5);
    }

    #[test]
    fn constant_phase_rotation_is_reported_and_phase_maximized() {
        let values = base();
        let phase: f64 = 0.73;
        let rotated = values.map(|value| {
            ComplexStrain::new(
                value.re * phase.cos() - value.im * phase.sin(),
                value.re * phase.sin() + value.im * phase.cos(),
            )
        });
        let result =
            compare_frequency_domain(series(&values), series(&rotated), psd(&PSD), band()).unwrap();
        assert!(result.phase_maximized_mismatch < 1e-14);
        assert!((result.best_constant_phase_offset_rad - phase).abs() < 1e-14);
        assert!(result.weighted_phase_residual_rms_rad.unwrap() < 1e-14);
    }

    #[test]
    fn amplitude_rescaling_is_not_hidden_by_normalized_overlap() {
        let values = base();
        let scaled = values.map(|value| ComplexStrain::new(1.25 * value.re, 1.25 * value.im));
        let result =
            compare_frequency_domain(series(&values), series(&scaled), psd(&PSD), band()).unwrap();
        assert!(result.phase_maximized_mismatch < 1e-14);
        assert!((result.relative_amplitude_l2_error - 0.25).abs() < 1e-14);
    }

    #[test]
    fn frequency_dependent_phase_error_survives_constant_phase_alignment() {
        let values = base();
        let shifted = values.map(|value| {
            let phase = 0.015 * (value.re * 100.0 + value.im * 20.0);
            ComplexStrain::new(
                value.re * phase.cos() - value.im * phase.sin(),
                value.re * phase.sin() + value.im * phase.cos(),
            )
        });
        let result =
            compare_frequency_domain(series(&values), series(&shifted), psd(&PSD), band()).unwrap();
        assert!(result.phase_maximized_mismatch > 1e-8);
        assert!(result.weighted_phase_residual_rms_rad.unwrap() > 1e-5);
    }

    #[test]
    fn psd_weighting_changes_mismatch_when_error_is_frequency_localized() {
        let values = base();
        let perturbed = [
            values[0],
            values[1],
            values[2],
            ComplexStrain::new(-values[3].re, -values[3].im),
            values[4],
        ];
        let low_noise_at_50hz = [2.0, 2.0, 2.0, 0.1, 2.0];
        let high_noise_at_50hz = [2.0, 2.0, 2.0, 20.0, 2.0];
        let a = compare_frequency_domain(
            series(&values),
            series(&perturbed),
            psd(&low_noise_at_50hz),
            band(),
        )
        .unwrap();
        let b = compare_frequency_domain(
            series(&values),
            series(&perturbed),
            psd(&high_noise_at_50hz),
            band(),
        )
        .unwrap();
        assert!(a.phase_maximized_mismatch > b.phase_maximized_mismatch);
    }

    #[test]
    fn mismatched_frequency_grids_are_rejected() {
        static OTHER: [f64; 5] = [20.0, 30.0, 40.1, 50.0, 60.0];
        let values = base();
        let candidate = FrequencyDomainStrain {
            frequencies_hz: &OTHER,
            samples: &values,
        };
        assert!(matches!(
            compare_frequency_domain(series(&values), candidate, psd(&PSD), band()),
            Err(WaveformMetricError::FrequencyGridMismatch { index: 2 })
        ));
    }

    #[test]
    fn nonuniform_frequency_grid_is_rejected() {
        static NONUNIFORM: [f64; 5] = [20.0, 30.0, 40.0, 51.0, 60.0];
        let values = base();
        let reference = FrequencyDomainStrain {
            frequencies_hz: &NONUNIFORM,
            samples: &values,
        };
        let candidate = FrequencyDomainStrain {
            frequencies_hz: &NONUNIFORM,
            samples: &values,
        };
        let noise = OneSidedPsd {
            frequencies_hz: &NONUNIFORM,
            values_strain2_per_hz: &PSD,
        };
        assert!(matches!(
            compare_frequency_domain(reference, candidate, noise, band()),
            Err(WaveformMetricError::NonUniformFrequencyGrid { .. })
        ));
    }

    #[test]
    fn nonpositive_psd_is_rejected() {
        static INVALID_PSD: [f64; 5] = [2.0, 2.0, 0.0, 2.0, 2.0];
        let values = base();
        assert!(matches!(
            compare_frequency_domain(series(&values), series(&values), psd(&INVALID_PSD), band()),
            Err(WaveformMetricError::NonPositivePsd { index: 2 })
        ));
    }

    #[test]
    fn zero_reference_norm_is_rejected() {
        let zero = [ComplexStrain::new(0.0, 0.0); 5];
        let values = base();
        assert!(matches!(
            compare_frequency_domain(series(&zero), series(&values), psd(&PSD), band()),
            Err(WaveformMetricError::ZeroReferenceNorm)
        ));
    }

    #[test]
    fn band_must_include_at_least_two_bins() {
        let values = base();
        let narrow = WaveformComparisonBand {
            minimum_hz: 39.9,
            maximum_hz: 40.1,
            reference_phase_floor_fraction: 0.0,
        };
        assert!(matches!(
            compare_frequency_domain(series(&values), series(&values), psd(&PSD), narrow),
            Err(WaveformMetricError::FewerThanTwoBinsInBand)
        ));
    }
}
