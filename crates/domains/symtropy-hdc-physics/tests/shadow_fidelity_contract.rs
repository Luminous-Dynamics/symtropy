// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_hdc_physics::ExactStateDigest;
use symtropy_hdc_physics::advisory::FidelityTier;
use symtropy_hdc_physics::advisory::fidelity::{AdaptiveFidelityPolicy, FidelityReason};
use symtropy_hdc_physics::advisory::shadow::{
    CalibratedErrorPrediction, CalibratedNovelty, NoveltyEvidenceKind,
    ShadowFidelityObservation, ShadowReductionBlocker, ShadowSignal,
};

fn digest() -> ExactStateDigest {
    ExactStateDigest {
        algorithm_version: 2,
        low: 0x1234,
        high: 0x5678,
    }
}

fn complete_observation() -> ShadowFidelityObservation {
    ShadowFidelityObservation {
        tick: 102,
        exact_digest: digest(),
        current_fidelity: FidelityTier::Exact,
        retrieval_similarity: Some(0.96),
        calibrated_novelty: Some(CalibratedNovelty {
            value: 0.04,
            encoder_fingerprint: 0xE11C_0001,
            calibration_fingerprint: 0xCA11_BA7E,
        }),
        numerically_healthy: Some(true),
        accounting_complete: Some(true),
        lifecycle_stable: Some(true),
        conservation_residual_ratio: Some(1.0e-12),
        constraint_error: Some(0.001),
        activity: Some(0.02),
        causal_importance: Some(0.10),
        error_prediction: Some(CalibratedErrorPrediction {
            target_fidelity: FidelityTier::High,
            predicted_relative_error: 0.0004,
            confidence: 0.99,
            metric_profile_fingerprint: 0xE770_1001,
            predictor_fingerprint: 0xCFC0_0001,
            calibration_fingerprint: 0xE770_0001,
        }),
    }
}

#[test]
fn public_shadow_api_keeps_missing_evidence_non_reducible() {
    let observation = ShadowFidelityObservation {
        tick: 100,
        exact_digest: digest(),
        current_fidelity: FidelityTier::Exact,
        retrieval_similarity: Some(0.92),
        calibrated_novelty: None,
        numerically_healthy: Some(true),
        accounting_complete: None,
        lifecycle_stable: None,
        conservation_residual_ratio: None,
        constraint_error: None,
        activity: Some(0.05),
        causal_importance: Some(0.10),
        error_prediction: None,
    };

    let assessment = observation
        .assess_known_risk(&AdaptiveFidelityPolicy::default())
        .unwrap();

    assert!(!assessment.evidence_complete);
    assert!(!assessment.reduction_ready);
    assert!(observation.complete_fidelity_evidence().unwrap().is_none());
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
    assert!(
        assessment
            .missing_for_reduction
            .contains(&ShadowSignal::ConstraintError)
    );
    assert!(
        assessment
            .missing_for_reduction
            .contains(&ShadowSignal::CalibratedNovelty)
    );
    assert!(
        assessment
            .missing_for_reduction
            .contains(&ShadowSignal::CalibratedErrorPrediction)
    );
}

#[test]
fn raw_retrieval_similarity_is_only_a_risk_raising_proxy() {
    let observation = ShadowFidelityObservation {
        tick: 101,
        exact_digest: digest(),
        current_fidelity: FidelityTier::Standard,
        retrieval_similarity: Some(0.10),
        calibrated_novelty: None,
        numerically_healthy: Some(true),
        accounting_complete: None,
        lifecycle_stable: None,
        conservation_residual_ratio: Some(1.0e-12),
        constraint_error: Some(0.001),
        activity: Some(0.05),
        causal_importance: Some(0.10),
        error_prediction: None,
    };

    let assessment = observation
        .assess_known_risk(&AdaptiveFidelityPolicy::default())
        .unwrap();

    assert_eq!(assessment.novelty_value, Some(0.90));
    assert_eq!(
        assessment.novelty_evidence,
        Some(NoveltyEvidenceKind::RetrievalProxy)
    );
    assert_eq!(assessment.known_risk_floor, FidelityTier::Exact);
    assert!(!assessment.evidence_complete);
    assert!(!assessment.reduction_ready);
}

#[test]
fn complete_calibrated_packet_builds_complete_controller_evidence() {
    let observation = complete_observation();
    let assessment = observation
        .assess_known_risk(&AdaptiveFidelityPolicy::default())
        .unwrap();

    assert!(assessment.evidence_complete);
    assert!(assessment.reduction_ready);
    assert!(assessment.missing_for_reduction.is_empty());
    assert!(assessment.reduction_blockers.is_empty());
    assert_eq!(
        assessment.novelty_evidence,
        Some(NoveltyEvidenceKind::Calibrated)
    );
    assert_eq!(assessment.known_risk_floor, FidelityTier::Coarse);

    let evidence = observation.complete_fidelity_evidence().unwrap().unwrap();
    assert!(evidence.numerically_healthy);
    assert!(evidence.accounting_complete);
    assert!(evidence.lifecycle_stable);
    assert_eq!(evidence.novelty, 0.04);
    assert_eq!(observation.exact_digest.algorithm_version, 2);
}

#[test]
fn zero_residual_cannot_hide_incomplete_accounting() {
    let mut observation = complete_observation();
    observation.accounting_complete = Some(false);
    observation.conservation_residual_ratio = Some(0.0);

    let assessment = observation
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
fn zero_residual_cannot_hide_unresolved_lifecycle() {
    let mut observation = complete_observation();
    observation.lifecycle_stable = Some(false);
    observation.conservation_residual_ratio = Some(0.0);

    let assessment = observation
        .assess_known_risk(&AdaptiveFidelityPolicy::default())
        .unwrap();

    assert_eq!(assessment.known_risk_floor, FidelityTier::Exact);
    assert!(assessment.reasons.contains(&FidelityReason::LifecycleStability));
    assert!(
        assessment
            .reduction_blockers
            .contains(&ShadowReductionBlocker::LifecycleUnstable)
    );
    assert!(!assessment.reduction_ready);
}

#[test]
fn calibrated_error_prediction_is_bound_to_the_one_tier_candidate() {
    let mut observation = complete_observation();
    observation
        .error_prediction
        .as_mut()
        .unwrap()
        .target_fidelity = FidelityTier::Standard;

    let assessment = observation
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
