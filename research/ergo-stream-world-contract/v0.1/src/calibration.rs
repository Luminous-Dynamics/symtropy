//! Proper scoring and selective-risk metrics for transfer forecasts.
//!
//! A transfer learner should not be graded only on whether its proposed map
//! passes a hidden evaluator. It should also be graded on whether its
//! probability forecast is calibrated and whether abstention controls the
//! risk of the claims it elects to make.
//!
//! This module is an evaluator-side metric implementation. It does not
//! estimate the probabilities, choose a confidence threshold, or authorize
//! transfers. Probabilities use integer parts-per-million (ppm) so Brier and
//! binned calibration errors have exact integer numerators.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Probability scale: 1_000_000 ppm represents probability 1.0.
pub const PROBABILITY_SCALE_PPM_V0: u32 = 1_000_000;
/// Guardrail against accidental or adversarial allocation of huge bin arrays.
pub const MAX_CALIBRATION_BINS_V0: usize = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDecisionV0 {
    AssertTransfer,
    RejectTransfer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferForecastV0 {
    pub sample_id: u64,
    /// Forecast probability that the declared transfer claim is valid.
    pub probability_transfer_ppm: u32,
    /// None is an explicit abstention, not a negative transfer prediction.
    pub decision: Option<TransferDecisionV0>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferOutcomeV0 {
    pub sample_id: u64,
    /// Owner/evaluator grouping label, never learner-authored.
    pub evaluation_stratum_id: u16,
    /// Owner/evaluator truth for the exact declared transfer claim.
    pub transfer_qualified: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalibrationBinV0 {
    pub bin_index: usize,
    pub sample_count: usize,
    pub positive_count: usize,
    /// Sum of probability ppm values for exact aggregation.
    pub probability_sum_ppm: u128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectiveRiskPointV0 {
    /// Minimum confidence in the forecast's selected class, inclusive.
    pub confidence_threshold_ppm: u32,
    pub total_samples: usize,
    pub covered_samples: usize,
    pub errors: usize,
}

impl SelectiveRiskPointV0 {
    pub fn coverage(self) -> Option<f64> {
        (self.total_samples > 0).then(|| self.covered_samples as f64 / self.total_samples as f64)
    }

    pub fn selective_risk(self) -> Option<f64> {
        (self.covered_samples > 0).then(|| self.errors as f64 / self.covered_samples as f64)
    }

    /// Two-sided 95% Wilson interval for empirical coverage.
    ///
    /// This is an IID binomial reference interval. It is not a valid
    /// uncertainty bound for clustered or temporally dependent claims.
    pub fn coverage_wilson_95(self) -> Option<(f64, f64)> {
        wilson_interval_95(self.covered_samples, self.total_samples)
    }

    /// Two-sided 95% Wilson interval for selective risk.
    ///
    /// The denominator is the number of claims selected at this threshold.
    /// This is an IID binomial reference interval only; use campaign/block
    /// bootstrap or another cluster-aware method for dependent episodes.
    pub fn selective_risk_wilson_95(self) -> Option<(f64, f64)> {
        wilson_interval_95(self.errors, self.covered_samples)
    }
}

fn wilson_interval_95(successes: usize, trials: usize) -> Option<(f64, f64)> {
    if trials == 0 || successes > trials {
        return None;
    }

    const Z_95: f64 = 1.959_963_984_540_054;
    let n = trials as f64;
    let p = successes as f64 / n;
    let z_squared = Z_95 * Z_95;
    let denominator = 1.0 + z_squared / n;
    let centre = (p + z_squared / (2.0 * n)) / denominator;
    let half_width = Z_95 * ((p * (1.0 - p) / n) + z_squared / (4.0 * n * n)).sqrt() / denominator;

    Some((
        (centre - half_width).max(0.0),
        (centre + half_width).min(1.0),
    ))
}

#[derive(Debug, Clone, PartialEq)]
pub struct StratifiedTransferCalibrationReportV0 {
    /// Evaluator-owned, preregistered semantic/format/claim stratum.
    pub evaluation_stratum_id: u16,
    pub report: TransferCalibrationReportV0,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransferCalibrationReportV0 {
    pub sample_count: usize,
    /// Exact Brier score = numerator / denominator.
    pub brier_numerator: u128,
    pub brier_denominator: u128,
    /// Mean binary log loss in nats, clipped only to avoid log(0).
    pub mean_log_loss_nats: f64,
    pub log_loss_clipped_samples: usize,
    pub calibration_bins: Vec<CalibrationBinV0>,
    /// Exact fixed-bin ECE = numerator / (sample_count * probability scale).
    pub expected_calibration_error_numerator: u128,
    pub expected_calibration_error_denominator: u128,
    /// Risk/coverage values at the caller's preregistered thresholds.
    pub selective_risk_curve: Vec<SelectiveRiskPointV0>,
}

impl TransferCalibrationReportV0 {
    pub fn brier_score(&self) -> f64 {
        self.brier_numerator as f64 / self.brier_denominator as f64
    }

    pub fn expected_calibration_error(&self) -> f64 {
        self.expected_calibration_error_numerator as f64
            / self.expected_calibration_error_denominator as f64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferCalibrationErrorV0 {
    EmptyForecasts,
    EmptyThresholds,
    ZeroCalibrationBins,
    TooManyCalibrationBins,
    DuplicateSampleId,
    DuplicateOutcomeId,
    DuplicateThreshold,
    MissingOutcome,
    UnknownOutcome,
    ProbabilityOutOfRange,
    ThresholdOutOfRange,
    ArithmeticOverflow,
    EmptyConformalCalibration,
    AlphaOutOfRange,
    MixedCalibrationStrata,
}

impl fmt::Display for TransferCalibrationErrorV0 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyForecasts => "at least one transfer forecast is required",
            Self::EmptyThresholds => "at least one selective-risk threshold is required",
            Self::ZeroCalibrationBins => "calibration bin count must be positive",
            Self::TooManyCalibrationBins => "calibration bin count exceeds the v0 bound",
            Self::DuplicateSampleId => "forecast sample IDs must be unique",
            Self::DuplicateOutcomeId => "outcome sample IDs must be unique",
            Self::DuplicateThreshold => "selective-risk thresholds must be unique",
            Self::MissingOutcome => "a forecast has no matching evaluator outcome",
            Self::UnknownOutcome => "an evaluator outcome has no matching forecast",
            Self::ProbabilityOutOfRange => "transfer probability must be in 0..=1_000_000 ppm",
            Self::ThresholdOutOfRange => "confidence threshold must be in 0..=1_000_000 ppm",
            Self::ArithmeticOverflow => "calibration metric arithmetic overflowed",
            Self::EmptyConformalCalibration => {
                "split-conformal calibration requires at least one matched sample"
            }
            Self::AlphaOutOfRange => "conformal miscoverage alpha must be in 1..999999 ppm",
            Self::MixedCalibrationStrata => {
                "split-conformal calibration samples must belong to one evaluator-owned stratum"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for TransferCalibrationErrorV0 {}

/// Score transfer forecasts against evaluator-owned outcomes.
///
/// Brier score and fixed-bin ECE use exact integer numerators. Log loss is a
/// floating-point diagnostic with a one-ppm clip at 0/1. Selective risk uses
/// confidence in the *selected class*: p for AssertTransfer and (1-p) for
/// RejectTransfer. Abstentions never count as covered claims, but their
/// probability forecasts remain in the proper-scoring-rule metrics.
pub fn evaluate_transfer_calibration(
    forecasts: &[TransferForecastV0],
    outcomes: &[TransferOutcomeV0],
    calibration_bin_count: usize,
    confidence_thresholds_ppm: &[u32],
) -> Result<TransferCalibrationReportV0, TransferCalibrationErrorV0> {
    if forecasts.is_empty() {
        return Err(TransferCalibrationErrorV0::EmptyForecasts);
    }
    if confidence_thresholds_ppm.is_empty() {
        return Err(TransferCalibrationErrorV0::EmptyThresholds);
    }
    if calibration_bin_count == 0 {
        return Err(TransferCalibrationErrorV0::ZeroCalibrationBins);
    }
    if calibration_bin_count > MAX_CALIBRATION_BINS_V0 {
        return Err(TransferCalibrationErrorV0::TooManyCalibrationBins);
    }

    let mut forecast_ids = BTreeSet::new();
    for forecast in forecasts {
        if !forecast_ids.insert(forecast.sample_id) {
            return Err(TransferCalibrationErrorV0::DuplicateSampleId);
        }
        if forecast.probability_transfer_ppm > PROBABILITY_SCALE_PPM_V0 {
            return Err(TransferCalibrationErrorV0::ProbabilityOutOfRange);
        }
    }

    let mut outcome_by_id = std::collections::BTreeMap::new();
    for outcome in outcomes {
        if outcome_by_id
            .insert(outcome.sample_id, outcome.transfer_qualified)
            .is_some()
        {
            return Err(TransferCalibrationErrorV0::DuplicateOutcomeId);
        }
    }
    if forecasts
        .iter()
        .any(|forecast| !outcome_by_id.contains_key(&forecast.sample_id))
    {
        return Err(TransferCalibrationErrorV0::MissingOutcome);
    }
    if outcome_by_id
        .keys()
        .any(|sample_id| !forecast_ids.contains(sample_id))
    {
        return Err(TransferCalibrationErrorV0::UnknownOutcome);
    }

    let mut thresholds = BTreeSet::new();
    for threshold in confidence_thresholds_ppm {
        if *threshold > PROBABILITY_SCALE_PPM_V0 {
            return Err(TransferCalibrationErrorV0::ThresholdOutOfRange);
        }
        if !thresholds.insert(*threshold) {
            return Err(TransferCalibrationErrorV0::DuplicateThreshold);
        }
    }

    let mut bins = vec![
        CalibrationBinV0 {
            bin_index: 0,
            sample_count: 0,
            positive_count: 0,
            probability_sum_ppm: 0,
        };
        calibration_bin_count
    ];
    let mut brier_numerator = 0_u128;
    let mut log_loss_sum = 0.0_f64;
    let mut log_loss_clipped_samples = 0_usize;
    let mut decisions = Vec::with_capacity(forecasts.len());

    for forecast in forecasts {
        let truth = *outcome_by_id
            .get(&forecast.sample_id)
            .ok_or(TransferCalibrationErrorV0::MissingOutcome)?;
        let probability = u128::from(forecast.probability_transfer_ppm);
        let target = if truth {
            u128::from(PROBABILITY_SCALE_PPM_V0)
        } else {
            0
        };
        let error = probability.abs_diff(target);
        let squared_error = error
            .checked_mul(error)
            .ok_or(TransferCalibrationErrorV0::ArithmeticOverflow)?;
        brier_numerator = brier_numerator
            .checked_add(squared_error)
            .ok_or(TransferCalibrationErrorV0::ArithmeticOverflow)?;

        let clipped_probability = forecast
            .probability_transfer_ppm
            .clamp(1, PROBABILITY_SCALE_PPM_V0 - 1);
        if clipped_probability != forecast.probability_transfer_ppm {
            log_loss_clipped_samples += 1;
        }
        let p = f64::from(clipped_probability) / f64::from(PROBABILITY_SCALE_PPM_V0);
        log_loss_sum += if truth { -p.ln() } else { -(1.0 - p).ln() };

        let scaled_bin = u64::from(forecast.probability_transfer_ppm)
            .checked_mul(
                u64::try_from(calibration_bin_count)
                    .map_err(|_| TransferCalibrationErrorV0::ArithmeticOverflow)?,
            )
            .ok_or(TransferCalibrationErrorV0::ArithmeticOverflow)?;
        let bin_index = usize::try_from(scaled_bin / u64::from(PROBABILITY_SCALE_PPM_V0))
            .map_err(|_| TransferCalibrationErrorV0::ArithmeticOverflow)?
            .min(calibration_bin_count - 1);

        let bin = &mut bins[bin_index];
        bin.sample_count = bin
            .sample_count
            .checked_add(1)
            .ok_or(TransferCalibrationErrorV0::ArithmeticOverflow)?;
        if truth {
            bin.positive_count = bin
                .positive_count
                .checked_add(1)
                .ok_or(TransferCalibrationErrorV0::ArithmeticOverflow)?;
        }
        bin.probability_sum_ppm = bin
            .probability_sum_ppm
            .checked_add(probability)
            .ok_or(TransferCalibrationErrorV0::ArithmeticOverflow)?;

        let (confidence_ppm, predicted_transfer) = match forecast.decision {
            Some(TransferDecisionV0::AssertTransfer) => {
                (forecast.probability_transfer_ppm, Some(true))
            }
            Some(TransferDecisionV0::RejectTransfer) => (
                PROBABILITY_SCALE_PPM_V0 - forecast.probability_transfer_ppm,
                Some(false),
            ),
            None => (0, None),
        };
        decisions.push((confidence_ppm, predicted_transfer, truth));
    }

    let sample_count = forecasts.len();
    let brier_denominator = u128::try_from(sample_count)
        .map_err(|_| TransferCalibrationErrorV0::ArithmeticOverflow)?
        .checked_mul(u128::from(PROBABILITY_SCALE_PPM_V0).pow(2))
        .ok_or(TransferCalibrationErrorV0::ArithmeticOverflow)?;

    let mut ece_numerator = 0_u128;
    for bin in &bins {
        if bin.sample_count == 0 {
            continue;
        }
        let positive_mass = u128::try_from(bin.positive_count)
            .map_err(|_| TransferCalibrationErrorV0::ArithmeticOverflow)?
            .checked_mul(u128::from(PROBABILITY_SCALE_PPM_V0))
            .ok_or(TransferCalibrationErrorV0::ArithmeticOverflow)?;
        ece_numerator = ece_numerator
            .checked_add(bin.probability_sum_ppm.abs_diff(positive_mass))
            .ok_or(TransferCalibrationErrorV0::ArithmeticOverflow)?;
    }

    let ece_denominator = u128::try_from(sample_count)
        .map_err(|_| TransferCalibrationErrorV0::ArithmeticOverflow)?
        .checked_mul(u128::from(PROBABILITY_SCALE_PPM_V0))
        .ok_or(TransferCalibrationErrorV0::ArithmeticOverflow)?;

    let selective_risk_curve = confidence_thresholds_ppm
        .iter()
        .map(|threshold| {
            let mut covered_samples = 0_usize;
            let mut errors = 0_usize;
            for (confidence, predicted, truth) in &decisions {
                if let Some(predicted) = predicted
                    && confidence >= threshold
                {
                    covered_samples += 1;
                    if predicted != truth {
                        errors += 1;
                    }
                }
            }
            SelectiveRiskPointV0 {
                confidence_threshold_ppm: *threshold,
                total_samples: sample_count,
                covered_samples,
                errors,
            }
        })
        .collect();

    Ok(TransferCalibrationReportV0 {
        sample_count,
        brier_numerator,
        brier_denominator,
        mean_log_loss_nats: log_loss_sum / sample_count as f64,
        log_loss_clipped_samples,
        calibration_bins: bins,
        expected_calibration_error_numerator: ece_numerator,
        expected_calibration_error_denominator: ece_denominator,
        selective_risk_curve,
    })
}

/// Evaluate calibration separately for evaluator-owned strata.
///
/// The strata label each outcome using information unavailable to the
/// candidate at forecast time (for example: representation-only shift,
/// semantic/mechanism shift, combined shift, or a particular claim family).
/// Each stratum receives its own Brier/log-loss/ECE and selective-risk report.
/// The full forecast/outcome set is validated first, so missing, duplicate or
/// orphan records cannot disappear by being excluded from a group.
pub fn evaluate_transfer_calibration_by_stratum(
    forecasts: &[TransferForecastV0],
    outcomes: &[TransferOutcomeV0],
    calibration_bin_count: usize,
    confidence_thresholds_ppm: &[u32],
) -> Result<Vec<StratifiedTransferCalibrationReportV0>, TransferCalibrationErrorV0> {
    // Validate the complete cohort before grouping to prevent silent loss of
    // unmatched samples or invalid strata.
    let _ = evaluate_transfer_calibration(
        forecasts,
        outcomes,
        calibration_bin_count,
        confidence_thresholds_ppm,
    )?;

    let forecasts_by_id = forecasts
        .iter()
        .map(|forecast| (forecast.sample_id, *forecast))
        .collect::<BTreeMap<_, _>>();
    let mut outcomes_by_stratum = BTreeMap::<u16, Vec<TransferOutcomeV0>>::new();
    for outcome in outcomes {
        outcomes_by_stratum
            .entry(outcome.evaluation_stratum_id)
            .or_default()
            .push(*outcome);
    }

    outcomes_by_stratum
        .into_iter()
        .map(|(evaluation_stratum_id, stratum_outcomes)| {
            let stratum_forecasts = stratum_outcomes
                .iter()
                .map(|outcome| {
                    forecasts_by_id
                        .get(&outcome.sample_id)
                        .copied()
                        .ok_or(TransferCalibrationErrorV0::MissingOutcome)
                })
                .collect::<Result<Vec<_>, _>>()?;
            let report = evaluate_transfer_calibration(
                &stratum_forecasts,
                &stratum_outcomes,
                calibration_bin_count,
                confidence_thresholds_ppm,
            )?;
            Ok(StratifiedTransferCalibrationReportV0 {
                evaluation_stratum_id,
                report,
            })
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitConformalBinaryTransferV0 {
    pub evaluation_stratum_id: u16,
    pub alpha_ppm: u32,
    pub calibration_count: usize,
    /// One-based finite-sample order statistic rank. calibration_count + 1
    /// denotes the conservative +infinity fallback, represented by scale.
    pub quantile_rank: usize,
    pub quantile_nonconformity_ppm: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferPredictionSetV0 {
    pub sample_id: u64,
    pub include_transfer: bool,
    pub include_non_transfer: bool,
    /// A singleton prediction set can be turned into a decision. Both labels
    /// or neither label produce no decision and must not be silently forced.
    pub decision: Option<TransferDecisionV0>,
}

impl SplitConformalBinaryTransferV0 {
    pub fn predict(
        self,
        sample_id: u64,
        probability_transfer_ppm: u32,
    ) -> Result<TransferPredictionSetV0, TransferCalibrationErrorV0> {
        if probability_transfer_ppm > PROBABILITY_SCALE_PPM_V0 {
            return Err(TransferCalibrationErrorV0::ProbabilityOutOfRange);
        }
        let transfer_score = PROBABILITY_SCALE_PPM_V0 - probability_transfer_ppm;
        let non_transfer_score = probability_transfer_ppm;
        let include_transfer = transfer_score <= self.quantile_nonconformity_ppm;
        let include_non_transfer = non_transfer_score <= self.quantile_nonconformity_ppm;
        let decision = match (include_transfer, include_non_transfer) {
            (true, false) => Some(TransferDecisionV0::AssertTransfer),
            (false, true) => Some(TransferDecisionV0::RejectTransfer),
            (true, true) | (false, false) => None,
        };
        Ok(TransferPredictionSetV0 {
            sample_id,
            include_transfer,
            include_non_transfer,
            decision,
        })
    }
}

/// Fit split conformal for the binary label "this exact scoped transfer claim
/// qualifies". Use only an independent calibration cohort from one
/// evaluator-owned stratum. The finite-sample marginal guarantee requires
/// exchangeability between this calibration cohort and future claims in the
/// same admitted stratum. It does not imply conditional coverage or validity
/// under arbitrary distribution shift.
pub fn fit_split_conformal_binary_transfer(
    calibration_forecasts: &[TransferForecastV0],
    calibration_outcomes: &[TransferOutcomeV0],
    evaluation_stratum_id: u16,
    alpha_ppm: u32,
) -> Result<SplitConformalBinaryTransferV0, TransferCalibrationErrorV0> {
    if calibration_forecasts.is_empty() || calibration_outcomes.is_empty() {
        return Err(TransferCalibrationErrorV0::EmptyConformalCalibration);
    }
    if alpha_ppm == 0 || alpha_ppm >= PROBABILITY_SCALE_PPM_V0 {
        return Err(TransferCalibrationErrorV0::AlphaOutOfRange);
    }

    let mut forecast_by_id = BTreeMap::new();
    for forecast in calibration_forecasts {
        if forecast.probability_transfer_ppm > PROBABILITY_SCALE_PPM_V0 {
            return Err(TransferCalibrationErrorV0::ProbabilityOutOfRange);
        }
        if forecast_by_id
            .insert(forecast.sample_id, *forecast)
            .is_some()
        {
            return Err(TransferCalibrationErrorV0::DuplicateSampleId);
        }
    }

    let mut outcomes_by_id = BTreeMap::new();
    for outcome in calibration_outcomes {
        if outcome.evaluation_stratum_id != evaluation_stratum_id {
            return Err(TransferCalibrationErrorV0::MixedCalibrationStrata);
        }
        if outcomes_by_id.insert(outcome.sample_id, *outcome).is_some() {
            return Err(TransferCalibrationErrorV0::DuplicateOutcomeId);
        }
    }

    if forecast_by_id.len() != outcomes_by_id.len() {
        return Err(TransferCalibrationErrorV0::MissingOutcome);
    }

    let mut nonconformity_scores = Vec::with_capacity(calibration_forecasts.len());
    for (sample_id, outcome) in &outcomes_by_id {
        let forecast = forecast_by_id
            .get(sample_id)
            .ok_or(TransferCalibrationErrorV0::MissingOutcome)?;
        let score = if outcome.transfer_qualified {
            PROBABILITY_SCALE_PPM_V0 - forecast.probability_transfer_ppm
        } else {
            forecast.probability_transfer_ppm
        };
        nonconformity_scores.push(score);
    }
    if forecast_by_id
        .keys()
        .any(|sample_id| !outcomes_by_id.contains_key(sample_id))
    {
        return Err(TransferCalibrationErrorV0::UnknownOutcome);
    }

    nonconformity_scores.sort_unstable();
    let calibration_count = nonconformity_scores.len();
    let n_plus_one = u128::try_from(calibration_count)
        .map_err(|_| TransferCalibrationErrorV0::ArithmeticOverflow)?
        .checked_add(1)
        .ok_or(TransferCalibrationErrorV0::ArithmeticOverflow)?;
    let target_numerator = n_plus_one
        .checked_mul(u128::from(PROBABILITY_SCALE_PPM_V0 - alpha_ppm))
        .ok_or(TransferCalibrationErrorV0::ArithmeticOverflow)?;
    let rank = target_numerator
        .checked_add(u128::from(PROBABILITY_SCALE_PPM_V0 - 1))
        .ok_or(TransferCalibrationErrorV0::ArithmeticOverflow)?
        / u128::from(PROBABILITY_SCALE_PPM_V0);
    let quantile_rank =
        usize::try_from(rank).map_err(|_| TransferCalibrationErrorV0::ArithmeticOverflow)?;

    let quantile_nonconformity_ppm = if quantile_rank > calibration_count {
        PROBABILITY_SCALE_PPM_V0
    } else {
        nonconformity_scores[quantile_rank - 1]
    };

    Ok(SplitConformalBinaryTransferV0 {
        evaluation_stratum_id,
        alpha_ppm,
        calibration_count,
        quantile_rank,
        quantile_nonconformity_ppm,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn forecast(
        sample_id: u64,
        probability_transfer_ppm: u32,
        decision: Option<TransferDecisionV0>,
    ) -> TransferForecastV0 {
        TransferForecastV0 {
            sample_id,
            probability_transfer_ppm,
            decision,
        }
    }

    fn conformal_calibration_fixture() -> (Vec<TransferForecastV0>, Vec<TransferOutcomeV0>) {
        let forecasts = vec![
            forecast(201, 900_000, None),
            forecast(202, 200_000, None),
            forecast(203, 500_000, None),
        ];
        let outcomes = vec![
            TransferOutcomeV0 {
                sample_id: 201,
                evaluation_stratum_id: 7,
                transfer_qualified: true,
            },
            TransferOutcomeV0 {
                sample_id: 202,
                evaluation_stratum_id: 7,
                transfer_qualified: false,
            },
            TransferOutcomeV0 {
                sample_id: 203,
                evaluation_stratum_id: 7,
                transfer_qualified: false,
            },
        ];
        (forecasts, outcomes)
    }

    #[test]
    fn split_conformal_fits_finite_sample_order_statistic_and_predicts_singleton() {
        let (forecasts, outcomes) = conformal_calibration_fixture();
        let calibrator =
            fit_split_conformal_binary_transfer(&forecasts, &outcomes, 7, 250_000).unwrap();

        assert_eq!(calibrator.calibration_count, 3);
        assert_eq!(calibrator.quantile_rank, 3);
        assert_eq!(calibrator.quantile_nonconformity_ppm, 500_000);

        let predicted = calibrator.predict(204, 900_000).unwrap();
        assert!(predicted.include_transfer);
        assert!(!predicted.include_non_transfer);
        assert_eq!(predicted.decision, Some(TransferDecisionV0::AssertTransfer));
    }

    #[test]
    fn split_conformal_returns_both_labels_when_calibration_is_too_small_for_rank() {
        let forecasts = [forecast(301, 900_000, None)];
        let outcomes = [TransferOutcomeV0 {
            sample_id: 301,
            evaluation_stratum_id: 2,
            transfer_qualified: true,
        }];
        let calibrator =
            fit_split_conformal_binary_transfer(&forecasts, &outcomes, 2, 100_000).unwrap();

        assert_eq!(calibrator.quantile_rank, 2);
        assert_eq!(
            calibrator.quantile_nonconformity_ppm,
            PROBABILITY_SCALE_PPM_V0
        );
        let predicted = calibrator.predict(302, 900_000).unwrap();
        assert!(predicted.include_transfer);
        assert!(predicted.include_non_transfer);
        assert_eq!(predicted.decision, None);
    }

    #[test]
    fn split_conformal_rejects_mixed_strata_and_invalid_alpha() {
        let (forecasts, outcomes) = conformal_calibration_fixture();
        assert_eq!(
            fit_split_conformal_binary_transfer(&forecasts, &outcomes, 8, 100_000),
            Err(TransferCalibrationErrorV0::MixedCalibrationStrata)
        );
        assert_eq!(
            fit_split_conformal_binary_transfer(&forecasts, &outcomes, 7, 0),
            Err(TransferCalibrationErrorV0::AlphaOutOfRange)
        );
        assert_eq!(
            fit_split_conformal_binary_transfer(&forecasts, &outcomes, 7, 1_000_000),
            Err(TransferCalibrationErrorV0::AlphaOutOfRange)
        );
    }

    #[test]
    fn split_conformal_rejects_missing_and_duplicate_calibration_records() {
        let (forecasts, outcomes) = conformal_calibration_fixture();
        assert_eq!(
            fit_split_conformal_binary_transfer(&forecasts[..2], &outcomes, 7, 100_000),
            Err(TransferCalibrationErrorV0::MissingOutcome)
        );
        let duplicate = [forecasts[0], forecasts[0]];
        assert_eq!(
            fit_split_conformal_binary_transfer(&duplicate, &outcomes[..1], 7, 100_000),
            Err(TransferCalibrationErrorV0::DuplicateSampleId)
        );
    }

    #[test]
    fn split_conformal_predict_rejects_probability_out_of_range() {
        let (forecasts, outcomes) = conformal_calibration_fixture();
        let calibrator =
            fit_split_conformal_binary_transfer(&forecasts, &outcomes, 7, 100_000).unwrap();
        assert_eq!(
            calibrator.predict(999, 1_000_001),
            Err(TransferCalibrationErrorV0::ProbabilityOutOfRange)
        );
    }

    #[test]
    fn zero_observed_selective_errors_does_not_mean_zero_risk() {
        let point = SelectiveRiskPointV0 {
            confidence_threshold_ppm: 900_000,
            total_samples: 100,
            covered_samples: 5,
            errors: 0,
        };
        let (lower, upper) = point.selective_risk_wilson_95().unwrap();

        assert_eq!(lower, 0.0);
        assert!(upper > 0.0);
        assert!(upper < 1.0);
    }

    #[test]
    fn selective_risk_and_coverage_have_separate_denominators() {
        let point = SelectiveRiskPointV0 {
            confidence_threshold_ppm: 800_000,
            total_samples: 20,
            covered_samples: 5,
            errors: 1,
        };
        let (coverage_lower, coverage_upper) = point.coverage_wilson_95().unwrap();
        let (risk_lower, risk_upper) = point.selective_risk_wilson_95().unwrap();

        assert!(coverage_lower < 0.25 && coverage_upper > 0.25);
        assert!(risk_lower < 0.2 && risk_upper > 0.2);
        assert_ne!((coverage_lower, coverage_upper), (risk_lower, risk_upper));
    }

    #[test]
    fn empty_selective_denominator_has_no_risk_interval() {
        let point = SelectiveRiskPointV0 {
            confidence_threshold_ppm: 1_000_000,
            total_samples: 10,
            covered_samples: 0,
            errors: 0,
        };

        assert_eq!(point.selective_risk_wilson_95(), None);
        assert!(point.coverage_wilson_95().is_some());
    }

    #[test]
    fn stratified_calibration_surfaces_domain_specific_failure_hidden_by_pooled_scores() {
        let forecasts = [
            forecast(101, 1_000_000, Some(TransferDecisionV0::AssertTransfer)),
            forecast(102, 950_000, Some(TransferDecisionV0::AssertTransfer)),
            forecast(103, 0, Some(TransferDecisionV0::RejectTransfer)),
        ];
        let outcomes = [
            TransferOutcomeV0 {
                sample_id: 101,
                evaluation_stratum_id: 0,
                transfer_qualified: true,
            },
            TransferOutcomeV0 {
                sample_id: 102,
                evaluation_stratum_id: 3,
                transfer_qualified: false,
            },
            TransferOutcomeV0 {
                sample_id: 103,
                evaluation_stratum_id: 0,
                transfer_qualified: false,
            },
        ];

        let reports =
            evaluate_transfer_calibration_by_stratum(&forecasts, &outcomes, 10, &[0, 900_000])
                .unwrap();

        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0].evaluation_stratum_id, 0);
        assert_eq!(reports[0].report.sample_count, 2);
        assert_eq!(reports[0].report.brier_numerator, 0);
        assert_eq!(reports[1].evaluation_stratum_id, 3);
        assert_eq!(reports[1].report.sample_count, 1);
        assert_eq!(
            reports[1].report.selective_risk_curve[1].selective_risk(),
            Some(1.0)
        );
    }

    #[test]
    fn perfect_extreme_forecasts_have_zero_brier_and_ece() {
        let forecasts = [
            forecast(1, 1_000_000, Some(TransferDecisionV0::AssertTransfer)),
            forecast(2, 0, Some(TransferDecisionV0::RejectTransfer)),
            forecast(3, 1_000_000, Some(TransferDecisionV0::AssertTransfer)),
            forecast(4, 0, Some(TransferDecisionV0::RejectTransfer)),
        ];
        let outcomes = [
            TransferOutcomeV0 {
                sample_id: 1,
                evaluation_stratum_id: 0,
                transfer_qualified: true,
            },
            TransferOutcomeV0 {
                sample_id: 2,
                evaluation_stratum_id: 0,
                transfer_qualified: false,
            },
            TransferOutcomeV0 {
                sample_id: 3,
                evaluation_stratum_id: 0,
                transfer_qualified: true,
            },
            TransferOutcomeV0 {
                sample_id: 4,
                evaluation_stratum_id: 0,
                transfer_qualified: false,
            },
        ];

        let report = evaluate_transfer_calibration(&forecasts, &outcomes, 10, &[0, 900_000])
            .expect("well-formed forecast set");

        assert_eq!(report.brier_numerator, 0);
        assert_eq!(report.expected_calibration_error_numerator, 0);
        assert_eq!(report.selective_risk_curve[0].covered_samples, 4);
        assert_eq!(report.selective_risk_curve[1].covered_samples, 4);
    }

    #[test]
    fn equal_width_bins_include_exact_lower_boundaries() {
        let forecasts = [forecast(1, 100_000, None), forecast(2, 1_000_000, None)];
        let outcomes = [
            TransferOutcomeV0 {
                sample_id: 1,
                evaluation_stratum_id: 0,
                transfer_qualified: false,
            },
            TransferOutcomeV0 {
                sample_id: 2,
                evaluation_stratum_id: 0,
                transfer_qualified: true,
            },
        ];

        let report = evaluate_transfer_calibration(&forecasts, &outcomes, 10, &[0]).unwrap();

        assert_eq!(report.calibration_bins[0].sample_count, 0);
        assert_eq!(report.calibration_bins[1].sample_count, 1);
        assert_eq!(report.calibration_bins[9].sample_count, 1);
    }

    #[test]
    fn overconfident_false_transfer_is_penalized_by_proper_scores() {
        let forecasts = [forecast(
            1,
            950_000,
            Some(TransferDecisionV0::AssertTransfer),
        )];
        let outcomes = [TransferOutcomeV0 {
            sample_id: 1,
            evaluation_stratum_id: 0,
            transfer_qualified: false,
        }];

        let report = evaluate_transfer_calibration(&forecasts, &outcomes, 10, &[0]).unwrap();

        assert_eq!(report.brier_numerator, u128::from(950_000_u32).pow(2));
        assert!(report.mean_log_loss_nats > 2.9);
        assert_eq!(report.selective_risk_curve[0].errors, 1);
        assert_eq!(report.selective_risk_curve[0].selective_risk(), Some(1.0));
    }

    #[test]
    fn abstention_reduces_coverage_without_being_counted_as_a_negative() {
        let forecasts = [
            forecast(1, 950_000, Some(TransferDecisionV0::AssertTransfer)),
            forecast(2, 550_000, Some(TransferDecisionV0::AssertTransfer)),
            forecast(3, 50_000, Some(TransferDecisionV0::RejectTransfer)),
            forecast(4, 500_000, None),
        ];
        let outcomes = [
            TransferOutcomeV0 {
                sample_id: 1,
                evaluation_stratum_id: 0,
                transfer_qualified: true,
            },
            TransferOutcomeV0 {
                sample_id: 2,
                evaluation_stratum_id: 0,
                transfer_qualified: false,
            },
            TransferOutcomeV0 {
                sample_id: 3,
                evaluation_stratum_id: 0,
                transfer_qualified: false,
            },
            TransferOutcomeV0 {
                sample_id: 4,
                evaluation_stratum_id: 0,
                transfer_qualified: true,
            },
        ];

        let report =
            evaluate_transfer_calibration(&forecasts, &outcomes, 10, &[0, 900_000]).unwrap();

        let all_decisions = report.selective_risk_curve[0];
        let high_confidence = report.selective_risk_curve[1];

        assert_eq!(all_decisions.covered_samples, 3);
        assert_eq!(all_decisions.errors, 1);
        assert_eq!(all_decisions.selective_risk(), Some(1.0 / 3.0));
        assert_eq!(high_confidence.covered_samples, 2);
        assert_eq!(high_confidence.errors, 0);
        assert_eq!(high_confidence.selective_risk(), Some(0.0));
        assert_eq!(high_confidence.coverage(), Some(0.5));
    }

    #[test]
    fn probability_and_decision_confidence_are_not_conflated() {
        let forecasts = [
            forecast(1, 50_000, Some(TransferDecisionV0::RejectTransfer)),
            forecast(2, 50_000, Some(TransferDecisionV0::AssertTransfer)),
        ];
        let outcomes = [
            TransferOutcomeV0 {
                sample_id: 1,
                evaluation_stratum_id: 0,
                transfer_qualified: true,
            },
            TransferOutcomeV0 {
                sample_id: 2,
                evaluation_stratum_id: 0,
                transfer_qualified: false,
            },
        ];

        let report = evaluate_transfer_calibration(&forecasts, &outcomes, 2, &[900_000]).unwrap();

        assert_eq!(report.selective_risk_curve[0].covered_samples, 1);
        assert_eq!(report.selective_risk_curve[0].errors, 1);
    }

    #[test]
    fn rejects_unmatched_duplicate_and_out_of_range_data() {
        let valid = [forecast(1, 500_000, None)];
        let outcome = [TransferOutcomeV0 {
            sample_id: 1,
            evaluation_stratum_id: 0,
            transfer_qualified: true,
        }];
        assert_eq!(
            evaluate_transfer_calibration(&[], &outcome, 10, &[0]),
            Err(TransferCalibrationErrorV0::EmptyForecasts)
        );
        assert_eq!(
            evaluate_transfer_calibration(&valid, &outcome, 10, &[]),
            Err(TransferCalibrationErrorV0::EmptyThresholds)
        );

        let duplicate = [forecast(1, 500_000, None), forecast(1, 500_000, None)];
        assert_eq!(
            evaluate_transfer_calibration(&duplicate, &[outcome[0], outcome[0]], 10, &[0],),
            Err(TransferCalibrationErrorV0::DuplicateSampleId)
        );

        let invalid = [forecast(2, 1_000_001, None)];
        assert_eq!(
            evaluate_transfer_calibration(
                &invalid,
                &[TransferOutcomeV0 {
                    sample_id: 2,
                    evaluation_stratum_id: 0,
                    transfer_qualified: false,
                }],
                10,
                &[0],
            ),
            Err(TransferCalibrationErrorV0::ProbabilityOutOfRange)
        );
    }

    #[test]
    fn rejects_duplicate_or_out_of_range_thresholds_and_invalid_bins() {
        let forecasts = [forecast(1, 500_000, None)];
        let outcomes = [TransferOutcomeV0 {
            sample_id: 1,
            evaluation_stratum_id: 0,
            transfer_qualified: true,
        }];

        assert_eq!(
            evaluate_transfer_calibration(&forecasts, &outcomes, 10, &[1_000_001]),
            Err(TransferCalibrationErrorV0::ThresholdOutOfRange)
        );
        assert_eq!(
            evaluate_transfer_calibration(&forecasts, &outcomes, 10, &[0, 0]),
            Err(TransferCalibrationErrorV0::DuplicateThreshold)
        );
        assert_eq!(
            evaluate_transfer_calibration(&forecasts, &outcomes, 0, &[0]),
            Err(TransferCalibrationErrorV0::ZeroCalibrationBins)
        );
    }

    #[test]
    fn rejects_duplicate_outcome_ids() {
        let forecasts = [forecast(1, 500_000, None)];
        let outcomes = [
            TransferOutcomeV0 {
                sample_id: 1,
                evaluation_stratum_id: 0,
                transfer_qualified: true,
            },
            TransferOutcomeV0 {
                sample_id: 1,
                evaluation_stratum_id: 0,
                transfer_qualified: false,
            },
        ];

        assert_eq!(
            evaluate_transfer_calibration(&forecasts, &outcomes, 2, &[0]),
            Err(TransferCalibrationErrorV0::DuplicateOutcomeId)
        );
    }

    #[test]
    fn rejects_an_outcome_without_a_forecast() {
        let forecasts = [forecast(1, 500_000, None)];
        let outcomes = [
            TransferOutcomeV0 {
                sample_id: 1,
                evaluation_stratum_id: 0,
                transfer_qualified: true,
            },
            TransferOutcomeV0 {
                sample_id: 2,
                evaluation_stratum_id: 0,
                transfer_qualified: false,
            },
        ];

        assert_eq!(
            evaluate_transfer_calibration(&forecasts, &outcomes, 2, &[0]),
            Err(TransferCalibrationErrorV0::UnknownOutcome)
        );
    }
}
