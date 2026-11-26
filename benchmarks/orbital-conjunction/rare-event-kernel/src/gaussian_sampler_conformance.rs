// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic replay conformance for continuous defensive Gaussian proposals.
//!
//! A receipt carries a component-selection ticket and a standard-normal vector.
//! The verifier replays the frozen mixture's inverse-CDF component selection,
//! reconstructs x = mu + Lz, compares the claimed point bit-for-bit, and
//! recomputes target/full-mixture densities and the bounded defensive
//! contribution.
//!
//! This verifies the deterministic transform only. It does not prove the
//! component ticket is uniform, the normal vector is standard normal, or the
//! declared RNG/seed source actually generated either value.

use std::collections::BTreeSet;

use sha2::{Digest, Sha256};

use crate::portfolio_proposal::{
    DefensiveGaussianSampleTransform, PortfolioDefensiveGaussianMixture,
    PortfolioEventModeKey, PortfolioProposalError,
};

/// Frozen identities for the external random source and sampler transform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GaussianProposalSamplerContract {
    /// Stable identity of the random-word / normal generator algorithm.
    pub rng_algorithm_digest: [u8; 32],
    /// Commitment to the seed or entropy source used for the campaign.
    pub rng_seed_commitment_digest: [u8; 32],
    /// Domain-separated stream identity for component-selection tickets.
    pub component_stream_digest: [u8; 32],
    /// Domain-separated stream identity for standard-normal variates.
    pub normal_stream_digest: [u8; 32],
    /// Identity of the exact sampler implementation/build under test.
    pub sampler_implementation_digest: [u8; 32],
}

/// One externally captured continuous Gaussian proposal sampling receipt.
#[derive(Debug, Clone, PartialEq)]
pub struct GaussianProposalExecutionReceipt {
    /// One-based, consecutive sample index beginning at one.
    pub sequence_index: u64,
    /// Stable nonzero identity assigned by the sampling system.
    pub sample_id: [u8; 32],
    /// Target probability measure identity.
    pub target_measure_id: [u8; 32],
    /// Density-space identity (frame, units, epoch and coordinates).
    pub density_space_id: [u8; 32],
    /// Single epistemic scenario identity.
    pub scenario_id: [u8; 32],
    /// Frozen full defensive-mixture configuration digest.
    pub proposal_configuration_digest: [u8; 32],
    /// RNG algorithm identity.
    pub rng_algorithm_digest: [u8; 32],
    /// Seed/entropy-source commitment.
    pub rng_seed_commitment_digest: [u8; 32],
    /// Component-selection random stream identity.
    pub component_stream_digest: [u8; 32],
    /// Standard-normal random stream identity.
    pub normal_stream_digest: [u8; 32],
    /// Exact sampler implementation identity.
    pub sampler_implementation_digest: [u8; 32],
    /// Ticket in [0,1) used to select a component in the complete flattened q.
    pub component_ticket: f64,
    /// Standard-normal vector used by x = mean + Lz.
    pub standard_normal_variates: Vec<f64>,
    /// Component identity selected by the deterministic inverse-CDF transform.
    pub selected_component_id: [u8; 32],
    /// True iff the selected component is part of the defensive target mixture.
    pub defensive_target_component_selected: bool,
    /// Mode key carried by a selected residual proposal component, else None.
    pub selected_proposal_mode_key: Option<PortfolioEventModeKey>,
    /// Caller-claimed sample point; verifier reconstructs it independently.
    pub claimed_point: Vec<f64>,
    /// Outcome/event-mode key associated with the directly evaluated contrast.
    pub outcome_mode_key: PortfolioEventModeKey,
    /// Directly evaluated consequence, already normalized into [0,1].
    pub normalized_contrast: f64,
}

/// Independently reconstructed and density-checked Gaussian proposal draw.
#[derive(Debug, Clone, PartialEq)]
pub struct VerifiedGaussianProposalDraw {
    /// One-based accepted sample index.
    pub sequence_index: u64,
    /// Sample identity from the execution receipt.
    pub sample_id: [u8; 32],
    /// Domain-separated SHA-256 digest of the complete receipt content.
    pub receipt_digest: [u8; 32],
    /// Deterministically reconstructed component index.
    pub selected_component_index: usize,
    /// Deterministically reconstructed component identity.
    pub selected_component_id: [u8; 32],
    /// True iff the defensive target component generated the sample.
    pub defensive_target_component_selected: bool,
    /// Residual proposal mode key, absent for a target-component draw.
    pub selected_proposal_mode_key: Option<PortfolioEventModeKey>,
    /// Outcome mode key to which the direct contrast applies.
    pub outcome_mode_key: PortfolioEventModeKey,
    /// Bit-exact replayed sample point.
    pub sampled_point: Vec<f64>,
    /// Log target density p(x).
    pub log_target_density: f64,
    /// Log density of the complete defensive mixture q(x).
    pub log_full_proposal_density: f64,
    /// Log target / full-proposal likelihood ratio.
    pub log_importance_weight: f64,
    /// Normalized direct contrast in [0,1].
    pub normalized_contrast: f64,
    /// Bounded defensive observation epsilon * g(x) * p(x)/q(x).
    pub defensive_contribution: f64,
    /// Frozen proposal configuration digest.
    pub proposal_configuration_digest: [u8; 32],
}

/// Fail-closed sampler receipt, identity or replay errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaussianSamplerConformanceError {
    /// A required RNG, seed, stream or sampler identity is zero.
    InvalidContractIdentity,
    /// Component and normal variate streams are not domain-separated.
    RandomStreamsNotDomainSeparated,
    /// Receipt identity fields do not match the pinned proposal/sampler contract.
    ReceiptScopeMismatch,
    /// Sample ID is zero or already accepted by this verifier.
    InvalidOrDuplicateSampleIdentity,
    /// Sample index is skipped, repeated, reordered or exceeds the bound.
    InvalidSequenceIndex,
    /// Claimed selected component or its mode differs from replay.
    ComponentSelectionMismatch,
    /// Claimed sample point differs bit-for-bit from mean + Lz.
    SampleTransformMismatch,
    /// Proposal density evaluation or bounded contribution failed.
    ProposalEvaluationFailure,
    /// The deterministic sample transform failed.
    SampleTransformFailure,
    /// Receipt digest could not be computed consistently.
    ReceiptDigestFailure,
}

impl core::fmt::Display for GaussianSamplerConformanceError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidContractIdentity => "sampler contract contains a zero identity",
            Self::RandomStreamsNotDomainSeparated => {
                "component and normal random streams are not domain-separated"
            }
            Self::ReceiptScopeMismatch => "sampler receipt does not match the frozen proposal contract",
            Self::InvalidOrDuplicateSampleIdentity => "sample identity is zero or duplicated",
            Self::InvalidSequenceIndex => "sampler receipt is not the next sequence index",
            Self::ComponentSelectionMismatch => "selected proposal component differs from deterministic replay",
            Self::SampleTransformMismatch => "claimed point differs from deterministic Gaussian transform",
            Self::ProposalEvaluationFailure => "full-mixture proposal evaluation failed",
            Self::SampleTransformFailure => "Gaussian mixture sample transform failed",
            Self::ReceiptDigestFailure => "sampler receipt digest calculation failed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for GaussianSamplerConformanceError {}

impl From<PortfolioProposalError> for GaussianSamplerConformanceError {
    fn from(_: PortfolioProposalError) -> Self {
        Self::SampleTransformFailure
    }
}

/// Stateful exact-transform replay verifier for one immutable defensive mixture.
#[derive(Debug, Clone)]
pub struct GaussianProposalExecutionVerifier {
    proposal: PortfolioDefensiveGaussianMixture,
    contract: GaussianProposalSamplerContract,
    accepted_sample_count: u64,
    seen_sample_ids: BTreeSet<[u8; 32]>,
}

impl GaussianProposalExecutionVerifier {
    /// Pin one immutable proposal mixture and the external sampler identities.
    pub fn new(
        proposal: PortfolioDefensiveGaussianMixture,
        contract: GaussianProposalSamplerContract,
    ) -> Result<Self, GaussianSamplerConformanceError> {
        let identities = [
            contract.rng_algorithm_digest,
            contract.rng_seed_commitment_digest,
            contract.component_stream_digest,
            contract.normal_stream_digest,
            contract.sampler_implementation_digest,
        ];
        if identities
            .iter()
            .any(|identity| identity.iter().all(|byte| *byte == 0))
        {
            return Err(GaussianSamplerConformanceError::InvalidContractIdentity);
        }
        if contract.component_stream_digest == contract.normal_stream_digest {
            return Err(GaussianSamplerConformanceError::RandomStreamsNotDomainSeparated);
        }
        Ok(Self {
            proposal,
            contract,
            accepted_sample_count: 0,
            seen_sample_ids: BTreeSet::new(),
        })
    }

    /// Number of sampler receipts accepted so far.
    pub fn sample_count(&self) -> u64 {
        self.accepted_sample_count
    }

    /// Replay and verify the next continuous Gaussian proposal sample.
    ///
    /// No caller-supplied densities or importance weights are accepted. The
    /// verifier reconstructs the selected component and point, recomputes
    /// p(x) and the complete q(x), then evaluates epsilon * g(x) * p(x)/q(x).
    ///
    /// The receipt's random values are treated as inputs. This verifies the
    /// deterministic transformation and scope binding, not the statistical
    /// distribution or authenticity of the external RNG source.
    pub fn verify_next(
        &mut self,
        receipt: &GaussianProposalExecutionReceipt,
    ) -> Result<VerifiedGaussianProposalDraw, GaussianSamplerConformanceError> {
        let expected_index = self
            .accepted_sample_count
            .checked_add(1)
            .ok_or(GaussianSamplerConformanceError::InvalidSequenceIndex)?;
        if receipt.sequence_index != expected_index
            || receipt.sequence_index > 1_000_000
        {
            return Err(GaussianSamplerConformanceError::InvalidSequenceIndex);
        }
        if receipt.sample_id.iter().all(|byte| *byte == 0)
            || self.seen_sample_ids.contains(&receipt.sample_id)
        {
            return Err(GaussianSamplerConformanceError::InvalidOrDuplicateSampleIdentity);
        }

        if receipt.target_measure_id != self.proposal.target_measure_id()
            || receipt.density_space_id != self.proposal.density_space_id()
            || receipt.scenario_id != self.proposal.scenario_id()
            || receipt.proposal_configuration_digest != self.proposal.configuration_digest()
            || receipt.rng_algorithm_digest != self.contract.rng_algorithm_digest
            || receipt.rng_seed_commitment_digest != self.contract.rng_seed_commitment_digest
            || receipt.component_stream_digest != self.contract.component_stream_digest
            || receipt.normal_stream_digest != self.contract.normal_stream_digest
            || receipt.sampler_implementation_digest != self.contract.sampler_implementation_digest
        {
            return Err(GaussianSamplerConformanceError::ReceiptScopeMismatch);
        }

        let replay = self
            .proposal
            .replay_sample_transform(
                receipt.component_ticket,
                &receipt.standard_normal_variates,
            )
            .map_err(|_| GaussianSamplerConformanceError::SampleTransformFailure)?;

        if receipt.selected_component_id != replay.selected_component_id
            || receipt.defensive_target_component_selected
                != replay.defensive_target_component_selected
            || receipt.selected_proposal_mode_key != replay.selected_proposal_mode_key
        {
            return Err(GaussianSamplerConformanceError::ComponentSelectionMismatch);
        }
        if !same_f64_bits(&receipt.claimed_point, &replay.sampled_point) {
            return Err(GaussianSamplerConformanceError::SampleTransformMismatch);
        }

        let evaluation = self
            .proposal
            .evaluate_draw(
                receipt.outcome_mode_key,
                &replay.sampled_point,
                receipt.normalized_contrast,
            )
            .map_err(|_| GaussianSamplerConformanceError::ProposalEvaluationFailure)?;
        let receipt_digest = gaussian_receipt_digest(receipt)?;

        // Commit only after every scope, transform, density and bounded-weight check.
        self.seen_sample_ids.insert(receipt.sample_id);
        self.accepted_sample_count = expected_index;

        Ok(VerifiedGaussianProposalDraw {
            sequence_index: receipt.sequence_index,
            sample_id: receipt.sample_id,
            receipt_digest,
            selected_component_index: replay.selected_component_index,
            selected_component_id: replay.selected_component_id,
            defensive_target_component_selected: replay.defensive_target_component_selected,
            selected_proposal_mode_key: replay.selected_proposal_mode_key,
            outcome_mode_key: receipt.outcome_mode_key,
            sampled_point: replay.sampled_point,
            log_target_density: evaluation.log_target_density,
            log_full_proposal_density: evaluation.log_full_proposal_density,
            log_importance_weight: evaluation.log_importance_weight,
            normalized_contrast: receipt.normalized_contrast,
            defensive_contribution: evaluation.defensive_contribution,
            proposal_configuration_digest: self.proposal.configuration_digest(),
        })
    }
}

fn same_f64_bits(left: &[f64], right: &[f64]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| left.to_bits() == right.to_bits())
}

fn gaussian_receipt_digest(
    receipt: &GaussianProposalExecutionReceipt,
) -> Result<[u8; 32], GaussianSamplerConformanceError> {
    let mut hasher = Sha256::new();
    hasher.update(b"luminous.orbital-gaussian-proposal-execution-receipt.v0.1\0");
    hasher.update(receipt.sequence_index.to_be_bytes());
    hasher.update(receipt.sample_id);
    hasher.update(receipt.target_measure_id);
    hasher.update(receipt.density_space_id);
    hasher.update(receipt.scenario_id);
    hasher.update(receipt.proposal_configuration_digest);
    hasher.update(receipt.rng_algorithm_digest);
    hasher.update(receipt.rng_seed_commitment_digest);
    hasher.update(receipt.component_stream_digest);
    hasher.update(receipt.normal_stream_digest);
    hasher.update(receipt.sampler_implementation_digest);
    hasher.update(receipt.component_ticket.to_bits().to_be_bytes());
    hasher.update((receipt.standard_normal_variates.len() as u64).to_be_bytes());
    for value in &receipt.standard_normal_variates {
        hasher.update(value.to_bits().to_be_bytes());
    }
    hasher.update(receipt.selected_component_id);
    hasher.update([u8::from(receipt.defensive_target_component_selected)]);
    match receipt.selected_proposal_mode_key {
        Some(key) => {
            hasher.update([1]);
            hasher.update(key.portfolio_id);
            hasher.update(key.scenario_id);
            hasher.update(key.event_mode_id);
        }
        None => {
            hasher.update([0]);
        }
    }
    hasher.update((receipt.claimed_point.len() as u64).to_be_bytes());
    for value in &receipt.claimed_point {
        hasher.update(value.to_bits().to_be_bytes());
    }
    hasher.update(receipt.outcome_mode_key.portfolio_id);
    hasher.update(receipt.outcome_mode_key.scenario_id);
    hasher.update(receipt.outcome_mode_key.event_mode_id);
    hasher.update(receipt.normalized_contrast.to_bits().to_be_bytes());
    Ok(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DefensiveProposalComponent, Gaussian, GaussianComponent, GaussianMixture,
    };

    const TARGET_MEASURE: [u8; 32] = [90; 32];
    const DENSITY_SPACE: [u8; 32] = [91; 32];
    const RNG_ALGORITHM: [u8; 32] = [92; 32];
    const SEED_COMMITMENT: [u8; 32] = [93; 32];
    const COMPONENT_STREAM: [u8; 32] = [94; 32];
    const NORMAL_STREAM: [u8; 32] = [95; 32];
    const SAMPLER_IMPLEMENTATION: [u8; 32] = [96; 32];

    fn gaussian(mean: f64, variance: f64) -> Gaussian {
        Gaussian::new(vec![mean], vec![variance]).unwrap()
    }

    fn key(portfolio: u8) -> PortfolioEventModeKey {
        PortfolioEventModeKey {
            portfolio_id: [portfolio; 32],
            scenario_id: [1; 32],
            event_mode_id: [portfolio; 32],
        }
    }

    fn proposal_component(
        component_id: u8,
        portfolio: u8,
        weight: f64,
        mean: f64,
    ) -> DefensiveProposalComponent {
        DefensiveProposalComponent {
            component_id: [component_id; 32],
            mode_key: key(portfolio),
            allocation_weight: weight,
            distribution: gaussian(mean, 1.0),
        }
    }

    fn proposal() -> PortfolioDefensiveGaussianMixture {
        let target = GaussianMixture::new(vec![
            GaussianComponent::new(0.5, gaussian(-1.0, 1.0)).unwrap(),
            GaussianComponent::new(0.5, gaussian(1.0, 1.0)).unwrap(),
        ])
        .unwrap();
        PortfolioDefensiveGaussianMixture::new(
            TARGET_MEASURE,
            DENSITY_SPACE,
            target,
            0.2,
            vec![key(10), key(20)],
            vec![
                proposal_component(1, 10, 0.5, -5.0),
                proposal_component(2, 20, 0.5, 5.0),
            ],
        )
        .unwrap()
    }

    fn contract() -> GaussianProposalSamplerContract {
        GaussianProposalSamplerContract {
            rng_algorithm_digest: RNG_ALGORITHM,
            rng_seed_commitment_digest: SEED_COMMITMENT,
            component_stream_digest: COMPONENT_STREAM,
            normal_stream_digest: NORMAL_STREAM,
            sampler_implementation_digest: SAMPLER_IMPLEMENTATION,
        }
    }

    fn receipt(
        proposal: &PortfolioDefensiveGaussianMixture,
        sequence_index: u64,
        sample_id: u8,
        component_ticket: f64,
        normal: f64,
        outcome_mode_key: PortfolioEventModeKey,
    ) -> GaussianProposalExecutionReceipt {
        let transform = proposal.replay_sample_transform(component_ticket, &[normal]).unwrap();
        let mut sample_identity = [0_u8; 32];
        sample_identity[0] = sample_id;
        GaussianProposalExecutionReceipt {
            sequence_index,
            sample_id: sample_identity,
            target_measure_id: proposal.target_measure_id(),
            density_space_id: proposal.density_space_id(),
            scenario_id: proposal.scenario_id(),
            proposal_configuration_digest: proposal.configuration_digest(),
            rng_algorithm_digest: RNG_ALGORITHM,
            rng_seed_commitment_digest: SEED_COMMITMENT,
            component_stream_digest: COMPONENT_STREAM,
            normal_stream_digest: NORMAL_STREAM,
            sampler_implementation_digest: SAMPLER_IMPLEMENTATION,
            component_ticket,
            standard_normal_variates: vec![normal],
            selected_component_id: transform.selected_component_id,
            defensive_target_component_selected: transform.defensive_target_component_selected,
            selected_proposal_mode_key: transform.selected_proposal_mode_key,
            claimed_point: transform.sampled_point,
            outcome_mode_key,
            normalized_contrast: 0.75,
        }
    }

    #[test]
    fn verifies_and_recomputes_full_mixture_density_and_bounded_contribution() {
        let proposal = proposal();
        let mut verifier = GaussianProposalExecutionVerifier::new(proposal.clone(), contract()).unwrap();
        let receipt = receipt(&proposal, 1, 1, 0.3, 0.5, key(10));
        let verified = verifier.verify_next(&receipt).unwrap();

        assert_eq!(verified.sequence_index, 1);
        assert_eq!(verified.sample_id, receipt.sample_id);
        assert_eq!(verified.sampled_point, receipt.claimed_point);
        assert!(!verified.defensive_target_component_selected);
        assert_eq!(verified.selected_proposal_mode_key, Some(key(10)));
        assert!(verified.log_target_density.is_finite());
        assert!(verified.log_full_proposal_density.is_finite());
        assert!(
            verified.log_importance_weight
                <= -proposal.defensive_fraction().ln() + 1.0e-12
        );
        assert!((0.0..=1.0).contains(&verified.defensive_contribution));
        assert_ne!(verified.receipt_digest, [0; 32]);
        assert_eq!(verifier.sample_count(), 1);
    }

    #[test]
    fn verifies_defensive_target_component_without_residual_mode_identity() {
        let proposal = proposal();
        let mut verifier = GaussianProposalExecutionVerifier::new(proposal.clone(), contract()).unwrap();
        let receipt = receipt(&proposal, 1, 2, 0.0, -0.25, key(20));
        let verified = verifier.verify_next(&receipt).unwrap();
        assert!(verified.defensive_target_component_selected);
        assert_eq!(verified.selected_proposal_mode_key, None);
    }

    #[test]
    fn rejects_one_ulp_point_mismatch_and_does_not_commit() {
        let proposal = proposal();
        let mut verifier = GaussianProposalExecutionVerifier::new(proposal.clone(), contract()).unwrap();
        let mut bad = receipt(&proposal, 1, 3, 0.3, 0.5, key(10));
        bad.claimed_point[0] = f64::from_bits(bad.claimed_point[0].to_bits() + 1);

        assert_eq!(
            verifier.verify_next(&bad),
            Err(GaussianSamplerConformanceError::SampleTransformMismatch)
        );
        assert_eq!(verifier.sample_count(), 0);
        let good = receipt(&proposal, 1, 4, 0.3, 0.5, key(10));
        verifier.verify_next(&good).unwrap();
        assert_eq!(verifier.sample_count(), 1);
    }

    #[test]
    fn rejects_component_identity_substitution_and_scope_mismatch() {
        let proposal = proposal();
        let mut verifier = GaussianProposalExecutionVerifier::new(proposal.clone(), contract()).unwrap();
        let mut wrong_component = receipt(&proposal, 1, 5, 0.3, 0.5, key(10));
        wrong_component.selected_component_id = [7; 32];
        assert_eq!(
            verifier.verify_next(&wrong_component),
            Err(GaussianSamplerConformanceError::ComponentSelectionMismatch)
        );

        let mut wrong_rng = receipt(&proposal, 1, 6, 0.3, 0.5, key(10));
        wrong_rng.rng_algorithm_digest = [8; 32];
        assert_eq!(
            verifier.verify_next(&wrong_rng),
            Err(GaussianSamplerConformanceError::ReceiptScopeMismatch)
        );
        assert_eq!(verifier.sample_count(), 0);
    }

    #[test]
    fn rejects_duplicate_sample_ids_and_reordered_sequence_indices() {
        let proposal = proposal();
        let mut verifier = GaussianProposalExecutionVerifier::new(proposal.clone(), contract()).unwrap();
        let first = receipt(&proposal, 1, 7, 0.3, 0.5, key(10));
        verifier.verify_next(&first).unwrap();

        let duplicate = receipt(&proposal, 2, 7, 0.6, 0.0, key(20));
        assert_eq!(
            verifier.verify_next(&duplicate),
            Err(GaussianSamplerConformanceError::InvalidOrDuplicateSampleIdentity)
        );
        let skipped = receipt(&proposal, 3, 8, 0.3, 0.5, key(10));
        assert_eq!(
            verifier.verify_next(&skipped),
            Err(GaussianSamplerConformanceError::InvalidSequenceIndex)
        );
        assert_eq!(verifier.sample_count(), 1);
    }

    #[test]
    fn rng_component_and_normal_streams_must_be_domain_separated() {
        let mut invalid = contract();
        invalid.normal_stream_digest = invalid.component_stream_digest;
        assert_eq!(
            GaussianProposalExecutionVerifier::new(proposal(), invalid).unwrap_err(),
            GaussianSamplerConformanceError::RandomStreamsNotDomainSeparated
        );
    }
}
