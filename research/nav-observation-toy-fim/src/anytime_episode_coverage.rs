//! Anytime-valid running miscoverage audit over completed trajectory episodes.
//!
//! This is an evaluator-side confidence sequence. It does not adapt the
//! predictor or authorize a live decision. Each monitoring step is one
//! completed mission/trajectory episode, and its outcome is revealed only
//! after the prediction and episode are sealed.
//!
//! The stitched empirical-Bernstein margin follows Fang (2026), arXiv:2611.20905,
//! for a time-uniform upper confidence sequence on the true running average
//! miscoverage rate. The filtration/ordering and bounded miss-indicator
//! contract must be respected by the caller.

use std::fmt;

const SHA256_BYTES: usize = 32;
const LOG_LOG_COEFFICIENT: f64 = 0.72;
const SQRT_MARGIN_COEFFICIENT: f64 = 1.7;
const LOG_LOG_MARGIN_COEFFICIENT: f64 = 3.4;
const LOG_MARGIN_COEFFICIENT: f64 = 2.45;
const STITCHED_LOG_CONSTANT: f64 = 5.19;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnytimeEpisodeCoverageConfig {
    /// Target running-average miscoverage probability alpha.
    pub target_miscoverage: f64,
    /// Time-uniform confidence-sequence error probability delta.
    pub confidence_error_probability: f64,
    /// Operational availability floor; not part of the statistical theorem.
    pub minimum_qualified_availability: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EpisodeCoverageAuditEvent {
    /// Strictly increasing source-ledger mission/trajectory identity.
    pub episode_id: u64,
    /// Strictly increasing source-ledger sequence for completed episodes.
    pub episode_sequence_id: u64,
    /// Strictly increasing decision-receipt sequence.
    pub decision_receipt_sequence_id: u64,
    /// Digest binds the decision, episode, sequence and frozen policy.
    pub decision_receipt_sha256: [u8; SHA256_BYTES],
    /// True when a bounded, qualified episode tube was issued.
    pub bounded_set_issued: bool,
    /// Some(true) means the sealed full-episode tube missed.
    pub missed: Option<bool>,
    /// Strictly increasing sequence of the sealed truth/evaluation result.
    pub outcome_receipt_sequence_id: Option<u64>,
    /// Digest binds the outcome to the episode and outcome receipt sequence.
    pub outcome_receipt_sha256: Option<[u8; SHA256_BYTES]>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnytimeEpisodeCoverageReport {
    pub monitoring_horizon: u64,
    pub bounded_episode_count: u64,
    pub abstention_count: u64,
    pub missed_episode_count: u64,
    pub observed_running_miscoverage: f64,
    pub bounded_episode_miscoverage: Option<f64>,
    pub qualified_availability: f64,
    /// The paper's stitched empirical-Bernstein margin omega_t.
    pub anytime_margin: f64,
    /// Upper confidence sequence for true running-average miscoverage.
    pub true_running_miscoverage_upper: f64,
    pub confidence_level: f64,
    pub target_miscoverage: f64,
    pub running_miscoverage_target_met: bool,
    pub minimum_qualified_availability: f64,
    pub availability_target_met: bool,
    pub operational_targets_met: bool,
    pub predictable_variation: f64,
    /// Certified upper miss sum: S_t + t * omega_t.
    pub certified_upper_miss_sum: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnytimeEpisodeCoverageError {
    InvalidConfiguration,
    InvalidEpisodeIdentity,
    EpisodeIdentityReplayOrReordering,
    EpisodeSequenceReplayOrReordering,
    DecisionReceiptSequenceReplayOrReordering,
    OutcomeReceiptSequenceReplayOrReordering,
    MissingDecisionReceipt,
    DecisionReceiptReplay,
    MissingOutcomeForBoundedEpisode,
    UnexpectedOutcomeForAbstention,
    MissingOutcomeReceipt,
    UnexpectedOutcomeReceiptForAbstention,
    OutcomeReceiptReplay,
    CounterOverflow,
    ArithmeticOverflow,
}

impl fmt::Display for AnytimeEpisodeCoverageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration => write!(f, "target miscoverage and confidence error must be in (0, 1), and availability target in [0, 1]"),
            Self::InvalidEpisodeIdentity => write!(f, "episode and sequence identities must be nonzero"),
            Self::EpisodeIdentityReplayOrReordering => write!(f, "episode identity is not strictly increasing"),
            Self::EpisodeSequenceReplayOrReordering => write!(f, "episode sequence is not strictly increasing"),
            Self::DecisionReceiptSequenceReplayOrReordering => write!(f, "decision receipt sequence is not strictly increasing"),
            Self::OutcomeReceiptSequenceReplayOrReordering => write!(f, "outcome receipt sequence is not strictly increasing"),
            Self::MissingDecisionReceipt => write!(f, "decision receipt digest is missing"),
            Self::DecisionReceiptReplay => write!(f, "decision receipt digest was replayed"),
            Self::MissingOutcomeForBoundedEpisode => write!(f, "a bounded episode requires a sealed miss/hit outcome"),
            Self::UnexpectedOutcomeForAbstention => write!(f, "an abstention must not contain a bounded-set miss outcome"),
            Self::MissingOutcomeReceipt => write!(f, "a bounded episode requires a sealed outcome receipt"),
            Self::UnexpectedOutcomeReceiptForAbstention => write!(f, "an abstention must not claim a bounded-set outcome receipt"),
            Self::OutcomeReceiptReplay => write!(f, "truth/evaluation receipt digest was replayed"),
            Self::CounterOverflow => write!(f, "episode audit counter overflowed"),
            Self::ArithmeticOverflow => write!(f, "confidence-sequence arithmetic overflowed"),
        }
    }
}

impl std::error::Error for AnytimeEpisodeCoverageError {}

#[derive(Clone, Debug, PartialEq)]
pub struct AnytimeEpisodeCoverageAudit {
    config: AnytimeEpisodeCoverageConfig,
    monitoring_horizon: u64,
    bounded_episode_count: u64,
    abstention_count: u64,
    missed_episode_count: u64,
    predictable_variation: f64,
    last_episode_id: Option<u64>,
    last_episode_sequence_id: Option<u64>,
    last_decision_receipt_sequence_id: Option<u64>,
    last_decision_receipt_sha256: Option<[u8; SHA256_BYTES]>,
    last_outcome_receipt_sequence_id: Option<u64>,
    last_outcome_receipt_sha256: Option<[u8; SHA256_BYTES]>,
}

impl AnytimeEpisodeCoverageAudit {
    pub fn new(
        config: AnytimeEpisodeCoverageConfig,
    ) -> Result<Self, AnytimeEpisodeCoverageError> {
        if !config.target_miscoverage.is_finite()
            || config.target_miscoverage <= 0.0
            || config.target_miscoverage >= 1.0
            || !config.confidence_error_probability.is_finite()
            || config.confidence_error_probability <= 0.0
            || config.confidence_error_probability >= 1.0
            || !config.minimum_qualified_availability.is_finite()
            || !(0.0..=1.0).contains(&config.minimum_qualified_availability)
        {
            return Err(AnytimeEpisodeCoverageError::InvalidConfiguration);
        }

        let confidence_ratio = STITCHED_LOG_CONSTANT / config.confidence_error_probability;
        if !confidence_ratio.is_finite() || confidence_ratio <= 1.0 {
            return Err(AnytimeEpisodeCoverageError::InvalidConfiguration);
        }

        Ok(Self {
            config,
            monitoring_horizon: 0,
            bounded_episode_count: 0,
            abstention_count: 0,
            missed_episode_count: 0,
            predictable_variation: 0.0,
            last_episode_id: None,
            last_episode_sequence_id: None,
            last_decision_receipt_sequence_id: None,
            last_decision_receipt_sha256: None,
            last_outcome_receipt_sequence_id: None,
            last_outcome_receipt_sha256: None,
        })
    }

    pub fn config(&self) -> AnytimeEpisodeCoverageConfig {
        self.config
    }

    pub fn report(&self) -> Result<AnytimeEpisodeCoverageReport, AnytimeEpisodeCoverageError> {
        if self.monitoring_horizon == 0 {
            return Ok(AnytimeEpisodeCoverageReport {
                monitoring_horizon: 0,
                bounded_episode_count: 0,
                abstention_count: 0,
                missed_episode_count: 0,
                observed_running_miscoverage: 0.0,
                bounded_episode_miscoverage: None,
                qualified_availability: 0.0,
                // No time-uniform claim is made before the first completed episode.
                // One is the trivial upper bound for an unobserved running average.
                anytime_margin: 1.0,
                true_running_miscoverage_upper: 1.0,
                confidence_level: 1.0 - self.config.confidence_error_probability,
                target_miscoverage: self.config.target_miscoverage,
                running_miscoverage_target_met: false,
                minimum_qualified_availability: self.config.minimum_qualified_availability,
                availability_target_met: self.config.minimum_qualified_availability == 0.0,
                operational_targets_met: false,
                predictable_variation: self.predictable_variation,
                certified_upper_miss_sum: 0.0,
            });
        }

        let horizon = self.monitoring_horizon as f64;
        let bounded = self.bounded_episode_count as f64;
        let misses = self.missed_episode_count as f64;
        let observed_running_miscoverage = misses / horizon;
        let bounded_episode_miscoverage = if self.bounded_episode_count == 0 {
            None
        } else {
            Some(misses / bounded)
        };
        let qualified_availability = bounded / horizon;
        let anytime_margin = self.stitched_empirical_bernstein_margin()?;
        let certified_upper_miss_sum = misses + horizon * anytime_margin;
        if !certified_upper_miss_sum.is_finite() {
            return Err(AnytimeEpisodeCoverageError::ArithmeticOverflow);
        }
        let true_running_miscoverage_upper =
            (observed_running_miscoverage + anytime_margin).clamp(0.0, 1.0);
        let running_miscoverage_target_met =
            true_running_miscoverage_upper <= self.config.target_miscoverage;
        let availability_target_met =
            qualified_availability >= self.config.minimum_qualified_availability;

        Ok(AnytimeEpisodeCoverageReport {
            monitoring_horizon: self.monitoring_horizon,
            bounded_episode_count: self.bounded_episode_count,
            abstention_count: self.abstention_count,
            missed_episode_count: self.missed_episode_count,
            observed_running_miscoverage,
            bounded_episode_miscoverage,
            qualified_availability,
            anytime_margin,
            true_running_miscoverage_upper,
            confidence_level: 1.0 - self.config.confidence_error_probability,
            target_miscoverage: self.config.target_miscoverage,
            running_miscoverage_target_met,
            minimum_qualified_availability: self.config.minimum_qualified_availability,
            availability_target_met,
            operational_targets_met: running_miscoverage_target_met && availability_target_met,
            predictable_variation: self.predictable_variation,
            certified_upper_miss_sum,
        })
    }

    /// Record one fully adjudicated episode outcome.
    ///
    /// The caller must decide whether a bounded tube is issued using only prior
    /// information. A bounded episode's miss/hit outcome is supplied only after
    /// its complete truth trajectory has been sealed and independently scored.
    pub fn observe_completed_episode(
        &mut self,
        event: EpisodeCoverageAuditEvent,
    ) -> Result<AnytimeEpisodeCoverageReport, AnytimeEpisodeCoverageError> {
        if event.episode_id == 0
            || event.episode_sequence_id == 0
            || event.decision_receipt_sequence_id == 0
        {
            return Err(AnytimeEpisodeCoverageError::InvalidEpisodeIdentity);
        }
        if self.last_episode_id.is_some_and(|last| event.episode_id <= last) {
            return Err(AnytimeEpisodeCoverageError::EpisodeIdentityReplayOrReordering);
        }
        if self
            .last_episode_sequence_id
            .is_some_and(|last| event.episode_sequence_id <= last)
        {
            return Err(AnytimeEpisodeCoverageError::EpisodeSequenceReplayOrReordering);
        }
        if self
            .last_decision_receipt_sequence_id
            .is_some_and(|last| event.decision_receipt_sequence_id <= last)
        {
            return Err(AnytimeEpisodeCoverageError::DecisionReceiptSequenceReplayOrReordering);
        }
        if event.decision_receipt_sha256.iter().all(|byte| *byte == 0) {
            return Err(AnytimeEpisodeCoverageError::MissingDecisionReceipt);
        }
        if self.last_decision_receipt_sha256 == Some(event.decision_receipt_sha256) {
            return Err(AnytimeEpisodeCoverageError::DecisionReceiptReplay);
        }

        let miss_indicator = if event.bounded_set_issued {
            let missed = event
                .missed
                .ok_or(AnytimeEpisodeCoverageError::MissingOutcomeForBoundedEpisode)?;
            let outcome_sequence = event
                .outcome_receipt_sequence_id
                .ok_or(AnytimeEpisodeCoverageError::MissingOutcomeReceipt)?;
            if outcome_sequence == 0 {
                return Err(AnytimeEpisodeCoverageError::MissingOutcomeReceipt);
            }
            if self
                .last_outcome_receipt_sequence_id
                .is_some_and(|last| outcome_sequence <= last)
            {
                return Err(AnytimeEpisodeCoverageError::OutcomeReceiptSequenceReplayOrReordering);
            }
            let outcome_receipt = event
                .outcome_receipt_sha256
                .ok_or(AnytimeEpisodeCoverageError::MissingOutcomeReceipt)?;
            if outcome_receipt.iter().all(|byte| *byte == 0) {
                return Err(AnytimeEpisodeCoverageError::MissingOutcomeReceipt);
            }
            if self.last_outcome_receipt_sha256 == Some(outcome_receipt) {
                return Err(AnytimeEpisodeCoverageError::OutcomeReceiptReplay);
            }
            Some((missed, outcome_sequence, outcome_receipt))
        } else {
            if event.outcome_receipt_sequence_id.is_some() {
                return Err(AnytimeEpisodeCoverageError::UnexpectedOutcomeReceiptForAbstention);
            }
            if event.missed.is_some() {
                return Err(AnytimeEpisodeCoverageError::UnexpectedOutcomeForAbstention);
            }
            if event.outcome_receipt_sha256.is_some() {
                return Err(AnytimeEpisodeCoverageError::UnexpectedOutcomeReceiptForAbstention);
            }
            None
        };

        let next_horizon = self
            .monitoring_horizon
            .checked_add(1)
            .ok_or(AnytimeEpisodeCoverageError::CounterOverflow)?;

        let mut next_bounded = self.bounded_episode_count;
        let mut next_abstentions = self.abstention_count;
        let mut next_misses = self.missed_episode_count;
        let mut next_variation = self.predictable_variation;
        let mut next_last_outcome_sequence = self.last_outcome_receipt_sequence_id;
        let mut next_last_outcome = self.last_outcome_receipt_sha256;

        if let Some((missed, outcome_sequence, outcome_receipt)) = miss_indicator {
            let previous_bounded_mean = if self.bounded_episode_count == 0 {
                0.0
            } else {
                self.missed_episode_count as f64 / self.bounded_episode_count as f64
            };
            let z = if missed { 1.0 } else { 0.0 };
            let deviation = z - previous_bounded_mean;
            next_variation += deviation * deviation;
            next_bounded = next_bounded
                .checked_add(1)
                .ok_or(AnytimeEpisodeCoverageError::CounterOverflow)?;
            if missed {
                next_misses = next_misses
                    .checked_add(1)
                    .ok_or(AnytimeEpisodeCoverageError::CounterOverflow)?;
            }
            next_last_outcome_sequence = Some(outcome_sequence);
            next_last_outcome = Some(outcome_receipt);
        } else {
            next_abstentions = next_abstentions
                .checked_add(1)
                .ok_or(AnytimeEpisodeCoverageError::CounterOverflow)?;
        }

        if !next_variation.is_finite() {
            return Err(AnytimeEpisodeCoverageError::ArithmeticOverflow);
        }

        self.monitoring_horizon = next_horizon;
        self.bounded_episode_count = next_bounded;
        self.abstention_count = next_abstentions;
        self.missed_episode_count = next_misses;
        self.predictable_variation = next_variation;
        self.last_episode_id = Some(event.episode_id);
        self.last_episode_sequence_id = Some(event.episode_sequence_id);
        self.last_decision_receipt_sequence_id = Some(event.decision_receipt_sequence_id);
        self.last_decision_receipt_sha256 = Some(event.decision_receipt_sha256);
        self.last_outcome_receipt_sequence_id = next_last_outcome_sequence;
        self.last_outcome_receipt_sha256 = next_last_outcome;

        self.report()
    }

    fn stitched_empirical_bernstein_margin(
        &self,
    ) -> Result<f64, AnytimeEpisodeCoverageError> {
        if self.monitoring_horizon == 0 {
            return Err(AnytimeEpisodeCoverageError::ArithmeticOverflow);
        }
        // Before the first bounded set, every outcome is a deterministic
        // abstention (Z_t = nu_t = 0) and Algorithm 1 keeps t*omega_t at zero.
        if self.bounded_episode_count == 0 {
            return Ok(0.0);
        }

        let variation = self.predictable_variation.max(1.0);
        let twice_variation = 2.0 * variation;
        if !twice_variation.is_finite() || twice_variation <= 1.0 {
            return Err(AnytimeEpisodeCoverageError::ArithmeticOverflow);
        }
        let log_log = twice_variation.ln().ln().max(0.0);
        let log_delta = (STITCHED_LOG_CONSTANT / self.config.confidence_error_probability).ln();
        let confidence_term = LOG_LOG_COEFFICIENT * log_delta;
        let inner = log_log + confidence_term;
        if !inner.is_finite() || inner <= 0.0 || !log_delta.is_finite() || log_delta <= 0.0 {
            return Err(AnytimeEpisodeCoverageError::ArithmeticOverflow);
        }

        // Exact three-term stitched empirical-Bernstein margin from Fang (2026),
        // Equation (29), not the simplified leading square-root term alone.
        let numerator = SQRT_MARGIN_COEFFICIENT * (variation * inner).sqrt()
            + LOG_LOG_MARGIN_COEFFICIENT * log_log
            + LOG_MARGIN_COEFFICIENT * log_delta;
        let margin = numerator / self.monitoring_horizon as f64;
        margin
            .is_finite()
            .then_some(margin)
            .ok_or(AnytimeEpisodeCoverageError::ArithmeticOverflow)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(sequence: u64, domain: u8) -> [u8; SHA256_BYTES] {
        let mut digest = [domain; SHA256_BYTES];
        digest[..8].copy_from_slice(&sequence.to_le_bytes());
        digest
    }

    fn config() -> AnytimeEpisodeCoverageConfig {
        AnytimeEpisodeCoverageConfig {
            target_miscoverage: 0.05,
            confidence_error_probability: 0.05,
            minimum_qualified_availability: 0.80,
        }
    }

    fn audit() -> AnytimeEpisodeCoverageAudit {
        AnytimeEpisodeCoverageAudit::new(config()).unwrap()
    }

    fn event(
        episode_id: u64,
        sequence: u64,
        bounded: bool,
        missed: Option<bool>,
    ) -> EpisodeCoverageAuditEvent {
        EpisodeCoverageAuditEvent {
            episode_id,
            episode_sequence_id: sequence,
            decision_receipt_sequence_id: sequence,
            decision_receipt_sha256: digest(sequence, 0xA1),
            bounded_set_issued: bounded,
            missed,
            outcome_receipt_sequence_id: if bounded { Some(sequence) } else { None },
            outcome_receipt_sha256: if bounded {
                Some(digest(sequence, 0xB2))
            } else {
                None
            },
        }
    }

    #[test]
    fn records_hits_misses_and_separate_availability_denominators() {
        let mut audit = audit();
        audit
            .observe_completed_episode(event(1, 1, true, Some(false)))
            .unwrap();
        let report = audit
            .observe_completed_episode(event(2, 2, true, Some(true)))
            .unwrap();

        assert_eq!(report.monitoring_horizon, 2);
        assert_eq!(report.bounded_episode_count, 2);
        assert_eq!(report.abstention_count, 0);
        assert_eq!(report.missed_episode_count, 1);
        assert_eq!(report.observed_running_miscoverage, 0.5);
        assert_eq!(report.bounded_episode_miscoverage, Some(0.5));
        assert_eq!(report.qualified_availability, 1.0);
        assert!(!report.running_miscoverage_target_met);
    }

    #[test]
    fn abstentions_contribute_zero_miscoverage_but_reduce_availability() {
        let mut audit = audit();
        let hit = audit
            .observe_completed_episode(event(1, 1, true, Some(false)))
            .unwrap();
        let abstain = audit
            .observe_completed_episode(event(2, 2, false, None))
            .unwrap();

        assert_eq!(hit.bounded_episode_count, 1);
        assert_eq!(abstain.bounded_episode_count, 1);
        assert_eq!(abstain.abstention_count, 1);
        assert_eq!(abstain.monitoring_horizon, 2);
        assert_eq!(abstain.observed_running_miscoverage, 0.0);
        assert_eq!(abstain.qualified_availability, 0.5);
        assert!(!abstain.availability_target_met);
        assert!(!abstain.operational_targets_met);
    }

    #[test]
    fn pre_first_bounded_set_abstentions_do_not_inflate_certified_margin() {
        let mut audit = audit();
        for sequence in 1..=25 {
            let report = audit
                .observe_completed_episode(event(sequence, sequence, false, None))
                .unwrap();
            assert_eq!(report.anytime_margin, 0.0);
            assert_eq!(report.certified_upper_miss_sum, 0.0);
            assert_eq!(report.true_running_miscoverage_upper, 0.0);
        }
    }

    #[test]
    fn all_abstention_cannot_qualify_as_an_operational_success() {
        let mut audit = AnytimeEpisodeCoverageAudit::new(AnytimeEpisodeCoverageConfig {
            target_miscoverage: 0.05,
            confidence_error_probability: 0.05,
            minimum_qualified_availability: 0.10,
        })
        .unwrap();
        let mut report = audit.report().unwrap();
        for sequence in 1..=2_000 {
            report = audit
                .observe_completed_episode(event(sequence, sequence, false, None))
                .unwrap();
        }
        assert!(report.running_miscoverage_target_met);
        assert_eq!(report.qualified_availability, 0.0);
        assert!(!report.availability_target_met);
        assert!(!report.operational_targets_met);
    }

    #[test]
    fn all_misses_keep_running_miscoverage_bound_above_target() {
        let mut audit = AnytimeEpisodeCoverageAudit::new(AnytimeEpisodeCoverageConfig {
            target_miscoverage: 0.05,
            confidence_error_probability: 0.05,
            minimum_qualified_availability: 0.0,
        })
        .unwrap();
        let mut report = audit.report().unwrap();
        for sequence in 1..=500 {
            report = audit
                .observe_completed_episode(event(sequence, sequence, true, Some(true)))
                .unwrap();
        }
        assert_eq!(report.missed_episode_count, 500);
        assert_eq!(report.observed_running_miscoverage, 1.0);
        assert_eq!(report.true_running_miscoverage_upper, 1.0);
        assert!(!report.running_miscoverage_target_met);
    }

    #[test]
    fn bounded_episode_requires_sealed_truth_and_outcome_receipt() {
        let mut audit = audit();
        let mut missing_miss = event(1, 1, true, Some(false));
        missing_miss.missed = None;
        assert_eq!(
            audit.observe_completed_episode(missing_miss),
            Err(AnytimeEpisodeCoverageError::MissingOutcomeForBoundedEpisode)
        );
        let mut missing_receipt = event(1, 1, true, Some(false));
        missing_receipt.outcome_receipt_sha256 = None;
        assert_eq!(
            audit.observe_completed_episode(missing_receipt),
            Err(AnytimeEpisodeCoverageError::MissingOutcomeReceipt)
        );
        assert_eq!(audit.report().unwrap().monitoring_horizon, 0);
    }

    #[test]
    fn abstention_cannot_claim_a_hit_or_truth_outcome() {
        let mut audit = audit();
        let mut invalid = event(1, 1, false, Some(false));
        assert_eq!(
            audit.observe_completed_episode(invalid),
            Err(AnytimeEpisodeCoverageError::UnexpectedOutcomeForAbstention)
        );
        invalid = event(1, 1, false, None);
        invalid.outcome_receipt_sha256 = Some([0xB2; SHA256_BYTES]);
        assert_eq!(
            audit.observe_completed_episode(invalid),
            Err(AnytimeEpisodeCoverageError::UnexpectedOutcomeReceiptForAbstention)
        );
    }

    #[test]
    fn non_monotonic_sequence_ids_and_receipt_replay_fail_closed() {
        let mut audit = audit();
        audit
            .observe_completed_episode(event(1, 5, true, Some(false)))
            .unwrap();
        let mut lower_episode_seq = event(2, 4, true, Some(false));
        lower_episode_seq.decision_receipt_sequence_id = 6;
        assert_eq!(
            audit.observe_completed_episode(lower_episode_seq),
            Err(AnytimeEpisodeCoverageError::EpisodeSequenceReplayOrReordering)
        );

        let mut lower_receipt_seq = event(2, 6, true, Some(false));
        lower_receipt_seq.decision_receipt_sequence_id = 5;
        assert_eq!(
            audit.observe_completed_episode(lower_receipt_seq),
            Err(AnytimeEpisodeCoverageError::DecisionReceiptSequenceReplayOrReordering)
        );

        let mut digest_replay = event(2, 6, true, Some(false));
        digest_replay.decision_receipt_sha256 = digest(5, 0xA1);
        assert_eq!(
            audit.observe_completed_episode(digest_replay),
            Err(AnytimeEpisodeCoverageError::DecisionReceiptReplay)
        );
    }

    #[test]
    fn episode_identity_and_outcome_receipt_sequences_cannot_replay_or_roll_back() {
        let mut audit = audit();
        audit
            .observe_completed_episode(event(10, 1, true, Some(false)))
            .unwrap();

        let repeated_episode_id = event(10, 2, true, Some(false));
        assert_eq!(
            audit.observe_completed_episode(repeated_episode_id),
            Err(AnytimeEpisodeCoverageError::EpisodeIdentityReplayOrReordering)
        );

        let mut reordered_outcome = event(11, 2, true, Some(false));
        reordered_outcome.outcome_receipt_sequence_id = Some(1);
        assert_eq!(
            audit.observe_completed_episode(reordered_outcome),
            Err(AnytimeEpisodeCoverageError::OutcomeReceiptSequenceReplayOrReordering)
        );

        let mut missing_outcome_sequence = event(11, 2, true, Some(false));
        missing_outcome_sequence.outcome_receipt_sequence_id = None;
        assert_eq!(
            audit.observe_completed_episode(missing_outcome_sequence),
            Err(AnytimeEpisodeCoverageError::MissingOutcomeReceipt)
        );

        // A malformed later event must not consume any sequence or count.
        assert_eq!(audit.report().unwrap().monitoring_horizon, 1);
        let accepted = audit
            .observe_completed_episode(event(11, 2, true, Some(false)))
            .unwrap();
        assert_eq!(accepted.monitoring_horizon, 2);
        assert_eq!(accepted.bounded_episode_count, 2);
    }

    #[test]
    fn confidence_sequence_margin_matches_stitched_empirical_bernstein_formula() {
        let mut audit = audit();
        let first = audit
            .observe_completed_episode(event(1, 1, true, Some(true)))
            .unwrap();
        let variation = 1.0_f64.max(audit.predictable_variation);
        let log_log = (2.0 * variation).ln().ln().max(0.0);
        let log_delta =
            (STITCHED_LOG_CONSTANT / config().confidence_error_probability).ln();
        let expected = (SQRT_MARGIN_COEFFICIENT
            * (variation * (log_log + LOG_LOG_COEFFICIENT * log_delta)).sqrt()
            + LOG_LOG_MARGIN_COEFFICIENT * log_log
            + LOG_MARGIN_COEFFICIENT * log_delta)
            / 1.0;
        assert!((first.anytime_margin - expected).abs() < 1e-12);
    }

    #[test]
    fn confidence_sequence_never_emits_nonfinite_values() {
        let mut audit = audit();
        for sequence in 1..=100 {
            let bounded = sequence % 4 != 0;
            let missed = if bounded { Some(sequence % 17 == 0) } else { None };
            let report = audit
                .observe_completed_episode(event(sequence, sequence, bounded, missed))
                .unwrap();
            assert!(report.anytime_margin.is_finite());
            assert!(report.true_running_miscoverage_upper.is_finite());
            assert!(report.certified_upper_miss_sum.is_finite());
        }
    }

    #[test]
    fn invalid_configuration_is_rejected() {
        assert_eq!(
            AnytimeEpisodeCoverageAudit::new(AnytimeEpisodeCoverageConfig {
                target_miscoverage: 1.0,
                confidence_error_probability: 0.05,
                minimum_qualified_availability: 0.8,
            }),
            Err(AnytimeEpisodeCoverageError::InvalidConfiguration)
        );
        assert_eq!(
            AnytimeEpisodeCoverageAudit::new(AnytimeEpisodeCoverageConfig {
                target_miscoverage: 0.05,
                confidence_error_probability: 0.0,
                minimum_qualified_availability: 0.8,
            }),
            Err(AnytimeEpisodeCoverageError::InvalidConfiguration)
        );
        assert_eq!(
            AnytimeEpisodeCoverageAudit::new(AnytimeEpisodeCoverageConfig {
                target_miscoverage: 0.05,
                confidence_error_probability: 0.05,
                minimum_qualified_availability: 1.1,
            }),
            Err(AnytimeEpisodeCoverageError::InvalidConfiguration)
        );
    }
}
