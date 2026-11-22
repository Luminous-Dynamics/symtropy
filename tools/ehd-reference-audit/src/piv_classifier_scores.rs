//! Probability-level evaluation for PIV vector-validity classifiers.
//!
//! The predicted probability is P(vector is invalid). This module reports
//! ranking and strictly proper scoring rules alongside threshold-based
//! confusion metrics from piv_qualification.rs. It does not train a classifier
//! or infer physical truth from labels.
//!
//! ROC-AUC is computed with average ranks for tied scores. Log loss clips
//! probabilities to [1e-15, 1-1e-15] solely to keep finite scores for exact
//! zero/one predictions. Reliability bins and ECE are descriptive diagnostics;
//! ECE is binning-dependent and is not a proper scoring rule.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::piv_qualification::FlowRegion;

const PROBABILITY_EPSILON: f64 = 1.0e-15;
const RELIABILITY_BIN_COUNT: usize = 10;

#[derive(Clone, Debug, PartialEq)]
pub struct PivProbabilityObservation {
    pub sample_id: String,
    pub region: FlowRegion,
    /// Reference label: true means the vector is invalid.
    pub reference_invalid: bool,
    /// Predicted probability that the vector is invalid, in [0, 1].
    pub predicted_invalid_probability: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReliabilityBin {
    pub bin_index: usize,
    pub lower_probability: f64,
    pub upper_probability: f64,
    pub sample_count: usize,
    pub mean_predicted_probability: Option<f64>,
    pub observed_invalid_fraction: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PivProbabilityMetrics {
    pub sample_count: usize,
    pub invalid_reference_count: usize,
    pub valid_reference_count: usize,
    /// None when this cohort contains only one reference class.
    pub roc_auc_invalid_positive: Option<f64>,
    /// Mean binary cross entropy in nats/sample; lower is better.
    pub log_loss_nats: f64,
    /// Mean squared probability error; lower is better.
    pub brier_score: f64,
    /// Ten-bin expected calibration error. Descriptive, not a proper score.
    pub expected_calibration_error: f64,
    pub reliability_bins: Vec<ReliabilityBin>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PivProbabilityScoreReport {
    pub overall: PivProbabilityMetrics,
    /// All four physical regions are listed; None means no samples in that region.
    /// AUC is None when a present region contains only one reference class.
    pub by_region: Vec<(FlowRegion, Option<PivProbabilityMetrics>)>,
    pub limitations: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PivProbabilityScoreError {
    EmptyObservations,
    EmptySampleId,
    DuplicateSampleId { sample_id: String },
    NonFiniteProbability { sample_id: String },
    ProbabilityOutsideUnitInterval { sample_id: String },
}

impl fmt::Display for PivProbabilityScoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyObservations => write!(f, "PIV probability evaluation requires observations"),
            Self::EmptySampleId => write!(f, "PIV probability sample ID must be non-empty"),
            Self::DuplicateSampleId { sample_id } => {
                write!(f, "duplicate PIV probability sample ID: {sample_id}")
            }
            Self::NonFiniteProbability { sample_id } => {
                write!(f, "non-finite invalid-vector probability for sample {sample_id}")
            }
            Self::ProbabilityOutsideUnitInterval { sample_id } => {
                write!(f, "invalid-vector probability is outside [0,1] for sample {sample_id}")
            }
        }
    }
}

impl Error for PivProbabilityScoreError {}

fn rank_auc(
    observations: &[PivProbabilityObservation],
    positive_count: usize,
    negative_count: usize,
) -> Option<f64> {
    if positive_count == 0 || negative_count == 0 {
        return None;
    }

    let mut ranked: Vec<(f64, bool)> = observations
        .iter()
        .map(|observation| (
            observation.predicted_invalid_probability,
            observation.reference_invalid,
        ))
        .collect();
    ranked.sort_by(|left, right| left.0.total_cmp(&right.0));

    let mut positive_rank_sum = 0.0;
    let mut index = 0usize;
    while index < ranked.len() {
        let score = ranked[index].0;
        let mut end = index + 1;
        while end < ranked.len() && ranked[end].0 == score {
            end += 1;
        }

        // Ranks are one-based. Average ranks give tied positive/negative pairs
        // exactly half credit, as required by the Mann-Whitney AUC statistic.
        let first_rank = (index + 1) as f64;
        let last_rank = end as f64;
        let average_rank = 0.5 * (first_rank + last_rank);
        let positives_in_tie = ranked[index..end]
            .iter()
            .filter(|(_, is_positive)| *is_positive)
            .count();
        positive_rank_sum += positives_in_tie as f64 * average_rank;
        index = end;
    }

    let positives = positive_count as f64;
    let negatives = negative_count as f64;
    let auc = (positive_rank_sum - positives * (positives + 1.0) / 2.0)
        / (positives * negatives);
    Some(auc.clamp(0.0, 1.0))
}

fn evaluate_cohort(
    observations: &[PivProbabilityObservation],
) -> PivProbabilityMetrics {
    let sample_count = observations.len();
    let invalid_reference_count = observations
        .iter()
        .filter(|observation| observation.reference_invalid)
        .count();
    let valid_reference_count = sample_count - invalid_reference_count;

    let mut log_loss_sum = 0.0;
    let mut brier_sum = 0.0;
    let mut bin_counts = [0usize; RELIABILITY_BIN_COUNT];
    let mut bin_probability_sums = [0.0f64; RELIABILITY_BIN_COUNT];
    let mut bin_positive_counts = [0usize; RELIABILITY_BIN_COUNT];

    for observation in observations {
        let raw_probability = observation.predicted_invalid_probability;
        let clipped = raw_probability.clamp(
            PROBABILITY_EPSILON,
            1.0 - PROBABILITY_EPSILON,
        );
        let target = if observation.reference_invalid { 1.0 } else { 0.0 };
        let log_loss = if observation.reference_invalid {
            -clipped.ln()
        } else {
            -(1.0 - clipped).ln()
        };
        let error = raw_probability - target;
        log_loss_sum += log_loss;
        brier_sum += error * error;

        let bin_index = ((raw_probability * RELIABILITY_BIN_COUNT as f64) as usize)
            .min(RELIABILITY_BIN_COUNT - 1);
        bin_counts[bin_index] += 1;
        bin_probability_sums[bin_index] += raw_probability;
        bin_positive_counts[bin_index] += if observation.reference_invalid { 1 } else { 0 };
    }

    let mut expected_calibration_error = 0.0;
    let reliability_bins = (0..RELIABILITY_BIN_COUNT)
        .map(|bin_index| {
            let count = bin_counts[bin_index];
            let mean_probability = if count > 0 {
                Some(bin_probability_sums[bin_index] / count as f64)
            } else {
                None
            };
            let observed_fraction = if count > 0 {
                Some(bin_positive_counts[bin_index] as f64 / count as f64)
            } else {
                None
            };
            if let (Some(mean_probability), Some(observed_fraction)) =
                (mean_probability, observed_fraction)
            {
                expected_calibration_error +=
                    count as f64 / sample_count as f64
                        * (mean_probability - observed_fraction).abs();
            }
            ReliabilityBin {
                bin_index,
                lower_probability: bin_index as f64 / RELIABILITY_BIN_COUNT as f64,
                upper_probability: (bin_index + 1) as f64 / RELIABILITY_BIN_COUNT as f64,
                sample_count: count,
                mean_predicted_probability: mean_probability,
                observed_invalid_fraction: observed_fraction,
            }
        })
        .collect();

    PivProbabilityMetrics {
        sample_count,
        invalid_reference_count,
        valid_reference_count,
        roc_auc_invalid_positive: rank_auc(
            observations,
            invalid_reference_count,
            valid_reference_count,
        ),
        log_loss_nats: log_loss_sum / sample_count as f64,
        brier_score: brier_sum / sample_count as f64,
        expected_calibration_error,
        reliability_bins,
    }
}

/// Evaluate threshold-independent probability predictions overall and by
/// physical region. Every probability refers to the invalid-vector class;
/// class-polarity is therefore explicit and consistent with confusion metrics.
pub fn evaluate_piv_probability_scores(
    observations: &[PivProbabilityObservation],
) -> Result<PivProbabilityScoreReport, PivProbabilityScoreError> {
    if observations.is_empty() {
        return Err(PivProbabilityScoreError::EmptyObservations);
    }

    let mut sample_ids = BTreeSet::new();
    let mut regional: BTreeMap<FlowRegion, Vec<PivProbabilityObservation>> =
        BTreeMap::new();

    for observation in observations {
        if observation.sample_id.trim().is_empty() {
            return Err(PivProbabilityScoreError::EmptySampleId);
        }
        if !sample_ids.insert(observation.sample_id.clone()) {
            return Err(PivProbabilityScoreError::DuplicateSampleId {
                sample_id: observation.sample_id.clone(),
            });
        }
        let probability = observation.predicted_invalid_probability;
        if !probability.is_finite() {
            return Err(PivProbabilityScoreError::NonFiniteProbability {
                sample_id: observation.sample_id.clone(),
            });
        }
        if !(0.0..=1.0).contains(&probability) {
            return Err(PivProbabilityScoreError::ProbabilityOutsideUnitInterval {
                sample_id: observation.sample_id.clone(),
            });
        }
        regional
            .entry(observation.region)
            .or_default()
            .push(observation.clone());
    }

    let overall = evaluate_cohort(observations);
    let by_region = [
        FlowRegion::Bulk,
        FlowRegion::NearWall,
        FlowRegion::ShearLayer,
        FlowRegion::Wake,
    ]
    .into_iter()
    .map(|region| {
        let cohort = regional.remove(&region).unwrap_or_default();
        let metrics = if cohort.is_empty() {
            None
        } else {
            Some(evaluate_cohort(&cohort))
        };
        (region, metrics)
    })
    .collect();

    Ok(PivProbabilityScoreReport {
        overall,
        by_region,
        limitations: "Probability-score receipt only. Labels are supplied references, not physical truth. ROC-AUC is undefined for one-class cohorts; log loss clips exact zero/one probabilities at 1e-15. ECE depends on ten equal-width bins and must not replace proper scores. No classifier training, raw PIV processing or empirical EHD validation is performed.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation(
        sample_id: &str,
        region: FlowRegion,
        invalid: bool,
        probability: f64,
    ) -> PivProbabilityObservation {
        PivProbabilityObservation {
            sample_id: sample_id.to_owned(),
            region,
            reference_invalid: invalid,
            predicted_invalid_probability: probability,
        }
    }

    fn fixture() -> Vec<PivProbabilityObservation> {
        vec![
            observation("a", FlowRegion::Bulk, true, 0.9),
            observation("b", FlowRegion::Bulk, true, 0.8),
            observation("c", FlowRegion::Bulk, false, 0.2),
            observation("d", FlowRegion::Bulk, false, 0.1),
            observation("e", FlowRegion::NearWall, true, 0.85),
            observation("f", FlowRegion::NearWall, false, 0.15),
            observation("g", FlowRegion::ShearLayer, true, 0.75),
            observation("h", FlowRegion::ShearLayer, false, 0.25),
            observation("i", FlowRegion::Wake, true, 0.65),
            observation("j", FlowRegion::Wake, false, 0.35),
        ]
    }

    #[test]
    fn perfect_ranking_has_unit_auc_and_low_proper_scores() {
        let report = evaluate_piv_probability_scores(&fixture()).unwrap();
        assert_eq!(report.overall.roc_auc_invalid_positive, Some(1.0));
        assert!(report.overall.log_loss_nats < 0.25);
        assert!(report.overall.brier_score < 0.06);
    }

    #[test]
    fn reversed_ranking_has_zero_auc() {
        let observations = vec![
            observation("a", FlowRegion::Bulk, true, 0.1),
            observation("b", FlowRegion::Bulk, true, 0.2),
            observation("c", FlowRegion::Bulk, false, 0.8),
            observation("d", FlowRegion::Bulk, false, 0.9),
        ];
        let report = evaluate_piv_probability_scores(&observations).unwrap();
        assert_eq!(report.overall.roc_auc_invalid_positive, Some(0.0));
    }

    #[test]
    fn complementing_probability_and_label_preserves_auc_and_proper_scores() {
        let original = fixture();
        let complemented = original.iter().map(|row| PivProbabilityObservation {
            sample_id: row.sample_id.clone(),
            region: row.region,
            reference_invalid: !row.reference_invalid,
            // PIVPostProcess defines its probability as P(valid); this adapter
            // converts it to the module's P(invalid) convention.
            predicted_invalid_probability: 1.0 - row.predicted_invalid_probability,
        }).collect::<Vec<_>>();
        let first = evaluate_piv_probability_scores(&original).unwrap();
        let second = evaluate_piv_probability_scores(&complemented).unwrap();
        assert_eq!(
            first.overall.roc_auc_invalid_positive,
            second.overall.roc_auc_invalid_positive
        );
        assert!((first.overall.log_loss_nats - second.overall.log_loss_nats).abs() < 1.0e-12);
        assert!((first.overall.brier_score - second.overall.brier_score).abs() < 1.0e-12);
    }

    #[test]
    fn tied_scores_receive_half_credit() {
        let observations = vec![
            observation("a", FlowRegion::Bulk, true, 0.5),
            observation("b", FlowRegion::Bulk, false, 0.5),
        ];
        let report = evaluate_piv_probability_scores(&observations).unwrap();
        assert_eq!(report.overall.roc_auc_invalid_positive, Some(0.5));
    }

    #[test]
    fn log_loss_clips_exact_zero_and_one_without_nan() {
        let observations = vec![
            observation("a", FlowRegion::Bulk, true, 1.0),
            observation("b", FlowRegion::Bulk, false, 0.0),
            observation("c", FlowRegion::Bulk, true, 0.0),
            observation("d", FlowRegion::Bulk, false, 1.0),
        ];
        let report = evaluate_piv_probability_scores(&observations).unwrap();
        assert!(report.overall.log_loss_nats.is_finite());
        assert!(report.overall.log_loss_nats > 10.0);
    }

    #[test]
    fn brier_score_is_bounded_between_zero_and_one() {
        let report = evaluate_piv_probability_scores(&fixture()).unwrap();
        assert!((0.0..=1.0).contains(&report.overall.brier_score));
    }

    #[test]
    fn reliability_bins_account_for_every_sample() {
        let report = evaluate_piv_probability_scores(&fixture()).unwrap();
        assert_eq!(
            report.overall.reliability_bins.iter().map(|bin| bin.sample_count).sum::<usize>(),
            report.overall.sample_count
        );
        assert!(report.overall.reliability_bins.iter().any(|bin| {
            bin.sample_count > 0
                && bin.mean_predicted_probability.is_some()
                && bin.observed_invalid_fraction.is_some()
        }));
    }

    #[test]
    fn regional_scores_are_reported_without_pooling_away_weak_regions() {
        let observations = fixture();
        let report = evaluate_piv_probability_scores(&observations).unwrap();
        assert_eq!(report.by_region.len(), 4);
        assert!(report.by_region.iter().all(|(region, metrics)| {
            metrics.as_ref().map(|metrics| metrics.sample_count)
                == Some(observations.iter().filter(|row| row.region == *region).count())
        }));
    }

    #[test]
    fn one_class_region_keeps_proper_scores_and_marks_auc_undefined() {
        let observations = vec![
            observation("a", FlowRegion::NearWall, true, 0.7),
            observation("b", FlowRegion::NearWall, true, 0.8),
        ];
        let report = evaluate_piv_probability_scores(&observations).unwrap();
        let near_wall = report.by_region.iter()
            .find(|(region, _)| *region == FlowRegion::NearWall).unwrap();
        let metrics = near_wall.1.as_ref().unwrap();
        assert_eq!(metrics.roc_auc_invalid_positive, None);
        assert!(metrics.log_loss_nats.is_finite());
    }

    #[test]
    fn missing_region_is_explicitly_unavailable_not_nan() {
        let observations = vec![
            observation("a", FlowRegion::Bulk, true, 0.8),
            observation("b", FlowRegion::Bulk, false, 0.2),
        ];
        let report = evaluate_piv_probability_scores(&observations).unwrap();
        let wake = report.by_region.iter()
            .find(|(region, _)| *region == FlowRegion::Wake).unwrap();
        assert!(wake.1.is_none());
    }

    #[test]
    fn duplicate_sample_identity_is_rejected() {
        let observations = vec![
            observation("same", FlowRegion::Bulk, true, 0.8),
            observation("same", FlowRegion::Bulk, false, 0.2),
        ];
        assert!(matches!(
            evaluate_piv_probability_scores(&observations),
            Err(PivProbabilityScoreError::DuplicateSampleId { .. })
        ));
    }

    #[test]
    fn empty_cohort_is_rejected() {
        assert_eq!(
            evaluate_piv_probability_scores(&[]).unwrap_err(),
            PivProbabilityScoreError::EmptyObservations
        );
    }

    #[test]
    fn invalid_probabilities_are_rejected() {
        let outside = vec![observation("outside", FlowRegion::Bulk, true, 1.1)];
        assert!(matches!(
            evaluate_piv_probability_scores(&outside),
            Err(PivProbabilityScoreError::ProbabilityOutsideUnitInterval { .. })
        ));
        let non_finite = vec![observation("nan", FlowRegion::Bulk, true, f64::NAN)];
        assert!(matches!(
            evaluate_piv_probability_scores(&non_finite),
            Err(PivProbabilityScoreError::NonFiniteProbability { .. })
        ));
    }
}
