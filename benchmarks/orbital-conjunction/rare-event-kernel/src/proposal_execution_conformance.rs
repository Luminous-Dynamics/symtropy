// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact finite-support execution conformance for defensive adaptive importance sampling.
//!
//! This reference kernel checks the *sampling transform*, not just the density
//! values reported after a draw. Given explicit branch and atom tickets, it
//! verifies the defensive-mixture component selection, inverse-CDF atom
//! selection, predictable residual-proposal identity, and exact complete-mixture
//! importance contribution.
//!
//! The ticket source is an external assumption: this module does not prove
//! that tickets are uniform or independent. It also does not establish
//! continuous-domain proposal domination or qualify a production RNG.

use std::collections::BTreeSet;

use crate::adaptive_importance_sampling::AdaptiveImportanceDraw;

/// Number of equiprobable integer tickets in each reference random stream.
pub const TICKET_SPACE: u16 = 256;

/// A finite atom in the declared target support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FiniteProposalAtom {
    /// Stable identity of this atom.
    pub atom_id: [u8; 32],
    /// Whether this atom belongs to the declared collision event.
    pub is_event: bool,
}

/// Pinned finite-support target and adaptive defensive-proposal contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiniteAdaptiveProposalConfig {
    /// Collision-event identity.
    pub event_digest: [u8; 32],
    /// Frozen target-measure identity.
    pub target_measure_digest: [u8; 32],
    /// Identity of the defensive target component.
    pub defensive_component_digest: [u8; 32],
    /// Identity of the predictable residual-proposal policy.
    pub proposal_policy_digest: [u8; 32],
    /// Identity of the sampler implementation under test.
    pub sampler_implementation_digest: [u8; 32],
    /// Identity of the random-word generator contract.
    pub rng_algorithm_digest: [u8; 32],
    /// Commitment identifying the random seed or entropy source.
    pub rng_seed_commitment_digest: [u8; 32],
    /// Domain-separated identity for defensive/residual branch tickets.
    pub branch_stream_digest: [u8; 32],
    /// Domain-separated identity for within-component atom tickets.
    pub atom_stream_digest: [u8; 32],
    /// Residual proposal used before any event has been observed.
    pub residual_before_event_digest: [u8; 32],
    /// Residual proposal used after the previous draw was an event.
    pub residual_after_event_digest: [u8; 32],
    /// Complete defensive-mixture proposal identity before any event has been observed.
    pub complete_proposal_before_event_digest: [u8; 32],
    /// Complete defensive-mixture proposal identity after the previous draw was an event.
    pub complete_proposal_after_event_digest: [u8; 32],
    /// Defensive mixture weight in integer units of 1/TICKET_SPACE.
    pub defensive_mixture_weight_ticks: u16,
    /// Ordered finite target support.
    pub atoms: Vec<FiniteProposalAtom>,
    /// Target categorical masses; must sum exactly to TICKET_SPACE.
    pub target_mass_ticks: Vec<u16>,
    /// Residual categorical masses used when the previous draw was not an event.
    pub residual_before_event_ticks: Vec<u16>,
    /// Residual categorical masses used when the previous draw was an event.
    pub residual_after_event_ticks: Vec<u16>,
}

/// One draw's execution evidence, including the explicit random tickets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FiniteProposalExecutionReceipt {
    /// Zero-based draw index; must equal the verifier's accepted history length.
    pub sequence_index: u64,
    /// Unique identity for this sample.
    pub sample_id: [u8; 32],
    /// Event identity.
    pub event_digest: [u8; 32],
    /// Target-measure identity.
    pub target_measure_digest: [u8; 32],
    /// Defensive-component identity.
    pub defensive_component_digest: [u8; 32],
    /// Proposal-policy identity.
    pub proposal_policy_digest: [u8; 32],
    /// Sampler implementation identity.
    pub sampler_implementation_digest: [u8; 32],
    /// Random-word generator identity.
    pub rng_algorithm_digest: [u8; 32],
    /// Random seed or entropy-source commitment.
    pub rng_seed_commitment_digest: [u8; 32],
    /// Branch-ticket stream identity.
    pub branch_stream_digest: [u8; 32],
    /// Atom-ticket stream identity.
    pub atom_stream_digest: [u8; 32],
    /// Residual proposal selected from the *prior* event history.
    pub proposal_digest: [u8; 32],
    /// Complete defensive-mixture proposal identity, including the target component.
    pub complete_proposal_digest: [u8; 32],
    /// Defensive mixture weight, in integer units of 1/TICKET_SPACE.
    pub defensive_mixture_weight_ticks: u16,
    /// Uniform ticket used to select target versus residual component.
    pub branch_ticket: u16,
    /// Uniform ticket used for inverse-CDF sampling within that component.
    pub atom_ticket: u16,
    /// Atom identity claimed by the sampler.
    pub selected_atom_id: [u8; 32],
}

/// The exact result independently reconstructed from one execution receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifiedFiniteProposalDraw {
    /// Collision-event identity.
    pub event_digest: [u8; 32],
    /// Target-measure identity.
    pub target_measure_digest: [u8; 32],
    /// Defensive-component identity.
    pub defensive_component_digest: [u8; 32],
    /// Complete defensive-mixture proposal identity.
    pub complete_proposal_digest: [u8; 32],
    /// Defensive mixture weight in integer units of 1/TICKET_SPACE.
    pub defensive_mixture_weight_ticks: u16,
    /// Zero-based draw index.
    pub sequence_index: u64,
    /// Accepted sample identity.
    pub sample_id: [u8; 32],
    /// Selected support atom identity.
    pub selected_atom_id: [u8; 32],
    /// Whether the selected atom belongs to the collision event.
    pub event_occurred: bool,
    /// True when the defensive target component generated the atom.
    pub defensive_component_selected: bool,
    /// Target mass of the selected atom, in ticks.
    pub target_mass_ticks: u16,
    /// Residual mass of the selected atom under the prior-history proposal, in ticks.
    pub residual_mass_ticks: u16,
    /// Numerator of the complete mixture mass, with denominator TICKET_SPACE².
    pub mixture_mass_numerator: u64,
    /// Numerator of alpha * p(atom) / q(atom), with the same denominator convention.
    pub normalized_weight_numerator: u64,
    /// Denominator of alpha * p(atom) / q(atom).
    pub normalized_weight_denominator: u64,
}

impl VerifiedFiniteProposalDraw {
    /// Return the exact normalized contribution as a numerator/denominator pair.
    ///
    /// The ratio is in [0, 1]. A zero event contribution is represented as 0/1.
    pub fn normalized_contribution(self) -> (u64, u64) {
        if self.normalized_weight_numerator == 0 {
            (0, 1)
        } else {
            (
                self.normalized_weight_numerator,
                self.normalized_weight_denominator,
            )
        }
    }

    /// Convert the verified finite-support draw into the existing confidence-sequence input.
    ///
    /// The target and complete-mixture categorical masses are converted to
    /// their exact probabilities and then represented in log space. A zero
    /// target mass maps to negative infinity, which the confidence-sequence
    /// adapter accepts as a zero target-density observation. The proposal
    /// density remains strictly positive because the defensive mixture weight
    /// is positive and the complete mixture mass was checked by the verifier.
    pub fn into_adaptive_importance_draw(self) -> AdaptiveImportanceDraw {
        let log_target_density = if self.target_mass_ticks == 0 {
            f64::NEG_INFINITY
        } else {
            (f64::from(self.target_mass_ticks) / f64::from(TICKET_SPACE)).ln()
        };
        let log_proposal_density =
            (self.mixture_mass_numerator as f64 / f64::from(TICKET_SPACE).powi(2)).ln();

        AdaptiveImportanceDraw {
            sequence_index: self.sequence_index,
            sample_id: self.sample_id,
            event_digest: self.event_digest,
            target_measure_digest: self.target_measure_digest,
            proposal_digest: self.complete_proposal_digest,
            defensive_component_digest: self.defensive_component_digest,
            defensive_mixture_weight: f64::from(self.defensive_mixture_weight_ticks)
                / f64::from(TICKET_SPACE),
            log_target_density,
            log_proposal_density,
            event_occurred: self.event_occurred,
        }
    }
}

/// Fail-closed configuration, receipt, or replay errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FiniteProposalExecutionError {
    /// A required identity is all zero.
    InvalidIdentity,
    /// Branch and atom random streams must have distinct identities.
    RandomStreamsNotDomainSeparated,
    /// Defensive mixture weight is outside (0, 1].
    InvalidDefensiveMixtureWeight,
    /// Atom identities are zero or duplicated.
    InvalidAtomIdentity,
    /// A probability vector has the wrong length or does not sum to TICKET_SPACE.
    InvalidProbabilityVector,
    /// A receipt's scope or implementation identity differs from the pinned config.
    ReceiptScopeMismatch,
    /// The receipt is not the next expected draw.
    SequenceMismatch,
    /// A sample identity is zero or has already been accepted.
    DuplicateOrInvalidSampleIdentity,
    /// A random ticket is outside the declared finite ticket space.
    TicketOutsideRange,
    /// The residual proposal was not the one selected by prior observed history.
    UnpredictableProposalSelection,
    /// The sampler's selected atom does not match the inverse-CDF result.
    SampleTransformMismatch,
    /// The selected atom has zero complete-mixture probability.
    ZeroMixtureProbability,
    /// The reconstructed normalized contribution exceeds one.
    DefensiveWeightBoundViolated,
    /// The accepted-draw counter cannot be incremented.
    CounterOverflow,
}

impl core::fmt::Display for FiniteProposalExecutionError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidIdentity => "a required proposal execution identity is invalid",
            Self::RandomStreamsNotDomainSeparated => {
                "branch and atom random streams are not domain-separated"
            }
            Self::InvalidDefensiveMixtureWeight => "defensive mixture weight is outside (0, 1]",
            Self::InvalidAtomIdentity => "atom identity is zero or duplicated",
            Self::InvalidProbabilityVector => "finite proposal probability vector is invalid",
            Self::ReceiptScopeMismatch => {
                "proposal execution receipt does not match the pinned scope"
            }
            Self::SequenceMismatch => "proposal execution receipt is not the next draw",
            Self::DuplicateOrInvalidSampleIdentity => "sample identity is zero or duplicated",
            Self::TicketOutsideRange => "random ticket is outside the declared ticket space",
            Self::UnpredictableProposalSelection => {
                "residual proposal was not selected from prior history"
            }
            Self::SampleTransformMismatch => {
                "selected atom does not match the declared sampling transform"
            }
            Self::ZeroMixtureProbability => "selected atom has zero complete-mixture probability",
            Self::DefensiveWeightBoundViolated => "normalized defensive contribution exceeds one",
            Self::CounterOverflow => "proposal execution draw counter overflowed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for FiniteProposalExecutionError {}

/// Stateful replay verifier for the exact finite-support defensive sampler.
#[derive(Debug, Clone)]
pub struct FiniteProposalExecutionVerifier {
    config: FiniteAdaptiveProposalConfig,
    prior_event_history: Vec<bool>,
    seen_sample_ids: BTreeSet<[u8; 32]>,
}

impl FiniteProposalExecutionVerifier {
    /// Validate and pin a finite adaptive defensive-proposal contract.
    pub fn new(config: FiniteAdaptiveProposalConfig) -> Result<Self, FiniteProposalExecutionError> {
        validate_config(&config)?;
        Ok(Self {
            config,
            prior_event_history: Vec::new(),
            seen_sample_ids: BTreeSet::new(),
        })
    }

    /// Number of prior draws accepted by this verifier.
    pub fn sample_count(&self) -> u64 {
        self.prior_event_history.len() as u64
    }

    /// Verify and atomically accept the next execution receipt.
    ///
    /// The residual proposal identity and mass vector are derived from the
    /// verifier's already-accepted event history. The branch ticket chooses
    /// between p and r_t; the atom ticket is then inverse-CDF mapped through
    /// the selected component. The returned importance contribution always
    /// uses the complete mixture q_t, never the selected component density.
    pub fn verify_next(
        &mut self,
        receipt: FiniteProposalExecutionReceipt,
    ) -> Result<VerifiedFiniteProposalDraw, FiniteProposalExecutionError> {
        if receipt.sequence_index != self.prior_event_history.len() as u64 {
            return Err(FiniteProposalExecutionError::SequenceMismatch);
        }
        if !same_receipt_scope(&self.config, receipt) {
            return Err(FiniteProposalExecutionError::ReceiptScopeMismatch);
        }
        if is_zero_digest(&receipt.sample_id) || self.seen_sample_ids.contains(&receipt.sample_id) {
            return Err(FiniteProposalExecutionError::DuplicateOrInvalidSampleIdentity);
        }
        if receipt.branch_ticket >= TICKET_SPACE || receipt.atom_ticket >= TICKET_SPACE {
            return Err(FiniteProposalExecutionError::TicketOutsideRange);
        }

        let prior_event = self.prior_event_history.last().copied().unwrap_or(false);
        let (expected_proposal_digest, expected_complete_proposal_digest, residual_masses) =
            if prior_event {
                (
                    self.config.residual_after_event_digest,
                    self.config.complete_proposal_after_event_digest,
                    &self.config.residual_after_event_ticks,
                )
            } else {
                (
                    self.config.residual_before_event_digest,
                    self.config.complete_proposal_before_event_digest,
                    &self.config.residual_before_event_ticks,
                )
            };
        if receipt.proposal_digest != expected_proposal_digest
            || receipt.complete_proposal_digest != expected_complete_proposal_digest
        {
            return Err(FiniteProposalExecutionError::UnpredictableProposalSelection);
        }

        let defensive = receipt.branch_ticket < self.config.defensive_mixture_weight_ticks;
        let sampling_masses = if defensive {
            &self.config.target_mass_ticks
        } else {
            residual_masses
        };
        let selected_index = inverse_cdf_atom(sampling_masses, receipt.atom_ticket)
            .ok_or(FiniteProposalExecutionError::InvalidProbabilityVector)?;
        let selected_atom = self.config.atoms[selected_index];
        if receipt.selected_atom_id != selected_atom.atom_id {
            return Err(FiniteProposalExecutionError::SampleTransformMismatch);
        }

        let target_mass = u64::from(self.config.target_mass_ticks[selected_index]);
        let residual_mass = u64::from(residual_masses[selected_index]);
        let alpha = u64::from(self.config.defensive_mixture_weight_ticks);
        let complement = u64::from(TICKET_SPACE - self.config.defensive_mixture_weight_ticks);
        let mixture_mass_numerator = alpha * target_mass + complement * residual_mass;
        if mixture_mass_numerator == 0 {
            return Err(FiniteProposalExecutionError::ZeroMixtureProbability);
        }
        let normalized_weight_numerator = if selected_atom.is_event {
            alpha * target_mass
        } else {
            0
        };
        if normalized_weight_numerator > mixture_mass_numerator {
            return Err(FiniteProposalExecutionError::DefensiveWeightBoundViolated);
        }

        // No state changes occur until every receipt check has succeeded.
        self.seen_sample_ids.insert(receipt.sample_id);
        self.prior_event_history.push(selected_atom.is_event);

        Ok(VerifiedFiniteProposalDraw {
            event_digest: self.config.event_digest,
            target_measure_digest: self.config.target_measure_digest,
            defensive_component_digest: self.config.defensive_component_digest,
            complete_proposal_digest: receipt.complete_proposal_digest,
            defensive_mixture_weight_ticks: self.config.defensive_mixture_weight_ticks,
            sequence_index: receipt.sequence_index,
            sample_id: receipt.sample_id,
            selected_atom_id: selected_atom.atom_id,
            event_occurred: selected_atom.is_event,
            defensive_component_selected: defensive,
            target_mass_ticks: target_mass as u16,
            residual_mass_ticks: residual_mass as u16,
            mixture_mass_numerator,
            normalized_weight_numerator,
            normalized_weight_denominator: mixture_mass_numerator,
        })
    }
}

fn validate_config(
    config: &FiniteAdaptiveProposalConfig,
) -> Result<(), FiniteProposalExecutionError> {
    let identities = [
        &config.event_digest,
        &config.target_measure_digest,
        &config.defensive_component_digest,
        &config.proposal_policy_digest,
        &config.sampler_implementation_digest,
        &config.rng_algorithm_digest,
        &config.rng_seed_commitment_digest,
        &config.branch_stream_digest,
        &config.atom_stream_digest,
        &config.residual_before_event_digest,
        &config.residual_after_event_digest,
        &config.complete_proposal_before_event_digest,
        &config.complete_proposal_after_event_digest,
    ];
    if identities.iter().any(|digest| is_zero_digest(digest)) {
        return Err(FiniteProposalExecutionError::InvalidIdentity);
    }
    if config.branch_stream_digest == config.atom_stream_digest {
        return Err(FiniteProposalExecutionError::RandomStreamsNotDomainSeparated);
    }
    if config.defensive_mixture_weight_ticks == 0
        || config.defensive_mixture_weight_ticks > TICKET_SPACE
    {
        return Err(FiniteProposalExecutionError::InvalidDefensiveMixtureWeight);
    }
    if config.atoms.is_empty()
        || config.atoms.len() > TICKET_SPACE as usize
        || config.target_mass_ticks.len() != config.atoms.len()
        || config.residual_before_event_ticks.len() != config.atoms.len()
        || config.residual_after_event_ticks.len() != config.atoms.len()
    {
        return Err(FiniteProposalExecutionError::InvalidProbabilityVector);
    }
    let mut atom_ids = BTreeSet::new();
    for atom in &config.atoms {
        if is_zero_digest(&atom.atom_id) || !atom_ids.insert(atom.atom_id) {
            return Err(FiniteProposalExecutionError::InvalidAtomIdentity);
        }
    }
    for masses in [
        &config.target_mass_ticks,
        &config.residual_before_event_ticks,
        &config.residual_after_event_ticks,
    ] {
        let total: u32 = masses.iter().map(|mass| u32::from(*mass)).sum();
        if total != u32::from(TICKET_SPACE) {
            return Err(FiniteProposalExecutionError::InvalidProbabilityVector);
        }
    }
    if config.residual_before_event_digest == config.residual_after_event_digest
        && config.residual_before_event_ticks != config.residual_after_event_ticks
    {
        return Err(FiniteProposalExecutionError::InvalidIdentity);
    }
    Ok(())
}

fn same_receipt_scope(
    config: &FiniteAdaptiveProposalConfig,
    receipt: FiniteProposalExecutionReceipt,
) -> bool {
    receipt.event_digest == config.event_digest
        && receipt.target_measure_digest == config.target_measure_digest
        && receipt.defensive_component_digest == config.defensive_component_digest
        && receipt.proposal_policy_digest == config.proposal_policy_digest
        && receipt.sampler_implementation_digest == config.sampler_implementation_digest
        && receipt.rng_algorithm_digest == config.rng_algorithm_digest
        && receipt.rng_seed_commitment_digest == config.rng_seed_commitment_digest
        && receipt.branch_stream_digest == config.branch_stream_digest
        && receipt.atom_stream_digest == config.atom_stream_digest
        && receipt.defensive_mixture_weight_ticks == config.defensive_mixture_weight_ticks
}

/// Return the atom selected by an integer inverse-CDF ticket.
fn inverse_cdf_atom(masses: &[u16], ticket: u16) -> Option<usize> {
    let mut cumulative = 0_u32;
    for (index, mass) in masses.iter().enumerate() {
        cumulative += u32::from(*mass);
        if u32::from(ticket) < cumulative {
            return Some(index);
        }
    }
    None
}

fn is_zero_digest(digest: &[u8; 32]) -> bool {
    digest.iter().all(|byte| *byte == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adaptive_importance_sampling::AdaptiveImportanceSamplingConfidenceSequence;

    const EVENT: [u8; 32] = [1; 32];
    const TARGET: [u8; 32] = [2; 32];
    const DEFENSIVE: [u8; 32] = [3; 32];
    const POLICY: [u8; 32] = [4; 32];
    const SAMPLER: [u8; 32] = [5; 32];
    const RNG: [u8; 32] = [6; 32];
    const SEED: [u8; 32] = [7; 32];
    const BRANCH_STREAM: [u8; 32] = [8; 32];
    const ATOM_STREAM: [u8; 32] = [9; 32];
    const RESIDUAL_BEFORE: [u8; 32] = [10; 32];
    const RESIDUAL_AFTER: [u8; 32] = [11; 32];
    const COMPLETE_BEFORE: [u8; 32] = [12; 32];
    const COMPLETE_AFTER: [u8; 32] = [13; 32];

    fn config() -> FiniteAdaptiveProposalConfig {
        FiniteAdaptiveProposalConfig {
            event_digest: EVENT,
            target_measure_digest: TARGET,
            defensive_component_digest: DEFENSIVE,
            proposal_policy_digest: POLICY,
            sampler_implementation_digest: SAMPLER,
            rng_algorithm_digest: RNG,
            rng_seed_commitment_digest: SEED,
            branch_stream_digest: BRANCH_STREAM,
            atom_stream_digest: ATOM_STREAM,
            residual_before_event_digest: RESIDUAL_BEFORE,
            residual_after_event_digest: RESIDUAL_AFTER,
            complete_proposal_before_event_digest: COMPLETE_BEFORE,
            complete_proposal_after_event_digest: COMPLETE_AFTER,
            defensive_mixture_weight_ticks: 64,
            atoms: vec![
                FiniteProposalAtom {
                    atom_id: [20; 32],
                    is_event: true,
                },
                FiniteProposalAtom {
                    atom_id: [21; 32],
                    is_event: false,
                },
                FiniteProposalAtom {
                    atom_id: [22; 32],
                    is_event: true,
                },
                FiniteProposalAtom {
                    atom_id: [23; 32],
                    is_event: false,
                },
            ],
            target_mass_ticks: vec![64, 64, 64, 64],
            residual_before_event_ticks: vec![128, 64, 32, 32],
            residual_after_event_ticks: vec![32, 32, 64, 128],
        }
    }

    fn receipt(
        sequence_index: u64,
        sample_id: [u8; 32],
        proposal_digest: [u8; 32],
        branch_ticket: u16,
        atom_ticket: u16,
        selected_atom_index: usize,
    ) -> FiniteProposalExecutionReceipt {
        let c = config();
        FiniteProposalExecutionReceipt {
            sequence_index,
            sample_id,
            event_digest: c.event_digest,
            target_measure_digest: c.target_measure_digest,
            defensive_component_digest: c.defensive_component_digest,
            proposal_policy_digest: c.proposal_policy_digest,
            sampler_implementation_digest: c.sampler_implementation_digest,
            rng_algorithm_digest: c.rng_algorithm_digest,
            rng_seed_commitment_digest: c.rng_seed_commitment_digest,
            branch_stream_digest: c.branch_stream_digest,
            atom_stream_digest: c.atom_stream_digest,
            proposal_digest,
            complete_proposal_digest: if proposal_digest == RESIDUAL_AFTER {
                COMPLETE_AFTER
            } else {
                COMPLETE_BEFORE
            },
            defensive_mixture_weight_ticks: c.defensive_mixture_weight_ticks,
            branch_ticket,
            atom_ticket,
            selected_atom_id: c.atoms[selected_atom_index].atom_id,
        }
    }

    fn expected_atom(masses: &[u16], ticket: u16) -> usize {
        let mut total = 0_u32;
        for (index, mass) in masses.iter().enumerate() {
            total += u32::from(*mass);
            if u32::from(ticket) < total {
                return index;
            }
        }
        unreachable!("test probability vector is normalized")
    }

    #[test]
    fn every_branch_and_atom_ticket_reconstructs_the_exact_defensive_mixture() {
        let c = config();
        let alpha = u64::from(c.defensive_mixture_weight_ticks);
        let complement = u64::from(TICKET_SPACE - c.defensive_mixture_weight_ticks);
        let mut selected_counts = [0_u64; 4];
        let mut defensive_branch_count = 0_u64;

        for branch_ticket in 0..TICKET_SPACE {
            for atom_ticket in 0..TICKET_SPACE {
                let defensive = branch_ticket < c.defensive_mixture_weight_ticks;
                let component = if defensive {
                    &c.target_mass_ticks
                } else {
                    &c.residual_before_event_ticks
                };
                let index = expected_atom(component, atom_ticket);
                let execution = receipt(
                    0,
                    [100; 32],
                    RESIDUAL_BEFORE,
                    branch_ticket,
                    atom_ticket,
                    index,
                );
                let mut verifier = FiniteProposalExecutionVerifier::new(c.clone()).unwrap();
                let verified = verifier.verify_next(execution).unwrap();

                assert_eq!(verified.selected_atom_id, c.atoms[index].atom_id);
                assert_eq!(verified.defensive_component_selected, defensive);
                assert_eq!(
                    verified.mixture_mass_numerator,
                    alpha * u64::from(c.target_mass_ticks[index])
                        + complement * u64::from(c.residual_before_event_ticks[index])
                );
                assert!(
                    verified.normalized_weight_numerator <= verified.normalized_weight_denominator
                );
                assert_eq!(
                    verified.normalized_weight_numerator,
                    if c.atoms[index].is_event {
                        alpha * u64::from(c.target_mass_ticks[index])
                    } else {
                        0
                    }
                );
                selected_counts[index] += 1;
                if defensive {
                    defensive_branch_count += 1;
                }
            }
        }

        for (index, sample_count) in selected_counts.iter().copied().enumerate() {
            let exact_mixture_count = alpha * u64::from(c.target_mass_ticks[index])
                + complement * u64::from(c.residual_before_event_ticks[index]);
            assert_eq!(sample_count, exact_mixture_count);
        }
        let exact_reward_sum = c
            .atoms
            .iter()
            .zip(&c.target_mass_ticks)
            .zip(&c.residual_before_event_ticks)
            .zip(&selected_counts)
            .filter(|(((atom, _), _), _)| atom.is_event)
            .map(|(((_, target_mass), residual_mass), sample_count)| {
                let q_count = alpha * u64::from(*target_mass)
                    + complement * u64::from(*residual_mass);
                let weighted_numerator = alpha * u64::from(*target_mass);
                assert_eq!(*sample_count, q_count);
                assert_eq!((*sample_count * weighted_numerator) % q_count, 0);
                *sample_count * weighted_numerator / q_count
            })
            .sum::<u64>();
        let event_target_mass = c
            .atoms
            .iter()
            .zip(&c.target_mass_ticks)
            .filter(|(atom, _)| atom.is_event)
            .map(|(_, mass)| u64::from(*mass))
            .sum::<u64>();
        assert_eq!(exact_reward_sum, alpha * event_target_mass);
        assert_eq!(exact_reward_sum * 8, 65_536);
        assert_eq!(
            defensive_branch_count,
            u64::from(c.defensive_mixture_weight_ticks) * u64::from(TICKET_SPACE)
        );
    }

    #[test]
    fn next_residual_proposal_is_selected_only_from_the_prior_event_outcome() {
        let mut verifier = FiniteProposalExecutionVerifier::new(config()).unwrap();
        let first = receipt(0, [101; 32], RESIDUAL_BEFORE, 0, 0, 0);
        let first_verified = verifier.verify_next(first).unwrap();
        assert!(first_verified.event_occurred);
        assert_eq!(verifier.sample_count(), 1);

        let wrong_next = receipt(1, [102; 32], RESIDUAL_BEFORE, 100, 200, 3);
        assert_eq!(
            verifier.verify_next(wrong_next),
            Err(FiniteProposalExecutionError::UnpredictableProposalSelection)
        );
        assert_eq!(verifier.sample_count(), 1);

        let correct_next = receipt(1, [103; 32], RESIDUAL_AFTER, 100, 200, 3);
        let second_verified = verifier.verify_next(correct_next).unwrap();
        assert_eq!(second_verified.residual_mass_ticks, 128);
        assert!(!second_verified.event_occurred);
        assert_eq!(verifier.sample_count(), 2);
    }

    #[test]
    fn a_forged_atom_identity_and_out_of_range_ticket_are_rejected_transactionally() {
        let c = config();
        let mut verifier = FiniteProposalExecutionVerifier::new(c).unwrap();
        let mut forged = receipt(0, [104; 32], RESIDUAL_BEFORE, 0, 0, 0);
        forged.selected_atom_id = [99; 32];
        assert_eq!(
            verifier.verify_next(forged),
            Err(FiniteProposalExecutionError::SampleTransformMismatch)
        );
        assert_eq!(verifier.sample_count(), 0);

        let mut out_of_range = receipt(0, [105; 32], RESIDUAL_BEFORE, 0, 0, 0);
        out_of_range.atom_ticket = TICKET_SPACE;
        assert_eq!(
            verifier.verify_next(out_of_range),
            Err(FiniteProposalExecutionError::TicketOutsideRange)
        );
        assert_eq!(verifier.sample_count(), 0);
    }

    #[test]
    fn duplicate_ids_scope_changes_and_sequence_gaps_are_rejected() {
        let mut verifier = FiniteProposalExecutionVerifier::new(config()).unwrap();
        let first = receipt(0, [106; 32], RESIDUAL_BEFORE, 0, 0, 0);
        verifier.verify_next(first).unwrap();
        let duplicate = receipt(1, [106; 32], RESIDUAL_BEFORE, 0, 0, 0);
        assert_eq!(
            verifier.verify_next(duplicate),
            Err(FiniteProposalExecutionError::DuplicateOrInvalidSampleIdentity)
        );
        let wrong_sequence = receipt(2, [107; 32], RESIDUAL_BEFORE, 0, 0, 0);
        assert_eq!(
            verifier.verify_next(wrong_sequence),
            Err(FiniteProposalExecutionError::SequenceMismatch)
        );

        let mut wrong_scope = receipt(1, [108; 32], RESIDUAL_BEFORE, 0, 0, 0);
        wrong_scope.target_measure_digest = [77; 32];
        assert_eq!(
            verifier.verify_next(wrong_scope),
            Err(FiniteProposalExecutionError::ReceiptScopeMismatch)
        );
    }

    #[test]
    fn complete_mixture_density_not_selected_component_density_is_used() {
        let c = config();
        let mut verifier = FiniteProposalExecutionVerifier::new(c.clone()).unwrap();
        // Residual branch, atom zero: r=128, p=64, alpha=64.
        let draw = receipt(0, [109; 32], RESIDUAL_BEFORE, 100, 0, 0);
        let verified = verifier.verify_next(draw).unwrap();
        assert!(!verified.defensive_component_selected);
        assert_eq!(verified.target_mass_ticks, 64);
        assert_eq!(verified.residual_mass_ticks, 128);
        assert_eq!(verified.mixture_mass_numerator, 64 * 64 + 192 * 128);
        assert_eq!(verified.normalized_weight_numerator, 64 * 64);
        assert_eq!(
            verified.normalized_contribution(),
            (64 * 64, 64 * 64 + 192 * 128)
        );
        assert!(verified.normalized_weight_numerator < verified.mixture_mass_numerator);
    }

    #[test]
    fn invalid_probability_ledgers_and_non_separated_streams_are_rejected() {
        let mut invalid = config();
        invalid.target_mass_ticks[0] -= 1;
        assert!(matches!(
            FiniteProposalExecutionVerifier::new(invalid),
            Err(FiniteProposalExecutionError::InvalidProbabilityVector)
        ));

        let mut shared_stream = config();
        shared_stream.atom_stream_digest = shared_stream.branch_stream_digest;
        assert!(matches!(
            FiniteProposalExecutionVerifier::new(shared_stream),
            Err(FiniteProposalExecutionError::RandomStreamsNotDomainSeparated)
        ));

        let mut zero_alpha = config();
        zero_alpha.defensive_mixture_weight_ticks = 0;
        assert!(matches!(
            FiniteProposalExecutionVerifier::new(zero_alpha),
            Err(FiniteProposalExecutionError::InvalidDefensiveMixtureWeight)
        ));
    }

    #[test]
    fn a_failed_receipt_does_not_consume_its_sample_identity() {
        let mut verifier = FiniteProposalExecutionVerifier::new(config()).unwrap();
        let mut receipt = receipt(0, [110; 32], RESIDUAL_BEFORE, 0, 0, 0);
        receipt.proposal_policy_digest = [77; 32];
        assert_eq!(
            verifier.verify_next(receipt),
            Err(FiniteProposalExecutionError::ReceiptScopeMismatch)
        );
        receipt.proposal_policy_digest = POLICY;
        assert!(verifier.verify_next(receipt).is_ok());
        assert_eq!(verifier.sample_count(), 1);
    }

    #[test]
    fn verified_finite_draw_converts_into_the_existing_confidence_sequence_adapter() {
        let mut verifier = FiniteProposalExecutionVerifier::new(config()).unwrap();
        let receipt = receipt(
            0,
            [120; 32],
            RESIDUAL_BEFORE,
            100,
            0,
            0,
        );
        let verified = verifier.verify_next(receipt).unwrap();
        let draw = verified.into_adaptive_importance_draw();

        assert_eq!(draw.event_digest, EVENT);
        assert_eq!(draw.target_measure_digest, TARGET);
        assert_eq!(draw.defensive_component_digest, DEFENSIVE);
        assert_eq!(draw.proposal_digest, COMPLETE_BEFORE);
        assert!((draw.defensive_mixture_weight - 0.25).abs() < f64::EPSILON);
        assert!((draw.log_target_density.exp() - 0.25).abs() < 1.0e-15);
        assert!((draw.log_proposal_density.exp() - 0.4375).abs() < 1.0e-15);

        let mut sequence = AdaptiveImportanceSamplingConfidenceSequence::new(
            EVENT,
            TARGET,
            DEFENSIVE,
            0.25,
            0.05,
        )
        .unwrap();
        sequence.observe(draw).unwrap();
        let summary = sequence.summary().unwrap();
        assert_eq!(summary.sample_count, 1);
        assert!((summary.normalized_mean_estimate - (1.0 / 7.0)).abs() < 1.0e-12);
    }

    #[test]
    fn zero_target_density_non_event_converts_to_negative_infinity_and_is_accepted() {
        let mut c = config();
        c.target_mass_ticks = vec![256, 0, 0, 0];
        let mut verifier = FiniteProposalExecutionVerifier::new(c).unwrap();
        let execution = receipt(
            0,
            [121; 32],
            RESIDUAL_BEFORE,
            100,
            130,
            1,
        );
        let verified = verifier.verify_next(execution).unwrap();
        assert_eq!(verified.target_mass_ticks, 0);
        assert!(!verified.event_occurred);

        let draw = verified.into_adaptive_importance_draw();
        assert_eq!(draw.log_target_density, f64::NEG_INFINITY);
        assert!(draw.log_proposal_density.is_finite());

        let mut sequence = AdaptiveImportanceSamplingConfidenceSequence::new(
            EVENT,
            TARGET,
            DEFENSIVE,
            0.25,
            0.05,
        )
        .unwrap();
        sequence.observe(draw).unwrap();
        assert_eq!(sequence.summary().unwrap().normalized_mean_estimate, 0.0);
    }

    #[test]
    fn complete_proposal_identity_is_checked_separately_from_residual_identity() {
        let mut verifier = FiniteProposalExecutionVerifier::new(config()).unwrap();
        let mut forged = receipt(
            0,
            [122; 32],
            RESIDUAL_BEFORE,
            100,
            0,
            0,
        );
        forged.complete_proposal_digest = COMPLETE_AFTER;
        assert_eq!(
            verifier.verify_next(forged),
            Err(FiniteProposalExecutionError::UnpredictableProposalSelection)
        );
        assert_eq!(verifier.sample_count(), 0);
    }

}
