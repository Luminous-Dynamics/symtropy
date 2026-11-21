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
        if run.run_id.trim().is_empty() {
            return Err("selector run ID must not be empty".to_owned());
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
        if partition.iter().any(|run_id| run_id.trim().is_empty()) {
            return Err("partition contains an empty run ID".to_owned());
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
