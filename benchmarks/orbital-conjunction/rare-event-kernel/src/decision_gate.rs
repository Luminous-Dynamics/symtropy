// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact decision-sufficiency classification for bounded collision-risk results.
//!
//! This module separates numerical risk estimation from the policy question:
//! does the complete qualified interval lie on one side of an explicitly
//! declared action threshold? An interval that overlaps the threshold is
//! indeterminate and must not be rounded into a binary decision.
//!
//! The classifier does not validate the provenance or correctness of its
//! input interval, select an operational threshold, or authorize a maneuver.

use crate::space_time_partition::{NonnegativeRationalInterval, Rational};

/// Boundary semantics for a policy's action threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskThresholdRule {
    /// The action condition is risk greater than or equal to the threshold.
    AtOrAbove,
    /// The action condition is risk strictly greater than the threshold.
    StrictlyAbove,
}

/// Three-way result of comparing a complete risk interval with a threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionSufficiency {
    /// Every value in the interval satisfies the declared action condition.
    ThresholdMet,
    /// No value in the interval satisfies the declared action condition.
    ThresholdNotMet,
    /// The interval overlaps the decision boundary; the result is unresolved.
    Indeterminate,
}

/// Exact, replayable result of a risk-interval threshold comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecisionSufficiencyReport {
    /// Caller-supplied risk interval. Its qualification is not established here.
    pub risk_interval: NonnegativeRationalInterval,
    /// Explicit policy threshold in the same units as the risk interval.
    pub threshold: Rational,
    /// Inclusive or strict boundary semantics used for the comparison.
    pub rule: RiskThresholdRule,
    /// Three-way classification; indeterminate is never coerced to a decision.
    pub disposition: DecisionSufficiency,
    /// Exact width of the supplied risk interval.
    pub interval_width: Rational,
}

/// Errors from exact risk-interval decision classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionGateError {
    /// The public interval fields are reversed.
    InvalidRiskInterval,
    /// Exact interval-width arithmetic is not representable.
    ArithmeticOverflow,
}

impl core::fmt::Display for DecisionGateError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidRiskInterval => "risk interval lower bound exceeds upper bound",
            Self::ArithmeticOverflow => "exact risk interval arithmetic overflowed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for DecisionGateError {}

/// Classify a nonnegative risk interval against an explicit action threshold.
///
/// For an inclusive action rule, the interval must be wholly at/above the
/// threshold to return `ThresholdMet`, and wholly below it to return
/// `ThresholdNotMet`. For a strict action rule, the interval must be wholly
/// above or at/below the threshold respectively. All overlapping intervals
/// return `Indeterminate`.
///
/// This function assumes the supplied interval is already qualified by an
/// independent process. It does not establish that assumption, calculate
/// collision probability, decide which threshold is appropriate, or grant
/// operational authority.
pub fn assess_risk_threshold(
    risk_interval: NonnegativeRationalInterval,
    threshold: Rational,
    rule: RiskThresholdRule,
) -> Result<DecisionSufficiencyReport, DecisionGateError> {
    if risk_interval.lower > risk_interval.upper {
        return Err(DecisionGateError::InvalidRiskInterval);
    }

    let disposition = match rule {
        RiskThresholdRule::AtOrAbove => {
            if risk_interval.lower >= threshold {
                DecisionSufficiency::ThresholdMet
            } else if risk_interval.upper < threshold {
                DecisionSufficiency::ThresholdNotMet
            } else {
                DecisionSufficiency::Indeterminate
            }
        }
        RiskThresholdRule::StrictlyAbove => {
            if risk_interval.lower > threshold {
                DecisionSufficiency::ThresholdMet
            } else if risk_interval.upper <= threshold {
                DecisionSufficiency::ThresholdNotMet
            } else {
                DecisionSufficiency::Indeterminate
            }
        }
    };

    let interval_width = risk_interval
        .upper
        .checked_sub(risk_interval.lower)
        .map_err(|_| DecisionGateError::ArithmeticOverflow)?;

    Ok(DecisionSufficiencyReport {
        risk_interval,
        threshold,
        rule,
        disposition,
        interval_width,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rational(numerator: u128, denominator: u128) -> Rational {
        Rational::new(numerator, denominator).unwrap()
    }

    fn interval(lower: Rational, upper: Rational) -> NonnegativeRationalInterval {
        NonnegativeRationalInterval { lower, upper }
    }

    #[test]
    fn inclusive_rule_is_not_met_when_the_complete_interval_is_below_threshold() {
        let report = assess_risk_threshold(
            interval(rational(1, 20), rational(9, 100)),
            rational(1, 10),
            RiskThresholdRule::AtOrAbove,
        )
        .unwrap();

        assert_eq!(report.disposition, DecisionSufficiency::ThresholdNotMet);
    }

    #[test]
    fn inclusive_rule_is_met_when_the_lower_bound_equals_threshold() {
        let report = assess_risk_threshold(
            interval(rational(1, 10), rational(1, 5)),
            rational(1, 10),
            RiskThresholdRule::AtOrAbove,
        )
        .unwrap();

        assert_eq!(report.disposition, DecisionSufficiency::ThresholdMet);
    }

    #[test]
    fn interval_straddling_threshold_remains_indeterminate() {
        let report = assess_risk_threshold(
            interval(rational(9, 100_000), rational(11, 100_000)),
            rational(1, 10_000),
            RiskThresholdRule::AtOrAbove,
        )
        .unwrap();

        assert_eq!(report.disposition, DecisionSufficiency::Indeterminate);
    }

    #[test]
    fn strict_rule_does_not_treat_exact_threshold_as_exceedance() {
        let report = assess_risk_threshold(
            interval(rational(1, 10), rational(1, 10)),
            rational(1, 10),
            RiskThresholdRule::StrictlyAbove,
        )
        .unwrap();

        assert_eq!(report.disposition, DecisionSufficiency::ThresholdNotMet);
    }

    #[test]
    fn strict_rule_is_met_only_when_the_lower_bound_is_above_threshold() {
        let report = assess_risk_threshold(
            interval(rational(1001, 10_000), rational(11, 100)),
            rational(1, 10),
            RiskThresholdRule::StrictlyAbove,
        )
        .unwrap();

        assert_eq!(report.disposition, DecisionSufficiency::ThresholdMet);
    }

    #[test]
    fn strict_rule_is_not_met_when_the_upper_bound_equals_threshold() {
        let report = assess_risk_threshold(
            interval(rational(9, 100), rational(1, 10)),
            rational(1, 10),
            RiskThresholdRule::StrictlyAbove,
        )
        .unwrap();

        assert_eq!(report.disposition, DecisionSufficiency::ThresholdNotMet);
    }

    #[test]
    fn interval_width_is_reported_as_an_exact_rational() {
        let report = assess_risk_threshold(
            interval(rational(1, 10), rational(1, 5)),
            rational(3, 20),
            RiskThresholdRule::AtOrAbove,
        )
        .unwrap();

        assert_eq!(report.interval_width, rational(1, 10));
        assert_eq!(report.disposition, DecisionSufficiency::Indeterminate);
    }

    #[test]
    fn unrepresentable_exact_interval_width_fails_closed() {
        let bounds = interval(
            rational(1, u128::MAX),
            rational(1, u128::MAX - 1),
        );

        assert_eq!(
            assess_risk_threshold(
                bounds,
                rational(1, 2),
                RiskThresholdRule::AtOrAbove,
            ),
            Err(DecisionGateError::ArithmeticOverflow)
        );
    }

    #[test]
    fn reversed_public_interval_is_rejected() {
        assert_eq!(
            assess_risk_threshold(
                interval(rational(2, 10), rational(1, 10)),
                rational(1, 10),
                RiskThresholdRule::AtOrAbove,
            ),
            Err(DecisionGateError::InvalidRiskInterval)
        );
    }
}
