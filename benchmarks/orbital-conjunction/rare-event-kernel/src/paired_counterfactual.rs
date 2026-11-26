// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact finite-support comparison of paired counterfactual consequences.
//!
//! This module treats one latent uncertainty atom as one joint observation
//! across a baseline arm and one action arm. It computes the exact population
//! variance of the paired consequence difference and compares it with the
//! variance that would result from independent draws from the same marginal
//! finite distributions.
//!
//! It is an arithmetic reference kernel only. It does not generate samples,
//! prove that atom identities represent the same physical uncertainty,
//! propagate an orbit, attach importance weights, construct confidence
//! sequences, or qualify a real-world decision.

use std::collections::{BTreeMap, BTreeSet};

use crate::space_time_partition::{Rational, SpaceTimePartitionError};

/// Maximum number of atoms accepted by the exact finite-support reference kernel.
pub const MAX_PAIRED_COUNTERFACTUAL_ATOMS: usize = 65_536;
/// Maximum candidate portfolio arms in one exact multi-arm comparison.
pub const MAX_PAIRED_COUNTERFACTUAL_PORTFOLIOS: usize = 32;

/// One joint observation of a baseline and an action portfolio on the same
/// exogenous latent uncertainty atom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairedCounterfactualAtom {
    /// Stable, nonzero identity of the latent uncertainty atom.
    pub atom_id: [u8; 32],
    /// Baseline consequence in the declared integer tick unit.
    pub baseline_consequence_ticks: i32,
    /// Directly evaluated action-portfolio consequence on the same latent atom.
    pub portfolio_consequence_ticks: i32,
}

/// A reduced signed rational number with a strictly positive denominator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignedRational {
    /// Reduced signed numerator.
    pub numerator: i128,
    /// Reduced positive denominator.
    pub denominator: u128,
}

impl SignedRational {
    fn new(numerator: i128, denominator: u128) -> Result<Self, PairedCounterfactualError> {
        if denominator == 0 {
            return Err(PairedCounterfactualError::ArithmeticOverflow);
        }
        let divisor = gcd(numerator.unsigned_abs(), denominator);
        let divisor_i128 =
            i128::try_from(divisor).map_err(|_| PairedCounterfactualError::ArithmeticOverflow)?;
        Ok(Self {
            numerator: numerator / divisor_i128,
            denominator: denominator / divisor,
        })
    }
}

/// Direction of the paired-versus-independent variance comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairingEffect {
    /// Pairing reduces the finite-support variance of the action difference.
    VarianceReduction,
    /// Pairing leaves the finite-support variance unchanged.
    NoVarianceChange,
    /// Pairing increases the finite-support variance of the action difference.
    VarianceInflation,
    /// Both marginal arms have zero variance, so a variance ratio is undefined.
    ZeroIndependentVariance,
}

/// Exact comparison of paired and independent-arm finite-support variances.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairedCounterfactualReport {
    /// Number of unique, equally weighted latent atoms.
    pub atom_count: usize,
    /// Exact population mean of baseline consequence minus portfolio consequence.
    pub mean_consequence_avoided_ticks: SignedRational,
    /// Exact population variance of the paired baseline-minus-portfolio difference.
    pub paired_difference_population_variance: Rational,
    /// Exact variance of the difference under independent draws from the same
    /// baseline and portfolio marginal finite distributions.
    pub independent_arm_difference_population_variance: Rational,
    /// Exact covariance between baseline and portfolio consequences on paired atoms.
    pub baseline_portfolio_covariance: SignedRational,
    /// Paired variance divided by independent-arm variance, when the latter is nonzero.
    pub paired_to_independent_variance_ratio: Option<Rational>,
    /// Whether the selected coupling reduces, preserves or increases variance.
    pub pairing_effect: PairingEffect,
}

/// Validation and arithmetic failures for paired finite-support comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PairedCounterfactualError {
    /// The finite-support atom set is empty.
    EmptySupport,
    /// The atom count exceeds the bounded reference-kernel limit.
    TooManyAtoms,
    /// No candidate portfolio arms were supplied.
    EmptyPortfolioUniverse,
    /// The candidate portfolio universe exceeds the bounded multi-arm limit.
    TooManyPortfolios,
    /// Atoms do not contain the same exact candidate portfolio identity set.
    InconsistentPortfolioUniverse,
    /// A portfolio identity is zero or duplicated within one atom.
    InvalidOrDuplicatePortfolioIdentity,
    /// A latent atom identity is zero or duplicated.
    InvalidOrDuplicateAtomIdentity,
    /// Exact integer arithmetic exceeded the representable range.
    ArithmeticOverflow,
    /// A variance numerator was negative despite exact sum-of-squares arithmetic.
    InconsistentVariance,
    /// The underlying exact rational kernel rejected an operation.
    InvalidRationalArithmetic,
}

impl core::fmt::Display for PairedCounterfactualError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::EmptySupport => "paired counterfactual support is empty",
            Self::TooManyAtoms => "paired counterfactual support exceeds the finite bound",
            Self::EmptyPortfolioUniverse => "paired counterfactual portfolio universe is empty",
            Self::TooManyPortfolios => "paired counterfactual portfolio universe exceeds the finite bound",
            Self::InconsistentPortfolioUniverse => {
                "paired counterfactual atoms contain inconsistent portfolio identities"
            }
            Self::InvalidOrDuplicatePortfolioIdentity => {
                "paired counterfactual portfolio identity is zero or duplicated"
            }
            Self::InvalidOrDuplicateAtomIdentity => {
                "paired counterfactual atom identity is zero or duplicated"
            }
            Self::ArithmeticOverflow => "paired counterfactual exact arithmetic overflowed",
            Self::InconsistentVariance => "exact finite-support variance became negative",
            Self::InvalidRationalArithmetic => "exact rational variance arithmetic failed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PairedCounterfactualError {}

impl From<SpaceTimePartitionError> for PairedCounterfactualError {
    fn from(_: SpaceTimePartitionError) -> Self {
        Self::InvalidRationalArithmetic
    }
}

/// Compute exact paired and independent-arm population variances on a complete
/// equally weighted finite support.
///
/// The row identity represents the *same exogenous uncertainty atom* for both
/// arms. Each portfolio consequence must already have been evaluated directly
/// for that arm; this function never adds singleton-action effects or reuses
/// the baseline event classification as the post-action classification.
///
/// The independent-arm comparator uses the same marginal baseline and
/// portfolio finite distributions but removes their cross-arm covariance:
///
/// \[
/// \operatorname{Var}(X-Y)_{\mathrm{independent}}
/// = \operatorname{Var}(X)+\operatorname{Var}(Y).
/// \]
///
/// The paired variance is computed directly from the observed per-atom
/// differences. All means, variances and covariance are exact rational values.
/// Each row has equal mass; weighted or importance-sampled supports require a
/// separate estimator and are deliberately rejected by omission from this API.
///
/// This establishes only finite arithmetic. It does not establish that the
/// support is representative of a continuous orbital distribution or that
/// atom identities are valid physical couplings.
pub fn compare_paired_counterfactual_support(
    atoms: &[PairedCounterfactualAtom],
) -> Result<PairedCounterfactualReport, PairedCounterfactualError> {
    if atoms.is_empty() {
        return Err(PairedCounterfactualError::EmptySupport);
    }
    if atoms.len() > MAX_PAIRED_COUNTERFACTUAL_ATOMS {
        return Err(PairedCounterfactualError::TooManyAtoms);
    }

    let mut identities = BTreeSet::new();
    let mut sum_baseline = 0_i128;
    let mut sum_portfolio = 0_i128;
    let mut sum_difference = 0_i128;
    let mut sum_baseline_squared = 0_i128;
    let mut sum_portfolio_squared = 0_i128;
    let mut sum_difference_squared = 0_i128;
    let mut sum_cross_product = 0_i128;

    for atom in atoms {
        if atom.atom_id.iter().all(|byte| *byte == 0) || !identities.insert(atom.atom_id) {
            return Err(PairedCounterfactualError::InvalidOrDuplicateAtomIdentity);
        }

        let baseline = i128::from(atom.baseline_consequence_ticks);
        let portfolio = i128::from(atom.portfolio_consequence_ticks);
        let difference = baseline
            .checked_sub(portfolio)
            .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;

        sum_baseline = sum_baseline
            .checked_add(baseline)
            .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;
        sum_portfolio = sum_portfolio
            .checked_add(portfolio)
            .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;
        sum_difference = sum_difference
            .checked_add(difference)
            .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;

        sum_baseline_squared = sum_baseline_squared
            .checked_add(
                baseline
                    .checked_mul(baseline)
                    .ok_or(PairedCounterfactualError::ArithmeticOverflow)?,
            )
            .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;
        sum_portfolio_squared = sum_portfolio_squared
            .checked_add(
                portfolio
                    .checked_mul(portfolio)
                    .ok_or(PairedCounterfactualError::ArithmeticOverflow)?,
            )
            .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;
        sum_difference_squared = sum_difference_squared
            .checked_add(
                difference
                    .checked_mul(difference)
                    .ok_or(PairedCounterfactualError::ArithmeticOverflow)?,
            )
            .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;
        sum_cross_product = sum_cross_product
            .checked_add(
                baseline
                    .checked_mul(portfolio)
                    .ok_or(PairedCounterfactualError::ArithmeticOverflow)?,
            )
            .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;
    }

    let count =
        i128::try_from(atoms.len()).map_err(|_| PairedCounterfactualError::ArithmeticOverflow)?;
    let denominator = (atoms.len() as u128)
        .checked_mul(atoms.len() as u128)
        .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;

    let baseline_variance_numerator =
        centered_sum_squares_numerator(count, sum_baseline, sum_baseline_squared)?;
    let portfolio_variance_numerator =
        centered_sum_squares_numerator(count, sum_portfolio, sum_portfolio_squared)?;
    let paired_variance_numerator =
        centered_sum_squares_numerator(count, sum_difference, sum_difference_squared)?;

    let independent_variance_numerator = baseline_variance_numerator
        .checked_add(portfolio_variance_numerator)
        .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;

    let paired_variance = Rational::new(paired_variance_numerator, denominator)?;
    let independent_variance = Rational::new(independent_variance_numerator, denominator)?;
    let covariance_numerator = count
        .checked_mul(sum_cross_product)
        .and_then(|value| {
            sum_baseline
                .checked_mul(sum_portfolio)
                .and_then(|product| value.checked_sub(product))
        })
        .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;

    let ratio = if independent_variance.numerator() == 0 {
        None
    } else {
        Some(paired_variance.checked_div(independent_variance)?)
    };
    let pairing_effect = match paired_variance.cmp(&independent_variance) {
        std::cmp::Ordering::Less => PairingEffect::VarianceReduction,
        std::cmp::Ordering::Equal if independent_variance.numerator() == 0 => {
            PairingEffect::ZeroIndependentVariance
        }
        std::cmp::Ordering::Equal => PairingEffect::NoVarianceChange,
        std::cmp::Ordering::Greater => PairingEffect::VarianceInflation,
    };

    Ok(PairedCounterfactualReport {
        atom_count: atoms.len(),
        mean_consequence_avoided_ticks: SignedRational::new(sum_difference, atoms.len() as u128)?,
        paired_difference_population_variance: paired_variance,
        independent_arm_difference_population_variance: independent_variance,
        baseline_portfolio_covariance: SignedRational::new(covariance_numerator, denominator)?,
        paired_to_independent_variance_ratio: ratio,
        pairing_effect,
    })
}

/// Direct consequence for one candidate portfolio on one latent atom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortfolioConsequence {
    /// Stable, nonzero identity of the exact candidate portfolio.
    pub portfolio_id: [u8; 32],
    /// Directly evaluated consequence in the declared integer tick unit.
    pub consequence_ticks: i32,
}

/// One joint observation of the baseline and every candidate portfolio on the
/// same exogenous latent uncertainty atom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiPortfolioPairedAtom {
    /// Stable, nonzero identity of the latent uncertainty atom.
    pub atom_id: [u8; 32],
    /// Baseline consequence in the declared integer tick unit.
    pub baseline_consequence_ticks: i32,
    /// Exactly one direct consequence for every portfolio in the frozen universe.
    ///
    /// The vector may arrive in any order; the evaluator canonicalizes it by
    /// portfolio identity and rejects missing, extra, zero or duplicate identities.
    pub portfolio_consequences: Vec<PortfolioConsequence>,
}

/// Exact paired comparison of one candidate portfolio against the baseline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortfolioCounterfactualSummary {
    /// Stable identity of the candidate portfolio.
    pub portfolio_id: [u8; 32],
    /// Exact population mean of baseline consequence minus portfolio consequence.
    pub mean_consequence_avoided_ticks: SignedRational,
    /// Exact population variance of the paired baseline-minus-portfolio difference.
    pub paired_difference_population_variance: Rational,
    /// Difference variance under independent draws from the same two marginals.
    pub independent_arm_difference_population_variance: Rational,
    /// Exact covariance between baseline and portfolio consequences on paired atoms.
    pub baseline_portfolio_covariance: SignedRational,
    /// Paired variance divided by independent-arm variance, when defined.
    pub paired_to_independent_variance_ratio: Option<Rational>,
    /// Whether pairing reduces, preserves or increases the comparison variance.
    pub pairing_effect: PairingEffect,
}

/// Exact paired comparison between two candidate portfolio arms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortfolioPairComparison {
    /// Lexicographically smaller portfolio identity.
    pub left_portfolio_id: [u8; 32],
    /// Lexicographically larger portfolio identity.
    pub right_portfolio_id: [u8; 32],
    /// Exact mean of left consequence minus right consequence.
    ///
    /// A positive value means the left portfolio has greater mean consequence
    /// than the right portfolio on this finite support.
    pub mean_left_minus_right_consequence_ticks: SignedRational,
    /// Exact population variance of the paired left-minus-right difference.
    pub paired_difference_population_variance: Rational,
    /// Difference variance under independent draws from the same two marginals.
    pub independent_arm_difference_population_variance: Rational,
    /// Exact covariance between the two portfolio consequences on paired atoms.
    pub portfolio_covariance: SignedRational,
    /// Paired variance divided by independent-arm variance, when defined.
    pub paired_to_independent_variance_ratio: Option<Rational>,
    /// Whether pairing reduces, preserves or increases the comparison variance.
    pub pairing_effect: PairingEffect,
}

/// Exact baseline-to-all-portfolios and all-pairs comparison on shared atoms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiPortfolioPairedReport {
    /// Number of unique, equally weighted latent atoms.
    pub atom_count: usize,
    /// Canonically sorted portfolio summaries, one per frozen portfolio identity.
    pub portfolio_summaries: Vec<PortfolioCounterfactualSummary>,
    /// Canonically sorted all-pairs comparison matrix in triangular form.
    pub pairwise_comparisons: Vec<PortfolioPairComparison>,
}

/// Compare a frozen portfolio universe on shared equally weighted finite support.
///
/// Every row is one joint sample containing the baseline and all candidate
/// portfolios under the same exogenous latent atom. Each candidate consequence
/// must already have been directly evaluated through that portfolio's own
/// post-action path; this function never synthesizes joint consequences by
/// adding singleton effects.
///
/// The function validates that every atom contains exactly the same portfolio
/// identity set, canonicalizes portfolio order, and returns each portfolio's
/// paired baseline comparison plus all pairwise portfolio comparisons. The
/// all-pairs report is quadratic in portfolio count and is bounded by
/// MAX_PAIRED_COUNTERFACTUAL_PORTFOLIOS.
///
/// All calculations use exact equally weighted finite-support arithmetic.
/// This is not a confidence procedure, adaptive importance-sampling estimator,
/// physical trajectory coupling verifier or operational decision qualification.
pub fn compare_multi_portfolio_paired_support(
    atoms: &[MultiPortfolioPairedAtom],
) -> Result<MultiPortfolioPairedReport, PairedCounterfactualError> {
    if atoms.is_empty() {
        return Err(PairedCounterfactualError::EmptySupport);
    }
    if atoms.len() > MAX_PAIRED_COUNTERFACTUAL_ATOMS {
        return Err(PairedCounterfactualError::TooManyAtoms);
    }

    let first = &atoms[0].portfolio_consequences;
    if first.is_empty() {
        return Err(PairedCounterfactualError::EmptyPortfolioUniverse);
    }
    if first.len() > MAX_PAIRED_COUNTERFACTUAL_PORTFOLIOS {
        return Err(PairedCounterfactualError::TooManyPortfolios);
    }

    let mut canonical_portfolio_ids = first.iter().map(|arm| arm.portfolio_id).collect::<Vec<_>>();
    canonical_portfolio_ids.sort_unstable();
    if canonical_portfolio_ids.iter().any(|id| id.iter().all(|byte| *byte == 0))
        || canonical_portfolio_ids.windows(2).any(|pair| pair[0] == pair[1])
    {
        return Err(PairedCounterfactualError::InvalidOrDuplicatePortfolioIdentity);
    }

    let portfolio_count = canonical_portfolio_ids.len();
    let mut atom_ids = BTreeSet::new();
    let mut sum_baseline = 0_i128;
    let mut sum_baseline_squared = 0_i128;
    let mut sum_portfolios = vec![0_i128; portfolio_count];
    let mut sum_portfolios_squared = vec![0_i128; portfolio_count];
    let mut sum_baseline_portfolio_products = vec![0_i128; portfolio_count];
    let mut sum_portfolio_cross_products = vec![vec![0_i128; portfolio_count]; portfolio_count];

    for atom in atoms {
        if atom.atom_id.iter().all(|byte| *byte == 0) || !atom_ids.insert(atom.atom_id) {
            return Err(PairedCounterfactualError::InvalidOrDuplicateAtomIdentity);
        }
        if atom.portfolio_consequences.len() != portfolio_count {
            return Err(PairedCounterfactualError::InconsistentPortfolioUniverse);
        }

        let mut arms = atom.portfolio_consequences.iter().copied().collect::<Vec<_>>();
        arms.sort_unstable_by_key(|arm| arm.portfolio_id);
        if arms.iter().any(|arm| arm.portfolio_id.iter().all(|byte| *byte == 0))
            || arms.windows(2).any(|pair| pair[0].portfolio_id == pair[1].portfolio_id)
        {
            return Err(PairedCounterfactualError::InvalidOrDuplicatePortfolioIdentity);
        }
        if arms
            .iter()
            .map(|arm| arm.portfolio_id)
            .ne(canonical_portfolio_ids.iter().copied())
        {
            return Err(PairedCounterfactualError::InconsistentPortfolioUniverse);
        }

        let baseline = i128::from(atom.baseline_consequence_ticks);
        sum_baseline = sum_baseline
            .checked_add(baseline)
            .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;
        sum_baseline_squared = sum_baseline_squared
            .checked_add(
                baseline
                    .checked_mul(baseline)
                    .ok_or(PairedCounterfactualError::ArithmeticOverflow)?,
            )
            .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;

        for (index, arm) in arms.iter().enumerate() {
            let value = i128::from(arm.consequence_ticks);
            sum_portfolios[index] = sum_portfolios[index]
                .checked_add(value)
                .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;
            sum_portfolios_squared[index] = sum_portfolios_squared[index]
                .checked_add(
                    value
                        .checked_mul(value)
                        .ok_or(PairedCounterfactualError::ArithmeticOverflow)?,
                )
                .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;
            sum_baseline_portfolio_products[index] = sum_baseline_portfolio_products[index]
                .checked_add(
                    baseline
                        .checked_mul(value)
                        .ok_or(PairedCounterfactualError::ArithmeticOverflow)?,
                )
                .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;

            for (right_index, right_arm) in arms.iter().enumerate().skip(index) {
                let right_value = i128::from(right_arm.consequence_ticks);
                sum_portfolio_cross_products[index][right_index] =
                    sum_portfolio_cross_products[index][right_index]
                        .checked_add(
                            value
                                .checked_mul(right_value)
                                .ok_or(PairedCounterfactualError::ArithmeticOverflow)?,
                        )
                        .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;
            }
        }
    }

    let count = i128::try_from(atoms.len())
        .map_err(|_| PairedCounterfactualError::ArithmeticOverflow)?;
    let atom_count = atoms.len() as u128;
    let denominator = atom_count
        .checked_mul(atom_count)
        .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;
    let baseline_variance_numerator =
        centered_sum_squares_numerator(count, sum_baseline, sum_baseline_squared)?;

    let mut portfolio_variance_numerators = Vec::with_capacity(portfolio_count);
    let mut portfolio_summaries = Vec::with_capacity(portfolio_count);
    for index in 0..portfolio_count {
        let portfolio_variance_numerator = centered_sum_squares_numerator(
            count,
            sum_portfolios[index],
            sum_portfolios_squared[index],
        )?;
        portfolio_variance_numerators.push(portfolio_variance_numerator);
        let difference_sum = sum_baseline
            .checked_sub(sum_portfolios[index])
            .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;
        let difference_sum_squares = sum_baseline_squared
            .checked_add(sum_portfolios_squared[index])
            .and_then(|value| {
                sum_baseline_portfolio_products[index]
                    .checked_mul(2)
                    .and_then(|cross| value.checked_sub(cross))
            })
            .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;
        let paired_variance_numerator =
            centered_sum_squares_numerator(count, difference_sum, difference_sum_squares)?;
        let independent_variance_numerator = baseline_variance_numerator
            .checked_add(portfolio_variance_numerator)
            .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;
        let paired_variance = Rational::new(paired_variance_numerator, denominator)?;
        let independent_variance = Rational::new(independent_variance_numerator, denominator)?;
        let covariance_numerator = count
            .checked_mul(sum_baseline_portfolio_products[index])
            .and_then(|value| {
                sum_baseline
                    .checked_mul(sum_portfolios[index])
                    .and_then(|product| value.checked_sub(product))
            })
            .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;
        portfolio_summaries.push(PortfolioCounterfactualSummary {
            portfolio_id: canonical_portfolio_ids[index],
            mean_consequence_avoided_ticks: SignedRational::new(difference_sum, atom_count)?,
            paired_difference_population_variance: paired_variance,
            independent_arm_difference_population_variance: independent_variance,
            baseline_portfolio_covariance: SignedRational::new(covariance_numerator, denominator)?,
            paired_to_independent_variance_ratio: if independent_variance.numerator() == 0 {
                None
            } else {
                Some(paired_variance.checked_div(independent_variance)?)
            },
            pairing_effect: pairing_effect(paired_variance, independent_variance),
        });
    }

    let pair_count = portfolio_count
        .checked_mul(portfolio_count.saturating_sub(1))
        .and_then(|value| value.checked_div(2))
        .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;
    let mut pairwise_comparisons = Vec::with_capacity(pair_count);
    for left in 0..portfolio_count {
        for right in (left + 1)..portfolio_count {
            let cross_sum = sum_portfolio_cross_products[left][right];
            let difference_sum = sum_portfolios[left]
                .checked_sub(sum_portfolios[right])
                .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;
            let difference_sum_squares = sum_portfolios_squared[left]
                .checked_add(sum_portfolios_squared[right])
                .and_then(|value| {
                    cross_sum
                        .checked_mul(2)
                        .and_then(|cross| value.checked_sub(cross))
                })
                .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;
            let paired_variance_numerator =
                centered_sum_squares_numerator(count, difference_sum, difference_sum_squares)?;
            let independent_variance_numerator = portfolio_variance_numerators[left]
                .checked_add(portfolio_variance_numerators[right])
                .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;
            let paired_variance = Rational::new(paired_variance_numerator, denominator)?;
            let independent_variance = Rational::new(independent_variance_numerator, denominator)?;
            let covariance_numerator = count
                .checked_mul(cross_sum)
                .and_then(|value| {
                    sum_portfolios[left]
                        .checked_mul(sum_portfolios[right])
                        .and_then(|product| value.checked_sub(product))
                })
                .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;
            pairwise_comparisons.push(PortfolioPairComparison {
                left_portfolio_id: canonical_portfolio_ids[left],
                right_portfolio_id: canonical_portfolio_ids[right],
                mean_left_minus_right_consequence_ticks: SignedRational::new(
                    difference_sum,
                    atom_count,
                )?,
                paired_difference_population_variance: paired_variance,
                independent_arm_difference_population_variance: independent_variance,
                portfolio_covariance: SignedRational::new(covariance_numerator, denominator)?,
                paired_to_independent_variance_ratio: if independent_variance.numerator() == 0 {
                    None
                } else {
                    Some(paired_variance.checked_div(independent_variance)?)
                },
                pairing_effect: pairing_effect(paired_variance, independent_variance),
            });
        }
    }

    Ok(MultiPortfolioPairedReport {
        atom_count: atoms.len(),
        portfolio_summaries,
        pairwise_comparisons,
    })
}

fn pairing_effect(paired: Rational, independent: Rational) -> PairingEffect {
    match paired.cmp(&independent) {
        std::cmp::Ordering::Less => PairingEffect::VarianceReduction,
        std::cmp::Ordering::Equal if independent.numerator() == 0 => {
            PairingEffect::ZeroIndependentVariance
        }
        std::cmp::Ordering::Equal => PairingEffect::NoVarianceChange,
        std::cmp::Ordering::Greater => PairingEffect::VarianceInflation,
    }
}

fn centered_sum_squares_numerator(
    count: i128,
    sum: i128,
    sum_squares: i128,
) -> Result<u128, PairedCounterfactualError> {
    let scaled_squares = count
        .checked_mul(sum_squares)
        .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;
    let squared_sum = sum
        .checked_mul(sum)
        .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;
    let numerator = scaled_squares
        .checked_sub(squared_sum)
        .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;
    u128::try_from(numerator).map_err(|_| PairedCounterfactualError::InconsistentVariance)
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atom(id: u8, baseline: i32, portfolio: i32) -> PairedCounterfactualAtom {
        PairedCounterfactualAtom {
            atom_id: [id; 32],
            baseline_consequence_ticks: baseline,
            portfolio_consequence_ticks: portfolio,
        }
    }

    #[test]
    fn positive_covariance_pairing_reduces_difference_variance_to_zero() {
        let report = compare_paired_counterfactual_support(&[
            atom(1, 0, 1),
            atom(2, 1, 2),
            atom(3, 2, 3),
            atom(4, 3, 4),
        ])
        .unwrap();

        assert_eq!(
            report.mean_consequence_avoided_ticks,
            SignedRational {
                numerator: -1,
                denominator: 1,
            }
        );
        assert_eq!(
            report.baseline_portfolio_covariance,
            SignedRational {
                numerator: 5,
                denominator: 4,
            }
        );
        assert_eq!(
            report.paired_difference_population_variance,
            Rational::from_integer(0)
        );
        assert_eq!(
            report.independent_arm_difference_population_variance,
            Rational::new(5, 2).unwrap()
        );
        assert_eq!(
            report.paired_to_independent_variance_ratio,
            Some(Rational::from_integer(0))
        );
        assert_eq!(report.pairing_effect, PairingEffect::VarianceReduction);
    }

    #[test]
    fn zero_covariance_pairing_leaves_difference_variance_unchanged() {
        let report = compare_paired_counterfactual_support(&[
            atom(1, 0, 0),
            atom(2, 1, 1),
            atom(3, 2, 1),
            atom(4, 3, 0),
        ])
        .unwrap();

        assert_eq!(
            report.baseline_portfolio_covariance,
            SignedRational {
                numerator: 0,
                denominator: 1,
            }
        );
        assert_eq!(
            report.paired_difference_population_variance,
            Rational::new(3, 2).unwrap()
        );
        assert_eq!(
            report.independent_arm_difference_population_variance,
            Rational::new(3, 2).unwrap()
        );
        assert_eq!(
            report.paired_to_independent_variance_ratio,
            Some(Rational::from_integer(1))
        );
        assert_eq!(report.pairing_effect, PairingEffect::NoVarianceChange);
    }

    #[test]
    fn negative_covariance_pairing_increases_difference_variance() {
        let report = compare_paired_counterfactual_support(&[
            atom(1, 0, 3),
            atom(2, 1, 2),
            atom(3, 2, 1),
            atom(4, 3, 0),
        ])
        .unwrap();

        assert_eq!(
            report.baseline_portfolio_covariance,
            SignedRational {
                numerator: -5,
                denominator: 4,
            }
        );
        assert_eq!(
            report.paired_difference_population_variance,
            Rational::from_integer(5)
        );
        assert_eq!(
            report.independent_arm_difference_population_variance,
            Rational::new(5, 2).unwrap()
        );
        assert_eq!(
            report.paired_to_independent_variance_ratio,
            Some(Rational::from_integer(2))
        );
        assert_eq!(report.pairing_effect, PairingEffect::VarianceInflation);
    }

    #[test]
    fn exact_counterfactual_comparison_is_invariant_to_atom_order() {
        let forward = [atom(1, 0, 3), atom(2, 1, 2), atom(3, 2, 1), atom(4, 3, 0)];
        let reverse = [atom(4, 3, 0), atom(3, 2, 1), atom(2, 1, 2), atom(1, 0, 3)];
        assert_eq!(
            compare_paired_counterfactual_support(&forward),
            compare_paired_counterfactual_support(&reverse)
        );
    }

    #[test]
    fn duplicate_and_zero_atom_identities_are_rejected() {
        assert_eq!(
            compare_paired_counterfactual_support(&[atom(1, 0, 0), atom(1, 1, 1)]),
            Err(PairedCounterfactualError::InvalidOrDuplicateAtomIdentity)
        );
        assert_eq!(
            compare_paired_counterfactual_support(&[PairedCounterfactualAtom {
                atom_id: [0; 32],
                baseline_consequence_ticks: 0,
                portfolio_consequence_ticks: 0,
            }]),
            Err(PairedCounterfactualError::InvalidOrDuplicateAtomIdentity)
        );
    }

    #[test]
    fn empty_support_is_not_a_zero_risk_result() {
        assert_eq!(
            compare_paired_counterfactual_support(&[]),
            Err(PairedCounterfactualError::EmptySupport)
        );
    }

    #[test]
    fn one_atom_has_zero_population_variance_and_no_variance_ratio() {
        let report = compare_paired_counterfactual_support(&[atom(1, 10, 7)]).unwrap();
        assert_eq!(
            report.mean_consequence_avoided_ticks,
            SignedRational {
                numerator: 3,
                denominator: 1,
            }
        );
        assert_eq!(
            report.independent_arm_difference_population_variance,
            Rational::from_integer(0)
        );
        assert_eq!(report.paired_to_independent_variance_ratio, None);
        assert_eq!(
            report.pairing_effect,
            PairingEffect::ZeroIndependentVariance
        );
    }
}
