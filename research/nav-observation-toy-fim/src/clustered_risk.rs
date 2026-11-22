//! Cluster bootstrap intervals for false-safe navigation-service risk.
//!
//! Resamples whole predeclared clusters (event, session, site, device, etc.)
//! rather than individual records. This only addresses dependence represented
//! by the declared single cluster ID; it is not a multi-way or hierarchical
//! bootstrap and cannot discover omitted common causes.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::risk_coverage::EvaluationCase;

const SCALE_PPM: u128 = 1_000_000;
const MIN_VALID_FRACTION_NUMERATOR: usize = 4;
const MIN_VALID_FRACTION_DENOMINATOR: usize = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClusterEvaluationCase {
    pub case: EvaluationCase,
    /// Preselected independent sampling unit, e.g. physical event or session.
    pub cluster_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PercentileIntervalPpm {
    pub lower_95_ppm: u32,
    pub upper_95_ppm: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClusterBootstrapSummary {
    pub threshold_ppm: u32,
    pub algorithm: String,
    pub seed: u64,
    pub requested_replicates: usize,
    pub valid_replicates: usize,
    pub zero_coverage_replicates: usize,
    pub cluster_count: usize,
    pub clusters_with_covered_records: usize,
    pub total_records: usize,
    pub covered_records: usize,
    pub false_safe_records: usize,
    pub total_exposure_seconds: u128,
    pub covered_exposure_seconds: u128,
    pub false_safe_exposure_seconds: u128,
    pub false_safe_rate_ppm: Option<u32>,
    pub false_safe_exposure_rate_ppm: Option<u32>,
    pub record_rate_interval: Option<PercentileIntervalPpm>,
    pub exposure_rate_interval: Option<PercentileIntervalPpm>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClusterBootstrapError {
    EmptyCaseSet,
    InsufficientIndependentClusters(usize),
    InvalidThreshold,
    InvalidReplicateCount,
    EmptyRecordId,
    DuplicateRecordId(String),
    EmptyClusterId,
    InvalidConfidence(String),
    ZeroExposure(String),
}

impl fmt::Display for ClusterBootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCaseSet => write!(f, "at least one clustered evaluation case is required"),
            Self::InsufficientIndependentClusters(count) => write!(
                f,
                "at least two independent clusters are required; received {count}"
            ),
            Self::InvalidThreshold => write!(f, "confidence threshold must be within 0..=1,000,001 ppm"),
            Self::InvalidReplicateCount => write!(f, "bootstrap replicate count must be at least 100"),
            Self::EmptyRecordId => write!(f, "record ID must not be empty"),
            Self::DuplicateRecordId(id) => write!(f, "duplicate record ID: {id}"),
            Self::EmptyClusterId => write!(f, "cluster ID must not be empty"),
            Self::InvalidConfidence(id) => {
                write!(f, "confidence must be within 0..=1,000,000 ppm: {id}")
            }
            Self::ZeroExposure(id) => write!(f, "exposure duration must be positive: {id}"),
        }
    }
}
impl std::error::Error for ClusterBootstrapError {}

/// Small deterministic SplitMix64 generator.
/// Algorithm ID and seed are returned in the summary for exact replay.
#[derive(Clone, Copy, Debug)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D049BB133111EB);
        value ^ (value >> 31)
    }

    fn index(&mut self, upper_exclusive: usize) -> usize {
        // Rejection sampling avoids modulo bias for the finite cluster count.
        let bound = upper_exclusive as u64;
        let zone = u64::MAX - (u64::MAX % bound);
        loop {
            let value = self.next_u64();
            if value < zone {
                return (value % bound) as usize;
            }
        }
    }
}

fn ratio_ppm(numerator: u128, denominator: u128) -> Option<u32> {
    (denominator > 0).then(|| {
        ((numerator.saturating_mul(SCALE_PPM) / denominator).min(SCALE_PPM)) as u32
    })
}

fn percentile_nearest_rank(sorted: &[u32], numerator: usize, denominator: usize) -> Option<u32> {
    if sorted.is_empty() || denominator == 0 || numerator == 0 || numerator > denominator {
        return None;
    }
    let rank_one_based = (numerator * sorted.len()).div_ceil(denominator).max(1);
    sorted.get(rank_one_based - 1).copied()
}

fn percentile_interval(values: &mut Vec<u32>, enough_valid: bool) -> Option<PercentileIntervalPpm> {
    if !enough_valid || values.is_empty() {
        return None;
    }
    values.sort_unstable();
    Some(PercentileIntervalPpm {
        lower_95_ppm: percentile_nearest_rank(values, 1, 40)?,
        upper_95_ppm: percentile_nearest_rank(values, 39, 40)?,
    })
}

fn validate_and_group<'a>(
    cases: &'a [ClusterEvaluationCase],
) -> Result<BTreeMap<&'a str, Vec<&'a EvaluationCase>>, ClusterBootstrapError> {
    if cases.is_empty() {
        return Err(ClusterBootstrapError::EmptyCaseSet);
    }

    let mut ids = BTreeSet::new();
    let mut groups = BTreeMap::<&str, Vec<&EvaluationCase>>::new();
    for item in cases {
        let case = &item.case;
        if case.record_id.trim().is_empty() {
            return Err(ClusterBootstrapError::EmptyRecordId);
        }
        if !ids.insert(case.record_id.as_str()) {
            return Err(ClusterBootstrapError::DuplicateRecordId(
                case.record_id.clone(),
            ));
        }
        if item.cluster_id.trim().is_empty() {
            return Err(ClusterBootstrapError::EmptyClusterId);
        }
        if case.confidence_ppm > SCALE_PPM as u32 {
            return Err(ClusterBootstrapError::InvalidConfidence(
                case.record_id.clone(),
            ));
        }
        if case.exposure_seconds == 0 {
            return Err(ClusterBootstrapError::ZeroExposure(
                case.record_id.clone(),
            ));
        }
        groups.entry(item.cluster_id.as_str()).or_default().push(&item.case);
    }
    if groups.len() < 2 {
        return Err(ClusterBootstrapError::InsufficientIndependentClusters(
            groups.len(),
        ));
    }

    // Stable order inside clusters makes output independent of input ordering.
    for records in groups.values_mut() {
        records.sort_by(|a, b| a.record_id.cmp(&b.record_id));
    }
    Ok(groups)
}

#[derive(Clone, Copy, Debug, Default)]
struct Counts {
    total_records: u128,
    covered_records: u128,
    false_safe_records: u128,
    total_exposure_seconds: u128,
    covered_exposure_seconds: u128,
    false_safe_exposure_seconds: u128,
}

impl Counts {
    fn add_record(&mut self, case: &EvaluationCase, threshold_ppm: u32) {
        self.total_records += 1;
        self.total_exposure_seconds += case.exposure_seconds as u128;
        if case.confidence_ppm >= threshold_ppm {
            self.covered_records += 1;
            self.covered_exposure_seconds += case.exposure_seconds as u128;
            if case.claim_state_one && !case.truth_state_one {
                self.false_safe_records += 1;
                self.false_safe_exposure_seconds += case.exposure_seconds as u128;
            }
        }
    }

    fn add_cluster(&mut self, records: &[&EvaluationCase], threshold_ppm: u32) {
        for case in records {
            self.add_record(case, threshold_ppm);
        }
    }
}

/// Bootstrap false-safe risk by resampling whole clusters with replacement.
///
/// Each replicate draws exactly N clusters, where N is the observed number of
/// distinct clusters. Every record in a sampled cluster is retained; selecting
/// the same cluster twice duplicates its full record block.
///
/// A two-sided percentile 95% interval is returned only when at least 80% of
/// requested replicates have non-zero coverage. Zero-coverage replicates are
/// counted separately, never imputed as zero risk.
pub fn cluster_bootstrap_false_safe(
    cases: &[ClusterEvaluationCase],
    threshold_ppm: u32,
    replicates: usize,
    seed: u64,
) -> Result<ClusterBootstrapSummary, ClusterBootstrapError> {
    if threshold_ppm > 1_000_001 {
        return Err(ClusterBootstrapError::InvalidThreshold);
    }
    if replicates < 100 {
        return Err(ClusterBootstrapError::InvalidReplicateCount);
    }

    let groups = validate_and_group(cases)?;
    let clusters: Vec<(&str, Vec<&EvaluationCase>)> = groups
        .into_iter()
        .map(|(id, records)| (id, records))
        .collect();

    let mut observed = Counts::default();
    let mut clusters_with_covered_records = 0_usize;
    for (_, records) in &clusters {
        let before = observed.covered_records;
        observed.add_cluster(records, threshold_ppm);
        if observed.covered_records > before {
            clusters_with_covered_records += 1;
        }
    }

    let mut rng = SplitMix64::new(seed);
    let mut record_rate_samples = Vec::with_capacity(replicates);
    let mut exposure_rate_samples = Vec::with_capacity(replicates);
    let mut zero_coverage_replicates = 0_usize;

    for _ in 0..replicates {
        let mut sample = Counts::default();
        for _ in 0..clusters.len() {
            let index = rng.index(clusters.len());
            sample.add_cluster(&clusters[index].1, threshold_ppm);
        }
        if sample.covered_records == 0 || sample.covered_exposure_seconds == 0 {
            zero_coverage_replicates += 1;
            continue;
        }
        if let Some(rate) = ratio_ppm(sample.false_safe_records, sample.covered_records) {
            record_rate_samples.push(rate);
        }
        if let Some(rate) = ratio_ppm(
            sample.false_safe_exposure_seconds,
            sample.covered_exposure_seconds,
        ) {
            exposure_rate_samples.push(rate);
        }
    }

    let valid_replicates = record_rate_samples.len().min(exposure_rate_samples.len());
    let enough_valid = valid_replicates * MIN_VALID_FRACTION_DENOMINATOR
        >= replicates * MIN_VALID_FRACTION_NUMERATOR;

    Ok(ClusterBootstrapSummary {
        threshold_ppm,
        algorithm: "splitmix64_cluster_percentile_v0.1".to_owned(),
        seed,
        requested_replicates: replicates,
        valid_replicates,
        zero_coverage_replicates,
        cluster_count: clusters.len(),
        clusters_with_covered_records,
        total_records: observed.total_records as usize,
        covered_records: observed.covered_records as usize,
        false_safe_records: observed.false_safe_records as usize,
        total_exposure_seconds: observed.total_exposure_seconds,
        covered_exposure_seconds: observed.covered_exposure_seconds,
        false_safe_exposure_seconds: observed.false_safe_exposure_seconds,
        false_safe_rate_ppm: ratio_ppm(
            observed.false_safe_records,
            observed.covered_records,
        ),
        false_safe_exposure_rate_ppm: ratio_ppm(
            observed.false_safe_exposure_seconds,
            observed.covered_exposure_seconds,
        ),
        record_rate_interval: percentile_interval(&mut record_rate_samples, enough_valid),
        exposure_rate_interval: percentile_interval(&mut exposure_rate_samples, enough_valid),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, cluster: &str, confidence: u32, claim: bool, truth: bool, seconds: u64) -> ClusterEvaluationCase {
        ClusterEvaluationCase {
            case: EvaluationCase {
                record_id: id.to_owned(),
                confidence_ppm: confidence,
                claim_state_one: claim,
                truth_state_one: truth,
                exposure_seconds: seconds,
            },
            cluster_id: cluster.to_owned(),
        }
    }

    fn cases() -> Vec<ClusterEvaluationCase> {
        vec![
            item("a1", "event-a", 900_000, true, false, 60),
            item("a2", "event-a", 950_000, true, true, 60),
            item("a3", "event-a", 800_000, false, false, 60),
            item("b1", "event-b", 900_000, true, true, 600),
            item("b2", "event-b", 920_000, false, true, 600),
            item("c1", "event-c", 930_000, true, true, 120),
        ]
    }

    #[test]
    fn produces_reproducible_cluster_level_percentile_intervals() {
        let data = cases();
        let a = cluster_bootstrap_false_safe(&data, 850_000, 1000, 12345).unwrap();
        let b = cluster_bootstrap_false_safe(&data, 850_000, 1000, 12345).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.cluster_count, 3);
        assert_eq!(a.valid_replicates + a.zero_coverage_replicates, 1000);
        assert!(a.record_rate_interval.is_some());
        assert!(a.exposure_rate_interval.is_some());
        assert_eq!(a.algorithm, "splitmix64_cluster_percentile_v0.1");
    }

    #[test]
    fn single_large_cluster_is_not_mistaken_for_many_independent_windows() {
        let data = vec![
            item("a1", "one-event", 900_000, true, false, 60),
            item("a2", "one-event", 900_000, true, false, 60),
            item("a3", "one-event", 900_000, true, false, 60),
            item("a4", "one-event", 900_000, true, false, 60),
        ];
        assert_eq!(
            cluster_bootstrap_false_safe(&data, 0, 1000, 1),
            Err(ClusterBootstrapError::InsufficientIndependentClusters(1))
        );
    }

    #[test]
    fn duplicating_windows_within_a_cluster_does_not_increase_cluster_count() {
        let mut data = cases();
        let base = cluster_bootstrap_false_safe(&data, 850_000, 1000, 77).unwrap();
        data.push(item("a4", "event-a", 900_000, true, false, 60));
        let duplicate = cluster_bootstrap_false_safe(&data, 850_000, 1000, 77).unwrap();
        assert_eq!(base.cluster_count, duplicate.cluster_count);
        assert_eq!(base.clusters_with_covered_records, duplicate.clusters_with_covered_records);
    }

    #[test]
    fn all_abstention_replicates_return_no_interval_not_zero() {
        let data = cases();
        let result = cluster_bootstrap_false_safe(&data, 1_000_001, 1000, 9).unwrap();
        assert_eq!(result.covered_records, 0);
        assert_eq!(result.valid_replicates, 0);
        assert_eq!(result.zero_coverage_replicates, 1000);
        assert_eq!(result.record_rate_interval, None);
        assert_eq!(result.exposure_rate_interval, None);
        assert_eq!(result.false_safe_rate_ppm, None);
    }

    #[test]
    fn cluster_with_failures_is_visible_in_upper_tail() {
        let data = vec![
            item("a1", "event-a", 900_000, true, false, 60),
            item("a2", "event-a", 900_000, true, false, 60),
            item("b1", "event-b", 900_000, true, true, 60),
            item("b2", "event-b", 900_000, true, true, 60),
            item("c1", "event-c", 900_000, true, true, 60),
            item("c2", "event-c", 900_000, true, true, 60),
        ];
        let result = cluster_bootstrap_false_safe(&data, 850_000, 2000, 99).unwrap();
        let interval = result.record_rate_interval.unwrap();
        assert!(interval.upper_95_ppm >= 500_000);
        assert!(interval.lower_95_ppm <= 333_333);
    }

    #[test]
    fn input_order_does_not_change_bootstrap_result() {
        let mut forward = cases();
        let mut reverse = cases();
        reverse.reverse();
        assert_eq!(
            cluster_bootstrap_false_safe(&forward, 850_000, 1000, 123).unwrap(),
            cluster_bootstrap_false_safe(&reverse, 850_000, 1000, 123).unwrap()
        );
        forward.reverse();
    }

    #[test]
    fn invalid_cluster_metadata_and_thresholds_fail_closed() {
        let mut data = cases();
        data[0].cluster_id.clear();
        assert_eq!(
            cluster_bootstrap_false_safe(&data, 0, 1000, 1),
            Err(ClusterBootstrapError::EmptyClusterId)
        );

        let data = cases();
        assert_eq!(
            cluster_bootstrap_false_safe(&data, 0, 99, 1),
            Err(ClusterBootstrapError::InvalidReplicateCount)
        );
        assert_eq!(
            cluster_bootstrap_false_safe(&data, 1_000_002, 1000, 1),
            Err(ClusterBootstrapError::InvalidThreshold)
        );
    }
}
