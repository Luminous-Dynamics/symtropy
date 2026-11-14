// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Proper scoring, interval diagnostics and selective forecast coverage.
//!
//! The kernel scores finite ensemble samples with the empirical CRPS and central
//! prediction intervals. It always reports how many eligible cases received a forecast.
//! It does not calibrate a model, estimate ecological process uncertainty or decide when
//! a model should abstain.

use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ForecastError {
    EmptyForecastIdentity,
    DuplicateForecastIdentity,
    EmptyBaseline,
    InvalidEnsembleSize,
    NonFiniteValue,
    InvalidUnit,
    MixedEvaluationSlice,
    NumericOverflow,
}

impl fmt::Display for ForecastError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ForecastError {}

#[derive(Clone, Debug, PartialEq)]
pub struct ForecastCase {
    pub forecast_id: String,
    pub variable: String,
    pub unit: String,
    pub horizon_days: u32,
    pub observed: f64,
    /// Empty means the system abstained. No prediction is imputed for scoring.
    pub model_samples: Vec<f64>,
    /// A paired reference forecast (e.g. seasonal climatology or persistence).
    pub baseline_samples: Vec<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ForecastCaseScore {
    pub model_crps: Option<f64>,
    pub baseline_crps: f64,
    pub model_mean: Option<f64>,
    pub absolute_error: Option<f64>,
    pub median: Option<f64>,
    pub lower_50: Option<f64>,
    pub upper_50: Option<f64>,
    pub lower_80: Option<f64>,
    pub upper_80: Option<f64>,
    pub lower_95: Option<f64>,
    pub upper_95: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ForecastSummary {
    pub eligible_case_count: usize,
    pub forecast_case_count: usize,
    pub abstained_case_count: usize,
    pub forecast_coverage_fraction: f64,
    pub mean_model_crps: Option<f64>,
    pub mean_baseline_crps_paired: Option<f64>,
    pub mean_baseline_crps_all: f64,
    /// Positive means lower CRPS than the paired baseline; undefined for a zero-score baseline.
    pub crps_skill_score: Option<f64>,
    pub mean_absolute_error: Option<f64>,
    pub interval_coverage_50: Option<f64>,
    pub interval_coverage_80: Option<f64>,
    pub interval_coverage_95: Option<f64>,
    pub mean_interval_width_80: Option<f64>,
    pub mean_interval_width_95: Option<f64>,
    pub disposition: &'static str,
}

/// Convert a bounded count without silently losing integer precision.
fn count_as_f64(count: usize) -> Result<f64, ForecastError> {
    let bounded = u32::try_from(count).map_err(|_| ForecastError::InvalidEnsembleSize)?;
    Ok(f64::from(bounded))
}

/// Empirical CRPS for equally weighted ensemble samples:
/// mean(|x_i - y|) - 0.5 * mean(|x_i - x_j|).
///
/// The sorted O(n log n) expression avoids constructing an O(n²) pairwise matrix.
pub fn empirical_crps(samples: &[f64], observed: f64) -> Result<f64, ForecastError> {
    if samples.is_empty() {
        return Err(ForecastError::InvalidEnsembleSize);
    }
    if samples.len() > 1_000_000 {
        return Err(ForecastError::InvalidEnsembleSize);
    }
    if !observed.is_finite() || samples.iter().any(|value| !value.is_finite()) {
        return Err(ForecastError::NonFiniteValue);
    }

    let count = count_as_f64(samples.len())?;
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);

    let absolute_error_sum = sorted.iter().map(|value| (value - observed).abs()).sum::<f64>();
    let mut pairwise_correction = 0.0;
    for (index, value) in sorted.iter().enumerate() {
        let index_f64 = count_as_f64(index)?;
        let coefficient = (2.0 * index_f64) - count + 1.0;
        pairwise_correction += coefficient * value;
    }

    let score = absolute_error_sum / count - pairwise_correction / (count * count);
    if !score.is_finite() || score < -1e-10 {
        return Err(ForecastError::NumericOverflow);
    }
    Ok(score.max(0.0))
}

/// Empirical nearest-rank quantile. For small ensembles this is intentionally discrete;
/// it does not pretend that finite ensemble tails are smoothly estimated.
pub fn empirical_quantile(samples: &[f64], probability: f64) -> Result<f64, ForecastError> {
    if samples.is_empty() || samples.len() > 1_000_000 {
        return Err(ForecastError::InvalidEnsembleSize);
    }
    if !probability.is_finite() || !(0.0..=1.0).contains(&probability) {
        return Err(ForecastError::NonFiniteValue);
    }
    if samples.iter().any(|value| !value.is_finite()) {
        return Err(ForecastError::NonFiniteValue);
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    if probability == 0.0 {
        return Ok(sorted[0]);
    }
    if probability == 1.0 {
        return Ok(*sorted.last().ok_or(ForecastError::InvalidEnsembleSize)?);
    }
    let count = count_as_f64(sorted.len())?;
    for (index, value) in sorted.iter().enumerate() {
        let rank = count_as_f64(index + 1)?;
        if rank / count >= probability {
            return Ok(*value);
        }
    }
    sorted.last().copied().ok_or(ForecastError::InvalidEnsembleSize)
}

fn ensemble_mean(samples: &[f64]) -> Result<f64, ForecastError> {
    if samples.is_empty() || samples.len() > 1_000_000 {
        return Err(ForecastError::InvalidEnsembleSize);
    }
    if samples.iter().any(|value| !value.is_finite()) {
        return Err(ForecastError::NonFiniteValue);
    }
    let total = samples.iter().sum::<f64>();
    let mean = total / count_as_f64(samples.len())?;
    if mean.is_finite() {
        Ok(mean)
    } else {
        Err(ForecastError::NumericOverflow)
    }
}

pub fn score_forecast_case(case: &ForecastCase) -> Result<ForecastCaseScore, ForecastError> {
    if case.forecast_id.trim().is_empty() {
        return Err(ForecastError::EmptyForecastIdentity);
    }
    if case.variable.trim().is_empty() || case.unit.trim().is_empty() {
        return Err(ForecastError::InvalidUnit);
    }
    if !case.observed.is_finite() {
        return Err(ForecastError::NonFiniteValue);
    }
    if case.baseline_samples.is_empty() {
        return Err(ForecastError::EmptyBaseline);
    }

    let baseline_crps = empirical_crps(&case.baseline_samples, case.observed)?;
    if case.model_samples.is_empty() {
        return Ok(ForecastCaseScore {
            model_crps: None,
            baseline_crps,
            model_mean: None,
            absolute_error: None,
            median: None,
            lower_50: None,
            upper_50: None,
            lower_80: None,
            upper_80: None,
            lower_95: None,
            upper_95: None,
        });
    }

    let model_crps = empirical_crps(&case.model_samples, case.observed)?;
    let model_mean = ensemble_mean(&case.model_samples)?;
    Ok(ForecastCaseScore {
        model_crps: Some(model_crps),
        baseline_crps,
        model_mean: Some(model_mean),
        absolute_error: Some((model_mean - case.observed).abs()),
        median: Some(empirical_quantile(&case.model_samples, 0.5)?),
        lower_50: Some(empirical_quantile(&case.model_samples, 0.25)?),
        upper_50: Some(empirical_quantile(&case.model_samples, 0.75)?),
        lower_80: Some(empirical_quantile(&case.model_samples, 0.1)?),
        upper_80: Some(empirical_quantile(&case.model_samples, 0.9)?),
        lower_95: Some(empirical_quantile(&case.model_samples, 0.025)?),
        upper_95: Some(empirical_quantile(&case.model_samples, 0.975)?),
    })
}

/// Evaluate an eligible set of forecasts. Abstentions count against forecast coverage and
/// are omitted from paired predictive scores. Baselines are scored both on all cases and
/// on the exact subset for which the model emitted a forecast.
pub fn summarize_forecasts(cases: &[ForecastCase]) -> Result<ForecastSummary, ForecastError> {
    if cases.is_empty() {
        return Err(ForecastError::InvalidEnsembleSize);
    }

    let first = &cases[0];
    if first.variable.trim().is_empty() || first.unit.trim().is_empty() {
        return Err(ForecastError::InvalidUnit);
    }
    let mut ids = BTreeSet::new();
    let mut forecast_case_count = 0usize;
    let mut model_crps_total = 0.0;
    let mut paired_baseline_crps_total = 0.0;
    let mut all_baseline_crps_total = 0.0;
    let mut absolute_error_total = 0.0;
    let mut coverage_50_count = 0usize;
    let mut coverage_80_count = 0usize;
    let mut coverage_95_count = 0usize;
    let mut width_80_total = 0.0;
    let mut width_95_total = 0.0;

    for case in cases {
        if !ids.insert(case.forecast_id.as_str()) {
            return Err(ForecastError::DuplicateForecastIdentity);
        }
        if case.variable != first.variable
            || case.unit != first.unit
            || case.horizon_days != first.horizon_days
        {
            return Err(ForecastError::MixedEvaluationSlice);
        }
        let score = score_forecast_case(case)?;
        all_baseline_crps_total += score.baseline_crps;
        if let Some(model_crps) = score.model_crps {
            forecast_case_count += 1;
            model_crps_total += model_crps;
            paired_baseline_crps_total += score.baseline_crps;
            absolute_error_total += score.absolute_error.ok_or(ForecastError::NumericOverflow)?;
            let lower_50 = score.lower_50.ok_or(ForecastError::NumericOverflow)?;
            let upper_50 = score.upper_50.ok_or(ForecastError::NumericOverflow)?;
            let lower_80 = score.lower_80.ok_or(ForecastError::NumericOverflow)?;
            let upper_80 = score.upper_80.ok_or(ForecastError::NumericOverflow)?;
            let lower_95 = score.lower_95.ok_or(ForecastError::NumericOverflow)?;
            let upper_95 = score.upper_95.ok_or(ForecastError::NumericOverflow)?;
            if case.observed >= lower_50 && case.observed <= upper_50 {
                coverage_50_count += 1;
            }
            if case.observed >= lower_80 && case.observed <= upper_80 {
                coverage_80_count += 1;
            }
            if case.observed >= lower_95 && case.observed <= upper_95 {
                coverage_95_count += 1;
            }
            width_80_total += upper_80 - lower_80;
            width_95_total += upper_95 - lower_95;
        }
    }

    let eligible_case_count = cases.len();
    let abstained_case_count = eligible_case_count - forecast_case_count;
    let forecast_coverage_fraction =
        forecast_case_count as f64 / count_as_f64(eligible_case_count)?;
    let mean_baseline_crps_all = all_baseline_crps_total / count_as_f64(eligible_case_count)?;
    let (mean_model_crps, mean_baseline_crps_paired, crps_skill_score, mean_absolute_error,
        interval_coverage_50, interval_coverage_80, interval_coverage_95,
        mean_interval_width_80, mean_interval_width_95, disposition) =
        if forecast_case_count == 0 {
            (None, None, None, None, None, None, None, None, None, "NO_FORECAST_COVERAGE")
        } else {
            let paired_count = count_as_f64(forecast_case_count)?;
            let model_mean = model_crps_total / paired_count;
            let baseline_mean = paired_baseline_crps_total / paired_count;
            let skill = if baseline_mean > 0.0 {
                Some(1.0 - model_mean / baseline_mean)
            } else {
                None
            };
            (
                Some(model_mean),
                Some(baseline_mean),
                skill,
                Some(absolute_error_total / paired_count),
                Some(coverage_50_count as f64 / paired_count),
                Some(coverage_80_count as f64 / paired_count),
                Some(coverage_95_count as f64 / paired_count),
                Some(width_80_total / paired_count),
                Some(width_95_total / paired_count),
                "SCORED_WITH_EXPLICIT_FORECAST_COVERAGE",
            )
        };

    Ok(ForecastSummary {
        eligible_case_count,
        forecast_case_count,
        abstained_case_count,
        forecast_coverage_fraction,
        mean_model_crps,
        mean_baseline_crps_paired,
        mean_baseline_crps_all,
        crps_skill_score,
        mean_absolute_error,
        interval_coverage_50,
        interval_coverage_80,
        interval_coverage_95,
        mean_interval_width_80,
        mean_interval_width_95,
        disposition,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn forecast(id: &str, model: &[f64], baseline: &[f64], observed: f64) -> ForecastCase {
        ForecastCase {
            forecast_id: id.to_string(),
            variable: "aboveground_biomass_density".to_string(),
            unit: "Mg/ha".to_string(),
            horizon_days: 365,
            observed,
            model_samples: model.to_vec(),
            baseline_samples: baseline.to_vec(),
        }
    }

    #[test]
    fn empirical_crps_matches_two_member_hand_calculation() {
        let score = empirical_crps(&[0.0, 2.0], 1.0).expect("empirical CRPS");
        assert!((score - 0.5).abs() < 1e-12);
    }

    #[test]
    fn empirical_crps_is_zero_for_perfect_degenerate_forecast() {
        assert_eq!(empirical_crps(&[3.0, 3.0, 3.0], 3.0), Ok(0.0));
    }

    #[test]
    fn empirical_crps_reduces_to_absolute_error_for_point_mass() {
        assert_eq!(empirical_crps(&[1.0], 4.0), Ok(3.0));
    }

    #[test]
    fn empirical_crps_is_permutation_invariant() {
        let left = empirical_crps(&[-2.0, 0.0, 2.0, 4.0], 1.0).unwrap();
        let right = empirical_crps(&[4.0, -2.0, 2.0, 0.0], 1.0).unwrap();
        assert_eq!(left, right);
    }

    #[test]
    fn empirical_quantile_uses_documented_nearest_rank_rule() {
        assert_eq!(empirical_quantile(&[4.0, 1.0, 3.0, 2.0], 0.5), Ok(2.0));
        assert_eq!(empirical_quantile(&[4.0, 1.0, 3.0, 2.0], 0.9), Ok(4.0));
    }

    #[test]
    fn empirical_crps_rejects_empty_and_non_finite_ensemble() {
        assert_eq!(empirical_crps(&[], 1.0), Err(ForecastError::InvalidEnsembleSize));
        assert_eq!(empirical_crps(&[f64::NAN], 1.0), Err(ForecastError::NonFiniteValue));
    }

    #[test]
    fn score_forecast_case_preserves_abstention() {
        let score = score_forecast_case(&forecast("abstain", &[], &[0.0], 2.0))
            .expect("abstention score");
        assert_eq!(score.model_crps, None);
        assert_eq!(score.baseline_crps, 2.0);
        assert_eq!(score.model_mean, None);
    }

    #[test]
    fn summary_reports_selective_coverage_and_paired_skill() {
        let cases = [
            forecast("a", &[0.0, 2.0], &[0.0], 1.0),
            forecast("b", &[1.0, 3.0], &[0.0], 2.0),
            forecast("c", &[2.0, 2.0], &[0.0], 5.0),
            forecast("d", &[-2.0, 0.0, 2.0, 4.0], &[0.0], 1.0),
            forecast("e", &[], &[0.0], 4.0),
            forecast("f", &[8.0, 10.0], &[0.0], 1.0),
        ];
        let result = summarize_forecasts(&cases).expect("forecast summary");
        assert_eq!(result.eligible_case_count, 6);
        assert_eq!(result.forecast_case_count, 5);
        assert_eq!(result.abstained_case_count, 1);
        assert!((result.forecast_coverage_fraction - (5.0 / 6.0)).abs() < 1e-12);
        assert_eq!(result.disposition, "SCORED_WITH_EXPLICIT_FORECAST_COVERAGE");
        assert!(result.mean_model_crps.is_some());
        assert!(result.crps_skill_score.is_some());
    }

    #[test]
    fn all_abstentions_do_not_produce_a_fake_zero_score() {
        let result = summarize_forecasts(&[
            forecast("a", &[], &[0.0], 1.0),
            forecast("b", &[], &[0.0], 2.0),
        ])
        .expect("all-abstention summary");
        assert_eq!(result.forecast_coverage_fraction, 0.0);
        assert_eq!(result.mean_model_crps, None);
        assert_eq!(result.crps_skill_score, None);
        assert_eq!(result.disposition, "NO_FORECAST_COVERAGE");
    }

    #[test]
    fn zero_score_baseline_has_undefined_skill_not_division_by_zero() {
        let result = summarize_forecasts(&[
            forecast("a", &[1.0, 1.0], &[1.0], 1.0),
        ])
        .expect("zero-baseline summary");
        assert_eq!(result.mean_baseline_crps_paired, Some(0.0));
        assert_eq!(result.crps_skill_score, None);
    }

    #[test]
    fn mixed_variable_or_horizon_slices_are_rejected() {
        let mut other = forecast("b", &[1.0], &[1.0], 1.0);
        other.horizon_days = 30;
        assert_eq!(
            summarize_forecasts(&[forecast("a", &[1.0], &[1.0], 1.0), other]),
            Err(ForecastError::MixedEvaluationSlice)
        );
    }

    #[test]
    fn duplicate_forecast_identity_is_rejected() {
        assert_eq!(
            summarize_forecasts(&[
                forecast("same", &[1.0], &[0.0], 1.0),
                forecast("same", &[1.0], &[0.0], 1.0),
            ]),
            Err(ForecastError::DuplicateForecastIdentity)
        );
    }
}
