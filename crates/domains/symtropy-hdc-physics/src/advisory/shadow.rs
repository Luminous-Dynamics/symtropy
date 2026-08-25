// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Shadow-only adaptive-physics telemetry.
//!
//! This module exists to prevent a dangerous research shortcut: treating
//! missing evidence as if it were favorable evidence. A shadow observation may
//! contain only the signals currently measured by a run. Unknown conservation,
//! constraint, novelty-calibration, or error-prediction state remains explicit
//! and therefore cannot make the observation reduction-ready.
//!
//! The shadow layer never mutates `PhysicsWorld` and never emits a fidelity
//! reduction. It may compute a known-risk fidelity floor for telemetry and
//! identify when the evidence package is complete enough for a later controlled
//! intervention through the epistemic firewall.

use serde::{Deserialize, Serialize};

use super::FidelityTier;
use super::fidelity::{AdaptiveFidelityPolicy, FidelityReason};
use crate::ExactStateDigest;

/// A semantic novelty estimate whose mapping was calibrated on a declared
/// reference/held-out corpus rather than inferred directly from raw HDC
/// similarity.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CalibratedNovelty {
    pub value: f32,
    /// Stable identifier for the calibration dataset/procedure.
    pub calibration_fingerprint: u64,
}

/// Calibrated prediction of the physical error expected from a cheaper solver
/// or lower fidelity tier.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CalibratedErrorPrediction {
    pub predicted_relative_error: f64,
    pub confidence: f32,
    /// Stable identifier for the predictor implementation/weights.
    pub predictor_fingerprint: u64,
    /// Stable identifier for the held-out calibration procedure/results.
    pub calibration_fingerprint: u64,
}

/// Signals intentionally represented as optional values. `None` means unknown,
/// not zero and not healthy.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ShadowFidelityObservation {
    pub tick: u64,
    pub exact_digest: ExactStateDigest,
    pub current_fidelity: FidelityTier,
    /// Nearest-episode HDC similarity in [-1, 1]. This is only a retrieval
    /// signal. Without an explicit calibration it is not sufficient to justify
    /// a fidelity reduction.
    pub retrieval_similarity: Option<f32>,
    pub calibrated_novelty: Option<CalibratedNovelty>,
    pub numerically_healthy: Option<bool>,
    pub conservation_residual_ratio: Option<f64>,
    pub constraint_error: Option<f32>,
    pub activity: Option<f32>,
    pub causal_importance: Option<f32>,
    pub error_prediction: Option<CalibratedErrorPrediction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShadowSignal {
    NumericalHealth,
    ConservationResidual,
    ConstraintError,
    PhysicalActivity,
    CausalImportance,
    CalibratedNovelty,
    CalibratedErrorPrediction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowTelemetryError {
    InvalidRetrievalSimilarity,
    InvalidCalibratedNovelty,
    InvalidNoveltyCalibrationFingerprint,
    InvalidConservationResidual,
    InvalidConstraintError,
    InvalidActivity,
    InvalidCausalImportance,
    InvalidPredictedError,
    InvalidPredictionConfidence,
    InvalidPredictorFingerprint,
    InvalidPredictionCalibrationFingerprint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoveltyEvidenceKind {
    Calibrated,
    RetrievalProxy,
}

/// Explainable shadow result. `known_risk_floor` may be used to request more
/// fidelity, but must never be interpreted as permission to reduce fidelity
/// when `reduction_ready` is false.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShadowFidelityAssessment {
    pub known_risk_floor: FidelityTier,
    pub reasons: Vec<FidelityReason>,
    pub novelty_value: Option<f32>,
    pub novelty_evidence: Option<NoveltyEvidenceKind>,
    pub missing_for_reduction: Vec<ShadowSignal>,
    pub reduction_ready: bool,
}

impl CalibratedNovelty {
    fn validate(self) -> Result<(), ShadowTelemetryError> {
        if !self.value.is_finite() || !(0.0..=1.0).contains(&self.value) {
            return Err(ShadowTelemetryError::InvalidCalibratedNovelty);
        }
        if self.calibration_fingerprint == 0 {
            return Err(ShadowTelemetryError::InvalidNoveltyCalibrationFingerprint);
        }
        Ok(())
    }
}

impl CalibratedErrorPrediction {
    fn validate(self) -> Result<(), ShadowTelemetryError> {
        if !self.predicted_relative_error.is_finite() || self.predicted_relative_error < 0.0 {
            return Err(ShadowTelemetryError::InvalidPredictedError);
        }
        if !self.confidence.is_finite() || !(0.0..=1.0).contains(&self.confidence) {
            return Err(ShadowTelemetryError::InvalidPredictionConfidence);
        }
        if self.predictor_fingerprint == 0 {
            return Err(ShadowTelemetryError::InvalidPredictorFingerprint);
        }
        if self.calibration_fingerprint == 0 {
            return Err(ShadowTelemetryError::InvalidPredictionCalibrationFingerprint);
        }
        Ok(())
    }
}

impl ShadowFidelityObservation {
    pub fn validate(&self) -> Result<(), ShadowTelemetryError> {
        if let Some(similarity) = self.retrieval_similarity {
            if !similarity.is_finite() || !(-1.0..=1.0).contains(&similarity) {
                return Err(ShadowTelemetryError::InvalidRetrievalSimilarity);
            }
        }
        if let Some(novelty) = self.calibrated_novelty {
            novelty.validate()?;
        }
        if let Some(residual) = self.conservation_residual_ratio {
            if !residual.is_finite() || residual < 0.0 {
                return Err(ShadowTelemetryError::InvalidConservationResidual);
            }
        }
        for (value, error) in [
            (self.constraint_error, ShadowTelemetryError::InvalidConstraintError),
            (self.activity, ShadowTelemetryError::InvalidActivity),
            (
                self.causal_importance,
                ShadowTelemetryError::InvalidCausalImportance,
            ),
        ] {
            if let Some(value) = value {
                if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                    return Err(error);
                }
            }
        }
        if let Some(prediction) = self.error_prediction {
            prediction.validate()?;
        }
        Ok(())
    }

    /// Conservative proxy used only to raise a shadow fidelity floor when no
    /// calibrated novelty mapping exists. Orthogonal or negatively correlated
    /// retrievals are treated as maximally novel.
    pub fn retrieval_novelty_proxy(&self) -> Result<Option<f32>, ShadowTelemetryError> {
        self.validate()?;
        Ok(self
            .retrieval_similarity
            .map(|similarity| 1.0 - similarity.clamp(0.0, 1.0)))
    }

    /// Return every signal still missing before this observation could even be
    /// considered for a controlled fidelity-reduction experiment.
    pub fn missing_for_reduction(&self) -> Result<Vec<ShadowSignal>, ShadowTelemetryError> {
        self.validate()?;
        let mut missing = Vec::new();
        if self.numerically_healthy.is_none() {
            missing.push(ShadowSignal::NumericalHealth);
        }
        if self.conservation_residual_ratio.is_none() {
            missing.push(ShadowSignal::ConservationResidual);
        }
        if self.constraint_error.is_none() {
            missing.push(ShadowSignal::ConstraintError);
        }
        if self.activity.is_none() {
            missing.push(ShadowSignal::PhysicalActivity);
        }
        if self.causal_importance.is_none() {
            missing.push(ShadowSignal::CausalImportance);
        }
        if self.calibrated_novelty.is_none() {
            missing.push(ShadowSignal::CalibratedNovelty);
        }
        if self.error_prediction.is_none() {
            missing.push(ShadowSignal::CalibratedErrorPrediction);
        }
        Ok(missing)
    }

    /// Compute the fidelity floor implied by evidence that is actually known.
    /// Missing evidence never contributes a favorable value. This method is for
    /// telemetry/promotion analysis only; reduction remains forbidden unless
    /// `reduction_ready` is true and a later `PhysicsAdvisory` passes the
    /// independent epistemic firewall.
    pub fn assess_known_risk(
        &self,
        policy: &AdaptiveFidelityPolicy,
    ) -> Result<ShadowFidelityAssessment, ShadowTelemetryError> {
        self.validate()?;

        let mut floor = policy.absolute_floor;
        let mut reasons = vec![FidelityReason::Baseline];

        if self.numerically_healthy == Some(false) {
            raise(
                &mut floor,
                FidelityTier::Exact,
                FidelityReason::NumericalHealth,
                &mut reasons,
            );
        }

        if let Some(residual) = self.conservation_residual_ratio {
            if residual >= policy.exact_conservation_residual {
                raise(
                    &mut floor,
                    FidelityTier::Exact,
                    FidelityReason::ConservationResidual,
                    &mut reasons,
                );
            } else if residual >= policy.high_conservation_residual {
                raise(
                    &mut floor,
                    FidelityTier::High,
                    FidelityReason::ConservationResidual,
                    &mut reasons,
                );
            }
        }

        if let Some(error) = self.constraint_error {
            if error >= policy.exact_constraint_error {
                raise(
                    &mut floor,
                    FidelityTier::Exact,
                    FidelityReason::ConstraintError,
                    &mut reasons,
                );
            } else if error >= policy.high_constraint_error {
                raise(
                    &mut floor,
                    FidelityTier::High,
                    FidelityReason::ConstraintError,
                    &mut reasons,
                );
            }
        }

        if self.activity.is_some_and(|value| value >= policy.high_activity) {
            raise(
                &mut floor,
                FidelityTier::High,
                FidelityReason::PhysicalActivity,
                &mut reasons,
            );
        }

        if let Some(importance) = self.causal_importance {
            if importance >= policy.high_causal_importance {
                raise(
                    &mut floor,
                    FidelityTier::High,
                    FidelityReason::CausalImportance,
                    &mut reasons,
                );
            } else if importance >= policy.standard_causal_importance {
                raise(
                    &mut floor,
                    FidelityTier::Standard,
                    FidelityReason::CausalImportance,
                    &mut reasons,
                );
            }
        }

        let (novelty_value, novelty_evidence) = if let Some(novelty) = self.calibrated_novelty {
            (Some(novelty.value), Some(NoveltyEvidenceKind::Calibrated))
        } else {
            (
                self.retrieval_novelty_proxy()?,
                self.retrieval_similarity
                    .map(|_| NoveltyEvidenceKind::RetrievalProxy),
            )
        };
        if let Some(novelty) = novelty_value {
            if novelty >= policy.exact_novelty {
                raise(
                    &mut floor,
                    FidelityTier::Exact,
                    FidelityReason::SemanticNovelty,
                    &mut reasons,
                );
            } else if novelty >= policy.high_novelty {
                raise(
                    &mut floor,
                    FidelityTier::High,
                    FidelityReason::SemanticNovelty,
                    &mut reasons,
                );
            }
        }

        if let Some(prediction) = self.error_prediction {
            let error = prediction.predicted_relative_error;
            if error >= policy.exact_predicted_error {
                raise(
                    &mut floor,
                    FidelityTier::Exact,
                    FidelityReason::PredictedError,
                    &mut reasons,
                );
            } else if error >= policy.high_predicted_error {
                raise(
                    &mut floor,
                    FidelityTier::High,
                    FidelityReason::PredictedError,
                    &mut reasons,
                );
            } else if error >= policy.standard_predicted_error {
                raise(
                    &mut floor,
                    FidelityTier::Standard,
                    FidelityReason::PredictedError,
                    &mut reasons,
                );
            }
        }

        let missing_for_reduction = self.missing_for_reduction()?;
        Ok(ShadowFidelityAssessment {
            known_risk_floor: floor,
            reasons,
            novelty_value,
            novelty_evidence,
            reduction_ready: missing_for_reduction.is_empty(),
            missing_for_reduction,
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn digest() -> ExactStateDigest {
        ExactStateDigest {
            algorithm_version: 2,
            low: 11,
            high: 22,
        }
    }

    fn empty_shadow() -> ShadowFidelityObservation {
        ShadowFidelityObservation {
            tick: 7,
            exact_digest: digest(),
            current_fidelity: FidelityTier::Exact,
            retrieval_similarity: None,
            calibrated_novelty: None,
            numerically_healthy: None,
            conservation_residual_ratio: None,
            constraint_error: None,
            activity: None,
            causal_importance: None,
            error_prediction: None,
        }
    }

    #[test]
    fn unknown_evidence_is_not_treated_as_zero() {
        let assessment = empty_shadow()
            .assess_known_risk(&AdaptiveFidelityPolicy::default())
            .unwrap();
        assert!(!assessment.reduction_ready);
        assert!(
            assessment
                .missing_for_reduction
                .contains(&ShadowSignal::ConservationResidual)
        );
        assert!(
            assessment
                .missing_for_reduction
                .contains(&ShadowSignal::CalibratedErrorPrediction)
        );
    }

    #[test]
    fn exact_health_failure_promotes_even_with_other_signals_missing() {
        let mut shadow = empty_shadow();
        shadow.numerically_healthy = Some(false);
        let assessment = shadow
            .assess_known_risk(&AdaptiveFidelityPolicy::default())
            .unwrap();
        assert_eq!(assessment.known_risk_floor, FidelityTier::Exact);
        assert!(assessment.reasons.contains(&FidelityReason::NumericalHealth));
        assert!(!assessment.reduction_ready);
    }

    #[test]
    fn unrelated_retrieval_is_high_novelty_proxy_but_not_reduction_evidence() {
        let mut shadow = empty_shadow();
        shadow.retrieval_similarity = Some(0.0);
        let assessment = shadow
            .assess_known_risk(&AdaptiveFidelityPolicy::default())
            .unwrap();
        assert_eq!(assessment.novelty_value, Some(1.0));
        assert_eq!(
            assessment.novelty_evidence,
            Some(NoveltyEvidenceKind::RetrievalProxy)
        );
        assert_eq!(assessment.known_risk_floor, FidelityTier::Exact);
        assert!(
            assessment
                .missing_for_reduction
                .contains(&ShadowSignal::CalibratedNovelty)
        );
    }

    #[test]
    fn complete_calibrated_evidence_can_be_marked_reduction_ready() {
        let shadow = ShadowFidelityObservation {
            tick: 9,
            exact_digest: digest(),
            current_fidelity: FidelityTier::Exact,
            retrieval_similarity: Some(0.95),
            calibrated_novelty: Some(CalibratedNovelty {
                value: 0.05,
                calibration_fingerprint: 100,
            }),
            numerically_healthy: Some(true),
            conservation_residual_ratio: Some(1.0e-12),
            constraint_error: Some(0.001),
            activity: Some(0.05),
            causal_importance: Some(0.10),
            error_prediction: Some(CalibratedErrorPrediction {
                predicted_relative_error: 0.0005,
                confidence: 0.99,
                predictor_fingerprint: 200,
                calibration_fingerprint: 300,
            }),
        };
        let assessment = shadow
            .assess_known_risk(&AdaptiveFidelityPolicy::default())
            .unwrap();
        assert!(assessment.reduction_ready);
        assert!(assessment.missing_for_reduction.is_empty());
        assert_eq!(assessment.known_risk_floor, FidelityTier::Coarse);
    }

    #[test]
    fn calibration_identifiers_are_required() {
        let mut shadow = empty_shadow();
        shadow.calibrated_novelty = Some(CalibratedNovelty {
            value: 0.1,
            calibration_fingerprint: 0,
        });
        assert_eq!(
            shadow.validate(),
            Err(ShadowTelemetryError::InvalidNoveltyCalibrationFingerprint)
        );
    }

    #[test]
    fn invalid_similarity_is_rejected() {
        let mut shadow = empty_shadow();
        shadow.retrieval_similarity = Some(1.01);
        assert_eq!(
            shadow.validate(),
            Err(ShadowTelemetryError::InvalidRetrievalSimilarity)
        );
    }
}
