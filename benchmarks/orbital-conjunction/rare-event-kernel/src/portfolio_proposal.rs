// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Defensive Gaussian proposal mixtures spanning a frozen portfolio/event-mode universe.
//!
//! The target Gaussian mixture is retained as a positive defensive component.
//! Every declared portfolio/scenario/event-mode key must be represented by at
//! least one proposal component. Importance weights use the density of the
//! complete flattened mixture, never only the selected component.
//!
//! This module evaluates supplied points; it does not generate samples or
//! prove that an external sampler actually drew from the declared mixture.
//! Mode coverage is relative to the frozen caller-declared mode universe and
//! is not proof that the physical event universe is complete.

use std::collections::BTreeSet;

use sha2::{Digest, Sha256};

use crate::{Gaussian, GaussianComponent, GaussianMixture, KernelError, log_importance_weight};

/// Maximum number of declared portfolio/scenario/event-mode keys in one mixture.
pub const MAX_PORTFOLIO_PROPOSAL_MODES: usize = 4_096;

/// Maximum number of proposal components in one defensive mixture.
pub const MAX_PORTFOLIO_PROPOSAL_COMPONENTS: usize = 16_384;

/// Content key for one portfolio × epistemic scenario × event-mode region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PortfolioEventModeKey {
    /// Stable, nonzero manoeuvre-portfolio identity.
    pub portfolio_id: [u8; 32],
    /// Stable, nonzero physical-world / loss-model scenario identity.
    pub scenario_id: [u8; 32],
    /// Stable, nonzero rare-event or trajectory-mode identity.
    pub event_mode_id: [u8; 32],
}

/// One weighted proposal Gaussian assigned to a declared portfolio/event mode.
#[derive(Debug, Clone, PartialEq)]
pub struct DefensiveProposalComponent {
    /// Stable, nonzero identity of this proposal component.
    pub component_id: [u8; 32],
    /// Portfolio/scenario/event-mode key this component is intended to cover.
    pub mode_key: PortfolioEventModeKey,
    /// Positive component allocation within the non-defensive proposal mixture.
    pub allocation_weight: f64,
    /// Gaussian proposal distribution for this component.
    pub distribution: Gaussian,
}

/// Errors constructing or evaluating the frozen defensive proposal mixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortfolioProposalError {
    /// Defensive fraction epsilon must be finite and strictly between zero and one.
    InvalidDefensiveFraction,
    /// Stable target probability-measure identity is zero.
    InvalidTargetMeasureIdentity,
    /// Density-space identity (coordinates, frame, units and epoch semantics) is zero.
    InvalidDensitySpaceIdentity,
    /// The frozen target mixture has no components.
    EmptyTargetMixture,
    /// Target or proposal dimensions differ.
    DimensionMismatch,
    /// The declared mode universe is empty.
    EmptyModeUniverse,
    /// The declared mode universe exceeds its bound.
    TooManyModes,
    /// The proposal component list is empty.
    EmptyProposalComponents,
    /// The proposal component list exceeds its bound.
    TooManyProposalComponents,
    /// A portfolio, scenario or event-mode identity is zero.
    InvalidModeIdentity,
    /// A mode identity occurs more than once in the frozen universe.
    DuplicateModeIdentity,
    /// A proposal component identity is zero or duplicated.
    InvalidOrDuplicateComponentIdentity,
    /// A proposal component references a mode outside the frozen universe.
    UnexpectedProposalMode,
    /// At least one declared mode has no proposal component.
    IncompleteModeCoverage,
    /// A proposal component allocation is not finite and strictly positive.
    InvalidComponentWeight,
    /// Proposal component allocations do not sum to one within the frozen tolerance.
    ComponentWeightsNotNormalized,
    /// A normalized contrast observation is not finite and in [0, 1].
    InvalidNormalizedContrast,
    /// The supplied likelihood ratio exceeds the defensive 1/epsilon bound.
    LikelihoodRatioExceedsDefensiveBound,
    /// The computed defensive contribution exceeds one; no clipping was applied.
    DefensiveContributionExceedsOne,
    /// A density, digest or transformed contribution failed numerically.
    NumericalFailure,
    /// The underlying Gaussian density kernel rejected an operation.
    GaussianKernelFailure,
}

impl core::fmt::Display for PortfolioProposalError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let message = match self {
            Self::InvalidDefensiveFraction => {
                "defensive mixture fraction must lie strictly in (0, 1)"
            }
            Self::InvalidTargetMeasureIdentity => "target measure identity is zero",
            Self::InvalidDensitySpaceIdentity => "density-space identity is zero",
            Self::EmptyTargetMixture => "target Gaussian mixture is empty",
            Self::DimensionMismatch => "target and proposal Gaussian dimensions differ",
            Self::EmptyModeUniverse => "portfolio event-mode universe is empty",
            Self::TooManyModes => "portfolio event-mode universe exceeds its configured bound",
            Self::EmptyProposalComponents => "defensive proposal component list is empty",
            Self::TooManyProposalComponents => {
                "defensive proposal component list exceeds its configured bound"
            }
            Self::InvalidModeIdentity => "portfolio, scenario or event-mode identity is zero",
            Self::DuplicateModeIdentity => "portfolio event-mode identity is duplicated",
            Self::InvalidOrDuplicateComponentIdentity => {
                "proposal component identity is zero or duplicated"
            }
            Self::UnexpectedProposalMode => {
                "proposal component references an undeclared event mode"
            }
            Self::IncompleteModeCoverage => {
                "at least one declared portfolio event mode has no proposal component"
            }
            Self::InvalidComponentWeight => {
                "proposal allocation weight must be finite and strictly positive"
            }
            Self::ComponentWeightsNotNormalized => "proposal allocation weights do not sum to one",
            Self::InvalidNormalizedContrast => "normalized portfolio contrast must lie in [0, 1]",
            Self::LikelihoodRatioExceedsDefensiveBound => {
                "importance ratio exceeds the defensive 1/epsilon bound"
            }
            Self::DefensiveContributionExceedsOne => {
                "defensive contribution exceeds one and was not clipped"
            }
            Self::NumericalFailure => {
                "defensive proposal density or contribution failed numerically"
            }
            Self::GaussianKernelFailure => {
                "underlying Gaussian density kernel rejected an operation"
            }
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PortfolioProposalError {}

impl From<KernelError> for PortfolioProposalError {
    fn from(_: KernelError) -> Self {
        Self::GaussianKernelFailure
    }
}

/// One evaluated importance draw under the full defensive mixture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DefensiveProposalEvaluation {
    /// Mode key whose coverage is being evaluated.
    pub mode_key: PortfolioEventModeKey,
    /// Log target density p(u).
    pub log_target_density: f64,
    /// Log density of the complete defensive mixture q(u).
    pub log_full_proposal_density: f64,
    /// Log importance ratio log p(u) - log q(u).
    pub log_importance_weight: f64,
    /// Log bounded contribution log(epsilon * g(u) * p(u)/q(u)).
    pub log_defensive_contribution: f64,
    /// Linear defensive contribution, guaranteed to be in [0, 1] or rejected.
    pub defensive_contribution: f64,
}

/// Frozen defensive Gaussian proposal mixture for a declared mode universe.
///
/// The proposal is q(u) = epsilon * p(u) + (1-epsilon) * sum_j alpha_j q_j(u).
/// The target and all keyed proposal components are flattened into one
/// canonical Gaussian mixture. This type does not contain an RNG or a sampler.
#[derive(Debug, Clone, PartialEq)]
pub struct PortfolioDefensiveGaussianMixture {
    target_measure_id: [u8; 32],
    density_space_id: [u8; 32],
    target: GaussianMixture,
    full_proposal: GaussianMixture,
    defensive_fraction: f64,
    expected_modes: Vec<PortfolioEventModeKey>,
    component_ids: Vec<[u8; 32]>,
    configuration_digest: [u8; 32],
}

impl PortfolioDefensiveGaussianMixture {
    /// Construct a defensive proposal mixture for a complete frozen mode universe.
    ///
    /// Target mixture weights and proposal allocations must each be normalized.
    /// Proposal allocations are normalized only for accepted residual floating
    /// point roundoff within 1e-12. Every declared mode must be covered.
    pub fn new(
        target_measure_id: [u8; 32],
        density_space_id: [u8; 32],
        target: GaussianMixture,
        defensive_fraction: f64,
        mut expected_modes: Vec<PortfolioEventModeKey>,
        mut proposal_components: Vec<DefensiveProposalComponent>,
    ) -> Result<Self, PortfolioProposalError> {
        if target_measure_id.iter().all(|byte| *byte == 0) {
            return Err(PortfolioProposalError::InvalidTargetMeasureIdentity);
        }
        if density_space_id.iter().all(|byte| *byte == 0) {
            return Err(PortfolioProposalError::InvalidDensitySpaceIdentity);
        }
        if !defensive_fraction.is_finite() || defensive_fraction <= 0.0 || defensive_fraction >= 1.0
        {
            return Err(PortfolioProposalError::InvalidDefensiveFraction);
        }
        if target.component_count() == 0 {
            return Err(PortfolioProposalError::EmptyTargetMixture);
        }
        if expected_modes.is_empty() {
            return Err(PortfolioProposalError::EmptyModeUniverse);
        }
        if expected_modes.len() > MAX_PORTFOLIO_PROPOSAL_MODES {
            return Err(PortfolioProposalError::TooManyModes);
        }
        if proposal_components.is_empty() {
            return Err(PortfolioProposalError::EmptyProposalComponents);
        }
        if proposal_components.len() > MAX_PORTFOLIO_PROPOSAL_COMPONENTS {
            return Err(PortfolioProposalError::TooManyProposalComponents);
        }

        expected_modes.sort_unstable();
        for (index, mode) in expected_modes.iter().enumerate() {
            if mode.portfolio_id.iter().all(|byte| *byte == 0)
                || mode.scenario_id.iter().all(|byte| *byte == 0)
                || mode.event_mode_id.iter().all(|byte| *byte == 0)
            {
                return Err(PortfolioProposalError::InvalidModeIdentity);
            }
            if index > 0 && expected_modes[index - 1] == *mode {
                return Err(PortfolioProposalError::DuplicateModeIdentity);
            }
        }

        let dimension = target.dimension();
        for component in &proposal_components {
            if component.distribution.dimension() != dimension {
                return Err(PortfolioProposalError::DimensionMismatch);
            }
            if !component.allocation_weight.is_finite() || component.allocation_weight <= 0.0 {
                return Err(PortfolioProposalError::InvalidComponentWeight);
            }
            if component.component_id.iter().all(|byte| *byte == 0) {
                return Err(PortfolioProposalError::InvalidOrDuplicateComponentIdentity);
            }
            if expected_modes.binary_search(&component.mode_key).is_err() {
                return Err(PortfolioProposalError::UnexpectedProposalMode);
            }
        }

        proposal_components.sort_unstable_by_key(|component| component.component_id);
        if proposal_components
            .windows(2)
            .any(|pair| pair[0].component_id == pair[1].component_id)
        {
            return Err(PortfolioProposalError::InvalidOrDuplicateComponentIdentity);
        }

        let allocation_sum: f64 = proposal_components
            .iter()
            .map(|component| component.allocation_weight)
            .sum();
        if !allocation_sum.is_finite() || (allocation_sum - 1.0).abs() > 1.0e-12 {
            return Err(PortfolioProposalError::ComponentWeightsNotNormalized);
        }
        for component in &mut proposal_components {
            component.allocation_weight /= allocation_sum;
        }

        let covered_modes = proposal_components
            .iter()
            .map(|component| component.mode_key)
            .collect::<BTreeSet<_>>();
        if expected_modes
            .iter()
            .any(|mode| !covered_modes.contains(mode))
        {
            return Err(PortfolioProposalError::IncompleteModeCoverage);
        }

        // Canonicalize target components by distribution content and then by
        // weight, so equivalent target mixtures have stable component order.
        let mut target_components = target
            .components()
            .iter()
            .map(|component| {
                Ok((
                    gaussian_digest(component.distribution())?,
                    canonical_f64_bits(component.weight()),
                    component.clone(),
                ))
            })
            .collect::<Result<Vec<_>, PortfolioProposalError>>()?;
        target_components.sort_unstable_by_key(|(digest, weight_bits, _)| (*digest, *weight_bits));
        let canonical_target_components = target_components
            .into_iter()
            .map(|(_, _, component)| {
                GaussianComponent::new(component.weight(), component.distribution().clone())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let canonical_target = GaussianMixture::new(canonical_target_components)?;

        let mut flattened =
            Vec::with_capacity(canonical_target.component_count() + proposal_components.len());
        for component in canonical_target.components() {
            let weight = defensive_fraction_times(defensive_fraction, component.weight())?;
            flattened.push(GaussianComponent::new(
                weight,
                component.distribution().clone(),
            )?);
        }
        for component in &proposal_components {
            let weight =
                defensive_fraction_times(1.0 - defensive_fraction, component.allocation_weight)?;
            flattened.push(GaussianComponent::new(
                weight,
                component.distribution.clone(),
            )?);
        }
        let full_proposal = GaussianMixture::new(flattened)?;
        // GaussianMixture normalizes the flattened weights to remove residual
        // floating-point summation error. Use the actual defensive mass after
        // that normalization in both the likelihood bound and the bounded
        // contribution, so the algebra and represented q use the same epsilon.
        let effective_defensive_fraction: f64 = full_proposal
            .components()
            .iter()
            .take(canonical_target.component_count())
            .map(GaussianComponent::weight)
            .sum();
        if !effective_defensive_fraction.is_finite()
            || effective_defensive_fraction <= 0.0
            || effective_defensive_fraction >= 1.0
        {
            return Err(PortfolioProposalError::NumericalFailure);
        }

        let configuration_digest = configuration_digest(
            target_measure_id,
            density_space_id,
            defensive_fraction,
            effective_defensive_fraction,
            &canonical_target,
            &expected_modes,
            &proposal_components,
        )?;
        let component_ids = proposal_components
            .iter()
            .map(|component| component.component_id)
            .collect();

        Ok(Self {
            target_measure_id,
            density_space_id,
            target: canonical_target,
            full_proposal,
            defensive_fraction: effective_defensive_fraction,
            expected_modes,
            component_ids,
            configuration_digest,
        })
    }

    /// Return the source-declared identity of the target probability measure.
    pub fn target_measure_id(&self) -> [u8; 32] {
        self.target_measure_id
    }

    /// Return the density-space identity covering coordinates, frame, units and epoch semantics.
    pub fn density_space_id(&self) -> [u8; 32] {
        self.density_space_id
    }

    /// Return the defensive target-mixture fraction epsilon.
    pub fn defensive_fraction(&self) -> f64 {
        self.defensive_fraction
    }

    /// Return the complete canonical expected mode universe.
    pub fn expected_modes(&self) -> &[PortfolioEventModeKey] {
        &self.expected_modes
    }

    /// Return the stable proposal component IDs in canonical order.
    pub fn component_ids(&self) -> &[[u8; 32]] {
        &self.component_ids
    }

    /// Return the SHA-256 digest of the frozen target, modes and proposal mixture.
    ///
    /// This binds configuration content only. It does not authenticate an issuer
    /// or establish that the mode universe is physically complete.
    pub fn configuration_digest(&self) -> [u8; 32] {
        self.configuration_digest
    }

    /// Return the full defensive proposal log density at one point.
    pub fn log_full_proposal_density(&self, point: &[f64]) -> Result<f64, PortfolioProposalError> {
        Ok(self.full_proposal.log_pdf(point)?)
    }

    /// Evaluate one normalized portfolio-contrast contribution under the full
    /// defensive mixture.
    ///
    /// The normalized contrast must be in [0,1]. The returned contribution is
    /// epsilon * contrast * p(point) / q(point) and is never clipped. Any value
    /// above one, or any importance ratio above the defensive bound, is rejected.
    ///
    /// This method does not verify that the point was sampled from q. Sampling
    /// provenance, predictability and unique draw identity must be established
    /// by a separate sampler receipt and independent execution verifier.
    pub fn evaluate_draw(
        &self,
        mode_key: PortfolioEventModeKey,
        point: &[f64],
        normalized_contrast: f64,
    ) -> Result<DefensiveProposalEvaluation, PortfolioProposalError> {
        if self.expected_modes.binary_search(&mode_key).is_err() {
            return Err(PortfolioProposalError::UnexpectedProposalMode);
        }
        if !normalized_contrast.is_finite() || !(0.0..=1.0).contains(&normalized_contrast) {
            return Err(PortfolioProposalError::InvalidNormalizedContrast);
        }

        let log_target_density = self.target.log_pdf(point)?;
        let log_full_proposal_density = self.full_proposal.log_pdf(point)?;
        let log_importance_weight =
            log_importance_weight(log_target_density, log_full_proposal_density)?;

        let log_bound = -self.defensive_fraction.ln();
        let bound_tolerance = 64.0 * f64::EPSILON * (1.0 + log_bound.abs());
        if log_importance_weight.is_finite() && log_importance_weight > log_bound + bound_tolerance
        {
            return Err(PortfolioProposalError::LikelihoodRatioExceedsDefensiveBound);
        }

        let log_defensive_contribution =
            if normalized_contrast == 0.0 || log_importance_weight == f64::NEG_INFINITY {
                f64::NEG_INFINITY
            } else {
                self.defensive_fraction.ln() + normalized_contrast.ln() + log_importance_weight
            };
        if log_defensive_contribution.is_nan()
            || log_defensive_contribution == f64::INFINITY
            || log_defensive_contribution > 0.0
        {
            return Err(PortfolioProposalError::DefensiveContributionExceedsOne);
        }
        let defensive_contribution = if log_defensive_contribution == f64::NEG_INFINITY {
            0.0
        } else {
            log_defensive_contribution.exp()
        };
        if !defensive_contribution.is_finite() || !(0.0..=1.0).contains(&defensive_contribution) {
            return Err(PortfolioProposalError::DefensiveContributionExceedsOne);
        }

        Ok(DefensiveProposalEvaluation {
            mode_key,
            log_target_density,
            log_full_proposal_density,
            log_importance_weight,
            log_defensive_contribution,
            defensive_contribution,
        })
    }
}

fn defensive_fraction_times(left: f64, right: f64) -> Result<f64, PortfolioProposalError> {
    let product = left * right;
    if !product.is_finite() || product <= 0.0 {
        return Err(PortfolioProposalError::InvalidComponentWeight);
    }
    Ok(product)
}

fn canonical_f64_bits(value: f64) -> u64 {
    if value == 0.0 {
        0.0_f64.to_bits()
    } else {
        value.to_bits()
    }
}

fn hash_f64(hasher: &mut Sha256, value: f64) {
    hasher.update(canonical_f64_bits(value).to_be_bytes());
}

fn hash_gaussian(hasher: &mut Sha256, gaussian: &Gaussian) {
    hasher.update((gaussian.dimension() as u64).to_be_bytes());
    for value in gaussian.mean() {
        hash_f64(hasher, *value);
    }
    for value in gaussian.lower_cholesky_values() {
        hash_f64(hasher, *value);
    }
}

fn gaussian_digest(gaussian: &Gaussian) -> Result<[u8; 32], PortfolioProposalError> {
    let mut hasher = Sha256::new();
    hasher.update(b"luminous.orbital-gaussian.v0.1\0");
    hash_gaussian(&mut hasher, gaussian);
    Ok(hasher.finalize().into())
}

fn configuration_digest(
    target_measure_id: [u8; 32],
    density_space_id: [u8; 32],
    declared_defensive_fraction: f64,
    effective_defensive_fraction: f64,
    target: &GaussianMixture,
    modes: &[PortfolioEventModeKey],
    proposal_components: &[DefensiveProposalComponent],
) -> Result<[u8; 32], PortfolioProposalError> {
    let mut hasher = Sha256::new();
    hasher.update(b"luminous.orbital-defensive-portfolio-mixture.v0.1\0");
    hasher.update(target_measure_id);
    hasher.update(density_space_id);
    hash_f64(&mut hasher, declared_defensive_fraction);
    hash_f64(&mut hasher, effective_defensive_fraction);
    hasher.update((target.component_count() as u64).to_be_bytes());
    for component in target.components() {
        hash_f64(&mut hasher, component.weight());
        hash_gaussian(&mut hasher, component.distribution());
    }

    hasher.update((modes.len() as u64).to_be_bytes());
    for mode in modes {
        hasher.update(mode.portfolio_id);
        hasher.update(mode.scenario_id);
        hasher.update(mode.event_mode_id);
    }

    hasher.update((proposal_components.len() as u64).to_be_bytes());
    for component in proposal_components {
        hasher.update(component.component_id);
        hasher.update(component.mode_key.portfolio_id);
        hasher.update(component.mode_key.scenario_id);
        hasher.update(component.mode_key.event_mode_id);
        hash_f64(&mut hasher, component.allocation_weight);
        hash_gaussian(&mut hasher, &component.distribution);
    }

    Ok(hasher.finalize().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TARGET_MEASURE_ID: [u8; 32] = [90; 32];
    const DENSITY_SPACE_ID: [u8; 32] = [91; 32];

    fn gaussian(mean: f64, variance: f64) -> Gaussian {
        Gaussian::new(vec![mean], vec![variance]).unwrap()
    }

    fn key(portfolio: u8, scenario: u8, event: u8) -> PortfolioEventModeKey {
        PortfolioEventModeKey {
            portfolio_id: [portfolio; 32],
            scenario_id: [scenario; 32],
            event_mode_id: [event; 32],
        }
    }

    fn proposal(
        component: u8,
        mode_key: PortfolioEventModeKey,
        allocation_weight: f64,
        mean: f64,
    ) -> DefensiveProposalComponent {
        DefensiveProposalComponent {
            component_id: [component; 32],
            mode_key,
            allocation_weight,
            distribution: gaussian(mean, 1.0),
        }
    }

    fn fixture() -> PortfolioDefensiveGaussianMixture {
        let target = GaussianMixture::new(vec![
            GaussianComponent::new(0.5, gaussian(-1.0, 1.0)).unwrap(),
            GaussianComponent::new(0.5, gaussian(1.0, 1.0)).unwrap(),
        ])
        .unwrap();
        PortfolioDefensiveGaussianMixture::new(
            TARGET_MEASURE_ID,
            DENSITY_SPACE_ID,
            target,
            0.2,
            vec![key(10, 1, 1), key(20, 1, 2)],
            vec![
                proposal(1, key(10, 1, 1), 0.5, -5.0),
                proposal(2, key(20, 1, 2), 0.5, 5.0),
            ],
        )
        .unwrap()
    }

    fn log_sum_exp(values: &[f64]) -> f64 {
        let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        maximum
            + values
                .iter()
                .map(|value| (value - maximum).exp())
                .sum::<f64>()
                .ln()
    }

    #[test]
    fn full_proposal_density_includes_target_and_every_mode_component() {
        let mixture = fixture();
        let point = [0.25];
        let target = GaussianMixture::new(vec![
            GaussianComponent::new(0.5, gaussian(-1.0, 1.0)).unwrap(),
            GaussianComponent::new(0.5, gaussian(1.0, 1.0)).unwrap(),
        ])
        .unwrap();
        let proposal_left = gaussian(-5.0, 1.0);
        let proposal_right = gaussian(5.0, 1.0);
        let expected = log_sum_exp(&[
            0.2_f64.ln() + target.log_pdf(&point).unwrap(),
            0.4_f64.ln() + proposal_left.log_pdf(&point).unwrap(),
            0.4_f64.ln() + proposal_right.log_pdf(&point).unwrap(),
        ]);
        let observed = mixture.log_full_proposal_density(&point).unwrap();
        assert!((mixture.defensive_fraction() - 0.2).abs() < 1.0e-15);
        assert!((observed - expected).abs() < 1.0e-13);
        assert_ne!(observed, proposal_left.log_pdf(&point).unwrap());
    }

    #[test]
    fn importance_ratio_and_defensive_contribution_respect_epsilon_bound() {
        let mixture = fixture();
        for point in [-40.0, -5.0, 0.0, 5.0, 40.0] {
            for mode in [key(10, 1, 1), key(20, 1, 2)] {
                let result = mixture.evaluate_draw(mode, &[point], 1.0).unwrap();
                assert!(
                    result.log_importance_weight <= -mixture.defensive_fraction().ln() + 1.0e-12
                );
                assert!((0.0..=1.0).contains(&result.defensive_contribution));
                assert!(result.log_defensive_contribution <= 0.0);
            }
        }
    }

    #[test]
    fn zero_contrast_has_zero_contribution_without_changing_density_evidence() {
        let mixture = fixture();
        let result = mixture.evaluate_draw(key(10, 1, 1), &[0.0], 0.0).unwrap();
        assert_eq!(result.defensive_contribution, 0.0);
        assert_eq!(result.log_defensive_contribution, f64::NEG_INFINITY);
        assert!(result.log_full_proposal_density.is_finite());
    }

    #[test]
    fn target_measure_and_density_space_identities_are_required() {
        let target = GaussianMixture::new(vec![
            GaussianComponent::new(1.0, gaussian(0.0, 1.0)).unwrap(),
        ])
        .unwrap();
        assert_eq!(
            PortfolioDefensiveGaussianMixture::new(
                [0; 32],
                DENSITY_SPACE_ID,
                target.clone(),
                0.1,
                vec![key(10, 1, 1)],
                vec![proposal(1, key(10, 1, 1), 1.0, 0.0)],
            )
            .unwrap_err(),
            PortfolioProposalError::InvalidTargetMeasureIdentity
        );
        assert_eq!(
            PortfolioDefensiveGaussianMixture::new(
                TARGET_MEASURE_ID,
                [0; 32],
                target,
                0.1,
                vec![key(10, 1, 1)],
                vec![proposal(1, key(10, 1, 1), 1.0, 0.0)],
            )
            .unwrap_err(),
            PortfolioProposalError::InvalidDensitySpaceIdentity
        );
    }

    #[test]
    fn omitted_declared_mode_fails_closed() {
        let target = GaussianMixture::new(vec![
            GaussianComponent::new(1.0, gaussian(0.0, 1.0)).unwrap(),
        ])
        .unwrap();
        assert_eq!(
            PortfolioDefensiveGaussianMixture::new(
                TARGET_MEASURE_ID,
                DENSITY_SPACE_ID,
                target,
                0.1,
                vec![key(10, 1, 1), key(20, 1, 2)],
                vec![proposal(1, key(10, 1, 1), 1.0, 0.0)],
            )
            .unwrap_err(),
            PortfolioProposalError::IncompleteModeCoverage
        );
    }

    #[test]
    fn proposal_component_for_undeclared_mode_is_rejected() {
        let target = GaussianMixture::new(vec![
            GaussianComponent::new(1.0, gaussian(0.0, 1.0)).unwrap(),
        ])
        .unwrap();
        assert_eq!(
            PortfolioDefensiveGaussianMixture::new(
                TARGET_MEASURE_ID,
                DENSITY_SPACE_ID,
                target,
                0.1,
                vec![key(10, 1, 1)],
                vec![proposal(1, key(20, 1, 2), 1.0, 0.0)],
            )
            .unwrap_err(),
            PortfolioProposalError::UnexpectedProposalMode
        );
    }

    #[test]
    fn duplicate_zero_mode_and_component_identities_are_rejected() {
        let target = GaussianMixture::new(vec![
            GaussianComponent::new(1.0, gaussian(0.0, 1.0)).unwrap(),
        ])
        .unwrap();
        assert_eq!(
            PortfolioDefensiveGaussianMixture::new(
                TARGET_MEASURE_ID,
                DENSITY_SPACE_ID,
                target.clone(),
                0.1,
                vec![key(10, 1, 1), key(10, 1, 1)],
                vec![proposal(1, key(10, 1, 1), 1.0, 0.0)],
            )
            .unwrap_err(),
            PortfolioProposalError::DuplicateModeIdentity
        );
        assert_eq!(
            PortfolioDefensiveGaussianMixture::new(
                TARGET_MEASURE_ID,
                DENSITY_SPACE_ID,
                target.clone(),
                0.1,
                vec![key(10, 1, 1)],
                vec![
                    proposal(0, key(10, 1, 1), 0.5, 0.0),
                    proposal(1, key(10, 1, 1), 0.5, 1.0),
                ],
            )
            .unwrap_err(),
            PortfolioProposalError::InvalidOrDuplicateComponentIdentity
        );
        assert_eq!(
            PortfolioDefensiveGaussianMixture::new(
                TARGET_MEASURE_ID,
                DENSITY_SPACE_ID,
                target,
                0.1,
                vec![key(10, 1, 1)],
                vec![
                    proposal(1, key(10, 1, 1), 0.5, 0.0),
                    proposal(1, key(10, 1, 1), 0.5, 1.0),
                ],
            )
            .unwrap_err(),
            PortfolioProposalError::InvalidOrDuplicateComponentIdentity
        );
    }

    #[test]
    fn component_weights_and_epsilon_must_be_valid() {
        let target = GaussianMixture::new(vec![
            GaussianComponent::new(1.0, gaussian(0.0, 1.0)).unwrap(),
        ])
        .unwrap();
        for epsilon in [0.0, 1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(
                PortfolioDefensiveGaussianMixture::new(
                    TARGET_MEASURE_ID,
                    DENSITY_SPACE_ID,
                    target.clone(),
                    epsilon,
                    vec![key(10, 1, 1)],
                    vec![proposal(1, key(10, 1, 1), 1.0, 0.0)],
                )
                .unwrap_err(),
                PortfolioProposalError::InvalidDefensiveFraction
            );
        }
        assert_eq!(
            PortfolioDefensiveGaussianMixture::new(
                TARGET_MEASURE_ID,
                DENSITY_SPACE_ID,
                target.clone(),
                0.1,
                vec![key(10, 1, 1)],
                vec![
                    proposal(1, key(10, 1, 1), 0.2, 0.0),
                    proposal(2, key(10, 1, 1), 0.2, 1.0),
                ],
            )
            .unwrap_err(),
            PortfolioProposalError::ComponentWeightsNotNormalized
        );
        assert_eq!(
            PortfolioDefensiveGaussianMixture::new(
                TARGET_MEASURE_ID,
                DENSITY_SPACE_ID,
                target,
                0.1,
                vec![key(10, 1, 1)],
                vec![proposal(1, key(10, 1, 1), 0.0, 0.0)],
            )
            .unwrap_err(),
            PortfolioProposalError::InvalidComponentWeight
        );
    }

    #[test]
    fn normalized_contrast_and_mode_are_validated_at_evaluation() {
        let mixture = fixture();
        assert_eq!(
            mixture.evaluate_draw(key(10, 1, 1), &[0.0], -0.1),
            Err(PortfolioProposalError::InvalidNormalizedContrast)
        );
        assert_eq!(
            mixture.evaluate_draw(key(10, 1, 1), &[0.0], 1.1),
            Err(PortfolioProposalError::InvalidNormalizedContrast)
        );
        assert_eq!(
            mixture.evaluate_draw(key(30, 1, 1), &[0.0], 0.5),
            Err(PortfolioProposalError::UnexpectedProposalMode)
        );
    }

    #[test]
    fn configuration_digest_and_density_are_order_invariant() {
        let target_a = GaussianMixture::new(vec![
            GaussianComponent::new(0.5, gaussian(-1.0, 1.0)).unwrap(),
            GaussianComponent::new(0.5, gaussian(1.0, 1.0)).unwrap(),
        ])
        .unwrap();
        let target_b = GaussianMixture::new(vec![
            GaussianComponent::new(0.5, gaussian(1.0, 1.0)).unwrap(),
            GaussianComponent::new(0.5, gaussian(-1.0, 1.0)).unwrap(),
        ])
        .unwrap();
        let a = PortfolioDefensiveGaussianMixture::new(
            TARGET_MEASURE_ID,
            DENSITY_SPACE_ID,
            target_a,
            0.2,
            vec![key(10, 1, 1), key(20, 1, 2)],
            vec![
                proposal(1, key(10, 1, 1), 0.5, -5.0),
                proposal(2, key(20, 1, 2), 0.5, 5.0),
            ],
        )
        .unwrap();
        let b = PortfolioDefensiveGaussianMixture::new(
            TARGET_MEASURE_ID,
            DENSITY_SPACE_ID,
            target_b,
            0.2,
            vec![key(20, 1, 2), key(10, 1, 1)],
            vec![
                proposal(2, key(20, 1, 2), 0.5, 5.0),
                proposal(1, key(10, 1, 1), 0.5, -5.0),
            ],
        )
        .unwrap();

        assert_eq!(a.configuration_digest(), b.configuration_digest());
        assert_eq!(
            a.log_full_proposal_density(&[0.75]).unwrap(),
            b.log_full_proposal_density(&[0.75]).unwrap()
        );
    }

    #[test]
    fn configuration_digest_changes_when_epsilon_or_a_component_changes() {
        let target = GaussianMixture::new(vec![
            GaussianComponent::new(1.0, gaussian(0.0, 1.0)).unwrap(),
        ])
        .unwrap();
        let a = PortfolioDefensiveGaussianMixture::new(
            TARGET_MEASURE_ID,
            DENSITY_SPACE_ID,
            target.clone(),
            0.2,
            vec![key(10, 1, 1)],
            vec![proposal(1, key(10, 1, 1), 1.0, 3.0)],
        )
        .unwrap();
        let b = PortfolioDefensiveGaussianMixture::new(
            TARGET_MEASURE_ID,
            DENSITY_SPACE_ID,
            target.clone(),
            0.3,
            vec![key(10, 1, 1)],
            vec![proposal(1, key(10, 1, 1), 1.0, 3.0)],
        )
        .unwrap();
        let c = PortfolioDefensiveGaussianMixture::new(
            TARGET_MEASURE_ID,
            DENSITY_SPACE_ID,
            target.clone(),
            0.2,
            vec![key(10, 1, 1)],
            vec![proposal(1, key(10, 1, 1), 1.0, 4.0)],
        )
        .unwrap();
        let d = PortfolioDefensiveGaussianMixture::new(
            [92; 32],
            DENSITY_SPACE_ID,
            target.clone(),
            0.2,
            vec![key(10, 1, 1)],
            vec![proposal(1, key(10, 1, 1), 1.0, 3.0)],
        )
        .unwrap();
        let e = PortfolioDefensiveGaussianMixture::new(
            TARGET_MEASURE_ID,
            [93; 32],
            target,
            0.2,
            vec![key(10, 1, 1)],
            vec![proposal(1, key(10, 1, 1), 1.0, 3.0)],
        )
        .unwrap();
        assert_ne!(a.configuration_digest(), b.configuration_digest());
        assert_ne!(a.configuration_digest(), c.configuration_digest());
        assert_ne!(a.configuration_digest(), d.configuration_digest());
        assert_ne!(a.configuration_digest(), e.configuration_digest());
    }
}
