//! Source-convention-aligned raw CIR window and feature extraction for Bregar UWB v2.
//!
//! The publisher reference processing converts complex CIR samples to
//! magnitude, divides magnitudes by RXPACC, truncates FP_INDEX toward zero,
//! starts five samples before that integer first-path index and extracts
//! a 152-sample window.
//!
//! This kernel implements those operations and adds transparent deterministic
//! shape/energy/delay features. It accepts no NLOS label, ground-truth range
//! or position field. It does not parse CSV/JSON, classify NLOS or claim that
//! the resulting features are physically calibrated.

use std::fmt;

pub const BREGAR_FP_OFFSET_SAMPLES: usize = 5;
pub const BREGAR_CIR_WINDOW_SAMPLES: usize = 152;
pub const BREGAR_EARLY_POSTCURSOR_SAMPLES: usize = 32;
pub const BREGAR_CIR_FEATURE_SCHEMA_VERSION: &str = "bregar-cir-features/v0.1";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ComplexCirSample {
    pub real: f64,
    pub imaginary: f64,
}

/// Input intentionally excludes evaluator labels and ground-truth values.
#[derive(Clone, Debug, PartialEq)]
pub struct BregarRawCirObservation {
    pub measurement_id: u64,
    /// Publisher FP_INDEX numeric value; fractional part is preserved.
    pub fp_index: f64,
    /// Number of preamble symbols accumulated during CIR estimation.
    pub rxpacc: f64,
    /// Independently established CIR sample spacing; not inferred from PRFR.
    pub sample_period_ns: f64,
    pub cir_samples: Vec<ComplexCirSample>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BregarCirFeatures {
    pub schema_version: &'static str,
    pub measurement_id: u64,
    pub fp_index_source: f64,
    pub fp_index_integer: usize,
    pub fp_index_fractional: f64,
    pub rxpacc: f64,
    pub sample_period_ns: f64,
    pub window_start_source_index: usize,
    pub window_end_source_index_exclusive: usize,
    /// Magnitude / RXPACC for each of the 152 publisher-aligned samples.
    pub normalized_magnitude_window: Vec<f64>,
    pub first_path_magnitude: f64,
    pub peak_magnitude: f64,
    pub peak_index_in_window: usize,
    pub peak_delay_from_first_path_ns: f64,
    pub pre_first_path_noise_rms: f64,
    /// None when the pre-first-path reference is exactly zero.
    pub peak_to_noise_db: Option<f64>,
    pub first_path_to_peak_ratio: f64,
    pub total_cir_energy: f64,
    pub pre_first_path_energy_fraction: f64,
    pub early_postcursor_energy_fraction: f64,
    pub late_energy_fraction: f64,
    /// Energy-weighted delay centroid relative to integer FP_INDEX.
    pub energy_delay_centroid_ns: f64,
    /// Energy-weighted RMS spread around the delay centroid.
    pub energy_delay_spread_ns: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BregarCirFeatureError {
    InvalidMeasurementIdentity,
    InvalidFirstPathIndex,
    FirstPathOutsideSourceCir,
    InvalidRxpacc,
    InvalidSamplePeriod,
    InvalidComplexCirSample(usize),
    IncompleteCirWindow,
    ZeroCirEnergy,
    ArithmeticOverflow,
}

impl fmt::Display for BregarCirFeatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMeasurementIdentity => write!(f, "measurement identity must be nonzero"),
            Self::InvalidFirstPathIndex => write!(f, "FP_INDEX must be finite, nonnegative and representable"),
            Self::FirstPathOutsideSourceCir => write!(f, "integer FP_INDEX is smaller than the publisher's five-sample pre-offset"),
            Self::InvalidRxpacc => write!(f, "RXPACC must be finite and strictly positive"),
            Self::InvalidSamplePeriod => write!(f, "CIR sample period must be finite and strictly positive"),
            Self::InvalidComplexCirSample(index) => write!(f, "CIR complex sample is non-finite or overflows magnitude at source index {index}"),
            Self::IncompleteCirWindow => write!(f, "source CIR does not contain the complete 152-sample first-path-aligned window"),
            Self::ZeroCirEnergy => write!(f, "the aligned CIR window has zero total energy"),
            Self::ArithmeticOverflow => write!(f, "CIR feature arithmetic produced a non-finite result"),
        }
    }
}
impl std::error::Error for BregarCirFeatureError {}

/// Extract the publisher-aligned 152-sample normalized magnitude window and
/// deterministic shape/energy features.
///
/// The source implementation applies Python int(FP_INDEX), which truncates a
/// positive value toward zero. The fractional component is retained here.
pub fn extract_bregar_cir_features(
    observation: &BregarRawCirObservation,
) -> Result<BregarCirFeatures, BregarCirFeatureError> {
    if observation.measurement_id == 0 {
        return Err(BregarCirFeatureError::InvalidMeasurementIdentity);
    }
    if !observation.fp_index.is_finite()
        || observation.fp_index < 0.0
        || observation.fp_index >= usize::MAX as f64
    {
        return Err(BregarCirFeatureError::InvalidFirstPathIndex);
    }
    if !observation.rxpacc.is_finite() || observation.rxpacc <= 0.0 {
        return Err(BregarCirFeatureError::InvalidRxpacc);
    }
    if !observation.sample_period_ns.is_finite() || observation.sample_period_ns <= 0.0 {
        return Err(BregarCirFeatureError::InvalidSamplePeriod);
    }

    let fp_index_integer = observation.fp_index.trunc() as usize;
    let fp_index_fractional = observation.fp_index - fp_index_integer as f64;
    let window_start = fp_index_integer
        .checked_sub(BREGAR_FP_OFFSET_SAMPLES)
        .ok_or(BregarCirFeatureError::FirstPathOutsideSourceCir)?;
    let window_end = window_start
        .checked_add(BREGAR_CIR_WINDOW_SAMPLES)
        .ok_or(BregarCirFeatureError::ArithmeticOverflow)?;
    if window_end > observation.cir_samples.len() {
        return Err(BregarCirFeatureError::IncompleteCirWindow);
    }

    let mut magnitudes = Vec::with_capacity(BREGAR_CIR_WINDOW_SAMPLES);
    let mut energies = Vec::with_capacity(BREGAR_CIR_WINDOW_SAMPLES);
    for (offset, sample) in observation.cir_samples[window_start..window_end]
        .iter()
        .enumerate()
    {
        if !sample.real.is_finite() || !sample.imaginary.is_finite() {
            return Err(BregarCirFeatureError::InvalidComplexCirSample(
                window_start + offset,
            ));
        }
        let magnitude = sample.real.hypot(sample.imaginary) / observation.rxpacc;
        let energy = magnitude * magnitude;
        if !magnitude.is_finite() || !energy.is_finite() {
            return Err(BregarCirFeatureError::InvalidComplexCirSample(
                window_start + offset,
            ));
        }
        magnitudes.push(magnitude);
        energies.push(energy);
    }

    let total_energy = checked_sum(&energies)?;
    if total_energy <= 0.0 {
        return Err(BregarCirFeatureError::ZeroCirEnergy);
    }

    let first_path_offset = BREGAR_FP_OFFSET_SAMPLES;
    let first_path_magnitude = magnitudes[first_path_offset];
    let (peak_index_in_window, peak_magnitude) = magnitudes
        .iter()
        .copied()
        .enumerate()
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .ok_or(BregarCirFeatureError::IncompleteCirWindow)?;

    let pre_first_path_energy = checked_sum(&energies[..first_path_offset])?;
    let pre_first_path_noise_rms =
        (pre_first_path_energy / BREGAR_FP_OFFSET_SAMPLES as f64).sqrt();

    let early_postcursor_end = (first_path_offset + BREGAR_EARLY_POSTCURSOR_SAMPLES)
        .min(BREGAR_CIR_WINDOW_SAMPLES);
    let early_postcursor_energy =
        checked_sum(&energies[first_path_offset + 1..early_postcursor_end])?;
    let late_energy = checked_sum(&energies[early_postcursor_end..])?;

    let peak_to_noise_db = if pre_first_path_noise_rms > 0.0 {
        let db = 20.0
            * (peak_magnitude.ln() - pre_first_path_noise_rms.ln())
            / 10.0_f64.ln();
        if !db.is_finite() {
            return Err(BregarCirFeatureError::ArithmeticOverflow);
        }
        Some(db)
    } else {
        None
    };

    let first_path_to_peak_ratio = first_path_magnitude / peak_magnitude;

    let mut weighted_delay_samples = 0.0;
    for (index, energy) in energies.iter().copied().enumerate() {
        weighted_delay_samples += (index as f64 - first_path_offset as f64) * energy;
    }
    let delay_centroid_samples = weighted_delay_samples / total_energy;
    let mut weighted_squared_spread = 0.0;
    for (index, energy) in energies.iter().copied().enumerate() {
        let delta = index as f64 - first_path_offset as f64 - delay_centroid_samples;
        weighted_squared_spread += delta * delta * energy;
    }
    let delay_spread_samples = (weighted_squared_spread / total_energy).sqrt();

    let sample_period_ns = observation.sample_period_ns;
    let peak_delay_from_first_path_ns =
        (peak_index_in_window as f64 - first_path_offset as f64) * sample_period_ns;
    let energy_delay_centroid_ns = delay_centroid_samples * sample_period_ns;
    let energy_delay_spread_ns = delay_spread_samples * sample_period_ns;

    let output_values = [
        first_path_magnitude,
        peak_magnitude,
        peak_delay_from_first_path_ns,
        pre_first_path_noise_rms,
        total_energy,
        pre_first_path_energy / total_energy,
        energies[first_path_offset] / total_energy,
        early_postcursor_energy / total_energy,
        late_energy / total_energy,
        energy_delay_centroid_ns,
        energy_delay_spread_ns,
        first_path_to_peak_ratio,
    ];
    if !output_values.iter().all(|value| value.is_finite()) {
        return Err(BregarCirFeatureError::ArithmeticOverflow);
    }

    Ok(BregarCirFeatures {
        schema_version: BREGAR_CIR_FEATURE_SCHEMA_VERSION,
        measurement_id: observation.measurement_id,
        fp_index_source: observation.fp_index,
        fp_index_integer,
        fp_index_fractional,
        rxpacc: observation.rxpacc,
        sample_period_ns,
        window_start_source_index: window_start,
        window_end_source_index_exclusive: window_end,
        normalized_magnitude_window: magnitudes,
        first_path_magnitude,
        peak_magnitude,
        peak_index_in_window,
        peak_delay_from_first_path_ns,
        pre_first_path_noise_rms,
        peak_to_noise_db,
        first_path_to_peak_ratio,
        total_cir_energy: total_energy,
        pre_first_path_energy_fraction: pre_first_path_energy / total_energy,
        early_postcursor_energy_fraction: early_postcursor_energy / total_energy,
        late_energy_fraction: late_energy / total_energy,
        energy_delay_centroid_ns,
        energy_delay_spread_ns,
    })
}

fn checked_sum(values: &[f64]) -> Result<f64, BregarCirFeatureError> {
    let mut sum = 0.0;
    for value in values {
        sum += value;
        if !sum.is_finite() {
            return Err(BregarCirFeatureError::ArithmeticOverflow);
        }
    }
    Ok(sum)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(fp_index: f64, rxpacc: f64) -> BregarRawCirObservation {
        BregarRawCirObservation {
            measurement_id: 7,
            fp_index,
            rxpacc,
            sample_period_ns: 1.0,
            cir_samples: vec![
                ComplexCirSample {
                    real: 0.0,
                    imaginary: 0.0,
                };
                260
            ],
        }
    }

    #[test]
    fn aligns_five_samples_before_integer_first_path_and_extracts_152_samples() {
        let mut input = observation(100.75, 2.0);
        input.cir_samples[95] = ComplexCirSample { real: 2.0, imaginary: 0.0 };
        input.cir_samples[100] = ComplexCirSample { real: 4.0, imaginary: 0.0 };
        input.cir_samples[110] = ComplexCirSample { real: 8.0, imaginary: 0.0 };

        let features = extract_bregar_cir_features(&input).unwrap();
        assert_eq!(features.window_start_source_index, 95);
        assert_eq!(features.window_end_source_index_exclusive, 247);
        assert_eq!(features.fp_index_integer, 100);
        assert!((features.fp_index_fractional - 0.75).abs() < 1.0e-12);
        assert_eq!(features.normalized_magnitude_window.len(), 152);
        assert!((features.first_path_magnitude - 2.0).abs() < 1.0e-12);
        assert!((features.peak_magnitude - 4.0).abs() < 1.0e-12);
        assert_eq!(features.peak_index_in_window, 15);
        assert!((features.peak_delay_from_first_path_ns - 10.0).abs() < 1.0e-12);
    }

    #[test]
    fn normalization_by_rxpacc_scales_magnitude_and_energy_but_not_ratios() {
        let mut first = observation(100.0, 2.0);
        let mut second = observation(100.0, 4.0);
        for index in 95..247 {
            let sample = ComplexCirSample {
                real: (index - 94) as f64,
                imaginary: 0.5,
            };
            first.cir_samples[index] = sample;
            second.cir_samples[index] = sample;
        }
        let a = extract_bregar_cir_features(&first).unwrap();
        let b = extract_bregar_cir_features(&second).unwrap();
        assert!((a.peak_magnitude - 2.0 * b.peak_magnitude).abs() < 1.0e-12);
        assert!((a.total_cir_energy - 4.0 * b.total_cir_energy).abs() < 1.0e-10);
        assert!((a.first_path_to_peak_ratio - b.first_path_to_peak_ratio).abs() < 1.0e-12);
    }

    #[test]
    fn preserves_publisher_truncation_semantics_for_fractional_fp_index() {
        let integer = extract_bregar_cir_features(&observation(100.0, 1.0)).unwrap();
        let fractional = extract_bregar_cir_features(&observation(100.875, 1.0)).unwrap();
        assert_eq!(integer.window_start_source_index, fractional.window_start_source_index);
        assert_eq!(fractional.fp_index_integer, 100);
        assert!((fractional.fp_index_fractional - 0.875).abs() < 1.0e-12);
    }

    #[test]
    fn rejects_fp_index_below_five_sample_pre_offset() {
        let input = observation(4.999, 1.0);
        assert_eq!(
            extract_bregar_cir_features(&input),
            Err(BregarCirFeatureError::FirstPathOutsideSourceCir)
        );
    }

    #[test]
    fn rejects_incomplete_aligned_cir_window() {
        let mut input = observation(200.0, 1.0);
        input.cir_samples.truncate(220);
        assert_eq!(
            extract_bregar_cir_features(&input),
            Err(BregarCirFeatureError::IncompleteCirWindow)
        );
    }

    #[test]
    fn rejects_invalid_rxpacc_sample_period_and_measurement_id() {
        let mut input = observation(100.0, 0.0);
        assert_eq!(
            extract_bregar_cir_features(&input),
            Err(BregarCirFeatureError::InvalidRxpacc)
        );

        let mut input = observation(100.0, 1.0);
        input.sample_period_ns = 0.0;
        assert_eq!(
            extract_bregar_cir_features(&input),
            Err(BregarCirFeatureError::InvalidSamplePeriod)
        );

        let mut input = observation(100.0, 1.0);
        input.measurement_id = 0;
        assert_eq!(
            extract_bregar_cir_features(&input),
            Err(BregarCirFeatureError::InvalidMeasurementIdentity)
        );
    }

    #[test]
    fn rejects_non_finite_fp_index_and_complex_samples() {
        let mut input = observation(f64::NAN, 1.0);
        assert_eq!(
            extract_bregar_cir_features(&input),
            Err(BregarCirFeatureError::InvalidFirstPathIndex)
        );

        let mut input = observation(100.0, 1.0);
        input.cir_samples[100].imaginary = f64::INFINITY;
        assert_eq!(
            extract_bregar_cir_features(&input),
            Err(BregarCirFeatureError::InvalidComplexCirSample(100))
        );
    }

    #[test]
    fn rejects_zero_energy_cir() {
        assert_eq!(
            extract_bregar_cir_features(&observation(100.0, 1.0)),
            Err(BregarCirFeatureError::ZeroCirEnergy)
        );
    }

    #[test]
    fn zero_prepath_noise_is_reported_as_unavailable_not_infinite() {
        let mut input = observation(100.0, 1.0);
        input.cir_samples[100] = ComplexCirSample { real: 2.0, imaginary: 0.0 };
        input.cir_samples[110] = ComplexCirSample { real: 4.0, imaginary: 0.0 };
        let features = extract_bregar_cir_features(&input).unwrap();
        assert_eq!(features.peak_to_noise_db, None);
        assert_eq!(features.pre_first_path_energy_fraction, 0.0);
    }

    #[test]
    fn energy_fractions_partition_the_window_without_double_counting() {
        let mut input = observation(100.0, 1.0);
        for index in 95..247 {
            input.cir_samples[index] = ComplexCirSample {
                real: (index % 13) as f64 + 1.0,
                imaginary: 0.25,
            };
        }
        let features = extract_bregar_cir_features(&input).unwrap();
        let sum = features.pre_first_path_energy_fraction
            + features.first_path_magnitude.powi(2) / features.total_cir_energy
            + features.early_postcursor_energy_fraction
            + features.late_energy_fraction;
        assert!((sum - 1.0).abs() < 1.0e-10);
    }
}
