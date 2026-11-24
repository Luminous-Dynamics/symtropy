// SPDX-License-Identifier: AGPL-3.0-or-later

//! Phase-locked acquisition design diagnostics for periodic PIV.
//!
//! The evaluator separates phase diversity from independent replication.
//! Multiple observations in one phase bin count as independent repeats only
//! when they belong to distinct declared dependence clusters (for example,
//! distinct forcing cycles). Multiple frames from one cycle remain visible in
//! the raw observation count but do not inflate independent-cycle replication.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseObservation {
    /// Forcing phase as a fraction of a cycle, in [0, 1).
    pub phase_fraction: f64,
    /// Highest dependence cluster used for independent replication.
    /// For a periodic flow, this will usually be a forcing-cycle/session ID.
    pub dependence_cluster_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseDesignConfig {
    pub phase_bin_count: usize,
    pub minimum_occupied_phase_bins: usize,
    pub minimum_independent_cycles_per_occupied_bin: usize,
    /// Maximum permitted cyclic gap as a fraction of one forcing cycle.
    pub maximum_cyclic_phase_gap: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhaseBinOccupancy {
    pub phase_bin_index: usize,
    pub observations: usize,
    pub independent_cycle_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhaseDesignFailure {
    InsufficientPhaseCoverage,
    InsufficientIndependentReplication,
    MaximumCyclicPhaseGapExceeded,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseDesignReport {
    pub total_observations: usize,
    pub distinct_dependence_clusters: usize,
    pub phase_bin_count: usize,
    pub occupied_phase_bins: usize,
    pub phase_coverage_fraction: f64,
    pub empty_phase_bins: usize,
    pub singleton_phase_bins: usize,
    pub singleton_phase_bin_fraction: f64,
    pub under_replicated_phase_bins: usize,
    pub minimum_independent_cycles_per_occupied_bin: usize,
    pub median_independent_cycles_per_occupied_bin: f64,
    pub maximum_cyclic_phase_gap: f64,
    /// Shannon entropy of independent-cycle occupancy, normalized by
    /// ln(phase_bin_count). Zero means no phase-distributed information.
    pub normalized_phase_occupancy_entropy: f64,
    pub bins: Vec<PhaseBinOccupancy>,
    pub failures: Vec<PhaseDesignFailure>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PhaseDesignError {
    EmptyObservations,
    InvalidConfiguration { field: &'static str },
    EmptyDependenceCluster { observation_index: usize },
    InvalidPhase { observation_index: usize },
}

impl fmt::Display for PhaseDesignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid periodic PIV phase design: {self:?}")
    }
}

impl Error for PhaseDesignError {}

pub fn evaluate_phase_design(
    observations: &[PhaseObservation],
    config: &PhaseDesignConfig,
) -> Result<PhaseDesignReport, PhaseDesignError> {
    validate_config(config)?;
    if observations.is_empty() {
        return Err(PhaseDesignError::EmptyObservations);
    }

    let mut sample_counts_by_bin = BTreeMap::<usize, usize>::new();
    let mut cycles_by_bin = BTreeMap::<usize, BTreeSet<String>>::new();
    let mut all_cycles = BTreeSet::<String>::new();

    for (observation_index, observation) in observations.iter().enumerate() {
        if !observation.phase_fraction.is_finite()
            || !(0.0..1.0).contains(&observation.phase_fraction)
        {
            return Err(PhaseDesignError::InvalidPhase { observation_index });
        }
        if observation.dependence_cluster_id.trim().is_empty() {
            return Err(PhaseDesignError::EmptyDependenceCluster { observation_index });
        }

        let bin_index =
            (observation.phase_fraction * config.phase_bin_count as f64).floor() as usize;
        let bin_index = bin_index.min(config.phase_bin_count - 1);
        *sample_counts_by_bin.entry(bin_index).or_default() += 1;
        cycles_by_bin
            .entry(bin_index)
            .or_default()
            .insert(observation.dependence_cluster_id.clone());
        all_cycles.insert(observation.dependence_cluster_id.clone());
    }

    let bins: Vec<PhaseBinOccupancy> = sample_counts_by_bin
        .iter()
        .map(|(bin_index, observation_count)| PhaseBinOccupancy {
            phase_bin_index: *bin_index,
            observations: *observation_count,
            independent_cycle_count: cycles_by_bin.get(bin_index).map_or(0, BTreeSet::len),
        })
        .collect();

    let occupied_phase_bins = bins.len();
    let phase_coverage_fraction = occupied_phase_bins as f64 / config.phase_bin_count as f64;
    let empty_phase_bins = config.phase_bin_count - occupied_phase_bins;
    let replication_counts: Vec<usize> =
        bins.iter().map(|bin| bin.independent_cycle_count).collect();
    let singleton_phase_bins = replication_counts
        .iter()
        .filter(|count| **count == 1)
        .count();
    let under_replicated_phase_bins = replication_counts
        .iter()
        .filter(|count| **count < config.minimum_independent_cycles_per_occupied_bin)
        .count();
    let minimum_independent_cycles_per_occupied_bin =
        replication_counts.iter().copied().min().unwrap_or(0);
    let median_independent_cycles_per_occupied_bin = median_usize(&replication_counts);
    let maximum_cyclic_phase_gap = maximum_cyclic_gap(&bins, config.phase_bin_count);
    let normalized_phase_occupancy_entropy =
        normalized_entropy(&replication_counts, config.phase_bin_count);

    let mut failures = Vec::new();
    if occupied_phase_bins < config.minimum_occupied_phase_bins {
        failures.push(PhaseDesignFailure::InsufficientPhaseCoverage);
    }
    if under_replicated_phase_bins > 0 {
        failures.push(PhaseDesignFailure::InsufficientIndependentReplication);
    }
    if maximum_cyclic_phase_gap > config.maximum_cyclic_phase_gap {
        failures.push(PhaseDesignFailure::MaximumCyclicPhaseGapExceeded);
    }

    Ok(PhaseDesignReport {
        total_observations: observations.len(),
        distinct_dependence_clusters: all_cycles.len(),
        phase_bin_count: config.phase_bin_count,
        occupied_phase_bins,
        phase_coverage_fraction,
        empty_phase_bins,
        singleton_phase_bins,
        singleton_phase_bin_fraction: singleton_phase_bins as f64 / occupied_phase_bins as f64,
        under_replicated_phase_bins,
        minimum_independent_cycles_per_occupied_bin,
        median_independent_cycles_per_occupied_bin,
        maximum_cyclic_phase_gap,
        normalized_phase_occupancy_entropy,
        bins,
        failures,
    })
}

fn validate_config(config: &PhaseDesignConfig) -> Result<(), PhaseDesignError> {
    if config.phase_bin_count == 0 || config.phase_bin_count > 1_000_000 {
        return Err(PhaseDesignError::InvalidConfiguration {
            field: "phase_bin_count",
        });
    }
    if config.minimum_occupied_phase_bins == 0
        || config.minimum_occupied_phase_bins > config.phase_bin_count
    {
        return Err(PhaseDesignError::InvalidConfiguration {
            field: "minimum_occupied_phase_bins",
        });
    }
    if config.minimum_independent_cycles_per_occupied_bin == 0 {
        return Err(PhaseDesignError::InvalidConfiguration {
            field: "minimum_independent_cycles_per_occupied_bin",
        });
    }
    if !config.maximum_cyclic_phase_gap.is_finite()
        || !(0.0..=1.0).contains(&config.maximum_cyclic_phase_gap)
        || config.maximum_cyclic_phase_gap == 0.0
    {
        return Err(PhaseDesignError::InvalidConfiguration {
            field: "maximum_cyclic_phase_gap",
        });
    }
    Ok(())
}

fn median_usize(values: &[usize]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let middle = sorted.len() / 2;
    if sorted.len().is_multiple_of(2) {
        (sorted[middle - 1] as f64 + sorted[middle] as f64) * 0.5
    } else {
        sorted[middle] as f64
    }
}

fn maximum_cyclic_gap(bins: &[PhaseBinOccupancy], phase_bin_count: usize) -> f64 {
    if bins.len() == 1 {
        return 1.0;
    }
    let mut maximum_gap = 0usize;
    for (index, current_bin) in bins.iter().enumerate() {
        let current = current_bin.phase_bin_index;
        let next = if index + 1 == bins.len() {
            bins[0].phase_bin_index + phase_bin_count
        } else {
            bins[index + 1].phase_bin_index
        };
        maximum_gap = maximum_gap.max(next - current);
    }
    maximum_gap as f64 / phase_bin_count as f64
}

fn normalized_entropy(counts: &[usize], phase_bin_count: usize) -> f64 {
    if phase_bin_count <= 1 {
        return 0.0;
    }
    let total: usize = counts.iter().sum();
    if total == 0 {
        return 0.0;
    }
    let entropy = counts
        .iter()
        .filter(|count| **count > 0)
        .map(|count| {
            let probability = *count as f64 / total as f64;
            -probability * probability.ln()
        })
        .sum::<f64>();
    entropy / (phase_bin_count as f64).ln()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> PhaseDesignConfig {
        PhaseDesignConfig {
            phase_bin_count: 160,
            minimum_occupied_phase_bins: 81,
            minimum_independent_cycles_per_occupied_bin: 2,
            maximum_cyclic_phase_gap: 0.02,
        }
    }

    fn observation(phase_fraction: f64, cycle: &str) -> PhaseObservation {
        PhaseObservation {
            phase_fraction,
            dependence_cluster_id: cycle.into(),
        }
    }

    #[test]
    fn one_sample_per_phase_fails_replication_even_with_phase_diversity() {
        let observations: Vec<PhaseObservation> = (0..81)
            .map(|index| {
                let bin = index * 160 / 81;
                observation((bin as f64 + 0.5) / 160.0, &format!("cycle-{index}"))
            })
            .collect();
        let mut settings = config();
        settings.maximum_cyclic_phase_gap = 0.02;
        let report = evaluate_phase_design(&observations, &settings).unwrap();
        assert_eq!(report.occupied_phase_bins, 81);
        assert_eq!(report.singleton_phase_bins, 81);
        assert_eq!(report.under_replicated_phase_bins, 81);
        assert!(
            report
                .failures
                .contains(&PhaseDesignFailure::InsufficientIndependentReplication)
        );
    }

    #[test]
    fn repeated_observations_at_one_phase_fail_phase_coverage() {
        let observations: Vec<PhaseObservation> = (0..20)
            .map(|index| observation(0.01, &format!("cycle-{index}")))
            .collect();
        let mut settings = config();
        settings.maximum_cyclic_phase_gap = 1.0;
        let report = evaluate_phase_design(&observations, &settings).unwrap();
        assert_eq!(report.occupied_phase_bins, 1);
        assert_eq!(report.minimum_independent_cycles_per_occupied_bin, 20);
        assert!(
            report
                .failures
                .contains(&PhaseDesignFailure::InsufficientPhaseCoverage)
        );
    }

    #[test]
    fn two_independent_cycles_in_eighty_one_of_160_bins_qualifies() {
        let mut observations = Vec::with_capacity(162);
        for phase_index in 0..81 {
            let bin = phase_index * 160 / 81;
            let phase = (bin as f64 + 0.5) / 160.0;
            observations.push(observation(phase, &format!("cycle-a-{phase_index}")));
            observations.push(observation(phase, &format!("cycle-b-{phase_index}")));
        }
        let report = evaluate_phase_design(&observations, &config()).unwrap();
        assert_eq!(report.total_observations, 162);
        assert_eq!(report.occupied_phase_bins, 81);
        assert_eq!(report.phase_coverage_fraction, 81.0 / 160.0);
        assert_eq!(report.minimum_independent_cycles_per_occupied_bin, 2);
        assert_eq!(report.under_replicated_phase_bins, 0);
        assert!(report.failures.is_empty());
    }

    #[test]
    fn repeated_frames_in_one_cycle_do_not_inflate_independent_replication() {
        let observations = vec![
            observation(0.1, "cycle-a"),
            observation(0.101, "cycle-a"),
            observation(0.102, "cycle-a"),
            observation(0.1, "cycle-b"),
        ];
        let settings = PhaseDesignConfig {
            phase_bin_count: 10,
            minimum_occupied_phase_bins: 1,
            minimum_independent_cycles_per_occupied_bin: 3,
            maximum_cyclic_phase_gap: 1.0,
        };
        let report = evaluate_phase_design(&observations, &settings).unwrap();
        assert_eq!(report.bins.len(), 1);
        assert_eq!(report.bins[0].phase_bin_index, 1);
        assert_eq!(report.bins[0].observations, 4);
        assert_eq!(report.bins[0].independent_cycle_count, 2);
        assert_eq!(report.minimum_independent_cycles_per_occupied_bin, 2);
        assert_eq!(report.under_replicated_phase_bins, 1);
        assert!(
            report
                .failures
                .contains(&PhaseDesignFailure::InsufficientIndependentReplication)
        );
    }

    #[test]
    fn wraparound_phase_gap_is_included() {
        let observations = vec![
            observation(0.01, "cycle-a"),
            observation(0.49, "cycle-b"),
            observation(0.99, "cycle-c"),
        ];
        let settings = PhaseDesignConfig {
            phase_bin_count: 100,
            minimum_occupied_phase_bins: 3,
            minimum_independent_cycles_per_occupied_bin: 1,
            maximum_cyclic_phase_gap: 0.5,
        };
        let report = evaluate_phase_design(&observations, &settings).unwrap();
        assert_eq!(report.maximum_cyclic_phase_gap, 0.5);
    }

    #[test]
    fn report_is_invariant_to_observation_order() {
        let mut observations = Vec::new();
        for phase_index in 0..81 {
            let phase = (phase_index as f64 + 0.5) / 160.0;
            observations.push(observation(phase, &format!("a-{phase_index}")));
            observations.push(observation(phase, &format!("b-{phase_index}")));
        }
        let forward = evaluate_phase_design(&observations, &config()).unwrap();
        observations.reverse();
        let reverse = evaluate_phase_design(&observations, &config()).unwrap();
        assert_eq!(forward, reverse);
    }

    #[test]
    fn phase_one_and_non_finite_phases_are_rejected() {
        let settings = config();
        assert_eq!(
            evaluate_phase_design(&[observation(1.0, "cycle-a")], &settings),
            Err(PhaseDesignError::InvalidPhase {
                observation_index: 0
            })
        );
        assert_eq!(
            evaluate_phase_design(&[observation(f64::NAN, "cycle-a")], &settings),
            Err(PhaseDesignError::InvalidPhase {
                observation_index: 0
            })
        );
    }

    #[test]
    fn malformed_cluster_identity_and_config_are_rejected() {
        let mut settings = config();
        settings.phase_bin_count = 0;
        assert_eq!(
            evaluate_phase_design(&[observation(0.1, "cycle-a")], &settings),
            Err(PhaseDesignError::InvalidConfiguration {
                field: "phase_bin_count"
            })
        );
        assert_eq!(
            evaluate_phase_design(
                &[observation(0.1, "  ")],
                &PhaseDesignConfig {
                    phase_bin_count: 10,
                    minimum_occupied_phase_bins: 1,
                    minimum_independent_cycles_per_occupied_bin: 1,
                    maximum_cyclic_phase_gap: 1.0
                }
            ),
            Err(PhaseDesignError::EmptyDependenceCluster {
                observation_index: 0
            })
        );
    }
}
