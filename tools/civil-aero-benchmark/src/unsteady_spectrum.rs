//! Deterministic Welch one-sided power spectral density and stationarity diagnostics.
//!
//! This module is a signal-processing component for unsteady CFD/experimental traces. It does
//! not establish sensor calibration, correct physical boundary conditions, or CFD validity.
//! The caller must disclose sampling interval, segment length, overlap and the preprocessing
//! convention. The initial implementation uses per-segment mean removal and a periodic Hann
//! window with standard one-sided PSD normalization.

use std::f64::consts::PI;

#[derive(Debug, Clone, PartialEq)]
pub struct WelchConfig {
    pub sample_interval_seconds: f64,
    pub segment_length_samples: usize,
    pub overlap_samples: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PsdEstimate {
    pub sample_interval_seconds: f64,
    pub sampling_rate_hz: f64,
    pub segment_length_samples: usize,
    pub overlap_samples: usize,
    pub segment_stride_samples: usize,
    pub segment_count: usize,
    pub window: &'static str,
    pub detrending: &'static str,
    pub frequencies_hz: Vec<f64>,
    pub psd_per_hz: Vec<f64>,
    pub frequency_bin_width_hz: f64,
    pub nyquist_frequency_hz: f64,
    pub integrated_mean_square: f64,
    pub integrated_rms: f64,
    pub time_domain_rms: f64,
    pub rms_relative_difference: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StationarityReport {
    pub window_count: usize,
    pub samples_per_window: usize,
    pub mean_min: f64,
    pub mean_max: f64,
    pub mean_drift_absolute: f64,
    pub rms_min: f64,
    pub rms_max: f64,
    pub rms_drift_fraction: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpectralComparison {
    pub reference_peak_frequency_hz: f64,
    pub candidate_peak_frequency_hz: f64,
    pub dominant_frequency_error_hz: f64,
    pub reference_integrated_mean_square: f64,
    pub candidate_integrated_mean_square: f64,
    pub integrated_mean_square_relative_error: f64,
    /// Half the L1 distance between unit-integral one-sided PSDs; dimensionless.
    pub normalized_spectral_shape_distance: f64,
}

pub fn welch_one_sided_psd(samples: &[f64], config: &WelchConfig) -> Result<PsdEstimate, String> {
    if !config.sample_interval_seconds.is_finite() || config.sample_interval_seconds <= 0.0 {
        return Err("sample_interval_seconds must be finite and > 0".to_owned());
    }
    if config.segment_length_samples < 8 || !config.segment_length_samples.is_power_of_two() {
        return Err("segment_length_samples must be a power of two >= 8".to_owned());
    }
    if config.overlap_samples >= config.segment_length_samples {
        return Err("overlap_samples must be smaller than segment_length_samples".to_owned());
    }
    if samples.iter().any(|value| !value.is_finite()) {
        return Err("time history contains a non-finite sample".to_owned());
    }
    if samples.len() < config.segment_length_samples {
        return Err("time history is shorter than one Welch segment".to_owned());
    }

    let segment_length = config.segment_length_samples;
    let stride = segment_length - config.overlap_samples;
    let starts = (0..=samples.len() - segment_length)
        .step_by(stride)
        .collect::<Vec<_>>();
    if starts.len() < 2 {
        return Err("Welch estimate requires at least two segments".to_owned());
    }

    let sample_interval = config.sample_interval_seconds;
    let sampling_rate = 1.0 / sample_interval;
    let nyquist = 0.5 * sampling_rate;
    if !sampling_rate.is_finite() || !nyquist.is_finite() {
        return Err("sampling rate overflowed".to_owned());
    }

    let window = periodic_hann(segment_length);
    let window_energy = window.iter().map(|value| value * value).sum::<f64>();
    if !window_energy.is_finite() || window_energy <= 0.0 {
        return Err("Hann window energy is invalid".to_owned());
    }

    let bin_count = segment_length / 2 + 1;
    let mut psd = vec![0.0; bin_count];
    for start in &starts {
        let segment = &samples[*start..*start + segment_length];
        let segment_mean = segment.iter().sum::<f64>() / segment_length as f64;
        if !segment_mean.is_finite() {
            return Err("segment mean overflowed".to_owned());
        }

        let mut real = vec![0.0; segment_length];
        let mut imaginary = vec![0.0; segment_length];
        for index in 0..segment_length {
            real[index] = (segment[index] - segment_mean) * window[index];
        }
        fft_radix2(&mut real, &mut imaginary)?;

        for bin in 0..bin_count {
            let magnitude_squared = real[bin] * real[bin] + imaginary[bin] * imaginary[bin];
            let mut value = sample_interval * magnitude_squared / window_energy;
            if bin != 0 && bin != segment_length / 2 {
                value *= 2.0;
            }
            if !value.is_finite() {
                return Err("PSD calculation produced a non-finite value".to_owned());
            }
            psd[bin] += value;
        }
    }

    let segment_count = starts.len();
    for value in &mut psd {
        *value /= segment_count as f64;
        if !value.is_finite() || *value < 0.0 {
            return Err("averaged PSD contains an invalid value".to_owned());
        }
    }

    let bin_width = sampling_rate / segment_length as f64;
    let integrated_mean_square = psd.iter().sum::<f64>() * bin_width;
    if !bin_width.is_finite()
        || bin_width <= 0.0
        || !integrated_mean_square.is_finite()
        || integrated_mean_square < 0.0
    {
        return Err("integrated PSD power is invalid".to_owned());
    }

    let global_mean = samples.iter().sum::<f64>() / samples.len() as f64;
    if !global_mean.is_finite() {
        return Err("global time-history mean overflowed".to_owned());
    }
    let time_domain_mean_square = samples
        .iter()
        .map(|value| (value - global_mean) * (value - global_mean))
        .sum::<f64>()
        / samples.len() as f64;
    if !time_domain_mean_square.is_finite() || time_domain_mean_square < 0.0 {
        return Err("time-domain mean-square is invalid".to_owned());
    }
    let time_domain_rms = time_domain_mean_square.sqrt();
    let integrated_rms = integrated_mean_square.sqrt();
    let rms_relative_difference = if time_domain_rms > 0.0 {
        (integrated_rms - time_domain_rms).abs() / time_domain_rms
    } else if integrated_rms == 0.0 {
        0.0
    } else {
        return Err("non-zero PSD power for zero-variance time history".to_owned());
    };
    if !time_domain_rms.is_finite() || !rms_relative_difference.is_finite() {
        return Err("time-domain RMS calculation overflowed".to_owned());
    }

    let frequencies_hz = (0..bin_count)
        .map(|bin| bin as f64 * bin_width)
        .collect::<Vec<_>>();

    Ok(PsdEstimate {
        sample_interval_seconds: sample_interval,
        sampling_rate_hz: sampling_rate,
        segment_length_samples: segment_length,
        overlap_samples: config.overlap_samples,
        segment_stride_samples: stride,
        segment_count,
        window: "periodic_hann",
        detrending: "per_segment_mean_removed",
        frequencies_hz,
        psd_per_hz: psd,
        frequency_bin_width_hz: bin_width,
        nyquist_frequency_hz: nyquist,
        integrated_mean_square,
        integrated_rms,
        time_domain_rms,
        rms_relative_difference,
    })
}

pub fn stationarity_report(
    samples: &[f64],
    window_count: usize,
) -> Result<StationarityReport, String> {
    if window_count < 2 {
        return Err("stationarity analysis requires at least two windows".to_owned());
    }
    if samples.iter().any(|value| !value.is_finite()) {
        return Err("time history contains a non-finite sample".to_owned());
    }
    let samples_per_window = samples.len() / window_count;
    if samples_per_window < 2 {
        return Err("each stationarity window must contain at least two samples".to_owned());
    }

    let mut means = Vec::with_capacity(window_count);
    let mut rms_values = Vec::with_capacity(window_count);
    for window_index in 0..window_count {
        let start = window_index * samples_per_window;
        let end = if window_index + 1 == window_count {
            samples.len()
        } else {
            start + samples_per_window
        };
        let window = &samples[start..end];
        let mean = window.iter().sum::<f64>() / window.len() as f64;
        let mean_square = window
            .iter()
            .map(|value| (value - mean) * (value - mean))
            .sum::<f64>()
            / window.len() as f64;
        let rms = mean_square.sqrt();
        if !mean.is_finite() || !rms.is_finite() {
            return Err("stationarity window metric overflowed".to_owned());
        }
        means.push(mean);
        rms_values.push(rms);
    }

    let mean_min = means.iter().copied().fold(f64::INFINITY, f64::min);
    let mean_max = means.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let rms_min = rms_values.iter().copied().fold(f64::INFINITY, f64::min);
    let rms_max = rms_values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mean_rms = rms_values.iter().sum::<f64>() / rms_values.len() as f64;
    let rms_drift_fraction = if mean_rms.is_finite() && mean_rms > 0.0 {
        (rms_max - rms_min) / mean_rms
    } else if mean_rms == 0.0 && rms_max == 0.0 {
        0.0
    } else {
        return Err("stationarity RMS normalization is invalid".to_owned());
    };
    let mean_drift_absolute = mean_max - mean_min;
    if !mean_drift_absolute.is_finite() || !rms_drift_fraction.is_finite() {
        return Err("stationarity drift metric overflowed".to_owned());
    }

    Ok(StationarityReport {
        window_count,
        samples_per_window,
        mean_min,
        mean_max,
        mean_drift_absolute,
        rms_min,
        rms_max,
        rms_drift_fraction,
    })
}

pub fn dominant_frequency_hz(psd: &PsdEstimate) -> Result<f64, String> {
    if psd.frequencies_hz.len() != psd.psd_per_hz.len()
        || psd.psd_per_hz.len() < 2
        || psd.frequencies_hz.iter().any(|value| !value.is_finite() || *value < 0.0)
        || psd.psd_per_hz.iter().any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err("PSD frequency and value vectors are inconsistent or non-finite".to_owned());
    }
    let (index, _) = psd
        .psd_per_hz
        .iter()
        .enumerate()
        .skip(1)
        .max_by(|left, right| left.1.total_cmp(right.1))
        .ok_or_else(|| "PSD has no non-DC frequency bins".to_owned())?;
    Ok(psd.frequencies_hz[index])
}

pub fn band_power(psd: &PsdEstimate, lower_hz: f64, upper_hz: f64) -> Result<f64, String> {
    if psd.frequencies_hz.len() != psd.psd_per_hz.len()
        || psd.frequencies_hz.is_empty()
        || psd.frequencies_hz.iter().any(|value| !value.is_finite() || *value < 0.0)
        || psd.psd_per_hz.iter().any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err("PSD frequency and value vectors are inconsistent or non-finite".to_owned());
    }
    if !lower_hz.is_finite()
        || !upper_hz.is_finite()
        || lower_hz < 0.0
        || upper_hz <= lower_hz
        || upper_hz > psd.nyquist_frequency_hz
    {
        return Err("frequency band must be finite, increasing, and within Nyquist".to_owned());
    }
    let power = psd
        .frequencies_hz
        .iter()
        .zip(&psd.psd_per_hz)
        .filter(|(frequency, _)| **frequency >= lower_hz && **frequency <= upper_hz)
        .map(|(_, density)| density * psd.frequency_bin_width_hz)
        .sum::<f64>();
    if !power.is_finite() || power < 0.0 {
        return Err("band power is invalid".to_owned());
    }
    Ok(power)
}

pub fn compare_spectra(
    reference: &PsdEstimate,
    candidate: &PsdEstimate,
) -> Result<SpectralComparison, String> {
    if reference.frequencies_hz.len() != reference.psd_per_hz.len()
        || candidate.frequencies_hz.len() != candidate.psd_per_hz.len()
        || reference.frequencies_hz.len() != candidate.frequencies_hz.len()
        || reference.psd_per_hz.len() != candidate.psd_per_hz.len()
        || reference.frequencies_hz.iter().chain(&candidate.frequencies_hz).any(|value| !value.is_finite())
        || reference.psd_per_hz.iter().chain(&candidate.psd_per_hz).any(|value| !value.is_finite() || *value < 0.0)
        || (reference.frequency_bin_width_hz - candidate.frequency_bin_width_hz).abs() > 1.0e-12
        || reference
            .frequencies_hz
            .iter()
            .zip(&candidate.frequencies_hz)
            .any(|(left, right)| (left - right).abs() > 1.0e-10)
    {
        return Err("spectra must use identical frequency bins for comparison".to_owned());
    }
    if reference.integrated_mean_square <= 0.0 || candidate.integrated_mean_square <= 0.0 {
        return Err("spectral comparison requires positive integrated power".to_owned());
    }
    let distance = reference
        .psd_per_hz
        .iter()
        .zip(&candidate.psd_per_hz)
        .map(|(left, right)| {
            (left / reference.integrated_mean_square
                - right / candidate.integrated_mean_square)
                .abs()
        })
        .sum::<f64>()
        * reference.frequency_bin_width_hz
        * 0.5;
    let integrated_mean_square_relative_error = (candidate.integrated_mean_square
        - reference.integrated_mean_square)
        .abs()
        / reference.integrated_mean_square;
    if !distance.is_finite() || !integrated_mean_square_relative_error.is_finite() {
        return Err("spectral comparison produced non-finite metrics".to_owned());
    }

    let reference_peak = dominant_frequency_hz(reference)?;
    let candidate_peak = dominant_frequency_hz(candidate)?;
    Ok(SpectralComparison {
        reference_peak_frequency_hz: reference_peak,
        candidate_peak_frequency_hz: candidate_peak,
        dominant_frequency_error_hz: (candidate_peak - reference_peak).abs(),
        reference_integrated_mean_square: reference.integrated_mean_square,
        candidate_integrated_mean_square: candidate.integrated_mean_square,
        integrated_mean_square_relative_error,
        normalized_spectral_shape_distance: distance,
    })
}

fn periodic_hann(length: usize) -> Vec<f64> {
    (0..length)
        .map(|index| 0.5 * (1.0 - (2.0 * PI * index as f64 / length as f64).cos()))
        .collect()
}

fn fft_radix2(real: &mut [f64], imaginary: &mut [f64]) -> Result<(), String> {
    let length = real.len();
    if length != imaginary.len() || length < 2 || !length.is_power_of_two() {
        return Err("FFT requires equal-length power-of-two real/imaginary buffers".to_owned());
    }

    let mut reversed = 0usize;
    for index in 1..length {
        let mut bit = length >> 1;
        while reversed & bit != 0 {
            reversed ^= bit;
            bit >>= 1;
        }
        reversed ^= bit;
        if index < reversed {
            real.swap(index, reversed);
            imaginary.swap(index, reversed);
        }
    }

    let mut block_length = 2usize;
    while block_length <= length {
        let angle = -2.0 * PI / block_length as f64;
        let step_real = angle.cos();
        let step_imaginary = angle.sin();
        for block_start in (0..length).step_by(block_length) {
            let mut twiddle_real = 1.0;
            let mut twiddle_imaginary = 0.0;
            for offset in 0..block_length / 2 {
                let even = block_start + offset;
                let odd = even + block_length / 2;
                let odd_real =
                    twiddle_real * real[odd] - twiddle_imaginary * imaginary[odd];
                let odd_imaginary =
                    twiddle_real * imaginary[odd] + twiddle_imaginary * real[odd];
                let even_real = real[even];
                let even_imaginary = imaginary[even];
                real[even] = even_real + odd_real;
                imaginary[even] = even_imaginary + odd_imaginary;
                real[odd] = even_real - odd_real;
                imaginary[odd] = even_imaginary - odd_imaginary;

                let next_real =
                    twiddle_real * step_real - twiddle_imaginary * step_imaginary;
                twiddle_imaginary =
                    twiddle_real * step_imaginary + twiddle_imaginary * step_real;
                twiddle_real = next_real;
            }
        }
        if block_length == length {
            break;
        }
        block_length *= 2;
    }

    if real.iter().chain(imaginary.iter()).any(|value| !value.is_finite()) {
        return Err("FFT produced a non-finite value".to_owned());
    }
    Ok(())
}

pub const PSD_CSV_HEADER: &str = "frequency_hz,psd_per_hz";
pub fn report_to_csv(psd: &PsdEstimate) -> String {
    let mut output = format!("{PSD_CSV_HEADER}\n");
    for (frequency, density) in psd.frequencies_hz.iter().zip(&psd.psd_per_hz) {
        output.push_str(&format!("{frequency:.12},{density:.15}\n"));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine_trace(amplitude: f64, frequency_hz: f64, sample_count: usize, sampling_rate_hz: f64, offset: f64) -> Vec<f64> {
        (0..sample_count)
            .map(|index| {
                offset + amplitude * (2.0 * PI * frequency_hz * index as f64 / sampling_rate_hz).sin()
            })
            .collect()
    }

    fn config() -> WelchConfig {
        WelchConfig {
            sample_interval_seconds: 1.0 / 512.0,
            segment_length_samples: 256,
            overlap_samples: 128,
        }
    }

    #[test]
    fn known_tone_peak_psd_integral_and_stationarity() {
        let samples = sine_trace(2.0, 32.0, 1024, 512.0, 1.25);
        let psd = welch_one_sided_psd(&samples, &config()).expect("valid tone");
        assert_eq!(psd.segment_count, 7);
        assert_eq!(psd.frequencies_hz.len(), 129);
        assert!((dominant_frequency_hz(&psd).unwrap() - 32.0).abs() < 1.0e-10);
        assert!((psd.integrated_mean_square - 2.0).abs() < 1.0e-10);
        assert!((psd.time_domain_rms - 2.0_f64.sqrt()).abs() < 1.0e-10);
        assert!(psd.rms_relative_difference < 1.0e-10);

        let stationary = stationarity_report(&samples, 8).expect("stationary tone");
        assert!(stationary.mean_drift_absolute < 1.0e-10);
        assert!(stationary.rms_drift_fraction < 1.0e-10);
        assert!((band_power(&psd, 30.0, 34.0).unwrap() - 2.0).abs() < 1.0e-10);
    }

    #[test]
    fn spectral_comparison_detects_frequency_and_shape_change() {
        let reference = welch_one_sided_psd(&sine_trace(2.0, 32.0, 1024, 512.0, 0.0), &config())
            .expect("reference tone");
        let candidate = welch_one_sided_psd(&sine_trace(2.0, 40.0, 1024, 512.0, 0.0), &config())
            .expect("shifted tone");
        let comparison = compare_spectra(&reference, &candidate).expect("same bins");
        assert!((comparison.dominant_frequency_error_hz - 8.0).abs() < 1.0e-10);
        assert!(comparison.normalized_spectral_shape_distance > 0.1);
        assert!(compare_spectra(
            &reference,
            &welch_one_sided_psd(&sine_trace(2.0, 40.0, 1024, 512.0, 0.0), &WelchConfig {
                sample_interval_seconds: 1.0 / 512.0,
                segment_length_samples: 128,
                overlap_samples: 64,
            }).unwrap()
        ).is_err());
    }

    #[test]
    fn nonstationary_amplitude_step_is_detected() {
        let mut samples = sine_trace(1.0, 32.0, 512, 512.0, 0.0);
        samples.extend(sine_trace(2.0, 32.0, 512, 512.0, 0.0));
        let report = stationarity_report(&samples, 8).expect("finite windows");
        assert!(report.rms_drift_fraction > 0.4);
    }

    #[test]
    fn public_spectrum_helpers_reject_malformed_vectors_and_nonfinite_values() {
        let psd = welch_one_sided_psd(
            &sine_trace(1.0, 32.0, 1024, 512.0, 0.0),
            &config(),
        )
        .expect("known spectrum");

        let mut truncated = psd.clone();
        truncated.psd_per_hz.pop();
        assert!(band_power(&truncated, 10.0, 20.0).is_err());
        assert!(compare_spectra(&psd, &truncated).is_err());

        let mut nonfinite = psd;
        nonfinite.psd_per_hz[4] = f64::NAN;
        assert!(dominant_frequency_hz(&nonfinite).is_err());
        assert!(band_power(&nonfinite, 10.0, 20.0).is_err());
    }

    #[test]
    fn rejects_bad_sampling_overlap_short_traces_and_nonfinite_values() {
        let samples = sine_trace(1.0, 32.0, 256, 512.0, 0.0);
        let mut bad = config();
        bad.sample_interval_seconds = 0.0;
        assert!(welch_one_sided_psd(&samples, &bad).is_err());
        let mut bad = config();
        bad.overlap_samples = bad.segment_length_samples;
        assert!(welch_one_sided_psd(&samples, &bad).is_err());
        assert!(welch_one_sided_psd(&samples[..64], &config()).is_err());
        let mut nonfinite = samples;
        nonfinite[3] = f64::NAN;
        assert!(welch_one_sided_psd(&nonfinite, &config()).is_err());
        assert!(stationarity_report(&nonfinite, 8).is_err());
    }
}
