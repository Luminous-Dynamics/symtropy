// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Out-of-sample calibration diagnostics for finite scenario probabilities.
//!
//! This module scores forecast distributions against later-observed outcomes.
//! It does not create a forecast, certify a probability model, or authorize
//! operations. Synthetic arithmetic remains explicitly non-empirical.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const SCENARIO_CALIBRATION_SCHEMA_VERSION: u16 = 1;
pub const SCENARIO_CALIBRATION_BENCHMARK_ID: &str = "MFB-SCENARIO-CALIBRATION-001";
pub const CALIBRATION_PROBABILITY_TOTAL_BPS: u32 = 10_000;
pub const CALIBRATION_BINS: u16 = 10;
pub const MAX_CALIBRATION_OUTCOMES: usize = 32;
pub const MAX_CALIBRATION_FORECASTS: usize = 10_000;
const SHA256_PREFIX: &str = "sha256:";
const BPS_SQUARED: f64 = 100_000_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbabilityBasis {
    SyntheticArithmetic,
    HistoricalFrequency,
    ForecastModel,
    ExpertElicited,
    Unknown,
    StressOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationOutcomeClass {
    pub outcome_id: String,
    /// Training-only climatology/reference probability, in basis points.
    pub baseline_probability_bps: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForecastOutcomeProbability {
    pub outcome_id: String,
    pub probability_bps: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalibrationForecastCase {
    pub forecast_id: String,
    pub region_id: String,
    pub lead_time_bucket: String,
    pub issued_at_unix_ms: u64,
    pub valid_at_unix_ms: u64,
    pub observed_at_unix_ms: u64,
    pub observed_outcome_id: String,
    pub forecast_source_reference: String,
    pub observation_source_reference: String,
    pub outcome_probabilities: Vec<ForecastOutcomeProbability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioCalibrationCorpus {
    pub schema_version: u16,
    pub benchmark_id: String,
    pub dataset_id: String,
    pub dataset_version: String,
    pub dataset_license: String,
    pub geography: String,
    pub target_definition: String,
    pub forecast_model_id: String,
    pub forecast_model_version: String,
    pub forecast_model_digest_sha256: String,
    pub probability_basis: ProbabilityBasis,
    pub probability_evidence_reference: String,
    pub model_trained_through_unix_ms: u64,
    pub evaluation_start_unix_ms: u64,
    pub evaluation_end_unix_ms: u64,
    pub outcomes: Vec<CalibrationOutcomeClass>,
    pub forecasts: Vec<CalibrationForecastCase>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReliabilityBin {
    pub bin_index: u8,
    pub lower_probability_bps: u16,
    pub upper_probability_bps: u16,
    pub mean_forecast_probability_bps: u16,
    pub observed_frequency_bps: u16,
    pub sample_count: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutcomeCalibrationMetrics {
    pub outcome_id: String,
    pub forecast_count: u64,
    pub observed_event_count: u64,
    pub expected_calibration_error_bps: u16,
    pub reliability_bins: Vec<ReliabilityBin>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbabilityClaimClass {
    SyntheticArithmeticOnly,
    EmpiricalDiagnosticsPendingIndependentReview,
    UnqualifiedProbabilityProvenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioCalibrationReport {
    pub schema_version: u16,
    pub benchmark_id: String,
    pub dataset_id: String,
    pub dataset_version: String,
    pub geography: String,
    pub target_definition: String,
    pub forecast_model_id: String,
    pub forecast_model_version: String,
    pub probability_basis: ProbabilityBasis,
    pub probability_claim_class: ProbabilityClaimClass,
    pub probability_evidence_reference: String,
    pub model_trained_through_unix_ms: u64,
    pub evaluation_start_unix_ms: u64,
    pub evaluation_end_unix_ms: u64,
    pub forecast_count: usize,
    pub outcome_class_count: usize,
    pub multiclass_brier_score: f64,
    pub climatology_brier_score: f64,
    pub brier_skill_score_vs_training_climatology: Option<f64>,
    /// None means at least one observed outcome had zero forecast probability.
    pub log_loss_nats: Option<f64>,
    pub zero_probability_observed_outcome_count: u64,
    pub expected_calibration_error_bps: u16,
    pub outcomes: Vec<OutcomeCalibrationMetrics>,
    pub input_digest_sha256: String,
    pub calibration_diagnostic_complete: bool,
    pub field_risk_claim_authorized: bool,
    pub independent_review_required: bool,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Copy, Default)]
struct ReliabilityAccumulator {
    forecast_probability_sum_bps: u128,
    observed_event_sum_bps: u128,
    count: u64,
}

fn valid_sha256(value: &str) -> bool {
    let Some(hex) = value.strip_prefix(SHA256_PREFIX) else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn rounded_ratio(numerator: u128, denominator: u128) -> Result<u64, String> {
    if denominator == 0 {
        return Err("cannot round a ratio with zero denominator".into());
    }
    let rounded = numerator
        .checked_add(denominator / 2)
        .ok_or_else(|| "ratio rounding overflow".to_string())?
        / denominator;
    u64::try_from(rounded).map_err(|_| "ratio exceeds u64".into())
}

fn round_six(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

fn canonical_digest(corpus: &ScenarioCalibrationCorpus) -> Result<String, String> {
    let mut canonical = corpus.clone();
    canonical.outcomes.sort_by(|left, right| left.outcome_id.cmp(&right.outcome_id));
    canonical.forecasts.sort_by(|left, right| left.forecast_id.cmp(&right.forecast_id));
    for forecast in &mut canonical.forecasts {
        forecast.outcome_probabilities
            .sort_by(|left, right| left.outcome_id.cmp(&right.outcome_id));
    }
    let encoded = serde_json::to_vec(&canonical)
        .map_err(|error| format!("cannot serialize canonical calibration corpus: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(encoded)))
}

fn validate_corpus(corpus: &ScenarioCalibrationCorpus) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if corpus.schema_version != SCENARIO_CALIBRATION_SCHEMA_VERSION {
        errors.push(format!(
            "schema_version: expected {}, got {}",
            SCENARIO_CALIBRATION_SCHEMA_VERSION, corpus.schema_version
        ));
    }
    if corpus.benchmark_id != SCENARIO_CALIBRATION_BENCHMARK_ID {
        errors.push("benchmark_id: unsupported calibration benchmark".into());
    }
    for (name, value) in [
        ("dataset_id", corpus.dataset_id.as_str()),
        ("dataset_version", corpus.dataset_version.as_str()),
        ("dataset_license", corpus.dataset_license.as_str()),
        ("geography", corpus.geography.as_str()),
        ("target_definition", corpus.target_definition.as_str()),
        ("forecast_model_id", corpus.forecast_model_id.as_str()),
        ("forecast_model_version", corpus.forecast_model_version.as_str()),
        ("probability_evidence_reference", corpus.probability_evidence_reference.as_str()),
    ] {
        if value.trim().is_empty() {
            errors.push(format!("{name}: must be non-empty"));
        }
    }
    if !valid_sha256(&corpus.forecast_model_digest_sha256) {
        errors.push("forecast_model_digest_sha256: malformed SHA-256 digest".into());
    }
    if corpus.probability_basis == ProbabilityBasis::StressOnly {
        errors.push(
            "stress_only_scenarios: keep outside probability-weighted forecast calibration".into(),
        );
    }
    if corpus.evaluation_end_unix_ms <= corpus.evaluation_start_unix_ms {
        errors.push("evaluation window must be nonempty".into());
    }
    if corpus.model_trained_through_unix_ms >= corpus.evaluation_start_unix_ms {
        errors.push("model training cutoff must precede held-out evaluation window".into());
    }
    if corpus.outcomes.len() < 2 || corpus.outcomes.len() > MAX_CALIBRATION_OUTCOMES {
        errors.push(format!(
            "outcomes: expected 2..={MAX_CALIBRATION_OUTCOMES} outcome classes"
        ));
    }
    if !corpus.outcomes.iter().any(|outcome| outcome.outcome_id == "other") {
        errors.push("outcomes: explicit 'other' class is required to expose support misses".into());
    }
    if corpus.forecasts.is_empty() || corpus.forecasts.len() > MAX_CALIBRATION_FORECASTS {
        errors.push(format!(
            "forecasts: expected 1..={MAX_CALIBRATION_FORECASTS} cases"
        ));
    }

    let mut outcome_ids = BTreeSet::new();
    let mut baseline_sum_bps = 0_u32;
    for (index, outcome) in corpus.outcomes.iter().enumerate() {
        if outcome.outcome_id.trim().is_empty()
            || !outcome_ids.insert(outcome.outcome_id.as_str())
        {
            errors.push(format!("outcomes[{index}].outcome_id: nonempty unique ID required"));
        }
        baseline_sum_bps += u32::from(outcome.baseline_probability_bps);
    }
    if baseline_sum_bps != CALIBRATION_PROBABILITY_TOTAL_BPS {
        errors.push(format!(
            "outcomes.baseline_probability_bps: expected sum {CALIBRATION_PROBABILITY_TOTAL_BPS}, got {baseline_sum_bps}"
        ));
    }

    let mut forecast_ids = BTreeSet::new();
    for (index, forecast) in corpus.forecasts.iter().enumerate() {
        if forecast.forecast_id.trim().is_empty()
            || !forecast_ids.insert(forecast.forecast_id.as_str())
        {
            errors.push(format!("forecasts[{index}].forecast_id: nonempty unique ID required"));
        }
        if forecast.region_id.trim().is_empty() || forecast.lead_time_bucket.trim().is_empty() {
            errors.push(format!("forecasts[{index}]: region and lead-time bucket required"));
        }
        if forecast.forecast_source_reference.trim().is_empty()
            || forecast.observation_source_reference.trim().is_empty()
        {
            errors.push(format!("forecasts[{index}]: forecast and observation source references required"));
        }
        if forecast.issued_at_unix_ms < corpus.evaluation_start_unix_ms
            || forecast.issued_at_unix_ms > corpus.evaluation_end_unix_ms
        {
            errors.push(format!("forecasts[{index}]: issue time outside held-out evaluation window"));
        }
        if forecast.issued_at_unix_ms >= forecast.valid_at_unix_ms
            || forecast.valid_at_unix_ms > forecast.observed_at_unix_ms
        {
            errors.push(format!("forecasts[{index}]: require issue < valid time <= observation time"));
        }
        if corpus.model_trained_through_unix_ms >= forecast.issued_at_unix_ms {
            errors.push(format!("forecasts[{index}]: training cutoff leaks into forecast issue time"));
        }
        if forecast.outcome_probabilities.len() != corpus.outcomes.len() {
            errors.push(format!(
                "forecasts[{index}].outcome_probabilities: expected one probability for every outcome class"
            ));
        }
        let mut probabilities = BTreeMap::new();
        let mut probability_sum_bps = 0_u32;
        for probability in &forecast.outcome_probabilities {
            if !outcome_ids.contains(probability.outcome_id.as_str())
                || probabilities.insert(probability.outcome_id.as_str(), probability.probability_bps).is_some()
            {
                errors.push(format!("forecasts[{index}]: unknown or duplicate outcome probability"));
            }
            probability_sum_bps += u32::from(probability.probability_bps);
        }
        if probability_sum_bps != CALIBRATION_PROBABILITY_TOTAL_BPS {
            errors.push(format!(
                "forecasts[{index}].outcome_probabilities: expected sum {CALIBRATION_PROBABILITY_TOTAL_BPS}, got {probability_sum_bps}"
            ));
        }
        if !probabilities.contains_key(forecast.observed_outcome_id.as_str()) {
            errors.push(format!(
                "forecasts[{index}].observed_outcome_id: observed outcome missing from forecast support (use the explicit 'other' class)"
            ));
        }
    }
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

fn build_reliability_metrics(
    corpus: &ScenarioCalibrationCorpus,
    outcome_id: &str,
) -> Result<OutcomeCalibrationMetrics, String> {
    let mut bins = [ReliabilityAccumulator::default(); CALIBRATION_BINS as usize];
    let mut observed_event_count = 0_u64;
    for forecast in &corpus.forecasts {
        let probability = forecast.outcome_probabilities
            .iter()
            .find(|item| item.outcome_id == outcome_id)
            .ok_or_else(|| format!("missing probability for outcome {outcome_id}"))?
            .probability_bps;
        let bin_index = usize::from((probability / 1_000).min(9));
        let bin = &mut bins[bin_index];
        bin.forecast_probability_sum_bps += u128::from(probability);
        let observed = if forecast.observed_outcome_id == outcome_id { 1_u64 } else { 0_u64 };
        bin.observed_event_sum_bps += u128::from(observed) * u128::from(CALIBRATION_PROBABILITY_TOTAL_BPS);
        bin.count = bin.count.checked_add(1)
            .ok_or_else(|| "reliability bin count overflow".to_string())?;
        observed_event_count = observed_event_count.checked_add(observed)
            .ok_or_else(|| "observed event count overflow".to_string())?;
    }

    let mut reliability_bins = Vec::new();
    let mut weighted_absolute_gap_bps = 0_u128;
    let mut total_count = 0_u128;
    for (bin_index, bin) in bins.iter().enumerate() {
        if bin.count == 0 {
            continue;
        }
        let count = u128::from(bin.count);
        let mean_forecast = rounded_ratio(bin.forecast_probability_sum_bps, count)?;
        let observed_frequency = rounded_ratio(bin.observed_event_sum_bps, count)?;
        let gap = mean_forecast.abs_diff(observed_frequency);
        weighted_absolute_gap_bps += u128::from(gap) * count;
        total_count += count;
        let lower = (bin_index as u16) * 1_000;
        let upper = if bin_index == 9 { 10_000 } else { lower + 999 };
        reliability_bins.push(ReliabilityBin {
            bin_index: bin_index as u8,
            lower_probability_bps: lower,
            upper_probability_bps: upper,
            mean_forecast_probability_bps: u16::try_from(mean_forecast)
                .map_err(|_| "mean forecast probability exceeds u16".to_string())?,
            observed_frequency_bps: u16::try_from(observed_frequency)
                .map_err(|_| "observed frequency exceeds u16".to_string())?,
            sample_count: bin.count,
        });
    }
    let ece = rounded_ratio(weighted_absolute_gap_bps, total_count)?;
    Ok(OutcomeCalibrationMetrics {
        outcome_id: outcome_id.into(),
        forecast_count: corpus.forecasts.len() as u64,
        observed_event_count,
        expected_calibration_error_bps: u16::try_from(ece)
            .map_err(|_| "expected calibration error exceeds u16".to_string())?,
        reliability_bins,
    })
}

fn brier_score(
    corpus: &ScenarioCalibrationCorpus,
    baseline: bool,
) -> Result<f64, String> {
    let baseline_probabilities: BTreeMap<&str, u16> = corpus.outcomes
        .iter()
        .map(|outcome| (outcome.outcome_id.as_str(), outcome.baseline_probability_bps))
        .collect();
    let mut squared_error_sum = 0_u128;
    for forecast in &corpus.forecasts {
        for probability in &forecast.outcome_probabilities {
            let target = if forecast.observed_outcome_id == probability.outcome_id {
                CALIBRATION_PROBABILITY_TOTAL_BPS
            } else {
                0
            };
            let predicted = if baseline {
                u32::from(*baseline_probabilities.get(probability.outcome_id.as_str())
                    .ok_or_else(|| "baseline missing outcome probability".to_string())?)
            } else {
                u32::from(probability.probability_bps)
            };
            let difference = i64::from(predicted) - i64::from(target);
            squared_error_sum += u128::from(difference.unsigned_abs())
                .checked_mul(u128::from(difference.unsigned_abs()))
                .ok_or_else(|| "Brier score accumulation overflow".to_string())?;
        }
    }
    let denominator = (corpus.forecasts.len() as f64) * BPS_SQUARED;
    Ok(round_six(squared_error_sum as f64 / denominator))
}

fn log_loss(corpus: &ScenarioCalibrationCorpus) -> Result<(Option<f64>, u64), String> {
    let mut log_loss_sum = 0.0_f64;
    let mut zero_probability_count = 0_u64;
    for forecast in &corpus.forecasts {
        let probability = forecast.outcome_probabilities
            .iter()
            .find(|item| item.outcome_id == forecast.observed_outcome_id)
            .ok_or_else(|| "observed outcome probability missing".to_string())?
            .probability_bps;
        if probability == 0 {
            zero_probability_count = zero_probability_count.checked_add(1)
                .ok_or_else(|| "zero-probability count overflow".to_string())?;
        } else {
            log_loss_sum += -(f64::from(probability) / 10_000.0).ln();
        }
    }
    if zero_probability_count > 0 {
        Ok((None, zero_probability_count))
    } else {
        Ok((
            Some(round_six(log_loss_sum / corpus.forecasts.len() as f64)),
            zero_probability_count,
        ))
    }
}

/// Score held-out multiclass outcome probabilities using proper scoring rules
/// and class-wise reliability diagnostics. This never qualifies field risk.
pub fn evaluate_scenario_calibration(
    corpus: &ScenarioCalibrationCorpus,
) -> Result<ScenarioCalibrationReport, Vec<String>> {
    validate_corpus(corpus)?;
    let input_digest_sha256 = canonical_digest(corpus).map_err(|error| vec![error])?;
    let mut outcomes = corpus.outcomes.clone();
    outcomes.sort_by(|left, right| left.outcome_id.cmp(&right.outcome_id));
    let mut calibration_metrics = Vec::new();
    let mut weighted_ece_sum = 0_u128;
    for outcome in &outcomes {
        let metrics = build_reliability_metrics(corpus, &outcome.outcome_id)
            .map_err(|error| vec![error])?;
        let total = metrics.forecast_count as u128;
        weighted_ece_sum += u128::from(metrics.expected_calibration_error_bps) * total;
        calibration_metrics.push(metrics);
    }
    let ece_denominator = (corpus.forecasts.len() as u128)
        .checked_mul(corpus.outcomes.len() as u128)
        .ok_or_else(|| vec!["ECE denominator overflow".into()])?;
    let ece_bps = rounded_ratio(weighted_ece_sum, ece_denominator)
        .map_err(|error| vec![error])?;
    let model_brier = brier_score(corpus, false).map_err(|error| vec![error])?;
    let baseline_brier = brier_score(corpus, true).map_err(|error| vec![error])?;
    let skill = if baseline_brier > 0.0 {
        Some(round_six(1.0 - model_brier / baseline_brier))
    } else {
        None
    };
    let (loss, zero_probability_count) = log_loss(corpus).map_err(|error| vec![error])?;
    let claim_class = match corpus.probability_basis {
        ProbabilityBasis::SyntheticArithmetic => ProbabilityClaimClass::SyntheticArithmeticOnly,
        ProbabilityBasis::Unknown => ProbabilityClaimClass::UnqualifiedProbabilityProvenance,
        ProbabilityBasis::HistoricalFrequency
        | ProbabilityBasis::ForecastModel
        | ProbabilityBasis::ExpertElicited => {
            ProbabilityClaimClass::EmpiricalDiagnosticsPendingIndependentReview
        }
        ProbabilityBasis::StressOnly => {
            return Err(vec![
                "stress_only_scenarios: keep outside probability-weighted forecast calibration".into()
            ]);
        }
    };
    Ok(ScenarioCalibrationReport {
        schema_version: SCENARIO_CALIBRATION_SCHEMA_VERSION,
        benchmark_id: SCENARIO_CALIBRATION_BENCHMARK_ID.into(),
        dataset_id: corpus.dataset_id.clone(),
        dataset_version: corpus.dataset_version.clone(),
        geography: corpus.geography.clone(),
        target_definition: corpus.target_definition.clone(),
        forecast_model_id: corpus.forecast_model_id.clone(),
        forecast_model_version: corpus.forecast_model_version.clone(),
        probability_basis: corpus.probability_basis,
        probability_claim_class: claim_class,
        probability_evidence_reference: corpus.probability_evidence_reference.clone(),
        model_trained_through_unix_ms: corpus.model_trained_through_unix_ms,
        evaluation_start_unix_ms: corpus.evaluation_start_unix_ms,
        evaluation_end_unix_ms: corpus.evaluation_end_unix_ms,
        forecast_count: corpus.forecasts.len(),
        outcome_class_count: corpus.outcomes.len(),
        multiclass_brier_score: model_brier,
        climatology_brier_score: baseline_brier,
        brier_skill_score_vs_training_climatology: skill,
        log_loss_nats: loss,
        zero_probability_observed_outcome_count: zero_probability_count,
        expected_calibration_error_bps: u16::try_from(ece_bps)
            .map_err(|_| vec!["ECE exceeds u16".into()])?,
        outcomes: calibration_metrics,
        input_digest_sha256,
        calibration_diagnostic_complete: true,
        field_risk_claim_authorized: false,
        independent_review_required: true,
        claim_ceiling: "Held-out probability calibration diagnostics only; not a calibrated safety guarantee, flight authorization, field-risk bound or dispatch authority".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha(byte: char) -> String {
        format!("sha256:{}", byte.to_string().repeat(64))
    }

    fn outcome_probability(outcome_id: &str, probability_bps: u16) -> ForecastOutcomeProbability {
        ForecastOutcomeProbability {
            outcome_id: outcome_id.into(),
            probability_bps,
        }
    }

    fn corpus() -> ScenarioCalibrationCorpus {
        let outcomes = vec![
            CalibrationOutcomeClass { outcome_id: "normal".into(), baseline_probability_bps: 7_500 },
            CalibrationOutcomeClass { outcome_id: "surge".into(), baseline_probability_bps: 1_500 },
            CalibrationOutcomeClass { outcome_id: "weather_blocked".into(), baseline_probability_bps: 800 },
            CalibrationOutcomeClass { outcome_id: "other".into(), baseline_probability_bps: 200 },
        ];
        let observed = [
            "normal", "normal", "normal", "normal", "normal",
            "normal", "normal", "normal", "normal", "normal",
            "normal", "normal", "surge", "surge", "surge", "surge",
            "weather_blocked", "weather_blocked", "weather_blocked", "other",
        ];
        let forecasts = observed.iter().enumerate().map(|(index, observed_outcome_id)| {
            let probabilities = match *observed_outcome_id {
                "normal" => [7_000, 2_000, 800, 200],
                "surge" => [5_000, 4_000, 800, 200],
                "weather_blocked" => [7_000, 1_000, 1_800, 200],
                "other" => [7_000, 1_000, 500, 1_500],
                _ => unreachable!("test outcomes are finite"),
            };
            CalibrationForecastCase {
                forecast_id: format!("forecast-{index:02}"),
                region_id: if index % 2 == 0 { "rural" } else { "urban" }.into(),
                lead_time_bucket: if index % 3 == 0 { "same_day" } else { "next_day" }.into(),
                issued_at_unix_ms: 2_000 + index as u64 * 1_000,
                valid_at_unix_ms: 100_000 + index as u64 * 1_000,
                observed_at_unix_ms: 110_000 + index as u64 * 1_000,
                observed_outcome_id: (*observed_outcome_id).into(),
                forecast_source_reference: format!("synthetic://forecast/{index}"),
                observation_source_reference: format!("synthetic://observation/{index}"),
                outcome_probabilities: vec![
                    outcome_probability("normal", probabilities[0]),
                    outcome_probability("surge", probabilities[1]),
                    outcome_probability("weather_blocked", probabilities[2]),
                    outcome_probability("other", probabilities[3]),
                ],
            }
        }).collect();
        ScenarioCalibrationCorpus {
            schema_version: 1,
            benchmark_id: SCENARIO_CALIBRATION_BENCHMARK_ID.into(),
            dataset_id: "synthetic-demand-weather-calibration".into(),
            dataset_version: "0.1.0".into(),
            dataset_license: "synthetic-test-data".into(),
            geography: "synthetic-two-region".into(),
            target_definition: "joint demand/weather operating regime".into(),
            forecast_model_id: "synthetic-probability-model".into(),
            forecast_model_version: "0.1.0".into(),
            forecast_model_digest_sha256: sha('a'),
            probability_basis: ProbabilityBasis::SyntheticArithmetic,
            probability_evidence_reference: "synthetic://probability-basis".into(),
            model_trained_through_unix_ms: 1_000,
            evaluation_start_unix_ms: 2_000,
            evaluation_end_unix_ms: 100_000,
            outcomes,
            forecasts,
        }
    }

    #[test]
    fn scores_held_out_multiclass_probabilities_and_reports_claim_ceiling() {
        let report = evaluate_scenario_calibration(&corpus()).expect("valid corpus");
        assert_eq!(report.forecast_count, 20);
        assert_eq!(report.outcome_class_count, 4);
        assert!(report.multiclass_brier_score >= 0.0);
        assert!(report.climatology_brier_score >= 0.0);
        assert!(!report.field_risk_claim_authorized);
        assert!(report.independent_review_required);
        assert_eq!(report.probability_claim_class, ProbabilityClaimClass::SyntheticArithmeticOnly);
        assert_eq!(report.zero_probability_observed_outcome_count, 0);
    }

    #[test]
    fn pins_synthetic_brier_log_loss_and_reliability_arithmetic() {
        let report = evaluate_scenario_calibration(&corpus()).expect("valid synthetic corpus");
        assert_eq!(report.multiclass_brier_score, 0.442610);
        assert_eq!(report.climatology_brier_score, 0.605800);
        assert_eq!(report.brier_skill_score_vs_training_climatology, Some(0.269379));
        assert_eq!(report.log_loss_nats, Some(0.749339));
        assert_eq!(report.expected_calibration_error_bps, 1_628);
    }

    #[test]
    fn reliability_bins_partition_forecasts_for_each_outcome() {
        let report = evaluate_scenario_calibration(&corpus()).expect("valid corpus");
        for outcome in &report.outcomes {
            assert_eq!(
                outcome.reliability_bins.iter().map(|bin| bin.sample_count).sum::<u64>(),
                20
            );
            assert_eq!(outcome.forecast_count, 20);
        }
    }

    #[test]
    fn zero_probability_observed_outcome_makes_raw_log_loss_infinite() {
        let mut test = corpus();
        test.forecasts[0].outcome_probabilities[0].probability_bps = 0;
        test.forecasts[0].outcome_probabilities[1].probability_bps = 9_000;
        test.forecasts[0].outcome_probabilities[2].probability_bps = 800;
        test.forecasts[0].outcome_probabilities[3].probability_bps = 200;
        let report = evaluate_scenario_calibration(&test).expect("valid but zero-probability outcome");
        assert_eq!(report.log_loss_nats, None);
        assert_eq!(report.zero_probability_observed_outcome_count, 1);
    }

    #[test]
    fn rejects_future_training_leakage() {
        let mut test = corpus();
        test.model_trained_through_unix_ms = 2_001;
        assert!(evaluate_scenario_calibration(&test).is_err());
    }

    #[test]
    fn rejects_observation_before_forecast_valid_time() {
        let mut test = corpus();
        test.forecasts[0].observed_at_unix_ms = test.forecasts[0].valid_at_unix_ms - 1;
        assert!(evaluate_scenario_calibration(&test).is_err());
    }

    #[test]
    fn rejects_forecast_case_issued_outside_frozen_evaluation_window() {
        let mut test = corpus();
        test.forecasts[0].issued_at_unix_ms = 100_001;
        assert!(evaluate_scenario_calibration(&test).is_err());
    }

    #[test]
    fn requires_explicit_other_outcome_for_support_misses() {
        let mut test = corpus();
        test.outcomes.retain(|outcome| outcome.outcome_id != "other");
        for forecast in &mut test.forecasts {
            forecast.outcome_probabilities.retain(|item| item.outcome_id != "other");
        }
        assert!(evaluate_scenario_calibration(&test).is_err());
    }

    #[test]
    fn rejects_observed_outcome_not_in_declared_support() {
        let mut test = corpus();
        test.forecasts[0].observed_outcome_id = "unexpected-class".into();
        assert!(evaluate_scenario_calibration(&test).is_err());
    }

    #[test]
    fn refuses_to_score_stress_only_scenarios_as_probabilities() {
        let mut test = corpus();
        test.probability_basis = ProbabilityBasis::StressOnly;
        assert!(evaluate_scenario_calibration(&test).is_err());
    }

    #[test]
    fn unknown_probability_provenance_is_never_field_qualified() {
        let mut test = corpus();
        test.probability_basis = ProbabilityBasis::Unknown;
        let report = evaluate_scenario_calibration(&test).expect("arithmetic still scores");
        assert_eq!(report.probability_claim_class, ProbabilityClaimClass::UnqualifiedProbabilityProvenance);
        assert!(!report.field_risk_claim_authorized);
    }

    #[test]
    fn input_order_permutations_produce_identical_report_and_digest() {
        let mut test = corpus();
        let first = evaluate_scenario_calibration(&test).expect("first report");
        test.outcomes.reverse();
        test.forecasts.reverse();
        for forecast in &mut test.forecasts {
            forecast.outcome_probabilities.reverse();
        }
        let second = evaluate_scenario_calibration(&test).expect("permuted report");
        assert_eq!(first, second);
    }

    #[test]
    fn rejects_duplicate_forecast_ids_and_duplicate_outcome_probability() {
        let mut test = corpus();
        test.forecasts[1].forecast_id = test.forecasts[0].forecast_id.clone();
        assert!(evaluate_scenario_calibration(&test).is_err());
        let mut test = corpus();
        test.forecasts[0].outcome_probabilities.push(test.forecasts[0].outcome_probabilities[0].clone());
        assert!(evaluate_scenario_calibration(&test).is_err());
    }

    #[test]
    fn rejects_probability_totals_outside_ten_thousand_basis_points() {
        let mut test = corpus();
        test.forecasts[0].outcome_probabilities[0].probability_bps = 6_999;
        assert!(evaluate_scenario_calibration(&test).is_err());
    }
}
