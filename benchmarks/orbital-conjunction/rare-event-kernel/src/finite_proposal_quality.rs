// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact finite-support proposal quality diagnostics.
//!
//! This module separates proposal support coverage from proposal fitting.
//! It evaluates both adaptive residual proposals under the complete defensive
//! mixture and reports exact event-estimator second moments, variance and ESS.
//! It is a finite categorical oracle, not a continuous Gaussian/GMM certificate.

use crate::proposal_execution_conformance::{
    FiniteAdaptiveProposalConfig, FiniteProposalExecutionVerifier, TICKET_SPACE,
};
use crate::space_time_partition::Rational;

/// Exact proposal-quality metrics for one adaptive residual-proposal state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiniteProposalStateQuality {
    /// Residual proposal identity for this state.
    pub residual_proposal_digest: [u8; 32],
    /// Complete defensive-mixture proposal identity for this state.
    pub complete_proposal_digest: [u8; 32],
    /// Number of atoms with positive target mass.
    pub target_support_atom_count: usize,
    /// Number of atoms with positive residual-proposal mass.
    pub residual_support_atom_count: usize,
    /// Number of target-support atoms missed by the residual proposal.
    pub residual_missing_target_atom_count: usize,
    /// Number of event atoms with positive target mass missed by the residual proposal.
    pub residual_missing_event_atom_count: usize,
    /// Number of target-support atoms missed by the complete defensive mixture.
    pub complete_missing_target_atom_count: usize,
    /// Maximum target/residual likelihood ratio; absent when residual support is incomplete.
    pub residual_max_importance_ratio: Option<Rational>,
    /// Maximum target/complete-mixture likelihood ratio over target support.
    pub complete_max_importance_ratio: Rational,
    /// Maximum normalized defensive weight alpha * p(x) / q(x); must be at most one.
    pub maximum_normalized_defensive_weight: Rational,
    /// Exact target event probability P_p(E).
    pub target_event_probability: Rational,
    /// Exact event-estimator second moment E_q[(1_E p/q)^2].
    pub event_estimator_second_moment: Rational,
    /// Exact event-estimator variance under q.
    pub event_estimator_variance: Rational,
    /// Exact ESS fraction P_p(E)^2 / E_q[(1_E p/q)^2]; absent for a zero-probability event.
    pub event_ess_fraction: Option<Rational>,
}

/// Exact comparison of the before-event and after-event residual proposals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiniteProposalQualityReport {
    /// Target-measure identity.
    pub target_measure_digest: [u8; 32],
    /// Defensive-mixture component identity.
    pub defensive_component_digest: [u8; 32],
    /// Defensive mixture weight alpha.
    pub defensive_mixture_weight: Rational,
    /// Quality metrics before any event has been observed.
    pub before_event: FiniteProposalStateQuality,
    /// Quality metrics after the previous draw was an event.
    pub after_event: FiniteProposalStateQuality,
}

/// Fail-closed errors from finite proposal-quality assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FiniteProposalQualityError {
    /// The supplied proposal contract is invalid.
    InvalidConfiguration,
    /// An exact rational calculation overflowed or had an invalid denominator.
    ArithmeticOverflow,
    /// Defensive-mixture support or normalized-weight bound was violated.
    DefensiveBoundViolated,
    /// The finite complete-mixture masses do not sum to one.
    MixtureMassDoesNotNormalize,
}

impl core::fmt::Display for FiniteProposalQualityError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidConfiguration => "finite proposal configuration is invalid",
            Self::ArithmeticOverflow => "finite proposal-quality rational arithmetic failed",
            Self::DefensiveBoundViolated => "complete defensive proposal violates its support or weight bound",
            Self::MixtureMassDoesNotNormalize => "finite defensive-mixture masses do not sum to one",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for FiniteProposalQualityError {}

/// Evaluate the exact finite-support quality of both adaptive residual proposals.
///
/// For each state, the complete proposal is q(i) = alpha p(i) + (1-alpha) r(i).
/// The routine reports support coverage of r separately from q, the maximum
/// likelihood ratios, and exact event-estimator second moment/variance/ESS.
/// The complete defensive mixture must cover every positive target atom and
/// its normalized defensive weight must not exceed one.
pub fn assess_finite_proposal_quality(
    config: &FiniteAdaptiveProposalConfig,
) -> Result<FiniteProposalQualityReport, FiniteProposalQualityError> {
    FiniteProposalExecutionVerifier::new(config.clone())
        .map_err(|_| FiniteProposalQualityError::InvalidConfiguration)?;

    let before_event = assess_state(
        config,
        config.residual_before_event_digest,
        config.complete_proposal_before_event_digest,
        &config.residual_before_event_ticks,
    )?;
    let after_event = assess_state(
        config,
        config.residual_after_event_digest,
        config.complete_proposal_after_event_digest,
        &config.residual_after_event_ticks,
    )?;

    Ok(FiniteProposalQualityReport {
        target_measure_digest: config.target_measure_digest,
        defensive_component_digest: config.defensive_component_digest,
        defensive_mixture_weight: rational(
            u128::from(config.defensive_mixture_weight_ticks),
            u128::from(TICKET_SPACE),
        )?,
        before_event,
        after_event,
    })
}

fn assess_state(
    config: &FiniteAdaptiveProposalConfig,
    residual_proposal_digest: [u8; 32],
    complete_proposal_digest: [u8; 32],
    residual_masses: &[u16],
) -> Result<FiniteProposalStateQuality, FiniteProposalQualityError> {
    let alpha = u64::from(config.defensive_mixture_weight_ticks);
    let complement = u64::from(TICKET_SPACE - config.defensive_mixture_weight_ticks);
    let mut target_support_atom_count = 0;
    let mut residual_support_atom_count = 0;
    let mut residual_missing_target_atom_count = 0;
    let mut residual_missing_event_atom_count = 0;
    let mut complete_missing_target_atom_count = 0;
    let mut mixture_mass_sum = 0_u64;
    let mut residual_max_importance_ratio: Option<Rational> = Some(Rational::from_integer(0));
    let mut complete_max_importance_ratio = Rational::from_integer(0);
    let mut maximum_normalized_defensive_weight = Rational::from_integer(0);
    let mut event_mass_ticks = 0_u64;
    let mut event_second_moment = Rational::from_integer(0);

    for ((atom, target_ticks), residual_ticks) in config
        .atoms
        .iter()
        .zip(&config.target_mass_ticks)
        .zip(residual_masses)
    {
        let target = u64::from(*target_ticks);
        let residual = u64::from(*residual_ticks);
        let mixture_numerator = alpha * target + complement * residual;
        mixture_mass_sum = mixture_mass_sum
            .checked_add(mixture_numerator)
            .ok_or(FiniteProposalQualityError::ArithmeticOverflow)?;

        if target > 0 {
            target_support_atom_count += 1;
            if residual == 0 {
                residual_missing_target_atom_count += 1;
                residual_max_importance_ratio = None;
                if atom.is_event {
                    residual_missing_event_atom_count += 1;
                }
            } else if let Some(current) = residual_max_importance_ratio {
                let ratio = rational(u128::from(target), u128::from(residual))?;
                if ratio > current {
                    residual_max_importance_ratio = Some(ratio);
                }
            }

            if mixture_numerator == 0 {
                complete_missing_target_atom_count += 1;
                return Err(FiniteProposalQualityError::DefensiveBoundViolated);
            }
            let complete_ratio = rational(
                u128::from(target) * u128::from(TICKET_SPACE),
                u128::from(mixture_numerator),
            )?;
            if complete_ratio > complete_max_importance_ratio {
                complete_max_importance_ratio = complete_ratio;
            }

            let normalized_weight = rational(
                alpha as u128 * u128::from(target),
                u128::from(mixture_numerator),
            )?;
            if normalized_weight > Rational::from_integer(1) {
                return Err(FiniteProposalQualityError::DefensiveBoundViolated);
            }
            if normalized_weight > maximum_normalized_defensive_weight {
                maximum_normalized_defensive_weight = normalized_weight;
            }
        }

        if atom.is_event {
            event_mass_ticks = event_mass_ticks
                .checked_add(target)
                .ok_or(FiniteProposalQualityError::ArithmeticOverflow)?;
            if target > 0 {
                let squared_target = u128::from(target)
                    .checked_mul(u128::from(target))
                    .ok_or(FiniteProposalQualityError::ArithmeticOverflow)?;
                let contribution = rational(squared_target, u128::from(mixture_numerator))?;
                event_second_moment = event_second_moment
                    .checked_add(contribution)
                    .map_err(|_| FiniteProposalQualityError::ArithmeticOverflow)?;
            }
        }

        if residual > 0 {
            residual_support_atom_count += 1;
        }
    }

    let expected_mixture_mass = u64::from(TICKET_SPACE) * u64::from(TICKET_SPACE);
    if mixture_mass_sum != expected_mixture_mass {
        return Err(FiniteProposalQualityError::MixtureMassDoesNotNormalize);
    }

    let target_event_probability = rational(
        u128::from(event_mass_ticks),
        u128::from(TICKET_SPACE),
    )?;
    let event_probability_squared = target_event_probability
        .checked_mul(target_event_probability)
        .map_err(|_| FiniteProposalQualityError::ArithmeticOverflow)?;
    let event_estimator_variance = event_second_moment
        .checked_sub(event_probability_squared)
        .map_err(|_| FiniteProposalQualityError::ArithmeticOverflow)?;
    let event_ess_fraction = if event_second_moment == Rational::from_integer(0) {
        None
    } else {
        Some(
            event_probability_squared
                .checked_div(event_second_moment)
                .map_err(|_| FiniteProposalQualityError::ArithmeticOverflow)?,
        )
    };

    Ok(FiniteProposalStateQuality {
        residual_proposal_digest,
        complete_proposal_digest,
        target_support_atom_count,
        residual_support_atom_count,
        residual_missing_target_atom_count,
        residual_missing_event_atom_count,
        complete_missing_target_atom_count,
        residual_max_importance_ratio,
        complete_max_importance_ratio,
        maximum_normalized_defensive_weight,
        target_event_probability,
        event_estimator_second_moment: event_second_moment,
        event_estimator_variance,
        event_ess_fraction,
    })
}

fn rational(numerator: u128, denominator: u128) -> Result<Rational, FiniteProposalQualityError> {
    Rational::new(numerator, denominator)
        .map_err(|_| FiniteProposalQualityError::ArithmeticOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> FiniteAdaptiveProposalConfig {
        FiniteAdaptiveProposalConfig {
            support_manifest_digest: [14; 32],
            event_digest: [1; 32],
            target_measure_digest: [2; 32],
            defensive_component_digest: [3; 32],
            proposal_policy_digest: [4; 32],
            sampler_implementation_digest: [5; 32],
            rng_algorithm_digest: [6; 32],
            rng_seed_commitment_digest: [7; 32],
            branch_stream_digest: [8; 32],
            atom_stream_digest: [9; 32],
            residual_before_event_digest: [10; 32],
            residual_after_event_digest: [11; 32],
            complete_proposal_before_event_digest: [12; 32],
            complete_proposal_after_event_digest: [13; 32],
            defensive_mixture_weight_ticks: 64,
            atoms: vec![
                crate::proposal_execution_conformance::FiniteProposalAtom {
                    atom_id: [20; 32],
                    is_event: true,
                },
                crate::proposal_execution_conformance::FiniteProposalAtom {
                    atom_id: [21; 32],
                    is_event: false,
                },
                crate::proposal_execution_conformance::FiniteProposalAtom {
                    atom_id: [22; 32],
                    is_event: true,
                },
                crate::proposal_execution_conformance::FiniteProposalAtom {
                    atom_id: [23; 32],
                    is_event: false,
                },
            ],
            target_mass_ticks: vec![64, 64, 64, 64],
            residual_before_event_ticks: vec![128, 64, 32, 32],
            residual_after_event_ticks: vec![32, 32, 64, 128],
        }
    }

    #[test]
    fn exact_metrics_distinguish_the_two_adaptive_proposal_states() {
        let report = assess_finite_proposal_quality(&config()).unwrap();
        let before = &report.before_event;
        let after = &report.after_event;

        assert_eq!(before.target_event_probability, rational(1, 2).unwrap());
        assert_eq!(before.complete_max_importance_ratio, rational(8, 5).unwrap());
        assert_eq!(before.maximum_normalized_defensive_weight, rational(2, 5).unwrap());
        assert_eq!(before.event_estimator_second_moment, rational(19, 35).unwrap());
        assert_eq!(before.event_estimator_variance, rational(41, 140).unwrap());
        assert_eq!(before.event_ess_fraction, Some(rational(35, 76).unwrap()));

        assert_eq!(after.target_event_probability, rational(1, 2).unwrap());
        assert_eq!(after.event_estimator_second_moment, rational(13, 20).unwrap());
        assert_eq!(after.event_estimator_variance, rational(2, 5).unwrap());
        assert_eq!(after.event_ess_fraction, Some(rational(5, 13).unwrap()));
        assert_eq!(before.complete_missing_target_atom_count, 0);
        assert!(before.event_ess_fraction > after.event_ess_fraction);
    }

    #[test]
    fn defensive_component_covers_event_modes_missed_by_residual_proposal() {
        let mut c = config();
        c.residual_before_event_ticks = vec![0, 128, 0, 128];
        let report = assess_finite_proposal_quality(&c).unwrap();
        let before = report.before_event;

        assert_eq!(before.residual_missing_target_atom_count, 2);
        assert_eq!(before.residual_missing_event_atom_count, 2);
        assert_eq!(before.complete_missing_target_atom_count, 0);
        assert_eq!(before.maximum_normalized_defensive_weight, rational(1, 1).unwrap());
        assert_eq!(before.event_estimator_second_moment, rational(2, 1).unwrap());
        assert_eq!(before.event_ess_fraction, Some(rational(1, 8).unwrap()));
        assert_eq!(before.residual_max_importance_ratio, None);
    }

    #[test]
    fn proposal_matching_target_has_optimal_finite_fixture_ess() {
        let mut c = config();
        c.residual_before_event_ticks = c.target_mass_ticks.clone();
        c.residual_after_event_ticks = c.target_mass_ticks.clone();
        let report = assess_finite_proposal_quality(&c).unwrap();

        assert_eq!(report.before_event.complete_max_importance_ratio, rational(1, 1).unwrap());
        assert_eq!(report.before_event.event_estimator_second_moment, rational(1, 2).unwrap());
        assert_eq!(report.before_event.event_estimator_variance, rational(1, 4).unwrap());
        assert_eq!(report.before_event.event_ess_fraction, Some(rational(1, 2).unwrap()));
    }

    #[test]
    fn invalid_defensive_or_probability_configurations_fail_closed() {
        let mut no_defense = config();
        no_defense.defensive_mixture_weight_ticks = 0;
        assert_eq!(
            assess_finite_proposal_quality(&no_defense),
            Err(FiniteProposalQualityError::InvalidConfiguration)
        );

        let mut invalid_mass = config();
        invalid_mass.residual_before_event_ticks[0] -= 1;
        assert_eq!(
            assess_finite_proposal_quality(&invalid_mass),
            Err(FiniteProposalQualityError::InvalidConfiguration)
        );
    }
}
