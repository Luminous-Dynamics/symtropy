use std::f64::consts::PI;
use std::process;

use civil_aero_benchmark::unsteady_spectrum::{
    WelchConfig, band_power, dominant_frequency_hz, report_to_csv, stationarity_report,
    welch_one_sided_psd,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("synthetic unsteady-spectrum demo rejected: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let sampling_rate_hz = 512.0;
    let sample_count = 1024usize;
    let amplitude = 2.0;
    let tone_frequency_hz = 32.0;
    let samples = (0..sample_count)
        .map(|index| {
            1.25 + amplitude
                * (2.0 * PI * tone_frequency_hz * index as f64 / sampling_rate_hz).sin()
        })
        .collect::<Vec<_>>();
    let config = WelchConfig {
        sample_interval_seconds: 1.0 / sampling_rate_hz,
        segment_length_samples: 256,
        overlap_samples: 128,
    };
    let psd = welch_one_sided_psd(&samples, &config)?;
    let stationarity = stationarity_report(&samples, 8)?;
    let peak = dominant_frequency_hz(&psd)?;
    let band = band_power(&psd, 30.0, 34.0)?;
    eprintln!(
        "synthetic_unsteady_spectrum_only samples={} segments={} bins={} peak_hz={:.12} integrated_mean_square={:.12} band_power_30_34={:.12} rms_relative_difference={:.12} stationarity_mean_drift={:.12} stationarity_rms_drift_fraction={:.12}",
        sample_count,
        psd.segment_count,
        psd.frequencies_hz.len(),
        peak,
        psd.integrated_mean_square,
        band,
        psd.rms_relative_difference,
        stationarity.mean_drift_absolute,
        stationarity.rms_drift_fraction
    );
    print!("{}", report_to_csv(&psd));
    Ok(())
}
