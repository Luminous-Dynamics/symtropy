// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

//! Independent-unit cluster bootstrap for bivariate population fidelity.
//!
//! Whole independent units are sampled with replacement. Repeated observations
//! within a person/household/site are therefore retained as one cluster instead
//! of being treated as independent rows. Observed and simulated populations
//! are resampled separately.

use std::collections::BTreeMap;

use crate::{FidelityError, PairBootstrapReport, PairFidelityReport, PairObservation, compare_pair};

pub const MIN_BOOTSTRAP_REPLICATES: usize = 200;
pub const BOOTSTRAP_CONFIDENCE_LEVEL: f64 = 0.95;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootstrapConfig {
    pub replicates: usize,
    pub seed: u64,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            replicates: 2_000,
            seed: 0x4349_5644_594e_4253,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BootstrapInterval {
    pub point_estimate: f64,
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PairBootstrapReport {
    pub bootstrap_replicates: usize,
    pub confidence_level: f64,
    pub observed_rows: usize,
    pub simulated_rows: usize,
    pub observed_independent_units: usize,
    pub simulated_independent_units: usize,
    pub left_marginal_tv: BootstrapInterval,
    pub right_marginal_tv: BootstrapInterval,
    pub joint_tv: BootstrapInterval,
    pub observed_mutual_information_nats: BootstrapInterval,
    pub simulated_mutual_information_nats: BootstrapInterval,
    pub mutual_information_abs_gap_nats: BootstrapInterval,
}

#[derive(Debug, Clone)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    /// Uniform index by rejection sampling; avoids modulo bias.
    fn index(&mut self, upper: usize) -> usize {
        assert!(upper > 0, "cannot sample an empty cluster set");
        let bound = upper as u64;
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let value = self.next_u64();
            if value >= threshold {
                return (value % bound) as usize;
            }
        }
    }
}

fn group_clusters(rows: &[PairObservation]) -> Vec<Vec<PairObservation>> {
    let mut grouped: BTreeMap<String, Vec<PairObservation>> = BTreeMap::new();
    for row in rows {
        grouped
            .entry(row.independent_unit_id.clone())
            .or_default()
            .push(row.clone());
    }
    grouped.into_values().collect()
}

fn resample_clusters(
    clusters: &[Vec<PairObservation>],
    rng: &mut SplitMix64,
    replicate: usize,
    population_prefix: &str,
) -> Vec<PairObservation> {
    let mut sampled = Vec::new();
    for draw in 0..clusters.len() {
        let cluster = &clusters[rng.index(clusters.len())];
        for (row_index, original) in cluster.iter().enumerate() {
            let mut row = original.clone();
            // A selected cluster may be drawn more than once. Each bootstrap
            // draw is a distinct resampled unit and needs unique row IDs.
            row.independent_unit_id = format!("{population_prefix}-boot-{replicate}-unit-{draw}");
            row.observation_id =
                format!("{population_prefix}-boot-{replicate}-unit-{draw}-row-{row_index}");
            sampled.push(row);
        }
    }
    sampled
}

fn percentile(sorted: &[f64], probability: f64) -> f64 {
    debug_assert!(!sorted.is_empty());
    let position = probability * (sorted.len() - 1) as f64;
    let lower_index = position.floor() as usize;
    let upper_index = position.ceil() as usize;
    if lower_index == upper_index {
        return sorted[lower_index];
    }
    let weight = position - lower_index as f64;
    sorted[lower_index] * (1.0 - weight) + sorted[upper_index] * weight
}

fn interval(point_estimate: f64, mut samples: Vec<f64>) -> BootstrapInterval {
    samples.sort_by(f64::total_cmp);
    BootstrapInterval {
        point_estimate,
        lower: percentile(&samples, 0.025),
        upper: percentile(&samples, 0.975),
    }
}

/// Compare two bivariate populations and quantify uncertainty by resampling
/// independent units with replacement, keeping every row within a sampled
/// unit together. The two populations are resampled independently.
///
/// The returned percentile intervals describe sampling variability under the
/// observed cluster sampling design. They do not correct selection bias,
/// guarantee representativeness, or establish causal or external validity.
pub fn bootstrap_compare_pair(
    observed: &[PairObservation],
    simulated: &[PairObservation],
    config: BootstrapConfig,
) -> Result<PairBootstrapReport, FidelityError> {
    if config.replicates < MIN_BOOTSTRAP_REPLICATES {
        return Err(FidelityError::InsufficientBootstrapReplicates {
            minimum: MIN_BOOTSTRAP_REPLICATES,
            actual: config.replicates,
        });
    }

    // Validate the original datasets and obtain the point estimate before
    // resampling. This also rejects duplicate row identities and missing data.
    let point: PairFidelityReport = compare_pair(observed, simulated)?;
    let observed_clusters = group_clusters(observed);
    let simulated_clusters = group_clusters(simulated);

    if observed_clusters.len() < 2 {
        return Err(FidelityError::InsufficientIndependentUnits {
            population: "observed",
            units: observed_clusters.len(),
        });
    }
    if simulated_clusters.len() < 2 {
        return Err(FidelityError::InsufficientIndependentUnits {
            population: "simulated",
            units: simulated_clusters.len(),
        });
    }

    let mut rng = SplitMix64::new(config.seed);
    let mut left_marginal_tv = Vec::with_capacity(config.replicates);
    let mut right_marginal_tv = Vec::with_capacity(config.replicates);
    let mut joint_tv = Vec::with_capacity(config.replicates);
    let mut observed_mi = Vec::with_capacity(config.replicates);
    let mut simulated_mi = Vec::with_capacity(config.replicates);
    let mut mi_gap = Vec::with_capacity(config.replicates);

    for replicate in 0..config.replicates {
        let observed_sample = resample_clusters(
            &observed_clusters,
            &mut rng,
            replicate,
            "observed",
        );
        let simulated_sample = resample_clusters(
            &simulated_clusters,
            &mut rng,
            replicate,
            "simulated",
        );
        let report = compare_pair(&observed_sample, &simulated_sample)?;

        left_marginal_tv.push(report.left_marginal_tv);
        right_marginal_tv.push(report.right_marginal_tv);
        joint_tv.push(report.joint_tv);
        observed_mi.push(report.observed_mutual_information_nats);
        simulated_mi.push(report.simulated_mutual_information_nats);
        mi_gap.push(report.mutual_information_abs_gap_nats);
    }

    Ok(PairBootstrapReport {
        bootstrap_replicates: config.replicates,
        confidence_level: BOOTSTRAP_CONFIDENCE_LEVEL,
        observed_rows: point.observed_rows,
        simulated_rows: point.simulated_rows,
        observed_independent_units: observed_clusters.len(),
        simulated_independent_units: simulated_clusters.len(),
        left_marginal_tv: interval(point.left_marginal_tv, left_marginal_tv),
        right_marginal_tv: interval(point.right_marginal_tv, right_marginal_tv),
        joint_tv: interval(point.joint_tv, joint_tv),
        observed_mutual_information_nats: interval(
            point.observed_mutual_information_nats,
            observed_mi,
        ),
        simulated_mutual_information_nats: interval(
            point.simulated_mutual_information_nats,
            simulated_mi,
        ),
        mutual_information_abs_gap_nats: interval(
            point.mutual_information_abs_gap_nats,
            mi_gap,
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clustered_rows(
        prefix: &str,
        cluster_pairs: &[(&str, &str, usize)],
        stratum: &str,
    ) -> Vec<PairObservation> {
        let mut rows = Vec::new();
        for (cluster_index, (left, right, repeats)) in cluster_pairs.iter().enumerate() {
            for row_index in 0..*repeats {
                rows.push(PairObservation {
                    observation_id: format!("{prefix}-cluster-{cluster_index}-row-{row_index}"),
                    independent_unit_id: format!("{prefix}-cluster-{cluster_index}"),
                    stratum: stratum.to_string(),
                    left_category: (*left).to_string(),
                    right_category: (*right).to_string(),
                });
            }
        }
        rows
    }

    #[test]
    fn cluster_bootstrap_is_deterministic_and_counts_units_not_rows() {
        let observed = clustered_rows(
            "o",
            &[("low", "low", 8), ("low", "low", 8), ("high", "high", 8), ("high", "high", 8)],
            "all",
        );
        let simulated = clustered_rows(
            "s",
            &[
                ("low", "low", 4),
                ("low", "high", 4),
                ("high", "low", 4),
                ("high", "high", 4),
            ],
            "all",
        );
        let config = BootstrapConfig {
            replicates: 500,
            seed: 7,
        };

        let first = bootstrap_compare_pair(&observed, &simulated, config).unwrap();
        let second = bootstrap_compare_pair(&observed, &simulated, config).unwrap();

        assert_eq!(first, second);
        assert_eq!(first.observed_rows, 32);
        assert_eq!(first.simulated_rows, 16);
        assert_eq!(first.observed_independent_units, 4);
        assert_eq!(first.simulated_independent_units, 4);
        assert_eq!(first.confidence_level, 0.95);
        assert_eq!(first.bootstrap_replicates, 500);
        assert_eq!(first.joint_tv.point_estimate, 0.5);
        assert!(first.joint_tv.lower <= first.joint_tv.upper);
    }

    #[test]
    fn bootstrap_rejects_a_single_independent_unit() {
        let observed = clustered_rows("o", &[("low", "low", 20)], "all");
        let simulated = clustered_rows(
            "s",
            &[("low", "low", 1), ("high", "high", 1)],
            "all",
        );

        assert_eq!(
            bootstrap_compare_pair(&observed, &simulated, BootstrapConfig::default()),
            Err(FidelityError::InsufficientIndependentUnits {
                population: "observed",
                units: 1,
            })
        );
    }

    #[test]
    fn bootstrap_requires_a_preregistered_minimum_replicate_count() {
        let observed = clustered_rows(
            "o",
            &[("low", "low", 2), ("high", "high", 2)],
            "all",
        );
        let simulated = clustered_rows(
            "s",
            &[("low", "high", 2), ("high", "low", 2)],
            "all",
        );
        assert_eq!(
            bootstrap_compare_pair(
                &observed,
                &simulated,
                BootstrapConfig {
                    replicates: MIN_BOOTSTRAP_REPLICATES - 1,
                    seed: 1,
                }
            ),
            Err(FidelityError::InsufficientBootstrapReplicates {
                minimum: MIN_BOOTSTRAP_REPLICATES,
                actual: MIN_BOOTSTRAP_REPLICATES - 1,
            })
        );
    }

    #[test]
    fn duplicate_ids_remain_rejected_before_bootstrap() {
        let mut observed =
            clustered_rows("o", &[("low", "low", 1), ("high", "high", 1)], "all");
        observed[1].observation_id = observed[0].observation_id.clone();
        let simulated =
            clustered_rows("s", &[("low", "low", 1), ("high", "high", 1)], "all");

        assert!(matches!(
            bootstrap_compare_pair(&observed, &simulated, BootstrapConfig::default()),
            Err(FidelityError::DuplicateObservationId { .. })
        ));
    }
}
