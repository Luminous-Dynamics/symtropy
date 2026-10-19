// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

mod uncertainty;
use uncertainty::{BootstrapPoint, estimate as estimate_uncertainty};

const SCHEMA_BYTES: &[u8] =
    include_bytes!("../../../docs/schemas/pirb-probability-forecast-v0.1.schema.json");

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    Synthetic,
    AuthorizedHistorical,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisUnit {
    CountryMonth,
    PrioGridMonth,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ForecastOutcome {
    Event,
    NoEvent,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SourceContract {
    provider: String,
    forecast_archive_id: String,
    model_id: String,
    model_version: String,
    forecast_variable: String,
    target_definition: String,
    analysis_unit: AnalysisUnit,
    threshold_definition: String,
    forecast_source_reference: String,
    forecast_archive_sha256: String,
    outcome_source_id: String,
    outcome_release_id: String,
    outcome_definition: String,
    outcome_source_reference: String,
    outcome_release_sha256: String,
    license_reference: String,
    attribution: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PrivacyPolicy {
    aggregate_only: bool,
    minimum_subgroup_outcomes: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResamplingPolicy {
    spatial_blocking_method_reference: String,
    spatial_block_map_sha256: String,
    temporal_block_length_months: usize,
    minimum_spatial_blocks: usize,
    minimum_temporal_blocks: usize,
    replicates: usize,
    minimum_valid_replicates: usize,
    seed: u64,
    confidence_level: f64,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TemporalSplit {
    forecast_origin_start_month: i64,
    training_end_exclusive_month: i64,
    calibration_end_exclusive_month: i64,
    test_end_exclusive_month: i64,
    evaluation_as_of_month: i64,
    minimum_climatology_history: usize,
    reliability_bin_count: usize,
    log_loss_epsilon: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ForecastRecord {
    forecast_id: String,
    forecast_release_id: String,
    unit_id: String,
    spatial_block_id: String,
    forecast_origin_month: i64,
    input_cutoff_month: i64,
    target_month: i64,
    lead_months: i64,
    probability: f64,
    outcome: ForecastOutcome,
    outcome_available_month: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ForecastDataset {
    schema_version: String,
    dataset_id: String,
    evidence_class: EvidenceClass,
    source: SourceContract,
    privacy: PrivacyPolicy,
    temporal_split: TemporalSplit,
    lead_months: i64,
    resampling: ResamplingPolicy,
    records: Vec<ForecastRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReliabilityBin {
    bin_index: usize,
    lower_inclusive: f64,
    upper_bound: f64,
    upper_inclusive: bool,
    count: usize,
    mean_probability: f64,
    observed_event_rate: f64,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ForecastMetrics {
    split: String,
    forecast_count: usize,
    scored_count: usize,
    unknown_outcome_count: usize,
    event_count: usize,
    no_event_count: usize,
    event_rate: Option<f64>,
    brier_score: Option<f64>,
    log_loss: Option<f64>,
    roc_auc: Option<f64>,
    average_precision: Option<f64>,
    expected_calibration_error: Option<f64>,
    reliability_bins: Vec<ReliabilityBin>,
    climatology_covered_count: usize,
    climatology_baseline_brier: Option<f64>,
    model_brier_on_climatology_covered_records: Option<f64>,
    brier_skill_score_vs_past_only_climatology: Option<f64>,
    baseline_insufficient_history_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForecastMetricBundle {
    training: ForecastMetrics,
    calibration: ForecastMetrics,
    test: ForecastMetrics,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnitMetrics {
    unit_id: String,
    scored_count: usize,
    distinct_target_months: usize,
    event_count: usize,
    no_event_count: usize,
    brier_score: f64,
    mean_probability: f64,
    observed_event_rate: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForecastEvaluationReceipt {
    receipt_version: String,
    evaluator: String,
    result: String,
    source_commit: String,
    source_commit_sha_valid: bool,
    input_sha256: String,
    schema_sha256: String,
    dependency_lock_sha256: String,
    rustc_version: String,
    cargo_version: String,
    dataset_id: String,
    evidence_class: EvidenceClass,
    provider: String,
    forecast_archive_id: String,
    model_id: String,
    model_version: String,
    forecast_variable: String,
    target_definition: String,
    analysis_unit: AnalysisUnit,
    lead_months: i64,
    threshold_definition: String,
    forecast_archive_sha256: String,
    forecast_source_reference: String,
    outcome_source_id: String,
    outcome_release_id: String,
    outcome_definition: String,
    outcome_release_sha256: String,
    forecast_release_ids: Vec<String>,
    forecast_record_count: usize,
    outcome_source_reference: String,
    license_reference: String,
    attribution: String,
    privacy: PrivacyPolicy,
    temporal_split: TemporalSplit,
    metrics: ForecastMetricBundle,
    test_unit_metrics: Vec<UnitMetrics>,
    test_suppressed_unit_count: usize,
    resampling: ResamplingPolicy,
    test_uncertainty: uncertainty::UncertaintyReport,
    probability_calibration_fitted: bool,
    causal_effect_estimated: bool,
    live_decision_authority: bool,
    execution_claim: String,
    claim_ceiling: String,
}

/// Validate the Draft 2020-12 contract and semantic chronology before scoring.
pub fn validate_dataset_json(input: &[u8]) -> Result<ForecastDataset, Vec<String>> {
    let schema: Value = serde_json::from_slice(SCHEMA_BYTES)
        .map_err(|error| vec![format!("checked-in schema JSON is invalid: {error}")])?;
    jsonschema::draft202012::meta::validate(&schema).map_err(|error| {
        vec![format!(
            "Draft 2020-12 schema meta-validation failed: {error}"
        )]
    })?;

    let instance: Value = serde_json::from_slice(input)
        .map_err(|error| vec![format!("forecast dataset JSON is invalid: {error}")])?;
    let validator = jsonschema::draft202012::new(&schema)
        .map_err(|error| vec![format!("cannot build forecast schema validator: {error}")])?;
    let schema_errors: Vec<String> = validator
        .iter_errors(&instance)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }

    let dataset: ForecastDataset = serde_json::from_value(instance)
        .map_err(|error| vec![format!("cannot decode forecast dataset: {error}")])?;
    let findings = semantic_errors(&dataset);
    if findings.is_empty() {
        Ok(dataset)
    } else {
        Err(findings)
    }
}

/// Evaluate archived probabilities against matured labels with strict temporal
/// partitions and an online, past-only climatology baseline.
pub fn evaluate(
    input: &[u8],
    source_commit: &str,
    rustc_version: &str,
    cargo_version: &str,
    lock_bytes: &[u8],
) -> Result<ForecastEvaluationReceipt, Vec<String>> {
    let source_commit_sha_valid = valid_commit_sha(source_commit);
    if source_commit != "UNPINNED_LOCAL_RUN" && !source_commit_sha_valid {
        return Err(vec![
            "source commit must be a lowercase 40-character Git SHA or UNPINNED_LOCAL_RUN"
                .to_owned(),
        ]);
    }

    let dataset = validate_dataset_json(input)?;
    let metrics = ForecastMetricBundle {
        training: compute_metrics(&dataset, SplitName::Training),
        calibration: compute_metrics(&dataset, SplitName::Calibration),
        test: compute_metrics(&dataset, SplitName::Test),
    };
    let (test_unit_metrics, test_suppressed_unit_count) =
        unit_metrics(&dataset, dataset.privacy.minimum_subgroup_outcomes);
    let test_uncertainty = compute_test_uncertainty(&dataset);

    Ok(ForecastEvaluationReceipt {
        receipt_version: "0.1".to_owned(),
        evaluator: "PIRB probability forecast evaluation".to_owned(),
        result: "pass".to_owned(),
        source_commit: source_commit.to_owned(),
        source_commit_sha_valid,
        input_sha256: sha256_hex(input),
        schema_sha256: sha256_hex(SCHEMA_BYTES),
        dependency_lock_sha256: sha256_hex(lock_bytes),
        rustc_version: rustc_version.to_owned(),
        cargo_version: cargo_version.to_owned(),
        dataset_id: dataset.dataset_id.clone(),
        evidence_class: dataset.evidence_class,
        provider: dataset.source.provider.clone(),
        forecast_archive_id: dataset.source.forecast_archive_id.clone(),
        model_id: dataset.source.model_id.clone(),
        model_version: dataset.source.model_version.clone(),
        forecast_variable: dataset.source.forecast_variable.clone(),
        target_definition: dataset.source.target_definition.clone(),
        analysis_unit: dataset.source.analysis_unit,
        lead_months: dataset.lead_months,
        threshold_definition: dataset.source.threshold_definition.clone(),
        forecast_archive_sha256: dataset.source.forecast_archive_sha256.clone(),
        forecast_source_reference: dataset.source.forecast_source_reference.clone(),
        outcome_source_id: dataset.source.outcome_source_id.clone(),
        outcome_release_id: dataset.source.outcome_release_id.clone(),
        outcome_definition: dataset.source.outcome_definition.clone(),
        outcome_release_sha256: dataset.source.outcome_release_sha256.clone(),
        forecast_release_ids: dataset
            .records
            .iter()
            .map(|record| record.forecast_release_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        forecast_record_count: dataset.records.len(),
        outcome_source_reference: dataset.source.outcome_source_reference.clone(),
        license_reference: dataset.source.license_reference.clone(),
        attribution: dataset.source.attribution.clone(),
        privacy: dataset.privacy.clone(),
        temporal_split: dataset.temporal_split,
        metrics,
        test_unit_metrics,
        test_suppressed_unit_count,
        resampling: dataset.resampling.clone(),
        test_uncertainty,
        probability_calibration_fitted: false,
        causal_effect_estimated: false,
        live_decision_authority: false,
        execution_claim: "descriptive_probability_forecast_performance_only".to_owned(),
        claim_ceiling: "A passing receipt establishes schema/semantic conformance and descriptive scores for the supplied archived forecasts and outcome vintage only. It does not establish source truth, representativeness, consent, authority legitimacy, intervention effectiveness, causal impact, reduced violence, or deployment safety.".to_owned(),
    })
}

#[derive(Debug, Clone, Copy)]
enum SplitName {
    Training,
    Calibration,
    Test,
}

impl SplitName {
    fn label(self) -> &'static str {
        match self {
            Self::Training => "training",
            Self::Calibration => "calibration",
            Self::Test => "test",
        }
    }
}

fn belongs_to_split(record: &ForecastRecord, dataset: &ForecastDataset, split: SplitName) -> bool {
    let t = dataset.temporal_split;
    match split {
        SplitName::Training => {
            record.forecast_origin_month >= t.forecast_origin_start_month
                && record.forecast_origin_month < t.training_end_exclusive_month
        }
        SplitName::Calibration => {
            record.forecast_origin_month >= t.training_end_exclusive_month
                && record.forecast_origin_month < t.calibration_end_exclusive_month
        }
        SplitName::Test => {
            record.forecast_origin_month >= t.calibration_end_exclusive_month
                && record.forecast_origin_month < t.test_end_exclusive_month
        }
    }
}

fn semantic_errors(dataset: &ForecastDataset) -> Vec<String> {
    let mut findings = Vec::new();
    let t = dataset.temporal_split;

    if dataset.schema_version != "0.1" {
        findings.push("schema_version must be 0.1".to_owned());
    }
    if !dataset.privacy.aggregate_only {
        findings.push("privacy.aggregate_only must be true".to_owned());
    }
    if dataset.privacy.minimum_subgroup_outcomes < 2 {
        findings.push("minimum_subgroup_outcomes must be at least 2".to_owned());
    }
    let resampling = &dataset.resampling;
    if resampling.spatial_blocking_method_reference.trim().is_empty() {
        findings.push("spatial_blocking_method_reference must not be empty".to_owned());
    }
    if !(1..=12).contains(&resampling.temporal_block_length_months)
        || resampling.minimum_spatial_blocks < 4
        || resampling.minimum_temporal_blocks < 6
        || !(200..=10_000).contains(&resampling.replicates)
        || resampling.minimum_valid_replicates < 100
        || resampling.minimum_valid_replicates > resampling.replicates
        || !resampling.confidence_level.is_finite()
        || resampling.confidence_level <= 0.5
        || resampling.confidence_level >= 1.0
    {
        findings.push("resampling support thresholds, replicate counts, and confidence level must be valid".to_owned());
    }
    if !(1..=36).contains(&dataset.lead_months) {
        findings.push(
            "dataset must declare exactly one valid forecast horizon from 1 to 36 months"
                .to_owned(),
        );
    }
    if dataset.evidence_class == EvidenceClass::AuthorizedHistorical {
        for (name, reference) in [
            ("provider", dataset.source.provider.as_str()),
            (
                "forecast_source_reference",
                dataset.source.forecast_source_reference.as_str(),
            ),
            (
                "outcome_source_reference",
                dataset.source.outcome_source_reference.as_str(),
            ),
            (
                "license_reference",
                dataset.source.license_reference.as_str(),
            ),
            (
                "spatial_blocking_method_reference",
                resampling.spatial_blocking_method_reference.as_str(),
            ),
        ] {
            if reference.trim().is_empty() || reference.starts_with("synthetic:") {
                findings.push(format!(
                    "authorized historical dataset requires a non-synthetic {name}"
                ));
            }
        }
    }
    if !(t.forecast_origin_start_month < t.training_end_exclusive_month
        && t.training_end_exclusive_month < t.calibration_end_exclusive_month
        && t.calibration_end_exclusive_month < t.test_end_exclusive_month
        && t.test_end_exclusive_month <= t.evaluation_as_of_month
        && t.minimum_climatology_history > 0
        && (2..=20).contains(&t.reliability_bin_count)
        && t.log_loss_epsilon.is_finite()
        && t.log_loss_epsilon > 0.0
        && t.log_loss_epsilon < 0.5)
    {
        findings.push("temporal partitions, history threshold, reliability bins, and log-loss epsilon must be valid and strictly ordered".to_owned());
    }

    let mut seen_forecast_ids = BTreeSet::new();
    let mut seen_forecast_keys = BTreeSet::new();
    let mut unit_spatial_blocks: BTreeMap<String, String> = BTreeMap::new();
    let mut outcomes_by_unit_month: BTreeMap<(String, i64), (ForecastOutcome, i64)> =
        BTreeMap::new();

    for record in &dataset.records {
        match unit_spatial_blocks.get(&record.unit_id) {
            Some(previous) if previous != &record.spatial_block_id => findings.push(format!(
                "{} changes spatial_block_id for a unit within one frozen dataset",
                record.forecast_id
            )),
            Some(_) => {}
            None => {
                unit_spatial_blocks.insert(
                    record.unit_id.clone(),
                    record.spatial_block_id.clone(),
                );
            }
        }
        if !seen_forecast_ids.insert(record.forecast_id.clone()) {
            findings.push(format!("{} duplicates a forecast_id", record.forecast_id));
        }
        if record.lead_months != dataset.lead_months {
            findings.push(format!(
                "{} lead_months differs from the dataset's declared horizon",
                record.forecast_id
            ));
        }
        let forecast_key = (
            record.unit_id.clone(),
            record.target_month,
            record.lead_months,
        );
        if !seen_forecast_keys.insert(forecast_key) {
            findings.push(format!(
                "{} duplicates a unit/target/horizon observation; a new release ID does not make the same outcome independent",
                record.forecast_id
            ));
        }

        if record.forecast_origin_month < t.forecast_origin_start_month
            || record.forecast_origin_month >= t.test_end_exclusive_month
        {
            findings.push(format!(
                "{} forecast issue month lies outside declared split bounds",
                record.forecast_id
            ));
        }
        if record.input_cutoff_month > record.forecast_origin_month {
            findings.push(format!(
                "{} uses an input cutoff later than the forecast issue month",
                record.forecast_id
            ));
        }
        if record.target_month > t.evaluation_as_of_month {
            findings.push(format!(
                "{} target month lies beyond evaluation_as_of_month",
                record.forecast_id
            ));
        }
        if record.target_month <= record.input_cutoff_month
            || record.target_month < record.forecast_origin_month
        {
            findings.push(format!(
                "{} target month must be after its input cutoff and on or after forecast origin",
                record.forecast_id
            ));
        }
        if record.target_month - record.input_cutoff_month != record.lead_months {
            findings.push(format!(
                "{} lead_months does not match target_month minus input_cutoff_month",
                record.forecast_id
            ));
        }
        if !(0.0..=1.0).contains(&record.probability) || !record.probability.is_finite() {
            findings.push(format!(
                "{} probability must be finite and within [0, 1]",
                record.forecast_id
            ));
        }

        match (record.outcome, record.outcome_available_month) {
            (ForecastOutcome::Unknown, None) => {}
            (ForecastOutcome::Unknown, Some(_)) => findings.push(format!(
                "{} unknown outcome cannot claim an availability month",
                record.forecast_id
            )),
            (_, None) => findings.push(format!(
                "{} known outcome requires an outcome_available_month",
                record.forecast_id
            )),
            (outcome, Some(available_month)) => {
                if outcome != ForecastOutcome::Unknown {
                    if available_month <= record.target_month {
                        findings.push(format!(
                            "{} outcome must be available after the target month has completed",
                            record.forecast_id
                        ));
                    }
                    if available_month > t.evaluation_as_of_month {
                        findings.push(format!(
                            "{} outcome was not mature by evaluation_as_of_month",
                            record.forecast_id
                        ));
                    }
                    let key = (record.unit_id.clone(), record.target_month);
                    match outcomes_by_unit_month.get(&key) {
                        Some((previous_outcome, previous_available))
                            if *previous_outcome != outcome
                                || *previous_available != available_month =>
                        {
                            findings.push(format!(
                                "{} conflicts with the frozen outcome vintage for this unit/month",
                                record.forecast_id
                            ));
                        }
                        Some(_) => {}
                        None => {
                            outcomes_by_unit_month.insert(key, (outcome, available_month));
                        }
                    }
                }
            }
        }

        // A forecast is eligible for tuning only if its label had matured by
        // the next partition boundary. Otherwise later labels could leak into
        // calibration choices or model selection.
        if let Some(available) = record.outcome_available_month
            && record.outcome != ForecastOutcome::Unknown
        {
            if belongs_to_split(record, dataset, SplitName::Training)
                && available >= t.training_end_exclusive_month
            {
                findings.push(format!(
                    "{} training outcome was not mature by the calibration boundary",
                    record.forecast_id
                ));
            }
            if belongs_to_split(record, dataset, SplitName::Calibration)
                && available >= t.calibration_end_exclusive_month
            {
                findings.push(format!(
                    "{} calibration outcome was not mature by the test boundary",
                    record.forecast_id
                ));
            }
        }
    }
    findings
}

#[derive(Clone, Copy)]
struct ScoredPoint {
    probability: f64,
    outcome: bool,
}

fn compute_test_uncertainty(dataset: &ForecastDataset) -> uncertainty::UncertaintyReport {
    let baseline_history = BaselineHistory::from_dataset(dataset);
    let points: Vec<BootstrapPoint> = dataset
        .records
        .iter()
        .filter(|record| belongs_to_split(record, dataset, SplitName::Test))
        .filter(|record| record.outcome != ForecastOutcome::Unknown)
        .map(|record| BootstrapPoint {
            spatial_block_id: record.spatial_block_id.clone(),
            target_month: record.target_month,
            probability: record.probability,
            outcome: record.outcome == ForecastOutcome::Event,
            baseline_probability: baseline_history.probability(
                record.input_cutoff_month,
                dataset.temporal_split.minimum_climatology_history,
            ),
        })
        .collect();
    estimate_uncertainty(
        &points,
        &dataset.resampling,
        dataset.temporal_split.log_loss_epsilon,
    )
}

fn compute_metrics(dataset: &ForecastDataset, split: SplitName) -> ForecastMetrics {
    let records: Vec<&ForecastRecord> = dataset
        .records
        .iter()
        .filter(|r| belongs_to_split(r, dataset, split))
        .collect();
    let known: Vec<&ForecastRecord> = records
        .iter()
        .copied()
        .filter(|r| r.outcome != ForecastOutcome::Unknown)
        .collect();
    let points: Vec<ScoredPoint> = known
        .iter()
        .map(|r| ScoredPoint {
            probability: r.probability,
            outcome: r.outcome == ForecastOutcome::Event,
        })
        .collect();

    let event_count = points.iter().filter(|p| p.outcome).count();
    let no_event_count = points.len() - event_count;
    let t = dataset.temporal_split;
    let epsilon = t.log_loss_epsilon;
    let brier_score = mean(points.iter().map(|p| {
        let y = if p.outcome { 1.0 } else { 0.0 };
        (p.probability - y).powi(2)
    }));
    let log_loss = mean(points.iter().map(|point| {
        let probability = point.probability.clamp(epsilon, 1.0 - epsilon);
        if point.outcome {
            -probability.ln()
        } else {
            -(1.0 - probability).ln()
        }
    }));
    let reliability_bins = build_reliability_bins(&points, t.reliability_bin_count);
    let ece = expected_calibration_error(&reliability_bins, points.len());
    let roc_auc = rank_roc_auc(&points);
    let average_precision = average_precision(&points);

    let baseline_history = BaselineHistory::from_dataset(dataset);
    let baseline_points: Vec<(f64, f64, bool)> = known
        .iter()
        .filter_map(|record| {
            baseline_history
                .probability(record.input_cutoff_month, t.minimum_climatology_history)
                .map(|baseline| {
                    (
                        baseline,
                        record.probability,
                        record.outcome == ForecastOutcome::Event,
                    )
                })
        })
        .collect();
    let climatology_covered_count = baseline_points.len();
    let baseline_brier = mean(baseline_points.iter().map(|(base, _, y)| {
        let target = if *y { 1.0 } else { 0.0 };
        (*base - target).powi(2)
    }));
    let model_brier_on_baseline = mean(baseline_points.iter().map(|(_, model, y)| {
        let target = if *y { 1.0 } else { 0.0 };
        (*model - target).powi(2)
    }));
    let brier_skill = match (baseline_brier, model_brier_on_baseline) {
        (Some(base), Some(model)) if base > f64::EPSILON => Some(1.0 - model / base),
        _ => None,
    };

    ForecastMetrics {
        split: split.label().to_owned(),
        forecast_count: records.len(),
        scored_count: points.len(),
        unknown_outcome_count: records.len() - points.len(),
        event_count,
        no_event_count,
        event_rate: if points.is_empty() {
            None
        } else {
            Some(event_count as f64 / points.len() as f64)
        },
        brier_score,
        log_loss,
        roc_auc,
        average_precision,
        expected_calibration_error: ece,
        reliability_bins,
        climatology_covered_count,
        climatology_baseline_brier: baseline_brier,
        model_brier_on_climatology_covered_records: model_brier_on_baseline,
        brier_skill_score_vs_past_only_climatology: brier_skill,
        baseline_insufficient_history_count: known.len() - climatology_covered_count,
    }
}

/// Online climatology built from unique, mature unit-month labels. A label
/// enters the prefix only at its declared availability month, so a future or
/// unreleased observation cannot improve an earlier forecast's baseline.
struct BaselineHistory {
    available_months: Vec<i64>,
    event_prefix: Vec<usize>,
}

impl BaselineHistory {
    fn from_dataset(dataset: &ForecastDataset) -> Self {
        let mut unique: BTreeMap<(String, i64), (bool, i64)> = BTreeMap::new();
        for record in &dataset.records {
            let Some(available_month) = record.outcome_available_month else {
                continue;
            };
            if record.outcome == ForecastOutcome::Unknown {
                continue;
            }
            let key = (record.unit_id.clone(), record.target_month);
            unique
                .entry(key)
                .or_insert((record.outcome == ForecastOutcome::Event, available_month));
        }

        let mut observations: Vec<(i64, bool)> = unique
            .values()
            .map(|(event, available)| (*available, *event))
            .collect();
        observations.sort_unstable_by_key(|(available, _)| *available);

        let available_months: Vec<i64> = observations
            .iter()
            .map(|(available, _)| *available)
            .collect();
        let mut event_prefix = Vec::with_capacity(observations.len() + 1);
        event_prefix.push(0);
        for (_, event) in &observations {
            event_prefix
                .push(event_prefix.last().copied().unwrap_or(0) + if *event { 1 } else { 0 });
        }
        Self {
            available_months,
            event_prefix,
        }
    }

    fn probability(&self, input_cutoff_month: i64, minimum_history: usize) -> Option<f64> {
        let count = self
            .available_months
            .partition_point(|available| *available <= input_cutoff_month);
        if count < minimum_history {
            return None;
        }
        Some(self.event_prefix[count] as f64 / count as f64)
    }
}

fn build_reliability_bins(points: &[ScoredPoint], bin_count: usize) -> Vec<ReliabilityBin> {
    let mut bins: Vec<Vec<ScoredPoint>> = vec![Vec::new(); bin_count];
    for point in points {
        let index = ((point.probability * bin_count as f64).floor() as usize).min(bin_count - 1);
        bins[index].push(*point);
    }
    bins.into_iter()
        .enumerate()
        .filter_map(|(index, points)| {
            if points.is_empty() {
                return None;
            }
            let count = points.len();
            let mean_probability = points.iter().map(|p| p.probability).sum::<f64>() / count as f64;
            let event_count = points.iter().filter(|p| p.outcome).count();
            Some(ReliabilityBin {
                bin_index: index,
                lower_inclusive: index as f64 / bin_count as f64,
                upper_bound: (index + 1) as f64 / bin_count as f64,
                upper_inclusive: index + 1 == bin_count,
                count,
                mean_probability,
                observed_event_rate: event_count as f64 / count as f64,
            })
        })
        .collect()
}

fn expected_calibration_error(bins: &[ReliabilityBin], total: usize) -> Option<f64> {
    if total == 0 {
        return None;
    }
    Some(
        bins.iter()
            .map(|bin| {
                (bin.count as f64 / total as f64)
                    * (bin.mean_probability - bin.observed_event_rate).abs()
            })
            .sum(),
    )
}

fn rank_roc_auc(points: &[ScoredPoint]) -> Option<f64> {
    let positives = points.iter().filter(|p| p.outcome).count();
    let negatives = points.len() - positives;
    if positives == 0 || negatives == 0 {
        return None;
    }
    let mut sorted = points.to_vec();
    sorted.sort_by(|a, b| a.probability.total_cmp(&b.probability));
    let mut positive_rank_sum = 0.0;
    let mut start = 0;
    while start < sorted.len() {
        let mut end = start + 1;
        while end < sorted.len() && sorted[end].probability == sorted[start].probability {
            end += 1;
        }
        let average_rank = ((start + 1) as f64 + end as f64) / 2.0;
        let group_positives = sorted[start..end].iter().filter(|p| p.outcome).count();
        positive_rank_sum += average_rank * group_positives as f64;
        start = end;
    }
    let wins = positive_rank_sum - (positives * (positives + 1) / 2) as f64;
    Some(wins / (positives * negatives) as f64)
}

fn average_precision(points: &[ScoredPoint]) -> Option<f64> {
    let positives = points.iter().filter(|p| p.outcome).count();
    if positives == 0 || positives == points.len() || points.is_empty() {
        return None;
    }
    let mut sorted = points.to_vec();
    sorted.sort_by(|a, b| b.probability.total_cmp(&a.probability));
    let mut true_positives = 0_usize;
    let mut seen = 0_usize;
    let mut score = 0.0;
    let mut start = 0;
    while start < sorted.len() {
        let mut end = start + 1;
        while end < sorted.len() && sorted[end].probability == sorted[start].probability {
            end += 1;
        }
        let group_positives = sorted[start..end].iter().filter(|p| p.outcome).count();
        true_positives += group_positives;
        seen += end - start;
        let precision = true_positives as f64 / seen as f64;
        score += (group_positives as f64 / positives as f64) * precision;
        start = end;
    }
    Some(score)
}

fn unit_metrics(dataset: &ForecastDataset, minimum_outcomes: usize) -> (Vec<UnitMetrics>, usize) {
    let test_records: Vec<&ForecastRecord> = dataset
        .records
        .iter()
        .filter(|r| belongs_to_split(r, dataset, SplitName::Test))
        .filter(|r| r.outcome != ForecastOutcome::Unknown)
        .collect();
    let mut groups: BTreeMap<String, Vec<&ForecastRecord>> = BTreeMap::new();
    for record in test_records {
        groups
            .entry(record.unit_id.clone())
            .or_default()
            .push(record);
    }

    let mut visible = Vec::new();
    let mut suppressed = 0;
    for (unit_id, records) in groups {
        let distinct_target_months: BTreeSet<i64> =
            records.iter().map(|record| record.target_month).collect();
        if distinct_target_months.len() < minimum_outcomes {
            suppressed += 1;
            continue;
        }
        let positives = records
            .iter()
            .filter(|r| r.outcome == ForecastOutcome::Event)
            .count();
        let brier = records
            .iter()
            .map(|r| {
                let y = if r.outcome == ForecastOutcome::Event {
                    1.0
                } else {
                    0.0
                };
                (r.probability - y).powi(2)
            })
            .sum::<f64>()
            / records.len() as f64;
        visible.push(UnitMetrics {
            unit_id,
            scored_count: records.len(),
            distinct_target_months: distinct_target_months.len(),
            event_count: positives,
            no_event_count: records.len() - positives,
            brier_score: brier,
            mean_probability: records.iter().map(|r| r.probability).sum::<f64>()
                / records.len() as f64,
            observed_event_rate: positives as f64 / records.len() as f64,
        });
    }
    (visible, suppressed)
}

fn mean(values: impl Iterator<Item = f64>) -> Option<f64> {
    let mut count = 0_usize;
    let mut total = 0.0;
    for value in values {
        count += 1;
        total += value;
    }
    if count == 0 {
        None
    } else {
        Some(total / count as f64)
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn valid_commit_sha(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &[u8] =
        include_bytes!("../../../docs/fixtures/pirb-probability-forecast-v0.1.json");
    const LOCK: &[u8] = b"synthetic-test-lock";

    fn dataset() -> ForecastDataset {
        validate_dataset_json(FIXTURE).expect("synthetic fixture must validate")
    }

    fn receipt() -> ForecastEvaluationReceipt {
        evaluate(
            FIXTURE,
            "abcdef0123456789abcdef0123456789abcdef01",
            "rustc 1.96.0",
            "cargo 1.96.0",
            LOCK,
        )
        .expect("synthetic fixture must evaluate")
    }

    #[test]
    fn schema_and_synthetic_fixture_validate() {
        let r = receipt();
        assert_eq!(r.result, "pass");
        assert_eq!(r.evidence_class, EvidenceClass::Synthetic);
        assert_eq!(r.lead_months, 1);
        assert_eq!(r.metrics.training.forecast_count, 8);
        assert_eq!(r.metrics.calibration.forecast_count, 4);
        assert_eq!(r.metrics.test.forecast_count, 6);
        assert_eq!(r.metrics.test.scored_count, 5);
        assert_eq!(r.metrics.test.unknown_outcome_count, 1);
        assert_eq!(r.forecast_record_count, 18);
        assert_eq!(r.forecast_release_ids.len(), 18);
    }

    #[test]
    fn brier_log_loss_auc_and_average_precision_are_reported() {
        let m = &receipt().metrics.test;
        assert!((m.brier_score.expect("score") - 0.281).abs() < 1e-12);
        assert!(m.log_loss.is_some_and(f64::is_finite));
        assert!((m.roc_auc.expect("both classes") - 0.5).abs() < 1e-12);
        assert!((m.average_precision.expect("both classes") - 0.7).abs() < 1e-12);
        assert!(m.expected_calibration_error.is_some());
    }

    #[test]
    fn unknown_labels_are_not_scored_as_negative() {
        let m = &receipt().metrics.test;
        assert_eq!(m.event_count, 2);
        assert_eq!(m.no_event_count, 3);
        assert_eq!(m.scored_count, 5);
        assert_eq!(m.unknown_outcome_count, 1);
    }

    #[test]
    fn past_only_climatology_has_coverage_and_skill_result() {
        let m = &receipt().metrics.test;
        assert_eq!(m.climatology_covered_count, 5);
        assert!(m.climatology_baseline_brier.is_some());
        assert!(m.brier_skill_score_vs_past_only_climatology.is_some());
    }

    #[test]
    fn subgroup_outputs_apply_minimum_denominator() {
        let r = receipt();
        assert_eq!(r.test_unit_metrics.len(), 2);
        assert_eq!(r.test_suppressed_unit_count, 1);
        assert!(
            r.test_unit_metrics
                .iter()
                .all(|group| group.distinct_target_months >= 2)
        );
    }

    #[test]
    fn baseline_excludes_outcomes_not_available_at_the_forecast_cutoff() {
        let mut not_yet_available = dataset();
        let test_record = not_yet_available
            .records
            .iter()
            .find(|record| record.forecast_id == "forecast-x1")
            .unwrap()
            .clone();
        not_yet_available.records[0].outcome_available_month =
            Some(test_record.input_cutoff_month + 1);
        let before = BaselineHistory::from_dataset(&not_yet_available).probability(
            test_record.input_cutoff_month,
            not_yet_available.temporal_split.minimum_climatology_history,
        );

        let mut revised_before_release = not_yet_available.clone();
        revised_before_release.records[0].outcome = ForecastOutcome::Event;
        let after = BaselineHistory::from_dataset(&revised_before_release).probability(
            test_record.input_cutoff_month,
            revised_before_release
                .temporal_split
                .minimum_climatology_history,
        );
        assert_eq!(before, after);
    }

    #[test]
    fn baseline_uses_only_unique_labels_matured_at_the_forecast_cutoff() {
        let d = dataset();
        let history = BaselineHistory::from_dataset(&d);
        assert!((history.probability(120, 3).unwrap() - 0.25).abs() < 1e-12);
        // forecast-x1's label is not available by cutoff 121, but is available
        // by cutoff 122; the second query may legitimately include it.
        assert!((history.probability(121, 3).unwrap() - 0.25).abs() < 1e-12);
        assert!((history.probability(122, 3).unwrap() - (4.0 / 13.0)).abs() < 1e-12);
    }

    #[test]
    fn duplicated_unit_month_with_conflicting_frozen_outcome_is_rejected() {
        let mut d = dataset();
        let mut repeated = d.records[0].clone();
        repeated.forecast_id = "forecast-duplicate".to_owned();
        repeated.forecast_release_id = "synthetic-repeat-run".to_owned();
        repeated.outcome = if repeated.outcome == ForecastOutcome::Event {
            ForecastOutcome::NoEvent
        } else {
            ForecastOutcome::Event
        };
        repeated.outcome_available_month = Some(repeated.target_month + 1);
        d.records.push(repeated);
        assert!(
            semantic_errors(&d)
                .iter()
                .any(|e| e.contains("conflicts with the frozen outcome vintage"))
        );
    }

    #[test]
    fn forecast_target_and_lead_must_be_future_consistent() {
        let mut d = dataset();
        d.records[0].lead_months = 2;
        assert!(
            semantic_errors(&d)
                .iter()
                .any(|e| e.contains("lead_months does not match"))
        );
    }

    #[test]
    fn training_labels_must_mature_before_calibration() {
        let mut d = dataset();
        d.records[0].outcome_available_month = Some(d.temporal_split.training_end_exclusive_month);
        assert!(
            semantic_errors(&d)
                .iter()
                .any(|e| e.contains("not mature by the calibration boundary"))
        );
    }

    #[test]
    fn calibration_labels_must_mature_before_test() {
        let mut d = dataset();
        let index = d
            .records
            .iter()
            .position(|r| r.forecast_id == "forecast-c1")
            .unwrap();
        d.records[index].outcome_available_month =
            Some(d.temporal_split.calibration_end_exclusive_month);
        assert!(
            semantic_errors(&d)
                .iter()
                .any(|e| e.contains("not mature by the test boundary"))
        );
    }

    #[test]
    fn forecast_targets_beyond_evaluation_as_of_are_rejected() {
        let mut d = dataset();
        let index = d
            .records
            .iter()
            .position(|r| r.forecast_id == "forecast-x6")
            .unwrap();
        d.records[index].target_month = d.temporal_split.evaluation_as_of_month + 1;
        d.records[index].lead_months =
            d.records[index].target_month - d.records[index].input_cutoff_month;
        assert!(
            semantic_errors(&d)
                .iter()
                .any(|e| e.contains("target month lies beyond evaluation_as_of_month"))
        );
    }

    #[test]
    fn probabilities_outside_unit_interval_are_rejected_by_schema() {
        let mut value: Value = serde_json::from_slice(FIXTURE).unwrap();
        value["records"][0]["probability"] = Value::from(1.1);
        let bytes = serde_json::to_vec(&value).unwrap();
        assert!(validate_dataset_json(&bytes).is_err());
    }

    #[test]
    fn duplicate_forecast_key_is_rejected() {
        let mut d = dataset();
        let mut repeated = d.records[0].clone();
        repeated.forecast_id = "forecast-new-id".to_owned();
        d.records.push(repeated);
        assert!(
            semantic_errors(&d)
                .iter()
                .any(|e| e.contains("unit/target/horizon observation"))
        );
    }

    #[test]
    fn duplicated_unit_target_horizon_is_rejected_even_with_a_new_release_id() {
        let mut d = dataset();
        let mut repeated = d.records[0].clone();
        repeated.forecast_id = "forecast-new-release".to_owned();
        repeated.forecast_release_id = "synthetic-revised-run".to_owned();
        d.records.push(repeated);
        assert!(
            semantic_errors(&d)
                .iter()
                .any(|e| e.contains("new release ID does not make the same outcome independent"))
        );
    }

    #[test]
    fn dataset_rejects_a_mixed_horizon() {
        let mut d = dataset();
        d.records[0].lead_months = 2;
        d.records[0].input_cutoff_month = d.records[0].target_month - 2;
        assert!(
            semantic_errors(&d)
                .iter()
                .any(|e| e.contains("differs from the dataset's declared horizon"))
        );
    }

    #[test]
    fn calibration_bins_include_probability_one_in_last_bin() {
        let points = [
            ScoredPoint {
                probability: 1.0,
                outcome: true,
            },
            ScoredPoint {
                probability: 0.0,
                outcome: false,
            },
        ];
        let bins = build_reliability_bins(&points, 5);
        assert_eq!(bins.iter().map(|b| b.count).sum::<usize>(), 2);
        assert!(
            bins.iter()
                .any(|b| b.bin_index == 4 && b.count == 1 && b.upper_inclusive)
        );
    }

    #[test]
    fn auc_and_average_precision_are_null_when_one_class_is_missing() {
        let points = [
            ScoredPoint {
                probability: 0.1,
                outcome: false,
            },
            ScoredPoint {
                probability: 0.3,
                outcome: false,
            },
        ];
        assert!(rank_roc_auc(&points).is_none());
        assert!(average_precision(&points).is_none());
    }

    #[test]
    fn roc_auc_and_average_precision_group_equal_probabilities() {
        let points = [
            ScoredPoint {
                probability: 0.5,
                outcome: true,
            },
            ScoredPoint {
                probability: 0.5,
                outcome: false,
            },
        ];
        assert!((rank_roc_auc(&points).unwrap() - 0.5).abs() < 1e-12);
        assert!((average_precision(&points).unwrap() - 0.5).abs() < 1e-12);
    }

    #[test]
    fn local_receipt_is_not_exact_head_evidence() {
        let r = evaluate(FIXTURE, "UNPINNED_LOCAL_RUN", "unknown", "unknown", LOCK).unwrap();
        assert!(!r.source_commit_sha_valid);
    }

    #[test]
    fn receipt_binds_input_schema_lock_and_non_claims() {
        let r = receipt();
        assert_eq!(r.input_sha256.len(), 64);
        assert_eq!(r.schema_sha256.len(), 64);
        assert_eq!(r.dependency_lock_sha256.len(), 64);
        assert!(!r.probability_calibration_fitted);
        assert!(!r.causal_effect_estimated);
        assert!(!r.live_decision_authority);
    }

    #[test]
    fn small_synthetic_test_panel_reports_uncertainty_not_estimable() {
        let r = receipt();
        let uncertainty = serde_json::to_value(&r.test_uncertainty).unwrap();
        assert_eq!(uncertainty["status"], "not_estimable");
        assert!(uncertainty["brier_score"].is_null());
        assert!(uncertainty["log_loss"].is_null());
        assert_eq!(uncertainty["spatial_block_count"], 3);
        assert_eq!(uncertainty["temporal_block_count"], 5);
    }

    #[test]
    fn unit_spatial_block_mapping_cannot_change_within_dataset() {
        let mut d = dataset();
        let index = d.records.iter().position(|record| record.unit_id == "unit-alpha" && record.forecast_id == "forecast-t4").unwrap();
        d.records[index].spatial_block_id = "block-conflicting".to_owned();
        assert!(
            semantic_errors(&d)
                .iter()
                .any(|finding| finding.contains("changes spatial_block_id"))
        );
    }

    #[test]
    fn authorized_historical_input_cannot_use_synthetic_references() {
        let mut d = dataset();
        d.evidence_class = EvidenceClass::AuthorizedHistorical;
        assert!(
            semantic_errors(&d)
                .iter()
                .any(|e| e.contains("non-synthetic"))
        );
    }
}
