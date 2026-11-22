//! Zero-failure evidence bounds for predeclared independent mission trials.
//!
//! These functions do not turn high-rate samples, repeated windows, or merely
//! distinct route IDs into independent trials. The caller must establish that
//! each trial is a separate, exchangeable Bernoulli mission unit of the same
//! declared scope and duration.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZeroFailureEvidence {
    pub independent_trials: u64,
    pub observed_failures: u64,
    pub confidence_level: f64,
    pub target_risk: f64,
    pub one_sided_upper_bound: f64,
    pub minimum_required_trials: u64,
    pub target_met: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ZeroFailureEvidenceError {
    NoIndependentTrials,
    NonZeroFailuresRequireGeneralBound,
    InvalidConfidenceLevel,
    InvalidTargetRisk,
    TrialCountUnrepresentable,
}

impl fmt::Display for ZeroFailureEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoIndependentTrials => write!(f, "at least one independent mission trial is required"),
            Self::NonZeroFailuresRequireGeneralBound => write!(f, "nonzero failures require a general binomial confidence bound"),
            Self::InvalidConfidenceLevel => write!(f, "confidence level must be finite and in (0,1)"),
            Self::InvalidTargetRisk => write!(f, "target risk must be finite and in (0,1)"),
            Self::TrialCountUnrepresentable => write!(f, "required trial count cannot be represented as u64"),
        }
    }
}
impl std::error::Error for ZeroFailureEvidenceError {}

fn validate_probability(value: f64) -> bool {
    value.is_finite() && value > 0.0 && value < 1.0
}

/// Exact one-sided binomial upper confidence bound after zero failures:
///
/// p_upper = 1 - (1-confidence)^(1/n)
///
/// Assumes n independent, exchangeable Bernoulli mission trials with the same
/// predefined scope and outcome definition. This is not a per-hour event rate,
/// not a cluster-robust bound, and not valid merely because there are n rows.
pub fn zero_failure_upper_bound(
    confidence_level: f64,
    independent_trials: u64,
) -> Result<f64, ZeroFailureEvidenceError> {
    if independent_trials == 0 {
        return Err(ZeroFailureEvidenceError::NoIndependentTrials);
    }
    if !validate_probability(confidence_level) {
        return Err(ZeroFailureEvidenceError::InvalidConfidenceLevel);
    }
    let alpha = 1.0 - confidence_level;
    let exponent = alpha.ln() / independent_trials as f64;
    let upper = -exponent.exp_m1();
    if !upper.is_finite() || !(0.0..=1.0).contains(&upper) {
        return Err(ZeroFailureEvidenceError::TrialCountUnrepresentable);
    }
    Ok(upper)
}

/// Minimum independent zero-failure mission trials needed for the one-sided
/// upper confidence bound to be no greater than the target risk.
pub fn minimum_zero_failure_trials(
    target_risk: f64,
    confidence_level: f64,
) -> Result<u64, ZeroFailureEvidenceError> {
    if !validate_probability(confidence_level) {
        return Err(ZeroFailureEvidenceError::InvalidConfidenceLevel);
    }
    if !validate_probability(target_risk) {
        return Err(ZeroFailureEvidenceError::InvalidTargetRisk);
    }
    let alpha = 1.0 - confidence_level;
    let required = (alpha.ln() / (-target_risk).ln_1p()).ceil();
    if !required.is_finite() || required < 1.0 || required > u64::MAX as f64 {
        return Err(ZeroFailureEvidenceError::TrialCountUnrepresentable);
    }
    Ok(required as u64)
}

/// Evaluate a zero-failure claim against an explicit target and confidence.
///
/// This intentionally rejects observed failures rather than applying a
/// zero-failure formula outside its derivation. A general exact binomial
/// interval is required when one or more failures have been observed.
pub fn qualify_zero_failure_evidence(
    observed_failures: u64,
    independent_trials: u64,
    target_risk: f64,
    confidence_level: f64,
) -> Result<ZeroFailureEvidence, ZeroFailureEvidenceError> {
    if independent_trials == 0 {
        return Err(ZeroFailureEvidenceError::NoIndependentTrials);
    }
    if observed_failures > 0 {
        return Err(ZeroFailureEvidenceError::NonZeroFailuresRequireGeneralBound);
    }
    if !validate_probability(target_risk) {
        return Err(ZeroFailureEvidenceError::InvalidTargetRisk);
    }
    let upper = zero_failure_upper_bound(confidence_level, independent_trials)?;
    let minimum_required_trials = minimum_zero_failure_trials(target_risk, confidence_level)?;
    Ok(ZeroFailureEvidence {
        independent_trials,
        observed_failures,
        confidence_level,
        target_risk,
        one_sided_upper_bound: upper,
        minimum_required_trials,
        target_met: upper <= target_risk,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_failures_need_2995_independent_missions_for_one_in_a_thousand_at_95_percent() {
        let required = minimum_zero_failure_trials(1.0e-3, 0.95).unwrap();
        assert_eq!(required, 2_995);

        let sufficient = qualify_zero_failure_evidence(0, 2_995, 1.0e-3, 0.95).unwrap();
        assert!(sufficient.target_met);
        assert!(sufficient.one_sided_upper_bound <= 1.0e-3);

        let insufficient = qualify_zero_failure_evidence(0, 2_994, 1.0e-3, 0.95).unwrap();
        assert!(!insufficient.target_met);
        assert!(insufficient.one_sided_upper_bound > 1.0e-3);
    }

    #[test]
    fn one_zero_failure_trial_has_upper_bound_equal_to_confidence_level() {
        let upper = zero_failure_upper_bound(0.95, 1).unwrap();
        assert!((upper - 0.95).abs() < 1.0e-12);
    }

    #[test]
    fn zero_failure_bound_is_monotonic_in_trial_count_and_confidence() {
        let n_100 = zero_failure_upper_bound(0.95, 100).unwrap();
        let n_200 = zero_failure_upper_bound(0.95, 200).unwrap();
        let c_99 = zero_failure_upper_bound(0.99, 100).unwrap();
        assert!(n_200 < n_100);
        assert!(c_99 > n_100);
    }

    #[test]
    fn nonzero_failures_are_not_passed_through_zero_failure_formula() {
        assert_eq!(
            qualify_zero_failure_evidence(1, 2_995, 1.0e-3, 0.95),
            Err(ZeroFailureEvidenceError::NonZeroFailuresRequireGeneralBound)
        );
    }

    #[test]
    fn invalid_probabilities_and_empty_trials_fail_closed() {
        assert_eq!(
            zero_failure_upper_bound(0.95, 0),
            Err(ZeroFailureEvidenceError::NoIndependentTrials)
        );
        assert_eq!(
            zero_failure_upper_bound(1.0, 10),
            Err(ZeroFailureEvidenceError::InvalidConfidenceLevel)
        );
        assert_eq!(
            minimum_zero_failure_trials(0.0, 0.95),
            Err(ZeroFailureEvidenceError::InvalidTargetRisk)
        );
        assert_eq!(
            minimum_zero_failure_trials(1.0e-3, 1.0),
            Err(ZeroFailureEvidenceError::InvalidConfidenceLevel)
        );
    }
}
