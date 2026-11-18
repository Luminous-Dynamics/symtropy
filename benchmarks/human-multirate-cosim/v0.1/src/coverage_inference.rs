//! Ordered coverage inference and fixed-delay adaptive conformal updates.
//!
//! Wilson intervals are iid-only comparisons. HAC and moving-block bootstrap
//! intervals are empirical/asymptotic diagnostics, not finite-sample guarantees.
//! The delayed ACI scheduler is a fixed-integer-delay recurrence reference,
//! not a general theorem or a general asynchronous feedback protocol.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt;

const Z95: f64 = 1.959_963_984_540_054;
const SPLITMIX_INCREMENT: u64 = 0x9E37_79B9_7F4A_7C15;
const MAX_BOOTSTRAP_REPLICATES: usize = 100_000;
const MAX_SEQUENCE_LENGTH: usize = 10_000_000;
const MAX_HAC_WORK: usize = 100_000_000;
const MAX_BOOTSTRAP_WORK: usize = 100_000_000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoverageOutcomeV1 {
    pub forecast_id: u64,
    pub issue_step: u64,
    pub availability_step: u64,
    pub issue_time: f64,
    pub event_time: f64,
    pub availability_time: f64,
    pub covered: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoverageSeriesV1 {
    pub series_id: String,
    pub outcomes: Vec<CoverageOutcomeV1>,
}

impl CoverageSeriesV1 {
    pub fn new(
        series_id: String,
        outcomes: Vec<CoverageOutcomeV1>,
    ) -> Result<Self, CoverageInferenceError> {
        if series_id.trim().is_empty() {
            return Err(CoverageInferenceError::EmptySeriesIdentity);
        }
        if outcomes.is_empty() {
            return Err(CoverageInferenceError::EmptySequence);
        }
        if outcomes.len() > MAX_SEQUENCE_LENGTH {
            return Err(CoverageInferenceError::SequenceTooLong);
        }
        let mut ids = HashSet::with_capacity(outcomes.len());
        let mut previous_step = None;
        let mut previous_issue_time = None;
        let mut previous_event_time = None;
        for outcome in &outcomes {
            if !ids.insert(outcome.forecast_id) {
                return Err(CoverageInferenceError::DuplicateForecastId(
                    outcome.forecast_id,
                ));
            }
            if !outcome.issue_time.is_finite()
                || !outcome.event_time.is_finite()
                || !outcome.availability_time.is_finite()
            {
                return Err(CoverageInferenceError::NonFiniteTime);
            }
            if outcome.issue_time > outcome.event_time
                || outcome.event_time > outcome.availability_time
                || outcome.issue_step > outcome.availability_step
            {
                return Err(CoverageInferenceError::InvalidOutcomeTimeOrder);
            }
            if previous_step.is_some_and(|step| outcome.issue_step <= step) {
                return Err(CoverageInferenceError::NonMonotonicIssueStep);
            }
            if previous_issue_time.is_some_and(|time| outcome.issue_time <= time) {
                return Err(CoverageInferenceError::NonMonotonicIssueTime);
            }
            if previous_event_time.is_some_and(|time| outcome.event_time <= time) {
                return Err(CoverageInferenceError::NonMonotonicEventTime);
            }
            previous_step = Some(outcome.issue_step);
            previous_issue_time = Some(outcome.issue_time);
            previous_event_time = Some(outcome.event_time);
        }
        Ok(Self { series_id, outcomes })
    }

    pub fn indicators(&self) -> Vec<bool> {
        self.outcomes.iter().map(|outcome| outcome.covered).collect()
    }

    pub fn realized_coverage(&self) -> f64 {
        let count = self.outcomes.iter().filter(|outcome| outcome.covered).count();
        count as f64 / self.outcomes.len() as f64
    }

    pub fn hac_report(
        &self,
        target_coverage: f64,
        bandwidth: usize,
    ) -> Result<HacCoverageReportV1, CoverageInferenceError> {
        hac_coverage_report(
            &self.indicators(),
            target_coverage,
            bandwidth,
            self.series_id.clone(),
        )
    }

    pub fn moving_block_bootstrap(
        &self,
        target_coverage: f64,
        block_length: usize,
        replicates: usize,
        seed: u64,
    ) -> Result<BlockBootstrapCoverageReportV1, CoverageInferenceError> {
        moving_block_bootstrap(
            &self.indicators(),
            target_coverage,
            block_length,
            replicates,
            seed,
            self.series_id.clone(),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverageInferenceError {
    EmptySeriesIdentity,
    EmptySequence,
    SequenceTooLong,
    DuplicateForecastId(u64),
    NonFiniteTime,
    InvalidOutcomeTimeOrder,
    NonMonotonicIssueStep,
    NonMonotonicIssueTime,
    NonMonotonicEventTime,
    InvalidTargetCoverage,
    BandwidthTooLarge,
    ExcessiveHacWork,
    InvalidBlockLength,
    ExcessiveBootstrapWork,
    InvalidBootstrapReplicateCount,
    NegativeLongRunVariance,
    InvalidAciConfig,
    DuplicateForecastIssue(u64),
    NonMonotonicIssueStepForAci,
    DueFeedbackMustSettleBeforeIssue(u64),
    UnknownForecastId(u64),
    PrematureFeedback(u64, u64),
    LateFeedback(u64, u64),
    OutOfOrderFeedback(u64),
}

impl fmt::Display for CoverageInferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySeriesIdentity => write!(f, "coverage series identity must be non-empty"),
            Self::EmptySequence => write!(f, "coverage sequence must be non-empty"),
            Self::SequenceTooLong => {
                write!(f, "coverage sequence exceeds the declared size limit")
            }
            Self::DuplicateForecastId(id) => write!(f, "duplicate forecast ID {id}"),
            Self::NonFiniteTime => write!(f, "coverage times must be finite"),
            Self::InvalidOutcomeTimeOrder => {
                write!(f, "issue, event and availability times are inconsistent")
            }
            Self::NonMonotonicIssueStep => {
                write!(f, "coverage issue steps must be strictly increasing")
            }
            Self::NonMonotonicIssueTime => {
                write!(f, "coverage issue times must be strictly increasing")
            }
            Self::NonMonotonicEventTime => {
                write!(f, "coverage event times must be strictly increasing")
            }
            Self::InvalidTargetCoverage => {
                write!(f, "target coverage must be finite and strictly between zero and one")
            }
            Self::BandwidthTooLarge => {
                write!(f, "HAC bandwidth must be smaller than the sequence length")
            }
            Self::ExcessiveHacWork => {
                write!(f, "HAC sequence length times bandwidth exceeds the work limit")
            }
            Self::InvalidBlockLength => {
                write!(f, "block length must be between one and the sequence length")
            }
            Self::ExcessiveBootstrapWork => {
                write!(f, "bootstrap sequence length times replicates exceeds the work limit")
            }
            Self::InvalidBootstrapReplicateCount => {
                write!(f, "bootstrap replicate count is outside the admitted range")
            }
            Self::NegativeLongRunVariance => {
                write!(f, "HAC long-run variance is materially negative or non-finite")
            }
            Self::InvalidAciConfig => write!(f, "delayed ACI configuration is invalid"),
            Self::DuplicateForecastIssue(id) => {
                write!(f, "forecast ID {id} has already been issued")
            }
            Self::NonMonotonicIssueStepForAci => {
                write!(f, "ACI issue steps must be strictly increasing")
            }
            Self::DueFeedbackMustSettleBeforeIssue(id) => {
                write!(f, "due forecast {id} must settle before this lane is reused")
            }
            Self::UnknownForecastId(id) => {
                write!(f, "forecast ID {id} is unknown or already settled")
            }
            Self::PrematureFeedback(id, due) => {
                write!(f, "forecast {id} cannot settle before step {due}")
            }
            Self::LateFeedback(id, due) => {
                write!(f, "forecast {id} missed its fixed due step {due}")
            }
            Self::OutOfOrderFeedback(id) => write!(
                f,
                "feedback must settle in due-step order; expected forecast {id}"
            ),
        }
    }
}

impl std::error::Error for CoverageInferenceError {}

#[derive(Debug, Clone, PartialEq)]
pub struct HacCoverageReportV1 {
    pub series_id: String,
    pub samples: usize,
    pub covered_count: usize,
    pub target_coverage: f64,
    pub realized_coverage: f64,
    pub target_gap: f64,
    pub bandwidth: usize,
    pub long_run_variance: f64,
    pub hac_standard_error: f64,
    pub hac_normal_95: [f64; 2],
    pub iid_wilson_95: [f64; 2],
    pub effective_sample_size_diagnostic: Option<f64>,
}

pub fn hac_coverage_report(
    indicators: &[bool],
    target_coverage: f64,
    bandwidth: usize,
    series_id: String,
) -> Result<HacCoverageReportV1, CoverageInferenceError> {
    validate_target(target_coverage)?;
    if series_id.trim().is_empty() {
        return Err(CoverageInferenceError::EmptySeriesIdentity);
    }
    if indicators.is_empty() {
        return Err(CoverageInferenceError::EmptySequence);
    }
    if bandwidth >= indicators.len() {
        return Err(CoverageInferenceError::BandwidthTooLarge);
    }
    let n = indicators.len();
    if n.checked_mul(bandwidth + 1).unwrap_or(usize::MAX) > MAX_HAC_WORK {
        return Err(CoverageInferenceError::ExcessiveHacWork);
    }
    let covered_count = indicators.iter().filter(|covered| **covered).count();
    let mean = covered_count as f64 / n as f64;
    let centered: Vec<f64> = indicators
        .iter()
        .map(|covered| if *covered { 1.0 - mean } else { -mean })
        .collect();

    let mut long_run_variance =
        centered.iter().map(|value| value * value).sum::<f64>() / n as f64;
    for lag in 1..=bandwidth {
        let covariance = centered[lag..]
            .iter()
            .zip(centered[..(n - lag)].iter())
            .map(|(right, left)| right * left)
            .sum::<f64>()
            / n as f64;
        let weight = 1.0 - lag as f64 / (bandwidth as f64 + 1.0);
        long_run_variance += 2.0 * weight * covariance;
    }
    if !long_run_variance.is_finite() || long_run_variance < -1.0e-12 {
        return Err(CoverageInferenceError::NegativeLongRunVariance);
    }
    long_run_variance = long_run_variance.max(0.0);
    let standard_error = (long_run_variance / n as f64).sqrt();
    let iid_variance = mean * (1.0 - mean);
    let effective_sample_size_diagnostic =
        if iid_variance > 0.0 && long_run_variance > 0.0 {
            Some((iid_variance * n as f64 / long_run_variance).max(0.0))
        } else {
            None
        };
    let half_width = Z95 * standard_error;

    Ok(HacCoverageReportV1 {
        series_id,
        samples: n,
        covered_count,
        target_coverage,
        realized_coverage: mean,
        target_gap: mean - target_coverage,
        bandwidth,
        long_run_variance,
        hac_standard_error: standard_error,
        hac_normal_95: [
            (mean - half_width).max(0.0),
            (mean + half_width).min(1.0),
        ],
        iid_wilson_95: wilson_interval(covered_count, n),
        effective_sample_size_diagnostic,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockBootstrapCoverageReportV1 {
    pub series_id: String,
    pub samples: usize,
    pub covered_count: usize,
    pub realized_coverage: f64,
    pub target_coverage: f64,
    pub target_gap: f64,
    pub block_length: usize,
    pub replicates: usize,
    pub seed: u64,
    pub percentile_95: [f64; 2],
}

pub fn moving_block_bootstrap(
    indicators: &[bool],
    target_coverage: f64,
    block_length: usize,
    replicates: usize,
    seed: u64,
    series_id: String,
) -> Result<BlockBootstrapCoverageReportV1, CoverageInferenceError> {
    validate_target(target_coverage)?;
    if series_id.trim().is_empty() {
        return Err(CoverageInferenceError::EmptySeriesIdentity);
    }
    if indicators.is_empty() {
        return Err(CoverageInferenceError::EmptySequence);
    }
    if block_length == 0 || block_length > indicators.len() {
        return Err(CoverageInferenceError::InvalidBlockLength);
    }
    if replicates == 0 || replicates > MAX_BOOTSTRAP_REPLICATES {
        return Err(CoverageInferenceError::InvalidBootstrapReplicateCount);
    }
    if indicators.len().checked_mul(replicates).unwrap_or(usize::MAX)
        > MAX_BOOTSTRAP_WORK
    {
        return Err(CoverageInferenceError::ExcessiveBootstrapWork);
    }

    let n = indicators.len();
    let covered_count = indicators.iter().filter(|covered| **covered).count();
    let realized_coverage = covered_count as f64 / n as f64;
    let mut rng = SplitMix64::new(seed);
    let last_start = n - block_length;
    let mut replicate_means = Vec::with_capacity(replicates);
    for _ in 0..replicates {
        let mut covered = 0_usize;
        let mut sampled = 0_usize;
        while sampled < n {
            let start = rng.index(last_start + 1);
            for offset in 0..block_length {
                if sampled == n {
                    break;
                }
                if indicators[start + offset] {
                    covered += 1;
                }
                sampled += 1;
            }
        }
        replicate_means.push(covered as f64 / n as f64);
    }
    replicate_means.sort_by(f64::total_cmp);

    Ok(BlockBootstrapCoverageReportV1 {
        series_id,
        samples: n,
        covered_count,
        realized_coverage,
        target_coverage,
        target_gap: realized_coverage - target_coverage,
        block_length,
        replicates,
        seed,
        percentile_95: [
            percentile(&replicate_means, 0.025),
            percentile(&replicate_means, 0.975),
        ],
    })
}

fn validate_target(target: f64) -> Result<(), CoverageInferenceError> {
    if !target.is_finite() || target <= 0.0 || target >= 1.0 {
        return Err(CoverageInferenceError::InvalidTargetCoverage);
    }
    Ok(())
}

fn wilson_interval(successes: usize, total: usize) -> [f64; 2] {
    let n = total as f64;
    let p = successes as f64 / n;
    let z2 = Z95 * Z95;
    let denominator = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / denominator;
    let half = Z95 * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt()
        / denominator;
    [
        (center - half).max(0.0),
        (center + half).min(1.0),
    ]
}

fn percentile(sorted: &[f64], probability: f64) -> f64 {
    if sorted.len() == 1 {
        return sorted[0];
    }
    let position = probability.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let fraction = position - lower as f64;
        sorted[lower] + fraction * (sorted[upper] - sorted[lower])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(SPLITMIX_INCREMENT);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn index(&mut self, upper: usize) -> usize {
        ((self.next_u64() as u128 * upper as u128) >> 64) as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DelayedAciConfigV1 {
    pub target_miscoverage: f64,
    pub initial_miscoverage: f64,
    pub learning_rate: f64,
    pub minimum_miscoverage: f64,
    pub maximum_miscoverage: f64,
    pub delay_steps: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AciForecastTicketV1 {
    pub forecast_id: u64,
    pub issue_step: u64,
    pub due_step: u64,
    pub lane: usize,
    pub miscoverage_at_issue: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AciUpdateReceiptV1 {
    pub forecast_id: u64,
    pub issue_step: u64,
    pub availability_step: u64,
    pub lane: usize,
    pub covered: bool,
    pub miscoverage_at_issue: f64,
    pub miscoverage_after_feedback: f64,
    pub unclipped_miscoverage: f64,
    pub clipping_applied: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DelayedAciSchedulerV1 {
    config: DelayedAciConfigV1,
    lane_miscoverage: Vec<f64>,
    pending: BTreeMap<u64, AciForecastTicketV1>,
    issued_ids: BTreeSet<u64>,
    last_issue_step: Option<u64>,
    updates: Vec<AciUpdateReceiptV1>,
}

impl DelayedAciSchedulerV1 {
    pub fn new(config: DelayedAciConfigV1) -> Result<Self, CoverageInferenceError> {
        if !config.target_miscoverage.is_finite()
            || !config.initial_miscoverage.is_finite()
            || !config.learning_rate.is_finite()
            || !config.minimum_miscoverage.is_finite()
            || !config.maximum_miscoverage.is_finite()
            || config.target_miscoverage <= 0.0
            || config.target_miscoverage >= 1.0
            || config.minimum_miscoverage <= 0.0
            || config.maximum_miscoverage >= 1.0
            || config.minimum_miscoverage >= config.maximum_miscoverage
            || config.target_miscoverage < config.minimum_miscoverage
            || config.target_miscoverage > config.maximum_miscoverage
            || config.initial_miscoverage < config.minimum_miscoverage
            || config.initial_miscoverage > config.maximum_miscoverage
            || config.learning_rate <= 0.0
            || config.learning_rate > 1.0
            || config.delay_steps == 0
        {
            return Err(CoverageInferenceError::InvalidAciConfig);
        }
        let lane_count = usize::try_from(config.delay_steps)
            .map_err(|_| CoverageInferenceError::InvalidAciConfig)?;
        if lane_count > 100_000 {
            return Err(CoverageInferenceError::InvalidAciConfig);
        }
        Ok(Self {
            config,
            lane_miscoverage: vec![config.initial_miscoverage; lane_count],
            pending: BTreeMap::new(),
            issued_ids: BTreeSet::new(),
            last_issue_step: None,
            updates: Vec::new(),
        })
    }

    pub fn issue(
        &mut self,
        forecast_id: u64,
        issue_step: u64,
    ) -> Result<AciForecastTicketV1, CoverageInferenceError> {
        if self.issued_ids.contains(&forecast_id) {
            return Err(CoverageInferenceError::DuplicateForecastIssue(forecast_id));
        }
        if self
            .last_issue_step
            .is_some_and(|previous| issue_step <= previous)
        {
            return Err(CoverageInferenceError::NonMonotonicIssueStepForAci);
        }
        let lane = (issue_step % self.config.delay_steps) as usize;
        if let Some(due) = self
            .pending
            .values()
            .find(|ticket| ticket.due_step <= issue_step)
        {
            return Err(CoverageInferenceError::DueFeedbackMustSettleBeforeIssue(
                due.forecast_id,
            ));
        }
        let due_step = issue_step
            .checked_add(self.config.delay_steps)
            .ok_or(CoverageInferenceError::InvalidAciConfig)?;
        let ticket = AciForecastTicketV1 {
            forecast_id,
            issue_step,
            due_step,
            lane,
            miscoverage_at_issue: self.lane_miscoverage[lane],
        };
        self.pending.insert(forecast_id, ticket);
        self.issued_ids.insert(forecast_id);
        self.last_issue_step = Some(issue_step);
        Ok(ticket)
    }

    pub fn resolve(
        &mut self,
        forecast_id: u64,
        availability_step: u64,
        covered: bool,
    ) -> Result<AciUpdateReceiptV1, CoverageInferenceError> {
        let ticket = self
            .pending
            .get(&forecast_id)
            .copied()
            .ok_or(CoverageInferenceError::UnknownForecastId(forecast_id))?;
        if availability_step < ticket.due_step {
            return Err(CoverageInferenceError::PrematureFeedback(
                forecast_id,
                ticket.due_step,
            ));
        }
        if availability_step > ticket.due_step {
            return Err(CoverageInferenceError::LateFeedback(
                forecast_id,
                ticket.due_step,
            ));
        }
        if let Some(expected) = self.pending.values().min_by_key(|item| item.due_step) {
            if expected.forecast_id != forecast_id {
                return Err(CoverageInferenceError::OutOfOrderFeedback(
                    expected.forecast_id,
                ));
            }
        }

        let miss = if covered { 0.0 } else { 1.0 };
        let unclipped = ticket.miscoverage_at_issue
            + self.config.learning_rate * (self.config.target_miscoverage - miss);
        let clipped = unclipped.clamp(
            self.config.minimum_miscoverage,
            self.config.maximum_miscoverage,
        );
        let receipt = AciUpdateReceiptV1 {
            forecast_id,
            issue_step: ticket.issue_step,
            availability_step,
            lane: ticket.lane,
            covered,
            miscoverage_at_issue: ticket.miscoverage_at_issue,
            miscoverage_after_feedback: clipped,
            unclipped_miscoverage: unclipped,
            clipping_applied: clipped != unclipped,
        };
        self.lane_miscoverage[ticket.lane] = clipped;
        self.pending.remove(&forecast_id);
        self.updates.push(receipt);
        Ok(receipt)
    }

    pub fn lane_miscoverage(&self, lane: usize) -> Option<f64> {
        self.lane_miscoverage.get(lane).copied()
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn updates(&self) -> &[AciUpdateReceiptV1] {
        &self.updates
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn series(indicators: &[bool]) -> CoverageSeriesV1 {
        CoverageSeriesV1::new(
            "coverage-series-fixture".to_owned(),
            indicators
                .iter()
                .enumerate()
                .map(|(index, covered)| CoverageOutcomeV1 {
                    forecast_id: index as u64 + 1,
                    issue_step: index as u64,
                    availability_step: index as u64 + 1,
                    issue_time: index as f64,
                    event_time: index as f64 + 0.5,
                    availability_time: index as f64 + 1.0,
                    covered: *covered,
                })
                .collect(),
        )
        .unwrap()
    }

    fn config() -> DelayedAciConfigV1 {
        DelayedAciConfigV1 {
            target_miscoverage: 0.1,
            initial_miscoverage: 0.1,
            learning_rate: 0.05,
            minimum_miscoverage: 0.01,
            maximum_miscoverage: 0.5,
            delay_steps: 2,
        }
    }

    #[test]
    fn bandwidth_zero_matches_iid_bernoulli_variance_component() {
        let report = series(&[true, false, true, false])
            .hac_report(0.5, 0)
            .unwrap();
        assert!((report.hac_standard_error - 0.25).abs() < 1.0e-14);
        assert_eq!(report.bandwidth, 0);
        assert_eq!(report.samples, 4);
    }

    #[test]
    fn positive_serial_dependence_increases_hac_uncertainty() {
        let report = series(&[
            true, true, true, false, false, false, true, true, true, false, false, false,
        ])
        .hac_report(0.5, 3)
        .unwrap();
        let iid_se = (report.realized_coverage
            * (1.0 - report.realized_coverage)
            / report.samples as f64)
            .sqrt();
        assert!(report.hac_standard_error > iid_se);
        assert!(report.hac_normal_95[1] - report.hac_normal_95[0]
            > report.iid_wilson_95[1] - report.iid_wilson_95[0]);
    }

    #[test]
    fn moving_block_bootstrap_is_deterministic_and_reports_block_identity() {
        let series = series(&[
            true, true, false, false, true, true, false, false, true, true, false, false,
        ]);
        let first = series.moving_block_bootstrap(0.5, 3, 2_000, 77).unwrap();
        let second = series.moving_block_bootstrap(0.5, 3, 2_000, 77).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.block_length, 3);
        assert_eq!(first.replicates, 2_000);
        assert!(first.percentile_95[0] >= 0.0);
        assert!(first.percentile_95[1] <= 1.0);
    }

    #[test]
    fn hac_rejects_invalid_bandwidth_and_empty_sequences() {
        assert!(matches!(
            hac_coverage_report(&[], 0.9, 0, "series".to_owned()),
            Err(CoverageInferenceError::EmptySequence)
        ));
        assert!(matches!(
            hac_coverage_report(&[true, false], 0.9, 2, "series".to_owned()),
            Err(CoverageInferenceError::BandwidthTooLarge)
        ));
    }

    #[test]
    fn hac_and_bootstrap_reject_excessive_work_before_allocating() {
        let indicators = vec![true; 20_000];
        assert!(matches!(
            hac_coverage_report(&indicators, 0.9, 10_000, "series".to_owned()),
            Err(CoverageInferenceError::ExcessiveHacWork)
        ));
        assert!(matches!(
            moving_block_bootstrap(
                &indicators,
                0.9,
                10,
                10_000,
                77,
                "series".to_owned()
            ),
            Err(CoverageInferenceError::ExcessiveBootstrapWork)
        ));
    }

    #[test]
    fn delayed_aci_does_not_update_before_label_availability() {
        let mut scheduler = DelayedAciSchedulerV1::new(config()).unwrap();
        let ticket = scheduler.issue(10, 0).unwrap();
        assert_eq!(ticket.miscoverage_at_issue, 0.1);
        assert!(matches!(
            scheduler.resolve(10, 1, false),
            Err(CoverageInferenceError::PrematureFeedback(10, 2))
        ));
        assert_eq!(scheduler.lane_miscoverage(0), Some(0.1));
        let update = scheduler.resolve(10, 2, true).unwrap();
        assert!((update.miscoverage_after_feedback - 0.105).abs() < 1.0e-14);
    }

    #[test]
    fn delayed_aci_uses_interleaved_lanes_and_preserves_issue_level() {
        let mut scheduler = DelayedAciSchedulerV1::new(config()).unwrap();
        let first = scheduler.issue(10, 0).unwrap();
        let second = scheduler.issue(11, 1).unwrap();
        assert_eq!(first.lane, 0);
        assert_eq!(second.lane, 1);
        let update = scheduler.resolve(10, 2, true).unwrap();
        assert!((update.miscoverage_after_feedback - 0.105).abs() < 1.0e-14);
        let third = scheduler.issue(12, 2).unwrap();
        assert_eq!(third.lane, 0);
        assert!((third.miscoverage_at_issue - 0.105).abs() < 1.0e-14);
        assert_eq!(second.miscoverage_at_issue, 0.1);
    }

    #[test]
    fn delayed_aci_rejects_duplicate_out_of_order_and_late_feedback() {
        let mut scheduler = DelayedAciSchedulerV1::new(config()).unwrap();
        scheduler.issue(20, 0).unwrap();
        scheduler.issue(21, 1).unwrap();
        assert!(matches!(
            scheduler.resolve(21, 3, true),
            Err(CoverageInferenceError::OutOfOrderFeedback(20))
        ));
        assert!(matches!(
            scheduler.resolve(20, 3, true),
            Err(CoverageInferenceError::LateFeedback(20, 2))
        ));
        scheduler.resolve(20, 2, false).unwrap();
        assert!(matches!(
            scheduler.resolve(20, 2, false),
            Err(CoverageInferenceError::UnknownForecastId(20))
        ));
    }

    #[test]
    fn any_overdue_feedback_blocks_new_issue_even_on_a_different_lane() {
        let mut scheduler = DelayedAciSchedulerV1::new(config()).unwrap();
        scheduler.issue(40, 0).unwrap();
        assert!(matches!(
            scheduler.issue(41, 3),
            Err(CoverageInferenceError::DueFeedbackMustSettleBeforeIssue(40))
        ));
    }

    #[test]
    fn target_miscoverage_must_be_inside_the_admitted_alpha_bounds() {
        let invalid = DelayedAciConfigV1 {
            target_miscoverage: 0.8,
            ..config()
        };
        assert!(matches!(
            DelayedAciSchedulerV1::new(invalid),
            Err(CoverageInferenceError::InvalidAciConfig)
        ));
    }

    #[test]
    fn due_feedback_must_settle_before_reusing_a_delay_lane() {
        let mut scheduler = DelayedAciSchedulerV1::new(config()).unwrap();
        scheduler.issue(30, 0).unwrap();
        assert!(matches!(
            scheduler.issue(31, 2),
            Err(CoverageInferenceError::DueFeedbackMustSettleBeforeIssue(30))
        ));
        scheduler.resolve(30, 2, true).unwrap();
        let next = scheduler.issue(31, 2).unwrap();
        assert!((next.miscoverage_at_issue - 0.105).abs() < 1.0e-14);
    }

    #[test]
    fn duplicate_forecast_ids_fail_closed() {
        let outcome = CoverageOutcomeV1 {
            forecast_id: 1,
            issue_step: 0,
            availability_step: 1,
            issue_time: 0.0,
            event_time: 0.5,
            availability_time: 1.0,
            covered: true,
        };
        assert!(matches!(
            CoverageSeriesV1::new(
                "duplicate".to_owned(),
                vec![outcome, CoverageOutcomeV1 { issue_step: 1, ..outcome }]
            ),
            Err(CoverageInferenceError::DuplicateForecastId(1))
        ));
    }
}
