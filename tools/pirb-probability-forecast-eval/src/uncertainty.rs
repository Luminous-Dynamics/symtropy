// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use super::ResamplingPolicy;
use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

const METHOD: &str =
    "two-way product bootstrap: spatial-cluster resampling x temporal moving blocks";

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum UncertaintyStatus {
    Estimated,
    NotEstimable,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub(super) struct ConfidenceInterval {
    lower: f64,
    upper: f64,
    valid_replicates: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct UncertaintyReport {
    pub(super) status: UncertaintyStatus,
    method: String,
    reason: Option<String>,
    confidence_level: f64,
    seed: u64,
    replicates_requested: usize,
    scored_record_count: usize,
    spatial_block_count: usize,
    temporal_span_months: usize,
    temporal_block_length_months: usize,
    temporal_block_count: usize,
    minimum_spatial_blocks: usize,
    minimum_temporal_blocks: usize,
    minimum_valid_replicates: usize,
    brier_score: Option<ConfidenceInterval>,
    log_loss: Option<ConfidenceInterval>,
    roc_auc: Option<ConfidenceInterval>,
    average_precision: Option<ConfidenceInterval>,
    brier_skill_score_vs_past_only_climatology: Option<ConfidenceInterval>,
}

#[derive(Debug, Clone)]
pub(super) struct BootstrapPoint {
    pub(super) spatial_block_id: String,
    pub(super) target_month: i64,
    pub(super) probability: f64,
    pub(super) outcome: bool,
    pub(super) baseline_probability: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
struct WeightedPoint {
    probability: f64,
    outcome: bool,
    weight: f64,
    baseline_probability: Option<f64>,
}

#[derive(Debug, Clone, Copy, Default)]
struct Scores {
    brier: Option<f64>,
    log_loss: Option<f64>,
    roc_auc: Option<f64>,
    average_precision: Option<f64>,
    brier_skill_score: Option<f64>,
}

/// Produces uncertainty intervals only when the declared spatial and temporal
/// support thresholds are met. The output is descriptive sampling uncertainty;
/// it is not a causal-effect interval or a guarantee that the block design
/// matches every real-world dependence structure.
pub(super) fn estimate(
    points: &[BootstrapPoint],
    policy: &ResamplingPolicy,
    log_loss_epsilon: f64,
) -> UncertaintyReport {
    let spatial_blocks: Vec<String> = points
        .iter()
        .map(|point| point.spatial_block_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let (minimum_month, maximum_month) = points
        .iter()
        .map(|point| point.target_month)
        .fold(None, |bounds, month| match bounds {
            None => Some((month, month)),
            Some((minimum, maximum)) => Some((minimum.min(month), maximum.max(month))),
        })
        .unwrap_or((0, -1));
    let temporal_span_months = if points.is_empty() {
        0
    } else {
        (maximum_month - minimum_month + 1) as usize
    };
    let temporal_block_count = temporal_span_months
        .checked_div(policy.temporal_block_length_months)
        .unwrap_or(0);

    let mut reasons = Vec::new();
    if points.is_empty() {
        reasons.push("no known-outcome test records are available to score".to_owned());
    }
    if spatial_blocks.len() < policy.minimum_spatial_blocks {
        reasons.push(format!(
            "only {} scored spatial blocks are available; {} are required",
            spatial_blocks.len(),
            policy.minimum_spatial_blocks
        ));
    }
    if temporal_block_count < policy.minimum_temporal_blocks {
        reasons.push(format!(
            "the scored time span yields {} complete temporal blocks; {} are required",
            temporal_block_count, policy.minimum_temporal_blocks
        ));
    }
    if !reasons.is_empty() {
        return empty_report(
            policy,
            points.len(),
            spatial_blocks.len(),
            temporal_span_months,
            temporal_block_count,
            reasons.join("; "),
        );
    }

    let span = temporal_span_months;
    let block_length = policy.temporal_block_length_months;
    let mut rng = SplitMix64::new(policy.seed);
    let mut brier_values = Vec::with_capacity(policy.replicates);
    let mut log_loss_values = Vec::with_capacity(policy.replicates);
    let mut auc_values = Vec::with_capacity(policy.replicates);
    let mut average_precision_values = Vec::with_capacity(policy.replicates);
    let mut skill_values = Vec::with_capacity(policy.replicates);

    for _ in 0..policy.replicates {
        let mut spatial_multiplicity: BTreeMap<&str, usize> = spatial_blocks
            .iter()
            .map(|block| (block.as_str(), 0))
            .collect();
        for _ in 0..spatial_blocks.len() {
            let selected = rng.below(spatial_blocks.len());
            let block = spatial_blocks[selected].as_str();
            *spatial_multiplicity.get_mut(block).expect("known block") += 1;
        }

        // Draw adjacent calendar-month blocks with replacement. The final
        // block is truncated so each replicate represents the same month span.
        let mut temporal_multiplicity: BTreeMap<i64, usize> = BTreeMap::new();
        let available_starts = span - block_length + 1;
        let mut sampled_months = 0;
        while sampled_months < span {
            let start = minimum_month + rng.below(available_starts) as i64;
            let take = block_length.min(span - sampled_months);
            for offset in 0..take {
                *temporal_multiplicity
                    .entry(start + offset as i64)
                    .or_insert(0) += 1;
            }
            sampled_months += take;
        }

        let weighted: Vec<WeightedPoint> = points
            .iter()
            .filter_map(|point| {
                let spatial = *spatial_multiplicity
                    .get(point.spatial_block_id.as_str())
                    .unwrap_or(&0);
                let temporal = *temporal_multiplicity.get(&point.target_month).unwrap_or(&0);
                let weight = spatial * temporal;
                (weight > 0).then_some(WeightedPoint {
                    probability: point.probability,
                    outcome: point.outcome,
                    weight: weight as f64,
                    baseline_probability: point.baseline_probability,
                })
            })
            .collect();

        let scores = weighted_scores(&weighted, log_loss_epsilon);
        if let Some(value) = scores.brier {
            brier_values.push(value);
        }
        if let Some(value) = scores.log_loss {
            log_loss_values.push(value);
        }
        if let Some(value) = scores.roc_auc {
            auc_values.push(value);
        }
        if let Some(value) = scores.average_precision {
            average_precision_values.push(value);
        }
        if let Some(value) = scores.brier_skill_score {
            skill_values.push(value);
        }
    }

    let brier_interval = percentile_interval(
        brier_values,
        policy.confidence_level,
        policy.minimum_valid_replicates,
    );
    let log_loss_interval = percentile_interval(
        log_loss_values,
        policy.confidence_level,
        policy.minimum_valid_replicates,
    );
    let auc_interval = percentile_interval(
        auc_values,
        policy.confidence_level,
        policy.minimum_valid_replicates,
    );
    let average_precision_interval = percentile_interval(
        average_precision_values,
        policy.confidence_level,
        policy.minimum_valid_replicates,
    );
    let skill_interval = percentile_interval(
        skill_values,
        policy.confidence_level,
        policy.minimum_valid_replicates,
    );

    let primary_available = brier_interval.is_some() && log_loss_interval.is_some();
    UncertaintyReport {
        status: if primary_available {
            UncertaintyStatus::Estimated
        } else {
            UncertaintyStatus::NotEstimable
        },
        method: METHOD.to_owned(),
        reason: if primary_available {
            None
        } else {
            Some(format!(
                "primary score intervals did not reach the required {} valid replicates (Brier: {}, log loss: {})",
                policy.minimum_valid_replicates,
                brier_interval.as_ref().map_or(0, |ci| ci.valid_replicates),
                log_loss_interval
                    .as_ref()
                    .map_or(0, |ci| ci.valid_replicates)
            ))
        },
        confidence_level: policy.confidence_level,
        seed: policy.seed,
        replicates_requested: policy.replicates,
        scored_record_count: points.len(),
        spatial_block_count: spatial_blocks.len(),
        temporal_span_months,
        temporal_block_length_months: block_length,
        temporal_block_count,
        minimum_spatial_blocks: policy.minimum_spatial_blocks,
        minimum_temporal_blocks: policy.minimum_temporal_blocks,
        minimum_valid_replicates: policy.minimum_valid_replicates,
        brier_score: brier_interval,
        log_loss: log_loss_interval,
        roc_auc: auc_interval,
        average_precision: average_precision_interval,
        brier_skill_score_vs_past_only_climatology: skill_interval,
    }
}

fn empty_report(
    policy: &ResamplingPolicy,
    scored_record_count: usize,
    spatial_block_count: usize,
    temporal_span_months: usize,
    temporal_block_count: usize,
    reason: String,
) -> UncertaintyReport {
    UncertaintyReport {
        status: UncertaintyStatus::NotEstimable,
        method: METHOD.to_owned(),
        reason: Some(reason),
        confidence_level: policy.confidence_level,
        seed: policy.seed,
        replicates_requested: policy.replicates,
        scored_record_count,
        spatial_block_count,
        temporal_span_months,
        temporal_block_length_months: policy.temporal_block_length_months,
        temporal_block_count,
        minimum_spatial_blocks: policy.minimum_spatial_blocks,
        minimum_temporal_blocks: policy.minimum_temporal_blocks,
        minimum_valid_replicates: policy.minimum_valid_replicates,
        brier_score: None,
        log_loss: None,
        roc_auc: None,
        average_precision: None,
        brier_skill_score_vs_past_only_climatology: None,
    }
}

fn weighted_scores(points: &[WeightedPoint], epsilon: f64) -> Scores {
    let total_weight: f64 = points.iter().map(|point| point.weight).sum();
    if total_weight <= 0.0 {
        return Scores::default();
    }

    let brier = points
        .iter()
        .map(|point| {
            let target = if point.outcome { 1.0 } else { 0.0 };
            point.weight * (point.probability - target).powi(2)
        })
        .sum::<f64>()
        / total_weight;
    let log_loss = points
        .iter()
        .map(|point| {
            let probability = point.probability.clamp(epsilon, 1.0 - epsilon);
            let loss = if point.outcome {
                -probability.ln()
            } else {
                -(1.0 - probability).ln()
            };
            point.weight * loss
        })
        .sum::<f64>()
        / total_weight;

    let roc_auc = weighted_roc_auc(points);
    let average_precision = weighted_average_precision(points);

    let baseline_weight: f64 = points
        .iter()
        .filter(|point| point.baseline_probability.is_some())
        .map(|point| point.weight)
        .sum();
    let brier_skill_score = if baseline_weight <= 0.0 {
        None
    } else {
        let model_brier = points
            .iter()
            .filter_map(|point| {
                point.baseline_probability.map(|_| {
                    let target = if point.outcome { 1.0 } else { 0.0 };
                    point.weight * (point.probability - target).powi(2)
                })
            })
            .sum::<f64>()
            / baseline_weight;
        let baseline_brier = points
            .iter()
            .filter_map(|point| {
                point.baseline_probability.map(|probability| {
                    let target = if point.outcome { 1.0 } else { 0.0 };
                    point.weight * (probability - target).powi(2)
                })
            })
            .sum::<f64>()
            / baseline_weight;
        (baseline_brier > f64::EPSILON).then_some(1.0 - model_brier / baseline_brier)
    };

    Scores {
        brier: Some(brier),
        log_loss: Some(log_loss),
        roc_auc,
        average_precision,
        brier_skill_score,
    }
}

fn weighted_roc_auc(points: &[WeightedPoint]) -> Option<f64> {
    let total_positive: f64 = points
        .iter()
        .filter(|point| point.outcome)
        .map(|point| point.weight)
        .sum();
    let total_negative: f64 = points
        .iter()
        .filter(|point| !point.outcome)
        .map(|point| point.weight)
        .sum();
    if total_positive <= 0.0 || total_negative <= 0.0 {
        return None;
    }

    let mut ordered = points.to_vec();
    ordered.sort_by(|left, right| left.probability.total_cmp(&right.probability));
    let mut negatives_below = 0.0;
    let mut concordant_pairs = 0.0;
    let mut start = 0;
    while start < ordered.len() {
        let mut end = start + 1;
        while end < ordered.len() && ordered[end].probability == ordered[start].probability {
            end += 1;
        }
        let group_positive: f64 = ordered[start..end]
            .iter()
            .filter(|point| point.outcome)
            .map(|point| point.weight)
            .sum();
        let group_negative: f64 = ordered[start..end]
            .iter()
            .filter(|point| !point.outcome)
            .map(|point| point.weight)
            .sum();
        concordant_pairs += group_positive * (negatives_below + 0.5 * group_negative);
        negatives_below += group_negative;
        start = end;
    }
    Some(concordant_pairs / (total_positive * total_negative))
}

fn weighted_average_precision(points: &[WeightedPoint]) -> Option<f64> {
    let total_positive: f64 = points
        .iter()
        .filter(|point| point.outcome)
        .map(|point| point.weight)
        .sum();
    let total_negative: f64 = points
        .iter()
        .filter(|point| !point.outcome)
        .map(|point| point.weight)
        .sum();
    if total_positive <= 0.0 || total_negative <= 0.0 {
        return None;
    }

    let mut ordered = points.to_vec();
    ordered.sort_by(|left, right| right.probability.total_cmp(&left.probability));
    let mut true_positives = 0.0;
    let mut seen = 0.0;
    let mut score = 0.0;
    let mut start = 0;
    while start < ordered.len() {
        let mut end = start + 1;
        while end < ordered.len() && ordered[end].probability == ordered[start].probability {
            end += 1;
        }
        let group_positive: f64 = ordered[start..end]
            .iter()
            .filter(|point| point.outcome)
            .map(|point| point.weight)
            .sum();
        let group_weight: f64 = ordered[start..end].iter().map(|point| point.weight).sum();
        true_positives += group_positive;
        seen += group_weight;
        score += (group_positive / total_positive) * (true_positives / seen);
        start = end;
    }
    Some(score)
}

fn percentile_interval(
    mut values: Vec<f64>,
    confidence_level: f64,
    minimum_valid_replicates: usize,
) -> Option<ConfidenceInterval> {
    if values.len() < minimum_valid_replicates {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let alpha = (1.0 - confidence_level) / 2.0;
    Some(ConfidenceInterval {
        lower: quantile(&values, alpha),
        upper: quantile(&values, 1.0 - alpha),
        valid_replicates: values.len(),
    })
}

fn quantile(sorted: &[f64], probability: f64) -> f64 {
    let position = probability * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let weight = position - lower as f64;
        sorted[lower] * (1.0 - weight) + sorted[upper] * weight
    }
}

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D049BB133111EB);
        value ^ (value >> 31)
    }

    fn below(&mut self, upper_exclusive: usize) -> usize {
        ((self.next() as u128 * upper_exclusive as u128) >> 64) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> ResamplingPolicy {
        ResamplingPolicy {
            spatial_blocking_method_reference: "synthetic:blocked-panel".to_owned(),
            spatial_block_map_sha256: "c".repeat(64),
            temporal_block_length_months: 2,
            minimum_spatial_blocks: 4,
            minimum_temporal_blocks: 6,
            replicates: 200,
            minimum_valid_replicates: 100,
            seed: 20261019,
            confidence_level: 0.95,
        }
    }

    fn supported_panel() -> Vec<BootstrapPoint> {
        let mut points = Vec::new();
        for block in 0..6 {
            for month in 100..112 {
                let outcome = (block + month as usize).is_multiple_of(3);
                let probability = if outcome { 0.65 } else { 0.25 };
                points.push(BootstrapPoint {
                    spatial_block_id: format!("block-{block}"),
                    target_month: month,
                    probability,
                    outcome,
                    baseline_probability: Some(0.2),
                });
            }
        }
        points
    }

    #[test]
    fn small_fixture_support_is_not_given_fabricated_intervals() {
        let points = vec![
            BootstrapPoint {
                spatial_block_id: "block-a".to_owned(),
                target_month: 100,
                probability: 0.7,
                outcome: true,
                baseline_probability: Some(0.2),
            },
            BootstrapPoint {
                spatial_block_id: "block-b".to_owned(),
                target_month: 101,
                probability: 0.3,
                outcome: false,
                baseline_probability: Some(0.2),
            },
            BootstrapPoint {
                spatial_block_id: "block-c".to_owned(),
                target_month: 102,
                probability: 0.4,
                outcome: false,
                baseline_probability: Some(0.2),
            },
        ];
        let report = estimate(&points, &policy(), 1e-6);
        assert_eq!(report.status, UncertaintyStatus::NotEstimable);
        assert!(report.reason.is_some());
        assert!(report.brier_score.is_none());
        assert!(report.log_loss.is_none());
    }

    #[test]
    fn supported_panel_gets_deterministic_intervals() {
        let points = supported_panel();
        let first = estimate(&points, &policy(), 1e-6);
        let second = estimate(&points, &policy(), 1e-6);
        assert_eq!(first.status, UncertaintyStatus::Estimated);
        assert!(first.brier_score.is_some());
        assert!(first.log_loss.is_some());
        assert!(first.roc_auc.is_some());
        assert!(first.average_precision.is_some());
        assert!(first.brier_skill_score_vs_past_only_climatology.is_some());
        assert_eq!(
            serde_json::to_value(&first).unwrap(),
            serde_json::to_value(&second).unwrap()
        );
        let brier = first.brier_score.unwrap();
        assert!(brier.lower <= brier.upper);
        assert!(brier.valid_replicates >= policy().minimum_valid_replicates);
    }

    #[test]
    fn tied_weighted_scores_match_two_point_oracle() {
        let points = vec![
            WeightedPoint {
                probability: 0.5,
                outcome: true,
                weight: 1.0,
                baseline_probability: Some(0.2),
            },
            WeightedPoint {
                probability: 0.5,
                outcome: false,
                weight: 1.0,
                baseline_probability: Some(0.2),
            },
        ];
        assert!((weighted_roc_auc(&points).unwrap() - 0.5).abs() < 1e-12);
        assert!((weighted_average_precision(&points).unwrap() - 0.5).abs() < 1e-12);
    }
}
