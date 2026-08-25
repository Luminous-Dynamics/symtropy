// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Explainable adaptive-fidelity policy built on top of the epistemic firewall.
//!
//! This module does not choose a concrete physics solver and never mutates the
//! world. It converts exact numerical evidence plus semantic/temporal signals
//! into a minimum fidelity floor and, optionally, a typed `PhysicsAdvisory`.
//! Any resulting reduction must still pass `EpistemicFirewallPolicy`.

use serde::{Deserialize, Serialize};

use super::{AdvisoryAction, AdvisorySource, FidelityTier, PhysicsAdvisory};
use crate::ExactStateDigest;

/// Normalized evidence available to the adaptive-fidelity controller.
///
/// Values in [0, 1] are policy-normalized signals, not raw physical units.
/// Exact physical diagnostics should be normalized by a scenario-specific,
/// versioned adapter before entering this policy.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FidelityEvidence {
    /// False means an exact invariant/finite-state guard has already failed.
    pub numerically_healthy: bool,
    /// Absolute conservation/reconciliation residual divided by the declared
    /// scenario energy scale.
    pub conservation_residual_ratio: f64,
    /// Normalized contact/constraint error in [0, 1].
    pub constraint_error: f32,
    /// Normalized physical activity/instability in [0, 1].
    pub activity: f32,
    /// Gameplay/causal importance in [0, 1].
    pub causal_importance: f32,
    /// Semantic novelty in [0, 1].
    pub novelty: f32,
    /// Predicted relative physical error at the current fidelity, if a
    /// calibrated model provides one.
    pub predicted_relative_error: Option<f64>,
    /// Calibrated confidence of the semantic/temporal advisor in [0, 1].
    pub model_confidence: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FidelityEvidenceError {
    InvalidConservationResidual,
    InvalidConstraintError,
    InvalidActivity,
    InvalidCausalImportance,
    InvalidNovelty,
    InvalidPredictedError,
    InvalidModelConfidence,
}

impl FidelityEvidence {
    pub fn validate(&self) -> Result<(), FidelityEvidenceError> {
        if !self.conservation_residual_ratio.is_finite() || self.conservation_residual_ratio < 0.0 {
            return Err(FidelityEvidenceError::InvalidConservationResidual);
        }
        for (value, error) in [
            (self.constraint_error, FidelityEvidenceError::InvalidConstraintError),
            (self.activity, FidelityEvidenceError::InvalidActivity),
            (
                self.causal_importance,
                FidelityEvidenceError::InvalidCausalImportance,
            ),
            (self.novelty, FidelityEvidenceError::InvalidNovelty),
            (
                self.model_confidence,
                FidelityEvidenceError::InvalidModelConfidence,
            ),
        ] {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(error);
            }
        }
        if let Some(error) = self.predicted_relative_error {
            if !error.is_finite() || error < 0.0 {
                return Err(FidelityEvidenceError::InvalidPredictedError);
            }
        }
        Ok(())
    }
}

/// Why a minimum fidelity floor was selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FidelityReason {
    NumericalHealth,
    ConservationResidual,
    ConstraintError,
    PhysicalActivity,
    CausalImportance,
    SemanticNovelty,
    PredictedError,
    Baseline,
}

/// Explainable fidelity floor. Reasons are emitted in deterministic order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FidelityAssessment {
    pub minimum: FidelityTier,
    pub reasons: Vec<FidelityReason>,
}

/// Versionable thresholds for converting evidence into a fidelity floor.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AdaptiveFidelityPolicy {
    pub exact_conservation_residual: f64,
    pub high_conservation_residual: f64,
    pub exact_constraint_error: f32,
    pub high_constraint_error: f32,
    pub exact_novelty: f32,
    pub high_novelty: f32,
    pub high_activity: f32,
    pub high_causal_importance: f32,
    pub standard_causal_importance: f32,
    pub exact_predicted_error: f64,
    pub high_predicted_error: f64,
    pub standard_predicted_error: f64,
    /// Lowest tier this policy may ever recommend, even in calm/familiar state.
    pub absolute_floor: FidelityTier,
}

impl Default for AdaptiveFidelityPolicy {
    fn default() -> Self {
        Self {
            exact_conservation_residual: 1.0e-3,
            high_conservation_residual: 1.0e-5,
            exact_constraint_error: 0.20,
            high_constraint_error: 0.05,
            exact_novelty: 0.80,
            high_novelty: 0.40,
            high_activity: 0.80,
            high_causal_importance: 0.80,
            standard_causal_importance: 0.50,
            exact_predicted_error: 0.05,
            high_predicted_error: 0.01,
            standard_predicted_error: 0.002,
            absolute_floor: FidelityTier::Coarse,
        }
    }
}

impl AdaptiveFidelityPolicy {
    /// Compute the minimum acceptable fidelity from independent evidence floors.
    ///
    /// The aggregation is a max-lattice, not a learned weighted sum: any exact
    /// or semantic risk signal may raise fidelity on its own, and every reason
    /// remains inspectable.
    pub fn assess(
        &self,
        evidence: &FidelityEvidence,
    ) -> Result<FidelityAssessment, FidelityEvidenceError> {
        evidence.validate()?;

        let mut minimum = self.absolute_floor;
        let mut reasons = vec![FidelityReason::Baseline];

        if !evidence.numerically_healthy {
            raise(
                &mut minimum,
                FidelityTier::Exact,
                FidelityReason::NumericalHealth,
                &mut reasons,
            );
        }

        if evidence.conservation_residual_ratio >= self.exact_conservation_residual {
            raise(
                &mut minimum,
                FidelityTier::Exact,
                FidelityReason::ConservationResidual,
                &mut reasons,
            );
        } else if evidence.conservation_residual_ratio >= self.high_conservation_residual {
            raise(
                &mut minimum,
                FidelityTier::High,
                FidelityReason::ConservationResidual,
                &mut reasons,
            );
        }

        if evidence.constraint_error >= self.exact_constraint_error {
            raise(
                &mut minimum,
                FidelityTier::Exact,
                FidelityReason::ConstraintError,
                &mut reasons,
            );
        } else if evidence.constraint_error >= self.high_constraint_error {
            raise(
                &mut minimum,
                FidelityTier::High,
                FidelityReason::ConstraintError,
                &mut reasons,
            );
        }

        if evidence.activity >= self.high_activity {
            raise(
                &mut minimum,
                FidelityTier::High,
                FidelityReason::PhysicalActivity,
                &mut reasons,
            );
        }

        if evidence.causal_importance >= self.high_causal_importance {
            raise(
                &mut minimum,
                FidelityTier::High,
                FidelityReason::CausalImportance,
                &mut reasons,
            );
        } else if evidence.causal_importance >= self.standard_causal_importance {
            raise(
                &mut minimum,
                FidelityTier::Standard,
                FidelityReason::CausalImportance,
                &mut reasons,
            );
        }

        if evidence.novelty >= self.exact_novelty {
            raise(
                &mut minimum,
                FidelityTier::Exact,
                FidelityReason::SemanticNovelty,
                &mut reasons,
            );
        } else if evidence.novelty >= self.high_novelty {
            raise(
                &mut minimum,
                FidelityTier::High,
                FidelityReason::SemanticNovelty,
                &mut reasons,
            );
        }

        if let Some(error) = evidence.predicted_relative_error {
            if error >= self.exact_predicted_error {
                raise(
                    &mut minimum,
                    FidelityTier::Exact,
                    FidelityReason::PredictedError,
                    &mut reasons,
                );
            } else if error >= self.high_predicted_error {
                raise(
                    &mut minimum,
                    FidelityTier::High,
                    FidelityReason::PredictedError,
                    &mut reasons,
                );
            } else if error >= self.standard_predicted_error {
                raise(
                    &mut minimum,
                    FidelityTier::Standard,
                    FidelityReason::PredictedError,
                    &mut reasons,
                );
            }
        }

        Ok(FidelityAssessment { minimum, reasons })
    }

    /// Convert an assessment into a typed advisory.
    ///
    /// Promotions may jump directly to the required floor. Demotions are
    /// deliberately limited to one tier per proposal and require a predicted
    /// error estimate so the epistemic firewall can independently admit them.
    #[allow(clippy::too_many_arguments)]
    pub fn propose(
        &self,
        proposal_id: u64,
        tick: u64,
        source: AdvisorySource,
        current: FidelityTier,
        exact_digest: ExactStateDigest,
        evidence: FidelityEvidence,
    ) -> Result<Option<PhysicsAdvisory>, FidelityEvidenceError> {
        let assessment = self.assess(&evidence)?;

        let action = if assessment.minimum > current {
            AdvisoryAction::PromoteFidelity {
                minimum: assessment.minimum,
            }
        } else if assessment.minimum < current {
            if evidence.predicted_relative_error.is_none() {
                return Ok(None);
            }
            AdvisoryAction::DemoteFidelity {
                maximum: one_tier_down(current).max(assessment.minimum),
            }
        } else {
            return Ok(None);
        };

        Ok(Some(PhysicsAdvisory {
            proposal_id,
            tick,
            source,
            action,
            confidence: evidence.model_confidence,
            predicted_relative_error: evidence.predicted_relative_error,
            novelty_score: Some(evidence.novelty),
            evidence_digests: vec![exact_digest],
        }))
    }
}

fn raise(
    current: &mut FidelityTier,
    requested: FidelityTier,
    reason: FidelityReason,
    reasons: &mut Vec<FidelityReason>,
) {
    if requested > *current {
        *current = requested;
    }
    if !reasons.contains(&reason) {
        reasons.push(reason);
    }
}

fn one_tier_down(tier: FidelityTier) -> FidelityTier {
    match tier {
        FidelityTier::Exact => FidelityTier::High,
        FidelityTier::High => FidelityTier::Standard,
        FidelityTier::Standard => FidelityTier::Coarse,
        FidelityTier::Coarse => FidelityTier::Reduced,
        FidelityTier::Reduced => FidelityTier::Reduced,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::advisory::{AdvisoryDisposition, EpistemicFirewallPolicy};

    fn digest() -> ExactStateDigest {
        ExactStateDigest {
            algorithm_version: 1,
            low: 1,
            high: 2,
        }
    }

    fn calm() -> FidelityEvidence {
        FidelityEvidence {
            numerically_healthy: true,
            conservation_residual_ratio: 1.0e-12,
            constraint_error: 0.001,
            activity: 0.05,
            causal_importance: 0.10,
            novelty: 0.05,
            predicted_relative_error: Some(0.001),
            model_confidence: 0.99,
        }
    }

    #[test]
    fn exact_health_failure_overrides_every_learned_signal() {
        let mut evidence = calm();
        evidence.numerically_healthy = false;
        evidence.model_confidence = 1.0;
        evidence.predicted_relative_error = Some(0.0);
        let assessment = AdaptiveFidelityPolicy::default().assess(&evidence).unwrap();
        assert_eq!(assessment.minimum, FidelityTier::Exact);
        assert!(assessment.reasons.contains(&FidelityReason::NumericalHealth));
    }

    #[test]
    fn novel_state_forces_exact_fidelity() {
        let mut evidence = calm();
        evidence.novelty = 0.95;
        let assessment = AdaptiveFidelityPolicy::default().assess(&evidence).unwrap();
        assert_eq!(assessment.minimum, FidelityTier::Exact);
        assert!(assessment.reasons.contains(&FidelityReason::SemanticNovelty));
    }

    #[test]
    fn high_causal_importance_promotes_far_or_quiet_state() {
        let mut evidence = calm();
        evidence.causal_importance = 0.95;
        let assessment = AdaptiveFidelityPolicy::default().assess(&evidence).unwrap();
        assert_eq!(assessment.minimum, FidelityTier::High);
    }

    #[test]
    fn calm_known_state_demotes_only_one_tier() {
        let proposal = AdaptiveFidelityPolicy::default()
            .propose(
                1,
                10,
                AdvisorySource::HdcContinuousTimeHybrid,
                FidelityTier::Exact,
                digest(),
                calm(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            proposal.action,
            AdvisoryAction::DemoteFidelity {
                maximum: FidelityTier::High
            }
        );
        assert_eq!(
            EpistemicFirewallPolicy::default().evaluate(&proposal),
            AdvisoryDisposition::Accept
        );
    }

    #[test]
    fn no_error_model_means_no_demotion_proposal() {
        let mut evidence = calm();
        evidence.predicted_relative_error = None;
        let proposal = AdaptiveFidelityPolicy::default()
            .propose(
                1,
                10,
                AdvisorySource::ContinuousTimeModel,
                FidelityTier::High,
                digest(),
                evidence,
            )
            .unwrap();
        assert!(proposal.is_none());
    }

    #[test]
    fn predicted_error_can_force_promotion() {
        let mut evidence = calm();
        evidence.predicted_relative_error = Some(0.02);
        let proposal = AdaptiveFidelityPolicy::default()
            .propose(
                1,
                10,
                AdvisorySource::ContinuousTimeModel,
                FidelityTier::Standard,
                digest(),
                evidence,
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            proposal.action,
            AdvisoryAction::PromoteFidelity {
                minimum: FidelityTier::High
            }
        );
    }
}
