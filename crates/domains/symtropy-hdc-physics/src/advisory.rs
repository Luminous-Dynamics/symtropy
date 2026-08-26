// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Typed, deterministic admission boundary between semantic/learned physics
//! advisors and the authoritative numerical simulation.
//!
//! The contract is intentionally asymmetric: advisors may request more
//! fidelity or an exact fallback freely, but any request that could reduce
//! numerical fidelity must carry a calibrated error estimate, sufficient
//! confidence, explicit low novelty, and provenance where required. This keeps
//! HDC/CfC-style intelligence useful without allowing it to mutate authoritative
//! physics state directly.
//!
//! Firewall admission is necessary but not sufficient for intervention. A
//! downstream authoritative policy must still require healthy numerical state,
//! complete conservation/reconciliation evidence, stable reservoir/world
//! lifecycle, and whatever solver-specific validity contract applies.

use serde::{Deserialize, Serialize};

use crate::ExactStateDigest;

/// Provenance class for a physics advisory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdvisorySource {
    /// Similarity/retrieval result from the HDC semantic shadow.
    HdcRetrieval,
    /// Continuous-time temporal model such as CfC/LTC.
    ContinuousTimeModel,
    /// Combined HDC semantic state plus continuous-time prediction.
    HdcContinuousTimeHybrid,
    /// Deterministic hand-authored policy or invariant monitor.
    RuleBased,
    /// External caller. Still subject to the same admission contract.
    External,
}

impl AdvisorySource {
    #[inline]
    pub fn is_learned_or_semantic(self) -> bool {
        matches!(
            self,
            Self::HdcRetrieval | Self::ContinuousTimeModel | Self::HdcContinuousTimeHybrid
        )
    }
}

/// Coarse fidelity levels used by the advisory contract.
///
/// These are policy labels, not solver implementations. A future solver
/// federation layer may map them to rigid/FEM/MPM/fluid/reduced-order backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FidelityTier {
    Reduced,
    Coarse,
    Standard,
    High,
    Exact,
}

impl FidelityTier {
    pub const fn rank(self) -> u8 {
        match self {
            Self::Reduced => 0,
            Self::Coarse => 1,
            Self::Standard => 2,
            Self::High => 3,
            Self::Exact => 4,
        }
    }
}

/// Advisory action. None of these directly mutate `PhysicsWorld`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdvisoryAction {
    /// Request at least this fidelity tier.
    PromoteFidelity { minimum: FidelityTier },
    /// Request no more than this fidelity tier. This can reduce accuracy and is
    /// therefore subject to the strictest admission checks.
    DemoteFidelity { maximum: FidelityTier },
    /// Request at least this many solver substeps.
    IncreaseSubsteps { minimum: u16 },
    /// Request at most this many solver substeps. Accuracy-reducing.
    DecreaseSubsteps { maximum: u16 },
    /// Explicitly ask the engine to leave any approximation and use its exact
    /// or highest-certified path.
    RequestExactFallback,
    /// Mark the current state as outside the advisor's familiar/calibrated
    /// regime. The firewall treats this as a request for exact handling.
    FlagAnomaly,
}

impl AdvisoryAction {
    #[inline]
    pub fn can_reduce_fidelity(&self) -> bool {
        matches!(
            self,
            Self::DemoteFidelity { .. } | Self::DecreaseSubsteps { .. }
        )
    }
}

/// Immutable proposal emitted by a semantic/learned supervisory layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhysicsAdvisory {
    pub proposal_id: u64,
    pub tick: u64,
    pub source: AdvisorySource,
    pub action: AdvisoryAction,
    /// Calibrated confidence in [0, 1].
    pub confidence: f32,
    /// Predicted relative physical error if this action is accepted.
    ///
    /// Required for any action that can reduce fidelity.
    pub predicted_relative_error: Option<f64>,
    /// Semantic/operational novelty in [0, 1], where 1 means maximally
    /// unfamiliar. Required for any action that can reduce fidelity: unknown
    /// novelty is not favorable evidence.
    pub novelty_score: Option<f32>,
    /// Exact source-state digests supporting the proposal. Learned/semantic
    /// proposals must retain at least one exact provenance link.
    pub evidence_digests: Vec<ExactStateDigest>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvisoryValidationError {
    InvalidConfidence,
    InvalidPredictedError,
    InvalidNovelty,
    MissingSemanticEvidence,
    InvalidSubstepRequest,
}

impl PhysicsAdvisory {
    pub fn validate(&self) -> Result<(), AdvisoryValidationError> {
        if !self.confidence.is_finite() || !(0.0..=1.0).contains(&self.confidence) {
            return Err(AdvisoryValidationError::InvalidConfidence);
        }
        if let Some(error) = self.predicted_relative_error {
            if !error.is_finite() || error < 0.0 {
                return Err(AdvisoryValidationError::InvalidPredictedError);
            }
        }
        if let Some(novelty) = self.novelty_score {
            if !novelty.is_finite() || !(0.0..=1.0).contains(&novelty) {
                return Err(AdvisoryValidationError::InvalidNovelty);
            }
        }
        if self.source.is_learned_or_semantic() && self.evidence_digests.is_empty() {
            return Err(AdvisoryValidationError::MissingSemanticEvidence);
        }
        match &self.action {
            AdvisoryAction::IncreaseSubsteps { minimum } if *minimum == 0 => {
                return Err(AdvisoryValidationError::InvalidSubstepRequest);
            }
            AdvisoryAction::DecreaseSubsteps { maximum } if *maximum == 0 => {
                return Err(AdvisoryValidationError::InvalidSubstepRequest);
            }
            _ => {}
        }
        Ok(())
    }
}

/// Deterministic admission thresholds for semantic/learned advisories.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EpistemicFirewallPolicy {
    /// Maximum predicted relative error permitted for fidelity-reducing actions.
    pub max_relative_error_for_reduction: f64,
    /// Minimum calibrated confidence required to reduce fidelity.
    pub min_confidence_for_reduction: f32,
    /// Novelty above this value forbids fidelity reduction.
    pub max_novelty_for_reduction: f32,
    /// Novelty at or above this value escalates directly to exact handling.
    pub exact_fallback_novelty: f32,
    /// Hard policy limit for requested substep values.
    pub max_substeps: u16,
}

impl Default for EpistemicFirewallPolicy {
    fn default() -> Self {
        Self {
            max_relative_error_for_reduction: 0.01,
            min_confidence_for_reduction: 0.95,
            max_novelty_for_reduction: 0.20,
            exact_fallback_novelty: 0.80,
            max_substeps: 256,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpistemicFirewallPolicyValidationError {
    InvalidMaxRelativeError,
    InvalidMinConfidence,
    InvalidMaxNoveltyForReduction,
    InvalidExactFallbackNovelty,
    InvalidNoveltyThresholdOrdering,
    InvalidMaxSubsteps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvisoryRejection {
    InvalidPolicy(EpistemicFirewallPolicyValidationError),
    InvalidProposal(AdvisoryValidationError),
    MissingErrorBound,
    MissingNoveltyEstimate,
    ErrorBoundTooLarge,
    ConfidenceTooLow,
    NoveltyTooHigh,
    RequestedSubstepsTooLarge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactEscalationReason {
    AdvisorRequested,
    AnomalyFlagged,
    HighNovelty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvisoryDisposition {
    /// Proposal is admissible. A downstream authoritative policy still decides
    /// whether and how to enact it.
    Accept,
    /// Proposal is not admissible.
    Reject(AdvisoryRejection),
    /// Approximation should be abandoned in favor of exact/highest-certified
    /// physics for this region or event.
    EscalateExact(ExactEscalationReason),
}

impl EpistemicFirewallPolicy {
    /// Validate the policy itself. Invalid policy must fail closed rather than
    /// relying on comparisons involving NaN or incoherent thresholds.
    pub fn validate(&self) -> Result<(), EpistemicFirewallPolicyValidationError> {
        if !self.max_relative_error_for_reduction.is_finite()
            || self.max_relative_error_for_reduction < 0.0
        {
            return Err(EpistemicFirewallPolicyValidationError::InvalidMaxRelativeError);
        }
        if !self.min_confidence_for_reduction.is_finite()
            || !(0.0..=1.0).contains(&self.min_confidence_for_reduction)
        {
            return Err(EpistemicFirewallPolicyValidationError::InvalidMinConfidence);
        }
        if !self.max_novelty_for_reduction.is_finite()
            || !(0.0..=1.0).contains(&self.max_novelty_for_reduction)
        {
            return Err(
                EpistemicFirewallPolicyValidationError::InvalidMaxNoveltyForReduction,
            );
        }
        if !self.exact_fallback_novelty.is_finite()
            || !(0.0..=1.0).contains(&self.exact_fallback_novelty)
        {
            return Err(EpistemicFirewallPolicyValidationError::InvalidExactFallbackNovelty);
        }
        if self.max_novelty_for_reduction > self.exact_fallback_novelty {
            return Err(
                EpistemicFirewallPolicyValidationError::InvalidNoveltyThresholdOrdering,
            );
        }
        if self.max_substeps == 0 {
            return Err(EpistemicFirewallPolicyValidationError::InvalidMaxSubsteps);
        }
        Ok(())
    }

    /// Evaluate a proposal without mutating simulation state.
    pub fn evaluate(&self, advisory: &PhysicsAdvisory) -> AdvisoryDisposition {
        if let Err(error) = self.validate() {
            return AdvisoryDisposition::Reject(AdvisoryRejection::InvalidPolicy(error));
        }
        if let Err(error) = advisory.validate() {
            return AdvisoryDisposition::Reject(AdvisoryRejection::InvalidProposal(error));
        }

        match &advisory.action {
            AdvisoryAction::RequestExactFallback => {
                return AdvisoryDisposition::EscalateExact(
                    ExactEscalationReason::AdvisorRequested,
                );
            }
            AdvisoryAction::FlagAnomaly => {
                return AdvisoryDisposition::EscalateExact(ExactEscalationReason::AnomalyFlagged);
            }
            AdvisoryAction::IncreaseSubsteps { minimum } if *minimum > self.max_substeps => {
                return AdvisoryDisposition::Reject(
                    AdvisoryRejection::RequestedSubstepsTooLarge,
                );
            }
            AdvisoryAction::DecreaseSubsteps { maximum } if *maximum > self.max_substeps => {
                return AdvisoryDisposition::Reject(
                    AdvisoryRejection::RequestedSubstepsTooLarge,
                );
            }
            _ => {}
        }

        if let Some(novelty) = advisory.novelty_score {
            if novelty >= self.exact_fallback_novelty {
                return AdvisoryDisposition::EscalateExact(ExactEscalationReason::HighNovelty);
            }
        }

        if !advisory.action.can_reduce_fidelity() {
            return AdvisoryDisposition::Accept;
        }

        let Some(predicted_error) = advisory.predicted_relative_error else {
            return AdvisoryDisposition::Reject(AdvisoryRejection::MissingErrorBound);
        };
        if predicted_error > self.max_relative_error_for_reduction {
            return AdvisoryDisposition::Reject(AdvisoryRejection::ErrorBoundTooLarge);
        }
        if advisory.confidence < self.min_confidence_for_reduction {
            return AdvisoryDisposition::Reject(AdvisoryRejection::ConfidenceTooLow);
        }
        let Some(novelty) = advisory.novelty_score else {
            return AdvisoryDisposition::Reject(AdvisoryRejection::MissingNoveltyEstimate);
        };
        if novelty > self.max_novelty_for_reduction {
            return AdvisoryDisposition::Reject(AdvisoryRejection::NoveltyTooHigh);
        }

        AdvisoryDisposition::Accept
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(seed: u64) -> ExactStateDigest {
        ExactStateDigest {
            algorithm_version: 1,
            low: seed,
            high: !seed,
        }
    }

    fn advisory(action: AdvisoryAction) -> PhysicsAdvisory {
        PhysicsAdvisory {
            proposal_id: 7,
            tick: 42,
            source: AdvisorySource::HdcContinuousTimeHybrid,
            action,
            confidence: 0.99,
            predicted_relative_error: Some(0.001),
            novelty_score: Some(0.05),
            evidence_digests: vec![digest(123)],
        }
    }

    #[test]
    fn default_policy_is_valid() {
        assert_eq!(EpistemicFirewallPolicy::default().validate(), Ok(()));
    }

    #[test]
    fn promotion_is_advisory_only_but_admissible() {
        let proposal = advisory(AdvisoryAction::PromoteFidelity {
            minimum: FidelityTier::High,
        });
        assert_eq!(
            EpistemicFirewallPolicy::default().evaluate(&proposal),
            AdvisoryDisposition::Accept
        );
    }

    #[test]
    fn reduction_requires_calibrated_error_bound() {
        let mut proposal = advisory(AdvisoryAction::DemoteFidelity {
            maximum: FidelityTier::Coarse,
        });
        proposal.predicted_relative_error = None;
        assert_eq!(
            EpistemicFirewallPolicy::default().evaluate(&proposal),
            AdvisoryDisposition::Reject(AdvisoryRejection::MissingErrorBound)
        );
    }

    #[test]
    fn reduction_requires_explicit_novelty_estimate() {
        let mut proposal = advisory(AdvisoryAction::DemoteFidelity {
            maximum: FidelityTier::Coarse,
        });
        proposal.novelty_score = None;
        assert_eq!(
            EpistemicFirewallPolicy::default().evaluate(&proposal),
            AdvisoryDisposition::Reject(AdvisoryRejection::MissingNoveltyEstimate)
        );
    }

    #[test]
    fn high_novelty_escalates_to_exact_even_when_confident() {
        let mut proposal = advisory(AdvisoryAction::DemoteFidelity {
            maximum: FidelityTier::Reduced,
        });
        proposal.novelty_score = Some(0.95);
        assert_eq!(
            EpistemicFirewallPolicy::default().evaluate(&proposal),
            AdvisoryDisposition::EscalateExact(ExactEscalationReason::HighNovelty)
        );
    }

    #[test]
    fn semantic_proposal_requires_exact_provenance_link() {
        let mut proposal = advisory(AdvisoryAction::IncreaseSubsteps { minimum: 8 });
        proposal.evidence_digests.clear();
        assert_eq!(
            EpistemicFirewallPolicy::default().evaluate(&proposal),
            AdvisoryDisposition::Reject(AdvisoryRejection::InvalidProposal(
                AdvisoryValidationError::MissingSemanticEvidence
            ))
        );
    }

    #[test]
    fn inaccurate_reduction_is_rejected() {
        let mut proposal = advisory(AdvisoryAction::DecreaseSubsteps { maximum: 1 });
        proposal.predicted_relative_error = Some(0.05);
        assert_eq!(
            EpistemicFirewallPolicy::default().evaluate(&proposal),
            AdvisoryDisposition::Reject(AdvisoryRejection::ErrorBoundTooLarge)
        );
    }

    #[test]
    fn anomaly_flag_always_requests_exact_handling() {
        let proposal = advisory(AdvisoryAction::FlagAnomaly);
        assert_eq!(
            EpistemicFirewallPolicy::default().evaluate(&proposal),
            AdvisoryDisposition::EscalateExact(ExactEscalationReason::AnomalyFlagged)
        );
    }

    #[test]
    fn invalid_nan_policy_fails_closed() {
        let mut policy = EpistemicFirewallPolicy::default();
        policy.max_relative_error_for_reduction = f64::NAN;
        let proposal = advisory(AdvisoryAction::DemoteFidelity {
            maximum: FidelityTier::Coarse,
        });
        assert_eq!(
            policy.evaluate(&proposal),
            AdvisoryDisposition::Reject(AdvisoryRejection::InvalidPolicy(
                EpistemicFirewallPolicyValidationError::InvalidMaxRelativeError
            ))
        );
    }

    #[test]
    fn inverted_novelty_thresholds_fail_closed() {
        let mut policy = EpistemicFirewallPolicy::default();
        policy.max_novelty_for_reduction = 0.9;
        policy.exact_fallback_novelty = 0.8;
        let proposal = advisory(AdvisoryAction::DemoteFidelity {
            maximum: FidelityTier::Coarse,
        });
        assert_eq!(
            policy.evaluate(&proposal),
            AdvisoryDisposition::Reject(AdvisoryRejection::InvalidPolicy(
                EpistemicFirewallPolicyValidationError::InvalidNoveltyThresholdOrdering
            ))
        );
    }

    #[test]
    fn oversized_decrease_substep_request_is_rejected() {
        let policy = EpistemicFirewallPolicy {
            max_substeps: 16,
            ..EpistemicFirewallPolicy::default()
        };
        let proposal = advisory(AdvisoryAction::DecreaseSubsteps { maximum: 32 });
        assert_eq!(
            policy.evaluate(&proposal),
            AdvisoryDisposition::Reject(AdvisoryRejection::RequestedSubstepsTooLarge)
        );
    }
}
