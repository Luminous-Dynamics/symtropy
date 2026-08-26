// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Shadow-only adaptive-physics telemetry.
//!
//! This module exists to prevent a dangerous research shortcut: treating
//! missing evidence as if it were favorable evidence. A shadow observation may
//! contain only the signals currently measured by a run. Unknown conservation,
//! accounting, lifecycle, constraint, novelty-calibration, or error-prediction
//! state remains explicit and cannot make the observation reduction-ready.
//!
//! The shadow layer never mutates `PhysicsWorld` and never emits a fidelity
//! reduction. It may compute a known-risk fidelity floor for telemetry and
//! identify when an evidence package is complete enough for a later controlled
//! intervention through the epistemic firewall.

use serde::{Deserialize, Serialize};

use super::FidelityTier;
use super::fidelity::{
    AdaptiveFidelityPolicy, AdaptiveFidelityPolicyValidationError, FidelityEvidence,
    FidelityReason,
};
use crate::ExactStateDigest;

/// A semantic novelty estimate calibrated on a declared held-out corpus.
///
/// The encoder fingerprint is explicit because novelty calibration for one HDC
/// schema/configuration must never be silently reused for another.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CalibratedNovelty {
    pub value: f32,
    pub encoder_fingerprint: u64,
    /// Stable identifier for the calibration dataset/procedure/results.
    pub calibration_fingerprint: u64,
}

/// Calibrated prediction of the physical error expected from a declared cheaper
/// fidelity tier.
///
/// The scalar error is meaningful only relative to a versioned metric profile.
/// That profile may define a conservative envelope over trajectory, contact,
/// conservation/reconciliation, topology, and task-specific metrics. The
/// fingerprint prevents a numerically small value under one metric definition
/// from being reused as evidence under another.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CalibratedErrorPrediction {
    pub target_fidelity: FidelityTier,
    pub predicted_relative_error: f64,
    pub confidence: f32,
    pub metric_profile_fingerprint: u64,
    /// Stable identifier for the predictor implementation/weights.
    pub predictor_fingerprint: u64,
    /// Stable identifier for the held-out calibration procedure/results.
    pub calibration_fingerprint: u64,
}

/// Signals intentionally represented as optional values. `None` means unknown,
/// not zero, not healthy, not complete, and not stable.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ShadowFidelityObservation {
    pub tick: u64,
    pub exact_digest: ExactStateDigest,
    pub current_fidelity: FidelityTier,
    /// Nearest-episode HDC similarity in [-1, 1]. This is only a retrieval
    /// signal. Without explicit calibration it cannot justify fidelity reduction.
    pub retrieval_similarity: Option<f32>,
    pub calibrated_novelty: Option<CalibratedNovelty>,
    pub numerically_healthy: Option<bool>,
    /// True only when the active model's declared energy/accounting reservoirs
    /// are known to be completely represented for the assessed interval.
    pub accounting_complete: Option<bool>,
    /// True only when body/reservoir/representation identity is known to have a
    /// complete authoritative lifecycle/transition provenance contract.
    pub lifecycle_stable: Option<bool>,
    pub conservation_residual_ratio: Option<f64>,
    pub constraint_error: Option<f32>,
    pub activity: Option<f32>,
    pub causal_importance: Option<f32>,
    pub error_prediction: Option<CalibratedErrorPrediction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShadowSignal {
    NumericalHealth,
    AccountingCompleteness,
    LifecycleStability,
    ConservationResidual,
    ConstraintError,
    PhysicalActivity,
    CausalImportance,
    CalibratedNovelty,
    CalibratedErrorPrediction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowTelemetryError {
    InvalidPolicy(AdaptiveFidelityPolicyValidationError),
    InvalidExactDigestVersion,
    InvalidRetrievalSimilarity,
    InvalidCalibratedNovelty,
    InvalidNoveltyEncoderFingerprint,
    InvalidNoveltyCalibrationFingerprint,
    InvalidConservationResidual,
    InvalidConstraintError,
    InvalidActivity,
    InvalidCausalImportance,
    InvalidPredictedError,
    InvalidPredictionConfidence,
    InvalidPredictionTarget,
    InvalidMetricProfileFingerprint,
    InvalidPredictorFingerprint,
    InvalidPredictionCalibrationFingerprint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoveltyEvidenceKind {
    Calibrated,
    RetrievalProxy,
}

/// Why a complete packet is still not an eligible one-tier reduction candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShadowReductionBlocker {
    NumericalHealthFailed,
    AccountingIncomplete,
    LifecycleUnstable,
    PredictionTargetMismatch,
    KnownRiskFloorNotBelowCurrent,
}

/// Explainable shadow result. `known_risk_floor` may be used to request more
/// fidelity, but must never be interpreted as permission to reduce fidelity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShadowFidelityAssessment {
    pub known_risk_floor: FidelityTier,
    pub reasons: Vec<FidelityReason>,
    pub novelty_value: Option<f32>,
    pub novelty_evidence: Option<NoveltyEvidenceKind>,
    pub missing_for_reduction: Vec<ShadowSignal>,
    /// True when every signal needed to construct a complete `FidelityEvidence`
    /// packet is present and valid. Completeness is not permission.
    pub evidence_complete: bool,
    /// Explicit reasons a complete packet still cannot be treated as a one-tier
    /// reduction candidate.
    pub reduction_blockers: Vec<ShadowReductionBlocker>,
    /// Convenience flag: complete evidence, healthy/complete/stable authority,
    /// a prediction for exactly one tier down, and a known-risk floor below the
    /// current tier. A later controller and epistemic firewall must still admit
    /// the proposal before any controlled intervention.
    pub reduction_ready: bool,
}

impl CalibratedNovelty {
    fn validate(self) -> Result<(), ShadowTelemetryError> {
        if !self.value.is_finite() || !(0.0..=1.0).contains(&self.value) {
            return Err(ShadowTelemetryError::InvalidCalibratedNovelty);
        }
        if self.encoder_fingerprint == 0 {
            return Err(ShadowTelemetryError::InvalidNoveltyEncoderFingerprint);
        }
        if self.calibration_fingerprint == 0 {
            return Err(ShadowTelemetryError::InvalidNoveltyCalibrationFingerprint);
        }
        Ok(())
    }
}

impl CalibratedErrorPrediction {
    fn validate_for(self, current_fidelity: FidelityTier) -> Result<(), ShadowTelemetryError> {
        if self.target_fidelity >= current_fidelity {
            return Err(ShadowTelemetryError::InvalidPredictionTarget);
        }
        if !self.predicted_relative_error.is_finite() || self.predicted_relative_error < 0.0 {
            return Err(ShadowTelemetryError::InvalidPredictedError);
        }
        if !self.confidence.is_finite() || !(0.0..=1.0).contains(&self.confidence) {
            return Err(ShadowTelemetryError::InvalidPredictionConfidence);
        }
        if self.metric_profile_fingerprint == 0 {
            return Err(ShadowTelemetryError::InvalidMetricProfileFingerprint);
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
        if self.exact_digest.algorithm_version == 0 {
            return Err(ShadowTelemetryError::InvalidExactDigestVersion);
        }
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
            prediction.validate_for(self.current_fidelity)?;
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

    /// Return every signal still missing before a complete controller evidence
    /// packet can be constructed.
    pub fn missing_for_reduction(&self) -> Result<Vec<ShadowSignal>, ShadowTelemetryError> {
        self.validate()?;
        let mut missing = Vec::new();
        if self.numerically_healthy.is_none() {
            missing.push(ShadowSignal::NumericalHealth);
        }
        if self.accounting_complete.is_none() {
            missing.push(ShadowSignal::AccountingCompleteness);
        }
        if self.lifecycle_stable.is_none() {
            missing.push(ShadowSignal::LifecycleStability);
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

    /// Construct the complete controller input only when every required signal
    /// is actually present. Missing values are never synthesized.
    pub fn complete_fidelity_evidence(
        &self,
    ) -> Result<Option<FidelityEvidence>, ShadowTelemetryError> {
        self.validate()?;
        let (
            Some(numerically_healthy),
            Some(accounting_complete),
            Some(lifecycle_stable),
            Some(conservation_residual_ratio),
            Some(constraint_error),
            Some(activity),
            Some(causal_importance),
            Some(calibrated_novelty),
            Some(error_prediction),
        ) = (
            self.numerically_healthy,
            self.accounting_complete,
            self.lifecycle_stable,
            self.conservation_residual_ratio,
            self.constraint_error,
            self.activity,
            self.causal_importance,
            self.calibrated_novelty,
            self.error_prediction,
        )
        else {
            return Ok(None);
        };

        Ok(Some(FidelityEvidence {
            numerically_healthy,
            accounting_complete,
            lifecycle_stable,
            conservation_residual_ratio,
            constraint_error,
            activity,
            causal_importance,
            novelty: calibrated_novelty.value,
            predicted_relative_error: Some(error_prediction.predicted_relative_error),
            model_confidence: error_prediction.confidence,
        }))
    }

    /// Compute the fidelity floor implied by evidence that is actually known.
    /// Missing evidence never contributes a favorable value. This method is for
    /// telemetry/promotion analysis only.
    pub fn assess_known_risk(
        &self,
        policy: &AdaptiveFidelityPolicy,
    ) -> Result<ShadowFidelityAssessment, ShadowTelemetryError> {
        policy
            .validate()
            .map_err(ShadowTelemetryError::InvalidPolicy)?;
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
        if self.accounting_complete == Some(false) {
            raise(
                &mut floor,
                FidelityTier::Exact,
                FidelityReason::AccountingCompleteness,
                &mut reasons,
            );
        }
        if self.lifecycle_stable == Some(false) {
            raise(
                &mut floor,
                FidelityTier::Exact,
                FidelityReason::LifecycleStability,
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
        let evidence_complete = missing_for_reduction.is_empty();
        let mut reduction_blockers = Vec::new();

        if evidence_complete {
            if self.numerically_healthy != Some(true) {
                reduction_blockers.push(ShadowReductionBlocker::NumericalHealthFailed);
            }
            if self.accounting_complete != Some(true) {
                reduction_blockers.push(ShadowReductionBlocker::AccountingIncomplete);
            }
            if self.lifecycle_stable != Some(true) {
                reduction_blockers.push(ShadowReductionBlocker::LifecycleUnstable);
            }
            if self
                .error_prediction
                .is_some_and(|prediction| prediction.target_fidelity != one_tier_down(self.current_fidelity))
            {
                reduction_blockers.push(ShadowReductionBlocker::PredictionTargetMismatch);
            }
            if floor >= self.current_fidelity {
                reduction_blockers.push(ShadowReductionBlocker::KnownRiskFloorNotBelowCurrent);
            }
        }

        let reduction_ready = evidence_complete && reduction_blockers.is_empty();

        Ok(ShadowFidelityAssessment {
            known_risk_floor: floor,
            reasons,
            novelty_value,
            novelty_evidence,
            missing_for_reduction,
            evidence_complete,
            reduction_blockers,
            reduction_ready,
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
            accounting_complete: None,
            lifecycle_stable: None,
            conservation_residual_ratio: None,
            constraint_error: None,
            activity: None,
            causal_importance: None,
            error_prediction: None,
        }
    }

    fn complete_shadow() -> ShadowFidelityObservation {
        ShadowFidelityObservation {
            tick: 9,
            exact_digest: digest(),
            current_fidelity: FidelityTier::Exact,
            retrieval_similarity: Some(0.95),
            calibrated_novelty: Some(CalibratedNovelty {
                value: 0.05,
                encoder_fingerprint: 100,
                calibration_fingerprint: 101,
            }),
            numerically_healthy: Some(true),
            accounting_complete: Some(true),
            lifecycle_stable: Some(true),
            conservation_residual_ratio: Some(1.0e-12),
            constraint_error: Some(0.001),
            activity: Some(0.05),
            causal_importance: Some(0.10),
            error_prediction: Some(CalibratedErrorPrediction {
                target_fidelity: FidelityTier::High,
                predicted_relative_error: 0.0005,
                confidence: 0.99,
                metric_profile_fingerprint: 150,
                predictor_fingerprint: 200,
                calibration_fingerprint: 300,
            }),
        }
    }

    #[test]
    fn unknown_evidence_is_not_treated_as_zero() {
        let assessment = empty_shadow()
            .assess_known_risk(&AdaptiveFidelityPolicy::default())
            .unwrap();
        assert!(!assessment.evidence_complete);
        assert!(!assessment.reduction_ready);
        assert!(
            assessment
                .missing_for_reduction
                .contains(&ShadowSignal::AccountingCompleteness)
        );
        assert!(
            assessment
                .missing_for_reduction
                .contains(&ShadowSignal::LifecycleStability)
        );
        assert!(
            assessment
                .missing_for_reduction
                .contains(&ShadowSignal::ConservationResidual)
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
    fn incomplete_accounting_forces_exact_even_with_zero_residual() {
        let mut shadow = complete_shadow();
        shadow.accounting_complete = Some(false);
        shadow.conservation_residual_ratio = Some(0.0);
        let assessment = shadow
            .assess_known_risk(&AdaptiveFidelityPolicy::default())
            .unwrap();
        assert!(assessment.evidence_complete);
        assert_eq!(assessment.known_risk_floor, FidelityTier::Exact);
        assert!(assessment.reasons.contains(&FidelityReason::AccountingCompleteness));
        assert!(
            assessment
                .reduction_blockers
                .contains(&ShadowReductionBlocker::AccountingIncomplete)
        );
        assert!(!assessment.reduction_ready);
    }

    #[test]
    fn unstable_lifecycle_forces_exact_even_with_zero_residual() {
        let mut shadow = complete_shadow();
        shadow.lifecycle_stable = Some(false);
        shadow.conservation_residual_ratio = Some(0.0);
        let assessment = shadow
            .assess_known_risk(&AdaptiveFidelityPolicy::default())
            .unwrap();
        assert_eq!(assessment.known_risk_floor, FidelityTier::Exact);
        assert!(assessment.reasons.contains(&FidelityReason::LifecycleStability));
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
    fn complete_calibrated_evidence_can_be_a_one_tier_candidate() {
        let shadow = complete_shadow();
        let assessment = shadow
            .assess_known_risk(&AdaptiveFidelityPolicy::default())
            .unwrap();
        assert!(assessment.evidence_complete);
        assert!(assessment.reduction_ready);
        assert!(assessment.missing_for_reduction.is_empty());
        assert!(assessment.reduction_blockers.is_empty());
        assert_eq!(assessment.known_risk_floor, FidelityTier::Coarse);

        let evidence = shadow.complete_fidelity_evidence().unwrap().unwrap();
        assert!(evidence.accounting_complete);
        assert!(evidence.lifecycle_stable);
    }

    #[test]
    fn prediction_must_target_exactly_one_tier_down_for_readiness() {
        let mut shadow = complete_shadow();
        shadow.error_prediction.as_mut().unwrap().target_fidelity = FidelityTier::Standard;
        let assessment = shadow
            .assess_known_risk(&AdaptiveFidelityPolicy::default())
            .unwrap();
        assert!(assessment.evidence_complete);
        assert!(
            assessment
                .reduction_blockers
                .contains(&ShadowReductionBlocker::PredictionTargetMismatch)
        );
        assert!(!assessment.reduction_ready);
    }

    #[test]
    fn calibration_identifiers_are_required() {
        let mut shadow = empty_shadow();
        shadow.calibrated_novelty = Some(CalibratedNovelty {
            value: 0.1,
            encoder_fingerprint: 100,
            calibration_fingerprint: 0,
        });
        assert_eq!(
            shadow.validate(),
            Err(ShadowTelemetryError::InvalidNoveltyCalibrationFingerprint)
        );
    }

    #[test]
    fn error_metric_profile_is_required() {
        let mut shadow = complete_shadow();
        shadow
            .error_prediction
            .as_mut()
            .unwrap()
            .metric_profile_fingerprint = 0;
        assert_eq!(
            shadow.validate(),
            Err(ShadowTelemetryError::InvalidMetricProfileFingerprint)
        );
    }

    #[test]
    fn invalid_policy_fails_closed() {
        let mut policy = AdaptiveFidelityPolicy::default();
        policy.high_novelty = f32::NAN;
        assert_eq!(
            empty_shadow().assess_known_risk(&policy),
            Err(ShadowTelemetryError::InvalidPolicy(
                AdaptiveFidelityPolicyValidationError::InvalidNoveltyThresholds
            ))
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
