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

use std::collections::BTreeSet;

use crate::space_time_partition::{Rational, SpaceTimePartitionError};

/// Maximum number of atoms accepted by the exact finite-support reference kernel.
pub const MAX_PAIRED_COUNTERFACTUAL_ATOMS: usize = 65_536;

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

    let count = i128::try_from(atoms.len())
        .map_err(|_| PairedCounterfactualError::ArithmeticOverflow)?;
    let denominator = (atoms.len() as u128)
        .checked_mul(atoms.len() as u128)
        .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;

    let baseline_variance_numerator = centered_sum_squares_numerator(
        count,
        sum_baseline,
        sum_baseline_squared,
    )?;
    let portfolio_variance_numerator = centered_sum_squares_numerator(
        count,
        sum_portfolio,
        sum_portfolio_squared,
    )?;
    let paired_variance_numerator = centered_sum_squares_numerator(
        count,
        sum_difference,
        sum_difference_squared,
    )?;

    let independent_variance_numerator = baseline_variance_numerator
        .checked_add(portfolio_variance_numerator)
        .ok_or(PairedCounterfactualError::ArithmeticOverflow)?;

    let paired_variance = Rational::new(
        paired_variance_numerator,
        denominator,
    )?;
    let independent_variance = Rational::new(
        independent_variance_numerator,
        denominator,
    )?;
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

        assert_eq!(report.mean_consequence_avoided_ticks, SignedRational {
            numerator: -1,
            denominator: 1,
        });
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
        let forward = [
            atom(1, 0, 3),
            atom(2, 1, 2),
            atom(3, 2, 1),
            atom(4, 3, 0),
        ];
        let reverse = [
            atom(4, 3, 0),
            atom(3, 2, 1),
            atom(2, 1, 2),
            atom(1, 0, 3),
        ];
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
        assert_eq!(report.pairing_effect, PairingEffect::ZeroIndependentVariance);
    }
}
