// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Small split-conformal reference for calibrating ETA prediction intervals.
//!
//! The calibration cutoff is strict: only forecasts and actuals observed at or
//! before the cutoff are admitted. A finite-sample coverage interpretation
//! requires exchangeability of calibration and future evaluation examples;
//! this module does not claim that maritime time series are exchangeable.

use std::collections::{BTreeMap, BTreeSet};

pub const BASIS_POINTS: u16 = 10_000;
pub const NANOS_PER_SECOND: i128 = 1_000_000_000;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArrivalResidualSample {
    pub forecast_id: String,
    pub forecast_issued_at_utc_nanos: i128,
    pub predicted_arrival_utc_nanos: i128,
    pub actual_arrival_utc_nanos: i128,
    /// When the actual event value became available to the calibration system.
    pub actual_observed_at_utc_nanos: i128,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ArrivalEvaluationCase {
    pub case_id: String,
    /// Caller-defined group such as port, source, lead-time bin or time regime.
    pub stratum: String,
    pub forecast_issued_at_utc_nanos: i128,
    pub predicted_arrival_utc_nanos: i128,
    pub actual_arrival_utc_nanos: i128,
    pub actual_observed_at_utc_nanos: i128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CalibrationError {
    EmptyCalibrationSet,
    InvalidMiscoverage,
    EmptyForecastId,
    DuplicateForecastId,
    FutureForecastAtCutoff,
    ActualObservedAfterCutoff,
    ForecastIssuedAfterObservation,
    ForecastIssuedAfterActualEvent,
    ActualObservedBeforeActualEvent,
    EmptyEvaluationSet,
    EmptyEvaluationId,
    EmptyEvaluationStratum,
    DuplicateEvaluationId,
    EvaluationForecastNotAfterCutoff,
    EvaluationTruthNotAfterCutoff,
    ArithmeticOverflow,
    IntervalOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalibratedArrivalEnvelope {
    pub calibration_as_of_utc_nanos: i128,
    /// Nominal miscoverage probability in basis points, e.g. 1_000 = 10%.
    pub miscoverage_bps: u16,
    pub sample_count: usize,
    /// None is the conformal +infinity result: no finite interval is licensed
    /// at this sample size and target coverage. Callers must abstain or apply a
    /// separately declared fallback rather than silently invent a radius.
    pub radius_nanos: Option<u128>,
    /// Sorted absolute residuals are retained for audit and deterministic replay.
    pub sorted_absolute_residuals_nanos: Vec<u128>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PredictionInterval {
    pub lower_utc_nanos: i128,
    pub upper_utc_nanos: i128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoverageMetrics {
    pub evaluation_count: usize,
    pub interval_count: usize,
    pub abstention_count: usize,
    pub covered_count: usize,
    /// Empirical coverage among cases for which a finite interval was issued.
    pub conditional_coverage_bps: Option<u16>,
    /// Fraction of all evaluation cases receiving a finite interval.
    pub interval_issuance_rate_bps: u16,
    /// Mean width among issued finite intervals; this excludes abstained cases.
    pub mean_interval_width_nanos: Option<u128>,
}

fn validate_sample(
    sample: &ArrivalResidualSample,
    calibration_as_of_utc_nanos: i128,
    seen: &mut BTreeSet<String>,
) -> Result<(), CalibrationError> {
    if sample.forecast_id.trim().is_empty() {
        return Err(CalibrationError::EmptyForecastId);
    }
    if !seen.insert(sample.forecast_id.clone()) {
        return Err(CalibrationError::DuplicateForecastId);
    }
    if sample.forecast_issued_at_utc_nanos > calibration_as_of_utc_nanos {
        return Err(CalibrationError::FutureForecastAtCutoff);
    }
    if sample.actual_observed_at_utc_nanos > calibration_as_of_utc_nanos {
        return Err(CalibrationError::ActualObservedAfterCutoff);
    }
    if sample.forecast_issued_at_utc_nanos > sample.actual_observed_at_utc_nanos {
        return Err(CalibrationError::ForecastIssuedAfterObservation);
    }
    if sample.forecast_issued_at_utc_nanos >= sample.actual_arrival_utc_nanos {
        return Err(CalibrationError::ForecastIssuedAfterActualEvent);
    }
    if sample.actual_observed_at_utc_nanos < sample.actual_arrival_utc_nanos {
        return Err(CalibrationError::ActualObservedBeforeActualEvent);
    }
    Ok(())
}

/// Fit a split-conformal absolute-residual interval using a finite-sample rank.
///
/// For n calibration examples, the one-indexed order statistic is
/// ceil((n + 1) * (1 - alpha)). If this rank exceeds n, the conservative
/// quantile is +infinity and radius_nanos is None.
///
/// The marginal coverage guarantee applies only if calibration residuals and
/// future evaluation examples are exchangeable. Time drift, source mix changes,
/// selection bias and port-specific regime changes can break that assumption.
pub fn fit_split_conformal(
    samples: &[ArrivalResidualSample],
    calibration_as_of_utc_nanos: i128,
    miscoverage_bps: u16,
) -> Result<CalibratedArrivalEnvelope, CalibrationError> {
    if samples.is_empty() {
        return Err(CalibrationError::EmptyCalibrationSet);
    }
    if miscoverage_bps == 0 || miscoverage_bps >= BASIS_POINTS {
        return Err(CalibrationError::InvalidMiscoverage);
    }

    let mut seen = BTreeSet::new();
    let mut residuals = Vec::with_capacity(samples.len());
    for sample in samples {
        validate_sample(sample, calibration_as_of_utc_nanos, &mut seen)?;
        residuals.push(
            sample
                .actual_arrival_utc_nanos
                .abs_diff(sample.predicted_arrival_utc_nanos),
        );
    }
    residuals.sort_unstable();

    let n_plus_one = u128::try_from(samples.len())
        .map_err(|_| CalibrationError::ArithmeticOverflow)?
        .checked_add(1)
        .ok_or(CalibrationError::ArithmeticOverflow)?;
    let target_mass = u128::from(BASIS_POINTS - miscoverage_bps);
    let numerator = n_plus_one
        .checked_mul(target_mass)
        .ok_or(CalibrationError::ArithmeticOverflow)?;
    let rounded_numerator = numerator
        .checked_add(u128::from(BASIS_POINTS - 1))
        .ok_or(CalibrationError::ArithmeticOverflow)?;
    let rank = rounded_numerator / u128::from(BASIS_POINTS);
    let sample_count =
        u128::try_from(samples.len()).map_err(|_| CalibrationError::ArithmeticOverflow)?;
    let radius_nanos = if rank == 0 || rank > sample_count {
        None
    } else {
        let index = usize::try_from(rank - 1).map_err(|_| CalibrationError::ArithmeticOverflow)?;
        Some(residuals[index])
    };

    Ok(CalibratedArrivalEnvelope {
        calibration_as_of_utc_nanos,
        miscoverage_bps,
        sample_count: samples.len(),
        radius_nanos,
        sorted_absolute_residuals_nanos: residuals,
    })
}

impl CalibratedArrivalEnvelope {
    /// Build a symmetric interval around a point forecast. None signals that
    /// the conformal rank was infinite, so no finite interval is justified.
    pub fn interval_for(
        &self,
        predicted_arrival_utc_nanos: i128,
    ) -> Result<Option<PredictionInterval>, CalibrationError> {
        let Some(radius_nanos) = self.radius_nanos else {
            return Ok(None);
        };
        let radius =
            i128::try_from(radius_nanos).map_err(|_| CalibrationError::IntervalOverflow)?;
        let lower_utc_nanos = predicted_arrival_utc_nanos
            .checked_sub(radius)
            .ok_or(CalibrationError::IntervalOverflow)?;
        let upper_utc_nanos = predicted_arrival_utc_nanos
            .checked_add(radius)
            .ok_or(CalibrationError::IntervalOverflow)?;
        Ok(Some(PredictionInterval {
            lower_utc_nanos,
            upper_utc_nanos,
        }))
    }
}

/// Evaluate a frozen, post-cutoff set without feeding outcomes back into fitting.
/// Coverage and issuance rate remain separate to prevent abstention from looking
/// like a narrow, well-calibrated interval.
pub fn evaluate_coverage(
    envelope: &CalibratedArrivalEnvelope,
    cases: &[ArrivalEvaluationCase],
) -> Result<CoverageMetrics, CalibrationError> {
    if cases.is_empty() {
        return Err(CalibrationError::EmptyEvaluationSet);
    }

    let mut seen = BTreeSet::new();
    let mut interval_count = 0usize;
    let mut abstention_count = 0usize;
    let mut covered_count = 0usize;
    let mut total_width_nanos = 0u128;

    for case in cases {
        if case.case_id.trim().is_empty() {
            return Err(CalibrationError::EmptyEvaluationId);
        }
        if case.stratum.trim().is_empty() {
            return Err(CalibrationError::EmptyEvaluationStratum);
        }
        if !seen.insert(case.case_id.clone()) {
            return Err(CalibrationError::DuplicateEvaluationId);
        }
        if case.forecast_issued_at_utc_nanos <= envelope.calibration_as_of_utc_nanos {
            return Err(CalibrationError::EvaluationForecastNotAfterCutoff);
        }
        if case.actual_observed_at_utc_nanos <= envelope.calibration_as_of_utc_nanos {
            return Err(CalibrationError::EvaluationTruthNotAfterCutoff);
        }
        if case.forecast_issued_at_utc_nanos >= case.actual_arrival_utc_nanos {
            return Err(CalibrationError::ForecastIssuedAfterActualEvent);
        }
        if case.actual_observed_at_utc_nanos < case.actual_arrival_utc_nanos {
            return Err(CalibrationError::ActualObservedBeforeActualEvent);
        }
        match envelope.interval_for(case.predicted_arrival_utc_nanos)? {
            None => abstention_count += 1,
            Some(interval) => {
                interval_count += 1;
                if (interval.lower_utc_nanos..=interval.upper_utc_nanos)
                    .contains(&case.actual_arrival_utc_nanos)
                {
                    covered_count += 1;
                }
                let width = interval.upper_utc_nanos.abs_diff(interval.lower_utc_nanos);
                total_width_nanos = total_width_nanos
                    .checked_add(width)
                    .ok_or(CalibrationError::ArithmeticOverflow)?;
            }
        }
    }

    let conditional_coverage_bps = if interval_count == 0 {
        None
    } else {
        let scaled = u128::try_from(covered_count)
            .map_err(|_| CalibrationError::ArithmeticOverflow)?
            .checked_mul(u128::from(BASIS_POINTS))
            .ok_or(CalibrationError::ArithmeticOverflow)?;
        let issued =
            u128::try_from(interval_count).map_err(|_| CalibrationError::ArithmeticOverflow)?;
        Some(u16::try_from(scaled / issued).map_err(|_| CalibrationError::ArithmeticOverflow)?)
    };
    let issued_scaled = u128::try_from(interval_count)
        .map_err(|_| CalibrationError::ArithmeticOverflow)?
        .checked_mul(u128::from(BASIS_POINTS))
        .ok_or(CalibrationError::ArithmeticOverflow)?;
    let evaluation_count =
        u128::try_from(cases.len()).map_err(|_| CalibrationError::ArithmeticOverflow)?;
    let interval_issuance_rate_bps = u16::try_from(issued_scaled / evaluation_count)
        .map_err(|_| CalibrationError::ArithmeticOverflow)?;
    let mean_interval_width_nanos = if interval_count == 0 {
        None
    } else {
        let issued =
            u128::try_from(interval_count).map_err(|_| CalibrationError::ArithmeticOverflow)?;
        Some(total_width_nanos / issued)
    };

    Ok(CoverageMetrics {
        evaluation_count: cases.len(),
        interval_count,
        abstention_count,
        covered_count,
        conditional_coverage_bps,
        interval_issuance_rate_bps,
        mean_interval_width_nanos,
    })
}

/// Evaluate each caller-defined stratum independently with deterministic key order.
pub fn evaluate_coverage_by_stratum(
    envelope: &CalibratedArrivalEnvelope,
    cases: &[ArrivalEvaluationCase],
) -> Result<BTreeMap<String, CoverageMetrics>, CalibrationError> {
    if cases.is_empty() {
        return Err(CalibrationError::EmptyEvaluationSet);
    }

    let mut seen = BTreeSet::new();
    let mut grouped = BTreeMap::<String, Vec<ArrivalEvaluationCase>>::new();
    for case in cases {
        if case.case_id.trim().is_empty() {
            return Err(CalibrationError::EmptyEvaluationId);
        }
        if case.stratum.trim().is_empty() {
            return Err(CalibrationError::EmptyEvaluationStratum);
        }
        if !seen.insert(case.case_id.clone()) {
            return Err(CalibrationError::DuplicateEvaluationId);
        }
        grouped
            .entry(case.stratum.clone())
            .or_default()
            .push(case.clone());
    }

    grouped
        .into_iter()
        .map(|(stratum, group)| Ok((stratum, evaluate_coverage(envelope, &group)?)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> i128 {
        10_000 * NANOS_PER_SECOND
    }

    fn sample(index: usize, error_seconds: i64) -> ArrivalResidualSample {
        let predicted = base() + index as i128 * 10 * NANOS_PER_SECOND;
        let error = i128::from(error_seconds) * NANOS_PER_SECOND;
        let actual = predicted + error;
        ArrivalResidualSample {
            forecast_id: format!("FORECAST-{index:03}"),
            forecast_issued_at_utc_nanos: base() - 3_600 * NANOS_PER_SECOND,
            predicted_arrival_utc_nanos: predicted,
            actual_arrival_utc_nanos: actual,
            actual_observed_at_utc_nanos: actual + NANOS_PER_SECOND,
        }
    }

    fn cases(errors_seconds: &[i64]) -> Vec<ArrivalEvaluationCase> {
        errors_seconds
            .iter()
            .enumerate()
            .map(|(index, error)| {
                let predicted =
                    CUTOFF + 3_600 * NANOS_PER_SECOND + index as i128 * 10 * NANOS_PER_SECOND;
                let actual = predicted + i128::from(*error) * NANOS_PER_SECOND;
                ArrivalEvaluationCase {
                    case_id: format!("EVAL-{index:03}"),
                    stratum: "SYNTHETIC".to_owned(),
                    forecast_issued_at_utc_nanos: predicted - 600 * NANOS_PER_SECOND,
                    predicted_arrival_utc_nanos: predicted,
                    actual_arrival_utc_nanos: actual,
                    actual_observed_at_utc_nanos: actual + NANOS_PER_SECOND,
                }
            })
            .collect()
    }

    const CUTOFF: i128 = 20_000 * NANOS_PER_SECOND;

    #[test]
    fn split_conformal_uses_exact_finite_sample_order_statistic() {
        let samples = (1..=9)
            .map(|index| sample(index, index as i64))
            .collect::<Vec<_>>();
        let fitted = fit_split_conformal(&samples, CUTOFF, 1_000).unwrap();
        assert_eq!(fitted.sample_count, 9);
        assert_eq!(fitted.radius_nanos, Some(9 * NANOS_PER_SECOND as u128));
        assert_eq!(fitted.sorted_absolute_residuals_nanos.len(), 9);
    }

    #[test]
    fn insufficient_sample_size_returns_infinite_radius_not_a_guessed_bound() {
        let samples = (1..=8)
            .map(|index| sample(index, index as i64))
            .collect::<Vec<_>>();
        let fitted = fit_split_conformal(&samples, CUTOFF, 1_000).unwrap();
        assert_eq!(fitted.radius_nanos, None);
        assert_eq!(fitted.interval_for(base()).unwrap(), None);
    }

    #[test]
    fn rejects_invalid_target_miscoverage_and_empty_calibration() {
        assert_eq!(
            fit_split_conformal(&[], CUTOFF, 1_000),
            Err(CalibrationError::EmptyCalibrationSet)
        );
        let samples = vec![sample(1, 1)];
        assert_eq!(
            fit_split_conformal(&samples, CUTOFF, 0),
            Err(CalibrationError::InvalidMiscoverage)
        );
        assert_eq!(
            fit_split_conformal(&samples, CUTOFF, BASIS_POINTS),
            Err(CalibrationError::InvalidMiscoverage)
        );
    }

    #[test]
    fn rejects_future_truth_and_forecast_cutoff_leakage() {
        let future_truth = sample(1, 1);
        assert_eq!(
            fit_split_conformal(&[future_truth], base(), 1_000),
            Err(CalibrationError::ActualObservedAfterCutoff)
        );
        let mut future_forecast = sample(2, 1);
        future_forecast.forecast_issued_at_utc_nanos = CUTOFF + 1;
        assert_eq!(
            fit_split_conformal(&[future_forecast], CUTOFF, 1_000),
            Err(CalibrationError::FutureForecastAtCutoff)
        );
    }

    #[test]
    fn rejects_duplicate_ids_and_invalid_temporal_order() {
        let duplicate = vec![sample(1, 1), sample(1, 2)];
        assert_eq!(
            fit_split_conformal(&duplicate, CUTOFF, 1_000),
            Err(CalibrationError::DuplicateForecastId)
        );
        let mut invalid = sample(3, 1);
        invalid.forecast_issued_at_utc_nanos = invalid.actual_arrival_utc_nanos;
        assert_eq!(
            fit_split_conformal(&[invalid], CUTOFF, 1_000),
            Err(CalibrationError::ForecastIssuedAfterActualEvent)
        );

        let mut issued_too_late = sample(4, 1);
        issued_too_late.forecast_issued_at_utc_nanos =
            issued_too_late.actual_observed_at_utc_nanos + 1;
        assert_eq!(
            fit_split_conformal(&[issued_too_late], CUTOFF, 1_000),
            Err(CalibrationError::ForecastIssuedAfterObservation)
        );

        let mut observed_too_early = sample(5, 1);
        observed_too_early.actual_observed_at_utc_nanos =
            observed_too_early.actual_arrival_utc_nanos - 1;
        assert_eq!(
            fit_split_conformal(&[observed_too_early], CUTOFF, 1_000),
            Err(CalibrationError::ActualObservedBeforeActualEvent)
        );
    }

    #[test]
    fn interval_bounds_are_symmetric_and_overflow_fails_closed() {
        let samples = (1..=9)
            .map(|index| sample(index, index as i64))
            .collect::<Vec<_>>();
        let fitted = fit_split_conformal(&samples, CUTOFF, 1_000).unwrap();
        let interval = fitted.interval_for(base()).unwrap().unwrap();
        assert_eq!(interval.lower_utc_nanos, base() - 9 * NANOS_PER_SECOND);
        assert_eq!(interval.upper_utc_nanos, base() + 9 * NANOS_PER_SECOND);
        assert_eq!(
            fitted.interval_for(i128::MAX),
            Err(CalibrationError::IntervalOverflow)
        );
    }

    #[test]
    fn coverage_metrics_report_coverage_width_and_abstention_separately() {
        let samples = (1..=9)
            .map(|index| sample(index, index as i64))
            .collect::<Vec<_>>();
        let fitted = fit_split_conformal(&samples, CUTOFF, 1_000).unwrap();
        let report = evaluate_coverage(&fitted, &cases(&[0, 3, -8, 9, 10])).unwrap();
        assert_eq!(report.evaluation_count, 5);
        assert_eq!(report.interval_count, 5);
        assert_eq!(report.abstention_count, 0);
        assert_eq!(report.covered_count, 4);
        assert_eq!(report.conditional_coverage_bps, Some(8_000));
        assert_eq!(report.interval_issuance_rate_bps, 10_000);
        assert_eq!(
            report.mean_interval_width_nanos,
            Some(18 * NANOS_PER_SECOND as u128)
        );
    }

    #[test]
    fn unbounded_conformal_result_reports_abstentions_not_perfect_coverage() {
        let samples = (1..=8)
            .map(|index| sample(index, index as i64))
            .collect::<Vec<_>>();
        let fitted = fit_split_conformal(&samples, CUTOFF, 1_000).unwrap();
        let report = evaluate_coverage(&fitted, &cases(&[0, 1, 2])).unwrap();
        assert_eq!(report.interval_count, 0);
        assert_eq!(report.abstention_count, 3);
        assert_eq!(report.conditional_coverage_bps, None);
        assert_eq!(report.interval_issuance_rate_bps, 0);
        assert_eq!(report.mean_interval_width_nanos, None);
    }

    #[test]
    fn evaluation_rejects_rows_that_leak_across_the_calibration_cutoff() {
        let samples = (1..=9)
            .map(|index| sample(index, index as i64))
            .collect::<Vec<_>>();
        let fitted = fit_split_conformal(&samples, CUTOFF, 1_000).unwrap();

        let mut forecast_leak = cases(&[1]);
        forecast_leak[0].forecast_issued_at_utc_nanos = CUTOFF;
        assert_eq!(
            evaluate_coverage(&fitted, &forecast_leak),
            Err(CalibrationError::EvaluationForecastNotAfterCutoff)
        );

        let mut truth_leak = cases(&[1]);
        truth_leak[0].actual_observed_at_utc_nanos = CUTOFF;
        assert_eq!(
            evaluate_coverage(&fitted, &truth_leak),
            Err(CalibrationError::EvaluationTruthNotAfterCutoff)
        );
    }

    #[test]
    fn fixed_calibration_exposes_coverage_decay_under_regime_shift() {
        let samples = (1_i64..=20)
            .map(|error| sample(usize::try_from(error).unwrap(), error))
            .collect::<Vec<_>>();
        let fitted = fit_split_conformal(&samples, CUTOFF, 1_000).unwrap();
        assert_eq!(fitted.radius_nanos, Some(19 * NANOS_PER_SECOND as u128));

        let baseline_errors = (1_i64..=20).collect::<Vec<_>>();
        let baseline = evaluate_coverage(&fitted, &cases(&baseline_errors)).unwrap();
        assert_eq!(baseline.conditional_coverage_bps, Some(9_500));

        let shifted_errors = (40_i64..=59).collect::<Vec<_>>();
        let shifted = evaluate_coverage(&fitted, &cases(&shifted_errors)).unwrap();
        assert_eq!(shifted.conditional_coverage_bps, Some(0));
        assert_eq!(shifted.interval_issuance_rate_bps, 10_000);
    }
    #[test]
    fn coverage_is_reported_per_stratum_without_pooling_away_shift() {
        let samples = (1_i64..=20)
            .map(|error| sample(usize::try_from(error).unwrap(), error))
            .collect::<Vec<_>>();
        let fitted = fit_split_conformal(&samples, CUTOFF, 1_000).unwrap();

        let mut baseline = cases(&(1_i64..=20).collect::<Vec<_>>());
        for case in &mut baseline {
            case.stratum = "BASELINE".to_owned();
            case.case_id = format!("BASE-{}", case.case_id);
        }
        let mut shifted = cases(&(40_i64..=59).collect::<Vec<_>>());
        for case in &mut shifted {
            case.stratum = "SHIFTED".to_owned();
            case.case_id = format!("SHIFT-{}", case.case_id);
        }
        baseline.extend(shifted);

        let report = evaluate_coverage_by_stratum(&fitted, &baseline).unwrap();
        assert_eq!(report["BASELINE"].conditional_coverage_bps, Some(9_500));
        assert_eq!(report["SHIFTED"].conditional_coverage_bps, Some(0));
        assert_eq!(report["BASELINE"].evaluation_count, 20);
        assert_eq!(report["SHIFTED"].evaluation_count, 20);
    }

    #[test]
    fn calibration_is_permutation_invariant_and_preserves_cutoff() {
        let first = (1..=9)
            .map(|index| sample(index, index as i64))
            .collect::<Vec<_>>();
        let mut second = first.clone();
        second.reverse();
        assert_eq!(
            fit_split_conformal(&first, CUTOFF, 1_000),
            fit_split_conformal(&second, CUTOFF, 1_000)
        );
        assert_eq!(
            fit_split_conformal(&first, CUTOFF, 1_000)
                .unwrap()
                .calibration_as_of_utc_nanos,
            CUTOFF
        );
    }
}
