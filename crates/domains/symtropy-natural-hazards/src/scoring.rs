// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Small dependency-light probabilistic verification metrics.
//!
//! These metrics score predictions against declared observations. They do not
//! establish that the underlying physical model is valid for a hazard regime.

use serde::{Deserialize, Serialize};

/// Invalid inputs for probabilistic forecast scores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoringError {
    EmptySample,
    InvalidProbability,
    InvalidBinCount,
    NonFiniteValue,
    InvalidScore,
    ZeroReferenceScore,
}

/// One probability forecast and its subsequently observed binary outcome.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BinaryForecast {
    /// Forecast probability of the event, in [0, 1].
    pub probability: f64,
    /// Whether the event occurred in the declared verification window.
    pub observed: bool,
}

/// Score a set of binary forecasts using the mean Brier score.
///
/// Lower is better; zero is perfect. Samples must use a consistent event
/// definition, spatial support, and verification window.
pub fn brier_score(forecasts: &[BinaryForecast]) -> Result<f64, ScoringError> {
    validate_binary_forecasts(forecasts)?;
    let sum = forecasts
        .iter()
        .map(|forecast| {
            let outcome = if forecast.observed { 1.0 } else { 0.0 };
            (forecast.probability - outcome).powi(2)
        })
        .sum::<f64>();
    Ok(sum / forecasts.len() as f64)
}

/// One bin in a reliability diagram for probabilistic binary forecasts.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReliabilityBin {
    /// Inclusive lower probability boundary.
    pub lower_bound: f64,
    /// Exclusive upper boundary except for the final bin, which includes 1.0.
    pub upper_bound: f64,
    pub count: usize,
    /// None when the bin contains no forecasts.
    pub mean_forecast_probability: Option<f64>,
    /// None when the bin contains no forecasts.
    pub observed_event_frequency: Option<f64>,
}

/// Group probability forecasts into equal-width bins for calibration diagnosis.
///
/// Empty bins are retained and reported with null means; they are not silently
/// treated as zero-probability or zero-event bins.
pub fn reliability_bins(
    forecasts: &[BinaryForecast],
    bin_count: usize,
) -> Result<Vec<ReliabilityBin>, ScoringError> {
    validate_binary_forecasts(forecasts)?;
    if bin_count == 0 {
        return Err(ScoringError::InvalidBinCount);
    }

    let mut counts = vec![0usize; bin_count];
    let mut probability_sums = vec![0.0f64; bin_count];
    let mut event_counts = vec![0usize; bin_count];

    for forecast in forecasts {
        // p=1 belongs to the final bin rather than an out-of-range bin.
        let index = ((forecast.probability * bin_count as f64).floor() as usize).min(bin_count - 1);
        counts[index] += 1;
        probability_sums[index] += forecast.probability;
        if forecast.observed {
            event_counts[index] += 1;
        }
    }

    Ok((0..bin_count)
        .map(|index| {
            let count = counts[index];
            ReliabilityBin {
                lower_bound: index as f64 / bin_count as f64,
                upper_bound: (index + 1) as f64 / bin_count as f64,
                count,
                mean_forecast_probability: (count > 0)
                    .then(|| probability_sums[index] / count as f64),
                observed_event_frequency: (count > 0)
                    .then(|| event_counts[index] as f64 / count as f64),
            }
        })
        .collect())
}

/// Compute the empirical Continuous Ranked Probability Score (CRPS) for an
/// equally weighted finite ensemble and one scalar observation.
///
/// The implementation uses the sorted-ensemble identity and is O(n log n),
/// rather than constructing all pairwise ensemble differences. Lower is better.
/// Units are the same as the observed quantity.
pub fn ensemble_crps(ensemble: &[f64], observation: f64) -> Result<f64, ScoringError> {
    if ensemble.is_empty() {
        return Err(ScoringError::EmptySample);
    }
    if !observation.is_finite() || ensemble.iter().any(|value| !value.is_finite()) {
        return Err(ScoringError::NonFiniteValue);
    }

    let mut sorted = ensemble.to_vec();
    sorted.sort_by(f64::total_cmp);
    let count = sorted.len() as f64;
    let absolute_error = sorted
        .iter()
        .map(|value| (value - observation).abs())
        .sum::<f64>()
        / count;
    let pairwise_correction = sorted
        .iter()
        .enumerate()
        .map(|(index, value)| (2.0 * index as f64 - count + 1.0) * value)
        .sum::<f64>()
        / count.powi(2);

    // The mathematical score is non-negative. Clamp only a possible tiny
    // negative floating-point residue at zero.
    Ok((absolute_error - pairwise_correction).max(0.0))
}

/// Brier skill relative to a declared reference score.
///
/// A positive value improves on the reference, zero ties it, and a negative
/// value is worse. A zero reference score makes the ratio undefined.
pub fn brier_skill_score(model_brier: f64, reference_brier: f64) -> Result<f64, ScoringError> {
    if !model_brier.is_finite()
        || !reference_brier.is_finite()
        || model_brier < 0.0
        || reference_brier < 0.0
    {
        return Err(ScoringError::InvalidScore);
    }
    if reference_brier == 0.0 {
        return Err(ScoringError::ZeroReferenceScore);
    }
    Ok(1.0 - model_brier / reference_brier)
}

fn validate_binary_forecasts(forecasts: &[BinaryForecast]) -> Result<(), ScoringError> {
    if forecasts.is_empty() {
        return Err(ScoringError::EmptySample);
    }
    if forecasts.iter().any(|forecast| {
        !forecast.probability.is_finite() || !(0.0..=1.0).contains(&forecast.probability)
    }) {
        return Err(ScoringError::InvalidProbability);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-12,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn brier_score_matches_hand_calculated_reference() {
        let forecasts = [
            BinaryForecast {
                probability: 0.1,
                observed: false,
            },
            BinaryForecast {
                probability: 0.9,
                observed: true,
            },
        ];
        close(brier_score(&forecasts).unwrap(), 0.01);
    }

    #[test]
    fn brier_score_rejects_empty_and_invalid_probabilities() {
        assert_eq!(brier_score(&[]), Err(ScoringError::EmptySample));
        assert_eq!(
            brier_score(&[BinaryForecast {
                probability: 1.01,
                observed: true,
            }]),
            Err(ScoringError::InvalidProbability)
        );
        assert_eq!(
            brier_score(&[BinaryForecast {
                probability: f64::NAN,
                observed: false,
            }]),
            Err(ScoringError::InvalidProbability)
        );
    }

    #[test]
    fn reliability_bins_keep_empty_bins_explicit_and_include_probability_one() {
        let forecasts = [
            BinaryForecast {
                probability: 0.0,
                observed: false,
            },
            BinaryForecast {
                probability: 1.0,
                observed: true,
            },
        ];
        let bins = reliability_bins(&forecasts, 4).unwrap();
        assert_eq!(bins.len(), 4);
        assert_eq!(bins[0].count, 1);
        assert_eq!(bins[3].count, 1);
        close(bins[0].mean_forecast_probability.unwrap(), 0.0);
        close(bins[3].observed_event_frequency.unwrap(), 1.0);
        assert_eq!(bins[1].count, 0);
        assert_eq!(bins[1].mean_forecast_probability, None);
        assert_eq!(bins[1].observed_event_frequency, None);
        assert_eq!(
            reliability_bins(&forecasts, 0),
            Err(ScoringError::InvalidBinCount)
        );
    }

    #[test]
    fn empirical_crps_matches_known_values_and_is_order_invariant() {
        close(ensemble_crps(&[0.0, 2.0], 0.0).unwrap(), 0.5);
        close(ensemble_crps(&[1.0], 3.0).unwrap(), 2.0);
        close(ensemble_crps(&[2.0, 0.0], 0.0).unwrap(), 0.5);
        assert_eq!(ensemble_crps(&[], 1.0), Err(ScoringError::EmptySample));
        assert_eq!(
            ensemble_crps(&[0.0, f64::INFINITY], 1.0),
            Err(ScoringError::NonFiniteValue)
        );
    }

    #[test]
    fn skill_score_exposes_better_and_worse_than_baseline() {
        close(brier_skill_score(0.1, 0.25).unwrap(), 0.6);
        close(brier_skill_score(0.3, 0.25).unwrap(), -0.2);
        assert_eq!(
            brier_skill_score(0.0, 0.0),
            Err(ScoringError::ZeroReferenceScore)
        );
        assert_eq!(
            brier_skill_score(f64::NAN, 0.25),
            Err(ScoringError::InvalidScore)
        );
    }
}
