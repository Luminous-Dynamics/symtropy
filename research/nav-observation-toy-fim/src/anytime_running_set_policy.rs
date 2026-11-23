//! Adaptive bounded-set / abstention policy from the anytime running audit.
//!
//! Implements the miscoverage-level update from Fang (2026), arXiv:2611.20905,
//! Equations (34)-(35), using the certified upper miss sum produced by the
//! anytime episode coverage audit.
//!
//! This module chooses only the requested miscoverage level or abstention. It
//! does not construct prediction sets, train/update the predictor, or authorize
//! an actuator. The caller must preserve the paper's pre-outcome filtration and
//! provide a prediction-set family defined for the requested level.

use std::fmt;

use crate::anytime_episode_coverage::AnytimeEpisodeCoverageReport;

const STITCHED_LOG_COEFFICIENT: f64 = 0.72;
const STITCHED_LOG_CONSTANT: f64 = 5.19;
const STITCHED_C_SQRT_COEFFICIENT: f64 = 1.7;
const STITCHED_C_LINEAR_COEFFICIENT: f64 = 2.45;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnytimeRunningSetPolicyConfig {
    /// Target true running-average miscoverage alpha.
    pub target_miscoverage: f64,
    /// Confidence-sequence error probability delta.
    pub confidence_error_probability: f64,
    /// Positive adaptation learning rate gamma.
    pub learning_rate_gamma: f64,
    /// Fixed overshoot buffer B from the paper's theorem.
    pub buffer: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnytimeRunningSetDecision {
    /// Requested level is nonpositive. The caller must abstain for this episode.
    Abstain {
        next_episode_index: u64,
        requested_miscoverage_level: f64,
        certified_upper_miss_sum: f64,
    },
    /// Caller may issue a bounded set at exactly this requested level, but only
    /// if its prediction-set implementation supports the requested domain.
    IssueBoundedSet {
        next_episode_index: u64,
        requested_miscoverage_level: f64,
        certified_upper_miss_sum: f64,
    },
}

impl AnytimeRunningSetDecision {
    pub fn next_episode_index(&self) -> u64 {
        match self {
            Self::Abstain { next_episode_index, .. }
            | Self::IssueBoundedSet { next_episode_index, .. } => *next_episode_index,
        }
    }

    pub fn requested_miscoverage_level(&self) -> f64 {
        match self {
            Self::Abstain { requested_miscoverage_level, .. }
            | Self::IssueBoundedSet { requested_miscoverage_level, .. } => {
                *requested_miscoverage_level
            }
        }
    }

    pub fn certified_upper_miss_sum(&self) -> f64 {
        match self {
            Self::Abstain { certified_upper_miss_sum, .. }
            | Self::IssueBoundedSet { certified_upper_miss_sum, .. } => {
                *certified_upper_miss_sum
            }
        }
    }

    pub fn is_abstention(&self) -> bool {
        matches!(self, Self::Abstain { .. })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnytimeRunningSetPolicyError {
    InvalidConfiguration,
    BufferBelowTheoremMinimum,
    InvalidAuditSnapshot,
    AuditPolicyMismatch,
    CounterOverflow,
    ArithmeticOverflow,
}

impl fmt::Display for AnytimeRunningSetPolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfiguration => write!(f, "alpha, delta and gamma must be finite and valid, and the buffer must be finite and nonnegative"),
            Self::BufferBelowTheoremMinimum => write!(f, "buffer B is below alpha/gamma - alpha + c required by the running-average theorem"),
            Self::InvalidAuditSnapshot => write!(f, "audit snapshot has invalid horizon, confidence level or certified miss sum"),
            Self::AuditPolicyMismatch => write!(f, "audit snapshot target alpha or confidence level does not match the policy"),
            Self::CounterOverflow => write!(f, "next episode index overflowed"),
            Self::ArithmeticOverflow => write!(f, "adaptive miscoverage-level arithmetic overflowed"),
        }
    }
}

impl std::error::Error for AnytimeRunningSetPolicyError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AnytimeRunningSetPolicy {
    config: AnytimeRunningSetPolicyConfig,
    stitched_one_step_increase_bound: f64,
    minimum_buffer: f64,
}

impl AnytimeRunningSetPolicy {
    pub fn new(
        config: AnytimeRunningSetPolicyConfig,
    ) -> Result<Self, AnytimeRunningSetPolicyError> {
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
        {
            return Err(AnytimeRunningSetPolicyError::InvalidConfiguration);
        }

        let log_term = (STITCHED_LOG_CONSTANT / config.confidence_error_probability).ln();
        if !log_term.is_finite() || log_term <= 0.0 {
            return Err(AnytimeRunningSetPolicyError::InvalidConfiguration);
        }

        // Fang (2026): c bounds the maximum one-step increase in the certified
        // miss sum for the stitched empirical-Bernstein confidence margin.
        let stitched_one_step_increase_bound = 1.0
            + STITCHED_C_SQRT_COEFFICIENT * (STITCHED_LOG_COEFFICIENT * log_term).sqrt()
            + STITCHED_C_LINEAR_COEFFICIENT * log_term;
        let minimum_buffer = config.target_miscoverage / config.learning_rate_gamma
            - config.target_miscoverage
            + stitched_one_step_increase_bound;

        if !stitched_one_step_increase_bound.is_finite() || !minimum_buffer.is_finite() {
            return Err(AnytimeRunningSetPolicyError::ArithmeticOverflow);
        }
        if config.buffer < minimum_buffer {
            return Err(AnytimeRunningSetPolicyError::BufferBelowTheoremMinimum);
        }

        Ok(Self {
            config,
            stitched_one_step_increase_bound,
            minimum_buffer,
        })
    }

    pub fn config(&self) -> AnytimeRunningSetPolicyConfig {
        self.config
    }

    pub fn stitched_one_step_increase_bound(&self) -> f64 {
        self.stitched_one_step_increase_bound
    }

    pub fn minimum_buffer(&self) -> f64 {
        self.minimum_buffer
    }

    /// Choose the next episode's level before its truth outcome is observed.
    ///
    /// At completed horizon t, the next level is:
    /// eta_(t+1) = alpha + gamma * (alpha*t - S^c_t - B).
    ///
    /// If eta <= 0, the caller abstains. Otherwise it requests a bounded set at
    /// exactly eta. The level is intentionally not clipped to [0,1]: the
    /// prediction-set adapter must define and test the exact parameter domain
    /// and set semantics, including eta >= 1, for its chosen conformal family.
    pub fn decide_next_episode(
        &self,
        audit: &AnytimeEpisodeCoverageReport,
    ) -> Result<AnytimeRunningSetDecision, AnytimeRunningSetPolicyError> {
        if !audit.certified_upper_miss_sum.is_finite()
            || audit.certified_upper_miss_sum < 0.0
            || !audit.target_miscoverage.is_finite()
            || !audit.confidence_level.is_finite()
            || audit.confidence_level <= 0.0
            || audit.confidence_level >= 1.0
        {
            return Err(AnytimeRunningSetPolicyError::InvalidAuditSnapshot);
        }

        let expected_confidence = 1.0 - self.config.confidence_error_probability;
        if (audit.target_miscoverage - self.config.target_miscoverage).abs() > 1e-12
            || (audit.confidence_level - expected_confidence).abs() > 1e-12
        {
            return Err(AnytimeRunningSetPolicyError::AuditPolicyMismatch);
        }

        let next_episode_index = audit
            .monitoring_horizon
            .checked_add(1)
            .ok_or(AnytimeRunningSetPolicyError::CounterOverflow)?;
        let completed_horizon = audit.monitoring_horizon as f64;
        let allowance = self.config.target_miscoverage * completed_horizon;
        let level = self.config.target_miscoverage
            + self.config.learning_rate_gamma
                * (allowance - audit.certified_upper_miss_sum - self.config.buffer);

        if !level.is_finite() {
            return Err(AnytimeRunningSetPolicyError::ArithmeticOverflow);
        }

        if level <= 0.0 {
            Ok(AnytimeRunningSetDecision::Abstain {
                next_episode_index,
                requested_miscoverage_level: level,
                certified_upper_miss_sum: audit.certified_upper_miss_sum,
            })
        } else {
            Ok(AnytimeRunningSetDecision::IssueBoundedSet {
                next_episode_index,
                requested_miscoverage_level: level,
                certified_upper_miss_sum: audit.certified_upper_miss_sum,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> AnytimeRunningSetPolicyConfig {
        let target_miscoverage = 0.05;
        let confidence_error_probability = 0.05;
        let learning_rate_gamma = 0.01;
        let log_term = (STITCHED_LOG_CONSTANT / confidence_error_probability).ln();
        let c = 1.0
            + STITCHED_C_SQRT_COEFFICIENT * (STITCHED_LOG_COEFFICIENT * log_term).sqrt()
            + STITCHED_C_LINEAR_COEFFICIENT * log_term;
        let minimum_buffer = target_miscoverage / learning_rate_gamma
            - target_miscoverage
            + c;
        AnytimeRunningSetPolicyConfig {
            target_miscoverage,
            confidence_error_probability,
            learning_rate_gamma,
            buffer: minimum_buffer,
        }
    }

    fn report(
        horizon: u64,
        certified_upper_miss_sum: f64,
    ) -> AnytimeEpisodeCoverageReport {
        let cfg = config();
        AnytimeEpisodeCoverageReport {
            monitoring_horizon: horizon,
            bounded_episode_count: horizon,
            abstention_count: 0,
            missed_episode_count: 0,
            observed_running_miscoverage: 0.0,
            bounded_episode_miscoverage: Some(0.0),
            qualified_availability: 1.0,
            anytime_margin: 0.0,
            true_running_miscoverage_upper: 0.0,
            confidence_level: 1.0 - cfg.confidence_error_probability,
            target_miscoverage: cfg.target_miscoverage,
            running_miscoverage_target_met: true,
            minimum_qualified_availability: 0.0,
            availability_target_met: true,
            operational_targets_met: true,
            predictable_variation: 0.0,
            certified_upper_miss_sum,
        }
    }

    fn policy() -> AnytimeRunningSetPolicy {
        AnytimeRunningSetPolicy::new(config()).unwrap()
    }

    #[test]
    fn theorem_buffer_is_enforced() {
        let mut invalid = config();
        invalid.buffer = policy().minimum_buffer() - 1e-9;
        assert_eq!(
            AnytimeRunningSetPolicy::new(invalid),
            Err(AnytimeRunningSetPolicyError::BufferBelowTheoremMinimum)
        );
    }

    #[test]
    fn starts_by_abstaining_when_minimum_theorem_buffer_is_used() {
        let policy = policy();
        let decision = policy.decide_next_episode(&report(0, 0.0)).unwrap();
        assert!(decision.is_abstention());
        let expected = config().target_miscoverage
            - config().learning_rate_gamma * config().buffer;
        assert!((decision.requested_miscoverage_level() - expected).abs() < 1e-12);
        assert!(decision.requested_miscoverage_level() < 0.0);
    }

    #[test]
    fn abstention_allowance_increases_by_gamma_alpha_per_completed_episode() {
        let policy = policy();
        let first = policy.decide_next_episode(&report(0, 0.0)).unwrap();
        let second = policy.decide_next_episode(&report(1, 0.0)).unwrap();
        let third = policy.decide_next_episode(&report(2, 0.0)).unwrap();
        let step = config().learning_rate_gamma * config().target_miscoverage;
        assert!(
            (second.requested_miscoverage_level()
                - first.requested_miscoverage_level()
                - step)
                .abs()
                < 1e-12
        );
        assert!(
            (third.requested_miscoverage_level()
                - second.requested_miscoverage_level()
                - step)
                .abs()
                < 1e-12
        );
        assert_eq!(second.next_episode_index(), 2);
        assert_eq!(third.next_episode_index(), 3);
    }

    #[test]
    fn a_larger_certified_miss_sum_can_force_abstention() {
        let policy = policy();
        let low = policy.decide_next_episode(&report(100, 1.0)).unwrap();
        let high = policy.decide_next_episode(&report(100, 10.0)).unwrap();
        assert!(high.requested_miscoverage_level() < low.requested_miscoverage_level());
        assert!(high.is_abstention());
    }

    #[test]
    fn a_bounded_decision_requests_the_exact_unclipped_level() {
        let policy = policy();
        let decision = policy.decide_next_episode(&report(10_000, 0.0)).unwrap();
        assert!(!decision.is_abstention());
        assert!(decision.requested_miscoverage_level() > 0.0);
        assert_eq!(decision.next_episode_index(), 10_001);
    }

    #[test]
    fn audit_target_or_confidence_mismatch_is_rejected() {
        let policy = policy();
        let mut mismatched = report(10, 0.0);
        mismatched.target_miscoverage += 0.01;
        assert_eq!(
            policy.decide_next_episode(&mismatched),
            Err(AnytimeRunningSetPolicyError::AuditPolicyMismatch)
        );
        mismatched = report(10, 0.0);
        mismatched.confidence_level -= 0.01;
        assert_eq!(
            policy.decide_next_episode(&mismatched),
            Err(AnytimeRunningSetPolicyError::AuditPolicyMismatch)
        );
    }

    #[test]
    fn invalid_audit_snapshot_and_overflow_fail_closed() {
        let policy = policy();
        let invalid = report(10, f64::NAN);
        assert_eq!(
            policy.decide_next_episode(&invalid),
            Err(AnytimeRunningSetPolicyError::InvalidAuditSnapshot)
        );
        let overflow = report(u64::MAX, 0.0);
        assert_eq!(
            policy.decide_next_episode(&overflow),
            Err(AnytimeRunningSetPolicyError::CounterOverflow)
        );
    }
}
