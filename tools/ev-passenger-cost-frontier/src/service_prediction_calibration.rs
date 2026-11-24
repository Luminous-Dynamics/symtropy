// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Evidence-bound, chronological forecast calibration audit.
//!
//! This module audits interval forecasts and on-time probabilities against
//! later observations. It does not train a forecaster or grant operational
//! authority. Split-conformal expansion is reported as an empirical diagnostic;
//! no exchangeability or deployment-shift guarantee is assumed by this code.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::sustainable_mobility_service_quality::{EvidenceClass, EvidenceEnvelope};

const MAX_RECORDS: usize = 2_000;
const MAX_METRICS_PER_RECORD: usize = 8;
const MAX_STRATUM_LEN: usize = 96;
const MAX_MODEL_ID_LEN: usize = 96;
const MAX_VERSION_LEN: usize = 64;
const MAX_VALUE: u64 = 1_000_000_000_000;
const MAX_COVERAGE_PERMILLE: u16 = 990;
const MIN_COVERAGE_PERMILLE: u16 = 500;
const CLAIM_CEILING: &str = "Chronological empirical forecast-calibration audit only; split-conformal expansion is not claimed valid under arbitrary temporal dependence or deployment shift. No battery safety, flight, dispatch, grid, market, production or physical authority.";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationCohort {
    Calibration,
    Holdout,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ForecastMetricKind {
    MissionDurationSeconds,
    PackTerminalEnergyWh,
    ChargeRestorationSeconds,
    ChargerInputEnergyWh,
    QueueWaitSeconds,
    SitePccPeakImportW,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementBoundary {
    MissionClock,
    PackTerminal,
    ChargerServiceClock,
    ChargerInput,
    SitePccMeter,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationGroupStatus {
    EmpiricallySupported,
    SyntheticEvidenceOnly,
    UndercoverageAlarm,
    CoverageNotDemonstrated,
    InsufficientCalibrationSamples,
    InsufficientHoldoutSamples,
    NoFiniteConformalQuantile,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ForecastMetricObservation {
    pub metric_kind: ForecastMetricKind,
    pub measurement_boundary: MeasurementBoundary,
    pub nominal_coverage_permille: u16,
    pub asset_id: String,
    pub configuration_digest: String,
    pub predicted_lower: u64,
    pub predicted_point: u64,
    pub predicted_upper: u64,
    pub observed_value: u64,
    pub forecast_evidence: EvidenceEnvelope,
    pub observation_evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationRecord {
    pub sample_id: String,
    pub cohort: CalibrationCohort,
    pub site_id: String,
    pub site_configuration_digest: String,
    pub stratum_id: String,
    pub forecast_model_id: String,
    pub forecast_model_version: String,
    pub decision_time_unix_ms: u64,
    pub target_start_unix_ms: u64,
    pub target_end_unix_ms: u64,
    pub on_time_probability_permille: u16,
    pub on_time_observed: bool,
    pub on_time_forecast_evidence: EvidenceEnvelope,
    pub on_time_evidence: EvidenceEnvelope,
    pub metrics: Vec<ForecastMetricObservation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServicePredictionCalibrationInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub audit_cutoff_unix_ms: u64,
    pub minimum_calibration_samples_per_group: u16,
    pub minimum_holdout_samples_per_group: u16,
    pub undercoverage_tolerance_permille: u16,
    pub records: Vec<CalibrationRecord>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WilsonInterval {
    pub lower_permille: u16,
    pub upper_permille: u16,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MetricCalibrationGroup {
    pub forecast_model_id: String,
    pub forecast_model_version: String,
    pub site_id: String,
    pub site_configuration_digest: String,
    pub stratum_id: String,
    pub asset_id: String,
    pub metric_kind: ForecastMetricKind,
    pub measurement_boundary: MeasurementBoundary,
    pub configuration_digest: String,
    pub nominal_coverage_permille: u16,
    pub calibration_sample_count: usize,
    pub holdout_sample_count: usize,
    pub conformal_adjustment: Option<u64>,
    pub conformal_quantile_rank: Option<usize>,
    pub holdout_coverage_before_permille: Option<u16>,
    pub holdout_coverage_after_permille: Option<u16>,
    pub holdout_coverage_after_wilson_95: Option<WilsonInterval>,
    pub mean_interval_width_before: Option<u64>,
    pub mean_interval_width_after: Option<u64>,
    pub mean_absolute_error: Option<u64>,
    pub mean_winkler_score: Option<f64>,
    pub undercoverage_alarm: bool,
    pub coverage_supported_by_wilson_lower_bound: bool,
    pub status: CalibrationGroupStatus,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ReliabilityBin {
    pub lower_probability_permille: u16,
    pub upper_probability_permille: u16,
    pub sample_count: usize,
    pub mean_predicted_probability_permille: u16,
    pub observed_on_time_rate_permille: u16,
    pub observed_rate_wilson_95: WilsonInterval,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OnTimeCalibrationGroup {
    pub forecast_model_id: String,
    pub forecast_model_version: String,
    pub site_id: String,
    pub stratum_id: String,
    pub site_configuration_digest: String,
    pub calibration_sample_count: usize,
    pub holdout_sample_count: usize,
    pub holdout_brier_score_millionths: u32,
    pub holdout_mean_predicted_probability_permille: u16,
    pub holdout_observed_on_time_rate_permille: u16,
    pub holdout_observed_rate_wilson_95: WilsonInterval,
    pub holdout_expected_calibration_error_permille: u16,
    pub holdout_reliability_bins: Vec<ReliabilityBin>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ServicePredictionCalibrationReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub audit_cutoff_unix_ms: u64,
    /// Wilson intervals are a reference calculation under independent Bernoulli samples;
    /// temporal/common-cause dependence requires clustered inference in a later version.
    pub wilson_reference_assumes_iid_samples: bool,
    pub record_count: usize,
    pub calibration_record_count: usize,
    pub holdout_record_count: usize,
    pub metric_group_count: usize,
    pub on_time_group_count: usize,
    pub all_evidence_source_grade: bool,
    pub any_synthetic_evidence: bool,
    pub any_undercoverage_alarm: bool,
    pub all_metric_groups_empirically_supported: bool,
    pub input_digest: String,
    pub metric_groups: Vec<MetricCalibrationGroup>,
    pub on_time_groups: Vec<OnTimeCalibrationGroup>,
    pub field_qualification_granted: bool,
    pub model_update_performed: bool,
    pub operational_authority_granted: bool,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct MetricGroupKey {
    model_id: String,
    model_version: String,
    site_id: String,
    site_configuration_digest: String,
    stratum_id: String,
    asset_id: String,
    metric_kind: ForecastMetricKind,
    boundary: MeasurementBoundary,
    configuration_digest: String,
    nominal_coverage_permille: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct OnTimeGroupKey {
    model_id: String,
    model_version: String,
    site_id: String,
    stratum_id: String,
    site_configuration_digest: String,
}

#[derive(Debug, Clone, Copy)]
struct IntervalPair {
    lower: u64,
    upper: u64,
    point: u64,
    observed: u64,
}

fn expected_boundary(kind: ForecastMetricKind) -> MeasurementBoundary {
    match kind {
        ForecastMetricKind::MissionDurationSeconds => MeasurementBoundary::MissionClock,
        ForecastMetricKind::PackTerminalEnergyWh => MeasurementBoundary::PackTerminal,
        ForecastMetricKind::ChargeRestorationSeconds | ForecastMetricKind::QueueWaitSeconds => {
            MeasurementBoundary::ChargerServiceClock
        }
        ForecastMetricKind::ChargerInputEnergyWh => MeasurementBoundary::ChargerInput,
        ForecastMetricKind::SitePccPeakImportW => MeasurementBoundary::SitePccMeter,
    }
}

fn add_error(errors: &mut Vec<String>, message: impl Into<String>) {
    errors.push(message.into());
}

fn evidence_is_well_formed(evidence: &EvidenceEnvelope) -> bool {
    !evidence.asset_id.trim().is_empty()
        && !evidence.configuration_digest.trim().is_empty()
        && !evidence.producer_id.trim().is_empty()
        && !evidence.method_id.trim().is_empty()
        && !evidence.evidence_reference.trim().is_empty()
        && evidence.evidence_class != EvidenceClass::Unknown
}

fn validate_forecast_evidence(
    evidence: &EvidenceEnvelope,
    asset_id: &str,
    configuration_digest: &str,
    decision_time: u64,
    target_end: u64,
) -> bool {
    if !evidence_is_well_formed(evidence)
        || evidence.asset_id != asset_id
        || evidence.configuration_digest != configuration_digest
    {
        return false;
    }
    matches!(
        (
            evidence.observed_at_unix_ms,
            evidence.valid_from_unix_ms,
            evidence.valid_until_unix_ms
        ),
        (Some(observed), Some(valid_from), Some(valid_until))
            if observed <= decision_time
                && valid_from <= decision_time
                && target_end <= valid_until
                && valid_from <= observed
                && observed <= valid_until
    )
}

fn validate_outcome_evidence(
    evidence: &EvidenceEnvelope,
    asset_id: &str,
    configuration_digest: &str,
    target_end: u64,
    audit_cutoff: u64,
) -> bool {
    if !evidence_is_well_formed(evidence)
        || evidence.asset_id != asset_id
        || evidence.configuration_digest != configuration_digest
    {
        return false;
    }
    matches!(
        (
            evidence.observed_at_unix_ms,
            evidence.valid_from_unix_ms,
            evidence.valid_until_unix_ms
        ),
        (Some(observed), Some(valid_from), Some(valid_until))
            if observed >= target_end
                && observed <= audit_cutoff
                && valid_from <= target_end
                && observed <= valid_until
                && valid_from <= valid_until
    )
}

fn wilson_interval(successes: usize, trials: usize) -> Option<WilsonInterval> {
    if trials == 0 || successes > trials {
        return None;
    }
    let z = 1.959_963_984_540_054_f64;
    let n = trials as f64;
    let p = successes as f64 / n;
    let z2 = z * z;
    let denominator = 1.0 + z2 / n;
    let centre = (p + z2 / (2.0 * n)) / denominator;
    let radius = z * ((p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt()) / denominator;
    let lower = ((centre - radius).max(0.0) * 1_000.0).floor() as u16;
    let upper = ((centre + radius).min(1.0) * 1_000.0).ceil() as u16;
    Some(WilsonInterval {
        lower_permille: lower,
        upper_permille: upper.min(1_000),
    })
}

fn mean_u64(values: &[u64]) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let sum = values
        .iter()
        .try_fold(0_u128, |sum, value| sum.checked_add(u128::from(*value)))?;
    u64::try_from(sum / values.len() as u128).ok()
}

fn interval_nonconformity(interval: IntervalPair) -> u64 {
    interval
        .lower
        .saturating_sub(interval.observed)
        .max(interval.observed.saturating_sub(interval.upper))
}

fn quantile_rank(sample_count: usize, nominal_coverage_permille: u16) -> Option<usize> {
    if sample_count == 0 {
        return None;
    }
    let numerator =
        (sample_count as u128 + 1).checked_mul(u128::from(nominal_coverage_permille))?;
    let rank = numerator.checked_add(999)? / 1_000;
    usize::try_from(rank).ok()
}

fn interval_metrics(
    pairs: &[IntervalPair],
    adjustment: u64,
    nominal_coverage_permille: u16,
) -> Option<(u16, u16, u64, u64, f64)> {
    if pairs.is_empty() {
        return None;
    }
    let mut covered_before = 0_usize;
    let mut covered_after = 0_usize;
    let mut widths_before = Vec::with_capacity(pairs.len());
    let mut widths_after = Vec::with_capacity(pairs.len());
    let mut winkler_sum = 0.0_f64;
    let alpha = 1.0 - f64::from(nominal_coverage_permille) / 1_000.0;

    for pair in pairs {
        if pair.observed >= pair.lower && pair.observed <= pair.upper {
            covered_before += 1;
        }
        let adjusted_lower = pair.lower.saturating_sub(adjustment);
        let adjusted_upper = pair.upper.checked_add(adjustment)?;
        if pair.observed >= adjusted_lower && pair.observed <= adjusted_upper {
            covered_after += 1;
        }
        widths_before.push(pair.upper.saturating_sub(pair.lower));
        widths_after.push(adjusted_upper.saturating_sub(adjusted_lower));
        let miss = if pair.observed < adjusted_lower {
            adjusted_lower - pair.observed
        } else if pair.observed > adjusted_upper {
            pair.observed - adjusted_upper
        } else {
            0
        };
        winkler_sum +=
            adjusted_upper.saturating_sub(adjusted_lower) as f64 + 2.0 * miss as f64 / alpha;
    }

    let n = pairs.len() as u64;
    let before = u16::try_from((covered_before as u64 * 1_000) / n).ok()?;
    let after = u16::try_from((covered_after as u64 * 1_000) / n).ok()?;
    Some((
        before,
        after,
        mean_u64(&widths_before)?,
        mean_u64(&widths_after)?,
        winkler_sum / pairs.len() as f64,
    ))
}

fn build_metric_group(
    key: &MetricGroupKey,
    calibration: &[IntervalPair],
    holdout: &[IntervalPair],
    minimum_calibration: usize,
    minimum_holdout: usize,
    tolerance_permille: u16,
) -> MetricCalibrationGroup {
    let mut base = MetricCalibrationGroup {
        forecast_model_id: key.model_id.clone(),
        forecast_model_version: key.model_version.clone(),
        site_id: key.site_id.clone(),
        site_configuration_digest: key.site_configuration_digest.clone(),
        stratum_id: key.stratum_id.clone(),
        asset_id: key.asset_id.clone(),
        metric_kind: key.metric_kind,
        measurement_boundary: key.boundary,
        configuration_digest: key.configuration_digest.clone(),
        nominal_coverage_permille: key.nominal_coverage_permille,
        calibration_sample_count: calibration.len(),
        holdout_sample_count: holdout.len(),
        conformal_adjustment: None,
        conformal_quantile_rank: None,
        holdout_coverage_before_permille: None,
        holdout_coverage_after_permille: None,
        holdout_coverage_after_wilson_95: None,
        mean_interval_width_before: None,
        mean_interval_width_after: None,
        mean_absolute_error: None,
        mean_winkler_score: None,
        undercoverage_alarm: false,
        coverage_supported_by_wilson_lower_bound: false,
        status: CalibrationGroupStatus::InsufficientCalibrationSamples,
    };

    if calibration.len() < minimum_calibration {
        return base;
    }
    if holdout.len() < minimum_holdout {
        base.status = CalibrationGroupStatus::InsufficientHoldoutSamples;
        return base;
    }

    let Some(rank) = quantile_rank(calibration.len(), key.nominal_coverage_permille) else {
        base.status = CalibrationGroupStatus::NoFiniteConformalQuantile;
        return base;
    };
    base.conformal_quantile_rank = Some(rank);
    if rank == 0 || rank > calibration.len() {
        base.status = CalibrationGroupStatus::NoFiniteConformalQuantile;
        return base;
    }

    let mut scores: Vec<u64> = calibration
        .iter()
        .copied()
        .map(interval_nonconformity)
        .collect();
    scores.sort_unstable();
    let adjustment = scores[rank - 1];
    base.conformal_adjustment = Some(adjustment);

    if let Some((before, after, width_before, width_after, winkler)) =
        interval_metrics(holdout, adjustment, key.nominal_coverage_permille)
    {
        let covered_after = holdout
            .iter()
            .filter(|pair| {
                let lower = pair.lower.saturating_sub(adjustment);
                let upper = pair.upper.saturating_add(adjustment);
                pair.observed >= lower && pair.observed <= upper
            })
            .count();
        let abs_errors: Vec<u64> = holdout
            .iter()
            .map(|pair| pair.point.abs_diff(pair.observed))
            .collect();
        base.holdout_coverage_before_permille = Some(before);
        base.holdout_coverage_after_permille = Some(after);
        base.holdout_coverage_after_wilson_95 = wilson_interval(covered_after, holdout.len());
        base.mean_interval_width_before = Some(width_before);
        base.mean_interval_width_after = Some(width_after);
        base.mean_absolute_error = mean_u64(&abs_errors);
        base.mean_winkler_score = Some(winkler);

        let nominal = key.nominal_coverage_permille;
        let undercoverage_threshold = nominal.saturating_sub(tolerance_permille);
        base.undercoverage_alarm = after < undercoverage_threshold;
        base.coverage_supported_by_wilson_lower_bound = base
            .holdout_coverage_after_wilson_95
            .as_ref()
            .is_some_and(|interval| interval.lower_permille >= nominal);
        base.status = if base.undercoverage_alarm {
            CalibrationGroupStatus::UndercoverageAlarm
        } else if base.coverage_supported_by_wilson_lower_bound {
            CalibrationGroupStatus::EmpiricallySupported
        } else {
            CalibrationGroupStatus::CoverageNotDemonstrated
        };
    } else {
        base.status = CalibrationGroupStatus::NoFiniteConformalQuantile;
    }
    base
}

fn build_on_time_group(
    key: &OnTimeGroupKey,
    calibration_count: usize,
    holdout: &[(u16, bool)],
) -> Option<OnTimeCalibrationGroup> {
    if holdout.is_empty() {
        return None;
    }
    let n = holdout.len() as u64;
    let brier_sum: u128 = holdout
        .iter()
        .map(|(probability, observed)| {
            let target = if *observed { 1_000_i64 } else { 0_i64 };
            let error = i64::from(*probability) - target;
            (error * error) as u128
        })
        .sum();
    let brier = u32::try_from(brier_sum / u128::from(n)).ok()?;

    let predicted_sum: u64 = holdout.iter().map(|(p, _)| u64::from(*p)).sum();
    let successes = holdout.iter().filter(|(_, observed)| *observed).count();
    let mut reliability_bins = Vec::with_capacity(10);
    let mut weighted_abs_gap = 0_u128;
    for bin in 0..10_u16 {
        let lower = bin * 100;
        let upper = if bin == 9 { 1_000 } else { lower + 100 };
        let selected: Vec<_> = holdout
            .iter()
            .filter(|(p, _)| *p >= lower && if bin == 9 { *p <= upper } else { *p < upper })
            .collect();
        if selected.is_empty() {
            continue;
        }
        let count = selected.len();
        let predicted = selected.iter().map(|(p, _)| u64::from(*p)).sum::<u64>() / count as u64;
        let bin_successes = selected.iter().filter(|(_, observed)| *observed).count();
        let observed_rate = (bin_successes as u64 * 1_000) / count as u64;
        weighted_abs_gap += u128::from(predicted.abs_diff(observed_rate)) * count as u128;
        reliability_bins.push(ReliabilityBin {
            lower_probability_permille: lower,
            upper_probability_permille: upper,
            sample_count: count,
            mean_predicted_probability_permille: u16::try_from(predicted).ok()?,
            observed_on_time_rate_permille: u16::try_from(observed_rate).ok()?,
            observed_rate_wilson_95: wilson_interval(bin_successes, count)?,
        });
    }
    let ece = u16::try_from(weighted_abs_gap / holdout.len() as u128).ok()?;
    Some(OnTimeCalibrationGroup {
        forecast_model_id: key.model_id.clone(),
        forecast_model_version: key.model_version.clone(),
        site_id: key.site_id.clone(),
        stratum_id: key.stratum_id.clone(),
        site_configuration_digest: key.site_configuration_digest.clone(),
        calibration_sample_count: calibration_count,
        holdout_sample_count: holdout.len(),
        holdout_brier_score_millionths: brier,
        holdout_mean_predicted_probability_permille: u16::try_from(
            predicted_sum / holdout.len() as u64,
        )
        .ok()?,
        holdout_observed_on_time_rate_permille: u16::try_from(
            successes as u64 * 1_000 / holdout.len() as u64,
        )
        .ok()?,
        holdout_observed_rate_wilson_95: wilson_interval(successes, holdout.len())?,
        holdout_expected_calibration_error_permille: ece,
        holdout_reliability_bins: reliability_bins,
    })
}

fn validate_record(
    record: &CalibrationRecord,
    audit_cutoff: u64,
    evidence_references: &mut BTreeSet<String>,
    all_source_grade: &mut bool,
    any_synthetic: &mut bool,
) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if record.sample_id.trim().is_empty()
        || record.sample_id.len() > 128
        || record.site_id.trim().is_empty()
        || record.site_configuration_digest.trim().is_empty()
        || record.stratum_id.trim().is_empty()
        || record.stratum_id.len() > MAX_STRATUM_LEN
        || record.forecast_model_id.trim().is_empty()
        || record.forecast_model_id.len() > MAX_MODEL_ID_LEN
        || record.forecast_model_version.trim().is_empty()
        || record.forecast_model_version.len() > MAX_VERSION_LEN
        || record.decision_time_unix_ms == 0
        || record.target_start_unix_ms <= record.decision_time_unix_ms
        || record.target_end_unix_ms <= record.target_start_unix_ms
        || record.target_end_unix_ms > audit_cutoff
        || record.on_time_probability_permille > 1_000
        || record.metrics.is_empty()
        || record.metrics.len() > MAX_METRICS_PER_RECORD
    {
        add_error(
            &mut errors,
            format!("record_identity_or_time_invalid:{}", record.sample_id),
        );
    }

    if !validate_outcome_evidence(
        &record.on_time_evidence,
        &record.site_id,
        &record.site_configuration_digest,
        record.target_end_unix_ms,
        audit_cutoff,
    ) {
        add_error(
            &mut errors,
            format!("on_time_outcome_evidence_invalid:{}", record.sample_id),
        );
    }
    if !evidence_references.insert(record.on_time_forecast_evidence.evidence_reference.clone()) {
        add_error(
            &mut errors,
            format!(
                "evidence_reference_reused:{}",
                record.on_time_forecast_evidence.evidence_reference
            ),
        );
    }
    if !validate_forecast_evidence(
        &record.on_time_forecast_evidence,
        &record.site_id,
        &record.site_configuration_digest,
        record.decision_time_unix_ms,
        record.target_end_unix_ms,
    ) {
        add_error(
            &mut errors,
            format!("on_time_forecast_evidence_invalid:{}", record.sample_id),
        );
    }
    if !evidence_references.insert(record.on_time_evidence.evidence_reference.clone()) {
        add_error(
            &mut errors,
            format!(
                "evidence_reference_reused:{}",
                record.on_time_evidence.evidence_reference
            ),
        );
    }
    *all_source_grade &= matches!(
        record.on_time_forecast_evidence.evidence_class,
        EvidenceClass::Measured | EvidenceClass::IndependentTest
    );
    *any_synthetic |= record.on_time_forecast_evidence.evidence_class == EvidenceClass::Synthetic;
    *all_source_grade &= matches!(
        record.on_time_evidence.evidence_class,
        EvidenceClass::Measured | EvidenceClass::IndependentTest
    );
    *any_synthetic |= record.on_time_evidence.evidence_class == EvidenceClass::Synthetic;

    let mut metric_keys = BTreeSet::new();
    for metric in &record.metrics {
        let key = (
            metric.metric_kind,
            metric.measurement_boundary,
            metric.nominal_coverage_permille,
        );
        if !metric_keys.insert(key) {
            add_error(
                &mut errors,
                format!("duplicate_metric_in_record:{}", record.sample_id),
            );
        }
        if metric.measurement_boundary != expected_boundary(metric.metric_kind)
            || !(MIN_COVERAGE_PERMILLE..=MAX_COVERAGE_PERMILLE)
                .contains(&metric.nominal_coverage_permille)
            || metric.asset_id.trim().is_empty()
            || metric.configuration_digest.trim().is_empty()
            || metric.predicted_lower > metric.predicted_point
            || metric.predicted_point > metric.predicted_upper
            || metric.predicted_upper > MAX_VALUE
            || metric.observed_value > MAX_VALUE
        {
            add_error(
                &mut errors,
                format!("metric_boundary_or_interval_invalid:{}", record.sample_id),
            );
        }
        if !validate_forecast_evidence(
            &metric.forecast_evidence,
            &metric.asset_id,
            &metric.configuration_digest,
            record.decision_time_unix_ms,
            record.target_end_unix_ms,
        ) {
            add_error(
                &mut errors,
                format!(
                    "forecast_evidence_invalid:{}:{:?}",
                    record.sample_id, metric.metric_kind
                ),
            );
        }
        if !validate_outcome_evidence(
            &metric.observation_evidence,
            &metric.asset_id,
            &metric.configuration_digest,
            record.target_end_unix_ms,
            audit_cutoff,
        ) {
            add_error(
                &mut errors,
                format!(
                    "metric_observation_evidence_invalid:{}:{:?}",
                    record.sample_id, metric.metric_kind
                ),
            );
        }
        for evidence in [&metric.forecast_evidence, &metric.observation_evidence] {
            if !evidence_references.insert(evidence.evidence_reference.clone()) {
                add_error(
                    &mut errors,
                    format!("evidence_reference_reused:{}", evidence.evidence_reference),
                );
            }
            *all_source_grade &= matches!(
                evidence.evidence_class,
                EvidenceClass::Measured | EvidenceClass::IndependentTest
            );
            *any_synthetic |= evidence.evidence_class == EvidenceClass::Synthetic;
        }
    }
    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        Err(errors)
    } else {
        Ok(())
    }
}

fn input_digest(input: &ServicePredictionCalibrationInput) -> Result<String, String> {
    let mut canonical = input.clone();
    canonical
        .records
        .sort_by(|a, b| a.sample_id.cmp(&b.sample_id));
    for record in &mut canonical.records {
        record.metrics.sort_by(|a, b| {
            (
                a.metric_kind,
                a.measurement_boundary,
                a.nominal_coverage_permille,
                a.asset_id.as_str(),
                a.configuration_digest.as_str(),
            )
                .cmp(&(
                    b.metric_kind,
                    b.measurement_boundary,
                    b.nominal_coverage_permille,
                    b.asset_id.as_str(),
                    b.configuration_digest.as_str(),
                ))
        });
    }
    let bytes = serde_json::to_vec(&canonical)
        .map_err(|error| format!("calibration_input_serialization_failed:{error}"))?;
    let digest = Sha256::digest(bytes);
    let mut output = String::from("sha256:");
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").map_err(|_| "digest_format_failed")?;
    }
    Ok(output)
}

pub fn evaluate_service_prediction_calibration(
    input: &ServicePredictionCalibrationInput,
) -> Result<ServicePredictionCalibrationReport, Vec<String>> {
    let mut errors = Vec::new();
    if input.schema_version != "0.1.0"
        || input.benchmark_id != "SERVICE-PREDICTION-CALIBRATION-001"
        || input.audit_cutoff_unix_ms == 0
        || input.minimum_calibration_samples_per_group < 5
        || input.minimum_calibration_samples_per_group > 1_000
        || input.minimum_holdout_samples_per_group < 5
        || input.minimum_holdout_samples_per_group > 1_000
        || input.undercoverage_tolerance_permille > 250
        || input.records.is_empty()
        || input.records.len() > MAX_RECORDS
        || input.claim_ceiling.trim().is_empty()
    {
        add_error(&mut errors, "calibration_identity_or_bounds_invalid");
    }

    let mut evidence_references = BTreeSet::new();
    let mut sample_ids = BTreeSet::new();
    let mut all_source_grade = true;
    let mut any_synthetic = false;
    for record in &input.records {
        if !sample_ids.insert(record.sample_id.as_str()) {
            add_error(
                &mut errors,
                format!("sample_id_duplicate:{}", record.sample_id),
            );
        }
        if let Err(record_errors) = validate_record(
            record,
            input.audit_cutoff_unix_ms,
            &mut evidence_references,
            &mut all_source_grade,
            &mut any_synthetic,
        ) {
            errors.extend(record_errors);
        }
    }

    let mut metric_groups: BTreeMap<
        MetricGroupKey,
        (Vec<IntervalPair>, Vec<IntervalPair>, u64, u64),
    > = BTreeMap::new();
    let mut on_time_groups: BTreeMap<
        OnTimeGroupKey,
        (Vec<(u16, bool)>, Vec<(u16, bool)>, u64, u64),
    > = BTreeMap::new();
    for record in &input.records {
        let on_key = OnTimeGroupKey {
            model_id: record.forecast_model_id.clone(),
            model_version: record.forecast_model_version.clone(),
            site_id: record.site_id.clone(),
            stratum_id: record.stratum_id.clone(),
            site_configuration_digest: record.site_configuration_digest.clone(),
        };
        let on_entry = on_time_groups
            .entry(on_key)
            .or_insert_with(|| (Vec::new(), Vec::new(), 0, 0));
        if record.cohort == CalibrationCohort::Calibration {
            on_entry
                .0
                .push((record.on_time_probability_permille, record.on_time_observed));
            on_entry.2 = on_entry
                .2
                .max(record.on_time_evidence.observed_at_unix_ms.unwrap_or(0));
        } else {
            on_entry
                .1
                .push((record.on_time_probability_permille, record.on_time_observed));
            let forecasted = record
                .on_time_forecast_evidence
                .observed_at_unix_ms
                .unwrap_or(u64::MAX);
            on_entry.3 = if on_entry.3 == 0 {
                forecasted
            } else {
                on_entry.3.min(forecasted)
            };
        }

        for metric in &record.metrics {
            let key = MetricGroupKey {
                model_id: record.forecast_model_id.clone(),
                model_version: record.forecast_model_version.clone(),
                site_id: record.site_id.clone(),
                site_configuration_digest: record.site_configuration_digest.clone(),
                stratum_id: record.stratum_id.clone(),
                asset_id: metric.asset_id.clone(),
                metric_kind: metric.metric_kind,
                boundary: metric.measurement_boundary,
                configuration_digest: metric.configuration_digest.clone(),
                nominal_coverage_permille: metric.nominal_coverage_permille,
            };
            let entry = metric_groups
                .entry(key)
                .or_insert_with(|| (Vec::new(), Vec::new(), 0, 0));
            let pair = IntervalPair {
                lower: metric.predicted_lower,
                upper: metric.predicted_upper,
                point: metric.predicted_point,
                observed: metric.observed_value,
            };
            if record.cohort == CalibrationCohort::Calibration {
                entry.0.push(pair);
                let observed = metric.observation_evidence.observed_at_unix_ms.unwrap_or(0);
                entry.2 = entry.2.max(observed);
            } else {
                entry.1.push(pair);
                let forecasted = metric
                    .forecast_evidence
                    .observed_at_unix_ms
                    .unwrap_or(u64::MAX);
                entry.3 = if entry.3 == 0 {
                    forecasted
                } else {
                    entry.3.min(forecasted)
                };
            }
        }
    }

    for (key, (calibration, holdout, last_calibration_outcome, first_holdout_forecast)) in
        &metric_groups
    {
        if !calibration.is_empty()
            && !holdout.is_empty()
            && last_calibration_outcome >= first_holdout_forecast
        {
            add_error(
                &mut errors,
                format!(
                    "calibration_outcome_after_holdout_forecast:{}:{}:{:?}",
                    key.model_id, key.stratum_id, key.metric_kind
                ),
            );
        }
    }

    for (key, (calibration, holdout, last_calibration_outcome, first_holdout_forecast)) in
        &on_time_groups
    {
        if !calibration.is_empty()
            && !holdout.is_empty()
            && last_calibration_outcome > first_holdout_forecast
        {
            add_error(
                &mut errors,
                format!(
                    "on_time_calibration_outcome_after_holdout_forecast:{}:{}",
                    key.model_id, key.stratum_id
                ),
            );
        }
    }

    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let mut metric_results = Vec::new();
    for (key, (calibration, holdout, _, _)) in metric_groups {
        metric_results.push(build_metric_group(
            &key,
            &calibration,
            &holdout,
            usize::from(input.minimum_calibration_samples_per_group),
            usize::from(input.minimum_holdout_samples_per_group),
            input.undercoverage_tolerance_permille,
        ));
    }
    let mut on_time_results = Vec::new();
    for (key, (calibration, holdout, _, _)) in on_time_groups {
        if let Some(group) = build_on_time_group(&key, calibration.len(), &holdout) {
            on_time_results.push(group);
        }
    }

    if any_synthetic {
        for group in &mut metric_results {
            if group.status == CalibrationGroupStatus::EmpiricallySupported {
                group.status = CalibrationGroupStatus::SyntheticEvidenceOnly;
            }
        }
    }
    let any_undercoverage_alarm = metric_results.iter().any(|group| group.undercoverage_alarm);
    let all_metric_groups_empirically_supported = all_source_grade
        && !metric_results.is_empty()
        && metric_results
            .iter()
            .all(|group| group.status == CalibrationGroupStatus::EmpiricallySupported);
    let digest = input_digest(input).map_err(|error| vec![error])?;
    Ok(ServicePredictionCalibrationReport {
        schema_version: input.schema_version.clone(),
        benchmark_id: input.benchmark_id.clone(),
        audit_cutoff_unix_ms: input.audit_cutoff_unix_ms,
        wilson_reference_assumes_iid_samples: true,
        record_count: input.records.len(),
        calibration_record_count: input
            .records
            .iter()
            .filter(|r| r.cohort == CalibrationCohort::Calibration)
            .count(),
        holdout_record_count: input
            .records
            .iter()
            .filter(|r| r.cohort == CalibrationCohort::Holdout)
            .count(),
        metric_group_count: metric_results.len(),
        on_time_group_count: on_time_results.len(),
        all_evidence_source_grade: all_source_grade,
        any_synthetic_evidence: any_synthetic,
        any_undercoverage_alarm,
        all_metric_groups_empirically_supported,
        input_digest: digest,
        metric_groups: metric_results,
        on_time_groups: on_time_results,
        field_qualification_granted: false,
        model_update_performed: false,
        operational_authority_granted: false,
        claim_ceiling: CLAIM_CEILING,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ServicePredictionCalibrationInput {
        serde_json::from_str(include_str!(
            "../fixtures/service_prediction_calibration_v0_1.json"
        ))
        .expect("frozen calibration fixture must parse")
    }

    #[test]
    fn fixture_emits_deterministic_stratified_calibration_report() {
        let input = fixture();
        let a = evaluate_service_prediction_calibration(&input).expect("fixture");
        let b = evaluate_service_prediction_calibration(&input).expect("replay");
        assert_eq!(
            serde_json::to_string(&a).expect("serialize first"),
            serde_json::to_string(&b).expect("serialize second")
        );
        assert_eq!(a.metric_group_count, 6);
        assert_eq!(a.on_time_group_count, 2);
        assert!(!a.field_qualification_granted);
        assert!(!a.model_update_performed);
        assert!(!a.operational_authority_granted);
    }

    #[test]
    fn conformal_adjustment_is_calculated_from_calibration_only() {
        let report = evaluate_service_prediction_calibration(&fixture()).expect("fixture");
        let group = report
            .metric_groups
            .iter()
            .find(|g| {
                g.stratum_id == "nominal"
                    && g.metric_kind == ForecastMetricKind::MissionDurationSeconds
            })
            .expect("mission duration group");
        assert_eq!(group.calibration_sample_count, 20);
        assert_eq!(group.holdout_sample_count, 30);
        assert!(group.conformal_adjustment.is_some());
    }

    #[test]
    fn held_out_deployment_shift_is_not_hidden_by_nominal_stratum() {
        let report = evaluate_service_prediction_calibration(&fixture()).expect("fixture");
        assert!(report.any_undercoverage_alarm);
        assert!(report.metric_groups.iter().any(|g| {
            g.stratum_id == "hot_shift" && g.status == CalibrationGroupStatus::UndercoverageAlarm
        }));
    }

    #[test]
    fn holdout_probability_scores_are_not_computed_from_calibration_outcomes() {
        let input = fixture();
        let baseline = evaluate_service_prediction_calibration(&input).expect("baseline");
        let baseline_hot = baseline
            .on_time_groups
            .iter()
            .find(|group| group.stratum_id == "hot_shift")
            .expect("hot shift probability group");
        assert_eq!(baseline_hot.calibration_sample_count, 20);
        assert_eq!(baseline_hot.holdout_sample_count, 30);
        assert_eq!(baseline_hot.holdout_brier_score_millionths, 330_000);

        let mut changed = fixture();
        for record in changed
            .records
            .iter_mut()
            .filter(|record| record.cohort == CalibrationCohort::Calibration)
        {
            record.on_time_observed = !record.on_time_observed;
            record.on_time_probability_permille = 0;
        }
        let changed_report = evaluate_service_prediction_calibration(&changed)
            .expect("calibration cohort changes remain auditable");
        let changed_hot = changed_report
            .on_time_groups
            .iter()
            .find(|group| group.stratum_id == "hot_shift")
            .expect("changed hot shift group");
        assert_eq!(
            changed_hot.holdout_brier_score_millionths,
            baseline_hot.holdout_brier_score_millionths
        );
        assert_eq!(
            changed_hot.holdout_observed_on_time_rate_permille,
            baseline_hot.holdout_observed_on_time_rate_permille
        );
    }

    #[test]
    fn coverage_report_keeps_width_and_wilson_support_separate() {
        let report = evaluate_service_prediction_calibration(&fixture()).expect("fixture");
        let nominal = report
            .metric_groups
            .iter()
            .find(|group| {
                group.stratum_id == "nominal"
                    && group.metric_kind == ForecastMetricKind::MissionDurationSeconds
            })
            .expect("nominal mission group");
        assert_eq!(nominal.conformal_adjustment, Some(70));
        assert_eq!(nominal.holdout_coverage_before_permille, Some(1_000));
        assert_eq!(nominal.holdout_coverage_after_permille, Some(1_000));
        assert_eq!(nominal.mean_interval_width_before, Some(160));
        assert_eq!(nominal.mean_interval_width_after, Some(300));
        assert!(
            nominal
                .holdout_coverage_after_wilson_95
                .as_ref()
                .expect("Wilson interval")
                .lower_permille
                < nominal.nominal_coverage_permille
        );
        assert_eq!(
            nominal.status,
            CalibrationGroupStatus::CoverageNotDemonstrated
        );
    }

    #[test]
    fn temporal_leakage_is_rejected() {
        let mut input = fixture();
        let holdout_time = input
            .records
            .iter()
            .filter(|r| r.cohort == CalibrationCohort::Holdout)
            .map(|r| r.decision_time_unix_ms)
            .min()
            .unwrap();
        let calibration = input
            .records
            .iter_mut()
            .find(|r| r.cohort == CalibrationCohort::Calibration)
            .unwrap();
        for m in &mut calibration.metrics {
            m.observation_evidence.observed_at_unix_ms = Some(holdout_time + 1);
        }
        let errors = evaluate_service_prediction_calibration(&input)
            .expect_err("future calibration outcome must not precede holdout forecast");
        assert!(
            errors
                .iter()
                .any(|e| e.starts_with("calibration_outcome_after_holdout_forecast"))
        );
    }

    #[test]
    fn synthetic_holdout_evidence_cannot_be_labelled_empirically_supported() {
        let mut input = fixture();
        let report = evaluate_service_prediction_calibration(&input).expect("synthetic fixture");
        assert!(!report.all_evidence_source_grade);
        assert!(!report.all_metric_groups_empirically_supported);
        assert!(
            report
                .metric_groups
                .iter()
                .all(|group| { group.status != CalibrationGroupStatus::EmpiricallySupported })
        );
        input.records[0].metrics[0]
            .observation_evidence
            .evidence_class = EvidenceClass::Measured;
        let mixed = evaluate_service_prediction_calibration(&input)
            .expect("mixed evidence remains auditable");
        assert!(!mixed.all_metric_groups_empirically_supported);
    }

    #[test]
    fn equal_timestamp_calibration_outcome_and_holdout_forecast_is_rejected() {
        let mut input = fixture();
        let first_holdout = input
            .records
            .iter()
            .find(|record| record.cohort == CalibrationCohort::Holdout)
            .expect("holdout record")
            .decision_time_unix_ms;
        let calibration = input
            .records
            .iter_mut()
            .find(|record| record.cohort == CalibrationCohort::Calibration)
            .expect("calibration record");
        calibration.on_time_evidence.observed_at_unix_ms = Some(first_holdout);
        for metric in &mut calibration.metrics {
            metric.observation_evidence.observed_at_unix_ms = Some(first_holdout);
        }
        let errors = evaluate_service_prediction_calibration(&input)
            .expect_err("equal timestamps do not establish outcome-before-prediction ordering");
        assert!(
            errors
                .iter()
                .any(|error| error.starts_with("calibration_outcome_after_holdout_forecast"))
        );
    }

    #[test]
    fn future_forecast_after_target_start_is_rejected() {
        let mut input = fixture();
        input.records[0].metrics[0]
            .forecast_evidence
            .observed_at_unix_ms = Some(input.records[0].target_start_unix_ms + 1);
        let errors = evaluate_service_prediction_calibration(&input)
            .expect_err("forecast after target start is leakage");
        assert!(
            errors
                .iter()
                .any(|e| e.contains("forecast_evidence_invalid"))
        );
    }

    #[test]
    fn cross_boundary_energy_comparison_is_rejected() {
        let mut input = fixture();
        let m = input.records[0]
            .metrics
            .iter_mut()
            .find(|m| m.metric_kind == ForecastMetricKind::PackTerminalEnergyWh)
            .unwrap();
        m.measurement_boundary = MeasurementBoundary::ChargerInput;
        let errors = evaluate_service_prediction_calibration(&input)
            .expect_err("pack-terminal and charger-input are not interchangeable");
        assert!(
            errors
                .iter()
                .any(|e| e.contains("metric_boundary_or_interval_invalid"))
        );
    }

    #[test]
    fn reused_observation_receipt_is_rejected() {
        let mut input = fixture();
        let ref_id = input.records[0].metrics[0]
            .observation_evidence
            .evidence_reference
            .clone();
        input.records[1].metrics[0]
            .observation_evidence
            .evidence_reference = ref_id;
        let errors = evaluate_service_prediction_calibration(&input)
            .expect_err("one measurement receipt cannot score two cases");
        assert!(
            errors
                .iter()
                .any(|e| e.contains("evidence_reference_reused"))
        );
    }

    #[test]
    fn wrong_asset_or_configuration_evidence_is_rejected() {
        let mut input = fixture();
        input.records[0].metrics[0].observation_evidence.asset_id = "different-pack".to_owned();
        let errors = evaluate_service_prediction_calibration(&input)
            .expect_err("observation must bind to exact pack");
        assert!(
            errors
                .iter()
                .any(|e| e.contains("metric_observation_evidence_invalid"))
        );
    }

    #[test]
    fn insufficient_calibration_samples_never_produce_qualification() {
        let mut input = fixture();
        input.minimum_calibration_samples_per_group = 100;
        let report =
            evaluate_service_prediction_calibration(&input).expect("audit remains reportable");
        assert!(
            report
                .metric_groups
                .iter()
                .all(|g| { g.status == CalibrationGroupStatus::InsufficientCalibrationSamples })
        );
        assert!(!report.all_metric_groups_empirically_supported);
    }

    #[test]
    fn probability_forecasts_report_brier_and_reliability_bins() {
        let report = evaluate_service_prediction_calibration(&fixture()).expect("fixture");
        for group in &report.on_time_groups {
            assert!(group.brier_score_millionths <= 1_000_000);
            assert!(!group.reliability_bins.is_empty());
            assert!(group.expected_calibration_error_permille <= 1_000);
        }
    }

    #[test]
    fn input_permutation_preserves_canonical_report() {
        let input = fixture();
        let mut permuted = fixture();
        permuted.records.reverse();
        for record in &mut permuted.records {
            record.metrics.reverse();
        }
        let a = evaluate_service_prediction_calibration(&input).expect("first");
        let b = evaluate_service_prediction_calibration(&permuted).expect("permuted");
        assert_eq!(
            serde_json::to_string(&a).expect("first JSON"),
            serde_json::to_string(&b).expect("permuted JSON")
        );
    }

    #[test]
    fn interval_with_point_outside_bounds_is_rejected() {
        let mut input = fixture();
        input.records[0].metrics[0].predicted_point =
            input.records[0].metrics[0].predicted_upper + 1;
        let errors = evaluate_service_prediction_calibration(&input)
            .expect_err("point forecast must be inside its interval");
        assert!(
            errors
                .iter()
                .any(|e| e.contains("metric_boundary_or_interval_invalid"))
        );
    }

    #[test]
    fn synthetic_evidence_never_grants_field_qualification() {
        let report = evaluate_service_prediction_calibration(&fixture()).expect("fixture");
        assert!(!report.all_evidence_source_grade);
        assert!(report.any_synthetic_evidence);
        assert!(!report.field_qualification_granted);
    }
}
