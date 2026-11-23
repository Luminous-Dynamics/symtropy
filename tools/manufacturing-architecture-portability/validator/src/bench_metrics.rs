// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Independent benchmark metrics for feature-selection stability and run-level leakage.
//!
//! This module consumes declared feature subsets and run identifiers only. It does
//! not fit a feature selector, inspect labels, or access a held-out evaluation set.

use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureSubsetRun {
    pub run_id: String,
    pub selected_features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeatureStabilityReport {
    pub run_count: usize,
    pub pair_count: usize,
    pub universe_size: usize,
    pub mean_selected_feature_count: f64,
    pub selected_feature_count_stddev: f64,
    pub mean_jaccard: f64,
    pub median_jaccard: f64,
    pub p10_jaccard: f64,
    pub minimum_jaccard: f64,
    pub kuncheva_pair_count: usize,
    pub mean_kuncheva: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunPartitionManifest {
    pub training_run_ids: BTreeSet<String>,
    pub validation_run_ids: BTreeSet<String>,
    pub test_run_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionLeakage {
    pub train_validation: Vec<String>,
    pub train_test: Vec<String>,
    pub validation_test: Vec<String>,
}

impl PartitionLeakage {
    pub fn is_empty(&self) -> bool {
        self.train_validation.is_empty()
            && self.train_test.is_empty()
            && self.validation_test.is_empty()
    }
}

fn intersection_size(left: &BTreeSet<String>, right: &BTreeSet<String>) -> usize {
    left.intersection(right).count()
}

fn jaccard(left: &BTreeSet<String>, right: &BTreeSet<String>) -> f64 {
    let intersection = intersection_size(left, right);
    let union = left.union(right).count();
    intersection as f64 / union as f64
}

/// Kuncheva stability for equal-cardinality feature subsets.
///
/// Returns None when the pair has unequal cardinality or selects the entire
/// universe, for which the usual chance-adjusted denominator is zero.
fn kuncheva(
    left: &BTreeSet<String>,
    right: &BTreeSet<String>,
    universe_size: usize,
) -> Option<f64> {
    let k = left.len();
    if k != right.len() || k == 0 || k >= universe_size {
        return None;
    }
    let intersection = intersection_size(left, right) as f64;
    let k = k as f64;
    let n = universe_size as f64;
    Some((intersection * n - k * k) / (k * (n - k)))
}

fn quantile_nearest_rank(sorted: &[f64], q: f64) -> f64 {
    let rank = ((q * sorted.len() as f64).ceil() as usize)
        .max(1)
        .min(sorted.len());
    sorted[rank - 1]
}

/// Measure feature-subset identity stability across independent selector runs.
///
/// All selected feature identifiers must belong to the declared candidate
/// universe. Empty or duplicate feature sets and duplicate run IDs are rejected.
/// Jaccard is reported for every pair; Kuncheva is reported only where its
/// equal-cardinality assumptions hold.
pub fn measure_feature_subset_stability(
    feature_universe: &BTreeSet<String>,
    runs: &[FeatureSubsetRun],
) -> Result<FeatureStabilityReport, String> {
    if feature_universe.is_empty() {
        return Err("feature universe must not be empty".to_owned());
    }
    if feature_universe
        .iter()
        .any(|feature| feature.trim().is_empty() || feature.trim() != feature)
    {
        return Err("feature universe contains an empty or non-canonical feature ID".to_owned());
    }
    if runs.len() < 2 {
        return Err("at least two independent selector runs are required".to_owned());
    }

    let mut run_ids = BTreeSet::new();
    let mut subsets = Vec::with_capacity(runs.len());
    let mut sizes = Vec::with_capacity(runs.len());

    for run in runs {
        if run.run_id.trim().is_empty() || run.run_id.trim() != run.run_id {
            return Err("selector run ID must be non-empty and canonical".to_owned());
        }
        if !run_ids.insert(run.run_id.clone()) {
            return Err(format!("duplicate selector run ID {:?}", run.run_id));
        }
        let subset: BTreeSet<String> = run.selected_features.iter().cloned().collect();
        if subset
            .iter()
            .any(|feature| feature.trim().is_empty() || feature.trim() != feature)
        {
            return Err(format!(
                "selector run {:?} contains an empty or non-canonical feature ID",
                run.run_id
            ));
        }
        if subset.is_empty() {
            return Err(format!(
                "selector run {:?} returned an empty feature set",
                run.run_id
            ));
        }
        if subset.len() != run.selected_features.len() {
            return Err(format!(
                "selector run {:?} contains duplicate features",
                run.run_id
            ));
        }
        if let Some(unknown) = subset.difference(feature_universe).next() {
            return Err(format!(
                "selector run {:?} selected unknown feature {:?}",
                run.run_id, unknown
            ));
        }
        sizes.push(subset.len() as f64);
        subsets.push(subset);
    }

    let mut jaccards = Vec::new();
    let mut kuncheva_values = Vec::new();
    for left in 0..subsets.len() {
        for right in (left + 1)..subsets.len() {
            jaccards.push(jaccard(&subsets[left], &subsets[right]));
            if let Some(value) = kuncheva(&subsets[left], &subsets[right], feature_universe.len()) {
                kuncheva_values.push(value);
            }
        }
    }

    jaccards.sort_by(f64::total_cmp);
    kuncheva_values.sort_by(f64::total_cmp);

    let mean = |values: &[f64]| values.iter().sum::<f64>() / values.len() as f64;
    let mean_size = mean(&sizes);
    let variance = sizes
        .iter()
        .map(|size| (size - mean_size).powi(2))
        .sum::<f64>()
        / sizes.len() as f64;

    let pair_count = jaccards.len();
    Ok(FeatureStabilityReport {
        run_count: runs.len(),
        pair_count,
        universe_size: feature_universe.len(),
        mean_selected_feature_count: mean_size,
        selected_feature_count_stddev: variance.sqrt(),
        mean_jaccard: mean(&jaccards),
        median_jaccard: if pair_count % 2 == 0 {
            (jaccards[pair_count / 2 - 1] + jaccards[pair_count / 2]) / 2.0
        } else {
            jaccards[pair_count / 2]
        },
        p10_jaccard: quantile_nearest_rank(&jaccards, 0.10),
        minimum_jaccard: jaccards[0],
        kuncheva_pair_count: kuncheva_values.len(),
        mean_kuncheva: (!kuncheva_values.is_empty()).then(|| mean(&kuncheva_values)),
    })
}

/// Reject leakage of complete simulation/production runs between partitions.
///
/// Samples/windows from one source run must never appear in more than one
/// partition. This check is intentionally performed on run IDs before any
/// sample-level windowing.
pub fn inspect_run_partition_leakage(
    partitions: &RunPartitionManifest,
) -> Result<PartitionLeakage, String> {
    if partitions.training_run_ids.is_empty() {
        return Err("training run partition must not be empty".to_owned());
    }
    if partitions.test_run_ids.is_empty() {
        return Err("test run partition must not be empty".to_owned());
    }
    for partition in [
        &partitions.training_run_ids,
        &partitions.validation_run_ids,
        &partitions.test_run_ids,
    ] {
        if partition
            .iter()
            .any(|run_id| run_id.trim().is_empty() || run_id.trim() != run_id)
        {
            return Err("partition contains an empty or non-canonical run ID".to_owned());
        }
    }

    let leakage = PartitionLeakage {
        train_validation: partitions
            .training_run_ids
            .intersection(&partitions.validation_run_ids)
            .cloned()
            .collect(),
        train_test: partitions
            .training_run_ids
            .intersection(&partitions.test_run_ids)
            .cloned()
            .collect(),
        validation_test: partitions
            .validation_run_ids
            .intersection(&partitions.test_run_ids)
            .cloned()
            .collect(),
    };
    Ok(leakage)
}


#[derive(Debug, Clone, PartialEq)]
pub struct BinaryCalibrationObservation {
    pub sample_id: String,
    pub actual_positive: bool,
    pub predicted_probability: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationBin {
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub sample_count: usize,
    pub mean_predicted_probability: Option<f64>,
    pub observed_positive_rate: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BinaryCalibrationReport {
    pub observation_count: usize,
    pub brier_score: f64,
    pub negative_log_likelihood: f64,
    pub expected_calibration_error: f64,
    pub maximum_calibration_error: f64,
    pub bins: Vec<CalibrationBin>,
}

/// Compute Brier score, fixed-width expected calibration error and reliability bins.
///
/// Labels must be independently adjudicated. Probabilities must already correspond
/// to the exact event being scored; this function does not select thresholds or
/// interpret an uncalibrated score as a probability.
pub fn evaluate_binary_calibration(
    observations: &[BinaryCalibrationObservation],
    bin_count: usize,
) -> Result<BinaryCalibrationReport, String> {
    if observations.is_empty() {
        return Err("calibration observations must not be empty".to_owned());
    }
    if !(2..=100).contains(&bin_count) {
        return Err("calibration bin count must be between 2 and 100".to_owned());
    }

    let mut sample_ids = BTreeSet::new();
    let mut bin_count_values = vec![0usize; bin_count];
    let mut bin_probability_sums = vec![0.0f64; bin_count];
    let mut bin_positive_counts = vec![0usize; bin_count];
    let mut brier_sum = 0.0f64;
    let mut negative_log_likelihood_sum = 0.0f64;
    const NLL_EPSILON: f64 = 1e-15;

    for observation in observations {
        if observation.sample_id.trim().is_empty()
            || observation.sample_id.trim() != observation.sample_id
        {
            return Err("calibration sample ID must be non-empty and canonical".to_owned());
        }
        if !sample_ids.insert(observation.sample_id.clone()) {
            return Err(format!(
                "duplicate calibration sample ID {:?}",
                observation.sample_id
            ));
        }
        let probability = observation.predicted_probability;
        if !probability.is_finite() || !(0.0..=1.0).contains(&probability) {
            return Err(format!(
                "sample {:?} probability must be finite and in [0, 1]",
                observation.sample_id
            ));
        }

        let target = if observation.actual_positive { 1.0 } else { 0.0 };
        brier_sum += (probability - target).powi(2);
        let clamped_probability = probability.clamp(NLL_EPSILON, 1.0 - NLL_EPSILON);
        negative_log_likelihood_sum -= if observation.actual_positive {
            clamped_probability.ln()
        } else {
            (1.0 - clamped_probability).ln()
        };

        let index = if probability == 1.0 {
            bin_count - 1
        } else {
            (probability * bin_count as f64).floor() as usize
        };
        bin_count_values[index] += 1;
        bin_probability_sums[index] += probability;
        bin_positive_counts[index] += usize::from(observation.actual_positive);
    }

    let observation_count = observations.len();
    let mut ece = 0.0f64;
    let mut mce = 0.0f64;
    let mut bins = Vec::with_capacity(bin_count);
    for index in 0..bin_count {
        let count = bin_count_values[index];
        let lower = index as f64 / bin_count as f64;
        let upper = (index + 1) as f64 / bin_count as f64;
        if count == 0 {
            bins.push(CalibrationBin {
                lower_bound: lower,
                upper_bound: upper,
                sample_count: 0,
                mean_predicted_probability: None,
                observed_positive_rate: None,
            });
            continue;
        }

        let mean_probability = bin_probability_sums[index] / count as f64;
        let observed_rate = bin_positive_counts[index] as f64 / count as f64;
        let error = (mean_probability - observed_rate).abs();
        ece += count as f64 / observation_count as f64 * error;
        mce = mce.max(error);
        bins.push(CalibrationBin {
            lower_bound: lower,
            upper_bound: upper,
            sample_count: count,
            mean_predicted_probability: Some(mean_probability),
            observed_positive_rate: Some(observed_rate),
        });
    }

    Ok(BinaryCalibrationReport {
        observation_count,
        brier_score: brier_sum / observation_count as f64,
        negative_log_likelihood: negative_log_likelihood_sum / observation_count as f64,
        expected_calibration_error: ece,
        maximum_calibration_error: mce,
        bins,
    })
}

pub const TEP_SCENARIO_IDS: [u8; 21] = [
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosisDisposition {
    Correct,
    ConfidentWrong,
    SafeDeferral,
    NotAttempted,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TepOnsetRecord {
    pub run_id: String,
    /// Some(1..=21) for fault runs; None for normal-operation exposure.
    pub scenario_id: Option<u8>,
    /// First detection delay from the ground-truth fault onset. None means no detection.
    pub detection_delay_ns: Option<u64>,
    /// Correctness/deferral label comes from independent ground truth, not this evaluator.
    pub diagnosis: DiagnosisDisposition,
    /// False-alarm events and exposure are recorded on normal-operation runs only.
    pub normal_false_alarm_count: u32,
    pub normal_exposure_ns: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TepScenarioOnsetMetrics {
    pub scenario_id: u8,
    pub fault_run_count: usize,
    pub detected_run_count: usize,
    pub detection_recall: f64,
    pub confident_wrong_count: usize,
    pub safe_deferral_count: usize,
    pub diagnosis_coverage: Option<f64>,
    pub selective_diagnosis_accuracy: Option<f64>,
    pub mean_detection_delay_ns: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TepOnsetMetrics {
    pub evaluation_horizon_ns: u64,
    pub fault_run_count: usize,
    pub detected_fault_run_count: usize,
    pub detection_recall: f64,
    pub confident_wrong_count: usize,
    pub confident_wrong_rate_detected: Option<f64>,
    pub safe_deferral_count: usize,
    pub safe_deferral_rate_detected: Option<f64>,
    pub diagnosis_coverage: Option<f64>,
    pub selective_diagnosis_accuracy: Option<f64>,
    pub mean_detection_delay_ns: Option<f64>,
    pub normal_false_alarm_count: u64,
    pub normal_exposure_hours: f64,
    pub false_alarm_rate_per_normal_hour: f64,
    pub scenarios: Vec<TepScenarioOnsetMetrics>,
}

/// Evaluate one pre-registered post-onset horizon over all 21 TEP fault scenarios.
///
/// The caller supplies detection and diagnosis outcomes already adjudicated against
/// independent labels. This function checks denominators, outcome consistency,
/// run identity and horizon bounds; it does not inspect model output or labels.
pub fn evaluate_tep_onset_horizon(
    records: &[TepOnsetRecord],
    evaluation_horizon_ns: u64,
) -> Result<TepOnsetMetrics, String> {
    if evaluation_horizon_ns == 0 {
        return Err("evaluation horizon must be greater than zero".to_owned());
    }
    if records.is_empty() {
        return Err("TEP onset evaluation must contain records".to_owned());
    }

    let mut run_ids = BTreeSet::new();
    let mut scenario_records: std::collections::BTreeMap<u8, Vec<&TepOnsetRecord>> =
        std::collections::BTreeMap::new();
    let mut normal_false_alarm_count = 0u64;
    let mut normal_exposure_ns = 0u64;
    let mut normal_run_count = 0usize;

    for record in records {
        if record.run_id.trim().is_empty() {
            return Err("TEP record run ID must not be empty".to_owned());
        }
        if !run_ids.insert(record.run_id.clone()) {
            return Err(format!("duplicate TEP run ID {:?}", record.run_id));
        }
        match record.scenario_id {
            Some(scenario_id) => {
                if !(1..=21).contains(&scenario_id) {
                    return Err(format!("invalid TEP fault scenario ID {scenario_id}"));
                }
                if record.normal_false_alarm_count != 0 || record.normal_exposure_ns != 0 {
                    return Err(format!(
                        "fault run {:?} must not contribute normal-operation exposure",
                        record.run_id
                    ));
                }
                match (record.detection_delay_ns, record.diagnosis) {
                    (None, DiagnosisDisposition::NotAttempted) => {}
                    (Some(delay), disposition)
                        if delay <= evaluation_horizon_ns
                            && matches!(
                                disposition,
                                DiagnosisDisposition::Correct
                                    | DiagnosisDisposition::ConfidentWrong
                                    | DiagnosisDisposition::SafeDeferral
                            ) => {}
                    (Some(delay), _) if delay > evaluation_horizon_ns => {
                        return Err(format!(
                            "detection delay for run {:?} exceeds the evaluation horizon",
                            record.run_id
                        ));
                    }
                    _ => {
                        return Err(format!(
                            "fault run {:?} has an inconsistent detection/diagnosis disposition",
                            record.run_id
                        ));
                    }
                }
                scenario_records
                    .entry(scenario_id)
                    .or_default()
                    .push(record);
            }
            None => {
                if record.detection_delay_ns.is_some()
                    || record.diagnosis != DiagnosisDisposition::NotApplicable
                {
                    return Err(format!(
                        "normal run {:?} must not carry fault detection or diagnosis",
                        record.run_id
                    ));
                }
                if record.normal_exposure_ns == 0 {
                    return Err(format!(
                        "normal run {:?} must have positive exposure",
                        record.run_id
                    ));
                }
                normal_run_count += 1;
                normal_false_alarm_count += u64::from(record.normal_false_alarm_count);
                normal_exposure_ns = normal_exposure_ns
                    .checked_add(record.normal_exposure_ns)
                    .ok_or_else(|| "normal exposure nanosecond sum overflowed".to_owned())?;
            }
        }
    }

    if normal_run_count == 0 || normal_exposure_ns == 0 {
        return Err(
            "at least one normal-operation run with positive exposure is required".to_owned(),
        );
    }
    let missing: Vec<u8> = TEP_SCENARIO_IDS
        .iter()
        .copied()
        .filter(|scenario| !scenario_records.contains_key(scenario))
        .collect();
    if !missing.is_empty() {
        return Err(format!("TEP evaluation is missing scenarios {missing:?}"));
    }

    let mut fault_run_count = 0usize;
    let mut detected_fault_run_count = 0usize;
    let mut confident_wrong_count = 0usize;
    let mut safe_deferral_count = 0usize;
    let mut correct_diagnosis_count = 0usize;
    let mut attempted_diagnosis_count = 0usize;
    let mut detection_delay_sum = 0u128;
    let mut detection_delay_count = 0usize;
    let mut scenarios = Vec::with_capacity(TEP_SCENARIO_IDS.len());

    for scenario_id in TEP_SCENARIO_IDS {
        let runs = scenario_records
            .get(&scenario_id)
            .expect("complete scenario set checked above");
        let mut detected = 0usize;
        let mut correct = 0usize;
        let mut wrong = 0usize;
        let mut deferred = 0usize;
        let mut delay_sum = 0u128;
        let mut delay_count = 0usize;

        for record in runs {
            if let Some(delay) = record.detection_delay_ns {
                detected += 1;
                delay_sum += u128::from(delay);
                delay_count += 1;
                detection_delay_sum += u128::from(delay);
                detection_delay_count += 1;
                match record.diagnosis {
                    DiagnosisDisposition::Correct => correct += 1,
                    DiagnosisDisposition::ConfidentWrong => wrong += 1,
                    DiagnosisDisposition::SafeDeferral => deferred += 1,
                    DiagnosisDisposition::NotAttempted | DiagnosisDisposition::NotApplicable => {
                        unreachable!("detection disposition validated above")
                    }
                }
            }
        }

        let scenario_runs = runs.len();
        let attempted = correct + wrong;
        fault_run_count += scenario_runs;
        detected_fault_run_count += detected;
        correct_diagnosis_count += correct;
        confident_wrong_count += wrong;
        safe_deferral_count += deferred;
        attempted_diagnosis_count += attempted;

        scenarios.push(TepScenarioOnsetMetrics {
            scenario_id,
            fault_run_count: scenario_runs,
            detected_run_count: detected,
            detection_recall: detected as f64 / scenario_runs as f64,
            confident_wrong_count: wrong,
            safe_deferral_count: deferred,
            diagnosis_coverage: (detected > 0).then(|| attempted as f64 / detected as f64),
            selective_diagnosis_accuracy: (attempted > 0)
                .then(|| correct as f64 / attempted as f64),
            mean_detection_delay_ns: (delay_count > 0)
                .then(|| delay_sum as f64 / delay_count as f64),
        });
    }

    let normal_exposure_hours = normal_exposure_ns as f64 / 3_600_000_000_000.0;
    Ok(TepOnsetMetrics {
        evaluation_horizon_ns,
        fault_run_count,
        detected_fault_run_count,
        detection_recall: detected_fault_run_count as f64 / fault_run_count as f64,
        confident_wrong_count,
        confident_wrong_rate_detected: (detected_fault_run_count > 0)
            .then(|| confident_wrong_count as f64 / detected_fault_run_count as f64),
        safe_deferral_count,
        safe_deferral_rate_detected: (detected_fault_run_count > 0)
            .then(|| safe_deferral_count as f64 / detected_fault_run_count as f64),
        diagnosis_coverage: (detected_fault_run_count > 0)
            .then(|| attempted_diagnosis_count as f64 / detected_fault_run_count as f64),
        selective_diagnosis_accuracy: (attempted_diagnosis_count > 0)
            .then(|| correct_diagnosis_count as f64 / attempted_diagnosis_count as f64),
        mean_detection_delay_ns: (detection_delay_count > 0)
            .then(|| detection_delay_sum as f64 / detection_delay_count as f64),
        normal_false_alarm_count,
        normal_exposure_hours,
        false_alarm_rate_per_normal_hour: normal_false_alarm_count as f64 / normal_exposure_hours,
        scenarios,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn run(id: &str, features: &[&str]) -> FeatureSubsetRun {
        FeatureSubsetRun {
            run_id: id.to_owned(),
            selected_features: features.iter().map(|value| (*value).to_owned()).collect(),
        }
    }



    #[test]
    fn perfectly_calibrated_binary_predictions_have_zero_brier_and_ece() {
        let observations = vec![
            BinaryCalibrationObservation {
                sample_id: "s1".to_owned(),
                actual_positive: false,
                predicted_probability: 0.0,
            },
            BinaryCalibrationObservation {
                sample_id: "s2".to_owned(),
                actual_positive: false,
                predicted_probability: 0.1,
            },
            BinaryCalibrationObservation {
                sample_id: "s3".to_owned(),
                actual_positive: true,
                predicted_probability: 0.9,
            },
            BinaryCalibrationObservation {
                sample_id: "s4".to_owned(),
                actual_positive: true,
                predicted_probability: 1.0,
            },
        ];
        let report = evaluate_binary_calibration(&observations, 10).unwrap();
        assert_eq!(report.observation_count, 4);
        assert!(report.brier_score <= 0.01);
        assert!(report.negative_log_likelihood < 0.06);
        assert!(report.expected_calibration_error <= 0.05);
        assert_eq!(report.bins.len(), 10);
        assert_eq!(report.bins[4].sample_count, 0);
        assert_eq!(report.bins[4].mean_predicted_probability, None);
    }

    #[test]
    fn overconfident_wrong_predictions_increase_brier_and_calibration_error() {
        let observations = vec![
            BinaryCalibrationObservation {
                sample_id: "wrong-positive".to_owned(),
                actual_positive: false,
                predicted_probability: 0.99,
            },
            BinaryCalibrationObservation {
                sample_id: "wrong-negative".to_owned(),
                actual_positive: true,
                predicted_probability: 0.01,
            },
        ];
        let report = evaluate_binary_calibration(&observations, 10).unwrap();
        assert!(report.brier_score > 0.95);
        assert!(report.negative_log_likelihood > 4.5);
        assert!(report.expected_calibration_error > 0.95);
        assert!(report.maximum_calibration_error > 0.95);
    }

    #[test]
    fn calibration_rejects_invalid_probabilities_and_duplicate_sample_ids() {
        let duplicate = vec![
            BinaryCalibrationObservation {
                sample_id: "same".to_owned(),
                actual_positive: false,
                predicted_probability: 0.2,
            },
            BinaryCalibrationObservation {
                sample_id: "same".to_owned(),
                actual_positive: true,
                predicted_probability: 0.8,
            },
        ];
        assert!(
            evaluate_binary_calibration(&duplicate, 10)
                .unwrap_err()
                .contains("duplicate calibration sample ID")
        );

        let invalid = vec![
            BinaryCalibrationObservation {
                sample_id: "out-of-range".to_owned(),
                actual_positive: false,
                predicted_probability: 1.1,
            },
        ];
        assert!(
            evaluate_binary_calibration(&invalid, 10)
                .unwrap_err()
                .contains("finite and in [0, 1]")
        );

        let not_finite = vec![
            BinaryCalibrationObservation {
                sample_id: "nan".to_owned(),
                actual_positive: false,
                predicted_probability: f64::NAN,
            },
        ];
        assert!(evaluate_binary_calibration(&not_finite, 10).is_err());
    }

    #[test]
    fn calibration_rejects_empty_noncanonical_samples_and_invalid_bin_count() {
        assert!(evaluate_binary_calibration(&[], 10).is_err());
        let padded = vec![
            BinaryCalibrationObservation {
                sample_id: " sample-1 ".to_owned(),
                actual_positive: true,
                predicted_probability: 0.5,
            },
        ];
        assert!(
            evaluate_binary_calibration(&padded, 10)
                .unwrap_err()
                .contains("canonical")
        );
        let valid = vec![
            BinaryCalibrationObservation {
                sample_id: "sample-1".to_owned(),
                actual_positive: true,
                predicted_probability: 0.5,
            },
        ];
        assert!(evaluate_binary_calibration(&valid, 1).is_err());
        assert!(evaluate_binary_calibration(&valid, 101).is_err());
    }

    fn tep_records() -> Vec<TepOnsetRecord> {
        let mut records = Vec::new();
        for scenario_id in TEP_SCENARIO_IDS {
            records.push(TepOnsetRecord {
                run_id: format!("fault-{scenario_id:02}"),
                scenario_id: Some(scenario_id),
                detection_delay_ns: Some(30_000_000_000),
                diagnosis: match scenario_id {
                    3 | 9 => DiagnosisDisposition::SafeDeferral,
                    15 => DiagnosisDisposition::ConfidentWrong,
                    _ => DiagnosisDisposition::Correct,
                },
                normal_false_alarm_count: 0,
                normal_exposure_ns: 0,
            });
        }
        records.push(TepOnsetRecord {
            run_id: "normal-1".to_owned(),
            scenario_id: None,
            detection_delay_ns: None,
            diagnosis: DiagnosisDisposition::NotApplicable,
            normal_false_alarm_count: 2,
            normal_exposure_ns: 7_200_000_000_000,
        });
        records
    }

    #[test]
    fn onset_metrics_separate_detection_wrong_labels_safe_deferral_and_false_alarms() {
        let metrics = evaluate_tep_onset_horizon(&tep_records(), 60_000_000_000).unwrap();
        assert_eq!(metrics.fault_run_count, 21);
        assert_eq!(metrics.detected_fault_run_count, 21);
        assert_eq!(metrics.detection_recall, 1.0);
        assert_eq!(metrics.confident_wrong_count, 1);
        assert_eq!(metrics.safe_deferral_count, 2);
        assert_eq!(metrics.confident_wrong_rate_detected, Some(1.0 / 21.0));
        assert_eq!(metrics.safe_deferral_rate_detected, Some(2.0 / 21.0));
        assert_eq!(metrics.diagnosis_coverage, Some(19.0 / 21.0));
        assert_eq!(metrics.selective_diagnosis_accuracy, Some(18.0 / 19.0));
        assert_eq!(metrics.mean_detection_delay_ns, Some(30_000_000_000.0));
        assert_eq!(metrics.false_alarm_rate_per_normal_hour, 1.0);
        assert_eq!(metrics.scenarios.len(), 21);
    }

    #[test]
    fn onset_evaluator_rejects_missing_scenarios_and_duplicate_run_ids() {
        let mut records = tep_records();
        records.retain(|record| record.scenario_id != Some(15));
        assert!(
            evaluate_tep_onset_horizon(&records, 60_000_000_000)
                .unwrap_err()
                .contains("missing scenarios")
        );

        let mut duplicate = tep_records();
        duplicate.push(duplicate[0].clone());
        assert!(
            evaluate_tep_onset_horizon(&duplicate, 60_000_000_000)
                .unwrap_err()
                .contains("duplicate TEP run ID")
        );
    }

    #[test]
    fn onset_evaluator_rejects_horizon_and_detection_disposition_mismatches() {
        let mut late = tep_records();
        late[0].detection_delay_ns = Some(120_000_000_000);
        assert!(
            evaluate_tep_onset_horizon(&late, 60_000_000_000)
                .unwrap_err()
                .contains("exceeds the evaluation horizon")
        );

        let mut inconsistent = tep_records();
        inconsistent[0].detection_delay_ns = None;
        inconsistent[0].diagnosis = DiagnosisDisposition::Correct;
        assert!(
            evaluate_tep_onset_horizon(&inconsistent, 60_000_000_000)
                .unwrap_err()
                .contains("inconsistent detection/diagnosis")
        );
    }

    #[test]
    fn onset_evaluator_rejects_fault_labels_on_normal_run_and_requires_normal_exposure() {
        let mut normal_fault = tep_records();
        normal_fault.last_mut().unwrap().diagnosis = DiagnosisDisposition::Correct;
        assert!(
            evaluate_tep_onset_horizon(&normal_fault, 60_000_000_000)
                .unwrap_err()
                .contains("normal run")
        );

        let mut no_exposure = tep_records();
        no_exposure.last_mut().unwrap().normal_exposure_ns = 0;
        assert!(
            evaluate_tep_onset_horizon(&no_exposure, 60_000_000_000)
                .unwrap_err()
                .contains("positive exposure")
        );
    }

    #[test]
    fn exact_equal_subsets_have_unit_jaccard_and_kuncheva() {
        let universe = set(&["x1", "x2", "x3", "x4", "x5"]);
        let report = measure_feature_subset_stability(
            &universe,
            &[run("seed-1", &["x1", "x2"]), run("seed-2", &["x1", "x2"])],
        )
        .unwrap();
        assert_eq!(report.pair_count, 1);
        assert_eq!(report.mean_jaccard, 1.0);
        assert_eq!(report.mean_kuncheva, Some(1.0));
        assert_eq!(report.selected_feature_count_stddev, 0.0);
    }

    #[test]
    fn equal_subset_size_can_hide_identity_instability() {
        let universe = set(&["x1", "x2", "x3", "x4", "x5"]);
        let report = measure_feature_subset_stability(
            &universe,
            &[
                run("seed-1", &["x1", "x2"]),
                run("seed-2", &["x3", "x4"]),
                run("seed-3", &["x1", "x4"]),
            ],
        )
        .unwrap();
        assert_eq!(report.selected_feature_count_stddev, 0.0);
        assert!(report.mean_jaccard < 0.5);
        assert!(report.mean_kuncheva.unwrap() < 0.0);
    }

    #[test]
    fn varying_subset_sizes_report_jaccard_and_skip_inapplicable_kuncheva_pairs() {
        let universe = set(&["x1", "x2", "x3", "x4", "x5", "x6"]);
        let report = measure_feature_subset_stability(
            &universe,
            &[
                run("seed-1", &["x1", "x2"]),
                run("seed-2", &["x1", "x2", "x3"]),
                run("seed-3", &["x1", "x2"]),
            ],
        )
        .unwrap();
        assert_eq!(report.pair_count, 3);
        assert_eq!(report.kuncheva_pair_count, 1);
        assert!(report.selected_feature_count_stddev > 0.0);
    }

    #[test]
    fn rejects_duplicate_run_ids_duplicate_features_and_unknown_features() {
        let universe = set(&["x1", "x2", "x3"]);
        assert!(
            measure_feature_subset_stability(
                &universe,
                &[run("same", &["x1"]), run("same", &["x2"])]
            )
            .unwrap_err()
            .contains("duplicate selector run ID")
        );
        assert!(
            measure_feature_subset_stability(
                &universe,
                &[run("a", &["x1", "x1"]), run("b", &["x2"])]
            )
            .unwrap_err()
            .contains("duplicate features")
        );
        assert!(
            measure_feature_subset_stability(&universe, &[run("a", &["x4"]), run("b", &["x2"])])
                .unwrap_err()
                .contains("unknown feature")
        );
    }

    #[test]
    fn rejects_noncanonical_feature_identifiers() {
        let padded_universe = set(&["x1", " x2"]);
        assert!(
            measure_feature_subset_stability(
                &padded_universe,
                &[run("seed-1", &["x1"]), run("seed-2", &["x1"])]
            )
            .unwrap_err()
            .contains("non-canonical feature ID")
        );

        let universe = set(&["x1", "x2"]);
        assert!(
            measure_feature_subset_stability(
                &universe,
                &[run("seed-1", &["x1"]), run("seed-2", &[" x2"])]
            )
            .unwrap_err()
            .contains("non-canonical feature ID")
        );
    }

    #[test]
    fn rejects_empty_feature_universe_or_single_selector_run() {
        assert!(
            measure_feature_subset_stability(
                &BTreeSet::new(),
                &[run("a", &["x1"]), run("b", &["x1"])]
            )
            .is_err()
        );
        assert!(measure_feature_subset_stability(&set(&["x1"]), &[run("a", &["x1"])]).is_err());
    }

    #[test]
    fn detects_each_kind_of_run_level_partition_leakage() {
        let partitions = RunPartitionManifest {
            training_run_ids: set(&["run-1", "run-2"]),
            validation_run_ids: set(&["run-2", "run-3"]),
            test_run_ids: set(&["run-3", "run-4", "run-1"]),
        };
        let leakage = inspect_run_partition_leakage(&partitions).unwrap();
        assert_eq!(leakage.train_validation, vec!["run-2"]);
        assert_eq!(leakage.train_test, vec!["run-1"]);
        assert_eq!(leakage.validation_test, vec!["run-3"]);
    }

    #[test]
    fn accepts_disjoint_run_partitions() {
        let partitions = RunPartitionManifest {
            training_run_ids: set(&["train-1", "train-2"]),
            validation_run_ids: set(&["validation-1"]),
            test_run_ids: set(&["test-1", "test-2"]),
        };
        assert!(
            inspect_run_partition_leakage(&partitions)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn rejects_whitespace_padded_run_identifiers() {
        let partitions = RunPartitionManifest {
            training_run_ids: set(&[" train-01 "]),
            validation_run_ids: set(&["validation-01"]),
            test_run_ids: set(&["test-01"]),
        };
        assert!(
            inspect_run_partition_leakage(&partitions)
                .unwrap_err()
                .contains("non-canonical run ID")
        );
    }

    #[test]
    fn rejects_missing_training_or_test_partition() {
        let no_train = RunPartitionManifest {
            training_run_ids: BTreeSet::new(),
            validation_run_ids: set(&["validation-1"]),
            test_run_ids: set(&["test-1"]),
        };
        assert!(inspect_run_partition_leakage(&no_train).is_err());
        let no_test = RunPartitionManifest {
            training_run_ids: set(&["train-1"]),
            validation_run_ids: BTreeSet::new(),
            test_run_ids: BTreeSet::new(),
        };
        assert!(inspect_run_partition_leakage(&no_test).is_err());
    }
}
