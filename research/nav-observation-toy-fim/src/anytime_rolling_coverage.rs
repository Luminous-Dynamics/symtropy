//! Anytime-valid rolling-window miscoverage control.
//!
//! Research implementation of Fang (2026), arXiv:2611.20905, Section 4.2,
//! Algorithm 2. It uses a two-sided stitched empirical-Bernstein confidence
//! sequence, a certified rolling miss sum, and a prospective feasibility
//! check for both possible outcomes across every future window that can
//! contain the next episode.
//!
//! This module chooses bounded-set/abstention and the requested miscoverage
//! level. It does not construct the prediction set, authenticate source
//! receipts, or authorize actuators. The caller must preserve the filtration:
//! decision before truth, sealed full-episode truth after decision.

use std::fmt;

use crate::rolling_feasibility_range_index::{
    ConservativeRollingRangeIndex, OutwardInterval, RollingRangeDecision,
    RollingRangeProofWitness,
};

const SHA256_BYTES: usize = 32;
const LOG_LOG_COEFFICIENT: f64 = 0.72;
const SQRT_MARGIN_COEFFICIENT: f64 = 1.7;
const LOG_LOG_MARGIN_COEFFICIENT: f64 = 3.4;
const LOG_MARGIN_COEFFICIENT: f64 = 2.45;
const STITCHED_LOG_CONSTANT: f64 = 5.19;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RollingWindowRule {
    /// Fixed reference window. Its asymptotic efficiency loss does not vanish.
    Fixed { length: u64 },
    /// k_t = ceil(t * numerator / denominator), with 0 < numerator <= denominator.
    Proportional { numerator: u64, denominator: u64 },
    /// k_t = ceil(t^beta), with 1/2 < beta < 1.
    Polynomial { beta: f64 },
}

impl RollingWindowRule {
    pub fn validate(self) -> Result<(), RollingWindowError> {
        match self {
            Self::Fixed { length } if length == 0 => {
                Err(RollingWindowError::InvalidWindowRule)
            }
            Self::Proportional {
                numerator,
                denominator,
            } if numerator == 0 || denominator == 0 || numerator > denominator => {
                Err(RollingWindowError::InvalidWindowRule)
            }
            Self::Polynomial { beta }
                if !beta.is_finite() || beta <= 0.5 || beta >= 1.0 =>
            {
                Err(RollingWindowError::InvalidWindowRule)
            }
            _ => Ok(()),
        }
    }

    /// Return (window length, start index) at one-based episode index t.
    pub fn window_at(self, t: u64) -> Result<(u64, u64), RollingWindowError> {
        self.validate()?;
        if t == 0 {
            return Err(RollingWindowError::InvalidEpisodeIndex);
        }
        let raw_length = match self {
            Self::Fixed { length } => length.min(t),
            Self::Proportional {
                numerator,
                denominator,
            } => {
                let product = (t as u128)
                    .checked_mul(numerator as u128)
                    .ok_or(RollingWindowError::ArithmeticOverflow)?;
                let quotient = product / denominator as u128;
                let remainder = product % denominator as u128;
                let rounded = quotient
                    .checked_add(u128::from(remainder != 0))
                    .ok_or(RollingWindowError::ArithmeticOverflow)?;
                u64::try_from(rounded).map_err(|_| RollingWindowError::ArithmeticOverflow)?
            }
            Self::Polynomial { beta } => {
                let value = (t as f64).powf(beta).ceil();
                if !value.is_finite() || value < 1.0 || value > u64::MAX as f64 {
                    return Err(RollingWindowError::ArithmeticOverflow);
                }
                value as u64
            }
        };
        let length = raw_length.clamp(1, t);
        Ok((length, t - length))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RollingWindowPolicyConfig {
    pub target_miscoverage: f64,
    /// Overall confidence-sequence error probability; two-sided margins use delta/2.
    pub confidence_error_probability: f64,
    pub learning_rate_gamma: f64,
    /// Nonnegative efficiency buffer. Unlike running-average control, no theorem
    /// lower bound on this buffer replaces the rolling feasibility check.
    pub buffer: f64,
    pub window_rule: RollingWindowRule,
    /// Maximum starts for the exact reference scan. The optional range proof
    /// may certify a pass without enumerating these starts; if it cannot, this
    /// budget still forces abstention rather than truncating the reference check.
    pub maximum_feasibility_starts: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RollingAbstentionReason {
    NonPositiveMiscoverageLevel,
    ProspectiveFeasibilityFailed,
    FeasibilityBudgetExceeded,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RollingFeasibilityWitness {
    pub window_start: u64,
    pub first_occurrence_at: u64,
    pub candidate_miss: bool,
    pub certified_miss_sum: f64,
    pub allowed_miss_sum: f64,
    pub passed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RollingWindowDisposition {
    BoundedSetRequested,
    Abstain(RollingAbstentionReason),
}

#[derive(Clone, Debug, PartialEq)]
pub struct RollingWindowDecision {
    pub decision_sequence_id: u64,
    pub episode_index: u64,
    pub window_length: u64,
    pub window_start: u64,
    pub requested_miscoverage_level: f64,
    pub disposition: RollingWindowDisposition,
    /// Per-start witnesses from the exact enumerated reference path. On the
    /// accelerated path this includes the exact current-start checks only.
    pub feasibility_witnesses: Vec<RollingFeasibilityWitness>,
    /// Compact range proof witnesses for strictly future starts. Empty when
    /// the reference path was used or no future starts existed.
    pub range_proof_witnesses: Vec<RollingRangeProofWitness>,
}

impl RollingWindowDecision {
    pub fn is_bounded_set_requested(&self) -> bool {
        matches!(self.disposition, RollingWindowDisposition::BoundedSetRequested)
    }

    pub fn is_abstention(&self) -> bool {
        !self.is_bounded_set_requested()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RollingWindowError {
    InvalidConfiguration,
    InvalidWindowRule,
    InvalidEpisodeIndex,
    PendingDecisionOutstanding,
    NoPendingDecision,
    DecisionDoesNotMatchPending,
    InvalidEpisodeIdentity,
    EpisodeIdentityReplayOrReordering,
    EpisodeSequenceReplayOrReordering,
    DecisionReceiptSequenceReplayOrReordering,
    MissingDecisionReceipt,
    DecisionReceiptReplay,
    MissingOutcomeForBoundedEpisode,
    UnexpectedOutcomeForAbstention,
    MissingOutcomeReceipt,
    UnexpectedOutcomeReceiptForAbstention,
    OutcomeReceiptSequenceReplayOrReordering,
    OutcomeReceiptReplay,
    HistoryIndexUnavailable,
    CertifiedCountUnderflow,
    CounterOverflow,
    ArithmeticOverflow,
}

impl fmt::Display for RollingWindowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration => write!(f, "alpha, delta, gamma, buffer and feasibility budget are invalid"),
            Self::InvalidWindowRule => write!(f, "window rule is invalid"),
            Self::InvalidEpisodeIndex => write!(f, "episode index must be positive"),
            Self::PendingDecisionOutstanding => write!(f, "the previous rolling-window decision has not been completed"),
            Self::NoPendingDecision => write!(f, "there is no pending rolling-window decision"),
            Self::DecisionDoesNotMatchPending => write!(f, "completed decision does not match the pending sealed decision"),
            Self::InvalidEpisodeIdentity => write!(f, "episode and sequence identities must be nonzero"),
            Self::EpisodeIdentityReplayOrReordering => write!(f, "episode identity is not strictly increasing"),
            Self::EpisodeSequenceReplayOrReordering => write!(f, "episode sequence is not strictly increasing"),
            Self::DecisionReceiptSequenceReplayOrReordering => write!(f, "decision receipt sequence is not strictly increasing"),
            Self::MissingDecisionReceipt => write!(f, "decision receipt digest is missing"),
            Self::DecisionReceiptReplay => write!(f, "decision receipt digest was replayed"),
            Self::MissingOutcomeForBoundedEpisode => write!(f, "a bounded episode requires a sealed full-episode hit/miss outcome"),
            Self::UnexpectedOutcomeForAbstention => write!(f, "an abstention must not carry a bounded-set hit/miss outcome"),
            Self::MissingOutcomeReceipt => write!(f, "a bounded episode requires a sealed outcome receipt"),
            Self::UnexpectedOutcomeReceiptForAbstention => write!(f, "an abstention must not carry a truth outcome receipt"),
            Self::OutcomeReceiptSequenceReplayOrReordering => write!(f, "outcome receipt sequence is not strictly increasing"),
            Self::OutcomeReceiptReplay => write!(f, "outcome receipt digest was replayed"),
            Self::HistoryIndexUnavailable => write!(f, "required rolling prefix snapshot is unavailable"),
            Self::CertifiedCountUnderflow => write!(f, "rolling prefix count or miss difference underflowed"),
            Self::CounterOverflow => write!(f, "rolling episode counter overflowed"),
            Self::ArithmeticOverflow => write!(f, "rolling confidence or feasibility arithmetic overflowed"),
        }
    }
}

impl std::error::Error for RollingWindowError {}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PrefixSnapshot {
    bounded_count: u64,
    miss_count: u64,
    predictable_variation: f64,
    /// s * omega'_s, the scaled two-sided confidence margin at this prefix.
    scaled_margin: f64,
}

impl PrefixSnapshot {
    fn zero() -> Self {
        Self {
            bounded_count: 0,
            miss_count: 0,
            predictable_variation: 0.0,
            scaled_margin: 0.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RollingWindowReport {
    pub monitoring_horizon: u64,
    pub current_window_length: u64,
    pub current_window_start: u64,
    pub bounded_episode_count: u64,
    pub missed_episode_count: u64,
    pub abstention_count: u64,
    pub observed_rolling_miscoverage: f64,
    pub bounded_set_availability_in_window: f64,
    pub certified_rolling_miscoverage_upper: f64,
    pub certified_rolling_miss_sum: f64,
    pub target_miscoverage: f64,
    pub confidence_level: f64,
    pub next_requested_miscoverage_level: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnytimeRollingCoveragePolicy {
    config: RollingWindowPolicyConfig,
    monitoring_horizon: u64,
    bounded_episode_count: u64,
    missed_episode_count: u64,
    abstention_count: u64,
    prefixes: Vec<PrefixSnapshot>,
    last_window_length: u64,
    last_window_start: u64,
    last_certified_rolling_sum: f64,
    last_episode_id: Option<u64>,
    last_episode_sequence_id: Option<u64>,
    last_decision_receipt_sequence_id: Option<u64>,
    last_decision_receipt_sha256: Option<[u8; SHA256_BYTES]>,
    last_outcome_receipt_sequence_id: Option<u64>,
    last_outcome_receipt_sha256: Option<[u8; SHA256_BYTES]>,
    pending_decision: Option<RollingWindowDecision>,
    /// Optional acceleration cache. Any interval/index uncertainty disables
    /// this cache and leaves the exact reference policy fully operational.
    rolling_range_index: Option<ConservativeRollingRangeIndex>,
}

impl AnytimeRollingCoveragePolicy {
    pub fn new(config: RollingWindowPolicyConfig) -> Result<Self, RollingWindowError> {
        if !config.target_miscoverage.is_finite()
            || config.target_miscoverage <= 0.0
            || config.target_miscoverage >= 1.0
            || !config.confidence_error_probability.is_finite()
            || config.confidence_error_probability <= 0.0
            || config.confidence_error_probability >= 1.0
            || !config.learning_rate_gamma.is_finite()
            || config.learning_rate_gamma <= 0.0
            || !config.buffer.is_finite()
            || config.buffer < 0.0
            || config.maximum_feasibility_starts == 0
        {
            return Err(RollingWindowError::InvalidConfiguration);
        }
        config.window_rule.validate()?;

        let mut range_index = ConservativeRollingRangeIndex::new();
        let initial_prefix = PrefixSnapshot::zero();
        let rolling_range_index = match prefix_range_intervals(
            initial_prefix,
            0,
            config.window_rule,
            config.target_miscoverage,
        ) {
            Ok(Some((r_interval, q_interval))) => range_index
                .append(r_interval, q_interval)
                .ok()
                .map(|_| range_index),
            _ => None,
        };

        Ok(Self {
            config,
            monitoring_horizon: 0,
            bounded_episode_count: 0,
            missed_episode_count: 0,
            abstention_count: 0,
            prefixes: vec![PrefixSnapshot::zero()],
            last_window_length: 0,
            last_window_start: 0,
            last_certified_rolling_sum: 0.0,
            last_episode_id: None,
            last_episode_sequence_id: None,
            last_decision_receipt_sequence_id: None,
            last_decision_receipt_sha256: None,
            last_outcome_receipt_sequence_id: None,
            last_outcome_receipt_sha256: None,
            pending_decision: None,
            rolling_range_index,
        })
    }

    pub fn config(&self) -> RollingWindowPolicyConfig {
        self.config
    }

    pub fn monitoring_horizon(&self) -> u64 {
        self.monitoring_horizon
    }

    pub fn bounded_episode_count(&self) -> u64 {
        self.bounded_episode_count
    }

    pub fn missed_episode_count(&self) -> u64 {
        self.missed_episode_count
    }

    pub fn abstention_count(&self) -> u64 {
        self.abstention_count
    }

    /// Decide for the next episode before any truth outcome is available.
    ///
    /// This remains the enumerated reference implementation. It checks every
    /// prospective start and both possible outcomes, or abstains if the exact
    /// scan would exceed the configured budget.
    pub fn decide_next_episode(&mut self) -> Result<RollingWindowDecision, RollingWindowError> {
        self.decide_next_episode_internal(false)
    }

    /// Opt-in acceleration using a conservative append-only range-min proof.
    ///
    /// The currently active start is checked by the exact reference equations.
    /// Only strictly later starts are compressed by the R/Q range proof. Any
    /// unavailable index, ambiguous interval, failed proof, or numerical issue
    /// falls back to the complete reference scan. The default method above is
    /// unchanged and remains the reference oracle.
    pub fn decide_next_episode_accelerated(
        &mut self,
    ) -> Result<RollingWindowDecision, RollingWindowError> {
        self.decide_next_episode_internal(true)
    }

    fn decide_next_episode_internal(
        &mut self,
        allow_acceleration: bool,
    ) -> Result<RollingWindowDecision, RollingWindowError> {
        if self.pending_decision.is_some() {
            return Err(RollingWindowError::PendingDecisionOutstanding);
        }

        let t = self
            .monitoring_horizon
            .checked_add(1)
            .ok_or(RollingWindowError::CounterOverflow)?;
        let (window_length, window_start) = self.config.window_rule.window_at(t)?;
        let allowance = self.config.target_miscoverage * self.last_window_length as f64;
        let eta = self.config.target_miscoverage
            + self.config.learning_rate_gamma
                * (allowance - self.last_certified_rolling_sum - self.config.buffer);
        if !eta.is_finite() {
            return Err(RollingWindowError::ArithmeticOverflow);
        }

        let mut witnesses = Vec::new();
        let mut range_proof_witnesses = Vec::new();
        let mut disposition = if eta <= 0.0 {
            RollingWindowDisposition::Abstain(
                RollingAbstentionReason::NonPositiveMiscoverageLevel,
            )
        } else {
            RollingWindowDisposition::BoundedSetRequested
        };

        if eta > 0.0 {
            let accelerated = if allow_acceleration {
                self.try_accelerated_range_pass(t, window_start)?
            } else {
                None
            };

            if let Some((current_witnesses, range_witnesses)) = accelerated {
                witnesses = current_witnesses;
                range_proof_witnesses = range_witnesses;
            } else {
                // Exact reference path. Never truncate the prospective window
                // set: an exhausted budget means abstention.
                let full_history_only = is_full_history_only(self.config.window_rule);
                let candidate_count = if full_history_only {
                    1_usize
                } else {
                    usize::try_from(window_length).unwrap_or(usize::MAX)
                };

                if candidate_count > self.config.maximum_feasibility_starts {
                    disposition = RollingWindowDisposition::Abstain(
                        RollingAbstentionReason::FeasibilityBudgetExceeded,
                    );
                } else {
                    let previous = *self
                        .prefixes
                        .get(usize::try_from(self.monitoring_horizon)
                            .map_err(|_| RollingWindowError::HistoryIndexUnavailable)?)
                        .ok_or(RollingWindowError::HistoryIndexUnavailable)?;

                    let candidate_starts = if full_history_only {
                        vec![0_u64]
                    } else {
                        (window_start..t).collect::<Vec<_>>()
                    };

                    for start in candidate_starts {
                        let Some(first_occurrence_at) =
                            first_time_with_window_start(self.config.window_rule, start)?
                        else {
                            continue;
                        };
                        let prefix_index = usize::try_from(start)
                            .map_err(|_| RollingWindowError::HistoryIndexUnavailable)?;
                        let prefix = *self
                            .prefixes
                            .get(prefix_index)
                            .ok_or(RollingWindowError::HistoryIndexUnavailable)?;
                        witnesses.extend(evaluate_start_candidates(
                            self.config,
                            t,
                            previous,
                            start,
                            first_occurrence_at,
                            prefix,
                        )?);
                    }

                    if witnesses.iter().any(|witness| !witness.passed) {
                        disposition = RollingWindowDisposition::Abstain(
                            RollingAbstentionReason::ProspectiveFeasibilityFailed,
                        );
                    }
                }
            }
        }

        let decision = RollingWindowDecision {
            decision_sequence_id: t,
            episode_index: t,
            window_length,
            window_start,
            requested_miscoverage_level: eta,
            disposition,
            feasibility_witnesses: witnesses,
            range_proof_witnesses,
        };

        self.pending_decision = Some(decision.clone());
        Ok(decision)
    }

    /// Returns Some only when the current start passes the exact reference
    /// check and the compact interval index proves both candidate outcomes for
    /// every strictly future start. None always means "run the reference path".
    fn try_accelerated_range_pass(
        &self,
        t: u64,
        window_start: u64,
    ) -> Result<Option<(Vec<RollingFeasibilityWitness>, Vec<RollingRangeProofWitness>)>, RollingWindowError> {
        let previous_index = usize::try_from(self.monitoring_horizon)
            .map_err(|_| RollingWindowError::HistoryIndexUnavailable)?;
        let Some(previous) = self.prefixes.get(previous_index).copied() else {
            return Ok(None);
        };

        let Some(current_first_occurrence) =
            first_time_with_window_start(self.config.window_rule, window_start)?
        else {
            return Ok(None);
        };
        if current_first_occurrence > t {
            return Ok(None);
        }

        let current_prefix_index = usize::try_from(window_start)
            .map_err(|_| RollingWindowError::HistoryIndexUnavailable)?;
        let Some(current_prefix) = self.prefixes.get(current_prefix_index).copied() else {
            return Ok(None);
        };
        let current_witnesses = evaluate_start_candidates(
            self.config,
            t,
            previous,
            window_start,
            current_first_occurrence,
            current_prefix,
        )?;
        if current_witnesses.iter().any(|witness| !witness.passed) {
            return Ok(None);
        }

        let mut range_witnesses = Vec::new();
        if !is_full_history_only(self.config.window_rule) {
            let future_start = window_start
                .checked_add(1)
                .ok_or(RollingWindowError::ArithmeticOverflow)?;
            if future_start < t {
                // Monotonicity of the supported start schedules implies that
                // all starts after the active start are first seen after t.
                // Check the first such start explicitly before using the index.
                let Some(first_future_occurrence) =
                    first_time_with_window_start(self.config.window_rule, future_start)?
                else {
                    return Ok(None);
                };
                if first_future_occurrence <= t {
                    return Ok(None);
                }

                let Some(index) = self.rolling_range_index.as_ref() else {
                    return Ok(None);
                };
                let end = usize::try_from(t)
                    .map_err(|_| RollingWindowError::HistoryIndexUnavailable)?;
                let start = usize::try_from(future_start)
                    .map_err(|_| RollingWindowError::HistoryIndexUnavailable)?;
                if index.len() != self.prefixes.len() || index.len() < end {
                    return Ok(None);
                }

                let bounded_total = previous
                    .bounded_count
                    .checked_add(1)
                    .ok_or(RollingWindowError::CounterOverflow)?;
                let bounded_value = bounded_total as f64;
                let r_threshold = match guarded_interval(
                    bounded_value,
                    bounded_value.abs() + 1.0,
                ) {
                    Ok(interval) => interval,
                    Err(_) => return Ok(None),
                };

                for candidate_miss in [false, true] {
                    let z = if candidate_miss { 1_u64 } else { 0_u64 };
                    let candidate_misses = previous
                        .miss_count
                        .checked_add(z)
                        .ok_or(RollingWindowError::CounterOverflow)?;
                    let prior_mean = if previous.bounded_count == 0 {
                        0.0
                    } else {
                        previous.miss_count as f64 / previous.bounded_count as f64
                    };
                    let candidate_variation = previous.predictable_variation
                        + (z as f64 - prior_mean).powi(2);
                    if !candidate_variation.is_finite() {
                        return Err(RollingWindowError::ArithmeticOverflow);
                    }
                    let candidate_margin = match stitched_two_sided_scaled_margin(
                        t,
                        candidate_variation,
                        self.config.confidence_error_probability,
                    ) {
                        Ok(margin) => margin,
                        Err(_) => return Ok(None),
                    };
                    let q_value = candidate_misses as f64 + candidate_margin;
                    let q_scale =
                        (candidate_misses as f64).abs() + candidate_margin.abs() + 1.0;
                    let q_threshold = match guarded_interval(q_value, q_scale) {
                        Ok(interval) => interval,
                        Err(_) => return Ok(None),
                    };

                    match index.prove_future_prefix(
                        start,
                        end,
                        r_threshold,
                        q_threshold,
                    ) {
                        Ok(RollingRangeDecision::ProvenPass(witness)) => {
                            range_witnesses.push(witness);
                        }
                        Ok(RollingRangeDecision::RequiresReference(_)) | Err(_) => {
                            return Ok(None);
                        }
                    }
                }
            }
        }

        Ok(Some((current_witnesses, range_witnesses)))
    }

    /// Complete the pending episode only after the whole trajectory has been
    /// sealed and independently adjudicated.
    ///
    /// A bounded decision requires a hit/miss and a monotonic outcome receipt.
    /// An abstention carries no hit/miss or outcome receipt and contributes no
    /// bounded-set miss count. Invalid inputs do not consume counters or
    /// sequence watermarks.
    pub fn complete_episode(
        &mut self,
        decision: &RollingWindowDecision,
        episode_id: u64,
        episode_sequence_id: u64,
        decision_receipt_sequence_id: u64,
        decision_receipt_sha256: [u8; SHA256_BYTES],
        missed: Option<bool>,
        outcome_receipt_sequence_id: Option<u64>,
        outcome_receipt_sha256: Option<[u8; SHA256_BYTES]>,
    ) -> Result<RollingWindowReport, RollingWindowError> {
        let pending = self
            .pending_decision
            .as_ref()
            .ok_or(RollingWindowError::NoPendingDecision)?;
        if pending != decision {
            return Err(RollingWindowError::DecisionDoesNotMatchPending);
        }
        if episode_id == 0 || episode_sequence_id == 0 || decision_receipt_sequence_id == 0 {
            return Err(RollingWindowError::InvalidEpisodeIdentity);
        }
        if self.last_episode_id.is_some_and(|last| episode_id <= last) {
            return Err(RollingWindowError::EpisodeIdentityReplayOrReordering);
        }
        if self
            .last_episode_sequence_id
            .is_some_and(|last| episode_sequence_id <= last)
        {
            return Err(RollingWindowError::EpisodeSequenceReplayOrReordering);
        }
        if self
            .last_decision_receipt_sequence_id
            .is_some_and(|last| decision_receipt_sequence_id <= last)
        {
            return Err(RollingWindowError::DecisionReceiptSequenceReplayOrReordering);
        }
        if decision_receipt_sha256.iter().all(|byte| *byte == 0) {
            return Err(RollingWindowError::MissingDecisionReceipt);
        }
        if self.last_decision_receipt_sha256 == Some(decision_receipt_sha256) {
            return Err(RollingWindowError::DecisionReceiptReplay);
        }

        let bounded = decision.is_bounded_set_requested();
        let missed_outcome = if bounded {
            let missed = missed.ok_or(RollingWindowError::MissingOutcomeForBoundedEpisode)?;
            let outcome_sequence =
                outcome_receipt_sequence_id.ok_or(RollingWindowError::MissingOutcomeReceipt)?;
            let outcome_digest =
                outcome_receipt_sha256.ok_or(RollingWindowError::MissingOutcomeReceipt)?;
            if outcome_sequence == 0 || outcome_digest.iter().all(|byte| *byte == 0) {
                return Err(RollingWindowError::MissingOutcomeReceipt);
            }
            if self
                .last_outcome_receipt_sequence_id
                .is_some_and(|last| outcome_sequence <= last)
            {
                return Err(RollingWindowError::OutcomeReceiptSequenceReplayOrReordering);
            }
            if self.last_outcome_receipt_sha256 == Some(outcome_digest) {
                return Err(RollingWindowError::OutcomeReceiptReplay);
            }
            Some((missed, outcome_sequence, outcome_digest))
        } else {
            if missed.is_some() {
                return Err(RollingWindowError::UnexpectedOutcomeForAbstention);
            }
            if outcome_receipt_sequence_id.is_some() || outcome_receipt_sha256.is_some() {
                return Err(RollingWindowError::UnexpectedOutcomeReceiptForAbstention);
            }
            None
        };

        let expected_episode_index = self
            .monitoring_horizon
            .checked_add(1)
            .ok_or(RollingWindowError::CounterOverflow)?;
        let t = decision.episode_index;
        if t != expected_episode_index || decision.decision_sequence_id != t {
            return Err(RollingWindowError::DecisionDoesNotMatchPending);
        }

        let previous_index = usize::try_from(self.monitoring_horizon)
            .map_err(|_| RollingWindowError::HistoryIndexUnavailable)?;
        let previous = *self
            .prefixes
            .get(previous_index)
            .ok_or(RollingWindowError::HistoryIndexUnavailable)?;
        let (next_bounded_count, next_miss_count, next_variation, next_scaled_margin) =
            if let Some((missed, _, _)) = missed_outcome {
                let z = if missed { 1_u64 } else { 0_u64 };
                let prior_mean = if previous.bounded_count == 0 {
                    0.0
                } else {
                    previous.miss_count as f64 / previous.bounded_count as f64
                };
                let variation = previous.predictable_variation + (z as f64 - prior_mean).powi(2);
                if !variation.is_finite() {
                    return Err(RollingWindowError::ArithmeticOverflow);
                }
                (
                    previous
                        .bounded_count
                        .checked_add(1)
                        .ok_or(RollingWindowError::CounterOverflow)?,
                    previous
                        .miss_count
                        .checked_add(z)
                        .ok_or(RollingWindowError::CounterOverflow)?,
                    variation,
                    stitched_two_sided_scaled_margin(
                        t,
                        variation,
                        self.config.confidence_error_probability,
                    )?,
                )
            } else {
                (
                    previous.bounded_count,
                    previous.miss_count,
                    previous.predictable_variation,
                    // Algorithm 2: at an abstention, carry forward t*omega'_t.
                    previous.scaled_margin,
                )
            };

        let prefix_index = usize::try_from(decision.window_start)
            .map_err(|_| RollingWindowError::HistoryIndexUnavailable)?;
        let prefix = *self
            .prefixes
            .get(prefix_index)
            .ok_or(RollingWindowError::HistoryIndexUnavailable)?;
        let window_bounded_count = next_bounded_count
            .checked_sub(prefix.bounded_count)
            .ok_or(RollingWindowError::CertifiedCountUnderflow)?;
        let window_miss_count = next_miss_count
            .checked_sub(prefix.miss_count)
            .ok_or(RollingWindowError::CertifiedCountUnderflow)?;
        let stochastic_upper = window_miss_count as f64
            + next_scaled_margin
            + prefix.scaled_margin;
        if !stochastic_upper.is_finite() {
            return Err(RollingWindowError::ArithmeticOverflow);
        }
        let current_certified_sum =
            (window_bounded_count as f64).min(stochastic_upper);
        let observed_rolling_miscoverage =
            window_miss_count as f64 / decision.window_length as f64;
        let bounded_set_availability_in_window =
            window_bounded_count as f64 / decision.window_length as f64;
        let certified_rolling_miscoverage_upper =
            (current_certified_sum / decision.window_length as f64).min(1.0);

        if !observed_rolling_miscoverage.is_finite()
            || !bounded_set_availability_in_window.is_finite()
            || !certified_rolling_miscoverage_upper.is_finite()
        {
            return Err(RollingWindowError::ArithmeticOverflow);
        }

        let next_horizon = self
            .monitoring_horizon
            .checked_add(1)
            .ok_or(RollingWindowError::CounterOverflow)?;
        let next_abstentions = if bounded {
            self.abstention_count
        } else {
            self.abstention_count
                .checked_add(1)
                .ok_or(RollingWindowError::CounterOverflow)?
        };

        let next_prefix = PrefixSnapshot {
            bounded_count: next_bounded_count,
            miss_count: next_miss_count,
            predictable_variation: next_variation,
            scaled_margin: next_scaled_margin,
        };

        let next_level = self.config.target_miscoverage
            + self.config.learning_rate_gamma
                * (self.config.target_miscoverage * decision.window_length as f64
                    - current_certified_sum
                    - self.config.buffer);
        if !next_level.is_finite() {
            return Err(RollingWindowError::ArithmeticOverflow);
        }

        // Maintain the optional range proof cache only after every episode
        // receipt and numerical check has passed. Cache failure disables the
        // optimization; it never blocks or changes the authoritative history.
        if self.rolling_range_index.is_some() {
            let disable_range_index = match prefix_range_intervals(
                next_prefix,
                next_horizon,
                self.config.window_rule,
                self.config.target_miscoverage,
            ) {
                Ok(Some((r_interval, q_interval))) => match self.rolling_range_index.as_mut() {
                    Some(index) => index.append(r_interval, q_interval).is_err(),
                    None => true,
                },
                Ok(None) if is_full_history_only(self.config.window_rule) => false,
                Ok(None) | Err(_) => true,
            };
            if disable_range_index {
                self.rolling_range_index = None;
            }
        }

        // Commit only after all event, receipt and numerical checks have passed.
        self.monitoring_horizon = next_horizon;
        self.bounded_episode_count = next_bounded_count;
        self.missed_episode_count = next_miss_count;
        self.abstention_count = next_abstentions;
        self.prefixes.push(next_prefix);
        self.last_window_length = decision.window_length;
        self.last_window_start = decision.window_start;
        self.last_certified_rolling_sum = current_certified_sum;
        self.last_episode_id = Some(episode_id);
        self.last_episode_sequence_id = Some(episode_sequence_id);
        self.last_decision_receipt_sequence_id = Some(decision_receipt_sequence_id);
        self.last_decision_receipt_sha256 = Some(decision_receipt_sha256);
        if let Some((_, sequence, digest)) = missed_outcome {
            self.last_outcome_receipt_sequence_id = Some(sequence);
            self.last_outcome_receipt_sha256 = Some(digest);
        }
        self.pending_decision = None;

        Ok(RollingWindowReport {
            monitoring_horizon: self.monitoring_horizon,
            current_window_length: decision.window_length,
            current_window_start: decision.window_start,
            bounded_episode_count: self.bounded_episode_count,
            missed_episode_count: self.missed_episode_count,
            abstention_count: self.abstention_count,
            observed_rolling_miscoverage,
            bounded_set_availability_in_window,
            certified_rolling_miscoverage_upper,
            certified_rolling_miss_sum: current_certified_sum,
            target_miscoverage: self.config.target_miscoverage,
            confidence_level: 1.0 - self.config.confidence_error_probability,
            next_requested_miscoverage_level: next_level,
        })
    }
}

/// The only rule whose start sequence never advances beyond zero.
fn is_full_history_only(rule: RollingWindowRule) -> bool {
    matches!(
        rule,
        RollingWindowRule::Proportional { numerator, denominator }
            if numerator == denominator
    )
}

/// Construct the stored R_s and Q_s intervals for one prefix start. The
/// absolute guard covers conversion/evaluation-order differences between the
/// factored range expressions and the direct reference comparisons. Any
/// overflow or non-finite input disables acceleration and triggers fallback.
fn prefix_range_intervals(
    prefix: PrefixSnapshot,
    start: u64,
    rule: RollingWindowRule,
    alpha: f64,
) -> Result<Option<(OutwardInterval, OutwardInterval)>, RollingWindowError> {
    let Some(first_occurrence) = first_time_with_window_start(rule, start)? else {
        return Ok(None);
    };
    let future_length = first_occurrence
        .checked_sub(start)
        .ok_or(RollingWindowError::ArithmeticOverflow)?;
    let alpha_future_length = alpha * future_length as f64;
    let bounded = prefix.bounded_count as f64;
    let misses = prefix.miss_count as f64;
    let r_value = bounded + alpha_future_length;
    let q_value = misses - prefix.scaled_margin + alpha_future_length;
    if !alpha_future_length.is_finite() || !r_value.is_finite() || !q_value.is_finite() {
        return Err(RollingWindowError::ArithmeticOverflow);
    }
    let r_scale = bounded.abs() + alpha_future_length.abs() + 1.0;
    let q_scale = misses.abs() + prefix.scaled_margin.abs()
        + alpha_future_length.abs() + 1.0;
    Ok(Some((
        guarded_interval(r_value, r_scale)?,
        guarded_interval(q_value, q_scale)?,
    )))
}

/// Outward interval around a computed value, enlarged by a conservative
/// absolute floating-point evaluation guard. The range proof is used only when
/// the resulting intervals separate strictly; boundary/ambiguous cases fall
/// back to the original per-start arithmetic.
fn guarded_interval(
    value: f64,
    magnitude_scale: f64,
) -> Result<OutwardInterval, RollingWindowError> {
    if !value.is_finite() || !magnitude_scale.is_finite() || magnitude_scale < 0.0 {
        return Err(RollingWindowError::ArithmeticOverflow);
    }
    // Covers the short sequences of count conversion, multiplication,
    // addition/subtraction and threshold comparison used by the reference
    // feasibility expression. Subnormal floor protects near-zero quantities.
    let guard = 64.0 * f64::EPSILON * magnitude_scale.max(1.0)
        + 64.0 * f64::from_bits(1);
    if !guard.is_finite() {
        return Err(RollingWindowError::ArithmeticOverflow);
    }
    OutwardInterval::around(value)
        .and_then(|interval| interval.expanded_by_absolute_error(guard))
        .map_err(|_| RollingWindowError::ArithmeticOverflow)
}

/// Evaluate both possible next outcomes for one start using the exact
/// reference expression and operation order.
fn evaluate_start_candidates(
    config: RollingWindowPolicyConfig,
    t: u64,
    previous: PrefixSnapshot,
    start: u64,
    first_occurrence_at: u64,
    prefix: PrefixSnapshot,
) -> Result<Vec<RollingFeasibilityWitness>, RollingWindowError> {
    let smallest_window_length = t.max(first_occurrence_at) - start;
    let allowed_miss_sum = config.target_miscoverage * smallest_window_length as f64;
    let prior_mean = if previous.bounded_count == 0 {
        0.0
    } else {
        previous.miss_count as f64 / previous.bounded_count as f64
    };

    let mut witnesses = Vec::with_capacity(2);
    for candidate_miss in [false, true] {
        let z = if candidate_miss { 1_u64 } else { 0_u64 };
        let bounded_count_with_candidate = previous
            .bounded_count
            .checked_add(1)
            .ok_or(RollingWindowError::CounterOverflow)?;
        let window_bounded_count = bounded_count_with_candidate
            .checked_sub(prefix.bounded_count)
            .ok_or(RollingWindowError::CertifiedCountUnderflow)?;

        let candidate_misses = previous
            .miss_count
            .checked_add(z)
            .ok_or(RollingWindowError::CounterOverflow)?;
        let window_misses = candidate_misses
            .checked_sub(prefix.miss_count)
            .ok_or(RollingWindowError::CertifiedCountUnderflow)?;

        let candidate_variation = previous.predictable_variation
            + (z as f64 - prior_mean).powi(2);
        if !candidate_variation.is_finite() {
            return Err(RollingWindowError::ArithmeticOverflow);
        }
        let candidate_scaled_margin = stitched_two_sided_scaled_margin(
            t,
            candidate_variation,
            config.confidence_error_probability,
        )?;

        let stochastic_upper = window_misses as f64
            + candidate_scaled_margin
            + prefix.scaled_margin;
        if !stochastic_upper.is_finite() {
            return Err(RollingWindowError::ArithmeticOverflow);
        }
        let certified_miss_sum =
            (window_bounded_count as f64).min(stochastic_upper);
        let passed = certified_miss_sum <= allowed_miss_sum;

        witnesses.push(RollingFeasibilityWitness {
            window_start: start,
            first_occurrence_at,
            candidate_miss,
            certified_miss_sum,
            allowed_miss_sum,
            passed,
        });
    }
    Ok(witnesses)
}

/// First episode index u for which the predeclared window rule starts at s.
/// The value may be greater than the current horizon; the rolling feasibility
/// check must include these not-yet-observed future windows.
fn first_time_with_window_start(
    rule: RollingWindowRule,
    start: u64,
) -> Result<Option<u64>, RollingWindowError> {
    rule.validate()?;
    match rule {
        RollingWindowRule::Fixed { length } => {
            if start == 0 {
                Ok(Some(1))
            } else {
                Ok(Some(
                    start
                        .checked_add(length)
                        .ok_or(RollingWindowError::ArithmeticOverflow)?,
                ))
            }
        }
        RollingWindowRule::Proportional {
            numerator,
            denominator,
        } => {
            let remaining = denominator - numerator;
            if remaining == 0 {
                return Ok((start == 0).then_some(1));
            }
            if start == 0 {
                return Ok(Some(1));
            }
            // For k_u=ceil(u*n/d), s_u=u-k_u=floor(u*(d-n)/d).
            // The first u with s_u=s is ceil(s*d/(d-n)).
            let product = (start as u128)
                .checked_mul(denominator as u128)
                .ok_or(RollingWindowError::ArithmeticOverflow)?;
            let divisor = remaining as u128;
            let quotient = product / divisor;
            let remainder = product % divisor;
            let rounded = quotient
                .checked_add(u128::from(remainder != 0))
                .ok_or(RollingWindowError::ArithmeticOverflow)?;
            let first = u64::try_from(rounded)
                .map_err(|_| RollingWindowError::ArithmeticOverflow)?;
            Ok(Some(first.max(1)))
        }
        RollingWindowRule::Polynomial { beta } => {
            if start == 0 {
                return Ok(Some(1));
            }
            // s_u=u-ceil(u^beta) is monotone and advances by zero or one
            // for beta in (1/2,1), so lower-bound search finds its first hit.
            let mut high = 1_u64;
            while rule.window_at(high)?.1 < start {
                high = high
                    .checked_mul(2)
                    .ok_or(RollingWindowError::ArithmeticOverflow)?;
            }
            let mut low = 1_u64;
            while low < high {
                let middle = low + (high - low) / 2;
                if rule.window_at(middle)?.1 < start {
                    low = middle + 1;
                } else {
                    high = middle;
                }
            }
            Ok((rule.window_at(low)?.1 == start).then_some(low))
        }
    }
}

fn stitched_two_sided_scaled_margin(
    horizon: u64,
    predictable_variation: f64,
    overall_delta: f64,
) -> Result<f64, RollingWindowError> {
    if horizon == 0
        || !predictable_variation.is_finite()
        || predictable_variation < 0.0
        || !overall_delta.is_finite()
        || overall_delta <= 0.0
        || overall_delta >= 1.0
    {
        return Err(RollingWindowError::ArithmeticOverflow);
    }

    // Lemma 4.1 uses a two-sided confidence sequence at error level delta;
    // Fang (2026) obtains it by applying the one-sided margin at delta/2.
    let delta_one_sided = overall_delta / 2.0;
    let variation = predictable_variation.max(1.0);
    let twice_variation = 2.0 * variation;
    let log_log = twice_variation.ln().ln().max(0.0);
    let log_delta = (STITCHED_LOG_CONSTANT / delta_one_sided).ln();
    let inner = log_log + LOG_LOG_COEFFICIENT * log_delta;
    if !twice_variation.is_finite()
        || !log_log.is_finite()
        || !log_delta.is_finite()
        || !inner.is_finite()
        || log_delta <= 0.0
        || inner <= 0.0
    {
        return Err(RollingWindowError::ArithmeticOverflow);
    }

    let scaled = SQRT_MARGIN_COEFFICIENT * (variation * inner).sqrt()
        + LOG_LOG_MARGIN_COEFFICIENT * log_log
        + LOG_MARGIN_COEFFICIENT * log_delta;
    if !scaled.is_finite() || scaled < 0.0 {
        return Err(RollingWindowError::ArithmeticOverflow);
    }
    Ok(scaled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(rule: RollingWindowRule) -> RollingWindowPolicyConfig {
        RollingWindowPolicyConfig {
            target_miscoverage: 0.5,
            confidence_error_probability: 0.5,
            learning_rate_gamma: 0.01,
            buffer: 0.0,
            window_rule: rule,
            maximum_feasibility_starts: 128,
        }
    }

    fn digest(sequence: u64, domain: u8) -> [u8; SHA256_BYTES] {
        let mut result = [domain; SHA256_BYTES];
        result[..8].copy_from_slice(&sequence.to_le_bytes());
        result
    }

    fn complete(
        policy: &mut AnytimeRollingCoveragePolicy,
        decision: &RollingWindowDecision,
        missed: Option<bool>,
    ) -> RollingWindowReport {
        let t = decision.episode_index;
        policy
            .complete_episode(
                decision,
                t,
                t,
                t,
                digest(t, 0xA1),
                missed,
                missed.map(|_| t),
                missed.map(|_| digest(t, 0xB2)),
            )
            .unwrap()
    }

    #[test]
    fn fixed_proportional_and_polynomial_window_rules_are_explicit() {
        assert_eq!(
            RollingWindowRule::Fixed { length: 5 }.window_at(3).unwrap(),
            (3, 0)
        );
        assert_eq!(
            RollingWindowRule::Fixed { length: 5 }.window_at(9).unwrap(),
            (5, 4)
        );
        assert_eq!(
            RollingWindowRule::Proportional {
                numerator: 1,
                denominator: 2
            }
            .window_at(9)
            .unwrap(),
            (5, 4)
        );
        assert_eq!(
            RollingWindowRule::Polynomial { beta: 0.75 }
                .window_at(16)
                .unwrap(),
            (8, 8)
        );
        assert!(matches!(
            RollingWindowRule::Polynomial { beta: 0.5 }.validate(),
            Err(RollingWindowError::InvalidWindowRule)
        ));
    }

    #[test]
    fn first_occurrence_lookup_includes_future_starts() {
        assert_eq!(
            first_time_with_window_start(RollingWindowRule::Fixed { length: 2 }, 1).unwrap(),
            Some(3)
        );
        assert_eq!(
            first_time_with_window_start(
                RollingWindowRule::Proportional {
                    numerator: 1,
                    denominator: 2
                },
                2
            )
            .unwrap(),
            Some(4)
        );
        assert_eq!(
            first_time_with_window_start(
                RollingWindowRule::Polynomial { beta: 0.75 },
                8
            )
            .unwrap(),
            Some(16)
        );
    }

    #[test]
    fn one_episode_window_abstains_if_either_possible_outcome_breaks_feasibility() {
        let mut policy = AnytimeRollingCoveragePolicy::new(config(RollingWindowRule::Fixed {
            length: 2,
        }))
        .unwrap();
        let first = policy.decide_next_episode().unwrap();
        assert!(first.is_abstention());
        assert_eq!(
            first.disposition,
            RollingWindowDisposition::Abstain(
                RollingAbstentionReason::ProspectiveFeasibilityFailed
            )
        );
        assert_eq!(first.feasibility_witnesses.len(), 2);
        assert!(first.feasibility_witnesses.iter().any(|w| !w.passed));
        complete(&mut policy, &first, None);

        let second = policy.decide_next_episode().unwrap();
        assert!(second.is_bounded_set_requested());
        assert_eq!(second.window_start, 0);
        assert_eq!(second.window_length, 2);
        assert_eq!(second.feasibility_witnesses.len(), 4);
        assert!(second.feasibility_witnesses.iter().all(|w| w.passed));
        assert!(second.feasibility_witnesses.iter().any(|w| {
            w.window_start == 1 && w.first_occurrence_at == 3
        }));
    }

    #[test]
    fn prospective_check_records_both_candidate_outcomes_for_every_window() {
        let mut policy = AnytimeRollingCoveragePolicy::new(config(
            RollingWindowRule::Proportional {
                numerator: 1,
                denominator: 1,
            },
        ))
        .unwrap();
        for _ in 0..3 {
            let decision = policy.decide_next_episode().unwrap();
            let outcome = decision.is_bounded_set_requested().then_some(false);
            complete(&mut policy, &decision, outcome);
        }
        let decision = policy.decide_next_episode().unwrap();
        let starts = decision
            .feasibility_witnesses
            .iter()
            .map(|w| w.window_start)
            .collect::<Vec<_>>();
        let starts_distinct = starts.iter().copied().collect::<std::collections::BTreeSet<_>>();
        assert_eq!(starts.len(), starts_distinct.len() * 2);
        assert!(decision.feasibility_witnesses.iter().any(|w| !w.candidate_miss));
        assert!(decision.feasibility_witnesses.iter().any(|w| w.candidate_miss));
    }

    #[test]
    fn abstention_does_not_change_prefix_counts_or_scaled_margin() {
        let mut forced_abstention = config(RollingWindowRule::Fixed { length: 3 });
        forced_abstention.buffer = 1_000.0;
        let mut policy = AnytimeRollingCoveragePolicy::new(forced_abstention).unwrap();
        for t in 1..=4 {
            let decision = policy.decide_next_episode().unwrap();
            complete(&mut policy, &decision, None);
            assert_eq!(policy.bounded_episode_count(), 0);
            assert_eq!(policy.missed_episode_count(), 0);
            assert_eq!(policy.abstention_count(), t);
            assert_eq!(policy.prefixes[t as usize].scaled_margin, 0.0);
        }
    }

    #[test]
    fn bounded_episode_uses_sealed_outcome_and_updates_window_audit() {
        let mut policy = AnytimeRollingCoveragePolicy::new(config(RollingWindowRule::Fixed {
            length: 2,
        }))
        .unwrap();
        let first = policy.decide_next_episode().unwrap();
        complete(&mut policy, &first, None);
        let second = policy.decide_next_episode().unwrap();
        assert!(second.is_bounded_set_requested());
        let report = complete(&mut policy, &second, Some(true));
        assert_eq!(report.monitoring_horizon, 2);
        assert_eq!(report.bounded_episode_count, 1);
        assert_eq!(report.missed_episode_count, 1);
        assert_eq!(report.abstention_count, 1);
        assert_eq!(report.current_window_start, 0);
        assert_eq!(report.current_window_length, 2);
        assert_eq!(report.observed_rolling_miscoverage, 0.5);
        assert_eq!(report.bounded_set_availability_in_window, 0.5);
        assert!(report.certified_rolling_miscoverage_upper >= 0.5);
    }

    #[test]
    fn invalid_or_replayed_truth_receipts_do_not_mutate_history() {
        let mut policy = AnytimeRollingCoveragePolicy::new(config(RollingWindowRule::Fixed {
            length: 2,
        }))
        .unwrap();
        let decision = policy.decide_next_episode().unwrap();
        assert_eq!(
            policy.complete_episode(
                &decision,
                1,
                1,
                1,
                [0; SHA256_BYTES],
                None,
                None,
                None,
            ),
            Err(RollingWindowError::MissingDecisionReceipt)
        );
        assert_eq!(policy.monitoring_horizon, 0);

        assert_eq!(
            policy.complete_episode(
                &decision,
                1,
                1,
                1,
                digest(1, 0xA1),
                Some(false),
                Some(1),
                Some(digest(1, 0xB2)),
            ),
            Err(RollingWindowError::UnexpectedOutcomeForAbstention)
        );
        assert_eq!(policy.monitoring_horizon, 0);

        let report = complete(&mut policy, &decision, None);
        assert_eq!(report.monitoring_horizon, 1);
    }

    fn range_ready_policy(
        mut cfg: RollingWindowPolicyConfig,
    ) -> AnytimeRollingCoveragePolicy {
        cfg.target_miscoverage = 0.9;
        cfg.confidence_error_probability = 0.99;
        let mut policy = AnytimeRollingCoveragePolicy::new(cfg).unwrap();
        policy.monitoring_horizon = 39;
        policy.prefixes = vec![PrefixSnapshot::zero(); 40];

        let mut index = ConservativeRollingRangeIndex::new();
        for start in 0..40_u64 {
            let (r_interval, q_interval) = prefix_range_intervals(
                PrefixSnapshot::zero(),
                start,
                policy.config.window_rule,
                policy.config.target_miscoverage,
            )
            .unwrap()
            .unwrap();
            index.append(r_interval, q_interval).unwrap();
        }
        policy.rolling_range_index = Some(index);
        policy
    }

    #[test]
    fn accelerated_range_proof_matches_reference_and_emits_compact_receipts() {
        let mut cfg = config(RollingWindowRule::Proportional {
            numerator: 1,
            denominator: 2,
        });
        cfg.maximum_feasibility_starts = 128;
        let mut reference = range_ready_policy(cfg);
        let mut accelerated = range_ready_policy(cfg);

        let reference_decision = reference.decide_next_episode().unwrap();
        let accelerated_decision =
            accelerated.decide_next_episode_accelerated().unwrap();

        assert_eq!(
            accelerated_decision.disposition,
            reference_decision.disposition
        );
        assert!(reference_decision.is_bounded_set_requested());
        assert_eq!(reference_decision.feasibility_witnesses.len(), 40);
        assert_eq!(accelerated_decision.feasibility_witnesses.len(), 2);
        assert_eq!(accelerated_decision.range_proof_witnesses.len(), 2);
        assert!(accelerated_decision
            .range_proof_witnesses
            .iter()
            .all(|witness| witness.q_minimum.is_none()));

        let reference_report = complete(&mut reference, &reference_decision, Some(false));
        let accelerated_report = complete(&mut accelerated, &accelerated_decision, Some(false));
        assert_eq!(accelerated_report, reference_report);
    }

    #[test]
    fn accelerated_range_proof_can_avoid_the_exact_scan_budget() {
        let mut cfg = config(RollingWindowRule::Proportional {
            numerator: 1,
            denominator: 2,
        });
        cfg.maximum_feasibility_starts = 1;
        let mut accelerated = range_ready_policy(cfg);
        let decision = accelerated.decide_next_episode_accelerated().unwrap();

        assert!(decision.is_bounded_set_requested());
        assert_eq!(decision.range_proof_witnesses.len(), 2);
        assert_eq!(decision.feasibility_witnesses.len(), 2);

        let mut reference = range_ready_policy(cfg);
        let reference_decision = reference.decide_next_episode().unwrap();
        assert_eq!(
            reference_decision.disposition,
            RollingWindowDisposition::Abstain(
                RollingAbstentionReason::FeasibilityBudgetExceeded
            )
        );
    }

    #[test]
    fn ambiguous_range_proof_falls_back_to_the_complete_reference_scan() {
        let mut cfg = config(RollingWindowRule::Proportional {
            numerator: 1,
            denominator: 2,
        });
        cfg.maximum_feasibility_starts = 128;
        let mut policy = range_ready_policy(cfg);

        let mut ambiguous = ConservativeRollingRangeIndex::new();
        let low_r = OutwardInterval::around(0.0).unwrap();
        let low_q = OutwardInterval::around(-100.0).unwrap();
        for _ in 0..40 {
            ambiguous.append(low_r, low_q).unwrap();
        }
        policy.rolling_range_index = Some(ambiguous);

        let decision = policy.decide_next_episode_accelerated().unwrap();
        assert!(decision.is_bounded_set_requested());
        assert_eq!(decision.range_proof_witnesses.len(), 0);
        assert_eq!(decision.feasibility_witnesses.len(), 40);
    }

    #[test]
    fn feasibility_budget_exhaustion_abstains_instead_of_skipping_windows() {
        let mut cfg = config(RollingWindowRule::Proportional {
            numerator: 1,
            denominator: 2,
        });
        cfg.maximum_feasibility_starts = 1;
        let mut policy = AnytimeRollingCoveragePolicy::new(cfg).unwrap();
        for _ in 0..2 {
            let decision = policy.decide_next_episode().unwrap();
            complete(&mut policy, &decision, None);
        }
        let third = policy.decide_next_episode().unwrap();
        assert_eq!(
            third.disposition,
            RollingWindowDisposition::Abstain(
                RollingAbstentionReason::FeasibilityBudgetExceeded
            )
        );
    }
}
