//! Interval-based dominance decisions for scientific experiment design.
//!
//! The decision layer does not estimate utility. It consumes utility bounds
//! produced by exact linear-Gaussian calculations, theorem-qualified bounds,
//! or a simultaneous anytime-valid Monte Carlo confidence sequence.
//!
//! A winner is certified only when its lower bound clears every competitor's
//! upper bound by a caller-declared margin. Heuristic or unqualified bounds
//! force an unresolved outcome even if their numerical intervals are disjoint.
//!
//! Evidence variants are typed declarations, not self-authenticating proofs.
//! Producers must attach receipts that an independent verifier can resolve.

use crate::ModelError;

/// Epistemic status attached to a utility interval.
#[derive(Debug, Clone, PartialEq)]
pub enum UtilityBoundEvidence {
    /// An exact value under a declared, validated mathematical model.
    Exact,
    /// A theorem-derived interval with externally verifiable assumptions and
    /// numerical-enclosure receipts.
    TheoremCertified {
        theorem_id: String,
        assumptions_receipt_id: String,
        enclosure_receipt_id: String,
    },
    /// A confidence sequence with a single receipt covering the complete
    /// candidate family simultaneously at all inspection/stopping times.
    AnytimeValidFamilywise {
        alpha: f64,
        family_receipt_id: String,
    },
    /// A numerical interval without a qualifying proof or coverage guarantee.
    Heuristic,
    /// The underlying model, source lineage, or calculation is unqualified.
    Unqualified,
}

/// A utility interval for one candidate portfolio. Larger is better.
#[derive(Debug, Clone, PartialEq)]
pub struct PortfolioUtilityInterval {
    pub portfolio_id: String,
    pub lower: f64,
    pub upper: f64,
    pub evidence: UtilityBoundEvidence,
}

/// Why the controller could not certify a unique winner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DominanceUnresolvedReason {
    UtilityIntervalsOverlap,
    UnqualifiedOrHeuristicBounds,
    IncompatibleFamilywiseReceipts,
}

/// Result of an interval-dominance decision.
#[derive(Debug, Clone, PartialEq)]
pub enum DominanceDecision {
    CertifiedWinner {
        portfolio_id: String,
        winner_lower_bound: f64,
        best_competitor_upper_bound: Option<f64>,
        separation_margin: Option<f64>,
        required_margin: f64,
        familywise_alpha: Option<f64>,
    },
    Unresolved {
        reason: DominanceUnresolvedReason,
    },
}

/// Certify a maximum-utility portfolio using only qualified utility intervals.
///
/// If any interval is heuristic or unqualified, no winner is certified.
/// Anytime-valid intervals must share one familywise receipt and alpha level;
/// independent pointwise confidence intervals are not silently combined.
/// Exact intervals must have identical lower and upper endpoints.
///
/// The minimum separation is in the same utility units as the interval
/// endpoints. A strict inequality is used: equality at the threshold remains
/// unresolved.
pub fn certify_maximum_utility(
    intervals: &[PortfolioUtilityInterval],
    minimum_separation: f64,
) -> Result<DominanceDecision, ModelError> {
    if intervals.is_empty() {
        return Err(ModelError::InvalidParameter(
            "utility_interval_candidate_set_must_be_nonempty",
        ));
    }
    if !minimum_separation.is_finite() {
        return Err(ModelError::NonFiniteValue(
            "utility_interval_minimum_separation",
        ));
    }
    if minimum_separation < 0.0 {
        return Err(ModelError::InvalidParameter(
            "utility_interval_minimum_separation_must_be_nonnegative",
        ));
    }

    let mut seen = std::collections::BTreeSet::new();
    let mut family_receipt: Option<(&str, f64)> = None;
    for interval in intervals {
        if interval.portfolio_id.trim().is_empty() || !seen.insert(interval.portfolio_id.as_str()) {
            return Err(ModelError::InvalidParameter(
                "utility_interval_portfolio_ids_must_be_nonempty_and_unique",
            ));
        }
        if !interval.lower.is_finite() || !interval.upper.is_finite() {
            return Err(ModelError::NonFiniteValue("utility_interval_endpoint"));
        }
        if interval.lower > interval.upper {
            return Err(ModelError::InvalidParameter(
                "utility_interval_lower_exceeds_upper",
            ));
        }

        match &interval.evidence {
            UtilityBoundEvidence::Exact => {
                if interval.lower != interval.upper {
                    return Err(ModelError::InvalidParameter(
                        "exact_utility_interval_must_have_equal_endpoints",
                    ));
                }
            }
            UtilityBoundEvidence::TheoremCertified {
                theorem_id,
                assumptions_receipt_id,
                enclosure_receipt_id,
            } => {
                if theorem_id.trim().is_empty()
                    || assumptions_receipt_id.trim().is_empty()
                    || enclosure_receipt_id.trim().is_empty()
                {
                    return Err(ModelError::InvalidParameter(
                        "theorem_utility_bound_receipts_must_be_nonempty",
                    ));
                }
            }
            UtilityBoundEvidence::AnytimeValidFamilywise {
                alpha,
                family_receipt_id,
            } => {
                if !alpha.is_finite() {
                    return Err(ModelError::NonFiniteValue(
                        "utility_interval_familywise_alpha",
                    ));
                }
                if *alpha <= 0.0 || *alpha >= 1.0 || family_receipt_id.trim().is_empty() {
                    return Err(ModelError::InvalidParameter(
                        "utility_interval_familywise_evidence_invalid",
                    ));
                }
                if let Some((existing_receipt, existing_alpha)) = family_receipt {
                    if existing_receipt != family_receipt_id || existing_alpha != *alpha {
                        return Ok(DominanceDecision::Unresolved {
                            reason: DominanceUnresolvedReason::IncompatibleFamilywiseReceipts,
                        });
                    }
                } else {
                    family_receipt = Some((family_receipt_id, *alpha));
                }
            }
            UtilityBoundEvidence::Heuristic | UtilityBoundEvidence::Unqualified => {
                return Ok(DominanceDecision::Unresolved {
                    reason: DominanceUnresolvedReason::UnqualifiedOrHeuristicBounds,
                });
            }
        }
    }

    let winner_index = intervals
        .iter()
        .enumerate()
        .max_by(|(_, left), (_, right)| {
            left.lower
                .total_cmp(&right.lower)
                .then_with(|| right.portfolio_id.cmp(&left.portfolio_id))
        })
        .map(|(index, _)| index)
        .ok_or(ModelError::MismatchedDataLengths)?;
    let winner = &intervals[winner_index];

    let mut best_competitor_upper_bound: Option<f64> = None;
    for (index, interval) in intervals.iter().enumerate() {
        if index == winner_index {
            continue;
        }
        best_competitor_upper_bound = Some(
            best_competitor_upper_bound
                .map_or(interval.upper, |current| current.max(interval.upper)),
        );
    }

    let familywise_alpha = family_receipt.map(|(_, alpha)| alpha);
    let Some(competitor_upper) = best_competitor_upper_bound else {
        return Ok(DominanceDecision::CertifiedWinner {
            portfolio_id: winner.portfolio_id.clone(),
            winner_lower_bound: winner.lower,
            best_competitor_upper_bound: None,
            separation_margin: None,
            required_margin: minimum_separation,
            familywise_alpha,
        });
    };

    let separation_margin = winner.lower - competitor_upper;
    if separation_margin > minimum_separation {
        Ok(DominanceDecision::CertifiedWinner {
            portfolio_id: winner.portfolio_id.clone(),
            winner_lower_bound: winner.lower,
            best_competitor_upper_bound: Some(competitor_upper),
            separation_margin: Some(separation_margin),
            required_margin: minimum_separation,
            familywise_alpha,
        })
    } else {
        Ok(DominanceDecision::Unresolved {
            reason: DominanceUnresolvedReason::UtilityIntervalsOverlap,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exact(id: &str, value: f64) -> PortfolioUtilityInterval {
        PortfolioUtilityInterval {
            portfolio_id: id.to_owned(),
            lower: value,
            upper: value,
            evidence: UtilityBoundEvidence::Exact,
        }
    }

    fn anytime(id: &str, lower: f64, upper: f64) -> PortfolioUtilityInterval {
        PortfolioUtilityInterval {
            portfolio_id: id.to_owned(),
            lower,
            upper,
            evidence: UtilityBoundEvidence::AnytimeValidFamilywise {
                alpha: 0.01,
                family_receipt_id: "family:all-candidates:v1".to_owned(),
            },
        }
    }

    #[test]
    fn exact_nonoverlapping_utilities_certify_a_winner() {
        let result = certify_maximum_utility(&[exact("A", 0.8), exact("B", 0.6)], 0.1)
            .expect("exact utility comparison");
        match result {
            DominanceDecision::CertifiedWinner {
                portfolio_id,
                winner_lower_bound,
                best_competitor_upper_bound,
                separation_margin,
                required_margin,
                familywise_alpha,
            } => {
                assert_eq!(portfolio_id, "A");
                assert_eq!(winner_lower_bound, 0.8);
                assert_eq!(best_competitor_upper_bound, Some(0.6));
                assert!((separation_margin.expect("margin") - 0.2).abs() < 1.0e-12);
                assert_eq!(required_margin, 0.1);
                assert_eq!(familywise_alpha, None);
            }
            other => panic!("expected certified winner, got {other:?}"),
        }
    }

    #[test]
    fn overlapping_intervals_abstain_instead_of_ranking_point_estimates() {
        let result =
            certify_maximum_utility(&[anytime("A", 0.6, 0.9), anytime("B", 0.7, 0.8)], 0.0)
                .expect("bounded comparison");
        assert_eq!(
            result,
            DominanceDecision::Unresolved {
                reason: DominanceUnresolvedReason::UtilityIntervalsOverlap,
            }
        );
    }

    #[test]
    fn simultaneous_anytime_bounds_can_certify_a_winner() {
        let result =
            certify_maximum_utility(&[anytime("A", 0.8, 0.9), anytime("B", 0.2, 0.6)], 0.1)
                .expect("familywise comparison");
        match result {
            DominanceDecision::CertifiedWinner {
                portfolio_id,
                familywise_alpha,
                ..
            } => {
                assert_eq!(portfolio_id, "A");
                assert_eq!(familywise_alpha, Some(0.01));
            }
            other => panic!("expected certified familywise winner, got {other:?}"),
        }
    }

    #[test]
    fn mismatched_family_receipts_do_not_certify() {
        let mut a = anytime("A", 0.8, 0.9);
        if let UtilityBoundEvidence::AnytimeValidFamilywise {
            family_receipt_id, ..
        } = &mut a.evidence
        {
            *family_receipt_id = "family:A-only".to_owned();
        }
        let result = certify_maximum_utility(&[a, anytime("B", 0.1, 0.4)], 0.0)
            .expect("incompatible evidence result");
        assert_eq!(
            result,
            DominanceDecision::Unresolved {
                reason: DominanceUnresolvedReason::IncompatibleFamilywiseReceipts,
            }
        );
    }

    #[test]
    fn heuristic_or_unqualified_bounds_never_certify_dominance() {
        let heuristic = PortfolioUtilityInterval {
            portfolio_id: "heuristic".to_owned(),
            lower: 0.99,
            upper: 1.0,
            evidence: UtilityBoundEvidence::Heuristic,
        };
        let result = certify_maximum_utility(&[exact("exact", 0.5), heuristic], 0.0)
            .expect("unqualified utility result");
        assert_eq!(
            result,
            DominanceDecision::Unresolved {
                reason: DominanceUnresolvedReason::UnqualifiedOrHeuristicBounds,
            }
        );
    }

    #[test]
    fn theorem_bound_requires_resolvable_assumption_and_enclosure_receipts() {
        let valid = PortfolioUtilityInterval {
            portfolio_id: "theorem".to_owned(),
            lower: 0.7,
            upper: 0.8,
            evidence: UtilityBoundEvidence::TheoremCertified {
                theorem_id: "log-sobolev-eig-v1".to_owned(),
                assumptions_receipt_id: "receipt:assumptions".to_owned(),
                enclosure_receipt_id: "receipt:matrix-enclosure".to_owned(),
            },
        };
        assert!(matches!(
            certify_maximum_utility(&[valid, exact("other", 0.2)], 0.1),
            Ok(DominanceDecision::CertifiedWinner { .. })
        ));

        let invalid = PortfolioUtilityInterval {
            portfolio_id: "invalid-theorem".to_owned(),
            lower: 0.7,
            upper: 0.8,
            evidence: UtilityBoundEvidence::TheoremCertified {
                theorem_id: "log-sobolev-eig-v1".to_owned(),
                assumptions_receipt_id: " ".to_owned(),
                enclosure_receipt_id: "receipt:matrix-enclosure".to_owned(),
            },
        };
        assert_eq!(
            certify_maximum_utility(&[invalid, exact("other", 0.2)], 0.1),
            Err(ModelError::InvalidParameter(
                "theorem_utility_bound_receipts_must_be_nonempty"
            ))
        );
    }

    #[test]
    fn exact_intervals_and_candidate_ids_are_validated() {
        let non_exact = PortfolioUtilityInterval {
            portfolio_id: "bad".to_owned(),
            lower: 0.1,
            upper: 0.2,
            evidence: UtilityBoundEvidence::Exact,
        };
        assert_eq!(
            certify_maximum_utility(&[non_exact], 0.0),
            Err(ModelError::InvalidParameter(
                "exact_utility_interval_must_have_equal_endpoints"
            ))
        );
        assert_eq!(
            certify_maximum_utility(&[], 0.0),
            Err(ModelError::InvalidParameter(
                "utility_interval_candidate_set_must_be_nonempty"
            ))
        );
    }
}
