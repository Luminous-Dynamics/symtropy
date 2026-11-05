// SPDX-License-Identifier: AGPL-3.0-or-later
//! Synthetic metamorphic oracle for common-mode-invariant four-quadrant WPT sensing.
//!
//! This crate tests one abstract property: a declared common positive scale
//! applied to every quadrant must not change a normalized four-channel
//! signature. A localized signature perturbation should remain distinguishable
//! under that same scale transformation.
//!
//! It does NOT implement or reproduce the published 4Q-FCR sensing equation,
//! OS-CFAR detector, or any hardware calibration. It is a known-truth control
//! for evaluating whether a candidate normalization pipeline preserves
//! nuisance invariance without erasing a localized anomaly.

use std::error::Error;
use std::fmt;

/// Raw non-negative magnitudes from four spatially identified quadrants.
///
/// Quadrant ordering is semantic and must be frozen by the sensor calibration.
/// This type intentionally does not sort or permute channels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuadrantVector {
    pub values: [f64; 4],
}

impl QuadrantVector {
    pub const fn new(values: [f64; 4]) -> Self {
        Self { values }
    }
}

/// Evidence state of an individual sensor channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelHealth {
    Healthy,
    Faulted,
    Unknown,
}

/// Scale-free signature whose components sum to one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NormalizedSignature {
    values: [f64; 4],
}

impl NormalizedSignature {
    pub const fn values(self) -> [f64; 4] {
        self.values
    }
}

/// Fail-closed validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvarianceError {
    NonFiniteChannel,
    NegativeChannel,
    ZeroTotalSignal,
    InvalidThreshold,
}

impl fmt::Display for InvarianceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteChannel => write!(formatter, "quadrant channels must be finite"),
            Self::NegativeChannel => write!(formatter, "quadrant magnitudes must be non-negative"),
            Self::ZeroTotalSignal => write!(formatter, "quadrant signal must have positive total magnitude"),
            Self::InvalidThreshold => write!(formatter, "decision threshold must be finite and in (0, 1]"),
        }
    }
}

impl Error for InvarianceError {}

/// Normalize four non-negative channel magnitudes with overflow-safe scaling.
///
/// Dividing by the maximum channel before summation avoids overflow for large
/// but finite magnitudes and underflow from a direct sum of disparate scales.
pub fn normalize_common_scale(
    vector: QuadrantVector,
) -> Result<NormalizedSignature, InvarianceError> {
    for value in vector.values {
        if !value.is_finite() {
            return Err(InvarianceError::NonFiniteChannel);
        }
        if value < 0.0 {
            return Err(InvarianceError::NegativeChannel);
        }
    }

    let maximum = vector.values.into_iter().fold(0.0_f64, f64::max);
    if maximum <= 0.0 {
        return Err(InvarianceError::ZeroTotalSignal);
    }

    let scaled = vector.values.map(|value| value / maximum);
    let total = scaled.into_iter().sum::<f64>();
    if !total.is_finite() || total <= 0.0 {
        return Err(InvarianceError::ZeroTotalSignal);
    }

    let values = scaled.map(|value| value / total);
    if !values.into_iter().all(f64::is_finite) {
        return Err(InvarianceError::NonFiniteChannel);
    }

    Ok(NormalizedSignature { values })
}

/// Total-variation distance between two normalized four-quadrant signatures.
///
/// The result lies in [0, 1]. It is a generic signature-distance fixture,
/// not an asserted threshold or the published paper's residual statistic.
pub fn total_variation_distance(
    left: NormalizedSignature,
    right: NormalizedSignature,
) -> f64 {
    0.5 * left
        .values
        .into_iter()
        .zip(right.values)
        .map(|(a, b)| (a - b).abs())
        .sum::<f64>()
}

/// Decision from a synthetic four-channel object-signature comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FodDecision {
    NoObjectSignature,
    ObjectSignaturePresent,
    NotEvaluable,
}

/// Auditable result for one fixture evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FodReport {
    pub baseline: NormalizedSignature,
    pub observed: Option<NormalizedSignature>,
    pub total_variation_distance: Option<f64>,
    pub threshold: f64,
    pub decision: FodDecision,
}

/// Compare a sample with a frozen object-free baseline.
///
/// Any faulted or unknown quadrant causes NOT_EVALUABLE. This is intentional:
/// the remaining three channels are not silently renormalized as though the
/// missing quadrant never existed.
pub fn evaluate_signature(
    baseline: NormalizedSignature,
    sample: QuadrantVector,
    channel_health: [ChannelHealth; 4],
    threshold: f64,
) -> Result<FodReport, InvarianceError> {
    if !threshold.is_finite() || threshold <= 0.0 || threshold > 1.0 {
        return Err(InvarianceError::InvalidThreshold);
    }

    if channel_health
        .into_iter()
        .any(|health| health != ChannelHealth::Healthy)
    {
        return Ok(FodReport {
            baseline,
            observed: None,
            total_variation_distance: None,
            threshold,
            decision: FodDecision::NotEvaluable,
        });
    }

    let observed = normalize_common_scale(sample)?;
    let distance = total_variation_distance(baseline, observed);
    let decision = if distance >= threshold {
        FodDecision::ObjectSignaturePresent
    } else {
        FodDecision::NoObjectSignature
    };

    Ok(FodReport {
        baseline,
        observed: Some(observed),
        total_variation_distance: Some(distance),
        threshold,
        decision,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEALTHY: [ChannelHealth; 4] = [ChannelHealth::Healthy; 4];

    fn baseline() -> NormalizedSignature {
        normalize_common_scale(QuadrantVector::new([25.0, 25.0, 25.0, 25.0])).unwrap()
    }

    #[test]
    fn object_free_signature_is_invariant_to_primary_common_scaling() {
        let reference = baseline();
        for scale in [0.01, 0.1, 0.5, 2.0, 10.0, 1.0e200] {
            let sample = QuadrantVector::new([25.0 * scale, 25.0 * scale, 25.0 * scale, 25.0 * scale]);
            let normalized = normalize_common_scale(sample).unwrap();
            assert!(total_variation_distance(reference, normalized) <= 1.0e-12);
        }
    }

    #[test]
    fn object_signature_remains_detectable_under_common_scaling() {
        let reference = baseline();
        for scale in [0.01, 0.5, 1.0, 4.0, 1.0e150] {
            let sample = QuadrantVector::new([
                50.0 * scale,
                25.0 * scale,
                15.0 * scale,
                10.0 * scale,
            ]);
            let report = evaluate_signature(reference, sample, HEALTHY, 0.10).unwrap();
            assert_eq!(report.decision, FodDecision::ObjectSignaturePresent);
        }
    }

    #[test]
    fn object_free_signature_does_not_trigger_detection() {
        let report = evaluate_signature(
            baseline(),
            QuadrantVector::new([100.0, 100.0, 100.0, 100.0]),
            HEALTHY,
            0.10,
        )
        .unwrap();
        assert_eq!(report.decision, FodDecision::NoObjectSignature);
        assert_eq!(report.total_variation_distance, Some(0.0));
    }

    #[test]
    fn localized_quadrant_perturbation_is_not_removed_by_normalization() {
        let report = evaluate_signature(
            baseline(),
            QuadrantVector::new([40.0, 20.0, 20.0, 20.0]),
            HEALTHY,
            0.10,
        )
        .unwrap();
        assert_eq!(report.decision, FodDecision::ObjectSignaturePresent);
        assert!(report.total_variation_distance.unwrap() > 0.10);
    }

    #[test]
    fn asymmetric_quadrant_permutation_is_not_silently_canonicalized() {
        let original = normalize_common_scale(QuadrantVector::new([40.0, 30.0, 20.0, 10.0])).unwrap();
        let permuted = normalize_common_scale(QuadrantVector::new([10.0, 20.0, 30.0, 40.0])).unwrap();
        assert!(total_variation_distance(original, permuted) > 0.0);
    }

    #[test]
    fn unknown_quadrant_health_fails_closed() {
        let report = evaluate_signature(
            baseline(),
            QuadrantVector::new([25.0, 25.0, 25.0, 25.0]),
            [
                ChannelHealth::Healthy,
                ChannelHealth::Unknown,
                ChannelHealth::Healthy,
                ChannelHealth::Healthy,
            ],
            0.10,
        )
        .unwrap();
        assert_eq!(report.decision, FodDecision::NotEvaluable);
        assert!(report.observed.is_none());
    }

    #[test]
    fn faulted_quadrant_is_not_silently_dropped() {
        let report = evaluate_signature(
            baseline(),
            QuadrantVector::new([50.0, 0.0, 25.0, 25.0]),
            [
                ChannelHealth::Healthy,
                ChannelHealth::Faulted,
                ChannelHealth::Healthy,
                ChannelHealth::Healthy,
            ],
            0.10,
        )
        .unwrap();
        assert_eq!(report.decision, FodDecision::NotEvaluable);
    }

    #[test]
    fn all_zero_channels_are_rejected() {
        assert_eq!(
            normalize_common_scale(QuadrantVector::new([0.0; 4])),
            Err(InvarianceError::ZeroTotalSignal)
        );
    }

    #[test]
    fn negative_channel_is_rejected() {
        assert_eq!(
            normalize_common_scale(QuadrantVector::new([1.0, -0.1, 1.0, 1.0])),
            Err(InvarianceError::NegativeChannel)
        );
    }

    #[test]
    fn non_finite_channel_is_rejected() {
        assert_eq!(
            normalize_common_scale(QuadrantVector::new([1.0, f64::NAN, 1.0, 1.0])),
            Err(InvarianceError::NonFiniteChannel)
        );
        assert_eq!(
            normalize_common_scale(QuadrantVector::new([1.0, f64::INFINITY, 1.0, 1.0])),
            Err(InvarianceError::NonFiniteChannel)
        );
    }

    #[test]
    fn invalid_threshold_is_rejected() {
        let reference = baseline();
        for threshold in [0.0, -0.1, f64::NAN, f64::INFINITY, 1.01] {
            assert_eq!(
                evaluate_signature(
                    reference,
                    QuadrantVector::new([25.0; 4]),
                    HEALTHY,
                    threshold
                ),
                Err(InvarianceError::InvalidThreshold)
            );
        }
    }

    #[test]
    fn normalization_handles_extreme_finite_magnitudes() {
        let small = normalize_common_scale(QuadrantVector::new([1.0e-250, 2.0e-250, 3.0e-250, 4.0e-250])).unwrap();
        let large = normalize_common_scale(QuadrantVector::new([1.0e250, 2.0e250, 3.0e250, 4.0e250])).unwrap();
        assert!(total_variation_distance(small, large) <= 1.0e-12);
    }
}
