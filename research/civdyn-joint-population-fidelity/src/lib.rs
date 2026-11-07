// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairObservation {
    pub observation_id: String,
    pub independent_unit_id: String,
    /// Declared stratum label; never inferred from free text.
    pub stratum: String,
    pub left_category: String,
    pub right_category: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PairFidelityReport {
    pub observed_rows: usize,
    pub simulated_rows: usize,
    pub observed_independent_units: usize,
    pub simulated_independent_units: usize,
    pub left_marginal_tv: f64,
    pub right_marginal_tv: f64,
    pub joint_tv: f64,
    pub observed_mutual_information_nats: f64,
    pub simulated_mutual_information_nats: f64,
    pub mutual_information_abs_gap_nats: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FidelityError {
    EmptyDataset,
    EmptyIdentity {
        field: &'static str,
    },
    DuplicateObservationId {
        id: String,
    },
    MissingStratum {
        stratum: String,
        population: &'static str,
    },
    InvalidBootstrapReplicates {
        requested: usize,
    },
    InvalidConfidenceLevel,
    InsufficientIndependentUnits {
        population: &'static str,
        count: usize,
    },
}

impl fmt::Display for FidelityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDataset => write!(f, "dataset must contain at least one observation"),
            Self::EmptyIdentity { field } => write!(f, "{field} must not be empty"),
            Self::DuplicateObservationId { id } => write!(f, "duplicate observation id: {id}"),
            Self::MissingStratum {
                stratum,
                population,
            } => {
                write!(
                    f,
                    "stratum {stratum} is missing from {population} population"
                )
            }
            Self::InvalidBootstrapReplicates { requested } => {
                write!(
                    f,
                    "cluster bootstrap requires at least 100 replicates, got {requested}"
                )
            }
            Self::InvalidConfidenceLevel => {
                write!(
                    f,
                    "confidence level must be finite and strictly between 0 and 1"
                )
            }
            Self::InsufficientIndependentUnits { population, count } => {
                write!(
                    f,
                    "{population} population has only {count} independent units; at least 2 are required"
                )
            }
        }
    }
}
impl std::error::Error for FidelityError {}

#[derive(Default)]
struct Counts {
    left: BTreeMap<String, u64>,
    right: BTreeMap<String, u64>,
    joint: BTreeMap<(String, String), u64>,
    independent_units: BTreeSet<String>,
    rows: usize,
}

fn validate(rows: &[PairObservation]) -> Result<(), FidelityError> {
    if rows.is_empty() {
        return Err(FidelityError::EmptyDataset);
    }
    let mut ids = BTreeSet::new();
    for row in rows {
        for (field, value) in [
            ("observation_id", row.observation_id.as_str()),
            ("independent_unit_id", row.independent_unit_id.as_str()),
            ("stratum", row.stratum.as_str()),
            ("left_category", row.left_category.as_str()),
            ("right_category", row.right_category.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(FidelityError::EmptyIdentity { field });
            }
        }
        if !ids.insert(row.observation_id.as_str()) {
            return Err(FidelityError::DuplicateObservationId {
                id: row.observation_id.clone(),
            });
        }
    }
    Ok(())
}

fn count_rows(rows: &[PairObservation]) -> Counts {
    let mut c = Counts {
        rows: rows.len(),
        ..Counts::default()
    };
    for row in rows {
        *c.left.entry(row.left_category.clone()).or_default() += 1;
        *c.right.entry(row.right_category.clone()).or_default() += 1;
        *c.joint
            .entry((row.left_category.clone(), row.right_category.clone()))
            .or_default() += 1;
        c.independent_units.insert(row.independent_unit_id.clone());
    }
    c
}

fn total<K: Ord>(counts: &BTreeMap<K, u64>) -> u64 {
    counts.values().sum()
}

fn tv<K: Ord + Clone>(left: &BTreeMap<K, u64>, right: &BTreeMap<K, u64>) -> f64 {
    let nl = total(left) as f64;
    let nr = total(right) as f64;
    let keys: BTreeSet<K> = left.keys().chain(right.keys()).cloned().collect();
    0.5 * keys
        .iter()
        .map(|key| {
            let p = *left.get(key).unwrap_or(&0) as f64 / nl;
            let q = *right.get(key).unwrap_or(&0) as f64 / nr;
            (p - q).abs()
        })
        .sum::<f64>()
}

fn mi(c: &Counts) -> f64 {
    let n = c.rows as f64;
    c.joint
        .iter()
        .map(|((x, y), count)| {
            let pxy = *count as f64 / n;
            let px = c.left[x] as f64 / n;
            let py = c.right[y] as f64 / n;
            pxy * (pxy / (px * py)).ln()
        })
        .sum()
}

fn make_report(o: &Counts, s: &Counts) -> PairFidelityReport {
    let oi = mi(o);
    let si = mi(s);
    PairFidelityReport {
        observed_rows: o.rows,
        simulated_rows: s.rows,
        observed_independent_units: o.independent_units.len(),
        simulated_independent_units: s.independent_units.len(),
        left_marginal_tv: tv(&o.left, &s.left),
        right_marginal_tv: tv(&o.right, &s.right),
        joint_tv: tv(&o.joint, &s.joint),
        observed_mutual_information_nats: oi,
        simulated_mutual_information_nats: si,
        mutual_information_abs_gap_nats: (oi - si).abs(),
    }
}

/// Return marginal TV, joint TV, mutual information and support counts separately.
/// Descriptive only: no p-values, confidence intervals or causal claims.
pub fn compare_pair(
    observed: &[PairObservation],
    simulated: &[PairObservation],
) -> Result<PairFidelityReport, FidelityError> {
    validate(observed)?;
    validate(simulated)?;
    Ok(make_report(&count_rows(observed), &count_rows(simulated)))
}

/// Compute a report for every declared stratum; missing strata fail closed.
pub fn compare_by_stratum(
    observed: &[PairObservation],
    simulated: &[PairObservation],
) -> Result<BTreeMap<String, PairFidelityReport>, FidelityError> {
    validate(observed)?;
    validate(simulated)?;
    let os: BTreeSet<_> = observed.iter().map(|r| r.stratum.as_str()).collect();
    let ss: BTreeSet<_> = simulated.iter().map(|r| r.stratum.as_str()).collect();
    let strata: BTreeSet<_> = os.union(&ss).copied().collect();
    let mut result = BTreeMap::new();

    for stratum in strata {
        let o: Vec<_> = observed
            .iter()
            .filter(|r| r.stratum.as_str() == stratum)
            .cloned()
            .collect();
        let s: Vec<_> = simulated
            .iter()
            .filter(|r| r.stratum.as_str() == stratum)
            .cloned()
            .collect();
        if o.is_empty() {
            return Err(FidelityError::MissingStratum {
                stratum: stratum.to_string(),
                population: "observed",
            });
        }
        if s.is_empty() {
            return Err(FidelityError::MissingStratum {
                stratum: stratum.to_string(),
                population: "simulated",
            });
        }
        result.insert(
            stratum.to_string(),
            make_report(&count_rows(&o), &count_rows(&s)),
        );
    }
    Ok(result)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BootstrapInterval {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PairMetricIntervals {
    pub left_marginal_tv: BootstrapInterval,
    pub right_marginal_tv: BootstrapInterval,
    pub joint_tv: BootstrapInterval,
    pub mutual_information_abs_gap_nats: BootstrapInterval,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PairBootstrapReport {
    pub point_estimate: PairFidelityReport,
    pub confidence_level: f64,
    pub replicates: usize,
    pub observed_cluster_count: usize,
    pub simulated_cluster_count: usize,
    pub intervals: PairMetricIntervals,
}

#[derive(Clone, Copy)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    fn index(&mut self, upper_exclusive: usize) -> usize {
        (self.next_u64() % upper_exclusive as u64) as usize
    }
}

fn independent_unit_count(rows: &[PairObservation]) -> usize {
    rows.iter()
        .map(|row| row.independent_unit_id.as_str())
        .collect::<BTreeSet<_>>()
        .len()
}

fn resample_clusters(
    rows: &[PairObservation],
    rng: &mut SplitMix64,
    replicate_index: usize,
) -> Vec<PairObservation> {
    let mut clusters: BTreeMap<&str, Vec<&PairObservation>> = BTreeMap::new();
    for row in rows {
        clusters
            .entry(row.independent_unit_id.as_str())
            .or_default()
            .push(row);
    }
    let cluster_rows: Vec<_> = clusters.values().collect();
    let mut sampled = Vec::new();

    for draw_index in 0..cluster_rows.len() {
        let selected = cluster_rows[rng.index(cluster_rows.len())];
        for (row_index, row) in selected.iter().enumerate() {
            let mut copy = (*row).clone();
            copy.observation_id = format!("bootstrap-{replicate_index}-{draw_index}-{row_index}");
            copy.independent_unit_id = format!("bootstrap-{replicate_index}-cluster-{draw_index}");
            sampled.push(copy);
        }
    }
    sampled
}

fn percentile(values: &mut [f64], probability: f64) -> f64 {
    values.sort_by(f64::total_cmp);
    let position = probability * (values.len() - 1) as f64;
    let lower_index = position.floor() as usize;
    let upper_index = position.ceil() as usize;
    let fraction = position - lower_index as f64;
    values[lower_index] * (1.0 - fraction) + values[upper_index] * fraction
}

fn interval(values: &mut [f64], confidence_level: f64) -> BootstrapInterval {
    let tail = (1.0 - confidence_level) / 2.0;
    BootstrapInterval {
        lower: percentile(values, tail),
        upper: percentile(values, 1.0 - tail),
    }
}

/// Cluster bootstrap for bivariate population diagnostics.
///
/// Whole independent units are sampled with replacement within each population.
/// Repeated observations sharing one independent_unit_id always travel together.
/// Intervals are conditional on the observed clusters; they do not represent model,
/// study-selection, measurement or specification uncertainty.
pub fn compare_pair_cluster_bootstrap(
    observed: &[PairObservation],
    simulated: &[PairObservation],
    replicates: usize,
    confidence_level: f64,
    seed: u64,
) -> Result<PairBootstrapReport, FidelityError> {
    validate(observed)?;
    validate(simulated)?;

    if replicates < 100 {
        return Err(FidelityError::InvalidBootstrapReplicates {
            requested: replicates,
        });
    }
    if !confidence_level.is_finite() || !(0.0..1.0).contains(&confidence_level) {
        return Err(FidelityError::InvalidConfidenceLevel);
    }

    let observed_clusters = independent_unit_count(observed);
    let simulated_clusters = independent_unit_count(simulated);
    if observed_clusters < 2 {
        return Err(FidelityError::InsufficientIndependentUnits {
            population: "observed",
            count: observed_clusters,
        });
    }
    if simulated_clusters < 2 {
        return Err(FidelityError::InsufficientIndependentUnits {
            population: "simulated",
            count: simulated_clusters,
        });
    }

    let point_estimate = compare_pair(observed, simulated)?;
    let mut observed_rng = SplitMix64::new(seed ^ 0xA0761D6478BD642F);
    let mut simulated_rng = SplitMix64::new(seed ^ 0xE7037ED1A0B428DB);

    let mut left_tv = Vec::with_capacity(replicates);
    let mut right_tv = Vec::with_capacity(replicates);
    let mut joint_tv = Vec::with_capacity(replicates);
    let mut mi_gap = Vec::with_capacity(replicates);

    for replicate_index in 0..replicates {
        let observed_sample = resample_clusters(observed, &mut observed_rng, replicate_index);
        let simulated_sample = resample_clusters(simulated, &mut simulated_rng, replicate_index);
        let sample = make_report(
            &count_rows(&observed_sample),
            &count_rows(&simulated_sample),
        );
        left_tv.push(sample.left_marginal_tv);
        right_tv.push(sample.right_marginal_tv);
        joint_tv.push(sample.joint_tv);
        mi_gap.push(sample.mutual_information_abs_gap_nats);
    }

    Ok(PairBootstrapReport {
        point_estimate,
        confidence_level,
        replicates,
        observed_cluster_count: observed_clusters,
        simulated_cluster_count: simulated_clusters,
        intervals: PairMetricIntervals {
            left_marginal_tv: interval(&mut left_tv, confidence_level),
            right_marginal_tv: interval(&mut right_tv, confidence_level),
            joint_tv: interval(&mut joint_tv, confidence_level),
            mutual_information_abs_gap_nats: interval(&mut mi_gap, confidence_level),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(prefix: &str, pairs: &[(&str, &str)], stratum: &str) -> Vec<PairObservation> {
        pairs
            .iter()
            .enumerate()
            .map(|(i, (left, right))| PairObservation {
                observation_id: format!("{prefix}-obs-{i}"),
                independent_unit_id: format!("{prefix}-unit-{i}"),
                stratum: stratum.to_string(),
                left_category: (*left).to_string(),
                right_category: (*right).to_string(),
            })
            .collect()
    }
    fn correlated() -> Vec<(&'static str, &'static str)> {
        let mut v = vec![("low", "low"); 50];
        v.extend(vec![("high", "high"); 50]);
        v
    }
    fn independent_pairs() -> Vec<(&'static str, &'static str)> {
        let mut v = vec![("low", "low"); 25];
        v.extend([("low", "high"); 25]);
        v.extend([("high", "low"); 25]);
        v.extend([("high", "high"); 25]);
        v
    }

    #[test]
    fn detects_joint_mismatch_when_both_marginals_match() {
        let o = rows("o", &correlated(), "all");
        let s = rows("s", &independent_pairs(), "all");
        let r = compare_pair(&o, &s).expect("fixture valid");
        assert_eq!(r.left_marginal_tv, 0.0);
        assert_eq!(r.right_marginal_tv, 0.0);
        assert!((r.joint_tv - 0.5).abs() < 1e-12);
        assert!((r.observed_mutual_information_nats - std::f64::consts::LN_2).abs() < 1e-12);
        assert!(r.simulated_mutual_information_nats.abs() < 1e-12);
    }

    #[test]
    fn identical_joint_distributions_have_zero_discrepancy() {
        let o = rows("o", &correlated(), "all");
        let s = rows("s", &correlated(), "all");
        let r = compare_pair(&o, &s).expect("fixture valid");
        assert_eq!(r.left_marginal_tv, 0.0);
        assert_eq!(r.right_marginal_tv, 0.0);
        assert_eq!(r.joint_tv, 0.0);
        assert_eq!(r.mutual_information_abs_gap_nats, 0.0);
    }

    #[test]
    fn stratum_reports_do_not_hide_local_association_failure() {
        let mut o = rows("oa", &correlated(), "a");
        o.extend(rows("ob", &correlated(), "b"));
        let mut s = rows("sa", &independent_pairs(), "a");
        s.extend(rows("sb", &correlated(), "b"));
        let r = compare_by_stratum(&o, &s).expect("strata supported");
        assert!((r["a"].joint_tv - 0.5).abs() < 1e-12);
        assert_eq!(r["b"].joint_tv, 0.0);
    }

    #[test]
    fn absent_stratum_fails_closed() {
        let o = rows("o", &correlated(), "all");
        let s = rows("s", &correlated(), "other");
        assert!(matches!(
            compare_by_stratum(&o, &s),
            Err(FidelityError::MissingStratum { .. })
        ));
    }

    #[test]
    fn duplicate_observation_id_is_rejected() {
        let mut o = rows("o", &correlated(), "all");
        o[1].observation_id = o[0].observation_id.clone();
        let s = rows("s", &correlated(), "all");
        assert!(matches!(
            compare_pair(&o, &s),
            Err(FidelityError::DuplicateObservationId { .. })
        ));
    }

    #[test]
    fn missing_category_fails_instead_of_becoming_zero() {
        let mut o = rows("o", &correlated(), "all");
        o[0].right_category.clear();
        let s = rows("s", &correlated(), "all");
        assert!(matches!(
            compare_pair(&o, &s),
            Err(FidelityError::EmptyIdentity {
                field: "right_category"
            })
        ));
    }
    fn clustered_rows(
        prefix: &str,
        clusters: &[Vec<(&str, &str)>],
        stratum: &str,
    ) -> Vec<PairObservation> {
        let mut rows = Vec::new();
        for (cluster_index, cluster) in clusters.iter().enumerate() {
            for (row_index, (left, right)) in cluster.iter().enumerate() {
                rows.push(PairObservation {
                    observation_id: format!("{prefix}-c{cluster_index}-r{row_index}"),
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
    fn bootstrap_resamples_whole_clusters_and_is_seed_reproducible() {
        let observed = clustered_rows(
            "observed",
            &[
                vec![("low", "low"), ("low", "low")],
                vec![("low", "low"), ("high", "high")],
                vec![("high", "high"), ("high", "high")],
                vec![("high", "high"), ("low", "low")],
            ],
            "all",
        );
        let simulated = clustered_rows(
            "simulated",
            &[
                vec![("low", "low"), ("low", "high")],
                vec![("low", "high"), ("high", "high")],
                vec![("high", "low"), ("high", "high")],
                vec![("high", "low"), ("low", "low")],
            ],
            "all",
        );

        let a = compare_pair_cluster_bootstrap(&observed, &simulated, 200, 0.95, 42)
            .expect("bootstrap succeeds");
        let b = compare_pair_cluster_bootstrap(&observed, &simulated, 200, 0.95, 42)
            .expect("same seed repeats");
        assert_eq!(a, b);
        assert_eq!(a.observed_cluster_count, 4);
        assert_eq!(a.simulated_cluster_count, 4);
        assert!(a.intervals.joint_tv.lower <= a.intervals.joint_tv.upper);
        assert!(a.intervals.joint_tv.lower >= 0.0);
        assert!(a.intervals.joint_tv.upper <= 1.0);
    }

    #[test]
    fn bootstrap_replicate_keeps_every_row_in_a_selected_cluster() {
        let data = clustered_rows(
            "observed",
            &[
                vec![("low", "low"), ("low", "high"), ("high", "low")],
                vec![("high", "high"), ("low", "low"), ("high", "low")],
            ],
            "all",
        );
        let mut rng = SplitMix64::new(12);
        let sample = resample_clusters(&data, &mut rng, 0);
        assert_eq!(sample.len(), 6);
        let mut sizes = BTreeMap::new();
        for row in sample {
            *sizes.entry(row.independent_unit_id).or_insert(0usize) += 1;
        }
        assert_eq!(sizes.len(), 2);
        assert!(sizes.values().all(|size| *size == 3));
    }

    #[test]
    fn bootstrap_rejects_too_few_replicates() {
        let data = rows("data", &correlated(), "all");
        let error = compare_pair_cluster_bootstrap(&data, &data, 99, 0.95, 1)
            .expect_err("replicate count must fail closed");
        assert!(matches!(
            error,
            FidelityError::InvalidBootstrapReplicates { requested: 99 }
        ));
    }

    #[test]
    fn bootstrap_rejects_single_independent_unit() {
        let make_single_cluster = |prefix: &str| {
            let mut data = rows(prefix, &correlated(), "all");
            for row in &mut data {
                row.independent_unit_id = format!("{prefix}-one-cluster");
            }
            data
        };
        let observed = make_single_cluster("observed");
        let simulated = make_single_cluster("simulated");
        let error = compare_pair_cluster_bootstrap(&observed, &simulated, 100, 0.95, 7)
            .expect_err("one cluster cannot estimate between-cluster uncertainty");
        assert!(matches!(
            error,
            FidelityError::InsufficientIndependentUnits { .. }
        ));
    }

    #[test]
    fn bootstrap_rejects_invalid_confidence_level() {
        let observed = rows("observed", &correlated(), "all");
        let simulated = rows("simulated", &correlated(), "all");
        assert!(matches!(
            compare_pair_cluster_bootstrap(&observed, &simulated, 100, 1.0, 9),
            Err(FidelityError::InvalidConfidenceLevel)
        ));
    }
}
