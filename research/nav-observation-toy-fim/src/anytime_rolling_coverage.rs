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
    /// Hard compute budget. Exceeding it forces abstention; it never truncates
    /// the prospective feasibility check and then issues a bounded set.
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
    pub feasibility_witnesses: Vec<RollingFeasibilityWitness>,
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
    /// The feasibility loop examines every observed first-seen window start in
    /// [s_t, t), and both possible outcomes z=0 and z=1. If the work budget would
    /// be exceeded, this method abstains rather than skipping checks.
    pub fn decide_next_episode(&mut self) -> Result<RollingWindowDecision, RollingWindowError> {
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
        let mut disposition = if eta <= 0.0 {
            RollingWindowDisposition::Abstain(
                RollingAbstentionReason::NonPositiveMiscoverageLevel,
            )
        } else {
            RollingWindowDisposition::BoundedSetRequested
        };

        if eta > 0.0 {
            // The feasibility check covers all future rolling windows
            // that may contain episode t. For a predeclared monotone window
            // rule, every integer start in [s_t,t) occurs eventually, except
            // the full-history proportional rule (theta=1), which always starts
            // at zero. The number of starts is therefore known without scanning
            // an unbounded future horizon.
            let full_history_only = matches!(
                self.config.window_rule,
                RollingWindowRule::Proportional {
                    numerator,
                    denominator
                } if numerator == denominator
            );
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

                let prior_mean = if previous.bounded_count == 0 {
                    0.0
                } else {
                    previous.miss_count as f64 / previous.bounded_count as f64
                };
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
                    let prefix = *self
                        .prefixes
                        .get(start as usize)
                        .ok_or(RollingWindowError::HistoryIndexUnavailable)?;
                    let smallest_window_length = t.max(first_occurrence_at) - start;
                    let allowed_miss_sum =
                        self.config.target_miscoverage * smallest_window_length as f64;

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
                            self.config.confidence_error_probability,
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
                }

                if witnesses.iter().any(|witness| !witness.passed) {
                    disposition = RollingWindowDisposition::Abstain(
                        RollingAbstentionReason::ProspectiveFeasibilityFailed,
                    );
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
        };

        self.pending_decision = Some(decision.clone());
        Ok(decision)
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

        let prefix = *self
            .prefixes
            .get(decision.window_start as usize)
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
            complete(&mut policy, &decision, None);
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
