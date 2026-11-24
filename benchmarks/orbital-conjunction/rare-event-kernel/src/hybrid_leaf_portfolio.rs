// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Finite exact-rational aggregation of structural and statistical event leaves.
//!
//! This module composes already-produced evidence records over a declared
//! finite partition. It validates identity uniqueness, exact target-mass
//! bookkeeping, confidence-budget allocation and the arithmetic risk envelope.
//! It does not verify that opaque partition commitments represent geometrically
//! disjoint/complete sets, that a closure digest proves event-freedom, or that
//! a statistical confidence interval has the claimed coverage.

use std::collections::BTreeSet;

use crate::space_time_partition::{NonnegativeRationalInterval, Rational};

/// Evidence state for one leaf of a finite target-space partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrbitalLeafEvidence {
    /// An independently supplied enclosure claims the entire leaf is event-free.
    ///
    /// The digest binds the closure record but is not verified by this module.
    StructurallyEventFree {
        /// Digest of the external event-exclusion enclosure / proof record.
        closure_digest: [u8; 32],
    },
    /// An independently supplied analytic probability enclosure for this leaf.
    Analytic {
        /// Conditional event-probability interval within the leaf.
        conditional_probability: NonnegativeRationalInterval,
        /// Digest of the external analytic enclosure record.
        enclosure_digest: [u8; 32],
    },
    /// A statistical confidence interval for the conditional event probability.
    Statistical {
        /// Conditional event-probability confidence interval within the leaf.
        conditional_probability: NonnegativeRationalInterval,
        /// Per-leaf one-sided/two-sided familywise error allocation as specified by the producer.
        confidence_alpha: Rational,
        /// Digest of the statistical sample stream and stopping record.
        sample_stream_digest: [u8; 32],
        /// Digest of the complete target/proposal density history used for the stream.
        proposal_history_digest: [u8; 32],
    },
    /// The leaf is not yet closed or statistically resolved.
    ///
    /// Its entire target-mass upper bound is conservatively added to event risk.
    Unresolved {
        /// Digest of the residual-mass calculation for this leaf.
        residual_mass_digest: [u8; 32],
    },
}

/// One leaf record in a finite, externally committed target-space partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrbitalRiskLeaf {
    /// Stable unique identity of this partition leaf.
    pub leaf_id: [u8; 32],
    /// Digest of its parent/ancestry path in the partition tree.
    pub ancestry_digest: [u8; 32],
    /// Target probability mass interval for the entire leaf.
    pub target_mass: NonnegativeRationalInterval,
    /// Exactly one evidence state for this leaf.
    pub evidence: OrbitalLeafEvidence,
}

/// Finite portfolio of partition leaves for one frozen orbital event measure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrbitalHybridLeafPortfolio {
    /// Immutable digest of this portfolio snapshot.
    pub portfolio_digest: [u8; 32],
    /// Identity of the fixed target probability measure.
    pub target_measure_digest: [u8; 32],
    /// Identity of the collision-event definition and conditioning semantics.
    pub event_definition_digest: [u8; 32],
    /// Commitment to the complete leaf partition manifest and ancestry.
    ///
    /// This binds the manifest but does not let this arithmetic module verify
    /// the geometric disjointness or coverage represented by the commitment.
    pub partition_commitment_digest: [u8; 32],
    /// Fixed maximum number of statistical leaves reserved by the familywise budget.
    pub max_leaf_count: usize,
    /// Total allowed confidence error allocation, in [0,1].
    pub global_confidence_alpha: Rational,
    /// All leaves in the committed finite partition.
    pub leaves: Vec<OrbitalRiskLeaf>,
}

/// Exact decomposition of the global risk-envelope upper-minus-lower width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrbitalRiskUncertaintyDecomposition {
    /// Width contributed by per-leaf statistical confidence intervals.
    pub statistical_confidence_width: Rational,
    /// Width contributed by per-leaf analytic probability enclosures.
    pub analytic_enclosure_width: Rational,
    /// Width contributed by uncertainty in target leaf masses for resolved event leaves.
    pub target_mass_uncertainty_width: Rational,
    /// Conservative upper risk contribution from unresolved leaf mass.
    pub unresolved_structural_mass_upper: Rational,
    /// Sum of all component widths before clipping the probability upper bound to one.
    pub uncapped_total_width: Rational,
}

/// Aggregate exact-rational risk envelope and audit counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrbitalHybridRiskEnvelope {
    /// Source portfolio identity.
    pub portfolio_digest: [u8; 32],
    /// Target probability-measure identity.
    pub target_measure_digest: [u8; 32],
    /// Collision-event definition identity.
    pub event_definition_digest: [u8; 32],
    /// Partition manifest commitment.
    pub partition_commitment_digest: [u8; 32],
    /// Number of leaves in the committed partition.
    pub leaf_count: usize,
    /// Number of structurally event-free leaves.
    pub event_free_leaf_count: usize,
    /// Number of analytically bounded leaves.
    pub analytic_leaf_count: usize,
    /// Number of statistically bounded leaves.
    pub statistical_leaf_count: usize,
    /// Number of unresolved leaves.
    pub unresolved_leaf_count: usize,
    /// Sum of per-leaf confidence allocations.
    pub allocated_confidence_alpha: Rational,
    /// Declared global confidence error budget.
    pub global_confidence_alpha: Rational,
    /// Lower bound on total target mass from the leaf mass ledger.
    pub target_mass_lower_sum: Rational,
    /// Upper bound on total target mass from the leaf mass ledger.
    pub target_mass_upper_sum: Rational,
    /// Lower endpoint of the aggregate collision-event probability envelope.
    pub risk_probability_lower: Rational,
    /// Upper endpoint, conservatively clipped to the probability domain [0,1].
    pub risk_probability_upper: Rational,
    /// Unclipped sum of the per-leaf upper risk contributions; may exceed one.
    pub uncapped_risk_probability_upper: Rational,
    /// Decomposition of uncertainty before clipping the upper endpoint to one.
    pub uncertainty: OrbitalRiskUncertaintyDecomposition,
}

/// Fail-closed validation and exact-arithmetic errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrbitalHybridPortfolioError {
    /// A required SHA-256 identity is all zero.
    InvalidDigest,
    /// The committed leaf count exceeds the reserved maximum.
    LeafCountExceedsMaximum,
    /// The fixed maximum leaf count is zero.
    InvalidMaximumLeafCount,
    /// A leaf identity is duplicated.
    DuplicateLeafIdentity,
    /// A leaf identity or ancestry digest is all zero.
    InvalidLeafIdentity,
    /// A target-mass interval is invalid or exceeds one.
    InvalidTargetMassInterval,
    /// Conditional event-probability interval is invalid or exceeds one.
    InvalidConditionalProbabilityInterval,
    /// Target-mass lower/upper sums do not enclose unit total mass.
    TargetMassLedgerDoesNotEncloseOne,
    /// Global confidence alpha must lie strictly between zero and one.
    InvalidGlobalConfidenceAlpha,
    /// A statistical leaf has an invalid confidence allocation.
    InvalidLeafConfidenceAlpha,
    /// Sum of leaf confidence allocations exceeds the global budget.
    ConfidenceAlphaBudgetExceeded,
    /// A required evidence record digest is all zero.
    MissingEvidenceDigest,
    /// A required rational subtraction, multiplication or addition overflowed.
    ArithmeticOverflow,
    /// Exact aggregation produced a risk lower endpoint greater than its upper endpoint.
    InvalidAggregateRiskEnvelope,
}

impl core::fmt::Display for OrbitalHybridPortfolioError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidDigest => "a portfolio identity digest is invalid",
            Self::LeafCountExceedsMaximum => "leaf count exceeds the declared maximum",
            Self::InvalidMaximumLeafCount => "maximum leaf count must be nonzero",
            Self::DuplicateLeafIdentity => "partition leaf identity is duplicated",
            Self::InvalidLeafIdentity => "partition leaf identity or ancestry digest is invalid",
            Self::InvalidTargetMassInterval => "target mass interval is invalid",
            Self::InvalidConditionalProbabilityInterval => {
                "conditional event probability interval is invalid"
            }
            Self::TargetMassLedgerDoesNotEncloseOne => {
                "sum of leaf target-mass intervals does not enclose unit mass"
            }
            Self::InvalidGlobalConfidenceAlpha => {
                "global confidence alpha must lie strictly between zero and one"
            }
            Self::InvalidLeafConfidenceAlpha => {
                "statistical leaf confidence alpha must lie strictly between zero and one"
            }
            Self::ConfidenceAlphaBudgetExceeded => {
                "sum of per-leaf confidence allocations exceeds global alpha"
            }
            Self::MissingEvidenceDigest => "a required leaf evidence digest is missing",
            Self::ArithmeticOverflow => "exact rational arithmetic overflowed",
            Self::InvalidAggregateRiskEnvelope => "aggregate risk interval is invalid",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for OrbitalHybridPortfolioError {}

/// Aggregate a finite structural/statistical leaf portfolio using exact rationals.
///
/// Resolved leaves contribute [mass.lower * probability.lower,
/// mass.upper * probability.upper]. Structurally event-free leaves contribute
/// zero. Unresolved leaves contribute [0, mass.upper]. Statistical leaf alpha
/// allocations are summed and checked against the portfolio's global budget.
///
/// The target-mass ledger must satisfy sum(lower) <= 1 <= sum(upper), and the
/// manifest must be committed by a nonzero digest. These checks establish exact
/// arithmetic consistency only. They do not prove that leaves form a disjoint
/// geometric partition, that event-free/analytic evidence is sound, or that
/// statistical intervals have their declared coverage. Those records require
/// independent producers and replay/qualification outside this function.
pub fn aggregate_orbital_hybrid_leaf_portfolio(
    portfolio: &OrbitalHybridLeafPortfolio,
) -> Result<OrbitalHybridRiskEnvelope, OrbitalHybridPortfolioError> {
    if is_zero_digest(&portfolio.portfolio_digest)
        || is_zero_digest(&portfolio.target_measure_digest)
        || is_zero_digest(&portfolio.event_definition_digest)
        || is_zero_digest(&portfolio.partition_commitment_digest)
    {
        return Err(OrbitalHybridPortfolioError::InvalidDigest);
    }
    if portfolio.max_leaf_count == 0 {
        return Err(OrbitalHybridPortfolioError::InvalidMaximumLeafCount);
    }
    if portfolio.leaves.len() > portfolio.max_leaf_count {
        return Err(OrbitalHybridPortfolioError::LeafCountExceedsMaximum);
    }
    let zero = Rational::from_integer(0);
    let one = Rational::from_integer(1);
    if portfolio.global_confidence_alpha <= zero || portfolio.global_confidence_alpha >= one {
        return Err(OrbitalHybridPortfolioError::InvalidGlobalConfidenceAlpha);
    }

    let mut leaf_ids = BTreeSet::new();
    let mut ordered_leaves = portfolio.leaves.clone();
    ordered_leaves.sort_by_key(|leaf| leaf.leaf_id);

    let mut mass_lower_sum = zero;
    let mut mass_upper_sum = zero;
    let mut allocated_alpha = zero;
    let mut risk_lower = zero;
    let mut uncapped_risk_upper = zero;
    let mut statistical_width = zero;
    let mut analytic_width = zero;
    let mut target_mass_width = zero;
    let mut unresolved_mass_upper = zero;
    let mut event_free_leaf_count = 0_usize;
    let mut analytic_leaf_count = 0_usize;
    let mut statistical_leaf_count = 0_usize;
    let mut unresolved_leaf_count = 0_usize;

    for leaf in &ordered_leaves {
        if is_zero_digest(&leaf.leaf_id) || is_zero_digest(&leaf.ancestry_digest) {
            return Err(OrbitalHybridPortfolioError::InvalidLeafIdentity);
        }
        if !leaf_ids.insert(leaf.leaf_id) {
            return Err(OrbitalHybridPortfolioError::DuplicateLeafIdentity);
        }
        if !validate_interval(leaf.target_mass, one) {
            return Err(OrbitalHybridPortfolioError::InvalidTargetMassInterval);
        }
        mass_lower_sum = mass_lower_sum
            .checked_add(leaf.target_mass.lower)
            .map_err(|_| OrbitalHybridPortfolioError::ArithmeticOverflow)?;
        mass_upper_sum = mass_upper_sum
            .checked_add(leaf.target_mass.upper)
            .map_err(|_| OrbitalHybridPortfolioError::ArithmeticOverflow)?;

        match leaf.evidence {
            OrbitalLeafEvidence::StructurallyEventFree { closure_digest } => {
                if is_zero_digest(&closure_digest) {
                    return Err(OrbitalHybridPortfolioError::MissingEvidenceDigest);
                }
                event_free_leaf_count = event_free_leaf_count
                    .checked_add(1)
                    .ok_or(OrbitalHybridPortfolioError::ArithmeticOverflow)?;
            }
            OrbitalLeafEvidence::Analytic {
                conditional_probability,
                enclosure_digest,
            } => {
                if is_zero_digest(&enclosure_digest) {
                    return Err(OrbitalHybridPortfolioError::MissingEvidenceDigest);
                }
                if !validate_interval(conditional_probability, one) {
                    return Err(OrbitalHybridPortfolioError::InvalidConditionalProbabilityInterval);
                }
                let lower_contribution = leaf
                    .target_mass
                    .lower
                    .checked_mul(conditional_probability.lower)
                    .map_err(|_| OrbitalHybridPortfolioError::ArithmeticOverflow)?;
                let upper_contribution = leaf
                    .target_mass
                    .upper
                    .checked_mul(conditional_probability.upper)
                    .map_err(|_| OrbitalHybridPortfolioError::ArithmeticOverflow)?;
                risk_lower = checked_add(risk_lower, lower_contribution)?;
                uncapped_risk_upper = checked_add(uncapped_risk_upper, upper_contribution)?;

                let probability_width =
                    checked_sub(conditional_probability.upper, conditional_probability.lower)?;
                let mass_width = checked_sub(leaf.target_mass.upper, leaf.target_mass.lower)?;
                analytic_width = checked_add(
                    analytic_width,
                    leaf.target_mass
                        .upper
                        .checked_mul(probability_width)
                        .map_err(|_| OrbitalHybridPortfolioError::ArithmeticOverflow)?,
                )?;
                target_mass_width = checked_add(
                    target_mass_width,
                    mass_width
                        .checked_mul(conditional_probability.lower)
                        .map_err(|_| OrbitalHybridPortfolioError::ArithmeticOverflow)?,
                )?;
                analytic_leaf_count = analytic_leaf_count
                    .checked_add(1)
                    .ok_or(OrbitalHybridPortfolioError::ArithmeticOverflow)?;
            }
            OrbitalLeafEvidence::Statistical {
                conditional_probability,
                confidence_alpha,
                sample_stream_digest,
                proposal_history_digest,
            } => {
                if is_zero_digest(&sample_stream_digest) || is_zero_digest(&proposal_history_digest) {
                    return Err(OrbitalHybridPortfolioError::MissingEvidenceDigest);
                }
                if confidence_alpha <= zero || confidence_alpha >= one {
                    return Err(OrbitalHybridPortfolioError::InvalidLeafConfidenceAlpha);
                }
                if !validate_interval(conditional_probability, one) {
                    return Err(
                        OrbitalHybridPortfolioError::InvalidConditionalProbabilityInterval
                    );
                }
                allocated_alpha = checked_add(allocated_alpha, confidence_alpha)?;
                if allocated_alpha > portfolio.global_confidence_alpha {
                    return Err(OrbitalHybridPortfolioError::ConfidenceAlphaBudgetExceeded);
                }

                let lower_contribution = leaf
                    .target_mass
                    .lower
                    .checked_mul(conditional_probability.lower)
                    .map_err(|_| OrbitalHybridPortfolioError::ArithmeticOverflow)?;
                let upper_contribution = leaf
                    .target_mass
                    .upper
                    .checked_mul(conditional_probability.upper)
                    .map_err(|_| OrbitalHybridPortfolioError::ArithmeticOverflow)?;
                risk_lower = checked_add(risk_lower, lower_contribution)?;
                uncapped_risk_upper = checked_add(uncapped_risk_upper, upper_contribution)?;

                let probability_width = checked_sub(
                    conditional_probability.upper,
                    conditional_probability.lower,
                )?;
                let mass_width =
                    checked_sub(leaf.target_mass.upper, leaf.target_mass.lower)?;
                statistical_width = checked_add(
                    statistical_width,
                    leaf.target_mass
                        .upper
                        .checked_mul(probability_width)
                        .map_err(|_| OrbitalHybridPortfolioError::ArithmeticOverflow)?,
                )?;
                target_mass_width = checked_add(
                    target_mass_width,
                    mass_width
                        .checked_mul(conditional_probability.lower)
                        .map_err(|_| OrbitalHybridPortfolioError::ArithmeticOverflow)?,
                )?;
                statistical_leaf_count = statistical_leaf_count
                    .checked_add(1)
                    .ok_or(OrbitalHybridPortfolioError::ArithmeticOverflow)?;
            }
            OrbitalLeafEvidence::Unresolved {
                residual_mass_digest,
            } => {
                if is_zero_digest(&residual_mass_digest) {
                    return Err(OrbitalHybridPortfolioError::MissingEvidenceDigest);
                }
                unresolved_mass_upper = checked_add(unresolved_mass_upper, leaf.target_mass.upper)?;
                uncapped_risk_upper = checked_add(uncapped_risk_upper, leaf.target_mass.upper)?;
                unresolved_leaf_count = unresolved_leaf_count
                    .checked_add(1)
                    .ok_or(OrbitalHybridPortfolioError::ArithmeticOverflow)?;
            }
        }
    }

    if mass_lower_sum > one || mass_upper_sum < one {
        return Err(OrbitalHybridPortfolioError::TargetMassLedgerDoesNotEncloseOne);
    }
    if risk_lower > uncapped_risk_upper {
        return Err(OrbitalHybridPortfolioError::InvalidAggregateRiskEnvelope);
    }

    let total_width = checked_sub(uncapped_risk_upper, risk_lower)?;
    let decomposition_sum = checked_add(
        checked_add(
            checked_add(statistical_width, analytic_width)?,
            target_mass_width,
        )?,
        unresolved_mass_upper,
    )?;
    if total_width != decomposition_sum {
        return Err(OrbitalHybridPortfolioError::InvalidAggregateRiskEnvelope);
    }

    Ok(OrbitalHybridRiskEnvelope {
        portfolio_digest: portfolio.portfolio_digest,
        target_measure_digest: portfolio.target_measure_digest,
        event_definition_digest: portfolio.event_definition_digest,
        partition_commitment_digest: portfolio.partition_commitment_digest,
        leaf_count: ordered_leaves.len(),
        event_free_leaf_count,
        analytic_leaf_count,
        statistical_leaf_count,
        unresolved_leaf_count,
        allocated_confidence_alpha: allocated_alpha,
        global_confidence_alpha: portfolio.global_confidence_alpha,
        target_mass_lower_sum: mass_lower_sum,
        target_mass_upper_sum: mass_upper_sum,
        risk_probability_lower: risk_lower,
        risk_probability_upper: uncapped_risk_upper.min(one),
        uncapped_risk_probability_upper: uncapped_risk_upper,
        uncertainty: OrbitalRiskUncertaintyDecomposition {
            statistical_confidence_width: statistical_width,
            analytic_enclosure_width: analytic_width,
            target_mass_uncertainty_width: target_mass_width,
            unresolved_structural_mass_upper: unresolved_mass_upper,
            uncapped_total_width: total_width,
        },
    })
}

fn validate_interval(interval: NonnegativeRationalInterval, maximum: Rational) -> bool {
    interval.lower <= interval.upper && interval.upper <= maximum
}

fn checked_add(left: Rational, right: Rational) -> Result<Rational, OrbitalHybridPortfolioError> {
    left.checked_add(right)
        .map_err(|_| OrbitalHybridPortfolioError::ArithmeticOverflow)
}

fn checked_sub(left: Rational, right: Rational) -> Result<Rational, OrbitalHybridPortfolioError> {
    left.checked_sub(right)
        .map_err(|_| OrbitalHybridPortfolioError::ArithmeticOverflow)
}

fn is_zero_digest(digest: &[u8; 32]) -> bool {
    digest.iter().all(|byte| *byte == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PORTFOLIO: [u8; 32] = [1; 32];
    const TARGET: [u8; 32] = [2; 32];
    const EVENT: [u8; 32] = [3; 32];
    const PARTITION: [u8; 32] = [4; 32];

    fn rational(numerator: u128, denominator: u128) -> Rational {
        Rational::new(numerator, denominator).unwrap()
    }

    fn interval(lower: (u128, u128), upper: (u128, u128)) -> NonnegativeRationalInterval {
        NonnegativeRationalInterval {
            lower: rational(lower.0, lower.1),
            upper: rational(upper.0, upper.1),
        }
    }

    fn leaf(
        id: u8,
        mass: ((u128, u128), (u128, u128)),
        evidence: OrbitalLeafEvidence,
    ) -> OrbitalRiskLeaf {
        OrbitalRiskLeaf {
            leaf_id: [id; 32],
            ancestry_digest: [id + 20; 32],
            target_mass: interval(mass.0, mass.1),
            evidence,
        }
    }

    fn portfolio(leaves: Vec<OrbitalRiskLeaf>) -> OrbitalHybridLeafPortfolio {
        OrbitalHybridLeafPortfolio {
            portfolio_digest: PORTFOLIO,
            target_measure_digest: TARGET,
            event_definition_digest: EVENT,
            partition_commitment_digest: PARTITION,
            max_leaf_count: 8,
            global_confidence_alpha: rational(1, 20),
            leaves,
        }
    }

    #[test]
    fn hybrid_portfolio_composes_statistical_analytic_and_unresolved_mass() {
        let result = aggregate_orbital_hybrid_leaf_portfolio(&portfolio(vec![
            leaf(
                10,
                ((1, 4), (1, 4)),
                OrbitalLeafEvidence::StructurallyEventFree {
                    closure_digest: [30; 32],
                },
            ),
            leaf(
                11,
                ((1, 2), (1, 2)),
                OrbitalLeafEvidence::Statistical {
                    conditional_probability: interval((1, 10), (1, 5)),
                    confidence_alpha: rational(1, 20),
                    sample_stream_digest: [31; 32],
                    proposal_history_digest: [32; 32],
                },
            ),
            leaf(
                12,
                ((1, 4), (1, 4)),
                OrbitalLeafEvidence::Unresolved {
                    residual_mass_digest: [33; 32],
                },
            ),
        ]))
        .unwrap();

        assert_eq!(result.risk_probability_lower, rational(1, 20));
        assert_eq!(result.risk_probability_upper, rational(7, 20));
        assert_eq!(
            result.uncertainty.statistical_confidence_width,
            rational(1, 20)
        );
        assert_eq!(
            result.uncertainty.unresolved_structural_mass_upper,
            rational(1, 4)
        );
        assert_eq!(result.uncertainty.uncapped_total_width, rational(3, 10));
        assert_eq!(result.event_free_leaf_count, 1);
        assert_eq!(result.statistical_leaf_count, 1);
        assert_eq!(result.unresolved_leaf_count, 1);
    }

    #[test]
    fn analytic_probability_and_target_mass_uncertainty_are_decomposed_exactly() {
        let result = aggregate_orbital_hybrid_leaf_portfolio(&portfolio(vec![leaf(
            10,
            ((1, 4), (1, 2)),
            OrbitalLeafEvidence::Analytic {
                conditional_probability: interval((1, 5), (2, 5)),
                enclosure_digest: [30; 32],
            },
        )]))
        .unwrap();

        assert_eq!(result.risk_probability_lower, rational(1, 20));
        assert_eq!(result.uncapped_risk_probability_upper, rational(1, 5));
        assert_eq!(result.uncertainty.analytic_enclosure_width, rational(1, 10));
        assert_eq!(
            result.uncertainty.target_mass_uncertainty_width,
            rational(1, 20)
        );
        assert_eq!(result.uncertainty.uncapped_total_width, rational(3, 20));
        assert_eq!(result.risk_probability_upper, rational(1, 5));
    }

    #[test]
    fn multiple_statistical_leaf_allocations_must_fit_global_alpha() {
        let mut input = portfolio(vec![
            leaf(
                10,
                ((1, 2), (1, 2)),
                OrbitalLeafEvidence::Statistical {
                    conditional_probability: interval((0, 1), (1, 1)),
                    confidence_alpha: rational(3, 100),
                    sample_stream_digest: [30; 32],
                    proposal_history_digest: [31; 32],
                },
            ),
            leaf(
                11,
                ((1, 2), (1, 2)),
                OrbitalLeafEvidence::Statistical {
                    conditional_probability: interval((0, 1), (1, 1)),
                    confidence_alpha: rational(3, 100),
                    sample_stream_digest: [32; 32],
                    proposal_history_digest: [33; 32],
                },
            ),
        ]);
        input.global_confidence_alpha = rational(1, 20);
        assert_eq!(
            aggregate_orbital_hybrid_leaf_portfolio(&input),
            Err(OrbitalHybridPortfolioError::ConfidenceAlphaBudgetExceeded)
        );
    }

    #[test]
    fn statistical_alpha_and_maximum_leaf_count_are_checked() {
        let mut input = portfolio(vec![leaf(
            10,
            ((1, 1), (1, 1)),
            OrbitalLeafEvidence::Statistical {
                conditional_probability: interval((0, 1), (1, 1)),
                confidence_alpha: rational(0, 1),
                sample_stream_digest: [30; 32],
                proposal_history_digest: [31; 32],
            },
        )]);
        assert_eq!(
            aggregate_orbital_hybrid_leaf_portfolio(&input),
            Err(OrbitalHybridPortfolioError::InvalidLeafConfidenceAlpha)
        );

        input.max_leaf_count = 0;
        assert_eq!(
            aggregate_orbital_hybrid_leaf_portfolio(&input),
            Err(OrbitalHybridPortfolioError::InvalidMaximumLeafCount)
        );
    }

    #[test]
    fn duplicate_leaves_and_missing_commitments_fail_closed() {
        let first = leaf(
            10,
            ((1, 2), (1, 2)),
            OrbitalLeafEvidence::StructurallyEventFree {
                closure_digest: [30; 32],
            },
        );
        let mut input = portfolio(vec![first, first]);
        assert_eq!(
            aggregate_orbital_hybrid_leaf_portfolio(&input),
            Err(OrbitalHybridPortfolioError::DuplicateLeafIdentity)
        );

        input = portfolio(vec![first]);
        input.partition_commitment_digest = [0; 32];
        assert_eq!(
            aggregate_orbital_hybrid_leaf_portfolio(&input),
            Err(OrbitalHybridPortfolioError::InvalidDigest)
        );
    }

    #[test]
    fn unresolved_residual_mass_is_never_discarded() {
        let result = aggregate_orbital_hybrid_leaf_portfolio(&portfolio(vec![leaf(
            10,
            ((1, 10), (1, 10)),
            OrbitalLeafEvidence::Unresolved {
                residual_mass_digest: [30; 32],
            },
        )]))
        .unwrap();

        assert_eq!(result.risk_probability_lower, rational(0, 1));
        assert_eq!(result.risk_probability_upper, rational(1, 10));
    }

    #[test]
    fn target_mass_ledger_must_enclose_one() {
        let input = portfolio(vec![leaf(
            10,
            ((1, 4), (1, 2)),
            OrbitalLeafEvidence::StructurallyEventFree {
                closure_digest: [30; 32],
            },
        )]);
        assert_eq!(
            aggregate_orbital_hybrid_leaf_portfolio(&input),
            Err(OrbitalHybridPortfolioError::TargetMassLedgerDoesNotEncloseOne)
        );
    }

    #[test]
    fn conditional_probability_and_mass_intervals_must_be_in_unit_domain() {
        let invalid_probability = portfolio(vec![leaf(
            10,
            ((1, 1), (1, 1)),
            OrbitalLeafEvidence::Analytic {
                conditional_probability: interval((1, 2), (2, 1)),
                enclosure_digest: [30; 32],
            },
        )]);
        assert_eq!(
            aggregate_orbital_hybrid_leaf_portfolio(&invalid_probability),
            Err(OrbitalHybridPortfolioError::InvalidConditionalProbabilityInterval)
        );

        let invalid_mass = portfolio(vec![leaf(
            10,
            ((1, 2), (3, 2)),
            OrbitalLeafEvidence::StructurallyEventFree {
                closure_digest: [30; 32],
            },
        )]);
        // A mass upper bound greater than one is invalid even if all evidence is event-free.
        assert_eq!(
            aggregate_orbital_hybrid_leaf_portfolio(&invalid_mass),
            Err(OrbitalHybridPortfolioError::InvalidTargetMassInterval)
        );
    }

    #[test]
    fn portfolio_rejects_missing_leaf_evidence_and_invalid_global_alpha() {
        let mut input = portfolio(vec![leaf(
            10,
            ((1, 1), (1, 1)),
            OrbitalLeafEvidence::StructurallyEventFree {
                closure_digest: [0; 32],
            },
        )]);
        assert_eq!(
            aggregate_orbital_hybrid_leaf_portfolio(&input),
            Err(OrbitalHybridPortfolioError::MissingEvidenceDigest)
        );

        input = portfolio(vec![leaf(
            10,
            ((1, 1), (1, 1)),
            OrbitalLeafEvidence::StructurallyEventFree {
                closure_digest: [30; 32],
            },
        )]);
        input.global_confidence_alpha = rational(1, 1);
        assert_eq!(
            aggregate_orbital_hybrid_leaf_portfolio(&input),
            Err(OrbitalHybridPortfolioError::InvalidGlobalConfidenceAlpha)
        );
    }

    #[test]
    fn partition_leaf_count_cannot_exceed_reserved_maximum() {
        let one_leaf = leaf(
            10,
            ((1, 2), (1, 2)),
            OrbitalLeafEvidence::StructurallyEventFree {
                closure_digest: [30; 32],
            },
        );
        let mut input = portfolio(vec![
            one_leaf,
            leaf(
                11,
                ((1, 2), (1, 2)),
                OrbitalLeafEvidence::StructurallyEventFree {
                    closure_digest: [31; 32],
                },
            ),
        ]);
        input.max_leaf_count = 1;
        assert_eq!(
            aggregate_orbital_hybrid_leaf_portfolio(&input),
            Err(OrbitalHybridPortfolioError::LeafCountExceedsMaximum)
        );
    }

    #[test]
    fn aggregate_probability_upper_bound_is_clipped_to_one() {
        let result = aggregate_orbital_hybrid_leaf_portfolio(&portfolio(vec![
            leaf(
                10,
                ((1, 2), (1, 1)),
                OrbitalLeafEvidence::Unresolved {
                    residual_mass_digest: [30; 32],
                },
            ),
            leaf(
                11,
                ((0, 1), (1, 1)),
                OrbitalLeafEvidence::Unresolved {
                    residual_mass_digest: [31; 32],
                },
            ),
        ]))
        .unwrap();

        assert_eq!(result.risk_probability_upper, rational(1, 1));
        assert_eq!(result.uncapped_risk_probability_upper, rational(2, 1));
        assert_eq!(result.uncertainty.uncapped_total_width, rational(2, 1));
    }
}
