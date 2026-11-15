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
    InvalidProbability,
    EmptyEventIdentity,
    MixedEventSlice,
    InvalidConfidenceScore,
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
    pub model_wis: Option<f64>,
    pub baseline_wis: f64,
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
    pub mean_model_wis: Option<f64>,
    pub mean_baseline_wis_paired: Option<f64>,
    pub mean_baseline_wis_all: f64,
    /// Positive means lower WIS than the paired baseline.
    pub wis_skill_score: Option<f64>,
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

    let absolute_error_sum = sorted
        .iter()
        .map(|value| (value - observed).abs())
        .sum::<f64>();
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

fn interval_score(lower: f64, upper: f64, observed: f64, alpha: f64) -> Result<f64, ForecastError> {
    if !lower.is_finite()
        || !upper.is_finite()
        || !observed.is_finite()
        || !alpha.is_finite()
        || alpha <= 0.0
        || alpha >= 1.0
        || lower > upper
    {
        return Err(ForecastError::NonFiniteValue);
    }
    let lower_penalty = if observed < lower {
        lower - observed
    } else {
        0.0
    };
    let upper_penalty = if observed > upper {
        observed - upper
    } else {
        0.0
    };
    let score = (upper - lower) + (2.0 / alpha) * (lower_penalty + upper_penalty);
    if score.is_finite() {
        Ok(score)
    } else {
        Err(ForecastError::NumericOverflow)
    }
}

/// Weighted interval score from central 50%, 80% and 95% empirical intervals.
///
/// WIS combines median absolute error with weighted interval scores. It rewards
/// calibrated, sharp intervals and penalizes observations outside them. The quantiles
/// use the same documented nearest-rank rule as the forecast receipt.
pub fn weighted_interval_score(samples: &[f64], observed: f64) -> Result<f64, ForecastError> {
    if samples.is_empty() || samples.len() > 1_000_000 {
        return Err(ForecastError::InvalidEnsembleSize);
    }
    if !observed.is_finite() || samples.iter().any(|value| !value.is_finite()) {
        return Err(ForecastError::NonFiniteValue);
    }

    let median = empirical_quantile(samples, 0.5)?;
    let interval_levels = [(0.5, 0.25, 0.75), (0.2, 0.1, 0.9), (0.05, 0.025, 0.975)];
    let mut weighted_sum = 0.5 * (observed - median).abs();
    for (alpha, lower_probability, upper_probability) in interval_levels {
        let lower = empirical_quantile(samples, lower_probability)?;
        let upper = empirical_quantile(samples, upper_probability)?;
        let score = interval_score(lower, upper, observed, alpha)?;
        weighted_sum += (alpha / 2.0) * score;
    }
    let result = weighted_sum / 3.5;
    if result.is_finite() {
        Ok(result)
    } else {
        Err(ForecastError::NumericOverflow)
    }
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
    sorted
        .last()
        .copied()
        .ok_or(ForecastError::InvalidEnsembleSize)
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
    let baseline_wis = weighted_interval_score(&case.baseline_samples, case.observed)?;
    if case.model_samples.is_empty() {
        return Ok(ForecastCaseScore {
            model_crps: None,
            baseline_crps,
            model_wis: None,
            baseline_wis,
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
    let model_wis = weighted_interval_score(&case.model_samples, case.observed)?;
    let model_mean = ensemble_mean(&case.model_samples)?;
    Ok(ForecastCaseScore {
        model_crps: Some(model_crps),
        baseline_crps,
        model_wis: Some(model_wis),
        baseline_wis,
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
    let mut model_wis_total = 0.0;
    let mut paired_baseline_wis_total = 0.0;
    let mut all_baseline_wis_total = 0.0;
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
        all_baseline_wis_total += score.baseline_wis;
        if let Some(model_crps) = score.model_crps {
            forecast_case_count += 1;
            model_crps_total += model_crps;
            paired_baseline_crps_total += score.baseline_crps;
            model_wis_total += score.model_wis.ok_or(ForecastError::NumericOverflow)?;
            paired_baseline_wis_total += score.baseline_wis;
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
        count_as_f64(forecast_case_count)? / count_as_f64(eligible_case_count)?;
    let mean_baseline_crps_all = all_baseline_crps_total / count_as_f64(eligible_case_count)?;
    let mean_baseline_wis_all = all_baseline_wis_total / count_as_f64(eligible_case_count)?;

    if forecast_case_count == 0 {
        return Ok(ForecastSummary {
            eligible_case_count,
            forecast_case_count,
            abstained_case_count,
            forecast_coverage_fraction,
            mean_model_crps: None,
            mean_baseline_crps_paired: None,
            mean_baseline_crps_all,
            mean_model_wis: None,
            mean_baseline_wis_paired: None,
            mean_baseline_wis_all,
            wis_skill_score: None,
            crps_skill_score: None,
            mean_absolute_error: None,
            interval_coverage_50: None,
            interval_coverage_80: None,
            interval_coverage_95: None,
            mean_interval_width_80: None,
            mean_interval_width_95: None,
            disposition: "NO_FORECAST_COVERAGE",
        });
    }

    let paired_count = count_as_f64(forecast_case_count)?;
    let model_mean = model_crps_total / paired_count;
    let baseline_mean = paired_baseline_crps_total / paired_count;
    let crps_skill_score = if baseline_mean > 0.0 {
        Some(1.0 - model_mean / baseline_mean)
    } else {
        None
    };
    let model_wis_mean = model_wis_total / paired_count;
    let baseline_wis_mean = paired_baseline_wis_total / paired_count;
    let wis_skill_score = if baseline_wis_mean > 0.0 {
        Some(1.0 - model_wis_mean / baseline_wis_mean)
    } else {
        None
    };

    Ok(ForecastSummary {
        eligible_case_count,
        forecast_case_count,
        abstained_case_count,
        forecast_coverage_fraction,
        mean_model_crps: Some(model_mean),
        mean_baseline_crps_paired: Some(baseline_mean),
        mean_baseline_crps_all,
        mean_model_wis: Some(model_wis_mean),
        mean_baseline_wis_paired: Some(baseline_wis_mean),
        mean_baseline_wis_all,
        wis_skill_score,
        crps_skill_score,
        mean_absolute_error: Some(absolute_error_total / paired_count),
        interval_coverage_50: Some(count_as_f64(coverage_50_count)? / paired_count),
        interval_coverage_80: Some(count_as_f64(coverage_80_count)? / paired_count),
        interval_coverage_95: Some(count_as_f64(coverage_95_count)? / paired_count),
        mean_interval_width_80: Some(width_80_total / paired_count),
        mean_interval_width_95: Some(width_95_total / paired_count),
        disposition: "SCORED_WITH_EXPLICIT_FORECAST_COVERAGE",
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectiveRiskCase {
    pub forecast_id: String,
    /// Declared confidence / acceptance score in [0, 1], higher ranks first.
    pub confidence_score: f64,
    pub forecast_samples: Vec<f64>,
    pub observed: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RiskCoveragePoint {
    pub selected_count: usize,
    pub coverage_fraction: f64,
    pub mean_crps_risk: f64,
}

/// Return the full deterministic risk–coverage curve.
///
/// Cases are ranked by descending confidence score. Ties are broken by forecast ID so
/// input order cannot change the curve. Risk is empirical CRPS on the selected prefix.
/// This is an offline benchmark diagnostic: every eligible case must provide a candidate
/// forecast, including cases that would have been operationally abstained.
pub fn selective_risk_coverage_curve(
    cases: &[SelectiveRiskCase],
) -> Result<Vec<RiskCoveragePoint>, ForecastError> {
    if cases.is_empty() {
        return Err(ForecastError::InvalidEnsembleSize);
    }
    let mut ids = BTreeSet::new();
    let mut ranked = Vec::with_capacity(cases.len());
    for case in cases {
        if case.forecast_id.trim().is_empty() {
            return Err(ForecastError::EmptyForecastIdentity);
        }
        if !ids.insert(case.forecast_id.as_str()) {
            return Err(ForecastError::DuplicateForecastIdentity);
        }
        if !case.confidence_score.is_finite() || !(0.0..=1.0).contains(&case.confidence_score) {
            return Err(ForecastError::InvalidConfidenceScore);
        }
        let crps = empirical_crps(&case.forecast_samples, case.observed)?;
        ranked.push((case.forecast_id.as_str(), case.confidence_score, crps));
    }
    ranked.sort_by(|left, right| right.1.total_cmp(&left.1).then_with(|| left.0.cmp(right.0)));

    let denominator = count_as_f64(ranked.len())?;
    let mut cumulative_risk = 0.0;
    let mut curve = Vec::with_capacity(ranked.len());
    for (index, (_, _, risk)) in ranked.iter().enumerate() {
        cumulative_risk += risk;
        let selected_count = index + 1;
        let selected_as_f64 = count_as_f64(selected_count)?;
        curve.push(RiskCoveragePoint {
            selected_count,
            coverage_fraction: selected_as_f64 / denominator,
            mean_crps_risk: cumulative_risk / selected_as_f64,
        });
    }
    Ok(curve)
}

#[derive(Clone, Debug, PartialEq)]
pub struct BinaryEventForecast {
    pub forecast_id: String,
    pub event_name: String,
    pub horizon_days: u32,
    /// None means the forecast abstained; it is not interpreted as probability zero.
    pub event_probability: Option<f64>,
    pub observed_event: bool,
    pub baseline_probability: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BinaryEventCaseScore {
    pub model_brier: Option<f64>,
    pub baseline_brier: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReliabilityBin {
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub case_count: usize,
    pub mean_forecast_probability: Option<f64>,
    pub observed_event_frequency: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrierSummary {
    pub eligible_case_count: usize,
    pub forecast_case_count: usize,
    pub abstained_case_count: usize,
    pub forecast_coverage_fraction: f64,
    pub mean_model_brier: Option<f64>,
    pub mean_baseline_brier_paired: Option<f64>,
    pub mean_baseline_brier_all: f64,
    /// Positive is better: 1 - model Brier / paired baseline Brier.
    pub brier_skill_score: Option<f64>,
    pub reliability_bins: Vec<ReliabilityBin>,
    pub disposition: &'static str,
}

/// Brier score for a Bernoulli event probability and a binary observation.
pub fn brier_score(probability: f64, observed_event: bool) -> Result<f64, ForecastError> {
    if !probability.is_finite() || !(0.0..=1.0).contains(&probability) {
        return Err(ForecastError::InvalidProbability);
    }
    let observed = if observed_event { 1.0 } else { 0.0 };
    Ok((probability - observed).powi(2))
}

pub fn score_binary_event_case(
    case: &BinaryEventForecast,
) -> Result<BinaryEventCaseScore, ForecastError> {
    if case.forecast_id.trim().is_empty() || case.event_name.trim().is_empty() {
        return Err(ForecastError::EmptyEventIdentity);
    }
    let baseline_brier = brier_score(case.baseline_probability, case.observed_event)?;
    let model_brier = match case.event_probability {
        Some(probability) => Some(brier_score(probability, case.observed_event)?),
        None => None,
    };
    Ok(BinaryEventCaseScore {
        model_brier,
        baseline_brier,
    })
}

fn reliability_bin_index(probability: f64) -> usize {
    if probability < 0.2 {
        0
    } else if probability < 0.4 {
        1
    } else if probability < 0.6 {
        2
    } else if probability < 0.8 {
        3
    } else {
        4
    }
}

/// Summarize binary event forecasts with five fixed-width reliability bins.
/// Abstentions count in eligible coverage but do not enter Brier or reliability-bin scores.
pub fn summarize_binary_event_forecasts(
    cases: &[BinaryEventForecast],
) -> Result<BrierSummary, ForecastError> {
    if cases.is_empty() {
        return Err(ForecastError::InvalidEnsembleSize);
    }
    let first = &cases[0];
    if first.event_name.trim().is_empty() {
        return Err(ForecastError::EmptyEventIdentity);
    }

    let mut ids = BTreeSet::new();
    let mut forecast_case_count = 0usize;
    let mut model_brier_total = 0.0;
    let mut paired_baseline_total = 0.0;
    let mut all_baseline_total = 0.0;
    let mut bin_counts = [0usize; 5];
    let mut bin_probability_totals = [0.0; 5];
    let mut bin_event_counts = [0usize; 5];

    for case in cases {
        if !ids.insert(case.forecast_id.as_str()) {
            return Err(ForecastError::DuplicateForecastIdentity);
        }
        if case.event_name != first.event_name || case.horizon_days != first.horizon_days {
            return Err(ForecastError::MixedEventSlice);
        }
        let score = score_binary_event_case(case)?;
        all_baseline_total += score.baseline_brier;
        if let (Some(model_brier), Some(probability)) = (score.model_brier, case.event_probability)
        {
            forecast_case_count += 1;
            model_brier_total += model_brier;
            paired_baseline_total += score.baseline_brier;
            let bin_index = reliability_bin_index(probability);
            bin_counts[bin_index] += 1;
            bin_probability_totals[bin_index] += probability;
            if case.observed_event {
                bin_event_counts[bin_index] += 1;
            }
        }
    }

    let eligible_case_count = cases.len();
    let abstained_case_count = eligible_case_count - forecast_case_count;
    let forecast_coverage_fraction =
        count_as_f64(forecast_case_count)? / count_as_f64(eligible_case_count)?;
    let mean_baseline_brier_all = all_baseline_total / count_as_f64(eligible_case_count)?;
    let mut reliability_bins = Vec::with_capacity(5);
    for (index, case_count) in bin_counts.iter().copied().enumerate() {
        let index_count = u32::try_from(index).map_err(|_| ForecastError::InvalidEnsembleSize)?;
        let lower_bound = f64::from(index_count) * 0.2;
        let upper_bound = if index == 4 { 1.0 } else { lower_bound + 0.2 };
        let count = count_as_f64(case_count)?;
        let mean_forecast_probability = if case_count == 0 {
            None
        } else {
            Some(bin_probability_totals[index] / count)
        };
        let observed_event_frequency = if case_count == 0 {
            None
        } else {
            Some(count_as_f64(bin_event_counts[index])? / count)
        };
        reliability_bins.push(ReliabilityBin {
            lower_bound,
            upper_bound,
            case_count,
            mean_forecast_probability,
            observed_event_frequency,
        });
    }

    if forecast_case_count == 0 {
        return Ok(BrierSummary {
            eligible_case_count,
            forecast_case_count,
            abstained_case_count,
            forecast_coverage_fraction,
            mean_model_brier: None,
            mean_baseline_brier_paired: None,
            mean_baseline_brier_all,
            brier_skill_score: None,
            reliability_bins,
            disposition: "NO_FORECAST_COVERAGE",
        });
    }

    let paired_count = count_as_f64(forecast_case_count)?;
    let model_mean = model_brier_total / paired_count;
    let baseline_mean = paired_baseline_total / paired_count;
    let brier_skill_score = if baseline_mean > 0.0 {
        Some(1.0 - model_mean / baseline_mean)
    } else {
        None
    };
    Ok(BrierSummary {
        eligible_case_count,
        forecast_case_count,
        abstained_case_count,
        forecast_coverage_fraction,
        mean_model_brier: Some(model_mean),
        mean_baseline_brier_paired: Some(baseline_mean),
        mean_baseline_brier_all,
        brier_skill_score,
        reliability_bins,
        disposition: "SCORED_WITH_EXPLICIT_EVENT_FORECAST_COVERAGE",
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
    fn risk_coverage_curve_ranks_high_confidence_forecasts_first() {
        let cases = [
            SelectiveRiskCase {
                forecast_id: "a".to_string(),
                confidence_score: 0.9,
                forecast_samples: vec![0.0, 1.0],
                observed: 0.5,
            },
            SelectiveRiskCase {
                forecast_id: "b".to_string(),
                confidence_score: 0.8,
                forecast_samples: vec![2.0, 2.0],
                observed: 3.0,
            },
            SelectiveRiskCase {
                forecast_id: "c".to_string(),
                confidence_score: 0.7,
                forecast_samples: vec![0.0, 2.0],
                observed: 1.0,
            },
        ];
        let curve = selective_risk_coverage_curve(&cases).expect("risk-coverage curve");
        assert_eq!(curve.len(), 3);
        assert_eq!(curve[0].coverage_fraction, 1.0 / 3.0);
        assert!((curve[0].mean_crps_risk - 0.25).abs() < 1e-12);
        assert!((curve[1].mean_crps_risk - 0.625).abs() < 1e-12);
    }

    #[test]
    fn risk_coverage_curve_is_input_order_invariant_with_id_tie_breaks() {
        let a = SelectiveRiskCase {
            forecast_id: "a".to_string(),
            confidence_score: 0.8,
            forecast_samples: vec![0.0],
            observed: 1.0,
        };
        let b = SelectiveRiskCase {
            forecast_id: "b".to_string(),
            confidence_score: 0.8,
            forecast_samples: vec![0.0],
            observed: 2.0,
        };
        let left = selective_risk_coverage_curve(&[a.clone(), b.clone()]).unwrap();
        let right = selective_risk_coverage_curve(&[b, a]).unwrap();
        assert_eq!(left, right);
    }

    #[test]
    fn risk_coverage_curve_rejects_out_of_range_confidence() {
        let result = selective_risk_coverage_curve(&[SelectiveRiskCase {
            forecast_id: "bad".to_string(),
            confidence_score: 1.1,
            forecast_samples: vec![0.0],
            observed: 0.0,
        }]);
        assert_eq!(result, Err(ForecastError::InvalidConfidenceScore));
    }

    #[test]
    fn brier_score_matches_known_binary_probabilities() {
        assert!((brier_score(0.8, true).unwrap() - 0.04).abs() < 1e-12);
        assert!((brier_score(0.1, false).unwrap() - 0.01).abs() < 1e-12);
    }

    #[test]
    fn brier_score_rejects_probabilities_outside_unit_interval() {
        assert_eq!(
            brier_score(-0.01, true),
            Err(ForecastError::InvalidProbability)
        );
        assert_eq!(
            brier_score(1.01, false),
            Err(ForecastError::InvalidProbability)
        );
        assert_eq!(
            brier_score(f64::NAN, true),
            Err(ForecastError::InvalidProbability)
        );
    }

    #[test]
    fn binary_event_summary_keeps_abstention_in_coverage_denominator() {
        let cases = [
            BinaryEventForecast {
                forecast_id: "a".to_string(),
                event_name: "disturbance".to_string(),
                horizon_days: 30,
                event_probability: Some(0.8),
                observed_event: true,
                baseline_probability: 0.5,
            },
            BinaryEventForecast {
                forecast_id: "b".to_string(),
                event_name: "disturbance".to_string(),
                horizon_days: 30,
                event_probability: Some(0.1),
                observed_event: false,
                baseline_probability: 0.5,
            },
            BinaryEventForecast {
                forecast_id: "c".to_string(),
                event_name: "disturbance".to_string(),
                horizon_days: 30,
                event_probability: None,
                observed_event: true,
                baseline_probability: 0.5,
            },
        ];
        let result = summarize_binary_event_forecasts(&cases).expect("Brier summary");
        assert_eq!(result.eligible_case_count, 3);
        assert_eq!(result.forecast_case_count, 2);
        assert_eq!(result.abstained_case_count, 1);
        assert!((result.forecast_coverage_fraction - (2.0 / 3.0)).abs() < 1e-12);
        assert_eq!(
            result
                .reliability_bins
                .iter()
                .map(|bin| bin.case_count)
                .sum::<usize>(),
            2
        );
    }

    #[test]
    fn binary_event_summary_has_no_brier_skill_when_all_abstain() {
        let result = summarize_binary_event_forecasts(&[BinaryEventForecast {
            forecast_id: "a".to_string(),
            event_name: "disturbance".to_string(),
            horizon_days: 30,
            event_probability: None,
            observed_event: true,
            baseline_probability: 0.5,
        }])
        .expect("all-abstention Brier summary");
        assert_eq!(result.forecast_coverage_fraction, 0.0);
        assert_eq!(result.mean_model_brier, None);
        assert_eq!(result.brier_skill_score, None);
        assert_eq!(result.disposition, "NO_FORECAST_COVERAGE");
    }

    #[test]
    fn binary_event_summary_rejects_duplicate_and_mixed_horizon_cases() {
        let first = BinaryEventForecast {
            forecast_id: "same".to_string(),
            event_name: "disturbance".to_string(),
            horizon_days: 30,
            event_probability: Some(0.5),
            observed_event: true,
            baseline_probability: 0.5,
        };
        let mut duplicate = first.clone();
        assert_eq!(
            summarize_binary_event_forecasts(&[first.clone(), duplicate.clone()]),
            Err(ForecastError::DuplicateForecastIdentity)
        );
        duplicate.forecast_id = "other".to_string();
        duplicate.horizon_days = 90;
        assert_eq!(
            summarize_binary_event_forecasts(&[first, duplicate]),
            Err(ForecastError::MixedEventSlice)
        );
    }

    #[test]
    fn weighted_interval_score_matches_two_member_hand_calculation() {
        let score = weighted_interval_score(&[0.0, 2.0], 1.0).expect("WIS");
        assert!((score - (1.25 / 3.5)).abs() < 1e-12);
    }

    #[test]
    fn weighted_interval_score_reduces_to_absolute_error_for_point_mass() {
        assert_eq!(weighted_interval_score(&[2.0], 5.0), Ok(3.0));
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
        assert_eq!(
            empirical_crps(&[], 1.0),
            Err(ForecastError::InvalidEnsembleSize)
        );
        assert_eq!(
            empirical_crps(&[f64::NAN], 1.0),
            Err(ForecastError::NonFiniteValue)
        );
    }

    #[test]
    fn score_forecast_case_preserves_abstention() {
        let score =
            score_forecast_case(&forecast("abstain", &[], &[0.0], 2.0)).expect("abstention score");
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
        let result = summarize_forecasts(&[forecast("a", &[1.0, 1.0], &[1.0], 1.0)])
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
