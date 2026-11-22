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
    InvalidFailureCount,
    TrialCountUnrepresentable,
    NumericalConvergenceFailure,
}

impl fmt::Display for ZeroFailureEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoIndependentTrials => write!(f, "at least one independent mission trial is required"),
            Self::NonZeroFailuresRequireGeneralBound => write!(f, "nonzero failures require a general binomial confidence bound"),
            Self::InvalidConfidenceLevel => write!(f, "confidence level must be finite and in (0,1)"),
            Self::InvalidTargetRisk => write!(f, "target risk must be finite and in (0,1)"),
            Self::InvalidFailureCount => write!(f, "observed failures cannot exceed independent trials"),
            Self::TrialCountUnrepresentable => write!(f, "required trial count cannot be represented as u64"),
            Self::NumericalConvergenceFailure => write!(f, "incomplete beta evaluation did not converge"),
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


#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BinomialIntegrityEvidence {
    pub independent_trials: u64,
    pub observed_failures: u64,
    pub confidence_level: f64,
    pub target_risk: f64,
    pub one_sided_upper_bound: f64,
    pub target_met: bool,
}

// Lanczos approximation for ln(Gamma(z)); used only by the exact
// Clopper-Pearson beta-quantile implementation below.
#[allow(clippy::excessive_precision)]
fn ln_gamma(z: f64) -> f64 {
    const COEFFICIENTS: [f64; 8] = [
        676.5203681218851,
        -1259.1392167224028,
        771.32342877765313,
        -176.61502916214059,
        12.507343278686905,
        -0.13857109526572012,
        9.9843695780195716e-6,
        1.5056327351493116e-7,
    ];
    if z < 0.5 {
        return std::f64::consts::PI.ln()
            - (std::f64::consts::PI * z).sin().ln()
            - ln_gamma(1.0 - z);
    }
    let shifted = z - 1.0;
    let mut x = 0.99999999999980993;
    for (index, coefficient) in COEFFICIENTS.iter().enumerate() {
        x += coefficient / (shifted + index as f64 + 1.0);
    }
    let t = shifted + 7.5;
    0.9189385332046727 + (shifted + 0.5) * t.ln() - t + x.ln()
}

fn beta_continued_fraction(a: f64, b: f64, x: f64) -> Result<f64, ZeroFailureEvidenceError> {
    const MAX_ITERATIONS: usize = 10_000;
    const EPSILON: f64 = 3.0e-14;
    const FLOOR: f64 = 1.0e-300;
    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < FLOOR {
        d = FLOOR;
    }
    d = 1.0 / d;
    let mut h = d;

    for m in 1..=MAX_ITERATIONS {
        let m_f = m as f64;
        let m2 = 2.0 * m_f;
        let mut aa = m_f * (b - m_f) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < FLOOR { d = FLOOR; }
        c = 1.0 + aa / c;
        if c.abs() < FLOOR { c = FLOOR; }
        d = 1.0 / d;
        h *= d * c;

        aa = -(a + m_f) * (qab + m_f) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < FLOOR { d = FLOOR; }
        c = 1.0 + aa / c;
        if c.abs() < FLOOR { c = FLOOR; }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if (delta - 1.0).abs() <= EPSILON {
            return Ok(h);
        }
    }
    Err(ZeroFailureEvidenceError::NumericalConvergenceFailure)
}

fn regularized_incomplete_beta(a: f64, b: f64, x: f64) -> Result<f64, ZeroFailureEvidenceError> {
    if x <= 0.0 { return Ok(0.0); }
    if x >= 1.0 { return Ok(1.0); }
    if !a.is_finite() || !b.is_finite() || a <= 0.0 || b <= 0.0 {
        return Err(ZeroFailureEvidenceError::TrialCountUnrepresentable);
    }
    let log_front = ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b)
        + a * x.ln() + b * (-x).ln_1p();
    let front = log_front.exp();
    let value = if x < (a + 1.0) / (a + b + 2.0) {
        front * beta_continued_fraction(a, b, x)? / a
    } else {
        1.0 - front * beta_continued_fraction(b, a, 1.0 - x)? / b
    };
    if !value.is_finite() {
        return Err(ZeroFailureEvidenceError::NumericalConvergenceFailure);
    }
    Ok(value.clamp(0.0, 1.0))
}

fn inverse_regularized_beta(probability: f64, a: f64, b: f64) -> Result<f64, ZeroFailureEvidenceError> {
    let mut low = 0.0;
    let mut high = 1.0;
    for _ in 0..160 {
        let midpoint = low + (high - low) * 0.5;
        let cdf = regularized_incomplete_beta(a, b, midpoint)?;
        if cdf < probability {
            low = midpoint;
        } else {
            high = midpoint;
        }
    }
    Ok(low + (high - low) * 0.5)
}

/// Exact one-sided Clopper-Pearson upper confidence limit for a binomial
/// mission-failure probability. Requires independent, exchangeable mission
/// trials with a fixed and predeclared outcome definition.
///
/// For 0 < failures < trials, the limit is the confidence quantile of
/// Beta(failures + 1, trials - failures). The zero-failure case uses the
/// closed form above. This is not a correction for dependent or clustered
/// epochs; the caller must supply valid mission-level Bernoulli trials.
pub fn one_sided_binomial_upper_bound(
    confidence_level: f64,
    observed_failures: u64,
    independent_trials: u64,
) -> Result<f64, ZeroFailureEvidenceError> {
    if independent_trials == 0 {
        return Err(ZeroFailureEvidenceError::NoIndependentTrials);
    }
    if observed_failures > independent_trials {
        return Err(ZeroFailureEvidenceError::InvalidFailureCount);
    }
    if !validate_probability(confidence_level) {
        return Err(ZeroFailureEvidenceError::InvalidConfidenceLevel);
    }
    if observed_failures == 0 {
        return zero_failure_upper_bound(confidence_level, independent_trials);
    }
    if observed_failures == independent_trials {
        return Ok(1.0);
    }
    // Above 2^53, integer-to-f64 conversion loses count resolution. Refuse
    // to return an apparently exact limit for an unrepresentable count.
    if independent_trials > (1_u64 << 53) {
        return Err(ZeroFailureEvidenceError::TrialCountUnrepresentable);
    }
    inverse_regularized_beta(
        confidence_level,
        observed_failures as f64 + 1.0,
        (independent_trials - observed_failures) as f64,
    )
}

/// Qualify general binomial mission evidence against a declared risk target.
pub fn qualify_binomial_evidence(
    observed_failures: u64,
    independent_trials: u64,
    target_risk: f64,
    confidence_level: f64,
) -> Result<BinomialIntegrityEvidence, ZeroFailureEvidenceError> {
    if !validate_probability(target_risk) {
        return Err(ZeroFailureEvidenceError::InvalidTargetRisk);
    }
    let upper = one_sided_binomial_upper_bound(
        confidence_level,
        observed_failures,
        independent_trials,
    )?;
    Ok(BinomialIntegrityEvidence {
        independent_trials,
        observed_failures,
        confidence_level,
        target_risk,
        one_sided_upper_bound: upper,
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
    fn general_clopper_pearson_bound_matches_binomial_lower_tail_equation() {
        let upper = one_sided_binomial_upper_bound(0.95, 1, 10).unwrap();
        let tail = (1.0 - upper).powi(10)
            + 10.0 * upper * (1.0 - upper).powi(9);
        assert!((tail - 0.05).abs() < 1.0e-12);
        assert!(upper > 0.1);
    }

    #[test]
    fn general_binomial_bound_matches_zero_failure_closed_form() {
        let general = one_sided_binomial_upper_bound(0.95, 0, 2_995).unwrap();
        let closed = zero_failure_upper_bound(0.95, 2_995).unwrap();
        assert!((general - closed).abs() < 1.0e-15);
    }

    #[test]
    fn binomial_upper_bound_increases_with_failures_and_is_one_at_all_failures() {
        let zero = one_sided_binomial_upper_bound(0.95, 0, 10).unwrap();
        let one = one_sided_binomial_upper_bound(0.95, 1, 10).unwrap();
        let two = one_sided_binomial_upper_bound(0.95, 2, 10).unwrap();
        let all = one_sided_binomial_upper_bound(0.95, 10, 10).unwrap();
        assert!(zero < one && one < two && two < all);
        assert_eq!(all, 1.0);
    }

    #[test]
    fn binomial_evidence_reports_target_and_rejects_impossible_counts() {
        let evidence = qualify_binomial_evidence(1, 10, 0.5, 0.95).unwrap();
        assert!(evidence.target_met);
        assert_eq!(evidence.observed_failures, 1);
        assert_eq!(
            one_sided_binomial_upper_bound(0.95, 11, 10),
            Err(ZeroFailureEvidenceError::InvalidFailureCount)
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
